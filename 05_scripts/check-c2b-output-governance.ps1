[CmdletBinding()]
param(
    [string]$RepoRoot
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Join-Path $PSScriptRoot '..'
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

function Read-RepoFile {
    param([Parameter(Mandatory)][string]$RelativePath)

    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "C2B governance is missing required file: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding utf8
}

function Assert-Contains {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Label
    )

    if (-not $Text.Contains($Value)) {
        throw "C2B governance is missing $Label"
    }
}

function Assert-Matches {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$Pattern,
        [Parameter(Mandatory)][string]$Label
    )

    if ($Text -notmatch $Pattern) {
        throw "C2B governance is missing $Label"
    }
}

function Get-GenericMbaPathStatus {
    $componentRules = [ordered]@{
        '05_scripts\prepare-mba-course-c1b.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'pending-user-acceptance' = 'pending_user_acceptance'
            'fresh-staging-root' = 'Use a fresh MBA course staging root'
            'fingerprint-candidate-selector' = 'select-mba-course-c1-candidate.ps1'
            'explicit-divergent-run-selection' = 'c1_run_overrides'
        }
        '05_scripts\select-mba-course-c1-candidate.ps1' = [ordered]@{
            'same-fingerprint-deduplication' = 'Repeated successful runs with the same managed content are one C1 identity.'
            'divergent-fingerprint-rejection' = 'divergent fingerprints'
        }
        '05_scripts\build-mba-course-learning-docs.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'candidate-learning-docs' = "status='candidate'"
            'fresh-staging-root' = 'Use a fresh learning-doc staging root'
            'bounded-hierarchical-reduction' = 'Group-MbaLearningDigestNodes'
            'reduction-module-coverage' = 'omitted module M-'
        }
        '05_scripts\register-mba-course-c1b-handoff.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'formal-process-registration' = "'process','register'"
            'registered-ledger-schema' = 'babata.mba-course-c1b-registration/v1'
            'fingerprint-candidate-selector' = 'Select-MbaCourseC1Candidate'
        }
        '05_scripts\register-mba-course-c2b-knowledge.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'pending-user-acceptance' = 'pending_user_acceptance'
            'formal-knowledge-ingest' = "'knowledge','ingest'"
        }
        '05_scripts\materialize-mba-course-c2b.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'pending-user-acceptance' = 'pending_user_acceptance'
            'formal-c1b-ledger-input' = 'C1BRegistrationLedgerPath'
            'formal-knowledge-ledger-input' = 'KnowledgeUniverseLedgerPath'
            'generic-map-renderer' = 'render-mba-course-map.ps1'
        }
        '05_scripts\render-mba-course-map.ps1' = [ordered]@{
            'package-only-input' = 'PackageRoot'
            'responsive-mermaid' = '"useMaxWidth": true'
            'native-internal-links' = 'internal-link'
            'right-growing-layout' = 'C2B-RIGHT-GROWING-MINDMAP-GATE'
        }
        '05_scripts\check-mba-course-c2b-package.ps1' = [ordered]@{
            'course-plan-schema' = 'babata.mba-course-c2b-plan/v1'
            'pending-user-acceptance' = 'pending_user_acceptance'
            'finance-status-rejection' = 'accepted_benchmark'
            'formal-registration-gate' = 'Package requires formal C1B and knowledge-universe registration'
            'dynamic-course-map-partition' = 'Course-map knowledge branches must partition the chapter notes exactly'
        }
        '05_scripts\publish-mba-course-c2b-live.ps1' = [ordered]@{
            'generic-package-checker' = 'check-mba-course-c2b-package.ps1'
            'pending-user-acceptance' = 'pending_user_acceptance'
            'finance-status-rejection' = 'accepted_benchmark'
            'checker-schema' = 'babata.mba-course-c2b-package-check/v1'
        }
        '05_scripts\verify-mba-course-c2b-closure.ps1' = [ordered]@{
            'explicit-user-acceptance' = 'Explicit user acceptance evidence is required'
            'accepted-course-result' = "course_acceptance='accepted'"
            'closed-course-result' = "closure='closed'"
            'package-live-hashes' = 'package/live hash failures'
            'published-package-root' = 'PublishedPackageRoot'
            'presentation-rename-map' = 'renamed_learning_support'
            'database-integrity' = 'PRAGMA quick_check;'
        }
    }
    $dedicatedTests = @(
        '05_scripts\test-select-mba-course-c1-candidate.ps1',
        '05_scripts\test-register-mba-course-c1b-handoff.ps1',
        '05_scripts\test-register-mba-course-c2b-knowledge.ps1',
        '05_scripts\test-group-mba-learning-digests.ps1',
        '05_scripts\test-mba-course-c2b-package.ps1'
    )
    $reasons = [Collections.Generic.List[string]]::new()

    foreach ($relativePath in $componentRules.Keys) {
        $path = Join-Path $RepoRoot $relativePath
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            [void]$reasons.Add("missing-component:$relativePath")
            continue
        }
        $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
        foreach ($gate in $componentRules[$relativePath].GetEnumerator()) {
            if (-not $text.Contains([string]$gate.Value)) {
                [void]$reasons.Add("missing-gate:${relativePath}:$($gate.Key)")
            }
        }
    }
    foreach ($relativePath in $dedicatedTests) {
        if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $relativePath) -PathType Leaf)) {
            [void]$reasons.Add("missing-dedicated-test:$relativePath")
        }
    }

    if ($reasons.Count -eq 0) {
        foreach ($relativePath in $dedicatedTests) {
            try {
                & (Join-Path $RepoRoot $relativePath) | Out-Null
            }
            catch {
                [void]$reasons.Add("dedicated-test-failed:${relativePath}:$($_.Exception.Message)")
                break
            }
        }
    }

    if ($reasons.Count -eq 0) {
        return [pscustomobject]@{ status = 'available'; reasons = @(); tests = $dedicatedTests }
    }
    return [pscustomobject]@{ status = 'candidate/unavailable'; reasons = @($reasons); tests = $dedicatedTests }
}

$documents = [ordered]@{
    requirements = Read-RepoFile '00_docs\00_requirements\00_a_REQUIREMENTS.md'
    prd = Read-RepoFile '00_docs\01_prd\01_a_PRD.md'
    acceptance = Read-RepoFile '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md'
    process = Read-RepoFile '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md'
    usage = Read-RepoFile '00_docs\04_process\04_b_USAGE_STATUS.md'
    tests = Read-RepoFile '00_docs\05_tests\05_a_TEST_CASES.md'
    outputs = Read-RepoFile '02_skills\00_specs\09_outputs.md'
    profile = Read-RepoFile '02_skills\00_specs\templates\semantic-obsidian-profile.md'
    cleanSkill = Read-RepoFile '02_skills\babata-clean\SKILL.md'
    cleanMetadata = Read-RepoFile '02_skills\babata-clean\agents\openai.yaml'
}

foreach ($name in @('outputs', 'profile', 'cleanSkill')) {
    Assert-Contains $documents[$name] 'C2B-DOCS-FIRST-GATE' "C2B-DOCS-FIRST-GATE in $name"
    Assert-Contains $documents[$name] 'C1B-FORMAL-HANDOFF-GATE' "C1B-FORMAL-HANDOFF-GATE in $name"
}
foreach ($name in @('outputs', 'profile')) {
    Assert-Contains $documents[$name] 'OBSIDIAN-HUMAN-VIEW-BOUNDARY' "OBSIDIAN-HUMAN-VIEW-BOUNDARY in $name"
}
foreach ($name in @('outputs', 'profile')) {
    Assert-Contains $documents[$name] 'C2B-KNOWLEDGE-UNIVERSE-GATE' "C2B-KNOWLEDGE-UNIVERSE-GATE in $name"
    Assert-Contains $documents[$name] 'C2B-PACKAGE-OWNED-COURSE-MAP' "C2B-PACKAGE-OWNED-COURSE-MAP in $name"
    Assert-Contains $documents[$name] 'C2B-MECE-COURSE-MAP-GATE' "C2B-MECE-COURSE-MAP-GATE in $name"
    Assert-Contains $documents[$name] 'C2B-CRASH-COURSE-MAP-GATE' "C2B-CRASH-COURSE-MAP-GATE in $name"
    Assert-Contains $documents[$name] 'C2B-MODERN-VISUAL-MAP-GATE' "C2B-MODERN-VISUAL-MAP-GATE in $name"
    Assert-Contains $documents[$name] 'C2B-RIGHT-GROWING-MINDMAP-GATE' "C2B-RIGHT-GROWING-MINDMAP-GATE in $name"
    Assert-Contains $documents[$name] 'C2B-RESPONSIVE-MAP-GATE' "C2B-RESPONSIVE-MAP-GATE in $name"
}
Assert-Contains $documents.requirements 'Obsidian 等输出是可删除重建的阅读视图，不是新的权威存储' 'rebuildable-view requirement'
Assert-Contains $documents.requirements 'C1B 在完整文字基础上判断哪些图片、音频、视频片段或附件对理解不可替代' 'C1B requirement'
Assert-Contains $documents.requirements '课程输出使用唯一 live' 'unique-live requirement'
Assert-Contains $documents.prd 'publisher 只发布通过完整性和链接验证的 package 到唯一 live' 'verified package publication behavior'
Assert-Contains $documents.acceptance 'publisher 只复制已验证 package，唯一 live 不成为 writer' 'publisher acceptance boundary'
Assert-Contains $documents.process 'read-back、hash、链接、package、恢复或用户可见结果' 'output verification process'
Assert-Contains $documents.tests 'publisher 只复制已验证 package；唯一 live 不反写知识' 'output verification coverage'
Assert-Matches $documents.outputs '(?is)course index.*universe-level\s+large index|universe-level\s+large index.*course index' 'course-index versus universe-index boundary in outputs'
Assert-Matches $documents.profile '(?is)course index.*universe-level large index|universe-level large index.*course index' 'course-index versus universe-index boundary in profile'
Assert-Matches $documents.cleanSkill '(?is)does not reread\s+the\s+external sovereign original' 'C1B-to-C2B no-original-reread boundary'
Assert-Matches $documents.cleanSkill '(?is)does not ask\s+`babata-clean`\s+to become an output writer' 'C1B-to-C2B writer boundary'
Assert-Contains $documents.cleanSkill 'End a general cleaning task at validated C1.' 'general C1 completion boundary'
Assert-Contains $documents.cleanSkill 'When C1B/C2B is explicitly in scope' 'explicit C1B scope boundary'
Assert-Contains $documents.cleanSkill 'babata process register --pipeline agent_import' 'formal C1 registration command'
Assert-Contains $documents.cleanMetadata 'display_name: "Babata Clean"' 'consistent Clean Skill UI name'
Assert-Contains $documents.cleanMetadata 'Use $babata-clean' 'Clean Skill UI invocation prompt'
$cleanShortDescription = [regex]::Match($documents.cleanMetadata, '(?m)^\s*short_description:\s*"([^"]+)"\s*$')
if (-not $cleanShortDescription.Success -or
    $cleanShortDescription.Groups[1].Value.Length -lt 25 -or
    $cleanShortDescription.Groups[1].Value.Length -gt 64) {
    throw 'C2B governance is missing Clean Skill UI short_description with 25-64 characters'
}

$materializer = Read-RepoFile '05_scripts\materialize-finance-c2b-from-c1b.ps1'
$renderer = Read-RepoFile '05_scripts\render-finance-course-map.ps1'
$publisher = Read-RepoFile '05_scripts\publish-finance-c2b-live.ps1'
$c1bRegistration = Read-RepoFile '05_scripts\register-finance-c1b-handoff.ps1'
$registration = Read-RepoFile '05_scripts\register-finance-c2b-knowledge.ps1'
$closureVerification = Read-RepoFile '05_scripts\verify-finance-c2b-formal-closure.ps1'

Assert-Matches $materializer '(?is)\[Parameter\(Mandatory\s*=\s*\$true\)\]\[string\]\$C1BRegistrationLedgerPath' 'mandatory C1BRegistrationLedgerPath parameter'
Assert-Contains $materializer "`$c1bLedger.status -ne 'registered'" 'registered C1B ledger rejection'
Assert-Contains $materializer '37 registered decisions and 76 registered media derivatives' 'formal C1B coverage rejection'
Assert-Contains $materializer "status: accepted_benchmark" 'formal Obsidian frontmatter status'
Assert-Contains $materializer "template_status: accepted" 'accepted Obsidian template frontmatter status'
Assert-Matches $materializer '(?is)\[Parameter\(Mandatory\s*=\s*\$true\)\]\[string\]\$KnowledgeUniverseLedgerPath' 'mandatory KnowledgeUniverseLedgerPath parameter'
Assert-Contains $materializer "`$universe.status -ne 'registered'" 'registered knowledge-universe ledger rejection'
Assert-Matches $materializer "(?is)render-finance-course-map\.ps1'.*?-PackageRoot\s+\`$pkg" 'package course-map renderer call'
Assert-Contains $materializer "layout='single_root_right_growing_mindmap'" 'materialized right-growing course-map layout'
Assert-Contains $materializer 'png_display_width=760' 'materialized readable PNG width'
Assert-Contains $materializer 'effective_font_px=$mapEffectiveFont' 'materialized effective font metric'
Assert-Contains $materializer 'aspect_ratio=$mapAspect' 'materialized aspect-ratio metric'
Assert-Contains $materializer 'responsive_svg=$true' 'materialized responsive SVG contract'
Assert-Contains $materializer "default_expanded='mermaid'" 'single default-expanded Mermaid contract'
Assert-Contains $materializer 'png_default_collapsed=$true' 'materialized collapsed PNG fallback contract'
Assert-Contains $materializer 'Duplicate visual-evidence sections' 'duplicate visual-evidence section rejection'
Assert-Contains $materializer 'Visual evidence path must occur exactly once' 'duplicate visual-evidence path rejection'
if ($materializer -match '(?i)\$LiveVaultPath|\[switch\]\$Publish|publish-candidate') {
    throw 'C2B materializer contains a direct live-publish path'
}

Assert-Matches $renderer '(?is)param\s*\(.*?\[Parameter\(Mandatory\s*=\s*\$true\)\]\[string\]\$PackageRoot' 'renderer package-only boundary'
if ($renderer -match '(?i)\$LiveRoot|\$LiveVaultPath|\$SourceRoot|\$ExternalRoot') {
    throw 'C2B renderer accepts a source or live root outside PackageRoot'
}
Assert-Contains $renderer "'财务管理课程脑图.mmd'" 'package Mermaid source generation'
Assert-Contains $renderer "'财务管理课程脑图.png'" 'package PNG generation'
Assert-Matches $renderer '(?i)@mermaid-js/mermaid-cli' 'Mermaid PNG renderer'
Assert-Contains $renderer '%% MECE 主分类轴：财务决策对象；学习支持独立成层' 'single-axis MECE map declaration'
Assert-Contains $renderer "'flowchart LR'" 'compact left-to-right course map'
Assert-Contains $renderer 'internal-link' 'Obsidian native internal-link nodes'
Assert-Contains $renderer '> [!info]- 位图版本（打印 / 离线 / 渲染回退）' 'default-collapsed PNG fallback callout'
Assert-Contains $renderer '> ![[media/财务管理课程脑图.png|760]]' 'explicit readable PNG fallback width'
Assert-Contains $documents.usage 'DOC-AUTHORITY-BOUNDARY: usage-status' 'usage-status authority role'
Assert-Contains $documents.usage '专用 MBA/Cherno Obsidian publisher' 'specialized route status'
Assert-Contains $documents.usage '不是通用 capability' 'specialized/general capability boundary'
Assert-Contains $documents.outputs 'The current course path belongs to usage status' 'output-spec URI ownership boundary'
Assert-Contains $documents.profile 'exact live URI registered by the course manifest/usage status' 'profile URI ownership boundary'
foreach ($name in @('requirements', 'prd', 'acceptance', 'process', 'tests', 'outputs', 'profile')) {
    if ($documents[$name] -match '(?i)obsidian://open\?') {
        throw "C2B governance found a concrete live Obsidian URI outside usage status: $name"
    }
}
Assert-Contains $renderer '"nodeSpacing": 16' 'compact readable Mermaid node spacing'
Assert-Contains $renderer '"rankSpacing": 34' 'readable Mermaid column spacing'
Assert-Contains $renderer '"wrappingWidth": 300' 'controlled knowledge-detail wrapping width'
Assert-Contains $renderer '"fontSize": "22px"' 'readable Mermaid font size'
Assert-Contains $renderer '"curve": "basis"' 'modern soft Mermaid curve'
Assert-Contains $renderer 'C2B-RIGHT-GROWING-MINDMAP-GATE' 'right-growing mind-map contract'
Assert-Contains $renderer 'single_root_right_growing' 'right-growing layout metric'
Assert-Contains $renderer 'classDef junction' 'small junction-dot style'
Assert-Contains $renderer 'classDef detail fill:transparent' 'borderless knowledge-detail style'
Assert-Contains $renderer 'finance --- $($group.id)' 'root-to-domain right-growing connection'
Assert-Contains $renderer '$($node.id)J --- $($node.id)D$($i + 1)' 'chapter-to-knowledge-detail path'
if ($renderer.Contains("'  knowledge[")) {
    throw 'C2B renderer retains an empty course-knowledge intermediary'
}
Assert-Contains $renderer 'five decision domains, eight chapters, four learning aids, and twenty grounded details' 'crash-course detail completeness rejection'
Assert-Contains $renderer 'Crash-course knowledge detail is not grounded in accepted C2B body' 'crash-course body-grounding rejection'
foreach ($knowledgeToken in @('股东财富最大化', 'NPV > 0', 'EOQ =', 'WACC｜', 'FCFE 上限', '协同')) {
    Assert-Contains $renderer $knowledgeToken "crash-course knowledge token: $knowledgeToken"
}
foreach ($visualToken in @('#2563EB', '#16A34A', '#EA8A00', '#EF4444', '#8B5CF6')) {
    Assert-Contains $renderer $visualToken "modern visual token: $visualToken"
}
Assert-Contains $renderer 'Course map text is unreadable at the 760px Index width' 'effective font-size rejection'
Assert-Contains $renderer 'Course map is too tall for one-view crash-course review' 'course-map aspect-ratio rejection'
Assert-Contains $renderer '$equivalentFontSize -lt 11.0' 'minimum 11px effective type threshold'
Assert-Contains $renderer '$aspectRatio -gt 1.40' 'maximum 1.40 aspect-ratio threshold'
Assert-Contains $renderer 'Obsidian Mermaid postprocessor selector mismatch' 'Obsidian postprocessor selector rejection'
Assert-Contains $renderer '"useMaxWidth": true' 'responsive Mermaid maximum-width setting'
if ($renderer.Contains('"useMaxWidth": false')) {
    throw 'C2B renderer disables responsive Mermaid sizing'
}
Assert-Contains $renderer 'Responsive Mermaid SVG root must have width="100%", viewBox, and controlled max-width' 'rendered SVG responsive-root rejection'
Assert-Contains $renderer "GetAttribute('width') -ne '100%'" 'rendered SVG percentage-width verification'
Assert-Contains $renderer "GetAttribute('viewBox')" 'rendered SVG viewBox verification'
Assert-Contains $renderer 'max-width:' 'rendered SVG max-width verification'
Assert-Contains $renderer 'png_default_collapsed=true' 'collapsed PNG renderer metric'
Assert-Contains $renderer "' internal-link '" 'Obsidian internal-link selector token'
Assert-Contains $renderer "'foreignObject'" 'Obsidian foreignObject selector token'
foreach ($domain in @('治理与目标', '资产配置与运营', '风险与价值评估', '融资与收益分配', '资本重组与增长')) {
    Assert-Contains $renderer $domain "MECE decision domain: $domain"
}
if ($renderer -match '(?m)^\s*\$lines\s*\+=\s*.*obsidian://|(?m)^\s*\$lines\s*\+=\s*.*\bclick\b|(?m)^\s*\$lines\s*\+=\s*.*-\.') {
    throw 'C2B renderer contains an external click URI or cross-branch edge'
}
if ($renderer.Contains('classDef knowledge-card') -or $renderer.Contains('centered_bilateral_three_layer')) {
    throw 'C2B renderer restores a forbidden knowledge-card or bilateral layout'
}

Assert-Matches $publisher '(?is)param\s*\(.*?\$PackageRoot' 'publisher PackageRoot input'
Assert-Contains $publisher 'publish hash mismatch' 'publisher hash verification'
Assert-Contains $publisher 'accepted_benchmark / registered C2B package' 'publisher formal C2B status rejection'
Assert-Contains $publisher 'formal C1B coverage: 37 decisions and 76 media derivatives' 'publisher formal C1B coverage rejection'
Assert-Contains $publisher 'accepted semantic-obsidian/v1 profile' 'publisher accepted template rejection'
Assert-Contains $publisher 'formal package manifest hash mismatch' 'publisher manifest-to-package hash rejection'
foreach ($forbidden in @('render-finance-course-map', '@mermaid-js/mermaid-cli', '财务管理课程脑图.mmd', '财务管理课程脑图.png')) {
    if ($publisher.Contains($forbidden)) {
        throw "C2B publisher contains forbidden render responsibility: $forbidden"
    }
}

Assert-Matches $registration "(?is)'process','register'.*'knowledge','ingest'.*'knowledge','review-suggestion'.*'knowledge','change-map-assignment'" 'formal knowledge registration sequence'
Assert-Contains $registration "status = 'registered'" 'registered universe ledger status'
Assert-Matches $c1bRegistration "(?is)'--kind','key_frame'.*'--model','finance-c1b-media-extractor'" 'formal C1B media registration sequence'
Assert-Matches $c1bRegistration "(?is)'--kind','structured_result'.*'--model','finance-c1b-essence-registrar'" 'formal C1B essence registration sequence'
Assert-Contains $c1bRegistration "status = 'registered'" 'registered C1B ledger status'
Assert-Contains $c1bRegistration 'complete_c1_reprocessed = 0' 'C1B complete-text reuse invariant'
Assert-Contains $documents.profile 'Profile status: accepted' 'accepted reusable Obsidian profile status'
Assert-Contains $closureVerification "tool_or_model IN ('finance-c1b-media-extractor','finance-c1b-essence-registrar')" 'closure C1B active-run verification'
Assert-Contains $closureVerification "tool_or_model='c1b-full-text-semantic-materializer'" 'closure C2B active-run verification'
Assert-Contains $closureVerification 'package/live hash failures' 'closure package/live hash verification'
Assert-Contains $closureVerification 'PRAGMA quick_check;' 'closure database integrity verification'
Assert-Contains $closureVerification "status='passed'" 'closure receipt status'
Assert-Contains $closureVerification 'live responsive map contract' 'closure responsive map verification'

$genericMba = Get-GenericMbaPathStatus
if ($genericMba.status -eq 'available') {
    Write-Output "Reusable generic MBA path: available; dedicated-tests=$($genericMba.tests.Count)"
}
else {
    Write-Output "Reusable generic MBA path: candidate/unavailable; reasons=$($genericMba.reasons -join ';')"
}
Write-Output 'Finance C2B reference validation: passed; formal C1B and universe ownership, accepted Obsidian profile, responsive package-owned right-growing Mermaid with collapsed PNG fallback, publisher-only export.'
