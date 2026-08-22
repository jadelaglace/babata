[CmdletBinding()]
param(
    [string]$Samples = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\results\representative-samples.json',
    [string]$Stage = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-pilot-20260822-v1',
    [string]$RawDatabase = 'D:\BabataData\01_raw\index\raw.sqlite',
    [decimal]$UnitPriceCnyPerSecond = 0.00022,
    [string]$PricingSource = 'https://help.aliyun.com/zh/model-studio/model-pricing',
    [switch]$RemoveRawProviderFiles
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)

foreach ($path in @($Samples, $RawDatabase)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}
foreach ($command in @('sqlite3', 'bl')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command is unavailable: $command"
    }
}

$providerDir = Join-Path $Stage 'provider'
$resultsDir = Join-Path $Stage 'results'
New-Item -ItemType Directory -Force -Path $providerDir, $resultsDir | Out-Null
$document = Get-Content -LiteralPath $Samples -Raw -Encoding utf8 | ConvertFrom-Json
$samplesById = @{}
foreach ($sample in @($document.items)) {
    $videoId = [string]$sample.video_id
    if ($videoId -notmatch '^[A-Za-z0-9_-]{11}$') {
        throw "Invalid YouTube video ID in representative sample: $videoId"
    }
    if ($samplesById.ContainsKey($videoId)) {
        throw "Duplicate representative sample video ID: $videoId"
    }
    $samplesById[$videoId] = $sample
}
if ($samplesById.Count -ne 3) {
    throw "Expected three representative samples, found $($samplesById.Count)."
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Format-Timestamp {
    param([long]$Milliseconds)
    $time = [TimeSpan]::FromMilliseconds($Milliseconds)
    return '{0:00}:{1:00}:{2:00}.{3:000}' -f [math]::Floor($time.TotalHours), $time.Minutes, $time.Seconds, $time.Milliseconds
}

function Get-C0Binding {
    param([string]$VideoId)
    $sql = @"
SELECT
  i.item_id,
  i.source_native_id,
  i.source_locator,
  r.revision_id,
  a.asset_id,
  a.logical_path,
  a.sha256,
  a.byte_size,
  a.original_filename
FROM items i
JOIN revisions r ON r.item_id = i.item_id
JOIN assets a ON a.revision_id = r.revision_id
WHERE i.source_native_id = '$VideoId'
  AND i.content_type = 'video'
  AND r.state = 'ready'
  AND a.asset_role = 'original'
  AND a.state = 'ready'
ORDER BY r.ordinal DESC
LIMIT 2;
"@
    $json = & sqlite3 '-json' $RawDatabase $sql
    if ($LASTEXITCODE -ne 0) {
        throw "sqlite3 failed while resolving C0 for $VideoId."
    }
    $rows = @($json | ConvertFrom-Json)
    if ($rows.Count -ne 1) {
        throw "Expected one ready C0 original for $VideoId, found $($rows.Count)."
    }
    return $rows[0]
}

$blVersion = ((& bl --version) -replace '^bl\s+', '').Trim()
$manifestItems = [Collections.Generic.List[object]]::new()
$reportRows = [Collections.Generic.List[object]]::new()
$totalDuration = [decimal]0
$totalEstimate = [decimal]0

foreach ($videoId in @($samplesById.Keys | Sort-Object)) {
    $sample = $samplesById[$videoId]
    $rawPath = Join-Path $providerDir "$videoId.raw.json"
    if (-not (Test-Path -LiteralPath $rawPath -PathType Leaf)) {
        throw "Missing provider result for ${videoId}: $rawPath"
    }

    $audioPath = [string]$sample.normalized_audio.path
    if (-not (Test-Path -LiteralPath $audioPath -PathType Leaf)) {
        throw "Missing normalized audio for ${videoId}: $audioPath"
    }
    $audioSha = Get-Sha256 -Path $audioPath
    if ($audioSha -ne [string]$sample.normalized_audio.provider_input_sha256) {
        throw "Normalized audio SHA-256 drift for $videoId."
    }

    $c0 = Get-C0Binding -VideoId $videoId
    if ([string]$c0.sha256 -ne [string]$sample.source_mp4_sha256) {
        throw "C0 asset SHA-256 does not match the frozen source manifest for $videoId."
    }

    $response = Get-Content -LiteralPath $rawPath -Raw -Encoding utf8 | ConvertFrom-Json
    if (-not ($response.PSObject.Properties.Name -contains 'file_url')) {
        throw "Provider result for $videoId has no temporary file_url binding."
    }
    if (@($response.transcripts).Count -ne 1) {
        throw "Expected one transcript channel for $videoId."
    }
    $transcript = @($response.transcripts)[0]
    if ([string]::IsNullOrWhiteSpace([string]$transcript.text)) {
        throw "Provider result for $videoId contains no transcript text."
    }
    $providerDurationMs = [long]$response.properties.original_duration_in_milliseconds
    $expectedDurationMs = [long][math]::Round(([double]$sample.normalized_audio.duration_seconds) * 1000)
    if ([math]::Abs($providerDurationMs - $expectedDurationMs) -gt 1000) {
        throw "Provider duration does not cover the complete normalized audio for $videoId."
    }

    $response.PSObject.Properties.Remove('file_url')
    $jsonPath = Join-Path $resultsDir "$videoId.asr.json"
    [IO.File]::WriteAllText($jsonPath, ($response | ConvertTo-Json -Depth 30), $utf8NoBom)

    $markdownPath = Join-Path $resultsDir "$videoId.transcript.md"
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("# $($sample.original_title)")
    $lines.Add('')
    $lines.Add("- Course: $($sample.course_title)")
    $lines.Add(('- Video ID: `{0}`' -f $videoId))
    $lines.Add("- Source: $($sample.source_url)")
    $lines.Add('')
    foreach ($sentence in @($transcript.sentences)) {
        $start = Format-Timestamp -Milliseconds ([long]$sentence.begin_time)
        $end = Format-Timestamp -Milliseconds ([long]$sentence.end_time)
        $lines.Add("[$start --> $end] $($sentence.text)".TrimEnd())
    }
    if (@($transcript.sentences).Count -eq 0) {
        $lines.Add([string]$transcript.text)
    }
    [IO.File]::WriteAllLines($markdownPath, $lines, $utf8NoBom)

    $duration = [decimal][double]$sample.normalized_audio.duration_seconds
    $estimate = [math]::Round($duration * $UnitPriceCnyPerSecond, 6)
    $totalDuration += $duration
    $totalEstimate += $estimate
    $params = [ordered]@{
        service = 'dashscope'
        adapter = 'bailian_cli'
        credential_source = 'environment'
        provider_input_sha256 = $audioSha
        preprocessing = @($sample.normalized_audio.preprocessing)
        sanitization = @('Removed the provider temporary file_url before ordinary staging and C1 registration.')
        authorized_duration_seconds = [double]$duration
        pricing = [ordered]@{
            currency = 'CNY'
            unit = 'input_audio_second'
            unit_price = [decimal]$UnitPriceCnyPerSecond
            estimated_cost = $estimate
            free_tier_assumed = $false
        }
    }
    $lossNotes = @(
        'Full authorized audio duration was submitted and provider original duration matched within one second.',
        'Native ASR sentence and word timing are retained where returned; visual-only content is not represented.'
    )
    $manifestItems.Add([ordered]@{
        id = "cherno-$videoId"
        video_id = $videoId
        course_slug = [string]$sample.course_slug
        c0 = [ordered]@{
            item_id = [string]$c0.item_id
            revision_id = [string]$c0.revision_id
            asset_id = [string]$c0.asset_id
            input_sha256 = [string]$c0.sha256
        }
        processing = @([ordered]@{
            provider = 'bailian_cli'
            model = 'fun-asr'
            tool_version = $blVersion
            provider_input_sha256 = $audioSha
            preprocessing = @($sample.normalized_audio.preprocessing)
            params = $params
            usage = [ordered]@{}
        })
        derivatives = @(
            [ordered]@{
                kind = 'transcript'
                path = "results/$videoId.transcript.md"
                sha256 = Get-Sha256 -Path $markdownPath
                loss_notes = $lossNotes
            },
            [ordered]@{
                kind = 'structured_result'
                path = "results/$videoId.asr.json"
                sha256 = Get-Sha256 -Path $jsonPath
                loss_notes = @('Provider temporary file_url removed; transcript structure otherwise retained.')
            }
        )
        registrations = @()
        status = 'staged_only'
    })
    $reportRows.Add([ordered]@{
        video_id = $videoId
        course = [string]$sample.course_title
        original_duration_seconds = [double]$duration
        provider_duration_seconds = $providerDurationMs / 1000.0
        sentence_count = @($transcript.sentences).Count
        estimated_cost_cny = $estimate
    })
}

$createdAt = [DateTimeOffset]::UtcNow.ToString('o')
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
    estimated_cost_cny_before_free_tier = [math]::Round($totalEstimate, 6)
}
[IO.File]::WriteAllText((Join-Path $resultsDir 'pricing.json'), ($pricing | ConvertTo-Json -Depth 10), $utf8NoBom)

$manifest = [ordered]@{
    schema = 'babata.c1-staging/v1'
    task = 'cherno-course-asr-pilot-20260822-v1'
    created_at = $createdAt
    pricing_path = 'results/pricing.json'
    items = @($manifestItems)
}
$manifestPath = Join-Path $Stage 'manifest.json'
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 30), $utf8NoBom)

$reportLines = [Collections.Generic.List[string]]::new()
$reportLines.Add('# Cherno representative ASR pilot')
$reportLines.Add('')
$reportLines.Add('- Scope: three full-duration representative videos, one per course.')
$reportLines.Add('- Provider/model: `bailian_cli` / `fun-asr`.')
$reportLines.Add(('- Pricing: {0} CNY per input audio second; official source captured in `results/pricing.json`.' -f $UnitPriceCnyPerSecond))
$reportLines.Add("- Total input: $([math]::Round($totalDuration, 3)) seconds; estimated cost before free-tier reconciliation: $([math]::Round($totalEstimate, 6)) CNY.")
$reportLines.Add('- Actual provider usage: not returned by the CLI response; recorded as `{}`.')
$reportLines.Add('- Status: staged only until both derivatives per sample are registered and read back through Babata core.')
$reportLines.Add('')
$reportLines.Add('| Video ID | Course | Input seconds | Provider seconds | Sentences | Estimated CNY |')
$reportLines.Add('| --- | --- | ---: | ---: | ---: | ---: |')
foreach ($row in $reportRows) {
    $reportLines.Add("| $($row.video_id) | $($row.course) | $($row.original_duration_seconds) | $($row.provider_duration_seconds) | $($row.sentence_count) | $($row.estimated_cost_cny) |")
}
[IO.File]::WriteAllLines((Join-Path $Stage 'REPORT.md'), $reportLines, $utf8NoBom)

$roundTrip = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if (@($roundTrip.items).Count -ne 3) {
    throw 'Manifest round-trip validation did not preserve all three items.'
}
foreach ($item in @($roundTrip.items)) {
    foreach ($derivative in @($item.derivatives)) {
        $path = Join-Path $Stage ([string]$derivative.path -replace '/', '\')
        if ((Get-Sha256 -Path $path) -ne [string]$derivative.sha256) {
            throw "Derivative hash read-back failed: $path"
        }
    }
}

if ($RemoveRawProviderFiles) {
    foreach ($videoId in @($samplesById.Keys)) {
        Remove-Item -LiteralPath (Join-Path $providerDir "$videoId.raw.json") -Force
    }
}

[ordered]@{
    schema = [string]$roundTrip.schema
    items = @($roundTrip.items).Count
    derivatives = @($roundTrip.items.derivatives).Count
    total_input_seconds = [math]::Round($totalDuration, 6)
    estimated_cost_cny = [math]::Round($totalEstimate, 6)
    raw_provider_files_removed = [bool]$RemoveRawProviderFiles
    manifest = $manifestPath
} | ConvertTo-Json
