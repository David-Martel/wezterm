#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('fmt', 'clippy', 'clippy-all-features', 'test-changed', 'nextest', 'test-full', 'deny', 'mdbook', 'doxygen')]
    [string]$Task
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$strictPackages = @(
    '-p', 'wezterm-module-framework',
    '-p', 'wezterm-utils-daemon',
    '-p', 'wezterm-benchmarks',
    '-p', 'wezterm-watch'
)

function Invoke-CommandOrSkip {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandName,

        [Parameter(Mandatory = $true)]
        [scriptblock]$Action,

        [Parameter(Mandatory = $true)]
        [string]$SkipMessage
    )

    if (Get-Command $CommandName -ErrorAction SilentlyContinue) {
        & $Action
        return
    }

    Write-Host $SkipMessage
}

Push-Location $repoRoot
try {
    switch ($Task) {
        'fmt' {
            & cargo fmt --all
        }
        'clippy' {
            $env:RUSTC_WRAPPER = ''  # Override global cargo config sccache setting
            & cargo clippy @strictPackages --all-targets --no-deps -- -D warnings -A clippy::type_complexity
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
            & cargo clippy --manifest-path wezterm-fs-explorer/Cargo.toml --all-targets --no-deps -- -D warnings -A clippy::type_complexity
        }
        'clippy-all-features' {
            $env:RUSTC_WRAPPER = ''  # Override global cargo config sccache setting
            & cargo clippy @strictPackages --all-targets --all-features --no-deps -- -D warnings -A clippy::type_complexity
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
            & cargo clippy --manifest-path wezterm-fs-explorer/Cargo.toml --all-targets --all-features --no-deps -- -D warnings -A clippy::type_complexity
        }
        'test-changed' {
            $stagedRust = @(
                & git diff --cached --name-only --diff-filter=ACMR -- '*.rs' |
                    Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
            )
            if ($stagedRust.Count -eq 0) {
                Write-Host 'No staged Rust changes; skipping tests.'
                exit 0
            }

            $env:RUSTC_WRAPPER = 'sccache'
            if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) {
                & cargo nextest run --workspace --no-fail-fast
            } else {
                & cargo test --workspace --no-fail-fast --quiet
            }
        }
        'nextest' {
            $env:RUSTC_WRAPPER = 'sccache'
            if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) {
                & cargo nextest run --workspace --no-fail-fast
            } else {
                & cargo test --workspace --no-fail-fast
            }
        }
        'test-full' {
            $env:RUSTC_WRAPPER = 'sccache'
            if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) {
                & cargo nextest run --workspace --all-features --no-fail-fast
            } else {
                & cargo test --workspace --all-features --no-fail-fast
            }
        }
        'deny' {
            Invoke-CommandOrSkip -CommandName 'cargo-deny' -SkipMessage 'cargo-deny not installed; skipping.' -Action {
                & cargo deny check advisories licenses bans sources
            }
        }
        'mdbook' {
            Invoke-CommandOrSkip -CommandName 'mdbook' -SkipMessage 'mdbook not installed; skipping.' -Action {
                & mdbook build docs --quiet
            }
        }
        'doxygen' {
            Invoke-CommandOrSkip -CommandName 'doxygen' -SkipMessage 'doxygen not installed; skipping.' -Action {
                & doxygen Doxyfile.rust | Out-Null
            }
        }
    }

    exit $LASTEXITCODE
} finally {
    Pop-Location
}
