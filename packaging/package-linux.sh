#!/usr/bin/env bash
# Builds PresentCraft for Linux with its bundled ffmpeg and makes a .tar.gz.
#
#   packaging/build-ffmpeg.sh linux     # small LGPL build (recommended), or
#   FETCH_FFMPEG=1 packaging/package-linux.sh   # use BtbN's ready-made LGPL build (bigger)
#   packaging/package-linux.sh
#
# Output: dist/presentcraft-<version>-linux-<arch>.tar.gz containing
#   PresentCraft, ffmpeg/ffmpeg, licenses/, install.sh (menu entry and .prcraft file type)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
ff="$here/vendor/ffmpeg/linux"
arch="$(uname -m)"

if [[ ! -x "$ff/ffmpeg" && "${FETCH_FFMPEG:-}" == 1 ]]; then
  case "$arch" in
    x86_64) asset=linux64 ;;
    aarch64) asset=linuxarm64 ;;
    *) echo "No ready-made ffmpeg for $arch; run build-ffmpeg.sh." >&2; exit 1 ;;
  esac
  name="ffmpeg-n9.0-latest-$asset-lgpl-9.0"
  mkdir -p "$here/build" "$ff"
  echo "Downloading $name…"
  curl -fL --retry 3 -o "$here/build/$name.tar.xz" \
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/$name.tar.xz"
  tar -xf "$here/build/$name.tar.xz" -C "$here/build"
  cp "$here/build/$name/bin/ffmpeg" "$ff/ffmpeg"
  cp "$here/build/$name/LICENSE.txt" "$ff/COPYING.LGPLv2.1"
  echo "BtbN FFmpeg-Builds $name (LGPL), https://github.com/BtbN/FFmpeg-Builds" >"$ff/BUILD-INFO.txt"
fi
if [[ ! -x "$ff/ffmpeg" ]]; then
  echo "No bundled ffmpeg: run packaging/build-ffmpeg.sh linux (or set FETCH_FFMPEG=1)." >&2
  exit 1
fi

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
echo "Building PresentCraft $version (release)…"
(cd "$root" && cargo build --locked --release)

name="presentcraft-$version-linux-$arch"
dir="$root/dist/$name"
rm -rf "$dir"
mkdir -p "$dir/ffmpeg" "$dir/licenses"
cp "$root/target/release/PresentCraft" "$dir/PresentCraft"
cp "$ff/ffmpeg" "$dir/ffmpeg/ffmpeg"
cp "$here/FFMPEG-NOTICE.txt" "$ff/COPYING.LGPLv2.1" "$ff/BUILD-INFO.txt" "$dir/licenses/"
# Desktop integration assets; install under ~/.local/share or /usr/share.
mkdir -p "$dir/share/applications" "$dir/share/mime/packages"
cp "$here/app.presentcraft.PresentCraft.desktop" "$dir/share/applications/"
cp "$here/linux/app.presentcraft.PresentCraft.xml" "$dir/share/mime/packages/"
cp "$here/linux/install.sh" "$dir/install.sh"
for size in 16 24 32 48 64 128 256 512 1024; do
  icons="$dir/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$icons"
  cp "$here/icons/$size.png" "$icons/app.presentcraft.PresentCraft.png"
done
tar -czf "$root/dist/$name.tar.gz" -C "$root/dist" "$name"
echo "Done: $root/dist/$name.tar.gz ($(du -h "$root/dist/$name.tar.gz" | cut -f1))"
