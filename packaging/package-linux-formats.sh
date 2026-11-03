#!/usr/bin/env bash
# Makes .deb, .rpm and AppImage packages from the folder that
# packaging/package-linux.sh builds.
#
#   packaging/package-linux.sh            # makes dist/presentcraft-<version>-linux-<arch>/
#   packaging/package-linux-formats.sh    # then this
#
# Output (in dist/):
#   presentcraft_<version>_<amd64|arm64>.deb
#   presentcraft-<version>-1.<x86_64|aarch64>.rpm
#   PresentCraft-<version>-<x86_64|aarch64>.AppImage
#
# Needs a Linux machine with dpkg-deb (.deb), rpmbuild (.rpm, package "rpm")
# and, for the AppImage, curl (downloads appimagetool), file and
# desktop-file-validate (package "desktop-file-utils"). Formats whose tools
# are missing are skipped unless you name them:
#   packaging/package-linux-formats.sh deb rpm appimage
#
# deb and rpm install under /usr/lib/presentcraft with a /usr/bin/presentcraft
# link, plus the menu entry, icons and .prcraft file type under /usr/share.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
arch="$(uname -m)"
version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
# "-" starts the revision in deb and is not allowed in rpm versions; "~" sorts before the release.
pkgver="${version//-/\~}"
id=app.myanmars.presentcraft

case "$arch" in
  x86_64) deb_arch=amd64 ;;
  aarch64) deb_arch=arm64 ;;
  *) echo "Unsupported architecture $arch." >&2; exit 1 ;;
esac

stage="$root/dist/presentcraft-$version-linux-$arch"
if [[ ! -x "$stage/PresentCraft" ]]; then
  echo "Missing $stage: run packaging/package-linux.sh first." >&2
  exit 1
fi

wanted=("$@")
[[ ${#wanted[@]} -gt 0 ]] || wanted=(deb rpm appimage)
explicit=$([[ $# -gt 0 ]] && echo 1 || echo 0)

want() { local f; for f in "${wanted[@]}"; do [[ "$f" == "$1" ]] && return 0; done; return 1; }
need() { # need <format> <tool>
  command -v "$2" >/dev/null && return 0
  if [[ "$explicit" == 1 ]]; then echo "$2 not found (needed for $1)." >&2; exit 1; fi
  echo "Skipping $1: $2 not found."; return 1
}

# The files every installed package carries, laid out as they install.
install_tree() { # install_tree <root>
  local r="$1"
  install -d "$r/usr/lib/presentcraft/ffmpeg" "$r/usr/lib/presentcraft/licenses" "$r/usr/bin" \
    "$r/usr/share/applications" "$r/usr/share/mime/packages"
  install -m 755 "$stage/PresentCraft" "$r/usr/lib/presentcraft/PresentCraft"
  install -m 755 "$stage/ffmpeg/ffmpeg" "$r/usr/lib/presentcraft/ffmpeg/ffmpeg"
  install -m 644 "$stage"/licenses/* "$r/usr/lib/presentcraft/licenses/"
  # The program finds ffmpeg next to its real file, so a link is enough.
  ln -sf ../lib/presentcraft/PresentCraft "$r/usr/bin/presentcraft"
  sed 's|^Exec=.*|Exec=presentcraft %f|' "$here/$id.desktop" >"$r/usr/share/applications/$id.desktop"
  chmod 644 "$r/usr/share/applications/$id.desktop"
  install -m 644 "$here/linux/$id.xml" "$r/usr/share/mime/packages/$id.xml"
  local size
  for size in 16 24 32 48 64 128 256 512 1024; do
    install -d "$r/usr/share/icons/hicolor/${size}x${size}/apps"
    install -m 644 "$here/icons/$size.png" "$r/usr/share/icons/hicolor/${size}x${size}/apps/$id.png"
  done
}

# Refresh menu, file type and icon caches after install or removal.
refresh_script='
if command -v update-mime-database >/dev/null; then update-mime-database /usr/share/mime || true; fi
if command -v update-desktop-database >/dev/null; then update-desktop-database -q /usr/share/applications || true; fi
if command -v gtk-update-icon-cache >/dev/null; then gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true; fi
'

work="$root/dist/formats-$arch"
rm -rf "$work"
mkdir -p "$work" "$root/dist"

if want deb && need deb dpkg-deb; then
  d="$work/deb"
  install_tree "$d"
  install -d "$d/DEBIAN"
  size_kb="$(du -sk "$d" | cut -f1)"
  cat >"$d/DEBIAN/control" <<EOF
Package: presentcraft
Version: $pkgver
Section: graphics
Priority: optional
Architecture: $deb_arch
Installed-Size: $size_kb
Depends: libc6 (>= 2.35), libgl1, libxkbcommon0, libwayland-client0
Recommends: xdg-desktop-portal
Maintainer: PresentCraft <noreply@presentcraft.invalid>
Description: Markdown presentation editor
 PresentCraft edits a Markdown script, shows slide thumbnails, changes the
 design, and presents or exports the deck. Includes a small LGPL build of
 FFmpeg for inline video.
EOF
  printf '#!/bin/sh\nset -e\n%s\n' "$refresh_script" >"$d/DEBIAN/postinst"
  printf '#!/bin/sh\nset -e\n%s\n' "$refresh_script" >"$d/DEBIAN/postrm"
  chmod 755 "$d/DEBIAN/postinst" "$d/DEBIAN/postrm"
  out="$root/dist/presentcraft_${pkgver}_${deb_arch}.deb"
  dpkg-deb --root-owner-group -Zxz --build "$d" "$out" >/dev/null
  echo "Done: $out ($(du -h "$out" | cut -f1))"
fi

if want rpm && need rpm rpmbuild; then
  r="$work/rpm"
  install_tree "$r/root"
  mkdir -p "$r/SPECS"
  cat >"$r/SPECS/presentcraft.spec" <<EOF
%global debug_package %{nil}
%global __strip /bin/true
%global __os_install_post %{nil}
AutoReqProv: no
Name: presentcraft
Version: $pkgver
Release: 1
Summary: Markdown presentation editor
License: Proprietary
URL: https://github.com/saturngod/PresentCraft
Requires: mesa-libGL, libxkbcommon, libwayland-client
Recommends: xdg-desktop-portal

%description
PresentCraft edits a Markdown script, shows slide thumbnails, changes the
design, and presents or exports the deck. Includes a small LGPL build of
FFmpeg for inline video.

%install
cp -a $r/root/. %{buildroot}/

%files
/usr/lib/presentcraft
/usr/bin/presentcraft
/usr/share/applications/$id.desktop
/usr/share/mime/packages/$id.xml
/usr/share/icons/hicolor/*/apps/$id.png

%post
$refresh_script

%postun
$refresh_script
EOF
  rpmbuild -bb --quiet --define "_topdir $r/top" --define "_rpmdir $r/out" \
    --buildroot "$r/buildroot" "$r/SPECS/presentcraft.spec"
  out="$root/dist/presentcraft-$pkgver-1.$arch.rpm"
  cp "$r"/out/*/*.rpm "$out"
  echo "Done: $out ($(du -h "$out" | cut -f1))"
fi

if want appimage && need appimage curl && need appimage file && need appimage desktop-file-validate; then
  tool="$here/build/appimagetool-$arch.AppImage"
  if [[ ! -x "$tool" ]]; then
    mkdir -p "$here/build"
    echo "Downloading appimagetool…"
    curl -fL --retry 3 -o "$tool" \
      "https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-$arch.AppImage"
    chmod +x "$tool"
  fi
  a="$work/PresentCraft.AppDir"
  install -d "$a/usr/bin/ffmpeg" "$a/usr/bin/licenses"
  install -m 755 "$stage/PresentCraft" "$a/usr/bin/PresentCraft"
  install -m 755 "$stage/ffmpeg/ffmpeg" "$a/usr/bin/ffmpeg/ffmpeg"
  install -m 644 "$stage"/licenses/* "$a/usr/bin/licenses/"
  sed 's|^Exec=.*|Exec=PresentCraft %f|' "$here/$id.desktop" >"$a/$id.desktop"
  install -m 644 "$here/icons/256.png" "$a/$id.png"
  ln -s "$id.png" "$a/.DirIcon"
  cat >"$a/AppRun" <<'EOF'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/PresentCraft" "$@"
EOF
  chmod 755 "$a/AppRun"
  out="$root/dist/PresentCraft-$version-$arch.AppImage"
  # --appimage-extract-and-run works where FUSE is missing (CI, containers).
  ARCH="$arch" "$tool" --appimage-extract-and-run --no-appstream "$a" "$out" >/dev/null
  echo "Done: $out ($(du -h "$out" | cut -f1))"
fi

rm -rf "$work"
