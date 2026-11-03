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
    /// Cover and title slides swap foreground and background.
    pub invert_titles: bool,
    pub size_scale: f32,
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

const ACCENTS: [Color32; 6] = [
    hex(0xFF3333),
    hex(0x43AA8B),
    hex(0xF9C74F),
    hex(0x90BE6D),
    hex(0xF8961E),
    hex(0x577590),
];
const ACCENTS_B: [Color32; 6] = [
    hex(0xF94144),
    hex(0x43AA8B),
    hex(0xF9C74F),
    hex(0x90BE6D),
    hex(0xF8961E),
    hex(0x577590),
];

const fn pal(bg: u32, title: u32, body: u32) -> Palette {
    Palette {
        bg: hex(bg),
        title: hex(title),
        body: hex(body),
    }
}

/// Name of the theme new presentations start with.
pub const DEFAULT_THEME: &str = "Studio";

pub fn presets() -> Vec<Preset> {
    vec![
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Swiss",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 1,
            invert_titles: false,
            light: pal(0xFFFFFF, 0x000000, 0x000000),
            dark: pal(0x000000, 0xFFFFFF, 0xFFFFFF),
            starts_dark: false,
            accents: ACCENTS,
            decor: Decor::None,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Contrast Serif",
            title_face: crate::fonts::Face::SERIF,
            body_face: crate::fonts::Face::SERIF,
            size_idx: 2,
            invert_titles: true,
            light: pal(0xFFFFFF, 0x000000, 0x000000),
            dark: pal(0x000000, 0xFFFFFF, 0xFFFFFF),
            starts_dark: false,
            accents: ACCENTS,
            decor: Decor::None,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Paper",
            title_face: crate::fonts::Face::SERIF,
            body_face: crate::fonts::Face::SERIF,
            size_idx: 1,
            invert_titles: false,
            light: pal(0xEBE7E7, 0x11100E, 0x11100E),
            dark: pal(0x11100E, 0xEBE7E7, 0xEBE7E7),
            starts_dark: false,
            accents: [
                hex(0x6A6256),
                hex(0x43AA8B),
                hex(0xF9C74F),
                hex(0x90BE6D),
                hex(0xF8961E),
                hex(0x577590),
            ],
            decor: Decor::None,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Signal",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 2,
            invert_titles: false,
            light: pal(0xFFFFFF, 0x000000, 0x000000),
            dark: pal(0x000000, 0xFFC400, 0xFFFFFF),
            starts_dark: true,
            accents: ACCENTS_B,
            decor: Decor::None,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Graphite",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 0,
            invert_titles: false,
            light: pal(0xF2F2F2, 0x000000, 0x000000),
            dark: pal(0x1C1C1E, 0xFFFFFF, 0xFFFFFF),
            starts_dark: true,
            accents: ACCENTS_B,
            decor: Decor::None,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Studio",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 1,
            invert_titles: false,
            light: pal(0xFFFFFF, 0x111C3D, 0x3C4560),
            dark: pal(0x0F1629, 0xFFFFFF, 0xC5CCE0),
            starts_dark: false,
            accents: [
                hex(0x4361EE),
                hex(0x22B8CF),
                hex(0xF7B32B),
                hex(0xEF476F),
                hex(0x7B61FF),
                hex(0x06D6A0),
            ],
            decor: Decor::Bar,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Midnight",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 2,
            invert_titles: false,
            light: pal(0xF3F6FB, 0x0B1124, 0x3B4766),
            dark: pal(0x0B1020, 0xFFFFFF, 0xB8C1DC),
            starts_dark: true,
            accents: [
                hex(0x38BDF8),
                hex(0xA78BFA),
                hex(0x34D399),
                hex(0xFBBF24),
                hex(0xF472B6),
                hex(0x60A5FA),
            ],
            decor: Decor::Glow,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Editorial",
            title_face: crate::fonts::Face::SERIF,
            body_face: crate::fonts::Face::SANS,
            size_idx: 1,
            invert_titles: false,
            light: pal(0xFAF7F2, 0x1F1A17, 0x4A403A),
            dark: pal(0x1A1714, 0xF3EDE4, 0xCFC6BB),
            starts_dark: false,
            accents: [
                hex(0xB4532A),
                hex(0x2F6F6A),
                hex(0xD9A441),
                hex(0x7A8B5A),
                hex(0x8C4A6B),
                hex(0x4D6A8C),
            ],
            decor: Decor::Frame,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Corporate",
            title_face: crate::fonts::Face::SANS,
            body_face: crate::fonts::Face::SANS,
            size_idx: 0,
            invert_titles: false,
            light: pal(0xFFFFFF, 0x0B3B5B, 0x334155),
            dark: pal(0x0B1F2A, 0xF1F5F9, 0xB6C3CF),
            starts_dark: false,
            accents: [
                hex(0x0E9F9E),
                hex(0x0B3B5B),
                hex(0xF59E0B),
                hex(0x64748B),
                hex(0x22C55E),
                hex(0xE11D48),
            ],
            decor: Decor::Band,
        },
        Preset {
            custom: false,
            file: None,
            gradients: [Vec::new(), Vec::new()],
            chart_colors: [Vec::new(), Vec::new()],
            accent1: [None, None],
            dark_accents: None,
            name: "Sunset",
            title_face: crate::fonts::Face::SERIF,
            body_face: crate::fonts::Face::SANS,
            size_idx: 2,
            invert_titles: false,
            light: pal(0xFFF6F0, 0x6B1D3A, 0x5B3946),
            dark: pal(0x24101C, 0xFFE9DF, 0xE6C3CC),
            starts_dark: false,
            accents: [
                hex(0xF0545A),
                hex(0xF59E42),
                hex(0x8B5CF6),
                hex(0x14B8A6),
                hex(0xEAB308),
                hex(0xEC4899),
            ],
            decor: Decor::Shapes,
        },
    ]
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
    // UI text in a clean system sans.
    if fonts.font_data.contains_key("sans") {
        if let Some(p) = fonts.families.get_mut(&FontFamily::Proportional) {
            p.insert(0, "sans".to_string());
        }
    }
    ctx.set_fonts(fonts);
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
