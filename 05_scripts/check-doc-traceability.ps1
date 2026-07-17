$ErrorActionPreference = 'Stop'

$docs = (Resolve-Path (Join-Path $PSScriptRoot '..\00_docs')).Path

$requiredMarkers = @(
    @('00_requirements/00_REQUIREMENTS.md', '## -1.'),
    @('01_prd/01_PRD.md', 'PRD-10'),
    @('02_acceptance/02_ACCEPTANCE_CRITERIA.md', 'AC-11'),
    @('03_architecture/03_ARCHITECTURE.md', 'AC-11'),
    @('03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md', '137'),
    @('03_architecture/05_RAW_FOUNDATION_BLUEPRINT.md', 'P3-G6'),
    @('03_architecture/06_RAW_FOUNDATION_EXECUTION_PLAN.md', 'P3-G6'),
    @('03_architecture/07_P4_FIRST_COLLECTION_PATHS.md', 'P4-G6'),
    @('03_architecture/08_SOURCE_TOOL_RESEARCH.md', 'Agent'),
    @('04_process/04_DEVELOPMENT_PROCESS.md', 'P2-G6'),
    @('05_tests/05_TEST_CASES.md', 'GT-P2-06')
)

foreach ($check in $requiredMarkers) {
    $path = Join-Path $docs $check[0]
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing authority document: $($check[0])"
    }
    if (-not (Select-String -SimpleMatch $check[1] -Path $path -Quiet)) {
        throw "Missing traceability marker '$($check[1])' in $($check[0])"
    }
}

$prd = Get-Content -Raw -Encoding utf8 (Join-Path $docs '01_prd/01_PRD.md')
$acceptance = Get-Content -Raw -Encoding utf8 (Join-Path $docs '02_acceptance/02_ACCEPTANCE_CRITERIA.md')
$architecture = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/03_ARCHITECTURE.md')
$tests = Get-Content -Raw -Encoding utf8 (Join-Path $docs '05_tests/05_TEST_CASES.md')

foreach ($id in 1..10) {
    $marker = 'PRD-{0:D2}' -f $id
    if (-not $prd.Contains($marker)) { throw "PRD is missing marker: $marker" }
    if (-not $acceptance.Contains($marker)) { throw "Acceptance is missing PRD trace: $marker" }
}

foreach ($id in 1..11) {
    $ac = 'AC-{0:D2}' -f $id
    $tc = 'TC-{0:D2}' -f $id
    if (-not $acceptance.Contains($ac)) { throw "Acceptance is missing marker: $ac" }
    if (-not $architecture.Contains($ac)) { throw "Architecture is missing acceptance trace: $ac" }
    if (-not $tests.Contains($ac)) { throw "Tests are missing acceptance trace: $ac" }
    if (-not $tests.Contains($tc)) { throw "Tests are missing marker: $tc" }
}

$skeleton = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md')
$rawBlueprint = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/05_RAW_FOUNDATION_BLUEPRINT.md')
$rawPlan = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/06_RAW_FOUNDATION_EXECUTION_PLAN.md')
$collection = Get-Content -Raw -Encoding utf8 (Join-Path $docs '03_architecture/07_P4_FIRST_COLLECTION_PATHS.md')
$process = Get-Content -Raw -Encoding utf8 (Join-Path $docs '04_process/04_DEVELOPMENT_PROCESS.md')

foreach ($id in 1..7) {
    $p2 = "P2-G$id"
    if (-not $skeleton.Contains($p2)) { throw "Skeleton is missing gate: $p2" }
    if (-not $process.Contains($p2)) { throw "Process is missing gate: $p2" }
    if (-not $tests.Contains($p2)) { throw "Tests are missing gate: $p2" }

    if ($id -le 6) {
        $p3 = "P3-G$id"
        if (-not $rawBlueprint.Contains($p3)) { throw "Raw blueprint is missing gate: $p3" }
        if (-not $rawPlan.Contains($p3)) { throw "Raw execution plan is missing gate: $p3" }
        if (-not $process.Contains($p3)) { throw "Process is missing gate: $p3" }

        $p4 = "P4-G$id"
        if (-not $collection.Contains($p4)) { throw "Collection paths are missing gate: $p4" }
        if (-not $process.Contains($p4)) { throw "Process is missing gate: $p4" }
    }
}

Write-Output 'Document traceability passed: 00 -> PRD-01..10 -> AC-01..11 -> architecture/process -> TC-01..11.'
