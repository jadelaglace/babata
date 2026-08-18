[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PlanPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Require-Text([object]$Value,[string]$Label) {
    $text=[string]$Value
    if([string]::IsNullOrWhiteSpace($text)){throw "$Label is required"}
    $text
}

function Require-SafeNote([object]$Value,[string]$Label) {
    $note=Require-Text $Value $Label
    if($note.EndsWith('.md',[StringComparison]::OrdinalIgnoreCase) -or $note -match '[\\/:*?"<>|\[\]]'){
        throw "$Label must be a safe extensionless note basename: $note"
    }
    $note
}

function Require-DisplayLive([object]$Live,[string]$DisplayName) {
    if($null -eq $Live){throw 'live is required'}
    $path=Require-Text $Live.path 'live.path'
    if(-not [IO.Path]::IsPathRooted($path)){throw 'live.path must be absolute'}
    $leaf=(Split-Path -Leaf $path).Trim()
    if($leaf -cne $DisplayName){throw "live directory must use the short course display name: expected '$DisplayName', got '$leaf'"}
    $vault=Require-Text $Live.vault 'live.vault'
    $file=Require-Text $Live.file 'live.file'
    $expectedFile='Babata/MBA/'+$DisplayName+'/index.md'
    if($file -cne $expectedFile){throw "live.file must use the short course display name: expected '$expectedFile', got '$file'"}
    [pscustomobject][ordered]@{path=$path;vault=$vault;file=$file}
}

function Get-Units($Outline) {
    $mode=[string]$Outline.mode
    if($mode -ceq 'flat'){
        if($null -eq $Outline.PSObject.Properties['units'] -or $null -ne $Outline.PSObject.Properties['sections']){
            throw 'flat outline must declare units and must not declare sections'
        }
        return @($Outline.units)
    }
    if($mode -ceq 'sectioned'){
        if($null -eq $Outline.PSObject.Properties['sections'] -or $null -ne $Outline.PSObject.Properties['units']){
            throw 'sectioned outline must declare sections and must not declare top-level units'
        }
        $units=@();$sectionIds=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach($section in @($Outline.sections)){
            $sectionId=Require-Text $section.id 'outline section id'
            Require-Text $section.title 'outline section title'|Out-Null
            if(-not $sectionIds.Add($sectionId)){throw "Duplicate outline section id: $sectionId"}
            $sectionUnits=@($section.units)
            if(-not $sectionUnits.Count){throw "Outline section has no units: $sectionId"}
            $units+=$sectionUnits
        }
        if(-not $sectionIds.Count){throw 'sectioned outline must contain at least one section'}
        return @($units)
    }
    throw "Unsupported outline mode: $mode"
}

$resolved=(Get-Item -LiteralPath $PlanPath -ErrorAction Stop).FullName
$plan=Get-Content -LiteralPath $resolved -Raw -Encoding utf8|ConvertFrom-Json
if([string]$plan.schema -cne 'babata.mba-course-presentation-plan/v2'){throw 'Unsupported MBA presentation-plan schema'}
Require-Text $plan.course 'course'|Out-Null
$courseName=[string]$plan.course
$displayName=Require-SafeNote $plan.short_name 'short_name'
$courseKey=Require-Text $plan.course_key 'course_key'
if($courseKey -cnotmatch '^[a-z0-9]+(?:-[a-z0-9]+)*$'){throw "Invalid course_key: $courseKey"}
$expected=[int]$plan.expected_modules
if($expected -lt 1){throw 'expected_modules must be positive'}
if([string]$plan.profile -cne 'semantic-obsidian/v2'){throw 'Presentation plan must use semantic-obsidian/v2'}
if([string]$plan.output_status -notin @('pending_user_acceptance','accepted')){throw 'Invalid presentation output_status'}
Require-DisplayLive $plan.live $displayName|Out-Null

foreach($field in @('plan_path','plan_sha256','manifest_path','manifest_sha256')){
    $value=Require-Text $plan.source.$field "source.$field"
    if($field.EndsWith('_path') -and -not [IO.Path]::IsPathRooted($value)){throw "source.$field must be absolute"}
    if($field.EndsWith('_sha256') -and $value -cnotmatch '^[0-9a-f]{64}$'){throw "source.$field must be lowercase SHA-256"}
}
foreach($field in @('predecessor_manifest_path','predecessor_manifest_sha256')){
    if($null -ne $plan.source.PSObject.Properties[$field]){
        $value=Require-Text $plan.source.$field "source.$field"
        if($field.EndsWith('_path') -and -not [IO.Path]::IsPathRooted($value)){throw "source.$field must be absolute"}
        if($field.EndsWith('_sha256') -and $value -cnotmatch '^[0-9a-f]{64}$'){throw "source.$field must be lowercase SHA-256"}
    }
}
if(($null -eq $plan.source.PSObject.Properties['predecessor_manifest_path']) -ne ($null -eq $plan.source.PSObject.Properties['predecessor_manifest_sha256'])){throw 'source predecessor manifest path/hash must appear together'}

$units=@(Get-Units $plan.outline)
if(-not $units.Count){throw 'outline must contain at least one unit'}
$unitIds=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$notes=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$modules=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($unit in $units){
    $unitId=Require-Text $unit.id 'unit id';$note=Require-SafeNote $unit.note 'unit note'
    Require-Text $unit.title 'unit title'|Out-Null
    if(-not $unitIds.Add($unitId)){throw "Duplicate unit id: $unitId"}
    if(-not $notes.Add($note)){throw "Duplicate presentation note: $note"}
    $bound=@($unit.source_modules|ForEach-Object{[string]$_})
    if(-not $bound.Count){throw "Unit has no source_modules: $unitId"}
    foreach($module in $bound){
        if([string]::IsNullOrWhiteSpace($module) -or -not $modules.Add($module)){throw "Duplicate or empty source module: $module"}
    }
}
if($modules.Count -ne $expected){throw "Presentation-plan module denominator mismatch: $($modules.Count)/$expected"}

$supports=@($plan.learning_support)
$slots=@('decision_tools','case_practice','review_self_test','evidence_index')
if($supports.Count -ne $slots.Count){throw 'learning_support must contain exactly four semantic slots'}
$expectedRename=[ordered]@{}
for($i=0;$i -lt $slots.Count;$i++){
    $support=$supports[$i];$slot=[string]$support.slot;$note=Require-SafeNote $support.note "learning_support[$i].note"
    Require-Text $support.id "learning_support[$i].id"|Out-Null
    if($slot -cne $slots[$i]){throw "learning_support slot order mismatch: expected $($slots[$i]), got $slot"}
    if($note -match '^(09|10|11)-'){throw "Numbered learning-support note is forbidden in v2: $note"}
    if($slot -eq 'evidence_index'){
        if($note -cne '视觉证据索引'){throw 'evidence_index must use 视觉证据索引'}
    } elseif(-not $note.StartsWith('学习支持-',[StringComparison]::Ordinal)){
        throw "Learning-support note must use 学习支持- prefix: $note"
    }
    if(-not $notes.Add($note)){throw "Duplicate presentation note: $note"}
    $legacy=Require-SafeNote $support.legacy_note "learning_support[$i].legacy_note"
    if($legacy -cne $note){$expectedRename[$legacy]=$note}
}

$actualRename=[ordered]@{}
foreach($property in @($plan.rename_map.PSObject.Properties)){
    $old=Require-SafeNote $property.Name 'rename_map source';$new=Require-SafeNote $property.Value "rename_map.$old"
    if($actualRename.Contains($old)){throw "Duplicate rename_map source: $old"}
    $actualRename[$old]=$new
}
if(($actualRename|ConvertTo-Json -Compress) -cne ($expectedRename|ConvertTo-Json -Compress)){
    throw 'rename_map must exactly equal changed learning_support legacy_note -> note pairs'
}

if($null -eq $plan.course_map -or @($plan.course_map.domains).Count -lt 1){throw 'course_map is required'}
$mapNotes=@($plan.course_map.domains.nodes.note|ForEach-Object{[string]$_})
$sortedMapNotes=(@($mapNotes|Sort-Object) -join "`n")
$sortedUnitNotes=(@($units.note|ForEach-Object{[string]$_}|Sort-Object) -join "`n")
if($sortedMapNotes -cne $sortedUnitNotes){
    throw 'course_map domains must partition outline units exactly'
}
$mapSupport=@($plan.course_map.learning.nodes.note|ForEach-Object{[string]$_})
if(($mapSupport -join "`n") -cne (@($supports.note|ForEach-Object{[string]$_}) -join "`n")){
    throw 'course_map learning nodes must equal ordered learning_support notes'
}

[pscustomobject][ordered]@{
    schema='babata.mba-course-presentation-plan-check/v1'
    status='passed'
    course=[string]$plan.course
    course_key=$courseKey
    outline_mode=[string]$plan.outline.mode
    units=$units.Count
    source_modules=$modules.Count
    learning_support=$supports.Count
    plan=$resolved
}
