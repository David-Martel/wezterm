$exe = "C:\Users\david\bin\wezterm-utils-daemon.exe"
$pipeName = "wezterm-test-manual-$(Get-Random)"
$proc = Start-Process -FilePath $exe `
    -ArgumentList '--log-level', 'warn', 'start', '--pipe', "\\.\pipe\$pipeName" `
    -PassThru -NoNewWindow `
    -RedirectStandardOutput ([IO.Path]::GetTempFileName()) `
    -RedirectStandardError ([IO.Path]::GetTempFileName())

Start-Sleep -Seconds 3
Write-Host "PID: $($proc.Id), Exited: $($proc.HasExited)"

if ($proc.HasExited) {
    Write-Host "DAEMON DEAD" -ForegroundColor Red
    exit 1
}

try {
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', $pipeName, [System.IO.Pipes.PipeDirection]::InOut)
    $pipe.Connect(5000)
    Write-Host "CONNECTED to $pipeName" -ForegroundColor Green

    $writer = [System.IO.StreamWriter]::new($pipe)
    $reader = [System.IO.StreamReader]::new($pipe)

    $writer.Write('{"jsonrpc":"2.0","method":"daemon/ping","id":1}' + "`n")
    $writer.Flush()
    $line = $reader.ReadLine()
    Write-Host "PING RESPONSE: $line" -ForegroundColor Green

    $pipe.Dispose()
} catch {
    Write-Host "CONNECT FAILED: $_" -ForegroundColor Red
} finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}
