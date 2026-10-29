$ErrorActionPreference = "Stop"
$source = Join-Path $PSScriptRoot "../conductor_track_inventory.ps1"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$errors)
if ($errors.Count -gt 0) { throw "Setup validator parse errors: $errors" }
$definition = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -ceq "Get-ConductorTrackDirectories"
}, $true))
if ($definition.Count -ne 1) { throw "Expected one track inventory function" }
Invoke-Expression $definition[0].Extent.Text

$root = Join-Path ([IO.Path]::GetTempPath()) ("kairos-track-inventory-" + [guid]::NewGuid())
$active = Join-Path $root "tracks"
$archive = Join-Path $root "archive"
function Assert-Rejected {
    param([string]$Message)
    try {
        Get-ConductorTrackDirectories -ExpectedTrackIds @("105", "106") -ActivePath $active -ArchivePath $archive | Out-Null
    } catch {
        if ($_.Exception.Message -notlike "*$Message*") { throw }
        return
    }
    throw "Invalid inventory was accepted: $Message"
}
try {
    New-Item -ItemType Directory -Path (Join-Path $active "105-current"), (Join-Path $archive "106-closed") -Force | Out-Null
    $found = @(Get-ConductorTrackDirectories -ExpectedTrackIds @("105", "106") -ActivePath $active -ArchivePath $archive)
    if ($found.Count -ne 2 -or $found.FullName -notcontains (Join-Path $archive "106-closed")) {
        throw "Archived track was omitted"
    }
    New-Item -ItemType Directory -Path (Join-Path $active "106-duplicate") | Out-Null
    Assert-Rejected "found 3"
    Remove-Item -LiteralPath (Join-Path $active "106-duplicate")
    New-Item -ItemType Directory -Path (Join-Path $archive "107-unregistered") | Out-Null
    Assert-Rejected "Extra: 107"
    Remove-Item -LiteralPath (Join-Path $archive "107-unregistered")
    Remove-Item -LiteralPath (Join-Path $archive "106-closed")
    Assert-Rejected "Missing: 106"
    Write-Host "Track inventory: archived inclusion and duplicate/extra/missing rejection passed."
} finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
