#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet('scan', 'fix-safe')]
    [string]$Mode = 'scan',

    [Parameter()]
    [switch]$Staged,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Paths
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
    }

    & sg @scanArgs @targetPaths
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
