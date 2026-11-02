//! Charts drawn from a Markdown table: first column = categories,
//! other columns = series. Chart types include line, spline, area,
//! gauge, radar, bubble, bar, pie, donut (+ step, horizontal, stacked).

use crate::parse::{ChartKind, ChartSpec};
use crate::theme::family;
use egui::epaint::TextShape;
use egui::{pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke};
use std::f32::consts::{PI, TAU};

pub struct ChartStyle {
    pub fg: Color32,
    pub accents: [Color32; 6],
    pub base: f32,
    pub face: crate::fonts::Face,
}

struct Data {
    cats: Vec<String>,
    series: Vec<(String, Vec<f64>)>,
}

fn num(s: &str) -> f64 {
    let t: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    t.parse().unwrap_or(0.0)
}

fn data(spec: &ChartSpec) -> Data {
    let t = &spec.table;
    let cats = t
        .rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();
    let ncols = t
        .header
        .len()
        .max(t.rows.iter().map(|r| r.len()).max().unwrap_or(0));
    let mut series = Vec::new();
    for j in 1..ncols {
        let name = t
            .header
            .get(j)
            .cloned()
            .unwrap_or_else(|| format!("Series {j}"));
        let vals = t
            .rows
            .iter()
            .map(|r| r.get(j).map(|s| num(s)).unwrap_or(0.0))
            .collect();
        series.push((name, vals));
    }
    Data { cats, series }
}

fn nice_step(range: f64, target: f64) -> f64 {
    let raw = (range / target).max(1e-9);
    let mag = 10f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let n = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    n * mag
}

fn fmt(v: f64, step: f64) -> String {
    if step.fract().abs() < 1e-9 && v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else if step >= 0.1 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

impl ChartStyle {
    fn font(&self, k: f32) -> FontId {
        FontId::new(self.base * k, family(self.face, false, false))
    }
}

fn color_for(spec: &ChartSpec, st: &ChartStyle, i: usize) -> Color32 {
    if !spec.colors.is_empty() {
        let c = spec.colors[i % spec.colors.len()];
        return Color32::from_rgb(c[0], c[1], c[2]);
    }
    st.accents[i % 6]
}

fn text(p: &Painter, pos: Pos2, align: Align2, s: &str, font: FontId, color: Color32) -> Rect {
    p.text(pos, align, s, font, color)
}

/// Legend row; returns its height.
fn legend(p: &Painter, rect: Rect, names: &[String], spec: &ChartSpec, st: &ChartStyle) -> f32 {
    let font = st.font(0.55);
    let sw = st.base * 0.45;
    let gap = st.base * 0.9;
    let mut items = Vec::new();
    let mut total = 0.0;
    for n in names {
        let g = p.layout_no_wrap(n.clone(), font.clone(), st.fg);
        total += sw + st.base * 0.25 + g.size().x + gap;
        items.push(g);
    }
    let h = st.base * 0.8;
    let mut x = rect.center().x - (total - gap) / 2.0;
    x = x.max(rect.left());
    let y = rect.top() + h / 2.0;
    for (i, g) in items.into_iter().enumerate() {
        p.rect_filled(
            Rect::from_center_size(pos2(x + sw / 2.0, y), vec2(sw, sw)),
            sw * 0.2,
            color_for(spec, st, i),
        );
        x += sw + st.base * 0.25;
        let w = g.size().x;
        p.galley(pos2(x, y - g.size().y / 2.0), g, st.fg);
        x += w + gap;
    }
    h + st.base * 0.4
}

pub fn draw_chart(p: &Painter, rect: Rect, spec: &ChartSpec, st: &ChartStyle) {
    let d = data(spec);
    let mut area = rect;
    if let Some(t) = &spec.title {
        let r = text(
            p,
            pos2(area.center().x, area.top()),
            Align2::CENTER_TOP,
            t,
            FontId::new(st.base * 0.8, family(st.face, true, false)),
            st.fg,
        );
        area.min.y = r.bottom() + st.base * 0.3;
    }
    if let Some(c) = &spec.table.caption {
        let r = text(
            p,
            pos2(area.center().x, area.bottom()),
            Align2::CENTER_BOTTOM,
            c,
            FontId::new(st.base * 0.55, family(st.face, false, true)),
            st.fg.gamma_multiply(0.7),
        );
        area.max.y = r.top() - st.base * 0.3;
    }
    if d.cats.is_empty() || d.series.is_empty() {
        text(
            p,
            area.center(),
            Align2::CENTER_CENTER,
            "No chart data",
            st.font(0.7),
            st.fg.gamma_multiply(0.5),
        );
        return;
    }
    match spec.kind {
        ChartKind::Pie | ChartKind::Donut => pie(p, area, spec, &d, st),
        ChartKind::Radar => radar(p, area, spec, &d, st),
        ChartKind::Gauge => gauge(p, area, spec, &d, st),
        _ => xy(p, area, spec, &d, st),
    }
}

fn pie(p: &Painter, mut area: Rect, spec: &ChartSpec, d: &Data, st: &ChartStyle) {
    if spec.legend {
        let h = legend(p, area, &d.cats, spec, st);
        area.min.y += h;
    }
    let vals: Vec<f64> = d.series[0].1.iter().map(|v| v.max(0.0)).collect();
    let total: f64 = vals.iter().sum();
    if total <= 0.0 {
        return;
    }
    let r = area.width().min(area.height()) / 2.0 * 0.95;
    let c = area.center();
    let inner = if spec.kind == ChartKind::Donut {
        r * 0.55
    } else {
        0.0
    };
    let mut a0 = -PI / 2.0;
    for (i, v) in vals.iter().enumerate() {
        let sweep = (*v / total) as f32 * TAU;
        let col = color_for(spec, st, i);
        let steps = ((sweep / TAU) * 120.0).ceil().max(2.0) as usize;
        let mut mesh = Mesh::default();
        for k in 0..=steps {
            let a = a0 + sweep * k as f32 / steps as f32;
            let dir = vec2(a.cos(), a.sin());
            mesh.colored_vertex(c + dir * r, col);
            mesh.colored_vertex(c + dir * inner, col);
        }
        for k in 0..steps as u32 {
            let b = k * 2;
            mesh.add_triangle(b, b + 1, b + 2);
            mesh.add_triangle(b + 1, b + 3, b + 2);
        }
        p.add(mesh);
        // separator line
        let dir0 = vec2(a0.cos(), a0.sin());
        p.line_segment(
            [c + dir0 * inner, c + dir0 * r],
            Stroke::new((r * 0.012).max(1.0), Color32::WHITE),
        );
        if sweep > 0.25 {
            let mid = a0 + sweep / 2.0;
            let lr = (r + inner) / 2.0 + if inner == 0.0 { r * 0.12 } else { 0.0 };
            let pct = (*v / total * 100.0).round();
            let lum = col.r() as f32 * 0.299 + col.g() as f32 * 0.587 + col.b() as f32 * 0.114;
            let tc = if lum > 150.0 {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            text(
                p,
                c + vec2(mid.cos(), mid.sin()) * lr,
                Align2::CENTER_CENTER,
                &format!("{pct}%"),
                st.font(0.6),
                tc,
            );
        }
        a0 += sweep;
    }
}

fn radar(p: &Painter, mut area: Rect, spec: &ChartSpec, d: &Data, st: &ChartStyle) {
    if spec.legend {
        let names: Vec<String> = d.series.iter().map(|s| s.0.clone()).collect();
        area.min.y += legend(p, area, &names, spec, st);
    }
    let n = d.cats.len().max(3);
    let maxv = d
        .series
        .iter()
        .flat_map(|s| s.1.iter())
        .cloned()
        .fold(0.0f64, f64::max)
        .max(1e-9);
    let step = nice_step(maxv, 4.0);
    let top = (maxv / step).ceil() * step;
    let r = area.width().min(area.height()) / 2.0 * 0.78;
    let c = area.center();
    let grid = st.fg.gamma_multiply(0.18);
    let ang = |i: usize| -PI / 2.0 + TAU * i as f32 / n as f32;
    let rings = (top / step).round() as usize;
    for k in 1..=rings {
        let rr = r * k as f32 / rings as f32;
        let pts: Vec<Pos2> = (0..n)
            .map(|i| c + vec2(ang(i).cos(), ang(i).sin()) * rr)
            .collect();
        p.add(Shape::closed_line(pts, Stroke::new(1.0_f32, grid)));
    }
    for (i, cat) in d.cats.iter().enumerate().take(n) {
        let dir = vec2(ang(i).cos(), ang(i).sin());
        p.line_segment([c, c + dir * r], Stroke::new(1.0_f32, grid));
        let align = if dir.x > 0.3 {
            Align2::LEFT_CENTER
        } else if dir.x < -0.3 {
            Align2::RIGHT_CENTER
        } else if dir.y < 0.0 {
            Align2::CENTER_BOTTOM
        } else {
            Align2::CENTER_TOP
        };
        text(
            p,
            c + dir * (r + st.base * 0.3),
            align,
            cat,
            st.font(0.55),
            st.fg,
        );
    }
    for (si, (_, vals)) in d.series.iter().enumerate() {
        let col = color_for(spec, st, si);
        let pts: Vec<Pos2> = (0..n)
            .map(|i| {
                let v = vals.get(i).cloned().unwrap_or(0.0).max(0.0);
                c + vec2(ang(i).cos(), ang(i).sin()) * (r * (v / top) as f32)
            })
            .collect();
        let mut mesh = Mesh::default();
        mesh.colored_vertex(c, col.gamma_multiply(0.25));
        for pt in &pts {
            mesh.colored_vertex(*pt, col.gamma_multiply(0.25));
        }
        for i in 0..n as u32 {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n as u32);
        }
        p.add(mesh);
        p.add(Shape::closed_line(
            pts,
            Stroke::new((st.base * 0.08).max(1.5), col),
        ));
    }
}

fn gauge(p: &Painter, area: Rect, spec: &ChartSpec, d: &Data, st: &ChartStyle) {
    let v = d.series[0].1.first().cloned().unwrap_or(0.0);
    let max = if v <= 100.0 {
        100.0
    } else {
        nice_step(v, 1.0) * (v / nice_step(v, 1.0)).ceil()
    };
    let frac = (v / max).clamp(0.0, 1.0) as f32;
    let r = (area.width() / 2.0).min(area.height() * 0.8) * 0.9;
    let c = pos2(area.center().x, area.center().y + r * 0.35);
    let thick = r * 0.28;
    let arc = |from: f32, to: f32, col: Color32| {
        let steps = 90;
        let mut mesh = Mesh::default();
        for k in 0..=steps {
            let a = PI + (to - from) * PI * k as f32 / steps as f32 + from * PI;
            let dir = vec2(a.cos(), a.sin());
            mesh.colored_vertex(c + dir * r, col);
            mesh.colored_vertex(c + dir * (r - thick), col);
        }
        for k in 0..steps as u32 {
            let b = k * 2;
            mesh.add_triangle(b, b + 1, b + 2);
            mesh.add_triangle(b + 1, b + 3, b + 2);
        }
        p.add(mesh);
    };
    arc(0.0, 1.0, st.fg.gamma_multiply(0.12));
    arc(0.0, frac, color_for(spec, st, 0));
    let label = if max == 100.0 {
        format!("{}%", fmt(v, 1.0))
    } else {
        fmt(v, 1.0)
    };
    text(
        p,
        c,
        Align2::CENTER_BOTTOM,
        &label,
        FontId::new(r * 0.32, family(st.face, true, false)),
        st.fg,
    );
    let name = d.cats.first().cloned().unwrap_or_default();
    text(
        p,
        c + vec2(0.0, st.base * 0.2),
        Align2::CENTER_TOP,
        &name,
        st.font(0.6),
        st.fg.gamma_multiply(0.7),
    );
}

fn catmull(pts: &[Pos2]) -> Vec<Pos2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = Vec::new();
    for i in 0..pts.len() - 1 {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() {
            pts[i + 2]
        } else {
            pts[i + 1]
        };
        for k in 0..16 {
            let t = k as f32 / 16.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * ((2.0 * b)
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push(pos2(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out.push(*pts.last().unwrap());
    out
}

fn xy(p: &Painter, mut area: Rect, spec: &ChartSpec, d: &Data, st: &ChartStyle) {
    let ncat = d.cats.len();
    let nser = d.series.len();
    let stacked = spec.stacked && matches!(spec.kind, ChartKind::Bar | ChartKind::Area);
    let horizontal = spec.horizontal && spec.kind == ChartKind::Bar;
    if spec.legend && (nser > 1 || spec.kind != ChartKind::Bubble) {
        let names: Vec<String> = d.series.iter().map(|s| s.0.clone()).collect();
        let shown = if spec.kind == ChartKind::Bubble {
            names[..1].to_vec()
        } else {
            names
        };
        area.min.y += legend(p, area, &shown, spec, st);
    }
    // value range
    let value_series: Vec<&Vec<f64>> = if spec.kind == ChartKind::Bubble {
        vec![&d.series[0].1]
    } else {
        d.series.iter().map(|s| &s.1).collect()
    };
    let (mut lo, mut hi) = (0.0f64, f64::MIN);
    if stacked {
        for c in 0..ncat {
            let pos: f64 = value_series.iter().map(|s| s[c].max(0.0)).sum();
            let neg: f64 = value_series.iter().map(|s| s[c].min(0.0)).sum();
            hi = hi.max(pos);
            lo = lo.min(neg);
        }
    } else {
        for s in &value_series {
            for v in s.iter() {
                hi = hi.max(*v);
                lo = lo.min(*v);
            }
        }
    }
    if hi <= lo {
        hi = lo + 1.0;
    }
    let step = nice_step(hi - lo, 5.0);
    let vmax = (hi / step).ceil() * step;
    let vmin = (lo / step).floor() * step;

    let tick_font = st.font(0.5);
    let label_font = st.font(0.55);
    // margins
    let mut widest = 0.0f32;
    let mut v = vmin;
    while v <= vmax + step * 0.5 {
        let w = p
            .layout_no_wrap(fmt(v, step), tick_font.clone(), st.fg)
            .size()
            .x;
        widest = widest.max(w);
        v += step;
    }
    let cat_w = d
        .cats
        .iter()
        .map(|c| {
            p.layout_no_wrap(c.clone(), label_font.clone(), st.fg)
                .size()
                .x
        })
        .fold(0.0f32, f32::max);
    let mut plot = area;
    let ylabel_w = if spec.ylabel.is_some() {
        st.base * 0.9
    } else {
        0.0
    };
    let xlabel_h = if spec.xlabel.is_some() {
        st.base * 0.9
    } else {
        0.0
    };
    if horizontal {
        plot.min.x += cat_w + st.base * 0.4 + xlabel_h;
        plot.max.y -= st.base * 0.8 + ylabel_w;
    } else {
        plot.min.x += widest + st.base * 0.4 + ylabel_w;
        plot.max.y -= st.base * 0.8 + xlabel_h;
    }
    plot.min.y += st.base * 0.2;
    plot.max.x -= st.base * 0.3;
    if plot.width() < 10.0 || plot.height() < 10.0 {
        return;
    }
    let grid = st.fg.gamma_multiply(0.15);
    let axis = st.fg.gamma_multiply(0.5);
    let val_to = |v: f64| -> f32 {
        let t = ((v - vmin) / (vmax - vmin)) as f32;
        if horizontal {
            plot.left() + t * plot.width()
        } else {
            plot.bottom() - t * plot.height()
        }
    };
    // grid + ticks
    let mut v = vmin;
    while v <= vmax + step * 0.5 {
        let q = val_to(v);
        if horizontal {
            p.line_segment(
                [pos2(q, plot.top()), pos2(q, plot.bottom())],
                Stroke::new(1.0_f32, grid),
            );
            text(
                p,
                pos2(q, plot.bottom() + st.base * 0.2),
                Align2::CENTER_TOP,
                &fmt(v, step),
                tick_font.clone(),
                st.fg.gamma_multiply(0.7),
            );
        } else {
            p.line_segment(
                [pos2(plot.left(), q), pos2(plot.right(), q)],
                Stroke::new(1.0_f32, grid),
            );
            text(
                p,
                pos2(plot.left() - st.base * 0.2, q),
                Align2::RIGHT_CENTER,
                &fmt(v, step),
                tick_font.clone(),
                st.fg.gamma_multiply(0.7),
            );
        }
        v += step;
    }
    let zero = val_to(0.0f64.clamp(vmin, vmax));
    let band = if horizontal {
        plot.height()
    } else {
        plot.width()
    } / ncat as f32;
    let cat_center = |i: usize| -> f32 {
        if horizontal {
            plot.top() + band * (i as f32 + 0.5)
        } else {
            plot.left() + band * (i as f32 + 0.5)
        }
    };
    // category labels (skip some if crowded)
    let every = if horizontal {
        1
    } else {
        ((cat_w + st.base * 0.3) / band).ceil().max(1.0) as usize
    };
    for (i, c) in d.cats.iter().enumerate() {
        if i % every != 0 {
            continue;
        }
        if horizontal {
            text(
                p,
                pos2(plot.left() - st.base * 0.25, cat_center(i)),
                Align2::RIGHT_CENTER,
                c,
                label_font.clone(),
                st.fg,
            );
        } else {
            text(
                p,
                pos2(cat_center(i), plot.bottom() + st.base * 0.2),
                Align2::CENTER_TOP,
                c,
                label_font.clone(),
                st.fg,
            );
        }
    }
    // axis labels
    let (xl, yl) = if horizontal {
        (&spec.ylabel, &spec.xlabel)
    } else {
        (&spec.xlabel, &spec.ylabel)
    };
    if let Some(x) = xl {
        text(
            p,
            pos2(plot.center().x, area.bottom()),
            Align2::CENTER_BOTTOM,
            x,
            label_font.clone(),
            st.fg.gamma_multiply(0.8),
        );
    }
    if let Some(y) = yl {
        let g = p.layout_no_wrap(y.clone(), label_font.clone(), st.fg.gamma_multiply(0.8));
        let w = g.size().x;
        let pos = pos2(area.left(), plot.center().y + w / 2.0);
        p.add(TextShape::new(pos, g, st.fg).with_angle(-PI / 2.0));
    }

    match spec.kind {
        ChartKind::Bar => {
            let inner = band * 0.72;
            for c in 0..ncat {
                let mut acc_pos = 0.0;
                let mut acc_neg = 0.0;
                for (s, (_, vals)) in d.series.iter().enumerate() {
                    let v = vals[c];
                    let col = color_for(spec, st, s);
                    let (a, b, lo_off, w) = if stacked {
                        let (from, to) = if v >= 0.0 {
                            let f = acc_pos;
                            acc_pos += v;
                            (f, acc_pos)
                        } else {
                            let f = acc_neg;
                            acc_neg += v;
                            (f, acc_neg)
                        };
                        (val_to(from), val_to(to), -inner / 2.0, inner)
                    } else {
                        let w = inner / nser as f32;
                        (zero, val_to(v), -inner / 2.0 + w * s as f32, w)
                    };
                    let cc = cat_center(c) + lo_off;
                    let r = if horizontal {
                        Rect::from_two_pos(pos2(a, cc + w * 0.06), pos2(b, cc + w * 0.94))
                    } else {
                        Rect::from_two_pos(pos2(cc + w * 0.06, a), pos2(cc + w * 0.94, b))
                    };
                    p.rect_filled(r, 0.0, col);
                }
            }
        }
        ChartKind::Bubble => {
            let sizes = d.series.get(1).map(|s| &s.1);
            let smax = sizes
                .map(|s| s.iter().cloned().fold(0.0f64, f64::max))
                .unwrap_or(1.0)
                .max(1e-9);
            let rmax = band.min(plot.height() / 3.0) * 0.45;
            for c in 0..ncat {
                let v = d.series[0].1[c];
                let rr = match sizes {
                    Some(s) => (rmax * ((s[c].max(0.0) / smax) as f32).sqrt()).max(3.0),
                    None => rmax * 0.4,
                };
                let col = color_for(spec, st, c);
                p.circle_filled(pos2(cat_center(c), val_to(v)), rr, col.gamma_multiply(0.75));
            }
        }
        _ => {
            let mut base_line: Vec<f64> = vec![0.0; ncat];
            for (s, (_, vals)) in d.series.iter().enumerate() {
                let col = color_for(spec, st, s);
                let tops: Vec<f64> = if stacked {
                    (0..ncat).map(|c| base_line[c] + vals[c]).collect()
                } else {
                    vals.clone()
                };
                let mut pts: Vec<Pos2> = (0..ncat)
                    .map(|c| pos2(cat_center(c), val_to(tops[c])))
                    .collect();
                if spec.kind == ChartKind::Step {
                    let mut sp = Vec::new();
                    for (i, pt) in pts.iter().enumerate() {
                        if i > 0 {
                            sp.push(pos2(pt.x, sp.last().map(|q: &Pos2| q.y).unwrap_or(pt.y)));
                        }
                        sp.push(*pt);
                    }
                    pts = sp;
                } else if spec.kind == ChartKind::Spline {
                    pts = catmull(&pts);
                }
                if spec.kind == ChartKind::Area {
                    let bottoms: Vec<f32> = (0..ncat)
                        .map(|c| if stacked { val_to(base_line[c]) } else { zero })
                        .collect();
                    let mut mesh = Mesh::default();
                    let fill = col.gamma_multiply(0.35);
                    for (i, pt) in pts.iter().enumerate() {
                        mesh.colored_vertex(*pt, fill);
                        mesh.colored_vertex(pos2(pt.x, bottoms[i]), fill);
                    }
                    for i in 0..(pts.len().saturating_sub(1)) as u32 {
                        let b = i * 2;
                        mesh.add_triangle(b, b + 1, b + 2);
                        mesh.add_triangle(b + 1, b + 3, b + 2);
                    }
                    p.add(mesh);
                }
                p.add(Shape::line(
                    pts,
                    Stroke::new((st.base * 0.09).max(1.5), col),
                ));
                if matches!(spec.kind, ChartKind::Line | ChartKind::Area) {
                    for c in 0..ncat {
                        p.circle_filled(
                            pos2(cat_center(c), val_to(tops[c])),
                            (st.base * 0.12).max(2.0),
                            col,
                        );
                    }
                }
                if stacked {
                    base_line = tops;
                }
            }
        }
    }
    // baseline axis
    if horizontal {
        p.line_segment(
            [pos2(zero, plot.top()), pos2(zero, plot.bottom())],
            Stroke::new(1.0_f32, axis),
        );
    } else {
        p.line_segment(
            [pos2(plot.left(), zero), pos2(plot.right(), zero)],
            Stroke::new(1.0_f32, axis),
        );
    }
}
