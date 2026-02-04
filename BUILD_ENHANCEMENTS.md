# Build-All.ps1 Enhancements

## Overview
Enhanced the master build script with cargo-binstall integration, release packaging, and changelog generation capabilities while preserving all existing functionality.

## New Features

### 1. Development Tools Installation

#### Install-CargoBinstall
Automatically installs `cargo-binstall` if not present, enabling fast binary installation of Rust development tools.

```powershell
function Install-CargoBinstall {
    # Checks for cargo-binstall and installs if missing
    # Returns $true if available, $false on failure
}
```

#### Install-DevTools
Installs essential Rust development tools using cargo-binstall:
- **cargo-nextest**: Fast parallel test runner
- **cargo-llvm-cov**: Code coverage tool
- **git-cliff**: Changelog generator from git history

```powershell
# Usage (call directly or integrate into build workflow)
Install-DevTools
```

### 2. Release Packaging

#### Get-ProjectVersion
Extracts version from multiple sources with fallback chain:
1. `-Version` parameter (if provided)
2. Workspace `Cargo.toml` version field
3. Git tags (`git describe --tags --abbrev=0`)
4. Fallback: `0.0.0-dev`

```powershell
$version = Get-ProjectVersion -OverrideVersion "1.2.3"
```

#### New-ReleasePackage
Creates versioned release packages for distribution:
- Packages each binary into versioned ZIP archives
- Naming convention: `{name}-{version}-x86_64-pc-windows-msvc.zip`
- Output directory: `artifacts/`
- Includes binary and README.md (if present)

```powershell
# Creates packages in artifacts/ directory
New-ReleasePackage -Version "1.0.0"
```

### 3. Changelog Generation

#### Update-Changelog
Generates or updates `CHANGELOG.md` using git-cliff:
- Extracts unreleased changes from git history
- Prepends to existing `CHANGELOG.md`
- Displays preview of changes

```powershell
# Standalone changelog update
.\build-all.ps1 -Changelog
```

## New Parameters

### -Release
Creates release artifacts in addition to normal installation.

```powershell
.\build-all.ps1 -Release -Version "1.0.0"
```

**Behavior**:
- Builds binaries using specified profile
- Installs to user PATH
- Creates versioned packages in `artifacts/`
- Runs verification tests (unless `-SkipTests`)

### -Package
Creates release packages without installing (CI/CD friendly).

```powershell
.\build-all.ps1 -Package
```

**Behavior**:
- Builds binaries using specified profile
- Skips installation steps
- Skips verification tests
- Creates versioned packages only

### -Version
Overrides version for release packages.

```powershell
.\build-all.ps1 -Release -Version "1.2.3"
```

If not specified, version is extracted from:
1. `Cargo.toml` workspace version
2. Git tags
3. Fallback: `0.0.0-dev`

### -Changelog
Standalone changelog generation (no build).

```powershell
.\build-all.ps1 -Changelog
```

**Behavior**:
- Runs git-cliff to update CHANGELOG.md
- Displays preview of changes
- Exits immediately (skips all build steps)

## Usage Examples

### Example 1: Standard Build and Install
```powershell
.\build-all.ps1
```
- Builds with `release` profile
- Installs to `~\.local\bin`
- Updates Lua modules and WezTerm config
- Runs verification tests

### Example 2: Fast Development Build
```powershell
.\build-all.ps1 -BuildProfile debug -SkipTests
```
- Builds with `debug` profile (faster compilation)
- Skips verification tests
- Installs to user PATH

### Example 3: Create Release Packages
```powershell
.\build-all.ps1 -Release -Version "1.0.0"
```
- Builds with `release` profile
- Installs to user PATH
- Creates versioned ZIP packages in `artifacts/`
- Example output:
  - `artifacts/wezterm-fs-explorer-1.0.0-x86_64-pc-windows-msvc.zip`
  - `artifacts/wezterm-watch-1.0.0-x86_64-pc-windows-msvc.zip`

### Example 4: CI/CD Package Creation
```powershell
.\build-all.ps1 -Package -BuildProfile release-fast
```
- Builds with `release-fast` profile
- Creates packages only (no installation)
- Skips verification tests
- Ideal for CI/CD pipelines

### Example 5: Update Changelog
```powershell
.\build-all.ps1 -Changelog
```
- Generates/updates `CHANGELOG.md` from git history
- No build or installation
- Displays preview of changes

### Example 6: Full Release Workflow
```powershell
# 1. Install development tools
Install-DevTools

# 2. Run tests with coverage
cargo nextest run
cargo llvm-cov --html

# 3. Update changelog
.\build-all.ps1 -Changelog

# 4. Create release packages
.\build-all.ps1 -Package -Version "1.0.0"

# 5. Verify packages
ls artifacts/
```

## Integration Points

### Existing Functionality Preserved
All original features remain intact:
- Parallel builds with optional sccache/lld
- Lua module installation
- WezTerm configuration updates
- PATH environment management
- Verification tests
- Rollback capability

### Build Acceleration
Works seamlessly with existing acceleration:
```powershell
.\build-all.ps1 -Sccache on -Lld on -Release
```

### Custom Installation Path
```powershell
.\build-all.ps1 -InstallPath "C:\tools\bin" -Release
```

## Technical Details

### File Structure
```
wezterm/
├── build-all.ps1              # Enhanced master script
├── test-build-enhancements.ps1 # Validation test suite
├── artifacts/                 # Created by -Release/-Package
│   ├── wezterm-fs-explorer-{version}-x86_64-pc-windows-msvc.zip
│   └── wezterm-watch-{version}-x86_64-pc-windows-msvc.zip
└── CHANGELOG.md              # Updated by -Changelog
```

### Function Organization
```
build-all.ps1
├── CONFIGURATION
├── LOGGING AND OUTPUT
├── VALIDATION
├── DEVELOPMENT TOOLS INSTALLATION  # NEW
│   ├── Install-CargoBinstall
│   └── Install-DevTools
├── BUILD FUNCTIONS
├── INSTALLATION FUNCTIONS
├── TESTING AND VERIFICATION
├── RELEASE AND PACKAGING          # NEW
│   ├── Get-ProjectVersion
│   ├── New-ReleasePackage
│   └── Update-Changelog
└── MAIN EXECUTION
```

### Error Handling
All new functions include:
- Try/catch blocks for robust error handling
- Detailed status messages with color coding
- Graceful degradation (tools missing → warning, not failure)
- Return values for success/failure tracking

## Validation

### Test Suite
Run the validation test suite:
```powershell
.\test-build-enhancements.ps1
```

**Tests performed**:
1. Script syntax validation
2. Help documentation completeness
3. Function definition verification
4. Parameter parsing validation
5. Configuration structure integrity

**Expected output**:
```
All enhancement tests passed successfully!

New features available:
  • cargo-binstall integration (Install-DevTools)
  • Release packaging (--Release, --Package)
  • Version management (--Version)
  • Changelog generation (--Changelog)
```

## Requirements

### Optional Tools
These tools are automatically installed by `Install-DevTools`:
- `cargo-binstall`: Fast binary installer
- `cargo-nextest`: Test runner
- `cargo-llvm-cov`: Coverage tool
- `git-cliff`: Changelog generator

### Manual Installation
```powershell
# Install cargo-binstall first
cargo install cargo-binstall

# Then install development tools
cargo binstall cargo-nextest cargo-llvm-cov git-cliff -y
```

Or use the script:
```powershell
# Source the script to access functions
. .\build-all.ps1
Install-DevTools
```

## Performance Impact

### Build Times
No impact on build times - new features are opt-in via parameters.

### Package Creation
Minimal overhead (~2-5 seconds) for ZIP compression of release binaries.

### Changelog Generation
Depends on repository size:
- Small repos (<1000 commits): <1 second
- Large repos (>10000 commits): 2-5 seconds

## Compatibility

### PowerShell Versions
- Minimum: PowerShell 5.1
- Tested: PowerShell 7+
- Platform: Windows (cross-platform Rust builds supported)

### Rust Toolchain
- Works with any Rust toolchain version
- Supports custom build profiles (release, release-fast, debug)

## Troubleshooting

### Issue: cargo-binstall installation fails
```powershell
# Manual installation
cargo install cargo-binstall --force
```

### Issue: Package creation fails
```powershell
# Ensure binaries are built first
.\build-all.ps1 -BuildProfile release
# Then package
.\build-all.ps1 -Package
```

### Issue: git-cliff not found for changelog
```powershell
# Install manually
cargo binstall git-cliff -y
# Or let the script install dev tools
Install-DevTools
```

### Issue: Version not detected
```powershell
# Explicitly specify version
.\build-all.ps1 -Package -Version "1.0.0"
```

## Future Enhancements

Potential additions:
- [ ] Cryptographic signing of release packages
- [ ] Automated GitHub release creation
- [ ] Multi-platform package generation (Linux, macOS)
- [ ] Delta package generation (incremental updates)
- [ ] Installer generation (MSI, Inno Setup)
- [ ] Docker container builds

## References

- **cargo-binstall**: https://github.com/cargo-bins/cargo-binstall
- **cargo-nextest**: https://nexte.st/
- **cargo-llvm-cov**: https://github.com/taiki-e/cargo-llvm-cov
- **git-cliff**: https://git-cliff.org/
