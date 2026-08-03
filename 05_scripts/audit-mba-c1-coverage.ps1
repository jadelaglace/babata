[CmdletBinding()]
param(
    [string]$DataHome = 'D:\BabataData',
    [string]$WebsiteTree = 'D:\BabataRecovery\mba\gaodun-complete-website-tree',
    [string]$OutputDir = 'D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\coverage'
)

$ErrorActionPreference = 'Stop'

$manifestPath = Join-Path $WebsiteTree '_mba-assembly-manifest.json'
$rawDb = Join-Path $DataHome '01_raw\index\raw.sqlite'
$derivedDb = Join-Path $DataHome '02_derived\index\derived.sqlite'

foreach ($path in @($manifestPath, $rawDb, $derivedDb)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required input: $path"
    }
}

if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
    throw 'sqlite3 is required for the read-only coverage query.'
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
$linkedEntries = @($manifest.entries | Where-Object status -eq 'linked')
if ($linkedEntries.Count -ne 763) {
    throw "Expected 763 linked website entries, found $($linkedEntries.Count)."
}

$entryByModule = @{}
foreach ($entry in $linkedEntries) {
    $key = [string]$entry.id
    if ($entryByModule.ContainsKey($key)) {
        throw "Duplicate website module id: $key"
    }
    $entryByModule[$key] = $entry
}

$rawSqlPath = $rawDb.Replace('\', '/').Replace("'", "''")
$query = @"
ATTACH '$rawSqlPath' AS raw;
SELECT
  i.source_native_id,
  i.item_id,
  r.revision_id,
  a.asset_id,
  a.sha256,
  a.byte_size,
  a.media_type,
  a.original_filename,
  COUNT(DISTINCT CASE
    WHEN pr.state = 'succeeded' AND pr.invalidated_at IS NULL THEN pr.run_id
  END) AS active_runs,
  GROUP_CONCAT(DISTINCT CASE
    WHEN pr.state = 'succeeded' AND pr.invalidated_at IS NULL THEN d.kind
  END) AS active_kinds
FROM raw.items i
JOIN raw.sources s ON s.source_id = i.source_id
JOIN raw.revisions r ON r.item_id = i.item_id AND r.state = 'ready'
JOIN raw.assets a ON a.revision_id = r.revision_id AND a.state = 'ready'
LEFT JOIN process_runs pr ON pr.input_item_id = i.item_id
LEFT JOIN derivatives d ON d.run_id = pr.run_id
WHERE s.provider = 'gaodun_mba'
GROUP BY i.item_id
ORDER BY CAST(SUBSTR(i.source_native_id, 8) AS INTEGER);
"@

$jsonLines = & sqlite3 -json $derivedDb $query
if ($LASTEXITCODE -ne 0) {
    throw "sqlite3 coverage query failed with exit code $LASTEXITCODE."
}
$json = $jsonLines -join [Environment]::NewLine
$rows = $json | ConvertFrom-Json
$rows = @($rows)
if ($rows.Count -ne 763) {
    throw "Expected 763 ready database items, found $($rows.Count)."
}

$ledger = foreach ($row in $rows) {
    $moduleId = ([string]$row.source_native_id) -replace '^module:', ''
    $entry = $entryByModule[$moduleId]
    if ($null -eq $entry) {
        throw "Database item has no website manifest entry: $($row.source_native_id)"
    }

    $extension = [IO.Path]::GetExtension([string]$entry.targetPath).ToLowerInvariant()
    $activeKinds = if ([string]::IsNullOrWhiteSpace([string]$row.active_kinds)) {
        @()
    } else {
        @(([string]$row.active_kinds).Split(',') | Sort-Object -Unique)
    }

    $requiredKinds = switch ([string]$entry.moduleType) {
        'video' { @('transcript') }
        'courseware' {
            switch ($extension) {
                '.png' { @('ocr_text', 'visual_description') }
                '.zip' { @('structured_result', 'extracted_text') }
                default { @('extracted_text', 'ocr_text', 'structured_result') }
            }
        }
        default { @('structured_result') }
    }

    $satisfyingKinds = @($activeKinds | Where-Object { $_ -in $requiredKinds })
    $coverageStatus = if ($satisfyingKinds.Count -gt 0) {
        'covered'
    } elseif ([int]$row.active_runs -gt 0) {
        'partial'
    } else {
        'pending'
    }

    $recommendedRoute = if ([string]$entry.moduleType -eq 'video') {
        'full_duration_asr'
    } else {
        switch ($extension) {
            '.png' { 'ocr_then_visual_if_needed' }
            '.zip' { 'local_archive_inventory_then_structure' }
            '.pdf' { 'local_text_then_ocr_if_textless' }
            '.docx' { 'local_docx_extract_then_optional_summary' }
            '.doc' { 'local_legacy_doc_extract_then_optional_summary' }
            '.xlsx' { 'local_workbook_structure_then_optional_summary' }
            '.pptx' { 'local_slide_text_then_optional_summary' }
            default { 'local_probe_then_select_adapter' }
        }
    }

    [pscustomobject]@{
        module_id = [int64]$moduleId
        item_id = [string]$row.item_id
        revision_id = [string]$row.revision_id
        asset_id = [string]$row.asset_id
        asset_sha256 = [string]$row.sha256
        byte_size = [int64]$row.byte_size
        media_type = [string]$row.media_type
        module_type = [string]$entry.moduleType
        extension = $extension
        course_order = [int]$entry.courseOrder
        course = [string]$entry.course
        phase = [string]$entry.phase
        title = [string]$entry.title
        website_path = @($entry.websitePath)
        target_path = [string]$entry.targetPath
        active_runs = [int]$row.active_runs
        active_kinds = $activeKinds
        required_any_kind = $requiredKinds
        coverage_status = $coverageStatus
        recommended_route = $recommendedRoute
    }
}

$summary = [ordered]@{
    schema = 'babata.mba.c1-coverage/v1'
    generated_at = (Get-Date).ToString('o')
    provider = 'gaodun_mba'
    total_items = $ledger.Count
    covered_items = @($ledger | Where-Object coverage_status -eq 'covered').Count
    partial_items = @($ledger | Where-Object coverage_status -eq 'partial').Count
    pending_items = @($ledger | Where-Object coverage_status -eq 'pending').Count
    items_with_active_runs = @($ledger | Where-Object active_runs -gt 0).Count
    active_runs = ($ledger | Measure-Object active_runs -Sum).Sum
    by_module_type = @(
        $ledger | Group-Object module_type | Sort-Object Name | ForEach-Object {
            [ordered]@{
                module_type = $_.Name
                total = $_.Count
                covered = @($_.Group | Where-Object coverage_status -eq 'covered').Count
                partial = @($_.Group | Where-Object coverage_status -eq 'partial').Count
                pending = @($_.Group | Where-Object coverage_status -eq 'pending').Count
            }
        }
    )
    by_course = @(
        $ledger | Group-Object course | Sort-Object { ($_.Group | Select-Object -First 1).course_order } |
            ForEach-Object {
                $first = $_.Group | Select-Object -First 1
                [ordered]@{
                    course_order = $first.course_order
                    course = $_.Name
                    total = $_.Count
                    covered = @($_.Group | Where-Object coverage_status -eq 'covered').Count
                    partial = @($_.Group | Where-Object coverage_status -eq 'partial').Count
                    pending = @($_.Group | Where-Object coverage_status -eq 'pending').Count
                }
            }
    )
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$audit = [ordered]@{ summary = $summary; items = @($ledger) }
$audit | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $OutputDir 'c1-coverage-audit.json') -Encoding utf8
$ledger | Select-Object module_id, course_order, course, phase, title, module_type, extension,
    coverage_status, active_runs, @{n='active_kinds';e={$_.active_kinds -join '|'}}, recommended_route,
    item_id, revision_id, asset_id, asset_sha256, byte_size, target_path |
    Export-Csv -LiteralPath (Join-Path $OutputDir 'c1-coverage-ledger.csv') -NoTypeInformation -Encoding utf8

$courseRows = foreach ($course in $summary.by_course) {
    "| $($course.course_order) | $($course.course) | $($course.total) | $($course.covered) | $($course.partial) | $($course.pending) |"
}
$moduleRows = foreach ($module in $summary.by_module_type) {
    "| $($module.module_type) | $($module.total) | $($module.covered) | $($module.partial) | $($module.pending) |"
}
$report = @(
    '# Gaodun MBA C1 coverage',
    '',
    "Generated: $($summary.generated_at)",
    '',
    "- Total C0 items: $($summary.total_items)",
    "- Covered: $($summary.covered_items)",
    "- Partial: $($summary.partial_items)",
    "- Pending: $($summary.pending_items)",
    "- Items with active C1: $($summary.items_with_active_runs)",
    "- Active C1 runs: $($summary.active_runs)",
    '',
    '## By modality',
    '',
    '| Modality | Total | Covered | Partial | Pending |',
    '| --- | ---: | ---: | ---: | ---: |',
    $moduleRows,
    '',
    '## By course',
    '',
    '| Order | Course | Total | Covered | Partial | Pending |',
    '| ---: | --- | ---: | ---: | ---: | ---: |',
    $courseRows,
    '',
    'The JSON and CSV ledgers contain the authoritative item/revision/asset/hash binding and route for every item.'
) -join [Environment]::NewLine
$report | Set-Content -LiteralPath (Join-Path $OutputDir 'C1_COVERAGE.md') -Encoding utf8

$summary | ConvertTo-Json -Depth 6
