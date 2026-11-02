//! The right-hand panel: Format (Markdown cheat sheet that inserts what you
//! click, plus table and chart maker), Pictures, and Design (theme, fonts,
//! colours, header & footer).

use crate::parse::Block;
use crate::render::ASPECT;
use crate::theme::Palette;
use crate::{
    icon_button, kbd, paint_icon, App, FontChoice, Icon, ACCENT_BLUE, DESIGN_PINK, FORMAT_BLUE,
    INSPECTOR_W, IS_MAC, SIZES,
};
use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Color32, FontId, Key, Modifiers, Rect, RichText, Sense, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Format,
    Pictures,
    Design,
    Share,
}

/// Share button colour (estimated from the person's screenshot).
const SHARE_ORANGE: Color32 = Color32::from_rgb(0xE5, 0x9A, 0x16);

const ON_SLIDE: Color32 = Color32::from_rgb(0x8E, 0xC6, 0xF5);
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

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(title)
            .font(FontId::new(13.5, crate::theme::family(false, true, false)))
            .color(crate::settings::pal().text_hi),
    );
    ui.add_space(4.0);
}

fn divider(ui: &mut egui::Ui) {
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
    let g = ui.fonts(|f| f.layout_job(it.label.clone()));
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
            let theme = self.theme_for(i, dark);
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
            ui.memory_mut(|m| m.toggle_popup(popup_id));
        }
        let open = ui.memory(|m| m.is_popup_open(popup_id));
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

        egui::popup_below_widget(
            ui,
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
                            if selected && !r.hovered() && ui.memory(|m| m.is_popup_open(popup_id))
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
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    });
            },
        );
        if !ui.memory(|m| m.is_popup_open(popup_id)) {
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

    pub fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .resizable(false)
            .exact_width(INSPECTOR_W)
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
            .show(ctx, |ui| {
                let full = ui.max_rect();
                ui.painter().line_segment(
                    [
                        pos2(full.left() - 16.0, full.top()),
                        pos2(full.left() - 16.0, full.bottom()),
                    ],
                    Stroke::new(1.0_f32, crate::settings::pal().divider),
                );
                self.inspector_top(ui, ctx);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Respect the scroll viewport so controls stay inside the margins.
                        ui.set_width(ui.available_width());
                        match self.tab {
                            Tab::Format => self.format_tab(ui),
                            Tab::Pictures => self.pictures_tab(ui),
                            Tab::Design => self.design_tab(ui),
                            Tab::Share => self.share_tab(ui),
                        }
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
            [Tab::Format, Tab::Pictures, Tab::Design]
                .iter()
                .map(|t| if *t == self.tab { 80.0 } else { 42.0 })
                .collect()
        };
        let mut x = seg.left() + 2.0;
        for (k, t) in [Tab::Format, Tab::Pictures, Tab::Design]
            .into_iter()
            .enumerate()
        {
            let r = Rect::from_min_size(pos2(x, seg.top() + 2.0), vec2(widths[k], 28.0));
            x += widths[k];
            let resp = ui
                .interact(r, ui.id().with(("seg", k)), Sense::click())
                .on_hover_text(match t {
                    Tab::Format => "Format: Markdown cheat sheet, tables and charts",
                    Tab::Pictures => "Pictures in this presentation",
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
                    Tab::Pictures => "Pictures",
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
                    Tab::Pictures => {
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
            .interact(share_r, ui.id().with("share"), Sense::click())
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
            .interact(play, ui.id().with("play"), Sense::click())
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
                label: dimmed("→ ", "Body Text", ON_SLIDE, false),
                shortcut: "Tab",
                action: Action::Prefix("\t"),
            },
            Item {
                icon: "H1",
                label: dimmed("", "# Title", ON_SLIDE, true),
                shortcut: "⌘1",
                action: Action::Prefix("# "),
            },
            Item {
                icon: "H2",
                label: dimmed("", "## Subtitle", ON_SLIDE, true),
                shortcut: "⌘2",
                action: Action::Prefix("## "),
            },
            Item {
                icon: "H3",
                label: dimmed("", "### Heading", ON_SLIDE, true),
                shortcut: "⌘3",
                action: Action::Prefix("### "),
            },
            Item {
                icon: "H4",
                label: dimmed("", "#### Subheading", ON_SLIDE, true),
                shortcut: "⌘4",
                action: Action::Prefix("#### "),
            },
            Item {
                icon: "▣",
                label: dimmed("", "/path/image.jpg", ON_SLIDE, false),
                shortcut: "",
                action: Action::AddPicture,
            },
            Item {
                icon: "▶",
                label: dimmed("", "/path/video.mp4", ON_SLIDE, false),
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
        section(ui, "Speech");
        for it in &speech {
            if item_row(ui, it) {
                todo = Some(&it.action);
            }
        }
        divider(ui);
        section(ui, "Text on Slide");
        for it in &on_slide {
            if item_row(ui, it) {
                todo = Some(&it.action);
            }
        }
        divider(ui);
        section(ui, "Format");
        for it in &format {
            if item_row(ui, it) {
                todo = Some(&it.action);
            }
        }
        if let Some(a) = todo {
            self.run(a);
            todo = None;
        }
        divider(ui);
        section(ui, "Table and Charts");
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
            let resp = ui.interact(r, ui.id().with(("disp", k)), Sense::click());
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
        divider(ui);
        section(ui, "Code and Math");
        for it in &code {
            if item_row(ui, it) {
                todo = Some(&it.action);
            }
        }
        if let Some(a) = todo {
            self.run(a);
        }
    }

    fn pictures_tab(&mut self, ui: &mut egui::Ui) {
        section(ui, "Pictures in this Presentation");
        let mut pics = Vec::new();
        for (i, s) in self.slides.iter().enumerate() {
            for b in &s.backgrounds {
                pics.push((i, b.clone()));
            }
            for g in &s.groups {
                for bl in g {
                    if let Block::Image(img) = bl {
                        pics.push((i, img.clone()));
                    }
                }
            }
        }
        if pics.is_empty() {
            ui.label(RichText::new("No pictures yet. Put a picture's path or web address on its own line, or drop a file onto the editor.").color(crate::settings::pal().text_dim));
        }
        let w = (ui.available_width() - 10.0) / 2.0;
        let accents = self.theme().accents;
        let mut jump = None;
        for chunk in pics.chunks(2) {
            ui.horizontal(|ui| {
                for (i, img) in chunk {
                    let (r, resp) =
                        ui.allocate_exact_size(vec2(w, w / ASPECT + 22.0), Sense::click());
                    let pr = Rect::from_min_size(r.min, vec2(w, w / ASPECT));
                    let painter = ui.painter_at(r);
                    let mut spec = img.clone();
                    spec.caption = None;
                    spec.opacity = 1.0;
                    spec.background = false;
                    crate::images::draw_image(
                        &painter,
                        pr,
                        &spec,
                        &mut self.images,
                        &accents,
                        Some(false),
                    );
                    let name = img.src.rsplit('/').next().unwrap_or(&img.src).to_string();
                    painter.text(
                        pos2(r.left(), pr.bottom() + 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}  ·  slide {}", name, i + 1),
                        FontId::proportional(11.0),
                        crate::settings::pal().text_dim,
                    );
                    if resp.hovered() {
                        painter.rect_stroke(
                            pr,
                            2.0,
                            Stroke::new(2.0_f32, FORMAT_BLUE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if resp.clicked() {
                        jump = Some(*i);
                    }
                }
            });
            ui.add_space(6.0);
        }
        if let Some(i) = jump {
            self.select_slide(i);
        }
        ui.add_space(8.0);
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Add Picture…"),
            )
            .clicked()
        {
            self.add_media(false);
        }
        divider(ui);
        section(ui, "Picture Settings");
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
                ui.label(RichText::new(k).color(ON_SLIDE).monospace());
                ui.label(RichText::new(v).color(crate::settings::pal().text_dim));
            });
        }
        ui.label(
            RichText::new("Write these on the lines right under a picture.")
                .color(crate::settings::pal().text_dim)
                .size(12.0),
        );
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

        section(ui, "Export");
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
        divider(ui);

        section(ui, "Document");
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
            .interact(card, ui.id().with("design-preview"), Sense::click())
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

        section(ui, "Theme");
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
        divider(ui);

        section(ui, "Fonts");
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
            egui::ComboBox::from_id_salt("size")
                .width(ui.available_width())
                .selected_text(SIZES[self.size_idx].0)
                .show_ui(ui, |ui| {
                    for (i, (n, _)) in SIZES.iter().enumerate() {
                        ui.selectable_value(&mut self.size_idx, i, *n);
                    }
                });
        });
        divider(ui);

        section(ui, "Colors");
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
                swatch(ui, pick(&mut light, k), w);
                swatch(ui, pick(&mut dark, k), w);
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
        divider(ui);

        section(ui, "Header & Footer");
        self.header_footer(ui);
        divider(ui);

        section(ui, "Metadata");
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
                .interact(cell, ui.id().with(("hf-slot", i)), Sense::click())
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
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(SLOT_NAMES[i]).color(pal.text_dim).size(13.0));
            let slot = &mut self.slots[i];
            egui::ComboBox::from_id_salt(("hf-kind", i))
                .width(86.0)
                .selected_text(slot.kind.label())
                .show_ui(ui, |ui| {
                    for k in SlotKind::ALL {
                        ui.selectable_value(&mut slot.kind, k, k.label());
                    }
                });
            if slot.kind == SlotKind::Custom {
                ui.add(
                    egui::TextEdit::singleline(&mut slot.text)
                        .id(ui.id().with(("hf-text", i)))
                        .desired_width(ui.available_width())
                        .hint_text("Text"),
                );
            }
        });
        self.logo_drop_rect = None;
        if self.slots[i].kind == SlotKind::Logo {
            ui.add_space(10.0);
            ui.separator();
            section(ui, "Logo");
            ui.horizontal(|ui| {
                let (preview, _) = ui.allocate_exact_size(vec2(90.0, 66.0), Sense::hover());
                self.logo_drop_rect = Some(preview.intersect(ui.clip_rect()));
                let slot = &mut self.slots[i];
                let image = if slot.logo.is_empty() {
                    None
                } else {
                    self.images.get(ui.ctx(), &slot.logo, None)
                };
                if let Some((texture, size)) = image {
                    let scale =
                        (preview.width() / size[0] as f32).min(preview.height() / size[1] as f32);
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
                    ui.painter().rect_stroke(
                        preview.shrink(2.0),
                        6.0,
                        Stroke::new(1.0_f32, pal.text_dim),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        preview.center(),
                        egui::Align2::CENTER_CENTER,
                        if slot.logo.is_empty() {
                            "Drop Logo"
                        } else {
                            "Unavailable"
                        },
                        FontId::proportional(12.0),
                        pal.text_dim,
                    );
                }
                ui.vertical(|ui| {
                    if ui.button("Select Logo…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                            .pick_file()
                        {
                            slot.logo = path.to_string_lossy().into_owned();
                        }
                    }
                    if ui
                        .add_enabled(!slot.logo.is_empty(), egui::Button::new("Remove Logo"))
                        .clicked()
                    {
                        slot.logo.clear();
                    }
                });
            });
            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut self.slots[i].logo_size, 0.02..=0.08)
                    .text("Size")
                    .show_value(false),
            );
        }
    }
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
    let b = ui.interact(btn, ui.id().with(("export", k)), Sense::click());
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

/// Font dropdown: built-in choices plus every installed family, with a
/// search box at the top that filters as you type (Enter picks the first match).
fn font_combo(
    ui: &mut egui::Ui,
    id: &str,
    v: &mut FontChoice,
    label: &dyn Fn(FontChoice) -> String,
) {
    let filter_id = ui.id().with(id).with("font_filter");
    let mut filter: Option<String> = ui.data(|d| d.get_temp(filter_id));
    let shown = egui::ComboBox::from_id_salt(id)
        .truncate()
        .width(ui.available_width())
        .height(360.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .selected_text(label(*v))
        .show_ui(ui, |ui| {
            ui.set_min_width(240.0);
            let first_open = filter.is_none();
            let text = filter.get_or_insert_with(String::new);
            let edit = ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text("Search fonts")
                    .desired_width(f32::INFINITY),
            );
            if first_open {
                edit.request_focus();
            }
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let needle = text.trim().to_lowercase();
            let matches = |name: &str| needle.is_empty() || name.to_lowercase().contains(&needle);
            let mut picked: Option<FontChoice> = None;

            let builtin = [FontChoice::Theme];
            for c in builtin.into_iter().filter(|c| matches(&label(*c))) {
                if enter && picked.is_none() {
                    picked = Some(c);
                }
                if ui.selectable_label(*v == c, label(c)).clicked() {
                    picked = Some(c);
                }
            }
            ui.separator();
            match crate::fonts::catalog() {
                None => {
                    ui.weak("Loading installed fonts…");
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(200));
                }
                Some(all) => {
                    let list: Vec<&crate::fonts::Family> =
                        all.iter().filter(|f| matches(&f.name)).collect();
                    if list.is_empty() {
                        ui.weak("No matching fonts");
                    }
                    if enter && picked.is_none() {
                        if let Some(f) = list.first() {
                            picked = Some(FontChoice::Named(crate::fonts::face_for(f)));
                        }
                    }
                    let current = match v {
                        FontChoice::Named(f) => Some(f.name()),
                        _ => None,
                    };
                    let row_h = ui.spacing().interact_size.y;
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .auto_shrink([false, true])
                        .show_rows(ui, row_h, list.len(), |ui, rows| {
                            for f in &list[rows] {
                                let on = current.as_deref() == Some(f.name.as_str());
                                let r = ui.add_sized(
                                    vec2(ui.available_width(), row_h),
                                    egui::SelectableLabel::new(on, &f.name),
                                );
                                if r.clicked() {
                                    picked = Some(FontChoice::Named(crate::fonts::face_for(f)));
                                }
                            }
                        });
                }
            }
            if let Some(c) = picked {
                *v = c;
                ui.memory_mut(|m| m.close_popup());
            }
        });
    // Start with an empty search each time the list opens.
    if shown.inner.is_none() || !ui.memory(|m| m.any_popup_open()) {
        filter = None;
    }
    ui.data_mut(|d| match filter {
        Some(f) => d.insert_temp(filter_id, f),
        None => d.remove::<String>(filter_id),
    });
}

fn pick(pal: &mut Palette, k: usize) -> &mut Color32 {
    match k {
        0 => &mut pal.title,
        1 => &mut pal.body,
        _ => &mut pal.bg,
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
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    if !ui.memory(|m| m.is_popup_open(popup_id)) {
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
        ui.memory_mut(|m| m.close_popup());
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
