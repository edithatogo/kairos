param(
    [switch]$SkipCargoTest
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$manifest = Join-Path $repoRoot "crates\kairo-ecs-gpu\Cargo.toml"
$forbiddenGpuDependencies = @(
    "wgpu ",
    "wgpu-core",
    "wgpu-hal",
    "naga ",
    "cudarc",
    "cuda-sys"
)

Push-Location $repoRoot
try {
    cargo check --manifest-path $manifest --no-default-features
    cargo check --manifest-path $manifest --features wgpu-backend,cuda-backend --tests

    $tree = cargo tree --manifest-path $manifest --no-default-features
    foreach ($dependency in $forbiddenGpuDependencies) {
        if ($tree -match [regex]::Escape($dependency)) {
            throw "Forbidden GPU dependency leaked into default build: $dependency"
        }
    }

    if (-not $SkipCargoTest) {
        cargo test --manifest-path $manifest
    }

    Write-Host "Track 32 validator passed."
}
finally {
    Pop-Location
}
