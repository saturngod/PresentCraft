//! Floating toolbar over the table under the cursor:
//! [table] [lines] [bars] [pie] | [edit] | [•••]. The bar picks how the data
//! is shown and opens the spreadsheet-style table editor; the ••• menu holds
//! chart variants, caption, legend, colours and axis labels, and
//! reformats/transposes the table. The data rows are
//! never changed by a type switch — only the `Chart:` line is rewritten.

use crate::parse::{ChartKind, ColorType};
use crate::tableblock::{char_of_line, find, line_of_char, lines_of};
use crate::{slide_color, App};
use eframe::egui;
use egui::text::CCursor;
use egui::{pos2, vec2, Color32, Galley, Pos2, Rect, RichText, Sense, Stroke};

enum Act {
    Chart(Option<(ChartKind, bool, bool)>),
    Caption(bool),
    Meta(&'static str, &'static str, Option<String>),
    Reformat,
    Transpose,
    EditTable,
    SaveCsv,
}

const COLOR_SETS: [(&str, &str); 4] = [
    ("Warm", "#F94144, #F3722C, #F8961E, #F9C74F, #90BE6D"),
    ("Cool", "#277DA1, #577590, #4D908E, #43AA8B, #90BE6D"),
    ("Grays", "#222222, #555555, #888888, #BBBBBB, #DDDDDD"),
    ("Vivid", "#FF3333, #3770FF, #FFC400, #43AA8B, #A784FF"),
];

/// The "Color Type" section shared by the chart type submenus.
fn color_type_rows(ui: &mut egui::Ui, current: &ColorType, act: &mut Option<Act>) {
    ui.separator();
    heading(ui, "Color Type");
    for (t, name, value) in [
        (ColorType::Categorical, "Categorical — Default", None),
        (ColorType::Sequential, "Sequential", Some("sequential")),
        (ColorType::Differential, "Differential", Some("differential")),
    ] {
        if check(ui, *current == t, name) {
            *act = Some(Act::Meta(
                "color-type",
                "Color-Type",
                value.map(str::to_string),
            ));
        }
    }
}

/// Menu rows share one layout: a checkmark column, then the text column.
const MENU_TICK_X: f32 = 15.0;
const MENU_TEXT_X: f32 = 30.0;
const MENU_ROW_H: f32 = 27.0;

// Draw checkmarks so menu state never depends on font glyph coverage.
pub(crate) fn tick(ui: &egui::Ui, rect: Rect) {
    let c = pos2(rect.left() + MENU_TICK_X, rect.center().y);
    ui.painter().add(egui::Shape::line(
        vec![
            c + vec2(-4.0, 0.0),
            c + vec2(-1.0, 3.0),
            c + vec2(5.0, -4.0),
        ],
        Stroke::new(1.7_f32, crate::settings::pal().text_hi),
    ));
}

/// A row that opens a submenu; ticked when one of its choices is active.
/// `label` may start with "✓" to show the tick.
pub(crate) fn submenu<R>(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let label = label.as_ref();
    let selected = label.starts_with('✓');
    let text = label.trim_start_matches('✓').trim();
    // Indent the text to the shared column (button padding is set by style_menu).
    let pad = ui.spacing().button_padding.x;
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        MENU_TEXT_X - pad,
        egui::TextFormat {
            font_id: egui::TextStyle::Button.resolve(ui.style()),
            color: crate::settings::pal().text_hi,
            ..Default::default()
        },
    );
    let response = ui.menu_button(job, |ui| {
        style_menu(ui);
        ui.set_min_width(200.0);
        contents(ui)
    });
    if selected {
        tick(ui, response.response.rect);
    }
}

/// A menu choice with an optional tick; closes the menu when clicked.
pub(crate) fn check(ui: &mut egui::Ui, on: bool, text: &str) -> bool {
    // While egui measures a new menu the available width is the whole
    // window; taking it there would make the menu that wide.
    let w = if ui.is_sizing_pass() {
        200.0
    } else {
        ui.available_width().max(200.0)
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, MENU_ROW_H), Sense::click());
    let pal = crate::settings::pal();
    if resp.hovered() {
        ui.painter().rect_filled(rect, 5.0, pal.hover);
    }
    if on {
        tick(ui, rect);
    }
    ui.painter().text(
        pos2(rect.left() + MENU_TEXT_X, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::TextStyle::Button.resolve(ui.style()),
        pal.text_hi,
    );
    if resp.clicked() {
        ui.close();
    }
    resp.clicked()
}

pub(crate) fn heading(ui: &mut egui::Ui, t: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add_space(MENU_TEXT_X - ui.spacing().item_spacing.x);
        heading_label(ui, t);
    });
}

fn heading_label(ui: &mut egui::Ui, t: &str) {
    ui.label(
        RichText::new(t)
            .size(11.5)
            .strong()
            .color(crate::settings::pal().text_dim),
    );
}

/// Floating toolbar background: a rounded pill with a soft shadow.
pub(crate) fn paint_bar(p: &egui::Painter, bar: Rect) {
    let pal = crate::settings::pal();
    let dark = crate::settings::is_dark();
    p.add(
        egui::epaint::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 110 } else { 34 }),
        }
        .as_shape(bar, egui::CornerRadius::same(22)),
    );
    p.rect_filled(
        bar,
        22.0,
        if dark {
            Color32::from_rgb(48, 48, 48)
        } else {
            pal.pill
        },
    );
    p.rect_stroke(
        bar,
        22.0,
        Stroke::new(
            1.0_f32,
            if dark {
                Color32::from_gray(74)
            } else {
                pal.border
            },
        ),
        egui::StrokeKind::Inside,
    );
}

/// Thin vertical divider between toolbar groups.
pub(crate) fn bar_divider(p: &egui::Painter, bar: Rect, x: f32) {
    p.line_segment(
        [
            pos2(bar.left() + x, bar.top() + 12.0),
            pos2(bar.left() + x, bar.bottom() - 12.0),
        ],
        Stroke::new(1.0_f32, crate::settings::pal().border),
    );
}

/// A round toolbar button; returns the response and the icon colour to use.
pub(crate) fn round_button(
    ui: &egui::Ui,
    p: &egui::Painter,
    c: Pos2,
    id: &str,
    tip: &str,
    selected: bool,
    accent: Color32,
) -> (egui::Response, Color32) {
    let resp = ui
        .interact(
            Rect::from_center_size(c, vec2(36.0, 36.0)),
            ui.scope_id().with(id),
            Sense::click(),
        )
        .on_hover_text(tip);
    let pal = crate::settings::pal();
    if selected {
        p.circle_filled(c, 17.0, accent);
    } else if resp.hovered() {
        p.circle_filled(c, 17.0, pal.hover);
    }
    (
        resp,
        if selected {
            Color32::WHITE
        } else {
            pal.text_hi
        },
    )
}

/// Menu look shared by the toolbars' ••• menus.
/// A menu opened by a custom button; returns the button's response.
pub(crate) fn menu_from_button<R>(
    ui: &mut egui::Ui,
    button: egui::Button<'_>,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::Response {
    egui::containers::menu::MenuButton::from_button(button)
        .ui(ui, contents)
        .0
}

/// A popup under `resp`, open while egui's popup memory says `id` is open.
pub(crate) fn popup_below<R>(
    id: egui::Id,
    resp: &egui::Response,
    close: egui::PopupCloseBehavior,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    egui::Popup::from_response(resp)
        .id(id)
        .open_memory(None)
        .close_behavior(close)
        .show(contents)
        .map(|r| r.inner)
}

pub(crate) fn style_menu(ui: &mut egui::Ui) {
    ui.set_min_width(240.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.spacing_mut().button_padding = vec2(8.0, 5.0);
    ui.spacing_mut().interact_size.y = 27.0;
    let pal = crate::settings::pal();
    ui.style_mut().visuals.override_text_color = Some(pal.text_hi);
    let widgets = &mut ui.style_mut().visuals.widgets;
    for w in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(5);
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0_f32, pal.text_hi);
    }
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_fill = Color32::TRANSPARENT;
    for w in [&mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        w.weak_bg_fill = pal.hover;
        w.bg_fill = pal.hover;
    }
}

/// The ••• button that opens a toolbar menu.
pub(crate) fn more_button<R>(
    ui: &mut egui::Ui,
    p: &egui::Painter,
    c: Pos2,
    id: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let more = Rect::from_center_size(c, vec2(34.0, 34.0));
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(id).max_rect(more));
    {
        let pal = crate::settings::pal();
        let v = &mut child.style_mut().visuals.widgets;
        for w in [&mut v.inactive, &mut v.hovered, &mut v.active, &mut v.open] {
            w.corner_radius = egui::CornerRadius::same(17);
            w.bg_stroke = Stroke::NONE;
        }
        v.inactive.weak_bg_fill = Color32::TRANSPARENT;
        v.hovered.weak_bg_fill = pal.hover;
        v.open.weak_bg_fill = pal.hover;
        v.active.weak_bg_fill = pal.hover;
    }
    let btn = egui::Button::new("").min_size(vec2(34.0, 34.0));
    menu_from_button(&mut child, btn, |ui| {
        style_menu(ui);
        contents(ui)
    })
    .on_hover_text("More options");
    let col = crate::settings::pal().text_hi;
    if !crate::icons::paint(&p, "ellipsis", c, 15.0, col) {
        for dx in [-6.0, 0.0, 6.0] {
            p.circle_filled(c + vec2(dx, 0.0), 2.0, col);
        }
    }
}

fn table_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    if crate::icons::paint(p, "tablecells", c, 15.0, col) {
        return;
    }
    let s = Stroke::new(1.6_f32, col);
    let b = Rect::from_center_size(c, vec2(18.0, 14.0));
    p.rect_stroke(b, 2.0, s, egui::StrokeKind::Inside);
    p.line_segment(
        [
            pos2(b.left(), b.top() + 4.7),
            pos2(b.right(), b.top() + 4.7),
        ],
        s,
    );
    p.line_segment(
        [
            pos2(b.left(), b.top() + 9.3),
            pos2(b.right(), b.top() + 9.3),
        ],
        s,
    );
    p.line_segment(
        [pos2(b.center().x, b.top()), pos2(b.center().x, b.bottom())],
        s,
    );
}

fn chart_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    if crate::icons::paint(p, "chart.bar.xaxis", c, 15.0, col) {
        return;
    }
    for (k, h) in [9.0, 13.0, 11.0, 7.0].iter().enumerate() {
        let x = c.x - 8.0 + k as f32 * 4.3;
        p.rect_filled(
            Rect::from_min_max(pos2(x, c.y + 5.0 - h), pos2(x + 2.8, c.y + 5.0)),
            0.5,
            col,
        );
    }
    p.line_segment(
        [pos2(c.x - 9.0, c.y + 7.5), pos2(c.x + 9.0, c.y + 7.5)],
        Stroke::new(1.5_f32, col),
    );
}

/// A rising line (SF Symbols "chart.xyaxis.line").
fn line_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    if crate::icons::paint(p, "chart.xyaxis.line", c, 15.0, col) {
        return;
    }
    p.add(egui::Shape::line(
        vec![
            c + vec2(-8.0, 5.0),
            c + vec2(-3.0, -1.0),
            c + vec2(1.5, 3.0),
            c + vec2(8.0, -6.0),
        ],
        Stroke::new(1.7_f32, col),
    ));
}

/// A circle with one slice cut out (SF Symbols "chart.pie").
fn pie_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    if crate::icons::paint(p, "chart.pie", c, 15.0, col) {
        return;
    }
    let s = Stroke::new(1.6_f32, col);
    p.circle_stroke(c, 8.0, s);
    p.line_segment([c, c + vec2(0.0, -8.0)], s);
    p.line_segment([c, c + vec2(8.0, 0.0)], s);
}

/// A sheet with a pencil across its corner (SF Symbols "square.and.pencil").
fn edit_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    if crate::icons::paint(p, "square.and.pencil", c, 15.0, col) {
        return;
    }
    p.add(egui::Shape::line(
        vec![
            c + vec2(0.5, -6.5),
            c + vec2(-7.5, -6.5),
            c + vec2(-7.5, 7.5),
            c + vec2(6.5, 7.5),
            c + vec2(6.5, -0.5),
        ],
        Stroke::new(1.6_f32, col),
    ));
    p.line_segment(
        [c + vec2(-1.0, 1.0), c + vec2(8.0, -8.0)],
        Stroke::new(2.0_f32, col),
    );
}

/// Whether the axis-labels popup is open (kept in egui's temporary memory).
fn axis_popup_id() -> egui::Id {
    egui::Id::unique("table-axis-labels")
}

/// A small popup under the bar with the chart's two axis labels. Typing in a
/// submenu is fragile (it closes when the pointer drifts), so these live here.
fn axis_labels_popup(
    ctx: &egui::Context,
    bar: Rect,
    block: &crate::tableblock::TableBlock,
    lines: &[String],
    has_axes: bool,
    act: &mut Option<Act>,
) {
    let id = axis_popup_id();
    if !ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false)) {
        return;
    }
    let focus = ctx.data_mut(|d| d.remove_temp::<bool>(id.with("focus")).unwrap_or(false));
    let mut close = !has_axes || ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(bar.left_bottom() + vec2(0.0, 6.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (k, (label, key, shown)) in
                    [("X Axis", "xlabel", "xLabel"), ("Y Axis", "ylabel", "yLabel")]
                        .into_iter()
                        .enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [48.0, 24.0],
                            egui::Label::new(
                                RichText::new(label).color(crate::settings::pal().text_dim),
                            ),
                        );
                        let mut v = block.get(lines, key).unwrap_or_default();
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut v)
                                .desired_width(190.0)
                                .hint_text("None"),
                        );
                        if k == 0 && focus {
                            resp.request_focus();
                        }
                        if resp.changed() {
                            *act = Some(Act::Meta(
                                key,
                                shown,
                                Some(v).filter(|s| !s.trim().is_empty()),
                            ));
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            close = true;
                        }
                    });
                }
            });
        });
    if close || area.response.clicked_elsewhere() {
        ctx.data_mut(|d| d.remove::<bool>(id));
    }
}

impl App {
    pub fn table_toolbar(&mut self, ui: &mut egui::Ui, galley: &Galley, galley_pos: Pos2) {
        let lines = lines_of(&self.text);
        let cur_line = line_of_char(&self.text, self.cursor);
        let Some(block) = find(&lines, cur_line) else {
            ui.ctx().data_mut(|d| d.remove::<bool>(axis_popup_id()));
            return;
        };
        let start_ch = char_of_line(&lines, block.start);
        let r = galley.pos_from_cursor(CCursor::new(start_ch));
        let clip = ui.clip_rect();
        // Above the table. When there is no room there, beside its first
        // line at the right edge of the column, so the bar does not cover it.
        const BAR_W: f32 = 278.0;
        let above = galley_pos.y + r.top() - 50.0;
        let mut left = galley_pos.x - 14.0;
        if above < clip.top() + 6.0 {
            let end_ch = start_ch + lines[block.start].chars().count();
            let e = galley.pos_from_cursor(CCursor::new(end_ch));
            let beside = galley_pos.x + galley.job.wrap.max_width + 14.0 - BAR_W;
            if (e.top() - r.top()).abs() < 1.0 && galley_pos.x + e.right() + 24.0 <= beside {
                left = beside;
            }
        }
        let y = if left > galley_pos.x {
            galley_pos.y + r.center().y - 22.0
        } else {
            above
        }
        .max(clip.top() + 6.0)
        .min(clip.bottom() - 50.0);
        let bar = Rect::from_min_size(pos2(left, y), vec2(BAR_W, 44.0));
        let chart = block.chart(&lines);
        let kind = chart.map(|c| c.0);
        let lines_on = matches!(
            kind,
            Some(ChartKind::Line | ChartKind::Spline | ChartKind::Area | ChartKind::Step)
        );
        let bars_on = kind == Some(ChartKind::Bar);
        let pie_on = matches!(kind, Some(ChartKind::Pie | ChartKind::Donut));
        // Pie, donut, radar and gauge charts have no x and y axes to label.
        let has_axes = lines_on || bars_on || kind == Some(ChartKind::Bubble);
        let accent = slide_color(self.current);
        // Keep the overlay above TextEdit for both painting and hit testing.
        // Sharing the editor layer lets its cursor handling consume toolbar clicks.
        let mut act: Option<Act> = None;
        egui::Area::new(egui::Id::unique("table-toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(bar.min)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(bar.size());
                let p = ui.painter().clone();
                paint_bar(&p, bar);
                let cy = bar.center().y;
                let c = |dx: f32| pos2(bar.left() + dx, cy);
                let c_more = c(253.0);
                let (t_resp, t_col) = round_button(
                    ui,
                    &p,
                    c(25.0),
                    "tt-table",
                    "Show as table",
                    chart.is_none(),
                    accent,
                );
                table_icon(&p, c(25.0), t_col);
                if t_resp.clicked() && chart.is_some() {
                    act = Some(Act::Chart(None));
                }
                type Icon = fn(&egui::Painter, Pos2, Color32);
                let kinds: [(f32, &str, &str, bool, ChartKind, Icon); 3] = [
                    (69.0, "tt-lines", "Show as line chart", lines_on, ChartKind::Line, line_icon),
                    (113.0, "tt-chart", "Show as bar chart", bars_on, ChartKind::Bar, chart_icon),
                    (157.0, "tt-pie", "Show as pie chart", pie_on, ChartKind::Pie, pie_icon),
                ];
                for (dx, id, tip, on, k, icon) in kinds {
                    let (resp, col) = round_button(ui, &p, c(dx), id, tip, on, accent);
                    icon(&p, c(dx), col);
                    if resp.clicked() && !on {
                        act = Some(Act::Chart(Some((k, false, false))));
                    }
                }
                bar_divider(&p, bar, 181.0);
                let (e_resp, e_col) = round_button(
                    ui,
                    &p,
                    c(205.0),
                    "tt-edit",
                    "Edit in the table editor",
                    false,
                    accent,
                );
                edit_icon(&p, c(205.0), e_col);
                if e_resp.clicked() {
                    act = Some(Act::EditTable);
                }
                bar_divider(&p, bar, 229.0);

                // ••• menu
                let caption_on = block.get(&lines, "[caption]").is_some();
                let colors = block.get(&lines, "colors");
                let color_type = block
                    .get(&lines, "color-type")
                    .map(|v| ColorType::from_word(&v))
                    .unwrap_or_default();
                let legend_off = block
                    .get(&lines, "legend")
                    .map(|v| {
                        matches!(
                            v.to_ascii_lowercase().as_str(),
                            "false" | "no" | "off" | "none"
                        )
                    })
                    .unwrap_or(false);
                more_button(ui, &p, c_more, "table-more", |ui| {
                    heading(ui, "Chart");
                    let is = |k: ChartKind| kind == Some(k);
                    submenu(
                        ui,
                        format!("{}Lines", if lines_on { "✓  " } else { "     " }),
                        |ui| {
                            heading(ui, "Style");
                            let stacked = chart.map(|c| c.2).unwrap_or(false);
                            for (k, s, name) in [
                                (ChartKind::Line, false, "Lines — Default"),
                                (ChartKind::Spline, false, "Smooth"),
                                (ChartKind::Step, false, "Steps"),
                                (ChartKind::Area, false, "Area"),
                                (ChartKind::Area, true, "Stacked Area"),
                            ] {
                                let on = is(k) && (k != ChartKind::Area || stacked == s);
                                if check(ui, on, name) {
                                    act = Some(Act::Chart(Some((k, false, s))));
                                }
                            }
                            color_type_rows(ui, &color_type, &mut act);
                        },
                    );
                    submenu(
                        ui,
                        format!("{}Bars", if is(ChartKind::Bar) { "✓  " } else { "     " }),
                        |ui| {
                            let bar = is(ChartKind::Bar);
                            let (h, s) = chart
                                .filter(|c| c.0 == ChartKind::Bar)
                                .map(|c| (c.1, c.2))
                                .unwrap_or((false, false));
                            heading(ui, "Style");
                            for (ss, name) in [(false, "Separated — Default"), (true, "Stacked")] {
                                if check(ui, bar && s == ss, name) {
                                    act = Some(Act::Chart(Some((ChartKind::Bar, h, ss))));
                                }
                            }
                            ui.separator();
                            heading(ui, "Orientation");
                            for (hh, name) in [(false, "Vertical — Default"), (true, "Horizontal")]
                            {
                                if check(ui, bar && h == hh, name) {
                                    act = Some(Act::Chart(Some((ChartKind::Bar, hh, s))));
                                }
                            }
                            color_type_rows(ui, &color_type, &mut act);
                        },
                    );
                    submenu(
                        ui,
                        format!(
                            "{}Pie",
                            if is(ChartKind::Pie) || is(ChartKind::Donut) {
                                "✓  "
                            } else {
                                "     "
                            }
                        ),
                        |ui| {
                            heading(ui, "Style");
                            for (k, name) in
                                [(ChartKind::Pie, "Pie — Default"), (ChartKind::Donut, "Donut")]
                            {
                                if check(ui, is(k), name) {
                                    act = Some(Act::Chart(Some((k, false, false))));
                                }
                            }
                            color_type_rows(ui, &color_type, &mut act);
                        },
                    );
                    submenu(
                        ui,
                        format!(
                            "{}Other Charts",
                            if is(ChartKind::Radar) || is(ChartKind::Gauge) || is(ChartKind::Bubble)
                            {
                                "✓  "
                            } else {
                                "     "
                            }
                        ),
                        |ui| {
                            for (k, name) in [
                                (ChartKind::Radar, "Radar"),
                                (ChartKind::Gauge, "Gauge"),
                                (ChartKind::Bubble, "Bubble"),
                            ] {
                                if check(ui, is(k), name) {
                                    act = Some(Act::Chart(Some((k, false, false))));
                                }
                            }
                        },
                    );
                    ui.separator();
                    if check(ui, caption_on, "Caption") {
                        act = Some(Act::Caption(!caption_on));
                    }
                    if chart.is_some() && check(ui, !legend_off, "Legend") {
                        act = Some(Act::Meta(
                            "legend",
                            "Legend",
                            if legend_off {
                                None
                            } else {
                                Some("false".into())
                            },
                        ));
                    }
                    if chart.is_some() {
                        submenu(ui, if colors.is_some() { "✓  Colors" } else { "Colors" }, |ui| {
                            if check(ui, colors.is_none(), "Theme Colors") {
                                act = Some(Act::Meta("colors", "Colors", None));
                            }
                            for (name, set) in COLOR_SETS {
                                let on = colors.as_deref() == Some(set);
                                let row_top = ui.cursor().top();
                                if check(ui, on, name) {
                                    act = Some(Act::Meta("colors", "Colors", Some(set.to_string())));
                                }
                                // swatches drawn at the right end of the same row
                                let row = egui::Rect::from_min_max(
                                    pos2(ui.min_rect().left(), row_top),
                                    pos2(ui.min_rect().right(), row_top + MENU_ROW_H),
                                );
                                let hexes: Vec<&str> = set.split(", ").collect();
                                let mut x = row.right() - 10.0 - hexes.len() as f32 * 14.0;
                                for hexs in hexes {
                                    let h = u32::from_str_radix(hexs.trim_start_matches('#'), 16)
                                        .unwrap_or(0);
                                    let r = egui::Rect::from_min_size(
                                        pos2(x, row.center().y - 5.0),
                                        vec2(10.0, 10.0),
                                    );
                                    ui.painter().rect_filled(r, 2.0, crate::theme::hex(h));
                                    x += 14.0;
                                }
                            }
                        });
                    }
                    if has_axes && check(ui, false, "Axis Labels…") {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(axis_popup_id(), true);
                            d.insert_temp(axis_popup_id().with("focus"), true);
                        });
                    }
                    ui.separator();
                    if check(ui, false, "Reformat Table") {
                        act = Some(Act::Reformat);
                    }
                    if check(ui, false, "Transpose Rows and Columns") {
                        act = Some(Act::Transpose);
                    }
                    if check(ui, false, "Edit in Table Editor…") {
                        act = Some(Act::EditTable);
                    }
                    if check(ui, false, "Save Data as CSV…") {
                        act = Some(Act::SaveCsv);
                    }
                });
            });
        axis_labels_popup(ui.ctx(), bar, &block, &lines, has_axes, &mut act);
        let Some(a) = act else { return };
        let mut lines = lines;
        let mut block = block;
        let mut refocus = true;
        match a {
            Act::Chart(c) => block.set_chart(&mut lines, c),
            Act::Caption(on) => block.set(
                &mut lines,
                "[caption]",
                "",
                if on { Some("Caption") } else { None },
            ),
            Act::Meta(key, shown, v) => {
                refocus = key != "xlabel" && key != "ylabel";
                block.set(&mut lines, key, shown, v.as_deref());
            }
            Act::Reformat => block.reformat(&mut lines),
            Act::Transpose => block.transpose(&mut lines),
            Act::EditTable => {
                self.open_table_editor(block.start);
                return;
            }
            Act::SaveCsv => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .set_file_name("table.csv")
                    .save_file()
                {
                    self.status = match std::fs::write(&path, block.to_csv(&lines)) {
                        Ok(_) => format!(
                            "Saved {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        Err(e) => format!("Couldn't save: {e}"),
                    };
                }
                return;
            }
        }
        self.text = lines.join("\n");
        self.dirty = true;
        self.reparse();
        let total = self.text.chars().count();
        if refocus {
            // keep the cursor on the table's first line
            let ch = char_of_line(&lines, block.start);
            self.cursor = ch.min(total);
            self.sel_start = self.cursor;
            self.jump_to = Some((self.cursor, self.cursor));
        } else {
            self.cursor = self.cursor.min(total);
            self.sel_start = self.sel_start.min(total);
        }
    }
}
