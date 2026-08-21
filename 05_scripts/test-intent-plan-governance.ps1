[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-intent-plan-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-intent-plan-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
    'AGENTS.md',
    'README.md',
    '00_docs\README.md',
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\00_requirements\00_b_USER_WORDING.md',
    '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md',
    '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md',
    '00_docs\04_process\04_b_USAGE_STATUS.md',
    '00_docs\04_process\04_f_ACTIVE_PLAN.md',
    '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md'
)

function New-TestRepo {
    param([string]$Name)
    $caseRoot = Join-Path $root $Name
    foreach ($relativePath in $requiredFiles) {
        $source = Join-Path $repo $relativePath
        $target = Join-Path $caseRoot $relativePath
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $target
    }
    return $caseRoot
}

function Replace-Once {
    param([string]$CaseRoot, [string]$RelativePath, [string]$Value, [string]$Replacement)
    $path = Join-Path $CaseRoot $RelativePath
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $first = $text.IndexOf($Value, [StringComparison]::Ordinal)
    if ($first -lt 0) { throw "Mutation fixture is missing ${RelativePath}: $Value" }
    $mutated = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Value.Length)
    Set-Content -LiteralPath $path -Value $mutated -Encoding utf8
}

function Resolve-RecoveryOptimizationCaptureFixture {
    param([string]$CaseRoot)
    $path = Join-Path $CaseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $text = $text.Replace(
        '| 恢复钩子、启动慢、简化、skill | 2026-08-21 | `active` | `AP-20260821-01`；优化恢复热路径并同步 product-docs skill |',
        '| 恢复钩子、启动慢、简化、skill | 2026-08-21 | `resolved` | `resolved_by`: test terminal |'
    )
    $marker = '<!-- DOCS-FIRST-CAPTURE: DFC-20260821-01; schema=v1 -->'
    $markerIndex = $text.IndexOf($marker, [StringComparison]::Ordinal)
    if ($markerIndex -lt 0) { throw 'Recovery optimization capture fixture marker is missing.' }
    $prefix = $text.Substring(0, $markerIndex)
    $capture = $text.Substring($markerIndex)
    $capture = $capture.Replace('状态：`active`。', '状态：`resolved`。')
    $capture = $capture.Replace(
        '`resolved_by`：留空，等待仓库 PR 与共享 skill 验证完成。',
        '`resolved_by`：test terminal。'
    )
    Set-Content -LiteralPath $path -Value ($prefix + $capture) -Encoding utf8
}

function Add-ActivePlanFixture {
    param([string]$CaseRoot)
    Resolve-RecoveryOptimizationCaptureFixture $CaseRoot
    $planPath = Join-Path $CaseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
    $plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8
    $planPattern = '(?ms)^## 1\. 当前活动项（恢复时先读，最多一个）.*?(?=^## 2\. 下次开工队列（禁止恢复时自动执行）)'
    $planFixture = @'
## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260815-05 -->

恢复后只执行唯一当前活动项，不从摘要或队列选择工作。

### AP-20260815-05：测试活动项

- 来源锚点：`DFC-20260815-02`、测试来源。
- Goal 锚点：当前 Goal API 返回空值，按 `unknown` 处理；以最新明确用户指令作为持久化 Goal。
- 状态转换类型：`user-explicit-goal-override`
- 状态转换依据：用户明确覆盖原课程 Goal，授权暂停 AP03 并把本治理修复设为唯一 active；这不是 blocker、自主重排或从旧 resolved 项推导出的重开。
- 当前状态：`in_progress`。
- 用户目标：修复 Agent 在信息不完整时的测试治理漏洞。
- 目标终端：测试 checker 与 mutation。
- 不改变：不改变队列项或产品数据。

#### 临时子计划与阶段结论

1. 测试 fixture 只用于覆盖 active-plan schema。

- 下一步：运行测试。
- 证据入口：测试输出。

'@
    $plan = [regex]::Replace($plan, $planPattern, $planFixture, 1)
    Set-Content -LiteralPath $planPath -Value $plan -Encoding utf8

    $recoveryPath = Join-Path $CaseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
    $recovery = Get-Content -LiteralPath $recoveryPath -Raw -Encoding utf8
    $recovery = $recovery.Replace(
        '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `resolved` | `resolved_by`: PR `#153` merged as `a1c6df7`; AP-20260815-05 terminal |',
        '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `active` | 目标去向：`AP-20260815-05` |'
    )
    $recovery = [regex]::Replace(
        $recovery,
        '(?ms)(<!-- DOCS-FIRST-CAPTURE: DFC-20260815-02; schema=v1 -->.*?^状态：)`resolved`(。).*?^`resolved_by`：.*?governance boundary/full gates。\s*',
        ('$1`active`$2' + "`r`n`r`n目标去向：`AP-20260815-05`。`r`n"),
        1
    )
    Set-Content -LiteralPath $recoveryPath -Value $recovery -Encoding utf8
}

function Add-QueueFixture {
    param([string]$CaseRoot)
    Resolve-RecoveryOptimizationCaptureFixture $CaseRoot
    $planPath = Join-Path $CaseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
    $plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8
    $plan = [regex]::Replace(
        $plan,
        '(?ms)(## 1\. 当前活动项（恢复时先读，最多一个）).*?(?=## 2\. 下次开工队列（禁止恢复时自动执行）)',
        '$1' + "`n`n<!-- CURRENT-ACTIVE: none -->`n`n当前无活动项。`n`n",
        1
    )
    $plan = [regex]::Replace(
        $plan,
        '(?ms)(## 2\. 下次开工队列（禁止恢复时自动执行）).*\z',
        ('$1' + "`n`n### AP-20260815-03：测试队列项`n`n- 来源锚点：测试队列。`n- 用户目标：复用既有 MBA C1B/C2B 链路。`n- 当前状态：``queued / paused-by-explicit-goal-override / requires-explicit-resume``。`n- 下一步：等待明确恢复。`n- 恢复入口：DOC-ACTIVE-PLAN。`n"),
        1
    )
    Set-Content -LiteralPath $planPath -Value $plan -Encoding utf8
}

function Replace-CurrentInProgressState {
    param([string]$CaseRoot, [string]$Replacement)
    $path = Join-Path $CaseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $pattern = '(?m)^- 当前状态：`in_progress(?: / [^`]+)*`。\r?$'
    $matches = @([regex]::Matches($text, $pattern))
    if ($matches.Count -ne 1) {
        throw "Mutation fixture requires exactly one in-progress current state, found $($matches.Count)."
    }
    $mutated = [regex]::Replace($text, $pattern, "- 当前状态：``$Replacement``。", 1)
    Set-Content -LiteralPath $path -Value $mutated -Encoding utf8
}

function Assert-CheckerFails {
    param([string]$Name, [string]$Expected, [scriptblock]$Mutate)
    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    $failed = $false
    try { & $checker -RepoRoot $caseRoot | Out-Null }
    catch {
        $failed = $true
        if (-not $_.Exception.Message.Contains($Expected)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failed) { throw "Case '$Name' unexpectedly passed intent/plan governance." }
}

function Assert-CheckerPasses {
    param([string]$Name, [scriptblock]$Mutate)
    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    & $checker -RepoRoot $caseRoot | Out-Null
}

try {
    New-Item -ItemType Directory -Path $root | Out-Null
    & $checker -RepoRoot $repo | Out-Null

    Assert-CheckerFails 'current-intent-restores-verbatim-quotes' 'forbids Markdown quote blocks' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\00_requirements\00_b_USER_WORDING.md') -Value "`n> 整理后的伪逐字原话`n" -Encoding utf8
    }
    Assert-CheckerFails 'current-intent-uses-compound-status' 'current intent uses an invalid base status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_b_USER_WORDING.md' '状态：`stable`' '状态：`stable-with-open-interface`'
    }
    Assert-CheckerFails 'current-intent-topic-loses-metadata' 'current-intent topic 10 is missing' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_b_USER_WORDING.md' '状态：`stable`。归因：`user-curated`。恢复词：`docs-first；完整轮次；可恢复；用人话`。' '当前仍有效。'
    }
    Assert-CheckerFails 'recovery-loses-append-first' 'append-first recovery contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' 'append-first 恢复账本' '终端整理记录'
    }
    Assert-CheckerFails 'recovery-loses-capture-marker' 'malformed docs-first capture marker' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '<!-- DOCS-FIRST-CAPTURE: DFC-20260815-02; schema=v1 -->' '<!-- DOCS-FIRST-CAPTURE: invalid -->'
    }
    Assert-CheckerFails 'recovery-loses-capture-source' 'invalid or empty time/source' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' 'capture time：`2026-08-15T23:43:42.3115117+08:00`。来源：当前用户对话。' 'capture time：`2026-08-15T23:43:42.3115117+08:00`。来源：。'
    }
    Assert-CheckerFails 'recovery-appends-unstructured-latest-entry' 'latest recovery entry is not the structured docs-first capture' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md') -Value "`n### 2099-01-01：遗漏结构`n`n> 新的治理输入。`n" -Encoding utf8
    }
    Assert-CheckerFails 'recovery-index-misses-latest-phrase' 'latest capture must map to exactly one fast-index row' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| 恢复钩子、启动慢、简化、skill |' '| 恢复钩子、启动慢、简化、共享能力 |'
    }
    Assert-CheckerFails 'recovery-index-uses-compound-status' 'recovery index uses an invalid base status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| `active` | 多仓/子 Git' '| `active-with-history` | 多仓/子 Git'
    }
    Assert-CheckerFails 'recovery-duplicates-retrieval-phrase' 'requires 3-8 unique retrieval phrases' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '极简检索词：`先做第二个解`；`MBA 全按第一个解`；`Obsidian 模板更新`；`Goal 不能恢复`；`统一看`。' '极简检索词：`先做第二个解`；`先做第二个解`；`Obsidian 模板更新`；`Goal 不能恢复`；`统一看`。'
    }
    Assert-CheckerFails 'recovery-terminal-capture-reappears-in-active-plan' 'terminal recovery capture still appears as an active-plan source obligation' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '来源锚点：`DFC-20260815-02`' '来源锚点：`DFC-20260815-01`'
    }
    Assert-CheckerFails 'recovery-active-capture-loses-plan-anchor' 'active recovery capture is not structurally anchored in the active plan' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('DFC-20260815-02', 'DFC-20260815-99')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-loses-governance-link' 'stable lifecycle authority link' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' 'DOC-INTENT-PLAN-GOVERNANCE' 'DOC-USAGE'
    }
    Assert-CheckerFails 'active-plan-reabsorbs-history' 'dynamic-only active-plan boundary' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '不进入本恢复热路径' '继续进入本恢复热路径'
    }
    Assert-CheckerFails 'recovery-resolved-without-resolved-by' 'resolved capture relationship' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?ms)(<!-- DOCS-FIRST-CAPTURE: DFC-20260815-02; schema=v1 -->.*?^状态：`resolved`。).*?^`resolved_by`：.*?governance boundary/full gates。\s*',
            ('$1' + "`r`n")
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'historical-recovery-resolved-without-resolved-by' 'resolved capture relationship: DFC-20260815-01' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace($text, '(?m)^`resolved_by`：`DOC-WORDING`.*\r?\n', '', 1)
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'recovery-index-status-diverges' 'status does not match its fast-index row' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| 恢复钩子、启动慢、简化、skill | 2026-08-21 | `resolved` |' '| 恢复钩子、启动慢、简化、skill | 2026-08-21 | `active` |'
    }
    Assert-CheckerFails 'recovery-context-marker-loses-attribution' 'malformed attributed-context marker' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' 'actor=agent; type=visual-observation' 'type=visual-observation'
    }
    Assert-CheckerFails 'recovery-loses-superseded-link' 'superseded recovery metadata contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '`superseded_by`' '`covered`'
    }
    Assert-CheckerFails 'recovery-allows-terminal-metadata-deletion' 'terminal metadata immutability' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' 'terminal 状态和关系元数据一旦形成不得静默删除' 'terminal 状态可以直接覆盖'
    }
    Assert-CheckerFails 'active-plan-duplicates-current-item' 'allows at most one current active item' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 2. 下次开工队列（禁止恢复时自动执行）',
            "### AP-20990101-01：duplicate-current`n`n## 2. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-hides-competing-heading' 'unrecognized competing level-three plan heading' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 2. 下次开工队列（禁止恢复时自动执行）',
            "### another current task`n`n- 当前状态：`in_progress`.`n`n## 2. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-duplicates-item-id' 'item IDs must be unique' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 2. 下次开工队列（禁止恢复时自动执行）',
            "### AP-20260815-05：duplicate-id`n`n- 当前状态：`queued / paused-by-explicit-goal-override / requires-explicit-resume`。`n`n## 2. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-empty-without-marker' 'explicit empty active-plan state' {
        param($caseRoot)
        Resolve-RecoveryOptimizationCaptureFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?s)## 1\. 当前活动项（恢复时先读，最多一个）.*\z',
            "## 1. 当前活动项（恢复时先读，最多一个）`n`n<!-- CURRENT-ACTIVE: none -->`n`n没有安排。`n`n## 2. 下次开工队列（禁止恢复时自动执行）`n`n当前无排队项。`n"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerPasses 'active-plan-held-queue-without-current' {
        param($caseRoot)
        Resolve-RecoveryOptimizationCaptureFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?s)(## 1\. 当前活动项（恢复时先读，最多一个）).*?(?=## 2\. 下次开工队列（禁止恢复时自动执行）)',
            '$1' + "`n`n<!-- CURRENT-ACTIVE: none -->`n`n当前无活动项。`n`n"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `resolved` | `resolved_by`: PR `#153` merged as `a1c6df7`; AP-20260815-05 terminal |' '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `resolved` | `resolved_by`: test terminal |'
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '状态：`resolved`。' '状态：`resolved`。'
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '目标去向：`AP-20260815-05`。' '`resolved_by`：test terminal。'
    }
    Assert-CheckerFails 'active-plan-auto-promote-queue-without-current' 'auto-promote queue while CURRENT-ACTIVE is none' {
        param($caseRoot)
        Resolve-RecoveryOptimizationCaptureFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?s)(## 1\. 当前活动项（恢复时先读，最多一个）).*?(?=## 2\. 下次开工队列（禁止恢复时自动执行）)',
            '$1' + "`n`n<!-- CURRENT-ACTIVE: none -->`n`n当前无活动项。`n`n"
        )
        $text = $text.Replace(
            '队列当前无其他可自动晋升项。',
            "### AP-20260815-03：测试队列项`n`n- 来源锚点：测试队列。`n- 用户目标：测试显式恢复边界。`n- 当前状态：``queued / paused-by-explicit-goal-override / requires-explicit-resume``。`n- 下一步：等待明确恢复。`n- 恢复入口：DOC-ACTIVE-PLAN。`n`n队列当前无其他可自动晋升项。"
        )
        $text = $text.Replace('requires-explicit-resume', 'auto-promote')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-keeps-successful-terminal' 'forbids successful terminal items' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-CurrentInProgressState $caseRoot 'completed / no-follow-up'
    }
    Assert-CheckerFails 'active-plan-hides-terminal-under-level-four-heading' 'AP records hidden under a non-item Markdown heading level' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 2. 下次开工队列（禁止恢复时自动执行）',
            "#### AP-20260816-06 terminal record`n`n- 当前状态：``completed``。`n`n## 2. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-exceeds-character-hot-path-limit' '6000-character hot-path limit' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        Add-Content -LiteralPath $path -Value ('x' * 6000) -Encoding utf8
    }
    Assert-CheckerFails 'crlf-current-state-mutation' 'forbids successful terminal items' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = (Get-Content -LiteralPath $path -Raw -Encoding utf8).Replace("`r`n", "`n").Replace("`n", "`r`n")
        Set-Content -LiteralPath $path -Value $text -Encoding utf8 -NoNewline
        Replace-CurrentInProgressState $caseRoot 'completed / no-follow-up'
    }
    Assert-CheckerFails 'blocked-item-loses-terminal-fields' 'abnormal terminal field: 终态原因' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-CurrentInProgressState $caseRoot 'blocked / waiting-user'
    }
    Assert-CheckerPasses 'blocked-item-keeps-recovery-contract' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-CurrentInProgressState $caseRoot 'blocked'
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 2. 下次开工队列（禁止恢复时自动执行）',
            "- 终态原因：等待外部决定。`n- 下一授权/决定：用户确认范围。`n- 恢复入口：DOC-ACTIVE-PLAN。`n`n## 2. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'current-item-loses-user-goal' 'active-plan user goal' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 用户目标：修复 Agent' '- 目标说明：修复 Agent'
    }
    Assert-CheckerFails 'current-item-loses-external-goal-anchor' 'active-plan external goal anchor' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- Goal 锚点：' '- 外部目标：'
    }
    Assert-CheckerFails 'current-item-loses-transition-authority' 'active-plan transition authority' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换依据：' '- 状态说明：'
    }
    Assert-CheckerFails 'current-item-loses-transition-type' 'structured Goal/transition values' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换类型：' '- 状态类型：'
    }
    Assert-CheckerFails 'transition-source-negation' 'affirmative source matching its structured type' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换依据：用户明确覆盖原课程 Goal，授权暂停 AP03 并把本治理修复设为唯一 active；这不是' '- 状态转换依据：未经用户明确，因工具中断初始化 active；'
    }
    Assert-CheckerPasses 'current-item-allows-explicit-goal-start' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换类型：`user-explicit-goal-override`' '- 状态转换类型：`user-explicit-goal-start`'
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换依据：用户明确覆盖' '- 状态转换依据：用户明确启动'
    }
    Assert-CheckerFails 'goal-start-rejects-override-evidence' 'affirmative source matching its structured type' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换类型：`user-explicit-goal-override`' '- 状态转换类型：`user-explicit-goal-start`'
    }
    Assert-CheckerFails 'queue-loses-user-goal' 'queue user goal' {
        param($caseRoot)
        Add-QueueFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 用户目标：复用既有 MBA C1B/C2B 链路' '- 目标说明：复用既有 MBA C1B/C2B 链路'
    }
    Assert-CheckerFails 'queue-uses-auto-resume-qualifier' 'require exactly one promotion qualifier' {
        param($caseRoot)
        Add-QueueFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' 'queued / paused-by-explicit-goal-override / requires-explicit-resume' 'queued / paused-by-explicit-goal-override / auto-resume'
    }
    Assert-CheckerFails 'current-state-hides-terminal-qualifier' 'current item uses a forbidden qualifier: closed' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-CurrentInProgressState $caseRoot 'in_progress / closed'
    }
    Assert-CheckerFails 'goal-anchor-uses-summary-inference' 'unauthorized inference source' {
        param($caseRoot)
        Add-ActivePlanFixture $caseRoot
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '当前 Goal API 返回空值，按 `unknown` 处理' '从摘要推断 Goal 为 `unknown`'
    }
    Assert-CheckerFails 'queue-exceeds-bound' 'next-start queue exceeds the five-item bound' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $extra = for ($i = 10; $i -le 15; $i++) {
            (@(
                "### AP-20990101-$i：queued-$i"
                ''
                "- 来源锚点：source-$i。"
                '- 当前状态：`queued / requires-explicit-resume`。'
                "- 用户目标：goal-$i。"
                "- 下一步：next-$i。"
                "- 恢复入口：entry-$i。"
                ''
            ) -join "`n")
        }
        $queueHeading = '## 2. 下次开工队列（禁止恢复时自动执行）'
        $text = $text.Replace($queueHeading, $queueHeading + "`n`n" + ($extra -join "`n`n"))
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'governance-records-all-thought' 'governance lifecycle marker: Agent 不记录全部思维过程' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' 'Agent 不记录全部思维过程' 'Agent 记录全部思维过程'
    }
    Assert-CheckerFails 'governance-adds-summary-as-fourth-layer' 'governance lifecycle marker: 压缩摘要、交接文字、旧消息和工具断点不构成第四层' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '压缩摘要、交接文字、旧消息和工具断点不构成第四层' '压缩摘要、交接文字、旧消息和工具断点构成第四层'
    }
    Assert-CheckerFails 'governance-reorders-three-layers' 'three-layer recovery order is missing or out of order' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $live = '**实时对话层**'
        $docs = '**Docs 持久层**'
        $text = $text.Replace($live, '__LIVE_LAYER__').Replace($docs, $live).Replace('__LIVE_LAYER__', $docs)
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'governance-reruns-business-when-writeback-lags' 'governance lifecycle marker: 不得重新执行业务' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '不得重新执行业务' '应重新执行业务'
    }
    Assert-CheckerFails 'governance-lets-docs-override-live-completion' 'governance lifecycle marker: Docs 中“仍在做”不能覆盖实时层已经发生的完成事实' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' 'Docs 中“仍在做”不能覆盖实时层已经发生的完成事实' 'Docs 中“仍在做”覆盖实时层完成事实'
    }
    Assert-CheckerFails 'governance-treats-summary-gap-as-unfinished' 'governance lifecycle marker: 摘要、断点或最近可见片段未包含的信息一律视为' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '摘要、断点或最近可见片段未包含的信息一律视为' '摘要、断点或最近可见片段未包含的信息可以视为'
    }
    Assert-CheckerFails 'governance-treats-recovery-as-new-task-authority' 'governance lifecycle marker: 恢复边界不是新任务授权' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '恢复边界不是新任务授权' '恢复边界可以选择新任务'
    }
    Assert-CheckerFails 'governance-skips-goal-api' 'Goal -> active-plan recovery order' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '在任何状态写入前先调用环境可用的' '在任何状态写入前可以跳过环境可用的'
    }
    Assert-CheckerFails 'governance-promotes-ordinary-tool-failure-to-full-recovery' 'governance lifecycle marker: 普通工具失败不是再入场' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '普通工具失败不是再入场' '普通工具失败必须重新入场'
    }
    Assert-CheckerFails 'governance-loses-empty-state-fast-exit' 'governance lifecycle marker: 恢复核对到此结束' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '恢复核对到此结束' '继续读取全部历史'
    }
    Assert-CheckerFails 'governance-allows-hidden-level-four-plan-history' 'governance lifecycle marker: 不得用四级或更深标题' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '不得用四级或更深标题' '可以用四级标题'
    }
    Assert-CheckerFails 'governance-reopens-without-provenance' 'governance lifecycle marker: 追加 `reopened_by`、证据和影响范围' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '追加 `reopened_by`、证据和影响范围' '直接改回 active'
    }
    Assert-CheckerFails 'governance-loses-queue-promotion' 'governance lifecycle marker: 只晋升排序最前且明确标记' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '只晋升排序最前且明确标记' '可以晋升任意队列项'
    }
    Assert-CheckerFails 'governance-forces-recovery-pollution' 'governance lifecycle marker: 纯通用待办没有 00_c 条目时不得为满足流程' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '纯通用待办没有 00_c 条目时不得为满足流程' '纯通用待办也必须为满足流程'
    }
    Assert-CheckerFails 'usage-absorbs-subplan' 'forbids temporary subplans in usage status' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md') -Value "`n临时子计划：继续分析。`n" -Encoding utf8
    }
    Assert-CheckerFails 'process-bypasses-recovery' 'process active-plan routing' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '`DOC-ACTIVE-PLAN`' '`DOC-WORDING`'
    }
    Assert-CheckerFails 'process-bypasses-external-goal' 'process shallow recovery routing' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' 'Goal/task-state ->' '摘要 ->'
    }
    Assert-CheckerFails 'root-agents-loses-recovery-hook' 'v1 recovery hook in root AGENTS.md' {
        param($caseRoot)
        Replace-Once $caseRoot 'AGENTS.md' '<!-- BABATA-RECOVERY-HOOK: v1 -->' '<!-- hook removed -->'
    }
    Assert-CheckerFails 'root-agents-promotes-ordinary-tool-failure' 'root AGENTS.md local tool-failure boundary' {
        param($caseRoot)
        Replace-Once $caseRoot 'AGENTS.md' 'ordinary synchronous tool, command, or API failure' 'ordinary work failure'
    }
    Assert-CheckerFails 'root-readme-promotes-ordinary-tool-failure' 'root README.md local tool-failure boundary' {
        param($caseRoot)
        Replace-Once $caseRoot 'README.md' '普通同步工具/命令/API 失败' '普通执行失败'
    }
    Assert-CheckerFails 'root-readme-loses-active-plan-link' 'root README.md recovery hook value: 00_docs/04_process/04_f_ACTIVE_PLAN.md' {
        param($caseRoot)
        Replace-Once $caseRoot 'README.md' '00_docs/04_process/04_f_ACTIVE_PLAN.md' '00_docs/README.md'
    }
    Assert-CheckerFails 'root-agents-hook-loses-end-marker' 'exactly one bounded v1 recovery hook' {
        param($caseRoot)
        Replace-Once $caseRoot 'AGENTS.md' '<!-- /BABATA-RECOVERY-HOOK: v1 -->' '<!-- hook end removed -->'
    }
    Assert-CheckerFails 'root-agents-hook-reverses-order' 'requires Goal/task-state API before Active Plan' {
        param($caseRoot)
        $path = Join-Path $caseRoot 'AGENTS.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $sentinel = '__GOAL_STEP_SENTINEL__'
        $text = $text.Replace('1. Call the available Goal/task-state API.', $sentinel)
        $text = $text.Replace('2. Immediately read `00_docs/04_process/04_f_ACTIVE_PLAN.md`', '1. Call the available Goal/task-state API.')
        $text = $text.Replace($sentinel, '2. Immediately read `00_docs/04_process/04_f_ACTIVE_PLAN.md`')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'root-readme-hook-reverses-order' 'requires Goal/task-state API before Active Plan' {
        param($caseRoot)
        $path = Join-Path $caseRoot 'README.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $sentinel = '__GOAL_PHRASE_SENTINEL__'
        $text = $text.Replace('先调用环境可用的 Goal/task-state API', $sentinel)
        $text = $text.Replace('[Active Plan](00_docs/04_process/04_f_ACTIVE_PLAN.md)', '先调用环境可用的 Goal/task-state API')
        $text = $text.Replace($sentinel, '[Active Plan](00_docs/04_process/04_f_ACTIVE_PLAN.md)')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'governance-replays-finished-action-after-compaction' 'governance lifecycle marker: 任何已有完成结果或 terminal 状态的旧实例不得重放' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '任何已有完成结果或 terminal 状态的旧实例不得重放' '仅 Docs terminal 禁止重放'
    }
    Assert-CheckerFails 'governance-infers-unfinished-after-doc-cleanup' 'governance lifecycle marker: Docs 已标记' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' 'Docs 已标记' 'Docs 未标记'
    }
    Assert-CheckerFails 'governance-conflates-goal-start-with-override' 'governance lifecycle marker: 首次建立 governing Goal 与覆盖一个仍在执行的 Goal 是两种不同转换' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '首次建立 governing Goal 与覆盖一个仍在执行的 Goal 是两种不同转换' '首次建立和覆盖 Goal 使用同一转换'
    }
    Assert-CheckerFails 'governance-adds-unknown-transition-authority' 'treats unknown information as transition authority' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md') -Value "`n信息缺失时可以重开 resolved 并切换 active。`n" -Encoding utf8
    }

    Write-Output 'Intent/plan governance mutation tests passed.'
}
finally {
    if (Test-Path -LiteralPath $root) {
        $resolved = (Resolve-Path -LiteralPath $root).Path
        $systemTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolved.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected test path: $resolved"
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
