param()

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Results = New-Object System.Collections.Generic.List[object]

function Add-Result {
    param(
        [string]$Name,
        [string]$Status,
        [string]$Detail = ""
    )

    $script:Results.Add([PSCustomObject]@{
        Name   = $Name
        Status = $Status
        Detail = $Detail
    })
}

function Invoke-RequiredCommand {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    Write-Host "Running $Name..." -ForegroundColor Cyan
    & $Command
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode"
    }

    Add-Result -Name $Name -Status "passed"
    Write-Host "$Name passed" -ForegroundColor Green
}

function Invoke-OptionalCommand {
    param(
        [string]$Name,
        [string]$Executable,
        [scriptblock]$Command
    )

    $tool = Get-Command -Name $Executable -ErrorAction SilentlyContinue
    if (-not $tool) {
        Add-Result -Name $Name -Status "skipped" -Detail "$Executable not installed"
        Write-Host "$Name skipped: $Executable not installed" -ForegroundColor Yellow
        return
    }

    Write-Host "Running $Name..." -ForegroundColor Cyan
    & $Command
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode"
    }

    Add-Result -Name $Name -Status "passed"
    Write-Host "$Name passed" -ForegroundColor Green
}

Push-Location $RepoRoot
try {
    Invoke-RequiredCommand -Name "Track 13 metadata validator" -Command {
        node scripts/validation/validate-track13-metadata.mjs
    }

    Invoke-RequiredCommand -Name "cargo metadata" -Command {
        cargo metadata --no-deps --format-version 1 | Out-Null
    }

    Invoke-OptionalCommand -Name "cargo deny advisories and sources" -Executable "cargo-deny" -Command {
        cargo deny check advisories sources
    }

    Invoke-OptionalCommand -Name "cargo audit" -Executable "cargo-audit" -Command {
        cargo audit
    }

    Write-Host ""
    Write-Host "Track 13 supply-chain gate summary:" -ForegroundColor Cyan
    foreach ($result in $Results) {
        if ($result.Detail) {
            Write-Host "  $($result.Name): $($result.Status) ($($result.Detail))"
        } else {
            Write-Host "  $($result.Name): $($result.Status)"
        }
    }
}
finally {
    Pop-Location
}
