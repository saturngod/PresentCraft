//! Theme folders in the iA Presenter layout: `template.json` (identity and
//! fonts) plus `presets.json` (light and dark colour variants).
//!
//! PresentCraft writes the same keys the original uses and adds three of its
//! own to `template.json` (`Decoration`, `TextSize`, `InvertTitles`) that the
//! original ignores. Reading accepts the original's relaxed JSON.

use crate::fonts::Face;
use crate::json::{obj, Json};
use crate::theme::{Decor, Palette, Preset};
use egui::Color32;
use std::path::{Path, PathBuf};

pub const TEMPLATE: &str = "template.json";
pub const PRESETS: &str = "presets.json";

/// True when `dir` looks like a theme folder.
pub fn is_theme_dir(dir: &Path) -> bool {
    dir.join(TEMPLATE).is_file()
}

/// Reads a theme folder into a custom preset.
pub fn load(dir: &Path) -> Result<Preset, String> {
    load_variant(dir, None)
}

/// Like [`load`], starting in the variant named `preset` (an iA Presenter
/// preset name such as "Light", "Dark" or "Default") when it exists.
pub fn load_variant(dir: &Path, preset: Option<&str>) -> Result<Preset, String> {
    let read = |name: &str| -> Result<Json, String> {
        let text = std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
        Json::parse(&text).map_err(|e| format!("{name}: {e}"))
    };
    let template = read(TEMPLATE)?;
    let mut presets = if dir.join(PRESETS).is_file() {
        read(PRESETS)?
    } else {
        Json::Obj(Vec::new())
    };
    if let Some(name) = preset {
        // The first variant listed is the one the deck starts in.
        let mut list = presets.arr("Presets").to_vec();
        if let Some(i) = list
            .iter()
            .position(|p| p.str("Name").is_some_and(|n| n.eq_ignore_ascii_case(name)))
        {
            let chosen = list.remove(i);
            list.insert(0, chosen);
            presets.set("Presets", Json::Arr(list));
        }
    }
    Ok(from_json(&template, &presets, dir))
}

/// Builds a preset from parsed `template.json` and `presets.json`.
pub fn from_json(template: &Json, presets: &Json, dir: &Path) -> Preset {
    let list = presets.arr("Presets");
    let by_look = |look: &str| {
        list.iter()
            .find(|p| p.str("Appearance").is_some_and(|a| a.eq_ignore_ascii_case(look)))
            .or(list.first())
    };
    let light_p = by_look("light");
    let dark_p = by_look("dark");
    // "Dark…" values are the dark-coloured ones, used as text on a light
    // background; "Light…" values are text on a dark background.
    let color = |p: Option<&Json>, key: &str, fallback: Color32| {
        p.and_then(|p| p.str(key)).and_then(parse_color).unwrap_or(fallback)
    };
    let light = Palette {
        bg: color(light_p, "LightBackgroundColor", Color32::WHITE),
        title: color(light_p, "DarkTitleTextColor", Color32::BLACK),
        body: color(light_p, "DarkBodyTextColor", Color32::BLACK),
    };
    let dark = Palette {
        bg: color(dark_p, "DarkBackgroundColor", Color32::BLACK),
        title: color(dark_p, "LightTitleTextColor", Color32::WHITE),
        body: color(dark_p, "LightBodyTextColor", Color32::WHITE),
    };
    let accent_src = light_p.or(dark_p);
    let defaults = crate::theme::presets()[0].accents;
    let accents = std::array::from_fn(|i| color(accent_src, &format!("Accent{}", i + 1), defaults[i]));
    let font = |key: &str| {
        template
            .str(key)
            .or_else(|| light_p.and_then(|p| p.str(key)))
            .map(face_named)
            .unwrap_or(Face::SANS)
    };
    let name = template
        .str("Name")
        .map(str::to_string)
        .filter(|n| !n.trim().is_empty())
        .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Imported".into());
    let classes = template.str("CssClasses").unwrap_or("");
    Preset {
        name: Box::leak(name.into_boxed_str()),
        custom: true,
        file: None,
        gradients: [
            gradient(light_p, "LightBgGradient"),
            gradient(dark_p, "DarkBgGradient"),
        ],
        title_face: font("TitleFont"),
        body_face: font("BodyFont"),
        size_idx: template
            .num("TextSize")
            .map(|n| (n.max(0.0) as usize).min(crate::SIZES.len() - 1))
            .unwrap_or(1),
        invert_titles: template
            .bool("InvertTitles")
            .unwrap_or_else(|| classes.split_whitespace().any(|c| c == "invert")),
        light,
        dark,
        // The first variant listed is the one the deck shows.
        starts_dark: list
            .first()
            .is_some_and(|p| p.str("Appearance").is_some_and(|a| a.eq_ignore_ascii_case("dark"))),
        accents,
        decor: template.str("Decoration").map(decor_from).unwrap_or(Decor::None),
    }
}

/// Writes `preset` as a theme folder (`template.json` and `presets.json`).
/// `dark` says which variant the deck shows, written first in the list.
pub fn save(dir: &Path, p: &Preset, dark: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let (template, presets) = to_json(p, dark);
    std::fs::write(dir.join(TEMPLATE), template.to_pretty())?;
    std::fs::write(dir.join(PRESETS), presets.to_pretty())
}

pub fn to_json(p: &Preset, dark: bool) -> (Json, Json) {
    let title_font = face_name(p.title_face);
    let body_font = face_name(p.body_face);
    let template = obj([
        ("Name", p.name.into()),
        ("Version", "1.0".into()),
        ("Author", "PresentCraft".into()),
        ("ShortDescription", "".into()),
        ("LongDescription", "".into()),
        ("Css", "".into()),
        ("TitleFont", title_font.clone().into()),
        ("BodyFont", body_font.clone().into()),
        (
            "CssClasses",
            if p.invert_titles { "invert" } else { "" }.into(),
        ),
        ("LayoutExamples", Json::Arr(Vec::new())),
        ("Decoration", decor_name(p.decor).into()),
        ("TextSize", p.size_idx.into()),
        ("InvertTitles", p.invert_titles.into()),
    ]);
    let variant = |look: &str| {
        let mut v = obj([
            ("Name", if look == "dark" { "Dark" } else { "Light" }.into()),
            ("TitleFont", title_font.clone().into()),
            ("BodyFont", body_font.clone().into()),
            ("DarkBodyTextColor", color_hex(p.light.body).into()),
            ("LightBodyTextColor", color_hex(p.dark.body).into()),
            ("DarkTitleTextColor", color_hex(p.light.title).into()),
            ("LightTitleTextColor", color_hex(p.dark.title).into()),
            ("DarkBackgroundColor", color_hex(p.dark.bg).into()),
            ("LightBackgroundColor", color_hex(p.light.bg).into()),
            ("Appearance", look.into()),
        ]);
        for (i, a) in p.accents.iter().enumerate() {
            v.set(&format!("Accent{}", i + 1), color_hex(*a).into());
        }
        for (key, stops) in ["LightBgGradient", "DarkBgGradient"].iter().zip(&p.gradients) {
            if !stops.is_empty() {
                let list = stops.iter().map(|c| color_hex(*c).into()).collect();
                v.set(key, Json::Arr(list));
            }
        }
        v
    };
    let order = if dark { ["dark", "light"] } else { ["light", "dark"] };
    let presets = obj([("Presets", Json::Arr(order.map(variant).to_vec()))]);
    (template, presets)
}

/// Background gradient stops from a preset (fewer than two: none).
fn gradient(p: Option<&Json>, key: &str) -> Vec<Color32> {
    let stops: Vec<Color32> = p
        .map(|p| p.arr(key))
        .unwrap_or_default()
        .iter()
        .filter_map(|c| match c {
            Json::Str(s) => parse_color(s),
            _ => None,
        })
        .collect();
    if stops.len() >= 2 {
        stops
    } else {
        Vec::new()
    }
}

/// `#RGB` or `#RRGGBB` (an optional alpha pair is ignored).
pub fn parse_color(s: &str) -> Option<Color32> {
    let h = s.trim().strip_prefix('#')?;
    let digits: Vec<u8> = h
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    match digits.len() {
        3 | 4 => Some(Color32::from_rgb(
            digits[0] * 17,
            digits[1] * 17,
            digits[2] * 17,
        )),
        6 | 8 => Some(Color32::from_rgb(
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
        )),
        _ => None,
    }
}

pub fn color_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

const SERIF_HINTS: &[&str] = &[
    "serif", "georgia", "times", "garamond", "baskerville", "charter", "palatino", "new york",
    "didot", "bodoni", "caslon", "merriweather", "playfair", "lora", "cambria",
];

/// The face for a font family name: an installed family when the font list
/// is ready, otherwise the built-in serif or sans.
pub fn face_named(name: &str) -> Face {
    let n = name.trim();
    let lower = n.to_ascii_lowercase();
    if lower.is_empty()
        || matches!(lower.as_str(), "system" | "system-ui" | "-apple-system" | "sans" | "sans-serif")
    {
        return Face::SANS;
    }
    if lower == "serif" {
        return Face::SERIF;
    }
    if let Some(f) = crate::fonts::catalog()
        .and_then(|c| c.iter().find(|f| f.name.eq_ignore_ascii_case(n)))
    {
        return crate::fonts::face_for(f);
    }
    // "sans serif" names mention serif too, so rule those out first.
    let serif = !lower.contains("sans") && SERIF_HINTS.iter().any(|h| lower.contains(h));
    Face::from_serif(serif)
}

pub fn face_name(face: Face) -> String {
    if face == Face::SANS {
        "System".into()
    } else if face == Face::SERIF {
        "Serif".into()
    } else {
        face.name()
    }
}

fn decor_name(d: Decor) -> &'static str {
    match d {
        Decor::None => "None",
        Decor::Bar => "Bar",
        Decor::Glow => "Glow",
        Decor::Frame => "Frame",
        Decor::Band => "Band",
        Decor::Shapes => "Shapes",
    }
}

fn decor_from(s: &str) -> Decor {
    match s.to_ascii_lowercase().as_str() {
        "bar" => Decor::Bar,
        "glow" => Decor::Glow,
        "frame" => Decor::Frame,
        "band" => Decor::Band,
        "shapes" => Decor::Shapes,
        _ => Decor::None,
    }
}

/// Same colours, fonts, size and decoration (names aside).
pub fn same_look(a: &Preset, b: &Preset) -> bool {
    a.light == b.light
        && a.dark == b.dark
        && a.accents == b.accents
        && a.title_face == b.title_face
        && a.body_face == b.body_face
        && a.size_idx == b.size_idx
        && a.invert_titles == b.invert_titles
        && a.decor == b.decor
        && a.gradients == b.gradients
}

// ---------- iA Presenter's themes ----------

/// Where an installed iA Presenter keeps its themes.
fn ia_theme_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from(
        "/Applications/iA Presenter.app/Contents/Resources/Themes",
    )];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(
            PathBuf::from(home).join("Applications/iA Presenter.app/Contents/Resources/Themes"),
        );
    }
    roots
}

/// The iA Presenter theme folder for a document's `"template"` value
/// (a folder name such as "tokyo"), when iA Presenter is installed.
pub fn ia_theme_dir(template: &str) -> Option<PathBuf> {
    let want = template.trim();
    for root in ia_theme_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if is_theme_dir(&p)
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(want))
            {
                return Some(p);
            }
        }
    }
    None
}

// ---------- the person's saved themes ----------

/// Folder holding the person's own themes, one sub-folder each.
pub fn user_dir() -> Option<PathBuf> {
    crate::settings::Settings::app_dir().map(|d| d.join("Themes"))
}

/// Every theme folder in the person's theme folder, sorted by name.
pub fn load_user_themes() -> Vec<(PathBuf, Preset)> {
    let Some(dir) = user_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_theme_dir(p))
        .collect();
    dirs.sort();
    dirs.into_iter()
        .filter_map(|d| load(&d).ok().map(|p| (d, p)))
        .collect()
}

/// A folder name for a theme that doesn't clash with an existing one.
pub fn unique_dir(parent: &Path, name: &str) -> PathBuf {
    let base: String = name
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '\0') { '-' } else { c })
        .collect();
    let base = base.trim().trim_start_matches('.');
    let base = if base.is_empty() { "Theme" } else { base };
    let mut candidate = parent.join(base);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base} {n}"));
        n += 1;
    }
    candidate
}

/// Copies a theme folder (CSS, preview pictures and all) into `dest`.
pub fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copies a theme folder into the person's themes and loads it.
pub fn import(src: &Path) -> Result<Preset, String> {
    let mut p = load(src)?;
    let parent = user_dir().ok_or("No folder for saved themes")?;
    let dest = unique_dir(&parent, p.name);
    copy_dir(src, &dest).map_err(|e| e.to_string())?;
    p.file = Some(dest);
    Ok(p)
}

/// Removes one of the person's saved theme folders. Only folders inside
/// the themes folder are touched.
pub fn remove(dir: &Path) {
    if let Some(parent) = user_dir() {
        if dir.parent() == Some(parent.as_path()) && is_theme_dir(dir) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ia_helvetica_theme() {
        let template = Json::parse(
            r##"{ "Name": "Helvetica", "TitleFont": "Helvetica", "BodyFont": "Helvetica",
                 "CssClasses": "variable-size-headings", "LayoutExamples": [ { "Name": "x", }, ] }"##,
        )
        .unwrap();
        let presets = Json::parse(
            r##"{ "Presets": [
              { "Name": "Light", "DarkBodyTextColor": "#000", "LightBodyTextColor": "#FFF",
                "DarkTitleTextColor": "#000", "LightTitleTextColor": "#FFF",
                "DarkBackgroundColor": "#000", "LightBackgroundColor": "#FFF",
                "Appearance": "light", "Accent1": "#FF3333", "Accent2": "#43aa8b" },
              { "Name": "Dark", "Appearance": "dark", "DarkBackgroundColor": "#111",
                "LightTitleTextColor": "#EEE", "LightBodyTextColor": "#DDD" } ] }"##,
        )
        .unwrap();
        let p = from_json(&template, &presets, Path::new("/x/Helvetica"));
        assert_eq!(p.name, "Helvetica");
        assert!(p.custom && !p.starts_dark && !p.invert_titles);
        assert_eq!(p.light.bg, Color32::WHITE);
        assert_eq!(p.light.title, Color32::BLACK);
        assert_eq!(p.dark.bg, Color32::from_rgb(0x11, 0x11, 0x11));
        assert_eq!(p.dark.body, Color32::from_rgb(0xDD, 0xDD, 0xDD));
        assert_eq!(p.accents[1], Color32::from_rgb(0x43, 0xAA, 0x8B));
    }

    #[test]
    fn loads_installed_ia_themes_when_present() {
        let dir = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes");
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = load(&e.path()).unwrap_or_else(|err| panic!("{:?}: {err}", e.path()));
            assert!(!p.name.is_empty());
        }
    }

    #[test]
    fn finds_ia_template_by_folder_name_when_installed() {
        if !Path::new("/Applications/iA Presenter.app").exists() {
            return;
        }
        let dir = ia_theme_dir("tokyo").expect("Tokyo theme");
        let p = load_variant(&dir, Some("Default")).unwrap();
        assert_eq!(p.name, "Tokyo");
        let helv = ia_theme_dir("helvetica").unwrap();
        assert!(load_variant(&helv, Some("Dark")).unwrap().starts_dark);
        assert!(!load_variant(&helv, Some("Light")).unwrap().starts_dark);
    }

    #[test]
    fn round_trips_a_builtin_preset() {
        for (i, original) in crate::theme::presets().into_iter().enumerate() {
            let (t, p) = to_json(&original, i % 2 == 0);
            let back = from_json(&t, &p, Path::new("/x"));
            assert_eq!(back.name, original.name);
            assert!(back.light == original.light && back.dark == original.dark);
            assert_eq!(back.accents, original.accents);
            assert_eq!(back.decor, original.decor);
            assert_eq!(back.gradients, original.gradients);
            assert_eq!(back.size_idx, original.size_idx);
            assert_eq!(back.invert_titles, original.invert_titles);
            assert_eq!(back.title_face == Face::SERIF, original.title_face == Face::SERIF);
            assert_eq!(back.starts_dark, i % 2 == 0);
        }
    }
}
