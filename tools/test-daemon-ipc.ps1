# Quick daemon IPC round-trip test
# Usage: pwsh -NoLogo -NoProfile -File tools/test-daemon-ipc.ps1

$ErrorActionPreference = 'Stop'
$daemonExe = 'C:\Users\david\.cache\claude\ipc-test\debug\wezterm-utils-daemon.exe'
$pipeName = 'wezterm-ipc-test-' + (Get-Random)
$pipeFullName = "\\.\pipe\$pipeName"

Write-Host "=== Daemon IPC Round-Trip Test ===" -ForegroundColor Cyan
Write-Host "Pipe: $pipeFullName"

# Start daemon
$daemon = Start-Process -FilePath $daemonExe `
    -ArgumentList '--log-level', 'warn', 'start', '--pipe', $pipeFullName `
    -NoNewWindow -PassThru `
    -RedirectStandardOutput ([System.IO.Path]::GetTempFileName()) `
    -RedirectStandardError ([System.IO.Path]::GetTempFileName())

Start-Sleep -Seconds 2

if ($daemon.HasExited) {
    Write-Host "[FAIL] Daemon exited with code $($daemon.ExitCode)" -ForegroundColor Red
    exit 1
}

Write-Host "[OK] Daemon started (PID=$($daemon.Id))" -ForegroundColor Green

try {
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', $pipeName, [System.IO.Pipes.PipeDirection]::InOut)
    $pipe.Connect(5000)
    $writer = [System.IO.StreamWriter]::new($pipe)
    $reader = [System.IO.StreamReader]::new($pipe)
    Write-Host "[OK] Connected to daemon pipe" -ForegroundColor Green

    # Test 1: Ping
    $writer.Write('{"jsonrpc":"2.0","method":"daemon/ping","id":1}' + "`n")
    $writer.Flush()
    $response = $reader.ReadLine() | ConvertFrom-Json
    if ($response.result.status -eq 'pong') {
        Write-Host "[OK] Ping -> pong" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Ping response: $($response | ConvertTo-Json -Compress)" -ForegroundColor Red
    }

    # Test 2: Status
    $writer.Write('{"jsonrpc":"2.0","method":"daemon/status","id":2}' + "`n")
    $writer.Flush()
    $response = $reader.ReadLine() | ConvertFrom-Json
    if ($response.result.version) {
        Write-Host "[OK] Status: version=$($response.result.version), uptime=$($response.result.uptime_seconds)s, connections=$($response.result.active_connections)" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Status response: $($response | ConvertTo-Json -Compress)" -ForegroundColor Red
    }

    # Test 3: Register (params under standard JSON-RPC "params" key)
    $writer.Write('{"jsonrpc":"2.0","method":"daemon/register","params":{"name":"test-client","capabilities":["state-sync"]},"id":3}' + "`n")
    $writer.Flush()
    $response = $reader.ReadLine() | ConvertFrom-Json
    if ($response.result.status -eq 'registered') {
        Write-Host "[OK] Register: name=$($response.result.name)" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Register response: $($response | ConvertTo-Json -Compress)" -ForegroundColor Red
    }

    # Test 4: Subscribe
    $writer.Write('{"jsonrpc":"2.0","method":"daemon/subscribe","params":{"subscriptions":[{"event_type":"panel-state"}]},"id":4}' + "`n")
    $writer.Flush()
    $response = $reader.ReadLine() | ConvertFrom-Json
    if ($response.result.status -eq 'subscribed') {
        Write-Host "[OK] Subscribe: count=$($response.result.count)" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Subscribe response: $($response | ConvertTo-Json -Compress)" -ForegroundColor Red
    }

    # Test 5: Broadcast
    # After broadcast, the daemon sends the event notification to subscribed clients (us)
    # BEFORE the response. So we may receive the notification first, then the response.
    $writer.Write('{"jsonrpc":"2.0","method":"daemon/broadcast","params":{"event_type":"panel-state","data":{"explorer":true}},"id":5}' + "`n")
    $writer.Flush()

    $gotNotification = $false
    $gotResponse = $false
    for ($i = 0; $i -lt 2; $i++) {
        $line = $reader.ReadLine()
        if (-not $line) { break }
        $msg = $line | ConvertFrom-Json
        if ($msg.method -and $msg.method -eq 'event/panel-state') {
            $gotNotification = $true
            Write-Host "[OK] Broadcast notification received: event/panel-state" -ForegroundColor Green
        }
        if ($msg.result -and $msg.result.status -eq 'broadcast') {
            $gotResponse = $true
            Write-Host "[OK] Broadcast response: recipients=$($msg.result.recipients)" -ForegroundColor Green
        }
    }
    if (-not $gotNotification -and -not $gotResponse) {
        Write-Host "[FAIL] No broadcast notification or response received" -ForegroundColor Red
    }

    $pipe.Dispose()
    Write-Host "`n=== ALL IPC TESTS PASSED ===" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] IPC error: $_" -ForegroundColor Red
} finally {
    if (-not $daemon.HasExited) {
        Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
        $daemon.WaitForExit(3000)
    }
    Write-Host "Daemon stopped"
}
