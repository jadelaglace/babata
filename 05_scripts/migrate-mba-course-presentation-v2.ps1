[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PresentationPlanPath,
    [Parameter(Mandatory=$true)][string]$SourceRoot,
    [Parameter(Mandatory=$true)][string]$StagingRoot
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Relative([string]$Root,[string]$Path){
    $prefix=[IO.Path]::GetFullPath($Root).TrimEnd('\')+'\';$full=[IO.Path]::GetFullPath($Path)
    if(-not $full.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw "Path is outside root: $Path"}
    $full.Substring($prefix.Length).Replace('\','/')
}
function Tree([string]$Root){
    @(Get-ChildItem -LiteralPath $Root -Recurse -File|Sort-Object FullName|ForEach-Object{[ordered]@{path=Relative $Root $_.FullName;sha256=Hash $_.FullName;bytes=[long]$_.Length}})
}
function Tree-Hash([object[]]$Rows){
    $payload=(@($Rows|ForEach-Object{"$($_.path)`t$($_.sha256)`t$($_.bytes)"}) -join "`n")
    $sha=[Security.Cryptography.SHA256]::Create()
    try{[Convert]::ToHexString($sha.ComputeHash([Text.UTF8Encoding]::new($false).GetBytes($payload))).ToLowerInvariant()}finally{$sha.Dispose()}
}
function Write-Json([object]$Value,[string]$Path){[IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 40),[Text.UTF8Encoding]::new($false))}
function Replace-Text([string]$Path,[Collections.IDictionary]$Map){
    $text=[IO.File]::ReadAllText($Path,[Text.Encoding]::UTF8)
    foreach($key in $Map.Keys){$text=$text.Replace([string]$key,[string]$Map[$key])}
    [IO.File]::WriteAllText($Path,$text,[Text.UTF8Encoding]::new($false))
}
function Complete-IndexNavigation([string]$Path,[object[]]$Supports){
    $text=[IO.File]::ReadAllText($Path,[Text.Encoding]::UTF8)
    $evidence='[['+[string]$Supports[3].note+']]';$count=[regex]::Matches($text,[regex]::Escape($evidence)).Count
    if($count -eq 0){
        $anchor='[['+[string]$Supports[2].note+']]'
        if([regex]::Matches($text,[regex]::Escape($anchor)).Count -ne 1){throw 'Cannot deterministically add evidence-index navigation'}
        $text=$text.Replace($anchor,$anchor+"`n- "+$evidence)
    }elseif($count -ne 1){throw 'Evidence-index navigation must occur at most once before migration'}
    [IO.File]::WriteAllText($Path,$text,[Text.UTF8Encoding]::new($false))
}
function Apply-OutlineNavigation([string]$Path,$Outline){
    $text=[IO.File]::ReadAllText($Path,[Text.Encoding]::UTF8);$lines=@('## 课程大纲')
    if([string]$Outline.mode -ceq 'flat'){$lines+=@($Outline.units|ForEach-Object{"- [[$([string]$_.note)]]"})}
    else{foreach($section in @($Outline.sections)){$lines+='';$lines+="### $([string]$section.title)";$lines+=@($section.units|ForEach-Object{"- [[$([string]$_.note)]]"})}}
    $pattern='(?ms)^## (课程章节|课程大纲)\s*\r?\n.*?(?=^## |\z)'
    if([regex]::Matches($text,$pattern).Count -ne 1){throw 'Index must contain exactly one replaceable course-outline section'}
    $text=[regex]::Replace($text,$pattern,(($lines -join "`n")+"`n`n"),1)
    [IO.File]::WriteAllText($Path,$text,[Text.UTF8Encoding]::new($false))
}

$planPath=(Get-Item -LiteralPath $PresentationPlanPath -ErrorAction Stop).FullName
$source=(Get-Item -LiteralPath $SourceRoot -ErrorAction Stop).FullName
$staging=[IO.Path]::GetFullPath($StagingRoot)
if(Test-Path -LiteralPath $staging){throw "Fresh migration staging root already exists: $staging"}
$plan=Get-Content -LiteralPath $planPath -Raw -Encoding utf8|ConvertFrom-Json
$check=@(& (Join-Path $PSScriptRoot 'check-mba-course-presentation-plan.ps1') -PlanPath $planPath)
if($check.Count -ne 1 -or [string]$check[0].status -cne 'passed'){throw 'Presentation plan did not pass'}
if((Hash ([string]$plan.source.plan_path)) -cne [string]$plan.source.plan_sha256 -or
   (Hash ([string]$plan.source.manifest_path)) -cne [string]$plan.source.manifest_sha256){throw 'Presentation plan source hashes no longer match'}

[void](New-Item -ItemType Directory -Path $staging)
$package=Join-Path $staging 'package';[void](New-Item -ItemType Directory -Path $package)
foreach($entry in Get-ChildItem -LiteralPath $source -Force){Copy-Item -LiteralPath $entry.FullName -Destination $package -Recurse -Force}
$sourceFiles=@(Tree $source)
$rename=[ordered]@{}
foreach($property in $plan.rename_map.PSObject.Properties){
    $old=[string]$property.Name;$new=[string]$property.Value;$rename[$old]=$new
    $oldPath=Join-Path $package ($old+'.md');$newPath=Join-Path $package ($new+'.md')
    if((-not(Test-Path -LiteralPath $oldPath -PathType Leaf)) -or (Test-Path -LiteralPath $newPath)){throw "Unsafe learning-support rename: $old -> $new"}
    Move-Item -LiteralPath $oldPath -Destination $newPath
}

$references=[ordered]@{};foreach($key in $rename.Keys){$references[$key]=$rename[$key]}
$indexPath=Join-Path $package 'index.md';$references['template_profile: semantic-obsidian/v1']='template_profile: semantic-obsidian/v2'
Replace-Text $indexPath $references
Complete-IndexNavigation $indexPath @($plan.learning_support)
Apply-OutlineNavigation $indexPath $plan.outline
$specPath=Join-Path $package 'media\course-map.spec.json'
if(Test-Path -LiteralPath $specPath -PathType Leaf){Replace-Text $specPath $rename}
$mmdFiles=@(Get-ChildItem -LiteralPath (Join-Path $package 'media') -File -Filter '*.mmd')
if($mmdFiles.Count -ne 1){throw 'Expected exactly one package-owned course-map Mermaid source'}
Replace-Text $mmdFiles[0].FullName $rename
$pngPath=[IO.Path]::ChangeExtension($mmdFiles[0].FullName,'.png')
$render=@(& npx --yes '@mermaid-js/mermaid-cli@11.12.0' -i $mmdFiles[0].FullName -o $pngPath -b white -w 2200 -H 1600 2>&1)
if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $pngPath -PathType Leaf)){throw "Mermaid PNG rendering failed: $($render -join ' ')"}

$packageFiles=@(Tree $package)
$manifestPath=Join-Path $staging 'manifest.json'
$manifest=[ordered]@{
    schema='babata.mba-course-presentation-migration-manifest/v1'
    status='passed_engineering_gates'
    course=[string]$plan.course
    course_key=[string]$plan.course_key
    profile='semantic-obsidian/v2'
    course_acceptance=[string]$plan.output_status
    closure_state='unchanged_from_source'
    presentation_plan=$planPath
    presentation_plan_sha256=Hash $planPath
    source_root=$source
    source_tree_sha256=Tree-Hash $sourceFiles
    source_plan=[string]$plan.source.plan_path
    source_plan_sha256=[string]$plan.source.plan_sha256
    source_manifest=[string]$plan.source.manifest_path
    source_manifest_sha256=[string]$plan.source.manifest_sha256
    predecessor_manifest=[string]$plan.source.predecessor_manifest_path
    predecessor_manifest_sha256=[string]$plan.source.predecessor_manifest_sha256
    rename_map=$rename
    source_files=$sourceFiles.Count
    package_files=$packageFiles
}
Write-Json $manifest $manifestPath
$checked=@(& (Join-Path $PSScriptRoot 'check-mba-course-presentation-migration.ps1') -PresentationPlanPath $planPath -SourceRoot $source -PackageRoot $package -ManifestPath $manifestPath)
if($checked.Count -ne 1 -or [string]$checked[0].status -cne 'passed'){throw 'Presentation migration package did not pass'}
$receipt=[ordered]@{
    schema='babata.mba-course-presentation-migration-receipt/v1'
    status='passed_engineering_gates'
    course=[string]$plan.course
    source_plan=[string]$plan.source.plan_path
    source_plan_sha256=[string]$plan.source.plan_sha256
    source_manifest=[string]$plan.source.manifest_path
    source_manifest_sha256=[string]$plan.source.manifest_sha256
    predecessor_manifest=[string]$plan.source.predecessor_manifest_path
    predecessor_manifest_sha256=[string]$plan.source.predecessor_manifest_sha256
    presentation_plan_sha256=Hash $planPath
    migration_manifest=$manifestPath
    migration_manifest_sha256=Hash $manifestPath
    source_tree_sha256=Tree-Hash $sourceFiles
    package_tree_sha256=Tree-Hash $packageFiles
    source_files=$sourceFiles.Count
    package_files=$packageFiles.Count
    renamed_learning_support=@($rename.Keys|ForEach-Object{[ordered]@{from=$_+'.md';to=[string]$rename[$_]+'.md';body_sha256=(Hash (Join-Path $package ([string]$rename[$_]+'.md')))}})
    content_regeneration_runs=0
    c1b_registration_runs=0
    knowledge_registration_runs=0
    closure_verifier_runs=0
    course_acceptance=[string]$plan.output_status
    closure_state='unchanged_from_source'
    checker=$checked[0]
}
$receiptPath=Join-Path $staging 'migration-receipt.json';Write-Json $receipt $receiptPath
[pscustomobject][ordered]@{schema='babata.mba-course-presentation-migration/v1';status='passed_engineering_gates';course=[string]$plan.course;package=$package;manifest=$manifestPath;receipt=$receiptPath;files=$packageFiles.Count}
