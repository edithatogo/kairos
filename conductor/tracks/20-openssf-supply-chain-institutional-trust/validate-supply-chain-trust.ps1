$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $RepoRoot

function Assert-Path {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required path: $Path"
    }
}

function Assert-Contains {
    param([string]$Path, [string]$Pattern, [string]$Description)
    if (-not (Select-String -LiteralPath $Path -Pattern $Pattern -Quiet)) {
        throw "Missing $Description in $Path"
    }
}

function Assert-RenovatePolicy {
    $config = Get-Content -LiteralPath 'renovate.json' -Raw | ConvertFrom-Json

    if (-not ($config.extends -contains 'config:recommended')) {
        throw 'Renovate config must extend config:recommended'
    }
    if (-not ($config.extends -contains ':dependencyDashboard')) {
        throw 'Renovate config must enable the dependency dashboard preset'
    }
    if ($config.dependencyDashboard -ne $true) {
        throw 'Renovate dependencyDashboard must be enabled'
    }
    if ($config.vulnerabilityAlerts.enabled -ne $true) {
        throw 'Renovate vulnerabilityAlerts.enabled must be true'
    }
    if (-not ($config.vulnerabilityAlerts.labels -contains 'security')) {
        throw 'Renovate vulnerability alerts must apply the security label'
    }
}

foreach ($path in @(
    'SECURITY.md',
    'CODEOWNERS',
    '.github/CODEOWNERS',
    'renovate.json',
    '.github/workflows/scorecard.yml',
    '.github/workflows/dependency-review.yml',
    '.github/workflows/sbom-attestations.yml',
    '.github/workflows/release-attestations.yml',
    '.github/workflows/actions-security.yml',
    '.github/workflows/workflow-security.yml',
    '.github/workflows/secret-scan.yml',
    'docs/release/supply-chain-verification.md',
    'conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md'
)) {
    Assert-Path $path
}

Assert-Contains '.github/workflows/scorecard.yml' 'ossf/scorecard-action@' 'OpenSSF Scorecard action'
Assert-Contains '.github/workflows/scorecard.yml' 'persist-credentials:\s*false' 'hardened checkout'
Assert-Contains '.github/workflows/dependency-review.yml' 'fail-on-severity:\s*high' 'high-severity dependency block'
Assert-Contains '.github/workflows/sbom-attestations.yml' 'attestations:\s*write' 'attestation permission'
Assert-Contains '.github/workflows/sbom-attestations.yml' 'sbom\.spdx\.json' 'SPDX SBOM output'
Assert-Contains '.github/workflows/sbom-attestations.yml' 'SHA256SUMS' 'checksum requirement'
Assert-Contains '.github/workflows/sbom-attestations.yml' 'RELEASE\.txt' 'release notes artifact requirement'
Assert-Contains '.github/workflows/sbom-attestations.yml' 'actions/upload-artifact@[a-f0-9]{40}' 'stable SBOM artifact upload action'
Assert-Contains '.github/workflows/release-attestations.yml' 'actions/attest' 'release attestation action'
Assert-Contains 'SECURITY.md' 'vulnerabilities|security advisory' 'vulnerability response path'
Assert-Contains 'SECURITY.md' 'business days|temporary operational exceptions|affected release stage' 'vulnerability triage and exception policy'
Assert-Contains 'CODEOWNERS' '/docs/' 'docs ownership'
Assert-Contains '.github/CODEOWNERS' '/.github/' 'workflow ownership'
Assert-Contains 'docs/release/supply-chain-verification.md' 'production_publish_enabled' 'dry-run publish boundary'
Assert-Contains 'conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md' 'Temporary operational exception' 'exception process'
Assert-Contains 'conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md' 'Release trust checklist' 'release trust checklist'
Assert-RenovatePolicy

Write-Host "track20_status=ok"
Write-Host 'supply_chain_gate=offline-trust-evidence'
