[CmdletBinding()]
param(
    [string]$SourceManifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\results\source-manifest.json',
    [string]$Stage = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-full-20260822-v1',
    [string]$RawDatabase = 'D:\BabataData\01_raw\index\raw.sqlite',
    [int]$ThrottleLimit = 4,
    [decimal]$UnitPriceCnyPerSecond = 0.00022,
    [string]$PricingSource = 'https://help.aliyun.com/zh/model-studio/model-pricing'
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
if ($ThrottleLimit -lt 1 -or $ThrottleLimit -gt 12) {
    throw 'ThrottleLimit must be between 1 and 12.'
}
foreach ($path in @($SourceManifest, $RawDatabase)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}
foreach ($command in @('sqlite3', 'ffmpeg', 'ffprobe')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command is unavailable: $command"
    }
}

$source = Get-Content -LiteralPath $SourceManifest -Raw -Encoding utf8 | ConvertFrom-Json
$sourceItems = @($source.items)
if ($sourceItems.Count -ne 269) {
    throw "Expected 269 frozen source items, found $($sourceItems.Count)."
}
$videoIds = @($sourceItems | ForEach-Object { [string]$_.video_id })
if (@($videoIds | Sort-Object -Unique).Count -ne 269) {
    throw 'The frozen source manifest contains duplicate video IDs.'
}

$preprocessedDir = Join-Path $Stage 'preprocessed'
$resultsDir = Join-Path $Stage 'results'
New-Item -ItemType Directory -Force -Path $preprocessedDir, $resultsDir | Out-Null

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

$c0Sql = @"
WITH ranked AS (
  SELECT
    i.source_native_id,
    i.item_id,
    r.revision_id,
    a.asset_id,
    a.logical_path,
    a.sha256,
    a.byte_size,
    a.original_filename,
    ROW_NUMBER() OVER (PARTITION BY i.source_native_id ORDER BY r.ordinal DESC) AS position
  FROM sources s
  JOIN items i ON i.source_id = s.source_id
  JOIN revisions r ON r.item_id = i.item_id
  JOIN assets a ON a.revision_id = r.revision_id
  WHERE s.provider = 'youtube'
    AND i.content_type = 'video'
    AND r.state = 'ready'
    AND a.asset_role = 'original'
    AND a.state = 'ready'
)
SELECT source_native_id,item_id,revision_id,asset_id,logical_path,sha256,byte_size,original_filename
FROM ranked
WHERE position = 1;
"@
$c0Rows = [object[]](((& sqlite3 '-json' $RawDatabase $c0Sql) -join "`n") | ConvertFrom-Json)
if ($LASTEXITCODE -ne 0) {
    throw 'Could not read full Cherno C0 bindings.'
}
$c0ByVideoId = @{}
foreach ($row in $c0Rows) {
    $c0ByVideoId[[string]$row.source_native_id] = $row
}

$jobs = [Collections.Generic.List[object]]::new()
foreach ($item in $sourceItems) {
    $videoId = [string]$item.video_id
    if (-not $c0ByVideoId.ContainsKey($videoId)) {
        throw "No ready formal C0 binding exists for $videoId."
    }
    $c0 = $c0ByVideoId[$videoId]
    if ([string]$c0.sha256 -ne [string]$item.local_media.sha256) {
        throw "Formal C0 SHA-256 differs from the frozen manifest for $videoId."
    }
    $sourcePath = [string]$item.local_media.local_path
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
        throw "Frozen source MP4 is missing for ${videoId}: $sourcePath"
    }
    if ((Get-Item -LiteralPath $sourcePath).Length -ne [long]$item.local_media.size_bytes) {
        throw "Frozen source MP4 size drift for $videoId."
    }
    $courseDir = Join-Path $preprocessedDir ([string]$item.course_slug)
    New-Item -ItemType Directory -Force -Path $courseDir | Out-Null
    $jobs.Add([pscustomobject]@{
        video_id = $videoId
        input = $sourcePath
        output = Join-Path $courseDir "$videoId.flac"
    })
}

$pending = @($jobs | Where-Object { -not (Test-Path -LiteralPath $_.output -PathType Leaf) })
$worker = {
    param([string]$VideoId, [string]$InputPath, [string]$OutputPath)
    $ErrorActionPreference = 'Continue'
    $PSNativeCommandUseErrorActionPreference = $false
    $partial = "$OutputPath.partial.flac"
    Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
    $nativeOutput = @(& ffmpeg -hide_banner -loglevel error -nostdin -y -i $InputPath -map 0:a:0 -vn -ac 1 -ar 16000 -c:a flac $partial 2>&1)
    $exitCode = $LASTEXITCODE
    $recovery = $false
    if ($exitCode -ne 0) {
        Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
        $nativeOutput = @(& ffmpeg -hide_banner -loglevel error -nostdin -y -fflags +discardcorrupt -err_detect ignore_err -i $InputPath -map 0:a:0 -vn -ac 1 -ar 16000 -c:a flac $partial 2>&1)
        $exitCode = $LASTEXITCODE
        $recovery = $true
    }
    if ($exitCode -ne 0 -or -not (Test-Path -LiteralPath $partial -PathType Leaf)) {
        Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
        $message = @($nativeOutput | Select-Object -Last 3) -join ' | '
        return [pscustomobject]@{ video_id = $VideoId; state = 'failed'; recovery = $recovery; error = "ffmpeg exit ${exitCode}: $message" }
    }
    Move-Item -LiteralPath $partial -Destination $OutputPath -Force
    return [pscustomobject]@{ video_id = $VideoId; state = 'succeeded'; recovery = $recovery; error = $null }
}
$queue = [Collections.Generic.Queue[object]]::new()
foreach ($pendingJob in $pending) { $queue.Enqueue($pendingJob) }
$running = [Collections.Generic.List[object]]::new()
$transcodeResults = [Collections.Generic.List[object]]::new()
while ($queue.Count -gt 0 -or $running.Count -gt 0) {
    while ($queue.Count -gt 0 -and $running.Count -lt $ThrottleLimit) {
        $next = $queue.Dequeue()
        $handle = Start-Job -ScriptBlock $worker -ArgumentList @(
            [string]$next.video_id,
            [string]$next.input,
            [string]$next.output
        )
        $running.Add([pscustomobject]@{ handle = $handle; video_id = [string]$next.video_id })
    }
    $completedHandle = Wait-Job -Job @($running | ForEach-Object { $_.handle }) -Any
    $entry = @($running | Where-Object { $_.handle.Id -eq $completedHandle.Id })[0]
    $result = @(Receive-Job -Job $completedHandle)
    if ($result.Count -eq 0) {
        $transcodeResults.Add([pscustomobject]@{
            video_id = [string]$entry.video_id
            state = 'failed'
            recovery = $false
            error = 'ffmpeg worker returned no result'
        })
    }
    else {
        foreach ($row in $result) { $transcodeResults.Add($row) }
    }
    Remove-Job -Job $completedHandle -Force
    [void]$running.Remove($entry)
}
$failed = @($transcodeResults | Where-Object state -ne 'succeeded')
if ($failed.Count -gt 0) {
    throw "ffmpeg failed for $($failed.Count) inputs: $($failed.video_id -join ', ')"
}
$transcodeByVideoId = @{}
foreach ($result in $transcodeResults) {
    $transcodeByVideoId[[string]$result.video_id] = $result
}

$ffmpegVersion = (& ffmpeg -version | Select-Object -First 1).Trim()
$manifestItems = [Collections.Generic.List[object]]::new()
$totalDuration = [decimal]0
$totalAudioBytes = [long]0
foreach ($item in $sourceItems) {
    $videoId = [string]$item.video_id
    $c0 = $c0ByVideoId[$videoId]
    $audioPath = Join-Path (Join-Path $preprocessedDir ([string]$item.course_slug)) "$videoId.flac"
    if (-not (Test-Path -LiteralPath $audioPath -PathType Leaf)) {
        throw "Normalized audio is missing for $videoId."
    }
    $probe = & ffprobe -v error -show_entries format=duration,size -show_entries stream=codec_name,sample_rate,channels -of json $audioPath | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or @($probe.streams).Count -ne 1) {
        throw "ffprobe failed for $videoId."
    }
    $stream = @($probe.streams)[0]
    if ([string]$stream.codec_name -ne 'flac' -or [int]$stream.sample_rate -ne 16000 -or [int]$stream.channels -ne 1) {
        throw "Normalized audio contract mismatch for $videoId."
    }
    $duration = [decimal][double]$probe.format.duration
    $sourceDuration = [decimal][double]$item.local_media.duration_seconds_local
    if ([math]::Abs([double]($duration - $sourceDuration)) -gt 1.0) {
        throw "Normalized audio duration drift for $videoId."
    }
    $audioSha = Get-Sha256 -Path $audioPath
    $audioBytes = (Get-Item -LiteralPath $audioPath).Length
    $totalDuration += $duration
    $totalAudioBytes += $audioBytes
    $preprocessing = @(
        'selected first audio stream only',
        'full authorized duration',
        'FLAC lossless audio',
        'mono',
        '16000 Hz'
    )
    $normalizationLossNotes = @()
    if ($transcodeByVideoId.ContainsKey($videoId) -and [bool]$transcodeByVideoId[$videoId].recovery) {
        $preprocessing += 'ffmpeg retry with discardcorrupt and ignore_err after native AAC decode error'
        $normalizationLossNotes += 'The source AAC stream raised a decode error; ffmpeg recovery flags were required and full-duration tolerance still passed.'
    }
    $manifestItems.Add([ordered]@{
        id = "cherno-$videoId"
        video_id = $videoId
        course_slug = [string]$item.course_slug
        course_title = [string]$item.course_title
        original_title = [string]$item.original_title
        source_url = [string]$item.source_url
        playlist_id = [string]$item.playlist_id
        playlist_title = [string]$item.playlist_title
        playlist_position_observed = [int]$item.playlist_position_observed
        c0 = [ordered]@{
            item_id = [string]$c0.item_id
            revision_id = [string]$c0.revision_id
            asset_id = [string]$c0.asset_id
            input_sha256 = [string]$c0.sha256
        }
        audio_file = $audioPath
        audio_sha256 = $audioSha
        audio_size_bytes = $audioBytes
        duration_seconds = [double]$duration
        preprocessing = $preprocessing
        normalization_loss_notes = $normalizationLossNotes
        processing = @([ordered]@{
            provider = 'bailian_cli'
            model = 'fun-asr'
            tool_version = $null
            provider_input_sha256 = $audioSha
            preprocessing = $preprocessing
            usage = [ordered]@{}
        })
        derivatives = @()
        registrations = @()
        status = 'audio_ready'
    })
}

$createdAt = [DateTimeOffset]::UtcNow.ToString('o')
$estimatedCost = [math]::Round($totalDuration * $UnitPriceCnyPerSecond, 6)
$pricing = [ordered]@{
    schema = 'babata.provider-pricing/v1'
    retrieved_at = $createdAt
    source_url = $PricingSource
    region = 'cn-beijing'
    model = 'fun-asr'
    effective_capability = 'fun-asr-2025-11-07'
    billing_unit = 'input_audio_second'
    unit_price_cny = [decimal]$UnitPriceCnyPerSecond
    page_free_tier_seconds = 36000
    free_tier_applied = 'unknown_until_billing_evidence'
    provider_reported_usage = [ordered]@{}
    total_input_seconds = [math]::Round($totalDuration, 6)
    estimated_cost_cny_before_free_tier = $estimatedCost
}
[IO.File]::WriteAllText((Join-Path $resultsDir 'pricing.json'), ($pricing | ConvertTo-Json -Depth 10), $utf8NoBom)

$manifest = [ordered]@{
    schema = 'babata.c1-staging/v1'
    task = 'cherno-course-asr-full-20260822-v1'
    created_at = $createdAt
    scope = [ordered]@{
        frozen_source_manifest = $SourceManifest
        source_manifest_sha256 = Get-Sha256 -Path $SourceManifest
        item_count = $sourceItems.Count
        total_input_seconds = [math]::Round($totalDuration, 6)
        normalized_audio_bytes = $totalAudioBytes
        ffmpeg_version = $ffmpegVersion
    }
    pricing_path = 'results/pricing.json'
    pricing = $pricing
    items = @($manifestItems)
}
$manifestPath = Join-Path $Stage 'manifest.json'
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 30), $utf8NoBom)

$report = @(
    '# Cherno full ASR round',
    '',
    "- Frozen scope: $($sourceItems.Count) videos across three explicitly authorized playlists.",
    "- Full normalized input: $([math]::Round($totalDuration, 3)) seconds; $totalAudioBytes bytes.",
    "- Estimated Fun-ASR cost before free-tier reconciliation: $estimatedCost CNY.",
    '- Source MP4 policy: read only; no source file was renamed, overwritten or deleted.',
    '- Status: normalized audio ready; ASR candidates and formal C1 registration pending.'
)
[IO.File]::WriteAllLines((Join-Path $Stage 'REPORT.md'), $report, $utf8NoBom)

$roundTrip = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if (@($roundTrip.items).Count -ne 269 -or @($roundTrip.items | Where-Object status -ne 'audio_ready').Count -ne 0) {
    throw 'Full-round manifest read-back failed.'
}

[ordered]@{
    items = @($roundTrip.items).Count
    audio_ready = @($roundTrip.items | Where-Object status -eq 'audio_ready').Count
    transcoded_now = $pending.Count
    reused_existing = $jobs.Count - $pending.Count
    total_input_seconds = [math]::Round($totalDuration, 6)
    normalized_audio_bytes = $totalAudioBytes
    estimated_cost_cny = $estimatedCost
    manifest = $manifestPath
} | ConvertTo-Json
