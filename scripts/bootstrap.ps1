param(
    [switch]$CheckOnly,
    [switch]$SkipPython,
    [switch]$SkipNpm,
    [switch]$SkipCargoInstalls
)

$ErrorActionPreference = "Stop"

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Optional {
    param(
        [string]$Label,
        [scriptblock]$Command
    )

    Write-Host "==> $Label"
    try {
        & $Command
    } catch {
        Write-Warning "$Label failed: $($_.Exception.Message)"
    }
}

Write-Host "KairoECS Windows bootstrap"

$required = @("rustup", "cargo", "python", "npm", "node", "pwsh", "git")
$missing = @($required | Where-Object { -not (Test-Command $_) })
if ($missing.Count -gt 0) {
    throw "Missing required bootstrap command(s): $($missing -join ', ')"
}

if ($CheckOnly) {
    Write-Host "Check-only mode: required bootstrap commands are present."
    if (-not (Test-Command "just")) {
        Write-Warning "just is not on PATH; run cargo install just --locked or use the npm/node fallback commands documented in docs/developer-experience/docs-workflow.md."
    }
    exit 0
}

Invoke-Optional "Install Rust formatter and linter components" {
    rustup component add rustfmt clippy
}

if (-not $SkipCargoInstalls) {
    foreach ($tool in @("just", "cargo-nextest", "cargo-vet", "cargo-deny", "cargo-audit", "cargo-llvm-cov")) {
        if (Test-Command $tool) {
            Write-Host "$tool already on PATH"
            continue
        }
        Invoke-Optional "Install $tool" {
            cargo install $tool --locked
        }
    }
}

if (-not $SkipPython) {
    Invoke-Optional "Upgrade pip" {
        python -m pip install -U pip
    }
    Invoke-Optional "Install Python development packages" {
        python -m pip install -U maturin pytest hypothesis ruff pyarrow
    }
}

if (-not $SkipNpm) {
    Invoke-Optional "Install docs dependencies" {
        npm --prefix website ci
    }
}

Write-Host ""
Write-Host "KairoECS Windows bootstrap complete. Run: just dev-validate"
