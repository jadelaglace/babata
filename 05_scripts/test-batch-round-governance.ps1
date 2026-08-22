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

function Replace-Once {
    param([string]$CaseRoot, [string]$RelativePath, [string]$Value, [string]$Replacement)
    $path = Join-Path $CaseRoot $RelativePath
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $first = $text.IndexOf($Value, [StringComparison]::Ordinal)
    if ($first -lt 0) { throw "Mutation fixture is missing ${RelativePath}: $Value" }
    $text = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Value.Length)
    [IO.File]::WriteAllText($path, $text, [Text.UTF8Encoding]::new($false))
}

function Assert-CheckerFails {
    param([string]$Name, [string]$Expected, [scriptblock]$Mutate)
    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    try {
        & $checker -RepoRoot $caseRoot | Out-Null
        throw "$Name unexpectedly passed"
    }
    catch {
        if ($_.Exception.Message -eq "$Name unexpectedly passed") { throw }
        if (-not $_.Exception.Message.Contains($Expected)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
}

try {
    New-Item -ItemType Directory -Path $root | Out-Null
    & $checker -RepoRoot $repo | Out-Null

    Assert-CheckerFails 'requirements-loses-fresh-rerun' 'fresh rerun requirement' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md' '成组修复后从干净 staging 开新轮完整复验' '在原 staging 继续复验'
    }
    Assert-CheckerFails 'process-stops-independent-work' 'non-invalidating continuation contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '非失效性缺陷进入 defect ledger，所有独立工作继续到显式终端' '任何缺陷立即停止全部工作'
    }
    Assert-CheckerFails 'runner-loses-fresh-root' 'fresh round root rejection' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\invoke-babata-execution-round.ps1' 'Refusing to reuse existing round root' 'Reusing existing round root'
    }
    Assert-CheckerFails 'runner-loses-build-identity' 'Babata build identity' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\invoke-babata-execution-round.ps1' 'babata_build' 'product_build'
    }
    Assert-CheckerFails 'runner-adds-string-eval' 'must not execute shell command strings' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '05_scripts\invoke-babata-execution-round.ps1') -Value "`nInvoke-Expression `$command" -Encoding utf8
    }
    Assert-CheckerFails 'tests-lose-drift-case' 'drift behavior test' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\test-babata-execution-round.ps1' 'Frozen state drift did not abort before the next stage' 'Drift was ignored'
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
