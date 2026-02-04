# Build Integration Tools

Comprehensive build tools integration for WezTerm development combining CargoTools, cargo-smart-release, gix (gitoxide), and build acceleration tools.

## Overview

`Build-Integration.ps1` provides a unified interface for:

- **Bootstrap Installation**: Fast tool installation via cargo-binstall
- **Health Checks**: Comprehensive build environment validation
- **Release Automation**: Intelligent releases with cargo-smart-release
- **Git Operations**: Fast git operations using gix (gitoxide)
- **Build Acceleration**: sccache and lld-link optimization
- **Integration**: Export functions for use in other scripts

## Quick Start

### Install All Tools

```powershell
.\tools\Build-Integration.ps1 -Action install
```

### Check Build Environment Health

```powershell
.\tools\Build-Integration.ps1 -Action health-check
```

### Optimize Build Environment

```powershell
.\tools\Build-Integration.ps1 -Action optimize
```

### View Repository Statistics

```powershell
.\tools\Build-Integration.ps1 -Action stats
```

## Command Reference

### Installation

```powershell
# Install all development tools
.\tools\Build-Integration.ps1 -Action install

# Force reinstall even if tools exist
.\tools\Build-Integration.ps1 -Action install -Force

# Install from another script (dot-source)
. .\tools\Build-Integration.ps1
Install-RustBuildTools -Force
```

**Tools Installed:**
- `cargo-binstall` - Fast binary installer (required)
- `cargo-nextest` - Next-generation test runner
- `cargo-llvm-cov` - Code coverage with LLVM
- `cargo-smart-release` - Intelligent release automation
- `git-cliff` - Changelog generator
- `gix` - Gitoxide CLI for fast git operations
- `sccache` - Shared compilation cache
- `cargo-deny` - Dependency checks
- `cargo-audit` - Security vulnerability scanner

### Health Checks

```powershell
# Basic health check
.\tools\Build-Integration.ps1 -Action health-check

# Detailed health check (from script)
. .\tools\Build-Integration.ps1
Test-BuildToolHealth -Detailed
```

**Checks Performed:**
- Rust toolchain (rustc, cargo)
- Build acceleration (sccache, lld-link)
- Development tools (nextest, llvm-cov, smart-release, gix, git-cliff)
- CargoTools module availability
- Environment configuration (CARGO_HOME, SCCACHE_DIR, etc.)

### Release Workflow

```powershell
# Dry-run patch release
.\tools\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer" -BumpLevel patch -DryRun

# Execute minor release
.\tools\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer" -BumpLevel minor -Execute

# Release multiple packages
.\tools\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer,wezterm-watch" -BumpLevel patch -Execute

# From script with more control
. .\tools\Build-Integration.ps1
Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -DryRun
```

**Release Process:**
1. Validates repository state (clean working directory)
2. Analyzes dependencies and determines version bumps
3. Updates Cargo.toml versions
4. Generates changelog entries
5. Creates git tags
6. Optionally publishes to crates.io (use `-NoPublish` to skip)

### Git Operations (gix)

```powershell
# View repository statistics
.\tools\Build-Integration.ps1 -Action stats

# From script
. .\tools\Build-Integration.ps1
Get-RepoStats
Get-UnreleasedChanges
```

**gix Operations:**
- Fast repository statistics (objects, packs, references)
- Commit analysis since last tag
- Performance-optimized git operations

### Changelog Generation

```powershell
# Generate/update CHANGELOG.md
.\tools\Build-Integration.ps1 -Action changelog

# From script with options
. .\tools\Build-Integration.ps1
Update-ProjectChangelog -Unreleased -Prepend
```

### Build Optimization

```powershell
# Optimize build environment
.\tools\Build-Integration.ps1 -Action optimize

# From script with custom settings
. .\tools\Build-Integration.ps1
$env = Optimize-BuildEnvironment -JobCount 8

# Apply environment
$env.GetEnumerator() | ForEach-Object {
    if ($null -ne $_.Value) {
        Set-Item "Env:$($_.Key)" $_.Value
    }
}

# View sccache statistics
Get-SccacheStats
Get-SccacheStats -Zero  # Reset stats
```

**Optimization Features:**
- Starts sccache server with optimal configuration
- Configures parallel build jobs (N-1 cores)
- Enables lld-link if available
- Sets up incremental compilation
- Configures shared target directory

## Integration with Existing Scripts

### Integration with build-all.ps1

```powershell
# At the beginning of build-all.ps1
. "$PSScriptRoot\tools\Build-Integration.ps1"

# Optimize environment before builds
$optimizedEnv = Optimize-BuildEnvironment

# Apply environment
$optimizedEnv.GetEnumerator() | ForEach-Object {
    if ($null -ne $_.Value -and $_.Key -ne 'RUSTFLAGS') {
        Set-Item "Env:$($_.Key)" $_.Value
    }
}

# Build with optimization
if ($optimizedEnv.RUSTFLAGS) {
    $env:RUSTFLAGS = $optimizedEnv.RUSTFLAGS
}

# Continue with existing build logic...
```

### Integration with Justfile

Add targets that call the integration script:

```justfile
# Install development tools
install-tools:
    powershell.exe -File .\tools\Build-Integration.ps1 -Action install

# Health check
health-check:
    powershell.exe -File .\tools\Build-Integration.ps1 -Action health-check

# Release workflow
release-dry-run PACKAGE:
    powershell.exe -File .\tools\Build-Integration.ps1 -Action release -PackageName {{PACKAGE}} -BumpLevel patch -DryRun

release-patch PACKAGE:
    powershell.exe -File .\tools\Build-Integration.ps1 -Action release -PackageName {{PACKAGE}} -BumpLevel patch -Execute

release-minor PACKAGE:
    powershell.exe -File .\tools\Build-Integration.ps1 -Action release -PackageName {{PACKAGE}} -BumpLevel minor -Execute
```

### Using Functions in Custom Scripts

```powershell
# Import functions
. .\tools\Build-Integration.ps1

# Check if tools are installed
$health = Test-BuildToolHealth
if (-not $health.Overall) {
    Write-Host "Installing missing tools..." -ForegroundColor Yellow
    Install-RustBuildTools
}

# Optimize before build
$env = Optimize-BuildEnvironment
$env.GetEnumerator() | ForEach-Object {
    if ($null -ne $_.Value) {
        Set-Item "Env:$($_.Key)" $_.Value
    }
}

# Build with cargo
cargo build --release

# Show cache statistics
Get-SccacheStats
```

## Environment Configuration

### Default Paths

```powershell
CARGO_HOME          = $env:USERPROFILE\.cargo
SCCACHE_DIR         = T:\RustCache\sccache
CARGO_TARGET_DIR    = $env:USERPROFILE\.cargo\shared-target
SCCACHE_CACHE_SIZE  = 15G
SCCACHE_SERVER_PORT = 4226
```

### Override Defaults

Set environment variables before running:

```powershell
$env:SCCACHE_DIR = "D:\cache\sccache"
$env:CARGO_TARGET_DIR = "D:\cargo-target"
.\tools\Build-Integration.ps1 -Action optimize
```

## Advanced Usage

### Custom Tool Installation

```powershell
. .\tools\Build-Integration.ps1

# Install specific tools only
Install-RustBuildTools -ToolSubset "cargo-nextest,gix"

# Force reinstall
Install-RustBuildTools -Force
```

### Release with Custom Options

```powershell
. .\tools\Build-Integration.ps1

# Release without publishing
Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -Execute -NoPublish

# Allow dirty working directory (not recommended)
Invoke-SmartRelease -Package "wezterm-fs-explorer" -Bump patch -Execute -AllowDirty
```

### Detailed Health Diagnostics

```powershell
. .\tools\Build-Integration.ps1

$health = Test-BuildToolHealth -Detailed

# Check specific components
if ($health.BuildAcceleration['sccache'].Available) {
    Write-Host "sccache version: $($health.BuildAcceleration['sccache'].Version)"
    if ($health.BuildAcceleration['sccache'].ServerRunning) {
        Write-Host "Server is running"
    }
}

# Check dev tools
foreach ($tool in $health.DevTools.Keys) {
    $status = $health.DevTools[$tool]
    if ($status.Available) {
        Write-Host "$tool : $($status.Version)"
    }
}
```

### Build Environment Information

```powershell
. .\tools\Build-Integration.ps1

# Get current configuration
$config = Get-BuildToolEnvironment

# Display configuration
$config | Format-Table -AutoSize

# Use in build scripts
$targetDir = $config.CARGO_TARGET_DIR
Write-Host "Building to: $targetDir"
```

## Workflow Examples

### Complete Development Setup

```powershell
# 1. Install all tools
.\tools\Build-Integration.ps1 -Action install

# 2. Check health
.\tools\Build-Integration.ps1 -Action health-check

# 3. Import CargoTools
. .\tools\Build-Integration.ps1
Import-CargoTools

# 4. Ready to develop!
```

### Pre-Release Checklist

```powershell
# 1. Check repository stats
.\tools\Build-Integration.ps1 -Action stats

# 2. Update changelog
.\tools\Build-Integration.ps1 -Action changelog

# 3. Dry-run release
.\tools\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer" -BumpLevel patch -DryRun

# 4. Execute release
.\tools\Build-Integration.ps1 -Action release -PackageName "wezterm-fs-explorer" -BumpLevel patch -Execute
```

### Optimized Build Workflow

```powershell
# 1. Optimize environment
.\tools\Build-Integration.ps1 -Action optimize

# 2. Build (using Justfile)
just build

# 3. Check cache efficiency
. .\tools\Build-Integration.ps1
Get-SccacheStats
```

## Troubleshooting

### Tool Installation Fails

```powershell
# Verify cargo is working
cargo --version

# Install cargo-binstall manually
cargo install cargo-binstall

# Retry with verbose output
$VerbosePreference = 'Continue'
.\tools\Build-Integration.ps1 -Action install -Verbose
```

### sccache Not Starting

```powershell
# Check sccache manually
sccache --version

# Stop existing server
sccache --stop-server

# Start with optimization script
.\tools\Build-Integration.ps1 -Action optimize

# Check server status
sccache --show-stats
```

### Release Workflow Issues

```powershell
# Ensure working directory is clean
git status

# Check for uncommitted changes
git diff

# Ensure up-to-date with remote
git pull --rebase

# Try dry-run first
.\tools\Build-Integration.ps1 -Action release -PackageName "package-name" -BumpLevel patch -DryRun
```

### gix Not Found

```powershell
# Install gix specifically
cargo binstall gix -y

# Verify installation
gix --version

# Test repository access
gix repo stats
```

## Performance Tips

1. **Use sccache**: Reduces rebuild times by 50-90%
   ```powershell
   .\tools\Build-Integration.ps1 -Action optimize
   ```

2. **Enable lld-link**: Faster linking on Windows
   - Install LLVM: `winget install LLVM.LLVM`
   - Automatically detected by optimization script

3. **Use cargo-nextest**: 2-3x faster test execution
   ```powershell
   cargo nextest run
   ```

4. **Use gix for git operations**: 2-10x faster than git
   ```powershell
   gix repo stats  # vs git count-objects
   ```

5. **Shared target directory**: Reduces disk usage and compilation time
   - Configured in `.cargo/config.toml`
   - Set `CARGO_TARGET_DIR` environment variable

## Integration with CI/CD

### GitHub Actions

```yaml
- name: Install Build Tools
  run: |
    .\tools\Build-Integration.ps1 -Action install

- name: Optimize Build
  run: |
    .\tools\Build-Integration.ps1 -Action optimize

- name: Build
  run: |
    cargo build --release

- name: Cache Statistics
  run: |
    sccache --show-stats
```

### Local Pre-Commit Hook

```powershell
# .git/hooks/pre-commit.ps1
. .\tools\Build-Integration.ps1

$health = Test-BuildToolHealth
if (-not $health.Overall) {
    Write-Host "Build environment issues detected!" -ForegroundColor Red
    exit 1
}
```

## API Reference

### Exported Functions

When dot-sourced, the following functions are available:

| Function | Purpose |
|----------|---------|
| `Install-CargoBinstall` | Install cargo-binstall |
| `Install-RustBuildTools` | Install all development tools |
| `Test-BuildToolHealth` | Check build environment health |
| `Get-RepoStats` | Get repository statistics via gix |
| `Get-UnreleasedChanges` | Get commits since last tag |
| `Invoke-SmartRelease` | Execute release workflow |
| `Update-ProjectChangelog` | Generate/update CHANGELOG.md |
| `Optimize-BuildEnvironment` | Optimize build configuration |
| `Get-SccacheStats` | Get sccache statistics |
| `Get-BuildToolEnvironment` | Get environment configuration |
| `Import-CargoTools` | Import CargoTools module |
| `Write-BuildStatus` | Formatted status output |
| `Write-BuildSection` | Section header output |

## See Also

- [CargoTools Module](./CargoTools/README.md)
- [build-all.ps1](../build-all.ps1) - Main build script
- [Justfile](../Justfile) - Just task runner configuration
- [cargo-smart-release](https://github.com/Byron/gitoxide/tree/main/cargo-smart-release)
- [gix](https://github.com/Byron/gitoxide) - Gitoxide CLI
- [sccache](https://github.com/mozilla/sccache) - Compilation cache

## License

Part of the WezTerm project. See LICENSE.md for details.
