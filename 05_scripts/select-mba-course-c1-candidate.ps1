function Select-MbaCourseC1Candidate {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)][AllowEmptyCollection()][object[]]$Candidates,
        [Parameter(Mandatory=$true)][string]$PreferredKind,
        [Parameter(Mandatory=$true)][string]$ModuleId,
        [string]$PreferredRunId
    )

    $active = @($Candidates | Where-Object { $null -ne $_ })
    if ($active.Count -eq 0) {
        throw "Expected one active $PreferredKind content fingerprint for module $ModuleId, found 0 candidates"
    }
    foreach ($candidate in $active) {
        if ([string]$candidate.output_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
            [string]::IsNullOrWhiteSpace([string]$candidate.logical_path)) {
            throw "Active $PreferredKind candidate identity is incomplete for module $ModuleId"
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($PreferredRunId)) {
        $preferred = @($active | Where-Object { [string]$_.run_id -ceq $PreferredRunId })
        if ($preferred.Count -ne 1) {
            throw "Expected explicit frozen $PreferredKind run $PreferredRunId for module $ModuleId among active candidates"
        }
        return $preferred[0]
    }

    $fingerprints = @($active | Group-Object { [string]$_.output_sha256 })
    if ($fingerprints.Count -ne 1) {
        throw "Expected one active $PreferredKind content fingerprint for module $ModuleId, found $($fingerprints.Count) divergent fingerprints across $($active.Count) candidates"
    }

    # Repeated successful runs with the same managed content are one C1 identity.
    return $active[-1]
}
