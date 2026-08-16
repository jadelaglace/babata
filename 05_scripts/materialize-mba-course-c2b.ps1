[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$LearningDocsManifestPath,
    [Parameter(Mandatory=$true)][string]$C1BRegistrationLedgerPath,
    [Parameter(Mandatory=$true)][string]$KnowledgeUniverseLedgerPath,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [string]$DataHome=$env:BABATA_DATA_HOME,
    [string]$RendererScript=(Join-Path $PSScriptRoot 'render-mba-course-map.ps1'),
    [string]$PackageCheckerScript=(Join-Path $PSScriptRoot 'check-mba-course-c2b-package.ps1')
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Is-Within([string]$Child,[string]$Parent){
    $c=[IO.Path]::GetFullPath($Child).TrimEnd('\');$p=[IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $c.Equals($p,[StringComparison]::OrdinalIgnoreCase) -or $c.StartsWith($p+'\',[StringComparison]::OrdinalIgnoreCase)
}
function Require-Text([object]$Value,[string]$Label){$text=[string]$Value;if([string]::IsNullOrWhiteSpace($text)){throw "$Label is required"};$text}
function Require-Hash([object]$Value,[string]$Label){$hash=[string]$Value;if($hash -cnotmatch '^[0-9a-f]{64}$'){throw "$Label must be canonical lowercase SHA256"};$hash}
function Assert-SameHash([string]$Path,[object]$Expected,[string]$Label){$expectedHash=Require-Hash $Expected "$Label hash";if((Hash $Path)-cne $expectedHash){throw "$Label hash mismatch"}}
function Safe-Basename([string]$Value,[string]$Label){
    $name=Require-Text $Value $Label
    if($name -match '[\\/:*?"<>|]' -or $name -in @('.','..')){throw "$Label is unsafe: $name"}
    $name
}
function Relative([string]$Root,[string]$Path){$prefix=$Root.TrimEnd('\')+'\';if(-not $Path.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw "Path outside package: $Path"};$Path.Substring($prefix.Length).Replace('\','/')}
function Flatten-SourceItems($SourceMap){$items=@();foreach($chunk in @($SourceMap.chunks)){foreach($item in @($chunk.items)){$items+=$item}};@($items)}
function Assert-ExactIds([object[]]$Actual,[string[]]$Expected,[string]$Label){
    $values=@($Actual|ForEach-Object{[string]$_})
    if($values.Count -ne $Expected.Count -or @($values|Sort-Object -Unique).Count -ne $values.Count -or
        (($values|Sort-Object)-join "`n") -cne (($Expected|Sort-Object)-join "`n")){throw "$Label module-id set does not equal the course denominator"}
}
function Resolve-Managed([string]$Logical,[string]$DataRoot,[string]$ManagedRoot,[string]$Label){
    if([string]::IsNullOrWhiteSpace($Logical) -or [IO.Path]::IsPathRooted($Logical)){throw "$Label logical path must be managed and relative"}
    $path=[IO.Path]::GetFullPath((Join-Path $DataRoot $Logical.Replace('/','\')))
    if(-not(Is-Within $path $ManagedRoot) -or -not(Test-Path -LiteralPath $path -PathType Leaf)){throw "$Label managed file is missing or outside derived storage"}
    $path
}
function Locator-Label($Locator){
    if($null -eq $Locator){return $null};$parts=@()
    foreach($name in @('page','time_seconds','percentage','crop')){if($Locator.PSObject.Properties[$name]){$parts+="$name=$($Locator.$name)"}}
    if($parts.Count){$parts -join ', '}else{$null}
}
function Media-Modality([string]$Extension){
    switch($Extension.ToLowerInvariant()){
        {$_ -in @('.png','.jpg','.jpeg','.gif','.webp','.svg')} {'image';break}
        {$_ -in @('.mp3','.wav','.m4a')} {'audio';break}
        {$_ -in @('.mp4','.webm','.mov')} {'video';break}
        '.pdf' {'attachment';break}
        default {throw "Unsupported registered media extension: $Extension"}
    }
}
function Resolve-MediaExtension($Registration,[string]$DerivedDatabase){
    $extension=[IO.Path]::GetExtension([string]$Registration.logical_path).ToLowerInvariant()
    if($extension){return $extension}
    $derivativeId=Require-Text $Registration.derivative_id 'registered media derivative_id'
    $escaped=$derivativeId.Replace("'","''")
    $mediaType=(& sqlite3 -readonly -noheader $DerivedDatabase "SELECT media_type FROM derivatives WHERE derivative_id='$escaped';" 2>&1 | Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($mediaType)){throw "Managed media_type read-back is missing: $derivativeId"}
    switch($mediaType.ToLowerInvariant()){
        'image/png' {'.png';break}
        'image/jpeg' {'.jpg';break}
        'image/jpg' {'.jpg';break}
        'image/gif' {'.gif';break}
        'image/webp' {'.webp';break}
        'image/svg+xml' {'.svg';break}
        'audio/mpeg' {'.mp3';break}
        'audio/wav' {'.wav';break}
        'audio/x-wav' {'.wav';break}
        'audio/mp4' {'.m4a';break}
        'video/mp4' {'.mp4';break}
        'video/webm' {'.webm';break}
        'video/quicktime' {'.mov';break}
        'application/pdf' {'.pdf';break}
        default {throw "Unsupported managed media_type: $mediaType ($derivativeId)"}
    }
}

if([string]::IsNullOrWhiteSpace($DataHome)){throw 'BABATA_DATA_HOME or -DataHome is required'}
$data=(Get-Item -LiteralPath $DataHome -ErrorAction Stop).FullName.TrimEnd('\')
$derivedDb=Join-Path $data '02_derived\index\derived.sqlite'
if(-not(Test-Path -LiteralPath $derivedDb -PathType Leaf)){throw "Managed derived database is missing: $derivedDb"}
$runtimeStaging=Join-Path $data '04_runtime\staging';$managedRoot=Join-Path $data '02_derived\files'
$staging=[IO.Path]::GetFullPath($StagingRoot).TrimEnd('\')
if(-not(Is-Within $staging $runtimeStaging)){throw 'C2B staging root must remain below BABATA_DATA_HOME/04_runtime/staging'}
if(Test-Path -LiteralPath $staging){throw "Use a fresh C2B staging root: $staging"}
$planPath=(Get-Item -LiteralPath $CoursePlanPath -ErrorAction Stop).FullName
$learningPath=(Get-Item -LiteralPath $LearningDocsManifestPath -ErrorAction Stop).FullName
$c1bPath=(Get-Item -LiteralPath $C1BRegistrationLedgerPath -ErrorAction Stop).FullName
$knowledgePath=(Get-Item -LiteralPath $KnowledgeUniverseLedgerPath -ErrorAction Stop).FullName
$renderer=(Get-Item -LiteralPath $RendererScript -ErrorAction Stop).FullName
$checker=(Get-Item -LiteralPath $PackageCheckerScript -ErrorAction Stop).FullName
foreach($path in @($planPath,$learningPath,$c1bPath,$knowledgePath,$renderer,$checker)){if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Missing materialization input: $path"}}

$plan=Get-Content -LiteralPath $planPath -Raw -Encoding utf8|ConvertFrom-Json
$learning=Get-Content -LiteralPath $learningPath -Raw -Encoding utf8|ConvertFrom-Json
$c1b=Get-Content -LiteralPath $c1bPath -Raw -Encoding utf8|ConvertFrom-Json
$knowledge=Get-Content -LiteralPath $knowledgePath -Raw -Encoding utf8|ConvertFrom-Json
$planSha=Hash $planPath;$learningSha=Hash $learningPath;$c1bSha=Hash $c1bPath;$knowledgeSha=Hash $knowledgePath
if([string]$plan.schema -cne 'babata.mba-course-c2b-plan/v1' -or [string]$plan.output_status -cne 'pending_user_acceptance'){throw 'Course plan must be MBA v1 and pending_user_acceptance'}
$course=Require-Text $plan.course 'course';$courseKey=Require-Text $plan.course_key 'course_key';$shortName=Safe-Basename ([string]$plan.short_name) 'short_name'
if($courseKey -notmatch '^[a-z0-9]+(?:-[a-z0-9]+)*$'){throw 'course_key must be a lowercase ASCII slug'}
$expected=[int]$plan.expected_modules;if($expected -lt 1){throw 'expected_modules must be positive'}
$planIds=@($plan.chapters|ForEach-Object{@($_.modules)}|ForEach-Object{[string]$_})
Assert-ExactIds $planIds @($planIds) 'course plan'
if($planIds.Count -ne $expected){throw 'Course plan chapter denominator mismatch'}
$chapterByModule=@{};foreach($chapter in @($plan.chapters)){$note=Safe-Basename ([string]$chapter.note) 'chapter note';foreach($module in @($chapter.modules)){$chapterByModule[[string]$module]=$note}}

if([string]$learning.schema -cne 'babata.mba-course-learning-docs/v1' -or [string]$learning.status -cne 'candidate' -or
    [string]$learning.course -cne $course -or [int]$learning.expected_modules -ne $expected -or [string]$learning.course_plan_sha256 -cne $planSha){throw 'Learning-doc manifest does not bind this pending course plan and denominator'}
if([int]$learning.complete_source_notes -ne $expected -or [int]$learning.chapter_documents -ne @($plan.chapters).Count -or
    [int]$learning.learning_documents -ne (1+@($plan.chapters).Count+3)){throw 'Learning-doc manifest coverage is incomplete'}
if([string]$c1b.schema -cne 'babata.mba-course-c1b-registration/v1' -or [string]$c1b.status -cne 'registered' -or
    [string]$c1b.course -cne $course -or [string]$c1b.course_key -cne $courseKey -or [string]$c1b.course_plan_sha256 -cne $planSha){throw 'Formal C1B ledger does not bind this course plan'}
if([string]$knowledge.schema -cne 'babata.mba-course-c2b-knowledge-registration/v1' -or [string]$knowledge.status -cne 'registered' -or
    [string]$knowledge.course_acceptance -cne 'pending_user_acceptance' -or [string]$knowledge.course -cne $course -or
    [string]$knowledge.plan_sha256 -cne $planSha -or [string]$knowledge.c1b_ledger_sha256 -cne $c1bSha -or [int]$knowledge.expected_modules -ne $expected){throw 'Knowledge-universe ledger does not bind this formal C1B course'}

$sourceMapPath=(Get-Item -LiteralPath ([string]$c1b.source_map) -ErrorAction Stop).FullName
Assert-SameHash $sourceMapPath $c1b.source_map_sha256 'C1B source map'
if([string]$learning.source_map_sha256 -cne [string]$c1b.source_map_sha256 -or [string]$knowledge.source_map_sha256 -cne [string]$c1b.source_map_sha256){throw 'Learning/C1B/knowledge source-map hashes disagree'}
$sourceMap=Get-Content -LiteralPath $sourceMapPath -Raw -Encoding utf8|ConvertFrom-Json
if([string]$sourceMap.schema -cne 'babata.mba.c2-source-map/v1' -or [string]$sourceMap.course -cne $course -or [int]$sourceMap.expected_modules -ne $expected){throw 'Source map does not bind the course denominator'}
$sourceItems=Flatten-SourceItems $sourceMap
Assert-ExactIds @($sourceItems.module_id) $planIds 'source map'
Assert-ExactIds @($learning.source_notes.module_id) $planIds 'learning manifest'
$registrations=@($c1b.registrations);Assert-ExactIds @($registrations.module_id) $planIds 'C1B ledger'
$knowledgeModules=@($knowledge.modules);Assert-ExactIds @($knowledgeModules.module_id) $planIds 'knowledge ledger'
if([int]$c1b.coverage.modules -ne $expected -or [int]$c1b.coverage.complete_c1_reused -ne $expected -or [int]$c1b.coverage.essence_decisions_registered -ne $expected){throw 'C1B ledger coverage is incomplete'}
$sourceByModule=@{};foreach($item in $sourceItems){$sourceByModule[[string]$item.module_id]=$item}
$learningSourceByModule=@{};foreach($item in @($learning.source_notes)){$learningSourceByModule[[string]$item.module_id]=$item}
foreach($registration in $registrations){
    $module=[string]$registration.module_id;$source=$sourceByModule[$module];$learningSource=$learningSourceByModule[$module]
    foreach($binding in @(
        @([string]$source.c0_item_id,[string]$registration.source_item_id,'C0 item'),
        @([string]$source.c0_revision_id,[string]$registration.source_revision_id,'C0 revision'),
        @([string]$source.c0_asset_id,[string]$registration.source_asset_id,'C0 asset'),
        @([string]$source.c0_asset_sha256,[string]$registration.source_asset_sha256,'C0 hash'),
        @([string]$source.c1_derivative_id,[string]$registration.complete_c1.derivative_id,'C1 derivative'),
        @([string]$source.c1_sha256,[string]$registration.complete_c1.output_sha256,'C1 hash'),
        @([string]$learningSource.input_derivative_id,[string]$registration.complete_c1.derivative_id,'learning C1 derivative'),
        @([string]$learningSource.input_sha256,[string]$registration.complete_c1.output_sha256,'learning C1 hash')
    )){
        if([string]::IsNullOrWhiteSpace([string]$binding[0]) -or [string]$binding[0] -cne [string]$binding[1]){throw "$($binding[2]) identity mismatch for module $module"}
    }
}

$generatedRoot=Join-Path (Split-Path $learningPath -Parent) 'generated'
if(-not(Test-Path -LiteralPath $generatedRoot -PathType Container)){throw 'Learning-doc generated root is missing'}
$learningNotes=@($plan.course_map.learning.nodes.note|ForEach-Object{[string]$_})
if($learningNotes.Count -ne 4 -or @($learningNotes|Sort-Object -Unique).Count -ne 4 -or $learningNotes -notcontains '视觉证据索引'){throw 'Course-map learning layer must contain three unique numbered learning documents and 视觉证据索引'}
$aidNotes=@();foreach($prefix in @('09-','10-','11-')){$matches=@($learningNotes|Where-Object{$_.StartsWith($prefix,[StringComparison]::Ordinal)});if($matches.Count -ne 1){throw "Course-map learning layer requires exactly one $prefix document"};$aidNotes+=Safe-Basename $matches[0] 'learning document'}
$expectedNotes=@('00-课程总览')+@($plan.chapters.note|ForEach-Object{[string]$_})+$aidNotes
$expectedFiles=@($expectedNotes|ForEach-Object{$_+'.md'})
$learningRows=@($learning.generated_files)
if($learningRows.Count -ne $expectedFiles.Count -or @($learningRows.name|Sort-Object -Unique).Count -ne $expectedFiles.Count -or
    (($learningRows.name|Sort-Object)-join "`n") -cne (($expectedFiles|Sort-Object)-join "`n")){throw 'Learning-doc manifest file set is not the exact reusable course document set'}
$learningBodies=@{};foreach($row in $learningRows){
    $name=Safe-Basename ([string]$row.name) 'learning file';$path=Join-Path $generatedRoot $name
    if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Missing learning document: $name"}
    Assert-SameHash $path $row.sha256 "learning document $name"
    $text=Get-Content -LiteralPath $path -Raw -Encoding utf8
    if($text.Length -ne [int]$row.chars -or $text.Length -lt 3000){throw "Learning document size mismatch or too thin: $name"}
    $learningBodies[$name]=$text
}

$decisionPath=(Get-Item -LiteralPath ([string]$c1b.decision_source) -ErrorAction Stop).FullName;Assert-SameHash $decisionPath $c1b.decision_source_sha256 'C1B decision source'
$decisions=@(Get-Content -LiteralPath $decisionPath -Raw -Encoding utf8|ConvertFrom-Json);Assert-ExactIds @($decisions.module_id) $planIds 'C1B decision source'
$decisionByModule=@{};foreach($decision in $decisions){$decisionByModule[[string]$decision.module_id]=$decision}
$derivativeIds=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal);$mediaWork=@()
foreach($row in $registrations){
    $module=[string]$row.module_id
    if([string]$row.chapter -cne [string]$chapterByModule[$module]){throw "C1B chapter binding mismatch: $module"}
    $decision=$decisionByModule[$module]
    if([string]$decision.c1_sha256 -cne [string]$row.complete_c1.output_sha256){throw "C1B decision and complete-C1 hash disagree: $module"}
    $decisionMedia=@($decision.retained_media|Where-Object{$null-ne$_}|ForEach-Object{[string]$_.sha256}|Sort-Object)
    $registeredMedia=@($row.media_registrations|Where-Object{$null-ne$_}|ForEach-Object{[string]$_.output_sha256}|Sort-Object)
    if(($decisionMedia -join "`n") -cne ($registeredMedia -join "`n")){throw "C1B decision and registered-media hashes disagree: $module"}
    foreach($derivative in @($row.complete_c1,$row.decision_registration)+@($row.media_registrations)){
        $id=Require-Text $derivative.derivative_id "derivative id $module";if(-not $derivativeIds.Add($id)){throw "Duplicate derivative id: $id"}
        $managed=Resolve-Managed ([string]$derivative.logical_path) $data $managedRoot "derivative $id";Assert-SameHash $managed $derivative.output_sha256 "derivative $id"
    }
    if([string]$row.decision_registration.registration -notin @('registered','reused')){throw "C1B decision is not formally registered: $module"}
    foreach($media in @($row.media_registrations)){
        if([string]$media.registration -notin @('registered','reused')){throw "C1B media is not formally registered: $module"}
        $managed=Resolve-Managed ([string]$media.logical_path) $data $managedRoot "media $module"
        $mediaWork += [pscustomobject]@{module=$module;chapter=[string]$chapterByModule[$module];source=$managed;registration=$media}
    }
}
if($mediaWork.Count -ne [int]$c1b.coverage.retained_media_registered){throw 'C1B retained-media denominator mismatch'}

$semanticIds=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
if([string]$knowledge.branch.name -cne [string]$plan.knowledge_universe.branch_name -or [string]::IsNullOrWhiteSpace([string]$knowledge.branch.id)){throw 'Knowledge-universe branch does not match the course plan'}
foreach($module in $knowledgeModules){
    $id=Require-Text $module.semantic_id "semantic id $($module.module_id)";if(-not $semanticIds.Add($id)){throw "Duplicate semantic id: $id"}
    if([string]$module.status -cne 'registered' -or [string]$module.review_state -cne 'accepted' -or [string]$module.assignment_state -cne 'assigned' -or
        [string]$module.chapter -cne [string]$chapterByModule[[string]$module.module_id]){throw "Knowledge registration is incomplete or misassigned: $($module.module_id)"}
    $semanticPackage=(Get-Item -LiteralPath ([string]$module.package_path) -ErrorAction Stop).FullName;Assert-SameHash $semanticPackage $module.fingerprint "semantic package $($module.module_id)"
}

# All authoritative inputs pass before the fresh, deletable package root is created.
$package=Join-Path $staging 'package';New-Item -ItemType Directory -Path $package -Force|Out-Null
$mediaRoot=Join-Path $package 'media';New-Item -ItemType Directory -Path $mediaRoot -Force|Out-Null
$forbidden=@('本试点','C2 应当','外部主权库负责','不是正式 C2','我们要求','作为 AI','作为AI')
$chapterSet=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal);foreach($name in @($plan.chapters.note)){[void]$chapterSet.Add(([string]$name)+'.md')}
$removedProvenanceSections=0
foreach($name in $expectedFiles){
    $text=[string]$learningBodies[$name]
    if($chapterSet.Contains($name)){
        $matches=[regex]::Matches($text,'(?ms)^## 来源索引\s*.*\z')
        if($matches.Count -ne 1){throw "Learning chapter must contain exactly one removable provenance index: $name"}
        $text=[regex]::Replace($text,'(?ms)^## 来源索引\s*.*\z','',1).Trim()+"`n"
        $removedProvenanceSections++
    } elseif($text -match '(?m)^## 来源索引\s*$'){throw "Non-chapter learning document contains provenance controls: $name"}
    foreach($token in $forbidden){if($text.Contains($token)){throw "Learning body contains control-plane residue: $name / $token"}}
    if($text.Contains('[[来源/')){throw "Learning body exposes the complete source layer: $name"}
    if($text -match '(?m)^## 视觉证据\s*$'){throw "Learning body already contains a visual-evidence section: $name"}
    Set-Content -LiteralPath (Join-Path $package $name) -Value $text -Encoding utf8
}

$mediaRows=@();$byChapter=@{}
foreach($work in $mediaWork){
    $ext=Resolve-MediaExtension $work.registration $derivedDb;$modality=Media-Modality $ext
    $derivativeId=([string]$work.registration.derivative_id) -replace '[^A-Za-z0-9_-]','_'
    $fileName="M-$($work.module)-$derivativeId$ext";$destination=Join-Path $mediaRoot $fileName
    if(Test-Path -LiteralPath $destination){throw "Media destination collision: $fileName"}
    Copy-Item -LiteralPath $work.source -Destination $destination
    Assert-SameHash $destination $work.registration.output_sha256 "copied media $fileName"
    $row=[ordered]@{module_id=$work.module;chapter=$work.chapter;path='media/'+$fileName;sha256=Hash $destination;bytes=(Get-Item $destination).Length;modality=$modality;role=[string]$work.registration.role;source_locator=$work.registration.source_locator;derivative_id=[string]$work.registration.derivative_id}
    $mediaRows+=$row;if(-not $byChapter.ContainsKey($work.chapter)){$byChapter[$work.chapter]=@()};$byChapter[$work.chapter]+=$row
}
foreach($chapter in @($plan.chapters.note|ForEach-Object{[string]$_})){
    if(-not $byChapter.ContainsKey($chapter)){continue};$section=@('','## 视觉证据','','以下媒体保留文字无法替代的公式、图表、板书或空间信息；正文仍是主要学习入口。','')
    foreach($row in @($byChapter[$chapter])){$label="模块 $($row.module_id)";$locator=Locator-Label $row.source_locator;if($locator){$label+="（$locator）"};$section+="### $label";switch($row.modality){'image'{$section+="![$label]($($row.path))"};default{$section+="[$label]($($row.path))"}};$section+=''}
    Add-Content -LiteralPath (Join-Path $package ($chapter+'.md')) -Value ($section -join "`n") -Encoding utf8
}
$visual=@('# 视觉证据索引','','按章节列出 C1B 正式登记并保留的必要媒体。','')
if(-not $mediaRows.Count){$visual+='本课程的正式 C1B 判断未要求额外媒体。'}else{foreach($chapter in @($plan.chapters.note|ForEach-Object{[string]$_})){if(-not $byChapter.ContainsKey($chapter)){continue};$visual+="## $chapter";foreach($row in @($byChapter[$chapter])){if($row.modality -eq 'image'){$visual+="- ![模块 $($row.module_id)]($($row.path))"}else{$visual+="- [模块 $($row.module_id)]($($row.path))"}};$visual+=''}}
Set-Content -LiteralPath (Join-Path $package '视觉证据索引.md') -Value (($visual -join "`n")+"`n") -Encoding utf8

$index=@('---','babata_type: c2b_course_knowledge_base',"course: $course",'variant: c2b','status: pending_user_acceptance','formal_registration: registered','c1b_registration: registered','knowledge_universe_registration: registered','template_profile: semantic-obsidian/v1','template_status: accepted','---','',"# $shortName 知识库",'', '从 [[00-课程总览]] 开始。这里按知识和决策链组织，媒体证据按章节挂载。','','## 课程章节')
foreach($chapter in @($plan.chapters.note|ForEach-Object{[string]$_})){$index+="- [[$chapter]]"}
$index+=@('','## 学习工具')+@($aidNotes|ForEach-Object{"- [[$_]]"})+@('- [[视觉证据索引]]','')
Set-Content -LiteralPath (Join-Path $package 'index.md') -Value ($index -join "`n") -Encoding utf8

$assetBase=$shortName+'课程脑图'
$mapSpec=[ordered]@{schema='babata.mba-course-map-spec/v1';status='pending_user_acceptance';course=$course;course_plan_sha256=$planSha;classification_axis=[string]$plan.course_map.classification_axis;root_id=[string]$plan.course_map.root_id;root_label=[string]$plan.course_map.root_label;tagline=[string]$plan.course_map.tagline;asset_basename=$assetBase;domains=@($plan.course_map.domains);learning=$plan.course_map.learning}
$mapSpec|ConvertTo-Json -Depth 20|Set-Content -LiteralPath (Join-Path $mediaRoot 'course-map.spec.json') -Encoding utf8
$renderResult=@(& $renderer -PackageRoot $package)
if($renderResult.Count -ne 1 -or [string]$renderResult[0].schema -cne 'babata.mba-course-map-render/v1' -or [string]$renderResult[0].status -cne 'passed'){throw 'Course-map renderer did not return one passed result'}
$map=$renderResult[0]

$manifest=[ordered]@{
    schema='babata.mba-course-c2b/v1';course=$course;course_key=$courseKey;status='pending_user_acceptance';course_plan=$planPath;course_plan_sha256=$planSha
    module_ids=@($planIds|Sort-Object);source_map=$sourceMapPath;source_map_sha256=Hash $sourceMapPath
    c1b_ledger_sha256=$c1bSha;knowledge_ledger_sha256=$knowledgeSha;learning_docs_manifest=$learningPath;learning_docs_manifest_sha256=$learningSha
    formal_registration='registered'
    c1b_registration=[ordered]@{status='registered';ledger=$c1bPath;decisions=$registrations.Count;media=$mediaRows.Count;decision_derivative_ids=@($registrations.decision_registration.derivative_id);media_derivative_ids=@($registrations.media_registrations.derivative_id)}
    knowledge_universe=[ordered]@{status='registered';ledger=$knowledgePath;foundation=$plan.knowledge_universe.foundation_id;discipline=$plan.knowledge_universe.discipline_id;branch=$knowledge.branch;semantic_ids=@($knowledgeModules.semantic_id)}
    obsidian_template=[ordered]@{profile='semantic-obsidian/v1';status='accepted'}
    publication=[ordered]@{live_path=[string]$plan.live.path;vault=[string]$plan.live.vault;file=[string]$plan.live.file}
    course_map=[ordered]@{mermaid=[string]$map.mermaid;png=[string]$map.png;classification_axis=[string]$map.classification_axis;layout=[string]$map.layout;mece_domains=[int]$map.mece_domains;knowledge_details=[int]$map.knowledge_details;internal_link_targets=[int]$map.internal_link_targets;responsive_svg=[bool]$map.responsive_svg;default_expanded=[string]$map.default_expanded;png_default_collapsed=[bool]$map.png_default_collapsed;png_display_width=[int]$map.png_display_width;png_width=[int]$map.png_width;png_height=[int]$map.png_height;effective_font_px=[double]$map.effective_font_px;aspect_ratio=[double]$map.aspect_ratio}
    media=$mediaRows;user_knowledge_has_full_c1=$false;removed_learning_provenance_sections=$removedProvenanceSections;old_c2b_inputs=0;external_sovereign_original_reads=0
    package_files=@(Get-ChildItem -LiteralPath $package -Recurse -File -Force|Sort-Object FullName|ForEach-Object{[ordered]@{path=Relative $package $_.FullName;sha256=Hash $_.FullName;bytes=[long]$_.Length}})
}
$manifestPath=Join-Path $staging 'manifest.json';$manifest|ConvertTo-Json -Depth 30|Set-Content -LiteralPath $manifestPath -Encoding utf8
$checkResult=@(& $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath)
if($checkResult.Count -ne 1 -or [string]$checkResult[0].schema -cne 'babata.mba-course-c2b-package-check/v1' -or [string]$checkResult[0].status -cne 'passed'){throw 'Materialized package did not pass the formal package checker'}
$verification=[ordered]@{schema='babata.mba-course-c2b-materialization-verification/v1';status='passed_engineering_gates';course_acceptance='pending_user_acceptance';course=$course;course_plan_sha256=$planSha;source_modules=$expected;c1b_decisions=$registrations.Count;c1b_media=$mediaRows.Count;knowledge_entries=$knowledgeModules.Count;package_files=[int]$checkResult[0].package_files;wiki_links=[int]$checkResult[0].wiki_links;markdown_links=[int]$checkResult[0].markdown_links;old_c2b_inputs=0;external_sovereign_original_reads=0}
$verification|ConvertTo-Json -Depth 12|Set-Content -LiteralPath (Join-Path $staging 'verification.json') -Encoding utf8
@("# $shortName C2B materialization report",'', '- status: passed_engineering_gates', '- user acceptance: pending_user_acceptance', "- complete C1B denominator: $expected/$expected", "- formally registered retained media: $($mediaRows.Count)", "- knowledge-universe entries: $($knowledgeModules.Count)/$expected", "- package files: $($checkResult[0].package_files)", '- old C2B inputs: 0', '- external sovereign original reads: 0')|Set-Content -LiteralPath (Join-Path $staging 'REPORT.md') -Encoding utf8
[pscustomobject][ordered]@{schema='babata.mba-course-c2b-materialization/v1';status='passed_engineering_gates';course_acceptance='pending_user_acceptance';staging=$staging;package=$package;manifest=$manifestPath;verification=(Join-Path $staging 'verification.json');files=[int]$checkResult[0].package_files}
