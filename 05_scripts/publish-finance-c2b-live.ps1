[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$PackageRoot,[Parameter(Mandatory=$true)][string]$LiveVaultPath,[Parameter(Mandatory=$true)][string]$ArchiveRoot)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
function GetHash([string]$path){(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
$pkg=(Get-Item $PackageRoot).FullName; $live=[IO.Path]::GetFullPath($LiveVaultPath); $archive=[IO.Path]::GetFullPath($ArchiveRoot); New-Item -ItemType Directory -Path $archive -Force|Out-Null
$manifestPath=Join-Path (Split-Path $pkg -Parent) 'manifest.json';if(-not(Test-Path -LiteralPath $manifestPath -PathType Leaf)){throw "formal package manifest missing: $manifestPath"};$manifest=Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8|ConvertFrom-Json
if($manifest.status -ne 'accepted_benchmark' -or $manifest.formal_registration -ne 'registered'){throw 'publisher accepts only an accepted_benchmark / registered C2B package'}
if($manifest.c1b_registration.status -ne 'registered' -or @($manifest.c1b_registration.decision_derivative_ids).Count -ne 37 -or @($manifest.c1b_registration.media_derivative_ids).Count -ne 76){throw 'publisher requires formal C1B coverage: 37 decisions and 76 media derivatives'}
if($manifest.knowledge_universe.status -ne 'registered' -or @($manifest.knowledge_universe.semantic_ids).Count -ne 37){throw 'publisher requires 37 formally registered knowledge-universe entries'}
if($manifest.obsidian_template.profile -ne 'semantic-obsidian/v1' -or $manifest.obsidian_template.status -ne 'accepted'){throw 'publisher requires the accepted semantic-obsidian/v1 profile'}
$src=Get-ChildItem $pkg -Recurse -File;$declared=@{};foreach($row in @($manifest.package_files)){$declared[[string]$row.path]=[string]$row.sha256};if($src.Count -ne $declared.Count){throw 'formal package manifest file count mismatch'}
foreach($f in $src){$rel=$f.FullName.Substring($pkg.Length).TrimStart('\').Replace('\','/');if(-not $declared.ContainsKey($rel)){throw "formal package manifest missing file: $rel"};if((GetHash ([string]$f.FullName)) -ne $declared[$rel]){throw "formal package manifest hash mismatch: $rel"}}
$index=Get-Content -LiteralPath (Join-Path $pkg 'index.md') -Raw -Encoding utf8;foreach($token in @('status: accepted_benchmark','formal_registration: registered','c1b_registration: registered','knowledge_universe_registration: registered','template_profile: semantic-obsidian/v1','template_status: accepted')){if(-not $index.Contains($token)){throw "formal Obsidian index missing: $token"}}
$candidate="$live.publish-candidate"; if(Test-Path $candidate){Remove-Item $candidate -Recurse -Force}; New-Item -ItemType Directory -Path $candidate -Force|Out-Null; Get-ChildItem $pkg -Force|Copy-Item -Destination $candidate -Recurse -Force
$dst=Get-ChildItem $candidate -Recurse -File; if($src.Count -ne $dst.Count){throw 'publish count mismatch'}
foreach($f in $src){$rel=$f.FullName.Substring($pkg.Length).TrimStart('\');$g=Join-Path $candidate $rel;if((GetHash ([string]$f.FullName)) -ne (GetHash ([string]$g))){throw "publish hash mismatch: $rel"}}
if(Test-Path $live){Move-Item $live (Join-Path $archive ('finance-live-before-final-'+(Get-Date -Format yyyyMMdd-HHmmss)))}; Move-Item $candidate $live
Write-Output "published=$live files=$($dst.Count)"
