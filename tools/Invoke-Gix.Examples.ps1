<#
.SYNOPSIS
    Example usage of Invoke-Gix.ps1 module functions.

.DESCRIPTION
    Demonstrates common workflows using the gix wrapper functions for
    repository analysis, release preparation, and performance benchmarking.
#>

# Import the module
Import-Module "$PSScriptRoot\Invoke-Gix.ps1" -Force -Verbose

Write-Host "`n=== Gix Module Examples ===" -ForegroundColor Cyan
Write-Host "WezTerm Repository Analysis and Release Tools`n" -ForegroundColor Gray

#region Example 1: Repository Statistics

Write-Host "Example 1: Repository Statistics" -ForegroundColor Yellow
Write-Host "Getting comprehensive repository stats...`n" -ForegroundColor Gray

$stats = Get-GixRepoStats
if ($stats) {
    $stats | Format-List
    Write-Host "Repository has $($stats.TotalCommits) commits across $($stats.TotalBranches) branches`n" -ForegroundColor Green
}

#endregion

#region Example 2: Unreleased Commits

Write-Host "Example 2: Unreleased Commits" -ForegroundColor Yellow
Write-Host "Getting commits since last release...`n" -ForegroundColor Gray

$unreleased = Get-GixUnreleasedCommits -Format Short
if ($unreleased) {
    Write-Host "Found $($unreleased.Count) unreleased commits:" -ForegroundColor Green
    $unreleased | Select-Object -First 10 | Format-Table -AutoSize
    if ($unreleased.Count -gt 10) {
        Write-Host "... and $($unreleased.Count - 10) more`n" -ForegroundColor Gray
    }
} else {
    Write-Host "No unreleased commits found.`n" -ForegroundColor Gray
}

#endregion

#region Example 3: Repository Health Check

Write-Host "Example 3: Repository Health Check" -ForegroundColor Yellow
Write-Host "Verifying repository integrity...`n" -ForegroundColor Gray

$health = Test-GixRepoHealth
if ($health) {
    if ($health.Healthy) {
        Write-Host "Repository is healthy!" -ForegroundColor Green
    } else {
        Write-Host "Repository has issues:" -ForegroundColor Red
        $health.Issues | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    }

    if ($health.Warnings) {
        Write-Host "Warnings:" -ForegroundColor Yellow
        $health.Warnings | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
    }
    Write-Host ""
}

#endregion

#region Example 4: Version Bump Recommendation

Write-Host "Example 4: Version Bump Recommendation" -ForegroundColor Yellow
Write-Host "Analyzing commits for semantic versioning...`n" -ForegroundColor Gray

$versionBump = Get-GixVersionBump
if ($versionBump) {
    $versionBump | Format-List

    $color = switch ($versionBump.RecommendedBump) {
        'major' { 'Red' }
        'minor' { 'Yellow' }
        'patch' { 'Green' }
        default { 'Gray' }
    }

    Write-Host "Recommendation: " -NoNewline
    Write-Host "$($versionBump.RecommendedBump.ToUpper()) version bump" -ForegroundColor $color
    Write-Host "Reason: $($versionBump.Reason)`n" -ForegroundColor Gray
}

#endregion

#region Example 5: Changelog Generation

Write-Host "Example 5: Changelog Generation" -ForegroundColor Yellow
Write-Host "Generating changelog from unreleased commits...`n" -ForegroundColor Gray

$changelog = Get-GixChangelog -GroupByType -IncludeBreaking
if ($changelog) {
    # Save to file
    $changelogPath = Join-Path $PSScriptRoot "CHANGELOG-PREVIEW.md"
    $changelog | Out-File $changelogPath -Encoding UTF8
    Write-Host "Changelog saved to: $changelogPath" -ForegroundColor Green

    # Display first few lines
    Write-Host "`nFirst 20 lines of changelog:" -ForegroundColor Gray
    ($changelog -split "`n" | Select-Object -First 20) -join "`n" | Write-Host
    Write-Host "`n... (see $changelogPath for full changelog)`n" -ForegroundColor Gray
}

#endregion

#region Example 6: Performance Measurement

Write-Host "Example 6: Performance Measurement" -ForegroundColor Yellow
Write-Host "Benchmarking repository stats operation...`n" -ForegroundColor Gray

$perfResult = Measure-GixOperation -Operation {
    Get-GixRepoStats | Out-Null
} -Name "Get-GixRepoStats" -Iterations 5 -Warmup 1

if ($perfResult) {
    $perfResult | Format-List
    Write-Host "Average execution time: $($perfResult.AverageMs)ms`n" -ForegroundColor Green
}

#endregion

#region Example 7: Git vs Gix Performance Comparison

Write-Host "Example 7: Git vs Gix Performance Comparison" -ForegroundColor Yellow
Write-Host "Comparing git and gix status command performance...`n" -ForegroundColor Gray

$comparison = Compare-GixPerformance -GitCommand "status" -GixCommand "status" -Iterations 3

if ($comparison) {
    Write-Host "Performance comparison completed successfully!`n" -ForegroundColor Green
}

#endregion

#region Example 8: Direct Gix Command

Write-Host "Example 8: Direct Gix Command" -ForegroundColor Yellow
Write-Host "Running raw gix command...`n" -ForegroundColor Gray

Write-Host "Output of 'gix --version':" -ForegroundColor Gray
Invoke-Gix --version
Write-Host ""

#endregion

#region Example 9: Workflow - Preparing a Release

Write-Host "Example 9: Complete Release Preparation Workflow" -ForegroundColor Yellow
Write-Host "Demonstrating full release preparation process...`n" -ForegroundColor Gray

Write-Host "Step 1: Check repository health" -ForegroundColor Cyan
$healthCheck = Test-GixRepoHealth
if (-not $healthCheck.Healthy) {
    Write-Host "  FAIL: Repository has integrity issues. Fix before releasing!" -ForegroundColor Red
} else {
    Write-Host "  PASS: Repository is healthy" -ForegroundColor Green
}

Write-Host "`nStep 2: Analyze unreleased commits" -ForegroundColor Cyan
$commits = Get-GixUnreleasedCommits
Write-Host "  Found $($commits.Count) unreleased commits" -ForegroundColor White

Write-Host "`nStep 3: Determine version bump" -ForegroundColor Cyan
$bump = Get-GixVersionBump
Write-Host "  Recommended: $($bump.RecommendedBump) version bump" -ForegroundColor White
Write-Host "  Reason: $($bump.Reason)" -ForegroundColor Gray

Write-Host "`nStep 4: Generate changelog" -ForegroundColor Cyan
$releaseChangelog = Get-GixChangelog -GroupByType -IncludeBreaking
$changelogFile = Join-Path $PSScriptRoot "RELEASE-NOTES.md"
$releaseChangelog | Out-File $changelogFile -Encoding UTF8
Write-Host "  Changelog generated: $changelogFile" -ForegroundColor White

Write-Host "`nStep 5: Review and prepare" -ForegroundColor Cyan
Write-Host "  Review the changelog at: $changelogFile" -ForegroundColor Gray
Write-Host "  Update version files with: $($bump.RecommendedBump) bump" -ForegroundColor Gray
Write-Host "  Create tag after version update and testing" -ForegroundColor Gray

Write-Host "`n=== Release preparation complete! ===`n" -ForegroundColor Green

#endregion

Write-Host "Examples completed. Check the generated files:" -ForegroundColor Cyan
Write-Host "  - $changelogPath" -ForegroundColor White
Write-Host "  - $changelogFile" -ForegroundColor White
Write-Host ""
