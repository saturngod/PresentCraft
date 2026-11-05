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
        chart: Option<String>,
    },
}

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

/// A Markdown table, with an optional `Chart: <kind>` line under it.
pub fn table_block(headers: &[String], rows: &[Vec<String>], chart: Option<&str>) -> String {
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
    if let Some(kind) = chart {
        lines.push(format!("Chart: {kind}"));
    }
    lines.join("\n")
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
            chart,
        } => {
            let i = check(*index)?;
            if headers.is_empty() {
                return Err("A table needs at least one column heading.".to_string());
            }
            let block = table_block(headers, rows, chart.as_deref());
            b[i] = append_block(&b[i], &block);
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
                chart: text_arg(a, "chart"),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(edit))
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

        let block = table_block(
            &["A".into(), "B|".into()],
            &[vec!["1".into(), "2".into()]],
            Some("bar"),
        );
        assert_eq!(block, "| A | B\\| |\n| --- | --- |\n| 1 | 2 |\nChart: bar");
    }
}
