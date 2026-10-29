function Get-ConductorTrackDirectories {
    param(
        [string[]]$ExpectedTrackIds,
        [string]$ActivePath = "conductor/tracks",
        [string]$ArchivePath = "conductor/archive"
    )

    $directories = @(Get-ChildItem -LiteralPath $ActivePath -Directory)
    if (Test-Path -LiteralPath $ArchivePath) {
        $directories += @(Get-ChildItem -LiteralPath $ArchivePath -Directory)
    }
    $actualIds = @($directories | ForEach-Object {
        if ($_.Name -match '^(\d+)-') {
            "{0:D2}" -f [int]$matches[1]
        } else {
            throw "Track directory name does not start with a numeric id: $($_.Name)"
        }
    } | Sort-Object -Unique)

    $missingIds = @($ExpectedTrackIds | Where-Object { $actualIds -notcontains $_ })
    $extraIds = @($actualIds | Where-Object { $ExpectedTrackIds -notcontains $_ })
    if ($missingIds.Count -gt 0 -or $extraIds.Count -gt 0) {
        throw "Track directory ids do not match conductor/tracks.yaml. Missing: $($missingIds -join ', '); Extra: $($extraIds -join ', ')"
    }
    if ($directories.Count -ne $ExpectedTrackIds.Count) {
        throw "Expected $($ExpectedTrackIds.Count) track directories from conductor/tracks.yaml, found $($directories.Count)"
    }
    return $directories
}

