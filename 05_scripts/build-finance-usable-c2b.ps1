[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceMapPath,

    [Parameter(Mandatory = $true)]
    [string]$QianwenTextScript,

    [Parameter(Mandatory = $true)]
    [string]$StagingRoot,

    [Parameter(Mandatory = $true)]
    [string]$LiveVaultPath,

    [string]$Model = 'qwen3.6-plus',
    [string]$MediaSourcePath,
    [switch]$Publish,
    [switch]$Resume,
    [switch]$AllowExistingStaging
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-RelativePath([string]$Base, [string]$Path) {
    $baseUri = [Uri]((Get-Item -LiteralPath $Base).FullName.TrimEnd('\') + '\')
    $pathUri = [Uri](Get-Item -LiteralPath $Path).FullName
    return [Uri]::UnescapeDataString($baseUri.MakeRelativeUri($pathUri).ToString()).Replace('\', '/')
}

function Remove-LeadingTitle([string]$Text) {
    $body = $Text.Trim()
    if ($body -match '(?m)^# .+\r?\n') {
        return ([regex]::Replace($body, '(?m)^# .+\r?\n', '', 1)).Trim()
    }
    return $body
}

function Get-ResponseText([string]$ResponsePath) {
    $response = Get-Content -LiteralPath $ResponsePath -Raw | ConvertFrom-Json
    if (-not $response.choices -or -not $response.choices[0].message.content) {
        throw "QianWen response contains no message content: $ResponsePath"
    }
    $text = [string]$response.choices[0].message.content
    $text = [regex]::Replace($text.Trim(), '\A```(?:markdown)?\s*', '')
    $text = [regex]::Replace($text, '\s*```\z', '')
    return $text.Trim()
}

function Replace-EditorialText([string]$Path, [string]$Before, [string]$After) {
    $text = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    if ($text.Contains($Before)) {
        $text = $text.Replace($Before, $After)
        Set-Content -LiteralPath $Path -Value $text -Encoding utf8
    }
}

function Replace-EditorialRegex([string]$Path, [string]$Pattern, [string]$After) {
    $text = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    $replacement = [System.Text.RegularExpressions.MatchEvaluator]{ param($match) $After }
    $updated = [regex]::Replace($text, $Pattern, $replacement)
    if ($updated -ne $text) {
        Set-Content -LiteralPath $Path -Value $updated -Encoding utf8
    }
}

function Invoke-EditorialCorrections([string]$Root) {
    $chapter1 = Join-Path $Root '01-财务管理目标与财务高管职责.md'
    Replace-EditorialText $chapter1 '亚马逊曾连续多年账面亏损，但其经营现金流极佳，支撑了其庞大的扩张和技术投入，最终使其成为全球最有价值的企业之一，创始人贝索斯多次登顶富豪榜。这证明了**现金流**对于企业生存和价值创造的重要性远超短期会计利润。' '课程以亚马逊早期多年账面亏损但现金流较好为例，说明会计利润不能单独解释企业的生存能力与市场价值；案例的教学重点是区分利润和现金流，而不是据此断言现金流在所有情境下都优先于其他指标。'
    Replace-EditorialText $chapter1 '若企业希望吸引养老基金作为战略投资者，财务高管应制定“稳定增长”的股利政策（如每年2%恒定增长）。因为养老基金需要可预测的现金流来支付退休金，厌恶波动。这种针对性的股利策略有助于优化投资者结构，稳定股价。' '课程用养老基金偏好稳定现金流说明“客户效应”：企业若希望吸引这类投资者，可考虑稳定或恒定增长的股利政策。课堂中的每年 2% 是说明 constant growth 的示例参数，不是适用于所有企业的建议值；实际比例仍受盈利、现金流、投资机会和法律约束。'

    $risk = Join-Path $Root '04-投资风险与不确定性.md'
    Replace-EditorialText $risk '单价 $10，' '单价 \$10，'
    Replace-EditorialRegex $risk '(?s)亏损概率 = Prob\(组合4\) \+ Prob\(组合6\) = \$0\.1500 \+ 0\.0625 = 0\.2125\$ \(21\.25%\)。\r?\n\*\(注：原文视频讲解中提到亏损概率为 6\.25%.*?\)\*' '按课件表格逐项扣除 12,000 初始投资后，组合 4 的 NPV 为 -2,000，组合 6 的 NPV 为 -18,000，因此亏损概率为 $0.1500 + 0.0625 = 0.2125$（21.25%）。视频转写讲解给出 6.25%，只计入了组合 6；该数值与课件表格不一致。ENPV 仍为 11,100。复习和作答时应明确采用哪一口径，并优先展示从表格逐项计算的过程。[[来源/M-892153]][[来源/M-892155]]'

    $valuation = Join-Path $Root '05-企业估值.md'
    Replace-EditorialText $valuation '被视为最先进且理论上最严谨的方法，尤其适用于评估长期价值和协同效应' '直接体现“价值等于未来现金流现值”的逻辑，适用于评估长期价值和协同效应，但结果高度依赖现金流、折现率和增长率假设'
    Replace-EditorialText $valuation '这是目前投行和学术界公认较先进的估值方法' '这种方法把估值落在未来自由现金流及其风险上，但对预测假设十分敏感'
    Replace-EditorialText $valuation '股价$4，面值$0.5，股本$20m，最新税后利润$10.1m，最新股利$6.0m' '股价 \$4，面值 \$0.5，股本 USD 20m，最新税后利润 USD 10.1m，最新股利 USD 6.0m'

    $cases = Join-Path $Root '10-案例练习.md'
    Replace-EditorialText $cases '        $$ \$10,000,000 \times 20\% \times \frac{10}{365} \approx \$547,945 $$' '        $$ \$10,000,000 \times 20\% \times \frac{10}{365} \approx \$54,795 $$'
    Replace-EditorialRegex $cases '(?s)    \*   \*\*新平均应收总额\*\*：\$\\\$547,945 \+ \\\$1,972,603 = \\\$2,520,548\$\r?\n    \*\(注：此处依据课件逻辑.*?原文指出新应收账款总额为 \\\$2,027,398。' '    *   **新平均应收总额**：$\$54,795 + \$1,972,603 = \$2,027,398$。这一结果与课件答案一致。'
    Replace-EditorialText $cases '净收益为正（\$3,835）' '净收益为正（精确值 \$3,835.5，按课件取整为 \$3,836）'
    Replace-EditorialText $cases '特斯拉的案例完美诠释了生命周期对财务决策的影响。' '课程用特斯拉案例说明企业生命周期会改变融资和分配决策的约束。'
    Replace-EditorialText $cases '股价：$4/股；面值：$0.5/股；股本总额：$20m。' '股价：\$4/股；面值：\$0.5/股；股本总额：USD 20m。'
    Replace-EditorialText $cases '资产数据：非流动资产$86m，存货$4.2m，应收账款$4.5m（预计变现率80%），流动负债$7.1m，非流动负债$25m。' '资产数据：非流动资产 USD 86m，存货 USD 4.2m，应收账款 USD 4.5m（预计变现率 80%），流动负债 USD 7.1m，非流动负债 USD 25m。'
    Replace-EditorialText $cases '**NRV ($61.7m)**' '**NRV（USD 61.7m）**'
    Replace-EditorialText $cases '**市值 ($160m)**' '**市值（USD 160m）**'
    Replace-EditorialText $cases '**P/E 法 ($171.7m)**' '**P/E 法（USD 171.7m）**'
    Replace-EditorialText $cases '$61.7m（清算底线）至 $171.7m（盈利预期上限）' 'USD 61.7m（清算底线）至 USD 171.7m（盈利预期上限）'
    Replace-EditorialText $cases '约 $234m' '约 USD 234m'
    Replace-EditorialText $cases 'EPS $0.25' 'EPS 为 \$0.25'
    Replace-EditorialText $cases '投资成本 $1.25/股' '投资成本 \$1.25/股'
    Replace-EditorialText $cases '对应收益 $5 万' '对应收益 5 万美元'

    $review = Join-Path $Root '11-复习与自测.md'
    Replace-EditorialText $review '二是**互斥项目冲突，当项目规模或期限不同时' '二是**互斥项目冲突**：当项目规模或期限不同时'
    Replace-EditorialText $review '年销售额为$10m' '年销售额为 USD 10m'

    $dividend = Join-Path $Root '07-股利决策.md'
    Replace-EditorialText $dividend '单位：$’000' '单位：USD ’000'
}

function Test-MarkdownStructure([string]$Root) {
    $errors = @()
    foreach ($file in Get-ChildItem -LiteralPath $Root -File -Filter '*.md') {
        $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8
        if ([regex]::Matches($text, '(?m)^# ').Count -ne 1) { $errors += "$($file.Name): expected exactly one H1" }
        if ([regex]::Matches($text, '```').Count % 2 -ne 0) { $errors += "$($file.Name): unclosed code fence" }
        $lineNo = 0
        foreach ($line in ($text -split "`r?`n")) {
            $lineNo++
            if ([regex]::Matches($line, '\*\*').Count % 2 -ne 0) { $errors += "$($file.Name):$lineNo unbalanced bold" }
            if ([regex]::Matches($line, '(?<!\\)\$').Count % 2 -ne 0) { $errors += "$($file.Name):$lineNo ambiguous dollar/math marker" }
        }
        foreach ($term in @('让我们复核原文逻辑','作为AI','完美诠释了','全球最有价值的企业之一','公认较先进')) {
            if ($text.Contains($term)) { $errors += "$($file.Name): editorial residue '$term'" }
        }
    }
    if ($errors.Count -gt 0) { throw "Markdown structure/editorial QA failed: $($errors -join '; ')" }
}

function Invoke-QianwenMarkdown(
    [string]$Id,
    [string]$SystemPrompt,
    [string]$UserPrompt,
    [string[]]$RequiredModuleIds,
    [int]$MinimumChars,
    [string[]]$RequiredHeadings,
    [int]$MinimumCitedModules = 0
) {
    $requestDir = Join-Path $requestsRoot $Id
    $responseDir = Join-Path $responsesRoot $Id
    New-Item -ItemType Directory -Path $requestDir, $responseDir -Force | Out-Null
    $request = [ordered]@{
        model = $Model
        enable_thinking = $false
        temperature = 0.15
        max_tokens = 12000
        messages = @(
            [ordered]@{ role = 'system'; content = $SystemPrompt }
            [ordered]@{ role = 'user'; content = $UserPrompt }
        )
    }
    $requestPath = Join-Path $requestDir 'request.json'
    $request | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $requestPath -Encoding utf8
    $responsePath = Join-Path $responseDir 'response.json'
    if (-not ($Resume -and (Test-Path -LiteralPath $responsePath -PathType Leaf))) {
        & python $QianwenTextScript --file $requestPath --output $responseDir --model $Model --stream --hide-reasoning
        if ($LASTEXITCODE -ne 0) { throw "QianWen request failed: $Id" }
    }
    $text = Get-ResponseText -ResponsePath $responsePath
    if ($text.Length -lt $MinimumChars) {
        throw "Generated note $Id is too short: $($text.Length) < $MinimumChars characters."
    }
    foreach ($heading in $RequiredHeadings) {
        if ($text -notmatch "(?m)^## $([regex]::Escape($heading))\s*$") {
            throw "Generated note $Id is missing required heading: $heading"
        }
    }
    $foundIds = @([regex]::Matches($text, 'M-(\d{6})') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
    $required = @($RequiredModuleIds | ForEach-Object { [string]$_ } | Sort-Object -Unique)
    $missing = @($required | Where-Object { $foundIds -notcontains $_ })
    $unknown = @($foundIds | Where-Object { $allModuleIds -notcontains $_ })
    if ($missing.Count -gt 0) { throw "Generated note $Id omitted source modules: $($missing -join ', ')" }
    if ($unknown.Count -gt 0) { throw "Generated note $Id cited unknown modules: $($unknown -join ', ')" }
    if ($foundIds.Count -lt $MinimumCitedModules) {
        throw "Generated note $Id cites too few distinct modules: $($foundIds.Count) < $MinimumCitedModules"
    }
    $outputPath = Join-Path $generatedRoot ($Id + '.md')
    Set-Content -LiteralPath $outputPath -Value $text -Encoding utf8
    $response = Get-Content -LiteralPath $responsePath -Raw | ConvertFrom-Json
    $usage = if ($response.PSObject.Properties['usage']) { $response.usage } else { [ordered]@{} }
    $script:runRows += [ordered]@{
        id = $Id
        model = if ($response.PSObject.Properties['model'] -and $response.model) { [string]$response.model } else { $Model }
        request_sha256 = Get-Sha256 -Path $requestPath
        response_sha256 = Get-Sha256 -Path $responsePath
        output_sha256 = Get-Sha256 -Path $outputPath
        output_chars = $text.Length
        required_modules = $required
        cited_modules = $foundIds
        usage = $usage
    }
    return $outputPath
}

$sourceMapPath = (Get-Item -LiteralPath $SourceMapPath).FullName
$qianwenTextScript = (Get-Item -LiteralPath $QianwenTextScript).FullName
$stagingRoot = [IO.Path]::GetFullPath($StagingRoot)
$liveVaultPath = [IO.Path]::GetFullPath($LiveVaultPath)
$packageRoot = Join-Path $stagingRoot 'package'
$requestsRoot = Join-Path $stagingRoot 'requests'
$responsesRoot = Join-Path $stagingRoot 'responses'
$generatedRoot = Join-Path $stagingRoot 'generated'
$sourceNotesRoot = Join-Path $packageRoot '来源'

if ((Test-Path -LiteralPath $stagingRoot) -and -not $AllowExistingStaging -and -not $Resume) {
    throw "Staging root already exists; use a fresh run root: $stagingRoot"
}
if ((Test-Path -LiteralPath $stagingRoot) -and -not $Resume) {
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $packageRoot, $requestsRoot, $responsesRoot, $generatedRoot, $sourceNotesRoot -Force | Out-Null

$sourceMap = Get-Content -LiteralPath $sourceMapPath -Raw | ConvertFrom-Json
$seen = @{}
$items = @()
foreach ($chunk in @($sourceMap.chunks)) {
    foreach ($raw in @($chunk.items)) {
        $key = [string]$raw.module_id
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $path = (Get-Item -LiteralPath $raw.c1_path).FullName
        $sha = Get-Sha256 -Path $path
        $declaredC1Hash = if ($raw.PSObject.Properties['c1_sha256']) { [string]$raw.c1_sha256 } else { '' }
        if ($declaredC1Hash -and $sha -ne $declaredC1Hash.ToLowerInvariant()) {
            throw "C1 hash mismatch for module $key"
        }
        $items += [ordered]@{
            module_id = $key
            title = [string]$raw.title
            module_type = [string]$raw.module_type
            parent = [string]$raw.parent
            c1_path = $path
            c1_sha256 = $sha
            c1_chars = (Get-Content -LiteralPath $path -Raw).Length
            c0_item_id = if ($raw.PSObject.Properties['c0_item_id']) { [string]$raw.c0_item_id } else { '' }
            c0_revision_id = if ($raw.PSObject.Properties['c0_revision_id']) { [string]$raw.c0_revision_id } else { '' }
            c0_asset_id = if ($raw.PSObject.Properties['c0_asset_id']) { [string]$raw.c0_asset_id } else { '' }
            c0_asset_sha256 = if ($raw.PSObject.Properties['c0_asset_sha256']) { [string]$raw.c0_asset_sha256 } else { '' }
        }
    }
}
$allModuleIds = @($items | ForEach-Object { [string]$_.module_id } | Sort-Object -Unique)
if ($allModuleIds.Count -ne 37) { throw "Expected 37 finance modules, found $($allModuleIds.Count)." }

$chapters = @(
    [ordered]@{ id='01-财务管理目标与财务高管职责'; title='财务管理目标与财务高管职责'; modules=@('892115','892117','892119') },
    [ordered]@{ id='02-营运资本管理'; title='营运资本管理'; modules=@('892121','892123','892125','892127','892129','892131','897613') },
    [ordered]@{ id='03-项目投资评估'; title='项目投资评估'; modules=@('892135','892137','892139','892141','892143','892145','901333','972135') },
    [ordered]@{ id='04-投资风险与不确定性'; title='投资风险与不确定性'; modules=@('892153','892155') },
    [ordered]@{ id='05-企业估值'; title='企业估值'; modules=@('892157','892159') },
    [ordered]@{ id='06-融资决策与资本结构'; title='融资决策与资本结构'; modules=@('892161','892163','892165','972531','972533') },
    [ordered]@{ id='07-股利决策'; title='股利决策'; modules=@('892171','892173') },
    [ordered]@{ id='08-并购与企业重组'; title='并购与企业重组'; modules=@('892175','892177','892179','892181','892185','892187','976503','976505') }
)
$assigned = @($chapters | ForEach-Object { $_.modules } | Sort-Object -Unique)
if ($assigned.Count -ne 37 -or @($allModuleIds | Where-Object { $assigned -notcontains $_ }).Count -gt 0) {
    throw 'Chapter assignment must cover every finance module exactly once.'
}

$sourceLedger = @()
foreach ($item in $items) {
    $rawText = Get-Content -LiteralPath $item.c1_path -Raw
    $body = Remove-LeadingTitle -Text $rawText
    $note = @(
        '---'
        'babata_type: c1_source'
        "module_id: $($item.module_id)"
        'course: 25春 MBAO5406 财务管理'
        "source_type: $($item.module_type)"
        '---'
        ''
        "# $($item.title)"
        ''
        $body
        ''
        '<details>'
        '<summary>来源与完整性</summary>'
        ''
        "- C1 SHA-256：$($item.c1_sha256)"
        "- C1 字符数：$($item.c1_chars)"
        "- C0 item：$($item.c0_item_id)"
        "- C0 revision：$($item.c0_revision_id)"
        "- C0 asset：$($item.c0_asset_id)"
        "- C0 SHA-256：$($item.c0_asset_sha256)"
        ''
        '</details>'
        ''
    ) -join "`n"
    $sourceNote = Join-Path $sourceNotesRoot ("M-$($item.module_id).md")
    Set-Content -LiteralPath $sourceNote -Value $note -Encoding utf8
    $sourceLedger += [ordered]@{
        module_id = $item.module_id
        title = $item.title
        chapter = [string](@($chapters | Where-Object { $_.modules -contains $item.module_id })[0].id)
        c1_path = $item.c1_path
        c1_sha256 = $item.c1_sha256
        c1_chars = $item.c1_chars
        source_note = "来源/M-$($item.module_id).md"
        source_note_sha256 = Get-Sha256 -Path $sourceNote
    }
}

$systemPrompt = @'
你是一名严谨的 MBA 财务管理教材编辑。只根据用户提供的课程 C1 原文编写可直接学习的中文章节，不得引入原文没有支持的事实、数字、公式或案例。目标是完整、清晰、可复习的知识正文，不是摘要卡片，也不是目录或元数据展示。

硬性规则：
1. 正文必须实质展开概念、因果、决策步骤、公式含义、适用条件、案例和易错点；禁止用一句话代替章节。
2. 每个重要判断紧邻引用一个或多个来源，格式只能是 [[来源/M-六位模块号]]；时间戳或页码可在引用后补充。
3. 只能引用本次输入列出的模块；不得编造页码、时间戳、数值、英文术语或结论。
4. 同一内容在课件与转写重复时整合表达，不机械重复；口误或 OCR/ASR 噪声不当作知识。
5. 公式必须解释变量、决策含义、使用条件和常见误用。原文不足时明确写“课程材料未给出足够细节”，不得自行补齐。
6. 不写系统提示、处理流程、模型、C1/C2、试点、审核状态、存储边界或任何控制面说明。
7. 只输出 Markdown 正文，不要代码围栏，不要 YAML frontmatter。
'@

$runRows = @()
$chapterOutputs = @()
foreach ($chapter in $chapters) {
    $chapterItems = @($items | Where-Object { $chapter.modules -contains $_.module_id })
    $sources = @()
    foreach ($item in $chapterItems) {
        $sources += @(
            "`n===== SOURCE [[来源/M-$($item.module_id)]] | $($item.title) | $($item.module_type) =====`n"
            (Get-Content -LiteralPath $item.c1_path -Raw)
        ) -join ''
    }
    $userPrompt = @"
编写《$($chapter.title)》完整学习章节。必须覆盖以下全部模块并在“来源索引”逐项列出：$($chapter.modules -join ', ')。

固定结构：
# $($chapter.title)
## 本章要解决的问题
## 核心结论
## 概念与逻辑
## 决策方法
## 公式与计算
## 案例与应用
## 易错点与边界
## 复习检查
## 来源索引

要求正文至少 3000 个中文字符；“来源索引”必须逐项写出 [[来源/M-模块号]] 和资料标题。不要为了篇幅重复同一句话。

以下是完整课程 C1 输入：
$($sources -join "`n")
"@
    $chapterOutputs += Invoke-QianwenMarkdown -Id $chapter.id -SystemPrompt $systemPrompt -UserPrompt $userPrompt -RequiredModuleIds $chapter.modules -MinimumChars 3000 -RequiredHeadings @('本章要解决的问题','核心结论','概念与逻辑','决策方法','公式与计算','案例与应用','易错点与边界','复习检查','来源索引')
}

$chapterCorpus = @()
foreach ($path in $chapterOutputs) {
    $chapterCorpus += "`n===== CHAPTER $(Split-Path -LeafBase $path) =====`n$(Get-Content -LiteralPath $path -Raw)"
}
$allChapterText = $chapterCorpus -join "`n"

$synthesisSpecs = @(
    [ordered]@{
        id='00-课程总览'; min=2800; headings=@('课程主线','八章关系','关键决策链','学习顺序','课程边界','章节导航'); prompt=@"
基于下列八篇已带来源引用的章节，编写整门《财务管理》课程总览。必须使用固定结构：
# 财务管理课程总览
## 课程主线
## 八章关系
## 关键决策链
## 学习顺序
## 课程边界
## 章节导航
保留章节中的来源引用；“章节导航”链接 [[01-财务管理目标与财务高管职责]] 至 [[08-并购与企业重组]]。正文至少 2800 字符。
$allChapterText
"@
    },
    [ordered]@{
        id='09-公式与决策工具'; min=3800; headings=@('使用方法','营运资本工具','投资评估工具','风险分析工具','估值工具','融资与股利工具','并购重组工具','公式易错检查表','来源索引'); prompt=@"
基于下列八篇章节，编写课程公式与决策工具手册。只收录章节来源支持的公式和方法，不补写教材之外的公式。每项写清用途、变量、步骤、接受/拒绝逻辑、假设、易错点和来源。固定结构：
# 公式与决策工具
## 使用方法
## 营运资本工具
## 投资评估工具
## 风险分析工具
## 估值工具
## 融资与股利工具
## 并购重组工具
## 公式易错检查表
## 来源索引
正文至少 3800 字符。
$allChapterText
"@
    },
    [ordered]@{
        id='10-案例练习'; min=3200; headings=@('案例使用方法','营运资本案例','项目投资案例','估值与融资案例','并购重组案例','综合练习','参考思路','来源索引'); prompt=@"
基于下列八篇章节中的真实课程案例，编写案例练习册。每个案例包含背景、任务、分析步骤、结论或参考思路、易错点和来源；不得编造缺失数字。固定结构：
# 案例练习
## 案例使用方法
## 营运资本案例
## 项目投资案例
## 估值与融资案例
## 并购重组案例
## 综合练习
## 参考思路
## 来源索引
正文至少 3200 字符。
$allChapterText
"@
    },
    [ordered]@{
        id='11-复习与自测'; min=2800; headings=@('复习路径','核心概念自测','计算与决策自测','案例论述题','易混点速查','答案要点','来源索引'); prompt=@"
基于下列八篇章节编写复习与自测文档。问题必须覆盖八章，答案要点不能只有一句话，必须说明判断路径并保留来源引用。固定结构：
# 复习与自测
## 复习路径
## 核心概念自测
## 计算与决策自测
## 案例论述题
## 易混点速查
## 答案要点
## 来源索引
正文至少 2800 字符。
$allChapterText
"@
    }
)
foreach ($spec in $synthesisSpecs) {
    [void](Invoke-QianwenMarkdown -Id $spec.id -SystemPrompt $systemPrompt -UserPrompt $spec.prompt -RequiredModuleIds @() -MinimumChars $spec.min -RequiredHeadings $spec.headings -MinimumCitedModules 8)
}

Invoke-EditorialCorrections -Root $generatedRoot
Test-MarkdownStructure -Root $generatedRoot
foreach ($row in $runRows) {
    $finalPath = Join-Path $generatedRoot ($row.id + '.md')
    $row['model_output_sha256'] = $row.output_sha256
    $row['final_output_sha256'] = Get-Sha256 -Path $finalPath
    $row['final_output_chars'] = (Get-Content -LiteralPath $finalPath -Raw -Encoding utf8).Length
    $row['editorial_corrections_applied'] = ($row.final_output_sha256 -ne $row.model_output_sha256)
}

foreach ($file in Get-ChildItem -LiteralPath $generatedRoot -File -Filter '*.md') {
    Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $packageRoot $file.Name) -Force
}

if ($MediaSourcePath) {
    $mediaSourcePath = (Get-Item -LiteralPath $MediaSourcePath).FullName
    $mediaTarget = Join-Path $packageRoot 'media'
    New-Item -ItemType Directory -Path $mediaTarget -Force | Out-Null
    Get-ChildItem -LiteralPath $mediaSourcePath -File | Copy-Item -Destination $mediaTarget -Force
    $visualBlock = @(
        ''
        '## 关键视觉证据'
        ''
        '![EOQ 公式课件截图](media/892131-page09-eoq-formula.png)'
        ''
        '![EOQ 与营运资本公式课件截图](media/892131-page39-formula.png)'
        ''
    ) -join "`n"
    Add-Content -LiteralPath (Join-Path $packageRoot '02-营运资本管理.md') -Value $visualBlock -Encoding utf8
    Add-Content -LiteralPath (Join-Path $packageRoot '09-公式与决策工具.md') -Value $visualBlock -Encoding utf8
}

$indexLines = @(
    '---'
    'babata_type: c2b_course_knowledge_base'
    'course: 25春 MBAO5406 财务管理'
    'status: usable_candidate'
    '---'
    ''
    '# 财务管理知识库'
    ''
    '从 [[00-课程总览]] 开始。正文按课程决策链组织，完整课程来源保留在 `来源/`。'
    ''
    '## 课程章节'
)
foreach ($chapter in $chapters) { $indexLines += "- [[$($chapter.id)]]" }
$indexLines += @(
    ''
    '## 学习工具'
    '- [[09-公式与决策工具]]'
    '- [[10-案例练习]]'
    '- [[11-复习与自测]]'
    ''
    '## 完整来源'
    '- [[来源索引]]'
    ''
)
$indexLines -join "`n" | Set-Content -LiteralPath (Join-Path $packageRoot 'index.md') -Encoding utf8

$sourceIndex = @('# 来源索引', '', '以下 37 份 C1 正文均完整物化，可从章节引用直接打开。', '')
foreach ($chapter in $chapters) {
    $sourceIndex += "## $($chapter.title)"
    foreach ($id in $chapter.modules) {
        $item = @($items | Where-Object module_id -eq $id)[0]
        $sourceIndex += "- [[来源/M-$id]] $($item.title)"
    }
    $sourceIndex += ''
}
$sourceIndex -join "`n" | Set-Content -LiteralPath (Join-Path $packageRoot '来源索引.md') -Encoding utf8

$banned = @('本试点','C2 应当','外部主权库负责','不是正式 C2','provider','模型生成','控制面')
$knowledgeFiles = @(Get-ChildItem -LiteralPath $packageRoot -File -Filter '*.md' | Where-Object Name -notin @('来源索引.md'))
$lintHits = @()
foreach ($file in $knowledgeFiles) {
    $text = Get-Content -LiteralPath $file.FullName -Raw
    foreach ($term in $banned) {
        if ($text.Contains($term)) { $lintHits += "$($file.Name):$term" }
    }
}
if ($lintHits.Count -gt 0) { throw "Knowledge body control-plane lint failed: $($lintHits -join ', ')" }

$wikiMissing = @()
foreach ($file in Get-ChildItem -LiteralPath $packageRoot -Recurse -File -Filter '*.md') {
    $text = Get-Content -LiteralPath $file.FullName -Raw
    foreach ($match in [regex]::Matches($text, '\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\]')) {
        $target = $match.Groups[1].Value.Replace('/', '\')
        $candidate = Join-Path $packageRoot ($target + '.md')
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            $wikiMissing += "$($file.Name):$($match.Groups[1].Value)"
        }
    }
}
if ($wikiMissing.Count -gt 0) { throw "Dangling Wiki links: $($wikiMissing -join ', ')" }

$manifest = [ordered]@{
    schema = 'babata.c2b.usable-course-knowledge-base/v1'
    task = Split-Path -Leaf $stagingRoot
    course = [string]$sourceMap.course
    status = 'usable_candidate'
    model = $Model
    provider = 'qianwen_skill'
    adapter = 'qianwen-text'
    source_map = $sourceMapPath
    source_map_sha256 = Get-Sha256 -Path $sourceMapPath
    source_modules = $sourceLedger
    chapters = $chapters
    runs = $runRows
    package_files = @(
        Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
            [ordered]@{
                path = Get-RelativePath -Base $packageRoot -Path $_.FullName
                sha256 = Get-Sha256 -Path $_.FullName
                bytes = $_.Length
            }
        }
    )
    output = [ordered]@{
        package = 'package'
        markdown_files = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File -Filter '*.md').Count
        knowledge_documents = $knowledgeFiles.Count
        source_documents = @(Get-ChildItem -LiteralPath $sourceNotesRoot -File -Filter '*.md').Count
        media_files = @(Get-ChildItem -LiteralPath (Join-Path $packageRoot 'media') -File -ErrorAction SilentlyContinue).Count
    }
    formal_registration = 'not_started'
}
$manifest | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath (Join-Path $stagingRoot 'manifest.json') -Encoding utf8

$verification = [ordered]@{
    status = 'passed'
    source_modules = $allModuleIds.Count
    chapter_documents = $chapters.Count
    synthesis_documents = $synthesisSpecs.Count
    thin_knowledge_cards = 0
    source_documents = @(Get-ChildItem -LiteralPath $sourceNotesRoot -File -Filter '*.md').Count
    dangling_wiki_links = 0
    control_plane_lint_hits = 0
    markdown_structure_errors = 0
    editorial_residue_hits = 0
    live_published = [bool]$Publish
}
$verification | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $stagingRoot 'verification.json') -Encoding utf8

if ($Publish) {
    $archiveRoot = Join-Path (Split-Path -Parent $stagingRoot) 'user-export-archive'
    New-Item -ItemType Directory -Path $archiveRoot -Force | Out-Null
    $publishCandidate = "$liveVaultPath.publish-candidate"
    if (Test-Path -LiteralPath $publishCandidate) {
        Remove-Item -LiteralPath $publishCandidate -Recurse -Force
    }
    New-Item -ItemType Directory -Path $publishCandidate -Force | Out-Null
    Get-ChildItem -LiteralPath $packageRoot -Force | Copy-Item -Destination $publishCandidate -Recurse -Force

    $packageFiles = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File)
    $candidateFiles = @(Get-ChildItem -LiteralPath $publishCandidate -Recurse -File)
    if ($candidateFiles.Count -ne $packageFiles.Count) {
        throw "Publish candidate file count mismatch: $($candidateFiles.Count) != $($packageFiles.Count)"
    }
    foreach ($file in $packageFiles) {
        $relative = Get-RelativePath -Base $packageRoot -Path $file.FullName
        $candidate = Join-Path $publishCandidate $relative.Replace('/', '\')
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf) -or (Get-Sha256 $candidate) -ne (Get-Sha256 $file.FullName)) {
            throw "Publish candidate hash mismatch: $relative"
        }
    }

    if (Test-Path -LiteralPath $liveVaultPath) {
        $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
        $archiveTarget = Join-Path $archiveRoot ("finance-live-before-usable-$stamp")
        Move-Item -LiteralPath $liveVaultPath -Destination $archiveTarget
    }
    Move-Item -LiteralPath $publishCandidate -Destination $liveVaultPath
}

@(
    '# 财务管理可用 C2B 知识库构建报告'
    ''
    '- 状态：usable_candidate'
    "- C1 来源：$($allModuleIds.Count)/37"
    "- 完整章节：$($chapters.Count)/8"
    "- 跨章工具文档：$($synthesisSpecs.Count)/4"
    '- 薄知识卡片：0'
    "- 来源正文：$(@(Get-ChildItem -LiteralPath $sourceNotesRoot -File -Filter '*.md').Count)/37"
    '- 悬空 Wiki 链接：0'
    '- 正文控制面污染：0'
    '- Markdown 结构错误：0'
    '- 编辑残留：0'
    "- 发布到 live Vault：$([bool]$Publish)"
    ''
    '本批以可学习的章节正文为交付单元，不再以模板文件数或一句话语义卡片作为完成标准。'
) | Set-Content -LiteralPath (Join-Path $stagingRoot 'REPORT.md') -Encoding utf8

Write-Output "staged=$stagingRoot chapters=$($chapters.Count) sources=$($allModuleIds.Count) published=$([bool]$Publish)"
