param(
    [string]$DocsRoot,
    [string]$RepoReadmePath
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

function Assert-NotMatches {
    param(
        [Parameter(Mandatory)]
        [string]$Text,
        [Parameter(Mandatory)]
        [string]$Pattern,
        [Parameter(Mandatory)]
        [string]$Label
    )

    if ($Text -match $Pattern) {
        throw "Document authority boundary violation: $Label"
    }
}

if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
    $DocsRoot = Join-Path $PSScriptRoot '..\00_docs'
}
$docs = (Resolve-Path -LiteralPath $DocsRoot).Path
if ([string]::IsNullOrWhiteSpace($RepoReadmePath)) {
    $candidateRepoReadme = Join-Path (Split-Path -Parent $docs) 'README.md'
    if (Test-Path -LiteralPath $candidateRepoReadme -PathType Leaf) {
        $RepoReadmePath = $candidateRepoReadme
    }
}

$requiredMarkers = @(
    @('00_requirements/00_b_USER_WORDING.md', '# Babata 当前有效用户意图集'),
    @('00_requirements/00_c_USER_WORDING_RECOVERY.md', '# Babata 用户原话与必要 Agent 上下文恢复账本'),
    @('00_requirements/00_a_REQUIREMENTS.md', '[00_c_USER_WORDING_RECOVERY.md](00_c_USER_WORDING_RECOVERY.md)'),
    @('01_prd/01_a_PRD.md', 'PRD-10'),
    @('02_acceptance/02_a_ACCEPTANCE_CRITERIA.md', 'AC-11'),
    @('03_architecture/03_a_ARCHITECTURE.md', 'AC-11'),
    @('03_architecture/03_b_P2_SYSTEM_SKELETON.md', '## 12. 交付权威边界'),
    @('03_architecture/03_c_P3_RAW_FOUNDATION.md', '交付与验证边界'),
    @('03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md', 'P2-G7: passed'),
    @('03_architecture/03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md', 'P6-BLUEPRINT-STATUS: adopted-design-baseline'),
    @('04_process/04_a_DEVELOPMENT_PROCESS.md', 'P2-G7'),
    @('04_process/04_b_USAGE_STATUS.md', '唯一的当前使用与交付状态权威'),
    @('04_process/04_f_ACTIVE_PLAN.md', '唯一的当前执行计划与进度控制面'),
    @('04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md', '三层恢复结构'),
    @('05_tests/05_a_TEST_CASES.md', 'GT-P2-07')
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

$requiredRoleMarkers = @(
    @('README.md', 'DOC-AUTHORITY-BOUNDARY: index'),
    @('00_requirements/00_b_USER_WORDING.md', 'DOC-AUTHORITY-BOUNDARY: curated-current-intent'),
    @('00_requirements/00_c_USER_WORDING_RECOVERY.md', 'DOC-AUTHORITY-BOUNDARY: intent-recovery-evidence'),
    @('00_requirements/00_a_REQUIREMENTS.md', 'DOC-AUTHORITY-BOUNDARY: current-requirements'),
    @('01_prd/01_a_PRD.md', 'DOC-AUTHORITY-BOUNDARY: product-behavior'),
    @('02_acceptance/02_a_ACCEPTANCE_CRITERIA.md', 'DOC-AUTHORITY-BOUNDARY: acceptance'),
    @('03_architecture/03_a_ARCHITECTURE.md', 'DOC-AUTHORITY-BOUNDARY: architecture'),
    @('03_architecture/03_b_P2_SYSTEM_SKELETON.md', 'DOC-AUTHORITY-BOUNDARY: architecture-design-record'),
    @('03_architecture/03_c_P3_RAW_FOUNDATION.md', 'DOC-AUTHORITY-BOUNDARY: architecture-supplement'),
    @('03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md', 'DOC-AUTHORITY-BOUNDARY: source-route-research'),
    @('03_architecture/03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md', 'DOC-AUTHORITY-BOUNDARY: adopted-design-baseline'),
    @('03_architecture/03_f_EXTERNAL_SOVEREIGN_NAVIGATOR_CANDIDATE.md', 'DOC-AUTHORITY-BOUNDARY: architecture-candidate'),
    @('03_architecture/03_g_C1B_C2B_MODALITY_LADDER.md', 'DOC-AUTHORITY-BOUNDARY: architecture-supplement'),
    @('04_process/04_a_DEVELOPMENT_PROCESS.md', 'DOC-AUTHORITY-BOUNDARY: delivery-process'),
    @('04_process/04_b_USAGE_STATUS.md', 'DOC-AUTHORITY-BOUNDARY: usage-status'),
    @('04_process/04_c_MBA_C2B_ROLLOUT.md', 'DOC-AUTHORITY-BOUNDARY: delivery-plan'),
    @('04_process/04_f_ACTIVE_PLAN.md', 'DOC-AUTHORITY-BOUNDARY: active-plan-progress'),
    @('04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md', 'DOC-AUTHORITY-BOUNDARY: delivery-governance'),
    @('05_tests/05_a_TEST_CASES.md', 'DOC-AUTHORITY-BOUNDARY: verification-procedure')
)
foreach ($check in $requiredRoleMarkers) {
    $path = Join-Path $docs $check[0]
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing authority document: $($check[0])"
    }
    if (-not (Select-String -SimpleMatch $check[1] -Path $path -Quiet)) {
        throw "Missing document authority role '$($check[1])' in $($check[0])"
    }
}

$requiredDocIds = @(
    @('README.md', 'DOC-INDEX'),
    @('00_requirements/00_b_USER_WORDING.md', 'DOC-WORDING'),
    @('00_requirements/00_c_USER_WORDING_RECOVERY.md', 'DOC-WORDING-RECOVERY'),
    @('00_requirements/00_a_REQUIREMENTS.md', 'DOC-REQ'),
    @('01_prd/01_a_PRD.md', 'DOC-PRD'),
    @('02_acceptance/02_a_ACCEPTANCE_CRITERIA.md', 'DOC-AC'),
    @('03_architecture/03_a_ARCHITECTURE.md', 'DOC-ARCH'),
    @('03_architecture/03_b_P2_SYSTEM_SKELETON.md', 'DOC-ARCH-SKELETON'),
    @('03_architecture/03_c_P3_RAW_FOUNDATION.md', 'DOC-ARCH-RAW'),
    @('03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md', 'DOC-ROUTES'),
    @('03_architecture/03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md', 'DOC-KNOWLEDGE-UNIVERSE'),
    @('03_architecture/03_f_EXTERNAL_SOVEREIGN_NAVIGATOR_CANDIDATE.md', 'DOC-EXT-SOVEREIGN-NAVIGATOR-CANDIDATE'),
    @('03_architecture/03_g_C1B_C2B_MODALITY_LADDER.md', 'DOC-MODALITY-LADDER'),
    @('04_process/04_a_DEVELOPMENT_PROCESS.md', 'DOC-PROCESS'),
    @('04_process/04_b_USAGE_STATUS.md', 'DOC-USAGE'),
    @('04_process/04_c_MBA_C2B_ROLLOUT.md', 'DOC-MBA-ROLLOUT'),
    @('04_process/04_f_ACTIVE_PLAN.md', 'DOC-ACTIVE-PLAN'),
    @('04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md', 'DOC-INTENT-PLAN-GOVERNANCE'),
    @('05_tests/05_a_TEST_CASES.md', 'DOC-TC')
)
$index = Get-Content -Raw -Encoding utf8 (Join-Path $docs 'README.md')
$registryTableLines = @($index -split "`r?`n" | Where-Object { $_.Trim().StartsWith('|') })
foreach ($entry in $requiredDocIds) {
    $path = Join-Path $docs $entry[0]
    $marker = "DOC-ID: $($entry[1])"
    if (-not (Select-String -SimpleMatch $marker -Path $path -Quiet)) {
        throw "Missing stable document ID '$($entry[1])' in $($entry[0])"
    }
    $matchingRegistryRows = @($registryTableLines | Where-Object {
        $_.Contains("``$($entry[1])``")
    })
    if ($matchingRegistryRows.Count -ne 1) {
        throw "Document registry must contain exactly one row for stable ID: $($entry[1])"
    }
    if (-not $matchingRegistryRows[0].Contains($entry[0])) {
        throw "Document registry row for $($entry[1]) is missing current path: $($entry[0])"
    }
}
foreach ($marker in @('DOC-REGISTRY: v1', '## 5. 核心术语词典', 'Product behavior', 'Full-scope run')) {
    if (-not $index.Contains($marker)) {
        throw "Document control plane is missing registry/glossary marker: $marker"
    }
}

$namingRules = @(
    @{ directory = '00_requirements'; pattern = '^(?:00_[bc]_USER_WORDING(?:_RECOVERY)?|00_a_REQUIREMENTS)\.md$' },
    @{ directory = '01_prd'; pattern = '^01_a_PRD\.md$' },
    @{ directory = '02_acceptance'; pattern = '^02_a_ACCEPTANCE_CRITERIA\.md$' },
    @{ directory = '03_architecture'; pattern = '^03_[a-z]_[A-Z0-9_]+\.md$' },
    @{ directory = '04_process'; pattern = '^04_[a-z]_[A-Z0-9_]+\.md$' },
    @{ directory = '05_tests'; pattern = '^05_a_TEST_CASES\.md$' }
)
foreach ($rule in $namingRules) {
    $directory = Join-Path $docs $rule.directory
    foreach ($file in Get-ChildItem -LiteralPath $directory -File -Filter '*.md') {
        if ($file.Name -notmatch $rule.pattern) {
            throw "Invalid intra-folder document number/name: $($rule.directory)/$($file.Name)"
        }
    }
}

$docIdOwners = @{}
foreach ($file in Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md') {
    $text = Get-Content -Raw -Encoding utf8 -LiteralPath $file.FullName
    foreach ($match in [regex]::Matches($text, '<!--\s*DOC-ID:\s*([A-Z0-9-]+)\s*-->')) {
        $id = $match.Groups[1].Value
        if ($docIdOwners.ContainsKey($id)) {
            throw "Duplicate stable document ID '$id' in $($docIdOwners[$id]) and $($file.FullName)"
        }
        $docIdOwners[$id] = $file.FullName
    }
}

$requirements = Get-Content -Raw -Encoding utf8 (Join-Path $docs '00_requirements/00_a_REQUIREMENTS.md')
$prd = Get-Content -Raw -Encoding utf8 (Join-Path $docs '01_prd/01_a_PRD.md')
$acceptance = Get-Content -Raw -Encoding utf8 (Join-Path $docs '02_acceptance/02_a_ACCEPTANCE_CRITERIA.md')
$architecture = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_a_ARCHITECTURE.md')
$tests = Get-Content -Raw -Encoding utf8 (Join-Path $docs '05_tests/05_a_TEST_CASES.md')
$process = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_a_DEVELOPMENT_PROCESS.md')
$usage = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_b_USAGE_STATUS.md')
$deliveryPlans = [ordered]@{
    mba_rollout = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_c_MBA_C2B_ROLLOUT.md')
}

$stableProductDocuments = [ordered]@{
    requirements = $requirements
    prd = $prd
    acceptance = $acceptance
    process = $process
    governance = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md')
    tests = $tests
}
foreach ($entry in $stableProductDocuments.GetEnumerator()) {
    Assert-NotMatches $entry.Value '(?m)\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b' "dated execution result in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)\bIssue\s*#\d+' "Issue-specific execution state in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)D:\\BabataData\\|mba-[a-z0-9-]+-20[0-9]{6}' "concrete runtime batch path in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)obsidian://open\?' "concrete live Obsidian URI in $($entry.Key)"
}
foreach ($entry in $deliveryPlans.GetEnumerator()) {
    Assert-NotMatches $entry.Value '(?m)\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b' "dated execution result in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)\bIssue\s*#\d+' "Issue-specific execution state in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)D:\\BabataData\\|mba-[a-z0-9-]+-20[0-9]{6}' "concrete runtime batch path in $($entry.Key)"
    Assert-NotMatches $entry.Value '(?i)obsidian://open\?' "concrete live Obsidian URI in $($entry.Key)"
}
Assert-NotMatches $deliveryPlans.mba_rollout '(?i)Bilibili|[0-9]+\s*个[^\r\n|]{0,30}暂缓' 'concrete deferred scope in MBA rollout plan'
if ([string]::IsNullOrWhiteSpace($RepoReadmePath) -or -not (Test-Path -LiteralPath $RepoReadmePath -PathType Leaf)) {
    throw "Missing repository README: $RepoReadmePath"
}
$repoReadme = Get-Content -Raw -Encoding utf8 -LiteralPath $RepoReadmePath
Assert-NotMatches $repoReadme '(?im)\bP[0-9]+(?:\.[0-9]+)?\b[^\r\n|]{0,80}\b(?:已完成|完成|进行中|未开始)\b' 'live phase snapshot in repository README'
Assert-NotMatches $repoReadme '(?m)\b[0-9]+/[0-9]+\b' 'usage ratio in repository README'

foreach ($marker in @('试跑 / dry-run', '试点 / pilot', '模板 / profile', '明确范围全量运行')) {
    if (-not $prd.Contains($marker)) {
        throw "PRD is missing reusable product-behavior boundary: $marker"
    }
}
if (-not $prd.Contains('具体试跑、试点、模板接受和全量运行结果统一记录在 usage/evidence，不反向改写产品行为')) {
    throw 'PRD is missing product behavior versus usage result boundary'
}
if (-not $process.Contains('某个明确范围全量跑通是 usage 事实，不能反向写入 PRD 或改变 phase 定义')) {
    throw 'Process is missing phase versus product authority boundary'
}
if (-not $tests.Contains('本文定义可重复的验证场景、步骤和预期结果。它不维护“当前通过/失败”')) {
    throw 'Tests are missing reusable-procedure versus execution-result boundary'
}
$liveUris = @([regex]::Matches($usage, 'obsidian://open\?[^\s`)]+'))
if ($liveUris.Count -lt 1) {
    throw 'Usage status is missing a current live Obsidian URI'
}

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

$skeleton = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_b_P2_SYSTEM_SKELETON.md')
$rawBlueprint = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_c_P3_RAW_FOUNDATION.md')
$sourceResearch = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md')

Assert-NotMatches $sourceResearch '(?i)\bIssue\s*#\d+|\bP8\.[0-9]+\b|\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b|mba-[a-z0-9-]+-20[0-9]{6}' 'usage or batch history in source route registry'
Assert-NotMatches $sourceResearch '用户(?:整体)?暂缓|当前分母|不进入[^\r\n|]{0,20}分母' 'user-scope status in source route registry'
foreach ($entry in ([ordered]@{
    architecture = $architecture
    process = $process
    source_routes = $sourceResearch
}).GetEnumerator()) {
    Assert-NotMatches $entry.Value '(?i)\bavailable\b' "legacy available route status in $($entry.Key)"
}

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
    if ($row.current_status -notin @('enabled', 'disabled', 'unavailable', 'absent')) {
        throw "Source '$($row.source_id)' has invalid current status: $($row.current_status)"
    }
    if ($evidenceLevel -lt 3 -and $row.current_status -eq 'enabled') {
        throw "Source '$($row.source_id)' is below E3 and cannot be enabled"
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
    if (-not $process.Contains($p2)) { throw "Process is missing gate: $p2" }
    if (-not $tests.Contains($p2)) { throw "Tests are missing gate: $p2" }

    if ($id -le 6) {
        $p3 = "P3-G$id"
        if (-not $process.Contains($p3)) { throw "Process is missing gate: $p3" }

        $p4 = "P4-G$id"
        if (-not $process.Contains($p4)) { throw "Process is missing gate: $p4" }
    }
}

foreach ($entry in ([ordered]@{
    p2_design = $skeleton
    p3_architecture = $rawBlueprint
    mba_rollout = $deliveryPlans.mba_rollout
}).GetEnumerator()) {
    Assert-NotMatches $entry.Value '(?m)^\s*(?:- |\| )P[234]-G[1-7]\b' "competing phase-gate definition in $($entry.Key)"
}

Write-Output "Document traceability passed: DOC-WORDING-RECOVERY -> DOC-WORDING -> DOC-REQ -> PRD-01..10 -> AC-01..11 -> architecture/process/active-plan -> TC-01..11; $($requiredP2Sources.Count) required source routes and $($representativeTools.Count) representative tools have structured P2-G7 evidence."
