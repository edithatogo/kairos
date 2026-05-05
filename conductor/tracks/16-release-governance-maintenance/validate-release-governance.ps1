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

foreach ($path in @(
    'CHANGELOG.md',
    'docs/release/release-governance.md',
    'docs/release/changelog-policy.md',
    'docs/release/compatibility.md',
    'docs/release/maintenance-handoff.md',
    'docs/release/release-checklist.md',
    'docs/release/release-notes.md'
)) {
    Assert-Path $path
}

Assert-Contains 'CHANGELOG.md' 'Release governance slice' 'R2 changelog entry'
Assert-Contains 'docs/release/release-governance.md' 'Compatibility gate' 'compatibility gate'
Assert-Contains 'docs/release/release-governance.md' 'dry-run' 'dry-run release posture'
Assert-Contains 'docs/release/changelog-policy.md' 'Public release surface changed without CHANGELOG.md' 'changelog enforcement wording'
Assert-Contains 'docs/release/compatibility.md' 'Deprecation register' 'deprecation register'
Assert-Contains 'docs/release/maintenance-handoff.md' 'R2 handoff status' 'maintenance handoff status'
Assert-Contains 'docs/release/release-notes.md' 'production publishing' 'publish-block wording'

Write-Host "track16_status=ok"
Write-Host 'release_governance=offline-doc-gate'
