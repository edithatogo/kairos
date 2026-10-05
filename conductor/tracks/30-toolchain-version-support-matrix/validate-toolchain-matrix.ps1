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

function ConvertFrom-RustcVersionText {
    param([string]$Text)
    if ($Text -match '^rustc\s+([0-9]+\.[0-9]+\.[0-9]+)(?:\s|$)') { return $Matches[1] }
    throw "Could not parse exact stable rustc version from: $Text"
}

function Test-ExactRustVersion {
    param([string]$Actual, [string]$Expected)
    return $Actual -ceq $Expected
}

function Assert-RustVersionParserContract {
    $accepted = ConvertFrom-RustcVersionText 'rustc 1.99.0 (abc123 2026-10-05)'
    if (-not (Test-ExactRustVersion -Actual $accepted -Expected '1.99.0')) { throw "Rust parser accepted the wrong stable version: $accepted" }
    foreach ($text in @('rustc 1.99.0-beta.1 (abc123 2026-10-05)', 'rustc 1.99.0-nightly (abc123 2026-10-05)', 'rustc 1.99 (abc123 2026-10-05)', 'rustc 1.99.0.1 (abc123 2026-10-05)')) {
        $rejected = $false
        try { [void](ConvertFrom-RustcVersionText $text) } catch { $rejected = $true }
        if (-not $rejected) { throw "Rust parser incorrectly accepted: $text" }
    }
    foreach ($version in @('1.98.9', '1.100.0')) {
        if (Test-ExactRustVersion -Actual $version -Expected '1.99.0') { throw "Exact Rust comparison incorrectly accepted $version" }
    }
}

Assert-RustVersionParserContract

function Get-InstalledVersion {
    param([string]$Name)

    switch ($Name) {
        "rust" {
            $text = Get-CommandOutput @("rustc", "--version")
            return ConvertFrom-RustcVersionText $text
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
$validatorText = Get-Content -LiteralPath $PSCommandPath -Raw
$bindingWorkflowPath = Join-Path $repoRoot ".github\workflows\ci-bindings.yml"
$bindingWorkflow = Get-Content -LiteralPath $bindingWorkflowPath -Raw
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

foreach ($token in @("conductor/toolchain-matrix.md", "bindings/python/pyproject.toml", "bindings/r/DESCRIPTION", "bindings/julia/Project.toml", "bindings/typescript/package.json", "bindings/csharp/global.json", "bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj", "bindings/go/go.mod")) {
    Assert-Contains -Text $workflow -Needle $token -Label "workflow trigger path"
}

$laneExpectations = @(
    @{ Matrix = 'Rust `1.99.0` only'; Workflow = 'expected-version: "1.99.0"' }
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

function Assert-NoSupersededRustWorkflowSelectors {
    param([string]$Text)
    foreach ($superseded in @('channel: stable', 'channel: beta', 'expected-prefix: "1.98"', 'expected-prefix: "1."')) {
        if ($Text.Contains($superseded)) { throw "Rust lane contains superseded alias or prefix selector: $superseded" }
    }
}
function Assert-NoSupersededRustMatrixClaims {
    param([string]$Text)
    foreach ($superseded in @('MSRV `1.76`', 'Rust `1.98.x` stable as of', 'Rust `beta` advisory lane')) {
        if ($Text.Contains($superseded)) { throw "Current Rust matrix contains superseded support policy: $superseded" }
    }
}
Assert-NoSupersededRustWorkflowSelectors -Text $workflow
Assert-NoSupersededRustMatrixClaims -Text $matrix
foreach ($token in @('function ConvertFrom-RustcVersionText', 'function Test-ExactRustVersion', 'Assert-RustVersionParserContract', 'rustc 1.99.0-beta.1', 'rustc 1.99.0-nightly')) {
    Assert-Contains -Text $validatorText -Needle $token -Label "exact Rust 1.99.0 parser contract"
}
Assert-Contains -Text $workflow -Needle '-ExpectedVersion $env:EXPECTED_VERSION' -Label "exact Rust 1.99.0 workflow wiring"
foreach ($superseded in @('channel: stable', 'channel: beta', 'expected-prefix: "1.98"', 'expected-prefix: "1."')) {
    $rejected = $false
    try { Assert-NoSupersededRustWorkflowSelectors -Text ($workflow + "`n" + $superseded) } catch { $rejected = $true }
    if (-not $rejected) { throw "Rust workflow negative self-check did not reject: $superseded" }
}
foreach ($superseded in @('MSRV `1.76`', 'Rust `1.98.x` stable as of 2026-09-28', 'Rust `beta` advisory lane')) {
    $rejected = $false
    try { Assert-NoSupersededRustMatrixClaims -Text ($matrix + "`n" + $superseded) } catch { $rejected = $true }
    if (-not $rejected) { throw "Rust matrix negative self-check did not reject: $superseded" }
}

Assert-Contains -Text $bindingWorkflow -Needle 'go-version: ''stable''' -Label "Go binding test toolchain"
Assert-Contains -Text $bindingWorkflow -Needle 'go vet ./...' -Label "Go binding vet coverage"
Assert-Contains -Text $bindingWorkflow -Needle 'go test ./...' -Label "Go binding test coverage"

if ($typescriptPackage.engines.node -ne ">=22 <25") {
    throw "TypeScript package engines.node must stay aligned with the Node 22/24 production support floor."
}

if ($CheckInstalled) {
    if ([string]::IsNullOrWhiteSpace($Ecosystem)) {
        throw "-CheckInstalled requires -Ecosystem and an expected version."
    }

    $actual = Get-InstalledVersion -Name $Ecosystem.ToLowerInvariant()
    if ($Ecosystem -eq "rust") {
        if ([string]::IsNullOrWhiteSpace($ExpectedVersion) -or -not [string]::IsNullOrWhiteSpace($ExpectedPrefix)) { throw "Rust checks require -ExpectedVersion and reject -ExpectedPrefix." }
        if ($actual -ne $ExpectedVersion) { throw "Rust version mismatch: expected exact $ExpectedVersion, got $actual." }
        Write-Host "Rust installed version check passed: exact $actual"
    } else {
        if ([string]::IsNullOrWhiteSpace($ExpectedPrefix) -or -not [string]::IsNullOrWhiteSpace($ExpectedVersion)) { throw "-CheckInstalled requires -ExpectedPrefix for $Ecosystem and rejects -ExpectedVersion." }
        if (-not $actual.StartsWith($ExpectedPrefix)) { throw "$Ecosystem version mismatch: expected prefix $ExpectedPrefix, got $actual." }
        Write-Host "$Ecosystem installed version check passed: $actual matches $ExpectedPrefix"
    }
}

Write-Host "Track 30 toolchain matrix validation passed."
