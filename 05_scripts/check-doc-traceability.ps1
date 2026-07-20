param(
    [string]$DocsRoot
)

$ErrorActionPreference = 'Stop'

function Split-MarkdownTableRow {
    param(
        [Parameter(Mandatory)]
        [string]$Line
    )

    $trimmed = $Line.Trim()
    if (-not ($trimmed.StartsWith('|') -and $trimmed.EndsWith('|'))) {
        throw "Invalid Markdown table row: $Line"
    }

    $inner = $trimmed.Substring(1, $trimmed.Length - 2)
    $cells = @([regex]::Split($inner, '(?<!\\)\|') | ForEach-Object {
        $_.Trim().Replace('\|', '|')
    })
    return $cells
}

function Get-MarkdownTableRows {
    param(
        [Parameter(Mandatory)]
        [string]$Markdown,
        [Parameter(Mandatory)]
        [string]$Heading,
        [Parameter(Mandatory)]
        [string[]]$RequiredColumns
    )

    $lines = @($Markdown -split "`r?`n")
    $headingIndexes = @()
    for ($index = 0; $index -lt $lines.Count; $index++) {
        if ($lines[$index].Trim() -eq $Heading) {
            $headingIndexes += $index
        }
    }
    if ($headingIndexes.Count -ne 1) {
        throw "Expected exactly one '$Heading' heading, found $($headingIndexes.Count)"
    }

    $headerIndex = $headingIndexes[0] + 1
    while ($headerIndex -lt $lines.Count -and [string]::IsNullOrWhiteSpace($lines[$headerIndex])) {
        $headerIndex++
    }
    while ($headerIndex -lt $lines.Count -and -not $lines[$headerIndex].Trim().StartsWith('|')) {
        $headerIndex++
    }
    if ($headerIndex + 1 -ge $lines.Count) {
        throw "Missing Markdown table after '$Heading'"
    }

    $headers = @(Split-MarkdownTableRow -Line $lines[$headerIndex])
    $duplicateHeaders = @($headers | Group-Object | Where-Object Count -gt 1)
    if ($duplicateHeaders.Count -gt 0) {
        throw "Duplicate columns in '$Heading': $($duplicateHeaders.Name -join ', ')"
    }
    foreach ($column in $RequiredColumns) {
        if ($headers -notcontains $column) {
            throw "Table '$Heading' is missing required column: $column"
        }
    }

    $separator = @(Split-MarkdownTableRow -Line $lines[$headerIndex + 1])
    if ($separator.Count -ne $headers.Count) {
        throw "Table '$Heading' separator has $($separator.Count) cells; expected $($headers.Count)"
    }
    foreach ($cell in $separator) {
        if ($cell -notmatch '^:?-{3,}:?$') {
            throw "Invalid Markdown separator in '$Heading': $cell"
        }
    }

    $rows = @()
    for ($index = $headerIndex + 2; $index -lt $lines.Count; $index++) {
        if (-not $lines[$index].Trim().StartsWith('|')) {
            break
        }
        $cells = @(Split-MarkdownTableRow -Line $lines[$index])
        if ($cells.Count -ne $headers.Count) {
            throw "Table '$Heading' row $($index + 1) has $($cells.Count) cells; expected $($headers.Count)"
        }
        $row = [ordered]@{}
        for ($columnIndex = 0; $columnIndex -lt $headers.Count; $columnIndex++) {
            $row[$headers[$columnIndex]] = $cells[$columnIndex]
        }
        $rows += [pscustomobject]$row
    }
    if ($rows.Count -eq 0) {
        throw "Table '$Heading' has no data rows"
    }
    return $rows
}

function Assert-LegalEvidence {
    param(
        [Parameter(Mandatory)]
        [string]$Id,
        [Parameter(Mandatory)]
        [string]$Evidence
    )

    $tokens = @([regex]::Matches($Evidence, '(?<![A-Za-z0-9])E([0-9]+)(?![A-Za-z0-9])'))
    if ($tokens.Count -eq 0) {
        throw "$Id has no evidence level E0-E3"
    }
    if ($tokens.Count -ne 1) {
        throw "$Id must have exactly one current evidence level, found $($tokens.Count)"
    }
    foreach ($token in $tokens) {
        if ([int]$token.Groups[1].Value -notin 0..3) {
            throw "$Id has invalid evidence level: $($token.Value)"
        }
    }
    return [int]$tokens[0].Groups[1].Value
}

if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
    $DocsRoot = Join-Path $PSScriptRoot '..\00_docs'
}
$docs = (Resolve-Path -LiteralPath $DocsRoot).Path

$requiredMarkers = @(
    @('00_requirements/00_REQUIREMENTS.md', '## -1.'),
    @('01_prd/01_PRD.md', 'PRD-10'),
    @('02_acceptance/02_ACCEPTANCE_CRITERIA.md', 'AC-11'),
    @('03_architecture/03_ARCHITECTURE.md', 'AC-11'),
    @('03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md', '137'),
    @('03_architecture/05_RAW_FOUNDATION_BLUEPRINT.md', 'P3-G6'),
    @('03_architecture/06_RAW_FOUNDATION_EXECUTION_PLAN.md', 'P3-G6'),
    @('03_architecture/07_P4_FIRST_COLLECTION_PATHS.md', 'P4-G6'),
    @('03_architecture/08_SOURCE_TOOL_RESEARCH.md', 'P2-G7: passed'),
    @('03_architecture/09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md', 'P6-BLUEPRINT-STATUS: not-started'),
    @('04_process/04_DEVELOPMENT_PROCESS.md', 'P2: completed; P2-G1..P2-G7: passed'),
    @('05_tests/05_TEST_CASES.md', 'GT-P2-07')
)

foreach ($check in $requiredMarkers) {
    $path = Join-Path $docs $check[0]
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing authority document: $($check[0])"
    }
    if (-not (Select-String -SimpleMatch $check[1] -Path $path -Quiet)) {
        throw "Missing traceability marker '$($check[1])' in $($check[0])"
    }
}

$prd = Get-Content -Raw -Encoding utf8 (Join-Path $docs '01_prd/01_PRD.md')
$acceptance = Get-Content -Raw -Encoding utf8 (Join-Path $docs '02_acceptance/02_ACCEPTANCE_CRITERIA.md')
$architecture = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_ARCHITECTURE.md')
$tests = Get-Content -Raw -Encoding utf8 (Join-Path $docs '05_tests/05_TEST_CASES.md')

foreach ($id in 1..10) {
    $marker = 'PRD-{0:D2}' -f $id
    if (-not $prd.Contains($marker)) { throw "PRD is missing marker: $marker" }
    if (-not $acceptance.Contains($marker)) { throw "Acceptance is missing PRD trace: $marker" }
}

foreach ($id in 1..11) {
    $ac = 'AC-{0:D2}' -f $id
    $tc = 'TC-{0:D2}' -f $id
    if (-not $acceptance.Contains($ac)) { throw "Acceptance is missing marker: $ac" }
    if (-not $architecture.Contains($ac)) { throw "Architecture is missing acceptance trace: $ac" }
    if (-not $tests.Contains($ac)) { throw "Tests are missing acceptance trace: $ac" }
    if (-not $tests.Contains($tc)) { throw "Tests are missing marker: $tc" }
}

$skeleton = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md')
$rawBlueprint = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/05_RAW_FOUNDATION_BLUEPRINT.md')
$rawPlan = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/06_RAW_FOUNDATION_EXECUTION_PLAN.md')
$collection = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/07_P4_FIRST_COLLECTION_PATHS.md')
$sourceResearch = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/08_SOURCE_TOOL_RESEARCH.md')
$process = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_DEVELOPMENT_PROCESS.md')

$requiredP2Sources = @(
    'source.feishu', 'source.yuque', 'source.onenote', 'source.evernote',
    'source.wechat_favorites', 'source.wechat_articles', 'source.wechat_channels',
    'source.wechat_chats', 'source.zhihu', 'source.bilibili', 'source.xiaohongshu',
    'source.douyin', 'source.browser_bookmarks', 'source.browser_pages', 'source.doubao',
    'source.kimi', 'source.chatgpt', 'source.local_files', 'source.first_party'
)

$sourceColumns = @(
    'source_id', 'source', 'normal_route', 'minimum_authorization',
    'current_evidence', 'current_gap', 'current_status'
)
$sourceRows = @(Get-MarkdownTableRows -Markdown $sourceResearch -Heading '<!-- P2-G7-SOURCE-TABLE -->' -RequiredColumns $sourceColumns)
foreach ($row in $sourceRows) {
    foreach ($column in $sourceColumns) {
        if ([string]::IsNullOrWhiteSpace($row.$column)) {
            throw "Source row '$($row.source_id)' has empty required field: $column"
        }
    }
    if ($row.source_id -notmatch '^source\.[a-z0-9_]+$') {
        throw "Invalid source_id: $($row.source_id)"
    }
    $evidenceLevel = Assert-LegalEvidence -Id $row.source_id -Evidence $row.current_evidence
    if ($row.current_status -notin @('disabled', 'available')) {
        throw "Source '$($row.source_id)' has invalid current status: $($row.current_status)"
    }
    if ($evidenceLevel -lt 3 -and $row.current_status -ne 'disabled') {
        throw "Source '$($row.source_id)' is below E3 and must remain disabled"
    }
}
$duplicateSources = @($sourceRows | Group-Object source_id | Where-Object Count -gt 1)
if ($duplicateSources.Count -gt 0) {
    throw "Duplicate source_id entries: $($duplicateSources.Name -join ', ')"
}
foreach ($source in $requiredP2Sources) {
    if (@($sourceRows | Where-Object source_id -eq $source).Count -ne 1) {
        throw "Source research must contain exactly one required source_id: $source"
    }
}

$representativeTools = @(
    'tool.lark_cli', 'tool.agent_browser', 'tool.browser_use', 'tool.codex_chrome',
    'tool.opencli'
)
$toolColumns = @('tool_id', 'tool', 'current_evidence', 'next_user_action')
$toolRows = @(Get-MarkdownTableRows -Markdown $sourceResearch -Heading '<!-- P2-G7-TOOL-TABLE -->' -RequiredColumns $toolColumns)
foreach ($row in $toolRows) {
    foreach ($column in $toolColumns) {
        if ([string]::IsNullOrWhiteSpace($row.$column)) {
            throw "Tool row '$($row.tool_id)' has empty required field: $column"
        }
    }
    if ($row.tool_id -notmatch '^tool\.[a-z0-9_]+$') {
        throw "Invalid tool_id: $($row.tool_id)"
    }
    $null = Assert-LegalEvidence -Id $row.tool_id -Evidence $row.current_evidence
}
$duplicateTools = @($toolRows | Group-Object tool_id | Where-Object Count -gt 1)
if ($duplicateTools.Count -gt 0) {
    throw "Duplicate tool_id entries: $($duplicateTools.Name -join ', ')"
}
foreach ($tool in $representativeTools) {
    if (@($toolRows | Where-Object tool_id -eq $tool).Count -ne 1) {
        throw "Source research must contain exactly one representative tool_id: $tool"
    }
}

foreach ($id in 1..7) {
    $p2 = "P2-G$id"
    if (-not $skeleton.Contains($p2)) { throw "Skeleton is missing gate: $p2" }
    if (-not $process.Contains($p2)) { throw "Process is missing gate: $p2" }
    if (-not $tests.Contains($p2)) { throw "Tests are missing gate: $p2" }

    if ($id -le 6) {
        $p3 = "P3-G$id"
        if (-not $rawBlueprint.Contains($p3)) { throw "Raw blueprint is missing gate: $p3" }
        if (-not $rawPlan.Contains($p3)) { throw "Raw execution plan is missing gate: $p3" }
        if (-not $process.Contains($p3)) { throw "Process is missing gate: $p3" }

        $p4 = "P4-G$id"
        if (-not $collection.Contains($p4)) { throw "Collection paths are missing gate: $p4" }
        if (-not $process.Contains($p4)) { throw "Process is missing gate: $p4" }
    }
}

Write-Output "Document traceability passed: 00 -> PRD-01..10 -> AC-01..11 -> architecture/process -> TC-01..11; $($requiredP2Sources.Count) required source routes and $($representativeTools.Count) representative tools have structured P2-G7 evidence."
