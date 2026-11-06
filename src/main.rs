// Release builds on Windows run as a windowed app (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod chart;
mod control;
mod docinfo;
mod export;
mod find;
mod fonts;
mod images;
mod imagetools;
mod inline;
mod inspector;
mod json;
mod math;
mod mechanics;
mod optionicons;
#[cfg(target_os = "macos")]
mod macos_open;
mod package;
mod parse;
mod presenter;
mod render;
mod settings;
mod syntax;
mod tableblock;
mod tableeditor;
mod tabletools;
mod theme;
mod titletools;
mod themefile;
mod update;
mod video;

use eframe::egui;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{pos2, vec2, Color32, FontId, Key, Modifiers, Rect, RichText, Sense, Stroke};
use images::ImageCache;
use parse::{parse_document, slide_at_char, Slide};
use render::{draw_slide, Ctx, ASPECT};
use std::path::PathBuf;
use theme::{hex, Palette, Preset, Theme};

const SLIDE_TEMPLATES: &[(&str, &[(&str, &str)])] = &[
    (
        "Text Slides",
        &[
            ("Title", "# Title"),
            ("Subtitle", "## Subtitle"),
            ("Heading", "### Heading"),
            ("Subheading", "#### Subheading"),
        ],
    ),
    (
        "Text and Image Slides",
        &[
            ("Title and Image", "# Title\n\nimage.jpg"),
            ("Subtitle and Image", "## Subtitle\n\nimage.jpg"),
            ("Heading and Image", "### Heading\n\nimage.jpg"),
            ("Image and Caption", "image.jpg\n\n\tCaption"),
            ("Caption and Image", "\tCaption\n\nimage.jpg"),
        ],
    ),
    (
        "Image Slides",
        &[
            ("Image", "image.jpg"),
            ("Two Images", "image-1.jpg\n\nimage-2.jpg"),
            ("Three Images", "image-1.jpg\n\nimage-2.jpg\n\nimage-3.jpg"),
            (
                "Four Images",
                "image-1.jpg\n\nimage-2.jpg\n\nimage-3.jpg\n\nimage-4.jpg",
            ),
        ],
    ),
    (
        "Background Image Slides",
        &[
            (
                "Title with Background",
                "# Title\n\nimage.jpg\nBackground: true",
            ),
            (
                "Subtitle with Background",
                "## Subtitle\n\nimage.jpg\nBackground: true",
            ),
        ],
    ),
    (
        "Columns Slides",
        &[
            ("Three Columns", "\tColumn 1\n\n\tColumn 2\n\n\tColumn 3"),
            ("Title and 2 columns", "# Title\n\n\tColumn 1\n\n\tColumn 2"),
            (
                "Title And 3 Columns",
                "# Title\n\n\tColumn 1\n\n\tColumn 2\n\n\tColumn 3",
            ),
        ],
    ),
    (
        "Tables and Charts Slides",
        &[
            (
                "Table",
                "| Item | Value |\n| --- | --- |\n| A | 10 |\n| B | 20 |",
            ),
            (
                "Bar Chart",
                "| Item | Value |\n| --- | --- |\n| A | 10 |\n| B | 20 |\nChart: Bar",
            ),
            (
                "Line Chart",
                "| Item | Value |\n| --- | --- |\n| A | 10 |\n| B | 20 |\nChart: Line",
            ),
        ],
    ),
];

const SAMPLE: &str = include_str!("sample.md");
const EDITOR_ID: &str = "markdown-editor";
const FIND_FIELD_ID: &str = "find-field";
/// Height of the find bar under the editor's top bar.
const FIND_BAR: f32 = 44.0;
const SCREENSHOT_FRAMES: u32 = 120;

/// The last captured window picture, filled in by egui's screenshot callback.
static SHOT: std::sync::Mutex<Option<std::sync::Arc<egui::ColorImage>>> =
    std::sync::Mutex::new(None);

/// Asks egui for a picture of the window; it arrives via `take_shot`.
fn request_shot(ctx: &egui::Context) {
    let repaint = ctx.clone();
    ctx.request_screenshot(move |image| {
        *SHOT.lock().unwrap() = Some(image);
        repaint.request_repaint();
    });
}

fn take_shot() -> Option<std::sync::Arc<egui::ColorImage>> {
    SHOT.lock().unwrap().take()
}

// Tab colours: they mark where you are, nothing else.
pub const FORMAT_BLUE: Color32 = hex(0x59BDFF);
pub const DESIGN_PINK: Color32 = hex(0xFC44B2);
/// The one action colour: primary buttons, links, sliders, switches,
/// selection, focus and drag-over, in every tab.
pub const ACCENT_BLUE: Color32 = hex(0x3770FF);
const TOP_BAR: f32 = 52.0;
pub const SIDEBAR_W: f32 = 180.0;
pub const SIDEBAR_MIN: f32 = 150.0;
pub const SIDEBAR_MAX: f32 = 420.0;
pub const INSPECTOR_W: f32 = 307.0;
const EDITOR_COLUMN: f32 = 700.0;
const EDITOR_FONT: f32 = 18.5;
const EDITOR_LINE: f32 = 31.7;
/// Gap before an option line, in editor-font units, that holds its icon.
const OPTION_GAP: f32 = 1.5;
/// Default words per minute used for the speaking-time estimate.
const SPEECH_WPM: f32 = mechanics::SPEECH_WPM as f32;

/// Slide number colour ramp; reverses after 15 slides.
const RAMP: [u32; 15] = [
    0x63C2EF, 0x72B8F3, 0x7FAEF7, 0x8BA4FA, 0x959AFD, 0x9E8FFF, 0xA784FF, 0xAF7EFF, 0xB87FF3,
    0xC17FE7, 0xCA7EDB, 0xD47ECF, 0xDD7DC3, 0xE67BB7, 0xE682AA,
];

/// The same hues, deepened so text stays readable on the light editor.
const LIGHT_RAMP: [u32; 15] = [
    0x227DA8, 0x226BA8, 0x2256A8, 0x2240A8, 0x2228A8, 0x3422A8, 0x4822A8, 0x5522A8, 0x6422A8,
    0x7722A8, 0x9022A8, 0xA822A0, 0xA82284, 0xA8226D, 0xA82258,
];

pub fn slide_color(i: usize) -> Color32 {
    let n = RAMP.len();
    let period = 2 * (n - 1);
    let k = i % period;
    let idx = if k < n { k } else { period - k };
    if settings::is_dark() {
        hex(RAMP[idx])
    } else {
        hex(LIGHT_RAMP[idx])
    }
}

/// See `App::media_ref`.
pub fn media_ref_for(doc_dir: Option<&std::path::Path>, file: &std::path::Path) -> String {
    let text = match doc_dir.and_then(|d| file.strip_prefix(d).ok()) {
        Some(rel) => {
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            format!("./{}", parts.join("/"))
        }
        None => file.to_string_lossy().into_owned(),
    };
    text.replace(' ', "%20")
}

/// Opens a file or web address with the system's default app.
pub fn open_external(target: &std::ffi::OsStr) -> bool {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        std::process::Command::new("explorer")
    } else {
        std::process::Command::new("xdg-open")
    };
    cmd.arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // explorer returns 1 even when it worked, so only a failed start counts.
    cmd.spawn().is_ok()
}

/// True on macOS, where the window has no title bar and shortcuts use ⌘.
pub const IS_MAC: bool = cfg!(target_os = "macos");

/// A shortcut written the macOS way ("⇧⌘L", "^⌘K", "⌘↵") in the local form:
/// unchanged on macOS, "Ctrl+Shift+L" style on Windows and Linux.
/// (⌃⌘ shortcuts become Ctrl+Alt elsewhere, where Ctrl and ⌘ are one key.)
pub fn kbd(mac: &str) -> String {
    if IS_MAC || !mac.contains(['⌘', '⇧', '⌥', '^']) {
        return mac.to_string();
    }
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    let mut key = String::new();
    for ch in mac.chars() {
        match ch {
            '⌘' => ctrl = true,
            '⇧' => shift = true,
            '⌥' | '^' => alt = true,
            '↵' | '↩' => key.push_str("Enter"),
            c => key.push(c),
        }
    }
    let mut out = String::new();
    for (on, name) in [(ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            out.push_str(name);
        }
    }
    out + &key
}

/// Dim, non-clickable text inside a menu, aligned with the item labels.
fn menu_note(ui: &mut egui::Ui, text: &str) {
    let pad = ui.spacing().button_padding.x as i8;
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(pad, 3))
        .show(ui, |ui| ui.weak(text));
}

/// A menu entry with its keyboard shortcut right-aligned.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    let mut b = egui::Button::new(label);
    if !shortcut.is_empty() {
        b = b.shortcut_text(kbd(shortcut));
    }
    ui.add(b).clicked()
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp") {
        // The MCP server AI clients start over stdio; it never opens a window.
        std::process::exit(control::shim::run());
    }
    let mut screenshot = None;
    let mut settings_page = None;
    let mut blank = false;
    let mut light_ui = false;
    let mut dark_ui = false;
    let mut present = false;
    let mut play_video = false;
    let mut file = None;
    let mut start_slide = 0usize;
    let mut theme_idx: Option<usize> = None;
    let mut tab: Option<inspector::Tab> = None;
    let mut export_req: Option<(export::Format, PathBuf)> = None;
    let mut cursor_line: Option<usize> = None;
    let mut find_query: Option<String> = None;
    let mut start_mcp = false;
    let mut i = 1;
    while i < args.len() {
        let next = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--settings" => {
                settings_page = Some(match next.as_str() {
                    "editor" => settings::Page::Editor,
                    "presentation" => settings::Page::Presentation,
                    "themes" => settings::Page::Themes,
                    "markdown" => settings::Page::Markdown,
                    _ => settings::Page::General,
                });
                i += 2;
            }
            "--start-mcp" => {
                start_mcp = true;
                i += 1;
            }
            "--new" => {
                blank = true;
                i += 1;
            }
            "--light" => {
                light_ui = true;
                i += 1;
            }
            "--dark" => {
                dark_ui = true;
                i += 1;
            }
            "--present" => {
                present = true;
                i += 1;
            }
            "--play-video" => {
                present = true;
                play_video = true;
                i += 1;
            }
            "--screenshot" => {
                screenshot = Some(PathBuf::from(next));
                i += 2;
            }
            "--slide" => {
                start_slide = next.parse::<usize>().unwrap_or(1).saturating_sub(1);
                i += 2;
            }
            "--theme" => {
                theme_idx = Some(next.parse::<usize>().unwrap_or(1).saturating_sub(1));
                i += 2;
            }
            "--export" => {
                // --export pdf|pptx|images|html <path>   (for testing; quits when done)
                let f = match next.as_str() {
                    "pptx" => export::Format::PowerPoint,
                    "images" => export::Format::Images,
                    "html" => export::Format::Html,
                    _ => export::Format::Pdf,
                };
                export_req = Some((
                    f,
                    PathBuf::from(args.get(i + 2).cloned().unwrap_or_default()),
                ));
                i += 3;
            }
            "--find" => {
                find_query = Some(next.clone());
                i += 2;
            }
            "--line" => {
                cursor_line = next.parse::<usize>().ok();
                i += 2;
            }
            "--tab" => {
                tab = Some(match next.as_str() {
                    "format" => inspector::Tab::Format,
                    "media" | "pictures" => inspector::Tab::Media,
                    "share" => inspector::Tab::Share,
                    _ => inspector::Tab::Design,
                });
                i += 2;
            }
            a if a.starts_with("-psn_") => i += 1,
            _ => {
                file = Some(PathBuf::from(&args[i]));
                i += 1;
            }
        }
    }
    #[cfg(target_os = "macos")]
    let icon_bytes: &[u8] = include_bytes!("../packaging/icons/macos-256.png");
    #[cfg(not(target_os = "macos"))]
    let icon_bytes: &[u8] = include_bytes!("../packaging/icons/256.png");
    let icon = eframe::icon_data::from_png_bytes(icon_bytes)
        .expect("Bundled PresentCraft icon must be a valid PNG");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1750.0, 1310.0])
            .with_min_inner_size([980.0, 560.0])
            .with_title("PresentCraft")
            .with_icon(icon)
            .with_app_id("app.myanmars.presentcraft")
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    macos_open::install();
    eframe::run_native(
        "PresentCraft",
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, file, screenshot);
            if blank {
                app.blank_doc();
            }
            if let Some(page) = settings_page {
                app.settings_open = true;
                app.settings_page = page;
            }
            if start_mcp {
                app.control_start(&cc.egui_ctx);
            }
            app.pending_present = present;
            app.pending_play = play_video;
            if light_ui || dark_ui {
                app.settings.match_system = false;
                app.settings.dark = dark_ui;
            }
            let start = start_slide.min(app.slides.len().saturating_sub(1));
            if start > 0 {
                app.select_slide(start);
            }
            if let Some(t) = theme_idx {
                app.apply_preset(t.min(app.presets.len() - 1));
            }
            if let Some(q) = find_query {
                app.find.query = q;
                app.find.open = true;
                app.find.focus_field = true;
            }
            if let Some(l) = cursor_line {
                // put the cursor at the start of line l (1-based) and show it
                let lines = tableblock::lines_of(&app.text);
                let ch = tableblock::char_of_line(&lines, l.saturating_sub(1));
                app.cursor = ch;
                app.sel_start = ch;
                app.jump_to = Some((ch, ch));
                app.scroll_editor = Some(ch);
                app.current = slide_at_char(&app.slides, ch);
            }
            if let Some((format, dest)) = export_req {
                app.export = Some(export::Job {
                    format,
                    dest,
                    idx: 0,
                    waiting: false,
                    frames: 0,
                    images: Vec::new(),
                    texts: Vec::new(),
                    pending_text: String::new(),
                    videos: Vec::new(),
                    pending_videos: Vec::new(),
                    text_layer: false,
                });
                app.quit_after_export = true;
            }
            if let Some(t) = tab {
                app.tab = t;
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(Clone, Copy, PartialEq)]
pub enum FontChoice {
    Theme,
    Sans,
    Serif,
    /// An installed system family.
    Named(fonts::Face),
}

pub struct App {
    pub settings: settings::Settings,
    pub settings_open: bool,
    pub settings_page: settings::Page,
    pub text: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub slides: Vec<Slide>,
    pub current: usize,
    pub presets: Vec<Preset>,
    pub preset_idx: usize,
    pub dark: bool,
    pub light_override: Option<Palette>,
    pub dark_override: Option<Palette>,
    pub deck_gradient: [bool; 2],
    pub gradient_colors: [Vec<Color32>; 2],
    pub title_font: FontChoice,
    pub body_font: FontChoice,
    pub size_idx: usize,
    /// Index into [`LINE_HEIGHTS`].
    pub line_idx: usize,
    /// Header and footer slots, and the one selected in the inspector.
    pub slots: [theme::Slot; 6],
    pub slot_sel: usize,
    /// The deck's one logo (shown by every slot set to Logo) and its height.
    pub logo: String,
    pub logo_size: f32,
    pub logo_drop_rect: Option<egui::Rect>,
    /// The Media panel's drop box, and its search text.
    pub media_drop_rect: Option<egui::Rect>,
    pub media_search: String,
    /// Media list and the state it was built from.
    pub media_cache: Option<(u64, Vec<inspector::MediaItem>)>,
    /// Bumped when files are added to or removed from the package.
    pub media_epoch: u64,
    /// Today's date (refreshed when it is shown in the inspector).
    pub today: String,
    pub author: String,
    pub audience: String,
    pub images: ImageCache,
    pub presenting: bool,
    pub prompter: bool,
    pub show_sidebar: bool,
    pub tab: inspector::Tab,
    pub table_cols: usize,
    pub table_rows: usize,
    pub table_values: bool,
    pub table_display: usize,
    /// The open "Edit in Table Editor…" dialog.
    pub table_editor: Option<tableeditor::TableEditor>,
    /// Cursor (char index) and selection start in the editor.
    pub cursor: usize,
    pub sel_start: usize,
    pub jump_to: Option<(usize, usize)>,
    /// Character to bring to the top of the editor (after clicking a slide).
    scroll_editor: Option<usize>,
    /// Find bar (⌘F) over the editor.
    find: find::Find,
    /// A running export (slides are drawn and captured one by one).
    pub export: Option<export::Job>,
    export_rect: Rect,
    quit_after_export: bool,
    pub status: String,
    status_seen: String,
    status_since: f64,
    /// Slide whose videos may be playing (videos stop when it changes).
    video_slide: usize,
    /// From the command line: start presenting / play the first video.
    pending_present: bool,
    pending_play: bool,
    /// Selected note in the presenter view.
    present_note: usize,
    /// Scroll the presenter view to the selected note.
    note_scroll: bool,
    /// Talk time counted before the timer was last paused.
    talk_elapsed: f64,
    /// When the running timer was started (`None` while paused).
    talk_since: Option<f64>,
    /// Bring the presentation window to the front on its next frame.
    present_focus: bool,
    title_sent: String,
    shot_path: Option<PathBuf>,
    frame: u32,
    shot_requested: bool,
    scroll_thumb: bool,
    recent_files: Vec<PathBuf>,
    /// Name and folder of an opened Markdown or iA Presenter document that
    /// hasn't been saved as a package yet.
    import_name: Option<String>,
    import_dir: Option<PathBuf>,
    /// The package's `info.json`, so keys this app doesn't know survive.
    doc_info: Option<json::Json>,
    /// Folder or zip, for a document opened from (or saved as) a package.
    doc_form: Option<package::Form>,
    /// A running check for a newer release, and a newer release to offer.
    pub update_check: Option<update::Check>,
    update_available: Option<update::Release>,
    /// When the next automatic update check may run (seconds since 1970).
    /// Zero, so the first one runs as soon as the app opens.
    next_auto_check: u64,
    /// The MCP control channel (File > MCP).
    pub control: control::Control,
}

pub const SIZES: [(&str, f32); 4] = [
    ("S — Small", 0.85),
    ("M — Default", 1.0),
    ("L — Large", 1.15),
    ("XL — Extra Large", 1.3),
];

/// Line spacing for slide text, relative to each theme's own spacing.
/// Scripts with tall stacked marks (Myanmar, Thai, Khmer) need more room.
pub const LINE_HEIGHTS: [(&str, f32); 8] = [
    ("80%", 0.8),
    ("90%", 0.9),
    ("100% — Default", 1.0),
    ("115%", 1.15),
    ("130%", 1.3),
    ("150%", 1.5),
    ("175%", 1.75),
    ("200%", 2.0),
];
pub const DEFAULT_LINE_IDX: usize = 2;

/// The [`LINE_HEIGHTS`] entry nearest a scale (1.0 is 100%).
pub fn line_idx_for(scale: f32) -> usize {
    LINE_HEIGHTS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 .1 - scale).abs().total_cmp(&(b.1 .1 - scale).abs()))
        .map(|(i, _)| i)
        .unwrap_or(DEFAULT_LINE_IDX)
}

impl App {
    fn new(cc: &eframe::CreationContext, file: Option<PathBuf>, shot: Option<PathBuf>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        fonts::start_scan();
        settings::set_visuals(&cc.egui_ctx, true);
        let mut presets = theme::presets();
        for (dir, mut p) in themefile::load_user_themes() {
            p.file = Some(dir);
            presets.push(p);
        }
        let default_theme = presets
            .iter()
            .position(|p| p.name == theme::DEFAULT_THEME)
            .unwrap_or(0);
        let mut app = App {
            settings: settings::Settings::load(SAMPLE),
            settings_open: false,
            settings_page: settings::Page::General,
            text: SAMPLE.to_string(),
            path: None,
            dirty: false,
            slides: Vec::new(),
            current: 0,
            dark: presets[default_theme].starts_dark,
            presets,
            preset_idx: default_theme,
            light_override: None,
            dark_override: None,
            deck_gradient: [false; 2],
            gradient_colors: std::array::from_fn(|_| vec![hex(0xF6D94C), hex(0x2463D4)]),
            title_font: FontChoice::Theme,
            body_font: FontChoice::Theme,
            size_idx: 1,
            line_idx: DEFAULT_LINE_IDX,
            slots: Default::default(),
            slot_sel: 5,
            logo: String::new(),
            logo_size: 0.05,
            logo_drop_rect: None,
            media_drop_rect: None,
            media_search: String::new(),
            media_cache: None,
            media_epoch: 0,
            today: theme::today(),
            author: String::new(),
            audience: String::new(),
            images: ImageCache::default(),
            presenting: false,
            prompter: false,
            show_sidebar: true,
            tab: inspector::Tab::Design,
            table_cols: 3,
            table_rows: 4,
            table_values: mechanics::TABLE_VALUES,
            table_display: 0,
            table_editor: None,
            cursor: 0,
            sel_start: 0,
            jump_to: None,
            scroll_editor: None,
            find: find::Find::default(),
            export: None,
            export_rect: Rect::NOTHING,
            quit_after_export: false,
            status: String::new(),
            status_seen: String::new(),
            status_since: 0.0,
            video_slide: usize::MAX,
            pending_present: false,
            pending_play: false,
            present_note: 0,
            note_scroll: false,
            talk_elapsed: 0.0,
            talk_since: None,
            present_focus: false,
            title_sent: String::new(),
            shot_path: shot,
            frame: 0,
            shot_requested: false,
            scroll_thumb: false,
            recent_files: Vec::new(),
            import_name: None,
            import_dir: None,
            doc_info: None,
            doc_form: None,
            update_check: None,
            update_available: None,
            next_auto_check: 0,
            control: Default::default(),
        };
        #[cfg(target_os = "macos")]
        macos_open::set_context(&cc.egui_ctx);
        app.apply_preset(default_theme);
        if let Some(f) = file {
            app.open_path(f);
        }
        app.reparse();
        app
    }

    /// Starts looking for a newer release. A manual check reports its result
    /// even when there is no update, and offers a version the person skipped.
    pub fn check_for_updates(&mut self, ctx: &egui::Context, manual: bool) {
        if let Some(check) = &mut self.update_check {
            check.manual |= manual;
            return;
        }
        if manual {
            self.status = "Checking for updates…".into();
        }
        self.update_check = Some(update::Check::start(ctx, manual));
    }

    /// Checks for a newer release when the app opens, then once a day while
    /// it stays open. Screenshot runs stay offline and show no banner.
    fn auto_check_updates(&mut self, ctx: &egui::Context) {
        if self.shot_path.is_some()
            || !self.settings.check_updates
            || self.update_check.is_some()
            || self.update_available.is_some()
        {
            return;
        }
        let now = update::now_secs();
        if now < self.next_auto_check {
            return;
        }
        self.next_auto_check = now + update::CHECK_INTERVAL_SECS;
        self.check_for_updates(ctx, false);
    }

    fn poll_update_check(&mut self) {
        let Some(outcome) = self.update_check.as_ref().and_then(|c| c.poll()) else {
            return;
        };
        let manual = self.update_check.take().is_some_and(|c| c.manual);
        if matches!(outcome, update::Outcome::Failed(_)) {
            // Offline or GitHub unreachable: try again in a while.
            if !manual {
                self.next_auto_check = update::now_secs() + update::RETRY_SECS;
            }
        } else {
            self.settings.last_update_check = update::now_secs();
            self.settings.save();
        }
        match outcome {
            update::Outcome::Newer(release) => {
                if manual || release.version != self.settings.skipped_version {
                    self.update_available = Some(release);
                }
                if manual {
                    self.status.clear();
                }
            }
            update::Outcome::UpToDate if manual => {
                self.status = format!("PresentCraft {} is up to date.", update::CURRENT);
            }
            update::Outcome::Failed(e) if manual => {
                self.status = format!("Couldn't check for updates: {e}");
            }
            _ => {}
        }
    }

    /// A small card under the top bar offering the newer release.
    fn update_banner(&mut self, ctx: &egui::Context) {
        let Some(release) = self.update_available.clone() else {
            return;
        };
        let pal = settings::pal();
        let mut download = false;
        let mut skip = false;
        let mut later = false;
        egui::Area::new(egui::Id::unique("update-banner"))
            .anchor(egui::Align2::CENTER_TOP, vec2(0.0, TOP_BAR + 10.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(pal.pill)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .corner_radius(10)
                    .shadow(ctx.global_style().visuals.popup_shadow)
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "PresentCraft {} is available",
                                        release.version
                                    ))
                                    .size(13.5)
                                    .strong()
                                    .color(pal.text_hi),
                                );
                                ui.label(
                                    RichText::new(format!("You have {}.", update::CURRENT))
                                        .size(12.0)
                                        .color(pal.text_dim),
                                );
                            });
                            ui.add_space(18.0);
                            download = ui.button("Download").clicked();
                            skip = ui.button("Skip This Version").clicked();
                            later = ui.button("Later").clicked();
                        });
                    });
            });
        if download {
            if !open_external(std::ffi::OsStr::new(&release.url)) {
                self.status = format!("Couldn't open {}", release.url);
            }
            self.update_available = None;
        } else if skip {
            self.settings.skipped_version = release.version;
            self.settings.save();
            self.update_available = None;
        } else if later {
            self.update_available = None;
        }
    }

    /// Apply the complete theme bundle, replacing manual design overrides.
    pub fn apply_preset(&mut self, i: usize) {
        self.preset_idx = i;
        if self.images.theme_dir != self.presets[i].file {
            self.images.theme_dir = self.presets[i].file.clone();
            self.images.clear_cache();
        }
        self.dark = self.presets[i].starts_dark;
        self.light_override = None;
        self.dark_override = None;
        self.deck_gradient = [false; 2];
        for (look, stops) in self.presets[i].gradients.iter().enumerate() {
            if stops.len() >= 2 {
                self.deck_gradient[look] = true;
                self.gradient_colors[look] = stops.clone();
            }
        }
        self.title_font = FontChoice::Theme;
        self.body_font = FontChoice::Theme;
        self.size_idx = self.presets[i].size_idx;
        self.line_idx = self.presets[i].line_idx;
    }

    pub fn theme(&self) -> Theme {
        self.theme_for(self.preset_idx, self.dark)
    }

    /// The theme for preset `i` (colour overrides apply to the current one only).
    pub fn theme_for(&self, i: usize, dark: bool) -> Theme {
        let p = &self.presets[i];
        let current = i == self.preset_idx;
        let light = self.light_override.filter(|_| current).unwrap_or(p.light);
        let dark_pal = self.dark_override.filter(|_| current).unwrap_or(p.dark);
        let (pal, inverse) = if dark {
            (dark_pal, light)
        } else {
            (light, dark_pal)
        };
        let pick = |c: FontChoice, default| match c {
            FontChoice::Theme => default,
            FontChoice::Sans => fonts::Face::SANS,
            FontChoice::Serif => fonts::Face::SERIF,
            FontChoice::Named(f) => f,
        };
        // Previews of other themes: a gradient theme shows its first colour
        // (the current theme gets its per-slide colours in paint_slide).
        let preview_bg = p.gradients[usize::from(dark)]
            .first()
            .copied()
            .filter(|_| !current);
        Theme {
            bg: preview_bg.unwrap_or(pal.bg),
            body: pal.body,
            title: pal.title,
            accents: {
                let mut a = if dark {
                    p.dark_accents.unwrap_or(p.accents)
                } else {
                    p.accents
                };
                if let Some(c) = p.accent1[usize::from(dark)] {
                    a[0] = c;
                }
                a
            },
            chart_colors: {
                let look = usize::from(dark);
                if p.chart_colors[look].is_empty() {
                    p.chart_colors[1 - look].clone()
                } else {
                    p.chart_colors[look].clone()
                }
            },
            face: if current {
                pick(self.body_font, p.body_face)
            } else {
                p.body_face
            },
            title_face: if current {
                pick(self.title_font, p.title_face)
            } else {
                p.title_face
            },
            invert_titles: p.invert_titles,
            inverse,
            size_scale: SIZES[if current { self.size_idx } else { p.size_idx }].1,
            line_scale: LINE_HEIGHTS[if current { self.line_idx } else { p.line_idx }].1,
            slots: self.slots.clone(),
            date: self.today.clone(),
            logo: self.logo.clone(),
            logo_size: self.logo_size,
            decor: p.decor,
        }
    }

    pub fn reparse(&mut self) {
        settings::apply_processing(&self.settings);
        self.slides = parse_document(&self.text);
        if self.current >= self.slides.len() {
            self.current = self.slides.len().saturating_sub(1);
        }
    }

    pub fn open_path(&mut self, p: PathBuf) {
        let p = package::package_for(&p);
        let doc = match package::open(&p) {
            Ok(doc) => doc,
            Err(e) => {
                self.status = format!("Couldn't open: {e}");
                return;
            }
        };
        self.text = doc.text;
        self.images.set_doc_dir(Some(doc.root.clone()));
        let mut imported = 0;
        if doc.kind != package::Kind::PrCraft && !package::is_work_dir(&doc.root) {
            // Copy the media it uses into a package of its own right away,
            // so the presentation never points at files outside it.
            if let Ok(work) = package::new_work_dir() {
                // An iA Presenter package's media folder comes along whole,
                // so the logo and unused files stay with the presentation.
                let src_assets = doc.root.join(package::ASSETS);
                if doc.kind == package::Kind::IaPresenter && src_assets.is_dir() {
                    if let Err(e) = themefile::copy_dir(&src_assets, &work.join(package::ASSETS)) {
                        self.status = format!("Couldn't copy media: {e}");
                    }
                }
                let images = &self.images;
                let resolve = |src: &str| images.resolve(src);
                match package::import_media(&self.text, &work, &resolve) {
                    Ok((text, n)) => {
                        self.text = text;
                        imported = n;
                        self.images.set_doc_dir(Some(work));
                    }
                    Err(e) => self.status = format!("Couldn't copy media: {e}"),
                }
            }
        }
        self.media_epoch += 1;
        self.status = match imported {
            0 => format!("Opened {}", p.file_name().unwrap_or_default().to_string_lossy()),
            n => format!(
                "Opened {} and copied {n} media file{} into the presentation",
                p.file_name().unwrap_or_default().to_string_lossy(),
                if n == 1 { "" } else { "s" }
            ),
        };
        self.recent_files.retain(|recent| recent != &p);
        self.recent_files.insert(0, p.clone());
        self.recent_files.truncate(10);
        if doc.kind == package::Kind::PrCraft {
            self.path = Some(p);
            self.import_name = None;
            self.import_dir = None;
        } else {
            // Saving makes a new package next to the original.
            self.import_name = p.file_stem().map(|n| n.to_string_lossy().into_owned());
            self.import_dir = p.parent().map(|d| d.to_path_buf());
            self.path = None;
        }
        let settings = doc.info.as_ref().map(docinfo::read).unwrap_or_default();
        self.pick_doc_theme(doc.theme, &settings);
        self.apply_doc_settings(&settings);
        // Kept so keys this app doesn't use (iA Presenter's among them)
        // survive saving.
        self.doc_info = doc.info.filter(|_| doc.kind != package::Kind::Text);
        self.doc_form = (doc.kind == package::Kind::PrCraft).then_some(doc.form);
        self.dirty = false;
        self.current = 0;
        self.cursor = 0;
        self.sel_start = 0;
        self.reparse();
    }

    /// Opens a document macOS or another window asked for: here when this
    /// window holds an untouched new document, otherwise in a new window.
    pub fn open_requested(&mut self, p: PathBuf) {
        let p = package::package_for(&p);
        if self.path.as_ref() == Some(&p) {
            return;
        }
        if self.path.is_none() && self.import_name.is_none() && !self.dirty {
            self.open_path(p);
            return;
        }
        let spawned = std::env::current_exe().ok().is_some_and(|exe| {
            std::process::Command::new(exe)
                .arg(&p)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
        });
        if !spawned && self.confirm_replace() {
            self.open_path(p);
        }
    }

    /// Selects the document's theme, adding it when no theme matches.
    fn use_doc_theme(&mut self, t: Preset) {
        let idx = match self
            .presets
            .iter()
            .position(|p| p.name == t.name && themefile::same_look(p, &t))
        {
            Some(i) => i,
            None => {
                self.presets.push(t.clone());
                self.presets.len() - 1
            }
        };
        self.apply_preset(idx);
        self.dark = t.starts_dark;
    }

    /// The current design (theme plus colour, font and size changes).
    pub fn current_as_preset(&self) -> Preset {
        let resolved = self.theme();
        let mut p = self.presets[self.preset_idx].clone();
        p.title_face = resolved.title_face;
        p.body_face = resolved.face;
        p.size_idx = self.size_idx;
        p.line_idx = self.line_idx;
        p.light = self.light_override.unwrap_or(p.light);
        p.dark = self.dark_override.unwrap_or(p.dark);
        p.starts_dark = self.dark;
        p.gradients = std::array::from_fn(|look| {
            if self.deck_gradient[look] {
                self.gradient_colors[look].clone()
            } else {
                Vec::new()
            }
        });
        p
    }

    /// Chooses the document's theme: its own `Theme/` folder, else a theme
    /// named in `info.json`, else the iA Presenter template it names.
    fn pick_doc_theme(&mut self, own: Option<Preset>, d: &docinfo::DocSettings) {
        if let Some(theme) = own {
            self.use_doc_theme(theme);
            return;
        }
        let by_name = |presets: &[Preset], name: &str| {
            presets
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(name.trim()))
        };
        if let Some(i) = d.theme.as_deref().and_then(|n| by_name(&self.presets, n)) {
            self.apply_preset(i);
            return;
        }
        let Some(template) = d.ia_template.as_deref() else {
            return;
        };
        let ia = themefile::ia_theme_dir(template)
            .and_then(|dir| themefile::load_variant(&dir, d.ia_preset.as_deref()).ok());
        match ia {
            Some(t) => self.use_doc_theme(t),
            None => {
                // iA Presenter isn't installed: use an imported copy.
                if let Some(i) = by_name(&self.presets, template) {
                    self.apply_preset(i);
                } else {
                    self.status = format!(
                        "This presentation uses the iA Presenter theme \"{template}\". Import it in Settings → Themes to use it here."
                    );
                }
            }
        }
    }

    /// Document settings written to `info.json`.
    fn doc_info_json(&self) -> json::Json {
        let d = docinfo::DocSettings {
            theme: Some(self.presets[self.preset_idx].name.to_string()),
            ia_template: None,
            ia_preset: None,
            dark: Some(self.dark),
            author: Some(self.author.clone()),
            audience: Some(self.audience.clone()),
            logo: Some(self.logo.clone()),
            logo_size: Some(self.logo_size),
            slots: Some(self.slots.clone()),
            gradients: [0, 1].map(|i| Some((self.deck_gradient[i], self.gradient_colors[i].clone()))),
        };
        docinfo::write(self.doc_info.as_ref(), &d)
    }

    /// Applies settings from `info.json`; unset ones keep current values.
    fn apply_doc_settings(&mut self, d: &docinfo::DocSettings) {
        if let Some(dark) = d.dark {
            self.dark = dark;
        }
        if let Some(a) = &d.author {
            self.author = a.clone();
        }
        if let Some(a) = &d.audience {
            self.audience = a.clone();
        }
        if let Some(l) = &d.logo {
            self.logo = l.clone();
        }
        if let Some(n) = d.logo_size {
            self.logo_size = n;
        }
        if let Some(slots) = &d.slots {
            self.slots = slots.clone();
        }
        for (i, g) in d.gradients.iter().enumerate() {
            if let Some((on, colors)) = g {
                self.deck_gradient[i] = *on;
                if colors.len() >= 2 {
                    self.gradient_colors[i] = colors.clone();
                }
            }
        }
    }

    pub fn open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter(
                "Presentations",
                &[
                    package::EXT,
                    package::IA_EXT,
                    "md",
                    "markdown",
                    "txt",
                    "text",
                ],
            )
            .add_filter("PresentCraft", &[package::EXT])
            .add_filter("iA Presenter", &[package::IA_EXT])
            .add_filter("Markdown", &["md", "markdown", "txt", "text"])
            .pick_file()
        {
            if self.confirm_replace() {
                self.open_path(p);
            }
        }
    }

    /// Saves as a `.prcraft` package; asks where when there's no package yet.
    pub fn save(&mut self, save_as: bool) {
        let target = if save_as || self.path.is_none() {
            let mut d = rfd::FileDialog::new()
                .add_filter("PresentCraft Presentation", &[package::EXT])
                .set_file_name(format!("{}.{}", self.doc_name(), package::EXT));
            let dir = self
                .import_dir
                .clone()
                .or_else(|| self.path.as_ref().and_then(|p| p.parent()).map(|d| d.to_path_buf()));
            if let Some(dir) = dir {
                d = d.set_directory(dir);
            }
            d.save_file().map(package::with_ext)
        } else {
            self.path.clone()
        };
        if let Some(p) = target {
            self.save_to(p);
        }
    }

    pub fn save_to(&mut self, p: PathBuf) -> bool {
        let theme = self.current_as_preset();
        let info = self.doc_info_json();
        let images = &self.images;
        let resolve = |src: &str| images.resolve(src);
        let form = self.doc_form.unwrap_or_else(package::default_form);
        let result = package::save(
            &p,
            form,
            package::SaveRequest {
                text: &self.text,
                info: info.clone(),
                theme: &theme,
                dark: self.dark,
                logo: &self.logo,
                resolve: &resolve,
            },
        );
        let saved = match result {
            Ok(saved) => saved,
            Err(e) => {
                self.status = format!("Couldn't save: {e}");
                return false;
            }
        };
        if saved.text != self.text {
            self.text = saved.text;
            let n = self.text.chars().count();
            self.cursor = self.cursor.min(n);
            self.sel_start = self.sel_start.min(n);
            self.reparse();
        }
        if !saved.logo.is_empty() {
            self.logo = saved.logo;
        }
        self.images.set_doc_dir(Some(saved.root.clone()));
        self.doc_form = Some(saved.form);
        self.status = match saved.copied {
            0 => format!("Saved {}", p.file_name().unwrap_or_default().to_string_lossy()),
            n => format!(
                "Saved {} and copied {n} media file{} into it",
                p.file_name().unwrap_or_default().to_string_lossy(),
                if n == 1 { "" } else { "s" }
            ),
        };
        self.recent_files.retain(|recent| recent != &p);
        self.recent_files.insert(0, p.clone());
        self.recent_files.truncate(10);
        self.path = Some(p);
        self.import_name = None;
        self.import_dir = None;
        self.doc_info = Some(info);
        self.dirty = false;
        true
    }

    /// How a dropped or chosen media file is written in the script: `./name`
    /// when it sits in the saved document's folder (or below), otherwise the
    /// full path. Spaces are written as %20 so the line stays one word.
    /// Copies the file into the presentation's `assets/` folder and returns
    /// its `/assets/…` reference. Unsaved documents use a private working
    /// folder that Save copies into the new package.
    pub fn media_ref(&mut self, file: &std::path::Path) -> String {
        if let Some(root) = self.media_root() {
            match package::add_asset(&root, file) {
                Ok(r) => {
                    self.media_epoch += 1;
                    if self.doc_form == Some(package::Form::Zip) {
                        // The zip is rewritten on the next save.
                        self.dirty = true;
                    }
                    return r;
                }
                Err(e) => self.status = format!("Couldn't add the file: {e}"),
            }
        }
        media_ref_for(self.images.doc_dir.as_deref(), file)
    }

    /// The folder holding this presentation's `assets/`.
    pub fn media_root(&mut self) -> Option<PathBuf> {
        if let Some(d) = self.images.doc_dir.clone() {
            if self.path.is_some() || package::is_work_dir(&d) {
                return Some(d);
            }
        }
        let work = package::new_work_dir().ok()?;
        self.images.set_doc_dir(Some(work.clone()));
        Some(work)
    }

    /// Opens a slide's video in the system's video player.
    pub fn play_video(&mut self, src: &str) {
        let target: Option<std::ffi::OsString> = if images::is_web(src) {
            Some(src.into())
        } else {
            self.images.resolve(src).map(|p| p.into_os_string())
        };
        let Some(target) = target else {
            self.status = format!("Couldn't find the video {src}");
            return;
        };
        if !open_external(&target) {
            self.status = "Couldn't open the video player".into();
        }
    }

    /// Opens a new window with a blank document. Each window is its own
    /// process, so the current document stays untouched.
    pub fn new_doc(&mut self) {
        let spawned = std::env::current_exe().ok().is_some_and(|exe| {
            std::process::Command::new(exe)
                .arg("--new")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
        });
        if spawned {
            return;
        }
        // Couldn't start a new window: fall back to replacing this document.
        self.status = "Couldn't open a new window".into();
        if self.confirm_replace() {
            self.blank_doc();
        }
    }

    /// Replaces the current document with the new-presentation template.
    fn blank_doc(&mut self) {
        self.text = self.settings.new_template.clone();
        self.path = None;
        self.import_name = None;
        self.import_dir = None;
        self.doc_info = None;
        self.doc_form = None;
        self.dirty = false;
        self.current = 0;
        self.cursor = 0;
        self.sel_start = 0;
        self.images.set_doc_dir(None);
        self.reparse();
    }

    fn confirm_replace(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        match rfd::MessageDialog::new()
            .set_title("Save changes?")
            .set_description("Save your changes before continuing? Choose No to discard them.")
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show()
        {
            rfd::MessageDialogResult::Yes => {
                self.save(false);
                !self.dirty
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    fn insert_slide_template(&mut self, content: &str) {
        let ch = self
            .slides
            .get(self.current + 1)
            .map(|s| s.start_char)
            .unwrap_or_else(|| self.text.chars().count());
        let byte = self.char_to_byte(ch);
        let addition = if byte == self.text.len() {
            format!("\n\n---\n\n{content}\n")
        } else {
            format!("{content}\n\n---\n\n")
        };
        self.text.insert_str(byte, &addition);
        let cursor = ch + addition.chars().count();
        self.after_edit(cursor, cursor);
    }

    fn menus(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let menu_btn = egui::Button::new(egui::RichText::new("   ").size(16.0))
            .frame(false)
            .min_size(vec2(30.0, 26.0));
        let inner = crate::tabletools::menu_from_button(ui, menu_btn, |ui| {
            crate::tabletools::style_menu(ui);
            ui.menu_button("File", |ui| {
                crate::tabletools::style_menu(ui);
                if menu_item(ui, "New", "⌘N") {
                    ui.close();
                    self.new_doc();
                }
                if menu_item(ui, "Open…", "⌘O") {
                    ui.close();
                    self.open_dialog();
                }
                // Outside macOS a package is a plain folder, which the file
                // picker can't choose as a document.
                if !cfg!(target_os = "macos") && ui.button("Open Package Folder…").clicked() {
                    ui.close();
                    if let Some(p) = rfd::FileDialog::new()
                        .set_title("Choose a .prcraft or .iapresenter folder")
                        .pick_folder()
                    {
                        if self.confirm_replace() {
                            self.open_path(p);
                        }
                    }
                }
                ui.menu_button("Open Recent", |ui| {
                    crate::tabletools::style_menu(ui);
                    if self.recent_files.is_empty() {
                        menu_note(ui, "No recent documents");
                    }
                    for path in self.recent_files.clone() {
                        if ui.button(path.display().to_string()).clicked() {
                            ui.close();
                            if self.confirm_replace() {
                                self.open_path(path);
                            }
                        }
                    }
                });
                ui.separator();
                if menu_item(ui, "Save", "⌘S") {
                    ui.close();
                    self.save(false);
                }
                if menu_item(ui, "Save As…", "⇧⌘S") {
                    ui.close();
                    self.save(true);
                }
                if ui.button("Duplicate…").clicked() {
                    ui.close();
                    let path = self.path.clone();
                    let dirty = self.dirty;
                    self.save(true);
                    if self.path == path {
                        self.dirty = dirty;
                    }
                }
                if ui
                    .add_enabled(self.path.is_some(), egui::Button::new("Revert to Saved…"))
                    .clicked()
                {
                    ui.close();
                    if self.confirm_replace() {
                        if let Some(path) = self.path.clone() {
                            self.open_path(path);
                        }
                    }
                }
                ui.separator();
                ui.menu_button("Import", |ui| {
                    crate::tabletools::style_menu(ui);
                    if ui.button("Markdown or Text…").clicked() {
                        ui.close();
                        self.open_dialog();
                    }
                });
                ui.menu_button("Export", |ui| {
                    crate::tabletools::style_menu(ui);
                    for (name, format) in [
                        ("PDF…", export::Format::Pdf),
                        ("PowerPoint…", export::Format::PowerPoint),
                        ("Images…", export::Format::Images),
                        ("HTML…", export::Format::Html),
                    ] {
                        if ui.button(name).clicked() {
                            ui.close();
                            self.start_export(format);
                        }
                    }
                    if ui.button("Markdown…").clicked() {
                        ui.close();
                        self.export_markdown();
                    }
                });
                ui.separator();
                ui.menu_button("MCP", |ui| {
                    crate::tabletools::style_menu(ui);
                    let port = self.control.running.as_ref().map(|r| r.port);
                    let status = match port {
                        Some(p) => format!("Server running on port {p}"),
                        None => "Server stopped".to_string(),
                    };
                    menu_note(ui, &status);
                    if ui
                        .add_enabled(port.is_none(), egui::Button::new("Start Server"))
                        .clicked()
                    {
                        ui.close();
                        self.control_start(ctx);
                    }
                    if ui
                        .add_enabled(port.is_some(), egui::Button::new("Stop Server"))
                        .clicked()
                    {
                        ui.close();
                        self.control_stop();
                    }
                });
                ui.separator();
                if menu_item(ui, if IS_MAC { "Close Window" } else { "Exit" }, "⌘W") {
                    ui.close();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Insert", |ui| {
                crate::tabletools::style_menu(ui);
                egui::ScrollArea::vertical()
                    .max_height(650.0)
                    .show(ui, |ui| {
                        if menu_item(ui, "Blank Slide", "⇧⌘N") {
                            ui.close();
                            self.insert_slide_template("");
                        }
                        for (group, templates) in SLIDE_TEMPLATES {
                            ui.separator();
                            menu_note(ui, group);
                            for (name, content) in *templates {
                                if ui.button(*name).clicked() {
                                    ui.close();
                                    self.insert_slide_template(content);
                                }
                            }
                        }
                    });
            });
            ui.menu_button("View", |ui| {
                crate::tabletools::style_menu(ui);
                if menu_item(
                    ui,
                    if self.show_sidebar {
                        "Hide Slide List"
                    } else {
                        "Show Slide List"
                    },
                    "",
                ) {
                    ui.close();
                    self.show_sidebar = !self.show_sidebar;
                }
                if menu_item(ui, "Settings…", "⌘,") {
                    self.settings_open = true;
                    ui.close();
                }
                if ui.button("Check for Updates…").clicked() {
                    ui.close();
                    self.check_for_updates(ctx, true);
                }
            });
            ui.separator();
            if menu_item(ui, "Start Presentation", "⌘↵") {
                ui.close();
                self.start_presenting(ctx);
            }
        });
        let r = inner.rect;
        let resp = inner.on_hover_text("Menu");
        if resp.hovered() {
            ui.painter()
                .rect_filled(r, 6.0, crate::settings::pal().hover);
        }
        paint_icon(ui.painter(), r, Icon::Menu, crate::settings::pal().icon);
    }

    fn char_to_byte(&self, ch: usize) -> usize {
        self.text
            .char_indices()
            .nth(ch)
            .map(|(b, _)| b)
            .unwrap_or(self.text.len())
    }

    /// Replaces the selection (or inserts at the cursor) and moves the cursor after it.
    pub fn insert_text(&mut self, s: &str) {
        let (a, b) = (
            self.sel_start.min(self.cursor),
            self.sel_start.max(self.cursor),
        );
        let (ba, bb) = (self.char_to_byte(a), self.char_to_byte(b));
        self.text.replace_range(ba..bb, s);
        let end = a + s.chars().count();
        self.after_edit(end, end);
    }

    /// Wraps the selection in `left` … `right` (e.g. ** for bold).
    pub fn wrap_selection(&mut self, left: &str, right: &str, placeholder: &str) {
        let (a, b) = (
            self.sel_start.min(self.cursor),
            self.sel_start.max(self.cursor),
        );
        let (ba, bb) = (self.char_to_byte(a), self.char_to_byte(b));
        let inner = if a == b {
            placeholder.to_string()
        } else {
            self.text[ba..bb].to_string()
        };
        let s = format!("{left}{inner}{right}");
        self.text.replace_range(ba..bb, &s);
        let start = a + left.chars().count();
        self.after_edit(start, start + inner.chars().count());
    }

    /// Puts `prefix` at the start of the cursor's line, replacing an existing
    /// heading/list/tab prefix.
    pub fn set_line_prefix(&mut self, prefix: &str) {
        let byte = self.char_to_byte(self.cursor);
        let line_start = self.text[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = self.text[byte..]
            .find('\n')
            .map(|i| byte + i)
            .unwrap_or(self.text.len());
        let line = &self.text[line_start..line_end];
        let mut rest = line.trim_start_matches('\t');
        rest = rest.trim_start_matches('#').trim_start();
        for m in ["- [ ] ", "- [x] ", "- ", "* ", "+ ", "> "] {
            if let Some(r) = rest.strip_prefix(m) {
                rest = r;
                break;
            }
        }
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits > 0 && rest[digits..].starts_with(". ") {
            rest = &rest[digits + 2..];
        }
        let new_line = format!("{prefix}{rest}");
        let line_start_ch = self.text[..line_start].chars().count();
        let new_len = new_line.chars().count();
        self.text.replace_range(line_start..line_end, &new_line);
        self.after_edit(line_start_ch + new_len, line_start_ch + new_len);
    }

    /// Inserts a block on its own lines after the cursor's line.
    pub fn insert_block(&mut self, block: &str) {
        let byte = self.char_to_byte(self.cursor);
        let line_end = self.text[byte..]
            .find('\n')
            .map(|i| byte + i)
            .unwrap_or(self.text.len());
        let s = format!("\n{block}\n");
        self.text.insert_str(line_end, &s);
        let ch = self.text[..line_end].chars().count() + s.chars().count() - 1;
        self.after_edit(ch, ch);
    }

    fn after_edit(&mut self, sel_start: usize, cursor: usize) {
        self.dirty = true;
        self.sel_start = sel_start;
        self.cursor = cursor;
        self.jump_to = Some((sel_start, cursor));
        self.reparse();
        self.current = slide_at_char(&self.slides, cursor);
    }

    fn go(&mut self, delta: i32) {
        let n = self.slides.len() as i32;
        self.current = (self.current as i32 + delta).clamp(0, (n - 1).max(0)) as usize;
    }

    pub fn select_slide(&mut self, i: usize) {
        self.current = i;
        // The cursor goes to the slide's first line and
        // the editor scrolls there.
        let start = self.slides.get(i).map(|s| s.start_char).unwrap_or(0);
        let ch = first_content_char(&self.text, start);
        self.cursor = ch;
        self.sel_start = ch;
        self.jump_to = Some((ch, ch));
        self.scroll_editor = Some(ch);
        self.scroll_thumb = true;
    }

    /// The Markdown text of slide `i`.
    fn slide_text(&self, i: usize) -> String {
        let start = self.slides.get(i).map(|s| s.start_char).unwrap_or(0);
        let end = self
            .slides
            .get(i + 1)
            .map(|s| s.start_char)
            .unwrap_or(usize::MAX);
        self.text
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect()
    }

    /// Estimated speaking time in seconds: spoken and on-slide words at SPEECH_WPM.
    fn speaking_seconds(&self) -> f32 {
        speaking_seconds(&self.text) * SPEECH_WPM / self.settings.wpm as f32
    }
}

/// Estimated speaking time in seconds of a piece of Markdown.
fn speaking_seconds(text: &str) -> f32 {
    {
        let mut words = 0usize;
        let mut in_fence = false;
        for l in text.lines() {
            let t = l.trim();
            if t.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence
                || t.is_empty()
                || parse::is_separator(t)
                || t.starts_with('|')
                || t.starts_with('#')
                || t.starts_with("//")
            {
                continue;
            }
            if !t.contains(' ') && (t.starts_with('/') || t.starts_with("http") || t.contains('.'))
            {
                continue;
            }
            if let Some((k, _)) = t.split_once(':') {
                let kl = k.trim().to_ascii_lowercase();
                if [
                    "size",
                    "x",
                    "y",
                    "background",
                    "opacity",
                    "filter",
                    "title",
                    "caption",
                    "chart",
                    "xlabel",
                    "ylabel",
                    "colors",
                    "xformat",
                    "legend",
                    "color-type",
                    "bar-type",
                    "orientation",
                    "type",
                ]
                .contains(&kl.as_str())
                {
                    continue;
                }
            }
            words += t
                .split_whitespace()
                .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
                .count();
        }
        words as f32 / SPEECH_WPM * 60.0
    }
}

impl App {
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| {
                    let p = f.path();
                    (!p.as_os_str().is_empty()).then(|| p.to_path_buf())
                })
                .collect()
        });
        for p in dropped {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if self.tab == inspector::Tab::Design
                && !self.presenting
                && !self.settings_open
                && self.logo_drop_rect.is_some_and(|r| {
                    ctx.input(|i| i.pointer.latest_pos().is_some_and(|p| r.contains(p)))
                })
                && matches!(
                    ext.as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
                )
            {
                self.logo = self.media_ref(&p);
                continue;
            }
            if self.tab == inspector::Tab::Media
                && !self.presenting
                && !self.settings_open
                && package::is_media_file(&p)
                && self.media_drop_rect.is_some_and(|r| {
                    ctx.input(|i| i.pointer.latest_pos().is_some_and(|p| r.contains(p)))
                })
            {
                // Dropped on the Media panel: add to the presentation only.
                let _ = self.media_ref(&p);
                self.status = format!(
                    "Added {}. Double-click it to put it on the current slide.",
                    p.file_name().unwrap_or_default().to_string_lossy()
                );
                continue;
            }
            if matches!(
                ext.as_str(),
                "md" | "markdown" | "txt" | "text" | package::EXT | package::IA_EXT
            ) {
                if self.confirm_replace() {
                    self.open_path(p);
                }
            } else if ext == "csv" {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    self.insert_block(&inspector::csv_to_table(&t));
                }
            } else {
                let r = self.media_ref(&p);
                self.insert_block(&r);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let t = format!(
            "{}{} — PresentCraft",
            self.doc_name(),
            if self.dirty { " •" } else { "" }
        );
        if t != self.title_sent {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title_sent = t;
        }
    }

    pub fn doc_name_pub(&self) -> String {
        self.doc_name()
    }

    fn doc_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|n| n.to_string_lossy().to_string())
            .or_else(|| self.import_name.clone())
            .unwrap_or_else(|| "Untitled".into())
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot_path.clone() else {
            return;
        };
        self.frame += 1;
        ctx.request_repaint();
        let mut saved = false;
        {
            if let Some(image) = take_shot() {
                {
                    let [w, h] = image.size;
                    let mut buf = Vec::with_capacity(w * h * 4);
                    for p in &image.pixels {
                        buf.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
                    }
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                        if let Err(e) = img.save(&path) {
                            eprintln!("screenshot save failed: {e}");
                        }
                    }
                    saved = true;
                }
            }
        }
        if saved || self.frame > SCREENSHOT_FRAMES + 240 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frame >= SCREENSHOT_FRAMES && !self.shot_requested {
            self.shot_requested = true;
            request_shot(ctx);
        }
    }

    pub fn start_presenting(&mut self, ctx: &egui::Context) {
        if self.settings.warn_empty
            && self
                .slides
                .iter()
                .any(|s| s.groups.is_empty() && s.backgrounds.is_empty())
        {
            self.status = mechanics::EMPTY_SLIDE_GUIDANCE.into();
        }
        self.presenting = true;
        self.present_focus = true;
        self.present_note = 0;
        self.note_scroll = true;
        self.talk_elapsed = 0.0;
        self.talk_since = Some(ctx.input(|i| i.time));
        // Inline presenting (screenshot runs, or a backend without separate
        // windows) still takes over the main window, so go fullscreen there.
        if self.shot_path.is_none() && ctx.embed_viewports() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
    }

    fn stop_presenting(&mut self, ctx: &egui::Context) {
        self.images.stop_videos();
        if self.presenting && self.shot_path.is_none() && ctx.embed_viewports() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        self.presenting = false;
        let i = self.current;
        self.select_slide(i);
    }

    const PRESENT_TITLE: &str = "Presentation";

    /// Shows the presentation in its own window, which can be moved,
    /// resized or made full screen, so the editor stays open behind it.
    /// Closing that window stops presenting.
    fn present_window(&mut self, ctx: &egui::Context) {
        let id = egui::ViewportId::from_hash_of("presentation-window");
        let builder = egui::ViewportBuilder::default()
            .with_title(Self::PRESENT_TITLE)
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([320.0, 180.0])
            .with_resizable(true);
        ctx.show_viewport_immediate(id, builder, |pui, _class| {
            let pctx = &pui.ctx().clone();
            if std::mem::take(&mut self.present_focus) {
                pctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if pctx.input(|i| i.viewport().close_requested()) {
                self.stop_presenting(pctx);
                return;
            }
            if pctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
                self.stop_presenting(pctx);
                return;
            }
            self.present_ui(pui);
        });
    }

    pub fn paint_slide(&mut self, painter: &egui::Painter, rect: Rect, idx: usize) {
        let mut theme = self.theme();
        let look = usize::from(self.dark);
        if self.deck_gradient[look] {
            if let Some(bg) = theme::deck_color(&self.gradient_colors[look], idx, self.slides.len())
            {
                theme.bg = bg;
                // A theme's own gradient comes with its own text colours
                // (San Francisco: white on every colour). For a gradient
                // picked here, choose black or white for contrast.
                let from_theme = self.presets[self.preset_idx].gradients[look]
                    == self.gradient_colors[look];
                if !from_theme {
                    theme.body = theme::contrast_text(bg);
                    theme.title = theme.body;
                }
                theme.invert_titles = false;
                theme.decor = theme::Decor::None;
            }
        }
        self.paint_slide_with(painter, rect, idx, &theme);
    }

    /// Draws slide `idx` with a given theme (used by the theme picker).
    pub fn paint_slide_with(
        &mut self,
        painter: &egui::Painter,
        rect: Rect,
        idx: usize,
        theme: &Theme,
    ) {
        let theme = theme.clone();
        if let Some(slide) = self.slides.get(idx) {
            let mut cx = Ctx {
                painter,
                theme: &theme,
                images: &mut self.images,
            };
            draw_slide(&mut cx, rect, slide, idx + 1);
        }
    }

    // ---------- exporting ----------
    /// Asks where to save, then draws and captures every slide.
    pub fn start_export(&mut self, format: export::Format) {
        let name = self.doc_name();
        let dest = match format {
            export::Format::Pdf => rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name(format!("{name}.pdf"))
                .save_file(),
            export::Format::PowerPoint => rfd::FileDialog::new()
                .add_filter("PowerPoint", &["pptx"])
                .set_file_name(format!("{name}.pptx"))
                .save_file(),
            export::Format::Html => rfd::FileDialog::new()
                .add_filter("Web page", &["html"])
                .set_file_name(format!("{name}.html"))
                .save_file(),
            export::Format::Images => rfd::FileDialog::new()
                .set_title("Choose a folder for the slide pictures")
                .pick_folder(),
        };
        if let Some(dest) = dest {
            self.export = Some(export::Job {
                format,
                dest,
                idx: 0,
                waiting: false,
                frames: 0,
                images: Vec::new(),
                texts: Vec::new(),
                pending_text: String::new(),
                videos: Vec::new(),
                pending_videos: Vec::new(),
                text_layer: false,
            });
        }
    }

    pub fn export_markdown(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "txt"])
            .set_file_name(format!("{}.md", self.doc_name()))
            .save_file()
        {
            self.status = match std::fs::write(&p, &self.text) {
                Ok(_) => format!(
                    "Exported {}",
                    p.file_name().unwrap_or_default().to_string_lossy()
                ),
                Err(e) => format!("Couldn't export: {e}"),
            };
        }
    }

    fn export_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let n = self.slides.len();
        let mut cancel = ctx.input(|i| i.key_pressed(Key::Escape));
        // a captured picture arrived?
        let shot = take_shot();
        if let (Some(img), Some(job)) = (shot, self.export.as_mut()) {
            let ppp = ctx.pixels_per_point();
            let r = self.export_rect;
            let [w, h] = img.size;
            let x0 = ((r.left() * ppp).round().max(0.0) as usize).min(w);
            let y0 = ((r.top() * ppp).round().max(0.0) as usize).min(h);
            let x1 = ((r.right() * ppp).round() as usize).min(w);
            let y1 = ((r.bottom() * ppp).round() as usize).min(h);
            let mut out = image::RgbaImage::new((x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = img.pixels[y * w + x];
                    out.put_pixel(
                        (x - x0) as u32,
                        (y - y0) as u32,
                        image::Rgba([p.r(), p.g(), p.b(), 255]),
                    );
                }
            }
            job.images.push(out);
            job.texts.push(std::mem::take(&mut job.pending_text));
            job.videos.push(std::mem::take(&mut job.pending_videos));
            job.idx += 1;
            job.text_layer = false;
            job.waiting = false;
            job.frames = 0;
        }
        let downloading = self.images.pending();
        let Some(job) = self.export.as_mut() else {
            return;
        };
        if job.idx >= n {
            let job = self.export.take().unwrap();
            let notes: Vec<String> = self.slides.iter().map(|s| s.notes.join("\n\n")).collect();
            let res: Result<(), String> = match job.format {
                export::Format::Pdf => {
                    export::write_pdf(&job.dest, &job.images).map_err(|e| e.to_string())
                }
                export::Format::PowerPoint => {
                    export::write_pptx(&job.dest, &job.images, &job.texts, &notes, &job.videos)
                }
                export::Format::Images => export::write_images(&job.dest, &job.images),
                export::Format::Html => export::write_html(
                    &job.dest,
                    &job.images,
                    &notes,
                    &self.doc_name(),
                    &job.videos,
                    &job.texts,
                )
                .map_err(|e| e.to_string()),
            };
            self.status = match res {
                Ok(_) => format!("Exported {} slides as {}", n, job.format.name()),
                Err(e) => format!("Export failed: {e}"),
            };
            if self.quit_after_export {
                eprintln!("{}", self.status);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        let idx = job.idx;
        job.frames += 1;
        // Wait for web pictures to arrive (up to about 20 seconds).
        let request = !job.waiting && job.frames >= 3 && (downloading == 0 || job.frames > 1200);
        if request {
            job.waiting = true;
        }
        let fmt_name = job.format.name();
        // PowerPoint gets real text boxes, so its slide pictures are drawn without text.
        // PowerPoint gets real text everywhere; HTML on slides with a
        // background video, so the text stays above the playing video.
        let is_html = job.format == export::Format::Html;
        let editable_text = job.format == export::Format::PowerPoint || (is_html && job.text_layer);
        let with_videos = matches!(
            job.format,
            export::Format::PowerPoint | export::Format::Html
        );
        let mut recorded = String::new();
        let mut recorded_videos = Vec::new();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK))
            .show(ui, |ui| {
                let avail = ui.max_rect();
                let rect = fit_16_9(avail.shrink2(vec2(0.0, 40.0)));
                self.export_rect = rect;
                let painter = ui.painter_at(avail);
                if editable_text {
                    render::begin_text_capture();
                }
                if with_videos {
                    images::begin_video_spots();
                }
                self.paint_slide(&painter, rect, idx);
                if with_videos {
                    let images = &self.images;
                    recorded_videos = images::take_video_spots()
                        .iter()
                        .filter_map(|v| {
                            export::ExportVideo::from_spot(v, rect, |s| images.resolve(s))
                        })
                        .collect();
                }
                if editable_text {
                    let boxes = render::end_text_capture();
                    recorded = if is_html {
                        export::html_text_layer(&boxes, rect)
                    } else {
                        export::pptx_text_shapes(&boxes, rect)
                    };
                }
                painter.text(
                    pos2(avail.center().x, avail.bottom() - 18.0),
                    egui::Align2::CENTER_CENTER,
                    if downloading > 0 {
                        format!(
                            "Downloading {downloading} picture{}…   ·   Esc to cancel",
                            if downloading == 1 { "" } else { "s" }
                        )
                    } else {
                        format!(
                            "Exporting {} — slide {} of {}   ·   Esc to cancel",
                            fmt_name,
                            idx + 1,
                            n
                        )
                    },
                    FontId::proportional(13.0),
                    Color32::from_gray(150),
                );
            });
        if let Some(job) = self.export.as_mut() {
            if editable_text {
                job.pending_text = recorded;
            }
            if with_videos {
                if is_html && recorded_videos.iter().any(|v| v.background) {
                    // draw this slide's text separately from the next frame on
                    job.text_layer = true;
                }
                job.pending_videos = recorded_videos;
            }
        }
        if request {
            request_shot(ctx);
        }
        if n == 0 {
            cancel = true;
        }
        if cancel {
            self.export = None;
            self.status = "Export cancelled".into();
        }
        ctx.request_repaint();
    }

    // ---------- presenting ----------
    /// F, F11 or ⌃⌘F (the macOS menu shortcut) switches the presentation
    /// window in and out of full screen. F is skipped when it is a custom
    /// next/previous key.
    fn fullscreen_toggle_pressed(&self, ctx: &egui::Context) -> bool {
        let f_is_nav = self.settings.custom_keys
            && (self.settings.key(self.settings.next_key) == Key::F
                || self.settings.key(self.settings.prev_key) == Key::F);
        ctx.input_mut(|i| {
            i.consume_key(Modifiers::MAC_CMD | Modifiers::CTRL, Key::F)
                || i.consume_key(Modifiers::NONE, Key::F11)
                || (!f_is_nav && i.consume_key(Modifiers::NONE, Key::F))
        })
    }

    fn present_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let mut next = 0;
        let mut stop = false;
        ctx.input(|i| {
            for k in if self.settings.custom_keys {
                vec![self.settings.key(self.settings.next_key)]
            } else {
                vec![
                    Key::ArrowRight,
                    Key::ArrowDown,
                    Key::Space,
                    Key::PageDown,
                    Key::Enter,
                ]
            } {
                if i.key_pressed(k) {
                    next += 1;
                }
            }
            for k in if self.settings.custom_keys {
                vec![self.settings.key(self.settings.prev_key)]
            } else {
                vec![Key::ArrowLeft, Key::ArrowUp, Key::PageUp, Key::Backspace]
            } {
                if i.key_pressed(k) {
                    next -= 1;
                }
            }
            if i.key_pressed(Key::Escape) {
                stop = true;
            }
        });
        if ctx.input(|i| i.key_pressed(Key::Home)) {
            self.current = 0;
        }
        if ctx.input(|i| i.key_pressed(Key::End)) {
            self.current = self.slides.len().saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(Key::T)) {
            self.prompter = !self.prompter;
        }
        if self.fullscreen_toggle_pressed(ctx) {
            let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
        }
        self.go(next);
        if self.current != self.video_slide {
            self.images.stop_videos();
            self.video_slide = self.current;
        }
        if stop {
            self.stop_presenting(ctx);
            return;
        }
        if self.prompter {
            egui::Panel::right("prompter")
                .resizable(true)
                .default_size(420.0)
                .frame(egui::Frame::new().fill(hex(0x141414)).inner_margin(28.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Slide {} of {}",
                            self.current + 1,
                            self.slides.len()
                        ))
                        .color(crate::settings::pal().text_dim)
                        .size(14.0),
                    );
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let notes = self
                            .slides
                            .get(self.current)
                            .map(|s| s.notes.join("\n\n"))
                            .unwrap_or_default();
                        let notes = if self.settings.focus_note {
                            notes
                        } else {
                            self.slides
                                .iter()
                                .enumerate()
                                .map(|(n, s)| format!("Slide {}\n{}", n + 1, s.notes.join("\n\n")))
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        let text = if notes.is_empty() {
                            "No speaker notes on this slide.".to_string()
                        } else {
                            notes
                        };
                        ui.label(RichText::new(text).color(Color32::WHITE).size(28.0));
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK))
            .show(ui, |ui| {
                let avail = ui.max_rect();
                let rect = if self.settings.responsive {
                    avail
                } else {
                    fit_aspect(avail, self.settings.slide_aspect())
                };
                let resp = ui.allocate_rect(avail, Sense::click());
                if resp.clicked() {
                    self.go(1);
                }
                if resp.secondary_clicked() {
                    self.go(-1);
                }
                let painter = ui.painter_at(avail);
                if self.images.tick_videos(ctx) {
                    ctx.request_repaint();
                }
                images::begin_video_spots();
                self.paint_slide(&painter, rect, self.current);
                let videos = images::take_video_spots();
                // Videos play in the system's player: click one, or press V.
                // Background videos start by themselves (silent, looping).
                for v in &videos {
                    if v.background {
                        self.images.autoplay(&v.src, v.filter);
                    }
                }
                let mut play: Option<(String, Option<parse::Filter>)> = None;
                for (k, vs) in videos.iter().enumerate() {
                    // Clicking a background still moves to the next slide.
                    if vs.background {
                        continue;
                    }
                    let r = vs.rect.intersect(rect);
                    if !r.is_positive() {
                        continue;
                    }
                    let v = ui
                        .interact(r, egui::Id::unique(("present-video", k)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if v.clicked() {
                        play = Some((vs.src.clone(), vs.filter));
                    }
                }
                if ui.input(|i| i.key_pressed(Key::V)) || (self.pending_play && !videos.is_empty())
                {
                    self.pending_play = false;
                    play = videos
                        .iter()
                        .find(|v| !v.background)
                        .or(videos.first())
                        .map(|v| (v.src.clone(), v.filter));
                }
                if let Some((src, filter)) = play {
                    // Inline with ffmpeg; otherwise the system's player.
                    if self.images.toggle_video(&src, filter).is_err() {
                        self.play_video(&src);
                    }
                    ctx.request_repaint();
                }
            });
    }

    // ---------- writing ----------
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let pal = crate::settings::pal();
        let panel = egui::Panel::left("thumbs")
            .resizable(true)
            .default_size(self.settings.sidebar_width)
            .size_range(SIDEBAR_MIN..=SIDEBAR_MAX)
            .frame(egui::Frame::new().fill(pal.sidebar_bg))
            .show(ui, |ui| {
                let sw = ui.max_rect().width();
                // top bar: the window buttons (macOS) + slide count + sidebar toggle
                let top = Rect::from_min_size(ui.max_rect().min, vec2(sw, TOP_BAR));
                let toggle = Rect::from_center_size(
                    pos2(top.right() - 26.0, top.center().y),
                    vec2(30.0, 26.0),
                );
                if !IS_MAC {
                    let n = self.slides.len();
                    ui.painter().text(
                        pos2(top.left() + 14.0, top.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{n} slide{}", if n == 1 { "" } else { "s" }),
                        FontId::proportional(12.5),
                        pal.text_dim,
                    );
                }
                if icon_button(ui, toggle, Icon::Sidebar, "Hide slide list").clicked() {
                    self.show_sidebar = false;
                }
                ui.allocate_rect(top, Sense::hover());
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut clicked = None;
                        let mut present_from = None;
                        let mut insert_after = None;
                        let thumb_w = sw - 42.0;
                        let thumb_h = thumb_w / ASPECT;
                        let row_h = thumb_h + 14.0;
                        ui.add_space(2.0);
                        for i in 0..self.slides.len() {
                            let (row, resp) =
                                ui.allocate_exact_size(vec2(sw, row_h), Sense::click());
                            if !ui.is_rect_visible(row) && !(i == self.current && self.scroll_thumb)
                            {
                                continue;
                            }
                            let painter = ui.painter_at(row);
                            let col = slide_color(i);
                            let selected = i == self.current;
                            let hovered = ui.rect_contains_pointer(row);
                            let thumb = Rect::from_min_size(
                                pos2(row.left() + 32.0, row.top() + 6.0),
                                vec2(thumb_w, thumb_h),
                            );
                            // slide number
                            let num_at = pos2(row.left() + 16.0, thumb.top() + 10.0);
                            painter.circle_filled(num_at, 10.0, col);
                            painter.text(
                                num_at,
                                egui::Align2::CENTER_CENTER,
                                format!("{}", i + 1),
                                FontId::new(11.0, theme::family(false, true, false)),
                                Color32::WHITE,
                            );
                            // thumbnail with a hairline, and a ring when selected or hovered
                            self.paint_slide(&painter, thumb, i);
                            painter.rect_stroke(
                                thumb,
                                2.0,
                                Stroke::new(1.0_f32, pal.border),
                                egui::StrokeKind::Outside,
                            );
                            if selected {
                                painter.rect_stroke(
                                    thumb.expand(2.0),
                                    5.0,
                                    Stroke::new(2.5_f32, col),
                                    egui::StrokeKind::Outside,
                                );
                            } else if hovered {
                                painter.rect_stroke(
                                    thumb.expand(2.0),
                                    5.0,
                                    Stroke::new(2.0_f32, pal.hover),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            if hovered {
                                // speaking time and a present button, only while hovering
                                let secs = (speaking_seconds(&self.slide_text(i)) * SPEECH_WPM
                                    / self.settings.wpm as f32)
                                    as u32;
                                let label = format!("{}:{:02}", secs / 60, secs % 60);
                                let font = FontId::proportional(10.5);
                                let g = painter.layout_no_wrap(label, font, Color32::WHITE);
                                let pill = Rect::from_min_size(
                                    pos2(thumb.left() + 4.0, thumb.bottom() - 19.0),
                                    vec2(g.size().x + 10.0, 15.0),
                                );
                                painter.rect_filled(pill, 7.5, Color32::from_black_alpha(165));
                                painter.galley(
                                    pos2(pill.left() + 5.0, pill.center().y - g.size().y / 2.0),
                                    g,
                                    Color32::WHITE,
                                );
                                let btn = Rect::from_center_size(
                                    pos2(thumb.right() - 14.0, thumb.bottom() - 13.0),
                                    vec2(22.0, 22.0),
                                );
                                let b = ui
                                    .interact(btn, ui.scope_id().with(("play-slide", i)), Sense::click())
                                    .on_hover_text("Present from this slide");
                                painter.circle_filled(
                                    btn.center(),
                                    11.0,
                                    if b.hovered() {
                                        ACCENT_BLUE
                                    } else {
                                        Color32::from_black_alpha(165)
                                    },
                                );
                                painter.add(egui::Shape::convex_polygon(
                                    vec![
                                        btn.center() + vec2(-3.0, -5.0),
                                        btn.center() + vec2(5.0, 0.0),
                                        btn.center() + vec2(-3.0, 5.0),
                                    ],
                                    Color32::WHITE,
                                    Stroke::NONE,
                                ));
                                if b.clicked() {
                                    present_from = Some(i);
                                }
                            }
                            let resp = resp.on_hover_text_at_pointer(
                                "Click to edit · Double-click to present · Right-click for more",
                            );
                            resp.context_menu(|ui| {
                                if ui.button("Present from Here").clicked() {
                                    ui.close();
                                    present_from = Some(i);
                                }
                                if ui.button("Insert Slide After").clicked() {
                                    ui.close();
                                    insert_after = Some(i);
                                }
                            });
                            if resp.clicked() {
                                clicked = Some(i);
                            }
                            if resp.double_clicked() {
                                present_from = Some(i);
                            }
                            if selected && self.scroll_thumb {
                                resp.scroll_to_me(None);
                                self.scroll_thumb = false;
                            }
                        }
                        // "New Slide" at the end of the list
                        ui.add_space(4.0);
                        let (row, resp) =
                            ui.allocate_exact_size(vec2(sw, thumb_h * 0.55), Sense::click());
                        let r = Rect::from_min_max(
                            pos2(row.left() + 32.0, row.top()),
                            pos2(row.left() + 32.0 + thumb_w, row.bottom()),
                        );
                        let line = if resp.hovered() { pal.icon } else { pal.border };
                        let pts = [
                            r.left_top(),
                            r.right_top(),
                            r.right_bottom(),
                            r.left_bottom(),
                            r.left_top(),
                        ];
                        for w in pts.windows(2) {
                            ui.painter().extend(egui::Shape::dashed_line(
                                &[w[0], w[1]],
                                Stroke::new(1.2_f32, line),
                                4.0,
                                3.0,
                            ));
                        }
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            "+  New Slide",
                            FontId::proportional(12.5),
                            if resp.hovered() {
                                pal.text_hi
                            } else {
                                pal.text_dim
                            },
                        );
                        if resp.on_hover_text(kbd("⇧⌘N")).clicked() {
                            insert_after = Some(self.slides.len().saturating_sub(1));
                        }
                        ui.add_space(16.0);
                        if let Some(i) = clicked {
                            self.select_slide(i);
                        }
                        if let Some(i) = insert_after {
                            self.current = i;
                            self.insert_slide_template("");
                            self.scroll_thumb = true;
                        }
                        if let Some(i) = present_from {
                            self.current = i;
                            self.start_presenting(ctx);
                        }
                    });
            });
        // Remember the width once a drag has finished.
        let w = panel.response.rect.width().round();
        let dragging = ctx.input(|i| i.pointer.any_down());
        if !dragging && (w - self.settings.sidebar_width).abs() >= 1.0 && self.shot_path.is_none() {
            self.settings.sidebar_width = w.clamp(SIDEBAR_MIN, SIDEBAR_MAX);
            self.settings.save();
        }
    }

    /// Show the find bar (or re-select its text if it is already open),
    /// starting from the editor's selection when there is one.
    fn open_find(&mut self) {
        let (a, b) = (self.sel_start.min(self.cursor), self.sel_start.max(self.cursor));
        if a != b && b - a < 200 {
            let sel: String = self.text.chars().skip(a).take(b - a).collect();
            if !sel.contains('\n') {
                self.find.query = sel;
            }
        }
        self.find.open = true;
        self.find.focus_field = true;
    }

    /// Close the find bar and leave the current match selected in the editor.
    fn close_find(&mut self) {
        self.find.open = false;
        let range = self.find.current_range();
        self.jump_to = Some(range.unwrap_or((self.sel_start, self.cursor)));
    }

    fn find_bar(&mut self, ui: &mut egui::Ui, bar: Rect) {
        let pal = crate::settings::pal();
        ui.painter().rect_filled(bar, 0.0, pal.editor_bg);
        ui.painter().line_segment(
            [
                pos2(bar.left(), bar.bottom() - 0.5),
                pos2(bar.right(), bar.bottom() - 0.5),
            ],
            Stroke::new(1.0_f32, pal.divider.gamma_multiply(0.6)),
        );
        let ctx = ui.ctx().clone();
        let now = ui.input(|i| i.time);
        let field_id = egui::Id::unique(FIND_FIELD_ID);
        let field_focused = ctx.memory(|m| m.has_focus(field_id));
        let editor_focused = ctx.memory(|m| m.has_focus(egui::Id::unique(EDITOR_ID)));
        if field_focused {
            if ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Enter)) {
                self.find.step(&self.text, self.cursor, false, now);
            } else if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                self.find.step(&self.text, self.cursor, true, now);
            }
        }
        let had_focus = std::mem::replace(&mut self.find.had_focus, field_focused || editor_focused);
        if (had_focus || field_focused || editor_focused)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.find.had_focus = false;
            self.close_find();
            return;
        }

        let h = 28.0;
        let y = bar.center().y;
        let done_w = 58.0;
        let done = Rect::from_center_size(pos2(bar.right() - 16.0 - done_w / 2.0, y), vec2(done_w, h));
        let next = Rect::from_center_size(pos2(done.left() - 10.0 - 15.0, y), vec2(28.0, h));
        let prev = Rect::from_center_size(pos2(next.left() - 2.0 - 14.0, y), vec2(28.0, h));
        let field = Rect::from_min_max(
            pos2(bar.left() + 16.0, y - h / 2.0),
            pos2(prev.left() - 8.0, y + h / 2.0),
        );

        // search field: rounded box with a magnifier, the text, a count and a clear button
        let ring = if field_focused {
            ui.visuals().selection.stroke.color
        } else {
            pal.border
        };
        ui.painter().rect_filled(field, 8.0, pal.field);
        ui.painter().rect_stroke(
            field,
            8.0,
            Stroke::new(if field_focused { 2.0 } else { 1.0 }, ring),
            egui::StrokeKind::Inside,
        );
        let glass = pos2(field.left() + 15.0, y - 1.0);
        let st = Stroke::new(1.4_f32, pal.icon);
        ui.painter().circle_stroke(glass, 5.0, st);
        ui.painter()
            .line_segment([glass + vec2(3.6, 3.6), glass + vec2(7.0, 7.0)], st);

        let count = if self.find.not_found {
            "Not found".to_string()
        } else {
            String::new()
        };
        let count_g = ui.painter().layout_no_wrap(
            count,
            FontId::proportional(12.0),
            pal.text_dim,
        );
        let clear = Rect::from_center_size(pos2(field.right() - 14.0, y), vec2(18.0, 18.0));
        let has_query = !self.find.query.is_empty();
        let count_right = if has_query { clear.left() - 6.0 } else { field.right() - 10.0 };
        ui.painter().galley(
            pos2(count_right - count_g.size().x, y - count_g.size().y / 2.0),
            count_g.clone(),
            pal.text_dim,
        );
        if has_query {
            let r = ui
                .interact(clear, egui::Id::unique("find-clear"), Sense::click())
                .on_hover_text("Clear");
            let c = clear.center();
            ui.painter().circle_filled(
                c,
                7.5,
                if r.hovered() { pal.text_dim } else { pal.icon.gamma_multiply(0.7) },
            );
            let x = Stroke::new(1.4_f32, pal.field);
            ui.painter().line_segment([c + vec2(-2.8, -2.8), c + vec2(2.8, 2.8)], x);
            ui.painter().line_segment([c + vec2(-2.8, 2.8), c + vec2(2.8, -2.8)], x);
            if r.clicked() {
                self.find.query.clear();
                self.find.focus_field = true;
            }
        }
        let text_rect = Rect::from_min_max(
            pos2(field.left() + 28.0, field.top() + 2.0),
            pos2(count_right - count_g.size().x - 8.0, field.bottom() - 2.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
        let out = egui::TextEdit::singleline(&mut self.find.query)
            .id(field_id)
            .frame(egui::Frame::NONE)
            .hint_text("Find in document")
            .font(FontId::proportional(14.0))
            .text_color(pal.text_hi)
            .desired_width(text_rect.width())
            .align(egui::Align2::LEFT_CENTER)
            .min_size(text_rect.size())
            .show(&mut child);
        if std::mem::take(&mut self.find.focus_field) {
            ctx.memory_mut(|m| m.request_focus(field_id));
            let mut st = out.state.clone();
            let n = self.find.query.chars().count();
            st.cursor
                .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(n))));
            st.store(&ctx, field_id);
        }

        if icon_button(ui, prev, Icon::ChevronLeft, &format!("Previous match ({})", kbd("⇧⌘G"))).clicked() {
            self.find.step(&self.text, self.cursor, false, now);
        }
        if icon_button(ui, next, Icon::ChevronRight, &format!("Next match ({})", kbd("⌘G"))).clicked() {
            self.find.step(&self.text, self.cursor, true, now);
        }
        let r = ui
            .interact(done, egui::Id::unique("find-done"), Sense::click())
            .on_hover_text("Close (Esc)");
        ui.painter().rect_filled(
            done,
            7.0,
            if r.hovered() { pal.hover } else { pal.button },
        );
        ui.painter().text(
            done.center(),
            egui::Align2::CENTER_CENTER,
            "Done",
            FontId::proportional(13.5),
            pal.text_hi,
        );
        if r.clicked() {
            self.close_find();
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(crate::settings::pal().editor_bg))
            .show(ui, |ui| {
                let full = ui.max_rect();
                ui.painter().line_segment(
                    [full.left_top(), full.left_bottom()],
                    Stroke::new(1.0_f32, crate::settings::pal().divider),
                );
                // top bar
                let top = Rect::from_min_size(full.min, vec2(full.width(), TOP_BAR));
                let pal = crate::settings::pal();
                let mut x = top.left() + 26.0;
                if !self.show_sidebar {
                    if IS_MAC {
                        x += 70.0; // room for the window buttons
                    }
                    let r = Rect::from_center_size(pos2(x, top.center().y), vec2(30.0, 26.0));
                    if icon_button(ui, r, Icon::Sidebar, "Show slide list").clicked() {
                        self.show_sidebar = true;
                    }
                    x += 36.0;
                }
                let plus = Rect::from_center_size(pos2(x, top.center().y), vec2(30.0, 26.0));
                if icon_button(ui, plus, Icon::Plus, "New slide below the cursor").clicked() {
                    let byte = self.char_to_byte(self.cursor);
                    let end = self.text[byte..]
                        .find('\n')
                        .map(|offset| byte + offset)
                        .unwrap_or(self.text.len());
                    self.text
                        .insert_str(end, crate::mechanics::EMPTY_SLIDE_INSERT);
                    let cursor = self.text[..end].chars().count()
                        + crate::mechanics::EMPTY_SLIDE_INSERT.chars().count();
                    self.after_edit(cursor, cursor);
                }
                // document name, with a dot while there are unsaved changes
                let name = self.doc_name();
                let g = ui.painter().layout_no_wrap(
                    name,
                    FontId::new(13.5, theme::family(false, true, false)),
                    pal.title_grey,
                );
                let title_pos = top.center() - g.size() / 2.0;
                let title_rect = Rect::from_min_size(title_pos, g.size());
                ui.painter().galley(title_pos, g, pal.title_grey);
                if self.dirty {
                    ui.painter().circle_filled(
                        pos2(title_rect.right() + 9.0, title_rect.center().y),
                        3.5,
                        pal.text_dim,
                    );
                }
                let title_tip = match &self.path {
                    Some(p) => format!(
                        "{}{}",
                        p.display(),
                        if self.dirty { "\nUnsaved changes" } else { "" }
                    ),
                    None => "Not saved yet".to_string(),
                };
                ui.interact(
                    title_rect.expand(4.0),
                    egui::Id::unique("doc-title"),
                    Sense::hover(),
                )
                .on_hover_text(title_tip);
                // status messages fade out after a few seconds
                let now = ui.input(|i| i.time);
                if self.status != self.status_seen {
                    self.status_seen = self.status.clone();
                    self.status_since = now;
                }
                let age = now - self.status_since;
                if !self.status.is_empty() && age < 5.0 {
                    let a = ((5.0 - age) / 1.0).clamp(0.0, 1.0) as f32;
                    ui.painter().text(
                        pos2(top.right() - 16.0, top.center().y),
                        egui::Align2::RIGHT_CENTER,
                        &self.status,
                        FontId::proportional(12.0),
                        pal.text_dim.gamma_multiply(a),
                    );
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(50));
                }
                ui.painter().line_segment(
                    [
                        pos2(full.left(), top.bottom() - 0.5),
                        pos2(full.right(), top.bottom() - 0.5),
                    ],
                    Stroke::new(1.0_f32, pal.divider.gamma_multiply(0.6)),
                );

                let mut body = Rect::from_min_max(pos2(full.left(), top.bottom()), full.max);
                if self.find.open {
                    let bar = Rect::from_min_size(body.min, vec2(body.width(), FIND_BAR));
                    self.find_bar(ui, bar);
                    body.min.y = bar.bottom();
                }
                let col_w = (EDITOR_COLUMN * self.settings.line_length as f32 / 64.0
                    * self.settings.editor_font
                    / EDITOR_FONT)
                    .min(body.width() - 120.0)
                    .max(240.0);
                let side = ((body.width() - col_w) / 2.0).max(60.0);
                let menu_rect = Rect::from_min_size(
                    pos2(plus.right() + 6.0, top.center().y - 13.0),
                    vec2(30.0, 26.0),
                );
                let mut menu_ui = ui.new_child(egui::UiBuilder::new().max_rect(menu_rect));
                menu_ui.style_mut().spacing.interact_size.y = 26.0;
                self.menus(&mut menu_ui, ctx);
                let id = egui::Id::unique(EDITOR_ID);
                if let Some((a, b)) = self.jump_to.take() {
                    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                    st.cursor
                        .set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    st.store(ctx, id);
                    ctx.memory_mut(|m| m.request_focus(id));
                }
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
                let starts: Vec<usize> = self.slides.iter().map(|s| s.start_char).collect();
                let editor_settings = self.settings.clone();
                let current = self.current;
                // option-line icons found by the last layout: (char, key, colour)
                let icons = std::cell::RefCell::new(Vec::new());
                let now = ui.input(|i| i.time);
                let find_marks = if self.find.open {
                    self.find.refresh(&self.text, self.cursor, now);
                    if self.find.flashing(now) {
                        ctx.request_repaint();
                    }
                    self.find.marks(now, ui.visuals().selection.bg_fill)
                } else {
                    Vec::new()
                };
                let find_scroll = std::mem::take(&mut self.find.scroll)
                    .then(|| self.find.current_range())
                    .flatten();
                let mut layouter = |ui: &egui::Ui, s: &dyn egui::TextBuffer, wrap: f32| {
                    let s = s.as_str();
                    let mut job =
                        highlight(s, &editor_settings, current, &mut icons.borrow_mut());
                    find::apply_marks(&mut job, &find_marks);
                    job.wrap.max_width = wrap;
                    let g = ui.fonts_mut(|f| f.layout_job(job));
                    hanging_indent(g, s)
                };
                let mut circles: Vec<(egui::Pos2, usize)> = Vec::new();
                let scroll_to = self.scroll_editor.take();
                let mut changed = false;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        ui.add_space(14.0);
                        ui.horizontal_top(|ui| {
                            ui.add_space(side);
                            let out = egui::TextEdit::multiline(&mut self.text)
                                .id(id)
                                .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4, 2)))
                                .desired_width(col_w)
                                .desired_rows(30)
                                .lock_focus(true)
                                .layouter(&mut layouter)
                                .show(ui);
                            changed = out.response.changed();
                            if let Some(cr) = out.cursor_range {
                                let c = cr.primary.index.0;
                                let s = cr.secondary.index.0;
                                if c != self.cursor || s != self.sel_start || changed {
                                    self.cursor = c;
                                    self.sel_start = s;
                                    self.scroll_thumb = true;
                                }
                            }
                            {
                                // icons sit in the gap left before each option line
                                let size = editor_settings.editor_font * 0.78;
                                let gap = editor_settings.editor_font * OPTION_GAP;
                                let painter = ui.painter();
                                for (ch, key, color) in icons.borrow().iter() {
                                    let r = out.galley.pos_from_cursor(CCursor::new(*ch));
                                    let c = pos2(
                                        out.galley_pos.x + r.left() - gap * 0.55,
                                        // text is centred in the row; nudge up to the
                                        // middle of capitals and digits
                                        out.galley_pos.y + r.center().y
                                            - editor_settings.editor_font * 0.04,
                                    );
                                    optionicons::paint(painter, key, c, size, *color);
                                }
                            }
                            if !changed {
                                self.table_toolbar(ui, &out.galley, out.galley_pos);
                                self.image_toolbar(ui, &out.galley, out.galley_pos);
                                self.title_toolbar(ui, &out.galley, out.galley_pos);
                            }
                            if let Some((a, b)) = find_scroll {
                                let r = out
                                    .galley
                                    .pos_from_cursor(CCursor::new(a))
                                    .union(out.galley.pos_from_cursor(CCursor::new(b)))
                                    .translate(out.galley_pos.to_vec2());
                                ui.scroll_to_rect(r, Some(egui::Align::Center));
                            }
                            if let Some(ch) = scroll_to {
                                // bring the clicked slide's first line near the top of the editor
                                let r = out.galley.pos_from_cursor(CCursor::new(ch));
                                let y = out.galley_pos.y + r.top() - 90.0;
                                let target =
                                    Rect::from_min_size(pos2(out.galley_pos.x, y), vec2(1.0, 1.0));
                                ui.scroll_to_rect(target, Some(egui::Align::TOP));
                            }
                            {
                                // After a keystroke the slide starts above are
                                // stale; read them from the new text so the
                                // numbers stay put instead of blinking out.
                                let fresh: Vec<usize>;
                                let starts = if changed {
                                    fresh = parse_document(&self.text)
                                        .iter()
                                        .map(|s| s.start_char)
                                        .collect();
                                    &fresh
                                } else {
                                    &starts
                                };
                                for (i, start) in starts.iter().enumerate() {
                                    let first = first_content_char(&self.text, *start);
                                    let r = out.galley.pos_from_cursor(CCursor::new(first));
                                    circles.push((
                                        pos2(
                                            out.galley_pos.x - 48.0,
                                            out.galley_pos.y + r.center().y,
                                        ),
                                        i,
                                    ));
                                }
                            }
                        });
                        ui.add_space(body.height() * 0.5);
                        let painter = ui.painter();
                        for (c, i) in &circles {
                            painter.circle_filled(*c, 11.5, slide_color(*i));
                            painter.text(
                                *c,
                                egui::Align2::CENTER_CENTER,
                                format!("{}", i + 1),
                                FontId::new(
                                    if *i + 1 >= 10 { 10.5 } else { 12.0 },
                                    theme::family(false, true, false),
                                ),
                                Color32::WHITE,
                            );
                        }
                    });
                if changed {
                    if self.settings.three_returns {
                        let at = self.char_to_byte(self.cursor);
                        if self.text[..at].ends_with("\n\n\n") {
                            self.text.replace_range(at - 3..at, "\n\n---\n");
                            self.cursor += 3;
                            self.jump_to = Some((self.cursor, self.cursor));
                        }
                    }
                    self.dirty = true;
                    self.reparse();
                }
                if self.scroll_thumb || changed {
                    self.current = slide_at_char(&self.slides, self.cursor);
                }
                // speaking time pill
                if self.settings.show_speaking_time {
                    let secs = self.speaking_seconds() as u32;
                    let label = format!(
                        "{:02}:{:02}:{:02}",
                        secs / 3600,
                        (secs / 60) % 60,
                        secs % 60
                    );
                    let font = FontId::proportional(13.0);
                    let g = ui.painter().layout_no_wrap(label, font, pal.text_hi);
                    let pill = Rect::from_min_size(
                        pos2(full.right() - g.size().x - 62.0, full.bottom() - 46.0),
                        vec2(g.size().x + 44.0, 30.0),
                    );
                    let p = ui.painter();
                    p.rect_filled(
                        pill.translate(vec2(0.0, 2.0)),
                        15.0,
                        Color32::from_black_alpha(18),
                    );
                    p.rect_filled(pill, 15.0, pal.pill);
                    p.rect_stroke(
                        pill,
                        15.0,
                        Stroke::new(1.0_f32, pal.border),
                        egui::StrokeKind::Inside,
                    );
                    let c = pos2(pill.left() + 17.0, pill.center().y);
                    let st = Stroke::new(1.3_f32, pal.icon);
                    p.circle_stroke(c, 6.0, st);
                    p.line_segment([c, c + vec2(0.0, -3.5)], st);
                    p.line_segment([c, c + vec2(2.8, 0.0)], st);
                    p.galley(
                        pos2(pill.left() + 30.0, pill.center().y - g.size().y / 2.0),
                        g,
                        pal.text_hi,
                    );
                    ui.interact(pill, egui::Id::unique("speaking-time"), Sense::hover())
                        .on_hover_text(format!(
                            "Estimated speaking time at {} words per minute",
                            self.settings.wpm
                        ));
                }
            });
    }
}

/// First non-blank character at or after `start` (where the slide's number goes).
fn first_content_char(text: &str, start: usize) -> usize {
    for (k, c) in text.chars().skip(start).enumerate() {
        if !c.is_whitespace() {
            return start + k;
        }
    }
    start
}

fn fit_16_9(avail: Rect) -> Rect {
    fit_aspect(avail, ASPECT)
}
fn fit_aspect(avail: Rect, aspect: f32) -> Rect {
    let w = avail.width().min(avail.height() * aspect);
    Rect::from_center_size(avail.center(), vec2(w, w / aspect))
}

#[derive(Clone, Copy)]
pub enum Icon {
    Sidebar,
    Plus,
    Picture,
    Pen,
    Play,
    Menu,
    Brush,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    ChevronUp,
    Pause,
    Gear,
    Copy,
}

pub fn paint_icon(p: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    let c = r.center();
    match icon {
        Icon::ChevronDown => {
            p.line_segment([c + vec2(-4.0, -2.0), c + vec2(0.0, 2.0)], s);
            p.line_segment([c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)], s);
        }
        Icon::ChevronUp => {
            p.line_segment([c + vec2(-4.0, 2.0), c + vec2(0.0, -2.0)], s);
            p.line_segment([c + vec2(0.0, -2.0), c + vec2(4.0, 2.0)], s);
        }
        Icon::Pause => {
            for dx in [-3.0, 3.0] {
                p.rect_filled(
                    Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.0, 12.0)),
                    1.0,
                    color,
                );
            }
        }
        Icon::Copy => {
            // two stacked sheets, like SF Symbols "doc.on.doc"
            let front = Rect::from_min_max(c + vec2(-2.0, -3.0), c + vec2(7.0, 8.0));
            p.rect_stroke(front, 2.0, s, egui::StrokeKind::Inside);
            // the back sheet shows only where the front one doesn't cover it
            p.add(egui::Shape::line(
                vec![
                    c + vec2(-2.0, 4.0),
                    c + vec2(-6.0, 4.0),
                    c + vec2(-6.0, -7.0),
                    c + vec2(3.0, -7.0),
                    c + vec2(3.0, -3.0),
                ],
                s,
            ));
        }
        Icon::Gear => {
            for k in 0..8 {
                let a = std::f32::consts::TAU * k as f32 / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 5.5, c + d * 8.0], Stroke::new(2.4_f32, color));
            }
            p.circle_stroke(c, 5.5, s);
            p.circle_stroke(c, 2.2, Stroke::new(1.3_f32, color));
        }
        Icon::Sidebar => {
            let b = Rect::from_center_size(c, vec2(18.0, 14.0));
            let pane = Rect::from_min_max(b.min, pos2(b.left() + 6.5, b.bottom()));
            p.rect_filled(
                pane,
                egui::CornerRadius {
                    nw: 3,
                    sw: 3,
                    ne: 0,
                    se: 0,
                },
                color.gamma_multiply(0.35),
            );
            p.rect_stroke(b, 3.0, s, egui::StrokeKind::Inside);
            p.line_segment(
                [pos2(pane.right(), b.top()), pos2(pane.right(), b.bottom())],
                s,
            );
        }
        Icon::Plus => {
            let b = Rect::from_center_size(c, vec2(17.0, 17.0));
            p.rect_stroke(b, 4.0, s, egui::StrokeKind::Inside);
            p.line_segment([c - vec2(4.5, 0.0), c + vec2(4.5, 0.0)], s);
            p.line_segment([c - vec2(0.0, 4.5), c + vec2(0.0, 4.5)], s);
        }
        Icon::Picture => {
            // frame, sun and two filled peaks
            let b = Rect::from_center_size(c, vec2(19.0, 15.0));
            p.rect_stroke(b, 3.0, s, egui::StrokeKind::Inside);
            p.circle_filled(pos2(b.right() - 5.0, b.top() + 4.8), 1.8, color);
            let base = b.bottom() - 2.6;
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(b.left() + 2.6, base),
                    pos2(b.left() + 7.5, b.top() + 5.5),
                    pos2(b.left() + 12.4, base),
                ],
                color,
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(b.left() + 9.5, base),
                    pos2(b.right() - 5.2, b.top() + 8.5),
                    pos2(b.right() - 2.6, base),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pen => {
            p.line_segment(
                [c + vec2(-6.0, 6.0), c + vec2(6.0, -6.0)],
                Stroke::new(2.2_f32, color),
            );
            p.line_segment([c + vec2(3.0, -9.0), c + vec2(9.0, -3.0)], s);
            p.line_segment([c + vec2(-8.0, 8.0), c + vec2(-5.0, 7.0)], s);
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(-4.5, -7.0),
                    c + vec2(7.5, 0.0),
                    c + vec2(-4.5, 7.0),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Brush => {
            // paintbrush.pointed: handle at top right, bristles to a point at bottom left
            let d = vec2(-1.0, 1.0) / 2.0_f32.sqrt(); // along the brush, toward the tip
            let n = vec2(1.0, 1.0) / 2.0_f32.sqrt(); // across the brush
            let base = c + vec2(0.5, -0.5);
            p.line_segment(
                [c + vec2(7.5, -7.5), base - d * 2.6],
                Stroke::new(2.4_f32, color),
            );
            // ferrule overlaps the bristles so no seam shows between them
            p.line_segment([base - d * 2.8, base + d * 0.6], Stroke::new(4.6_f32, color));
            let tip = base + d * 10.0;
            p.add(egui::Shape::convex_polygon(
                vec![
                    base + n * 2.5,
                    base + d * 2.6 + n * 3.0,
                    base + d * 6.0 + n * 1.9,
                    tip,
                    base + d * 6.0 - n * 1.9,
                    base + d * 2.6 - n * 3.0,
                    base - n * 2.5,
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Menu => {
            for dy in [-5.0, 0.0, 5.0] {
                p.line_segment([c + vec2(-8.0, dy), c + vec2(8.0, dy)], s);
            }
        }
        Icon::ChevronLeft => {
            p.add(egui::Shape::line(
                vec![c + vec2(3.0, -6.0), c + vec2(-3.0, 0.0), c + vec2(3.0, 6.0)],
                Stroke::new(2.0_f32, color),
            ));
        }
        Icon::ChevronRight => {
            p.add(egui::Shape::line(
                vec![
                    c + vec2(-3.0, -6.0),
                    c + vec2(3.0, 0.0),
                    c + vec2(-3.0, 6.0),
                ],
                Stroke::new(2.0_f32, color),
            ));
        }
    }
}

pub fn icon_button(ui: &mut egui::Ui, r: Rect, icon: Icon, tip: &str) -> egui::Response {
    let resp = ui
        .interact(r, ui.scope_id().with(tip), Sense::click())
        .on_hover_text(tip);
    let pal = crate::settings::pal();
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(r, 6.0, pal.button);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 6.0, pal.hover);
    }
    paint_icon(
        ui.painter(),
        r,
        icon,
        if resp.hovered() {
            pal.text_hi
        } else {
            pal.icon
        },
    );
    resp
}

/// Wrapped lines of a tab-indented paragraph line up under its text (after
/// the ⇥ marker) instead of under the marker, like a hanging indent.
fn hanging_indent(g: std::sync::Arc<egui::Galley>, text: &str) -> std::sync::Arc<egui::Galley> {
    use std::cell::RefCell;
    use std::sync::Arc;
    thread_local! {
        static CACHE: RefCell<Option<(Arc<egui::Galley>, Arc<egui::Galley>)>> =
            const { RefCell::new(None) };
    }
    // egui reuses the same galley while nothing changes, so cache by identity
    // (the input is kept alive so its address can't be reused).
    if let Some(hit) = CACHE.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(input, _)| Arc::ptr_eq(input, &g))
            .map(|(_, out)| out.clone())
    }) {
        return hit;
    }
    let tabs: Vec<usize> = text
        .split('\n')
        .map(|l| l.chars().take_while(|c| *c == '\t').count())
        .collect();
    let mut out = (*g).clone();
    let mut para = 0usize;
    let mut first_row = true;
    let mut indent = 0.0_f32;
    let mut changed = false;
    for row in out.rows.iter_mut() {
        let k = tabs.get(para).copied().unwrap_or(0);
        if first_row {
            indent = if k > 0 {
                // Glyph positions are relative to their row.
                row.glyphs.get(k).map(|gl| gl.pos.x).unwrap_or(0.0)
            } else {
                0.0
            };
        } else if k > 0 && indent > 0.0 {
            row.pos.x += indent;
            changed = true;
        }
        first_row = row.ends_with_newline;
        if row.ends_with_newline {
            para += 1;
        }
    }
    let input = g.clone();
    let result = if changed {
        for row in &out.rows {
            out.rect = out.rect.union(row.rect());
            out.mesh_bounds = out
                .mesh_bounds
                .union(row.visuals.mesh_bounds.translate(row.pos.to_vec2()));
        }
        Arc::new(out)
    } else {
        g
    };
    CACHE.with(|c| *c.borrow_mut() = Some((input, result.clone())));
    result
}

/// Editor colouring: on-slide text (headings, tabbed lines,
/// pictures, tables) in the slide's colour, spoken text light grey, slide
/// breaks dim grey, tabs shown as an arrow.
fn highlight(
    s: &str,
    settings: &settings::Settings,
    current: usize,
    icons: &mut Vec<(usize, &'static str, Color32)>,
) -> LayoutJob {
    icons.clear();
    let mut job = LayoutJob::default();
    let mut at = 0usize; // char index of the current line
    let reg = FontId::new(
        settings.editor_font,
        if settings.typography == 2 {
            egui::FontFamily::Proportional
        } else {
            theme::mono(false)
        },
    );
    let bold = FontId::new(
        settings.editor_font,
        if settings.typography == 2 {
            theme::family(false, true, false)
        } else {
            theme::mono(true)
        },
    );
    let mut slide = 0usize;
    let mut prev_media = false;
    // Just after a level-one heading, where `x:` / `y:` place the cover title.
    let mut prev_cover = false;
    let mut in_fence = false;
    let fmt = |font: &FontId, color: Color32| TextFormat {
        font_id: font.clone(),
        color,
        line_height: Some(EDITOR_LINE * settings.editor_font / EDITOR_FONT),
        valign: egui::Align::Center,
        ..Default::default()
    };
    for line in s.split_inclusive('\n') {
        let line_at = at;
        at += line.chars().count();
        let body = line.trim_end_matches('\n');
        let t = body.trim();
        let lower = t.to_ascii_lowercase();
        let col = if settings.focus_slide && slide != current {
            settings::pal().text_dim
        } else {
            slide_color(slide)
        };
        if t.starts_with("```") {
            in_fence = !in_fence;
            prev_cover = false;
            job.append(line, 0.0, fmt(&reg, col));
            continue;
        }
        if in_fence {
            job.append(line, 0.0, fmt(&reg, col));
            continue;
        }
        if parse::is_separator(body) {
            job.append(line, 0.0, fmt(&reg, crate::settings::pal().sep_grey));
            slide += 1;
            prev_media = false;
            prev_cover = false;
            continue;
        }
        let cover_opt = prev_cover && !body.starts_with('\t') && parse::title_option(t).is_some();
        let is_kv = optionicons::key_of(t).is_some();
        let is_media = !t.contains(' ')
            && !t.is_empty()
            && (lower.starts_with("http")
                || [
                    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".mp4", ".mov", ".tif",
                    ".tiff", ".pdf",
                ]
                .iter()
                .any(|e| lower.ends_with(e)));
        let media_line = t.starts_with('|')
            || is_media
            || t.starts_with("![")
            || (prev_media && (is_kv || t.starts_with('[')))
            || cover_opt;
        // Syntax (pipes, option lines, alignment rows) is quieter than content.
        let quiet = col.gamma_multiply(0.55);
        if body.starts_with('#') {
            let hashes = body.chars().take_while(|c| *c == '#').count();
            let (marks, rest) = line.split_at(hashes);
            job.append(marks, 0.0, fmt(&bold, col.gamma_multiply(0.7)));
            job.append(rest, 0.0, fmt(&bold, col));
        } else if t.starts_with('|') {
            let separator = t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'));
            if separator {
                job.append(line, 0.0, fmt(&reg, quiet));
            } else {
                let mut cell = String::new();
                for ch in line.chars() {
                    if ch == '|' {
                        if !cell.is_empty() {
                            job.append(&std::mem::take(&mut cell), 0.0, fmt(&reg, col));
                        }
                        job.append("|", 0.0, fmt(&reg, quiet));
                    } else {
                        cell.push(ch);
                    }
                }
                if !cell.is_empty() {
                    job.append(&cell, 0.0, fmt(&reg, col));
                }
            }
        } else if media_line && !is_media && (is_kv || t.starts_with('[')) {
            if let Some(key) = optionicons::key_of(t) {
                let lead = line.len() - line.trim_start().len();
                let (ws, rest) = line.split_at(lead);
                job.append(ws, 0.0, fmt(&reg, quiet));
                icons.push((line_at + ws.chars().count(), key, quiet));
                job.append(rest, settings.editor_font * OPTION_GAP, fmt(&reg, quiet));
            } else {
                job.append(line, 0.0, fmt(&reg, quiet));
            }
        } else if media_line {
            job.append(line, 0.0, fmt(&reg, col));
        } else if let Some(rest) = line.strip_prefix('\t') {
            let tf = fmt(&reg, crate::settings::pal().tab_grey);
            job.append("⇥", 0.0, tf);
            let mut f = fmt(&reg, col);
            if rest.trim_start().to_ascii_lowercase().starts_with("- [x]") {
                if settings.fade_tasks {
                    f.color = settings::pal().text_dim;
                }
                if settings.strike_tasks {
                    f.strikethrough = Stroke::new(1.0_f32, f.color);
                }
            }
            // space after the marker so on-slide text starts on its own column
            job.append(rest, settings.editor_font * 0.75, f);
        } else {
            job.append(line, 0.0, fmt(&reg, crate::settings::pal().note_grey));
        }
        prev_media = if t.is_empty() || cover_opt {
            false
        } else {
            media_line
        };
        prev_cover = cover_opt || (body.starts_with("# ") || body.trim_end() == "#");
    }
    job
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        #[cfg(target_os = "macos")]
        for p in macos_open::take_pending() {
            self.open_requested(p);
        }
        // A newly picked system font: load it now, draw with it next frame.
        if fonts::begin_frame() {
            theme::install_fonts(ctx);
            ctx.request_repaint();
        }
        settings::apply_appearance(ctx, &self.settings);
        self.poll_update_check();
        self.auto_check_updates(ctx);
        self.screenshot_tick(ctx);
        self.control_drain(ctx);
        if self.pending_present {
            self.pending_present = false;
            self.start_presenting(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Comma)) {
            self.settings_open = !self.settings_open;
        }
        self.handle_drops(ctx);
        self.update_title(ctx);
        let cmd = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, k));
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S)) {
            self.save(true);
        } else if cmd(Key::S) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::N)) {
            self.insert_slide_template("");
        }
        if cmd(Key::W) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_replace() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if cmd(Key::O) {
            self.open_dialog();
        }
        if cmd(Key::N) {
            self.new_doc();
        }
        if !self.presenting && self.export.is_none() {
            if cmd(Key::F) {
                self.open_find();
            }
            if self.find.open {
                let now = ctx.input(|i| i.time);
                if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::G)) {
                    self.find.step(&self.text, self.cursor, false, now);
                } else if cmd(Key::G) {
                    self.find.step(&self.text, self.cursor, true, now);
                }
            }
        }
        if cmd(Key::Enter) {
            if self.presenting {
                self.stop_presenting(ctx);
            } else {
                self.start_presenting(ctx);
            }
        }
        // Presenting normally happens in its own window; screenshot runs and
        // single-window backends draw it here instead of the editor.
        let inline_present = self.shot_path.is_some() || ctx.embed_viewports();
        if self.presenting && !inline_present {
            self.present_window(ctx);
        }
        if self.export.is_some() {
            self.export_ui(ui);
            return;
        }
        if self.presenting && inline_present {
            self.present_ui(ui);
            return;
        }
        // The editor window becomes the presenter view.
        if self.presenting {
            self.presenter_view(ui);
            return;
        }
        if !self.settings_open && ctx.memory(|m| m.has_focus(egui::Id::unique(EDITOR_ID))) {
            let settings = self.settings.clone();
            ctx.input_mut(|i| {
                for event in &mut i.events {
                    if let egui::Event::Text(text) = event {
                        *text = settings::typing(text, &settings);
                    }
                }
            });
        }
        self.format_shortcuts(ctx);
        if self.show_sidebar {
            self.sidebar(ui);
        }
        self.inspector(ui);
        self.editor(ui);
        self.table_editor_ui(ctx);
        self.settings_window(ctx);
        self.update_banner(ctx);
    }
}

#[cfg(test)]
mod shortcut_label_tests {
    use super::kbd;

    #[test]
    fn media_paths_are_relative_to_the_document() {
        use std::path::Path;
        let doc = Path::new("/a/b");
        assert_eq!(
            super::media_ref_for(Some(doc), Path::new("/a/b/sagarwar.mp4")),
            "./sagarwar.mp4"
        );
        assert_eq!(
            super::media_ref_for(Some(doc), Path::new("/a/b/media/x y.jpg")),
            "./media/x%20y.jpg"
        );
        assert_eq!(
            super::media_ref_for(Some(doc), Path::new("/c/d.jpg")),
            "/c/d.jpg"
        );
        assert_eq!(
            super::media_ref_for(None, Path::new("/a/b/e.jpg")),
            "/a/b/e.jpg"
        );
    }

    #[test]
    fn labels_follow_the_platform() {
        if super::IS_MAC {
            assert_eq!(kbd("⇧⌘L"), "⇧⌘L");
        } else {
            assert_eq!(kbd("⇧⌘L"), "Ctrl+Shift+L");
            assert_eq!(kbd("^⌘K"), "Ctrl+Alt+K");
            assert_eq!(kbd("⌥⌘U"), "Ctrl+Alt+U");
            assert_eq!(kbd("⌘↵"), "Ctrl+Enter");
        }
        assert_eq!(kbd("Tab"), "Tab");
        assert_eq!(kbd("Just Type"), "Just Type");
    }
}
