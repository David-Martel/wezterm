#!/usr/bin/env pwsh
# Build script for wezterm-utils-daemon

param(
    [Parameter()]
    [ValidateSet('debug', 'release', 'release-fast')]
    [string]$Profile = 'release',

    [Parameter()]
    [switch]$Clean,

    [Parameter()]
    [switch]$Test,

    [Parameter()]
    [switch]$Bench,

    [Parameter()]
    [switch]$Install,

    [Parameter()]
    [string]$InstallPath = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = 'Stop'

Write-Host "🦀 Building wezterm-utils-daemon" -ForegroundColor Cyan
Write-Host "Profile: $Profile" -ForegroundColor Gray

# Clean if requested
if ($Clean) {
    Write-Host "🧹 Cleaning..." -ForegroundColor Yellow
    cargo clean
}

# Build
Write-Host "🔨 Building..." -ForegroundColor Green
$buildArgs = @('build')

if ($Profile -ne 'debug') {
    $buildArgs += '--profile', $Profile
}

& cargo @buildArgs

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

# Run tests if requested
if ($Test) {
    Write-Host "🧪 Running tests..." -ForegroundColor Green
    cargo test --all-features

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Tests failed"
        exit 1
    }
}

# Run benchmarks if requested
if ($Bench) {
    Write-Host "📊 Running benchmarks..." -ForegroundColor Green
    cargo bench

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Benchmarks failed"
        exit 1
    }
}

# Install if requested
if ($Install) {
    Write-Host "📦 Installing..." -ForegroundColor Green

    # Determine binary location
    $targetDir = if ($Profile -eq 'debug') {
        'target\debug'
    } else {
        "target\$Profile"
    }

    $binaryPath = Join-Path $targetDir 'wezterm-utils-daemon.exe'

    if (-not (Test-Path $binaryPath)) {
        Write-Error "Binary not found at: $binaryPath"
        exit 1
    }

    # Create install directory if it doesn't exist
    if (-not (Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    }

    # Copy binary
    $destPath = Join-Path $InstallPath 'wezterm-utils-daemon.exe'
    Copy-Item $binaryPath $destPath -Force

    Write-Host "✅ Installed to: $destPath" -ForegroundColor Green

    # Check if install path is in PATH
    $pathParts = $env:PATH -split ';'
    if ($pathParts -notcontains $InstallPath) {
        Write-Host "⚠️  Warning: $InstallPath is not in your PATH" -ForegroundColor Yellow
        Write-Host "   Add it with:" -ForegroundColor Yellow
        Write-Host "   `$env:PATH += `;$InstallPath`" -ForegroundColor Gray
    }
}

# Show binary location
$targetDir = if ($Profile -eq 'debug') {
    'target\debug'
} else {
    "target\$Profile"
}

$binaryPath = Join-Path $targetDir 'wezterm-utils-daemon.exe'

if (Test-Path $binaryPath) {
    $fileInfo = Get-Item $binaryPath
    Write-Host "✅ Build complete!" -ForegroundColor Green
    Write-Host "   Binary: $binaryPath" -ForegroundColor Gray
    Write-Host "   Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
} else {
    Write-Error "Binary not found at expected location: $binaryPath"
    exit 1
}

Write-Host ""
Write-Host "Usage examples:" -ForegroundColor Cyan
Write-Host "  .\$binaryPath start" -ForegroundColor Gray
Write-Host "  .\$binaryPath generate-config" -ForegroundColor Gray
Write-Host "  .\$binaryPath status" -ForegroundColor Gray