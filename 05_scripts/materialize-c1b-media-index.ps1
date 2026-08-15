[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$PackageRoot,[Parameter(Mandatory=$true)][string]$DecisionPath)
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
$pkg=(Get-Item $PackageRoot).FullName; $dec=@(Get-Content $DecisionPath -Raw|ConvertFrom-Json)
$lines=@('# C1B 多媒体证据索引','','本页按模块列出新一轮 C1B 判断保留的视觉证据。完整原件不复制到用户知识层；每个模块的文字证据与 C0 定位见对应证据页。','')
foreach($d in $dec | Sort-Object {[int]$_.module_id}){
  $lines += "## M-$($d.module_id) $($d.title)"
  $lines += "- [[证据/M-$($d.module_id)]]"
  foreach($m in @($d.retained_media)){
    $src=Split-Path $m.path -Leaf; $name="M-$($d.module_id)-$src"; $target="media/$name"
    if(-not(Test-Path (Join-Path $pkg $target))){throw "Missing materialized media: $target"}
    if($name -match '\.(png|jpg|jpeg)$'){$lines += "![$($d.title) $src]($target)"}else{$lines += "[$src]($target)"}
  }
  $lines += ''
}
($lines -join "`n")|Set-Content (Join-Path $pkg '媒体证据索引.md') -Encoding utf8
Write-Output "media_index=created"
