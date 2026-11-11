[CmdletBinding()]
param(
    [hashtable]$Roots,
    [object[]]$Entries = @(),
    [switch]$CheckFilesystem,
    [switch]$SkipDocumentCheck
)

$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$requiredMarkers = @{
    '00_docs/03_architecture/03_a_ARCHITECTURE.md' = @(
        'BABATA_EVIDENCE_HOME',
        'BABATA_RECOVERY_HOME',
        'BABATA_ASSISTANT_HOME',
        '02_derived/',
        '03_views/',
        'staging、模型 workspace、execution ledger、缓存和日志位于 Git 外'
    )
    '02_skills/babata-clean/SKILL.md' = @(
        '04_runtime/staging/model-workspaces/'
    )
    '05_scripts/migrate-auxiliary-data-roots.ps1' = @(
        'babata.auxiliary-root-migration/v1',
        'Refusing to remove unexpected source path'
    )
}

if (-not $SkipDocumentCheck) {
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
        ForEach-Object {
            $path = Join-Path $repo $_
            if (Test-Path -LiteralPath $path -PathType Leaf) {
                $path
            }
        }
)
if (-not $SkipDocumentCheck) {
foreach ($path in $trackedText) {
    $content = Get-Content -LiteralPath $path -Raw -Encoding utf8
    foreach ($marker in $forbidden) {
        if ($content.Contains($marker)) {
            $relative = [IO.Path]::GetRelativePath($repo, $path)
            throw "Legacy auxiliary path remains in ${relative}: $marker"
        }
    }
}

}

function Resolve-ContractPath([string]$Path) {
    if (-not [IO.Path]::IsPathFullyQualified($Path) -or $Path.Split([char[]]@('/', [char]92)) -contains '..') {
        throw 'Contract paths must be absolute without parent traversal.'
    }
    $full = [IO.Path]::GetFullPath($Path)
    $parts = [Collections.Generic.List[string]]::new()
    $cursor = $full
    while (-not (Test-Path -LiteralPath $cursor)) {
        $parts.Add([IO.Path]::GetFileName($cursor))
        $cursor = [IO.Path]::GetDirectoryName($cursor)
        if (-not $cursor) { throw 'Path has no existing ancestor.' }
    }
    $chain = [Collections.Generic.List[string]]::new()
    $scan = $cursor
    while ($scan) {
        $chain.Add($scan)
        $scan = [IO.Path]::GetDirectoryName($scan)
    }
    $resolved = $chain[$chain.Count - 1]
    for ($index = $chain.Count - 2; $index -ge 0; $index--) {
        $resolved = Join-Path $resolved ([IO.Path]::GetFileName($chain[$index]))
        $item = Get-Item -LiteralPath $resolved -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            $resolved = $item.ResolveLinkTarget($true).FullName
        }
    }
    for ($index = $parts.Count - 1; $index -ge 0; $index--) { $resolved = Join-Path $resolved $parts[$index] }
    return [IO.Path]::GetFullPath($resolved).TrimEnd([char[]]@('/', [char]92))
}

function Test-Overlap([string]$Left, [string]$Right) {
    $separator = [IO.Path]::DirectorySeparatorChar
    return $Left.Equals($Right, [StringComparison]::OrdinalIgnoreCase) -or
        $Left.StartsWith($Right + $separator, [StringComparison]::OrdinalIgnoreCase) -or
        $Right.StartsWith($Left + $separator, [StringComparison]::OrdinalIgnoreCase)
}

if ($null -ne $Roots) {
    $aliases = @('data', 'evidence', 'recovery', 'assistant')
    if (@($Roots.Keys | Where-Object { $_ -notin $aliases }).Count -or $Roots.Count -ne 4) {
        throw 'Exactly four aliases are required: data/evidence/recovery/assistant.'
    }
    $resolved = @{}
    foreach ($alias in $aliases) {
        $path = Resolve-ContractPath $Roots[$alias]
        if (Test-Overlap $path (Resolve-ContractPath $repo)) { throw 'Runtime roots must stay outside Git.' }
        $ancestor = $path
        while ($ancestor) {
            if (Test-Path -LiteralPath (Join-Path $ancestor '.git')) { throw 'Runtime root is inside another Git checkout.' }
            $ancestor = [IO.Path]::GetDirectoryName($ancestor)
        }
        foreach ($other in $resolved.Values) {
            if (Test-Overlap $path $other) { throw 'Runtime roots overlap.' }
        }
        if ($CheckFilesystem -and -not (Test-Path -LiteralPath $path -PathType Container)) {
            throw "Missing runtime root: $alias"
        }
        $resolved[$alias] = $path
    }
    $partitions = @('00_inbox','01_raw','02_derived','03_views','04_runtime','05_logs')
    if ($CheckFilesystem) {
        foreach ($item in Get-ChildItem -LiteralPath $Roots.data -Force) {
            if ($item.Name -notin $partitions -and $item.Name -notmatch '^README(?:\..*)?$') {
                throw "Unexpected data partition: $($item.Name)"
            }
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Data partition is a junction/symlink.' }
        }
        foreach ($name in $partitions) {
            if (-not (Test-Path -LiteralPath (Join-Path $Roots.data $name) -PathType Container)) {
                throw "Missing data partition: $name"
            }
        }
    }
    foreach ($entry in $Entries) {
        if ($entry.root_alias -notin $aliases) { throw 'Unknown entry root alias.' }
        $relative = [string]$entry.relative_path
        if (-not $relative -or [IO.Path]::IsPathRooted($relative) -or
            @($relative.Split([char[]]@('/', [char]92)) | Where-Object { $_ -in @('', '.', '..') }).Count) {
            throw 'Unsafe relative path.'
        }
        $target = Resolve-ContractPath (Join-Path $Roots[$entry.root_alias] $relative)
        $root = $resolved[$entry.root_alias]
        if (-not $target.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Entry escapes its declared root.'
        }
        if ($entry.root_alias -eq 'data' -and ($relative -split '[/\\]')[0] -notin $partitions) {
            throw 'Entry uses an invalid data partition.'
        }
        if ($entry.root_alias -eq 'assistant' -and $relative -notmatch '^solutions[/\\][^/\\]+[/\\](runtime|workspaces)[/\\].+') {
            throw 'Assistant entry must use a solution runtime/workspaces layout.'
        }
    }
}
elseif ($CheckFilesystem -or $Entries.Count) { throw 'Filesystem/entry validation requires explicit four roots.' }

Write-Output 'Data-root boundary check passed: four-root contract; explicit filesystem and entry checks when requested.'
