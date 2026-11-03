#!/usr/bin/env bash
# Builds the small ffmpeg that PresentCraft bundles for inline video.
#
#   packaging/build-ffmpeg.sh            # for this computer (macOS or Linux)
#   packaging/build-ffmpeg.sh windows    # Windows x64, cross-built with MinGW-w64
#
# The build is LGPL (no --enable-gpl, no external libraries) and only has
# what the app uses: MP4/MOV/M4V and WebM/MKV video, H.264, HEVC, VP8/VP9,
# MPEG-4, ProRes and MJPEG pictures, AAC/MP3/Opus/Vorbis/ALAC/PCM sound,
# the scale and colour filters, raw-frame output, JPEG stills and WAV sound.
#
# Output: packaging/vendor/ffmpeg/<platform>/ffmpeg[.exe] plus BUILD-INFO.txt
# and the LGPL licence text, which the package scripts copy into releases.
#
# Needs: curl, tar, make and a C compiler (Xcode Command Line Tools on macOS,
# build-essential on Linux). For Windows: x86_64-w64-mingw32-gcc
# (`brew install mingw-w64` or `apt install mingw-w64`).
set -euo pipefail

FFMPEG_VERSION="9.0"
FFMPEG_SHA256="7f607a00dd0d28a729d5a4811205812eef01cf6ef6155025febb6f36a9062d52"

here="$(cd "$(dirname "$0")" && pwd)"
target="${1:-}"
if [[ -z "$target" ]]; then
  case "$(uname -s)" in
    Darwin) target=macos ;;
    Linux) target=linux ;;
    *) echo "Pass macos, linux or windows." >&2; exit 1 ;;
  esac
fi

work="$here/build/ffmpeg-$target"
out="$here/vendor/ffmpeg/$target"
src_tar="$here/build/ffmpeg-$FFMPEG_VERSION.tar.xz"
mkdir -p "$here/build" "$out"

if [[ ! -f "$src_tar" ]]; then
  echo "Downloading FFmpeg $FFMPEG_VERSION source…"
  curl -fL --retry 5 --retry-all-errors --retry-delay 10 -o "$src_tar.part" "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
  mv "$src_tar.part" "$src_tar"
fi
actual="$( (shasum -a 256 "$src_tar" 2>/dev/null || sha256sum "$src_tar") | cut -d' ' -f1)"
if [[ "$actual" != "$FFMPEG_SHA256" ]]; then
  echo "Checksum mismatch for $src_tar (got $actual)." >&2
  exit 1
fi

rm -rf "$work"
mkdir -p "$work"
tar -xf "$src_tar" -C "$work" --strip-components 1

flags=(
  --disable-everything --disable-autodetect --disable-doc --disable-debug
  --disable-network --disable-ffplay --disable-ffprobe
  --enable-static --disable-shared --enable-pic
  --enable-protocol=file,pipe
  --enable-demuxer=mov,matroska,avi,mpegts,wav,mp3
  --enable-decoder=h264,hevc,vp8,vp9,mpeg4,prores,mjpeg,png,aac,aac_latm,mp3,mp3float,opus,vorbis,alac,flac,pcm_s16le,pcm_s24le,pcm_f32le
  --enable-parser=h264,hevc,vp8,vp9,mpeg4video,aac,aac_latm,mpegaudio,opus,vorbis,flac,mjpeg
  --enable-filter=scale,format,null,anull,hue,colorchannelmixer,colorlevels,gblur,aresample,aformat,transpose,hflip,vflip,rotate
  --enable-encoder=rawvideo,mjpeg,pcm_s16le
  --enable-muxer=rawvideo,image2,wav
  --enable-swscale --enable-swresample
)

case "$target" in
  macos)
    export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
    ;;
  linux)
    ;;
  windows)
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null; then
      echo "x86_64-w64-mingw32-gcc not found. Install MinGW-w64 first." >&2
      exit 1
    fi
    flags+=(--enable-cross-compile --target-os=mingw32 --arch=x86_64
            --cross-prefix=x86_64-w64-mingw32- --extra-ldflags=-static)
    ;;
  *) echo "Unknown target $target" >&2; exit 1 ;;
esac

echo "Configuring FFmpeg $FFMPEG_VERSION for ${target}…"
(cd "$work" && ./configure "${flags[@]}" >"$work/configure.log" 2>&1) || {
  tail -30 "$work/configure.log" >&2
  exit 1
}
echo "Building (this takes a few minutes)…"
jobs="$( (sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) )"
make -C "$work" -j"$jobs" ffmpeg >"$work/build.log" 2>&1 || {
  tail -30 "$work/build.log" >&2
  exit 1
}

exe=""
[[ "$target" == windows ]] && exe=".exe"
cp "$work/ffmpeg$exe" "$out/ffmpeg$exe"
[[ "$target" != windows ]] && strip "$out/ffmpeg$exe" 2>/dev/null || true
cp "$work/COPYING.LGPLv2.1" "$out/COPYING.LGPLv2.1"
{
  echo "FFmpeg $FFMPEG_VERSION, built for $target on $(date -u +%Y-%m-%d)"
  echo "Source: https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
  echo "SHA-256: $FFMPEG_SHA256"
  echo "Licence: LGPL v2.1 or later (no GPL or non-free parts enabled)"
  echo "Configure: ${flags[*]}"
} >"$out/BUILD-INFO.txt"

size="$(du -h "$out/ffmpeg$exe" | cut -f1)"
echo "Done: $out/ffmpeg$exe ($size)"
