$ErrorActionPreference = 'Stop'

function Get-Utf8Sha256 {
    param(
        [Parameter(Mandatory)]
        [string]$Value
    )

    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Value)
        return ([BitConverter]::ToString($sha256.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
    }
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$docs = Join-Path $repo '00_docs'
$checker = Join-Path $PSScriptRoot 'check-doc-provenance.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("babata-doc-provenance-{0}" -f [guid]::NewGuid())

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
    & $Mutate $caseDocs

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
        throw "Case '$Name' unexpectedly passed document provenance"
    }
}

try {
    & $checker -DocsRoot $docs | Out-Null

    Assert-CheckerFails -Name 'mutated-user-wording' -ExpectedMessage 'missing or changed a required verbatim excerpt' -Mutate {
        param($caseDocs)
        $path = Join-Path $caseDocs '00_requirements\00_REQUIREMENTS.md'
        $lines = @(Get-Content -Encoding utf8 -LiteralPath $path)
        $targetHash = 'ef88a5a23026a980540f196a3342da8bf2ebcc24d2c43f3da113b6a419365581'
        $matchingIndexes = @()
        for ($index = 0; $index -lt $lines.Count; $index++) {
            if (-not [string]::IsNullOrEmpty($lines[$index]) -and
                (Get-Utf8Sha256 -Value $lines[$index]) -eq $targetHash) {
                $matchingIndexes += $index
            }
        }
        if ($matchingIndexes.Count -ne 1) {
            throw "Mutation setup expected one verbatim line, found $($matchingIndexes.Count)"
        }
        $lines[$matchingIndexes[0]] += ' [mutated]'
        Set-Content -Encoding utf8 -LiteralPath $path -Value $lines
    }

    Assert-CheckerFails -Name 'local-thread-id' -ExpectedMessage 'UUID-shaped local/runtime identifier' -Mutate {
        param($caseDocs)
        $path = Join-Path $caseDocs '03_architecture\09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value ("`nLocal turn: {0}" -f [guid]::NewGuid())
    }

    Assert-CheckerFails -Name 'builder-as-user-wording' -ExpectedMessage 'missing provenance distinction' -Mutate {
        param($caseDocs)
        $path = Join-Path $caseDocs '03_architecture\09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md'
        $content = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $content = $content.Replace(
            '<!-- P6-TREE-PROVENANCE: builder-summary; user-confirmed; not-verbatim -->',
            '<!-- P6-TREE-PROVENANCE: user-verbatim -->'
        )
        Set-Content -Encoding utf8 -LiteralPath $path -Value $content
    }

    Assert-CheckerFails -Name 'sensitive-token' -ExpectedMessage 'possible bearer token' -Mutate {
        param($caseDocs)
        $path = Join-Path $caseDocs '03_architecture\09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value ("`nBearer {0}" -f ('a' * 32))
    }

    Write-Output 'Document provenance mutation tests passed: changed user wording, local UUID, Builder/user conflation, and a sensitive token pattern all fail closed.'
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
