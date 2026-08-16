[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-batch-round-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-round-governance-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\01_prd\01_a_PRD.md',
    '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md',
    '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md',
    '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md',
    '00_docs\05_tests\05_a_TEST_CASES.md',
    '05_scripts\invoke-babata-execution-round.ps1',
    '05_scripts\test-babata-execution-round.ps1'
)

function New-TestRepo {
    param([Parameter(Mandatory)][string]$Name)
    $caseRoot = Join-Path $root $Name
    foreach ($relativePath in $requiredFiles) {
        $source = Join-Path $repo $relativePath
        $target = Join-Path $caseRoot $relativePath
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $target
    }
    return $caseRoot
}

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Expected,
        [Parameter(Mandatory)][scriptblock]$Mutate
    )
    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    $failed = $false
    try {
        & $checker -RepoRoot $caseRoot | Out-Null
    }
    catch {
        $failed = $true
        if (-not $_.Exception.Message.Contains($Expected)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failed) {
        throw "Case '$Name' unexpectedly passed batch round governance."
    }
}

try {
    New-Item -ItemType Directory -Path $root | Out-Null
    & $checker -RepoRoot $repo | Out-Null

    Assert-CheckerFails -Name 'missing-acceptance-marker' -Expected 'BATCH-ROUND-TERMINAL-GATE in acceptance' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('BATCH-ROUND-TERMINAL-GATE', 'ROUND-GATE-REMOVED')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'rollout-loses-process-routing' -Expected 'MBA rollout routing to the process-owned round contract' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('本节观察遵守 `DOC-PROCESS` 的完整执行轮和失效性终止规则，不在此复制通用状态机', '本节自行定义完整执行轮状态机')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'rollout-copies-generic-round-gate' -Expected 'competing generic round-gate definition in MBA rollout' -Mutate {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md') -Value "`nBATCH-ROUND-TERMINAL-GATE`n" -Encoding utf8
    }
    Assert-CheckerFails -Name 'loses-continuation-policy' -Expected 'non-invalidating continuation policy' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\invoke-babata-execution-round.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('record_and_continue', 'stop_every_failure')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'loses-fresh-root' -Expected 'fresh round root rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\invoke-babata-execution-round.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Refusing to reuse existing round root', 'Existing directory')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'adds-string-eval' -Expected 'must not execute shell command strings' -Mutate {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '05_scripts\invoke-babata-execution-round.ps1') -Value "`nInvoke-Expression `$command`n" -Encoding utf8
    }
    Assert-CheckerFails -Name 'loses-drift-test' -Expected 'drift behavior test' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\test-babata-execution-round.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Frozen state drift did not abort before the next stage', 'Drift test failed')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }

    Write-Output 'Batch round governance mutation tests passed.'
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
