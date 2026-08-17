[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$FinanceLedgerPath,
    [Parameter(Mandatory=$true)][string]$SupplyChainLedgerPath,
    [Parameter(Mandatory=$true)][string]$OutputRoot,
    [string]$DataHome='D:\BabataData',
    [string]$BabataExe
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Write-Json([object]$Value,[string]$Path){[IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 50),[Text.UTF8Encoding]::new($false))}
function Invoke-Babata([string[]]$Arguments){
    $output=@(& $script:Babata @Arguments 2>&1)
    if($LASTEXITCODE -ne 0){throw "babata failed ($($Arguments -join ' ')): $($output -join ' ')"}
    ($output -join "`n")|ConvertFrom-Json
}
function Assignment([string]$Node,[string]$Role,[int]$Strength,[int]$Confidence,[string]$Rationale){
    [ordered]@{map_node_id=$Node;role=$Role;strength=$Strength;confidence=$Confidence;rationale=$Rationale;method_version='mba-foundation-assignment/v1'}
}
function Finance-Assignments([string]$Chapter){
    $items=@(
        (Assignment 'mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6' 'primary' 100 100 '该语义条目来自财务管理课程并直接覆盖财务管理专业分支。'),
        (Assignment 'mapnode_01KZXVE4ZXFX3EKF82JKNXN4HG' 'secondary' 95 100 '财务管理是管理学中的企业资源配置与价值决策分支。'),
        (Assignment 'mapnode_p6_consciousness' 'primary' 90 95 '本章核心是管理者对价值、风险、权衡和治理规则的判断。')
    )
    switch -Regex($Chapter){
        '^01-' {$items+=Assignment 'mapnode_p6_matter' 'secondary' 65 85 '财务目标最终作用于企业资产、现金与资本等物质资源。';$items+=Assignment 'mapnode_p6_time' 'contextual' 45 80 '财务职责包含跨期价值维护与长期目标。'}
        '^02-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '营运资本直接处理现金、存货、应收应付和流动资产。';$items+=Assignment 'mapnode_p6_time' 'secondary' 75 90 '现金周转和期限匹配具有明确时间结构。'}
        '^03-' {$items+=Assignment 'mapnode_p6_time' 'primary' 95 98 '项目投资评估以跨期现金流、折现和期限为核心。';$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '投资决策配置真实资产与资本。'}
        '^04-' {$items+=Assignment 'mapnode_p6_time' 'primary' 90 95 '风险分析比较未来情景、概率和时间跨度。';$items+=Assignment 'mapnode_p6_matter' 'secondary' 75 88 '风险结果作用于项目资产和现金流。'}
        '^05-' {$items+=Assignment 'mapnode_p6_time' 'primary' 90 95 '企业估值依赖未来现金流和终值折现。';$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '估值对象是企业资产与资本索取权。'}
        '^06-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '融资与资本结构配置债务、权益和资本资源。';$items+=Assignment 'mapnode_p6_time' 'secondary' 80 90 '期限、现金流和融资周期影响资本成本。'}
        '^07-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 85 92 '股利政策分配企业现金与权益价值。';$items+=Assignment 'mapnode_p6_time' 'secondary' 75 90 '股利稳定性和留存形成跨期分配。'}
        '^08-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '并购重组重新组合企业资产、资本和资源。';$items+=Assignment 'mapnode_p6_time' 'secondary' 80 90 '协同兑现、整合和困境重组跨越多个时期。';$items+=Assignment 'mapnode_p6_space' 'contextual' 40 75 '并购可能跨越组织、市场和地域边界。'}
        default {throw "Unknown finance chapter: $Chapter"}
    }
    @($items)
}
function Supply-Assignments([string]$Chapter){
    $items=@(
        (Assignment 'mapnode_01M024K6142W6AQJPDE7NGSKEH' 'primary' 100 100 '该语义条目来自全球供应链课程并直接覆盖供应链管理专业分支。'),
        (Assignment 'mapnode_01KZXVE4ZXFX3EKF82JKNXN4HG' 'secondary' 95 100 '供应链管理是管理学中的跨组织运营与资源协同分支。'),
        (Assignment 'mapnode_p6_consciousness' 'primary' 90 95 '本章需要管理者形成网络、库存、风险和绩效的系统判断。')
    )
    switch -Regex($Chapter){
        '^01-' {$items+=Assignment 'mapnode_p6_space' 'primary' 90 95 '供应链战略与网络首先描述跨节点和地域的空间连接。';$items+=Assignment 'mapnode_p6_matter' 'primary' 85 92 '网络承载产品、设施和物流资源。';$items+=Assignment 'mapnode_p6_time' 'secondary' 75 88 '战略配置决定长期流动与响应周期。'}
        '^02-' {$items+=Assignment 'mapnode_p6_space' 'primary' 95 98 '设施选址与网络设计直接优化地理和节点空间。';$items+=Assignment 'mapnode_p6_matter' 'primary' 85 92 '设施与运输能力属于实体资源配置。';$items+=Assignment 'mapnode_p6_time' 'secondary' 75 88 '交付时效和长期容量影响选址。'}
        '^03-' {$items+=Assignment 'mapnode_p6_time' 'primary' 95 98 '预测与计划以需求时间序列和计划期为核心。';$items+=Assignment 'mapnode_p6_space' 'secondary' 75 88 '需求在区域和网络节点间分布。';$items+=Assignment 'mapnode_p6_matter' 'secondary' 70 85 '预测驱动物料和产能配置。'}
        '^04-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 95 98 '库存分类与补货直接管理实体库存。';$items+=Assignment 'mapnode_p6_time' 'secondary' 85 92 '补货点、周期和提前期具有时间结构。';$items+=Assignment 'mapnode_p6_space' 'secondary' 75 88 '库存分布在不同仓储和网络位置。'}
        '^05-' {$items+=Assignment 'mapnode_p6_matter' 'primary' 95 98 '库存模型量化实体资源水平与安全库存。';$items+=Assignment 'mapnode_p6_time' 'primary' 90 95 '提前期、需求周期和服务期是模型核心。';$items+=Assignment 'mapnode_p6_space' 'secondary' 70 85 '多节点库存需要空间配置。'}
        '^06-' {$items+=Assignment 'mapnode_p6_space' 'primary' 90 95 '物流、采购和协同连接供应商、设施与运输节点。';$items+=Assignment 'mapnode_p6_matter' 'primary' 90 95 '采购、运输和交付作用于实体物料。';$items+=Assignment 'mapnode_p6_time' 'secondary' 75 88 'SCOR 流程与交付周期具有时间约束。'}
        '^07-' {$items+=Assignment 'mapnode_p6_time' 'primary' 90 95 '牛鞭效应和韧性表现为跨期波动、恢复和响应。';$items+=Assignment 'mapnode_p6_space' 'primary' 85 92 '风险沿多级网络传播。';$items+=Assignment 'mapnode_p6_matter' 'secondary' 80 90 '风险影响库存、产能和物流资产。'}
        '^08-' {$items+=Assignment 'mapnode_p6_time' 'primary' 85 92 '现金周期、周转与可持续绩效均需跨期衡量。';$items+=Assignment 'mapnode_p6_matter' 'primary' 85 92 '绩效指标度量库存、成本和资源消耗。';$items+=Assignment 'mapnode_p6_space' 'secondary' 75 88 '绩效与可持续影响跨越供应网络。'}
        default {throw "Unknown supply-chain chapter: $Chapter"}
    }
    @($items)
}

$financePath=(Get-Item -LiteralPath $FinanceLedgerPath -ErrorAction Stop).FullName
$supplyPath=(Get-Item -LiteralPath $SupplyChainLedgerPath -ErrorAction Stop).FullName
$out=[IO.Path]::GetFullPath($OutputRoot)
if(Test-Path -LiteralPath $out){throw "Fresh successor registration root already exists: $out"}
[void](New-Item -ItemType Directory -Path $out)
if([string]::IsNullOrWhiteSpace($BabataExe)){$BabataExe=Join-Path (Split-Path $PSScriptRoot -Parent) '01_app\target\debug\babata.exe'}
$script:Babata=(Get-Item -LiteralPath $BabataExe -ErrorAction Stop).FullName
$env:BABATA_DATA_HOME=[IO.Path]::GetFullPath($DataHome)
$finance=Get-Content -LiteralPath $financePath -Raw -Encoding utf8|ConvertFrom-Json
$supply=Get-Content -LiteralPath $supplyPath -Raw -Encoding utf8|ConvertFrom-Json
$financeRows=@($finance.semantic_entries);$supplyRows=@($supply.modules)
if($financeRows.Count -ne 37 -or $supplyRows.Count -ne 101){throw 'Historical semantic denominators changed'}

$lensInput=[ordered]@{
    title='Dominican University of California MBA knowledge lens'
    purpose='Versioned non-owning navigation lens for MBA courses and their stable covered branches.'
    selection=[ordered]@{}
    manual_include=@();manual_exclude=@()
    course_refs=@('course:finance-management@1','course:supply-chain@1')
    map_node_refs=@('mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6','mapnode_01M024K6142W6AQJPDE7NGSKEH')
    organisation_rules=@('map_then_title','title')
    include_unreviewed=$false
}
$lensPath=Join-Path $out 'mba-lens-definition-v1.json';Write-Json $lensInput $lensPath
$lens=Invoke-Babata @('--json','sublibraries','create','--definition',$lensPath)
$lensId=[string]$lens.id

$financeModules=@($financeRows|ForEach-Object{[ordered]@{module_id=[string]$_.module_id;semantic_id=[string]$_.semantic_id;chapter_id=[string]$_.chapter;assignments=@(Finance-Assignments ([string]$_.chapter))}})
$supplyModules=@($supplyRows|ForEach-Object{[ordered]@{module_id=[string]$_.module_id;semantic_id=[string]$_.semantic_id;chapter_id=[string]$_.chapter;assignments=@(Supply-Assignments ([string]$_.chapter))}})
$created='2026-08-17T00:00:00Z'
$definitions=@(
    [ordered]@{schema_version='babata.course-registration/v1';course_key='finance-management';version=1;title='25春 MBAO5406 财务管理';source='Dominican University of California MBA';term='2025-spring';acceptance_state='accepted';closure_state='closed';branches=@([ordered]@{branch_map_node_id='mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6';rationale='课程完整覆盖财务管理专业分支。'});modules=$financeModules;map_relations=@([ordered]@{from_map_node_id='mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6';kind='draws_from';to_map_node_id='mapnode_01KZXVE4ZXFX3EKF82JKNXN4HG';rationale='财务管理从管理学的组织决策与资源配置方法中发展。'},[ordered]@{from_map_node_id='mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6';kind='intersects_with';to_map_node_id='mapnode_01M024K6142W6AQJPDE7NGSKEH';rationale='营运资本、库存、现金周期与供应链管理交叉。'});lens=[ordered]@{sublibrary_id=$lensId;definition_version=1};author_kind='machine';author='codex-governance-migration';created_at=$created},
    [ordered]@{schema_version='babata.course-registration/v1';course_key='supply-chain';version=1;title='25春 MBAO 5403 全球供应链和可持续运营';source='Dominican University of California MBA';term='2025-spring';acceptance_state='accepted';closure_state='closed';branches=@([ordered]@{branch_map_node_id='mapnode_01M024K6142W6AQJPDE7NGSKEH';rationale='课程完整覆盖供应链管理专业分支。'});modules=$supplyModules;map_relations=@([ordered]@{from_map_node_id='mapnode_01M024K6142W6AQJPDE7NGSKEH';kind='draws_from';to_map_node_id='mapnode_01KZXVE4ZXFX3EKF82JKNXN4HG';rationale='供应链管理从管理学的组织协调与运营决策方法中发展。'},[ordered]@{from_map_node_id='mapnode_01M024K6142W6AQJPDE7NGSKEH';kind='intersects_with';to_map_node_id='mapnode_01KZXVEE3A3SAE7W0BDGB0FMP6';rationale='库存、现金周期、采购融资与财务管理交叉。'});lens=[ordered]@{sublibrary_id=$lensId;definition_version=1};author_kind='machine';author='codex-governance-migration';created_at=$created}
)
$registered=@();foreach($definition in $definitions){
    $path=Join-Path $out (([string]$definition.course_key)+'-course-registration-v1.json');Write-Json $definition $path
    $write=Invoke-Babata @('--json','knowledge','register-course','--definition',$path)
    $read=Invoke-Babata @('--json','knowledge','show-course','--course',[string]$definition.course_key,'--version','1')
    if([string]$write.course_id -cne [string]$read.course_id -or [string]$write.definition_sha256 -cne [string]$read.definition_sha256){throw "Course read-back mismatch: $($definition.course_key)"}
    $registered+=[ordered]@{course_key=[string]$definition.course_key;course_id=[string]$read.course_id;definition=$path;definition_sha256=[string]$read.definition_sha256;modules=@($read.definition.modules).Count;assignments=@($read.definition.modules.assignments).Count;acceptance_state=[string]$read.definition.acceptance_state;closure_state=[string]$read.definition.closure_state}
}
$receipt=[ordered]@{schema='babata.mba-course-governance-successor-registration/v1';status='registered';created_at=$created;lens=[ordered]@{id=$lensId;version=1;authority=$lens.authority;definition=$lensPath;definition_sha256=Hash $lensPath};sources=@([ordered]@{path=$financePath;sha256=Hash $financePath;semantic_entries=$financeRows.Count},[ordered]@{path=$supplyPath;sha256=Hash $supplyPath;semantic_entries=$supplyRows.Count});courses=$registered;direct_sql_writes=0;historical_assignments_deleted=0;historical_packages_rewritten=0;closure_verifier_runs=0}
$receiptPath=Join-Path $out 'registration-receipt.json';Write-Json $receipt $receiptPath
[pscustomobject][ordered]@{schema=$receipt.schema;status='registered';lens=$lensId;courses=$registered.Count;modules=($financeRows.Count+$supplyRows.Count);receipt=$receiptPath}
