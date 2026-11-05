# Development

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
| `--find table` | Open the find bar (⌘F / Ctrl+F) searching for the given text. |
| `--slide 2` | Select a slide, numbered from 1. |
| `--theme 1` | Select a built-in theme, numbered from 1 in the order listed under Themes. Yangon is the default. |
| `--light` | Open with the light editor appearance. |
| `--dark` | Open with the dark editor appearance. |
| `--settings general\|editor\|presentation\|themes\|markdown` | Open a settings page. |
| `--present` | Start presenting (from `--slide`). |
| `--play-video` | Start presenting and play the slide's first video (for checks). |
| `--screenshot shots/check.png` | Save the window as a PNG and close. |
| `--export pdf\|pptx\|images\|html PATH` | Export and close; use a directory for `images`. |
| `--start-mcp` | Start the MCP server when the window opens (same as File › MCP › Start Server). |
| `--mcp` | Run the MCP server over stdio for an AI client. Does not open a window. See [MCP server](mcp-server.md). |

## Project files

`src/main.rs` owns the window and editor. The other modules handle parsing, rendering, images, tables, charts, settings, themes, and export. `src/sample.md` is the bundled sample. `AGENTS.md` contains contributor guidance.

Temporary screenshots and exports belong in `shots/`; release packages and FFmpeg build/download output belong in `dist/` and `packaging/build/` or `packaging/vendor/`. These generated outputs are ignored by version control. Generated screenshots, release packages, FFmpeg downloads and build intermediates are not included in the source tree; the documented commands recreate them.

## Built-in themes

- **Storage:** each theme is a theme folder in `themes/`, built into the app. The folder holds `template.json`, `presets.json`, a stylesheet with `@font-face` rules, previews, and its fonts in `fonts/`.
- **Fonts:** they come from [Google Fonts](https://fonts.google.com) under the SIL Open Font License (licence files are in each `fonts/` folder). They work without being installed.
- **First run:** the app writes the folders to `Built-in Themes/` next to the settings file.
- **Saving:** a presentation copies its theme, fonts included, into its `Theme/` folder, so it looks the same on another computer.

To change one, edit its `themes/<City>/` files, then refresh the stylesheets and previews with `cargo test -- --ignored write_built_in_theme_folders`.

## Application icon

`icon.png` is the source artwork. Committed assets in `packaging/icons/` are used by normal builds; Python is not required to build the app. To regenerate them on macOS after replacing the square 1024×1024 source:

```sh
python3 packaging/generate-icons.py
```

- macOS: `PresentCraft.icns` contains 16, 32, 128, 256, and 512 point images at 1× and 2× resolution (up to 1024 pixels). The artwork occupies 824×824 pixels centered in a transparent 1024×1024 canvas so its Dock size matches other macOS apps. The packaging script registers it in the app bundle; source launches use the same padded artwork.
- Windows: `PresentCraft.ico` includes 16, 24, 32, 48, 64, 128, and 256 pixel images. Cargo embeds it in the executable using `winresource`; native Windows builds require the Windows SDK resource compiler (included with the C++ build tools).
- Linux: the package includes a desktop entry and PNGs from 16 to 1024 pixels under `share/icons/hicolor/`. Put the executable on `PATH`, copy the package's `share/` contents into `~/.local/share/` (or `/usr/share/` for a system installation), and run `update-desktop-database ~/.local/share/applications` if available. The running window also uses the embedded PNG icon.
