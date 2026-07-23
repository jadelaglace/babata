$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$migration = Join-Path $PSScriptRoot 'migrate-auxiliary-data-roots.ps1'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("babata-auxiliary-roots-{0}" -f [guid]::NewGuid())

function New-DataRootFixture {
    param(
        [Parameter(Mandatory)]
        [string]$Root
    )

    foreach ($name in @('00_inbox', '01_raw', '02_derived', '03_views', '04_runtime', '05_logs')) {
        New-Item -ItemType Directory -Path (Join-Path $Root $name) -Force | Out-Null
    }
    Set-Content -LiteralPath (Join-Path $Root 'README.local.md') -Encoding utf8 -Value 'local fixture'
}

function Assert-FileText {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$Expected
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Expected migrated file is missing: $Path"
    }
    $actual = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    if ($actual.TrimEnd() -ne $Expected) {
        throw "Migrated file content changed: $Path"
    }
}

try {
    $dataRoot = Join-Path $testRoot 'BabataData'
    $evidenceRoot = Join-Path $testRoot 'BabataEvidence'
    $recoveryRoot = Join-Path $testRoot 'BabataRecovery'
    New-DataRootFixture -Root $dataRoot

    New-Item -ItemType Directory -Path (Join-Path $dataRoot 'verification\p6') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $dataRoot 'recovery-staging\doubao\batch') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $dataRoot 'generated\task\results') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $dataRoot 'verification\p6\report.md') -Encoding utf8 -Value 'evidence'
    Set-Content -LiteralPath (Join-Path $dataRoot 'recovery-staging\doubao\batch\source.docx') -Encoding utf8 -Value 'original'
    Set-Content -LiteralPath (Join-Path $dataRoot 'generated\task\results\output.md') -Encoding utf8 -Value 'staging'

    $audit = @(& $migration -DataRoot $dataRoot -EvidenceRoot $evidenceRoot -RecoveryRoot $recoveryRoot -OperationId 'fixture-audit')
    if (-not ($audit | Out-String).Contains('Audit only')) {
        throw 'Default migration mode must be audit-only.'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $dataRoot 'verification\p6\report.md'))) {
        throw 'Audit-only mode changed the source data root.'
    }
    if (Test-Path -LiteralPath $evidenceRoot) {
        throw 'Audit-only mode created the evidence root.'
    }

    & $migration -DataRoot $dataRoot -EvidenceRoot $evidenceRoot -RecoveryRoot $recoveryRoot -OperationId 'fixture-apply' -Apply | Out-Null

    Assert-FileText -Path (Join-Path $evidenceRoot 'runs\p6\report.md') -Expected 'evidence'
    Assert-FileText -Path (Join-Path $recoveryRoot 'batches\doubao\batch\source.docx') -Expected 'original'
    Assert-FileText -Path (Join-Path $dataRoot '04_runtime\staging\model-workspaces\task\results\output.md') -Expected 'staging'
    foreach ($legacy in @('verification', 'recovery-staging', 'generated')) {
        if (Test-Path -LiteralPath (Join-Path $dataRoot $legacy)) {
            throw "Legacy top-level directory remains after migration: $legacy"
        }
    }

    $manifestPath = Join-Path $evidenceRoot 'governance\migrations\fixture-apply\manifest.json'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($manifest.schema -ne 'babata.auxiliary-root-migration/v1' -or $manifest.status -ne 'completed') {
        throw 'Migration manifest did not record a completed v1 operation.'
    }
    if (@($manifest.entries).Count -ne 3) {
        throw "Migration manifest expected three entries, found $(@($manifest.entries).Count)"
    }
    foreach ($entry in $manifest.entries) {
        if ($entry.file_count -ne 1 -or @($entry.files).Count -ne 1 -or $entry.files[0].sha256.Length -ne 64) {
            throw "Migration manifest is missing a complete hash inventory for $($entry.source_name)"
        }
    }

    $clean = @(& $migration -DataRoot $dataRoot -EvidenceRoot $evidenceRoot -RecoveryRoot $recoveryRoot -OperationId 'fixture-clean')
    if (-not ($clean | Out-String).Contains('already clean')) {
        throw 'A second audit should report an already-clean data root.'
    }

    $collisionData = Join-Path $testRoot 'CollisionData'
    $collisionEvidence = Join-Path $testRoot 'CollisionEvidence'
    $collisionRecovery = Join-Path $testRoot 'CollisionRecovery'
    New-DataRootFixture -Root $collisionData
    New-Item -ItemType Directory -Path (Join-Path $collisionData 'verification') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $collisionData 'verification\source.txt') -Encoding utf8 -Value 'must remain'
    New-Item -ItemType Directory -Path (Join-Path $collisionEvidence 'runs') -Force | Out-Null

    $failedAsExpected = $false
    try {
        & $migration -DataRoot $collisionData -EvidenceRoot $collisionEvidence -RecoveryRoot $collisionRecovery -OperationId 'fixture-collision' -Apply | Out-Null
    }
    catch {
        $failedAsExpected = $_.Exception.Message.Contains('Destination already exists')
    }
    if (-not $failedAsExpected) {
        throw 'A pre-existing destination must fail closed.'
    }
    Assert-FileText -Path (Join-Path $collisionData 'verification\source.txt') -Expected 'must remain'

    Write-Output 'Auxiliary data-root migration tests passed: audit-only, verified migration, clean rerun, and collision fail-closed.'
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        $resolvedTest = (Resolve-Path -LiteralPath $testRoot).Path
        $systemTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolvedTest.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected test path: $resolvedTest"
        }
        Remove-Item -LiteralPath $resolvedTest -Recurse -Force
    }
}
