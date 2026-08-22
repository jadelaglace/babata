$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-doc-provenance.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ('babata-doc-prov-' + [guid]::NewGuid().ToString('N'))

function New-Case {
    param([string]$Name)
    $case = Join-Path $tempRoot $Name
    Copy-Item -LiteralPath (Join-Path $repo '00_docs') -Destination $case -Recurse
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
    try {
        & $checker -DocsRoot $case | Out-Null
        throw "$Name did not fail"
    }
    catch {
        if ($_.Exception.Message -eq "$Name did not fail") { throw }
        if ($_.Exception.Message -notlike "*$Expected*") {
            throw "$Name failed for the wrong reason: $($_.Exception.Message)"
        }
    }
}

try {
    & $checker -DocsRoot (Join-Path $repo '00_docs') | Out-Null

    Assert-Fails 'missing-selected-quote' 'selected verbatim evidence' {
        param($case)
        $path = Join-Path $case '90_archive\2026-08-23_P0-P9_CLOSEOUT.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('> 第三大界：我的认知轨迹（日志与感悟层）', '> rewritten summary')
        Write-Text $path $text
    }
    Assert-Fails 'requirements-fake-quote' 'must not present curated interpretation' {
        param($case)
        $path = Join-Path $case '00_requirements\00_a_REQUIREMENTS.md'
        Add-Content -LiteralPath $path -Value "`n> Agent interpretation presented as user wording" -Encoding utf8
    }
    Assert-Fails 'missing-freeze-recovery' 'archive provenance boundary' {
        param($case)
        $path = Join-Path $case '90_archive\2026-08-23_P0-P9_CLOSEOUT.md'
        $text = (Get-Content -Raw -Encoding utf8 $path).Replace('git show 9182a77:<旧相对路径>', 'history unavailable')
        Write-Text $path $text
    }
    Assert-Fails 'run-id-in-requirements' 'concrete runtime identity' {
        param($case)
        Add-Content -LiteralPath (Join-Path $case '00_requirements\00_a_REQUIREMENTS.md') -Value "`nD:\BabataData\round-20260823\mba-test-20260823-v1" -Encoding utf8
    }
    Assert-Fails 'secret-in-archive' 'Sensitive token pattern' {
        param($case)
        Add-Content -LiteralPath (Join-Path $case '90_archive\2026-08-23_P0-P9_CLOSEOUT.md') -Value "`nghp_123456789012345678901234567890" -Encoding utf8
    }

    Write-Output 'Document provenance mutation tests passed.'
}
finally {
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}
