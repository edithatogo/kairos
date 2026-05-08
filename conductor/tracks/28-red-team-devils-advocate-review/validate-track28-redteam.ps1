param(
    [string]$LedgerPath = "conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json",
    [string]$ReportPath = "reviews/red-team-report.md",
    [string]$ChecklistPath = "conductor/delivery-readiness-checklist.md",
    [int]$MaxFreshnessDays = 14
)

$ErrorActionPreference = "Stop"
$issues = @()

function Add-Issue {
    param([string]$Severity, [string]$Message)
    $script:issues += [PSCustomObject]@{ Severity = $Severity; Message = $Message }
}

function Assert-PathExists {
    param(
        [string]$Path,
        [string]$MissingSeverity = "ERROR"
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        Add-Issue -Severity $MissingSeverity -Message "Missing required path: $Path"
    }
}

function Read-RequiredText {
    param([string]$Path)
    Assert-PathExists -Path $Path
    if (Test-Path -LiteralPath $Path) {
        return Get-Content -LiteralPath $Path -Raw
    }
    return ""
}

Write-Host "=== Validate Track 28 Red-Team Gate ===" -ForegroundColor Cyan

$ledgerText = Read-RequiredText -Path $LedgerPath
$report = Read-RequiredText -Path $ReportPath
$checklist = Read-RequiredText -Path $ChecklistPath

if ($ledgerText.Trim().Length -gt 0) {
    $ledger = $ledgerText | ConvertFrom-Json
} else {
    $ledger = $null
}

if ($null -eq $ledger) {
    Add-Issue -Severity "ERROR" -Message "Claim-capability ledger did not parse."
} else {
    if (-not $ledger.freshness_date) {
        Add-Issue -Severity "ERROR" -Message "Ledger is missing freshness_date."
    } else {
        $freshnessDate = [DateTime]::ParseExact($ledger.freshness_date, "yyyy-MM-dd", [Globalization.CultureInfo]::InvariantCulture)
        $ageDays = ([DateTime]::UtcNow.Date - $freshnessDate.Date).Days
        if ($ageDays -gt $MaxFreshnessDays) {
            Add-Issue -Severity "ERROR" -Message "Ledger freshness date $($ledger.freshness_date) is older than $MaxFreshnessDays days."
        }
    }

    $entries = @($ledger.entries)
    if ($entries.Count -eq 0) {
        Add-Issue -Severity "ERROR" -Message "Ledger has no claim-capability entries."
    }

    $ownerRequired = @($ledger.owner_required_for)
    $missingOwners = @($entries | Where-Object { $_.class -in $ownerRequired -and [string]::IsNullOrWhiteSpace($_.owner) })
    if ($missingOwners.Count -gt 0) {
        Add-Issue -Severity "ERROR" -Message "Ledger has blocker/warning entries without owners: $($missingOwners.id -join ', ')"
    }

    $missingStageImpact = @($entries | Where-Object { $_.class -in @("blocker", "warning") -and [string]::IsNullOrWhiteSpace($_.stage_impact) })
    if ($missingStageImpact.Count -gt 0) {
        Add-Issue -Severity "ERROR" -Message "Ledger has blocker/warning entries without stage impact: $($missingStageImpact.id -join ', ')"
    }

    foreach ($entry in $entries) {
        foreach ($path in @($entry.evidence)) {
            if ($path -and $path -notmatch '^\.github/workflows/') {
                $missingSeverity = if ($entry.class -eq "blocker") { "WARN" } else { "ERROR" }
                Assert-PathExists -Path $path -MissingSeverity $missingSeverity
            }
        }
    }

    $criticalBlockers = @($entries | Where-Object {
        ($_.class -eq "critical" -or $_.severity -eq "critical") -and
        ($_.status -ne "accepted" -and $_.status -ne "mitigated")
    })
    if ($criticalBlockers.Count -gt 0) {
        Add-Issue -Severity "ERROR" -Message "Unresolved critical release blockers: $($criticalBlockers.id -join ', ')"
    }

    $recordedBlockers = @($entries | Where-Object { $_.class -eq "blocker" })
    Write-Host "Ledger entries: $($entries.Count); recorded stage-scoped blockers: $($recordedBlockers.Count); unresolved critical blockers: $($criticalBlockers.Count)"
}

foreach ($needle in @(
    "Freshness date",
    "Blocker rubric",
    "Stage impact",
    "Red-team release gate"
)) {
    if ($report -notmatch [regex]::Escape($needle)) {
        Add-Issue -Severity "ERROR" -Message "Report missing required marker: $needle"
    }
}

foreach ($needle in @(
    "Claim-versus-capability ledger",
    "Blocker rubric",
    "Red-team validation commands"
)) {
    if ($checklist -notmatch [regex]::Escape($needle)) {
        Add-Issue -Severity "ERROR" -Message "Delivery checklist missing Track 28 marker: $needle"
    }
}

$errors = @($issues | Where-Object { $_.Severity -eq "ERROR" })
$warnings = @($issues | Where-Object { $_.Severity -eq "WARN" })
Write-Host ""
Write-Host "$($errors.Count) error(s), $($warnings.Count) warning(s)" -ForegroundColor $(if ($errors.Count -gt 0) { "Red" } elseif ($warnings.Count -gt 0) { "Yellow" } else { "Green" })
$issues | ConvertTo-Json -Depth 3

if ($errors.Count -gt 0) {
    exit 1
}

Write-Host "Track 28 no-critical-release-blockers gate passed; stage-scoped blockers remain recorded for release planning." -ForegroundColor Green
