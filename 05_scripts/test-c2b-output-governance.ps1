$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-c2b-output-governance.ps1'
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("babata-c2b-governance-{0}" -f [guid]::NewGuid())
$requiredFiles = @(
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\01_prd\01_a_PRD.md',
    '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md',
    '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md',
    '00_docs\04_process\04_b_USAGE_STATUS.md',
    '00_docs\05_tests\05_a_TEST_CASES.md',
    '02_skills\00_specs\09_outputs.md',
    '02_skills\00_specs\templates\semantic-obsidian-profile.md',
    '02_skills\babata-clean\SKILL.md',
    '02_skills\babata-clean\agents\openai.yaml',
    '05_scripts\register-finance-c1b-handoff.ps1',
    '05_scripts\verify-finance-c2b-formal-closure.ps1',
    '05_scripts\materialize-finance-c2b-from-c1b.ps1',
    '05_scripts\render-finance-course-map.ps1',
    '05_scripts\publish-finance-c2b-live.ps1',
    '05_scripts\register-finance-c2b-knowledge.ps1',
    '05_scripts\prepare-mba-course-c1b.ps1',
    '05_scripts\select-mba-course-c1-candidate.ps1',
    '05_scripts\build-mba-course-learning-docs.ps1',
    '05_scripts\group-mba-learning-digests.ps1',
    '05_scripts\register-mba-course-c1b-handoff.ps1',
    '05_scripts\register-mba-course-c2b-knowledge.ps1',
    '05_scripts\materialize-mba-course-c2b.ps1',
    '05_scripts\render-mba-course-map.ps1',
    '05_scripts\check-mba-course-c2b-package.ps1',
    '05_scripts\publish-mba-course-c2b-live.ps1',
    '05_scripts\verify-mba-course-c2b-closure.ps1',
    '05_scripts\test-select-mba-course-c1-candidate.ps1',
    '05_scripts\test-register-mba-course-c1b-handoff.ps1',
    '05_scripts\test-register-mba-course-c2b-knowledge.ps1',
    '05_scripts\test-group-mba-learning-digests.ps1',
    '05_scripts\test-mba-course-c2b-package.ps1'
)

function New-TestRepo {
    param([Parameter(Mandatory)][string]$Name)

    $caseRoot = Join-Path $tempRoot $Name
    foreach ($relativePath in $requiredFiles) {
        $destination = Join-Path $caseRoot $relativePath
        New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $repo $relativePath) -Destination $destination
    }
    return $caseRoot
}

function Remove-MarkerOnce {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Marker
    )

    $text = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    $index = $text.IndexOf($Marker, [StringComparison]::Ordinal)
    if ($index -lt 0) { throw "Mutation setup could not find marker: $Marker" }
    $text = $text.Remove($index, $Marker.Length)
    Set-Content -LiteralPath $Path -Value $text -Encoding utf8
}

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Mutate,
        [Parameter(Mandatory)][string]$ExpectedMessage
    )

    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    $failedAsExpected = $false
    try {
        & $checker -RepoRoot $caseRoot | Out-Null
    }
    catch {
        $failedAsExpected = $true
        if (-not $_.Exception.Message.Contains($ExpectedMessage)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failedAsExpected) {
        throw "Case '$Name' unexpectedly passed C2B output governance"
    }
}

function Assert-GenericStatus {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Mutate,
        [Parameter(Mandatory)][string]$ExpectedStatus,
        [Parameter(Mandatory)][string]$ExpectedDetail
    )

    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    $output = @(& $checker -RepoRoot $caseRoot)
    $genericLine = @($output | Where-Object { [string]$_ -like 'Reusable generic MBA path:*' })
    if ($genericLine.Count -ne 1 -or -not $genericLine[0].Contains("Reusable generic MBA path: $ExpectedStatus") -or
        -not $genericLine[0].Contains($ExpectedDetail)) {
        throw "Case '$Name' returned the wrong generic MBA status: $($genericLine -join ' | ')"
    }
    $financeLine = @($output | Where-Object { [string]$_ -like 'Finance C2B reference validation:*' })
    if ($financeLine.Count -ne 1 -or -not $financeLine[0].Contains('Finance C2B reference validation: passed')) {
        throw "Case '$Name' lost the separate finance reference result"
    }
}

try {
    New-Item -ItemType Directory -Path $tempRoot | Out-Null
    $repoOutput = @(& $checker -RepoRoot $repo)
    if (@($repoOutput | Where-Object { [string]$_ -like 'Reusable generic MBA path:*' }).Count -ne 1) {
        throw 'Current generic MBA path did not report exactly one explicit readiness status'
    }
    if (@($repoOutput | Where-Object { [string]$_ -like 'Finance C2B reference validation: passed*' }).Count -ne 1) {
        throw 'Finance reference validation did not report its separate passed status'
    }

    Assert-GenericStatus -Name 'generic-mba-missing-components' `
        -ExpectedStatus 'candidate/unavailable' `
        -ExpectedDetail 'missing-component:05_scripts\materialize-mba-course-c2b.ps1' `
        -Mutate {
            param($caseRoot)
            Remove-Item -LiteralPath (Join-Path $caseRoot '05_scripts\materialize-mba-course-c2b.ps1')
        }

    Assert-GenericStatus -Name 'generic-mba-partial-publisher-gate' `
        -ExpectedStatus 'candidate/unavailable' `
        -ExpectedDetail 'missing-gate:05_scripts\publish-mba-course-c2b-live.ps1:pending-user-acceptance' `
        -Mutate {
            param($caseRoot)
            $path = Join-Path $caseRoot '05_scripts\publish-mba-course-c2b-live.ps1'
            $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
            $text = $text.Replace('pending_user_acceptance','awaiting_review')
            Set-Content -LiteralPath $path -Value $text -Encoding utf8
        }

    Assert-GenericStatus -Name 'generic-mba-dedicated-test-failure' `
        -ExpectedStatus 'candidate/unavailable' `
        -ExpectedDetail 'dedicated-test-failed:05_scripts\test-mba-course-c2b-package.ps1' `
        -Mutate {
            param($caseRoot)
            Add-Content -LiteralPath (Join-Path $caseRoot '05_scripts\test-mba-course-c2b-package.ps1') `
                -Value "`nthrow 'injected dedicated test failure'" -Encoding utf8
        }

    Assert-GenericStatus -Name 'generic-mba-complete-and-tested' `
        -ExpectedStatus 'available' `
        -ExpectedDetail 'dedicated-tests=5' `
        -Mutate {
            param($caseRoot)
        }

    Assert-CheckerFails -Name 'missing-outputs-docs-first' -ExpectedMessage 'C2B-DOCS-FIRST-GATE in outputs' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\09_outputs.md') 'C2B-DOCS-FIRST-GATE'
    }
    Assert-CheckerFails -Name 'usage-loses-current-obsidian-uri' -ExpectedMessage 'current user-facing Obsidian URI in usage status' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md') 'obsidian://open?vault=Obsidian%20Vault&file=Babata%2FMBA%2Fmba_finance_c2b_latest%2Findex.md'
    }
    Assert-CheckerFails -Name 'outputs-hardcodes-live-obsidian-uri' -ExpectedMessage 'concrete live Obsidian URI outside usage status: outputs' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\00_specs\09_outputs.md'
        Add-Content -Encoding utf8 -LiteralPath $path -Value "`nobsidian://open?vault=Obsidian%20Vault&file=Babata%2FMBA%2Fmba_finance_c2b_latest%2Findex.md`n"
    }
    Assert-CheckerFails -Name 'missing-formal-c1b-handoff' -ExpectedMessage 'C1B-FORMAL-HANDOFF-GATE in outputs' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\09_outputs.md') 'C1B-FORMAL-HANDOFF-GATE'
    }
    Assert-CheckerFails -Name 'clean-skill-forces-all-work-into-c1b' -ExpectedMessage 'general C1 completion boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\babata-clean\SKILL.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('End a general cleaning task at validated C1.', 'End every cleaning task at C1B.')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'clean-skill-loses-explicit-c1b-scope' -ExpectedMessage 'explicit C1B scope boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\babata-clean\SKILL.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('When C1B/C2B is explicitly in scope', 'For every cleaning task')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'clean-skill-ui-description-too-long' -ExpectedMessage 'Clean Skill UI short_description with 25-64 characters' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\babata-clean\agents\openai.yaml'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = [regex]::Replace($text, '(?m)^\s*short_description:.*$', '  short_description: "This clean skill description is deliberately much longer than sixty four characters"')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'missing-profile-package-map' -ExpectedMessage 'C2B-PACKAGE-OWNED-COURSE-MAP in profile' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md') 'C2B-PACKAGE-OWNED-COURSE-MAP'
    }
    Assert-CheckerFails -Name 'missing-ledger-parameter' -ExpectedMessage 'mandatory KnowledgeUniverseLedgerPath parameter' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\materialize-finance-c2b-from-c1b.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('[Parameter(Mandatory=$true)][string]$KnowledgeUniverseLedgerPath,', '[string]$KnowledgeUniverseLedgerPath,')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'missing-c1b-registration-ledger-parameter' -ExpectedMessage 'mandatory C1BRegistrationLedgerPath parameter' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\materialize-finance-c2b-from-c1b.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('[Parameter(Mandatory=$true)][string]$C1BRegistrationLedgerPath,', '[string]$C1BRegistrationLedgerPath,')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'obsidian-reverts-to-live-candidate' -ExpectedMessage 'formal Obsidian frontmatter status' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\materialize-finance-c2b-from-c1b.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('status: accepted_benchmark', 'status: live_candidate')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'profile-reverts-to-candidate' -ExpectedMessage 'accepted reusable Obsidian profile status' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Profile status: accepted', 'Profile status: candidate')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'publisher-allows-candidate-package' -ExpectedMessage 'publisher formal C2B status rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\publish-finance-c2b-live.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('accepted_benchmark / registered C2B package', 'candidate package rejected')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'closure-skips-database-integrity' -ExpectedMessage 'closure database integrity verification' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\verify-finance-c2b-formal-closure.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('PRAGMA quick_check;', 'SELECT 1;')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-GenericStatus -Name 'generic-closure-skips-user-acceptance' `
        -ExpectedStatus 'candidate/unavailable' `
        -ExpectedDetail 'missing-gate:05_scripts\verify-mba-course-c2b-closure.ps1:explicit-user-acceptance' `
        -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\verify-mba-course-c2b-closure.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Explicit user acceptance evidence is required', 'Acceptance evidence omitted')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-package-boundary' -ExpectedMessage 'renderer package-only boundary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('[Parameter(Mandatory=$true)][string]$PackageRoot', '[string]$PackageRoot')
        if ($text.Contains('[Parameter(Mandatory=$true)][string]$PackageRoot')) {
            throw 'Mutation setup did not remove the renderer package boundary'
        }
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'missing-requirements-docs-first' -ExpectedMessage 'C2B-DOCS-FIRST-GATE in requirements' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md') 'C2B-DOCS-FIRST-GATE'
    }
    Assert-CheckerFails -Name 'missing-profile-mece-map' -ExpectedMessage 'C2B-MECE-COURSE-MAP-GATE in profile' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md') 'C2B-MECE-COURSE-MAP-GATE'
    }
    Assert-CheckerFails -Name 'missing-profile-crash-course-map' -ExpectedMessage 'C2B-CRASH-COURSE-MAP-GATE in profile' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md') 'C2B-CRASH-COURSE-MAP-GATE'
    }
    Assert-CheckerFails -Name 'missing-profile-modern-visual-map' -ExpectedMessage 'C2B-MODERN-VISUAL-MAP-GATE in profile' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md') 'C2B-MODERN-VISUAL-MAP-GATE'
    }
    Assert-CheckerFails -Name 'missing-profile-right-growing-map' -ExpectedMessage 'C2B-RIGHT-GROWING-MINDMAP-GATE in profile' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md') 'C2B-RIGHT-GROWING-MINDMAP-GATE'
    }
    Assert-CheckerFails -Name 'missing-profile-responsive-map' -ExpectedMessage 'C2B-RESPONSIVE-MAP-GATE in profile' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('C2B-RESPONSIVE-MAP-GATE', '')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'missing-obsidian-human-view-boundary' -ExpectedMessage 'OBSIDIAN-HUMAN-VIEW-BOUNDARY in outputs' -Mutate {
        param($caseRoot)
        Remove-MarkerOnce (Join-Path $caseRoot '02_skills\00_specs\09_outputs.md') 'OBSIDIAN-HUMAN-VIEW-BOUNDARY'
    }
    Assert-CheckerFails -Name 'renderer-loses-crash-course-content' -ExpectedMessage 'crash-course knowledge token: NPV > 0' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('NPV > 0', 'positive project')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-body-grounding-gate' -ExpectedMessage 'crash-course body-grounding rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Crash-course knowledge detail is not grounded in accepted C2B body', 'Course summary evidence absent')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-modern-curve' -ExpectedMessage 'modern soft Mermaid curve' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('"curve": "basis"', '"curve": "linear"')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-disables-responsive-sizing' -ExpectedMessage 'responsive Mermaid maximum-width setting' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('"useMaxWidth": true', '"useMaxWidth": false')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-responsive-svg-root-gate' -ExpectedMessage 'rendered SVG responsive-root rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Responsive Mermaid SVG root must have width="100%", viewBox, and controlled max-width', 'Responsive SVG check failed')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-restores-knowledge-card' -ExpectedMessage 'restores a forbidden knowledge-card or bilateral layout' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text += "`n# classDef knowledge-card`n"
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-restores-empty-knowledge-layer' -ExpectedMessage 'retains an empty course-knowledge intermediary' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text += "`n# '  knowledge[`"课程知识`"]`n"
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-effective-font-gate' -ExpectedMessage 'minimum 11px effective type threshold' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('$equivalentFontSize -lt 11.0', '$equivalentFontSize -lt 8.0')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-aspect-ratio-gate' -ExpectedMessage 'maximum 1.40 aspect-ratio threshold' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('$aspectRatio -gt 1.40', '$aspectRatio -gt 2.50')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-native-internal-link' -ExpectedMessage 'Obsidian native internal-link nodes' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('internal-link', 'course-leaf')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-compact-width' -ExpectedMessage 'explicit readable PNG fallback width' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('![[media/财务管理课程脑图.png|760]]', '![[media/财务管理课程脑图.png]]')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-expands-png-fallback' -ExpectedMessage 'default-collapsed PNG fallback callout' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('> [!info]- 位图版本（打印 / 离线 / 渲染回退）', '[!info] 位图版本')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'materializer-allows-duplicate-visual-sections' -ExpectedMessage 'duplicate visual-evidence section rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\materialize-finance-c2b-from-c1b.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Duplicate visual-evidence sections', 'Repeated display blocks')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }
    Assert-CheckerFails -Name 'renderer-loses-obsidian-selector-check' -ExpectedMessage 'Obsidian postprocessor selector rejection' -Mutate {
        param($caseRoot)
        $path = Join-Path $caseRoot '05_scripts\render-finance-course-map.ps1'
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $text = $text.Replace('Obsidian Mermaid postprocessor selector mismatch', 'Mermaid label mismatch')
        Set-Content -LiteralPath $path -Value $text -Encoding utf8
    }

    Write-Output 'C2B output governance mutation tests passed.'
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
