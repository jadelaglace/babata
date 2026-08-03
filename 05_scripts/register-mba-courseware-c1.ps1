[CmdletBinding()]
param(
    [string]$DataHome = 'D:\BabataData',
    [string]$Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\courseware-manifest.json',
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe')
)

$ErrorActionPreference = 'Stop'

foreach ($path in @($Manifest, $BabataExe)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}

$env:BABATA_DATA_HOME = $DataHome
$document = Get-Content -LiteralPath $Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($document.items)
$registered = 0
$skipped = 0

foreach ($item in $items) {
    if ($item.status -eq 'registered') {
        $skipped++
        continue
    }
    if ($item.status -ne 'candidate_ready') {
        continue
    }

    $existingOutput = & $BabataExe --json process list-runs --revision ([string]$item.revision_id)
    if ($LASTEXITCODE -ne 0) {
        throw "Cannot inspect existing C1 runs for revision:$($item.revision_id)."
    }
    $existingRuns = ($existingOutput -join [Environment]::NewLine) | ConvertFrom-Json
    $existingRuns = @($existingRuns)
    $existingRun = $existingRuns |
        Where-Object {
            $_.state -eq 'succeeded' -and
            $null -eq $_.invalidated_at -and
            [string]$_.target_kind -eq [string]$item.kind -and
            [string]$_.input_asset_id -eq [string]$item.asset_id -and
            [string]$_.input_sha256 -eq [string]$item.asset_sha256 -and
            [string]$_.provider -eq [string]$item.provider -and
            [string]$_.tool_or_model -eq [string]$item.model
        } |
        Sort-Object created_at -Descending |
        Select-Object -First 1
    if ($null -ne $existingRun) {
        $existingShowOutput = & $BabataExe --json process show-run --run ([string]$existingRun.id)
        if ($LASTEXITCODE -ne 0) {
            throw "Cannot read back existing C1 run:$($existingRun.id)."
        }
        $existingShown = ($existingShowOutput -join [Environment]::NewLine) | ConvertFrom-Json
        $existingDerivative = @($existingShown.derivatives) |
            Where-Object output_sha256 -eq ([string]$item.output_sha256) |
            Select-Object -First 1
        if ($null -ne $existingDerivative) {
            $item.status = 'registered'
            $item.registration = [pscustomobject]@{
                run_id = $existingRun.id
                derivative_id = $existingDerivative.id
                kind = $existingDerivative.kind
                output_sha256 = $existingDerivative.output_sha256
                logical_path = $existingDerivative.logical_path
            }
            $skipped++
            $document | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Manifest -Encoding utf8
            Write-Output "recovered module:$($item.module_id) from run:$($existingRun.id)"
            continue
        }
    }

    $paramsJson = $item.params | ConvertTo-Json -Compress -Depth 8
    $paramsArgument = $paramsJson.Replace(' ', '\u0020').Replace('"', '\"')
    $arguments = @(
        '--json', 'process', 'register',
        '--pipeline', 'agent_import',
        '--revision', [string]$item.revision_id,
        '--item', [string]$item.item_id,
        '--kind', [string]$item.kind,
        '--provider', [string]$item.provider,
        '--model', [string]$item.model,
        '--tool-version', [string]$item.tool_version,
        '--input-sha256', [string]$item.asset_sha256,
        '--input-asset-id', [string]$item.asset_id,
        '--output-file', [string]$item.output_file,
        '--params-json', $paramsArgument,
        '--usage-json', '{}',
        '--language', 'mul',
        '--loss-notes', [string]$item.loss_notes
    )
    if ($item.kind -eq 'structured_result') {
        $arguments += @('--json-file', [string]$item.output_file, '--media-type', 'application/json')
    }
    else {
        $arguments += @('--text-file', [string]$item.output_file, '--media-type', 'text/markdown')
    }

    $output = & $BabataExe @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "C1 registration failed for module:$($item.module_id): $($output -join [Environment]::NewLine)"
    }
    $result = ($output -join [Environment]::NewLine) | ConvertFrom-Json
    $showOutput = & $BabataExe --json process show-run --run ([string]$result.run_id)
    if ($LASTEXITCODE -ne 0) {
        throw "C1 read-back failed for run:$($result.run_id): $($showOutput -join [Environment]::NewLine)"
    }
    $shown = ($showOutput -join [Environment]::NewLine) | ConvertFrom-Json
    $derivative = @($shown.derivatives) | Select-Object -First 1
    if ($null -eq $derivative -or [string]$derivative.id -ne [string]$result.derivative_id) {
        throw "C1 read-back derivative mismatch for run:$($result.run_id)."
    }
    $item.status = 'registered'
    $item.registration = [pscustomobject]@{
        run_id = $result.run_id
        derivative_id = $derivative.id
        kind = $derivative.kind
        output_sha256 = $derivative.output_sha256
        logical_path = $derivative.logical_path
    }
    $registered++
    $document | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Manifest -Encoding utf8
    Write-Output "[$registered] registered module:$($item.module_id) $($item.title)"
}

[pscustomobject]@{
    total = $items.Count
    registered_now = $registered
    already_registered = $skipped
    remaining_candidates = @($items | Where-Object status -eq 'candidate_ready').Count
    non_candidates = @($items | Where-Object status -notin @('candidate_ready', 'registered')).Count
} | ConvertTo-Json
