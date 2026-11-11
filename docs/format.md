# Document and theme format

## Cover titles

A slide that starts with an `#` heading is a cover. Its title and subtitle sit on the left, in the middle of the slide. To move them, put `x:` and `y:` lines directly under the heading:

```markdown
# Markdown Slides
x: left
y: bottom
	Saturngod
```

`x:` takes `left`, `center` or `right`; `y:` takes `top`, `middle` or `bottom`. Other values are read as speaker notes. You can also set them from the toolbar that appears when the cursor is on the heading: three buttons for left, centre and right, three for top, middle and bottom, and the same choices in its ••• menu. Only level-one headings take these options; a `##` heading alone on a slide is always centred.

## Pictures and videos

Local image paths are resolved relative to the document. Web pictures (`https://…`) download in the background with the system's `curl` (built into macOS, Windows 10+ and most Linux systems) and are cached in the temp folder under `presentcraft-web`; exports wait for them. Pictures that can't be loaded display a placeholder.

When the document is saved, pictures and videos you add or drop that sit in the same folder (or below it) are written as `./name.ext`; others use the full path, and spaces are written as `%20`. Videos show a frame from the video with a play button. While presenting, click the video or press **V** to play or pause it on the slide; a control bar shows play / pause, the time and progress (click the bar to jump), and the video stops when you change slides. **Settings → Presentation → Content** turns the control bar off (**Show controls**) and makes each slide's video start by itself (**Auto play**); both also apply to HTML exports. A YouTube link (`![](https://www.youtube.com/watch?v=…)` or `https://youtu.be/…`, with an optional `t=1m30s` start) is a video too: it shows YouTube's preview picture, and while presenting it plays in YouTube's own embedded player, placed over the video on the slide (nothing is downloaded). The player uses the system's web view on macOS and Windows; on Linux the video opens in the browser. **Show controls** shows or hides YouTube's controls, the arrow keys and Escape still work while the player has the keyboard, and **V** closes it. A YouTube video can't be a background video. In HTML exports a YouTube video becomes the embedded player when the page is served over http(s), and a link to YouTube when the file is opened directly. Background videos (`background: true`) start by themselves, silent and looping. Video uses `ffmpeg`: release packages bundle their own small copy; when running from source, install it (`brew install ffmpeg`, `winget install ffmpeg`, or your Linux package manager). Sound plays with `ffplay` if installed, otherwise through the system (`afplay` on macOS, Windows' built-in player, `pw-play`/`paplay`/`aplay` on Linux). Without ffmpeg, the preview frame comes from Quick Look on macOS and videos open in the system's video player instead.

## Videos in exports

- **HTML:** videos play on the page, at the same place and crop as in the app. They are copied into a `<name>-media` folder next to the `.html` file, so keep the two together. Background videos start by themselves (silent, looping); other videos have play controls, or press **V**. On slides with a background video, the text is real HTML text drawn above the video.
- **PowerPoint:** videos are embedded in the `.pptx` (each file once) with their still frame as the cover, and play when clicked in the slideshow. Background videos also play on click there, and picture filters only change the cover, not the playing video.
- **YouTube in PowerPoint:** a YouTube video is written as a PowerPoint online video, with its preview picture as the cover. In the slideshow it plays in YouTube's player on the slide and needs an internet connection. Apps that don't support online videos show the cover picture. Background YouTube videos stay a picture.
- **PDF and images** show the video's still frame.

## Design overrides

Themes bundle colours, title and body fonts, and text size. Selecting a theme applies the full bundle, replacing manual design overrides. You can adjust fonts and size in Design afterward; Duplicate Theme in Settings captures those choices in a custom theme.

In **Design → Colors**, click a **Background** swatch and choose **Gradient**. Pick a preset or edit the color stops in the picker; choose **Solid** to return to the theme’s background colors. Light and dark appearances have independent background modes and gradient colors. The first slide uses the first color, the last uses the last color, and slides between them blend evenly through the color stops. Add or remove colors, or reverse their order. A one-slide deck uses the first color. Text automatically switches between black and white for contrast; theme decorations and cover inversion are replaced by the chosen background, while background pictures and videos still appear on top. The gradient applies to thumbnails, presenting, and all exports. Like other Design overrides, it lasts for the current session; selecting a theme turns it off, and Duplicate Theme saves the base theme without the deck gradient.

In **Design → Logo**, select an image or drop it onto the logo preview, then adjust **Size**. The presentation has one logo: in **Design → Header & Footer**, set any of the six places to **Logo** to show it there — one place or several. Logos keep their aspect ratio and appear in presentations and all export formats. **Remove Logo** clears the image everywhere. Like the other Design controls, these choices apply to the current presentation session.

## Packages

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

## Theme folders

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

When a package has no `Theme/` folder, `info.json`'s `Theme` key picks an installed theme by name.

## Opening documents by double-click

- **macOS:** the release bundle registers `.prcraft` (folder and zip) and offers PresentCraft for Markdown.
- **Windows:** in the unpacked release folder, run `powershell -ExecutionPolicy Bypass -File register-file-types.ps1` once. It registers `.prcraft` for the current user (no administrator rights needed) and adds PresentCraft to *Open with* for Markdown. Run it again if you move the folder; add `-Remove` to undo.
- **Linux:** in the unpacked release folder, run `./install.sh` once. It adds the menu entry, icons and the `application/x-presentcraft` file type under `~/.local/share`. `./install.sh --remove` undoes it.

A folder package from a Mac is an ordinary folder on Windows and Linux. Open it with File → Open Package Folder…, by dragging it onto the window, or by passing its path on the command line.
