//! Turns a Markdown document into slides.
//!
//! Parsing rules:
//! * a line made only of three or more dashes starts a new slide;
//! * headings are always visible;
//! * paragraphs, lists and quotes are speaker notes unless the line starts with a tab;
//! * pictures (a path or web address alone on a line, or `![alt](src)`) and tables are visible;
//! * a table followed by `Chart: <type>` is drawn as a chart;
//! * visible elements separated by an empty line (or by notes) form separate groups,
//!   and the number of groups picks the layout.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    Grayscale,
    Sepia,
    Darken,
    Lighten,
    Blur,
}

#[derive(Clone, Debug)]
pub struct ImageSpec {
    pub src: String,
    pub contain: bool,
    /// 0 = left, 0.5 = center, 1 = right
    pub x: f32,
    /// 0 = top, 0.5 = center, 1 = bottom
    pub y: f32,
    pub background: bool,
    pub opacity: f32,
    pub filter: Option<Filter>,
    pub caption: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TableSpec {
    pub header: Vec<String>,
    pub aligns: Vec<Align>,
    pub rows: Vec<Vec<String>>,
    pub caption: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChartKind {
    Bar,
    Line,
    Spline,
    Area,
    Step,
    Pie,
    Donut,
    Radar,
    Gauge,
    Bubble,
}

impl ChartKind {
    pub fn from_word(w: &str) -> Option<ChartKind> {
        Some(match w.to_ascii_lowercase().as_str() {
            "bar" | "bars" | "column" | "columns" => ChartKind::Bar,
            "line" | "lines" => ChartKind::Line,
            "spline" | "curve" => ChartKind::Spline,
            "area" => ChartKind::Area,
            "step" => ChartKind::Step,
            "pie" => ChartKind::Pie,
            "donut" | "doughnut" => ChartKind::Donut,
            "radar" => ChartKind::Radar,
            "gauge" => ChartKind::Gauge,
            "bubble" | "scatter" => ChartKind::Bubble,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChartSpec {
    pub table: TableSpec,
    pub kind: ChartKind,
    pub horizontal: bool,
    pub stacked: bool,
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    pub colors: Vec<[u8; 3]>,
    pub legend: bool,
}

#[derive(Clone, Debug)]
pub struct ListItem {
    pub indent: usize,
    pub text: String,
    pub task: Option<bool>,
    pub number: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List(Vec<ListItem>),
    Quote(String),
    Image(ImageSpec),
    Table(TableSpec),
    Chart(ChartSpec),
    /// Code text and the fence's language name (may be empty).
    Code(String, String),
    Math(String),
}

impl Block {
    pub fn is_flexible(&self) -> bool {
        matches!(self, Block::Image(_) | Block::Table(_) | Block::Chart(_))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Slide {
    pub groups: Vec<Vec<Block>>,
    pub backgrounds: Vec<ImageSpec>,
    pub notes: Vec<String>,
    /// Character offset of the slide's first line in the document.
    pub start_char: usize,
}

/// Is this line a slide break?
pub fn is_separator(line: &str) -> bool {
    let t = line.trim_end();
    t.len() >= 3 && t.chars().all(|c| c == '-')
}

pub fn parse_document(text: &str) -> Vec<Slide> {
    let mut slides = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut in_fence = false;
    let mut start_char = 0usize;
    let mut char_pos = 0usize;
    for line in text.split('\n') {
        let len = line.chars().count() + 1;
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && is_separator(line) {
            let mut s = parse_slide(&current);
            s.start_char = start_char;
            slides.push(s);
            current.clear();
            start_char = char_pos + len;
        } else {
            current.push(line);
        }
        char_pos += len;
    }
    let mut s = parse_slide(&current);
    s.start_char = start_char;
    slides.push(s);
    slides
}

/// Index of the slide that contains the given character offset.
pub fn slide_at_char(slides: &[Slide], ch: usize) -> usize {
    let mut idx = 0;
    for (i, s) in slides.iter().enumerate() {
        if s.start_char <= ch {
            idx = i;
        }
    }
    idx
}

struct Builder {
    slide: Slide,
    pending_break: bool,
    note_buf: String,
    open_paragraph: bool,
    open_list: bool,
    open_quote: bool,
}

impl Builder {
    fn push(&mut self, b: Block) {
        if self.pending_break || self.slide.groups.is_empty() {
            self.slide.groups.push(Vec::new());
        }
        self.pending_break = false;
        self.slide.groups.last_mut().unwrap().push(b);
    }
    fn close_open(&mut self) {
        self.open_paragraph = false;
        self.open_list = false;
        self.open_quote = false;
    }
    fn flush_notes(&mut self) {
        let n = self.note_buf.trim();
        if !n.is_empty() {
            self.slide.notes.push(n.to_string());
        }
        self.note_buf.clear();
    }
    fn last_block(&mut self) -> Option<&mut Block> {
        if self.pending_break {
            return None;
        }
        self.slide.groups.last_mut().and_then(|g| g.last_mut())
    }
}

const IMAGE_EXTS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".tif", ".tiff", ".svg", ".pdf", ".apng", ".bmp",
    ".mp4", ".mov", ".m4v",
];

pub(crate) fn is_image_src(s: &str) -> bool {
    if s.is_empty() || s.contains(char::is_whitespace) || s.starts_with('|') {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return true;
    }
    IMAGE_EXTS.iter().any(|e| lower.ends_with(e))
}

/// `![alt](src)` alone on a line.
pub(crate) fn markdown_image(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if !s.starts_with("![") || !s.ends_with(')') {
        return None;
    }
    let close = s.find("](")?;
    let alt = s[2..close].to_string();
    let mut src = s[close + 2..s.len() - 1].trim().to_string();
    if let Some(sp) = src.find(' ') {
        src.truncate(sp);
    }
    Some((alt, src))
}

fn key_value(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    let colon = t.find(':')?;
    let key = t[..colon].trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some((key.to_ascii_lowercase(), t[colon + 1..].trim().to_string()))
}

const IMAGE_KEYS: &[&str] = &[
    "size",
    "x",
    "y",
    "background",
    "opacity",
    "filter",
    "title",
    "caption",
    "position",
];
// Supported chart option keys.
const CHART_KEYS: &[&str] = &[
    "chart", "type", "title", "caption", "xlabel", "ylabel", "colors", "xformat", "legend",
];

fn new_image(src: String, caption: Option<String>) -> ImageSpec {
    ImageSpec {
        src,
        contain: false,
        x: 0.5,
        y: 0.5,
        background: false,
        opacity: 1.0,
        filter: None,
        caption: caption.filter(|c| !c.is_empty()),
    }
}

fn apply_image_meta(img: &mut ImageSpec, key: &str, val: &str) {
    let v = val.to_ascii_lowercase();
    match key {
        "size" => img.contain = v.contains("contain"),
        "x" => {
            img.x = match v.as_str() {
                "left" => 0.0,
                "right" => 1.0,
                _ => 0.5,
            }
        }
        "y" => {
            img.y = match v.as_str() {
                "top" => 0.0,
                "bottom" => 1.0,
                _ => 0.5,
            }
        }
        "position" => {
            if v.contains("left") {
                img.x = 0.0
            }
            if v.contains("right") {
                img.x = 1.0
            }
            if v.contains("top") {
                img.y = 0.0
            }
            if v.contains("bottom") {
                img.y = 1.0
            }
        }
        "background" => img.background = matches!(v.as_str(), "true" | "yes" | "1" | "on"),
        "opacity" => {
            let num: String = v
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(n) = num.parse::<f32>() {
                img.opacity = if v.contains('%') || n > 1.0 {
                    n / 100.0
                } else {
                    n
                }
                .clamp(0.0, 1.0);
            }
        }
        "filter" => {
            img.filter = match v.as_str() {
                "grayscale" | "greyscale" | "gray" | "grey" => Some(Filter::Grayscale),
                "sepia" => Some(Filter::Sepia),
                "darken" | "dark" => Some(Filter::Darken),
                "lighten" | "light" => Some(Filter::Lighten),
                "blur" => Some(Filter::Blur),
                _ => None,
            }
        }
        "title" | "caption" => {
            let c = val.trim().trim_matches('"').to_string();
            img.caption = if c.is_empty() { None } else { Some(c) };
        }
        _ => {}
    }
}

fn split_row(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_align_row(line: &str) -> bool {
    let t = line.trim();
    t.contains('-') && t.chars().all(|c| matches!(c, '|' | ':' | '-' | ' ' | '\t'))
}

fn parse_colors(v: &str) -> Vec<[u8; 3]> {
    v.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| {
            let h = s.trim().trim_start_matches('#');
            let h: String = if h.len() == 3 {
                h.chars().flat_map(|c| [c, c]).collect()
            } else {
                h.to_string()
            };
            if h.len() != 6 {
                return None;
            }
            let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
            Some([p(0)?, p(2)?, p(4)?])
        })
        .collect()
}

fn strip_tab(raw: &str) -> (bool, &str) {
    if let Some(rest) = raw.strip_prefix('\t') {
        (true, rest)
    } else {
        (false, raw)
    }
}

fn list_marker(s: &str) -> Option<(Option<String>, &str)> {
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = s.strip_prefix(m) {
            return Some((None, rest));
        }
    }
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &s[digits.len()..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some((Some(digits), r));
        }
    }
    None
}

pub fn parse_slide(lines: &[&str]) -> Slide {
    let mut b = Builder {
        slide: Slide::default(),
        pending_break: false,
        note_buf: String::new(),
        open_paragraph: false,
        open_list: false,
        open_quote: false,
    };
    let n = lines.len();
    let mut i = 0;
    while i < n {
        let raw = lines[i].trim_end_matches('\r');
        if raw.trim().is_empty() {
            b.flush_notes();
            b.close_open();
            b.pending_break = true;
            i += 1;
            continue;
        }
        // Comments are neither shown nor spoken.
        if raw.trim_start().starts_with("//") {
            i += 1;
            continue;
        }
        let (tabbed, content) = strip_tab(raw);
        let ct = content.trim_start();

        // Headings
        if !tabbed && raw.starts_with('#') {
            let level = raw.chars().take_while(|c| *c == '#').count();
            let rest = &raw[level..];
            if level <= 6 && (rest.is_empty() || rest.starts_with(' ')) {
                b.flush_notes();
                b.close_open();
                let text = rest.trim().trim_end_matches('#').trim().to_string();
                if !text.is_empty() {
                    b.push(Block::Heading {
                        level: level as u8,
                        text,
                    });
                }
                i += 1;
                continue;
            }
        }

        // Fenced code (visible)
        if ct.starts_with("```") {
            b.flush_notes();
            b.close_open();
            let lang = ct.trim_start_matches('`').trim().to_string();
            let mut code = Vec::new();
            i += 1;
            while i < n && !lines[i].trim_start().starts_with("```") {
                code.push(strip_tab(lines[i]).1.to_string());
                i += 1;
            }
            i += 1;
            b.push(Block::Code(code.join("\n"), lang));
            continue;
        }

        // Block math (visible)
        if ct.starts_with("$$") || ct.starts_with("\\[") {
            b.flush_notes();
            b.close_open();
            let close = if ct.starts_with("$$") { "$$" } else { "\\]" };
            let mut body = ct[2..].to_string();
            if body.trim_end().ends_with(close) {
                body = body.trim_end().trim_end_matches(close).to_string();
                i += 1;
            } else {
                i += 1;
                while i < n {
                    let l = lines[i].trim();
                    i += 1;
                    if let Some(p) = l.find(close) {
                        body.push(' ');
                        body.push_str(&l[..p]);
                        break;
                    }
                    body.push(' ');
                    body.push_str(l);
                }
            }
            b.push(Block::Math(body.trim().to_string()));
            continue;
        }

        // Tables and charts (visible by default)
        if ct.starts_with('|') {
            b.flush_notes();
            b.close_open();
            let mut rows_raw = Vec::new();
            while i < n && strip_tab(lines[i]).1.trim_start().starts_with('|') {
                rows_raw.push(strip_tab(lines[i]).1);
                i += 1;
            }
            let mut table = TableSpec::default();
            let mut start = 0;
            if !rows_raw.is_empty() {
                table.header = split_row(rows_raw[0]);
                start = 1;
                if rows_raw.len() > 1 && is_align_row(rows_raw[1]) {
                    table.aligns = split_row(rows_raw[1])
                        .iter()
                        .map(|c| {
                            let l = c.starts_with(':');
                            let r = c.ends_with(':');
                            match (l, r) {
                                (true, true) => Align::Center,
                                (false, true) => Align::Right,
                                _ => Align::Left,
                            }
                        })
                        .collect();
                    start = 2;
                }
            }
            for r in &rows_raw[start..] {
                table.rows.push(split_row(r));
            }
            // metadata lines after the table
            let mut meta: Vec<(String, String)> = Vec::new();
            while i < n {
                let l = strip_tab(lines[i]).1.trim();
                if l.starts_with('[') && l.ends_with(']') && l.len() > 2 {
                    table.caption = Some(l[1..l.len() - 1].to_string());
                    i += 1;
                    continue;
                }
                match key_value(l) {
                    Some((k, v)) if CHART_KEYS.contains(&k.as_str()) => {
                        meta.push((k, v));
                        i += 1;
                    }
                    _ => break,
                }
            }
            let chart_word = meta
                .iter()
                .find(|(k, _)| k == "chart" || k == "type")
                .map(|(_, v)| v.clone());
            let mut kind = None;
            let mut horizontal = false;
            let mut stacked = false;
            if let Some(w) = &chart_word {
                for word in w.split(|c: char| c == ',' || c.is_whitespace() || c == '-') {
                    if word.is_empty() {
                        continue;
                    }
                    match word.to_ascii_lowercase().as_str() {
                        "horizontal" => horizontal = true,
                        "stacked" => stacked = true,
                        other => {
                            if kind.is_none() {
                                kind = ChartKind::from_word(other)
                            }
                        }
                    }
                }
                if kind.is_none() {
                    kind = Some(ChartKind::Bar);
                }
            }
            let get = |key: &str| {
                meta.iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
                    .filter(|v| !v.is_empty())
            };
            if let Some(kind) = kind {
                if table.caption.is_none() {
                    table.caption = get("caption");
                }
                let legend = get("legend")
                    .map(|v| {
                        !matches!(
                            v.to_ascii_lowercase().as_str(),
                            "false" | "no" | "off" | "none"
                        )
                    })
                    .unwrap_or(true);
                b.push(Block::Chart(ChartSpec {
                    kind,
                    horizontal,
                    stacked,
                    title: get("title"),
                    xlabel: get("xlabel"),
                    ylabel: get("ylabel"),
                    colors: get("colors").map(|c| parse_colors(&c)).unwrap_or_default(),
                    legend,
                    table,
                }));
            } else {
                if table.caption.is_none() {
                    table.caption = get("caption").or_else(|| get("title"));
                }
                b.push(Block::Table(table));
            }
            continue;
        }

        // Pictures (visible by default)
        let md_img = markdown_image(ct);
        if md_img.is_some() || is_image_src(ct.trim()) {
            b.flush_notes();
            b.close_open();
            let mut img = match md_img {
                Some((alt, src)) => new_image(src, Some(alt)),
                None => new_image(ct.trim().to_string(), None),
            };
            i += 1;
            while i < n {
                match key_value(lines[i]) {
                    Some((k, v)) if IMAGE_KEYS.contains(&k.as_str()) => {
                        apply_image_meta(&mut img, &k, &v);
                        i += 1;
                    }
                    _ => break,
                }
            }
            if img.background {
                b.slide.backgrounds.push(img);
            } else {
                b.push(Block::Image(img));
            }
            continue;
        }

        if tabbed {
            b.flush_notes();
            // Visible text: list, quote or paragraph.
            let indent = content.len() - content.trim_start().len();
            if let Some((number, rest)) = list_marker(ct) {
                let (task, text) = if let Some(r) = rest.strip_prefix("[ ] ") {
                    (Some(false), r)
                } else if let Some(r) = rest
                    .strip_prefix("[x] ")
                    .or_else(|| rest.strip_prefix("[X] "))
                {
                    (Some(true), r)
                } else {
                    (None, rest)
                };
                let item = ListItem {
                    indent: indent / 2,
                    text: text.trim().to_string(),
                    task,
                    number,
                };
                let open = b.open_list;
                if open {
                    if let Some(Block::List(items)) = b.last_block() {
                        items.push(item);
                        i += 1;
                        continue;
                    }
                }
                b.close_open();
                b.push(Block::List(vec![item]));
                b.open_list = true;
            } else if let Some(rest) = ct.strip_prefix('>') {
                let text = rest.trim_start_matches('>').trim().to_string();
                let open = b.open_quote;
                if open {
                    if let Some(Block::Quote(q)) = b.last_block() {
                        if text.is_empty() {
                            q.push('\n');
                        } else {
                            if !q.is_empty() && !q.ends_with('\n') {
                                q.push(' ');
                            }
                            q.push_str(&text);
                        }
                        i += 1;
                        continue;
                    }
                }
                b.close_open();
                b.push(Block::Quote(text));
                b.open_quote = true;
            } else {
                let text = ct.trim_end();
                let hard_break = content.ends_with("  ") || text.ends_with('\\');
                let text = text.trim_end_matches('\\').to_string();
                let open = b.open_paragraph && !crate::settings::single_return();
                if open {
                    if let Some(Block::Paragraph(p)) = b.last_block() {
                        if !p.ends_with('\n') {
                            p.push(' ');
                        }
                        p.push_str(&text);
                        if hard_break {
                            p.push('\n');
                        }
                        i += 1;
                        continue;
                    }
                }
                b.close_open();
                let mut p = text;
                if hard_break {
                    p.push('\n');
                }
                b.push(Block::Paragraph(p));
                b.open_paragraph = true;
            }
            i += 1;
            continue;
        }

        // Everything else is spoken, not shown.
        b.close_open();
        if !b.note_buf.is_empty() {
            b.note_buf.push('\n');
        }
        b.note_buf.push_str(raw.trim());
        b.pending_break = true;
        i += 1;
    }
    b.flush_notes();
    b.slide
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_classifies() {
        let doc = "# Title\n\tSub\n\nSpoken words.\n---\n### Data\n| | A |\n|:--|:--|\n| x | 1 |\nChart: Bar\nyLabel: Units\n---\n/a.png\n\n### Hi";
        let s = parse_document(doc);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].groups.len(), 1);
        assert_eq!(s[0].groups[0].len(), 2);
        assert_eq!(s[0].notes, vec!["Spoken words.".to_string()]);
        assert!(matches!(s[1].groups[0][1], Block::Chart(_)));
        assert_eq!(s[2].groups.len(), 2);
        assert_eq!(slide_at_char(&s, 0), 0);
    }
}
