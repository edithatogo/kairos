param(
    [switch]$SkipCargoTests
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$oldPythonPath = $env:PYTHONPATH
$oldRustupToolchain = $env:RUSTUP_TOOLCHAIN
$gnuToolchain = "stable-x86_64-pc-windows-gnu"
$useGnuToolchain = $IsWindows -and (rustup toolchain list | Select-String -SimpleMatch $gnuToolchain)
$env:RUSTUP_TOOLCHAIN = if ($useGnuToolchain) { $gnuToolchain } else { $oldRustupToolchain }
$env:PYTHONPATH = (Join-Path $repo "python\kairo_gym\src")
Push-Location $repo
try {
    $commands = @(
        @{ Name = "streaming check"; Command = @("cargo", "check", "--manifest-path", "crates\kairo-ecs-streaming\Cargo.toml", "--all-features", "--tests") },
        @{ Name = "ml check"; Command = @("cargo", "check", "--manifest-path", "crates\kairo-ecs-ml\Cargo.toml", "--all-features", "--tests") },
        @{ Name = "fmi check"; Command = @("cargo", "check", "--manifest-path", "crates\kairo-ecs-fmi\Cargo.toml", "--all-features", "--tests") },
        @{ Name = "debug check"; Command = @("cargo", "check", "--manifest-path", "crates\kairo-ecs-debug\Cargo.toml", "--tests") },
        @{ Name = "kairo_gym unittest"; Command = @("python", "-m", "unittest", "discover", "-s", "python\kairo_gym\tests") },
        @{ Name = "cloud offline validator"; Command = @("python", "cloud\validate_cloud_hpc.py") },
        @{ Name = "time travel demo validator"; Command = @("node", "website\time-travel-demo\validate-demo.mjs") }
    )

    foreach ($item in $commands) {
        Write-Host "==> $($item.Name)"
        $exe = $item.Command[0]
        $args = $item.Command[1..($item.Command.Count - 1)]
        & $exe @args
        if ($LASTEXITCODE -ne 0) {
            throw "$($item.Name) failed with exit code $LASTEXITCODE"
        }
    }

    $docs = @(
        @{ Path = "docs\streaming\architecture.md"; Needles = @("contract test doubles only", "not Kafka, NATS, WebSocket") },
        @{ Path = "docs\streaming\broker-setup.md"; Needles = @("Tutorial: local contract smoke", "Evidence boundary") },
        @{ Path = "docs\ml\architecture.md"; Needles = @("contract double", "does not load or execute a real ONNX graph") },
        @{ Path = "docs\ml\surrogate-authoring.md"; Needles = @("Tutorial: dependency-free surrogate scaffold", "Evidence boundary") },
        @{ Path = "docs\fmi-digital-twin\import-guide.md"; Needles = @("does not perform XSD validation", "still needs the dynamic loader implementation", "Tutorial: unpacked FMU preflight", "Evidence boundary") },
        @{ Path = "docs\cloud-hpc\checkpoint-spot-policy.md"; Needles = @("does not prove", "Live provider validation", "Tutorial: offline cloud/HPC smoke", "Evidence boundary") },
        @{ Path = "docs\debugging\trace-format.md"; Needles = @("offline line encoding", "Arrow IPC serialization remains") },
        @{ Path = "docs\debugging\cli-reference.md"; Needles = @("Tutorial: local trace smoke", "Evidence boundary") }
    )

    foreach ($entry in $docs) {
        $text = Get-Content -LiteralPath $entry.Path -Raw
        foreach ($needle in $entry.Needles) {
            if ($text -notlike "*$needle*") {
                throw "$($entry.Path) is missing bounded-claim text: $needle"
            }
        }
    }

    $invalidExperiment = Join-Path $repo "k8s\operator\invalid-experiment.validation.json"
    @{
        kind = "KairoECSExperiment"
        spec = @{
            image = "kairo-ecs-cli:dev"
            parallelism = 0
            storage = @{
                backend = "filesystem"
                path = "/tmp/out"
            }
        }
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $invalidExperiment -Encoding UTF8
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $operator = & python "k8s\operator\kairoecs_operator.py" --experiment $invalidExperiment 2>&1
    $operatorExit = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    Remove-Item -LiteralPath $invalidExperiment -Force
    if ($operatorExit -eq 0) {
        throw "Kubernetes operator validation did not reject invalid parallelism"
    }

    if (-not $SkipCargoTests) {
        Write-Host "==> cargo test probes"
        & cargo test --manifest-path crates\kairo-ecs-streaming\Cargo.toml --no-default-features
        if ($LASTEXITCODE -ne 0) { throw "streaming cargo test failed" }
        & cargo test --manifest-path crates\kairo-ecs-ml\Cargo.toml --no-default-features
        if ($LASTEXITCODE -ne 0) { throw "ml cargo test failed" }
    }

    Write-Host "Track 36-40 offline validation passed."
}
finally {
    Pop-Location
    $env:PYTHONPATH = $oldPythonPath
    $env:RUSTUP_TOOLCHAIN = $oldRustupToolchain
}
