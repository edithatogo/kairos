$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
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

$tutorials = @(
    'docs/tutorials/index.md',
    'docs/tutorials/rust-getting-started.md',
    'docs/tutorials/python-getting-started.md',
    'docs/tutorials/wasm-getting-started.md',
    'docs/tutorials/model-building.md'
)

foreach ($path in $tutorials) {
    Assert-Path $path
}

foreach ($path in @(
    'examples/docs/README.md',
    'docs/community/README.md',
    'docs/community/adoption.md',
    'docs/community/model-zoo.md',
    'docs/tutorials/coverage-matrix.md',
    'docs/developer-experience/docs-platform.md',
    'bindings/python/README.md',
    'bindings/typescript/README.md',
    'crates/kairo-ecs-core/src/lib.rs'
)) {
    Assert-Path $path
}

Assert-Contains 'docs/tutorials/index.md' 'Learning paths' 'tutorial learning path table'
Assert-Contains 'docs/tutorials/index.md' 'Claim boundary' 'tutorial claim boundary'
Assert-Contains 'docs/tutorials/rust-getting-started.md' 'cargo check --workspace' 'Rust validation command'
Assert-Contains 'docs/tutorials/python-getting-started.md' 'Scheduler.cancel' 'Python cancellation contract'
Assert-Contains 'docs/tutorials/wasm-getting-started.md' 'not-configured' 'Wasm not-configured boundary'
Assert-Contains 'docs/tutorials/model-building.md' 'model card' 'model-building model card guidance'
Assert-Contains 'docs/tutorials/model-building.md' 'M/M/1 queue' 'DES example link text'
Assert-Contains 'docs/tutorials/model-building.md' 'Factory bottleneck' 'factory example link text'
Assert-Contains 'docs/tutorials/model-building.md' 'Flocking' 'ABM example link text'
Assert-Contains 'docs/tutorials/model-building.md' 'Emergency department flow' 'hybrid example link text'
Assert-Contains 'docs/tutorials/index.md' 'coverage-matrix.md' 'tutorial coverage matrix cross-link'
Assert-Contains 'docs/tutorials/index.md' 'Learning Coverage Matrix' 'tutorial coverage matrix label'
Assert-Contains 'docs/tutorials/coverage-matrix.md' 'Learning Coverage Matrix' 'coverage matrix title'
Assert-Contains 'docs/tutorials/coverage-matrix.md' 'Docs platform status' 'docs platform note'
Assert-Contains 'docs/tutorials/coverage-matrix.md' 'The repository does not require a notebook for every language' 'notebook exclusion note'

Assert-Contains 'examples/docs/README.md' 'docs/tutorials/index.md' 'examples docs tutorial index cross-link'
Assert-Contains 'docs/community/README.md' 'docs-tutorials' 'community docs-tutorials gate'
Assert-Contains 'docs/community/adoption.md' 'docs/tutorials/index.md' 'adoption tutorial path cross-link'
Assert-Contains 'docs/developer-experience/docs-platform.md' 'Astro/Starlight' 'docs platform roadmap target'
Assert-Contains 'docs/developer-experience/docs-platform.md' 'custom Node' 'docs platform current site'

foreach ($path in $tutorials) {
    $content = Get-Content -LiteralPath $path -Raw
    foreach ($match in [regex]::Matches($content, '\[[^\]]+\]\(([^)#][^)]+)\)')) {
        $target = $match.Groups[1].Value
        if ($target -match '^[a-z]+:' -or $target.StartsWith('#')) {
            continue
        }

        $targetPath = $target.Split('#')[0]
        if ([string]::IsNullOrWhiteSpace($targetPath)) {
            continue
        }

        $resolved = Join-Path (Split-Path -Parent $path) $targetPath
        if (-not (Test-Path -LiteralPath $resolved)) {
            throw "Broken tutorial link in ${path}: $target"
        }
    }
}

Write-Host 'tutorial_docs_status=ok'
Write-Host 'tutorial_gate=docs-tutorials'
