$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$app = Join-Path $repo '01_app'
$required = @(
    '01_babata_domain/src/lib.rs',
    '01_babata_domain/src/ids.rs',
    '01_babata_domain/src/kinds.rs',
    '01_babata_domain/src/entities.rs',
    '01_babata_domain/src/value.rs',
    '01_babata_domain/src/error.rs',
    '02_babata_application/src/lib.rs',
    '02_babata_application/src/dto.rs',
    '02_babata_application/src/error.rs',
    '02_babata_application/src/ports/mod.rs',
    '02_babata_application/src/ports/raw_repository.rs',
    '02_babata_application/src/ports/asset_store.rs',
    '02_babata_application/src/usecases/mod.rs',
    '02_babata_application/src/usecases/capture.rs',
    '02_babata_application/src/usecases/workspace.rs',
    '03_babata_infrastructure/src/lib.rs',
    '03_babata_infrastructure/src/config.rs',
    '03_babata_infrastructure/src/paths.rs',
    '03_babata_infrastructure/src/observability.rs',
    '03_babata_infrastructure/src/sqlite/mod.rs',
    '03_babata_infrastructure/src/sqlite/migrate.rs',
    '03_babata_infrastructure/src/sqlite/raw_repository.rs',
    '03_babata_infrastructure/src/assets/mod.rs',
    '03_babata_infrastructure/src/assets/file_store.rs',
    '04_babata_cli/src/main.rs',
    '04_babata_cli/src/app.rs',
    '04_babata_cli/src/commands/mod.rs',
    '04_babata_cli/src/commands/capture.rs',
    '04_babata_cli/src/render.rs'
)

if ($required.Count -ne 29) { throw 'P3 raw inventory baseline must contain 29 active files' }
foreach ($relative in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $app $relative))) {
        throw "Missing P3 raw file: $relative"
    }
}
foreach ($migration in @('0001_raw_schema.sql', '0002_raw_indexes.sql', '0003_raw_fts.sql', '0004_capture_operations.sql')) {
    if (-not (Test-Path -LiteralPath (Join-Path $repo "03_migrations/01_raw/$migration"))) {
        throw "Missing P3 raw migration: $migration"
    }
}
$rawMigrations = @(Get-ChildItem -File -LiteralPath (Join-Path $repo '03_migrations/01_raw') -Filter '*.sql')
if ($rawMigrations.Count -ne 4) {
    throw "P3 raw migration set must contain exactly 4 migrations, found $($rawMigrations.Count)"
}
$collectionMigration = Join-Path $repo '03_migrations/02_collection/0001_route_evidence.sql'
if (-not (Test-Path -LiteralPath $collectionMigration)) {
    throw 'P4 route evidence migration must remain preserved outside the P3 raw migration set'
}

$capabilities = Get-Content -Raw -Encoding utf8 (Join-Path $app '03_babata_infrastructure/src/capabilities.rs')
if (-not $capabilities.Contains('CapabilityDescriptor::unavailable("capture.candidate", "P4")')) {
    throw 'P4 candidate capture must remain unavailable during P3'
}
$cliApp = Get-Content -Raw -Encoding utf8 (Join-Path $app '04_babata_cli/src/app.rs')
foreach ($marker in @(
    'RootCommand::Capture(_) => return Err(unavailable("capture.provider", "P4"))',
    'RootCommand::Routes(_) => return Err(unavailable("routes", "P4"))'
)) {
    if (-not $cliApp.Contains($marker)) {
        throw "P3 CLI is missing an inactive later-phase boundary: $marker"
    }
}

$commandOwner = Join-Path $app '04_babata_cli/src/commands'
foreach ($symbol in @('CaptureCommand', 'WorkspaceCommand', 'Create(NoteInput)', 'Revise(ReviseInput)', 'Annotate(AnnotateInput)')) {
    if (-not (rg --fixed-strings $symbol $commandOwner -g '*.rs')) {
        throw "Missing P3 CLI command mapping: $symbol"
    }
}

$testGroups = @{
    domain = @('01_babata_domain/src')
    application = @('02_babata_application/src/usecases/capture.rs', '02_babata_application/src/usecases/workspace.rs')
    infrastructure = @('03_babata_infrastructure/src/paths.rs', '03_babata_infrastructure/src/sqlite/migrate.rs', '03_babata_infrastructure/src/sqlite/raw_repository.rs', '03_babata_infrastructure/src/assets/file_store.rs')
    cli = @('04_babata_cli/tests')
}
$minimums = @{ domain = 6; application = 6; infrastructure = 6; cli = 2 }
$total = 0
foreach ($group in $testGroups.Keys) {
    $count = 0
    foreach ($relative in $testGroups[$group]) {
        $path = Join-Path $app $relative
        if (Test-Path -LiteralPath $path -PathType Container) {
            $count += (Get-ChildItem -File -Recurse $path -Filter '*.rs' | Select-String -SimpleMatch '#[test]' | Measure-Object).Count
        } else {
            $count += (Select-String -Path $path -SimpleMatch '#[test]' | Measure-Object).Count
        }
    }
    if ($count -lt $minimums[$group]) {
        throw "P3 $group test baseline requires $($minimums[$group]), found $count"
    }
    $total += $count
}
if ($total -lt 22) { throw "P3 requires at least 22 functional tests, found $total" }

$unexpectedRuntime = @(rg --files $repo -g '*.sqlite' -g '*.sqlite-wal' -g '*.sqlite-shm' -g '*.db' -g '*.log')
if ($unexpectedRuntime.Count -gt 0) {
    throw "Generated runtime data entered the repository: $($unexpectedRuntime -join ', ')"
}

Write-Output "P3 raw inventory check passed: 29 active files and $total raw-functional tests."
