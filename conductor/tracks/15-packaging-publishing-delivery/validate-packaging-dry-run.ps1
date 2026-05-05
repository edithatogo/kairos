$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $RepoRoot

function Assert-Path {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required path: $Path"
    }
}

Assert-Path 'packaging/release-package-manifest.json'
Assert-Path 'packaging/scripts/build_release_manifest.py'
Assert-Path 'packaging/README.md'

$manifest = Get-Content -LiteralPath 'packaging/release-package-manifest.json' -Raw | ConvertFrom-Json
if ($manifest.production_publish_enabled -ne $false) {
    throw 'production_publish_enabled must remain false for this track'
}
if ($manifest.release_stage -ne 'r2-dry-run') {
    throw "Expected release_stage r2-dry-run, found $($manifest.release_stage)"
}

$expected = @('rust', 'python', 'r', 'julia', 'typescript', 'csharp', 'go')
$actual = @($manifest.surfaces | ForEach-Object { $_.ecosystem })
foreach ($ecosystem in $expected) {
    if ($actual -notcontains $ecosystem) {
        throw "Missing package surface: $ecosystem"
    }
}

foreach ($surface in $manifest.surfaces) {
    if ($surface.registry_mode -match 'production|live|publish-now') {
        throw "Unsafe registry_mode for $($surface.ecosystem): $($surface.registry_mode)"
    }
    if (-not $surface.fallback) {
        throw "Missing fallback for $($surface.ecosystem)"
    }
    if (-not $surface.dry_run_commands -or $surface.dry_run_commands.Count -eq 0) {
        throw "Missing dry_run_commands for $($surface.ecosystem)"
    }
    foreach ($entry in $surface.manifests) {
        Assert-Path $entry.path
    }
}

if ($manifest.output.artifact_manifest -ne 'dist/release-artifact-manifest.json') {
    throw 'Unexpected artifact_manifest output path'
}
if ($manifest.output.checksum_manifest -ne 'dist/SHA256SUMS') {
    throw 'Unexpected checksum_manifest output path'
}

Write-Host "track15_status=ok"
Write-Host "ecosystems=$($actual -join ',')"
Write-Host "production_publish_enabled=$($manifest.production_publish_enabled)"
