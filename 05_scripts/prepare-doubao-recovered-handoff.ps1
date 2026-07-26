param(
    [Parameter(Mandatory = $true)]
    [string]$RecoveredConversation,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$source = Get-Item -LiteralPath $RecoveredConversation
$recovered = Get-Content -Raw -Encoding UTF8 -LiteralPath $source.FullName | ConvertFrom-Json
$conversationId = [string]$recovered.conversation_id
if ($conversationId -notmatch '^\d{8,}$') {
    throw 'Recovered Doubao conversation has an invalid conversation ID.'
}
$conversationInfo = $recovered.conversation_info.conversation_info
if ([string]$conversationInfo.conversation_id -ne $conversationId) {
    throw 'Recovered Doubao conversation info does not match the conversation ID.'
}
$messages = @($recovered.messages)
if ($messages.Count -eq 0) {
    throw 'Recovered Doubao conversation has no messages.'
}
$declaredFiles = @($messages | Where-Object { [int64]$_.content_type -eq 20 })
if ($declaredFiles.Count -ne 0) {
    throw 'This preparation path is only for recovered conversations with no declared file attachments.'
}

$handoff = [ordered]@{
    protocol_version = '1'
    provider = 'doubao'
    acquisition_route = 'chrome_native'
    conversation_id = $conversationId
    source_url = "https://www.doubao.com/chat/$conversationId"
    conversation_info = $conversationInfo
    messages = $messages
    has_more = $false
    message_cursor = if ([string]::IsNullOrWhiteSpace([string]$conversationInfo.msg_cursor)) { 'complete' } else { [string]$conversationInfo.msg_cursor }
    assets = @()
}

[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$outputPath = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) 'acquisition-handoff.json'
[IO.File]::WriteAllText(
    $outputPath,
    ($handoff | ConvertTo-Json -Depth 100),
    [Text.UTF8Encoding]::new($false)
)
[pscustomobject]@{
    conversation_id = $conversationId
    title = [string]$recovered.title
    message_count = $messages.Count
    attachment_count = 0
    handoff_path = $outputPath
} | ConvertTo-Json
