$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

$checker=Join-Path $PSScriptRoot 'check-mba-course-presentation-plan.ps1'
$root=Join-Path ([IO.Path]::GetTempPath()) ('babata-presentation-'+[guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $root)
function Write-Plan([object]$Plan,[string]$Name){$path=Join-Path $root $Name;[IO.File]::WriteAllText($path,($Plan|ConvertTo-Json -Depth 40),[Text.UTF8Encoding]::new($false));$path}
function Assert-Fails([scriptblock]$Action,[string]$Label){$failed=$false;try{&$Action|Out-Null}catch{$failed=$true};if(-not $failed){throw "Expected failure: $Label"}}
function Support {
    @(
        [ordered]@{slot='decision_tools';id='tools';note='学习支持-长课工具箱';legacy_note='09-长课工具箱'},
        [ordered]@{slot='case_practice';id='cases';note='学习支持-长课案例练习';legacy_note='10-长课案例练习'},
        [ordered]@{slot='review_self_test';id='review';note='学习支持-复习与自测';legacy_note='11-复习与自测'},
        [ordered]@{slot='evidence_index';id='evidence';note='视觉证据索引';legacy_note='视觉证据索引'}
    )
}
function Base([object]$Outline,[object[]]$Nodes){
    [ordered]@{
        schema='babata.mba-course-presentation-plan/v2';course='Long Course';short_name='长课';course_key='long-course';expected_modules=101
        profile='semantic-obsidian/v2';output_status='pending_user_acceptance'
        source=[ordered]@{plan_path='C:\fixture\v1.json';plan_sha256=('a'*64);manifest_path='C:\fixture\manifest.json';manifest_sha256=('b'*64)}
        outline=$Outline;learning_support=@(Support);rename_map=[ordered]@{'09-长课工具箱'='学习支持-长课工具箱';'10-长课案例练习'='学习支持-长课案例练习';'11-复习与自测'='学习支持-复习与自测'}
        course_map=[ordered]@{domains=@([ordered]@{id='domain';label='课程';nodes=$Nodes});learning=[ordered]@{nodes=@((Support)|ForEach-Object{[ordered]@{id=$_.id;note=$_.note}})}}
        live=[ordered]@{path='C:\fixture\live';vault='fixture';file='Babata/MBA/long/index.md'}
    }
}

try{
    $units=@();for($i=1;$i -le 101;$i++){$units+=[ordered]@{id=('u{0:d3}' -f $i);note=('第{0:d3}节' -f $i);title=('第{0}节' -f $i);source_modules=@([string]$i)}}
    $sections=@();for($s=0;$s -lt 10;$s++){$start=$s*10;$count=if($s -eq 9){11}else{10};$sections+=[ordered]@{id=('s{0:d2}' -f ($s+1));title=('主题{0}' -f ($s+1));units=@($units[$start..($start+$count-1)])}}
    $sectioned=Base ([ordered]@{mode='sectioned';sections=$sections}) @($units|ForEach-Object{[ordered]@{id=$_.id;note=$_.note}})
    $sectionedPath=Write-Plan $sectioned 'sectioned.json';$result=@(&$checker -PlanPath $sectionedPath)
    if($result.Count -ne 1 -or [int]$result[0].units -ne 101){throw '101-unit sectioned plan did not pass'}

    $flat=Base ([ordered]@{mode='flat';units=$units}) @($units|ForEach-Object{[ordered]@{id=$_.id;note=$_.note}})
    $flatPath=Write-Plan $flat 'flat.json';if(@(&$checker -PlanPath $flatPath).Count -ne 1){throw '101-unit flat plan did not pass'}

    $duplicate=$flat|ConvertTo-Json -Depth 40|ConvertFrom-Json;$duplicate.outline.units[100].source_modules=@('100')
    $duplicatePath=Write-Plan $duplicate 'duplicate.json';Assert-Fails {&$checker -PlanPath $duplicatePath} 'duplicate source module'
    $numbered=$flat|ConvertTo-Json -Depth 40|ConvertFrom-Json;$numbered.learning_support[0].note='09-长课工具箱';$numbered.course_map.learning.nodes[0].note='09-长课工具箱'
    $numberedPath=Write-Plan $numbered 'numbered.json';Assert-Fails {&$checker -PlanPath $numberedPath} 'numbered learning support'
    $mixed=$flat|ConvertTo-Json -Depth 40|ConvertFrom-Json;$mixed.outline|Add-Member -NotePropertyName sections -NotePropertyValue @([ordered]@{id='bad';title='bad';units=@($units[0])})
    $mixedPath=Write-Plan $mixed 'mixed.json';Assert-Fails {&$checker -PlanPath $mixedPath} 'mixed outline shape'
    [pscustomobject]@{schema='babata.mba-course-presentation-governance-test/v1';status='passed';flat_units=101;sectioned_units=101;mutations_rejected=3}
}finally{
    if(Test-Path -LiteralPath $root){Remove-Item -LiteralPath $root -Recurse -Force}
}
