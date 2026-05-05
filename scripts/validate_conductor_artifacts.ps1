param(
    [string]$TrackDirsPath = "conductor/tracks"
)

$ErrorActionPreference = "Stop"
$issues = @()

function Add-Issue {
    param([string]$Severity, [string]$Message)
    $issues += [PSCustomObject]@{ Severity = $Severity; Message = $Message }
}

Write-Host "=== Validate Conductor Artifacts ===" -ForegroundColor Cyan

# ------------------------------------------------------------------
# 1. Discover track directories
# ------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $TrackDirsPath)) {
    Add-Issue -Severity "ERROR" -Message "Track directories path '$TrackDirsPath' not found"
    $issues | ConvertTo-Json -Depth 3
    exit 1
}

$trackDirs = Get-ChildItem -LiteralPath $TrackDirsPath -Directory
Write-Host "  Found $($trackDirs.Count) track directories" -ForegroundColor Gray

$requiredArtifacts = @("spec.md", "plan.md", "agent-contract.md", "risk-register.md", "test-matrix.md", "handoff.md")

# ------------------------------------------------------------------
# 2. Check every track directory has all 6 required artifacts
# ------------------------------------------------------------------
Write-Host "  Checking required artifacts..." -ForegroundColor Gray
$allArtifactIssues = @()
foreach ($dir in $trackDirs) {
    foreach ($artifact in $requiredArtifacts) {
        $path = Join-Path $dir.FullName $artifact
        if (-not (Test-Path -LiteralPath $path)) {
            Add-Issue -Severity "ERROR" -Message "Missing artifact: $($dir.Name)/$artifact"
        }
    }
}

# ------------------------------------------------------------------
# 3. Validate handoff.md freshness
# ------------------------------------------------------------------
Write-Host "  Validating handoff.md freshness..." -ForegroundColor Gray
foreach ($dir in $trackDirs) {
    $handoffPath = Join-Path $dir.FullName "handoff.md"
    if (-not (Test-Path -LiteralPath $handoffPath)) { continue }
    try {
        $handoff = Get-Content -LiteralPath $handoffPath -Raw
    } catch {
        Add-Issue -Severity "ERROR" -Message "Failed to read $($dir.Name)/handoff.md: $_"
        continue
    }

    # Check for "No code files were changed" when status should indicate active work
    $trackName = $dir.Name
    # Parse tracks.yaml to get the status for this track
    $tracksYamlPath = "conductor/tracks.yaml"
    $trackStatus = "Unknown"
    if (Test-Path -LiteralPath $tracksYamlPath) {
        try {
            $tracksContent = Get-Content -LiteralPath $tracksYamlPath -Raw
            # Extract track ID from directory name
            if ($trackName -match '^(\d+)-') {
                $trackId = $matches[1]
                # Find the block for this track id
                $pattern = "(?s)- id:\s*$trackId\s*\n.*?status:\s*(\S+)"
                if ($tracksContent -match $pattern) {
                    $trackStatus = $matches[1]
                }
            }
        } catch {}
    }

    # Statuses that imply active work
    $activeStatuses = @("In Progress", "In Review", "Spec Approved", "Blocked")
    if ($trackStatus -in $activeStatuses -or $trackStatus -eq "Unknown") {
        if ($handoff -match "No code files were changed") {
            Add-Issue -Severity "WARN" -Message "$($dir.Name)/handoff.md contains 'No code files were changed' but track status is '$trackStatus'"
        }
    }

    # Check for a freshness/date indicator
    $hasDate = $handoff -match '\d{4}-\d{2}-\d{2}' -or
               $handoff -match '\b(January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}\b' -or
               $handoff -match '\b\d{1,2}\s+(January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}\b'
    if (-not $hasDate) {
        Add-Issue -Severity "WARN" -Message "$($dir.Name)/handoff.md has no visible freshness date (YYYY-MM-DD or date string)"
    }
}

# ------------------------------------------------------------------
# 4. Validate risk-register.md severity scoring format
# ------------------------------------------------------------------
Write-Host "  Validating risk-register.md format..." -ForegroundColor Gray
foreach ($dir in $trackDirs) {
    $riskPath = Join-Path $dir.FullName "risk-register.md"
    if (-not (Test-Path -LiteralPath $riskPath)) { continue }
    try {
        $riskContent = Get-Content -LiteralPath $riskPath -Raw
    } catch {
        Add-Issue -Severity "ERROR" -Message "Failed to read $($dir.Name)/risk-register.md: $_"
        continue
    }

    # Check for a table with severity-related columns
    # Look for Likelihood, Impact, Severity, or L x I patterns in a table row
    $hasRiskTable = $riskContent -match '\|\s*Risk\s*\|' -or
                    $riskContent -match '\|\s*#\s*\|\s*Risk\s*\|'
    if (-not $hasRiskTable) {
        Add-Issue -Severity "WARN" -Message "$($dir.Name)/risk-register.md does not appear to have a risk table with severity columns"
        continue
    }

    # Check for consequence/risk table header markers
    $hasConsequenceRef = $riskContent -match '\|\s*Consequence' -or
                         $riskContent -match '\|\s*Impact' -or
                         $riskContent -match '\|\s*Likelihood' -or
                         $riskContent -match '\|\s*Severity\s*\|'
    if (-not $hasConsequenceRef) {
        Add-Issue -Severity "INFO" -Message "$($dir.Name)/risk-register.md: Consider adding Likelihood/Impact/Severity columns"
    }

    # Check for severity scoring ranges
    $hasSeverityScale = $riskContent -match '\b(Low|Medium|High|Critical|Release.Blocker)\b.*\b(1-\d|1\s*[-–]\s*\d)\b'
    if (-not $hasSeverityScale) {
        Add-Issue -Severity "INFO" -Message "$($dir.Name)/risk-register.md: Consider adding severity scoring scale (Low/Medium/High/Critical)"
    }
}

# ------------------------------------------------------------------
# 5. Check handoff.md has all required sections
# ------------------------------------------------------------------
Write-Host "  Validating handoff.md structure..." -ForegroundColor Gray
$requiredHandoffSections = @("Summary", "Files changed", "Contracts consumed", "Contracts changed", "Tests added", "Known risks", "Follow-up issues", "Integration notes")
foreach ($dir in $trackDirs) {
    $handoffPath = Join-Path $dir.FullName "handoff.md"
    if (-not (Test-Path -LiteralPath $handoffPath)) { continue }
    try {
        $handoff = Get-Content -LiteralPath $handoffPath -Raw
    } catch { continue }
    foreach ($section in $requiredHandoffSections) {
    $escapedSection = [regex]::Escape($section)
    if ($handoff -notmatch "(?m)^##\s*$escapedSection") {
        Add-Issue -Severity "INFO" -Message "$($dir.Name)/handoff.md missing section: ## $section"
    }
    }
}

# ------------------------------------------------------------------
# 6. Check spec.md has release-implications section (mandatory)
# ------------------------------------------------------------------
Write-Host "  Validating spec.md mandatory sections..." -ForegroundColor Gray
foreach ($dir in $trackDirs) {
    $specPath = Join-Path $dir.FullName "spec.md"
    if (-not (Test-Path -LiteralPath $specPath)) { continue }
    try {
        $spec = Get-Content -LiteralPath $specPath -Raw
    } catch { continue }
    if ($spec -notmatch "(?m)^##\s*Release implications") {
        Add-Issue -Severity "ERROR" -Message "$($dir.Name)/spec.md missing mandatory section: ## Release implications"
    }
    if ($spec -notmatch "(?m)^##\s*Blocked paths") {
        Add-Issue -Severity "ERROR" -Message "$($dir.Name)/spec.md missing mandatory section: ## Blocked paths"
    }
}

# ------------------------------------------------------------------
# 7. Summary
# ------------------------------------------------------------------
Write-Host ""
$errors = $issues | Where-Object { $_.Severity -eq "ERROR" }
$warnings = $issues | Where-Object { $_.Severity -eq "WARN" }
$infos = $issues | Where-Object { $_.Severity -eq "INFO" }

Write-Host "$($errors.Count) error(s), $($warnings.Count) warning(s), $($infos.Count) info" -ForegroundColor $(if ($errors.Count -gt 0) { "Red" } elseif ($warnings.Count -gt 0) { "Yellow" } else { "Green" })

$issues | ConvertTo-Json -Depth 3

if ($errors.Count -gt 0) {
    exit 1
} else {
    Write-Host "Artifact validation passed." -ForegroundColor Green
    exit 0
}
