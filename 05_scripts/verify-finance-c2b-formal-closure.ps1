[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$C1BRegistrationLedgerPath,
    [Parameter(Mandatory=$true)][string]$KnowledgeUniverseLedgerPath,
    [Parameter(Mandatory=$true)][string]$C2BStagingRoot,
    [Parameter(Mandatory=$true)][string]$LiveVaultPath,
    [string]$DataHome=$env:BABATA_DATA_HOME,
    [string]$OutputPath
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Escape-Sql([string]$Value){$Value.Replace("'","''")}
function SqlRows([string]$Database,[string]$Sql){
    $output=& sqlite3 -json $Database $Sql
    if($LASTEXITCODE -ne 0){throw "sqlite read failed: $Sql"}
    $text=($output -join "`n").Trim()
    if([string]::IsNullOrWhiteSpace($text)-or$text-eq'[]'){return @()}
    return @($text|ConvertFrom-Json)
}
function Assert-Equal($Actual,$Expected,[string]$Label){if($Actual -ne $Expected){throw "$Label mismatch: expected $Expected, got $Actual"}}

if([string]::IsNullOrWhiteSpace($DataHome)){throw 'BABATA_DATA_HOME or -DataHome is required'}
$dataHomeResolved=[IO.Path]::GetFullPath($DataHome)
$rawDb=Join-Path $dataHomeResolved '01_raw\index\raw.sqlite'
$derivedDb=Join-Path $dataHomeResolved '02_derived\index\derived.sqlite'
$c1bLedgerPath=(Get-Item -LiteralPath $C1BRegistrationLedgerPath).FullName
$universeLedgerPath=(Get-Item -LiteralPath $KnowledgeUniverseLedgerPath).FullName
$staging=(Get-Item -LiteralPath $C2BStagingRoot).FullName
$package=Join-Path $staging 'package'
$live=(Get-Item -LiteralPath $LiveVaultPath).FullName
$manifestPath=Join-Path $staging 'manifest.json'
foreach($path in @($rawDb,$derivedDb,$package,$manifestPath)){if(-not(Test-Path -LiteralPath $path)){throw "closure input missing: $path"}}
if([string]::IsNullOrWhiteSpace($OutputPath)){$OutputPath=Join-Path $staging 'closure-verification.json'}

$c1b=Get-Content -LiteralPath $c1bLedgerPath -Raw -Encoding utf8|ConvertFrom-Json
Assert-Equal ([string]$c1b.status) 'registered' 'C1B ledger status'
$c1bRows=@($c1b.registrations)
$mediaRows=@($c1bRows|ForEach-Object{$_.media_registrations})
Assert-Equal $c1bRows.Count 37 'C1B decision coverage'
Assert-Equal $mediaRows.Count 76 'C1B media coverage'
$managedFailures=@()
foreach($registration in @($c1bRows|ForEach-Object{$_.decision_registration})+$mediaRows){
    $logical=[string]$registration.logical_path
    $managed=Join-Path $dataHomeResolved $logical.Replace('/','\')
    if(-not $logical.StartsWith('02_derived/files/sha256/',[StringComparison]::Ordinal)-or-not(Test-Path -LiteralPath $managed -PathType Leaf)-or(Hash $managed)-ne[string]$registration.output_sha256){$managedFailures+=[string]$registration.derivative_id}
}
Assert-Equal $managedFailures.Count 0 'C1B managed output failures'

$c1bDb=@(SqlRows $derivedDb "SELECT p.tool_or_model,COUNT(*) active_runs,COUNT(DISTINCT p.input_revision_id) revisions,COUNT(DISTINCT d.derivative_id) derivatives,COUNT(DISTINCT d.output_sha256) output_hashes FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id WHERE p.invalidated_at IS NULL AND p.state='succeeded' AND p.tool_or_model IN ('finance-c1b-media-extractor','finance-c1b-essence-registrar') GROUP BY p.tool_or_model ORDER BY p.tool_or_model;")
$decisionDb=@($c1bDb|Where-Object tool_or_model -eq 'finance-c1b-essence-registrar')[0]
$mediaDb=@($c1bDb|Where-Object tool_or_model -eq 'finance-c1b-media-extractor')[0]
foreach($field in @('active_runs','revisions','derivatives','output_hashes')){Assert-Equal ([int]$decisionDb.$field) 37 "C1B decision DB $field"}
Assert-Equal ([int]$mediaDb.active_runs) 76 'C1B media DB active runs'
Assert-Equal ([int]$mediaDb.derivatives) 76 'C1B media DB derivatives'
Assert-Equal ([int]$mediaDb.output_hashes) 76 'C1B media DB output hashes'

$semanticDb=@(SqlRows $derivedDb "SELECT COUNT(*) active_runs,COUNT(DISTINCT input_revision_id) revisions FROM process_runs WHERE invalidated_at IS NULL AND state='succeeded' AND tool_or_model='c1b-full-text-semantic-materializer';")[0]
Assert-Equal ([int]$semanticDb.active_runs) 37 'C2B semantic active runs'
Assert-Equal ([int]$semanticDb.revisions) 37 'C2B semantic revisions'

$universe=Get-Content -LiteralPath $universeLedgerPath -Raw -Encoding utf8|ConvertFrom-Json
Assert-Equal ([string]$universe.status) 'registered' 'knowledge-universe ledger status'
$semanticEntries=@($universe.semantic_entries)
Assert-Equal $semanticEntries.Count 37 'knowledge-universe semantic entries'
$assignmentFailures=@()
foreach($entry in $semanticEntries){
    $sql="SELECT COUNT(*) n FROM semantic_map_assignments WHERE semantic_id='$(Escape-Sql ([string]$entry.semantic_id))' AND map_node_id='$(Escape-Sql ([string]$universe.branch.id))';"
    $row=@(SqlRows $rawDb $sql)[0]
    if([int]$row.n-ne1){$assignmentFailures+=[string]$entry.semantic_id}
}
Assert-Equal $assignmentFailures.Count 0 'knowledge-universe assignment failures'

$manifest=Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8|ConvertFrom-Json
Assert-Equal ([string]$manifest.status) 'accepted_benchmark' 'C2B manifest status'
Assert-Equal ([string]$manifest.formal_registration) 'registered' 'C2B formal registration'
Assert-Equal ([string]$manifest.c1b_registration.status) 'registered' 'manifest C1B status'
Assert-Equal @($manifest.c1b_registration.decision_derivative_ids).Count 37 'manifest C1B decisions'
Assert-Equal @($manifest.c1b_registration.media_derivative_ids).Count 76 'manifest C1B media'
Assert-Equal ([string]$manifest.knowledge_universe.status) 'registered' 'manifest universe status'
Assert-Equal @($manifest.knowledge_universe.semantic_ids).Count 37 'manifest universe entries'
Assert-Equal ([string]$manifest.obsidian_template.profile) 'semantic-obsidian/v1' 'Obsidian profile'
Assert-Equal ([string]$manifest.obsidian_template.status) 'accepted' 'Obsidian profile status'
Assert-Equal ([bool]$manifest.course_map.responsive_svg) $true 'manifest responsive SVG status'
Assert-Equal ([string]$manifest.course_map.default_expanded) 'mermaid' 'manifest default-expanded map'
Assert-Equal ([bool]$manifest.course_map.png_default_collapsed) $true 'manifest collapsed PNG fallback'

$declared=@{};foreach($row in @($manifest.package_files)){$declared[[string]$row.path]=[string]$row.sha256}
$packageFiles=Get-ChildItem -LiteralPath $package -Recurse -File
$liveFiles=Get-ChildItem -LiteralPath $live -Recurse -File
Assert-Equal $packageFiles.Count 92 'package file count'
Assert-Equal $liveFiles.Count 92 'live file count'
Assert-Equal $declared.Count 92 'manifest package file count'
$packageFailures=@();$liveFailures=@()
foreach($file in $packageFiles){
    $relative=$file.FullName.Substring($package.Length).TrimStart('\').Replace('\','/')
    if(-not$declared.ContainsKey($relative)-or(Hash $file.FullName)-ne$declared[$relative]){$packageFailures+=$relative}
    $liveFile=Join-Path $live $relative.Replace('/','\')
    if(-not(Test-Path -LiteralPath $liveFile -PathType Leaf)-or(Hash $file.FullName)-ne(Hash $liveFile)){$liveFailures+=$relative}
}
Assert-Equal $packageFailures.Count 0 'package manifest hash failures'
Assert-Equal $liveFailures.Count 0 'package/live hash failures'

$index=Get-Content -LiteralPath (Join-Path $live 'index.md') -Raw -Encoding utf8
foreach($token in @('status: accepted_benchmark','formal_registration: registered','c1b_registration: registered','knowledge_universe_registration: registered','template_profile: semantic-obsidian/v1','template_status: accepted')){if(-not$index.Contains($token)){throw "formal live index missing: $token"}}
$mapSource=Get-Content -LiteralPath (Join-Path $live 'media\财务管理课程脑图.mmd') -Raw -Encoding utf8
if($mapSource.Contains('"useMaxWidth": false') -or -not $mapSource.Contains('"useMaxWidth": true') -or
   [regex]::Matches($index,'(?m)^```mermaid$').Count -ne 1 -or
   [regex]::Matches($index,[regex]::Escape('![[media/财务管理课程脑图.png|760]]')).Count -ne 1 -or
   $index -notmatch '(?m)^> \[!info\]- 位图版本（打印 / 离线 / 渲染回退）\r?\n> !\[\[media/财务管理课程脑图\.png\|760\]\]$'){
    throw 'live responsive map contract failed'
}
$wikiFailures=@();$mediaFailures=@()
foreach($file in Get-ChildItem -LiteralPath $live -Filter '*.md' -File){
    $text=Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8
    foreach($match in [regex]::Matches($text,'(?<!!)\[\[([^\]|#]+)')){if(-not(Test-Path -LiteralPath (Join-Path $live ($match.Groups[1].Value+'.md')))){$wikiFailures+="$($file.Name):$($match.Groups[1].Value)"}}
    foreach($match in [regex]::Matches($text,'!\[\[[^\]|]+(?:\|[^\]]+)?\]\]|!\[[^\]]*\]\(([^)]+)\)')){
        $target=if($match.Value.StartsWith('![[')){[regex]::Match($match.Value,'!\[\[([^\]|]+)').Groups[1].Value}else{$match.Groups[1].Value}
        if(-not(Test-Path -LiteralPath (Join-Path $live $target))){$mediaFailures+="$($file.Name):$target"}
    }
}
Assert-Equal $wikiFailures.Count 0 'live Wiki-link failures'
Assert-Equal $mediaFailures.Count 0 'live media-link failures'

$rawQuick=(& sqlite3 $rawDb 'PRAGMA quick_check;').Trim();$derivedQuick=(& sqlite3 $derivedDb 'PRAGMA quick_check;').Trim()
Assert-Equal $rawQuick 'ok' 'raw database quick_check';Assert-Equal $derivedQuick 'ok' 'derived database quick_check'
$rawFk=@(& sqlite3 $rawDb 'PRAGMA foreign_key_check;');$derivedFk=@(& sqlite3 $derivedDb 'PRAGMA foreign_key_check;')
Assert-Equal @($rawFk|Where-Object{$_}).Count 0 'raw database foreign keys';Assert-Equal @($derivedFk|Where-Object{$_}).Count 0 'derived database foreign keys'

$result=[ordered]@{
    schema='babata.finance-c2b-formal-closure/v2'
    verified_at=(Get-Date).ToUniversalTime().ToString('o')
    status='passed'
    c1b=[ordered]@{complete_c1=37;essence_decisions=37;retained_media=76;managed_hash_failures=0}
    c2b=[ordered]@{semantic_active_runs=37;semantic_revisions=37;knowledge_universe_entries=37;branch_assignments=37}
    obsidian=[ordered]@{status='accepted_benchmark';formal_registration='registered';template_profile='semantic-obsidian/v1';template_status='accepted';responsive_svg=$true;default_expanded='mermaid';png_default_collapsed=$true;package_files=92;live_files=92;hash_differences=0;wiki_link_failures=0;media_link_failures=0}
    databases=[ordered]@{raw_quick_check=$rawQuick;derived_quick_check=$derivedQuick;foreign_key_failures=0}
    inputs=[ordered]@{c1b_ledger=$c1bLedgerPath;c1b_ledger_sha256=Hash $c1bLedgerPath;universe_ledger=$universeLedgerPath;universe_ledger_sha256=Hash $universeLedgerPath;manifest=$manifestPath;manifest_sha256=Hash $manifestPath;live=$live}
}
$result|ConvertTo-Json -Depth 12|Set-Content -LiteralPath $OutputPath -Encoding utf8
Write-Output "closure=$OutputPath status=passed c1b=37+76 c2b=37 package_live=92/92 hash_differences=0"
