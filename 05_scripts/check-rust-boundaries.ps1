$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\01_app')).Path
$application = Join-Path $root '02_babata_application'
$domain = Join-Path $root '01_babata_domain'
$forbidden = @('rusqlite', 'std::fs', 'std::net', 'std::process', 'std::env', 'reqwest', 'axum', 'clap', 'tokio::process', 'OffsetDateTime::now_utc', 'UtcTimestamp::now')

foreach ($path in @((Join-Path $domain 'src'), (Join-Path $application 'src'))) {
    foreach ($term in $forbidden) {
        if (rg --fixed-strings --glob '*.rs' $term $path) {
            throw "Forbidden dependency '$term' found under $path"
        }
    }
}

$metadata = cargo metadata --format-version 1 --no-deps --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
$workspace = @($metadata.packages | Where-Object { $_.name -like 'babata-*' })
if ($workspace.Count -ne 6) { throw "Expected 6 Babata packages, found $($workspace.Count)" }
$allowed = @{
    'babata-domain' = @()
    'babata-application' = @('babata-domain')
    'babata-infrastructure' = @('babata-domain', 'babata-application')
    'babata-cli' = @('babata-domain', 'babata-application', 'babata-infrastructure')
    'babata-local-api' = @('babata-domain', 'babata-application', 'babata-infrastructure')
    'babata-worker' = @('babata-domain', 'babata-application', 'babata-infrastructure')
}
foreach ($package in $workspace) {
    $internal = @($package.dependencies | Where-Object { $_.name -like 'babata-*' } | ForEach-Object { $_.name })
    foreach ($dependency in $internal) {
        if ($dependency -notin $allowed[$package.name]) {
            throw "Forbidden workspace dependency: $($package.name) -> $dependency"
        }
    }
}

Write-Output 'Rust dependency boundary check passed.'
