#Requires -Version 5.1

<#
.SYNOPSIS
    Example integration of Build-Integration.ps1 with build-all.ps1

.DESCRIPTION
    This example shows how to integrate the Build-Integration.ps1 tooling
    into existing build scripts like build-all.ps1 to leverage build
    acceleration and development tools.

.NOTES
    This is an example/reference implementation. Copy the relevant parts
    into your actual build scripts.
#>

[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet('release', 'debug')]
    [string]$BuildProfile = 'release',

    [Parameter()]
    [switch]$SkipOptimization
)

$ErrorActionPreference = 'Stop'

# ============================================================================
# STEP 1: Import Build-Integration functions
# ============================================================================

Write-Host "Loading Build-Integration tooling..." -ForegroundColor Cyan
. "$PSScriptRoot\Build-Integration.ps1"

# ============================================================================
# STEP 2: Check build environment health (optional but recommended)
# ============================================================================

Write-Host ""
Write-Host "Checking build environment health..." -ForegroundColor Cyan

$health = Test-BuildToolHealth

if (-not $health.Overall) {
    Write-BuildStatus "Build environment has issues. Run: .\tools\Build-Integration.ps1 -Action health-check" -Level Warning
    Write-Host ""
    $response = Read-Host "Continue anyway? (y/N)"
    if ($response -notmatch '^y') {
        Write-BuildStatus "Build cancelled" -Level Error
        exit 1
    }
}

# ============================================================================
# STEP 3: Optimize build environment
# ============================================================================

if (-not $SkipOptimization) {
    Write-Host ""
    $optimizedEnv = Optimize-BuildEnvironment

    # Apply optimized environment variables
    foreach ($key in $optimizedEnv.Keys) {
        $value = $optimizedEnv[$key]

        if ($null -ne $value) {
            Set-Item "Env:$key" $value
            Write-Verbose "Set $key = $value"
        }
    }

    Write-BuildStatus "Build environment optimized" -Level Success
}

# ============================================================================
# STEP 4: Build with cargo (example)
# ============================================================================

Write-Host ""
Write-BuildSection "Building Rust Binaries"

$packages = @('wezterm-fs-explorer', 'wezterm-watch')

foreach ($package in $packages) {
    Write-BuildStatus "Building $package..." -Level Info

    $buildArgs = @('build', '--package', $package)

    if ($BuildProfile -eq 'release') {
        $buildArgs += '--release'
    }

    # Execute build
    try {
        $output = cargo $buildArgs 2>&1

        if ($LASTEXITCODE -eq 0) {
            Write-BuildStatus "$package built successfully" -Level Success
        } else {
            Write-BuildStatus "$package build failed" -Level Error
            $output | ForEach-Object { Write-Host $_ -ForegroundColor Red }
            exit 1
        }

    } catch {
        Write-BuildStatus "Build failed: $_" -Level Error
        exit 1
    }
}

# ============================================================================
# STEP 5: Display sccache statistics
# ============================================================================

Write-Host ""
if ($health.BuildAcceleration['sccache'].Available) {
    Get-SccacheStats
}

# ============================================================================
# STEP 6: Installation (example from build-all.ps1)
# ============================================================================

Write-Host ""
Write-BuildSection "Installing Binaries"

$installPath = "$env:USERPROFILE\.local\bin"

if (-not (Test-Path $installPath)) {
    New-Item -ItemType Directory -Path $installPath -Force | Out-Null
}

foreach ($package in $packages) {
    $binaryName = "$package.exe"
    $sourcePath = Join-Path $env:CARGO_TARGET_DIR "$BuildProfile\$binaryName"

    if (Test-Path $sourcePath) {
        Copy-Item $sourcePath $installPath -Force
        Write-BuildStatus "$binaryName installed to $installPath" -Level Success
    } else {
        Write-BuildStatus "$binaryName not found at $sourcePath" -Level Warning
    }
}

Write-Host ""
Write-BuildStatus "Build complete!" -Level Success

# ============================================================================
# ALTERNATIVE: Use CargoTools wrapper
# ============================================================================

<#
# If you want to use CargoTools for more sophisticated builds:

Import-CargoTools

# Use Invoke-CargoWrapper for builds with preflight checks
Invoke-CargoWrapper build --package wezterm-fs-explorer --release --preflight
#>

# ============================================================================
# INTEGRATION PATTERNS
# ============================================================================

<#
## Pattern 1: Health check before build

. .\tools\Build-Integration.ps1
$health = Test-BuildToolHealth
if (-not $health.Overall) {
    Install-RustBuildTools
}

## Pattern 2: Optimize and build

. .\tools\Build-Integration.ps1
$env = Optimize-BuildEnvironment
# ... apply environment ...
cargo build --release

## Pattern 3: Pre-release workflow

. .\tools\Build-Integration.ps1

# Check unreleased changes
$changes = Get-UnreleasedChanges
Write-Host "Found $($changes.CommitCount) commits since last release"

# Update changelog
Update-ProjectChangelog -Unreleased -Prepend

# Dry-run release
Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -DryRun

# Execute release (after review)
Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -Execute

## Pattern 4: CI/CD integration

. .\tools\Build-Integration.ps1

# Install tools in CI
if ($env:CI) {
    Install-RustBuildTools -ToolSubset "cargo-nextest,cargo-llvm-cov"
}

# Optimize and build
Optimize-BuildEnvironment | Out-Null
cargo build --release

# Show cache stats at end
Get-SccacheStats

## Pattern 5: Development workflow

. .\tools\Build-Integration.ps1

# One-time setup
if (-not (Get-Command cargo-nextest -ErrorAction SilentlyContinue)) {
    Install-RustBuildTools
}

# Regular development builds
Optimize-BuildEnvironment | Out-Null
cargo check

# Run tests with nextest
cargo nextest run

# Before committing
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings

## Pattern 6: Repository statistics

. .\tools\Build-Integration.ps1

# Show repo stats
Get-RepoStats

# Check what's changed
Get-UnreleasedChanges

## Pattern 7: Complete build-all.ps1 integration

# At the top of build-all.ps1
. "$PSScriptRoot\tools\Build-Integration.ps1"

# Before building
if (-not $SkipHealthCheck) {
    $health = Test-BuildToolHealth
    if (-not $health.Overall) {
        Write-Warning "Health check failed. Installing tools..."
        Install-RustBuildTools | Out-Null
    }
}

# Optimize build
if (-not $NoOptimization) {
    $env = Optimize-BuildEnvironment
    foreach ($key in $env.Keys) {
        if ($null -ne $env[$key]) {
            Set-Item "Env:$key" $env[$key]
        }
    }
}

# ... existing build logic ...

# At the end
if (Get-Command sccache -ErrorAction SilentlyContinue) {
    Write-Host ""
    Get-SccacheStats
}
#>
