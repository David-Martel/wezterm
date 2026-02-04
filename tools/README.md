# WezTerm Development Tools

This directory contains PowerShell-based development tools for building, testing, and managing the WezTerm project.

## Overview

The tools are organized into modules that provide specialized functionality:

- **CargoTools**: Cross-platform Cargo wrapper with build optimization
- **Invoke-Gix**: Git operations using gitoxide (gix) for high performance
- **Build Acceleration**: sccache integration and parallel build support
- **Release Management**: Changelog generation and version bump analysis

## Quick Start

### Import Modules

```powershell
# Import CargoTools module
Import-Module .\tools\CargoTools\CargoTools.psd1

# Import Gix wrapper module
Import-Module .\tools\Invoke-Gix.ps1

# Verify modules loaded
Get-Module CargoTools, Invoke-Gix
```

### Common Workflows

**Build WezTerm with optimizations:**
```powershell
just build              # Standard build with sccache
just release            # Release build with optimizations
```

**Check repository status:**
```powershell
Get-GixRepoStats        # Comprehensive repo statistics
Test-GixRepoHealth      # Verify repository integrity
```

**Prepare a release:**
```powershell
Get-GixVersionBump      # Analyze commits for version bump
Get-GixChangelog -GroupByType -IncludeBreaking | Out-File CHANGELOG.md
```

## Modules

### CargoTools Module

Cross-platform Cargo wrapper that handles platform-specific build configurations, sccache integration, and rust-analyzer optimization.

**Location**: `tools/CargoTools/`

**Public Functions**:
- `Invoke-CargoWrapper` - Main Cargo wrapper with platform detection
- `Invoke-CargoRoute` - Routes Cargo commands to appropriate platform handler
- `Invoke-CargoWsl` - WSL-based builds
- `Invoke-CargoMacos` - macOS-specific builds
- `Invoke-CargoDocker` - Containerized builds
- `Invoke-RustAnalyzerWrapper` - Optimized rust-analyzer execution
- `Test-RustAnalyzerHealth` - Verify rust-analyzer configuration

**Helper Functions**:
- `Initialize-CargoEnv` - Set up Cargo environment variables
- `Start-SccacheServer` / `Stop-SccacheServer` - Manage sccache daemon
- `Get-SccacheMemoryMB` - Calculate optimal sccache memory allocation
- `Get-OptimalBuildJobs` - Determine parallel job count
- `Format-CargoOutput` - LLM-friendly build output formatting
- `ConvertTo-LlmContext` - Convert Cargo errors to LLM context
- `Get-RustProjectContext` - Extract project context for AI assistance

**Example Usage**:
```powershell
# Standard build with automatic platform detection
Invoke-CargoWrapper build --release

# Get build context for AI analysis
$context = Get-RustProjectContext
$context | ConvertTo-Json | Out-File build-context.json

# Check rust-analyzer health
Test-RustAnalyzerHealth
```

**Configuration**:
The module reads configuration from `.cargo/config.toml` and respects environment variables:
- `RUSTC_WRAPPER` - Set to `sccache` for build caching
- `SCCACHE_CACHE_SIZE` - Cache size limit (default: 15G)
- `SCCACHE_DIR` - Cache directory location
- `CARGO_BUILD_JOBS` - Parallel build job count

### Invoke-Gix Module

High-performance git operations using gitoxide (gix), a Rust-based git implementation that provides faster operations than traditional git commands.

**Location**: `tools/Invoke-Gix.ps1`

**Installation Requirement**:
```bash
cargo binstall gix-cli
# OR
cargo install gix-cli
```

**Core Functions**:
- `Invoke-Gix` - Direct wrapper for gix commands

**Repository Analysis**:
- `Get-GixRepoStats` - Comprehensive repository statistics
  - Total commits, branches, tags
  - Repository size and health metrics
  - Current branch and last commit info

- `Get-GixUnreleasedCommits` - Commits since last tag/release
  - Configurable output format (Short, Full, Oneline)
  - Parse conventional commits
  - Filter by date range

- `Test-GixRepoHealth` - Repository integrity verification
  - Object database integrity
  - Reference validity
  - Index consistency
  - Optional deep checks

**Release Preparation**:
- `Get-GixChangelog` - Generate changelog from commits
  - Group by conventional commit type (feat, fix, docs, etc.)
  - Highlight breaking changes
  - Markdown-formatted output

- `Get-GixVersionBump` - Semantic version bump recommendation
  - Analyzes commits for breaking changes, features, fixes
  - Suggests major/minor/patch bump
  - Follows conventional commit standards

**Performance Measurement**:
- `Measure-GixOperation` - Benchmark gix operations
  - Multiple iterations with warmup
  - Statistical analysis (avg, min, max, stddev)
  - Performance profiling

- `Compare-GixPerformance` - Git vs Gix comparison
  - Side-by-side benchmarking
  - Speedup factor calculation
  - Performance improvement percentage

**Example Usage**:
```powershell
# Get repository statistics
$stats = Get-GixRepoStats
Write-Host "Repository has $($stats.TotalCommits) commits"

# Generate changelog for release
Get-GixChangelog -GroupByType -IncludeBreaking | Out-File CHANGELOG.md

# Check what version bump is needed
$bump = Get-GixVersionBump
Write-Host "Recommended: $($bump.RecommendedBump) version bump"
Write-Host "Reason: $($bump.Reason)"

# Benchmark repository status check
Measure-GixOperation -Operation { Invoke-Gix status } -Name "Status" -Iterations 10

# Compare git vs gix performance
Compare-GixPerformance -GitCommand "status" -GixCommand "status" -Iterations 5
```

**Conventional Commit Support**:

The changelog and version bump functions recognize conventional commit prefixes:
- `feat:` - New feature (minor version bump)
- `fix:` - Bug fix (patch version bump)
- `docs:` - Documentation changes
- `perf:` - Performance improvements
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Build/tooling changes
- `BREAKING CHANGE` or `!` - Breaking change (major version bump)

**Complete Release Workflow Example**:
```powershell
# Import module
Import-Module .\tools\Invoke-Gix.ps1

# 1. Verify repository health
$health = Test-GixRepoHealth
if (-not $health.Healthy) {
    Write-Error "Repository has integrity issues!"
    exit 1
}

# 2. Analyze unreleased commits
$commits = Get-GixUnreleasedCommits
Write-Host "Found $($commits.Count) unreleased commits"

# 3. Determine version bump
$bump = Get-GixVersionBump
Write-Host "Recommended: $($bump.RecommendedBump) version bump"

# 4. Generate changelog
Get-GixChangelog -GroupByType -IncludeBreaking | Out-File CHANGELOG.md

# 5. Review and tag
# ... manual review of CHANGELOG.md ...
# git tag v1.2.3
# git push origin v1.2.3
```

## Example Scripts

### Invoke-Gix.Examples.ps1

Comprehensive examples demonstrating all Invoke-Gix module functions.

**Run examples**:
```powershell
.\tools\Invoke-Gix.Examples.ps1
```

**Includes**:
1. Repository statistics analysis
2. Unreleased commits listing
3. Repository health checks
4. Version bump recommendations
5. Changelog generation
6. Performance benchmarking
7. Git vs Gix comparisons
8. Direct gix command execution
9. Complete release preparation workflow

**Output Files**:
- `CHANGELOG-PREVIEW.md` - Generated changelog preview
- `RELEASE-NOTES.md` - Release notes for next version

## Performance Considerations

### Build Acceleration with sccache

The CargoTools module integrates sccache for shared compilation caching:

**Benefits**:
- Reduces rebuild times by caching compilation artifacts
- Shared cache across multiple projects
- Configurable cache size and location

**Configuration** (`.cargo/config.toml`):
```toml
[env]
SCCACHE_CACHE_SIZE = "15G"
SCCACHE_DIR = "${CARGO_HOME}/../sccache-cache"
RUSTC_WRAPPER = "sccache"
```

**Commands**:
```powershell
# Check cache statistics
just sccache-stats

# Reset statistics
just sccache-zero

# Build with sccache (automatic via Justfile)
just build
```

**Note**: Clippy requires sccache to be disabled due to probe issues. The Justfile automatically handles this:
```powershell
just clippy              # Automatically removes RUSTC_WRAPPER
```

### Gix Performance Benefits

Gitoxide (gix) provides significant performance improvements over standard git:

**Typical Speedups**:
- `git status`: 2-5x faster with gix
- `git log`: 3-10x faster with gix
- Large repository operations: Up to 15x faster

**Benchmark Example**:
```powershell
Compare-GixPerformance -GitCommand "log --oneline -n 1000" -GixCommand "log --oneline -n 1000"
# Typical result: 3-5x speedup
```

## Integration with WezTerm Build System

### Justfile Integration

The tools integrate with WezTerm's Justfile build system:

```makefile
# Windows builds (PowerShell-based)
just build              # Build with sccache
just release            # Release build
just clippy             # Lint (sccache disabled automatically)
just test               # Run tests with sccache
just full-verify        # Complete quality check

# Development helpers
just sccache-stats      # Show cache statistics
just fmt                # Format code
just full-local-ci      # Full local CI validation
```

### Pre-commit Integration

Tools support pre-commit hooks for quality checks:

```bash
# Install hooks
pre-commit install --hook-type pre-commit --hook-type pre-push

# Pre-commit (fast)
# - Format check
# - Clippy (changed crates only)
# - Quick tests
# - Dependency check

# Pre-push (comprehensive)
# - Full clippy with all features
# - All tests with all features
# - Documentation build
# - Architecture documentation
```

## Troubleshooting

### Gix Not Installed

**Error**: `gix not installed`

**Solution**:
```bash
cargo binstall gix-cli
# OR
cargo install gix-cli
```

**Verify**:
```powershell
gix --version
```

### sccache Issues

**Error**: `sccache server failed to start`

**Solution**:
```powershell
# Check if sccache is running
Get-Process sccache -ErrorAction SilentlyContinue

# Stop existing server
Stop-SccacheServer

# Start fresh
Start-SccacheServer

# Verify
sccache --show-stats
```

### Clippy with sccache

**Error**: `error: failed to get rustc version: -vV`

**Cause**: sccache wrapper interferes with clippy's rustc version probe

**Solution**: Use Justfile which automatically disables wrapper:
```powershell
just clippy             # Removes RUSTC_WRAPPER automatically
```

**Manual fix**:
```powershell
$env:RUSTC_WRAPPER = ""
cargo clippy --workspace --all-targets
```

### Module Import Issues

**Error**: `Module not found`

**Solution**:
```powershell
# Use absolute path
Import-Module C:\Users\david\wezterm\tools\Invoke-Gix.ps1 -Force

# Or navigate to directory first
cd C:\Users\david\wezterm
Import-Module .\tools\Invoke-Gix.ps1
```

**Verify**:
```powershell
Get-Module              # List loaded modules
Get-Command -Module Invoke-Gix  # List module commands
```

## Development

### Adding New Functions

**CargoTools Module**:
1. Create new function in `CargoTools/Public/` or `CargoTools/Private/`
2. Public functions are automatically exported
3. Update `CargoTools.psd1` version if needed
4. Add tests to verify functionality

**Invoke-Gix Module**:
1. Add function to appropriate region in `Invoke-Gix.ps1`
2. Include comment-based help documentation
3. Add to `Export-ModuleMember` list
4. Add usage example to `Invoke-Gix.Examples.ps1`

### Testing

**Manual Testing**:
```powershell
# Test individual functions
Import-Module .\tools\Invoke-Gix.ps1 -Force
Get-GixRepoStats
Test-GixRepoHealth -Verbose

# Run example script
.\tools\Invoke-Gix.Examples.ps1
```

**Integration Testing**:
```powershell
# Full build verification
just full-verify

# Full local CI
just full-local-ci
```

### Code Style

Follow PowerShell best practices:
- Use approved verbs (`Get-Verb` to list)
- Include comment-based help for all public functions
- Use `[CmdletBinding()]` for advanced function features
- Implement proper error handling with try/catch
- Use `Write-Verbose` for debug output
- Validate parameters with `[ValidateScript()]`, `[ValidateSet()]`, etc.
- Return strongly-typed objects (`[PSCustomObject]`)

## Additional Resources

### Documentation
- [gitoxide (gix) GitHub](https://github.com/Byron/gitoxide)
- [sccache Documentation](https://github.com/mozilla/sccache)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)

### WezTerm Build Documentation
- Project `CLAUDE.md` - Build commands and architecture
- `Justfile` - Build task definitions
- `.cargo/config.toml` - Cargo configuration

### PowerShell Resources
- [PowerShell Best Practices](https://docs.microsoft.com/en-us/powershell/scripting/developer/cmdlet/cmdlet-development-guidelines)
- [Comment-Based Help](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_comment_based_help)

## License

These tools are part of the WezTerm project and follow the same license as the main project.

## Contributing

Contributions are welcome! Please:
1. Follow existing code style and conventions
2. Add comment-based help to new functions
3. Update this README with new functionality
4. Test changes thoroughly before submitting
5. Add examples demonstrating new features
