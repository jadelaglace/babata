$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$sourceSkill = Join-Path $repo '02_skills\babata-collect'
$checker = Join-Path $PSScriptRoot 'check-collection-skill.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("babata-collection-skill-{0}" -f [guid]::NewGuid())

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)]
        [string]$Name,
        [Parameter(Mandatory)]
        [scriptblock]$Mutate,
        [Parameter(Mandatory)]
        [string]$ExpectedMessage
    )

    $caseRoot = Join-Path $tempRoot $Name
    Copy-Item -LiteralPath $sourceSkill -Destination $caseRoot -Recurse
    & $Mutate $caseRoot
    $failedAsExpected = $false
    try {
        & $checker -SkillRoot $caseRoot | Out-Null
    }
    catch {
        $failedAsExpected = $true
        if (-not $_.Exception.Message.Contains($ExpectedMessage)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failedAsExpected) {
        throw "Case '$Name' unexpectedly passed collection Skill validation"
    }
}

try {
    New-Item -ItemType Directory -Path $tempRoot | Out-Null
    & $checker -SkillRoot $sourceSkill | Out-Null

    Assert-CheckerFails -Name 'missing-runtime-preflight' -ExpectedMessage 'runtime capability truth' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'references\route-catalog.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('babata --json capabilities list', 'babata capabilities')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'catalog-duplicates-runtime-status' -ExpectedMessage 'duplicates runtime status' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'references\route-catalog.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`n| source | status |`n| --- | --- |`n| doubao | enabled |`n"
    }

    Assert-CheckerFails -Name 'doubao-allows-incomplete-pagination' -ExpectedMessage 'pagination completion condition' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'references\source-doubao.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('`has_more=false`', '`pagination flag`')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'doubao-loses-item-isolation' -ExpectedMessage 'per-item recollection boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'references\source-doubao.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('recollect --item <item_id>', 'recollect-session --session <session_id>')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'missing-no-c1-boundary' -ExpectedMessage 'no-C1 execution boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'SKILL.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('Never call `babata process`', 'Do not usually call processing')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'ui-description-too-long' -ExpectedMessage 'short_description must contain 25-64 characters' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'agents\openai.yaml'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = [regex]::Replace($text, '(?m)^\s*short_description:.*$', '  short_description: "This collection skill description is deliberately much longer than sixty four characters"')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'executable-process-command' -ExpectedMessage 'executable Process/Knowledge command' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'SKILL.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`nbabata process list-pipelines"
    }

    Write-Output 'Collection Skill mutation tests passed.'
}
finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
