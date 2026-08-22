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
        throw "Knowledge-universe governance is missing required file: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding utf8
}

function Assert-Contains {
    param([string]$Text, [string]$Value, [string]$Label)
    if (-not $Text.Contains($Value)) { throw "Knowledge-universe governance is missing $Label" }
}

$requirements = Read-RequiredFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
$prd = Read-RequiredFile '00_docs\01_prd\01_a_PRD.md'
$acceptance = Read-RequiredFile '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
$architecture = Read-RequiredFile '00_docs\03_architecture\03_a_ARCHITECTURE.md'
$usage = Read-RequiredFile '00_docs\04_process\04_b_USAGE_STATUS.md'
$tests = Read-RequiredFile '00_docs\05_tests\05_a_TEST_CASES.md'
$outputs = Read-RequiredFile '02_skills\00_specs\09_outputs.md'
$profile = Read-RequiredFile '02_skills\00_specs\templates\semantic-obsidian-profile.md'
$legacyRegistrar = Read-RequiredFile '05_scripts\register-mba-course-c2b-knowledge.ps1'

Assert-Contains $requirements '稳定但可重叠的世界观基石，不是四选一内容桶' 'overlapping foundation requirement'
Assert-Contains $requirements 'Discipline/Branch 形成有类型、受约束的多父 DAG；Course 保持独立身份，通过 `covers` 关联 Branch' 'Course/Branch requirement'
Assert-Contains $requirements 'MBA 等集合是非拥有型 lens/sublibrary' 'non-owning MBA lens requirement'
Assert-Contains $requirements '默认不强制合计 100%' 'independent intensity requirement'
Assert-Contains $prd '内容可同时关联多个基石、Branch、Course 和非拥有型 lens' 'multi-assignment behavior'
Assert-Contains $acceptance '基石、Branch、Course、lens、相关度和 confidence 不混用' 'identity acceptance boundary'

Assert-Contains $architecture '时间、空间、物质、意识是可重叠 foundation assignment' 'foundation architecture'
Assert-Contains $architecture 'Discipline/Branch 使用有类型多父 DAG；Course 与 Branch 身份分离，通过 `covers` 等关系关联' 'typed Course/Branch architecture'
Assert-Contains $architecture 'MBA 等跨学科集合是 versioned non-owning lens/sublibrary' 'MBA lens architecture'
Assert-Contains $architecture '机器建议与人工审阅分别存储' 'machine/human review boundary'
Assert-Contains $tests '创建多 foundation assignment、typed relation、`covers`、动态相关度和 confidence' 'ontology verification coverage'

Assert-Contains $profile 'These display domains are not ontology `Branch`' 'display-domain/Branch separation'
Assert-Contains $outputs 'Display domains are not universe ontology branches' 'output display/ontology separation'
Assert-Contains $outputs 'Successor MBA universe-registration contract' 'successor registration contract'
Assert-Contains $outputs 'complete map-node non-parent typed relations' 'successor typed relations'
Assert-Contains $outputs 'registration is not conformant for new formal registrations' 'historical registrar quarantine'
Assert-Contains $usage '13/13 MBA 课程完成正式 C1/C1B' 'accepted MBA result preservation'

Assert-Contains $legacyRegistrar 'function Assert-ExactParent' 'historical exact-parent behavior'
Assert-Contains $legacyRegistrar "'--change','unassign'" 'historical unassign behavior'

Write-Output 'Knowledge-universe governance passed: consolidated authorities preserve overlapping foundations, typed multi-parent Course/Branch semantics, non-owning lenses, machine/human boundaries, successor contracts, and historical registrar quarantine.'
