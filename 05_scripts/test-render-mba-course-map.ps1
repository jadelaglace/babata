[CmdletBinding()]
param()

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=Join-Path ([IO.Path]::GetTempPath()) ('babata-map-test-'+[Guid]::NewGuid().ToString('N'))
$package=Join-Path $root 'package';$media=Join-Path $package 'media';$renderer=Join-Path $PSScriptRoot 'render-mba-course-map.ps1'
function Assert-Throws([scriptblock]$Action,[string]$Label){$thrown=$false;try{&$Action|Out-Null}catch{$thrown=$true};if(-not $thrown){throw "Expected rejection: $Label"}}
try{
    New-Item -ItemType Directory -Path $media -Force|Out-Null
    Set-Content -LiteralPath (Join-Path $package 'index.md') -Value "---`nstatus: pending_user_acceptance`n---`n`n# Test`n`n## 课程章节`n" -Encoding utf8
    $colors=@('#2563EB','#0F766E','#EA8A00','#7C3AED','#DC2626','#16A34A');$domains=@()
    for($i=1;$i -le 6;$i++){
        $note=('0{0}-章节{0}' -f $i);$token="知识$i";$body="# $note`n`n$token 是本章的核心依据。`n"; $evidence=@($token)
        if($i -eq 2){$body += "连续监控与定期审查是本章的控制方式。`n";$evidence=@('连续检查','周期检查')}
        Set-Content -LiteralPath (Join-Path $package ($note+'.md')) -Value $body -Encoding utf8
        $domains+=[ordered]@{id="domain-$i";label="域$i";color=$colors[$i-1];evidence=$evidence;nodes=@([ordered]@{id="chapter-$i";note=$note;details=@("${token}：决策规则")})}
    }
    $learningNotes=@('09-公式与决策工具','10-案例练习','11-复习与自测','视觉证据索引');$learningNodes=@()
    for($i=0;$i -lt $learningNotes.Count;$i++){Set-Content -LiteralPath (Join-Path $package ($learningNotes[$i]+'.md')) -Value "# $($learningNotes[$i])`n" -Encoding utf8;$learningNodes+=[ordered]@{id="aid-$($i+1)";note=$learningNotes[$i]}}
    $spec=[ordered]@{schema='babata.mba-course-map-spec/v1';status='pending_user_acceptance';course='Test Course';course_plan_sha256=('a'*64);classification_axis='测试决策对象';root_id='courseRoot';root_label='测试课程';tagline='目标 · 方法 · 边界';asset_basename='测试课程脑图';domains=$domains;learning=[ordered]@{id='learning';label='学习支持';color='#64748B';nodes=$learningNodes}}
    $spec|ConvertTo-Json -Depth 20|Set-Content -LiteralPath (Join-Path $media 'course-map.spec.json') -Encoding utf8
    $result=@(&$renderer -PackageRoot $package)
    if($result.Count -ne 1 -or $result[0].status -ne 'passed' -or [int]$result[0].mece_domains -ne 6 -or [int]$result[0].internal_link_targets -ne 10){throw 'Valid six-domain course map did not pass'}
    if(-not(Test-Path -LiteralPath (Join-Path $package ([string]$result[0].mermaid)))-or -not(Test-Path -LiteralPath (Join-Path $package ([string]$result[0].png)))){throw 'Renderer omitted package-owned artifacts'}
    $spec.domains[1].id='domain-1';$spec|ConvertTo-Json -Depth 20|Set-Content -LiteralPath (Join-Path $media 'course-map.spec.json') -Encoding utf8
    Assert-Throws {&$renderer -PackageRoot $package} 'duplicate domain id'
    'mba-course-map-tests=passed'
}finally{if(Test-Path -LiteralPath $root){Remove-Item -LiteralPath $root -Recurse -Force}}
