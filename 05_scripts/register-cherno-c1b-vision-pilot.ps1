[CmdletBinding()]
param(
    [string]$Stage = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-pilot-20260822-v1',
    [string]$Babata = 'C:\Users\Aiano\Babata\01_app\target\debug\babata.exe',
    [int]$ExpectedItems = 3,
    [string]$LedgerScope = 'representative_pilot'
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$manifestPath = Join-Path $Stage 'manifest.json'
$ledgerPath = Join-Path $Stage 'c1b-registration-ledger.json'
foreach ($path in @($manifestPath, $Babata)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing required input: $path" }
}
$dataHome = [Environment]::GetEnvironmentVariable('BABATA_DATA_HOME')
if ([string]::IsNullOrWhiteSpace($dataHome)) { throw 'BABATA_DATA_HOME is not set.' }
$ffmpegVersion = (& ffmpeg -version | Select-Object -First 1).Trim()
if ($LASTEXITCODE -ne 0) { throw 'ffmpeg is unavailable.' }

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
    $job = Start-Job -ScriptBlock {
        param($exe, $argv)
        $out = @(& $exe @argv 2>&1)
        [pscustomobject]@{
            exit_code = $LASTEXITCODE
            output = (($out | ForEach-Object { [string]$_ }) -join "`n")
        }
    } -ArgumentList $Babata, (,[string[]]$Arguments)
    try {
        $result = Wait-Job -Job $job -Timeout 120 | Receive-Job -Keep
        if ($null -eq $result) { throw "Babata command timed out after 120s: $($Arguments -join ' ')" }
        $stdout = [string]$result.output
        if ([int]$result.exit_code -ne 0) { throw "Babata command failed: $($Arguments -join ' ')`n$stdout" }
    }
    finally {
        if ($job.State -eq 'Running') { Stop-Job -Job $job -Force -ErrorAction SilentlyContinue }
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    }
    $parsed = ConvertFrom-Json -InputObject $stdout
    if ($parsed -is [System.Array]) {
        foreach ($entry in $parsed) { Write-Output $entry }
        return
    }
    return $parsed
}

function Read-And-VerifyRun {
    param(
        [string]$RunId,
        [object]$Item,
        [string]$Kind,
        [string]$Provider,
        [string]$Model,
        [string]$ToolVersion,
        [string]$OutputSha
    )
    $detail = Invoke-BabataJson -Arguments @('--json', 'process', 'show-run', '--run', $RunId)
    $run = $detail.run
    if ([string]$run.pipeline_id -ne 'agent_import' -or
        [string]$run.input_revision_id -ne [string]$Item.c0.revision_id -or
        [string]$run.input_item_id -ne [string]$Item.c0.item_id -or
        [string]$run.input_sha256 -ne [string]$Item.c0.input_sha256 -or
        [string]$run.input_asset_id -ne [string]$Item.c0.asset_id -or
        [string]$run.target_kind -ne $Kind -or
        [string]$run.provider -ne $Provider -or
        [string]$run.tool_or_model -ne $Model -or
        [string]$run.tool_version -ne $ToolVersion -or
        [string]$run.state -ne 'succeeded' -or $null -ne $run.invalidated_at) {
        throw "C1B run identity mismatch for $RunId."
    }
    $derivatives = @($detail.derivatives)
    if ($derivatives.Count -ne 1 -or [string]$derivatives[0].kind -ne $Kind -or
        [string]$derivatives[0].output_sha256 -ne $OutputSha) {
        throw "C1B derivative identity mismatch for $RunId."
    }
    $logicalPath = [string]$derivatives[0].logical_path
    if ($logicalPath -notmatch '^02_derived/files/sha256/[0-9a-f]{2}/[0-9a-f]{64}$') {
        throw "C1B derivative did not enter managed storage for $RunId."
    }
    $managedPath = Join-Path $dataHome ($logicalPath -replace '/', '\')
    if ((Get-Sha256 -Path $managedPath) -ne $OutputSha) {
        throw "C1B managed hash read-back failed for $RunId."
    }
    return [ordered]@{
        registration = 'registered_or_reused'
        kind = $Kind
        run_id = [string]$run.id
        derivative_id = [string]$derivatives[0].id
        logical_path = $logicalPath
        output_sha256 = $OutputSha
    }
}

function Find-MatchingRun {
    param(
        [object[]]$Runs,
        [object]$Item,
        [string]$Kind,
        [string]$Provider,
        [string]$Model,
        [string]$ToolVersion,
        [object]$Params,
        [string]$OutputSha
    )
    $matching = [Collections.Generic.List[object]]::new()
    foreach ($run in @($Runs)) {
        $identityMatches = (
            [string]$run.pipeline_id -eq 'agent_import' -and
            [string]$run.input_revision_id -eq [string]$Item.c0.revision_id -and
            [string]$run.input_item_id -eq [string]$Item.c0.item_id -and
            [string]$run.input_sha256 -eq [string]$Item.c0.input_sha256 -and
            [string]$run.input_asset_id -eq [string]$Item.c0.asset_id -and
            [string]$run.target_kind -eq $Kind -and
            [string]$run.provider -eq $Provider -and
            [string]$run.tool_or_model -eq $Model -and
            [string]$run.tool_version -eq $ToolVersion -and
            [string]$run.state -eq 'succeeded' -and $null -eq $run.invalidated_at -and
            (
                ($Provider -eq 'qianwen_skill' -and [string]$run.params.provider_input_sha256 -eq [string]$Params.provider_input_sha256) -or
                ($Provider -eq 'local_extract' -and
                    [string]$run.params.essence_decision_sha256 -eq [string]$Params.essence_decision_sha256 -and
                    [string]$run.params.role -eq [string]$Params.role -and
                    [double]$run.params.source_locator.start_seconds -eq [double]$Params.source_locator.start_seconds -and
                    [double]$run.params.source_locator.end_seconds -eq [double]$Params.source_locator.end_seconds -and
                    [double]$run.params.source_locator.frame_timestamp_seconds -eq [double]$Params.source_locator.frame_timestamp_seconds)
            )
        )
        if (-not $identityMatches) { continue }
        $detail = Invoke-BabataJson -Arguments @('--json', 'process', 'show-run', '--run', [string]$run.id)
        $derivatives = @($detail.derivatives)
        if ($derivatives.Count -eq 1 -and [string]$derivatives[0].output_sha256 -eq $OutputSha) {
            $matching.Add($run)
        }
    }
    if ($matching.Count -gt 1) { throw "Multiple active C1B runs match $($Item.video_id)/$Kind."
    }
    if ($matching.Count -eq 1) { return [string]$matching[0].id }
    return $null
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($ExpectedItems -lt 1 -or [string]$manifest.status -notin @('staged_only', 'registered') -or @($manifest.items).Count -ne $ExpectedItems) {
    throw "The C1B vision round is not a complete $ExpectedItems-item candidate."
}
$existingLedger = $null
if ([string]$manifest.status -eq 'registered') {
    if (-not (Test-Path -LiteralPath $ledgerPath -PathType Leaf)) {
        throw 'Registered C1B manifest is missing its registration ledger.'
    }
    $existingLedger = Get-Content -LiteralPath $ledgerPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ([string]$existingLedger.schema -ne 'babata.cherno-course-c1b-registration/v1' -or
        [string]$existingLedger.status -ne 'registered' -or @($existingLedger.items).Count -ne $ExpectedItems) {
        throw 'Existing C1B registration ledger is incomplete or has the wrong schema.'
    }
}
$ledgerItems = [Collections.Generic.List[object]]::new()
$registeredNow = 0
$reused = 0
foreach ($item in @($manifest.items)) {
    if ([string]$item.status -notin @('staged_only', 'registered')) {
        throw "C1B item is not registerable: $($item.video_id)/$($item.status)"
    }
    if ([string]$item.status -eq 'registered') {
        $existingItems = @($existingLedger.items | Where-Object { [string]$_.video_id -eq [string]$item.video_id })
        if ($existingItems.Count -ne 1) {
            throw "Registered C1B ledger does not uniquely identify $($item.video_id)."
        }
        $existingItem = $existingItems[0]
        if ([string]$existingItem.source_item_id -ne [string]$item.c0.item_id -or
            [string]$existingItem.source_revision_id -ne [string]$item.c0.revision_id -or
            [string]$existingItem.source_asset_id -ne [string]$item.c0.asset_id -or
            [string]$existingItem.source_asset_sha256 -ne [string]$item.c0.input_sha256) {
            throw "Registered C1B ledger source identity drifted for $($item.video_id)."
        }
        $decisionRegistration = Read-And-VerifyRun -RunId ([string]$existingItem.decision_registration.run_id) -Item $item -Kind 'structured_result' -Provider 'qianwen_skill' -Model ([string]$item.processing.model) -ToolVersion ([string]$item.processing.tool_version) -OutputSha ([string]$item.essence_decision.sha256)
        $mediaRegistrations = [Collections.Generic.List[object]]::new()
        foreach ($media in @($item.retained_derivatives)) {
            $existingMedia = @($existingItem.media_registrations | Where-Object {
                [string]$_.kind -eq [string]$media.kind -and [string]$_.output_sha256 -eq [string]$media.sha256
            })
            if ($existingMedia.Count -ne 1) {
                throw "Registered C1B ledger does not uniquely identify $($item.video_id)/$($media.path)."
            }
            $registration = Read-And-VerifyRun -RunId ([string]$existingMedia[0].run_id) -Item $item -Kind ([string]$media.kind) -Provider 'local_extract' -Model 'ffmpeg' -ToolVersion $ffmpegVersion -OutputSha ([string]$media.sha256)
            $registration.role = [string]$media.role
            $registration.source_locator = $media.source_locator
            $mediaRegistrations.Add($registration)
        }
        $reused += 1 + $mediaRegistrations.Count
        $item.registrations = @($decisionRegistration) + @($mediaRegistrations)
        $ledgerItems.Add([ordered]@{
            video_id = [string]$item.video_id
            course_slug = [string]$item.course_slug
            source_item_id = [string]$item.c0.item_id
            source_revision_id = [string]$item.c0.revision_id
            source_asset_id = [string]$item.c0.asset_id
            source_asset_sha256 = [string]$item.c0.input_sha256
            complete_c1 = $item.complete_c1
            decision_registration = $decisionRegistration
            media_registrations = @($mediaRegistrations)
            text_sufficient = [bool]$item.essence_decision.text_sufficient
            status = 'registered'
        })
        continue
    }
    $allRuns = @(Invoke-BabataJson -Arguments @('--json', 'process', 'list-runs', '--revision', [string]$item.c0.revision_id))
    $decisionPath = Join-Path $Stage ([string]$item.essence_decision.path -replace '/', '\')
    if ((Get-Sha256 -Path $decisionPath) -ne [string]$item.essence_decision.sha256) {
        throw "C1B essence decision hash drift for $($item.video_id)."
    }
    $decisionParams = [ordered]@{
        service = 'dashscope'
        adapter = 'qianwen-vision'
        credential_source = 'environment'
        provider_input_sha256 = [string]$item.processing.provider_input_sha256
        video_input_sha256 = [string]$item.processing.video_input_sha256
        transcript_sha256 = [string]$item.processing.transcript_sha256
        fps = [double]$item.processing.fps
        pricing = [ordered]@{ estimated_cost_cny = [decimal]$item.processing.estimated_cost_cny; free_tier_assumed = $false }
    }
    $decisionRun = Find-MatchingRun -Runs $allRuns -Item $item -Kind 'structured_result' -Provider 'qianwen_skill' -Model ([string]$item.processing.model) -ToolVersion ([string]$item.processing.tool_version) -Params $decisionParams -OutputSha ([string]$item.essence_decision.sha256)
    if ($null -eq $decisionRun) {
        $receipt = Invoke-BabataJson -Arguments @(
            '--json', 'process', 'register', '--pipeline', 'agent_import',
            '--revision', [string]$item.c0.revision_id, '--item', [string]$item.c0.item_id,
            '--kind', 'structured_result', '--provider', 'qianwen_skill',
            '--model', [string]$item.processing.model, '--tool-version', [string]$item.processing.tool_version,
            '--input-sha256', [string]$item.c0.input_sha256, '--input-asset-id', [string]$item.c0.asset_id,
            '--json-file', $decisionPath, '--output-file', $decisionPath,
            '--params-json', ($decisionParams | ConvertTo-Json -Compress -Depth 20),
            '--usage-json', ($item.processing.usage | ConvertTo-Json -Compress -Depth 20),
            '--loss-notes', (@($item.essence_decision.limitations) -join ' ')
        )
        $decisionRun = [string]$receipt.run_id
        $allRuns += $receipt
        $registeredNow++
    }
    else { $reused++ }
    $decisionRegistration = Read-And-VerifyRun -RunId $decisionRun -Item $item -Kind 'structured_result' -Provider 'qianwen_skill' -Model ([string]$item.processing.model) -ToolVersion ([string]$item.processing.tool_version) -OutputSha ([string]$item.essence_decision.sha256)

    $mediaRegistrations = [Collections.Generic.List[object]]::new()
    foreach ($media in @($item.retained_derivatives)) {
        $mediaPath = Join-Path $Stage ([string]$media.path -replace '/', '\')
        if ((Get-Sha256 -Path $mediaPath) -ne [string]$media.sha256) {
            throw "C1B media hash drift for $($item.video_id)/$($media.path)."
        }
        $mediaParams = [ordered]@{
            provider_input_sha256 = [string]$item.c0.input_sha256
            preprocessing = @('ffmpeg extraction from the complete read-only C0 video')
            source_locator = $media.source_locator
            original_model_source_locator = $media.original_model_source_locator
            role = [string]$media.role
            essence_decision_sha256 = [string]$item.essence_decision.sha256
        }
        $runId = Find-MatchingRun -Runs $allRuns -Item $item -Kind ([string]$media.kind) -Provider 'local_extract' -Model 'ffmpeg' -ToolVersion $ffmpegVersion -Params $mediaParams -OutputSha ([string]$media.sha256)
        if ($null -eq $runId) {
            $receipt = Invoke-BabataJson -Arguments @(
                '--json', 'process', 'register', '--pipeline', 'agent_import',
                '--revision', [string]$item.c0.revision_id, '--item', [string]$item.c0.item_id,
                '--kind', [string]$media.kind, '--provider', 'local_extract',
                '--model', 'ffmpeg', '--tool-version', $ffmpegVersion,
                '--input-sha256', [string]$item.c0.input_sha256, '--input-asset-id', [string]$item.c0.asset_id,
                '--output-file', $mediaPath,
                '--params-json', ($mediaParams | ConvertTo-Json -Compress -Depth 20), '--usage-json', '{}',
                '--loss-notes', (@($media.loss_notes) -join ' ')
            )
            $runId = [string]$receipt.run_id
            $allRuns += $receipt
            $registeredNow++
        }
        else { $reused++ }
        $registration = Read-And-VerifyRun -RunId $runId -Item $item -Kind ([string]$media.kind) -Provider 'local_extract' -Model 'ffmpeg' -ToolVersion $ffmpegVersion -OutputSha ([string]$media.sha256)
        $registration.role = [string]$media.role
        $registration.source_locator = $media.source_locator
        $mediaRegistrations.Add($registration)
    }
    $item.registrations = @($decisionRegistration) + @($mediaRegistrations)
    $item.status = 'registered'
    $ledgerItems.Add([ordered]@{
        video_id = [string]$item.video_id
        course_slug = [string]$item.course_slug
        source_item_id = [string]$item.c0.item_id
        source_revision_id = [string]$item.c0.revision_id
        source_asset_id = [string]$item.c0.asset_id
        source_asset_sha256 = [string]$item.c0.input_sha256
        complete_c1 = $item.complete_c1
        decision_registration = $decisionRegistration
        media_registrations = @($mediaRegistrations)
        text_sufficient = [bool]$item.essence_decision.text_sufficient
        status = 'registered'
    })
}

$manifest.status = 'registered'
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 40), $utf8NoBom)
$ledger = [ordered]@{
    schema = 'babata.cherno-course-c1b-registration/v1'
    generated_at = [DateTimeOffset]::UtcNow.ToString('o')
    status = 'registered'
    scope = $LedgerScope
    source_manifest = [string]$manifest.source_manifest
    source_manifest_sha256 = [string]$manifest.source_manifest_sha256
    c1_manifest = [string]$manifest.c1_manifest
    c1_manifest_sha256 = [string]$manifest.c1_manifest_sha256
    coverage = [ordered]@{
        items = $ledgerItems.Count
        complete_c1_reused = $ledgerItems.Count
        essence_decisions_registered = @($ledgerItems.decision_registration).Count
        retained_media_registered = @($ledgerItems.media_registrations).Count
    }
    items = @($ledgerItems)
}
[IO.File]::WriteAllText($ledgerPath, ($ledger | ConvertTo-Json -Depth 40), $utf8NoBom)
$roundTrip = Get-Content -LiteralPath $ledgerPath -Raw -Encoding utf8 | ConvertFrom-Json
if ([string]$roundTrip.status -ne 'registered' -or @($roundTrip.items).Count -ne $ExpectedItems -or
    @($roundTrip.items | Where-Object status -ne 'registered').Count -ne 0) {
    throw 'C1B registration ledger read-back failed.'
}

[ordered]@{
    items_registered = @($roundTrip.items).Count
    decisions = @($roundTrip.items.decision_registration).Count
    retained_media = @($roundTrip.items.media_registrations).Count
    registered_now = $registeredNow
    reused_and_verified = $reused
    ledger = $ledgerPath
} | ConvertTo-Json
