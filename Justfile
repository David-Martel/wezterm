# Justfile: convenient developer commands

set shell := ["powershell", "-NoLogo", "-Command"]

default: build

build:
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace

release:
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace --release

fmt:
    cargo fmt --all

clippy:
    # Run clippy without sccache (wrapper causes -vV probe failure in current environment)
    Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue; cargo clippy --workspace --all-targets -- -D warnings -A clippy::type_complexity

clippy-cache:
    # Attempt clippy with sccache (may fail). Use for experimentation.
    $env:RUSTC_WRAPPER="sccache"; cargo clippy --workspace --all-targets -- -D warnings -A clippy::type_complexity

test:
    $env:RUSTC_WRAPPER="sccache"; cargo test --workspace --no-fail-fast

test-nextest:
    # Run tests with nextest if installed; fallback to cargo test
    if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) { $env:RUSTC_WRAPPER="sccache"; cargo nextest run --workspace } else { Write-Host 'cargo-nextest not installed; using cargo test'; $env:RUSTC_WRAPPER="sccache"; cargo test --workspace --no-fail-fast }

bench:
    cargo bench

lint: fmt clippy

check-docs:
    mdbook build docs

arch-docs:
    doxygen Doxyfile.rust

sccache-stats:
    sccache --show-stats

sccache-zero:
    sccache --zero-stats

lint-ast-grep:
    & sg scan wezterm-utils-daemon/src/ wezterm-module-framework/src/ wezterm-watch/src/ wezterm-fs-explorer/src/ wezterm-benchmarks/src/

lint-ast-grep-all:
    & sg scan

full-verify: fmt clippy lint-ast-grep test check-docs sccache-stats

full-local-ci: fmt clippy lint-ast-grep test-nextest check-docs arch-docs sccache-stats
    Write-Host "Full local CI complete"

quick-check:
    cargo check --workspace
    cargo fmt --all --check
    & sg scan wezterm-utils-daemon/src/ wezterm-module-framework/src/ wezterm-watch/src/
    Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue; cargo clippy --workspace -- -D warnings

# Build profiling and analysis
build-timings:
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace --timings

build-timings-release:
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace --release --timings

# Coverage reporting with llvm-cov
coverage:
    cargo llvm-cov nextest --workspace --html --output-dir target/coverage

coverage-open:
    cargo llvm-cov nextest --workspace --html --output-dir target/coverage --open

# Nextest archive caching for faster CI
test-archive:
    cargo nextest archive --workspace --archive-file target/nextest-archive.tar.zst

test-from-archive:
    cargo nextest run --archive-file target/nextest-archive.tar.zst

# Release automation with cargo-smart-release
release-dry-run:
    cargo smart-release --dry-run --bump minor wezterm-fs-explorer wezterm-watch

release-minor:
    cargo smart-release --execute --bump minor wezterm-fs-explorer wezterm-watch

release-patch:
    cargo smart-release --execute --bump patch wezterm-fs-explorer wezterm-watch

changelog:
    git cliff --unreleased --prepend CHANGELOG.md

# Tool installation with cargo-binstall
install-tools:
    cargo binstall cargo-nextest cargo-llvm-cov cargo-smart-release git-cliff -y

install-dev-tools:
    cargo binstall cargo-nextest cargo-llvm-cov cargo-smart-release git-cliff cargo-deny cargo-audit sccache -y

# Custom WezTerm utilities
build-utils:
    $env:RUSTC_WRAPPER="sccache"; cargo build --release -p wezterm-fs-explorer -p wezterm-watch

install-utils:
    powershell.exe -File .\build-all.ps1 -BuildProfile release

# ==============================================================================
# Enhanced Development Tools
# ==============================================================================

# Bootstrap all development tools (comprehensive installation)
bootstrap-tools:
    cargo binstall -y cargo-smart-release gix-cli cargo-nextest cargo-llvm-cov git-cliff sccache cargo-deny cargo-audit

# Check tool health and versions
check-tools:
    @Write-Host "Checking build tools..." -ForegroundColor Cyan
    @if (Get-Command sccache -ErrorAction SilentlyContinue) { Write-Host "`nsccache:" -ForegroundColor Green; sccache --show-stats } else { Write-Host "`nsccache: NOT FOUND" -ForegroundColor Yellow }
    @if (Get-Command gix -ErrorAction SilentlyContinue) { Write-Host "`ngix:" -ForegroundColor Green; gix --version } else { Write-Host "`ngix: NOT FOUND" -ForegroundColor Yellow }
    @if (Get-Command cargo-smart-release -ErrorAction SilentlyContinue) { Write-Host "`ncargo-smart-release:" -ForegroundColor Green; cargo smart-release --version } else { Write-Host "`ncargo-smart-release: NOT FOUND" -ForegroundColor Yellow }
    @if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) { Write-Host "`ncargo-nextest:" -ForegroundColor Green; cargo nextest --version } else { Write-Host "`ncargo-nextest: NOT FOUND" -ForegroundColor Yellow }
    @if (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue) { Write-Host "`ncargo-llvm-cov:" -ForegroundColor Green; cargo llvm-cov --version } else { Write-Host "`ncargo-llvm-cov: NOT FOUND" -ForegroundColor Yellow }
    @if (Get-Command git-cliff -ErrorAction SilentlyContinue) { Write-Host "`ngit-cliff:" -ForegroundColor Green; git-cliff --version } else { Write-Host "`ngit-cliff: NOT FOUND" -ForegroundColor Yellow }

# ==============================================================================
# Smart Release Workflow
# ==============================================================================

# Preview release for utilities (alias for release-dry-run)
release-preview:
    cargo smart-release --dry-run wezterm-fs-explorer wezterm-watch

# Execute patch release
release-execute:
    cargo smart-release --execute --bump patch wezterm-fs-explorer wezterm-watch

# Bump and release with automatic changelog generation
release-with-changelog:
    git cliff --unreleased --prepend CHANGELOG.md
    cargo smart-release --execute wezterm-fs-explorer wezterm-watch

# ==============================================================================
# gix Integration (Fast Git Operations)
# ==============================================================================

# Fast repository statistics using gix
repo-stats:
    @if (Get-Command gix -ErrorAction SilentlyContinue) { gix repo stats } else { Write-Host "gix not installed. Run 'just bootstrap-tools'" -ForegroundColor Yellow }

# Analyze commits since last tag
unreleased-commits:
    @if (Get-Command gix -ErrorAction SilentlyContinue) { $lastTag = git describe --tags --abbrev=0 2>$null; if ($lastTag) { gix log --oneline "$lastTag..HEAD" } else { gix log --oneline HEAD~10..HEAD } } else { Write-Host "gix not installed. Run 'just bootstrap-tools'" -ForegroundColor Yellow }

# Fast repository verification
repo-verify:
    @if (Get-Command gix -ErrorAction SilentlyContinue) { gix repo verify } else { Write-Host "gix not installed. Run 'just bootstrap-tools'" -ForegroundColor Yellow }

# ==============================================================================
# Enhanced Build Acceleration
# ==============================================================================

# Optimized parallel build using all CPU cores
build-parallel:
    $jobs = [Environment]::ProcessorCount; $env:RUSTC_WRAPPER="sccache"; cargo build --workspace -j $jobs

# Build with full diagnostics and timing logs
build-diag:
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace --timings 2>&1 | Tee-Object -FilePath build-timings.log
    Write-Host "`nBuild diagnostics saved to build-timings.log" -ForegroundColor Green
    Write-Host "Timing report: target/cargo-timings/cargo-timing.html" -ForegroundColor Green

# Clean and rebuild with cache statistics
rebuild-clean:
    sccache --zero-stats
    cargo clean
    $env:RUSTC_WRAPPER="sccache"; cargo build --workspace
    Write-Host "`n=== Cache Statistics ===" -ForegroundColor Cyan
    sccache --show-stats

# ==============================================================================
# Development Workflow Shortcuts
# ==============================================================================

# Full development cycle (check, test, coverage)
dev-cycle: quick-check test-nextest coverage

# Pre-commit validation (fast checks before committing)
pre-commit: fmt clippy test-nextest

# CI-like validation (comprehensive checks)
ci-validate: fmt clippy test-nextest coverage check-tools
    Write-Host "`n=== CI Validation Complete ===" -ForegroundColor Green
