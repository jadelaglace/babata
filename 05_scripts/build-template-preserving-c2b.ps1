[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceMapPath,

    [Parameter(Mandatory = $true)]
    [string]$TemplatePath,

    [Parameter(Mandatory = $true)]
    [string]$MediaPlanPath,

    [Parameter(Mandatory = $true)]
    [string]$StagingRoot,

    [string]$CourseName,
    [switch]$AllowExistingStaging
)

$ErrorActionPreference = 'Stop'

function Get-RelativePath([string]$Base, [string]$Path) {
    $baseUri = [Uri]((Get-Item -LiteralPath $Base).FullName.TrimEnd('\') + '\')
    $pathUri = [Uri](Get-Item -LiteralPath $Path).FullName
    return [Uri]::UnescapeDataString($baseUri.MakeRelativeUri($pathUri).ToString()).Replace('\', '/')
}

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-StringSha256([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-FileRows([string]$Root) {
    $rows = @()
    foreach ($file in Get-ChildItem -LiteralPath $Root -Recurse -File) {
        $rows += [ordered]@{
            path = Get-RelativePath -Base $Root -Path $file.FullName
            bytes = $file.Length
            sha256 = Get-FileSha256 -Path $file.FullName
        }
    }
    return @($rows | Sort-Object path)
}

function Get-TemplateFrontmatter([string]$Text) {
    $match = [regex]::Match($Text, '(?s)\A---\r?\n.*?\r?\n---\r?\n?')
    if (-not $match.Success) { throw 'Template module note is missing YAML frontmatter.' }
    return $match.Value.TrimEnd("`r", "`n")
}

function Get-TemplateNavigationBody([string]$Text) {
    $match = [regex]::Match($Text, '(?ms)^## 关联知识节点\s*\r?\n(?<body>.*)$')
    if (-not $match.Success) { return '- 暂无课程级知识节点引用' }
    $body = $match.Groups['body'].Value.Trim()
    if ([string]::IsNullOrWhiteSpace($body)) { return '- 暂无课程级知识节点引用' }
    return $body
}

function Remove-LeadingTitle([string]$Text) {
    $body = $Text.Trim()
    if ($body -match '(?m)^# .+\r?\n') {
        return ([regex]::Replace($body, '(?m)^# .+\r?\n', '', 1)).Trim()
    }
    return $body
}

$sourceMapPath = (Get-Item -LiteralPath $SourceMapPath).FullName
$templatePath = (Get-Item -LiteralPath $TemplatePath).FullName
$mediaPlanPath = (Get-Item -LiteralPath $MediaPlanPath).FullName
$stagingRoot = [IO.Path]::GetFullPath($StagingRoot)
$packagePath = Join-Path $stagingRoot 'package'
$vaultPath = Join-Path $stagingRoot 'vault'

if ((Test-Path -LiteralPath $stagingRoot) -and -not $AllowExistingStaging) {
    throw "Staging root already exists. Use a new run directory; this builder never performs an implicit incremental update: $stagingRoot"
}
if (Test-Path -LiteralPath $stagingRoot) {
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $packagePath -Force | Out-Null

$sourceMap = Get-Content -LiteralPath $sourceMapPath -Raw | ConvertFrom-Json
if ($CourseName -and $sourceMap.course -ne $CourseName) {
    throw "Source map course '$($sourceMap.course)' does not match requested course '$CourseName'."
}
$course = if ($CourseName) { $CourseName } else { [string]$sourceMap.course }

$seen = @{}
$c1Items = @()
foreach ($chunk in @($sourceMap.chunks)) {
    foreach ($item in @($chunk.items)) {
        $key = [string]$item.module_id
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $c1Path = (Get-Item -LiteralPath $item.c1_path).FullName
        $actualHash = Get-FileSha256 -Path $c1Path
        if ($actualHash -ne ([string]$item.c1_sha256).ToLowerInvariant() -and $item.c1_sha256) {
            throw "C1 hash mismatch for module $key."
        }
        $c1Items += [ordered]@{
            module_id = [int]$item.module_id
            title = [string]$item.title
            module_type = [string]$item.module_type
            parent = [string]$item.parent
            c0_asset_sha256 = [string]$item.c0_asset_sha256
            c0_item_id = [string]$item.c0_item_id
            c0_revision_id = [string]$item.c0_revision_id
            c0_asset_id = [string]$item.c0_asset_id
            c1_path = $c1Path
            c1_sha256 = $actualHash
            c1_bytes = (Get-Item -LiteralPath $c1Path).Length
            c1_chars = (Get-Content -LiteralPath $c1Path -Raw).Length
            full_text_preserved = $true
        }
    }
}
if ($c1Items.Count -eq 0) { throw 'Source map contains no C1 items.' }

$mediaPlan = Get-Content -LiteralPath $mediaPlanPath -Raw | ConvertFrom-Json
$mediaRows = @($mediaPlan.media)
$targetNotes = @($mediaRows | ForEach-Object { @($_.target_notes) } | Sort-Object -Unique)

# Start every run from the approved semantic baseline. No source C1 text or template
# note is mutated in place; only the explicit target notes receive media blocks.
Get-ChildItem -LiteralPath $templatePath -Force | Copy-Item -Destination $packagePath -Recurse -Force
$baselineRows = Get-FileRows -Root $packagePath

# The approved template supplies the semantic navigation and note identity, but its
# module leaves are intentionally short evidence stubs. Materialize the complete,
# hash-checked C1 body into each corresponding module note so a reader does not have
# to leave the C2B export to reach the actual course content.
$contentTargetNotes = @()
foreach ($item in $c1Items) {
    $moduleNotePath = "模块/M-$($item.module_id).md"
    $moduleNote = Join-Path $packagePath ($moduleNotePath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $moduleNote -PathType Leaf)) {
        throw "C1 module $($item.module_id) has no matching template leaf: $moduleNotePath"
    }
    $templateNoteText = Get-Content -LiteralPath $moduleNote -Raw
    $frontmatter = Get-TemplateFrontmatter -Text $templateNoteText
    $navigation = Get-TemplateNavigationBody -Text $templateNoteText
    $c1Body = Remove-LeadingTitle -Text (Get-Content -LiteralPath $item.c1_path -Raw)
    if ([string]::IsNullOrWhiteSpace($c1Body)) {
        throw "C1 module $($item.module_id) is empty and cannot be materialized."
    }
    $item.c1_body_sha256 = Get-StringSha256 -Text $c1Body
    $trace = @(
        '- C1 文件：' + $item.c1_path.Replace('\', '/')
        '- C1 SHA-256：' + $item.c1_sha256
        '- C1 字符数：' + $item.c1_chars
        '- C0 item：' + $item.c0_item_id
        '- C0 revision：' + $item.c0_revision_id
        '- C0 asset：' + $item.c0_asset_id
        '- C0 SHA-256：' + $item.c0_asset_sha256
    ) -join "`n"
    $materialized = @(
        $frontmatter
        ''
        '# ' + $item.title
        ''
        '## 完整 C1 正文'
        ''
        $c1Body
        ''
        '## 关联知识节点'
        ''
        $navigation
        ''
        '<details>'
        '<summary>来源与追溯</summary>'
        ''
        $trace
        ''
        '</details>'
        ''
    ) -join "`n"
    Set-Content -LiteralPath $moduleNote -Value $materialized -Encoding utf8
    $contentTargetNotes += $moduleNotePath
}
$contentTargetNotes = @($contentTargetNotes | Sort-Object -Unique)

$mediaDir = Join-Path $packagePath 'media'
New-Item -ItemType Directory -Path $mediaDir -Force | Out-Null
$retainedMedia = @()
foreach ($media in $mediaRows) {
    if (-not $media.source_path -or -not $media.filename -or -not $media.target_notes) {
        throw 'Each media plan row requires source_path, filename and target_notes.'
    }
    $source = (Get-Item -LiteralPath $media.source_path).FullName
    $destination = Join-Path $mediaDir ([IO.Path]::GetFileName([string]$media.filename))
    Copy-Item -LiteralPath $source -Destination $destination -Force
    $retainedMedia += [ordered]@{
        path = 'media/' + [IO.Path]::GetFileName([string]$media.filename)
        modality = [string]$media.modality
        role = [string]$media.role
        source_module_id = [int]$media.source_module_id
        source_path = $source
        source_locator = $media.source_locator
        processing = @($media.processing)
        loss_notes = @($media.loss_notes)
        sha256 = Get-FileSha256 -Path $destination
        target_notes = @($media.target_notes)
    }
}

foreach ($notePath in $targetNotes) {
    $note = Join-Path $packagePath ($notePath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $note -PathType Leaf)) { throw "Media target note does not exist: $notePath" }
    $noteMedia = @($retainedMedia | Where-Object { $_.target_notes -contains $notePath })
    $block = "`n`n### 视觉证据`n`n"
    foreach ($media in $noteMedia) {
        $label = ([string]$media.role).Trim()
        if ($media.modality -eq 'image') {
            $block += "![${label}](../$($media.path))`n`n"
        } elseif ($media.modality -eq 'audio') {
            $block += "[${label}](../$($media.path))`n`n"
        } elseif ($media.modality -eq 'video') {
            $block += "[${label}](../$($media.path))`n`n"
        } else {
            $block += "[${label}](../$($media.path))`n`n"
        }
    }
    Add-Content -LiteralPath $note -Value $block -Encoding utf8
}

$allowedChanges = @($targetNotes + $contentTargetNotes | Sort-Object -Unique)
$unexpectedChanges = @()
foreach ($row in $baselineRows) {
    $candidate = Join-Path $packagePath ($row.path.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { $unexpectedChanges += "missing:$($row.path)"; continue }
    if ((Get-FileSha256 -Path $candidate) -ne $row.sha256 -and $allowedChanges -notcontains $row.path) {
        $unexpectedChanges += "unexpected-change:$($row.path)"
    }
}
if ($unexpectedChanges.Count -gt 0) { throw "Approved template changed outside media targets: $($unexpectedChanges -join ', ')" }

$mediaByModule = @{}
foreach ($media in $retainedMedia) {
    $key = [string]$media.source_module_id
    if (-not $mediaByModule.ContainsKey($key)) { $mediaByModule[$key] = @() }
    $mediaByModule[$key] += [string]$media.modality
}
$decisionItems = @()
foreach ($item in $c1Items) {
    $modalities = @('text')
    if ($mediaByModule.ContainsKey([string]$item.module_id)) { $modalities += @($mediaByModule[[string]$item.module_id]) }
    $item.retained_modalities = @($modalities | Select-Object -Unique)
    $item.decision = if ($item.retained_modalities.Count -gt 1) { 'full C1 text carried forward; retained C1B excerpts listed in media plan' } else { 'full C1 text carried forward; no necessary additional modality identified' }
    $decisionItems += $item
}

$decision = [ordered]@{
    schema = 'babata.c1b-full-course-decision/v2'
    variant = 'c1b'
    task = Split-Path -Leaf $stagingRoot
    course = $course
    status = 'staged_only'
    source_c1 = [ordered]@{
        source_map = $sourceMapPath
        source_map_sha256 = Get-FileSha256 -Path $sourceMapPath
        module_count = $c1Items.Count
        full_text_preserved = @($decisionItems | Where-Object full_text_preserved).Count
    }
    text_contract = 'Every unique C1 derivative is read, hash-checked and carried into the C1B decision before C2B materialization.'
    items = $decisionItems
    retained_media = $retainedMedia
    summary = [ordered]@{
        c1_modules = $c1Items.Count
        full_text_preserved = @($decisionItems | Where-Object full_text_preserved).Count
        retained_images = @($retainedMedia | Where-Object modality -eq 'image').Count
        retained_audio = @($retainedMedia | Where-Object modality -eq 'audio').Count
        retained_video = @($retainedMedia | Where-Object modality -eq 'video').Count
        retained_attachments = @($retainedMedia | Where-Object modality -eq 'attachment').Count
    }
    provider = 'codex_agent_local_judgment'
    remote_model_calls = 0
}
$decisionPath = Join-Path $stagingRoot 'c1b-decision.json'
$decision | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $decisionPath -Encoding utf8

New-Item -ItemType Directory -Path $vaultPath -Force | Out-Null
Get-ChildItem -LiteralPath $packagePath -Force | Copy-Item -Destination $vaultPath -Recurse -Force
$packageRows = Get-FileRows -Root $packagePath
$vaultRows = Get-FileRows -Root $vaultPath
$packageComparable = @($packageRows | ConvertTo-Json -Compress -Depth 8)
$vaultComparable = @($vaultRows | ConvertTo-Json -Compress -Depth 8)
if ($packageComparable -ne $vaultComparable) { throw 'Vault does not match package.' }

$manifest = [ordered]@{
    schema = 'babata.c2b.template-preserving-manifest/v1'
    task = Split-Path -Leaf $stagingRoot
    course = $course
    status = 'staged_only'
    pipeline = 'full_c1_to_c1b_to_template_preserving_c2b'
    source_c1 = [ordered]@{ source_map = $sourceMapPath; source_map_sha256 = Get-FileSha256 -Path $sourceMapPath; module_count = $c1Items.Count; c1_items = $decisionItems }
    template = [ordered]@{
        path = $templatePath
        baseline_file_count = $baselineRows.Count
        content_target_notes = $contentTargetNotes
        media_target_notes = $targetNotes
        target_notes = $allowedChanges
    }
    c1b_decision = 'c1b-decision.json'
    retained_media = $retainedMedia
    output = [ordered]@{
        package_files = $packageRows.Count
        vault_files = $vaultRows.Count
        package_vault_hash_differences = 0
        c1_content_materialized_modules = $contentTargetNotes.Count
        package = 'package'
        vault = 'vault'
    }
    formal_registration = 'not_started'
    remote_model_calls = 0
    embedding = $false
    rag = $false
}
$manifest | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath (Join-Path $stagingRoot 'manifest.json') -Encoding utf8

$recipe = [ordered]@{
    schema = 'babata.c2b.template-preserving-rebuild/v1'
    command = "build-template-preserving-c2b.ps1 -SourceMapPath '<source-map.json>' -TemplatePath '<approved-template>' -MediaPlanPath '<media-plan.json>' -StagingRoot '<new-run-root>'"
    rule = 'Always create a fresh staging root from C1; never patch an existing C2B output.'
    allowed_note_changes = $allowedChanges
}
$recipe | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $stagingRoot 'rebuild-recipe.json') -Encoding utf8

@("# $course full C1 -> C2B pilot", '', '- status: `staged_only`', "- C1 modules: $($c1Items.Count)/$($c1Items.Count)", "- C1B full text preserved: $(@($decisionItems | Where-Object full_text_preserved).Count)/$($c1Items.Count)", "- C2B module leaves with full C1 body: $($contentTargetNotes.Count)/$($c1Items.Count)", "- retained media: $($retainedMedia.Count)", "- package/vault files: $($packageRows.Count)/$($vaultRows.Count)", '- Obsidian output is a deletable export; package/manifest is the rebuild source.', '- Module leaves are content-first; provenance is collapsed after the body. Semantic notes remain the approved template baseline.', '', 'Control-plane requirements belong in manifest/README/REPORT, not knowledge notes.') | Set-Content -LiteralPath (Join-Path $stagingRoot 'REPORT.md') -Encoding utf8

[ordered]@{
    status = 'passed'
    c1_modules = $c1Items.Count
    full_text_preserved = @($decisionItems | Where-Object full_text_preserved).Count
    retained_media = $retainedMedia.Count
    c1_content_materialized_modules = $contentTargetNotes.Count
    target_notes = $allowedChanges
    package_files = $packageRows.Count
    vault_files = $vaultRows.Count
    package_vault_hash_differences = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $stagingRoot 'verification.json') -Encoding utf8

Write-Output "staged=$stagingRoot c1=$($c1Items.Count) retained_media=$($retainedMedia.Count) package=$($packageRows.Count) vault=$($vaultRows.Count)"
