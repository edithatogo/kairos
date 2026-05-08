param(
    [string]$ReviewPath = "docs/interoperability/standards-review.md",
    [string]$ConductorPath = "conductor/interoperability-standards.md",
    [string]$MappingPath = "docs/interoperability/standards-mapping.md",
    [string]$AdrPath = "docs/interoperability/adr-recommendations.md"
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
$mapping = Get-Content -LiteralPath $MappingPath -Raw
$adr = Get-Content -LiteralPath $AdrPath -Raw

foreach ($standard in $requiredStandards) {
    Assert-Contains -Text $review -Needle $standard -Scope $ReviewPath
    Assert-Contains -Text $conductor -Needle $standard -Scope $ConductorPath
    Assert-Contains -Text $mapping -Needle $standard -Scope $MappingPath
    Assert-Contains -Text $adr -Needle $standard -Scope $AdrPath
}

foreach ($label in $requiredLabels) {
    Assert-Contains -Text $review -Needle $label -Scope $ReviewPath
    Assert-Contains -Text $conductor -Needle $label -Scope $ConductorPath
    Assert-Contains -Text $mapping -Needle $label -Scope $MappingPath
}

foreach ($term in $requiredEvidenceTerms) {
    Assert-Contains -Text $review -Needle $term -Scope $ReviewPath
    Assert-Contains -Text $mapping -Needle $term -Scope $MappingPath
}

foreach ($term in $requiredReleaseTerms) {
    Assert-Contains -Text $review -Needle $term -Scope "$ReviewPath release-impacting assertions"
    Assert-Contains -Text $mapping -Needle $term -Scope "$MappingPath release language matrix"
}

foreach ($term in @("standards-mapping", "Claim allowed now", "Missing behavior or release guard", "Release language matrix")) {
    Assert-Contains -Text $mapping -Needle $term -Scope "$MappingPath standards-mapping gate"
}

foreach ($term in @("adr-recommendations", "ADR-026-001", "ADR-026-009", "ADR required before claim changes?", "Initial ADR backlog")) {
    Assert-Contains -Text $adr -Needle $term -Scope "$AdrPath adr-recommendations gate"
}

$mappingRows = Select-String -Path $ReviewPath -Pattern "^\| (DEVS|FMI/FMU|SBML|CellML|OpenTelemetry semantic conventions|Arrow C Data Interface|Arrow IPC|Parquet) \| (Supported|Partial|Deferred|Unsupported) \|"
if ($mappingRows.Count -ne $requiredStandards.Count) {
    throw "Expected $($requiredStandards.Count) standards mapping rows, found $($mappingRows.Count)"
}

$gateMappingRows = Select-String -Path $MappingPath -Pattern "^\| (DEVS|FMI/FMU|SBML|CellML|OpenTelemetry semantic conventions|Arrow C Data Interface|Arrow IPC|Parquet) \| (Supported|Partial|Deferred|Unsupported) \|"
if ($gateMappingRows.Count -ne $requiredStandards.Count) {
    throw "Expected $($requiredStandards.Count) standards-mapping gate rows, found $($gateMappingRows.Count)"
}

$adrRows = Select-String -Path $AdrPath -Pattern "^\| ADR-026-\d{3} \|"
if ($adrRows.Count -lt 9) {
    throw "Expected at least 9 ADR recommendation rows, found $($adrRows.Count)"
}

Write-Output "Track 26 standards review validation passed: $($requiredStandards.Count) standards, $($requiredLabels.Count) labels, evidence, release guards, standards-mapping, and adr-recommendations found."
