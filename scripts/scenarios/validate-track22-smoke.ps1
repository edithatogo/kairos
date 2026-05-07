param(
    [string]$IndexPath = "scenarios/manifest-index.json"
)

$ErrorActionPreference = "Stop"

function Read-Json([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing file: $Path"
    }

    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Read-FlatToml([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing file: $Path"
    }

    $fields = @{}
    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if ($trimmed.Length -eq 0 -or $trimmed.StartsWith("#") -or $trimmed.StartsWith("[")) {
            continue
        }

        $parts = $trimmed.Split("=", 2)
        if ($parts.Count -ne 2) {
            continue
        }

        $key = $parts[0].Trim()
        $value = $parts[1].Trim().Trim('"').Trim("'").Trim()
        $fields[$key] = $value
    }

    $fields
}

function Require-Equal($Name, $Actual, $Expected) {
    if ($Actual -ne $Expected) {
        throw "$Name mismatch: actual=[$Actual] expected=[$Expected]"
    }
}

function Assert-Contains([string]$Path, [string]$Pattern, [string]$Description) {
    if (-not (Select-String -LiteralPath $Path -Pattern $Pattern -Quiet)) {
        throw "Missing $Description in $Path"
    }
}

function Join-Order($Values) {
    ($Values | ForEach-Object { [string]$_ }) -join ","
}

$index = Read-Json $IndexPath
Require-Equal "index schema_version" $index.schema_version "kairoecs.scenario-index.v1"

if (-not (Test-Path -LiteralPath 'docs/cli/kairo-ecs-cli.md')) {
    throw "Missing required file: docs/cli/kairo-ecs-cli.md"
}
Assert-Contains 'docs/cli/kairo-ecs-cli.md' 'validate-scenario' 'validate-scenario docs'
Assert-Contains 'docs/cli/kairo-ecs-cli.md' 'replay --scenario' 'replay docs'
Assert-Contains 'docs/cli/kairo-ecs-cli.md' 'resume-plan' 'resume-plan docs'
Assert-Contains 'website/docs-link-manifest.json' 'docs/cli/kairo-ecs-cli.md' 'docs site CLI nav link'
Assert-Contains 'website/src/index.md' 'docs/cli/kairo-ecs-cli.md' 'docs home CLI link'

if (-not $index.scenarios -or $index.scenarios.Count -eq 0) {
    throw "Scenario index contains no scenarios"
}

$checked = @()

foreach ($scenarioRef in $index.scenarios) {
    $scenario = Read-FlatToml $scenarioRef.scenario_manifest
    $seed = Read-FlatToml $scenarioRef.seed_manifest
    $conformance = Read-Json $scenarioRef.conformance_manifest

    Require-Equal "scenario_id" $scenario["scenario_id"] $scenarioRef.scenario_id
    Require-Equal "seed scenario_id" $seed["scenario_id"] $scenarioRef.scenario_id
    Require-Equal "scenario schema_version" $scenario["schema_version"] "kairoecs.scenario.v1"
    Require-Equal "seed schema_version" $seed["schema_version"] "kairoecs.seed.v1"
    Require-Equal "base_seed" $scenario["base_seed"] $seed["base_seed"]
    Require-Equal "fixture_id" $scenario["fixture_id"] $seed["fixture_id"]
    Require-Equal "execution fixture" $scenario["fixture_id"] $scenarioRef.execution_fixture_id

    $replayEntry = $conformance.fixtures | Where-Object { $_.id -eq $scenarioRef.replay_fixture_id }
    if (-not $replayEntry) {
        throw "Missing replay fixture in conformance manifest: $($scenarioRef.replay_fixture_id)"
    }
    Require-Equal "replay fixture status" $replayEntry.status "ready"

    $executionEntry = $conformance.fixtures | Where-Object { $_.id -eq $scenarioRef.execution_fixture_id }
    if (-not $executionEntry) {
        throw "Missing execution fixture in conformance manifest: $($scenarioRef.execution_fixture_id)"
    }
    Require-Equal "execution fixture status" $executionEntry.status "ready"

    $fixtureRoot = if ($conformance.root) { $conformance.root } else { Split-Path -Parent $scenarioRef.conformance_manifest }
    $replayPath = Join-Path $fixtureRoot $replayEntry.source
    $executionPath = Join-Path $fixtureRoot $executionEntry.source
    $replay = Read-Json $replayPath
    $execution = Read-Json $executionPath

    Require-Equal "replay scenario_id" $replay.scenario_id $scenarioRef.scenario_id
    Require-Equal "replay scenario_manifest" $replay.scenario_manifest $scenarioRef.scenario_manifest
    Require-Equal "replay seed_manifest" $replay.seed_manifest $scenarioRef.seed_manifest
    Require-Equal "replay execution fixture" $replay.replay_fixture_id $scenarioRef.execution_fixture_id
    Require-Equal "replay comparison_basis" $replay.comparison_basis $scenarioRef.comparison_basis
    Require-Equal "replay expected_summary_hash" $replay.expected_summary_hash $scenarioRef.expected_summary_hash

    $expectedOrder = Join-Order $scenarioRef.expected_kind_order
    Require-Equal "scenario expected_kind_order" $scenario["expected_kind_order"].Replace(" ", "") $expectedOrder
    Require-Equal "replay expected_kind_order" (Join-Order $replay.expected_kind_order) $expectedOrder

    $observedOrder = $execution.events |
        Sort-Object @{ Expression = "at_ticks"; Ascending = $true },
                    @{ Expression = "priority"; Ascending = $true },
                    @{ Expression = "sequence"; Ascending = $true } |
        ForEach-Object { $_.kind }
    Require-Equal "execution expected_kind_order" (Join-Order $observedOrder) $expectedOrder
    Require-Equal "fixture expected_kind_order" (Join-Order $execution.expected_kind_order) $expectedOrder

    foreach ($requiredOutput in $replay.required_outputs) {
        if ($scenarioRef.expected_outputs -notcontains $requiredOutput) {
            throw "Scenario index missing required output: $requiredOutput"
        }
    }

    $checked += [pscustomobject]@{
        scenario_id = $scenarioRef.scenario_id
        replay_fixture_id = $scenarioRef.replay_fixture_id
        execution_fixture_id = $scenarioRef.execution_fixture_id
        expected_kind_order = $expectedOrder
        expected_summary_hash = $scenarioRef.expected_summary_hash
    }
}

@{
    status = "ok"
    checked = $checked
    cli_docs = "docs/cli/kairo-ecs-cli.md"
} | ConvertTo-Json -Depth 5
