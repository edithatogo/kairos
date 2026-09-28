param(
    [string]$Ecosystem = "",
    [string]$ExpectedPrefix = "",
    [string]$ExpectedVersion = "",
    [switch]$CheckInstalled
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$matrixPath = Join-Path $repoRoot "conductor\toolchain-matrix.md"
$gatesPath = Join-Path $repoRoot "conductor\quality-gates.md"
$workflowPath = Join-Path $repoRoot ".github\workflows\toolchain-check.yml"

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Needle,
        [string]$Label
    )

    if (-not $Text.Contains($Needle)) {
        throw "Missing $Label`: $Needle"
    }
}

function Get-CommandOutput {
    param([string[]]$CommandLine)

    $output = & $CommandLine[0] @($CommandLine | Select-Object -Skip 1) 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed: $($CommandLine -join ' ')`n$output"
    }
    return ($output -join "`n").Trim()
}

function Get-InstalledVersion {
    param([string]$Name)

    switch ($Name) {
        "rust" {
            if (-not [string]::IsNullOrWhiteSpace($env:RUSTUP_TOOLCHAIN)) {
                $text = Get-CommandOutput @("rustup", "run", $env:RUSTUP_TOOLCHAIN, "rustc", "--version")
            } else {
                $text = Get-CommandOutput @("rustc", "--version")
            }
            if ($text -match "rustc\s+([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)") { return $Matches[1] }
            throw "Could not parse rustc version from: $text"
        }
        "python" {
            $text = Get-CommandOutput @("python", "--version")
            if ($text -match "Python\s+([0-9]+\.[0-9]+)") { return $Matches[1] }
            throw "Could not parse Python version from: $text"
        }
        "r" {
            $text = Get-CommandOutput @("Rscript", "-e", "cat(as.character(getRversion()))")
            if ($text -match "([0-9]+\.[0-9]+)") { return $Matches[1] }
            throw "Could not parse R version from: $text"
        }
        "julia" {
            $text = Get-CommandOutput @("julia", "-e", "print(VERSION)")
            if ($text -match "([0-9]+\.[0-9]+)") { return $Matches[1] }
            throw "Could not parse Julia version from: $text"
        }
        "node" {
            $text = Get-CommandOutput @("node", "--version")
            if ($text -match "v?([0-9]+)") { return $Matches[1] }
            throw "Could not parse Node version from: $text"
        }
        "dotnet" {
            $text = Get-CommandOutput @("dotnet", "--version")
            if ($text -match "([0-9]+\.[0-9]+)") { return $Matches[1] }
            throw "Could not parse .NET version from: $text"
        }
        "go" {
            $text = Get-CommandOutput @("go", "version")
            if ($text -match "go([0-9]+\.[0-9]+)") { return $Matches[1] }
            throw "Could not parse Go version from: $text"
        }
        default {
            throw "Unknown ecosystem for installed check: $Name"
        }
    }
}

$matrix = Get-Content -LiteralPath $matrixPath -Raw
$gates = Get-Content -LiteralPath $gatesPath -Raw
$workflow = Get-Content -LiteralPath $workflowPath -Raw
$toolchainPath = Join-Path $repoRoot "rust-toolchain.toml"
$toolchain = Get-Content -LiteralPath $toolchainPath -Raw
$templateToolchainPath = Join-Path $repoRoot "templates\rust-toolchain.toml"
$templateToolchain = Get-Content -LiteralPath $templateToolchainPath -Raw
$misePath = Join-Path $repoRoot "mise.toml"
$mise = Get-Content -LiteralPath $misePath -Raw
$packageDryRunPath = Join-Path $repoRoot ".github\workflows\package-dry-run.yml"
$packageDryRun = Get-Content -LiteralPath $packageDryRunPath -Raw
$typescriptPackagePath = Join-Path $repoRoot "bindings\typescript\package.json"
$typescriptPackage = Get-Content -LiteralPath $typescriptPackagePath -Raw | ConvertFrom-Json

$requiredRows = @(
    "| Rust core |",
    "| Python binding |",
    "| R binding |",
    "| Julia binding |",
    "| TypeScript/Wasm binding |",
    "| C# binding |",
    "| Go binding |"
)

foreach ($row in $requiredRows) {
    Assert-Contains -Text $matrix -Needle $row -Label "support matrix row"
}

Assert-Contains -Text $toolchain -Needle 'channel = "1.98.1"' -Label "pinned Rust developer toolchain"
Assert-Contains -Text $templateToolchain -Needle 'channel = "1.98.1"' -Label "pinned Rust template toolchain"
Assert-Contains -Text $mise -Needle 'rust = "1.98.1"' -Label "pinned mise Rust toolchain"

foreach ($token in @("Minimum supported version", "Latest/current supported version", "Deprecation horizon", "Linux x86_64", "macOS aarch64", "Windows x86_64")) {
    Assert-Contains -Text $matrix -Needle $token -Label "matrix column"
}

foreach ($token in @("CI-covered", "best-effort", "unsupported")) {
    Assert-Contains -Text $matrix -Needle $token -Label "support label"
}

foreach ($token in @("two KairoECS release cycles or six calendar months", "release notes", "binding README", "upstream vendor EOL", "version-drop-policy-check", "toolchain-matrix-current")) {
    Assert-Contains -Text $matrix -Needle $token -Label "version-drop policy"
}

foreach ($token in @('## Proposed Drops', '| Node.js | `20.x` |', '| Go | `1.24.x` package dry-run lane |', 'Earliest removal', 'Deprecated')) {
    Assert-Contains -Text $matrix -Needle $token -Label "proposed drop notice"
}

foreach ($token in @("toolchain-matrix-current", "version-drop-policy-check", "conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1")) {
    Assert-Contains -Text $gates -Needle $token -Label "quality gate definition"
}

foreach ($token in @("conductor/toolchain-matrix.md", "rust-toolchain.toml", "templates/rust-toolchain.toml", "mise.toml", "bindings/python/pyproject.toml", "bindings/r/DESCRIPTION", "bindings/julia/Project.toml", "bindings/typescript/package.json", "bindings/csharp/global.json", "bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj", "bindings/go/go.mod")) {
    Assert-Contains -Text $workflow -Needle $token -Label "workflow trigger path"
}

$laneExpectations = @(
    @{ Matrix = 'Rust `1.98.1` stable'; Workflow = 'expected-version: "1.98.1"' }
    @{ Matrix = 'Rust `1.99.0-beta.8`'; Workflow = 'expected-version: "1.99.0-beta.8"' }
    @{ Matrix = 'CPython `3.10`'; Workflow = 'python-version: ["3.10", "3.11", "3.12", "3.13", "3.14"]' }
    @{ Matrix = 'CPython `3.14.x`'; Workflow = 'python-version: ["3.10", "3.11", "3.12", "3.13", "3.14"]' }
    @{ Matrix = 'R `4.6.x`'; Workflow = 'expected-prefix: "4.6"' }
    @{ Matrix = 'Julia `1.10`'; Workflow = 'julia-version: ["1.10", "1.12"]' }
    @{ Matrix = 'Julia `1.12.x`'; Workflow = 'julia-version: ["1.10", "1.12"]' }
    @{ Matrix = 'Node `22`'; Workflow = 'node-version: ["22", "24"]' }
    @{ Matrix = 'Node `24`'; Workflow = 'node-version: ["22", "24"]' }
    @{ Matrix = '.NET SDK `10.0.x`'; Workflow = 'dotnet-version: "10.0.x"' }
    @{ Matrix = '.NET SDK `11.0.x`'; Workflow = 'dotnet-version: "11.0.x"' }
    @{ Matrix = 'Go `1.26.x`'; Workflow = 'go-version: "1.26.x"' }
    @{ Matrix = 'CI support floor `1.25`'; Workflow = 'go-version: "1.25.x"' }
)

foreach ($expectation in $laneExpectations) {
    Assert-Contains -Text $matrix -Needle $expectation.Matrix -Label "matrix lane"
    Assert-Contains -Text $workflow -Needle $expectation.Workflow -Label "workflow lane matching matrix"
}

foreach ($token in @('continue-on-error: ${{ matrix.advisory }}', 'RUSTUP_TOOLCHAIN: ${{ matrix.channel }}', 'cargo check --workspace --locked')) {
    Assert-Contains -Text $workflow -Needle $token -Label "exact advisory canary behavior"
}

Assert-Contains -Text $packageDryRun -Needle "go-version: '1.25.x'" -Label "package dry-run Go support floor"
if ($packageDryRun.Contains("go-version: '1.24'") -or $packageDryRun.Contains('go-version: "1.24"')) {
    throw "Package dry-run must not reintroduce deprecated Go 1.24."
}

if ($typescriptPackage.engines.node -ne ">=22 <25") {
    throw "TypeScript package engines.node must stay aligned with the Node 22/24 production support floor."
}

if ($CheckInstalled) {
    if ([string]::IsNullOrWhiteSpace($Ecosystem) -or ([string]::IsNullOrWhiteSpace($ExpectedPrefix) -and [string]::IsNullOrWhiteSpace($ExpectedVersion))) {
        throw "-CheckInstalled requires -Ecosystem and either -ExpectedPrefix or -ExpectedVersion."
    }

    $actual = Get-InstalledVersion -Name $Ecosystem.ToLowerInvariant()
    if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion)) {
        if ($actual -ne $ExpectedVersion) {
            throw "$Ecosystem version mismatch: expected exact version $ExpectedVersion, got $actual."
        }
        Write-Host "$Ecosystem installed version check passed: $actual matches exact version $ExpectedVersion"
    } elseif (-not $actual.StartsWith($ExpectedPrefix)) {
        throw "$Ecosystem version mismatch: expected prefix $ExpectedPrefix, got $actual."
    } else {
        Write-Host "$Ecosystem installed version check passed: $actual matches $ExpectedPrefix"
    }
}

Write-Host "Track 30 toolchain matrix validation passed."
