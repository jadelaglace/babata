[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$SourceMapPath,
    [Parameter(Mandatory=$true)][string]$C1BDecisionPath,
    [Parameter(Mandatory=$true)][string]$C1BRoot,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe'),
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' }
$env:BABATA_DATA_HOME = [IO.Path]::GetFullPath($DataHome)
$exe = (Get-Item -LiteralPath $BabataExe).FullName
$sourceMapPathResolved = (Get-Item -LiteralPath $SourceMapPath).FullName
$decisionPathResolved = (Get-Item -LiteralPath $C1BDecisionPath).FullName
$c1bRootResolved = (Get-Item -LiteralPath $C1BRoot).FullName
$staging = [IO.Path]::GetFullPath($StagingRoot)
if ((Test-Path -LiteralPath $staging) -and -not $Resume) {
    throw "Use a fresh C1B registration staging root or pass -Resume: $staging"
}
New-Item -ItemType Directory -Path $staging -Force | Out-Null
$resultRoot = Join-Path $staging 'results'
New-Item -ItemType Directory -Path $resultRoot -Force | Out-Null

$rawDb = Join-Path $env:BABATA_DATA_HOME '01_raw\index\raw.sqlite'
$derivedDb = Join-Path $env:BABATA_DATA_HOME '02_derived\index\derived.sqlite'
foreach ($path in @($rawDb, $derivedDb)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing Babata database: $path" }
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Escape-Sql([string]$Value) { return $Value.Replace("'", "''") }

function Invoke-SqliteJson([string]$Database, [string]$Sql) {
    $output = & sqlite3 -json $Database $Sql
    if ($LASTEXITCODE -ne 0) { throw "sqlite read failed: $Sql" }
    $text = ($output -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text) -or $text -eq '[]') { return @() }
    return @($text | ConvertFrom-Json)
}

function Invoke-BabataJson([string[]]$Arguments) {
    $output = & $exe --json @Arguments
    if ($LASTEXITCODE -ne 0) { throw "babata command failed: $($Arguments -join ' ')" }
    return (($output -join "`n") | ConvertFrom-Json)
}

function Assert-Equal([string]$Actual, [string]$Expected, [string]$Label) {
    if ($Actual -cne $Expected) { throw "$Label mismatch: expected '$Expected', got '$Actual'" }
}

function Get-MediaType([string]$Path) {
    switch ([IO.Path]::GetExtension($Path).ToLowerInvariant()) {
        '.png' { return 'image/png' }
        '.jpg' { return 'image/jpeg' }
        '.jpeg' { return 'image/jpeg' }
        default { throw "Unsupported retained C1B media type: $Path" }
    }
}

function Get-SourceLocator($Media) {
    $locator = [ordered]@{}
    foreach ($name in @('page', 'time_seconds', 'percentage', 'crop')) {
        if ($Media.PSObject.Properties[$name]) { $locator[$name] = $Media.$name }
    }
    return $locator
}

function Find-ActiveDerivative(
    [string]$RevisionId,
    [string]$Kind,
    [string]$Model,
    [string]$OutputSha256,
    [string]$InputAssetId,
    [string]$InputSha256
) {
    $sql = @"
SELECT p.run_id,d.derivative_id,d.output_sha256,d.logical_path
FROM process_runs p
JOIN derivatives d ON d.run_id=p.run_id
WHERE p.input_revision_id='$(Escape-Sql $RevisionId)'
  AND p.state='succeeded'
  AND p.invalidated_at IS NULL
  AND p.target_kind='$(Escape-Sql $Kind)'
  AND p.tool_or_model='$(Escape-Sql $Model)'
  AND p.input_asset_id='$(Escape-Sql $InputAssetId)'
  AND p.input_sha256='$(Escape-Sql $InputSha256)'
  AND d.output_sha256='$(Escape-Sql $OutputSha256)'
ORDER BY p.created_at;
"@
    $rows = @(Invoke-SqliteJson $derivedDb $sql)
    if ($rows.Count -gt 1) {
        throw "Multiple active $Model derivatives for revision $RevisionId and output $OutputSha256"
    }
    return $rows
}

function Confirm-RegisteredRun(
    [string]$RunId,
    [string]$DerivativeId,
    [string]$RevisionId,
    [string]$ItemId,
    [string]$AssetId,
    [string]$InputSha256,
    [string]$Kind,
    [string]$Model,
    [string]$OutputSha256
) {
    $shown = Invoke-BabataJson @('process','show-run','--run',$RunId)
    Assert-Equal ([string]$shown.run.id) $RunId 'run id'
    Assert-Equal ([string]$shown.run.input_revision_id) $RevisionId 'run revision'
    Assert-Equal ([string]$shown.run.input_item_id) $ItemId 'run item'
    Assert-Equal ([string]$shown.run.input_asset_id) $AssetId 'run input asset'
    Assert-Equal ([string]$shown.run.input_sha256) $InputSha256 'run input hash'
    Assert-Equal ([string]$shown.run.target_kind) $Kind 'run target kind'
    Assert-Equal ([string]$shown.run.tool_or_model) $Model 'run model/tool'
    Assert-Equal ([string]$shown.run.state) 'succeeded' 'run state'
    if ($null -ne $shown.run.invalidated_at) { throw "Run is invalidated: $RunId" }
    $derivatives = @($shown.derivatives)
    if ($derivatives.Count -ne 1) { throw "Expected one derivative for run $RunId" }
    $derivative = $derivatives[0]
    Assert-Equal ([string]$derivative.id) $DerivativeId 'derivative id'
    Assert-Equal ([string]$derivative.kind) $Kind 'derivative kind'
    Assert-Equal ([string]$derivative.input_asset_id) $AssetId 'derivative input asset'
    Assert-Equal ([string]$derivative.output_sha256) $OutputSha256 'derivative output hash'
    $logicalPath = [string]$derivative.logical_path
    if (-not $logicalPath.StartsWith('02_derived/files/sha256/', [StringComparison]::Ordinal)) {
        throw "Derivative is outside managed C1 storage: $logicalPath"
    }
    $managedPath = Join-Path $env:BABATA_DATA_HOME ($logicalPath.Replace('/', '\'))
    if (-not (Test-Path -LiteralPath $managedPath -PathType Leaf)) { throw "Managed C1 file missing: $managedPath" }
    Assert-Equal (Get-Sha256 $managedPath) $OutputSha256 'managed C1 output hash'
    return [ordered]@{
        run_id = $RunId
        derivative_id = $DerivativeId
        output_sha256 = $OutputSha256
        logical_path = $logicalPath
    }
}

$sourceMap = Get-Content -LiteralPath $sourceMapPathResolved -Raw -Encoding utf8 | ConvertFrom-Json
$decisions = @(Get-Content -LiteralPath $decisionPathResolved -Raw -Encoding utf8 | ConvertFrom-Json)
$sourceItems = @{}
foreach ($item in @($sourceMap.chunks.items)) { $sourceItems[[string]$item.module_id] = $item }
if ($sourceItems.Count -ne 37 -or $decisions.Count -ne 37) {
    throw 'Finance C1B registration requires exactly 37 source-map items and 37 decisions'
}

$ledgerRows = @()
$registeredMedia = 0
$reusedMedia = 0
$registeredDecisions = 0
$reusedDecisions = 0

foreach ($decision in $decisions) {
    $moduleId = [string]$decision.module_id
    $sourceItem = $sourceItems[$moduleId]
    if ($null -eq $sourceItem) { throw "Source-map item missing for module $moduleId" }
    foreach ($property in @('c0_item_id','c0_revision_id','c0_asset_id','c0_asset_sha256')) {
        Assert-Equal ([string]$decision.$property) ([string]$sourceItem.$property) "module $moduleId $property"
    }

    $itemId = [string]$decision.c0_item_id
    $revisionId = [string]$decision.c0_revision_id
    $assetId = [string]$decision.c0_asset_id
    $assetSha = ([string]$decision.c0_asset_sha256).ToLowerInvariant()
    $c0Sql = @"
SELECT r.revision_id,r.item_id,r.state AS revision_state,a.asset_id,a.sha256,a.state AS asset_state
FROM revisions r
JOIN assets a ON a.revision_id=r.revision_id
WHERE r.revision_id='$(Escape-Sql $revisionId)'
  AND r.item_id='$(Escape-Sql $itemId)'
  AND a.asset_id='$(Escape-Sql $assetId)'
  AND a.sha256='$(Escape-Sql $assetSha)';
"@
    $c0 = @(Invoke-SqliteJson $rawDb $c0Sql)
    if ($c0.Count -ne 1 -or $c0[0].revision_state -ne 'ready' -or $c0[0].asset_state -ne 'ready') {
        throw "C0 identity is not uniquely ready for module $moduleId"
    }

    $textPath = Join-Path $c1bRootResolved ([string]$decision.c1b_text_path -replace '/', '\')
    if (-not (Test-Path -LiteralPath $textPath -PathType Leaf)) { throw "C1B text missing: $textPath" }
    $c1Sha = (Get-Sha256 $textPath)
    Assert-Equal $c1Sha ([string]$decision.c1_sha256) "module $moduleId C1 text hash"
    $c1Sql = @"
SELECT p.run_id,d.derivative_id,d.output_sha256,d.logical_path,p.target_kind
FROM process_runs p
JOIN derivatives d ON d.run_id=p.run_id
WHERE p.input_revision_id='$(Escape-Sql $revisionId)'
  AND p.state='succeeded'
  AND p.invalidated_at IS NULL
  AND d.output_sha256='$(Escape-Sql $c1Sha)'
ORDER BY p.created_at;
"@
    $c1Rows = @(Invoke-SqliteJson $derivedDb $c1Sql)
    if ($c1Rows.Count -ne 1) { throw "Expected one active complete C1 derivative for module $moduleId and hash $c1Sha" }
    $c1Evidence = $c1Rows[0]
    $c1ManagedPath = Join-Path $env:BABATA_DATA_HOME ([string]$c1Evidence.logical_path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $c1ManagedPath -PathType Leaf)) { throw "Managed complete C1 file missing: $c1ManagedPath" }
    Assert-Equal (Get-Sha256 $c1ManagedPath) $c1Sha "module $moduleId managed complete C1 hash"

    $mediaRegistrations = @()
    foreach ($media in @($decision.retained_media)) {
        $mediaPath = Join-Path $c1bRootResolved ([string]$media.path -replace '/', '\')
        if (-not (Test-Path -LiteralPath $mediaPath -PathType Leaf)) { throw "C1B media missing: $mediaPath" }
        $mediaSha = Get-Sha256 $mediaPath
        Assert-Equal $mediaSha ([string]$media.sha256) "module $moduleId retained-media hash"
        if ($media.PSObject.Properties['bytes'] -and [long]$media.bytes -ne (Get-Item -LiteralPath $mediaPath).Length) {
            throw "Module $moduleId retained-media byte count mismatch: $mediaPath"
        }
        $locator = Get-SourceLocator $media
        $paramsObject = [ordered]@{
            schema = 'babata.finance-c1b-media/v1'
            adapter = 'deterministic_local'
            course = '25春 MBAO5406 财务管理'
            module_id = $moduleId
            source_variant = 'c1b'
            source_locator = $locator
            processing = @($media.processing)
            selection_review = [ordered]@{
                role = [string]$media.role
                reason = if ($media.PSObject.Properties['review_reason']) { [string]$media.review_reason } else { $null }
                reviewer = if (@($media.processing) -match 'Qwen Vision') { 'qianwen_skill' } else { 'deterministic_local' }
            }
            complete_c1_derivative_id = [string]$c1Evidence.derivative_id
            complete_c1_sha256 = $c1Sha
        }
        $paramsJson = $paramsObject | ConvertTo-Json -Depth 12 -Compress
        $existing = @(Find-ActiveDerivative $revisionId 'key_frame' 'finance-c1b-media-extractor' $mediaSha $assetId $assetSha)
        if ($existing.Count -eq 1) {
            $runId = [string]$existing[0].run_id
            $derivativeId = [string]$existing[0].derivative_id
            $registrationState = 'reused'
            $reusedMedia++
        } else {
            $lossNotes = (@($media.loss_notes) -join '; ')
            $registered = Invoke-BabataJson @(
                'process','register','--pipeline','agent_import',
                '--revision',$revisionId,'--item',$itemId,
                '--kind','key_frame','--provider','local_extract',
                '--model','finance-c1b-media-extractor','--tool-version','1.0.0',
                '--input-sha256',$assetSha,'--input-asset-id',$assetId,
                '--output-file',$mediaPath,'--media-type',(Get-MediaType $mediaPath),
                '--language','zh','--params-json',$paramsJson,'--loss-notes',$lossNotes
            )
            $runId = [string]$registered.run_id
            $derivativeId = [string]$registered.derivative_id
            $registrationState = 'registered'
            $registeredMedia++
        }
        $confirmed = Confirm-RegisteredRun $runId $derivativeId $revisionId $itemId $assetId $assetSha 'key_frame' 'finance-c1b-media-extractor' $mediaSha
        $mediaRegistrations += [ordered]@{
            excerpt_sha256 = $mediaSha
            output_sha256 = $confirmed.output_sha256
            source_locator = $locator
            role = [string]$media.role
            review_reason = if ($media.PSObject.Properties['review_reason']) { [string]$media.review_reason } else { $null }
            processing = @($media.processing)
            loss_notes = @($media.loss_notes)
            run_id = $confirmed.run_id
            derivative_id = $confirmed.derivative_id
            logical_path = $confirmed.logical_path
            registration = $registrationState
        }
    }

    $decisionDocument = [ordered]@{
        schema = 'babata.finance-c1b-essence/v1'
        course = '25春 MBAO5406 财务管理'
        module_id = $moduleId
        title = [string]$decision.title
        module_type = [string]$decision.module_type
        source = [ordered]@{
            item_id = $itemId
            revision_id = $revisionId
            asset_id = $assetId
            asset_sha256 = $assetSha
        }
        complete_c1 = [ordered]@{
            run_id = [string]$c1Evidence.run_id
            derivative_id = [string]$c1Evidence.derivative_id
            sha256 = $c1Sha
            logical_path = [string]$c1Evidence.logical_path
            reused_without_reprocessing = $true
        }
        essence_judgment = [ordered]@{
            text_sufficient = [bool]$decision.text_sufficient
            retained_modalities = @($decision.retained_modalities)
            decision_basis = [string]$decision.decision_basis
            audio = $decision.audio_decision
            video = $decision.video_decision
            attachment = $decision.attachment_decision
        }
        retained_media = @($mediaRegistrations | ForEach-Object {
            [ordered]@{
                excerpt_sha256 = $_.excerpt_sha256
                source_locator = $_.source_locator
                role = $_.role
                review_reason = $_.review_reason
                processing = $_.processing
                loss_notes = $_.loss_notes
                run_id = $_.run_id
                derivative_id = $_.derivative_id
                logical_path = $_.logical_path
            }
        })
        handoff = [ordered]@{
            status = 'registered'
            c2b_may_consume = $true
            external_original_reread_required = $false
        }
    }
    $moduleRoot = Join-Path $resultRoot "module-$moduleId"
    New-Item -ItemType Directory -Path $moduleRoot -Force | Out-Null
    $decisionFile = Join-Path $moduleRoot 'c1b-essence.json'
    $decisionDocument | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $decisionFile -Encoding utf8
    $decisionSha = Get-Sha256 $decisionFile
    $decisionParams = [ordered]@{
        schema = 'babata.finance-c1b-essence-registration/v1'
        adapter = 'deterministic_local'
        course = '25春 MBAO5406 财务管理'
        module_id = $moduleId
        source_variant = 'c1b'
        complete_c1_derivative_id = [string]$c1Evidence.derivative_id
        retained_media_derivative_ids = @($mediaRegistrations | ForEach-Object { $_.derivative_id })
    } | ConvertTo-Json -Depth 10 -Compress
    $decisionExisting = @(Find-ActiveDerivative $revisionId 'structured_result' 'finance-c1b-essence-registrar' $decisionSha $assetId $assetSha)
    if ($decisionExisting.Count -eq 1) {
        $decisionRunId = [string]$decisionExisting[0].run_id
        $decisionDerivativeId = [string]$decisionExisting[0].derivative_id
        $decisionRegistrationState = 'reused'
        $reusedDecisions++
    } else {
        $registeredDecision = Invoke-BabataJson @(
            'process','register','--pipeline','agent_import',
            '--revision',$revisionId,'--item',$itemId,
            '--kind','structured_result','--provider','local_extract',
            '--model','finance-c1b-essence-registrar','--tool-version','1.0.0',
            '--input-sha256',$assetSha,'--input-asset-id',$assetId,
            '--json-file',$decisionFile,'--output-file',$decisionFile,
            '--media-type','application/json','--language','zh',
            '--params-json',$decisionParams,
            '--loss-notes','C1B 保存完整 C1 引用和必要模态片段；不复制外部主权原件，也不替代 C2B 语义组织。'
        )
        $decisionRunId = [string]$registeredDecision.run_id
        $decisionDerivativeId = [string]$registeredDecision.derivative_id
        $decisionRegistrationState = 'registered'
        $registeredDecisions++
    }
    $confirmedDecision = Confirm-RegisteredRun $decisionRunId $decisionDerivativeId $revisionId $itemId $assetId $assetSha 'structured_result' 'finance-c1b-essence-registrar' $decisionSha
    $ledgerRows += [ordered]@{
        module_id = $moduleId
        title = [string]$decision.title
        source_item_id = $itemId
        source_revision_id = $revisionId
        source_asset_id = $assetId
        source_asset_sha256 = $assetSha
        complete_c1 = [ordered]@{
            run_id = [string]$c1Evidence.run_id
            derivative_id = [string]$c1Evidence.derivative_id
            output_sha256 = $c1Sha
            logical_path = [string]$c1Evidence.logical_path
        }
        decision_registration = [ordered]@{
            run_id = $confirmedDecision.run_id
            derivative_id = $confirmedDecision.derivative_id
            output_sha256 = $confirmedDecision.output_sha256
            logical_path = $confirmedDecision.logical_path
            registration = $decisionRegistrationState
        }
        media_registrations = $mediaRegistrations
    }
}

$allMedia = @($ledgerRows | ForEach-Object { @($_.media_registrations) })
if ($ledgerRows.Count -ne 37 -or $allMedia.Count -ne 76) {
    throw "C1B registration coverage mismatch: decisions=$($ledgerRows.Count), media=$($allMedia.Count)"
}
if (@($ledgerRows | ForEach-Object { $_.decision_registration.derivative_id } | Sort-Object -Unique).Count -ne 37) {
    throw 'C1B decision derivative identities are not unique'
}
if (@($allMedia | ForEach-Object { $_.derivative_id } | Sort-Object -Unique).Count -ne 76) {
    throw 'C1B media derivative identities are not unique'
}

$ledger = [ordered]@{
    schema = 'babata.finance-c1b-registration/v1'
    course = '25春 MBAO5406 财务管理'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    status = 'registered'
    source_map = $sourceMapPathResolved
    source_map_sha256 = Get-Sha256 $sourceMapPathResolved
    decision_source = $decisionPathResolved
    decision_source_sha256 = Get-Sha256 $decisionPathResolved
    coverage = [ordered]@{
        modules = 37
        complete_c1_reused = 37
        essence_decisions_registered = 37
        retained_media_registered = 76
    }
    registrations = $ledgerRows
}
$ledgerPath = Join-Path $staging 'c1b-registration-ledger.json'
$ledger | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $ledgerPath -Encoding utf8
$ledgerSha = Get-Sha256 $ledgerPath

$manifest = [ordered]@{
    schema = 'babata.finance-c1b-registration-manifest/v1'
    task = Split-Path $staging -Leaf
    status = 'registered'
    course = '25春 MBAO5406 财务管理'
    ledger = $ledgerPath
    ledger_sha256 = $ledgerSha
    decisions = 37
    media = 76
    registered_decisions = $registeredDecisions
    reused_decisions = $reusedDecisions
    registered_media = $registeredMedia
    reused_media = $reusedMedia
    c0_mutations = 0
    complete_c1_reprocessed = 0
}
$manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $staging 'manifest.json') -Encoding utf8

@(
    '# 财务管理 C1B 正式登记报告',
    '',
    '- 状态：registered',
    '- 范围：37/37 个模块',
    '- 完整 C1：37/37 复用正式派生物，未重跑',
    '- 本质判断：37/37 正式 structured_result',
    '- 必要视觉片段：76/76 正式 key_frame',
    "- 本次新登记：判断 $registeredDecisions，媒体 $registeredMedia",
    "- 幂等复用：判断 $reusedDecisions，媒体 $reusedMedia",
    '- C0 改动：0',
    '- 外部原件重读：0',
    "- 正式账本：$ledgerPath",
    "- 账本 SHA-256：$ledgerSha"
) | Set-Content -LiteralPath (Join-Path $staging 'REPORT.md') -Encoding utf8

Write-Output "ledger=$ledgerPath status=registered decisions=37 media=76 registered_decisions=$registeredDecisions reused_decisions=$reusedDecisions registered_media=$registeredMedia reused_media=$reusedMedia"
