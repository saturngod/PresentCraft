# Release packages

Release packages include a small LGPL build of FFmpeg (about 7 MB) so inline video works without installing anything. The app looks for it next to itself first (`Contents/Resources/ffmpeg/` in the macOS app, `ffmpeg/` beside the program on Windows and Linux), then on the system.

macOS (on a Mac):

```sh
packaging/build-ffmpeg.sh macos
packaging/package-macos.sh
```

This makes `dist/PresentCraft.app` and `dist/PresentCraft-<version>-macos-<arch>.zip`. The app is signed ad hoc unless `APPLE_SIGNING_IDENTITY` is set (for example `Developer ID Application: Name (TEAMID)`); then it is signed with that certificate and the hardened runtime. Also set `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID` to notarize and staple it.

Windows: build `ffmpeg.exe` on macOS or Linux with `packaging/build-ffmpeg.sh windows` (needs MinGW-w64: `brew install mingw-w64`) and copy `packaging/vendor/ffmpeg/windows/` to the Windows machine, then run `powershell -ExecutionPolicy Bypass -File packaging\package-windows.ps1` there (add `-Arch arm64` for Windows on ARM; the default is the computer's own architecture). Without the small build, the script downloads BtbN's ready-made LGPL `ffmpeg.exe` instead (larger, about 60 MB zipped); ARM64 always uses it.

Linux (on Linux): `packaging/build-ffmpeg.sh linux` then `packaging/package-linux.sh`, or `FETCH_FFMPEG=1 packaging/package-linux.sh` to use BtbN's ready-made LGPL build.

Linux also gets a `.deb`, an `.rpm` and an AppImage. After `package-linux.sh`, run `packaging/package-linux-formats.sh` (all three; or name them: `deb rpm appimage`). It needs `dpkg-deb`, `rpmbuild` (package `rpm`), and for the AppImage `curl`, `file` and `desktop-file-validate` (package `desktop-file-utils`); formats whose tools are missing are skipped. The `.deb` and `.rpm` install to `/usr/lib/presentcraft` with a `presentcraft` command, a menu entry, icons and the `.prcraft` file type. The AppImage is one file: make it executable and run it (`chmod +x PresentCraft-*.AppImage`). It needs the `fusermount` command (package `fuse3`, already on most desktops), or run it with `--appimage-extract-and-run`. All Linux packages need glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 or later); the `.deb` and `.rpm` declare the exact minimum, read from the program when packaging.

## GitHub Actions

`.github/workflows/release.yml` builds all packages when a tag starting with `v` is pushed and attaches them to a GitHub release for that tag: macOS (Apple silicon, signed and notarized), Linux x86_64 and aarch64 (`.tar.gz`, `.deb`, `.rpm` and AppImage each), and Windows x64 and ARM64. Running it by hand from the Actions tab builds the packages without publishing.

The version comes from `Cargo.toml`. It shows in the app, the package file names, the macOS bundle and the Windows exe. Make releases with `packaging/release.sh`, which sets the version, commits it and adds the matching tag, so the build always has the right number:

```sh
packaging/release.sh patch            # 0.1.0 -> 0.1.1
packaging/release.sh minor            # 0.1.0 -> 0.2.0
packaging/release.sh major            # 0.1.0 -> 1.0.0
packaging/release.sh 0.2.0            # an exact version
packaging/release.sh patch --dry-run  # show the new version and tag only
packaging/release.sh patch --push     # also push the branch and tag, which starts the build
```

The working tree must be clean first. The script updates `Cargo.toml` and `Cargo.lock`, commits "Release v0.1.1" and creates the tag `v0.1.1`. Without `--push`, push yourself with `git push origin <branch> && git push origin v0.1.1`. A tag made by hand that does not match `Cargo.toml` fails the workflow's first check before anything is built.

The macOS job needs these repository secrets:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | The Developer ID Application certificate and key exported as `.p12`, base64-encoded (`base64 -i cert.p12 \| pbcopy`). |
| `APPLE_CERTIFICATE_PASSWORD` | The `.p12` export password. |
| `APPLE_SIGNING_IDENTITY` | The certificate name, for example `Developer ID Application: Name (TEAMID)`. |
| `APPLE_ID` | The Apple ID email used for notarization. |
| `APPLE_PASSWORD` | An app-specific password for that Apple ID. |
| `APPLE_TEAM_ID` | The 10-character team ID. |

Every package carries `licenses/FFMPEG-NOTICE.txt`, the LGPL text and `BUILD-INFO.txt` (FFmpeg version, source link, configure options). FFmpeg's GPL-only parts are never enabled. Build output goes to `dist/` and `packaging/build|vendor/`, which git ignores.

