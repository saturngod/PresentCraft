# PresentCraft

<p align="center">
  <img src="icon.png" alt="PresentCraft icon" width="128">
</p>

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

With no document argument, the app opens its bundled sample presentation. To open your own presentation (a `.prcraft` package or a Markdown file):

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

In **Design → Colors**, click a **Background** swatch and choose **Gradient**. Pick a preset or edit the color stops in the picker; choose **Solid** to return to the theme’s background colors. Light and dark appearances have independent background modes and gradient colors. The first slide uses the first color, the last uses the last color, and slides between them blend evenly through the color stops. Add or remove colors, or reverse their order. A one-slide deck uses the first color. Text automatically switches between black and white for contrast; theme decorations and cover inversion are replaced by the chosen background, while background pictures and videos still appear on top. The gradient applies to thumbnails, presenting, and all exports. Like other Design overrides, it lasts for the current session; selecting a theme turns it off, and Duplicate Theme saves the base theme without the deck gradient.

In **Design → Logo**, select an image or drop it onto the logo preview, then adjust **Size**. The presentation has one logo: in **Design → Header & Footer**, set any of the six places to **Logo** to show it there — one place or several. Logos keep their aspect ratio and appear in presentations and all export formats. **Remove Logo** clears the image everywhere. Like the other Design controls, these choices apply to the current presentation session.

## Document format

PresentCraft saves presentations as `Name.prcraft` package folders:

```text
Name.prcraft/
├── text.md          the script (Markdown)
├── info.json        document settings (under "app.myanmars.presentcraft")
├── assets/          pictures and videos, referenced as /assets/photo.jpg
└── Theme/
    ├── template.json  theme name, fonts, decoration, text size
    └── presets.json   light and dark colours, accents and background gradients
```

A package is stored in one of two forms with the same contents:

| | macOS | Windows and Linux |
| --- | --- | --- |
| New documents | Folder package | Single zip file |
| Opens | Folder and zip | Folder and zip |
| Saving an existing document | Keeps its form | Keeps its form |

On macOS, Finder shows a folder package as one file. Saving only rewrites `text.md`, `info.json` and `Theme/`, so large videos in `assets/` are never rewritten. On Windows and Linux there is no package concept, so a zip file is what lets you double-click a presentation. A zipped document is edited in a private working folder in the temporary directory and zipped again on each save. The new archive replaces the old one only once it is complete. Pictures and videos are stored uncompressed, because they are already compressed and this keeps saving quick.

Every picture and video lives inside the presentation. Adding one (dropping it on the editor or the Media panel, Add Images or Videos…, or Select Logo) copies it into `assets/` straight away and writes `/assets/<name>` in the script, never a path like `/Users/…/Downloads/photo.png`. A presentation that hasn't been saved yet keeps its media in a private working folder, and Save copies it into the new package. Opening a Markdown file copies the pictures and videos it uses in the same way. If a script still points outside the package, Save copies those files in too.

The **Media** panel lists everything in the package under **Images** and **Videos**, with a thumbnail and the slides that use each file (or *Not used*). Click a file to go to its slide, and double-click it to put it at the cursor. Right-click for Show in Finder, Copy into Presentation (for files outside the package), and Remove from Presentation (for unused files only). Files in the package that PresentCraft doesn't write are left alone, and unknown `info.json` keys are kept.

`info.json` has the top-level keys `creatorIdentifier`, `type`, `version` and `transient`. PresentCraft's settings (theme, appearance, author, audience, logo, header/footer and gradients) sit under `"app.myanmars.presentcraft"`.

Open also accepts plain Markdown (`.md`, `.markdown`, `.txt`). These files are imported: Save asks where to put a new `.prcraft` package next to the original and never changes the original file. Export → Markdown still writes plain text.

Themes are folders laid out like this:

```text
New York/
├── template.json    name, version, author, descriptions, Css, TitleFont, BodyFont, CssClasses, LayoutExamples
├── presets.json     colour variants (Dark…/Light… text and background colours, Accent1–6, gradients)
├── newyork.css      stylesheet named by template.json's "Css"
├── template.png     500 × 281 preview (also template.webp)
├── fonts/           the theme's font files
└── image1.webp …    example pictures, used as /Theme/image1.webp
```

- **Saving a theme** (Duplicate Theme, or the `Theme/` folder in a package) writes this whole structure. A theme that came from a folder (for example an imported theme) keeps everything in it, including its CSS, fonts, pictures, preview, layout examples and keys like `DarkAccent1`. PresentCraft only updates the values you changed, and colours written as `#000` or `transparent` stay as they were.
- **New themes** get a stylesheet named after the theme in lower case, with spaces kept ("My Theme" → `my theme.css`), and a preview drawn in its colours. PresentCraft also adds `Decoration`, `TextSize` and `InvertTitles` to `template.json`.
- **Layouts and fonts:**
  - Inverted cover and title slides are written as `"Layouts": [{"Name": "Cover", "Classes": "invert"}, {"Name": "Title", "Classes": "invert"}]`.
  - `template.json` holds font display names (`System`, `Inter`). `presets.json` holds CSS names (`system-ui`, `ui-serif`, or a list like `-apple-system-ui-serif, ui-serif`).
- **Colours and charts:**
  - Colours may include transparency (`#000000b3` is black at 70%).
  - `DarkAccent1` / `LightAccent1` set the first accent for light and dark slides.
  - A Dark preset may have its own `Accent1`–`Accent6`.
  - `LightGraphDataColors` / `DarkGraphDataColors` (`data-0`, `data-1`, …) colour charts.
- **Gradients:** when the theme supplies `LightBgGradient` / `DarkBgGradient`, its own text colours are used on every slide (San Francisco: white throughout). A gradient you pick yourself gets black or white text for contrast.
- **Fonts:** `.ttf` and `.otf` files anywhere in a theme's folder (usually `fonts/`) are used for its slides. Web-only fonts (`.woff`, `.woff2`) can't be drawn natively, so those themes use an installed font with the same name, or the built-in sans or serif.
- **Importing:** Settings → Themes → Import Theme… adds any such folder. Saved themes live in `Themes/` next to the settings file.

AI agents can create packages with the `create-prcraft` skill in `.claude/skills/create-prcraft/`. Its helper script builds a package from a Markdown script and checks it:

```sh
python3 .claude/skills/create-prcraft/scripts/prcraft.py build talk.md Talk.prcraft --theme Bagan
python3 .claude/skills/create-prcraft/scripts/prcraft.py check Talk.prcraft
```

When a package has no `Theme/` folder, `info.json`'s `Theme` key picks an installed theme by name.

### Built-in themes

The built-in themes are named after Southeast Asian cities. Each one's colours and typography are drawn from its city; there are no pictures.

- **Storage:** each theme is a theme folder in `themes/`, built into the app. The folder holds `template.json`, `presets.json`, a stylesheet with `@font-face` rules, previews, and its fonts in `fonts/`.
- **Fonts:** they come from [Google Fonts](https://fonts.google.com) under the SIL Open Font License (licence files are in each `fonts/` folder). They work without being installed.
- **First run:** the app writes the folders to `Built-in Themes/` next to the settings file.
- **Saving:** a presentation copies its theme, fonts included, into its `Theme/` folder, so it looks the same on another computer.

| Theme | Look | Titles / body |
| --- | --- | --- |
| Yangon | Lacquer maroon on ivory; gold on twilight indigo. Covers in the dark look. | Cormorant Garamond / Lato |
| Mandalay | Royal vermilion on warm paper; gilded titles on teak. | Cinzel / EB Garamond |
| Bagan | Dawn gradient, a new colour per slide: rose, apricot, amber; brick to dusk plum when dark. | Marcellus / Nunito Sans |
| Singapore | Singapore red on crisp white; covers in night-bay navy. | Space Grotesk / Inter |
| Bangkok | Jewel-tone gradient (emerald, sapphire, ruby, gold) with white titles. | Prompt / Sarabun |
| Chiang Mai | Forest green on morning mist; lantern orange on a night sky. | Lora / Source Sans 3 |
| Hanoi | Shutter green on mustard; covers in deep green with mustard titles. | Be Vietnam Pro / Be Vietnam Pro |
| Hoi An | Silk-lantern gradient; deep silk tones by default, lantern glow when light. | Playfair Display / Mulish |
| Kuala Lumpur | Navy on cool white; steel on a city night, with songket gold. | Archivo / IBM Plex Sans |
| Penang | Peranakan pastel gradient (pink, peach, mint, lilac); jewel tones when dark. | Fraunces / Karla |
| Siem Reap | Temple brown on sandstone; covers in pre-dawn blue with sunrise titles. | Spectral / Kantumruy Pro |
| Luang Prabang | Saffron on temple white and on dark teak. | Libre Baskerville / Work Sans |
| Bali | Terrace greens to ripe gold; deep jungle greens when dark. | DM Serif Display / DM Sans |
| Manila | Sunset gradient from bay blue to gold, white titles. | Oswald / Poppins |

To change one, edit its `themes/<City>/` files, then refresh the stylesheets and previews with `cargo test -- --ignored write_built_in_theme_folders`.

### Opening documents by double-click

- **macOS:** the release bundle registers `.prcraft` (folder and zip) and offers PresentCraft for Markdown.
- **Windows:** in the unpacked release folder, run `powershell -ExecutionPolicy Bypass -File register-file-types.ps1` once. It registers `.prcraft` for the current user (no administrator rights needed) and adds PresentCraft to *Open with* for Markdown. Run it again if you move the folder; add `-Remove` to undo.
- **Linux:** in the unpacked release folder, run `./install.sh` once. It adds the menu entry, icons and the `application/x-presentcraft` file type under `~/.local/share`. `./install.sh --remove` undoes it.

A folder package from a Mac is an ordinary folder on Windows and Linux. Open it with File → Open Package Folder…, by dragging it onto the window, or by passing its path on the command line.

## Release packages

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

### GitHub Actions

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
| `--theme 1` | Select a built-in theme, numbered from 1 in the order listed under Themes. Yangon is the default. |
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
