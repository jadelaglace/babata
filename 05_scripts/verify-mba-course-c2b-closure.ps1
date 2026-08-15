[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$LearningDocsManifestPath,
    [Parameter(Mandatory=$true)][string]$C1BRegistrationLedgerPath,
    [Parameter(Mandatory=$true)][string]$KnowledgeUniverseLedgerPath,
    [Parameter(Mandatory=$true)][string]$C2BStagingRoot,
    [Parameter(Mandatory=$true)][string]$LiveVaultPath,
    [string]$DataHome=$env:BABATA_DATA_HOME,
    [string]$PackageCheckerScript=(Join-Path $PSScriptRoot 'check-mba-course-c2b-package.ps1'),
    [string]$OutputPath,
    [Parameter(Mandatory=$true)][string]$UserAcceptanceEvidence
)

$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest

function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Read-Json([string]$Path){Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json -Depth 100}
function Require-Text([object]$Value,[string]$Label){$text=[string]$Value;if([string]::IsNullOrWhiteSpace($text)){throw "$Label is required"};$text}
function Relative([string]$Root,[string]$Path){$Path.Substring($Root.Length).TrimStart('\').Replace('\','/')}
function Assert-Equal($Actual,$Expected,[string]$Label){if($Actual -ne $Expected){throw "$Label mismatch: expected $Expected, got $Actual"}}

if([string]::IsNullOrWhiteSpace($DataHome)){throw 'BABATA_DATA_HOME or -DataHome is required'}
if([string]::IsNullOrWhiteSpace($UserAcceptanceEvidence)){throw 'Explicit user acceptance evidence is required'}
$data=(Get-Item -LiteralPath $DataHome -ErrorAction Stop).FullName.TrimEnd('\')
$plan=(Get-Item -LiteralPath $CoursePlanPath -ErrorAction Stop).FullName
$learning=(Get-Item -LiteralPath $LearningDocsManifestPath -ErrorAction Stop).FullName
$c1bPath=(Get-Item -LiteralPath $C1BRegistrationLedgerPath -ErrorAction Stop).FullName
$knowledgePath=(Get-Item -LiteralPath $KnowledgeUniverseLedgerPath -ErrorAction Stop).FullName
$staging=(Get-Item -LiteralPath $C2BStagingRoot -ErrorAction Stop).FullName.TrimEnd('\')
$package=Join-Path $staging 'package'
$manifestPath=Join-Path $staging 'manifest.json'
$live=(Get-Item -LiteralPath $LiveVaultPath -ErrorAction Stop).FullName.TrimEnd('\')
foreach($path in @($plan,$learning,$c1bPath,$knowledgePath,$package,$manifestPath,$live)){if(-not(Test-Path -LiteralPath $path)){throw "closure input missing: $path"}}
if([string]::IsNullOrWhiteSpace($OutputPath)){$OutputPath=Join-Path $staging 'closure-verification.json'}

$planObject=Read-Json $plan
$learningObject=Read-Json $learning
$c1b=Read-Json $c1bPath
$knowledge=Read-Json $knowledgePath
$manifest=Read-Json $manifestPath
$course=Require-Text $planObject.course 'course'
$courseKey=Require-Text $planObject.course_key 'course_key'
if([string]$planObject.schema -cne 'babata.mba-course-c2b-plan/v1' -or [string]$planObject.output_status -cne 'pending_user_acceptance'){throw 'Course plan must remain a pending-user-acceptance snapshot'}
if([string]$manifest.status -cne 'pending_user_acceptance'){throw 'C2B package snapshot must remain pending_user_acceptance'}
Assert-Equal ([string]$manifest.course) $course 'manifest course'
Assert-Equal ([string]$manifest.course_key) $courseKey 'manifest course_key'
Assert-Equal ([string]$manifest.course_plan_sha256) (Hash $plan) 'manifest course-plan hash'
Assert-Equal ([string]$learningObject.course_plan_sha256) (Hash $plan) 'learning course-plan hash'
Assert-Equal ([string]$c1b.course_plan_sha256) (Hash $plan) 'C1B course-plan hash'
Assert-Equal ([string]$knowledge.plan_sha256) (Hash $plan) 'knowledge course-plan hash'
Assert-Equal ([string]$manifest.formal_registration) 'registered' 'formal registration'
Assert-Equal ([string]$manifest.c1b_registration.status) 'registered' 'C1B registration'
Assert-Equal ([string]$manifest.knowledge_universe.status) 'registered' 'knowledge-universe registration'

$checker=(Get-Item -LiteralPath $PackageCheckerScript -ErrorAction Stop).FullName
& $checker -CoursePlanPath $plan -PackageRoot $package -ManifestPath $manifestPath | Out-Null

$packageFiles=@(Get-ChildItem -LiteralPath $package -Recurse -File)
$liveFiles=@(Get-ChildItem -LiteralPath $live -Recurse -File)
$packageByRelative=@{};foreach($file in $packageFiles){$packageByRelative[(Relative $package $file.FullName)]=$file}
$liveByRelative=@{};foreach($file in $liveFiles){$liveByRelative[(Relative $live $file.FullName)]=$file}
$hashFailures=[Collections.Generic.List[string]]::new()
foreach($relative in $packageByRelative.Keys){
    if(-not $liveByRelative.ContainsKey($relative)){[void]$hashFailures.Add("missing:$relative");continue}
    if((Hash $packageByRelative[$relative].FullName) -cne (Hash $liveByRelative[$relative].FullName)){[void]$hashFailures.Add($relative)}
}
foreach($relative in $liveByRelative.Keys){if(-not $packageByRelative.ContainsKey($relative)){[void]$hashFailures.Add("extra:$relative")}}
Assert-Equal $hashFailures.Count 0 'package/live hash failures'

$rawDb=Join-Path $data '01_raw\index\raw.sqlite'
$derivedDb=Join-Path $data '02_derived\index\derived.sqlite'
foreach($db in @($rawDb,$derivedDb)){if(-not(Test-Path -LiteralPath $db)){throw "closure database missing: $db"}}
$rawQuick=(& sqlite3 $rawDb 'PRAGMA quick_check;').Trim()
$derivedQuick=(& sqlite3 $derivedDb 'PRAGMA quick_check;').Trim()
Assert-Equal $rawQuick 'ok' 'raw database quick_check'
Assert-Equal $derivedQuick 'ok' 'derived database quick_check'
$rawFk=@(& sqlite3 $rawDb 'PRAGMA foreign_key_check;') | Where-Object {$_}
$derivedFk=@(& sqlite3 $derivedDb 'PRAGMA foreign_key_check;') | Where-Object {$_}
Assert-Equal @($rawFk).Count 0 'raw database foreign keys'
Assert-Equal @($derivedFk).Count 0 'derived database foreign keys'

$verifiedAt=(Get-Date).ToUniversalTime().ToString('o')
$receipt=[ordered]@{
    schema='babata.mba-course-c2b-formal-closure/v1'
    status='passed'
    course_acceptance='accepted'
    closure='closed'
    verified_at=$verifiedAt
    user_acceptance=[ordered]@{evidence=$UserAcceptanceEvidence;recorded_at=$verifiedAt}
    course=[ordered]@{name=$course;course_key=$courseKey;expected_modules=[int]$planObject.expected_modules}
    c1b=[ordered]@{complete_c1=[int]$c1b.coverage.complete_c1_reused;essence_decisions=[int]$c1b.coverage.essence_decisions_registered;retained_media=[int]$c1b.coverage.retained_media_registered;ledger_sha256=Hash $c1bPath}
    knowledge_universe=[ordered]@{entries=@($knowledge.modules).Count;branch=[string]$knowledge.branch.name;ledger_sha256=Hash $knowledgePath}
    package=[ordered]@{status=[string]$manifest.status;staging=$staging;manifest_sha256=Hash $manifestPath;files=$packageFiles.Count}
    live=[ordered]@{path=$live;files=$liveFiles.Count;hash_differences=0}
    databases=[ordered]@{raw_quick_check=$rawQuick;derived_quick_check=$derivedQuick;foreign_key_failures=0}
}
$parent=Split-Path -Parent $OutputPath;if(-not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent -Force|Out-Null}
$receipt|ConvertTo-Json -Depth 15|Set-Content -LiteralPath $OutputPath -Encoding utf8
Write-Output "closure=$OutputPath status=passed course_acceptance=accepted closure=closed package_live=$($packageFiles.Count)/$($liveFiles.Count) hash_differences=0"
