[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$LegacyPlanPath,
    [Parameter(Mandatory=$true)][string]$SourceManifestPath,
    [Parameter(Mandatory=$true)][string]$SourceRoot,
    [Parameter(Mandatory=$true)][string]$OutputPath
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Relative([string]$Root,[string]$Path){
    $prefix=[IO.Path]::GetFullPath($Root).TrimEnd('\')+'\';$full=[IO.Path]::GetFullPath($Path)
    if(-not $full.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw "Path is outside source root: $Path"}
    $full.Substring($prefix.Length).Replace('\','/')
}
function Tree([string]$Root){
    @(Get-ChildItem -LiteralPath $Root -Recurse -File|Sort-Object FullName|ForEach-Object{[ordered]@{path=Relative $Root $_.FullName;sha256=Hash $_.FullName;bytes=[long]$_.Length}})
}
function Normalized-Markdown([string]$Path){
    ([IO.File]::ReadAllText($Path,[Text.Encoding]::UTF8).Replace("`r`n","`n")).TrimEnd("`n")
}
function Write-Utf8Json([object]$Value,[string]$Path){
    $parent=Split-Path $Path -Parent
    if(-not(Test-Path -LiteralPath $parent)){[void](New-Item -ItemType Directory -Path $parent)}
    [IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 40),[Text.UTF8Encoding]::new($false))
}
function Learning-Note([string]$Legacy,[string]$Slot){
    switch($Slot){
        'decision_tools' {
            if($Legacy -cnotmatch '^09-(.+)$'){throw "Legacy decision-tools note must start with 09-: $Legacy"}
            return '学习支持-'+$Matches[1]
        }
        'case_practice' {
            if($Legacy -cnotmatch '^10-(.+)$'){throw "Legacy case-practice note must start with 10-: $Legacy"}
            return '学习支持-'+$Matches[1]
        }
        'review_self_test' {
            if($Legacy -cne '11-复习与自测'){throw "Legacy review note is not canonical: $Legacy"}
            return '学习支持-复习与自测'
        }
        'evidence_index' {
            if($Legacy -cne '视觉证据索引'){throw "Legacy evidence index is not canonical: $Legacy"}
            return '视觉证据索引'
        }
    }
    throw "Unknown learning-support slot: $Slot"
}

$legacyPath=(Get-Item -LiteralPath $LegacyPlanPath -ErrorAction Stop).FullName
$predecessorManifestPath=(Get-Item -LiteralPath $SourceManifestPath -ErrorAction Stop).FullName
$source=(Get-Item -LiteralPath $SourceRoot -ErrorAction Stop).FullName
$legacy=Get-Content -LiteralPath $legacyPath -Raw -Encoding utf8|ConvertFrom-Json
$manifest=Get-Content -LiteralPath $predecessorManifestPath -Raw -Encoding utf8|ConvertFrom-Json
if([string]$legacy.schema -cne 'babata.mba-course-c2b-plan/v1'){throw 'Source plan must be historical MBA v1'}
if($null -ne $manifest.PSObject.Properties['course'] -and [string]$manifest.course -cne [string]$legacy.course){
    throw 'Source manifest course does not match legacy plan'
}
$predecessorPackage=Join-Path (Split-Path $predecessorManifestPath -Parent) 'package'
if(-not(Test-Path -LiteralPath $predecessorPackage -PathType Container)){throw 'Predecessor package required for compatibility comparison is missing'}
$sourceFiles=@(Tree $source);$declared=@($manifest.package_files)
if($sourceFiles.Count -ne $declared.Count){throw 'Canonical live file count differs from predecessor package manifest'}
$sourceByPath=@{};foreach($row in $sourceFiles){$sourceByPath[[string]$row.path]=$row}
$drift=@()
foreach($row in $declared){
    $relative=[string]$row.path
    if(-not $sourceByPath.ContainsKey($relative)){throw "Canonical live is missing predecessor path: $relative"}
    $predecessorFile=Join-Path $predecessorPackage $relative.Replace('/','\')
    if((-not(Test-Path -LiteralPath $predecessorFile -PathType Leaf)) -or (Hash $predecessorFile) -cne [string]$row.sha256){throw "Predecessor package does not match its manifest: $relative"}
    if([string]$sourceByPath[$relative].sha256 -cne [string]$row.sha256){
        if([IO.Path]::GetExtension($relative) -cne '.md' -or (Normalized-Markdown (Join-Path $source $relative.Replace('/','\'))) -cne (Normalized-Markdown $predecessorFile)){
            throw "Canonical live differs semantically from predecessor package: $relative"
        }
        $drift+=[ordered]@{path=$relative;kind='markdown_newline_normalization_only';predecessor_sha256=[string]$row.sha256;source_sha256=[string]$sourceByPath[$relative].sha256}
    }
}

$units=@();foreach($chapter in @($legacy.chapters)){
    $units+=[ordered]@{
        id=[string]$chapter.id
        note=[string]$chapter.note
        title=[string]$chapter.title
        source_modules=@($chapter.modules|ForEach-Object{[string]$_})
    }
}
$slots=@('decision_tools','case_practice','review_self_test','evidence_index')
$legacyLearning=@($legacy.course_map.learning.nodes)
if($legacyLearning.Count -ne 4){throw 'Legacy plan must contain four learning nodes'}
$support=@();$rename=[ordered]@{}
for($i=0;$i -lt 4;$i++){
    $old=[string]$legacyLearning[$i].note;$new=Learning-Note $old $slots[$i]
    $support+=[ordered]@{slot=$slots[$i];id=[string]$legacyLearning[$i].id;note=$new;legacy_note=$old}
    if($old -cne $new){$rename[$old]=$new}
}

$courseMap=$legacy.course_map|ConvertTo-Json -Depth 30|ConvertFrom-Json
for($i=0;$i -lt 4;$i++){$courseMap.learning.nodes[$i].note=$support[$i].note}
$unitByNote=@{};foreach($unit in $units){$unitByNote[[string]$unit.note]=$unit}
$groupedDomains=@($courseMap.domains|Where-Object{@($_.nodes).Count -gt 1})
if($groupedDomains.Count){
    $sections=@();foreach($domain in @($courseMap.domains)){
        $sectionUnits=@();foreach($node in @($domain.nodes)){
            $note=[string]$node.note
            if(-not $unitByNote.ContainsKey($note)){throw "Course-map domain references unknown outline unit: $note"}
            $sectionUnits+=$unitByNote[$note]
        }
        $sections+=[ordered]@{id=[string]$domain.id;title=[string]$domain.label;units=$sectionUnits}
    }
    $outline=[ordered]@{mode='sectioned';sections=$sections}
}else{$outline=[ordered]@{mode='flat';units=$units}}
$plan=[ordered]@{
    schema='babata.mba-course-presentation-plan/v2'
    course=[string]$legacy.course
    short_name=[string]$legacy.short_name
    course_key=[string]$legacy.course_key
    expected_modules=[int]$legacy.expected_modules
    profile='semantic-obsidian/v2'
    output_status=[string]$legacy.output_status
    source=[ordered]@{}
    outline=$outline
    learning_support=$support
    rename_map=$rename
    course_map=$courseMap
    live=$legacy.live
}
$output=[IO.Path]::GetFullPath($OutputPath)
$snapshotManifestPath=[IO.Path]::ChangeExtension($output,'.source-snapshot.json')
$snapshotManifest=[ordered]@{
    schema='babata.mba-course-presentation-source-snapshot/v1'
    status='compatible_source_frozen'
    course=[string]$legacy.course
    course_key=[string]$legacy.course_key
    source_root=$source
    predecessor_manifest=$predecessorManifestPath
    predecessor_manifest_sha256=Hash $predecessorManifestPath
    compatibility_drift=$drift
    package_files=$sourceFiles
}
Write-Utf8Json $snapshotManifest $snapshotManifestPath
$plan.source=[ordered]@{
    plan_path=$legacyPath
    plan_sha256=Hash $legacyPath
    manifest_path=$snapshotManifestPath
    manifest_sha256=Hash $snapshotManifestPath
    predecessor_manifest_path=$predecessorManifestPath
    predecessor_manifest_sha256=Hash $predecessorManifestPath
}
Write-Utf8Json $plan $output
$checker=Join-Path $PSScriptRoot 'check-mba-course-presentation-plan.ps1'
$check=@(& $checker -PlanPath $output)
if($check.Count -ne 1 -or [string]$check[0].status -cne 'passed'){throw 'Generated presentation plan did not pass its checker'}
[pscustomobject][ordered]@{schema='babata.mba-course-presentation-plan-generation/v1';status='passed';course=[string]$legacy.course;plan=$output;sha256=Hash $output;source_snapshot_manifest=$snapshotManifestPath;source_compatibility_drift=$drift.Count;units=$units.Count;source_modules=[int]$legacy.expected_modules}
