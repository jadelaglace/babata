$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$app = Join-Path $root '01_app'
$traits = @{
    RawRepositoryPort = '02_babata_application/src/ports/raw_repository.rs'
    AssetStorePort = '02_babata_application/src/ports/asset_store.rs'
    DerivedRepositoryPort = '02_babata_application/src/ports/derived_repository.rs'
    JobRepositoryPort = '02_babata_application/src/ports/job_repository.rs'
    ProcessProviderPort = '02_babata_application/src/ports/process_provider.rs'
    SourceAdapterPort = '02_babata_application/src/ports/source_adapter.rs'
    CandidateRunnerPort = '02_babata_application/src/ports/candidate_runner.rs'
    ReadProjectionPort = '02_babata_application/src/ports/read_projection.rs'
    ViewBuilderPort = '02_babata_application/src/ports/view_builder.rs'
    OutputBuilderPort = '02_babata_application/src/ports/output_builder.rs'
    BackupDriverPort = '02_babata_application/src/ports/backup_driver.rs'
    CapabilityRegistryPort = '02_babata_application/src/ports/capability_registry.rs'
    ClockPort = '02_babata_application/src/ports/clock.rs'
}
foreach ($name in $traits.Keys) {
    $matches = @(rg --files-with-matches "pub trait $name" $app -g '*.rs')
    if ($matches.Count -ne 1 -or -not $matches[0].Replace('\', '/').EndsWith($traits[$name])) {
        throw "$name must have exactly one owner: $($traits[$name])"
    }
}

$portMethodCounts = @{
    RawRepositoryPort = 8
    AssetStorePort = 6
    DerivedRepositoryPort = 6
    JobRepositoryPort = 7
    ProcessProviderPort = 6
    SourceAdapterPort = 4
    CandidateRunnerPort = 2
    ReadProjectionPort = 5
    ViewBuilderPort = 3
    OutputBuilderPort = 4
    BackupDriverPort = 3
    CapabilityRegistryPort = 2
    ClockPort = 1
}
if (($portMethodCounts.Values | Measure-Object -Sum).Sum -ne 57) {
    throw 'P2 port method baseline must total 57'
}
foreach ($name in $portMethodCounts.Keys) {
    $path = Join-Path $app $traits[$name]
    $actual = (Select-String -Path $path -Pattern '^\s*fn\s+').Count
    if ($actual -lt $portMethodCounts[$name]) {
        throw "$name requires at least $($portMethodCounts[$name]) methods, found $actual"
    }
}

$services = @{
    CollectorSessionService = 'usecases/collector.rs'
    CaptureService = 'usecases/capture.rs'
    WorkspaceService = 'usecases/workspace.rs'
    KnowledgeService = 'usecases/knowledge.rs'
    ProcessService = 'usecases/process.rs'
    ExploreService = 'usecases/explore.rs'
    SublibraryService = 'usecases/sublibraries.rs'
    ViewService = 'usecases/views.rs'
    OutputService = 'usecases/outputs.rs'
    RouteService = 'usecases/routes.rs'
    OpsService = 'usecases/ops.rs'
    CapabilityService = 'usecases/capabilities.rs'
}
foreach ($name in $services.Keys) {
    $matches = @(rg --files-with-matches "pub struct $name" (Join-Path $app '02_babata_application/src') -g '*.rs')
    if ($matches.Count -ne 1 -or -not $matches[0].Replace('\', '/').EndsWith($services[$name])) {
        throw "$name must have exactly one application owner"
    }
}

$serviceMethods = @{
    CollectorSessionService = @('start', 'candidates', 'select', 'status', 'recollect')
    CaptureService = @('capture_text', 'capture_file', 'capture_export', 'capture_candidate')
    WorkspaceService = @('create', 'revise', 'annotate')
    KnowledgeService = @('record', 'relate', 'classify', 'model', 'score', 'analyze', 'decide_suggestion')
    ProcessService = @('enqueue', 'run_once', 'status', 'retry', 'cancel', 'list_pipelines')
    ExploreService = @('search', 'show')
    SublibraryService = @('create', 'revise', 'show', 'materialize')
    ViewService = @('list', 'build')
    OutputService = @('list', 'build', 'status', 'verify')
    RouteService = @('list', 'show', 'evaluate', 'collect')
    OpsService = @('status', 'doctor', 'backup', 'restore_verify')
    CapabilityService = @('list')
}
if (($serviceMethods.Values | ForEach-Object { $_.Count } | Measure-Object -Sum).Sum -ne 46) {
    throw 'P2 service method baseline must total 46'
}
# P6.1 activates review preparation without freezing the corrected semantic write model.
$serviceMethods.KnowledgeService = @('review')
foreach ($name in $serviceMethods.Keys) {
    $path = Join-Path $app "02_babata_application/src/$($services[$name])"
    foreach ($method in $serviceMethods[$name]) {
        if (-not (Select-String -Path $path -Pattern "pub fn $method\b" -Quiet)) {
            throw "$name is missing required P2 method: $method"
        }
    }
}

$commandModules = @(
    'data', 'capabilities', 'collector', 'capture', 'workspace', 'knowledge',
    'process', 'explore', 'sublibraries', 'views', 'outputs', 'routes', 'ops'
)
foreach ($module in $commandModules) {
    if (-not (Test-Path (Join-Path $app "04_babata_cli/src/commands/$module.rs"))) {
        throw "Missing CLI command owner: $module"
    }
}
$routeModules = @('collector', 'capture', 'workspace', 'process', 'explore', 'outputs', 'health')
foreach ($module in $routeModules) {
    if (-not (Test-Path (Join-Path $app "05_babata_local_api/src/routes/$module.rs"))) {
        throw "Missing local API route owner: $module"
    }
}
foreach ($symbol in @('pub fn build', 'pub fn run', 'pub fn claim_once', 'pub fn heartbeat', 'pub fn shutdown')) {
    if (-not (rg --fixed-strings $symbol (Join-Path $app '06_babata_worker/src') -g '*.rs')) {
        throw "Missing worker lifecycle symbol: $symbol"
    }
}
Write-Output 'Interface ownership check passed: 13 P2 ports, 12 services with P6.1 review preparation, 13 CLI modules, local API owners, and worker lifecycle.'
