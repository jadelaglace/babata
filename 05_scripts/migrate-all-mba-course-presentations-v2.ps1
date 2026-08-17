[CmdletBinding()]
param(
    [string]$RuntimeRoot='D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v2',
    [string]$PlanRoot='D:\BabataData\04_runtime\staging\model-workspaces\mba-course-plans',
    [string]$VaultRoot='C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA',
    [string]$FinanceLedgerPath='D:\BabataData\04_runtime\staging\model-workspaces\mba-finance-knowledge-universe-registration-20260813-v1\knowledge-universe-registration.json'
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Write-Json([object]$Value,[string]$Path){[IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 50),[Text.UTF8Encoding]::new($false))}
function Copy-Tree([string]$Source,[string]$Destination){
    if(Test-Path -LiteralPath $Destination){throw "Copy destination already exists: $Destination"}
    [void](New-Item -ItemType Directory -Path $Destination)
    foreach($entry in Get-ChildItem -LiteralPath $Source -Force){Copy-Item -LiteralPath $entry.FullName -Destination $Destination -Recurse -Force}
}
function Is-Within([string]$Child,[string]$Parent){
    $childPath=[IO.Path]::GetFullPath($Child).TrimEnd('\');$parentPath=[IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $childPath.StartsWith($parentPath+'\',[StringComparison]::OrdinalIgnoreCase)
}
function Assert-SourceLiveMatches([string]$Live,[string]$ManifestPath){
    $manifest=Get-Content -LiteralPath $ManifestPath -Raw -Encoding utf8|ConvertFrom-Json
    $declared=@($manifest.package_files);$files=@(Get-ChildItem -LiteralPath $Live -Recurse -File)
    if(-not $declared.Count -or $files.Count -ne $declared.Count){throw "Source live does not match manifest file count: $Live"}
    $predecessorPackage=Join-Path (Split-Path $ManifestPath -Parent) 'package'
    if(-not(Test-Path -LiteralPath $predecessorPackage -PathType Container)){throw "Predecessor package is missing: $predecessorPackage"}
    $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach($row in $declared){
        $relative=[string]$row.path
        if(-not $seen.Add($relative)){throw "Source manifest duplicates path: $relative"}
        $path=Join-Path $Live $relative.Replace('/','\');$predecessor=Join-Path $predecessorPackage $relative.Replace('/','\')
        if((-not(Test-Path -LiteralPath $path -PathType Leaf)) -or (-not(Test-Path -LiteralPath $predecessor -PathType Leaf)) -or (Hash $predecessor) -cne [string]$row.sha256){throw "Source or predecessor file is not manifest-bound: $relative"}
        if((Hash $path) -cne [string]$row.sha256){
            if([IO.Path]::GetExtension($relative) -cne '.md'){throw "Source live binary hash mismatch: $path"}
            $liveText=([IO.File]::ReadAllText($path,[Text.Encoding]::UTF8).Replace("`r`n","`n")).TrimEnd("`n")
            $predecessorText=([IO.File]::ReadAllText($predecessor,[Text.Encoding]::UTF8).Replace("`r`n","`n")).TrimEnd("`n")
            if($liveText -cne $predecessorText){throw "Source live Markdown differs semantically from predecessor: $path"}
        }
    }
}
function Assert-LiveMatches([string]$Live,[string]$ManifestPath){
    $manifest=Get-Content -LiteralPath $ManifestPath -Raw -Encoding utf8|ConvertFrom-Json
    $files=@(Get-ChildItem -LiteralPath $Live -Recurse -File)
    if($files.Count -ne @($manifest.package_files).Count){throw "Published live file count mismatch: $Live"}
    foreach($row in @($manifest.package_files)){
        $path=Join-Path $Live ([string]$row.path).Replace('/','\')
        if(-not(Test-Path -LiteralPath $path -PathType Leaf) -or (Hash $path) -cne [string]$row.sha256){throw "Published live hash mismatch: $path"}
    }
}

$runtime=[IO.Path]::GetFullPath($RuntimeRoot)
if(Test-Path -LiteralPath $runtime){throw "Fresh rollout root already exists: $runtime"}
$planRootResolved=(Get-Item -LiteralPath $PlanRoot -ErrorAction Stop).FullName
$vaultRootResolved=(Get-Item -LiteralPath $VaultRoot -ErrorAction Stop).FullName
$financeLedgerResolved=(Get-Item -LiteralPath $FinanceLedgerPath -ErrorAction Stop).FullName

$rows=@(
    [ordered]@{key='business-analytics';plan='business-analytics-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-business-analytics-c2b-20260816-v3\manifest.json'},
    [ordered]@{key='data-security-ethics-risk';plan='data-security-ethics-risk-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-data-security-ethics-risk-c2b-20260816-v1\manifest.json'},
    [ordered]@{key='decision-accounting';plan='decision-accounting-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-decision-accounting-c2b-20260816-v8\manifest.json'},
    [ordered]@{key='executive-business-communication';plan='executive-business-communication-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-executive-business-communication-c2b-20260816-v3\manifest.json'},
    [ordered]@{key='global-business-environment';plan='global-business-environment-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-global-business-environment-c2b-20260816-v1\manifest.json'},
    [ordered]@{key='managerial-economics';plan='managerial-economics-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-managerial-economics-c2b-20260816-v1\manifest.json'},
    [ordered]@{key='mba-primer';plan='mba-primer-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-primer-c2b-20260816-v8\manifest.json'},
    [ordered]@{key='organizational-behavior';plan='organizational-behavior-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-organizational-behavior-c2b-20260816-v1\manifest.json'},
    [ordered]@{key='strategic-leadership';plan='strategic-leadership-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-leadership-c2b-20260816-v2\manifest.json'},
    [ordered]@{key='strategic-management';plan='strategic-management-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-management-c2b-20260816-v1\manifest.json'},
    [ordered]@{key='supply-chain';plan='supply-chain-20260815-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-supply-chain-c2b-20260815-v5\manifest.json'},
    [ordered]@{key='value-creating-marketing';plan='value-creating-marketing-20260816-v1.json';manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-value-creating-marketing-c2b-20260816-v2\manifest.json'}
)
foreach($row in $rows){$row.plan=Join-Path $planRootResolved ([string]$row.plan)}

$financeLedger=Get-Content -LiteralPath $financeLedgerResolved -Raw -Encoding utf8|ConvertFrom-Json
$financeChapters=@();foreach($group in @($financeLedger.semantic_entries|Group-Object chapter)){
    $name=[string]$group.Name;$financeChapters+=[ordered]@{id=$name.Substring(0,2);note=$name;title=$name.Substring(3);modules=@($group.Group.module_id|ForEach-Object{[string]$_})}
}
$financeDomains=@(
    [ordered]@{id='governance';label='治理与目标';nodes=@([ordered]@{id='target';note='01-财务管理目标与财务高管职责'})},
    [ordered]@{id='assets';label='资产配置与运营';nodes=@([ordered]@{id='working';note='02-营运资本管理'},[ordered]@{id='invest';note='03-项目投资评估'})},
    [ordered]@{id='valuation';label='风险与价值评估';nodes=@([ordered]@{id='risk';note='04-投资风险与不确定性'},[ordered]@{id='value';note='05-企业估值'})},
    [ordered]@{id='funding';label='融资与收益分配';nodes=@([ordered]@{id='capital';note='06-融资决策与资本结构'},[ordered]@{id='dividend';note='07-股利决策'})},
    [ordered]@{id='restructure';label='资本重组与增长';nodes=@([ordered]@{id='mna';note='08-并购与企业重组'})}
)
$financeLive=Join-Path $vaultRootResolved 'mba_finance_c2b_latest'
$financeLegacy=[ordered]@{
    schema='babata.mba-course-c2b-plan/v1';course='25春 MBAO5406 财务管理';short_name='财务管理';course_key='finance-management';expected_modules=37;output_status='accepted'
    chapters=$financeChapters;knowledge_universe=[ordered]@{foundation_id='mapnode_p6_consciousness';discipline_id='mapnode_01KZXVE4ZXFX3EKF82JKNXN4HG';branch_name='财务管理'}
    course_map=[ordered]@{classification_axis='财务决策对象';root_id='finance';root_label='财务管理';tagline='价值创造 · 评估 · 分配 · 维护';domains=$financeDomains;learning=[ordered]@{id='learning';label='学习支持';nodes=@([ordered]@{id='tools';note='09-公式与决策工具'},[ordered]@{id='cases';note='10-案例练习'},[ordered]@{id='review';note='11-复习与自测'},[ordered]@{id='evidence';note='视觉证据索引'})}}
    live=[ordered]@{path=$financeLive;vault='Obsidian Vault';file='Babata/MBA/mba_finance_c2b_latest/index.md'}
}
$rows+=,[ordered]@{key='finance-management';plan=$null;manifest='D:\BabataData\04_runtime\staging\model-workspaces\mba-finance-c2b-benchmark-20260815-v17-responsive-map\manifest.json';legacy=$financeLegacy}

# Validate every immutable input and canonical live before creating rollout state.
foreach($row in $rows){
    if(-not $row.Contains('legacy')){
        if(-not(Test-Path -LiteralPath ([string]$row.plan) -PathType Leaf)){throw "Missing rollout input: $($row.plan)"}
        $legacy=Get-Content -LiteralPath ([string]$row.plan) -Raw -Encoding utf8|ConvertFrom-Json
    }else{$legacy=$row.legacy}
    if([string]$legacy.schema -cne 'babata.mba-course-c2b-plan/v1' -or [string]$legacy.course_key -cne [string]$row.key){throw "Invalid rollout course plan: $($row.key)"}
    if(-not(Test-Path -LiteralPath ([string]$row.manifest) -PathType Leaf)){throw "Missing rollout input: $($row.manifest)"}
    $live=[IO.Path]::GetFullPath([string]$legacy.live.path)
    if(-not(Is-Within $live $vaultRootResolved) -or -not(Test-Path -LiteralPath $live -PathType Container)){throw "Missing or unsafe canonical live: $live"}
    Assert-SourceLiveMatches $live ([string]$row.manifest)
}

[void](New-Item -ItemType Directory -Path $runtime)
$presentationPlans=Join-Path $runtime 'presentation-plans';[void](New-Item -ItemType Directory -Path $presentationPlans)
$sourceSnapshots=Join-Path $runtime 'source-snapshots';[void](New-Item -ItemType Directory -Path $sourceSnapshots)
$migrations=Join-Path $runtime 'migrations';[void](New-Item -ItemType Directory -Path $migrations)
$archives=Join-Path $runtime 'archived-live-v1';[void](New-Item -ItemType Directory -Path $archives)
$failedLives=Join-Path $runtime 'failed-published-live-v2'
$financeLegacyPath=Join-Path $runtime 'finance-compatibility-course-plan-v1.json';Write-Json $financeLegacy $financeLegacyPath
($rows|Where-Object{[string]$_.key -ceq 'finance-management'}|Select-Object -First 1).plan=$financeLegacyPath

$prepared=@();foreach($row in $rows){
    foreach($path in @([string]$row.plan,[string]$row.manifest)){if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Missing rollout input: $path"}}
    $legacy=Get-Content -LiteralPath ([string]$row.plan) -Raw -Encoding utf8|ConvertFrom-Json
    $live=[IO.Path]::GetFullPath([string]$legacy.live.path)
    if(-not(Test-Path -LiteralPath $live -PathType Container)){throw "Missing canonical live: $live"}
    $snapshot=Join-Path $sourceSnapshots ([string]$row.key);Copy-Tree $live $snapshot
    $plan=Join-Path $presentationPlans (([string]$row.key)+'-presentation-v2.json')
    $generated=@(& (Join-Path $PSScriptRoot 'new-mba-course-presentation-plan.ps1') -LegacyPlanPath ([string]$row.plan) -SourceManifestPath ([string]$row.manifest) -SourceRoot $snapshot -OutputPath $plan)
    if($generated.Count -ne 1 -or [string]$generated[0].status -cne 'passed'){throw "Plan generation failed: $($row.key)"}
    $migrationRoot=Join-Path $migrations ([string]$row.key)
    $migrated=@(& (Join-Path $PSScriptRoot 'migrate-mba-course-presentation-v2.ps1') -PresentationPlanPath $plan -SourceRoot $snapshot -StagingRoot $migrationRoot)
    if($migrated.Count -ne 1 -or [string]$migrated[0].status -cne 'passed_engineering_gates'){throw "Presentation migration failed: $($row.key)"}
    $prepared+=[ordered]@{key=[string]$row.key;course=[string]$legacy.course;live=$live;snapshot=$snapshot;plan=$plan;source_compatibility_drift=[int]$generated[0].source_compatibility_drift;migration=$migrationRoot;package=[string]$migrated[0].package;manifest=[string]$migrated[0].manifest;receipt=[string]$migrated[0].receipt;files=[int]$migrated[0].files}
}
if($prepared.Count -ne 13){throw "MBA presentation denominator mismatch: $($prepared.Count)/13"}

$published=@();$obsoletePilot=Join-Path $vaultRootResolved '25春 MBAO5406 财务管理 C2 试点-语义导航版';$obsoleteArchive=$null
try{
    foreach($course in $prepared){
        $archive=Join-Path $archives ([string]$course.key)
        if(Test-Path -LiteralPath $archive){throw "Archive target already exists: $archive"}
        Move-Item -LiteralPath ([string]$course.live) -Destination $archive
        try{
            Copy-Tree ([string]$course.package) ([string]$course.live)
            Assert-LiveMatches ([string]$course.live) ([string]$course.manifest)
        }catch{
            if(Test-Path -LiteralPath ([string]$course.live)){
                if(-not(Test-Path -LiteralPath $failedLives)){[void](New-Item -ItemType Directory -Path $failedLives)}
                Move-Item -LiteralPath ([string]$course.live) -Destination (Join-Path $failedLives ([string]$course.key))
            }
            Move-Item -LiteralPath $archive -Destination ([string]$course.live)
            throw
        }
        $published+=[ordered]@{key=[string]$course.key;course=[string]$course.course;live=[string]$course.live;archived_live=$archive;staged_package=[string]$course.package;plan=[string]$course.plan;source_compatibility_drift=[int]$course.source_compatibility_drift;manifest=[string]$course.manifest;receipt=[string]$course.receipt;files=[int]$course.files;status='published_presentation_v2'}
    }
    if(Test-Path -LiteralPath $obsoletePilot -PathType Container){$obsoleteArchive=Join-Path $runtime 'archived-obsolete-finance-pilot';Move-Item -LiteralPath $obsoletePilot -Destination $obsoleteArchive}
    $rollout=[ordered]@{schema='babata.mba-course-presentation-rollout/v1';status='published';profile='semantic-obsidian/v2';courses=$published;course_count=$published.Count;source_content_regeneration_runs=0;c1b_registration_runs=0;knowledge_registration_runs=0;closure_verifier_runs=0;obsolete_finance_pilot_archive=$obsoleteArchive}
    $rolloutPath=Join-Path $runtime 'rollout-receipt.json';Write-Json $rollout $rolloutPath
}catch{
    if($obsoleteArchive -and (Test-Path -LiteralPath $obsoleteArchive) -and -not(Test-Path -LiteralPath $obsoletePilot)){Move-Item -LiteralPath $obsoleteArchive -Destination $obsoletePilot}
    for($i=$published.Count-1;$i -ge 0;$i--){
        $course=$published[$i]
        if(Test-Path -LiteralPath ([string]$course.live)){
            if(-not(Test-Path -LiteralPath $failedLives)){[void](New-Item -ItemType Directory -Path $failedLives)}
            Move-Item -LiteralPath ([string]$course.live) -Destination (Join-Path $failedLives ([string]$course.key))
        }
        if(Test-Path -LiteralPath ([string]$course.archived_live)){Move-Item -LiteralPath ([string]$course.archived_live) -Destination ([string]$course.live)}
    }
    throw
}
[pscustomobject][ordered]@{schema=$rollout.schema;status='published';courses=$published.Count;profile=$rollout.profile;receipt=$rolloutPath}
