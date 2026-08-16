[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CoursePlanPath,
    [Parameter(Mandatory = $true)][string]$C1BRegistrationLedgerPath,
    [Parameter(Mandatory = $true)][string]$StagingRoot,
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe'),
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [string]$SqliteExe = 'sqlite3',
    [switch]$AllowLegacyGeneratedAt,
    [switch]$PreflightOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Native stdout is UTF-8 (sqlite3 JSON and babata --json). Hidden/non-console
# runners can otherwise inherit an OEM console code page and corrupt Chinese
# strings before ConvertFrom-Json sees them.
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$OutputEncoding = $utf8NoBom
[Console]::OutputEncoding = $utf8NoBom

if ([string]::IsNullOrWhiteSpace($DataHome)) {
    throw 'BABATA_DATA_HOME or -DataHome is required'
}

$dataRoot = [IO.Path]::GetFullPath($DataHome).TrimEnd('\')
$env:BABATA_DATA_HOME = $dataRoot
foreach ($inputFile in @($CoursePlanPath, $C1BRegistrationLedgerPath, $BabataExe)) {
    if (-not (Test-Path -LiteralPath $inputFile -PathType Leaf)) { throw "Required input file is missing: $inputFile" }
}
$planPath = (Get-Item -LiteralPath $CoursePlanPath).FullName
$ledgerPath = (Get-Item -LiteralPath $C1BRegistrationLedgerPath).FullName
$exe = (Get-Item -LiteralPath $BabataExe).FullName
$staging = [IO.Path]::GetFullPath($StagingRoot).TrimEnd('\')
if (Test-Path -LiteralPath $staging) {
    throw "Use a fresh C2B knowledge staging root: $staging"
}

$derivedDb = Join-Path $dataRoot '02_derived\index\derived.sqlite'
$rawDb = Join-Path $dataRoot '01_raw\index\raw.sqlite'
foreach ($path in @($derivedDb, $rawDb)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Babata database: $path"
    }
}

New-Item -ItemType Directory -Path $staging -Force | Out-Null
$manifestPath = Join-Path $staging 'knowledge-universe-registration.json'
$state = [ordered]@{
    schema = 'babata.mba-course-c2b-knowledge-registration/v1'
    status = 'preflight'
    course_acceptance = 'pending_user_acceptance'
    legacy_generated_at_compatibility = [bool]$AllowLegacyGeneratedAt
    course = $null
    plan_sha256 = $null
    c1b_ledger_sha256 = $null
    source_map_sha256 = $null
    decision_source_sha256 = $null
    expected_modules = $null
    branch = $null
    modules = @()
    failure = $null
}
$packageRows = @()

function Hash-File([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-Hash([string]$Actual, [string]$Expected, [string]$Label) {
    if ($Actual -notmatch '^[0-9a-fA-F]{64}$' -or $Actual.ToLowerInvariant() -ne $Expected.ToLowerInvariant()) {
        throw "$Label hash mismatch: expected=$Expected actual=$Actual"
    }
}

function Escape-Sql([string]$Value) {
    return $Value.Replace("'", "''")
}

function Invoke-SqliteJson([string]$Database, [string]$Sql) {
    $output = & $SqliteExe '-readonly' '-json' $Database $Sql 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "SQLite read failed: $Sql`n$($output -join [Environment]::NewLine)"
    }
    $text = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text)) { return @() }
    try {
        return @($text | ConvertFrom-Json -DateKind String)
    } catch {
        throw "SQLite JSON read was invalid: $Sql`n$text"
    }
}

function Invoke-BabataJson([string[]]$Arguments) {
    $output = & $exe '--json' @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "babata command failed: $($Arguments -join ' ')`n$($output -join [Environment]::NewLine)"
    }
    $text = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text)) { throw "babata command returned no JSON: $($Arguments -join ' ')" }
    try { return ($text | ConvertFrom-Json -DateKind String) } catch { throw "babata command returned invalid JSON: $($Arguments -join ' ')`n$text" }
}

function Resolve-ManagedLogicalPath([string]$LogicalPath) {
    if ([string]::IsNullOrWhiteSpace($LogicalPath) -or [IO.Path]::IsPathRooted($LogicalPath)) {
        throw "Derivative logical_path must be a relative managed path: $LogicalPath"
    }
    $candidate = [IO.Path]::GetFullPath((Join-Path $dataRoot ($LogicalPath.Replace('/', '\'))))
    $prefix = $dataRoot + '\'
    if (-not $candidate.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Derivative logical_path escapes BABATA_DATA_HOME: $LogicalPath"
    }
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Missing managed derivative file: $LogicalPath"
    }
    return $candidate
}

function Require-Text([object]$Value, [string]$Label) {
    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) { throw "$Label is required" }
    return $text
}

function Require-Rfc3339Utc([object]$Value, [string]$Label) {
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

function Resolve-PackageGeneratedAt([object]$Value, [string]$Label) {
    $text = [string]$Value
    try { return Require-Rfc3339Utc $text $Label } catch {
        if (-not $AllowLegacyGeneratedAt) { throw }
    }
    $parsed = [DateTimeOffset]::MinValue
    if (-not [DateTimeOffset]::TryParseExact(
        $text,
        'MM/dd/yyyy HH:mm:ss',
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AssumeUniversal -bor [Globalization.DateTimeStyles]::AdjustToUniversal,
        [ref]$parsed
    )) {
        throw "$Label is neither RFC3339 UTC nor the authorized legacy UTC format: $text"
    }
    return $text
}

function Require-SafeId([string]$Value, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Value) -or $Value -notmatch '^[A-Za-z0-9_-]+$') {
        throw "$Label must be a stable safe identifier: $Value"
    }
    return $Value
}

function First-Statement([string]$Text, [string]$Fallback) {
    $body = [regex]::Replace($Text, '(?m)^#{1,6}\s+.*$', '').Trim()
    $parts = @($body -split '(?<=[。！？])\s+' | Where-Object { $_.Trim().Length -ge 12 })
    $value = if ($parts.Count) { $parts[0].Trim() } else { $Fallback }
    if ($value.Length -gt 500) { $value = $value.Substring(0, 500) }
    return $value
}

function Outline([string]$Text, [string]$Title) {
    $headings = @([regex]::Matches($Text, '(?m)^#{1,4}\s+(.+)$') | ForEach-Object { $_.Groups[1].Value.Trim() } | Select-Object -Unique -First 12)
    if (-not $headings.Count) { $headings = @($Title) }
    return (($headings | ForEach-Object { "- $_" }) -join "`n")
}

function Write-State([string]$Status, [string]$Failure = $null) {
    $state.status = $Status
    $state.failure = $Failure
    $temporary = "$manifestPath.tmp"
    $state | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $temporary -Encoding utf8
    [IO.File]::Move($temporary, $manifestPath, $true)
}

function Get-SourceMapItems([object]$SourceMap) {
    $result = @()
    foreach ($chunk in @($SourceMap.chunks)) {
        foreach ($item in @($chunk.items)) { $result += $item }
    }
    if (-not $result.Count -and $SourceMap.chunks.items) {
        $result = @($SourceMap.chunks.items)
    }
    return @($result)
}

function Get-MapNode([string]$MapNodeId) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT map_node_id,node_level,canonical_key,name,lifecycle_state
FROM knowledge_map_nodes
WHERE map_node_id='$(Escape-Sql $MapNodeId)';
"@)
}

function Get-ActiveMapNodesByName([string]$Level, [string]$Name) {
    $rows = @(Invoke-SqliteJson $rawDb @"
SELECT map_node_id,node_level,canonical_key,name,lifecycle_state
FROM knowledge_map_nodes
WHERE node_level='$(Escape-Sql $Level)'
  AND lifecycle_state='active'
ORDER BY map_node_id;
"@)
    return @($rows | Where-Object { [string]$_.name -ceq $Name })
}

function Get-ParentRows([string]$MapNodeId) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT map_version_id,parent_node_id,child_node_id
FROM knowledge_map_edges
WHERE child_node_id='$(Escape-Sql $MapNodeId)'
ORDER BY map_version_id,parent_node_id;
"@)
}

function Assert-ExactParent([string]$MapNodeId, [string]$ExpectedParentId, [string]$Label) {
    $parents = @(Get-ParentRows $MapNodeId)
    if ($parents.Count -ne 1 -or [string]$parents[0].parent_node_id -cne $ExpectedParentId) {
        $actual = @($parents | ForEach-Object { [string]$_.parent_node_id }) -join ','
        throw "$Label parent set must be exactly [$ExpectedParentId], actual=[$actual]"
    }
}

function Get-AssignmentRows([string]$SemanticId) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT semantic_id,map_node_id,provenance_kind,suggestion_id
FROM semantic_map_assignments
WHERE semantic_id='$(Escape-Sql $SemanticId)'
ORDER BY map_node_id,provenance_kind;
"@)
}

function Get-Reviews([string]$SuggestionId) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT review_id,decision,reason,reviewer,created_at
FROM suggestion_reviews
WHERE suggestion_id='$(Escape-Sql $SuggestionId)'
ORDER BY created_at,review_id;
"@)
}

function Get-ProcessFingerprint([string]$RevisionId, [string]$ItemId, [string]$PackageHash) {
    return @(Invoke-SqliteJson $derivedDb @"
SELECT d.derivative_id,d.run_id,d.output_sha256,d.logical_path,
       p.input_revision_id,p.input_item_id,p.state,p.invalidated_at,
       p.pipeline_id,p.provider,p.tool_or_model,p.tool_version
FROM derivatives d
JOIN process_runs p ON p.run_id=d.run_id
WHERE d.kind='structured_result'
  AND d.output_sha256='$(Escape-Sql $PackageHash)'
  AND p.input_revision_id='$(Escape-Sql $RevisionId)'
  AND p.input_item_id='$(Escape-Sql $ItemId)'
  AND p.pipeline_id='agent_import'
  AND p.provider='deterministic_local'
  AND p.tool_or_model='c1b-full-text-semantic-materializer'
  AND p.state='succeeded';
"@)
}

function Get-SuggestionFingerprint([string]$SourceDerivativeId, [string]$PackageHash) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT suggestion_id,source_item_id,source_revision_id,source_derivative_id,source_output_sha256
FROM model_suggestions
WHERE source_derivative_id='$(Escape-Sql $SourceDerivativeId)'
  AND source_output_sha256='$(Escape-Sql $PackageHash)';
"@)
}

function Get-RevisionSemanticRows([string]$RevisionId) {
    return @(Invoke-SqliteJson $rawDb @"
SELECT semantic.semantic_id,semantic.suggestion_id,suggestion.source_derivative_id,
       suggestion.source_output_sha256
FROM semantic_entries semantic
JOIN model_suggestions suggestion ON suggestion.suggestion_id=semantic.suggestion_id
WHERE semantic.source_revision_id='$(Escape-Sql $RevisionId)'
  AND semantic.author='c1b-full-text-semantic-materializer';
"@)
}

function Get-C1Readback([string]$DerivativeId, [string]$ExpectedHash, [string]$ExpectedLogicalPath, [string]$RevisionId, [string]$ItemId) {
    $rows = @(Invoke-SqliteJson $derivedDb @"
SELECT d.derivative_id,d.output_sha256,d.logical_path,d.kind,
       p.run_id,p.input_revision_id,p.input_item_id,p.state,p.invalidated_at
FROM derivatives d
JOIN process_runs p ON p.run_id=d.run_id
WHERE d.derivative_id='$(Escape-Sql $DerivativeId)';
"@)
    if ($rows.Count -ne 1) { throw "C1B ledger derivative must have exactly one read-back row: $DerivativeId ($($rows.Count))" }
    $row = $rows[0]
    if ([string]$row.output_sha256 -ne $ExpectedHash -or [string]$row.logical_path -ne $ExpectedLogicalPath -or
        [string]$row.input_revision_id -ne $RevisionId -or [string]$row.input_item_id -ne $ItemId -or
        [string]$row.state -ne 'succeeded' -or $null -ne $row.invalidated_at) {
        throw "C1B ledger derivative read-back mismatch: $DerivativeId"
    }
    return $row
}

try {
    $plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    $ledger = Get-Content -LiteralPath $ledgerPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    $state.course = Require-Text $plan.course 'course'
    $state.plan_sha256 = Hash-File $planPath
    $state.c1b_ledger_sha256 = Hash-File $ledgerPath

    if ([string]$plan.schema -ne 'babata.mba-course-c2b-plan/v1') { throw "Unsupported course plan schema: $($plan.schema)" }
    if ([string]$ledger.schema -ne 'babata.mba-course-c1b-registration/v1') { throw "Unsupported C1B ledger schema: $($ledger.schema)" }
    if ([string]$ledger.status -cne 'registered') { throw "C1B ledger must be registered, not $($ledger.status)" }
    if ([string]$ledger.course -cne [string]$plan.course) { throw 'Course plan and C1B ledger course mismatch' }
    if ([string]$ledger.course_key -cne [string]$plan.course_key) { throw 'Course plan and C1B ledger course_key mismatch' }
    if ([string]$ledger.short_name -cne [string]$plan.short_name) { throw 'Course plan and C1B ledger short_name mismatch' }
    if ([string]$plan.output_status -cne 'pending_user_acceptance') { throw 'Course plan must remain pending_user_acceptance before user review' }
    $generatedAt = Resolve-PackageGeneratedAt $ledger.generated_at 'C1B ledger generated_at'
    Require-Text $plan.short_name 'course short_name' | Out-Null
    Require-SafeId ([string]$plan.course_key) 'course_key' | Out-Null
    $declaredPlanValue = Require-Text $ledger.course_plan 'C1B ledger course_plan'
    if (-not (Test-Path -LiteralPath $declaredPlanValue -PathType Leaf)) { throw "C1B ledger course_plan is missing: $declaredPlanValue" }
    $declaredPlanPath = (Get-Item -LiteralPath $declaredPlanValue).FullName
    if (-not $declaredPlanPath.Equals($planPath, [StringComparison]::OrdinalIgnoreCase)) {
        throw "C1B ledger course_plan path mismatch: $declaredPlanPath != $planPath"
    }
    Assert-Hash ([string]$ledger.course_plan_sha256) $state.plan_sha256 'C1B ledger course_plan'
    $expected = [int]$plan.expected_modules
    if ($expected -lt 1) { throw 'Course plan expected_modules must be positive' }
    $state.expected_modules = $expected

    $chapterByModule = @{}
    $chapterIds = @{}
    $chapterNotes = @{}
    foreach ($chapter in @($plan.chapters)) {
        $chapterId = Require-SafeId ([string]$chapter.id) 'chapter.id'
        $chapterNote = Require-Text $chapter.note 'chapter.note'
        Require-Text $chapter.title 'chapter.title' | Out-Null
        if ($chapterNote -match '[\\/]') { throw "chapter.note must not contain path separators: $chapterNote" }
        if ($chapterIds.ContainsKey($chapterId) -or $chapterNotes.ContainsKey($chapterNote)) { throw "Duplicate chapter identity: $chapterId / $chapterNote" }
        $chapterIds[$chapterId] = $true
        $chapterNotes[$chapterNote] = $true
        foreach ($module in @($chapter.modules)) {
            $moduleId = Require-SafeId ([string]$module) 'chapter module_id'
            if ($chapterByModule.ContainsKey($moduleId)) { throw "Duplicate chapter assignment: $moduleId" }
            $chapterByModule[$moduleId] = [ordered]@{ id=$chapterId; note=$chapterNote; title=[string]$chapter.title }
        }
    }
    if ($chapterByModule.Count -ne $expected) { throw "Chapter plan must assign exactly $expected modules: $($chapterByModule.Count)" }

    $universe = $plan.knowledge_universe
    $foundationId = Require-SafeId ([string]$universe.foundation_id) 'knowledge_universe.foundation_id'
    $foundationName = Require-Text $universe.foundation_name 'knowledge_universe.foundation_name'
    $disciplineId = Require-SafeId ([string]$universe.discipline_id) 'knowledge_universe.discipline_id'
    $disciplineName = Require-Text $universe.discipline_name 'knowledge_universe.discipline_name'
    $branchName = Require-Text $universe.branch_name 'knowledge_universe.branch_name'
    if ($branchName -match '[\\/]') { throw 'knowledge_universe.branch_name must not contain path separators' }

    if (-not (Test-Path -LiteralPath ([string]$ledger.source_map) -PathType Leaf)) { throw "C1B ledger source_map is missing: $($ledger.source_map)" }
    $sourceMapPath = (Get-Item -LiteralPath ([string]$ledger.source_map)).FullName
    Assert-Hash (Hash-File $sourceMapPath) ([string]$ledger.source_map_sha256) 'source_map'
    $state.source_map_sha256 = [string]$ledger.source_map_sha256
    $sourceMap = Get-Content -LiteralPath $sourceMapPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    if ([string]$sourceMap.schema -cne 'babata.mba.c2-source-map/v1') { throw "Unsupported source map schema: $($sourceMap.schema)" }
    if ([string]$sourceMap.course -cne [string]$plan.course) { throw 'Course plan and source map course mismatch' }
    if ([int]$sourceMap.expected_modules -ne $expected) { throw 'Course plan and source map expected_modules mismatch' }
    if ($sourceMap.PSObject.Properties['course_key'] -and [string]$sourceMap.course_key -cne [string]$plan.course_key) {
        throw 'Course plan and source map course_key mismatch'
    }
    if ($sourceMap.PSObject.Properties['course_plan_sha256']) {
        Assert-Hash ([string]$sourceMap.course_plan_sha256) $state.plan_sha256 'source map course_plan'
    }
    $sourceItems = @(Get-SourceMapItems $sourceMap)
    if ($sourceItems.Count -ne $expected) { throw "Source map denominator mismatch: $($sourceItems.Count) != $expected" }
    $sourceByModule = @{}
    foreach ($sourceItem in $sourceItems) {
        $moduleId = Require-SafeId ([string]$sourceItem.module_id) 'source map module_id'
        if ($sourceByModule.ContainsKey($moduleId)) { throw "Duplicate source-map module: $moduleId" }
        $sourceByModule[$moduleId] = $sourceItem
        if (-not $chapterByModule.ContainsKey($moduleId)) { throw "Source-map module is absent from plan: $moduleId" }
    }

    if (-not (Test-Path -LiteralPath ([string]$ledger.decision_source) -PathType Leaf)) { throw "C1B ledger decision_source is missing: $($ledger.decision_source)" }
    $decisionSourcePath = (Get-Item -LiteralPath ([string]$ledger.decision_source)).FullName
    Assert-Hash (Hash-File $decisionSourcePath) ([string]$ledger.decision_source_sha256) 'decision_source'
    $state.decision_source_sha256 = [string]$ledger.decision_source_sha256
    $registrations = @($ledger.registrations)
    if ($registrations.Count -ne $expected) { throw "C1B registration denominator mismatch: $($registrations.Count) != $expected" }
    if ($null -eq $ledger.coverage -or [int]$ledger.coverage.modules -ne $expected -or
        [int]$ledger.coverage.complete_c1_reused -ne $expected -or
        [int]$ledger.coverage.essence_decisions_registered -ne $expected) {
        throw "C1B ledger coverage is not complete for the plan denominator"
    }
    $registrationByModule = @{}
    $sourceItemIds = @{}
    $sourceRevisionIds = @{}
    $sourceAssetIds = @{}
    $derivativeIds = @{}
    foreach ($registration in $registrations) {
        $moduleId = Require-SafeId ([string]$registration.module_id) 'C1B module_id'
        if ($registrationByModule.ContainsKey($moduleId)) { throw "Duplicate C1B registration: $moduleId" }
        if (-not $sourceByModule.ContainsKey($moduleId) -or -not $chapterByModule.ContainsKey($moduleId)) { throw "C1B module is outside the authorized plan: $moduleId" }
        $sourceItemId = Require-SafeId ([string]$registration.source_item_id) 'source_item_id'
        $sourceRevisionId = Require-SafeId ([string]$registration.source_revision_id) 'source_revision_id'
        $sourceAssetId = Require-SafeId ([string]$registration.source_asset_id) 'source_asset_id'
        Require-Text $registration.title 'C1B registration title' | Out-Null
        if ($sourceItemIds.ContainsKey($sourceItemId) -or $sourceRevisionIds.ContainsKey($sourceRevisionId) -or $sourceAssetIds.ContainsKey($sourceAssetId)) { throw "Duplicate source identity in C1B ledger: $moduleId" }
        $sourceItemIds[$sourceItemId] = $true
        $sourceRevisionIds[$sourceRevisionId] = $true
        $sourceAssetIds[$sourceAssetId] = $true
        if ([string]$registration.source_asset_sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "Invalid source asset hash: $moduleId" }
        $sourceItem = $sourceByModule[$moduleId]
        foreach ($binding in @(
            @('c0_item_id',$sourceItemId),
            @('c0_revision_id',$sourceRevisionId),
            @('c0_asset_id',$sourceAssetId),
            @('c0_asset_sha256',[string]$registration.source_asset_sha256),
            @('c1_derivative_id',[string]$registration.complete_c1.derivative_id),
            @('c1_sha256',[string]$registration.complete_c1.output_sha256)
        )) {
            $property = [string]$binding[0]
            if ($sourceItem.PSObject.Properties[$property] -and -not [string]::IsNullOrWhiteSpace([string]$sourceItem.$property) -and [string]$sourceItem.$property -ne [string]$binding[1]) {
                throw "Source map and C1B ledger disagree on $property for module $moduleId"
            }
        }
        $registrationByModule[$moduleId] = $registration
        foreach ($row in @($registration, $registration.complete_c1, $registration.decision_registration) + @($registration.media_registrations)) {
            if ($null -ne $row -and $row.PSObject.Properties['registration'] -and [string]$row.registration -in @('rejected','modified')) { throw "C1B ledger contains rejected/modified state: $moduleId" }
        }
        if ([string]$registration.decision_registration.registration -notin @('registered','reused')) { throw "C1B decision registration is not formal: $moduleId" }
        foreach ($derivative in @($registration.complete_c1, $registration.decision_registration)) {
            $derivativeId = Require-SafeId ([string]$derivative.derivative_id) 'C1B derivative_id'
            if ($derivativeIds.ContainsKey($derivativeId)) { throw "Duplicate C1B derivative identity: $derivativeId" }
            $derivativeIds[$derivativeId] = $true
            if ([string]$derivative.output_sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "Invalid C1B derivative hash: $moduleId" }
            $logical = Require-Text $derivative.logical_path 'C1B derivative logical_path'
            $managed = Resolve-ManagedLogicalPath $logical
            Assert-Hash (Hash-File $managed) ([string]$derivative.output_sha256) "C1B derivative $($derivative.derivative_id)"
            [void](Get-C1Readback ([string]$derivative.derivative_id) ([string]$derivative.output_sha256) $logical ([string]$registration.source_revision_id) ([string]$registration.source_item_id))
        }
        $rawIdentity = @(Invoke-SqliteJson $rawDb @"
SELECT i.item_id,r.revision_id,r.state,a.asset_id,a.sha256,a.state AS asset_state
FROM items i JOIN revisions r ON r.item_id=i.item_id
JOIN assets a ON a.revision_id=r.revision_id
WHERE i.item_id='$(Escape-Sql ([string]$registration.source_item_id))'
  AND r.revision_id='$(Escape-Sql ([string]$registration.source_revision_id))'
  AND a.asset_id='$(Escape-Sql ([string]$registration.source_asset_id))';
"@)
        if ($rawIdentity.Count -ne 1 -or [string]$rawIdentity[0].state -ne 'ready' -or [string]$rawIdentity[0].asset_state -ne 'ready' -or [string]$rawIdentity[0].sha256 -ne [string]$registration.source_asset_sha256) {
            throw "C0 identity/asset read-back mismatch: $moduleId"
        }
    }
    if ($registrationByModule.Count -ne $expected) { throw 'C1B registration set does not cover the plan denominator' }

    $foundationRows = @(Get-MapNode $foundationId)
    $disciplineRows = @(Get-MapNode $disciplineId)
    if ($foundationRows.Count -ne 1 -or [string]$foundationRows[0].node_level -cne 'foundation' -or [string]$foundationRows[0].lifecycle_state -cne 'active' -or [string]$foundationRows[0].name -cne $foundationName) { throw "Foundation map node identity mismatch: $foundationId / $foundationName" }
    if ($disciplineRows.Count -ne 1 -or [string]$disciplineRows[0].node_level -cne 'discipline' -or [string]$disciplineRows[0].lifecycle_state -cne 'active' -or [string]$disciplineRows[0].name -cne $disciplineName) { throw "Discipline map node identity mismatch: $disciplineId / $disciplineName" }
    $foundationNameRows = @(Get-ActiveMapNodesByName 'foundation' $foundationName)
    $disciplineNameRows = @(Get-ActiveMapNodesByName 'discipline' $disciplineName)
    if ($foundationNameRows.Count -ne 1 -or [string]$foundationNameRows[0].map_node_id -cne $foundationId) { throw "Foundation name must resolve to exactly the requested active node: $foundationName" }
    if ($disciplineNameRows.Count -ne 1 -or [string]$disciplineNameRows[0].map_node_id -cne $disciplineId) { throw "Discipline name must resolve to exactly the requested active node: $disciplineName" }
    $foundationKey = Require-Text $foundationRows[0].canonical_key 'foundation canonical_key'
    if ($foundationKey -cnotin @('foundation:time','foundation:space','foundation:matter','foundation:consciousness')) { throw "Unsupported foundation canonical_key for semantic package: $foundationKey" }
    Assert-ExactParent $disciplineId $foundationId 'Discipline'

    $branchRows = @(Get-ActiveMapNodesByName 'branch' $branchName)
    if ($branchRows.Count -gt 1) { throw "Duplicate active branch fingerprint: $branchName" }
    $branchNeedsCreate = $branchRows.Count -eq 0
    $branchId = if ($branchNeedsCreate) { $null } else { [string]$branchRows[0].map_node_id }
    if (-not $branchNeedsCreate) { Assert-ExactParent $branchId $disciplineId 'Branch' }
    $branchState = [ordered]@{ id=$branchId; name=$branchName; created=$false; preflight='missing' }
    $state.branch = $branchState

    $model = 'c1b-full-text-semantic-materializer'
    $promptVersion = 'mba-course-c2b-deterministic-v1'
    foreach ($moduleId in ($registrationByModule.Keys | Sort-Object)) {
        $registration = $registrationByModule[$moduleId]
        $chapter = $chapterByModule[$moduleId]
        $bodyPath = Resolve-ManagedLogicalPath ([string]$registration.complete_c1.logical_path)
        $body = Get-Content -LiteralPath $bodyPath -Raw -Encoding utf8
        if ([string]::IsNullOrWhiteSpace($body)) { throw "Complete C1 body is empty: $moduleId" }
        $entryTitle = "$([string]$plan.short_name)｜$([string]$registration.title)"
        $package = [ordered]@{
            schema_version = 'p6-semantic-candidate/v1'
            source_item_id = [string]$registration.source_item_id
            source_revision_id = [string]$registration.source_revision_id
            evidence_derivatives = @(
                [ordered]@{ derivative_id=[string]$registration.complete_c1.derivative_id; output_sha256=[string]$registration.complete_c1.output_sha256 },
                [ordered]@{ derivative_id=[string]$registration.decision_registration.derivative_id; output_sha256=[string]$registration.decision_registration.output_sha256 }
            )
            provider = 'deterministic_local'
            model = $model
            model_version = '2.0.0'
            prompt_version = $promptVersion
            generated_at = $generatedAt
            map_nodes = @()
            entries = @([ordered]@{
                local_ref = "entry:mba-module-$moduleId"
                title = $entryTitle
                payload = [ordered]@{ kind='knowledge'; statement=(First-Statement $body ([string]$registration.title)); details=$body.Trim() }
                map_node_refs = @($foundationKey)
                tags = @('MBA',[string]$plan.course_key,$branchName,[string]$chapter.note,"module-$moduleId")
                dense_expressions = @([ordered]@{ kind='outline'; content=(Outline $body ([string]$registration.title)) })
                relevance = [ordered]@{ interest=50; strategy=50; consensus=50; rationale='课程语义登记使用中性基线；用户验收仍由独立课程验收流程决定。' }
            })
            relations = @()
            limitations = @(
                '该条目严格绑定正式 C1B ledger 的单一 C0 revision；课程级章节关系由课程 package 表达。',
                "course_plan_sha256:$($state.plan_sha256)",
                "c1b_ledger_sha256:$($state.c1b_ledger_sha256)"
            )
        }
        $moduleRoot = Join-Path $staging "module-$moduleId"
        New-Item -ItemType Directory -Path $moduleRoot -Force | Out-Null
        $packagePath = Join-Path $moduleRoot 'semantic-candidate.json'
        $package | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $packagePath -Encoding utf8
        $packageHash = Hash-File $packagePath
        $processRows = @(Get-ProcessFingerprint ([string]$registration.source_revision_id) ([string]$registration.source_item_id) $packageHash)
        if ($processRows.Count -gt 1) { throw "Duplicate structured-result fingerprint: $moduleId" }
        if ($processRows.Count -eq 1 -and $null -ne $processRows[0].invalidated_at) { throw "Structured-result fingerprint is only invalidated, not reusable: $moduleId" }
        $revisionSemanticRows = @(Get-RevisionSemanticRows ([string]$registration.source_revision_id))
        if ($revisionSemanticRows.Count -gt 1) { throw "Duplicate semantic entries for C0 revision: $moduleId" }
        if ($revisionSemanticRows.Count -eq 1 -and [string]$revisionSemanticRows[0].source_output_sha256 -ne $packageHash) { throw "Conflicting semantic fingerprint for C0 revision: $moduleId" }
        $processDerivativeId = if ($processRows.Count -eq 1) { [string]$processRows[0].derivative_id } else { $null }
        $suggestionRows = @(if ($processDerivativeId) { Get-SuggestionFingerprint $processDerivativeId $packageHash })
        if ($suggestionRows.Count -gt 1) { throw "Duplicate suggestion fingerprint: $moduleId" }
        if ($suggestionRows.Count -eq 1 -and ([string]$suggestionRows[0].source_item_id -ne [string]$registration.source_item_id -or [string]$suggestionRows[0].source_revision_id -ne [string]$registration.source_revision_id)) { throw "Suggestion fingerprint source identity mismatch: $moduleId" }
        $suggestionId = if ($suggestionRows.Count -eq 1) { [string]$suggestionRows[0].suggestion_id } else { $null }
        $semanticRows = @(if ($suggestionId) { Invoke-SqliteJson $rawDb "SELECT semantic_id,suggestion_id FROM semantic_entries WHERE suggestion_id='$(Escape-Sql $suggestionId)';" })
        if ($suggestionId -and $semanticRows.Count -ne 1) { throw "Suggestion must have exactly one semantic entry: $moduleId" }
        $semanticId = if ($semanticRows.Count -eq 1) { [string]$semanticRows[0].semantic_id } else { $null }
        $reviews = @(if ($suggestionId) { Get-Reviews $suggestionId })
        if (@($reviews | Where-Object { [string]$_.decision -in @('rejected','modified') }).Count) { throw "Rejected/modified suggestion cannot be reused: $moduleId" }
        if (@($reviews | Where-Object { [string]$_.decision -eq 'accepted' }).Count -gt 1) { throw "Duplicate accepted review: $moduleId" }
        $assignments = @(if ($semanticId) { Get-AssignmentRows $semanticId })
        if ($semanticId) {
            $targetAssignments = @($assignments | Where-Object map_node_id -eq $branchId)
            if ($targetAssignments.Count -gt 1) { throw "Duplicate target branch assignment: $moduleId" }
            $unexpected = @($assignments | Where-Object { $_.map_node_id -ne $branchId -and $_.map_node_id -ne $foundationId })
            if ($unexpected.Count) { throw "Semantic has an unexpected map assignment: $moduleId" }
            $foundationAssignments = @($assignments | Where-Object map_node_id -eq $foundationId)
            if ($foundationAssignments.Count -gt 1) { throw "Duplicate foundation assignment: $moduleId" }
        }
        $packageRows += [ordered]@{
            module_id=$moduleId; chapter=[string]$chapter.note; title=[string]$registration.title
            source_item_id=[string]$registration.source_item_id; source_revision_id=[string]$registration.source_revision_id
            package_path=$packagePath; fingerprint=$packageHash; process_derivative_id=$processDerivativeId
            suggestion_id=$suggestionId; semantic_id=$semanticId; review_state=if($reviews.Count){[string]$reviews[-1].decision}else{'unreviewed'}
            assignment_state=if($semanticId -and @($assignments|Where-Object map_node_id -eq $branchId).Count){'assigned'}else{'pending'}
            status='preflight_passed'
        }
    }
    $state.modules = @($packageRows)
    $branchState.preflight = if ($branchNeedsCreate) { 'missing_create_after_preflight' } else { 'resolved' }
    Write-State 'preflight_passed'
    if ($PreflightOnly) {
        Write-Output "preflight=passed course=$($plan.course) entries=$expected branch=$branchName branch_action=$(if($branchNeedsCreate){'create'}else{'reuse'})"
        return
    }

    if ($branchNeedsCreate) {
        $created = Invoke-BabataJson @('knowledge','create-map-node','--level','branch','--name',$branchName,'--parent',$disciplineId,'--rationale',"MBA course branch for $([string]$plan.course)")
        $createdId = [string]$created.map_node_id
        if ([string]::IsNullOrWhiteSpace($createdId)) { throw 'Branch creation returned no map_node_id' }
        $branchRows = @(Get-ActiveMapNodesByName 'branch' $branchName)
        if ($branchRows.Count -ne 1 -or [string]$branchRows[0].map_node_id -cne $createdId) { throw "Branch creation read-back was not exactly one matching active branch: $branchName" }
        $branchId = $createdId
        Assert-ExactParent $branchId $disciplineId 'Created branch'
        $branchState.id = $branchId
        $branchState.created = $true
        $branchState.preflight = 'created_and_read_back'
    }
    if ([string]::IsNullOrWhiteSpace($branchId)) { throw 'Branch id was not resolved' }

    $completed = 0
    Write-State 'in_progress'
    foreach ($row in @($packageRows)) {
        $registration = $registrationByModule[[string]$row.module_id]
        if ([string]::IsNullOrWhiteSpace([string]$row.process_derivative_id)) {
            $params = [ordered]@{ course=[string]$plan.course; course_key=[string]$plan.course_key; module_id=[string]$row.module_id; c1b_ledger_sha256=$state.c1b_ledger_sha256; package_sha256=[string]$row.fingerprint; branch_id=$branchId } | ConvertTo-Json -Compress
            $registered = Invoke-BabataJson @('process','register','--pipeline','agent_import','--revision',[string]$row.source_revision_id,'--kind','structured_result','--provider','deterministic_local','--model','c1b-full-text-semantic-materializer','--tool-version','2.0.0','--input-sha256',[string]$registration.source_asset_sha256,'--input-asset-id',[string]$registration.source_asset_id,'--json-file',[string]$row.package_path,'--output-file',[string]$row.package_path,'--media-type','application/json','--language','zh','--params-json',$params,'--loss-notes','MBA C2B semantic candidate is bound to the formal C1B ledger; user acceptance remains separate.')
            $row.process_derivative_id = [string]$registered.derivative_id
            if ([string]::IsNullOrWhiteSpace($row.process_derivative_id)) { throw "Process registration returned no derivative: $($row.module_id)" }
        }
        $processRows = @(Get-ProcessFingerprint ([string]$row.source_revision_id) ([string]$row.source_item_id) ([string]$row.fingerprint))
        if ($processRows.Count -ne 1 -or [string]$processRows[0].derivative_id -ne [string]$row.process_derivative_id) { throw "Process registration read-back mismatch: $($row.module_id)" }
        if ([string]::IsNullOrWhiteSpace([string]$row.suggestion_id)) {
            $ingested = Invoke-BabataJson @('knowledge','ingest','--derivative',[string]$row.process_derivative_id)
            $row.suggestion_id = [string]$ingested.suggestion_id
            $row.semantic_id = [string]$ingested.semantic_ids[0]
            if ([string]::IsNullOrWhiteSpace($row.suggestion_id) -or [string]::IsNullOrWhiteSpace($row.semantic_id)) { throw "Knowledge ingest returned incomplete ids: $($row.module_id)" }
        }
        $suggestionRows = @(Get-SuggestionFingerprint ([string]$row.process_derivative_id) ([string]$row.fingerprint))
        if ($suggestionRows.Count -ne 1 -or [string]$suggestionRows[0].suggestion_id -ne [string]$row.suggestion_id) { throw "Suggestion read-back mismatch: $($row.module_id)" }
        if ([string]$suggestionRows[0].source_item_id -ne [string]$row.source_item_id -or [string]$suggestionRows[0].source_revision_id -ne [string]$row.source_revision_id) { throw "Suggestion source identity read-back mismatch: $($row.module_id)" }
        $semanticRows = @(Invoke-SqliteJson $rawDb "SELECT semantic_id,suggestion_id FROM semantic_entries WHERE suggestion_id='$(Escape-Sql ([string]$row.suggestion_id))';")
        if ($semanticRows.Count -ne 1 -or [string]$semanticRows[0].semantic_id -ne [string]$row.semantic_id) { throw "Semantic entry read-back mismatch: $($row.module_id)" }
        $reviews = @(Get-Reviews ([string]$row.suggestion_id))
        if (@($reviews | Where-Object { [string]$_.decision -in @('rejected','modified') }).Count) { throw "Suggestion became rejected/modified: $($row.module_id)" }
        if (@($reviews | Where-Object { [string]$_.decision -eq 'accepted' }).Count -eq 0) {
            [void](Invoke-BabataJson @('knowledge','review-suggestion','--suggestion',[string]$row.suggestion_id,'--decision','accept'))
        }
        $reviews = @(Get-Reviews ([string]$row.suggestion_id))
        if (@($reviews | Where-Object { [string]$_.decision -eq 'accepted' }).Count -ne 1 -or @($reviews | Where-Object { [string]$_.decision -in @('rejected','modified') }).Count) { throw "Suggestion review read-back is not exactly accepted: $($row.module_id)" }
        $assignments = @(Get-AssignmentRows ([string]$row.semantic_id))
        $targetAssignments = @($assignments | Where-Object map_node_id -eq $branchId)
        if ($targetAssignments.Count -eq 0) {
            [void](Invoke-BabataJson @('knowledge','change-map-assignment','--semantic',[string]$row.semantic_id,'--map-node',$branchId,'--change','assign','--rationale',"Formal MBA course assignment for $([string]$plan.course)"))
        }
        $assignments = @(Get-AssignmentRows ([string]$row.semantic_id))
        $targetAssignments = @($assignments | Where-Object map_node_id -eq $branchId)
        if ($targetAssignments.Count -ne 1) { throw "Target branch assignment read-back is not exactly one: $($row.module_id)" }
        $foundationAssignments = @($assignments | Where-Object map_node_id -eq $foundationId)
        if ($foundationAssignments.Count -gt 1) { throw "Duplicate foundation assignment after ingest: $($row.module_id)" }
        if ($foundationAssignments.Count -eq 1) {
            [void](Invoke-BabataJson @('knowledge','change-map-assignment','--semantic',[string]$row.semantic_id,'--map-node',$foundationId,'--change','unassign','--rationale','Replace temporary foundation assignment with the approved course branch.'))
        }
        $assignments = @(Get-AssignmentRows ([string]$row.semantic_id))
        if (@($assignments | Where-Object { $_.map_node_id -ne $branchId }).Count -ne 0) { throw "Unexpected assignment remains after branch placement: $($row.module_id)" }
        $row.review_state = 'accepted'
        $row.assignment_state = 'assigned'
        $row.status = 'registered'
        $completed++
        Write-State 'in_progress'
    }
    if ($completed -ne $expected -or @($packageRows | Where-Object status -ne 'registered').Count) { throw "Registration did not complete the full denominator: $completed/$expected" }
    foreach ($row in @($packageRows)) {
        Assert-Hash (Hash-File ([string]$row.package_path)) ([string]$row.fingerprint) "semantic package $($row.module_id)"
    }
    $finalBranchRows = @(Get-ActiveMapNodesByName 'branch' $branchName)
    if ($finalBranchRows.Count -ne 1 -or [string]$finalBranchRows[0].map_node_id -cne $branchId) { throw 'Final branch read-back no longer matches the registered course branch' }
    Assert-ExactParent $branchId $disciplineId 'Final branch'
    $state.modules = @($packageRows)
    $state.branch = $branchState
    Write-State 'registered'
    Write-Output "ledger=$manifestPath course=$($plan.course) entries=$completed status=registered branch=$branchId"
} catch {
    try {
        if ($manifestPath) {
            $state.modules = @($packageRows)
            Write-State 'blocked' $_.Exception.Message
        }
    } catch { }
    throw
}
