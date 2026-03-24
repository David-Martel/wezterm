#Requires -Version 5.1

<#
.SYNOPSIS
    Uninstall WezTerm utilities and remove configurations

.DESCRIPTION
    Safely removes WezTerm utilities:
    - Removes installed binaries
    - Optionally removes Lua modules
    - Optionally removes WezTerm configuration
    - Cleans up PATH environment variable
    - Creates backup before removal

    Features:
    - Dry-run mode to preview changes
    - Backup creation before removal
    - Selective uninstallation
    - Rollback capability

.PARAMETER KeepConfig
    Keep WezTerm configuration files (.wezterm.lua)

.PARAMETER KeepModules
    Keep Lua modules (wezterm-utils.lua)

.PARAMETER Backup
    Create backup before uninstalling (recommended)

.PARAMETER DryRun
    Show what would be removed without actually removing

.PARAMETER Force
    Skip confirmation prompts

.EXAMPLE
    .\uninstall.ps1
    Interactive uninstall with prompts

.EXAMPLE
    .\uninstall.ps1 -KeepConfig -KeepModules
    Remove only binaries, keep configuration

.EXAMPLE
    .\uninstall.ps1 -DryRun
    Preview what would be removed

.EXAMPLE
    .\uninstall.ps1 -Force -Backup
    Uninstall everything with backup, no prompts
#>

[CmdletBinding()]
param(
    [Parameter()]
    [switch]$KeepConfig,

    [Parameter()]
    [switch]$KeepModules,

    [Parameter()]
    [switch]$Backup = $true,

    [Parameter()]
    [switch]$DryRun,

    [Parameter()]
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# ============================================================================
# CONFIGURATION
# ============================================================================

$Script:Config = @{
    InstallPath = "$env:USERPROFILE\bin"
    WeztermConfigDir = "$env:USERPROFILE\.config\wezterm"
    WeztermConfigFile = "$env:USERPROFILE\.wezterm.lua"
    BackupDir = "$env:USERPROFILE\.wezterm-backup-$(Get-Date -Format 'yyyyMMdd-HHmmss')"

    Binaries = @(
        'wezterm-fs-explorer.exe',
        'wezterm-watch.exe'
    )

    LuaModules = @(
        'wezterm-utils.lua'
    )
    LuaModuleDirectories = @(
        'wezterm-utils'
    )

    ConfigFiles = @(
        '.wezterm.lua'
    )
}

$Script:Stats = @{
    BinariesRemoved = 0
    ModulesRemoved = 0
    ConfigsRemoved = 0
    BackupCreated = $false
}

# ============================================================================
# OUTPUT FUNCTIONS
# ============================================================================

function Write-Status {
    param(
        [string]$Message,
        [string]$Level = 'Info'
    )

    $color = switch ($Level) {
        'Success' { 'Green' }
        'Error' { 'Red' }
        'Warning' { 'Yellow' }
        'Info' { 'Cyan' }
        default { 'White' }
    }

    $prefix = switch ($Level) {
        'Success' { '✓' }
        'Error' { '✗' }
        'Warning' { '⚠' }
        'Info' { '→' }
        default { ' ' }
    }

    Write-Host "$prefix $Message" -ForegroundColor $color
}

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
}

# ============================================================================
# BACKUP FUNCTIONS
# ============================================================================

function New-Backup {
    Write-Section "Creating Backup"

    if ($DryRun) {
        Write-Status "DRY RUN: Would create backup at: $($Script:Config.BackupDir)" -Level Info
        return $true
    }

    try {
        # Create backup directory
        New-Item -ItemType Directory -Path $Script:Config.BackupDir -Force | Out-Null
        Write-Status "Created backup directory: $($Script:Config.BackupDir)" -Level Info

        $itemsBackedUp = 0

        # Backup binaries
        foreach ($binary in $Script:Config.Binaries) {
            $sourcePath = Join-Path $Script:Config.InstallPath $binary

            if (Test-Path $sourcePath) {
                $destPath = Join-Path $Script:Config.BackupDir "binaries\$binary"
                $destDir = Split-Path $destPath -Parent

                if (-not (Test-Path $destDir)) {
                    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
                }

                Copy-Item $sourcePath $destPath -Force
                Write-Status "Backed up: $binary" -Level Success
                $itemsBackedUp++
            }
        }

        # Backup Lua modules
        foreach ($module in $Script:Config.LuaModules) {
            $sourcePath = Join-Path $Script:Config.WeztermConfigDir $module

            if (Test-Path $sourcePath) {
                $destPath = Join-Path $Script:Config.BackupDir "lua-modules\$module"
                $destDir = Split-Path $destPath -Parent

                if (-not (Test-Path $destDir)) {
                    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
                }

                Copy-Item $sourcePath $destPath -Force
                Write-Status "Backed up: $module" -Level Success
                $itemsBackedUp++
            }
        }

        foreach ($moduleDir in $Script:Config.LuaModuleDirectories) {
            $sourcePath = Join-Path $Script:Config.WeztermConfigDir $moduleDir

            if (Test-Path $sourcePath) {
                $destPath = Join-Path $Script:Config.BackupDir "lua-modules\$moduleDir"
                $destDir = Split-Path $destPath -Parent

                if (-not (Test-Path $destDir)) {
                    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
                }

                Copy-Item $sourcePath $destPath -Recurse -Force
                Write-Status "Backed up: $moduleDir" -Level Success
                $itemsBackedUp++
            }
        }

        # Backup config file
        if (Test-Path $Script:Config.WeztermConfigFile) {
            $destPath = Join-Path $Script:Config.BackupDir ".wezterm.lua"
            Copy-Item $Script:Config.WeztermConfigFile $destPath -Force
            Write-Status "Backed up: .wezterm.lua" -Level Success
            $itemsBackedUp++
        }

        Write-Status "Backup created successfully ($itemsBackedUp items)" -Level Success
        $Script:Stats.BackupCreated = $true
        return $true

    } catch {
        Write-Status "Backup failed: $_" -Level Error
        return $false
    }
}

# ============================================================================
# REMOVAL FUNCTIONS
# ============================================================================

function Remove-Binaries {
    Write-Section "Removing Binaries"

    $removed = 0

    foreach ($binary in $Script:Config.Binaries) {
        $binaryPath = Join-Path $Script:Config.InstallPath $binary

        if (Test-Path $binaryPath) {
            if ($DryRun) {
                Write-Status "DRY RUN: Would remove $binary" -Level Warning
            } else {
                try {
                    Remove-Item $binaryPath -Force
                    Write-Status "Removed: $binary" -Level Success
                    $removed++
                } catch {
                    Write-Status "Failed to remove $binary`: $_" -Level Error
                }
            }
        } else {
            Write-Status "Not found: $binary (already removed)" -Level Info
        }
    }

    $Script:Stats.BinariesRemoved = $removed

    if ($removed -gt 0 -and -not $DryRun) {
        Write-Status "Removed $removed binaries" -Level Success
    }
}

function Remove-LuaModules {
    if ($KeepModules) {
        Write-Section "Keeping Lua Modules"
        Write-Status "Skipping Lua module removal (--KeepModules)" -Level Info
        return
    }

    Write-Section "Removing Lua Modules"

    $removed = 0

    foreach ($module in $Script:Config.LuaModules) {
        $modulePath = Join-Path $Script:Config.WeztermConfigDir $module

        if (Test-Path $modulePath) {
            if ($DryRun) {
                Write-Status "DRY RUN: Would remove $module" -Level Warning
            } else {
                try {
                    Remove-Item $modulePath -Force
                    Write-Status "Removed: $module" -Level Success
                    $removed++
                } catch {
                    Write-Status "Failed to remove $module`: $_" -Level Error
                }
            }
        } else {
            Write-Status "Not found: $module (already removed)" -Level Info
        }
    }

    foreach ($moduleDir in $Script:Config.LuaModuleDirectories) {
        $modulePath = Join-Path $Script:Config.WeztermConfigDir $moduleDir

        if (Test-Path $modulePath) {
            if ($DryRun) {
                Write-Status "DRY RUN: Would remove $moduleDir" -Level Warning
            } else {
                try {
                    Remove-Item $modulePath -Recurse -Force
                    Write-Status "Removed: $moduleDir" -Level Success
                    $removed++
                } catch {
                    Write-Status "Failed to remove $moduleDir`: $_" -Level Error
                }
            }
        } else {
            Write-Status "Not found: $moduleDir (already removed)" -Level Info
        }
    }

    $Script:Stats.ModulesRemoved = $removed

    if ($removed -gt 0 -and -not $DryRun) {
        Write-Status "Removed $removed Lua modules" -Level Success
    }
}

function Remove-Configuration {
    if ($KeepConfig) {
        Write-Section "Keeping Configuration"
        Write-Status "Skipping configuration removal (--KeepConfig)" -Level Info
        return
    }

    Write-Section "Removing Configuration"

    $configPath = $Script:Config.WeztermConfigFile

    if (Test-Path $configPath) {
        if ($DryRun) {
            Write-Status "DRY RUN: Would remove .wezterm.lua" -Level Warning
        } else {
            # Create an inline backup just in case
            $inlineBackup = "$configPath.pre-uninstall"

            try {
                Copy-Item $configPath $inlineBackup -Force
                Write-Status "Created inline backup: $inlineBackup" -Level Info

                Remove-Item $configPath -Force
                Write-Status "Removed: .wezterm.lua" -Level Success
                $Script:Stats.ConfigsRemoved++

                Write-Status "Note: Inline backup kept at $inlineBackup" -Level Warning
            } catch {
                Write-Status "Failed to remove .wezterm.lua: $_" -Level Error
            }
        }
    } else {
        Write-Status ".wezterm.lua not found (already removed)" -Level Info
    }
}

function Update-PathVariable {
    Write-Section "Cleaning PATH Environment"

    $installDir = $Script:Config.InstallPath
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')

    if ($userPath -like "*$installDir*") {
        if ($DryRun) {
            Write-Status "DRY RUN: Would remove $installDir from PATH" -Level Warning
        } else {
            try {
                # Remove installation directory from PATH
                $newPath = ($userPath -split ';' | Where-Object { $_ -ne $installDir }) -join ';'

                [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
                Write-Status "Removed $installDir from user PATH" -Level Success
                Write-Status "Restart terminal for PATH changes to take effect" -Level Warning
            } catch {
                Write-Status "Failed to update PATH: $_" -Level Error
            }
        }
    } else {
        Write-Status "$installDir not in user PATH (already cleaned)" -Level Info
    }
}

# ============================================================================
# CONFIRMATION
# ============================================================================

function Get-UninstallConfirmation {
    if ($Force) {
        return $true
    }

    Write-Section "Uninstallation Summary"

    Write-Host ""
    Write-Host "  The following will be removed:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  Binaries:" -ForegroundColor Cyan
    foreach ($binary in $Script:Config.Binaries) {
        Write-Host "    - $binary" -ForegroundColor White
    }

    if (-not $KeepModules) {
        Write-Host ""
        Write-Host "  Lua Modules:" -ForegroundColor Cyan
        foreach ($module in $Script:Config.LuaModules) {
            Write-Host "    - $module" -ForegroundColor White
        }
    }

    if (-not $KeepConfig) {
        Write-Host ""
        Write-Host "  Configuration:" -ForegroundColor Cyan
        Write-Host "    - .wezterm.lua" -ForegroundColor White
    }

    Write-Host ""
    Write-Host "  PATH Cleanup:" -ForegroundColor Cyan
    Write-Host "    - Remove $($Script:Config.InstallPath) from user PATH" -ForegroundColor White

    if ($Backup -and -not $DryRun) {
        Write-Host ""
        Write-Host "  Backup will be created at:" -ForegroundColor Green
        Write-Host "    $($Script:Config.BackupDir)" -ForegroundColor White
    }

    Write-Host ""
    $response = Read-Host "Continue with uninstallation? (y/N)"

    return $response -eq 'y' -or $response -eq 'Y'
}

# ============================================================================
# SUMMARY
# ============================================================================

function Write-UninstallSummary {
    Write-Section "Uninstallation Summary"

    if ($DryRun) {
        Write-Status "DRY RUN COMPLETED - No files were actually removed" -Level Warning
        return 0
    }

    $total = $Script:Stats.BinariesRemoved + $Script:Stats.ModulesRemoved + $Script:Stats.ConfigsRemoved

    Write-Host ""
    Write-Host "  Binaries removed: $($Script:Stats.BinariesRemoved)" -ForegroundColor Cyan
    Write-Host "  Modules removed: $($Script:Stats.ModulesRemoved)" -ForegroundColor Cyan
    Write-Host "  Configs removed: $($Script:Stats.ConfigsRemoved)" -ForegroundColor Cyan
    Write-Host "  Total items removed: $total" -ForegroundColor Cyan

    if ($Script:Stats.BackupCreated) {
        Write-Host ""
        Write-Status "Backup location: $($Script:Config.BackupDir)" -Level Success
    }

    Write-Host ""
    Write-Status "WezTerm utilities uninstalled successfully" -Level Success
    Write-Host ""
    Write-Status "To reinstall, run: .\build-all.ps1" -Level Info
    Write-Status "Restart your terminal for PATH changes to take effect" -Level Info
    Write-Host ""

    return 0
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

function Invoke-Uninstall {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║        WezTerm Utilities - Uninstaller                        ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

    if ($DryRun) {
        Write-Host ""
        Write-Status "Running in DRY RUN mode - no files will be removed" -Level Warning
    }

    try {
        # Get confirmation
        if (-not (Get-UninstallConfirmation)) {
            Write-Status "Uninstallation cancelled by user" -Level Warning
            return 1
        }

        # Create backup if requested
        if ($Backup -and -not $DryRun) {
            $backupSuccess = New-Backup
            if (-not $backupSuccess) {
                Write-Status "Backup failed - aborting uninstallation" -Level Error
                return 1
            }
        }

        # Perform removal
        Remove-Binaries
        Remove-LuaModules
        Remove-Configuration
        Update-PathVariable

        # Show summary
        $exitCode = Write-UninstallSummary
        return $exitCode

    } catch {
        Write-Section "Uninstallation Failed"
        Write-Status "Error: $_" -Level Error

        if ($Script:Stats.BackupCreated) {
            Write-Host ""
            Write-Status "Backup available at: $($Script:Config.BackupDir)" -Level Info
            Write-Status "You can manually restore files from the backup if needed" -Level Info
        }

        return 1
    }
}

# ============================================================================
# SCRIPT ENTRY POINT
# ============================================================================

$exitCode = Invoke-Uninstall
exit $exitCode
