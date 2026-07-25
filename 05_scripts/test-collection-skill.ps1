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

    Assert-CheckerFails -Name 'doubao-falsely-enabled' -ExpectedMessage 'disabled Doubao route' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'references\route-catalog.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text -replace '(?m)^(\| `source\.doubao` \|.*\| )disabled( \|.*)$', '${1}enabled${2}'
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'missing-no-c1-boundary' -ExpectedMessage 'no-C1 execution boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot 'SKILL.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('Never call `babata process`', 'Do not usually call processing')
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
