param(
    [string]$LedgerPath = "conductor/phase-closeout.yaml",
    [switch]$RequireCleanWorkingTree
)

$ErrorActionPreference = "Stop"
$issues = @()

function Add-Issue {
    param([string]$Message)
    $script:issues += $Message
}

function Invoke-Git {
    param([string[]]$GitArgs)
    $output = & git @GitArgs 2>&1
    return [PSCustomObject]@{
        ExitCode = $LASTEXITCODE
        Output = @($output)
    }
}

function Get-LedgerEntries {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        Add-Issue "Missing phase closeout ledger: $Path"
        return @()
    }

    $content = Get-Content -LiteralPath $Path -Raw
    $entries = @()
    $matches = [regex]::Matches($content, '(?ms)^\s*-\s*track_id:\s*"([^"]+)"\s*$.*?(?=^\s*-\s*track_id:\s*"[^"]+"\s*$|\z)')
    foreach ($match in $matches) {
        $block = $match.Value
        $state = ""
        if ($block -match '(?m)^\s*state:\s*(.+?)\s*$') {
            $state = $Matches[1].Trim().Trim('"')
        }
        $commitSha = ""
        if ($block -match '(?m)^\s*commit_sha:\s*"([^"]+)"\s*$') {
            $commitSha = $Matches[1].Trim()
        }
        $pushedRef = ""
        if ($block -match '(?m)^\s*pushed_ref:\s*"([^"]+)"\s*$') {
            $pushedRef = $Matches[1].Trim()
        }
        $gitStatus = ""
        if ($block -match '(?m)^\s*git_status:\s*(.+?)\s*$') {
            $gitStatus = $Matches[1].Trim().Trim('"')
        }
        $entries += [PSCustomObject]@{
            TrackId = $match.Groups[1].Value
            State = $state
            CommitSha = $commitSha
            PushedRef = $pushedRef
            GitStatus = $gitStatus
        }
    }
    return $entries
}

Write-Host "=== Validate Conductor Git Closeout ===" -ForegroundColor Cyan

$inside = Invoke-Git -GitArgs @("rev-parse", "--is-inside-work-tree")
if ($inside.ExitCode -ne 0 -or ($inside.Output -join "").Trim() -ne "true") {
    throw "Not inside a git working tree"
}

$entries = @(Get-LedgerEntries -Path $LedgerPath)
if ($entries.Count -eq 0) {
    Add-Issue "No closeout ledger entries parsed from $LedgerPath"
}

foreach ($entry in $entries) {
    if ($entry.State -notin @("closed", "legacy_review_status")) {
        continue
    }

    if ($entry.CommitSha -notmatch '^[0-9a-f]{40}$') {
        Add-Issue "Track $($entry.TrackId) ledger commit is not a 40-character SHA: $($entry.CommitSha)"
        continue
    }

    $commitExists = Invoke-Git -GitArgs @("cat-file", "-e", "$($entry.CommitSha)^{commit}")
    if ($commitExists.ExitCode -ne 0) {
        Add-Issue "Track $($entry.TrackId) ledger commit does not exist locally: $($entry.CommitSha)"
        continue
    }

    if ([string]::IsNullOrWhiteSpace($entry.PushedRef)) {
        Add-Issue "Track $($entry.TrackId) ledger entry has no pushed_ref"
        continue
    }

    $refExists = Invoke-Git -GitArgs @("rev-parse", "--verify", "--quiet", $entry.PushedRef)
    if ($refExists.ExitCode -ne 0) {
        Add-Issue "Track $($entry.TrackId) pushed_ref does not resolve locally: $($entry.PushedRef)"
        continue
    }

    $isAncestor = Invoke-Git -GitArgs @("merge-base", "--is-ancestor", $entry.CommitSha, $entry.PushedRef)
    if ($isAncestor.ExitCode -ne 0) {
        Add-Issue "Track $($entry.TrackId) ledger commit $($entry.CommitSha) is not contained in $($entry.PushedRef)"
    }
}

if ($RequireCleanWorkingTree) {
    $status = Invoke-Git -GitArgs @("status", "--porcelain")
    if ($status.ExitCode -ne 0) {
        Add-Issue "git status failed: $($status.Output -join '; ')"
    } elseif (@($status.Output).Count -gt 0) {
        Add-Issue "Working tree has uncommitted tracked or untracked changes; closeout requires a clean tree"
    }
}

if ($issues.Count -gt 0) {
    Write-Host "$($issues.Count) error(s)" -ForegroundColor Red
    $issues | ConvertTo-Json
    exit 1
}

Write-Host "0 error(s)" -ForegroundColor Green
Write-Host "Conductor git closeout validation passed." -ForegroundColor Green
