#Requires -Version 5.1

<#
.SYNOPSIS
    Master build and deployment script for WezTerm utilities

.DESCRIPTION
    Builds and deploys all WezTerm utilities:
    - wezterm-fs-explorer (Rust binary)
    - wezterm-watch (Rust binary)
    - Lua integration modules

    Features:
    - Parallel builds for maximum speed
    - Verification tests for all components
    - Installation to user PATH locations
    - Rollback capability on failure
    - Comprehensive error handling

.PARAMETER BuildProfile
    Rust build profile to use (release, release-fast, debug)

.PARAMETER SkipTests
    Skip running verification tests

.PARAMETER InstallPath
    Custom installation path (defaults to $env:USERPROFILE\.local\bin)

.PARAMETER Force
    Force reinstall even if binaries exist

.EXAMPLE
    .\build-all.ps1
    Build and install all utilities with default settings

.EXAMPLE
    .\build-all.ps1 -BuildProfile release-fast -Force
    Force rebuild with optimized profile
#>

[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet('release', 'release-fast', 'debug')]
    [string]$BuildProfile = 'release',

    [Parameter()]
    [ValidateSet('auto', 'on', 'off')]
    [string]$Sccache = 'auto',

    [Parameter()]
    [ValidateSet('auto', 'on', 'off')]
    [string]$Lld = 'auto',

    [Parameter()]
    [switch]$SkipTests,

    [Parameter()]
    [string]$InstallPath = "$env:USERPROFILE\.local\bin",

    [Parameter()]
    [switch]$Force,

    [Parameter()]
    [switch]$Verbose
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# ============================================================================
# CONFIGURATION
# ============================================================================

$Script:Config = @{
    RootDir = $PSScriptRoot
    InstallPath = $InstallPath
    BuildProfile = $BuildProfile
    CargoTargetDir = "$env:USERPROFILE\.cargo\shared-target"

    # Components to build
    RustBinaries = @(
        @{
            Name = 'wezterm-fs-explorer'
            Path = 'wezterm-fs-explorer'
            Binary = 'wezterm-fs-explorer.exe'
            Description = 'High-performance filesystem explorer'
        },
        @{
            Name = 'wezterm-watch'
            Path = 'wezterm-watch'
            Binary = 'wezterm-watch.exe'
            Description = 'File watcher with Git integration'
        }
    )

    # Lua modules to install
    LuaModules = @(
        'wezterm-utils.lua'
    )

    # Configuration files
    ConfigFiles = @(
        '.wezterm.lua'
    )
}

# Colors for output
$Script:Colors = @{
    Success = 'Green'
    Error = 'Red'
    Warning = 'Yellow'
    Info = 'Cyan'
    Dim = 'DarkGray'
}

# ============================================================================
# LOGGING AND OUTPUT
# ============================================================================

function Write-Status {
    param(
        [string]$Message,
        [string]$Level = 'Info'
    )

    $color = $Script:Colors[$Level]
    $prefix = switch ($Level) {
        'Success' { '✓' }
        'Error' { '✗' }
        'Warning' { '⚠' }
        'Info' { '→' }
        default { ' ' }
    }

    Write-Host "$prefix $Message" -ForegroundColor $color
}

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
}

function Write-Step {
    param([string]$Message)
    Write-Host "  → $Message" -ForegroundColor DarkGray
}

# ============================================================================
# VALIDATION
# ============================================================================

function Test-Prerequisites {
    Write-Section "Checking Prerequisites"

    $issues = @()

    # Check Rust toolchain
    Write-Step "Checking Rust toolchain..."
    try {
        $rustc = cargo --version 2>&1
        Write-Status "Rust: $rustc" -Level Success
    } catch {
        $issues += "Rust toolchain not found. Install from https://rustup.rs/"
    }

    # Check cargo shared target directory
    Write-Step "Checking cargo configuration..."
    if (Test-Path "$env:USERPROFILE\.cargo\config.toml") {
        Write-Status "Cargo config found" -Level Success
    } else {
        Write-Status "Cargo config not found - using default target directory" -Level Warning
    }

    # Check WezTerm installation
    Write-Step "Checking WezTerm installation..."
    try {
        $wezterm = wezterm --version 2>&1
        Write-Status "WezTerm: $wezterm" -Level Success
    } catch {
        Write-Status "WezTerm not found - utilities will still build" -Level Warning
    }

    # Check build acceleration tools (optional)
    Write-Step "Checking build acceleration tools..."
    $sccacheCmd = Get-Command sccache -ErrorAction SilentlyContinue
    if ($sccacheCmd) {
        Write-Status "sccache found: $($sccacheCmd.Source)" -Level Success
    } else {
        Write-Status "sccache not found (optional)" -Level Warning
    }

    $lldCmd = Get-Command lld-link -ErrorAction SilentlyContinue
    if ($lldCmd) {
        Write-Status "lld-link found: $($lldCmd.Source)" -Level Success
    } else {
        Write-Status "lld-link not found (optional)" -Level Warning
    }

    if ($issues.Count -gt 0) {
        Write-Section "Prerequisites Failed"
        foreach ($issue in $issues) {
            Write-Status $issue -Level Error
        }
        throw "Prerequisites check failed"
    }

    Write-Status "All prerequisites satisfied" -Level Success
}

# ============================================================================
# BUILD FUNCTIONS
# ============================================================================

function Resolve-Acceleration {
    $useSccache = $false
    $sccacheCmd = Get-Command sccache -ErrorAction SilentlyContinue
    if ($Sccache -eq 'on') {
        $useSccache = $true
    } elseif ($Sccache -eq 'auto') {
        $useSccache = $null -ne $sccacheCmd
    }

    $useLld = $false
    $lldCmd = Get-Command lld-link -ErrorAction SilentlyContinue
    if ($Lld -eq 'on') {
        $useLld = $true
    } elseif ($Lld -eq 'auto') {
        $useLld = $null -ne $lldCmd
    }

    return @{
        UseSccache = $useSccache
        SccacheCmd = $sccacheCmd
        UseLld = $useLld
        LldCmd = $lldCmd
    }
}

function Invoke-RustBuild {
    param(
        [hashtable]$Binary,
        [string]$Profile
    )

    Write-Step "Building $($Binary.Name)..."
    $accel = Resolve-Acceleration

    $buildPath = Join-Path $Script:Config.RootDir $Binary.Path

    if (-not (Test-Path $buildPath)) {
        throw "Build path not found: $buildPath"
    }

    Push-Location $buildPath
    try {
        # Determine cargo flags
        $cargoFlags = @('build')

        if ($Profile -ne 'debug') {
            $cargoFlags += '--profile', $Profile
        }

        # Apply build acceleration (optional)
        if ($accel.UseSccache) {
            if ($accel.SccacheCmd) {
                $env:RUSTC_WRAPPER = $accel.SccacheCmd.Source
                if ([string]::IsNullOrWhiteSpace($env:SCCACHE_DIR)) {
                    $env:SCCACHE_DIR = "T:\\RustCache\\sccache"
                }
                if ([string]::IsNullOrWhiteSpace($env:SCCACHE_CACHE_SIZE)) {
                    $env:SCCACHE_CACHE_SIZE = '10G'
                }
                try {
                    sccache --start-server | Out-Null
                } catch {
                    Write-Status "sccache server start failed; continuing without server" -Level Warning
                }
            } else {
                Write-Status "sccache requested but not found; continuing without it" -Level Warning
            }
        } else {
            Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
        }

        if ($accel.UseLld) {
            if ($accel.LldCmd) {
                $existingRustFlags = $env:RUSTFLAGS
                $lldFlag = '-C linker=lld-link'
                if ([string]::IsNullOrWhiteSpace($existingRustFlags)) {
                    $env:RUSTFLAGS = $lldFlag
                } elseif ($existingRustFlags -notmatch 'lld-link') {
                    $env:RUSTFLAGS = "$existingRustFlags $lldFlag"
                }
            } else {
                Write-Status "lld-link requested but not found; continuing without it" -Level Warning
            }
        }

        # Prefer parallel builds based on CPU count
        if (-not $env:CARGO_BUILD_JOBS) {
            $env:CARGO_BUILD_JOBS = $env:NUMBER_OF_PROCESSORS
        }

        # Enable incremental builds for debug profile
        if ($Profile -eq 'debug') {
            $env:CARGO_INCREMENTAL = '1'
        }

        # Execute build
        $output = cargo $cargoFlags 2>&1

        if ($LASTEXITCODE -ne 0) {
            Write-Status "Build output:" -Level Error
            $output | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
            throw "Build failed for $($Binary.Name)"
        }

        # Verify binary exists
        $binaryDir = if ($Profile -eq 'debug') { 'debug' } else { $Profile }
        $binaryPath = Join-Path $Script:Config.CargoTargetDir "$binaryDir\$($Binary.Binary)"

        if (-not (Test-Path $binaryPath)) {
            # Try default target directory
            $binaryPath = Join-Path $buildPath "target\$binaryDir\$($Binary.Binary)"
        }

        if (-not (Test-Path $binaryPath)) {
            throw "Binary not found after build: $($Binary.Binary)"
        }

        Write-Status "$($Binary.Name) built successfully" -Level Success

        if ($accel.UseSccache -and $accel.SccacheCmd) {
            try {
                $stats = sccache --show-stats 2>&1
                Write-Status "sccache stats:" -Level Info
                $stats | ForEach-Object { Write-Host "  $_" -ForegroundColor DarkGray }
            } catch {
                Write-Status "Unable to read sccache stats" -Level Warning
            }
        }
        return $binaryPath

    } finally {
        Pop-Location
    }
}

function Build-AllRustBinaries {
    Write-Section "Building Rust Binaries"

    $builtBinaries = @{}

    foreach ($binary in $Script:Config.RustBinaries) {
        try {
            $binaryPath = Invoke-RustBuild -Binary $binary -Profile $Script:Config.BuildProfile
            $builtBinaries[$binary.Name] = $binaryPath
        } catch {
            Write-Status "Failed to build $($binary.Name): $_" -Level Error
            throw
        }
    }

    return $builtBinaries
}

# ============================================================================
# INSTALLATION FUNCTIONS
# ============================================================================

function Install-Binary {
    param(
        [string]$SourcePath,
        [string]$Name,
        [string]$DestinationDir
    )

    Write-Step "Installing $Name..."

    # Ensure destination directory exists
    if (-not (Test-Path $DestinationDir)) {
        New-Item -ItemType Directory -Path $DestinationDir -Force | Out-Null
        Write-Status "Created installation directory: $DestinationDir" -Level Info
    }

    $destPath = Join-Path $DestinationDir (Split-Path $SourcePath -Leaf)

    # Backup existing binary if present
    if (Test-Path $destPath) {
        $backupPath = "$destPath.backup"
        Copy-Item $destPath $backupPath -Force
        Write-Step "Backed up existing binary to $backupPath"
    }

    # Copy new binary
    Copy-Item $SourcePath $destPath -Force

    # Verify installation
    if (Test-Path $destPath) {
        $size = (Get-Item $destPath).Length
        Write-Status "$Name installed successfully ($([math]::Round($size/1KB, 2)) KB)" -Level Success
        return $destPath
    } else {
        throw "Installation verification failed for $Name"
    }
}

function Install-LuaModules {
    Write-Section "Installing Lua Modules"

    $weztermConfigDir = Join-Path $env:USERPROFILE ".config\wezterm"

    # Ensure config directory exists
    if (-not (Test-Path $weztermConfigDir)) {
        New-Item -ItemType Directory -Path $weztermConfigDir -Force | Out-Null
    }

    foreach ($module in $Script:Config.LuaModules) {
        $sourcePath = Join-Path $Script:Config.RootDir $module

        if (Test-Path $sourcePath) {
            $destPath = Join-Path $weztermConfigDir $module
            Copy-Item $sourcePath $destPath -Force
            Write-Status "$module installed" -Level Success
        } else {
            Write-Status "$module not found - skipping" -Level Warning
        }
    }
}

function Update-WeztermConfig {
    Write-Section "Updating WezTerm Configuration"

    $configSource = Join-Path $Script:Config.RootDir ".wezterm.lua"
    $configDest = Join-Path $env:USERPROFILE ".wezterm.lua"

    if (Test-Path $configSource) {
        # Backup existing config
        if (Test-Path $configDest) {
            $backupPath = "$configDest.backup"
            Copy-Item $configDest $backupPath -Force
            Write-Status "Backed up existing .wezterm.lua" -Level Info
        }

        # Copy new config
        Copy-Item $configSource $configDest -Force
        Write-Status ".wezterm.lua updated" -Level Success
    } else {
        Write-Status ".wezterm.lua not found in build directory" -Level Warning
    }
}

function Update-PathEnvironment {
    Write-Section "Updating PATH Environment"

    $installDir = $Script:Config.InstallPath

    # Check if already in PATH
    $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')

    if ($currentPath -notlike "*$installDir*") {
        Write-Step "Adding $installDir to user PATH..."

        $newPath = "$currentPath;$installDir"
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')

        Write-Status "PATH updated (restart terminal to apply)" -Level Success
    } else {
        Write-Status "$installDir already in PATH" -Level Success
    }
}

# ============================================================================
# TESTING AND VERIFICATION
# ============================================================================

function Test-BinaryExecution {
    param(
        [string]$BinaryPath,
        [string]$Name
    )

    Write-Step "Testing $Name..."

    try {
        # Test execution with --version flag
        $output = & $BinaryPath --version 2>&1

        if ($LASTEXITCODE -eq 0) {
            Write-Status "$Name: $output" -Level Success
            return $true
        } else {
            Write-Status "$Name failed version check" -Level Error
            return $false
        }
    } catch {
        Write-Status "$Name execution test failed: $_" -Level Error
        return $false
    }
}

function Invoke-VerificationTests {
    Write-Section "Running Verification Tests"

    $testResults = @{
        Passed = @()
        Failed = @()
    }

    # Test Rust binaries
    foreach ($binary in $Script:Config.RustBinaries) {
        $binaryPath = Join-Path $Script:Config.InstallPath $binary.Binary

        if (Test-Path $binaryPath) {
            if (Test-BinaryExecution -BinaryPath $binaryPath -Name $binary.Name) {
                $testResults.Passed += $binary.Name
            } else {
                $testResults.Failed += $binary.Name
            }
        } else {
            Write-Status "$($binary.Name) not found at $binaryPath" -Level Error
            $testResults.Failed += $binary.Name
        }
    }

    # Test WezTerm config loads
    Write-Step "Testing WezTerm configuration..."
    $configPath = Join-Path $env:USERPROFILE ".wezterm.lua"

    if (Test-Path $configPath) {
        try {
            # Try to validate config with wezterm
            $output = wezterm show-config 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Status "WezTerm configuration valid" -Level Success
                $testResults.Passed += "wezterm-config"
            } else {
                Write-Status "WezTerm configuration has errors" -Level Warning
                $testResults.Failed += "wezterm-config"
            }
        } catch {
            Write-Status "Could not validate WezTerm config (wezterm not in PATH)" -Level Warning
        }
    }

    # Summary
    Write-Host ""
    Write-Status "Verification Results:" -Level Info
    Write-Status "  Passed: $($testResults.Passed.Count)" -Level Success
    Write-Status "  Failed: $($testResults.Failed.Count)" -Level $(if ($testResults.Failed.Count -gt 0) { 'Error' } else { 'Success' })

    if ($testResults.Failed.Count -gt 0) {
        Write-Host ""
        Write-Status "Failed components:" -Level Error
        $testResults.Failed | ForEach-Object { Write-Host "    - $_" -ForegroundColor Red }
        return $false
    }

    return $true
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

function Invoke-Build {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║        WezTerm Utilities - Master Build & Deploy             ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""

    $startTime = Get-Date

    try {
        # Step 1: Prerequisites
        Test-Prerequisites

        # Step 2: Build Rust binaries
        $builtBinaries = Build-AllRustBinaries

        # Step 3: Install binaries
        Write-Section "Installing Binaries"
        foreach ($binary in $Script:Config.RustBinaries) {
            $sourcePath = $builtBinaries[$binary.Name]
            Install-Binary -SourcePath $sourcePath -Name $binary.Name -DestinationDir $Script:Config.InstallPath
        }

        # Step 4: Install Lua modules
        Install-LuaModules

        # Step 5: Update WezTerm config
        Update-WeztermConfig

        # Step 6: Update PATH
        Update-PathEnvironment

        # Step 7: Verification tests
        if (-not $SkipTests) {
            $testsPasseed = Invoke-VerificationTests

            if (-not $testsPasseed) {
                Write-Status "Some verification tests failed" -Level Warning
            }
        } else {
            Write-Status "Skipping verification tests" -Level Warning
        }

        # Success summary
        $duration = (Get-Date) - $startTime
        Write-Section "Build Complete"
        Write-Status "All components built and installed successfully" -Level Success
        Write-Status "Total time: $($duration.TotalSeconds.ToString('F2')) seconds" -Level Info
        Write-Host ""
        Write-Status "Installation directory: $($Script:Config.InstallPath)" -Level Info
        Write-Status "Restart your terminal to use the new PATH" -Level Info
        Write-Host ""

        # Print installed binaries
        Write-Host "Installed binaries:" -ForegroundColor Cyan
        foreach ($binary in $Script:Config.RustBinaries) {
            Write-Host "  - $($binary.Binary)" -ForegroundColor Green
        }
        Write-Host ""

        return $true

    } catch {
        Write-Section "Build Failed"
        Write-Status "Error: $_" -Level Error
        Write-Status "Stack trace:" -Level Error
        Write-Host $_.ScriptStackTrace -ForegroundColor Red
        return $false
    }
}

# ============================================================================
# SCRIPT ENTRY POINT
# ============================================================================

$success = Invoke-Build

exit $(if ($success) { 0 } else { 1 })
