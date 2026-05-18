param(
    [switch]$SkipCargoTest,
    [switch]$RunRuntimeTests
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

function Invoke-NativeChecked {
    param(
        [string]$Label,
        [string[]]$Command
    )

    & $Command[0] @($Command | Select-Object -Skip 1)
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE."
    }
}

function Assert-Contains {
    param(
        [string]$Path,
        [string]$Needle,
        [string]$Label
    )

    $text = Get-Content -LiteralPath $Path -Raw
    if (-not $text.Contains($Needle)) {
        throw "Missing $Label in $Path`: $Needle"
    }
}

Push-Location $repoRoot
try {
    Invoke-NativeChecked -Label "GPU no-default-features check" -Command @("cargo", "check", "--manifest-path", $manifest, "--no-default-features")
    Invoke-NativeChecked -Label "GPU feature tests compile check" -Command @("cargo", "check", "--manifest-path", $manifest, "--features", "wgpu-backend,cuda-backend", "--tests")

    $tree = cargo tree --manifest-path $manifest --no-default-features
    foreach ($dependency in $forbiddenGpuDependencies) {
        if ($tree -match [regex]::Escape($dependency)) {
            throw "Forbidden GPU dependency leaked into default build: $dependency"
        }
    }

    Assert-Contains -Path "crates\kairo-ecs-gpu\src\compute.rs" -Needle "#[repr(C)]" -Label "stable host/device struct layout"
    Assert-Contains -Path "crates\kairo-ecs-gpu\src\compute.rs" -Needle "wrapping_mul(747_796_405)" -Label "CPU/WGSL jitter constant alignment"
    Assert-Contains -Path "crates\kairo-ecs-gpu\src\shaders\abm_step.wgsl" -Needle "747796405u" -Label "WGSL jitter constant alignment"
    Assert-Contains -Path "crates\kairo-ecs-gpu\src\shaders\des_dispatch.wgsl" -Needle "event_count" -Label "DES dispatch event-count guard"
    Assert-Contains -Path "crates\kairo-ecs-gpu\src\shaders\des_dispatch.wgsl" -Needle "entity_count" -Label "DES dispatch entity-count guard"

    if (-not $SkipCargoTest -and $RunRuntimeTests) {
        Invoke-NativeChecked -Label "GPU runtime cargo tests" -Command @("rustup", "run", "stable-x86_64-pc-windows-gnu", "cargo", "test", "--manifest-path", $manifest)
    } else {
        Write-Host "Skipping runtime integration tests for host-only hardening."
    }

    Write-Host "Track 32 validator passed."
}
finally {
    Pop-Location
}
