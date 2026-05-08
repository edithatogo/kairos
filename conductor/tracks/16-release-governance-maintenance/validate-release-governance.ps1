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

function Assert-RawContains {
    param([string]$Path, [string]$Pattern, [string]$Description)
    $content = Get-Content -LiteralPath $Path -Raw
    if ($content -notmatch $Pattern) {
        throw "Missing $Description in $Path"
    }
}

foreach ($path in @(
    'CHANGELOG.md',
    '.github/workflows/changelog-policy.yml',
    '.github/workflows/release.yml',
    'docs/release/release-governance.md',
    'docs/release/changelog-policy.md',
    'docs/release/compatibility.md',
    'docs/release/maintenance-handoff.md',
    'docs/release/maintainer-rotation.md',
    'docs/release/release-checklist.md',
    'docs/release/release-notes.md'
)) {
    Assert-Path $path
}

Assert-Contains 'CHANGELOG.md' 'Release governance slice' 'R2 changelog entry'
Assert-Contains 'CHANGELOG.md' 'maintainer rotation' 'maintainer rotation changelog entry'
Assert-RawContains 'conductor/tracks.yaml' '(?s)- id: 16.*?required_gates:.*?compatibility-policy' 'Track 16 compatibility-policy gate'
Assert-RawContains 'conductor/tracks.yaml' '(?s)- id: 16.*?required_gates:.*?changelog-check' 'Track 16 changelog-check gate'
Assert-Contains 'conductor/quality-gates.md' '\*\*compatibility-policy\*\*' 'central compatibility-policy gate definition'
Assert-Contains 'conductor/quality-gates.md' '\*\*changelog-check\*\*' 'central changelog-check gate definition'
Assert-Contains '.github/workflows/changelog-policy.yml' 'Public release surface changed without CHANGELOG.md' 'changelog-policy workflow gate'
Assert-Contains '.github/workflows/changelog-policy.yml' 'changelog_policy=ok' 'changelog-policy workflow success marker'
Assert-Contains '.github/workflows/release.yml' 'Validate release checklist' 'release workflow checklist step'
Assert-Contains '.github/workflows/release.yml' 'Build release manifest' 'release manifest build step'
Assert-Contains '.github/workflows/release.yml' 'Validate release manifest' 'release manifest validation step'
Assert-Contains '.github/workflows/release.yml' 'cargo publish --dry-run --workspace' 'release dry-run publish gate'
Assert-Contains '.github/workflows/release.yml' 'release workflow is dry-run only' 'release dry-run posture'
Assert-Contains 'docs/release/release-governance.md' 'Compatibility gate' 'compatibility gate'
Assert-Contains 'docs/release/release-governance.md' 'dry-run' 'dry-run release posture'
Assert-Contains 'docs/release/changelog-policy.md' 'Public release surface changed without CHANGELOG.md' 'changelog enforcement wording'
Assert-Contains 'docs/release/compatibility.md' 'Deprecation register' 'deprecation register'
Assert-Contains 'docs/release/maintenance-handoff.md' 'R2 handoff status' 'maintenance handoff status'
Assert-Contains 'docs/release/maintainer-rotation.md' 'Maturity: preview' 'maintainer rotation maturity label'
Assert-Contains 'docs/release/maintainer-rotation.md' 'Release manager' 'release manager rotation role'
Assert-Contains 'docs/release/maintainer-rotation.md' 'Escalation path' 'maintainer escalation path'
Assert-Contains 'docs/release/release-notes.md' 'production publishing' 'publish-block wording'

Write-Host "track16_status=ok"
Write-Host 'release_governance=offline-doc-gate'
Write-Host 'compatibility_policy=ok'
Write-Host 'changelog_check=ok'
