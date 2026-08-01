[CmdletBinding()]
param(
    [ValidateSet('All', 'Rust', 'Adapters')]
    [string]$Scope = 'All'
)

$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $repo '01_app\Cargo.toml'
$browserAdapter = Join-Path $repo '08_adapters\01_browser_extension'
$total = [Diagnostics.Stopwatch]::StartNew()

function Invoke-TimedCheck {
    param(
        [Parameter(Mandatory)]
        [string]$Name,

        [Parameter(Mandatory)]
        [scriptblock]$Action
    )

    $timer = [Diagnostics.Stopwatch]::StartNew()
    Write-Output "==> $Name"
    & $Action
    $timer.Stop()
    Write-Output ("<== {0} passed in {1:N1}s" -f $Name, $timer.Elapsed.TotalSeconds)
}

function Assert-NativeCommandPassed {
    param(
        [Parameter(Mandatory)]
        [string]$Command
    )

    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE."
    }
}

& (Join-Path $PSScriptRoot 'check-fast.ps1') -Scope $Scope -WorkspaceRust

if ($Scope -in @('All', 'Rust')) {
    Invoke-TimedCheck 'Rust Clippy' {
        cargo clippy --quiet --workspace --all-targets --manifest-path $manifest -- -D warnings
        Assert-NativeCommandPassed 'cargo clippy'
    }
}

if ($Scope -in @('All', 'Adapters')) {
    Invoke-TimedCheck 'TypeScript tests' {
        Push-Location $browserAdapter
        try {
            npm test
            Assert-NativeCommandPassed 'npm test'
        }
        finally {
            Pop-Location
        }
    }
    Invoke-TimedCheck 'TypeScript build' {
        Push-Location $browserAdapter
        try {
            npm run build
            Assert-NativeCommandPassed 'npm run build'
        }
        finally {
            Pop-Location
        }
    }
}

if ($Scope -eq 'All') {
    & (Join-Path $PSScriptRoot 'check-boundary.ps1')
}

$total.Stop()
Write-Output ("Full checks passed for scope {0} in {1:N1}s." -f $Scope, $total.Elapsed.TotalSeconds)
