[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Join-Path ([IO.Path]::GetTempPath()) ('babata-mba-package-test-' + [Guid]::NewGuid().ToString('N'))
$vault = Join-Path $root 'Obsidian Vault'
$live = Join-Path $vault 'Babata\MBA\demo_c2b_latest'
$stage = Join-Path $root 'stage'
$package = Join-Path $stage 'package'
$manifestPath = Join-Path $stage 'manifest.json'
$planPath = Join-Path $root 'plan.json'
$sourceMapPath = Join-Path $root 'source-map.json'
$c1bLedgerPath = Join-Path $root 'c1b-ledger.json'
$knowledgeLedgerPath = Join-Path $root 'knowledge-ledger.json'
$learningManifestPath = Join-Path $root 'learning-docs-manifest.json'
$checker = Join-Path $PSScriptRoot 'check-mba-course-c2b-package.ps1'

function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Assert-Throws([scriptblock]$Action,[string]$Label) {
    $thrown = $false
    try { & $Action | Out-Null } catch { $thrown = $true }
    if (-not $thrown) { throw "Expected checker rejection did not occur: $Label" }
}

try {
    New-Item -ItemType Directory -Path (Join-Path $package 'media'),(Split-Path $live -Parent) -Force | Out-Null
    $mmd = @(
        '%%{init: {"flowchart": {"useMaxWidth": true}}}%%'
        'flowchart LR'
        '  root["Demo"]'
        '  domain["域"]'
        '  chapter["01-章节"]'
        '  learning["学习"]'
        '  root --- domain'
        '  domain --- chapter'
        '  root --- learning'
        '  learning --- tools'
        '  tools["工具"]'
        '  class chapter,tools internal-link'
    ) -join "`n"
    $index = @(
        '---'
        'course: Demo Course'
        'variant: c2b'
        'status: pending_user_acceptance'
        'formal_registration: registered'
        'c1b_registration: registered'
        'knowledge_universe_registration: registered'
        'template_profile: semantic-obsidian/v1'
        'template_status: accepted'
        '---'
        ''
        '# Demo'
        ''
        '## 课程脑图'
        ''
        '```mermaid'
        $mmd
        '```'
        ''
        '> [!info]- 位图回退'
        '> ![[media/map.png|760]]'
    ) -join "`n"
    Set-Content -LiteralPath (Join-Path $package 'index.md') -Value ($index + "`n") -Encoding utf8
    Set-Content -LiteralPath (Join-Path $package '01-章节.md') -Value "# 章节`n`n正文。`n" -Encoding utf8
    Set-Content -LiteralPath (Join-Path $package '工具.md') -Value "# 工具`n" -Encoding utf8
    Set-Content -LiteralPath (Join-Path $package 'media\map.mmd') -Value ($mmd + "`n") -Encoding utf8
    Add-Type -AssemblyName System.Drawing
    $bitmap = [Drawing.Bitmap]::new(2000,1600)
    try {
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([Drawing.Color]::White)
            $pen = [Drawing.Pens]::SteelBlue
            for ($x=0; $x -lt 2000; $x += 40) { $graphics.DrawLine($pen,$x,0,2000-$x,1600) }
        } finally { $graphics.Dispose() }
        $bitmap.Save((Join-Path $package 'media\map.png'),[Drawing.Imaging.ImageFormat]::Png)
    } finally { $bitmap.Dispose() }

    $plan = [ordered]@{
        schema='babata.mba-course-c2b-plan/v1'; course='Demo Course'; course_key='demo-course'; output_status='pending_user_acceptance'; expected_modules=1
        chapters=@([ordered]@{id='01';note='01-章节';title='章节';modules=@(1)})
        course_map=[ordered]@{
            classification_axis='demo'; domains=@([ordered]@{id='domain';label='域';color='#2563EB';evidence=@('正文');nodes=@([ordered]@{id='chapter';note='01-章节';details=@('正文')})})
            learning=[ordered]@{id='learning';label='学习';color='#64748B';nodes=@([ordered]@{id='tools';note='工具'})}
        }
        live=[ordered]@{path=$live;vault='Obsidian Vault';file='Babata/MBA/demo_c2b_latest/index.md'}
    }
    $plan | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $planPath -Encoding utf8
    $planHash = Hash $planPath
    $sourceMap=[ordered]@{schema='babata.mba.c2-source-map/v1';course='Demo Course';expected_modules=1;chunks=@([ordered]@{items=@([ordered]@{module_id=1})})}
    $sourceMap|ConvertTo-Json -Depth 10|Set-Content -LiteralPath $sourceMapPath -Encoding utf8
    $sourceMapHash=Hash $sourceMapPath
    $c1bLedger=[ordered]@{schema='babata.mba-course-c1b-registration/v1';status='registered';course='Demo Course';course_key='demo-course';course_plan_sha256=$planHash;source_map_sha256=$sourceMapHash;registrations=@([ordered]@{module_id=1})}
    $c1bLedger|ConvertTo-Json -Depth 10|Set-Content -LiteralPath $c1bLedgerPath -Encoding utf8
    $c1bHash=Hash $c1bLedgerPath
    $knowledgeLedger=[ordered]@{schema='babata.mba-course-c2b-knowledge-registration/v1';status='registered';course_acceptance='pending_user_acceptance';course='Demo Course';plan_sha256=$planHash;c1b_ledger_sha256=$c1bHash;source_map_sha256=$sourceMapHash;modules=@([ordered]@{module_id=1})}
    $knowledgeLedger|ConvertTo-Json -Depth 10|Set-Content -LiteralPath $knowledgeLedgerPath -Encoding utf8
    $learningManifest=[ordered]@{schema='babata.mba-course-learning-docs/v1';status='candidate';course='Demo Course';expected_modules=1;course_plan_sha256=$planHash;source_map_sha256=$sourceMapHash;source_notes=@([ordered]@{module_id=1})}
    $learningManifest|ConvertTo-Json -Depth 10|Set-Content -LiteralPath $learningManifestPath -Encoding utf8
    $rows = @(Get-ChildItem -LiteralPath $package -Recurse -File -Force | Sort-Object FullName | ForEach-Object {
        $rel = $_.FullName.Substring($package.Length).TrimStart('\').Replace('\','/')
        [ordered]@{path=$rel;sha256=(Hash $_.FullName);bytes=[long]$_.Length}
    })
    $manifest = [ordered]@{
        schema='babata.mba-course-c2b/v1';course='Demo Course';course_key='demo-course';status='pending_user_acceptance';course_plan_sha256=$planHash
        module_ids=@('1');source_map=$sourceMapPath;source_map_sha256=(Hash $sourceMapPath)
        c1b_ledger_sha256=(Hash $c1bLedgerPath);knowledge_ledger_sha256=(Hash $knowledgeLedgerPath)
        learning_docs_manifest=$learningManifestPath;learning_docs_manifest_sha256=(Hash $learningManifestPath)
        formal_registration='registered';c1b_registration=[ordered]@{status='registered';ledger=$c1bLedgerPath};knowledge_universe=[ordered]@{status='registered';ledger=$knowledgeLedgerPath}
        obsidian_template=[ordered]@{profile='semantic-obsidian/v1';status='accepted'}
        publication=[ordered]@{live_path=$live;vault='Obsidian Vault';file='Babata/MBA/demo_c2b_latest/index.md'}
        course_map=[ordered]@{mermaid='media/map.mmd';png='media/map.png';default_expanded='mermaid';responsive_svg=$true;png_default_collapsed=$true;png_display_width=760;png_width=2000;png_height=1600;aspect_ratio=0.8;effective_font_px=11;internal_link_targets=2}
        package_files=$rows
    }
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8

    $good = @(& $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath)
    if ($good.Count -ne 1 -or $good[0].status -ne 'passed') { throw 'Valid package was not accepted' }

    $manifest.package_files[0].bytes = [long]$manifest.package_files[0].bytes + 1
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'manifest bytes'
    $manifest.package_files[0].bytes = [long]$rows[0].bytes
    $savedSourceHash = [string]$manifest.source_map_sha256
    $manifest.source_map_sha256 = ('0' * 64)
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'tampered source-map hash'
    $manifest.source_map_sha256 = $savedSourceHash
    [void]$manifest.Remove('learning_docs_manifest_sha256')
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'missing learning-docs hash'
    $manifest.learning_docs_manifest_sha256 = Hash $learningManifestPath
    $manifest.module_ids=@('1','1')
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'duplicate manifest module id'
    $manifest.module_ids=@('1')
    $savedKnowledge=Get-Content -LiteralPath $knowledgeLedgerPath -Raw -Encoding utf8
    $knowledgeLedger.modules=@([ordered]@{module_id=1},[ordered]@{module_id=2})
    $knowledgeLedger|ConvertTo-Json -Depth 10|Set-Content -LiteralPath $knowledgeLedgerPath -Encoding utf8
    $manifest.knowledge_ledger_sha256=Hash $knowledgeLedgerPath
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'extra knowledge-ledger module id'
    Set-Content -LiteralPath $knowledgeLedgerPath -Value $savedKnowledge -Encoding utf8
    $manifest.knowledge_ledger_sha256=Hash $knowledgeLedgerPath
    $manifest.status = 'accepted_benchmark'
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'accepted_benchmark'
    $manifest.status = 'pending_user_acceptance'
    Set-Content -LiteralPath (Join-Path $package '01-章节.md') -Value "# 章节`n`n[悬空](missing.md)`n" -Encoding utf8
    $manifest.package_files = @(Get-ChildItem -LiteralPath $package -Recurse -File -Force | Sort-Object FullName | ForEach-Object {
        $rel = $_.FullName.Substring($package.Length).TrimStart('\').Replace('\','/')
        [ordered]@{path=$rel;sha256=(Hash $_.FullName);bytes=[long]$_.Length}
    })
    $manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Assert-Throws { & $checker -CoursePlanPath $planPath -PackageRoot $package -ManifestPath $manifestPath } 'dangling link'
    'mba-package-check-tests=passed'
}
finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
