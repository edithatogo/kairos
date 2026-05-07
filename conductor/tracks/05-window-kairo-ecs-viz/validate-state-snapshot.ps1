param(
    [switch]$SkipCargo
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $repoRoot

$checks = @(
    @{
        Path = 'crates/kairo-ecs-state/src/lib.rs'
        Needles = @(
            'pub struct EntitySnapshot',
            'pub struct WorldSnapshot',
            'pub fn snapshot(&self) -> WorldSnapshot',
            'sort_unstable_by_key'
        )
    },
    @{
        Path = 'crates/kairo-ecs-viz/src/lib.rs'
        Needles = @(
            'use kairo_ecs_state::WorldSnapshot;',
            'pub fn from_world_snapshot',
            'pub fn from_world_snapshot_and_events',
            'pub fn render_headless_text',
            'pub fn render_fixture_json',
            'pub fn render_headless_svg',
            'kairo_ecs.visualization.frame.v1',
            'converts_world_snapshot_to_deterministic_headless_frame'
        )
    },
    @{
        Path = 'examples/viz/headless-snapshot/src/main.rs'
        Needles = @(
            'use kairo_ecs_state::World;',
            'RenderFrame::from_world_snapshot',
            'render_fixture_json',
            'render_headless_svg'
        )
    }
)

$failures = @()
foreach ($check in $checks) {
    $content = Get-Content -LiteralPath $check.Path -Raw
    foreach ($needle in $check.Needles) {
        if ($content -notlike "*$needle*") {
            $failures += "$($check.Path) missing required evidence: $needle"
        }
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}

if (-not $SkipCargo) {
    cargo check -p kairo-ecs-state --tests
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo check --manifest-path crates/kairo-ecs-viz/Cargo.toml --no-default-features --tests
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host 'Track 05 state snapshot validator passed.'
