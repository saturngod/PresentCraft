//! Floating toolbar over a level-one heading under the cursor:
//! [left] [center] [right] | [top] [middle] [bottom] | [•••].
//! It edits the `x:` / `y:` lines written under the heading, which place the
//! cover title; default values (`x: left`, `y: middle`) remove their line.

use crate::parse::{title_option, Align, TitleOption, VAlign};
use crate::tableblock::{char_of_line, line_of_char, lines_of};
use crate::tabletools::{bar_divider, check, heading, more_button, paint_bar, round_button};
use crate::{slide_color, App};
use eframe::egui;
use egui::text::CCursor;
use egui::{pos2, vec2, Color32, Galley, Pos2, Rect, Stroke};

fn is_h1(line: &str) -> bool {
    let l = line.trim_end_matches('\r');
    l.starts_with("# ") && !l[2..].trim().is_empty()
}

fn option_of(line: &str) -> Option<TitleOption> {
    if line.starts_with('\t') {
        return None;
    }
    title_option(line.trim_end_matches('\r'))
}

/// A level-one heading and the `x:` / `y:` lines right under it: lines `head..end`.
pub struct TitleBlock {
    pub head: usize,
    pub end: usize,
}

pub fn find(lines: &[String], cur: usize) -> Option<TitleBlock> {
    let mut i = cur.min(lines.len().checked_sub(1)?);
    while !is_h1(&lines[i]) {
        if option_of(&lines[i]).is_none() || i == 0 {
            return None;
        }
        i -= 1;
    }
    let mut end = i + 1;
    while end < lines.len() && option_of(&lines[end]).is_some() {
        end += 1;
    }
    if cur >= end {
        return None;
    }
    Some(TitleBlock { head: i, end })
}

impl TitleBlock {
    fn line_of(&self, lines: &[String], key: &str) -> Option<usize> {
        (self.head + 1..self.end).find(|&i| match option_of(&lines[i]) {
            Some(TitleOption::X(_)) => key == "x",
            Some(TitleOption::Y(_)) => key == "y",
            None => false,
        })
    }

    pub fn x(&self, lines: &[String]) -> Align {
        (self.head + 1..self.end)
            .filter_map(|i| match option_of(&lines[i]) {
                Some(TitleOption::X(x)) => Some(x),
                _ => None,
            })
            .last()
            .unwrap_or_default()
    }

    pub fn y(&self, lines: &[String]) -> VAlign {
        (self.head + 1..self.end)
            .filter_map(|i| match option_of(&lines[i]) {
                Some(TitleOption::Y(y)) => Some(y),
                _ => None,
            })
            .last()
            .unwrap_or_default()
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

/// Text lines aligned left, centre or right (SF Symbols "text.align…").
fn halign_icon(p: &egui::Painter, c: Pos2, col: Color32, a: Align) {
    let name = match a {
        Align::Left => "text.alignleft",
        Align::Center => "text.aligncenter",
        Align::Right => "text.alignright",
    };
    if crate::icons::paint(p, name, c, 15.0, col) {
        return;
    }
    let s = Stroke::new(1.6_f32, col);
    for (dy, w) in [(-6.0, 16.0), (-2.0, 10.0), (2.0, 16.0), (6.0, 10.0)] {
        let x0 = match a {
            Align::Left => c.x - 8.0,
            Align::Center => c.x - w / 2.0,
            Align::Right => c.x + 8.0 - w,
        };
        p.line_segment([pos2(x0, c.y + dy), pos2(x0 + w, c.y + dy)], s);
    }
}

/// A frame with a text block at its top, middle or bottom.
fn valign_icon(p: &egui::Painter, c: Pos2, col: Color32, a: VAlign) {
    let name = match a {
        VAlign::Top => "rectangle.topthird.inset.filled",
        VAlign::Middle => "rectangle.center.inset.filled",
        VAlign::Bottom => "rectangle.bottomthird.inset.filled",
    };
    if crate::icons::paint(p, name, c, 15.0, col) {
        return;
    }
    let b = Rect::from_center_size(c, vec2(18.0, 16.0));
    p.rect_stroke(b, 3.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
    let cy = match a {
        VAlign::Top => b.top() + 5.0,
        VAlign::Middle => c.y,
        VAlign::Bottom => b.bottom() - 5.0,
    };
    p.rect_filled(
        Rect::from_center_size(pos2(c.x, cy), vec2(10.0, 3.5)),
        1.0,
        col,
    );
}

fn x_word(a: Align) -> Option<&'static str> {
    match a {
        Align::Left => None,
        Align::Center => Some("center"),
        Align::Right => Some("right"),
    }
}

fn y_word(a: VAlign) -> Option<&'static str> {
    match a {
        VAlign::Top => Some("top"),
        VAlign::Middle => None,
        VAlign::Bottom => Some("bottom"),
    }
}

impl App {
    pub fn title_toolbar(&mut self, ui: &mut egui::Ui, galley: &Galley, galley_pos: Pos2) {
        let lines = lines_of(&self.text);
        let cur_line = line_of_char(&self.text, self.cursor);
        let Some(block) = find(&lines, cur_line) else {
            return;
        };
        let start_ch = char_of_line(&lines, block.head);
        let r = galley.pos_from_cursor(CCursor::new(start_ch));
        let clip = ui.clip_rect();
        // Above the heading. When there is no room there, beside it at the
        // right edge of the column, so the bar never covers the title or the
        // lines under it; below only when the heading is too long for that.
        const BAR_W: f32 = 322.0;
        let above = galley_pos.y + r.top() - 50.0;
        let mut left = galley_pos.x - 14.0;
        let y = if above >= clip.top() + 6.0 {
            above
        } else {
            let head_end = start_ch + lines[block.head].chars().count();
            let e = galley.pos_from_cursor(CCursor::new(head_end));
            let beside = galley_pos.x + galley.job.wrap.max_width + 14.0 - BAR_W;
            let one_row = (e.top() - r.top()).abs() < 1.0;
            if one_row && galley_pos.x + e.right() + 24.0 <= beside {
                left = beside;
                (galley_pos.y + r.center().y - 22.0).max(clip.top() + 6.0)
            } else {
                galley_pos.y + r.bottom() + 6.0
            }
        }
        .min(clip.bottom() - 50.0);
        let bar = Rect::from_min_size(pos2(left, y), vec2(BAR_W, 44.0));
        let accent = slide_color(self.current);
        let x = block.x(&lines);
        let yv = block.y(&lines);
        let mut act: Option<(&'static str, Option<&'static str>)> = None;
        egui::Area::new(egui::Id::unique("title-toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(bar.min)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(bar.size());
                let p = ui.painter().clone();
                paint_bar(&p, bar);
                let cy = bar.center().y;
                let c = |dx: f32| pos2(bar.left() + dx, cy);
                for (dx, a, id, tip) in [
                    (25.0, Align::Left, "tt-left", "Align the title left"),
                    (69.0, Align::Center, "tt-center", "Center the title"),
                    (113.0, Align::Right, "tt-right", "Align the title right"),
                ] {
                    let (resp, col) = round_button(ui, &p, c(dx), id, tip, x == a, accent);
                    halign_icon(&p, c(dx), col, a);
                    if resp.clicked() && x != a {
                        act = Some(("x", x_word(a)));
                    }
                }
                bar_divider(&p, bar, 137.0);
                for (dx, a, id, tip) in [
                    (161.0, VAlign::Top, "tt-top", "Place the title at the top"),
                    (205.0, VAlign::Middle, "tt-middle", "Place the title in the middle"),
                    (249.0, VAlign::Bottom, "tt-bottom", "Place the title at the bottom"),
                ] {
                    let (resp, col) = round_button(ui, &p, c(dx), id, tip, yv == a, accent);
                    valign_icon(&p, c(dx), col, a);
                    if resp.clicked() && yv != a {
                        act = Some(("y", y_word(a)));
                    }
                }
                bar_divider(&p, bar, 273.0);
                more_button(ui, &p, c(297.0), "title-more", |ui| {
                    heading(ui, "Horizontal Alignment");
                    for (a, name) in [
                        (Align::Left, "Left — Default"),
                        (Align::Center, "Center"),
                        (Align::Right, "Right"),
                    ] {
                        if check(ui, x == a, name) {
                            act = Some(("x", x_word(a)));
                        }
                    }
                    ui.separator();
                    heading(ui, "Vertical Alignment");
                    for (a, name) in [
                        (VAlign::Middle, "Middle — Default"),
                        (VAlign::Top, "Top"),
                        (VAlign::Bottom, "Bottom"),
                    ] {
                        if check(ui, yv == a, name) {
                            act = Some(("y", y_word(a)));
                        }
                    }
                });
            });
        let Some((key, value)) = act else { return };
        let mut lines = lines;
        let mut block = block;
        block.set(&mut lines, key, value);
        self.text = lines.join("\n");
        self.dirty = true;
        self.reparse();
        // keep the cursor at the end of the heading line
        let ch = char_of_line(&lines, block.head) + lines[block.head].chars().count();
        self.cursor = ch.min(self.text.chars().count());
        self.sel_start = self.cursor;
        self.jump_to = Some((self.cursor, self.cursor));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_heading_from_option_line() {
        let l = lines_of("# Title\nx: right\ny: bottom\n\tSub\n## Two");
        let b = find(&l, 2).expect("block");
        assert_eq!((b.head, b.end), (0, 3));
        assert_eq!(b.x(&l), Align::Right);
        assert_eq!(b.y(&l), VAlign::Bottom);
        assert!(find(&l, 3).is_none());
        assert!(find(&l, 4).is_none());
    }

    #[test]
    fn set_adds_changes_and_removes_options() {
        let mut l = lines_of("# Title\n\tSub");
        let mut b = find(&l, 0).unwrap();
        assert_eq!((b.x(&l), b.y(&l)), (Align::Left, VAlign::Middle));
        b.set(&mut l, "y", Some("bottom"));
        b.set(&mut l, "x", Some("center"));
        assert_eq!(l.join("\n"), "# Title\ny: bottom\nx: center\n\tSub");
        b.set(&mut l, "x", Some("right"));
        b.set(&mut l, "y", None);
        assert_eq!(l.join("\n"), "# Title\nx: right\n\tSub");
        let s = crate::parse::parse_document(&l.join("\n"));
        assert_eq!(s[0].title_x, Align::Right);
    }
}
