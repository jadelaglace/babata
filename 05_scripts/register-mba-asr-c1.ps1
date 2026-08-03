[CmdletBinding()]
param(
    [string]$DataHome = 'D:\BabataData',
    [string]$Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\asr-manifest.json',
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe'),
    [int]$MaxItems = 0
)

$ErrorActionPreference = 'Stop'

foreach ($path in @($Manifest, $BabataExe)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}
if ($MaxItems -lt 0) {
    throw 'MaxItems must be zero (unlimited) or greater.'
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Save-Manifest {
    $document | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Manifest -Encoding utf8
}

$env:BABATA_DATA_HOME = $DataHome
$document = Get-Content -LiteralPath $Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($document.items)
$registeredDerivatives = 0
$recoveredDerivatives = 0
$skippedItems = 0
$processedItems = 0

foreach ($item in $items) {
    if ($item.status -eq 'registered') {
        $skippedItems++
        continue
    }
    if ($item.status -ne 'candidate_ready') {
        continue
    }
    if ($MaxItems -gt 0 -and $processedItems -ge $MaxItems) {
        break
    }
    $processedItems++

    $outputs = @(
        [pscustomobject]@{
            kind = 'transcript'
            path = [string]$item.transcript_file
            sha256 = [string]$item.transcript_sha256
            input_flag = '--text-file'
            media_type = 'text/markdown'
            language = 'mul'
            loss_notes = [string]$item.loss_notes
        },
        [pscustomobject]@{
            kind = 'structured_result'
            path = [string]$item.structured_file
            sha256 = [string]$item.structured_sha256
            input_flag = '--json-file'
            media_type = 'application/json'
            language = 'mul'
            loss_notes = 'Sanitized native ASR response with sentence and word timing where returned; temporary provider file URL removed.'
        }
    )

    foreach ($candidate in $outputs) {
        if (-not (Test-Path -LiteralPath $candidate.path -PathType Leaf)) {
            throw "Missing $($candidate.kind) candidate for module:$($item.module_id): $($candidate.path)"
        }
        $actualHash = Get-Sha256 -Path $candidate.path
        if ($actualHash -ne $candidate.sha256) {
            throw "Candidate hash mismatch for module:$($item.module_id) kind:$($candidate.kind)."
        }
        if ($candidate.kind -eq 'structured_result') {
            $rawJson = Get-Content -LiteralPath $candidate.path -Raw -Encoding utf8
            $null = $rawJson | ConvertFrom-Json
            if ($rawJson -match '"file_url"') {
                throw "Unsanitized file_url remains for module:$($item.module_id)."
            }
        }
    }

    $registrations = @{}
    foreach ($registration in @($item.registrations)) {
        if ($null -ne $registration -and -not [string]::IsNullOrWhiteSpace([string]$registration.kind)) {
            $registrations[[string]$registration.kind] = $registration
        }
    }

    $existingOutput = & $BabataExe --json process list-runs --revision ([string]$item.revision_id)
    if ($LASTEXITCODE -ne 0) {
        throw "Cannot inspect existing C1 runs for revision:$($item.revision_id)."
    }
    $existingRuns = @(($existingOutput -join [Environment]::NewLine) | ConvertFrom-Json)

    foreach ($candidate in $outputs) {
        if ($registrations.ContainsKey($candidate.kind)) {
            continue
        }

        $existingRun = $existingRuns |
            Where-Object {
                $_.state -eq 'succeeded' -and
                $null -eq $_.invalidated_at -and
                [string]$_.target_kind -eq $candidate.kind -and
                [string]$_.input_asset_id -eq [string]$item.asset_id -and
                [string]$_.input_sha256 -eq [string]$item.asset_sha256 -and
                [string]$_.provider -eq [string]$item.provider -and
                [string]$_.tool_or_model -eq [string]$item.model
            } |
            Sort-Object created_at -Descending

        $matchedDerivative = $null
        $matchedRun = $null
        foreach ($run in @($existingRun)) {
            $showOutput = & $BabataExe --json process show-run --run ([string]$run.id)
            if ($LASTEXITCODE -ne 0) {
                throw "Cannot read back existing C1 run:$($run.id)."
            }
            $shown = ($showOutput -join [Environment]::NewLine) | ConvertFrom-Json
            $derivative = @($shown.derivatives) |
                Where-Object {
                    [string]$_.kind -eq $candidate.kind -and
                    [string]$_.output_sha256 -eq $candidate.sha256
                } |
                Select-Object -First 1
            if ($null -ne $derivative) {
                $matchedRun = $run
                $matchedDerivative = $derivative
                break
            }
        }

        if ($null -ne $matchedDerivative) {
            $registrations[$candidate.kind] = [pscustomobject]@{
                run_id = $matchedRun.id
                derivative_id = $matchedDerivative.id
                kind = $matchedDerivative.kind
                output_sha256 = $matchedDerivative.output_sha256
                logical_path = $matchedDerivative.logical_path
            }
            $recoveredDerivatives++
            Write-Output "recovered module:$($item.module_id) kind:$($candidate.kind) from run:$($matchedRun.id)"
        }
        else {
            $paramsJson = $item.params | ConvertTo-Json -Compress -Depth 8
            $paramsArgument = $paramsJson.Replace(' ', '\u0020').Replace('"', '\"')
            $arguments = @(
                '--json', 'process', 'register',
                '--pipeline', 'agent_import',
                '--revision', [string]$item.revision_id,
                '--item', [string]$item.item_id,
                '--kind', $candidate.kind,
                '--provider', [string]$item.provider,
                '--model', [string]$item.model,
                '--tool-version', [string]$item.tool_version,
                '--input-sha256', [string]$item.asset_sha256,
                '--input-asset-id', [string]$item.asset_id,
                '--output-file', $candidate.path,
                '--params-json', $paramsArgument,
                '--usage-json', '{}',
                '--language', $candidate.language,
                '--loss-notes', $candidate.loss_notes,
                $candidate.input_flag, $candidate.path,
                '--media-type', $candidate.media_type
            )
            $output = & $BabataExe @arguments
            if ($LASTEXITCODE -ne 0) {
                throw "C1 registration failed for module:$($item.module_id) kind:$($candidate.kind): $($output -join [Environment]::NewLine)"
            }
            $result = ($output -join [Environment]::NewLine) | ConvertFrom-Json
            $showOutput = & $BabataExe --json process show-run --run ([string]$result.run_id)
            if ($LASTEXITCODE -ne 0) {
                throw "C1 read-back failed for run:$($result.run_id)."
            }
            $shown = ($showOutput -join [Environment]::NewLine) | ConvertFrom-Json
            $derivative = @($shown.derivatives) |
                Where-Object {
                    [string]$_.id -eq [string]$result.derivative_id -and
                    [string]$_.kind -eq $candidate.kind -and
                    [string]$_.output_sha256 -eq $candidate.sha256 -and
                    [string]$_.input_asset_id -eq [string]$item.asset_id
                } |
                Select-Object -First 1
            if ($null -eq $derivative) {
                throw "C1 read-back mismatch for run:$($result.run_id)."
            }
            $registrations[$candidate.kind] = [pscustomobject]@{
                run_id = $result.run_id
                derivative_id = $derivative.id
                kind = $derivative.kind
                output_sha256 = $derivative.output_sha256
                logical_path = $derivative.logical_path
            }
            $registeredDerivatives++
            Write-Output "[$registeredDerivatives] registered module:$($item.module_id) kind:$($candidate.kind)"
        }

        $item | Add-Member -NotePropertyName registrations -NotePropertyValue @($registrations.Values | Sort-Object kind) -Force
        Save-Manifest
    }

    if ($registrations.ContainsKey('transcript') -and $registrations.ContainsKey('structured_result')) {
        $item.status = 'registered'
        Save-Manifest
    }
}

[pscustomobject]@{
    total_items = $items.Count
    registered_items = @($items | Where-Object status -eq 'registered').Count
    registered_derivatives_now = $registeredDerivatives
    recovered_derivatives = $recoveredDerivatives
    already_registered_items = $skippedItems
    remaining_candidates = @($items | Where-Object status -eq 'candidate_ready').Count
    non_candidates = @($items | Where-Object status -notin @('candidate_ready', 'registered')).Count
} | ConvertTo-Json
