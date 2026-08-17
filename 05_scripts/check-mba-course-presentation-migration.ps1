[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PresentationPlanPath,
    [Parameter(Mandatory=$true)][string]$SourceRoot,
    [Parameter(Mandatory=$true)][string]$PackageRoot,
    [Parameter(Mandatory=$true)][string]$ManifestPath
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Relative([string]$Root,[string]$Path){
    $prefix=[IO.Path]::GetFullPath($Root).TrimEnd('\')+'\';$full=[IO.Path]::GetFullPath($Path)
    if(-not $full.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw "Path is outside root: $Path"}
    $full.Substring($prefix.Length).Replace('\','/')
}
function File-Map([string]$Root){
    $map=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
    foreach($file in Get-ChildItem -LiteralPath $Root -Recurse -File|Sort-Object FullName){$map.Add((Relative $Root $file.FullName),(Hash $file.FullName))}
    $map
}
function Replace-Text([string]$Text,[Collections.IDictionary]$Map){
    foreach($key in $Map.Keys){$Text=$Text.Replace([string]$key,[string]$Map[$key])}
    $Text
}
function Complete-IndexNavigation([string]$Text,[object[]]$Supports){
    $evidence='[['+[string]$Supports[3].note+']]';$count=[regex]::Matches($Text,[regex]::Escape($evidence)).Count
    if($count -eq 0){
        $anchor='[['+[string]$Supports[2].note+']]'
        if([regex]::Matches($Text,[regex]::Escape($anchor)).Count -ne 1){throw 'Cannot deterministically add evidence-index navigation'}
        return $Text.Replace($anchor,$anchor+"`n- "+$evidence)
    }
    if($count -ne 1){throw 'Evidence-index navigation occurs more than once'}
    $Text
}
function Apply-OutlineNavigation([string]$Text,$Outline){
    $lines=@('## 课程大纲')
    if([string]$Outline.mode -ceq 'flat'){$lines+=@($Outline.units|ForEach-Object{"- [[$([string]$_.note)]]"})}
    else{foreach($section in @($Outline.sections)){$lines+='';$lines+="### $([string]$section.title)";$lines+=@($section.units|ForEach-Object{"- [[$([string]$_.note)]]"})}}
    $pattern='(?ms)^## (课程章节|课程大纲)\s*\r?\n.*?(?=^## |\z)'
    if([regex]::Matches($Text,$pattern).Count -ne 1){throw 'Index must contain exactly one replaceable course-outline section'}
    [regex]::Replace($Text,$pattern,(($lines -join "`n")+"`n`n"),1)
}

$planPath=(Get-Item -LiteralPath $PresentationPlanPath -ErrorAction Stop).FullName
$source=(Get-Item -LiteralPath $SourceRoot -ErrorAction Stop).FullName
$package=(Get-Item -LiteralPath $PackageRoot -ErrorAction Stop).FullName
$manifestResolved=(Get-Item -LiteralPath $ManifestPath -ErrorAction Stop).FullName
$plan=Get-Content -LiteralPath $planPath -Raw -Encoding utf8|ConvertFrom-Json
$manifest=Get-Content -LiteralPath $manifestResolved -Raw -Encoding utf8|ConvertFrom-Json
$planCheck=@(& (Join-Path $PSScriptRoot 'check-mba-course-presentation-plan.ps1') -PlanPath $planPath)
if($planCheck.Count -ne 1 -or [string]$planCheck[0].status -cne 'passed'){throw 'Presentation plan did not pass'}
if([string]$manifest.schema -cne 'babata.mba-course-presentation-migration-manifest/v1' -or [string]$manifest.status -cne 'passed_engineering_gates'){
    throw 'Unsupported presentation migration manifest'
}
if([string]$manifest.course -cne [string]$plan.course -or [string]$manifest.course_key -cne [string]$plan.course_key -or
   [string]$manifest.presentation_plan_sha256 -cne (Hash $planPath) -or [string]$manifest.source_root -cne $source){
    throw 'Migration manifest does not bind plan and source root'
}

$sourceManifest=Get-Content -LiteralPath ([string]$plan.source.manifest_path) -Raw -Encoding utf8|ConvertFrom-Json
if([string]$sourceManifest.schema -cne 'babata.mba-course-presentation-source-snapshot/v1' -or
   [string]$sourceManifest.status -cne 'compatible_source_frozen' -or
   [string]$sourceManifest.course -cne [string]$plan.course -or [string]$sourceManifest.course_key -cne [string]$plan.course_key -or
   -not [IO.Path]::GetFullPath([string]$sourceManifest.source_root).Equals($source,[StringComparison]::OrdinalIgnoreCase)){
    throw 'Source snapshot manifest does not bind this course and source root'
}
if([string]$sourceManifest.predecessor_manifest -cne [string]$plan.source.predecessor_manifest_path -or
   [string]$sourceManifest.predecessor_manifest_sha256 -cne [string]$plan.source.predecessor_manifest_sha256){throw 'Presentation plan and source snapshot disagree on predecessor manifest'}
$sourceDeclared=@($sourceManifest.package_files)
$sourceMap=File-Map $source;$packageMap=File-Map $package
if($sourceDeclared.Count -ne $sourceMap.Count){throw 'Source live does not match its immutable manifest file count'}
foreach($row in $sourceDeclared){
    $path=[string]$row.path
    if(-not $sourceMap.ContainsKey($path) -or [string]$row.sha256 -cne $sourceMap[$path]){throw "Source live is not bound to its immutable manifest: $path"}
}
$allowed=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($path in @('index.md','media/course-map.spec.json')){[void]$allowed.Add($path)}
$mmd=@($packageMap.Keys|Where-Object{$_ -like 'media/*.mmd'})
$png=@($packageMap.Keys|Where-Object{$_ -like 'media/*.png' -and $_ -match '课程脑图\.png$'})
if($mmd.Count -ne 1 -or $png.Count -ne 1){throw 'Package must contain one course-map Mermaid source and PNG'}
[void]$allowed.Add($mmd[0]);[void]$allowed.Add($png[0])

foreach($property in $plan.rename_map.PSObject.Properties){
    $old=$property.Name+'.md';$new=[string]$property.Value+'.md'
    if(-not $sourceMap.ContainsKey($old) -or $packageMap.ContainsKey($old) -or -not $packageMap.ContainsKey($new)){
        throw "Learning-support rename is incomplete: $old -> $new"
    }
    if($sourceMap[$old] -cne $packageMap[$new]){throw "Renamed learning-support body changed: $old -> $new"}
    [void]$allowed.Add($old);[void]$allowed.Add($new)
}

$expectedPaths=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($path in $sourceMap.Keys){[void]$expectedPaths.Add($path)}
foreach($property in $plan.rename_map.PSObject.Properties){
    [void]$expectedPaths.Remove($property.Name+'.md')
    [void]$expectedPaths.Add(([string]$property.Value)+'.md')
}
if($expectedPaths.Count -ne $packageMap.Count){throw 'Presentation migration exact file set has drifted'}
foreach($path in $expectedPaths){if(-not $packageMap.ContainsKey($path)){throw "Presentation migration package is missing expected path: $path"}}

foreach($path in $sourceMap.Keys){
    if($allowed.Contains($path)){continue}
    if(-not $packageMap.ContainsKey($path) -or $sourceMap[$path] -cne $packageMap[$path]){throw "Unauthorized presentation migration change: $path"}
}
$oldCount=@($plan.rename_map.PSObject.Properties).Count
if($packageMap.Count -ne $sourceMap.Count){throw "Presentation migration file-count drift: $($packageMap.Count)/$($sourceMap.Count)"}
if(@($packageMap.Keys|Where-Object{[IO.Path]::GetFileName($_) -match '^(09|10|11)-'}).Count){throw 'Numbered learning-support file remains in v2 package'}

$index=Get-Content -LiteralPath (Join-Path $package 'index.md') -Raw -Encoding utf8
$textRename=[ordered]@{};foreach($property in $plan.rename_map.PSObject.Properties){$textRename[$property.Name]=[string]$property.Value}
$indexRename=[ordered]@{};foreach($key in $textRename.Keys){$indexRename[$key]=$textRename[$key]};$indexRename['template_profile: semantic-obsidian/v1']='template_profile: semantic-obsidian/v2'
$expectedIndex=Apply-OutlineNavigation (Complete-IndexNavigation (Replace-Text ([IO.File]::ReadAllText((Join-Path $source 'index.md'),[Text.Encoding]::UTF8)) $indexRename) @($plan.learning_support)) $plan.outline
if($index -cne $expectedIndex){throw 'Index changed outside the declared profile and learning-support replacements'}
if($index -notmatch '(?m)^template_profile:\s*semantic-obsidian/v2\s*$'){throw 'Index does not declare semantic-obsidian/v2'}
foreach($support in @($plan.learning_support)){
    if([regex]::Matches($index,[regex]::Escape('[[{0}]]' -f [string]$support.note)).Count -ne 1){throw "Index must link learning support exactly once: $($support.note)"}
}
$orderedUnits=if([string]$plan.outline.mode -ceq 'flat'){@($plan.outline.units)}else{@($plan.outline.sections.units)}
$cursor=-1;foreach($unit in $orderedUnits){$match=@([regex]::Matches($index,[regex]::Escape("[[$([string]$unit.note)]]")));if($match.Count -ne 1 -or $match[0].Index -le $cursor){throw "Index does not preserve presentation unit order: $($unit.note)"};$cursor=$match[0].Index}
if([string]$plan.outline.mode -ceq 'sectioned'){$cursor=-1;foreach($section in @($plan.outline.sections)){$match=@([regex]::Matches($index,"(?m)^###\s+"+[regex]::Escape([string]$section.title)+"\s*$"));if($match.Count -ne 1 -or $match[0].Index -le $cursor){throw "Index does not preserve presentation section order: $($section.title)"};$cursor=$match[0].Index}}
foreach($property in $plan.rename_map.PSObject.Properties){if($index.Contains([string]$property.Name)){throw "Index retains legacy learning-support name: $($property.Name)"}}
$mmdText=Get-Content -LiteralPath (Join-Path $package $mmd[0]) -Raw -Encoding utf8
$expectedMmd=Replace-Text ([IO.File]::ReadAllText((Join-Path $source $mmd[0]),[Text.Encoding]::UTF8)) $textRename
if($mmdText -cne $expectedMmd){throw 'Course-map Mermaid changed outside declared learning-support replacements'}
foreach($support in @($plan.learning_support)){if(-not $mmdText.Contains([string]$support.note)){throw "Course map omits learning support: $($support.note)"}}
if($sourceMap.ContainsKey('media/course-map.spec.json')){
    $spec=Get-Content -LiteralPath (Join-Path $package 'media\course-map.spec.json') -Raw -Encoding utf8
    $expectedSpec=Replace-Text ([IO.File]::ReadAllText((Join-Path $source 'media\course-map.spec.json'),[Text.Encoding]::UTF8)) $textRename
    if($spec -cne $expectedSpec){throw 'Course-map spec changed outside declared learning-support replacements'}
}
if((Get-Item -LiteralPath (Join-Path $package $png[0])).Length -lt 10000){throw 'Course-map PNG is unexpectedly small'}

$declared=@($manifest.package_files)
if($declared.Count -ne $packageMap.Count){throw 'Migration manifest package-file count mismatch'}
foreach($row in $declared){
    $path=[string]$row.path
    if(-not $packageMap.ContainsKey($path) -or [string]$row.sha256 -cne $packageMap[$path]){throw "Migration manifest hash mismatch: $path"}
}
[pscustomobject][ordered]@{schema='babata.mba-course-presentation-migration-check/v1';status='passed';course=[string]$plan.course;outline_mode=[string]$plan.outline.mode;source_files=$sourceMap.Count;package_files=$packageMap.Count;renamed_learning_support=$oldCount;unauthorized_changes=0}
