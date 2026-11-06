# PresentCraft Design Guide

How PresentCraft's interface should look, feel and behave. Follow this guide when
you add or change UI. It describes the app as it is today, so a new control should
look like it was always there.

The source of truth for values is the code: `settings.rs` (`Pal`, `set_visuals`),
`main.rs` (accent colours, layout constants, `icon_button`, `paint_icon`) and
`inspector.rs` (inspector controls). If you change a value here, change it there too,
and the other way round.

---

## 1. Principles

1. **Writing comes first.** The Markdown editor is the centre of the app. Chrome
   (rails, toolbars, inspector) stays quiet, low-contrast and out of the way.
2. **Native, calm, Mac-like.** Soft greys, hairline borders, rounded corners, no
   heavy gradients or bevels. It should feel at home next to Keynote or iA Presenter.
3. **Colour means something.** Neutral greys for structure. Colour only for the
   current mode (Format blue, Media green, Design pink, Share orange), the primary action
   (Present blue), selection and focus.
4. **Show on hover, not always.** Secondary controls (section Hide/Show, slide
   speaking time, per-row actions) appear when the pointer is over them.
5. **One look, two themes.** Every control must work in both light and dark.
   Never hard-code a colour that only reads well on one of them.
6. **Plain words.** Labels are short, sentence-like and free of jargon:
   "Drop images or videos", "or click to browse", "Release to add".

---

## 2. Colour

Always read colours from `crate::settings::pal()`; it switches between
`DARK_PAL` and `LIGHT_PAL`. Use `settings::is_dark()` only when a value has no
palette slot.

### 2.1 Window palette (`Pal`)

| Token          | Dark      | Light     | Use |
|----------------|-----------|-----------|-----|
| `editor_bg`    | `#181818` | `#FFFFFF` | Editor and main panel background |
| `sidebar_bg`   | `#222222` | `#F4F4F5` | Slide rail |
| `inspector_bg` | `#1E1E1E` | `#F7F7F8` | Inspector panel |
| `divider`      | `#2C2C2C` | `#E2E2E4` | Lines between sections and panels |
| `border`       | `#343434` | `#D6D6DA` | Hairline around fields, thumbnails, cards |
| `text_hi`      | `#E8E8E8` | `#1C1C1E` | Primary text and labels |
| `text_dim`     | `#6B6B6B` | `#86868B` | Hints, notes, shortcuts, secondary labels |
| `icon`         | `#A8A8A8` | `#6E6E73` | Toolbar and inline icons at rest |
| `button`       | `#383838` | `#E8E8EA` | Button fill at rest; pressed fill for flat rows |
| `hover`        | `#3A3A3A` | `#DDDDE0` | Hover fill for buttons and rows |
| `control`      | `#2F2F2F` | `#EAEAEC` | Segmented-control and slider track |
| `field`        | `#262626` | `#FFFFFF` | Text field background |
| `pill`         | `#2A2A2A` | `#FFFFFF` | Floating pills and banners |
| `tab_grey`     | `#515151` | `#BDBDBD` | Selected segment (dark) |

Popups use `#262626` (dark) / `#FFFFFF` (light). The Settings window uses
`inspector_bg` with `field` cards on top.

For secondary text that must stay readable (more than `text_dim`), use
`text_mid()` in the inspector: `#B4B4B8` dark / `#4A4A4F` light.

### 2.2 Accent colours

| Name            | Value     | Meaning |
|-----------------|-----------|---------|
| `ACCENT_BLUE`   | `#3770FF` | **The one action colour**: primary buttons (Present, Add Table), links, sliders, switches, menu highlight, selection, focus, drag-over |
| `FORMAT_BLUE`   | `#59BDFF` | Format tab pill only |
| `MEDIA_GREEN`   | `#34B27D` | Media tab pill only |
| `DESIGN_PINK`   | `#FC44B2` | Design tab pill only |
| `SHARE_ORANGE`  | `#E59A16` | Share pill when active |
| Text cursor     | `#72B8F3` | 2 px caret |
| Selection fill  | `#2D4A66` / `#C5DCFA` | Selected text (dark / light) |

Destructive or warning states use red `#D9483B` text. File-type tags use their own
fixed colours (PDF `#E5484D`, PPT `#D96C2C`, PNG `#30A46C`, HTML `ACCENT_BLUE`,
MD `#6E6E73`) with white text.

Slide numbers use the `slide_color(i)` ramp (blue → violet → pink and back). In
light mode use the deeper `LIGHT_RAMP` so numbers stay readable.

### 2.3 Rules

- Text on an accent fill is always white.
- Do not introduce a new accent for a single control. Reuse the table above.
- Tab colours only mark the current tab. Every control in every tab uses
  `ACCENT_BLUE`, so a button or slider looks the same wherever it is.
- Contrast: body text must read clearly on its background in both themes. If
  `text_dim` is too faint for something people must read, use `text_mid()`.
- Light mode needs hairline borders on controls so they stand out from pale
  panels; dark mode controls usually have none (see `set_visuals`).

---

## 3. Typography

- UI font: the proportional family set in `fonts.rs`. Bold via
  `crate::theme::family(false, true, false)`.
- Never use more than three sizes in one panel.

| Role                         | Size  | Weight | Colour |
|------------------------------|-------|--------|--------|
| Body, buttons (default)      | 13.5  | Regular | `text_hi` |
| Section title (inspector)    | 13.5  | Bold   | `text_hi` |
| Card or deck title           | 14.5  | Bold   | `text_hi` |
| Settings row label           | 14.0  | Regular | `text_hi` |
| Text field input             | 14.0  | Regular | `text_hi` |
| Form label (inspector)       | 13.0  | Regular | `text_dim`, right-aligned |
| Secondary line / description | 12.0  | Regular | `text_dim` |
| Segment and small button     | 12.5  | Regular | `text_hi` / `icon` |
| Small / caption              | 11.5  | Regular | `text_dim` |
| Picker heading               | 11.0  | Regular | `text_dim` |
| Badges (slide number, pill)  | 10.5–11 | Bold | White |
| Editor text                  | 18.5, line height 31.7 | Regular | theme |

Write labels in Title Case for buttons and menu items ("Show Media Folder",
"Skip This Version") and sentence case for notes and hints. Use "…" (one
character) when an action opens a dialog: "Export…", "Add Images or Videos…".

---

## 4. Spacing, size and shape

Global spacing (`set_visuals`):

- Item spacing: 8 horizontal, 6 vertical.
- Button padding: 10 × 4.
- Standard control height: 24. Menu rows: 26–27. Inspector rows: 26.
  Search fields: 28–30. Top bar: 52.
- Menu margin 6, window margin 12.
- Section gap in the inspector: 12 above a section title, 4 below it.

Corner radius:

| Element                          | Radius |
|----------------------------------|--------|
| Buttons, rows, inputs (default)  | 6 |
| Flat button                      | 7 |
| Text and search fields, cards    | 8 |
| Menus / popups                   | 8 |
| Windows, banners, drop zones     | 10 |
| Settings window                  | 12 |
| Pills (tab bar, Share)           | 14–16 (fully round) |
| Round icon buttons (Present)     | Circle |

Strokes: hairlines are 1 px. Focus rings are 2 px. Selection rings around slide
thumbnails are 2.5 px in the slide colour; hover rings are 2 px in `hover`.

Shadows: popups `0 4 16` and windows `0 10 32`, black at alpha 110 (dark) / 40
(light). Use `ctx.global_style().visuals.popup_shadow` or `window_shadow`; do not
invent new shadows.

Layout constants: slide rail 180 px (150–420, resizable), inspector 307 px,
editor column max 700 px.

---

## 5. Components

Reuse the existing helpers before writing a new control. If you need a new one,
put it next to its siblings and give it the same states.

### 5.1 Every interactive control has these states

| State     | Look |
|-----------|------|
| Rest      | `button` fill (or transparent for flat/icon rows), `text_hi` / `icon` |
| Hover     | `hover` fill, icon becomes `text_hi`, pointing-hand cursor for custom controls |
| Pressed   | `button` fill (for flat rows) or `hover` (for filled buttons) |
| Focus     | 2 px ring in `ACCENT_BLUE` |
| Disabled  | Fill at ~55 % (`gamma_multiply(0.55)`), text `text_dim`, no hover, no click |
| Selected  | Accent fill with white text, or a raised segment (see 5.4) |

Every icon-only control needs a tooltip (`on_hover_text`) that names the action and
its shortcut, written with `kbd()`: `"Present (⌘↵)"`, `"Next match (⌘G)"`.

### 5.2 Buttons

**Standard button** — `egui::Button` / `ui.button`. Uses the global widget style:
`button` fill, radius 6, 13.5 text, 24 high. Use it in dialogs, banners and
settings. Keep labels to one to three words.

**Primary button** — only one per area. Rest like a standard button, hover turns
`ACCENT_BLUE` with white text (see the Export… button in `export_row`). The app's
main primary action, Present, is a 34 px `ACCENT_BLUE` circle with a white play
icon, hover `#4C80FF`.

**Flat button** — `flat_button(ui, rect, label, enabled)` in `inspector.rs`.
Radius 7, 13.0 text, `button` fill, `hover` on hover. Use for actions inside the
inspector.

**Icon button** — `icon_button(ui, rect, icon, tip)` in `main.rs`. Transparent at
rest, `hover` fill on hover, `button` fill while pressed, radius 6. Icons are drawn
with `paint_icon` in `icon` colour, `text_hi` on hover. Typical size 28 × 28; 34 ×
34 for the toolbar ••• menu (`tabletools::more_button`).

**Pill buttons** — in the top bar (the Format / Media / Design tabs and Share),
fully rounded, 32 high. Selected tab fills with its mode colour and shows its name
in white; unselected tabs show just an icon.

Don'ts:
- No more than one coloured (accent) button in a group.
- No icon-only button without a tooltip.
- Don't put destructive actions next to the primary action without a gap or a
  confirmation.

### 5.3 Text fields

Single-line fields, search fields and find bars all share one look
(see `find_bar` in `main.rs` and `media_library` in `inspector.rs`):

- Height 28–30, radius 8, `field` fill.
- 1 px `border` ring at rest; **2 px selection-colour ring when focused**.
- Text 14.0 in `text_hi`; hint text in the default weak colour, written as a short
  instruction: "Find in document", "Search media".
- Leading icon (magnifier) at 15 px from the left in `icon` colour; text starts at 28.
- Clear button: an 18 px circle on the right that appears only when the field has
  text. Tooltip "Clear". Clicking it clears and keeps focus.
- Status text inside the field (e.g. "Not found") sits right-aligned in 12.0
  `text_dim`, before the clear button.
- Enter / ⇧Enter step forward / back where it makes sense; Esc closes or clears.
- Draw the frame yourself and put an unframed `TextEdit` (`Frame::NONE`) inside it,
  so all fields match.

Numbers: use `egui::DragValue`. For small whole numbers use `stepper` (− value +),
24 × 24 buttons. Colours: use `hex_field` / `color_field`, not a free text box.

### 5.4 Selection controls

**Segmented control** — `segmented()` in `inspector.rs`. Track in `control`,
radius 7, 26 high. The selected segment is inset 2 px, radius 5, filled `field`
(light) or `tab_grey` (dark) with a 1 px border; text `text_hi`. Others are
`icon` coloured and get a `hover` fill. Use for two or three exclusive options.

**Dropdown** — `menu_combo` / `font_combo`. Full-width button, checkmark on the
current value, left-aligned rows. Style the popup with `tabletools::style_menu`.

**Checkbox / toggle** — use egui's checkbox (16 px icon, 6 px gap) in the inspector,
and `switch_row` in Settings (see 5.8). Label is a short statement of what turns
on: "Show speaking time".

**Slider** — `pill_slider`: 6 px track in `control`, filled part `ACCENT_BLUE`,
white 22 × 16 knob with a soft shadow and 0.5 px border, optional tick dots below.

### 5.5 Menus

Style every menu with `tabletools::style_menu(ui)`:

- Min width 240, rows 27 high, padding 8 × 5, 2 px gap between rows, radius 5.
- Rows are transparent at rest and get a `hover` fill.
- Shortcuts right-aligned with `menu_item(ui, label, shortcut)`; write them the Mac
  way and let `kbd()` convert them for Windows and Linux.
- Non-clickable notes use `menu_note` (dim, aligned with labels).
- Group items with separators; put destructive items last.

### 5.6 Inspector layout

- **Sections** use `section(ui, title)`: bold 13.5 title, a Hide/Show toggle on the
  right that only appears on hover. Separate groups with `divider(ui)`.
- **Form rows** use `labeled(ui, label, …)`: 80 px right-aligned `text_dim` label,
  control fills the rest.
- **List rows** (`item_row`): 26 high, radius 6, icon column at 20 px, label at
  46 px, shortcut right-aligned in 12.0 `text_dim`. Whole row is clickable, `hover`
  fill on hover, `button` fill while pressed.
- **Rich rows** (`export_row`): coloured 34 px tag tile, bold title, one-line
  description in `text_dim`, action button on the right. The whole row is clickable.

### 5.7 Cards, drop zones and banners

- **Card**: `field` fill, radius 8, 1 px `border`, 12 px inner padding.
- **Drop zone**: radius 10, dashed 1.2 px border. At rest the border is faint
  `text_dim`; on hover `hover` fill; while files are dragged over it the fill and
  border turn `ACCENT_BLUE` (fill at 10 %) and the text says "Release to add".
- **Banner / toast** (`update_banner`): floating `pill` frame, 1 px border, radius
  10, popup shadow, 14 × 10 padding, anchored under the top bar. Bold 13.5 title,
  12.0 `text_dim` detail, then buttons (primary first). Always offer a way to
  dismiss it.

### 5.8 Windows and dialogs

- In-app windows (Settings): no title bar, radius 12, 1 px border, window shadow,
  padding 20 / 14 / 18. Not resizable unless content needs it.
- **Settings pages** (`settings.rs`) are built from grouped cards on an
  `inspector_bg` window:
  - `heading`: bold 12.0 `text_mid()` title above each card, Title Case.
  - `card`: `field` fill, 1 px `border`, radius 10, 14 px side padding.
  - `row`: title 14.0 `text_hi` with an optional 12.0 `text_dim` description on
    the left, control right-aligned; at least 46 high. Rows are separated by
    `row_div` hairlines (`divider` light / `border` dark). A disabled row dims its
    title to `text_dim` and its control to 55 %.
  - `switch_row` / `switch`: on/off settings use a 34 × 20 switch (track
    `ACCENT_BLUE` when on, `border` light / `tab_grey` dark when off, white knob).
    The whole row toggles it.
  - `seg` / `seg_bool`: two to four exclusive choices, same look as `segmented`.
  - `dropdown`: `menu_combo` at a fixed width (140). Numbers use a 56 × 26
    `DragValue` with a dim unit label after it.
  - Prefer these over radio buttons and plain checkboxes in Settings.
- Questions that block work ("Save changes?") use native `rfd::MessageDialog` with
  a clear title and a one-sentence description that says what each choice does.
- File pickers use native `rfd` dialogs.

### 5.9 Slide rail

- Number badge: 10 px radius circle in `slide_color(i)`, white bold 11.0 number.
- Thumbnail: 1 px `border` hairline; selected gets a 2.5 px ring in the slide
  colour, hover a 2 px `hover` ring.
- Extra info (speaking time pill, present button) shows only while hovering. Pills
  are black at alpha 165 with white 10.5 text.

---

## 6. Icons

- Draw icons with `paint_icon` (vector strokes). Don't add image icons.
- **No emoji anywhere in the UI** — not in buttons, labels, menus, tooltips or
  status messages. Draw the icon with `paint_icon` instead, modelled on the
  matching SF Symbol (for example `Icon::Copy` follows `doc.on.doc`).
- egui's stock widgets can bring emoji with them (the built-in colour picker shows
  a 📋 copy button and a "U8" toggle). Don't use those widgets; build the control
  from our helpers (`color_field`, `hex_field`, `icon_button`).
- Monochrome text symbols that are part of typography are fine: keyboard symbols
  (⌘ ⇧ ⌥), the menu tick ✓, bullets and list markers.
- Stroke weight 1.3–2.0 px, round and simple; match the existing set
  (Picture, Brush, Play, chevrons, magnifier).
- Colour `icon` at rest, `text_hi` on hover, white on an accent fill.
- Keep icons centred in a square hit area of at least 24 × 24.

---

## 7. Interaction and behaviour

- **Cursor**: pointing hand on custom clickable areas; text cursor in fields.
- **Keyboard**: every main action has a shortcut. Show it in tooltips and menus.
  Esc closes the top-most bar, popup or mode.
- **Feedback**: changes apply immediately (no Save button in Settings or the
  inspector). Short status messages go to `self.status`. Errors say what failed and
  the item involved: "Couldn't open {url}".
- **Empty states**: tell people what to do next in one sentence
  (`EMPTY_SLIDE_GUIDANCE`).
- **Labels are not selectable** (`selectable_labels = false`), like native UI.
- **Scroll bars** float over content, 6 px wide, and take no layout space.
- **Motion**: keep it subtle; no bouncing or decorative animation. Hover expansion
  is 0.
- **Platform**: on macOS use ⌘ symbols and "Show in Finder"; elsewhere
  `kbd()` produces "Ctrl+…" and labels say "Show in Explorer" / "Show in Folder" (`REVEAL_LABEL`).

---

## 8. Checklist for a UI change

- [ ] Uses `pal()` tokens and existing accents; no new hard-coded colours.
- [ ] Looks right in **both** light and dark mode.
- [ ] Rest, hover, pressed, focus and disabled states are all handled.
- [ ] Icon-only controls have a tooltip with the shortcut (`kbd()`).
- [ ] No emoji in the UI; icons are drawn with `paint_icon`.
- [ ] Sizes, radii and fonts come from the tables above.
- [ ] Reuses an existing helper (`icon_button`, `flat_button`, `segmented`,
      `section`, `labeled`, `style_menu`, …) where one fits.
- [ ] Copy is short, plain and in the right case.
- [ ] Fresh Format and Design screenshots checked (see `AGENTS.md`).
