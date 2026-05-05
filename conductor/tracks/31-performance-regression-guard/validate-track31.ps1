param(
  [switch]$KeepReports
)

$ErrorActionPreference = 'Stop'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$ReportDir = Join-Path $RepoRoot '.tmp\track31-regression-guard'

function Invoke-RepoCommand {
  param(
    [Parameter(Mandatory = $true)][string[]]$Command,
    [Parameter(Mandatory = $true)][int]$ExpectedExitCode
  )

  Push-Location $RepoRoot
  try {
    & $Command[0] @($Command[1..($Command.Length - 1)])
    $exitCode = if ($LASTEXITCODE -eq $null) { 0 } else { $LASTEXITCODE }
    if ($exitCode -ne $ExpectedExitCode) {
      throw "Expected exit code $ExpectedExitCode from '$($Command -join ' ')' but got $exitCode"
    }
  } finally {
    Pop-Location
  }
}

function Read-JsonReport {
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    throw "Missing expected report: $Path"
  }
  return Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

$thresholdReport = Join-Path $ReportDir 'threshold-report.json'
Invoke-RepoCommand -Command @('python', 'benches/regression/compare.py', '--report', $thresholdReport) -ExpectedExitCode 0
$threshold = Read-JsonReport $thresholdReport
if ($threshold.status -ne 'pass') {
  throw 'threshold-definition-exists positive gate did not pass'
}

$positiveReport = Join-Path $ReportDir 'positive-comparison-report.json'
Invoke-RepoCommand -Command @(
  'python', 'benches/regression/compare.py',
  '--base', 'benches/regression/sample-base.json',
  '--current', 'benches/regression/sample-current.json',
  '--report', $positiveReport
) -ExpectedExitCode 0
$positive = Read-JsonReport $positiveReport
if ($positive.status -ne 'pass') {
  throw 'benchmark-regression-check positive fixture did not pass'
}
foreach ($row in $positive.comparison) {
  foreach ($required in @('benchmark', 'base_mean', 'current_mean', 'change_percent', 'threshold_percent', 'gate', 'status')) {
    if (-not ($row.PSObject.Properties.Name -contains $required)) {
      throw "comparison row for $($row.benchmark) is missing '$required'"
    }
  }
}

$regressionReport = Join-Path $ReportDir 'blocking-regression-report.json'
Invoke-RepoCommand -Command @(
  'python', 'benches/regression/compare.py',
  '--base', 'benches/regression/sample-base.json',
  '--current', 'benches/regression/sample-current-regression.json',
  '--report', $regressionReport
) -ExpectedExitCode 1
$regression = Read-JsonReport $regressionReport
$failedSchedule = @($regression.comparison | Where-Object { $_.benchmark -eq 'schedule_1m_events' -and $_.status -eq 'fail' })
if ($regression.status -ne 'fail' -or $failedSchedule.Count -ne 1) {
  throw 'blocking regression fixture did not fail with schedule_1m_events details'
}

$unknownReport = Join-Path $ReportDir 'unknown-id-report.json'
Invoke-RepoCommand -Command @(
  'python', 'benches/regression/compare.py',
  '--base', 'benches/regression/sample-base.json',
  '--current', 'benches/regression/sample-current-unknown.json',
  '--report', $unknownReport
) -ExpectedExitCode 1
$unknown = Read-JsonReport $unknownReport
if (-not (@($unknown.result_id_coverage.unknown_current_results) -contains 'preview_unregistered_benchmark')) {
  throw 'unknown benchmark ID fixture did not report preview_unregistered_benchmark'
}

$missingThresholds = Join-Path $ReportDir 'missing-thresholds.md'
$sourceThresholds = Join-Path $RepoRoot 'conductor\performance-thresholds.md'
$filtered = Get-Content -LiteralPath $sourceThresholds | Where-Object { $_ -notmatch 'hybrid_des_abm_smoke_100k' }
Set-Content -LiteralPath $missingThresholds -Value $filtered -Encoding utf8
$missingReport = Join-Path $ReportDir 'missing-threshold-report.json'
Invoke-RepoCommand -Command @(
  'python', 'benches/regression/compare.py',
  '--thresholds', $missingThresholds,
  '--report', $missingReport
) -ExpectedExitCode 1
$missing = Read-JsonReport $missingReport
if (-not (@($missing.coverage.missing_thresholds) -contains 'hybrid_des_abm_smoke_100k')) {
  throw 'missing threshold fixture did not report hybrid_des_abm_smoke_100k'
}

if (-not $KeepReports) {
  Remove-Item -LiteralPath $ReportDir -Recurse -Force
}

Write-Host 'Track 31 regression guard validation passed'
