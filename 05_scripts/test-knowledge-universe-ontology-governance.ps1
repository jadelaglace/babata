[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$checker = Join-Path $PSScriptRoot 'check-knowledge-universe-ontology-governance.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-ontology-governance-' + [Guid]::NewGuid().ToString('N'))
$requiredFiles = @(
    '00_docs\00_requirements\00_b_USER_WORDING.md',
    '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md',
    '00_docs\00_requirements\00_a_REQUIREMENTS.md',
    '00_docs\01_prd\01_a_PRD.md',
    '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md',
    '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md',
    '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md',
    '00_docs\04_process\04_b_USAGE_STATUS.md',
    '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md',
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
    param(
        [Parameter(Mandatory)][string]$CaseRoot,
        [Parameter(Mandatory)][string]$RelativePath,
        [Parameter(Mandatory)][string]$Value,
        [Parameter(Mandatory)][string]$Replacement
    )

    $path = Join-Path $CaseRoot $RelativePath
    $text = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $first = $text.IndexOf($Value, [StringComparison]::Ordinal)
    if ($first -lt 0) {
        throw "Mutation fixture is missing expected text in ${RelativePath}: $Value"
    }
    if ($text.IndexOf($Value, $first + $Value.Length, [StringComparison]::Ordinal) -ge 0) {
        throw "Mutation fixture text is not unique in ${RelativePath}: $Value"
    }
    $mutated = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Value.Length)
    Set-Content -LiteralPath $path -Value $mutated -Encoding utf8
}

function Assert-CheckerFails {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Expected,
        [Parameter(Mandatory)][scriptblock]$Mutate
    )

    $caseRoot = New-TestRepo $Name
    & $Mutate $caseRoot
    $failed = $false
    try {
        & $checker -RepoRoot $caseRoot | Out-Null
    }
    catch {
        $failed = $true
        if (-not $_.Exception.Message.Contains($Expected)) {
            throw "Case '$Name' failed for the wrong reason: $($_.Exception.Message)"
        }
    }
    if (-not $failed) {
        throw "Case '$Name' unexpectedly passed knowledge-universe ontology governance."
    }
}

try {
    New-Item -ItemType Directory -Path $root | Out-Null
    & $checker -RepoRoot $repo | Out-Null

    Assert-CheckerFails 'adopted-analysis-loses-attribution' 'adopted-analysis recovery attribution boundary' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_c_USER_WORDING_RECOVERY.md' '以下是被用户整体采用的 Agent 分析摘要，**不是用户逐字原话**' '以下是用户逐字确认的完整分析'
    }
    Assert-CheckerFails 'requirements-makes-foundations-exhaustive' 'non-exhaustive foundation semantics in requirements' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md' '也不以未定义的 MECE/穷尽性强迫未知内容归类' '并要求以 MECE 穷尽所有未知内容'
    }
    Assert-CheckerFails 'prd-loses-overlap' 'overlapping and non-exhaustive foundations in PRD' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\01_prd\01_a_PRD.md' '可重叠的世界观观察维度，不是要求对象单选或强行穷尽的文件夹' '互斥且必须穷尽的单选文件夹'
    }
    Assert-CheckerFails 'acceptance-allows-forced-classification' 'unknown-safe foundation acceptance' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\02_acceptance\02_a_ACCEPTANCE_CRITERIA.md' '系统不强迫四选一、不把“未知”伪装成已归类' '系统允许把未知自动归入默认基石'
    }
    Assert-CheckerFails 'architecture-loses-zero-or-many' 'zero-or-many foundation architecture' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md' '对象可以关联零个、一个或多个基石' '对象必须且只能关联一个基石'
    }
    Assert-CheckerFails 'requirements-merges-course-and-branch' 'Course/Branch identity separation in requirements' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md' '稳定专业分支与具体课程不是同一身份' '稳定专业分支与具体课程共用同一身份'
    }
    Assert-CheckerFails 'prd-loses-many-to-many-covers' 'many-to-many covers behavior in PRD' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\01_prd\01_a_PRD.md' '通过 `covers` 多对多覆盖稳定 Branch' '通过目录名映射唯一分支'
    }
    Assert-CheckerFails 'architecture-restores-undefined-topic' 'undefined Topic covers target in architecture' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md') -Value "`nCourse may also cover branch/topic.`n" -Encoding utf8
    }
    Assert-CheckerFails 'architecture-turns-mba-into-discipline' 'MBA non-discipline boundary' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md' '不作为 discipline' '作为唯一 discipline'
    }
    Assert-CheckerFails 'requirements-forces-100-percent' 'independent foundation intensity requirement' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\00_requirements\00_a_REQUIREMENTS.md' '不要求相加为 `100%`，判断置信度必须另记' '必须相加为 `100%`，并用强度代替置信度'
    }
    Assert-CheckerFails 'profile-promotes-display-domain' 'display-domain/Branch separation in profile' {
        param($caseRoot)
        Replace-Once $caseRoot '02_skills\00_specs\templates\semantic-obsidian-profile.md' 'These display domains are not ontology `Branch`' 'These display domains create ontology `Branch`'
    }
    Assert-CheckerFails 'outputs-loses-successor-implementation' 'successor contract implementation status' {
        param($caseRoot)
        Replace-Once $caseRoot '02_skills\00_specs\09_outputs.md' 'Successor MBA universe-registration contract (adopted; core writer and register/show CLI implemented,' 'Successor MBA universe-registration contract (adopted; implementation status unspecified,'
    }
    Assert-CheckerFails 'architecture-omits-typed-map-relation-semantics' 'typed relation semantics in architecture' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md' '`intersects_with`、`draws_from`、`applies_to`、`prerequisite_of` 等跨学科语义进入类型化关系' '跨学科语义进入未定义关系'
    }
    Assert-CheckerFails 'architecture-reintroduces-current-status' 'competing current implementation claim in architecture' {
        param($caseRoot)
        Add-Content -LiteralPath (Join-Path $caseRoot '00_docs\03_architecture\03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md') -Value "`n当前已支持全部后继语义。`n" -Encoding utf8
    }
    Assert-CheckerFails 'rollout-loses-stable-registrar-contract' 'stable successor registrar rollout contract' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_c_MBA_C2B_ROLLOUT.md' '任何课程 registrar 合同都必须支持本节的 course/branch 分离、typed `covers`、多重 assignment' '课程 registrar 可以沿用单一路径归属'
    }
    Assert-CheckerFails 'process-promotes-current-p6-conformance' 'versioned P6 baseline closure' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_a_DEVELOPMENT_PROCESS.md' '历史 baseline 退出' '退出'
    }
    Assert-CheckerFails 'usage-promotes-p6-successor' 'honest P6 historical status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md' '| P6 | 历史 baseline 已完成 |' '| P6 | 已完成 |'
    }
    Assert-CheckerFails 'usage-loses-finance-successor' 'finance successor registration status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md' '财务管理的 successor' '财务管理的兼容记录'
    }
    Assert-CheckerFails 'usage-loses-supply-chain-successor' 'supply-chain successor registration status' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\04_process\04_b_USAGE_STATUS.md' '全球供应链的 successor' '全球供应链的兼容记录'
    }
    Assert-CheckerFails 'outputs-reenable-v1' 'new-registration prohibition' {
        param($caseRoot)
        Replace-Once $caseRoot '02_skills\00_specs\09_outputs.md' 'registration is not conformant for new formal registrations' 'registration is conformant for new formal registrations'
    }
    Assert-CheckerFails 'tests-narrow-historical-protection' 'historical acceptance dimensions in TC' {
        param($caseRoot)
        Replace-Once $caseRoot '00_docs\05_tests\05_a_TEST_CASES.md' '既有 C1B、内容、媒体、profile、package/live、用户验收和' '既有内容和'
    }
    Assert-CheckerFails 'legacy-registrar-loses-exact-parent-fingerprint' 'exact-parent behavior in historical registrar' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\register-mba-course-c2b-knowledge.ps1' 'function Assert-ExactParent' 'function Assert-LegacyParent'
    }
    Assert-CheckerFails 'legacy-registrar-loses-unassign-fingerprint' 'unassign behavior in historical registrar' {
        param($caseRoot)
        Replace-Once $caseRoot '05_scripts\register-mba-course-c2b-knowledge.ps1' "'--change','unassign'" "'--change','retain'"
    }

    Write-Output 'Knowledge-universe ontology governance mutation tests passed.'
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
