$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$docs = Join-Path $repo '00_docs'
$checker = Join-Path $PSScriptRoot 'check-doc-traceability.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("babata-doc-traceability-{0}" -f [guid]::NewGuid())

function Set-SourceFieldEmpty {
    param(
        [Parameter(Mandatory)]
        [string]$ResearchPath,
        [Parameter(Mandatory)]
        [string]$SourceId,
        [Parameter(Mandatory)]
        [int]$CellIndex
    )

    $lines = @(Get-Content -Encoding utf8 -LiteralPath $ResearchPath)
    $matchingIndexes = @()
    for ($index = 0; $index -lt $lines.Count; $index++) {
        if ($lines[$index] -match "^\|\s*$([regex]::Escape($SourceId))\s*\|") {
            $matchingIndexes += $index
        }
    }
    if ($matchingIndexes.Count -ne 1) {
        throw "Mutation setup expected one $SourceId row, found $($matchingIndexes.Count)"
    }

    $cells = @($lines[$matchingIndexes[0]].Split('|'))
    if ($cells.Count -ne 9) {
        throw "Mutation setup expected seven source fields, found $($cells.Count - 2)"
    }
    $cells[$CellIndex] = ' '
    $lines[$matchingIndexes[0]] = $cells -join '|'
    Set-Content -Encoding utf8 -LiteralPath $ResearchPath -Value $lines
}

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)]
        [string]$Name,
        [Parameter(Mandatory)]
        [scriptblock]$Mutate,
        [Parameter(Mandatory)]
        [string]$ExpectedMessage
    )

    $caseDocs = Join-Path $tempRoot $Name
    Copy-Item -LiteralPath $docs -Destination $caseDocs -Recurse
    $research = Join-Path $caseDocs '03_architecture\08_SOURCE_TOOL_RESEARCH.md'
    & $Mutate $research $caseDocs

    $failedAsExpected = $false
    try {
        & $checker -DocsRoot $caseDocs | Out-Null
    }
    catch {
        $failedAsExpected = $true
        if (-not $_.Exception.Message.Contains($ExpectedMessage)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failedAsExpected) {
        throw "Case '$Name' unexpectedly passed document traceability"
    }
}

try {
    & $checker -DocsRoot $docs | Out-Null

    Assert-CheckerFails -Name 'missing-user-wording-authority' -ExpectedMessage 'Missing authority document: 00_requirements/-1_USER_WORDING.md' -Mutate {
        param($research, $caseDocs)
        Remove-Item -LiteralPath (Join-Path $caseDocs '00_requirements\-1_USER_WORDING.md') -Force
    }

    Assert-CheckerFails -Name 'missing-required-source' -ExpectedMessage 'exactly one required source_id: source.feishu' -Mutate {
        param($research)
        $lines = @(Get-Content -Encoding utf8 -LiteralPath $research)
        $filtered = @($lines | Where-Object { $_ -notmatch '^\|\s*source\.feishu\s*\|' })
        if ($filtered.Count -ne $lines.Count - 1) {
            throw 'Mutation setup did not remove exactly one source.feishu row'
        }
        Set-Content -Encoding utf8 -LiteralPath $research -Value $filtered
    }

    Assert-CheckerFails -Name 'empty-kimi-evidence' -ExpectedMessage 'empty required field: current_evidence' -Mutate {
        param($research)
        Set-SourceFieldEmpty -ResearchPath $research -SourceId 'source.kimi' -CellIndex 5
    }

    Assert-CheckerFails -Name 'empty-kimi-authorization' -ExpectedMessage 'empty required field: minimum_authorization' -Mutate {
        param($research)
        Set-SourceFieldEmpty -ResearchPath $research -SourceId 'source.kimi' -CellIndex 4
    }

    Assert-CheckerFails -Name 'e1-wechat-channels-marked-available' -ExpectedMessage 'below E3 and must remain disabled' -Mutate {
        param($research)
        $lines = @(Get-Content -Encoding utf8 -LiteralPath $research)
        $index = 0..($lines.Count - 1) | Where-Object { $lines[$_] -match '^\|\s*source\.wechat_channels\s*\|' }
        if (@($index).Count -ne 1) {
            throw 'Mutation setup expected exactly one source.wechat_channels row'
        }
        $cells = @($lines[$index].Split('|'))
        $cells[7] = ' available '
        $lines[$index] = $cells -join '|'
        Set-Content -Encoding utf8 -LiteralPath $research -Value $lines
    }

    Write-Output 'Document traceability mutation tests passed: a missing -1 wording authority, missing/empty source fields, and below-E3 availability promotion all fail closed.'
}
finally {
    if (Test-Path -LiteralPath $tempRoot) {
        $resolvedTemp = (Resolve-Path -LiteralPath $tempRoot).Path
        $systemTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolvedTemp.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected test path: $resolvedTemp"
        }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
