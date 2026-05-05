param()

$ErrorActionPreference = "Stop"

$rootCargo = Get-Content -LiteralPath "Cargo.toml" -Raw
$tracksYaml = Get-Content -LiteralPath "conductor/tracks.yaml" -Raw
$issues = [System.Collections.Generic.List[string]]::new()

function Add-Issue {
    param([string]$Message)
    $script:issues.Add($Message)
}

function Assert-WorkspaceMember {
    param(
        [string]$TrackId,
        [string]$CratePath
    )

    if ((Test-Path -LiteralPath $CratePath) -and ($rootCargo -notmatch [regex]::Escape("`"$CratePath`""))) {
        Add-Issue -Message "Track $TrackId crate exists but is not listed in root Cargo.toml workspace members: $CratePath"
    }
}

function Assert-No-StaleEvidence {
    param(
        [string]$TrackId,
        [string]$TrackPath,
        [string[]]$CratePaths,
        [string[]]$ConcretePaths
    )

    $files = @("plan.md", "handoff.md", "test-matrix.md") |
        ForEach-Object { Join-Path $TrackPath $_ } |
        Where-Object { Test-Path -LiteralPath $_ }

    $content = ($files | ForEach-Object { Get-Content -LiteralPath $_ -Raw }) -join "`n"

    foreach ($cratePath in $CratePaths) {
        Assert-WorkspaceMember -TrackId $TrackId -CratePath $cratePath
    }

    $existingCrates = @($CratePaths | Where-Object { Test-Path -LiteralPath $_ })
    $workspaceCrates = @($existingCrates | Where-Object { $rootCargo -match [regex]::Escape("`"$_`"") })
    if ($workspaceCrates.Count -gt 0) {
        $staleWorkspacePatterns = @(
            'root workspace integration is blocked',
            'root Cargo.toml is outside',
            'root `Cargo.toml` is outside',
            'did not edit root `Cargo.toml`',
            'did not wire the new crates into root',
            'workspace membership .* pending',
            'not included in the root workspace',
            'isolated package with its own `[workspace]`'
        )

        foreach ($pattern in $staleWorkspacePatterns) {
            if ($content -match $pattern) {
                Add-Issue -Message "Track $TrackId contains stale workspace blocker claim after root workspace wiring: $pattern"
            }
        }
    }

    foreach ($cratePath in $CratePaths) {
        $crateName = Split-Path -Leaf $cratePath
        if ((Test-Path -LiteralPath $cratePath) -and ($content -match "No\s+`?$([regex]::Escape($crateName))`?\s+crate exists yet")) {
            Add-Issue -Message "Track $TrackId claims $crateName does not exist, but $cratePath exists"
        }
    }

    $existingConcrete = @($ConcretePaths | Where-Object { Test-Path -LiteralPath $_ })
    if (($existingConcrete.Count -gt 0) -and ($content -match "No code files were changed")) {
        Add-Issue -Message "Track $TrackId claims no code files changed, but concrete owned paths exist: $($existingConcrete -join ', ')"
    }

    $statusMatch = [regex]::Match($tracksYaml, "(?ms)^\s*-\s*id:\s*$TrackId\s*.*?^\s*status:\s*(\S.*)$")
    if (-not $statusMatch.Success) {
        Add-Issue -Message "Track $TrackId missing from conductor/tracks.yaml"
        return
    }
    $status = $statusMatch.Groups[1].Value.Trim()
    $existingConcreteForStatus = @($ConcretePaths | Where-Object { Test-Path -LiteralPath $_ })
    if (($status -eq "Planned") -and ($existingConcreteForStatus.Count -gt 0)) {
        Add-Issue -Message "Track $TrackId is still Planned in conductor/tracks.yaml despite concrete owned paths: $($existingConcreteForStatus -join ', ')"
    }
    if (($status -eq "Done" -or $status -eq "In Review") -and ($content -match "Not marked complete|Not complete|Blocked validation|blocked until|not implemented|remain future work")) {
        Add-Issue -Message "Track $TrackId status is $status but its evidence still names incomplete or blocked work"
    }
}

$trackChecks = @(
    @{
        Id = "02"
        Path = "conductor/tracks/02-bridge-kairo-ecs-ffi-uniffi-diplomat"
        Crates = @("crates/kairo-ecs-ffi", "crates/kairo-ecs-uniffi", "crates/kairo-ecs-diplomat")
        Concrete = @("crates/kairo-ecs-ffi", "crates/kairo-ecs-uniffi", "crates/kairo-ecs-diplomat", "include")
    },
    @{
        Id = "03"
        Path = "conductor/tracks/03-flow-des-trajectory-abm-behavior"
        Crates = @("crates/kairo-ecs-des", "crates/kairo-ecs-abm")
        Concrete = @("crates/kairo-ecs-des", "crates/kairo-ecs-abm", "examples/flow")
    },
    @{
        Id = "04"
        Path = "conductor/tracks/04-analyst-kairo-ecs-arrow"
        Crates = @("crates/kairo-ecs-arrow")
        Concrete = @("crates/kairo-ecs-arrow", "schemas/arrow", "examples/telemetry")
    },
    @{
        Id = "05"
        Path = "conductor/tracks/05-window-kairo-ecs-viz"
        Crates = @("crates/kairo-ecs-viz")
        Concrete = @("crates/kairo-ecs-viz", "examples/viz", "website/docs/visualization")
    },
    @{
        Id = "06"
        Path = "conductor/tracks/06-python-binding-310-314"
        Crates = @()
        Concrete = @("bindings/python", "packaging/python")
    },
    @{
        Id = "07"
        Path = "conductor/tracks/07-r-binding"
        Crates = @()
        Concrete = @("bindings/r", "packaging/r")
    },
    @{
        Id = "08"
        Path = "conductor/tracks/08-julia-binding"
        Crates = @()
        Concrete = @("bindings/julia", "packaging/julia")
    },
    @{
        Id = "09"
        Path = "conductor/tracks/09-typescript-wasm-binding"
        Crates = @("crates/kairo-ecs-wasm")
        Concrete = @("bindings/typescript", "crates/kairo-ecs-wasm", "packaging/npm")
    },
    @{
        Id = "10"
        Path = "conductor/tracks/10-csharp-dotnet-10-11-binding"
        Crates = @()
        Concrete = @("bindings/csharp", "packaging/nuget")
    },
    @{
        Id = "11"
        Path = "conductor/tracks/11-go-binding"
        Crates = @()
        Concrete = @("bindings/go", "packaging/go")
    },
    @{
        Id = "12"
        Path = "conductor/tracks/12-conformance-testing-benchmarks"
        Crates = @("crates/kairo-ecs-bench")
        Concrete = @("conformance", "tests/conformance", "benches", "crates/kairo-ecs-bench")
    },
    @{
        Id = "13"
        Path = "conductor/tracks/13-ci-cd-quality-supply-chain"
        Crates = @()
        Concrete = @(".github", "deny.toml", "rust-toolchain.toml")
    },
    @{
        Id = "14"
        Path = "conductor/tracks/14-docs-site-education"
        Crates = @()
        Concrete = @("docs", "website", "examples/docs")
    },
    @{
        Id = "15"
        Path = "conductor/tracks/15-packaging-publishing-delivery"
        Crates = @()
        Concrete = @("packaging", ".github/workflows/release.yml", "docs/release")
    },
    @{
        Id = "16"
        Path = "conductor/tracks/16-release-governance-maintenance"
        Crates = @()
        Concrete = @("conductor/maintenance-governance.md", "docs/release", "CHANGELOG.md")
    },
    @{
        Id = "17"
        Path = "conductor/tracks/17-community-adoption-education-ecosystem"
        Crates = @()
        Concrete = @("docs/community", "examples")
    },
    @{
        Id = "18"
        Path = "conductor/tracks/18-comparative-benchmarks-reproducibility"
        Crates = @()
        Concrete = @("docs/benchmarks", "benches/benchmark_reproducibility.py", "benches/benchmark-smoke.json")
    },
    @{
        Id = "19"
        Path = "conductor/tracks/19-research-software-citation-archival"
        Crates = @()
        Concrete = @("CITATION.cff", "codemeta.json", ".zenodo.json", "docs/research", "paper", "conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1")
    },
    @{
        Id = "20"
        Path = "conductor/tracks/20-openssf-supply-chain-institutional-trust"
        Crates = @()
        Concrete = @("SECURITY.md", "CODEOWNERS", ".github/workflows/scorecard.yml", ".github/workflows/sbom-attestations.yml", "conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md")
    },
    @{
        Id = "21"
        Path = "conductor/tracks/21-verification-validation-uncertainty"
        Crates = @()
        Concrete = @("docs/trustworthy-simulation", "docs/validation", "scripts/validation", "conformance")
    },
    @{
        Id = "22"
        Path = "conductor/tracks/22-experiment-runner-scenario-management"
        Crates = @("crates/kairo-ecs-cli")
        Concrete = @("crates/kairo-ecs-cli", "examples/experiments", "scenarios", "docs/scenarios", "scripts/scenarios")
    },
    @{
        Id = "23"
        Path = "conductor/tracks/23-domain-starter-kits-model-zoo"
        Crates = @()
        Concrete = @("examples/model-zoo", "examples/starter-kits", "docs/model-zoo", "docs/starter-kits")
    },
    @{
        Id = "24"
        Path = "conductor/tracks/24-playground-demos-visualization-ux"
        Crates = @()
        Concrete = @("website/playground", "docs/playground", "website/scripts/smoke-playground.mjs", "docs/community/playground.md")
    },
    @{
        Id = "25"
        Path = "conductor/tracks/25-api-design-review-compatibility-governance"
        Crates = @()
        Concrete = @("docs/design/protected-surface-inventory.json", "docs/design/compatibility-governance.md", "docs/design/validate-compatibility-pack.ps1", "conductor/contracts/versioning-compatibility.md")
    },
    @{
        Id = "26"
        Path = "conductor/tracks/26-interoperability-standards-review"
        Crates = @()
        Concrete = @("docs/interoperability", "conductor/interoperability-standards.md", "conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1")
    },
    @{
        Id = "27"
        Path = "conductor/tracks/27-developer-experience-reproducible-environments"
        Crates = @()
        Concrete = @("justfile", "docs/developer-experience", "scripts/dx")
    },
    @{
        Id = "28"
        Path = "conductor/tracks/28-red-team-devils-advocate-review"
        Crates = @()
        Concrete = @("reviews", "conductor/red-team-review.md", "conductor/devils-advocate-review.md", "conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json")
    },
    @{
        Id = "29"
        Path = "conductor/tracks/29-wave-manager-execution-gatekeeper"
        Crates = @()
        Concrete = @("conductor/wave-policy.md", "conductor/gates", "conductor/tracks/29-wave-manager-execution-gatekeeper/validate-wave-gates.ps1")
    },
    @{
        Id = "30"
        Path = "conductor/tracks/30-toolchain-version-support-matrix"
        Crates = @()
        Concrete = @("conductor/toolchain-matrix.md", ".github/workflows/toolchain-check.yml", "conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1")
    },
    @{
        Id = "31"
        Path = "conductor/tracks/31-performance-regression-guard"
        Crates = @()
        Concrete = @("conductor/performance-thresholds.md", "benches/regression", ".github/workflows/bench-regression.yml")
    },
    @{
        Id = "32"
        Path = "conductor/tracks/32-gpu-compute-acceleration"
        Crates = @("crates/kairo-ecs-gpu")
        Concrete = @("crates/kairo-ecs-gpu", "docs/gpu-compute")
    },
    @{
        Id = "33"
        Path = "conductor/tracks/33-webgpu-compute-browser"
        Crates = @("crates/kairo-ecs-webgpu")
        Concrete = @("crates/kairo-ecs-webgpu", "website/webgpu-demo")
    },
    @{
        Id = "34"
        Path = "conductor/tracks/34-pdes-parallel-execution"
        Crates = @("crates/kairo-ecs-pdes")
        Concrete = @("crates/kairo-ecs-pdes", "docs/pdes", "benches/pdes")
    },
    @{
        Id = "35"
        Path = "conductor/tracks/35-distributed-simulation-mpi-grpc"
        Crates = @("crates/kairo-ecs-mpi", "crates/kairo-ecs-grpc")
        Concrete = @("crates/kairo-ecs-mpi", "crates/kairo-ecs-grpc", "docs/distributed")
    },
    @{
        Id = "36"
        Path = "conductor/tracks/36-streaming-real-time-processing"
        Crates = @("crates/kairo-ecs-streaming")
        Concrete = @("crates/kairo-ecs-streaming", "docs/streaming")
    },
    @{
        Id = "37"
        Path = "conductor/tracks/37-ml-ai-integration-inference"
        Crates = @("crates/kairo-ecs-ml")
        Concrete = @("crates/kairo-ecs-ml", "docs/ml", "examples/ml-surrogate", "python/kairo_gym")
    },
    @{
        Id = "38"
        Path = "conductor/tracks/38-fmi-fmu-digital-twin-bridge"
        Crates = @("crates/kairo-ecs-fmi")
        Concrete = @("crates/kairo-ecs-fmi", "docs/fmi-digital-twin", "examples/fmi-co-simulation")
    },
    @{
        Id = "39"
        Path = "conductor/tracks/39-cloud-hpc-batch-runners"
        Crates = @()
        Concrete = @("docker", "k8s", "cloud", "hpc/slurm", "docs/cloud-hpc")
    },
    @{
        Id = "40"
        Path = "conductor/tracks/40-time-travel-debugging-interactive-stepping"
        Crates = @("crates/kairo-ecs-debug")
        Concrete = @("crates/kairo-ecs-debug", "docs/debugging", "website/time-travel-demo")
    }
)

foreach ($check in $trackChecks) {
    Assert-No-StaleEvidence -TrackId $check.Id -TrackPath $check.Path -CratePaths $check.Crates -ConcretePaths $check.Concrete
}

$controlFiles = @(
    "conductor/tracks.md",
    "conductor/tracks.yaml",
    "conductor/subagents.yaml",
    "conductor/subagents.md",
    "conductor/track-map.md",
    "conductor/implementation-readiness.md",
    "conductor/wave-policy.md",
    "conductor/quality-gates.md",
    "conductor/delivery-readiness-checklist.md"
) + @(Get-ChildItem -LiteralPath "scripts" -Filter "*.ps1" | ForEach-Object { $_.FullName }) +
    @(Get-ChildItem -LiteralPath "conductor/gates" -File | ForEach-Object { $_.FullName })

foreach ($file in $controlFiles | Where-Object { Test-Path -LiteralPath $_ }) {
    $content = Get-Content -LiteralPath $file -Raw
    if ($content -match '(?i)(expected|exactly|required)\s+(32|41)\s+track') {
        Add-Issue -Message "Hard-coded track count found in control file: $file"
    }
    if ($content -match 'There are\s+\d+\s+track directories') {
        Add-Issue -Message "Hard-coded track directory count found in control file: $file"
    }
    if ($content -match 'There are\s+32\s+track directories') {
        Add-Issue -Message "Stale 32-track directory count found in control file: $file"
    }
    if ($content -match 'covering tracks\s+`00`\s+through\s+`31`') {
        Add-Issue -Message "Stale 00-31 coverage claim found in control file: $file"
    }
    if ($content -match 'Tracks?\s+32-40.*R0|Track\s+3[2-9]\).*Planned only|Track\s+40\).*Planned only') {
        Add-Issue -Message "Stale planning-only readiness claim for Tracks 32-40 found in control file: $file"
    }
}

if ($issues.Count -gt 0) {
    throw "Track no-skip claim validation failed:`n- $($issues -join "`n- ")"
}

Write-Host "Track no-skip claim validation passed."
