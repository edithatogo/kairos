param(
    [string]$TracksYamlPath = "conductor/tracks.yaml",
    [string]$TracksPath = "conductor/tracks"
)

$ErrorActionPreference = "Stop"
$issues = @()

function Add-Issue {
    param([string]$Severity, [string]$Message)
    $script:issues += [PSCustomObject]@{ Severity = $Severity; Message = $Message }
}

function Read-Text {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        Add-Issue -Severity "ERROR" -Message "Missing required file: $Path"
        return ""
    }
    return Get-Content -LiteralPath $Path -Raw
}

function Assert-Contains {
    param([string]$Path, [string]$Needle, [string]$Label)
    $content = Read-Text -Path $Path
    if ($content -notmatch [regex]::Escape($Needle)) {
        Add-Issue -Severity "ERROR" -Message "$Label missing required text in ${Path}: $Needle"
    }
}

function Get-TrackRecords {
    param([string]$Path)
    $content = Read-Text -Path $Path
    $records = @()
    $matches = [regex]::Matches($content, '(?ms)^\s*-\s*id:\s*(\d+)\s*$.*?(?=^\s*-\s*id:\s*\d+\s*$|\z)')
    foreach ($match in $matches) {
        $block = $match.Value
        $id = "{0:D2}" -f [int]$match.Groups[1].Value
        $status = "Unknown"
        if ($block -match '(?m)^\s*status:\s*(.+?)\s*$') {
            $status = $Matches[1].Trim()
        }
        $records += [PSCustomObject]@{ Id = $id; Status = $status; Block = $block }
    }
    return $records
}

Write-Host "=== Validate Conductor Phase Gates ===" -ForegroundColor Cyan

Assert-Contains -Path "conductor/phase-gate-policy.md" -Needle "Automatic phase closeout gate" -Label "phase gate policy"
Assert-Contains -Path "conductor/phase-gate-policy.md" -Needle '$conductor-review' -Label "phase gate policy"
Assert-Contains -Path "conductor/workflow.md" -Needle "Automatic phase closeout gate" -Label "workflow"
Assert-Contains -Path "conductor/workflow.md" -Needle "auto-apply accepted review fixes" -Label "workflow"
Assert-Contains -Path ".github/workflows/validate-conductor.yml" -Needle "Validate conductor phase gates" -Label "validate conductor workflow"
Assert-Contains -Path ".github/workflows/validate-conductor.yml" -Needle "scripts/validate_conductor*.ps1" -Label "validate conductor workflow paths"
Assert-Contains -Path "scripts/validate_conductor_setup.ps1" -Needle "validate_conductor_phase_gates.ps1" -Label "setup validator"
Assert-Contains -Path "conductor/quality-gates.md" -Needle "**phase-closeout-check**" -Label "quality gate catalogue"

$terminalStatuses = @("Done", "Deferred", "Cancelled")
$records = @(Get-TrackRecords -Path $TracksYamlPath)
if ($records.Count -eq 0) {
    Add-Issue -Severity "ERROR" -Message "No tracks parsed from $TracksYamlPath"
}

foreach ($record in $records) {
    if ($record.Status -in $terminalStatuses) {
        continue
    }

    if ($record.Block -notmatch '(?m)^\s*-\s*phase-closeout-check\s*$') {
        Add-Issue -Severity "ERROR" -Message "Track $($record.Id) is non-terminal but does not require phase-closeout-check in $TracksYamlPath"
    }

    $trackDir = Get-ChildItem -LiteralPath $TracksPath -Directory |
        Where-Object { $_.Name -match "^$($record.Id)-" } |
        Select-Object -First 1
    if (-not $trackDir) {
        Add-Issue -Severity "ERROR" -Message "No directory found for non-terminal track $($record.Id)"
        continue
    }

    $planPath = Join-Path $trackDir.FullName "plan.md"
    $handoffPath = Join-Path $trackDir.FullName "handoff.md"
    $matrixPath = Join-Path $trackDir.FullName "test-matrix.md"

    $plan = Read-Text -Path $planPath
    foreach ($needle in @(
        "## Phase closeout gate",
        '$conductor-review',
        "Auto-apply accepted review fixes",
        "validate_conductor_phase_gates.ps1",
        "Commit and push the cleaned slice"
    )) {
        if ($plan -notmatch [regex]::Escape($needle)) {
            Add-Issue -Severity "ERROR" -Message "$($trackDir.Name)/plan.md missing phase gate marker: $needle"
        }
    }

    $handoff = Read-Text -Path $handoffPath
    foreach ($needle in @("## Follow-up issues", "## Integration notes", "## Phase closeout evidence")) {
        if ($handoff -notmatch "(?m)^$([regex]::Escape($needle))") {
            Add-Issue -Severity "ERROR" -Message "$($trackDir.Name)/handoff.md missing required closeout section: $needle"
        }
    }

    $matrix = Read-Text -Path $matrixPath
    if ($matrix -notmatch "validate_conductor_phase_gates\.ps1") {
        Add-Issue -Severity "ERROR" -Message "$($trackDir.Name)/test-matrix.md does not name the phase gate validator"
    }
}

Write-Host ""
$errors = @($issues | Where-Object { $_.Severity -eq "ERROR" })
$warnings = @($issues | Where-Object { $_.Severity -eq "WARN" })
Write-Host "$($errors.Count) error(s), $($warnings.Count) warning(s)" -ForegroundColor $(if ($errors.Count -gt 0) { "Red" } elseif ($warnings.Count -gt 0) { "Yellow" } else { "Green" })
$issues | ConvertTo-Json -Depth 3

if ($errors.Count -gt 0) {
    exit 1
}

Write-Host "Conductor phase gate validation passed." -ForegroundColor Green
