[CmdletBinding()]
param(
    [string]$RepoRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$scriptPath = Join-Path (Resolve-Path -LiteralPath $RepoRoot).Path '05_scripts\register-mba-course-c2b-knowledge.ps1'
if (-not (Test-Path -LiteralPath $scriptPath -PathType Leaf)) { throw 'Generic MBA knowledge registrar is missing' }
$sqliteExe = (Get-Command sqlite3 -ErrorAction Stop).Source

$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count) { throw "Registrar has PowerShell syntax errors: $($errors | Out-String)" }

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Write-Json([string]$Path, [object]$Value) {
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
    $Value | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $Path -Encoding utf8
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Invoke-Sql([string]$Database, [string]$Sql) {
    $output = & $sqliteExe $Database $Sql 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Fixture SQLite failed: $Sql`n$($output -join [Environment]::NewLine)" }
}

$testRoot = Join-Path ([IO.Path]::GetTempPath()) "babata-c2b-registrar-test-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
$fakeBabata = Join-Path $testRoot 'fake-babata.ps1'

@'
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$script:cliArgs = @($args)

function Sql-Quote([string]$Value) { return "'" + $Value.Replace("'", "''") + "'" }
function Sql-Text([string]$Value) {
    $hex = [Convert]::ToHexString([Text.UTF8Encoding]::new($false).GetBytes($Value))
    return "CAST(X'$hex' AS TEXT)"
}
function Arg-Value([string]$Name) {
    $index = [Array]::IndexOf($script:cliArgs, $Name)
    if ($index -lt 0 -or $index + 1 -ge $script:cliArgs.Count) { throw "Missing fake argument: $Name" }
    return [string]$script:cliArgs[$index + 1]
}
function Run-Sql([string]$Database, [string]$Sql) {
    $output = & sqlite3 $Database $Sql 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Fake SQLite failed: $Sql`n$($output -join [Environment]::NewLine)" }
}
function Read-Scalar([string]$Database, [string]$Sql) {
    $output = & sqlite3 -noheader $Database $Sql 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Fake SQLite read failed: $Sql`n$($output -join [Environment]::NewLine)" }
    return ($output -join "`n").Trim()
}
function Emit([object]$Value) {
    $global:LASTEXITCODE = 0
    Write-Output ($Value | ConvertTo-Json -Depth 8 -Compress)
}

$commandKey = "$($script:cliArgs[1]) $($script:cliArgs[2])"
if ($env:BABATA_FAKE_LOG) { Add-Content -LiteralPath $env:BABATA_FAKE_LOG -Value $commandKey -Encoding utf8 }
if ($env:BABATA_FAKE_FAIL_COMMAND -eq $commandKey -and $env:BABATA_FAKE_FAIL_MARKER -and -not (Test-Path -LiteralPath $env:BABATA_FAKE_FAIL_MARKER)) {
    Set-Content -LiteralPath $env:BABATA_FAKE_FAIL_MARKER -Value $commandKey -Encoding utf8
    $global:LASTEXITCODE = 17
    Write-Output '{"error":"injected interruption"}'
    return
}

$rawDb = Join-Path $env:BABATA_DATA_HOME '01_raw\index\raw.sqlite'
$derivedDb = Join-Path $env:BABATA_DATA_HOME '02_derived\index\derived.sqlite'
switch ($commandKey) {
    'knowledge create-map-node' {
        $name = Arg-Value '--name'
        $parent = Arg-Value '--parent'
        Run-Sql $rawDb "INSERT INTO knowledge_map_nodes VALUES('mapnode_supply','branch','node:mapnode_supply',$(Sql-Text $name),'active'); INSERT INTO knowledge_map_edges VALUES('map_version_test',$(Sql-Quote $parent),'mapnode_supply','first_party',NULL,'2026-08-15T00:00:00Z');"
        Emit ([ordered]@{ map_node_id = 'mapnode_supply' })
    }
    'process register' {
        $revision = Arg-Value '--revision'
        $outputFile = Arg-Value '--output-file'
        $hash = (Get-FileHash -LiteralPath $outputFile -Algorithm SHA256).Hash.ToLowerInvariant()
        $item = Read-Scalar $rawDb "SELECT item_id FROM revisions WHERE revision_id=$(Sql-Quote $revision);"
        Run-Sql $derivedDb "INSERT INTO process_runs VALUES('run_semantic',$(Sql-Quote $revision),$(Sql-Quote $item),'succeeded',NULL,'agent_import','deterministic_local','c1b-full-text-semantic-materializer','2.0.0'); INSERT INTO derivatives VALUES('derivative_semantic','run_semantic',$(Sql-Quote $hash),$(Sql-Quote $outputFile),'structured_result');"
        Emit ([ordered]@{ derivative_id = 'derivative_semantic' })
    }
    'knowledge ingest' {
        $derivative = Arg-Value '--derivative'
        $row = Read-Scalar $derivedDb "SELECT p.input_item_id||'|'||p.input_revision_id||'|'||d.output_sha256 FROM derivatives d JOIN process_runs p ON p.run_id=d.run_id WHERE d.derivative_id=$(Sql-Quote $derivative);"
        $parts = @($row -split '\|')
        Run-Sql $rawDb "INSERT INTO model_suggestions VALUES('suggestion_supply',$(Sql-Quote $parts[0]),$(Sql-Quote $parts[1]),$(Sql-Quote $derivative),$(Sql-Quote $parts[2])); INSERT INTO semantic_entries VALUES('semantic_supply','suggestion_supply',$(Sql-Quote $parts[1]),'c1b-full-text-semantic-materializer'); INSERT INTO semantic_map_assignments VALUES('semantic_supply','mapnode_p6_consciousness','model_suggested','suggestion_supply');"
        Emit ([ordered]@{ suggestion_id = 'suggestion_supply'; semantic_ids = @('semantic_supply') })
    }
    'knowledge review-suggestion' {
        Run-Sql $rawDb "INSERT INTO suggestion_reviews VALUES('review_supply','suggestion_supply','accepted','fixture','test','2026-08-15T00:00:01Z');"
        Emit ([ordered]@{ review_id = 'review_supply' })
    }
    'knowledge change-map-assignment' {
        $semantic = Arg-Value '--semantic'
        $mapNode = Arg-Value '--map-node'
        $change = Arg-Value '--change'
        if ($change -eq 'assign') {
            Run-Sql $rawDb "INSERT OR IGNORE INTO semantic_map_assignments VALUES($(Sql-Quote $semantic),$(Sql-Quote $mapNode),'reviewed','suggestion_supply');"
        } else {
            Run-Sql $rawDb "DELETE FROM semantic_map_assignments WHERE semantic_id=$(Sql-Quote $semantic) AND map_node_id=$(Sql-Quote $mapNode);"
        }
        Emit ([ordered]@{ status = $change })
    }
    default { throw "Unexpected fake babata command: $commandKey" }
}
'@ | Set-Content -LiteralPath $fakeBabata -Encoding utf8

function New-Fixture([string]$Name) {
    $root = Join-Path $testRoot $Name
    $dataHome = Join-Path $root 'data'
    $inputRoot = Join-Path $root 'inputs'
    $rawDb = Join-Path $dataHome '01_raw\index\raw.sqlite'
    $derivedDb = Join-Path $dataHome '02_derived\index\derived.sqlite'
    $completePath = Join-Path $dataHome '02_derived\files\complete-c1.md'
    $decisionPath = Join-Path $dataHome '02_derived\files\decision.json'
    foreach ($directory in @((Split-Path $rawDb -Parent), (Split-Path $derivedDb -Parent), (Split-Path $completePath -Parent), $inputRoot)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    Set-Content -LiteralPath $completePath -Value "# Module one`n`nThis complete C1 body is long enough for a deterministic semantic statement。" -Encoding utf8
    Set-Content -LiteralPath $decisionPath -Value '{"decision":"text_sufficient"}' -Encoding utf8
    $completeHash = Get-Sha256 $completePath
    $decisionHash = Get-Sha256 $decisionPath
    $assetHash = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

    Invoke-Sql $rawDb @"
CREATE TABLE knowledge_map_nodes(map_node_id TEXT,node_level TEXT,canonical_key TEXT,name TEXT,lifecycle_state TEXT);
CREATE TABLE knowledge_map_edges(map_version_id TEXT,parent_node_id TEXT,child_node_id TEXT,provenance_kind TEXT,suggestion_id TEXT,created_at TEXT);
CREATE TABLE items(item_id TEXT);
CREATE TABLE revisions(revision_id TEXT,item_id TEXT,state TEXT);
CREATE TABLE assets(asset_id TEXT,revision_id TEXT,sha256 TEXT,state TEXT);
CREATE TABLE model_suggestions(suggestion_id TEXT,source_item_id TEXT,source_revision_id TEXT,source_derivative_id TEXT,source_output_sha256 TEXT);
CREATE TABLE semantic_entries(semantic_id TEXT,suggestion_id TEXT,source_revision_id TEXT,author TEXT);
CREATE TABLE suggestion_reviews(review_id TEXT,suggestion_id TEXT,decision TEXT,reason TEXT,reviewer TEXT,created_at TEXT);
CREATE TABLE semantic_map_assignments(semantic_id TEXT,map_node_id TEXT,provenance_kind TEXT,suggestion_id TEXT,PRIMARY KEY(semantic_id,map_node_id));
INSERT INTO knowledge_map_nodes VALUES('mapnode_p6_consciousness','foundation','foundation:consciousness',CAST(X'E6848FE8AF86' AS TEXT),'active');
INSERT INTO knowledge_map_nodes VALUES('mapnode_management','discipline','node:mapnode_management',CAST(X'E7AEA1E79086E5ADA6' AS TEXT),'active');
INSERT INTO knowledge_map_edges VALUES('map_version_test','mapnode_p6_consciousness','mapnode_management','first_party',NULL,'2026-08-15T00:00:00Z');
INSERT INTO items VALUES('item_m1');
INSERT INTO revisions VALUES('rev_m1','item_m1','ready');
INSERT INTO assets VALUES('asset_m1','rev_m1','$assetHash','ready');
"@
    Invoke-Sql $derivedDb @"
CREATE TABLE process_runs(run_id TEXT,input_revision_id TEXT,input_item_id TEXT,state TEXT,invalidated_at TEXT,pipeline_id TEXT,provider TEXT,tool_or_model TEXT,tool_version TEXT);
CREATE TABLE derivatives(derivative_id TEXT,run_id TEXT,output_sha256 TEXT,logical_path TEXT,kind TEXT);
INSERT INTO process_runs VALUES('run_c1','rev_m1','item_m1','succeeded',NULL,'extract','fixture','fixture','1');
INSERT INTO process_runs VALUES('run_decision','rev_m1','item_m1','succeeded',NULL,'agent_import','fixture','fixture','1');
INSERT INTO derivatives VALUES('derivative_c1','run_c1','$completeHash','02_derived/files/complete-c1.md','cleaned_text');
INSERT INTO derivatives VALUES('derivative_decision','run_decision','$decisionHash','02_derived/files/decision.json','structured_result');
"@

    $planPath = Join-Path $inputRoot 'course-plan.json'
    $plan = [ordered]@{
        schema = 'babata.mba-course-c2b-plan/v1'
        course = 'Fixture MBA course'
        short_name = 'Fixture course'
        course_key = 'fixture-course'
        expected_modules = 1
        output_status = 'pending_user_acceptance'
        chapters = @([ordered]@{ id='01'; note='01-fixture'; title='Fixture'; modules=@('m1') })
        knowledge_universe = [ordered]@{
            foundation_id='mapnode_p6_consciousness'; foundation_name='意识'
            discipline_id='mapnode_management'; discipline_name='管理学'; branch_name='供应链管理'
        }
    }
    Write-Json $planPath $plan
    $planHash = Get-Sha256 $planPath

    $sourceMapPath = Join-Path $inputRoot 'source-map.json'
    $sourceMap = [ordered]@{
        schema='babata.mba.c2-source-map/v1'; course=$plan.course; course_key=$plan.course_key
        course_plan_sha256=$planHash; expected_modules=1
        chunks=@([ordered]@{ chunk_id='fixture'; items=@([ordered]@{
            module_id='m1'; c0_item_id='item_m1'; c0_revision_id='rev_m1'; c0_asset_id='asset_m1'; c0_asset_sha256=$assetHash
            c1_derivative_id='derivative_c1'; c1_sha256=$completeHash
        }) })
    }
    Write-Json $sourceMapPath $sourceMap
    $decisionSourcePath = Join-Path $inputRoot 'decisions.json'
    Write-Json $decisionSourcePath @([ordered]@{ module_id='m1'; decision='text_sufficient' })

    $ledgerPath = Join-Path $inputRoot 'c1b-registration-ledger.json'
    $ledger = [ordered]@{
        schema='babata.mba-course-c1b-registration/v1'; course=$plan.course; course_key=$plan.course_key; short_name=$plan.short_name
        generated_at='2026-08-15T00:00:00Z'; status='registered'; course_plan=$planPath; course_plan_sha256=$planHash
        source_map=$sourceMapPath; source_map_sha256=(Get-Sha256 $sourceMapPath)
        decision_source=$decisionSourcePath; decision_source_sha256=(Get-Sha256 $decisionSourcePath)
        coverage=[ordered]@{ modules=1; complete_c1_reused=1; essence_decisions_registered=1; retained_media_registered=0 }
        registrations=@([ordered]@{
            module_id='m1'; title='Fixture module'; source_item_id='item_m1'; source_revision_id='rev_m1'; source_asset_id='asset_m1'; source_asset_sha256=$assetHash
            complete_c1=[ordered]@{ derivative_id='derivative_c1'; output_sha256=$completeHash; logical_path='02_derived/files/complete-c1.md'; registration='reused' }
            decision_registration=[ordered]@{ derivative_id='derivative_decision'; output_sha256=$decisionHash; logical_path='02_derived/files/decision.json'; registration='registered' }
            media_registrations=@()
        })
    }
    Write-Json $ledgerPath $ledger
    return [pscustomobject]@{ Root=$root; DataHome=$dataHome; RawDb=$rawDb; DerivedDb=$derivedDb; PlanPath=$planPath; SourceMapPath=$sourceMapPath; LedgerPath=$ledgerPath; LogPath=(Join-Path $root 'fake.log') }
}

function Invoke-Registrar([object]$Fixture, [string]$StagingName, [bool]$ShouldSucceed, [string]$ExpectedError = '', [bool]$AllowLegacy = $false) {
    $staging = Join-Path $Fixture.Root $StagingName
    $env:BABATA_FAKE_LOG = $Fixture.LogPath
    $caught = $null
    $output = $null
    try {
        $arguments = @{
            CoursePlanPath = $Fixture.PlanPath
            C1BRegistrationLedgerPath = $Fixture.LedgerPath
            StagingRoot = $staging
            BabataExe = $fakeBabata
            DataHome = $Fixture.DataHome
            SqliteExe = $sqliteExe
        }
        if ($AllowLegacy) { $arguments.AllowLegacyGeneratedAt = $true }
        $output = & $scriptPath @arguments 2>&1
    } catch {
        $caught = $_
    }
    if ($ShouldSucceed) {
        if ($null -ne $caught) { throw "Registrar unexpectedly failed: $($caught.Exception.Message)" }
    } else {
        if ($null -eq $caught) { throw "Registrar unexpectedly succeeded: $($output -join [Environment]::NewLine)" }
        if ($ExpectedError -and $caught.Exception.Message -notmatch [regex]::Escape($ExpectedError)) {
            throw "Registrar failed with the wrong error. Expected '$ExpectedError', actual '$($caught.Exception.Message)'`n$($caught.ScriptStackTrace)"
        }
    }
    return [pscustomobject]@{ Staging=$staging; Output=$output; Error=$caught }
}

try {
    $text = Get-Content -LiteralPath $scriptPath -Raw -Encoding utf8
    Assert-True ($text.Contains("-readonly")) 'Registrar must keep every SQLite read in readonly mode'
    Assert-True (-not ($text -match '(?im)^\s*(INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)\s+')) 'Registrar contains a direct SQLite write statement'
    Assert-True (-not $text.Contains('foundation:$foundationKey')) 'Registrar must not double-prefix a canonical foundation key'
    Assert-True ($text.Contains('map_node_refs = @($foundationKey)')) 'Registrar must use the full canonical foundation key directly'
    Assert-True ($text.Contains('ConvertFrom-Json -DateKind String')) 'Registrar must preserve RFC3339 JSON timestamps as strings'
    Assert-True ($text.Contains('Require-Rfc3339Utc')) 'Registrar must validate C1B ledger generated_at before mutation'
    Assert-True ($text.Contains('[switch]$AllowLegacyGeneratedAt')) 'Registrar must make legacy timestamp compatibility explicit'
    Assert-True ($text.Contains('[IO.File]::Move($temporary, $manifestPath, $true)')) 'Registrar must atomically overwrite an existing state receipt on Windows'
    Assert-True (-not $text.Contains('Move-Item -LiteralPath $temporary -Destination $manifestPath -Force')) 'Registrar retains a Windows-incompatible state overwrite path'
    Assert-True (-not $text.Contains('101')) 'Generic registrar must not hardcode the supply-chain denominator'

    $preflight = New-Fixture 'preflight-only'
    $preflightStaging = Join-Path $preflight.Root 'staging'
    $preflightOutput = & $scriptPath -CoursePlanPath $preflight.PlanPath -C1BRegistrationLedgerPath $preflight.LedgerPath -StagingRoot $preflightStaging -BabataExe $fakeBabata -DataHome $preflight.DataHome -SqliteExe $sqliteExe -PreflightOnly
    Assert-True (($preflightOutput -join "`n") -match 'preflight=passed') 'Preflight-only mode did not report success'
    $preflightManifest = Get-Content -LiteralPath (Join-Path $preflightStaging 'knowledge-universe-registration.json') -Raw -Encoding utf8 | ConvertFrom-Json
    Assert-True ([string]$preflightManifest.status -ceq 'preflight_passed') 'Preflight-only mode wrote the wrong status'
    Assert-True (-not (Test-Path -LiteralPath $preflight.LogPath)) 'Preflight-only mode invoked a Rust mutation command'

    $happy = New-Fixture 'happy-interruption'
    $env:BABATA_FAKE_FAIL_COMMAND = 'knowledge ingest'
    $env:BABATA_FAKE_FAIL_MARKER = Join-Path $happy.Root 'fail-once.marker'
    [void](Invoke-Registrar $happy 'staging-interrupted' $false 'babata command failed')
    $env:BABATA_FAKE_FAIL_COMMAND = ''
    $env:BABATA_FAKE_FAIL_MARKER = ''
    $resumed = Invoke-Registrar $happy 'staging-resumed' $true
    $manifest = Get-Content -LiteralPath (Join-Path $resumed.Staging 'knowledge-universe-registration.json') -Raw -Encoding utf8 | ConvertFrom-Json
    Assert-True ([string]$manifest.status -ceq 'registered') 'Fresh-root retry did not complete registration'
    Assert-True ([string]$manifest.course_acceptance -ceq 'pending_user_acceptance') 'Engineering registration promoted course acceptance'
    Assert-True (@($manifest.modules).Count -eq 1) 'Dynamic one-module denominator was not completed'
    $candidate = Get-Content -LiteralPath (Join-Path $resumed.Staging 'module-m1\semantic-candidate.json') -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    Assert-True (@($candidate.entries[0].map_node_refs).Count -eq 1 -and [string]$candidate.entries[0].map_node_refs[0] -ceq 'foundation:consciousness') 'Canonical foundation reference was not preserved exactly'
    Assert-True ([string]$candidate.generated_at -ceq '2026-08-15T00:00:00Z') 'Registrar changed a canonical generated_at during JSON round-trip'
    $callsBeforeIdempotentRun = @(Get-Content -LiteralPath $happy.LogPath).Count
    [void](Invoke-Registrar $happy 'staging-idempotent' $true)
    $callsAfterIdempotentRun = @(Get-Content -LiteralPath $happy.LogPath).Count
    Assert-True ($callsAfterIdempotentRun -eq $callsBeforeIdempotentRun) 'Idempotent fresh-root retry issued new Rust mutations'

    $ledgerMismatch = New-Fixture 'ledger-mismatch'
    $ledger = Get-Content -LiteralPath $ledgerMismatch.LedgerPath -Raw -Encoding utf8 | ConvertFrom-Json
    $ledger.course_plan_sha256 = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
    Write-Json $ledgerMismatch.LedgerPath $ledger
    [void](Invoke-Registrar $ledgerMismatch 'staging' $false 'C1B ledger course_plan hash mismatch')

    $sourceMismatch = New-Fixture 'source-map-mismatch'
    $sourceMap = Get-Content -LiteralPath $sourceMismatch.SourceMapPath -Raw -Encoding utf8 | ConvertFrom-Json
    $sourceMap.course = 'Another MBA course'
    Write-Json $sourceMismatch.SourceMapPath $sourceMap
    $ledger = Get-Content -LiteralPath $sourceMismatch.LedgerPath -Raw -Encoding utf8 | ConvertFrom-Json
    $ledger.source_map_sha256 = Get-Sha256 $sourceMismatch.SourceMapPath
    Write-Json $sourceMismatch.LedgerPath $ledger
    [void](Invoke-Registrar $sourceMismatch 'staging' $false 'Course plan and source map course mismatch')

    $extraParent = New-Fixture 'extra-parent'
    Invoke-Sql $extraParent.RawDb "INSERT INTO knowledge_map_nodes VALUES('mapnode_extra','foundation','foundation:matter','物质','active'); INSERT INTO knowledge_map_edges VALUES('map_version_test','mapnode_extra','mapnode_management','first_party',NULL,'2026-08-15T00:00:00Z');"
    [void](Invoke-Registrar $extraParent 'staging' $false 'Discipline parent set must be exactly')

    $denominator = New-Fixture 'denominator'
    $plan = Get-Content -LiteralPath $denominator.PlanPath -Raw -Encoding utf8 | ConvertFrom-Json
    $plan.expected_modules = 2
    Write-Json $denominator.PlanPath $plan
    $ledger = Get-Content -LiteralPath $denominator.LedgerPath -Raw -Encoding utf8 | ConvertFrom-Json
    $ledger.course_plan_sha256 = Get-Sha256 $denominator.PlanPath
    Write-Json $denominator.LedgerPath $ledger
    [void](Invoke-Registrar $denominator 'staging' $false 'Chapter plan must assign exactly 2 modules')

    $localizedTimestamp = New-Fixture 'localized-timestamp'
    $ledger = Get-Content -LiteralPath $localizedTimestamp.LedgerPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    $ledger.generated_at = '08/15/2026 06:37:24'
    Write-Json $localizedTimestamp.LedgerPath $ledger
    [void](Invoke-Registrar $localizedTimestamp 'staging' $false 'C1B ledger generated_at must be RFC3339 UTC')

    $legacyCompatible = New-Fixture 'legacy-compatible-timestamp'
    $ledger = Get-Content -LiteralPath $legacyCompatible.LedgerPath -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    $ledger.generated_at = '08/15/2026 06:37:24'
    Write-Json $legacyCompatible.LedgerPath $ledger
    $legacyResult = Invoke-Registrar $legacyCompatible 'staging' $true '' $true
    $legacyCandidate = Get-Content -LiteralPath (Join-Path $legacyResult.Staging 'module-m1\semantic-candidate.json') -Raw -Encoding utf8 | ConvertFrom-Json -DateKind String
    Assert-True ([string]$legacyCandidate.generated_at -ceq '08/15/2026 06:37:24') 'Explicit compatibility changed the legacy package fingerprint input'

    $duplicate = New-Fixture 'duplicate-readback'
    Invoke-Sql $duplicate.DerivedDb "INSERT INTO process_runs VALUES('run_c1_duplicate','rev_m1','item_m1','succeeded',NULL,'extract','fixture','fixture','1'); INSERT INTO derivatives SELECT derivative_id,'run_c1_duplicate',output_sha256,logical_path,kind FROM derivatives WHERE derivative_id='derivative_c1';"
    [void](Invoke-Registrar $duplicate 'staging' $false 'C1B ledger derivative must have exactly one read-back row')

    Write-Output 'register-mba-course-c2b-knowledge behavioral contract: passed'
} finally {
    Remove-Item Env:BABATA_FAKE_LOG -ErrorAction SilentlyContinue
    Remove-Item Env:BABATA_FAKE_FAIL_COMMAND -ErrorAction SilentlyContinue
    Remove-Item Env:BABATA_FAKE_FAIL_MARKER -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $testRoot) { Remove-Item -LiteralPath $testRoot -Recurse -Force }
}
