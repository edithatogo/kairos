$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$setupPath = Join-Path $repoRoot "scripts\validate_conductor_setup.ps1"
$tokens = $null
$parseErrors = $null
$setupAst = [System.Management.Automation.Language.Parser]::ParseFile(
    $setupPath,
    [ref]$tokens,
    [ref]$parseErrors
)
if ($parseErrors.Count -gt 0) {
    throw "Setup validator has PowerShell parse errors: $($parseErrors -join '; ')"
}

foreach ($functionName in @("Test-RustupToolchainInstalled", "Invoke-CargoWorkspaceTests", "Invoke-SetupValidatorScript")) {
    $definitions = @($setupAst.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -ceq $functionName
    }, $true))
    if ($definitions.Count -ne 1) {
        throw "Expected one production definition for $functionName, found $($definitions.Count)"
    }
    Invoke-Expression $definitions[0].Extent.Text
}

$script:MockState = $null
function Test-WindowsHost { return [bool]$script:MockState.Windows }

function rustup {
    $arguments = @($args)
    $script:MockState.RustupCalls += ,($arguments -join " ")
    $global:LASTEXITCODE = 0
    if ($arguments.Count -ge 2 -and $arguments[0] -eq "toolchain" -and $arguments[1] -eq "list") {
        return $script:MockState.Installed
    }
    if ($arguments.Count -eq 1 -and $arguments[0] -eq "show") {
        return $script:MockState.HostOutput
    }
    if ($arguments.Count -eq 4 -and $arguments[0] -eq "which" -and $arguments[1] -eq "--toolchain") {
        return $arguments[3]
    }
    throw "Unexpected rustup mock invocation: $($arguments -join ' ')"
}

function cargo {
    $arguments = @($args)
    if ($arguments.Count -eq 1 -and $arguments[0] -eq "--version") {
        $global:LASTEXITCODE = 0
        return "cargo $($script:MockState.CargoVersion) (mock)"
    }
    if ($arguments.Count -ge 1 -and $arguments[0] -eq "test") {
        $script:MockState.CargoTestCalls++
        $script:MockState.CargoTestArgs = $arguments
        $script:MockState.CargoEnvironment = @{
            RUSTUP_TOOLCHAIN = [Environment]::GetEnvironmentVariable("RUSTUP_TOOLCHAIN", "Process")
            RUSTC = [Environment]::GetEnvironmentVariable("RUSTC", "Process")
            RUSTDOC = [Environment]::GetEnvironmentVariable("RUSTDOC", "Process")
            RUSTC_WRAPPER = [Environment]::GetEnvironmentVariable("RUSTC_WRAPPER", "Process")
            RUSTC_WORKSPACE_WRAPPER = [Environment]::GetEnvironmentVariable("RUSTC_WORKSPACE_WRAPPER", "Process")
        }
        $global:LASTEXITCODE = $script:MockState.CargoTestExit
        return
    }
    throw "Unexpected cargo mock invocation: $($arguments -join ' ')"
}

function rustc {
    $global:LASTEXITCODE = 0
    return "rustc $($script:MockState.RustcVersion) (mock)"
}

function rustdoc {
    $global:LASTEXITCODE = 0
    return "rustdoc $($script:MockState.RustdocVersion) (mock)"
}

function New-MockState {
    param(
        [bool]$Windows = $false,
        [string]$HostTriple = "aarch64-apple-darwin",
        [string[]]$Installed = @("1.99.0-aarch64-apple-darwin"),
        [string]$CargoVersion = "1.99.0",
        [string]$RustcVersion = "1.99.0",
        [string]$RustdocVersion = "1.99.0",
        [int]$CargoTestExit = 0
    )
    return [pscustomobject]@{
        Windows = $Windows
        Host = $HostTriple
        HostOutput = @("Default host: $HostTriple", "rustup home: /mock/.rustup")
        Installed = $Installed
        CargoVersion = $CargoVersion
        RustcVersion = $RustcVersion
        RustdocVersion = $RustdocVersion
        CargoTestExit = $CargoTestExit
        CargoTestCalls = 0
        CargoTestArgs = @()
        CargoEnvironment = @{}
        RustupCalls = @()
    }
}

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -is [array] -or $Expected -is [array]) {
        if ((@($Actual) -join "`n") -cne (@($Expected) -join "`n")) { throw $Message }
    } elseif ($Actual -cne $Expected) {
        throw "$Message (actual '$Actual', expected '$Expected')"
    }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    $threw = $false
    try { & $Action } catch { $threw = $true }
    if (-not $threw) { throw $Message }
}

function Set-TestEnvironment {
    $env:RUSTUP_TOOLCHAIN = "caller-toolchain"
    $env:RUSTC = "caller-rustc"
    $env:RUSTDOC = "caller-rustdoc"
    $env:RUSTC_WRAPPER = "caller-wrapper"
    $env:RUSTC_WORKSPACE_WRAPPER = "caller-workspace-wrapper"
}

function Assert-TestEnvironmentRestored {
    Assert-Equal $env:RUSTUP_TOOLCHAIN "caller-toolchain" "RUSTUP_TOOLCHAIN was not restored"
    Assert-Equal $env:RUSTC "caller-rustc" "RUSTC was not restored"
    Assert-Equal $env:RUSTDOC "caller-rustdoc" "RUSTDOC was not restored"
    Assert-Equal $env:RUSTC_WRAPPER "caller-wrapper" "RUSTC_WRAPPER was not restored"
    Assert-Equal $env:RUSTC_WORKSPACE_WRAPPER "caller-workspace-wrapper" "RUSTC_WORKSPACE_WRAPPER was not restored"
}

# The AST-extracted production functions must select the exact native host toolchain.
$script:MockState = New-MockState
Set-TestEnvironment
Invoke-CargoWorkspaceTests
Assert-Equal $script:MockState.CargoTestCalls 1 "Native toolchain did not run Cargo exactly once"
Assert-True ($script:MockState.RustupCalls -contains "show") "Native branch did not query rustup show"
Assert-True ($script:MockState.RustupCalls -contains "which --toolchain 1.99.0-aarch64-apple-darwin cargo") "Native branch did not select the host's canonical 1.99.0 toolchain"
Assert-Equal (@($script:MockState.CargoTestArgs) -join " ") "test --workspace --locked" "Cargo did not receive the locked workspace command"
Assert-Equal $script:MockState.CargoEnvironment.RUSTUP_TOOLCHAIN "1.99.0-aarch64-apple-darwin" "Cargo did not receive the exact native toolchain"
Assert-Equal $script:MockState.CargoEnvironment.RUSTC "rustc" "Cargo did not receive the rustup-resolved rustc"
Assert-Equal $script:MockState.CargoEnvironment.RUSTDOC "rustdoc" "Cargo did not receive the rustup-resolved rustdoc"
Assert-Equal $script:MockState.CargoEnvironment.RUSTC_WRAPPER "" "RUSTC_WRAPPER was not cleared during Cargo"
Assert-Equal $script:MockState.CargoEnvironment.RUSTC_WORKSPACE_WRAPPER "" "RUSTC_WORKSPACE_WRAPPER was not cleared during Cargo"
Assert-TestEnvironmentRestored

# Malformed rustup show output must fail before invoking Cargo or changing the caller environment.
$script:MockState = New-MockState
$script:MockState.HostOutput = @("rustup home: /mock/.rustup", "installed toolchains", "stable-aarch64-apple-darwin")
Set-TestEnvironment
Assert-Throws { Invoke-CargoWorkspaceTests } "Missing Default host line did not fail closed"
Assert-Equal $script:MockState.CargoTestCalls 0 "Cargo ran without a validated rustup host"
Assert-TestEnvironmentRestored

$script:MockState = New-MockState
$script:MockState.HostOutput += "Default host: x86_64-unknown-linux-gnu"
Set-TestEnvironment
Assert-Throws { Invoke-CargoWorkspaceTests } "Duplicate Default host lines did not fail closed"
Assert-Equal $script:MockState.CargoTestCalls 0 "Cargo ran with ambiguous rustup host output"
Assert-TestEnvironmentRestored

# Windows must use GNU when present and fail closed instead of using an ambient/native alias.
$script:MockState = New-MockState -Windows $true -Installed @("1.99.0-aarch64-apple-darwin", "1.99.0-x86_64-pc-windows-gnu")
Set-TestEnvironment
Invoke-CargoWorkspaceTests
Assert-True ($script:MockState.RustupCalls -contains "which --toolchain 1.99.0-x86_64-pc-windows-gnu cargo") "Windows did not prefer the canonical GNU toolchain"
Assert-TestEnvironmentRestored

$script:MockState = New-MockState -Windows $true -Installed @("1.99.0-aarch64-apple-darwin")
Set-TestEnvironment
Assert-Throws { Invoke-CargoWorkspaceTests } "Missing Windows GNU toolchain did not fail closed"
Assert-Equal $script:MockState.CargoTestCalls 0 "Cargo ran after the canonical Windows toolchain was missing"
Assert-TestEnvironmentRestored

# A mismatched actual compiler version blocks Cargo before environment mutation or workspace execution.
$script:MockState = New-MockState -RustcVersion "1.98.9"
Set-TestEnvironment
Assert-Throws { Invoke-CargoWorkspaceTests } "Wrong rustc version did not fail closed"
Assert-Equal $script:MockState.CargoTestCalls 0 "Cargo ran with a non-1.99.0 compiler"
Assert-TestEnvironmentRestored

# Environment restoration is required even when the pinned cargo test command fails.
$script:MockState = New-MockState -CargoTestExit 23
Set-TestEnvironment
Assert-Throws { Invoke-CargoWorkspaceTests } "Cargo test failure was not surfaced"
Assert-Equal $script:MockState.CargoTestCalls 1 "Cargo test failure fixture did not reach the mocked test command"
Assert-TestEnvironmentRestored

function ConvertFrom-MiseSubset {
    param([string]$Path)
    $document = @{}
    $section = $null
    foreach ($sourceLine in Get-Content -LiteralPath $Path) {
        $line = ([string]$sourceLine -replace '\s+#.*$', '').Trim()
        if ($line.Length -eq 0) { continue }
        if ($line -match '^\[(?<section>[A-Za-z0-9_-]+)\]$') {
            $section = $Matches.section
            if ($document.ContainsKey($section)) { throw "Duplicate mise table $section" }
            $document[$section] = @{}
            continue
        }
        if ($null -eq $section -or $line -notmatch '^(?<key>"(?:[^"\\]|\\.)+"|[A-Za-z0-9_-]+)\s*=\s*(?<value>.+)$') {
            throw "Unsupported mise TOML syntax: $sourceLine"
        }
        $key = $Matches.key
        if ($key.StartsWith('"')) { $key = ConvertFrom-Json -InputObject $key }
        $valueText = $Matches.value.Trim()
        if ($valueText.StartsWith('"')) {
            $value = ConvertFrom-Json -InputObject $valueText
        } elseif ($valueText -match '^\[(?<items>.*)\]$') {
            $items = @()
            foreach ($item in ($Matches.items -split ',')) {
                $item = $item.Trim()
                if ($item.Length -gt 0) { $items += (ConvertFrom-Json -InputObject $item) }
            }
            $value = $items
        } else {
            throw "Unsupported mise TOML value: $sourceLine"
        }
        if ($document[$section].ContainsKey($key)) { throw "Duplicate mise key $section.$key" }
        $document[$section][$key] = $value
    }
    return $document
}

$mise = ConvertFrom-MiseSubset (Join-Path $repoRoot "mise.toml")
$expectedMise = @{
    tools = @{
        rust = "1.99.0"
        python = "3.14.7"
        node = "lts"
        go = "latest"
        julia = "latest"
        R = "latest"
        dotnet = "10.0"
        just = "latest"
        "ubi:dtolnay/cargo-nextest" = "latest"
        "ubi:mozilla/cargo-vet" = "latest"
    }
}
Assert-Equal $mise.Keys.Count $expectedMise.Keys.Count "mise table count changed"
foreach ($tableName in $expectedMise.Keys) {
    Assert-True $mise.ContainsKey($tableName) "mise table $tableName is missing"
    Assert-Equal $mise[$tableName].Keys.Count $expectedMise[$tableName].Keys.Count "mise keys changed in table $tableName"
    foreach ($key in $expectedMise[$tableName].Keys) {
        Assert-True $mise[$tableName].ContainsKey($key) "mise key $tableName.$key is missing"
        Assert-Equal $mise[$tableName][$key] $expectedMise[$tableName][$key] "mise value changed unexpectedly at $tableName.$key"
    }
}

$workflowPath = Join-Path $repoRoot ".github\workflows\validate-conductor.yml"
$workflow = Get-Content -LiteralPath $workflowPath -Raw
$setupIndex = $workflow.IndexOf("run: pwsh -NoProfile -File scripts/validate_conductor_setup.ps1", [StringComparison]::Ordinal)
$linuxInstall = "rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy"
$windowsInstall = "rustup toolchain install 1.99.0-x86_64-pc-windows-gnu --profile minimal --component rustfmt --component clippy"
$linuxIndex = $workflow.IndexOf($linuxInstall, [StringComparison]::Ordinal)
$windowsIndex = $workflow.IndexOf($windowsInstall, [StringComparison]::Ordinal)
Assert-True ($setupIndex -gt 0 -and $linuxIndex -ge 0 -and $linuxIndex -lt $setupIndex) "Workflow did not install exact host Rust 1.99.0 before setup validation"
Assert-True ($setupIndex -gt 0 -and $windowsIndex -ge 0 -and $windowsIndex -lt $setupIndex) "Workflow did not install exact Windows GNU Rust 1.99.0 before setup validation"

$childDirectory = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $childDirectory | Out-Null
try {
    $successChild = Join-Path $childDirectory "success.ps1"
    $failureChild = Join-Path $childDirectory "failure.ps1"
    Set-Content -LiteralPath $successChild -Value "exit 0" -NoNewline
    Set-Content -LiteralPath $failureChild -Value "exit 1" -NoNewline

    Invoke-SetupValidatorScript -Path $successChild
    Assert-Equal $global:LASTEXITCODE 0 "Successful child validator left a failing exit code"
    Assert-Throws { Invoke-SetupValidatorScript -Path $failureChild } "Child validator exit 1 did not fail the parent setup validator"
}
finally {
    Remove-Item -LiteralPath $childDirectory -Recurse -Force
}

Write-Host "Rust setup toolchain regression tests passed (mocked; no Rust process executed)."
