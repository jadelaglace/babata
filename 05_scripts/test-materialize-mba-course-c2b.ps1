[CmdletBinding()]
param()

$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$root=Join-Path ([IO.Path]::GetTempPath()) ('babata-materialize-test-'+[Guid]::NewGuid().ToString('N'))
$data=Join-Path $root 'data';$plans=Join-Path $root 'inputs';$learningRoot=Join-Path $root 'learning';$generated=Join-Path $learningRoot 'generated'
$derived=Join-Path $data '02_derived\files\sha256\aa';$derivedIndex=Join-Path $data '02_derived\index';$runtime=Join-Path $data '04_runtime\staging';$vault=Join-Path $root 'Obsidian Vault';$live=Join-Path $vault 'Babata\MBA\test_latest'
$materializer=Join-Path $PSScriptRoot 'materialize-mba-course-c2b.ps1';$checker=Join-Path $PSScriptRoot 'check-mba-course-c2b-package.ps1'
$planPath=Join-Path $plans 'plan.json';$sourceMapPath=Join-Path $plans 'source-map.json';$decisionPath=Join-Path $plans 'decisions.json';$c1bPath=Join-Path $plans 'c1b-ledger.json';$knowledgePath=Join-Path $plans 'knowledge-ledger.json';$learningPath=Join-Path $learningRoot 'manifest.json'
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Write-Json([string]$Path,$Value,[int]$Depth=20){$Value|ConvertTo-Json -Depth $Depth|Set-Content -LiteralPath $Path -Encoding utf8}
function Assert-Throws([scriptblock]$Action,[string]$Label){$thrown=$false;try{&$Action|Out-Null}catch{$thrown=$true};if(-not $thrown){throw "Expected rejection: $Label"}}
try{
    New-Item -ItemType Directory -Path $plans,$generated,$derived,$derivedIndex,$runtime,(Split-Path $live -Parent) -Force|Out-Null
    & sqlite3 (Join-Path $derivedIndex 'derived.sqlite') 'CREATE TABLE derivatives(derivative_id TEXT PRIMARY KEY, media_type TEXT); INSERT INTO derivatives VALUES (''derivative_media_1'',''image/png'');'
    $colors=@('#2563EB','#16A34A','#EA8A00','#EF4444','#8B5CF6');$chapters=@();$domains=@();$sourceItems=@()
    for($i=1;$i -le 5;$i++){$note=('0{0}-章节{0}' -f $i);$token="知识$i";$chapters+=[ordered]@{id="0$i";note=$note;title="章节$i";modules=@($i)};$domains+=[ordered]@{id="domain$i";label="域$i";color=$colors[$i-1];evidence=@($token);nodes=@([ordered]@{id="chapter$i";note=$note;details=@("${token}：决策规则")})};$sourceItems+=[ordered]@{module_id=$i;c1_sha256=$null}}
    $aids=@('09-测试课程专属工具','10-案例练习','11-复习与自测','视觉证据索引');$aidNodes=@();for($i=0;$i -lt 4;$i++){$aidNodes+=[ordered]@{id="aid$($i+1)";note=$aids[$i]}}
    $plan=[ordered]@{schema='babata.mba-course-c2b-plan/v1';course='Test MBA Course';short_name='测试课程';course_key='test-course';expected_modules=5;output_status='pending_user_acceptance';chapters=$chapters;knowledge_universe=[ordered]@{foundation_id='foundation_test';discipline_id='discipline_test';branch_name='测试管理'};course_map=[ordered]@{classification_axis='测试决策对象';root_id='courseRoot';root_label='测试课程';tagline='目标 · 方法 · 边界';domains=$domains;learning=[ordered]@{id='learning';label='学习支持';color='#64748B';nodes=$aidNodes}};live=[ordered]@{path=$live;vault='Obsidian Vault';file='Babata/MBA/test_latest/index.md'}}
    Write-Json $planPath $plan;$planSha=Hash $planPath
    $registrations=@();$knowledgeModules=@();$sourceNotes=@()
    for($i=1;$i -le 5;$i++){
        $c1=Join-Path $derived "c1-$i.txt";$decision=Join-Path $derived "decision-$i.json";$semantic=Join-Path $plans "semantic-$i.json"
        Set-Content -LiteralPath $c1 -Value ("完整 C1 $i `n"+('课程原文。'*100)) -Encoding utf8;Set-Content -LiteralPath $decision -Value "{`"module`":$i}" -Encoding utf8;Set-Content -LiteralPath $semantic -Value "{`"module`":$i}" -Encoding utf8
        $sourceItems[$i-1].c0_item_id="item_$i";$sourceItems[$i-1].c0_revision_id="revision_$i";$sourceItems[$i-1].c0_asset_id="asset_$i";$sourceItems[$i-1].c0_asset_sha256=('b'*64);$sourceItems[$i-1].c1_derivative_id="derivative_c1_$i";$sourceItems[$i-1].c1_sha256=Hash $c1
        $sourceNotes+=[ordered]@{module_id=$i;input_derivative_id="derivative_c1_$i";input_sha256=Hash $c1}
        $registrations+=[ordered]@{module_id=$i;title="模块$i";module_type='courseware';chapter=[string]$chapters[$i-1].note;source_item_id="item_$i";source_revision_id="revision_$i";source_asset_id="asset_$i";source_asset_sha256=('b'*64);complete_c1=[ordered]@{run_id="run_c1_$i";derivative_id="derivative_c1_$i";output_sha256=Hash $c1;logical_path=('02_derived/files/sha256/aa/'+(Split-Path $c1 -Leaf))};decision_registration=[ordered]@{run_id="run_decision_$i";derivative_id="derivative_decision_$i";output_sha256=Hash $decision;logical_path=('02_derived/files/sha256/aa/'+(Split-Path $decision -Leaf));registration='registered'};media_registrations=@()}
        $knowledgeModules+=[ordered]@{module_id=$i;chapter=[string]$chapters[$i-1].note;package_path=$semantic;fingerprint=Hash $semantic;semantic_id="semantic_$i";review_state='accepted';assignment_state='assigned';status='registered'}
    }
    Add-Type -AssemblyName System.Drawing;$mediaFile=Join-Path $derived 'frame.png';$bitmap=[Drawing.Bitmap]::new(200,120);try{$graphics=[Drawing.Graphics]::FromImage($bitmap);try{$graphics.Clear([Drawing.Color]::White);$graphics.DrawLine([Drawing.Pens]::Blue,0,0,199,119)}finally{$graphics.Dispose()};$bitmap.Save($mediaFile,[Drawing.Imaging.ImageFormat]::Png)}finally{$bitmap.Dispose()}
    $registrations[0].media_registrations=@([ordered]@{output_sha256=Hash $mediaFile;source_locator=[ordered]@{page=1};role='formula_chart';review_reason='spatial';processing=@('render');loss_notes=@();run_id='run_media_1';derivative_id='derivative_media_1';logical_path='02_derived/files/sha256/aa/frame.png';registration='registered'})
    $sourceMap=[ordered]@{schema='babata.mba.c2-source-map/v1';course='Test MBA Course';expected_modules=5;chunks=@([ordered]@{items=$sourceItems})};Write-Json $sourceMapPath $sourceMap;$sourceMapSha=Hash $sourceMapPath
    $decisions=@();for($i=1;$i -le 5;$i++){$retained=if($i -eq 1){@([ordered]@{sha256=Hash $mediaFile})}else{@()};$decisions+=[ordered]@{module_id=$i;c1_sha256=[string]$registrations[$i-1].complete_c1.output_sha256;retained_media=$retained}}
    Write-Json $decisionPath $decisions 10
    $c1b=[ordered]@{schema='babata.mba-course-c1b-registration/v1';course='Test MBA Course';course_key='test-course';status='registered';course_plan_sha256=$planSha;source_map=$sourceMapPath;source_map_sha256=$sourceMapSha;decision_source=$decisionPath;decision_source_sha256=Hash $decisionPath;coverage=[ordered]@{modules=5;complete_c1_reused=5;essence_decisions_registered=5;retained_media_registered=1};registrations=$registrations};Write-Json $c1bPath $c1b 30;$c1bSha=Hash $c1bPath
    $knowledge=[ordered]@{schema='babata.mba-course-c2b-knowledge-registration/v1';status='registered';course_acceptance='pending_user_acceptance';course='Test MBA Course';plan_sha256=$planSha;c1b_ledger_sha256=$c1bSha;source_map_sha256=$sourceMapSha;expected_modules=5;branch=[ordered]@{id='branch_test';name='测试管理';created=$true;preflight='created_and_read_back'};modules=$knowledgeModules};Write-Json $knowledgePath $knowledge 30
    $learningRows=@();$docNames=@('00-课程总览')+@($chapters.note)+@($aids[0..2])
    foreach($name in $docNames){$body="# $name`n`n"+('完整学习正文，解释概念、公式、关系、边界和案例。'*180);if($name -match '^0[1-5]-'){$n=[int]$name.Substring(1,1);$body+="`n`n知识$n 是本章的核心依据。`n`n## 来源索引`n`n- [[来源/M-$n]]`n"};$path=Join-Path $generated ($name+'.md');Set-Content -LiteralPath $path -Value $body -Encoding utf8;$learningRows+=[ordered]@{name=$name+'.md';sha256=Hash $path;chars=(Get-Content -LiteralPath $path -Raw -Encoding utf8).Length}}
    $learning=[ordered]@{schema='babata.mba-course-learning-docs/v1';course='Test MBA Course';status='candidate';course_plan_sha256=$planSha;source_map=$sourceMapPath;source_map_sha256=$sourceMapSha;expected_modules=5;complete_source_notes=5;source_notes=$sourceNotes;chapter_documents=5;learning_documents=$docNames.Count;generated_files=$learningRows};Write-Json $learningPath $learning 20
    $stage=Join-Path $runtime 'valid';$result=@(&$materializer -CoursePlanPath $planPath -LearningDocsManifestPath $learningPath -C1BRegistrationLedgerPath $c1bPath -KnowledgeUniverseLedgerPath $knowledgePath -StagingRoot $stage -DataHome $data)
    if($result.Count -ne 1 -or $result[0].status -ne 'passed_engineering_gates'){throw 'Valid mock materialization failed'}
    $manifestPath=[string]$result[0].manifest;$manifest=Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8|ConvertFrom-Json
    if($manifest.status -ne 'pending_user_acceptance' -or [int]$manifest.old_c2b_inputs -ne 0 -or [int]$manifest.external_sovereign_original_reads -ne 0){throw 'Materialized status or source boundary is wrong'}
    if(-not (Get-Content -LiteralPath (Join-Path $stage 'package\index.md') -Raw -Encoding utf8).Contains("[[$($aids[0])]]")){throw 'Course-specific learning-tool link was not materialized'}
    if((Get-Content -LiteralPath (Join-Path $stage 'package\01-章节1.md') -Raw -Encoding utf8).Contains('[[来源/')){throw 'Complete source-layer link leaked into package'}
    $checked=@(&$checker -CoursePlanPath $planPath -PackageRoot (Join-Path $stage 'package') -ManifestPath $manifestPath);if($checked.Count -ne 1 -or $checked[0].status -ne 'passed'){throw 'Materialized mock package did not pass checker'}
    $knowledge.modules=@($knowledge.modules)+@($knowledge.modules[0]);Write-Json $knowledgePath $knowledge 30
    $badStage=Join-Path $runtime 'bad-duplicate-module';Assert-Throws {&$materializer -CoursePlanPath $planPath -LearningDocsManifestPath $learningPath -C1BRegistrationLedgerPath $c1bPath -KnowledgeUniverseLedgerPath $knowledgePath -StagingRoot $badStage -DataHome $data} 'duplicate knowledge module'
    if(Test-Path -LiteralPath $badStage){throw 'Failed preflight created a staging root'}
    'mba-course-materialization-tests=passed'
}finally{if(Test-Path -LiteralPath $root){Remove-Item -LiteralPath $root -Recurse -Force}}
