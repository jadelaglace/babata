param([string]$DocsRoot)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
    $DocsRoot = Join-Path $PSScriptRoot '..\00_docs'
}
$docs = (Resolve-Path -LiteralPath $DocsRoot).Path

function Read-Doc {
    param([string]$Relative)
    $path = Join-Path $docs $Relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing document: $Relative" }
    return Get-Content -Raw -Encoding utf8 -LiteralPath $path
}

function Assert-Contains {
    param([string]$Text, [string]$Value, [string]$Label)
    if (-not $Text.Contains($Value)) { throw "Missing ${Label}: $Value" }
}

$requirements = Read-Doc '00_requirements/00_a_REQUIREMENTS.md'
$prd = Read-Doc '01_prd/01_a_PRD.md'
$acceptance = Read-Doc '02_acceptance/02_a_ACCEPTANCE_CRITERIA.md'
$architecture = Read-Doc '03_architecture/03_a_ARCHITECTURE.md'
$process = Read-Doc '04_process/04_a_DEVELOPMENT_PROCESS.md'
$tests = Read-Doc '05_tests/05_a_TEST_CASES.md'
$archive = Read-Doc '90_archive/2026-08-23_P0-P9_CLOSEOUT.md'

foreach ($quote in @(
    '> 好的 我现在认为 整个Compass都走偏了 协议过度严格 但是没有实际工具技术支撑 总想着怎么接口化 大宏观 淡忘了目的的感觉',
    '> 第三大界：我的认知轨迹（日志与感悟层）',
    '> 你处理事情，走一步卡一步，层层设卡。',
    '> 基于之前的原始文件要完整留存，Babata 给它导出一个兼容维护层、独立主权库，保留记忆宫殿。',
    '> 微调一下我的想法： 1可以去重 保留最旧的'
)) {
    Assert-Contains $archive $quote 'selected verbatim evidence'
}
foreach ($marker in @(
    'ARCHIVE-STATE: closed-read-only',
    '不再拥有当前需求、产品行为、架构、状态、计划或',
    'git show 9182a77:<旧相对路径>',
    '精选用户原话'
)) {
    Assert-Contains $archive $marker 'archive provenance boundary'
}

if ($requirements -match '(?m)^>') {
    throw 'Current requirements must not present curated interpretation as verbatim quotation'
}
foreach ($marker in @(
    '本文同时保存仍有效的用户意图和当前解释后的产品需求',
    '当前对话中用户对 Backs、归档分析和文档聚焦的最新纠偏',
    'adopted requirement / not implemented'
)) {
    Assert-Contains $requirements $marker 'requirements attribution boundary'
}

$stable = [ordered]@{
    requirements = $requirements
    prd = $prd
    acceptance = $acceptance
    architecture = $architecture
    process = $process
    tests = $tests
}
foreach ($entry in $stable.GetEnumerator()) {
    if ($entry.Value -match '(?i)D:\\BabataData\\[^\r\n]*20[0-9]{6}|mba-[a-z0-9-]+-20[0-9]{6}|session_[0-9A-Z]{20,}') {
        throw "Stable authority contains a concrete runtime identity: $($entry.Key)"
    }
    if ($entry.Value -match '(?i)\bIssue\s*#\d+\b') {
        throw "Stable authority contains Issue-specific delivery history: $($entry.Key)"
    }
}

$allDocs = (Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md' | ForEach-Object {
    Get-Content -Raw -Encoding utf8 -LiteralPath $_.FullName
}) -join "`n"
foreach ($pattern in @(
    '(?i)ghp_[A-Za-z0-9]{20,}',
    '(?i)sk-[A-Za-z0-9_-]{20,}',
    '(?i)Bearer\s+[A-Za-z0-9._-]{20,}',
    '(?i)restic[_ -]?password\s*[:=]\s*\S+'
)) {
    if ($allDocs -match $pattern) { throw "Sensitive token pattern found: $pattern" }
}

Write-Output 'Document provenance passed: selected user wording is preserved verbatim in the immutable closeout archive, current requirements remain interpreted and attributed, exact originals are recoverable from commit 9182a77, stable authorities contain no run-specific identities, and sensitive token patterns are absent.'
