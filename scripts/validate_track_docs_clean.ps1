param(
    [string[]]$TrackRoots = @("conductor/tracks")
)

$ErrorActionPreference = "Stop"

$bannedPatterns = @(
    "TBD by subagent",
    "TBD\.",
    "CI command placeholder",
    "placeholder docs",
    "placeholder text",
    "placeholder exits",
    "still a future surface; skipping",
    "not present yet",
    "Add fixture stubs for conformance where relevant\.",
    "Add minimal tests that prove the scaffold is wired into CI\.",
    "Add minimal tests that prove the scaffold is wired into CI through the shared workspace checks\.",
    "Add conformance fixture tests where relevant\.",
    "Add benchmarks where performance-sensitive\."
)

$offenders = New-Object System.Collections.Generic.List[string]

foreach ($root in $TrackRoots) {
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -File -Include *.md) {
        $content = Get-Content -LiteralPath $file.FullName -Raw
        foreach ($pattern in $bannedPatterns) {
            if ($content -match $pattern) {
                $offenders.Add("$($file.FullName):$pattern")
                break
            }
        }
    }
}

if ($offenders.Count -gt 0) {
    $message = $offenders -join "`n"
    throw "Track docs still contain banned placeholder text:`n$message"
}

Write-Host "Track docs are placeholder-clean."
