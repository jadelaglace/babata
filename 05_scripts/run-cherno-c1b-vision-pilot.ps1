[CmdletBinding()]
param(
    [string]$SourceManifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\results\source-manifest.json',
    [string]$C1Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-pilot-20260822-v1\manifest.json',
    [string]$Stage = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-pilot-20260822-v1',
    [string]$VisionScript = 'C:\Users\Aiano\.agents\skills\qianwen-vision\scripts\analyze.py',
    [string]$Model = 'qwen3.6-plus',
    [double]$FramesPerSecond = 0.5,
    [int]$ExpectedItems = 3,
    [string]$Scope = 'representative_pilot',
    [decimal]$InputPriceCnyPerMillionTokens = 2,
    [decimal]$OutputPriceCnyPerMillionTokens = 12,
    [string]$PricingSource = 'https://platform.qianwenai.com/docs/developer-guides/getting-started/pricing'
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
foreach ($path in @($SourceManifest, $C1Manifest, $VisionScript)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}
foreach ($command in @('python', 'ffmpeg')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command is unavailable: $command"
    }
}
if ([string]::IsNullOrWhiteSpace($env:DASHSCOPE_API_KEY)) {
    $userKey = [Environment]::GetEnvironmentVariable('DASHSCOPE_API_KEY', 'User')
    if ([string]::IsNullOrWhiteSpace($userKey)) {
        throw 'DASHSCOPE_API_KEY is unavailable.'
    }
    $env:DASHSCOPE_API_KEY = $userKey
}

$source = Get-Content -LiteralPath $SourceManifest -Raw -Encoding utf8 | ConvertFrom-Json
$c1 = Get-Content -LiteralPath $C1Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($c1.items)
if ($ExpectedItems -lt 1 -or $items.Count -ne $ExpectedItems -or @($items | Where-Object status -ne 'registered').Count -ne 0) {
    throw "The vision round requires exactly $ExpectedItems formally registered C1 items."
}
$sourceByVideoId = @{}
foreach ($item in @($source.items)) {
    $sourceByVideoId[[string]$item.video_id] = $item
}

$requestsDir = Join-Path $Stage 'requests'
$providerDir = Join-Path $Stage 'provider-temp'
$resultsDir = Join-Path $Stage 'results'
$mediaDir = Join-Path $resultsDir 'media'
New-Item -ItemType Directory -Force -Path $requestsDir, $providerDir, $resultsDir, $mediaDir | Out-Null

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Write-Json {
    param([string]$Path, [object]$Value)
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 40), $utf8NoBom)
}

function Get-TokenCount {
    param([object]$Usage, [string[]]$Names)
    foreach ($name in $Names) {
        if ($Usage.PSObject.Properties.Name -contains $name) {
            return [long]$Usage.$name
        }
    }
    return [long]0
}

$schema = [ordered]@{
    type = 'object'
    additionalProperties = $false
    required = @('text_sufficient', 'decision_basis', 'segments', 'limitations')
    properties = [ordered]@{
        text_sufficient = [ordered]@{ type = 'boolean' }
        decision_basis = [ordered]@{ type = 'string' }
        segments = [ordered]@{
            type = 'array'
            maxItems = 8
            items = [ordered]@{
                type = 'object'
                additionalProperties = $false
                required = @('type', 'start_seconds', 'end_seconds', 'frame_timestamp_seconds', 'visual_role', 'summary', 'why_text_insufficient')
                properties = [ordered]@{
                    type = [ordered]@{ type = 'string'; enum = @('key_frame', 'video_excerpt') }
                    start_seconds = [ordered]@{ type = 'number'; minimum = 0 }
                    end_seconds = [ordered]@{ type = 'number'; minimum = 0 }
                    frame_timestamp_seconds = [ordered]@{ type = 'number'; minimum = 0 }
                    visual_role = [ordered]@{ type = 'string'; enum = @('code', 'ui_operation', 'rendering_output', 'diagram', 'animation', 'other') }
                    summary = [ordered]@{ type = 'string' }
                    why_text_insufficient = [ordered]@{ type = 'string' }
                }
            }
        }
        limitations = [ordered]@{ type = 'array'; items = [ordered]@{ type = 'string' } }
    }
}

$manifestItems = [Collections.Generic.List[object]]::new()
$visionToolHash = Get-Sha256 -Path $VisionScript
$totalPromptTokens = [long]0
$totalCompletionTokens = [long]0
$totalEstimatedCost = [decimal]0
$updateSignal = $false

foreach ($c1Item in $items) {
    $videoId = [string]$c1Item.video_id
    if (-not $sourceByVideoId.ContainsKey($videoId)) {
        throw "No frozen source manifest row exists for $videoId."
    }
    $sourceItem = $sourceByVideoId[$videoId]
    $videoPath = [string]$sourceItem.local_media.local_path
    if (-not (Test-Path -LiteralPath $videoPath -PathType Leaf)) {
        throw "Source video is missing for ${videoId}: $videoPath"
    }
    if ((Get-Sha256 -Path $videoPath) -ne [string]$c1Item.c0.input_sha256) {
        throw "Source video hash drift for $videoId."
    }
    $transcriptDerivative = @($c1Item.derivatives | Where-Object kind -eq 'transcript')
    $transcriptRegistration = @($c1Item.registrations | Where-Object kind -eq 'transcript')
    if ($transcriptDerivative.Count -ne 1 -or $transcriptRegistration.Count -ne 1) {
        throw "The complete transcript identity is ambiguous for $videoId."
    }
    $transcriptPath = Join-Path (Split-Path -Parent $C1Manifest) ([string]$transcriptDerivative[0].path -replace '/', '\')
    if ((Get-Sha256 -Path $transcriptPath) -ne [string]$transcriptDerivative[0].sha256) {
        throw "Transcript hash drift for $videoId."
    }
    $transcript = Get-Content -LiteralPath $transcriptPath -Raw -Encoding utf8
    $prompt = @"
You are making a formal C1B essence judgment for a programming-course lesson.
Analyze the complete video chronologically and use the timestamped ASR transcript below as context.
Retain only visual evidence that materially changes understanding and cannot be represented reliably by the transcript alone:
source code state, IDE/editor operations, build/debug UI, diagrams, rendered graphics, or time-continuous animation.
Ignore talking-head footage, branding, decorative shots, repeated unchanged code, and evidence already explicit in the transcript.
Use key_frame for a stable code/UI/result state. Use video_excerpt only when motion or interaction sequence is essential.
Choose at most eight non-overlapping segments. Keep video excerpts between 3 and 20 seconds and keep their total duration under 90 seconds.
Set text_sufficient=true and return no segments only when the transcript genuinely preserves the whole lesson.

Course: $($sourceItem.course_title)
Lesson: $($sourceItem.original_title)
Video ID: $videoId
Duration seconds: $($sourceItem.local_media.duration_seconds_local)

Timestamped transcript:
$transcript
"@
    $request = [ordered]@{
        prompt = $prompt
        video = $videoPath
        fps = $FramesPerSecond
        model = $Model
        json_mode = $true
        schema = $schema
        enable_thinking = $false
        max_tokens = 3000
        temperature = 0
        timeout_s = 1800
        max_retries = 2
    }
    $requestPath = Join-Path $requestsDir "$videoId.request.json"
    Write-Json -Path $requestPath -Value $request
    $requestHash = Get-Sha256 -Path $requestPath
    $responsePath = Join-Path $providerDir "$videoId.response.json"
    $responseRequestHashPath = Join-Path $providerDir "$videoId.request.sha256"
    if (Test-Path -LiteralPath $responsePath -PathType Leaf) {
        if (-not (Test-Path -LiteralPath $responseRequestHashPath -PathType Leaf) -or
            (Get-Content -LiteralPath $responseRequestHashPath -Raw -Encoding ascii).Trim() -ne $requestHash) {
            throw "Existing Qwen response is not bound to the current request hash for $videoId."
        }
    }
    if (-not (Test-Path -LiteralPath $responsePath -PathType Leaf)) {
        $previousErrorAction = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $commandOutput = @(& python $VisionScript --file $requestPath --upload-files --output $responsePath 2>&1)
        $pythonExitCode = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorAction
        if ($pythonExitCode -ne 0) {
            throw "Qwen vision failed for ${videoId}: $($commandOutput -join ' | ')"
        }
        if (($commandOutput -join "`n") -match '\[UPDATE_AVAILABLE\]') {
            $updateSignal = $true
        }
        [IO.File]::WriteAllText($responseRequestHashPath, $requestHash, [Text.Encoding]::ASCII)
    }
    $response = Get-Content -LiteralPath $responsePath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($null -eq $response.json -or [string]::IsNullOrWhiteSpace([string]$response.model)) {
        throw "Qwen vision returned no structured result for $videoId."
    }
    $decisionRows = @($response.json)
    if ($decisionRows.Count -ne 1) {
        throw "Qwen vision must return exactly one C1B decision for $videoId."
    }
    $decision = $decisionRows[0]
    $segments = @($decision.segments)
    if ([bool]$decision.text_sufficient -and $segments.Count -ne 0) {
        throw "A text-sufficient decision retained visual segments for $videoId."
    }
    if (-not [bool]$decision.text_sufficient -and $segments.Count -eq 0) {
        throw "A text-insufficient decision retained no visual evidence for $videoId."
    }
    $duration = [double]$sourceItem.local_media.duration_seconds_local
    $excerptSeconds = [double]0
    $previousEnd = [double]-1
    $derivatives = [Collections.Generic.List[object]]::new()
    $rejectedSegments = [Collections.Generic.List[object]]::new()
    $index = 0
    foreach ($segment in $segments) {
        $index++
        $start = [double]$segment.start_seconds
        $end = [double]$segment.end_seconds
        $frame = [double]$segment.frame_timestamp_seconds
        if ($start -lt 0 -or $end -le $start -or $end -gt ($duration + 1) -or $frame -lt $start -or $frame -gt $end) {
            $rejectedSegments.Add([ordered]@{ segment = $segment; reason = 'The model locator was outside the authoritative video duration or internally inconsistent.' })
            continue
        }
        if ($start -lt $previousEnd) {
            $rejectedSegments.Add([ordered]@{ segment = $segment; reason = 'The model locator overlapped an earlier accepted segment or was out of chronological order.' })
            continue
        }
        $previousEnd = $end
        $originalLocator = [ordered]@{ start_seconds = $start; end_seconds = $end; frame_timestamp_seconds = $frame }
        $normalizedType = [string]$segment.type
        $normalizationNotes = [Collections.Generic.List[string]]::new()
        if ($normalizedType -eq 'video_excerpt' -and [string]$segment.visual_role -in @('code', 'rendering_output', 'diagram')) {
            $normalizedType = 'key_frame'
            $normalizationNotes.Add('Converted a stable code/diagram/rendering-state suggestion from video excerpt to a single key frame.')
        }
        elseif ($normalizedType -eq 'video_excerpt' -and ($end - $start) -gt 20) {
            $start = [math]::Max(0, $frame - 10)
            $end = [math]::Min($duration, $start + 20)
            $start = [math]::Max(0, $end - 20)
            $normalizationNotes.Add('Trimmed an overlong model suggestion to a maximum 20-second excerpt centered on the suggested evidence frame.')
        }
        $baseName = '{0}-{1:000}' -f $videoId, $index
        if ($normalizedType -eq 'key_frame') {
            $outputPath = Join-Path $mediaDir "$baseName.png"
            if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
                & ffmpeg -hide_banner -loglevel error -nostdin -y -ss $frame -i $videoPath -frames:v 1 $outputPath
                if ($LASTEXITCODE -ne 0) { throw "Key-frame extraction failed for $videoId segment $index." }
            }
            $kind = 'key_frame'
            $modality = 'image'
        }
        elseif ($normalizedType -eq 'video_excerpt') {
            $clipDuration = $end - $start
            if ($clipDuration -lt 3 -or $clipDuration -gt 20) {
                throw "Video excerpt duration is outside the 3-20 second pilot contract for $videoId segment $index."
            }
            $excerptSeconds += $clipDuration
            $outputPath = Join-Path $mediaDir "$baseName.mp4"
            if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
                & ffmpeg -hide_banner -loglevel error -nostdin -y -ss $start -i $videoPath -t $clipDuration -map '0:v:0' -map '0:a:0?' -c:v libx264 -preset medium -crf 18 -c:a aac -b:a 160k -movflags '+faststart' $outputPath
                if ($LASTEXITCODE -ne 0) { throw "Video-excerpt extraction failed for $videoId segment $index." }
            }
            $kind = 'video_excerpt'
            $modality = 'video'
        }
        else {
            throw "Unsupported C1B segment type for ${videoId}: $normalizedType"
        }
        $derivatives.Add([ordered]@{
            kind = $kind
            modality = $modality
            path = 'results/media/' + (Split-Path -Leaf $outputPath)
            sha256 = Get-Sha256 -Path $outputPath
            source_locator = [ordered]@{ start_seconds = $start; end_seconds = $end; frame_timestamp_seconds = $frame }
            original_model_source_locator = $originalLocator
            role = [string]$segment.visual_role
            summary = [string]$segment.summary
            why_text_insufficient = [string]$segment.why_text_insufficient
            loss_notes = @('Selected from a 0.5 fps full-video model review; exact code text may still require the retained source pixels.') + @($normalizationNotes)
        })
    }
    if ($excerptSeconds -gt 90) {
        throw "Total retained video duration exceeds 90 seconds for $videoId."
    }
    if (-not [bool]$decision.text_sufficient -and $derivatives.Count -eq 0) {
        throw "No valid retained visual evidence remained for text-insufficient item $videoId."
    }
    $decision | Add-Member -NotePropertyName normalized_retained_derivatives -NotePropertyValue @($derivatives) -Force
    $decision | Add-Member -NotePropertyName rejected_segments -NotePropertyValue @($rejectedSegments) -Force
    $decisionPath = Join-Path $resultsDir "$videoId.essence.json"
    Write-Json -Path $decisionPath -Value $decision

    $promptTokens = Get-TokenCount -Usage $response.usage -Names @('prompt_tokens', 'input_tokens')
    $completionTokens = Get-TokenCount -Usage $response.usage -Names @('completion_tokens', 'output_tokens')
    $estimatedCost = [math]::Round(
        ([decimal]$promptTokens / 1000000 * $InputPriceCnyPerMillionTokens) +
        ([decimal]$completionTokens / 1000000 * $OutputPriceCnyPerMillionTokens),
        6
    )
    $totalPromptTokens += $promptTokens
    $totalCompletionTokens += $completionTokens
    $totalEstimatedCost += $estimatedCost
    $manifestItems.Add([ordered]@{
        video_id = $videoId
        course_slug = [string]$sourceItem.course_slug
        original_title = [string]$sourceItem.original_title
        c0 = $c1Item.c0
        complete_c1 = $transcriptRegistration[0]
        processing = [ordered]@{
            provider = 'qianwen_skill'
            service = 'dashscope'
            adapter = 'qianwen-vision'
            model = [string]$response.model
            tool_version = "analyze.py@sha256:$visionToolHash"
            credential_source = 'environment'
            provider_input_sha256 = $requestHash
            video_input_sha256 = [string]$c1Item.c0.input_sha256
            transcript_sha256 = [string]$transcriptDerivative[0].sha256
            fps = $FramesPerSecond
            usage = $response.usage
            estimated_cost_cny = $estimatedCost
        }
        essence_decision = [ordered]@{
            path = 'results/' + (Split-Path -Leaf $decisionPath)
            sha256 = Get-Sha256 -Path $decisionPath
            text_sufficient = [bool]$decision.text_sufficient
            decision_basis = [string]$decision.decision_basis
            limitations = @($decision.limitations)
        }
        retained_derivatives = @($derivatives)
        registrations = @()
        status = 'staged_only'
    })
    Write-Json -Path (Join-Path $Stage 'progress.json') -Value ([ordered]@{
        schema = 'babata.cherno-c1b-vision-progress/v1'
        updated_at = [DateTimeOffset]::UtcNow.ToString('o')
        status = 'in_progress'
        scope = $Scope
        expected_items = $ExpectedItems
        completed_items = $manifestItems.Count
        retained_derivatives = @($manifestItems.retained_derivatives).Count
        estimated_cost_cny = [math]::Round($totalEstimatedCost, 6)
    })
}

$createdAt = [DateTimeOffset]::UtcNow.ToString('o')
$pricing = [ordered]@{
    schema = 'babata.provider-pricing/v1'
    retrieved_at = $createdAt
    source_url = $PricingSource
    model = $Model
    currency = 'CNY'
    input_price_per_million_tokens = $InputPriceCnyPerMillionTokens
    output_price_per_million_tokens = $OutputPriceCnyPerMillionTokens
    prompt_tokens = $totalPromptTokens
    completion_tokens = $totalCompletionTokens
    estimated_cost_cny = [math]::Round($totalEstimatedCost, 6)
    free_tier_applied = 'unknown_until_billing_evidence'
}
$manifest = [ordered]@{
    schema = 'babata.cherno-c1b-vision/v1'
    created_at = $createdAt
    status = 'staged_only'
    scope = $Scope
    source_manifest = $SourceManifest
    source_manifest_sha256 = Get-Sha256 -Path $SourceManifest
    c1_manifest = $C1Manifest
    c1_manifest_sha256 = Get-Sha256 -Path $C1Manifest
    pricing = $pricing
    update_available_signal_observed = $updateSignal
    items = @($manifestItems)
}
$manifestPath = Join-Path $Stage 'manifest.json'
Write-Json -Path $manifestPath -Value $manifest

$report = @(
    "# Cherno C1B vision round: $Scope",
    '',
    "- Items: $($manifestItems.Count)/$ExpectedItems staged.",
    "- Retained visual derivatives: $(@($manifestItems.retained_derivatives).Count).",
    "- Qwen prompt/completion tokens: $totalPromptTokens / $totalCompletionTokens.",
    "- Estimated model cost before free-tier reconciliation: $([math]::Round($totalEstimatedCost, 6)) CNY.",
    '- Status: staging only; formal C1B registration is still required.',
    '- Source MP4 files remained read only.'
)
[IO.File]::WriteAllLines((Join-Path $Stage 'REPORT.md'), $report, $utf8NoBom)

[ordered]@{
    items = $manifestItems.Count
    retained_derivatives = @($manifestItems.retained_derivatives).Count
    prompt_tokens = $totalPromptTokens
    completion_tokens = $totalCompletionTokens
    estimated_cost_cny = [math]::Round($totalEstimatedCost, 6)
    update_available_signal_observed = $updateSignal
    manifest = $manifestPath
} | ConvertTo-Json
