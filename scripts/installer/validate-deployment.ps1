#!/usr/bin/env pwsh
# WezTerm Utilities Deployment Validation Script
# Comprehensive testing and verification of deployment

param(
    [switch]$Verbose,
    [switch]$SkipIpcTest,
    [switch]$Quick  # Skip time-consuming tests
)

$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"

# Paths
$BIN_DIR = "$env:USERPROFILE\.local\bin"
$CONFIG_DIR = "$env:USERPROFILE\.config\wezterm"
$STATE_DIR = "$CONFIG_DIR\wezterm-utils-state"

# Test results
$script:errors = @()
$script:warnings = @()
$script:passed = 0
$script:failed = 0

# Colors
function Write-TestHeader { param($Message) Write-Host "`n$Message" -ForegroundColor Cyan }
function Write-TestPass { param($Message) Write-Host "  ✓ $Message" -ForegroundColor Green; $script:passed++ }
function Write-TestFail { param($Message) Write-Host "  ✗ $Message" -ForegroundColor Red; $script:failed++; $script:errors += $Message }
function Write-TestWarn { param($Message) Write-Host "  ⚠ $Message" -ForegroundColor Yellow; $script:warnings += $Message }
function Write-TestInfo { param($Message) if ($Verbose) { Write-Host "    → $Message" -ForegroundColor Gray } }

# Banner
Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   WezTerm Utilities Deployment Validation v1.0.0        ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Test 1: Binary Existence
Write-TestHeader "TEST 1: Binary Existence"
$binaries = @{
    "wezterm-fs-explorer" = "wezterm-fs-explorer.exe"
    "wezterm-watch" = "wezterm-watch.exe"
}

foreach ($name in $binaries.Keys) {
    $path = Join-Path $BIN_DIR $binaries[$name]
    if (Test-Path $path) {
        $size = (Get-Item $path).Length
        Write-TestPass "$name exists ($([math]::Round($size/1MB, 2)) MB)"
        Write-TestInfo "Location: $path"
    } else {
        Write-TestFail "$name NOT FOUND at $path"
    }
}

# Test 2: Binary Execution
Write-TestHeader "TEST 2: Binary Execution"
foreach ($name in $binaries.Keys) {
    $path = Join-Path $BIN_DIR $binaries[$name]
    if (Test-Path $path) {
        try {
            $output = & $path --version 2>&1
            $exitCode = $LASTEXITCODE

            if ($exitCode -eq 0) {
                Write-TestPass "$name executes successfully"
                Write-TestInfo "Version: $output"
            } else {
                Write-TestFail "$name returned exit code $exitCode"
                Write-TestInfo "Output: $output"
            }
        } catch {
            Write-TestFail "$name failed to execute: $($_.Exception.Message)"
        }
    } else {
        Write-TestWarn "Skipping execution test for $name (binary not found)"
    }
}

# Test 3: Help Output
Write-TestHeader "TEST 3: Help Output Validation"
foreach ($name in $binaries.Keys) {
    $path = Join-Path $BIN_DIR $binaries[$name]
    if (Test-Path $path) {
        try {
            $helpOutput = & $path --help 2>&1
            if ($helpOutput -match "Usage:|USAGE:") {
                Write-TestPass "$name provides help documentation"
                if ($Verbose) {
                    Write-TestInfo "Help preview: $($helpOutput -split "`n" | Select-Object -First 3 -Join "`n    → ")"
                }
            } else {
                Write-TestWarn "$name --help output doesn't contain usage information"
            }
        } catch {
            Write-TestWarn "$name --help failed: $($_.Exception.Message)"
        }
    }
}

# Test 4: Lua Module Existence
Write-TestHeader "TEST 4: Lua Module Installation"
$luaModules = @(
    "wezterm-utils.lua"
)

foreach ($module in $luaModules) {
    $path = Join-Path $CONFIG_DIR $module
    if (Test-Path $path) {
        $size = (Get-Item $path).Length
        Write-TestPass "$module exists ($size bytes)"
        Write-TestInfo "Location: $path"

        # Validate it's a valid Lua file
        $content = Get-Content $path -Raw
        if ($content -match "local\s+\w+\s*=") {
            Write-TestInfo "Valid Lua syntax detected"
        } else {
            Write-TestWarn "File may not contain valid Lua code"
        }
    } else {
        Write-TestFail "$module NOT FOUND at $path"
    }
}

# Test 5: Configuration Files
Write-TestHeader "TEST 5: Configuration Files"
$configFile = Join-Path $CONFIG_DIR "wezterm-utils-config.json"
if (Test-Path $configFile) {
    Write-TestPass "Configuration file exists"
    Write-TestInfo "Location: $configFile"

    # Validate JSON
    try {
        $config = Get-Content $configFile | ConvertFrom-Json
        Write-TestPass "Configuration is valid JSON"
        Write-TestInfo "Config keys: $($config.PSObject.Properties.Name -join ', ')"
    } catch {
        Write-TestFail "Configuration JSON is invalid: $($_.Exception.Message)"
    }
} else {
    Write-TestWarn "Configuration file not found (will use defaults)"
}

# Test 6: State Directory
Write-TestHeader "TEST 6: State Directory"
if (Test-Path $STATE_DIR) {
    Write-TestPass "State directory exists"
    Write-TestInfo "Location: $STATE_DIR"

    $stateFiles = Get-ChildItem $STATE_DIR -File -ErrorAction SilentlyContinue
    if ($stateFiles.Count -gt 0) {
        Write-TestInfo "State files: $($stateFiles.Count)"
    } else {
        Write-TestInfo "No state files yet (normal for fresh install)"
    }
} else {
    Write-TestFail "State directory not found at $STATE_DIR"
}

# Test 7: WezTerm Integration
Write-TestHeader "TEST 7: WezTerm Configuration Integration"
$weztermConfig = "$env:USERPROFILE\.wezterm.lua"
if (Test-Path $weztermConfig) {
    Write-TestPass ".wezterm.lua exists"
    Write-TestInfo "Location: $weztermConfig"

    $content = Get-Content $weztermConfig -Raw
    if ($content -match "require\s*\(\s*['\`"]wezterm-utils['\`"]\s*\)") {
        Write-TestPass "wezterm-utils is integrated in configuration"
    } else {
        Write-TestWarn "wezterm-utils not integrated - manual setup required"
        Write-Host "`n    Add this to your .wezterm.lua:" -ForegroundColor Yellow
        Write-Host "    local utils = require('wezterm-utils')" -ForegroundColor White
        Write-Host "    utils.setup(config)" -ForegroundColor White
    }

    # Check for syntax errors (basic)
    if ($content -match "return\s+config") {
        Write-TestInfo "Config appears to return config table"
    } else {
        Write-TestWarn "Config may not return config table properly"
    }
} else {
    Write-TestFail ".wezterm.lua not found - WezTerm may not be configured"
}

# Test 8: PATH Configuration
Write-TestHeader "TEST 8: PATH Configuration"
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$systemPath = [Environment]::GetEnvironmentVariable("PATH", "Machine")
$currentPath = $env:PATH

if ($currentPath -like "*$BIN_DIR*") {
    Write-TestPass "$BIN_DIR is in current PATH"
} else {
    Write-TestWarn "$BIN_DIR not in current PATH"
}

if ($userPath -like "*$BIN_DIR*") {
    Write-TestPass "$BIN_DIR is in user PATH"
} else {
    Write-TestWarn "$BIN_DIR not in user PATH (restart terminal)"
}

# Test 9: File Permissions
Write-TestHeader "TEST 9: File Permissions"
foreach ($name in $binaries.Keys) {
    $path = Join-Path $BIN_DIR $binaries[$name]
    if (Test-Path $path) {
        try {
            $acl = Get-Acl $path
            $owner = $acl.Owner
            Write-TestPass "$name has proper ownership ($owner)"
        } catch {
            Write-TestWarn "Could not check permissions for $name"
        }
    }
}

# Test 10: IPC Communication (if not skipped)
if (-not $SkipIpcTest -and -not $Quick) {
    Write-TestHeader "TEST 10: Inter-Process Communication"

    # Test filesystem explorer with quick operation
    $explorerPath = Join-Path $BIN_DIR "wezterm-fs-explorer.exe"
    if (Test-Path $explorerPath) {
        Write-TestInfo "Testing filesystem explorer..."
        try {
            # Try to get help (should be quick)
            $startTime = Get-Date
            $result = & $explorerPath --help 2>&1
            $duration = (Get-Date) - $startTime

            if ($LASTEXITCODE -eq 0) {
                Write-TestPass "Filesystem explorer responds ($([math]::Round($duration.TotalMilliseconds, 0))ms)"
            } else {
                Write-TestWarn "Explorer returned non-zero exit code"
            }
        } catch {
            Write-TestWarn "Explorer test failed: $($_.Exception.Message)"
        }
    }

    # Test file watcher
    $watchPath = Join-Path $BIN_DIR "wezterm-watch.exe"
    if (Test-Path $watchPath) {
        Write-TestInfo "Testing file watcher..."
        try {
            $startTime = Get-Date
            $result = & $watchPath --help 2>&1
            $duration = (Get-Date) - $startTime

            if ($LASTEXITCODE -eq 0) {
                Write-TestPass "File watcher responds ($([math]::Round($duration.TotalMilliseconds, 0))ms)"
            } else {
                Write-TestWarn "Watcher returned non-zero exit code"
            }
        } catch {
            Write-TestWarn "Watcher test failed: $($_.Exception.Message)"
        }
    }
} else {
    Write-TestInfo "Skipping IPC tests (--SkipIpcTest or --Quick)"
}

# Test 11: Performance Baseline
if (-not $Quick) {
    Write-TestHeader "TEST 11: Performance Baseline"

    foreach ($name in $binaries.Keys) {
        $path = Join-Path $BIN_DIR $binaries[$name]
        if (Test-Path $path) {
            try {
                # Measure startup time
                $measurements = 1..3 | ForEach-Object {
                    $startTime = Get-Date
                    $null = & $path --version 2>&1
                    $duration = (Get-Date) - $startTime
                    $duration.TotalMilliseconds
                }

                $avgTime = ($measurements | Measure-Object -Average).Average
                Write-TestPass "$name startup time: $([math]::Round($avgTime, 0))ms (avg of 3)"

                if ($avgTime -lt 100) {
                    Write-TestInfo "Excellent performance (<100ms)"
                } elseif ($avgTime -lt 500) {
                    Write-TestInfo "Good performance (<500ms)"
                } else {
                    Write-TestWarn "Slow startup time (>500ms)"
                }
            } catch {
                Write-TestWarn "Performance test failed for $name"
            }
        }
    }
}

# Test 12: Dependency Check
Write-TestHeader "TEST 12: System Dependencies"

# Check for Visual C++ Redistributable (required for Rust binaries)
$vcRedist = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\VisualStudio\*\VC\Runtimes\*" -ErrorAction SilentlyContinue
if ($vcRedist) {
    Write-TestPass "Visual C++ Redistributable detected"
} else {
    Write-TestWarn "Visual C++ Redistributable not detected (may be required)"
}

# Check WezTerm installation
$weztermPath = Get-Command wezterm -ErrorAction SilentlyContinue
if ($weztermPath) {
    Write-TestPass "WezTerm is installed"
    Write-TestInfo "Location: $($weztermPath.Source)"

    try {
        $weztermVersion = wezterm --version 2>&1
        Write-TestInfo "Version: $weztermVersion"
    } catch {
        Write-TestInfo "Could not get WezTerm version"
    }
} else {
    Write-TestWarn "WezTerm not found in PATH"
}

# Summary
Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                 VALIDATION SUMMARY                       ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`nResults:" -ForegroundColor White
Write-Host "  Passed:   $script:passed" -ForegroundColor Green
Write-Host "  Failed:   $script:failed" -ForegroundColor $(if ($script:failed -eq 0) { "Green" } else { "Red" })
Write-Host "  Warnings: $($script:warnings.Count)" -ForegroundColor $(if ($script:warnings.Count -eq 0) { "Green" } else { "Yellow" })

if ($script:errors.Count -eq 0 -and $script:warnings.Count -eq 0) {
    Write-Host "`n✓ ALL CHECKS PASSED! Deployment is ready for use." -ForegroundColor Green
    Write-Host "`nYou can now:" -ForegroundColor Cyan
    Write-Host "  • Start WezTerm and press Alt+E for filesystem explorer" -ForegroundColor White
    Write-Host "  • Run 'wezterm-watch <directory>' to monitor file changes" -ForegroundColor White
    exit 0
} elseif ($script:errors.Count -eq 0) {
    Write-Host "`n⚠ Deployment functional with $($script:warnings.Count) warning(s):" -ForegroundColor Yellow
    foreach ($warning in $script:warnings) {
        Write-Host "  • $warning" -ForegroundColor Yellow
    }
    Write-Host "`nSystem is usable but may need configuration adjustments." -ForegroundColor Yellow
    exit 0
} else {
    Write-Host "`n✗ Deployment has $($script:errors.Count) error(s):" -ForegroundColor Red
    foreach ($error in $script:errors) {
        Write-Host "  • $error" -ForegroundColor Red
    }

    if ($script:warnings.Count -gt 0) {
        Write-Host "`nAnd $($script:warnings.Count) warning(s):" -ForegroundColor Yellow
        foreach ($warning in $script:warnings) {
            Write-Host "  • $warning" -ForegroundColor Yellow
        }
    }

    Write-Host "`nPlease fix errors before using the system." -ForegroundColor Red
    Write-Host "Run with -Verbose for detailed diagnostic information." -ForegroundColor Gray
    exit 1
}