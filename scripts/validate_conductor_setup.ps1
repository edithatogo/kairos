param(
    [switch]$SkipCargo
)

$ErrorActionPreference = "Stop"

$requiredPaths = @(
    "conductor/product.md",
    "conductor/product-guidelines.md",
    "conductor/tech-stack.md",
    "conductor/workflow.md",
    "conductor/tracks.md",
    "conductor/tracks.yaml",
    "conductor/implementation-readiness.md",
    "docs/release/release-checklist.md",
    "conductor/package-catalog.md",
    "conductor/package-matrix.md",
    "conductor/gates/wave-progression-check.yml",
    "conductor/gates/dependency-closure-check.yml",
    "conductor/toolchain-matrix.md",
    "conductor/performance-thresholds.md",
    "Cargo.toml",
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "MAINTAINERS.md",
    "GOVERNANCE.md",
    ".github/CODEOWNERS",
    ".github/workflows/codeql.yml",
    ".github/workflows/secret-scan.yml",
    ".github/workflows/toolchain-check.yml",
    ".github/workflows/bench-regression.yml",
    "crates/kairo-ecs-types/Cargo.toml",
    "crates/kairo-ecs-core/Cargo.toml",
    "crates/kairo-ecs-state/Cargo.toml",
    "crates/kairo-ecs-rng/Cargo.toml",
    "conformance/fixtures/deterministic_ordering.json",
    "conformance/fixtures/cancellation.json",
    "conformance/fixtures/rng_replay.json",
    "website/package.json",
    "website/src/index.md",
    ".github/workflows/conformance.yml",
    "conductor/community-adoption.md",
    "conductor/verification-validation-uncertainty.md",
    "conductor/experiment-runner.md",
    "conductor/domain-model-zoo.md",
    "docs/trustworthy-simulation/replay-and-seeds.md",
    "docs/trustworthy-simulation/verification-validation-uncertainty.md"
)

$missing = @($requiredPaths | Where-Object { -not (Test-Path -LiteralPath $_) })
if ($missing.Count -gt 0) {
    throw "Missing required setup paths: $($missing -join ', ')"
}

$requiredTrackFiles = @(
    "spec.md",
    "plan.md",
    "agent-contract.md",
    "risk-register.md",
    "test-matrix.md",
    "handoff.md"
)

function Get-TrackIdsFromTracksYaml {
    param([string]$Path)

    $content = Get-Content -LiteralPath $Path -Raw
    $ids = [regex]::Matches($content, '(?m)^\s*-\s*id:\s*(\d+)\s*$') |
        ForEach-Object { "{0:D2}" -f [int]$_.Groups[1].Value }

    return @($ids | Sort-Object -Unique)
}

$expectedTrackIds = @(Get-TrackIdsFromTracksYaml -Path "conductor/tracks.yaml")
if ($expectedTrackIds.Count -eq 0) {
    throw "No track ids found in conductor/tracks.yaml"
}

$trackDirs = @(Get-ChildItem -LiteralPath "conductor/tracks" -Directory)
$actualTrackIds = @($trackDirs | ForEach-Object {
    if ($_.Name -match '^(\d+)-') {
        "{0:D2}" -f [int]$matches[1]
    } else {
        throw "Track directory name does not start with a numeric id: $($_.Name)"
    }
} | Sort-Object -Unique)

$missingTrackIds = @($expectedTrackIds | Where-Object { $actualTrackIds -notcontains $_ })
$extraTrackIds = @($actualTrackIds | Where-Object { $expectedTrackIds -notcontains $_ })
if ($missingTrackIds.Count -gt 0 -or $extraTrackIds.Count -gt 0) {
    throw "Track directory ids do not match conductor/tracks.yaml. Missing: $($missingTrackIds -join ', '); Extra: $($extraTrackIds -join ', ')"
}
if ($trackDirs.Count -ne $expectedTrackIds.Count) {
    throw "Expected $($expectedTrackIds.Count) track directories from conductor/tracks.yaml, found $($trackDirs.Count)"
}

foreach ($track in $trackDirs) {
    foreach ($file in $requiredTrackFiles) {
        $path = Join-Path $track.FullName $file
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing track artifact: $path"
        }
    }
}

$workflowFiles = @(Get-ChildItem -LiteralPath ".github/workflows" -Filter "*.yml")
$bootstrapAllowed = @(
    "ci-bindings.yml"
)
$placeholderPatterns = @(
    "not present yet",
    "placeholder",
    "Missing release checklist",
    "No workspace yet",
    "Wire this workflow",
    "skip.*core CI",
    "Add lychee"
)
foreach ($workflow in $workflowFiles) {
    $content = Get-Content -LiteralPath $workflow.FullName -Raw
    if (($bootstrapAllowed -notcontains $workflow.Name) -and ($placeholderPatterns | Where-Object { $content -match $_ })) {
        throw "Workflow still contains bootstrap placeholder text: $($workflow.Name)"
    }
}

$strictWorkflowFiles = @("ci-bindings.yml", "package-dry-run.yml", "benchmarks.yml", "benchmark-smoke.yml", "fuzzing.yml")
foreach ($workflowName in $strictWorkflowFiles) {
    $workflowPath = Join-Path ".github/workflows" $workflowName
    $content = Get-Content -LiteralPath $workflowPath -Raw
    if ($content -match "future surface; skipping") {
        throw "Workflow still quietly skips missing concrete surfaces: $workflowName"
    }
    if ($workflowName -eq "benchmarks.yml" -and $content -match "No benchmarks yet") {
        throw "Workflow still skips missing benchmark harnesses: $workflowName"
    }
    if ($workflowName -eq "fuzzing.yml" -and $content -match "No fuzz harness yet") {
        throw "Workflow still skips missing fuzz harnesses: $workflowName"
    }
    if ($workflowName -eq "fuzzing.yml" -and $content -match "\|\| true") {
        throw "Workflow still suppresses fuzz failures: $workflowName"
    }
    if ($workflowName -eq "ci-bindings.yml" -and $content -match "--if-present") {
        throw "Workflow still allows optional TypeScript scripts to be skipped: $workflowName"
    }
    if ($workflowName -eq "benchmarks.yml" -and $content -match "if-no-files-found:\s*ignore") {
        throw "Workflow still ignores missing benchmark artifacts: $workflowName"
    }
}

foreach ($fixture in Get-ChildItem -LiteralPath "conformance/fixtures" -Filter "*.json") {
    Get-Content -LiteralPath $fixture.FullName -Raw | ConvertFrom-Json | Out-Null
}

function Assert-Contains {
    param(
        [string]$Content,
        [string]$Needle,
        [string]$Label
    )

    if ($Content -notmatch [regex]::Escape($Needle)) {
        throw "$Label missing required text: $Needle"
    }
}

$deliveryChecklist = Get-Content -LiteralPath "conductor/delivery-readiness-checklist.md" -Raw
foreach ($row in @(
    "Comparative benchmarks and reproducibility guidance from Track 18 is green.",
    "Research software, citation, and archival guidance from Track 19 is green.",
    "OpenSSF, supply-chain trust, and institutional-readiness guidance from Track 20 is green.",
    "Community adoption, education, and ecosystem guidance from Track 17 is green.",
    "Verification, validation, and uncertainty guidance from Track 21 is green.",
    "Scenario runner and replay guidance from Track 22 is green.",
    "Starter-kit and model-zoo guidance from Track 23 is green.",
    "Playground and demo guidance from Track 24 is green.",
    "API design review and compatibility governance guidance from Track 25 is green.",
    "Interoperability standards review guidance from Track 26 is green.",
    "Wave manager controls from Track 29 are green.",
    "Toolchain matrix and version-drop policy from Track 30 are green.",
    "Performance regression guard from Track 31 is green or explicitly marked advisory."
)) {
    Assert-Contains -Content $deliveryChecklist -Needle $row -Label "delivery-readiness-checklist.md"
}

$releaseChecklist = Get-Content -LiteralPath "docs/release/release-checklist.md" -Raw
foreach ($row in @(
    "Naming/package availability confirmed.",
    "Conformance suite green.",
    "Benchmarks reviewed.",
    "Security scans reviewed.",
    "GitHub Pages build green.",
    "Registry dry-runs complete."
)) {
    Assert-Contains -Content $releaseChecklist -Needle $row -Label "docs/release/release-checklist.md"
}

$packageCatalog = Get-Content -LiteralPath "conductor/package-catalog.md" -Raw
foreach ($row in @(
    "Current checked-in package surfaces",
    "The workspace is real; the root meta crate and bridge crates remain future track outputs.",
    'Current package surface: `bindings/typescript/package.json` declares `@kairo-ecs/typescript` and a plain TypeScript test/typecheck loop.'
)) {
    Assert-Contains -Content $packageCatalog -Needle $row -Label "conductor/package-catalog.md"
}

$packageMatrix = Get-Content -LiteralPath "conductor/package-matrix.md" -Raw
foreach ($row in @(
    "Current checked-in package surfaces",
    'The checked-in workspace currently ships `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, and `kairo-ecs-rng`',
    "Current release posture"
)) {
    Assert-Contains -Content $packageMatrix -Needle $row -Label "conductor/package-matrix.md"
}

$conformanceWorkflow = Get-Content -LiteralPath ".github/workflows/conformance.yml" -Raw
foreach ($row in @(
    "Conformance Fixtures",
    "deterministic_ordering.json",
    "cancellation.json",
    "rng_replay.json",
    "Validate conformance fixtures"
)) {
    Assert-Contains -Content $conformanceWorkflow -Needle $row -Label ".github/workflows/conformance.yml"
}

$docsIndex = Get-Content -LiteralPath "website/src/index.md" -Raw
foreach ($row in @(
    "docs/community/",
    "docs/trustworthy-simulation/",
    "docs/release/",
    "docs/community/playground.md"
)) {
    Assert-Contains -Content $docsIndex -Needle $row -Label "website/src/index.md"
}

$qualityGates = Get-Content -LiteralPath "conductor/quality-gates.md" -Raw
foreach ($row in @(
    "Comparative benchmarks and reproducibility guidance (Track 18)",
    "Research software, citation, and archival guidance (Track 19)",
    "OpenSSF, supply-chain trust, and institutional-readiness guidance (Track 20)",
    "Community adoption, education, and ecosystem guidance (Track 17)",
    "Verification, validation, and uncertainty guidance (Track 21)",
    "Scenario runner and replay guidance (Track 22)",
    "Starter-kit and model-zoo guidance (Track 23)",
    "Playground and demo guidance (Track 24)",
    "API design review and compatibility governance guidance (Track 25)",
    "Interoperability standards review guidance (Track 26)",
    "Wave manager and execution gatekeeper guidance (Track 29)",
    "Toolchain and version support matrix guidance (Track 30)",
    "Performance regression guard guidance (Track 31)"
)) {
    Assert-Contains -Content $qualityGates -Needle $row -Label "quality-gates.md"
}

$wavePolicy = Get-Content -LiteralPath "conductor/wave-policy.md" -Raw
foreach ($row in @(
    "Wave 5",
    "29 Wave Manager & Execution Gatekeeper",
    "30 Toolchain & Version Support Matrix",
    "31 Performance Regression Guard"
)) {
    Assert-Contains -Content $wavePolicy -Needle $row -Label "wave-policy.md"
}

$toolchainMatrix = Get-Content -LiteralPath "conductor/toolchain-matrix.md" -Raw
foreach ($row in @(
    "## Rust",
    "## Python",
    "## C#",
    "## Go"
)) {
    Assert-Contains -Content $toolchainMatrix -Needle $row -Label "conductor/toolchain-matrix.md"
}

$perfThresholds = Get-Content -LiteralPath "conductor/performance-thresholds.md" -Raw
foreach ($row in @(
    "schedule_1m_events_preview",
    "hybrid_des_abm_smoke_preview"
)) {
    Assert-Contains -Content $perfThresholds -Needle $row -Label "conductor/performance-thresholds.md"
}

if (-not $SkipCargo) {
    cargo test --workspace
}

Write-Host "Conductor setup validation passed."
