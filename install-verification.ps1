#Requires -Version 5.1

<#
.SYNOPSIS
    Comprehensive verification script for WezTerm utilities installation

.DESCRIPTION
    Verifies that all WezTerm utilities are properly installed and functional:
    - Binary existence and PATH configuration
    - Version checks and execution tests
    - Configuration file validation
    - Lua module integration
    - WezTerm configuration syntax

    Returns detailed status report with recommendations

.PARAMETER Verbose
    Show detailed verification output

.PARAMETER Fix
    Attempt to fix common issues automatically

.EXAMPLE
    .\install-verification.ps1
    Run verification with standard output

.EXAMPLE
    .\install-verification.ps1 -Verbose -Fix
    Run verification with detailed output and auto-fix
#>

[CmdletBinding()]
param(
    [Parameter()]
    [switch]$Fix,

    [Parameter()]
    [switch]$Detailed
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

    Binaries = @(
        @{
            Name = 'wezterm-fs-explorer'
            File = 'wezterm-fs-explorer.exe'
            VersionFlag = '--version'
            Description = 'Filesystem Explorer'
        },
        @{
            Name = 'wezterm-watch'
            File = 'wezterm-watch.exe'
            VersionFlag = '--version'
            Description = 'File Watcher'
        }
    )

    LuaModules = @(
        @{
            Name = 'wezterm-utils'
            File = 'wezterm-utils.lua'
            Description = 'Utilities integration module'
        }
    )
    LuaModuleDirectories = @(
        @{
            Name = 'wezterm-utils-submodules'
            File = 'wezterm-utils'
            Description = 'Utilities support module tree'
        }
    )

    RequiredFiles = @(
        '.wezterm.lua'
    )
}

$Script:Results = @{
    Binaries = @{}
    LuaModules = @{}
    Configuration = @{}
    PATH = @{}
    Overall = $true
}

# ============================================================================
# OUTPUT FUNCTIONS
# ============================================================================

function Write-TestResult {
    param(
        [string]$Test,
        [bool]$Passed,
        [string]$Message = '',
        [string]$Details = ''
    )

    $status = if ($Passed) { '✓ PASS' } else { '✗ FAIL' }
    $color = if ($Passed) { 'Green' } else { 'Red' }

    Write-Host "  $status - $Test" -ForegroundColor $color

    if ($Message) {
        Write-Host "         $Message" -ForegroundColor DarkGray
    }

    if ($Details -and $Detailed) {
        Write-Host "         Details: $Details" -ForegroundColor DarkGray
    }

    if (-not $Passed) {
        $Script:Results.Overall = $false
    }

    return $Passed
}

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
}

function Write-Recommendation {
    param([string]$Message)
    Write-Host "  ⚡ Recommendation: $Message" -ForegroundColor Yellow
}

# ============================================================================
# VERIFICATION TESTS
# ============================================================================

function Test-PathConfiguration {
    Write-Section "PATH Configuration"

    $installDir = $Script:Config.InstallPath
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $currentPath = $env:Path

    # Test 1: Installation directory exists
    $dirExists = Test-Path $installDir
    Write-TestResult -Test "Installation directory exists" -Passed $dirExists `
        -Message $installDir

    if (-not $dirExists) {
        Write-Recommendation "Run build-all.ps1 to create installation directory"
        $Script:Results.PATH['DirectoryExists'] = $false
        return $false
    }

    # Test 2: Installation directory in user PATH
    $inUserPath = $userPath -like "*$installDir*"
    Write-TestResult -Test "Installation directory in user PATH" -Passed $inUserPath `
        -Message "User PATH includes: $installDir"

    $Script:Results.PATH['InUserPath'] = $inUserPath

    # Test 3: Installation directory in current session PATH
    $inCurrentPath = $currentPath -like "*$installDir*"
    Write-TestResult -Test "Installation directory in current session" -Passed $inCurrentPath `
        -Message "Current session PATH includes: $installDir"

    if (-not $inCurrentPath -and $inUserPath) {
        Write-Recommendation "Restart your terminal to update PATH for current session"
    }

    $Script:Results.PATH['InCurrentPath'] = $inCurrentPath

    return $dirExists -and $inUserPath
}

function Test-BinaryInstallation {
    Write-Section "Binary Installation"

    $allPassed = $true

    foreach ($binary in $Script:Config.Binaries) {
        Write-Host ""
        Write-Host "  Testing $($binary.Description)..." -ForegroundColor Cyan

        $binaryPath = Join-Path $Script:Config.InstallPath $binary.File

        # Test 1: Binary file exists
        $exists = Test-Path $binaryPath
        Write-TestResult -Test "Binary file exists" -Passed $exists `
            -Message $binaryPath

        if (-not $exists) {
            Write-Recommendation "Run build-all.ps1 to build and install $($binary.Name)"
            $Script:Results.Binaries[$binary.Name] = @{ Exists = $false }
            $allPassed = $false
            continue
        }

        # Test 2: Binary is executable
        $fileInfo = Get-Item $binaryPath
        $isExecutable = $fileInfo.Extension -eq '.exe'
        Write-TestResult -Test "Binary is executable" -Passed $isExecutable `
            -Details "File extension: $($fileInfo.Extension)"

        # Test 3: Binary can be executed
        $canExecute = $false
        $version = ''
        try {
            $output = & $binaryPath $binary.VersionFlag 2>&1
            if ($LASTEXITCODE -eq 0) {
                $canExecute = $true
                $version = $output | Out-String
            }
        } catch {
            $canExecute = $false
        }

        Write-TestResult -Test "Binary executes successfully" -Passed $canExecute `
            -Message $(if ($version) { $version.Trim() } else { '' })

        # Test 4: Binary accessible from PATH
        $inPath = $null -ne (Get-Command $binary.File -ErrorAction SilentlyContinue)
        Write-TestResult -Test "Binary accessible from PATH" -Passed $inPath

        if (-not $inPath) {
            Write-Recommendation "Restart terminal or run: refreshenv (if using chocolatey)"
        }

        $Script:Results.Binaries[$binary.Name] = @{
            Exists = $exists
            Executable = $isExecutable
            CanExecute = $canExecute
            InPath = $inPath
            Version = $version
        }

        if (-not ($exists -and $isExecutable -and $canExecute)) {
            $allPassed = $false
        }
    }

    return $allPassed
}

function Test-LuaModules {
    Write-Section "Lua Module Installation"

    $allPassed = $true
    $configDir = $Script:Config.WeztermConfigDir

    # Test 1: Config directory exists
    $dirExists = Test-Path $configDir
    Write-TestResult -Test "WezTerm config directory exists" -Passed $dirExists `
        -Message $configDir

    if (-not $dirExists) {
        if ($Fix) {
            Write-Host "  → Creating config directory..." -ForegroundColor Yellow
            New-Item -ItemType Directory -Path $configDir -Force | Out-Null
            $dirExists = $true
        } else {
            Write-Recommendation "Run build-all.ps1 to create config directory"
        }
    }

    # Test 2: Lua modules installed
    foreach ($module in $Script:Config.LuaModules) {
        $modulePath = Join-Path $configDir $module.File
        $exists = Test-Path $modulePath

        Write-TestResult -Test "$($module.Description) installed" -Passed $exists `
            -Message $modulePath

        if (-not $exists) {
            Write-Recommendation "Run build-all.ps1 to install Lua modules"
            $allPassed = $false
        }

        $Script:Results.LuaModules[$module.Name] = @{
            Exists = $exists
            Path = $modulePath
        }
    }

    foreach ($moduleDir in $Script:Config.LuaModuleDirectories) {
        $modulePath = Join-Path $configDir $moduleDir.File
        $exists = Test-Path $modulePath

        Write-TestResult -Test "$($moduleDir.Description) installed" -Passed $exists `
            -Message $modulePath

        if (-not $exists) {
            Write-Recommendation "Run build-all.ps1 to install Lua module directories"
            $allPassed = $false
        }

        $Script:Results.LuaModules[$moduleDir.Name] = @{
            Exists = $exists
            Path = $modulePath
        }
    }

    return $allPassed
}

function Test-WeztermConfiguration {
    Write-Section "WezTerm Configuration"

    $configPath = $Script:Config.WeztermConfigFile

    # Test 1: Configuration file exists
    $exists = Test-Path $configPath
    Write-TestResult -Test "WezTerm configuration file exists" -Passed $exists `
        -Message $configPath

    if (-not $exists) {
        Write-Recommendation "Ensure $configPath exists before launching WezTerm"
        $Script:Results.Configuration['Exists'] = $false
        return $false
    }

    # Test 2: Configuration loads without errors
    $configValid = $false
    $configError = ''

    try {
        # Try to validate with wezterm if available
        $weztermAvailable = $null -ne (Get-Command 'wezterm' -ErrorAction SilentlyContinue)

        if ($weztermAvailable) {
            $output = wezterm show-config 2>&1
            if ($LASTEXITCODE -eq 0) {
                $configValid = $true
            } else {
                $configError = $output | Out-String
            }
        } else {
            # Basic Lua syntax check
            $content = Get-Content $configPath -Raw

            # Check for basic Lua syntax issues
            if ($content -match 'require.*wezterm-utils' -and
                $content -match 'utils_available' -and
                $content -match 'return config') {
                $configValid = $true
            } else {
                $configError = "Configuration may be incomplete"
            }
        }
    } catch {
        $configError = $_.Exception.Message
    }

    Write-TestResult -Test "Configuration syntax valid" -Passed $configValid `
        -Details $configError

    # Test 3: Utilities integration present
    $content = Get-Content $configPath -Raw
    $hasIntegration = $content -match 'wezterm-utils' -or $content -match 'utils_available'

    Write-TestResult -Test "Utilities integration present" -Passed $hasIntegration

    if (-not $hasIntegration) {
        Write-Recommendation "Update .wezterm.lua to include utilities integration"
    }

    $Script:Results.Configuration['Exists'] = $exists
    $Script:Results.Configuration['Valid'] = $configValid
    $Script:Results.Configuration['HasIntegration'] = $hasIntegration

    return $exists -and $configValid
}

function Test-WeztermInstallation {
    Write-Section "WezTerm Installation"

    # Test: WezTerm executable available
    $weztermAvailable = $null -ne (Get-Command 'wezterm' -ErrorAction SilentlyContinue)

    if ($weztermAvailable) {
        try {
            $version = wezterm --version 2>&1
            Write-TestResult -Test "WezTerm installed" -Passed $true `
                -Message $version
        } catch {
            Write-TestResult -Test "WezTerm installed" -Passed $false
        }
    } else {
        Write-TestResult -Test "WezTerm installed" -Passed $false `
            -Message "wezterm not found in PATH"

        Write-Recommendation "Install WezTerm from https://wezfurlong.org/wezterm/"
    }

    return $weztermAvailable
}

# ============================================================================
# SUMMARY AND RECOMMENDATIONS
# ============================================================================

function Write-Summary {
    Write-Section "Verification Summary"

    $totalTests = 0
    $passedTests = 0

    # Count results
    foreach ($category in @('Binaries', 'LuaModules', 'Configuration', 'PATH')) {
        $results = $Script:Results[$category]

        if ($results -is [hashtable]) {
            foreach ($item in $results.Values) {
                if ($item -is [hashtable]) {
                    foreach ($test in $item.Values) {
                        $totalTests++
                        if ($test -eq $true) {
                            $passedTests++
                        }
                    }
                } elseif ($item -is [bool]) {
                    $totalTests++
                    if ($item) {
                        $passedTests++
                    }
                }
            }
        }
    }

    $percentPassed = if ($totalTests -gt 0) {
        [math]::Round(($passedTests / $totalTests) * 100, 1)
    } else {
        0
    }

    Write-Host ""
    Write-Host "  Total Tests: $totalTests" -ForegroundColor Cyan
    Write-Host "  Passed: $passedTests" -ForegroundColor Green
    Write-Host "  Failed: $($totalTests - $passedTests)" -ForegroundColor Red
    $rateColor = if ($percentPassed -ge 90) { 'Green' } elseif ($percentPassed -ge 70) { 'Yellow' } else { 'Red' }
    Write-Host "  Success Rate: $percentPassed%" -ForegroundColor $rateColor
    Write-Host ""

    if ($Script:Results.Overall) {
        Write-Host "✓ All verifications passed! WezTerm utilities are properly installed." -ForegroundColor Green
        return 0
    } else {
        Write-Host "✗ Some verifications failed. See recommendations above." -ForegroundColor Red
        Write-Host ""
        Write-Recommendation "Run: .\build-all.ps1 to rebuild and reinstall all components"
        return 1
    }
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

function Invoke-Verification {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║    WezTerm Utilities - Installation Verification             ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

    # Run all tests
    Test-PathConfiguration
    Test-BinaryInstallation
    Test-LuaModules
    Test-WeztermConfiguration
    Test-WeztermInstallation

    # Show summary
    $exitCode = Write-Summary

    Write-Host ""
    return $exitCode
}

# ============================================================================
# SCRIPT ENTRY POINT
# ============================================================================

$exitCode = Invoke-Verification
exit $exitCode
