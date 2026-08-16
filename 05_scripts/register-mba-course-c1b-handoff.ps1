[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$PreparationManifestPath,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe'),
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [string]$CandidateSelector = (Join-Path $PSScriptRoot 'select-mba-course-c1-candidate.ps1'),
    [switch]$Resume,
    [switch]$PreflightOnly,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# sqlite3 and babata emit UTF-8 JSON. Hidden/non-console Windows runners can
# otherwise decode native stdout with an OEM code page and corrupt Chinese
# fingerprint fields before ConvertFrom-Json and canonical comparison.
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$OutputEncoding = $utf8NoBom
[Console]::OutputEncoding = $utf8NoBom

$pipelineId = 'agent_import'
$provider = 'local_extract'
$toolVersion = '1.0.0'
$mediaModel = 'mba-course-c1b-media-extractor'
$decisionModel = 'mba-course-c1b-essence-registrar'

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-StringSha256([string]$Value) {
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($Value)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    return [Convert]::ToHexString($hash).ToLowerInvariant()
}

function ConvertTo-CompactJson($Value, [int]$Depth = 30) {
    return ($Value | ConvertTo-Json -Depth $Depth -Compress)
}

function ConvertTo-CanonicalJsonValue($Value) {
    if ($null -eq $Value) { return $null }
    if ($Value -is [string] -or $Value -is [ValueType]) { return $Value }
    if ($Value -is [System.Collections.IDictionary]) {
        $ordered = [ordered]@{}
        foreach ($key in @($Value.Keys | Sort-Object)) {
            $ordered[[string]$key] = ConvertTo-CanonicalJsonValue $Value[$key]
        }
        return $ordered
    }
    if ($Value -is [System.Collections.IEnumerable]) {
        return @($Value | ForEach-Object { ConvertTo-CanonicalJsonValue $_ })
    }
    $properties = @($Value.PSObject.Properties | Sort-Object Name)
    if ($properties.Count -eq 0) { return $Value }
    $orderedObject = [ordered]@{}
    foreach ($property in $properties) {
        $orderedObject[[string]$property.Name] = ConvertTo-CanonicalJsonValue $property.Value
    }
    return $orderedObject
}

function Normalize-Json([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return '' }
    $parsed = $Value | ConvertFrom-Json -DateKind String
    return ConvertTo-CompactJson (ConvertTo-CanonicalJsonValue $parsed) 100
}

function Select-ExactFingerprintRows($Rows, [string]$ParamsJson, [string]$Label) {
    $expected = Normalize-Json $ParamsJson
    $matches = @($Rows | Where-Object { (Normalize-Json ([string]$_.params_json)) -ceq $expected })
    if ($matches.Count -gt 1) {
        throw "Multiple active exact fingerprints for $Label"
    }
    return $matches
}

function Escape-Sql([string]$Value) {
    return $Value.Replace("'", "''")
}

function Assert-Equal([string]$Actual, [string]$Expected, [string]$Label) {
    if ($Actual -cne $Expected) {
        throw "$Label mismatch: expected '$Expected', got '$Actual'"
    }
}

function Assert-Sha256([string]$Value, [string]$Label) {
    if ($Value -cnotmatch '^[0-9a-f]{64}$') { throw "$Label is not a lowercase SHA-256: $Value" }
}

function Require-Rfc3339Utc($Value, [string]$Label) {
    $text = [string]$Value
    $parsed = [DateTimeOffset]::MinValue
    if ($text -cnotmatch '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,7})?Z$' -or
        -not [DateTimeOffset]::TryParse(
            $text,
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::AssumeUniversal,
            [ref]$parsed
        ) -or $parsed.Offset -ne [TimeSpan]::Zero) {
        throw "$Label must be RFC3339 UTC: $text"
    }
    return $text
}

function Resolve-InputFile([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "$Label missing: $Path" }
    return (Get-Item -LiteralPath $Path).FullName
}

function Test-IsWithin([string]$Root, [string]$Candidate) {
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    $candidateFull = [IO.Path]::GetFullPath($Candidate)
    $prefix = $rootFull + [IO.Path]::DirectorySeparatorChar
    return $candidateFull.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)
}

function Resolve-PreparationPath([string]$PreparationRoot, [string]$DeclaredPath, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($DeclaredPath)) { throw "$Label path is missing" }
    $candidate = if ([IO.Path]::IsPathRooted($DeclaredPath)) {
        [IO.Path]::GetFullPath($DeclaredPath)
    } else {
        [IO.Path]::GetFullPath((Join-Path $PreparationRoot $DeclaredPath))
    }
    if (-not (Test-IsWithin $PreparationRoot $candidate)) {
        throw "$Label must remain inside the preparation root: $candidate"
    }
    return Resolve-InputFile $candidate $Label
}

function Resolve-RelativePreparationFile([string]$PreparationRoot, [string]$RelativePath, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or [IO.Path]::IsPathRooted($RelativePath)) {
        throw "$Label must be a relative preparation path"
    }
    $candidate = [IO.Path]::GetFullPath((Join-Path $PreparationRoot ($RelativePath -replace '/', '\')))
    if (-not (Test-IsWithin $PreparationRoot $candidate)) {
        throw "$Label escapes the preparation root: $RelativePath"
    }
    return Resolve-InputFile $candidate $Label
}

function Invoke-SqliteJson([string]$Database, [string]$Sql) {
    $output = & sqlite3 -json $Database $Sql
    if ($LASTEXITCODE -ne 0) { throw "sqlite read failed: $Sql" }
    $text = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text) -or $text -eq '[]') { return @() }
    return @($text | ConvertFrom-Json -DateKind String)
}

function Invoke-BabataJson([string[]]$Arguments) {
    $output = & $script:exe --json @Arguments
    if ($LASTEXITCODE -ne 0) { throw "babata command failed: $($Arguments -join ' ')" }
    $text = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text)) { throw "babata command returned no JSON: $($Arguments -join ' ')" }
    return ($text | ConvertFrom-Json -DateKind String)
}

function Get-MediaType([string]$Path) {
    switch ([IO.Path]::GetExtension($Path).ToLowerInvariant()) {
        '.png' { return 'image/png' }
        '.jpg' { return 'image/jpeg' }
        default { throw "Only PNG/JPG retained C1B media is supported: $Path" }
    }
}

function Get-SourceLocator($Media) {
    $locator = [ordered]@{}
    foreach ($name in @('page', 'time_seconds', 'percentage', 'crop')) {
        if ($Media.PSObject.Properties[$name]) { $locator[$name] = $Media.$name }
    }
    if ($locator.Count -eq 0) { throw "Retained media requires a source locator: $($Media.path)" }
    return $locator
}

function Find-ExactFingerprint(
    [string]$RevisionId,
    [string]$ItemId,
    [string]$AssetId,
    [string]$InputSha256,
    [string]$Kind,
    [string]$Model,
    [string]$OutputSha256,
    [string]$ParamsJson,
    [string]$LossNotes,
    [string]$MediaType
) {
    if ($script:fingerprintCacheLoaded) {
        $candidates = @($script:fingerprintRows | Where-Object {
            [string]$_.input_revision_id -ceq $RevisionId -and
            [string]$_.input_item_id -ceq $ItemId -and
            [string]$_.input_asset_id -ceq $AssetId -and
            [string]$_.input_sha256 -ceq $InputSha256 -and
            [string]$_.target_kind -ceq $Kind -and
            [string]$_.provider -ceq $provider -and
            [string]$_.tool_or_model -ceq $Model -and
            [string]$_.tool_version -ceq $toolVersion -and
            [string]$_.loss_notes -ceq $LossNotes -and
            [string]$_.kind -ceq $Kind -and
            [string]$_.derivative_input_asset_id -ceq $AssetId -and
            [string]$_.output_sha256 -ceq $OutputSha256 -and
            [string]$_.media_type -ceq $MediaType -and
            [string]$_.language -ceq 'zh'
        })
        return @(Select-ExactFingerprintRows $candidates $ParamsJson "module input $RevisionId, model $Model, output $OutputSha256")
    }
    $sql = @"
SELECT p.run_id,p.pipeline_id,p.input_revision_id,p.input_item_id,p.input_asset_id,
       p.input_sha256,p.target_kind,p.provider,p.tool_or_model,p.tool_version,
       p.params_json,p.loss_notes,p.state,p.invalidated_at,
       d.derivative_id,d.kind,d.output_sha256,d.logical_path,d.media_type,d.language,
       d.input_asset_id AS derivative_input_asset_id
FROM process_runs p
JOIN derivatives d ON d.run_id=p.run_id
WHERE p.pipeline_id='$(Escape-Sql $pipelineId)'
  AND p.input_revision_id='$(Escape-Sql $RevisionId)'
  AND p.input_item_id='$(Escape-Sql $ItemId)'
  AND p.input_asset_id='$(Escape-Sql $AssetId)'
  AND p.input_sha256='$(Escape-Sql $InputSha256)'
  AND p.target_kind='$(Escape-Sql $Kind)'
  AND p.provider='$(Escape-Sql $provider)'
  AND p.tool_or_model='$(Escape-Sql $Model)'
  AND p.tool_version='$(Escape-Sql $toolVersion)'
  AND COALESCE(p.loss_notes,'')='$(Escape-Sql $LossNotes)'
  AND p.state='succeeded'
  AND p.invalidated_at IS NULL
  AND d.kind='$(Escape-Sql $Kind)'
  AND d.input_asset_id='$(Escape-Sql $AssetId)'
  AND d.output_sha256='$(Escape-Sql $OutputSha256)'
  AND d.media_type='$(Escape-Sql $MediaType)'
  AND d.language='zh'
ORDER BY p.created_at,p.run_id,d.derivative_id;
"@
    $candidates = @(Invoke-SqliteJson $script:derivedDb $sql)
    return @(Select-ExactFingerprintRows $candidates $ParamsJson "module input $RevisionId, model $Model, output $OutputSha256")
}

function Find-LiveExactFingerprint(
    [string]$RevisionId,
    [string]$ItemId,
    [string]$AssetId,
    [string]$InputSha256,
    [string]$Kind,
    [string]$Model,
    [string]$OutputSha256,
    [string]$ParamsJson,
    [string]$LossNotes,
    [string]$MediaType
) {
    $cacheState = $script:fingerprintCacheLoaded
    try {
        $script:fingerprintCacheLoaded = $false
        $rows = @(Find-ExactFingerprint $RevisionId $ItemId $AssetId $InputSha256 $Kind $Model $OutputSha256 $ParamsJson $LossNotes $MediaType)
    } finally {
        $script:fingerprintCacheLoaded = $cacheState
    }
    foreach ($row in $rows) {
        if (@($script:fingerprintRows | Where-Object { [string]$_.derivative_id -ceq [string]$row.derivative_id }).Count -eq 0) {
            $script:fingerprintRows += $row
        }
    }
    return $rows
}

function Confirm-ManagedRow($Row, [string]$ExpectedHash, [string]$Label) {
    $logicalPath = [string]$Row.logical_path
    if (-not $logicalPath.StartsWith('02_derived/files/sha256/', [StringComparison]::Ordinal)) {
        throw "$Label is outside managed C1 storage: $logicalPath"
    }
    $managedPath = Join-Path $script:data ($logicalPath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $managedPath -PathType Leaf)) { throw "$Label managed file missing: $managedPath" }
    Assert-Equal (Get-Sha256 $managedPath) $ExpectedHash "$Label managed output hash"
    return $managedPath
}

function Confirm-RegisteredRun(
    [string]$RunId,
    [string]$DerivativeId,
    [string]$RevisionId,
    [string]$ItemId,
    [string]$AssetId,
    [string]$InputSha256,
    [string]$Kind,
    [string]$Model,
    [string]$OutputSha256,
    [string]$ParamsJson,
    [string]$LossNotes,
    [string]$MediaType
) {
    $rows = @(Find-LiveExactFingerprint $RevisionId $ItemId $AssetId $InputSha256 $Kind $Model $OutputSha256 $ParamsJson $LossNotes $MediaType)
    if ($rows.Count -ne 1) { throw "Registered fingerprint was not uniquely readable for run $RunId" }
    $row = $rows[0]
    Assert-Equal ([string]$row.run_id) $RunId 'registered run id'
    Assert-Equal ([string]$row.derivative_id) $DerivativeId 'registered derivative id'

    $shown = Invoke-BabataJson @('process','show-run','--run',$RunId)
    Assert-Equal ([string]$shown.run.id) $RunId 'show-run id'
    Assert-Equal ([string]$shown.run.pipeline_id) $pipelineId 'show-run pipeline'
    Assert-Equal ([string]$shown.run.input_revision_id) $RevisionId 'show-run revision'
    Assert-Equal ([string]$shown.run.input_item_id) $ItemId 'show-run item'
    Assert-Equal ([string]$shown.run.input_asset_id) $AssetId 'show-run input asset'
    Assert-Equal ([string]$shown.run.input_sha256) $InputSha256 'show-run input hash'
    Assert-Equal ([string]$shown.run.target_kind) $Kind 'show-run target kind'
    Assert-Equal ([string]$shown.run.provider) $provider 'show-run provider'
    Assert-Equal ([string]$shown.run.tool_or_model) $Model 'show-run model'
    Assert-Equal ([string]$shown.run.tool_version) $toolVersion 'show-run tool version'
    Assert-Equal ([string]$shown.run.state) 'succeeded' 'show-run state'
    if ($null -ne $shown.run.invalidated_at) { throw "Registered run is invalidated: $RunId" }
    $shownDerivatives = @($shown.derivatives)
    if ($shownDerivatives.Count -ne 1) { throw "Expected one derivative for registered run $RunId" }
    Assert-Equal ([string]$shownDerivatives[0].id) $DerivativeId 'show-run derivative id'
    Assert-Equal ([string]$shownDerivatives[0].kind) $Kind 'show-run derivative kind'
    Assert-Equal ([string]$shownDerivatives[0].input_asset_id) $AssetId 'show-run derivative input asset'
    Assert-Equal ([string]$shownDerivatives[0].output_sha256) $OutputSha256 'show-run derivative output hash'

    [void](Confirm-ManagedRow $row $OutputSha256 "run $RunId")
    return [ordered]@{
        run_id = $RunId
        derivative_id = $DerivativeId
        output_sha256 = $OutputSha256
        logical_path = [string]$row.logical_path
    }
}

function Write-Utf8File([string]$Path, [string]$Content) {
    [IO.File]::WriteAllText($Path, $Content, [Text.UTF8Encoding]::new($false))
}

function Write-JsonAtomic([string]$Path, $Value, [int]$Depth = 40) {
    $parent = Split-Path $Path -Parent
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $temporary = Join-Path $parent ('.{0}.{1}.tmp' -f ([IO.Path]::GetFileName($Path)), [guid]::NewGuid().ToString('N'))
    try {
        Write-Utf8File $temporary ($Value | ConvertTo-Json -Depth $Depth)
        [IO.File]::Move($temporary, $Path, $true)
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force }
    }
}

function New-SqlInList([string[]]$Values) {
    return (($Values | ForEach-Object { "'$(Escape-Sql $_)'" }) -join ',')
}

if ($SelfTest) {
    $expectedParams = '{"schema":"v1","adapter":"deterministic_local","nested":{"course":"demo","module_id":"42"},"steps":[{"tool":"render","version":1}]}'
    $rustNormalizedParams = '{"adapter":"deterministic_local","nested":{"module_id":"42","course":"demo"},"schema":"v1","steps":[{"version":1,"tool":"render"}]}'
    if ((Normalize-Json $expectedParams) -cne (Normalize-Json $rustNormalizedParams)) {
        throw 'Self-test failed: semantic JSON equality depends on object key order'
    }
    $single = @(Select-ExactFingerprintRows @([pscustomobject]@{ params_json=$rustNormalizedParams; derivative_id='derivative-one' }) $expectedParams 'self-test single candidate')
    if ($single.Count -ne 1 -or [string]$single[0].derivative_id -cne 'derivative-one') {
        throw 'Self-test failed: Rust-normalized params did not select the exact fingerprint'
    }
    $conflictRejected = $false
    try {
        [void](Select-ExactFingerprintRows @(
            [pscustomobject]@{ params_json=$expectedParams; derivative_id='derivative-one' },
            [pscustomobject]@{ params_json=$rustNormalizedParams; derivative_id='derivative-two' }
        ) $expectedParams 'self-test conflicting candidates')
    } catch {
        if (-not $_.Exception.Message.Contains('Multiple active exact fingerprints')) { throw }
        $conflictRejected = $true
    }
    if (-not $conflictRejected) { throw 'Self-test failed: multiple semantic fingerprint matches were accepted' }
    $atomicPath = Join-Path ([IO.Path]::GetTempPath()) ('babata-c1b-atomic-{0}.json' -f [guid]::NewGuid().ToString('N'))
    try {
        Write-JsonAtomic $atomicPath ([ordered]@{ value = 'first' }) 4
        Write-JsonAtomic $atomicPath ([ordered]@{ value = 'second' }) 4
        $atomicValue = Get-Content -LiteralPath $atomicPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
        if ([string]$atomicValue.value -cne 'second') { throw 'Self-test failed to overwrite an existing atomic JSON target' }
    } finally {
        if (Test-Path -LiteralPath $atomicPath) { Remove-Item -LiteralPath $atomicPath -Force }
    }
    $canonicalTimestamp = '2026-08-15T06:37:24.0000000Z'
    $timestampRoundTrip = '{"started_at":"2026-08-15T06:37:24.0000000Z"}' | ConvertFrom-Json -DateKind String
    if ($timestampRoundTrip.started_at -isnot [string] -or
        (Require-Rfc3339Utc $timestampRoundTrip.started_at 'self-test started_at') -cne $canonicalTimestamp) {
        throw 'Self-test failed to preserve an RFC3339 timestamp as a JSON string'
    }
    $localizedRejected = $false
    try { [void](Require-Rfc3339Utc '08/15/2026 06:37:24' 'self-test started_at') } catch {
        if (-not $_.Exception.Message.Contains('must be RFC3339 UTC')) { throw }
        $localizedRejected = $true
    }
    if (-not $localizedRejected) { throw 'Self-test accepted a culture-formatted timestamp' }
    Write-Output 'generic MBA C1B registrar fingerprint self-test passed; timestamp self-test passed'
    return
}

if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' }
$candidateSelectorPath = Resolve-InputFile $CandidateSelector 'C1 candidate selector'
. $candidateSelectorPath
$script:data = [IO.Path]::GetFullPath($DataHome)
$env:BABATA_DATA_HOME = $script:data
$planPath = Resolve-InputFile $CoursePlanPath 'course plan'
$preparationManifestPathResolved = Resolve-InputFile $PreparationManifestPath 'preparation manifest'
$preparationRoot = Split-Path $preparationManifestPathResolved -Parent
$staging = [IO.Path]::GetFullPath($StagingRoot)
$script:exe = Resolve-InputFile $BabataExe 'Babata executable'
if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) { throw 'sqlite3 is required for read-only preflight' }

$runtimeStagingRoot = [IO.Path]::GetFullPath((Join-Path $script:data '04_runtime\staging\model-workspaces'))
$normalizedStaging = $staging.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
$normalizedPreparation = $preparationRoot.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
if ($normalizedStaging -ceq $normalizedPreparation) { throw 'Registration staging root must differ from the preparation input root' }
if (-not (Test-IsWithin $runtimeStagingRoot $staging)) {
    throw "Registration staging root must be a task directory under $runtimeStagingRoot"
}

if ($Resume) {
    if (-not (Test-Path -LiteralPath $staging -PathType Container)) { throw "Resume staging root does not exist: $staging" }
} elseif (Test-Path -LiteralPath $staging) {
    throw "Use a fresh C1B registration staging root or pass -Resume: $staging"
}
if (Test-IsWithin $preparationRoot $staging) { throw 'Registration staging root must not be inside the preparation input root' }
if (Test-IsWithin $staging $preparationRoot) { throw 'Preparation input root must not be inside the registration staging root' }

$script:rawDb = Resolve-InputFile (Join-Path $script:data '01_raw\index\raw.sqlite') 'raw database'
$script:derivedDb = Resolve-InputFile (Join-Path $script:data '02_derived\index\derived.sqlite') 'derived database'
$rawCheck = @(& sqlite3 $script:rawDb 'PRAGMA quick_check; PRAGMA foreign_key_check;')
if ($LASTEXITCODE -ne 0 -or $rawCheck.Count -ne 1 -or $rawCheck[0] -ne 'ok') { throw 'Raw database integrity preflight failed' }
$derivedCheck = @(& sqlite3 $script:derivedDb 'PRAGMA quick_check; PRAGMA foreign_key_check;')
if ($LASTEXITCODE -ne 0 -or $derivedCheck.Count -ne 1 -or $derivedCheck[0] -ne 'ok') { throw 'Derived database integrity preflight failed' }

$plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
if ([string]$plan.schema -cne 'babata.mba-course-c2b-plan/v1') { throw 'Unsupported MBA course plan schema' }
$course = [string]$plan.course
$courseKey = [string]$plan.course_key
$shortName = [string]$plan.short_name
$expectedModules = [int]$plan.expected_modules
if ([string]::IsNullOrWhiteSpace($course) -or [string]::IsNullOrWhiteSpace($courseKey) -or
    [string]::IsNullOrWhiteSpace($shortName) -or $expectedModules -lt 1) {
    throw 'Course plan requires course, course_key, short_name and expected_modules'
}
if ($courseKey -cnotmatch '^[a-z0-9]+(?:-[a-z0-9]+)*$') { throw "Unsafe course_key: $courseKey" }
$planSha = Get-Sha256 $planPath

$chapterByModule = @{}
$chapterNotes = @{}
foreach ($chapter in @($plan.chapters)) {
    $chapterNote = [string]$chapter.note
    if ([string]::IsNullOrWhiteSpace($chapterNote)) { throw 'Every course chapter requires a note' }
    if ($chapterNotes.ContainsKey($chapterNote)) { throw "Duplicate chapter note: $chapterNote" }
    $chapterNotes[$chapterNote] = $true
    foreach ($module in @($chapter.modules)) {
        $moduleId = [string]$module
        if ([string]::IsNullOrWhiteSpace($moduleId)) { throw "Blank module in chapter $chapterNote" }
        if ($chapterByModule.ContainsKey($moduleId)) { throw "Module appears in multiple chapters: $moduleId" }
        $chapterByModule[$moduleId] = $chapterNote
    }
}
if ($chapterByModule.Count -ne $expectedModules) {
    throw "Course plan denominator mismatch: expected $expectedModules, mapped $($chapterByModule.Count)"
}

$preparationManifest = Get-Content -LiteralPath $preparationManifestPathResolved -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
if ([string]$preparationManifest.schema -cne 'babata.mba-course-c1b-preparation/v1') { throw 'Unsupported MBA C1B preparation manifest schema' }
Assert-Equal ([string]$preparationManifest.status) 'staged_only' 'preparation status'
Assert-Equal ([string]$preparationManifest.course) $course 'preparation course'
if ([int]$preparationManifest.expected_modules -ne $expectedModules -or
    [int]$preparationManifest.complete_c1 -ne $expectedModules -or
    [int]$preparationManifest.c1b_decisions -ne $expectedModules) {
    throw 'Preparation manifest does not cover the complete course denominator'
}
if ([int]$preparationManifest.old_c2b_inputs -ne 0 -or [int]$preparationManifest.external_sovereign_original_reads -ne 0) {
    throw 'Preparation manifest crossed the C1B input boundary'
}
if ($preparationManifest.PSObject.Properties['course_plan_sha256']) {
    Assert-Equal ([string]$preparationManifest.course_plan_sha256) $planSha 'preparation course plan hash'
}
if ($preparationManifest.PSObject.Properties['course_plan']) {
    $declaredPlan = Resolve-InputFile ([string]$preparationManifest.course_plan) 'declared course plan'
    Assert-Equal $declaredPlan $planPath 'preparation course plan path'
}
$preparationManifestSha = Get-Sha256 $preparationManifestPathResolved
$sourceMapPath = Resolve-PreparationPath $preparationRoot ([string]$preparationManifest.source_map) 'source map'
$decisionPath = Resolve-PreparationPath $preparationRoot ([string]$preparationManifest.decisions) 'C1B decisions'
$sourceMapSha = Get-Sha256 $sourceMapPath
$decisionSourceSha = Get-Sha256 $decisionPath
Assert-Equal $sourceMapSha ([string]$preparationManifest.source_map_sha256) 'source map hash'
Assert-Equal $decisionSourceSha ([string]$preparationManifest.decisions_sha256) 'C1B decisions hash'
if ($preparationManifest.PSObject.Properties['visual_plan'] -and -not [string]::IsNullOrWhiteSpace([string]$preparationManifest.visual_plan)) {
    $visualPlanPath = Resolve-InputFile ([string]$preparationManifest.visual_plan) 'visual plan'
    if ($preparationManifest.PSObject.Properties['visual_plan_sha256']) {
        Assert-Equal (Get-Sha256 $visualPlanPath) ([string]$preparationManifest.visual_plan_sha256) 'visual plan hash'
    }
}

$sourceMap = Get-Content -LiteralPath $sourceMapPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
if ([string]$sourceMap.schema -cne 'babata.mba.c2-source-map/v1') { throw 'Unsupported MBA source-map schema' }
Assert-Equal ([string]$sourceMap.course) $course 'source-map course'
if ([int]$sourceMap.expected_modules -ne $expectedModules) { throw 'Source-map denominator does not match the course plan' }
$sourceItems = @{}
foreach ($item in @($sourceMap.chunks | ForEach-Object { @($_.items) })) {
    $moduleId = [string]$item.module_id
    if ([string]::IsNullOrWhiteSpace($moduleId)) { throw 'Source-map item has no module_id' }
    if ($sourceItems.ContainsKey($moduleId)) { throw "Duplicate source-map module: $moduleId" }
    $sourceItems[$moduleId] = $item
}
if ($sourceItems.Count -ne $expectedModules) { throw "Source-map item count mismatch: $($sourceItems.Count)" }
$sourceMissingFromPlan = @($sourceItems.Keys | Where-Object { -not $chapterByModule.ContainsKey($_) })
$planMissingFromSource = @($chapterByModule.Keys | Where-Object { -not $sourceItems.ContainsKey($_) })
if ($sourceMissingFromPlan.Count -or $planMissingFromSource.Count) {
    throw "Course plan/source-map partition mismatch: source_extra=$($sourceMissingFromPlan -join ',') plan_extra=$($planMissingFromSource -join ',')"
}

$decisions = @(Get-Content -LiteralPath $decisionPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String)
if ($decisions.Count -ne $expectedModules) { throw "C1B decision count mismatch: $($decisions.Count)" }
$decisionIds = @($decisions | ForEach-Object { [string]$_.module_id })
if (@($decisionIds | Sort-Object -Unique).Count -ne $expectedModules) { throw 'C1B decision module IDs are not unique' }
$decisionExtra = @($decisionIds | Where-Object { -not $sourceItems.ContainsKey($_) })
$decisionMissing = @($sourceItems.Keys | Where-Object { $decisionIds -notcontains $_ })
if ($decisionExtra.Count -or $decisionMissing.Count) {
    throw "C1B decision/source-map partition mismatch: decision_extra=$($decisionExtra -join ',') decision_missing=$($decisionMissing -join ',')"
}

# Load all authoritative rows once. This remains read-only, but avoids starting
# sqlite3 hundreds of times for a large course preflight.
$revisionIds = @($sourceItems.Values | ForEach-Object { [string]$_.c0_revision_id } | Sort-Object -Unique)
$revisionIn = New-SqlInList $revisionIds
$c0RowsAll = @(Invoke-SqliteJson $script:rawDb @"
SELECT r.revision_id,r.item_id,r.state AS revision_state,
       a.asset_id,a.sha256,a.state AS asset_state,a.logical_path
FROM revisions r JOIN assets a ON a.revision_id=r.revision_id
WHERE r.revision_id IN ($revisionIn);
"@)
$script:c0Cache = @{}
foreach ($row in $c0RowsAll) {
    $key = "$( [string]$row.revision_id)|$( [string]$row.item_id)|$( [string]$row.asset_id)|$( [string]$row.sha256 )"
    if ($script:c0Cache.ContainsKey($key)) { throw "Duplicate C0 identity row: $key" }
    $script:c0Cache[$key] = $row
}
$c1RowsAll = @(Invoke-SqliteJson $script:derivedDb @"
SELECT p.run_id,p.input_item_id,p.input_revision_id,p.state,p.invalidated_at,
       d.derivative_id,d.output_sha256,d.logical_path,d.kind,d.input_asset_id
FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id
WHERE p.input_revision_id IN ($revisionIn)
  AND p.state='succeeded' AND p.invalidated_at IS NULL;
"@)
$script:c1Cache = @{}
foreach ($row in $c1RowsAll) {
    $key = "$( [string]$row.input_revision_id)|$( [string]$row.output_sha256 )"
    if (-not $script:c1Cache.ContainsKey($key)) { $script:c1Cache[$key] = @() }
    $script:c1Cache[$key] += $row
}
$script:fingerprintRows = @(Invoke-SqliteJson $script:derivedDb @"
SELECT p.run_id,p.pipeline_id,p.input_revision_id,p.input_item_id,p.input_asset_id,
       p.input_sha256,p.target_kind,p.provider,p.tool_or_model,p.tool_version,
       p.params_json,p.loss_notes,p.state,p.invalidated_at,
       d.derivative_id,d.kind,d.output_sha256,d.logical_path,d.media_type,d.language,
       d.input_asset_id AS derivative_input_asset_id
FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id
WHERE p.pipeline_id='$(Escape-Sql $pipelineId)'
  AND p.tool_or_model IN ('$(Escape-Sql $mediaModel)','$(Escape-Sql $decisionModel)')
  AND p.input_revision_id IN ($revisionIn)
  AND p.state='succeeded' AND p.invalidated_at IS NULL;
"@)
$script:fingerprintCacheLoaded = $true

$moduleWork = @()
$mediaPaths = @{}
$mediaFingerprints = @{}
$mediaCount = 0
$preexistingMedia = 0
$preexistingDecisions = 0

foreach ($decision in $decisions) {
    $moduleId = [string]$decision.module_id
    $sourceItem = $sourceItems[$moduleId]
    Assert-Equal ([string]$decision.variant) 'c1b' "module $moduleId variant"
    foreach ($property in @('c0_item_id','c0_revision_id','c0_asset_id','c0_asset_sha256','c1_sha256','title','module_type')) {
        if (-not $decision.PSObject.Properties[$property]) { throw "Module $moduleId decision is missing $property" }
        Assert-Equal ([string]$decision.$property) ([string]$sourceItem.$property) "module $moduleId $property"
    }
    $assetSha = ([string]$decision.c0_asset_sha256).ToLowerInvariant()
    $c1Sha = ([string]$decision.c1_sha256).ToLowerInvariant()
    Assert-Sha256 $assetSha "module $moduleId C0 asset hash"
    Assert-Sha256 $c1Sha "module $moduleId C1 hash"
    if (-not $decision.PSObject.Properties['text_sufficient'] -or
        [string]::IsNullOrWhiteSpace([string]$decision.decision_basis) -or
        -not $decision.PSObject.Properties['retained_modalities'] -or
        -not $decision.PSObject.Properties['audio_decision'] -or
        -not $decision.PSObject.Properties['video_decision'] -or
        -not $decision.PSObject.Properties['attachment_decision']) {
        throw "Module $moduleId has an incomplete essence judgment"
    }

    $itemId = [string]$decision.c0_item_id
    $revisionId = [string]$decision.c0_revision_id
    $assetId = [string]$decision.c0_asset_id
    $c0Key = "$revisionId|$itemId|$assetId|$assetSha"
    $c0Rows = if ($script:c0Cache.ContainsKey($c0Key)) { @($script:c0Cache[$c0Key]) } else { @() }
    if ($c0Rows.Count -ne 1 -or [string]$c0Rows[0].revision_state -cne 'ready' -or [string]$c0Rows[0].asset_state -cne 'ready') {
        throw "C0 identity is not uniquely ready for module $moduleId"
    }
    $c0ManagedPath = Join-Path $script:data ([string]$c0Rows[0].logical_path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $c0ManagedPath -PathType Leaf)) { throw "Managed C0 asset missing for module $moduleId" }
    Assert-Equal (Get-Sha256 $c0ManagedPath) $assetSha "module $moduleId managed C0 hash"

    $c1Key = "$revisionId|$c1Sha"
    $c1Rows = if ($script:c1Cache.ContainsKey($c1Key)) {
        @($script:c1Cache[$c1Key] | Where-Object { [string]$_.output_sha256 -ceq $c1Sha })
    } else { @() }
    $c1Evidence = Select-MbaCourseC1Candidate -Candidates $c1Rows -PreferredKind 'complete C1' -ModuleId $moduleId -PreferredRunId ([string]$sourceItem.c1_run_id)
    Assert-Equal ([string]$c1Evidence.run_id) ([string]$sourceItem.c1_run_id) "module $moduleId C1 run"
    Assert-Equal ([string]$c1Evidence.derivative_id) ([string]$sourceItem.c1_derivative_id) "module $moduleId C1 derivative"
    Assert-Equal ([string]$c1Evidence.input_item_id) $itemId "module $moduleId C1 item"
    [void](Confirm-ManagedRow $c1Evidence $c1Sha "module $moduleId complete C1")

    $textPath = Resolve-RelativePreparationFile $preparationRoot ([string]$decision.c1b_text_path) "module $moduleId C1B text"
    Assert-Equal (Get-Sha256 $textPath) $c1Sha "module $moduleId C1B text hash"

    $mediaWork = @()
    $decisionMedia = @()
    foreach ($media in @($decision.retained_media)) {
        $mediaCount++
        $mediaPath = Resolve-RelativePreparationFile $preparationRoot ([string]$media.path) "module $moduleId retained media"
        $mediaType = Get-MediaType $mediaPath
        $mediaSha = Get-Sha256 $mediaPath
        Assert-Sha256 ([string]$media.sha256).ToLowerInvariant() "module $moduleId retained media hash"
        Assert-Equal $mediaSha ([string]$media.sha256).ToLowerInvariant() "module $moduleId retained media hash"
        if (-not $media.PSObject.Properties['bytes'] -or [long]$media.bytes -ne (Get-Item -LiteralPath $mediaPath).Length) {
            throw "Module $moduleId retained media byte count mismatch: $mediaPath"
        }
        if ([string]::IsNullOrWhiteSpace([string]$media.role) -or
            [string]::IsNullOrWhiteSpace([string]$media.review_reason) -or
            @($media.processing).Count -eq 0 -or -not $media.PSObject.Properties['loss_notes']) {
            throw "Module $moduleId retained media lacks role/review/processing/loss evidence: $mediaPath"
        }
        $relativeKey = ([string]$media.path).Replace('\','/')
        if ($mediaPaths.ContainsKey($relativeKey)) { throw "Duplicate retained media path: $relativeKey" }
        $mediaPaths[$relativeKey] = $true
        $locator = Get-SourceLocator $media
        $mediaParamsObject = [ordered]@{
            schema = 'babata.mba-course-c1b-media-registration/v1'
            adapter = 'deterministic_local'
            course = $course
            course_key = $courseKey
            course_plan_sha256 = $planSha
            preparation_manifest_sha256 = $preparationManifestSha
            module_id = $moduleId
            source_variant = 'c1b'
            source_locator = $locator
            processing = @($media.processing)
            selection_review = [ordered]@{
                role = [string]$media.role
                reason = [string]$media.review_reason
                evidence = 'preparation_decision'
            }
            complete_c1_derivative_id = [string]$c1Evidence.derivative_id
            complete_c1_sha256 = $c1Sha
        }
        $mediaParamsJson = ConvertTo-CompactJson $mediaParamsObject 15
        $mediaLossNotes = (@($media.loss_notes) -join '; ')
        $fingerprintKey = "$revisionId|$assetId|key_frame|$mediaModel|$mediaSha|$mediaParamsJson|$mediaLossNotes"
        if ($mediaFingerprints.ContainsKey($fingerprintKey)) { throw "Duplicate retained media fingerprint for module $moduleId" }
        $mediaFingerprints[$fingerprintKey] = $true
        $existingMedia = @(Find-ExactFingerprint $revisionId $itemId $assetId $assetSha 'key_frame' $mediaModel $mediaSha $mediaParamsJson $mediaLossNotes $mediaType)
        if ($existingMedia.Count -eq 1) {
            [void](Confirm-ManagedRow $existingMedia[0] $mediaSha "module $moduleId retained media")
            $preexistingMedia++
        }
        $mediaWork += [pscustomobject][ordered]@{
            Source = $media
            Path = $mediaPath
            Sha256 = $mediaSha
            MediaType = $mediaType
            Locator = $locator
            ParamsJson = $mediaParamsJson
            LossNotes = $mediaLossNotes
            Existing = $existingMedia
        }
        $decisionMedia += [ordered]@{
            path = $relativeKey
            sha256 = $mediaSha
            bytes = [long]$media.bytes
            media_type = $mediaType
            source_locator = $locator
            role = [string]$media.role
            review_reason = [string]$media.review_reason
            processing = @($media.processing)
            loss_notes = @($media.loss_notes)
        }
    }
    if ($mediaWork.Count -gt 0 -and @($decision.retained_modalities) -notcontains 'image') {
        throw "Module $moduleId retains image evidence but retained_modalities omits image"
    }

    $decisionDocument = [ordered]@{
        schema = 'babata.mba-course-c1b-essence/v1'
        course = $course
        course_key = $courseKey
        course_plan_sha256 = $planSha
        module_id = $moduleId
        title = [string]$decision.title
        module_type = [string]$decision.module_type
        chapter = [string]$chapterByModule[$moduleId]
        source = [ordered]@{
            item_id = $itemId
            revision_id = $revisionId
            asset_id = $assetId
            asset_sha256 = $assetSha
        }
        complete_c1 = [ordered]@{
            run_id = [string]$c1Evidence.run_id
            derivative_id = [string]$c1Evidence.derivative_id
            sha256 = $c1Sha
            logical_path = [string]$c1Evidence.logical_path
            reused_without_reprocessing = $true
        }
        essence_judgment = [ordered]@{
            text_sufficient = [bool]$decision.text_sufficient
            retained_modalities = @($decision.retained_modalities)
            decision_basis = [string]$decision.decision_basis
            audio = $decision.audio_decision
            video = $decision.video_decision
            attachment = $decision.attachment_decision
        }
        retained_media = $decisionMedia
        handoff = [ordered]@{
            c2b_may_consume_after_formal_ledger = $true
            external_original_reread_required = $false
        }
    }
    $decisionJson = ConvertTo-CompactJson $decisionDocument 35
    $decisionSha = Get-StringSha256 $decisionJson
    $decisionParamsObject = [ordered]@{
        schema = 'babata.mba-course-c1b-essence-registration/v1'
        adapter = 'deterministic_local'
        course = $course
        course_key = $courseKey
        course_plan_sha256 = $planSha
        preparation_manifest_sha256 = $preparationManifestSha
        module_id = $moduleId
        source_variant = 'c1b'
        complete_c1_derivative_id = [string]$c1Evidence.derivative_id
        retained_media_sha256 = @($decisionMedia | ForEach-Object { $_.sha256 })
    }
    $decisionParamsJson = ConvertTo-CompactJson $decisionParamsObject 12
    $decisionLossNotes = 'C1B retains complete registered C1 references and necessary image excerpts; it does not copy external sovereign originals or replace C2B semantic organization.'
    $existingDecision = @(Find-ExactFingerprint $revisionId $itemId $assetId $assetSha 'structured_result' $decisionModel $decisionSha $decisionParamsJson $decisionLossNotes 'application/json')
    if ($existingDecision.Count -eq 1) {
        [void](Confirm-ManagedRow $existingDecision[0] $decisionSha "module $moduleId essence decision")
        $preexistingDecisions++
    }
    $moduleWork += [pscustomobject][ordered]@{
        ModuleId = $moduleId
        Decision = $decision
        SourceItem = $sourceItem
        ItemId = $itemId
        RevisionId = $revisionId
        AssetId = $assetId
        AssetSha256 = $assetSha
        C1Evidence = $c1Evidence
        C1Sha256 = $c1Sha
        Media = $mediaWork
        DecisionDocument = $decisionDocument
        DecisionJson = $decisionJson
        DecisionSha256 = $decisionSha
        DecisionParamsJson = $decisionParamsJson
        DecisionLossNotes = $decisionLossNotes
        ExistingDecision = $existingDecision
    }
}

if ($mediaCount -ne [int]$preparationManifest.retained_media) {
    throw "Preparation media denominator mismatch: manifest=$($preparationManifest.retained_media), actual=$mediaCount"
}

$partialPath = Join-Path $staging 'partial-registration.json'
$ledgerPath = Join-Path $staging 'c1b-registration-ledger.json'
$manifestPath = Join-Path $staging 'manifest.json'
$reportPath = Join-Path $staging 'REPORT.md'
$existingReceipt = $null
if ($Resume) {
    if (-not (Test-Path -LiteralPath $partialPath -PathType Leaf)) { throw "Resume receipt missing: $partialPath" }
    $existingReceipt = Get-Content -LiteralPath $partialPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    if ([string]$existingReceipt.schema -cne 'babata.mba-course-c1b-registration-partial/v1') { throw 'Unsupported resume receipt schema' }
    foreach ($binding in @(
        @('course_key', $courseKey),
        @('course_plan_sha256', $planSha),
        @('preparation_manifest_sha256', $preparationManifestSha),
        @('source_map_sha256', $sourceMapSha),
        @('decision_source_sha256', $decisionSourceSha)
    )) {
        Assert-Equal ([string]$existingReceipt.($binding[0])) ([string]$binding[1]) "resume $($binding[0])"
    }
    if ([string]$existingReceipt.status -ceq 'registered') {
        if (-not (Test-Path -LiteralPath $ledgerPath -PathType Leaf) -or -not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
            throw 'Registered resume receipt is missing its final ledger or manifest'
        }
        $finalManifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
        Assert-Equal (Get-Sha256 $ledgerPath) ([string]$finalManifest.ledger_sha256) 'registered resume ledger hash'
        Write-Output "ledger=$ledgerPath status=registered decisions=$expectedModules media=$mediaCount resumed=already_complete"
        return
    }
}

$preflight = [ordered]@{
    schema = 'babata.mba-course-c1b-registration-preflight/v1'
    status = 'passed'
    course = $course
    course_key = $courseKey
    course_plan = $planPath
    course_plan_sha256 = $planSha
    preparation_manifest = $preparationManifestPathResolved
    preparation_manifest_sha256 = $preparationManifestSha
    expected_modules = $expectedModules
    decisions = $moduleWork.Count
    retained_media = $mediaCount
    preexisting_decisions = $preexistingDecisions
    preexisting_media = $preexistingMedia
    new_decisions = $expectedModules - $preexistingDecisions
    new_media = $mediaCount - $preexistingMedia
}
if ($PreflightOnly) {
    $preflight | ConvertTo-Json -Depth 8
    return
}

# PRECHECK_COMPLETE: no staging, Rust CLI, or database mutation occurs above this line.
# MUTATION_START
$mutexIdentity = Get-StringSha256 "$script:data|$courseKey"
$registrationMutex = [Threading.Mutex]::new($false, "Babata.MbaCourseC1b.$mutexIdentity")
$mutexHeld = $false
try {
try {
    $mutexHeld = $registrationMutex.WaitOne(0)
} catch [Threading.AbandonedMutexException] {
    $mutexHeld = $true
}
if (-not $mutexHeld) { throw "Another C1B registrar is active for course $courseKey" }

New-Item -ItemType Directory -Path $staging -Force | Out-Null
$resultRoot = Join-Path $staging 'results'
New-Item -ItemType Directory -Path $resultRoot -Force | Out-Null

$startedAt = if ($null -ne $existingReceipt) {
    Require-Rfc3339Utc $existingReceipt.started_at 'resume started_at'
} else {
    [DateTime]::UtcNow.ToString('o', [Globalization.CultureInfo]::InvariantCulture)
}
$receipt = [ordered]@{
    schema = 'babata.mba-course-c1b-registration-partial/v1'
    status = 'in_progress'
    course = $course
    course_key = $courseKey
    course_plan = $planPath
    course_plan_sha256 = $planSha
    preparation_manifest = $preparationManifestPathResolved
    preparation_manifest_sha256 = $preparationManifestSha
    source_map_sha256 = $sourceMapSha
    decision_source_sha256 = $decisionSourceSha
    expected_modules = $expectedModules
    expected_media = $mediaCount
    started_at = $startedAt
    updated_at = (Get-Date).ToUniversalTime().ToString('o')
    modules = if ($null -ne $existingReceipt) { @($existingReceipt.modules) } else { @() }
}
Write-JsonAtomic $partialPath $receipt 30

$ledgerRows = @()
$registeredMedia = 0
$reusedMedia = 0
$registeredDecisions = 0
$reusedDecisions = 0

foreach ($work in $moduleWork) {
    $moduleId = [string]$work.ModuleId
    try {
        $mediaRegistrations = @()
        foreach ($mediaWorkItem in @($work.Media)) {
            $existing = @(Find-LiveExactFingerprint $work.RevisionId $work.ItemId $work.AssetId $work.AssetSha256 'key_frame' $mediaModel $mediaWorkItem.Sha256 $mediaWorkItem.ParamsJson $mediaWorkItem.LossNotes $mediaWorkItem.MediaType)
            if ($existing.Count -eq 1) {
                $runId = [string]$existing[0].run_id
                $derivativeId = [string]$existing[0].derivative_id
                $registrationState = 'reused'
                $reusedMedia++
            } else {
                $registered = Invoke-BabataJson @(
                    'process','register','--pipeline',$pipelineId,
                    '--revision',$work.RevisionId,'--item',$work.ItemId,
                    '--kind','key_frame','--provider',$provider,
                    '--model',$mediaModel,'--tool-version',$toolVersion,
                    '--input-sha256',$work.AssetSha256,'--input-asset-id',$work.AssetId,
                    '--output-file',$mediaWorkItem.Path,'--media-type',$mediaWorkItem.MediaType,
                    '--language','zh','--params-json',$mediaWorkItem.ParamsJson,
                    '--loss-notes',$mediaWorkItem.LossNotes
                )
                $runId = [string]$registered.run_id
                $derivativeId = [string]$registered.derivative_id
                if ([string]::IsNullOrWhiteSpace($runId) -or [string]::IsNullOrWhiteSpace($derivativeId)) {
                    throw "Media registration returned no run/derivative ID for module $moduleId"
                }
                $registrationState = 'registered'
                $registeredMedia++
            }
            $confirmed = Confirm-RegisteredRun $runId $derivativeId $work.RevisionId $work.ItemId $work.AssetId $work.AssetSha256 'key_frame' $mediaModel $mediaWorkItem.Sha256 $mediaWorkItem.ParamsJson $mediaWorkItem.LossNotes $mediaWorkItem.MediaType
            $mediaRegistrations += [ordered]@{
                output_sha256 = $confirmed.output_sha256
                source_locator = $mediaWorkItem.Locator
                role = [string]$mediaWorkItem.Source.role
                review_reason = [string]$mediaWorkItem.Source.review_reason
                processing = @($mediaWorkItem.Source.processing)
                loss_notes = @($mediaWorkItem.Source.loss_notes)
                run_id = $confirmed.run_id
                derivative_id = $confirmed.derivative_id
                logical_path = $confirmed.logical_path
                registration = $registrationState
            }
        }

        $moduleRoot = Join-Path $resultRoot "module-$moduleId"
        New-Item -ItemType Directory -Path $moduleRoot -Force | Out-Null
        $decisionFile = Join-Path $moduleRoot 'c1b-essence.json'
        Write-Utf8File $decisionFile $work.DecisionJson
        Assert-Equal (Get-Sha256 $decisionFile) $work.DecisionSha256 "module $moduleId decision candidate hash"

        $decisionExisting = @(Find-LiveExactFingerprint $work.RevisionId $work.ItemId $work.AssetId $work.AssetSha256 'structured_result' $decisionModel $work.DecisionSha256 $work.DecisionParamsJson $work.DecisionLossNotes 'application/json')
        if ($decisionExisting.Count -eq 1) {
            $decisionRunId = [string]$decisionExisting[0].run_id
            $decisionDerivativeId = [string]$decisionExisting[0].derivative_id
            $decisionRegistrationState = 'reused'
            $reusedDecisions++
        } else {
            $registeredDecision = Invoke-BabataJson @(
                'process','register','--pipeline',$pipelineId,
                '--revision',$work.RevisionId,'--item',$work.ItemId,
                '--kind','structured_result','--provider',$provider,
                '--model',$decisionModel,'--tool-version',$toolVersion,
                '--input-sha256',$work.AssetSha256,'--input-asset-id',$work.AssetId,
                '--json-file',$decisionFile,'--output-file',$decisionFile,
                '--media-type','application/json','--language','zh',
                '--params-json',$work.DecisionParamsJson,
                '--loss-notes',$work.DecisionLossNotes
            )
            $decisionRunId = [string]$registeredDecision.run_id
            $decisionDerivativeId = [string]$registeredDecision.derivative_id
            if ([string]::IsNullOrWhiteSpace($decisionRunId) -or [string]::IsNullOrWhiteSpace($decisionDerivativeId)) {
                throw "Decision registration returned no run/derivative ID for module $moduleId"
            }
            $decisionRegistrationState = 'registered'
            $registeredDecisions++
        }
        $confirmedDecision = Confirm-RegisteredRun $decisionRunId $decisionDerivativeId $work.RevisionId $work.ItemId $work.AssetId $work.AssetSha256 'structured_result' $decisionModel $work.DecisionSha256 $work.DecisionParamsJson $work.DecisionLossNotes 'application/json'

        $ledgerRow = [ordered]@{
            module_id = $moduleId
            title = [string]$work.Decision.title
            module_type = [string]$work.Decision.module_type
            chapter = [string]$chapterByModule[$moduleId]
            source_item_id = $work.ItemId
            source_revision_id = $work.RevisionId
            source_asset_id = $work.AssetId
            source_asset_sha256 = $work.AssetSha256
            complete_c1 = [ordered]@{
                run_id = [string]$work.C1Evidence.run_id
                derivative_id = [string]$work.C1Evidence.derivative_id
                output_sha256 = $work.C1Sha256
                logical_path = [string]$work.C1Evidence.logical_path
            }
            decision_registration = [ordered]@{
                run_id = $confirmedDecision.run_id
                derivative_id = $confirmedDecision.derivative_id
                output_sha256 = $confirmedDecision.output_sha256
                logical_path = $confirmedDecision.logical_path
                registration = $decisionRegistrationState
            }
            media_registrations = $mediaRegistrations
        }
        $ledgerRows += $ledgerRow
        $receipt.modules = @($receipt.modules | Where-Object { [string]$_.module_id -cne $moduleId }) + @([ordered]@{
            module_id = $moduleId
            status = 'registered'
            decision_run_id = $confirmedDecision.run_id
            decision_derivative_id = $confirmedDecision.derivative_id
            media_run_ids = @($mediaRegistrations | ForEach-Object { $_.run_id })
            media_derivative_ids = @($mediaRegistrations | ForEach-Object { $_.derivative_id })
            updated_at = (Get-Date).ToUniversalTime().ToString('o')
        })
        $receipt.updated_at = (Get-Date).ToUniversalTime().ToString('o')
        Write-JsonAtomic $partialPath $receipt 30
    } catch {
        $receipt.status = 'partial'
        $receipt.modules = @($receipt.modules | Where-Object { [string]$_.module_id -cne $moduleId }) + @([ordered]@{
            module_id = $moduleId
            status = 'failed'
            error = $_.Exception.Message
            updated_at = (Get-Date).ToUniversalTime().ToString('o')
        })
        $receipt.updated_at = (Get-Date).ToUniversalTime().ToString('o')
        Write-JsonAtomic $partialPath $receipt 30
        throw
    }
}

$allMedia = @($ledgerRows | ForEach-Object { @($_.media_registrations) })
if ($ledgerRows.Count -ne $expectedModules -or $allMedia.Count -ne $mediaCount) {
    throw "C1B registration coverage mismatch: decisions=$($ledgerRows.Count)/$expectedModules media=$($allMedia.Count)/$mediaCount"
}
if (@($ledgerRows.decision_registration.derivative_id | Sort-Object -Unique).Count -ne $expectedModules) {
    throw 'C1B decision derivative identities are not unique'
}
if ($mediaCount -gt 0 -and @($allMedia.derivative_id | Sort-Object -Unique).Count -ne $mediaCount) {
    throw 'C1B media derivative identities are not unique'
}

$ledger = [ordered]@{
    schema = 'babata.mba-course-c1b-registration/v1'
    course = $course
    course_key = $courseKey
    short_name = $shortName
    generated_at = $startedAt
    status = 'registered'
    course_plan = $planPath
    course_plan_sha256 = $planSha
    preparation_manifest = $preparationManifestPathResolved
    preparation_manifest_sha256 = $preparationManifestSha
    source_map = $sourceMapPath
    source_map_sha256 = $sourceMapSha
    decision_source = $decisionPath
    decision_source_sha256 = $decisionSourceSha
    coverage = [ordered]@{
        modules = $expectedModules
        complete_c1_reused = $expectedModules
        essence_decisions_registered = $ledgerRows.Count
        retained_media_registered = $allMedia.Count
    }
    registrations = $ledgerRows
}
Write-JsonAtomic $ledgerPath $ledger 40
$ledgerSha = Get-Sha256 $ledgerPath

$finalManifest = [ordered]@{
    schema = 'babata.mba-course-c1b-registration-manifest/v1'
    task = Split-Path $staging -Leaf
    status = 'registered'
    course = $course
    course_key = $courseKey
    course_plan_sha256 = $planSha
    preparation_manifest_sha256 = $preparationManifestSha
    ledger = $ledgerPath
    ledger_sha256 = $ledgerSha
    decisions = $expectedModules
    media = $mediaCount
    registered_decisions = $registeredDecisions
    reused_decisions = $reusedDecisions
    registered_media = $registeredMedia
    reused_media = $reusedMedia
    c0_mutations = 0
    complete_c1_reprocessed = 0
}
Write-JsonAtomic $manifestPath $finalManifest 12

$report = @(
    "# $shortName C1B formal registration report",
    '',
    '- status: registered',
    "- scope: $expectedModules/$expectedModules modules",
    "- complete C1: $expectedModules/$expectedModules managed derivatives reused without reprocessing",
    "- essence decisions: $expectedModules/$expectedModules structured_result derivatives",
    "- necessary image excerpts: $mediaCount/$mediaCount key_frame derivatives",
    "- newly registered: decisions $registeredDecisions, media $registeredMedia",
    "- idempotently reused: decisions $reusedDecisions, media $reusedMedia",
    '- C0 mutations: 0',
    '- external sovereign original reads: 0',
    "- course plan SHA-256: $planSha",
    "- formal ledger: $ledgerPath",
    "- ledger SHA-256: $ledgerSha"
) -join "`n"
Write-Utf8File $reportPath $report

$receipt.status = 'registered'
$receipt.updated_at = (Get-Date).ToUniversalTime().ToString('o')
$receipt.ledger = $ledgerPath
$receipt.ledger_sha256 = $ledgerSha
$receipt.manifest = $manifestPath
Write-JsonAtomic $partialPath $receipt 30

Write-Output "ledger=$ledgerPath status=registered decisions=$expectedModules media=$mediaCount registered_decisions=$registeredDecisions reused_decisions=$reusedDecisions registered_media=$registeredMedia reused_media=$reusedMedia"
} finally {
    if ($mutexHeld) { [void]$registrationMutex.ReleaseMutex() }
    $registrationMutex.Dispose()
}
