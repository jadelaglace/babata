param(
    [string]$DocsRoot
)

$ErrorActionPreference = 'Stop'

function Get-Utf8Sha256 {
    param(
        [Parameter(Mandatory)]
        [string]$Value
    )

    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Value)
        return ([BitConverter]::ToString($sha256.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
    }
}

if ([string]::IsNullOrWhiteSpace($DocsRoot)) {
    $DocsRoot = Join-Path $PSScriptRoot '..\00_docs'
}
$docs = (Resolve-Path -LiteralPath $DocsRoot).Path
$requirementsPath = Join-Path $docs '00_requirements\00_REQUIREMENTS.md'
$blueprintPath = Join-Path $docs '03_architecture\09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md'

foreach ($path in @($requirementsPath, $blueprintPath)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing provenance authority document: $path"
    }
}

$authorityFiles = @(Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md')
$uuidPattern = '(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b'
$sensitivePatterns = [ordered]@{
    'private key' = '-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'
    'bearer token' = '(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{20,}'
    'GitHub token' = '\bgh[pousr]_[A-Za-z0-9]{20,}\b'
    'OpenAI-style key' = '\bsk-[A-Za-z0-9]{20,}\b'
    'AWS access key' = '\b(?:AKIA|ASIA)[A-Z0-9]{16}\b'
    'Alibaba access key' = '\bLTAI[A-Za-z0-9]{16,}\b'
}

foreach ($file in $authorityFiles) {
    $content = Get-Content -Raw -Encoding utf8 -LiteralPath $file.FullName
    if ($content -match $uuidPattern) {
        throw "Authority document contains a UUID-shaped local/runtime identifier: $($file.FullName)"
    }
    foreach ($entry in $sensitivePatterns.GetEnumerator()) {
        if ($content -match $entry.Value) {
            throw "Authority document contains a possible $($entry.Key): $($file.FullName)"
        }
    }
}

$requiredVerbatimHashes = @(
    # P6 decisions 1 through 9.
    'ef88a5a23026a980540f196a3342da8bf2ebcc24d2c43f3da113b6a419365581',
    '922db9477e08441144309fd7efb910d28da10bb050e8e14704880ccee1916b4a',
    '1c1594f3536cd7b17a7af4d0ed66baa7ff373cfd42d0c82c5cd52e0cede7a9c8',
    '7a2e755c2361eaa0c830f45e8e11e5ba6754e7ae1e95c7fe27a5896a2d32f3af',
    'd2438ce47dd3027c85483d490eb3a92a9653340624fb7b886beaa998d1ca97d3',
    'ce34de866ccf28168023f5d33d5ce9e8679b07e6dcd3326bd8fa23bcc4a2b955',
    'd9ec256f71052346545d6f6467c8f36939f45a0d6b7debd5114df2944a3586ef',
    'd542954974569671ee1a555b8d4a46412884ae85ca4ed0e639cdbe63482333f4',
    'f6210dee1b0bc49d92da5ecf74e3df30a4c64436c6e95c3d75c12127274ccdcf',
    # Foundation confirmation, revision correction, and provenance correction.
    '85c3edae64394ac8ebd49b5379509e90ce2b8d26b6bdfddcc6373179b49e34fb',
    'a0e871e8f189b34d5fc394e5f4114eac8cc3f4f5bd27a3421d0e28f8031722ba',
    '928468d2b0e9896d9a8612dad1226d0652b8c603eb50075e66bf09f78f3f2504',
    '881dae0e021c1e6fdcb00767317aa05f6fe594813b94e06447d0a0f96ad2a097',
    'd3e5f82c02a8c89697bb241af5507eae0565b3b1cdd28b25580a397556d3d53c',
    # 2026-07-21 local-session provenance governance.
    '2ab9e0945fa8f7919e0a43ceb88bb60d52524330a35bfc3d5254de82f465590e',
    'b04870df50a08cb2b44c4b76df75567c63c9554e8abd4a84a1a87b6827006627',
    '7ab28bb2085e93ec382d58fda214a0491de0667e9bb7c982acfd864c4e527521',
    '0b08c5740a918c2afce51f14f4361c85e133eb36b00e11c28d457402be66e0c1',
    '10e53d50bf3ed4ccecafacdb424d64adae4e85b4775859318b0d43398979000b',
    '224af42c8c5488acf2ff52c7bf89eb82a70f72f9c7f118f2419c9105408e9871'
)
$requirementsLines = @(Get-Content -Encoding utf8 -LiteralPath $requirementsPath)
$verbatimHashes = @{}
foreach ($line in $requirementsLines) {
    if ($line.StartsWith('> ')) {
        $verbatimHashes[(Get-Utf8Sha256 -Value $line)] = $true
    }
}
foreach ($hash in $requiredVerbatimHashes) {
    if (-not $verbatimHashes.ContainsKey($hash)) {
        throw "Requirements are missing or changed a required verbatim excerpt: sha256=$hash"
    }
}

$blueprint = Get-Content -Raw -Encoding utf8 -LiteralPath $blueprintPath
$requiredBlueprintMarkers = @(
    '<!-- P6-TREE-PROVENANCE: builder-summary; user-confirmed; not-verbatim -->',
    '<!-- P6-DECISION-PROVENANCE: builder-questions; user-answers-in-requirements; not-verbatim -->'
)
foreach ($marker in $requiredBlueprintMarkers) {
    if (-not $blueprint.Contains($marker)) {
        throw "P6 blueprint is missing provenance distinction: $marker"
    }
}

$decisionRows = @([regex]::Matches($blueprint, '(?m)^\|\s*([1-9])\s*\|'))
if ($decisionRows.Count -ne 9) {
    throw "P6 blueprint must contain exactly nine numbered decision rows, found $($decisionRows.Count)"
}
$decisionNumbers = @($decisionRows | ForEach-Object { [int]$_.Groups[1].Value } | Sort-Object -Unique)
if (($decisionNumbers -join ',') -ne '1,2,3,4,5,6,7,8,9') {
    throw 'P6 blueprint decision rows must cover 1 through 9 exactly once'
}

Write-Output 'Document provenance passed: required P6 user wording hashes match, Builder questions and confirmed conclusions are distinguished, authority docs contain no UUID-shaped runtime IDs, and sensitive token patterns are absent.'
