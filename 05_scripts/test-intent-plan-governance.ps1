[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-intent-plan-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-intent-plan-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
    'AGENTS.md',
    'README.md',
    '00_docs\README.md',
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md',
    '00_docs\04_process\04_b_USAGE_STATUS.md',
    '00_docs\04_process\04_c_ACTIVE_PLAN.md',
    '00_docs\90_archive\2026-08-23_P0-P9_CLOSEOUT.md'
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
    param(
        [Parameter(Mandatory)][string]$CaseRoot,
        [Parameter(Mandatory)][string]$RelativePath,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Replacement
    )
    $path = Join-Path $CaseRoot $RelativePath
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $first = $text.IndexOf($Value, [StringComparison]::Ordinal)
    if ($first -lt 0) { throw "Mutation fixture is missing ${RelativePath}: $Value" }
    $mutated = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Value.Length)
    [IO.File]::WriteAllText($path, $mutated, [Text.UTF8Encoding]::new($false))
}

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Expected,
        [Parameter(Mandatory)][scriptblock]$Mutate
    )
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

    Assert-CheckerFails 'agents-loses-plan-link' '04_c Active Plan' {
        param($caseRoot)
        Replace-Once $caseRoot 'AGENTS.md' '00_docs/04_process/04_c_ACTIVE_PLAN.md' '00_docs/README.md'
    }
    Assert-CheckerFails 'readme-reverses-recovery-order' 'Goal/task-state API before 04_c Active Plan' {
        param($caseRoot)
        $path = Join-Path $caseRoot 'README.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $start = $text.IndexOf('<!-- BABATA-RECOVERY-HOOK: v1 -->', [StringComparison]::Ordinal)
        $end = $text.IndexOf('<!-- /BABATA-RECOVERY-HOOK: v1 -->', [StringComparison]::Ordinal)
        $block = $text.Substring($start, $end - $start)
        $goal = $block.IndexOf('Goal/task-state API', [StringComparison]::Ordinal)
        $plan = $block.IndexOf('00_docs/04_process/04_c_ACTIVE_PLAN.md', [StringComparison]::Ordinal)
        if ($goal -lt 0 -or $plan -lt 0) { throw 'Recovery-order fixture is missing.' }
        $text = $text.Remove($start + $plan, '00_docs/04_process/04_c_ACTIVE_PLAN.md'.Length).Insert($start + $plan, 'Goal/task-state API')
        $text = $text.Remove($start + $goal, 'Goal/task-state API'.Length).Insert($start + $goal, '00_docs/04_process/04_c_ACTIVE_PLAN.md')
        [IO.File]::WriteAllText($path, $text, [Text.UTF8Encoding]::new($false))
    }
    Assert-CheckerFails 'duplicate-active-marker' 'exactly one CURRENT-ACTIVE marker' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\04_process\04_c_ACTIVE_PLAN.md') -Value "`n<!-- CURRENT-ACTIVE: none -->" -Encoding utf8
    }
    Assert-CheckerFails 'active-marker-loses-item' 'active item heading' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_c_ACTIVE_PLAN.md' '<!-- CURRENT-ACTIVE: none -->' '<!-- CURRENT-ACTIVE: AP-TEST-01 -->'
    }
    Assert-CheckerFails 'process-allows-summary-authorization' 'recovery lifecycle contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '压缩摘要、交接文字、旧消息和 tool checkpoint 只能定位' '压缩摘要、交接文字、旧消息和 tool checkpoint 可以授权'
    }
    Assert-CheckerFails 'process-restarts-on-local-failure' 'recovery lifecycle contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '普通同步命令/API 明确失败' '普通同步命令/API 任意失败'
    }
    Assert-CheckerFails 'archive-schedules-work' 'archive replay boundary' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\90_archive\2026-08-23_P0-P9_CLOSEOUT.md' '不从本文恢复任务' '从本文恢复任务'
    }

    Write-Output 'Intent and plan governance mutation tests passed.'
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
