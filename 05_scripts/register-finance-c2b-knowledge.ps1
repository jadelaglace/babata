[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$SourceMapPath,
    [Parameter(Mandatory=$true)][string]$C1BDecisionPath,
    [Parameter(Mandatory=$true)][string]$C1BRoot,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [Parameter(Mandatory=$true)][string]$ManagementMapNodeId,
    [Parameter(Mandatory=$true)][string]$FinanceMapNodeId,
    [string]$BabataExe = (Join-Path $PSScriptRoot '..\01_app\target\debug\babata.exe'),
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' }
$env:BABATA_DATA_HOME = [IO.Path]::GetFullPath($DataHome)
$exe = (Get-Item -LiteralPath $BabataExe).FullName
$sourceMap = Get-Content -LiteralPath $SourceMapPath -Raw -Encoding utf8 | ConvertFrom-Json
$decisions = @(Get-Content -LiteralPath $C1BDecisionPath -Raw -Encoding utf8 | ConvertFrom-Json)
$c1b = (Get-Item -LiteralPath $C1BRoot).FullName
$staging = [IO.Path]::GetFullPath($StagingRoot)
if ((Test-Path -LiteralPath $staging) -and -not $Resume) { throw "Use a fresh semantic staging root or pass -Resume: $staging" }
New-Item -ItemType Directory -Path $staging -Force | Out-Null

$items = @{}
foreach ($item in @($sourceMap.chunks.items)) { $items[[string]$item.module_id] = $item }
if ($decisions.Count -ne 37 -or $items.Count -ne 37) { throw 'Finance registration requires 37 source items and 37 C1B decisions' }

$derivedDb = Join-Path $env:BABATA_DATA_HOME '02_derived\index\derived.sqlite'
$rawDb = Join-Path $env:BABATA_DATA_HOME '01_raw\index\raw.sqlite'
foreach ($path in @($derivedDb, $rawDb)) { if (-not (Test-Path -LiteralPath $path)) { throw "Missing Babata database: $path" } }

function Invoke-SqliteJson([string]$Database, [string]$Sql) {
    $raw = & sqlite3 -json $Database $Sql
    if ($LASTEXITCODE -ne 0) { throw "sqlite read failed: $Sql" }
    if ([string]::IsNullOrWhiteSpace(($raw -join ''))) { return @() }
    return @((($raw -join "`n") | ConvertFrom-Json))
}
function Escape-Sql([string]$Value) { return $Value.Replace("'", "''") }
function FirstStatement([string]$Text, [string]$Fallback) {
    $body = [regex]::Replace($Text, '(?m)^#{1,6}\s+.*$', '').Trim()
    $parts = @($body -split '(?<=[。！？])\s+' | Where-Object { $_.Trim().Length -ge 12 })
    $value = if ($parts.Count) { $parts[0].Trim() } else { $Fallback }
    if ($value.Length -gt 500) { $value = $value.Substring(0, 500) }
    return $value
}
function Outline([string]$Text, [string]$Title) {
    $headings = @([regex]::Matches($Text, '(?m)^#{1,4}\s+(.+)$') | ForEach-Object { $_.Groups[1].Value.Trim() } | Select-Object -Unique -First 12)
    if (-not $headings.Count) { $headings = @($Title) }
    return (($headings | ForEach-Object { "- $_" }) -join "`n")
}
function Invoke-BabataJson([string[]]$Arguments) {
    $output = & $exe --json @Arguments
    if ($LASTEXITCODE -ne 0) { throw "babata command failed: $($Arguments -join ' ')" }
    return ($output -join "`n") | ConvertFrom-Json
}

$chapterByModule = @{}
$chapterSpecs = @(
    @{ chapter='01-财务管理目标与财务高管职责'; modules=@('892115','892117','892119') },
    @{ chapter='02-营运资本管理'; modules=@('892121','892123','892125','892127','892129','892131','897613') },
    @{ chapter='03-项目投资评估'; modules=@('892135','892137','892139','892141','892143','892145','901333','972135') },
    @{ chapter='04-投资风险与不确定性'; modules=@('892153','892155') },
    @{ chapter='05-企业估值'; modules=@('892157','892159') },
    @{ chapter='06-融资决策与资本结构'; modules=@('892161','892163','892165','972531','972533') },
    @{ chapter='07-股利决策'; modules=@('892171','892173') },
    @{ chapter='08-并购与企业重组'; modules=@('892175','892177','892179','892181','892185','892187','976503','976505') }
)
foreach ($spec in $chapterSpecs) { foreach ($module in $spec.modules) { $chapterByModule[$module] = $spec.chapter } }

$ledgerRows = @()
$generatedAt = (Get-Date).ToUniversalTime().ToString('o')
foreach ($decision in $decisions) {
    $module = [string]$decision.module_id
    $item = $items[$module]
    if ($null -eq $item) { throw "Source map item missing for module $module" }
    $textPath = Join-Path $c1b ([string]$decision.c1b_text_path -replace '^c1b/', 'c1b/')
    if (-not (Test-Path -LiteralPath $textPath)) { throw "C1B text missing: $textPath" }
    $textHash = (Get-FileHash -LiteralPath $textPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($textHash -ne [string]$decision.c1_sha256) { throw "C1B text hash mismatch: $module" }

    $title = "财务管理｜$([string]$decision.title)"
    $existingSql = "SELECT semantic_id,suggestion_id FROM semantic_entries WHERE source_revision_id='$(Escape-Sql ([string]$decision.c0_revision_id))' AND author='c1b-full-text-semantic-materializer' ORDER BY created_at LIMIT 1;"
    $existing = @(Invoke-SqliteJson $rawDb $existingSql)
    if ($existing.Count) {
        $semanticId = [string]$existing[0].semantic_id
        $suggestionId = [string]$existing[0].suggestion_id
        $registrationState = 'reused'
    } else {
        $evidenceSql = "SELECT d.derivative_id,d.output_sha256 FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id WHERE p.input_revision_id='$(Escape-Sql ([string]$decision.c0_revision_id))' AND p.state='succeeded' AND p.invalidated_at IS NULL AND d.output_sha256='$(Escape-Sql $textHash)' ORDER BY p.created_at LIMIT 1;"
        $evidence = @(Invoke-SqliteJson $derivedDb $evidenceSql)
        if ($evidence.Count -ne 1) { throw "No unique active C1 evidence derivative for module $module and hash $textHash" }
        $body = Get-Content -LiteralPath $textPath -Raw -Encoding utf8
        $package = [ordered]@{
            schema_version = 'p6-semantic-candidate/v1'
            source_item_id = [string]$decision.c0_item_id
            source_revision_id = [string]$decision.c0_revision_id
            evidence_derivatives = @([ordered]@{ derivative_id=[string]$evidence[0].derivative_id; output_sha256=[string]$evidence[0].output_sha256 })
            provider = 'local_extract'
            model = 'c1b-full-text-semantic-materializer'
            model_version = '1.0.0'
            prompt_version = 'finance-c2b-deterministic-v1'
            generated_at = $generatedAt
            map_nodes = @()
            entries = @([ordered]@{
                local_ref = "entry:finance-module-$module"
                title = $title
                payload = [ordered]@{ kind='knowledge'; statement=(FirstStatement $body ([string]$decision.title)); details=$body.Trim() }
                map_node_refs = @('foundation:consciousness')
                tags = @('MBA','财务管理',$chapterByModule[$module],"module-$module",[string]$decision.module_type)
                dense_expressions = @([ordered]@{ kind='outline'; content=(Outline $body ([string]$decision.title)) })
                relevance = [ordered]@{ interest=50; strategy=50; consensus=50; rationale='用户已验收该课程为 MBA C2B 标杆；先使用中性基线，保留后续独立调权空间。' }
            })
            relations = @()
            limitations = @('该条目按单一 C0 revision 的正式证据边界物化；课程级跨模块关系由 C2B 课程 package 表达。')
        }
        $moduleRoot = Join-Path $staging "module-$module"
        New-Item -ItemType Directory -Path $moduleRoot -Force | Out-Null
        $packagePath = Join-Path $moduleRoot 'semantic-candidate.json'
        $package | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $packagePath -Encoding utf8
        $params = [ordered]@{ adapter='deterministic_local'; source_variant='c1b'; course='25春 MBAO5406 财务管理'; module_id=$module; evidence_derivative_id=[string]$evidence[0].derivative_id; map_assignment_target=$FinanceMapNodeId } | ConvertTo-Json -Compress
        $packageHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
        $registeredSql = "SELECT d.derivative_id FROM process_runs p JOIN derivatives d ON d.run_id=p.run_id WHERE p.input_revision_id='$(Escape-Sql ([string]$decision.c0_revision_id))' AND p.state='succeeded' AND p.invalidated_at IS NULL AND p.target_kind='structured_result' AND p.tool_or_model='c1b-full-text-semantic-materializer' AND d.output_sha256='$(Escape-Sql $packageHash)' ORDER BY p.created_at LIMIT 1;"
        $registeredExisting = @(Invoke-SqliteJson $derivedDb $registeredSql)
        if ($registeredExisting.Count) {
            $derivativeId = [string]$registeredExisting[0].derivative_id
        } else {
            $registered = Invoke-BabataJson @(
                'process','register','--pipeline','agent_import','--revision',[string]$decision.c0_revision_id,
                '--item',[string]$decision.c0_item_id,'--kind','structured_result','--provider','local_extract',
                '--model','c1b-full-text-semantic-materializer','--tool-version','1.0.0',
                '--input-sha256',[string]$decision.c0_asset_sha256,'--input-asset-id',[string]$decision.c0_asset_id,
                '--json-file',$packagePath,'--output-file',$packagePath,'--media-type','application/json',
                '--language','zh','--params-json',$params,
                '--loss-notes','单条语义包保持单一 C0 revision 证据边界；课程级关系由 C2B package 提供。'
            )
            $derivativeId = [string]$registered.derivative_id
        }
        if ([string]::IsNullOrWhiteSpace($derivativeId)) { throw "Registration returned no derivative id for module $module" }
        $ingested = Invoke-BabataJson @('knowledge','ingest','--derivative',$derivativeId)
        $semanticId = [string]$ingested.semantic_ids[0]
        $suggestionId = [string]$ingested.suggestion_id
        if ([string]::IsNullOrWhiteSpace($semanticId)) { throw "Knowledge ingest returned no semantic id for module $module" }
        [void](Invoke-BabataJson @('knowledge','review-suggestion','--suggestion',$suggestionId,'--decision','accept'))
        $registrationState = 'registered'
    }

    $assignmentSql = "SELECT 1 AS present FROM semantic_map_assignments WHERE semantic_id='$(Escape-Sql $semanticId)' AND map_node_id='$(Escape-Sql $FinanceMapNodeId)';"
    if (@(Invoke-SqliteJson $rawDb $assignmentSql).Count -eq 0) {
        [void](Invoke-BabataJson @('knowledge','change-map-assignment','--semantic',$semanticId,'--map-node',$FinanceMapNodeId,'--change','assign','--rationale','财务管理 C2B 标杆课程的正式知识条目归入财务管理分支。'))
    }
    $foundationSql = "SELECT 1 AS present FROM semantic_map_assignments WHERE semantic_id='$(Escape-Sql $semanticId)' AND map_node_id='mapnode_p6_consciousness';"
    if (@(Invoke-SqliteJson $rawDb $foundationSql).Count -gt 0) {
        [void](Invoke-BabataJson @('knowledge','change-map-assignment','--semantic',$semanticId,'--map-node','mapnode_p6_consciousness','--change','unassign','--rationale','用管理学下的财务管理分支替代临时 foundation 直挂。'))
    }
    $ledgerRows += [ordered]@{ module_id=$module; chapter=$chapterByModule[$module]; semantic_id=$semanticId; suggestion_id=$suggestionId; registration=$registrationState; map_node_id=$FinanceMapNodeId; source_item_id=[string]$decision.c0_item_id; source_revision_id=[string]$decision.c0_revision_id; c1_sha256=$textHash }
}

$ledger = [ordered]@{
    schema = 'babata.finance-c2b-knowledge-universe/v1'
    course = '25春 MBAO5406 财务管理'
    generated_at = $generatedAt
    status = 'registered'
    foundation = [ordered]@{ id='mapnode_p6_consciousness'; name='意识' }
    discipline = [ordered]@{ id=$ManagementMapNodeId; name='管理学' }
    branch = [ordered]@{ id=$FinanceMapNodeId; name='财务管理' }
    semantic_entries = $ledgerRows
}
$ledgerPath = Join-Path $staging 'knowledge-universe-registration.json'
$ledger | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ledgerPath -Encoding utf8
Write-Output "ledger=$ledgerPath entries=$($ledgerRows.Count) registered=$(@($ledgerRows | Where-Object registration -eq 'registered').Count) reused=$(@($ledgerRows | Where-Object registration -eq 'reused').Count)"
