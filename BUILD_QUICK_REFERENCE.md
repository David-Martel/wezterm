# Build-All.ps1 Quick Reference

## Common Commands

### Development
```powershell
# Standard build and install
.\build-all.ps1

# Fast debug build (skip tests)
.\build-all.ps1 -BuildProfile debug -SkipTests

# Launch the bundled GUI without the CLI attach-to-console path
.\build-all.ps1
wezterm-launch

# Build with acceleration
.\build-all.ps1 -Sccache on -Lld on
```

### Release Management
```powershell
# Create release packages
.\build-all.ps1 -Package -Version "1.0.0"

# Build, install, and package
.\build-all.ps1 -Release -Version "1.0.0"

# Update changelog
.\build-all.ps1 -Changelog
```

### Development Tools
```powershell
# Install dev tools (cargo-nextest, cargo-llvm-cov, git-cliff)
# Source script first to access functions:
. .\build-all.ps1
Install-DevTools
```

## Parameter Reference

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `-BuildProfile` | String | `release` | Build profile: `release`, `release-fast`, `debug` |
| `-Sccache` | String | `auto` | Build cache: `auto`, `on`, `off` |
| `-Lld` | String | `auto` | Fast linker: `auto`, `on`, `off` |
| `-SkipTests` | Switch | Off | Skip verification tests |
| `-InstallPath` | String | `~\bin` | Installation directory |
| `-Force` | Switch | Off | Force reinstall |
| `-Release` | Switch | Off | Create release packages + install |
| `-Package` | Switch | Off | Create packages only (no install) |
| `-Version` | String | Auto | Override release version |
| `-Changelog` | Switch | Off | Update CHANGELOG.md only |

## Workflow Examples

### Daily Development
```powershell
# Quick iteration cycle
.\build-all.ps1 -BuildProfile debug -SkipTests
```

### Pre-Release
```powershell
# 1. Update changelog
.\build-all.ps1 -Changelog

# 2. Create packages
.\build-all.ps1 -Package -Version "1.0.0"

# 3. Verify artifacts
ls artifacts/
```

### CI/CD Pipeline
```powershell
# Build and package (no installation)
.\build-all.ps1 -Package -BuildProfile release-fast
```

## Output Locations

| Item | Location |
|------|----------|
| Launchers | `~\bin\wezterm-launch.cmd`, `~\bin\wezterm-launch.ps1`, `~\bin\wezterm-cli.cmd` |
| Bundled app | `~\bin\wezterm-app\` (or custom `-InstallPath\wezterm-app\`) |
| Utility binaries | `~\bin\` (or custom `-InstallPath`) |
| Release packages | `artifacts/` |
| Lua modules | `~\.config\wezterm\` |
| WezTerm config | `~\.wezterm.lua` |
| Changelog | `CHANGELOG.md` |

## Validation

```powershell
# Test enhancements
.\test-build-enhancements.ps1

# Verify bundled launch path
.\install-verification.ps1 -Detailed

# View help
Get-Help .\build-all.ps1 -Full
```
