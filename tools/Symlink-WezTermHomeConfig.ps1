param(
  [string]$RepoRoot = "C:\Users\david\wezterm",
  [string]$HomeRoot = "C:\Users\david"
)

$ErrorActionPreference = "Stop"

$repoConfigRoot = Join-Path $RepoRoot "config\wezterm"
$liveConfigRoot = Join-Path $HomeRoot ".config\wezterm"
$repoWezTermLua = Join-Path $RepoRoot ".wezterm.lua"
$liveWezTermLua = Join-Path $HomeRoot ".wezterm.lua"

function Ensure-Directory {
  param([string]$Path)
  New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Replace-WithSymlink {
  param(
    [string]$LinkPath,
    [string]$TargetPath,
    [ValidateSet("SymbolicLink")]
    [string]$ItemType = "SymbolicLink"
  )

  if (Test-Path -LiteralPath $LinkPath) {
    Remove-Item -LiteralPath $LinkPath -Recurse -Force
  }

  New-Item -ItemType $ItemType -Path $LinkPath -Target $TargetPath | Out-Null
}

Ensure-Directory $repoConfigRoot
Ensure-Directory $liveConfigRoot

if ((Test-Path -LiteralPath $liveWezTermLua) -and -not (Test-Path -LiteralPath $repoWezTermLua)) {
  Move-Item -LiteralPath $liveWezTermLua -Destination $repoWezTermLua -Force
}

Get-ChildItem -LiteralPath $liveConfigRoot -File | Where-Object { $_.Name -ne "wezterm-utils.lua" } | ForEach-Object {
  $dest = Join-Path $repoConfigRoot $_.Name
  if (-not (Test-Path -LiteralPath $dest)) {
    Move-Item -LiteralPath $_.FullName -Destination $dest -Force
  }
  else {
    Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
    Remove-Item -LiteralPath $_.FullName -Force
  }
}

$liveCodexUi = Join-Path $liveConfigRoot "codex_ui"
$repoCodexUi = Join-Path $RepoRoot "codex_ui"
if (Test-Path -LiteralPath $liveCodexUi) {
  if (-not (Test-Path -LiteralPath $repoCodexUi)) {
    Move-Item -LiteralPath $liveCodexUi -Destination $repoCodexUi -Force
  }
  else {
    Copy-Item -Path (Join-Path $liveCodexUi "*") -Destination $repoCodexUi -Recurse -Force
    Remove-Item -LiteralPath $liveCodexUi -Recurse -Force
  }
}

$liveUtilsLua = Join-Path $liveConfigRoot "wezterm-utils.lua"
$repoUtilsLua = Join-Path $RepoRoot "wezterm-utils.lua"
if (Test-Path -LiteralPath $liveUtilsLua) {
  Copy-Item -LiteralPath $liveUtilsLua -Destination $repoUtilsLua -Force
  Remove-Item -LiteralPath $liveUtilsLua -Force
}

$liveUtilsDir = Join-Path $liveConfigRoot "wezterm-utils"
$repoUtilsDir = Join-Path $RepoRoot "wezterm-utils"
if (Test-Path -LiteralPath $liveUtilsDir) {
  Copy-Item -Path (Join-Path $liveUtilsDir "*") -Destination $repoUtilsDir -Recurse -Force
  Remove-Item -LiteralPath $liveUtilsDir -Recurse -Force
}

Replace-WithSymlink -LinkPath $liveWezTermLua -TargetPath $repoWezTermLua

Get-ChildItem -LiteralPath $repoConfigRoot -File | ForEach-Object {
  Replace-WithSymlink -LinkPath (Join-Path $liveConfigRoot $_.Name) -TargetPath $_.FullName
}

Replace-WithSymlink -LinkPath $liveCodexUi -TargetPath $repoCodexUi
Replace-WithSymlink -LinkPath $liveUtilsLua -TargetPath $repoUtilsLua
Replace-WithSymlink -LinkPath $liveUtilsDir -TargetPath $repoUtilsDir

Write-Host "Symlinked WezTerm home config into repo."
