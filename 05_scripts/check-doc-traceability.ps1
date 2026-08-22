param(
    [string]$DocsRoot,
    [string]$RepoReadmePath
)

$ErrorActionPreference = 'Stop'

function Read-RequiredFile {
    param([string]$RelativePath)
    $path = Join-Path $docs $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing authority document: $RelativePath"
    }
    return Get-Content -Raw -Encoding utf8 -LiteralPath $path
}

function Assert-Contains {
    param([string]$Text, [string]$Expected, [string]$Label)
    if (-not $Text.Contains($Expected)) {
        throw "Missing ${Label}: $Expected"
    }
}

if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
    $DocsRoot = Join-Path $PSScriptRoot '..\00_docs'
}
$docs = (Resolve-Path -LiteralPath $DocsRoot).Path

$currentDocs = @(
    @{ path = 'README.md'; id = 'DOC-INDEX'; role = 'index' },
    @{ path = '00_requirements/00_a_REQUIREMENTS.md'; id = 'DOC-REQ'; role = 'current-intent-and-requirements' },
    @{ path = '01_prd/01_a_PRD.md'; id = 'DOC-PRD'; role = 'product-behavior' },
    @{ path = '02_acceptance/02_a_ACCEPTANCE_CRITERIA.md'; id = 'DOC-AC'; role = 'acceptance' },
    @{ path = '03_architecture/03_a_ARCHITECTURE.md'; id = 'DOC-ARCH'; role = 'architecture' },
    @{ path = '03_architecture/03_b_SOURCE_ROUTE_REGISTRY.md'; id = 'DOC-ROUTES'; role = 'source-route-research' },
    @{ path = '04_process/04_a_DEVELOPMENT_PROCESS.md'; id = 'DOC-PROCESS'; role = 'operations-delivery-and-recovery' },
    @{ path = '04_process/04_b_USAGE_STATUS.md'; id = 'DOC-USAGE'; role = 'usage-status' },
    @{ path = '04_process/04_c_ACTIVE_PLAN.md'; id = 'DOC-ACTIVE-PLAN'; role = 'active-plan-progress' },
    @{ path = '05_tests/05_a_TEST_CASES.md'; id = 'DOC-TC'; role = 'verification-procedure' }
)

$archive = @{ path = '90_archive/2026-08-23_P0-P9_CLOSEOUT.md'; id = 'DOC-P0-P9-ARCHIVE'; role = 'immutable-history' }
$index = Read-RequiredFile 'README.md'

foreach ($entry in $currentDocs + @($archive)) {
    $text = Read-RequiredFile $entry.path
    Assert-Contains $text "DOC-ID: $($entry.id)" "stable document ID in $($entry.path)"
    Assert-Contains $text "DOC-AUTHORITY-BOUNDARY: $($entry.role)" "authority role in $($entry.path)"
    $registryPattern = '(?m)^\|[^\r\n]*' + [regex]::Escape("``$($entry.id)``") + '[^\r\n]*\|\r?$'
    $registryMatches = @([regex]::Matches($index, $registryPattern))
    if ($registryMatches.Count -ne 1) {
        throw "Document registry must contain exactly one row for $($entry.id); found $($registryMatches.Count)"
    }
    Assert-Contains $index $entry.path "registry path for $($entry.id)"
}

$activeMarkdown = @(Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md' | Where-Object {
    $_.FullName -notlike "*\90_archive\*"
})
if ($activeMarkdown.Count -ne 10) {
    throw "Expected 10 active Markdown documents, found $($activeMarkdown.Count)"
}

$expectedRelativePaths = @($currentDocs | ForEach-Object { $_.path.Replace('/', [IO.Path]::DirectorySeparatorChar) })
foreach ($file in $activeMarkdown) {
    $relative = [IO.Path]::GetRelativePath($docs, $file.FullName)
    if ($relative -notin $expectedRelativePaths) {
        throw "Unregistered active document: $relative"
    }
}

$docIdOwners = @{}
foreach ($file in Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md') {
    $text = Get-Content -Raw -Encoding utf8 -LiteralPath $file.FullName
    foreach ($match in [regex]::Matches($text, '<!--\s*DOC-ID:\s*([A-Z0-9-]+)\s*-->')) {
        $id = $match.Groups[1].Value
        if ($docIdOwners.ContainsKey($id)) {
            throw "Duplicate stable document ID '$id'"
        }
        $docIdOwners[$id] = $file.FullName
    }
}

$requirements = Read-RequiredFile '00_requirements/00_a_REQUIREMENTS.md'
$prd = Read-RequiredFile '01_prd/01_a_PRD.md'
$acceptance = Read-RequiredFile '02_acceptance/02_a_ACCEPTANCE_CRITERIA.md'
$architecture = Read-RequiredFile '03_architecture/03_a_ARCHITECTURE.md'
$routes = Read-RequiredFile '03_architecture/03_b_SOURCE_ROUTE_REGISTRY.md'
$process = Read-RequiredFile '04_process/04_a_DEVELOPMENT_PROCESS.md'
$usage = Read-RequiredFile '04_process/04_b_USAGE_STATUS.md'
$activePlan = Read-RequiredFile '04_process/04_c_ACTIVE_PLAN.md'
$tests = Read-RequiredFile '05_tests/05_a_TEST_CASES.md'
$archiveText = Read-RequiredFile $archive.path

foreach ($id in 1..10) {
    $marker = 'PRD-{0:D2}' -f $id
    Assert-Contains $prd $marker 'PRD behavior marker'
    Assert-Contains $acceptance $marker 'acceptance PRD trace'
}
foreach ($id in 1..11) {
    $ac = 'AC-{0:D2}' -f $id
    $tc = 'TC-{0:D2}' -f $id
    Assert-Contains $acceptance $ac 'acceptance marker'
    Assert-Contains $architecture $ac 'architecture acceptance trace'
    Assert-Contains $tests $ac 'test acceptance trace'
    Assert-Contains $tests $tc 'test case marker'
}

foreach ($marker in @(
    'adopted requirement / not implemented',
    '精确重复',
    '归档分析/',
    'TC-11 独立系统回执'
)) {
    Assert-Contains $requirements $marker 'current requirement boundary'
}
foreach ($marker in @(
    'ARCHIVE-STATE: closed-read-only',
    '9182a77',
    '完整轮次',
    'Backs 最新纠偏',
    '明确未完成、暂缓或跳过'
)) {
    Assert-Contains $archiveText $marker 'closeout archive evidence'
}
foreach ($marker in @(
    'Goal/task-state API',
    '04_process/04_c_ACTIVE_PLAN.md',
    'CURRENT-ACTIVE',
    'DOC-P0-P9-ARCHIVE'
)) {
    Assert-Contains $index $marker 'document control-plane marker'
}
foreach ($marker in @(
    '完整轮次为最小收敛单位',
    'promoted-for-fix',
    'Goal/task-state API',
    'requires-explicit-resume',
    '已有 live completion'
)) {
    Assert-Contains $process $marker 'operations/recovery contract'
}
foreach ($marker in @('唯一当前使用', 'outputs.obsidian', 'not-run', '9182a77')) {
    Assert-Contains $usage $marker 'usage status boundary'
}
Assert-Contains $activePlan '<!-- CURRENT-ACTIVE:' 'active-plan marker'

foreach ($source in @(
    'source.feishu', 'source.yuque', 'source.onenote', 'source.evernote',
    'source.wechat_favorites', 'source.wechat_articles', 'source.wechat_channels',
    'source.wechat_chats', 'source.zhihu', 'source.bilibili', 'source.xiaohongshu',
    'source.douyin', 'source.browser_bookmarks', 'source.browser_pages', 'source.doubao',
    'source.kimi', 'source.chatgpt', 'source.local_files', 'source.youtube', 'source.first_party'
)) {
    if (@([regex]::Matches($routes, "(?m)^\| $([regex]::Escape($source)) \|")).Count -ne 1) {
        throw "Route registry must contain exactly one row for $source"
    }
}
foreach ($tool in @('tool.lark_cli', 'tool.agent_browser', 'tool.browser_use', 'tool.codex_chrome', 'tool.opencli')) {
    if (@([regex]::Matches($routes, "(?m)^\| $([regex]::Escape($tool)) \|")).Count -ne 1) {
        throw "Route registry must contain exactly one row for $tool"
    }
}
if ($routes -match '(?m)^\| source\.[^|]+\|[^\r\n]*\| (available|planned) \|') {
    throw 'Route registry contains an invalid runtime status'
}

if ([string]::IsNullOrWhiteSpace($RepoReadmePath)) {
    $candidate = Join-Path (Split-Path -Parent $docs) 'README.md'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { $RepoReadmePath = $candidate }
}
if (-not (Test-Path -LiteralPath $RepoReadmePath -PathType Leaf)) {
    throw "Missing repository README: $RepoReadmePath"
}
$repoReadme = Get-Content -Raw -Encoding utf8 -LiteralPath $RepoReadmePath
foreach ($value in @('00_docs/04_process/04_c_ACTIVE_PLAN.md', '00_docs/00_requirements/00_a_REQUIREMENTS.md')) {
    Assert-Contains $repoReadme $value 'repository README authority link'
}

Write-Output 'Document traceability passed: 10 active authorities plus one immutable P0-P9 archive; continuous numbering, unique DOC-IDs, PRD-01..10, AC/TC-01..11, Backs boundaries, route rows, and root navigation are consistent.'
