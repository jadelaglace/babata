[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$SourceMapPath,
    [Parameter(Mandatory = $true)][string]$C1BDecisionPath,
    [Parameter(Mandatory = $true)][string]$C1BRoot,
    [Parameter(Mandatory = $true)][string]$QianwenTextScript,
    [Parameter(Mandatory = $true)][string]$StagingRoot,
    [Parameter(Mandatory = $true)][string]$LiveVaultPath,
    [string]$Model = 'qwen3.6-plus',
    [switch]$Resume,
    [switch]$Publish
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Hash([string]$p) { (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() }
function Relative([string]$base, [string]$p) {
    $b = [Uri]((Get-Item $base).FullName.TrimEnd('\') + '\')
    $u = [Uri](Get-Item $p).FullName
    [Uri]::UnescapeDataString($b.MakeRelativeUri($u).ToString()).Replace('\','/')
}
function Body([string]$p) {
    $t = Get-Content -LiteralPath $p -Raw -Encoding utf8
    if ($t -match '(?m)^# .+\r?\n') { $t = [regex]::Replace($t,'(?m)^# .+\r?\n','',1) }
    $t.Trim()
}
function Set-CourseNavigation([string]$path,[object[]]$chapterSpecs) {
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $matches = [regex]::Matches($text, '(?ms)^## 章节导航\s*.*?(?=^## |\z)')
    if ($matches.Count -ne 1) { throw "Expected exactly one chapter navigation section: $path" }
    $navigation = @('## 章节导航', '')
    foreach ($chapter in $chapterSpecs) { $navigation += "- [[$($chapter.id)]]" }
    $text = [regex]::Replace($text, '(?ms)^## 章节导航\s*.*?(?=^## |\z)', (($navigation -join "`n") + "`n"), 1)
    Set-Content -LiteralPath $path -Value ($text.Trim() + "`n") -Encoding utf8
}
function Test-CourseNavigation([string]$root,[object[]]$chapterSpecs) {
    $errors=@()
    $expected=@($chapterSpecs|ForEach-Object{$_.id})
    $overview=Get-Content -LiteralPath (Join-Path $root '00-课程总览.md') -Raw -Encoding utf8
    $section=[regex]::Match($overview,'(?ms)^## 章节导航\s*(.*?)(?=^## |\z)')
    $actual=if($section.Success){@([regex]::Matches($section.Groups[1].Value,'\[\[([^\]|#]+)')|ForEach-Object{$_.Groups[1].Value})}else{@()}
    if(($actual.Count -ne $expected.Count) -or (($actual -join "`n") -ne ($expected -join "`n"))){$errors+='00-课程总览.md: chapter navigation does not match the eight chapter files'}
    foreach($chapter in $expected){if(-not(Test-Path -LiteralPath (Join-Path $root ($chapter+'.md')))){$errors+="missing chapter file $chapter"}}
    if($errors.Count){throw "Chapter navigation verification failed: $($errors -join '; ')"}
    $expected.Count
}

$mapPath = (Get-Item $SourceMapPath).FullName
$decisionPath = (Get-Item $C1BDecisionPath).FullName
$c1bRoot = (Get-Item $C1BRoot).FullName
$staging = [IO.Path]::GetFullPath($StagingRoot)
if ((Test-Path $staging) -and -not $Resume) { throw "Use a fresh staging root: $staging" }
$package = Join-Path $staging 'package'; $generated = Join-Path $staging 'generated'; $requests = Join-Path $staging 'requests'; $responses = Join-Path $staging 'responses'
New-Item -ItemType Directory -Path $package,$generated,$requests,$responses -Force | Out-Null
$map = Get-Content $mapPath -Raw | ConvertFrom-Json
$decisions = @(Get-Content $decisionPath -Raw | ConvertFrom-Json)
if ($decisions.Count -ne 37 -or @($decisions | Where-Object { $_.variant -ne 'c1b' }).Count -gt 0) { throw 'C1B decision must cover 37 c1b items.' }
$byId = @{}; foreach ($d in $decisions) { $byId[[string]$d.module_id] = $d }
$items = @(); $seen=@{}
foreach ($chunk in @($map.chunks)) { foreach ($raw in @($chunk.items)) {
    $id=[string]$raw.module_id; if($seen.ContainsKey($id)){continue}; $seen[$id]=$true
    if(-not $byId.ContainsKey($id)){throw "Missing C1B decision $id"}
    $d=$byId[$id]; $c1=Join-Path $c1bRoot ($d.c1b_text_path -replace '^c1b/','c1b/')
    if(-not (Test-Path $c1)){ $c1 = Join-Path $c1bRoot ($d.c1b_text_path -replace '^c1b/','') }
    if(-not (Test-Path $c1)){ throw "Missing C1B text $id" }
    if((Hash $c1) -ne [string]$d.c1_sha256){throw "C1B text hash mismatch $id"}
    $items += [ordered]@{ id=$id; title=[string]$raw.title; type=[string]$raw.module_type; parent=[string]$raw.parent; c1=$c1; c1_sha256=[string]$d.c1_sha256; c0_item_id=[string]$d.c0_item_id; c0_revision_id=[string]$d.c0_revision_id; c0_asset_id=[string]$d.c0_asset_id; c0_asset_sha256=[string]$d.c0_asset_sha256; decision=$d }
}}
if($items.Count -ne 37){throw "Expected 37 items, found $($items.Count)"}
$chapters=@(
 @{id='01-财务管理目标与财务高管职责'; title='财务管理目标与财务高管职责'; modules=@('892115','892117','892119')},
 @{id='02-营运资本管理'; title='营运资本管理'; modules=@('892121','892123','892125','892127','892129','892131','897613')},
 @{id='03-项目投资评估'; title='项目投资评估'; modules=@('892135','892137','892139','892141','892143','892145','901333','972135')},
 @{id='04-投资风险与不确定性'; title='投资风险与不确定性'; modules=@('892153','892155')},
 @{id='05-企业估值'; title='企业估值'; modules=@('892157','892159')},
 @{id='06-融资决策与资本结构'; title='融资决策与资本结构'; modules=@('892161','892163','892165','972531','972533')},
 @{id='07-股利决策'; title='股利决策'; modules=@('892171','892173')},
 @{id='08-并购与企业重组'; title='并购与企业重组'; modules=@('892175','892177','892179','892181','892185','892187','976503','976505')}
)
$system=@'
你是一名严谨的 MBA 财务管理教材编辑。只根据输入的 C1B 完整文字证据编写可直接学习的中文章节。正文必须展开概念、因果、决策步骤、公式变量与适用条件、案例、边界和复习检查；不得写目录字段、处理过程、模型、C1B/C2B、试点或存储说明；不得引入输入没有支持的事实。只输出 Markdown 正文。
'@
$chapterTexts=@{}
foreach($ch in $chapters){
  $src=@(); foreach($it in @($items|Where-Object {$ch.modules -contains $_.id})){ $src += "`n===== C1B SOURCE M-$($it.id) =====`n$(Get-Content $it.c1 -Raw -Encoding utf8)" }
  $req=[ordered]@{model=$Model;enable_thinking=$false;temperature=0.15;max_tokens=12000;messages=@([ordered]@{role='system';content=$system},[ordered]@{role='user';content="编写《$($ch.title)》完整学习章节，至少 3000 字符。固定结构：# $($ch.title)；## 本章要解决的问题；## 核心结论；## 概念与逻辑；## 决策方法；## 公式与计算；## 案例与应用；## 易错点与边界；## 复习检查。每个关键结论在正文中用 [[M-$($ch.modules[0])]] 等模块标记来源。`n$($src -join "`n")"})}
  $dir=Join-Path $requests $ch.id; $out=Join-Path $responses $ch.id; New-Item -ItemType Directory -Path $dir,$out -Force|Out-Null; $rp=Join-Path $dir 'request.json'; $req|ConvertTo-Json -Depth 12|Set-Content $rp -Encoding utf8; $responseFile=Join-Path $out 'response.json'; if(-not($Resume -and (Test-Path $responseFile))){ & python $QianwenTextScript --file $rp --output $out --model $Model --stream --hide-reasoning; if($LASTEXITCODE -ne 0){throw "model failed $($ch.id)"} }; $resp=Get-Content $responseFile -Raw|ConvertFrom-Json; $txt=[string]$resp.choices[0].message.content; if($txt.Length -lt 3000){throw "thin chapter $($ch.id)"}; $chapterTexts[$ch.id]=$txt; Set-Content (Join-Path $generated ($ch.id+'.md')) $txt -Encoding utf8
}
foreach($f in Get-ChildItem $generated -File){Copy-Item $f.FullName (Join-Path $package $f.Name)}

$allChapters = ($chapters | ForEach-Object { "`n===== CHAPTER $($_.id) =====`n$(Get-Content (Join-Path $generated ($_.id+'.md')) -Raw -Encoding utf8)" }) -join "`n"
$synth = @(
  @{id='00-课程总览'; prompt="基于以下八章编写课程总览，至少 2400 字符。固定结构：# 财务管理课程总览；## 课程主线；## 八章关系；## 关键决策链；## 学习顺序；## 课程边界；## 章节导航。章节导航链接八篇章节。`n$allChapters"; min=2400},
  @{id='09-公式与决策工具'; prompt="基于以下八章编写公式与决策工具手册，至少 3800 字符。固定结构：# 公式与决策工具；## 使用方法；## 营运资本工具；## 投资评估工具；## 风险分析工具；## 估值工具；## 融资与股利工具；## 并购重组工具；## 公式易错检查表。每项解释变量、步骤、假设和误用。`n$allChapters"; min=3800},
  @{id='10-案例练习'; prompt="基于以下八章编写案例练习册，至少 3200 字符。固定结构：# 案例练习；## 案例使用方法；## 营运资本案例；## 项目投资案例；## 估值与融资案例；## 并购重组案例；## 综合练习；## 参考思路。不得编造数字。`n$allChapters"; min=3200},
  @{id='11-复习与自测'; prompt="基于以下八章编写复习与自测，至少 2800 字符。固定结构：# 复习与自测；## 复习路径；## 核心概念自测；## 计算与决策自测；## 案例论述题；## 易混点速查；## 答案要点。问题覆盖八章，答案说明判断路径。`n$allChapters"; min=2800}
)
foreach($spec in $synth){
  $dir=Join-Path $requests $spec.id; $out=Join-Path $responses $spec.id; New-Item -ItemType Directory -Path $dir,$out -Force|Out-Null; $rp=Join-Path $dir 'request.json'; $r=[ordered]@{model=$Model;enable_thinking=$false;temperature=0.15;max_tokens=12000;messages=@([ordered]@{role='system';content=$system},[ordered]@{role='user';content=$spec.prompt})}; $r|ConvertTo-Json -Depth 12|Set-Content $rp -Encoding utf8; $responseFile=Join-Path $out 'response.json'; if(-not($Resume -and (Test-Path $responseFile))){& python $QianwenTextScript --file $rp --output $out --model $Model --stream --hide-reasoning; if($LASTEXITCODE -ne 0){throw "model failed $($spec.id)"}}; $resp=Get-Content $responseFile -Raw|ConvertFrom-Json; $txt=[string]$resp.choices[0].message.content; if($txt.Length -lt $spec.min){throw "thin synthesis $($spec.id)"}; Set-Content (Join-Path $generated ($spec.id+'.md')) $txt -Encoding utf8; Copy-Item (Join-Path $generated ($spec.id+'.md')) (Join-Path $package ($spec.id+'.md')) -Force
}
Set-CourseNavigation (Join-Path $generated '00-课程总览.md') $chapters
Copy-Item (Join-Path $generated '00-课程总览.md') (Join-Path $package '00-课程总览.md') -Force
$mediaTarget=Join-Path $package 'media'; New-Item -ItemType Directory $mediaTarget -Force|Out-Null
$mediaRows=@()
foreach($d in $decisions){
  foreach($m in @($d.retained_media)){
    $src=Join-Path $c1bRoot ($m.path -replace '^c1b/','c1b/')
    if(-not(Test-Path $src)){throw "Missing media $($m.path)"}
    $name="M-$($d.module_id)-"+(Split-Path $src -Leaf)
    Copy-Item $src (Join-Path $mediaTarget $name)
    $locator=if($m.PSObject.Properties['time_seconds']){$m.time_seconds}elseif($m.PSObject.Properties['page']){"page $($m.page)"}else{$null}
    $mediaRows += [ordered]@{
      module_id=[string]$d.module_id; path="media/$name"; sha256=Hash (Join-Path $mediaTarget $name)
      source_locator=$locator; modality='image'; role=if($m.PSObject.Properties['role']){[string]$m.role}else{'visual_evidence'}
      review_reason=if($m.PSObject.Properties['review_reason']){[string]$m.review_reason}else{$null}
    }
  }
}
$chapterByModule=@{}
foreach($ch in $chapters){ foreach($module in $ch.modules){ $chapterByModule[[string]$module]=$ch.id } }
$visualByChapter=@{}
foreach($row in $mediaRows){
  $chapterId=$chapterByModule[[string]$row.module_id]
  if(-not $visualByChapter.ContainsKey($chapterId)){ $visualByChapter[$chapterId]=@() }
  $visualByChapter[$chapterId]+=$row
}
foreach($ch in $chapters){
  $chapterId=$ch.id
  if(-not $visualByChapter.ContainsKey($chapterId)){ continue }
  $visual=@('','## 视觉证据','','以下图片保留公式、图表、板书或屏幕操作的空间信息；正文仍是主要学习入口。','')
  foreach($row in @($visualByChapter[$chapterId])){
    $label="模块 $($row.module_id)"
    if($row.source_locator){ $label += "（$($row.source_locator)）" }
    $visual += "### $label"
    $visual += "![$label]($($row.path))"
    $visual += ''
  }
  Add-Content (Join-Path $package ($chapterId+'.md')) (($visual -join "`n")) -Encoding utf8
}
$index=@('---','babata_type: c2b_course_knowledge_base','course: 25春 MBAO5406 财务管理','variant: c2b','status: live_candidate','---','','# 财务管理知识库','','从 [[00-课程总览]] 开始。这里按知识和决策链组织，媒体证据按章节挂载。','','## 课程章节'); foreach($ch in $chapters){$index += "- [[$($ch.id)]]"}; $index += @('','## 学习工具','- [[09-公式与决策工具]]','- [[10-案例练习]]','- [[11-复习与自测]]',''); ($index -join "`n")|Set-Content (Join-Path $package 'index.md') -Encoding utf8
$chapterNavigationLinks=Test-CourseNavigation $package $chapters
$banned=@('本试点','C2 应当','外部主权库负责','不是正式 C2','provider','模型生成','控制面','C1B 视觉证据','C2B'); foreach($f in Get-ChildItem $package -File -Filter '*.md'){ $t=Get-Content $f -Raw; foreach($term in $banned){if($t.Contains($term)){throw "Control-plane contamination $($f.Name):$term"}}}
$manifest=[ordered]@{schema='babata.c1b-c2b.finance-final/v1';task=(Split-Path $staging -Leaf);variant='c2b';status='live_candidate';source_map=$mapPath;source_map_sha256=Hash $mapPath;c1b_decision=$decisionPath;c1b_decision_sha256=Hash $decisionPath;source_modules=37;c1b_decisions=37;media=$mediaRows;package_files=@(Get-ChildItem $package -Recurse -File|ForEach-Object{[ordered]@{path=Relative $package $_.FullName;sha256=Hash $_.FullName;bytes=$_.Length}});user_knowledge_has_full_c1=$false;formal_registration='not_started'}; $manifest|ConvertTo-Json -Depth 30|Set-Content (Join-Path $staging 'manifest.json') -Encoding utf8
$verify=[ordered]@{status='passed';c1=37;c1b=37;chapter_documents=8;source_documents=0;media_files=$mediaRows.Count;chapter_navigation_links=$chapterNavigationLinks;control_plane_hits=0;full_c1_user_layer=$false;formal_registration='not_started'}; $verify|ConvertTo-Json|Set-Content (Join-Path $staging 'verification.json') -Encoding utf8; @('# 财务管理 C1B→C2B 构建报告','','- C1：37/37 hash 校验通过','- C1B：37/37 独立判断完成',"- C1B 媒体：$($mediaRows.Count) 个候选物化",'- 章节导航：8/8 指向真实章节文件','- 用户知识层完整 C1：0 份','- 正式登记：未开始')|Set-Content (Join-Path $staging 'REPORT.md') -Encoding utf8
if($Publish){$archive=Join-Path (Split-Path $staging -Parent) 'user-export-archive'; New-Item $archive -Force|Out-Null; $candidate="$LiveVaultPath.publish-candidate"; if(Test-Path $candidate){Remove-Item $candidate -Recurse -Force}; New-Item $candidate -Force|Out-Null; Get-ChildItem $package -Force|Copy-Item -Destination $candidate -Recurse -Force; if(Test-Path $LiveVaultPath){Move-Item $LiveVaultPath (Join-Path $archive ('finance-live-before-final-'+(Get-Date -Format yyyyMMdd-HHmmss)))}; Move-Item $candidate $LiveVaultPath}
Write-Output "staged=$staging c1b=37 media=$($mediaRows.Count) published=$Publish"
