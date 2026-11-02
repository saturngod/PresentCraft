#!/usr/bin/env bash
# Builds "PresentCraft.app" with its bundled ffmpeg and zips it.
#
#   packaging/build-ffmpeg.sh macos     # once (or after changing the ffmpeg version)
#   packaging/package-macos.sh
#
# Output: dist/PresentCraft.app and dist/PresentCraft-<version>-macos-<arch>.zip
# The app is signed ad hoc (fine for your own Macs). To share it widely,
# sign it with a Developer ID and notarize it (see README).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
ff="$here/vendor/ffmpeg/macos"
if [[ ! -x "$ff/ffmpeg" ]]; then
  echo "No bundled ffmpeg yet: run packaging/build-ffmpeg.sh macos first." >&2
  exit 1
fi

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
arch="$(uname -m)"

echo "Building PresentCraft $version (release)…"
(cd "$root" && cargo build --locked --release)

app="$root/dist/PresentCraft.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/ffmpeg" "$app/Contents/Resources/licenses"
cp "$root/target/release/PresentCraft" "$app/Contents/MacOS/PresentCraft"
cp "$here/icons/PresentCraft.icns" "$app/Contents/Resources/PresentCraft.icns"
cp "$ff/ffmpeg" "$app/Contents/Resources/ffmpeg/ffmpeg"
cp "$here/FFMPEG-NOTICE.txt" "$ff/COPYING.LGPLv2.1" "$ff/BUILD-INFO.txt" \
  "$app/Contents/Resources/licenses/"

cat >"$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>PresentCraft</string>
  <key>CFBundleDisplayName</key><string>PresentCraft</string>
  <key>CFBundleIdentifier</key><string>app.presentcraft.PresentCraft</string>
  <key>CFBundleExecutable</key><string>PresentCraft</string>
  <key>CFBundleIconFile</key><string>PresentCraft.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict>
</plist>
PLIST

# Ad hoc signature so the bundle (and the ffmpeg inside it) runs on Apple silicon.
codesign --force --sign - "$app/Contents/Resources/ffmpeg/ffmpeg"
codesign --force --deep --sign - "$app"

zip="$root/dist/PresentCraft-$version-macos-$arch.zip"
rm -f "$zip"
(cd "$root/dist" && ditto -c -k --keepParent "PresentCraft.app" "$zip")
echo "Done: $app"
echo "      $zip ($(du -h "$zip" | cut -f1))"
