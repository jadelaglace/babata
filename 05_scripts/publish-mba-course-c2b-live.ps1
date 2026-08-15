[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$PackageRoot,
    [string]$ManifestPath,
    [string]$DataHome=$env:BABATA_DATA_HOME,
    [string]$CheckerScript=(Join-Path $PSScriptRoot 'check-mba-course-c2b-package.ps1')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Hash([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Same-Path([string]$Left,[string]$Right) {
    [IO.Path]::GetFullPath($Left).TrimEnd('\').Equals(
        [IO.Path]::GetFullPath($Right).TrimEnd('\'),
        [StringComparison]::OrdinalIgnoreCase)
}

function Is-Within([string]$Child,[string]$Parent) {
    $childFull = [IO.Path]::GetFullPath($Child).TrimEnd('\')
    $parentFull = [IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $childFull.Equals($parentFull,[StringComparison]::OrdinalIgnoreCase) -or
        $childFull.StartsWith($parentFull + '\',[StringComparison]::OrdinalIgnoreCase)
}

function Relative([string]$Root,[string]$Path) {
    $prefix = $Root.TrimEnd('\') + '\'
    if (-not $Path.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)) { throw "Path outside tree: $Path" }
    $Path.Substring($prefix.Length).Replace('\','/')
}

function Tree-Rows([string]$Root) {
    $base = (Get-Item -LiteralPath $Root -ErrorAction Stop).FullName.TrimEnd('\')
    $reparse = @(Get-ChildItem -LiteralPath $base -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
    if ($reparse.Count) { throw "Tree contains a reparse point: $($reparse[0].FullName)" }
    @(
        Get-ChildItem -LiteralPath $base -Recurse -File -Force |
            Sort-Object FullName |
            ForEach-Object {
                [ordered]@{ path=Relative $base $_.FullName; bytes=[long]$_.Length; sha256=Get-Hash $_.FullName }
            }
    )
}

function Assert-TreeEqual($Expected,$Actual,[string]$Label) {
    if (@($Expected).Count -ne @($Actual).Count) { throw "$Label file count mismatch" }
    for ($i=0; $i -lt @($Expected).Count; $i++) {
        if ([string]$Expected[$i].path -cne [string]$Actual[$i].path -or
            [long]$Expected[$i].bytes -ne [long]$Actual[$i].bytes -or
            [string]$Expected[$i].sha256 -cne [string]$Actual[$i].sha256) {
            throw "$Label hash mismatch: $($Expected[$i].path)"
        }
    }
}

function Invoke-PackageCheck([string]$Plan,[string]$Root,[string]$Manifest,[string]$Checker) {
    if (-not (Test-Path -LiteralPath $Checker -PathType Leaf)) { throw "Package checker missing: $Checker" }
    $result = @( & $Checker -CoursePlanPath $Plan -PackageRoot $Root -ManifestPath $Manifest )
    if ($result.Count -ne 1 -or [string]$result[0].schema -cne 'babata.mba-course-c2b-package-check/v1' -or
        [string]$result[0].status -cne 'passed') {
        throw 'Package checker did not return one passed result'
    }
    $result[0]
}

if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' }
$data = (Get-Item -LiteralPath $DataHome -ErrorAction Stop).FullName.TrimEnd('\')
$planPath = (Get-Item -LiteralPath $CoursePlanPath -ErrorAction Stop).FullName
$package = (Get-Item -LiteralPath $PackageRoot -ErrorAction Stop).FullName
if (-not (Test-Path -LiteralPath $package -PathType Container)) { throw "Package root is not a directory: $package" }
if ([string]::IsNullOrWhiteSpace($ManifestPath)) { $ManifestPath = Join-Path (Split-Path $package -Parent) 'manifest.json' }
$manifestPath = (Get-Item -LiteralPath $ManifestPath -ErrorAction Stop).FullName
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if ([string]$plan.schema -cne 'babata.mba-course-c2b-plan/v1') { throw 'Unsupported MBA course plan schema' }
if ([string]$plan.output_status -cne 'pending_user_acceptance') { throw 'Publisher only handles a pending_user_acceptance course plan' }
if ([string]$manifest.status -eq 'accepted_benchmark') { throw 'Generic MBA publisher rejects accepted_benchmark' }

$initialCheck = Invoke-PackageCheck $planPath $package $manifestPath $CheckerScript
$live = [IO.Path]::GetFullPath([string]$initialCheck.live_path).TrimEnd('\')
$liveParent = Split-Path $live -Parent
if (-not (Test-Path -LiteralPath $liveParent -PathType Container)) { throw "Live parent directory is missing: $liveParent" }
$liveName = Split-Path $live -Leaf
if ([string]::IsNullOrWhiteSpace($liveName) -or $liveName -in @('.','..')) { throw 'Live course directory is invalid' }
$candidate = $live + '.publish-candidate'
$rollback = $live + '.publish-rollback'
$failed = $live + '.publish-failed'
foreach ($path in @($candidate,$rollback,$failed)) {
    if (Test-Path -LiteralPath $path) { throw "Refusing to overwrite a pre-existing publish safety path: $path" }
}
$archiveRoot = Join-Path $data ('04_runtime\archive\mba-course-c2b\' + [string]$plan.course_key)
$receiptRoot = Join-Path $data ('04_runtime\receipts\mba-course-c2b\' + [string]$plan.course_key)
if ($manifest.PSObject.Properties['publication'] -and $manifest.publication.PSObject.Properties['archive_root'] -and
    -not (Same-Path $archiveRoot ([string]$manifest.publication.archive_root))) {
    throw 'Manifest archive root does not match the deterministic external archive root'
}
if ($manifest.PSObject.Properties['publication'] -and $manifest.publication.PSObject.Properties['candidate_path'] -and
    -not (Same-Path $candidate ([string]$manifest.publication.candidate_path))) {
    throw 'Manifest candidate path does not match the deterministic sibling candidate path'
}
if ([IO.Path]::GetPathRoot($live) -ine [IO.Path]::GetPathRoot($candidate)) { throw 'Candidate and live must be on the same volume' }
if (Same-Path $live $archiveRoot -or Same-Path $live $data) { throw 'Live path is outside the permitted course-vault boundary' }
if ((Is-Within $archiveRoot $live) -or (Is-Within $live $archiveRoot) -or
    (Is-Within $package $live) -or (Is-Within $live $package)) {
    throw 'Package, live, and external archive roots must be disjoint'
}

$lockRoot = Join-Path $data '04_runtime\locks\mba-course-c2b'
$lockPath = Join-Path $lockRoot (([string]$plan.course_key) + '.lock')
foreach ($runtimePath in @($archiveRoot,$receiptRoot,$lockRoot)) {
    if ((Is-Within $runtimePath $package) -or (Is-Within $package $runtimePath) -or
        (Is-Within $runtimePath $live) -or (Is-Within $live $runtimePath)) {
        throw "Runtime archive/receipt/lock paths must be disjoint from package and live: $runtimePath"
    }
}
New-Item -ItemType Directory -Path $lockRoot -Force | Out-Null
$lock = $null
$lockOwned = $false
$oldExists = Test-Path -LiteralPath $live -PathType Container
$oldArchive = $null
$switched = $false
$oldMoved = $false
$receiptPath = $null
try {
    try { $lock = [IO.File]::Open($lockPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None) }
    catch { throw "Another publish is already running for course $($plan.course_key)" }
    $lockOwned = $true

    # Recheck while holding the course lock so a concurrent package cannot win the race.
    $initialCheck = Invoke-PackageCheck $planPath $package $manifestPath $CheckerScript
    foreach ($path in @($candidate,$rollback,$failed)) {
        if (Test-Path -LiteralPath $path) { throw "Refusing to overwrite a pre-existing publish safety path: $path" }
    }
    $oldExists = Test-Path -LiteralPath $live -PathType Container
    New-Item -ItemType Directory -Path $archiveRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $receiptRoot -Force | Out-Null

    New-Item -ItemType Directory -Path $candidate -Force | Out-Null
    Get-ChildItem -LiteralPath $package -Force | Copy-Item -Destination $candidate -Recurse -Force
    $candidateCheck = Invoke-PackageCheck $planPath $candidate $manifestPath $CheckerScript

    if ($oldExists) {
        $oldTree = @(Tree-Rows $live)
        $archiveName = 'live-before-' + (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff') + '-' + (Get-Hash $manifestPath).Substring(0,12)
        $oldArchive = Join-Path $archiveRoot $archiveName
        if (Test-Path -LiteralPath $oldArchive) { throw "Archive collision: $oldArchive" }
        New-Item -ItemType Directory -Path $oldArchive -Force | Out-Null
        Get-ChildItem -LiteralPath $live -Force | Copy-Item -Destination $oldArchive -Recurse -Force
        Assert-TreeEqual $oldTree @(Tree-Rows $oldArchive) 'Archived live'
    }

    try {
        if ($oldExists) { Move-Item -LiteralPath $live -Destination $rollback; $oldMoved = $true }
        Move-Item -LiteralPath $candidate -Destination $live
        $switched = $true
        $liveCheck = Invoke-PackageCheck $planPath $live $manifestPath $CheckerScript
    }
    catch {
        if ($switched -and (Test-Path -LiteralPath $live)) { Move-Item -LiteralPath $live -Destination $failed }
        if ($oldMoved -and (Test-Path -LiteralPath $rollback)) { Move-Item -LiteralPath $rollback -Destination $live }
        $switched = $false
        $oldMoved = $false
        throw
    }

    $receipt = [ordered]@{
        schema='babata.mba-course-c2b-publish-receipt/v1'
        status='pending_user_acceptance'
        course=[string]$plan.course
        course_key=[string]$plan.course_key
        course_plan=$planPath
        course_plan_sha256=Get-Hash $planPath
        manifest=$manifestPath
        manifest_sha256=Get-Hash $manifestPath
        package_root=$package
        live_path=$live
        live_file=[string]$plan.live.file
        archive_path=$oldArchive
        package_files=[int]$liveCheck.package_files
        live_hashes_verified=$true
        checker_schema='babata.mba-course-c2b-package-check/v1'
        published_at=(Get-Date).ToUniversalTime().ToString('o')
    }
    $receiptName = 'publish-' + (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff') + '-' + $receipt.manifest_sha256.Substring(0,12) + '.json'
    $receiptPath = Join-Path $receiptRoot $receiptName
    $receiptTemp = $receiptPath + '.tmp'
    $receipt | ConvertTo-Json -Depth 15 | Set-Content -LiteralPath $receiptTemp -Encoding utf8
    Move-Item -LiteralPath $receiptTemp -Destination $receiptPath
    if ($oldMoved -and (Test-Path -LiteralPath $rollback)) { Remove-Item -LiteralPath $rollback -Recurse -Force }
    [pscustomobject][ordered]@{
        schema='babata.mba-course-c2b-publish/v1'
        status='published_pending_user_acceptance'
        course=[string]$plan.course
        course_key=[string]$plan.course_key
        live_path=$live
        live_file=[string]$plan.live.file
        archive_path=$oldArchive
        receipt=$receiptPath
        package_files=[int]$liveCheck.package_files
        package_live_hashes_equal=$true
    }
}
catch {
    if ($receiptPath -and (Test-Path -LiteralPath $receiptPath)) { Remove-Item -LiteralPath $receiptPath -Force }
    if ($receiptPath -and (Test-Path -LiteralPath ($receiptPath + '.tmp'))) { Remove-Item -LiteralPath ($receiptPath + '.tmp') -Force }
    if ($switched -and (Test-Path -LiteralPath $live)) {
        if (Test-Path -LiteralPath $failed) { throw "Rollback safety path already exists: $failed" }
        Move-Item -LiteralPath $live -Destination $failed
    }
    if ($oldMoved -and (Test-Path -LiteralPath $rollback)) {
        Move-Item -LiteralPath $rollback -Destination $live
    }
    if (Test-Path -LiteralPath $candidate) { Remove-Item -LiteralPath $candidate -Recurse -Force }
    throw
}
finally {
    if ($null -ne $lock) { $lock.Dispose() }
    if ($lockOwned -and (Test-Path -LiteralPath $lockPath)) { Remove-Item -LiteralPath $lockPath -Force }
}
