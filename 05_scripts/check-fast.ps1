[CmdletBinding()]
param(
    [ValidateSet('All', 'Rust', 'Adapters')]
    [string]$Scope = 'All',

    [string[]]$RustPackage = @(),

    [switch]$WorkspaceRust
)

$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $repo '01_app\Cargo.toml'
$browserAdapter = Join-Path $repo '08_adapters\01_browser_extension'
$pythonAdapter = Join-Path $repo '08_adapters\02_python_bridge'
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

function Get-ChangedRustPackages {
    $changedPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $pathCommands = @(
        @('-c', 'core.safecrlf=false', 'diff', '--name-only', '--diff-filter=ACMR', 'origin/main...HEAD'),
        @('-c', 'core.safecrlf=false', 'diff', '--name-only', '--diff-filter=ACMR'),
        @('-c', 'core.safecrlf=false', 'diff', '--cached', '--name-only', '--diff-filter=ACMR'),
        @('-c', 'core.safecrlf=false', 'ls-files', '--others', '--exclude-standard')
    )
    foreach ($arguments in $pathCommands) {
        $paths = @(git @arguments)
        Assert-NativeCommandPassed "git $($arguments -join ' ')"
        foreach ($path in $paths) {
            if (-not [string]::IsNullOrWhiteSpace($path)) {
                [void]$changedPaths.Add($path.Replace('\', '/'))
            }
        }
    }

    $workspaceChanges = @($changedPaths | Where-Object {
        $_ -in @('01_app/Cargo.toml', '01_app/Cargo.lock') -or $_.StartsWith('03_migrations/')
    })
    if ($workspaceChanges.Count -gt 0) {
        return @('*')
    }

    $packages = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($path in $changedPaths) {
        if ($path -match '^01_app/([^/]+)/') {
            $packageManifest = Join-Path $repo "01_app\$($Matches[1])\Cargo.toml"
            if (Test-Path -LiteralPath $packageManifest -PathType Leaf) {
                $nameLine = Select-String -LiteralPath $packageManifest -Pattern '^name\s*=\s*"([^"]+)"' |
                    Select-Object -First 1
                if ($null -eq $nameLine) {
                    throw "Cannot read the Rust package name from $packageManifest."
                }
                [void]$packages.Add($nameLine.Matches[0].Groups[1].Value)
            }
        }
    }
    return @($packages | Sort-Object)
}

function Get-RustPackageArguments {
    param(
        [Parameter(Mandatory)]
        [string[]]$Packages
    )

    if ($Packages -contains '*') {
        return @('--workspace')
    }

    $arguments = [Collections.Generic.List[string]]::new()
    foreach ($package in $Packages) {
        if ([string]::IsNullOrWhiteSpace($package)) {
            throw 'RustPackage cannot contain an empty package name.'
        }
        $arguments.Add('--package')
        $arguments.Add($package)
    }
    return $arguments.ToArray()
}

if ($Scope -in @('All', 'Rust')) {
    Invoke-TimedCheck 'Rust formatting' {
        cargo fmt --all --check --manifest-path $manifest
        Assert-NativeCommandPassed 'cargo fmt'
    }

    $selectedPackages = if ($WorkspaceRust) {
        @('*')
    }
    elseif ($RustPackage.Count -gt 0) {
        @($RustPackage)
    }
    else {
        @(Get-ChangedRustPackages)
    }
    if ($selectedPackages.Count -eq 0) {
        Write-Output '<== No changed Rust package detected; Rust check and tests skipped.'
    }
    else {
        $packageArguments = @(Get-RustPackageArguments -Packages $selectedPackages)
        $selection = if ($selectedPackages -contains '*') { 'workspace' } else { $selectedPackages -join ', ' }
        Invoke-TimedCheck "Rust check ($selection)" {
            cargo check --quiet @packageArguments --manifest-path $manifest
            Assert-NativeCommandPassed 'cargo check'
        }
        Invoke-TimedCheck "Rust tests ($selection)" {
            cargo test --quiet @packageArguments --manifest-path $manifest
            Assert-NativeCommandPassed 'cargo test'
        }
    }
}

if ($Scope -in @('All', 'Adapters')) {
    if (-not (Test-Path -LiteralPath (Join-Path $browserAdapter 'node_modules') -PathType Container)) {
        throw "Browser adapter dependencies are missing. Run 'npm ci' in $browserAdapter first."
    }

    Invoke-TimedCheck 'TypeScript typecheck' {
        Push-Location $browserAdapter
        try {
            npm run check
            Assert-NativeCommandPassed 'npm run check'
        }
        finally {
            Pop-Location
        }
    }

    Invoke-TimedCheck 'Python bridge compile/import smoke' {
        Push-Location $pythonAdapter
        $previousPythonPath = $env:PYTHONPATH
        try {
            $env:PYTHONPATH = Join-Path $pythonAdapter 'src'
            python -m compileall -q src
            Assert-NativeCommandPassed 'python -m compileall'
            python -c "from babata_adapter import CandidateEnvelope; from babata_adapter.runner import validate; validate(CandidateEnvelope('1', 'source.smoke', 'smoke:fixture', 'text', '0' * 64))"
            Assert-NativeCommandPassed 'python import smoke'
        }
        finally {
            $env:PYTHONPATH = $previousPythonPath
            Pop-Location
        }
    }
}

$total.Stop()
Write-Output ("Fast checks passed for scope {0} in {1:N1}s." -f $Scope, $total.Elapsed.TotalSeconds)
