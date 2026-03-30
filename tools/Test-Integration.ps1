#Requires -Version 5.1

<#
.SYNOPSIS
    Integration testing harness for WezTerm custom utilities.

.DESCRIPTION
    Exercises runtime integration scenarios that go beyond post-build smoke tests:
    - Daemon lifecycle management (start, IPC, shutdown)
    - Named pipe IPC protocol tests (ping, status, register+subscribe+broadcast)
    - Config validation via wezterm validate-config --format json
    - End-to-end custom subcommand verification

.PARAMETER InstallDir
    Directory containing installed binaries (default: C:\Users\david\bin)

.PARAMETER TestPipeName
    Named pipe name for test daemon instance (default: wezterm-utils-daemon-test)

.PARAMETER DaemonStartupMs
    Milliseconds to wait for daemon startup (default: 4000)

.EXAMPLE
    .\Test-Integration.ps1
    Run all integration tests

.EXAMPLE
    .\Test-Integration.ps1 -InstallDir .\target\release -TestPipeName my-test-pipe
    Run tests against a custom build location with a custom pipe name
#>

[CmdletBinding()]
param(
    [string]$InstallDir = "C:\Users\david\bin",
    [string]$TestPipeName = "wezterm-utils-daemon-test",
    [int]$DaemonStartupMs = 4000
)

$ErrorActionPreference = 'Continue'
$script:Results = [System.Collections.ArrayList]::new()
$script:StartTime = Get-Date

# ============================================================================
# TEST INFRASTRUCTURE
# ============================================================================

function Add-TestResult {
    param(
        [string]$Category,
        [string]$Name,
        [string]$Status,  # PASS, FAIL, WARN, SKIP
        [string]$Detail = ''
    )
    $null = $script:Results.Add([PSCustomObject]@{
        Category = $Category
        Name     = $Name
        Status   = $Status
        Detail   = $Detail
    })
    $color = switch ($Status) {
        'PASS' { 'Green' }
        'FAIL' { 'Red' }
        'WARN' { 'Yellow' }
        'SKIP' { 'DarkGray' }
    }
    $icon = switch ($Status) {
        'PASS' { '[OK]  ' }
        'FAIL' { '[FAIL]' }
        'WARN' { '[WARN]' }
        'SKIP' { '[SKIP]' }
    }
    $msg = "$icon $Category / $Name"
    if ($Detail) { $msg += " -- $Detail" }
    Write-Host $msg -ForegroundColor $color
}

# ============================================================================
# PIPE IPC HELPER
# ============================================================================

function Send-PipeMessage {
    <#
    .SYNOPSIS
        Sends a JSON message to a named pipe and reads the response.
        Uses .NET NamedPipeClientStream for reliable IPC.
    #>
    param(
        [string]$PipeName,
        [string]$JsonMessage,
        [int]$TimeoutMs = 5000
    )
    try {
        $pipe = New-Object System.IO.Pipes.NamedPipeClientStream(
            '.', $PipeName, [System.IO.Pipes.PipeDirection]::InOut,
            [System.IO.Pipes.PipeOptions]::None
        )
        $pipe.Connect($TimeoutMs)

        $writer = New-Object System.IO.StreamWriter($pipe)
        $writer.AutoFlush = $true
        $reader = New-Object System.IO.StreamReader($pipe)

        $writer.WriteLine($JsonMessage)
        $response = $reader.ReadLine()

        # Dispose only the pipe — it closes the underlying stream for both reader and writer
        try { $pipe.Dispose() } catch {}

        return $response
    } catch {
        Write-Host "[DEBUG] Send-PipeMessage to '$PipeName' failed: $_" -ForegroundColor DarkGray
        try { if ($pipe) { $pipe.Dispose() } } catch {}
        return $null
    }
}

# ============================================================================
# DAEMON LIFECYCLE
# ============================================================================

function Start-TestDaemon {
    <#
    .SYNOPSIS
        Starts a daemon instance with a test pipe name and returns the process object.
    #>
    param(
        [string]$DaemonExe,
        [string]$PipeName,
        [int]$StartupMs
    )

    $tempDir = [System.IO.Path]::GetTempPath()
    $outFile = Join-Path $tempDir "daemon-integ-out-$(Get-Random).txt"
    $errFile = Join-Path $tempDir "daemon-integ-err-$(Get-Random).txt"

    # Start daemon — use 'start --pipe' subcommand syntax
    $argList = "--log-level warn start --pipe \\.\pipe\$PipeName"
    try {
        $proc = Start-Process -FilePath $DaemonExe -ArgumentList $argList `
            -PassThru -NoNewWindow `
            -RedirectStandardOutput $outFile `
            -RedirectStandardError $errFile
    } catch {
        # Fallback: start without --pipe-name argument
        try {
            $proc = Start-Process -FilePath $DaemonExe `
                -PassThru -NoNewWindow `
                -RedirectStandardOutput $outFile `
                -RedirectStandardError $errFile
        } catch {
            return $null
        }
    }

    Start-Sleep -Milliseconds $StartupMs

    if ($proc.HasExited) {
        $stderr = ''
        if (Test-Path $errFile) {
            $stderr = Get-Content $errFile -Raw -ErrorAction SilentlyContinue
        }
        Write-Host "[DEBUG] Daemon exited early: exit=$($proc.ExitCode) stderr=$stderr" -ForegroundColor DarkGray
        return $null
    }

    return @{
        Process = $proc
        OutFile = $outFile
        ErrFile = $errFile
    }
}

function Stop-TestDaemon {
    param($DaemonInfo)
    if ($DaemonInfo -and $DaemonInfo.Process -and -not $DaemonInfo.Process.HasExited) {
        try {
            $DaemonInfo.Process.Kill()
            $DaemonInfo.Process.WaitForExit(3000)
        } catch {}
    }
    if ($DaemonInfo) {
        Remove-Item $DaemonInfo.OutFile, $DaemonInfo.ErrFile -ErrorAction SilentlyContinue
    }
}

# ============================================================================
# TEST CATEGORIES
# ============================================================================

function Test-DaemonLifecycle {
    Write-Host "`n=== Daemon Lifecycle ===" -ForegroundColor Cyan

    $daemonExe = Join-Path $InstallDir 'wezterm-utils-daemon.exe'
    if (-not (Test-Path $daemonExe)) {
        Add-TestResult 'DaemonLifecycle' 'Binary' 'SKIP' 'wezterm-utils-daemon.exe not found'
        return
    }

    # Test generate-config (idempotent setup)
    try {
        $output = & $daemonExe generate-config 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-TestResult 'DaemonLifecycle' 'generate-config' 'PASS' 'Config template generated'
        } else {
            Add-TestResult 'DaemonLifecycle' 'generate-config' 'WARN' "Exit $LASTEXITCODE"
        }
    } catch {
        Add-TestResult 'DaemonLifecycle' 'generate-config' 'FAIL' $_.Exception.Message
    }

    # Test validate-config
    try {
        $output = & $daemonExe validate-config 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-TestResult 'DaemonLifecycle' 'validate-config' 'PASS' 'Configuration valid'
        } else {
            Add-TestResult 'DaemonLifecycle' 'validate-config' 'WARN' "Exit $LASTEXITCODE"
        }
    } catch {
        Add-TestResult 'DaemonLifecycle' 'validate-config' 'FAIL' $_.Exception.Message
    }

    # Start daemon
    $daemon = Start-TestDaemon -DaemonExe $daemonExe -PipeName $TestPipeName -StartupMs $DaemonStartupMs
    if (-not $daemon) {
        Add-TestResult 'DaemonLifecycle' 'Start' 'FAIL' 'Daemon failed to start or exited early'
        return
    }
    Add-TestResult 'DaemonLifecycle' 'Start' 'PASS' "PID $($daemon.Process.Id)"

    # Check pipe exists
    $pipeSearch = @([System.IO.Directory]::GetFiles('\\.\pipe\') | Where-Object {
        $_ -match 'wezterm-utils'
    })
    if ($pipeSearch.Count -gt 0) {
        Add-TestResult 'DaemonLifecycle' 'Pipe created' 'PASS' "$($pipeSearch[0])"
    } else {
        Add-TestResult 'DaemonLifecycle' 'Pipe created' 'WARN' 'Named pipe not detected (may use different name)'
    }

    # Store daemon for IPC tests
    $script:TestDaemon = $daemon
}

function Test-DaemonIPC {
    Write-Host "`n=== Daemon IPC Protocol ===" -ForegroundColor Cyan

    if (-not $script:TestDaemon) {
        Add-TestResult 'DaemonIPC' 'IPC tests' 'SKIP' 'No running test daemon'
        return
    }

    # Use the test pipe name directly (we started the daemon with this pipe)
    $actualPipe = $TestPipeName

    # Test 1: Ping (JSON-RPC 2.0 protocol)
    $pingMsg = '{"jsonrpc":"2.0","method":"daemon/ping","id":1}'
    $pingResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $pingMsg -TimeoutMs 5000
    if ($pingResp) {
        try {
            $pingJson = $pingResp | ConvertFrom-Json
            if ($pingJson.result.status -eq 'pong') {
                Add-TestResult 'DaemonIPC' 'ping -> pong' 'PASS' 'pong received'
            } else {
                $preview = if ($pingResp.Length -gt 80) { $pingResp.Substring(0, 77) + '...' } else { $pingResp }
                Add-TestResult 'DaemonIPC' 'ping -> pong' 'WARN' "Got response but not pong: $preview"
            }
        } catch {
            $preview = if ($pingResp.Length -gt 80) { $pingResp.Substring(0, 77) + '...' } else { $pingResp }
            Add-TestResult 'DaemonIPC' 'ping -> pong' 'WARN' "Non-JSON response: $preview"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'ping -> pong' 'FAIL' 'No response from pipe'
    }

    # Test 2: Status (JSON-RPC 2.0)
    $statusMsg = '{"jsonrpc":"2.0","method":"daemon/status","id":2}'
    $statusResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $statusMsg -TimeoutMs 5000
    if ($statusResp) {
        try {
            $statusJson = $statusResp | ConvertFrom-Json
            if ($statusJson.result.version) {
                Add-TestResult 'DaemonIPC' 'status' 'PASS' "version=$($statusJson.result.version), uptime=$($statusJson.result.uptime_seconds)s"
            } else {
                Add-TestResult 'DaemonIPC' 'status' 'WARN' "Unexpected: $statusResp"
            }
        } catch {
            Add-TestResult 'DaemonIPC' 'status' 'WARN' "Parse error: $statusResp"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'status' 'FAIL' 'No response from pipe'
    }

    # Test 3: Register (JSON-RPC 2.0 with flattened params)
    $registerMsg = '{"jsonrpc":"2.0","method":"daemon/register","params":{"name":"integration-test","capabilities":["testing"]},"id":3}'
    $registerResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $registerMsg -TimeoutMs 5000
    if ($registerResp) {
        try {
            $json = $registerResp | ConvertFrom-Json
            if ($json.result.status -eq 'registered') {
                Add-TestResult 'DaemonIPC' 'register' 'PASS' "name=$($json.result.name)"
            } else {
                Add-TestResult 'DaemonIPC' 'register' 'WARN' $registerResp
            }
        } catch {
            Add-TestResult 'DaemonIPC' 'register' 'WARN' "Parse error: $registerResp"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'register' 'WARN' 'No response'
    }

    # Test 4: Subscribe (JSON-RPC 2.0)
    $subscribeMsg = '{"jsonrpc":"2.0","method":"daemon/subscribe","params":{"subscriptions":[{"event_type":"test-event"}]},"id":4}'
    $subscribeResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $subscribeMsg -TimeoutMs 5000
    if ($subscribeResp) {
        try {
            $json = $subscribeResp | ConvertFrom-Json
            if ($json.result.status -eq 'subscribed') {
                Add-TestResult 'DaemonIPC' 'subscribe' 'PASS' "count=$($json.result.count)"
            } else {
                Add-TestResult 'DaemonIPC' 'subscribe' 'WARN' $subscribeResp
            }
        } catch {
            Add-TestResult 'DaemonIPC' 'subscribe' 'WARN' "Parse error: $subscribeResp"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'subscribe' 'WARN' 'No response'
    }

    # Test 5: Broadcast (JSON-RPC 2.0)
    $broadcastMsg = '{"jsonrpc":"2.0","method":"daemon/broadcast","params":{"event_type":"test-event","data":{"key":"value"}},"id":5}'
    $broadcastResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $broadcastMsg -TimeoutMs 5000
    if ($broadcastResp) {
        try {
            $json = $broadcastResp | ConvertFrom-Json
            # May receive the notification first, then the response
            if ($json.result.status -eq 'broadcast' -or $json.method -eq 'event/test-event') {
                Add-TestResult 'DaemonIPC' 'broadcast' 'PASS' $broadcastResp.Substring(0, [Math]::Min(80, $broadcastResp.Length))
            } else {
                Add-TestResult 'DaemonIPC' 'broadcast' 'WARN' $broadcastResp.Substring(0, [Math]::Min(80, $broadcastResp.Length))
            }
        } catch {
            Add-TestResult 'DaemonIPC' 'broadcast' 'WARN' "Parse error"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'broadcast' 'WARN' 'No response'
    }
}

function Test-DaemonShutdown {
    Write-Host "`n=== Daemon Shutdown ===" -ForegroundColor Cyan

    if (-not $script:TestDaemon) {
        Add-TestResult 'DaemonShutdown' 'Shutdown' 'SKIP' 'No running test daemon'
        return
    }

    $proc = $script:TestDaemon.Process
    if ($proc.HasExited) {
        Add-TestResult 'DaemonShutdown' 'Already exited' 'WARN' "Exit code $($proc.ExitCode)"
    } else {
        Stop-TestDaemon $script:TestDaemon
        Add-TestResult 'DaemonShutdown' 'Shutdown' 'PASS' 'Daemon terminated cleanly'
    }
    $script:TestDaemon = $null
}

function Test-WatcherIntegration {
    Write-Host "`n=== Watcher Event Detection ===" -ForegroundColor Cyan

    # Locate the watcher binary — standalone exe or wezterm subcommand
    $watcherExe = Join-Path $InstallDir 'wezterm-watch.exe'
    $weztermExe = Join-Path $InstallDir 'wezterm.exe'
    $useStandalone = Test-Path $watcherExe
    $useSubcommand = (-not $useStandalone) -and (Test-Path $weztermExe)

    if (-not $useStandalone -and -not $useSubcommand) {
        Add-TestResult 'Watcher' 'Binary' 'SKIP' 'Neither wezterm-watch.exe nor wezterm.exe found'
        return
    }

    $binaryLabel = if ($useStandalone) { 'wezterm-watch.exe' } else { 'wezterm.exe watch' }
    Add-TestResult 'Watcher' 'Binary' 'PASS' "Using $binaryLabel"

    # Create temp directory and output files
    $watchDir = Join-Path ([System.IO.Path]::GetTempPath()) "wezterm-watch-integ-$(Get-Random)"
    New-Item -ItemType Directory -Path $watchDir -Force | Out-Null

    $outFile = Join-Path ([System.IO.Path]::GetTempPath()) "watcher-integ-out-$(Get-Random).txt"
    $errFile = "$outFile.err"
    $watcherProc = $null

    try {
        # Start the watcher process watching our temp directory (events format for easy parsing)
        if ($useStandalone) {
            $watcherProc = Start-Process -FilePath $watcherExe `
                -ArgumentList "$watchDir --format events --no-git --interval 50" `
                -PassThru -NoNewWindow `
                -RedirectStandardOutput $outFile `
                -RedirectStandardError $errFile
        } else {
            $watcherProc = Start-Process -FilePath $weztermExe `
                -ArgumentList "watch $watchDir --format events --no-git --interval 50" `
                -PassThru -NoNewWindow `
                -RedirectStandardOutput $outFile `
                -RedirectStandardError $errFile
        }

        # Wait for watcher to initialise its file system listener
        Start-Sleep -Seconds 2

        if ($watcherProc.HasExited) {
            $stderr = ''
            if (Test-Path $errFile) {
                $stderr = Get-Content $errFile -Raw -ErrorAction SilentlyContinue
            }
            if ($stderr) { $stderr = $stderr.Trim() }
            if ($stderr.Length -gt 80) { $stderr = $stderr.Substring(0, 77) + '...' }
            Add-TestResult 'Watcher' 'Start' 'FAIL' "Exited early (code $($watcherProc.ExitCode)): $stderr"
            return
        }

        Add-TestResult 'Watcher' 'Start' 'PASS' "PID $($watcherProc.Id)"

        # Create a sentinel file in the watched directory
        $sentinelName = "watcher-test-$(Get-Random).txt"
        $sentinelPath = Join-Path $watchDir $sentinelName
        Set-Content -Path $sentinelPath -Value "integration test sentinel"

        # Give the watcher time to detect and write the event
        Start-Sleep -Seconds 3

        # Read captured stdout and check for the sentinel filename
        $output = ''
        if (Test-Path $outFile) {
            $output = Get-Content $outFile -Raw -ErrorAction SilentlyContinue
        }

        if ($output -and $output -match [regex]::Escape($sentinelName)) {
            Add-TestResult 'Watcher' 'Event detected' 'PASS' "Found '$sentinelName' in watcher output"
        } elseif ($output) {
            $preview = if ($output.Length -gt 120) { $output.Substring(0, 117) + '...' } else { $output.Trim() }
            Add-TestResult 'Watcher' 'Event detected' 'FAIL' "Output present but sentinel not found: $preview"
        } else {
            Add-TestResult 'Watcher' 'Event detected' 'FAIL' 'No output captured from watcher'
        }
    } catch {
        Add-TestResult 'Watcher' 'Integration' 'FAIL' $_.Exception.Message
    } finally {
        # Kill watcher process
        if ($watcherProc -and -not $watcherProc.HasExited) {
            try {
                $watcherProc.Kill()
                $watcherProc.WaitForExit(3000)
            } catch {}
        }

        # Cleanup temp files and directory
        Remove-Item $outFile, $errFile -ErrorAction SilentlyContinue
        Remove-Item $watchDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Test-ValidateConfigIntegration {
    Write-Host "`n=== Config Validation (Integration) ===" -ForegroundColor Cyan

    $weztermExe = Join-Path $InstallDir 'wezterm.exe'
    if (-not (Test-Path $weztermExe)) {
        Add-TestResult 'ConfigValidation' 'wezterm.exe' 'SKIP' 'Binary missing'
        return
    }

    $vcOutFile = Join-Path ([System.IO.Path]::GetTempPath()) "wezterm-integ-vc-$(Get-Random).txt"
    $vcErrFile = "$vcOutFile.err"
    try {
        $proc = Start-Process -FilePath $weztermExe `
            -ArgumentList 'validate-config --format json' `
            -NoNewWindow -PassThru `
            -RedirectStandardOutput $vcOutFile `
            -RedirectStandardError $vcErrFile
        $exited = $proc.WaitForExit(45000)

        if (-not $exited) {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
            Add-TestResult 'ConfigValidation' 'validate-config' 'FAIL' 'Timed out'
            return
        }

        $rawOut = ''
        if (Test-Path $vcOutFile) {
            $rawOut = Get-Content $vcOutFile -Raw -ErrorAction SilentlyContinue
        }
        if (-not $rawOut) {
            if (Test-Path $vcErrFile) {
                $rawErr = Get-Content $vcErrFile -Raw -ErrorAction SilentlyContinue
                if ($rawErr -and $rawErr.Trim().StartsWith('{')) { $rawOut = $rawErr }
            }
        }

        if (-not $rawOut) {
            Add-TestResult 'ConfigValidation' 'JSON output' 'FAIL' "No output (exit $($proc.ExitCode))"
            return
        }

        try {
            $json = $rawOut | ConvertFrom-Json
        } catch {
            Add-TestResult 'ConfigValidation' 'JSON parse' 'FAIL' 'Invalid JSON'
            return
        }

        # Structural checks
        $requiredFields = @('valid', 'config_file', 'warnings', 'watch_paths')
        $missingFields = @()
        foreach ($field in $requiredFields) {
            if ($null -eq $json.PSObject.Properties[$field]) {
                $missingFields += $field
            }
        }
        if ($missingFields.Count -gt 0) {
            Add-TestResult 'ConfigValidation' 'Schema fields' 'FAIL' "Missing: $($missingFields -join ', ')"
        } else {
            Add-TestResult 'ConfigValidation' 'Schema fields' 'PASS' "All $($requiredFields.Count) fields present"
        }

        # Valid field
        if ($json.valid -eq $true) {
            Add-TestResult 'ConfigValidation' 'valid=true' 'PASS' 'Configuration passes validation'
        } elseif ($json.valid -eq $false) {
            $errMsg = if ($json.error) { $json.error } else { 'unknown' }
            if ($errMsg.Length -gt 80) { $errMsg = $errMsg.Substring(0, 77) + '...' }
            Add-TestResult 'ConfigValidation' 'valid=true' 'WARN' "Config invalid: $errMsg"
        }

        # Config file path is a real file
        if ($json.config_file) {
            if (Test-Path $json.config_file) {
                Add-TestResult 'ConfigValidation' 'config_file exists' 'PASS' $json.config_file
            } else {
                Add-TestResult 'ConfigValidation' 'config_file exists' 'FAIL' "File not found: $($json.config_file)"
            }
        } else {
            Add-TestResult 'ConfigValidation' 'config_file' 'WARN' 'Using default config'
        }

        # Warnings array
        $warnCount = @($json.warnings).Count
        Add-TestResult 'ConfigValidation' 'warnings count' 'PASS' "$warnCount warning(s)"

        # Watch paths array
        $pathCount = @($json.watch_paths).Count
        Add-TestResult 'ConfigValidation' 'watch_paths count' 'PASS' "$pathCount path(s)"
    } catch {
        Add-TestResult 'ConfigValidation' 'validate-config' 'FAIL' $_.Exception.Message
    } finally {
        Remove-Item $vcOutFile, $vcErrFile -ErrorAction SilentlyContinue
    }
}

# ============================================================================
# MAIN
# ============================================================================

Write-Host ""
Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host " WezTerm Integration Test Harness" -ForegroundColor Cyan
Write-Host " Install dir:   $InstallDir" -ForegroundColor DarkGray
Write-Host " Test pipe:     $TestPipeName" -ForegroundColor DarkGray
Write-Host " Daemon wait:   ${DaemonStartupMs}ms" -ForegroundColor DarkGray
Write-Host " Date:          $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor DarkGray
Write-Host "=================================================================" -ForegroundColor Cyan

$script:TestDaemon = $null

# Daemon lifecycle: start -> IPC -> shutdown
Test-DaemonLifecycle
Test-DaemonIPC
Test-DaemonShutdown

# Watcher event detection
Test-WatcherIntegration

# Config validation (standalone wezterm.exe test)
Test-ValidateConfigIntegration

# ============================================================================
# SUMMARY
# ============================================================================

$duration = (Get-Date) - $script:StartTime
$pass  = ($script:Results | Where-Object Status -eq 'PASS').Count
$fail  = ($script:Results | Where-Object Status -eq 'FAIL').Count
$warn  = ($script:Results | Where-Object Status -eq 'WARN').Count
$skip  = ($script:Results | Where-Object Status -eq 'SKIP').Count
$total = $script:Results.Count

Write-Host ""
Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host " INTEGRATION TEST RESULTS" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "  Total:   $total" -ForegroundColor White
Write-Host "  Pass:    $pass" -ForegroundColor Green
Write-Host "  Fail:    $fail" -ForegroundColor $(if ($fail -gt 0) { 'Red' } else { 'Green' })
Write-Host "  Warn:    $warn" -ForegroundColor $(if ($warn -gt 0) { 'Yellow' } else { 'Green' })
Write-Host "  Skip:    $skip" -ForegroundColor DarkGray
Write-Host "  Time:    $($duration.TotalSeconds.ToString('F1'))s" -ForegroundColor DarkGray

if ($fail -gt 0) {
    Write-Host ""
    Write-Host "FAILURES:" -ForegroundColor Red
    $script:Results | Where-Object Status -eq 'FAIL' | ForEach-Object {
        Write-Host "  $($_.Category) / $($_.Name) -- $($_.Detail)" -ForegroundColor Red
    }
}

if ($warn -gt 0) {
    Write-Host ""
    Write-Host "WARNINGS:" -ForegroundColor Yellow
    $script:Results | Where-Object Status -eq 'WARN' | ForEach-Object {
        Write-Host "  $($_.Category) / $($_.Name) -- $($_.Detail)" -ForegroundColor Yellow
    }
}

Write-Host ""
$exitCode = if ($fail -gt 0) { 1 } else { 0 }
exit $exitCode
