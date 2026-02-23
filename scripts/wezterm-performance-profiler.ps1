# WezTerm Performance Profiler
# Comprehensive performance analysis and benchmarking

param(
    [string]$ConfigPath = "$env:USERPROFILE\.wezterm.lua",
    [int]$Iterations = 5,
    [switch]$Baseline,
    [switch]$Optimized,
    [switch]$Compare
)

$ErrorActionPreference = "Stop"

# Performance metrics storage
$global:metrics = @{
    StartupTime = @()
    MemoryBaseline = @()
    MemoryPeak = @()
    CPUUsage = @()
    GPUMemory = @()
    ConfigLoadTime = @()
    FirstRenderTime = @()
}

function Get-ProcessMetrics {
    param([string]$ProcessName = "wezterm-gui")

    $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    if ($proc) {
        return @{
            WorkingSet = [math]::Round($proc.WorkingSet64 / 1MB, 2)
            PrivateMemory = [math]::Round($proc.PrivateMemorySize64 / 1MB, 2)
            VirtualMemory = [math]::Round($proc.VirtualMemorySize64 / 1MB, 2)
            HandleCount = $proc.HandleCount
            ThreadCount = $proc.Threads.Count
            CPU = $proc.CPU
        }
    }
    return $null
}

function Measure-StartupTime {
    Write-Host "Measuring startup time..." -ForegroundColor Cyan

    $times = @()
    for ($i = 1; $i -le $Iterations; $i++) {
        Write-Host "  Iteration $i/$Iterations" -ForegroundColor Gray

        # Kill any existing WezTerm processes
        Stop-Process -Name "wezterm-gui" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500

        # Measure cold start
        $startTime = Get-Date
        $proc = Start-Process -FilePath "wezterm" -ArgumentList "start", "--always-new-process" -PassThru

        # Wait for window to be responsive
        $timeout = 10
        $elapsed = 0
        while ($elapsed -lt $timeout) {
            $wnd = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
            if ($wnd -and $wnd.MainWindowHandle -ne 0) {
                break
            }
            Start-Sleep -Milliseconds 100
            $elapsed += 0.1
        }

        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalMilliseconds
        $times += $duration

        # Get memory after startup
        Start-Sleep -Milliseconds 500
        $metrics = Get-ProcessMetrics
        if ($metrics) {
            $global:metrics.MemoryBaseline += $metrics.WorkingSet
        }

        # Clean up
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 1000
    }

    $global:metrics.StartupTime = $times

    $avg = ($times | Measure-Object -Average).Average
    $min = ($times | Measure-Object -Minimum).Minimum
    $max = ($times | Measure-Object -Maximum).Maximum

    Write-Host "`nStartup Time Results:" -ForegroundColor Green
    Write-Host "  Average: $([math]::Round($avg, 2))ms" -ForegroundColor Yellow
    Write-Host "  Min: $([math]::Round($min, 2))ms" -ForegroundColor Yellow
    Write-Host "  Max: $([math]::Round($max, 2))ms" -ForegroundColor Yellow

    return $avg
}

function Measure-MemoryUsage {
    Write-Host "`nMeasuring memory usage..." -ForegroundColor Cyan

    # Start WezTerm
    $proc = Start-Process -FilePath "wezterm" -ArgumentList "start", "--always-new-process" -PassThru
    Start-Sleep -Seconds 2

    # Baseline memory
    $baseline = Get-ProcessMetrics
    Write-Host "  Baseline Memory: $($baseline.WorkingSet) MB" -ForegroundColor Yellow

    # Open multiple tabs to test memory scaling
    Write-Host "  Opening multiple tabs..." -ForegroundColor Gray
    for ($i = 1; $i -le 5; $i++) {
        Start-Process -FilePath "wezterm" -ArgumentList "cli", "spawn"
        Start-Sleep -Milliseconds 500
    }

    # Memory after tabs
    $withTabs = Get-ProcessMetrics
    Write-Host "  With 5 Tabs: $($withTabs.WorkingSet) MB" -ForegroundColor Yellow

    # Test scrollback memory
    Write-Host "  Testing scrollback memory..." -ForegroundColor Gray
    Start-Process -FilePath "wezterm" -ArgumentList "cli", "send-text", "--no-paste", ("x" * 1000 + "`n") * 1000
    Start-Sleep -Seconds 2

    $withScrollback = Get-ProcessMetrics
    Write-Host "  With Scrollback: $($withScrollback.WorkingSet) MB" -ForegroundColor Yellow

    # Clean up
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

    return @{
        Baseline = $baseline.WorkingSet
        WithTabs = $withTabs.WorkingSet
        WithScrollback = $withScrollback.WorkingSet
        MemoryPerTab = [math]::Round(($withTabs.WorkingSet - $baseline.WorkingSet) / 5, 2)
    }
}

function Measure-RenderingPerformance {
    Write-Host "`nMeasuring rendering performance..." -ForegroundColor Cyan

    # Start WezTerm with performance logging
    $env:WEZTERM_LOG = "wezterm_gui=trace,wezterm_font=trace"
    $logFile = "$env:TEMP\wezterm_perf.log"

    $proc = Start-Process -FilePath "wezterm" -ArgumentList "start", "--always-new-process" -PassThru -RedirectStandardError $logFile
    Start-Sleep -Seconds 2

    # Send rapid text to test rendering
    Write-Host "  Testing text rendering speed..." -ForegroundColor Gray
    $testText = (1..1000 | ForEach-Object { "Line $_: " + ("x" * 100) }) -join "`n"

    $startTime = Get-Date
    Start-Process -FilePath "wezterm" -ArgumentList "cli", "send-text", "--no-paste", $testText
    Start-Sleep -Seconds 2
    $endTime = Get-Date

    $renderTime = ($endTime - $startTime).TotalMilliseconds
    Write-Host "  Render Time for 1000 lines: $([math]::Round($renderTime, 2))ms" -ForegroundColor Yellow

    # Check GPU usage if available
    try {
        $gpu = Get-WmiObject Win32_VideoController | Select-Object -First 1
        Write-Host "  GPU: $($gpu.Name)" -ForegroundColor Gray
        Write-Host "  GPU Driver: $($gpu.DriverVersion)" -ForegroundColor Gray
    } catch {
        Write-Host "  GPU info not available" -ForegroundColor Gray
    }

    # Clean up
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Remove-Item $env:WEZTERM_LOG -ErrorAction SilentlyContinue

    return $renderTime
}

function Test-ConfigurationLoad {
    Write-Host "`nTesting configuration load time..." -ForegroundColor Cyan

    $times = @()
    for ($i = 1; $i -le 3; $i++) {
        $startTime = Get-Date
        & wezterm show-config 2>&1 | Out-Null
        $endTime = Get-Date

        $duration = ($endTime - $startTime).TotalMilliseconds
        $times += $duration
    }

    $avg = ($times | Measure-Object -Average).Average
    Write-Host "  Config Load Time: $([math]::Round($avg, 2))ms" -ForegroundColor Yellow

    return $avg
}

function Generate-Report {
    param(
        [hashtable]$Results,
        [string]$OutputFile = "wezterm-performance-report.json"
    )

    Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
    Write-Host "PERFORMANCE REPORT SUMMARY" -ForegroundColor Green
    Write-Host ("=" * 60) -ForegroundColor Cyan

    Write-Host "`nStartup Performance:" -ForegroundColor Yellow
    Write-Host "  Target: <500ms | Actual: $([math]::Round($Results.StartupTime, 2))ms" -ForegroundColor White
    if ($Results.StartupTime -lt 500) {
        Write-Host "  ✓ PASS" -ForegroundColor Green
    } else {
        Write-Host "  ✗ FAIL - Optimization needed" -ForegroundColor Red
    }

    Write-Host "`nMemory Usage:" -ForegroundColor Yellow
    Write-Host "  Target: <150MB | Actual: $($Results.Memory.Baseline)MB" -ForegroundColor White
    if ($Results.Memory.Baseline -lt 150) {
        Write-Host "  ✓ PASS" -ForegroundColor Green
    } else {
        Write-Host "  ✗ FAIL - Memory optimization needed" -ForegroundColor Red
    }
    Write-Host "  Memory per tab: $($Results.Memory.MemoryPerTab)MB" -ForegroundColor White

    Write-Host "`nRendering Performance:" -ForegroundColor Yellow
    Write-Host "  Target: <1000ms for 1000 lines | Actual: $([math]::Round($Results.RenderTime, 2))ms" -ForegroundColor White
    if ($Results.RenderTime -lt 1000) {
        Write-Host "  ✓ PASS" -ForegroundColor Green
    } else {
        Write-Host "  ✗ FAIL - GPU acceleration may need tuning" -ForegroundColor Red
    }

    Write-Host "`nConfiguration Load:" -ForegroundColor Yellow
    Write-Host "  Target: <100ms | Actual: $([math]::Round($Results.ConfigLoadTime, 2))ms" -ForegroundColor White
    if ($Results.ConfigLoadTime -lt 100) {
        Write-Host "  ✓ PASS" -ForegroundColor Green
    } else {
        Write-Host "  ✗ FAIL - Config optimization needed" -ForegroundColor Red
    }

    # Save detailed results
    $Results | ConvertTo-Json -Depth 10 | Set-Content -Path $OutputFile
    Write-Host "`nDetailed results saved to: $OutputFile" -ForegroundColor Cyan
}

# Main execution
Write-Host "WezTerm Performance Profiler" -ForegroundColor Magenta
Write-Host ("=" * 60) -ForegroundColor Magenta

$results = @{
    Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    ConfigPath = $ConfigPath
    Iterations = $Iterations
}

# Run all benchmarks
$results.StartupTime = Measure-StartupTime
$results.Memory = Measure-MemoryUsage
$results.RenderTime = Measure-RenderingPerformance
$results.ConfigLoadTime = Test-ConfigurationLoad

# Generate report
Generate-Report -Results $results

# If comparing, load and compare with baseline
if ($Compare -and (Test-Path "wezterm-baseline.json")) {
    Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
    Write-Host "COMPARISON WITH BASELINE" -ForegroundColor Green
    Write-Host ("=" * 60) -ForegroundColor Cyan

    $baseline = Get-Content "wezterm-baseline.json" | ConvertFrom-Json

    $startupImprovement = [math]::Round((($baseline.StartupTime - $results.StartupTime) / $baseline.StartupTime) * 100, 2)
    $memoryImprovement = [math]::Round((($baseline.Memory.Baseline - $results.Memory.Baseline) / $baseline.Memory.Baseline) * 100, 2)

    Write-Host "`nStartup Time:" -ForegroundColor Yellow
    Write-Host "  Baseline: $([math]::Round($baseline.StartupTime, 2))ms" -ForegroundColor White
    Write-Host "  Current: $([math]::Round($results.StartupTime, 2))ms" -ForegroundColor White
    Write-Host "  Improvement: $startupImprovement%" -ForegroundColor $(if ($startupImprovement -gt 0) { "Green" } else { "Red" })

    Write-Host "`nMemory Usage:" -ForegroundColor Yellow
    Write-Host "  Baseline: $($baseline.Memory.Baseline)MB" -ForegroundColor White
    Write-Host "  Current: $($results.Memory.Baseline)MB" -ForegroundColor White
    Write-Host "  Improvement: $memoryImprovement%" -ForegroundColor $(if ($memoryImprovement -gt 0) { "Green" } else { "Red" })
}

# Save as baseline if requested
if ($Baseline) {
    $results | ConvertTo-Json -Depth 10 | Set-Content -Path "wezterm-baseline.json"
    Write-Host "`nBaseline saved for future comparisons" -ForegroundColor Green
}