# Contributor instructions

## Scope and stack

Work in the project root (the directory containing `Cargo.toml`). This is a Rust 2021 native desktop application using egui/eframe 0.37.0, not a website or SwiftUI app. Read `README.md` for build and launch instructions. Keep changes focused on the user's request and preserve existing work.

## Build and verify

Run commands from this directory:

```sh
cargo build --locked
cargo test --locked
```

Use `cargo run --locked --bin PresentCraft` to launch the sample, or pass a Markdown path after `--`. Fix build errors before delivery. Run the existing tests for code changes; documentation-only changes still require a build when requested.

For visual verification, capture fresh Format and Design screenshots with the commands below and inspect the editor, slide rail, typography, toolbar, and inspector. Screenshot commands require a graphical desktop and close automatically. Do not delete the user's saved preferences.

```sh
cargo run --locked --bin PresentCraft -- --tab format --screenshot shots/check-format.png
cargo run --locked --bin PresentCraft -- --tab design --screenshot shots/check-design.png
```

## Implementation map

- `src/main.rs`: application state, command-line options, editor, navigation, presentation, screenshots.
- `src/parse.rs`, `inline.rs`: document and inline Markdown parsing.
- `src/render.rs`, `theme.rs`: slide layout, drawing, and themes.
- `src/inspector.rs`, `settings.rs`: inspector controls and preferences.
- `src/images.rs`, `chart.rs`, `tableblock.rs`, `tabletools.rs`: media and structured content.
- `src/video.rs`, `webvideo.rs`: inline video playback with ffmpeg; YouTube's player in the system web view.
- `src/export.rs`: PDF, PowerPoint, image, and HTML export.
- `src/mechanics.rs`: application defaults and editor behavior constants.

## UI design

Follow `DESIGN.md` for colours, typography, spacing and component rules (buttons, text fields, menus, inspector rows) when adding or changing UI.

Never use emoji in the UI (buttons, labels, menus, tooltips, status text). Draw icons with `paint_icon` in `src/main.rs`, modelled on SF Symbols, and avoid egui stock widgets that show emoji, such as `egui::color_picker`.

## Evidence and maintenance

Preserve the existing appearance during unrelated changes. Avoid hand-editing `target/` build products or treating old screenshots as fresh validation. Keep `Cargo.lock` consistent with intentional dependency changes and update the README when build commands or launch options change.

## Creating presentations

To create, edit or validate a `.prcraft` presentation package, follow `.claude/skills/create-prcraft/SKILL.md`. It documents the package layout, the Markdown slide rules, `info.json` and theme files. It also provides `scripts/prcraft.py` (`build`, `check`, `zip`, `unzip`; Python standard library only).
