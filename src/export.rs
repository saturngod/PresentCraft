//! Writing exported presentations. Slides arrive as pictures (each slide is
//! drawn full size and captured), then go into a PDF, a PowerPoint file,
//! a folder of PNG images or a self-contained web page with speaker notes.
//! For PowerPoint the picture is drawn without text, and the text is added
//! as native, editable text boxes on top.

use image::RgbaImage;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Pdf,
    PowerPoint,
    Images,
    Html,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Pdf => "PDF",
            Format::PowerPoint => "PowerPoint",
            Format::Images => "images",
            Format::Html => "web page",
        }
    }
}

pub struct Job {
    pub format: Format,
    pub dest: PathBuf,
    pub idx: usize,
    pub waiting: bool,
    pub frames: u32,
    pub images: Vec<RgbaImage>,
    /// PowerPoint only: each slide's text as native text-box XML.
    pub texts: Vec<String>,
    /// Text recorded for the slide currently being captured.
    pub pending_text: String,
    /// HTML and PowerPoint: each slide's videos.
    pub videos: Vec<Vec<ExportVideo>>,
    pub pending_videos: Vec<ExportVideo>,
    /// HTML: this slide has a background video, so its text is drawn as
    /// HTML above the video instead of into the picture.
    pub text_layer: bool,
}

/// A video on an exported slide. Position and size are fractions of the slide.
#[derive(Clone, Debug)]
pub struct ExportVideo {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// A local file, or a web address (HTML only).
    pub path: Option<PathBuf>,
    pub url: Option<String>,
    pub background: bool,
    pub contain: bool,
    pub align: [f32; 2],
    pub filter: Option<crate::parse::Filter>,
    pub opacity: f32,
}

impl ExportVideo {
    /// From a video drawn at `spot` on a slide drawn in `slide`.
    pub fn from_spot(
        spot: &crate::images::VideoSpot,
        slide: egui::Rect,
        resolve: impl Fn(&str) -> Option<PathBuf>,
    ) -> Option<ExportVideo> {
        let r = spot.rect.intersect(slide);
        if !r.is_positive() {
            return None;
        }
        let web = crate::images::is_web(&spot.src);
        let path = if web { None } else { resolve(&spot.src) };
        if path.is_none() && !web {
            return None;
        }
        Some(ExportVideo {
            x: (r.left() - slide.left()) / slide.width(),
            y: (r.top() - slide.top()) / slide.height(),
            w: r.width() / slide.width(),
            h: r.height() / slide.height(),
            path,
            url: web.then(|| spot.src.clone()),
            background: spot.background,
            contain: spot.contain,
            align: spot.align,
            filter: spot.filter,
            opacity: spot.opacity,
        })
    }
}

/// Encodes a relative URL path (spaces and other unsafe characters).
fn url_path(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn css_filter(f: Option<crate::parse::Filter>) -> &'static str {
    use crate::parse::Filter;
    match f {
        Some(Filter::Grayscale) => "grayscale(1)",
        Some(Filter::Sepia) => "sepia(1)",
        Some(Filter::Darken) => "brightness(.55)",
        Some(Filter::Lighten) => "brightness(1.3) contrast(.75)",
        Some(Filter::Blur) => "blur(1vmin)",
        None => "none",
    }
}

const SLIDE_W_EMU: f32 = 12_192_000.0;
const SLIDE_H_EMU: f32 = 6_858_000.0;
/// Slide height in points (7.5 in).
const SLIDE_H_PT: f32 = 540.0;

fn rgb_hex(c: egui::Color32) -> (String, u8) {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    (format!("{r:02X}{g:02X}{b:02X}"), a)
}

fn color_xml(c: egui::Color32) -> String {
    let (hex, a) = rgb_hex(c);
    if a >= 255 {
        format!("<a:srgbClr val=\"{hex}\"/>")
    } else {
        format!(
            "<a:srgbClr val=\"{hex}\"><a:alpha val=\"{}\"/></a:srgbClr>",
            (a as u32 * 100_000 / 255)
        )
    }
}

/// Font name, bold and italic for an egui font family used on slides.
fn face(f: &egui::FontFamily) -> (String, bool, bool) {
    match f {
        egui::FontFamily::Name(n) => {
            let n: &str = n;
            let bold = n.ends_with("-b") || n.ends_with("-bi");
            let italic = n.ends_with("-i") || n.ends_with("-bi");
            let base = n.split('-').next().unwrap_or(n);
            let name = match base {
                "mono" => "Consolas".to_string(),
                "serif" => "Georgia".to_string(),
                // A system family picked in the inspector ("f<n>").
                _ => match base.strip_prefix('f').and_then(|i| i.parse::<u16>().ok()) {
                    Some(i) => crate::fonts::Face(i + 2).name(),
                    None => "Arial".to_string(),
                },
            };
            (name, bold, italic)
        }
        egui::FontFamily::Monospace => ("Consolas".into(), false, false),
        egui::FontFamily::Proportional => ("Arial".into(), false, false),
    }
}

/// The CSS colour for an egui colour.
fn css_color(c: egui::Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a >= 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One slide's text as positioned HTML (used above background videos).
/// Sizes are in `cqh`, hundredths of the slide's height, so the text
/// scales with the slide.
pub fn html_text_layer(boxes: &[crate::render::TextBox], slide: egui::Rect) -> String {
    let h = slide.height().max(1.0);
    let w = slide.width().max(1.0);
    let cq = |v: f32| v / h * 100.0;
    let mut out = String::new();
    for b in boxes {
        let job = &b.job;
        if job.text.trim().is_empty() {
            continue;
        }
        let centered = job.halign == egui::Align::Center;
        let wrap = job.wrap.max_width;
        let (x, bw, nowrap) = if wrap.is_finite() && wrap > 0.0 {
            let bw = wrap.max(b.rect.width()) * 1.03;
            let x = if centered {
                b.rect.center().x - bw / 2.0
            } else {
                b.rect.left()
            };
            (x, bw, false)
        } else {
            (b.rect.left(), b.rect.width() * 1.1 + 2.0, true)
        };
        let line = job
            .sections
            .first()
            .and_then(|s| s.format.line_height)
            .map(|l| format!("line-height:{:.3}cqh;", cq(l)))
            .unwrap_or_default();
        out.push_str(&format!(
            "<div class=\"t\" style=\"left:{:.3}%;top:{:.3}%;width:{:.3}%;{}{}{}\">",
            (x - slide.left()) / w * 100.0,
            (b.rect.top() - slide.top()) / h * 100.0,
            bw / w * 100.0,
            line,
            if centered { "text-align:center;" } else { "" },
            if nowrap { "white-space:pre;" } else { "" },
        ));
        for sec in &job.sections {
            let f = &sec.format;
            let (font, bold, italic) = face(&f.font_id.family);
            let family = match font.as_str() {
                "Consolas" => "Menlo,Consolas,monospace".to_string(),
                "Georgia" => "Georgia,serif".to_string(),
                "Arial" => "Arial,Helvetica,sans-serif".to_string(),
                other => format!(
                    "'{}',Arial,Helvetica,sans-serif",
                    html_escape(other).replace(['\'', '"'], "")
                ),
            };
            let color = if f.color == egui::Color32::PLACEHOLDER {
                b.fallback
            } else {
                f.color
            };
            let mut deco = Vec::new();
            if f.strikethrough.width > 0.0 {
                deco.push("line-through");
            }
            if f.underline.width > 0.0 {
                deco.push("underline");
            }
            let bg = if f.background.to_srgba_unmultiplied()[3] > 0 {
                format!("background:{};", css_color(f.background))
            } else {
                String::new()
            };
            out.push_str(&format!(
                "<span style=\"font:{}{}{:.3}cqh {family};color:{};{}{}\">{}</span>",
                if italic || f.italics { "italic " } else { "" },
                if bold { "700 " } else { "" },
                cq(f.font_id.size),
                css_color(color),
                if deco.is_empty() {
                    String::new()
                } else {
                    format!("text-decoration:{};", deco.join(" "))
                },
                bg,
                html_escape(&job.text[sec.byte_range.clone()]).replace('\n', "<br>"),
            ));
        }
        out.push_str("</div>");
    }
    out
}

/// Native PowerPoint text boxes (`<p:sp>` elements) for one slide's text.
/// `slide` is the slide's rectangle in the same points as the boxes.
pub fn pptx_text_shapes(boxes: &[crate::render::TextBox], slide: egui::Rect) -> String {
    let emu = SLIDE_H_EMU / slide.height().max(1.0);
    let pt = SLIDE_H_PT / slide.height().max(1.0);
    let mut out = String::new();
    for (k, b) in boxes.iter().enumerate() {
        let job = &b.job;
        if job.text.trim().is_empty() {
            continue;
        }
        let centered = job.halign == egui::Align::Center;
        let wrap = job.wrap.max_width;
        // Give PowerPoint a little slack so lines break where the app breaks them.
        let (x, w, wrap_attr) = if wrap.is_finite() && wrap > 0.0 {
            let w = wrap.max(b.rect.width()) * 1.03;
            let x = if centered {
                b.rect.center().x - w / 2.0
            } else {
                b.rect.left()
            };
            (x, w, "square")
        } else {
            let w = b.rect.width() * 1.1 + 2.0;
            (b.rect.left(), w, "none")
        };
        let ox = ((x - slide.left()) * emu).round() as i64;
        let oy = ((b.rect.top() - slide.top()) * emu).round() as i64;
        let cx = (w * emu).round().max(1.0) as i64;
        let cy = (b.rect.height() * emu).round().max(1.0) as i64;
        let ox = ox.clamp(-(SLIDE_W_EMU as i64), SLIDE_W_EMU as i64);

        // Paragraphs: split sections on line breaks.
        let mut paras: Vec<Vec<(String, &egui::text::TextFormat)>> = vec![Vec::new()];
        for sec in &job.sections {
            let text = &job.text[sec.byte_range.clone()];
            let mut first = true;
            for part in text.split('\n') {
                if !first {
                    paras.push(Vec::new());
                }
                first = false;
                if !part.is_empty() {
                    paras
                        .last_mut()
                        .unwrap()
                        .push((part.to_string(), &sec.format));
                }
            }
        }
        let first_fmt = job.sections.first().map(|s| &s.format);
        let line_pt = first_fmt
            .and_then(|f| f.line_height)
            .map(|h| (h * pt * 100.0).round() as i64);
        let size_of =
            |f: &egui::text::TextFormat| ((f.font_id.size * pt * 100.0).round() as i64).max(100);
        let mut body = String::new();
        for runs in paras.iter_mut() {
            // PowerPoint collapses leading and repeated spaces; keep indents.
            let mut at_start = true;
            for (text, _) in runs.iter_mut() {
                let mut t = String::with_capacity(text.len());
                let mut prev_space = false;
                for ch in text.chars() {
                    match ch {
                        '\t' => t.push_str("\u{a0}\u{a0}\u{a0}\u{a0}"),
                        ' ' if at_start || prev_space => t.push('\u{a0}'),
                        c => t.push(c),
                    }
                    if ch != ' ' && ch != '\t' {
                        at_start = false;
                    }
                    prev_space = ch == ' ';
                }
                *text = t;
            }
            let runs = &*runs;
            body.push_str("<a:p><a:pPr");
            if centered {
                body.push_str(" algn=\"ctr\"");
            }
            body.push('>');
            if let Some(l) = line_pt {
                body.push_str(&format!("<a:lnSpc><a:spcPts val=\"{l}\"/></a:lnSpc>"));
            }
            body.push_str("<a:spcBef><a:spcPts val=\"0\"/></a:spcBef><a:spcAft><a:spcPts val=\"0\"/></a:spcAft></a:pPr>");
            for (text, f) in runs {
                let (font, bold, italic) = face(&f.font_id.family);
                let italic = italic || f.italics;
                let font = html_escape(&font).replace('"', "&quot;");
                let color = if f.color == egui::Color32::PLACEHOLDER {
                    b.fallback
                } else {
                    f.color
                };
                body.push_str(&format!(
                    "<a:r><a:rPr lang=\"en-US\" sz=\"{}\"{}{}{}{} dirty=\"0\"><a:solidFill>{}</a:solidFill>",
                    size_of(f),
                    if bold { " b=\"1\"" } else { "" },
                    if italic { " i=\"1\"" } else { "" },
                    if f.strikethrough.width > 0.0 { " strike=\"sngStrike\"" } else { "" },
                    if f.underline.width > 0.0 { " u=\"sng\"" } else { "" },
                    color_xml(color),
                ));
                // Solid highlights only; faint tints (inline code) would turn dark.
                if f.background.to_srgba_unmultiplied()[3] >= 160 {
                    body.push_str(&format!(
                        "<a:highlight>{}</a:highlight>",
                        color_xml(f.background.to_opaque())
                    ));
                }
                body.push_str(&format!(
                    "<a:latin typeface=\"{font}\"/><a:cs typeface=\"{font}\"/></a:rPr><a:t>{}</a:t></a:r>",
                    xml_escape(text)
                ));
            }
            let end_size = runs
                .first()
                .map(|(_, f)| size_of(f))
                .or_else(|| first_fmt.map(size_of))
                .unwrap_or(1800);
            body.push_str(&format!(
                "<a:endParaRPr lang=\"en-US\" sz=\"{end_size}\" dirty=\"0\"/></a:p>"
            ));
        }
        let id = 3 + k;
        out.push_str(&format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Text {id}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"{ox}\" y=\"{oy}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>\
<p:txBody><a:bodyPr wrap=\"{wrap_attr}\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"t\"><a:noAutofit/></a:bodyPr><a:lstStyle/>{body}</p:txBody></p:sp>"
        ));
    }
    out
}

fn jpeg_bytes(img: &RgbaImage) -> Vec<u8> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut buf = Vec::new();
    let _ = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 92).encode_image(&rgb);
    buf
}

/// One page per slide, 960 × 540 pt (16:9).
pub fn write_pdf(path: &Path, imgs: &[RgbaImage]) -> std::io::Result<()> {
    let n = imgs.len();
    let mut out: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let begin = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, num: usize| {
        offsets.push(out.len());
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
    };
    begin(&mut out, &mut offsets, 1);
    out.extend_from_slice(b"<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    begin(&mut out, &mut offsets, 2);
    let kids: Vec<String> = (0..n).map(|k| format!("{} 0 R", 3 + 3 * k)).collect();
    out.extend_from_slice(
        format!(
            "<< /Type /Pages /Kids [{}] /Count {n} >>\nendobj\n",
            kids.join(" ")
        )
        .as_bytes(),
    );
    for (k, img) in imgs.iter().enumerate() {
        let (page, content, image_obj) = (3 + 3 * k, 4 + 3 * k, 5 + 3 * k);
        begin(&mut out, &mut offsets, page);
        out.extend_from_slice(
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 960 540] /Resources << /XObject << /Im0 {image_obj} 0 R >> >> /Contents {content} 0 R >>\nendobj\n")
                .as_bytes(),
        );
        let stream = b"q 960 0 0 540 0 0 cm /Im0 Do Q";
        begin(&mut out, &mut offsets, content);
        out.extend_from_slice(format!("<< /Length {} >>\nstream\n", stream.len()).as_bytes());
        out.extend_from_slice(stream);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        let jpg = jpeg_bytes(img);
        begin(&mut out, &mut offsets, image_obj);
        out.extend_from_slice(
            format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                img.width(),
                img.height(),
                jpg.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&jpg);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes(),
    );
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    std::fs::write(path, out)
}

/// slide-01.png, slide-02.png … in `dir`.
pub fn write_images(dir: &Path, imgs: &[RgbaImage]) -> Result<(), String> {
    for (k, img) in imgs.iter().enumerate() {
        img.save(dir.join(format!("slide-{:02}.png", k + 1)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if ch.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if ch.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '<' => o.push_str("\\u003c"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A single web page: arrows/space/click to move, N for speaker notes.
/// One web page: slide pictures, with real videos placed over their still
/// frames. Videos are copied into a "<name>-media" folder next to the page.
pub fn write_html(
    path: &Path,
    imgs: &[RgbaImage],
    notes: &[String],
    title: &str,
    videos: &[Vec<ExportVideo>],
    texts: &[String],
) -> std::io::Result<()> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "slides".into());
    let media_name = format!("{stem}-media");
    let media_dir = path.with_file_name(&media_name);
    let aspect = imgs
        .first()
        .map(|i| i.width() as f32 / i.height().max(1) as f32)
        .unwrap_or(16.0 / 9.0);
    // Each video file is copied once, even when several slides use it.
    let mut copied: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
    let mut h = String::new();
    h.push_str(&format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>{}</title><style>\
html,body{{margin:0;height:100%;background:#000;overflow:hidden}}\
.slide{{display:none;position:absolute;inset:0;align-items:center;justify-content:center}}\
.slide.on{{display:flex}}\
.frame{{position:relative;width:min(100vw,calc(100vh*{aspect:.5}));aspect-ratio:{aspect:.5};container-type:size}}\
.frame img{{display:block;width:100%;height:100%}}\
.frame video{{position:absolute;background:transparent}}\
.frame .t{{position:absolute;white-space:pre-wrap;pointer-events:none}}\
#notes{{display:none;position:fixed;left:0;right:0;bottom:0;max-height:35vh;overflow:auto;background:rgba(20,20,20,.94);color:#eee;font:20px/1.5 -apple-system,Helvetica,sans-serif;padding:16px 28px;white-space:pre-wrap}}\
#n{{position:fixed;right:12px;bottom:8px;color:#777;font:12px -apple-system,sans-serif}}\
</style></head><body>\n",
        xml_escape(title)
    ));
    for (k, img) in imgs.iter().enumerate() {
        let mut vids = String::new();
        for v in videos.get(k).map(|v| v.as_slice()).unwrap_or(&[]) {
            let src = if let Some(p) = &v.path {
                let name = match copied.get(p) {
                    Some(n) => n.clone(),
                    None => {
                        std::fs::create_dir_all(&media_dir)?;
                        let file = p
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "video.mp4".into());
                        // Two different files with the same name get a number.
                        let name = if copied.values().any(|n| *n == file) {
                            format!("{}-{file}", copied.len() + 1)
                        } else {
                            file
                        };
                        std::fs::copy(p, media_dir.join(&name))?;
                        copied.insert(p.clone(), name.clone());
                        name
                    }
                };
                url_path(&format!("{media_name}/{name}"))
            } else if let Some(u) = &v.url {
                xml_escape(u)
            } else {
                continue;
            };
            vids.push_str(&format!(
                "<video src=\"{src}\" playsinline preload=\"metadata\" {} style=\"left:{:.4}%;top:{:.4}%;width:{:.4}%;height:{:.4}%;object-fit:{};object-position:{:.1}% {:.1}%;opacity:{:.3};filter:{}\"></video>",
                if v.background { "muted loop data-bg" } else { "controls" },
                v.x * 100.0,
                v.y * 100.0,
                v.w * 100.0,
                v.h * 100.0,
                if v.contain { "contain" } else { "cover" },
                v.align[0] * 100.0,
                v.align[1] * 100.0,
                v.opacity,
                css_filter(v.filter),
            ));
        }
        h.push_str(&format!(
            "<div class=\"slide\"><div class=\"frame\"><img alt=\"Slide {}\" src=\"data:image/jpeg;base64,{}\">{vids}{}</div></div>\n",
            k + 1,
            b64(&jpeg_bytes(img)),
            texts.get(k).map(|t| t.as_str()).unwrap_or(""),
        ));
    }
    let notes_js: Vec<String> = notes.iter().map(|n| json_str(n)).collect();
    h.push_str(&format!(
        "<div id=\"notes\"></div><div id=\"n\"></div><script>\
const notes=[{}];let i=0;const s=document.querySelectorAll('.slide');\
function show(k){{i=Math.max(0,Math.min(s.length-1,k));s.forEach((e,j)=>e.classList.toggle('on',j==i));\
document.querySelectorAll('video').forEach(v=>{{if(v.closest('.slide')!==s[i])v.pause();else if('bg' in v.dataset)v.play().catch(()=>{{}})}});\
document.getElementById('notes').textContent=notes[i]||'';document.getElementById('n').textContent=(i+1)+' / '+s.length+'  ·  N notes'+(s[i].querySelector('video:not([data-bg])')?'  ·  V video':'')}}\
document.addEventListener('keydown',e=>{{if(['ArrowRight','ArrowDown',' ','PageDown','Enter'].includes(e.key)){{e.preventDefault();show(i+1)}}\
if(['ArrowLeft','ArrowUp','PageUp','Backspace'].includes(e.key))show(i-1);if(e.key=='Home')show(0);if(e.key=='End')show(s.length-1);\
if(e.key=='v'||e.key=='V'){{const v=s[i].querySelector('video:not([data-bg])');if(v)v.paused?v.play():v.pause()}}\
if(e.key=='n'||e.key=='N'){{const n=document.getElementById('notes');n.style.display=n.style.display=='block'?'none':'block'}}}});\
document.addEventListener('click',e=>{{if(e.target.id!='notes'&&e.target.tagName!='VIDEO')show(i+1)}});show(0);</script></body></html>\n",
        notes_js.join(",")
    ));
    std::fs::write(path, h)
}

const NS: &str = "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"";
const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const EMPTY_TREE: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";

fn rels(items: &[(&str, &str, String)]) -> String {
    let mut s = format!("{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    for (id, kind, target) in items {
        let ty = if kind.starts_with("http") {
            kind.to_string()
        } else {
            format!("{REL}/{kind}")
        };
        s.push_str(&format!(
            "<Relationship Id=\"{id}\" Type=\"{ty}\" Target=\"{target}\"/>"
        ));
    }
    s.push_str("</Relationships>");
    s
}

fn theme_xml() -> String {
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line = "<a:ln w=\"9525\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>";
    let effect = "<a:effectStyle><a:effectLst/></a:effectStyle>";
    format!(
        "{HEAD}<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Slides\"><a:themeElements>\
<a:clrScheme name=\"Slides\"><a:dk1><a:srgbClr val=\"000000\"/></a:dk1><a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1>\
<a:dk2><a:srgbClr val=\"1F1F1F\"/></a:dk2><a:lt2><a:srgbClr val=\"EEEEEE\"/></a:lt2>\
<a:accent1><a:srgbClr val=\"FF3333\"/></a:accent1><a:accent2><a:srgbClr val=\"43AA8B\"/></a:accent2>\
<a:accent3><a:srgbClr val=\"F9C74F\"/></a:accent3><a:accent4><a:srgbClr val=\"90BE6D\"/></a:accent4>\
<a:accent5><a:srgbClr val=\"F8961E\"/></a:accent5><a:accent6><a:srgbClr val=\"577590\"/></a:accent6>\
<a:hlink><a:srgbClr val=\"3770FF\"/></a:hlink><a:folHlink><a:srgbClr val=\"7F5FBF\"/></a:folHlink></a:clrScheme>\
<a:fontScheme name=\"Slides\"><a:majorFont><a:latin typeface=\"Helvetica\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont>\
<a:minorFont><a:latin typeface=\"Helvetica\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme>\
<a:fmtScheme name=\"Slides\"><a:fillStyleLst>{fill}{fill}{fill}</a:fillStyleLst><a:lnStyleLst>{line}{line}{line}</a:lnStyleLst>\
<a:effectStyleLst>{effect}{effect}{effect}</a:effectStyleLst><a:bgFillStyleLst>{fill}{fill}{fill}</a:bgFillStyleLst></a:fmtScheme>\
</a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"
    )
}

/// A PowerPoint file with one full-slide picture per slide.
/// The notes master: where the slide thumbnail and the notes text sit on a
/// printed notes page.
fn notes_master_xml() -> String {
    format!(
        "{HEAD}<p:notesMaster {NS}><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg><p:spTree>{EMPTY_TREE}\
<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Slide Image Placeholder 1\"/><p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldImg\" idx=\"2\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"381000\" y=\"685800\"/><a:ext cx=\"6096000\" cy=\"3429000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/><a:ln w=\"12700\"><a:solidFill><a:prstClr val=\"black\"/></a:solidFill></a:ln></p:spPr></p:sp>\
<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Notes Placeholder 2\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" sz=\"quarter\" idx=\"3\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"685800\" y=\"4400550\"/><a:ext cx=\"5486400\" cy=\"3600450\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>\
<p:txBody><a:bodyPr vert=\"horz\" lIns=\"91440\" tIns=\"45720\" rIns=\"91440\" bIns=\"45720\" rtlCol=\"0\"/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody></p:sp>\
</p:spTree></p:cSld>\
<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
<p:notesStyle><a:lvl1pPr marL=\"0\" algn=\"l\" defTabSz=\"914400\" rtl=\"0\" eaLnBrk=\"1\" latinLnBrk=\"0\" hangingPunct=\"1\"><a:defRPr sz=\"1200\" kern=\"1200\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl1pPr></p:notesStyle></p:notesMaster>"
    )
}

/// One slide's speaker notes page.
fn notes_slide_xml(notes: &str) -> String {
    let mut paras = String::new();
    for line in notes.lines() {
        if line.trim().is_empty() {
            paras.push_str("<a:p><a:endParaRPr lang=\"en-US\" dirty=\"0\"/></a:p>");
        } else {
            paras.push_str(&format!(
                "<a:p><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/><a:t>{}</a:t></a:r></a:p>",
                xml_escape(line.trim_end())
            ));
        }
    }
    format!(
        "{HEAD}<p:notes {NS}><p:cSld><p:spTree>{EMPTY_TREE}\
<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Slide Image Placeholder 1\"/><p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldImg\"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>\
<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Notes Placeholder 2\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/>\
<p:txBody><a:bodyPr/><a:lstStyle/>{paras}</p:txBody></p:sp>\
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>"
    )
}

/// Writes a PowerPoint file: each slide's picture as the background, with
/// `texts[k]` (native text boxes, may be empty) on top so text stays editable,
/// and `notes[k]` as that slide's speaker notes.
pub fn write_pptx(
    path: &Path,
    imgs: &[RgbaImage],
    texts: &[String],
    notes: &[String],
    videos: &[Vec<ExportVideo>],
) -> Result<(), String> {
    // Each video file goes into the presentation once, shared by its slides.
    let mut embedded: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
    // Only local files can go inside the presentation.
    let local = |k: usize| -> Vec<&ExportVideo> {
        videos
            .get(k)
            .map(|v| v.iter().filter(|v| v.path.is_some()).collect())
            .unwrap_or_default()
    };
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(file);
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let put = |z: &mut zip::ZipWriter<std::fs::File>,
               name: &str,
               data: &[u8],
               o: zip::write::SimpleFileOptions|
     -> Result<(), String> {
        z.start_file(name, o).map_err(|e| e.to_string())?;
        z.write_all(data).map_err(|e| e.to_string())
    };
    let n = imgs.len();
    let mut types = format!(
        "{HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>\
<Default Extension=\"mp4\" ContentType=\"video/mp4\"/><Default Extension=\"m4v\" ContentType=\"video/x-m4v\"/><Default Extension=\"mov\" ContentType=\"video/quicktime\"/><Default Extension=\"webm\" ContentType=\"video/webm\"/>\
<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\
<Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>\
<Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>\
<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>\
<Override PartName=\"/ppt/theme/theme2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>\
<Override PartName=\"/ppt/notesMasters/notesMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml\"/>"
    );
    let has_notes = |k: usize| notes.get(k).map(|t| !t.trim().is_empty()).unwrap_or(false);
    for k in 1..=n {
        types.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{k}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"
        ));
        if has_notes(k - 1) {
            types.push_str(&format!(
                "<Override PartName=\"/ppt/notesSlides/notesSlide{k}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>"
            ));
        }
    }
    types.push_str("</Types>");
    put(&mut z, "[Content_Types].xml", types.as_bytes(), opt)?;
    put(
        &mut z,
        "_rels/.rels",
        rels(&[("rId1", "officeDocument", "ppt/presentation.xml".into())]).as_bytes(),
        opt,
    )?;

    let notes_master_rid = format!("rId{}", n + 3);
    let mut pres = format!("{HEAD}<p:presentation {NS}><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:notesMasterIdLst><p:notesMasterId r:id=\"{notes_master_rid}\"/></p:notesMasterIdLst><p:sldIdLst>");
    let mut pres_rels: Vec<(String, &str, String)> = vec![
        (
            "rId1".into(),
            "slideMaster",
            "slideMasters/slideMaster1.xml".into(),
        ),
        ("rId2".into(), "theme", "theme/theme1.xml".into()),
    ];
    for k in 1..=n {
        pres.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rId{}\"/>",
            255 + k,
            k + 2
        ));
        pres_rels.push((
            format!("rId{}", k + 2),
            "slide",
            format!("slides/slide{k}.xml"),
        ));
    }
    pres_rels.push((
        notes_master_rid.clone(),
        "notesMaster",
        "notesMasters/notesMaster1.xml".into(),
    ));
    pres.push_str("</p:sldIdLst><p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>");
    put(&mut z, "ppt/presentation.xml", pres.as_bytes(), opt)?;
    let pr: Vec<(&str, &str, String)> = pres_rels
        .iter()
        .map(|(a, b, c)| (a.as_str(), *b, c.clone()))
        .collect();
    put(
        &mut z,
        "ppt/_rels/presentation.xml.rels",
        rels(&pr).as_bytes(),
        opt,
    )?;

    let master = format!(
        "{HEAD}<p:sldMaster {NS}><p:cSld><p:spTree>{EMPTY_TREE}</p:spTree></p:cSld>\
<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst></p:sldMaster>"
    );
    put(
        &mut z,
        "ppt/slideMasters/slideMaster1.xml",
        master.as_bytes(),
        opt,
    )?;
    put(
        &mut z,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        rels(&[
            (
                "rId1",
                "slideLayout",
                "../slideLayouts/slideLayout1.xml".into(),
            ),
            ("rId2", "theme", "../theme/theme1.xml".into()),
        ])
        .as_bytes(),
        opt,
    )?;
    let layout = format!(
        "{HEAD}<p:sldLayout {NS} type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree>{EMPTY_TREE}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"
    );
    put(
        &mut z,
        "ppt/slideLayouts/slideLayout1.xml",
        layout.as_bytes(),
        opt,
    )?;
    put(
        &mut z,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        rels(&[(
            "rId1",
            "slideMaster",
            "../slideMasters/slideMaster1.xml".into(),
        )])
        .as_bytes(),
        opt,
    )?;
    put(&mut z, "ppt/theme/theme1.xml", theme_xml().as_bytes(), opt)?;
    put(&mut z, "ppt/theme/theme2.xml", theme_xml().as_bytes(), opt)?;
    put(
        &mut z,
        "ppt/notesMasters/notesMaster1.xml",
        notes_master_xml().as_bytes(),
        opt,
    )?;
    put(
        &mut z,
        "ppt/notesMasters/_rels/notesMaster1.xml.rels",
        rels(&[("rId1", "theme", "../theme/theme2.xml".into())]).as_bytes(),
        opt,
    )?;

    for (k, img) in imgs.iter().enumerate() {
        let text = texts.get(k).map(|t| t.as_str()).unwrap_or("");
        let slide_videos = local(k);
        let mut video_xml = String::new();
        let mut video_rels: Vec<(String, String, String)> = Vec::new();
        let mut video_files: Vec<(String, Vec<u8>)> = Vec::new();
        for (j, v) in slide_videos.iter().enumerate() {
            let Some(src) = &v.path else { continue };
            let n = k + 1;
            let media = match embedded.get(src) {
                Some(m) => m.clone(),
                None => {
                    let Ok(bytes) = std::fs::read(src) else {
                        continue;
                    };
                    let ext = src
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase())
                        .filter(|e| ["mp4", "m4v", "mov", "webm"].contains(&e.as_str()))
                        .unwrap_or_else(|| "mp4".into());
                    let m = format!("video{}.{ext}", embedded.len() + 1);
                    video_files.push((format!("ppt/media/{m}"), bytes));
                    embedded.insert(src.clone(), m.clone());
                    m
                }
            };
            let poster = format!("poster{n}_{}.jpeg", j + 1);
            // Cover image: the video's full still frame, so PowerPoint's crop
            // applies the same way to the cover and to the playing video.
            // Without one, the area as drawn on the slide (no crop needed).
            let still = crate::images::poster_file(src)
                .and_then(|p| image::open(p).ok())
                .map(|i| i.to_rgba8());
            let cover = match &still {
                Some(full) => full.clone(),
                None => {
                    let (iw, ih) = (img.width() as f32, img.height() as f32);
                    let px =
                        |f: f32, full: f32| ((f * full).round().max(0.0) as u32).min(full as u32);
                    let (x0, y0) = (px(v.x, iw), px(v.y, ih));
                    let (x1, y1) = (px(v.x + v.w, iw).max(x0 + 1), px(v.y + v.h, ih).max(y0 + 1));
                    image::imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image()
                }
            };
            video_files.push((format!("ppt/media/{poster}"), jpeg_bytes(&cover)));
            let (rv, rm, rp) = (
                format!("rId{}", 10 + 3 * j),
                format!("rId{}", 11 + 3 * j),
                format!("rId{}", 12 + 3 * j),
            );
            video_rels.push((rv.clone(), "video".into(), format!("../media/{media}")));
            video_rels.push((
                rm.clone(),
                "http://schemas.microsoft.com/office/2007/relationships/media".into(),
                format!("../media/{media}"),
            ));
            video_rels.push((rp.clone(), "image".into(), format!("../media/{poster}")));
            // Crop like the app does when the video fills ("cover") its area.
            let mut src_rect = String::new();
            if let (false, Some(full)) = (v.contain, &still) {
                let va = full.width() as f32 / full.height().max(1) as f32;
                let ra = (v.w * img.width() as f32) / (v.h * img.height() as f32).max(1.0);
                let pct = |f: f32| (f * 100_000.0).round() as i64;
                if va > ra {
                    let cut = 1.0 - ra / va;
                    src_rect = format!(
                        "<a:srcRect l=\"{}\" r=\"{}\"/>",
                        pct(cut * v.align[0]),
                        pct(cut * (1.0 - v.align[0]))
                    );
                } else if va < ra {
                    let cut = 1.0 - va / ra;
                    src_rect = format!(
                        "<a:srcRect t=\"{}\" b=\"{}\"/>",
                        pct(cut * v.align[1]),
                        pct(cut * (1.0 - v.align[1]))
                    );
                }
            }
            let id = 1000 + j;
            let emu_x = (v.x * SLIDE_W_EMU).round() as i64;
            let emu_y = (v.y * SLIDE_H_EMU).round() as i64;
            let emu_w = (v.w * SLIDE_W_EMU).round().max(1.0) as i64;
            let emu_h = (v.h * SLIDE_H_EMU).round().max(1.0) as i64;
            video_xml.push_str(&format!(
                "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"Video {}\"><a:hlinkClick r:id=\"\" action=\"ppaction://media\"/></p:cNvPr>\
<p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr>\
<p:nvPr><a:videoFile r:link=\"{rv}\"/><p:extLst><p:ext uri=\"{{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}}\"><p14:media xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" r:embed=\"{rm}\"/></p:ext></p:extLst></p:nvPr></p:nvPicPr>\
<p:blipFill><a:blip r:embed=\"{rp}\"/>{src_rect}<a:stretch><a:fillRect/></a:stretch></p:blipFill>\
<p:spPr><a:xfrm><a:off x=\"{emu_x}\" y=\"{emu_y}\"/><a:ext cx=\"{emu_w}\" cy=\"{emu_h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
                j + 1
            ));
        }
        let k = k + 1;
        let slide = format!(
            "{HEAD}<p:sld {NS}><p:cSld><p:spTree>{EMPTY_TREE}\
<p:pic><p:nvPicPr><p:cNvPr id=\"2\" name=\"Slide {k}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>\
<p:blipFill><a:blip r:embed=\"rId2\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>\
<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"12192000\" cy=\"6858000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>\
{video_xml}{text}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"
        );
        put(
            &mut z,
            &format!("ppt/slides/slide{k}.xml"),
            slide.as_bytes(),
            opt,
        )?;
        let mut slide_rels = vec![
            (
                "rId1",
                "slideLayout",
                "../slideLayouts/slideLayout1.xml".to_string(),
            ),
            ("rId2", "image", format!("../media/image{k}.jpeg")),
        ];
        if has_notes(k - 1) {
            slide_rels.push((
                "rId3",
                "notesSlide",
                format!("../notesSlides/notesSlide{k}.xml"),
            ));
        }
        for (id, kind, target) in &video_rels {
            slide_rels.push((id.as_str(), kind.as_str(), target.clone()));
        }
        put(
            &mut z,
            &format!("ppt/slides/_rels/slide{k}.xml.rels"),
            rels(&slide_rels).as_bytes(),
            opt,
        )?;
        for (name, data) in &video_files {
            put(&mut z, name, data, stored)?;
        }
        if has_notes(k - 1) {
            put(
                &mut z,
                &format!("ppt/notesSlides/notesSlide{k}.xml"),
                notes_slide_xml(&notes[k - 1]).as_bytes(),
                opt,
            )?;
            put(
                &mut z,
                &format!("ppt/notesSlides/_rels/notesSlide{k}.xml.rels"),
                rels(&[
                    (
                        "rId1",
                        "notesMaster",
                        "../notesMasters/notesMaster1.xml".into(),
                    ),
                    ("rId2", "slide", format!("../slides/slide{k}.xml")),
                ])
                .as_bytes(),
                opt,
            )?;
        }
        put(
            &mut z,
            &format!("ppt/media/image{k}.jpeg"),
            &jpeg_bytes(img),
            stored,
        )?;
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_files() {
        let dir = std::env::temp_dir().join("presentcraft-export-test");
        let _ = std::fs::create_dir_all(&dir);
        let imgs = vec![RgbaImage::from_pixel(64, 36, image::Rgba([200, 40, 40, 255])); 2];
        write_pdf(&dir.join("t.pdf"), &imgs).unwrap();
        write_pptx(
            &dir.join("t.pptx"),
            &imgs,
            &[],
            &["Say hello & smile".into()],
            &[],
        )
        .unwrap();
        write_html(
            &dir.join("t.html"),
            &imgs,
            &["a \"b\"".into(), "c".into()],
            "T",
            &[],
            &[],
        )
        .unwrap();
        write_images(&dir, &imgs).unwrap();
        assert!(std::fs::read(dir.join("t.pdf"))
            .unwrap()
            .starts_with(b"%PDF"));
        assert_eq!(b64(b"Man"), "TWFu");
        assert_eq!(b64(b"Ma"), "TWE=");
    }

    #[test]
    fn pptx_text_boxes_keep_styling() {
        use egui::text::{LayoutJob, TextFormat};
        use egui::{pos2, Color32, FontFamily, FontId, Rect};
        let mut job = LayoutJob::default();
        let bold = TextFormat {
            font_id: FontId::new(54.0, FontFamily::Name("sans-b".into())),
            color: Color32::from_rgb(0x11, 0x22, 0x33),
            ..Default::default()
        };
        let mono = TextFormat {
            font_id: FontId::new(27.0, FontFamily::Name("mono".into())),
            color: Color32::WHITE,
            ..Default::default()
        };
        job.append("Tom & Jerry", 0.0, bold);
        job.append("\n  x < 1", 0.0, mono);
        job.halign = egui::Align::Center;
        let slide = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(960.0, 540.0));
        let b = crate::render::TextBox {
            rect: Rect::from_min_size(pos2(100.0, 50.0), egui::vec2(300.0, 80.0)),
            job: std::sync::Arc::new(job),
            fallback: Color32::BLACK,
        };
        let xml = pptx_text_shapes(&[b], slide);
        assert!(xml.contains("<a:t>Tom &amp; Jerry</a:t>"));
        assert!(xml.contains("sz=\"5400\" b=\"1\""));
        assert!(xml.contains("val=\"112233\""));
        assert!(xml.contains("algn=\"ctr\""));
        assert!(xml.contains("typeface=\"Consolas\""));
        // leading spaces survive, the rest is escaped
        assert!(xml.contains("<a:t>\u{a0}\u{a0}x &lt; 1</a:t>"));
        assert_eq!(xml.matches("<a:p>").count(), 2);
    }
}
