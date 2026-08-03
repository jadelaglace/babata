[CmdletBinding()]
param(
    [string]$ProbeLedger = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\coverage\pending-video-probe-ledger.csv',
    [string]$OutputDir = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803',
    [int[]]$CourseOrder = @(3, 4, 2),
    [switch]$All
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ProbeLedger -PathType Leaf)) {
    throw "Missing video probe ledger: $ProbeLedger"
}
foreach ($command in @('ffmpeg', 'ffprobe')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "$command is required."
    }
}

$audioDir = Join-Path $OutputDir 'preprocessed\asr-full'
$manifestPath = Join-Path $OutputDir 'asr-manifest.json'
New-Item -ItemType Directory -Force -Path $audioDir | Out-Null

$existingByModule = @{}
if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
    $existing = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    foreach ($item in @($existing.items)) {
        $existingByModule[[string]$item.module_id] = $item
    }
}

$priority = @{ 3 = 0; 4 = 1; 2 = 2 }
$rows = @(Import-Csv -LiteralPath $ProbeLedger)
if (-not $All) {
    $rows = @($rows | Where-Object { [int]$_.course_order -in $CourseOrder })
}
$rows = @($rows | Sort-Object @{e={ if ($priority.ContainsKey([int]$_.course_order)) { $priority[[int]$_.course_order] } else { 99 } }}, @{e={[int]$_.course_order}}, @{e={[int64]$_.module_id}})
$items = [Collections.Generic.List[object]]::new()
$selectedModules = @{}
foreach ($row in $rows) {
    $selectedModules[[string]$row.module_id] = $true
}
foreach ($previous in @($existingByModule.Values | Sort-Object course_order, module_id)) {
    if (-not $selectedModules.ContainsKey([string]$previous.module_id) -and
        $previous.status -in @('audio_ready', 'candidate_ready', 'registered')) {
        $items.Add($previous)
    }
}

for ($index = 0; $index -lt $rows.Count; $index++) {
    $row = $rows[$index]
    $moduleKey = [string]$row.module_id
    $previous = $existingByModule[$moduleKey]
    if ($null -ne $previous -and $previous.status -in @('audio_ready', 'candidate_ready', 'registered')) {
        if (Test-Path -LiteralPath ([string]$previous.audio_file) -PathType Leaf) {
            $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath ([string]$previous.audio_file)).Hash.ToLowerInvariant()
            if ($actual -eq [string]$previous.audio_sha256) {
                $items.Add($previous)
                Write-Output "[$($index + 1)/$($rows.Count)] reused module:$moduleKey $($row.title)"
                continue
            }
        }
    }

    $item = [ordered]@{
        module_id = [int64]$row.module_id
        course_order = [int]$row.course_order
        course = [string]$row.course
        title = [string]$row.title
        source_path = [string]$row.target_path
        duration_seconds = [double]$row.duration_seconds
        item_id = [string]$row.item_id
        revision_id = [string]$row.revision_id
        asset_id = [string]$row.asset_id
        asset_sha256 = [string]$row.asset_sha256
        status = 'pending'
        registration = $null
    }
    try {
        $codec = (& ffprobe -v error -select_streams a:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 -- $row.target_path).Trim()
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($codec)) {
            throw 'No readable audio stream.'
        }
        if ($codec -eq 'aac') {
            $audioFile = Join-Path $audioDir "module-$moduleKey.m4a"
            & ffmpeg -hide_banner -loglevel error -i $row.target_path -map 0:a:0 -vn -c:a copy -movflags +faststart -y $audioFile
            $preprocessing = @('Copied the complete original AAC audio stream without re-encoding.')
        }
        else {
            $audioFile = Join-Path $audioDir "module-$moduleKey.flac"
            & ffmpeg -hide_banner -loglevel error -i $row.target_path -map 0:a:0 -vn -ac 1 -ar 16000 -c:a flac -y $audioFile
            $preprocessing = @("Decoded the complete $codec audio stream to mono 16 kHz lossless FLAC.")
        }
        if ($LASTEXITCODE -ne 0) {
            throw "ffmpeg audio preparation failed with exit code $LASTEXITCODE."
        }
        $item['audio_codec'] = $codec
        $item['audio_file'] = $audioFile
        $item['audio_sha256'] = (Get-FileHash -Algorithm SHA256 -LiteralPath $audioFile).Hash.ToLowerInvariant()
        $item['audio_byte_size'] = (Get-Item -LiteralPath $audioFile).Length
        $item['preprocessing'] = $preprocessing
        $item['status'] = 'audio_ready'
    }
    catch {
        $item['status'] = 'failed'
        $item['error'] = $_.Exception.Message
    }
    $items.Add([pscustomobject]$item)
    [pscustomobject]@{ schema = 'babata.mba.asr-staging/v1'; items = $items } |
        ConvertTo-Json -Depth 10 |
        Set-Content -LiteralPath $manifestPath -Encoding utf8
    Write-Output "[$($index + 1)/$($rows.Count)] $($item.status) module:$moduleKey $($row.title)"
}

$summary = [ordered]@{
    total = $items.Count
    audio_ready = @($items | Where-Object status -eq 'audio_ready').Count
    failed = @($items | Where-Object status -eq 'failed').Count
    audio_bytes = ($items | Where-Object status -eq 'audio_ready' | Measure-Object audio_byte_size -Sum).Sum
    duration_hours = [math]::Round((($items | Measure-Object duration_seconds -Sum).Sum) / 3600, 3)
    manifest = $manifestPath
}
$summary | ConvertTo-Json
