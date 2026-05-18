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

Push-Location $repoRoot
try {
    Invoke-NativeChecked -Label "PDES no-default-features check" -Command @("cargo", "check", "--manifest-path", $crateManifest, "--no-default-features")
    Invoke-NativeChecked -Label "PDES tests compile check" -Command @("cargo", "check", "--manifest-path", $crateManifest, "--features", "pdes", "--tests")

    Assert-Contains -Path "docs\pdes\validation-evidence.md" -Needle "does not claim scheduler integration or real speedup" -Label "evidence boundary"
    Assert-Contains -Path "docs\pdes\event-exchange-protocol.md" -Needle "Stale null messages are non-regressive" -Label "stale null-message boundary"
    Assert-Contains -Path "docs\pdes\event-exchange-protocol.md" -Needle 'embedded `source_lp` is known' -Label "strict source validation boundary"
    Assert-Contains -Path "conductor\quality-gates.md" -Needle "non-regressive stale/null safe-time handling" -Label "quality gate stale/null safe-time text"
    Assert-Contains -Path "docs\pdes\benchmark-results.md" -Needle "Local deterministic scaling smoke evidence now exists" -Label "local scaling-smoke evidence"
    Assert-Contains -Path "docs\pdes\benchmark-results.md" -Needle "No hardware-speedup or hardware-parity claim is made in this slice." -Label "hardware parity boundary"
    Assert-Contains -Path "benches\pdes\README.md" -Needle "Benchmark-smoke coverage is complete for 4/8/16/32 LP logical configurations" -Label "benchmark-smoke completion boundary"
    Assert-Contains -Path "docs\pdes\time-warp-spike.md" -Needle "Recommendation: keep Track 34 on conservative CMB scheduling" -Label "time warp spike boundary"

    if ($RunTests) {
        Invoke-NativeChecked -Label "PDES runtime tests" -Command @("rustup", "run", "stable-x86_64-pc-windows-gnu", "cargo", "test", "--manifest-path", $crateManifest, "--features", "pdes")
    }

    Write-Host "Track 34 validator passed."
}
finally {
    Pop-Location
}
