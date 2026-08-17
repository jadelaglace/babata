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

function Assert-NotMatches {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Pattern,
        [Parameter(Mandatory)][string]$Label
    )

    if ($Text -match $Pattern) {
        throw "Knowledge-universe ontology governance contains forbidden $Label"
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

# The architecture owns stable successor semantics; implementation state stays in Usage/output contracts.
Assert-Contains $outputs 'Successor MBA universe-registration contract (adopted; core writer and register/show CLI implemented,' 'successor contract implementation status'
Assert-Contains $architecture '本设计基线只定义 P6 稳定语义、writer 和数据关系，不维护实施切片、当前缺口或完成状态' 'architecture current-status ownership boundary'
Assert-Contains $architecture '当前实现覆盖与缺口只查 `DOC-USAGE`' 'architecture Usage routing'
Assert-Contains $architecture '`intersects_with`、`draws_from`、`applies_to`、`prerequisite_of` 等跨学科语义进入类型化关系' 'typed relation semantics in architecture'
Assert-NotMatches $architecture '(?m)^(?!.*(?:不维护|只查|不是|不得)).*(?:当前已支持|当前尚未|尚无|待实现)' 'competing current implementation claim in architecture'
Assert-Contains $rollout '任何课程 registrar 合同都必须支持本节的 course/branch 分离、typed `covers`、多重 assignment' 'stable successor registrar rollout contract'
Assert-Contains $outputs 'complete map-node non-parent typed relations' 'map-node typed-relation successor output contract'
Assert-Contains $process '历史 baseline 退出' 'versioned P6 baseline closure'
Assert-Contains $process '由 P8.9 单独交付和追踪' 'ontology successor delivery phase'
Assert-Contains $usage '| P6 | 历史 baseline 已完成 |' 'honest P6 historical status'
Assert-Contains $usage '财务管理、全球供应链已完成知识治理 successor 登记' 'P8.9 successor usage status'
Assert-Contains $usage '13/13 MBA C2B 呈现 v2 迁移完成' 'P8.9 presentation rollout status'
Assert-Contains $usage '财务管理的 successor' 'finance successor registration status'
Assert-Contains $usage '全球供应链的 successor' 'supply-chain successor registration status'

# The reusable output contract owns the historical-v1 compatibility boundary.
Assert-Contains $outputs 'Historical MBA rollout path: `babata.mba-course-c2b-plan/v1` and the current generic scripts' 'historical registrar identification'
Assert-Contains $outputs 'registration is not conformant for new formal registrations' 'new-registration prohibition'
Assert-Contains $tests '既有 C1B、内容、媒体、profile、package/live、用户验收和' 'historical acceptance dimensions in TC'
Assert-Matches $legacyRegistrar '\$foundationId\s*=.*\[string\]\$universe\.foundation_id' 'singular foundation_id behavior in historical registrar'
Assert-Contains $legacyRegistrar 'function Assert-ExactParent' 'exact-parent behavior in historical registrar'
Assert-Contains $legacyRegistrar "'--change','unassign'" 'unassign behavior in historical registrar'

Write-Output 'Knowledge-universe ontology governance passed: overlapping foundations, typed multi-parent semantics, Course/covers, MBA lens, intensity/confidence, display boundaries, successor status, historical acceptance, and legacy-registrar quarantine remain explicit.'
