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
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Label
    )
    if (-not $Text.Contains($Value)) {
        throw "Batch round governance is missing $Label"
    }
}

$documents = [ordered]@{
    requirements = Read-RequiredFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
    prd = Read-RequiredFile '00_docs\01_prd\01_a_PRD.md'
    acceptance = Read-RequiredFile '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
    process = Read-RequiredFile '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
    rollout = Read-RequiredFile '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md'
    tests = Read-RequiredFile '00_docs\05_tests\05_a_TEST_CASES.md'
}
$runner = Read-RequiredFile '05_scripts\invoke-babata-execution-round.ps1'
$runnerTests = Read-RequiredFile '05_scripts\test-babata-execution-round.ps1'

foreach ($entry in ([ordered]@{
    requirements = $documents.requirements
    prd = $documents.prd
    acceptance = $documents.acceptance
    process = $documents.process
    tests = $documents.tests
}).GetEnumerator()) {
    Assert-Contains -Text $entry.Value -Value 'BATCH-ROUND-TERMINAL-GATE' -Label "BATCH-ROUND-TERMINAL-GATE in $($entry.Key)"
}
Assert-Contains $documents.rollout '本节观察遵守 `DOC-PROCESS` 的完整执行轮和失效性终止规则，不在此复制通用状态机' 'MBA rollout routing to the process-owned round contract'
if ($documents.rollout.Contains('BATCH-ROUND-TERMINAL-GATE')) {
    throw 'Batch round governance forbids a competing generic round-gate definition in MBA rollout.'
}
Assert-Contains $runner 'babata.execution-round-plan/v1' 'round plan schema'
Assert-Contains $runner 'babata.execution-round-ledger/v1' 'round ledger schema'
Assert-Contains $runner 'babata_build' 'Babata build identity in round ledger'
Assert-Contains $runner 'worktree_dirty' 'Babata worktree identity in round ledger'
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

Write-Output 'Batch round governance passed: docs-first round contract, fresh root, frozen state, continuation, abort, and terminal ledger are present.'
