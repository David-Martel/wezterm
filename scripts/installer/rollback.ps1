#!/usr/bin/env pwsh
# WezTerm Utilities Rollback Script
# Restore from backup in case of installation issues

param(
    [string]$BackupTimestamp,  # Specific backup to restore
    [switch]$ListBackups,       # List available backups
    [switch]$Latest,            # Restore latest backup
    [switch]$Force              # Skip confirmation
)

$ErrorActionPreference = "Stop"

# Paths
$BACKUP_DIR = "$env:USERPROFILE\.wezterm-backup"
$BIN_DIR = "$env:USERPROFILE\.local\bin"
$CONFIG_DIR = "$env:USERPROFILE\.config\wezterm"

function Write-Success { param($Message) Write-Host "  ✓ $Message" -ForegroundColor Green }
function Write-Error { param($Message) Write-Host "  ✗ $Message" -ForegroundColor Red }
function Write-Warning { param($Message) Write-Host "  ⚠ $Message" -ForegroundColor Yellow }
function Write-Info { param($Message) Write-Host "  → $Message" -ForegroundColor Cyan }
function Write-Header { param($Message) Write-Host "`n$Message" -ForegroundColor Cyan }

function Get-AvailableBackups {
    if (-not (Test-Path $BACKUP_DIR)) {
        return @()
    }

    $backups = Get-ChildItem $BACKUP_DIR -Directory | Where-Object {
        $_.Name -match "backup_(\d{8}_\d{6})"
    } | ForEach-Object {
        [PSCustomObject]@{
            Timestamp = $matches[1]
            Path = $_.FullName
            Created = $_.CreationTime
            Size = (Get-ChildItem $_.FullName -Recurse -File | Measure-Object -Property Length -Sum).Sum
        }
    } | Sort-Object Created -Descending

    return $backups
}

function Show-AvailableBackups {
    Write-Header "Available Backups"

    $backups = Get-AvailableBackups

    if ($backups.Count -eq 0) {
        Write-Warning "No backups found in $BACKUP_DIR"
        return $null
    }

    Write-Host "`nFound $($backups.Count) backup(s):" -ForegroundColor White
    Write-Host ""

    $index = 1
    foreach ($backup in $backups) {
        $sizeStr = if ($backup.Size -gt 1MB) {
            "$([math]::Round($backup.Size/1MB, 2)) MB"
        } else {
            "$([math]::Round($backup.Size/1KB, 2)) KB"
        }

        Write-Host "  [$index] $($backup.Timestamp)" -ForegroundColor White
        Write-Host "      Created: $($backup.Created)" -ForegroundColor Gray
        Write-Host "      Size: $sizeStr" -ForegroundColor Gray
        Write-Host "      Path: $($backup.Path)" -ForegroundColor DarkGray
        Write-Host ""

        $index++
    }

    return $backups
}

function Restore-FromBackup {
    param(
        [Parameter(Mandatory=$true)]
        [string]$BackupPath
    )

    Write-Header "Restoring from backup..."
    Write-Info "Backup location: $BackupPath"

    if (-not (Test-Path $BackupPath)) {
        Write-Error "Backup directory not found: $BackupPath"
        exit 1
    }

    $restored = 0

    # Restore binaries
    Write-Host "`nRestoring binaries..." -ForegroundColor Cyan
    $binBackups = Get-ChildItem $BackupPath -Filter "bin_*.exe"

    foreach ($binBackup in $binBackups) {
        $originalName = $binBackup.Name.Substring(4)  # Remove "bin_" prefix
        $destPath = Join-Path $BIN_DIR $originalName

        try {
            # Stop any running processes using the binary
            $processName = [System.IO.Path]::GetFileNameWithoutExtension($originalName)
            $processes = Get-Process -Name $processName -ErrorAction SilentlyContinue
            if ($processes) {
                Write-Warning "Stopping running process: $processName"
                $processes | Stop-Process -Force
                Start-Sleep -Milliseconds 500
            }

            Copy-Item $binBackup.FullName $destPath -Force
            Write-Success "Restored $originalName"
            $restored++
        } catch {
            Write-Error "Failed to restore $originalName`: $($_.Exception.Message)"
        }
    }

    # Restore .wezterm.lua
    Write-Host "`nRestoring configuration..." -ForegroundColor Cyan
    $weztermLuaBackup = Join-Path $BackupPath ".wezterm.lua"
    if (Test-Path $weztermLuaBackup) {
        try {
            $dest = "$env:USERPROFILE\.wezterm.lua"
            Copy-Item $weztermLuaBackup $dest -Force
            Write-Success "Restored .wezterm.lua"
            $restored++
        } catch {
            Write-Error "Failed to restore .wezterm.lua: $($_.Exception.Message)"
        }
    }

    # Restore Lua modules
    $luaModuleBackup = Join-Path $BackupPath "wezterm-utils.lua"
    if (Test-Path $luaModuleBackup) {
        try {
            $dest = Join-Path $CONFIG_DIR "wezterm-utils.lua"
            Copy-Item $luaModuleBackup $dest -Force
            Write-Success "Restored wezterm-utils.lua"
            $restored++
        } catch {
            Write-Error "Failed to restore Lua module: $($_.Exception.Message)"
        }
    }

    return $restored
}

# Main execution
Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Yellow
Write-Host "║       WezTerm Utilities Rollback Tool v1.0.0            ║" -ForegroundColor Yellow
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Yellow

# List backups mode
if ($ListBackups) {
    Show-AvailableBackups
    exit 0
}

# Get available backups
$backups = Get-AvailableBackups

if ($backups.Count -eq 0) {
    Write-Error "No backups found in $BACKUP_DIR"
    Write-Host "`nBackups are created automatically during installation." -ForegroundColor Gray
    Write-Host "Install the utilities first to create a backup." -ForegroundColor Gray
    exit 1
}

# Determine which backup to restore
$backupToRestore = $null

if ($BackupTimestamp) {
    # Restore specific backup
    $backupToRestore = $backups | Where-Object { $_.Timestamp -eq $BackupTimestamp } | Select-Object -First 1
    if (-not $backupToRestore) {
        Write-Error "Backup with timestamp '$BackupTimestamp' not found"
        Write-Host "`nAvailable backups:" -ForegroundColor Yellow
        Show-AvailableBackups | Out-Null
        exit 1
    }
} elseif ($Latest) {
    # Restore latest backup
    $backupToRestore = $backups | Select-Object -First 1
} else {
    # Interactive selection
    $backupList = Show-AvailableBackups

    Write-Host "Select backup to restore (1-$($backupList.Count)), or 0 to cancel: " -NoNewline -ForegroundColor Yellow
    $selection = Read-Host

    if ($selection -eq "0" -or [string]::IsNullOrWhiteSpace($selection)) {
        Write-Host "Rollback cancelled" -ForegroundColor Cyan
        exit 0
    }

    $selectionNum = [int]$selection
    if ($selectionNum -lt 1 -or $selectionNum -gt $backupList.Count) {
        Write-Error "Invalid selection"
        exit 1
    }

    $backupToRestore = $backupList[$selectionNum - 1]
}

# Confirm restoration
if (-not $Force) {
    Write-Host "`nYou are about to restore from backup:" -ForegroundColor Yellow
    Write-Host "  Timestamp: $($backupToRestore.Timestamp)" -ForegroundColor White
    Write-Host "  Created: $($backupToRestore.Created)" -ForegroundColor White
    Write-Host "`nThis will overwrite current files. Continue? (Y/N): " -NoNewline -ForegroundColor Yellow
    $response = Read-Host

    if ($response -ne "Y" -and $response -ne "y") {
        Write-Host "Rollback cancelled" -ForegroundColor Cyan
        exit 0
    }
}

# Perform restoration
try {
    $restored = Restore-FromBackup -BackupPath $backupToRestore.Path

    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║           ✓ Rollback Successful!                        ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green

    Write-Host "`nRestored $restored file(s) from backup" -ForegroundColor White
    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "  1. Restart WezTerm to apply changes" -ForegroundColor White
    Write-Host "  2. Test functionality" -ForegroundColor White
    Write-Host "  3. Run validate-deployment.ps1 to verify" -ForegroundColor White

    exit 0
} catch {
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║           ✗ Rollback Failed                             ║" -ForegroundColor Red
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Red

    Write-Host "`nError: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "`nManual restoration may be required." -ForegroundColor Yellow
    Write-Host "Backup location: $($backupToRestore.Path)" -ForegroundColor Gray

    exit 1
}