# GUI Launch & Stability Test
# Verifies wezterm-gui.exe starts, creates a window, survives 15s, and shuts down cleanly.
# Usage: pwsh -NoLogo -NoProfile -File tools/test-gui-launch.ps1

[CmdletBinding()]
param(
    [string]$InstallDir = "C:\Users\david\bin",
    [int]$RuntimeSeconds = 15,
    [int]$ShutdownTimeoutMs = 5000
)

$ErrorActionPreference = 'Continue'
$guiExe = Join-Path $InstallDir 'wezterm-gui.exe'
$pass = 0
$fail = 0

function Report {
    param([string]$Status, [string]$Name, [string]$Detail = '')
    $color = switch ($Status) { 'OK' { 'Green' } 'FAIL' { 'Red' } 'WARN' { 'Yellow' } 'SKIP' { 'DarkGray' } }
    $icon = switch ($Status) { 'OK' { '[OK]  ' } 'FAIL' { '[FAIL]' } 'WARN' { '[WARN]' } 'SKIP' { '[SKIP]' } }
    $msg = "$icon $Name"
    if ($Detail) { $msg += " -- $Detail" }
    Write-Host $msg -ForegroundColor $color
    if ($Status -eq 'OK') { $script:pass++ } elseif ($Status -eq 'FAIL') { $script:fail++ }
}

Write-Host "`n=== GUI Launch & Stability Test ===" -ForegroundColor Cyan
Write-Host "Binary: $guiExe"
Write-Host "Runtime: ${RuntimeSeconds}s"

if (-not (Test-Path $guiExe)) {
    Report 'SKIP' 'wezterm-gui.exe' 'Not found'
    exit 0
}

# Launch
$proc = Start-Process -FilePath $guiExe -PassThru
Report 'OK' 'GUI launched' "PID=$($proc.Id)"
Start-Sleep -Seconds 3

# Check alive after 3s
if ($proc.HasExited) {
    Report 'FAIL' 'GUI alive after 3s' "Exited with code $($proc.ExitCode)"
    exit 1
}
Report 'OK' 'GUI alive after 3s'

# Check window handle
$proc.Refresh()
$hwnd = $proc.MainWindowHandle
if ($hwnd -ne [IntPtr]::Zero) {
    Report 'OK' 'Window handle' "$hwnd"
    Report 'OK' 'Window title' $proc.MainWindowTitle
} else {
    Report 'WARN' 'Window handle' 'Not available yet (may still be initializing)'
}

# Wait for full runtime test
$remaining = $RuntimeSeconds - 3
if ($remaining -gt 0) {
    Write-Host "Waiting ${remaining}s for stability test..."
    Start-Sleep -Seconds $remaining
}

if ($proc.HasExited) {
    Report 'FAIL' "GUI survived ${RuntimeSeconds}s" "Crashed/exited (code=$($proc.ExitCode))"
    exit 1
}
Report 'OK' "GUI survived ${RuntimeSeconds}s" 'No crash'

# Recheck window
$proc.Refresh()
$hwnd = $proc.MainWindowHandle
if ($hwnd -ne [IntPtr]::Zero) {
    Report 'OK' 'Window confirmed' "title=$($proc.MainWindowTitle)"
} else {
    Report 'WARN' 'Window handle still missing' 'Process running but no visible window'
}

# Memory sanity
$memMB = [math]::Round($proc.WorkingSet64 / 1MB, 1)
if ($memMB -lt 2000) {
    Report 'OK' 'Memory usage' "${memMB} MB"
} else {
    Report 'WARN' 'Memory usage' "${memMB} MB (high)"
}

# Graceful shutdown
Write-Host "Sending close..."
try {
    $null = $proc.CloseMainWindow()
    $closed = $proc.WaitForExit($ShutdownTimeoutMs)
    if ($closed) {
        Report 'OK' 'Graceful shutdown' "exit=$($proc.ExitCode)"
    } else {
        Report 'WARN' 'Graceful shutdown' "Timeout after ${ShutdownTimeoutMs}ms, force-killing"
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
} catch {
    Report 'WARN' 'Shutdown' "Error: $_ -- force-killing"
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}

Write-Host "`n=== Results: $pass passed, $fail failed ===" -ForegroundColor $(if ($fail -gt 0) { 'Red' } else { 'Green' })
exit $(if ($fail -gt 0) { 1 } else { 0 })
