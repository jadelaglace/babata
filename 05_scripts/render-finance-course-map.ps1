[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PackageRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$package = (Get-Item -LiteralPath $PackageRoot).FullName
$indexPath = Join-Path $package 'index.md'
if (-not (Test-Path -LiteralPath $indexPath -PathType Leaf)) {
    throw "Missing C2B index: $indexPath"
}

$knowledgeGroups = @(
    @{ id='governance'; label='一、治理与目标'; color='#2563EB'; evidence=@('股东财富最大化','投资决策','融资决策','股利分配','法律','道德','环境'); nodes=@(
        @{ id='target'; note='01-财务管理目标与财务高管职责'; details=@(
            '目标：股东财富最大化',
            '抓手：投资 · 融资 · 分配',
            '判断：现金流/风险调整 > 会计利润',
            '边界：法律 · 道德 · 环境 · 利益相关者'
        ) }
    ) },
    @{ id='assets'; label='二、资产配置与运营'; color='#16A34A'; evidence=@('营运资本','流动性','盈利能力','EOQ','相关现金流','增量','税后现金流','NPV > 0'); nodes=@(
        @{ id='working'; note='02-营运资本管理'; details=@(
            '营运：流动性 ↔ 盈利；长配长/短配短',
            '存货：EOQ = √(2CoD / Ch)'
        ) },
        @{ id='invest'; note='03-项目投资评估'; details=@(
            '项目：只计增量税后现金流',
            '规则：NPV > 0 接受'
        ) }
    ) },
    @{ id='valuation'; label='三、风险与价值评估'; color='#EA8A00'; evidence=@('敏感性分析','概率分析','模拟法','风险调整折现率','FCF','终值','FCFF','WACC','FCFE','Ke'); nodes=@(
        @{ id='risk'; note='04-投资风险与不确定性'; details=@(
            '风险：敏感性 → 概率/情景 → 模拟',
            '匹配：高风险 → 高要求回报/折现率'
        ) },
        @{ id='value'; note='05-企业估值'; details=@(
            '估值：FCF 现值 + 终值现值',
            '口径：FCFF/WACC|BR|　　　FCFE/Ke'
        ) }
    ) },
    @{ id='funding'; label='四、融资与收益分配'; color='#EF4444'; evidence=@('WACC','税盾','财务困境','优序理论','FCFE','信号'); nodes=@(
        @{ id='capital'; note='06-融资决策与资本结构'; details=@(
            'WACC｜Ke·E/V +|BR|　　　Kd·(1−T)·D/V',
            '杠杆：税盾收益 ↔ 财务困境成本',
            '优序：留存 > 债务 > 权益'
        ) },
        @{ id='dividend'; note='07-股利决策'; details=@(
            '股利：FCFE 上限；稳定政策传递信号'
        ) }
    ) },
    @{ id='restructure'; label='五、资本重组与增长'; color='#8B5CF6'; evidence=@('合并后公司的市值','协同效应','现金','换股','融资','溢价','整合成本','财务困境'); nodes=@(
        @{ id='mna'; note='08-并购与企业重组'; details=@(
            '前提：合并价值 > 独立价值之和',
            '协同：收入 + 成本 + 财务协同',
            '交易：估值 · 现金/换股 · 融资',
            '兑现：溢价 · 整合 · 困境重组'
        ) }
    ) }
)
$learning = @{ id='learning'; label='学习支持'; color='#64748B'; nodes=@(
    @{ id='tools'; note='09-公式与决策工具' },
    @{ id='cases'; note='10-案例练习' },
    @{ id='review'; note='11-复习与自测' },
    @{ id='evidence'; note='视觉证据索引' }
) }
$knowledgeNodes = @($knowledgeGroups | ForEach-Object { @($_.nodes) } | ForEach-Object { $_ })
$linkedNodes = @($knowledgeNodes) + @($learning.nodes)
$detailCount = @($knowledgeNodes | ForEach-Object { @($_.details) }).Count

if ($knowledgeGroups.Count -ne 5 -or $knowledgeNodes.Count -ne 8 -or @($learning.nodes).Count -ne 4 -or $detailCount -ne 20) {
    throw 'Right-growing course map must contain five decision domains, eight chapters, four learning aids, and twenty grounded details'
}
$duplicateNotes = @($linkedNodes | Group-Object note | Where-Object Count -ne 1)
if ($duplicateNotes.Count -ne 0) {
    throw "Course-map linked notes must be unique: $($duplicateNotes.Name -join ', ')"
}
foreach ($group in $knowledgeGroups) {
    $groundingText = @($group.nodes | ForEach-Object {
        $notePath = Join-Path $package ($_.note + '.md')
        if (-not (Test-Path -LiteralPath $notePath -PathType Leaf)) {
            throw "Crash-course grounding note is missing: $($_.note)"
        }
        Get-Content -LiteralPath $notePath -Raw -Encoding utf8
    }) -join "`n"
    foreach ($evidenceToken in @($group.evidence)) {
        if (-not $groundingText.Contains([string]$evidenceToken)) {
            throw "Crash-course knowledge detail is not grounded in accepted C2B body: $($group.label) / $evidenceToken"
        }
    }
}

$lines = [System.Collections.Generic.List[string]]::new()
@(
    '%% MECE 主分类轴：财务决策对象；学习支持独立成层'
    '%% C2B-RIGHT-GROWING-MINDMAP-GATE：单根、全右向、五域分色、透明文字节点、小圆分叉、无知识卡片'
    '%% C2B-RESPONSIVE-MAP-GATE：原生 Mermaid 为唯一默认展开主图；PNG 仅作折叠回退'
    '%% 四级知识展开：课程根 --- 决策域 --- 可点击章节 --- 分叉圆点 --- 正文有据知识细节'
    '%%{init: {"flowchart": {"useMaxWidth": true, "htmlLabels": true, "nodeSpacing": 16, "rankSpacing": 34, "wrappingWidth": 300, "curve": "basis"}, "themeVariables": {"fontFamily": "Inter, Segoe UI, Microsoft YaHei, sans-serif", "fontSize": "22px", "lineColor": "#94A3B8"}}}%%'
    'flowchart LR'
    '  finance["<b>财务管理</b><br/><small>价值创造 · 评估 · 分配 · 维护</small>"]'
) | ForEach-Object { [void]$lines.Add($_) }

foreach ($group in $knowledgeGroups) {
    [void]$lines.Add("  $($group.id)[`"<b>$($group.label)</b>&nbsp;<span style='color:$($group.color)'>&#9675;</span>`"]")
    foreach ($node in @($group.nodes)) {
        [void]$lines.Add("  $($node.id)[`"$($node.note)`"]")
        [void]$lines.Add("  $($node.id)J(( ))")
        for ($i = 0; $i -lt @($node.details).Count; $i++) {
            $detail = ([string]$node.details[$i]).Replace('&', '&amp;').Replace('<', '&lt;').Replace('>', '&gt;').Replace('|BR|', '<br/>')
            [void]$lines.Add("  $($node.id)D$($i + 1)[`"$detail`"]")
        }
    }
}
[void]$lines.Add("  $($learning.id)[`"<b>$($learning.label)</b>&nbsp;<span style='color:$($learning.color)'>&#9675;</span>`"]")
foreach ($node in @($learning.nodes)) {
    [void]$lines.Add("  $($node.id)[`"$($node.note)`"]")
}

$edgeLines = [System.Collections.Generic.List[string]]::new()
$edgeGroups = @{}
$edgeIndex = 0
foreach ($group in $knowledgeGroups) {
    $indices = [System.Collections.Generic.List[int]]::new()
    [void]$edgeLines.Add("  finance --- $($group.id)"); [void]$indices.Add($edgeIndex); $edgeIndex++
    foreach ($node in @($group.nodes)) {
        [void]$edgeLines.Add("  $($group.id) --- $($node.id)"); [void]$indices.Add($edgeIndex); $edgeIndex++
        [void]$edgeLines.Add("  $($node.id) --- $($node.id)J"); [void]$indices.Add($edgeIndex); $edgeIndex++
        for ($i = 0; $i -lt @($node.details).Count; $i++) {
            [void]$edgeLines.Add("  $($node.id)J --- $($node.id)D$($i + 1)"); [void]$indices.Add($edgeIndex); $edgeIndex++
        }
    }
    $edgeGroups[$group.id] = @($indices)
}
$learningIndices = [System.Collections.Generic.List[int]]::new()
[void]$edgeLines.Add("  finance --- $($learning.id)"); [void]$learningIndices.Add($edgeIndex); $edgeIndex++
foreach ($node in @($learning.nodes)) {
    [void]$edgeLines.Add("  $($learning.id) --- $($node.id)"); [void]$learningIndices.Add($edgeIndex); $edgeIndex++
}
$edgeGroups[$learning.id] = @($learningIndices)
$edgeLines | ForEach-Object { [void]$lines.Add($_) }

@(
    '  classDef root fill:transparent,color:#0F172A,stroke:transparent,stroke-width:0px,font-size:24px,font-weight:700,text-align:left'
    '  classDef branch fill:transparent,color:#111827,stroke:transparent,stroke-width:0px,font-size:22px,font-weight:700,text-align:left'
    '  classDef internal-link fill:transparent,color:#111827,stroke:transparent,stroke-width:0px,font-size:21px,font-weight:600,text-align:left'
    '  classDef detail fill:transparent,color:#273244,stroke:transparent,stroke-width:0px,font-size:20px,font-weight:400,text-align:left'
    '  classDef junction fill:#FFFFFF,color:transparent,stroke-width:2px,padding:2px'
    '  class finance root'
    '  class governance,assets,valuation,funding,restructure,learning branch'
    '  class target,working,invest,risk,value,capital,dividend,mna,tools,cases,review,evidence internal-link'
) | ForEach-Object { [void]$lines.Add($_) }
$detailIds = @($knowledgeNodes | ForEach-Object {
    $node = $_
    for ($i = 0; $i -lt @($node.details).Count; $i++) { "$($node.id)D$($i + 1)" }
})
[void]$lines.Add('  class ' + ($detailIds -join ',') + ' detail')
foreach ($group in $knowledgeGroups) {
    $junctions = @($group.nodes | ForEach-Object { "$($_.id)J" })
    [void]$lines.Add("  class $($junctions -join ',') junction")
    foreach ($junction in $junctions) {
        [void]$lines.Add("  style $junction fill:#FFFFFF,stroke:$($group.color),stroke-width:2px")
    }
    [void]$lines.Add("  linkStyle $($edgeGroups[$group.id] -join ',') stroke:$($group.color),stroke-width:2px")
}
[void]$lines.Add("  linkStyle $($edgeGroups[$learning.id] -join ',') stroke:$($learning.color),stroke-width:1.75px")
$mermaid = $lines -join "`n"

if ($mermaid.Contains('obsidian://') -or $mermaid -match '(?m)^\s*click\s+' -or $mermaid.Contains('-.') -or
    $mermaid.Contains('-->') -or $mermaid.Contains('<--') -or $mermaid.Contains('knowledge-card') -or
    $mermaid.Contains('centered_bilateral')) {
    throw 'Right-growing course map contains a forbidden URI, cross-branch edge, arrow, card, or bilateral layout'
}
if ([regex]::Matches($mermaid, '(?m)^\s*finance\[').Count -ne 1 -or
    [regex]::Matches($mermaid, '(?m)^\s*finance --- ').Count -ne 6) {
    throw 'Right-growing course map must have exactly one root and six right-growing first-level branches'
}

$mediaRoot = Join-Path $package 'media'
New-Item -ItemType Directory -Path $mediaRoot -Force | Out-Null
$mmdPath = Join-Path $mediaRoot '财务管理课程脑图.mmd'
$pngPath = Join-Path $mediaRoot '财务管理课程脑图.png'
$svgCheckPath = Join-Path $mediaRoot '.财务管理课程脑图-link-check.svg'
Set-Content -LiteralPath $mmdPath -Value ($mermaid + "`n") -Encoding utf8

& npx --yes '@mermaid-js/mermaid-cli@11.12.0' -i $mmdPath -o $pngPath -b white -w 2200 -H 1600
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $pngPath -PathType Leaf)) {
    throw 'Mermaid PNG rendering failed'
}
& npx --yes '@mermaid-js/mermaid-cli@11.12.0' -i $mmdPath -o $svgCheckPath -b white -w 2200 -H 1600
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $svgCheckPath -PathType Leaf)) {
    throw 'Mermaid SVG link-check rendering failed'
}
[xml]$svgCheck = Get-Content -LiteralPath $svgCheckPath -Raw -Encoding utf8
$svgRoot = $svgCheck.DocumentElement
if ($null -eq $svgRoot -or $svgRoot.LocalName -ne 'svg' -or
    [string]$svgRoot.GetAttribute('width') -ne '100%' -or
    [string]::IsNullOrWhiteSpace([string]$svgRoot.GetAttribute('viewBox')) -or
    [string]$svgRoot.GetAttribute('style') -notmatch '(?:^|;)\s*max-width:\s*[0-9]+(?:\.[0-9]+)?px(?:;|$)') {
    throw 'Responsive Mermaid SVG root must have width="100%", viewBox, and controlled max-width'
}
$selectorXPath = "//*[contains(concat(' ', normalize-space(@class), ' '), ' internal-link ')]/*[local-name()='g' and contains(concat(' ', normalize-space(@class), ' '), ' label ')]/*[local-name()='foreignObject']/*[local-name()='div']"
$obsidianLinkNodes = @($svgCheck.SelectNodes($selectorXPath))
$actualLinkLabels = @($obsidianLinkNodes | ForEach-Object { $_.InnerText })
$expectedLinkLabels = @($linkedNodes | ForEach-Object { [string]$_.note })
$actualSorted = @($actualLinkLabels | Sort-Object -Unique)
$expectedSorted = @($expectedLinkLabels | Sort-Object -Unique)
if ($actualLinkLabels.Count -ne $expectedLinkLabels.Count -or
    $actualSorted.Count -ne $actualLinkLabels.Count -or
    ($actualSorted -join "`n") -ne ($expectedSorted -join "`n")) {
    throw "Obsidian Mermaid postprocessor selector mismatch: expected $($expectedLinkLabels.Count), got $($actualLinkLabels.Count)"
}
Remove-Item -LiteralPath $svgCheckPath -Force

$index = Get-Content -LiteralPath $indexPath -Raw -Encoding utf8
$block = @(
    '## 课程脑图'
    ''
    '按财务决策对象向右展开。彩色曲线区分知识域；章节节点可下钻完整笔记，末端短句用于紧急复习。'
    ''
    '```mermaid'
    $mermaid
    '```'
    ''
    '> [!info]- 位图版本（打印 / 离线 / 渲染回退）'
    '> ![[media/财务管理课程脑图.png|760]]'
) -join "`n"
if ($index -match '(?ms)^## 课程脑图\s*.*?(?=^## |\z)') {
    $index = [regex]::Replace($index, '(?ms)^## 课程脑图\s*.*?(?=^## |\z)', ($block + "`n`n"), 1)
} else {
    $insertBefore = '(?m)^## 课程章节\s*$'
    if ($index -notmatch $insertBefore) { throw 'C2B index has no course chapter section' }
    $index = [regex]::Replace($index, $insertBefore, ($block + "`n`n## 课程章节"), 1)
}
Set-Content -LiteralPath $indexPath -Value ($index.TrimEnd() + "`n") -Encoding utf8

$rendered = Get-Item -LiteralPath $pngPath
if ($rendered.Length -lt 10000) { throw "Rendered course map is unexpectedly small: $($rendered.Length) bytes" }
Add-Type -AssemblyName System.Drawing
$pngImage = [System.Drawing.Image]::FromFile($pngPath)
try {
    $pngWidth = [double]$pngImage.Width
    $pngHeight = [double]$pngImage.Height
}
finally {
    $pngImage.Dispose()
}
$embedWidth = 760.0
$baseFontSize = 22.0
$equivalentFontSize = $baseFontSize * $embedWidth / $pngWidth
$aspectRatio = $pngHeight / $pngWidth
if ($equivalentFontSize -lt 11.0) {
    throw "Course map text is unreadable at the 760px Index width: equivalent_font_px=$([Math]::Round($equivalentFontSize, 2))"
}
if ($aspectRatio -gt 1.40) {
    throw "Course map is too tall for one-view crash-course review: aspect_ratio=$([Math]::Round($aspectRatio, 3))"
}
$indexCheck = Get-Content -LiteralPath $indexPath -Raw -Encoding utf8
if ([regex]::Matches($indexCheck, '(?m)^```mermaid$').Count -ne 1 -or
    [regex]::Matches($indexCheck, '(?m)^```$').Count -lt 1 -or
    $indexCheck.Contains('.Trim()')) {
    throw 'Mermaid Markdown fence verification failed'
}
foreach ($node in $linkedNodes) {
    if (-not (Test-Path -LiteralPath (Join-Path $package ($node.note + '.md')) -PathType Leaf)) {
        throw "Mermaid internal-link target is missing: $($node.note)"
    }
    if ([regex]::Matches($mermaid, [regex]::Escape('["' + $node.note + '"]')).Count -ne 1) {
        throw "Mermaid internal-link target must appear exactly once: $($node.note)"
    }
}
$disabledMaxWidth = '"useMaxWidth": ' + 'false'
if ($mermaid.Contains($disabledMaxWidth) -or -not $mermaid.Contains('"useMaxWidth": true')) {
    throw 'Responsive Mermaid must enable useMaxWidth and reject false'
}
if ([regex]::Matches($mermaid, '(?m)^\s*class\s+.*\binternal-link\s*$').Count -ne 1 -or
    [regex]::Matches($indexCheck, [regex]::Escape('![[media/财务管理课程脑图.png|760]]')).Count -ne 1 -or
    $indexCheck -notmatch '(?m)^> \[!info\]- 位图版本（打印 / 离线 / 渲染回退）\r?\n> !\[\[media/财务管理课程脑图\.png\|760\]\]$') {
    throw 'Obsidian internal-link class or default-collapsed PNG fallback is missing'
}

Write-Output "mmd=$mmdPath png=$pngPath internal_links=$($linkedNodes.Count) obsidian_selector_matches=$($actualLinkLabels.Count) responsive_svg=true png_default_collapsed=true mece_domains=$($knowledgeGroups.Count) knowledge_details=$detailCount layout=single_root_right_growing width=760 png_dimensions=$([int]$pngWidth)x$([int]$pngHeight) equivalent_font_px=$([Math]::Round($equivalentFontSize,2)) aspect_ratio=$([Math]::Round($aspectRatio,3)) bytes=$($rendered.Length)"
