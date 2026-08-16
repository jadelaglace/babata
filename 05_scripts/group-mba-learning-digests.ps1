function Group-MbaLearningDigestNodes {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)][AllowEmptyCollection()][object[]]$Nodes,
        [Parameter(Mandatory=$true)][int]$CharBudget
    )

    if ($CharBudget -lt 1000) { throw 'Digest grouping character budget must be at least 1000' }
    if ($Nodes.Count -eq 0) { throw 'Digest grouping requires at least one node' }

    $groups = [Collections.Generic.List[object]]::new()
    $current = [Collections.Generic.List[object]]::new()
    $currentChars = 0
    foreach ($node in $Nodes) {
        $text = [string]$node.text
        $modules = @($node.modules | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        if ([string]::IsNullOrWhiteSpace($text) -or $modules.Count -eq 0) {
            throw 'Digest node requires text and at least one module identity'
        }
        $nodeChars = $text.Length + 200
        if ($nodeChars -gt $CharBudget) {
            throw "Single digest node exceeds reduction character budget: $($node.id)"
        }
        if ($current.Count -and ($currentChars + $nodeChars) -gt $CharBudget) {
            $groups.Add([pscustomobject]@{nodes=@($current);chars=$currentChars})
            $current = [Collections.Generic.List[object]]::new()
            $currentChars = 0
        }
        $current.Add($node)
        $currentChars += $nodeChars
    }
    if ($current.Count) { $groups.Add([pscustomobject]@{nodes=@($current);chars=$currentChars}) }
    return $groups.ToArray()
}
