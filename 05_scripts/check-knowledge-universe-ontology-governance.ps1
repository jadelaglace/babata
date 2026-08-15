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
        throw "Knowledge-universe ontology governance is missing required file: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding utf8
}

function Assert-Contains {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Label
    )

    if (-not $Text.Contains($Value)) {
        throw "Knowledge-universe ontology governance is missing $Label"
    }
}

function Assert-Matches {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Pattern,
        [Parameter(Mandatory)][string]$Label
    )

    if ($Text -notmatch $Pattern) {
        throw "Knowledge-universe ontology governance is missing $Label"
    }
}

function Assert-NotContains {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Label
    )

    if ($Text.Contains($Value)) {
        throw "Knowledge-universe ontology governance contains forbidden $Label"
    }
}

$wording = Read-RequiredFile '00_docs\00_requirements\00_b_USER_WORDING.md'
$wordingRecovery = Read-RequiredFile '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md'
$requirements = Read-RequiredFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
$prd = Read-RequiredFile '00_docs\01_prd\01_a_PRD.md'
$acceptance = Read-RequiredFile '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
$architecture = Read-RequiredFile '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md'
$process = Read-RequiredFile '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
$rollout = Read-RequiredFile '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md'
$usage = Read-RequiredFile '00_docs\04_process\04_b_USAGE_STATUS.md'
$tests = Read-RequiredFile '00_docs\05_tests\05_a_TEST_CASES.md'
$outputs = Read-RequiredFile '02_skills\00_specs\09_outputs.md'
$profile = Read-RequiredFile '02_skills\00_specs\templates\semantic-obsidian-profile.md'
$legacyRegistrar = Read-RequiredFile '05_scripts\register-mba-course-c2b-knowledge.ps1'

# Highest-authority product semantics must remain explicit in requirements, PRD, AC and architecture.
Assert-Contains $wording '交叉本体结论为' 'adopted-analysis current-intent attribution'
Assert-Contains $wording '`agent-analysis-user-adopted`' 'adopted-analysis current-intent role'
Assert-Contains $wordingRecovery '<!-- KNOWLEDGE-UNIVERSE-ADOPTED-ANALYSIS: agent-summary; user-adopted; not-verbatim -->' 'adopted-analysis recovery provenance marker'
Assert-Contains $wordingRecovery '以下是被用户整体采用的 Agent 分析摘要，**不是用户逐字原话**' 'adopted-analysis recovery attribution boundary'
Assert-Contains $requirements '稳定但允许重叠的观察维度' 'overlapping-foundation semantics in requirements'
Assert-Contains $requirements '也不以未定义的 MECE/穷尽性强迫未知内容归类' 'non-exhaustive foundation semantics in requirements'
Assert-Contains $prd '可重叠的世界观观察维度，不是要求对象单选或强行穷尽的文件夹' 'overlapping and non-exhaustive foundations in PRD'
Assert-Contains $acceptance '系统不强迫四选一、不把“未知”伪装成已归类' 'unknown-safe foundation acceptance'
Assert-Contains $architecture '对象可以关联零个、一个或多个基石' 'zero-or-many foundation architecture'

Assert-Contains $requirements '稳定专业分支与具体课程不是同一身份' 'Course/Branch identity separation in requirements'
Assert-Contains $requirements '一个或多个稳定 `Branch`（专业分支）' 'stable Branch covers target in requirements'
Assert-Contains $prd '通过 `covers` 多对多覆盖稳定 Branch' 'many-to-many covers behavior in PRD'
Assert-Contains $acceptance '一课可 `covers` 多分支、一个分支可由' 'many-to-many covers acceptance'
Assert-Contains $architecture '身份和生命周期分开' 'Course/Branch lifecycle separation in architecture'
Assert-Contains $architecture '一个或多个稳定 `Branch`' 'stable Branch covers target in architecture'
Assert-Contains $rollout '一个或多个稳定 `Branch` 的 `covers` 关系' 'stable Branch covers target in rollout'
Assert-Contains $tests '一个 branch 被多课覆盖' 'many-to-many covers test coverage'
Assert-NotContains $architecture 'branch/topic' 'undefined Topic covers target in architecture'
Assert-NotContains $rollout 'branch/topic' 'undefined Topic covers target in rollout'

Assert-Contains $requirements 'MBA 是跨学科的培养/课程集合和导航视角，不是单一学科' 'MBA lens requirement'
Assert-Contains $architecture 'MBA 当前建模为版本化 `SublibraryDefinition`/lens' 'MBA Sublibrary/lens architecture'
Assert-Contains $architecture '不作为 discipline' 'MBA non-discipline boundary'

Assert-Contains $requirements '不要求相加为 `100%`，判断置信度必须另记' 'independent foundation intensity requirement'
Assert-Contains $architecture '判断置信度独立记录，不能用高强度冒充高置信度' 'separate confidence architecture'
Assert-Contains $acceptance '默认不强制总和为 `100%`' 'non-normalized foundation acceptance'

# Course-local learning navigation must not acquire ontology authority.
Assert-Contains $profile 'These display domains are not ontology `Branch`' 'display-domain/Branch separation in profile'
Assert-Contains $outputs 'Display domains are not universe ontology branches' 'display-domain/ontology separation in output contract'

# The adopted successor and accepted historical instances must remain different state claims.
Assert-Contains $outputs 'Successor MBA universe-registration contract (adopted, not yet implemented or enabled for new formal' 'successor contract implementation status'
Assert-Contains $architecture '以上为已采用、' 'adopted successor architecture state'
Assert-Contains $architecture '待实现的后继核心语义' 'not-yet-implemented successor architecture state'
Assert-Contains $architecture '`intersects_with/draws_from/applies_to/prerequisite_of` 等非父边类型化关系' 'unimplemented map-node typed-relation scope in architecture'
Assert-Contains $rollout 'map-node 非父边类型化关系、course/branch 分离' 'map-node typed-relation rollout scope'
Assert-Contains $outputs 'complete map-node non-parent typed relations' 'map-node typed-relation successor output contract'
Assert-Contains $process '历史 baseline 退出' 'versioned P6 baseline closure'
Assert-Contains $process '作为 P8.9 当前交付缺口' 'ontology successor delivery phase'
Assert-Contains $usage '| P6 | 历史 baseline 已完成 |' 'honest P6 historical status'
Assert-Contains $usage 'map-node 非父边类型化关系、course/branch 分离' 'complete P8.9 ontology gap'
Assert-Contains $usage '使用历史单路径 `意识 -> 管理学 -> 财务管理`，不证明当前已采用的 course/branch 分离' 'finance acceptance/ontology separation'
Assert-Contains $usage '使用历史单路径 `意识 -> 管理学 -> 供应链管理`，不证明当前已采用的 course/branch 分离' 'supply-chain acceptance/ontology separation'

# The historical v1 path remains reconstructable evidence, but cannot register another formal course.
Assert-Contains $rollout '当前 `babata.mba-course-c2b-plan/v1` 和 `register-mba-course-c2b-knowledge.ps1`' 'historical registrar identification'
Assert-Contains $rollout '不得用于新增课程的正式宇宙归属或宣称新架构 conformance' 'new-registration prohibition'
Assert-Contains $tests '既有 C1B、内容、媒体、profile、package/live、用户验收和' 'historical acceptance dimensions in TC'
Assert-Matches $legacyRegistrar '\$foundationId\s*=.*\[string\]\$universe\.foundation_id' 'singular foundation_id behavior in historical registrar'
Assert-Contains $legacyRegistrar 'function Assert-ExactParent' 'exact-parent behavior in historical registrar'
Assert-Contains $legacyRegistrar "'--change','unassign'" 'unassign behavior in historical registrar'

Write-Output 'Knowledge-universe ontology governance passed: overlapping foundations, typed multi-parent semantics, Course/covers, MBA lens, intensity/confidence, display boundaries, successor status, historical acceptance, and legacy-registrar quarantine remain explicit.'
