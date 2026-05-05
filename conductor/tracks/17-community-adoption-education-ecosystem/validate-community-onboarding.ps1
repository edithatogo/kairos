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

foreach ($path in @(
    'docs/community/README.md',
    'docs/community/adoption.md',
    'docs/community/contributor-onboarding.md',
    'docs/community/governance.md',
    'docs/community/model-zoo.md',
    'docs/community/roadmap.md',
    'examples/model-zoo/README.md',
    'examples/model-zoo/model-zoo.yaml',
    'website/src/index.md',
    'conductor/tracks/17-community-adoption-education-ecosystem/community-plan.md',
    'conductor/tracks/17-community-adoption-education-ecosystem/test-matrix.md'
)) {
    Assert-Path $path
}

Assert-Contains 'docs/community/README.md' 'onboarding-docs' 'onboarding-docs gate'
Assert-Contains 'docs/community/adoption.md' 'First-user path' 'first-user path'
Assert-Contains 'docs/community/contributor-onboarding.md' 'First contribution path' 'first contribution path'
Assert-Contains 'docs/community/model-zoo.md' 'Inventory update rule' 'inventory update rule'
Assert-Contains 'docs/community/roadmap.md' 'stable' 'maturity labels'

foreach ($id in @('mm1_queue', 'factory_bottleneck', 'flocking', 'emergency_department_flow')) {
    Assert-Contains 'docs/community/model-zoo.md' $id "model-zoo docs id $id"
    Assert-Contains 'examples/model-zoo/model-zoo.yaml' $id "model-zoo inventory id $id"
}

foreach ($label in @('alpha', 'beta', 'stable', 'preview')) {
    Assert-Contains 'website/src/index.md' $label "website maturity label $label"
}

Write-Host "track17_status=ok"
Write-Host 'community_gate=onboarding-docs'
