$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$scriptPath = Join-Path $PSScriptRoot 'register-mba-course-c1b-handoff.ps1'
$tokens = $null
$errors = $null
[void][Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$errors)
if (@($errors).Count -gt 0) {
    throw "Generic MBA C1B registrar has PowerShell parse errors: $($errors.Message -join '; ')"
}

$text = Get-Content -LiteralPath $scriptPath -Raw -Encoding utf8
foreach ($required in @(
    '[Parameter(Mandatory=$true)][string]$CoursePlanPath',
    '[Parameter(Mandatory=$true)][string]$PreparationManifestPath',
    '[Parameter(Mandatory=$true)][string]$StagingRoot',
    'babata.mba-course-c2b-plan/v1',
    'babata.mba-course-c1b-preparation/v1',
    'babata.mba-course-c1b-registration/v1',
    'mba-course-c1b-media-extractor',
    'mba-course-c1b-essence-registrar',
    'Find-ExactFingerprint',
    'Select-ExactFingerprintRows',
    'ConvertTo-CanonicalJsonValue',
    'Confirm-RegisteredRun',
    'partial-registration.json',
    'Write-JsonAtomic',
    'ConvertFrom-Json -DateKind String',
    '[Console]::OutputEncoding = $utf8NoBom',
    'Require-Rfc3339Utc',
    'timestamp self-test passed',
    '[IO.File]::Move($temporary, $Path, $true)',
    'Only PNG/JPG retained C1B media is supported',
    'course_plan_sha256',
    '# PRECHECK_COMPLETE',
    '# MUTATION_START'
)) {
    if (-not $text.Contains($required)) { throw "Generic MBA C1B registrar is missing required marker: $required" }
}

if ($text.Contains("json(p.params_json)=json('", [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Generic MBA C1B registrar compares params_json in SQLite and remains sensitive to object key order'
}
if ($text.Contains('Move-Item -LiteralPath $temporary -Destination $Path -Force', [StringComparison]::Ordinal)) {
    throw 'Generic MBA C1B registrar uses a Windows-incompatible atomic overwrite path'
}

$selfTest = & $scriptPath -CoursePlanPath 'self-test' -PreparationManifestPath 'self-test' -StagingRoot 'self-test' -SelfTest
if (($selfTest -join "`n") -cnotmatch 'fingerprint self-test passed') {
    throw 'Generic MBA C1B registrar fingerprint mutation tests did not pass'
}
if (($selfTest -join "`n") -cnotmatch 'timestamp self-test passed') {
    throw 'Generic MBA C1B registrar timestamp round-trip and rejection tests did not pass'
}

foreach ($forbidden in @('finance', '财务管理', 'MBAO5406', '-ne 37', '-ne 76', '= 37', '= 76')) {
    if ($text.Contains($forbidden, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Generic MBA C1B registrar contains course-specific hardcoding: $forbidden"
    }
}

$preflightIndex = $text.IndexOf('# PRECHECK_COMPLETE', [StringComparison]::Ordinal)
$mutationIndex = $text.IndexOf('# MUTATION_START', [StringComparison]::Ordinal)
$firstStagingWrite = $text.IndexOf("New-Item -ItemType Directory -Path `$staging", [StringComparison]::Ordinal)
$firstRegister = $text.IndexOf("'process','register'", [StringComparison]::Ordinal)
if ($preflightIndex -lt 0 -or $mutationIndex -le $preflightIndex -or
    $firstStagingWrite -le $mutationIndex -or $firstRegister -le $mutationIndex) {
    throw 'Generic MBA C1B registrar mutation boundary is not ordered after full preflight'
}

if ($text -match '(?im)^\s*(INSERT\s+INTO|UPDATE\s+[a-z_]|DELETE\s+FROM|REPLACE\s+INTO)') {
    throw 'Generic MBA C1B registrar contains direct SQL mutation text'
}

Write-Output 'generic MBA C1B registrar static checks passed'
