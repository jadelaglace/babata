param([string]$RepoRoot)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Join-Path $PSScriptRoot '..'
}
$repo = (Resolve-Path -LiteralPath $RepoRoot).Path

function Read-Required {
    param([string]$Relative)
    $path = Join-Path $repo $Relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing governance file: $Relative" }
    return Get-Content -Raw -Encoding utf8 -LiteralPath $path
}

function Assert-Contains {
    param([string]$Text, [string]$Value, [string]$Label)
    if (-not $Text.Contains($Value)) { throw "Missing ${Label}: $Value" }
}

function Get-BoundedBlock {
    param([string]$Text, [string]$Start, [string]$End, [string]$Label)
    $starts = @([regex]::Matches($Text, [regex]::Escape($Start)))
    $ends = @([regex]::Matches($Text, [regex]::Escape($End)))
    if ($starts.Count -ne 1 -or $ends.Count -ne 1) { throw "$Label must have exactly one bounded hook" }
    if ($ends[0].Index -le $starts[0].Index) { throw "$Label hook markers are reversed" }
    return $Text.Substring($starts[0].Index, $ends[0].Index + $End.Length - $starts[0].Index)
}

function Assert-RecoveryOrder {
    param([string]$Block, [string]$Label)
    $goal = $Block.IndexOf('Goal/task-state API', [StringComparison]::Ordinal)
    $plan = $Block.IndexOf('00_docs/04_process/04_c_ACTIVE_PLAN.md', [StringComparison]::Ordinal)
    if ($plan -lt 0) {
        $plan = $Block.IndexOf('04_process/04_c_ACTIVE_PLAN.md', [StringComparison]::Ordinal)
    }
    if ($goal -lt 0 -or $plan -lt 0 -or $goal -ge $plan) {
        throw "$Label must place Goal/task-state API before 04_c Active Plan"
    }
}

$agents = Read-Required 'AGENTS.md'
$readme = Read-Required 'README.md'
$index = Read-Required '00_docs\README.md'
$requirements = Read-Required '00_docs\00_requirements\00_a_REQUIREMENTS.md'
$process = Read-Required '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
$usage = Read-Required '00_docs\04_process\04_b_USAGE_STATUS.md'
$active = Read-Required '00_docs\04_process\04_c_ACTIVE_PLAN.md'
$archive = Read-Required '00_docs\90_archive\2026-08-23_P0-P9_CLOSEOUT.md'

$agentsHook = Get-BoundedBlock $agents '<!-- BABATA-RECOVERY-HOOK: v1 -->' '<!-- /BABATA-RECOVERY-HOOK: v1 -->' 'AGENTS.md'
$readmeHook = Get-BoundedBlock $readme '<!-- BABATA-RECOVERY-HOOK: v1 -->' '<!-- /BABATA-RECOVERY-HOOK: v1 -->' 'README.md'
$indexHook = Get-BoundedBlock $index '<!-- BABATA-DOCS-RECOVERY-ENTRY: v2 -->' '## 1. 当前权威链' 'DOC-INDEX'
Assert-RecoveryOrder $agentsHook 'AGENTS.md'
Assert-RecoveryOrder $readmeHook 'README.md'
Assert-RecoveryOrder $indexHook 'DOC-INDEX'

foreach ($hook in @($agentsHook, $readmeHook, $indexHook)) {
    Assert-Contains $hook 'CURRENT-ACTIVE' 'unique active marker instruction'
    if ($hook -notmatch '(?i)unknown') { throw 'Recovery hook must preserve unknown semantics' }
}
Assert-Contains $agentsHook '00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md' 'AGENTS lifecycle authority'
Assert-Contains $readmeHook '00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md' 'README lifecycle authority'
Assert-Contains $indexHook 'DOC-PROCESS' 'index lifecycle authority'

$markers = @([regex]::Matches($active, '(?m)^<!-- CURRENT-ACTIVE: ([A-Za-z0-9-]+|none) -->\r?$'))
if ($markers.Count -ne 1) { throw "Active Plan must have exactly one CURRENT-ACTIVE marker; found $($markers.Count)" }
$currentId = $markers[0].Groups[1].Value
$lineCount = @($active -split '\r?\n').Count
if ($lineCount -gt 120 -or $active.Length -gt 6000) {
    throw "Active Plan exceeds hot-path bounds: $lineCount lines / $($active.Length) characters"
}
if ($currentId -ne 'none') {
    Assert-Contains $active "### $currentId" 'active item heading'
    foreach ($field in @(
        '- 来源锚点：', '- transition：', '- transition evidence：', '- 用户目标：',
        '- 当前状态：', '- 声明终端：', '- 受保护边界：', '- 下一步：', '- 恢复入口：'
    )) {
        Assert-Contains $active $field "active item field"
    }
}
Assert-Contains $active '## 2. 下次开工队列（禁止恢复时自动执行）' 'bounded queue heading'
Assert-Contains $active 'DOC-PROCESS' 'active-plan lifecycle authority link'

foreach ($marker in @(
    '完整恢复边界包括',
    'Goal/task-state API',
    'requires-explicit-resume',
    '压缩摘要、交接文字、旧消息和 tool checkpoint 只能定位',
    '已有 live completion',
    '只补终端维护',
    '普通同步命令/API 明确失败',
    '成功且无后续义务的临时项',
    'CURRENT-ACTIVE: none'
)) {
    Assert-Contains $process $marker 'recovery lifecycle contract'
}
Assert-Contains $requirements '已关闭工作不得因摘要、压缩或旧状态重跑' 'requirements replay prohibition'
Assert-Contains $usage '这些项目不是 Active Plan 队列' 'usage/queue boundary'
Assert-Contains $archive '不从本文恢复任务' 'archive replay boundary'

Write-Output 'Intent and plan governance passed: root hooks route Goal first to 04_c Active Plan, DOC-PROCESS owns lifecycle, the hot path is bounded and singular, recovery selection is conservative, terminal work is replay-protected, and the archive cannot schedule work.'
