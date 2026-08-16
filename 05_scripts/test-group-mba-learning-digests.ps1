[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'group-mba-learning-digests.ps1')

function Assert-Throws([scriptblock]$Action,[string]$Expected) {
    try { & $Action | Out-Null } catch { if ($_.Exception.Message.Contains($Expected)) { return }; throw }
    throw "Expected rejection containing: $Expected"
}

$nodes = @(1..50 | ForEach-Object {
    [pscustomobject]@{id="digest-$_";text=('x' * 8000);modules=@("M$_")}
})
$groups = @(Group-MbaLearningDigestNodes -Nodes $nodes -CharBudget 100000)
if ($groups.Count -le 1 -or $groups.Count -ge $nodes.Count) { throw 'Large digest set was not reduced into bounded groups' }
if (@($groups | Where-Object chars -gt 100000).Count) { throw 'Digest group exceeded its character budget' }
$grouped = @($groups.nodes)
if ($grouped.Count -ne 50) { throw 'Digest grouping lost or duplicated nodes' }
if ((@($grouped.modules | Sort-Object -Unique) -join ',') -cne (@(1..50 | ForEach-Object { "M$_" } | Sort-Object) -join ',')) {
    throw 'Digest grouping changed module coverage'
}

Assert-Throws { Group-MbaLearningDigestNodes -Nodes @() -CharBudget 100000 } 'at least one node'
Assert-Throws { Group-MbaLearningDigestNodes -Nodes @([pscustomobject]@{id='large';text=('x' * 2000);modules=@('M1')}) -CharBudget 1000 } 'exceeds reduction'
Assert-Throws { Group-MbaLearningDigestNodes -Nodes @([pscustomobject]@{id='empty';text='';modules=@('M1')}) -CharBudget 100000 } 'requires text'

'mba-learning-digest-grouping-tests=passed'
