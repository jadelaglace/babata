[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-knowledge-universe-ontology-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-ontology-governance-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\01_prd\01_a_PRD.md',
    '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md',
    '00_docs\03_architecture\03_a_ARCHITECTURE.md',
    '00_docs\04_process\04_b_USAGE_STATUS.md',
    '00_docs\05_tests\05_a_TEST_CASES.md',
    '02_skills\00_specs\09_outputs.md',
    '02_skills\00_specs\templates\semantic-obsidian-profile.md',
    '05_scripts\register-mba-course-c2b-knowledge.ps1'
)

function New-TestRepo {
    param([Parameter(Mandatory)][string]$Name)
    $caseRoot = Join-Path $root $Name
    foreach ($relativePath in $requiredFiles) {
        $source = Join-Path $repo $relativePath
        $target = Join-Path $caseRoot $relativePath
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $target
    }
    return $caseRoot
}

function Replace-Once {
    param([string]$CaseRoot, [string]$RelativePath, [string]$Value, [string]$Replacement)
    $path = Join-Path $CaseRoot $RelativePath
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $first = $text.IndexOf($Value, [StringComparison]::Ordinal)
    if ($first -lt 0) { throw "Mutation fixture is missing ${RelativePath}: $Value" }
    $text = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Value.Length)
    [IO.File]::WriteAllText($path, $text, [Text.UTF8Encoding]::new($false))
}

function Assert-CheckerFails {
    param([string]$Name, [string]$Expected, [scriptblock]$Mutate)
    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    try {
        & $checker -RepoRoot $caseRoot | Out-Null
        throw "$Name unexpectedly passed"
    }
    catch {
        if ($_.Exception.Message -eq "$Name unexpectedly passed") { throw }
        if (-not $_.Exception.Message.Contains($Expected)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
}

try {
    New-Item -ItemType Directory -Path $root | Out-Null
    & $checker -RepoRoot $repo | Out-Null

    Assert-CheckerFails 'requirements-force-single-foundation' 'overlapping foundation requirement' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md' '稳定但可重叠的世界观基石，不是四选一内容桶' '互斥的四选一内容桶'
    }
    Assert-CheckerFails 'architecture-merges-course-branch' 'typed Course/Branch architecture' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\03_architecture\03_a_ARCHITECTURE.md' 'Course 与 Branch 身份分离' 'Course 与 Branch 共用身份'
    }
    Assert-CheckerFails 'architecture-lets-model-own-review' 'machine/human review boundary' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\03_architecture\03_a_ARCHITECTURE.md' '机器建议与人工审阅分别存储' '机器建议直接替代人工审阅'
    }
    Assert-CheckerFails 'profile-promotes-display-domain' 'display-domain/Branch separation' {
        param($caseRoot)
        Replace-Once $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md' 'These display domains are not ontology `Branch`' 'These display domains create ontology `Branch`'
    }
    Assert-CheckerFails 'outputs-reenable-legacy-registration' 'historical registrar quarantine' {
        param($caseRoot)
        Replace-Once $caseRoot '02_skills\00_specs\09_outputs.md' 'registration is not conformant for new formal registrations' 'registration is conformant for new formal registrations'
    }
    Assert-CheckerFails 'legacy-registrar-loses-unassign' 'historical unassign behavior' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\register-mba-course-c2b-knowledge.ps1' "'--change','unassign'" "'--change','retain'"
    }

    Write-Output 'Knowledge-universe governance mutation tests passed.'
}
finally {
    if (Test-Path -LiteralPath $root) {
        $resolved = (Resolve-Path -LiteralPath $root).Path
        $systemTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolved.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected test path: $resolved"
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
