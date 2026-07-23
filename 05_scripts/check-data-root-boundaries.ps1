$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$requiredMarkers = @{
    '00_docs/03_architecture/03_ARCHITECTURE.md' = @(
        'BABATA_EVIDENCE_HOME',
        'BABATA_RECOVERY_HOME',
        '04_runtime/staging/model-workspaces/'
    )
    '02_skills/babata-bailian-clean/SKILL.md' = @(
        '04_runtime/staging/model-workspaces/'
    )
    '05_scripts/migrate-auxiliary-data-roots.ps1' = @(
        'babata.auxiliary-root-migration/v1',
        'Refusing to remove unexpected source path'
    )
}

foreach ($relative in $requiredMarkers.Keys) {
    $path = Join-Path $repo $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing data-root boundary file: $relative"
    }
    $content = Get-Content -LiteralPath $path -Raw -Encoding utf8
    foreach ($marker in $requiredMarkers[$relative]) {
        if (-not $content.Contains($marker)) {
            throw "Missing data-root boundary marker in ${relative}: $marker"
        }
    }
}

$forbidden = @(
    'BABATA_DATA_HOME/verification',
    'BABATA_DATA_HOME}/verification',
    'BABATA_DATA_HOME/recovery-staging',
    'BABATA_DATA_HOME}/recovery-staging',
    'BABATA_DATA_HOME/generated',
    'BABATA_DATA_HOME}/generated',
    'BabataData\verification',
    'BabataData\recovery-staging'
)
$trackedText = @(
    git -C $repo ls-files |
        Where-Object {
            ($_ -eq 'README.md' -or
                $_.StartsWith('00_docs/') -or
                $_.StartsWith('02_skills/') -or
                $_.StartsWith('06_config/')) -and
            $_ -match '\.(?:md|toml|ps1)$'
        } |
        ForEach-Object { Join-Path $repo $_ }
)
foreach ($path in $trackedText) {
    $content = Get-Content -LiteralPath $path -Raw -Encoding utf8
    foreach ($marker in $forbidden) {
        if ($content.Contains($marker)) {
            $relative = [IO.Path]::GetRelativePath($repo, $path)
            throw "Legacy auxiliary path remains in ${relative}: $marker"
        }
    }
}

Write-Output 'Data-root boundary check passed: active, evidence, recovery, and model-workspace paths are separated.'
