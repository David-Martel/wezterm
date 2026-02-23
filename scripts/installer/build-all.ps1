#!/usr/bin/env pwsh
# WezTerm Utilities Build Script
# Compiles all binaries with production optimizations

param(
    [switch]$Release,      # Build in release mode (default)
    [switch]$Debug,        # Build in debug mode
    [switch]$Clean,        # Clean before build
    [switch]$Test,         # Run tests after build
    [switch]$Bench,        # Run benchmarks after build
    [switch]$Package,      # Package binaries after build
    [switch]$Verbose       # Verbose output
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Default to release mode
$BuildMode = if ($Debug) { "debug" } else { "release" }
$ReleaseFlag = if ($Debug) { "" } else { "--release" }

# Paths
$EXPLORER_DIR = "C:\Users\david\wezterm\wezterm-fs-explorer"
$WATCH_DIR = "C:\Users\david\wezterm\wezterm-watch"
$PACKAGE_DIR = "T:\projects\wezterm-utilities-installer\wezterm-utils\bin"

# Build configuration
$env:RUSTFLAGS = "-C target-cpu=native -C opt-level=3 -C lto=fat"
if ($env:SCCACHE_PATH) {
    $env:RUSTC_WRAPPER = $env:SCCACHE_PATH
}

# Colors
function Write-Success { param($Message) Write-Host "  ✓ $Message" -ForegroundColor Green }
function Write-Error { param($Message) Write-Host "  ✗ $Message" -ForegroundColor Red }
function Write-Warning { param($Message) Write-Host "  ⚠ $Message" -ForegroundColor Yellow }
function Write-Info { param($Message) Write-Host "  → $Message" -ForegroundColor Cyan }
function Write-Header { param($Message) Write-Host "`n$Message" -ForegroundColor Cyan }
function Write-Step { param($Step, $Total, $Message) Write-Host "`n[$Step/$Total] $Message" -ForegroundColor Magenta }

# Timing
$script:stepTimes = @{}
function Start-StepTimer { param($Name) $script:stepTimes[$Name] = Get-Date }
function Stop-StepTimer {
    param($Name)
    $elapsed = (Get-Date) - $script:stepTimes[$Name]
    $script:stepTimes.Remove($Name)
    return $elapsed
}

# Banner
Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║       WezTerm Utilities Build System v1.0.0             ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`nBuild Configuration:" -ForegroundColor White
Write-Host "  Mode: $BuildMode" -ForegroundColor $(if ($BuildMode -eq "release") { "Green" } else { "Yellow" })
Write-Host "  Optimizations: $(if ($BuildMode -eq "release") { "Enabled (LTO, native CPU)" } else { "Disabled" })" -ForegroundColor Gray
Write-Host "  Parallel: Enabled" -ForegroundColor Gray
if ($env:RUSTC_WRAPPER) {
    Write-Host "  Cache: sccache" -ForegroundColor Gray
}

# Step 1: Verify environment
Write-Step 1 6 "Verifying Build Environment"
Start-StepTimer "verify"

# Check Rust
try {
    $rustVersion = rustc --version
    Write-Success "Rust: $rustVersion"
} catch {
    Write-Error "Rust not found. Install from https://rustup.rs"
    exit 1
}

# Check Cargo
try {
    $cargoVersion = cargo --version
    Write-Success "Cargo: $cargoVersion"
} catch {
    Write-Error "Cargo not found"
    exit 1
}

# Verify project directories
if (Test-Path $EXPLORER_DIR) {
    Write-Success "Found filesystem explorer project"
} else {
    Write-Error "Filesystem explorer project not found at $EXPLORER_DIR"
    exit 1
}

if (Test-Path $WATCH_DIR) {
    Write-Success "Found file watcher project"
} else {
    Write-Error "File watcher project not found at $WATCH_DIR"
    exit 1
}

$verifyTime = Stop-StepTimer "verify"
Write-Info "Verification completed in $([math]::Round($verifyTime.TotalSeconds, 1))s"

# Step 2: Clean (if requested)
if ($Clean) {
    Write-Step 2 6 "Cleaning Build Artifacts"
    Start-StepTimer "clean"

    Write-Info "Cleaning filesystem explorer..."
    Push-Location $EXPLORER_DIR
    try {
        cargo clean
        Write-Success "Explorer cleaned"
    } catch {
        Write-Warning "Failed to clean explorer: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }

    Write-Info "Cleaning file watcher..."
    Push-Location $WATCH_DIR
    try {
        cargo clean
        Write-Success "Watcher cleaned"
    } catch {
        Write-Warning "Failed to clean watcher: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }

    $cleanTime = Stop-StepTimer "clean"
    Write-Info "Cleaning completed in $([math]::Round($cleanTime.TotalSeconds, 1))s"
} else {
    Write-Info "Skipping clean (use --Clean to clean before build)"
}

# Step 3: Build filesystem explorer
Write-Step 3 6 "Building Filesystem Explorer"
Start-StepTimer "explorer"

Push-Location $EXPLORER_DIR
try {
    Write-Info "Compiling wezterm-fs-explorer..."
    Write-Info "Flags: $env:RUSTFLAGS"

    if ($Verbose) {
        cargo build $ReleaseFlag --locked
    } else {
        cargo build $ReleaseFlag --locked 2>&1 | Out-Null
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    $binaryPath = "target\$BuildMode\wezterm-fs-explorer.exe"
    if (Test-Path $binaryPath) {
        $size = (Get-Item $binaryPath).Length / 1MB
        Write-Success "Built successfully ($([math]::Round($size, 2)) MB)"
    } else {
        Write-Error "Binary not found after build"
        exit 1
    }
} catch {
    Write-Error "Build failed: $($_.Exception.Message)"
    exit 1
} finally {
    Pop-Location
}

$explorerTime = Stop-StepTimer "explorer"
Write-Info "Explorer build completed in $([math]::Round($explorerTime.TotalSeconds, 1))s"

# Step 4: Build file watcher
Write-Step 4 6 "Building File Watcher"
Start-StepTimer "watcher"

Push-Location $WATCH_DIR
try {
    Write-Info "Compiling wezterm-watch..."
    Write-Info "Flags: $env:RUSTFLAGS"

    if ($Verbose) {
        cargo build $ReleaseFlag --locked
    } else {
        cargo build $ReleaseFlag --locked 2>&1 | Out-Null
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    $binaryPath = "target\$BuildMode\wezterm-watch.exe"
    if (Test-Path $binaryPath) {
        $size = (Get-Item $binaryPath).Length / 1MB
        Write-Success "Built successfully ($([math]::Round($size, 2)) MB)"
    } else {
        Write-Error "Binary not found after build"
        exit 1
    }
} catch {
    Write-Error "Build failed: $($_.Exception.Message)"
    exit 1
} finally {
    Pop-Location
}

$watcherTime = Stop-StepTimer "watcher"
Write-Info "Watcher build completed in $([math]::Round($watcherTime.TotalSeconds, 1))s"

# Step 5: Run tests (if requested)
if ($Test) {
    Write-Step 5 6 "Running Tests"
    Start-StepTimer "test"

    Write-Info "Testing filesystem explorer..."
    Push-Location $EXPLORER_DIR
    try {
        if ($Verbose) {
            cargo test $ReleaseFlag
        } else {
            cargo test $ReleaseFlag --quiet
        }

        if ($LASTEXITCODE -eq 0) {
            Write-Success "Explorer tests passed"
        } else {
            Write-Warning "Explorer tests failed"
        }
    } catch {
        Write-Warning "Test execution failed: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }

    Write-Info "Testing file watcher..."
    Push-Location $WATCH_DIR
    try {
        if ($Verbose) {
            cargo test $ReleaseFlag
        } else {
            cargo test $ReleaseFlag --quiet
        }

        if ($LASTEXITCODE -eq 0) {
            Write-Success "Watcher tests passed"
        } else {
            Write-Warning "Watcher tests failed"
        }
    } catch {
        Write-Warning "Test execution failed: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }

    $testTime = Stop-StepTimer "test"
    Write-Info "Testing completed in $([math]::Round($testTime.TotalSeconds, 1))s"
}

# Step 6: Package binaries (if requested)
if ($Package) {
    Write-Step 6 6 "Packaging Binaries"
    Start-StepTimer "package"

    # Create package directory
    New-Item -ItemType Directory -Force -Path $PACKAGE_DIR | Out-Null
    Write-Success "Created package directory"

    # Copy explorer
    $explorerSrc = "$EXPLORER_DIR\target\$BuildMode\wezterm-fs-explorer.exe"
    $explorerDst = "$PACKAGE_DIR\wezterm-fs-explorer.exe"
    if (Test-Path $explorerSrc) {
        Copy-Item $explorerSrc $explorerDst -Force
        $size = (Get-Item $explorerDst).Length / 1MB
        Write-Success "Packaged filesystem explorer ($([math]::Round($size, 2)) MB)"
    } else {
        Write-Error "Explorer binary not found for packaging"
    }

    # Copy watcher
    $watcherSrc = "$WATCH_DIR\target\$BuildMode\wezterm-watch.exe"
    $watcherDst = "$PACKAGE_DIR\wezterm-watch.exe"
    if (Test-Path $watcherSrc) {
        Copy-Item $watcherSrc $watcherDst -Force
        $size = (Get-Item $watcherDst).Length / 1MB
        Write-Success "Packaged file watcher ($([math]::Round($size, 2)) MB)"
    } else {
        Write-Error "Watcher binary not found for packaging"
    }

    # Strip binaries for smaller size (release mode only)
    if ($BuildMode -eq "release") {
        Write-Info "Stripping debug symbols..."
        try {
            $strip = Get-Command strip -ErrorAction SilentlyContinue
            if ($strip) {
                & strip "$explorerDst"
                & strip "$watcherDst"
                Write-Success "Binaries stripped"
            } else {
                Write-Info "Strip utility not found (optional)"
            }
        } catch {
            Write-Info "Could not strip binaries (optional)"
        }
    }

    $packageTime = Stop-StepTimer "package"
    Write-Info "Packaging completed in $([math]::Round($packageTime.TotalSeconds, 1))s"
}

# Run benchmarks (if requested)
if ($Bench -and $BuildMode -eq "release") {
    Write-Header "Running Benchmarks"
    Start-StepTimer "bench"

    Push-Location $EXPLORER_DIR
    try {
        Write-Info "Benchmarking filesystem explorer..."
        cargo bench --quiet
    } catch {
        Write-Warning "Benchmarks failed or not available"
    } finally {
        Pop-Location
    }

    Push-Location $WATCH_DIR
    try {
        Write-Info "Benchmarking file watcher..."
        cargo bench --quiet
    } catch {
        Write-Warning "Benchmarks failed or not available"
    } finally {
        Pop-Location
    }

    $benchTime = Stop-StepTimer "bench"
    Write-Info "Benchmarking completed in $([math]::Round($benchTime.TotalSeconds, 1))s"
}

# Summary
Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║           ✓ Build Successful!                           ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green

Write-Host "`nBinaries:" -ForegroundColor White
Write-Host "  Filesystem Explorer: $EXPLORER_DIR\target\$BuildMode\wezterm-fs-explorer.exe" -ForegroundColor Gray
Write-Host "  File Watcher: $WATCH_DIR\target\$BuildMode\wezterm-watch.exe" -ForegroundColor Gray

if ($Package) {
    Write-Host "`nPackaged in: $PACKAGE_DIR" -ForegroundColor Cyan
}

Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "  1. Run .\install.ps1 to install utilities" -ForegroundColor White
Write-Host "  2. Run .\validate-deployment.ps1 to verify" -ForegroundColor White
Write-Host "  3. Restart WezTerm to use utilities" -ForegroundColor White

# Show cache stats if using sccache
if ($env:RUSTC_WRAPPER -and (Get-Command sccache -ErrorAction SilentlyContinue)) {
    Write-Host "`nCache Statistics:" -ForegroundColor Gray
    sccache --show-stats
}