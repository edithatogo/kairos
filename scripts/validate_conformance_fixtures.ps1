param(
    [string]$ManifestPath = "conformance/fixtures/manifest.json",
    [string]$FixturesRoot = "conformance/fixtures",
    [string]$BenchPlanPath = "benches/benchmark-plan.md"
)

$ErrorActionPreference = "Stop"

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$requiredReadyIds = @(
    "scheduler_ordering_v1",
    "scheduler_cancellation_v1",
    "rng_reproducibility_v1"
)

foreach ($id in $requiredReadyIds) {
    $fixture = $manifest.fixtures | Where-Object { $_.id -eq $id }
    if (-not $fixture) {
        throw "Missing fixture in manifest: $id"
    }
    if ($fixture.status -ne "ready") {
        throw "Fixture is not ready: $id"
    }
    $fixturePath = Join-Path $FixturesRoot $fixture.source
    if (-not (Test-Path -LiteralPath $fixturePath)) {
        throw "Missing ready fixture file: $fixturePath"
    }
    Get-Content -LiteralPath $fixturePath -Raw | ConvertFrom-Json | Out-Null
}

$benchmarkPlan = Get-Content -LiteralPath $BenchPlanPath -Raw
if (-not $manifest.benchmarks) {
    throw "Missing benchmark inventory in manifest"
}

$requiredScenarios = @(
    "schedule_1m_events",
    "pop_1m_events",
    "schedule_cancel_1m_mixed",
    "create_1m_entities",
    "component_insert_1m",
    "hybrid_des_abm_smoke_100k"
)

foreach ($scenario in $requiredScenarios) {
    $benchmark = $manifest.benchmarks | Where-Object { $_.id -eq $scenario }
    if (-not $benchmark) {
        throw "Missing benchmark in manifest: $scenario"
    }
    if ($benchmark.status -ne "canonical") {
        throw "Benchmark is not canonical: $scenario"
    }
    if ($benchmarkPlan -notmatch [regex]::Escape($scenario)) {
        throw "Missing benchmark scenario: $scenario"
    }
}

Write-Host "Conformance fixture validation passed."
