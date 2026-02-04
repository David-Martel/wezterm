<#
.SYNOPSIS
    PowerShell wrapper for gix (gitoxide CLI) providing fast git operations.

.DESCRIPTION
    This module wraps the gix command-line tool with PowerShell convenience functions
    for repository analysis, release preparation, and performance benchmarking.

    Gix is a high-performance Rust-based git implementation that offers faster operations
    than traditional git commands, especially for large repositories.

.NOTES
    Author: WezTerm Development Team
    Requires: gix-cli (install via: cargo binstall gix-cli)

.EXAMPLE
    Import-Module .\Invoke-Gix.ps1
    Get-GixRepoStats

.EXAMPLE
    Get-GixUnreleasedCommits -Since "20250101"

.LINK
    https://github.com/Byron/gitoxide
#>

#Requires -Version 5.1

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

#region Private Helper Functions

function Test-GixInstalled {
    <#
    .SYNOPSIS
        Checks if gix is installed and accessible.

    .OUTPUTS
        [bool] True if gix is installed, False otherwise.
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param()

    $gixPath = Get-Command gix -ErrorAction SilentlyContinue
    if (-not $gixPath) {
        Write-Warning "gix not installed. Install with: cargo binstall gix-cli"
        Write-Warning "Or build from source: cargo install gix-cli"
        return $false
    }

    Write-Verbose "Found gix at: $($gixPath.Source)"
    return $true
}

function Get-GixPath {
    <#
    .SYNOPSIS
        Gets the full path to the gix executable.

    .OUTPUTS
        [string] Path to gix executable or $null if not found.
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param()

    $gixCmd = Get-Command gix -ErrorAction SilentlyContinue
    if ($gixCmd) {
        return $gixCmd.Source
    }
    return $null
}

function ConvertFrom-GixOutput {
    <#
    .SYNOPSIS
        Parses gix command output into PowerShell objects.

    .PARAMETER Output
        Raw output from gix command.

    .PARAMETER Format
        Expected output format (Text, JSON, Table).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, ValueFromPipeline)]
        [string[]]$Output,

        [ValidateSet('Text', 'JSON', 'Table')]
        [string]$Format = 'Text'
    )

    process {
        switch ($Format) {
            'JSON' {
                try {
                    $Output -join "`n" | ConvertFrom-Json
                } catch {
                    Write-Warning "Failed to parse JSON output: $_"
                    $Output
                }
            }
            'Table' {
                # Parse table output into objects
                $lines = $Output | Where-Object { $_.Trim() -ne '' }
                if ($lines.Count -gt 1) {
                    $headers = $lines[0] -split '\s{2,}' | ForEach-Object { $_.Trim() }
                    $lines[1..($lines.Count - 1)] | ForEach-Object {
                        $values = $_ -split '\s{2,}' | ForEach-Object { $_.Trim() }
                        $obj = [ordered]@{}
                        for ($i = 0; $i -lt [Math]::Min($headers.Count, $values.Count); $i++) {
                            $obj[$headers[$i]] = $values[$i]
                        }
                        [PSCustomObject]$obj
                    }
                } else {
                    $Output
                }
            }
            default {
                $Output
            }
        }
    }
}

#endregion

#region Core Wrapper Functions

function Invoke-Gix {
    <#
    .SYNOPSIS
        Direct wrapper for gix command-line tool.

    .DESCRIPTION
        Executes gix commands with pass-through arguments. This is the base function
        that all other specialized functions build upon.

    .PARAMETER Arguments
        Arguments to pass to gix command.

    .OUTPUTS
        Command output from gix.

    .EXAMPLE
        Invoke-Gix status

    .EXAMPLE
        Invoke-Gix log --oneline -n 10

    .EXAMPLE
        Invoke-Gix commit -m "feat: add new feature"
    #>
    [CmdletBinding()]
    param(
        [Parameter(ValueFromRemainingArguments)]
        [string[]]$Arguments
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    $gixPath = Get-GixPath
    Write-Verbose "Executing: gix $($Arguments -join ' ')"

    try {
        & $gixPath @Arguments
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "gix exited with code $LASTEXITCODE"
        }
    } catch {
        Write-Error "Failed to execute gix: $_"
        return $null
    }
}

#endregion

#region Repository Analysis Functions

function Get-GixRepoStats {
    <#
    .SYNOPSIS
        Retrieves comprehensive repository statistics using gix.

    .DESCRIPTION
        Analyzes the current git repository and returns statistics including:
        - Total commits
        - Number of branches
        - Number of tags
        - Repository size
        - Object counts

    .PARAMETER Path
        Path to git repository. Defaults to current directory.

    .OUTPUTS
        [PSCustomObject] Repository statistics.

    .EXAMPLE
        Get-GixRepoStats

    .EXAMPLE
        Get-GixRepoStats -Path C:\Projects\MyRepo
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [ValidateScript({ Test-Path $_ })]
        [string]$Path = $PWD
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    Push-Location $Path
    try {
        Write-Verbose "Analyzing repository at: $Path"

        # Get commit count
        $commitCount = (git rev-list --count HEAD 2>$null)
        if (-not $commitCount) { $commitCount = 0 }

        # Get branch count
        $branchCount = (git branch -a 2>$null | Measure-Object).Count

        # Get tag count
        $tagCount = (git tag 2>$null | Measure-Object).Count

        # Get remote count
        $remoteCount = (git remote 2>$null | Measure-Object).Count

        # Get current branch
        $currentBranch = git rev-parse --abbrev-ref HEAD 2>$null

        # Get repository root
        $repoRoot = git rev-parse --show-toplevel 2>$null

        # Get last commit info
        $lastCommitHash = git rev-parse HEAD 2>$null
        $lastCommitDate = git log -1 --format=%ci HEAD 2>$null

        # Calculate repository size (approximate)
        $gitDir = Join-Path $repoRoot ".git"
        $repoSize = 0
        if (Test-Path $gitDir) {
            $repoSize = (Get-ChildItem $gitDir -Recurse -File -ErrorAction SilentlyContinue |
                Measure-Object -Property Length -Sum).Sum
        }

        [PSCustomObject]@{
            RepositoryPath = $repoRoot
            CurrentBranch  = $currentBranch
            TotalCommits   = [int]$commitCount
            TotalBranches  = $branchCount
            TotalTags      = $tagCount
            TotalRemotes   = $remoteCount
            LastCommitHash = $lastCommitHash
            LastCommitDate = $lastCommitDate
            RepositorySize = [math]::Round($repoSize / 1MB, 2)
            RepositorySizeUnit = 'MB'
        }

    } catch {
        Write-Error "Failed to get repository stats: $_"
        return $null
    } finally {
        Pop-Location
    }
}

function Get-GixUnreleasedCommits {
    <#
    .SYNOPSIS
        Gets commits since the last tag or specified commit.

    .DESCRIPTION
        Retrieves all commits that have been made since the last release tag
        or a specified commit reference. Useful for generating changelogs and
        planning releases.

    .PARAMETER Since
        Commit reference to start from. Defaults to latest tag.

    .PARAMETER Until
        Commit reference to end at. Defaults to HEAD.

    .PARAMETER Format
        Output format: Short, Full, or Oneline.

    .OUTPUTS
        [PSCustomObject[]] Array of commit objects.

    .EXAMPLE
        Get-GixUnreleasedCommits

    .EXAMPLE
        Get-GixUnreleasedCommits -Since "v1.0.0" -Format Oneline

    .EXAMPLE
        Get-GixUnreleasedCommits -Since "20250101" -Until "main"
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject[]])]
    param(
        [Parameter()]
        [string]$Since,

        [Parameter()]
        [string]$Until = 'HEAD',

        [ValidateSet('Short', 'Full', 'Oneline')]
        [string]$Format = 'Short'
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    try {
        # If no 'Since' specified, use latest tag
        if (-not $Since) {
            $latestTag = git describe --tags --abbrev=0 2>$null
            if ($latestTag) {
                $Since = $latestTag
                Write-Verbose "Using latest tag as starting point: $Since"
            } else {
                Write-Warning "No tags found in repository. Using all commits."
                $Since = ""
            }
        }

        # Build commit range
        $range = if ($Since) { "$Since..$Until" } else { $Until }
        Write-Verbose "Getting commits in range: $range"

        # Get commits
        $formatString = switch ($Format) {
            'Short'   { '%h|%s|%an|%ar' }
            'Full'    { '%H|%s|%b|%an|%ae|%ad|%ar' }
            'Oneline' { '%h %s' }
        }

        $commits = git log $range --format=$formatString 2>$null

        if (-not $commits) {
            Write-Verbose "No unreleased commits found."
            return @()
        }

        # Parse commits into objects
        $commits | ForEach-Object {
            if ($Format -eq 'Oneline') {
                $_
            } else {
                $parts = $_ -split '\|'
                if ($Format -eq 'Short') {
                    [PSCustomObject]@{
                        Hash       = $parts[0]
                        Subject    = $parts[1]
                        Author     = $parts[2]
                        AuthorDate = $parts[3]
                    }
                } else {
                    [PSCustomObject]@{
                        Hash       = $parts[0]
                        Subject    = $parts[1]
                        Body       = $parts[2]
                        Author     = $parts[3]
                        AuthorEmail = $parts[4]
                        Date       = $parts[5]
                        DateRelative = $parts[6]
                    }
                }
            }
        }

    } catch {
        Write-Error "Failed to get unreleased commits: $_"
        return $null
    }
}

function Test-GixRepoHealth {
    <#
    .SYNOPSIS
        Verifies repository integrity using gix.

    .DESCRIPTION
        Runs various checks to verify the health and integrity of the git repository:
        - Object database integrity
        - Reference validity
        - Index consistency
        - Working tree status

    .PARAMETER Path
        Path to git repository. Defaults to current directory.

    .PARAMETER Deep
        Perform deep integrity checks (slower but more thorough).

    .OUTPUTS
        [PSCustomObject] Health check results.

    .EXAMPLE
        Test-GixRepoHealth

    .EXAMPLE
        Test-GixRepoHealth -Deep
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [ValidateScript({ Test-Path $_ })]
        [string]$Path = $PWD,

        [Parameter()]
        [switch]$Deep
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    Push-Location $Path
    try {
        Write-Verbose "Checking repository health at: $Path"
        $issues = @()
        $warnings = @()

        # Check if it's a valid git repository
        $isRepo = git rev-parse --git-dir 2>$null
        if (-not $isRepo) {
            $issues += "Not a valid git repository"
            return [PSCustomObject]@{
                Healthy = $false
                Issues  = $issues
                Warnings = $warnings
            }
        }

        # Run git fsck
        Write-Verbose "Running git fsck..."
        $fsckOutput = git fsck --no-progress 2>&1
        if ($LASTEXITCODE -ne 0) {
            $issues += "Repository integrity check failed"
            $issues += $fsckOutput | Where-Object { $_ -match 'error' }
        }

        # Check for dangling objects
        $danglingObjects = $fsckOutput | Where-Object { $_ -match 'dangling' }
        if ($danglingObjects) {
            $warnings += "Found $($danglingObjects.Count) dangling objects"
        }

        # Check references
        Write-Verbose "Checking references..."
        $brokenRefs = git for-each-ref --format='%(refname)' 2>&1 | Where-Object { $_ -match 'error' }
        if ($brokenRefs) {
            $issues += "Broken references found"
            $issues += $brokenRefs
        }

        # Check index
        Write-Verbose "Checking index..."
        $indexCheck = git status 2>&1
        if ($LASTEXITCODE -ne 0) {
            $issues += "Index corruption detected"
        }

        # Deep checks
        if ($Deep) {
            Write-Verbose "Performing deep integrity checks..."

            # Check pack files
            $packCheck = git verify-pack -v .git/objects/pack/*.idx 2>&1
            if ($LASTEXITCODE -ne 0) {
                $issues += "Pack file integrity issues detected"
            }

            # Check all objects
            $objectCheck = git count-objects -v 2>&1
            Write-Verbose "Object count: $objectCheck"
        }

        [PSCustomObject]@{
            Healthy  = ($issues.Count -eq 0)
            Issues   = $issues
            Warnings = $warnings
            Timestamp = Get-Date
            DeepCheck = $Deep.IsPresent
        }

    } catch {
        Write-Error "Failed to check repository health: $_"
        return $null
    } finally {
        Pop-Location
    }
}

#endregion

#region Release Preparation Functions

function Get-GixChangelog {
    <#
    .SYNOPSIS
        Extracts changelog-worthy commits between two references.

    .DESCRIPTION
        Parses commits to generate a changelog following conventional commit format.
        Categorizes commits by type (feat, fix, docs, etc.) and formats them for
        release notes.

    .PARAMETER Since
        Starting commit reference. Defaults to latest tag.

    .PARAMETER Until
        Ending commit reference. Defaults to HEAD.

    .PARAMETER GroupByType
        Group commits by type (feat, fix, docs, etc.).

    .PARAMETER IncludeBreaking
        Highlight breaking changes.

    .OUTPUTS
        [string] Formatted changelog text.

    .EXAMPLE
        Get-GixChangelog

    .EXAMPLE
        Get-GixChangelog -Since "v1.0.0" -GroupByType

    .EXAMPLE
        Get-GixChangelog -IncludeBreaking | Out-File CHANGELOG.md
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param(
        [Parameter()]
        [string]$Since,

        [Parameter()]
        [string]$Until = 'HEAD',

        [Parameter()]
        [switch]$GroupByType,

        [Parameter()]
        [switch]$IncludeBreaking
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    try {
        # Get unreleased commits
        $commits = Get-GixUnreleasedCommits -Since $Since -Until $Until -Format Full

        if (-not $commits -or $commits.Count -eq 0) {
            Write-Verbose "No commits to include in changelog."
            return "No changes since $Since"
        }

        # Parse conventional commits
        $categorized = @{
            Features       = @()
            Fixes          = @()
            Documentation  = @()
            Performance    = @()
            Refactoring    = @()
            Tests          = @()
            Chores         = @()
            Breaking       = @()
            Other          = @()
        }

        foreach ($commit in $commits) {
            $subject = $commit.Subject
            $hash = $commit.Hash

            # Check for breaking change
            if ($subject -match 'BREAKING CHANGE|!' -or $commit.Body -match 'BREAKING CHANGE') {
                $categorized.Breaking += "- $subject ($hash)"
            }

            # Categorize by conventional commit type
            if ($subject -match '^feat(\(.*?\))?:') {
                $categorized.Features += "- $subject ($hash)"
            } elseif ($subject -match '^fix(\(.*?\))?:') {
                $categorized.Fixes += "- $subject ($hash)"
            } elseif ($subject -match '^docs(\(.*?\))?:') {
                $categorized.Documentation += "- $subject ($hash)"
            } elseif ($subject -match '^perf(\(.*?\))?:') {
                $categorized.Performance += "- $subject ($hash)"
            } elseif ($subject -match '^refactor(\(.*?\))?:') {
                $categorized.Refactoring += "- $subject ($hash)"
            } elseif ($subject -match '^test(\(.*?\))?:') {
                $categorized.Tests += "- $subject ($hash)"
            } elseif ($subject -match '^chore(\(.*?\))?:') {
                $categorized.Chores += "- $subject ($hash)"
            } else {
                $categorized.Other += "- $subject ($hash)"
            }
        }

        # Build changelog
        $changelog = @()
        $changelog += "# Changelog"
        $changelog += ""
        $changelog += "Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
        $changelog += "Range: $Since..$Until"
        $changelog += ""

        if ($IncludeBreaking -and $categorized.Breaking.Count -gt 0) {
            $changelog += "## BREAKING CHANGES"
            $changelog += ""
            $changelog += $categorized.Breaking
            $changelog += ""
        }

        if ($GroupByType) {
            $sections = @(
                @{Name = 'Features'; Items = $categorized.Features},
                @{Name = 'Bug Fixes'; Items = $categorized.Fixes},
                @{Name = 'Performance Improvements'; Items = $categorized.Performance},
                @{Name = 'Documentation'; Items = $categorized.Documentation},
                @{Name = 'Code Refactoring'; Items = $categorized.Refactoring},
                @{Name = 'Tests'; Items = $categorized.Tests},
                @{Name = 'Chores'; Items = $categorized.Chores},
                @{Name = 'Other Changes'; Items = $categorized.Other}
            )

            foreach ($section in $sections) {
                if ($section.Items.Count -gt 0) {
                    $changelog += "## $($section.Name)"
                    $changelog += ""
                    $changelog += $section.Items
                    $changelog += ""
                }
            }
        } else {
            $changelog += "## All Changes"
            $changelog += ""
            foreach ($commit in $commits) {
                $changelog += "- $($commit.Subject) ($($commit.Hash))"
            }
            $changelog += ""
        }

        $changelog += "---"
        $changelog += "Total commits: $($commits.Count)"

        return ($changelog -join "`n")

    } catch {
        Write-Error "Failed to generate changelog: $_"
        return $null
    }
}

function Get-GixVersionBump {
    <#
    .SYNOPSIS
        Analyzes commits to suggest semantic version bump.

    .DESCRIPTION
        Examines unreleased commits following conventional commit format to determine
        the appropriate semantic version increment:
        - Major: Breaking changes
        - Minor: New features
        - Patch: Bug fixes only

    .PARAMETER Since
        Starting commit reference. Defaults to latest tag.

    .PARAMETER Until
        Ending commit reference. Defaults to HEAD.

    .OUTPUTS
        [PSCustomObject] Version bump recommendation.

    .EXAMPLE
        Get-GixVersionBump

    .EXAMPLE
        $bump = Get-GixVersionBump -Since "v1.2.3"
        Write-Host "Recommended bump: $($bump.RecommendedBump)"
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter()]
        [string]$Since,

        [Parameter()]
        [string]$Until = 'HEAD'
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    try {
        # Get unreleased commits
        $commits = Get-GixUnreleasedCommits -Since $Since -Until $Until -Format Full

        if (-not $commits -or $commits.Count -eq 0) {
            Write-Verbose "No commits to analyze for version bump."
            return [PSCustomObject]@{
                RecommendedBump = 'none'
                Reason          = 'No unreleased commits found'
                BreakingChanges = 0
                Features        = 0
                Fixes           = 0
                Other           = 0
            }
        }

        # Analyze commit types
        $breakingCount = 0
        $featureCount = 0
        $fixCount = 0
        $otherCount = 0

        foreach ($commit in $commits) {
            $subject = $commit.Subject
            $body = $commit.Body

            # Check for breaking changes
            if ($subject -match 'BREAKING CHANGE|!' -or $body -match 'BREAKING CHANGE') {
                $breakingCount++
            }

            # Check commit type
            if ($subject -match '^feat(\(.*?\))?:') {
                $featureCount++
            } elseif ($subject -match '^fix(\(.*?\))?:') {
                $fixCount++
            } else {
                $otherCount++
            }
        }

        # Determine version bump
        $recommendedBump = 'patch'
        $reason = 'Bug fixes only'

        if ($breakingCount -gt 0) {
            $recommendedBump = 'major'
            $reason = "Breaking changes detected ($breakingCount)"
        } elseif ($featureCount -gt 0) {
            $recommendedBump = 'minor'
            $reason = "New features added ($featureCount)"
        } elseif ($fixCount -gt 0) {
            $recommendedBump = 'patch'
            $reason = "Bug fixes ($fixCount)"
        } elseif ($otherCount -gt 0) {
            $recommendedBump = 'patch'
            $reason = "Other changes ($otherCount)"
        }

        [PSCustomObject]@{
            RecommendedBump = $recommendedBump
            Reason          = $reason
            BreakingChanges = $breakingCount
            Features        = $featureCount
            Fixes           = $fixCount
            Other           = $otherCount
            TotalCommits    = $commits.Count
            CurrentVersion  = $Since
        }

    } catch {
        Write-Error "Failed to analyze version bump: $_"
        return $null
    }
}

#endregion

#region Performance Functions

function Measure-GixOperation {
    <#
    .SYNOPSIS
        Measures the execution time of gix operations.

    .DESCRIPTION
        Benchmarks gix commands by executing them multiple times and calculating
        average execution time. Useful for performance comparisons and optimization.

    .PARAMETER Operation
        ScriptBlock containing the gix operation to measure.

    .PARAMETER Name
        Descriptive name for the operation being measured.

    .PARAMETER Iterations
        Number of times to execute the operation. Defaults to 1.

    .PARAMETER Warmup
        Number of warmup iterations before measurement. Defaults to 0.

    .OUTPUTS
        [PSCustomObject] Performance measurement results.

    .EXAMPLE
        Measure-GixOperation -Operation { Invoke-Gix status } -Name "Status Check"

    .EXAMPLE
        $result = Measure-GixOperation -Operation {
            Get-GixRepoStats
        } -Name "Repo Stats" -Iterations 10 -Warmup 2
        Write-Host "Average time: $($result.AverageMs)ms"
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter(Mandatory)]
        [scriptblock]$Operation,

        [Parameter(Mandatory)]
        [string]$Name,

        [Parameter()]
        [ValidateRange(1, 1000)]
        [int]$Iterations = 1,

        [Parameter()]
        [ValidateRange(0, 100)]
        [int]$Warmup = 0
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    try {
        Write-Verbose "Measuring operation: $Name"
        $timings = @()

        # Warmup iterations
        if ($Warmup -gt 0) {
            Write-Verbose "Performing $Warmup warmup iterations..."
            for ($i = 0; $i -lt $Warmup; $i++) {
                & $Operation | Out-Null
            }
        }

        # Measurement iterations
        Write-Verbose "Performing $Iterations measurement iterations..."
        for ($i = 0; $i -lt $Iterations; $i++) {
            $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
            & $Operation | Out-Null
            $stopwatch.Stop()
            $timings += $stopwatch.Elapsed.TotalMilliseconds

            Write-Progress -Activity "Measuring $Name" -Status "Iteration $($i + 1)/$Iterations" -PercentComplete (($i + 1) / $Iterations * 100)
        }
        Write-Progress -Activity "Measuring $Name" -Completed

        # Calculate statistics
        $avgTime = ($timings | Measure-Object -Average).Average
        $minTime = ($timings | Measure-Object -Minimum).Minimum
        $maxTime = ($timings | Measure-Object -Maximum).Maximum
        $stdDev = if ($timings.Count -gt 1) {
            $mean = $avgTime
            $variance = ($timings | ForEach-Object { [Math]::Pow($_ - $mean, 2) } | Measure-Object -Average).Average
            [Math]::Sqrt($variance)
        } else {
            0
        }

        [PSCustomObject]@{
            OperationName    = $Name
            Iterations       = $Iterations
            WarmupIterations = $Warmup
            AverageMs        = [Math]::Round($avgTime, 2)
            MinimumMs        = [Math]::Round($minTime, 2)
            MaximumMs        = [Math]::Round($maxTime, 2)
            StdDeviationMs   = [Math]::Round($stdDev, 2)
            TotalTimeMs      = [Math]::Round(($timings | Measure-Object -Sum).Sum, 2)
            Timestamp        = Get-Date
        }

    } catch {
        Write-Error "Failed to measure operation: $_"
        return $null
    }
}

function Compare-GixPerformance {
    <#
    .SYNOPSIS
        Compares performance between gix and standard git commands.

    .DESCRIPTION
        Runs the same operation using both gix and git, measuring execution time
        to demonstrate the performance benefits of gix.

    .PARAMETER GitCommand
        Standard git command to execute.

    .PARAMETER GixCommand
        Equivalent gix command to execute.

    .PARAMETER Iterations
        Number of iterations for each command. Defaults to 5.

    .OUTPUTS
        [PSCustomObject] Performance comparison results.

    .EXAMPLE
        Compare-GixPerformance -GitCommand "status" -GixCommand "status"

    .EXAMPLE
        Compare-GixPerformance -GitCommand "log --oneline -n 100" -GixCommand "log --oneline -n 100" -Iterations 10
    #>
    [CmdletBinding()]
    [OutputType([PSCustomObject])]
    param(
        [Parameter(Mandatory)]
        [string]$GitCommand,

        [Parameter(Mandatory)]
        [string]$GixCommand,

        [Parameter()]
        [ValidateRange(1, 100)]
        [int]$Iterations = 5
    )

    if (-not (Test-GixInstalled)) {
        return $null
    }

    try {
        Write-Host "Comparing performance: git vs gix" -ForegroundColor Cyan
        Write-Host "Command: $GitCommand" -ForegroundColor Gray
        Write-Host "Iterations: $Iterations" -ForegroundColor Gray
        Write-Host ""

        # Measure git command
        $gitTimings = @()
        Write-Host "Measuring git performance..." -ForegroundColor Yellow
        for ($i = 0; $i -lt $Iterations; $i++) {
            $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
            Invoke-Expression "git $GitCommand" | Out-Null
            $stopwatch.Stop()
            $gitTimings += $stopwatch.Elapsed.TotalMilliseconds
            Write-Progress -Activity "Measuring git" -Status "Iteration $($i + 1)/$Iterations" -PercentComplete (($i + 1) / $Iterations * 100)
        }
        Write-Progress -Activity "Measuring git" -Completed

        # Measure gix command
        $gixTimings = @()
        Write-Host "Measuring gix performance..." -ForegroundColor Yellow
        for ($i = 0; $i -lt $Iterations; $i++) {
            $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
            Invoke-Expression "gix $GixCommand" | Out-Null
            $stopwatch.Stop()
            $gixTimings += $stopwatch.Elapsed.TotalMilliseconds
            Write-Progress -Activity "Measuring gix" -Status "Iteration $($i + 1)/$Iterations" -PercentComplete (($i + 1) / $Iterations * 100)
        }
        Write-Progress -Activity "Measuring gix" -Completed

        # Calculate statistics
        $gitAvg = ($gitTimings | Measure-Object -Average).Average
        $gixAvg = ($gixTimings | Measure-Object -Average).Average
        $speedup = if ($gixAvg -gt 0) { $gitAvg / $gixAvg } else { 0 }
        $improvement = if ($gitAvg -gt 0) { (($gitAvg - $gixAvg) / $gitAvg) * 100 } else { 0 }

        $result = [PSCustomObject]@{
            Command        = $GitCommand
            Iterations     = $Iterations
            GitAverageMs   = [Math]::Round($gitAvg, 2)
            GixAverageMs   = [Math]::Round($gixAvg, 2)
            SpeedupFactor  = [Math]::Round($speedup, 2)
            ImprovementPct = [Math]::Round($improvement, 2)
            Winner         = if ($gixAvg -lt $gitAvg) { 'gix' } elseif ($gitAvg -lt $gixAvg) { 'git' } else { 'tie' }
        }

        # Display results
        Write-Host ""
        Write-Host "Results:" -ForegroundColor Green
        Write-Host "  git average: $($result.GitAverageMs)ms" -ForegroundColor White
        Write-Host "  gix average: $($result.GixAverageMs)ms" -ForegroundColor White
        Write-Host "  Speedup: $($result.SpeedupFactor)x" -ForegroundColor $(if ($result.SpeedupFactor -gt 1) { 'Green' } else { 'Yellow' })
        Write-Host "  Improvement: $($result.ImprovementPct)%" -ForegroundColor $(if ($result.ImprovementPct -gt 0) { 'Green' } else { 'Yellow' })
        Write-Host "  Winner: $($result.Winner)" -ForegroundColor Cyan
        Write-Host ""

        return $result

    } catch {
        Write-Error "Failed to compare performance: $_"
        return $null
    }
}

#endregion

# Module initialization (when imported)
if (Test-GixInstalled) {
    Write-Verbose "Gix module loaded successfully. Type 'Get-Command -Module Invoke-Gix' for available functions."
} else {
    Write-Warning "Gix not installed. Install with: cargo binstall gix-cli"
    Write-Warning "Module functions will return null until gix is available."
}
