---
name: create-prcraft
description: Create, edit or check PresentCraft presentations (.prcraft files) and PresentCraft themes. Use when someone asks for a presentation, slide deck or talk for PresentCraft, wants notes or Markdown turned into a .prcraft file, wants slides, pictures, tables or charts added, wants a theme picked or designed (colours, fonts, gradients), or wants a .prcraft package checked or converted between folder and zip.
---

# Creating PresentCraft packages (.prcraft)

A `.prcraft` file is a [PresentCraft](https://github.com/saturngod/PresentCraft) presentation. It holds a Markdown script, settings, media, and optionally a theme.

This skill's folder also holds `scripts/prcraft.py`, a helper that builds and checks packages. It needs only Python 3 (standard library). Below, `SKILL_DIR` means the folder that holds this `SKILL.md`.

```text
Talk.prcraft/
├── text.md        the script (required, UTF-8)
├── info.json      document settings (optional)
├── assets/        pictures and videos, referenced as /assets/<name>
└── Theme/         optional theme folder (see Themes)
```

The same members can also be stored as a **single zip file** named `Talk.prcraft`, with the members at the zip's top level. PresentCraft opens both forms on every platform. Choose the form like this:
- **Folder:** the default, and best on macOS. It is easy to edit and diff.
- **Zip:** use it when the file will be shared or opened by double-click on Windows or Linux, or when the person asks for "a single file".

## Fast path

1. Write the script as a Markdown file following **Script rules** below. Reference local pictures by path.
2. Build the package with the helper. It copies media into `assets/` and rewrites the references:
   ```bash
   python3 "$SKILL_DIR/scripts/prcraft.py" build talk.md Talk.prcraft --theme Bagan --appearance dark --author "Name"
   ```
   Add `--zip` for a single file. Add `--theme-dir path/to/theme` to embed a custom theme.
3. Check the package. Fix every `error:` it reports, and read the slide summary to confirm each slide shows something:
   ```bash
   python3 "$SKILL_DIR/scripts/prcraft.py" check Talk.prcraft
   ```
4. Look at the result. If PresentCraft is installed, open the package in it:
   - **macOS:** `open Talk.prcraft`
   - **Windows:** double-click the file, or run `PresentCraft.exe Talk.prcraft`
   - **Linux:** double-click the file, or run `presentcraft Talk.prcraft`

   To check it without a person looking, have the app save a screenshot and close. Then look at the picture, especially the slide thumbnails on the left:
   ```bash
   # macOS
   /Applications/PresentCraft.app/Contents/MacOS/PresentCraft Talk.prcraft --tab design --screenshot talk.png
   # Windows: PresentCraft.exe …   Linux: presentcraft …   (same arguments)
   ```
   Add `--present --slide N` to capture slide N full-size, as the audience sees it.

Other helper commands:
- `zip FOLDER.prcraft [OUT.prcraft]` converts a folder to a zip.
- `unzip FILE.prcraft OUT.prcraft` converts a zip to a folder.
- `theme-check THEME_DIR` validates a theme folder (see **Themes**).
- `font "FAMILY" THEME_DIR/fonts` downloads a Google Fonts family for a theme.

You can also write the package by hand. Create the folder, write `text.md` and `info.json`, and put media in `assets/`. Then run `check`.

## Script rules (text.md)

PresentCraft splits the script into slides and decides what the audience sees. **Text is speaker notes unless it starts with a real tab character.**

| You write | Result |
| --- | --- |
| `---` on its own line (3+ dashes) | Starts a new slide |
| `# Title` … `#### Subheading` | Always visible on the slide |
| A paragraph or list starting with a **tab** (`\t`) | Visible body text |
| A paragraph, list or quote with no tab | Speaker notes only (not shown) |
| A path or URL alone on a line: `/assets/photo.jpg`, `https://…/x.png` | Picture. A `.mp4`, `.mov` or `.m4v` path is a video |
| `![Caption](/assets/photo.jpg)` alone on a line | Picture with a caption |
| A Markdown table | Visible table. A `[Caption]` line right after the table adds a caption |
| `Chart: bar` right after a table | The table is drawn as a chart |
| A line starting with `//` | Comment: neither shown nor spoken |
| A blank line between visible items | Separates layout groups. The number of groups picks the layout (title only, two columns, picture and text, …) |

- The tab must be a real tab character, not spaces and not the two characters `\t`. When writing the file from code, use a real `\t` escape in the string.
- **Inline formatting** works in visible text: `**bold**`, `*italic*`, `==highlight==`, `` `code` ``, `~~strike~~`, `[link](https://…)`, `$math$`.
- **Fenced code blocks** and `$$ … $$` math blocks are supported. A code fence's contents are never treated as media.
- **Picture options** go on the lines directly under the picture, one `key: value` per line:
  - `size: contain` fits the whole picture; the default fills the area.
  - `x: left|center|right`, `y: top|center|bottom`, or `position: top left`.
  - `background: true` puts the picture behind the slide text, full bleed.
  - `opacity: 40%`.
  - `filter: grayscale|sepia|darken|lighten|blur`.
  - `caption: Text`.
- **Cover title placement**: on the lines directly under an `#` heading, `x: left|center|right` and `y: top|middle|bottom` place the title and its subtitle on the cover. The default is `x: left`, `y: middle`. Only level-one headings take these; a `##` title alone is always centred.
- **Chart options** go on the lines directly under the table:
  - `Chart:` with a type: `bar`, `line`, `spline`, `area`, `step`, `pie`, `donut`, `radar`, `gauge` or `bubble`. Add `horizontal` and/or `stacked`, for example `Chart: bar, horizontal, stacked`.
  - For bars you can also write `Bar-Type: stacked` and `Orientation: horizontal` on their own lines.
  - `title:`, `xLabel:`, `yLabel:`, `caption:`.
  - `colors: #E63946, #457B9D`, or theme accents: `colors: accent 1, accent 3` (`Accent1`–`Accent6` from the theme's `presets.json`).
  - `Color-Type: sequential` uses shades of one colour (the first `colors:` entry, else the theme's first chart colour); `Color-Type: differential` runs from the first colour through a neutral middle to the second. Omit it for the default categorical palette.
  - `legend: off`.
  - The first column holds the labels and the other columns are series. Pie, donut and gauge charts use the first number column.

### Example

Indented lines below start with a real tab.

```markdown
# Quarterly Review
	Q3 2026 · Growth team

Welcome everyone — this line is a speaker note.

---
## One Number Matters

---
### Revenue by Region
| Region | Q1 | Q2 | Q3 |
|:--|--:|--:|--:|
| North | 12 | 18 | 24 |
| South | 9 | 14 | 21 |
Chart: bar
yLabel: $ thousands

North doubled since Q1; mention the new office.

---
/assets/team.jpg

### The Team
	- Six engineers
	- Two designers

---
/assets/sunset.jpg
background: true
opacity: 40%

## Thank You
```

### Writing a good deck

- Put one idea on each slide, and keep visible text short.
- Put the detail in speaker notes, the lines with no tab.
- Start with a cover slide: an `#` title plus a tabbed subtitle.
- Use a `##` heading alone on its own slide as a section divider.
- Prefer a chart to a table of numbers when the trend is the point.
- Every slide should have something visible. `check` flags slides that only have notes.

## info.json

- Four top-level keys: `creatorIdentifier`, `type`, `version` and `transient`.
- PresentCraft's settings go under `"app.myanmars.presentcraft"`.

All settings are optional. Write strict JSON. Unknown keys and blocks are kept when the app saves.

```json
{
  "creatorIdentifier": "app.myanmars.presentcraft",
  "type": "net.daringfireball.markdown",
  "version": 2,
  "transient": false,
  "app.myanmars.presentcraft": {
    "theme": "Bagan",
    "appearance": "dark",
    "author": "Ada Lovelace",
    "audience": "Engineering all-hands",
    "logo": "/assets/logo.png",
    "logoSize": 0.05,
    "slots": [
      { "kind": "Empty", "text": "" },
      { "kind": "Empty", "text": "" },
      { "kind": "Logo", "text": "" },
      { "kind": "Empty", "text": "" },
      { "kind": "Custom", "text": "ACME Corp" },
      { "kind": "Number", "text": "" }
    ],
    "deckGradient": {
      "light": { "enabled": false, "colors": ["#F6D94C", "#2463D4"] },
      "dark": { "enabled": true, "colors": ["#0B1020", "#3A1C71"] }
    }
  }
}
```

| Key (under `app.myanmars.presentcraft`) | Meaning |
| --- | --- |
| `theme` | A built-in theme name (see **Themes**). It is used only when there is no `Theme/` folder. |
| `appearance` | `"light"` or `"dark"`: which colour variant the slides use |
| `author`, `audience` | Metadata shown in the Design panel |
| `logo`, `logoSize` | The logo picture (put it in `assets/`), and its height as a fraction of slide height (about 0.02–0.08). The logo appears in slots set to `Logo` |
| `slots` | Six header/footer places, in this order: leading, middle and trailing header, then leading, middle and trailing footer. Each `kind` is `Empty`, `Number` (slide number), `Date`, `Logo` or `Custom` (uses `text`) |
| `deckGradient` | A per-deck gradient, set separately for light and dark slides; `colors` needs 2 or more `#RRGGBB` values. Gradient themes set this themselves, so leave it out unless you want to override one |

## Themes

A theme is the design of a deck: colours, typography and optional slide gradients. There are no pictures in a theme's look. A theme is a folder of JSON settings, a stylesheet and its fonts.

### Choosing a theme

Usually pick a built-in theme by name in `info.json` (`"theme": "Bagan"`) or with `build --theme Bagan`. Write a `Theme/` folder only when no built-in theme fits. A `Theme/` folder always wins over the `theme` name.

The built-in themes are named after Southeast Asian cities. Each has its own colours and its own Google Fonts pairing:

| Theme | Look | Titles / body | Style |
| --- | --- | --- | --- |
| Yangon (default) | lacquer maroon on ivory; gold on twilight indigo | Cormorant Garamond / Lato | covers in the dark look |
| Mandalay | vermilion on warm paper; gold on teak | Cinzel / EB Garamond | solid |
| Bagan | dawn: rose, apricot, amber | Marcellus / Nunito Sans | gradient |
| Taunggyi | pine green on mist; balloon flame on night navy | Fraunces / Nunito Sans | covers in the dark look |
| Singapore | Singapore red on white; night-bay navy | Space Grotesk / Inter | covers in the dark look |
| Bangkok | emerald, sapphire, ruby, gold; white titles | Prompt / Sarabun | gradient |
| Chiang Mai | forest green on mist; lantern orange on night | Lora / Source Sans 3 | solid |
| Hanoi | shutter green on mustard; mustard on deep green | Be Vietnam Pro | covers in the dark look |
| Hoi An | silk-lantern colours, dark by default | Playfair Display / Mulish | gradient |
| Da Nang | sea blues into lagoon green; gold on deep water | Be Vietnam Pro / Mulish | gradient |
| Kuala Lumpur | navy on cool white; steel on night, dark by default | Archivo / IBM Plex Sans | solid |
| Penang | Peranakan pastels; jewel tones when dark | Fraunces / Karla | gradient |
| Malacca | terracotta on lime-wash white; clay on maroon | Libre Baskerville / Lato | solid |
| Siem Reap | temple brown on sandstone; sunrise on pre-dawn | Spectral / Kantumruy Pro | covers in the dark look |
| Phnom Penh | royal blue on cream; gold on royal blue | Marcellus / Kantumruy Pro | solid |
| Luang Prabang | saffron on temple white and dark teak | Libre Baskerville / Work Sans | solid |
| Vientiane | dusk plum on white; champa yellow on dusk | Lora / Karla | solid |
| Bali | terrace greens to ripe gold | DM Serif Display / DM Sans | gradient |
| Jakarta | graphite and white, one orange accent, dark by default | Poppins / Inter | solid |
| Yogyakarta | sogan brown on cream; wax gold on indigo | Playfair Display / Source Sans 3 | covers in the dark look |
| Manila | bay blue to sunset; white titles | Oswald / Poppins | gradient |
| Cebu | mango into reef green; mango on deep reef | Oswald / DM Sans | gradient |
| Bandar Seri Begawan | black on marble white; dome gold on black | Cormorant Garamond / Work Sans | covers in the dark look |
| Dili | hill green on sea mist; sandalwood on forest night | Space Grotesk / IBM Plex Sans | solid |

Choose by mood and by the deck's text:
- **Formal** decks: Yangon, Mandalay, Luang Prabang, Phnom Penh, Yogyakarta, Bandar Seri Begawan, Malacca.
- **Modern** decks: Singapore, Kuala Lumpur, Jakarta, Dili.
- **Warm and colourful** decks: Bagan, Bangkok, Hoi An, Penang, Bali, Manila, Cebu.
- **Calm** decks: Chiang Mai, Siem Reap, Taunggyi, Vientiane, Da Nang.
- **Vietnamese** text: Hanoi, Hoi An or Da Nang, whose fonts cover Vietnamese.

Use `"appearance": "dark"` for a dark deck.

Only Latin-script text renders well. Burmese, Thai, Khmer and Lao need text shaping PresentCraft doesn't do yet.

The app writes its built-in theme folders to a `Built-in Themes` folder next to its settings. They make good starting points for a custom theme:
- **macOS:** `~/Library/Application Support/PresentCraft/Built-in Themes/`
- **Windows:** `%APPDATA%\PresentCraft\Built-in Themes\`
- **Linux:** `~/.config/presentcraft/Built-in Themes/`

Copy one, rename the folder and its `Name`, then edit it.

### Theme folder

```text
Harbor/                     (or Theme/ inside a package)
├── template.json           required: identity, fonts, layouts
├── presets.json            colour variants (light and dark)
├── harbor.css              stylesheet named by "Css" (holds the @font-face rules)
├── fonts/                  .ttf/.otf files + their licence files
│   ├── Lora-Bold.ttf
│   ├── SourceSans3-Regular.ttf, -Bold.ttf, -Italic.ttf
│   └── Lora-OFL.txt, SourceSans3-OFL.txt
├── template.png            500 × 281 preview (also template.webp); the app draws one if missing
└── image1.webp …           optional pictures, usable in slides as /Theme/image1.webp
```

When an app saves a theme from a folder, it keeps every file and every key it doesn't use. It only rewrites values that changed, and colours keep their original spelling (`#000` or `transparent` stay as written).

### template.json

```json
{
  "Name": "Harbor",
  "Version": "1.0",
  "Author": "Your name",
  "ShortDescription": "Calm blues by the water.",
  "LongDescription": "Calm blues by the water.\n- Navy on sea-salt white\n- Fonts: Lora and Source Sans 3",
  "Css": "harbor.css",
  "TitleFont": "Lora",
  "BodyFont": "Source Sans 3",
  "CssClasses": "variable-size-headings",
  "LayoutExamples": [],
  "Layouts": [
    { "Name": "Cover", "Classes": "invert" },
    { "Name": "Title", "Classes": "invert" }
  ],
  "TextSize": 1
}
```

| Key | Meaning |
| --- | --- |
| `Name`, `Version`, `Author` | Identity; `Name` appears in the theme menu. |
| `ShortDescription`, `LongDescription` | One line, then details with `- ` bullet lines. |
| `Css` | The stylesheet, named after the theme in lower case (`"My Theme"` → `my theme.css`). |
| `TitleFont`, `BodyFont` | **Display names** of the fonts (`Lora`, `System`, `Serif`). |
| `CssClasses` | Style classes. `variable-size-headings` is usual; add `has-graph-background` for gradient themes. |
| `Layouts` | Per-layout classes. `invert` on `Cover` and `Title` shows those slides in the other appearance. |
| `LayoutExamples` | Optional example slides (`{"Name", "Markdown"}`). |
| `TextSize` | PresentCraft only: 0 small to 3 extra large; 1 is medium. |

PresentCraft also accepts two shorthand keys. `InvertTitles: true` means the same as the two `invert` layouts, and `Decoration` (`None`, `Bar`, `Glow`, `Frame`, `Band`, `Shapes`) adds a simple ornament; leave it `None` for a pure design theme.

### presets.json

```json
{
  "Presets": [
    {
      "Name": "Light",
      "Appearance": "light",
      "TitleFont": "Lora",
      "BodyFont": "Source Sans 3",
      "LightBackgroundColor": "#F6F8FA",
      "DarkTitleTextColor": "#0B2545",
      "DarkBodyTextColor": "#2E3A48",
      "DarkBackgroundColor": "#0B2545",
      "LightTitleTextColor": "#FFFFFF",
      "LightBodyTextColor": "#D5DEE8",
      "Accent1": "#EF8354", "Accent2": "#2A9D8F", "Accent3": "#E9C46A",
      "Accent4": "#8DA9C4", "Accent5": "#4F5D75", "Accent6": "#D96C75"
    },
    { "Name": "Dark", "Appearance": "dark", "…": "the same keys and values" }
  ]
}
```

**Colour naming is the tricky part.** The names say what colour the value is, not which slides use it:
- **Light slides** use `LightBackgroundColor`, with the `DarkTitleTextColor` and `DarkBodyTextColor` text colours.
- **Dark slides** use `DarkBackgroundColor`, with the `LightTitleTextColor` and `LightBodyTextColor` text colours.
- Write the **same colour values in every preset**. Presets differ only in `Name`, `Appearance` and, optionally, accents.
- **The first preset listed** is the look the deck starts in. A theme may have a single `"Default"` preset.

| Key | Meaning |
| --- | --- |
| `TitleFont`, `BodyFont` | **CSS names**: the `font-family` from your `@font-face` rules (`Lora`), or a CSS list such as `-apple-system-ui-serif, ui-serif`. |
| `Accent1`–`Accent6` | Accent colours. They are used for highlights, bullets, chart series and rules. A `Dark` preset may use its own set. |
| `DarkAccent1`, `LightAccent1` | The first accent for light slides and for dark slides. |
| `LightBgGradient`, `DarkBgGradient` | 2 or more colours. Each slide takes the next colour along the gradient, from the first slide to the last. Set the background colours to `"transparent"` when you use these. |
| `LightGraphDataColors`, `DarkGraphDataColors` | Chart series colours: `{"data-0": "#EA3E55", "data-1": "#FFC400", …}`. Without them, charts use the accents. |

Colours are written `#RGB`, `#RRGGBB`, `#RRGGBBAA` (see-through, for example `#000000b3` is 70% black) or `transparent`.

### Fonts

Typography matters as much as colour. Give every theme its own pairing:
- A **title face** with character: a serif, a display face, or a condensed sans.
- A **readable body face** in regular, bold and italic.

1. **Download from Google Fonts.** The helper fetches static `.ttf` files and the licence, and prints the `@font-face` rules:
   ```bash
   python3 "$SKILL_DIR/scripts/prcraft.py" font "Lora" Harbor/fonts --weights 700
   python3 "$SKILL_DIR/scripts/prcraft.py" font "Source Sans 3" Harbor/fonts --weights 400,700 --italic
   ```
   Paste the printed rules at the top of the theme's CSS file.
2. **Use `.ttf` or `.otf` files.** PresentCraft draws text itself and can't read `.woff`/`.woff2` web fonts.
3. **Keep the licence file** next to the fonts. Google Fonts are SIL OFL or Apache licensed, which allows bundling.
4. **Use the same family name** in `template.json` (display name), `presets.json` (CSS name) and the `@font-face` rules. It must match the name inside the font file. `theme-check` reports a mismatch.
5. Without bundled files, `System`/`system-ui` is the built-in sans and `Serif`/`ui-serif` the built-in serif. Another name works only if the font is installed on the computer showing the deck.

### Designing a theme

- **Readability first.** Aim for these contrast ratios against every background the text can sit on:
  - Body text: at least 4.5:1.
  - Titles: at least 3:1.
  - Gradients: check every stop, because each slide uses a different one. `theme-check` measures all of these.
- **Both looks.** Design the light and the dark look together. Same hues, with lightness reversed.
- **Gradients:**
  - Use 5–11 stops that change hue gradually while keeping similar lightness, so one text colour works on every slide.
  - Use white text on mid-to-deep stops, or dark text on pastels.
- **Inverted covers** (`invert` layouts) give a deck a strong opening, for example a dark cover before light content slides.
- **Accents:** pick six that work on both backgrounds. `Accent1` is the most visible.

### Checking a theme

```bash
python3 "$SKILL_DIR/scripts/prcraft.py" theme-check Harbor
```

It reports:
- missing or invalid JSON and malformed colours;
- the stylesheet's status;
- fonts that aren't bundled or are web-only;
- `@font-face` rules missing for bundled fonts;
- low-contrast text in either look.

`check` runs the same checks on a package's `Theme/` folder. Fix every `error:` and read every `warning:`.

To see a theme, put it in a package as `Theme/` (or build with `--theme-dir Harbor`), then screenshot both looks:
1. Set `"appearance": "light"` in `info.json` and take a screenshot.
2. Set `"appearance": "dark"` and take another.

```bash
# macOS
/Applications/PresentCraft.app/Contents/MacOS/PresentCraft Talk.prcraft --present --slide 1 --screenshot cover.png
```

To use it in the app for every deck, add it with Settings → Themes → Import Theme….

## Rules that keep packages working

- Put every local picture or video in `assets/` and reference it as `/assets/<name>`. Write spaces as `%20`, for example `/assets/team%20photo.jpg`. Absolute paths and paths outside the package break on another computer.
- Web addresses (`https://…`) are fine. The app downloads and caches them.
- Keep `text.md` as UTF-8 with LF line endings.
- Never put PresentCraft-only data anywhere other than the `app.myanmars.presentcraft` block of `info.json` or `Theme/`. The app preserves other files in a folder package but doesn't read them.
- **Zip form:** members go at the top level (`text.md`, not `Talk/text.md`, though the app accepts one wrapping folder). Store media uncompressed and Deflate text. The helper's `--zip` and `zip` commands do this.
- **Editing an existing package:** change `text.md` and add files to `assets/`, leaving other members alone. For a zip, `unzip` it, edit the folder, then `zip` it back.

## When the PresentCraft MCP tools are available

If the client lists PresentCraft's MCP tools (`deck_info`, `slide_add`, `insert_picture`, …), prefer them over editing files by hand when the app is open: changes appear in the window at once and each one can be undone with `undo_ai_change`. Slide numbers start at 1. Call `deck_info` first, and check the result with `slide_screenshot` and `deck_check`. Without the app, the script tools still edit a folder-form package when given its path in the `package` argument.
