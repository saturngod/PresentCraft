//! Floating toolbar over the picture under the cursor:
//! [cover] [contain] | [on slide] [background] | [•••].
//! It edits the option lines written under the picture (`size:`, `x:`, `y:`,
//! `background:`, `filter:`, `opacity:`, `title:`); default values remove
//! their line so the script stays short.

use crate::tableblock::{char_of_line, line_of_char, lines_of};
use crate::tabletools::{
    bar_divider, check, heading, more_button, paint_bar, round_button, submenu,
};
use crate::{slide_color, App};
use eframe::egui;
use egui::text::CCursor;
use egui::{pos2, vec2, Color32, Galley, Pos2, Rect, Stroke};

const KEYS: &[&str] = &[
    "size",
    "x",
    "y",
    "background",
    "opacity",
    "filter",
    "title",
    "caption",
];

fn key_of(line: &str) -> Option<String> {
    let (k, _) = line.trim().split_once(':')?;
    let k = k.trim().to_ascii_lowercase();
    KEYS.contains(&k.as_str()).then_some(k)
}

fn is_src(line: &str) -> bool {
    let t = line.trim();
    crate::parse::is_image_src(t) || crate::parse::markdown_image(t).is_some()
}

/// A picture line and the option lines right under it: lines `src..end`.
pub struct ImageBlock {
    pub src: usize,
    pub end: usize,
}

pub fn find(lines: &[String], cur: usize) -> Option<ImageBlock> {
    let mut i = cur.min(lines.len().checked_sub(1)?);
    while !is_src(&lines[i]) {
        if key_of(&lines[i]).is_none() || i == 0 {
            return None;
        }
        i -= 1;
    }
    let mut end = i + 1;
    while end < lines.len() && key_of(&lines[end]).is_some() {
        end += 1;
    }
    if cur >= end {
        return None;
    }
    Some(ImageBlock { src: i, end })
}

impl ImageBlock {
    fn line_of(&self, lines: &[String], key: &str) -> Option<usize> {
        (self.src + 1..self.end).find(|&i| key_of(&lines[i]).as_deref() == Some(key))
    }

    pub fn get(&self, lines: &[String], key: &str) -> Option<String> {
        let i = self.line_of(lines, key)?;
        lines[i]
            .split_once(':')
            .map(|(_, v)| v.trim().to_ascii_lowercase())
    }

    /// Sets `key: value`, or removes the line when `value` is `None`.
    pub fn set(&mut self, lines: &mut Vec<String>, key: &str, value: Option<&str>) {
        match (self.line_of(lines, key), value) {
            (Some(i), Some(v)) => lines[i] = format!("{key}: {v}"),
            (Some(i), None) => {
                lines.remove(i);
                self.end -= 1;
            }
            (None, Some(v)) => {
                lines.insert(self.end, format!("{key}: {v}"));
                self.end += 1;
            }
            (None, None) => {}
        }
    }
}

fn cover_icon(p: &egui::Painter, c: Pos2, col: Color32, hole: Color32) {
    let b = Rect::from_center_size(c, vec2(19.0, 15.0));
    p.rect_filled(b, 3.0, col);
    p.add(egui::Shape::convex_polygon(
        vec![
            pos2(b.left() + 3.0, b.bottom() - 3.0),
            pos2(b.left() + 8.0, b.top() + 6.0),
            pos2(b.left() + 13.0, b.bottom() - 3.0),
        ],
        hole,
        Stroke::NONE,
    ));
    p.circle_filled(pos2(b.right() - 5.0, b.top() + 4.5), 1.7, hole);
}

fn contain_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let b = Rect::from_center_size(c, vec2(19.0, 15.0));
    p.rect_stroke(b, 3.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
    p.rect_filled(Rect::from_center_size(c, vec2(9.0, 7.0)), 1.5, col);
}

fn inline_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(1.5_f32, col);
    let b = Rect::from_center_size(c, vec2(19.0, 15.0));
    p.rect_stroke(b, 3.0, s, egui::StrokeKind::Inside);
    p.rect_filled(
        Rect::from_min_size(b.min + vec2(3.0, 3.0), vec2(6.0, 9.0)),
        1.0,
        col,
    );
    for dy in [4.5, 7.5, 10.5] {
        p.line_segment(
            [
                pos2(b.left() + 11.0, b.top() + dy),
                pos2(b.right() - 3.0, b.top() + dy),
            ],
            Stroke::new(1.2_f32, col),
        );
    }
}

fn background_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let b = Rect::from_center_size(c, vec2(19.0, 15.0));
    p.rect_filled(b, 3.0, col.gamma_multiply(0.35));
    p.rect_stroke(b, 3.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
    for (dy, w) in [(5.0, 11.0), (9.0, 7.0)] {
        p.line_segment(
            [
                pos2(b.left() + 4.0, b.top() + dy),
                pos2(b.left() + 4.0 + w, b.top() + dy),
            ],
            Stroke::new(1.6_f32, col),
        );
    }
}

impl App {
    pub fn image_toolbar(&mut self, ui: &mut egui::Ui, galley: &Galley, galley_pos: Pos2) {
        let lines = lines_of(&self.text);
        let cur_line = line_of_char(&self.text, self.cursor);
        let Some(block) = find(&lines, cur_line) else {
            return;
        };
        let start_ch = char_of_line(&lines, block.src);
        let r = galley.pos_from_ccursor(CCursor::new(start_ch));
        let clip = ui.clip_rect();
        let y = (galley_pos.y + r.top() - 50.0)
            .max(clip.top() + 6.0)
            .min(clip.bottom() - 50.0);
        let bar = Rect::from_min_size(pos2(galley_pos.x - 14.0, y), vec2(234.0, 44.0));
        let accent = slide_color(self.current);
        let contain = block
            .get(&lines, "size")
            .map(|v| v.contains("contain"))
            .unwrap_or(false);
        let background = block
            .get(&lines, "background")
            .map(|v| matches!(v.as_str(), "true" | "yes" | "1" | "on"))
            .unwrap_or(false);
        let x = block.get(&lines, "x").unwrap_or_else(|| "center".into());
        let yv = block.get(&lines, "y").unwrap_or_else(|| "center".into());
        let filter = block.get(&lines, "filter");
        let opacity = block.get(&lines, "opacity");
        let caption =
            block.get(&lines, "title").is_some() || block.get(&lines, "caption").is_some();
        let mut act: Option<(&'static str, Option<String>)> = None;
        egui::Area::new(egui::Id::new("image-toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(bar.min)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(bar.size());
                let p = ui.painter().clone();
                paint_bar(&p, bar);
                let cy = bar.center().y;
                let c = |dx: f32| pos2(bar.left() + dx, cy);
                let (r1, k1) = round_button(
                    ui,
                    &p,
                    c(25.0),
                    "it-cover",
                    "Fill the space (cover)",
                    !contain,
                    accent,
                );
                let (r2, k2) = round_button(
                    ui,
                    &p,
                    c(69.0),
                    "it-contain",
                    "Show the whole picture (contain)",
                    contain,
                    accent,
                );
                bar_divider(&p, bar, 93.0);
                let (r3, k3) = round_button(
                    ui,
                    &p,
                    c(117.0),
                    "it-inline",
                    "Place with the slide's content",
                    !background,
                    accent,
                );
                let (r4, k4) = round_button(
                    ui,
                    &p,
                    c(161.0),
                    "it-bg",
                    "Use as the slide background",
                    background,
                    accent,
                );
                bar_divider(&p, bar, 185.0);
                let hole = if !contain {
                    accent
                } else {
                    crate::settings::pal().pill
                };
                cover_icon(&p, c(25.0), k1, hole);
                contain_icon(&p, c(69.0), k2);
                inline_icon(&p, c(117.0), k3);
                background_icon(&p, c(161.0), k4);
                if r1.clicked() && contain {
                    act = Some(("size", None));
                }
                if r2.clicked() && !contain {
                    act = Some(("size", Some("contain".into())));
                }
                if r3.clicked() && background {
                    act = Some(("background", None));
                }
                if r4.clicked() && !background {
                    act = Some(("background", Some("true".into())));
                }
                more_button(ui, &p, c(209.0), "image-more", |ui| {
                    heading(ui, "Size");
                    if check(ui, !contain, "Cover — Default") {
                        act = Some(("size", None));
                    }
                    if check(ui, contain, "Contain") {
                        act = Some(("size", Some("contain".into())));
                    }
                    ui.separator();
                    heading(ui, "Horizontal Alignment");
                    for (v, name) in [
                        ("center", "Center — Default"),
                        ("left", "Left"),
                        ("right", "Right"),
                    ] {
                        if check(ui, x == v, name) {
                            act = Some(("x", (v != "center").then(|| v.to_string())));
                        }
                    }
                    ui.separator();
                    heading(ui, "Vertical Alignment");
                    for (v, name) in [
                        ("center", "Center — Default"),
                        ("top", "Top"),
                        ("bottom", "Bottom"),
                    ] {
                        if check(ui, yv == v, name) {
                            act = Some(("y", (v != "center").then(|| v.to_string())));
                        }
                    }
                    ui.separator();
                    heading(ui, "Order");
                    if check(ui, !background, "Content — Default") {
                        act = Some(("background", None));
                    }
                    if check(ui, background, "Background") {
                        act = Some(("background", Some("true".into())));
                    }
                    ui.separator();
                    heading(ui, "Style");
                    submenu(
                        ui,
                        format!("{}Filter", if filter.is_some() { "✓  " } else { "     " }),
                        |ui| {
                            if check(ui, filter.is_none(), "None") {
                                act = Some(("filter", None));
                            }
                            for (v, name) in [
                                ("grayscale", "Grayscale"),
                                ("sepia", "Sepia"),
                                ("darken", "Darken"),
                                ("lighten", "Lighten"),
                                ("blur", "Blur"),
                            ] {
                                if check(ui, filter.as_deref() == Some(v), name) {
                                    act = Some(("filter", Some(v.to_string())));
                                }
                            }
                        },
                    );
                    submenu(
                        ui,
                        format!("{}Opacity", if opacity.is_some() { "✓  " } else { "     " }),
                        |ui| {
                            for pct in [100, 80, 60, 40, 20] {
                                let v = format!("{pct}%");
                                let on = match &opacity {
                                    None => pct == 100,
                                    Some(o) => o.trim_end_matches('%') == pct.to_string(),
                                };
                                if check(ui, on, &v) {
                                    act = Some(("opacity", (pct != 100).then_some(v)));
                                }
                            }
                        },
                    );
                    ui.separator();
                    heading(ui, "Description");
                    if check(ui, caption, "Caption") {
                        act = Some((
                            "title",
                            if caption {
                                None
                            } else {
                                Some("Caption".into())
                            },
                        ));
                    }
                });
            });
        let Some((key, value)) = act else { return };
        let mut lines = lines;
        let mut block = block;
        if key == "title" && value.is_none() {
            block.set(&mut lines, "caption", None);
        }
        block.set(&mut lines, key, value.as_deref());
        self.text = lines.join("\n");
        self.dirty = true;
        self.reparse();
        // keep the cursor on the picture line
        let ch = char_of_line(&lines, block.src) + lines[block.src].chars().count();
        self.cursor = ch.min(self.text.chars().count());
        self.sel_start = self.cursor;
        self.jump_to = Some((self.cursor, self.cursor));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        lines_of(s)
    }

    #[test]
    fn finds_picture_from_option_line() {
        let l = lines("text\n/a.jpg\nx: left\ny: top\n\nmore");
        let b = find(&l, 3).expect("block");
        assert_eq!((b.src, b.end), (1, 4));
        assert!(find(&l, 0).is_none());
        assert!(find(&l, 5).is_none());
    }

    #[test]
    fn set_adds_changes_and_removes_options() {
        let mut l = lines("/a.jpg\nx: left\n\nnotes");
        let mut b = find(&l, 0).unwrap();
        b.set(&mut l, "size", Some("contain"));
        assert_eq!(l.join("\n"), "/a.jpg\nx: left\nsize: contain\n\nnotes");
        b.set(&mut l, "x", Some("right"));
        b.set(&mut l, "size", None);
        assert_eq!(l.join("\n"), "/a.jpg\nx: right\n\nnotes");
        assert_eq!(b.get(&l, "x").as_deref(), Some("right"));
    }
}
