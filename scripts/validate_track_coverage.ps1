param(
    [switch]$SkipCargo
)

$ErrorActionPreference = "Stop"

$tracksYaml = Get-Content -LiteralPath "conductor/tracks.yaml" -Raw
if ($tracksYaml -notmatch "schema_version:\s*1") {
    throw "tracks.yaml missing schema version"
}
if ($tracksYaml -notmatch "required_gates:") {
    throw "tracks.yaml missing required gates"
}

$wavePolicy = Get-Content -LiteralPath "conductor/wave-policy.md" -Raw

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

foreach ($track in $expectedTrackIds) {
    if ($tracksYaml -notmatch "id:\s*$track") {
        throw "tracks.yaml missing track id $track"
    }
}

$tracksIndex = Get-Content -LiteralPath "conductor/tracks.md" -Raw
foreach ($track in $expectedTrackIds) {
    if ($tracksIndex -notmatch "\|\s*$track\s*\|") {
        throw "tracks.md missing track row $track"
    }
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

$requiredFiles = @("spec.md","plan.md","agent-contract.md","risk-register.md","test-matrix.md","handoff.md")
foreach ($dir in $trackDirs) {
    foreach ($file in $requiredFiles) {
        $path = Join-Path $dir.FullName $file
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing track artifact: $path"
        }
    }
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
    "Community adoption, education, and ecosystem guidance from Track 17 is green.",
    "Verification, validation, and uncertainty guidance from Track 21 is green.",
    "Scenario runner and replay guidance from Track 22 is green.",
    "Starter-kit and model-zoo guidance from Track 23 is green.",
    "Playground and demo guidance from Track 24 is green."
)) {
    Assert-Contains -Content $deliveryChecklist -Needle $row -Label "delivery-readiness-checklist.md"
}

$qualityGates = Get-Content -LiteralPath "conductor/quality-gates.md" -Raw
foreach ($row in @(
    "Community adoption, education, and ecosystem guidance (Track 17)",
    "Verification, validation, and uncertainty guidance (Track 21)",
    "Scenario runner and replay guidance (Track 22)",
    "Starter-kit and model-zoo guidance (Track 23)",
    "Playground and demo guidance (Track 24)",
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

$releaseChecklist = Get-Content -LiteralPath "docs/release/release-checklist.md" -Raw
foreach ($row in @(
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
    'Current checked-in package surfaces',
    'The workspace is real; the root meta crate and bridge crates remain future track outputs.',
    'Current package surface: `bindings/typescript/package.json` declares `@kairo-ecs/typescript` and a plain TypeScript test/typecheck loop.'
)) {
    Assert-Contains -Content $packageCatalog -Needle $row -Label "conductor/package-catalog.md"
}

$packageMatrix = Get-Content -LiteralPath "conductor/package-matrix.md" -Raw
foreach ($row in @(
    'Current checked-in package surfaces',
    'The checked-in workspace currently ships `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, and `kairo-ecs-rng`',
    'Current release posture'
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

$trackChecks = @(
    @{
        Path = "conductor/tracks/18-comparative-benchmarks-reproducibility/test-matrix.md"
        Needles = @(
            "Benchmark plan exists",
            "Fixture manifest exists",
            "Ready fixture IDs are named",
            "Measurement inputs are explicit",
            "Smoke workflow matches the real contract",
            "Reproducibility claim is tied to a real fixture or benchmark target"
        )
    },
    @{
        Path = "conductor/tracks/18-comparative-benchmarks-reproducibility/handoff.md"
        Needles = @(
            'The reproducibility surface now points at the real benchmark plan, the real',
            'The track treats committed benchmark-plan text, the fixture manifest, and the',
            'The concrete risk is benchmark drift if fixture IDs, seed notes, or comparison',
            'baselines change after publication. Keep `benches/benchmark-plan.md` and',
            '`conformance/fixtures/manifest.json` versioned together.'
        )
    },
    @{
        Path = "conductor/tracks/19-research-software-citation-archival/test-matrix.md"
        Needles = @(
            "Citation metadata file exists and validates",
            "Archive note or release metadata exists",
            "Markdown lint/link check",
            "Artifact existence check",
            "Docs build smoke test passes",
            "Release gate integration",
            "Citation guidance is explicit enough for reuse"
        )
    },
    @{
        Path = "conductor/tracks/19-research-software-citation-archival/handoff.md"
        Needles = @(
            'Documented the citation and archival metadata that should accompany a release or published package',
            'Citation metadata, archive notes, DOI/Zenodo path, and `just docs-build` now sit on the public-release path before a package is allowed out.',
            'stale author, version, or DOI metadata'
        )
    },
    @{
        Path = "conductor/tracks/20-openssf-supply-chain-institutional-trust/test-matrix.md"
        Needles = @(
            "Track docs exist and render cleanly",
            "includes the exact OpenSSF and supply-chain gate section",
            '`SECURITY.md`, `CODEOWNERS`, and dependency policy are present',
            "Scorecard workflow exists",
            "SBOM/provenance plan is recorded",
            "Vulnerability response path is documented",
            "Release waiver and exception process is documented",
            "Red-team or release-blocker escalation path is defined"
        )
    },
    @{
        Path = "conductor/tracks/20-openssf-supply-chain-institutional-trust/handoff.md"
        Needles = @(
            'Captured the supply-chain and institutional-readiness checks that should sit alongside the release evidence pack',
            'OpenSSF Scorecard, dependency-review, SBOM, provenance, and waiver handling now feed the release gate surface before any draft release can move to publish.',
            'The concrete risk is a missing or incomplete GitHub Actions workflow for Scorecard'
        )
    },
    @{
        Path = "conductor/tracks/25-api-design-review-compatibility-governance/test-matrix.md"
        Needles = @(
            'Surface inventory exists for `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, and `bindings/go`',
            "Compatibility policy names the live crate and package roots",
            "compatibility gate rows",
            "compatibility gate section",
            "Breaking-change definition is explicit",
            "ADR requirement is explicit",
            "Migration note requirement is explicit",
            "Release-stage decision rules are explicit",
            "Release hold path is documented"
        )
    },
    @{
        Path = "conductor/tracks/25-api-design-review-compatibility-governance/handoff.md"
        Needles = @(
            'Captured the compatibility policy surface so release planning can distinguish stable, experimental, and migration-only APIs across the current Rust crates and binding package roots.',
            'Compatibility review, ADR requirements, migration-note requirements, and release-hold decisions now sit on the public release path',
            'later API change outrunning the compatibility policy'
        )
    },
    @{
        Path = "conductor/tracks/26-interoperability-standards-review/test-matrix.md"
        Needles = @(
            "Standards inventory names DEVS, FMI/FMU, SBML, CellML, OpenTelemetry, Arrow C Data Interface, Arrow IPC, and Parquet",
            "Mapping table distinguishes supported, partial, deferred, and unsupported mappings",
            "Markdown lint/link check",
            "Artifact existence check",
            "Docs build smoke test passes",
            "Known gaps are documented",
            "Red-team objections about false interoperability claims are answered"
        )
    },
    @{
        Path = "conductor/tracks/26-interoperability-standards-review/handoff.md"
        Needles = @(
            'Captured the interoperability mapping story so downstream tracks can rely on a named set of supported, partial, deferred, and unsupported translations.',
            'Interoperability review now names the release-impacting assertions that need review before an external-compatibility claim is made.',
            'supported, partial, deferred, and unsupported translations'
        )
    },
    @{
        Path = "conductor/tracks/29-wave-manager-execution-gatekeeper/test-matrix.md"
        Needles = @(
            'Wave assignment is derivable from the dependency graph',
            '`wave-progression-check` gate exists and is documented in `conductor/quality-gates.md`',
            '`dependency-closure-check` gate exists and is documented in `conductor/quality-gates.md`'
        )
    },
    @{
        Path = "conductor/tracks/29-wave-manager-execution-gatekeeper/handoff.md"
        Needles = @(
            "wave policy",
            "dependency-closure-check",
            "wave-progression-check"
        )
    },
    @{
        Path = "conductor/tracks/30-toolchain-version-support-matrix/test-matrix.md"
        Needles = @(
            'conductor/toolchain-matrix.md` exists and contains rows for Rust, Python, .NET, Julia, R, Go, Node/Wasm',
            'Version-drop policy is documented with notice period and removal criteria',
            'conductor/quality-gates.md` includes `toolchain-matrix-current` and `version-drop-policy-check`',
            '.github/workflows/toolchain-check.yml` exists and is referenced in CI'
        )
    },
    @{
        Path = "conductor/tracks/30-toolchain-version-support-matrix/handoff.md"
        Needles = @(
            'Defined the cross-language toolchain version support matrix',
            'toolchain-check.yml',
            'version-drop policy'
        )
    },
    @{
        Path = "conductor/tracks/31-performance-regression-guard/test-matrix.md"
        Needles = @(
            'conductor/performance-thresholds.md` exists and lists every active benchmark',
            'Each benchmark row includes baseline value, acceptable regression %, and measurement methodology',
            'conductor/quality-gates.md` includes `benchmark-regression-check` and `threshold-definition-exists`',
            '.github/workflows/bench-regression.yml` exists and is referenced in CI'
        )
    },
    @{
        Path = "conductor/tracks/31-performance-regression-guard/handoff.md"
        Needles = @(
            'Defined the performance regression detection framework for KairoECS',
            'bench-regression.yml',
            'benchmark-regression-check'
        )
    },
    @{
        Path = "conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md"
        Needles = @(
            "Discovery page renders and shows a clear install path, quickstart path, and contributor entry points",
            "Community claims match tracked repo artifacts"
        )
    },
    @{
        Path = "conductor/tracks/21-verification-validation-uncertainty/test-matrix.md"
        Needles = @(
            "Docs page names the three terms",
            "Evidence boundary is explicit",
            "Replay/scenario fixture tie-in exists"
        )
    },
    @{
        Path = "conductor/tracks/22-experiment-runner-scenario-management/test-matrix.md"
        Needles = @(
            "Scenario manifest exists",
            "Replay or seed control is documented",
            "Scenario output shape is documented",
            'Real fixture reference included (`scheduler_ordering_v1`)'
        )
    },
    @{
        Path = "conductor/tracks/23-domain-starter-kits-model-zoo/test-matrix.md"
        Needles = @(
            "Starter-kit inventory exists",
            "Model-zoo entry points are linked",
            "Example or kit path is concrete"
        )
    },
    @{
        Path = "conductor/tracks/24-playground-demos-visualization-ux/test-matrix.md"
        Needles = @(
            "Demo or playground page exists",
            '`docs/community/playground.md` exists and is linked from the docs home page',
            "Visualization assets are present",
            "Screenshot target paths are named explicitly"
        )
    }
)

foreach ($check in $trackChecks) {
    $content = Get-Content -LiteralPath $check.Path -Raw
    foreach ($needle in $check.Needles) {
        Assert-Contains -Content $content -Needle $needle -Label $check.Path
    }
}

if ($wavePolicy -notmatch "No-skip controls") {
    throw "Wave policy missing no-skip controls"
}

if (-not $SkipCargo) {
    cargo test --workspace
}

Write-Host "Track coverage validation passed."
