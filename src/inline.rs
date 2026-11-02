//! Inline Markdown (bold, italic, code, highlight, strikethrough, links, math)
//! turned into styled text runs.

use crate::theme::family;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Stroke};

#[derive(Clone, Copy)]
pub struct Style {
    pub size: f32,
    pub color: Color32,
    pub face: crate::fonts::Face,
    pub bold: bool,
    pub italic: bool,
    pub accent: Color32,
    pub line_height: f32,
}

#[derive(Default, Clone, Copy)]
struct Flags {
    bold: bool,
    italic: bool,
    code: bool,
    mark: bool,
    strike: bool,
    link: bool,
    math: bool,
}

fn format(style: &Style, f: Flags) -> TextFormat {
    let font_id = if f.code {
        FontId::new(style.size * 0.9, FontFamily::Monospace)
    } else {
        FontId::new(
            style.size,
            family(
                if f.math {
                    crate::fonts::Face::SERIF
                } else {
                    style.face
                },
                style.bold || f.bold,
                style.italic ^ (f.italic || f.math),
            ),
        )
    };
    let mut tf = TextFormat {
        font_id,
        color: style.color,
        line_height: Some(style.size * style.line_height),
        valign: egui::Align::Center,
        ..Default::default()
    };
    if f.code {
        tf.background = style.color.gamma_multiply(0.12);
    }
    if f.mark {
        tf.background = Color32::from_rgba_unmultiplied(255, 230, 0, 150);
        if style.color.r() > 200 && style.color.g() > 200 {
            tf.color = Color32::BLACK;
        }
    }
    if f.strike {
        tf.strikethrough = Stroke::new((style.size * 0.06).max(1.0), style.color);
    }
    if f.link {
        tf.color = style.accent;
        tf.underline = Stroke::new((style.size * 0.05).max(1.0), style.accent);
    }
    tf
}

/// Appends `text` (with inline Markdown) to `job`.
pub fn append(job: &mut LayoutJob, text: &str, style: &Style) {
    let smart;
    let text = if crate::settings::smart_punct() {
        smart = crate::settings::smarten(text);
        &smart
    } else {
        text
    };
    let chars: Vec<char> = text.chars().collect();
    let mut flags = Flags::default();
    let mut buf = String::new();
    let flush = |job: &mut LayoutJob, buf: &mut String, flags: Flags| {
        if !buf.is_empty() {
            job.append(buf, 0.0, format(style, flags));
            buf.clear();
        }
    };
    let mut i = 0;
    let n = chars.len();
    let at = |i: usize, s: &str| -> bool {
        let sc: Vec<char> = s.chars().collect();
        i + sc.len() <= n && chars[i..i + sc.len()] == sc[..]
    };
    while i < n {
        let c = chars[i];
        if flags.code {
            if c == '`' {
                flush(job, &mut buf, flags);
                flags.code = false;
            } else {
                buf.push(c);
            }
            i += 1;
            continue;
        }
        if c == '\\' && i + 1 < n && "*_`=~[]$\\#".contains(chars[i + 1]) {
            buf.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '`' {
            flush(job, &mut buf, flags);
            flags.code = true;
            i += 1;
            continue;
        }
        if at(i, "**") || at(i, "__") {
            flush(job, &mut buf, flags);
            flags.bold = !flags.bold;
            i += 2;
            continue;
        }
        if at(i, "==") {
            flush(job, &mut buf, flags);
            flags.mark = !flags.mark;
            i += 2;
            continue;
        }
        if at(i, "~~") {
            flush(job, &mut buf, flags);
            flags.strike = !flags.strike;
            i += 2;
            continue;
        }
        let prev_alnum = i > 0 && chars[i - 1].is_alphanumeric();
        let next_alnum = i + 1 < n && chars[i + 1].is_alphanumeric();
        if c == '*' || (c == '_' && !(prev_alnum && next_alnum)) {
            if (flags.italic && i > 0 && !chars[i - 1].is_whitespace())
                || (!flags.italic && i + 1 < n && !chars[i + 1].is_whitespace())
            {
                flush(job, &mut buf, flags);
                flags.italic = !flags.italic;
                i += 1;
                continue;
            }
        }
        if c == '$' && (flags.math || (i + 1 < n && !chars[i + 1].is_whitespace())) {
            flush(job, &mut buf, flags);
            flags.math = !flags.math;
            i += 1;
            continue;
        }
        if c == '[' {
            // [text](url) → link text; [^note] footnotes are dropped.
            if let Some(close) = chars[i..].iter().position(|&ch| ch == ']') {
                let close = i + close;
                let inner: String = chars[i + 1..close].iter().collect();
                if inner.starts_with('^') {
                    i = close + 1;
                    continue;
                }
                if close + 1 < n && chars[close + 1] == '(' {
                    if let Some(end) = chars[close + 1..].iter().position(|&ch| ch == ')') {
                        flush(job, &mut buf, flags);
                        let mut lf = flags;
                        lf.link = true;
                        let mut b2 = String::new();
                        // inner text may itself carry emphasis; keep it simple.
                        b2.push_str(&inner.replace("**", "").replace('*', ""));
                        job.append(&b2, 0.0, format(style, lf));
                        i = close + 1 + end + 1;
                        continue;
                    }
                }
            }
        }
        if c == '^' {
            // superscript markers are shown without the caret
            i += 1;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    flush(job, &mut buf, flags);
}

pub fn job(text: &str, style: &Style, wrap: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    append(&mut job, text, style);
    job.wrap.max_width = wrap;
    job
}
