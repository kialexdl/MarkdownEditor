[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$resolved = (Resolve-Path $ArtifactDirectory).Path
$packages = @(Get-ChildItem $resolved -File -Filter "*.zip")
if ($packages.Count -ne 1) {
    throw "Expected exactly one portable ZIP package, but found $($packages.Count)."
}
if ($packages[0].Length -lt 100KB) {
    throw "Package is unexpectedly small: $($packages[0].Name)"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($packages[0].FullName)
try {
    $entries = @($archive.Entries | Where-Object { -not $_.FullName.EndsWith("/") })
    if ($entries.Count -ne 1 -or $entries[0].FullName -ne "MarkdownEditor.exe") {
        throw "Portable ZIP must contain exactly one root file named MarkdownEditor.exe."
    }
    if ($entries[0].Length -lt 100KB) {
        throw "MarkdownEditor.exe inside the ZIP is unexpectedly small."
    }
} finally {
    $archive.Dispose()
}

$checksumPath = Join-Path $resolved "SHA256SUMS.txt"
if (-not (Test-Path $checksumPath -PathType Leaf)) {
    throw "SHA256SUMS.txt is missing."
}
$actualHash = (Get-FileHash $packages[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$checksumText = Get-Content $checksumPath -Raw
if (-not $checksumText.Contains("$actualHash  $($packages[0].Name)")) {
    throw "SHA256SUMS.txt does not match the portable ZIP."
}

Write-Host "Portable ZIP structure and checksum validation passed."
