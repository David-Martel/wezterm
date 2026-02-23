#!/usr/bin/env pwsh
# WezTerm Utilities Installer
# Production deployment script for WezTerm utilities system

param(
    [switch]$Uninstall,
    [switch]$Dev,  # Development mode (symlinks instead of copy)
    [switch]$SkipBackup,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Paths
$INSTALL_DIR = "$env:USERPROFILE\.local\wezterm-utilities"
$BIN_DIR = "$env:USERPROFILE\.local\bin"
$CONFIG_DIR = "$env:USERPROFILE\.config\wezterm"
$STATE_DIR = "$CONFIG_DIR\wezterm-utils-state"
$BACKUP_DIR = "$env:USERPROFILE\.wezterm-backup"

# Colors for output
function Write-Success { param($Message) Write-Host "  ✓ $Message" -ForegroundColor Green }
function Write-Error { param($Message) Write-Host "  ✗ $Message" -ForegroundColor Red }
function Write-Warning { param($Message) Write-Host "  ⚠ $Message" -ForegroundColor Yellow }
function Write-Info { param($Message) Write-Host "  → $Message" -ForegroundColor Cyan }
function Write-Header { param($Message) Write-Host "`n$Message" -ForegroundColor Cyan }

function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Backup-Existing {
    if ($SkipBackup) {
        Write-Warning "Skipping backup (--SkipBackup specified)"
        return
    }

    Write-Header "Creating backup..."

    # Create backup directory with timestamp
    $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $backupPath = "$BACKUP_DIR\backup_$timestamp"
    New-Item -ItemType Directory -Force -Path $backupPath | Out-Null

    # Backup existing binaries
    if (Test-Path $BIN_DIR) {
        $binaries = @(
            "wezterm-fs-explorer.exe",
            "wezterm-watch.exe"
        )

        foreach ($bin in $binaries) {
            $source = Join-Path $BIN_DIR $bin
            if (Test-Path $source) {
                Copy-Item $source (Join-Path $backupPath "bin_$bin") -Force
                Write-Info "Backed up $bin"
            }
        }
    }

    # Backup .wezterm.lua
    $weztermConfig = "$env:USERPROFILE\.wezterm.lua"
    if (Test-Path $weztermConfig) {
        Copy-Item $weztermConfig (Join-Path $backupPath ".wezterm.lua") -Force
        Write-Info "Backed up .wezterm.lua"
    }

    # Backup Lua modules
    if (Test-Path "$CONFIG_DIR\wezterm-utils.lua") {
        Copy-Item "$CONFIG_DIR\wezterm-utils.lua" (Join-Path $backupPath "wezterm-utils.lua") -Force
        Write-Info "Backed up Lua modules"
    }

    Write-Success "Backup created at $backupPath"

    # Create restore script
    $restoreScript = @"
#!/usr/bin/env pwsh
# Restore from backup: $timestamp
Write-Host "Restoring from backup $timestamp..." -ForegroundColor Cyan

Copy-Item "$backupPath\bin_*" "$BIN_DIR\" -Force
Copy-Item "$backupPath\.wezterm.lua" "$env:USERPROFILE\" -Force -ErrorAction SilentlyContinue
Copy-Item "$backupPath\wezterm-utils.lua" "$CONFIG_DIR\" -Force -ErrorAction SilentlyContinue

Write-Host "✓ Restore complete. Please restart WezTerm." -ForegroundColor Green
"@

    Set-Content -Path (Join-Path $backupPath "restore.ps1") -Value $restoreScript
    Write-Info "Restore script: $backupPath\restore.ps1"
}

function Install-WezTermUtilities {
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║       WezTerm Utilities Installer v1.0.0                ║" -ForegroundColor Cyan
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

    # Backup existing installation
    Backup-Existing

    # Create directories
    Write-Header "Creating directories..."
    $directories = @($BIN_DIR, $CONFIG_DIR, $STATE_DIR, $INSTALL_DIR)
    foreach ($dir in $directories) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Write-Success "Created $dir"
    }

    # Install binaries
    Write-Header "Installing binaries..."

    # Check if we're running from the installer directory
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $binSource = Join-Path $scriptDir "wezterm-utils\bin"

    if (-not (Test-Path $binSource)) {
        Write-Error "Binary source directory not found: $binSource"
        Write-Host "Please ensure you're running the installer from the correct directory" -ForegroundColor Red
        exit 1
    }

    # Copy/symlink binaries
    $binaries = Get-ChildItem -Path $binSource -Filter "*.exe"
    if ($binaries.Count -eq 0) {
        Write-Error "No binaries found in $binSource"
        Write-Host "Please build the binaries first using the build script" -ForegroundColor Red
        exit 1
    }

    foreach ($binary in $binaries) {
        $dest = Join-Path $BIN_DIR $binary.Name

        if ($Dev) {
            # Create symlink in dev mode
            if (Test-Path $dest) { Remove-Item $dest -Force }
            New-Item -ItemType SymbolicLink -Path $dest -Target $binary.FullName -Force | Out-Null
            Write-Info "Symlinked $($binary.Name) (dev mode)"
        } else {
            # Copy in production mode
            Copy-Item $binary.FullName $dest -Force
            Write-Success "Installed $($binary.Name)"
        }
    }

    # Verify binaries are executable
    Write-Header "Verifying binaries..."
    foreach ($binary in $binaries) {
        $binPath = Join-Path $BIN_DIR $binary.Name
        try {
            $version = & $binPath --version 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Success "$($binary.Name) - OK"
            } else {
                Write-Warning "$($binary.Name) - returned exit code $LASTEXITCODE"
            }
        } catch {
            Write-Warning "$($binary.Name) - failed to execute: $($_.Exception.Message)"
        }
    }

    # Install Lua modules
    Write-Header "Installing Lua modules..."
    $luaSource = Join-Path $scriptDir "wezterm-utils\lua"

    if (Test-Path $luaSource) {
        Get-ChildItem -Path $luaSource -Recurse | ForEach-Object {
            $relativePath = $_.FullName.Substring($luaSource.Length + 1)
            $destPath = Join-Path $CONFIG_DIR $relativePath

            if ($_.PSIsContainer) {
                New-Item -ItemType Directory -Force -Path $destPath | Out-Null
            } else {
                Copy-Item $_.FullName $destPath -Force
                Write-Info "Installed Lua module: $relativePath"
            }
        }
        Write-Success "Lua modules installed"
    } else {
        Write-Warning "Lua modules not found at $luaSource"
    }

    # Install default configuration
    Write-Header "Installing configuration..."
    $configSource = Join-Path $scriptDir "wezterm-utils\config\wezterm-utils-config.json"
    $configDest = Join-Path $CONFIG_DIR "wezterm-utils-config.json"

    if (Test-Path $configSource) {
        if (-not (Test-Path $configDest)) {
            Copy-Item $configSource $configDest -Force
            Write-Success "Installed default configuration"
        } else {
            Write-Info "Configuration already exists (not overwriting)"
        }
    }

    # Check WezTerm integration
    Write-Header "Checking WezTerm integration..."
    $weztermConfig = "$env:USERPROFILE\.wezterm.lua"

    if (Test-Path $weztermConfig) {
        $content = Get-Content $weztermConfig -Raw

        if ($content -match "require\s*\(\s*['\`"]wezterm-utils['\`"]\s*\)") {
            Write-Success "wezterm-utils already integrated"
        } else {
            Write-Warning "wezterm-utils not integrated in .wezterm.lua"
            Write-Host "`nTo integrate, add this line to your .wezterm.lua:" -ForegroundColor Yellow
            Write-Host "    local utils = require('wezterm-utils')" -ForegroundColor White
            Write-Host "    utils.setup(config)" -ForegroundColor White
            Write-Host "`nSee docs/INTEGRATION_GUIDE.md for details`n" -ForegroundColor Cyan
        }
    } else {
        Write-Warning ".wezterm.lua not found"
        Write-Info "You'll need to create a .wezterm.lua configuration file"
    }

    # Add to PATH if needed
    Write-Header "Checking PATH configuration..."
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -notlike "*$BIN_DIR*") {
        Write-Warning "$BIN_DIR not in PATH"
        Write-Host "Would you like to add it to your PATH? (Y/N): " -NoNewline -ForegroundColor Yellow
        $response = Read-Host
        if ($response -eq "Y" -or $response -eq "y") {
            [Environment]::SetEnvironmentVariable(
                "PATH",
                "$currentPath;$BIN_DIR",
                "User"
            )
            Write-Success "Added to PATH (restart terminal to take effect)"
        }
    } else {
        Write-Success "Already in PATH"
    }

    # Final summary
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║           ✓ Installation Successful!                    ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green

    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "  1. Restart WezTerm to load new utilities" -ForegroundColor White
    Write-Host "  2. Press Alt+E to open filesystem explorer" -ForegroundColor White
    Write-Host "  3. See docs\QUICKSTART.md for usage guide" -ForegroundColor White

    if ($Dev) {
        Write-Host "`n  Development mode: Binaries are symlinked" -ForegroundColor Yellow
        Write-Host "  Rebuild binaries to update installation" -ForegroundColor Yellow
    }

    Write-Host "`nInstallation directory: $INSTALL_DIR" -ForegroundColor Gray
    Write-Host "Binary directory: $BIN_DIR" -ForegroundColor Gray
    Write-Host "Configuration: $CONFIG_DIR" -ForegroundColor Gray
}

function Uninstall-WezTermUtilities {
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Yellow
    Write-Host "║       WezTerm Utilities Uninstaller                     ║" -ForegroundColor Yellow
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Yellow

    Write-Host "`nThis will remove all WezTerm utilities binaries and modules." -ForegroundColor Yellow
    Write-Host "Configuration and state files will be preserved." -ForegroundColor Yellow
    Write-Host "`nAre you sure? (Y/N): " -NoNewline -ForegroundColor Yellow
    $response = Read-Host

    if ($response -ne "Y" -and $response -ne "y") {
        Write-Host "Uninstall cancelled" -ForegroundColor Cyan
        exit 0
    }

    Write-Header "Uninstalling WezTerm Utilities..."

    # Remove binaries
    $binaries = @(
        "wezterm-fs-explorer.exe",
        "wezterm-watch.exe"
    )

    foreach ($bin in $binaries) {
        $path = Join-Path $BIN_DIR $bin
        if (Test-Path $path) {
            Remove-Item $path -Force
            Write-Success "Removed $bin"
        }
    }

    # Remove Lua modules
    if (Test-Path "$CONFIG_DIR\wezterm-utils.lua") {
        Remove-Item "$CONFIG_DIR\wezterm-utils.lua" -Force
        Write-Success "Removed wezterm-utils.lua"
    }

    if (Test-Path "$CONFIG_DIR\wezterm-utils") {
        Remove-Item "$CONFIG_DIR\wezterm-utils" -Recurse -Force
        Write-Success "Removed Lua modules directory"
    }

    # Note about preserved files
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║           ✓ Uninstallation Complete                     ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green

    Write-Host "`nPreserved for reinstallation:" -ForegroundColor Cyan
    Write-Host "  • Configuration: $CONFIG_DIR\wezterm-utils-config.json" -ForegroundColor White
    Write-Host "  • State files: $STATE_DIR" -ForegroundColor White
    Write-Host "  • Backups: $BACKUP_DIR" -ForegroundColor White

    Write-Host "`nTo completely remove all files, manually delete:" -ForegroundColor Gray
    Write-Host "  $CONFIG_DIR" -ForegroundColor Gray
    Write-Host "  $STATE_DIR" -ForegroundColor Gray
    Write-Host "  $BACKUP_DIR" -ForegroundColor Gray
}

# Main execution
try {
    if ($Uninstall) {
        Uninstall-WezTermUtilities
    } else {
        Install-WezTermUtilities
    }
} catch {
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║           ✗ Installation Failed                         ║" -ForegroundColor Red
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Red
    Write-Host "`nError: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "`nStack trace:" -ForegroundColor Gray
    Write-Host $_.ScriptStackTrace -ForegroundColor Gray
    exit 1
}