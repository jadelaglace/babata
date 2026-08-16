[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$CoursePlanPath,
    [Parameter(Mandatory=$true)][string]$SourceMapPath,
    [Parameter(Mandatory=$true)][string]$C1BPreparationReceiptPath,
    [Parameter(Mandatory=$true)][string]$QianwenTextScript,
    [Parameter(Mandatory=$true)][string]$StagingRoot,
    [string]$DataHome = $env:BABATA_DATA_HOME,
    [string]$Model = 'qwen3.6-plus',
    [int]$ChunkCharLimit = 150000,
    [int]$MaxRequestChars = 180000,
    [string]$DigestGrouper = (Join-Path $PSScriptRoot 'group-mba-learning-digests.ps1'),
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not (Test-Path -LiteralPath $DigestGrouper -PathType Leaf)) {
    throw "Digest grouper not found: $DigestGrouper"
}
. $DigestGrouper
if (-not (Get-Command Group-MbaLearningDigestNodes -CommandType Function -ErrorAction SilentlyContinue)) {
    throw 'Digest grouper must define Group-MbaLearningDigestNodes'
}

function Hash([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Is-Contained([string]$Path,[string]$Root) {
    $resolvedPath = [IO.Path]::GetFullPath($Path)
    $resolvedRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    return $resolvedPath.StartsWith($resolvedRoot + '\',[StringComparison]::OrdinalIgnoreCase)
}

function Assert-SafeBasename([string]$Value,[string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Value) -or $Value.Length -gt 100 -or $Value -in @('.','..') -or
        $Value.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0 -or $Value.Contains('/') -or $Value.Contains('\')) {
        throw "Unsafe $Label basename: $Value"
    }
}

function Escape-Sql([string]$Value) { return $Value.Replace("'", "''") }
function Sql-Rows([string]$Database,[string]$Sql) {
    $raw = & sqlite3 -json $Database $Sql
    if ($LASTEXITCODE -ne 0) { throw 'SQLite read failed' }
    $text = ($raw -join "`n").Trim()
    if ([string]::IsNullOrWhiteSpace($text) -or $text -eq '[]') { return @() }
    return @($text | ConvertFrom-Json)
}

function Split-Evidence([string]$Text,[int]$Limit) {
    if ($Limit -lt 1000) { throw 'ChunkCharLimit must be at least 1000' }
    $segments = @(); $offset = 0
    while ($offset -lt $Text.Length) {
        $length = [Math]::Min($Limit,$Text.Length-$offset)
        if (($offset+$length) -lt $Text.Length) {
            $window = $Text.Substring($offset,$length)
            $break = $window.LastIndexOf("`n`n",[StringComparison]::Ordinal)
            if ($break -gt [int]($Limit*0.6)) { $length = $break+2 }
        }
        $segments += [ordered]@{start=$offset;end=$offset+$length;text=$Text.Substring($offset,$length)}
        $offset += $length
    }
    return $segments
}

function Response-Text([string]$Path) {
    $response = Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json
    if (-not $response.choices -or -not $response.choices[0].message.content) {
        throw "QianWen response contains no message content: $Path"
    }
    $text = [string]$response.choices[0].message.content
    $text = [regex]::Replace($text.Trim(), '\A```(?:markdown)?\s*', '')
    return [regex]::Replace($text, '\s*```\z', '').Trim()
}

function Assert-Markdown([string]$Path,[int]$MinimumChars) {
    $text = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    if ($text.Length -lt $MinimumChars) { throw "Learning document is too short: $Path ($($text.Length) < $MinimumChars)" }
    if ([regex]::Matches($text,'(?m)^# ').Count -ne 1) { throw "Learning document requires exactly one H1: $Path" }
    if ([regex]::Matches($text,'```').Count % 2 -ne 0) { throw "Unclosed Markdown fence: $Path" }
    foreach ($line in ($text -split "`r?`n")) {
        if ([regex]::Matches($line,'\*\*').Count % 2 -ne 0) { throw "Unbalanced bold marker: $Path" }
    }
    foreach ($term in @('作为AI','语言模型','QianWen','C1B','staging','控制面','prompt','model response','request.json','response.json','SHA-256','token usage')) {
        if ($text.Contains($term,[StringComparison]::OrdinalIgnoreCase)) { throw "Control-plane/editorial residue in $Path`: $term" }
    }
    foreach ($pattern in @('(?i)\bpipeline_id\b','(?i)\b(?:build|registration|processing|model|generation)\s+pipeline\b','(?i)\bpipeline\s+(?:run|receipt|manifest)\b')) {
        if ($text -match $pattern) { throw "Control-plane/editorial residue in $Path`: $pattern" }
    }
    if ($text -match '(?i)[A-Z]:\\') { throw "Absolute filesystem path leaked into learning document: $Path" }
    $knowledgeBody = ($text -split '(?m)^## 来源索引\s*$',2)[0]
    if ($knowledgeBody -match 'M-\d{6}') { throw "Technical module id leaked into learning prose: $Path" }
}

function Invoke-Qianwen(
    [string]$Id,
    [string]$SystemPrompt,
    [string]$UserPrompt,
    [int]$MinimumChars
) {
    if (($SystemPrompt.Length + $UserPrompt.Length) -gt $MaxRequestChars) {
        throw "QianWen request exceeds character budget: $Id"
    }
    $requestDir = Join-Path $requestsRoot $Id
    New-Item -ItemType Directory -Path $requestDir -Force | Out-Null
    $requestPath = Join-Path $requestDir 'request.json'
    [ordered]@{
        model=$Model
        enable_thinking=$false
        temperature=0.12
        max_tokens=12000
        messages=@(
            [ordered]@{role='system';content=$SystemPrompt}
            [ordered]@{role='user';content=$UserPrompt}
        )
    } | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $requestPath -Encoding utf8
    $requestHash = Hash $requestPath
    $responseDir = Join-Path (Join-Path $responsesRoot $Id) $requestHash
    New-Item -ItemType Directory -Path $responseDir -Force | Out-Null
    $responsePath = Join-Path $responseDir 'response.json'
    if (-not ($Resume -and (Test-Path -LiteralPath $responsePath -PathType Leaf))) {
        & python $qianwenScript --file $requestPath --output $responseDir --model $Model --stream --hide-reasoning
        if ($LASTEXITCODE -ne 0) { throw "QianWen request failed: $Id" }
    }
    $text = Response-Text $responsePath
    if ($text.Length -lt $MinimumChars) { throw "QianWen output is too short: $Id ($($text.Length) < $MinimumChars)" }
    $response = Get-Content -LiteralPath $responsePath -Raw -Encoding utf8 | ConvertFrom-Json
    $script:runs += [ordered]@{
        id=$Id
        request_sha256=$requestHash
        response_sha256=Hash $responsePath
        output_chars=$text.Length
        model=if ($response.model) { [string]$response.model } else { $Model }
        usage=if ($response.PSObject.Properties['usage']) { $response.usage } else { [ordered]@{} }
    }
    return $text
}

function New-ChapterPrompt([object]$Chapter,[object[]]$DigestNodes) {
    $digestText = @($DigestNodes | ForEach-Object { [string]$_.text }) -join "`n`n---`n`n"
    return @"
将下列分组证据整合为一篇可连续学习、可复习、可用于决策的完整章节。合并重复内容但不要删掉独有知识。必须使用：
# $($Chapter.title)
## 本章要解决的问题
## 核心结论
## 概念与逻辑
## 决策方法
## 公式与计算
## 案例与应用
## 易错点与边界
## 复习检查
不要生成“来源索引”，系统会追加。正文叙述不得出现 M-编号或其他工程 ID；用课程标题、概念或案例名称自然指代来源。正文至少 5000 个字符。

$digestText
"@
}

function New-DigestReductionPrompt(
    [object]$Chapter,
    [int]$Level,
    [int]$GroupNo,
    [int]$GroupCount,
    [object[]]$Nodes
) {
    $requiredModules = @($Nodes.modules | ForEach-Object { [string]$_ } | Sort-Object -Unique)
    $required = @($requiredModules | ForEach-Object { "M-$_" }) -join '、'
    $evidence = @($Nodes | ForEach-Object {
        "`n===== $([string]$_.id) | $(@($_.modules | ForEach-Object { "M-$_" }) -join '、') =====`n$([string]$_.text)"
    }) -join "`n"
    return @"
为《$($Chapter.title)》执行第 $Level 层证据归约，第 $GroupNo/$GroupCount 组。完整保留组内全部独有概念、公式、变量、条件、案例、冲突、风险和边界；只合并重复表达，不得删除独有知识。必须在末尾用“## 本组来源”逐项列出全部 $required，不能遗漏、替换或编造 M-编号。不要跨越所给证据推断。输出可供下一层继续合成的中文 Markdown，至少 1800 个字符。

$evidence
"@
}

function Reduce-DigestNodesToBudget(
    [object]$Chapter,
    [object[]]$Nodes,
    [string]$SystemPrompt
) {
    $level = 0
    $currentNodes = @($Nodes)
    while (($SystemPrompt.Length + (New-ChapterPrompt $Chapter $currentNodes).Length) -gt $MaxRequestChars) {
        $level++
        if ($level -gt 12) { throw "Digest reduction did not converge: $($Chapter.id)" }

        # Leave ample room for instructions, node labels, and module identities.
        $groupBudget = $MaxRequestChars - $SystemPrompt.Length - 5000
        if ($groupBudget -lt 1000) { throw 'MaxRequestChars is too small for digest reduction' }
        $groups = @(Group-MbaLearningDigestNodes -Nodes $currentNodes -CharBudget $groupBudget)
        $nextNodes = @()
        $groupNo = 0
        foreach ($group in $groups) {
            $groupNo++
            $groupNodes = @($group.nodes)
            $modules = @($groupNodes.modules | ForEach-Object { [string]$_ } | Sort-Object -Unique)
            $id = '{0}-reduce-{1:d2}-{2:d2}' -f [string]$Chapter.id,$level,$groupNo
            $prompt = New-DigestReductionPrompt $Chapter $level $groupNo $groups.Count $groupNodes
            $reduced = Invoke-Qianwen $id $SystemPrompt $prompt 1800
            foreach ($module in $modules) {
                if ($reduced -notmatch "M-$module(?!\d)") { throw "Digest reduction $id omitted module M-$module" }
            }
            Set-Content -LiteralPath (Join-Path $digestRoot ($id + '.md')) -Value $reduced -Encoding utf8
            $nextNodes += [pscustomobject]@{id=$id;text=$reduced;modules=$modules}
        }
        $beforeChars = (@($currentNodes | ForEach-Object { ([string]$_.text).Length }) | Measure-Object -Sum).Sum
        $afterChars = (@($nextNodes | ForEach-Object { ([string]$_.text).Length }) | Measure-Object -Sum).Sum
        if ($afterChars -ge $beforeChars) { throw "Digest reduction made no progress: $($Chapter.id) level $level" }
        $currentNodes = @($nextNodes)
    }
    return $currentNodes
}

$planPath = (Get-Item -LiteralPath $CoursePlanPath).FullName
$mapPath = (Get-Item -LiteralPath $SourceMapPath).FullName
$preparationPath = (Get-Item -LiteralPath $C1BPreparationReceiptPath).FullName
$qianwenScript = (Get-Item -LiteralPath $QianwenTextScript).FullName
$data = if ([string]::IsNullOrWhiteSpace($DataHome)) { throw 'BABATA_DATA_HOME or -DataHome is required' } else { [IO.Path]::GetFullPath($DataHome) }
$runtimeStaging = Join-Path $data '04_runtime\staging'
$managedC1Root = Join-Path $data '02_derived\files'
$derivedDb = Join-Path $data '02_derived\index\derived.sqlite'
$root = [IO.Path]::GetFullPath($StagingRoot)
if (-not (Is-Contained $root $runtimeStaging)) { throw 'Learning-doc staging root must stay under BABATA_DATA_HOME/04_runtime/staging' }
if (-not (Test-Path -LiteralPath $derivedDb -PathType Leaf)) { throw "Missing derived index: $derivedDb" }
if ((Test-Path -LiteralPath $root) -and -not $Resume) { throw "Use a fresh learning-doc staging root: $root" }
$requestsRoot = Join-Path $root 'requests'
$responsesRoot = Join-Path $root 'responses'
$digestRoot = Join-Path $root 'digests'
$generatedRoot = Join-Path $root 'generated'
$sourceRoot = Join-Path $root 'sources'
New-Item -ItemType Directory -Path $requestsRoot,$responsesRoot,$digestRoot,$generatedRoot,$sourceRoot -Force | Out-Null

$plan = Get-Content -LiteralPath $planPath -Raw -Encoding utf8 | ConvertFrom-Json
$sourceMap = Get-Content -LiteralPath $mapPath -Raw -Encoding utf8 | ConvertFrom-Json
$preparation = Get-Content -LiteralPath $preparationPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($plan.schema -ne 'babata.mba-course-c2b-plan/v1') { throw 'Unsupported course plan schema' }
if ($sourceMap.course -cne $plan.course) { throw 'Course plan and source map course mismatch' }
$expected = [int]$plan.expected_modules
if ($preparation.schema -ne 'babata.mba-course-c1b-preparation/v1' -or $preparation.course -cne $plan.course -or
    [int]$preparation.complete_c1 -ne $expected -or [int]$preparation.c1b_decisions -ne $expected -or
    ([string]$preparation.course_plan_sha256).ToLowerInvariant() -cne (Hash $planPath) -or
    ([string]$preparation.source_map_sha256).ToLowerInvariant() -cne (Hash $mapPath)) {
    throw 'C1B preparation receipt does not bind this complete source map and course denominator'
}
$items = @($sourceMap.chunks.items)
if ($items.Count -ne $expected -or @($items.module_id | Sort-Object -Unique).Count -ne $expected) {
    throw "Source map must contain exactly $expected unique modules"
}

$chapterByModule = @{}
$chapterIds = @($plan.chapters.id | ForEach-Object { [string]$_ })
$chapterNotes = @($plan.chapters.note | ForEach-Object { [string]$_ })
$chapterTitles = @($plan.chapters.title | ForEach-Object { [string]$_ })
if (@($chapterIds | Sort-Object -Unique).Count -ne $chapterIds.Count -or
    @($chapterNotes | Sort-Object -Unique).Count -ne $chapterNotes.Count -or
    @($chapterTitles | Sort-Object -Unique).Count -ne $chapterTitles.Count) {
    throw 'Chapter id, note and title must each be unique'
}
foreach ($chapter in @($plan.chapters)) {
    if ([string]$chapter.id -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') { throw "Unsafe chapter id: $($chapter.id)" }
    Assert-SafeBasename ([string]$chapter.note) 'chapter note'
    Assert-SafeBasename ([string]$chapter.title) 'chapter title'
    foreach ($module in @($chapter.modules)) {
        $key = [string]$module
        if ($chapterByModule.ContainsKey($key)) { throw "Duplicate chapter assignment: $key" }
        $chapterByModule[$key] = [string]$chapter.note
    }
}
$missing = @($items | Where-Object { -not $chapterByModule.ContainsKey([string]$_.module_id) })
if ($chapterByModule.Count -ne $expected -or $missing.Count) { throw 'Chapter plan must partition every source module exactly once' }

$mappedNotes = @($plan.course_map.domains.nodes.note | ForEach-Object { [string]$_ })
if ($mappedNotes.Count -ne $chapterNotes.Count -or @($mappedNotes | Sort-Object -Unique).Count -ne $mappedNotes.Count -or
    @($chapterNotes | Where-Object { $mappedNotes -notcontains $_ }).Count) {
    throw 'Course-map domains must cover every chapter note exactly once'
}
$learningNotes = @($plan.course_map.learning.nodes.note | ForEach-Object { [string]$_ })
if ($learningNotes.Count -ne 4 -or @($learningNotes | Sort-Object -Unique).Count -ne 4 -or
    $learningNotes -notcontains '视觉证据索引') {
    throw 'Course-map learning layer must contain three unique numbered learning documents and 视觉证据索引'
}
$aidNotes = @()
foreach ($prefix in @('09-','10-','11-')) {
    $matches = @($learningNotes | Where-Object { $_.StartsWith($prefix,[StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) { throw "Course-map learning layer requires exactly one $prefix document" }
    Assert-SafeBasename $matches[0] 'learning document'
    $aidNotes += $matches[0]
}

$sourceRows = @()
foreach ($item in $items) {
    if ([string]$item.module_id -notmatch '^\d+$') { throw "Unsafe module id: $($item.module_id)" }
    $source = (Get-Item -LiteralPath ([string]$item.c1_path)).FullName
    if (-not (Is-Contained $source $managedC1Root)) { throw "C1 path is outside managed derived files: $source" }
    $actual = Hash $source
    if ($actual -cne ([string]$item.c1_sha256).ToLowerInvariant()) { throw "C1 hash mismatch: $($item.module_id)" }
    $derivativeId = Escape-Sql ([string]$item.c1_derivative_id)
    $rows = @(Sql-Rows $derivedDb @"
SELECT d.derivative_id,d.logical_path,d.output_sha256,p.run_id,p.input_revision_id,p.state,p.invalidated_at
FROM derivatives d JOIN process_runs p ON p.run_id=d.run_id
WHERE d.derivative_id='$derivativeId';
"@)
    if ($rows.Count -ne 1) { throw "Managed C1 derivative is not unique: $($item.module_id)" }
    $row = $rows[0]
    $managedPath = Join-Path $data ([string]$row.logical_path).Replace('/','\')
    if (-not [IO.Path]::GetFullPath($managedPath).Equals($source,[StringComparison]::OrdinalIgnoreCase) -or [string]$row.run_id -cne [string]$item.c1_run_id -or
        [string]$row.input_revision_id -cne [string]$item.c0_revision_id -or [string]$row.state -ne 'succeeded' -or
        $null -ne $row.invalidated_at -or ([string]$row.output_sha256).ToLowerInvariant() -cne $actual) {
        throw "Managed C1 read-back mismatch: $($item.module_id)"
    }
    $body = Get-Content -LiteralPath $source -Raw -Encoding utf8
    if ([string]::IsNullOrWhiteSpace($body)) { throw "Empty C1: $($item.module_id)" }
    @(
        '---'
        'babata_type: c1_source_evidence'
        "module_id: $($item.module_id)"
        "course: $($plan.course)"
        "chapter: $($item.chapter_note)"
        "c1_sha256: $actual"
        '---'
        ''
        "# $($item.title)"
        ''
        $body.Trim()
        ''
    ) -join "`n" | Set-Content -LiteralPath (Join-Path $sourceRoot "M-$($item.module_id).md") -Encoding utf8
    $stagedSource = Join-Path $sourceRoot "M-$($item.module_id).md"
    $sourceRows += [ordered]@{
        module_id=[int]$item.module_id
        input_derivative_id=[string]$item.c1_derivative_id
        input_sha256=$actual
        staged_path="sources/M-$($item.module_id).md"
        staged_sha256=Hash $stagedSource
        body_preserved=$true
    }
}

$expectedSourceNames = @($items | ForEach-Object { "M-$($_.module_id).md" } | Sort-Object)
$actualSourceNames = @(Get-ChildItem -LiteralPath $sourceRoot -File | Select-Object -ExpandProperty Name | Sort-Object)
if (($expectedSourceNames -join "`n") -cne ($actualSourceNames -join "`n")) { throw 'Source-note file set does not equal the authorized denominator' }

$system = @"
你是一名严谨的 MBA《$($plan.short_name)》教材编辑。只能根据提供的课程原文组织内容，不得补充原文没有支持的事实、数字、公式、案例或结论。区分课程事实、案例假设、计算示例和一般原则；保留重要公式、变量、条件、边界和相互矛盾之处。输出中文 Markdown 学习正文，不写生成过程、模型、数据层级或工程说明。不要用摘要卡片代替完整解释。
"@
$runs = @()
$chapterFiles = @()
foreach ($chapter in @($plan.chapters)) {
    $chapterIds = @($chapter.modules | ForEach-Object { [string]$_ })
    $chapterItems = @($items | Where-Object { $chapterIds -contains [string]$_.module_id })
    $segments = @()
    foreach ($item in $chapterItems) {
        $body = Get-Content -LiteralPath $item.c1_path -Raw -Encoding utf8
        foreach ($part in @(Split-Evidence $body ($ChunkCharLimit-2000))) {
            $segments += [ordered]@{
                module_id=[string]$item.module_id
                title=[string]$item.title
                start=[int]$part.start
                end=[int]$part.end
                text=[string]$part.text
            }
        }
    }
    $batches = @(); $current = @(); $chars = 0
    foreach ($segment in $segments) {
        $segmentChars = $segment.text.Length + 200
        if ($current.Count -and ($chars+$segmentChars) -gt $ChunkCharLimit) {
            $batches += ,@($current); $current=@(); $chars=0
        }
        $current += $segment; $chars += $segmentChars
    }
    if ($current.Count) { $batches += ,@($current) }

    $digestNodes = @(); $batchNo = 0
    foreach ($batch in $batches) {
        $batchNo++
        $evidence = foreach ($segment in $batch) {
            "`n===== M-$($segment.module_id) | $($segment.title) | chars $($segment.start)-$($segment.end) =====`n" + $segment.text
        }
        $requiredModules = @($batch.module_id | Sort-Object -Unique)
        $required = @($requiredModules | ForEach-Object { "M-$_" }) -join '、'
        $prompt = @"
为《$($chapter.title)》整理第 $batchNo/$($batches.Count) 组证据。完整覆盖 $required。使用以下结构：
# $($chapter.title) 证据整理 $batchNo
## 概念与关系
## 方法、公式与计算
## 案例与操作
## 风险、边界与易错点
## 本组来源
“本组来源”逐项列出本组所有 M-编号和标题。不要跨越所给证据推断。
正文至少 2200 个字符，不能用短摘要、提纲或术语列表代替完整证据整理。

$($evidence -join "`n")
"@
        $id = '{0}-digest-{1:d2}' -f [string]$chapter.id,$batchNo
        $digest = Invoke-Qianwen $id $system $prompt 1800
        $digestPath = Join-Path $digestRoot ($id + '.md')
        Set-Content -LiteralPath $digestPath -Value $digest -Encoding utf8
        foreach ($module in $requiredModules) {
            if ($digest -notmatch "M-$module(?!\d)") { throw "Digest $id omitted module M-$module" }
        }
        $digestNodes += [pscustomobject]@{id=$id;text=$digest;modules=@($requiredModules)}
    }

    $sourceLinks = @('','## 来源索引','')
    foreach ($item in $chapterItems) { $sourceLinks += "- [[来源/M-$($item.module_id)|$($item.title)]]" }
    $digestNodes = @(Reduce-DigestNodesToBudget $chapter $digestNodes $system)
    $chapterPrompt = New-ChapterPrompt $chapter $digestNodes
    $chapterText = Invoke-Qianwen ([string]$chapter.note) $system $chapterPrompt 5000
    $chapterText = $chapterText.Trim() + "`n" + ($sourceLinks -join "`n") + "`n"
    $chapterPath = Join-Path $generatedRoot (([string]$chapter.note) + '.md')
    Set-Content -LiteralPath $chapterPath -Value $chapterText -Encoding utf8
    Assert-Markdown $chapterPath 5200
    $chapterFiles += $chapterPath
}

$corpus = foreach ($path in $chapterFiles) {
    $chapterText = Get-Content -LiteralPath $path -Raw -Encoding utf8
    $chapterBody = ($chapterText -split '(?m)^## 来源索引\s*$', 2)[0].Trim()
    "`n===== $(Split-Path -LeafBase $path) =====`n" + $chapterBody
}
$corpusText = $corpus -join "`n"
$chapterLinks = @($plan.chapters | ForEach-Object { "- [[$([string]$_.note)]]" }) -join "`n"
$overviewPrompt = @"
基于下列 $($plan.chapters.Count) 章正文编写整门《$($plan.short_name)》课程总览。使用：
# $($plan.short_name) 课程总览
## 课程主线
## 章节导航
## 端到端决策框架
## 关键公式与指标
## 典型决策场景
## 风险与边界
## 学习路径
“章节导航”必须原样包含：
$chapterLinks
至少 3500 字符；保留章节 Wiki 链接，不写生成过程、来源索引、M-编号或其他工程 ID。

$corpusText
"@
$overview = Invoke-Qianwen '00-课程总览' $system $overviewPrompt 3500
Set-Content -LiteralPath (Join-Path $generatedRoot '00-课程总览.md') -Value $overview -Encoding utf8

$chapterSectionTitles = @($plan.chapters.title | ForEach-Object { [string]$_ })
$aidSpecs = @(
    [ordered]@{id=$aidNotes[0];title=($aidNotes[0] -replace '^09-','');sections=@('工具选择总览')+$chapterSectionTitles+@('使用条件与易错点');minimum=3500},
    [ordered]@{id=$aidNotes[1];title=($aidNotes[1] -replace '^10-','');sections=@('练习说明')+$chapterSectionTitles+@('参考思路');minimum=4000},
    [ordered]@{id=$aidNotes[2];title=($aidNotes[2] -replace '^11-','');sections=@('知识检查','公式检查','情境判断','综合自测','答案与解释','薄弱点回链');minimum=4000}
)
foreach ($spec in $aidSpecs) {
    $headings = @($spec.sections | ForEach-Object { "## $_" }) -join "`n"
    $prompt = @"
基于下列 $($plan.chapters.Count) 章正文编写《$($spec.title)》。使用唯一 H1“# $($spec.title)”并严格包含这些二级标题：
$headings
内容必须覆盖全部 $($plan.chapters.Count) 章并用章节 Wiki 链接回链。计算题给出步骤、单位和适用条件；答案不得脱离课程证据。不要写来源索引、M-编号或其他工程 ID。至少 $($spec.minimum) 字符。

$corpusText
"@
    $text = Invoke-Qianwen ([string]$spec.id) $system $prompt ([int]$spec.minimum)
    $path = Join-Path $generatedRoot (([string]$spec.id) + '.md')
    Set-Content -LiteralPath $path -Value $text -Encoding utf8
}

$expectedDocs = @('00-课程总览') + @($plan.chapters.note) + $aidNotes
foreach ($name in $expectedDocs) {
    $path = Join-Path $generatedRoot ($name + '.md')
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing learning document: $name" }
    Assert-Markdown $path 3000
}

$manifest = [ordered]@{
    schema='babata.mba-course-learning-docs/v1'
    course=$plan.course
    status='candidate'
    course_plan=$planPath
    course_plan_sha256=Hash $planPath
    c1b_preparation_receipt=$preparationPath
    c1b_preparation_receipt_sha256=Hash $preparationPath
    source_map=$mapPath
    source_map_sha256=Hash $mapPath
    expected_modules=$expected
    complete_source_notes=@(Get-ChildItem -LiteralPath $sourceRoot -File -Filter '*.md').Count
    source_notes=$sourceRows
    chapter_documents=@($plan.chapters).Count
    learning_documents=$expectedDocs.Count
    model=$Model
    runs=$runs
    generated_files=@(Get-ChildItem -LiteralPath $generatedRoot -File | Sort-Object Name | ForEach-Object { [ordered]@{name=$_.Name;sha256=Hash $_.FullName;chars=(Get-Content -LiteralPath $_.FullName -Raw -Encoding utf8).Length} })
}
$manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $root 'manifest.json') -Encoding utf8
Write-Output "staged=$root course=$($plan.course) sources=$expected chapters=$(@($plan.chapters).Count) learning_docs=$($expectedDocs.Count) model_calls=$($runs.Count)"
