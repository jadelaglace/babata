[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$total = [Diagnostics.Stopwatch]::StartNew()

$checks = [ordered]@{
    'P2 skeleton inventory' = 'check-p2-skeleton-inventory.ps1'
    'P3 raw inventory' = 'check-p3-raw-inventory.ps1'
    'Interface ownership' = 'check-interface-ownership.ps1'
    'Rust boundaries' = 'check-rust-boundaries.ps1'
    'No secondary writer' = 'check-no-secondary-writer.ps1'
    'Document traceability' = 'check-doc-traceability.ps1'
    'Document traceability checker tests' = 'test-doc-traceability.ps1'
    'Collection Skill' = 'check-collection-skill.ps1'
    'Collection Skill checker tests' = 'test-collection-skill.ps1'
    'Document provenance' = 'check-doc-provenance.ps1'
    'Document provenance checker tests' = 'test-doc-provenance.ps1'
    'Data-root boundaries' = 'check-data-root-boundaries.ps1'
    'Auxiliary data-root migration tests' = 'test-auxiliary-data-root-migration.ps1'
}

foreach ($entry in $checks.GetEnumerator()) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    Write-Output "==> $($entry.Key)"
    & (Join-Path $PSScriptRoot $entry.Value)
    $timer.Stop()
    Write-Output ("<== {0} passed in {1:N1}s" -f $entry.Key, $timer.Elapsed.TotalSeconds)
}

$total.Stop()
Write-Output ("Boundary checks passed in {0:N1}s." -f $total.Elapsed.TotalSeconds)
