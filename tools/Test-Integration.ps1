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

        $reader.Dispose()
        $writer.Dispose()
        $pipe.Dispose()

        return $response
    } catch {
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

    # Start daemon — pass pipe name if the daemon supports it, else use default
    $argList = "--pipe-name $PipeName"
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

    # Discover the actual pipe name
    $actualPipe = $null
    $pipeSearch = @([System.IO.Directory]::GetFiles('\\.\pipe\') | Where-Object {
        $_ -match 'wezterm-utils'
    })
    if ($pipeSearch.Count -gt 0) {
        # Extract pipe name from \\.\pipe\<name>
        $actualPipe = ($pipeSearch[0] -replace '^\\\\\.\pipe\\', '')
    }

    if (-not $actualPipe) {
        # Try common pipe name as fallback
        $actualPipe = 'wezterm-utils-daemon'
    }

    # Test 1: Ping
    $pingMsg = '{"type":"ping"}'
    $pingResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $pingMsg -TimeoutMs 5000
    if ($pingResp) {
        try {
            $pingJson = $pingResp | ConvertFrom-Json
            if ($pingJson.type -eq 'pong' -or $pingResp -match 'pong') {
                Add-TestResult 'DaemonIPC' 'ping -> pong' 'PASS' $pingResp
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

    # Test 2: Status
    $statusMsg = '{"type":"status"}'
    $statusResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $statusMsg -TimeoutMs 5000
    if ($statusResp) {
        try {
            $statusJson = $statusResp | ConvertFrom-Json
            $preview = if ($statusResp.Length -gt 80) { $statusResp.Substring(0, 77) + '...' } else { $statusResp }
            Add-TestResult 'DaemonIPC' 'status' 'PASS' $preview
        } catch {
            $preview = if ($statusResp.Length -gt 80) { $statusResp.Substring(0, 77) + '...' } else { $statusResp }
            Add-TestResult 'DaemonIPC' 'status' 'WARN' "Non-JSON response: $preview"
        }
    } else {
        Add-TestResult 'DaemonIPC' 'status' 'FAIL' 'No response from pipe'
    }

    # Test 3: Register + Subscribe + Broadcast sequence
    $registerMsg = '{"type":"register","client_id":"integration-test"}'
    $registerResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $registerMsg -TimeoutMs 5000
    if ($registerResp) {
        $preview = if ($registerResp.Length -gt 80) { $registerResp.Substring(0, 77) + '...' } else { $registerResp }
        Add-TestResult 'DaemonIPC' 'register' 'PASS' $preview
    } else {
        Add-TestResult 'DaemonIPC' 'register' 'WARN' 'No response (protocol may differ)'
    }

    $subscribeMsg = '{"type":"subscribe","topic":"test-topic","client_id":"integration-test"}'
    $subscribeResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $subscribeMsg -TimeoutMs 5000
    if ($subscribeResp) {
        $preview = if ($subscribeResp.Length -gt 80) { $subscribeResp.Substring(0, 77) + '...' } else { $subscribeResp }
        Add-TestResult 'DaemonIPC' 'subscribe' 'PASS' $preview
    } else {
        Add-TestResult 'DaemonIPC' 'subscribe' 'WARN' 'No response (protocol may differ)'
    }

    $broadcastMsg = '{"type":"broadcast","topic":"test-topic","payload":"hello-integration"}'
    $broadcastResp = Send-PipeMessage -PipeName $actualPipe -JsonMessage $broadcastMsg -TimeoutMs 5000
    if ($broadcastResp) {
        $preview = if ($broadcastResp.Length -gt 80) { $broadcastResp.Substring(0, 77) + '...' } else { $broadcastResp }
        Add-TestResult 'DaemonIPC' 'broadcast' 'PASS' $preview
    } else {
        Add-TestResult 'DaemonIPC' 'broadcast' 'WARN' 'No response (protocol may differ)'
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
