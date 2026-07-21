$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$app = Join-Path $repo '01_app'
$inventory = [ordered]@{
    '01_babata_domain' = @'
lib.rs ids.rs kinds.rs entities.rs value.rs error.rs capability.rs route.rs collection.rs processing.rs knowledge.rs query.rs sublibrary.rs output.rs view.rs ops.rs
'@
    '02_babata_application' = @'
lib.rs dto.rs error.rs ports/mod.rs ports/raw_repository.rs ports/asset_store.rs ports/derived_repository.rs ports/dense_expression_preview.rs ports/job_repository.rs ports/collection_repository.rs ports/process_provider.rs ports/source_adapter.rs ports/candidate_runner.rs ports/read_projection.rs ports/view_builder.rs ports/output_builder.rs ports/backup_driver.rs ports/capability_registry.rs ports/clock.rs ports/knowledge_core_repository.rs ports/semantic_digest_provider.rs usecases/mod.rs usecases/collector.rs usecases/capture.rs usecases/workspace.rs usecases/knowledge.rs usecases/dense_expression_preview.rs usecases/semantic_digest.rs usecases/process.rs usecases/explore.rs usecases/sublibraries.rs usecases/views.rs usecases/outputs.rs usecases/routes.rs usecases/ops.rs usecases/capabilities.rs
'@
    '03_babata_infrastructure' = @'
lib.rs config.rs paths.rs observability.rs capabilities.rs sqlite/mod.rs sqlite/migrate.rs sqlite/derived_migrate.rs sqlite/raw_repository.rs sqlite/derived_repository.rs sqlite/job_repository.rs sqlite/read_projection.rs sqlite/collection_migrate.rs sqlite/collection_repository.rs sqlite/knowledge_core_repository.rs assets/mod.rs assets/file_store.rs sources/mod.rs sources/registry.rs sources/candidate.rs sources/providers/mod.rs sources/providers/feishu.rs sources/providers/yuque.rs sources/providers/onenote.rs sources/providers/evernote.rs sources/providers/wechat.rs sources/providers/zhihu.rs sources/providers/bilibili.rs sources/providers/bilibili_collection.rs sources/providers/xiaohongshu.rs sources/providers/douyin.rs sources/providers/browser.rs sources/providers/conversations.rs sources/providers/doubao.rs sources/providers/kimi.rs sources/providers/chatgpt.rs sources/providers/local_files.rs sources/providers/first_party.rs processing/mod.rs processing/registry.rs processing/local_extract.rs processing/bailian_cli.rs processing/bailian_api.rs processing/semantic_digest.rs views/mod.rs views/dense_expression.rs views/datasette.rs views/obsidian.rs views/exports.rs views/sublibrary.rs views/output.rs views/manifest.rs backup/mod.rs backup/sqlite_snapshot.rs backup/restic.rs backup/manifest.rs tools/mod.rs tools/command_runner.rs tools/yt_dlp.rs security/mod.rs security/secrets.rs security/privacy.rs
'@
    '04_babata_cli' = @'
main.rs app.rs render.rs commands/mod.rs commands/data.rs commands/capabilities.rs commands/collector.rs commands/capture.rs commands/workspace.rs commands/knowledge.rs commands/process.rs commands/explore.rs commands/sublibraries.rs commands/views.rs commands/outputs.rs commands/routes.rs commands/ops.rs
'@
    '05_babata_local_api' = @'
lib.rs main.rs app.rs state.rs auth.rs error.rs requests.rs responses.rs routes/mod.rs routes/collector.rs routes/capture.rs routes/workspace.rs routes/process.rs routes/explore.rs routes/outputs.rs routes/health.rs
'@
    '06_babata_worker' = @'
main.rs app.rs runner.rs lease.rs shutdown.rs metrics.rs
'@
}

$expectedRust = foreach ($crate in $inventory.Keys) {
    $files = $inventory[$crate] -split '\s+' | Where-Object { $_ }
    foreach ($file in $files) { "$crate/src/$file" }
    $actualCount = (Get-ChildItem -File -Recurse (Join-Path $app "$crate/src") -Filter '*.rs').Count
    if ($actualCount -ne $files.Count) {
        throw "$crate expected $($files.Count) Rust files, found $actualCount"
    }
}
$actualRust = Get-ChildItem -File -Recurse $app -Filter '*.rs' |
    Where-Object { $_.FullName -match '[\\/]src[\\/]' } |
    ForEach-Object { $_.FullName.Substring($app.Length + 1).Replace('\', '/') }
$difference = Compare-Object ($expectedRust | Sort-Object) ($actualRust | Sort-Object)
if ($difference) { throw "P2 Rust inventory mismatch: $($difference | Out-String)" }
if ($actualRust.Count -ne 153) { throw "Expected 153 current Rust source files, found $($actualRust.Count)" }

$required = @'
08_adapters/01_browser_extension/package.json
08_adapters/01_browser_extension/tsconfig.json
08_adapters/01_browser_extension/manifest.json
08_adapters/01_browser_extension/src/index.ts
08_adapters/01_browser_extension/src/capture.ts
08_adapters/01_browser_extension/src/transport.ts
08_adapters/01_browser_extension/src/types.ts
08_adapters/02_python_bridge/pyproject.toml
08_adapters/02_python_bridge/src/babata_adapter/__init__.py
08_adapters/02_python_bridge/src/babata_adapter/runner.py
08_adapters/02_python_bridge/src/babata_adapter/envelope.py
02_skills/00_specs/01_capture.md
02_skills/00_specs/02_process.md
02_skills/00_specs/03_workspace.md
02_skills/00_specs/04_explore.md
02_skills/00_specs/05_routes.md
02_skills/00_specs/06_ops.md
02_skills/00_specs/07_knowledge.md
02_skills/00_specs/08_sublibraries.md
02_skills/00_specs/09_outputs.md
03_migrations/00_REGISTRY.md
04_tests/01_architecture/README.md
04_tests/02_contract/README.md
04_tests/03_integration/README.md
04_tests/04_end_to_end/README.md
04_tests/05_fixtures/README.md
06_config/data-root.example.toml
06_config/app.example.toml
06_config/routes.example.toml
06_config/providers.example.toml
06_config/pipelines.example.toml
06_config/views.example.toml
06_config/privacy.example.toml
06_config/backup.example.toml
'@ -split '\s+' | Where-Object { $_ }
foreach ($relative in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $repo $relative))) {
        throw "Missing P2 peripheral file: $relative"
    }
}

Write-Output 'P2 skeleton inventory passed: 137-file P2 baseline plus 16 post-P2 activation files, 6 crates, and corrected peripheral skeletons.'
