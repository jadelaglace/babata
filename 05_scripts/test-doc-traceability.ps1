$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-doc-traceability.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ('babata-doc-trace-' + [guid]::NewGuid().ToString('N'))

function New-Case {
    param([string]$Name)
    $case = Join-Path $tempRoot $Name
    New-Item -ItemType Directory -Path $case -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo '00_docs') -Destination (Join-Path $case '00_docs') -Recurse
    Copy-Item -LiteralPath (Join-Path $repo 'README.md') -Destination (Join-Path $case 'README.md')
    return $case
}

function Write-Text {
    param([string]$Path, [string]$Text)
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}

function Assert-Fails {
    param([string]$Name, [string]$Expected, [scriptblock]$Mutate)
    $case = New-Case $Name
    & $Mutate $case
    $failed = $false
    try {
        & $checker -DocsRoot (Join-Path $case '00_docs') -RepoReadmePath (Join-Path $case 'README.md') | Out-Null
    }
    catch {
        $failed = $true
        if ($_.Exception.Message -notlike "*$Expected*") {
            throw "$Name failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failed) { throw "$Name did not fail" }
}

try {
    & $checker -DocsRoot (Join-Path $repo '00_docs') -RepoReadmePath (Join-Path $repo 'README.md') | Out-Null

    Assert-Fails 'missing-requirements' 'Missing authority document' {
        param($case)
        Remove-Item -LiteralPath (Join-Path $case '00_docs\00_requirements\00_a_REQUIREMENTS.md') -Force
    }
    Assert-Fails 'duplicate-doc-id' 'Duplicate stable document ID' {
        param($case)
        $path = Join-Path $case '00_docs\01_prd\01_a_PRD.md'
        Add-Content -LiteralPath $path -Value "`n<!-- DOC-ID: DOC-REQ -->" -Encoding utf8
    }
    Assert-Fails 'wrong-active-plan-path' 'registry path for DOC-ACTIVE-PLAN' {
        param($case)
        $path = Join-Path $case '00_docs\README.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('04_process/04_c_ACTIVE_PLAN.md', '04_process/04_f_ACTIVE_PLAN.md')
        Write-Text $path $text
    }
    Assert-Fails 'missing-prd-marker' 'PRD behavior marker' {
        param($case)
        $path = Join-Path $case '00_docs\01_prd\01_a_PRD.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('PRD-10', 'PRD-X')
        Write-Text $path $text
    }
    Assert-Fails 'missing-backs-status' 'current requirement boundary' {
        param($case)
        $path = Join-Path $case '00_docs\00_requirements\00_a_REQUIREMENTS.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('adopted requirement / not implemented', 'implemented')
        Write-Text $path $text
    }
    Assert-Fails 'missing-archive-freeze' 'closeout archive evidence' {
        param($case)
        $path = Join-Path $case '00_docs\90_archive\2026-08-23_P0-P9_CLOSEOUT.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('9182a77', 'missing-freeze')
        Write-Text $path $text
    }
    Assert-Fails 'missing-youtube-route' 'source.youtube' {
        param($case)
        $path = Join-Path $case '00_docs\03_architecture\03_b_SOURCE_ROUTE_REGISTRY.md'
        $lines = @(Get-Content -Encoding utf8 $path | Where-Object { $_ -notmatch '^\| source\.youtube \|' })
        Write-Text $path ($lines -join "`n")
    }

    Write-Output 'Document traceability mutation tests passed.'
}
finally {
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}
