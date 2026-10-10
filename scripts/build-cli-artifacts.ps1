param(
    [switch]$Windows,
    [switch]$LinuxGlibc,
    [switch]$LinuxMusl
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$rustDir = Join-Path $repoRoot "rust"
$distDir = Join-Path $repoRoot "dist"

if (-not ($Windows -or $LinuxGlibc -or $LinuxMusl)) {
    $Windows = $true
    $LinuxGlibc = $true
    $LinuxMusl = $true
}

function Invoke-LoggedStep {
    param(
        [string]$Message,
        [scriptblock]$Action
    )

    Write-Host "==> $Message"
    & $Action
}

function Invoke-WslCargoBuild {
    param(
        [string]$CargoArgs
    )

    $escapedRustDir = $rustDir.Replace("\", "/").Replace("D:", "/mnt/d")
    $command = "source `"`$HOME/.cargo/env`" && cd `"$escapedRustDir`" && cargo build $CargoArgs"
    wsl bash -lc $command
    if ($LASTEXITCODE -ne 0) {
        throw "WSL cargo build failed: cargo build $CargoArgs"
    }
}

function Copy-Artifact {
    param(
        [string]$Source,
        [string]$DestinationDir,
        [string]$FileName
    )

    New-Item -ItemType Directory -Force $DestinationDir | Out-Null
    Copy-Item -Force $Source (Join-Path $DestinationDir $FileName)
}

if ($Windows) {
    Invoke-LoggedStep "Building Windows release CLI" {
        cargo build --manifest-path (Join-Path $rustDir "Cargo.toml") -p slicer-converter-cli --release
        if ($LASTEXITCODE -ne 0) {
            throw "Windows cargo build failed"
        }
    }

    Invoke-LoggedStep "Publishing Windows artifact to dist/windows" {
        Copy-Artifact `
            -Source (Join-Path $rustDir "target/release/slicer-converter.exe") `
            -DestinationDir (Join-Path $distDir "windows") `
            -FileName "slicer-converter.exe"
    }
}

if ($LinuxGlibc) {
    Invoke-LoggedStep "Building Linux glibc release CLI in WSL" {
        Invoke-WslCargoBuild "-p slicer-converter-cli --release"
    }

    Invoke-LoggedStep "Publishing Linux glibc artifact to dist/linux-glibc" {
        Copy-Artifact `
            -Source (Join-Path $rustDir "target/release/slicer-converter") `
            -DestinationDir (Join-Path $distDir "linux-glibc") `
            -FileName "slicer-converter"
    }
}

if ($LinuxMusl) {
    Invoke-LoggedStep "Building Linux musl release CLI in WSL" {
        Invoke-WslCargoBuild "-p slicer-converter-cli --release --target x86_64-unknown-linux-musl"
    }

    Invoke-LoggedStep "Publishing Linux musl artifact to dist/linux-musl" {
        Copy-Artifact `
            -Source (Join-Path $rustDir "target/x86_64-unknown-linux-musl/release/slicer-converter") `
            -DestinationDir (Join-Path $distDir "linux-musl") `
            -FileName "slicer-converter"
    }
}
