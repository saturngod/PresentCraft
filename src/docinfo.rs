//! `info.json`: document settings, in iA Presenter's layout.
//!
//! iA Presenter writes:
//!
//! ```json
//! { "creatorIdentifier": "net.ia.presenter", "type": "net.daringfireball.markdown",
//!   "version": 2, "transient": false,
//!   "net.ia.presenter": { "template": "tokyo", "preset": "Default",
//!                         "logo": "/assets/logo.png", "headertrailing": "logo" } }
//! ```
//!
//! PresentCraft writes the same top-level keys, keeps its own settings
//! under `"app.myanmars.presentcraft"`, and keeps the `"net.ia.presenter"` block's
//! logo and header/footer places up to date so iA Presenter can read them.
//! Other keys in either block are left as they were. The first version's
//! flat keys (`"Theme"`, `"Logo"`, …) are still read.

use crate::json::{obj, Json};
use crate::theme::{Slot, SlotKind};
use egui::Color32;

pub const NS: &str = "app.myanmars.presentcraft";
pub const IA_NS: &str = "net.ia.presenter";

/// iA Presenter's keys for the six header and footer places, in
/// [`crate::theme::SLOT_NAMES`] order.
const IA_SLOTS: [&str; 6] = [
    "headerleading",
    "headermiddle",
    "headertrailing",
    "footerleading",
    "footermiddle",
    "footertrailing",
];

/// Settings found in an `info.json`; `None` means "not set".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocSettings {
    /// A PresentCraft theme name.
    pub theme: Option<String>,
    /// iA Presenter's theme folder name and preset name.
    pub ia_template: Option<String>,
    pub ia_preset: Option<String>,
    pub dark: Option<bool>,
    pub author: Option<String>,
    pub audience: Option<String>,
    pub logo: Option<String>,
    pub logo_size: Option<f32>,
    pub slots: Option<[Slot; 6]>,
    /// Deck background gradient for light and dark slides.
    pub gradients: [Option<(bool, Vec<Color32>)>; 2],
}

pub fn read(info: &Json) -> DocSettings {
    let mut d = DocSettings::default();
    let ia = info.get(IA_NS);
    let ours = info.get(NS);
    // Newest first: our block, then the first version's flat keys.
    let pick = |key: &str, old: &str| -> Option<&Json> {
        ours.and_then(|o| o.get(key)).or_else(|| info.get(old))
    };
    let text = |v: Option<&Json>| match v {
        Some(Json::Str(s)) => Some(s.clone()),
        _ => None,
    };
    d.theme = text(pick("theme", "Theme")).filter(|s| !s.trim().is_empty());
    d.dark = text(pick("appearance", "Appearance")).map(|a| a.eq_ignore_ascii_case("dark"));
    d.author = text(pick("author", "Author"));
    d.audience = text(pick("audience", "Audience"));
    d.logo = text(pick("logo", "Logo")).or_else(|| ia.and_then(|i| i.str("logo")).map(str::to_string));
    d.logo_size = match pick("logoSize", "LogoSize") {
        Some(Json::Num(n)) => Some((*n as f32).clamp(0.01, 0.5)),
        // iA Presenter: percent of the slide height, written as text.
        _ => ia.and_then(|i| i.num("logo-size")).map(|p| (p as f32 / 100.0).clamp(0.01, 0.5)),
    };
    if let Some(Json::Arr(list)) = pick("slots", "Slots") {
        let mut slots: [Slot; 6] = Default::default();
        for (slot, j) in slots.iter_mut().zip(list) {
            let kind = j.str("kind").or_else(|| j.str("Kind")).unwrap_or("");
            slot.kind = SlotKind::ALL
                .into_iter()
                .find(|k| format!("{k:?}").eq_ignore_ascii_case(kind))
                .unwrap_or_default();
            slot.text = j
                .str("text")
                .or_else(|| j.str("Text"))
                .unwrap_or_default()
                .to_string();
        }
        d.slots = Some(slots);
    } else if let Some(ia) = ia {
        if IA_SLOTS.iter().any(|k| ia.get(k).is_some()) {
            d.slots = Some(std::array::from_fn(|i| {
                ia.str(IA_SLOTS[i]).map(slot_from_ia).unwrap_or_default()
            }));
        }
    }
    if let Some(g) = pick("deckGradient", "DeckGradient") {
        for (i, look) in ["light", "dark"].into_iter().enumerate() {
            let v = g.get(look).or_else(|| g.get(&capitalised(look)));
            let Some(v) = v else { continue };
            let enabled = v.bool("enabled").or_else(|| v.bool("Enabled")).unwrap_or(false);
            let colors_json = match v.get("colors").or_else(|| v.get("Colors")) {
                Some(Json::Arr(a)) => a.as_slice(),
                _ => &[],
            };
            let colors: Vec<Color32> = colors_json
                .iter()
                .filter_map(|c| match c {
                    Json::Str(s) => crate::themefile::parse_color(s),
                    _ => None,
                })
                .collect();
            d.gradients[i] = Some((enabled, colors));
        }
    }
    if let Some(ia) = ia {
        d.ia_template = ia.str("template").map(str::to_string).filter(|s| !s.is_empty());
        d.ia_preset = ia.str("preset").map(str::to_string).filter(|s| !s.is_empty());
    }
    d
}

/// The full settings to write. Every field is used, so build it with all
/// values set.
pub fn write(base: Option<&Json>, d: &DocSettings) -> Json {
    let mut info = match base {
        Some(j @ Json::Obj(_)) => j.clone(),
        _ => Json::Obj(Vec::new()),
    };
    // The first version's flat keys now live in our block.
    for old in [
        "Format",
        "FormatVersion",
        "Creator",
        "Theme",
        "Appearance",
        "Author",
        "Audience",
        "Logo",
        "LogoSize",
        "Slots",
        "DeckGradient",
    ] {
        info.remove(old);
    }
    info.set("creatorIdentifier", NS.into());
    info.set("type", "net.daringfireball.markdown".into());
    info.set("version", 2usize.into());
    info.set("transient", false.into());

    let mut ours = match info.get(NS) {
        Some(j @ Json::Obj(_)) => j.clone(),
        _ => Json::Obj(Vec::new()),
    };
    ours.set(
        "creator",
        format!("PresentCraft {}", env!("CARGO_PKG_VERSION")).into(),
    );
    if let Some(t) = &d.theme {
        ours.set("theme", t.clone().into());
    }
    let dark = d.dark.unwrap_or(false);
    ours.set("appearance", if dark { "dark" } else { "light" }.into());
    ours.set("author", d.author.clone().unwrap_or_default().into());
    ours.set("audience", d.audience.clone().unwrap_or_default().into());
    let logo = d.logo.clone().unwrap_or_default();
    ours.set("logo", logo.clone().into());
    ours.set("logoSize", (d.logo_size.unwrap_or(0.05) as f64).into());
    let slots = d.slots.clone().unwrap_or_default();
    ours.set(
        "slots",
        Json::Arr(
            slots
                .iter()
                .map(|s| {
                    obj([
                        ("kind", format!("{:?}", s.kind).into()),
                        ("text", s.text.clone().into()),
                    ])
                })
                .collect(),
        ),
    );
    let gradients = [0, 1].map(|i| {
        let (on, colors) = d.gradients[i].clone().unwrap_or_default();
        (
            if i == 0 { "light" } else { "dark" },
            obj([
                ("enabled", on.into()),
                (
                    "colors",
                    Json::Arr(
                        colors
                            .iter()
                            .map(|c| crate::themefile::color_hex(*c).into())
                            .collect(),
                    ),
                ),
            ]),
        )
    });
    ours.set("deckGradient", obj(gradients));
    info.set(NS, ours);

    // iA Presenter's own block: logo and header/footer places.
    let mut ia = match info.get(IA_NS) {
        Some(j @ Json::Obj(_)) => j.clone(),
        _ => Json::Obj(Vec::new()),
    };
    if logo.is_empty() {
        ia.remove("logo");
    } else {
        ia.set("logo", logo.into());
    }
    if let Some(size) = d.logo_size {
        // Percent with at most one decimal, as text: "7.5".
        let pct = (size as f64 * 1000.0).round() / 10.0;
        ia.set("logo-size", format!("{pct}").into());
    }
    for (key, slot) in IA_SLOTS.iter().zip(&slots) {
        match slot_to_ia(slot) {
            Some(v) => ia.set(key, v.into()),
            None => ia.remove(key),
        }
    }
    if matches!(&ia, Json::Obj(items) if !items.is_empty()) {
        info.set(IA_NS, ia);
    } else {
        info.remove(IA_NS);
    }
    info
}

/// Points the settings at a new logo reference (after bundling).
pub fn set_logo(info: &mut Json, logo: &str) {
    for ns in [NS, IA_NS] {
        if let Some(mut block) = info.get(ns).cloned() {
            if ns == NS || !logo.is_empty() {
                block.set("logo", logo.into());
                info.set(ns, block);
            }
        }
    }
}

fn slot_from_ia(v: &str) -> Slot {
    let kind = match v.trim().to_ascii_lowercase().as_str() {
        "" | "none" | "empty" => SlotKind::Empty,
        "logo" => SlotKind::Logo,
        "date" => SlotKind::Date,
        "slidenumber" | "slide number" | "number" | "pagenumber" => SlotKind::Number,
        _ => {
            return Slot {
                kind: SlotKind::Custom,
                text: v.to_string(),
            }
        }
    };
    Slot {
        kind,
        text: String::new(),
    }
}

fn slot_to_ia(s: &Slot) -> Option<String> {
    match s.kind {
        SlotKind::Empty => None,
        SlotKind::Logo => Some("logo".into()),
        SlotKind::Date => Some("date".into()),
        SlotKind::Number => Some("slideNumber".into()),
        SlotKind::Custom => Some(s.text.trim().to_string()).filter(|t| !t.is_empty()),
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IA: &str = r#"{
      "creatorIdentifier" : "net.ia.presenter",
      "net.ia.presenter" : {
        "headertrailing" : "logo",
        "logo" : "\/assets\/presentcraft.png",
        "logo-size" : "7.5",
        "preset" : "Default",
        "template" : "tokyo"
      },
      "transient" : false,
      "type" : "net.daringfireball.markdown",
      "version" : 2
    }"#;

    #[test]
    fn reads_ia_presenter_info() {
        let d = read(&Json::parse(IA).unwrap());
        assert_eq!(d.logo.as_deref(), Some("/assets/presentcraft.png"));
        assert_eq!(d.logo_size, Some(0.075));
        assert_eq!(d.ia_template.as_deref(), Some("tokyo"));
        assert_eq!(d.ia_preset.as_deref(), Some("Default"));
        let slots = d.slots.unwrap();
        assert_eq!(slots[2].kind, SlotKind::Logo);
        assert_eq!(slots[0].kind, SlotKind::Empty);
        assert_eq!(d.theme, None);
    }

    #[test]
    fn writes_ia_layout_and_keeps_ia_keys() {
        let base = Json::parse(IA).unwrap();
        let mut d = read(&base);
        d.theme = Some("Midnight".into());
        d.dark = Some(true);
        let mut slots = d.slots.clone().unwrap();
        slots[5] = Slot {
            kind: SlotKind::Custom,
            text: "ACME".into(),
        };
        slots[2] = Slot::default();
        d.slots = Some(slots);
        let out = write(Some(&base), &d);
        assert_eq!(out.str("creatorIdentifier"), Some(NS));
        assert_eq!(out.num("version"), Some(2.0));
        let ia = out.get(IA_NS).unwrap();
        assert_eq!(ia.str("template"), Some("tokyo"));
        assert_eq!(ia.str("footertrailing"), Some("ACME"));
        assert_eq!(ia.get("headertrailing"), None);
        assert_eq!(ia.str("logo-size"), Some("7.5"));
        // reading back gives the same settings
        let back = read(&out);
        assert_eq!(back.theme.as_deref(), Some("Midnight"));
        assert_eq!(back.dark, Some(true));
        assert_eq!(back.slots, d.slots);
        assert_eq!(back.logo, d.logo);
    }

    #[test]
    fn still_reads_first_version_keys() {
        let v1 = Json::parse(
            r#"{ "Theme": "Paper", "Appearance": "dark", "Logo": "/assets/l.png",
                 "Slots": [ {"Kind": "Number", "Text": ""} ] }"#,
        )
        .unwrap();
        let d = read(&v1);
        assert_eq!(d.theme.as_deref(), Some("Paper"));
        assert_eq!(d.dark, Some(true));
        assert_eq!(d.logo.as_deref(), Some("/assets/l.png"));
        assert_eq!(d.slots.unwrap()[0].kind, SlotKind::Number);
        let out = write(Some(&v1), &read(&v1));
        assert!(out.get("Theme").is_none() && out.get("Logo").is_none());
    }
}
