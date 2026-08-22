[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$PlanPath,

    [Parameter(Mandatory)]
    [string]$RoundRoot,

    [string]$RepoRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Join-Path $PSScriptRoot '..'
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$PlanPath = (Resolve-Path -LiteralPath $PlanPath).Path
$planDirectory = Split-Path -Parent $PlanPath
$RoundRoot = [IO.Path]::GetFullPath($RoundRoot)

function Test-PathWithin {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Parent
    )

    $candidate = [IO.Path]::GetFullPath($Path).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $boundary = [IO.Path]::GetFullPath($Parent).TrimEnd([IO.Path]::DirectorySeparatorChar)
    return $candidate.Equals($boundary, [StringComparison]::OrdinalIgnoreCase) -or
        $candidate.StartsWith($boundary + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)
}

function Get-RequiredText {
    param(
        [Parameter(Mandatory)]$Object,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Context
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
        throw "$Context requires non-empty '$Name'."
    }
    return [string]$property.Value
}

function Get-OptionalArray {
    param(
        [Parameter(Mandatory)]$Object,
        [Parameter(Mandatory)][string]$Name
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) {
        return @()
    }
    return @($property.Value)
}

function Get-BabataBuildIdentity {
    $manifestPath = Join-Path $RepoRoot '01_app\Cargo.toml'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8
    $versionMatch = [regex]::Match(
        $manifest,
        '(?ms)^\[workspace\.package\]\s+.*?^version\s*=\s*"(?<version>[^"]+)"\s*$'
    )
    if (-not $versionMatch.Success) {
        throw "Babata workspace version is missing from $manifestPath"
    }

    $LASTEXITCODE = 0
    $commit = (& git -C $RepoRoot rev-parse HEAD 2>$null | Select-Object -First 1)
    $commitExitCode = $LASTEXITCODE
    if ($commitExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($commit)) {
        throw "Cannot resolve Babata Git commit from $RepoRoot"
    }
    $commit = $commit.Trim()
    $version = $versionMatch.Groups['version'].Value
    $LASTEXITCODE = 0
    $releaseTag = @(& git -C $RepoRoot tag --points-at $commit --list "v$version" 2>$null |
        Where-Object { $_ -eq "v$version" } | Select-Object -First 1)
    $tagExitCode = $LASTEXITCODE
    if ($tagExitCode -ne 0) {
        throw "Cannot resolve Babata Git tag from $RepoRoot"
    }
    $LASTEXITCODE = 0
    $status = @(& git -C $RepoRoot status --porcelain=v1 --untracked-files=normal 2>$null)
    $statusExitCode = $LASTEXITCODE
    if ($statusExitCode -ne 0) {
        throw "Cannot resolve Babata worktree status from $RepoRoot"
    }

    return [pscustomobject][ordered]@{
        version = $version
        release_tag = if ($releaseTag.Count -eq 1) { [string]$releaseTag[0] } else { $null }
        git_commit = $commit
        worktree_dirty = ($status.Count -gt 0)
    }
}

function Expand-RoundToken {
    param([Parameter(Mandatory)][string]$Value)

    return $Value.Replace('{repo}', $RepoRoot).
        Replace('{plan_dir}', $planDirectory).
        Replace('{round_root}', $RoundRoot)
}

function Resolve-PlanPathValue {
    param([Parameter(Mandatory)][string]$Value)

    $expanded = Expand-RoundToken $Value
    if (-not [IO.Path]::IsPathRooted($expanded)) {
        $expanded = Join-Path $planDirectory $expanded
    }
    return [IO.Path]::GetFullPath($expanded)
}

function Get-BytesHash {
    param([Parameter(Mandatory)][byte[]]$Bytes)

    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ([Convert]::ToHexString($sha.ComputeHash($Bytes))).ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Get-PathFingerprint {
    param([Parameter(Mandatory)][string]$Path)

    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        return [pscustomobject][ordered]@{
            path = $Path
            kind = 'file'
            file_count = 1
            sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Frozen path does not exist: $Path"
    }

    $lines = [Collections.Generic.List[string]]::new()
    $files = @(Get-ChildItem -LiteralPath $Path -File -Recurse | Sort-Object FullName)
    foreach ($file in $files) {
        $relative = [IO.Path]::GetRelativePath($Path, $file.FullName).Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $lines.Add("$relative`t$hash")
    }
    $payload = [Text.UTF8Encoding]::new($false).GetBytes(($lines -join "`n"))
    return [pscustomobject][ordered]@{
        path = $Path
        kind = 'directory'
        file_count = $files.Count
        sha256 = Get-BytesHash $payload
    }
}

function Write-JsonAtomic {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )

    $json = $Value | ConvertTo-Json -Depth 40
    $temporary = "$Path.tmp"
    [IO.File]::WriteAllText($temporary, $json, [Text.UTF8Encoding]::new($false))
    [IO.File]::Move($temporary, $Path, $true)
}

function Get-FrozenDrift {
    param([Parameter(Mandatory)][object[]]$Baseline)

    $differences = [Collections.Generic.List[object]]::new()
    foreach ($expected in $Baseline) {
        try {
            $actual = Get-PathFingerprint -Path ([string]$expected.path)
            if ($actual.kind -ne $expected.kind -or
                $actual.file_count -ne $expected.file_count -or
                $actual.sha256 -ne $expected.sha256) {
                $differences.Add([pscustomobject][ordered]@{
                    path = [string]$expected.path
                    expected_sha256 = [string]$expected.sha256
                    actual_sha256 = [string]$actual.sha256
                    reason = 'fingerprint_changed'
                })
            }
        }
        catch {
            $differences.Add([pscustomobject][ordered]@{
                path = [string]$expected.path
                expected_sha256 = [string]$expected.sha256
                actual_sha256 = $null
                reason = 'missing_or_unreadable'
            })
        }
    }
    return @($differences)
}

function Add-RemainingStageResults {
    param(
        [Parameter(Mandatory)][int]$StartIndex,
        [Parameter(Mandatory)][string]$Status,
        [Parameter(Mandatory)][string]$Reason
    )

    for ($remainingIndex = $StartIndex; $remainingIndex -lt $stages.Count; $remainingIndex++) {
        $remaining = $stages[$remainingIndex]
        $stageResults.Add([pscustomobject][ordered]@{
            id = [string]$remaining.id
            status = $Status
            reason = $Reason
            failure_policy = [string]$remaining.failure_policy
            started_at = $null
            finished_at = $null
            exit_code = $null
            stdout_log = $null
            stderr_log = $null
        })
    }
}

if (Test-Path -LiteralPath $RoundRoot) {
    throw "Refusing to reuse existing round root: $RoundRoot"
}
if (Test-PathWithin -Path $RoundRoot -Parent $RepoRoot) {
    throw "Round root must remain outside the Git repository: $RoundRoot"
}

$planText = Get-Content -LiteralPath $PlanPath -Raw -Encoding utf8
$plan = $planText | ConvertFrom-Json
if ((Get-RequiredText -Object $plan -Name schema -Context 'Round plan') -ne 'babata.execution-round-plan/v1') {
    throw 'Round plan schema must be babata.execution-round-plan/v1.'
}
$roundId = Get-RequiredText -Object $plan -Name round_id -Context 'Round plan'
$scope = Get-RequiredText -Object $plan -Name scope -Context 'Round plan'
$targetTerminal = Get-RequiredText -Object $plan -Name target_terminal -Context 'Round plan'
$stages = @(Get-OptionalArray -Object $plan -Name stages)
$acceptanceDefinitions = @(Get-OptionalArray -Object $plan -Name acceptance)
if ($stages.Count -eq 0) {
    throw 'Round plan requires at least one stage.'
}
if ($acceptanceDefinitions.Count -eq 0) {
    throw 'Round plan requires at least one acceptance item.'
}

$stageIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$resolvedScripts = [Collections.Generic.List[string]]::new()
for ($index = 0; $index -lt $stages.Count; $index++) {
    $stage = $stages[$index]
    $stageId = Get-RequiredText -Object $stage -Name id -Context "Stage $index"
    if ($stageId -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$' -or -not $stageIds.Add($stageId)) {
        throw "Stage id is invalid or duplicated: $stageId"
    }
    $failurePolicy = Get-RequiredText -Object $stage -Name failure_policy -Context "Stage '$stageId'"
    if ($failurePolicy -notin @('record_and_continue', 'abort_round')) {
        throw "Stage '$stageId' has invalid failure_policy '$failurePolicy'."
    }
    $script = Resolve-PlanPathValue (Get-RequiredText -Object $stage -Name script -Context "Stage '$stageId'")
    if (-not (Test-Path -LiteralPath $script -PathType Leaf) -or [IO.Path]::GetExtension($script) -ne '.ps1') {
        throw "Stage '$stageId' script must be an existing .ps1 file: $script"
    }
    $stage | Add-Member -NotePropertyName resolved_script -NotePropertyValue $script
    $resolvedScripts.Add($script)
}

$seenStages = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($stage in $stages) {
    foreach ($dependency in @(Get-OptionalArray -Object $stage -Name depends_on)) {
        $dependencyId = [string]$dependency
        if (-not $seenStages.Contains($dependencyId)) {
            throw "Stage '$($stage.id)' dependency must name an earlier stage: $dependencyId"
        }
    }
    [void]$seenStages.Add([string]$stage.id)
}

foreach ($acceptance in $acceptanceDefinitions) {
    $acceptanceId = Get-RequiredText -Object $acceptance -Name id -Context 'Acceptance item'
    $requiredStages = @(Get-OptionalArray -Object $acceptance -Name stage_ids)
    if ($requiredStages.Count -eq 0) {
        throw "Acceptance item '$acceptanceId' requires stage_ids."
    }
    foreach ($requiredStage in $requiredStages) {
        if (-not $stageIds.Contains([string]$requiredStage)) {
            throw "Acceptance item '$acceptanceId' names unknown stage '$requiredStage'."
        }
    }
}

$frozenPathSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
[void]$frozenPathSet.Add($PlanPath)
foreach ($script in $resolvedScripts) {
    [void]$frozenPathSet.Add($script)
}
$explicitFrozenPaths = @(Get-OptionalArray -Object $plan -Name frozen_paths)
if ($explicitFrozenPaths.Count -eq 0) {
    throw 'Round plan must declare at least one explicit frozen input or contract path.'
}
foreach ($entry in $explicitFrozenPaths) {
    $value = if ($entry -is [string]) { [string]$entry } else { Get-RequiredText -Object $entry -Name path -Context 'Frozen path' }
    $resolved = Resolve-PlanPathValue $value
    if (Test-PathWithin -Path $resolved -Parent $RoundRoot) {
        throw "Frozen path cannot be inside the new round root: $resolved"
    }
    [void]$frozenPathSet.Add($resolved)
}

$frozenBaseline = @($frozenPathSet | Sort-Object | ForEach-Object { Get-PathFingerprint -Path $_ })
$babataBuild = Get-BabataBuildIdentity
New-Item -ItemType Directory -Path $RoundRoot | Out-Null
$logsRoot = Join-Path $RoundRoot 'logs'
New-Item -ItemType Directory -Path $logsRoot | Out-Null
$snapshotPath = Join-Path $RoundRoot 'round-plan.snapshot.json'
[IO.File]::WriteAllText($snapshotPath, $planText, [Text.UTF8Encoding]::new($false))
$ledgerPath = Join-Path $RoundRoot 'round-ledger.json'
$stageResults = [Collections.Generic.List[object]]::new()
$defects = [Collections.Generic.List[object]]::new()
$startedAt = [DateTimeOffset]::UtcNow.ToString('o')
$ledger = [ordered]@{
    schema = 'babata.execution-round-ledger/v1'
    round_id = $roundId
    scope = $scope
    target_terminal = $targetTerminal
    actual_terminal = 'running'
    status = 'running'
    started_at = $startedAt
    finished_at = $null
    repo_root = $RepoRoot
    round_root = $RoundRoot
    babata_build = $babataBuild
    plan = [ordered]@{
        source = $PlanPath
        snapshot = $snapshotPath
        sha256 = (Get-FileHash -LiteralPath $snapshotPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    frozen_paths = $frozenBaseline
    stages = $stageResults
    defects = $defects
    acceptance = @()
}
Write-JsonAtomic -Path $ledgerPath -Value $ledger

$pwsh = (Get-Command pwsh -ErrorAction Stop).Source
$aborted = $false
$stageStatus = @{}

for ($index = 0; $index -lt $stages.Count; $index++) {
    $stage = $stages[$index]
    $stageId = [string]$stage.id
    $dependencies = @(Get-OptionalArray -Object $stage -Name depends_on)
    $failedDependencies = @($dependencies | Where-Object { $stageStatus[[string]$_] -ne 'passed' })
    if ($failedDependencies.Count -gt 0) {
        $stageStatus[$stageId] = 'skipped_dependency'
        $stageResults.Add([pscustomobject][ordered]@{
            id = $stageId
            status = 'skipped_dependency'
            reason = "dependency_not_passed:$($failedDependencies -join ',')"
            failure_policy = [string]$stage.failure_policy
            started_at = $null
            finished_at = $null
            exit_code = $null
            stdout_log = $null
            stderr_log = $null
        })
        Write-JsonAtomic -Path $ledgerPath -Value $ledger
        continue
    }

    $drift = @(Get-FrozenDrift -Baseline $frozenBaseline)
    if ($drift.Count -gt 0) {
        $stageStatus[$stageId] = 'not_run_frozen_drift'
        $stageResults.Add([pscustomobject][ordered]@{
            id = $stageId
            status = 'not_run_frozen_drift'
            reason = 'frozen_state_drift_before_stage'
            failure_policy = [string]$stage.failure_policy
            started_at = $null
            finished_at = [DateTimeOffset]::UtcNow.ToString('o')
            exit_code = $null
            stdout_log = $null
            stderr_log = $null
        })
        $defects.Add([pscustomobject][ordered]@{
            id = 'D{0:D4}' -f ($defects.Count + 1)
            stage_id = $stageId
            severity = 'invalidating'
            category = 'frozen_state_drift'
            detail = $drift
        })
        Add-RemainingStageResults -StartIndex ($index + 1) -Status 'not_run_aborted' -Reason 'round_aborted'
        $aborted = $true
        break
    }

    $stdoutPath = Join-Path $logsRoot ('{0:D3}-{1}.stdout.log' -f ($index + 1), $stageId)
    $stderrPath = Join-Path $logsRoot ('{0:D3}-{1}.stderr.log' -f ($index + 1), $stageId)
    $workingDirectory = $RepoRoot
    $workingDirectoryProperty = $stage.PSObject.Properties['working_directory']
    if ($null -ne $workingDirectoryProperty -and -not [string]::IsNullOrWhiteSpace([string]$workingDirectoryProperty.Value)) {
        $workingDirectory = Resolve-PlanPathValue ([string]$workingDirectoryProperty.Value)
    }
    if (-not (Test-Path -LiteralPath $workingDirectory -PathType Container)) {
        throw "Stage '$stageId' working directory does not exist: $workingDirectory"
    }

    $timeoutSeconds = 3600
    $timeoutProperty = $stage.PSObject.Properties['timeout_seconds']
    if ($null -ne $timeoutProperty -and $null -ne $timeoutProperty.Value) {
        $timeoutSeconds = [int]$timeoutProperty.Value
    }
    if ($timeoutSeconds -lt 1 -or $timeoutSeconds -gt 86400) {
        throw "Stage '$stageId' timeout_seconds must be between 1 and 86400."
    }

    $processInfo = [Diagnostics.ProcessStartInfo]::new()
    $processInfo.FileName = $pwsh
    $processInfo.UseShellExecute = $false
    $processInfo.RedirectStandardOutput = $true
    $processInfo.RedirectStandardError = $true
    $processInfo.WorkingDirectory = $workingDirectory
    $processInfo.ArgumentList.Add('-NoProfile')
    $processInfo.ArgumentList.Add('-NonInteractive')
    $processInfo.ArgumentList.Add('-File')
    $processInfo.ArgumentList.Add([string]$stage.resolved_script)
    foreach ($argument in @(Get-OptionalArray -Object $stage -Name arguments)) {
        $processInfo.ArgumentList.Add((Expand-RoundToken ([string]$argument)))
    }

    $stageStarted = [DateTimeOffset]::UtcNow.ToString('o')
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $processInfo
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $completed = $process.WaitForExit($timeoutSeconds * 1000)
    if (-not $completed) {
        $process.Kill($true)
        $process.WaitForExit()
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    [IO.File]::WriteAllText($stdoutPath, $stdout, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($stderrPath, $stderr, [Text.UTF8Encoding]::new($false))
    $exitCode = if ($completed) { $process.ExitCode } else { 124 }
    $process.Dispose()
    $stageFinished = [DateTimeOffset]::UtcNow.ToString('o')
    $resultStatus = if ($exitCode -eq 0) { 'passed' } else { 'failed' }
    $stageStatus[$stageId] = $resultStatus
    $stageResults.Add([pscustomobject][ordered]@{
        id = $stageId
        status = $resultStatus
        reason = if ($completed) { $null } else { 'timeout' }
        failure_policy = [string]$stage.failure_policy
        started_at = $stageStarted
        finished_at = $stageFinished
        exit_code = $exitCode
        stdout_log = $stdoutPath
        stderr_log = $stderrPath
    })

    if ($exitCode -ne 0) {
        $categoryProperty = $stage.PSObject.Properties['failure_category']
        $category = if ($null -eq $categoryProperty -or [string]::IsNullOrWhiteSpace([string]$categoryProperty.Value)) {
            'stage_failure'
        }
        else {
            [string]$categoryProperty.Value
        }
        $defects.Add([pscustomobject][ordered]@{
            id = 'D{0:D4}' -f ($defects.Count + 1)
            stage_id = $stageId
            severity = if ($stage.failure_policy -eq 'abort_round') { 'invalidating' } else { 'non_invalidating' }
            category = $category
            exit_code = $exitCode
            stdout_log = $stdoutPath
            stderr_log = $stderrPath
            root_cause_cluster = $null
        })
        if ($stage.failure_policy -eq 'abort_round') {
            Add-RemainingStageResults -StartIndex ($index + 1) -Status 'not_run_aborted' -Reason "invalidating_failure:$stageId"
            $aborted = $true
            Write-JsonAtomic -Path $ledgerPath -Value $ledger
            break
        }
    }

    $postStageDrift = @(Get-FrozenDrift -Baseline $frozenBaseline)
    if ($postStageDrift.Count -gt 0) {
        $defects.Add([pscustomobject][ordered]@{
            id = 'D{0:D4}' -f ($defects.Count + 1)
            stage_id = $stageId
            severity = 'invalidating'
            category = 'frozen_state_drift'
            detail = $postStageDrift
        })
        Add-RemainingStageResults -StartIndex ($index + 1) -Status 'not_run_aborted' -Reason 'frozen_state_drift_after_stage'
        $aborted = $true
        Write-JsonAtomic -Path $ledgerPath -Value $ledger
        break
    }
    Write-JsonAtomic -Path $ledgerPath -Value $ledger
}

$acceptanceResults = [Collections.Generic.List[object]]::new()
foreach ($definition in $acceptanceDefinitions) {
    $acceptanceId = [string]$definition.id
    $requiredProperty = $definition.PSObject.Properties['required']
    $required = $null -eq $requiredProperty -or $null -eq $requiredProperty.Value -or [bool]$requiredProperty.Value
    $requiredStages = @($definition.stage_ids | ForEach-Object { [string]$_ })
    $failedStages = @($requiredStages | Where-Object { $stageStatus[$_] -ne 'passed' })
    $descriptionProperty = $definition.PSObject.Properties['description']
    $acceptanceResults.Add([pscustomobject][ordered]@{
        id = $acceptanceId
        description = if ($null -eq $descriptionProperty) { $acceptanceId } else { [string]$descriptionProperty.Value }
        required = $required
        stage_ids = $requiredStages
        status = if ($failedStages.Count -eq 0) { 'passed' } else { 'failed' }
        failed_stage_ids = $failedStages
    })
}
$ledger.acceptance = $acceptanceResults
$requiredFailures = @($acceptanceResults | Where-Object { $_.required -and $_.status -ne 'passed' })
$ledger.finished_at = [DateTimeOffset]::UtcNow.ToString('o')
if ($aborted) {
    $ledger.status = 'aborted'
    $ledger.actual_terminal = 'aborted'
    $exitCode = 3
}
elseif ($requiredFailures.Count -gt 0 -or $defects.Count -gt 0) {
    $ledger.status = 'terminal_with_defects'
    $ledger.actual_terminal = 'terminal_with_defects'
    $exitCode = 2
}
else {
    $ledger.status = 'passed'
    $ledger.actual_terminal = $targetTerminal
    $exitCode = 0
}
Write-JsonAtomic -Path $ledgerPath -Value $ledger

[pscustomobject]@{
    round_id = $roundId
    status = $ledger.status
    actual_terminal = $ledger.actual_terminal
    defects = $defects.Count
    ledger = $ledgerPath
}
exit $exitCode
