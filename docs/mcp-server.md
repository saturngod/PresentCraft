# MCP server

PresentCraft can run as an MCP server, so an AI client can do what you can do in the app: write the script, add and move slides, change the theme, colours, fonts and sizes, add pictures and videos, set the logo and the header and footer, check the deck, export and present.

## Turn it on

- **In the app:** File › MCP › **Start Server**. The menu shows the port while it runs; **Stop Server** turns it off. The server listens on `127.0.0.1` only, with a new random token each time it starts.
- **At launch:** `PresentCraft --start-mcp`.

While the server runs, PresentCraft writes `mcp.json` (port, token and process id) to its settings folder: `~/Library/Application Support/PresentCraft` on macOS, `%APPDATA%\PresentCraft` on Windows, and `~/.config/presentcraft` on Linux. Stopping the server removes it. If PresentCraft quits unexpectedly the file can remain; clients then report that PresentCraft is not running, and starting the server again replaces it.

## Connect a client

The client starts PresentCraft's stdio server with `--mcp`. That process never opens a window.

```sh
claude mcp add presentcraft -- /path/to/PresentCraft --mcp
```

For other clients, use the same command and arguments in their MCP configuration.

## How it works

- Each tool call goes to the open window. Changes appear at once, as if you had made them.
- With the app closed, the script tools can still edit a folder-form `.prcraft` package. Pass its path in the `package` argument. Zipped presentations need the app open.
- Slide numbers start at 1, as the app shows them.
- Every change made through the server is one step in the undo list. `undo_ai_change` reverts the last one. It refuses if you have edited the presentation by hand since that change, so your own work is never lost.
- `insert_table` adds a table, or draws it as any of the app's 10 chart types: bar, line, spline, area, step, pie, donut, radar, gauge and bubble. It also sets the chart options: horizontal, stacked, title, axis labels, caption, series colours (hex or theme accents), colour type (categorical, sequential, diverging) and legend.
- `slide_screenshot` returns a PNG of the window with that slide shown. `deck_check` reports empty slides, missing pictures, a missing logo, and text contrast under 4.5:1.

## Tools

| Group | Tools |
| --- | --- |
| Look at the deck | `deck_info`, `deck_read_markdown`, `deck_check`, `slide_list`, `slide_get`, `slide_screenshot` |
| Script and slides | `deck_write_markdown`, `slide_add`, `slide_update`, `slide_delete`, `slide_move`, `slide_duplicate`, `slide_select`, `insert_picture`, `insert_table`, `undo_ai_change` |
| Themes and colours | `theme_list`, `theme_apply`, `theme_import`, `theme_export`, `appearance_set`, `palette_get`, `palette_set`, `background_set` |
| Fonts, size and info | `font_set`, `size_set`, `line_height_set`, `document_info_set` |
| Header, footer and logo | `header_footer_get`, `header_footer_set`, `logo_set`, `logo_size`, `logo_remove` |
| Media | `media_list`, `media_add`, `media_remove` |
| Preferences | `settings_get`, `settings_set` |
| Presenting and files | `present_start`, `present_stop`, `export`, `package_save`, `package_open` |

Run `tools/list` on the server, or read the client's tool list, for the arguments of each tool.

## Limits and safety

- Replacing the whole script (`deck_write_markdown`), deleting a slide and removing media need `confirm: true`.
- Files are read from and written to only inside your home folder, the temporary folder, or the presentation's own folder. Paths with `..` are refused.
- Export refuses to overwrite a file unless `overwrite: true` is given.
- Undo covers the script and the design. It does not undo file exports, media removal or opening another presentation.
- Settings changed by the server are saved, like settings you change yourself. Only the preferences `settings_get` lists can be changed.
