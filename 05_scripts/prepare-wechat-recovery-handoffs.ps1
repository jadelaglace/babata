param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Text)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $algorithm.Dispose()
    }
}

function Resolve-RecoveryAsset {
    param(
        [Parameter(Mandatory = $true)][string]$BatchRoot,
        [Parameter(Mandatory = $true)][string]$RelativePath,
        [Parameter(Mandatory = $true)][long]$ExpectedBytes,
        [Parameter(Mandatory = $true)][string]$ExpectedSha256,
        [Parameter(Mandatory = $true)][string]$Role
    )

    $path = [IO.Path]::GetFullPath((Join-Path $BatchRoot $RelativePath))
    if (-not $path.StartsWith($BatchRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Recovery asset escaped the batch root: $RelativePath"
    }
    $file = Get-Item -LiteralPath $path
    if ($file.PSIsContainer -or $file.Length -ne $ExpectedBytes) {
        throw "Recovery asset size changed: $RelativePath"
    }
    return [ordered]@{
        path = $path
        role = $Role
        sha256 = $ExpectedSha256
    }
}

$manifestFile = Get-Item -LiteralPath $ManifestPath
$batchRoot = $manifestFile.Directory.FullName
$batchId = $manifestFile.Directory.Name
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestFile.FullName | ConvertFrom-Json
$complete = @($manifest.items | Where-Object { $_.c0_a2_status -eq 'complete' })
if ($complete.Count -lt 10 -or $complete.Count -gt 20) {
    throw "Expected 10-20 complete C0-A2 samples, found $($complete.Count)"
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$outputRoot = (Get-Item -LiteralPath $OutputDirectory).FullName
$handoffs = [Collections.Generic.List[object]]::new()

foreach ($item in $complete) {
    $isFavorite = $item.source_kind -eq 'favorites_url'
    $routeId = if ($isFavorite) { 'source.wechat_favorites' } else { 'source.wechat_chats' }
    $assets = [Collections.Generic.List[object]]::new()

    if ($item.body.status -eq 'saved') {
        $assets.Add((Resolve-RecoveryAsset -BatchRoot $batchRoot -RelativePath $item.body.page_path -ExpectedBytes $item.body.page_bytes -ExpectedSha256 $item.body.page_sha256 -Role 'original'))
        $bodyHtml = Get-Item -LiteralPath (Join-Path $batchRoot $item.body.html_path)
        $assets.Add((Resolve-RecoveryAsset -BatchRoot $batchRoot -RelativePath $item.body.html_path -ExpectedBytes $bodyHtml.Length -ExpectedSha256 $item.body.html_sha256 -Role 'export'))
        $bodyText = Get-Item -LiteralPath (Join-Path $batchRoot $item.body.text_path)
        $assets.Add((Resolve-RecoveryAsset -BatchRoot $batchRoot -RelativePath $item.body.text_path -ExpectedBytes $bodyText.Length -ExpectedSha256 $item.body.text_sha256 -Role 'export'))
        $bodyTextValue = Get-Content -Raw -Encoding UTF8 -LiteralPath $bodyText.FullName
        $contentType = 'document'
        $sourceReference = $item.url
    }
    else {
        $contentType = if ($item.render_type -eq 'video') { 'video' } else { 'image' }
        $sourceReference = "recovery:$batchId/$($item.stable_id)"
        $bodyTextValue = $null
    }

    foreach ($media in @($item.embedded_media.items | Where-Object { $_.status -eq 'saved' })) {
        $role = if ($media.kind -eq 'video_thumb') { 'preview' } elseif ($item.body.status -eq 'saved') { 'attachment' } else { 'original' }
        $assets.Add((Resolve-RecoveryAsset -BatchRoot $batchRoot -RelativePath $media.path -ExpectedBytes $media.bytes -ExpectedSha256 $media.sha256 -Role $role))
    }
    foreach ($attachment in @($item.required_attachments.items | Where-Object { $_.status -eq 'saved' })) {
        $assets.Add((Resolve-RecoveryAsset -BatchRoot $batchRoot -RelativePath $attachment.path -ExpectedBytes $attachment.bytes -ExpectedSha256 $attachment.sha256 -Role 'attachment'))
    }
    if ($assets.Count -eq 0) {
        throw "Prepared item has no assets: $($item.stable_id)"
    }

    $nativeId = if ($isFavorite) { "favorite:$($item.local_id)" } else { [string]$item.stable_id }
    $payloadObject = [ordered]@{
        batch_id = $batchId
        body_text = $bodyTextValue
        kind = $contentType
        local_id = $item.local_id
        platform = 'wechat'
        server_id = $item.server_id
        source_id = if ($null -eq $item.source_id) { $null } else { [string]$item.source_id }
        source_kind = [string]$item.source_kind
        source_url = if ($null -eq $item.url) { $null } else { [string]$item.url }
        stable_id = [string]$item.stable_id
        title = if ([string]::IsNullOrWhiteSpace([string]$item.title)) { $null } else { [string]$item.title }
    }
    $payload = $payloadObject | ConvertTo-Json -Compress
    $payloadSha256 = Get-Sha256Text -Text $payload
    $assetHashes = @($assets | ForEach-Object { $_.sha256 } | Sort-Object)
    $fingerprintInput = [ordered]@{
        asset_sha256 = $assetHashes
        native_id = $nativeId
        payload_sha256 = $payloadSha256
    } | ConvertTo-Json -Compress
    $fingerprint = Get-Sha256Text -Text $fingerprintInput
    $title = if ([string]::IsNullOrWhiteSpace($item.title)) { "File Transfer Assistant $($item.render_type)" } else { $item.title }
    $candidate = [ordered]@{
        protocolVersion = '1'
        routeId = $routeId
        sourceReference = $sourceReference
        contentType = $contentType
        payloadSha256 = $payloadSha256
        metadata = [ordered]@{
            title = $title
            source_kind = $item.source_kind
            stable_id = $item.stable_id
            c0_a2_manifest = $manifestFile.FullName
            sovereignty_depth = 'C0-A2'
            management_readiness = 'prepared'
            preparation_schema = 'babata/wechat-c0-b/1'
            content_fingerprint = $fingerprint
            attempt_limit = 2
            _babata_acquisition_assets = @($assets)
        }
        payload = [ordered]@{ kind = 'text'; text = $payload }
        context = if ($isFavorite) { 'WeChat / Favorites / P7 sample' } else { 'WeChat / File Transfer Assistant / P7 sample' }
        nativeId = $nativeId
    }
    $handoff = [ordered]@{
        protocol_version = 'babata/wechat-recovery-handoff/1'
        provider = 'wechat'
        route_id = $routeId
        candidate = $candidate
    }
    $safeId = $item.stable_id -replace '[^A-Za-z0-9._-]', '_'
    $path = Join-Path $outputRoot ('{0:D2}-{1}.json' -f $item.sample_index, $safeId)
    [IO.File]::WriteAllText(
        $path,
        ($handoff | ConvertTo-Json -Depth 20),
        [Text.UTF8Encoding]::new($false)
    )
    $handoffs.Add([ordered]@{
        sample_index = $item.sample_index
        stable_id = $item.stable_id
        route_id = $routeId
        handoff_path = $path
        asset_count = $assets.Count
        content_fingerprint = $fingerprint
    })
}

$summary = [ordered]@{
    schema_version = 1
    stage = 'C0-B'
    source = $manifestFile.FullName
    source_boundary = 'decrypted DB/ZIP Recovery only; WeChat UI prohibited unless the user explicitly requests it'
    complete_items = $handoffs.Count
    favorites = @($handoffs | Where-Object route_id -eq 'source.wechat_favorites').Count
    chats = @($handoffs | Where-Object route_id -eq 'source.wechat_chats').Count
    handoffs = @($handoffs)
}
$summaryPath = Join-Path $outputRoot 'manifest.json'
[IO.File]::WriteAllText(
    $summaryPath,
    ($summary | ConvertTo-Json -Depth 10),
    [Text.UTF8Encoding]::new($false)
)
$summary | ConvertTo-Json -Depth 10
