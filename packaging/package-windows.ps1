# Builds PresentCraft for Windows with its bundled ffmpeg and makes a .zip.
# Run in PowerShell from the app folder:
#
#   powershell -ExecutionPolicy Bypass -File packaging\package-windows.ps1 [-Arch x64|arm64]
#
# -Arch defaults to this computer's architecture. The build uses Rust's
# x86_64-pc-windows-msvc or aarch64-pc-windows-msvc target; add it with
# `rustup target add <target>` if it is not this computer's own.
#
# ffmpeg.exe comes from packaging\vendor\ffmpeg\windows (made with
# `packaging/build-ffmpeg.sh windows` on macOS/Linux - small, x64 only), or,
# when that is missing, from BtbN's ready-made LGPL build (bigger, downloaded here).
#
# Output: dist\PresentCraft-<version>-windows-<arch>.zip containing
#   PresentCraft.exe, ffmpeg\ffmpeg.exe, licenses\, register-file-types.ps1
param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch
)
$ErrorActionPreference = "Stop"

if (-not $Arch) {
    $os = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $Arch = if ($os -eq [System.Runtime.InteropServices.Architecture]::Arm64) { "arm64" } else { "x64" }
}

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here
$ff = if ($Arch -eq "x64") { Join-Path $here "vendor\ffmpeg\windows" } else { Join-Path $here "vendor\ffmpeg\windows-$Arch" }

if (-not (Test-Path (Join-Path $ff "ffmpeg.exe"))) {
    $asset = if ($Arch -eq "arm64") { "winarm64" } else { "win64" }
    $name = "ffmpeg-n9.0-latest-$asset-lgpl-9.0"
    $build = Join-Path $here "build"
    New-Item -ItemType Directory -Force -Path $build, $ff | Out-Null
    $zipPath = Join-Path $build "$name.zip"
    Write-Host "Downloading $name..."
    Invoke-WebRequest -Uri "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/$name.zip" -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $build -Force
    Copy-Item (Join-Path $build "$name\bin\ffmpeg.exe") (Join-Path $ff "ffmpeg.exe")
    Copy-Item (Join-Path $build "$name\LICENSE.txt") (Join-Path $ff "COPYING.LGPLv2.1")
    Set-Content (Join-Path $ff "BUILD-INFO.txt") "BtbN FFmpeg-Builds $name (LGPL), https://github.com/BtbN/FFmpeg-Builds"
}

$target = if ($Arch -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }

$version = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version\s*=\s*"(.*)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "Building PresentCraft $version (release, $target)..."
Push-Location $root
cargo build --locked --release --target $target
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed" }
Pop-Location

$name = "PresentCraft-$version-windows-$Arch"
$dir = Join-Path $root "dist\$name"
if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
New-Item -ItemType Directory -Force -Path (Join-Path $dir "ffmpeg"), (Join-Path $dir "licenses") | Out-Null
Copy-Item (Join-Path $root "target\$target\release\PresentCraft.exe") (Join-Path $dir "PresentCraft.exe")
Copy-Item (Join-Path $here "windows\register-file-types.ps1") (Join-Path $dir "register-file-types.ps1")
Copy-Item (Join-Path $ff "ffmpeg.exe") (Join-Path $dir "ffmpeg\ffmpeg.exe")
Copy-Item (Join-Path $here "FFMPEG-NOTICE.txt"), (Join-Path $ff "COPYING.LGPLv2.1"), (Join-Path $ff "BUILD-INFO.txt") (Join-Path $dir "licenses")

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $dir -DestinationPath $zip
Write-Host "Done: $zip"
