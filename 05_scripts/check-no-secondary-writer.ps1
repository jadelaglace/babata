$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$app = Join-Path $root '01_app'
$nonInfrastructure = @(
    (Join-Path $app '01_babata_domain'),
    (Join-Path $app '02_babata_application'),
    (Join-Path $app '04_babata_cli'),
    (Join-Path $app '05_babata_local_api'),
    (Join-Path $app '06_babata_worker')
)
foreach ($term in @('rusqlite', 'Connection::open', 'sqlite3', 'pragma_update')) {
    foreach ($path in $nonInfrastructure) {
        if (rg --fixed-strings $term $path -g '*.rs' -g 'Cargo.toml') {
            throw "Secondary database writer marker '$term' found under $path"
        }
    }
}
$adapters = Join-Path $root '08_adapters'
foreach ($term in @('sqlite3', 'rusqlite', 'better-sqlite', 'fs.write', 'writeFile', 'open("w', "open('w")) {
    if (rg --fixed-strings $term $adapters -g '*.ts' -g '*.py' -g '*.json' -g '*.toml') {
        throw "Peripheral adapter contains forbidden writer marker: $term"
    }
}
$writerOwners = @(rg --files-with-matches 'impl RawRepositoryPort for' (Join-Path $app '03_babata_infrastructure/src') -g '*.rs')
if ($writerOwners.Count -ne 1 -or -not $writerOwners[0].Replace('\', '/').EndsWith('03_babata_infrastructure/src/sqlite/raw_repository.rs')) {
    throw 'RawRepositoryPort must have exactly one infrastructure implementation owner in P2'
}
Write-Output 'No-secondary-writer check passed: Rust infrastructure remains the sole persistence owner.'
