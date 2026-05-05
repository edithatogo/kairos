param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path,
    [switch]$ReleaseGate
)

$ErrorActionPreference = 'Stop'

function Read-TextFile([string]$RelativePath) {
    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required file missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw
}

$inventoryPath = Join-Path $RepoRoot 'docs/design/protected-surface-inventory.json'
if (-not (Test-Path -LiteralPath $inventoryPath)) {
    throw 'Required file missing: docs/design/protected-surface-inventory.json'
}

$inventory = Get-Content -LiteralPath $inventoryPath -Raw | ConvertFrom-Json
$requiredKinds = @('rust_api', 'c_abi', 'arrow_schema', 'host_api', 'conformance')
$seenKinds = @{}
$failures = New-Object System.Collections.Generic.List[string]

foreach ($surface in $inventory.surfaces) {
    $seenKinds[$surface.kind] = $true

    if ([string]::IsNullOrWhiteSpace($surface.id)) {
        $failures.Add('Surface has blank id')
    }
    if ([string]::IsNullOrWhiteSpace($surface.root)) {
        $failures.Add("Surface '$($surface.id)' has blank root")
        continue
    }

    $rootPath = Join-Path $RepoRoot $surface.root
    if (-not (Test-Path -LiteralPath $rootPath)) {
        $failures.Add("Inventory root missing on disk: $($surface.root)")
    }

    foreach ($field in @('status', 'example')) {
        if ([string]::IsNullOrWhiteSpace($surface.$field)) {
            $failures.Add("Surface '$($surface.id)' has blank $field")
        }
    }

    if (-not $surface.breaking_change_requires -or $surface.breaking_change_requires.Count -eq 0) {
        $failures.Add("Surface '$($surface.id)' has no breaking_change_requires entries")
    }
    if (-not $surface.release_hold_when -or $surface.release_hold_when.Count -eq 0) {
        $failures.Add("Surface '$($surface.id)' has no release_hold_when entries")
    }
}

foreach ($kind in $requiredKinds) {
    if (-not $seenKinds.ContainsKey($kind)) {
        $failures.Add("Inventory has no surface kind: $kind")
    }
}

$policy = Read-TextFile 'conductor/contracts/versioning-compatibility.md'
$quality = Read-TextFile 'conductor/quality-gates.md'
$readiness = Read-TextFile 'conductor/delivery-readiness-checklist.md'
$design = Read-TextFile 'docs/design/api-review.md'
$release = $null
if ($ReleaseGate) {
    $release = Read-TextFile 'docs/release/compatibility.md'
}

$policyRequiredPhrases = @(
    'Protected surface inventory',
    'Breaking-change rules',
    'ADR required',
    'Migration note required',
    'Release hold criteria',
    'docs/design/protected-surface-inventory.json'
)

foreach ($phrase in $policyRequiredPhrases) {
    if ($policy -notmatch [regex]::Escape($phrase)) {
        $failures.Add("Policy missing required phrase: $phrase")
    }
}

foreach ($surface in $inventory.surfaces) {
    $root = [regex]::Escape($surface.root)
    if ($policy -notmatch $root) {
        $failures.Add("Policy does not name inventory root: $($surface.root)")
    }
    if ($ReleaseGate -and $release -notmatch $root) {
        $failures.Add("Release compatibility note does not name inventory root: $($surface.root)")
    }
}

foreach ($fileCheck in @(
    @{ Name = 'quality gates'; Text = $quality },
    @{ Name = 'delivery readiness'; Text = $readiness },
    @{ Name = 'design index'; Text = $design }
)) {
    if ($fileCheck.Text -notmatch 'validate-compatibility-pack\.ps1') {
        $failures.Add("$($fileCheck.Name) does not reference validate-compatibility-pack.ps1")
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Output "ERROR: $_" }
    exit 1
}

if ($ReleaseGate) {
    Write-Output "compatibility release-gate validation passed: $($inventory.surfaces.Count) protected surfaces"
} else {
    Write-Output "compatibility pack validation passed: $($inventory.surfaces.Count) protected surfaces"
}
