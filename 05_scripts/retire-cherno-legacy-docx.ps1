[CmdletBinding()]
param(
    [string]$C1Manifest = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-full-20260822-v1\manifest.json',
    [string]$LegacyRoot = 'E:\Cherno\OpenGL',
    [string]$ReceiptRoot = 'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-legacy-docx-retirement-20260822-v1',
    [int]$ExpectedC1Items = 269,
    [int]$ExpectedDocx = 31
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = [Text.UTF8Encoding]::new($false)
if (-not (Test-Path -LiteralPath $C1Manifest -PathType Leaf)) { throw "Missing C1 manifest: $C1Manifest" }
if (-not (Test-Path -LiteralPath $LegacyRoot -PathType Container)) { throw "Missing legacy root: $LegacyRoot" }

$c1 = Get-Content -LiteralPath $C1Manifest -Raw -Encoding utf8 | ConvertFrom-Json
$items = @($c1.items)
if ($items.Count -ne $ExpectedC1Items -or @($items | Where-Object status -ne 'registered').Count -ne 0) {
    throw "Legacy DOCX retirement requires $ExpectedC1Items registered C1 items."
}
foreach ($item in $items) {
    $registrations = @($item.registrations)
    if ($registrations.Count -ne 2 -or @($registrations.kind | Sort-Object) -join ',' -ne 'structured_result,transcript') {
        throw "Complete replacement C1 is missing for $($item.video_id)."
    }
    if (@($registrations | Where-Object { [string]$_.state -ne 'succeeded' }).Count -ne 0) {
        throw "Replacement C1 read-back is not succeeded for $($item.video_id)."
    }
}

$root = [IO.Path]::GetFullPath($LegacyRoot).TrimEnd('\')
$files = @(Get-ChildItem -LiteralPath $root -File -Filter '*.docx' | Sort-Object FullName)
if ($files.Count -ne $ExpectedDocx) { throw "Expected $ExpectedDocx legacy DOCX files, found $($files.Count)." }

$rows = [Collections.Generic.List[object]]::new()
foreach ($file in $files) {
    $full = [IO.Path]::GetFullPath($file.FullName)
    if (-not $full.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetExtension($full) -ine '.docx') {
        throw "Unsafe legacy DOCX target: $full"
    }
    $rows.Add([ordered]@{
        path = $full
        bytes = [long]$file.Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash.ToLowerInvariant()
        outcome = 'planned'
    })
}

New-Item -ItemType Directory -Force -Path $ReceiptRoot | Out-Null
$planPath = Join-Path $ReceiptRoot 'deletion-plan.json'
$plan = [ordered]@{
    schema = 'babata.cherno-legacy-docx-retirement/v1'
    frozen_at = [DateTimeOffset]::UtcNow.ToString('o')
    status = 'frozen_before_removal'
    c1_manifest = $C1Manifest
    c1_manifest_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $C1Manifest).Hash.ToLowerInvariant()
    replacement_c1_items = $items.Count
    replacement_c1_registrations = @($items.registrations).Count
    legacy_root = $root
    files = @($rows)
}
[IO.File]::WriteAllText($planPath, ($plan | ConvertTo-Json -Depth 12), $utf8NoBom)
$roundTrip = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json
if ([string]$roundTrip.status -ne 'frozen_before_removal' -or @($roundTrip.files).Count -ne $ExpectedDocx) {
    throw 'Legacy DOCX deletion plan read-back failed.'
}

Add-Type -AssemblyName Microsoft.VisualBasic
foreach ($row in $rows) {
    [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile(
        [string]$row.path,
        [Microsoft.VisualBasic.FileIO.UIOption]::OnlyErrorDialogs,
        [Microsoft.VisualBasic.FileIO.RecycleOption]::SendToRecycleBin
    )
    if (Test-Path -LiteralPath ([string]$row.path)) { throw "Legacy DOCX still exists after retirement: $($row.path)" }
    $row.outcome = 'moved_to_recycle_bin'
}

$receiptPath = Join-Path $ReceiptRoot 'retirement-receipt.json'
$receipt = [ordered]@{
    schema = 'babata.cherno-legacy-docx-retirement/v1'
    completed_at = [DateTimeOffset]::UtcNow.ToString('o')
    status = 'removed_from_source_tree'
    recovery = 'Windows Recycle Bin while retained by the operating system'
    deletion_plan = $planPath
    deletion_plan_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $planPath).Hash.ToLowerInvariant()
    legacy_root = $root
    files = @($rows)
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 12), $utf8NoBom)
$receiptReadBack = Get-Content -LiteralPath $receiptPath -Raw -Encoding utf8 | ConvertFrom-Json
if ([string]$receiptReadBack.status -ne 'removed_from_source_tree' -or
    @($receiptReadBack.files | Where-Object outcome -ne 'moved_to_recycle_bin').Count -ne 0) {
    throw 'Legacy DOCX retirement receipt read-back failed.'
}

[ordered]@{
    removed_from_source_tree = @($receiptReadBack.files).Count
    recovery = [string]$receiptReadBack.recovery
    deletion_plan = $planPath
    receipt = $receiptPath
} | ConvertTo-Json
