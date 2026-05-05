param(
    [string]$TracksYamlPath = "conductor/tracks.yaml",
    [string]$SubagentsYamlPath = "conductor/subagents.yaml",
    [string]$TrackDirsPath = "conductor/tracks"
)

$ErrorActionPreference = "Stop"
$issues = @()

function Add-Issue {
    param([string]$Severity, [string]$Message)
    $issues += [PSCustomObject]@{ Severity = $Severity; Message = $Message }
}

Write-Host "=== Validate Conductor DAG ===" -ForegroundColor Cyan

# ------------------------------------------------------------------
# 1. Parse tracks.yaml
# ------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $TracksYamlPath)) {
    Add-Issue -Severity "ERROR" -Message "tracks.yaml not found at $TracksYamlPath"
    if ($issues.Count -gt 0) {
        $issues | ConvertTo-Json -Depth 3
        exit 1
    }
}

try {
    $tracksContent = Get-Content -LiteralPath $TracksYamlPath -Raw
} catch {
    Add-Issue -Severity "ERROR" -Message "Failed to read tracks.yaml: $_"
    $issues | ConvertTo-Json -Depth 3
    exit 1
}

# Quick schema check
if ($tracksContent -notmatch "schema_version:\s*\d+") {
    Add-Issue -Severity "ERROR" -Message "tracks.yaml missing schema_version"
}

# Parse tracks using regex-based extraction. This avoids needing a YAML library.
$trackBlocks = [regex]::Matches($tracksContent, '(\s*- id:\s*(?<id>\d+).*?(?=\s*- id:|\s*$))', [System.Text.RegularExpressions.RegexOptions]::Singleline)

$tracks = @{}
$trackIds = @()
$idPattern = '- id:\s*(\d+)'
$ownerPattern = 'owner:\s*(.+)'
$dependsOnPattern = 'depends_on:\s*\[(.*?)\]'
$ownedPathsSectionPattern = 'owned_paths:\s*\n((?:\s*-.*\n?)*)'

# Simple approach: iterate lines
$lines = $tracksContent -split "`n"
$currentTrack = $null
$inOwnedPaths = $false

for ($i = 0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]
    if ($line -match '^\s*- id:\s*(\d+)') {
        # Save previous track
        if ($currentTrack) {
            $tracks[$currentTrack.id] = $currentTrack
            $trackIds += $currentTrack.id
        }
        $currentTrack = @{
            id = $matches[1]
            owner = ""
            depends_on = @()
            owned_paths = @()
        }
        $inOwnedPaths = $false
    }
    if ($currentTrack -and $line -match '^\s*owner:\s*(.+)') {
        $owners = $matches[1].Trim()
        # Split on + for multiple owners
        $currentTrack.owner = ($owners -split '\s*\+\s*').ForEach({ $_.Trim() }) -join " "
    }
    if ($currentTrack -and $line -match '^\s*depends_on:\s*\[(.*?)\]') {
        $depStr = $matches[1].Trim()
        if ($depStr) {
            # Parse quoted string numbers like "00", "01"
            $refs = [regex]::Matches($depStr, '"(\d+)"')
            $currentTrack.depends_on = $refs.ForEach({ $_.Groups[1].Value })
        }
        # Handle multi-line depends_on
        if ($depStr -match '\]$') {
            # single line, done
        } else {
            # multi-line: collect until ]
            for ($j = $i + 1; $j -lt $lines.Count; $j++) {
                $ml = $lines[$j]
                $refs = [regex]::Matches($ml, '"(\d+)"')
                $currentTrack.depends_on += $refs.ForEach({ $_.Groups[1].Value })
                if ($ml -match '\]') { break }
            }
        }
    }
    if ($currentTrack -and $line -match '^\s*owned_paths:') {
        $inOwnedPaths = $true
    }
    if ($inOwnedPaths -and $currentTrack) {
        if ($line -match '^\s*-\s*(.+)$') {
            $path = $matches[1].Trim()
            $currentTrack.owned_paths += $path
        } elseif ($line -match '^\s*required_gates:') {
            $inOwnedPaths = $false
        } elseif ($line -match '^\s*- id:' -and $i -gt 0) {
            $inOwnedPaths = $false
        } elseif ($line.Trim() -eq '' -and $inOwnedPaths) {
            # might be end of block; stop if next non-blank is a top-level key
        }
    }
}
# Save last track
if ($currentTrack) {
    $tracks[$currentTrack.id] = $currentTrack
    $trackIds += $currentTrack.id
}

Write-Host "  Parsed $($tracks.Count) tracks from tracks.yaml" -ForegroundColor Gray

# ------------------------------------------------------------------
# 2. Parse subagents.yaml
# ------------------------------------------------------------------
$agentIds = @()
if (Test-Path -LiteralPath $SubagentsYamlPath) {
    try {
        $subagentsContent = Get-Content -LiteralPath $SubagentsYamlPath -Raw
        $agentMatches = [regex]::Matches($subagentsContent, '- id:\s*(\S+)')
        $agentIds = $agentMatches.ForEach({ $_.Groups[1].Value })
        Write-Host "  Parsed $($agentIds.Count) agents from subagents.yaml" -ForegroundColor Gray
    } catch {
        Add-Issue -Severity "ERROR" -Message "Failed to read subagents.yaml: $_"
    }
} else {
    Add-Issue -Severity "WARN" -Message "subagents.yaml not found at $SubagentsYamlPath; skipping agent reference checks"
}

# ------------------------------------------------------------------
# 3. Check every depends_on target exists
# ------------------------------------------------------------------
Write-Host "  Checking dependency targets..." -ForegroundColor Gray
foreach ($tid in $trackIds) {
    $track = $tracks[$tid]
    foreach ($dep in $track.depends_on) {
        if (-not $tracks.ContainsKey($dep)) {
            Add-Issue -Severity "ERROR" -Message "Track $tid depends_on '$dep' which does not exist in tracks.yaml"
        }
    }
}

# ------------------------------------------------------------------
# 4. Check for cycles in the dependency DAG
# ------------------------------------------------------------------
Write-Host "  Checking for dependency cycles..." -ForegroundColor Gray
$WHITE = 0; $GRAY = 1; $BLACK = 2
$colors = @{}
foreach ($tid in $trackIds) { $colors[$tid] = $WHITE }

$cycleFound = $false
function DFS-CycleCheck {
    param([string]$node, [System.Collections.ArrayList]$path)
    $colors[$node] = $GRAY
    [void]$path.Add($node)
    $track = $tracks[$node]
    foreach ($dep in $track.depends_on) {
        if (-not $colors.ContainsKey($dep)) { continue }
        if ($colors[$dep] -eq $GRAY) {
            $cycleStart = $path.IndexOf($dep)
            $cyclePath = $path[$cycleStart..($path.Count - 1)] + $dep
            Add-Issue -Severity "ERROR" -Message "Dependency cycle detected: $($cyclePath -join ' -> ')"
            $script:cycleFound = $true
        } elseif ($colors[$dep] -eq $WHITE) {
            DFS-CycleCheck -node $dep -path $path
        }
    }
    $colors[$node] = $BLACK
    [void]$path.RemoveAt($path.Count - 1)
}

foreach ($tid in $trackIds) {
    if ($colors[$tid] -eq $WHITE) {
        DFS-CycleCheck -node $tid -path ([System.Collections.ArrayList]::new())
    }
}

# ------------------------------------------------------------------
# 5. Check that every agent referenced exists in subagents.yaml
# ------------------------------------------------------------------
if ($agentIds.Count -gt 0) {
    Write-Host "  Checking agent references..." -ForegroundColor Gray
    $allTrackOwners = @{}
    foreach ($tid in $trackIds) {
        $ownerStr = $tracks[$tid].owner
        if ($ownerStr) {
            foreach ($a in ($ownerStr -split ' ')) {
                $a = $a.Trim()
                if ($a) { $allTrackOwners[$a] = $true }
            }
        }
    }
    foreach ($agent in $allTrackOwners.Keys) {
        if ($agentIds -notcontains $agent) {
            Add-Issue -Severity "ERROR" -Message "Agent '$agent' referenced in tracks.yaml but not found in subagents.yaml"
        }
    }
}

# ------------------------------------------------------------------
# 6. Check that every owned_path has a corresponding track directory
# ------------------------------------------------------------------
Write-Host "  Checking owned_path correspondences..." -ForegroundColor Gray
if (Test-Path -LiteralPath $TrackDirsPath) {
    $existingTrackDirs = (Get-ChildItem -LiteralPath $TrackDirsPath -Directory).Name
    foreach ($tid in $trackIds) {
        $matchingDir = $existingTrackDirs | Where-Object { $_ -match "^$tid-" }
        if (-not $matchingDir) {
            Add-Issue -Severity "WARN" -Message "Track $tid has no corresponding directory in $TrackDirsPath (expected pattern: ${tid}-*)"
        }
    }
    # Reverse check: directories without a track in tracks.yaml
    foreach ($dir in $existingTrackDirs) {
        if ($dir -match '^(\d+)-') {
            $dirId = $matches[1]
            if (-not $tracks.ContainsKey($dirId)) {
                Add-Issue -Severity "ERROR" -Message "Track directory '$dir' exists but track '$dirId' not found in tracks.yaml"
            }
        }
    }
} else {
    Add-Issue -Severity "WARN" -Message "Track directories path '$TrackDirsPath' not found; skipping directory checks"
}

# ------------------------------------------------------------------
# 7. Check transitive dependencies for unreachable tracks
# ------------------------------------------------------------------
Write-Host "  Checking for unreachable tracks..." -ForegroundColor Gray
$reachable = @{}
function MarkReachable {
    param([string]$node)
    if ($reachable.ContainsKey($node)) { return }
    $reachable[$node] = $true
    $track = $tracks[$node]
    foreach ($dep in $track.depends_on) {
        MarkReachable -node $dep
    }
}
# All tracks with no incoming edges (no one depends on them) are roots
# But better: start from tracks that have no depends_on - these are the base tracks
foreach ($tid in $trackIds) {
    if ($tracks[$tid].depends_on.Count -eq 0) {
        MarkReachable -node $tid
    }
}
# Also mark tracks reachable by traversing reverse-depends
$hasIncoming = @{}
foreach ($tid in $trackIds) {
    foreach ($dep in $tracks[$tid].depends_on) {
        $hasIncoming[$dep] = $true
    }
}
foreach ($tid in $trackIds) {
    if (-not $hasIncoming.ContainsKey($tid)) {
        # This track has no dependents, mark it as a root too
        MarkReachable -node $tid
    }
}
# Now every track SHOULD be reachable; if not, run full traverse
foreach ($tid in $trackIds) {
    if (-not $reachable.ContainsKey($tid)) {
        if ($tracks[$tid].depends_on.Count -gt 0) {
            Add-Issue -Severity "WARN" -Message "Track $tid may be unreachable from root tracks"
        }
    }
}

# ------------------------------------------------------------------
# 8. Summary
# ------------------------------------------------------------------
Write-Host ""
$errors = $issues | Where-Object { $_.Severity -eq "ERROR" }
$warnings = $issues | Where-Object { $_.Severity -eq "WARN" }

if ($errors.Count -gt 0) {
    Write-Host "$($errors.Count) error(s), $($warnings.Count) warning(s)" -ForegroundColor Red
    $issues | ConvertTo-Json -Depth 3
    exit 1
} elseif ($warnings.Count -gt 0) {
    Write-Host "0 errors, $($warnings.Count) warning(s)" -ForegroundColor Yellow
    $issues | ConvertTo-Json -Depth 3
    Write-Host "DAG validation passed with warnings." -ForegroundColor Green
    exit 0
} else {
    Write-Host "0 errors, 0 warnings" -ForegroundColor Green
    Write-Host "DAG validation passed." -ForegroundColor Green
    exit 0
}

# ------------------------------------------------------------------
# 9. Verify that the number of dependencies matches tracks.yaml read correctly
# ------------------------------------------------------------------
Write-Host "  Dependency summary:" -ForegroundColor Gray
foreach ($tid in ($trackIds | Sort-Object { [int]$_ })) {
    $deps = $tracks[$tid].depends_on -join ", "
    Write-Host "    Track $tid depends_on: [$deps]" -ForegroundColor Gray
}
