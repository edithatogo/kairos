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
    $cargoTools = @(
        @{ Name = "just"; Version = "1.42.4" },
        @{ Name = "cargo-nextest"; Version = "0.9.100" },
        @{ Name = "cargo-vet"; Version = "0.10.0" },
        @{ Name = "cargo-deny"; Version = "0.18.5" },
        @{ Name = "cargo-audit"; Version = "0.21.2" },
        @{ Name = "cargo-llvm-cov"; Version = "0.6.18" }
    )
    foreach ($tool in $cargoTools) {
        $name = $tool.Name
        if (Test-Command $name) {
            Write-Host "$name already on PATH"
            continue
        }
        Invoke-Optional "Install $name" {
            cargo install $name --version $tool.Version --locked
        }
    }
}

if (-not $SkipPython) {
    Invoke-Optional "Upgrade pip" {
        python -m pip install -U pip
    }
    Invoke-Optional "Install Python development packages" {
        python -m pip install -U maturin==1.9.6 pytest==8.3.5 hypothesis==6.131.0 ruff==0.11.13 pyarrow==24.0.0
    }
}

if (-not $SkipNpm) {
    Invoke-Optional "Install docs dependencies" {
        npm --prefix website ci
    }
}

Write-Host ""
Write-Host "KairoECS Windows bootstrap complete. Run: just dev-validate"
