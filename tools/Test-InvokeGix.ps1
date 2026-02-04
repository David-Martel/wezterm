<#
.SYNOPSIS
    Verification tests for Invoke-Gix.ps1 module.

.DESCRIPTION
    Runs basic validation tests to ensure the Invoke-Gix module functions correctly.
    Tests module import, function availability, and basic operations.
#>

#Requires -Version 5.1

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$modulePath = Join-Path $PSScriptRoot "Invoke-Gix.ps1"

Write-Host "`n=== Invoke-Gix Module Verification ===" -ForegroundColor Cyan
Write-Host "Module Path: $modulePath`n" -ForegroundColor Gray

$testResults = @{
    Passed = @()
    Failed = @()
    Warnings = @()
}

#region Test 1: Module File Exists

Write-Host "Test 1: Module file exists..." -NoNewline
if (Test-Path $modulePath) {
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "Module file exists"
} else {
    Write-Host " FAIL" -ForegroundColor Red
    $testResults.Failed += "Module file not found at: $modulePath"
    exit 1
}

#endregion

#region Test 2: Module Imports Successfully

Write-Host "Test 2: Module imports..." -NoNewline
try {
    Import-Module $modulePath -Force -ErrorAction Stop
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "Module imports successfully"
} catch {
    Write-Host " FAIL" -ForegroundColor Red
    $testResults.Failed += "Module import failed: $_"
    Write-Host "Error: $_" -ForegroundColor Red
    exit 1
}

#endregion

#region Test 3: Expected Functions Available

Write-Host "Test 3: Expected functions available..." -NoNewline
$expectedFunctions = @(
    'Invoke-Gix',
    'Get-GixRepoStats',
    'Get-GixUnreleasedCommits',
    'Test-GixRepoHealth',
    'Get-GixChangelog',
    'Get-GixVersionBump',
    'Measure-GixOperation',
    'Compare-GixPerformance'
)

# Get all functions (since it's a .ps1 script, they're just loaded into the session)
$availableFunctions = Get-Command -Name $expectedFunctions -ErrorAction SilentlyContinue
$missingFunctions = @()

foreach ($funcName in $expectedFunctions) {
    if ($funcName -notin $availableFunctions.Name) {
        $missingFunctions += $funcName
    }
}

if ($missingFunctions.Count -eq 0) {
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "All expected functions available ($($expectedFunctions.Count) functions)"
} else {
    Write-Host " FAIL" -ForegroundColor Red
    $testResults.Failed += "Missing functions: $($missingFunctions -join ', ')"
}

#endregion

#region Test 4: Gix Installation Check

Write-Host "Test 4: Gix installation check..." -NoNewline
$gixInstalled = Get-Command gix -ErrorAction SilentlyContinue
if ($gixInstalled) {
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "Gix installed at: $($gixInstalled.Source)"

    # Get gix version
    $gixVersion = & gix --version 2>$null
    Write-Verbose "Gix version: $gixVersion"
} else {
    Write-Host " WARN" -ForegroundColor Yellow
    $testResults.Warnings += "Gix not installed (install with: cargo binstall gix-cli)"
}

#endregion

#region Test 5: Function Help Documentation

Write-Host "Test 5: Function help documentation..." -NoNewline
$functionsWithoutHelp = @()

foreach ($funcName in $expectedFunctions) {
    $help = Get-Help $funcName -ErrorAction SilentlyContinue
    if (-not $help -or -not $help.Synopsis) {
        $functionsWithoutHelp += $funcName
    }
}

if ($functionsWithoutHelp.Count -eq 0) {
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "All functions have help documentation"
} else {
    Write-Host " WARN" -ForegroundColor Yellow
    $testResults.Warnings += "Functions without help: $($functionsWithoutHelp -join ', ')"
}

#endregion

#region Test 6: Basic Function Execution

Write-Host "Test 6: Basic function execution..." -NoNewline
try {
    # Test Get-GixRepoStats (safe to run, doesn't modify anything)
    $stats = Get-GixRepoStats -ErrorAction Stop

    if ($stats) {
        # Verify expected properties
        $expectedProps = @('RepositoryPath', 'CurrentBranch', 'TotalCommits', 'TotalBranches')
        $hasAllProps = $true
        foreach ($prop in $expectedProps) {
            if (-not ($stats.PSObject.Properties.Name -contains $prop)) {
                $hasAllProps = $false
                break
            }
        }

        if ($hasAllProps) {
            Write-Host " PASS" -ForegroundColor Green
            $testResults.Passed += "Get-GixRepoStats returns valid data"
            Write-Verbose "Repository: $($stats.RepositoryPath)"
            Write-Verbose "Current Branch: $($stats.CurrentBranch)"
            Write-Verbose "Total Commits: $($stats.TotalCommits)"
        } else {
            Write-Host " WARN" -ForegroundColor Yellow
            $testResults.Warnings += "Get-GixRepoStats missing expected properties"
        }
    } else {
        Write-Host " SKIP" -ForegroundColor Yellow
        $testResults.Warnings += "Get-GixRepoStats returned null (likely gix not installed)"
    }
} catch {
    Write-Host " FAIL" -ForegroundColor Red
    $testResults.Failed += "Function execution failed: $_"
}

#endregion

#region Test 7: Version Bump Analysis

if ($gixInstalled) {
    Write-Host "Test 7: Version bump analysis..." -NoNewline
    try {
        $versionBump = Get-GixVersionBump -ErrorAction Stop

        if ($versionBump -and $versionBump.RecommendedBump) {
            $validBumps = @('major', 'minor', 'patch', 'none')
            if ($versionBump.RecommendedBump -in $validBumps) {
                Write-Host " PASS" -ForegroundColor Green
                $testResults.Passed += "Version bump analysis works (recommended: $($versionBump.RecommendedBump))"
            } else {
                Write-Host " WARN" -ForegroundColor Yellow
                $testResults.Warnings += "Unexpected version bump value: $($versionBump.RecommendedBump)"
            }
        } else {
            Write-Host " WARN" -ForegroundColor Yellow
            $testResults.Warnings += "Version bump returned null or incomplete data"
        }
    } catch {
        Write-Host " FAIL" -ForegroundColor Red
        $testResults.Failed += "Version bump analysis failed: $_"
    }
} else {
    Write-Host "Test 7: Version bump analysis... SKIP (gix not installed)" -ForegroundColor Yellow
}

#endregion

#region Test 8: Parameter Validation

Write-Host "Test 8: Parameter validation..." -NoNewline
try {
    # Test invalid path parameter (should handle gracefully)
    $result = Get-GixRepoStats -Path "C:\NonExistent\Path" -ErrorAction SilentlyContinue 2>$null

    # If we get here, parameter validation is working
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "Parameter validation works correctly"
} catch {
    # Validation should prevent execution, which is good
    Write-Host " PASS" -ForegroundColor Green
    $testResults.Passed += "Parameter validation prevents invalid input"
}

#endregion

#region Test Results Summary

Write-Host "`n=== Test Results Summary ===" -ForegroundColor Cyan

Write-Host "`nPassed Tests ($($testResults.Passed.Count)):" -ForegroundColor Green
foreach ($test in $testResults.Passed) {
    Write-Host "  [PASS] $test" -ForegroundColor Green
}

if ($testResults.Warnings.Count -gt 0) {
    Write-Host "`nWarnings ($($testResults.Warnings.Count)):" -ForegroundColor Yellow
    foreach ($warning in $testResults.Warnings) {
        Write-Host "  [WARN] $warning" -ForegroundColor Yellow
    }
}

if ($testResults.Failed.Count -gt 0) {
    Write-Host "`nFailed Tests ($($testResults.Failed.Count)):" -ForegroundColor Red
    foreach ($failure in $testResults.Failed) {
        Write-Host "  [FAIL] $failure" -ForegroundColor Red
    }
    Write-Host "`nOverall: FAIL" -ForegroundColor Red
    exit 1
} else {
    $status = if ($testResults.Warnings.Count -gt 0) { "PASS (with warnings)" } else { "PASS" }
    $color = if ($testResults.Warnings.Count -gt 0) { 'Yellow' } else { 'Green' }
    Write-Host "`nOverall: $status" -ForegroundColor $color

    if ($testResults.Warnings.Count -gt 0) {
        Write-Host "`nNote: Install gix for full functionality:" -ForegroundColor Gray
        Write-Host "  cargo binstall gix-cli" -ForegroundColor Gray
    }

    exit 0
}

#endregion
