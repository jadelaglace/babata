$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$path = Join-Path $PSScriptRoot '../02_skills/babata-collect/scripts/acquire-doubao-originals.ps1'
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Resolve-Path $path),[ref]$tokens,[ref]$errors)
if ($errors.Count -ne 0) { throw 'Invalid acquisition helper syntax.' }
$functionAst = $ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Invoke-OpenCliJson'},$true)
. ([scriptblock]::Create($functionAst.Extent.Text))
function opencli { $global:LASTEXITCODE = $script:TestExit; return $script:TestJson }
$script:TestExit = 0
foreach ($json in @('[]','[{"Id":"1"}]','{"ok":true,"data":[]}','{"ok":true,"data":[{"Id":"1"}]}')) {
    $script:TestJson=$json
    $value=Invoke-OpenCliJson -Arguments @('fixture') -ExpectArray
    if ($value -isnot [array]) { throw 'Lost array identity.' }
    if ($json.Contains('Id') -and ($value.Count -ne 1 -or $value[0].Id -ne '1')) { throw 'Changed array content.' }
}
$script:TestJson='{"ok":true,"data":{"code":0,"data":{"file_urls":[{"uri":"original.docx"}]}}}'
$value=Invoke-OpenCliJson -Arguments @('fixture')
if ($value.code -ne 0 -or $value.data.file_urls[0].uri -ne 'original.docx') { throw 'Browser object envelope was not normalized.' }
foreach ($json in @('{"ok":false,"data":[]}','{"ok":true}','{"ok":"true","data":[]}','{"unexpected":[]}','null','broken')) {
    $script:TestJson=$json
    $rejected=$false
    try { Invoke-OpenCliJson -Arguments @('fixture') -ExpectArray | Out-Null } catch { $rejected=$true }
    if (-not $rejected) { throw 'Malformed response was accepted.' }
}
$script:TestExit=1
$script:TestJson='[]'
$rejected=$false
try { Invoke-OpenCliJson -Arguments @('fixture') -ExpectArray | Out-Null } catch { $rejected=$true }
if (-not $rejected) { throw 'Nonzero process exit was accepted.' }
Write-Output 'Doubao OpenCLI JSON contracts passed: arrays, v2 browser objects, errors, and malformed responses.'
