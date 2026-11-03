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
            bullet: 0,                // markdownListBullet = 0 (hyphen)
            ordered_paren: false,     // markdownOrderedListDelimiter = 0 (period)
            table_padded: true,       // markdownTableFormatting = 1 (padded)
            italic_underscore: false, // markdownEmphasisKind = 0 (asterisks)
            bold_underscore: false,   // markdownStrongEmphasisKind = 0
            single_return: crate::mechanics::SINGLE_RETURN, // singleReturnStartsANewParagraph = true
            smart_punct: false,                             // smartPonctuation = false
            sidebar_width: crate::SIDEBAR_W,
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
            "match_system={}\ndark={}\nnew_template={}\neditor_font={}\ntypography={}\nline_length={}\nfocus_slide={}\nfade_tasks={}\nstrike_tasks={}\nthree_returns={}\nsmart_quotes={}\nsmart_dashes={}\nresponsive={}\naspect={}\nfit_text={}\nfocus_note={}\ncustom_keys={}\nprev_key={}\nnext_key={}\nwarn_empty={}\nwpm={}\nshow_speaking_time={}\nbullet={}\nordered_paren={}\ntable_padded={}\nitalic_underscore={}\nbold_underscore={}\nsingle_return={}\nsmart_punct={}\nsidebar_width={}\n",
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
            self.sidebar_width.round()
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
    v.selection.stroke = Stroke::new(1.0_f32, crate::FORMAT_BLUE);
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
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, crate::FORMAT_BLUE);
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

fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(190.0, 22.0), Sense::hover());
        if !label.is_empty() {
            ui.painter().text(
                pos2(r.right(), r.center().y),
                egui::Align2::RIGHT_CENTER,
                label,
                FontId::proportional(14.0),
                pal().text_hi,
            );
        }
        ui.add_space(6.0);
        ui.vertical(|ui| add(ui));
    });
}

fn note(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).size(12.0).color(pal().text_dim));
}

fn sep(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
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
            .fill(if dark {
                crate::theme::hex(0x262626)
            } else {
                Color32::WHITE
            })
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
            .id(egui::Id::new("settings-window"))
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
                    .interact(close, ui.id().with("settings-close"), Sense::click())
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
                    let resp = ui.interact(r, ui.id().with(("settings-tab", k)), Sense::click());
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
        row(ui, "Appearance:", |ui| {
            ui.checkbox(&mut s.match_system, "Match system appearance");
            ui.add_enabled_ui(!s.match_system, |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.dark, false, "Light");
                    ui.radio_value(&mut s.dark, true, "Dark");
                });
            });
        });
        sep(ui);
        row(ui, "New presentations:", |ui| {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut s.new_template)
                            .code_editor()
                            .desired_width(360.0)
                            .desired_rows(8),
                    );
                });
            ui.horizontal(|ui| {
                if ui.button("Restore Default").clicked() {
                    s.new_template = crate::SAMPLE.to_string();
                }
                if ui.button("Clear").clicked() {
                    s.new_template.clear();
                }
            });
        });
    }

    fn page_editor(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        ui.horizontal(|ui| {
            ui.add_space(150.0);
            ui.label(RichText::new("A").size(13.0));
            ui.spacing_mut().slider_width = 240.0;
            ui.add(
                egui::Slider::new(&mut s.editor_font, 13.0..=26.0)
                    .step_by(0.5)
                    .show_value(false),
            );
            ui.label(RichText::new("A").size(22.0));
        });
        sep(ui);
        row(ui, "Typography:", |ui| {
            egui::ComboBox::from_id_salt("typography")
                .selected_text(TYPOGRAPHY[s.typography])
                .show_ui(ui, |ui| {
                    for (i, n) in TYPOGRAPHY.iter().enumerate() {
                        ui.selectable_value(&mut s.typography, i, *n);
                    }
                });
        });
        row(ui, "Line length limit:", |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut s.line_length).range(40..=120));
                ui.label("characters per line");
            });
        });
        sep(ui);
        row(ui, "Focus:", |ui| {
            ui.checkbox(&mut s.focus_slide, "Focus on current slide");
        });
        sep(ui);
        row(ui, "Completed tasks:", |ui| {
            ui.checkbox(&mut s.fade_tasks, "Fade");
            ui.checkbox(&mut s.strike_tasks, "Strikethrough");
        });
        sep(ui);
        row(ui, "Typing:", |ui| {
            ui.checkbox(&mut s.three_returns, "Three returns to start a new slide");
        });
        sep(ui);
        row(ui, "Smart substitutions:", |ui| {
            ui.checkbox(&mut s.smart_quotes, "Quotes");
            ui.checkbox(&mut s.smart_dashes, "Dashes");
        });
    }

    fn page_presentation(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        row(ui, "Layout:", |ui| {
            ui.radio_value(&mut s.responsive, true, "Responsive");
            note(ui, "Automatically adapt slides to the presentation screen.");
            ui.horizontal(|ui| {
                ui.radio_value(&mut s.responsive, false, "Fixed aspect ratio");
                ui.add_enabled_ui(!s.responsive, |ui| {
                    egui::ComboBox::from_id_salt("aspect")
                        .selected_text(ASPECTS[s.aspect].0)
                        .show_ui(ui, |ui| {
                            for (i, (n, _)) in ASPECTS.iter().enumerate() {
                                ui.selectable_value(&mut s.aspect, i, *n);
                            }
                        });
                });
            });
        });
        sep(ui);
        row(ui, "Content:", |ui| {
            ui.checkbox(&mut s.fit_text, "Reduce text size to fit in slide");
        });
        sep(ui);
        row(ui, "Focus:", |ui| {
            ui.checkbox(&mut s.focus_note, "Focus on current speaker note");
        });
        sep(ui);
        row(ui, "Navigation:", |ui| {
            ui.checkbox(&mut s.custom_keys, "Use custom navigation keys");
            ui.add_enabled_ui(s.custom_keys, |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized([100.0, 22.0], egui::Label::new("Previous slide"));
                    egui::ComboBox::from_id_salt("prev-key")
                        .width(100.0)
                        .selected_text(KEYS[s.prev_key])
                        .show_ui(ui, |ui| {
                            for (i, n) in KEYS.iter().enumerate() {
                                ui.selectable_value(&mut s.prev_key, i, *n);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.add_sized([100.0, 22.0], egui::Label::new("Next slide"));
                    egui::ComboBox::from_id_salt("next-key")
                        .width(100.0)
                        .selected_text(KEYS[s.next_key])
                        .show_ui(ui, |ui| {
                            for (i, n) in KEYS.iter().enumerate() {
                                ui.selectable_value(&mut s.next_key, i, *n);
                            }
                        });
                });
            });
        });
        sep(ui);
        row(ui, "Warnings:", |ui| {
            ui.checkbox(&mut s.warn_empty, "Warn on empty slides with speaker notes");
        });
        sep(ui);
        row(ui, "Speaking Rate:", |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut s.wpm).range(40..=400));
                ui.label("words/minute");
            });
            ui.checkbox(&mut s.show_speaking_time, "Show speaking time");
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
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Duplicate Theme").min_size(vec2(130.0, 28.0)))
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
            if ui
                .add(egui::Button::new("Import Theme…").min_size(vec2(120.0, 28.0)))
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
            let can_remove = self.presets[self.preset_idx].custom;
            if ui
                .add_enabled(
                    can_remove,
                    egui::Button::new("Delete").min_size(vec2(80.0, 28.0)),
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
            if let Some(dir) = crate::themefile::user_dir() {
                if ui
                    .button("Show Themes Folder")
                    .on_hover_text(dir.display().to_string())
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&dir);
                    crate::open_external(dir.as_os_str());
                }
            }
        });
        ui.add_space(6.0);
        note(ui, "Pick a theme to use it. To make your own, change colours, fonts and size in the Design panel, then choose Duplicate Theme. Themes are saved as folders with template.json and presets.json, the same layout iA Presenter uses.");
    }

    /// A titled grid of theme previews (cover slide drawn in each theme).
    fn theme_grid(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        items: &[usize],
        chosen: &mut Option<usize>,
    ) {
        ui.label(
            RichText::new(title)
                .size(12.0)
                .strong()
                .color(pal().text_dim),
        );
        ui.add_space(4.0);
        let cols = 4;
        let gap = 12.0;
        let cw = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = cw / crate::render::ASPECT;
        for row in items.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    let (cell, resp) = ui.allocate_exact_size(vec2(cw, ch + 24.0), Sense::click());
                    let thumb = Rect::from_min_size(cell.min + vec2(0.0, 2.0), vec2(cw, ch));
                    let theme = self.theme_for(i, self.presets[i].starts_dark);
                    let painter = ui.painter_at(cell.expand(4.0));
                    self.paint_slide_with(&painter, thumb, 0, &theme);
                    let sel = i == self.preset_idx;
                    painter.rect_stroke(
                        thumb,
                        3.0,
                        Stroke::new(1.0_f32, pal().border),
                        egui::StrokeKind::Outside,
                    );
                    if sel {
                        painter.rect_stroke(
                            thumb.expand(2.0),
                            5.0,
                            Stroke::new(2.0_f32, crate::ACCENT_BLUE),
                            egui::StrokeKind::Outside,
                        );
                    } else if resp.hovered() {
                        painter.rect_stroke(
                            thumb.expand(2.0),
                            5.0,
                            Stroke::new(2.0_f32, pal().hover),
                            egui::StrokeKind::Outside,
                        );
                    }
                    painter.text(
                        pos2(thumb.left() + 1.0, thumb.bottom() + 12.0),
                        egui::Align2::LEFT_CENTER,
                        self.presets[i].name,
                        if sel {
                            FontId::new(12.5, crate::theme::family(false, true, false))
                        } else {
                            FontId::proportional(12.5)
                        },
                        if sel { pal().text_hi } else { pal().text_dim },
                    );
                    if resp.clicked() {
                        *chosen = Some(i);
                    }
                }
            });
            ui.add_space(6.0);
        }
    }

    fn page_markdown(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(220.0);
            ui.selectable_value(&mut self.markdown_processing, false, "Editing");
            ui.selectable_value(&mut self.markdown_processing, true, "Processing");
        });
        ui.add_space(8.0);
        let s = &mut self.settings;
        if !self.markdown_processing {
            row(ui, "List bullet:", |ui| {
                ui.radio_value(&mut s.bullet, 0, "Hyphen");
                ui.radio_value(&mut s.bullet, 1, "Asterisk");
                ui.radio_value(&mut s.bullet, 2, "Plus");
            });
            row(ui, "Ordered list delimiter:", |ui| {
                ui.radio_value(&mut s.ordered_paren, false, "Period");
                ui.radio_value(&mut s.ordered_paren, true, "Parenthesis");
            });
            sep(ui);
            row(ui, "Table columns:", |ui| {
                ui.radio_value(&mut s.table_padded, false, "Compact");
                ui.radio_value(&mut s.table_padded, true, "Padded");
            });
            sep(ui);
            row(ui, "Italic:", |ui| {
                ui.radio_value(&mut s.italic_underscore, false, "Asterisks");
                ui.radio_value(&mut s.italic_underscore, true, "Underscores");
            });
            row(ui, "Bold:", |ui| {
                ui.radio_value(&mut s.bold_underscore, false, "Double asterisks");
                ui.radio_value(&mut s.bold_underscore, true, "Double underscores");
            });
        } else {
            row(ui, "", |ui| {
                ui.checkbox(&mut s.single_return, "Single return starts a new paragraph");
                note(ui, "When on, each line you show on a slide (tab in front) is its own paragraph.\nWhen off, lines join until an empty line, as in standard Markdown.");
                ui.add_space(8.0);
                ui.checkbox(&mut s.smart_punct, "Apply smart punctuation");
                note(ui, "Show straight quotes and plain dashes as curly quotes and long dashes on slides.");
            });
        }
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
