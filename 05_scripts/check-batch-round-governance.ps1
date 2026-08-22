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
        throw "Batch round governance is missing required file: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding utf8
}

function Assert-Contains {
    param([string]$Text, [string]$Value, [string]$Label)
    if (-not $Text.Contains($Value)) { throw "Batch round governance is missing $Label" }
}

$requirements = Read-RequiredFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
$prd = Read-RequiredFile '00_docs\01_prd\01_a_PRD.md'
$acceptance = Read-RequiredFile '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
$process = Read-RequiredFile '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
$tests = Read-RequiredFile '00_docs\05_tests\05_a_TEST_CASES.md'
$runner = Read-RequiredFile '05_scripts\invoke-babata-execution-round.ps1'
$runnerTests = Read-RequiredFile '05_scripts\test-babata-execution-round.ps1'

Assert-Contains $requirements '多阶段或重复工作按完整 execution round 推进' 'round requirement'
Assert-Contains $requirements '成组修复后从干净 staging 开新轮完整复验' 'fresh rerun requirement'
Assert-Contains $prd 'execution round | 冻结输入和终端，连续运行到终端后统一收敛缺陷' 'round behavior'
Assert-Contains $acceptance '完整轮次能核对冻结分母、成功、失败、跳过和缺口' 'round acceptance'
Assert-Contains $process '完整轮次为最小收敛单位' 'process-owned round contract'
Assert-Contains $process '非失效性缺陷进入 defect ledger，所有独立工作继续到显式终端' 'non-invalidating continuation contract'
Assert-Contains $process '实现/config/input/acceptance 发生变化后从全新 staging 开新轮' 'fresh staging contract'
Assert-Contains $tests '终端矩阵与冻结分母一致' 'round verification coverage'

Assert-Contains $runner 'babata.execution-round-plan/v1' 'round plan schema'
Assert-Contains $runner 'babata.execution-round-ledger/v1' 'round ledger schema'
Assert-Contains $runner 'babata_build' 'Babata build identity'
Assert-Contains $runner 'worktree_dirty' 'worktree identity'
Assert-Contains $runner 'record_and_continue' 'non-invalidating continuation policy'
Assert-Contains $runner 'abort_round' 'invalidating abort policy'
Assert-Contains $runner 'Refusing to reuse existing round root' 'fresh round root rejection'
Assert-Contains $runner 'frozen_state_drift' 'frozen state drift rejection'
Assert-Contains $runner 'terminal_with_defects' 'terminal defect state'
Assert-Contains $runner 'not_run_aborted' 'explicit aborted-stage state'
Assert-Contains $runnerTests 'Non-invalidating defect stopped the independent stage' 'continuation behavior test'
Assert-Contains $runnerTests 'Frozen state drift did not abort before the next stage' 'drift behavior test'
if ($runner -match '(?i)Invoke-Expression|\biex\b') {
    throw 'Batch round runner must not execute shell command strings through Invoke-Expression.'
}

Write-Output 'Batch round governance passed: consolidated authorities and runner preserve frozen scope, continuation, fail-fast invalidation, fresh staging, build identity, and explicit terminal evidence.'
