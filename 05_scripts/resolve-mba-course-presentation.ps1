Set-StrictMode -Version Latest

function Resolve-MbaCoursePresentation {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)][object]$CoursePlan,
        [Parameter(Mandatory=$true)][string]$CoursePlanPath,
        [string]$PresentationPlanPath,
        [Parameter(Mandatory=$true)][string]$PresentationCheckerPath
    )

    if([string]$CoursePlan.schema -cne 'babata.mba-course-c2b-plan/v1'){throw 'Unsupported course plan schema'}
    if([string]::IsNullOrWhiteSpace($PresentationPlanPath)){
        return [pscustomobject][ordered]@{
            profile='semantic-obsidian/v1'
            plan_path=$null
            plan_sha256=$null
            outline_mode='flat'
            sections=@()
            chapters=@($CoursePlan.chapters)
            course_map=$CoursePlan.course_map
            learning_notes=@($CoursePlan.course_map.learning.nodes.note|ForEach-Object{[string]$_})
            live=$CoursePlan.live
        }
    }

    $presentationPath=(Get-Item -LiteralPath $PresentationPlanPath -ErrorAction Stop).FullName
    $check=@(& $PresentationCheckerPath -PlanPath $presentationPath)
    if($check.Count -ne 1 -or [string]$check[0].status -cne 'passed'){throw 'Presentation plan did not pass its checker'}
    $presentation=Get-Content -LiteralPath $presentationPath -Raw -Encoding utf8|ConvertFrom-Json
    $legacyPath=(Get-Item -LiteralPath $CoursePlanPath -ErrorAction Stop).FullName
    $legacyHash=(Get-FileHash -LiteralPath $legacyPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $boundPath=[IO.Path]::GetFullPath([string]$presentation.source.plan_path)
    if(-not $boundPath.Equals($legacyPath,[StringComparison]::OrdinalIgnoreCase) -or [string]$presentation.source.plan_sha256 -cne $legacyHash){
        throw 'Presentation plan does not bind this immutable course-content plan'
    }
    if([string]$presentation.course -cne [string]$CoursePlan.course -or
       [string]$presentation.course_key -cne [string]$CoursePlan.course_key -or
       [int]$presentation.expected_modules -ne [int]$CoursePlan.expected_modules -or
       [string]$presentation.output_status -cne [string]$CoursePlan.output_status){
        throw 'Presentation plan identity does not match the course-content plan'
    }

    $units=@();$sections=@()
    if([string]$presentation.outline.mode -ceq 'flat'){$units=@($presentation.outline.units)}
    else{
        foreach($section in @($presentation.outline.sections)){
            $sectionUnits=@($section.units);$units+=$sectionUnits
            $sections+=[pscustomobject][ordered]@{id=[string]$section.id;title=[string]$section.title;notes=@($sectionUnits.note|ForEach-Object{[string]$_})}
        }
    }
    $chapters=@($units|ForEach-Object{[pscustomobject][ordered]@{
        id=[string]$_.id
        note=[string]$_.note
        title=[string]$_.title
        modules=@($_.source_modules|ForEach-Object{[string]$_})
    }})
    [pscustomobject][ordered]@{
        profile='semantic-obsidian/v2'
        plan_path=$presentationPath
        plan_sha256=(Get-FileHash -LiteralPath $presentationPath -Algorithm SHA256).Hash.ToLowerInvariant()
        outline_mode=[string]$presentation.outline.mode
        sections=$sections
        chapters=$chapters
        course_map=$presentation.course_map
        learning_notes=@($presentation.learning_support.note|ForEach-Object{[string]$_})
        live=$presentation.live
    }
}
