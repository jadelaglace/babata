[CmdletBinding()]
param(
    [string]$Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\asr-manifest.json',
    [int]$BatchSize = 10,
    [int]$MaxBatches = 0,
    [switch]$RecoverOnly
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
    throw "Missing ASR manifest: $Manifest"
}
if ($BatchSize -lt 1 -or $BatchSize -gt 100) {
    throw 'BatchSize must be between 1 and 100.'
}
if ($MaxBatches -lt 0) {
    throw 'MaxBatches must be zero (unlimited) or greater.'
}
if (-not (Get-Command bl -ErrorAction SilentlyContinue)) {
    throw 'Bailian CLI (bl) is required.'
}
$userKey = [Environment]::GetEnvironmentVariable('DASHSCOPE_API_KEY', 'User')
if ([string]::IsNullOrWhiteSpace($userKey)) {
    throw 'The user-level DASHSCOPE_API_KEY is not set.'
}
$env:DASHSCOPE_API_KEY = $userKey

$document = Get-Content -LiteralPath $Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($document.items)
$root = Split-Path -Parent $Manifest
$resultsDir = Join-Path $root 'results\asr-full'
$tempDir = Join-Path $root 'provider-temp\asr-full'
New-Item -ItemType Directory -Force -Path $resultsDir, $tempDir | Out-Null
$blVersion = ((& bl --version) -replace '^bl\s+', '').Trim()
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Format-Timestamp {
    param([long]$Milliseconds)
    $time = [TimeSpan]::FromMilliseconds($Milliseconds)
    return '{0:00}:{1:00}:{2:00}.{3:000}' -f [math]::Floor($time.TotalHours), $time.Minutes, $time.Seconds, $time.Milliseconds
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
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

function Save-Candidate {
    param(
        [object]$Item,
        [object]$Response,
        [int]$PendingCount
    )
    $Response.PSObject.Properties.Remove('file_url')
    $moduleId = [string]$Item.module_id
    $jsonPath = Join-Path $resultsDir "module-$moduleId.asr.json"
    $markdownPath = Join-Path $resultsDir "module-$moduleId.transcript.md"
    $json = $Response | ConvertTo-Json -Depth 20
    [IO.File]::WriteAllText($jsonPath, $json, $utf8NoBom)

    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("# $($Item.title)")
    $lines.Add('')
    $lines.Add("Course: $($Item.course)")
    $lines.Add('')
    $sentenceCount = 0
    foreach ($transcript in @($Response.transcripts)) {
        foreach ($sentence in @($transcript.sentences)) {
            $start = Format-Timestamp -Milliseconds ([long]$sentence.begin_time)
            $end = Format-Timestamp -Milliseconds ([long]$sentence.end_time)
            $speaker = if ($null -ne $sentence.speaker_id) { " Speaker $($sentence.speaker_id)" } else { '' }
            $lines.Add("[$start --> $end]$speaker $($sentence.text)".TrimEnd())
            $sentenceCount++
        }
        if ($sentenceCount -eq 0 -and -not [string]::IsNullOrWhiteSpace([string]$transcript.text)) {
            $lines.Add([string]$transcript.text)
        }
    }
    if ($lines.Count -le 4) {
        throw "ASR result for module:$moduleId contains no transcript text."
    }
    $lines | Set-Content -LiteralPath $markdownPath -Encoding utf8

    $Item.status = 'candidate_ready'
    $Item | Add-Member -NotePropertyName provider -NotePropertyValue 'bailian_cli' -Force
    $Item | Add-Member -NotePropertyName model -NotePropertyValue 'fun-asr' -Force
    $Item | Add-Member -NotePropertyName tool_version -NotePropertyValue $blVersion -Force
    $Item | Add-Member -NotePropertyName transcript_file -NotePropertyValue $markdownPath -Force
    $Item | Add-Member -NotePropertyName transcript_sha256 -NotePropertyValue (Get-Sha256 -Path $markdownPath) -Force
    $Item | Add-Member -NotePropertyName structured_file -NotePropertyValue $jsonPath -Force
    $Item | Add-Member -NotePropertyName structured_sha256 -NotePropertyValue (Get-Sha256 -Path $jsonPath) -Force
    $Item | Add-Member -NotePropertyName sentence_count -NotePropertyValue $sentenceCount -Force
    $params = [pscustomobject]@{
        service = 'dashscope'
        adapter = 'bailian_cli'
        credential_source = 'environment'
        source_authority = 'website'
        website_module_id = [int64]$Item.module_id
        provider_input_sha256 = [string]$Item.audio_sha256
        preprocessing = @($Item.preprocessing)
        sanitization = @('Removed the provider temporary file_url before ordinary staging and C1 registration.')
        authorized_duration_seconds = [double]$Item.duration_seconds
    }
    $Item | Add-Member -NotePropertyName params -NotePropertyValue $params -Force
    $Item | Add-Member -NotePropertyName loss_notes -NotePropertyValue 'Full audio duration transcribed with native sentence and word timing where returned; visual-only content is not represented.' -Force
    $script:completed++
    $document | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Manifest -Encoding utf8
    Write-Output "[$script:completed/$PendingCount] ASR candidate module:$moduleId $($Item.title)"
}

$itemByAudioLeaf = @{}
foreach ($item in $items) {
    if (-not [string]::IsNullOrWhiteSpace([string]$item.audio_file)) {
        $leaf = Split-Path -Leaf ([string]$item.audio_file)
        if ($itemByAudioLeaf.ContainsKey($leaf)) {
            throw "Duplicate provider input filename in manifest: $leaf"
        }
        $itemByAudioLeaf[$leaf] = $item
    }
}

$script:completed = 0
foreach ($rawFile in @(Get-ChildItem -LiteralPath $tempDir -Filter '*.raw.json' -File | Sort-Object Name)) {
    $responses = @((Get-Content -LiteralPath $rawFile.FullName -Raw -Encoding utf8 | ConvertFrom-Json))
    $responseMap = Get-ResponseMap -Responses $responses
    foreach ($leaf in @($responseMap.Keys)) {
        if (-not $itemByAudioLeaf.ContainsKey($leaf)) {
            throw "Cannot recover unknown ASR provider input $leaf from $($rawFile.Name)."
        }
        $item = $itemByAudioLeaf[$leaf]
        if ($item.status -eq 'audio_ready') {
            Save-Candidate -Item $item -Response $responseMap[$leaf] -PendingCount $responses.Count
        }
    }
    Remove-Item -LiteralPath $rawFile.FullName -Force
    Write-Output "recovered and sanitized $($rawFile.Name)"
}

$pending = @($items | Where-Object status -eq 'audio_ready')
for ($offset = 0; -not $RecoverOnly -and $offset -lt $pending.Count; $offset += $BatchSize) {
    $batchNumber = [math]::Floor($offset / $BatchSize) + 1
    if ($MaxBatches -gt 0 -and $batchNumber -gt $MaxBatches) {
        break
    }
    $last = [math]::Min($offset + $BatchSize - 1, $pending.Count - 1)
    $batch = @($pending[$offset..$last])
    $rawPath = Join-Path $tempDir ("batch-{0:000}.raw.json" -f $batchNumber)
    $arguments = @('speech', 'recognize')
    foreach ($item in $batch) {
        $arguments += @('--url', [string]$item.audio_file)
    }
    $arguments += @(
        '--model', 'fun-asr',
        '--language', 'zh',
        '--out', $rawPath,
        '--output', 'json',
        '--timeout', '7200',
        '--quiet'
    )

    & bl @arguments | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Bailian ASR batch $batchNumber failed with exit code $LASTEXITCODE."
    }
    $parsed = Get-Content -LiteralPath $rawPath -Raw -Encoding utf8 | ConvertFrom-Json
    $responses = @($parsed)
    if ($responses.Count -ne $batch.Count) {
        throw "ASR batch $batchNumber returned $($responses.Count) results for $($batch.Count) inputs."
    }
    $responseMap = Get-ResponseMap -Responses $responses
    foreach ($item in $batch) {
        $localLeaf = Split-Path -Leaf ([string]$item.audio_file)
        if (-not $responseMap.ContainsKey($localLeaf)) {
            throw "ASR batch $batchNumber did not return provider input $localLeaf."
        }
    }
    foreach ($item in $batch) {
        $localLeaf = Split-Path -Leaf ([string]$item.audio_file)
        Save-Candidate -Item $item -Response $responseMap[$localLeaf] -PendingCount $pending.Count
    }
    Remove-Item -LiteralPath $rawPath -Force
}

[pscustomobject]@{
    candidates_now = $script:completed
    total_candidates = @($items | Where-Object status -eq 'candidate_ready').Count
    remaining_audio_ready = @($items | Where-Object status -eq 'audio_ready').Count
    failed = @($items | Where-Object status -eq 'failed').Count
} | ConvertTo-Json
