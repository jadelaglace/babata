[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$PackageRoot,[Parameter(Mandatory=$true)][string]$DecisionPath)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
$pkg=(Get-Item $PackageRoot).FullName; $ev=Join-Path $pkg '证据'; New-Item -ItemType Directory -Path $ev -Force|Out-Null
$dec=@(Get-Content $DecisionPath -Raw|ConvertFrom-Json)
foreach($d in $dec){
  $lines=@('---',"babata_type: c1b_evidence_locator","module_id: M-$($d.module_id)",'variant: c1b','---','',"# $($d.title)",'',"- 完整文字 C1B：$($d.c1b_text_path)","- C1 SHA-256：$($d.c1_sha256)","- C0 item：$($d.c0_item_id)","- C0 revision：$($d.c0_revision_id)","- C0 asset：$($d.c0_asset_id)",'')
  foreach($m in @($d.retained_media)){ $src=Split-Path $m.path -Leaf; $lines += "- 媒体证据：../media/M-$($d.module_id)-$src" }
  ($lines -join "`n")|Set-Content (Join-Path $ev ("M-$($d.module_id).md")) -Encoding utf8
}
$missing=@(); foreach($f in Get-ChildItem $pkg -Recurse -File -Filter '*.md'){
  $t=Get-Content $f -Raw
  foreach($m in [regex]::Matches($t,'\[\[([^\]|#]+)')){
    $target=$m.Groups[1].Value.Replace('/','\'); $candidate=if($target -match '^M-'){Join-Path $ev ($target+'.md')}else{Join-Path $pkg ($target+'.md')}; if(-not(Test-Path $candidate)){$missing += "$($f.Name):$target"}
  }
}
if($missing.Count){throw "Dangling links: $($missing -join ',')"}
Write-Output "evidence_pages=$(@(Get-ChildItem $ev -File).Count) missing=0"
