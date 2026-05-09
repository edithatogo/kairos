param(
    [string]$ReleaseWorkflowPath = ".github/workflows/release.yml",
    [string]$TrackValidatorPath = "conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1",
    [string]$ReleaseChecklistPath = "docs/release/release-checklist.md",
    [string]$SupplyChainPath = "docs/release/supply-chain-verification.md",
    [string]$MaintenanceHandoffPath = "docs/release/maintenance-handoff.md"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

$issues = [System.Collections.Generic.List[string]]::new()

function Add-Issue {
    param([string]$Message)
    $script:issues.Add($Message)
}

function Assert-Path {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        Add-Issue -Message "Missing required path: $Path"
        return $false
    }
    return $true
}

function Read-Text {
    param([string]$Path)
    if (-not (Assert-Path -Path $Path)) {
        return ""
    }
    return Get-Content -LiteralPath $Path -Raw
}

function Get-LineIndex {
    param(
        [string[]]$Lines,
        [string]$Pattern
    )

    for ($index = 0; $index -lt $Lines.Count; $index++) {
        if ($Lines[$index] -match $Pattern) {
            return $index
        }
    }

    return -1
}

Write-Host "=== Validate Track 15 Release Delivery ===" -ForegroundColor Cyan

if (Assert-Path -Path $TrackValidatorPath) {
    Write-Host "  Running Track 15 dry-run validator..." -ForegroundColor Gray
    & pwsh -NoProfile -ExecutionPolicy Bypass -File $TrackValidatorPath
    if ($LASTEXITCODE -ne 0) {
        Add-Issue -Message "Track 15 dry-run validator failed with exit code $LASTEXITCODE"
    }
}

$releaseWorkflow = Read-Text -Path $ReleaseWorkflowPath
if ($releaseWorkflow.Length -gt 0) {
    $workflowLines = @($releaseWorkflow -split "`r?`n")
    $gateIndex = Get-LineIndex -Lines $workflowLines -Pattern 'Validate release delivery gate'
    $gateRunIndex = Get-LineIndex -Lines $workflowLines -Pattern 'scripts/validate_track15_release_delivery\.ps1'
    $verifyEvidenceIndex = Get-LineIndex -Lines $workflowLines -Pattern 'build_release_manifest\.py --verify-existing'
    $uploadIndex = Get-LineIndex -Lines $workflowLines -Pattern 'Upload artifacts'
    $dryRunIndex = Get-LineIndex -Lines $workflowLines -Pattern 'release workflow is dry-run only'

    if ($gateIndex -lt 0) {
        Add-Issue -Message "release.yml is missing the Track 15 release delivery gate step"
    }
    if ($gateRunIndex -lt 0) {
        Add-Issue -Message "release.yml does not invoke scripts/validate_track15_release_delivery.ps1"
    }
    if ($uploadIndex -lt 0) {
        Add-Issue -Message "release.yml is missing the artifact upload step"
    }
    if ($verifyEvidenceIndex -lt 0) {
        Add-Issue -Message "release.yml does not verify generated release evidence before artifact upload"
    }
    if (($gateIndex -ge 0) -and ($uploadIndex -ge 0) -and ($gateIndex -gt $uploadIndex)) {
        Add-Issue -Message "Track 15 release delivery gate must run before artifact upload"
    }
    if (($gateRunIndex -ge 0) -and ($uploadIndex -ge 0) -and ($gateRunIndex -gt $uploadIndex)) {
        Add-Issue -Message "Track 15 validator invocation appears after artifact upload"
    }
    if (($verifyEvidenceIndex -ge 0) -and ($uploadIndex -ge 0) -and ($verifyEvidenceIndex -gt $uploadIndex)) {
        Add-Issue -Message "Generated release evidence verification must run before artifact upload"
    }
    if ($dryRunIndex -lt 0) {
        Add-Issue -Message "release.yml no longer reports its dry-run-only release posture"
    }
}

$checklist = Read-Text -Path $ReleaseChecklistPath
if ($checklist.Length -gt 0) {
    foreach ($needle in @(
        "SBOM generated.",
        "Provenance or attestation generated.",
        "Generated release evidence verified: ``python packaging/scripts/build_release_manifest.py --verify-existing``.",
        "Any remaining publish blockers are recorded in the maintenance handoff before leaving dry-run mode."
    )) {
        if ($checklist -notmatch [regex]::Escape($needle)) {
            Add-Issue -Message "docs/release/release-checklist.md missing required release-delivery text: $needle"
        }
    }
}

$supplyChain = Read-Text -Path $SupplyChainPath
if ($supplyChain.Length -gt 0) {
    foreach ($needle in @("SBOMs", "artifact attestations/provenance", "current blocker state is dry-run only")) {
        if ($supplyChain -notmatch [regex]::Escape($needle)) {
            Add-Issue -Message "docs/release/supply-chain-verification.md missing required text: $needle"
        }
    }
}

$handoff = Read-Text -Path $MaintenanceHandoffPath
if ($handoff.Length -gt 0) {
    foreach ($needle in @(
        "publication remains blocked",
        "name/toolchain verification remains unverified",
        "production publish stays disabled"
    )) {
        if ($handoff -notmatch [regex]::Escape($needle)) {
            Add-Issue -Message "docs/release/maintenance-handoff.md missing blocker text: $needle"
        }
    }
}

$sbomEvidence = Test-Path -LiteralPath "dist/sbom.spdx.json"
$provenanceEvidence = @(
    "dist/provenance.json",
    "dist/provenance.intoto.jsonl"
) | Where-Object { Test-Path -LiteralPath $_ }

if ($sbomEvidence -or $provenanceEvidence.Count -gt 0) {
    Write-Host "release_delivery_waits_on_attestation=false"
    Write-Host "release_delivery_evidence=sbom_or_provenance_present"
} else {
    $blocker = "registry name availability remains unverified; target-machine toolchains remain unverified; production publish stays disabled"
    Write-Host "release_delivery_waits_on_attestation=true"
    Write-Host "release_delivery_blocker=$blocker"
}

Write-Host ""
$errors = @($issues | Where-Object { $_.Length -gt 0 })
Write-Host "$($errors.Count) error(s)" -ForegroundColor $(if ($errors.Count -gt 0) { "Red" } else { "Green" })
if ($errors.Count -gt 0) {
    $issues | ForEach-Object { Write-Host $_ }
    exit 1
}

Write-Host "Track 15 release delivery validation passed." -ForegroundColor Green
