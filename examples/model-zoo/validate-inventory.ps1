param(
    [string]$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
)

$ErrorActionPreference = 'Stop'

function Get-InventoryItems {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$RootKey
    )

    $items = @()
    $current = $null
    $pendingListKey = $null

    foreach ($rawLine in Get-Content -LiteralPath $Path) {
        $line = $rawLine.TrimEnd()

        if ($line -match "^\s*$RootKey\s*:\s*$") {
            continue
        }

        if ($line -match '^\s*-\s+([A-Za-z0-9_]+):\s*(.+?)\s*$') {
            if ($null -ne $current) {
                $items += [pscustomobject]$current
            }
            $current = [ordered]@{}
            $pendingListKey = $null
            $current[$matches[1]] = Convert-InventoryValue $matches[2]
            continue
        }

        if ($null -eq $current) {
            continue
        }

        if ($line -match '^\s+([A-Za-z0-9_]+):\s*(.*)$') {
            $key = $matches[1]
            $value = $matches[2].Trim()
            if ($value -eq '') {
                $current[$key] = @()
                $pendingListKey = $key
            } else {
                $current[$key] = Convert-InventoryValue $value
                $pendingListKey = $null
            }
            continue
        }

        if ($null -ne $pendingListKey -and $line -match '^\s+-\s+(.+?)\s*$') {
            $current[$pendingListKey] = @($current[$pendingListKey]) + (Convert-InventoryValue $matches[1])
        }
    }

    if ($null -ne $current) {
        $items += [pscustomobject]$current
    }

    return $items
}

function Convert-InventoryValue {
    param([string]$Value)

    $trimmed = $Value.Trim()
    if ($trimmed -match '^\[(.*)\]$') {
        $body = $matches[1].Trim()
        if ($body -eq '') {
            return @()
        }
        return @($body -split ',' | ForEach-Object { $_.Trim().Trim('"').Trim("'") })
    }
    return $trimmed.Trim('"').Trim("'")
}

function Assert-RelativePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RelativePath,
        [Parameter(Mandatory = $true)]
        [string]$Context
    )

    $fullPath = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $fullPath)) {
        throw "$Context references missing path: $RelativePath"
    }
}

function Assert-StarterKitReadme {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RelativePath,
        [Parameter(Mandatory = $true)]
        [string]$KitId
    )

    $fullPath = Join-Path $RepoRoot $RelativePath
    $content = Get-Content -LiteralPath $fullPath -Raw
    if ($content -notmatch '(?im)^Maturity:\s*`?[A-Za-z0-9_-]+`?\s*$') {
        throw "starter-kit entry $KitId README is missing a maturity label"
    }
    if ($content -notmatch '(?im)^##\s+Dependency list\s*$') {
        throw "starter-kit entry $KitId README is missing a dependency list section"
    }
    if ($content -notmatch '(?im)^##\s+Expected outputs\s*$') {
        throw "starter-kit entry $KitId README is missing an expected outputs section"
    }
    if ($content -notmatch '(?im)^##\s+Validation commands\s*$') {
        throw "starter-kit entry $KitId README is missing a validation commands section"
    }
}

function Assert-ModelReadme {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RelativePath,
        [Parameter(Mandatory = $true)]
        [string]$ModelId
    )

    $fullPath = Join-Path $RepoRoot $RelativePath
    $content = Get-Content -LiteralPath $fullPath -Raw
    if ($content -notmatch '(?im)^Maturity:\s*`?[A-Za-z0-9_-]+`?\s*$') {
        throw "model-zoo entry $ModelId README is missing a maturity label"
    }
    if ($content -notmatch '(?im)^##\s+Tutorial path\s*$') {
        throw "model-zoo entry $ModelId README is missing a tutorial path section"
    }
    if ($content -notmatch '(?im)^##\s+Expected outputs\s*$') {
        throw "model-zoo entry $ModelId README is missing an expected outputs section"
    }
    if ($content -notmatch '(?im)^##\s+Validation commands\s*$') {
        throw "model-zoo entry $ModelId README is missing a validation commands section"
    }
}

$modelZooPath = Join-Path $RepoRoot 'examples/model-zoo/model-zoo.yaml'
$starterKitPath = Join-Path $RepoRoot 'examples/starter-kits/starter-kits.yaml'

Assert-RelativePath 'examples/model-zoo/model-zoo.yaml' 'model-zoo inventory'
Assert-RelativePath 'examples/starter-kits/starter-kits.yaml' 'starter-kit inventory'

$models = @(Get-InventoryItems -Path $modelZooPath -RootKey 'models')
$kits = @(Get-InventoryItems -Path $starterKitPath -RootKey 'kits')

if ($models.Count -eq 0) {
    throw 'model-zoo inventory has no models'
}

if ($kits.Count -eq 0) {
    throw 'starter-kit inventory has no kits'
}

$modelIds = @{}
foreach ($model in $models) {
    if (-not $model.id) {
        throw 'model-zoo entry is missing id'
    }
    $modelIds[$model.id] = $true
    Assert-RelativePath $model.path "model-zoo entry $($model.id)"
    Assert-RelativePath $model.docs "model-zoo entry $($model.id)"
    Assert-ModelReadme $model.docs $model.id
    if ($model.tutorial) {
        Assert-RelativePath $model.tutorial "model-zoo entry $($model.id) tutorial"
    }
    if ($model.figure) {
        Assert-RelativePath $model.figure "model-zoo entry $($model.id) figure"
    }
}

foreach ($kit in $kits) {
    if (-not $kit.id) {
        throw 'starter-kit entry is missing id'
    }
    Assert-RelativePath $kit.kit_path "starter-kit entry $($kit.id)"
    Assert-RelativePath $kit.docs "starter-kit entry $($kit.id)"
    Assert-StarterKitReadme $kit.docs $kit.id

    foreach ($modelId in @($kit.model_zoo_ids)) {
        if (-not $modelIds.ContainsKey($modelId)) {
            throw "starter-kit entry $($kit.id) references unknown model_zoo_id: $modelId"
        }
    }

    foreach ($examplePath in @($kit.example_paths)) {
        Assert-RelativePath $examplePath "starter-kit entry $($kit.id)"
    }
}

Write-Host "Validated $($models.Count) model-zoo entries and $($kits.Count) starter-kit entries."
