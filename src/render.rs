//! Lays out and paints one slide into a rectangle.
//! Layout picks: cover (first heading level 1), centred title (level 2 alone),
//! single picture full-bleed, split (2–3 groups side by side), grid (4+ groups).

use crate::chart::{draw_chart, ChartStyle};
use crate::images::{draw_image, ImageCache};
use crate::inline::{self, Style};
use crate::parse::{Align, Block, Slide, TableSpec};
use crate::theme::{family, Decor, Theme};
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Color32, FontId, Galley, Painter, Rect, Stroke};
use std::sync::Arc;

/// Body text size as a share of slide height.
pub const BASE_TEXT: f32 = 0.037;
/// Heading 1 cap.
pub const H1_EM: f32 = 2.9;
/// Heading 2 cap.
pub const H2_EM: f32 = 2.5;
/// Estimated.
pub const H3_EM: f32 = 1.55;
/// Estimated.
pub const H4_EM: f32 = 1.15;
/// Side padding as a share of slide width (estimated).
pub const PAD_X: f32 = 0.06;
/// Top/bottom padding as a share of slide height (estimated).
pub const PAD_Y: f32 = 0.08;
pub const ASPECT: f32 = 16.0 / 9.0;

/// A piece of slide text recorded instead of drawn (for editable
/// PowerPoint export): where it sits and how it is styled.
pub struct TextBox {
    /// Bounds in screen points.
    pub rect: Rect,
    pub job: Arc<LayoutJob>,
    /// Colour for sections that use the painter's fallback colour.
    pub fallback: Color32,
}

thread_local! {
    static CAPTURE: std::cell::RefCell<Option<Vec<TextBox>>> = const { std::cell::RefCell::new(None) };
}

/// From now on, slide text is recorded instead of drawn.
pub fn begin_text_capture() {
    CAPTURE.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

/// Stops recording and returns the text boxes drawn since `begin_text_capture`.
pub fn end_text_capture() -> Vec<TextBox> {
    CAPTURE.with(|c| c.borrow_mut().take().unwrap_or_default())
}

/// Draws a galley, or records it while a text capture is running.
fn put_galley(p: &Painter, pos: egui::Pos2, g: Arc<Galley>, fallback: Color32) {
    let recorded = CAPTURE.with(|c| match c.borrow_mut().as_mut() {
        Some(list) => {
            list.push(TextBox {
                rect: g.rect.translate(pos.to_vec2()),
                job: g.job.clone(),
                fallback,
            });
            true
        }
        None => false,
    });
    if !recorded {
        p.galley(pos, g, fallback);
    }
}

/// Like `Painter::text`, but goes through `put_galley`.
fn put_text(
    p: &Painter,
    pos: egui::Pos2,
    anchor: egui::Align2,
    text: &str,
    font: FontId,
    color: Color32,
) {
    let g = p.layout_no_wrap(text.to_string(), font, color);
    let r = anchor.anchor_size(pos, g.size());
    put_galley(p, r.min, g, color);
}

#[derive(Clone, Copy, PartialEq)]
enum Anchor {
    Top,
    Center,
}

struct Colors {
    bg: Color32,
    body: Color32,
    title: Color32,
}

pub struct Ctx<'a> {
    pub painter: &'a Painter,
    pub theme: &'a Theme,
    pub images: &'a mut ImageCache,
}

fn is_text_only(g: &[Block]) -> bool {
    g.iter().all(|b| !b.is_flexible())
}

fn single_image(g: &[Block]) -> bool {
    g.len() == 1 && matches!(g[0], Block::Image(_))
}

/// Draws slide `number` (1-based) with header, footer and slide number.
pub fn draw_slide(cx: &mut Ctx, rect: Rect, slide: &Slide, number: usize) {
    let fg = draw_content(cx, rect, slide);
    let theme = cx.theme;
    let painter = cx
        .painter
        .with_clip_rect(rect.intersect(cx.painter.clip_rect()));
    let small = rect.height() * BASE_TEXT * 0.5 * theme.size_scale;
    let font = FontId::new(small, family(theme.face, false, false));
    let col = fg.gamma_multiply(0.6);
    let pad = vec2(rect.width() * PAD_X, rect.height() * PAD_Y * 0.45);
    use egui::Align2;
    let (top, bottom) = (rect.top() + pad.y, rect.bottom() - pad.y);
    let (left, mid, right) = (rect.left() + pad.x, rect.center().x, rect.right() - pad.x);
    let places = [
        (pos2(left, top), Align2::LEFT_TOP),
        (pos2(mid, top), Align2::CENTER_TOP),
        (pos2(right, top), Align2::RIGHT_TOP),
        (pos2(left, bottom), Align2::LEFT_BOTTOM),
        (pos2(mid, bottom), Align2::CENTER_BOTTOM),
        (pos2(right, bottom), Align2::RIGHT_BOTTOM),
    ];
    for (slot, (pos, anchor)) in theme.slots.iter().zip(places) {
        if slot.kind == crate::theme::SlotKind::Logo {
            if !theme.logo.is_empty() {
                if let Some((texture, size)) = cx.images.get(painter.ctx(), &theme.logo, None) {
                    let height = rect.height() * theme.logo_size.clamp(0.02, 0.08);
                    let width =
                        (height * size[0] as f32 / size[1].max(1) as f32).min(rect.width() * 0.25);
                    let height = height.min(width * size[1] as f32 / size[0].max(1) as f32);
                    painter.image(
                        texture,
                        anchor.anchor_size(pos, vec2(width, height)),
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }
            continue;
        }
        let text = slot.resolve(number, &theme.date);
        if !text.is_empty() {
            put_text(&painter, pos, anchor, &text, font.clone(), col);
        }
    }
}

/// Paints the slide body; returns the text colour used.
fn draw_content(cx: &mut Ctx, rect: Rect, slide: &Slide) -> Color32 {
    let painter = cx
        .painter
        .with_clip_rect(rect.intersect(cx.painter.clip_rect()));
    let theme = cx.theme;
    let h = rect.height();
    let base = h * BASE_TEXT * theme.size_scale;
    let groups = &slide.groups;

    // Which layout?
    let first_heading = groups.first().and_then(|g| g.first()).and_then(|b| {
        if let Block::Heading { level, .. } = b {
            Some(*level)
        } else {
            None
        }
    });
    let one = groups.len() == 1;
    let cover = one && first_heading == Some(1);
    let title = one && first_heading == Some(2) && is_text_only(&groups[0]);
    let invert = (cover || title) && theme.invert_titles && slide.backgrounds.is_empty();
    // As in iA Presenter, "invert" shows the slide in the other appearance.
    let mut colors = if invert {
        Colors {
            bg: theme.inverse.bg,
            body: theme.inverse.body,
            title: theme.inverse.title,
        }
    } else {
        Colors {
            bg: theme.bg,
            body: theme.body,
            title: theme.title,
        }
    };

    painter.rect_filled(rect, 0.0, colors.bg);
    let full_bleed = one && groups.first().map(|g| single_image(g)).unwrap_or(false);
    if slide.backgrounds.is_empty() && !full_bleed {
        let kind = if cover {
            Kind::Cover
        } else if title {
            Kind::Title
        } else {
            Kind::Content
        };
        draw_decor(&painter, rect, theme, colors.bg, colors.body, kind);
    }
    for bgimg in &slide.backgrounds {
        draw_image(
            &painter,
            rect,
            bgimg,
            cx.images,
            &theme.accents,
            Some(false),
        );
    }
    if !slide.backgrounds.is_empty()
        && slide.backgrounds.iter().any(|b| b.opacity > 0.6)
        && !groups.is_empty()
    {
        // keep text readable over a strong picture
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(90));
        colors.body = Color32::WHITE;
        colors.title = Color32::WHITE;
    }
    if groups.is_empty() {
        return colors.body;
    }

    let pad = vec2(rect.width() * PAD_X, h * PAD_Y);
    let inner = rect.shrink2(pad);
    let mut sub = Ctx {
        painter: &painter,
        theme,
        images: cx.images,
    };

    if one {
        let g = &groups[0];
        if single_image(g) {
            if let Block::Image(img) = &g[0] {
                draw_image(&painter, rect, img, sub.images, &theme.accents, None);
            }
            return colors.body;
        }
        let (anchor, centered) = if cover {
            (Anchor::Center, false)
        } else if title {
            (Anchor::Center, true)
        } else {
            (Anchor::Top, false)
        };
        draw_group(&mut sub, inner, g, base, &colors, anchor, centered);
        return colors.body;
    }

    let n = groups.len();
    if n <= 3 {
        // Split: side by side. Pictures alone in a column fill it edge to edge.
        let w = rect.width() / n as f32;
        for (i, g) in groups.iter().enumerate() {
            let col = Rect::from_min_size(pos2(rect.left() + w * i as f32, rect.top()), vec2(w, h));
            if single_image(g) {
                if let Block::Image(img) = &g[0] {
                    draw_image(&painter, col, img, sub.images, &theme.accents, None);
                }
            } else {
                let mut c = col.shrink2(vec2(pad.x * 0.75, pad.y));
                if i == 0 {
                    c.min.x = rect.left() + pad.x;
                }
                if i == n - 1 {
                    c.max.x = rect.right() - pad.x;
                }
                let anchor = if is_text_only(g) {
                    Anchor::Center
                } else {
                    Anchor::Top
                };
                draw_group(&mut sub, c, g, base * 0.9, &colors, anchor, false);
            }
        }
        return colors.body;
    }

    // Grid
    let cols = if n == 4 { 2 } else { 3 };
    let rows = n.div_ceil(cols);
    let gap = pad.x * 0.5;
    let cw = (inner.width() - gap * (cols - 1) as f32) / cols as f32;
    let ch = (inner.height() - gap * (rows - 1) as f32) / rows as f32;
    for (i, g) in groups.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let cell = Rect::from_min_size(
            pos2(
                inner.left() + (cw + gap) * c as f32,
                inner.top() + (ch + gap) * r as f32,
            ),
            vec2(cw, ch),
        );
        if single_image(g) {
            if let Block::Image(img) = &g[0] {
                draw_image(&painter, cell, img, sub.images, &theme.accents, None);
            }
        } else {
            draw_group(&mut sub, cell, g, base * 0.8, &colors, Anchor::Top, false);
        }
    }
    colors.body
}

/// Code colours: GitHub-style light and dark sets, picked by the slide
/// background so code stays readable on every theme.
struct CodePalette {
    plain: Color32,
    keyword: Color32,
    literal: Color32,
    string: Color32,
    number: Color32,
    comment: Color32,
    function: Color32,
    kind: Color32,
    punct: Color32,
}

impl CodePalette {
    fn color(&self, t: crate::syntax::Tok) -> Color32 {
        use crate::syntax::Tok;
        match t {
            Tok::Plain => self.plain,
            Tok::Keyword => self.keyword,
            Tok::Literal => self.literal,
            Tok::String => self.string,
            Tok::Number => self.number,
            Tok::Comment => self.comment,
            Tok::Function => self.function,
            Tok::Type => self.kind,
            Tok::Punct => self.punct,
        }
    }
}

fn code_palette(bg: Color32, body: Color32) -> CodePalette {
    let h = |v: u32| crate::theme::hex(v);
    if is_dark(bg) {
        CodePalette {
            plain: body,
            keyword: h(0xFF7B72),
            literal: h(0x79C0FF),
            string: h(0xA5D6FF),
            number: h(0x79C0FF),
            comment: h(0x8B949E),
            function: h(0xD2A8FF),
            kind: h(0xFFA657),
            punct: body.gamma_multiply(0.75),
        }
    } else {
        CodePalette {
            plain: body,
            keyword: h(0xCF222E),
            literal: h(0x0550AE),
            string: h(0x0A3069),
            number: h(0x0550AE),
            comment: h(0x6E7781),
            function: h(0x8250DF),
            kind: h(0x953800),
            punct: body.gamma_multiply(0.75),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Cover,
    Title,
    Content,
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn is_dark(c: Color32) -> bool {
    (c.r() as u32 * 299 + c.g() as u32 * 587 + c.b() as u32 * 114) / 1000 < 128
}

fn vgradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

/// A soft radial glow: `color` at the centre fading to nothing at `radius`.
fn glow(p: &Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(center, color);
    let n = 64;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(
            center + vec2(a.cos(), a.sin()) * radius,
            Color32::TRANSPARENT,
        );
    }
    for k in 0..n {
        m.add_triangle(0, 1 + k as u32, 1 + ((k + 1) % n) as u32);
    }
    p.add(m);
}

/// Theme decorations. They stay in the margins (or are faint and behind the
/// text) so the text layout is the same for every theme.
fn draw_decor(p: &Painter, rect: Rect, theme: &Theme, bg: Color32, body: Color32, kind: Kind) {
    let (w, h) = (rect.width(), rect.height());
    let accent = theme.accents[0];
    let second = theme.accents[1];
    let dark = is_dark(bg);
    let pad_x = w * PAD_X;
    match theme.decor {
        Decor::None => {}
        Decor::Bar => match kind {
            Kind::Cover => {
                p.rect_filled(
                    Rect::from_min_size(rect.min, vec2(w * 0.014, h)),
                    0.0,
                    accent,
                );
                p.rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left() + pad_x, rect.bottom() - h * 0.052),
                        vec2(w * 0.06, h * 0.011),
                    ),
                    h * 0.006,
                    accent,
                );
            }
            Kind::Title => {
                p.rect_filled(
                    Rect::from_center_size(
                        pos2(rect.center().x, rect.bottom() - h * 0.12),
                        vec2(w * 0.06, h * 0.011),
                    ),
                    h * 0.006,
                    accent,
                );
            }
            Kind::Content => {
                p.rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left() + pad_x, rect.top() + h * 0.042),
                        vec2(w * 0.04, h * 0.009),
                    ),
                    h * 0.005,
                    accent,
                );
            }
        },
        Decor::Glow => {
            let bottom = mix(bg, accent, if dark { 0.14 } else { 0.08 });
            vgradient(p, rect, bg, bottom);
            let strength = if dark { 0.30 } else { 0.16 };
            match kind {
                Kind::Cover | Kind::Title => {
                    glow(
                        p,
                        pos2(rect.right() - w * 0.08, rect.top() + h * 0.12),
                        w * 0.55,
                        accent.gamma_multiply(strength),
                    );
                    glow(
                        p,
                        pos2(rect.left() + w * 0.05, rect.bottom() - h * 0.02),
                        w * 0.4,
                        second.gamma_multiply(strength * 0.7),
                    );
                }
                Kind::Content => glow(
                    p,
                    pos2(rect.right() - w * 0.02, rect.top()),
                    w * 0.35,
                    accent.gamma_multiply(strength * 0.6),
                ),
            }
        }
        Decor::Frame => {
            let inset = rect.shrink(h * 0.03);
            p.rect_stroke(
                inset,
                0.0,
                Stroke::new((h * 0.0022).max(0.6), body.gamma_multiply(0.28)),
                egui::StrokeKind::Inside,
            );
            let rule = vec2(w * 0.035, (h * 0.006).max(1.0));
            match kind {
                Kind::Title => p.rect_filled(
                    Rect::from_center_size(pos2(rect.center().x, rect.bottom() - h * 0.12), rule),
                    0.0,
                    accent,
                ),
                _ => p.rect_filled(
                    Rect::from_min_size(pos2(rect.left() + pad_x, rect.top() + h * 0.052), rule),
                    0.0,
                    accent,
                ),
            };
        }
        Decor::Band => {
            let band = if kind == Kind::Content { 0.016 } else { 0.03 };
            p.rect_filled(
                Rect::from_min_size(rect.min, vec2(w, h * band)),
                0.0,
                accent,
            );
            if kind != Kind::Content {
                p.rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left(), rect.top() + h * band),
                        vec2(w * 0.3, h * 0.012),
                    ),
                    0.0,
                    theme.title,
                );
                p.rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left(), rect.bottom() - h * 0.012),
                        vec2(w, h * 0.012),
                    ),
                    0.0,
                    accent.gamma_multiply(0.35),
                );
            }
        }
        Decor::Shapes => {
            let bottom = mix(bg, second, if dark { 0.14 } else { 0.10 });
            vgradient(p, rect, bg, bottom);
            let a = if dark { 0.30 } else { 0.20 };
            match kind {
                Kind::Cover | Kind::Title => {
                    p.circle_filled(
                        pos2(rect.right() - w * 0.02, rect.top() - h * 0.06),
                        h * 0.5,
                        accent.gamma_multiply(a),
                    );
                    p.circle_filled(
                        pos2(rect.right() - w * 0.17, rect.top() + h * 0.08),
                        h * 0.16,
                        second.gamma_multiply(a),
                    );
                    p.circle_filled(
                        pos2(rect.left() - w * 0.03, rect.bottom() + h * 0.1),
                        h * 0.3,
                        second.gamma_multiply(a * 0.7),
                    );
                }
                Kind::Content => {
                    p.circle_filled(
                        pos2(rect.right() + w * 0.01, rect.top() - h * 0.05),
                        h * 0.2,
                        accent.gamma_multiply(a * 0.8),
                    );
                    p.circle_filled(
                        pos2(rect.right() - w * 0.1, rect.top() - h * 0.01),
                        h * 0.06,
                        second.gamma_multiply(a),
                    );
                }
            }
        }
    }
}

enum Item<'a> {
    Text {
        galley: Arc<Galley>,
        after: f32,
        quote: bool,
        code: bool,
    },
    Flex(&'a Block),
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => H1_EM,
        2 => H2_EM,
        3 => H3_EM,
        4 => H4_EM,
        _ => 1.0,
    }
}

fn text_galley(
    painter: &Painter,
    b: &Block,
    width: f32,
    base: f32,
    colors: &Colors,
    theme: &Theme,
    centered: bool,
) -> Option<(Arc<Galley>, f32, bool, bool)> {
    let style = |size: f32, color: Color32, bold: bool, italic: bool| Style {
        size,
        color,
        face: theme.face,
        bold,
        italic,
        accent: theme.accents[0],
        line_height: 1.3,
    };
    let mut quote = false;
    let mut code = false;
    let (mut job, after) = match b {
        Block::Heading { level, text } => {
            let size = base * heading_size(*level);
            let mut st = style(
                size,
                if *level >= 4 {
                    colors.body
                } else {
                    colors.title
                },
                *level <= 3,
                false,
            );
            st.face = theme.title_face;
            st.line_height = 1.12;
            (inline::job(text, &st, width), base * 0.45)
        }
        Block::Paragraph(t) => (
            inline::job(t, &style(base, colors.body, false, false), width),
            base * 0.6,
        ),
        Block::Quote(t) => {
            quote = true;
            (
                inline::job(
                    t,
                    &style(base * 1.1, colors.body, false, true),
                    width - base * 1.2,
                ),
                base * 0.6,
            )
        }
        Block::List(items) => {
            let mut job = LayoutJob::default();
            let st = style(base, colors.body, false, false);
            let mut counter = 0;
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    job.append(
                        "\n",
                        0.0,
                        TextFormat {
                            font_id: FontId::new(base, family(theme.face, false, false)),
                            line_height: Some(base * 1.55),
                            ..Default::default()
                        },
                    );
                }
                let marker = match (it.task, &it.number) {
                    (Some(true), _) => "●  ".to_string(),
                    (Some(false), _) => "○  ".to_string(),
                    (None, Some(_)) => {
                        counter += 1;
                        format!("{counter}.  ")
                    }
                    (None, None) => "•  ".to_string(),
                };
                let indent = "    ".repeat(it.indent.min(4));
                job.append(
                    &format!("{indent}{marker}"),
                    0.0,
                    TextFormat {
                        font_id: FontId::new(base, family(theme.face, false, false)),
                        color: theme.accents[0],
                        line_height: Some(base * 1.3),
                        ..Default::default()
                    },
                );
                inline::append(&mut job, &it.text, &st);
            }
            job.wrap.max_width = width;
            (job, base * 0.6)
        }
        Block::Code(t, lang) => {
            code = true;
            let mut job = LayoutJob::default();
            let pal = code_palette(colors.bg, colors.body);
            let size = base * 0.75;
            for (run, tok) in crate::syntax::highlight(t, lang) {
                let bold = matches!(tok, crate::syntax::Tok::Keyword);
                let italic = matches!(tok, crate::syntax::Tok::Comment);
                job.append(
                    &run,
                    0.0,
                    TextFormat {
                        font_id: FontId::new(size, crate::theme::mono(bold)),
                        color: pal.color(tok),
                        italics: italic,
                        line_height: Some(base * 1.05),
                        ..Default::default()
                    },
                );
            }
            job.wrap.max_width = width - base;
            (job, base * 0.8)
        }
        Block::Math(t) => {
            let mut st = style(base * 1.2, colors.body, false, true);
            st.face = crate::fonts::Face::SERIF;
            let mut job = inline::job(t, &st, width);
            job.halign = egui::Align::Center;
            (job, base * 0.6)
        }
        _ => return None,
    };
    if centered {
        job.halign = egui::Align::Center;
    }
    Some((painter.layout_job(job), after, quote, code))
}

fn draw_group(
    cx: &mut Ctx,
    rect: Rect,
    blocks: &[Block],
    base: f32,
    colors: &Colors,
    anchor: Anchor,
    centered: bool,
) {
    let painter = cx.painter;
    let theme = cx.theme;
    let flex_count = blocks.iter().filter(|b| b.is_flexible()).count();
    let min_flex = rect.height() * 0.3;
    let mut scale = 1.0;
    let mut items: Vec<Item>;
    let mut text_total;
    loop {
        items = Vec::new();
        text_total = 0.0;
        for b in blocks {
            if b.is_flexible() {
                items.push(Item::Flex(b));
            } else if let Some((g, after, quote, code)) = text_galley(
                painter,
                b,
                rect.width(),
                base * scale,
                colors,
                theme,
                centered,
            ) {
                let extra = if code { base * scale } else { 0.0 };
                text_total += g.size().y + after + extra;
                items.push(Item::Text {
                    galley: g,
                    after,
                    quote,
                    code,
                });
            }
        }
        if let Some(Item::Text { after, .. }) = items.last() {
            text_total -= after;
        }
        let need = text_total
            + if flex_count > 0 {
                min_flex * flex_count as f32
            } else {
                0.0
            };
        if !crate::settings::fit_text() || need <= rect.height() || scale < 0.35 {
            break;
        }
        scale *= 0.88;
    }
    let gap = base * scale * 0.6;
    let flex_h = if flex_count > 0 {
        ((rect.height() - text_total - gap * flex_count as f32) / flex_count as f32)
            .max(min_flex * 0.5)
    } else {
        0.0
    };
    let content_h = text_total + (flex_h + gap) * flex_count as f32;
    let mut y = match anchor {
        _ if flex_count > 0 => rect.top(),
        Anchor::Top => rect.top(),
        Anchor::Center => rect.center().y - content_h / 2.0,
    };
    let x = if centered {
        rect.center().x
    } else {
        rect.left()
    };
    let n = items.len();
    for (i, it) in items.into_iter().enumerate() {
        match it {
            Item::Text {
                galley,
                after,
                quote,
                code,
            } => {
                let gh = galley.size().y;
                if code {
                    let r = Rect::from_min_size(
                        pos2(rect.left(), y),
                        vec2(rect.width(), gh + base * scale),
                    );
                    painter.rect_filled(r, base * 0.15, colors.body.gamma_multiply(0.07));
                    put_galley(
                        painter,
                        pos2(rect.left() + base * scale * 0.5, y + base * scale * 0.5),
                        galley,
                        colors.body,
                    );
                    y += gh + base * scale;
                } else if quote {
                    painter.rect_filled(
                        Rect::from_min_size(pos2(rect.left(), y), vec2(base * 0.15, gh)),
                        0.0,
                        theme.accents[0],
                    );
                    put_galley(
                        painter,
                        pos2(rect.left() + base * 1.2, y),
                        galley,
                        colors.body,
                    );
                    y += gh;
                } else {
                    let gx = if galley.job.halign == egui::Align::Center && !centered {
                        rect.center().x
                    } else {
                        x
                    };
                    put_galley(painter, pos2(gx, y), galley, colors.body);
                    y += gh;
                }
                if i + 1 < n {
                    y += after;
                }
            }
            Item::Flex(b) => {
                if i > 0 {
                    y += gap * 0.5;
                }
                let r = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), flex_h));
                match b {
                    Block::Image(img) => {
                        draw_image(painter, r, img, cx.images, &theme.accents, None);
                    }
                    Block::Table(t) => {
                        let used = draw_table(painter, r, t, base * scale, colors, theme);
                        let _ = used;
                    }
                    Block::Chart(c) => draw_chart(
                        painter,
                        r,
                        c,
                        &ChartStyle {
                            fg: colors.body,
                            bg: colors.bg,
                            accents: theme.accents,
                            series: theme.chart_colors.clone(),
                            base: base * scale.max(0.7),
                            face: theme.face,
                        },
                    ),
                    _ => {}
                }
                y += flex_h + gap * 0.5;
            }
        }
    }
}

/// Draws a table across the full width of `rect`; returns the height used.
/// Cells wrap onto more lines (and `<br>` breaks a line) when the text does
/// not fit; rows grow taller to hold them.
fn draw_table(
    painter: &Painter,
    rect: Rect,
    t: &TableSpec,
    base: f32,
    colors: &Colors,
    theme: &Theme,
) -> f32 {
    let ncols = t
        .header
        .len()
        .max(t.rows.iter().map(|r| r.len()).max().unwrap_or(0))
        .max(1);
    let nrows = t.rows.len() + 1;
    let cap_h = if t.caption.is_some() { base * 1.2 } else { 0.0 };
    let avail = rect.height() - cap_h;
    let start_fs = (base * 0.9).min(avail / nrows as f32 / 1.9).max(4.0);
    let font = |bold: bool, fs: f32| FontId::new(fs, family(theme.face, bold, false));
    let cell = |ri: usize, j: usize| -> &str {
        let cells = if ri == 0 { &t.header } else { &t.rows[ri - 1] };
        cells.get(j).map(|s| s.as_str()).unwrap_or("")
    };
    let cell_job = |s: &str, bold: bool, fs: f32, wrap: f32| {
        inline::job(
            s,
            &Style {
                size: fs,
                color: colors.body,
                face: theme.face,
                bold,
                italic: false,
                accent: theme.accents[0],
                line_height: 1.2,
            },
            wrap,
        )
    };
    // Column widths and row heights for a font size.
    let plan = |fs: f32| -> (f32, Vec<f32>, Vec<f32>, Vec<Vec<Option<Arc<Galley>>>>) {
        let cell_pad = fs * 0.6;
        let measure = |s: &str, bold: bool| {
            painter
                .layout_no_wrap(s.replace("**", ""), font(bold, fs), colors.body)
                .size()
                .x
        };
        // natural (one line per `<br>`) width of each column
        let mut natural = vec![0.0f32; ncols];
        for (j, w) in natural.iter_mut().enumerate() {
            for ri in 0..nrows {
                *w = w.max(measure(cell(ri, j), ri == 0));
            }
            *w += cell_pad * 2.0;
        }
        // longest single word per column (capped so one long word can't
        // starve the other columns)
        let cap = rect.width() / ncols as f32;
        let mut min_w = vec![0.0f32; ncols];
        for (j, w) in min_w.iter_mut().enumerate() {
            for ri in 0..nrows {
                for word in cell(ri, j).split_whitespace() {
                    *w = w.max(measure(word, ri == 0));
                }
            }
            *w = (*w + cell_pad * 2.0).min(cap).min(natural[j]);
        }
        let total: f32 = natural.iter().sum();
        let mut widths = natural.clone();
        if total <= rect.width() {
            let extra = (rect.width() - total) / ncols as f32;
            for w in widths.iter_mut() {
                *w += extra;
            }
        } else {
            // Narrow columns keep their natural width; the wide ones share
            // what is left in proportion to their natural width.
            let mut fixed = vec![false; ncols];
            loop {
                let fixed_w: f32 = (0..ncols).filter(|&j| fixed[j]).map(|j| natural[j]).sum();
                let flex_nat: f32 = (0..ncols).filter(|&j| !fixed[j]).map(|j| natural[j]).sum();
                let room = (rect.width() - fixed_w).max(0.0);
                let mut grew = false;
                for j in 0..ncols {
                    if !fixed[j] {
                        let share = room * natural[j] / flex_nat.max(1e-3);
                        if natural[j] <= share {
                            fixed[j] = true;
                            grew = true;
                        }
                    }
                }
                if !grew {
                    for j in 0..ncols {
                        widths[j] = if fixed[j] {
                            natural[j]
                        } else {
                            room * natural[j] / flex_nat.max(1e-3)
                        };
                    }
                    // Lift columns below their longest word, taking the room
                    // from columns that have spare width above theirs.
                    let short: f32 = (0..ncols).map(|j| (min_w[j] - widths[j]).max(0.0)).sum();
                    if short > 0.0 {
                        let spare: f32 = (0..ncols).map(|j| (widths[j] - min_w[j]).max(0.0)).sum();
                        let k = (short / spare.max(1e-3)).min(1.0);
                        for j in 0..ncols {
                            if widths[j] < min_w[j] {
                                widths[j] = min_w[j];
                            } else {
                                widths[j] -= (widths[j] - min_w[j]) * k;
                            }
                        }
                    }
                    break;
                }
            }
        }
        let mut galleys = Vec::with_capacity(nrows);
        let mut heights = Vec::with_capacity(nrows);
        for ri in 0..nrows {
            let mut row = Vec::with_capacity(ncols);
            let mut row_h = fs * 1.9;
            for (j, w) in widths.iter().enumerate() {
                let s = cell(ri, j);
                if s.is_empty() {
                    row.push(None);
                    continue;
                }
                let g = painter.layout_job(cell_job(s, ri == 0, fs, (w - cell_pad * 2.0).max(1.0)));
                // a one-line cell keeps the old row height
                row_h = row_h.max(g.size().y + fs * 0.7);
                row.push(Some(g));
            }
            galleys.push(row);
            heights.push(row_h);
        }
        (cell_pad, widths, heights, galleys)
    };
    let mut fs = start_fs;
    let (mut cell_pad, mut widths, mut heights, mut galleys) = plan(fs);
    // Shrink the text a little while the wrapped table is too tall.
    for _ in 0..8 {
        if heights.iter().sum::<f32>() <= avail || fs <= (start_fs * 0.6).max(4.0) {
            break;
        }
        fs = (fs * 0.9).max((start_fs * 0.6).max(4.0));
        (cell_pad, widths, heights, galleys) = plan(fs);
    }
    let line = colors.body.gamma_multiply(0.2);
    let mut y = rect.top();
    for (ri, (row, row_h)) in galleys.into_iter().zip(heights).enumerate() {
        let mut x = rect.left();
        for (j, (g, w)) in row.into_iter().zip(&widths).enumerate() {
            if let Some(g) = g {
                let gw = g.size().x;
                let align = t.aligns.get(j).copied().unwrap_or(Align::Left);
                let gx = match align {
                    Align::Left => x + cell_pad,
                    Align::Center => x + (w - gw) / 2.0,
                    Align::Right => x + w - cell_pad - gw,
                };
                painter.galley(pos2(gx, y + (row_h - g.size().y) / 2.0), g, colors.body);
            }
            x += w;
        }
        y += row_h;
        let stroke = if ri == 0 {
            Stroke::new((fs * 0.08).max(1.0), colors.body)
        } else {
            Stroke::new(1.0_f32, line)
        };
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], stroke);
    }
    if let Some(c) = &t.caption {
        painter.text(
            pos2(rect.left(), y + base * 0.3),
            egui::Align2::LEFT_TOP,
            c,
            FontId::new(base * 0.6, family(theme.face, false, true)),
            colors.body.gamma_multiply(0.7),
        );
        y += cap_h;
    }
    y - rect.top()
}
