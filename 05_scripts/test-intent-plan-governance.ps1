[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-intent-plan-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-intent-plan-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
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
    Assert-CheckerFails 'recovery-index-misses-latest-phrase' 'fast recovery index does not contain' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 |' '| 深层入口、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 |'
    }
    Assert-CheckerFails 'recovery-index-uses-compound-status' 'recovery index uses an invalid base status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| `active` | 多仓/子 Git' '| `active-with-history` | 多仓/子 Git'
    }
    Assert-CheckerFails 'recovery-duplicates-retrieval-phrase' 'retrieval phrases must be unique' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '极简检索词：`浅层强制钩子`；`根目录 AGENTS/README`；`Docs 去冗余`；`先 PR 后干净分支`；`恢复入口`。' '极简检索词：`浅层强制钩子`；`浅层强制钩子`；`Docs 去冗余`；`先 PR 后干净分支`；`恢复入口`。'
    }
    Assert-CheckerFails 'recovery-terminal-capture-reappears-in-active-plan' 'terminal recovery capture still appears as an active-plan obligation' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '#### 临时子计划与阶段结论',
            "#### 临时子计划与阶段结论`n`n- 错误重开：DFC-20260815-01。"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'recovery-active-capture-loses-plan-anchor' 'active recovery capture is not anchored in the active plan' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('DFC-20260815-02', 'DFC-20260815-99')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-loses-Agent-task-handoff-boundary' 'Agent-task-handoff recovery guard' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = (Get-Content -LiteralPath $path -Raw -Encoding utf8).Replace('Agent/任务交接', '普通交接')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-loses-Agent-tool-interruption-boundary' 'Agent-tool-interruption recovery guard' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = (Get-Content -LiteralPath $path -Raw -Encoding utf8).Replace('Agent 或工具中断', '执行中断')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-loses-explicit-resume-boundary' 'explicit-resume-instruction guard' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '“继续”“恢复”“接着做”' '“再次开始”'
    }
    Assert-CheckerFails 'recovery-resolved-without-resolved-by' 'resolved capture relationship' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `active` |',
            '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `resolved` |'
        )
        $text = [regex]::Replace(
            $text,
            '(?ms)(<!-- DOCS-FIRST-CAPTURE: DFC-20260815-02; schema=v1 -->.*?状态：)`active`(。)',
            '$1`resolved`$2'
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'recovery-index-status-diverges' 'status does not match its fast-index row' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `active` |' '| 浅层强制钩子、根目录 AGENTS/README、Docs 去冗余、先 PR 后干净分支、恢复入口 | 2026-08-15 23:43 | `resolved` |'
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
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 3. 下次开工队列（禁止恢复时自动执行）',
            "### AP-20990101-01：duplicate-current`n`n## 3. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-hides-competing-heading' 'unrecognized competing level-three plan heading' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace(
            '## 3. 下次开工队列（禁止恢复时自动执行）',
            "### another current task`n`n- 当前状态：`in_progress`.`n`n## 3. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-duplicates-item-id' 'item IDs must be unique' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '### AP-20260815-03：继续完成下一门 MBA 课程' '### AP-20260815-05：继续完成下一门 MBA 课程'
    }
    Assert-CheckerFails 'active-plan-empty-without-marker' 'explicit empty active-plan state' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?s)## 2\. 当前活动项（恢复时先读，最多一个）.*\z',
            "## 2. 当前活动项（恢复时先读，最多一个）`n`n<!-- CURRENT-ACTIVE: none -->`n`n没有安排。`n`n## 3. 下次开工队列（禁止恢复时自动执行）`n`n当前无排队项。`n"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-queue-without-current' 'queued item without an explicit current active decision' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace(
            $text,
            '(?s)(## 2\. 当前活动项（恢复时先读，最多一个）).*?(?=## 3\. 下次开工队列（禁止恢复时自动执行）)',
            '$1' + "`n`n<!-- CURRENT-ACTIVE: none -->`n`n当前无活动项。`n`n"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'active-plan-keeps-successful-terminal' 'forbids successful terminal items' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '当前状态：`in_progress / governance-repair-and-validation`' '当前状态：`completed / no-follow-up`'
    }
    Assert-CheckerFails 'blocked-item-loses-terminal-fields' 'abnormal terminal field: 终态原因' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '当前状态：`in_progress / governance-repair-and-validation`' '当前状态：`blocked / waiting-user`'
    }
    Assert-CheckerPasses 'blocked-item-keeps-recovery-contract' {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('当前状态：`in_progress / governance-repair-and-validation`', '当前状态：`blocked`')
        $text = $text.Replace(
            '## 3. 下次开工队列（禁止恢复时自动执行）',
            "- 终态原因：等待外部决定。`n- 下一授权/决定：用户确认范围。`n- 恢复入口：DOC-ACTIVE-PLAN。`n`n## 3. 下次开工队列（禁止恢复时自动执行）"
        )
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'current-item-loses-user-goal' 'active-plan user goal' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 用户目标：修复 Agent' '- 目标说明：修复 Agent'
    }
    Assert-CheckerFails 'current-item-loses-external-goal-anchor' 'active-plan external goal anchor' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- Goal 锚点：' '- 外部目标：'
    }
    Assert-CheckerFails 'current-item-loses-transition-authority' 'active-plan transition authority' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 状态转换依据：' '- 状态说明：'
    }
    Assert-CheckerFails 'active-plan-treats-missing-context-as-authority' 'unknown-state recovery rule' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '信息缺失只产生' '信息缺失可以产生'
    }
    Assert-CheckerFails 'active-plan-continue-selects-visible-subtopic' 'continue-resume goal preservation' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '“继续/恢复”只授权继续' '“继续/恢复”允许选择'
    }
    Assert-CheckerFails 'queue-loses-user-goal' 'queue user goal' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_f_ACTIVE_PLAN.md' '- 用户目标：复用既有 MBA C1B/C2B 链路' '- 目标说明：复用既有 MBA C1B/C2B 链路'
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
                '- 当前状态：`queued / next-start`。'
                "- 用户目标：goal-$i。"
                "- 下一步：next-$i。"
                "- 恢复入口：entry-$i。"
                ''
            ) -join "`n")
        }
        $text = $text.Replace('禁止自动执行：', ($extra -join '') + "`n禁止自动执行：")
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails 'governance-records-all-thought' 'governance lifecycle marker: Agent 不记录全部思维过程' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' 'Agent 不记录全部思维过程' 'Agent 记录全部思维过程'
    }
    Assert-CheckerFails 'governance-treats-summary-gap-as-unfinished' 'governance lifecycle marker: 摘要、断点或最近可见片段未包含的信息一律视为' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '摘要、断点或最近可见片段未包含的信息一律视为' '摘要、断点或最近可见片段未包含的信息可以视为'
    }
    Assert-CheckerFails 'governance-treats-recovery-as-new-task-authority' 'governance lifecycle marker: 恢复边界不是新任务授权' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '恢复边界不是新任务授权' '恢复边界可以选择新任务'
    }
    Assert-CheckerFails 'governance-skips-goal-api' 'governance lifecycle marker: 环境提供 Goal/task-state API 时必须先调用' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '环境提供 Goal/task-state API 时必须先调用' '环境 Goal 可以从摘要猜测'
    }
    Assert-CheckerFails 'governance-reopens-without-provenance' 'governance lifecycle marker: 追加 `reopened_by`、证据和影响范围' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '追加 `reopened_by`、证据和影响范围' '直接改回 active'
    }
    Assert-CheckerFails 'governance-loses-queue-promotion' 'governance lifecycle marker: 若队列非空，把顶部条目晋升为' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '若队列非空，把顶部条目晋升为' '队列永远不晋升为'
    }
    Assert-CheckerFails 'governance-forces-recovery-pollution' 'governance lifecycle marker: 纯通用待办没有 00_c 条目时不得为满足流程' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md' '纯通用待办没有 00_c 条目时不得为满足流程' '纯通用待办也必须为满足流程'
    }
    Assert-CheckerFails 'usage-absorbs-subplan' 'forbids temporary subplans in usage status' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md') -Value "`n临时子计划：继续分析。`n" -Encoding utf8
    }
    Assert-CheckerFails 'process-bypasses-recovery' 'process recovery routing' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '`DOC-WORDING-RECOVERY`' '`DOC-WORDING`'
    }
    Assert-CheckerFails 'process-bypasses-external-goal' 'process recovery-boundary goal guard' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '每次新 session、Agent/任务交接、Agent 或工具中断、长暂停、上下文压缩' '恢复时直接继续最近可见片段'
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
