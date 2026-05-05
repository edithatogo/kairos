$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $RepoRoot

function Assert-Path {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required path: $Path"
    }
}

function Assert-Contains {
    param([string]$Path, [string]$Pattern, [string]$Description)
    if (-not (Select-String -LiteralPath $Path -Pattern $Pattern -Quiet)) {
        throw "Missing $Description in $Path"
    }
}

Assert-Path 'website/package.json'
Assert-Path 'website/docs-link-manifest.json'
Assert-Path 'website/src/index.md'

$package = Get-Content -LiteralPath 'website/package.json' -Raw | ConvertFrom-Json
foreach ($script in @('build', 'check:links', 'dev')) {
    if (-not $package.scripts.$script) {
        throw "website/package.json is missing script: $script"
    }
}
foreach ($script in @('check:quality', 'check:all')) {
    if (-not $package.scripts.$script) {
        throw "website/package.json is missing SOTA docs script: $script"
    }
}

$manifest = Get-Content -LiteralPath 'website/docs-link-manifest.json' -Raw | ConvertFrom-Json
foreach ($path in $manifest.requiredPaths) {
    Assert-Path $path
}
foreach ($path in $manifest.siteSources) {
    Assert-Path $path
}
foreach ($path in $manifest.quality.requiredBuildOutputs) {
    if (-not ($path -like 'website/build/*')) {
        throw "Unexpected docs build output outside website/build: $path"
    }
}
if (-not $manifest.quality.minimumGeneratedPages -or $manifest.quality.minimumGeneratedPages -lt 20) {
    throw "Docs quality gate must require at least 20 generated source pages"
}
foreach ($section in $manifest.navigationSections) {
    if (-not $section.title) {
        throw "Docs navigation section is missing a title"
    }
    if (-not $section.links -or $section.links.Count -eq 0) {
        throw "Docs navigation section has no links: $($section.title)"
    }
    foreach ($link in $section.links) {
        Assert-Path $link.path
    }
}

foreach ($pattern in @(
    'docs/adr',
    'docs/README',
    'docs/community',
    'docs/release',
    'docs/research',
    'docs/benchmarks',
    'docs/trustworthy-simulation',
    'bindings/python',
    'bindings/r',
    'bindings/julia',
    'bindings/typescript',
    'bindings/csharp',
    'bindings/go',
    'Tutorials and Examples',
    'Jupyter notebooks and figures',
    'check:quality'
)) {
    Assert-Contains 'website/src/index.md' ([regex]::Escape($pattern)) $pattern
}

Write-Host "track14_status=ok"
Write-Host "required_paths=$($manifest.requiredPaths.Count)"
Write-Host "site_sources=$($manifest.siteSources.Count)"
Write-Host "navigation_sections=$($manifest.navigationSections.Count)"
Write-Host "minimum_generated_pages=$($manifest.quality.minimumGeneratedPages)"
