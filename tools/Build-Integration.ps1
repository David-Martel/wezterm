#Requires -Version 5.1

<#
.SYNOPSIS
    Comprehensive build tools integration for WezTerm development.

.DESCRIPTION
    Master integration script that combines CargoTools, cargo-smart-release, gix (gitoxide),
    and build acceleration tools (sccache, nextest, llvm-cov) into a unified development
    workflow.

    Key Features:
    - Bootstrap installation of all Rust development tools via cargo-binstall
    - Health checks for build environment and tool availability
    - Release workflow automation with cargo-smart-release
    - Fast git operations using gix (gitoxide CLI)
    - Build acceleration with sccache and lld-link
    - Integration with existing build-all.ps1 and Justfile
    - Export functions for dot-sourcing into other scripts

.PARAMETER Action
    Action to perform: install, health-check, release, stats, optimize, changelog

.PARAMETER PackageName
    Package name(s) for release operations (comma-separated for multiple packages)

.PARAMETER BumpLevel
    Version bump level for releases: patch, minor, major

.PARAMETER DryRun
    Perform dry-run without making actual changes (for release operations)

.PARAMETER Execute
    Execute release operations (required for actual releases)

.PARAMETER Force
    Force reinstall of tools even if already present

.EXAMPLE
    .\Build-Integration.ps1 -Action install
    Install all Rust development tools via cargo-binstall

.EXAMPLE
    .\Build-Integration.ps1 -Action health-check
    Check health of all build tools and environment

.EXAMPLE
    .\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer" -BumpLevel patch -DryRun
    Perform dry-run release of wezterm-fs-explorer with patch version bump

.EXAMPLE
    .\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer,wezterm-watch" -BumpLevel minor -Execute
    Execute release of both utilities with minor version bump

.EXAMPLE
    .\Build-Integration.ps1 -Action stats
    Display repository statistics using gix

.EXAMPLE
    .\Build-Integration.ps1 -Action optimize
    Optimize build environment and display configuration

.EXAMPLE
    .\Build-Integration.ps1 -Action changelog
    Generate/update CHANGELOG.md from git history

.NOTES
    This script can be dot-sourced to import functions into other scripts:
    . .\tools\Build-Integration.ps1

    Then call individual functions like:
    Install-RustBuildTools -Force
    Test-BuildToolHealth
    Optimize-BuildEnvironment
#>

[CmdletBinding(DefaultParameterSetName = 'General')]
param(
    [Parameter(ParameterSetName = 'General')]
    [ValidateSet('install', 'health-check', 'release', 'stats', 'optimize', 'changelog')]
    [string]$Action,

    [Parameter(ParameterSetName = 'Release', Mandatory = $true)]
    [Parameter(ParameterSetName = 'General')]
    [string]$PackageName,

    [Parameter(ParameterSetName = 'Release')]
    [Parameter(ParameterSetName = 'General')]
    [ValidateSet('patch', 'minor', 'major')]
    [string]$BumpLevel = 'patch',

    [Parameter(ParameterSetName = 'Release')]
    [Parameter(ParameterSetName = 'General')]
    [switch]$DryRun,

    [Parameter(ParameterSetName = 'Release')]
    [Parameter(ParameterSetName = 'General')]
    [switch]$Execute,

    [Parameter(ParameterSetName = 'General')]
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# ============================================================================
# MODULE CONFIGURATION
# ============================================================================

$Script:ModuleConfig = @{
    # Core Rust tools
    RustTools = @(
        @{ Name = 'cargo-binstall'; Description = 'Fast binary installer for Rust crates'; Required = $true }
        @{ Name = 'cargo-nextest'; Description = 'Next-generation test runner'; Required = $false }
        @{ Name = 'cargo-llvm-cov'; Description = 'Code coverage with LLVM'; Required = $false }
        @{ Name = 'cargo-smart-release'; Description = 'Intelligent release automation'; Required = $false }
        @{ Name = 'git-cliff'; Description = 'Changelog generator from git history'; Required = $false }
        @{ Name = 'gix'; Description = 'Gitoxide CLI - fast git operations'; Required = $false }
        @{ Name = 'sccache'; Description = 'Shared compilation cache'; Required = $false }
        @{ Name = 'cargo-deny'; Description = 'Cargo plugin for dependency checks'; Required = $false }
        @{ Name = 'cargo-audit'; Description = 'Security vulnerability scanner'; Required = $false }
    )

    # Paths (PowerShell 5.1 compatible - no ?? operator)
    CargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    SccacheDir = if ($env:SCCACHE_DIR) { $env:SCCACHE_DIR } else { 'T:\RustCache\sccache' }
    CargoTargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $env:USERPROFILE '.cargo\shared-target' }

    # Build optimization defaults
    SccacheCacheSize = '15G'
    SccachePort = 4226
    DefaultBuildJobs = $env:NUMBER_OF_PROCESSORS
}

# Colors for output
$Script:Colors = @{
    Success = 'Green'
    Error = 'Red'
    Warning = 'Yellow'
    Info = 'Cyan'
    Dim = 'DarkGray'
    Highlight = 'Magenta'
}

# ============================================================================
# UTILITY FUNCTIONS
# ============================================================================

function Write-BuildStatus {
    <#
    .SYNOPSIS
        Writes formatted status messages to console.

    .PARAMETER Message
        The message to display

    .PARAMETER Level
        Message level: Success, Error, Warning, Info, Dim, Highlight
    #>
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message,

        [Parameter()]
        [ValidateSet('Success', 'Error', 'Warning', 'Info', 'Dim', 'Highlight')]
        [string]$Level = 'Info'
    )

    $color = $Script:Colors[$Level]
    $prefix = switch ($Level) {
        'Success' { '[OK]  ' }
        'Error' { '[ERR] ' }
        'Warning' { '[WARN]' }
        'Info' { '[INFO]' }
        'Highlight' { '[>>]  ' }
        default { '      ' }
    }

    Write-Host "$prefix $Message" -ForegroundColor $color
}

function Write-BuildSection {
    <#
    .SYNOPSIS
        Writes a section header.

    .PARAMETER Title
        Section title text
    #>
    param([Parameter(Mandatory = $true)][string]$Title)

    Write-Host ""
    Write-Host "=================================================================" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "=================================================================" -ForegroundColor Cyan
}

function Test-CommandExists {
    <#
    .SYNOPSIS
        Tests if a command exists in PATH.

    .PARAMETER Name
        Command name to test

    .OUTPUTS
        System.Management.Automation.CommandInfo or $null
    #>
    param([Parameter(Mandatory = $true)][string]$Name)

    return Get-Command $Name -ErrorAction SilentlyContinue
}

# ============================================================================
# BOOTSTRAP INSTALLATION
# ============================================================================

function Install-CargoBinstall {
    <#
    .SYNOPSIS
        Installs cargo-binstall if not already present.

    .DESCRIPTION
        cargo-binstall enables fast binary installation of Rust crates without
        compilation, significantly speeding up development tool setup.

    .OUTPUTS
        System.Boolean - $true if installed or already present, $false on failure
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param()

    if (Test-CommandExists 'cargo-binstall') {
        Write-BuildStatus "cargo-binstall already installed" -Level Success
        return $true
    }

    Write-BuildStatus "Installing cargo-binstall..." -Level Info

    try {
        $output = cargo install cargo-binstall 2>&1 | Out-String

        if ($LASTEXITCODE -ne 0) {
            Write-BuildStatus "cargo-binstall installation failed" -Level Error
            Write-Verbose $output
            return $false
        }

        Write-BuildStatus "cargo-binstall installed successfully" -Level Success
        return $true

    } catch {
        Write-BuildStatus "Failed to install cargo-binstall: $_" -Level Error
        return $false
    }
}

function Install-RustBuildTools {
    <#
    .SYNOPSIS
        Installs comprehensive Rust development toolchain.

    .DESCRIPTION
        Installs all build and development tools for WezTerm development:
        - cargo-nextest: Fast parallel test runner
        - cargo-llvm-cov: Code coverage reporting
        - cargo-smart-release: Intelligent release automation
        - git-cliff: Changelog generation from git history
        - gix: Fast git operations via gitoxide
        - sccache: Shared compilation cache for build acceleration
        - cargo-deny: Dependency license and security checks
        - cargo-audit: Security vulnerability scanning

    .PARAMETER ToolSubset
        Install only specific tools (comma-separated names)

    .PARAMETER Force
        Force reinstall even if tools already exist

    .EXAMPLE
        Install-RustBuildTools
        Install all development tools

    .EXAMPLE
        Install-RustBuildTools -ToolSubset "cargo-nextest,gix" -Force
        Force reinstall specific tools

    .OUTPUTS
        System.Management.Automation.PSCustomObject with InstallResults
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [string]$ToolSubset,

        [Parameter()]
        [switch]$Force
    )

    Write-BuildSection "Installing Rust Build Tools"

    # Ensure cargo-binstall is available
    if (-not (Install-CargoBinstall)) {
        throw "Cannot install tools without cargo-binstall"
    }

    # Filter tools if subset specified
    $toolsToInstall = if ($ToolSubset) {
        $subsetNames = $ToolSubset -split ',' | ForEach-Object { $_.Trim() }
        $Script:ModuleConfig.RustTools | Where-Object { $subsetNames -contains $_.Name }
    } else {
        $Script:ModuleConfig.RustTools
    }

    $results = @{
        Installed = [System.Collections.Generic.List[string]]::new()
        Skipped = [System.Collections.Generic.List[string]]::new()
        Failed = [System.Collections.Generic.List[string]]::new()
    }

    foreach ($tool in $toolsToInstall) {
        # Skip cargo-binstall as it's already installed
        if ($tool.Name -eq 'cargo-binstall') {
            continue
        }

        Write-BuildStatus "Checking $($tool.Name)..." -Level Dim

        # Check if already installed
        $existing = Test-CommandExists $tool.Name

        if ($existing -and -not $Force) {
            Write-BuildStatus "$($tool.Name) already installed" -Level Success
            $results.Skipped.Add($tool.Name)
            continue
        }

        Write-BuildStatus "Installing $($tool.Name) - $($tool.Description)..." -Level Info

        try {
            $installArgs = @('binstall', $tool.Name, '-y')
            if ($Force) {
                $installArgs += '--force'
            }

            $output = & cargo $installArgs 2>&1 | Out-String

            if ($LASTEXITCODE -eq 0) {
                Write-BuildStatus "$($tool.Name) installed successfully" -Level Success
                $results.Installed.Add($tool.Name)
            } else {
                Write-BuildStatus "$($tool.Name) installation failed" -Level Error
                Write-Verbose $output
                $results.Failed.Add($tool.Name)
            }

        } catch {
            Write-BuildStatus "Failed to install $($tool.Name): $_" -Level Error
            $results.Failed.Add($tool.Name)
        }
    }

    # Summary
    Write-Host ""
    Write-BuildStatus "Installation Summary:" -Level Highlight
    Write-BuildStatus "  Installed: $($results.Installed.Count)" -Level Success
    Write-BuildStatus "  Skipped:   $($results.Skipped.Count)" -Level Info
    if ($results.Failed.Count -gt 0) {
        Write-BuildStatus "  Failed:    $($results.Failed.Count)" -Level Error
        $results.Failed | ForEach-Object {
            Write-BuildStatus "    - $_" -Level Error
        }
    }

    return [PSCustomObject]@{
        Installed = $results.Installed
        Skipped = $results.Skipped
        Failed = $results.Failed
        Success = ($results.Failed.Count -eq 0)
    }
}

# ============================================================================
# HEALTH CHECKS
# ============================================================================

function Test-BuildToolHealth {
    <#
    .SYNOPSIS
        Performs comprehensive health check of build environment.

    .DESCRIPTION
        Checks availability and status of:
        - Rust toolchain (rustc, cargo)
        - Build acceleration tools (sccache, lld-link)
        - Development tools (nextest, llvm-cov, smart-release, gix, git-cliff)
        - CargoTools module
        - Environment configuration

    .PARAMETER Detailed
        Include detailed version and path information

    .EXAMPLE
        Test-BuildToolHealth
        Basic health check

    .EXAMPLE
        Test-BuildToolHealth -Detailed
        Detailed health check with versions and paths

    .OUTPUTS
        System.Management.Automation.PSCustomObject with health check results
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [switch]$Detailed
    )

    Write-BuildSection "Build Tool Health Check"

    $healthStatus = @{
        Rust = @{}
        BuildAcceleration = @{}
        DevTools = @{}
        Environment = @{}
        CargoTools = @{}
        Overall = $true
    }

    # Check Rust toolchain
    Write-BuildStatus "Checking Rust toolchain..." -Level Info

    $rustc = Test-CommandExists 'rustc'
    if ($rustc) {
        $version = (rustc --version 2>&1) -replace 'rustc\s+', ''
        Write-BuildStatus "rustc: $version" -Level Success
        $healthStatus.Rust['rustc'] = @{ Available = $true; Version = $version; Path = $rustc.Source }
    } else {
        Write-BuildStatus "rustc not found" -Level Error
        $healthStatus.Rust['rustc'] = @{ Available = $false }
        $healthStatus.Overall = $false
    }

    $cargo = Test-CommandExists 'cargo'
    if ($cargo) {
        $version = (cargo --version 2>&1) -replace 'cargo\s+', ''
        Write-BuildStatus "cargo: $version" -Level Success
        $healthStatus.Rust['cargo'] = @{ Available = $true; Version = $version; Path = $cargo.Source }
    } else {
        Write-BuildStatus "cargo not found" -Level Error
        $healthStatus.Rust['cargo'] = @{ Available = $false }
        $healthStatus.Overall = $false
    }

    # Check build acceleration tools
    Write-Host ""
    Write-BuildStatus "Checking build acceleration..." -Level Info

    $sccache = Test-CommandExists 'sccache'
    if ($sccache) {
        try {
            $version = (sccache --version 2>&1) | Select-Object -First 1
            $stats = sccache --show-stats 2>&1 | Out-String

            # Check if server is running
            $serverRunning = $stats -match 'Cache location'

            Write-BuildStatus "sccache: $version $(if ($serverRunning) { '(server running)' } else { '(server stopped)' })" -Level Success
            $healthStatus.BuildAcceleration['sccache'] = @{
                Available = $true
                Version = $version
                Path = $sccache.Source
                ServerRunning = $serverRunning
                Stats = $stats
            }
        } catch {
            Write-BuildStatus "sccache found but not responding" -Level Warning
            $healthStatus.BuildAcceleration['sccache'] = @{ Available = $true; Healthy = $false }
        }
    } else {
        Write-BuildStatus "sccache not found (optional)" -Level Warning
        $healthStatus.BuildAcceleration['sccache'] = @{ Available = $false }
    }

    $lld = Test-CommandExists 'lld-link'
    if ($lld) {
        Write-BuildStatus "lld-link: $(Split-Path $lld.Source -Leaf)" -Level Success
        $healthStatus.BuildAcceleration['lld-link'] = @{ Available = $true; Path = $lld.Source }
    } else {
        Write-BuildStatus "lld-link not found (optional)" -Level Warning
        $healthStatus.BuildAcceleration['lld-link'] = @{ Available = $false }
    }

    # Check development tools
    Write-Host ""
    Write-BuildStatus "Checking development tools..." -Level Info

    $devToolNames = @('cargo-nextest', 'cargo-llvm-cov', 'cargo-smart-release', 'gix', 'git-cliff', 'cargo-deny', 'cargo-audit')

    foreach ($toolName in $devToolNames) {
        $tool = Test-CommandExists $toolName
        if ($tool) {
            try {
                $version = & $toolName --version 2>&1 | Select-Object -First 1
                Write-BuildStatus "${toolName}: $version" -Level Success
                $healthStatus.DevTools[$toolName] = @{ Available = $true; Version = $version; Path = $tool.Source }
            } catch {
                Write-BuildStatus "$toolName found but version check failed" -Level Warning
                $healthStatus.DevTools[$toolName] = @{ Available = $true; Healthy = $false }
            }
        } else {
            Write-BuildStatus "$toolName not found" -Level Warning
            $healthStatus.DevTools[$toolName] = @{ Available = $false }
        }
    }

    # Check CargoTools module
    Write-Host ""
    Write-BuildStatus "Checking CargoTools module..." -Level Info

    $cargoToolsPath = Join-Path $PSScriptRoot 'CargoTools\CargoTools.psd1'
    if (Test-Path $cargoToolsPath) {
        try {
            $manifest = Import-PowerShellDataFile $cargoToolsPath
            Write-BuildStatus "CargoTools v$($manifest.ModuleVersion) found" -Level Success
            $healthStatus.CargoTools['Available'] = $true
            $healthStatus.CargoTools['Version'] = $manifest.ModuleVersion
            $healthStatus.CargoTools['Path'] = $cargoToolsPath
        } catch {
            Write-BuildStatus "CargoTools manifest invalid" -Level Error
            $healthStatus.CargoTools['Available'] = $false
            $healthStatus.Overall = $false
        }
    } else {
        Write-BuildStatus "CargoTools module not found" -Level Warning
        $healthStatus.CargoTools['Available'] = $false
    }

    # Check environment configuration
    Write-Host ""
    Write-BuildStatus "Checking environment configuration..." -Level Info

    $envChecks = @{
        'CARGO_HOME' = $Script:ModuleConfig.CargoHome
        'SCCACHE_DIR' = $Script:ModuleConfig.SccacheDir
        'CARGO_TARGET_DIR' = $Script:ModuleConfig.CargoTargetDir
    }

    foreach ($varName in $envChecks.Keys) {
        $value = $envChecks[$varName]
        if ($value) {
            $exists = Test-Path $value -ErrorAction SilentlyContinue
            $status = if ($exists) { 'exists' } else { 'not created yet' }
            Write-BuildStatus "$varName = $value ($status)" -Level $(if ($exists) { 'Success' } else { 'Info' })
            $healthStatus.Environment[$varName] = @{ Value = $value; Exists = $exists }
        } else {
            Write-BuildStatus "$varName not set (using defaults)" -Level Info
            $healthStatus.Environment[$varName] = @{ Value = $null; Exists = $false }
        }
    }

    # Overall health summary
    Write-Host ""
    if ($healthStatus.Overall) {
        Write-BuildStatus "Overall health: HEALTHY" -Level Success
    } else {
        Write-BuildStatus "Overall health: ISSUES DETECTED" -Level Error
    }

    return [PSCustomObject]$healthStatus
}

# ============================================================================
# GIX INTEGRATION (GITOXIDE)
# ============================================================================

function Get-RepoStats {
    <#
    .SYNOPSIS
        Gets repository statistics using gix (gitoxide).

    .DESCRIPTION
        Uses gix for fast repository analysis including:
        - Object count and pack statistics
        - Reference counts
        - Worktree status

    .EXAMPLE
        Get-RepoStats
        Display repository statistics

    .OUTPUTS
        System.String - gix repo stats output
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param()

    if (-not (Test-CommandExists 'gix')) {
        throw "gix not installed. Install with: cargo binstall gix -y"
    }

    Write-BuildSection "Repository Statistics (gix)"

    try {
        $stats = gix repo stats 2>&1 | Out-String

        if ($LASTEXITCODE -eq 0) {
            Write-Host $stats
            return $stats
        } else {
            throw "gix repo stats failed"
        }

    } catch {
        Write-BuildStatus "Failed to get repository stats: $_" -Level Error
        return $null
    }
}

function Get-UnreleasedChanges {
    <#
    .SYNOPSIS
        Gets commits since last tag using gix.

    .DESCRIPTION
        Uses gix to efficiently analyze commits since the last git tag,
        providing information for release planning.

    .PARAMETER LastTag
        Override automatic last tag detection

    .EXAMPLE
        Get-UnreleasedChanges
        Show commits since last tag

    .OUTPUTS
        System.Management.Automation.PSCustomObject with unreleased commits
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [string]$LastTag
    )

    if (-not (Test-CommandExists 'gix')) {
        throw "gix not installed. Install with: cargo binstall gix -y"
    }

    Write-BuildSection "Unreleased Changes (gix)"

    try {
        # Get last tag if not provided
        if (-not $LastTag) {
            $LastTag = git describe --tags --abbrev=0 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-BuildStatus "No tags found in repository" -Level Warning
                $LastTag = "HEAD~10"
            }
        }

        Write-BuildStatus "Analyzing commits since: $LastTag" -Level Info

        # Use gix to count commits
        $commitList = git log "$LastTag..HEAD" --oneline 2>&1

        if ($LASTEXITCODE -eq 0) {
            $commits = $commitList | Where-Object { $_ -match '\S' }
            $commitCount = ($commits | Measure-Object).Count

            Write-BuildStatus "Found $commitCount commits since $LastTag" -Level Success
            Write-Host ""

            if ($commitCount -gt 0) {
                $commits | ForEach-Object {
                    Write-Host "  $_" -ForegroundColor DarkGray
                }
            }

            return [PSCustomObject]@{
                LastTag = $LastTag
                CommitCount = $commitCount
                Commits = $commits
            }

        } else {
            throw "Failed to get commit log"
        }

    } catch {
        Write-BuildStatus "Failed to get unreleased changes: $_" -Level Error
        return $null
    }
}

# ============================================================================
# RELEASE WORKFLOW
# ============================================================================

function Invoke-SmartRelease {
    <#
    .SYNOPSIS
        Executes intelligent release workflow using cargo-smart-release.

    .DESCRIPTION
        Automates version bumping, changelog generation, and release preparation:
        1. Validates repository state (clean working directory, up-to-date)
        2. Analyzes dependencies and determines version bumps
        3. Updates Cargo.toml versions
        4. Generates changelog entries
        5. Creates git tags
        6. Optionally publishes to crates.io

    .PARAMETER Package
        Package name(s) to release (comma-separated for multiple)

    .PARAMETER Bump
        Version bump level: patch, minor, major

    .PARAMETER DryRun
        Perform dry-run without making actual changes

    .PARAMETER Execute
        Execute release (required for actual releases)

    .PARAMETER AllowDirty
        Allow release with uncommitted changes (not recommended)

    .PARAMETER NoPublish
        Skip publishing to crates.io

    .EXAMPLE
        Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -DryRun
        Dry-run patch release of wezterm-fs-explorer

    .EXAMPLE
        Invoke-SmartRelease -Package "wezterm-fs-explorer,wezterm-watch" -Bump minor -Execute
        Execute minor release of both utilities

    .OUTPUTS
        System.Boolean - $true if successful, $false otherwise
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param(
        [Parameter(Mandatory = $true)]
        [string]$Package,

        [Parameter()]
        [ValidateSet('patch', 'minor', 'major')]
        [string]$Bump = 'patch',

        [Parameter()]
        [switch]$DryRun,

        [Parameter()]
        [switch]$Execute,

        [Parameter()]
        [switch]$AllowDirty,

        [Parameter()]
        [switch]$NoPublish
    )

    if (-not (Test-CommandExists 'cargo-smart-release')) {
        throw "cargo-smart-release not installed. Install with: cargo binstall cargo-smart-release -y"
    }

    Write-BuildSection "Smart Release - $Bump"

    # Validate that either DryRun or Execute is specified
    if (-not $DryRun -and -not $Execute) {
        Write-BuildStatus "Must specify either -DryRun or -Execute" -Level Error
        return $false
    }

    # Parse package names
    $packages = $Package -split ',' | ForEach-Object { $_.Trim() }

    Write-BuildStatus "Packages: $($packages -join ', ')" -Level Info
    Write-BuildStatus "Bump level: $Bump" -Level Info
    Write-BuildStatus "Mode: $(if ($DryRun) { 'DRY-RUN' } else { 'EXECUTE' })" -Level Highlight

    try {
        # Build cargo-smart-release arguments
        $releaseArgs = @('smart-release')

        if ($DryRun) {
            $releaseArgs += '--dry-run'
        }

        if ($Execute) {
            $releaseArgs += '--execute'
        }

        if ($AllowDirty) {
            $releaseArgs += '--allow-dirty'
        }

        if ($NoPublish) {
            $releaseArgs += '--no-publish'
        }

        $releaseArgs += '--bump', $Bump
        $releaseArgs += $packages

        Write-BuildStatus "Executing: cargo $($releaseArgs -join ' ')" -Level Info
        Write-Host ""

        # Execute cargo-smart-release
        $output = & cargo $releaseArgs 2>&1

        # Display output
        $output | ForEach-Object {
            $line = $_.ToString()
            if ($line -match 'error|fail') {
                Write-Host $line -ForegroundColor Red
            } elseif ($line -match 'warning') {
                Write-Host $line -ForegroundColor Yellow
            } elseif ($line -match 'success|complete|published') {
                Write-Host $line -ForegroundColor Green
            } else {
                Write-Host $line
            }
        }

        Write-Host ""

        if ($LASTEXITCODE -eq 0) {
            Write-BuildStatus "Release workflow completed successfully" -Level Success
            return $true
        } else {
            Write-BuildStatus "Release workflow failed with exit code $LASTEXITCODE" -Level Error
            return $false
        }

    } catch {
        Write-BuildStatus "Release workflow failed: $_" -Level Error
        return $false
    }
}

function Update-ProjectChangelog {
    <#
    .SYNOPSIS
        Generates or updates CHANGELOG.md using git-cliff.

    .DESCRIPTION
        Uses git-cliff to generate conventional changelog from git history.
        Can generate full changelog or prepend unreleased changes.

    .PARAMETER Unreleased
        Only include unreleased changes (since last tag)

    .PARAMETER Prepend
        Prepend to existing CHANGELOG.md instead of overwriting

    .PARAMETER Output
        Output file path (defaults to CHANGELOG.md)

    .EXAMPLE
        Update-ProjectChangelog -Unreleased -Prepend
        Prepend unreleased changes to CHANGELOG.md

    .EXAMPLE
        Update-ProjectChangelog
        Regenerate complete CHANGELOG.md

    .OUTPUTS
        System.Boolean - $true if successful
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param(
        [Parameter()]
        [switch]$Unreleased,

        [Parameter()]
        [switch]$Prepend,

        [Parameter()]
        [string]$Output = 'CHANGELOG.md'
    )

    if (-not (Test-CommandExists 'git-cliff')) {
        throw "git-cliff not installed. Install with: cargo binstall git-cliff -y"
    }

    Write-BuildSection "Updating Changelog"

    try {
        $cliffArgs = @('cliff')

        if ($Unreleased) {
            $cliffArgs += '--unreleased'
        }

        if ($Prepend) {
            $cliffArgs += '--prepend', $Output
        } else {
            $cliffArgs += '--output', $Output
        }

        Write-BuildStatus "Generating changelog..." -Level Info

        $result = & git $cliffArgs 2>&1

        if ($LASTEXITCODE -eq 0) {
            Write-BuildStatus "CHANGELOG.md updated successfully" -Level Success

            # Display preview
            if (Test-Path $Output) {
                Write-Host ""
                Write-BuildStatus "Preview (first 20 lines):" -Level Info
                Get-Content $Output -Head 20 | ForEach-Object {
                    Write-Host "  $_" -ForegroundColor DarkGray
                }
            }

            return $true

        } else {
            Write-BuildStatus "git-cliff failed" -Level Error
            $result | ForEach-Object { Write-Host $_ -ForegroundColor Red }
            return $false
        }

    } catch {
        Write-BuildStatus "Failed to update changelog: $_" -Level Error
        return $false
    }
}

# ============================================================================
# BUILD OPTIMIZATION
# ============================================================================

function Optimize-BuildEnvironment {
    <#
    .SYNOPSIS
        Optimizes build environment for maximum performance.

    .DESCRIPTION
        Configures build environment with optimal settings:
        - Ensures sccache server is running
        - Configures optimal parallel job count
        - Sets up linker optimizations (lld-link if available)
        - Configures incremental compilation
        - Returns hashtable with optimized environment variables

    .PARAMETER JobCount
        Override default parallel job count

    .PARAMETER NoSccache
        Disable sccache

    .PARAMETER NoLld
        Disable lld-link linker

    .EXAMPLE
        Optimize-BuildEnvironment
        Configure optimal build environment

    .EXAMPLE
        $env = Optimize-BuildEnvironment -JobCount 8
        Configure with 8 parallel jobs

    .OUTPUTS
        System.Collections.Hashtable - Optimized environment configuration
    #>
    [CmdletBinding()]
    [OutputType([hashtable])]
    param(
        [Parameter()]
        [ValidateRange(1, 128)]
        [int]$JobCount,

        [Parameter()]
        [switch]$NoSccache,

        [Parameter()]
        [switch]$NoLld
    )

    Write-BuildSection "Optimizing Build Environment"

    $optimizedEnv = @{
        RUSTC_WRAPPER = $null
        SCCACHE_DIR = $null
        SCCACHE_CACHE_SIZE = $null
        SCCACHE_SERVER_PORT = $null
        CARGO_BUILD_JOBS = $null
        CARGO_INCREMENTAL = '1'
        RUSTFLAGS = @()
    }

    # Configure sccache
    if (-not $NoSccache) {
        $sccache = Test-CommandExists 'sccache'
        if ($sccache) {
            Write-BuildStatus "Configuring sccache..." -Level Info

            $optimizedEnv.RUSTC_WRAPPER = $sccache.Source
            $optimizedEnv.SCCACHE_DIR = $Script:ModuleConfig.SccacheDir
            $optimizedEnv.SCCACHE_CACHE_SIZE = $Script:ModuleConfig.SccacheCacheSize
            $optimizedEnv.SCCACHE_SERVER_PORT = $Script:ModuleConfig.SccachePort

            # Ensure sccache directory exists
            if (-not (Test-Path $optimizedEnv.SCCACHE_DIR)) {
                New-Item -ItemType Directory -Path $optimizedEnv.SCCACHE_DIR -Force | Out-Null
                Write-BuildStatus "Created sccache directory: $($optimizedEnv.SCCACHE_DIR)" -Level Success
            }

            # Start sccache server
            try {
                sccache --start-server 2>&1 | Out-Null
                Write-BuildStatus "sccache server started" -Level Success

                # Show current stats
                $stats = sccache --show-stats 2>&1 | Out-String
                Write-Verbose $stats

            } catch {
                Write-BuildStatus "Could not start sccache server (may already be running)" -Level Warning
            }

        } else {
            Write-BuildStatus "sccache not found - skipping" -Level Warning
        }
    }

    # Configure lld-link linker
    if (-not $NoLld) {
        $lld = Test-CommandExists 'lld-link'
        if ($lld) {
            Write-BuildStatus "Configuring lld-link..." -Level Info
            $optimizedEnv.RUSTFLAGS += '-C linker=lld-link'
            Write-BuildStatus "lld-link enabled" -Level Success
        } else {
            Write-BuildStatus "lld-link not found - using default linker" -Level Warning
        }
    }

    # Configure optimal job count
    $jobs = if ($JobCount) {
        $JobCount
    } else {
        $cpuCount = [int]$Script:ModuleConfig.DefaultBuildJobs
        # Use N-1 cores to leave one for system
        [Math]::Max(1, $cpuCount - 1)
    }

    $optimizedEnv.CARGO_BUILD_JOBS = $jobs
    Write-BuildStatus "Build jobs: $jobs" -Level Success

    # Combine RUSTFLAGS
    if ($optimizedEnv.RUSTFLAGS.Count -gt 0) {
        $optimizedEnv.RUSTFLAGS = $optimizedEnv.RUSTFLAGS -join ' '
    } else {
        $optimizedEnv.RUSTFLAGS = $null
    }

    # Display configuration summary
    Write-Host ""
    Write-BuildStatus "Optimized Configuration:" -Level Highlight

    foreach ($key in $optimizedEnv.Keys) {
        $value = $optimizedEnv[$key]
        if ($null -ne $value) {
            Write-BuildStatus "  $key = $value" -Level Dim
        }
    }

    return $optimizedEnv
}

function Get-SccacheStats {
    <#
    .SYNOPSIS
        Retrieves sccache statistics.

    .DESCRIPTION
        Gets current sccache compilation cache statistics including
        hit rate, cache size, and compilation counts.

    .PARAMETER Zero
        Reset statistics after displaying

    .EXAMPLE
        Get-SccacheStats
        Display current sccache statistics

    .EXAMPLE
        Get-SccacheStats -Zero
        Display and reset statistics

    .OUTPUTS
        System.String - sccache statistics
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param(
        [Parameter()]
        [switch]$Zero
    )

    if (-not (Test-CommandExists 'sccache')) {
        Write-BuildStatus "sccache not installed" -Level Warning
        return $null
    }

    Write-BuildSection "sccache Statistics"

    try {
        $stats = sccache --show-stats 2>&1 | Out-String

        Write-Host $stats

        if ($Zero) {
            Write-BuildStatus "Resetting statistics..." -Level Info
            sccache --zero-stats 2>&1 | Out-Null
            Write-BuildStatus "Statistics reset" -Level Success
        }

        return $stats

    } catch {
        Write-BuildStatus "Failed to get sccache stats: $_" -Level Error
        return $null
    }
}

# ============================================================================
# INTEGRATION HELPERS
# ============================================================================

function Get-BuildToolEnvironment {
    <#
    .SYNOPSIS
        Gets current build tool environment configuration.

    .DESCRIPTION
        Returns hashtable with current environment settings for build tools.
        Useful for diagnostics and integration with other scripts.

    .EXAMPLE
        $env = Get-BuildToolEnvironment
        Get current environment configuration

    .OUTPUTS
        System.Collections.Hashtable - Environment configuration
    #>
    [CmdletBinding()]
    [OutputType([hashtable])]
    param()

    return @{
        CARGO_HOME = $Script:ModuleConfig.CargoHome
        SCCACHE_DIR = $Script:ModuleConfig.SccacheDir
        CARGO_TARGET_DIR = $Script:ModuleConfig.CargoTargetDir
        SCCACHE_CACHE_SIZE = $Script:ModuleConfig.SccacheCacheSize
        SCCACHE_PORT = $Script:ModuleConfig.SccachePort
        DEFAULT_BUILD_JOBS = $Script:ModuleConfig.DefaultBuildJobs
        RUST_TOOLS = $Script:ModuleConfig.RustTools
    }
}

function Import-CargoTools {
    <#
    .SYNOPSIS
        Imports CargoTools module if available.

    .DESCRIPTION
        Attempts to import CargoTools PowerShell module from tools directory.

    .EXAMPLE
        Import-CargoTools
        Import CargoTools module

    .OUTPUTS
        System.Boolean - $true if imported successfully
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param()

    $cargoToolsPath = Join-Path $PSScriptRoot 'CargoTools\CargoTools.psd1'

    if (Test-Path $cargoToolsPath) {
        try {
            Import-Module $cargoToolsPath -Force -ErrorAction Stop
            Write-BuildStatus "CargoTools module imported" -Level Success
            return $true
        } catch {
            Write-BuildStatus "Failed to import CargoTools: $_" -Level Error
            return $false
        }
    } else {
        Write-BuildStatus "CargoTools module not found at: $cargoToolsPath" -Level Warning
        return $false
    }
}

# ============================================================================
# MAIN EXECUTION (when run as script)
# ============================================================================

function Invoke-BuildIntegration {
    <#
    .SYNOPSIS
        Main entry point when script is executed directly.

    .DESCRIPTION
        Handles command-line execution based on -Action parameter.
    #>
    [CmdletBinding()]
    param()

    if (-not $Action) {
        Get-Help $PSCommandPath -Full
        return
    }

    try {
        switch ($Action) {
            'install' {
                $result = Install-RustBuildTools -Force:$Force
                exit $(if ($result.Success) { 0 } else { 1 })
            }

            'health-check' {
                $health = Test-BuildToolHealth -Detailed
                exit $(if ($health.Overall) { 0 } else { 1 })
            }

            'release' {
                if (-not $PackageName) {
                    Write-BuildStatus "Package name required for release action" -Level Error
                    exit 1
                }

                $success = Invoke-SmartRelease -Package $PackageName -Bump $BumpLevel -DryRun:$DryRun -Execute:$Execute
                exit $(if ($success) { 0 } else { 1 })
            }

            'stats' {
                Get-RepoStats
                Write-Host ""
                Get-UnreleasedChanges
                exit 0
            }

            'optimize' {
                $env = Optimize-BuildEnvironment
                Write-Host ""
                Get-SccacheStats
                exit 0
            }

            'changelog' {
                $success = Update-ProjectChangelog -Unreleased -Prepend
                exit $(if ($success) { 0 } else { 1 })
            }

            default {
                Write-BuildStatus "Unknown action: $Action" -Level Error
                exit 1
            }
        }

    } catch {
        Write-BuildStatus "Execution failed: $_" -Level Error
        Write-Host $_.ScriptStackTrace -ForegroundColor Red
        exit 1
    }
}

# ============================================================================
# SCRIPT ENTRY POINT
# ============================================================================

# Only execute main function if script is run directly (not dot-sourced)
if ($MyInvocation.InvocationName -ne '.') {
    Invoke-BuildIntegration
}
