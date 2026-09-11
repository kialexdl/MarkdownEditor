[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Require-Command([string]$Name, [string]$Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required command: $Name. $Hint"
    }
}

function Invoke-NativeCommand([string]$Name, [string[]]$Arguments) {
    & $Name @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE."
    }
}

Require-Command "git" "Install Git for Windows and add it to PATH."
Require-Command "node" "Install Node.js 20.19 or later."
Require-Command "pnpm" "Run corepack enable, then install the pnpm version required by this project."
Require-Command "cargo" "Install stable Rust and the MSVC target through rustup."
Require-Command "rustc" "Install stable Rust and the MSVC target through rustup."

$nodeVersionText = (& node --version).Trim().TrimStart("v")
$nodeVersion = [Version]$nodeVersionText
if ($nodeVersion -lt [Version]"20.19.0") {
    throw "Node.js 20.19 or later is required."
}

Write-Host (git --version)
Write-Host (node --version)
Write-Host (pnpm --version)
Write-Host (rustc --version)

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $projectRoot
try {
    if (Test-Path (Join-Path $projectRoot "pnpm-lock.yaml")) {
        Invoke-NativeCommand "pnpm" @("install", "--frozen-lockfile")
    } else {
        Write-Warning "pnpm-lock.yaml is not present. Dependencies will be resolved and a lockfile will be generated; commit it to the repository."
        Invoke-NativeCommand "pnpm" @("install", "--no-frozen-lockfile")
    }
} finally {
    Pop-Location
}
Write-Host "Dependencies are ready. Run 'pnpm tauri dev' to start the development environment."
