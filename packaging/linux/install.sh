#!/usr/bin/env bash
# Installs PresentCraft for the current user: a menu entry, icons, and the
# .prcraft file type, so documents open with a double-click. Run it from the
# unpacked release folder; PresentCraft runs from this folder, so keep it.
#
#   ./install.sh            install
#   ./install.sh --remove   undo
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
id=app.myanmars.presentcraft

refresh() {
  command -v update-mime-database >/dev/null && update-mime-database "$data/mime" || true
  command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$data/icons/hicolor" || true
}

if [[ "${1:-}" == "--remove" ]]; then
  rm -f "$data/applications/$id.desktop" "$data/mime/packages/$id.xml"
  find "$data/icons/hicolor" -name "$id.png" -delete 2>/dev/null || true
  refresh
  echo "PresentCraft removed from the menu and file types."
  exit 0
fi

mkdir -p "$data/applications" "$data/mime/packages"
cp "$here/share/mime/packages/$id.xml" "$data/mime/packages/"
cp -R "$here/share/icons" "$data/"
# Point the menu entry at this folder's PresentCraft.
sed "s|^Exec=.*|Exec=\"$here/PresentCraft\" %f|" "$here/share/applications/$id.desktop" \
  >"$data/applications/$id.desktop"
command -v xdg-mime >/dev/null && xdg-mime default "$id.desktop" application/x-presentcraft || true
refresh
echo "PresentCraft installed. .prcraft files now open with it."
