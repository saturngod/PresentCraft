# Builds PresentCraft for Windows with its bundled ffmpeg and makes a .zip.
# Run in PowerShell from the app folder:
#
#   powershell -ExecutionPolicy Bypass -File packaging\package-windows.ps1
#
# ffmpeg.exe comes from packaging\vendor\ffmpeg\windows (made with
# `packaging/build-ffmpeg.sh windows` on macOS/Linux - small), or, when that
# is missing, from BtbN's ready-made LGPL build (bigger, downloaded here).
#
# Output: dist\PresentCraft-<version>-windows-x64.zip containing
#   PresentCraft.exe, ffmpeg\ffmpeg.exe, licenses\, register-file-types.ps1
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here
$ff = Join-Path $here "vendor\ffmpeg\windows"

if (-not (Test-Path (Join-Path $ff "ffmpeg.exe"))) {
    $name = "ffmpeg-n9.0-latest-win64-lgpl-9.0"
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

$version = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version\s*=\s*"(.*)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "Building PresentCraft $version (release)..."
Push-Location $root
cargo build --locked --release
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed" }
Pop-Location

$name = "PresentCraft-$version-windows-x64"
$dir = Join-Path $root "dist\$name"
if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
New-Item -ItemType Directory -Force -Path (Join-Path $dir "ffmpeg"), (Join-Path $dir "licenses") | Out-Null
Copy-Item (Join-Path $root "target\release\PresentCraft.exe") (Join-Path $dir "PresentCraft.exe")
Copy-Item (Join-Path $here "windows\register-file-types.ps1") (Join-Path $dir "register-file-types.ps1")
Copy-Item (Join-Path $ff "ffmpeg.exe") (Join-Path $dir "ffmpeg\ffmpeg.exe")
Copy-Item (Join-Path $here "FFMPEG-NOTICE.txt"), (Join-Path $ff "COPYING.LGPLv2.1"), (Join-Path $ff "BUILD-INFO.txt") (Join-Path $dir "licenses")

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $dir -DestinationPath $zip
Write-Host "Done: $zip"
