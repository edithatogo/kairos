param(
    [string]$ReviewPath = "docs/interoperability/standards-review.md",
    [string]$ConductorPath = "conductor/interoperability-standards.md"
)

$ErrorActionPreference = "Stop"

$requiredStandards = @(
    "DEVS",
    "FMI/FMU",
    "SBML",
    "CellML",
    "OpenTelemetry semantic conventions",
    "Arrow C Data Interface",
    "Arrow IPC",
    "Parquet"
)

$requiredLabels = @("Supported", "Partial", "Deferred", "Unsupported")
$requiredEvidenceTerms = @(
    "crates/kairo-ecs-arrow/src/lib.rs",
    "crates/kairo-ecs-arrow/tests/schema_compatibility.rs",
    "docs/fmi-digital-twin/import-guide.md",
    "docs/fmi-digital-twin/export-guide.md",
    "docs/debugging/trace-format.md",
    "docs/streaming/stream-schema.md"
)
$requiredReleaseTerms = @(
    "Arrow C Data Interface",
    "Arrow IPC",
    "Parquet",
    "OpenTelemetry",
    "FMI/FMU",
    "SBML",
    "CellML",
    "unsupported ecosystem"
)

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Needle,
        [string]$Scope
    )

    if (-not $Text.Contains($Needle)) {
        throw "$Scope is missing required text: $Needle"
    }
}

$review = Get-Content -LiteralPath $ReviewPath -Raw
$conductor = Get-Content -LiteralPath $ConductorPath -Raw

foreach ($standard in $requiredStandards) {
    Assert-Contains -Text $review -Needle $standard -Scope $ReviewPath
    Assert-Contains -Text $conductor -Needle $standard -Scope $ConductorPath
}

foreach ($label in $requiredLabels) {
    Assert-Contains -Text $review -Needle $label -Scope $ReviewPath
    Assert-Contains -Text $conductor -Needle $label -Scope $ConductorPath
}

foreach ($term in $requiredEvidenceTerms) {
    Assert-Contains -Text $review -Needle $term -Scope $ReviewPath
}

foreach ($term in $requiredReleaseTerms) {
    Assert-Contains -Text $review -Needle $term -Scope "$ReviewPath release-impacting assertions"
}

$mappingRows = Select-String -Path $ReviewPath -Pattern "^\| (DEVS|FMI/FMU|SBML|CellML|OpenTelemetry semantic conventions|Arrow C Data Interface|Arrow IPC|Parquet) \| (Supported|Partial|Deferred|Unsupported) \|"
if ($mappingRows.Count -ne $requiredStandards.Count) {
    throw "Expected $($requiredStandards.Count) standards mapping rows, found $($mappingRows.Count)"
}

Write-Output "Track 26 standards review validation passed: $($requiredStandards.Count) standards, $($requiredLabels.Count) labels, evidence, and release guards found."
