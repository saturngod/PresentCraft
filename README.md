# PresentCraft

A native Markdown presentation editor built with Rust and egui/eframe. Edit a script, browse slide thumbnails, change the design, and present or export the deck.

## Requirements

- macOS, Windows or Linux with a graphical desktop session (developed and screenshot-checked on macOS; Windows is type-checked).
- A stable Rust toolchain with Cargo, installed through rustup.
- The platform's native build tools: Xcode Command Line Tools on macOS (`xcode-select --install`), Visual Studio Build Tools on Windows, or a C toolchain plus the usual X11/Wayland development packages on Linux.
- Internet access for the first build to download dependencies. Later builds can use Cargo's cache.

## Build and run

Open Terminal in the project folder and run:

```sh
cargo build --locked
cargo run --locked --bin PresentCraft
```

With no document argument, the app opens its bundled sample presentation. To open your own Markdown document:

```sh
cargo run --locked --bin PresentCraft -- "/path/to/presentation.md"
```

The debug executable is `target/debug/PresentCraft`. You can also double-click it in Finder. For an optimized build:

```sh
cargo build --locked --release
./target/release/PresentCraft
```

The executable is a native program, not a packaged `.app` bundle or a web server. No Node.js setup is needed. Local image paths are resolved relative to the document. Web pictures (`https://…`) download in the background with the system's `curl` (built into macOS, Windows 10+ and most Linux systems) and are cached in the temp folder under `presentcraft-web`; exports wait for them. Pictures that can't be loaded display a placeholder.

When the document is saved, pictures and videos you add or drop that sit in the same folder (or below it) are written as `./name.ext`; others use the full path, and spaces are written as `%20`. Videos show a frame from the video with a play button. While presenting, click the video or press **V** to play or pause it on the slide; a thin bar shows its progress and it stops when you change slides. Background videos (`background: true`) start by themselves, silent and looping. Video uses `ffmpeg`: release packages bundle their own small copy; when running from source, install it (`brew install ffmpeg`, `winget install ffmpeg`, or your Linux package manager). Sound plays with `ffplay` if installed, otherwise through the system (`afplay` on macOS, Windows' built-in player, `pw-play`/`paplay`/`aplay` on Linux). Without ffmpeg, the preview frame comes from Quick Look on macOS and videos open in the system's video player instead.

### Videos in exports

- **HTML:** videos play on the page, at the same place and crop as in the app. They are copied into a `<name>-media` folder next to the `.html` file, so keep the two together. Background videos start by themselves (silent, looping); other videos have play controls, or press **V**. On slides with a background video, the text is real HTML text drawn above the video.
- **PowerPoint:** videos are embedded in the `.pptx` (each file once) with their still frame as the cover, and play when clicked in the slideshow. Background videos also play on click there, and picture filters only change the cover, not the playing video.
- **PDF and images** show the video's still frame.

Themes bundle colours, title and body fonts, and text size. Selecting a theme applies the full bundle, replacing manual design overrides. You can adjust fonts and size in Design afterward; Duplicate Theme in Settings captures those choices in a custom theme.

In **Design → Header & Footer**, select any of the six slots and choose **Logo**. Select an image or drop it onto the logo preview, then adjust **Size**. Each slot can use a different image; logos keep their aspect ratio and appear in presentations and all export formats. **Remove Logo** clears the selected slot’s image. Like the other Design controls, these choices apply to the current presentation session.

## Release packages

Release packages include a small LGPL build of FFmpeg (about 7 MB) so inline video works without installing anything. The app looks for it next to itself first (`Contents/Resources/ffmpeg/` in the macOS app, `ffmpeg/` beside the program on Windows and Linux), then on the system.

macOS (on a Mac):

```sh
packaging/build-ffmpeg.sh macos
packaging/package-macos.sh
```

This makes `dist/PresentCraft.app` and `dist/PresentCraft-<version>-macos-<arch>.zip`. The app is signed ad hoc; to give it to other people without Gatekeeper warnings, sign it with a Developer ID certificate and notarize it.

Windows: build `ffmpeg.exe` on macOS or Linux with `packaging/build-ffmpeg.sh windows` (needs MinGW-w64: `brew install mingw-w64`) and copy `packaging/vendor/ffmpeg/windows/` to the Windows machine, then run `powershell -ExecutionPolicy Bypass -File packaging\package-windows.ps1` there. Without the small build, the script downloads BtbN's ready-made LGPL `ffmpeg.exe` instead (larger, about 60 MB zipped).

Linux (on Linux): `packaging/build-ffmpeg.sh linux` then `packaging/package-linux.sh`, or `FETCH_FFMPEG=1 packaging/package-linux.sh` to use BtbN's ready-made LGPL build.

Every package carries `licenses/FFMPEG-NOTICE.txt`, the LGPL text and `BUILD-INFO.txt` (FFmpeg version, source link, configure options). FFmpeg's GPL-only parts are never enabled. Build output goes to `dist/` and `packaging/build|vendor/`, which git ignores.

## Checks and screenshots

Run these commands from the project root (the folder containing `Cargo.toml`):

```sh
cargo test --locked
cargo run --locked --bin PresentCraft -- --tab format --screenshot shots/check-format.png
cargo run --locked --bin PresentCraft -- --tab design --screenshot shots/check-design.png
```

Screenshot runs need a graphical desktop. Each saves a PNG after the window has drawn and then closes. Review the editor, slide rail, toolbar, and inspector for rendering issues. Saved preferences can affect appearance; they live at `~/Library/Application Support/PresentCraft/settings.txt` on macOS, `%APPDATA%\PresentCraft\settings.txt` on Windows, and `~/.config/presentcraft/settings.txt` on Linux. On first launch, PresentCraft reads preferences from the previous Slide Writer location if no PresentCraft settings exist; future saves use the new location. Use `--light` or `--dark` to check either appearance without changing them.

## Useful launch options

Pass options after Cargo's `--` separator:

| Option | Purpose |
| --- | --- |
| `--new` | Start with a blank document from the new-presentation template (File › New and ⌘N / Ctrl+N open a new window this way). |
| `--tab format\|pictures\|design\|share` | Select the inspector tab. |
| `--slide 2` | Select a slide, numbered from 1. |
| `--theme 1` | Select a theme, numbered from 1 (1–5 classic, 6–10 Studio, Midnight, Editorial, Corporate, Sunset). Studio is the default. |
| `--light` | Open with the light editor appearance. |
| `--dark` | Open with the dark editor appearance. |
| `--settings general\|editor\|presentation\|themes\|markdown` | Open a settings page. |
| `--present` | Start presenting (from `--slide`). |
| `--play-video` | Start presenting and play the slide's first video (for checks). |
| `--screenshot shots/check.png` | Save the window as a PNG and close. |
| `--export pdf\|pptx\|images\|html PATH` | Export and close; use a directory for `images`. |

## Project files

`src/main.rs` owns the window and editor. The other modules handle parsing, rendering, images, tables, charts, settings, themes, and export. `src/sample.md` is the bundled sample. `AGENTS.md` contains contributor guidance.

Temporary screenshots and exports belong in `shots/`; release packages and FFmpeg build/download output belong in `dist/` and `packaging/build/` or `packaging/vendor/`. These generated outputs are ignored by version control. Generated screenshots, release packages, FFmpeg downloads and build intermediates are not included in the source tree; the documented commands recreate them.

## Application icon

`icon.png` is the source artwork. Committed assets in `packaging/icons/` are used by normal builds; Python is not required to build the app. To regenerate them on macOS after replacing the square 1024×1024 source:

```sh
python3 packaging/generate-icons.py
```

- macOS: `PresentCraft.icns` contains 16, 32, 128, 256, and 512 point images at 1× and 2× resolution (up to 1024 pixels). The artwork occupies 824×824 pixels centered in a transparent 1024×1024 canvas so its Dock size matches other macOS apps. The packaging script registers it in the app bundle; source launches use the same padded artwork.
- Windows: `PresentCraft.ico` includes 16, 24, 32, 48, 64, 128, and 256 pixel images. Cargo embeds it in the executable using `winresource`; native Windows builds require the Windows SDK resource compiler (included with the C++ build tools).
- Linux: the package includes a desktop entry and PNGs from 16 to 1024 pixels under `share/icons/hicolor/`. Put the executable on `PATH`, copy the package's `share/` contents into `~/.local/share/` (or `/usr/share/` for a system installation), and run `update-desktop-database ~/.local/share/applications` if available. The running window also uses the embedded PNG icon.
