param(
    [string]$SkillRoot
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($SkillRoot)) {
    $repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
    $SkillRoot = Join-Path $repo '02_skills\babata-collect'
}
$SkillRoot = (Resolve-Path -LiteralPath $SkillRoot).Path

function Assert-Contains {
    param(
        [Parameter(Mandatory)]
        [string]$Text,
        [Parameter(Mandatory)]
        [string]$Value,
        [Parameter(Mandatory)]
        [string]$Label
    )
    if (-not $Text.Contains($Value)) {
        throw "Collection Skill is missing $Label"
    }
}

function Assert-Matches {
    param(
        [Parameter(Mandatory)]
        [string]$Text,
        [Parameter(Mandatory)]
        [string]$Pattern,
        [Parameter(Mandatory)]
        [string]$Label
    )
    if ($Text -notmatch $Pattern) {
        throw "Collection Skill is missing $Label"
    }
}

$requiredFiles = @(
    'SKILL.md',
    'agents\openai.yaml',
    'references\collection-contract.md',
    'references\route-catalog.md',
    'references\source-onenote.md',
    'references\source-evernote.md',
    'references\source-doubao.md',
    'references\source-browser-and-ui.md'
)
foreach ($relativePath in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $SkillRoot $relativePath) -PathType Leaf)) {
        throw "Collection Skill is missing required file: $relativePath"
    }
}

$unexpectedDirectories = @(Get-ChildItem -LiteralPath (Join-Path $SkillRoot 'references') -Directory -Recurse)
if ($unexpectedDirectories.Count -ne 0) {
    throw 'Collection Skill references must remain one level deep'
}

$skill = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'SKILL.md')
$contract = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'references\collection-contract.md')
$catalog = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'references\route-catalog.md')
$onenote = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'references\source-onenote.md')
$evernote = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'references\source-evernote.md')
$doubao = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'references\source-doubao.md')
$metadata = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $SkillRoot 'agents\openai.yaml')

Assert-Contains $skill 'name: babata-collect' 'the aggregate Skill name'
Assert-Contains $skill 'the single user-visible collection Skill' 'the single-entry description'
Assert-Contains $skill 'babata --json capabilities list' 'runtime capability preflight'
Assert-Contains $skill 'babata --json collector start' 'the unified Collector start command'
Assert-Contains $skill 'babata --json collector cancel' 'cancellation handling'
Assert-Contains $skill 'Never call `babata process`' 'the no-C1 execution boundary'
Assert-Contains $skill 'never create `babata-<platform>-collect`' 'the no-platform-Skill extension rule'
$acquired = ([string][char]0x62FF) + [char]0x56DE
$registered = ([string][char]0x6B63) + [char]0x5F0F + [char]0x767B + [char]0x8BB0
$repeatable = ([string][char]0x957F) + [char]0x671F + [char]0x91CD + [char]0x590D
Assert-Contains $contract "**$acquired**" 'the acquired-material report layer'
Assert-Contains $contract "**$registered**" 'the formal-C0 report layer'
Assert-Contains $contract "**$repeatable**" 'the repeatability report layer'
Assert-Contains $catalog 'babata --json capabilities list' 'runtime capability truth'
Assert-Contains $catalog 'Do not copy their current status or counts into this index.' 'no duplicated route status'
if ($catalog -match '(?m)^\|.*\|\s*(?:enabled|disabled|unavailable|absent)\s*\|\s*$') {
    throw 'Collection Skill route catalog duplicates runtime status'
}
Assert-Matches $catalog '(?m)^\| `source\.onenote` \| `source-onenote\.md` \|' 'OneNote recipe navigation'
Assert-Matches $catalog '(?m)^\| `source\.evernote` \| `source-evernote\.md` \|' 'Evernote recipe navigation'
Assert-Matches $catalog '(?m)^\| `source\.doubao` \| `source-doubao\.md` \|' 'Doubao recipe navigation'
Assert-Contains $onenote 'pair:<absolute-path-to.mht>|<absolute-path-to.pdf>' 'OneNote pair source shape'
Assert-Contains $onenote 'mht-list:<absolute-path-a.mht>|<absolute-path-b.mht>|...' 'OneNote MHT-list source shape'
Assert-Contains $evernote 'notes:<absolute-path-to-export.notes>' 'Evernote source shape'
Assert-Contains $doubao 'Query runtime capability before use' 'Doubao runtime preflight'
Assert-Contains $doubao 'conversations:<id>,<id>,...' 'Doubao explicit batch scope'
Assert-Contains $doubao 'has_more=false' 'Doubao pagination completion condition'
Assert-Contains $doubao 'complete pagination with no repeated/missing message IDs' 'Doubao pagination integrity condition'
Assert-Contains $doubao 'recollect --item <item_id>' 'Doubao per-item recollection boundary'
Assert-Contains $skill 'stable, accurate, real, then fast' 'the stable-accurate-real-fast order'
Assert-Contains $metadata 'Use $babata-collect' 'Skill UI invocation prompt'
Assert-Contains $metadata 'display_name: "Babata Collect"' 'consistent Skill UI name'
$shortDescriptionMatch = [regex]::Match($metadata, '(?m)^\s*short_description:\s*"([^"]+)"\s*$')
if (-not $shortDescriptionMatch.Success -or
    $shortDescriptionMatch.Groups[1].Value.Length -lt 25 -or
    $shortDescriptionMatch.Groups[1].Value.Length -gt 64) {
    throw 'Collection Skill UI short_description must contain 25-64 characters'
}

$commandLines = @($skill -split "`r?`n" | Where-Object {
    $_.Trim() -match '^(babata|cargo\s+run\b).*(process|knowledge)\b'
})
if ($commandLines.Count -ne 0) {
    throw 'Collection Skill contains an executable Process/Knowledge command'
}

foreach ($marker in @('sqlite3 ', 'Connection::open(', 'rusqlite::')) {
    if ($skill.Contains($marker) -or $contract.Contains($marker)) {
        throw "Collection Skill contains a direct persistence marker: $marker"
    }
}

Write-Output 'Collection Skill check passed: one routed entry, runtime-owned capability status, unified C0, no C1 command.'
