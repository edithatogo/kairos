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
    'docs/benchmarks/README.md',
    'benches/benchmark-plan.md',
    'benches/benchmark-smoke.json',
    'benches/benchmark_smoke.py',
    'benches/benchmark_reproducibility.py',
    'conformance/fixtures/manifest.json',
    '.github/workflows/benchmark-smoke.yml',
    'docs/benchmarks/reproduce-comparison.md',
    'website/docs-link-manifest.json'
)) {
    Assert-Path $path
}

$fixtureManifest = Get-Content -LiteralPath 'conformance/fixtures/manifest.json' -Raw | ConvertFrom-Json
$readyIds = @($fixtureManifest.fixtures | Where-Object { $_.status -eq 'ready' } | ForEach-Object { $_.id })
foreach ($id in @('scheduler_ordering_v1', 'scheduler_cancellation_v1', 'rng_reproducibility_v1')) {
    if ($readyIds -notcontains $id) {
        throw "Missing ready fixture id: $id"
    }
    Assert-Contains 'docs/benchmarks/reproduce-comparison.md' $id "public reproduction fixture $id"
}

$benchmarkIds = @($fixtureManifest.benchmarks | Where-Object { $_.status -eq 'canonical' } | ForEach-Object { $_.id })
foreach ($id in @('schedule_1m_events', 'pop_1m_events', 'schedule_cancel_1m_mixed', 'create_1m_entities', 'component_insert_1m', 'hybrid_des_abm_smoke_100k')) {
    if ($benchmarkIds -notcontains $id) {
        throw "Missing canonical benchmark id: $id"
    }
    Assert-Contains 'benches/benchmark-plan.md' $id "benchmark plan id $id"
}

Assert-Contains 'docs/benchmarks/reproduce-comparison.md' 'metadata gates' 'metadata-gate caveat'
Assert-Contains 'docs/benchmarks/README.md' 'Benchmark readers should start' 'benchmark landing page'
Assert-Contains '.github/workflows/benchmark-smoke.yml' 'python benches/benchmark_smoke\.py' 'benchmark metadata smoke gate'
Assert-Contains '.github/workflows/benchmark-smoke.yml' 'cargo check -p kairo-ecs-bench' 'bench crate compile gate'

Write-Host "track18_status=ok"
Write-Host "ready_fixtures=$($readyIds -join ',')"
Write-Host "canonical_benchmarks=$($benchmarkIds.Count)"
