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

function Invoke-CargoChecked {
    param(
        [string[]]$Arguments,
        [string]$Label
    )

    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed."
    }
}

Push-Location $repoRoot
try {
    Invoke-CargoChecked -Arguments @('check', '--manifest-path', $mpiManifest, '--no-default-features') -Label 'MPI no-default-features check'
    Invoke-CargoChecked -Arguments @('check', '--manifest-path', $mpiManifest, '--features', 'mpi', '--tests') -Label 'MPI tests check'
    Invoke-CargoChecked -Arguments @('check', '--manifest-path', $grpcManifest, '--no-default-features') -Label 'gRPC no-default-features check'
    Invoke-CargoChecked -Arguments @('check', '--manifest-path', $grpcManifest, '--features', 'grpc', '--tests') -Label 'gRPC tests check'

    Assert-Contains -Path "crates\kairo-ecs-mpi\src\lib.rs" -Needle "pub const MPI_PROTOCOL_ID" -Label "MPI protocol contract id"
    Assert-Contains -Path "crates\kairo-ecs-grpc\src\lib.rs" -Needle "pub const GRPC_PROTOCOL_ID" -Label "gRPC protocol contract id"
    Assert-Contains -Path "crates\kairo-ecs-grpc\src\lib.rs" -Needle "kairo.ecs.simulation.v1.SimulationTransport" -Label "gRPC Rust/proto service identity"
    Assert-Contains -Path "crates\kairo-ecs-grpc\proto\simulation.proto" -Needle "service SimulationTransport" -Label "gRPC service definition"
    Assert-Contains -Path "crates\kairo-ecs-mpi\src\lib.rs" -Needle "Placeholder transport" -Label "MPI placeholder boundary"
    Assert-Contains -Path "crates\kairo-ecs-grpc\src\lib.rs" -Needle "Placeholder transport" -Label "gRPC placeholder boundary"
    Assert-Contains -Path "docs\distributed\deployment-guide.md" -Needle "Current status: crates compile as transport scaffolds only" -Label "production deployment boundary"
    Assert-Contains -Path "docs\distributed\deployment-guide.md" -Needle "Local two-node contract proof helpers are available now" -Label "local two-node proof boundary"
    Assert-Contains -Path "docs\distributed\transport-trait.md" -Needle "Local proof scope" -Label "offline transport scope"
    Assert-Contains -Path "docs\distributed\transport-trait.md" -Needle "Two-node local contract proof is dependency-free" -Label "two-node local proof scope"
    Assert-Contains -Path "docs\distributed\entity-migration-protocol.md" -Needle "Protocol contract placeholders" -Label "migration contract boundary"
    Assert-Contains -Path "docs\distributed\entity-migration-protocol.md" -Needle "local two-node emulator path" -Label "migration local proof boundary"
    Assert-Contains -Path "docs\distributed\telemetry-aggregation.md" -Needle "Local contract evidence" -Label "telemetry contract boundary"
    Assert-Contains -Path "docs\distributed\telemetry-aggregation.md" -Needle "merge contract shape without" -Label "telemetry local proof boundary"

    if ($RunTests) {
        & rustup run stable-x86_64-pc-windows-gnu cargo test --manifest-path $mpiManifest --features mpi
        if ($LASTEXITCODE -ne 0) {
            throw "MPI runtime test failed."
        }
        & rustup run stable-x86_64-pc-windows-gnu cargo test --manifest-path $grpcManifest --features grpc
        if ($LASTEXITCODE -ne 0) {
            throw "gRPC runtime test failed."
        }
    }

    Write-Host "Track 35 validator passed."
}
finally {
    Pop-Location
}
