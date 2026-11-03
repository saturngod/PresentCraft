#!/usr/bin/env bash
# Builds "PresentCraft.app" with its bundled ffmpeg and zips it.
#
#   packaging/build-ffmpeg.sh macos     # once (or after changing the ffmpeg version)
#   packaging/package-macos.sh
#
# Output: dist/PresentCraft.app and dist/PresentCraft-<version>-macos-<arch>.zip
# The app is signed ad hoc (fine for your own Macs). Set APPLE_SIGNING_IDENTITY
# to sign with a Developer ID, plus APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID
# to notarize (see README).
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
  <key>CFBundleIdentifier</key><string>app.myanmars.presentcraft</string>
  <key>CFBundleExecutable</key><string>PresentCraft</string>
  <key>CFBundleIconFile</key><string>PresentCraft.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>PresentCraft Presentation</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Owner</string>
      <key>LSTypeIsPackage</key><true/>
      <key>CFBundleTypeIconFile</key><string>PresentCraft.icns</string>
      <key>LSItemContentTypes</key><array><string>app.myanmars.presentcraft.prcraft</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>PresentCraft Presentation</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Owner</string>
      <key>CFBundleTypeIconFile</key><string>PresentCraft.icns</string>
      <key>LSItemContentTypes</key><array><string>app.myanmars.presentcraft.prcraft.zip</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>iA Presenter Document</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array><string>net.ia.presenter.package</string><string>net.ia.presenter.package.zip</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Markdown</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>net.daringfireball.markdown</string></array>
    </dict>
  </array>
  <key>UTExportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key><string>app.myanmars.presentcraft.prcraft</string>
      <key>UTTypeDescription</key><string>PresentCraft Presentation</string>
      <key>UTTypeConformsTo</key><array><string>com.apple.package</string><string>public.composite-content</string></array>
      <key>UTTypeIconFile</key><string>PresentCraft.icns</string>
      <key>UTTypeTagSpecification</key>
      <dict><key>public.filename-extension</key><array><string>prcraft</string></array></dict>
    </dict>
    <!-- The same extension as a single zip file (saved on Windows and Linux). -->
    <dict>
      <key>UTTypeIdentifier</key><string>app.myanmars.presentcraft.prcraft.zip</string>
      <key>UTTypeDescription</key><string>PresentCraft Presentation</string>
      <key>UTTypeConformsTo</key><array><string>public.data</string><string>public.composite-content</string></array>
      <key>UTTypeIconFile</key><string>PresentCraft.icns</string>
      <key>UTTypeTagSpecification</key>
      <dict><key>public.filename-extension</key><array><string>prcraft</string></array></dict>
    </dict>
  </array>
  <key>UTImportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key><string>net.ia.presenter.package</string>
      <key>UTTypeDescription</key><string>iA Presenter Document</string>
      <key>UTTypeConformsTo</key><array><string>com.apple.package</string></array>
      <key>UTTypeTagSpecification</key>
      <dict><key>public.filename-extension</key><array><string>iapresenter</string></array></dict>
    </dict>
    <dict>
      <key>UTTypeIdentifier</key><string>net.ia.presenter.package.zip</string>
      <key>UTTypeDescription</key><string>iA Presenter Document</string>
      <key>UTTypeConformsTo</key><array><string>public.data</string></array>
      <key>UTTypeTagSpecification</key>
      <dict><key>public.filename-extension</key><array><string>iapresenter</string></array></dict>
    </dict>
    <dict>
      <key>UTTypeIdentifier</key><string>net.daringfireball.markdown</string>
      <key>UTTypeDescription</key><string>Markdown</string>
      <key>UTTypeConformsTo</key><array><string>public.plain-text</string></array>
      <key>UTTypeTagSpecification</key>
      <dict><key>public.filename-extension</key><array><string>md</string><string>markdown</string></array></dict>
    </dict>
  </array>
</dict>
</plist>
PLIST

zip="$root/dist/PresentCraft-$version-macos-$arch.zip"
rm -f "$zip"

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  # Developer ID signature with the hardened runtime (needed for notarization).
  # The certificate must already be in a keychain.
  echo "Signing with $APPLE_SIGNING_IDENTITY…"
  codesign --force --timestamp --options runtime --sign "$APPLE_SIGNING_IDENTITY" \
    "$app/Contents/Resources/ffmpeg/ffmpeg"
  codesign --force --timestamp --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$app"
  codesign --verify --strict --verbose=2 "$app"

  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    echo "Notarizing…"
    (cd "$root/dist" && ditto -c -k --keepParent "PresentCraft.app" "$zip")
    xcrun notarytool submit "$zip" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" --wait
    xcrun stapler staple "$app"
    xcrun stapler validate "$app"
    rm -f "$zip"
  fi
else
  # Ad hoc signature so the bundle (and the ffmpeg inside it) runs on Apple silicon.
  codesign --force --sign - "$app/Contents/Resources/ffmpeg/ffmpeg"
  codesign --force --deep --sign - "$app"
fi

(cd "$root/dist" && ditto -c -k --keepParent "PresentCraft.app" "$zip")
echo "Done: $app"
echo "      $zip ($(du -h "$zip" | cut -f1))"
