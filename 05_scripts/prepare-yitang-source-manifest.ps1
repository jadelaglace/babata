param(
    [Parameter(Mandatory)][string]$AcquisitionRoot,
    [Parameter(Mandatory)][string]$ReplayMetadataPath,
    [Parameter(Mandatory)][string]$DocumentBlocksPath,
    [Parameter(Mandatory)][string]$OutputPath
)

$ErrorActionPreference = 'Stop'

function Read-JsonFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Missing input: $Path" }
    return Get-Content -Raw -Encoding utf8 -LiteralPath $Path | ConvertFrom-Json
}

function Assert-ExactHttpsUrl {
    param([string]$Actual, [string]$Expected, [string]$Label)
    $uri = [Uri]$Actual
    if ($uri.Scheme -ne 'https' -or $Actual -cne $Expected) {
        throw "$Label does not match the authorised HTTPS identity"
    }
}

function Assert-SafeProperties {
    param($Value)
    $forbidden = @('user', 'cookie', 'credential', 'session', 'chats', 'zans', 'promotes', 'controls', 'progressDot')
    if ($null -eq $Value) { return }
    if ($Value -is [System.Collections.IDictionary] -or $Value -is [pscustomobject]) {
        foreach ($property in $Value.PSObject.Properties) {
            if ($forbidden -contains $property.Name) { throw "Forbidden metadata field: $($property.Name)" }
            Assert-SafeProperties $property.Value
        }
    }
    elseif ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        foreach ($item in $Value) { Assert-SafeProperties $item }
    }
}

function Get-StringSha256 {
    param([string]$Text)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    return [Convert]::ToHexString($hash).ToLowerInvariant()
}

$root = (Resolve-Path -LiteralPath $AcquisitionRoot).Path.TrimEnd('\')
$rootPrefix = $root + [IO.Path]::DirectorySeparatorChar
$replay = Read-JsonFile $ReplayMetadataPath
$document = Read-JsonFile $DocumentBlocksPath
Assert-SafeProperties $replay

if ($replay.schema -ne 'babata.yitang-safe-replay-metadata/v1') { throw 'Unsupported replay metadata schema' }
if ($document.schema -ne 'babata.yitang-rendered-document/v1') { throw 'Unsupported rendered document schema' }
if ($replay.replay_sid -notmatch '^[A-Za-z0-9_-]{6,64}$') { throw 'Invalid replay sid' }
if ($document.acl_token -notmatch '^[a-f0-9]{32}$') { throw 'Invalid document ACL token' }
if ($document.document_token -notmatch '^[A-Za-z0-9_-]{8,128}$') { throw 'Invalid document token' }

$expectedReplay = "https://air.yitang.top/live/$($replay.replay_sid)"
$expectedDocument = "https://yitang.top/fs-doc/$($document.acl_token)/$($document.document_token)"
Assert-ExactHttpsUrl $replay.replay_url $expectedReplay 'Replay URL'
Assert-ExactHttpsUrl $document.source_url $expectedDocument 'Document URL'

foreach ($locator in @($replay.media.hls_url, $replay.media.audio_url)) {
    $uri = [Uri]$locator
    if ($uri.Scheme -ne 'https' -or $uri.Host -cne 'cdn.yitang.top') { throw 'Unexpected replay media host' }
}
if ([double]$replay.media.duration_seconds -le 0) { throw 'Replay source duration is missing' }
if (-not $document.traversal.reached_bottom -or [int]$document.traversal.sample_count -lt 2) {
    throw 'Rendered document traversal did not reach the bottom'
}
if ([int]$document.block_count -ne @($document.blocks).Count -or
    [int]$document.nonempty_block_count -le 0 -or
    [int]$document.heading_count -le 0 -or
    [int]$document.image_count -ne @($document.images).Count) {
    throw 'Rendered document summary is incomplete or inconsistent'
}
$nativeIds = @($document.blocks | ForEach-Object { $_.native_id })
if (@($nativeIds | Sort-Object -Unique).Count -ne $nativeIds.Count) { throw 'Duplicate document block identity' }

$ffprobe = Get-Command ffprobe -ErrorAction Stop
$mediaRelative = "media/$($replay.replay_sid).mp4"
$mediaPath = Join-Path $root $mediaRelative
$probe = & $ffprobe.Source -v error -show_entries 'format=duration,format_name,size,bit_rate:stream=index,codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels' -of json $mediaPath | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $null -eq $probe.format) { throw 'ffprobe failed for replay media' }
$localDuration = [double]$probe.format.duration
if ([Math]::Abs($localDuration - [double]$replay.media.duration_seconds) -gt 2.0) {
    throw 'Replay duration does not match source metadata'
}
if (-not @($probe.streams | Where-Object codec_type -eq 'video') -or
    -not @($probe.streams | Where-Object codec_type -eq 'audio')) {
    throw 'Replay media must contain video and audio streams'
}

function New-AssetRecord {
    param([string]$RelativePath, [string]$Role, [string]$MediaType, [string]$SourceUrl)
    $relative = $RelativePath.Replace('\', '/')
    if ([IO.Path]::IsPathRooted($relative) -or $relative.Split('/') -contains '..') {
        throw "Asset path escapes acquisition root: $RelativePath"
    }
    $candidate = Join-Path $root $relative
    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    if (-not $resolved.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Asset path escapes acquisition root: $RelativePath"
    }
    $file = Get-Item -LiteralPath $resolved
    if ($file.Length -le 0) { throw "Asset is empty: $RelativePath" }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolved).Hash.ToLowerInvariant()
    return [ordered]@{
        relative_path = $relative
        local_path = $resolved
        role = $Role
        media_type = $MediaType
        source_url = $SourceUrl
        byte_size = [uint64]$file.Length
        sha256 = $hash
    }
}

$assets = [Collections.Generic.List[object]]::new()
$assets.Add((New-AssetRecord $mediaRelative 'original' 'video/mp4' $replay.replay_url))
foreach ($export in @(
    @{ path = 'document/document.md'; type = 'text/markdown' },
    @{ path = 'document/blocks.json'; type = 'application/json' },
    @{ path = 'document/rendered.txt'; type = 'text/plain' },
    @{ path = 'document/source-rendered.html'; type = 'text/html' }
)) {
    $assets.Add((New-AssetRecord $export.path 'export' $export.type $document.source_url))
}

$seenImages = @{}
foreach ($image in @($document.images | Sort-Object order)) {
    if ($image.local_filename -notmatch '^\d{3}\.(png|jpg|jpeg|webp|gif)$') {
        throw 'Document image filename is not controlled'
    }
    if ($seenImages.ContainsKey($image.source_url)) { throw 'Duplicate document image source URL' }
    $seenImages[$image.source_url] = $true
    $imageUri = [Uri]$image.source_url
    if ($imageUri.Scheme -ne 'https' -or $imageUri.Host -cne 'cdn.yitang.top') {
        throw 'Unexpected document image host'
    }
    $assets.Add((New-AssetRecord "document/images/$($image.local_filename)" 'attachment' "image/$($image.local_filename.Split('.')[-1].Replace('jpg','jpeg'))" $image.source_url))
}
$actualImages = @(Get-ChildItem -LiteralPath (Join-Path $root 'document/images') -File)
if ($actualImages.Count -ne [int]$document.image_count) { throw 'Document image directory denominator is inconsistent' }

$relativePaths = @($assets | ForEach-Object relative_path)
if (@($relativePaths | Sort-Object -Unique).Count -ne $relativePaths.Count) { throw 'Duplicate manifest asset path' }

$fingerprintLines = @(
    "replay_sid=$($replay.replay_sid)",
    "replay_url=$($replay.replay_url)",
    "document_acl_token=$($document.acl_token)",
    "document_token=$($document.document_token)",
    "document_url=$($document.source_url)"
)
$fingerprintLines += @($assets | ForEach-Object {
    "asset=$($_.relative_path)|$($_.role)|$($_.byte_size)|$($_.sha256)"
})
$contentFingerprint = Get-StringSha256 ($fingerprintLines -join "`n")

$manifest = [ordered]@{
    schema = 'babata.yitang-source-manifest/v1'
    readiness = 'prepared'
    generated_at = (Get-Date).ToString('o')
    acquisition_root = $root
    summary = [ordered]@{ source_items = 1; assets = $assets.Count; images = [int]$document.image_count; unresolved = 0 }
    items = @([ordered]@{
        stable_identity = "replay:$($replay.replay_sid)|document:$($document.document_token)"
        replay_sid = $replay.replay_sid
        replay_url = $replay.replay_url
        document_acl_token = $document.acl_token
        document_token = $document.document_token
        document_url = $document.source_url
        title = $document.title
        subtitle = $replay.subtitle
        begin_time = $replay.begin_time
        source_status = $replay.status
        hls_url = $replay.media.hls_url
        audio_url = $replay.media.audio_url
        duration_seconds_source = [double]$replay.media.duration_seconds
        duration_seconds_local = $localDuration
        document = [ordered]@{
            traversal_complete = $true
            block_count = [int]$document.block_count
            nonempty_block_count = [int]$document.nonempty_block_count
            heading_count = [int]$document.heading_count
            image_count = [int]$document.image_count
            markdown_relative_path = 'document/document.md'
            blocks_relative_path = 'document/blocks.json'
            text_relative_path = 'document/rendered.txt'
            html_relative_path = 'document/source-rendered.html'
        }
        media_probe = $probe
        content_fingerprint = $contentFingerprint
        assets = @($assets)
        readiness = 'prepared'
    })
    preparation = [ordered]@{ tool = 'prepare-yitang-source-manifest.ps1'; version = '1' }
}

$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
$resolvedOutput = [IO.Path]::GetFullPath($OutputPath)
[IO.File]::WriteAllText($resolvedOutput, ($manifest | ConvertTo-Json -Depth 12), [Text.UTF8Encoding]::new($false))
[ordered]@{
    manifest_path = $resolvedOutput
    stable_identity = $manifest.items[0].stable_identity
    assets = $manifest.summary.assets
    images = $manifest.summary.images
    content_fingerprint = $manifest.items[0].content_fingerprint
} | ConvertTo-Json
