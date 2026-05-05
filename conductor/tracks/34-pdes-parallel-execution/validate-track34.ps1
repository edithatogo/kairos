param(
    [switch]$RunTests
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$crateManifest = Join-Path $repoRoot "crates\kairo-ecs-pdes\Cargo.toml"

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
    cargo check --manifest-path $crateManifest --features pdes --tests

    Assert-Contains -Path "docs\pdes\validation-evidence.md" -Needle "does not claim scheduler integration or real speedup" -Label "evidence boundary"
    Assert-Contains -Path "docs\pdes\benchmark-results.md" -Needle "No scaling run has been performed yet" -Label "no scaling result boundary"
    Assert-Contains -Path "benches\pdes\README.md" -Needle "No benchmark result should be marked complete" -Label "benchmark completion boundary"

    if ($RunTests) {
        cargo test --manifest-path $crateManifest --features pdes
    }

    Write-Host "Track 34 validator passed."
}
finally {
    Pop-Location
}
