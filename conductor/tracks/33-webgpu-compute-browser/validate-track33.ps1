param(
    [switch]$SkipNpm,
    [switch]$RunRuntimeTests
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$crateManifest = Join-Path $repoRoot "crates\kairo-ecs-webgpu\Cargo.toml"
$demoRoot = Join-Path $repoRoot "website\webgpu-demo"

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

function Assert-NotContains {
    param(
        [string]$Path,
        [string[]]$Needles,
        [string]$Label
    )

    $text = Get-Content -LiteralPath $Path -Raw
    foreach ($needle in $Needles) {
        if ($text.Contains($needle)) {
            throw "Unexpected $Label in $Path`: $needle"
        }
    }
}

function Assert-NoPrematureRuntimeClaims {
    param(
        [string]$Path
    )

    $text = Get-Content -LiteralPath $Path -Raw
    $claimPatterns = @(
        "validated parity",
        "achieved parity",
        "30\s*fps",
        ">=\s*30",
        "speedup achieved",
        "runtime-ready",
        "WebGPU\s+(?:is\s+)?(?:faster|accelerated|validated|verified|production-ready)",
        "(?:faster|accelerated|validated|verified|production-ready)\s+WebGPU",
        "performance\s+(?:validated|verified|proven|achieved|ready)",
        "(?:validated|verified|proven|achieved)\s+performance"
    )
    $allowedBoundaryPatterns = @(
        "not claimed",
        "No browser dispatch or framerate claims",
        "backend not configured",
        "backend-not-configured",
        "blocked until",
        "browser GPU device proof remains blocked",
        "forbid unverified performance wording"
    )

    $lineNumber = 0
    foreach ($line in ($text -split "`r?`n")) {
        $lineNumber += 1
        $isAllowedBoundary = $false
        foreach ($allowedPattern in $allowedBoundaryPatterns) {
            if ($line -match $allowedPattern) {
                $isAllowedBoundary = $true
                break
            }
        }

        if ($isAllowedBoundary) {
            continue
        }

        foreach ($claimPattern in $claimPatterns) {
            if ($line -match $claimPattern) {
                throw "Unexpected premature runtime/performance claim in $Path at line $lineNumber`: $line"
            }
        }
    }
}

Push-Location $repoRoot
try {
    cargo check --manifest-path $crateManifest --no-default-features
    cargo check --manifest-path $crateManifest --features webgpu --tests

    Assert-Contains -Path (Join-Path $demoRoot "README.md") -Needle "backend not configured" -Label "explicit WebGPU unavailable boundary"
    Assert-Contains -Path (Join-Path $demoRoot "index.html") -Needle "not claimed" -Label "no performance claim table cell"
    Assert-Contains -Path (Join-Path $demoRoot "src/main.js") -Needle "fallbackContract" -Label "fallback contract object"
    Assert-Contains -Path (Join-Path $demoRoot "src/main.js") -Needle "resolveBackendStatus" -Label "backend resolver contract"
    Assert-Contains -Path (Join-Path $repoRoot "docs\gpu-compute\webgpu-wgsl-subset.md") -Needle "GPU-free" -Label "offline WGSL validator scope"
    Assert-Contains -Path (Join-Path $demoRoot "package.json") -Needle "validate:wgsl" -Label "wgsl validation script"
    Assert-Contains -Path (Join-Path $repoRoot "conductor\tracks\33-webgpu-compute-browser\test-matrix.md") -Needle "blocked" -Label "host-gated closeout blockers"
    Assert-Contains -Path (Join-Path $repoRoot "conductor\tracks\33-webgpu-compute-browser\plan.md") -Needle "Next-harvest" -Label "next-harvest host-only planning"

    $claimScanPaths = @(
      "docs\gpu-compute\webgpu-comparison.md",
      "website\webgpu-demo\README.md",
      "website\webgpu-demo\index.html",
      "website\webgpu-demo\src\main.js",
      "conductor\tracks\33-webgpu-compute-browser\handoff.md",
      "conductor\tracks\33-webgpu-compute-browser\spec.md",
      "conductor\tracks\33-webgpu-compute-browser\test-matrix.md"
    )

    foreach ($claimPath in $claimScanPaths) {
      Assert-NoPrematureRuntimeClaims -Path (Join-Path $repoRoot $claimPath)
    }

    Assert-Contains -Path (Join-Path $repoRoot "crates\kairo-ecs-webgpu\src\bridge.rs") -Needle "InvalidLengthForStride" -Label "buffer descriptor length/stride validation"
    Assert-Contains -Path (Join-Path $repoRoot "crates\kairo-ecs-webgpu\src\bridge.rs") -Needle "ZeroStride" -Label "buffer descriptor zero-stride validation"
    Assert-Contains -Path (Join-Path $repoRoot "crates\kairo-ecs-webgpu\src\capability.rs") -Needle "effective_backend == ComputeBackend::BrowserWebGpu" -Label "browser GPU validation metadata"

    if (-not $SkipNpm) {
        npm test --prefix $demoRoot
    }

    if ($RunRuntimeTests) {
        cargo +stable-x86_64-pc-windows-gnu test --manifest-path $crateManifest
        if ($LASTEXITCODE -ne 0) {
            throw "WebGPU runtime cargo tests failed."
        }
    }

    Write-Host "Track 33 validator passed."
}
finally {
    Pop-Location
}
