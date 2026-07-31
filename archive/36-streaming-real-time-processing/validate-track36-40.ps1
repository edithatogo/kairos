param(
    [switch]$SkipCargoTests
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$oldPythonPath = $env:PYTHONPATH
$oldRustupToolchain = $env:RUSTUP_TOOLCHAIN
$gnuToolchain = "stable-x86_64-pc-windows-gnu"

function Test-KairosWindowsHost {
    $isWindowsVariable = Get-Variable -Name IsWindows -ErrorAction SilentlyContinue
    if ($isWindowsVariable -and $isWindowsVariable.Value) {
        return $true
    }

    return ($env:OS -eq "Windows_NT" -or [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT)
}

function Test-KairosRustupToolchainAvailable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Toolchain
    )

    $rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE ".rustup" }
    $toolchainPath = Join-Path (Join-Path $rustupHome "toolchains") $Toolchain
    $hasToolchainDirectory = Test-Path -LiteralPath $toolchainPath

    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        if ($hasToolchainDirectory) {
            Write-Host "rustup not found, but $Toolchain exists under $rustupHome; using it for this validation run."
            return $true
        }

        Write-Host "rustup not found and $Toolchain was not found under $rustupHome; leaving RUSTUP_TOOLCHAIN unchanged."
        return $false
    }

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $toolchains = & rustup toolchain list 2>&1
        $rustupExit = $LASTEXITCODE
    }
    catch {
        if ($hasToolchainDirectory) {
            Write-Warning "Could not list rustup toolchains; using $Toolchain because its toolchain directory exists under $rustupHome. $($_.Exception.Message)"
            return $true
        }

        Write-Warning "Could not list rustup toolchains and $Toolchain was not found under $rustupHome; leaving RUSTUP_TOOLCHAIN unchanged. $($_.Exception.Message)"
        return $false
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($rustupExit -ne 0) {
        if ($hasToolchainDirectory) {
            Write-Warning "Could not list rustup toolchains; using $Toolchain because its toolchain directory exists under $rustupHome. rustup exited with $rustupExit."
            return $true
        }

        Write-Warning "Could not list rustup toolchains and $Toolchain was not found under $rustupHome; leaving RUSTUP_TOOLCHAIN unchanged. rustup exited with $rustupExit."
        return $false
    }

    foreach ($entry in $toolchains) {
        if ($entry.ToString().Contains($Toolchain)) {
            return $true
        }
    }

    return $false
}

$useGnuToolchain = (Test-KairosWindowsHost) -and (Test-KairosRustupToolchainAvailable -Toolchain $gnuToolchain)
$env:RUSTUP_TOOLCHAIN = if ($useGnuToolchain) { $gnuToolchain } else { $oldRustupToolchain }
$env:PYTHONPATH = (Join-Path $repo "python/kairo_gym/src")
Push-Location $repo
try {
    $commands = @(
        @{ Name = "streaming check"; Command = @("cargo", "check", "--manifest-path", "crates/kairo-ecs-streaming/Cargo.toml", "--all-features", "--tests") },
        @{ Name = "ml check"; Command = @("cargo", "check", "--manifest-path", "crates/kairo-ecs-ml/Cargo.toml", "--all-features", "--tests") },
        @{ Name = "fmi check"; Command = @("cargo", "check", "--manifest-path", "crates/kairo-ecs-fmi/Cargo.toml", "--all-features", "--tests") },
        @{ Name = "debug check"; Command = @("cargo", "check", "--manifest-path", "crates/kairo-ecs-debug/Cargo.toml", "--tests") },
        @{ Name = "kairo_gym unittest"; Command = @("python", "-m", "unittest", "discover", "-s", "python/kairo_gym/tests") },
        @{ Name = "cloud offline validator"; Command = @("python", "cloud/validate_cloud_hpc.py") },
        @{ Name = "time travel demo validator"; Command = @("node", "website/time-travel-demo/validate-demo.mjs") }
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
        @{ Path = "docs/streaming/architecture.md"; Needles = @("contract test doubles only", "not Kafka, NATS, WebSocket") },
        @{ Path = "docs/streaming/broker-setup.md"; Needles = @("Tutorial: local contract smoke", "Evidence boundary") },
        @{ Path = "docs/ml/architecture.md"; Needles = @("contract double", "does not load or execute a real ONNX graph") },
        @{ Path = "docs/ml/surrogate-authoring.md"; Needles = @("Tutorial: dependency-free surrogate scaffold", "Evidence boundary") },
        @{ Path = "docs/fmi-digital-twin/import-guide.md"; Needles = @("does not perform XSD validation", "still needs the dynamic loader implementation", "Tutorial: unpacked FMU preflight", "Evidence boundary") },
        @{ Path = "docs/cloud-hpc/checkpoint-spot-policy.md"; Needles = @("does not prove", "Live provider validation", "Tutorial: offline cloud/HPC smoke", "Evidence boundary") },
        @{ Path = "docs/debugging/trace-format.md"; Needles = @("offline line encoding", "Arrow IPC serialization remains") },
        @{ Path = "docs/debugging/cli-reference.md"; Needles = @("Tutorial: local trace smoke", "Evidence boundary") }
    )

    foreach ($entry in $docs) {
        $text = Get-Content -LiteralPath $entry.Path -Raw
        foreach ($needle in $entry.Needles) {
            if ($text -notlike "*$needle*") {
                throw "$($entry.Path) is missing bounded-claim text: $needle"
            }
        }
    }

    $invalidExperiment = Join-Path $repo "k8s/operator/invalid-experiment.validation.json"
    $invalidExperimentJson = @{
        kind = "KairoECSExperiment"
        spec = @{
            image = "kairo-ecs-cli:dev"
            parallelism = 0
            storage = @{
                backend = "filesystem"
                path = "/tmp/out"
            }
        }
    } | ConvertTo-Json -Depth 8
    $utf8NoBom = New-Object System.Text.UTF8Encoding -ArgumentList $false
    [System.IO.File]::WriteAllText($invalidExperiment, $invalidExperimentJson, $utf8NoBom)
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & python "k8s/operator/kairoecs_operator.py" --experiment $invalidExperiment
    $operatorExit = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    Remove-Item -LiteralPath $invalidExperiment -Force
    if ($operatorExit -eq 0) {
        throw "Kubernetes operator validation did not reject invalid parallelism"
    }

    if (-not $SkipCargoTests) {
        Write-Host "==> cargo test probes"
        & cargo test --manifest-path crates/kairo-ecs-streaming/Cargo.toml --no-default-features
        if ($LASTEXITCODE -ne 0) { throw "streaming cargo test failed" }
        & cargo test --manifest-path crates/kairo-ecs-ml/Cargo.toml --no-default-features
        if ($LASTEXITCODE -ne 0) { throw "ml cargo test failed" }
    }

    Write-Host "Track 36-40 offline validation passed."
}
finally {
    Pop-Location
    $env:PYTHONPATH = $oldPythonPath
    $env:RUSTUP_TOOLCHAIN = $oldRustupToolchain
}
