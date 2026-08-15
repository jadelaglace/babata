$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$docs = Join-Path $repo '00_docs'
$repoReadme = Join-Path $repo 'README.md'
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

    $caseRoot = Join-Path $tempRoot $Name
    $caseDocs = Join-Path $caseRoot '00_docs'
    New-Item -ItemType Directory -Path $caseRoot | Out-Null
    Copy-Item -LiteralPath $docs -Destination $caseDocs -Recurse
    $caseReadme = Join-Path $caseRoot 'README.md'
    Copy-Item -LiteralPath $repoReadme -Destination $caseReadme
    $research = Join-Path $caseDocs '03_architecture\03_d_SOURCE_ROUTE_REGISTRY.md'
    & $Mutate $research $caseDocs $caseReadme

    $failedAsExpected = $false
    try {
        & $checker -DocsRoot $caseDocs -RepoReadmePath $caseReadme | Out-Null
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
    & $checker -DocsRoot $docs -RepoReadmePath $repoReadme | Out-Null

    Assert-CheckerFails -Name 'missing-wording-role-marker' -ExpectedMessage 'Missing document authority role' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '00_requirements\00_b_USER_WORDING.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('DOC-AUTHORITY-BOUNDARY: curated-current-intent', 'current intent')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'root-readme-live-status' -ExpectedMessage 'live phase snapshot in repository README' -Mutate {
        param($research, $caseDocs, $caseReadme)
        Add-Content -Encoding utf8 -LiteralPath $caseReadme -Value "`nP8.9 进行中。`n"
    }

    Assert-CheckerFails -Name 'delivery-plan-dated-result' -ExpectedMessage 'dated execution result in mba_rollout' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '04_process\04_c_MBA_C2B_ROLLOUT.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`n2026-08-15 execution result: passed.`n"
    }

    Assert-CheckerFails -Name 'mba-rollout-concrete-deferred-scope' -ExpectedMessage 'concrete deferred scope in MBA rollout plan' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '04_process\04_c_MBA_C2B_ROLLOUT.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`nBilibili 和 132 个群聊继续暂缓。`n"
    }

    Assert-CheckerFails -Name 'route-registry-usage-history' -ExpectedMessage 'usage or batch history in source route registry' -Mutate {
        param($research)
        Add-Content -Encoding utf8 -LiteralPath $research -Value "`nP8.4 completed run history.`n"
    }

    Assert-CheckerFails -Name 'route-registry-user-defer-status' -ExpectedMessage 'user-scope status in source route registry' -Mutate {
        param($research)
        Add-Content -Encoding utf8 -LiteralPath $research -Value "`n该来源用户暂缓，不进入当前分母。`n"
    }

    Assert-CheckerFails -Name 'route-registry-legacy-available-status' -ExpectedMessage 'legacy available route status in source_routes' -Mutate {
        param($research)
        Add-Content -Encoding utf8 -LiteralPath $research -Value "`nCurrent route is available.`n"
    }

    Assert-CheckerFails -Name 'architecture-legacy-available-status' -ExpectedMessage 'legacy available route status in architecture' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '03_architecture\03_a_ARCHITECTURE.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`nRuntime route is available.`n"
    }

    Assert-CheckerFails -Name 'missing-user-wording-authority' -ExpectedMessage 'Missing authority document: 00_requirements/00_b_USER_WORDING.md' -Mutate {
        param($research, $caseDocs)
        Remove-Item -LiteralPath (Join-Path $caseDocs '00_requirements\00_b_USER_WORDING.md') -Force
    }

    Assert-CheckerFails -Name 'missing-wording-recovery-authority' -ExpectedMessage 'Missing authority document: 00_requirements/00_c_USER_WORDING_RECOVERY.md' -Mutate {
        param($research, $caseDocs)
        Remove-Item -LiteralPath (Join-Path $caseDocs '00_requirements\00_c_USER_WORDING_RECOVERY.md') -Force
    }

    Assert-CheckerFails -Name 'missing-active-plan-authority' -ExpectedMessage 'Missing authority document: 04_process/04_f_ACTIVE_PLAN.md' -Mutate {
        param($research, $caseDocs)
        Remove-Item -LiteralPath (Join-Path $caseDocs '04_process\04_f_ACTIVE_PLAN.md') -Force
    }

    Assert-CheckerFails -Name 'missing-usage-authority' -ExpectedMessage 'Missing authority document: 04_process/04_b_USAGE_STATUS.md' -Mutate {
        param($research, $caseDocs)
        Remove-Item -LiteralPath (Join-Path $caseDocs '04_process\04_b_USAGE_STATUS.md') -Force
    }

    Assert-CheckerFails -Name 'missing-prd-role-marker' -ExpectedMessage 'Missing document authority role' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '01_prd\01_a_PRD.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('DOC-AUTHORITY-BOUNDARY: product-behavior', 'product document')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'missing-prd-doc-id' -ExpectedMessage "Missing stable document ID 'DOC-PRD'" -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '01_prd\01_a_PRD.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('DOC-ID: DOC-PRD', 'document identifier removed')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }

    Assert-CheckerFails -Name 'invalid-process-secondary-number' -ExpectedMessage 'Invalid intra-folder document number/name' -Mutate {
        param($research, $caseDocs)
        Copy-Item -LiteralPath (Join-Path $caseDocs '04_process\04_b_USAGE_STATUS.md') -Destination (Join-Path $caseDocs '04_process\05_USAGE_STATUS.md')
    }

    Assert-CheckerFails -Name 'registry-loses-glossary' -ExpectedMessage 'Document control plane is missing registry/glossary marker' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs 'README.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('## 5. 核心术语词典', '## Terms removed')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }
    Assert-CheckerFails -Name 'registry-doc-id-path-mismatch' -ExpectedMessage 'registry row for DOC-MODALITY-LADDER is missing current path' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs 'README.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('03_architecture/03_g_C1B_C2B_MODALITY_LADDER.md', '03_architecture/03_x_WRONG.md')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
    }
    Assert-CheckerFails -Name 'architecture-redefines-phase-gate' -ExpectedMessage 'competing phase-gate definition in p2_design' -Mutate {
        param($research, $caseDocs)
        Add-Content -Encoding utf8 -LiteralPath (Join-Path $caseDocs '03_architecture\03_b_P2_SYSTEM_SKELETON.md') -Value "`n- P2-G2: competing architecture definition.`n"
    }

    Assert-CheckerFails -Name 'prd-concrete-current-batch' -ExpectedMessage 'concrete runtime batch path in prd' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '01_prd\01_a_PRD.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`nCurrent batch: mba-finance-c2b-benchmark-20260815-v17.`n"
    }

    Assert-CheckerFails -Name 'acceptance-dated-result' -ExpectedMessage 'dated execution result in acceptance' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`n2026-08-15 execution result: passed.`n"
    }

    Assert-CheckerFails -Name 'tests-dated-result' -ExpectedMessage 'dated execution result in tests' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '05_tests\05_a_TEST_CASES.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`n2026-08-15 execution result: passed.`n"
    }

    Assert-CheckerFails -Name 'prd-loses-usage-boundary' -ExpectedMessage 'PRD is missing product behavior versus usage result boundary' -Mutate {
        param($research, $caseDocs)
        $path = Join-Path $caseDocs '01_prd\01_a_PRD.md'
        $text = Get-Content -Raw -Encoding utf8 -LiteralPath $path
        $text = $text.Replace('具体试跑、试点、模板接受和全量运行结果统一记录在 usage/evidence，不反向改写产品行为', '具体结果另行记录')
        Set-Content -Encoding utf8 -LiteralPath $path -Value $text
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

    Assert-CheckerFails -Name 'e1-wechat-channels-marked-enabled' -ExpectedMessage 'below E3 and cannot be enabled' -Mutate {
        param($research)
        $lines = @(Get-Content -Encoding utf8 -LiteralPath $research)
        $index = 0..($lines.Count - 1) | Where-Object { $lines[$_] -match '^\|\s*source\.wechat_channels\s*\|' }
        if (@($index).Count -ne 1) {
            throw 'Mutation setup expected exactly one source.wechat_channels row'
        }
        $cells = @($lines[$index].Split('|'))
        $cells[7] = ' enabled '
        $lines[$index] = $cells -join '|'
        Set-Content -Encoding utf8 -LiteralPath $research -Value $lines
    }

    Write-Output 'Document traceability mutation tests passed: authority roles, usage ownership, product/result boundaries, and source-route evidence all fail closed.'
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
