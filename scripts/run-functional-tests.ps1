<#
.SYNOPSIS
    Run WezTerm functional test suite.

.DESCRIPTION
    This script runs various categories of functional tests for WezTerm:
    - SSH config parsing tests
    - SSH client integration tests
    - Terminal rendering tests
    - SFTP tests (when sshd available)

.PARAMETER Category
    Test category to run: all, ssh, config, terminal, sftp

.PARAMETER Verbose
    Show verbose test output

.PARAMETER Docker
    Run Docker-based tests (requires Docker)

.EXAMPLE
    .\run-functional-tests.ps1 -Category all
    .\run-functional-tests.ps1 -Category ssh -Verbose
    .\run-functional-tests.ps1 -Docker
#>

param(
    [ValidateSet("all", "ssh", "config", "terminal", "sftp", "russh")]
    [string]$Category = "all",

    [switch]$Verbose,

    [switch]$Docker,

    [switch]$Help
)

if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Detailed
    exit 0
}

$ErrorActionPreference = "Continue"
$script:TestsPassed = 0
$script:TestsFailed = 0
$script:TestsSkipped = 0

function Write-TestHeader {
    param([string]$Title)
    Write-Host ""
    Write-Host ("=" * 70) -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host ("=" * 70) -ForegroundColor Cyan
}

function Write-TestResult {
    param(
        [string]$Name,
        [bool]$Passed,
        [string]$Message = ""
    )

    if ($Passed) {
        Write-Host "[PASS] " -ForegroundColor Green -NoNewline
        $script:TestsPassed++
    } else {
        Write-Host "[FAIL] " -ForegroundColor Red -NoNewline
        $script:TestsFailed++
    }
    Write-Host $Name
    if ($Message -and $Verbose) {
        Write-Host "       $Message" -ForegroundColor Gray
    }
}

function Write-TestSkipped {
    param([string]$Name, [string]$Reason)
    Write-Host "[SKIP] " -ForegroundColor Yellow -NoNewline
    Write-Host "$Name - $Reason"
    $script:TestsSkipped++
}

function Test-DockerAvailable {
    try {
        $null = docker info 2>$null
        return $LASTEXITCODE -eq 0
    } catch {
        return $false
    }
}

function Test-SshdAvailable {
    # Check for WSL sshd or native sshd
    if ($IsLinux -or $IsMacOS) {
        return Test-Path "/usr/sbin/sshd"
    }
    # On Windows, check for OpenSSH
    return (Get-Command sshd -ErrorAction SilentlyContinue) -ne $null
}

# =============================================================================
# Run Tests
# =============================================================================

Write-Host ""
Write-Host "WezTerm Functional Test Suite" -ForegroundColor White
Write-Host "=============================" -ForegroundColor White
Write-Host ""

$startTime = Get-Date

# -----------------------------------------------------------------------------
# SSH Config Tests
# -----------------------------------------------------------------------------
if ($Category -in @("all", "config", "ssh")) {
    Write-TestHeader "SSH Config Parsing Tests"

    $configResult = cargo test -p wezterm-ssh config:: 2>&1
    $configPassed = $LASTEXITCODE -eq 0

    if ($Verbose) {
        Write-Host $configResult -ForegroundColor Gray
    }

    # Count tests from output
    $match = [regex]::Match($configResult, "(\d+) passed")
    if ($match.Success) {
        $count = $match.Groups[1].Value
        Write-TestResult "Config unit tests ($count tests)" $configPassed
    } else {
        Write-TestResult "Config unit tests" $configPassed
    }

    # Run functional config tests
    Write-Host ""
    Write-Host "Running SSH config functional tests..." -ForegroundColor Gray

    $funcResult = cargo test --test ssh_config_functional 2>&1
    $funcPassed = $LASTEXITCODE -eq 0

    if ($Verbose) {
        Write-Host $funcResult -ForegroundColor Gray
    }

    $match = [regex]::Match($funcResult, "(\d+) passed")
    if ($match.Success) {
        $count = $match.Groups[1].Value
        Write-TestResult "Config functional tests ($count tests)" $funcPassed
    } else {
        Write-TestResult "Config functional tests" $funcPassed
    }
}

# -----------------------------------------------------------------------------
# Russh Backend Tests
# -----------------------------------------------------------------------------
if ($Category -in @("all", "ssh", "russh")) {
    Write-TestHeader "Russh SSH Backend Tests"

    $russhResult = cargo test -p wezterm-ssh --features russh --no-default-features 2>&1
    $russhPassed = $LASTEXITCODE -eq 0

    if ($Verbose) {
        Write-Host $russhResult -ForegroundColor Gray
    }

    # Parse test counts
    $matches = [regex]::Matches($russhResult, "test result:.*?(\d+) passed")
    $totalPassed = 0
    foreach ($m in $matches) {
        $totalPassed += [int]$m.Groups[1].Value
    }

    Write-TestResult "Russh backend tests ($totalPassed tests)" $russhPassed
}

# -----------------------------------------------------------------------------
# Terminal Rendering Tests
# -----------------------------------------------------------------------------
if ($Category -in @("all", "terminal")) {
    Write-TestHeader "Terminal Rendering Tests"

    $termResult = cargo test --test terminal_rendering_tests 2>&1
    $termPassed = $LASTEXITCODE -eq 0

    if ($Verbose) {
        Write-Host $termResult -ForegroundColor Gray
    }

    $match = [regex]::Match($termResult, "(\d+) passed")
    if ($match.Success) {
        $count = $match.Groups[1].Value
        Write-TestResult "Terminal rendering tests ($count tests)" $termPassed
    } else {
        Write-TestResult "Terminal rendering tests" $termPassed
    }
}

# -----------------------------------------------------------------------------
# FS Utils Tests
# -----------------------------------------------------------------------------
if ($Category -eq "all") {
    Write-TestHeader "Filesystem Utils Tests"

    $fsResult = cargo test -p wezterm-fs-utils 2>&1
    $fsPassed = $LASTEXITCODE -eq 0

    if ($Verbose) {
        Write-Host $fsResult -ForegroundColor Gray
    }

    $match = [regex]::Match($fsResult, "(\d+) passed")
    if ($match.Success) {
        $count = $match.Groups[1].Value
        Write-TestResult "FS utils tests ($count tests)" $fsPassed
    } else {
        Write-TestResult "FS utils tests" $fsPassed
    }
}

# -----------------------------------------------------------------------------
# SFTP Tests (requires sshd)
# -----------------------------------------------------------------------------
if ($Category -in @("all", "sftp")) {
    Write-TestHeader "SFTP Tests"

    if (Test-SshdAvailable) {
        $sftpResult = cargo test -p wezterm-ssh e2e::sftp 2>&1
        $sftpPassed = $LASTEXITCODE -eq 0

        if ($Verbose) {
            Write-Host $sftpResult -ForegroundColor Gray
        }

        Write-TestResult "SFTP integration tests" $sftpPassed
    } else {
        Write-TestSkipped "SFTP integration tests" "sshd not available"
    }
}

# -----------------------------------------------------------------------------
# Docker Tests
# -----------------------------------------------------------------------------
if ($Docker) {
    Write-TestHeader "Docker-based Tests"

    if (Test-DockerAvailable) {
        Write-Host "Docker available, running container tests..." -ForegroundColor Gray

        # Run docker SSH test
        $dockerResult = cargo test --test docker_ssh_test -- --ignored 2>&1
        $dockerPassed = $LASTEXITCODE -eq 0

        if ($Verbose) {
            Write-Host $dockerResult -ForegroundColor Gray
        }

        Write-TestResult "Docker SSH tests" $dockerPassed
    } else {
        Write-TestSkipped "Docker SSH tests" "Docker not available"
    }
}

# =============================================================================
# Summary
# =============================================================================

$endTime = Get-Date
$duration = $endTime - $startTime

Write-Host ""
Write-Host ("=" * 70) -ForegroundColor Cyan
Write-Host "  TEST SUMMARY" -ForegroundColor Cyan
Write-Host ("=" * 70) -ForegroundColor Cyan
Write-Host ""
Write-Host "  Passed:  $($script:TestsPassed)" -ForegroundColor Green
Write-Host "  Failed:  $($script:TestsFailed)" -ForegroundColor $(if ($script:TestsFailed -gt 0) { "Red" } else { "Gray" })
Write-Host "  Skipped: $($script:TestsSkipped)" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Duration: $($duration.TotalSeconds.ToString('F2')) seconds" -ForegroundColor Gray
Write-Host ""

if ($script:TestsFailed -gt 0) {
    Write-Host "Some tests FAILED!" -ForegroundColor Red
    exit 1
} else {
    Write-Host "All tests PASSED!" -ForegroundColor Green
    exit 0
}
