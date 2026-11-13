//! Small line icons drawn in the editor's left gap before option lines
//! (`title:`, `x:`, `Chart:`, `Colors:` …), in the style of iA Presenter.
//! They are painted shapes, so no icon font is needed.

use eframe::egui;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};

/// Option keys that get an icon in the editor, lower case.
pub const KEYS: &[&str] = &[
    "size",
    "scale",
    "x",
    "y",
    "background",
    "opacity",
    "filter",
    "title",
    "caption",
    "position",
    "chart",
    "type",
    "xlabel",
    "ylabel",
    "colors",
    "xformat",
    "legend",
    "color-type",
    "bar-type",
    "orientation",
];

/// The known option key at the start of `line`, if any.
pub fn key_of(line: &str) -> Option<&'static str> {
    let (k, _) = line.trim().split_once(':')?;
    let k = k.trim().to_ascii_lowercase();
    KEYS.iter().copied().find(|key| *key == k)
}

/// Draws the icon for `key` centred on `c`, `s` points tall.
pub fn paint(p: &Painter, key: &str, c: Pos2, s: f32, color: Color32) {
    let name = match key {
        "title" | "caption" => "note.text",
        "x" => "align.horizontal.left",
        "y" => "align.vertical.bottom",
        "position" => "square.grid.3x3.middle.filled",
        "size" => "arrow.up.left.and.arrow.down.right",
        "scale" => "arrow.up.left.and.arrow.down.right.square",
        "background" => "photo",
        "opacity" => "circle.lefthalf.filled",
        "filter" => "slider.horizontal.3",
        "chart" | "type" => "chart.bar.fill",
        "color-type" => "arrow.left.arrow.right",
        "bar-type" => "square.stack.3d.up.fill",
        "colors" => "paintpalette",
        "orientation" => "arrow.clockwise",
        "legend" => "list.bullet",
        "xlabel" => "arrow.left.and.right",
        "ylabel" => "arrow.up.and.down",
        "xformat" => "number",
        _ => "",
    };
    // Whole points, so every line shares one cached picture per symbol.
    if !name.is_empty() && crate::icons::paint(p, name, c, (s * 0.8).round(), color) {
        return;
    }
    let st = Stroke::new((s * 0.09).max(1.0), color);
    let u = s / 2.0; // half size
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Rect::from_min_max(
            pos2(c.x + x0 * u, c.y + y0 * u),
            pos2(c.x + x1 * u, c.y + y1 * u),
        )
    };
    let pt = |x: f32, y: f32| pos2(c.x + x * u, c.y + y * u);
    let line = |a: Pos2, b: Pos2| p.line_segment([a, b], st);
    let outline = |rect: Rect, round: f32| {
        p.rect_stroke(rect, round * u, st, egui::StrokeKind::Middle);
    };
    match key {
        // note / document
        "title" | "caption" => {
            outline(r(-0.7, -0.9, 0.7, 0.9), 0.15);
            line(pt(-0.35, -0.4), pt(0.35, -0.4));
            line(pt(-0.35, 0.0), pt(0.35, 0.0));
            line(pt(-0.35, 0.4), pt(0.1, 0.4));
        }
        // align left: bar on the left, two blocks
        "x" => {
            line(pt(-0.85, -0.9), pt(-0.85, 0.9));
            outline(r(-0.6, -0.65, 0.75, -0.1), 0.1);
            outline(r(-0.6, 0.1, 0.3, 0.65), 0.1);
        }
        // align top/bottom: bar along the bottom, two columns
        "y" => {
            line(pt(-0.9, 0.85), pt(0.9, 0.85));
            outline(r(-0.65, -0.75, -0.1, 0.6), 0.1);
            outline(r(0.1, -0.3, 0.65, 0.6), 0.1);
        }
        // grid position
        "position" => {
            outline(r(-0.85, -0.85, 0.85, 0.85), 0.1);
            p.circle_filled(c, u * 0.25, color);
            line(pt(0.0, -0.85), pt(0.0, -0.45));
            line(pt(0.0, 0.85), pt(0.0, 0.45));
            line(pt(-0.85, 0.0), pt(-0.45, 0.0));
            line(pt(0.85, 0.0), pt(0.45, 0.0));
        }
        // resize arrows
        "size" => {
            line(pt(-0.75, 0.75), pt(0.75, -0.75));
            line(pt(0.75, -0.75), pt(0.2, -0.75));
            line(pt(0.75, -0.75), pt(0.75, -0.2));
            line(pt(-0.75, 0.75), pt(-0.2, 0.75));
            line(pt(-0.75, 0.75), pt(-0.75, 0.2));
        }
        // scale: a small box growing towards the frame's corner
        "scale" => {
            outline(r(-0.85, -0.85, 0.85, 0.85), 0.15);
            outline(r(-0.85, 0.0, 0.0, 0.85), 0.1);
            line(pt(0.1, -0.1), pt(0.5, -0.5));
            line(pt(0.5, -0.5), pt(0.15, -0.5));
            line(pt(0.5, -0.5), pt(0.5, -0.15));
        }
        // picture: frame, sun, hill
        "background" => {
            outline(r(-0.9, -0.75, 0.9, 0.75), 0.15);
            p.circle_filled(pt(0.35, -0.3), u * 0.17, color);
            p.add(egui::Shape::line(
                vec![pt(-0.9, 0.55), pt(-0.3, -0.05), pt(0.3, 0.55)],
                st,
            ));
        }
        // half-filled circle
        "opacity" => {
            p.circle_stroke(c, u * 0.85, st);
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 16.0;
                    pos2(c.x + a.cos() * u * 0.85, c.y - a.sin() * u * 0.85)
                })
                .collect();
            p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        // sliders
        "filter" => {
            for (y, k) in [(-0.55, -0.3), (0.0, 0.4), (0.55, -0.1)] {
                line(pt(-0.9, y), pt(0.9, y));
                p.circle_filled(pt(k, y), u * 0.2, color);
            }
        }
        // bar chart
        "chart" | "type" => {
            let w = 0.42;
            for (x, h) in [(-0.75, 1.5), (-0.2, 1.0), (0.35, 0.65)] {
                p.rect_filled(r(x, 0.8 - h, x + w, 0.8), 0.08 * u, color);
            }
        }
        // two opposed arrows
        "color-type" => {
            line(pt(-0.85, -0.35), pt(0.85, -0.35));
            line(pt(0.85, -0.35), pt(0.45, -0.75));
            line(pt(0.85, 0.35), pt(-0.85, 0.35));
            line(pt(-0.85, 0.35), pt(-0.45, 0.75));
        }
        // stacked layers
        "bar-type" => {
            let diamond = |dy: f32| {
                vec![
                    pt(0.0, -0.45 + dy),
                    pt(0.85, 0.0 + dy),
                    pt(0.0, 0.45 + dy),
                    pt(-0.85, 0.0 + dy),
                ]
            };
            p.add(egui::Shape::convex_polygon(
                diamond(0.35),
                color.gamma_multiply(0.6),
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                diamond(-0.2),
                color,
                Stroke::NONE,
            ));
        }
        // palette
        "colors" => {
            p.circle_stroke(c, u * 0.88, st);
            for (x, y) in [(-0.35, -0.35), (0.2, -0.45), (0.5, 0.0), (-0.45, 0.2)] {
                p.circle_filled(pt(x, y), u * 0.13, color);
            }
        }
        // rotate
        "orientation" => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = -0.6 + 4.6 * i as f32 / 20.0;
                    pos2(c.x + a.cos() * u * 0.75, c.y - a.sin() * u * 0.75)
                })
                .collect();
            let end = pts[0];
            p.add(egui::Shape::line(pts, st));
            line(end, end + vec2(-0.45 * u, 0.0));
            line(end, end + vec2(0.0, -0.45 * u));
        }
        // legend: swatches with lines
        "legend" => {
            for y in [-0.5, 0.0, 0.5] {
                p.rect_filled(r(-0.85, y - 0.15, -0.55, y + 0.15), 0.0, color);
                line(pt(-0.3, y), pt(0.85, y));
            }
        }
        // axis labels: an axis with a tick label
        "xlabel" => {
            line(pt(-0.85, 0.2), pt(0.85, 0.2));
            line(pt(-0.4, 0.55), pt(0.4, 0.55));
            line(pt(-0.85, -0.6), pt(-0.85, 0.2));
        }
        "ylabel" => {
            line(pt(-0.2, -0.85), pt(-0.2, 0.85));
            line(pt(-0.55, -0.4), pt(-0.55, 0.4));
            line(pt(-0.2, 0.85), pt(0.6, 0.85));
        }
        // number format: '#'
        "xformat" => {
            line(pt(-0.25, -0.85), pt(-0.4, 0.85));
            line(pt(0.4, -0.85), pt(0.25, 0.85));
            line(pt(-0.8, -0.3), pt(0.8, -0.3));
            line(pt(-0.8, 0.3), pt(0.8, 0.3));
        }
        _ => {
            p.circle_filled(c, u * 0.25, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_case_insensitively() {
        assert_eq!(key_of("Bar-Type: stacked"), Some("bar-type"));
        assert_eq!(key_of("  title: \"Placeholder\""), Some("title"));
        assert_eq!(key_of("Colors: accent 1, accent 2"), Some("colors"));
        assert_eq!(key_of("Hello: world"), None);
        assert_eq!(key_of("no colon"), None);
    }
}
