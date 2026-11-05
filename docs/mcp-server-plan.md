# MCP server plan: let AI use PresentCraft

**Status:** all phases are implemented. See [MCP server](mcp-server.md) for how to use it. Decisions taken while building: localhost TCP with a random token and a discovery file (not a Unix socket); the stdio shim is `PresentCraft --mcp` in the same binary; JSON-RPC is hand-written (no new dependency); slide numbers are 1-based; undo is one step per AI change and refuses to undo over hand edits. Known limits: slide screenshots capture the whole window; with the app closed only folder-form packages can be edited, and only script tools work there.

Goal: an AI client (Claude Desktop, Claude Code, any MCP client) can do everything a person can do in PresentCraft: write slide text, create and reorder slides, change themes, colours, fonts and sizes, add pictures, videos and logos, set header and footer slots, export, and present. The AI should see the result, so it can check its own work.

## 1. Architecture

There are two ways to connect, and the plan uses both.

| Mode | How it works | When |
| --- | --- | --- |
| **Live** (primary) | The running app opens a local control channel. The MCP server is a thin process that forwards each tool call to the app. Edits show up in the open window at once. | The user has PresentCraft open. |
| **Headless** (fallback) | The MCP server loads a `.prcraft` package itself and edits it on disk with the same command code, without a window. Exports and checks still work. | The app is closed, or the AI works on a file in a batch. |

Key decisions:

1. **One command layer for both the GUI and the AI.** Add `src/control/` with a `Command` enum (for example `SetSlideText`, `ApplyTheme`, `SetPaletteColor`, `SetTitleFont`, `AddPicture`, `SetHeaderSlot`, `Export`) and a `Response` type. The GUI buttons and the MCP tools both call the same functions. This keeps the AI and the app from drifting apart, and it means an AI edit is an ordinary user edit: it marks the deck dirty and works with Undo, if Undo exists (confirm this first).
2. **The UI thread owns all state.** Commands from the socket go into a queue. `App::update` drains the queue each frame, runs each command, and calls `ctx.request_repaint()`. Nothing outside the UI thread touches `App`.
3. **Local channel only.** The app listens on `127.0.0.1` (or a Unix socket in the app support folder) with a random token it writes to a file readable only by the user. Any local process that wants to drive the app needs that token. The port is off until the user turns it on in Settings.
4. **MCP shim.** A second entry point, `PresentCraft --mcp` (or a separate `presentcraft-mcp` binary, to decide), speaks MCP over stdio: `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`. The shim connects to the live app when it is running. If not, it uses headless mode.
5. **Protocol code.** The MCP surface is small, so hand-write JSON-RPC over stdio with the existing `src/json.rs`. This avoids adding an async runtime. The alternative is the `rmcp` crate. Decide before phase 0.

Pieces that already exist and can be reused:

- `parse.rs` / `parse_document`: slides from Markdown.
- `package.rs`: open, save, import media, `add_asset`, `list_assets`.
- `themefile.rs`: load, save and import theme folders, presets and colours.
- `docinfo.rs`: `DocSettings`, logo setting.
- `export.rs`: PDF, PowerPoint, images and HTML export jobs.
- `render.rs` / screenshots (`--screenshot` path): render a slide to PNG for the AI.
- `Skills/create-prcraft/scripts/prcraft.py`: Python build and check, which covers the headless file path for simple cases.

## 2. Tool surface

Each tool maps to something a person can do in the app. The list below is the inventory to check against the inspector (`src/inspector.rs`), the settings window (`src/settings.rs`), the menus and the shortcut list in `src/main.rs`.

**Document and slides**
- `deck_get`: slide count, titles, current slide, theme name, dirty flag, path.
- `deck_read_markdown` / `deck_write_markdown`: whole script (replace, or replace a range).
- `slide_list`, `slide_get`, `slide_add` (position, layout, text), `slide_update_text`, `slide_delete` (needs `confirm: true`), `slide_move`, `slide_duplicate`, `slide_set_notes`, `slide_select` (current slide, which also drives the screenshot).
- `text_format` (bold, italic, strike, code, heading, list, quote, tab-prefixed on-slide text) mirrors the Format tab shortcuts.
- `insert_block` for table, chart (line, bar, pie), code, math, footnote, and a picture or video.

**Media and logos**
- `media_list` (library, search), `media_add` (local path, or a web URL, which goes through the existing background download), `media_remove` (needs confirm, refuses while in use unless forced), `media_insert_on_slide`.
- `logo_set` (path or media item), `logo_size`, `logo_remove`.
- `header_footer_set` for each of the six slots (`App.slots: [Slot; 6]`). The kinds are the real `SlotKind` values: `empty`, `number` (Slide N°), `date`, `logo`, and `custom` (with a text value). Each slot can be set independently, so a logo can sit in several places.
- `picture_options_set` for an image on a slide, matching `ImageSpec` in `parse.rs`: `contain` (fit instead of fill), `x` and `y` position (0 to 1), `background` (full-slide picture), `opacity`, `filter` (grayscale, sepia, blur, lighten, darken), and `caption`. The image toolbar in `imagetools.rs` edits these same lines of Markdown, so the AI and the toolbar stay in step.

**Theme, colours, fonts**
- `theme_list` (built-in and user themes), `theme_apply` (applies the full bundle and clears overrides, as the app does).
- `appearance_set`: light or dark, match system.
- `palette_get` / `palette_set`: each colour slot in `Palette`, for light and dark separately.
- `background_set`: solid or gradient, gradient stops, add or remove stops, reverse.
- `font_set`: title or body face (theme, sans, serif, or a named installed family), text size (S, M, L, XL), line height.
- `theme_duplicate`, `theme_import`, `theme_export` (folder or zip), `theme_save_custom`.

**Settings and info**
- `document_info_set`: author, audience, date, aspect ratio (16:9, 16:10, 4:3).
- `settings_get` / `settings_set` for the preferences in `settings.rs` that are safe to change (editor font size, smart punctuation, single-return, and so on). Anything touching system or security settings is out of scope.

**Present and export**
- `present_start` (from slide N), `present_stop`, `present_goto`.
- `export` with format (`pdf`, `pptx`, `png`, `html`) and destination path. Overwriting an existing file needs `overwrite: true`.
- `package_save` (folder or zip), `package_open` (path).

**Checks and feedback (important for quality)**
- `slide_screenshot` returns a PNG of the given slide (or a grid of all slides). The AI uses this to check layout, contrast and overflow.
- `deck_check` returns warnings: missing media, low contrast (the same rules as `prcraft.py check`), empty slides, text that does not fit.

**Resources (read-only context)**
- `prcraft://deck/markdown`, `prcraft://deck/slides`, `prcraft://theme/current`, `prcraft://themes`, `prcraft://media`.

## 3. Safety

- **Confirmation for destructive actions.** Deleting slides, removing media, overwriting files, and replacing the whole script need `confirm: true`. The default is refusal.
- **Paths.** Reading media and writing exports go through an explicit path check. Relative paths resolve against the open package. Writing outside the user's home or the package folder needs a flag the user sets in Settings. Web URLs are allowed only for `http` and `https`, with a size limit on downloads.
- **Token and off switch.** The control channel is off by default and uses a random token. The user starts and stops it from the menu: **File > MCP > Start Server** and **File > MCP > Stop Server**. The menu shows the current state (running or stopped), and the status line confirms each change. Stopping the server closes the listener and invalidates the token. No emoji in the UI (AGENTS.md). Use `paint_icon` if an icon is needed. Follow DESIGN.md for the new rows.
- **Visible changes.** The app shows a status line when the AI changes something ("AI: set title font to Cormorant Garamond"), so the user can see what happened.
- **No secrets.** The MCP responses never include the token, file paths outside the deck, or environment variables.

## 4. Phases

**Phase 0: foundation (small, testable)**
- `src/control/` with `Command`, `Response`, and a queue drained in `App::update`.
- Local listener with token; off by default.
- **File > MCP menu** (inserted after Export in `src/main.rs`, File menu at about line 1363): `MCP` submenu with `Start Server` (enabled when stopped) and `Stop Server` (enabled when running), plus a disabled status line that reads "Server running on port N" or "Server stopped". Start and stop send commands to the listener; they do not touch the deck. Build it with the listener so the menu never shows a state the server does not have.
- MCP stdio shim with `initialize`, `tools/list`, `tools/call`.
- Read-only tools: `deck_get`, `slide_list`, `slide_get`, `theme_list`, `palette_get`, `slide_screenshot`.
- Unit tests for the command layer, with no window (the command functions are pure over `App` state where possible).

**Phase 1: writing and layout**
- Document and slide edits, text formatting, `insert_block`, notes, present controls.
- Verify with the screenshot tool: `cargo run --locked --bin PresentCraft -- --tab format --screenshot shots/check-format.png` and the Design variant, per AGENTS.md.

**Phase 2: theme, colours, fonts**
- `theme_apply`, `appearance_set`, `palette_set`, `background_set`, `font_set`, document info, aspect ratio.
- Check every new control against the inspector, so AI and user results match.

**Phase 3: media, logos, header and footer**
- Media add, insert, remove, logo set, six header and footer slots.
- Test: AI adds a company logo, sets the footer slot to Logo, and the screenshot shows it on every slide.

**Phase 4: export, packages, themes, headless**
- Export in all formats, package save and open, theme import, export and save.
- Headless mode for the MCP shim when the app is not running.

**Phase 5: docs and skill**
- README "For AI agents" section: how to enable the server, the MCP config block, and the tool list.
- Update `Skills/create-prcraft` so the skill mentions the MCP tools when they are available.
- Update `docs/development.md` with launch options (`--mcp`, if added).

## 5. Parity checklist

Keep this list in the code review for each phase. Each user-facing feature gets a command, or a documented reason it does not.

- [ ] Format tab: all `Action` variants (prefix, wrap, block, picture, video)
- [ ] Media tab: show, insert, copy in, remove, search
- [ ] Picture options: contain, position, background, opacity, filter, caption, size (`ImageSpec`, `imagetools.rs`)
- [ ] Header and footer: all six slots, each with empty, slide number, date, logo, or custom text
- [ ] Design tab: theme, appearance, palette, background gradient, fonts, size, line height, header and footer, logo
- [ ] Share tab: export formats, present
- [ ] Settings: each page (General, Editor, Presentation, Themes, Markdown), safe items only
- [ ] Menus and shortcuts: new document, open, save, find, next and previous slide
- [ ] Theme files: import, duplicate, save, remove user theme

## 6. Open questions for you

1. **Transport:** stdio shim talking to a local socket (recommended), or a local HTTP server with a token? The stdio shim works with the most MCP clients.
2. **Binary:** `PresentCraft --mcp` in the same binary (simpler packaging), or a separate `presentcraft-mcp`?
3. **MCP library:** hand-written JSON-RPC (no new dependency, recommended), or the `rmcp` crate?
4. **Scope of phase 0:** should I start building the command layer and read-only tools now?
5. **Undo:** the app has no Undo of its own (a search finds no undo stack; the text editor's built-in undo is the only one). AI edits therefore cannot be undone in the app yet. Phase 1 needs a decision: add an undo stack for document and design changes (recommended, and each AI call becomes one undo step), or keep the AI on a copy and save only when the user approves.
