param(
    [string]$TracksYamlPath = "conductor/tracks.yaml",
    [string]$TrackDirsPath = "conductor/tracks",
    [string]$TrackId = "",
    [switch]$ReportOnly
)

$ErrorActionPreference = "Stop"

$requiredArtifacts = @(
    "spec.md",
    "plan.md",
    "agent-contract.md",
    "risk-register.md",
    "test-matrix.md",
    "handoff.md"
)

$advancingStatuses = @("In Progress", "In Review", "Done")

function ConvertTo-TrackId {
    param([string]$Value)
    return ([int]$Value).ToString("00")
}

function Add-Issue {
    param(
        [System.Collections.Generic.List[object]]$Issues,
        [string]$Gate,
        [string]$Code,
        [string]$Message
    )
    $Issues.Add([PSCustomObject]@{
        gate = $Gate
        code = $Code
        message = $Message
    })
}

function Read-TrackInventory {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Track inventory not found: $Path"
    }

    $lines = Get-Content -LiteralPath $Path
    $tracks = [ordered]@{}
    $current = $null
    $section = $null

    foreach ($line in $lines) {
        if ($line -match '^\s*-\s*id:\s*(\d+)\s*$') {
            if ($null -ne $current) {
                $tracks[$current.id] = [PSCustomObject]$current
            }
            $current = [ordered]@{
                id = ConvertTo-TrackId $matches[1]
                name = ""
                status = ""
                owner = ""
                depends_on = @()
                required_gates = @()
            }
            $section = $null
            continue
        }

        if ($null -eq $current) {
            continue
        }

        if ($line -match '^\s*name:\s*(.+?)\s*$') {
            $current.name = $matches[1].Trim('"').Trim()
            $section = $null
            continue
        }

        if ($line -match '^\s*status:\s*(.+?)\s*$') {
            $current.status = $matches[1].Trim('"').Trim()
            $section = $null
            continue
        }

        if ($line -match '^\s*owner:\s*(.+?)\s*$') {
            $current.owner = $matches[1].Trim('"').Trim()
            $section = $null
            continue
        }

        if ($line -match '^\s*depends_on:\s*\[(.*?)\]\s*$') {
            $depText = $matches[1]
            $current.depends_on = @(
                [regex]::Matches($depText, '"?(\d+)"?') |
                    ForEach-Object { ConvertTo-TrackId $_.Groups[1].Value }
            )
            $section = $null
            continue
        }

        if ($line -match '^\s*required_gates:\s*$') {
            $section = "required_gates"
            continue
        }

        if ($line -match '^\s*[a-zA-Z_]+:\s*') {
            $section = $null
            continue
        }

        if ($section -eq "required_gates" -and $line -match '^\s*-\s*(.+?)\s*$') {
            $current.required_gates += $matches[1].Trim('"').Trim()
        }
    }

    if ($null -ne $current) {
        $tracks[$current.id] = [PSCustomObject]$current
    }

    return $tracks
}

function Test-Cycles {
    param($Tracks, [System.Collections.Generic.List[object]]$Issues)

    $state = @{}
    foreach ($id in $Tracks.Keys) {
        $state[$id] = "white"
    }

    function Visit {
        param([string]$Id, [string[]]$Path)

        $state[$Id] = "gray"
        foreach ($dep in $Tracks[$Id].depends_on) {
            if (-not $Tracks.Contains($dep)) {
                continue
            }
            if ($state[$dep] -eq "gray") {
                $start = [Array]::IndexOf($Path, $dep)
                $cyclePath = @($Path[$start..($Path.Count - 1)] + $dep)
                Add-Issue $Issues "dependency-closure-check" "dependency-cycle" "Dependency cycle detected: $($cyclePath -join ' -> ')"
                continue
            }
            if ($state[$dep] -eq "white") {
                Visit $dep @($Path + $dep)
            }
        }
        $state[$Id] = "black"
    }

    foreach ($id in $Tracks.Keys) {
        if ($state[$id] -eq "white") {
            Visit $id @($id)
        }
    }
}

function Get-DependencyClosure {
    param($Tracks, [string]$TrackId)

    $seen = [ordered]@{}
    function Walk {
        param([string]$Id)
        foreach ($dep in $Tracks[$Id].depends_on) {
            if (-not $Tracks.Contains($dep)) {
                continue
            }
            if (-not $seen.Contains($dep)) {
                $seen[$dep] = $true
                Walk $dep
            }
        }
    }

    Walk $TrackId
    return @($seen.Keys)
}

function Get-WaveNumber {
    param($Tracks, [string]$TrackId, [hashtable]$Memo, [hashtable]$Stack = @{})

    if ($Memo.ContainsKey($TrackId)) {
        return $Memo[$TrackId]
    }

    if ($Stack.ContainsKey($TrackId)) {
        return 0
    }
    $Stack[$TrackId] = $true

    $deps = @($Tracks[$TrackId].depends_on | Where-Object { $Tracks.Contains($_) })
    if ($deps.Count -eq 0) {
        $Memo[$TrackId] = 0
        $Stack.Remove($TrackId)
        return 0
    }

    $maxDepWave = -1
    foreach ($dep in $deps) {
        $depWave = Get-WaveNumber $Tracks $dep $Memo $Stack
        if ($depWave -gt $maxDepWave) {
            $maxDepWave = $depWave
        }
    }
    $Memo[$TrackId] = $maxDepWave + 1
    $Stack.Remove($TrackId)
    return $Memo[$TrackId]
}

function Get-TrackDir {
    param([string]$TrackDirsPath, [string]$TrackId)

    if (-not (Test-Path -LiteralPath $TrackDirsPath)) {
        return $null
    }

    return Get-ChildItem -LiteralPath $TrackDirsPath -Directory |
        Where-Object { $_.Name -match "^$TrackId-" } |
        Select-Object -First 1
}

$issues = [System.Collections.Generic.List[object]]::new()
$tracks = Read-TrackInventory $TracksYamlPath
$targetTrackIds = @($tracks.Keys)

if (-not [string]::IsNullOrWhiteSpace($TrackId)) {
    $normalizedTrackId = ConvertTo-TrackId $TrackId
    if (-not $tracks.Contains($normalizedTrackId)) {
        Add-Issue $issues "wave-progression-check" "unknown-target-track" "Target track $normalizedTrackId does not exist in $TracksYamlPath."
        $targetTrackIds = @()
    } else {
        $targetTrackIds = @($normalizedTrackId)
    }
}

foreach ($id in $tracks.Keys) {
    $track = $tracks[$id]

    if ([string]::IsNullOrWhiteSpace($track.owner)) {
        Add-Issue $issues "wave-progression-check" "missing-owner" "Track $id has no declared owner."
    }

    if (@($track.required_gates).Count -eq 0) {
        Add-Issue $issues "wave-progression-check" "missing-required-gates" "Track $id has no required gates."
    }

    foreach ($dep in $track.depends_on) {
        if (-not $tracks.Contains($dep)) {
            Add-Issue $issues "dependency-closure-check" "unknown-dependency" "Track $id depends on unknown track $dep."
        }
    }

    $trackDir = Get-TrackDir $TrackDirsPath $id
    if ($null -eq $trackDir) {
        Add-Issue $issues "wave-progression-check" "missing-track-directory" "Track $id has no directory under $TrackDirsPath."
    } else {
        foreach ($artifact in $requiredArtifacts) {
            $artifactPath = Join-Path $trackDir.FullName $artifact
            if (-not (Test-Path -LiteralPath $artifactPath)) {
                Add-Issue $issues "wave-progression-check" "missing-required-artifact" "Track $id is missing required artifact $($trackDir.Name)/$artifact."
            }
        }
    }

    if (($targetTrackIds -contains $id) -and $track.status -in $advancingStatuses) {
        foreach ($dep in $track.depends_on) {
            if ($tracks.Contains($dep) -and $tracks[$dep].status -ne "Done") {
                Add-Issue $issues "wave-progression-check" "direct-dependency-not-done" "Track $id is $($track.status) but direct dependency $dep is $($tracks[$dep].status), not Done."
            }
        }

        foreach ($dep in (Get-DependencyClosure $tracks $id)) {
            if ($tracks[$dep].status -ne "Done") {
                Add-Issue $issues "dependency-closure-check" "transitive-dependency-not-done" "Track $id is $($track.status) but transitive dependency $dep is $($tracks[$dep].status), not Done."
            }
        }
    }
}

Test-Cycles $tracks $issues

$waveMemo = @{}
$waves = [ordered]@{}
foreach ($id in $tracks.Keys) {
    $wave = Get-WaveNumber $tracks $id $waveMemo
    $waveKey = "Wave $wave"
    if (-not $waves.Contains($waveKey)) {
        $waves[$waveKey] = @()
    }
    $waves[$waveKey] += $id
}

$criticalPath = @(foreach ($id in $tracks.Keys) {
    $downstream = @()
    foreach ($candidate in $tracks.Keys) {
        if ($candidate -eq $id) {
            continue
        }
        if ((Get-DependencyClosure $tracks $candidate) -contains $id) {
            $downstream += $candidate
        }
    }

    [PSCustomObject]@{
        track = $id
        name = $tracks[$id].name
        wave = $waveMemo[$id]
        direct_dependents = @($tracks.Keys | Where-Object { $tracks[$_].depends_on -contains $id }).Count
        transitive_dependents = @($downstream).Count
        status = $tracks[$id].status
    }
}) | Sort-Object -Property @{ Expression = "transitive_dependents"; Descending = $true }, @{ Expression = "direct_dependents"; Descending = $true }, "track"

$report = [PSCustomObject]@{
    generated_at = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssK")
    source = $TracksYamlPath
    target_track_ids = @($targetTrackIds)
    track_count = $tracks.Count
    wave_membership = [PSCustomObject]$waves
    critical_path_heatmap = @($criticalPath)
    issues = @($issues)
}

$report | ConvertTo-Json -Depth 8

if ((-not $ReportOnly) -and $issues.Count -gt 0) {
    exit 1
}
