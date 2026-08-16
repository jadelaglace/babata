[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [string]$CoverageAuditPath,
    [string]$VisualPlanPath,
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [int]$ChunkCharLimit = 180000,
    [string]$MediaExtractor = (Join-Path $PSScriptRoot 'extract-mba-course-c1b-media.py'),
    [string]$TemplateBuilder = (Join-Path $PSScriptRoot 'build-template-preserving-c2b.ps1')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' }
$data = [IO.Path]::GetFullPath($DataHome)
$env:BABATA_DATA_HOME = $data
$planPath = (Get-Item -LiteralPath $CoursePlanPath).FullName
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($plan.schema -ne 'babata.mba-course-c2b-plan/v1') { throw 'Unsupported MBA course plan schema' }
$planStatus = [string]$plan.output_status
if ($planStatus -ne 'pending_user_acceptance') { throw 'New MBA course plans must remain pending_user_acceptance before direct user approval' }
$course = [string]$plan.course
$expected = [int]$plan.expected_modules
if ([string]::IsNullOrWhiteSpace($course) -or $expected -lt 1) { throw 'Course plan requires course and expected_modules' }

if ([string]::IsNullOrWhiteSpace($CoverageAuditPath)) {
    $CoverageAuditPath = Join-Path $data '04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\coverage\c1-coverage-audit.json'
}
$auditPath = (Get-Item -LiteralPath $CoverageAuditPath).FullName
$staging = [IO.Path]::GetFullPath($StagingRoot)
if (Test-Path -LiteralPath $staging) { throw "Use a fresh MBA course staging root: $staging" }

$rawDb = Join-Path $data '01_raw\index\raw.sqlite'
$derivedDb = Join-Path $data '02_derived\index\derived.sqlite'
foreach ($path in @($rawDb,$derivedDb,$MediaExtractor,$TemplateBuilder)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing required input: $path" }
}

function Escape-Sql([string]$Value) { return $Value.Replace("'", "''") }
function Sql-Rows([string]$Database,[string]$Sql) {
    $raw = & sqlite3 -json $Database $Sql
    if ($LASTEXITCODE -ne 0) { throw "sqlite read failed: $Sql" }
    $text = ($raw -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text) -or $text -eq '[]') { return @() }
    return @($text | ConvertFrom-Json)
}
function Hash([string]$Path) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

$audit = Get-Content -LiteralPath $auditPath -Raw -Encoding utf8 | ConvertFrom-Json
$courseRows = @($audit.items | Where-Object { $_.course -ceq $course })
if ($courseRows.Count -ne $expected) { throw "Course denominator mismatch: expected $expected, found $($courseRows.Count)" }
$notCovered = @($courseRows | Where-Object { $_.coverage_status -ne 'covered' })
if ($notCovered.Count) { throw "Course has non-covered C1 inputs: $($notCovered.module_id -join ', ')" }

$chapterByModule = @{}
foreach ($chapter in @($plan.chapters)) {
    foreach ($module in @($chapter.modules)) {
        $key = [string]$module
        if ($chapterByModule.ContainsKey($key)) { throw "Module appears in multiple chapters: $key" }
        $chapterByModule[$key] = [string]$chapter.note
    }
}
$courseIds = @($courseRows | ForEach-Object { [string]$_.module_id })
$unmapped = @($courseIds | Where-Object { -not $chapterByModule.ContainsKey($_) })
$extra = @($chapterByModule.Keys | Where-Object { $courseIds -notcontains $_ })
if ($unmapped.Count -or $extra.Count) {
    throw "Chapter mapping must cover the course exactly. unmapped=$($unmapped -join ',') extra=$($extra -join ',')"
}

# Delay mutation until the immutable inputs and exact course partition pass.
New-Item -ItemType Directory -Path $staging -Force | Out-Null

$items = @()
$sequence = 0
foreach ($row in $courseRows) {
    $sequence++
    $revision = Escape-Sql ([string]$row.revision_id)
    $activeKinds = @($row.active_kinds | ForEach-Object { [string]$_ })
    $preferredKinds = if ([string]$row.module_type -eq 'video') {
        @('transcript')
    } else {
        @('extracted_text','ocr_text','structured_result')
    }
    $preferredKind = @($preferredKinds | Where-Object { $activeKinds -contains $_ } | Select-Object -First 1)
    if ($preferredKind.Count -ne 1) {
        throw "Coverage audit has no supported active C1 kind for module $($row.module_id): $($activeKinds -join ',')"
    }
    $preferredKind = [string]$preferredKind[0]
    $candidates = @(Sql-Rows $derivedDb @"
SELECT p.run_id,p.target_kind,p.tool_or_model,d.derivative_id,d.output_sha256,d.logical_path
FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id
WHERE p.input_revision_id='$revision' AND p.state='succeeded' AND p.invalidated_at IS NULL
  AND p.target_kind='$(Escape-Sql $preferredKind)'
ORDER BY p.created_at;
"@)
    if ($candidates.Count -ne 1) { throw "Expected one active $preferredKind for module $($row.module_id), found $($candidates.Count)" }
    $c1 = $candidates[0]
    if ([string]::IsNullOrWhiteSpace([string]$c1.logical_path)) { throw "Managed C1 path missing for module $($row.module_id)" }
    $c1Path = Join-Path $data ([string]$c1.logical_path).Replace('/','\')
    if (-not (Test-Path -LiteralPath $c1Path -PathType Leaf)) { throw "Managed C1 file missing: $c1Path" }
    $c1Hash = Hash $c1Path
    if ($c1Hash -cne ([string]$c1.output_sha256).ToLowerInvariant()) { throw "Managed C1 hash mismatch for module $($row.module_id)" }

    $assetId = Escape-Sql ([string]$row.asset_id)
    $assets = @(Sql-Rows $rawDb "SELECT logical_path,sha256,state FROM assets WHERE asset_id='$assetId' AND revision_id='$revision';")
    if ($assets.Count -ne 1 -or $assets[0].state -ne 'ready') { throw "Managed C0 asset is not uniquely ready for module $($row.module_id)" }
    $c0Path = Join-Path $data ([string]$assets[0].logical_path).Replace('/','\')
    if (-not (Test-Path -LiteralPath $c0Path -PathType Leaf)) { throw "Managed C0 file missing: $c0Path" }
    $c0Hash = Hash $c0Path
    if ($c0Hash -cne ([string]$row.asset_sha256).ToLowerInvariant() -or $c0Hash -cne ([string]$assets[0].sha256).ToLowerInvariant()) {
        throw "Managed C0 hash mismatch for module $($row.module_id)"
    }

    $text = Get-Content -LiteralPath $c1Path -Raw -Encoding utf8
    if ([string]::IsNullOrWhiteSpace($text)) { throw "Complete C1 is empty for module $($row.module_id)" }
    $items += [ordered]@{
        sequence = $sequence
        module_id = [int]$row.module_id
        title = [string]$row.title
        module_type = [string]$row.module_type
        source_extension = ([string]$row.extension).ToLowerInvariant()
        phase = [string]$row.phase
        parent = [string]$row.website_path
        website_path = $c0Path
        external_source_reference = [string]$row.target_path
        c0_asset_sha256 = $c0Hash
        c0_item_id = [string]$row.item_id
        c0_revision_id = [string]$row.revision_id
        c0_asset_id = [string]$row.asset_id
        c1_run_id = [string]$c1.run_id
        c1_derivative_id = [string]$c1.derivative_id
        c1_path = $c1Path
        chars = $text.Length
        c1_sha256 = $c1Hash
        chapter_note = $chapterByModule[[string]$row.module_id]
    }
}

$chunks = @()
$chunkItems = @()
$chunkChars = 0
$chunkIndex = 0
foreach ($item in $items) {
    if ($chunkItems.Count -and ($chunkChars + [int]$item.chars) -gt $ChunkCharLimit) {
        $chunkIndex++
        $chunks += [ordered]@{ chunk_id=('{0}-c1-{1:d2}' -f $plan.course_key,$chunkIndex); chars=$chunkChars; items=$chunkItems }
        $chunkItems = @(); $chunkChars = 0
    }
    $chunkItems += $item; $chunkChars += [int]$item.chars
}
if ($chunkItems.Count) {
    $chunkIndex++
    $chunks += [ordered]@{ chunk_id=('{0}-c1-{1:d2}' -f $plan.course_key,$chunkIndex); chars=$chunkChars; items=$chunkItems }
}
$sourceMap = [ordered]@{
    schema = 'babata.mba.c2-source-map/v1'
    course = $course
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    source = 'managed_complete_c1'
    expected_modules = $expected
    chunks = $chunks
}
$sourceMapPath = Join-Path $staging 'c1b-source-map.json'
$sourceMap | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $sourceMapPath -Encoding utf8

$template = Join-Path $staging 'semantic-template'
$moduleRoot = Join-Path $template '模块'
New-Item -ItemType Directory -Path $moduleRoot -Force | Out-Null
$index = @('---',"course: $course",'variant: c2b','status: staged_only','template_profile: semantic-obsidian/v1','---','',"# $($plan.short_name) 课程证据入口",'','## 课程章节')
foreach ($chapter in @($plan.chapters)) { $index += "- [[$([string]$chapter.note)]]" }
$index += @('','## 模块证据','- 完整模块文字由 builder 从 hash-checked C1 物化；该层只用于正式 handoff/重建证据。','')
$index -join "`n" | Set-Content -LiteralPath (Join-Path $template 'index.md') -Encoding utf8
foreach ($item in $items) {
    $note = @('---','babata_type: c2b_module_evidence',"module_id: $($item.module_id)","course: $course",'---','',"# $($item.title)",'','## 关联知识节点','',"- [[$($item.chapter_note)]]",'') -join "`n"
    Set-Content -LiteralPath (Join-Path $moduleRoot "M-$($item.module_id).md") -Value $note -Encoding utf8
}

$resolvedVisualPlan = $null
$extractorArgs = @($MediaExtractor, '--source-map', $sourceMapPath, '--staging-root', $staging)
if (-not [string]::IsNullOrWhiteSpace($VisualPlanPath)) {
    $resolvedVisualPlan = (Get-Item -LiteralPath $VisualPlanPath).FullName
    $extractorArgs += @('--visual-plan', $resolvedVisualPlan)
}
& python @extractorArgs
if ($LASTEXITCODE -ne 0) { throw 'C1B media extraction failed' }
$decisionPath = Join-Path $staging 'c1b\decisions.json'
$decisions = @(Get-Content -LiteralPath $decisionPath -Raw -Encoding utf8 | ConvertFrom-Json)
if ($decisions.Count -ne $expected) { throw "C1B decision count mismatch: $($decisions.Count)" }

$mediaRows = @()
foreach ($decision in $decisions) {
    foreach ($media in @($decision.retained_media)) {
        $source = Join-Path $staging ([string]$media.path).Replace('/','\')
        $locator = [ordered]@{}
        foreach ($name in @('page','time_seconds','percentage','crop')) {
            if ($media.PSObject.Properties[$name]) { $locator[$name] = $media.$name }
        }
        $mediaRows += [ordered]@{
            source_path = $source
            filename = "M-$($decision.module_id)-$([IO.Path]::GetFileName([string]$media.path))"
            modality = 'image'
            role = [string]$media.role
            source_module_id = [int]$decision.module_id
            source_locator = $locator
            processing = @($media.processing)
            loss_notes = @($media.loss_notes)
            target_notes = @("模块/M-$($decision.module_id).md")
        }
    }
}
$mediaPlan = [ordered]@{ schema='babata.c1b-media-plan/v1'; course=$course; media=$mediaRows }
$mediaPlanPath = Join-Path $staging 'media-plan.json'
$mediaPlan | ConvertTo-Json -Depth 15 | Set-Content -LiteralPath $mediaPlanPath -Encoding utf8

$evidenceBuild = Join-Path $staging 'template-preserving-evidence'
& $TemplateBuilder -SourceMapPath $sourceMapPath -TemplatePath $template -MediaPlanPath $mediaPlanPath -StagingRoot $evidenceBuild -CourseName $course
if ($LASTEXITCODE -ne 0) { throw 'Template-preserving C2B evidence build failed' }

$receipt = [ordered]@{
    schema='babata.mba-course-c1b-preparation/v1'; course=$course; status='staged_only'; expected_modules=$expected
    course_plan=$planPath; course_plan_sha256=Hash $planPath; output_status=$planStatus
    complete_c1=$items.Count; c1b_decisions=$decisions.Count; retained_media=$mediaRows.Count
    source_map=$sourceMapPath; source_map_sha256=Hash $sourceMapPath
    decisions=$decisionPath; decisions_sha256=Hash $decisionPath
    visual_plan=$resolvedVisualPlan
    visual_plan_sha256=$(if ($resolvedVisualPlan) { Hash $resolvedVisualPlan } else { $null })
    template_preserving_manifest=(Join-Path $evidenceBuild 'manifest.json')
    old_c2b_inputs=0; external_sovereign_original_reads=0
}
$receipt | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $staging 'manifest.json') -Encoding utf8
@("# $($plan.short_name) C1B 准备报告",'', '- 状态：staged_only', "- 完整 C1：$expected/$expected", "- C1B 判断：$expected/$expected", "- 必要视觉片段：$($mediaRows.Count)", '- 输入：managed C0/C1；旧 C2B 输入 0；外部主权原件直接读取 0', '- template-preserving builder：通过', '- 正式 C1B 登记：未开始') | Set-Content -LiteralPath (Join-Path $staging 'REPORT.md') -Encoding utf8
Write-Output "staged=$staging course=$course c1=$expected c1b=$($decisions.Count) media=$($mediaRows.Count)"
