$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$Validator = Join-Path $RepoRoot 'scripts/validate_track15_release_delivery.ps1'
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("kairos-release-evidence-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $TempRoot | Out-Null

function Invoke-ReleaseEvidenceGate {
    param([string]$EvidencePath)
    $output = & pwsh -NoProfile -ExecutionPolicy Bypass -File $Validator -EvidenceDirectoryPath $EvidencePath 2>&1 | Out-String
    return @{
        ExitCode = $LASTEXITCODE
        Output = $output
    }
}

function Write-ValidEvidence {
    param([string]$EvidencePath)
    New-Item -ItemType Directory -Path $EvidencePath -Force | Out-Null
    $expectedDigest = ('a' * 64)
    $sbom = @{
        spdxVersion = 'SPDX-2.3'
        dataLicense = 'CC0-1.0'
        SPDXID = 'SPDXRef-DOCUMENT'
        name = 'test-release'
        documentNamespace = 'https://example.invalid/test-release'
        creationInfo = @{
            creators = @('Tool: release-evidence-test')
            created = '2026-10-01T00:00:00Z'
        }
        packages = @(@{ name = 'test-package'; SPDXID = 'SPDXRef-Package' })
    } | ConvertTo-Json -Depth 10
    Set-Content -LiteralPath (Join-Path $EvidencePath 'sbom.spdx.json') -Value $sbom -NoNewline

    $manifest = @{
        artifacts = @(@{ path = 'Cargo.toml'; sha256 = $expectedDigest })
    } | ConvertTo-Json -Depth 10
    Set-Content -LiteralPath (Join-Path $EvidencePath 'release-artifact-manifest.json') -Value $manifest -NoNewline

    $provenance = @{
        _type = 'https://in-toto.io/Statement/v1'
        predicateType = 'https://slsa.dev/provenance/v1'
        subject = @(@{ name = 'Cargo.toml'; digest = @{ sha256 = $expectedDigest } })
        predicate = @{}
    } | ConvertTo-Json -Depth 10
    Set-Content -LiteralPath (Join-Path $EvidencePath 'provenance.json') -Value $provenance -NoNewline

    $sbomHash = (Get-FileHash -LiteralPath (Join-Path $EvidencePath 'sbom.spdx.json') -Algorithm SHA256).Hash.ToLowerInvariant()
    $provenanceHash = (Get-FileHash -LiteralPath (Join-Path $EvidencePath 'provenance.json') -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $EvidencePath 'SUPPLY-CHAIN-SHA256SUMS') -Value "$sbomHash  sbom.spdx.json`n$provenanceHash  provenance.json`n" -NoNewline
}

try {
    $missing = Invoke-ReleaseEvidenceGate -EvidencePath $TempRoot
    if ($missing.ExitCode -eq 0 -or $missing.Output -notmatch 'Missing required release SBOM' -or
        $missing.Output -notmatch 'Missing required release provenance' -or
        $missing.Output -notmatch 'Missing required release evidence checksum list') {
        throw "Gate did not fail closed for absent evidence. Exit=$($missing.ExitCode)`n$($missing.Output)"
    }

    Write-ValidEvidence -EvidencePath $TempRoot
    $valid = Invoke-ReleaseEvidenceGate -EvidencePath $TempRoot
    if ($valid.ExitCode -ne 0 -or $valid.Output -notmatch 'sbom_provenance_and_checksums_valid') {
        throw "Gate rejected complete matching evidence. Exit=$($valid.ExitCode)`n$($valid.Output)"
    }

    Remove-Item -LiteralPath (Join-Path $TempRoot 'provenance.json')
    $missingProvenance = Invoke-ReleaseEvidenceGate -EvidencePath $TempRoot
    if ($missingProvenance.ExitCode -eq 0 -or $missingProvenance.Output -notmatch 'Missing required release provenance') {
        throw "Gate did not fail when provenance was removed. Exit=$($missingProvenance.ExitCode)`n$($missingProvenance.Output)"
    }

    Write-ValidEvidence -EvidencePath $TempRoot
    Add-Content -LiteralPath (Join-Path $TempRoot 'sbom.spdx.json') -Value 'tampered'
    $badChecksum = Invoke-ReleaseEvidenceGate -EvidencePath $TempRoot
    if ($badChecksum.ExitCode -eq 0 -or $badChecksum.Output -notmatch 'Release evidence checksum mismatch for sbom.spdx.json') {
        throw "Gate did not fail for a checksum mismatch. Exit=$($badChecksum.ExitCode)`n$($badChecksum.Output)"
    }

    Write-ValidEvidence -EvidencePath $TempRoot
    $manifest = Get-Content -LiteralPath (Join-Path $TempRoot 'release-artifact-manifest.json') -Raw | ConvertFrom-Json
    $manifest.artifacts[0].sha256 = ('b' * 64)
    $manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $TempRoot 'release-artifact-manifest.json')
    $mismatchedSubject = Invoke-ReleaseEvidenceGate -EvidencePath $TempRoot
    if ($mismatchedSubject.ExitCode -eq 0 -or $mismatchedSubject.Output -notmatch 'does not cover all generated release manifest subjects') {
        throw "Gate did not fail when provenance omitted a manifest subject. Exit=$($mismatchedSubject.ExitCode)`n$($mismatchedSubject.Output)"
    }

    Write-Host 'Release evidence gate tests passed: missing files, provenance subject mismatch, and checksum mismatch fail.'
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
