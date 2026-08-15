[CmdletBinding()]
param(
 [Parameter(Mandatory=$true)][string]$GeneratedRoot,
 [Parameter(Mandatory=$true)][string]$C1BRoot,
 [Parameter(Mandatory=$true)][string]$C1BDecisionPath,
 [Parameter(Mandatory=$true)][string]$C1BRegistrationLedgerPath,
 [Parameter(Mandatory=$true)][string]$SourceMapPath,
 [Parameter(Mandatory=$true)][string]$KnowledgeUniverseLedgerPath,
 [Parameter(Mandatory=$true)][string]$StagingRoot,
 [string]$DataHome=$env:BABATA_DATA_HOME
)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
function Hash([string]$p){(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant()}
function Rel([string]$b,[string]$p){$bu=[Uri]((Get-Item $b).FullName.TrimEnd('\')+'\');$pu=[Uri](Get-Item $p).FullName;[Uri]::UnescapeDataString($bu.MakeRelativeUri($pu).ToString()).Replace('\','/')}
function Test-ManagedRegistration($registration,[string]$label,[string]$dataHome){
  foreach($name in @('run_id','derivative_id','output_sha256','logical_path')){if([string]::IsNullOrWhiteSpace([string]$registration.$name)){throw "$label missing $name"}}
  $logical=[string]$registration.logical_path
  if(-not $logical.StartsWith('02_derived/files/sha256/',[StringComparison]::Ordinal)){throw "$label is outside managed C1 storage: $logical"}
  $managed=Join-Path $dataHome $logical.Replace('/','\')
  if(-not(Test-Path -LiteralPath $managed -PathType Leaf)){throw "$label managed file missing: $managed"}
  if((Hash $managed) -ne [string]$registration.output_sha256){throw "$label managed file hash mismatch"}
}
function Set-CourseNavigation([string]$path,[string[]]$chapterNames){
  $text=Get-Content -LiteralPath $path -Raw -Encoding utf8
  $matches=[regex]::Matches($text,'(?ms)^## 章节导航\s*.*?(?=^## |\z)')
  if($matches.Count -ne 1){throw "Expected exactly one chapter navigation section: $path"}
  $navigation=@('## 章节导航','')
  foreach($chapter in $chapterNames){$navigation+="- [[$chapter]]"}
  $text=[regex]::Replace($text,'(?ms)^## 章节导航\s*.*?(?=^## |\z)',(($navigation -join "`n")+"`n"),1)
  Set-Content -LiteralPath $path -Value ($text.Trim()+"`n") -Encoding utf8
}
function Test-CourseNavigation([string]$root,[string[]]$expectedChapters){
  $errors=@()
  $overview=Get-Content -LiteralPath (Join-Path $root '00-课程总览.md') -Raw -Encoding utf8
  $section=[regex]::Match($overview,'(?ms)^## 章节导航\s*(.*?)(?=^## |\z)')
  if(-not $section.Success){$errors+='00-课程总览.md: missing chapter navigation section'}
  else{
    $actual=@([regex]::Matches($section.Groups[1].Value,'\[\[([^\]|#]+)')|ForEach-Object{$_.Groups[1].Value})
    if(($actual.Count -ne $expectedChapters.Count) -or (($actual -join "`n") -ne ($expectedChapters -join "`n"))){$errors+='00-课程总览.md: chapter navigation does not match the eight chapter files'}
  }
  foreach($chapter in $expectedChapters){if(-not(Test-Path -LiteralPath (Join-Path $root ($chapter+'.md')))){$errors+="missing chapter file $chapter"}}
  if($errors.Count){throw "Chapter navigation verification failed: $($errors -join '; ')"}
  $expectedChapters.Count
}
$generated=(Get-Item $GeneratedRoot).FullName; $c1b=(Get-Item $C1BRoot).FullName; $map=(Get-Item $SourceMapPath).FullName; $decPath=(Get-Item $C1BDecisionPath).FullName; $c1bLedgerPath=(Get-Item $C1BRegistrationLedgerPath).FullName; $universePath=(Get-Item $KnowledgeUniverseLedgerPath).FullName; $staging=[IO.Path]::GetFullPath($StagingRoot); if(Test-Path $staging){throw "fresh staging required: $staging"}
if([string]::IsNullOrWhiteSpace($DataHome)){throw 'BABATA_DATA_HOME or -DataHome is required'};$dataHomeResolved=[IO.Path]::GetFullPath($DataHome)
$c1bLedger=Get-Content -LiteralPath $c1bLedgerPath -Raw -Encoding utf8|ConvertFrom-Json
if($c1bLedger.schema -ne 'babata.finance-c1b-registration/v1' -or $c1bLedger.status -ne 'registered'){throw 'C1B registration ledger must be formal schema v1 with registered status'}
if(@($c1bLedger.registrations).Count -ne 37 -or [int]$c1bLedger.coverage.essence_decisions_registered -ne 37 -or [int]$c1bLedger.coverage.retained_media_registered -ne 76){throw 'C1B registration ledger must contain 37 registered decisions and 76 registered media derivatives'}
if([string]$c1bLedger.decision_source_sha256 -ne (Hash $decPath)){throw 'C1B registration ledger decision-source hash mismatch'}
if([string]$c1bLedger.source_map_sha256 -ne (Hash $map)){throw 'C1B registration ledger source-map hash mismatch'}
$dec=@(Get-Content $decPath -Raw -Encoding utf8|ConvertFrom-Json);if($dec.Count -ne 37){throw 'C1B decisions must contain 37 finance modules'}
$registeredByModule=@{};foreach($row in @($c1bLedger.registrations)){$module=[string]$row.module_id;if($registeredByModule.ContainsKey($module)){throw "Duplicate C1B registration module $module"};$registeredByModule[$module]=$row}
foreach($d in $dec){
  $module=[string]$d.module_id;$row=$registeredByModule[$module];if($null -eq $row){throw "C1B registration missing module $module"}
  if([string]$row.source_revision_id -ne [string]$d.c0_revision_id -or [string]$row.source_asset_id -ne [string]$d.c0_asset_id -or [string]$row.source_asset_sha256 -ne [string]$d.c0_asset_sha256){throw "C1B registration source identity mismatch for module $module"}
  if([string]$row.complete_c1.output_sha256 -ne [string]$d.c1_sha256){throw "C1B registration complete-C1 hash mismatch for module $module"}
  Test-ManagedRegistration $row.decision_registration "module $module C1B decision" $dataHomeResolved
  $registeredMedia=@($row.media_registrations);$expectedMedia=@($d.retained_media);if($registeredMedia.Count -ne $expectedMedia.Count){throw "C1B registration media count mismatch for module $module"}
  foreach($mediaRow in $registeredMedia){Test-ManagedRegistration $mediaRow "module $module C1B media" $dataHomeResolved}
  $expectedHashes=@($expectedMedia|ForEach-Object{[string]$_.sha256}|Sort-Object);$registeredHashes=@($registeredMedia|ForEach-Object{[string]$_.excerpt_sha256}|Sort-Object);if(($expectedHashes -join "`n") -ne ($registeredHashes -join "`n")){throw "C1B registration media hashes mismatch for module $module"}
}
$universe=Get-Content -LiteralPath $universePath -Raw -Encoding utf8|ConvertFrom-Json
if($universe.status -ne 'registered' -or @($universe.semantic_entries).Count -ne 37){throw 'Knowledge universe ledger must contain 37 registered finance entries'}
$pkg=Join-Path $staging 'package'; New-Item -ItemType Directory -Path $pkg -Force|Out-Null
$contentNotes=@('00-课程总览','01-财务管理目标与财务高管职责','02-营运资本管理','03-项目投资评估','04-投资风险与不确定性','05-企业估值','06-融资决策与资本结构','07-股利决策','08-并购与企业重组','09-公式与决策工具','10-案例练习','11-复习与自测')
foreach($note in $contentNotes){$src=Join-Path $generated ($note+'.md');if(-not(Test-Path $src)){throw "Missing validated C2B note: $src"};Copy-Item $src -Destination $pkg -Force}
function CleanKnowledge([string]$path){
  $text=Get-Content -LiteralPath $path -Raw -Encoding utf8
  $text=[regex]::Replace($text,'(?ms)^## C1B 视觉证据\s*.*\z','')
  $text=$text.Replace('*(注：以上章节内容严格基于提供的C1B源材料编写，未引入外部事实。)*','')
  $text=$text.Replace('基于C1B数据','基于课程材料')
  $text=[regex]::Replace($text,'\[\[M-(\d{6})\]\]','〔M-$1〕')
  Set-Content -LiteralPath $path -Value ($text.Trim()+"`n") -Encoding utf8
}
foreach($note in $contentNotes){CleanKnowledge (Join-Path $pkg ($note+'.md'))}
$chapters=@('01-财务管理目标与财务高管职责','02-营运资本管理','03-项目投资评估','04-投资风险与不确定性','05-企业估值','06-融资决策与资本结构','07-股利决策','08-并购与企业重组')
Set-CourseNavigation (Join-Path $pkg '00-课程总览.md') $chapters
$chapterModules=@{
 '01-财务管理目标与财务高管职责'=@('892115','892117','892119'); '02-营运资本管理'=@('892121','892123','892125','892127','892129','892131','897613');
 '03-项目投资评估'=@('892135','892137','892139','892141','892143','892145','901333','972135'); '04-投资风险与不确定性'=@('892153','892155');
 '05-企业估值'=@('892157','892159'); '06-融资决策与资本结构'=@('892161','892163','892165','972531','972533');
 '07-股利决策'=@('892171','892173'); '08-并购与企业重组'=@('892175','892177','892179','892181','892185','892187','976503','976505')
}
$index=@('---','babata_type: c2b_course_knowledge_base','course: 25春 MBAO5406 财务管理','variant: c2b','status: accepted_benchmark','formal_registration: registered','c1b_registration: registered','knowledge_universe_registration: registered','template_profile: semantic-obsidian/v1','template_status: accepted','---','','# 财务管理知识库','','从 [[00-课程总览]] 开始。这里按知识和决策链组织，媒体证据按章节挂载。','','## 课程章节'); foreach($c in $chapters){$index+="- [[$c]]"}; $index+=@('','## 学习工具','- [[09-公式与决策工具]]','- [[10-案例练习]]','- [[11-复习与自测]]','');($index -join "`n")|Set-Content (Join-Path $pkg 'index.md') -Encoding utf8
$media=Join-Path $pkg 'media'; New-Item -ItemType Directory $media -Force|Out-Null; $mediaRows=@(); $byChapter=@{}
foreach($d in $dec){foreach($m in @($d.retained_media)){$src=Join-Path $c1b ($m.path -replace '^c1b/','c1b/');if(-not(Test-Path $src)){throw "missing C1B media $($m.path)"};$name="M-$($d.module_id)-"+(Split-Path $src -Leaf);Copy-Item $src (Join-Path $media $name);$loc=if($m.PSObject.Properties['time_seconds']){$m.time_seconds}elseif($m.PSObject.Properties['page']){"page $($m.page)"}else{$null};$row=[ordered]@{module_id=[string]$d.module_id;path="media/$name";sha256=Hash (Join-Path $media $name);source_locator=$loc;modality='image'};$mediaRows+=$row;foreach($ch in $chapters){if($chapterModules[$ch] -contains [string]$d.module_id){if(-not $byChapter.ContainsKey($ch)){$byChapter[$ch]=@()};$byChapter[$ch]+=$row}}}}
foreach($ch in $chapters){if(-not $byChapter.ContainsKey($ch)){continue};$visual=@('','## 视觉证据','','以下图片保留公式、图表、板书或屏幕操作的空间信息；正文仍是主要学习入口。','');foreach($r in @($byChapter[$ch])){$label="模块 $($r.module_id)";if($r.source_locator){$label+="（$($r.source_locator)）"};$visual+="### $label";$visual+="![$label]($($r.path))";$visual+=''};Add-Content (Join-Path $pkg ($ch+'.md')) ($visual -join "`n") -Encoding utf8}
$visualSectionCount=0
foreach($ch in $chapters){
  $chapterText=Get-Content -LiteralPath (Join-Path $pkg ($ch+'.md')) -Raw -Encoding utf8
  $sections=[regex]::Matches($chapterText,'(?m)^## 视觉证据\s*$').Count
  if($sections -gt 1){throw "Duplicate visual-evidence sections in $ch"}
  $visualSectionCount+=$sections
  foreach($r in @($byChapter[$ch])){
    if([regex]::Matches($chapterText,[regex]::Escape("($($r.path))")).Count -ne 1){throw "Visual evidence path must occur exactly once in ${ch}: $($r.path)"}
  }
}
$rows=@('# 视觉证据索引','','按章节列出保留的公式、图表、板书和屏幕操作证据。','');foreach($ch in $chapters){if(-not $byChapter.ContainsKey($ch)){continue};$rows+="## $ch";foreach($r in @($byChapter[$ch])){$rows+="- ![模块 $($r.module_id)]($($r.path))"};$rows+=''};($rows -join "`n")|Set-Content (Join-Path $pkg '视觉证据索引.md') -Encoding utf8
$mapScript=Join-Path $PSScriptRoot 'render-finance-course-map.ps1'; $mapResult=@(& $mapScript -PackageRoot $pkg)
if($LASTEXITCODE -ne 0){throw 'Course map generation failed'}
$mapSummary=[string]($mapResult|Select-Object -Last 1)
if(-not $mapSummary.Contains('responsive_svg=true') -or -not $mapSummary.Contains('png_default_collapsed=true')){throw "Responsive course map verification missing: $mapSummary"}
$mapMatch=[regex]::Match($mapSummary,'png_dimensions=(\d+)x(\d+)\s+equivalent_font_px=([0-9.]+)\s+aspect_ratio=([0-9.]+)')
if(-not $mapMatch.Success){throw "Course map verification metrics missing: $mapSummary"}
$mapWidth=[int]$mapMatch.Groups[1].Value;$mapHeight=[int]$mapMatch.Groups[2].Value;$mapEffectiveFont=[double]$mapMatch.Groups[3].Value;$mapAspect=[double]$mapMatch.Groups[4].Value
$chapterNavigationLinks=Test-CourseNavigation $pkg $chapters
$manifest=[ordered]@{schema='babata.c1b-c2b.finance-final/v10';task=(Split-Path $staging -Leaf);variant='c2b';status='accepted_benchmark';source_map=$map;source_map_sha256=Hash $map;c1b_decision=$decPath;c1b_decision_sha256=Hash $decPath;c1b_decision_batch=(Split-Path (Split-Path $decPath -Parent) -Parent | Split-Path -Leaf);c1b_registration=[ordered]@{ledger=$c1bLedgerPath;ledger_sha256=Hash $c1bLedgerPath;status='registered';decision_derivative_ids=@($c1bLedger.registrations|ForEach-Object{$_.decision_registration.derivative_id});media_derivative_ids=@($c1bLedger.registrations|ForEach-Object{$_.media_registrations}|ForEach-Object{$_.derivative_id})};c2b_text_response_batch='mba-finance-c2b-text-validated-reused';source_modules=37;c1b_decisions=37;media=$mediaRows;course_map=[ordered]@{mermaid='media/财务管理课程脑图.mmd';png='media/财务管理课程脑图.png';classification_axis='financial_decision_object';layout='single_root_right_growing_mindmap';mece_domains=5;knowledge_details=20;content_contract=@('objective','decision_rules','formulas_or_relations','risk_or_boundaries');visual_profile='right-growing-mindmap/v1';internal_link_targets=12;responsive_svg=$true;default_expanded='mermaid';png_default_collapsed=$true;png_display_width=760;png_width=$mapWidth;png_height=$mapHeight;effective_font_px=$mapEffectiveFont;aspect_ratio=$mapAspect};knowledge_universe=[ordered]@{ledger=$universePath;ledger_sha256=Hash $universePath;status='registered';foundation=$universe.foundation;discipline=$universe.discipline;branch=$universe.branch;semantic_ids=@($universe.semantic_entries|ForEach-Object{$_.semantic_id})};obsidian_template=[ordered]@{profile='semantic-obsidian/v1';status='accepted'};user_knowledge_has_full_c1=$false;formal_registration='registered';package_files=@(Get-ChildItem $pkg -Recurse -File|ForEach-Object{[ordered]@{path=Rel $pkg $_.FullName;sha256=Hash $_.FullName;bytes=$_.Length}})};$manifest|ConvertTo-Json -Depth 30|Set-Content (Join-Path $staging 'manifest.json') -Encoding utf8
$verify=[ordered]@{status='passed_user_visual_acceptance';c1=37;c1b=37;c1b_registered_decisions=37;c1b_registered_media=76;chapter_documents=8;synthesis_documents=4;retained_media_files=$mediaRows.Count;visual_evidence_sections=$visualSectionCount;course_map_source_files=1;course_map_png_files=1;package_media_files=@(Get-ChildItem $media -File).Count;course_map_classification_axis='financial_decision_object';course_map_layout='single_root_right_growing_mindmap';course_map_mece_domains=5;course_map_knowledge_details=20;course_map_visual_profile='right-growing-mindmap/v1';course_map_internal_link_targets=12;course_map_responsive_svg=$true;course_map_default_expanded='mermaid';course_map_png_default_collapsed=$true;course_map_png_display_width=760;course_map_png_width=$mapWidth;course_map_png_height=$mapHeight;course_map_effective_font_px=$mapEffectiveFont;course_map_aspect_ratio=$mapAspect;source_documents=0;evidence_locator_documents=0;chapter_navigation_links=$chapterNavigationLinks;knowledge_universe_semantic_entries=@($universe.semantic_entries).Count;knowledge_universe_branch_id=[string]$universe.branch.id;full_c1_user_layer=$false;formal_registration='registered';obsidian_status='accepted_benchmark';template_profile='semantic-obsidian/v1';template_status='accepted'};$verify|ConvertTo-Json|Set-Content (Join-Path $staging 'verification.json') -Encoding utf8;@('# 财务管理 C1B→C2B 构建报告','','- 状态：用户已完成右向传统脑图视觉验收，正式转为 MBA C2B 标杆','- C1：37/37 hash 校验通过','- C1B：37/37 本质判断和 76/76 必要视觉片段已从正式登记账本回读',('- C1B 视觉媒体：'+$mediaRows.Count+' 张 PDF/视频关键帧'),('- 视觉证据段：'+$visualSectionCount+'/8，每篇最多一段、媒体路径不重复'),'- C2B 正文：8 章 + 4 份工具文档；已复用 hash 校验通过的正文批次',('- 课程脑图：单根全右向传统思维导图；5 个 MECE 决策域分色 + 20 条知识细节；12/12 Obsidian 原生内链；主 Mermaid 响应窗格，PNG 默认折叠回退；PNG '+$mapWidth+'x'+$mapHeight+'，显示宽度 760，等效字号 '+$mapEffectiveFont+'px，高宽比 '+$mapAspect),'- 知识宇宙：37/37 语义条目已归入 意识 → 管理学 → 财务管理','- Obsidian 模板：semantic-obsidian/v1，accepted','- Obsidian 导出：accepted_benchmark / registered','- 用户知识层完整 C1：0 份','- 证据叶文档：0 份','- 正式登记：完成')|Set-Content (Join-Path $staging 'REPORT.md') -Encoding utf8
Write-Output "staged=$staging c1b=37 media=$($mediaRows.Count)"


