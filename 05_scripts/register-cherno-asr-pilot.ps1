[CmdletBinding()]
param(
    [string]$Stage = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-pilot-20260822-v1',
    [string]$Babata = 'C:\Users\Aiano\Babata\01_app\target\debug\babata.exe'
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$manifestPath = Join-Path $Stage 'manifest.json'
$reportPath = Join-Path $Stage 'REPORT.md'

foreach ($path in @($manifestPath, $Babata)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}
$dataHome = [Environment]::GetEnvironmentVariable('BABATA_DATA_HOME')
if ([string]::IsNullOrWhiteSpace($dataHome)) {
    throw 'BABATA_DATA_HOME is not set.'
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function ConvertTo-WindowsCommandLineArgument {
    param([string]$Value)
    if ($Value -notmatch '[\s"]') { return $Value }
    $escaped = [regex]::Replace($Value, '(\\*)"', '$1$1\"')
    $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
    return '"' + $escaped + '"'
}

function Invoke-BabataJson {
    param([string[]]$Arguments)
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Babata
    $startInfo.Arguments = (@($Arguments | ForEach-Object { ConvertTo-WindowsCommandLineArgument -Value $_ }) -join ' ')
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($startInfo)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "Babata command failed: $($Arguments -join ' ')`n$stdout`n$stderr"
    }
    $parsed = ConvertFrom-Json -InputObject $stdout
    if ($parsed -is [System.Array]) {
        foreach ($entry in $parsed) { Write-Output $entry }
        return
    }
    return $parsed
}

function Test-RunIdentity {
    param([object]$Run, [object]$Item, [object]$Processing, [object]$Derivative)
    return (
        [string]$Run.pipeline_id -eq 'agent_import' -and
        [string]$Run.input_revision_id -eq [string]$Item.c0.revision_id -and
        [string]$Run.input_item_id -eq [string]$Item.c0.item_id -and
        [string]$Run.input_sha256 -eq [string]$Item.c0.input_sha256 -and
        [string]$Run.input_asset_id -eq [string]$Item.c0.asset_id -and
        [string]$Run.target_kind -eq [string]$Derivative.kind -and
        [string]$Run.provider -eq [string]$Processing.provider -and
        [string]$Run.tool_or_model -eq [string]$Processing.model -and
        [string]$Run.tool_version -eq [string]$Processing.tool_version -and
        [string]$Run.params.provider_input_sha256 -eq [string]$Processing.provider_input_sha256 -and
        [string]$Run.state -eq 'succeeded' -and
        $null -eq $Run.invalidated_at
    )
}

function Read-And-VerifyRun {
    param([string]$RunId, [object]$Item, [object]$Processing, [object]$Derivative)
    $detail = Invoke-BabataJson -Arguments @('--json', 'process', 'show-run', '--run', $RunId)
    if (-not (Test-RunIdentity -Run $detail.run -Item $Item -Processing $Processing -Derivative $Derivative)) {
        throw "Core read-back identity mismatch for run $RunId."
    }
    $outputs = @($detail.derivatives)
    if ($outputs.Count -ne 1) {
        throw "Expected one derivative for run $RunId, found $($outputs.Count)."
    }
    $output = $outputs[0]
    if ([string]$output.kind -ne [string]$Derivative.kind -or [string]$output.output_sha256 -ne [string]$Derivative.sha256) {
        throw "Core derivative identity/hash mismatch for run $RunId."
    }
    if ([string]$output.logical_path -notmatch '^02_derived/files/sha256/[0-9a-f]{2}/[0-9a-f]{64}$') {
        throw "Run $RunId did not enter managed C1 storage."
    }
    $managedPath = Join-Path $dataHome ([string]$output.logical_path -replace '/', '\')
    if (-not (Test-Path -LiteralPath $managedPath -PathType Leaf)) {
        throw "Managed C1 file is missing for run ${RunId}: $managedPath"
    }
    if ((Get-Sha256 -Path $managedPath) -ne [string]$output.output_sha256) {
        throw "Managed C1 hash read-back failed for run $RunId."
    }
    return [ordered]@{
        kind = [string]$Derivative.kind
        run_id = [string]$detail.run.id
        derivative_id = [string]$output.id
        managed_path = [string]$output.logical_path
        output_sha256 = [string]$output.output_sha256
        state = [string]$detail.run.state
    }
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
$notReady = @($manifest.items | Where-Object { [string]$_.status -notin @('candidate_ready', 'registered') })
if ($notReady.Count -gt 0) {
    throw "All items must be candidate_ready or registered before formal C1 registration; $($notReady.Count) are not ready."
}
$expectedItems = @($manifest.items).Count
$expectedRegistrations = @($manifest.items | ForEach-Object { @($_.derivatives).Count } | Measure-Object -Sum).Sum
$registeredNow = 0
$reused = 0
foreach ($item in @($manifest.items)) {
    $processing = @($item.processing)[0]
    $allRuns = @(Invoke-BabataJson -Arguments @('--json', 'process', 'list-runs', '--revision', [string]$item.c0.revision_id))
    $registrations = [Collections.Generic.List[object]]::new()

    foreach ($derivative in @($item.derivatives)) {
        $matching = [Collections.Generic.List[object]]::new()
        foreach ($run in @($allRuns)) {
            if (Test-RunIdentity -Run $run -Item $item -Processing $processing -Derivative $derivative) {
                $matching.Add($run)
            }
        }
        if ($matching.Count -gt 1) {
            throw "Multiple active complete C1 runs match $($item.video_id)/$($derivative.kind)."
        }
        if ($matching.Count -eq 1) {
            $runId = [string]$matching[0].id
            $reused++
        }
        else {
            $file = Join-Path $Stage ([string]$derivative.path -replace '/', '\')
            if ((Get-Sha256 -Path $file) -ne [string]$derivative.sha256) {
                throw "Staging derivative hash drift: $file"
            }
            $arguments = @(
                '--json', 'process', 'register',
                '--pipeline', 'agent_import',
                '--revision', [string]$item.c0.revision_id,
                '--item', [string]$item.c0.item_id,
                '--kind', [string]$derivative.kind,
                '--provider', [string]$processing.provider,
                '--model', [string]$processing.model,
                '--tool-version', [string]$processing.tool_version,
                '--input-sha256', [string]$item.c0.input_sha256,
                '--input-asset-id', [string]$item.c0.asset_id,
                '--output-file', $file,
                '--params-json', ($processing.params | ConvertTo-Json -Compress -Depth 20),
                '--usage-json', '{}',
                '--loss-notes', (@($derivative.loss_notes) -join ' ')
            )
            if ([string]$derivative.kind -eq 'transcript') {
                $arguments += @('--text-file', $file, '--language', 'en')
            }
            elseif ([string]$derivative.kind -eq 'structured_result') {
                $arguments += @('--json-file', $file)
            }
            else {
                throw "Unsupported Cherno pilot derivative kind: $($derivative.kind)"
            }
            $receipt = Invoke-BabataJson -Arguments $arguments
            $runId = [string]$receipt.run_id
            $registeredNow++
            $allRuns += $receipt
        }
        $registrations.Add((Read-And-VerifyRun -RunId $runId -Item $item -Processing $processing -Derivative $derivative))
    }
    $item.registrations = @($registrations)
    $item.status = 'registered'
}

$manifest | ConvertTo-Json -Depth 30 | ForEach-Object {
    [IO.File]::WriteAllText($manifestPath, $_, $utf8NoBom)
}
$roundTrip = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if (@($roundTrip.items | Where-Object status -ne 'registered').Count -ne 0 -or @($roundTrip.items.registrations).Count -ne $expectedRegistrations) {
    throw 'Registered manifest round-trip validation failed.'
}

if (Test-Path -LiteralPath $reportPath -PathType Leaf) {
    $report = Get-Content -LiteralPath $reportPath -Raw -Encoding utf8
    $report = [regex]::Replace(
        $report,
        '(?m)^- Status:.*$',
        "- Status: formal C1 registered and hash-verified for $expectedRegistrations/$expectedRegistrations derivatives across $expectedItems/$expectedItems items."
    )
    if ($report -notmatch '(?m)^## Formal C1 registration$') {
        $report = $report.TrimEnd() + "`r`n`r`n## Formal C1 registration`r`n`r`n- Registered items: $expectedItems/$expectedItems.`r`n- Registered derivatives: $expectedRegistrations/$expectedRegistrations.`r`n- Every managed path and SHA-256 was read back from Babata core.`r`n"
    }
    [IO.File]::WriteAllText($reportPath, $report, $utf8NoBom)
}

[ordered]@{
    items_registered = @($roundTrip.items | Where-Object status -eq 'registered').Count
    registrations = @($roundTrip.items.registrations).Count
    registered_now = $registeredNow
    reused_and_verified = $reused
    manifest = $manifestPath
} | ConvertTo-Json
