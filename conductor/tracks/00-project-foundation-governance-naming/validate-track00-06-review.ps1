Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
$trackRoot = Join-Path $repoRoot 'conductor\tracks'

$trackNames = @(
    '00-project-foundation-governance-naming',
    '01-heart-kairo-ecs-core-state',
    '02-bridge-kairo-ecs-ffi-uniffi-diplomat',
    '03-flow-des-trajectory-abm-behavior',
    '04-analyst-kairo-ecs-arrow',
    '05-window-kairo-ecs-viz',
    '06-python-binding-310-314'
)

$requiredTrackFiles = @(
    'spec.md',
    'plan.md',
    'agent-contract.md',
    'risk-register.md',
    'test-matrix.md',
    'handoff.md'
)

$requiredImplementationPaths = @(
    'crates\kairo-ecs-types\src\lib.rs',
    'crates\kairo-ecs-core\src\lib.rs',
    'crates\kairo-ecs-state\src\lib.rs',
    'crates\kairo-ecs-rng\src\lib.rs',
    'crates\kairo-ecs-ffi\src\lib.rs',
    'crates\kairo-ecs-uniffi\src\lib.rs',
    'crates\kairo-ecs-diplomat\src\lib.rs',
    'crates\kairo-ecs-des\src\lib.rs',
    'crates\kairo-ecs-abm\src\lib.rs',
    'crates\kairo-ecs-arrow\src\lib.rs',
    'crates\kairo-ecs-viz\src\lib.rs',
    'bindings\python\kairo_ecs\__init__.py',
    'include\kairo_ecs.h',
    'schemas\arrow\event_log_v1.schema.json',
    'examples\flow\README.md',
    'examples\telemetry\event_log_roundtrip.rs',
    'examples\viz\headless-snapshot\Cargo.toml',
    'website\docs\visualization\README.md'
)

$stalePhrases = @(
    'No code files were changed in this handoff pass.',
    'those package directories are still pending',
    'only the schema documentation scaffold exists today',
    'only the website docs scaffold exists today',
    'package crate is still pending'
)

$errors = New-Object System.Collections.Generic.List[string]

foreach ($trackName in $trackNames) {
    $trackPath = Join-Path $trackRoot $trackName
    if (-not (Test-Path -LiteralPath $trackPath -PathType Container)) {
        $errors.Add("Missing track directory: conductor/tracks/$trackName")
        continue
    }

    foreach ($fileName in $requiredTrackFiles) {
        $path = Join-Path $trackPath $fileName
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            $errors.Add("Missing track file: conductor/tracks/$trackName/$fileName")
        }
    }
}

foreach ($relativePath in $requiredImplementationPaths) {
    $path = Join-Path $repoRoot $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $errors.Add("Missing implementation path: $relativePath")
    }
}

$markdownFiles = foreach ($trackName in $trackNames) {
    Get-ChildItem -LiteralPath (Join-Path $trackRoot $trackName) -Filter '*.md' -File
}

foreach ($phrase in $stalePhrases) {
    $matches = $markdownFiles | Select-String -SimpleMatch -Pattern $phrase
    foreach ($match in $matches) {
        $relativePath = Resolve-Path -LiteralPath $match.Path -Relative
        $errors.Add("Stale phrase '$phrase' in ${relativePath}:$($match.LineNumber)")
    }
}

if ($errors.Count -gt 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output "Track 00-06 review validator passed."
