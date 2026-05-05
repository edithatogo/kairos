param(
    [switch]$SkipNpm
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

Push-Location $repoRoot
try {
    cargo check --manifest-path $crateManifest --no-default-features
    cargo check --manifest-path $crateManifest --features webgpu --tests

    Assert-Contains -Path (Join-Path $demoRoot "README.md") -Needle "backend not configured" -Label "explicit WebGPU unavailable boundary"
    Assert-Contains -Path (Join-Path $demoRoot "index.html") -Needle "not claimed" -Label "no performance claim table cell"
    Assert-Contains -Path (Join-Path $repoRoot "docs\gpu-compute\webgpu-wgsl-subset.md") -Needle "GPU-free" -Label "offline WGSL validator scope"

    if (-not $SkipNpm) {
        npm test --prefix $demoRoot
    }

    Write-Host "Track 33 validator passed."
}
finally {
    Pop-Location
}
