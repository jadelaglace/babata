[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'select-mba-course-c1-candidate.ps1')

function Candidate([string]$Run,[string]$Hash,[string]$Path) {
    [pscustomobject]@{run_id=$Run;output_sha256=$Hash;logical_path=$Path}
}
function Assert-Throws([scriptblock]$Action,[string]$Expected) {
    try { & $Action | Out-Null } catch { if ($_.Exception.Message.Contains($Expected)) { return }; throw }
    throw "Expected rejection containing: $Expected"
}

$hashA='a'*64;$hashB='b'*64;$pathA='02_derived/files/sha256/aa/content-a'
$single=Select-MbaCourseC1Candidate -Candidates @(Candidate 'run-1' $hashA $pathA) -PreferredKind 'extracted_text' -ModuleId '1'
if ($single.run_id -ne 'run-1') { throw 'Single C1 candidate was not selected' }

$duplicate=Select-MbaCourseC1Candidate -Candidates @(
    Candidate 'run-1' $hashA $pathA
    Candidate 'run-2' $hashA $pathA
) -PreferredKind 'extracted_text' -ModuleId '2'
if ($duplicate.run_id -ne 'run-2') { throw 'Same-fingerprint duplicate runs did not collapse to the latest candidate' }
$frozen=Select-MbaCourseC1Candidate -Candidates @(
    Candidate 'run-1' $hashA $pathA
    Candidate 'run-2' $hashA $pathA
) -PreferredKind 'extracted_text' -ModuleId '2' -PreferredRunId 'run-1'
if ($frozen.run_id -ne 'run-1') { throw 'Frozen same-fingerprint run identity was not preserved' }
$divergentFrozen=Select-MbaCourseC1Candidate -Candidates @(
    Candidate 'run-1' $hashA $pathA
    Candidate 'run-2' $hashB '02_derived/files/sha256/bb/content-b'
) -PreferredKind 'ocr_text' -ModuleId '3' -PreferredRunId 'run-1'
if ($divergentFrozen.run_id -ne 'run-1') { throw 'Explicit divergent run identity was not preserved' }
Assert-Throws { Select-MbaCourseC1Candidate -Candidates @(Candidate 'run-1' $hashA $pathA) -PreferredKind 'extracted_text' -ModuleId '2' -PreferredRunId 'run-missing' } 'Expected explicit frozen extracted_text run run-missing'

Assert-Throws { Select-MbaCourseC1Candidate -Candidates @(
    Candidate 'run-1' $hashA $pathA
    Candidate 'run-2' $hashB '02_derived/files/sha256/bb/content-b'
) -PreferredKind 'extracted_text' -ModuleId '3' } '2 divergent fingerprints'
Assert-Throws { Select-MbaCourseC1Candidate -Candidates @() -PreferredKind 'transcript' -ModuleId '4' } 'found 0 candidates'
Assert-Throws { Select-MbaCourseC1Candidate -Candidates @(Candidate 'run-1' '' $pathA) -PreferredKind 'extracted_text' -ModuleId '5' } 'identity is incomplete'

'mba-course-c1-candidate-selection-tests=passed'
