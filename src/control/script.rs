//! Slide edits on the Markdown script. Pure text functions, so the app and
//! the headless shim make the same changes.
//!
//! Slides are numbered from 1 in the tools (as the app shows them). Internally
//! a slide is its body text: the lines between slide breaks, split exactly as
//! `parse_document` splits them.

use crate::json::Json;

/// One change to the script.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Replace the whole script.
    Replace(String),
    /// Insert a slide at a 0-based position (default: the end).
    Add {
        at: Option<usize>,
        md: String,
    },
    Update {
        index: usize,
        md: String,
    },
    Delete {
        index: usize,
    },
    Move {
        from: usize,
        to: usize,
    },
    Duplicate {
        index: usize,
    },
    /// Add a picture (or video, for a video path) to the end of a slide.
    Picture {
        index: usize,
        src: String,
        caption: Option<String>,
        opts: Vec<(String, String)>,
    },
    /// Add a table to the end of a slide, optionally drawn as a chart.
    Table {
        index: usize,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        caption: Option<String>,
        chart: Option<Chart>,
    },
    /// Place a cover title: the `x:` / `y:` lines under the slide's `#`
    /// heading. `None` keeps the current value.
    TitleAlign {
        index: usize,
        x: Option<String>,
        y: Option<String>,
    },
}

/// How a table is drawn as a chart: the `Chart:` line and the option lines
/// the parser reads under a table.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Chart {
    /// bar, line, spline, area, step, pie, donut, radar, gauge or bubble.
    pub kind: String,
    pub horizontal: bool,
    pub stacked: bool,
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    /// Hex colours (`#E63946`) or theme accents (`accent 1`).
    pub colors: Vec<String>,
    /// categorical, sequential or diverging.
    pub color_type: Option<String>,
    /// `Some(false)` hides the legend.
    pub legend: Option<bool>,
}

/// The chart types the parser draws.
pub const CHART_KINDS: [&str; 10] = [
    "bar", "line", "spline", "area", "step", "pie", "donut", "radar", "gauge", "bubble",
];

/// The body text of every slide, split as `parse_document` splits them.
pub fn bodies(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut in_fence = false;
    for line in text.split('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && crate::parse::is_separator(line) {
            out.push(cur.join("\n").trim_matches('\n').to_string());
            cur.clear();
        } else {
            cur.push(line);
        }
    }
    out.push(cur.join("\n").trim_matches('\n').to_string());
    out
}

/// Joins slide bodies back into a script.
pub fn join(bodies: &[String]) -> String {
    let parts: Vec<&str> = bodies.iter().map(|b| b.trim_matches('\n')).collect();
    parts.join("\n\n---\n\n")
}

/// The slide's title: its first heading, else its first line of text.
pub fn title_of(body: &str) -> String {
    let heading = body
        .lines()
        .find(|l| l.trim_start().starts_with('#'))
        .map(|l| l.trim().trim_start_matches('#').trim().to_string());
    let title = heading.unwrap_or_else(|| {
        body.lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty() && !l.starts_with("//"))
            .unwrap_or("")
            .to_string()
    });
    title.chars().take(60).collect()
}

/// Does a line of a slide show on the slide (rather than being notes)?
pub fn is_visible_line(line: &str) -> bool {
    let t = line.trim_start();
    line.starts_with('\t')
        || t.starts_with('#')
        || t.starts_with("![")
        || t.starts_with('|')
        || t.starts_with("/assets/")
        || t.starts_with("http://")
        || t.starts_with("https://")
}

/// Adds a block after a slide's text, separated by a blank line.
pub fn append_block(body: &str, block: &str) -> String {
    let b = body.trim_end_matches('\n');
    if b.trim().is_empty() {
        block.to_string()
    } else {
        format!("{b}\n\n{block}")
    }
}

/// `![caption](src)` followed by option lines (`size: contain`, ...).
pub fn picture_block(src: &str, caption: Option<&str>, opts: &[(String, String)]) -> String {
    let src = src.trim().replace(' ', "%20");
    let cap: String = caption
        .unwrap_or("")
        .chars()
        .filter(|c| *c != '[' && *c != ']')
        .collect();
    let mut out = format!("![{cap}]({src})");
    for (k, v) in opts {
        out.push('\n');
        out.push_str(&format!("{k}: {v}"));
    }
    out
}

/// One line of text: option values and captions cannot span lines.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A Markdown table, with an optional `[caption]` line and chart lines under it.
pub fn table_block(
    headers: &[String],
    rows: &[Vec<String>],
    caption: Option<&str>,
    chart: Option<&Chart>,
) -> String {
    let cell = |s: &str| s.replace('|', "\\|").replace('\n', " ");
    let width = headers.len();
    let mut lines = vec![
        format!(
            "| {} |",
            headers
                .iter()
                .map(|h| cell(h))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        format!(
            "| {} |",
            headers
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join(" | ")
        ),
    ];
    for row in rows {
        let cells: Vec<String> = (0..width)
            .map(|i| cell(row.get(i).map(|s| s.as_str()).unwrap_or("")))
            .collect();
        lines.push(format!("| {} |", cells.join(" | ")));
    }
    if let Some(cap) = caption {
        let cap: String = one_line(cap)
            .chars()
            .filter(|c| *c != '[' && *c != ']')
            .collect();
        if !cap.is_empty() {
            lines.push(format!("[{cap}]"));
        }
    }
    if let Some(c) = chart {
        let mut kind = c.kind.clone();
        if c.horizontal {
            kind.push_str(", horizontal");
        }
        if c.stacked {
            kind.push_str(", stacked");
        }
        lines.push(format!("Chart: {kind}"));
        for (key, value) in [
            ("title", &c.title),
            ("xlabel", &c.xlabel),
            ("ylabel", &c.ylabel),
        ] {
            if let Some(v) = value.as_deref().map(one_line).filter(|v| !v.is_empty()) {
                lines.push(format!("{key}: {v}"));
            }
        }
        if !c.colors.is_empty() {
            lines.push(format!("colors: {}", c.colors.join(", ")));
        }
        if let Some(t) = &c.color_type {
            lines.push(format!("color-type: {t}"));
        }
        if c.legend == Some(false) {
            lines.push("legend: off".to_string());
        }
    }
    lines.join("\n")
}

/// Rewrites the `x:` / `y:` lines under the first `#` heading of a slide.
/// `None` keeps the current value; default values (left, middle) are left out.
pub fn title_align(body: &str, x: Option<&str>, y: Option<&str>) -> Result<String, String> {
    use crate::parse::{title_option, Align, TitleOption, VAlign};
    let mut lines: Vec<String> = body.split('\n').map(|l| l.to_string()).collect();
    let head = lines
        .iter()
        .position(|l| l.starts_with("# ") && !l[2..].trim().is_empty())
        .ok_or("This slide has no # heading. Only a cover title (a # heading) can be placed.")?;
    let mut cur_x = Align::Left;
    let mut cur_y = VAlign::Middle;
    let mut end = head + 1;
    while end < lines.len() && !lines[end].starts_with('\t') {
        match title_option(lines[end].trim_end_matches('\r')) {
            Some(TitleOption::X(v)) => cur_x = v,
            Some(TitleOption::Y(v)) => cur_y = v,
            None => break,
        }
        end += 1;
    }
    let x = match x {
        None => cur_x,
        Some(w) => match title_option(&format!("x: {w}")) {
            Some(TitleOption::X(v)) => v,
            _ => return Err(format!("x must be left, center or right, not {w}.")),
        },
    };
    let y = match y {
        None => cur_y,
        Some(w) => match title_option(&format!("y: {w}")) {
            Some(TitleOption::Y(v)) => v,
            _ => return Err(format!("y must be top, middle or bottom, not {w}.")),
        },
    };
    let mut opts = Vec::new();
    match x {
        Align::Left => {}
        Align::Center => opts.push("x: center".to_string()),
        Align::Right => opts.push("x: right".to_string()),
    }
    match y {
        VAlign::Middle => {}
        VAlign::Top => opts.push("y: top".to_string()),
        VAlign::Bottom => opts.push("y: bottom".to_string()),
    }
    lines.splice(head + 1..end, opts);
    Ok(lines.join("\n"))
}

/// Applies one edit to a script and returns the new script.
pub fn apply(text: &str, edit: &Edit) -> Result<String, String> {
    if let Edit::Replace(md) = edit {
        return Ok(md.clone());
    }
    let mut b = bodies(text);
    let n = b.len();
    let check = |i: usize| {
        if i < n {
            Ok(i)
        } else {
            Err(format!(
                "There is no slide {}. The deck has {} slides.",
                i + 1,
                n
            ))
        }
    };
    let no_break = |md: &str| {
        if md.lines().any(crate::parse::is_separator) {
            Err(
                "A slide cannot contain a line of dashes; that line starts a new slide."
                    .to_string(),
            )
        } else {
            Ok(())
        }
    };
    match edit {
        Edit::Replace(_) => unreachable!(),
        Edit::Add { at, md } => {
            no_break(md)?;
            let pos = at.unwrap_or(n);
            if pos > n {
                return Err(format!(
                    "A new slide can go at position 1 to {}; the deck has {} slides.",
                    n + 1,
                    n
                ));
            }
            b.insert(pos, md.trim_matches('\n').to_string());
        }
        Edit::Update { index, md } => {
            no_break(md)?;
            let i = check(*index)?;
            b[i] = md.trim_matches('\n').to_string();
        }
        Edit::Delete { index } => {
            let i = check(*index)?;
            if n == 1 {
                b[0] = String::new();
            } else {
                b.remove(i);
            }
        }
        Edit::Move { from, to } => {
            let f = check(*from)?;
            check(*to)?;
            let slide = b.remove(f);
            b.insert((*to).min(b.len()), slide);
        }
        Edit::Duplicate { index } => {
            let i = check(*index)?;
            let copy = b[i].clone();
            b.insert(i + 1, copy);
        }
        Edit::Picture {
            index,
            src,
            caption,
            opts,
        } => {
            let i = check(*index)?;
            let block = picture_block(src, caption.as_deref(), opts);
            b[i] = append_block(&b[i], &block);
        }
        Edit::Table {
            index,
            headers,
            rows,
            caption,
            chart,
        } => {
            let i = check(*index)?;
            if headers.is_empty() {
                return Err("A table needs at least one column heading.".to_string());
            }
            let block = table_block(headers, rows, caption.as_deref(), chart.as_ref());
            b[i] = append_block(&b[i], &block);
        }
        Edit::TitleAlign { index, x, y } => {
            let i = check(*index)?;
            if x.is_none() && y.is_none() {
                return Err("Pass x, y or both.".to_string());
            }
            b[i] = title_align(&b[i], x.as_deref(), y.as_deref())?;
        }
    }
    Ok(join(&b))
}

fn text_arg(a: &Json, key: &str) -> Option<String> {
    match a.get(key)? {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn slide_arg(a: &Json, key: &str) -> Result<usize, String> {
    match a.num(key) {
        Some(n) if n >= 1.0 && n.fract() == 0.0 => Ok(n as usize - 1),
        _ => Err(format!("{key} must be a slide number, starting at 1.")),
    }
}

/// Reads the arguments of a script edit tool. `Ok(None)` when `tool` is not
/// a script edit.
pub fn parse_edit(tool: &str, a: &Json) -> Result<Option<Edit>, String> {
    let need = |key: &str| text_arg(a, key).ok_or_else(|| format!("{key} is required."));
    let edit = match tool {
        "deck_write_markdown" => Edit::Replace(need("markdown")?),
        "slide_add" => Edit::Add {
            at: match a.num("position") {
                Some(_) => Some(slide_arg(a, "position")?),
                None => None,
            },
            md: need("markdown")?,
        },
        "slide_update" => Edit::Update {
            index: slide_arg(a, "slide")?,
            md: need("markdown")?,
        },
        "slide_delete" => Edit::Delete {
            index: slide_arg(a, "slide")?,
        },
        "slide_move" => Edit::Move {
            from: slide_arg(a, "slide")?,
            to: slide_arg(a, "to")?,
        },
        "slide_duplicate" => Edit::Duplicate {
            index: slide_arg(a, "slide")?,
        },
        "insert_picture" => Edit::Picture {
            index: slide_arg(a, "slide")?,
            src: need("src")?,
            caption: text_arg(a, "caption"),
            opts: picture_opts(a),
        },
        "insert_table" => {
            let headers = strings(a, "headers");
            let rows = a
                .arr("rows")
                .iter()
                .map(|r| match r {
                    Json::Arr(cells) => cells
                        .iter()
                        .map(|c| match c {
                            Json::Str(s) => s.clone(),
                            Json::Num(n) => format!("{n}"),
                            _ => String::new(),
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .collect();
            Edit::Table {
                index: slide_arg(a, "slide")?,
                headers,
                rows,
                caption: text_arg(a, "caption"),
                chart: chart_arg(a)?,
            }
        }
        "title_align" => Edit::TitleAlign {
            index: slide_arg(a, "slide")?,
            x: text_arg(a, "x"),
            y: text_arg(a, "y"),
        },
        _ => return Ok(None),
    };
    Ok(Some(edit))
}

/// The chart arguments of `insert_table`, checked against what the parser
/// understands. Chart options without `chart` are refused.
fn chart_arg(a: &Json) -> Result<Option<Chart>, String> {
    let options = [
        "horizontal",
        "stacked",
        "title",
        "xlabel",
        "ylabel",
        "colors",
        "color_type",
        "legend",
    ];
    let Some(kind) = text_arg(a, "chart") else {
        if let Some(o) = options.iter().find(|o| a.get(o).is_some()) {
            return Err(format!(
                "{o} is a chart option; also pass chart (for example bar)."
            ));
        }
        return Ok(None);
    };
    let kind = kind.trim().to_ascii_lowercase();
    if !CHART_KINDS.contains(&kind.as_str()) {
        return Err(format!(
            "{kind} is not a chart type. Use one of: {}.",
            CHART_KINDS.join(", ")
        ));
    }
    let colors: Vec<String> = strings(a, "colors")
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    if let Some(bad) = colors
        .iter()
        .find(|c| crate::parse::parse_colors(c).len() != 1)
    {
        return Err(format!(
            "{bad} is not a chart colour. Use hex such as #E63946, or a theme accent such as accent 1."
        ));
    }
    let color_type = match text_arg(a, "color_type").map(|t| t.trim().to_ascii_lowercase()) {
        None => None,
        Some(t) if matches!(t.as_str(), "categorical" | "sequential" | "diverging") => Some(t),
        Some(t) => {
            return Err(format!(
                "{t} is not a colour type. Use categorical, sequential or diverging."
            ))
        }
    };
    Ok(Some(Chart {
        kind,
        horizontal: a.bool("horizontal") == Some(true),
        stacked: a.bool("stacked") == Some(true),
        title: text_arg(a, "title"),
        xlabel: text_arg(a, "xlabel"),
        ylabel: text_arg(a, "ylabel"),
        colors,
        color_type,
        legend: a.bool("legend"),
    }))
}

/// Array of strings under `key`, skipping other values.
pub fn strings(a: &Json, key: &str) -> Vec<String> {
    a.arr(key)
        .iter()
        .filter_map(|v| match v {
            Json::Str(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Picture options from tool arguments, as `key: value` lines.
pub fn picture_opts(a: &Json) -> Vec<(String, String)> {
    let mut opts = Vec::new();
    if let Some(size) = text_arg(a, "size") {
        opts.push(("size".to_string(), size));
    }
    if let Some(s) = a.num("scale") {
        let (lo, hi) = crate::parse::SCALE_RANGE;
        let pct = (s.clamp(lo as f64 * 100.0, hi as f64 * 100.0)).round() as i64;
        if pct != 100 {
            opts.push(("scale".to_string(), format!("{pct}%")));
        }
    }
    if let Some(x) = text_arg(a, "x") {
        opts.push(("x".to_string(), x));
    }
    if let Some(y) = text_arg(a, "y") {
        opts.push(("y".to_string(), y));
    }
    if a.bool("background") == Some(true) {
        opts.push(("background".to_string(), "true".to_string()));
    }
    if let Some(o) = a.num("opacity") {
        opts.push(("opacity".to_string(), format!("{o}")));
    }
    if let Some(f) = text_arg(a, "filter") {
        opts.push(("filter".to_string(), f));
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# One\n\n\tfirst\n\n---\n\n# Two\n\n```\n---\n```\n\n---\n\n# Three";

    #[test]
    fn title_align_writes_option_lines() {
        let t = "# Talk\nx: right\n\tSaturngod\n\nNotes\n\n---\n\n## Part";
        let set = |text: &str, x: Option<&str>, y: Option<&str>| {
            apply(
                text,
                &Edit::TitleAlign {
                    index: 0,
                    x: x.map(String::from),
                    y: y.map(String::from),
                },
            )
        };
        // y is added, x is kept
        let out = set(t, None, Some("bottom")).unwrap();
        assert!(out.starts_with("# Talk\nx: right\ny: bottom\n\tSaturngod\n"));
        let s = crate::parse::parse_document(&out);
        assert_eq!(s[0].title_x, crate::parse::Align::Right);
        assert_eq!(s[0].title_y, crate::parse::VAlign::Bottom);
        // defaults remove the lines
        let out = set(&out, Some("left"), Some("middle")).unwrap();
        assert!(out.starts_with("# Talk\n\tSaturngod\n"));
        // errors
        assert!(set(t, Some("up"), None).is_err());
        assert!(set(t, None, None).is_err());
        let two = apply(
            t,
            &Edit::TitleAlign {
                index: 1,
                x: Some("right".into()),
                y: None,
            },
        );
        assert!(two.is_err());
        let edit = parse_edit(
            "title_align",
            &Json::parse(r#"{"slide": 1, "x": "center"}"#).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            edit,
            Edit::TitleAlign {
                index: 0,
                x: Some("center".into()),
                y: None
            }
        );
    }

    #[test]
    fn bodies_match_parse_document() {
        let slides = crate::parse::parse_document(SAMPLE);
        assert_eq!(bodies(SAMPLE).len(), slides.len());
        assert_eq!(
            slides.len(),
            3,
            "a dash line inside a code fence is not a break"
        );
    }

    #[test]
    fn join_round_trips_bodies() {
        let b = bodies(SAMPLE);
        assert_eq!(bodies(&join(&b)), b);
    }

    #[test]
    fn titles_come_from_headings() {
        assert_eq!(title_of("\t\n# Hello  \n\ttext"), "Hello");
        assert_eq!(title_of("\nplain line\n"), "plain line");
    }

    #[test]
    fn add_update_delete_move_duplicate() {
        let t = apply(
            SAMPLE,
            &Edit::Add {
                at: None,
                md: "# Four".into(),
            },
        )
        .unwrap();
        assert_eq!(bodies(&t).len(), 4);
        assert_eq!(title_of(&bodies(&t)[3]), "Four");

        let t = apply(
            SAMPLE,
            &Edit::Update {
                index: 0,
                md: "# Uno".into(),
            },
        )
        .unwrap();
        assert_eq!(title_of(&bodies(&t)[0]), "Uno");

        let t = apply(SAMPLE, &Edit::Delete { index: 1 }).unwrap();
        assert_eq!(bodies(&t).len(), 2);
        assert_eq!(title_of(&bodies(&t)[1]), "Three");

        let t = apply(SAMPLE, &Edit::Move { from: 2, to: 0 }).unwrap();
        assert_eq!(title_of(&bodies(&t)[0]), "Three");

        let t = apply(SAMPLE, &Edit::Duplicate { index: 0 }).unwrap();
        assert_eq!(bodies(&t).len(), 4);
    }

    #[test]
    fn bad_numbers_and_breaks_are_refused() {
        assert!(apply(
            SAMPLE,
            &Edit::Update {
                index: 9,
                md: "x".into()
            }
        )
        .is_err());
        assert!(apply(
            SAMPLE,
            &Edit::Add {
                at: None,
                md: "a\n---\nb".into()
            }
        )
        .is_err());
    }

    #[test]
    fn picture_scale_argument() {
        let opts = |json: &str| picture_opts(&Json::parse(json).unwrap());
        let scale = |json: &str| {
            opts(json)
                .into_iter()
                .find(|(k, _)| k == "scale")
                .map(|(_, v)| v)
        };
        assert_eq!(scale(r#"{"scale": 50}"#).as_deref(), Some("50%"));
        assert_eq!(scale(r#"{"scale": 5}"#).as_deref(), Some("25%"));
        assert_eq!(scale(r#"{"scale": 900}"#).as_deref(), Some("200%"));
        assert_eq!(scale(r#"{"scale": 100}"#), None);
        assert_eq!(scale("{}"), None);
    }

    #[test]
    fn picture_and_table_blocks() {
        let t = apply(
            SAMPLE,
            &Edit::Picture {
                index: 0,
                src: "/assets/my photo.jpg".into(),
                caption: Some("Hi [x]".into()),
                opts: vec![("size".into(), "contain".into())],
            },
        )
        .unwrap();
        assert!(t.contains("![Hi x](/assets/my%20photo.jpg)\nsize: contain"));
        let slides = crate::parse::parse_document(&t);
        assert_eq!(slides.len(), 3);

        let bar = Chart {
            kind: "bar".into(),
            ..Default::default()
        };
        let block = table_block(
            &["A".into(), "B|".into()],
            &[vec!["1".into(), "2".into()]],
            None,
            Some(&bar),
        );
        assert_eq!(block, "| A | B\\| |\n| --- | --- |\n| 1 | 2 |\nChart: bar");
    }

    /// The first chart the parser finds on slide `i` of a script.
    fn chart_on(text: &str, i: usize) -> crate::parse::ChartSpec {
        let slides = crate::parse::parse_document(text);
        slides[i]
            .groups
            .iter()
            .flatten()
            .find_map(|b| match b {
                crate::parse::Block::Chart(c) => Some(c.clone()),
                _ => None,
            })
            .expect("a chart on the slide")
    }

    fn table_args(extra: &str) -> Json {
        Json::parse(&format!(
            r#"{{"slide": 1, "headers": ["Quarter", "Sales", "Costs"],
                "rows": [["Q1", "12", "8"], ["Q2", "18", "9"]]{extra}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn every_chart_type_is_drawn_as_that_type() {
        for kind in CHART_KINDS {
            let a = table_args(&format!(r#", "chart": "{kind}""#));
            let edit = parse_edit("insert_table", &a).unwrap().unwrap();
            let t = apply(SAMPLE, &edit).unwrap();
            let spec = chart_on(&t, 0);
            assert_eq!(
                Some(spec.kind),
                crate::parse::ChartKind::from_word(kind),
                "{kind}"
            );
        }
    }

    #[test]
    fn chart_options_are_read_back() {
        let a = table_args(
            r##", "chart": "Bar", "horizontal": true, "stacked": true,
                "title": "Sales by quarter", "xlabel": "Quarter", "ylabel": "USD",
                "caption": "Source: finance", "colors": ["#E63946", "accent 2"],
                "color_type": "sequential", "legend": false"##,
        );
        let edit = parse_edit("insert_table", &a).unwrap().unwrap();
        let t = apply(SAMPLE, &edit).unwrap();
        let c = chart_on(&t, 0);
        assert_eq!(c.kind, crate::parse::ChartKind::Bar);
        assert!(c.horizontal && c.stacked);
        assert_eq!(c.title.as_deref(), Some("Sales by quarter"));
        assert_eq!(c.xlabel.as_deref(), Some("Quarter"));
        assert_eq!(c.ylabel.as_deref(), Some("USD"));
        assert_eq!(c.table.caption.as_deref(), Some("Source: finance"));
        assert_eq!(
            c.colors,
            vec![
                crate::parse::ChartColor::Rgb([0xE6, 0x39, 0x46]),
                crate::parse::ChartColor::Accent(2)
            ]
        );
        assert_eq!(c.color_type, crate::parse::ColorType::Sequential);
        assert!(!c.legend);
        assert_eq!(c.table.rows.len(), 2);
    }

    #[test]
    fn diverging_colours_and_plain_table_caption() {
        let a = table_args(r#", "chart": "line", "color_type": "diverging""#);
        let t = apply(SAMPLE, &parse_edit("insert_table", &a).unwrap().unwrap()).unwrap();
        assert_eq!(
            chart_on(&t, 0).color_type,
            crate::parse::ColorType::Differential
        );
        assert!(chart_on(&t, 0).legend, "the legend shows unless turned off");

        let a = table_args(r#", "caption": "Plain [table]""#);
        let t = apply(SAMPLE, &parse_edit("insert_table", &a).unwrap().unwrap()).unwrap();
        assert!(t.contains("\n[Plain table]"));
        let slides = crate::parse::parse_document(&t);
        let caption = slides[0].groups.iter().flatten().find_map(|b| match b {
            crate::parse::Block::Table(tb) => tb.caption.clone(),
            _ => None,
        });
        assert_eq!(caption.as_deref(), Some("Plain table"));
    }

    #[test]
    fn bad_chart_arguments_are_refused() {
        let bad = [
            r#", "chart": "histogram""#,
            r#", "chart": "bar", "colors": ["blue"]"#,
            r#", "chart": "bar", "color_type": "rainbow""#,
            r#", "stacked": true"#,
        ];
        for extra in bad {
            assert!(
                parse_edit("insert_table", &table_args(extra)).is_err(),
                "{extra}"
            );
        }
    }
}
