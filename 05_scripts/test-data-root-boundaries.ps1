$ErrorActionPreference = 'Stop'
$checker = Join-Path $PSScriptRoot 'check-data-root-boundaries.ps1'
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('babata-root-contract-' + [guid]::NewGuid())
function Expect-Failure([scriptblock]$Action, [string]$Reason) {
    $failed = $false
    try { & $Action | Out-Null } catch { $failed = $_.Exception.Message.Contains($Reason) }
    if (-not $failed) { throw "Expected failure: $Reason" }
}
try {
    $roots = @{}
    foreach ($alias in @('data','evidence','recovery','assistant')) {
        $roots[$alias] = Join-Path $fixture $alias
        New-Item -ItemType Directory -Path $roots[$alias] -Force | Out-Null
    }
    foreach ($name in @('00_inbox','01_raw','02_derived','03_views','04_runtime','05_logs')) {
        New-Item -ItemType Directory -Path (Join-Path $roots.data $name) | Out-Null
    }
    & $checker -Roots $roots -CheckFilesystem -SkipDocumentCheck | Out-Null
    & $checker -Roots $roots -Entries @(@{root_alias='data';relative_path='04_runtime/staging/model-workspaces/old/results/file.json'},
        @{root_alias='assistant';relative_path='solutions/wbs/workspaces/run/payload.json'}) -SkipDocumentCheck | Out-Null
    $nested = $roots.Clone(); $nested.assistant = Join-Path $roots.data 'child'
    Expect-Failure { & $checker -Roots $nested -SkipDocumentCheck } 'overlap'
    $insideGit = $roots.Clone(); $insideGit.assistant = Join-Path $PSScriptRoot 'private'
    Expect-Failure { & $checker -Roots $insideGit -SkipDocumentCheck } 'outside Git'
    Expect-Failure { & $checker -Roots $roots -Entries @(@{root_alias='data';relative_path='../escape'}) -SkipDocumentCheck } 'Unsafe relative'
    Expect-Failure { & $checker -Roots $roots -Entries @(@{root_alias='data';relative_path='02_processed/file'}) -SkipDocumentCheck } 'invalid data partition'
    Expect-Failure { & $checker -Roots $roots -Entries @(@{root_alias='assistant';relative_path='runtime/script.ps1'}) -SkipDocumentCheck } 'solution runtime'
    New-Item -ItemType Directory -Path (Join-Path $roots.data 'tmp') | Out-Null
    Expect-Failure { & $checker -Roots $roots -CheckFilesystem -SkipDocumentCheck } 'Unexpected data partition'
    if ($IsWindows) {
        $link = Join-Path $roots.assistant 'alias'
        New-Item -ItemType Junction -Path $link -Target $roots.data | Out-Null
        $aliased = $roots.Clone(); $aliased.assistant = Join-Path $link 'new'
        Expect-Failure { & $checker -Roots $aliased -SkipDocumentCheck } 'overlap'
        Expect-Failure { & $checker -Roots $roots -Entries @(@{root_alias='assistant';relative_path='alias/01_raw/file'}) -SkipDocumentCheck } 'escapes'
        [IO.Directory]::Delete($link)
    }
    Write-Output 'Four-root negative tests passed: overlap, Git, traversal, partition, layout, legacy and junction.'
}
finally {
    $resolved = [IO.Path]::GetFullPath($fixture)
    if ([IO.Path]::GetDirectoryName($resolved).TrimEnd([char]92) -ne [IO.Path]::GetTempPath().TrimEnd([char]92)) {
        throw 'Unsafe fixture cleanup path.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
