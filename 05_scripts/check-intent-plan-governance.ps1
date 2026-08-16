[CmdletBinding()]
param([string]$RepoRoot)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Join-Path $PSScriptRoot '..'
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

function Read-RequiredFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Intent/plan governance is missing required file: $RelativePath"
    }
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    return $text.Replace("`r`n", "`n")
}

function Assert-Contains {
    param([string]$Text, [string]$Value, [string]$Label)
    if (-not $Text.Contains($Value)) {
        throw "Intent/plan governance is missing $Label"
    }
}

function Get-MarkdownSection {
    param([string]$Text, [string]$Heading)
    $pattern = '(?ms)^## ' + [regex]::Escape($Heading) + '\s*\r?\n(?<body>.*?)(?=^## |\z)'
    $match = [regex]::Match($Text, $pattern)
    if (-not $match.Success) {
        throw "Intent/plan governance is missing section: $Heading"
    }
    return $match.Groups['body'].Value
}

function Get-PlanItemBlocks {
    param([string]$Section)
    return @([regex]::Matches(
        $Section,
        '(?ms)^### AP-[0-9]{8}-[0-9]+[:：].*?(?=^### AP-[0-9]{8}-[0-9]+[:：]|\z)'
    ) | ForEach-Object { $_.Value })
}

function Get-PlanStateInfo {
    param([string]$Item)
    $match = [regex]::Match(
        $Item,
        '(?m)^- 当前状态：`(?<value>[^`]+)`。?\s*$'
    )
    if (-not $match.Success) {
        throw 'Intent/plan governance plan item is missing a valid current-state field.'
    }
    $parts = @($match.Groups['value'].Value -split '/' | ForEach-Object { $_.Trim() })
    if ($parts.Count -eq 0 -or $parts[0] -notmatch '^[a-z_]+$') {
        throw 'Intent/plan governance plan item has an invalid base state.'
    }
    $qualifiers = @($parts | Select-Object -Skip 1)
    foreach ($qualifier in $qualifiers) {
        if ($qualifier -notmatch '^[a-z0-9_-]+$') {
            throw "Intent/plan governance plan item has an invalid state qualifier: $qualifier"
        }
    }
    return [pscustomobject]@{
        Base = $parts[0]
        Qualifiers = $qualifiers
    }
}

function Assert-ShallowRecoveryHook {
    param([string]$Text, [string]$Label)
    $openMarker = '<!-- BABATA-RECOVERY-HOOK: v1 -->'
    $closeMarker = '<!-- /BABATA-RECOVERY-HOOK: v1 -->'
    $openMarkers = @([regex]::Matches($Text, [regex]::Escape($openMarker)))
    $closeMarkers = @([regex]::Matches($Text, [regex]::Escape($closeMarker)))
    if ($openMarkers.Count -ne 1 -or $closeMarkers.Count -ne 1) {
        throw "Intent/plan governance requires exactly one bounded v1 recovery hook in $Label."
    }
    $open = $openMarkers[0]
    $close = $closeMarkers[0]
    $hookStart = $open.Index + $open.Length
    if ($close.Index -le $hookStart) {
        throw "Intent/plan governance requires a non-empty bounded recovery hook in $Label."
    }
    $hook = $Text.Substring($hookStart, $close.Index - $hookStart)
    foreach ($value in @(
        'Goal/task-state',
        '00_docs/04_process/04_f_ACTIVE_PLAN.md',
        'CURRENT-ACTIVE',
        'unknown',
        'requires-explicit-resume',
        '00_docs/04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md'
    )) {
        Assert-Contains $hook $value "$Label recovery hook value: $value"
    }
    $goalIndex = $hook.IndexOf('Goal/task-state API', [StringComparison]::Ordinal)
    $activePlanIndex = $hook.IndexOf('00_docs/04_process/04_f_ACTIVE_PLAN.md', [StringComparison]::Ordinal)
    if ($goalIndex -lt 0 -or $activePlanIndex -lt 0 -or $goalIndex -ge $activePlanIndex) {
        throw "Intent/plan governance requires Goal/task-state API before Active Plan in the bounded $Label recovery hook."
    }
}

$rootAgents = Read-RequiredFile 'AGENTS.md'
$rootReadme = Read-RequiredFile 'README.md'
$index = Read-RequiredFile '00_docs\README.md'
$currentIntent = Read-RequiredFile '00_docs\00_requirements\00_b_USER_WORDING.md'
$recovery = Read-RequiredFile '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
$requirements = Read-RequiredFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
$process = Read-RequiredFile '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
$usage = Read-RequiredFile '00_docs\04_process\04_b_USAGE_STATUS.md'
$activePlan = Read-RequiredFile '00_docs\04_process\04_f_ACTIVE_PLAN.md'
$governance = Read-RequiredFile '00_docs\04_process\04_g_INTENT_AND_PLAN_GOVERNANCE.md'

Assert-ShallowRecoveryHook $rootAgents 'root AGENTS.md'
Assert-ShallowRecoveryHook $rootReadme 'root README.md'
Assert-Contains $index '<!-- BABATA-DOCS-RECOVERY-ENTRY: v1 -->' 'Docs index recovery entry'
foreach ($value in @('Goal/task-state API', '04_process/04_f_ACTIVE_PLAN.md', 'CURRENT-ACTIVE', 'requires-explicit-resume')) {
    Assert-Contains $index $value "Docs index recovery value: $value"
}
if ($governance -match '(?im)^(?!\s*>).*?(?:信息缺失|unknown)[^\r\n]*(?:可以|允许)[^\r\n]*(?:重开|切换|晋升)') {
    throw 'Intent/plan governance contains a rule that treats unknown information as transition authority.'
}

Assert-Contains $currentIntent 'DOC-AUTHORITY-BOUNDARY: curated-current-intent' 'current-intent role'
Assert-Contains $currentIntent '允许不改变原意的标点/格式整理、明显错字修正、非文明表达纠偏和逻辑去冗余' 'permitted current-intent curation'
Assert-Contains $currentIntent '不是逐字引文，也不使用 Markdown 引号块' 'non-verbatim current-intent boundary'
Assert-Contains $currentIntent 'stable' 'current-intent status vocabulary'
Assert-Contains $currentIntent 'conditional' 'conditional current-intent status'
Assert-Contains $currentIntent 'open' 'open current-intent status'
if ($currentIntent -match '(?m)^>') {
    throw 'Intent/plan governance forbids Markdown quote blocks in curated current intent.'
}
$currentIntentStatuses = @([regex]::Matches($currentIntent, '(?m)^状态：`(?<status>[^`]+)`') |
    ForEach-Object { $_.Groups['status'].Value })
foreach ($status in $currentIntentStatuses) {
    if ($status -notin @('stable', 'conditional', 'open')) {
        throw "Intent/plan governance current intent uses an invalid base status: $status"
    }
}
$intentTopicSections = @([regex]::Matches(
    $currentIntent,
    '(?ms)^## (?<number>(?:[2-9]|10))\.[^\r\n]*\r?\n(?<body>.*?)(?=^## |\z)'
))
if ($intentTopicSections.Count -eq 0) {
    throw 'Intent/plan governance current intent has no governed topic sections.'
}
foreach ($section in $intentTopicSections) {
    $metadata = [regex]::Match(
        $section.Groups['body'].Value,
        '(?s)状态：`(?<status>stable|conditional|open)`。归因：(?<attribution>.+?)。恢复词：`(?<phrases>[^`]+)`。'
    )
    if (-not $metadata.Success -or [string]::IsNullOrWhiteSpace($metadata.Groups['attribution'].Value)) {
        throw "Intent/plan governance current-intent topic $($section.Groups['number'].Value) is missing status, attribution, or recovery phrases."
    }
    $topicPhrases = @($metadata.Groups['phrases'].Value -split '[；;]' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($topicPhrases.Count -lt 2 -or $topicPhrases.Count -gt 8) {
        throw "Intent/plan governance current-intent topic $($section.Groups['number'].Value) requires 2-8 recovery phrases."
    }
}

Assert-Contains $recovery 'DOC-AUTHORITY-BOUNDARY: intent-recovery-evidence' 'recovery-evidence role'
Assert-Contains $recovery 'append-first 恢复账本' 'append-first recovery contract'
Assert-Contains $recovery '严重阻塞' 'last-resort recovery trigger'
Assert-Contains $recovery '恢复后目标漂移' 'recovery-drift trigger'
Assert-Contains $recovery '## 极速恢复索引' 'fast recovery index'
Assert-Contains $recovery 'active/superseded/resolved/recovery-only' 'recovery status vocabulary'
Assert-Contains $recovery '`superseded_by`' 'superseded recovery metadata contract'
Assert-Contains $recovery '`resolved_by`' 'resolved recovery metadata contract'
Assert-Contains $recovery 'terminal 状态和关系元数据一旦形成不得静默删除' 'terminal metadata immutability'
Assert-Contains $recovery '追加 `reopened_by`、证据和影响范围' 'reopen provenance contract'
$contextMarkers = @([regex]::Matches(
    $recovery,
    '<!-- ATTRIBUTED-CONTEXT: (?<id>ACX-[0-9]{8}-[0-9]+); actor=(?<actor>agent|builder|external-ai); type=(?<type>[a-z0-9-]+) -->'
))
$rawContextMarkerCount = @([regex]::Matches($recovery, '<!-- ATTRIBUTED-CONTEXT:')).Count
if ($contextMarkers.Count -ne $rawContextMarkerCount) {
    throw 'Intent/plan governance found a malformed attributed-context marker.'
}
$contextIds = @($contextMarkers | ForEach-Object { $_.Groups['id'].Value })
if ($contextIds.Count -ne @($contextIds | Select-Object -Unique).Count) {
    throw 'Intent/plan governance attributed context IDs must be unique.'
}

$captureMarkers = @([regex]::Matches(
    $recovery,
    '<!-- DOCS-FIRST-CAPTURE: (?<id>DFC-[0-9]{8}-[0-9]+); schema=v1 -->'
))
$rawCaptureMarkerCount = @([regex]::Matches($recovery, '<!-- DOCS-FIRST-CAPTURE:')).Count
if ($captureMarkers.Count -ne $rawCaptureMarkerCount) {
    throw 'Intent/plan governance found a malformed docs-first capture marker.'
}
if ($captureMarkers.Count -eq 0) {
    throw 'Intent/plan governance is missing a structured docs-first capture marker.'
}
$captureIds = @($captureMarkers | ForEach-Object { $_.Groups['id'].Value })
if ($captureIds.Count -ne @($captureIds | Select-Object -Unique).Count) {
    throw 'Intent/plan governance docs-first capture IDs must be unique.'
}
$latestCaptureMarker = $captureMarkers[-1]
$captureIndex = $latestCaptureMarker.Index
$captureId = $latestCaptureMarker.Groups['id'].Value
$recoveryHeadings = @([regex]::Matches($recovery, '(?m)^### .+$'))
if ($recoveryHeadings.Count -eq 0 -or $recoveryHeadings[-1].Index -gt $captureIndex) {
    throw 'Intent/plan governance latest recovery entry is not the structured docs-first capture.'
}
$latestCapture = $recovery.Substring($captureIndex)
$captureMetadata = [regex]::Match(
    $latestCapture,
    '(?m)^capture time：`(?<time>[^`]+)`。来源：(?<source>[^\r\n。]+)。\r?$'
)
if (-not $captureMetadata.Success -or [string]::IsNullOrWhiteSpace($captureMetadata.Groups['source'].Value)) {
    throw 'Intent/plan governance latest capture has invalid or empty time/source metadata.'
}
$parsedCaptureTime = [DateTimeOffset]::MinValue
if (-not [DateTimeOffset]::TryParse($captureMetadata.Groups['time'].Value, [ref]$parsedCaptureTime)) {
    throw 'Intent/plan governance latest capture time is not parseable.'
}
Assert-Contains $latestCapture '极简检索词：' 'latest capture retrieval phrases'
$phraseLine = [regex]::Match($latestCapture, '(?m)^极简检索词：(?<phrases>.*)$')
$phrases = @([regex]::Matches($phraseLine.Groups['phrases'].Value, '`(?<phrase>[^`]+)`') |
    ForEach-Object { $_.Groups['phrase'].Value })
if ($phrases.Count -lt 3 -or $phrases.Count -gt 8) {
    throw "Intent/plan governance requires 3-8 retrieval phrases for the latest capture, found $($phrases.Count)."
}
if ($phrases.Count -ne @($phrases | Select-Object -Unique).Count) {
    throw 'Intent/plan governance latest capture retrieval phrases must be unique.'
}
$captureStatusMatch = [regex]::Match($latestCapture, '(?m)^状态：`(?<status>active|superseded|resolved|recovery-only)`。$')
if (-not $captureStatusMatch.Success) {
    throw 'Intent/plan governance latest capture has an invalid or missing base status.'
}
$captureStatus = $captureStatusMatch.Groups['status'].Value
switch ($captureStatus) {
    'active' {
        if ($latestCapture -notmatch '(?m)^(目标去向|无覆盖关系)[:：]') {
            throw 'Intent/plan governance active capture is missing a target or explicit empty relationship.'
        }
    }
    'resolved' { Assert-Contains $latestCapture '`resolved_by`：' 'resolved capture relationship' }
    'superseded' { Assert-Contains $latestCapture '`superseded_by`：' 'superseded capture relationship' }
    'recovery-only' {
        if ($latestCapture -notmatch '(?m)^(`resolved_by`|`superseded_by`|无覆盖关系)[:：]') {
            throw 'Intent/plan governance recovery-only capture is missing an explicit relationship field.'
        }
    }
}
if ($latestCapture -notmatch '(?m)^>') {
    throw 'Intent/plan governance latest capture is missing direct verbatim wording.'
}
$recoveryIndexMatch = [regex]::Match(
    $recovery,
    '(?ms)^## 极速恢复索引\s*\r?\n\r?\n(?<table>(?:\|[^\r\n]*\|\r?\n)+)'
)
if (-not $recoveryIndexMatch.Success) {
    throw 'Intent/plan governance fast recovery index table is missing or malformed.'
}
$recoveryIndexSection = $recoveryIndexMatch.Groups['table'].Value
if (-not $recoveryIndexSection.Contains($phrases[0])) {
    throw 'Intent/plan governance fast recovery index does not contain the latest capture retrieval phrase.'
}
$recoveryIndexRows = @([regex]::Matches(
    $recoveryIndexSection,
    '(?m)^\| (?<phrases>[^|]+) \| (?<period>[^|]+) \| `(?<status>[^`]+)` \| (?<target>[^|]+) \|$'
))
foreach ($row in $recoveryIndexRows) {
    $status = $row.Groups['status'].Value
    if ($status -notin @('active', 'superseded', 'resolved', 'recovery-only')) {
        throw "Intent/plan governance recovery index uses an invalid base status: $status"
    }
}
$matchingIndexRows = @($recoveryIndexRows | Where-Object {
    $cell = $_.Groups['phrases'].Value
    @($phrases | Where-Object { -not $cell.Contains($_) }).Count -eq 0
})
if ($matchingIndexRows.Count -ne 1) {
    throw "Intent/plan governance latest capture must map to exactly one fast-index row, found $($matchingIndexRows.Count)."
}
if ($matchingIndexRows[0].Groups['status'].Value -ne $captureStatus) {
    throw 'Intent/plan governance latest capture status does not match its fast-index row.'
}

Assert-Contains $requirements '[00_b_USER_WORDING.md](00_b_USER_WORDING.md)' 'requirements current-intent link'
Assert-Contains $requirements '[00_c_USER_WORDING_RECOVERY.md](00_c_USER_WORDING_RECOVERY.md)' 'requirements recovery link'

Assert-Contains $activePlan '唯一的当前执行计划与进度控制面' 'single active-plan authority'
Assert-Contains $activePlan '每次恢复边界' 'all recovery-boundary guard'
Assert-Contains $activePlan '新 session' 'new-session recovery guard'
Assert-Contains $activePlan 'Agent/任务交接' 'Agent-task-handoff recovery guard'
Assert-Contains $activePlan 'Agent 或工具中断' 'Agent-tool-interruption recovery guard'
Assert-Contains $activePlan '长暂停' 'long-pause recovery guard'
Assert-Contains $activePlan '上下文压缩' 'context-compaction recovery guard'
Assert-Contains $activePlan '“继续”“恢复”“接着做”' 'explicit-resume-instruction guard'
Assert-Contains $activePlan '信息缺失只产生' 'unknown-state recovery rule'
Assert-Contains $activePlan '“继续/恢复”只授权继续' 'continue-resume goal preservation'
Assert-Contains $activePlan '当前 active 默认不可变' 'active-goal immutability'
Assert-Contains $activePlan '`resolved/superseded/closed` 默认不得重开' 'terminal-state immutability'
Assert-Contains $activePlan '恢复时同时核对实时对话、Goal 运行态和本文持久态' 'three-layer recovery reconciliation'
Assert-Contains $activePlan '只补终端状态、证据和清理，不重跑业务动作、收尾、测试或发布' 'lagging-terminal writeback rule'
Assert-Contains $activePlan '不能因摘要、最近消息、旧指令或工具断点再次出现而重跑' 'completed-step replay guard'
$currentHeading = '2. 当前活动项（恢复时先读，最多一个）'
$queueHeading = '3. 下次开工队列（禁止恢复时自动执行）'
$currentSection = Get-MarkdownSection $activePlan $currentHeading
$queueSection = Get-MarkdownSection $activePlan $queueHeading
if ($activePlan.IndexOf("## $currentHeading", [StringComparison]::Ordinal) -gt
    $activePlan.IndexOf("## $queueHeading", [StringComparison]::Ordinal)) {
    throw 'Intent/plan governance requires the current active item to appear before the next-start queue.'
}
$queueItems = @(Get-PlanItemBlocks $queueSection)
$currentItems = @(Get-PlanItemBlocks $currentSection)
$allThirdLevelHeadings = @([regex]::Matches(
    $queueSection + $currentSection,
    '(?m)^### (?<title>[^\r\n]+)$'
))
if ($allThirdLevelHeadings.Count -ne ($queueItems.Count + $currentItems.Count)) {
    throw 'Intent/plan governance found an unrecognized competing level-three plan heading.'
}
$planItemIds = @([regex]::Matches(
    $queueSection + $currentSection,
    '(?m)^### (?<id>AP-[0-9]{8}-[0-9]+)[:：]'
) | ForEach-Object { $_.Groups['id'].Value })
if ($planItemIds.Count -ne @($planItemIds | Select-Object -Unique).Count) {
    throw 'Intent/plan governance active-plan item IDs must be unique.'
}
if ($currentItems.Count -gt 1) {
    throw "Intent/plan governance allows at most one current active item, found $($currentItems.Count)."
}
$currentAnchorMatches = @([regex]::Matches(
    $currentSection,
    '(?m)^<!-- CURRENT-ACTIVE: (?<id>AP-[0-9]{8}-[0-9]+|none) -->$'
))
if ($currentAnchorMatches.Count -ne 1) {
    throw 'Intent/plan governance requires exactly one shallow CURRENT-ACTIVE recovery anchor.'
}
if ($currentItems.Count -eq 1) {
    $currentItemId = [regex]::Match($currentItems[0], '(?m)^### (?<id>AP-[0-9]{8}-[0-9]+)[:：]').Groups['id'].Value
    if ($currentAnchorMatches[0].Groups['id'].Value -ne $currentItemId) {
        throw 'Intent/plan governance CURRENT-ACTIVE anchor does not match the unique active item.'
    }
}
elseif ($currentAnchorMatches[0].Groups['id'].Value -ne 'none') {
    throw 'Intent/plan governance empty current state must use CURRENT-ACTIVE: none.'
}
if ($queueItems.Count -gt 5) {
    throw "Intent/plan governance next-start queue exceeds the five-item bound: $($queueItems.Count)"
}
foreach ($queueItem in $queueItems) {
    Assert-Contains $queueItem '来源锚点' 'queue source anchor'
    Assert-Contains $queueItem '用户目标' 'queue user goal'
    Assert-Contains $queueItem '下一步' 'queue next action'
    Assert-Contains $queueItem '恢复入口' 'queue recovery entry'
    $queueState = Get-PlanStateInfo $queueItem
    if ($queueState.Base -ne 'queued') {
        throw 'Intent/plan governance queue items must use queued as their base state.'
    }
    $promotionQualifiers = @($queueState.Qualifiers | Where-Object {
        $_ -in @('auto-promote', 'requires-explicit-resume')
    })
    if ($promotionQualifiers.Count -ne 1) {
        throw 'Intent/plan governance queue items require exactly one promotion qualifier: auto-promote or requires-explicit-resume.'
    }
    foreach ($forbidden in @('auto-resume', 'completed', 'resolved', 'closed', 'succeeded')) {
        if ($queueState.Qualifiers -contains $forbidden) {
            throw "Intent/plan governance queue item uses a forbidden qualifier: $forbidden"
        }
    }
}
if ($currentItems.Count -eq 0 -and $queueItems.Count -gt 0) {
    $autoPromoteItems = @($queueItems | Where-Object {
        (Get-PlanStateInfo $_).Qualifiers -contains 'auto-promote'
    })
    if ($autoPromoteItems.Count -gt 0) {
        throw 'Intent/plan governance forbids an auto-promote queue while CURRENT-ACTIVE is none.'
    }
    Assert-Contains $activePlan '当前无活动项' 'explicit held-queue active state'
}
if ($currentItems.Count -eq 0 -and $queueItems.Count -eq 0) {
    Assert-Contains $activePlan '当前无活动项' 'explicit empty active-plan state'
}
if ($currentItems.Count -eq 1) {
    $currentItem = $currentItems[0]
    Assert-Contains $currentItem '来源锚点' 'active-plan source anchor'
    Assert-Contains $currentItem 'Goal 锚点' 'active-plan external goal anchor'
    Assert-Contains $currentItem '状态转换依据' 'active-plan transition authority'
    Assert-Contains $currentItem '用户目标' 'active-plan user goal'
    Assert-Contains $currentItem '当前状态' 'active-plan current state'
    Assert-Contains $currentItem '目标终端' 'active-plan declared terminal'
    Assert-Contains $currentItem '不改变' 'active-plan protected boundary'
    Assert-Contains $currentItem '临时子计划与阶段结论' 'active-plan temporary subplan'
    Assert-Contains $currentItem '下一步' 'active-plan next action'
    Assert-Contains $currentItem '证据入口' 'active-plan evidence entry'
    $currentStateInfo = Get-PlanStateInfo $currentItem
    $currentState = $currentStateInfo.Base
    foreach ($forbidden in @('queued', 'completed', 'resolved', 'closed', 'succeeded', 'auto-promote', 'requires-explicit-resume')) {
        if ($currentStateInfo.Qualifiers -contains $forbidden) {
            throw "Intent/plan governance current item uses a forbidden qualifier: $forbidden"
        }
    }
    if ($currentState -in @('succeeded', 'completed', 'resolved', 'closed')) {
        throw 'Intent/plan governance forbids successful terminal items from accumulating in the active plan.'
    }
    if ($currentState -notin @('in_progress', 'failed', 'blocked', 'interrupted', 'handoff')) {
        throw "Intent/plan governance current item uses an invalid base state: $currentState"
    }
    if ($currentState -in @('failed', 'blocked', 'interrupted', 'handoff')) {
        foreach ($field in @('终态原因', '证据入口', '下一授权/决定', '恢复入口')) {
            Assert-Contains $currentItem $field "abnormal terminal field: $field"
        }
    }
    $goalAnchor = [regex]::Match($currentItem, '(?m)^- Goal 锚点：(?<value>.+)$')
    $transitionType = [regex]::Match($currentItem, '(?m)^- 状态转换类型：`(?<value>[a-z0-9-]+)`$')
    $transition = [regex]::Match($currentItem, '(?m)^- 状态转换依据：(?<value>.+)$')
    if (-not $goalAnchor.Success -or -not $transitionType.Success -or -not $transition.Success) {
        throw 'Intent/plan governance current item is missing structured Goal/transition values.'
    }
    if ($goalAnchor.Groups['value'].Value -match '摘要推断|最近消息推断' -or
        $transition.Groups['value'].Value -match 'Agent 自主|摘要推断|最近消息推断') {
        throw 'Intent/plan governance Goal/transition fields use an unauthorized inference source.'
    }
    if ($goalAnchor.Groups['value'].Value.Contains('unknown') -and
        -not $goalAnchor.Groups['value'].Value.Contains('Goal API')) {
        throw 'Intent/plan governance unknown Goal anchor must identify the Goal API result.'
    }
    $transitionPrefixes = @{
        'user-explicit-goal-start' = '用户明确启动'
        'user-explicit-goal-override' = '用户明确覆盖'
        'legal-terminal-auto-promote' = '当前项到达合法终端后晋升'
        'authority-approved-blocker-replan' = '出现真实阻断并由用户或既定 authority 明确授权重排'
    }
    $transitionTypeValue = $transitionType.Groups['value'].Value
    if (-not $transitionPrefixes.ContainsKey($transitionTypeValue)) {
        throw "Intent/plan governance current transition type is not an allowed enum: $transitionTypeValue"
    }
    $requiredPrefix = $transitionPrefixes[$transitionTypeValue]
    if (-not $transition.Groups['value'].Value.StartsWith($requiredPrefix, [StringComparison]::Ordinal)) {
        throw 'Intent/plan governance current transition must use an affirmative source matching its structured type.'
    }
}
for ($captureNumber = 0; $captureNumber -lt $captureMarkers.Count; $captureNumber++) {
    $marker = $captureMarkers[$captureNumber]
    $nextIndex = if ($captureNumber + 1 -lt $captureMarkers.Count) {
        $captureMarkers[$captureNumber + 1].Index
    }
    else {
        $recovery.Length
    }
    $captureBlock = $recovery.Substring($marker.Index, $nextIndex - $marker.Index)
    $captureBlockId = $marker.Groups['id'].Value
    $blockMetadata = [regex]::Match(
        $captureBlock,
        '(?m)^capture time：`(?<time>[^`]+)`。来源：(?<source>[^\r\n。]+)。\r?$'
    )
    if (-not $blockMetadata.Success -or [string]::IsNullOrWhiteSpace($blockMetadata.Groups['source'].Value)) {
        throw "Intent/plan governance capture $captureBlockId has invalid or empty time/source metadata."
    }
    $blockTime = [DateTimeOffset]::MinValue
    if (-not [DateTimeOffset]::TryParse($blockMetadata.Groups['time'].Value, [ref]$blockTime)) {
        throw "Intent/plan governance capture $captureBlockId has an unparseable capture time."
    }
    $blockPhraseLine = [regex]::Match($captureBlock, '(?m)^极简检索词：(?<phrases>.*)$')
    $blockPhrases = @([regex]::Matches($blockPhraseLine.Groups['phrases'].Value, '`(?<phrase>[^`]+)`') |
        ForEach-Object { $_.Groups['phrase'].Value })
    if ($blockPhrases.Count -lt 3 -or $blockPhrases.Count -gt 8 -or
        $blockPhrases.Count -ne @($blockPhrases | Select-Object -Unique).Count) {
        throw "Intent/plan governance capture $captureBlockId requires 3-8 unique retrieval phrases."
    }
    if ($captureBlock -notmatch '(?m)^>') {
        throw "Intent/plan governance capture $captureBlockId is missing direct verbatim wording."
    }
    $statusMatch = [regex]::Match(
        $captureBlock,
        '(?m)^状态：`(?<status>active|superseded|resolved|recovery-only)`。$'
    )
    if (-not $statusMatch.Success) {
        throw "Intent/plan governance capture $($marker.Groups['id'].Value) has an invalid or missing base status."
    }
    $itemId = $captureBlockId
    $itemStatus = $statusMatch.Groups['status'].Value
    switch ($itemStatus) {
        'active' {
            if ($captureBlock -notmatch '(?m)^(目标去向|无覆盖关系)[:：]') {
                throw "Intent/plan governance active capture $itemId is missing a target relationship."
            }
        }
        'resolved' { Assert-Contains $captureBlock '`resolved_by`：' "resolved capture relationship: $itemId" }
        'superseded' { Assert-Contains $captureBlock '`superseded_by`：' "superseded capture relationship: $itemId" }
        'recovery-only' {
            if ($captureBlock -notmatch '(?m)^(`resolved_by`|`superseded_by`|无覆盖关系)[:：]') {
                throw "Intent/plan governance recovery-only capture $itemId is missing a relationship."
            }
        }
    }
    $blockIndexRows = @($recoveryIndexRows | Where-Object {
        $cell = $_.Groups['phrases'].Value
        @($blockPhrases | Where-Object { -not $cell.Contains($_) }).Count -eq 0
    })
    if ($blockIndexRows.Count -ne 1 -or $blockIndexRows[0].Groups['status'].Value -ne $itemStatus) {
        throw "Intent/plan governance capture $itemId does not match exactly one fast-index row and status."
    }
    $sourceAnchorPattern = '(?m)^- 来源锚点：[^\r\n]*' + [regex]::Escape($itemId) + '[^\r\n]*$'
    $structuredPlanLinks = @([regex]::Matches($activePlan, $sourceAnchorPattern)).Count
    if ($itemStatus -eq 'active' -and $structuredPlanLinks -ne 1) {
        throw 'Intent/plan governance active recovery capture is not structurally anchored in the active plan.'
    }
    if ($itemStatus -ne 'active' -and $structuredPlanLinks -ne 0) {
        throw 'Intent/plan governance terminal recovery capture still appears as an active-plan source obligation.'
    }
}
$activePlanLines = @($activePlan -split "`r?`n").Count
if ($activePlanLines -gt 250) {
    throw "Intent/plan governance active plan exceeds the 250-line maintenance threshold: $activePlanLines"
}

if ($governance -notmatch '(?s)在任何状态写入前先调用环境可用的\s+Goal/task-state API，再立即读取\s+`DOC-ACTIVE-PLAN` 并核对唯一 `CURRENT-ACTIVE`') {
    throw 'Intent/plan governance is missing the Goal -> active-plan recovery order.'
}

foreach ($marker in @(
    '新产品输入 append-first 原样捕获',
    '恢复状态分为三个互补层级',
    '**实时对话层**',
    '**Goal 运行层**',
    '**Docs 持久层**',
    '判定为**回写滞后**',
    '不得重新执行业务',
    '只补写 Goal/Docs/证据的剩余终端维护',
    'Docs 中“仍在做”不能覆盖实时层已经发生的完成事实',
    'Docs 已标记',
    '压缩摘要、交接文字、旧消息和工具断点不构成第四层',
    '只有被用户在恢复后重新明确发出',
    'Agent 不记录全部思维过程',
    '摘要、断点或最近可见片段未包含的信息一律视为',
    '恢复边界不是新任务授权',
    '恢复后的首个执行动作必须由三层核对结果产生',
    '实时层已完成而',
    '任何已有完成结果或 terminal 状态的旧实例不得重放',
    '“继续/恢复”只授权继续既有 Goal',
    '当前 active goal/item 和 `resolved/superseded/closed` terminal 状态默认不可变',
    '用户明确覆盖当前 Goal',
    '首次建立 governing Goal 与覆盖一个仍在执行的 Goal 是两种不同转换',
    '追加 `reopened_by`、证据和影响范围',
    '用户编号子任务或声明阶段到达终端后',
    '失去它是否会让下一位 Agent 走不同路线或重复大段工作',
    '建议保持在约 200 行内',
    '将 adopted decision、当前需求、稳定架构、实际 usage、运行证据和仍未解决的义务提升',
    '纯通用待办没有 00_c 条目时不得为满足流程',
    '只晋升排序最前且明确标记',
    '`requires-explicit-resume` 项继续排队',
    '失败、阻塞、中断或仍有交接：保留活动项',
    '线程/外部 Goal（若可用） -> DOC-ACTIVE-PLAN 的 CURRENT-ACTIVE -> DOC-INDEX -> DOC-WORDING'
)) {
    Assert-Contains $governance $marker "governance lifecycle marker: $marker"
}

$recoveryLayers = @(
    '**实时对话层**',
    '**Goal 运行层**',
    '**Docs 持久层**'
)
$previousIndex = -1
foreach ($value in $recoveryLayers) {
    $currentIndex = $governance.IndexOf($value, [StringComparison]::Ordinal)
    if ($currentIndex -le $previousIndex) {
        throw 'Intent/plan governance three-layer recovery order is missing or out of order.'
    }
    $previousIndex = $currentIndex
}

Assert-Contains $process '`DOC-ACTIVE-PLAN`' 'process active-plan routing'
Assert-Contains $process '`DOC-INTENT-PLAN-GOVERNANCE`' 'process governance routing'
Assert-Contains $process 'Goal/task-state ->' 'process shallow recovery routing'
Assert-Contains $process '`CURRENT-ACTIVE`' 'process current-active routing'
Assert-Contains $index '`DOC-WORDING-RECOVERY`' 'recovery registry entry'
Assert-Contains $index '`DOC-ACTIVE-PLAN`' 'active-plan registry entry'
Assert-Contains $index '`DOC-INTENT-PLAN-GOVERNANCE`' 'governance registry entry'

if ($usage -match '(?i)临时子计划|temporary subplan|AP-[0-9]{8}-[0-9]+') {
    throw 'Intent/plan governance forbids temporary subplans in usage status.'
}

Write-Output 'Intent/plan governance passed: curated current intent, last-resort verbatim recovery, one bounded active plan, staged Agent conclusions, terminal promotion/cleanup, and usage separation are explicit.'
