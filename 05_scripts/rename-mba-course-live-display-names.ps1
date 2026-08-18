[CmdletBinding()]
param(
    [string]$SourceRolloutReceipt='D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v3\rollout-receipt.json',
    [string]$RuntimeRoot='D:\BabataData\04_runtime\staging\model-workspaces\mba-course-live-display-names-20260818-v1',
    [string]$VaultRoot='C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA'
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Write-Json([object]$Value,[string]$Path){
    $parent=Split-Path -Parent $Path
    if(-not(Test-Path -LiteralPath $parent)){[void](New-Item -ItemType Directory -Path $parent -Force)}
    [IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 50),[Text.UTF8Encoding]::new($false))
}
function Is-Within([string]$Child,[string]$Parent){
    $childPath=[IO.Path]::GetFullPath($Child).TrimEnd('\')
    $parentPath=[IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $childPath.StartsWith($parentPath+'\',[StringComparison]::OrdinalIgnoreCase)
}
function Relative([string]$Root,[string]$Path){
    $prefix=[IO.Path]::GetFullPath($Root).TrimEnd('\')+'\'
    $full=[IO.Path]::GetFullPath($Path)
    if(-not $full.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw "Path is outside root: $Path"}
    $full.Substring($prefix.Length).Replace('\','/')
}
function Tree-Fingerprint([string]$Root){
    $lines=@(Get-ChildItem -LiteralPath $Root -Recurse -File -Force|Sort-Object FullName|ForEach-Object{
        "$(Relative $Root $_.FullName) $(Hash $_.FullName)"
    })
    $bytes=[Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))
    $sha=[Security.Cryptography.SHA256]::Create()
    try{([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-','').ToLowerInvariant()}finally{$sha.Dispose()}
}
function Assert-LiveMatchesManifest([string]$Live,[object]$Manifest){
    $files=@(Get-ChildItem -LiteralPath $Live -Recurse -File -Force)
    $declared=@($Manifest.package_files)
    if($files.Count -ne $declared.Count){throw "Live file count mismatch: $Live ($($files.Count)/$($declared.Count))"}
    foreach($row in $declared){
        $path=Join-Path $Live ([string]$row.path).Replace('/','\')
        if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "Live file missing: $path"}
        if((Hash $path) -cne [string]$row.sha256){throw "Live file hash mismatch: $path"}
    }
}

$receiptPath=(Get-Item -LiteralPath $SourceRolloutReceipt -ErrorAction Stop).FullName
$sourceReceipt=Get-Content -LiteralPath $receiptPath -Raw -Encoding utf8|ConvertFrom-Json
if([string]$sourceReceipt.status -ne 'published' -and [string]$sourceReceipt.status -ne 'renamed'){
    throw 'Source receipt must be a published rollout or a completed display-name migration'
}
$vault=(Get-Item -LiteralPath $VaultRoot -ErrorAction Stop).FullName
$runtime=[IO.Path]::GetFullPath($RuntimeRoot)
if(Test-Path -LiteralPath $runtime){throw "Fresh naming migration root already exists: $runtime"}
$checker=Join-Path $PSScriptRoot 'check-mba-course-presentation-plan.ps1'
$plans=Join-Path $runtime 'presentation-plans';[void](New-Item -ItemType Directory -Path $plans -Force)

$prepared=@()
foreach($row in @($sourceReceipt.courses)){
    $planInput=if($sourceReceipt.schema -ceq 'babata.mba-course-presentation-rollout/v1'){[string]$row.plan}else{[string]$row.new_plan}
    $manifestInput=if($sourceReceipt.schema -ceq 'babata.mba-course-presentation-rollout/v1'){[string]$row.manifest}else{[string]$row.old_manifest}
    $liveInput=if($sourceReceipt.schema -ceq 'babata.mba-course-presentation-rollout/v1'){[string]$row.live}else{[string]$row.new_live}
    $oldPlanPath=(Get-Item -LiteralPath $planInput -ErrorAction Stop).FullName
    $oldPlan=Get-Content -LiteralPath $oldPlanPath -Raw -Encoding utf8|ConvertFrom-Json
    if([string]$oldPlan.course_key -cne [string]$row.key -or [string]$oldPlan.course -cne [string]$row.course){throw "Rollout row does not bind plan: $($row.key)"}
    $oldManifestPath=(Get-Item -LiteralPath $manifestInput -ErrorAction Stop).FullName
    $oldManifest=Get-Content -LiteralPath $oldManifestPath -Raw -Encoding utf8|ConvertFrom-Json
    $sourceLive=[IO.Path]::GetFullPath($liveInput)
    $targetLive=Join-Path $vault ([string]$oldPlan.short_name)
    if(-not(Is-Within $sourceLive $vault) -or -not(Test-Path -LiteralPath $sourceLive -PathType Container)){throw "Unsafe or missing source live: $sourceLive"}
    if(-not(Is-Within $targetLive $vault)){throw "Unsafe display live target: $targetLive"}
    $sourceLeaf=Split-Path -Leaf $sourceLive
    if($sourceLeaf -notmatch '(?i)(?:_c2b|latest)' -and $sourceLeaf -cne [string]$oldPlan.course){throw "Source live is not a known prior display name: $sourceLive"}
    if(Test-Path -LiteralPath $targetLive){throw "Display live target already exists: $targetLive"}
    Assert-LiveMatchesManifest $sourceLive $oldManifest
    $successor=$oldPlan|ConvertTo-Json -Depth 50|ConvertFrom-Json
    $successor.live.path=$targetLive
    $successor.live.file='Babata/MBA/'+[string]$oldPlan.short_name+'/index.md'
    $newPlanPath=Join-Path $plans (([string]$row.key)+'-presentation-v2.json')
    Write-Json $successor $newPlanPath
    $check=@(& $checker -PlanPath $newPlanPath)
    if($check.Count -ne 1 -or [string]$check[0].status -cne 'passed'){throw "Successor presentation plan failed checker: $($row.key)"}
    $prepared+=[ordered]@{
        key=[string]$row.key;course=[string]$row.course;old_live=$sourceLive;new_live=$targetLive
        old_plan=$oldPlanPath;old_plan_sha256=Hash $oldPlanPath;new_plan=$newPlanPath;new_plan_sha256=Hash $newPlanPath
        old_manifest=$oldManifestPath;old_manifest_sha256=Hash $oldManifestPath;files=@($oldManifest.package_files).Count
        tree_sha256=Tree-Fingerprint $sourceLive;status='prepared'
    }
}
if($prepared.Count -ne 13){throw "MBA naming denominator mismatch: $($prepared.Count)/13"}

$moved=@()
try{
    foreach($course in $prepared){
        Move-Item -LiteralPath ([string]$course.old_live) -Destination ([string]$course.new_live)
        $course.status='renamed'
        Assert-LiveMatchesManifest ([string]$course.new_live) (Get-Content -LiteralPath ([string]$course.old_manifest) -Raw -Encoding utf8|ConvertFrom-Json)
        if((Tree-Fingerprint ([string]$course.new_live)) -cne [string]$course.tree_sha256){throw "Display live tree changed during rename: $($course.key)"}
        $moved+=$course
    }
}catch{
    for($i=$moved.Count-1;$i -ge 0;$i--){
        $course=$moved[$i]
        if(Test-Path -LiteralPath ([string]$course.new_live) -PathType Container -and -not(Test-Path -LiteralPath ([string]$course.old_live))){Move-Item -LiteralPath ([string]$course.new_live) -Destination ([string]$course.old_live)}
        $course.status='rolled_back'
    }
    throw
}

$migrationReceipt=[ordered]@{
    schema='babata.mba-course-live-display-name-migration/v1';status='renamed';profile='semantic-obsidian/v2'
    source_receipt=$receiptPath;source_receipt_sha256=Hash $receiptPath
    vault=$vault;course_count=$prepared.Count;renamed_count=@($prepared|Where-Object{$_.status -ceq 'renamed'}).Count
    implementation_fields_removed=@('c2b','latest','25春','MBAO','OMBA','course_number');content_files_changed=0;source_content_regeneration_runs=0;c1b_registration_runs=0;knowledge_registration_runs=0;closure_verifier_runs=0
    courses=$prepared
}
$receiptOut=Join-Path $runtime 'display-name-migration-receipt.json';Write-Json $migrationReceipt $receiptOut
[pscustomobject][ordered]@{schema=$migrationReceipt.schema;status=$migrationReceipt.status;courses=$migrationReceipt.course_count;renamed=$migrationReceipt.renamed_count;receipt=$receiptOut}
