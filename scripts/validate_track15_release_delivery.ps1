param(
    [string]$ReleaseWorkflowPath = ".github/workflows/release.yml",
    [string]$TrackValidatorPath = "conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1",
    [string]$ReleaseChecklistPath = "docs/release/release-checklist.md",
    [string]$SupplyChainPath = "docs/release/supply-chain-verification.md",
    [string]$MaintenanceHandoffPath = "docs/release/maintenance-handoff.md",
    [string]$EvidenceDirectoryPath = "dist"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot
$EvidenceRoot = if ([System.IO.Path]::IsPathRooted($EvidenceDirectoryPath)) {
    [System.IO.Path]::GetFullPath($EvidenceDirectoryPath)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $EvidenceDirectoryPath))
}

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
    if (($gateIndex -ge 0) -and ($verifyEvidenceIndex -ge 0) -and ($gateIndex -lt $verifyEvidenceIndex)) {
        Add-Issue -Message "Track 15 release delivery gate must run after generated manifest verification"
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
        "Valid SPDX 2.3 SBOM generated at ``dist/sbom.spdx.json``.",
        "Provenance generated at ``dist/provenance.json`` or ``dist/provenance.intoto.jsonl`` and covers every SHA-256 subject in the generated release manifest.",
        "``dist/SUPPLY-CHAIN-SHA256SUMS`` verifies the SBOM and provenance files.",
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

$sbomPath = Join-Path $EvidenceRoot "sbom.spdx.json"
$provenancePath = @(
    (Join-Path $EvidenceRoot "provenance.json"),
    (Join-Path $EvidenceRoot "provenance.intoto.jsonl")
) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
$supplyChainChecksumsPath = Join-Path $EvidenceRoot "SUPPLY-CHAIN-SHA256SUMS"
$releaseManifestPath = Join-Path $EvidenceRoot "release-artifact-manifest.json"

$sbom = $null
if (-not (Test-Path -LiteralPath $sbomPath -PathType Leaf)) {
    Add-Issue -Message "Missing required release SBOM: $sbomPath"
} else {
    try {
        $sbom = Get-Content -LiteralPath $sbomPath -Raw | ConvertFrom-Json -ErrorAction Stop
        if ($sbom.spdxVersion -ne "SPDX-2.3" -or
            $sbom.SPDXID -ne "SPDXRef-DOCUMENT" -or
            -not $sbom.creationInfo.created -or
            @($sbom.creationInfo.creators).Count -eq 0 -or
            @($sbom.packages).Count -eq 0) {
            Add-Issue -Message "Release SBOM is not a populated SPDX-2.3 JSON document: $sbomPath"
        }
    } catch {
        Add-Issue -Message "Release SBOM is not valid JSON: $sbomPath ($($_.Exception.Message))"
    }
}

$provenanceDigests = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
if (-not $provenancePath) {
    Add-Issue -Message "Missing required release provenance: expected provenance.json or provenance.intoto.jsonl in $EvidenceRoot"
} else {
    try {
        $statements = [System.Collections.Generic.List[object]]::new()
        if ($provenancePath.EndsWith(".jsonl", [System.StringComparison]::OrdinalIgnoreCase)) {
            foreach ($line in (Get-Content -LiteralPath $provenancePath | Where-Object { $_.Trim().Length -gt 0 })) {
                $statements.Add(($line | ConvertFrom-Json -ErrorAction Stop))
            }
        } else {
            $provenance = Get-Content -LiteralPath $provenancePath -Raw | ConvertFrom-Json -ErrorAction Stop
            if ($provenance.dsseEnvelope.payload) {
                $payloadJson = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$provenance.dsseEnvelope.payload))
                $statements.Add(($payloadJson | ConvertFrom-Json -ErrorAction Stop))
            } else {
                $statements.Add($provenance)
            }
        }

        foreach ($statement in $statements) {
            $subjects = if ($statement.subject) { @($statement.subject) } else { @($statement.subjects) }
            foreach ($subject in $subjects) {
                $digest = [string]$subject.digest.sha256
                if ($digest -match "^[0-9a-fA-F]{64}$") {
                    [void]$provenanceDigests.Add($digest)
                }
            }
        }
        if ($provenanceDigests.Count -eq 0) {
            Add-Issue -Message "Release provenance has no subject with a SHA-256 digest: $provenancePath"
        }
    } catch {
        Add-Issue -Message "Release provenance is not valid JSON/in-toto evidence: $provenancePath ($($_.Exception.Message))"
    }
}

if (-not (Test-Path -LiteralPath $releaseManifestPath -PathType Leaf)) {
    Add-Issue -Message "Missing required generated release artifact manifest: $releaseManifestPath"
} elseif ($provenanceDigests.Count -gt 0) {
    try {
        $releaseManifest = Get-Content -LiteralPath $releaseManifestPath -Raw | ConvertFrom-Json -ErrorAction Stop
        $expectedDigests = @($releaseManifest.artifacts | ForEach-Object { [string]$_.sha256 } | Where-Object { $_ -match "^[0-9a-fA-F]{64}$" } | Select-Object -Unique)
        if ($expectedDigests.Count -eq 0) {
            Add-Issue -Message "Generated release artifact manifest has no SHA-256 subjects: $releaseManifestPath"
        } else {
            $missingProvenanceDigests = @($expectedDigests | Where-Object { -not $provenanceDigests.Contains($_) })
            if ($missingProvenanceDigests.Count -gt 0) {
                Add-Issue -Message "Release provenance does not cover all generated release manifest subjects ($($missingProvenanceDigests.Count) missing)"
            }
        }
    } catch {
        Add-Issue -Message "Generated release artifact manifest is not valid JSON: $releaseManifestPath ($($_.Exception.Message))"
    }
}

$evidenceFiles = @("sbom.spdx.json")
if ($provenancePath) {
    $evidenceFiles += [System.IO.Path]::GetFileName($provenancePath)
}
if (-not (Test-Path -LiteralPath $supplyChainChecksumsPath -PathType Leaf)) {
    Add-Issue -Message "Missing required release evidence checksum list: $supplyChainChecksumsPath"
} else {
    $checksumEntries = @{}
    foreach ($line in Get-Content -LiteralPath $supplyChainChecksumsPath) {
        if ($line -match "^\s*([0-9a-fA-F]{64})\s+\*?(.+?)\s*$") {
            $checksumEntries[[System.IO.Path]::GetFileName($Matches[2])] = $Matches[1].ToLowerInvariant()
        }
    }
    foreach ($fileName in $evidenceFiles) {
        $path = Join-Path $EvidenceRoot $fileName
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }
        if (-not $checksumEntries.ContainsKey($fileName)) {
            Add-Issue -Message "Release evidence checksum list does not cover $fileName"
            continue
        }
        $actualHash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($checksumEntries[$fileName] -ne $actualHash) {
            Add-Issue -Message "Release evidence checksum mismatch for $fileName"
        }
    }
}

if ($issues.Count -gt 0) {
    Write-Host "release_delivery_waits_on_attestation=true"
    Write-Host "release_delivery_evidence=missing_or_invalid"
} else {
    Write-Host "release_delivery_waits_on_attestation=false"
    Write-Host "release_delivery_evidence=sbom_provenance_and_checksums_valid"
}

Write-Host ""
$errors = @($issues | Where-Object { $_.Length -gt 0 })
Write-Host "$($errors.Count) error(s)" -ForegroundColor $(if ($errors.Count -gt 0) { "Red" } else { "Green" })
if ($errors.Count -gt 0) {
    $issues | ForEach-Object { Write-Host $_ }
    exit 1
}

Write-Host "Track 15 release delivery validation passed." -ForegroundColor Green
