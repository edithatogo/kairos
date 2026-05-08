param(
  [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
$errors = New-Object System.Collections.Generic.List[string]

function Add-Failure([string]$Message) {
  $errors.Add($Message) | Out-Null
}

function Assert-File([string]$RelativePath) {
  $path = Join-Path $repoRoot $RelativePath
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    Add-Failure "Missing required file: $RelativePath"
  }
  return $path
}

function Assert-Contains([string]$RelativePath, [string]$Needle) {
  $path = Assert-File $RelativePath
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    return
  }
  $content = Get-Content -LiteralPath $path -Raw
  if (-not $content.Contains($Needle)) {
    Add-Failure "Expected '$Needle' in $RelativePath"
  }
}

function Assert-JsonField([string]$RelativePath, [scriptblock]$Predicate, [string]$Message) {
  $path = Assert-File $RelativePath
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    return
  }
  $json = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
  if (-not (& $Predicate $json)) {
    Add-Failure $Message
  }
}

Write-Host 'Validating binding Tracks 06-11 deterministic facade and metadata controls...'

# Python: cancellation must reject unknown, duplicate, and already-dispatched events without native artifacts.
Assert-Contains 'bindings/python/kairo_ecs/_scheduler.py' 'def _is_pending'
Assert-Contains 'bindings/python/kairo_ecs/_scheduler.py' 'not self._is_pending(event_id)'
Assert-Contains 'bindings/python/tests/test_scheduler.py' 'test_scheduler_rejects_unknown_duplicate_and_dispatched_cancellation'
Assert-Contains 'bindings/python/pyproject.toml' 'name = "kairo-ecs"'
Assert-Contains 'bindings/python/pyproject.toml' 'license = "Apache-2.0"'

# R: cancellation must fail for non-pending events while staying dependency-light.
Assert-Contains 'bindings/r/R/kairoecs.R' 'Event is not pending and cannot be cancelled.'
Assert-Contains 'bindings/r/tests/testthat/test-smoke.R' 'scheduler rejects unknown duplicate and dispatched cancellation'
Assert-Contains 'bindings/r/tests/smoke-base.R' 'not pending'
Assert-Contains 'bindings/r/DESCRIPTION' 'Package: kairoECS'

# Julia: this slice remains a pure package facade with explicit native-not-configured status.
Assert-Contains 'bindings/julia/src/KairoECS.jl' 'ffi_status()'
Assert-Contains 'bindings/julia/src/KairoECS.jl' 'configured = false'
Assert-Contains 'bindings/julia/Project.toml' 'name = "KairoECS"'

# TypeScript/Wasm: cancellation parity must be visible in snapshots and event-log rows.
Assert-Contains 'bindings/typescript/src/index.ts' 'cancel(eventId: bigint | number | string): boolean'
Assert-Contains 'bindings/typescript/src/index.ts' 'cancelledEvents'
Assert-Contains 'bindings/typescript/test/index.test.ts' 'scheduler.cancel(cancelled.eventId)'
Assert-Contains 'crates/kairo-ecs-wasm/src/lib.rs' 'pub const EVENT_LOG_SCHEMA'
Assert-JsonField 'bindings/typescript/package.json' {
  param($json)
  $json.name -eq '@kairo-ecs/typescript' -and $json.license -eq 'Apache-2.0'
} 'TypeScript package metadata must keep the expected package name and Apache-2.0 license.'

# C#: existing managed facade must keep explicit cancellation and no native-runtime overclaiming.
Assert-Contains 'bindings/csharp/src/Kairo.ECS/DeterministicScheduler.cs' 'public bool Cancel'
Assert-Contains 'bindings/csharp/src/Kairo.ECS/NativeBinding.cs' 'Native FFI is not configured'
Assert-Contains 'bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj' '<TargetFrameworks>net10.0;net11.0</TargetFrameworks>'
Assert-Contains 'bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj' '<PackageLicenseExpression>Apache-2.0</PackageLicenseExpression>'

# Go: cancellation must reject unknown, duplicate, and already-dispatched events.
Assert-Contains 'bindings/go/kairoecs.go' 'foundPending := false'
Assert-Contains 'bindings/go/kairoecs_test.go' 'TestCancellationRejectsUnknownDuplicateAndDispatchedEvent'
Assert-Contains 'bindings/go/go.mod' 'module github.com/edithatogo/kairos/bindings/go'

if ($errors.Count -gt 0) {
  Write-Host "Binding Tracks 06-11 validation failed with $($errors.Count) issue(s):"
  foreach ($errorItem in $errors) {
    Write-Host " - $errorItem"
  }
  exit 1
}

Write-Host 'Binding Tracks 06-11 validation passed.'
Write-Host 'Native FFI runtime loading, registry publication, and unavailable R/Julia execution remain intentionally out of scope.'
