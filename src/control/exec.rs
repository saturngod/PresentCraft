//! Runs control tools against the open window. Everything here runs on the
//! UI thread, so it can change `App` directly.

use super::script::{self, Edit};
use super::server::{self, Req};
use super::tools;
use super::wire::{self, base64, obj, text};
use crate::json::Json;
use crate::theme::{Palette, Slot, SlotKind};
use crate::{export, fonts, package, themefile, App, FontChoice, LINE_HEIGHTS, SIZES};
use eframe::egui;
use egui::Color32;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Tools that change the script or the design. Each call is one undo step.
const CHANGES: &[&str] = &[
    "deck_write_markdown",
    "slide_add",
    "slide_update",
    "slide_delete",
    "slide_move",
    "slide_duplicate",
    "insert_picture",
    "insert_table",
    "title_align",
    "theme_apply",
    "theme_import",
    "appearance_set",
    "palette_set",
    "background_set",
    "font_set",
    "size_set",
    "line_height_set",
    "document_info_set",
    "header_footer_set",
    "logo_set",
    "logo_size",
    "logo_remove",
    "settings_set",
];

/// Tools that replace or remove content and need `confirm: true`.
const CONFIRMED: &[&str] = &["deck_write_markdown", "slide_delete", "media_remove"];

/// Everything an undo has to bring back.
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    text: String,
    settings: crate::settings::Settings,
    preset_idx: usize,
    dark: bool,
    light_override: Option<Palette>,
    dark_override: Option<Palette>,
    deck_gradient: [bool; 2],
    gradient_colors: [Vec<Color32>; 2],
    title_font: FontChoice,
    body_font: FontChoice,
    size_idx: usize,
    line_idx: usize,
    slots: [Slot; 6],
    logo: String,
    logo_size: f32,
    author: String,
    audience: String,
}

/// One AI change: the tool, the state before it, and the state after it.
/// An undo runs only while the state still matches `after`, so a change made
/// by hand since then is never lost.
pub struct Entry {
    pub tool: String,
    before: Snapshot,
    after: Snapshot,
}

/// A slide screenshot waiting for the next frame.
pub struct PendingShot {
    pub reply: std::sync::mpsc::Sender<Result<Json, String>>,
    pub started: Instant,
    pub slide: usize,
}

pub enum Outcome {
    Ready(Result<Json, String>),
    /// Screenshot of this slide (0-based), sent when the frame is drawn.
    Shot(usize),
}

fn str_arg(a: &Json, key: &str) -> Option<String> {
    match a.get(key)? {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn need(a: &Json, key: &str) -> Result<String, String> {
    str_arg(a, key).ok_or_else(|| format!("{key} is required."))
}

fn parse_hex(s: &str) -> Result<Color32, String> {
    themefile::parse_color(s)
        .ok_or_else(|| format!("{s} is not a colour. Use hex, such as #1C1C1E."))
}

fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn font_label(c: FontChoice) -> String {
    match c {
        FontChoice::Theme => "theme".into(),
        FontChoice::Sans => "sans".into(),
        FontChoice::Serif => "serif".into(),
        FontChoice::Named(f) => themefile::face_name(f),
    }
}

fn size_label(i: usize) -> &'static str {
    SIZES.get(i).map(|s| s.0).unwrap_or("")
}

/// Encodes a window capture as PNG.
fn encode_png(image: &egui::ColorImage) -> Result<Vec<u8>, String> {
    let [w, h] = image.size;
    let mut buf = Vec::with_capacity(w * h * 4);
    for p in &image.pixels {
        buf.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
    }
    let img = image::RgbaImage::from_raw(w as u32, h as u32, buf)
        .ok_or_else(|| "The screenshot has the wrong size.".to_string())?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

impl App {
    /// Starts the server for File > MCP > Start Server.
    pub(crate) fn control_start(&mut self, ctx: &egui::Context) {
        if self.control.running.is_some() {
            return;
        }
        match server::start(ctx.clone()) {
            Ok(running) => {
                self.status = format!("MCP server started on port {}", running.port);
                self.control.running = Some(running);
            }
            Err(e) => self.status = format!("Couldn't start the MCP server: {e}"),
        }
    }

    /// Stops the server for File > MCP > Stop Server.
    pub(crate) fn control_stop(&mut self) {
        if let Some(running) = self.control.running.take() {
            server::stop(running);
            self.status = "MCP server stopped".into();
        }
    }

    /// Runs the requests the server queued. Called at the start of each frame.
    pub(crate) fn control_drain(&mut self, ctx: &egui::Context) {
        if let Some(pending) = self.control.shot.take() {
            if let Some(image) = crate::take_shot() {
                let reply = encode_png(&image).map(|png| {
                    obj(vec![
                        ("slide", Json::Num((pending.slide + 1) as f64)),
                        ("png_base64", text(&base64(&png))),
                    ])
                });
                let _ = pending.reply.send(reply);
            } else if pending.started.elapsed() > Duration::from_secs(10) {
                let _ = pending.reply.send(Err("The screenshot timed out.".into()));
            } else {
                self.control.shot = Some(pending);
                ctx.request_repaint();
            }
        }
        let reqs: Vec<Req> = match &self.control.running {
            Some(running) => running.rx.try_iter().collect(),
            None => return,
        };
        for req in reqs {
            match self.control_exec(ctx, &req.tool, &req.args) {
                Outcome::Ready(result) => {
                    let _ = req.reply.send(result);
                }
                Outcome::Shot(slide) => {
                    if self.control.shot.is_some() {
                        let _ = req
                            .reply
                            .send(Err("Another screenshot is still being taken.".into()));
                    } else {
                        self.control.shot = Some(PendingShot {
                            reply: req.reply,
                            started: Instant::now(),
                            slide,
                        });
                        crate::request_shot(ctx);
                    }
                }
            }
        }
    }

    pub(crate) fn control_exec(&mut self, ctx: &egui::Context, tool: &str, a: &Json) -> Outcome {
        if !tools::exists(tool) {
            return Outcome::Ready(Err(format!("There is no tool called {tool}.")));
        }
        if tool == "slide_screenshot" {
            return match self.slide_no(a, "slide") {
                Ok(i) => {
                    self.select_slide(i);
                    Outcome::Shot(i)
                }
                Err(e) => Outcome::Ready(Err(e)),
            };
        }
        let changes = CHANGES.contains(&tool);
        let before = changes.then(|| self.snapshot());
        let result = self.run_tool(ctx, tool, a);
        if let Some(before) = before {
            match &result {
                Ok(_) => {
                    let after = self.snapshot();
                    if after != before {
                        self.control.undo.push(Entry {
                            tool: tool.to_string(),
                            before,
                            after,
                        });
                        if self.control.undo.len() > 100 {
                            self.control.undo.remove(0);
                        }
                        self.status = format!("AI changed the presentation ({tool})");
                    }
                }
                Err(_) => {
                    if self.snapshot() != before {
                        self.restore(before);
                    }
                }
            }
        }
        Outcome::Ready(result)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            settings: self.settings.clone(),
            preset_idx: self.preset_idx,
            dark: self.dark,
            light_override: self.light_override,
            dark_override: self.dark_override,
            deck_gradient: self.deck_gradient,
            gradient_colors: self.gradient_colors.clone(),
            title_font: self.title_font,
            body_font: self.body_font,
            size_idx: self.size_idx,
            line_idx: self.line_idx,
            slots: self.slots.clone(),
            logo: self.logo.clone(),
            logo_size: self.logo_size,
            author: self.author.clone(),
            audience: self.audience.clone(),
        }
    }

    fn restore(&mut self, s: Snapshot) {
        self.text = s.text;
        self.settings = s.settings;
        self.preset_idx = s.preset_idx;
        self.dark = s.dark;
        self.light_override = s.light_override;
        self.dark_override = s.dark_override;
        self.deck_gradient = s.deck_gradient;
        self.gradient_colors = s.gradient_colors;
        self.title_font = s.title_font;
        self.body_font = s.body_font;
        self.size_idx = s.size_idx;
        self.line_idx = s.line_idx;
        self.slots = s.slots;
        self.logo = s.logo;
        self.logo_size = s.logo_size;
        self.author = s.author;
        self.audience = s.audience;
        self.dirty = true;
        self.reparse();
    }

    /// Slide number (1-based in the tools) to a 0-based index in the deck.
    fn slide_no(&self, a: &Json, key: &str) -> Result<usize, String> {
        let n = self.slides.len();
        match a.num(key) {
            Some(v) if v >= 1.0 && v.fract() == 0.0 && (v as usize) <= n => Ok(v as usize - 1),
            _ => Err(format!("{key} must be a slide number from 1 to {n}.")),
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        self.dirty = true;
        self.reparse();
        let len = self.text.chars().count();
        self.cursor = self.cursor.min(len);
        self.sel_start = self.sel_start.min(len);
    }

    /// A path the tools may write to or read from: absolute, with no `..`,
    /// inside the home folder, the temporary folder or the presentation's folder.
    fn user_path(&self, p: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(p);
        let folder = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);
        wire::check_path(&path, folder.as_deref())?;
        Ok(path)
    }

    /// A picture reference: a local file is copied into the presentation.
    fn resolve_src(&mut self, src: &str) -> Result<String, String> {
        let p = Path::new(src);
        if p.is_absolute() && p.is_file() {
            let folder = self
                .path
                .as_ref()
                .and_then(|f| f.parent())
                .map(Path::to_path_buf);
            wire::check_path(p, folder.as_deref())?;
            if !package::is_media_file(p) {
                return Err(format!("{src} is not a picture or video file."));
            }
            return Ok(self.media_ref(p));
        }
        if src.starts_with("/assets/") || src.starts_with("http://") || src.starts_with("https://")
        {
            return Ok(src.to_string());
        }
        if !p.is_absolute() && !src.trim().is_empty() {
            return Ok(src.to_string());
        }
        Err(format!("There is no file at {src}."))
    }

    fn run_tool(&mut self, ctx: &egui::Context, tool: &str, a: &Json) -> Result<Json, String> {
        if CONFIRMED.contains(&tool) && a.bool("confirm") != Some(true) {
            return Err(format!(
                "{tool} changes or removes content. Pass confirm: true to go ahead."
            ));
        }
        if let Some(mut edit) = script::parse_edit(tool, a)? {
            if let Edit::Picture { src, .. } = &mut edit {
                *src = self.resolve_src(src)?;
            }
            let new_text = script::apply(&self.text, &edit)?;
            self.set_text(new_text);
            let n = self.slides.len().max(1);
            let focus = match &edit {
                Edit::Add { at, .. } => at.unwrap_or(n - 1),
                Edit::Update { index, .. }
                | Edit::Picture { index, .. }
                | Edit::Table { index, .. }
                | Edit::TitleAlign { index, .. }
                | Edit::Delete { index } => *index,
                Edit::Move { to, .. } => *to,
                Edit::Duplicate { index } => index + 1,
                Edit::Replace(_) => self.current,
            }
            .min(n - 1);
            self.select_slide(focus);
            return Ok(obj(vec![
                ("slides", Json::Num(n as f64)),
                ("slide", Json::Num((focus + 1) as f64)),
            ]));
        }
        match tool {
            "deck_info" => Ok(self.deck_info()),
            "deck_read_markdown" => Ok(obj(vec![("markdown", text(&self.text))])),
            "deck_check" => Ok(self.deck_check()),
            "slide_list" => {
                let items = script::bodies(&self.text)
                    .iter()
                    .enumerate()
                    .map(|(i, body)| {
                        let picture = body
                            .lines()
                            .any(|l| l.trim_start().starts_with("![") || l.starts_with("/assets/"));
                        obj(vec![
                            ("slide", Json::Num((i + 1) as f64)),
                            ("title", text(&script::title_of(body))),
                            ("picture", Json::Bool(picture)),
                        ])
                    })
                    .collect();
                Ok(Json::Arr(items))
            }
            "slide_get" => {
                let i = self.slide_no(a, "slide")?;
                let body = script::bodies(&self.text).swap_remove(i);
                Ok(obj(vec![
                    ("slide", Json::Num((i + 1) as f64)),
                    ("markdown", text(&body)),
                ]))
            }
            "slide_select" => {
                let i = self.slide_no(a, "slide")?;
                self.select_slide(i);
                Ok(obj(vec![("slide", Json::Num((i + 1) as f64))]))
            }
            "undo_ai_change" => {
                let Some(entry) = self.control.undo.pop() else {
                    return Err("There is no AI change to undo.".into());
                };
                if self.snapshot() != entry.after {
                    self.control.undo.push(entry);
                    return Err("The presentation was changed by hand after the last AI change, so that change cannot be undone safely.".into());
                }
                let name = entry.tool.clone();
                self.restore(entry.before);
                Ok(obj(vec![("undone", text(&name))]))
            }
            "theme_list" => Ok(Json::Arr(
                self.presets
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        obj(vec![
                            ("number", Json::Num((i + 1) as f64)),
                            ("name", text(p.name)),
                            ("custom", Json::Bool(p.custom)),
                            ("starts_dark", Json::Bool(p.starts_dark)),
                        ])
                    })
                    .collect(),
            )),
            "theme_apply" => {
                let name = need(a, "name")?;
                let i = self
                    .presets
                    .iter()
                    .position(|p| p.name.eq_ignore_ascii_case(name.trim()))
                    .ok_or_else(|| format!("There is no theme called {name}. Use theme_list."))?;
                self.apply_preset(i);
                Ok(obj(vec![("theme", text(self.presets[i].name))]))
            }
            "theme_import" => {
                let path = self.user_path(&need(a, "path")?)?;
                let preset = themefile::import(&path)?;
                let name = preset.name;
                self.presets.push(preset);
                let i = self.presets.len() - 1;
                self.apply_preset(i);
                Ok(obj(vec![("theme", text(name))]))
            }
            "theme_export" => {
                let path = self.user_path(&need(a, "path")?)?;
                let preset = self.current_as_preset();
                themefile::save(&path, &preset, self.dark, None).map_err(|e| e.to_string())?;
                Ok(obj(vec![("saved", text(&path.display().to_string()))]))
            }
            "appearance_set" => {
                match need(a, "mode")?.as_str() {
                    "light" => {
                        self.settings.match_system = false;
                        self.settings.dark = false;
                        self.dark = false;
                    }
                    "dark" => {
                        self.settings.match_system = false;
                        self.settings.dark = true;
                        self.dark = true;
                    }
                    "system" => self.settings.match_system = true,
                    other => return Err(format!("{other} is not light, dark or system.")),
                }
                Ok(obj(vec![(
                    "appearance",
                    text(if self.dark { "dark" } else { "light" }),
                )]))
            }
            "palette_get" => {
                let dark = look_is_dark(a)?;
                let p = self.current_palette(dark);
                Ok(obj(vec![
                    ("look", text(if dark { "dark" } else { "light" })),
                    ("background", text(&themefile::color_hex(p.bg))),
                    ("title", text(&themefile::color_hex(p.title))),
                    ("body", text(&themefile::color_hex(p.body))),
                    (
                        "accents",
                        Json::Arr(
                            self.presets[self.preset_idx]
                                .accents
                                .iter()
                                .map(|c| text(&themefile::color_hex(*c)))
                                .collect(),
                        ),
                    ),
                ]))
            }
            "palette_set" => {
                let dark = look_is_dark(a)?;
                let mut p = self.current_palette(dark);
                if let Some(s) = str_arg(a, "background") {
                    p.bg = parse_hex(&s)?;
                }
                if let Some(s) = str_arg(a, "title") {
                    p.title = parse_hex(&s)?;
                }
                if let Some(s) = str_arg(a, "body") {
                    p.body = parse_hex(&s)?;
                }
                if dark {
                    self.dark_override = Some(p);
                } else {
                    self.light_override = Some(p);
                }
                Ok(obj(vec![(
                    "look",
                    text(if dark { "dark" } else { "light" }),
                )]))
            }
            "background_set" => {
                let dark = look_is_dark(a)?;
                let i = usize::from(dark);
                match need(a, "mode")?.as_str() {
                    "solid" => self.deck_gradient[i] = false,
                    "gradient" => {
                        let colors = script::strings(a, "colors")
                            .iter()
                            .map(|s| parse_hex(s))
                            .collect::<Result<Vec<_>, _>>()?;
                        if colors.len() < 2 {
                            return Err("A gradient needs at least two colours.".into());
                        }
                        self.gradient_colors[i] = colors;
                        self.deck_gradient[i] = true;
                    }
                    other => return Err(format!("{other} is not solid or gradient.")),
                }
                Ok(obj(vec![("mode", text(need(a, "mode")?.as_str()))]))
            }
            "font_set" => {
                let choice = match need(a, "face")?.trim() {
                    "theme" => FontChoice::Theme,
                    "sans" => FontChoice::Sans,
                    "serif" => FontChoice::Serif,
                    name => FontChoice::Named(themefile::face_named(name)),
                };
                let role = need(a, "role")?;
                match role.as_str() {
                    "title" => self.title_font = choice,
                    "body" => self.body_font = choice,
                    other => return Err(format!("{other} is not title or body.")),
                }
                fonts::begin_frame();
                Ok(obj(vec![("font", text(&font_label(choice)))]))
            }
            "size_set" => {
                let size = need(a, "size")?;
                self.size_idx = match size.as_str() {
                    "S" => 0,
                    "M" => 1,
                    "L" => 2,
                    "XL" => 3,
                    other => return Err(format!("{other} is not S, M, L or XL.")),
                };
                Ok(obj(vec![("size", text(size_label(self.size_idx)))]))
            }
            "line_height_set" => {
                let p = a.num("percent").ok_or("percent is required.")?;
                if !(80.0..=200.0).contains(&p) {
                    return Err("percent must be from 80 to 200.".into());
                }
                self.line_idx = crate::line_idx_for((p / 100.0) as f32);
                Ok(obj(vec![(
                    "line_height",
                    text(LINE_HEIGHTS[self.line_idx].0),
                )]))
            }
            "document_info_set" => {
                if let Some(v) = str_arg(a, "author") {
                    self.author = v;
                }
                if let Some(v) = str_arg(a, "audience") {
                    self.audience = v;
                }
                if let Some(v) = str_arg(a, "aspect") {
                    self.settings.aspect = match v.as_str() {
                        "16:9" => 0,
                        "16:10" => 1,
                        "4:3" => 2,
                        other => return Err(format!("{other} is not 16:9, 16:10 or 4:3.")),
                    };
                }
                Ok(obj(vec![(
                    "aspect",
                    text(crate::settings::ASPECTS[self.settings.aspect].0),
                )]))
            }
            "header_footer_get" => Ok(Json::Arr(
                self.slots
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        obj(vec![
                            ("place", Json::Num((i + 1) as f64)),
                            ("name", text(crate::theme::SLOT_NAMES[i])),
                            ("kind", text(slot_kind_name(s.kind))),
                            ("text", text(&s.text)),
                        ])
                    })
                    .collect(),
            )),
            "header_footer_set" => {
                let place = a
                    .num("place")
                    .filter(|v| (1.0..=6.0).contains(v) && v.fract() == 0.0)
                    .ok_or("place must be from 1 to 6.")? as usize;
                let kind = match need(a, "kind")?.as_str() {
                    "empty" => SlotKind::Empty,
                    "number" => SlotKind::Number,
                    "date" => SlotKind::Date,
                    "logo" => SlotKind::Logo,
                    "custom" => SlotKind::Custom,
                    other => return Err(format!("{other} is not a header or footer kind.")),
                };
                let slot = &mut self.slots[place - 1];
                slot.kind = kind;
                if let Some(t) = str_arg(a, "text") {
                    slot.text = t;
                }
                Ok(obj(vec![
                    ("place", Json::Num(place as f64)),
                    ("kind", text(slot_kind_name(kind))),
                ]))
            }
            "logo_set" => {
                let src = self.resolve_src(&need(a, "src")?)?;
                self.logo = src.clone();
                Ok(obj(vec![("logo", text(&src))]))
            }
            "logo_size" => {
                let s = a.num("size").ok_or("size is required.")?;
                if !(0.02..=0.08).contains(&s) {
                    return Err("size must be from 0.02 to 0.08.".into());
                }
                self.logo_size = s as f32;
                Ok(obj(vec![("size", Json::Num(s))]))
            }
            "logo_remove" => {
                self.logo.clear();
                Ok(obj(vec![("logo", text(""))]))
            }
            "media_list" => {
                let Some(root) = self.images.doc_dir.clone() else {
                    return Ok(Json::Arr(Vec::new()));
                };
                let items = package::list_assets(&root)
                    .iter()
                    .map(|file| {
                        obj(vec![
                            ("ref", text(&package::asset_ref(&root, file))),
                            (
                                "name",
                                text(&file.file_name().unwrap_or_default().to_string_lossy()),
                            ),
                            ("video", Json::Bool(package::is_video_file(file))),
                        ])
                    })
                    .collect();
                Ok(Json::Arr(items))
            }
            "media_add" => {
                let path = self.user_path(&need(a, "path")?)?;
                if !path.is_file() {
                    return Err(format!("There is no file at {}.", path.display()));
                }
                if !package::is_media_file(&path) {
                    return Err("That is not a picture or video file.".into());
                }
                let r = self.media_ref(&path);
                Ok(obj(vec![("ref", text(&r))]))
            }
            "media_remove" => {
                let r = need(a, "ref")?;
                if self.text.contains(&r) {
                    return Err(format!("{r} is still used in a slide or the logo. Remove the slides that use it first."));
                }
                let root = self
                    .images
                    .doc_dir
                    .clone()
                    .ok_or("This presentation has no media folder.")?;
                let file = package::list_assets(&root)
                    .into_iter()
                    .find(|f| package::asset_ref(&root, f) == r)
                    .ok_or_else(|| format!("There is no picture or video {r}."))?;
                std::fs::remove_file(&file).map_err(|e| e.to_string())?;
                self.media_epoch += 1;
                Ok(obj(vec![("removed", text(&r))]))
            }
            "settings_get" => {
                let s = &self.settings;
                Ok(obj(vec![
                    ("editor_font", Json::Num(s.editor_font as f64)),
                    ("smart_quotes", Json::Bool(s.smart_quotes)),
                    ("smart_dashes", Json::Bool(s.smart_dashes)),
                    ("single_return", Json::Bool(s.single_return)),
                    ("fit_text", Json::Bool(s.fit_text)),
                    ("wpm", Json::Num(s.wpm as f64)),
                    ("show_speaking_time", Json::Bool(s.show_speaking_time)),
                    ("warn_empty", Json::Bool(s.warn_empty)),
                ]))
            }
            "settings_set" => {
                let key = need(a, "key")?;
                let value = need(a, "value")?;
                let flag = || match value.as_str() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err(format!("{key} takes true or false.")),
                };
                let s = &mut self.settings;
                match key.as_str() {
                    "editor_font" => {
                        s.editor_font = value.parse().map_err(|_| "editor_font takes a number.")?
                    }
                    "smart_quotes" => s.smart_quotes = flag()?,
                    "smart_dashes" => s.smart_dashes = flag()?,
                    "single_return" => s.single_return = flag()?,
                    "fit_text" => s.fit_text = flag()?,
                    "wpm" => s.wpm = value.parse().map_err(|_| "wpm takes a whole number.")?,
                    "show_speaking_time" => s.show_speaking_time = flag()?,
                    "warn_empty" => s.warn_empty = flag()?,
                    other => {
                        return Err(format!("{other} is not a preference settings_get lists."))
                    }
                }
                self.settings.save();
                self.reparse();
                Ok(obj(vec![("key", text(&key)), ("value", text(&value))]))
            }
            "present_start" => {
                if a.get("slide").is_some() {
                    let i = self.slide_no(a, "slide")?;
                    self.select_slide(i);
                }
                self.start_presenting(ctx);
                Ok(obj(vec![("presenting", Json::Bool(self.presenting))]))
            }
            "present_stop" => {
                self.stop_presenting(ctx);
                Ok(obj(vec![("presenting", Json::Bool(self.presenting))]))
            }
            "export" => {
                if self.export.is_some() {
                    return Err("An export is already running. Wait for it to finish.".into());
                }
                let format = match need(a, "format")?.as_str() {
                    "pdf" => export::Format::Pdf,
                    "pptx" => export::Format::PowerPoint,
                    "images" => export::Format::Images,
                    "html" => export::Format::Html,
                    other => return Err(format!("{other} is not pdf, pptx, images or html.")),
                };
                let dest = self.user_path(&need(a, "path")?)?;
                if dest.exists() && a.bool("overwrite") != Some(true) {
                    return Err(format!(
                        "{} exists. Pass overwrite: true to replace it.",
                        dest.display()
                    ));
                }
                self.export = Some(export::Job {
                    format,
                    dest: dest.clone(),
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
                Ok(obj(vec![
                    ("exporting", text(&dest.display().to_string())),
                    ("slides", Json::Num(self.slides.len() as f64)),
                ]))
            }
            "package_save" => {
                let path = self.user_path(&need(a, "path")?)?;
                if path.extension().and_then(|e| e.to_str()) != Some(package::EXT) {
                    return Err(format!("The path must end in .{}.", package::EXT));
                }
                if !self.save_to(path.clone()) {
                    return Err("The presentation could not be saved.".into());
                }
                Ok(obj(vec![("saved", text(&path.display().to_string()))]))
            }
            "package_open" => {
                let path = self.user_path(&need(a, "path")?)?;
                if self.dirty && a.bool("discard") != Some(true) {
                    return Err(
                        "There are unsaved changes. Save first, or pass discard: true.".into(),
                    );
                }
                self.control.undo.clear();
                self.open_path(path.clone());
                Ok(obj(vec![("opened", text(&path.display().to_string()))]))
            }
            "slide_add"
            | "slide_update"
            | "slide_delete"
            | "slide_move"
            | "slide_duplicate"
            | "insert_picture"
            | "insert_table"
            | "deck_write_markdown" => Err("Internal: script edit not parsed.".into()),
            other => Err(format!("{other} is not handled by the app yet.")),
        }
    }

    fn current_palette(&self, dark: bool) -> Palette {
        let p = &self.presets[self.preset_idx];
        if dark {
            self.dark_override.unwrap_or(p.dark)
        } else {
            self.light_override.unwrap_or(p.light)
        }
    }

    fn deck_info(&self) -> Json {
        let s = &self.settings;
        obj(vec![
            ("name", text(&self.doc_name_pub())),
            (
                "path",
                text(
                    &self
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                ),
            ),
            ("unsaved_changes", Json::Bool(self.dirty)),
            ("slides", Json::Num(self.slides.len() as f64)),
            ("current_slide", Json::Num((self.current + 1) as f64)),
            ("theme", text(self.presets[self.preset_idx].name)),
            ("appearance", text(if self.dark { "dark" } else { "light" })),
            ("title_font", text(&font_label(self.title_font))),
            ("body_font", text(&font_label(self.body_font))),
            ("size", text(size_label(self.size_idx))),
            ("line_height", text(LINE_HEIGHTS[self.line_idx].0)),
            ("author", text(&self.author)),
            ("audience", text(&self.audience)),
            ("aspect", text(crate::settings::ASPECTS[s.aspect].0)),
            ("logo", text(&self.logo)),
            ("presenting", Json::Bool(self.presenting)),
        ])
    }

    fn deck_check(&self) -> Json {
        let mut warnings: Vec<String> = Vec::new();
        let root = self.images.doc_dir.clone();
        let exists = |src: &str| {
            let decoded = src.replace("%20", " ");
            root.as_ref()
                .map(|r| r.join(decoded.trim_start_matches('/')).is_file())
                .unwrap_or(false)
        };
        for (i, body) in script::bodies(&self.text).iter().enumerate() {
            if !body.lines().any(script::is_visible_line) {
                warnings.push(format!(
                    "Slide {}: no visible content, so the slide is blank.",
                    i + 1
                ));
            }
            for line in body.lines() {
                let t = line.trim();
                let src = match t.strip_prefix("![") {
                    Some(rest) => rest
                        .find("](")
                        .and_then(|p| rest[p + 2..].strip_suffix(')')),
                    None if t.starts_with("/assets/") => Some(t),
                    None => None,
                };
                if let Some(src) = src {
                    if !src.starts_with("http") && !exists(src) {
                        warnings.push(format!("Slide {}: the picture {src} is missing.", i + 1));
                    }
                }
            }
        }
        if !self.logo.is_empty() && !self.logo.starts_with("http") && !exists(&self.logo) {
            warnings.push(format!("The logo {} is missing.", self.logo));
        }
        for (look, dark) in [("light", false), ("dark", true)] {
            let p = self.current_palette(dark);
            for (what, c) in [("title", p.title), ("body", p.body)] {
                let r = contrast(c, p.bg);
                if r < 4.5 {
                    warnings.push(format!(
                        "{look} appearance: {what} text contrast is {r:.1}:1; 4.5:1 is recommended."
                    ));
                }
            }
        }
        obj(vec![
            ("ok", Json::Bool(warnings.is_empty())),
            (
                "warnings",
                Json::Arr(warnings.iter().map(|w| text(w)).collect()),
            ),
        ])
    }
}

fn look_is_dark(a: &Json) -> Result<bool, String> {
    match str_arg(a, "look").as_deref() {
        Some("light") => Ok(false),
        Some("dark") => Ok(true),
        _ => Err("look must be light or dark.".into()),
    }
}

pub fn slot_kind_name(kind: SlotKind) -> &'static str {
    match kind {
        SlotKind::Empty => "empty",
        SlotKind::Number => "number",
        SlotKind::Date => "date",
        SlotKind::Logo => "logo",
        SlotKind::Custom => "custom",
    }
}
