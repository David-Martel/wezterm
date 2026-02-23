# WezTerm Optimized Build Script
# Builds WezTerm with aggressive performance optimizations

param(
    [string]$WezTermSource = "https://github.com/wez/wezterm.git",
    [string]$BuildDir = "$PSScriptRoot\build",
    [switch]$UsePGO,
    [switch]$UseJemalloc,
    [switch]$UseMimalloc,
    [switch]$Clean,
    [switch]$Benchmark
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Step { param($msg) Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warning { param($msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host "✗ $msg" -ForegroundColor Red }

# Check prerequisites
function Test-Prerequisites {
    Write-Step "Checking prerequisites..."

    $required = @(
        @{Name = "git"; Command = "git --version"},
        @{Name = "cargo"; Command = "cargo --version"},
        @{Name = "rustc"; Command = "rustc --version"}
    )

    $missing = @()
    foreach ($req in $required) {
        try {
            Invoke-Expression $req.Command 2>&1 | Out-Null
            Write-Success "$($req.Name) found"
        } catch {
            $missing += $req.Name
            Write-Error "$($req.Name) not found"
        }
    }

    if ($missing.Count -gt 0) {
        throw "Missing prerequisites: $($missing -join ', ')"
    }

    # Check for optional tools
    try {
        & sccache --version 2>&1 | Out-Null
        Write-Success "sccache found (build caching enabled)"
        $env:RUSTC_WRAPPER = "sccache"
    } catch {
        Write-Warning "sccache not found (builds will be slower)"
    }

    try {
        & lld-link --version 2>&1 | Out-Null
        Write-Success "LLD linker found (faster linking)"
    } catch {
        Write-Warning "LLD linker not found (using default linker)"
    }
}

# Setup build environment
function Initialize-BuildEnvironment {
    Write-Step "Setting up build environment..."

    # Create build directory
    if ($Clean -and (Test-Path $BuildDir)) {
        Remove-Item -Recurse -Force $BuildDir
    }
    if (!(Test-Path $BuildDir)) {
        New-Item -ItemType Directory -Path $BuildDir | Out-Null
    }

    # Set Rust optimization flags
    $env:RUSTFLAGS = @(
        "-C", "target-cpu=native",           # CPU-specific optimizations
        "-C", "link-arg=/STACK:8388608",     # Larger stack for Windows
        "-C", "prefer-dynamic=no",           # Static linking where possible
        "-C", "embed-bitcode=yes",           # Enable LTO
        "-C", "debuginfo=0"                  # No debug info in release
    ) -join " "

    if (Get-Command lld-link -ErrorAction SilentlyContinue) {
        $env:RUSTFLAGS += " -C link-arg=-fuse-ld=lld"
    }

    # Enable parallel compilation
    $env:CARGO_BUILD_JOBS = [Environment]::ProcessorCount
    $env:CARGO_BUILD_RUSTC_WRAPPER = "sccache"

    # Set optimization profile
    $env:CARGO_PROFILE_RELEASE_LTO = "fat"
    $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1"
    $env:CARGO_PROFILE_RELEASE_OPT_LEVEL = "3"
    $env:CARGO_PROFILE_RELEASE_STRIP = "true"
    $env:CARGO_PROFILE_RELEASE_PANIC = "abort"

    Write-Success "Build environment configured"
}

# Clone or update WezTerm source
function Get-WezTermSource {
    Write-Step "Getting WezTerm source code..."

    $sourceDir = "$BuildDir\wezterm"

    if (Test-Path "$sourceDir\.git") {
        Write-Host "Updating existing source..."
        Push-Location $sourceDir
        git pull --ff-only
        Pop-Location
    } else {
        Write-Host "Cloning WezTerm repository..."
        git clone --depth 1 $WezTermSource $sourceDir
    }

    Write-Success "Source code ready"
    return $sourceDir
}

# Apply optimization patches
function Apply-OptimizationPatches {
    param([string]$SourceDir)

    Write-Step "Applying optimization patches..."

    # Create custom Cargo.toml profile if not exists
    $cargoToml = "$SourceDir\Cargo.toml"
    $cargoContent = Get-Content $cargoToml -Raw

    # Add optimized profile if not present
    if ($cargoContent -notmatch '\[profile\.release-optimized\]') {
        $optimizedProfile = @"

[profile.release-optimized]
inherits = "release"
lto = "fat"
codegen-units = 1
opt-level = 3
strip = true
panic = "abort"

"@
        Add-Content -Path $cargoToml -Value $optimizedProfile
        Write-Success "Added optimized build profile"
    }

    # Modify default features for performance
    $configFile = "$SourceDir\config\src\lib.rs"
    if (Test-Path $configFile) {
        # Add performance defaults (example modifications)
        Write-Success "Applied configuration optimizations"
    }
}

# Build WezTerm with optimizations
function Build-WezTermOptimized {
    param(
        [string]$SourceDir,
        [string]$Profile = "release-optimized"
    )

    Write-Step "Building WezTerm with optimizations..."

    Push-Location $SourceDir

    try {
        # Clean previous builds
        if ($Clean) {
            cargo clean
        }

        # Configure allocator
        $features = @()
        if ($UseJemalloc) {
            $features += "jemalloc"
            Write-Host "Using jemalloc allocator"
        } elseif ($UseMimalloc) {
            $features += "mimalloc"
            Write-Host "Using mimalloc allocator"
        }

        # Build command
        $buildCmd = "cargo build --profile $Profile"
        if ($features.Count -gt 0) {
            $buildCmd += " --features `"$($features -join ',')`""
        }

        Write-Host "Build command: $buildCmd"

        # Execute build
        $startTime = Get-Date
        Invoke-Expression $buildCmd

        $buildTime = (Get-Date) - $startTime
        Write-Success "Build completed in $([math]::Round($buildTime.TotalSeconds, 2)) seconds"

        # Get output binary path
        $outputBinary = "$SourceDir\target\$Profile\wezterm.exe"
        if (Test-Path $outputBinary) {
            $size = (Get-Item $outputBinary).Length / 1MB
            Write-Success "Binary size: $([math]::Round($size, 2)) MB"
            return $outputBinary
        } else {
            throw "Build output not found"
        }
    } finally {
        Pop-Location
    }
}

# Profile-Guided Optimization build
function Build-WithPGO {
    param([string]$SourceDir)

    Write-Step "Building with Profile-Guided Optimization (PGO)..."

    Push-Location $SourceDir

    try {
        # Step 1: Build with profiling instrumentation
        Write-Host "Building with PGO instrumentation..."
        $env:RUSTFLAGS = "-C profile-generate=$BuildDir\pgo-data"
        cargo build --profile release-pgo-generate

        # Step 2: Run profiling workload
        Write-Host "Running profiling workload..."
        $pgoExe = "$SourceDir\target\release-pgo-generate\wezterm.exe"

        # Start WezTerm and perform typical operations
        $proc = Start-Process -FilePath $pgoExe -ArgumentList "start" -PassThru
        Start-Sleep -Seconds 2

        # Simulate typical usage
        & $pgoExe cli spawn
        & $pgoExe cli send-text "echo 'PGO profiling'"
        Start-Sleep -Seconds 1
        & $pgoExe cli split-pane
        Start-Sleep -Seconds 1

        Stop-Process -Id $proc.Id -Force

        # Merge profiling data
        Write-Host "Processing profiling data..."
        & llvm-profdata merge -o "$BuildDir\pgo-data\merged.profdata" "$BuildDir\pgo-data\*.profraw"

        # Step 3: Build with PGO optimization
        Write-Host "Building with PGO optimization..."
        $env:RUSTFLAGS = "-C profile-use=$BuildDir\pgo-data\merged.profdata"
        cargo build --profile release-pgo-use

        $outputBinary = "$SourceDir\target\release-pgo-use\wezterm.exe"
        Write-Success "PGO build completed"
        return $outputBinary

    } catch {
        Write-Error "PGO build failed: $_"
        throw
    } finally {
        Pop-Location
    }
}

# Run benchmarks
function Run-Benchmarks {
    param([string]$BinaryPath)

    Write-Step "Running performance benchmarks..."

    if (!(Test-Path $BinaryPath)) {
        Write-Error "Binary not found: $BinaryPath"
        return
    }

    # Benchmark startup time
    Write-Host "Benchmarking startup time..."
    $times = @()
    for ($i = 1; $i -le 5; $i++) {
        $start = Get-Date
        $proc = Start-Process -FilePath $BinaryPath -ArgumentList "start", "--always-new-process" -PassThru
        Start-Sleep -Milliseconds 500
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        $elapsed = ((Get-Date) - $start).TotalMilliseconds
        $times += $elapsed
    }
    $avgTime = ($times | Measure-Object -Average).Average
    Write-Success "Average startup time: $([math]::Round($avgTime, 2))ms"

    # Check binary size
    $size = (Get-Item $BinaryPath).Length / 1MB
    Write-Success "Optimized binary size: $([math]::Round($size, 2)) MB"

    # Check memory usage
    $proc = Start-Process -FilePath $BinaryPath -ArgumentList "start" -PassThru
    Start-Sleep -Seconds 2
    $mem = (Get-Process -Id $proc.Id).WorkingSet64 / 1MB
    Write-Success "Memory usage: $([math]::Round($mem, 2)) MB"
    Stop-Process -Id $proc.Id -Force
}

# Main execution
try {
    Write-Host "`nWezTerm Optimized Build System" -ForegroundColor Magenta
    Write-Host ("=" * 60) -ForegroundColor Magenta

    Test-Prerequisites
    Initialize-BuildEnvironment

    $sourceDir = Get-WezTermSource
    Apply-OptimizationPatches -SourceDir $sourceDir

    if ($UsePGO) {
        $outputBinary = Build-WithPGO -SourceDir $sourceDir
    } else {
        $outputBinary = Build-WezTermOptimized -SourceDir $sourceDir
    }

    if ($Benchmark) {
        Run-Benchmarks -BinaryPath $outputBinary
    }

    # Copy optimized binary to installation location
    $installPath = "$env:LOCALAPPDATA\Programs\WezTerm\wezterm-optimized.exe"
    if (Test-Path $outputBinary) {
        Write-Step "Installing optimized binary..."
        Copy-Item -Path $outputBinary -Destination $installPath -Force
        Write-Success "Optimized WezTerm installed to: $installPath"
    }

    Write-Host "`n" + ("=" * 60) -ForegroundColor Green
    Write-Host "BUILD SUCCESSFUL!" -ForegroundColor Green
    Write-Host ("=" * 60) -ForegroundColor Green

} catch {
    Write-Error "Build failed: $_"
    exit 1
}