<#
.SYNOPSIS
    QuiTa installer — Windows.

.EXAMPLE
    irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex

.DESCRIPTION
    Downloads the MSI that matches your CPU, verifies its SHA-256 against the
    checksum file published with the same release, then installs it silently.

.EXAMPLE
    irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex -Version 1.0.0
#>

[CmdletBinding()]
param(
  [string]$Version = 'latest'
)

$ErrorActionPreference = 'Stop'
$Repo = if ($env:QUITA_REPO) { $env:QUITA_REPO } else { 'zeroscrypt/QuiTa' }
$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x86' }

function Write-Step($m) { Write-Host "==> $m" -ForegroundColor Green }
function Write-Info($m) { Write-Host "    $m" -ForegroundColor DarkGray }

$headers = @{ 'User-Agent' = 'QuiTa-installer' }

Write-Step 'Looking up the latest release'
if ($Version -eq 'latest') {
  $release = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases/latest"
  $tag = $release.tag_name
  $assets = $release.assets
} else {
  $tag = if ($Version.StartsWith('v')) { $Version } else { "v$Version" }
  $assets = (Invoke-RestMethod -Headers $headers `
    -Uri "https://api.github.com/repos/$Repo/releases/tags/$tag").assets
}
Write-Info $tag

$asset = $assets | Where-Object { $_.name -like "*_${Arch}_en-US.msi" } | Select-Object -First 1
if (-not $asset) { throw "no Windows build for $Arch in $tag" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("quita-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  $file = Join-Path $tmp $asset.name
  Write-Step "Downloading $($asset.name)"
  Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $file -UseBasicParsing

  Write-Step 'Verifying SHA-256'
  $sumsFile = Join-Path $tmp 'SHA256SUMS'
  try {
    Invoke-WebRequest -Uri "https://github.com/$Repo/releases/download/$tag/SHA256SUMS" `
      -OutFile $sumsFile -UseBasicParsing
    $expected = (Select-String -Path $sumsFile -Pattern " $($asset.name)$").Line.Split()[0]
  } catch {
    Write-Warning 'no checksum published for this build — continuing without verification'
  }
  if ($expected) {
    $actual = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected.ToLower()) { throw 'checksum mismatch — refusing to install' }
    Write-Info 'hash matches'
  }

  Write-Step 'Installing'
  $p = Start-Process msiexec.exe -ArgumentList '/i', "`"$file`"", '/quiet', '/norestart' -Wait -PassThru
  if ($p.ExitCode -ne 0) { throw "msiexec failed with exit code $($p.ExitCode)" }
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

Write-Host "`nQuiTa installed.`n" -ForegroundColor Green