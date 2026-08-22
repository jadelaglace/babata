[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$runner = Join-Path $PSScriptRoot 'invoke-babata-execution-round.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-round-test-' + [Guid]::NewGuid().ToString('N'))
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Write-TestJson {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-TestRound {
    param(
        [Parameter(Mandatory)][string]$Plan,
        [Parameter(Mandatory)][string]$OutputRoot
    )
    $output = @(& pwsh -NoProfile -NonInteractive -File $runner -PlanPath $Plan -RoundRoot $OutputRoot -RepoRoot $repo 2>&1)
    return [pscustomobject]@{ exit_code = $LASTEXITCODE; output = $output }
}

function Read-Ledger {
    param([Parameter(Mandatory)][string]$OutputRoot)
    return Get-Content -LiteralPath (Join-Path $OutputRoot 'round-ledger.json') -Raw -Encoding utf8 | ConvertFrom-Json
}

try {
    $scripts = Join-Path $root 'scripts'
    $plans = Join-Path $root 'plans'
    New-Item -ItemType Directory -Path $scripts, $plans | Out-Null
    $inputPath = Join-Path $root 'input.txt'
    Set-Content -LiteralPath $inputPath -Value 'frozen input' -Encoding utf8
    $passScript = Join-Path $scripts 'pass.ps1'
    $failScript = Join-Path $scripts 'fail.ps1'
    $sentinelScript = Join-Path $scripts 'sentinel.ps1'
    $mutateScript = Join-Path $scripts 'mutate.ps1'
    Set-Content -LiteralPath $passScript -Value "param([string]`$Label='ok'); Write-Output `$Label" -Encoding utf8
    Set-Content -LiteralPath $failScript -Value "Write-Error 'injected stage failure'; exit 7" -Encoding utf8
    Set-Content -LiteralPath $sentinelScript -Value "param([Parameter(Mandatory)][string]`$Path); Set-Content -LiteralPath `$Path -Value 'ran'" -Encoding utf8
    Set-Content -LiteralPath $mutateScript -Value "param([Parameter(Mandatory)][string]`$Path); Add-Content -LiteralPath `$Path -Value 'changed'" -Encoding utf8

    $passPlanPath = Join-Path $plans 'pass.json'
    $passPlan = [ordered]@{
        schema = 'babata.execution-round-plan/v1'
        round_id = 'round-pass'
        scope = 'test/pass'
        target_terminal = 'candidate_ready'
        frozen_paths = @($inputPath)
        stages = @(
            [ordered]@{ id = 'first'; script = $passScript; arguments = @('-Label', 'first'); failure_policy = 'abort_round' },
            [ordered]@{ id = 'second'; script = $passScript; arguments = @('-Label', 'second'); failure_policy = 'record_and_continue'; depends_on = @('first') }
        )
        acceptance = @([ordered]@{ id = 'terminal'; description = 'all stages passed'; required = $true; stage_ids = @('first', 'second') })
    }
    Write-TestJson -Path $passPlanPath -Value $passPlan
    $passRoot = Join-Path $root 'round-pass'
    $passResult = Invoke-TestRound -Plan $passPlanPath -OutputRoot $passRoot
    if ($passResult.exit_code -ne 0) { throw "Passing round returned $($passResult.exit_code): $($passResult.output -join ' | ')" }
    $passLedger = Read-Ledger $passRoot
    if ($passLedger.status -ne 'passed' -or $passLedger.actual_terminal -ne 'candidate_ready' -or @($passLedger.stages).Count -ne 2) {
        throw 'Passing round ledger is incomplete.'
    }
    if ($passLedger.babata_build.version -ne '0.1.0' -or
        $passLedger.babata_build.git_commit -notmatch '^[0-9a-f]{40}$' -or
        $null -eq $passLedger.babata_build.PSObject.Properties['worktree_dirty']) {
        throw 'Passing round ledger is missing the Babata build identity.'
    }

    $reused = Invoke-TestRound -Plan $passPlanPath -OutputRoot $passRoot
    if ($reused.exit_code -eq 0 -or -not (($reused.output -join "`n").Contains('Refusing to reuse existing round root'))) {
        throw 'Runner reused an existing round root.'
    }

    $recordPlanPath = Join-Path $plans 'record.json'
    $recordSentinel = Join-Path $root 'record-sentinel.txt'
    $recordPlan = [ordered]@{
        schema = 'babata.execution-round-plan/v1'
        round_id = 'round-record'
        scope = 'test/record'
        target_terminal = 'candidate_ready'
        frozen_paths = @($inputPath)
        stages = @(
            [ordered]@{ id = 'start'; script = $passScript; arguments = @(); failure_policy = 'abort_round' },
            [ordered]@{ id = 'observe-defect'; script = $failScript; arguments = @(); failure_policy = 'record_and_continue'; failure_category = 'content_quality' },
            [ordered]@{ id = 'independent'; script = $sentinelScript; arguments = @('-Path', $recordSentinel); failure_policy = 'abort_round' },
            [ordered]@{ id = 'dependent'; script = $passScript; arguments = @(); failure_policy = 'record_and_continue'; depends_on = @('observe-defect') }
        )
        acceptance = @([ordered]@{ id = 'terminal'; required = $true; stage_ids = @('start', 'observe-defect', 'independent', 'dependent') })
    }
    Write-TestJson -Path $recordPlanPath -Value $recordPlan
    $recordRoot = Join-Path $root 'round-record'
    $recordResult = Invoke-TestRound -Plan $recordPlanPath -OutputRoot $recordRoot
    if ($recordResult.exit_code -ne 2) { throw "Defect round returned $($recordResult.exit_code), expected 2." }
    $recordLedger = Read-Ledger $recordRoot
    if ($recordLedger.status -ne 'terminal_with_defects' -or -not (Test-Path -LiteralPath $recordSentinel)) {
        throw 'Non-invalidating defect stopped the independent stage.'
    }
    if (@($recordLedger.defects).Count -ne 1 -or $recordLedger.defects[0].severity -ne 'non_invalidating') {
        throw 'Non-invalidating defect ledger is wrong.'
    }
    if (($recordLedger.stages | Where-Object id -eq 'dependent').status -ne 'skipped_dependency') {
        throw 'Failed dependency was not given an explicit terminal state.'
    }

    $abortPlanPath = Join-Path $plans 'abort.json'
    $abortSentinel = Join-Path $root 'abort-sentinel.txt'
    $abortPlan = [ordered]@{
        schema = 'babata.execution-round-plan/v1'
        round_id = 'round-abort'
        scope = 'test/abort'
        target_terminal = 'candidate_ready'
        frozen_paths = @($inputPath)
        stages = @(
            [ordered]@{ id = 'invalidating'; script = $failScript; arguments = @(); failure_policy = 'abort_round'; failure_category = 'authority_boundary' },
            [ordered]@{ id = 'must-not-run'; script = $sentinelScript; arguments = @('-Path', $abortSentinel); failure_policy = 'record_and_continue' }
        )
        acceptance = @([ordered]@{ id = 'terminal'; required = $true; stage_ids = @('invalidating', 'must-not-run') })
    }
    Write-TestJson -Path $abortPlanPath -Value $abortPlan
    $abortRoot = Join-Path $root 'round-abort'
    $abortResult = Invoke-TestRound -Plan $abortPlanPath -OutputRoot $abortRoot
    if ($abortResult.exit_code -ne 3) { throw "Aborted round returned $($abortResult.exit_code), expected 3." }
    $abortLedger = Read-Ledger $abortRoot
    if ($abortLedger.status -ne 'aborted' -or (Test-Path -LiteralPath $abortSentinel)) {
        throw 'Invalidating failure did not stop the round.'
    }
    if (($abortLedger.stages | Where-Object id -eq 'must-not-run').status -ne 'not_run_aborted') {
        throw 'Aborted stage did not receive an explicit terminal state.'
    }

    Set-Content -LiteralPath $inputPath -Value 'fresh frozen input' -Encoding utf8
    $driftPlanPath = Join-Path $plans 'drift.json'
    $driftSentinel = Join-Path $root 'drift-sentinel.txt'
    $driftPlan = [ordered]@{
        schema = 'babata.execution-round-plan/v1'
        round_id = 'round-drift'
        scope = 'test/drift'
        target_terminal = 'candidate_ready'
        frozen_paths = @($inputPath)
        stages = @(
            [ordered]@{ id = 'mutate-input'; script = $mutateScript; arguments = @('-Path', $inputPath); failure_policy = 'record_and_continue' },
            [ordered]@{ id = 'must-not-run'; script = $sentinelScript; arguments = @('-Path', $driftSentinel); failure_policy = 'record_and_continue' }
        )
        acceptance = @([ordered]@{ id = 'terminal'; required = $true; stage_ids = @('mutate-input', 'must-not-run') })
    }
    Write-TestJson -Path $driftPlanPath -Value $driftPlan
    $driftRoot = Join-Path $root 'round-drift'
    $driftResult = Invoke-TestRound -Plan $driftPlanPath -OutputRoot $driftRoot
    if ($driftResult.exit_code -ne 3) { throw "Drift round returned $($driftResult.exit_code), expected 3." }
    $driftLedger = Read-Ledger $driftRoot
    if ($driftLedger.status -ne 'aborted' -or $driftLedger.defects[-1].category -ne 'frozen_state_drift' -or
        (Test-Path -LiteralPath $driftSentinel)) {
        throw 'Frozen state drift did not abort before the next stage.'
    }

    Write-Output 'babata-execution-round-tests=passed'
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
