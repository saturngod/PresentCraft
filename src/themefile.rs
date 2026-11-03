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
/// Preview pictures iA Presenter shows in its theme list (500 × 281).
pub const PREVIEW_PNG: &str = "template.png";
pub const PREVIEW_WEBP: &str = "template.webp";

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
    let mut p = from_json(&template, &presets, dir);
    p.file = Some(dir.to_path_buf());
    Ok(p)
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
    let defaults = crate::theme::presets()[0].accents;
    let accents_of = |p: Option<&Json>| -> [Color32; 6] {
        std::array::from_fn(|i| color(p, &format!("Accent{}", i + 1), defaults[i]))
    };
    let accents = accents_of(light_p.or(dark_p));
    let dark_accents = Some(accents_of(dark_p)).filter(|a| *a != accents);
    let font = |key: &str| {
        template
            .str(key)
            .or_else(|| light_p.and_then(|p| p.str(key)))
            .map(|n| face_named_in(n, Some(dir)))
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
        chart_colors: [
            graph_colors(light_p, "LightGraphDataColors"),
            graph_colors(dark_p, "DarkGraphDataColors"),
        ],
        // Named like the text colours: "Dark…" is used on light slides.
        dark_accents,
        accent1: [
            light_p.and_then(|p| p.str("DarkAccent1")).and_then(parse_color),
            dark_p.and_then(|p| p.str("LightAccent1")).and_then(parse_color),
        ],
        title_face: font("TitleFont"),
        body_face: font("BodyFont"),
        size_idx: template
            .num("TextSize")
            .map(|n| (n.max(0.0) as usize).min(crate::SIZES.len() - 1))
            .unwrap_or(1),
        invert_titles: template.bool("InvertTitles").unwrap_or_else(|| {
            layouts_inverted(template) || classes.split_whitespace().any(|c| c == "invert")
        }),
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

/// Writes `p` as a theme folder laid out like iA Presenter's:
///
/// ```text
/// Name/
/// ├── template.json    identity, fonts, stylesheet name, layout examples
/// ├── presets.json     colour variants
/// ├── name.css         stylesheet (iA Presenter reads it)
/// ├── template.png     preview, 500 × 281 (also template.webp)
/// ├── fonts/           font files, when the theme has its own
/// └── image1.webp …    example pictures, when the theme has them
/// ```
///
/// When the theme came from another folder (`source`), everything in it is
/// copied first. Our values are then merged into its JSON, so keys this app
/// doesn't use, and colours written as `#000`, stay as they were.
/// `dark` says which variant the deck shows, listed first.
pub fn save(dir: &Path, p: &Preset, dark: bool, source: Option<&Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    if let Some(src) = source.filter(|s| s.is_dir() && !same_dir(s, dir)) {
        copy_dir(src, dir)?;
    }
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .and_then(|t| Json::parse(&t).ok())
    };
    let fonts = dir.to_path_buf();
    let (mut template, presets) = merge_json(p, dark, read(TEMPLATE), read(PRESETS), &fonts);
    let has_css = template
        .str("Css")
        .is_some_and(|c| !c.trim().is_empty() && dir.join(c).is_file());
    if !has_css {
        let name = format!("{}.css", css_file_stem(p.name));
        if !dir.join(&name).exists() {
            std::fs::write(dir.join(&name), default_css(p.name))?;
        }
        template.set("Css", name.into());
    }
    // Keep a theme's own preview art; redraw the ones we made.
    if template.bool("PreviewGenerated").unwrap_or(false) || !dir.join(PREVIEW_PNG).is_file() {
        write_previews(dir, p).map_err(std::io::Error::other)?;
        template.set("PreviewGenerated", true.into());
    }
    std::fs::write(dir.join(TEMPLATE), template.to_pretty())?;
    std::fs::write(dir.join(PRESETS), presets.to_pretty())
}

/// `template.json` and `presets.json` for a new theme folder.
#[cfg(test)]
pub fn to_json(p: &Preset, dark: bool) -> (Json, Json) {
    merge_json(p, dark, None, None, Path::new(""))
}

/// Our values merged into an existing `template.json` / `presets.json`.
fn merge_json(
    p: &Preset,
    dark: bool,
    base_template: Option<Json>,
    base_presets: Option<Json>,
    fonts: &Path,
) -> (Json, Json) {
    let mut t = match base_template {
        Some(j @ Json::Obj(_)) => j,
        _ => Json::Obj(Vec::new()),
    };
    let set_default = |t: &mut Json, key: &str, v: Json| {
        if t.get(key).is_none() {
            t.set(key, v);
        }
    };
    t.set("Name", p.name.into());
    set_default(&mut t, "Version", "1.0".into());
    set_default(&mut t, "Author", "PresentCraft".into());
    set_default(&mut t, "ShortDescription", "".into());
    set_default(&mut t, "LongDescription", "".into());
    set_default(&mut t, "Css", "".into());
    // template.json holds display names, presets.json CSS names.
    let title_font = keep_font(t.str("TitleFont"), p.title_face, fonts, face_name);
    let body_font = keep_font(t.str("BodyFont"), p.body_face, fonts, face_name);
    t.set("TitleFont", title_font.clone().into());
    t.set("BodyFont", body_font.clone().into());
    // "invert" belongs on layouts, not on the whole theme.
    let classes: Vec<&str> = t
        .str("CssClasses")
        .unwrap_or("")
        .split_whitespace()
        .filter(|c| *c != "invert")
        .collect();
    t.set("CssClasses", classes.join(" ").into());
    set_inverted_layouts(&mut t, p.invert_titles);
    set_default(&mut t, "LayoutExamples", Json::Arr(Vec::new()));
    t.set("Decoration", decor_name(p.decor).into());
    t.set("TextSize", p.size_idx.into());
    t.set("InvertTitles", p.invert_titles.into());

    let mut list = base_presets
        .as_ref()
        .map(|j| j.arr("Presets").to_vec())
        .unwrap_or_default();
    if list.is_empty() {
        list = ["Light", "Dark"]
            .map(|n| obj([("Name", n.into()), ("Appearance", n.to_ascii_lowercase().into())]))
            .to_vec();
    }
    let defaults = crate::theme::presets()[0].accents;
    for v in &mut list {
        let tf = keep_font(v.str("TitleFont"), p.title_face, fonts, css_font_name);
        let bf = keep_font(v.str("BodyFont"), p.body_face, fonts, css_font_name);
        v.set("TitleFont", tf.into());
        v.set("BodyFont", bf.into());
        // "Dark…" colours are used on light slides, "Light…" on dark ones.
        for (key, ours, fallback) in [
            ("DarkBodyTextColor", p.light.body, Color32::BLACK),
            ("LightBodyTextColor", p.dark.body, Color32::WHITE),
            ("DarkTitleTextColor", p.light.title, Color32::BLACK),
            ("LightTitleTextColor", p.dark.title, Color32::WHITE),
            ("DarkBackgroundColor", p.dark.bg, Color32::BLACK),
            ("LightBackgroundColor", p.light.bg, Color32::WHITE),
        ] {
            keep_color(v, key, ours, fallback);
        }
        if v.get("Appearance").is_none() {
            v.set("Appearance", "light".into());
        }
        let is_dark = v
            .str("Appearance")
            .is_some_and(|a| a.eq_ignore_ascii_case("dark"));
        let accents = if is_dark {
            p.dark_accents.unwrap_or(p.accents)
        } else {
            p.accents
        };
        for (i, a) in accents.iter().enumerate() {
            keep_color(v, &format!("Accent{}", i + 1), *a, defaults[i]);
        }
        for (key, c) in [("DarkAccent1", p.accent1[0]), ("LightAccent1", p.accent1[1])] {
            if let Some(c) = c {
                keep_color(v, key, c, c);
            }
        }
        for (key, stops) in ["LightBgGradient", "DarkBgGradient"].iter().zip(&p.gradients) {
            if stops.is_empty() {
                v.remove(key);
            } else if gradient(Some(v), key) != *stops {
                let list = stops.iter().map(|c| color_hex(*c).into()).collect();
                v.set(key, Json::Arr(list));
            }
        }
    }
    // The variant the deck shows goes first.
    let look = if dark { "dark" } else { "light" };
    if let Some(i) = list.iter().position(|v| {
        v.str("Appearance")
            .is_some_and(|a| a.eq_ignore_ascii_case(look))
    }) {
        let chosen = list.remove(i);
        list.insert(0, chosen);
    }
    let mut presets = match base_presets {
        Some(j @ Json::Obj(_)) => j,
        _ => Json::Obj(Vec::new()),
    };
    presets.set("Presets", Json::Arr(list));
    (t, presets)
}

/// Layouts whose `invert` class swaps cover and title slides to the
/// opposite appearance (iA Presenter's way of inverting titles).
const INVERTED_LAYOUTS: [&str; 2] = ["Cover", "Title"];

fn layouts_inverted(template: &Json) -> bool {
    template.arr("Layouts").iter().any(|l| {
        l.str("Name")
            .is_some_and(|n| INVERTED_LAYOUTS.iter().any(|x| x.eq_ignore_ascii_case(n)))
            && l.str("Classes")
                .is_some_and(|c| c.split_whitespace().any(|c| c == "invert"))
    })
}

/// Adds or removes `invert` on the Cover and Title layouts, keeping any
/// other layouts and classes.
fn set_inverted_layouts(t: &mut Json, invert: bool) {
    let had_key = t.get("Layouts").is_some();
    let mut list = t.arr("Layouts").to_vec();
    for name in INVERTED_LAYOUTS {
        let pos = list
            .iter()
            .position(|l| l.str("Name").is_some_and(|n| n.eq_ignore_ascii_case(name)));
        let mut entry = match pos {
            Some(i) => list[i].clone(),
            None if invert => obj([("Name", name.into())]),
            None => continue,
        };
        let mut classes: Vec<String> = entry
            .str("Classes")
            .unwrap_or("")
            .split_whitespace()
            .filter(|c| *c != "invert")
            .map(str::to_string)
            .collect();
        if invert {
            classes.push("invert".into());
        }
        entry.set("Classes", classes.join(" ").into());
        let empty = classes.is_empty()
            && matches!(&entry, Json::Obj(items) if items.len() == 2);
        match (pos, empty) {
            (Some(i), true) => {
                list.remove(i);
            }
            (Some(i), false) => list[i] = entry,
            (None, _) => list.push(entry),
        }
    }
    if !list.is_empty() || had_key {
        t.set("Layouts", Json::Arr(list));
    }
}

/// Keeps a colour as written (`#000`, `transparent`) when it still means
/// the same colour; otherwise writes ours as `#RRGGBB`.
fn keep_color(v: &mut Json, key: &str, ours: Color32, fallback: Color32) {
    let current = v
        .str(key)
        .map(|s| parse_color(s).unwrap_or(fallback));
    if current != Some(ours) {
        v.set(key, color_hex(ours).into());
    }
}

/// Keeps a font name as written when it still picks the same face.
fn keep_font(current: Option<&str>, face: Face, fonts: &Path, name: fn(Face) -> String) -> String {
    match current {
        Some(n) if face_named_in(n, Some(fonts)) == face => n.to_string(),
        _ => name(face),
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// iA Presenter names a new theme's stylesheet after the theme in lower
/// case ("My Theme" → "my theme.css").
fn css_file_stem(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '\0') { '-' } else { c })
        .collect::<String>()
        .to_lowercase();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() {
        "theme".into()
    } else {
        s
    }
}

/// Letters and digits only, lower case: "New York" → "newyork". Used to
/// match iA Presenter template names.
fn css_slug(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if s.is_empty() {
        "theme".into()
    } else {
        s
    }
}

fn default_css(name: &str) -> String {
    format!(
        "/* {name}: made with PresentCraft.\n   Colours, accents and fonts are in presets.json and template.json.\n   iA Presenter reads this stylesheet; add theme CSS below. */\n"
    )
}

/// Draws `template.png` and `template.webp`: a light and a dark slide card,
/// overlapping like iA Presenter's own previews, in the theme's colours.
fn write_previews(dir: &Path, p: &Preset) -> Result<(), String> {
    use image::{Rgba, RgbaImage};
    let (w, h) = (500u32, 281u32);
    let mut img = RgbaImage::new(w, h);
    let rgba = |c: Color32, a: u8| Rgba([c.r(), c.g(), c.b(), a]);
    let fill = |img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, c: Rgba<u8>| {
        for y in y0..y1.min(h) {
            for x in x0..x1.min(w) {
                let dst = img.get_pixel_mut(x, y);
                let a = c[3] as u32;
                for k in 0..3 {
                    dst[k] = ((c[k] as u32 * a + dst[k] as u32 * (255 - a)) / 255) as u8;
                }
                dst[3] = (a + dst[3] as u32 * (255 - a) / 255).min(255) as u8;
            }
        }
    };
    let card = |img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, look: usize| {
        let pal = if look == 0 { p.light } else { p.dark };
        let stops = &p.gradients[look];
        for y in y0..y1 {
            let bg = if stops.len() >= 2 {
                let t = (y - y0) as f32 / (y1 - y0).max(1) as f32 * (stops.len() - 1) as f32;
                let i = (t.floor() as usize).min(stops.len() - 2);
                lerp(stops[i], stops[i + 1], t - i as f32)
            } else {
                pal.bg
            };
            fill(img, x0, y, x1, y + 1, rgba(bg, 255));
        }
        let cw = x1 - x0;
        // Decoration hint, title lines and a body line.
        match p.decor {
            Decor::Bar | Decor::Band => fill(img, x0, y0, x1, y0 + 8, rgba(p.accents[0], 255)),
            Decor::Frame => {
                let c = rgba(pal.title, 90);
                fill(img, x0 + 8, y0 + 8, x1 - 8, y0 + 9, c);
                fill(img, x0 + 8, y1 - 9, x1 - 8, y1 - 8, c);
                fill(img, x0 + 8, y0 + 8, x0 + 9, y1 - 8, c);
                fill(img, x1 - 9, y0 + 8, x1 - 8, y1 - 8, c);
            }
            _ => {}
        }
        let left = x0 + 20;
        let base = y1 - 30;
        fill(img, left, base - 62, left + cw * 64 / 100, base - 40, rgba(pal.title, 255));
        fill(img, left, base - 34, left + cw * 46 / 100, base - 12, rgba(pal.title, 255));
        fill(img, left, base - 2, left + cw * 40 / 100, base + 8, rgba(pal.body, 220));
        fill(img, left, base + 14, left + 24, base + 17, rgba(p.accents[0], 255));
    };
    // Light card top left, dark card bottom right with a soft shadow.
    let (light_first, dark_second) = ((0, 0, 338, 210), (150, 57, 487, 268));
    fill(&mut img, dark_second.0 + 13, dark_second.1 + 13, 500, 281, Rgba([0, 0, 0, 60]));
    card(&mut img, dark_second.0, dark_second.1, dark_second.2, dark_second.3, 1);
    fill(&mut img, light_first.0 + 6, light_first.1 + 6, light_first.2 + 6, light_first.3 + 6, Rgba([0, 0, 0, 40]));
    card(&mut img, light_first.0, light_first.1, light_first.2, light_first.3, 0);
    img.save(dir.join(PREVIEW_PNG)).map_err(|e| e.to_string())?;
    let file = std::fs::File::create(dir.join(PREVIEW_WEBP)).map_err(|e| e.to_string())?;
    image::codecs::webp::WebPEncoder::new_lossless(std::io::BufWriter::new(file))
        .encode(img.as_raw(), w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Chart colours from `{"data-0": "#…", "data-1": …}`, in number order.
fn graph_colors(p: Option<&Json>, key: &str) -> Vec<Color32> {
    let Some(Json::Obj(items)) = p.and_then(|p| p.get(key)) else {
        return Vec::new();
    };
    let mut list: Vec<(u32, Color32)> = items
        .iter()
        .filter_map(|(k, v)| {
            let n = k.strip_prefix("data-")?.parse().ok()?;
            match v {
                Json::Str(s) => parse_color(s).map(|c| (n, c)),
                _ => None,
            }
        })
        .collect();
    list.sort_by_key(|(n, _)| *n);
    list.into_iter().map(|(_, c)| c).collect()
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

/// `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` (Copenhagen's titles are
/// `#000000b3`, black at 70%).
pub fn parse_color(s: &str) -> Option<Color32> {
    let h = s.trim().strip_prefix('#')?;
    let d: Vec<u8> = h
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    let (r, g, b, a) = match d.len() {
        3 => (d[0] * 17, d[1] * 17, d[2] * 17, 255),
        4 => (d[0] * 17, d[1] * 17, d[2] * 17, d[3] * 17),
        6 => (d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5], 255),
        8 => (
            d[0] * 16 + d[1],
            d[2] * 16 + d[3],
            d[4] * 16 + d[5],
            d[6] * 16 + d[7],
        ),
        _ => return None,
    };
    Some(Color32::from_rgba_unmultiplied(r, g, b, a))
}

/// `#RRGGBB`, or `#RRGGBBAA` for a see-through colour.
pub fn color_hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

const SERIF_HINTS: &[&str] = &[
    "serif", "georgia", "times", "garamond", "baskerville", "charter", "palatino", "new york",
    "didot", "bodoni", "caslon", "merriweather", "playfair", "lora", "cambria",
];

/// Like [`face_named`], trying font files in the theme's folder first (in
/// `fonts/` or anywhere else, as `@font-face` rules may point). A CSS list
/// such as `-apple-system-ui-serif, ui-serif` uses its first known name.
pub fn face_named_in(name: &str, theme_dir: Option<&Path>) -> Face {
    let names: Vec<&str> = name
        .split(',')
        .map(|n| n.trim().trim_matches(|c| c == '"' || c == '\''))
        .filter(|n| !n.is_empty())
        .collect();
    let families = theme_dir
        .filter(|d| d.is_dir())
        .map(crate::fonts::families_in)
        .unwrap_or_default();
    for n in &names {
        if let Some(f) = families.iter().find(|f| f.name.eq_ignore_ascii_case(n)) {
            return crate::fonts::face_for(f);
        }
        if let Some(face) = css_generic(n) {
            return face;
        }
        if crate::fonts::catalog().is_some_and(|c| c.iter().any(|f| f.name.eq_ignore_ascii_case(n))) {
            return face_named(n);
        }
    }
    face_named(names.first().copied().unwrap_or(""))
}

/// CSS generic and system font keywords.
fn css_generic(name: &str) -> Option<Face> {
    match name.to_ascii_lowercase().as_str() {
        "system" | "system-ui" | "-apple-system" | "blinkmacsystemfont" | "ui-sans-serif"
        | "sans" | "sans-serif" | "ui-rounded" | "ui-monospace" | "monospace" => Some(Face::SANS),
        "serif" | "ui-serif" | "-apple-system-ui-serif" => Some(Face::SERIF),
        _ => None,
    }
}

/// The name `presets.json` uses: a CSS font-family value.
pub fn css_font_name(face: Face) -> String {
    if face == Face::SANS {
        "system-ui".into()
    } else if face == Face::SERIF {
        "ui-serif".into()
    } else {
        face.name()
    }
}

/// The face for a font family name: an installed family when the font list
/// is ready, otherwise the built-in serif or sans.
pub fn face_named(name: &str) -> Face {
    let n = name.trim();
    let lower = n.to_ascii_lowercase();
    if lower.is_empty() {
        return Face::SANS;
    }
    if let Some(face) = css_generic(&lower) {
        return face;
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
        && a.chart_colors == b.chart_colors
        && a.accent1 == b.accent1
        && a.dark_accents == b.dark_accents
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
    // "new york", "newyork" and "New York" all name the same folder.
    let want = css_slug(template);
    for root in ia_theme_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if is_theme_dir(&p)
                && p.file_name()
                    .is_some_and(|n| css_slug(&n.to_string_lossy()) == want)
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
        assert!(ia_theme_dir("newyork").is_some() && ia_theme_dir("New York").is_some());
        let helv = ia_theme_dir("helvetica").unwrap();
        assert!(load_variant(&helv, Some("Dark")).unwrap().starts_dark);
        assert!(!load_variant(&helv, Some("Light")).unwrap().starts_dark);
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pc-theme-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn new_theme_folder_has_ia_structure() {
        let dir = scratch("new");
        let mut p = crate::theme::presets()[5].clone();
        p.custom = true;
        save(&dir, &p, false, None).unwrap();
        let t = Json::parse(&std::fs::read_to_string(dir.join(TEMPLATE)).unwrap()).unwrap();
        assert_eq!(t.str("Css"), Some("studio.css"));
        assert!(dir.join("studio.css").is_file());
        assert!(dir.join(PREVIEW_WEBP).is_file());
        let png = image::open(dir.join(PREVIEW_PNG)).unwrap();
        assert_eq!((png.width(), png.height()), (500, 281));
        for key in ["Version", "Author", "ShortDescription", "LongDescription", "LayoutExamples"] {
            assert!(t.get(key).is_some(), "{key}");
        }
        let back = load(&dir).unwrap();
        assert!(same_look(&back, &p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_an_ia_theme_keeps_its_files_and_keys() {
        let src = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes/New York");
        if !src.is_dir() {
            return;
        }
        let dir = scratch("ny");
        let mut p = load(src).unwrap();
        save(&dir, &p, false, Some(src)).unwrap();
        for f in ["newyork.css", "image2.webp", "logo.png", "fonts/Inter-Regular.woff2"] {
            assert!(dir.join(f).is_file(), "{f}");
        }
        // iA's own preview art is kept, not redrawn
        assert_eq!(
            std::fs::read(dir.join(PREVIEW_PNG)).unwrap(),
            std::fs::read(src.join(PREVIEW_PNG)).unwrap()
        );
        let t = Json::parse(&std::fs::read_to_string(dir.join(TEMPLATE)).unwrap()).unwrap();
        assert_eq!(t.str("Css"), Some("newyork.css"));
        assert_eq!(t.str("TitleFont"), Some("Inter"));
        assert!(t.str("LongDescription").unwrap().contains("Inter"));
        assert_eq!(t.arr("LayoutExamples").len(), 2);
        let pr = Json::parse(&std::fs::read_to_string(dir.join(PRESETS)).unwrap()).unwrap();
        let v = &pr.arr("Presets")[0];
        assert_eq!(v.str("Name"), Some("Default"));
        assert_eq!(v.str("DarkAccent1"), Some("#FFC400"));
        assert_eq!(v.str("DarkBodyTextColor"), Some("#000"));
        assert_eq!(v.str("TitleFont"), Some("inter"));
        // a changed colour is written; the rest stay as they were
        p.light.bg = Color32::from_rgb(0xFF, 0xF4, 0xD6);
        save(&dir, &p, false, Some(&dir)).unwrap();
        let pr = Json::parse(&std::fs::read_to_string(dir.join(PRESETS)).unwrap()).unwrap();
        let v = &pr.arr("Presets")[0];
        assert_eq!(v.str("LightBackgroundColor"), Some("#FFF4D6"));
        assert_eq!(v.str("DarkBodyTextColor"), Some("#000"));
        assert!(same_look(&load(&dir).unwrap(), &p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_preview_samples() {
        // Leaves previews in the temp folder to look at by eye.
        for (i, name) in [(6, "midnight"), (7, "editorial"), (9, "sunset")] {
            let dir = std::env::temp_dir().join(format!("pc-preview-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            save(&dir, &crate::theme::presets()[i], false, None).unwrap();
        }
    }

    #[test]
    fn reads_san_francisco_gradients_and_chart_colors() {
        let src = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes/San Francisco");
        if !src.is_dir() {
            return;
        }
        let p = load(src).unwrap();
        assert_eq!(p.gradients[0].len(), 11);
        assert_eq!(p.gradients[0][0], Color32::from_rgb(0x00, 0xA8, 0xFF));
        assert_eq!(p.gradients[1][10], Color32::from_rgb(0x9D, 0x79, 0x00));
        assert_eq!(p.light.title, Color32::WHITE);
        assert_eq!(p.chart_colors[0].len(), 7);
        assert_eq!(p.chart_colors[0][1], Color32::from_rgb(0xFF, 0xC4, 0x00));
        assert!(p.chart_colors[1].is_empty());
        // saving keeps "transparent", the gradients and the chart colours
        let dir = scratch("sf");
        save(&dir, &p, false, Some(src)).unwrap();
        let pr = Json::parse(&std::fs::read_to_string(dir.join(PRESETS)).unwrap()).unwrap();
        let v = &pr.arr("Presets")[0];
        assert_eq!(v.str("LightBackgroundColor"), Some("transparent"));
        assert_eq!(v.arr("LightBgGradient").len(), 11);
        assert_eq!(v.get("LightGraphDataColors").and_then(|g| g.str("data-1")), Some("#FFC400"));
        assert!(same_look(&load(&dir).unwrap(), &p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_installed_ia_theme_loads_and_round_trips() {
        let root = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes");
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for e in entries.flatten() {
            let src = e.path();
            let name = src.file_name().unwrap().to_string_lossy().into_owned();
            let p = load(&src).unwrap_or_else(|err| panic!("{name}: {err}"));
            // Themes that ship TrueType fonts get them, not a stand-in.
            let has_ttf = !crate::fonts::families_in(&src).is_empty();
            if has_ttf {
                assert!(p.title_face.0 >= 2, "{name}: title font not loaded from fonts/");
            }
            let dir = scratch(&format!("all-{}", css_slug(&name)));
            save(&dir, &p, p.starts_dark, Some(&src)).unwrap();
            let back = load(&dir).unwrap();
            assert!(same_look(&back, &p), "{name}: changed after saving");
            let layouts = |d: &Path| {
                Json::parse(&std::fs::read_to_string(d.join(TEMPLATE)).unwrap())
                    .unwrap()
                    .get("Layouts")
                    .cloned()
            };
            assert_eq!(layouts(&dir), layouts(&src), "{name}: Layouts changed");
            // Unchanged themes are written back exactly.
            assert_eq!(
                Json::parse(&std::fs::read_to_string(dir.join(PRESETS)).unwrap()).unwrap(),
                Json::parse(&std::fs::read_to_string(src.join(PRESETS)).unwrap()).unwrap(),
                "{name}: presets.json changed"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn follows_ia_theme_documentation() {
        // Inverted titles live in "Layouts", as in Basel.
        let mut p = crate::theme::presets()[1].clone(); // Contrast Serif inverts titles
        assert!(p.invert_titles);
        p.name = "My Theme";
        let dir = scratch("doc");
        save(&dir, &p, false, None).unwrap();
        let t = Json::parse(&std::fs::read_to_string(dir.join(TEMPLATE)).unwrap()).unwrap();
        assert_eq!(t.str("Css"), Some("my theme.css"));
        assert!(dir.join("my theme.css").is_file());
        assert_eq!(t.arr("Layouts").len(), 2);
        assert!(!t.str("CssClasses").unwrap().contains("invert"));
        // Display names in template.json, CSS names in presets.json.
        assert_eq!(t.str("TitleFont"), Some("Serif"));
        let pr = Json::parse(&std::fs::read_to_string(dir.join(PRESETS)).unwrap()).unwrap();
        assert_eq!(pr.arr("Presets")[0].str("TitleFont"), Some("ui-serif"));
        assert!(load(&dir).unwrap().invert_titles);
        // Turning it off removes the entries again.
        p.invert_titles = false;
        save(&dir, &p, false, Some(&dir)).unwrap();
        let t = Json::parse(&std::fs::read_to_string(dir.join(TEMPLATE)).unwrap()).unwrap();
        assert!(t.arr("Layouts").is_empty());
        assert!(!load(&dir).unwrap().invert_titles);
        // CSS font lists.
        assert_eq!(face_named_in("-apple-system-ui-serif, ui-serif", None), Face::SERIF);
        assert_eq!(face_named_in("system-ui", None), Face::SANS);
        let _ = std::fs::remove_dir_all(&dir);
        let basel = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes/Basel");
        if basel.is_dir() {
            assert!(load(basel).unwrap().invert_titles);
        }
    }

    #[test]
    fn reads_see_through_colours_and_per_look_accents() {
        assert_eq!(
            parse_color("#000000b3"),
            Some(Color32::from_rgba_unmultiplied(0, 0, 0, 0xB3))
        );
        assert_eq!(color_hex(parse_color("#ffffffe6").unwrap()), "#FFFFFFE6");
        assert_eq!(color_hex(parse_color("#FFF").unwrap()), "#FFFFFF");
        let ny = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes/New York");
        if ny.is_dir() {
            let p = load(ny).unwrap();
            let yellow = Some(Color32::from_rgb(0xFF, 0xC4, 0x00));
            assert_eq!(p.accent1, [yellow, yellow]);
        }
    }

    #[test]
    fn loads_ttf_fonts_shipped_with_a_theme() {
        let src = Path::new("/Applications/iA Presenter.app/Contents/Resources/Themes/Copenhagen");
        if !src.is_dir() {
            return;
        }
        let fams = crate::fonts::families_in(&src.join("fonts"));
        assert!(!fams.is_empty(), "Copenhagen ships .ttf fonts");
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
