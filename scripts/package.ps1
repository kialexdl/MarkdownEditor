[CmdletBinding()]
param(
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-NativeCommand([string]$Name, [string[]]$Arguments) {
    & $Name @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE."
    }
}

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$cargoManifest = Join-Path $projectRoot "src-tauri\Cargo.toml"

& (Join-Path $PSScriptRoot "bootstrap.ps1")

Push-Location $projectRoot
try {
    if (-not $SkipTests) {
        Invoke-NativeCommand "pnpm" @("check")
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $cargoManifest, "--all", "--", "--check")
        Invoke-NativeCommand "cargo" @("clippy", "--manifest-path", $cargoManifest, "--all-targets", "--", "-D", "warnings")
        Invoke-NativeCommand "cargo" @("test", "--manifest-path", $cargoManifest)
    }

    Invoke-NativeCommand "pnpm" @("tauri", "build", "--no-bundle")
} finally {
    Pop-Location
}

$version = (Get-Content "$projectRoot\package.json" | ConvertFrom-Json).version
$artifactRoot = Join-Path $projectRoot "artifacts\$version\windows-x64-portable"
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

$releaseExecutable = Join-Path $projectRoot "src-tauri\target\release\markdown-editor.exe"
if (-not (Test-Path $releaseExecutable -PathType Leaf)) {
    throw "Release executable was not found: $releaseExecutable"
}

$zipName = "MarkdownEditor-$version-windows-x64.zip"
$zipPath = Join-Path $artifactRoot $zipName
$checksumPath = Join-Path $artifactRoot "SHA256SUMS.txt"
$stagingRoot = Join-Path $artifactRoot ".portable-staging"
$portableExecutable = Join-Path $stagingRoot "MarkdownEditor.exe"

if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}
if (Test-Path $checksumPath) {
    Remove-Item $checksumPath -Force
}
if (Test-Path $stagingRoot) {
    Remove-Item $stagingRoot -Recurse -Force
}

try {
    New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
    Copy-Item $releaseExecutable $portableExecutable -Force
    Compress-Archive -LiteralPath $portableExecutable -DestinationPath $zipPath -CompressionLevel Optimal
} finally {
    if (Test-Path $stagingRoot) {
        Remove-Item $stagingRoot -Recurse -Force
    }
}

$hash = Get-FileHash $zipPath -Algorithm SHA256
"$($hash.Hash.ToLowerInvariant())  $zipName" | Set-Content $checksumPath -Encoding ascii

& (Join-Path $PSScriptRoot "verify-package.ps1") -ArtifactDirectory $artifactRoot
Write-Host "Portable release ZIP was created at: $zipPath"
