[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d{8,}$')]
    [string]$ConversationId,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$BrowserSession = 'babata-doubao-originals',

    [switch]$KeepBrowserSession
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-OpenCliJson {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $output = & opencli @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "OpenCLI failed: opencli $($Arguments -join ' ')"
    }
    return ($output | Out-String | ConvertFrom-Json)
}

function Get-PropertyValue {
    param(
        [Parameter(Mandatory = $true)][object]$InputObject,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $property = $InputObject.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Get-DeclaredFiles {
    param([Parameter(Mandatory = $true)][object[]]$Messages)

    $files = foreach ($message in $Messages) {
        if ([int64]$message.content_type -ne 20) {
            continue
        }
        $content = $message.content | ConvertFrom-Json
        foreach ($entity in @($content.entities)) {
            $entityContent = Get-PropertyValue -InputObject $entity -Name 'entity_content'
            if ($null -eq $entityContent) {
                continue
            }
            $file = Get-PropertyValue -InputObject $entityContent -Name 'file'
            if ($null -eq $file) {
                continue
            }
            if ([string]::IsNullOrWhiteSpace([string]$file.key)) {
                throw "Doubao attachment '$($file.file_name)' has no original object key."
            }
            [pscustomobject]@{
                file_name = [string]$file.file_name
                key = [string]$file.key
                md5 = ([string]$file.md5).ToLowerInvariant()
                size = [int64]$file.size
            }
        }
    }
    return @($files)
}

function Test-DocxStructure {
    param([Parameter(Mandatory = $true)][string]$Path)

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $names = @($archive.Entries.FullName)
        return ($names -contains '[Content_Types].xml') -and
            ($names -contains 'word/document.xml')
    }
    finally {
        $archive.Dispose()
    }
}

$detail = Invoke-OpenCliJson -Arguments @(
    'doubao', 'detail-full', $ConversationId,
    '-f', 'json',
    '--window', 'background',
    '--site-session', 'persistent',
    '--keep-tab', 'true'
)
$row = @($detail)[0]
if ($null -eq $row -or [bool]$row.HasMore) {
    throw 'Doubao message pagination is incomplete.'
}
if ([string]::IsNullOrWhiteSpace([string]$row.MessageCursor)) {
    throw 'Doubao response has no message cursor.'
}

$files = Get-DeclaredFiles -Messages @($row.Messages)
if ($files.Count -eq 0) {
    throw 'The conversation declares no downloadable file attachments.'
}
$duplicateNames = $files | Group-Object file_name | Where-Object Count -gt 1
if ($duplicateNames) {
    throw "Doubao response contains duplicate attachment names: $($duplicateNames.Name -join ', ')"
}

$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($outputRoot) | Out-Null
$originalsRoot = Join-Path $outputRoot 'originals'
[System.IO.Directory]::CreateDirectory($originalsRoot) | Out-Null

Invoke-OpenCliJson -Arguments @(
    'browser', $BrowserSession, 'open',
    "https://www.doubao.com/chat/$ConversationId",
    '--window', 'background'
) | Out-Null
& opencli browser $BrowserSession wait time 2 | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw 'OpenCLI could not initialise the signed-in Doubao page.'
}

$requestBody = @{ uris = @($files.key); type = 'file' } | ConvertTo-Json -Compress
$requestBodyBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($requestBody))
$pageScript = "(async()=>{const b=atob('$requestBodyBase64');const r=await fetch('/alice/message/get_file_url',{method:'POST',headers:{'content-type':'application/json'},body:b});if(!r.ok)throw new Error('get_file_url HTTP '+r.status);return await r.json()})()"
$signed = Invoke-OpenCliJson -Arguments @('browser', $BrowserSession, 'eval', $pageScript)
$signedFiles = @($signed.data.file_urls)
if ([int]$signed.code -ne 0 -or $signedFiles.Count -ne $files.Count) {
    throw "get_file_url returned $($signedFiles.Count) URLs for $($files.Count) declared files."
}

$assets = foreach ($file in $files) {
    $signedFile = $signedFiles | Where-Object uri -EQ $file.key | Select-Object -First 1
    if ($null -eq $signedFile) {
        throw "get_file_url did not return '$($file.key)'."
    }
    $mainUrl = Get-PropertyValue -InputObject $signedFile -Name 'main_url'
    $backUrl = Get-PropertyValue -InputObject $signedFile -Name 'back_url'
    $url = if (-not [string]::IsNullOrWhiteSpace([string]$mainUrl)) { $mainUrl } else { $backUrl }
    if ([string]::IsNullOrWhiteSpace([string]$url) -or
        -not ([string]$signedFile.uri).EndsWith('.docx', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Doubao did not return an original DOCX URL for '$($file.file_name)'."
    }
    $destination = Join-Path $originalsRoot $file.file_name
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $destination -TimeoutSec 180
    $actual = Get-Item -LiteralPath $destination
    $md5 = (Get-FileHash -LiteralPath $destination -Algorithm MD5).Hash.ToLowerInvariant()
    $sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual.Length -ne $file.size -or $md5 -ne $file.md5) {
        throw "Downloaded bytes do not match Doubao metadata for '$($file.file_name)'."
    }
    if (-not (Test-DocxStructure -Path $destination)) {
        throw "Downloaded file is not a valid DOCX package: '$($file.file_name)'."
    }
    [ordered]@{
        path = $actual.FullName
        file_name = $file.file_name
        byte_size = $actual.Length
        md5 = $md5
        sha256 = $sha256
    }
}

$conversationInfo = $row.Info.conversation_info
$handoff = [ordered]@{
    protocol_version = '1'
    provider = 'doubao'
    acquisition_route = 'chrome_native'
    conversation_id = $ConversationId
    source_url = "https://www.doubao.com/chat/$ConversationId"
    conversation_info = $conversationInfo
    messages = @($row.Messages)
    has_more = $false
    message_cursor = [string]$row.MessageCursor
    assets = @($assets)
}
$handoffPath = Join-Path $outputRoot 'acquisition-handoff.json'
$handoffJson = $handoff | ConvertTo-Json -Depth 100
[System.IO.File]::WriteAllText(
    $handoffPath,
    $handoffJson,
    [System.Text.UTF8Encoding]::new($false)
)

if (-not $KeepBrowserSession) {
    & opencli browser $BrowserSession close | Out-Null
}

$totalBytes = ($assets | ForEach-Object { [int64]$_['byte_size'] } | Measure-Object -Sum).Sum
[pscustomobject]@{
    conversation_id = $ConversationId
    message_count = @($row.Messages).Count
    attachment_count = $assets.Count
    total_bytes = $totalBytes
    handoff_path = $handoffPath
    originals_directory = $originalsRoot
} | ConvertTo-Json -Depth 5
