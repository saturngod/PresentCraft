//! The right-hand panel: Format (Markdown cheat sheet that inserts what you
//! click, plus table and chart maker), Media, and Design (theme, fonts,
//! colours, header & footer).

use crate::parse::Block;
use crate::render::ASPECT;
use crate::theme::Palette;
use crate::{
    icon_button, kbd, paint_icon, App, FontChoice, Icon, ACCENT_BLUE, DESIGN_PINK, FORMAT_BLUE,
    INSPECTOR_W, IS_MAC, SIZES,
};
use eframe::egui;
use std::path::PathBuf;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Color32, FontId, Key, Modifiers, Rect, RichText, Sense, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Format,
    Media,
    Design,
    Share,
}

/// Share button colour (estimated from the person's screenshot).
const SHARE_ORANGE: Color32 = Color32::from_rgb(0xE5, 0x9A, 0x16);

/// Markdown syntax colour in the inspector, readable on either background.
fn on_slide() -> Color32 {
    if crate::settings::is_dark() {
        Color32::from_rgb(0x8E, 0xC6, 0xF5)
    } else {
        Color32::from_rgb(0x1E, 0x6F, 0xC0)
    }
}

/// Secondary text that must stay readable (stronger than `pal().text_dim`).
fn text_mid() -> Color32 {
    if crate::settings::is_dark() {
        Color32::from_rgb(0xB4, 0xB4, 0xB8)
    } else {
        Color32::from_rgb(0x4A, 0x4A, 0x4F)
    }
}
const ROW_H: f32 = 26.0;

enum Action {
    Prefix(&'static str),
    Wrap(&'static str, &'static str, &'static str),
    Block(&'static str),
    AddPicture,
    AddVideo,
}

struct Item {
    icon: &'static str,
    label: LayoutJob,
    shortcut: &'static str,
    action: Action,
}

fn job(parts: &[(&str, Color32, bool, bool, bool)]) -> LayoutJob {
    let mut j = LayoutJob::default();
    for (t, c, bold, strike, mark) in parts {
        let fam = crate::theme::family(false, *bold, false);
        let mut tf = TextFormat {
            font_id: FontId::new(13.5, fam),
            color: *c,
            valign: egui::Align::Center,
            ..Default::default()
        };
        if *strike {
            tf.strikethrough = Stroke::new(1.0_f32, *c);
        }
        if *mark {
            if crate::settings::is_dark() {
                tf.background = Color32::from_rgb(0x8A, 0x7A, 0x10);
                tf.underline = Stroke::new(1.0_f32, Color32::from_rgb(0xF5, 0xD0, 0x30));
            } else {
                tf.background = Color32::from_rgb(0xFB, 0xE7, 0x7E);
                tf.color = crate::settings::pal().text_hi;
            }
        }
        j.append(t, 0.0, tf);
    }
    j
}

fn plain(t: &str) -> LayoutJob {
    job(&[(t, crate::settings::pal().text_hi, false, false, false)])
}

fn dimmed(prefix: &str, t: &str, c: Color32, bold: bool) -> LayoutJob {
    job(&[
        (prefix, crate::settings::pal().text_dim, false, false, false),
        (t, c, bold, false, false),
    ])
}

/// Section title with a Hide/Show toggle on the right. The toggle only
/// appears while the pointer is over the section. Returns whether the body
/// should be drawn; the section's hover area closes at the next `divider`
/// or `section_end`.
fn section(ui: &mut egui::Ui, title: &str) -> bool {
    let pal = crate::settings::pal();
    section_end(ui);
    let key = egui::Id::unique(("inspector_section", title));
    let open_id = key.with("open");
    let area_id = key.with("area");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(true);
    let last_area: Option<Rect> = ui.data(|d| d.get_temp(area_id));
    ui.add_space(12.0);
    let top = ui.cursor().top();
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        FontId::new(13.5, crate::theme::family(false, true, false)),
        pal.text_hi,
    );
    let over = ui.rect_contains_pointer(row)
        || last_area.is_some_and(|r| ui.rect_contains_pointer(r));
    let label = if open { "Hide" } else { "Show" };
    let font = FontId::proportional(13.0);
    let chevron = 10.0;
    let text_w = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), pal.text_dim)
        .size()
        .x;
    let w = text_w + 6.0 + chevron;
    let hit = Rect::from_min_max(pos2(row.right() - w - 4.0, row.top()), row.right_bottom());
    let resp = ui.interact(hit, key.with("toggle"), Sense::click());
    if over || resp.hovered() {
        let color = if resp.hovered() { pal.text_hi } else { pal.text_dim };
        let g = ui.painter().layout_no_wrap(label.to_owned(), font, color);
        ui.painter()
            .galley(pos2(row.right() - w, row.center().y - g.size().y / 2.0), g, color);
        let c = pos2(row.right() - chevron / 2.0, row.center().y);
        let st = Stroke::new(1.4_f32, color);
        let p = ui.painter();
        if open {
            p.line_segment([c + vec2(-4.5, -2.0), c + vec2(0.0, 2.5)], st);
            p.line_segment([c + vec2(0.0, 2.5), c + vec2(4.5, -2.0)], st);
        } else {
            p.line_segment([c + vec2(-2.0, -4.5), c + vec2(2.5, 0.0)], st);
            p.line_segment([c + vec2(2.5, 0.0), c + vec2(-2.0, 4.5)], st);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            open = !open;
        }
    }
    ui.data_mut(|d| {
        d.insert_temp(open_id, open);
        d.insert_temp(egui::Id::unique("inspector_section_current"), (area_id, top));
    });
    ui.add_space(4.0);
    open
}

/// Records where the current section ends, for the hover-only toggle.
fn section_end(ui: &mut egui::Ui) {
    let cur = egui::Id::unique("inspector_section_current");
    let Some((area_id, top)) = ui.data(|d| d.get_temp::<(egui::Id, f32)>(cur)) else {
        return;
    };
    ui.data_mut(|d| d.remove::<(egui::Id, f32)>(cur));
    let max = ui.max_rect();
    let r = Rect::from_min_max(pos2(max.left(), top), pos2(max.right(), ui.cursor().top()));
    ui.data_mut(|d| d.insert_temp(area_id, r));
}

fn divider(ui: &mut egui::Ui) {
    section_end(ui);
    ui.add_space(12.0);
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [pos2(r.left(), r.top()), pos2(r.right(), r.top())],
        Stroke::new(1.0_f32, crate::settings::pal().divider),
    );
    ui.add_space(2.0);
}

fn item_row(ui: &mut egui::Ui, it: &Item) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let resp = resp.on_hover_text_at_pointer("Click to insert at the cursor");
    let p = ui.painter();
    if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 6.0, crate::settings::pal().button);
    } else if resp.hovered() {
        p.rect_filled(r, 6.0, crate::settings::pal().hover);
    }
    p.text(
        pos2(r.left() + 20.0, r.center().y),
        egui::Align2::CENTER_CENTER,
        it.icon,
        FontId::new(12.5, crate::theme::family(false, true, false)),
        crate::settings::pal().icon,
    );
    let g = ui.fonts_mut(|f| f.layout_job(it.label.clone()));
    p.galley(
        pos2(r.left() + 46.0, r.center().y - g.size().y / 2.0),
        g,
        crate::settings::pal().text_hi,
    );
    p.text(
        pos2(r.right() - 10.0, r.center().y),
        egui::Align2::RIGHT_CENTER,
        kbd(it.shortcut),
        FontId::proportional(12.0),
        crate::settings::pal().text_dim,
    );
    resp.clicked()
}

pub fn make_table(cols: usize, rows: usize, values: bool, display: usize) -> String {
    let cols = cols.max(2);
    let rows = rows.max(2);
    let mut out = String::new();
    out.push_str("|        ");
    for c in 1..cols {
        out.push_str(&format!("| Series {c} "));
    }
    out.push_str("|\n");
    for _ in 0..cols {
        out.push_str("| :----- ");
    }
    out.push_str("|\n");
    for r in 1..rows {
        let name = (b'A' + ((r - 1) % 26) as u8) as char;
        out.push_str(&format!("| Group {name} "));
        for c in 1..cols {
            if values {
                out.push_str(&format!(
                    "| {:<8} ",
                    5 + 5 * r + 5 * c + if r == rows - 1 { 5 } else { 0 }
                ));
            } else {
                out.push_str("|          ");
            }
        }
        out.push_str("|\n");
    }
    match display {
        1 => out.push_str("Chart: Line\n"),
        2 => out.push_str("Chart: Bar\n"),
        3 => out.push_str("Chart: Pie\n"),
        _ => {}
    }
    out.trim_end().to_string()
}

pub fn csv_to_table(text: &str) -> String {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut cells = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        for ch in line.chars() {
            match ch {
                '"' => quoted = !quoted,
                ',' | ';' if !quoted => cells.push(std::mem::take(&mut cur)),
                _ => cur.push(ch),
            }
        }
        cells.push(cur);
        rows.push(
            cells
                .into_iter()
                .map(|c| c.trim().replace('|', "/"))
                .collect(),
        );
    }
    if rows.is_empty() {
        return String::new();
    }
    let n = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let mut out = String::new();
    for (i, r) in rows.iter().enumerate() {
        out.push('|');
        for c in 0..n {
            out.push_str(&format!(
                " {} |",
                r.get(c).map(|s| s.as_str()).unwrap_or("")
            ));
        }
        out.push('\n');
        if i == 0 {
            out.push('|');
            for _ in 0..n {
                out.push_str(" :-- |");
            }
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

fn chart_icon(p: &egui::Painter, r: Rect, kind: usize, color: Color32) {
    let c = r.center();
    let s = Stroke::new(1.8_f32, color);
    match kind {
        0 => {
            let b = Rect::from_center_size(c, vec2(22.0, 17.0));
            p.rect_stroke(b, 2.0, s, egui::StrokeKind::Inside);
            p.line_segment(
                [
                    pos2(b.left(), b.top() + 5.5),
                    pos2(b.right(), b.top() + 5.5),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(b.left(), b.top() + 11.0),
                    pos2(b.right(), b.top() + 11.0),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(b.left() + 8.0, b.top()),
                    pos2(b.left() + 8.0, b.bottom()),
                ],
                s,
            );
        }
        1 => {
            p.add(egui::Shape::line(
                vec![
                    c + vec2(-11.0, 6.0),
                    c + vec2(-4.0, -2.0),
                    c + vec2(2.0, 3.0),
                    c + vec2(11.0, -7.0),
                ],
                Stroke::new(2.2_f32, color),
            ));
        }
        2 => {
            for (k, h) in [10.0, 15.0, 20.0, 13.0].iter().enumerate() {
                let x = c.x - 10.5 + k as f32 * 6.0;
                p.rect_filled(
                    Rect::from_min_max(pos2(x, c.y + 9.0 - h), pos2(x + 4.5, c.y + 9.0)),
                    1.0,
                    color,
                );
            }
        }
        _ => {
            p.circle_filled(c, 10.0, color);
            p.add(egui::Shape::convex_polygon(
                vec![
                    c,
                    c + vec2(0.0, -11.0),
                    c + vec2(11.0, -11.0),
                    c + vec2(11.0, 0.0),
                ],
                crate::settings::pal().inspector_bg,
                Stroke::NONE,
            ));
        }
    }
}

/// Up/down chevrons shown at the right of a pop-up button.
fn paint_updown(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    let (w, h, gap) = (4.5, 3.5, 3.0);
    p.line_segment([pos2(c.x - w, c.y - gap), pos2(c.x, c.y - gap - h)], s);
    p.line_segment([pos2(c.x, c.y - gap - h), pos2(c.x + w, c.y - gap)], s);
    p.line_segment([pos2(c.x - w, c.y + gap), pos2(c.x, c.y + gap + h)], s);
    p.line_segment([pos2(c.x, c.y + gap + h), pos2(c.x + w, c.y + gap)], s);
}

impl App {
    /// Paints theme `i` as two overlapping cards: dark variant behind,
    /// light variant in front (like the macOS theme pop-up).
    fn paint_theme_stack(&mut self, painter: &egui::Painter, area: Rect, i: usize) {
        let w = area.width() * 0.68;
        let h = w / ASPECT;
        let back = Rect::from_min_size(area.max - vec2(w, h), vec2(w, h));
        let front = Rect::from_min_size(area.min, vec2(w, h));
        let border = crate::settings::pal().border;
        for (r, dark) in [(back, true), (front, false)] {
            let mut theme = self.theme_for(i, dark);
            let look = usize::from(dark);
            // The current theme with a deck gradient: show its first colour.
            if i == self.preset_idx && self.deck_gradient[look] {
                if let Some(&bg) = self.gradient_colors[look].first() {
                    theme.bg = bg;
                    if self.presets[i].gradients[look] != self.gradient_colors[look] {
                        theme.title = crate::theme::contrast_text(bg);
                        theme.body = theme.title;
                    }
                }
            }
            self.paint_slide_with(painter, r, 0, &theme);
            painter.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0_f32, border),
                egui::StrokeKind::Outside,
            );
        }
    }

    /// Compact theme picker: a pill button that opens a list of themes.
    fn theme_combo(&mut self, ui: &mut egui::Ui, order: &[usize]) {
        let pal = crate::settings::pal();
        let thumb = vec2(64.0, 40.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
        let resp = resp.on_hover_text("Choose a theme");
        let popup_id = ui.make_persistent_id("theme-popup");
        if resp.clicked() {
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }
        let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let fill = if resp.hovered() || open {
            pal.hover
        } else {
            pal.control
        };
        ui.painter().rect_filled(rect, rect.height() / 2.0, fill);
        let t = Rect::from_min_size(
            pos2(rect.left() + 14.0, rect.center().y - thumb.y / 2.0),
            thumb,
        );
        let painter = ui.painter_at(rect);
        let cur = self.preset_idx;
        self.paint_theme_stack(&painter, t, cur);
        ui.painter().text(
            pos2(t.right() + 14.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            self.presets[cur].name,
            FontId::proportional(15.0),
            pal.text_hi,
        );
        paint_updown(
            ui.painter(),
            pos2(rect.right() - 22.0, rect.center().y),
            pal.text_hi,
        );

        crate::tabletools::popup_below(
            popup_id,
            &resp,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_width(rect.width() - 12.0);
                egui::ScrollArea::vertical()
                    .max_height(460.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for &i in order {
                            let (row, r) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 48.0),
                                Sense::click(),
                            );
                            let selected = i == self.preset_idx;
                            let p = ui.painter_at(row);
                            if selected {
                                p.rect_filled(row, 6.0, ACCENT_BLUE);
                            } else if r.hovered() {
                                p.rect_filled(row, 6.0, pal.hover);
                            }
                            if selected {
                                let c = pos2(row.left() + 14.0, row.center().y);
                                let s = Stroke::new(2.0_f32, Color32::WHITE);
                                p.line_segment([c + vec2(-5.0, 0.0), c + vec2(-1.5, 4.0)], s);
                                p.line_segment([c + vec2(-1.5, 4.0), c + vec2(5.5, -5.0)], s);
                            }
                            let t = Rect::from_min_size(
                                pos2(row.left() + 30.0, row.center().y - thumb.y / 2.0),
                                thumb,
                            );
                            self.paint_theme_stack(&p, t, i);
                            p.text(
                                pos2(t.right() + 14.0, row.center().y),
                                egui::Align2::LEFT_CENTER,
                                self.presets[i].name,
                                FontId::proportional(15.0),
                                if selected {
                                    Color32::WHITE
                                } else {
                                    pal.text_hi
                                },
                            );
                            if selected && !r.hovered() && egui::Popup::is_id_open(ui.ctx(), popup_id)
                            {
                                // Keep the chosen theme in view when the list opens.
                                if !ui.ctx().memory(|m| {
                                    m.data
                                        .get_temp::<bool>(popup_id.with("scrolled"))
                                        .unwrap_or(false)
                                }) {
                                    r.scroll_to_me(Some(egui::Align::Center));
                                    ui.ctx().memory_mut(|m| {
                                        m.data.insert_temp(popup_id.with("scrolled"), true)
                                    });
                                }
                            }
                            if r.clicked() {
                                self.preset_idx = i;
                                egui::Popup::close_all(ui.ctx());
                            }
                        }
                    });
            },
        );
        if !egui::Popup::is_id_open(ui.ctx(), popup_id) {
            ui.ctx()
                .memory_mut(|m| m.data.remove::<bool>(popup_id.with("scrolled")));
        }
    }

    pub fn format_shortcuts(&mut self, ctx: &egui::Context) {
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let ca = Modifiers::COMMAND | Modifiers::ALT;
        // ⌃⌘ on macOS; Ctrl+Alt elsewhere, where Ctrl and ⌘ are the same key.
        let cc = if IS_MAC {
            Modifiers::COMMAND | Modifiers::CTRL
        } else {
            Modifiers::COMMAND | Modifiers::ALT
        };
        if sc(cs, Key::L) {
            self.set_line_prefix(self.settings.ordered_str());
        } else if sc(ca, Key::L) {
            self.set_line_prefix("- [ ] ");
        } else if sc(Modifiers::COMMAND, Key::L) {
            self.set_line_prefix(self.settings.bullet_str());
        }
        if sc(cs, Key::Minus) {
            self.insert_block("---");
        }
        if sc(cs, Key::Period) {
            self.set_line_prefix("> ");
        }
        for (k, p) in [
            (Key::Num1, "# "),
            (Key::Num2, "## "),
            (Key::Num3, "### "),
            (Key::Num4, "#### "),
        ] {
            if sc(Modifiers::COMMAND, k) {
                self.set_line_prefix(p);
            }
        }
        if sc(Modifiers::COMMAND, Key::B) {
            let m = if self.settings.bold_underscore {
                "__"
            } else {
                "**"
            };
            self.wrap_selection(m, m, "bold");
        }
        if sc(Modifiers::COMMAND, Key::I) {
            let m = if self.settings.italic_underscore {
                "_"
            } else {
                "*"
            };
            self.wrap_selection(m, m, "italic");
        }
        if sc(Modifiers::COMMAND, Key::Slash) {
            self.set_line_prefix("// ");
        }
        if sc(ca, Key::U) {
            self.wrap_selection("~~", "~~", "text");
        } else if sc(cs, Key::U) {
            self.wrap_selection("==", "==", "text");
        }
        if sc(cc, Key::K) {
            self.wrap_selection("[^", "]", "Footnote");
        } else if sc(Modifiers::COMMAND, Key::K) {
            self.wrap_selection("[", "](https://)", "Link");
        }
        if sc(cs, Key::J) {
            self.insert_block("```\n\n```");
        } else if sc(Modifiers::COMMAND, Key::J) {
            self.wrap_selection("`", "`", "code");
        }
        if sc(cc, Key::M) {
            self.wrap_selection("$", "$", "x^2");
        }
    }

    fn add_media(&mut self, video: bool) {
        let mut d = rfd::FileDialog::new();
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            d = d.set_directory(dir);
        }
        d = if video {
            d.add_filter("Video", &["mp4", "mov", "m4v", "webm"])
        } else {
            d.add_filter(
                "Pictures",
                &["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp"],
            )
        };
        if let Some(p) = d.pick_file() {
            let r = self.media_ref(&p);
            self.insert_block(&r);
        }
    }

    fn run(&mut self, a: &Action) {
        match a {
            Action::Prefix(p) => {
                let p = match *p {
                    "- " => self.settings.bullet_str(),
                    "1. " => self.settings.ordered_str(),
                    _ => p,
                };
                self.set_line_prefix(p);
            }
            Action::Wrap(l, r, ph) => {
                let m = match *l {
                    "**" if self.settings.bold_underscore => Some("__"),
                    "*" if self.settings.italic_underscore => Some("_"),
                    _ => None,
                };
                self.wrap_selection(m.unwrap_or(l), m.unwrap_or(r), ph);
            }
            Action::Block(b) => self.insert_block(b),
            Action::AddPicture => self.add_media(false),
            Action::AddVideo => self.add_media(true),
        }
    }

    pub fn inspector(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        egui::Panel::right("inspector")
            .resizable(false)
            .exact_size(INSPECTOR_W)
            .frame(
                egui::Frame::new()
                    .fill(crate::settings::pal().inspector_bg)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show(ui, |ui| {
                let full = ui.max_rect();
                ui.painter().line_segment(
                    [
                        pos2(full.left() - 16.0, full.top()),
                        pos2(full.left() - 16.0, full.bottom()),
                    ],
                    Stroke::new(1.0_f32, crate::settings::pal().divider),
                );
                self.inspector_top(ui, ctx);
                // Scrolls with the wheel/trackpad but never draws a scroll bar.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        // Respect the scroll viewport so controls stay inside the margins.
                        ui.set_width(ui.available_width());
                        match self.tab {
                            Tab::Format => self.format_tab(ui),
                            Tab::Media => self.pictures_tab(ui),
                            Tab::Design => self.design_tab(ui),
                            Tab::Share => self.share_tab(ui),
                        }
                        section_end(ui);
                        ui.add_space(30.0);
                    });
            });
    }

    fn inspector_top(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (top, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
        let cy = top.center().y;
        // segmented control
        let seg = Rect::from_min_size(pos2(top.left(), cy - 16.0), vec2(168.0, 32.0));
        ui.painter()
            .rect_filled(seg, 16.0, crate::settings::pal().control);
        let widths: Vec<f32> = if self.tab == Tab::Share {
            vec![54.0, 54.0, 56.0]
        } else {
            [Tab::Format, Tab::Media, Tab::Design]
                .iter()
                .map(|t| if *t == self.tab { 80.0 } else { 42.0 })
                .collect()
        };
        let mut x = seg.left() + 2.0;
        for (k, t) in [Tab::Format, Tab::Media, Tab::Design]
            .into_iter()
            .enumerate()
        {
            let r = Rect::from_min_size(pos2(x, seg.top() + 2.0), vec2(widths[k], 28.0));
            x += widths[k];
            let resp = ui
                .interact(r, ui.scope_id().with(("seg", k)), Sense::click())
                .on_hover_text(match t {
                    Tab::Format => "Format: Markdown cheat sheet, tables and charts",
                    Tab::Media => "Media: pictures and videos in this presentation",
                    _ => "Design: theme, fonts, colours, header and footer",
                });
            let selected = t == self.tab;
            let p = ui.painter();
            if selected {
                let fill = match t {
                    Tab::Design => DESIGN_PINK,
                    _ => FORMAT_BLUE,
                };
                #[allow(unreachable_patterns)]
                p.rect_filled(r, 14.0, fill);
                let label = match t {
                    Tab::Format => "Format",
                    Tab::Media => "Media",
                    _ => "Design",
                };
                p.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(13.5),
                    Color32::WHITE,
                );
            } else {
                if resp.hovered() {
                    p.rect_filled(r, 14.0, crate::settings::pal().button);
                }
                match t {
                    Tab::Format => {
                        p.text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            "Aa",
                            FontId::proportional(14.5),
                            crate::settings::pal().text_hi,
                        );
                    }
                    Tab::Media => {
                        paint_icon(p, r, Icon::Picture, crate::settings::pal().text_hi)
                    }
                    _ => paint_icon(p, r, Icon::Palette, crate::settings::pal().text_hi),
                }
            }
            if resp.clicked() {
                self.tab = t;
            }
        }
        // Share
        let share_r = Rect::from_min_size(pos2(seg.right() + 6.0, cy - 16.0), vec2(62.0, 32.0));
        let resp = ui
            .interact(share_r, ui.scope_id().with("share"), Sense::click())
            .on_hover_text("Export as PDF, PowerPoint, pictures, web page or Markdown");
        let on = self.tab == Tab::Share;
        let fill = if on {
            SHARE_ORANGE
        } else if resp.hovered() {
            crate::settings::pal().hover
        } else {
            crate::settings::pal().button
        };
        ui.painter().rect_filled(share_r, 16.0, fill);
        ui.painter().text(
            share_r.center(),
            egui::Align2::CENTER_CENTER,
            "Share",
            FontId::proportional(13.5),
            if on {
                Color32::WHITE
            } else {
                crate::settings::pal().text_hi
            },
        );
        if resp.clicked() {
            self.tab = if on { Tab::Design } else { Tab::Share };
        }
        // Play
        let play = Rect::from_center_size(pos2(top.right() - 18.0, cy), vec2(34.0, 34.0));
        let resp = ui
            .interact(play, ui.scope_id().with("play"), Sense::click())
            .on_hover_text(format!("Present ({})", kbd("⌘↵")));
        let p = ui.painter();
        p.circle_filled(
            play.center(),
            17.0,
            if resp.hovered() {
                Color32::from_rgb(0x4C, 0x80, 0xFF)
            } else {
                ACCENT_BLUE
            },
        );
        paint_icon(
            p,
            play.translate(vec2(1.0, 0.0)),
            Icon::Play,
            Color32::WHITE,
        );
        if resp.clicked() {
            self.start_presenting(ctx);
        }
    }

    fn format_tab(&mut self, ui: &mut egui::Ui) {
        let speech = [
            Item {
                icon: "¶",
                label: plain("Spoken Text"),
                shortcut: "Just Type",
                action: Action::Prefix(""),
            },
            Item {
                icon: "1.",
                label: plain("1. Ordered List"),
                shortcut: "⇧⌘L",
                action: Action::Prefix("1. "),
            },
            Item {
                icon: "•",
                label: plain("- Unordered List"),
                shortcut: "⌘L",
                action: Action::Prefix("- "),
            },
            Item {
                icon: "☐",
                label: plain("- [ ] Task List"),
                shortcut: "⌥⌘L",
                action: Action::Prefix("- [ ] "),
            },
            Item {
                icon: "“",
                label: plain("> Blockquote"),
                shortcut: "⇧⌘.",
                action: Action::Prefix("> "),
            },
        ];
        let on_slide = [
            Item {
                icon: "---",
                label: dimmed("", "Slide Break", crate::settings::pal().text_hi, false),
                shortcut: "⇧⌘-",
                action: Action::Block("---"),
            },
            Item {
                icon: "A",
                label: dimmed("→ ", "Body Text", on_slide(), false),
                shortcut: "Tab",
                action: Action::Prefix("\t"),
            },
            Item {
                icon: "H1",
                label: dimmed("", "# Title", on_slide(), true),
                shortcut: "⌘1",
                action: Action::Prefix("# "),
            },
            Item {
                icon: "H2",
                label: dimmed("", "## Subtitle", on_slide(), true),
                shortcut: "⌘2",
                action: Action::Prefix("## "),
            },
            Item {
                icon: "H3",
                label: dimmed("", "### Heading", on_slide(), true),
                shortcut: "⌘3",
                action: Action::Prefix("### "),
            },
            Item {
                icon: "H4",
                label: dimmed("", "#### Subheading", on_slide(), true),
                shortcut: "⌘4",
                action: Action::Prefix("#### "),
            },
            Item {
                icon: "▣",
                label: dimmed("", "/path/image.jpg", on_slide(), false),
                shortcut: "",
                action: Action::AddPicture,
            },
            Item {
                icon: "▶",
                label: dimmed("", "/path/video.mp4", on_slide(), false),
                shortcut: "",
                action: Action::AddVideo,
            },
        ];
        let format = [
            Item {
                icon: "B",
                label: job(&[(
                    "**Bold**",
                    crate::settings::pal().text_hi,
                    true,
                    false,
                    false,
                )]),
                shortcut: "⌘B",
                action: Action::Wrap("**", "**", "bold"),
            },
            Item {
                icon: "I",
                label: plain("*Italic*"),
                shortcut: "⌘I",
                action: Action::Wrap("*", "*", "italic"),
            },
            Item {
                icon: "//",
                label: job(&[(
                    "// Comment",
                    crate::settings::pal().text_dim,
                    false,
                    false,
                    false,
                )]),
                shortcut: "⌘/",
                action: Action::Prefix("// "),
            },
            Item {
                icon: "S",
                label: job(&[(
                    "~~Strikethrough~~",
                    crate::settings::pal().text_hi,
                    false,
                    true,
                    false,
                )]),
                shortcut: "⌥⌘U",
                action: Action::Wrap("~~", "~~", "text"),
            },
            Item {
                icon: "✎",
                label: job(&[(
                    "==Highlight==",
                    Color32::from_rgb(0xF5, 0xD0, 0x30),
                    false,
                    false,
                    true,
                )]),
                shortcut: "⇧⌘U",
                action: Action::Wrap("==", "==", "text"),
            },
            Item {
                icon: "A'",
                label: dimmed("", "[Footnote]", crate::settings::pal().text_hi, false),
                shortcut: "^⌘K",
                action: Action::Wrap("[^", "]", "Footnote"),
            },
            Item {
                icon: "∞",
                label: job(&[
                    (
                        "[Link]",
                        crate::settings::pal().text_hi,
                        false,
                        false,
                        false,
                    ),
                    (
                        "(http://...)",
                        crate::settings::pal().text_dim,
                        false,
                        false,
                        false,
                    ),
                ]),
                shortcut: "⌘K",
                action: Action::Wrap("[", "](https://)", "Link"),
            },
        ];
        let code = [
            Item {
                icon: "{}",
                label: plain("`Code"),
                shortcut: "⌘J",
                action: Action::Wrap("`", "`", "code"),
            },
            Item {
                icon: "[]",
                label: dimmed("``` ", "Code Block", crate::settings::pal().text_hi, false),
                shortcut: "⇧⌘J",
                action: Action::Block("```\n\n```"),
            },
            Item {
                icon: "$",
                label: dimmed("$", "Math$", crate::settings::pal().text_hi, false),
                shortcut: "^⌘M",
                action: Action::Wrap("$", "$", "x^2"),
            },
            Item {
                icon: "$$",
                label: dimmed("$$ ", "Math Block", crate::settings::pal().text_hi, false),
                shortcut: "",
                action: Action::Block("$$\nx^2 + y^2 = z^2\n$$"),
            },
        ];
        let mut todo: Option<&Action> = None;
        if section(ui, "Speech") {
            for it in &speech {
                if item_row(ui, it) {
                    todo = Some(&it.action);
                }
            }
        }
        divider(ui);
        if section(ui, "Text on Slide") {
            for it in &on_slide {
                if item_row(ui, it) {
                    todo = Some(&it.action);
                }
            }
        }
        divider(ui);
        if section(ui, "Format") {
            for it in &format {
                if item_row(ui, it) {
                    todo = Some(&it.action);
                }
            }
            if let Some(a) = todo {
                self.run(a);
                todo = None;
            }
        }
        divider(ui);
        if section(ui, "Table and Charts") {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label("Columns");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    stepper(ui, &mut self.table_cols, 2, 12);
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label("Rows");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    stepper(ui, &mut self.table_rows, 2, 40);
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.checkbox(&mut self.table_values, "Include values");
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label("Display as:");
            });
            ui.add_space(4.0);
            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::hover());
            let w = row.width() / 4.0;
            for (k, name) in ["Table", "Line", "Bar", "Pie"].iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(row.left() + w * k as f32 + 4.0, row.top()),
                    vec2(w - 8.0, 56.0),
                );
                let resp = ui.interact(r, ui.scope_id().with(("disp", k)), Sense::click());
                let sel = self.table_display == k;
                let p = ui.painter();
                if sel {
                    p.rect_filled(r, 8.0, ACCENT_BLUE.gamma_multiply(0.16));
                    p.rect_stroke(
                        r,
                        8.0,
                        Stroke::new(1.5_f32, ACCENT_BLUE),
                        egui::StrokeKind::Inside,
                    );
                } else if resp.hovered() {
                    p.rect_filled(r, 8.0, crate::settings::pal().hover);
                }
                let icon_col = ACCENT_BLUE;
                chart_icon(
                    p,
                    Rect::from_center_size(pos2(r.center().x, r.top() + 20.0), vec2(26.0, 24.0)),
                    k,
                    icon_col,
                );
                p.text(
                    pos2(r.center().x, r.bottom() - 10.0),
                    egui::Align2::CENTER_CENTER,
                    *name,
                    FontId::proportional(12.0),
                    crate::settings::pal().text_hi,
                );
                if resp.clicked() {
                    self.table_display = k;
                }
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let bw = (ui.available_width() - 10.0) / 2.0;
                if ui
                    .add_sized([bw, 28.0], egui::Button::new("Embed CSV…"))
                    .on_hover_text("Insert a table from a CSV file")
                    .clicked()
                {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv", "tsv", "txt"])
                        .pick_file()
                    {
                        if let Ok(t) = std::fs::read_to_string(p) {
                            let mut table = csv_to_table(&t);
                            match self.table_display {
                                1 => table.push_str("\nChart: Line"),
                                2 => table.push_str("\nChart: Bar"),
                                3 => table.push_str("\nChart: Pie"),
                                _ => {}
                            }
                            self.insert_block(&table);
                        }
                    }
                }
                if ui
                    .add_sized(
                        [bw, 28.0],
                        egui::Button::new(RichText::new("Add Table").color(Color32::WHITE))
                            .fill(ACCENT_BLUE)
                            .stroke(Stroke::NONE),
                    )
                    .on_hover_text("Insert this table at the cursor")
                    .clicked()
                {
                    let mut t = make_table(
                        self.table_cols,
                        self.table_rows,
                        self.table_values,
                        self.table_display,
                    );
                    if !self.settings.table_padded {
                        t = compact_table(&t);
                    }
                    self.insert_block(&t);
                }
            });
        }
        divider(ui);
        if section(ui, "Code and Math") {
            for it in &code {
                if item_row(ui, it) {
                    todo = Some(&it.action);
                }
            }
        }
        if let Some(a) = todo {
            self.run(a);
        }
    }

    fn pictures_tab(&mut self, ui: &mut egui::Ui) {
        self.media_library(ui);
        divider(ui);
        if section(ui, "Media Settings") {
            for (k, v) in [
                ("size:", "cover · contain"),
                ("x:", "left · center · right"),
                ("y:", "top · center · bottom"),
                ("background:", "true"),
                ("opacity:", "30%"),
                ("filter:", "grayscale · sepia · darken · lighten · blur"),
                ("title:", "\"Caption\""),
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(k).color(on_slide()).monospace());
                    ui.label(RichText::new(v).color(text_mid()));
                });
            }
            ui.label(
                RichText::new("Write these on the lines right under a picture or video.")
                    .color(text_mid())
                    .size(12.0),
            );
        }
    }

    /// Every picture and video in the package (and any the script uses from
    /// elsewhere), rebuilt only when the script, logo or files change.
    fn media_items(&mut self) -> Vec<MediaItem> {
        use std::hash::{Hash, Hasher};
        let root = self.images.doc_dir.clone();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.text, &root, &self.logo, self.media_epoch).hash(&mut h);
        let key = h.finish();
        if let Some((k, items)) = &self.media_cache {
            if *k == key {
                return items.clone();
            }
        }
        let canon = |p: PathBuf| p.canonicalize().unwrap_or(p);
        let mut items: Vec<MediaItem> = match &root {
            Some(r) => crate::package::list_assets(r)
                .into_iter()
                .map(|p| MediaItem {
                    name: file_label(&p.to_string_lossy()),
                    src: crate::package::asset_ref(r, &p),
                    video: crate::package::is_video_file(&p),
                    path: Some(canon(p)),
                    slides: Vec::new(),
                    logo: false,
                    place: Place::Package,
                })
                .collect(),
            None => Vec::new(),
        };
        let mut refs: Vec<(Option<usize>, String)> = Vec::new();
        for (i, s) in self.slides.iter().enumerate() {
            for b in &s.backgrounds {
                refs.push((Some(i), b.src.clone()));
            }
            for g in &s.groups {
                for bl in g {
                    if let Block::Image(img) = bl {
                        refs.push((Some(i), img.src.clone()));
                    }
                }
            }
        }
        if !self.logo.trim().is_empty() {
            refs.push((None, self.logo.trim().to_string()));
        }
        let root_canon = root.clone().map(canon);
        for (slide, src) in refs {
            let resolved = self.images.resolve(&src).filter(|p| p.is_file()).map(canon);
            let found = match &resolved {
                Some(p) => items.iter().position(|it| it.path.as_ref() == Some(p)),
                None => items.iter().position(|it| it.src == src),
            };
            let idx = found.unwrap_or_else(|| {
                let place = if crate::images::is_web(&src) {
                    Place::Web
                } else {
                    match &resolved {
                        Some(p) if root_canon.as_ref().is_some_and(|r| p.starts_with(r)) => {
                            Place::Package
                        }
                        Some(_) => Place::Outside,
                        None => Place::Missing,
                    }
                };
                items.push(MediaItem {
                    name: file_label(&src),
                    video: crate::images::is_video(&src),
                    src: src.clone(),
                    path: resolved.clone(),
                    slides: Vec::new(),
                    logo: false,
                    place,
                });
                items.len() - 1
            });
            match slide {
                Some(i) if !items[idx].slides.contains(&i) => items[idx].slides.push(i),
                Some(_) => {}
                None => items[idx].logo = true,
            }
        }
        self.media_cache = Some((key, items.clone()));
        items
    }

    /// Asks for pictures or videos and adds them to the package.
    fn pick_media(&mut self) {
        let files = rfd::FileDialog::new()
            .set_title("Add images and videos to the presentation")
            .add_filter(
                "Images and videos",
                &[
                    "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "svg", "heic",
                    "avif", "mp4", "mov", "m4v", "webm",
                ],
            )
            .pick_files()
            .unwrap_or_default();
        for f in &files {
            let _ = self.media_ref(f);
        }
        if !files.is_empty() {
            self.status = format!(
                "Added {} file{}. Double-click one to put it on the current slide.",
                files.len(),
                if files.len() == 1 { "" } else { "s" }
            );
        }
    }

    /// The media library: drop box, search, and Images / Videos lists.
    fn media_library(&mut self, ui: &mut egui::Ui) {
        let pal = crate::settings::pal();
        ui.add_space(12.0);
        let full = ui.available_width();

        // Drop zone: full width, icon with a title and a "browse" hint.
        let (zone, _) = ui.allocate_exact_size(vec2(full, 104.0), Sense::hover());
        let zone_resp = ui
            .interact(zone, ui.scope_id().with("media-drop"), Sense::click())
            .on_hover_text("Click to choose files, or drop them here")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let dragging_here = ui.ctx().input(|i| {
            !i.raw.hovered_files.is_empty()
                && i.pointer.latest_pos().is_some_and(|p| zone.expand(6.0).contains(p))
        });
        let active = dragging_here || zone_resp.hovered();
        let fill = if dragging_here {
            FORMAT_BLUE.gamma_multiply(0.10)
        } else if active {
            pal.hover
        } else {
            pal.field.gamma_multiply(0.6)
        };
        ui.painter().rect_filled(zone, 10.0, fill);
        dashed_round_rect(
            ui.painter(),
            zone.shrink(0.5),
            10.0,
            Stroke::new(
                1.2_f32,
                if dragging_here {
                    FORMAT_BLUE
                } else if active {
                    pal.text_dim
                } else {
                    pal.text_dim.gamma_multiply(0.55)
                },
            ),
        );
        let icon_c = pos2(zone.center().x, zone.top() + 32.0);
        ui.painter().circle_filled(
            icon_c,
            16.0,
            if dragging_here { FORMAT_BLUE.gamma_multiply(0.18) } else { pal.button },
        );
        paint_icon(
            ui.painter(),
            Rect::from_center_size(icon_c, vec2(24.0, 24.0)),
            Icon::Picture,
            if dragging_here { FORMAT_BLUE } else { pal.text_hi },
        );
        ui.painter().text(
            pos2(zone.center().x, zone.top() + 63.0),
            egui::Align2::CENTER_CENTER,
            if dragging_here { "Release to add" } else { "Drop images or videos" },
            FontId::proportional(13.5),
            pal.text_hi,
        );
        ui.painter().text(
            pos2(zone.center().x, zone.top() + 82.0),
            egui::Align2::CENTER_CENTER,
            "or click to browse",
            FontId::proportional(12.0),
            if dragging_here { text_mid() } else { on_slide() },
        );
        self.media_drop_rect = Some(zone.expand(6.0).intersect(ui.clip_rect()));
        if zone_resp.clicked() {
            self.pick_media();
        }

        // Search field and options menu on one row.
        ui.add_space(10.0);
        const ROW_H: f32 = 30.0;
        const GEAR_W: f32 = 44.0;
        let (row, _) = ui.allocate_exact_size(vec2(full, ROW_H), Sense::hover());
        let search_rect =
            Rect::from_min_max(row.min, pos2(row.right() - GEAR_W - 8.0, row.bottom()));
        let gear = Rect::from_min_max(pos2(row.right() - GEAR_W, row.top()), row.max);

        // Same look as the editor's Find field: rounded box, magnifier, clear button.
        let field_id = ui.scope_id().with("media-search");
        let focused = ui.ctx().memory(|m| m.has_focus(field_id));
        let ring = if focused { ui.visuals().selection.stroke.color } else { pal.border };
        ui.painter().rect_filled(search_rect, 8.0, pal.field);
        ui.painter().rect_stroke(
            search_rect,
            8.0,
            Stroke::new(if focused { 2.0 } else { 1.0 }, ring),
            egui::StrokeKind::Inside,
        );
        let y = search_rect.center().y;
        let glass = pos2(search_rect.left() + 15.0, y - 1.0);
        let st = Stroke::new(1.4_f32, pal.icon);
        ui.painter().circle_stroke(glass, 5.0, st);
        ui.painter()
            .line_segment([glass + vec2(3.6, 3.6), glass + vec2(7.0, 7.0)], st);
        let has_query = !self.media_search.is_empty();
        let clear = Rect::from_center_size(pos2(search_rect.right() - 14.0, y), vec2(18.0, 18.0));
        if has_query {
            let r = ui
                .interact(clear, ui.scope_id().with("media-search-clear"), Sense::click())
                .on_hover_text("Clear");
            let c = clear.center();
            ui.painter().circle_filled(
                c,
                7.5,
                if r.hovered() { pal.text_dim } else { pal.icon.gamma_multiply(0.7) },
            );
            let x = Stroke::new(1.4_f32, pal.field);
            ui.painter().line_segment([c + vec2(-2.8, -2.8), c + vec2(2.8, 2.8)], x);
            ui.painter().line_segment([c + vec2(-2.8, 2.8), c + vec2(2.8, -2.8)], x);
            if r.clicked() {
                self.media_search.clear();
            }
        }
        let text_right = if has_query { clear.left() - 4.0 } else { search_rect.right() - 8.0 };
        let text_rect = Rect::from_min_max(
            pos2(search_rect.left() + 28.0, search_rect.top() + 2.0),
            pos2(text_right, search_rect.bottom() - 2.0),
        );
        let mut search_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
        egui::TextEdit::singleline(&mut self.media_search)
            .id(field_id)
            .frame(egui::Frame::NONE)
            .hint_text("Search media")
            .font(FontId::proportional(14.0))
            .text_color(pal.text_hi)
            .desired_width(text_rect.width())
            .align(egui::Align2::LEFT_CENTER)
            .min_size(text_rect.size())
            .show(&mut search_ui);

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(gear));
        let btn = egui::Button::new("")
            .min_size(gear.size())
            .corner_radius(6.0);
        let menu = crate::tabletools::menu_from_button(&mut child, btn, |ui| {
            crate::tabletools::style_menu(ui);
            if ui.button("Add Images or Videos…").clicked() {
                ui.close();
                self.pick_media();
            }
            let assets = self
                .images
                .doc_dir
                .as_ref()
                .map(|d| d.join(crate::package::ASSETS))
                .filter(|d| d.is_dir());
            if ui
                .add_enabled(assets.is_some(), egui::Button::new("Show Media Folder"))
                .clicked()
            {
                ui.close();
                if let Some(d) = assets {
                    crate::open_external(d.as_os_str());
                }
            }
            ui.separator();
            if ui.button("Clear Thumbnails Cache").clicked() {
                ui.close();
                self.images.clear_cache();
                self.media_cache = None;
            }
        });
        let gr = menu.rect;
        paint_icon(
            ui.painter(),
            Rect::from_min_size(pos2(gr.left() + 4.0, gr.top()), vec2(22.0, gr.height())),
            Icon::Gear,
            pal.text_hi,
        );
        paint_icon(
            ui.painter(),
            Rect::from_min_size(pos2(gr.left() + 24.0, gr.top()), vec2(16.0, gr.height())),
            Icon::ChevronDown,
            pal.text_dim,
        );
        menu.on_hover_text("Media options");
        ui.add_space(6.0);

        let items = self.media_items();
        let needle = self.media_search.trim().to_lowercase();
        let outside = items
            .iter()
            .filter(|it| it.place == Place::Outside)
            .count();
        if outside > 0 {
            ui.label(
                RichText::new(format!(
                    "{outside} file{} outside the presentation. Saving copies {} in.",
                    if outside == 1 { " is" } else { "s are" },
                    if outside == 1 { "it" } else { "them" }
                ))
                .color(pal.text_dim)
                .size(12.0),
            );
        }
        if items.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "No media yet. Drop pictures or videos above, or onto the editor to put them on a slide.",
                )
                .color(pal.text_dim),
            );
            return;
        }
        let accents = self.theme().accents;
        let mut action = None;
        for (title, video) in [("Images", false), ("Videos", true)] {
            let list: Vec<&MediaItem> = items
                .iter()
                .filter(|it| it.video == video)
                .filter(|it| needle.is_empty() || it.name.to_lowercase().contains(&needle))
                .collect();
            let total = items.iter().filter(|it| it.video == video).count();
            if total == 0 {
                continue;
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("{title} ({})", list.len()))
                    .size(13.5)
                    .strong()
                    .color(pal.text_hi),
            );
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, pal.divider);
            ui.add_space(4.0);
            for it in list {
                if let Some(a) = self.media_row(ui, it, &accents) {
                    action = Some(a);
                }
            }
        }
        if let Some(a) = action {
            self.run_media_action(a);
        }
    }

    fn media_row(
        &mut self,
        ui: &mut egui::Ui,
        it: &MediaItem,
        accents: &[Color32; 6],
    ) -> Option<MediaAction> {
        let pal = crate::settings::pal();
        let w = ui.available_width();
        let (row, resp) = ui.allocate_exact_size(vec2(w, 74.0), Sense::click());
        let painter = ui.painter_at(row);
        if resp.hovered() {
            painter.rect_filled(row, 6.0, pal.hover);
        }
        let thumb = Rect::from_min_size(row.min + vec2(4.0, 5.0), vec2(96.0, 64.0));
        let mut spec = crate::parse::new_image(it.src.clone(), None);
        spec.contain = false;
        crate::images::draw_image(&painter, thumb, &spec, &mut self.images, accents, Some(false));
        let tx = thumb.right() + 14.0;
        let name_galley = painter.layout_no_wrap(
            it.name.clone(),
            FontId::proportional(14.5),
            pal.text_hi,
        );
        let clip = Rect::from_min_max(pos2(tx, row.top()), pos2(row.right() - 4.0, row.bottom()));
        painter
            .with_clip_rect(clip)
            .galley(pos2(tx, row.top() + 16.0), name_galley, pal.text_hi);
        let mut sub = match it.place {
            Place::Web => "Web address".to_string(),
            Place::Missing => "Missing file".to_string(),
            Place::Outside => "Not in package".to_string(),
            Place::Package => String::new(),
        };
        let mut used = Vec::new();
        if !it.slides.is_empty() {
            let nums: Vec<String> = it.slides.iter().map(|i| (i + 1).to_string()).collect();
            used.push(format!(
                "Slide{} {}",
                if nums.len() == 1 { "" } else { "s" },
                nums.join(", ")
            ));
        }
        if it.logo {
            used.push("Logo".into());
        }
        let usage = if used.is_empty() {
            "Not used".to_string()
        } else {
            used.join(" · ")
        };
        if sub.is_empty() {
            sub = usage;
        } else {
            sub = format!("{sub} · {usage}");
        }
        painter.with_clip_rect(clip).text(
            pos2(tx, row.top() + 42.0),
            egui::Align2::LEFT_TOP,
            sub,
            FontId::proportional(12.5),
            if it.place == Place::Missing {
                Color32::from_rgb(0xD9, 0x48, 0x3B)
            } else {
                pal.text_dim
            },
        );
        let mut action = None;
        if resp.double_clicked() {
            action = Some(MediaAction::Insert(it.src.clone()));
        } else if resp.clicked() {
            if let Some(&i) = it.slides.first() {
                action = Some(MediaAction::Show(i));
            }
        }
        let removable = it.place == Place::Package && it.slides.is_empty() && !it.logo;
        resp.on_hover_text("Double-click to put it on the current slide")
            .context_menu(|ui| {
                crate::tabletools::style_menu(ui);
                if ui.button("Insert at Cursor").clicked() {
                    ui.close();
                    action = Some(MediaAction::Insert(it.src.clone()));
                }
                if let Some(p) = &it.path {
                    if ui.button(REVEAL_LABEL).clicked() {
                        ui.close();
                        reveal(p);
                    }
                    if it.place == Place::Outside && ui.button("Copy into Presentation").clicked() {
                        ui.close();
                        action = Some(MediaAction::CopyIn(it.src.clone(), p.clone()));
                    }
                }
                ui.separator();
                if ui
                    .add_enabled(removable, egui::Button::new("Remove from Presentation…"))
                    .on_disabled_hover_text("Only media no slide uses can be removed")
                    .clicked()
                {
                    ui.close();
                    if let Some(p) = &it.path {
                        action = Some(MediaAction::Remove(p.clone()));
                    }
                }
            });
        action
    }

    fn run_media_action(&mut self, a: MediaAction) {
        match a {
            MediaAction::Show(i) => self.select_slide(i),
            MediaAction::Insert(src) => self.insert_block(&src),
            MediaAction::CopyIn(old, path) => {
                let new = self.media_ref(&path);
                if new != old {
                    self.text = crate::package::replace_media_ref(&self.text, &old, &new);
                    if self.logo.trim() == old {
                        self.logo = new;
                    }
                    let n = self.text.chars().count();
                    let c = self.cursor.min(n);
                    self.after_edit(c, c);
                }
            }
            MediaAction::Remove(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let ok = rfd::MessageDialog::new()
                    .set_title("Remove from presentation?")
                    .set_description(format!(
                        "{name} will be deleted from the presentation's media folder."
                    ))
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show();
                if ok == rfd::MessageDialogResult::Ok {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            self.media_epoch += 1;
                            if self.doc_form == Some(crate::package::Form::Zip) {
                                self.dirty = true;
                            }
                            self.status = format!("Removed {name}");
                        }
                        Err(e) => self.status = format!("Couldn't remove {name}: {e}"),
                    }
                }
            }
        }
    }

    fn share_tab(&mut self, ui: &mut egui::Ui) {
        let pal = crate::settings::pal();
        // Deck summary: cover preview, name, slide count and speaking time.
        ui.add_space(12.0);
        let w = ui.available_width();
        let preview_h = w / ASPECT;
        let (card, _) = ui.allocate_exact_size(vec2(w, preview_h + 58.0), Sense::hover());
        let preview = Rect::from_min_size(card.min, vec2(w, preview_h));
        {
            let painter = ui.painter_at(card.expand(4.0));
            painter.rect_filled(card, 8.0, pal.field);
            self.paint_slide(&painter, preview, 0);
            painter.rect_stroke(
                card,
                8.0,
                Stroke::new(1.0_f32, pal.border),
                egui::StrokeKind::Outside,
            );
            painter.line_segment(
                [preview.left_bottom(), preview.right_bottom()],
                Stroke::new(1.0_f32, pal.border),
            );
            painter.text(
                pos2(card.left() + 12.0, preview.bottom() + 18.0),
                egui::Align2::LEFT_CENTER,
                self.doc_name_pub(),
                FontId::new(14.5, crate::theme::family(false, true, false)),
                pal.text_hi,
            );
            let n = self.slides.len();
            let secs = self.speaking_seconds() as u32;
            painter.text(
                pos2(card.left() + 12.0, preview.bottom() + 39.0),
                egui::Align2::LEFT_CENTER,
                format!(
                    "{n} slide{} · about {} min speaking",
                    if n == 1 { "" } else { "s" },
                    ((secs + 30) / 60).max(1)
                ),
                FontId::proportional(12.0),
                pal.text_dim,
            );
        }

        if section(ui, "Export") {
            let rows: [(&str, Color32, &str, &str, Option<crate::export::Format>); 5] = [
                (
                    "PDF",
                    Color32::from_rgb(0xE5, 0x48, 0x4D),
                    "PDF",
                    "One page per slide. Best for sending and printing.",
                    Some(crate::export::Format::Pdf),
                ),
                (
                    "PPT",
                    Color32::from_rgb(0xD9, 0x6C, 0x2C),
                    "PowerPoint",
                    "Editable text. Opens in Keynote and Google Slides.",
                    Some(crate::export::Format::PowerPoint),
                ),
                (
                    "PNG",
                    Color32::from_rgb(0x30, 0xA4, 0x6C),
                    "Images",
                    "A picture of every slide, saved into a folder.",
                    Some(crate::export::Format::Images),
                ),
                (
                    "HTML",
                    ACCENT_BLUE,
                    "Web Page",
                    "One page to host anywhere. Press N for notes.",
                    Some(crate::export::Format::Html),
                ),
                (
                    "MD",
                    Color32::from_rgb(0x6E, 0x6E, 0x73),
                    "Markdown",
                    "The plain-text script, slides and notes included.",
                    None,
                ),
            ];
            let mut start = None;
            let mut md = false;
            for (k, (tag, color, name, note, fmt)) in rows.iter().enumerate() {
                if export_row(ui, k, tag, *color, name, note) {
                    match fmt {
                        Some(f) => start = Some(*f),
                        None => md = true,
                    }
                }
            }
            if let Some(f) = start {
                self.start_export(f);
            }
            if md {
                self.export_markdown();
            }
        }
        divider(ui);

        if section(ui, "Document") {
            let gap = ui.spacing().item_spacing.x;
            let bw = (ui.available_width() - gap) / 2.0;
            ui.horizontal(|ui| {
                if ui
                    .add_sized([bw, 28.0], egui::Button::new("New"))
                    .on_hover_text(kbd("⌘N"))
                    .clicked()
                {
                    self.new_doc();
                }
                if ui
                    .add_sized([bw, 28.0], egui::Button::new("Open…"))
                    .on_hover_text(kbd("⌘O"))
                    .clicked()
                {
                    self.open_dialog();
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_sized([bw, 28.0], egui::Button::new("Save"))
                    .on_hover_text(kbd("⌘S"))
                    .clicked()
                {
                    self.save(false);
                }
                if ui
                    .add_sized([bw, 28.0], egui::Button::new("Save As…"))
                    .on_hover_text(kbd("⇧⌘S"))
                    .clicked()
                {
                    self.save(true);
                }
            });
            let where_ = match &self.path {
                Some(p) => p.display().to_string(),
                None => "Not saved yet".to_string(),
            };
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(RichText::new(where_).color(pal.text_dim).size(11.5))
                    .selectable(false)
                    .truncate(),
            );
        }
    }

    fn design_tab(&mut self, ui: &mut egui::Ui) {
        // Preview of the current slide, with previous / next buttons.
        ui.add_space(6.0);
        let n = self.slides.len();
        let c = self.current;
        let w = ui.available_width();
        let (card, _) = ui.allocate_exact_size(vec2(w, w / ASPECT), Sense::hover());
        {
            let painter = ui.painter_at(card.expand(6.0));
            painter.rect_filled(
                card.translate(vec2(0.0, 2.0)).expand(1.0),
                4.0,
                Color32::from_black_alpha(40),
            );
            self.paint_slide(&painter, card, c);
            painter.rect_stroke(
                card,
                2.0,
                Stroke::new(1.0_f32, crate::settings::pal().border),
                egui::StrokeKind::Outside,
            );
        }
        if ui
            .interact(card, ui.scope_id().with("design-preview"), Sense::click())
            .on_hover_text("Double-click to present")
            .double_clicked()
        {
            self.start_presenting(ui.ctx());
        }
        ui.add_space(4.0);
        let (bar, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
        let prev_r =
            Rect::from_center_size(pos2(bar.left() + 15.0, bar.center().y), vec2(28.0, 26.0));
        let next_r =
            Rect::from_center_size(pos2(bar.right() - 15.0, bar.center().y), vec2(28.0, 26.0));
        ui.painter().text(
            bar.center(),
            egui::Align2::CENTER_CENTER,
            format!("Slide {} of {}", c + 1, n),
            FontId::proportional(12.0),
            crate::settings::pal().text_dim,
        );
        if c > 0 && icon_button(ui, prev_r, Icon::ChevronLeft, "Previous slide").clicked() {
            self.select_slide(c - 1);
        }
        if c + 1 < n && icon_button(ui, next_r, Icon::ChevronRight, "Next slide").clicked() {
            self.select_slide(c + 1);
        }
        divider(ui);

        if section(ui, "Theme") {
            let before = self.preset_idx;
            // Styled themes first, then the classic flat ones.
            let mut order: Vec<usize> = (0..self.presets.len())
                .filter(|i| self.presets[*i].decor != crate::theme::Decor::None)
                .collect();
            order.extend(
                (0..self.presets.len()).filter(|i| self.presets[*i].decor == crate::theme::Decor::None),
            );
            self.theme_combo(ui, &order);
            ui.add_space(4.0);
            if before != self.preset_idx {
                self.apply_preset(self.preset_idx);
            }
        }
        divider(ui);

        if section(ui, "Fonts") {
            for (label, which) in [("Title Font", 0), ("Body Font", 1)] {
                let default = if which == 0 {
                    self.presets[self.preset_idx].title_face
                } else {
                    self.presets[self.preset_idx].body_face
                };
                let font_label = |c: FontChoice| match c {
                    FontChoice::Theme => format!("{} — Theme default", default.name()),
                    FontChoice::Sans => crate::fonts::Face::SANS.name(),
                    FontChoice::Serif => crate::fonts::Face::SERIF.name(),
                    FontChoice::Named(f) => f.name(),
                };
                labeled(ui, label, |ui| {
                    let v = if which == 0 {
                        &mut self.title_font
                    } else {
                        &mut self.body_font
                    };
                    font_combo(ui, label, v, &font_label);
                });
            }
            labeled(ui, "Size", |ui| {
                let options: Vec<(usize, &str)> =
                    SIZES.iter().enumerate().map(|(i, (n, _))| (i, *n)).collect();
                menu_combo(ui, "size", &mut self.size_idx, &options);
            });
        }
        divider(ui);

        if section(ui, "Colors") {
            labeled(ui, "Appearance", |ui| {
                ui.radio_value(&mut self.dark, false, "Light");
                ui.add_space(10.0);
                ui.radio_value(&mut self.dark, true, "Dark");
            });
            labeled(ui, "", |ui| {
                let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                for name in ["Light slides", "Dark slides"] {
                    let (r, _) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        name,
                        FontId::proportional(11.0),
                        crate::settings::pal().text_dim,
                    );
                }
            });
            let p = self.presets[self.preset_idx].clone();
            let mut light = self.light_override.unwrap_or(p.light);
            let mut dark = self.dark_override.unwrap_or(p.dark);
            let (l0, d0) = (light, dark);
            for (label, k) in [("Titles", 0), ("Body", 1), ("Background", 2)] {
                labeled(ui, label, |ui| {
                    let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                    if k == 2 {
                        background_swatch(
                            ui,
                            &mut light.bg,
                            &mut self.deck_gradient[0],
                            &mut self.gradient_colors[0],
                            w,
                        );
                        background_swatch(
                            ui,
                            &mut dark.bg,
                            &mut self.deck_gradient[1],
                            &mut self.gradient_colors[1],
                            w,
                        );
                    } else {
                        swatch(ui, pick(&mut light, k), w);
                        swatch(ui, pick(&mut dark, k), w);
                    }
                });
            }
            if light != l0 {
                self.light_override = Some(light);
            }
            if dark != d0 {
                self.dark_override = Some(dark);
            }
            labeled(ui, "", |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Button::new("Reset Colors"),
                    )
                    .on_hover_text("Go back to the theme's colours")
                    .clicked()
                {
                    self.light_override = None;
                    self.dark_override = None;
                }
            });
        }
        divider(ui);

        if section(ui, "Header & Footer") {
            self.header_footer(ui);
        }
        divider(ui);
        self.logo_section(ui);
        divider(ui);

        if section(ui, "Metadata") {
            labeled(ui, "Author", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.author).desired_width(ui.available_width()),
                );
            });
            labeled(ui, "Audience", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.audience).desired_width(ui.available_width()),
                );
            });
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new("Design settings apply only to this presentation.")
                .color(crate::settings::pal().text_dim)
                .size(11.5),
        );
    }
}

impl App {
    /// Six-place header/footer picker: click a place in the preview, then
    /// choose what it shows.
    fn header_footer(&mut self, ui: &mut egui::Ui) {
        use crate::theme::{SlotKind, SLOT_NAMES};
        let pal = crate::settings::pal();
        self.today = crate::theme::today();
        let w = ui.available_width();
        let h = (w * 0.42).round();
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let r = r.shrink2(vec2(18.0, 6.0));
        let row_h = (r.height() * 0.26).round();
        let col_w = r.width() / 3.0;
        let dash = |a: egui::Pos2, b: egui::Pos2| {
            ui.painter().extend(egui::Shape::dashed_line(
                &[a, b],
                Stroke::new(1.0_f32, pal.text_dim.gamma_multiply(0.6)),
                4.0,
                3.0,
            ))
        };
        for i in 0..6 {
            let (row, col) = (i / 3, i % 3);
            let y = if row == 0 {
                r.top()
            } else {
                r.bottom() - row_h
            };
            let cell =
                Rect::from_min_size(pos2(r.left() + col as f32 * col_w, y), vec2(col_w, row_h));
            let resp = ui
                .interact(cell, ui.scope_id().with(("hf-slot", i)), Sense::click())
                .on_hover_text(SLOT_NAMES[i]);
            if resp.clicked() {
                self.slot_sel = i;
            }
            let selected = self.slot_sel == i;
            let text = if self.slots[i].kind == SlotKind::Logo {
                "◇ Logo".to_owned()
            } else {
                self.slots[i].resolve(self.current + 1, &self.today)
            };
            let fill = if selected {
                crate::FORMAT_BLUE
            } else if resp.hovered() {
                pal.hover
            } else if !text.is_empty() {
                pal.control
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(cell.shrink(1.0), 3.0, fill);
            if !text.is_empty() {
                let color = if selected {
                    Color32::WHITE
                } else {
                    pal.text_hi
                };
                let font = FontId::proportional(11.5);
                let g = ui.painter().layout_no_wrap(text, font, color);
                let anchor = [
                    egui::Align2::LEFT_CENTER,
                    egui::Align2::CENTER_CENTER,
                    egui::Align2::RIGHT_CENTER,
                ][col];
                let at = match col {
                    0 => pos2(cell.left() + 8.0, cell.center().y),
                    1 => cell.center(),
                    _ => pos2(cell.right() - 8.0, cell.center().y),
                };
                let gr = anchor.anchor_size(at, g.size());
                ui.painter()
                    .with_clip_rect(cell.shrink(2.0))
                    .galley(gr.min, g, color);
            }
        }
        // dashed outline, row lines and column dividers
        let (tl, tr, bl, br) = (
            r.left_top(),
            r.right_top(),
            r.left_bottom(),
            r.right_bottom(),
        );
        dash(tl, tr);
        dash(tr, br);
        dash(br, bl);
        dash(bl, tl);
        let (y1, y2) = (r.top() + row_h, r.bottom() - row_h);
        dash(pos2(r.left(), y1), pos2(r.right(), y1));
        dash(pos2(r.left(), y2), pos2(r.right(), y2));
        for c in 1..3 {
            let x = r.left() + c as f32 * col_w;
            dash(pos2(x, r.top()), pos2(x, y1));
            dash(pos2(x, y2), pos2(x, r.bottom()));
        }
        ui.add_space(6.0);

        let i = self.slot_sel.min(5);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(SLOT_NAMES[i]).color(pal.text_dim).size(11.5));
        });
        ui.add_space(4.0);
        let slot = &mut self.slots[i];
        labeled(ui, "Shows", |ui| {
            let options: Vec<(SlotKind, &str)> =
                SlotKind::ALL.iter().map(|k| (*k, k.label())).collect();
            menu_combo(ui, ("hf-kind", i), &mut slot.kind, &options);
        });
        if slot.kind == SlotKind::Custom {
            labeled(ui, "Text", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut slot.text)
                        .id(ui.scope_id().with(("hf-text", i)))
                        .desired_width(ui.available_width())
                        .hint_text("Footer text"),
                );
            });
        }
    }

    /// The deck's one logo: image, size, and a hint when no place shows it.
    fn logo_section(&mut self, ui: &mut egui::Ui) {
        use crate::theme::SlotKind;
        let pal = crate::settings::pal();
        self.logo_drop_rect = None;
        if !section(ui, "Logo") {
            return;
        }
        ui.add_space(6.0);

        // Drop box on the left, stacked buttons on the right.
        let full = ui.available_width();
        let gap = 12.0;
        let right_w = (full * 0.5).round();
        let left_w = full - right_w - gap;
        let btn_h = 30.0;
        let (area, _) = ui.allocate_exact_size(vec2(full, btn_h * 2.0 + 8.0), Sense::hover());
        let left = Rect::from_min_size(area.min, vec2(left_w, area.height()));
        let preview = Rect::from_center_size(left.center(), vec2((left_w - 24.0).min(120.0), 52.0));
        self.logo_drop_rect = Some(preview.expand(8.0).intersect(ui.clip_rect()));
        let image = if self.logo.is_empty() {
            None
        } else {
            self.images.get(ui.ctx(), &self.logo, None)
        };
        if let Some((texture, size)) = image {
            let scale = (preview.width() / size[0] as f32).min(preview.height() / size[1] as f32);
            ui.painter().image(
                texture,
                Rect::from_center_size(
                    preview.center(),
                    vec2(size[0] as f32, size[1] as f32) * scale,
                ),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            dashed_round_rect(
                ui.painter(),
                preview,
                10.0,
                Stroke::new(2.0_f32, pal.icon),
            );
            ui.painter().text(
                preview.center(),
                egui::Align2::CENTER_CENTER,
                if self.logo.is_empty() {
                    "Drop Logo"
                } else {
                    "Unavailable"
                },
                FontId::proportional(13.0),
                pal.text_dim,
            );
        }
        let right_x = area.right() - right_w;
        let select = Rect::from_min_size(pos2(right_x, area.top()), vec2(right_w, btn_h));
        let remove = Rect::from_min_size(pos2(right_x, select.bottom() + 8.0), vec2(right_w, btn_h));
        if flat_button(ui, select, "Select Logo", true).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                .pick_file()
            {
                self.logo = self.media_ref(&path);
            }
        }
        if flat_button(ui, remove, "Remove Logo", !self.logo.is_empty()).clicked() {
            self.logo.clear();
        }

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Size").size(13.0).color(pal.text_hi));
            ui.add_space(6.0);
            pill_slider(ui, &mut self.logo_size, 0.02..=0.08, 7);
        });
        if !self.slots.iter().any(|s| s.kind == SlotKind::Logo) {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Set any header or footer place to “Logo” to show it.")
                    .size(11.5)
                    .color(pal.text_dim),
            );
        }
    }
}

/// Rounded rectangle outline drawn as short dashes.
fn dashed_round_rect(p: &egui::Painter, r: Rect, radius: f32, stroke: Stroke) {
    let rad = radius.min(r.height() / 2.0).min(r.width() / 2.0);
    let mut pts = Vec::new();
    let corners = [
        (pos2(r.right() - rad, r.top() + rad), -90.0_f32),
        (pos2(r.right() - rad, r.bottom() - rad), 0.0),
        (pos2(r.left() + rad, r.bottom() - rad), 90.0),
        (pos2(r.left() + rad, r.top() + rad), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=8 {
            let a = (start + 90.0 * k as f32 / 8.0).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    pts.push(pts[0]);
    p.extend(egui::Shape::dashed_line(&pts, stroke, 4.0, 3.5));
}

/// One picture or video in the Media panel.
#[derive(Clone)]
pub struct MediaItem {
    pub name: String,
    /// How the script refers to it.
    pub src: String,
    pub path: Option<PathBuf>,
    pub video: bool,
    /// Slides (0-based) that show it.
    pub slides: Vec<usize>,
    pub logo: bool,
    pub place: Place,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Place {
    Package,
    Outside,
    Web,
    Missing,
}

enum MediaAction {
    Show(usize),
    Insert(String),
    CopyIn(String, PathBuf),
    Remove(PathBuf),
}

/// The file name a reference shows, with `%20` turned back into spaces.
fn file_label(src: &str) -> String {
    let last = src.trim_end_matches('/').rsplit(['/', '\\']).next().unwrap_or(src);
    let last = last.split(['?', '#']).next().unwrap_or(last);
    last.replace("%20", " ")
}

const REVEAL_LABEL: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(windows) {
    "Show in Explorer"
} else {
    "Show in Folder"
};

/// Shows a file selected in the system file browser.
fn reveal(path: &std::path::Path) {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("explorer");
        let mut arg = std::ffi::OsString::from("/select,");
        arg.push(path);
        c.arg(arg);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    let _ = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Flat rounded button that fills a given rect.
fn flat_button(ui: &mut egui::Ui, r: Rect, label: &str, enabled: bool) -> egui::Response {
    let pal = crate::settings::pal();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let resp = ui.interact(r, ui.scope_id().with(("flat_button", label)), sense);
    let fill = if !enabled {
        pal.button.gamma_multiply(0.55)
    } else if resp.is_pointer_button_down_on() {
        pal.hover.gamma_multiply(0.9)
    } else if resp.hovered() {
        pal.hover
    } else {
        pal.button
    };
    ui.painter().rect_filled(r, 7.0, fill);
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        if enabled { pal.text_hi } else { pal.text_dim },
    );
    resp
}

/// Slider with a blue filled track, white pill knob and tick marks below.
fn pill_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    ticks: usize,
) {
    let pal = crate::settings::pal();
    let (lo, hi) = (*range.start(), *range.end());
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click_and_drag());
    let knob = vec2(22.0, 16.0);
    let track_y = r.center().y - 1.0;
    let x0 = r.left() + knob.x / 2.0;
    let x1 = r.right() - knob.x / 2.0;
    if let Some(p) = resp.interact_pointer_pos() {
        let t = ((p.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        *value = lo + t * (hi - lo);
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let kx = x0 + t * (x1 - x0);
    let p = ui.painter();
    let track = Rect::from_min_max(pos2(r.left(), track_y - 3.0), pos2(r.right(), track_y + 3.0));
    p.rect_filled(track, 3.0, pal.control);
    p.rect_filled(
        Rect::from_min_max(track.min, pos2(kx, track.bottom())),
        3.0,
        MENU_ACCENT,
    );
    if ticks > 1 {
        for i in 0..ticks {
            let x = x0 + (x1 - x0) * i as f32 / (ticks - 1) as f32;
            p.circle_filled(pos2(x, r.bottom() - 3.0), 1.2, pal.sep_grey);
        }
    }
    let kr = Rect::from_center_size(pos2(kx, track_y), knob);
    p.rect_filled(kr.translate(vec2(0.0, 1.0)).expand(0.5), 8.0, Color32::from_black_alpha(40));
    p.rect_filled(kr, 8.0, Color32::WHITE);
    p.rect_stroke(kr, 8.0, Stroke::new(0.5_f32, pal.border), egui::StrokeKind::Inside);
}

/// One export choice: coloured format tag, name, short description and an
/// Export button. The whole row is clickable.
fn export_row(
    ui: &mut egui::Ui,
    k: usize,
    tag: &str,
    color: Color32,
    name: &str,
    note: &str,
) -> bool {
    let pal = crate::settings::pal();
    let w = ui.available_width();
    let (pad, tile, btn_w) = (8.0, 34.0, 70.0);
    let text_x = pad + tile + 10.0;
    let text_w = w - text_x - btn_w - pad - 8.0;
    let bold = FontId::new(13.5, crate::theme::family(false, true, false));
    let title = ui
        .painter()
        .layout_no_wrap(name.to_string(), bold, pal.text_hi);
    let desc = ui.painter().layout(
        note.to_string(),
        FontId::proportional(12.0),
        pal.text_dim,
        text_w,
    );
    let h = (title.size().y + 3.0 + desc.size().y).max(tile) + 2.0 * pad + 2.0;
    let (r, row) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let btn = Rect::from_min_size(
        pos2(r.right() - btn_w - pad, r.center().y - 13.0),
        vec2(btn_w, 26.0),
    );
    let b = ui.interact(btn, ui.scope_id().with(("export", k)), Sense::click());
    let row_hover = ui.rect_contains_pointer(r);
    let p = ui.painter();
    if row_hover {
        p.rect_filled(r, 8.0, pal.hover.gamma_multiply(0.6));
    }
    let t = Rect::from_min_size(pos2(r.left() + pad, r.top() + pad), vec2(tile, tile));
    p.rect_filled(t, 8.0, color);
    p.text(
        t.center(),
        egui::Align2::CENTER_CENTER,
        tag,
        FontId::new(
            if tag.len() > 3 { 9.5 } else { 11.0 },
            crate::theme::family(false, true, false),
        ),
        Color32::WHITE,
    );
    let tx = r.left() + text_x;
    p.galley(pos2(tx, r.top() + pad), title.clone(), pal.text_hi);
    p.galley(
        pos2(tx, r.top() + pad + title.size().y + 3.0),
        desc,
        pal.text_dim,
    );
    let fill = if b.is_pointer_button_down_on() {
        pal.hover
    } else if b.hovered() {
        ACCENT_BLUE
    } else {
        pal.button
    };
    p.rect_filled(btn, 6.0, fill);
    if !crate::settings::is_dark() && !b.hovered() {
        p.rect_stroke(
            btn,
            6.0,
            Stroke::new(1.0_f32, pal.border),
            egui::StrokeKind::Inside,
        );
    }
    p.text(
        btn.center(),
        egui::Align2::CENTER_CENTER,
        "Export…",
        FontId::proportional(12.5),
        if b.hovered() {
            Color32::WHITE
        } else {
            pal.text_hi
        },
    );
    let row = row.on_hover_text_at_pointer(format!("Export as {name}"));
    row.clicked() || b.clicked()
}

/// A form row: right-aligned label, then the control filling the rest.
fn labeled(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let full = ui.available_width();
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(80.0, 24.0), Sense::hover());
        ui.painter().text(
            pos2(r.right(), r.center().y),
            egui::Align2::RIGHT_CENTER,
            label,
            FontId::proportional(13.0),
            crate::settings::pal().text_dim,
        );
        ui.add_space(4.0);
        // The label, spacer and control each consume horizontal item spacing.
        let w = (full - 84.0 - 3.0 * ui.spacing().item_spacing.x).max(0.0);
        ui.allocate_ui_with_layout(
            vec2(w, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_width(w);
                add(ui)
            },
        );
    });
}

/// A − value + control for small whole numbers.
fn stepper(ui: &mut egui::Ui, v: &mut usize, min: usize, max: usize) {
    ui.spacing_mut().item_spacing.x = 4.0;
    // right-to-left layout: + first, then the value, then −
    if ui
        .add_enabled(*v < max, egui::Button::new("+").min_size(vec2(24.0, 24.0)))
        .clicked()
    {
        *v += 1;
    }
    ui.add(
        egui::DragValue::new(v)
            .range(min..=max)
            .speed(0.1)
            .update_while_editing(false),
    )
    .on_hover_text(format!("{min}–{max}"));
    if ui
        .add_enabled(*v > min, egui::Button::new("−").min_size(vec2(24.0, 24.0)))
        .clicked()
    {
        *v -= 1;
    }
}

/// Menu-style dropdown matching the font picker: full-width button,
/// checkmark on the current value, left-aligned rows with a hover bar.
fn menu_combo<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut T,
    options: &[(T, &str)],
) {
    let current = options
        .iter()
        .find(|(v, _)| v == value)
        .map(|(_, l)| *l)
        .unwrap_or("");
    let width = ui.available_width();
    let salt = egui::Id::unique(id_salt);
    egui::ComboBox::from_id_salt(salt)
        .truncate()
        .width(width)
        .selected_text(current)
        .show_ui(ui, |ui| {
            let room = ui.ctx().content_rect().right() - ui.max_rect().left() - 20.0;
            ui.set_width((width - 12.0).min(room).max(120.0_f32.min(room)));
            ui.spacing_mut().item_spacing.y = 0.0;
            for (k, (v, label)) in options.iter().enumerate() {
                let (text, note) = split_font_label(label);
                let r = menu_row(ui, salt.with(k), text, note, v == value, None, 24.0);
                if r.clicked() {
                    *value = *v;
                    egui::Popup::close_all(ui.ctx());
                }
            }
        });
}

/// Split "Arial — Theme default" into the font name and its dim note.
fn split_font_label(text: &str) -> (&str, Option<&str>) {
    match text.split_once(" — ") {
        Some((name, note)) => (name, Some(note)),
        None => (text, None),
    }
}

/// One menu row: checkmark for the current value, left-aligned text,
/// optional dim note on the right, and a hover/keyboard highlight.
fn menu_row(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &str,
    note: Option<&str>,
    checked: bool,
    highlighted: Option<bool>,
    h: f32,
) -> egui::Response {
    let pal = crate::settings::pal();
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    // None: plain menu that follows the mouse; Some: keyboard-driven highlight.
    let lit = highlighted.unwrap_or_else(|| resp.hovered());
    if lit {
        p.rect_filled(r.shrink2(vec2(0.0, 1.0)), 5.0, MENU_ACCENT);
    }
    let fg = if lit { Color32::WHITE } else { pal.text_hi };
    if checked {
        let c = pos2(r.left() + 10.0, r.center().y);
        let st = Stroke::new(1.6_f32, fg);
        p.line_segment([c + vec2(-3.5, 0.0), c + vec2(-1.0, 3.0)], st);
        p.line_segment([c + vec2(-1.0, 3.0), c + vec2(4.0, -3.5)], st);
    }
    let font = FontId::proportional(13.0);
    let mut right = r.right() - 8.0;
    if let Some(note) = note {
        let g = p.layout_no_wrap(
            note.to_owned(),
            FontId::proportional(11.5),
            if lit { Color32::from_white_alpha(210) } else { pal.text_dim },
        );
        right -= g.size().x;
        p.galley(pos2(right, r.center().y - g.size().y / 2.0), g, fg);
        right -= 8.0;
    }
    let left = r.left() + 22.0;
    let g = p.layout(text.to_owned(), font, fg, f32::INFINITY);
    let clip = Rect::from_min_max(pos2(left, r.top()), pos2(right.max(left), r.bottom()));
    p.with_clip_rect(clip)
        .galley(pos2(left, r.center().y - g.size().y / 2.0), g, fg);
    resp
}

/// Accent used for highlighted menu rows (macOS-style selection blue).
const MENU_ACCENT: Color32 = Color32::from_rgb(0x1F, 0x6F, 0xEB);

/// Font dropdown: built-in choices plus every installed family, with a
/// search box at the top that filters as you type. Arrow keys move the
/// highlight; Enter picks it.
fn font_combo(
    ui: &mut egui::Ui,
    id: &str,
    v: &mut FontChoice,
    label: &dyn Fn(FontChoice) -> String,
) {
    let pal = crate::settings::pal();
    let filter_id = ui.scope_id().with(id).with("font_filter");
    let hl_id = filter_id.with("hl");
    let mut filter: Option<String> = ui.data(|d| d.get_temp(filter_id));
    let closed_text = {
        let full = label(*v);
        let (name, note) = split_font_label(&full);
        match note {
            Some(_) => format!("{name} (Default)"),
            None => name.to_owned(),
        }
    };
    let shown = egui::ComboBox::from_id_salt(id)
        .truncate()
        .width(ui.available_width())
        .height(520.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .selected_text(closed_text)
        .show_ui(ui, |ui| {
            // Wide enough for long names, but never past the window's right edge
            // (the popup opens at the button's left edge).
            let room = ui.ctx().content_rect().right() - ui.max_rect().left() - 20.0;
            ui.set_width(260.0_f32.min(room).max(ui.available_width().min(room)));
            ui.spacing_mut().item_spacing.y = 0.0;
            let first_open = filter.is_none();
            let text = filter.get_or_insert_with(String::new);

            // Search field with a magnifier icon.
            let edit = egui::Frame::new()
                .fill(pal.field)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        let c = r.center() - vec2(1.0, 1.0);
                        let st = Stroke::new(1.4_f32, pal.text_dim);
                        ui.painter().circle_stroke(c, 4.5, st);
                        ui.painter()
                            .line_segment([c + vec2(3.2, 3.2), c + vec2(6.0, 6.0)], st);
                        ui.add(
                            egui::TextEdit::singleline(text)
                                .hint_text("Search fonts")
                                .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 2)))
                                .desired_width(ui.available_width()),
                        )
                    })
                    .inner
                })
                .inner;
            if first_open {
                edit.request_focus();
            }
            let needle = text.trim().to_lowercase();
            let matches = |name: &str| needle.is_empty() || name.to_lowercase().contains(&needle);

            // Every visible choice, in order, so the keyboard can walk them.
            let theme_label = label(FontChoice::Theme);
            let show_theme = true;
            let fonts: Vec<&crate::fonts::Family> = crate::fonts::catalog()
                .map(|all| all.iter().filter(|f| matches(&f.name)).collect())
                .unwrap_or_default();
            let total = show_theme as usize + fonts.len();
            let choice = |i: usize| -> FontChoice {
                if show_theme && i == 0 {
                    FontChoice::Theme
                } else {
                    FontChoice::Named(crate::fonts::face_for(fonts[i - show_theme as usize]))
                }
            };

            let mut hl: usize = if edit.changed() {
                0
            } else {
                ui.data(|d| d.get_temp(hl_id)).unwrap_or(0)
            };
            let (down, up, enter) = ui.input(|i| {
                (
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::Enter),
                )
            });
            let moved = ui.input(|i| i.pointer.delta() != vec2(0.0, 0.0));
            let mut keyed = false;
            if total > 0 {
                if down {
                    hl = (hl + 1).min(total - 1);
                    keyed = true;
                }
                if up {
                    hl = hl.saturating_sub(1);
                    keyed = true;
                }
                hl = hl.min(total - 1);
            }
            if keyed {
                edit.request_focus();
            }
            let mut picked: Option<FontChoice> = None;
            if enter && total > 0 {
                picked = Some(choice(hl));
            }

            let row_h = 24.0;
            let heading = |ui: &mut egui::Ui, t: &str| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(t).size(11.0).color(pal.text_dim));
                });
                ui.add_space(2.0);
            };
            let current = match v {
                FontChoice::Named(f) => Some(f.name()),
                _ => None,
            };
            if show_theme {
                ui.add_space(4.0);
                let (name, note) = split_font_label(&theme_label);
                let r = menu_row(
                    ui,
                    filter_id.with("theme"),
                    name,
                    note,
                    *v == FontChoice::Theme,
                    Some(hl == 0),
                    row_h,
                );
                if r.hovered() && moved {
                    hl = 0;
                }
                if r.clicked() {
                    picked = Some(FontChoice::Theme);
                }
                ui.add_space(4.0);
                ui.separator();
            }
            // Fixed-height list: egui popups keep their previous size, so a list
            // that shrinks (loading, narrow search) would otherwise stay small.
            heading(ui, "Installed fonts");
            let list_h = 300.0;
            let w = ui.available_width();
            ui.allocate_ui_with_layout(
                vec2(w, list_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_size(vec2(w, list_h));
                    if crate::fonts::catalog().is_none() {
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| ui.weak("Loading installed fonts…"));
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(200));
                        return;
                    }
                    if fonts.is_empty() {
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| ui.weak("No matching fonts"));
                        return;
                    }
                    let offset = show_theme as usize;
                    let scroll_id = ui.make_persistent_id("fonts_scroll");
                    let mut area = egui::ScrollArea::vertical()
                        .id_salt("fonts_scroll")
                        .max_height(list_h)
                        .auto_shrink([false, false]);
                    if keyed && hl >= offset {
                        let row = (hl - offset) as f32 * row_h;
                        let top = egui::scroll_area::State::load(ui.ctx(), scroll_id)
                            .map(|s| s.unclamped_offset().y)
                            .unwrap_or(0.0);
                        if row < top {
                            area = area.vertical_scroll_offset(row);
                        } else if row + row_h > top + list_h {
                            area = area.vertical_scroll_offset(row + row_h - list_h);
                        }
                    }
                    area.show_rows(ui, row_h, fonts.len(), |ui, rows| {
                        for i in rows {
                            let f = fonts[i];
                            let on = current.as_deref() == Some(f.name.as_str());
                            let r = menu_row(
                                ui,
                                filter_id.with(("font", i)),
                                &f.name,
                                None,
                                on,
                                Some(hl == i + offset),
                                row_h,
                            );
                            if r.hovered() && moved {
                                hl = i + offset;
                            }
                            if r.clicked() {
                                picked = Some(FontChoice::Named(crate::fonts::face_for(f)));
                            }
                        }
                    });
                },
            );
            ui.data_mut(|d| d.insert_temp(hl_id, hl));
            if let Some(c) = picked {
                *v = c;
                egui::Popup::close_all(ui.ctx());
            }
        });
    // Start with an empty search each time the list opens.
    if shown.inner.is_none() || !egui::Popup::is_any_open(ui.ctx()) {
        filter = None;
        ui.data_mut(|d| d.remove::<usize>(hl_id));
    }
    ui.data_mut(|d| match filter {
        Some(f) => {
            d.insert_temp(filter_id, f);
        }
        None => {
            d.remove::<String>(filter_id);
        }
    });
}

fn pick(pal: &mut Palette, k: usize) -> &mut Color32 {
    match k {
        0 => &mut pal.title,
        1 => &mut pal.body,
        _ => &mut pal.bg,
    }
}

/// Smooth, rounded gradient chip shared by the inspector and preset picker.
fn gradient_chip(
    ui: &mut egui::Ui,
    colors: &[Color32],
    size: egui::Vec2,
    selected: bool,
) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    // Vertical strips follow the rounded outline without square end caps.
    let radius = 4.0_f32.min(r.height() / 2.0);
    let steps = (r.width() * 2.0).ceil().max(2.0) as usize;
    for i in 0..steps {
        let x = r.left() + r.width() * i as f32 / steps as f32;
        let next = r.left() + r.width() * (i + 1) as f32 / steps as f32;
        let edge = (x - r.left()).min(r.right() - next).max(0.0);
        let inset = if edge < radius {
            radius - (radius * radius - (radius - edge).powi(2)).sqrt()
        } else {
            0.0
        };
        let color = crate::theme::deck_color(colors, i, steps).unwrap_or(Color32::WHITE);
        painter.rect_filled(
            Rect::from_min_max(pos2(x, r.top() + inset), pos2(next, r.bottom() - inset)),
            0.0,
            color,
        );
    }
    painter.rect_stroke(
        r,
        radius,
        Stroke::new(
            if selected { 2.0_f32 } else { 1.0_f32 },
            if selected || response.hovered() {
                FORMAT_BLUE
            } else {
                crate::settings::pal().border
            },
        ),
        egui::StrokeKind::Inside,
    );
    response
}

/// Inner width of the background picker popup.
const PICKER_W: f32 = 248.0;

fn picker_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(11.0)
            .color(crate::settings::pal().text_dim),
    );
    ui.add_space(2.0);
}

/// Two-option segmented control (Solid / Gradient).
fn segmented(ui: &mut egui::Ui, value: &mut bool, labels: [&str; 2]) {
    let pal = crate::settings::pal();
    let dark = std::ptr::eq(pal, &crate::settings::DARK_PAL);
    let (r, _) = ui.allocate_exact_size(vec2(PICKER_W, 26.0), Sense::hover());
    ui.painter().rect_filled(r, 7.0, pal.control);
    let half = r.width() / 2.0;
    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(pos2(r.left() + half * i as f32, r.top()), vec2(half, r.height()));
        let resp = ui.interact(seg, ui.scope_id().with(("seg", i)), Sense::click());
        let active = *value == (i == 1);
        if active {
            let inner = seg.shrink(2.0);
            ui.painter()
                .rect_filled(inner, 5.0, if dark { pal.tab_grey } else { pal.field });
            ui.painter().rect_stroke(
                inner,
                5.0,
                Stroke::new(1.0_f32, pal.border),
                egui::StrokeKind::Inside,
            );
        } else if resp.hovered() {
            ui.painter().rect_filled(seg.shrink(2.0), 5.0, pal.hover);
        }
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            FontId::proportional(12.5),
            if active { pal.text_hi } else { pal.icon },
        );
        if resp.clicked() {
            *value = i == 1;
        }
    }
}

fn background_swatch(
    ui: &mut egui::Ui,
    solid: &mut Color32,
    enabled: &mut bool,
    colors: &mut Vec<Color32>,
    width: f32,
) {
    let id = ui.auto_id_with("background_picker");
    let preview = if *enabled {
        colors.clone()
    } else {
        vec![*solid]
    };
    let response = gradient_chip(ui, &preview, vec2(width, 24.0), false)
        .on_hover_text("Choose a solid background or deck gradient");
    if response.clicked() {
        egui::Popup::toggle_id(ui.ctx(), id);
    }
    if !egui::Popup::is_id_open(ui.ctx(), id) {
        return;
    }
    let popup_w = PICKER_W + 24.0;
    let area = egui::Area::new(id)
        .kind(egui::UiKind::Picker)
        .order(egui::Order::Foreground)
        .fixed_pos({
            let screen = ui.ctx().content_rect().shrink(12.0);
            pos2(
                (response.rect.right() - popup_w)
                    .min(screen.right() - popup_w)
                    .max(screen.left()),
                (response.rect.bottom() + 6.0)
                    .min(screen.bottom() - 470.0)
                    .max(screen.top()),
            )
        })
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(12.0)
                .corner_radius(10.0)
                .show(ui, |ui| {
                    ui.set_width(PICKER_W);
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Background").strong().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (r, close) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                            let close = close.on_hover_text("Close");
                            let pal = crate::settings::pal();
                            if close.hovered() {
                                ui.painter().circle_filled(r.center(), 9.0, pal.hover);
                            }
                            let k = 3.5;
                            let st = Stroke::new(1.5_f32, pal.icon);
                            let c = r.center();
                            ui.painter().line_segment([c + vec2(-k, -k), c + vec2(k, k)], st);
                            ui.painter().line_segment([c + vec2(-k, k), c + vec2(k, -k)], st);
                            if close.clicked() {
                                egui::Popup::close_all(ui.ctx());
                            }
                        });
                    });
                    ui.add_space(2.0);
                    segmented(ui, enabled, ["Solid", "Gradient"]);
                    ui.add_space(6.0);
                    if !*enabled {
                        color_editor(ui, id.with("solid"), solid);
                        return;
                    }
                    gradient_editor(ui, id, colors);
                });
        })
        .response;
    if !response.clicked() && (ui.input(|i| i.key_pressed(Key::Escape)) || area.clicked_elsewhere())
    {
        egui::Popup::close_all(ui.ctx());
    }
}

/// Presets, stop bar with handles, and an editor for the selected stop.
fn gradient_editor(ui: &mut egui::Ui, id: egui::Id, colors: &mut Vec<Color32>) {
    let pal = crate::settings::pal();
    let presets: [(&str, &[u32]); 6] = [
        ("Daylight", &[0xFFE052, 0x0073D9]),
        ("Pastel", &[0xD4EDF4, 0xF5DFE7, 0xF2F1D9]),
        ("Sunset", &[0xFFC987, 0xF17B89, 0x7055B6]),
        ("Ocean", &[0xB8F3DF, 0x40ADCF, 0x244B9B]),
        ("Aurora", &[0x555ACB, 0xCF89CF, 0x8DDFC3]),
        ("Spectrum", &[0x53BCE5, 0xB66CAC, 0xED7754, 0xEDCF5C, 0x69BB8B]),
    ];
    picker_heading(ui, "PRESETS");
    let gap = 6.0;
    let chip_w = (PICKER_W - gap * 2.0) / 3.0;
    for row in presets.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (name, values) in row {
                let stops: Vec<_> = values.iter().map(|v| crate::theme::hex(*v)).collect();
                if gradient_chip(ui, &stops, vec2(chip_w, 22.0), *colors == stops)
                    .on_hover_text(*name)
                    .clicked()
                {
                    *colors = stops;
                }
            }
        });
    }
    ui.add_space(8.0);

    // Stop bar: the gradient plus one handle per colour, spaced as the deck uses them.
    let sel_id = id.with("stop");
    let mut selected = ui
        .data(|d| d.get_temp::<usize>(sel_id))
        .unwrap_or(0)
        .min(colors.len() - 1);
    ui.horizontal(|ui| {
        picker_heading(ui, "COLORS");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let small = |t: &str| egui::Button::new(RichText::new(t).size(11.5)).small();
            if ui
                .add(small("⇄"))
                .on_hover_text("Reverse order")
                .clicked()
            {
                colors.reverse();
                selected = colors.len() - 1 - selected;
            }
            if ui
                .add_enabled(colors.len() > 2, small("−"))
                .on_hover_text("Remove selected color")
                .clicked()
            {
                colors.remove(selected);
                selected = selected.min(colors.len() - 1);
            }
            if ui
                .add_enabled(colors.len() < 6, small("+"))
                .on_hover_text("Add a color after the selected one")
                .clicked()
            {
                let next = colors.get(selected + 1).copied().unwrap_or(colors[selected]);
                let a = colors[selected].to_array();
                let b = next.to_array();
                let mid = |i: usize| ((a[i] as u16 + b[i] as u16) / 2) as u8;
                colors.insert(selected + 1, Color32::from_rgb(mid(0), mid(1), mid(2)));
                selected += 1;
            }
        });
    });
    let handle_r = 8.0;
    let (r, _) = ui.allocate_exact_size(vec2(PICKER_W, 26.0 + 4.0 + handle_r * 2.0 + 16.0), Sense::hover());
    let bar = Rect::from_min_size(
        pos2(r.left() + handle_r, r.top()),
        vec2(r.width() - handle_r * 2.0, 26.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
    gradient_chip(&mut child, colors, bar.size(), false);
    let n = colors.len();
    let x_of = |i: usize| bar.left() + bar.width() * i as f32 / (n - 1) as f32;
    let cy = bar.bottom() + 4.0 + handle_r;
    let mut drag_to = None;
    for i in 0..n {
        let c = pos2(x_of(i), cy);
        let hit = Rect::from_center_size(c, vec2(handle_r * 2.0 + 4.0, handle_r * 2.0 + 4.0));
        let resp = ui.interact(hit, id.with(("handle", i)), Sense::click_and_drag());
        if resp.clicked() || resp.drag_started() {
            selected = i;
        }
        if resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                let t = ((p.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                let to = (t * (n - 1) as f32).round() as usize;
                if to != i {
                    drag_to = Some((i, to));
                }
            }
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        let p = ui.painter();
        // Notch pointing up at the bar.
        p.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x, bar.bottom() + 1.0),
                pos2(c.x - 4.0, c.y - handle_r + 1.0),
                pos2(c.x + 4.0, c.y - handle_r + 1.0),
            ],
            if i == selected { FORMAT_BLUE } else { pal.sep_grey },
            Stroke::NONE,
        ));
        p.circle_filled(c, handle_r + 1.0, Color32::from_black_alpha(110));
        p.circle_filled(c, handle_r, if i == selected { FORMAT_BLUE } else { Color32::WHITE });
        p.circle_filled(c, handle_r - 2.5, colors[i]);
    }
    if let Some((from, to)) = drag_to {
        let c = colors.remove(from);
        colors.insert(to, c);
        selected = to;
    }
    // Endpoints label.
    let p = ui.painter();
    let font = FontId::proportional(10.0);
    p.text(
        pos2(bar.left() - handle_r, r.bottom()),
        egui::Align2::LEFT_BOTTOM,
        "First slide",
        font.clone(),
        pal.text_dim,
    );
    p.text(
        pos2(bar.right() + handle_r, r.bottom()),
        egui::Align2::RIGHT_BOTTOM,
        "Last slide",
        font,
        pal.text_dim,
    );
    ui.data_mut(|d| d.insert_temp(sel_id, selected));
    ui.add_space(4.0);
    ui.separator();
    color_editor(ui, id.with(("stop_color", selected)), &mut colors[selected]);
}

/// Swatch + hex row above the saturation field and hue strip.
fn color_editor(ui: &mut egui::Ui, id: egui::Id, color: &mut Color32) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
        ui.painter().rect_filled(r, 5.0, *color);
        ui.painter().rect_stroke(
            r,
            5.0,
            Stroke::new(1.0_f32, crate::settings::pal().border),
            egui::StrokeKind::Inside,
        );
        hex_field(ui, id.with("hex"), color);
    });
    ui.add_space(2.0);
    color_field(ui, id.with("field"), color);
}

/// Direct saturation/value field and hue strip, without the debug-style RGB controls.
fn color_field(ui: &mut egui::Ui, id: egui::Id, color: &mut Color32) {
    let mut hsv = egui::ecolor::Hsva::from(*color);
    if hsv.s == 0.0 || hsv.v == 0.0 {
        hsv.h = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(hsv.h);
    }
    let width = ui.available_width().min(PICKER_W);
    let (r, response) = ui.allocate_exact_size(vec2(width, 120.0), Sense::click_and_drag());
    let mut touched = response.is_pointer_button_down_on();
    let mut mesh = egui::Mesh::default();
    let n = 32;
    for y in 0..=n {
        for x in 0..=n {
            let sx = x as f32 / n as f32;
            let vy = y as f32 / n as f32;
            mesh.colored_vertex(
                pos2(r.left() + sx * r.width(), r.top() + vy * r.height()),
                Color32::from(egui::ecolor::Hsva::new(hsv.h, sx, 1.0 - vy, 1.0)),
            );
        }
    }
    for y in 0..n {
        for x in 0..n {
            let i = (y * (n + 1) + x) as u32;
            mesh.add_triangle(i, i + 1, i + (n + 1) as u32);
            mesh.add_triangle(i + 1, i + (n + 2) as u32, i + (n + 1) as u32);
        }
    }
    ui.painter().add(mesh);
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0_f32, crate::settings::pal().border),
        egui::StrokeKind::Inside,
    );
    if let Some(pos) = response.interact_pointer_pos() {
        hsv.s = ((pos.x - r.left()) / r.width()).clamp(0.0, 1.0);
        hsv.v = (1.0 - (pos.y - r.top()) / r.height()).clamp(0.0, 1.0);
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let marker = pos2(r.left() + hsv.s * r.width(), r.bottom() - hsv.v * r.height());
    let p = ui.painter();
    p.circle_stroke(marker, 7.0, Stroke::new(1.0_f32, Color32::from_black_alpha(90)));
    p.circle_stroke(marker, 6.0, Stroke::new(2.0_f32, Color32::WHITE));

    ui.add_space(4.0);
    let (r, response) = ui.allocate_exact_size(vec2(width, 12.0), Sense::click_and_drag());
    touched |= response.is_pointer_button_down_on();
    let knob = r.height() / 2.0;
    let track = r.shrink2(vec2(knob, 0.0));
    let mut mesh = egui::Mesh::default();
    let steps = 36;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let c = Color32::from(egui::ecolor::Hsva::new(t, 1.0, 1.0, 1.0));
        let x = track.left() + t * track.width();
        mesh.colored_vertex(pos2(x, r.top()), c);
        mesh.colored_vertex(pos2(x, r.bottom()), c);
    }
    for i in 0..steps as u32 {
        let k = i * 2;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    let p = ui.painter();
    p.circle_filled(pos2(track.left(), r.center().y), knob, Color32::from(egui::ecolor::Hsva::new(0.0, 1.0, 1.0, 1.0)));
    p.circle_filled(pos2(track.right(), r.center().y), knob, Color32::from(egui::ecolor::Hsva::new(1.0, 1.0, 1.0, 1.0)));
    p.add(mesh);
    if let Some(pos) = response.interact_pointer_pos() {
        hsv.h = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
    }
    let kc = pos2(track.left() + hsv.h * track.width(), r.center().y);
    p.circle_filled(kc, knob + 2.0, Color32::from_black_alpha(60));
    p.circle_filled(kc, knob + 1.5, Color32::WHITE);
    p.circle_filled(kc, knob - 1.0, Color32::from(egui::ecolor::Hsva::new(hsv.h, 1.0, 1.0, 1.0)));
    ui.data_mut(|d| d.insert_temp(id, hsv.h));
    let next = Color32::from(hsv);
    if touched && next != *color {
        *color = next;
    }
}

fn swatch(ui: &mut egui::Ui, c: &mut Color32, w: f32) {
    let popup_id = ui.auto_id_with("swatch_popup");
    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), egui::Sense::click());
    let resp = resp.on_hover_text("Click to edit color");
    ui.painter().rect_filled(r, 3.0, *c);
    ui.painter().rect_stroke(
        r,
        3.0,
        Stroke::new(1.0_f32, crate::settings::pal().border),
        egui::StrokeKind::Inside,
    );
    if resp.clicked() {
        egui::Popup::toggle_id(ui.ctx(), popup_id);
    }
    if !egui::Popup::is_id_open(ui.ctx(), popup_id) {
        return;
    }
    let area = egui::Area::new(popup_id)
        .kind(egui::UiKind::Picker)
        .order(egui::Order::Foreground)
        .fixed_pos(r.left_bottom() + vec2(0.0, 4.0))
        .show(ui.ctx(), |ui| {
            ui.spacing_mut().slider_width = 275.0;
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                hex_field(ui, popup_id.with("hex"), c);
                ui.add_space(4.0);
                egui::color_picker::color_picker_color32(ui, c, egui::color_picker::Alpha::Opaque);
            });
        })
        .response;
    if !resp.clicked()
        && (ui.input(|i| i.key_pressed(egui::Key::Escape)) || area.clicked_elsewhere())
    {
        egui::Popup::close_all(ui.ctx());
    }
}

/// "#RRGGBB" text box; applies the colour as soon as the text is a valid hex code.
fn hex_field(ui: &mut egui::Ui, id: egui::Id, c: &mut Color32) {
    let current = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data(|d| d.get_temp::<String>(id)).unwrap_or(current)
    } else {
        current
    };
    ui.horizontal(|ui| {
        ui.label("Hex");
        let edit = ui.add(
            egui::TextEdit::singleline(&mut text)
                .id(id)
                .font(egui::TextStyle::Monospace)
                .char_limit(7)
                .desired_width(90.0),
        );
        if edit.changed() {
            if let Some(rgb) = parse_hex(&text) {
                *c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            }
        }
        if !parse_hex(&text).is_some() && edit.has_focus() {
            ui.weak("e.g. #1A2B3C");
        }
    });
    ui.data_mut(|d| d.insert_temp(id, text));
}

/// Parses "#RGB", "#RRGGBB" (the "#" is optional).
fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if !h.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        6 => Some([digit(0, 2)?, digit(2, 2)?, digit(4, 2)?]),
        3 => Some([digit(0, 1)? * 17, digit(1, 1)? * 17, digit(2, 1)? * 17]),
        _ => None,
    }
}

fn compact_table(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.trim_start().starts_with('|') {
                format!(
                    "|{}|",
                    line.trim()
                        .trim_matches('|')
                        .split('|')
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join("|")
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod hex_tests {
    use super::parse_hex;

    #[test]
    fn parses_hex_codes() {
        assert_eq!(parse_hex("#111C3D"), Some([0x11, 0x1C, 0x3D]));
        assert_eq!(parse_hex("ffffff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#abc"), Some([0xAA, 0xBB, 0xCC]));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("#zzzzzz"), None);
    }
}
