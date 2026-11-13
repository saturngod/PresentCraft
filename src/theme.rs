//! Slide themes and fonts, with light and dark colour pairs.

use egui::{Color32, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// The theme actually used to draw slides (preset + person's choices).
#[derive(Clone)]
pub struct Theme {
    pub bg: Color32,
    pub body: Color32,
    pub title: Color32,
    pub accents: [Color32; 6],
    /// Chart series colours; empty means the accents.
    pub chart_colors: Vec<Color32>,
    pub face: crate::fonts::Face,
    pub title_face: crate::fonts::Face,
    /// Cover and title slides use the opposite appearance.
    pub invert_titles: bool,
    /// The other appearance's colours, for inverted slides.
    pub inverse: Palette,
    pub size_scale: f32,
    /// Multiplies slide text line spacing (1.0 is the default).
    pub line_scale: f32,
    /// Header and footer slots (see [`SLOT_NAMES`] for the order).
    pub slots: [Slot; 6],
    /// Today's date, shown by `SlotKind::Date` slots.
    pub date: String,
    /// The deck's one logo, drawn in every slot set to `SlotKind::Logo`.
    pub logo: String,
    /// Logo height as a fraction of slide height.
    pub logo_size: f32,
    pub decor: Decor,
}

/// What a header or footer slot shows.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum SlotKind {
    #[default]
    Empty,
    Number,
    Date,
    Logo,
    Custom,
}

impl SlotKind {
    pub const ALL: [SlotKind; 5] = [
        SlotKind::Empty,
        SlotKind::Number,
        SlotKind::Date,
        SlotKind::Logo,
        SlotKind::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SlotKind::Empty => "Empty",
            SlotKind::Number => "Slide N°",
            SlotKind::Date => "Date",
            SlotKind::Logo => "Logo",
            SlotKind::Custom => "Custom",
        }
    }
}

/// One header or footer place and its content.
#[derive(Clone, PartialEq, Debug)]
pub struct Slot {
    pub kind: SlotKind,
    /// Text for `SlotKind::Custom`.
    pub text: String,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            kind: SlotKind::Empty,
            text: String::new(),
        }
    }
}

/// Slot order: top row leading, middle, trailing; then the bottom row.
pub const SLOT_NAMES: [&str; 6] = [
    "Leading Header",
    "Middle Header",
    "Trailing Header",
    "Leading Footer",
    "Middle Footer",
    "Trailing Footer",
];

impl Slot {
    /// The text this slot shows on slide `number`, or "" when empty.
    pub fn resolve(&self, number: usize, date: &str) -> String {
        match self.kind {
            SlotKind::Empty | SlotKind::Logo => String::new(),
            SlotKind::Number => number.to_string(),
            SlotKind::Date => date.to_string(),
            SlotKind::Custom => self.text.trim().to_string(),
        }
    }
}

/// Today's local date as DD/MM/YYYY (UTC where local time is unavailable).
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_date(secs + local_offset(secs));
    format!("{d:02}/{m:02}/{y:04}")
}

/// Seconds east of UTC for the local time zone at `secs`.
#[cfg(unix)]
fn local_offset(secs: i64) -> i64 {
    // Layout of `struct tm` on macOS, Linux and the BSDs.
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        mday: i32,
        mon: i32,
        year: i32,
        wday: i32,
        yday: i32,
        isdst: i32,
        gmtoff: std::os::raw::c_long,
        zone: *const std::os::raw::c_char,
    }
    extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm {
        sec: 0,
        min: 0,
        hour: 0,
        mday: 0,
        mon: 0,
        year: 0,
        wday: 0,
        yday: 0,
        isdst: 0,
        gmtoff: 0,
        zone: std::ptr::null(),
    };
    // SAFETY: both pointers are valid for the call; localtime_r is thread-safe.
    let ok = unsafe { !localtime_r(&secs, &mut tm).is_null() };
    if ok {
        tm.gmtoff as i64
    } else {
        0
    }
}

#[cfg(not(unix))]
fn local_offset(_secs: i64) -> i64 {
    0
}

/// (year, month, day) for a Unix time in seconds (Howard Hinnant's algorithm).
fn civil_date(secs: i64) -> (i64, u32, u32) {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod slot_tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29));
        assert_eq!(civil_date(1_793_577_600), (2026, 11, 2));
    }

    #[test]
    fn slot_text() {
        let mut s = Slot::default();
        assert_eq!(s.resolve(3, "02/11/2026"), "");
        s.kind = SlotKind::Number;
        assert_eq!(s.resolve(3, "02/11/2026"), "3");
        s.kind = SlotKind::Date;
        assert_eq!(s.resolve(3, "02/11/2026"), "02/11/2026");
        s.kind = SlotKind::Logo;
        assert_eq!(s.resolve(3, "02/11/2026"), "");
        s.kind = SlotKind::Custom;
        s.text = " Saturngod ".into();
        assert_eq!(s.resolve(3, "02/11/2026"), "Saturngod");
    }
}

/// Visual styling a theme adds around the text. Decorations sit in the
/// slide margins or behind the text, so layouts are unchanged.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Decor {
    /// Flat background only.
    None,
    /// Accent bar in the top margin; accent stripe down the cover's left edge.
    Bar,
    /// Soft vertical gradient with a glow on cover and title slides.
    Glow,
    /// Thin inset frame and a short accent rule, like a printed page.
    Frame,
    /// Accent band across the top of every slide.
    Band,
    /// Gradient with large soft circles in the corners.
    Shapes,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub title: Color32,
    pub body: Color32,
}

#[derive(Clone)]
pub struct Preset {
    pub name: &'static str,
    pub custom: bool,
    /// The theme folder this came from, for the person's saved themes.
    pub file: Option<std::path::PathBuf>,
    /// Background gradient stops for light and dark slides (empty: none).
    pub gradients: [Vec<Color32>; 2],
    /// Chart series colours for light and dark slides (empty: accents).
    pub chart_colors: [Vec<Color32>; 2],
    /// First accent for light and dark slides, when the theme sets one
    /// per look (iA Presenter's `DarkAccent1` / `LightAccent1`).
    pub accent1: [Option<Color32>; 2],
    /// Accents for dark slides when they differ from `accents` (iA
    /// Presenter's Helvetica and Garamond use greys there).
    pub dark_accents: Option<[Color32; 6]>,
    pub title_face: crate::fonts::Face,
    pub body_face: crate::fonts::Face,
    /// Index into the shared slide text sizes.
    pub size_idx: usize,
    /// Index into the shared line heights.
    pub line_idx: usize,
    pub invert_titles: bool,
    pub light: Palette,
    pub dark: Palette,
    pub starts_dark: bool,
    pub accents: [Color32; 6],
    pub decor: Decor,
}

pub const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Name of the theme new presentations start with.
pub const DEFAULT_THEME: &str = "Yangon";

/// The built-in themes: Southeast Asian cities, in menu order. Each is an
/// iA Presenter theme folder in `themes/` (template.json, presets.json,
/// stylesheet, preview and its Google Fonts), built into the app.
pub const BUILT_IN_ORDER: [&str; 24] = [
    "Yangon",
    "Mandalay",
    "Bagan",
    "Taunggyi",
    "Singapore",
    "Bangkok",
    "Chiang Mai",
    "Hanoi",
    "Hoi An",
    "Da Nang",
    "Kuala Lumpur",
    "Penang",
    "Malacca",
    "Siem Reap",
    "Phnom Penh",
    "Luang Prabang",
    "Vientiane",
    "Bali",
    "Jakarta",
    "Yogyakarta",
    "Manila",
    "Cebu",
    "Bandar Seri Begawan",
    "Dili",
];

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/built_in_themes.rs"));
}

/// Writes the built-in theme folders where the app can read their fonts:
/// "Built-in Themes" next to the settings file. Files are rewritten only
/// when they differ from the ones built into the app.
fn built_in_root() -> Option<std::path::PathBuf> {
    let root = if cfg!(test) {
        std::env::temp_dir().join(format!("presentcraft-built-in-{}", std::process::id()))
    } else {
        crate::settings::Settings::app_dir()?.join("Built-in Themes")
    };
    for (rel, bytes) in embedded::BUILT_IN_FILES {
        let path = root.join(rel);
        if std::fs::read(&path).ok().as_deref() == Some(*bytes) {
            continue;
        }
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(&path, bytes).ok()?;
    }
    Some(root)
}

fn embedded_text(rel: &str) -> Option<crate::json::Json> {
    embedded::BUILT_IN_FILES
        .iter()
        .find(|(p, _)| *p == rel)
        .and_then(|(_, b)| std::str::from_utf8(b).ok())
        .and_then(|t| crate::json::Json::parse(t).ok())
}

pub fn presets() -> Vec<Preset> {
    static CACHE: std::sync::OnceLock<Vec<Preset>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let root = built_in_root();
            BUILT_IN_ORDER
                .iter()
                .filter_map(|name| {
                    // From the written folder (with fonts); else from memory.
                    let mut p = root
                        .as_ref()
                        .and_then(|r| crate::themefile::load(&r.join(name)).ok())
                        .or_else(|| {
                            let t = embedded_text(&format!("{name}/template.json"))?;
                            let ps = embedded_text(&format!("{name}/presets.json"))?;
                            Some(crate::themefile::from_json(&t, &ps, std::path::Path::new("")))
                        })?;
                    p.custom = false;
                    Some(p)
                })
                .collect()
        })
        .clone()
}

pub fn family(face: impl Into<crate::fonts::Face>, bold: bool, italic: bool) -> FontFamily {
    let base = face.into().base();
    let suffix = match (bold, italic) {
        (false, false) => "",
        (true, false) => "-b",
        (false, true) => "-i",
        (true, true) => "-bi",
    };
    FontFamily::Name(format!("{base}{suffix}").into())
}

pub fn mono(bold: bool) -> FontFamily {
    FontFamily::Name(if bold { "mono-b".into() } else { "mono".into() })
}

/// Registers the font families. System fonts are used when present,
/// otherwise egui's built-in fonts stand in.
/// The Windows fonts folder (with trailing separator), or an empty-ish path elsewhere.
fn windows_fonts_dir() -> String {
    let root = std::env::var("WINDIR")
        .or_else(|_| std::env::var("SystemRoot"))
        .unwrap_or_else(|_| "C:\\Windows".to_string());
    format!("{root}\\Fonts\\")
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    // Math symbols (arrows, set and logic signs) that text fonts often lack.
    let symbols = [
        "/System/Library/Fonts/Supplemental/STIXGeneral.otf".to_string(),
        format!("{}seguisym.ttf", windows_fonts_dir()),
        format!("{}cambria.ttc", windows_fonts_dir()),
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/TTF/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf".to_string(),
    ];
    let mut fallback = fallback;
    if let Some(bytes) = symbols.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts
            .font_data
            .insert("math-symbols".into(), Arc::new(FontData::from_owned(bytes)));
        fallback.insert(0, "math-symbols".into());
    }
    let mono_fallback = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    // Each face lists candidate files for macOS, Windows and Linux; the first
    // one found is used, otherwise egui's built-in fonts are the fallback.
    let mac = "/System/Library/Fonts/Supplemental/";
    let win = windows_fonts_dir();
    let w = |f: &str| format!("{win}{f}");
    let lx = |f: &str| f.to_string();
    let sets: Vec<(&str, Vec<(String, u32)>, bool)> = vec![
        (
            "sans",
            vec![
                (format!("{mac}Arial.ttf"), 0),
                (w("arial.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSans-Regular.ttf"), 0),
                (lx("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"), 0),
                (lx("/usr/share/fonts/TTF/DejaVuSans.ttf"), 0),
            ],
            false,
        ),
        (
            "sans-b",
            vec![
                (format!("{mac}Arial Bold.ttf"), 0),
                (w("arialbd.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSans-Bold.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"), 0),
            ],
            false,
        ),
        (
            "sans-i",
            vec![
                (format!("{mac}Arial Italic.ttf"), 0),
                (w("ariali.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSans-Italic.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-sans/LiberationSans-Italic.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSans-Italic.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/dejavu/DejaVuSans-Oblique.ttf"),
                    0,
                ),
            ],
            false,
        ),
        (
            "sans-bi",
            vec![
                (format!("{mac}Arial Bold Italic.ttf"), 0),
                (w("arialbi.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-sans/LiberationSans-BoldItalic.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSans-BoldItalic.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf"),
                    0,
                ),
            ],
            false,
        ),
        (
            "serif",
            vec![
                (format!("{mac}Georgia.ttf"), 0),
                (w("georgia.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-serif/LiberationSerif-Regular.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSerif-Regular.ttf"), 0),
                (lx("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"), 0),
            ],
            false,
        ),
        (
            "serif-b",
            vec![
                (format!("{mac}Georgia Bold.ttf"), 0),
                (w("georgiab.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-serif/LiberationSerif-Bold.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSerif-Bold.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
                    0,
                ),
            ],
            false,
        ),
        (
            "serif-i",
            vec![
                (format!("{mac}Georgia Italic.ttf"), 0),
                (w("georgiai.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSerif-Italic.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-serif/LiberationSerif-Italic.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSerif-Italic.ttf"), 0),
            ],
            false,
        ),
        (
            "serif-bi",
            vec![
                (format!("{mac}Georgia Bold Italic.ttf"), 0),
                (w("georgiaz.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationSerif-BoldItalic.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/liberation-serif/LiberationSerif-BoldItalic.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/LiberationSerif-BoldItalic.ttf"), 0),
            ],
            false,
        ),
        (
            "mono",
            vec![
                (lx("/System/Library/Fonts/Menlo.ttc"), 0),
                (w("consola.ttf"), 0),
                (lx("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"), 0),
                (lx("/usr/share/fonts/TTF/DejaVuSansMono.ttf"), 0),
                (
                    lx("/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf"),
                    0,
                ),
            ],
            true,
        ),
        (
            "mono-b",
            vec![
                (lx("/System/Library/Fonts/Menlo.ttc"), 1),
                (w("consolab.ttf"), 0),
                (
                    lx("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"),
                    0,
                ),
                (lx("/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf"), 0),
                (
                    lx("/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono-Bold.ttf"),
                    0,
                ),
                (
                    lx("/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"),
                    0,
                ),
            ],
            true,
        ),
    ];
    for (name, candidates, is_mono) in sets {
        let mut list = Vec::new();
        let found = candidates
            .iter()
            .find_map(|(path, index)| std::fs::read(path).ok().map(|b| (b, *index)));
        if let Some((bytes, index)) = found {
            let mut fd = FontData::from_owned(bytes);
            fd.index = index;
            fonts.font_data.insert(name.to_string(), Arc::new(fd));
            list.push(name.to_string());
        }
        list.extend(
            if is_mono {
                mono_fallback.iter()
            } else {
                fallback.iter()
            }
            .cloned(),
        );
        fonts.families.insert(FontFamily::Name(name.into()), list);
    }
    // Families picked from the system fonts: "f<n>" plus -b, -i, -bi.
    for (i, fam) in crate::fonts::families_to_install().iter().enumerate() {
        for (style, file) in ["", "-b", "-i", "-bi"].iter().zip(fam.styles()) {
            let name = format!("f{i}{style}");
            let mut list = Vec::new();
            if let Some((path, index)) = file {
                if let Ok(bytes) = std::fs::read(path) {
                    let mut fd = FontData::from_owned(bytes);
                    fd.index = *index;
                    fonts.font_data.insert(name.clone(), Arc::new(fd));
                    list.push(name.clone());
                }
            }
            // Missing glyphs come from the built-in sans of the same style.
            let sans = format!("sans{style}");
            if let Some(fallback) = fonts.families.get(&FontFamily::Name(sans.into())) {
                list.extend(fallback.iter().cloned());
            }
            fonts.families.insert(FontFamily::Name(name.into()), list);
        }
    }
    // Symbols (⇧ ⌥ ☐ ❝ ✎ …) from the system symbol font, as a last fallback everywhere.
    let symbols = [
        "/System/Library/Fonts/Apple Symbols.ttf".to_string(),
        w("seguisym.ttf"),
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/TTF/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf".to_string(),
    ];
    if let Some(bytes) = symbols.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts
            .font_data
            .insert("symbols".to_string(), Arc::new(FontData::from_owned(bytes)));
        for fam in fonts.families.values_mut() {
            fam.push("symbols".to_string());
        }
    }
    // Other scripts (Myanmar, Thai, Indic, CJK, Arabic, …) from system fonts,
    // after everything else so existing glyphs keep their look.
    for (name, data) in script_fallbacks() {
        fonts.font_data.insert(name.clone(), data.clone());
        for fam in fonts.families.values_mut() {
            fam.push(name.clone());
        }
    }
    // UI text in a clean system sans.
    if fonts.font_data.contains_key("sans") {
        if let Some(p) = fonts.families.get_mut(&FontFamily::Proportional) {
            p.insert(0, "sans".to_string());
        }
    }
    crate::icons::install_fonts(&mut fonts);
    ctx.set_fonts(fonts);
}

/// System fonts for scripts the main fonts lack, one per script (first file
/// found wins). Read once and reused whenever fonts are reinstalled.
fn script_fallbacks() -> &'static [(String, Arc<FontData>)] {
    static CACHE: std::sync::OnceLock<Vec<(String, Arc<FontData>)>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let mac = "/System/Library/Fonts/";
        let win = windows_fonts_dir();
        let noto = [
            "/usr/share/fonts/truetype/noto/",
            "/usr/share/fonts/noto/",
            "/usr/share/fonts/google-noto/",
            "/usr/share/fonts/google-noto-vf/",
            "/usr/share/fonts/opentype/noto/",
            "/usr/share/fonts/noto-cjk/",
            "/usr/share/fonts/google-noto-cjk/",
        ];
        // Each script: macOS files, Windows files, then Linux Noto file names.
        let scripts: &[(&str, &[&str], &[&str], &[&str])] = &[
            (
                "myanmar",
                &["NotoSansMyanmar.ttc", "Supplemental/Myanmar Sangam MN.ttc"],
                &["mmrtext.ttf"],
                &[
                    "NotoSansMyanmar-Regular.ttf",
                    "NotoSansMyanmar-Regular.otf",
                    "NotoSansMyanmar[wdth,wght].ttf",
                ],
            ),
            (
                "thai",
                &["ThonburiUI.ttc", "Supplemental/Thonburi.ttc"],
                &["leelawui.ttf", "tahoma.ttf"],
                &["NotoSansThai-Regular.ttf", "NotoSansThai[wdth,wght].ttf"],
            ),
            (
                "lao",
                &["Supplemental/Lao Sangam MN.ttf"],
                &["leelawui.ttf"],
                &["NotoSansLao-Regular.ttf", "NotoSansLao[wdth,wght].ttf"],
            ),
            (
                "khmer",
                &["Supplemental/Khmer Sangam MN.ttf"],
                &["leelawui.ttf", "daunpenh.ttf"],
                &["NotoSansKhmer-Regular.ttf", "NotoSansKhmer[wdth,wght].ttf"],
            ),
            (
                "indic",
                &["Kohinoor.ttc", "Supplemental/Devanagari Sangam MN.ttc"],
                &["nirmala.ttc", "nirmala.ttf"],
                &[
                    "NotoSansDevanagari-Regular.ttf",
                    "NotoSansDevanagari[wdth,wght].ttf",
                ],
            ),
            (
                "bengali",
                &["KohinoorBangla.ttc"],
                &["nirmala.ttc", "nirmala.ttf"],
                &[
                    "NotoSansBengali-Regular.ttf",
                    "NotoSansBengali[wdth,wght].ttf",
                ],
            ),
            (
                "tamil",
                &["Supplemental/Tamil Sangam MN.ttc"],
                &[],
                &["NotoSansTamil-Regular.ttf", "NotoSansTamil[wdth,wght].ttf"],
            ),
            (
                "telugu",
                &["KohinoorTelugu.ttc", "Supplemental/Telugu Sangam MN.ttc"],
                &[],
                &[
                    "NotoSansTelugu-Regular.ttf",
                    "NotoSansTelugu[wdth,wght].ttf",
                ],
            ),
            (
                "kannada",
                &["NotoSansKannada.ttc", "Supplemental/Kannada Sangam MN.ttc"],
                &[],
                &[
                    "NotoSansKannada-Regular.ttf",
                    "NotoSansKannada[wdth,wght].ttf",
                ],
            ),
            (
                "malayalam",
                &["Supplemental/Malayalam Sangam MN.ttc"],
                &[],
                &[
                    "NotoSansMalayalam-Regular.ttf",
                    "NotoSansMalayalam[wdth,wght].ttf",
                ],
            ),
            (
                "gujarati",
                &[
                    "KohinoorGujarati.ttc",
                    "Supplemental/Gujarati Sangam MN.ttc",
                ],
                &[],
                &[
                    "NotoSansGujarati-Regular.ttf",
                    "NotoSansGujarati[wdth,wght].ttf",
                ],
            ),
            (
                "gurmukhi",
                &["Supplemental/Gurmukhi Sangam MN.ttc"],
                &[],
                &[
                    "NotoSansGurmukhi-Regular.ttf",
                    "NotoSansGurmukhi[wdth,wght].ttf",
                ],
            ),
            (
                "oriya",
                &["NotoSansOriya.ttc", "Supplemental/Oriya Sangam MN.ttc"],
                &[],
                &["NotoSansOriya-Regular.ttf", "NotoSansOriya[wdth,wght].ttf"],
            ),
            (
                "sinhala",
                &["Supplemental/Sinhala Sangam MN.ttc"],
                &["nirmala.ttc", "nirmala.ttf"],
                &[
                    "NotoSansSinhala-Regular.ttf",
                    "NotoSansSinhala[wdth,wght].ttf",
                ],
            ),
            (
                "tibetan",
                &["Supplemental/Kokonor.ttf"],
                &["himalaya.ttf"],
                &[
                    "NotoSerifTibetan-Regular.ttf",
                    "NotoSansTibetan-Regular.ttf",
                ],
            ),
            (
                "arabic",
                &["SFArabic.ttf", "GeezaPro.ttc"],
                &["segoeui.ttf", "arial.ttf"],
                &[
                    "NotoSansArabic-Regular.ttf",
                    "NotoSansArabic[wdth,wght].ttf",
                    "NotoNaskhArabic-Regular.ttf",
                ],
            ),
            (
                "hebrew",
                &["SFHebrew.ttf"],
                &["segoeui.ttf"],
                &[
                    "NotoSansHebrew-Regular.ttf",
                    "NotoSansHebrew[wdth,wght].ttf",
                ],
            ),
            (
                "armenian",
                &["SFArmenian.ttf", "NotoSansArmenian.ttc"],
                &["segoeui.ttf"],
                &[
                    "NotoSansArmenian-Regular.ttf",
                    "NotoSansArmenian[wdth,wght].ttf",
                ],
            ),
            (
                "georgian",
                &["SFGeorgian.ttf"],
                &["segoeui.ttf"],
                &[
                    "NotoSansGeorgian-Regular.ttf",
                    "NotoSansGeorgian[wdth,wght].ttf",
                ],
            ),
            (
                "ethiopic",
                &["Supplemental/KefaIII.ttf"],
                &["ebrima.ttf"],
                &[
                    "NotoSansEthiopic-Regular.ttf",
                    "NotoSansEthiopic[wdth,wght].ttf",
                ],
            ),
            (
                "cjk",
                &["Hiragino Sans GB.ttc", "ヒラギノ角ゴシック W3.ttc"],
                &[
                    "msyh.ttc",
                    "msyh.ttf",
                    "YuGothR.ttc",
                    "meiryo.ttc",
                    "simsun.ttc",
                ],
                &[
                    "NotoSansCJK-Regular.ttc",
                    "NotoSansCJKsc-Regular.otf",
                    "NotoSansCJK-VF.ttc",
                    "NotoSansSC-Regular.otf",
                ],
            ),
            (
                "korean",
                &["AppleSDGothicNeo.ttc"],
                &["malgun.ttf"],
                &[
                    "NotoSansCJK-Regular.ttc",
                    "NotoSansCJKkr-Regular.otf",
                    "NotoSansKR-Regular.otf",
                ],
            ),
            (
                "unicode",
                &["Supplemental/Arial Unicode.ttf"],
                &["seguisym.ttf"],
                &["DejaVuSans.ttf", "FreeSans.ttf"],
            ),
        ];
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (name, mac_files, win_files, linux_files) in scripts {
            let mut candidates: Vec<String> = Vec::new();
            candidates.extend(mac_files.iter().map(|f| format!("{mac}{f}")));
            candidates.extend(win_files.iter().map(|f| format!("{win}{f}")));
            for dir in noto.iter().chain(
                [
                    "/usr/share/fonts/truetype/dejavu/",
                    "/usr/share/fonts/TTF/",
                    "/usr/share/fonts/truetype/freefont/",
                ]
                .iter(),
            ) {
                candidates.extend(linux_files.iter().map(|f| format!("{dir}{f}")));
            }
            // Skip a file already loaded for an earlier script (e.g. Nirmala).
            let Some((path, bytes)) = candidates
                .into_iter()
                .filter(|p| !seen.contains(p))
                .find_map(|p| std::fs::read(&p).ok().map(|b| (p, b)))
            else {
                continue;
            };
            seen.insert(path);
            out.push((
                format!("script-{name}"),
                Arc::new(FontData::from_owned(bytes)),
            ));
        }
        out
    })
}

/// Evenly spaced color stops sampled across the deck, including both endpoints.
pub fn deck_color(stops: &[Color32], index: usize, count: usize) -> Option<Color32> {
    let first = *stops.first()?;
    if stops.len() == 1 || count <= 1 {
        return Some(first);
    }
    let position = index.min(count - 1) as f32 / (count - 1) as f32 * (stops.len() - 1) as f32;
    let left = (position.floor() as usize).min(stops.len() - 2);
    let t = position - left as f32;
    let a = stops[left].to_array();
    let b = stops[left + 1].to_array();
    let channel = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    Some(Color32::from_rgb(channel(0), channel(1), channel(2)))
}

/// Choose the higher-contrast black or white foreground using sRGB luminance.
pub fn contrast_text(bg: Color32) -> Color32 {
    let linear = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(bg.r()) + 0.7152 * linear(bg.g()) + 0.0722 * linear(bg.b());
    if luminance > 0.179 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

#[cfg(test)]
mod gradient_tests {
    use super::*;

    #[test]
    fn deck_endpoints_and_intermediate_stops() {
        let stops = [hex(0xFFFF00), hex(0x00FF00), hex(0x0000FF)];
        assert_eq!(deck_color(&stops, 0, 10), Some(stops[0]));
        assert_eq!(deck_color(&stops, 9, 10), Some(stops[2]));
        assert_eq!(deck_color(&stops, 2, 5), Some(stops[1]));
        assert_eq!(deck_color(&stops, 1, 5), Some(hex(0x80FF00)));
        assert_eq!(deck_color(&stops, 100, 10), Some(stops[2]));
    }

    #[test]
    fn short_decks_and_empty_stops_are_safe() {
        let stops = [Color32::RED, Color32::BLUE];
        assert_eq!(deck_color(&stops, 0, 1), Some(Color32::RED));
        assert_eq!(deck_color(&stops, 0, 0), Some(Color32::RED));
        assert_eq!(deck_color(&stops[..1], 9, 10), Some(Color32::RED));
        assert_eq!(deck_color(&[], 0, 10), None);
    }

    #[test]
    fn text_contrasts_with_both_ends() {
        assert_eq!(contrast_text(hex(0xF6D94C)), Color32::BLACK);
        assert_eq!(contrast_text(hex(0x2463D4)), Color32::WHITE);
    }
}
