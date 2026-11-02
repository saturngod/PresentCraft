//! Installed system fonts: a background scan of the usual font folders,
//! grouped by family, plus the set of families loaded into egui for slides.
//!
//! A slide font is a `Face`: 0 is the built-in sans, 1 the built-in serif,
//! and 2.. a system family picked by the person (see `face_for`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// Which typeface slide text uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Face(pub u16);

impl Face {
    pub const SANS: Face = Face(0);
    pub const SERIF: Face = Face(1);

    pub fn from_serif(serif: bool) -> Face {
        if serif {
            Face::SERIF
        } else {
            Face::SANS
        }
    }

    /// Index into the loaded system families, if this is one.
    fn custom(self) -> Option<usize> {
        (self.0 >= 2).then(|| self.0 as usize - 2)
    }

    /// Base egui family name ("sans", "serif" or "f<n>"). Falls back to sans
    /// until a newly picked family is active in egui, so lookups never panic.
    pub fn base(self) -> String {
        match self.custom() {
            Some(i) if i < ACTIVE.load(Ordering::Relaxed) => format!("f{i}"),
            _ if self == Face::SERIF => "serif".into(),
            _ => "sans".into(),
        }
    }

    /// Display / export name of the typeface.
    pub fn name(self) -> String {
        match self.custom() {
            Some(i) => loaded().lock().unwrap().get(i).map(|f| f.name.clone()),
            None => None,
        }
        .unwrap_or_else(|| {
            if self == Face::SERIF {
                "Georgia"
            } else {
                "Arial"
            }
            .into()
        })
    }
}

impl From<bool> for Face {
    /// `true` is the built-in serif, `false` the built-in sans.
    fn from(serif: bool) -> Face {
        Face::from_serif(serif)
    }
}

/// One file inside a font folder: path and index within a collection.
pub type FontFile = (PathBuf, u32);

#[derive(Clone, Debug, Default)]
pub struct Family {
    pub name: String,
    pub regular: Option<FontFile>,
    pub bold: Option<FontFile>,
    pub italic: Option<FontFile>,
    pub bold_italic: Option<FontFile>,
    /// How well each slot's file fits (lower is better); used while scanning.
    scores: [u32; 4],
}

impl Family {
    /// Files for regular, bold, italic and bold italic, each falling back to
    /// the closest style the family has.
    pub fn styles(&self) -> [Option<&FontFile>; 4] {
        let any = self
            .regular
            .as_ref()
            .or(self.bold.as_ref())
            .or(self.italic.as_ref())
            .or(self.bold_italic.as_ref());
        let regular = self.regular.as_ref().or(any);
        let bold = self.bold.as_ref().or(regular);
        let italic = self.italic.as_ref().or(regular);
        let bold_italic = self.bold_italic.as_ref().or(self.bold.as_ref()).or(italic);
        [regular, bold, italic, bold_italic]
    }
}

static CATALOG: OnceLock<Vec<Family>> = OnceLock::new();
static LOADED: OnceLock<Mutex<Vec<Family>>> = OnceLock::new();
/// How many loaded families egui can draw with right now.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// How many were handed to egui last time fonts were installed.
static PENDING: AtomicUsize = AtomicUsize::new(0);
/// A family was added and the egui fonts need rebuilding.
static DIRTY: AtomicBool = AtomicBool::new(false);

fn loaded() -> &'static Mutex<Vec<Family>> {
    LOADED.get_or_init(|| Mutex::new(Vec::new()))
}

/// Starts scanning the system font folders in the background.
pub fn start_scan() {
    std::thread::spawn(|| {
        let _ = CATALOG.set(scan(&font_dirs()));
    });
}

/// All installed families (sorted by name), or `None` while still scanning.
pub fn catalog() -> Option<&'static [Family]> {
    CATALOG.get().map(|v| v.as_slice())
}

/// The face for a system family, loading it for slides if needed.
pub fn face_for(family: &Family) -> Face {
    let mut list = loaded().lock().unwrap();
    let i = match list.iter().position(|f| f.name == family.name) {
        Some(i) => i,
        None => {
            list.push(family.clone());
            DIRTY.store(true, Ordering::Relaxed);
            list.len() - 1
        }
    };
    Face(i as u16 + 2)
}

/// Families egui should register, in `Face` order.
pub(crate) fn families_to_install() -> Vec<Family> {
    let list = loaded().lock().unwrap().clone();
    PENDING.store(list.len(), Ordering::Relaxed);
    DIRTY.store(false, Ordering::Relaxed);
    list
}

/// Call once at the start of each frame: fonts installed last frame are
/// now active. Returns true when fonts need installing again.
pub fn begin_frame() -> bool {
    ACTIVE.store(PENDING.load(Ordering::Relaxed), Ordering::Relaxed);
    DIRTY.load(Ordering::Relaxed)
}

fn font_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(h) = &home {
        dirs.push(h.join("Library/Fonts"));
        dirs.push(h.join(".local/share/fonts"));
        dirs.push(h.join(".fonts"));
    }
    if let Some(w) = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot")) {
        dirs.push(PathBuf::from(w).join("Fonts"));
    }
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(l).join("Microsoft/Windows/Fonts"));
    }
    dirs
}

fn scan(dirs: &[PathBuf]) -> Vec<Family> {
    let mut files = Vec::new();
    for d in dirs {
        walk(d, 0, &mut files);
    }
    let mut map: BTreeMap<String, Family> = BTreeMap::new();
    for path in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let count = ttf_parser::fonts_in_collection(&bytes).unwrap_or(1);
        for index in 0..count {
            let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
                continue;
            };
            let Some(name) = family_name(&face) else {
                continue;
            };
            // Hidden system faces (".SF NS…") aren't meant to be picked.
            if name.starts_with('.') {
                continue;
            }
            let fam = map.entry(name.to_lowercase()).or_insert_with(|| Family {
                name: name.clone(),
                ..Default::default()
            });
            let bold = face.is_bold() || face.weight().to_number() >= 600;
            let italic = face.is_italic() || face.is_oblique() || subfamily_is_italic(&face);
            let (slot, score) = match (bold, italic) {
                (false, false) => (&mut fam.regular, &mut fam.scores[0]),
                (true, false) => (&mut fam.bold, &mut fam.scores[1]),
                (false, true) => (&mut fam.italic, &mut fam.scores[2]),
                (true, true) => (&mut fam.bold_italic, &mut fam.scores[3]),
            };
            // Closest to 400 (or 700 for bold) at normal width wins, so Light,
            // Medium or Condensed cuts don't stand in for Regular and Bold.
            let target = if bold { 700 } else { 400 };
            let mut s = (face.weight().to_number() as i32 - target).unsigned_abs();
            if face.width() != ttf_parser::Width::Normal {
                s += 1000;
            }
            if slot.is_none() || s < *score {
                *slot = Some((path.clone(), index));
                *score = s;
            }
        }
    }
    map.into_values().collect()
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                walk(&p, depth + 1, out);
            }
        } else if matches!(
            p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_ascii_lowercase())
                .as_deref(),
            Some("ttf" | "otf" | "ttc" | "otc")
        ) {
            out.push(p);
        }
    }
}

/// The family name a font menu would show (typographic family first).
fn family_name(face: &ttf_parser::Face) -> Option<String> {
    use ttf_parser::name_id;
    let mut best: Option<(u8, String)> = None;
    for n in face.names() {
        let rank = match n.name_id {
            name_id::TYPOGRAPHIC_FAMILY => 2,
            name_id::FAMILY => 1,
            _ => continue,
        };
        // Prefer English names; skip ones we can't decode.
        let english = n.language_id == 0x0409 || n.language_id == 0;
        let Some(s) = decode_name(&n) else { continue };
        let rank = rank * 2 + english as u8;
        if best.as_ref().map_or(true, |(r, _)| rank > *r) {
            best = Some((rank, s));
        }
    }
    best.map(|(_, s)| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Text of a name-table entry. ttf-parser decodes the Unicode ones; older
/// Apple fonts (Helvetica, Menlo…) only carry Mac Roman names.
fn decode_name(n: &ttf_parser::name::Name) -> Option<String> {
    if let Some(s) = n.to_string() {
        return Some(s);
    }
    if n.platform_id == ttf_parser::PlatformId::Macintosh && n.encoding_id == 0 {
        return Some(n.name.iter().map(|&b| mac_roman(b)).collect());
    }
    None
}

fn mac_roman(b: u8) -> char {
    const HIGH: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»…\u{A0}ÀÃÕŒœ–—“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔ\u{F8FF}ÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";
    if b < 0x80 {
        b as char
    } else {
        HIGH.chars().nth(b as usize - 0x80).unwrap_or('?')
    }
}

/// Whether the style name (e.g. "Medium Italic") says italic or oblique.
fn subfamily_is_italic(face: &ttf_parser::Face) -> bool {
    face.names().into_iter().any(|n| {
        (n.name_id == ttf_parser::name_id::SUBFAMILY
            || n.name_id == ttf_parser::name_id::TYPOGRAPHIC_SUBFAMILY)
            && decode_name(&n).is_some_and(|s| {
                let s = s.to_lowercase();
                s.contains("italic") || s.contains("oblique")
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_roman_table_is_complete() {
        assert_eq!(mac_roman(b'A'), 'A');
        assert_eq!(mac_roman(0x80), 'Ä');
        assert_eq!(mac_roman(0xCA), '\u{A0}');
        assert_eq!(mac_roman(0xFF), 'ˇ');
    }

    #[test]
    fn styles_fall_back_to_regular() {
        let f = Family {
            name: "X".into(),
            regular: Some((PathBuf::from("r.ttf"), 0)),
            bold: Some((PathBuf::from("b.ttf"), 0)),
            ..Default::default()
        };
        let [r, b, i, bi] = f.styles();
        assert_eq!(r.unwrap().0, PathBuf::from("r.ttf"));
        assert_eq!(b.unwrap().0, PathBuf::from("b.ttf"));
        assert_eq!(i.unwrap().0, PathBuf::from("r.ttf"));
        assert_eq!(bi.unwrap().0, PathBuf::from("b.ttf"));
    }

    /// Lists what the scan finds here: `cargo test scan_system -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn scan_system_fonts() {
        let all = scan(&font_dirs());
        let styled = all
            .iter()
            .filter(|f| f.bold.is_some() && f.italic.is_some())
            .count();
        println!("{} families, {} with bold+italic", all.len(), styled);
        for f in all.iter().filter(|f| {
            ["Helvetica", "Helvetica Neue", "Georgia", "Menlo"].contains(&f.name.as_str())
        }) {
            println!("{:?}", f);
        }
        assert!(!all.is_empty());
    }

    #[test]
    fn unloaded_face_draws_with_builtin() {
        assert_eq!(Face::SANS.base(), "sans");
        assert_eq!(Face::SERIF.base(), "serif");
        assert_eq!(Face(900).base(), "sans");
    }
}
