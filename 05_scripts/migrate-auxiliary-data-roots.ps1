[CmdletBinding()]
param(
    [string]$DataRoot = $env:BABATA_DATA_HOME,
    [string]$EvidenceRoot = $env:BABATA_EVIDENCE_HOME,
    [string]$RecoveryRoot = $env:BABATA_RECOVERY_HOME,
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._-]*$')]
    [string]$OperationId = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ'),
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'

function Get-NormalizedPath {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    return [IO.Path]::GetFullPath($Path).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
}

function Test-PathInside {
    param(
        [Parameter(Mandatory)]
        [string]$Candidate,
        [Parameter(Mandatory)]
        [string]$Owner
    )

    $ownerPrefix = $Owner + [IO.Path]::DirectorySeparatorChar
    return $Candidate.Equals($Owner, [StringComparison]::OrdinalIgnoreCase) -or
        $Candidate.StartsWith($ownerPrefix, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NotVolumeRoot {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$Name
    )

    $volumeRoot = [IO.Path]::GetPathRoot($Path).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
    if ($Path.Equals($volumeRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Name must not be a filesystem volume root."
    }
}

function Get-Inventory {
    param(
        [Parameter(Mandatory)]
        [string]$Root
    )

    $inventory = @()
    foreach ($file in Get-ChildItem -LiteralPath $Root -Force -Recurse -File | Sort-Object FullName) {
        $relative = $file.FullName.Substring($Root.Length + 1).Replace('\', '/')
        $inventory += [pscustomobject]@{
            path = $relative
            byte_size = $file.Length
            sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
    return @($inventory)
}

function Assert-InventoriesEqual {
    param(
        [Parameter(Mandatory)]
        [object[]]$Expected,
        [Parameter(Mandatory)]
        [object[]]$Actual,
        [Parameter(Mandatory)]
        [string]$Name
    )

    if ($Expected.Count -ne $Actual.Count) {
        throw "$Name file count changed during copy: expected $($Expected.Count), found $($Actual.Count)"
    }
    for ($index = 0; $index -lt $Expected.Count; $index++) {
        $left = $Expected[$index]
        $right = $Actual[$index]
        if ($left.path -ne $right.path -or
            $left.byte_size -ne $right.byte_size -or
            $left.sha256 -ne $right.sha256) {
            throw "$Name hash inventory mismatch at '$($left.path)'"
        }
    }
}

if ([string]::IsNullOrWhiteSpace($DataRoot)) {
    throw 'DataRoot is required; pass -DataRoot or set BABATA_DATA_HOME.'
}
if ([string]::IsNullOrWhiteSpace($EvidenceRoot)) {
    throw 'EvidenceRoot is required; pass -EvidenceRoot or set BABATA_EVIDENCE_HOME.'
}
if ([string]::IsNullOrWhiteSpace($RecoveryRoot)) {
    throw 'RecoveryRoot is required; pass -RecoveryRoot or set BABATA_RECOVERY_HOME.'
}

$dataRootPath = Get-NormalizedPath $DataRoot
$evidenceRootPath = Get-NormalizedPath $EvidenceRoot
$recoveryRootPath = Get-NormalizedPath $RecoveryRoot
$repoRootPath = Get-NormalizedPath (Join-Path $PSScriptRoot '..')

Assert-NotVolumeRoot -Path $dataRootPath -Name 'DataRoot'
Assert-NotVolumeRoot -Path $evidenceRootPath -Name 'EvidenceRoot'
Assert-NotVolumeRoot -Path $recoveryRootPath -Name 'RecoveryRoot'

if (-not (Test-Path -LiteralPath $dataRootPath -PathType Container)) {
    throw "Data root does not exist: $dataRootPath"
}
foreach ($externalRoot in @(
    [pscustomobject]@{ Name = 'EvidenceRoot'; Path = $evidenceRootPath },
    [pscustomobject]@{ Name = 'RecoveryRoot'; Path = $recoveryRootPath }
)) {
    if (Test-PathInside -Candidate $externalRoot.Path -Owner $dataRootPath) {
        throw "$($externalRoot.Name) must be outside BABATA_DATA_HOME."
    }
    if (Test-PathInside -Candidate $externalRoot.Path -Owner $repoRootPath) {
        throw "$($externalRoot.Name) must be outside the Git repository."
    }
}
if ((Test-PathInside -Candidate $evidenceRootPath -Owner $recoveryRootPath) -or
    (Test-PathInside -Candidate $recoveryRootPath -Owner $evidenceRootPath)) {
    throw 'EvidenceRoot and RecoveryRoot must be separate local roots.'
}

$allowedEntries = @(
    '00_inbox',
    '01_raw',
    '02_derived',
    '03_views',
    '04_runtime',
    '05_logs',
    'README.local.md',
    'generated',
    'recovery-staging',
    'verification'
)
$unexpected = @(
    Get-ChildItem -LiteralPath $dataRootPath -Force |
        Where-Object { $_.Name -notin $allowedEntries } |
        Select-Object -ExpandProperty Name
)
if ($unexpected.Count -gt 0) {
    throw "Unexpected data-root entries require manual classification: $($unexpected -join ', ')"
}

$mappings = @(
    [pscustomobject]@{
        name = 'verification'
        class = 'development_evidence'
        source = Join-Path $dataRootPath 'verification'
        destination = Join-Path $evidenceRootPath 'runs'
    },
    [pscustomobject]@{
        name = 'recovery-staging'
        class = 'uncommitted_recovery_material'
        source = Join-Path $dataRootPath 'recovery-staging'
        destination = Join-Path $recoveryRootPath 'batches'
    },
    [pscustomobject]@{
        name = 'generated'
        class = 'disposable_model_workspace'
        source = Join-Path $dataRootPath 'generated'
        destination = Join-Path $dataRootPath '04_runtime\staging\model-workspaces'
    }
)
$activeMappings = @($mappings | Where-Object { Test-Path -LiteralPath $_.source -PathType Container })

if ($activeMappings.Count -eq 0) {
    $remaining = @(
        Get-ChildItem -LiteralPath $dataRootPath -Force |
            Where-Object { $_.Name -notin $allowedEntries[0..6] } |
            Select-Object -ExpandProperty Name
    )
    if ($remaining.Count -gt 0) {
        throw "Data root contains unsupported entries: $($remaining -join ', ')"
    }
    Write-Output 'No auxiliary top-level directories found; data-root layout is already clean.'
    exit 0
}

$plans = @()
foreach ($mapping in $activeMappings) {
    $sourceItem = Get-Item -LiteralPath $mapping.source -Force
    if (($sourceItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Refusing to migrate reparse-point source: $($mapping.source)"
    }
    if (Test-Path -LiteralPath $mapping.destination) {
        throw "Destination already exists and must be classified before migration: $($mapping.destination)"
    }
    $inventory = @(Get-Inventory -Root $mapping.source)
    $plans += [pscustomobject]@{
        mapping = $mapping
        inventory = $inventory
        file_count = $inventory.Count
        byte_size = ($inventory | Measure-Object -Property byte_size -Sum).Sum
    }
}

if (-not $Apply) {
    foreach ($plan in $plans) {
        [pscustomobject]@{
            mode = 'audit_only'
            source_name = $plan.mapping.name
            destination_class = $plan.mapping.class
            file_count = $plan.file_count
            byte_size = $plan.byte_size
            destination = $plan.mapping.destination
        }
    }
    Write-Output 'Audit only; pass -Apply after reviewing the planned destinations.'
    exit 0
}

$finalized = @()
foreach ($plan in $plans) {
    $destinationParent = Split-Path -Parent $plan.mapping.destination
    $destinationLeaf = Split-Path -Leaf $plan.mapping.destination
    $staging = Join-Path $destinationParent (".$destinationLeaf.incoming-$OperationId")
    if (Test-Path -LiteralPath $staging) {
        throw "Migration staging path already exists: $staging"
    }

    New-Item -ItemType Directory -Path $staging -Force | Out-Null
    foreach ($entry in Get-ChildItem -LiteralPath $plan.mapping.source -Force) {
        Copy-Item -LiteralPath $entry.FullName -Destination $staging -Recurse -Force
    }
    $copiedInventory = @(Get-Inventory -Root $staging)
    Assert-InventoriesEqual -Expected $plan.inventory -Actual $copiedInventory -Name $plan.mapping.name
    Move-Item -LiteralPath $staging -Destination $plan.mapping.destination
    $finalized += $plan
}

$manifestDirectory = Join-Path $evidenceRootPath "governance\migrations\$OperationId"
New-Item -ItemType Directory -Path $manifestDirectory -Force | Out-Null
$evidenceReadme = Join-Path $evidenceRootPath 'README.local.md'
if (-not (Test-Path -LiteralPath $evidenceReadme)) {
    @'
# Babata Evidence Root

This Git-external root contains sensitive development and acceptance evidence,
isolated data roots, migration inventories, and necessary historical snapshots.
It is not an active Babata data root and not a substitute for a P8 backup.
'@ | Set-Content -LiteralPath $evidenceReadme -Encoding utf8
}
$recoveryReadme = Join-Path $recoveryRootPath 'README.local.md'
if (-not (Test-Path -LiteralPath $recoveryReadme)) {
    @'
# Babata Recovery Root

This Git-external root contains source material recovered locally but not yet
accepted by the Babata Capture/C0 path. Presence here does not mean that an item
has been formally collected.
'@ | Set-Content -LiteralPath $recoveryReadme -Encoding utf8
}
$manifestPath = Join-Path $manifestDirectory 'manifest.json'
$manifest = [ordered]@{
    schema = 'babata.auxiliary-root-migration/v1'
    operation_id = $OperationId
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    status = 'verified_copy'
    roots = [ordered]@{
        data = $dataRootPath
        evidence = $evidenceRootPath
        recovery = $recoveryRootPath
    }
    entries = @(
        $finalized | ForEach-Object {
            [ordered]@{
                source_name = $_.mapping.name
                destination_class = $_.mapping.class
                destination = $_.mapping.destination
                file_count = $_.file_count
                byte_size = $_.byte_size
                files = @($_.inventory)
            }
        }
    )
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding utf8

foreach ($plan in $finalized) {
    $expectedSource = Get-NormalizedPath (Join-Path $dataRootPath $plan.mapping.name)
    $actualSource = Get-NormalizedPath $plan.mapping.source
    if (-not $actualSource.Equals($expectedSource, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove unexpected source path: $actualSource"
    }
    Remove-Item -LiteralPath $actualSource -Recurse -Force
}

$manifest.status = 'completed'
$manifest.completed_at = (Get-Date).ToUniversalTime().ToString('o')
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding utf8

$remaining = @(
    Get-ChildItem -LiteralPath $dataRootPath -Force |
        Where-Object { $_.Name -notin $allowedEntries[0..6] } |
        Select-Object -ExpandProperty Name
)
if ($remaining.Count -gt 0) {
    throw "Migration copied data but the active root is not clean: $($remaining -join ', ')"
}

Write-Output "Auxiliary-root migration completed. Manifest: $manifestPath"
