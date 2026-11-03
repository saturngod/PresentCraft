//! "Edit in Table Editor…": a spreadsheet dialog over the table under the
//! cursor. Rows and columns can be added, removed, moved and aligned; Save
//! writes the table back as tidy Markdown and leaves the chart settings alone.

use crate::tableblock::{char_of_line, find, lines_of, TableBlock};
use crate::tabletools::{check, style_menu};
use crate::App;
use eframe::egui;
use egui::{pos2, vec2, Color32, FontId, Id, Key, Modifiers, Rect, RichText, Sense, Stroke};

const HEAD_H: f32 = 32.0;
const NUM_W: f32 = 46.0;
const ROW_H: f32 = 36.0;
const ADD_W: f32 = 44.0;
const MIN_COL_W: f32 = 120.0;
const MAX_COL_W: f32 = 340.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Align {
    Default,
    Left,
    Center,
    Right,
}

impl Align {
    fn from_spec(s: &str) -> Self {
        let s = s.trim();
        match (s.starts_with(':'), s.ends_with(':') && s.len() > 1) {
            (true, true) => Align::Center,
            (true, false) => Align::Left,
            (false, true) => Align::Right,
            _ => Align::Default,
        }
    }
    fn spec(self) -> &'static str {
        match self {
            Align::Default => "---",
            Align::Left => ":--",
            Align::Center => ":-:",
            Align::Right => "--:",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sel {
    Row(usize),
    Col(usize),
}

#[derive(Clone, Copy, Debug)]
enum Op {
    InsertRow(usize),
    DeleteRow(usize),
    MoveRow(usize, usize),
    InsertCol(usize),
    DeleteCol(usize),
    MoveCol(usize, usize),
    Align(usize, Align),
    Transpose,
}

/// The grid being edited; the first row is the table's header row.
pub struct TableEditor {
    /// First line of the table in the document, and its lines when opened.
    start: usize,
    original: Vec<String>,
    rows: Vec<Vec<String>>,
    aligns: Vec<Align>,
    sel: Option<Sel>,
    focus: Option<(usize, usize)>,
    pending_focus: Option<(usize, usize)>,
}

enum Outcome {
    Open,
    Cancel,
    Save,
}

fn col_name(mut j: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (j % 26) as u8) as char);
        if j < 26 {
            return s;
        }
        j = j / 26 - 1;
    }
}

impl TableEditor {
    pub fn open(block: &TableBlock, lines: &[String]) -> Self {
        let (mut rows, specs) = block.cells(lines);
        let n = rows
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(0)
            .max(specs.len())
            .max(1);
        if rows.is_empty() {
            rows.push(vec![String::new(); n]);
        }
        for r in &mut rows {
            r.resize(n, String::new());
        }
        let aligns = (0..n)
            .map(|j| specs.get(j).map(|s| Align::from_spec(s)).unwrap_or(Align::Left))
            .collect();
        Self {
            start: block.start,
            original: lines[block.start..block.table_end].to_vec(),
            rows,
            aligns,
            sel: None,
            focus: None,
            pending_focus: Some((0, 0)),
        }
    }

    fn ncols(&self) -> usize {
        self.aligns.len()
    }

    fn apply(&mut self, op: Op) {
        let n = self.ncols();
        match op {
            Op::InsertRow(at) => {
                let at = at.min(self.rows.len());
                self.rows.insert(at, vec![String::new(); n]);
                self.sel = Some(Sel::Row(at));
                self.pending_focus = Some((at, 0));
            }
            Op::DeleteRow(i) => {
                if self.rows.len() > 1 && i < self.rows.len() {
                    self.rows.remove(i);
                }
                self.sel = None;
            }
            Op::MoveRow(a, b) => {
                if a < self.rows.len() && b < self.rows.len() {
                    self.rows.swap(a, b);
                    self.sel = Some(Sel::Row(b));
                }
            }
            Op::InsertCol(at) => {
                let at = at.min(n);
                for r in &mut self.rows {
                    r.insert(at, String::new());
                }
                self.aligns.insert(at, Align::Left);
                self.sel = Some(Sel::Col(at));
                self.pending_focus = Some((0, at));
            }
            Op::DeleteCol(j) => {
                if n > 1 && j < n {
                    for r in &mut self.rows {
                        r.remove(j);
                    }
                    self.aligns.remove(j);
                }
                self.sel = None;
            }
            Op::MoveCol(a, b) => {
                if a < n && b < n {
                    for r in &mut self.rows {
                        r.swap(a, b);
                    }
                    self.aligns.swap(a, b);
                    self.sel = Some(Sel::Col(b));
                }
            }
            Op::Align(j, a) => {
                if let Some(x) = self.aligns.get_mut(j) {
                    *x = a;
                }
            }
            Op::Transpose => {
                let rows = self.rows.len();
                self.rows = (0..n)
                    .map(|j| self.rows.iter().map(|r| r[j].clone()).collect())
                    .collect();
                self.aligns = vec![Align::Left; rows];
                self.sel = None;
            }
        }
        if let Some((i, j)) = self.focus {
            if i >= self.rows.len() || j >= self.ncols() {
                self.focus = None;
            }
        }
    }

    /// Rows and alignment specs ready for `TableBlock::write`.
    fn output(&self) -> (Vec<Vec<String>>, Vec<String>) {
        let rows = self
            .rows
            .iter()
            .map(|r| {
                r.iter()
                    .map(|c| c.replace(['\n', '\r'], " ").replace('|', "/").trim().to_string())
                    .collect()
            })
            .collect();
        let aligns = self.aligns.iter().map(|a| a.spec().to_string()).collect();
        (rows, aligns)
    }

    fn row_menu(&self, ui: &mut egui::Ui, i: usize, op: &mut Option<Op>) {
        style_menu(ui);
        if check(ui, false, "Insert Row Above") {
            *op = Some(Op::InsertRow(i));
        }
        if check(ui, false, "Insert Row Below") {
            *op = Some(Op::InsertRow(i + 1));
        }
        ui.separator();
        if i > 0 && check(ui, false, "Move Row Up") {
            *op = Some(Op::MoveRow(i, i - 1));
        }
        if i + 1 < self.rows.len() && check(ui, false, "Move Row Down") {
            *op = Some(Op::MoveRow(i, i + 1));
        }
        if self.rows.len() > 1 {
            ui.separator();
            if check(ui, false, "Delete Row") {
                *op = Some(Op::DeleteRow(i));
            }
        }
    }

    fn col_menu(&self, ui: &mut egui::Ui, j: usize, op: &mut Option<Op>) {
        style_menu(ui);
        if check(ui, false, "Insert Column Left") {
            *op = Some(Op::InsertCol(j));
        }
        if check(ui, false, "Insert Column Right") {
            *op = Some(Op::InsertCol(j + 1));
        }
        ui.separator();
        let cur = self.aligns[j];
        for (a, name) in [
            (Align::Left, "Align Left"),
            (Align::Center, "Align Center"),
            (Align::Right, "Align Right"),
        ] {
            if check(ui, cur == a, name) {
                *op = Some(Op::Align(j, a));
            }
        }
        ui.separator();
        if j > 0 && check(ui, false, "Move Column Left") {
            *op = Some(Op::MoveCol(j, j - 1));
        }
        if j + 1 < self.ncols() && check(ui, false, "Move Column Right") {
            *op = Some(Op::MoveCol(j, j + 1));
        }
        if self.ncols() > 1 {
            ui.separator();
            if check(ui, false, "Delete Column") {
                *op = Some(Op::DeleteCol(j));
            }
        }
    }

    /// Screen row of table row `i`: the alignment row sits under the header.
    fn line_of(i: usize) -> usize {
        if i == 0 {
            0
        } else {
            i + 1
        }
    }

    fn show(&mut self, ctx: &egui::Context) -> Outcome {
        let pal = crate::settings::pal();
        let dark = crate::settings::is_dark();
        let accent = Color32::from_rgb(0x37, 0x70, 0xFF);
        let sel_fill = if dark {
            Color32::from_rgb(0x34, 0x3E, 0x5C)
        } else {
            Color32::from_rgb(0xC9, 0xD5, 0xF2)
        };
        let head_fill = if dark {
            Color32::from_gray(44)
        } else {
            Color32::from_gray(232)
        };
        let body_fill = if dark {
            crate::theme::hex(0x262626)
        } else {
            Color32::WHITE
        };
        let screen = ctx.content_rect();
        let w = (screen.width() * 0.8).clamp(420.0, 900.0);
        let h = (screen.height() * 0.72).clamp(280.0, 620.0);
        let grid_h = h - 64.0;
        let font = FontId::new(16.0, crate::theme::mono(false));
        let pipe_col = pal.text_dim.gamma_multiply(0.7);

        let mut out = Outcome::Open;
        let mut op: Option<Op> = None;
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            return Outcome::Cancel;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            || ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S))
        {
            return Outcome::Save;
        }

        let frame = egui::Frame::new()
            .fill(body_fill)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .corner_radius(12)
            .shadow(ctx.global_style().visuals.window_shadow)
            .inner_margin(egui::Margin::same(0));
        egui::Modal::new(Id::unique("table-editor"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(w);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                // Column widths follow their content.
                let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '-'));
                let widths: Vec<f32> = (0..self.ncols())
                    .map(|j| {
                        let longest = self
                            .rows
                            .iter()
                            .map(|r| r[j].chars().count())
                            .max()
                            .unwrap_or(0);
                        (longest as f32 * char_w + 40.0).clamp(MIN_COL_W, MAX_COL_W)
                    })
                    .collect();
                let lines = self.rows.len() + 1;
                let total_w = NUM_W + widths.iter().sum::<f32>() + ADD_W;
                let total_h = HEAD_H + ROW_H * lines as f32 + ROW_H;

                egui::ScrollArea::both()
                    .id_salt("table-editor-scroll")
                    .min_scrolled_height(grid_h)
                    .max_height(grid_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (area, _) = ui.allocate_exact_size(
                            vec2(total_w.max(ui.available_width()), total_h.max(grid_h)),
                            Sense::hover(),
                        );
                        let o = area.min;
                        let p = ui.painter().clone();
                        let col_x = |j: usize| o.x + NUM_W + widths[..j].iter().sum::<f32>();
                        let row_y = |line: usize| o.y + HEAD_H + ROW_H * line as f32;
                        let grid_right = col_x(widths.len());
                        let line_div = Stroke::new(1.0_f32, pal.border);

                        // header strips
                        p.rect_filled(
                            Rect::from_min_max(o, pos2(area.right(), o.y + HEAD_H)),
                            0.0,
                            head_fill,
                        );
                        p.rect_filled(
                            Rect::from_min_max(o, pos2(o.x + NUM_W, area.bottom())),
                            0.0,
                            head_fill,
                        );
                        p.line_segment(
                            [pos2(o.x, o.y + HEAD_H), pos2(area.right(), o.y + HEAD_H)],
                            line_div,
                        );
                        p.line_segment(
                            [pos2(o.x + NUM_W, o.y), pos2(o.x + NUM_W, area.bottom())],
                            line_div,
                        );

                        // corner: transpose
                        let corner = Rect::from_min_size(o, vec2(NUM_W, HEAD_H));
                        let t = ui
                            .interact(corner, Id::unique("te-transpose"), Sense::click())
                            .on_hover_text("Transpose table");
                        if t.hovered() {
                            p.rect_filled(corner, 0.0, pal.hover);
                        }
                        transpose_icon(&p, corner.center(), pal.text_hi, accent);
                        if t.clicked() {
                            op = Some(Op::Transpose);
                        }

                        // column headers
                        for j in 0..widths.len() {
                            let r = Rect::from_min_size(pos2(col_x(j), o.y), vec2(widths[j], HEAD_H));
                            let resp = ui.interact(r, Id::unique(("te-col", j)), Sense::click());
                            let on = self.sel == Some(Sel::Col(j));
                            if on {
                                p.rect_filled(r, 0.0, sel_fill);
                            } else if resp.hovered() {
                                p.rect_filled(r, 0.0, pal.hover);
                            }
                            p.line_segment([r.right_top(), r.right_bottom()], line_div);
                            let col = if on { accent } else { pal.text_dim };
                            p.text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                col_name(j),
                                FontId::proportional(13.0),
                                col,
                            );
                            if on || resp.hovered() {
                                let c = pos2(r.right() - 16.0, r.center().y);
                                p.line(
                                    vec![c + vec2(-4.0, -2.0), c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)],
                                    Stroke::new(1.5_f32, col),
                                );
                            }
                            let pid = Id::unique(("te-col-menu", j));
                            if resp.clicked() {
                                self.sel = Some(Sel::Col(j));
                                egui::Popup::toggle_id(ui.ctx(), pid);
                            }
                            crate::tabletools::popup_below(
                                pid,
                                &resp,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| self.col_menu(ui, j, &mut op),
                            );
                            resp.context_menu(|ui| {
                                self.sel = Some(Sel::Col(j));
                                self.col_menu(ui, j, &mut op)
                            });
                        }
                        let add_c = pos2(grid_right + ADD_W / 2.0, o.y + HEAD_H / 2.0);
                        if round_button(ui, &p, add_c, "te-add-col", "Add column", accent, true) {
                            op = Some(Op::InsertCol(self.ncols()));
                        }

                        // row numbers (the alignment row is line 2 and has no menu)
                        for line in 0..lines {
                            let r = Rect::from_min_size(pos2(o.x, row_y(line)), vec2(NUM_W, ROW_H));
                            p.line_segment([r.left_bottom(), r.right_bottom()], line_div);
                            let row = match line {
                                0 => Some(0),
                                1 => None,
                                l => Some(l - 1),
                            };
                            let Some(i) = row else {
                                p.text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "2",
                                    FontId::proportional(13.0),
                                    pal.text_dim,
                                );
                                continue;
                            };
                            let resp = ui.interact(r, Id::unique(("te-row", i)), Sense::click());
                            let on = self.sel == Some(Sel::Row(i));
                            if on {
                                p.rect_filled(r, 0.0, sel_fill);
                            } else if resp.hovered() {
                                p.rect_filled(r, 0.0, pal.hover);
                            }
                            p.text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                (line + 1).to_string(),
                                FontId::proportional(13.0),
                                if on { accent } else { pal.text_dim },
                            );
                            let pid = Id::unique(("te-row-menu", i));
                            if resp.clicked() {
                                self.sel = Some(Sel::Row(i));
                                egui::Popup::toggle_id(ui.ctx(), pid);
                            }
                            crate::tabletools::popup_below(
                                pid,
                                &resp,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| self.row_menu(ui, i, &mut op),
                            );
                            resp.context_menu(|ui| {
                                self.sel = Some(Sel::Row(i));
                                self.row_menu(ui, i, &mut op)
                            });
                        }
                        let add_r = pos2(o.x + NUM_W / 2.0, row_y(lines) + ROW_H / 2.0);
                        if round_button(ui, &p, add_r, "te-add-row", "Add row", accent, false) {
                            op = Some(Op::InsertRow(self.rows.len()));
                        }

                        // pipes, like the Markdown they stand for
                        for line in 0..lines {
                            let y = row_y(line) + ROW_H / 2.0;
                            for j in 0..=widths.len() {
                                p.text(
                                    pos2(col_x(j) + 2.0, y),
                                    egui::Align2::CENTER_CENTER,
                                    "|",
                                    font.clone(),
                                    pipe_col,
                                );
                            }
                        }
                        // the alignment row
                        for (j, wj) in widths.iter().enumerate() {
                            let n = (((wj - 24.0) / char_w).floor() as usize).max(3);
                            let dashes = |k: usize| "-".repeat(n.saturating_sub(k));
                            let s = match self.aligns[j] {
                                Align::Left => format!(":{}", dashes(1)),
                                Align::Right => format!("{}:", dashes(1)),
                                Align::Center => format!(":{}:", dashes(2)),
                                Align::Default => dashes(0),
                            };
                            p.text(
                                pos2(col_x(j) + 16.0, row_y(1) + ROW_H / 2.0),
                                egui::Align2::LEFT_CENTER,
                                s,
                                font.clone(),
                                pipe_col,
                            );
                        }

                        // cells
                        let enter = ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.command);
                        let (up, down) = ui.input(|i| {
                            (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown))
                        });
                        let shift = ui.input(|i| i.modifiers.shift);
                        let mut move_to: Option<(usize, usize)> = None;
                        let mut grow = false;
                        let nrows = self.rows.len();
                        let row_h = ui.fonts_mut(|f| f.row_height(&font));
                        for i in 0..nrows {
                            for j in 0..widths.len() {
                                let cell = Rect::from_min_size(
                                    pos2(col_x(j), row_y(Self::line_of(i))),
                                    vec2(widths[j], ROW_H),
                                );
                                let r = Rect::from_center_size(
                                    cell.center() + vec2(2.0, 0.0),
                                    vec2(cell.width() - 32.0, row_h),
                                );
                                let align = match self.aligns[j] {
                                    Align::Center => egui::Align::Center,
                                    Align::Right => egui::Align::Max,
                                    _ => egui::Align::Min,
                                };
                                let resp = ui.put(
                                    r,
                                    egui::TextEdit::singleline(&mut self.rows[i][j])
                                        .id(Id::unique(("te-cell", i, j)))
                                        .frame(
                                            egui::Frame::NONE
                                                .inner_margin(egui::Margin::symmetric(4, 2)),
                                        )
                                        .font(font.clone())
                                        .text_color(pal.text_hi)
                                        .align(egui::Align2([align, egui::Align::TOP]))
                                        .desired_width(r.width())
                                        .margin(vec2(0.0, 0.0)),
                                );
                                if self.pending_focus == Some((i, j)) {
                                    resp.request_focus();
                                    resp.scroll_to_me(None);
                                    self.pending_focus = None;
                                }
                                if resp.gained_focus() {
                                    self.sel = None;
                                }
                                if resp.has_focus() {
                                    self.focus = Some((i, j));
                                    if up && i > 0 {
                                        move_to = Some((i - 1, j));
                                    } else if down && i + 1 < nrows {
                                        move_to = Some((i + 1, j));
                                    }
                                }
                                if resp.lost_focus() && enter {
                                    if shift {
                                        move_to = Some((i.saturating_sub(1), j));
                                    } else {
                                        grow = i + 1 == nrows;
                                        move_to = Some((i + 1, j));
                                    }
                                }
                            }
                        }
                        if grow && op.is_none() {
                            self.rows.push(vec![String::new(); self.aligns.len()]);
                        }
                        if move_to.is_some() {
                            self.pending_focus = move_to;
                            ctx.request_repaint();
                        }
                    });

                // footer
                let (sep, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
                ui.painter().rect_filled(sep, 0.0, pal.border);
                ui.horizontal(|ui| {
                    ui.set_height(63.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(18.0);
                        ui.spacing_mut().item_spacing.x = 10.0;
                        let button = |t: &str| {
                            egui::Button::new(RichText::new(t).size(14.0))
                                .fill(head_fill)
                                .stroke(Stroke::NONE)
                                .corner_radius(7)
                                .min_size(vec2(76.0, 30.0))
                        };
                        if ui.add(button("Save")).clicked() {
                            out = Outcome::Save;
                        }
                        if ui.add(button("Cancel")).clicked() {
                            out = Outcome::Cancel;
                        }
                    });
                });
            });
        if let Some(op) = op {
            self.apply(op);
            ctx.request_repaint();
        }
        out
    }
}

/// The round "add" buttons: `||` adds a column, `=` adds a row.
fn round_button(
    ui: &egui::Ui,
    p: &egui::Painter,
    c: egui::Pos2,
    id: &str,
    tip: &str,
    accent: Color32,
    column: bool,
) -> bool {
    let resp = ui
        .interact(Rect::from_center_size(c, vec2(26.0, 26.0)), Id::unique(id), Sense::click())
        .on_hover_text(tip);
    let pal = crate::settings::pal();
    if resp.hovered() {
        p.circle_filled(c, 12.0, pal.hover);
    }
    p.circle_stroke(c, 10.0, Stroke::new(1.6_f32, pal.text_dim));
    let s = Stroke::new(2.4_f32, accent);
    for d in [-2.5, 2.5] {
        if column {
            p.line_segment([c + vec2(d, -4.0), c + vec2(d, 4.0)], s);
        } else {
            p.line_segment([c + vec2(-4.0, d), c + vec2(4.0, d)], s);
        }
    }
    resp.clicked()
}

/// Two offset frames joined by a turning arrow.
fn transpose_icon(p: &egui::Painter, c: egui::Pos2, col: Color32, accent: Color32) {
    let s = Stroke::new(1.5_f32, col);
    let r = Rect::from_center_size(c + vec2(-1.0, 1.5), vec2(14.0, 9.0));
    p.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
    let a = Stroke::new(1.4_f32, accent);
    let arc: Vec<egui::Pos2> = (0..=8)
        .map(|k| {
            let t = std::f32::consts::PI * (1.0 + 0.5 * k as f32 / 8.0);
            c + vec2(3.0, 3.0) + vec2(t.cos(), t.sin()) * 9.0
        })
        .collect();
    let end = *arc.last().unwrap();
    p.line(arc, a);
    p.line(vec![end + vec2(-3.0, -2.5), end, end + vec2(-3.0, 2.5)], a);
}

impl App {
    /// Opens the table editor for the table at `line` (any line of it).
    pub fn open_table_editor(&mut self, line: usize) {
        let lines = lines_of(&self.text);
        if let Some(block) = find(&lines, line) {
            self.table_editor = Some(TableEditor::open(&block, &lines));
        }
    }

    pub fn table_editor_ui(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.table_editor.as_mut() else {
            return;
        };
        match ed.show(ctx) {
            Outcome::Open => return,
            Outcome::Cancel => {}
            Outcome::Save => {
                let mut lines = lines_of(&self.text);
                let block = find(&lines, ed.start).filter(|b| {
                    b.start == ed.start && lines[b.start..b.table_end] == ed.original[..]
                });
                if let Some(mut block) = block {
                    let (rows, aligns) = ed.output();
                    block.write(&mut lines, rows, aligns);
                    self.text = lines.join("\n");
                    self.dirty = true;
                    self.reparse();
                    let total = self.text.chars().count();
                    self.cursor = char_of_line(&lines, block.start).min(total);
                    self.sel_start = self.cursor;
                    self.jump_to = Some((self.cursor, self.cursor));
                } else {
                    self.status = "The table changed while editing; nothing was saved".into();
                }
            }
        }
        self.table_editor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(text: &str) -> (Vec<String>, TableEditor) {
        let lines = lines_of(text);
        let b = find(&lines, 0).unwrap();
        let e = TableEditor::open(&b, &lines);
        (lines, e)
    }

    #[test]
    fn opens_and_writes_back() {
        let (mut lines, mut e) =
            editor("| | S1 | S2 |\n|:--|:-:|--:|\n| A | 10 | 15 |\n| B | 15 |\nChart: Bar");
        assert_eq!(e.rows.len(), 3);
        assert_eq!(e.rows[2], vec!["B", "15", ""]);
        assert_eq!(e.aligns, vec![Align::Left, Align::Center, Align::Right]);
        e.apply(Op::InsertRow(3));
        e.rows[3] = vec!["C".into(), "25".into(), "30".into()];
        e.apply(Op::InsertCol(3));
        e.rows[0][3] = "S3".into();
        e.apply(Op::DeleteCol(1));
        let (rows, aligns) = e.output();
        let mut b = find(&lines, 0).unwrap();
        b.write(&mut lines, rows, aligns);
        assert_eq!(lines.len(), 6);
        assert!(lines[0].contains("S2") && lines[0].contains("S3") && !lines[0].contains("S1"));
        assert_eq!(lines[1].matches("-: |").count(), 1);
        assert!(lines[4].contains('C') && lines[4].contains("30"));
        assert_eq!(lines[5], "Chart: Bar");
    }

    #[test]
    fn keeps_at_least_one_row_and_column() {
        let (_, mut e) = editor("| a |\n|--|");
        e.apply(Op::DeleteRow(0));
        e.apply(Op::DeleteCol(0));
        assert_eq!((e.rows.len(), e.ncols()), (1, 1));
        e.apply(Op::InsertCol(1));
        e.apply(Op::InsertRow(1));
        e.rows[1][1] = "x".into();
        e.apply(Op::Transpose);
        assert_eq!(e.rows[1][1], "x");
        e.apply(Op::MoveRow(1, 0));
        assert_eq!(e.rows[0][1], "x");
    }

    #[test]
    fn column_letters() {
        assert_eq!(col_name(0), "A");
        assert_eq!(col_name(25), "Z");
        assert_eq!(col_name(26), "AA");
    }
}
