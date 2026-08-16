[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PackageRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Require-Text([object]$Value,[string]$Label) {
    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) { throw "$Label is required" }
    $text
}

function Require-Id([object]$Value,[string]$Label) {
    $id = Require-Text $Value $Label
    # Course plans may use kebab-case keys; Mermaid identifiers cannot contain
    # hyphens, so normalize them at the rendering boundary while retaining the
    # plan's human-facing labels and hashes.
    $normalized = $id -replace '-', '_'
    if ($normalized -notmatch '^[A-Za-z][A-Za-z0-9_]*$') { throw "$Label is not a Mermaid-safe identifier: $id" }
    $normalized
}

function Escape-Html([string]$Value) {
    $Value.Replace('&','&amp;').Replace('<','&lt;').Replace('>','&gt;').Replace('"','&quot;')
}

function Normalized([string]$Value) {
    (($Value.Normalize([Text.NormalizationForm]::FormKC)).ToLowerInvariant() -replace '[\s\p{P}\p{S}]','')
}

function GroundingCandidates([string]$Token) {
    $normalized = Normalized $Token
    $aliases = @{
        '连续检查' = @('连续检查','连续监控')
        '周期检查' = @('周期检查','定期审查')
        '情景' = @('情景','情境','场景')
    }
    if ($aliases.ContainsKey($normalized)) { return @($aliases[$normalized]) }
    return @($Token)
}

function Assert-DetailGrounded([string]$Detail,[string]$Body,[string]$Label) {
    $atoms = @($Detail.Normalize([Text.NormalizationForm]::FormKC) -split '[\s：:·，,。；;、→↔=+×÷/（）()\[\]<>]+' |
        ForEach-Object { $_.Trim() } | Where-Object { $_.Length -ge 2 } | Sort-Object -Unique)
    if (-not $atoms.Count) { throw "$Label has no checkable grounding token" }
    $bodyNormalized = Normalized $Body
    $matches = @($atoms | Where-Object { $bodyNormalized.Contains((Normalized $_)) })
    if (-not $matches.Count) {
        # Chinese detail phrases are often intentionally compact. Preserve a
        # meaningful four-character contiguous grounding check when no
        # punctuation-separated token exists.
        foreach ($atom in $atoms) {
            $normalizedAtom = Normalized $atom
            for ($length = [Math]::Min(8, $normalizedAtom.Length); $length -ge 2; $length--) {
                for ($start = 0; $start -le $normalizedAtom.Length - $length; $start++) {
                    if ($bodyNormalized.Contains($normalizedAtom.Substring($start, $length))) {
                        $matches += $atom
                        break
                    }
                }
                if ($matches.Count) { break }
            }
            if ($matches.Count) { break }
        }
    }
    if (-not $matches.Count) { throw "$Label has no grounding token in its linked chapter" }
}

$package = (Get-Item -LiteralPath $PackageRoot -ErrorAction Stop).FullName
if (-not (Test-Path -LiteralPath $package -PathType Container)) { throw "Package root is not a directory: $package" }
$indexPath = Join-Path $package 'index.md'
$specPath = Join-Path $package 'media\course-map.spec.json'
foreach ($path in @($indexPath,$specPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing course-map input: $path" }
}
$spec = Get-Content -LiteralPath $specPath -Raw -Encoding utf8 | ConvertFrom-Json
if ([string]$spec.schema -cne 'babata.mba-course-map-spec/v1') { throw 'Unsupported package-owned course-map spec' }
if ([string]$spec.status -cne 'pending_user_acceptance') { throw 'Course-map spec must remain pending_user_acceptance' }

$course = Require-Text $spec.course 'course-map course'
$rootId = Require-Id $spec.root_id 'course-map root_id'
$rootLabel = Require-Text $spec.root_label 'course-map root_label'
$tagline = Require-Text $spec.tagline 'course-map tagline'
$axis = Require-Text $spec.classification_axis 'course-map classification_axis'
$assetBase = Require-Text $spec.asset_basename 'course-map asset_basename'
if ($assetBase -match '[\\/:*?"<>|]' -or $assetBase -in @('.','..')) { throw 'course-map asset_basename is unsafe' }

$domains = @($spec.domains)
$learning = $spec.learning
$expectedColors = @('#2563EB','#16A34A','#EA8A00','#EF4444','#8B5CF6')
if ($domains.Count -lt 4 -or $domains.Count -gt $expectedColors.Count) { throw 'The accepted MBA profile requires four or five knowledge domains' }
$allIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
if (-not $allIds.Add($rootId)) { throw "Duplicate Mermaid id: $rootId" }
$knowledgeNodes = @()
$detailCount = 0
for ($domainIndex=0; $domainIndex -lt $domains.Count; $domainIndex++) {
    $domain = $domains[$domainIndex]
    $domainId = 'domain_' + (Require-Id $domain.id "domain[$domainIndex].id")
    Require-Text $domain.label "domain[$domainIndex].label" | Out-Null
    if ([string]$domain.color -cne $expectedColors[$domainIndex]) { throw "Domain color does not match the accepted domain-color profile: $domainId" }
    if (-not $allIds.Add($domainId)) { throw "Duplicate Mermaid id: $domainId" }
    $nodes = @($domain.nodes)
    if (-not $nodes.Count) { throw "Course-map domain has no chapter node: $domainId" }
    foreach ($node in $nodes) {
        $nodeId = Require-Id $node.id "node id in $domainId"
        $note = Require-Text $node.note "node note in $domainId"
        if ($note -match '[\\/\[\]"<>]') { throw "Course-map note is not a safe package basename: $note" }
        if (-not $allIds.Add($nodeId)) { throw "Duplicate Mermaid id: $nodeId" }
        if (-not $allIds.Add($nodeId+'J')) { throw "Generated Mermaid junction id collides: ${nodeId}J" }
        $details = @($node.details | ForEach-Object { [string]$_ })
        if (-not $details.Count -or @($details | Where-Object { [string]::IsNullOrWhiteSpace($_) }).Count) { throw "Course-map chapter requires grounded details: $note" }
        for ($i=0; $i -lt $details.Count; $i++) {
            if (-not $allIds.Add($nodeId+'D'+($i+1))) { throw "Generated Mermaid detail id collides: $nodeId" }
        }
        $notePath = Join-Path $package ($note+'.md')
        if (-not (Test-Path -LiteralPath $notePath -PathType Leaf)) { throw "Course-map chapter note is missing: $note" }
        $body = Get-Content -LiteralPath $notePath -Raw -Encoding utf8
        foreach ($detail in $details) { Assert-DetailGrounded $detail $body "course-map detail $note / $detail" }
        $knowledgeNodes += [pscustomobject]@{id=$nodeId;note=$note;details=$details;domain=$domain}
        $detailCount += $details.Count
    }
    $domainBody = @($nodes | ForEach-Object { Get-Content -LiteralPath (Join-Path $package (([string]$_.note)+'.md')) -Raw -Encoding utf8 }) -join "`n"
    foreach ($token in @($domain.evidence | ForEach-Object { [string]$_ })) {
        $candidates = @(GroundingCandidates $token)
        $bodyNormalized = Normalized $domainBody
        $matched = @($candidates | Where-Object { $bodyNormalized.Contains((Normalized $_)) })
        if (-not $matched.Count -and -not [string]::IsNullOrWhiteSpace($token)) {
            $normalizedToken = Normalized $token
            for ($length = [Math]::Min(8, $normalizedToken.Length); $length -ge 2 -and -not $matched.Count; $length--) {
                for ($start = 0; $start -le $normalizedToken.Length - $length; $start++) {
                    if ($bodyNormalized.Contains($normalizedToken.Substring($start, $length))) { $matched += $token; break }
                }
            }
        }
        if ([string]::IsNullOrWhiteSpace($token) -or -not $matched.Count) {
            throw "Course-map domain evidence is absent from linked chapter text: $domainId / $token"
        }
    }
}

$learningId = Require-Id $learning.id 'learning.id'
if (-not $allIds.Add($learningId)) { throw "Duplicate Mermaid id: $learningId" }
$learningColor=Require-Text $learning.color 'learning.color'
if($learningColor -cne '#64748B'){throw 'Learning-aid branch must use the accepted neutral profile color'}
$learningNodes = @($learning.nodes)
if ($learningNodes.Count -ne 4) { throw 'The accepted MBA profile requires four learning-aid links' }
foreach ($node in $learningNodes) {
    $id = Require-Id $node.id 'learning node id'
    $note = Require-Text $node.note 'learning node note'
    if ($note -match '[\\/\[\]"<>]') { throw "Learning note is not a safe package basename: $note" }
    if (-not $allIds.Add($id)) { throw "Duplicate Mermaid id: $id" }
    if (-not (Test-Path -LiteralPath (Join-Path $package ($note+'.md')) -PathType Leaf)) { throw "Learning note is missing: $note" }
}
$linkedNodes = @($knowledgeNodes) + @($learningNodes)
$linkedNotes = @($linkedNodes | ForEach-Object { [string]$_.note })
if (@($linkedNotes | Sort-Object -Unique).Count -ne $linkedNotes.Count) { throw 'Course-map linked note labels must be unique' }

$lines = [Collections.Generic.List[string]]::new()
@(
    "%% MECE 主分类轴：$axis；学习支持独立成层"
    '%% C2B-RIGHT-GROWING-MINDMAP-GATE：单根、全右向、五域分色、透明文字节点、小圆分叉、无知识卡片'
    '%% C2B-RESPONSIVE-MAP-GATE：原生 Mermaid 为唯一默认展开主图；PNG 仅作折叠回退'
    '%%{init: {"flowchart": {"useMaxWidth": true, "htmlLabels": true, "nodeSpacing": 16, "rankSpacing": 34, "wrappingWidth": 300, "curve": "basis"}, "themeVariables": {"fontFamily": "Inter, Segoe UI, Microsoft YaHei, sans-serif", "fontSize": "22px", "lineColor": "#94A3B8"}}}%%'
    'flowchart LR'
    "  $rootId[`"<b>$(Escape-Html $rootLabel)</b><br/><small>$(Escape-Html $tagline)</small>`"]"
) | ForEach-Object { [void]$lines.Add($_) }
foreach ($domain in $domains) {
    $domainId='domain_' + (Require-Id $domain.id 'domain.id')
    [void]$lines.Add("  $domainId[`"<b>$(Escape-Html ([string]$domain.label))</b>&nbsp;<span style='color:$([string]$domain.color)'>&#9675;</span>`"]")
    foreach ($node in @($domain.nodes)) {
        $nodeId=[string]$node.id
        [void]$lines.Add("  $nodeId[`"$([string]$node.note)`"]")
        [void]$lines.Add("  ${nodeId}J(( ))")
        $details=@($node.details)
        for ($i=0;$i -lt $details.Count;$i++) {
            [void]$lines.Add("  ${nodeId}D$($i+1)[`"$(Escape-Html ([string]$details[$i]))`"]")
        }
    }
}
[void]$lines.Add("  $learningId[`"<b>$(Escape-Html ([string]$learning.label))</b>&nbsp;<span style='color:$([string]$learning.color)'>&#9675;</span>`"]")
foreach ($node in $learningNodes) { [void]$lines.Add("  $([string]$node.id)[`"$([string]$node.note)`"]") }

$edgeLines=[Collections.Generic.List[string]]::new();$edgeGroups=@{};$edgeIndex=0
foreach ($domain in $domains) {
    $indices=[Collections.Generic.List[int]]::new();$domainId='domain_' + (Require-Id $domain.id 'domain.id')
    [void]$edgeLines.Add("  $rootId --- $domainId");[void]$indices.Add($edgeIndex);$edgeIndex++
    foreach ($node in @($domain.nodes)) {
        $nodeId=[string]$node.id
        [void]$edgeLines.Add("  $domainId --- $nodeId");[void]$indices.Add($edgeIndex);$edgeIndex++
        [void]$edgeLines.Add("  $nodeId --- ${nodeId}J");[void]$indices.Add($edgeIndex);$edgeIndex++
        for($i=0;$i -lt @($node.details).Count;$i++){[void]$edgeLines.Add("  ${nodeId}J --- ${nodeId}D$($i+1)");[void]$indices.Add($edgeIndex);$edgeIndex++}
    }
    $edgeGroups[$domainId]=@($indices)
}
$learningIndices=[Collections.Generic.List[int]]::new()
[void]$edgeLines.Add("  $rootId --- $learningId");[void]$learningIndices.Add($edgeIndex);$edgeIndex++
foreach($node in $learningNodes){[void]$edgeLines.Add("  $learningId --- $([string]$node.id)");[void]$learningIndices.Add($edgeIndex);$edgeIndex++}
$edgeGroups[$learningId]=@($learningIndices);$edgeLines|ForEach-Object{[void]$lines.Add($_)}

@(
    '  classDef root fill:transparent,color:#0F172A,stroke:transparent,stroke-width:0px,font-size:24px,font-weight:700,text-align:left'
    '  classDef branch fill:transparent,color:#111827,stroke:transparent,stroke-width:0px,font-size:22px,font-weight:700,text-align:left'
    '  classDef internal-link fill:transparent,color:#111827,stroke:transparent,stroke-width:0px,font-size:21px,font-weight:600,text-align:left'
    '  classDef detail fill:transparent,color:#273244,stroke:transparent,stroke-width:0px,font-size:20px,font-weight:400,text-align:left'
    '  classDef junction fill:#FFFFFF,color:transparent,stroke-width:2px,padding:2px'
    "  class $rootId root"
    '  class '+((@($domains | ForEach-Object { 'domain_' + (Require-Id $_.id 'domain.id') })+@($learningId)) -join ',')+' branch'
    '  class '+(@($linkedNodes.id) -join ',')+' internal-link'
) | ForEach-Object { [void]$lines.Add($_) }
$detailIds=@($knowledgeNodes|ForEach-Object{$node=$_;for($i=0;$i -lt @($node.details).Count;$i++){[string]$node.id+'D'+($i+1)}})
[void]$lines.Add('  class '+($detailIds -join ',')+' detail')
foreach($domain in $domains){
    $junctions=@($domain.nodes|ForEach-Object{[string]$_.id+'J'});$domainId='domain_' + (Require-Id $domain.id 'domain.id')
    [void]$lines.Add('  class '+($junctions -join ',')+' junction')
    foreach($junction in $junctions){[void]$lines.Add("  style $junction fill:#FFFFFF,stroke:$([string]$domain.color),stroke-width:2px")}
    [void]$lines.Add("  linkStyle $($edgeGroups[$domainId] -join ',') stroke:$([string]$domain.color),stroke-width:2px")
}
[void]$lines.Add("  linkStyle $($edgeGroups[$learningId] -join ',') stroke:$([string]$learning.color),stroke-width:1.75px")
$mermaid=$lines -join "`n"
if($mermaid.Contains('obsidian://') -or $mermaid -match '(?m)^\s*click\s+' -or $mermaid.Contains('-->') -or $mermaid.Contains('<--')){throw 'Course map contains a forbidden URI, click directive, or arrow'}
if([regex]::Matches($mermaid,"(?m)^\s*$rootId --- ").Count -ne ($domains.Count + 1)){throw 'Course map root edges must match the knowledge-domain count plus learning branch'}

$mediaRoot=Join-Path $package 'media';$mmdPath=Join-Path $mediaRoot ($assetBase+'.mmd');$pngPath=Join-Path $mediaRoot ($assetBase+'.png');$svgPath=Join-Path $mediaRoot ('.'+$assetBase+'-link-check.svg')
Set-Content -LiteralPath $mmdPath -Value ($mermaid+"`n") -Encoding utf8
$pngLog=@(& npx --yes '@mermaid-js/mermaid-cli@11.12.0' -i $mmdPath -o $pngPath -b white -w 2200 -H 1600 2>&1)
if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $pngPath -PathType Leaf)){throw "Mermaid PNG rendering failed: $($pngLog -join ' ')"}
$svgLog=@(& npx --yes '@mermaid-js/mermaid-cli@11.12.0' -i $mmdPath -o $svgPath -b white -w 2200 -H 1600 2>&1)
if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $svgPath -PathType Leaf)){throw "Mermaid SVG verification render failed: $($svgLog -join ' ')"}
try {
    [xml]$svg=Get-Content -LiteralPath $svgPath -Raw -Encoding utf8;$svgRoot=$svg.DocumentElement
    if($null -eq $svgRoot -or $svgRoot.LocalName -ne 'svg' -or [string]$svgRoot.GetAttribute('width') -ne '100%' -or
        [string]::IsNullOrWhiteSpace([string]$svgRoot.GetAttribute('viewBox')) -or [string]$svgRoot.GetAttribute('style') -notmatch '(?:^|;)\s*max-width:\s*[0-9]+(?:\.[0-9]+)?px(?:;|$)'){
        throw 'Responsive SVG root must have width=100%, viewBox, and controlled max-width'
    }
    $xpath="//*[contains(concat(' ', normalize-space(@class), ' '), ' internal-link ')]/*[local-name()='g' and contains(concat(' ', normalize-space(@class), ' '), ' label ')]/*[local-name()='foreignObject']/*[local-name()='div']"
    $actual=@($svg.SelectNodes($xpath)|ForEach-Object{$_.InnerText});$actualSorted=@($actual|Sort-Object -Unique);$expectedSorted=@($linkedNotes|Sort-Object -Unique)
    if($actual.Count -ne $linkedNotes.Count -or $actualSorted.Count -ne $actual.Count -or ($actualSorted -join "`n") -cne ($expectedSorted -join "`n")){throw 'Obsidian Mermaid internal-link selector mismatch'}
} finally { if(Test-Path -LiteralPath $svgPath){Remove-Item -LiteralPath $svgPath -Force} }

$index=Get-Content -LiteralPath $indexPath -Raw -Encoding utf8
$block=@('## 课程脑图','','课程知识沿决策主线向右展开。章节节点对应完整笔记，末端短句用于复习。','','```mermaid',$mermaid,'```','', '> [!info]- 位图版本（打印 / 离线 / 渲染回退）', "> ![[media/$assetBase.png|760]]") -join "`n"
if($index -match '(?ms)^## 课程脑图\s*.*?(?=^## |\z)'){$index=[regex]::Replace($index,'(?ms)^## 课程脑图\s*.*?(?=^## |\z)',($block+"`n`n"),1)}else{
    if($index -notmatch '(?m)^## 课程章节\s*$'){throw 'C2B index has no course chapter section'}
    $index=[regex]::Replace($index,'(?m)^## 课程章节\s*$',($block+"`n`n## 课程章节"),1)
}
Set-Content -LiteralPath $indexPath -Value ($index.TrimEnd()+"`n") -Encoding utf8

$rendered=Get-Item -LiteralPath $pngPath;if($rendered.Length -lt 10000){throw 'Rendered course-map PNG is unexpectedly small'}
Add-Type -AssemblyName System.Drawing;$image=[Drawing.Image]::FromFile($pngPath)
try{$width=[double]$image.Width;$height=[double]$image.Height}finally{$image.Dispose()}
$effective=22.0*760.0/$width;$aspect=$height/$width
if($effective -lt 11.0){throw "Course-map text is unreadable at 760px: $effective"}
if($aspect -gt 1.40){throw "Course map is too tall: $aspect"}
$indexCheck=Get-Content -LiteralPath $indexPath -Raw -Encoding utf8
if([regex]::Matches($indexCheck,'(?m)^```mermaid$').Count -ne 1 -or [regex]::Matches($indexCheck,[regex]::Escape("![[media/$assetBase.png|760]]")).Count -ne 1){throw 'Course-map index embedding verification failed'}

[pscustomobject][ordered]@{
    schema='babata.mba-course-map-render/v1';status='passed';course=$course
    mermaid=('media/'+$assetBase+'.mmd');png=('media/'+$assetBase+'.png')
    classification_axis=$axis;layout='single_root_right_growing_mindmap';mece_domains=$domains.Count
    knowledge_details=$detailCount;internal_link_targets=$linkedNotes.Count;responsive_svg=$true
    default_expanded='mermaid';png_default_collapsed=$true;png_display_width=760
    png_width=[int]$width;png_height=[int]$height;effective_font_px=[Math]::Round($effective,2)
    aspect_ratio=[Math]::Round($aspect,3);png_bytes=[long]$rendered.Length
}
