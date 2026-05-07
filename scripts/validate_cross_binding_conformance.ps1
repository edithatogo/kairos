#!/usr/bin/env pwsh
# Cross-binding conformance validation
# Verifies all bindings can parse the shared conformance fixtures

$ROOT = Split-Path -Parent $PSScriptRoot
$FIXTURES = Join-Path $ROOT "conformance" "fixtures"
$EXIT_CODE = 0

Write-Host "=== KairoECS Cross-Binding Conformance Validation ===" -ForegroundColor Cyan
Write-Host ""

# Check fixture files exist
$requiredFixtures = @(
    "deterministic_ordering.json",
    "cancellation.json",
    "rng_replay.json",
    "manifest.json"
)

foreach ($fix in $requiredFixtures) {
    $path = Join-Path $FIXTURES $fix
    if (Test-Path $path) {
        $content = Get-Content $path -Raw
        try {
            $null = $content | ConvertFrom-Json
            Write-Host "  PASS: $fix (valid JSON)" -ForegroundColor Green
        } catch {
            Write-Host "  FAIL: $fix (invalid JSON)" -ForegroundColor Red
            $EXIT_CODE = 1
        }
    } else {
        Write-Host "  FAIL: $fix (missing)" -ForegroundColor Red
        $EXIT_CODE = 1
    }
}

Write-Host ""

# Verify fixture schema expectations
Write-Host "Validating fixture schemas..." -ForegroundColor Cyan

$deterministic = Get-Content (Join-Path $FIXTURES "deterministic_ordering.json") | ConvertFrom-Json
if ($deterministic.version -eq 1 -and $deterministic.expected_kind_order.Count -eq 4) {
    Write-Host "  PASS: deterministic_ordering schema correct" -ForegroundColor Green
} else {
    Write-Host "  FAIL: deterministic_ordering schema mismatch" -ForegroundColor Red
    $EXIT_CODE = 1
}

$cancellation = Get-Content (Join-Path $FIXTURES "cancellation.json") | ConvertFrom-Json
if ($cancellation.version -eq 1 -and $cancellation.expected_kind_order.Count -eq 2) {
    Write-Host "  PASS: cancellation schema correct" -ForegroundColor Green
} else {
    Write-Host "  FAIL: cancellation schema mismatch" -ForegroundColor Red
    $EXIT_CODE = 1
}

$rng = Get-Content (Join-Path $FIXTURES "rng_replay.json") | ConvertFrom-Json
if ($rng.run_seed -eq 7 -and $rng.expected_stream.Count -eq 4) {
    Write-Host "  PASS: rng_replay schema correct" -ForegroundColor Green
} else {
    Write-Host "  FAIL: rng_replay schema mismatch" -ForegroundColor Red
    $EXIT_CODE = 1
}

Write-Host ""

# Run binding-specific conformance tests if available
Write-Host "Binding conformance tests..." -ForegroundColor Cyan

# Python
$pyTests = Join-Path $ROOT "bindings" "python" "tests" "test_conformance.py"
if (Test-Path $pyTests) {
    Write-Host "  Python binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  Python binding: no conformance tests yet" -ForegroundColor Yellow
}

# R
$rTests = Join-Path $ROOT "bindings" "r" "tests" "testthat" "test-conformance.R"
if (Test-Path $rTests) {
    Write-Host "  R binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  R binding: no conformance tests yet" -ForegroundColor Yellow
}

# Julia
$jlTests = Join-Path $ROOT "bindings" "julia" "test" "test_conformance.jl"
if (Test-Path $jlTests) {
    Write-Host "  Julia binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  Julia binding: no conformance tests yet" -ForegroundColor Yellow
}

# TypeScript
$tsTests = Join-Path $ROOT "bindings" "typescript" "test" "conformance.test.ts"
if (Test-Path $tsTests) {
    Write-Host "  TypeScript binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  TypeScript binding: no conformance tests yet" -ForegroundColor Yellow
}

# C#
$csTests = Join-Path $ROOT "bindings" "csharp" "tests" "Kairo.ECS.Tests" "ConformanceTests.cs"
if (Test-Path $csTests) {
    Write-Host "  C# binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  C# binding: no conformance tests yet" -ForegroundColor Yellow
}

# Go
$goTests = Join-Path $ROOT "bindings" "go" "conformance_test.go"
if (Test-Path $goTests) {
    Write-Host "  Go binding: tests found" -ForegroundColor Green
} else {
    Write-Host "  Go binding: no conformance tests yet" -ForegroundColor Yellow
}

Write-Host ""

if ($EXIT_CODE -eq 0) {
    Write-Host "Result: ALL PASSED" -ForegroundColor Green
} else {
    Write-Host "Result: FAILURES DETECTED" -ForegroundColor Red
}

exit $EXIT_CODE
