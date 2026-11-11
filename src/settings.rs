//! PresentCraft settings: General, Editor, Presentation, Themes, Markdown.
//! Saved as simple `key=value` lines in the user's Application Support folder.

use eframe::egui;
use egui::{pos2, vec2, Color32, FontId, Rect, RichText, Sense, Stroke};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------- window colours (light / dark) ----------

pub struct Pal {
    pub editor_bg: Color32,
    pub sidebar_bg: Color32,
    pub inspector_bg: Color32,
    pub divider: Color32,
    pub text_hi: Color32,
    pub text_dim: Color32,
    pub title_grey: Color32,
    pub note_grey: Color32,
    pub sep_grey: Color32,
    pub tab_grey: Color32,
    pub button: Color32,
    pub control: Color32,
    pub hover: Color32,
    /// Toolbar icons.
    pub icon: Color32,
    /// Hairline around slide thumbnails and text fields.
    pub border: Color32,
    /// Text field background.
    pub field: Color32,
    /// Floating pills (speaking time).
    pub pill: Color32,
}

use crate::theme::hex;

/// Dark editor palette.
pub const DARK_PAL: Pal = Pal {
    editor_bg: hex(0x181818),
    sidebar_bg: hex(0x222222),
    inspector_bg: hex(0x1E1E1E),
    divider: hex(0x2C2C2C),
    text_hi: hex(0xE8E8E8),
    text_dim: hex(0x6B6B6B),
    title_grey: hex(0xBFBDBF),
    note_grey: hex(0xC5C5C5),
    sep_grey: hex(0x757575),
    tab_grey: hex(0x515151),
    button: hex(0x383838),
    control: hex(0x2F2F2F),
    hover: hex(0x3A3A3A),
    icon: hex(0xA8A8A8),
    border: hex(0x343434),
    field: hex(0x262626),
    pill: hex(0x2A2A2A),
};

/// Light editor palette.
pub const LIGHT_PAL: Pal = Pal {
    editor_bg: hex(0xFFFFFF),
    sidebar_bg: hex(0xF4F4F5),
    inspector_bg: hex(0xF7F7F8),
    divider: hex(0xE2E2E4),
    text_hi: hex(0x1C1C1E),
    text_dim: hex(0x86868B),
    title_grey: hex(0x4A4A4A),
    note_grey: hex(0x3C3C3C),
    sep_grey: hex(0xA0A0A0),
    tab_grey: hex(0xBDBDBD),
    button: hex(0xE8E8EA),
    control: hex(0xEAEAEC),
    hover: hex(0xDDDDE0),
    icon: hex(0x6E6E73),
    border: hex(0xD6D6DA),
    field: hex(0xFFFFFF),
    pill: hex(0xFFFFFF),
};

static DARK_UI: AtomicBool = AtomicBool::new(true);
static SMART_PUNCT: AtomicBool = AtomicBool::new(false);
static SINGLE_RETURN: AtomicBool = AtomicBool::new(false);
static FIT_TEXT: AtomicBool = AtomicBool::new(true);

pub fn pal() -> &'static Pal {
    if DARK_UI.load(Ordering::Relaxed) {
        &DARK_PAL
    } else {
        &LIGHT_PAL
    }
}
/// Whether the window currently uses the dark look.
pub fn is_dark() -> bool {
    DARK_UI.load(Ordering::Relaxed)
}
pub fn single_return() -> bool {
    SINGLE_RETURN.load(Ordering::Relaxed)
}
pub fn smart_punct() -> bool {
    SMART_PUNCT.load(Ordering::Relaxed)
}
pub fn fit_text() -> bool {
    FIT_TEXT.load(Ordering::Relaxed)
}

// ---------- settings ----------

pub const TYPOGRAPHY: [&str; 3] = ["Duo", "Mono", "Quattro"];
pub const ASPECTS: [(&str, f32); 3] = [
    ("Wide 16:9", 16.0 / 9.0),
    ("Wide 16:10", 16.0 / 10.0),
    ("Standard 4:3", 4.0 / 3.0),
];
pub const KEYS: [&str; 7] = [
    "Up",
    "Down",
    "Left",
    "Right",
    "Space",
    "Page Up",
    "Page Down",
];

#[derive(Clone, PartialEq)]
pub struct Settings {
    // General
    pub match_system: bool,
    pub dark: bool,
    pub new_template: String,
    // Editor
    pub editor_font: f32,
    pub typography: usize,
    pub line_length: usize,
    pub focus_slide: bool,
    pub fade_tasks: bool,
    pub strike_tasks: bool,
    pub three_returns: bool,
    pub smart_quotes: bool,
    pub smart_dashes: bool,
    // Presentation
    pub responsive: bool,
    pub aspect: usize,
    pub fit_text: bool,
    pub focus_note: bool,
    pub custom_keys: bool,
    pub prev_key: usize,
    pub next_key: usize,
    pub warn_empty: bool,
    pub wpm: u32,
    pub show_speaking_time: bool,
    /// Videos start by themselves when their slide appears.
    pub video_autoplay: bool,
    /// Play button, time and progress bar over videos.
    pub video_controls: bool,
    // Markdown
    pub bullet: usize,
    pub ordered_paren: bool,
    pub table_padded: bool,
    pub italic_underscore: bool,
    pub bold_underscore: bool,
    pub single_return: bool,
    pub smart_punct: bool,
    /// Width of the slide list, in points (the person can drag it).
    pub sidebar_width: f32,
    // Updates
    pub check_updates: bool,
    /// When the last update check succeeded (seconds since 1970).
    pub last_update_check: u64,
    /// A release the person chose to skip; it isn't offered again.
    pub skipped_version: String,
}

impl Settings {
    pub fn defaults(sample: &str) -> Self {
        Settings {
            match_system: false,                          // useSystemAppearance = false
            dark: true,                                   // useDarkMode = true
            new_template: sample.to_string(), // includeSampleSlidesOnNewPresentation = true
            editor_font: 18.5,                // measured from reference/picture-1.png
            typography: 0,                    // "Duo" (Resources/Duo), person's screenshot
            line_length: 64,                  // person's screenshot (lineLengthGroup)
            focus_slide: false,               // editorFocus = false
            fade_tasks: crate::mechanics::FADE_TASKS, // fadeCompletedTasks = true
            strike_tasks: crate::mechanics::STRIKE_TASKS, // strikethroughCompletedTasks = true
            three_returns: crate::mechanics::THREE_RETURNS,
            smart_quotes: false, // automaticQuoteSubstitutionEnabled = false
            smart_dashes: false, // automaticDashSubstitutionEnabled = false
            responsive: true,    // preferedPresentationLayout = "responsive"
            aspect: 0,           // preferedPresentationAspectRatio = "wide"
            fit_text: true,      // fitText = true
            focus_note: true,    // teleprompterFocus = true
            custom_keys: false,  // useCustomPresenterKeys = false
            prev_key: 0,         // previousSlideKey = 0 (Up)
            next_key: 1,         // nextSlideKey = 1 (Down)
            warn_empty: crate::mechanics::WARN_EMPTY, // showEmptySlidesWarning = true
            wpm: crate::mechanics::SPEECH_WPM, // speechWordsPerMinute = 110
            show_speaking_time: crate::mechanics::SHOW_SPEAKING_TIME,
            video_autoplay: crate::mechanics::VIDEO_AUTOPLAY,
            video_controls: crate::mechanics::VIDEO_CONTROLS,
            bullet: 0,                // markdownListBullet = 0 (hyphen)
            ordered_paren: false,     // markdownOrderedListDelimiter = 0 (period)
            table_padded: true,       // markdownTableFormatting = 1 (padded)
            italic_underscore: false, // markdownEmphasisKind = 0 (asterisks)
            bold_underscore: false,   // markdownStrongEmphasisKind = 0
            single_return: crate::mechanics::SINGLE_RETURN, // singleReturnStartsANewParagraph = true
            smart_punct: false,                             // smartPonctuation = false
            sidebar_width: crate::SIDEBAR_W,
            check_updates: true,
            last_update_check: 0,
            skipped_version: String::new(),
        }
    }

    /// Where preferences live: Application Support on macOS, %APPDATA% on
    /// Windows, and $XDG_CONFIG_HOME (or ~/.config) on Linux and others.
    fn path_for(app_name: &str, config_name: &str) -> Option<PathBuf> {
        if cfg!(target_os = "macos") {
            let home = std::env::var_os("HOME")?;
            return Some(
                PathBuf::from(home)
                    .join("Library/Application Support")
                    .join(app_name)
                    .join("settings.txt"),
            );
        }
        if cfg!(windows) {
            let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("USERPROFILE"))?;
            return Some(PathBuf::from(base).join(app_name).join("settings.txt"));
        }
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join(config_name).join("settings.txt"))
    }

    fn path() -> Option<PathBuf> {
        Self::path_for("PresentCraft", "presentcraft")
    }

    /// The app's own preferences folder.
    pub fn app_dir() -> Option<PathBuf> {
        Self::path().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    pub fn load(sample: &str) -> Self {
        let mut s = Settings::defaults(sample);
        let Some(p) = Self::path() else { return s };
        // Keep existing preferences when upgrading from the previous app name.
        let loaded = if p.exists() {
            std::fs::read_to_string(p)
        } else {
            Self::path_for("Slide Writer", "slide-writer")
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "No settings directory")
                })
                .and_then(std::fs::read_to_string)
        };
        let Ok(text) = loaded else {
            return s;
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let b = v == "true";
            let n = v.parse::<f64>().unwrap_or(0.0);
            match k {
                "match_system" => s.match_system = b,
                "dark" => s.dark = b,
                "new_template" => s.new_template = decode_template(v),
                "editor_font" => s.editor_font = (n as f32).clamp(12.0, 30.0),
                "typography" => s.typography = (n as usize).min(2),
                "line_length" => s.line_length = (n as usize).clamp(40, 120),
                "focus_slide" => s.focus_slide = b,
                "fade_tasks" => s.fade_tasks = v.parse().unwrap_or(crate::mechanics::FADE_TASKS),
                "strike_tasks" => {
                    s.strike_tasks = v.parse().unwrap_or(crate::mechanics::STRIKE_TASKS)
                }
                "three_returns" => {
                    s.three_returns = v.parse().unwrap_or(crate::mechanics::THREE_RETURNS)
                }
                "smart_quotes" => {
                    s.smart_quotes = v.parse().unwrap_or(crate::mechanics::SMART_QUOTES_INVALID)
                }
                "smart_dashes" => {
                    s.smart_dashes = v.parse().unwrap_or(crate::mechanics::SMART_DASHES_INVALID)
                }
                "responsive" => s.responsive = b,
                "aspect" => s.aspect = (n as usize).min(2),
                "fit_text" => s.fit_text = b,
                "focus_note" => s.focus_note = b,
                "custom_keys" => s.custom_keys = b,
                "prev_key" => s.prev_key = (n as usize).min(KEYS.len() - 1),
                "next_key" => s.next_key = (n as usize).min(KEYS.len() - 1),
                "warn_empty" => s.warn_empty = v.parse().unwrap_or(crate::mechanics::WARN_EMPTY),
                "wpm" => {
                    s.wpm = v
                        .parse::<u32>()
                        .unwrap_or(crate::mechanics::SPEECH_WPM)
                        .clamp(40, 400)
                }
                "show_speaking_time" => {
                    s.show_speaking_time = v.parse().unwrap_or(crate::mechanics::SHOW_SPEAKING_TIME)
                }
                "video_autoplay" => {
                    s.video_autoplay = v.parse().unwrap_or(crate::mechanics::VIDEO_AUTOPLAY)
                }
                "video_controls" => {
                    s.video_controls = v.parse().unwrap_or(crate::mechanics::VIDEO_CONTROLS)
                }
                "bullet" => s.bullet = (n as usize).min(2),
                "ordered_paren" => s.ordered_paren = b,
                "table_padded" => s.table_padded = b,
                "italic_underscore" => s.italic_underscore = b,
                "bold_underscore" => s.bold_underscore = b,
                "single_return" => {
                    s.single_return = v.parse().unwrap_or(crate::mechanics::SINGLE_RETURN)
                }
                "smart_punct" => s.smart_punct = b,
                "sidebar_width" => {
                    s.sidebar_width = v
                        .parse::<f32>()
                        .unwrap_or(crate::SIDEBAR_W)
                        .clamp(crate::SIDEBAR_MIN, crate::SIDEBAR_MAX)
                }
                "check_updates" => s.check_updates = v.parse().unwrap_or(true),
                "last_update_check" => s.last_update_check = v.parse().unwrap_or(0),
                "skipped_version" => s.skipped_version = v.trim().to_string(),
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let Some(p) = Self::path() else { return };
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let t = self
            .new_template
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
        let out = format!(
            "match_system={}\ndark={}\nnew_template={}\neditor_font={}\ntypography={}\nline_length={}\nfocus_slide={}\nfade_tasks={}\nstrike_tasks={}\nthree_returns={}\nsmart_quotes={}\nsmart_dashes={}\nresponsive={}\naspect={}\nfit_text={}\nfocus_note={}\ncustom_keys={}\nprev_key={}\nnext_key={}\nwarn_empty={}\nwpm={}\nshow_speaking_time={}\nbullet={}\nordered_paren={}\ntable_padded={}\nitalic_underscore={}\nbold_underscore={}\nsingle_return={}\nsmart_punct={}\nsidebar_width={}\ncheck_updates={}\nlast_update_check={}\nskipped_version={}\nvideo_autoplay={}\nvideo_controls={}\n",
            self.match_system,
            self.dark,
            t,
            self.editor_font,
            self.typography,
            self.line_length,
            self.focus_slide,
            self.fade_tasks,
            self.strike_tasks,
            self.three_returns,
            self.smart_quotes,
            self.smart_dashes,
            self.responsive,
            self.aspect,
            self.fit_text,
            self.focus_note,
            self.custom_keys,
            self.prev_key,
            self.next_key,
            self.warn_empty,
            self.wpm,
            self.show_speaking_time,
            self.bullet,
            self.ordered_paren,
            self.table_padded,
            self.italic_underscore,
            self.bold_underscore,
            self.single_return,
            self.smart_punct,
            self.sidebar_width.round(),
            self.check_updates,
            self.last_update_check,
            self.skipped_version.replace(['\n', '\r'], ""),
            self.video_autoplay,
            self.video_controls,
        );
        let _ = std::fs::write(p, out);
    }

    pub fn bullet_str(&self) -> &'static str {
        ["- ", "* ", "+ "][self.bullet.min(2)]
    }

    pub fn ordered_str(&self) -> &'static str {
        if self.ordered_paren {
            "1) "
        } else {
            "1. "
        }
    }

    /// Thumbnails, preview and exports use this shape.
    pub fn slide_aspect(&self) -> f32 {
        if self.responsive {
            ASPECTS[0].1
        } else {
            ASPECTS[self.aspect].1
        }
    }

    pub fn key(&self, idx: usize) -> egui::Key {
        use egui::Key::*;
        [
            ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Space, PageUp, PageDown,
        ][idx.min(6)]
    }
}

/// Applies the window appearance; returns whether the dark look is used.
pub fn apply_processing(s: &Settings) {
    SMART_PUNCT.store(s.smart_punct, Ordering::Relaxed);
    FIT_TEXT.store(s.fit_text, Ordering::Relaxed);
    SINGLE_RETURN.store(s.single_return, Ordering::Relaxed);
}

pub fn apply_appearance(ctx: &egui::Context, s: &Settings) -> bool {
    let dark = if s.match_system {
        ctx.input(|i| i.raw.system_theme)
            .map(|t| t == egui::Theme::Dark)
            .unwrap_or(s.dark)
    } else {
        s.dark
    };
    let was = DARK_UI.swap(dark, Ordering::Relaxed);
    SMART_PUNCT.store(s.smart_punct, Ordering::Relaxed);
    FIT_TEXT.store(s.fit_text, Ordering::Relaxed);
    SINGLE_RETURN.store(s.single_return, Ordering::Relaxed);
    if was != dark || ctx.cumulative_pass_nr() == 0 {
        set_visuals(ctx, dark);
    }
    dark
}

pub fn set_visuals(ctx: &egui::Context, dark: bool) {
    let p = if dark { &DARK_PAL } else { &LIGHT_PAL };
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.editor_bg;
    v.window_fill = if dark { hex(0x262626) } else { hex(0xFFFFFF) };
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.extreme_bg_color = p.field;
    v.faint_bg_color = p.sidebar_bg;
    v.code_bg_color = p.control;
    v.selection.bg_fill = if dark { hex(0x2D4A66) } else { hex(0xC5DCFA) };
    v.selection.stroke = Stroke::new(1.0_f32, crate::ACCENT_BLUE);
    v.text_cursor.stroke = Stroke::new(2.0_f32, hex(0x72B8F3));
    v.hyperlink_color = crate::ACCENT_BLUE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.divider);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_hi);
    // Light mode needs hairlines so controls stand out from the pale panels.
    let edge = if dark {
        Stroke::NONE
    } else {
        Stroke::new(1.0_f32, p.border)
    };
    v.widgets.inactive.weak_bg_fill = p.button;
    v.widgets.inactive.bg_fill = p.button;
    v.widgets.inactive.bg_stroke = edge;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_hi);
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = edge;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text_hi);
    v.widgets.hovered.expansion = 0.0;
    v.widgets.active.weak_bg_fill = p.hover;
    v.widgets.active.bg_fill = p.hover;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, crate::ACCENT_BLUE);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.text_hi);
    v.widgets.active.expansion = 0.0;
    v.widgets.open.weak_bg_fill = p.hover;
    v.widgets.open.bg_fill = p.hover;
    v.widgets.open.bg_stroke = edge;
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.text_hi);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(6);
    }
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.menu_corner_radius = egui::CornerRadius::same(8);
    let shadow_alpha = if dark { 110 } else { 40 };
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    v.striped = false;
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    ctx.set_visuals_of(theme, v);
    ctx.style_mut_of(theme, |s| {
        s.spacing.button_padding = vec2(10.0, 4.0);
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.interact_size.y = 24.0;
        // Labels behave like native UI text: clicking them doesn't select words.
        s.interaction.selectable_labels = false;
        s.spacing.icon_width = 16.0;
        s.spacing.icon_width_inner = 9.0;
        s.spacing.icon_spacing = 6.0;
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.window_margin = egui::Margin::same(12);
        s.spacing.combo_height = 320.0;
        // Thin scroll bars that float over content and take no layout width.
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.scroll.floating_width = 6.0;
        s.text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(13.5));
        s.text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.5));
        s.text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(11.5));
    });
}

// ---------- the Settings window ----------

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    General,
    Editor,
    Presentation,
    Themes,
    Markdown,
}

fn page_icon(p: &egui::Painter, c: egui::Pos2, page: Page, col: Color32) {
    let s = Stroke::new(1.8_f32, col);
    match page {
        Page::General => {
            p.circle_stroke(c, 9.0, s);
            p.circle_stroke(c, 3.5, s);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 9.0, c + d * 12.5], Stroke::new(2.6_f32, col));
            }
        }
        Page::Editor => {
            let r = Rect::from_center_size(c, vec2(20.0, 22.0));
            p.rect_stroke(r, 3.0, s, egui::StrokeKind::Inside);
            for k in 0..3 {
                let y = r.top() + 7.0 + k as f32 * 4.5;
                p.line_segment(
                    [pos2(r.left() + 4.0, y), pos2(r.right() - 4.0, y)],
                    Stroke::new(1.4_f32, col),
                );
            }
        }
        Page::Presentation => {
            let r = Rect::from_center_size(c, vec2(24.0, 18.0));
            p.rect_stroke(r, 2.5, s, egui::StrokeKind::Inside);
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(-3.5, -5.0),
                    c + vec2(5.5, 0.0),
                    c + vec2(-3.5, 5.0),
                ],
                col,
                Stroke::NONE,
            ));
        }
        Page::Themes => {
            let r = Rect::from_center_size(c, vec2(18.0, 23.0));
            p.rect_stroke(r, 2.5, s, egui::StrokeKind::Inside);
            p.text(
                pos2(r.left() + 6.0, r.top() + 6.5),
                egui::Align2::CENTER_CENTER,
                "A",
                FontId::proportional(8.5),
                col,
            );
            p.rect_filled(
                Rect::from_min_max(
                    pos2(r.left() + 3.0, r.top() + 12.0),
                    pos2(r.right() - 3.0, r.bottom() - 3.0),
                ),
                1.0,
                col,
            );
        }
        Page::Markdown => {
            let r = Rect::from_center_size(c, vec2(28.0, 18.0));
            p.rect_stroke(r, 2.5, s, egui::StrokeKind::Inside);
            p.text(
                c + vec2(-4.5, 0.5),
                egui::Align2::CENTER_CENTER,
                "M",
                FontId::new(11.0, crate::theme::family(false, true, false)),
                col,
            );
            p.line_segment(
                [c + vec2(7.0, -4.5), c + vec2(7.0, 4.0)],
                Stroke::new(1.8_f32, col),
            );
            p.add(egui::Shape::convex_polygon(
                vec![c + vec2(3.5, 1.5), c + vec2(10.5, 1.5), c + vec2(7.0, 6.0)],
                col,
                Stroke::NONE,
            ));
        }
    }
}

// ---------- Settings components ----------
//
// Pages are built from grouped cards: each card holds rows with a label (and
// an optional one-line description) on the left and the control on the right.

/// Settings window background (cards sit on top in `field`).
fn window_fill() -> Color32 {
    pal().inspector_bg
}

/// Hairline between rows inside a card.
fn row_line() -> Color32 {
    if is_dark() {
        pal().border
    } else {
        pal().divider
    }
}

/// Small bold title above a card.
fn heading(ui: &mut egui::Ui, t: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 4.0, r.bottom() - 7.0),
        egui::Align2::LEFT_CENTER,
        t,
        FontId::new(12.0, crate::theme::family(false, true, false)),
        crate::inspector::text_mid(),
    );
    ui.add_space(4.0);
}

/// A rounded group of rows. Rows add their own vertical padding.
fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = pal();
    egui::Frame::new()
        .fill(p.field)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui);
        });
    ui.add_space(16.0);
}

/// Hairline between two rows of a card.
fn row_div(ui: &mut egui::Ui) {
    let r = ui.max_rect();
    let y = ui.cursor().top();
    ui.painter()
        .hline(r.left()..=r.right(), y, Stroke::new(1.0_f32, row_line()));
}

/// One settings row: title (and optional description) on the left, the
/// control right-aligned in a column `ctrl_w` wide. Returns the row's
/// response when `click_row` is set, so the whole row can act as the control.
fn row(
    ui: &mut egui::Ui,
    enabled: bool,
    title: &str,
    desc: Option<&str>,
    ctrl_w: f32,
    click_row: bool,
    add: impl FnOnce(&mut egui::Ui),
) -> Option<egui::Response> {
    let p = pal();
    let w = ui.available_width();
    let text_w = (w - ctrl_w - 20.0).max(120.0);
    let fg = if enabled { p.text_hi } else { p.text_dim };
    let tg = ui
        .painter()
        .layout(title.to_owned(), FontId::proportional(14.0), fg, text_w);
    let dg = desc.map(|d| {
        ui.painter()
            .layout(d.to_owned(), FontId::proportional(12.0), p.text_dim, text_w)
    });
    let text_h = tg.size().y + dg.as_ref().map_or(0.0, |g| 2.0 + g.size().y);
    let h = (text_h.max(26.0) + 20.0).max(46.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let resp = (click_row && enabled).then(|| {
        let resp = ui.interact(r, ui.scope_id().with(("settings-row", title)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp
    });
    let mut y = r.center().y - text_h / 2.0;
    let th = tg.size().y;
    ui.painter().galley(pos2(r.left(), y), tg, fg);
    y += th + 2.0;
    if let Some(g) = dg {
        ui.painter().galley(pos2(r.left(), y), g, p.text_dim);
    }
    let ctrl = Rect::from_min_max(pos2(r.right() - ctrl_w, r.top()), r.max);
    let mut cui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(ctrl)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    cui.spacing_mut().item_spacing = vec2(8.0, 6.0);
    if !enabled {
        cui.disable();
    }
    add(&mut cui);
    resp
}

/// A row whose control is an on/off switch; clicking anywhere on it toggles.
fn switch_row(ui: &mut egui::Ui, enabled: bool, title: &str, desc: Option<&str>, on: &mut bool) {
    let mut flip = false;
    let r = row(ui, enabled, title, desc, 60.0, true, |ui| {
        flip = switch(ui, *on, title).clicked();
    });
    if flip || r.is_some_and(|r| r.clicked()) {
        *on = !*on;
    }
}

/// macOS-style on/off switch: accent track when on, grey when off.
fn switch(ui: &mut egui::Ui, on: bool, label: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(34.0, 20.0), Sense::click());
    let enabled = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, enabled, on, label));
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, on, 0.12);
    let p = pal();
    let off = if is_dark() { p.tab_grey } else { p.border };
    let mut track = off.lerp_to_gamma(crate::inspector::MENU_ACCENT, t);
    let mut knob = Color32::WHITE;
    if !enabled {
        track = track.gamma_multiply(0.55);
        knob = knob.gamma_multiply(0.8);
    }
    let painter = ui.painter();
    painter.rect_filled(r, 10.0, track);
    let kx = egui::lerp((r.left() + 10.0)..=(r.right() - 10.0), t);
    let kc = pos2(kx, r.center().y);
    painter.circle_filled(kc + vec2(0.0, 0.8), 8.2, Color32::from_black_alpha(45));
    painter.circle_filled(kc, 8.0, knob);
    if resp.has_focus() {
        painter.rect_stroke(
            r.expand(2.0),
            12.0,
            Stroke::new(2.0_f32, crate::ACCENT_BLUE),
            egui::StrokeKind::Outside,
        );
    }
    resp
}

/// Segmented control for two to four exclusive choices.
fn seg(ui: &mut egui::Ui, value: &mut usize, labels: &[&str]) {
    let p = pal();
    let dark = is_dark();
    let enabled = ui.is_enabled();
    let font = FontId::proportional(12.5);
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| {
            let g = ui.painter().layout_no_wrap((*l).to_owned(), font.clone(), p.text_hi);
            (g.size().x + 24.0).max(60.0)
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let (r, resp) = ui.allocate_exact_size(vec2(total, 26.0), Sense::hover());
    let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.55) };
    ui.painter().rect_filled(r, 7.0, fade(p.control));
    let mut x = r.left() + 2.0;
    for (i, label) in labels.iter().enumerate() {
        let s = Rect::from_min_size(pos2(x, r.top() + 2.0), vec2(widths[i], r.height() - 4.0));
        x += widths[i];
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let sr = ui.interact(s, resp.id.with(i), sense);
        let active = *value == i;
        if active {
            ui.painter()
                .rect_filled(s, 5.0, fade(if dark { p.tab_grey } else { p.field }));
            ui.painter().rect_stroke(
                s,
                5.0,
                Stroke::new(1.0_f32, fade(p.border)),
                egui::StrokeKind::Inside,
            );
        } else if enabled && sr.hovered() {
            ui.painter().rect_filled(s, 5.0, p.hover);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let fg = if !enabled {
            p.text_dim
        } else if active {
            p.text_hi
        } else {
            p.icon
        };
        ui.painter()
            .text(s.center(), egui::Align2::CENTER_CENTER, *label, font.clone(), fg);
        if sr.clicked() {
            *value = i;
        }
    }
}

/// Two-choice segmented control for a bool (`false` is the first label).
fn seg_bool(ui: &mut egui::Ui, value: &mut bool, labels: [&str; 2]) {
    let mut i = *value as usize;
    seg(ui, &mut i, &labels);
    *value = i == 1;
}

/// Dropdown of names, `w` wide, with a checkmark on the current one.
fn dropdown(ui: &mut egui::Ui, id: &str, value: &mut usize, names: &[&str], w: f32) {
    let opts: Vec<(usize, &str)> = names.iter().copied().enumerate().collect();
    ui.allocate_ui_with_layout(
        vec2(w, 26.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(w);
            crate::inspector::menu_combo(ui, id, value, &opts);
        },
    );
}

/// Dim helper text under a control or below a card.
fn note(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).size(12.0).color(pal().text_dim));
}

/// Settings button: standard look, 28 high.
fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(label).min_size(vec2(0.0, 28.0)))
}

impl crate::App {
    pub fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let before = self.settings.clone();
        let title = match self.settings_page {
            Page::General => "General",
            Page::Editor => "Editor",
            Page::Presentation => "Presentation",
            Page::Themes => "Themes",
            Page::Markdown => "Markdown",
        };
        let dark = is_dark();
        let frame = egui::Frame::new()
            .fill(window_fill())
            .stroke(Stroke::new(1.0_f32, pal().border))
            .corner_radius(12)
            .shadow(ctx.global_style().visuals.window_shadow)
            .inner_margin(egui::Margin {
                left: 20,
                right: 20,
                top: 14,
                bottom: 18,
            });
        egui::Window::new("Settings")
            .id(egui::Id::unique("settings-window"))
            .title_bar(false)
            .frame(frame)
            .collapsible(false)
            .resizable(false)
            .default_width(640.0)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                ui.set_width(640.0);
                // header: page title and close button
                let (head, _) = ui.allocate_exact_size(vec2(640.0, 30.0), Sense::hover());
                ui.painter().text(
                    pos2(head.left(), head.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("Settings  ·  {title}"),
                    FontId::new(15.0, crate::theme::family(false, true, false)),
                    pal().text_hi,
                );
                let close = Rect::from_center_size(
                    pos2(head.right() - 12.0, head.center().y),
                    vec2(26.0, 26.0),
                );
                let c = ui
                    .interact(close, ui.scope_id().with("settings-close"), Sense::click())
                    .on_hover_text("Close (Esc)");
                if c.hovered() {
                    ui.painter()
                        .circle_filled(close.center(), 13.0, pal().hover);
                }
                let xs = Stroke::new(
                    1.6_f32,
                    if c.hovered() {
                        pal().text_hi
                    } else {
                        pal().icon
                    },
                );
                let cc = close.center();
                ui.painter()
                    .line_segment([cc + vec2(-5.0, -5.0), cc + vec2(5.0, 5.0)], xs);
                ui.painter()
                    .line_segment([cc + vec2(-5.0, 5.0), cc + vec2(5.0, -5.0)], xs);
                if c.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    open = false;
                }
                ui.add_space(8.0);
                // tab bar
                let pages = [
                    (Page::General, "General"),
                    (Page::Editor, "Editor"),
                    (Page::Presentation, "Presentation"),
                    (Page::Themes, "Themes"),
                    (Page::Markdown, "Markdown"),
                ];
                let (bar, _) = ui.allocate_exact_size(vec2(640.0, 58.0), Sense::hover());
                let w = 104.0;
                let x0 = bar.center().x - w * pages.len() as f32 / 2.0;
                for (k, (pg, name)) in pages.iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(x0 + w * k as f32 + 2.0, bar.top()),
                        vec2(w - 4.0, 56.0),
                    );
                    let resp = ui.interact(r, ui.scope_id().with(("settings-tab", k)), Sense::click());
                    let on = self.settings_page == *pg;
                    let p = ui.painter();
                    if on {
                        p.rect_filled(
                            r,
                            10.0,
                            crate::ACCENT_BLUE.gamma_multiply(if dark { 0.22 } else { 0.10 }),
                        );
                    } else if resp.hovered() {
                        p.rect_filled(r, 10.0, pal().hover.gamma_multiply(0.7));
                    }
                    let col = if on {
                        crate::ACCENT_BLUE
                    } else if resp.hovered() {
                        pal().text_hi
                    } else {
                        pal().icon
                    };
                    page_icon(p, pos2(r.center().x, r.top() + 21.0), *pg, col);
                    p.text(
                        pos2(r.center().x, r.bottom() - 11.0),
                        egui::Align2::CENTER_CENTER,
                        *name,
                        if on {
                            FontId::new(12.5, crate::theme::family(false, true, false))
                        } else {
                            FontId::proportional(12.5)
                        },
                        col,
                    );
                    if resp.clicked() {
                        self.settings_page = *pg;
                    }
                }
                ui.add_space(6.0);
                let y = ui.cursor().top();
                ui.painter().line_segment(
                    [pos2(bar.left(), y), pos2(bar.right(), y)],
                    Stroke::new(1.0_f32, pal().divider),
                );
                ui.add_space(14.0);
                // Every page gets the same height so the window doesn't jump
                // when switching tabs; taller pages scroll inside it.
                let page_h = (ctx.content_rect().height() - 220.0).clamp(240.0, 560.0);
                egui::ScrollArea::vertical()
                    .id_salt(("settings-page", self.settings_page as u8))
                    .max_height(page_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_height(page_h);
                        ui.set_width(640.0);
                        match self.settings_page {
                            Page::General => self.page_general(ui),
                            Page::Editor => self.page_editor(ui),
                            Page::Presentation => self.page_presentation(ui),
                            Page::Themes => self.page_themes(ui),
                            Page::Markdown => self.page_markdown(ui),
                        }
                    });
                ui.add_space(6.0);
            });
        if !open {
            self.settings_open = false;
        }
        if self.settings != before {
            self.settings.save();
            if self.settings.single_return != before.single_return {
                self.reparse();
            }
        }
    }

    fn page_general(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        heading(ui, "Appearance");
        card(ui, |ui| {
            row(ui, true, "Window appearance", None, 260.0, false, |ui| {
                let mut mode = if s.match_system { 0 } else if s.dark { 2 } else { 1 };
                seg(ui, &mut mode, &["System", "Light", "Dark"]);
                s.match_system = mode == 0;
                if mode != 0 {
                    s.dark = mode == 2;
                }
            });
        });
        heading(ui, "New Presentations");
        card(ui, |ui| {
            row(
                ui,
                true,
                "Starting text",
                Some("The Markdown every new presentation begins with."),
                300.0,
                false,
                |ui| {
                    if button(ui, "Clear").clicked() {
                        s.new_template.clear();
                    }
                    if button(ui, "Restore Default").clicked() {
                        s.new_template = crate::SAMPLE.to_string();
                    }
                },
            );
            let p = pal();
            egui::Frame::new()
                .fill(p.editor_bg)
                .stroke(Stroke::new(1.0_f32, p.border))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("new-template")
                        .max_height(170.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut s.new_template)
                                    .code_editor()
                                    .frame(egui::Frame::NONE)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(8),
                            );
                        });
                });
            ui.add_space(14.0);
        });
        let mut check_now = false;
        heading(ui, "Updates");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Check for updates automatically",
                None,
                &mut s.check_updates,
            );
            row_div(ui);
            let version = format!("You're using PresentCraft {}.", crate::update::CURRENT);
            let checking = self.update_check.is_some();
            row(ui, true, "Version", Some(&version), 160.0, false, |ui| {
                if ui
                    .add_enabled(!checking, egui::Button::new("Check Now").min_size(vec2(0.0, 28.0)))
                    .clicked()
                {
                    check_now = true;
                }
                if checking {
                    ui.spinner();
                }
            });
        });
        if check_now {
            self.check_for_updates(ui.ctx(), true);
        }
    }

    fn page_editor(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        heading(ui, "Text");
        card(ui, |ui| {
            row(ui, true, "Text size", None, 280.0, false, |ui| {
                ui.label(RichText::new("A").size(19.0).color(pal().text_hi));
                ui.allocate_ui_with_layout(
                    vec2(200.0, 28.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        crate::inspector::pill_slider(ui, &mut s.editor_font, 13.0..=26.0, 0);
                        s.editor_font = (s.editor_font * 2.0).round() / 2.0;
                    },
                );
                ui.label(RichText::new("A").size(12.0).color(pal().text_hi));
            });
            row_div(ui);
            row(ui, true, "Typography", None, 200.0, false, |ui| {
                dropdown(ui, "typography", &mut s.typography, &TYPOGRAPHY, 140.0);
            });
            row_div(ui);
            row(ui, true, "Line length limit", None, 240.0, false, |ui| {
                ui.label(RichText::new("characters").color(pal().text_dim));
                ui.add_sized(
                    [56.0, 26.0],
                    egui::DragValue::new(&mut s.line_length).range(40..=120),
                );
            });
        });
        heading(ui, "Writing");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Focus on current slide",
                Some("Dim every slide except the one you're editing."),
                &mut s.focus_slide,
            );
            row_div(ui);
            switch_row(
                ui,
                true,
                "Three returns start a new slide",
                None,
                &mut s.three_returns,
            );
        });
        heading(ui, "Completed Tasks");
        card(ui, |ui| {
            switch_row(ui, true, "Fade", None, &mut s.fade_tasks);
            row_div(ui);
            switch_row(ui, true, "Strikethrough", None, &mut s.strike_tasks);
        });
        heading(ui, "Smart Substitutions");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Quotes",
                Some("Replace straight quotes with curly quotes as you type."),
                &mut s.smart_quotes,
            );
            row_div(ui);
            switch_row(
                ui,
                true,
                "Dashes",
                Some("Replace two hyphens with a long dash as you type."),
                &mut s.smart_dashes,
            );
        });
    }

    fn page_presentation(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        heading(ui, "Layout");
        card(ui, |ui| {
            let desc = if s.responsive {
                "Slides adapt to the presentation screen."
            } else {
                "Slides keep one shape on every screen."
            };
            row(ui, true, "Slide layout", Some(desc), 240.0, false, |ui| {
                let mut fixed = !s.responsive;
                seg_bool(ui, &mut fixed, ["Responsive", "Fixed"]);
                s.responsive = !fixed;
            });
            row_div(ui);
            let names: Vec<&str> = ASPECTS.iter().map(|(n, _)| *n).collect();
            row(ui, !s.responsive, "Aspect ratio", None, 200.0, false, |ui| {
                dropdown(ui, "aspect", &mut s.aspect, &names, 140.0);
            });
            row_div(ui);
            switch_row(
                ui,
                true,
                "Reduce text size to fit in slide",
                None,
                &mut s.fit_text,
            );
        });
        heading(ui, "Presenting");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Focus on current speaker note",
                None,
                &mut s.focus_note,
            );
            row_div(ui);
            switch_row(
                ui,
                true,
                "Warn on empty slides with speaker notes",
                None,
                &mut s.warn_empty,
            );
        });
        heading(ui, "Content");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Auto play",
                Some("Videos start when their slide appears."),
                &mut s.video_autoplay,
            );
            row_div(ui);
            switch_row(
                ui,
                true,
                "Show controls",
                Some("Play button, time and progress bar on videos."),
                &mut s.video_controls,
            );
        });
        heading(ui, "Navigation");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Use custom navigation keys",
                None,
                &mut s.custom_keys,
            );
            row_div(ui);
            row(ui, s.custom_keys, "Previous slide", None, 200.0, false, |ui| {
                dropdown(ui, "prev-key", &mut s.prev_key, &KEYS, 140.0);
            });
            row_div(ui);
            row(ui, s.custom_keys, "Next slide", None, 200.0, false, |ui| {
                dropdown(ui, "next-key", &mut s.next_key, &KEYS, 140.0);
            });
        });
        heading(ui, "Speaking Time");
        card(ui, |ui| {
            row(
                ui,
                true,
                "Speaking rate",
                Some("Used to estimate how long each slide takes."),
                240.0,
                false,
                |ui| {
                    ui.label(RichText::new("words/minute").color(pal().text_dim));
                    ui.add_sized([56.0, 26.0], egui::DragValue::new(&mut s.wpm).range(40..=400));
                },
            );
            row_div(ui);
            switch_row(ui, true, "Show speaking time", None, &mut s.show_speaking_time);
        });
    }

    fn page_themes(&mut self, ui: &mut egui::Ui) {
        let builtin: Vec<usize> = (0..self.presets.len())
            .filter(|i| !self.presets[*i].custom)
            .collect();
        let custom: Vec<usize> = (0..self.presets.len())
            .filter(|i| self.presets[*i].custom)
            .collect();
        let mut chosen = None;
        self.theme_grid(ui, "Built-in", &builtin, &mut chosen);
        if !custom.is_empty() {
            ui.add_space(6.0);
            self.theme_grid(ui, "Your Themes", &custom, &mut chosen);
        }
        if let Some(i) = chosen {
            self.apply_preset(i);
        }
        ui.add_space(4.0);
        note(ui, "Pick a theme to use it. To make your own, change colours, fonts and size in the Design panel, then choose Duplicate Theme.");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if button(ui, "Duplicate Theme")
                .on_hover_text(
                    "Save a copy of the selected theme with its current colours, fonts and size",
                )
                .clicked()
            {
                let mut p = self.current_as_preset();
                let n = self.presets.iter().filter(|p| p.custom).count() + 1;
                p.name = Box::leak(format!("Custom {n}").into_boxed_str());
                p.custom = true;
                // A theme from a folder brings its stylesheet, fonts and pictures.
                let source = p.file.take();
                if let Some(parent) = crate::themefile::user_dir() {
                    let dir = crate::themefile::unique_dir(&parent, p.name);
                    match crate::themefile::save(&dir, &p, self.dark, source.as_deref()) {
                        Ok(()) => p.file = Some(dir),
                        Err(e) => self.status = format!("Couldn't save the theme: {e}"),
                    }
                }
                self.presets.push(p);
                self.preset_idx = self.presets.len() - 1;
                self.light_override = None;
                self.dark_override = None;
            }
            if button(ui, "Import Theme…")
                .on_hover_text("Add a theme folder with template.json and presets.json (iA Presenter themes work too)")
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Choose a theme folder")
                    .pick_folder()
                {
                    match crate::themefile::import(&dir) {
                        Ok(p) => {
                            self.presets.push(p);
                            self.apply_preset(self.presets.len() - 1);
                        }
                        Err(e) => self.status = format!("Couldn't import the theme: {e}"),
                    }
                }
            }
            if let Some(dir) = crate::themefile::user_dir() {
                if button(ui, "Show Themes Folder")
                    .on_hover_text(dir.display().to_string())
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&dir);
                    crate::open_external(dir.as_os_str());
                }
            }
            // Destructive action sits apart, on the right.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_remove = self.presets[self.preset_idx].custom;
                if ui
                    .add_enabled(
                        can_remove,
                        egui::Button::new(RichText::new("Delete Theme").color(if can_remove {
                            hex(0xD9483B)
                        } else {
                            pal().text_dim
                        }))
                        .min_size(vec2(0.0, 28.0)),
                    )
                    .on_hover_text("Delete the selected custom theme")
                    .on_disabled_hover_text("Built-in themes can't be deleted")
                    .clicked()
                {
                    let removed = self.presets.remove(self.preset_idx);
                    if let Some(dir) = removed.file {
                        crate::themefile::remove(&dir);
                    }
                    self.apply_preset(0);
                }
            });
        });
    }

    /// A titled grid of theme previews (cover slide drawn in each theme).
    fn theme_grid(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        items: &[usize],
        chosen: &mut Option<usize>,
    ) {
        heading(ui, title);
        let cols = 4;
        let gap = 14.0;
        let cw = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = cw / crate::render::ASPECT;
        for row in items.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    let (cell, resp) = ui.allocate_exact_size(vec2(cw, ch + 26.0), Sense::click());
                    let thumb = Rect::from_min_size(cell.min + vec2(0.0, 3.0), vec2(cw, ch));
                    let theme = self.theme_for(i, self.presets[i].starts_dark);
                    let painter = ui.painter_at(cell.expand(4.0));
                    self.paint_slide_with(&painter, thumb, 0, &theme);
                    let sel = i == self.preset_idx;
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    painter.rect_stroke(
                        thumb,
                        4.0,
                        Stroke::new(1.0_f32, pal().border),
                        egui::StrokeKind::Outside,
                    );
                    if sel {
                        painter.rect_stroke(
                            thumb.expand(2.5),
                            6.0,
                            Stroke::new(2.0_f32, crate::inspector::MENU_ACCENT),
                            egui::StrokeKind::Outside,
                        );
                    } else if resp.hovered() {
                        painter.rect_stroke(
                            thumb.expand(2.5),
                            6.0,
                            Stroke::new(2.0_f32, pal().hover),
                            egui::StrokeKind::Outside,
                        );
                    }
                    painter.text(
                        pos2(thumb.left() + 1.0, thumb.bottom() + 13.0),
                        egui::Align2::LEFT_CENTER,
                        self.presets[i].name,
                        if sel {
                            FontId::new(12.5, crate::theme::family(false, true, false))
                        } else {
                            FontId::proportional(12.5)
                        },
                        if sel {
                            pal().text_hi
                        } else {
                            crate::inspector::text_mid()
                        },
                    );
                    if resp.clicked() {
                        *chosen = Some(i);
                    }
                }
            });
            ui.add_space(8.0);
        }
    }

    fn page_markdown(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        heading(ui, "Lists");
        card(ui, |ui| {
            row(ui, true, "Bullet", Some("Used when PresentCraft writes a list."), 340.0, false, |ui| {
                seg(ui, &mut s.bullet, &["-  Hyphen", "*  Asterisk", "+  Plus"]);
            });
            row_div(ui);
            row(ui, true, "Numbered list", None, 260.0, false, |ui| {
                seg_bool(ui, &mut s.ordered_paren, ["1.  Period", "1)  Parenthesis"]);
            });
        });
        heading(ui, "Emphasis");
        card(ui, |ui| {
            row(ui, true, "Italic", None, 260.0, false, |ui| {
                seg_bool(ui, &mut s.italic_underscore, ["*Asterisks*", "_Underscores_"]);
            });
            row_div(ui);
            row(ui, true, "Bold", None, 260.0, false, |ui| {
                seg_bool(ui, &mut s.bold_underscore, ["**Asterisks**", "__Underscores__"]);
            });
        });
        heading(ui, "Tables");
        card(ui, |ui| {
            row(
                ui,
                true,
                "Columns",
                Some("Padded lines up the pipes so tables read well as text."),
                260.0,
                false,
                |ui| {
                    seg_bool(ui, &mut s.table_padded, ["Compact", "Padded"]);
                },
            );
        });
        heading(ui, "Slides");
        card(ui, |ui| {
            switch_row(
                ui,
                true,
                "Single return starts a new paragraph",
                Some("On: each line shown on a slide (tab in front) is its own paragraph. Off: lines join until an empty line, as in standard Markdown."),
                &mut s.single_return,
            );
            row_div(ui);
            switch_row(
                ui,
                true,
                "Apply smart punctuation",
                Some("Show straight quotes and plain dashes as curly quotes and long dashes on slides."),
                &mut s.smart_punct,
            );
        });
    }
}

/// Curly quotes and long dashes for display (outside `code`).
pub fn smarten(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut in_code = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            in_code = !in_code;
            out.push(c);
            i += 1;
            continue;
        }
        if in_code {
            out.push(c);
            i += 1;
            continue;
        }
        let prev_space = i == 0 || chars[i - 1].is_whitespace() || "([{".contains(chars[i - 1]);
        match c {
            '"' => out.push(if prev_space { '“' } else { '”' }),
            '\'' => out.push(if prev_space { '‘' } else { '’' }),
            '-' if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] == '-' => {
                out.push('—');
                i += 2;
            }
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                out.push('–');
                i += 1;
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn template_escaping_preserves_literal_sequences() {
        let template = "# Title\n\tC:\\notes\nLiteral \\n and \\t";
        let encoded = template
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
        assert_eq!(super::decode_template(&encoded), template);
    }
    #[test]
    fn typing_options_are_independent() {
        let mut settings = super::Settings::defaults("");
        settings.smart_quotes = true;
        assert_eq!(super::typing("\"Hello\" -- —", &settings), "“Hello” -- —");
        settings.smart_quotes = false;
        settings.smart_dashes = true;
        assert_eq!(
            super::typing("\"Hello\" -- ---", &settings),
            "\"Hello\" – —"
        );
        settings.responsive = false;
        settings.aspect = 2;
        assert_eq!(settings.slide_aspect(), 4.0 / 3.0);
    }

    #[test]
    fn smart() {
        assert_eq!(
            super::smarten("\"Hi\" -- it's `a--b`"),
            "“Hi” – it’s `a--b`"
        );
    }
}

/// Apply only the substitutions explicitly enabled for typing.
pub fn typing(text: &str, settings: &Settings) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let opening = i == 0 || chars[i - 1].is_whitespace() || "([{ ".contains(chars[i - 1]);
        if settings.smart_quotes && (c == '\"' || c == '\'') {
            out.push(match (c, opening) {
                ('\"', true) => '“',
                ('\"', false) => '”',
                (_, true) => '‘',
                _ => '’',
            });
        } else if settings.smart_dashes && c == '-' && chars.get(i + 1) == Some(&'-') {
            if chars.get(i + 2) == Some(&'-') {
                out.push('—');
                i += 2;
            } else {
                out.push('–');
                i += 1;
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn decode_template(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(c) => {
                    out.push('\\');
                    out.push(c);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
