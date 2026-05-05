param(
    [switch]$RunTests
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$mpiManifest = Join-Path $repoRoot "crates\kairo-ecs-mpi\Cargo.toml"
$grpcManifest = Join-Path $repoRoot "crates\kairo-ecs-grpc\Cargo.toml"

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
    cargo check --manifest-path $mpiManifest --no-default-features
    cargo check --manifest-path $mpiManifest --features mpi --tests
    cargo check --manifest-path $grpcManifest --no-default-features
    cargo check --manifest-path $grpcManifest --features grpc --tests

    Assert-Contains -Path "crates\kairo-ecs-mpi\src\lib.rs" -Needle "Placeholder transport" -Label "MPI placeholder boundary"
    Assert-Contains -Path "crates\kairo-ecs-grpc\src\lib.rs" -Needle "Placeholder transport" -Label "gRPC placeholder boundary"
    Assert-Contains -Path "docs\distributed\deployment-guide.md" -Needle "before production use" -Label "production deployment boundary"
    Assert-Contains -Path "docs\distributed\transport-trait.md" -Needle "dependency-free" -Label "offline transport scope"

    if ($RunTests) {
        cargo test --manifest-path $mpiManifest --features mpi
        cargo test --manifest-path $grpcManifest --features grpc
    }

    Write-Host "Track 35 validator passed."
}
finally {
    Pop-Location
}
