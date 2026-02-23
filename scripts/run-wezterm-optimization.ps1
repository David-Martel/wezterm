# WezTerm Optimization Runner
# Quick script to apply optimizations and measure improvements

param(
    [switch]$ApplyOptimizations,
    [switch]$RevertToOriginal,
    [switch]$ComparePerformance,
    [switch]$QuickTest
)

$ErrorActionPreference = "Stop"

# Paths
$originalConfig = "$env:USERPROFILE\.wezterm.lua"
$optimizedConfig = "$env:USERPROFILE\.wezterm-optimized.lua"
$backupConfig = "$env:USERPROFILE\.wezterm.lua.backup"
$profilerScript = "$env:USERPROFILE\wezterm-performance-profiler.ps1"

# Colors
function Write-Step { param($msg) Write-Host "`n➤ $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warning { param($msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host "✗ $msg" -ForegroundColor Red }
function Write-Info { param($msg) Write-Host "ℹ $msg" -ForegroundColor Blue }

# Header
Write-Host "`n" + ("=" * 70) -ForegroundColor Magenta
Write-Host "WEZTERM PERFORMANCE OPTIMIZATION SYSTEM" -ForegroundColor Magenta
Write-Host ("=" * 70) -ForegroundColor Magenta

# Quick system check
function Test-Prerequisites {
    Write-Step "Checking prerequisites..."

    $checks = @{
        "WezTerm Installed" = { Get-Command wezterm -ErrorAction SilentlyContinue }
        "Optimized Config" = { Test-Path $optimizedConfig }
        "Profiler Script" = { Test-Path $profilerScript }
        "PowerShell 5+" = { $PSVersionTable.PSVersion.Major -ge 5 }
    }

    $allPassed = $true
    foreach ($check in $checks.GetEnumerator()) {
        if (& $check.Value) {
            Write-Success $check.Key
        } else {
            Write-Error $check.Key
            $allPassed = $false
        }
    }

    if (!$allPassed) {
        Write-Error "Prerequisites check failed"
        exit 1
    }
}

# Apply optimizations
function Apply-Optimizations {
    Write-Step "Applying WezTerm optimizations..."

    # Backup current config
    if (Test-Path $originalConfig) {
        Copy-Item -Path $originalConfig -Destination $backupConfig -Force
        Write-Success "Created backup: $backupConfig"
    }

    # Apply optimized config
    Copy-Item -Path $optimizedConfig -Destination $originalConfig -Force
    Write-Success "Applied optimized configuration"

    # Restart WezTerm if running
    $wezterm = Get-Process -Name "wezterm-gui" -ErrorAction SilentlyContinue
    if ($wezterm) {
        Write-Info "Restarting WezTerm to apply changes..."
        Stop-Process -Name "wezterm-gui" -Force
        Start-Sleep -Seconds 1
        Start-Process wezterm
    }

    Write-Success "Optimizations applied successfully!"
}

# Revert to original
function Revert-ToOriginal {
    Write-Step "Reverting to original configuration..."

    if (!(Test-Path $backupConfig)) {
        Write-Error "No backup found at $backupConfig"
        return
    }

    Copy-Item -Path $backupConfig -Destination $originalConfig -Force
    Write-Success "Reverted to original configuration"

    # Restart WezTerm
    $wezterm = Get-Process -Name "wezterm-gui" -ErrorAction SilentlyContinue
    if ($wezterm) {
        Stop-Process -Name "wezterm-gui" -Force
        Start-Sleep -Seconds 1
        Start-Process wezterm
    }

    Write-Success "Successfully reverted to original configuration"
}

# Quick performance test
function Run-QuickTest {
    Write-Step "Running quick performance test..."

    # Kill any existing WezTerm
    Stop-Process -Name "wezterm-gui" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    # Measure startup time
    Write-Info "Testing startup time (3 iterations)..."
    $times = @()
    for ($i = 1; $i -le 3; $i++) {
        $start = Get-Date
        $proc = Start-Process -FilePath "wezterm" -ArgumentList "start", "--always-new-process" -PassThru

        # Wait for window
        $timeout = [DateTime]::Now.AddSeconds(5)
        while ([DateTime]::Now -lt $timeout) {
            $wnd = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
            if ($wnd -and $wnd.MainWindowHandle -ne 0) {
                break
            }
            Start-Sleep -Milliseconds 50
        }

        $elapsed = ((Get-Date) - $start).TotalMilliseconds
        $times += $elapsed

        # Measure memory
        Start-Sleep -Milliseconds 500
        $memory = (Get-Process -Id $proc.Id).WorkingSet64 / 1MB

        Write-Info "  Run $i : Startup: $([math]::Round($elapsed, 0))ms, Memory: $([math]::Round($memory, 1))MB"

        Stop-Process -Id $proc.Id -Force
        Start-Sleep -Milliseconds 500
    }

    $avgTime = ($times | Measure-Object -Average).Average

    Write-Host "`n" + ("-" * 50) -ForegroundColor Gray
    Write-Success "Average Startup Time: $([math]::Round($avgTime, 0))ms"

    # Performance assessment
    if ($avgTime -lt 500) {
        Write-Success "✓ EXCELLENT - Target achieved (<500ms)"
    } elseif ($avgTime -lt 700) {
        Write-Warning "○ GOOD - Close to target"
    } else {
        Write-Error "✗ NEEDS IMPROVEMENT - Above target"
    }
}

# Compare performance
function Compare-Performance {
    Write-Step "Comparing performance (Original vs Optimized)..."

    # Test with original config
    if (Test-Path $backupConfig) {
        Write-Info "Testing ORIGINAL configuration..."
        Copy-Item -Path $backupConfig -Destination $originalConfig -Force
        & $profilerScript -Iterations 3 | Out-File "$env:TEMP\wezterm-original.txt"
        $originalResults = Get-Content "$env:TEMP\wezterm-original.txt" | Select-String "Average:|Baseline Memory:"
    }

    # Test with optimized config
    Write-Info "Testing OPTIMIZED configuration..."
    Copy-Item -Path $optimizedConfig -Destination $originalConfig -Force
    & $profilerScript -Iterations 3 | Out-File "$env:TEMP\wezterm-optimized.txt"
    $optimizedResults = Get-Content "$env:TEMP\wezterm-optimized.txt" | Select-String "Average:|Baseline Memory:"

    # Display comparison
    Write-Host "`n" + ("=" * 50) -ForegroundColor Yellow
    Write-Host "PERFORMANCE COMPARISON" -ForegroundColor Yellow
    Write-Host ("=" * 50) -ForegroundColor Yellow

    Write-Host "`nOriginal Configuration:" -ForegroundColor Red
    $originalResults | ForEach-Object { Write-Host "  $_" }

    Write-Host "`nOptimized Configuration:" -ForegroundColor Green
    $optimizedResults | ForEach-Object { Write-Host "  $_" }

    # Parse and calculate improvement
    if ($originalResults -and $optimizedResults) {
        $origStartup = [regex]::Match($originalResults[0], '(\d+\.?\d*)ms').Groups[1].Value
        $optStartup = [regex]::Match($optimizedResults[0], '(\d+\.?\d*)ms').Groups[1].Value

        if ($origStartup -and $optStartup) {
            $improvement = [math]::Round((($origStartup - $optStartup) / $origStartup) * 100, 1)

            Write-Host "`n" + ("=" * 50) -ForegroundColor Green
            if ($improvement -gt 0) {
                Write-Success "Performance Improvement: $improvement% faster!"
            } else {
                Write-Warning "Performance Change: $improvement%"
            }
        }
    }
}

# Main menu
function Show-Menu {
    Write-Host "`nWhat would you like to do?" -ForegroundColor Cyan
    Write-Host "1. Apply optimizations" -ForegroundColor White
    Write-Host "2. Run quick performance test" -ForegroundColor White
    Write-Host "3. Compare original vs optimized" -ForegroundColor White
    Write-Host "4. Revert to original configuration" -ForegroundColor White
    Write-Host "5. Run full benchmark suite" -ForegroundColor White
    Write-Host "6. View optimization report" -ForegroundColor White
    Write-Host "Q. Quit" -ForegroundColor White

    $choice = Read-Host "`nEnter your choice"

    switch ($choice) {
        "1" { Apply-Optimizations; Show-Menu }
        "2" { Run-QuickTest; Show-Menu }
        "3" { Compare-Performance; Show-Menu }
        "4" { Revert-ToOriginal; Show-Menu }
        "5" { & $profilerScript -Iterations 5 -Baseline; Show-Menu }
        "6" { Start-Process notepad "$env:USERPROFILE\wezterm-optimization-report.md"; Show-Menu }
        "Q" { Write-Success "Goodbye!"; exit }
        "q" { Write-Success "Goodbye!"; exit }
        default { Write-Warning "Invalid choice"; Show-Menu }
    }
}

# Main execution
Test-Prerequisites

if ($ApplyOptimizations) {
    Apply-Optimizations
} elseif ($RevertToOriginal) {
    Revert-ToOriginal
} elseif ($ComparePerformance) {
    Compare-Performance
} elseif ($QuickTest) {
    Run-QuickTest
} else {
    # Interactive mode
    Show-Menu
}

Write-Host "`n" + ("=" * 70) -ForegroundColor Green
Write-Host "WezTerm Optimization Complete!" -ForegroundColor Green
Write-Host ("=" * 70) -ForegroundColor Green