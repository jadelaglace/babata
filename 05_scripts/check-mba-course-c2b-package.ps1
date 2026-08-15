[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$PackageRoot,
    [string]$ManifestPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Hash([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-ExternalHash([string]$Path,[string]$Expected,[string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not [IO.Path]::IsPathRooted($Path)) {
        throw "$Label path must be absolute"
    }
    if ($Expected -cnotmatch '^[0-9a-f]{64}$') { throw "$Label SHA256 must be canonical lowercase hex" }
    $resolved = (Get-Item -LiteralPath $Path -ErrorAction Stop).FullName
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) { throw "$Label input is not a file: $resolved" }
    if ((Get-Hash $resolved) -cne $Expected) { throw "$Label SHA256 mismatch" }
}

function Get-SourceMapItems($SourceMap) {
    $items=@();foreach($chunk in @($SourceMap.chunks)){foreach($item in @($chunk.items)){$items+=$item}};@($items)
}

function Assert-ExactModuleIds([object[]]$Actual,[string[]]$Expected,[string]$Label) {
    $ids=@($Actual|ForEach-Object{[string]$_})
    if($ids.Count -ne $Expected.Count -or @($ids|Sort-Object -Unique).Count -ne $ids.Count -or
        (($ids|Sort-Object)-join "`n") -cne (($Expected|Sort-Object)-join "`n")) {
        throw "$Label module-id set does not equal the exact course denominator"
    }
}

function Get-RelativePath([string]$Root,[string]$Path) {
    $prefix = $Root.TrimEnd('\') + '\'
    if (-not $Path.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside package root: $Path"
    }
    $Path.Substring($prefix.Length).Replace('\','/')
}

function Test-SamePath([string]$Left,[string]$Right) {
    [IO.Path]::GetFullPath($Left).TrimEnd('\').Equals(
        [IO.Path]::GetFullPath($Right).TrimEnd('\'),
        [StringComparison]::OrdinalIgnoreCase)
}

function Test-IsWithin([string]$Child,[string]$Parent) {
    $childFull = [IO.Path]::GetFullPath($Child).TrimEnd('\')
    $parentFull = [IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $childFull.Equals($parentFull,[StringComparison]::OrdinalIgnoreCase) -or
        $childFull.StartsWith($parentFull + '\',[StringComparison]::OrdinalIgnoreCase)
}

function Assert-RelativePath([string]$Path,[string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path.Contains('\') -or
        $Path.StartsWith('/') -or [IO.Path]::IsPathRooted($Path)) {
        throw "$Label must be a non-rooted forward-slash path: $Path"
    }
    $parts = @($Path.Split('/'))
    if ($parts.Count -eq 0 -or @($parts | Where-Object {
        [string]::IsNullOrWhiteSpace($_) -or $_ -eq '.' -or $_ -eq '..' -or $_.Contains(':')
    }).Count -ne 0) {
        throw "$Label contains an unsafe path segment: $Path"
    }
}

function Get-Frontmatter([string]$Text) {
    $lines = @($Text -split "\r?\n")
    if ($lines.Count -lt 3 -or $lines[0] -cne '---') { throw 'index.md must start with YAML frontmatter' }
    $end = -1
    for ($i = 1; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -ceq '---') { $end = $i; break }
    }
    if ($end -lt 2) { throw 'index.md frontmatter is not closed' }
    $result = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
    for ($i = 1; $i -lt $end; $i++) {
        $line = $lines[$i]
        if ([string]::IsNullOrWhiteSpace($line) -or $line.TrimStart().StartsWith('#')) { continue }
        if ($line -notmatch '^([A-Za-z0-9_-]+):\s*(.*?)\s*$') { continue }
        $key = [string]$Matches[1]
        $value = [string]$Matches[2]
        if ($result.ContainsKey($key)) { throw "index.md frontmatter duplicates key: $key" }
        if (($value.StartsWith('"') -and $value.EndsWith('"')) -or
            ($value.StartsWith("'") -and $value.EndsWith("'"))) {
            $value = $value.Substring(1,$value.Length-2)
        }
        $result.Add($key,$value)
    }
    $result
}

function Get-LocalTarget([string]$RawTarget,[string]$SourceRelative,[bool]$WikiLink,
    [Collections.Generic.Dictionary[string,string]]$Files,
    [Collections.Generic.Dictionary[string,System.Collections.Generic.List[string]]]$ByLeaf) {
    $target = $RawTarget.Trim()
    if ($target.StartsWith('<') -and $target.EndsWith('>')) { $target = $target.Substring(1,$target.Length-2) }
    if ($WikiLink) { $target = ($target -split '\|',2)[0].Trim() }
    $target = ($target -split '#',2)[0].Trim()
    if ([string]::IsNullOrWhiteSpace($target)) { return $SourceRelative }
    if ($target -match '^[A-Za-z][A-Za-z0-9+.-]*:') {
        if ($WikiLink -or $target.StartsWith('obsidian:',[StringComparison]::OrdinalIgnoreCase)) {
            throw "Forbidden URI target in ${SourceRelative}: $target"
        }
        return $null
    }
    try { $target = [Uri]::UnescapeDataString($target) } catch { throw "Invalid escaped link target in ${SourceRelative}: $target" }
    if ($target.Contains('\') -or $target.StartsWith('/') -or $target.Contains('../') -or
        $target.Contains('/..') -or [IO.Path]::IsPathRooted($target)) {
        throw "Unsafe local link target in ${SourceRelative}: $target"
    }

    $candidates = [Collections.Generic.List[string]]::new()
    $hasSlash = $target.Contains('/')
    $extension = [IO.Path]::GetExtension($target)
    $possible = @($target)
    if ([string]::IsNullOrWhiteSpace($extension)) { $possible += ($target + '.md') }
    if ($hasSlash -or $target.StartsWith('.')) {
        $sourceDir = [IO.Path]::GetDirectoryName($SourceRelative.Replace('/','\'))
        foreach ($item in $possible) {
            $normalized = $item.TrimStart('.','/').Replace('\','/')
            foreach ($candidate in @($normalized,$([IO.Path]::Combine($sourceDir,$item).Replace('\','/')))) {
                if ($Files.ContainsKey($candidate) -and -not $candidates.Contains($Files[$candidate])) {
                    [void]$candidates.Add($Files[$candidate])
                }
            }
        }
    } else {
        foreach ($item in $possible) {
            $leaf = [IO.Path]::GetFileName($item)
            if ($ByLeaf.ContainsKey($leaf)) {
                foreach ($candidate in $ByLeaf[$leaf]) { if (-not $candidates.Contains($candidate)) { [void]$candidates.Add($candidate) } }
            }
        }
    }
    if ($candidates.Count -ne 1) {
        throw "Local link target must resolve exactly once in ${SourceRelative}: $target (matches=$($candidates.Count))"
    }
    $candidates[0]
}

$planPath = (Get-Item -LiteralPath $CoursePlanPath -ErrorAction Stop).FullName
$package = (Get-Item -LiteralPath $PackageRoot -ErrorAction Stop).FullName
if (-not (Test-Path -LiteralPath $package -PathType Container)) { throw "Package root is not a directory: $package" }
if ([string]::IsNullOrWhiteSpace($ManifestPath)) { $ManifestPath = Join-Path (Split-Path $package -Parent) 'manifest.json' }
$manifestPathResolved = (Get-Item -LiteralPath $ManifestPath -ErrorAction Stop).FullName
if ((Test-IsWithin $planPath $package) -or (Test-IsWithin $manifestPathResolved $package)) {
    throw 'Course plan and package manifest must remain outside the user-visible package'
}

$plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json
$manifest = Get-Content -LiteralPath $manifestPathResolved -Raw -Encoding utf8 | ConvertFrom-Json
if ($plan.schema -cne 'babata.mba-course-c2b-plan/v1') { throw 'Unsupported MBA course plan schema' }
if ([int]$plan.expected_modules -lt 1 -or @($plan.chapters).Count -lt 1) { throw 'Course plan must declare a positive denominator and at least one chapter' }
if ([string]$plan.output_status -cne 'pending_user_acceptance') { throw 'Course plan must remain pending_user_acceptance before direct user approval' }
if ([string]$manifest.status -eq 'accepted_benchmark') { throw 'Generic MBA checker rejects the finance-only accepted_benchmark status' }
if ([string]$manifest.status -cne 'pending_user_acceptance') { throw 'MBA package status must be pending_user_acceptance' }
if ([string]$manifest.course -cne [string]$plan.course -or [string]$manifest.course_key -cne [string]$plan.course_key) {
    throw 'Manifest course identity does not match the course plan'
}
if ([string]$plan.course_key -notmatch '^[a-z0-9]+(?:-[a-z0-9]+)*$') { throw 'course_key must be a lowercase ASCII slug' }
if ([string]$manifest.course_plan_sha256 -cne (Get-Hash $planPath)) { throw 'Manifest course-plan hash mismatch' }
if ([string]$manifest.formal_registration -cne 'registered' -or
    [string]$manifest.c1b_registration.status -cne 'registered' -or
    [string]$manifest.knowledge_universe.status -cne 'registered') {
    throw 'Package requires formal C1B and knowledge-universe registration'
}
if ([string]$manifest.obsidian_template.profile -cne 'semantic-obsidian/v1' -or
    [string]$manifest.obsidian_template.status -cne 'accepted') {
    throw 'Package requires the accepted semantic-obsidian/v1 profile'
}
Assert-ExternalHash ([string]$manifest.source_map) ([string]$manifest.source_map_sha256) 'source map'
Assert-ExternalHash ([string]$manifest.c1b_registration.ledger) ([string]$manifest.c1b_ledger_sha256) 'C1B registration ledger'
Assert-ExternalHash ([string]$manifest.knowledge_universe.ledger) ([string]$manifest.knowledge_ledger_sha256) 'knowledge-universe ledger'
Assert-ExternalHash ([string]$manifest.learning_docs_manifest) ([string]$manifest.learning_docs_manifest_sha256) 'learning-docs manifest'
foreach ($externalPath in @(
    [string]$manifest.source_map,
    [string]$manifest.c1b_registration.ledger,
    [string]$manifest.knowledge_universe.ledger,
    [string]$manifest.learning_docs_manifest
)) {
    if (Test-IsWithin $externalPath $package) { throw "Provenance input must remain outside the package: $externalPath" }
}

$expectedModules=[int]$plan.expected_modules
$planModuleIds=@($plan.chapters|ForEach-Object{@($_.modules)}|ForEach-Object{[string]$_})
if($planModuleIds.Count -ne $expectedModules -or @($planModuleIds|Sort-Object -Unique).Count -ne $expectedModules){throw 'Course-plan chapter mapping is not an exact unique module denominator'}
Assert-ExactModuleIds @($manifest.module_ids) $planModuleIds 'package manifest'
$sourceMap=Get-Content -LiteralPath ([string]$manifest.source_map) -Raw -Encoding utf8|ConvertFrom-Json
$c1bLedger=Get-Content -LiteralPath ([string]$manifest.c1b_registration.ledger) -Raw -Encoding utf8|ConvertFrom-Json
$knowledgeLedger=Get-Content -LiteralPath ([string]$manifest.knowledge_universe.ledger) -Raw -Encoding utf8|ConvertFrom-Json
$learningManifest=Get-Content -LiteralPath ([string]$manifest.learning_docs_manifest) -Raw -Encoding utf8|ConvertFrom-Json
if([string]$sourceMap.schema -cne 'babata.mba.c2-source-map/v1' -or [string]$sourceMap.course -cne [string]$plan.course -or [int]$sourceMap.expected_modules -ne $expectedModules){throw 'Source map does not bind the plan course and denominator'}
if([string]$c1bLedger.schema -cne 'babata.mba-course-c1b-registration/v1' -or [string]$c1bLedger.status -cne 'registered' -or
    [string]$c1bLedger.course -cne [string]$plan.course -or [string]$c1bLedger.course_plan_sha256 -cne (Get-Hash $planPath) -or
    [string]$c1bLedger.source_map_sha256 -cne [string]$manifest.source_map_sha256){throw 'C1B ledger does not bind the plan and source map'}
if([string]$knowledgeLedger.schema -cne 'babata.mba-course-c2b-knowledge-registration/v1' -or [string]$knowledgeLedger.status -cne 'registered' -or
    [string]$knowledgeLedger.course_acceptance -cne 'pending_user_acceptance' -or [string]$knowledgeLedger.course -cne [string]$plan.course -or
    [string]$knowledgeLedger.plan_sha256 -cne (Get-Hash $planPath) -or [string]$knowledgeLedger.c1b_ledger_sha256 -cne [string]$manifest.c1b_ledger_sha256 -or
    [string]$knowledgeLedger.source_map_sha256 -cne [string]$manifest.source_map_sha256){throw 'Knowledge ledger does not bind the plan, source map, and formal C1B ledger'}
if([string]$learningManifest.schema -cne 'babata.mba-course-learning-docs/v1' -or [string]$learningManifest.status -cne 'candidate' -or
    [string]$learningManifest.course -cne [string]$plan.course -or [int]$learningManifest.expected_modules -ne $expectedModules -or
    [string]$learningManifest.course_plan_sha256 -cne (Get-Hash $planPath) -or [string]$learningManifest.source_map_sha256 -cne [string]$manifest.source_map_sha256){throw 'Learning-doc manifest does not bind the plan and source map'}
Assert-ExactModuleIds @((Get-SourceMapItems $sourceMap).module_id) $planModuleIds 'source map'
Assert-ExactModuleIds @($c1bLedger.registrations.module_id) $planModuleIds 'C1B ledger'
Assert-ExactModuleIds @($knowledgeLedger.modules.module_id) $planModuleIds 'knowledge ledger'
Assert-ExactModuleIds @($learningManifest.source_notes.module_id) $planModuleIds 'learning manifest'

$planLive = [IO.Path]::GetFullPath([string]$plan.live.path).TrimEnd('\')
$manifestLive = [IO.Path]::GetFullPath([string]$manifest.publication.live_path).TrimEnd('\')
if (-not (Test-SamePath $planLive $manifestLive) -or
    [string]$manifest.publication.vault -cne [string]$plan.live.vault -or
    [string]$manifest.publication.file -cne [string]$plan.live.file) {
    throw 'Manifest publication target does not exactly match the course plan'
}
$liveFile = [string]$plan.live.file
Assert-RelativePath $liveFile 'plan live file'
if (-not $liveFile.StartsWith('Babata/MBA/',[StringComparison]::Ordinal) -or -not $liveFile.EndsWith('/index.md',[StringComparison]::Ordinal)) {
    throw 'MBA live file must be a course index below Babata/MBA'
}
$vaultName = [string]$plan.live.vault
$cursor = Get-Item -LiteralPath (Split-Path $planLive -Parent) -ErrorAction Stop
while ($null -ne $cursor -and $cursor.Name -cne $vaultName) { $cursor = $cursor.Parent }
if ($null -eq $cursor) { throw 'Plan live path is not below the named Obsidian vault' }
$expectedIndex = Join-Path $cursor.FullName $liveFile.Replace('/','\')
if (-not (Test-SamePath $expectedIndex (Join-Path $planLive 'index.md'))) {
    throw 'Plan vault-relative file does not identify the plan live directory index'
}

$reparse = @(Get-ChildItem -LiteralPath $package -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
if ($reparse.Count) { throw "Package must not contain reparse points: $($reparse[0].FullName)" }
$actualFiles = @(Get-ChildItem -LiteralPath $package -Recurse -File -Force | Sort-Object FullName)
if ($actualFiles.Count -eq 0) { throw 'Package is empty' }
$actualByRel = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
$byLeaf = [Collections.Generic.Dictionary[string,System.Collections.Generic.List[string]]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $actualFiles) {
    $rel = Get-RelativePath $package $file.FullName
    Assert-RelativePath $rel 'package file'
    if ($actualByRel.ContainsKey($rel)) { throw "Package contains a case-insensitive path collision: $rel" }
    $actualByRel.Add($rel,$rel)
    $leaf = [IO.Path]::GetFileName($rel)
    if (-not $byLeaf.ContainsKey($leaf)) { $byLeaf.Add($leaf,[Collections.Generic.List[string]]::new()) }
    [void]$byLeaf[$leaf].Add($rel)
}

$declaredRows = @($manifest.package_files)
if ($declaredRows.Count -ne $actualFiles.Count) { throw 'Manifest package file count does not match the exact package file set' }
$declared = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($row in $declaredRows) {
    $rel = [string]$row.path
    Assert-RelativePath $rel 'manifest package file'
    if ($declared.ContainsKey($rel)) { throw "Manifest duplicates a package path: $rel" }
    $declared.Add($rel,$row)
}
foreach ($file in $actualFiles) {
    $rel = Get-RelativePath $package $file.FullName
    if (-not $declared.ContainsKey($rel)) { throw "Manifest omits package file: $rel" }
    $row = $declared[$rel]
    if ([long]$row.bytes -ne [long]$file.Length) { throw "Manifest byte count mismatch: $rel" }
    if ([string]$row.sha256 -cne (Get-Hash $file.FullName)) { throw "Manifest hash mismatch: $rel" }
}

$indexPath = Join-Path $package 'index.md'
if (-not (Test-Path -LiteralPath $indexPath -PathType Leaf)) { throw 'Package is missing index.md' }
$indexText = Get-Content -LiteralPath $indexPath -Raw -Encoding utf8
$frontmatter = Get-Frontmatter $indexText
$requiredFrontmatter = [ordered]@{
    course=[string]$plan.course; variant='c2b'; status='pending_user_acceptance';
    formal_registration='registered'; c1b_registration='registered';
    knowledge_universe_registration='registered'; template_profile='semantic-obsidian/v1'; template_status='accepted'
}
foreach ($key in $requiredFrontmatter.Keys) {
    if (-not $frontmatter.ContainsKey($key) -or $frontmatter[$key] -cne [string]$requiredFrontmatter[$key]) {
        throw "index.md frontmatter mismatch for ${key}: expected $($requiredFrontmatter[$key])"
    }
}

$referencedMedia = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$wikiLinks = 0
$markdownLinks = 0
$markdownFiles = @($actualFiles | Where-Object Extension -eq '.md')
foreach ($file in $markdownFiles) {
    $sourceRel = Get-RelativePath $package $file.FullName
    $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8
    $perFileMedia = [Collections.Generic.Dictionary[string,int]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($match in [regex]::Matches($text,'(?<embed>!)?\[\[(?<target>[^\]]+)\]\]')) {
        $wikiLinks++
        $resolved = Get-LocalTarget $match.Groups['target'].Value $sourceRel $true $actualByRel $byLeaf
        if ([string]::IsNullOrWhiteSpace($resolved)) { continue }
        if ($resolved.StartsWith('media/',[StringComparison]::OrdinalIgnoreCase)) {
            [void]$referencedMedia.Add($resolved)
            if (-not $perFileMedia.ContainsKey($resolved)) { $perFileMedia.Add($resolved,0) }
            $perFileMedia[$resolved]++
        }
    }
    foreach ($match in [regex]::Matches($text,'(?<embed>!)?\[[^\]]*\]\((?<target><[^>]+>|[^)\r\n]+)\)')) {
        $markdownLinks++
        $rawTarget = $match.Groups['target'].Value.Trim()
        if ($rawTarget -match '^(.+?)\s+["''].*["'']$') { $rawTarget = $Matches[1] }
        $resolved = Get-LocalTarget $rawTarget $sourceRel $false $actualByRel $byLeaf
        if ([string]::IsNullOrWhiteSpace($resolved)) { continue }
        if ($resolved.StartsWith('media/',[StringComparison]::OrdinalIgnoreCase)) {
            [void]$referencedMedia.Add($resolved)
            if (-not $perFileMedia.ContainsKey($resolved)) { $perFileMedia.Add($resolved,0) }
            $perFileMedia[$resolved]++
        }
    }
    foreach ($entry in $perFileMedia.GetEnumerator()) {
        if ($entry.Value -gt 1) { throw "Media path occurs more than once in one note: $sourceRel / $($entry.Key)" }
    }
}

$courseMap = $manifest.course_map
foreach ($name in @('mermaid','png')) {
    $path = [string]$courseMap.$name
    Assert-RelativePath $path "course_map.$name"
    if (-not $path.StartsWith('media/',[StringComparison]::OrdinalIgnoreCase)) { throw "Course-map artifact must be package-owned below media/: $path" }
    if (-not $actualByRel.ContainsKey($path)) { throw "Course-map artifact is missing from package: $path" }
}
if ([string]$courseMap.default_expanded -cne 'mermaid' -or -not [bool]$courseMap.responsive_svg -or
    -not [bool]$courseMap.png_default_collapsed) {
    throw 'Course map must use responsive Mermaid as the only default-expanded view and collapse the PNG fallback'
}
$mmdRel = [string]$courseMap.mermaid
$pngRel = [string]$courseMap.png
$mmd = Get-Content -LiteralPath (Join-Path $package $mmdRel.Replace('/','\')) -Raw -Encoding utf8
if ($mmd.Contains('obsidian://') -or $mmd -match '(?m)^\s*click\s+' -or
    $mmd.Contains('"useMaxWidth": false') -or -not $mmd.Contains('"useMaxWidth": true')) {
    throw 'Course-map Mermaid violates URI or responsive rendering boundaries'
}
$mapBlocks = @([regex]::Matches($indexText,'(?ms)^```mermaid\r?\n(.*?)\r?\n```'))
if ($mapBlocks.Count -ne 1 -or $mapBlocks[0].Groups[1].Value.Trim() -cne $mmd.Trim()) {
    throw 'index.md must contain exactly one Mermaid block identical to the package-owned Mermaid source'
}
if ([regex]::Matches($indexText,'(?m)^## 课程脑图\s*$').Count -ne 1) { throw 'index.md must contain exactly one course-map section' }
$displayWidth = [int]$courseMap.png_display_width
if ($displayWidth -lt 1 -or $displayWidth -gt 760) { throw 'Course-map PNG display width must be between 1 and 760 pixels' }
$pngEmbedPattern = '(?m)^> \[!info\]- .*\r?\n> !\[\[' + [regex]::Escape($pngRel) + '\|' + $displayWidth + '\]\]$'
if ([regex]::Matches($indexText,$pngEmbedPattern).Count -ne 1) { throw 'Course-map PNG must appear exactly once in a default-collapsed info callout' }

$chapterNotes = @($plan.chapters | ForEach-Object { [string]$_.note })
$mapChapterNotes = @($plan.course_map.domains | ForEach-Object { @($_.nodes) } | ForEach-Object { [string]$_.note })
$learningNotes = @($plan.course_map.learning.nodes | ForEach-Object { [string]$_.note })
if (@($chapterNotes | Sort-Object -Unique).Count -ne $chapterNotes.Count -or
    @($mapChapterNotes | Sort-Object -Unique).Count -ne $mapChapterNotes.Count -or
    (($chapterNotes | Sort-Object) -join "`n") -cne (($mapChapterNotes | Sort-Object) -join "`n")) {
    throw 'Course-map knowledge branches must partition the chapter notes exactly'
}
$linkedNotes = @($mapChapterNotes + $learningNotes)
if (@($linkedNotes | Sort-Object -Unique).Count -ne $linkedNotes.Count) { throw 'Course-map linked note labels must be unique' }
foreach ($note in $linkedNotes) {
    if (-not $actualByRel.ContainsKey($note + '.md')) { throw "Course-map linked note is missing: $note" }
    if ([regex]::Matches($mmd,[regex]::Escape('["' + $note + '"]')).Count -ne 1) {
        throw "Course-map note label must occur exactly once in Mermaid: $note"
    }
}
$internalClass = @([regex]::Matches($mmd,'(?m)^\s*class\s+(?<ids>[A-Za-z0-9_,]+)\s+internal-link\s*$'))
if ($internalClass.Count -ne 1) { throw 'Course-map Mermaid must contain one internal-link class assignment' }
$internalIds = @($internalClass[0].Groups['ids'].Value.Split(',') | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if (@($internalIds | Sort-Object -Unique).Count -ne $internalIds.Count) { throw 'Course-map internal-link node IDs must be unique' }
if ($internalIds.Count -ne $linkedNotes.Count -or [int]$courseMap.internal_link_targets -ne $linkedNotes.Count) {
    throw 'Course-map internal-link count does not match the plan and manifest'
}

$pngPath = Join-Path $package $pngRel.Replace('/','\')
if ((Get-Item -LiteralPath $pngPath).Length -lt 10000) { throw 'Course-map PNG is unexpectedly small' }
Add-Type -AssemblyName System.Drawing
$image = [Drawing.Image]::FromFile($pngPath)
try { $pngWidth=[int]$image.Width; $pngHeight=[int]$image.Height } finally { $image.Dispose() }
if ($pngWidth -ne [int]$courseMap.png_width -or $pngHeight -ne [int]$courseMap.png_height) {
    throw 'Course-map PNG dimensions do not match the manifest'
}
$aspect = $pngHeight / [double]$pngWidth
if ($aspect -gt 1.40 -or [Math]::Abs($aspect - [double]$courseMap.aspect_ratio) -gt 0.005 -or
    [double]$courseMap.effective_font_px -lt 11.0) {
    throw 'Course-map PNG violates the aspect-ratio or effective-font gate'
}

$mediaExtensions = @('.png','.jpg','.jpeg','.gif','.webp','.svg','.pdf','.mp3','.wav','.m4a','.mp4','.webm','.mov')
$mediaFiles = @($actualFiles | ForEach-Object { Get-RelativePath $package $_.FullName } | Where-Object {
    $_.StartsWith('media/',[StringComparison]::OrdinalIgnoreCase) -and $mediaExtensions -contains [IO.Path]::GetExtension($_).ToLowerInvariant()
})
foreach ($mediaFile in $mediaFiles) {
    if (-not $referencedMedia.Contains($mediaFile)) { throw "Package media is not referenced by any note: $mediaFile" }
}

[pscustomobject][ordered]@{
    schema='babata.mba-course-c2b-package-check/v1'
    status='passed'
    course=[string]$plan.course
    course_key=[string]$plan.course_key
    package_root=$package
    manifest=$manifestPathResolved
    manifest_sha256=Get-Hash $manifestPathResolved
    course_plan=$planPath
    course_plan_sha256=Get-Hash $planPath
    live_path=$planLive
    vault=$vaultName
    live_file=$liveFile
    package_files=$actualFiles.Count
    markdown_files=$markdownFiles.Count
    wiki_links=$wikiLinks
    markdown_links=$markdownLinks
    media_files=$mediaFiles.Count
    course_map_internal_links=$linkedNotes.Count
    course_map_png_width=$pngWidth
    course_map_png_height=$pngHeight
}
