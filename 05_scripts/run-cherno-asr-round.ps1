[CmdletBinding()]
param(
    [string]$Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-full-20260822-v1\manifest.json',
    [string]$ReuseManifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-pilot-20260822-v1\manifest.json',
    [int]$BatchSize = 5,
    [int]$MaxBatches = 0,
    [switch]$RecoverOnly
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
    throw "Missing ASR manifest: $Manifest"
}
if ($BatchSize -lt 1 -or $BatchSize -gt 100) {
    throw 'BatchSize must be between 1 and 100.'
}
if ($MaxBatches -lt 0) {
    throw 'MaxBatches must be zero or greater.'
}
if (-not (Get-Command bl -ErrorAction SilentlyContinue)) {
    throw 'Bailian CLI (bl) is required.'
}
if ([string]::IsNullOrWhiteSpace($env:DASHSCOPE_API_KEY)) {
    $userKey = [Environment]::GetEnvironmentVariable('DASHSCOPE_API_KEY', 'User')
    if ([string]::IsNullOrWhiteSpace($userKey)) {
        throw 'DASHSCOPE_API_KEY is unavailable.'
    }
    $env:DASHSCOPE_API_KEY = $userKey
}

$document = Get-Content -LiteralPath $Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($document.items)
$root = Split-Path -Parent $Manifest
$resultsDir = Join-Path $root 'results\asr'
$tempDir = Join-Path $root 'provider-temp\asr'
New-Item -ItemType Directory -Force -Path $resultsDir, $tempDir | Out-Null
$blVersion = ((& bl --version) -replace '^bl\s+', '').Trim()

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Format-Timestamp {
    param([long]$Milliseconds)
    $time = [TimeSpan]::FromMilliseconds($Milliseconds)
    return '{0:00}:{1:00}:{2:00}.{3:000}' -f [math]::Floor($time.TotalHours), $time.Minutes, $time.Seconds, $time.Milliseconds
}

function Get-ResponseMap {
    param([object[]]$Responses)
    $map = @{}
    foreach ($response in $Responses) {
        if (-not ($response.PSObject.Properties.Name -contains 'file_url')) {
            throw 'ASR result has no file_url; cannot bind it to a provider input.'
        }
        $remotePath = ([uri][string]$response.file_url).AbsolutePath
        $remoteLeaf = [uri]::UnescapeDataString(($remotePath -split '/')[-1])
        if ([string]::IsNullOrWhiteSpace($remoteLeaf) -or $map.ContainsKey($remoteLeaf)) {
            throw "ASR response contains a missing or duplicate input filename: $remoteLeaf"
        }
        $map[$remoteLeaf] = $response
    }
    return $map
}

function Save-Manifest {
    [IO.File]::WriteAllText($Manifest, ($document | ConvertTo-Json -Depth 30), $utf8NoBom)
}

function Save-Candidate {
    param([object]$Item, [object]$Response)
    $videoId = [string]$Item.video_id
    $providerDurationMs = [long]$Response.properties.original_duration_in_milliseconds
    $expectedDurationMs = [long][math]::Round(([double]$Item.duration_seconds) * 1000)
    if ([math]::Abs($providerDurationMs - $expectedDurationMs) -gt 1000) {
        throw "Provider duration does not cover the complete normalized audio for $videoId."
    }
    $transcripts = @($Response.transcripts)
    if ($transcripts.Count -ne 1 -or [string]::IsNullOrWhiteSpace([string]$transcripts[0].text)) {
        throw "ASR result for $videoId contains no complete transcript."
    }
    $Response.PSObject.Properties.Remove('file_url')

    $jsonPath = Join-Path $resultsDir "$videoId.asr.json"
    [IO.File]::WriteAllText($jsonPath, ($Response | ConvertTo-Json -Depth 30), $utf8NoBom)
    $markdownPath = Join-Path $resultsDir "$videoId.transcript.md"
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("# $($Item.original_title)")
    $lines.Add('')
    $lines.Add("- Course: $($Item.course_title)")
    $lines.Add(('- Video ID: `{0}`' -f $videoId))
    $lines.Add("- Source: $($Item.source_url)")
    $lines.Add('')
    $sentenceCount = 0
    foreach ($transcript in $transcripts) {
        foreach ($sentence in @($transcript.sentences)) {
            $start = Format-Timestamp -Milliseconds ([long]$sentence.begin_time)
            $end = Format-Timestamp -Milliseconds ([long]$sentence.end_time)
            $lines.Add("[$start --> $end] $($sentence.text)".TrimEnd())
            $sentenceCount++
        }
        if ($sentenceCount -eq 0) {
            $lines.Add([string]$transcript.text)
        }
    }
    [IO.File]::WriteAllLines($markdownPath, $lines, $utf8NoBom)

    $estimate = [math]::Round(([decimal][double]$Item.duration_seconds) * [decimal]$document.pricing.unit_price_cny, 6)
    $params = [ordered]@{
        service = 'dashscope'
        adapter = 'bailian_cli'
        credential_source = 'environment'
        provider_input_sha256 = [string]$Item.audio_sha256
        preprocessing = @($Item.preprocessing)
        sanitization = @('Removed the provider temporary file_url before ordinary staging and C1 registration.')
        authorized_duration_seconds = [double]$Item.duration_seconds
        pricing = [ordered]@{
            currency = 'CNY'
            unit = 'input_audio_second'
            unit_price = [decimal]$document.pricing.unit_price_cny
            estimated_cost = $estimate
            free_tier_assumed = $false
        }
    }
    $Item.processing[0].tool_version = $blVersion
    $Item.processing[0] | Add-Member -NotePropertyName params -NotePropertyValue $params -Force
    $Item.processing[0].usage = [ordered]@{}
    $Item.derivatives = @(
        [ordered]@{
            kind = 'transcript'
            path = "results/asr/$videoId.transcript.md"
            sha256 = Get-Sha256 -Path $markdownPath
            loss_notes = @(
                'Full authorized audio duration was submitted and provider original duration matched within one second.',
                'Native ASR sentence and word timing are retained where returned; visual-only content is not represented.'
            )
        },
        [ordered]@{
            kind = 'structured_result'
            path = "results/asr/$videoId.asr.json"
            sha256 = Get-Sha256 -Path $jsonPath
            loss_notes = @('Provider temporary file_url removed; transcript structure otherwise retained.')
        }
    )
    $Item | Add-Member -NotePropertyName sentence_count -NotePropertyValue $sentenceCount -Force
    $Item.PSObject.Properties.Remove('staging_error')
    $Item.status = 'candidate_ready'
    Save-Manifest
    $script:completed++
}

$itemByAudioLeaf = @{}
foreach ($item in $items) {
    $leaf = Split-Path -Leaf ([string]$item.audio_file)
    if ($itemByAudioLeaf.ContainsKey($leaf)) {
        throw "Duplicate provider input filename in manifest: $leaf"
    }
    $itemByAudioLeaf[$leaf] = $item
}

$script:completed = 0
$script:failedBatches = 0
$script:reusedCandidates = 0
$script:reusedSeconds = [decimal]0
if (-not [string]::IsNullOrWhiteSpace($ReuseManifest) -and (Test-Path -LiteralPath $ReuseManifest -PathType Leaf)) {
    $reuseDocument = Get-Content -LiteralPath $ReuseManifest -Raw -Encoding utf8 | ConvertFrom-Json
    foreach ($reuseItem in @($reuseDocument.items | Where-Object status -eq 'registered')) {
        $videoId = [string]$reuseItem.video_id
        $target = @($items | Where-Object { [string]$_.video_id -eq $videoId })
        if ($target.Count -ne 1) {
            throw "Reusable ASR item is absent or ambiguous in the full manifest: $videoId"
        }
        $target = $target[0]
        if ([string]$target.status -ne 'audio_ready') { continue }
        foreach ($field in @('item_id', 'input_sha256')) {
            if ([string]$target.c0.$field -ne [string]$reuseItem.c0.$field) {
                throw "Reusable ASR C0 identity mismatch for $videoId/$field."
            }
        }
        if ([string]$target.c0.revision_id -ne [string]$reuseItem.c0.revision_id -or
            [string]$target.c0.asset_id -ne [string]$reuseItem.c0.asset_id) {
            $target | Add-Member -NotePropertyName latest_identical_c0_capture -NotePropertyValue $target.c0 -Force
            $target | Add-Member -NotePropertyName c1_reuse_reason -NotePropertyValue 'Reused the existing formal C1 bound to an earlier C0 revision of the same item and exact source bytes; the later duplicate capture remains auditable.' -Force
            $target.c0 = $reuseItem.c0
        }
        $reuseProcessing = @($reuseItem.processing)
        if ($reuseProcessing.Count -ne 1 -or
            [string]$reuseProcessing[0].provider_input_sha256 -ne [string]$target.audio_sha256) {
            throw "Reusable ASR provider input mismatch for $videoId."
        }
        $reusedDerivatives = [Collections.Generic.List[object]]::new()
        foreach ($derivative in @($reuseItem.derivatives)) {
            $sourcePath = Join-Path (Split-Path -Parent $ReuseManifest) ([string]$derivative.path -replace '/', '\')
            if ((Get-Sha256 -Path $sourcePath) -ne [string]$derivative.sha256) {
                throw "Reusable ASR derivative hash drift for $videoId/$($derivative.kind)."
            }
            $leaf = Split-Path -Leaf $sourcePath
            $destination = Join-Path $resultsDir $leaf
            Copy-Item -LiteralPath $sourcePath -Destination $destination -Force
            if ((Get-Sha256 -Path $destination) -ne [string]$derivative.sha256) {
                throw "Copied ASR derivative hash mismatch for $videoId/$($derivative.kind)."
            }
            $copy = $derivative | Select-Object *
            $copy.path = "results/asr/$leaf"
            $reusedDerivatives.Add($copy)
        }
        if ($reusedDerivatives.Count -ne 2) {
            throw "Reusable ASR derivative denominator mismatch for $videoId."
        }
        $target.processing = $reuseProcessing
        $target.derivatives = @($reusedDerivatives)
        $target.registrations = @()
        if ($reuseItem.PSObject.Properties.Name -contains 'sentence_count') {
            $target | Add-Member -NotePropertyName sentence_count -NotePropertyValue ([int]$reuseItem.sentence_count) -Force
        }
        $target.status = 'candidate_ready'
        $script:reusedCandidates++
        $script:reusedSeconds += [decimal][double]$target.duration_seconds
    }
    if ($script:reusedCandidates -gt 0) {
        $incrementalSeconds = [decimal][double]$document.pricing.total_input_seconds - $script:reusedSeconds
        $incrementalCost = [math]::Round($incrementalSeconds * [decimal]$document.pricing.unit_price_cny, 6)
        $document.pricing | Add-Member -NotePropertyName reused_input_seconds -NotePropertyValue ([math]::Round($script:reusedSeconds, 6)) -Force
        $document.pricing | Add-Member -NotePropertyName incremental_input_seconds -NotePropertyValue ([math]::Round($incrementalSeconds, 6)) -Force
        $document.pricing | Add-Member -NotePropertyName incremental_estimated_cost_cny_before_free_tier -NotePropertyValue $incrementalCost -Force
        $pricingPath = Join-Path $root ([string]$document.pricing_path -replace '/', '\')
        [IO.File]::WriteAllText($pricingPath, ($document.pricing | ConvertTo-Json -Depth 20), $utf8NoBom)
        Save-Manifest
    }
}
foreach ($rawFile in @(Get-ChildItem -LiteralPath $tempDir -Filter '*.raw.json' -File | Sort-Object Name)) {
    try {
        $responses = @((Get-Content -LiteralPath $rawFile.FullName -Raw -Encoding utf8 | ConvertFrom-Json))
        $responseMap = Get-ResponseMap -Responses $responses
        foreach ($leaf in @($responseMap.Keys)) {
            if (-not $itemByAudioLeaf.ContainsKey($leaf)) {
                throw "Cannot recover unknown ASR provider input $leaf."
            }
            $item = $itemByAudioLeaf[$leaf]
            if ([string]$item.status -in @('audio_ready', 'failed')) {
                Save-Candidate -Item $item -Response $responseMap[$leaf]
            }
        }
    }
    finally {
        Remove-Item -LiteralPath $rawFile.FullName -Force -ErrorAction SilentlyContinue
    }
}

$pending = @($items | Where-Object { [string]$_.status -in @('audio_ready', 'failed') })
for ($offset = 0; -not $RecoverOnly -and $offset -lt $pending.Count; $offset += $BatchSize) {
    $batchNumber = [math]::Floor($offset / $BatchSize) + 1
    if ($MaxBatches -gt 0 -and $batchNumber -gt $MaxBatches) {
        break
    }
    $last = [math]::Min($offset + $BatchSize - 1, $pending.Count - 1)
    $batch = @($pending[$offset..$last])
    $rawPath = Join-Path $tempDir ("batch-{0:000}-{1}.raw.json" -f $batchNumber, $batch[0].video_id)
    $arguments = @('speech', 'recognize')
    foreach ($item in $batch) {
        $arguments += @('--url', [string]$item.audio_file)
    }
    $arguments += @(
        '--model', 'fun-asr',
        '--language', 'en',
        '--out', $rawPath,
        '--output', 'json',
        '--timeout', '7200',
        '--quiet'
    )

    $succeeded = $false
    $lastError = $null
    for ($attempt = 1; $attempt -le 2 -and -not $succeeded; $attempt++) {
        Remove-Item -LiteralPath $rawPath -Force -ErrorAction SilentlyContinue
        try {
            & bl @arguments | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "bl exit $LASTEXITCODE"
            }
            $responses = @((Get-Content -LiteralPath $rawPath -Raw -Encoding utf8 | ConvertFrom-Json))
            if ($responses.Count -ne $batch.Count) {
                throw "returned $($responses.Count) results for $($batch.Count) inputs"
            }
            $responseMap = Get-ResponseMap -Responses $responses
            foreach ($item in $batch) {
                $leaf = Split-Path -Leaf ([string]$item.audio_file)
                if (-not $responseMap.ContainsKey($leaf)) {
                    throw "missing provider response for $leaf"
                }
            }
            foreach ($item in $batch) {
                $leaf = Split-Path -Leaf ([string]$item.audio_file)
                Save-Candidate -Item $item -Response $responseMap[$leaf]
            }
            $succeeded = $true
        }
        catch {
            $lastError = $_.Exception.Message
        }
        finally {
            Remove-Item -LiteralPath $rawPath -Force -ErrorAction SilentlyContinue
        }
    }
    if (-not $succeeded) {
        $script:failedBatches++
        foreach ($item in $batch) {
            $item.status = 'failed'
            $item | Add-Member -NotePropertyName staging_error -NotePropertyValue $lastError -Force
        }
        Save-Manifest
    }
}

$ready = @($items | Where-Object { [string]$_.status -in @('candidate_ready', 'registered') }).Count
$failed = @($items | Where-Object status -eq 'failed').Count
$remaining = @($items | Where-Object status -eq 'audio_ready').Count
$reportPath = Join-Path $root 'REPORT.md'
if (Test-Path -LiteralPath $reportPath -PathType Leaf) {
    $report = Get-Content -LiteralPath $reportPath -Raw -Encoding utf8
    $report = [regex]::Replace($report, '(?m)^- Status:.*$', "- Status: ASR candidate_ready $ready/$($items.Count); failed $failed; audio_ready $remaining.")
    [IO.File]::WriteAllText($reportPath, $report, $utf8NoBom)
}

[ordered]@{
    candidates_now = $script:completed
    candidates_reused = $script:reusedCandidates
    reused_input_seconds = [math]::Round($script:reusedSeconds, 6)
    total_candidates = $ready
    remaining_audio_ready = $remaining
    failed = $failed
    failed_batches = $script:failedBatches
} | ConvertTo-Json
