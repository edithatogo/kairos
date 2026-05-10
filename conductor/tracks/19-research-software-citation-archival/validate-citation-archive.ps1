param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
)

$ErrorActionPreference = 'Stop'
$failures = New-Object System.Collections.Generic.List[string]

function Add-Failure {
    param([string]$Message)
    $failures.Add($Message) | Out-Null
}

function Require-File {
    param([string]$RelativePath)
    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Add-Failure "Missing required file: $RelativePath"
        return $null
    }
    return $path
}

function Read-Text {
    param([string]$RelativePath)
    $path = Require-File $RelativePath
    if ($null -eq $path) {
        return ''
    }
    return Get-Content -LiteralPath $path -Raw
}

function Get-CffScalar {
    param(
        [string]$Text,
        [string]$Key
    )
    $match = [regex]::Match($Text, "(?m)^$([regex]::Escape($Key)):\s*(.+?)\s*$")
    if (-not $match.Success) {
        return $null
    }
    return $match.Groups[1].Value.Trim().Trim('"')
}

function Require-CffScalar {
    param(
        [string]$Text,
        [string]$Key
    )
    $value = Get-CffScalar $Text $Key
    if ([string]::IsNullOrWhiteSpace($value)) {
        Add-Failure "CITATION.cff missing scalar field: $Key"
    }
    return $value
}

function Require-JsonField {
    param(
        [object]$Json,
        [string]$Path,
        [string]$Label
    )
    $value = $Json
    foreach ($segment in $Path.Split('.')) {
        if ($null -eq $value -or -not ($value.PSObject.Properties.Name -contains $segment)) {
            Add-Failure "$Label missing field: $Path"
            return $null
        }
        $value = $value.$segment
    }
    if ($null -eq $value -or ($value -is [string] -and [string]::IsNullOrWhiteSpace($value))) {
        Add-Failure "$Label has empty field: $Path"
    }
    return $value
}

function Normalize-License {
    param([string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $Value
    }
    return $Value.
        Replace('https://spdx.org/licenses/Apache-2.0.html', 'Apache-2.0').
        Replace('https://spdx.org/licenses/MIT.html', 'MIT').
        Trim()
}

$citationText = Read-Text 'CITATION.cff'
$codeMetaText = Read-Text 'codemeta.json'
$zenodoText = Read-Text '.zenodo.json'
$guideText = Read-Text 'docs/research/citation.md'
$paperText = Read-Text 'paper/paper.md'
$paperBibText = Read-Text 'paper/paper.bib'

$codeMeta = $null
$zenodo = $null
try {
    $codeMeta = $codeMetaText | ConvertFrom-Json
}
catch {
    Add-Failure "codemeta.json is not valid JSON: $($_.Exception.Message)"
}
try {
    $zenodo = $zenodoText | ConvertFrom-Json
}
catch {
    Add-Failure ".zenodo.json is not valid JSON: $($_.Exception.Message)"
}

$cffRequired = @(
    'cff-version',
    'message',
    'title',
    'version',
    'date-released',
    'type',
    'abstract',
    'license',
    'repository-code'
)

$cff = @{}
foreach ($field in $cffRequired) {
    $cff[$field] = Require-CffScalar $citationText $field
}

foreach ($block in @('authors', 'keywords')) {
    if ($citationText -notmatch "(?m)^${block}:\s*$") {
        Add-Failure "CITATION.cff missing list field: $block"
    }
}

if ($null -ne $codeMeta) {
    foreach ($field in @('@context', '@type', 'name', 'description', 'version', 'datePublished', 'programmingLanguage', 'license', 'codeRepository', 'developmentStatus')) {
        Require-JsonField $codeMeta $field 'codemeta.json' | Out-Null
    }
    if ($codeMeta.'@context' -ne 'https://doi.org/10.5063/schema/codemeta-3.0') {
        Add-Failure "codemeta.json @context must use the CodeMeta 3.0 crosswalk"
    }
}

if ($null -ne $zenodo) {
    foreach ($field in @('title', 'upload_type', 'version', 'publication_date', 'access_right', 'description', 'creators', 'license', 'keywords')) {
        Require-JsonField $zenodo $field '.zenodo.json' | Out-Null
    }
}

if ($null -ne $codeMeta -and $null -ne $zenodo) {
    if ($cff['version'] -ne $codeMeta.version -or $cff['version'] -ne $zenodo.version) {
        Add-Failure "Version mismatch across citation metadata: CITATION=$($cff['version']), codemeta=$($codeMeta.version), zenodo=$($zenodo.version)"
    }
    if ($cff['date-released'] -ne $codeMeta.datePublished -or $cff['date-released'] -ne $zenodo.publication_date) {
        Add-Failure "Release date mismatch across citation metadata: CITATION=$($cff['date-released']), codemeta=$($codeMeta.datePublished), zenodo=$($zenodo.publication_date)"
    }
    if ($cff['repository-code'] -ne $codeMeta.codeRepository) {
        Add-Failure "Repository URL mismatch: CITATION=$($cff['repository-code']), codemeta=$($codeMeta.codeRepository)"
    }
    if ($cff['title'] -ne $zenodo.title) {
        Add-Failure "Title mismatch: CITATION=$($cff['title']), zenodo=$($zenodo.title)"
    }
    if ($cff['license'] -ne $zenodo.license) {
        Add-Failure "License mismatch: CITATION=$($cff['license']), zenodo=$($zenodo.license)"
    }
    $citationLicense = Normalize-License $cff['license']
    $codeMetaLicense = Normalize-License $codeMeta.license
    if ($citationLicense -ne $codeMetaLicense) {
        Add-Failure "CodeMeta license mismatch: CITATION=$($cff['license']), codemeta=$($codeMeta.license)"
    }
}

$requiredNeedles = @(
    $cff['version'],
    $cff['repository-code'],
    'CITATION.cff',
    'codemeta.json',
    '.zenodo.json',
    'Zenodo',
    'DOI',
    'not yet DOI-minted'
)

foreach ($needle in $requiredNeedles) {
    if (-not [string]::IsNullOrWhiteSpace($needle) -and $guideText -notlike "*$needle*") {
        Add-Failure "docs/research/citation.md missing required text: $needle"
    }
}

foreach ($needle in @($cff['title'], $cff['repository-code'], $cff['version'], 'KairoECS contributors')) {
    if (-not [string]::IsNullOrWhiteSpace($needle) -and ($paperText + $paperBibText) -notlike "*$needle*") {
        Add-Failure "paper metadata missing required text: $needle"
    }
}

if ($failures.Count -gt 0) {
    Write-Host "Track 19 citation/archive validation FAILED"
    foreach ($failure in $failures) {
        Write-Host "- $failure"
    }
    exit 1
}

Write-Host "Track 19 citation/archive validation passed"
Write-Host "version=$($cff['version'])"
Write-Host "repository=$($cff['repository-code'])"
Write-Host "archive_status=pre-release metadata seed, not yet DOI-minted"
