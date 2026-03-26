#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet('scan', 'fix-safe')]
    [string]$Mode = 'scan',

    [Parameter()]
    [switch]$Staged,

    [Parameter()]
    [switch]$Changed,

    [Parameter()]
    [string[]]$Paths,

    [Parameter()]
    [ValidateSet('full', 'safe-gate')]
    [string]$Profile = 'full'
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$defaultPaths = @(
    'wezterm-utils-daemon/src',
    'wezterm-module-framework/src',
    'wezterm-watch/src',
    'wezterm-fs-explorer/src',
    'wezterm-benchmarks/src'
)

function Get-StagedRustPaths {
    $gitArgs = @('diff', '--cached', '--name-only', '--diff-filter=ACMR', '--', '*.rs')
    return @(
        & git @gitArgs |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_.Trim() } |
            Where-Object { Test-Path -LiteralPath (Join-Path $repoRoot $_) }
    )
}

function Get-ChangedRustPaths {
    $gitArgs = @('diff', '--name-only', '--diff-filter=ACMR', 'HEAD', '--', '*.rs')
    return @(
        & git @gitArgs |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_.Trim() } |
            Where-Object { Test-Path -LiteralPath (Join-Path $repoRoot $_) }
    )
}

Push-Location $repoRoot
try {
    if (-not (Get-Command sg -ErrorAction SilentlyContinue)) {
        throw 'ast-grep (`sg`) is required for this repository.'
    }

    $targetPaths = @()
    if ($Staged) {
        $targetPaths = Get-StagedRustPaths
        if ($targetPaths.Count -eq 0) {
            Write-Host 'No staged Rust files detected; skipping ast-grep.'
            exit 0
        }
    } elseif ($Changed) {
        $targetPaths = Get-ChangedRustPaths
        if ($targetPaths.Count -eq 0) {
            Write-Host 'No changed Rust files detected; skipping ast-grep.'
            exit 0
        }
    } elseif ($Paths -and $Paths.Count -gt 0) {
        $targetPaths = $Paths
    } else {
        $targetPaths = $defaultPaths
    }

    $scanArgs = @(
        'scan',
        '-c', 'sgconfig.yml',
        '--color', 'never',
        '--report-style', 'medium'
    )

    if ($Mode -eq 'fix-safe') {
        # Restrict auto-fix to local, syntax-preserving rewrites only.
        $scanArgs += @(
            '--update-all',
            '--filter', 'prefer-expect-over-allow|remove-redundant-format'
        )
    } elseif ($Profile -eq 'safe-gate') {
        # Build/CI gate profile: block only on rules we can enforce today without tripping the
        # broader unwrap/panic backlog that still needs targeted cleanup.
        $scanArgs += @(
            '--filter', 'prefer-expect-over-allow|remove-redundant-format|avoid-static-mut|dbg-macro-in-production',
            '--error=prefer-expect-over-allow',
            '--error=remove-redundant-format',
            '--error=avoid-static-mut',
            '--error=dbg-macro-in-production'
        )
    }

    & sg @scanArgs @targetPaths
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
