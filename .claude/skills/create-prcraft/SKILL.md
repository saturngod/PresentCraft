---
name: create-prcraft
description: Create, edit or check PresentCraft presentation packages (.prcraft). Use when asked to make a presentation, slide deck or talk for PresentCraft, to turn notes or Markdown into a .prcraft file, to add slides, pictures, tables or charts to one, or to validate or convert (folder/zip) a .prcraft package. Also covers importing iA Presenter-style content.
---

# Creating PresentCraft packages (.prcraft)

A `.prcraft` file is a PresentCraft presentation. It holds a Markdown script, settings, media, and optionally a theme. The layout matches iA Presenter's `.iapresenter` packages.

```text
Talk.prcraft/
├── text.md        the script (required, UTF-8)
├── info.json      document settings (optional)
├── assets/        pictures and videos, referenced as /assets/<name>
└── Theme/         optional custom theme: template.json + presets.json
```

The same members can also be stored as a **single zip file** named `Talk.prcraft`, with the members at the zip's top level. PresentCraft opens both forms on every platform. Choose the form like this:
- **Folder:** the default, and best on macOS. It is easy to edit and diff.
- **Zip:** use it when the file will be shared or opened by double-click on Windows or Linux, or when the person asks for "a single file".

## Fast path

1. Write the script as a Markdown file following **Script rules** below. Reference local pictures by path.
2. Build the package with the helper. It copies media into `assets/` and rewrites the references:
   ```bash
   python3 .claude/skills/create-prcraft/scripts/prcraft.py build talk.md Talk.prcraft --theme Midnight --appearance dark --author "Name"
   ```
   Add `--zip` for a single file. Add `--theme-dir path/to/theme` to embed a custom theme.
3. Check the package. Fix every `error:` it reports, and read the slide summary to confirm each slide shows something:
   ```bash
   python3 .claude/skills/create-prcraft/scripts/prcraft.py check Talk.prcraft
   ```
4. In this repository, look at the result in the app:
   ```bash
   cargo run --locked --bin PresentCraft -- Talk.prcraft --tab design --screenshot shots/talk.png
   ```
   Open `shots/talk.png` and check the slide rail on the left. Add `--present` to start in presentation mode, or `--slide N` to select a slide.

Other helper commands:
- `zip FOLDER.prcraft [OUT.prcraft]` converts a folder to a zip.
- `unzip FILE.prcraft OUT.prcraft` converts a zip to a folder.

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
- **Chart options** go on the lines directly under the table:
  - `Chart:` with a type: `bar`, `line`, `spline`, `area`, `step`, `pie`, `donut`, `radar`, `gauge` or `bubble`. Add `horizontal` and/or `stacked`, for example `Chart: bar, horizontal, stacked`.
  - `title:`, `xLabel:`, `yLabel:`, `caption:`.
  - `colors: #E63946, #457B9D`.
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

The layout matches iA Presenter's `info.json`:
- Four top-level keys: `creatorIdentifier`, `type`, `version` and `transient`.
- PresentCraft's settings go under `"app.myanmars.presentcraft"`.
- An optional `"net.ia.presenter"` block holds iA Presenter's own settings.

All settings are optional. Write strict JSON. Unknown keys and blocks are kept when the app saves.

```json
{
  "creatorIdentifier": "app.myanmars.presentcraft",
  "type": "net.daringfireball.markdown",
  "version": 2,
  "transient": false,
  "app.myanmars.presentcraft": {
    "theme": "Midnight",
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
| `theme` | A theme name. It is used only when there is no `Theme/` folder. Built-in themes: `Swiss`, `Contrast Serif`, `Paper`, `Signal`, `Graphite`, `Studio` (the default), `Midnight`, `Editorial`, `Corporate`, `Sunset` |
| `appearance` | `"light"` or `"dark"`: which colour variant the slides use |
| `author`, `audience` | Metadata shown in the Design panel |
| `logo`, `logoSize` | The logo picture (put it in `assets/`), and its height as a fraction of slide height (about 0.02–0.08). The logo appears in slots set to `Logo` |
| `slots` | Six header/footer places, in this order: leading, middle and trailing header, then leading, middle and trailing footer. Each `kind` is `Empty`, `Number` (slide number), `Date`, `Logo` or `Custom` (uses `text`) |
| `deckGradient` | Background colours that blend from the first to the last slide, set separately for light and dark slides. `colors` needs 2 or more `#RRGGBB` values |

### iA Presenter's block

The app reads iA Presenter's settings and keeps them up to date when it saves. You don't need to write them. When you do (or when converting an `.iapresenter`), use iA's keys:

```json
"net.ia.presenter": {
  "template": "tokyo",
  "preset": "Default",
  "logo": "/assets/logo.png",
  "logo-size": "7.5",
  "headertrailing": "logo",
  "footertrailing": "slideNumber"
}
```

- **`template` and `preset`:** an iA Presenter theme folder name and its preset name. They are used only when there is no `Theme/` folder and no `theme` key. The app loads the theme from an installed iA Presenter, or from a theme with that name imported in Settings → Themes.
- **`logo-size`:** a percentage of slide height, written as text (`"7.5"` means 0.075).
- **Header and footer places:** the keys are `headerleading`, `headermiddle`, `headertrailing`, `footerleading`, `footermiddle` and `footertrailing`. Each value is `logo`, `date`, `slideNumber`, or your own custom text.

## Custom theme (Theme/)

Include a `Theme/` folder only for a look the built-in themes can't give. A `Theme/` folder overrides `info.json`'s `theme`. The format is iA Presenter's, so an iA Presenter theme folder can be copied in as it is.

```text
Theme/
├── template.json    required
├── presets.json     colour variants
├── <name>.css       stylesheet named by template.json "Css" (the app writes one if missing)
├── template.png     500 × 281 preview, optional (the app draws one if missing)
├── fonts/           optional .ttf/.otf files named by TitleFont/BodyFont
└── image1.webp …    optional pictures, referenced from slides as /Theme/image1.webp
```

`Theme/template.json`:
```json
{
  "Name": "Harbor",
  "Version": "1.0",
  "Author": "PresentCraft",
  "TitleFont": "Serif",
  "BodyFont": "System",
  "ShortDescription": "Calm blues",
  "LongDescription": "",
  "Css": "harbor.css",
  "CssClasses": "",
  "LayoutExamples": [],
  "Decoration": "Bar",
  "TextSize": 1,
  "InvertTitles": false
}
```

`Theme/presets.json`:
```json
{
  "Presets": [
    {
      "Name": "Light",
      "Appearance": "light",
      "DarkTitleTextColor": "#0B2545",
      "DarkBodyTextColor": "#13315C",
      "LightBackgroundColor": "#F4F7FB",
      "LightTitleTextColor": "#FFFFFF",
      "LightBodyTextColor": "#DCE6F2",
      "DarkBackgroundColor": "#0B2545",
      "Accent1": "#EF8354", "Accent2": "#4F5D75", "Accent3": "#2D3142",
      "Accent4": "#BFC0C0", "Accent5": "#8DA9C4", "Accent6": "#134074"
    },
    { "Name": "Dark", "Appearance": "dark", "...": "same colour keys" }
  ]
}
```

- **`Dark…` and `Light…` colours:**
  - **Light slides** use `LightBackgroundColor` with the `Dark…Text` colours.
  - **Dark slides** use `DarkBackgroundColor` with the `Light…Text` colours.
  - Write the same colour values in both presets.
- **Which variant shows first:** the first preset listed is the one the deck starts in.
- **Colour values:** `#RGB` or `#RRGGBB`.
- **Background gradients:** add `"LightBgGradient": [...]` and/or `"DarkBgGradient": [...]`, each a list of 2 or more colours.
- **Fonts:**
  - `System` is the built-in sans and `Serif` the built-in serif.
  - Another name, such as `Helvetica`, is used if that font is installed on the computer.
  - Serif-sounding names otherwise fall back to the built-in serif.
- **PresentCraft-only keys** (iA Presenter ignores them):
  - `Decoration`: `None`, `Bar`, `Glow`, `Frame`, `Band` or `Shapes`.
  - `TextSize`: 0–3, small to extra large.
  - `InvertTitles`: cover and title slides swap the text and background colours.

## Rules that keep packages working

- Put every local picture or video in `assets/` and reference it as `/assets/<name>`. Write spaces as `%20`, for example `/assets/team%20photo.jpg`. Absolute paths and paths outside the package break on another computer.
- Web addresses (`https://…`) are fine. The app downloads and caches them.
- Keep `text.md` as UTF-8 with LF line endings.
- Never put PresentCraft-only data anywhere other than the `app.myanmars.presentcraft` block of `info.json` or `Theme/`. The app preserves other files in a folder package but doesn't read them.
- **Zip form:** members go at the top level (`text.md`, not `Talk/text.md`, though the app accepts one wrapping folder). Store media uncompressed and Deflate text. The helper's `--zip` and `zip` commands do this.
- **Editing an existing package:** change `text.md` and add files to `assets/`, leaving other members alone. For a zip, `unzip` it, edit the folder, then `zip` it back.
