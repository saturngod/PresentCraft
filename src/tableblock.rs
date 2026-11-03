//! The table/chart block under the cursor, and edits to it that never touch
//! the data: change chart type, caption, axis labels, colours, reformat,
//! transpose. Used by the floating table toolbar in the editor.

use crate::parse::ChartKind;

#[derive(Clone, Debug)]
pub struct TableBlock {
    /// First table line (line index in the document).
    pub start: usize,
    /// One past the last `|` line.
    pub table_end: usize,
    /// One past the last settings line (Chart:, yLabel:, [caption] …).
    pub end: usize,
    pub tab: bool,
}

use crate::parse::CHART_KEYS as META_KEYS;

fn strip(l: &str) -> &str {
    l.strip_prefix('\t').unwrap_or(l).trim_start()
}

fn is_table(l: &str) -> bool {
    strip(l).starts_with('|')
}

fn meta_key(l: &str) -> Option<String> {
    let t = strip(l).trim();
    if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
        return Some("[caption]".into());
    }
    let (k, _) = t.split_once(':')?;
    let k = k.trim().to_ascii_lowercase();
    META_KEYS.contains(&k.as_str()).then_some(k)
}

pub fn lines_of(text: &str) -> Vec<String> {
    text.split('\n').map(|s| s.to_string()).collect()
}

pub fn line_of_char(text: &str, ch: usize) -> usize {
    text.chars().take(ch).filter(|c| *c == '\n').count()
}

pub fn char_of_line(lines: &[String], line: usize) -> usize {
    lines.iter().take(line).map(|l| l.chars().count() + 1).sum()
}

pub fn find(lines: &[String], cur: usize) -> Option<TableBlock> {
    if cur >= lines.len() {
        return None;
    }
    let mut i = cur;
    if !is_table(&lines[i]) {
        while i > 0 && meta_key(&lines[i]).is_some() {
            i -= 1;
        }
        if !is_table(&lines[i]) {
            return None;
        }
    }
    while i > 0 && is_table(&lines[i - 1]) {
        i -= 1;
    }
    let start = i;
    let mut e = start;
    while e < lines.len() && is_table(&lines[e]) {
        e += 1;
    }
    let table_end = e;
    while e < lines.len() && meta_key(&lines[e]).is_some() {
        e += 1;
    }
    Some(TableBlock {
        start,
        table_end,
        end: e,
        tab: lines[start].starts_with('\t'),
    })
}

impl TableBlock {
    fn find_meta(&self, lines: &[String], key: &str) -> Option<usize> {
        (self.table_end..self.end).find(|&i| meta_key(&lines[i]).as_deref() == Some(key))
    }

    pub fn get(&self, lines: &[String], key: &str) -> Option<String> {
        let i = self.find_meta(lines, key)?;
        let t = strip(&lines[i]).trim();
        if key == "[caption]" {
            return Some(t[1..t.len() - 1].to_string());
        }
        t.split_once(':').map(|(_, v)| v.trim().to_string())
    }

    /// Sets (or removes with None) a settings line under the table.
    pub fn set(
        &mut self,
        lines: &mut Vec<String>,
        key: &str,
        display_key: &str,
        value: Option<&str>,
    ) {
        let pre = if self.tab { "\t" } else { "" };
        let new_line = value.map(|v| {
            if key == "[caption]" {
                format!("{pre}[{v}]")
            } else {
                format!("{pre}{display_key}: {v}")
            }
        });
        match (self.find_meta(lines, key), new_line) {
            (Some(i), Some(l)) => lines[i] = l,
            (Some(i), None) => {
                lines.remove(i);
                self.end -= 1;
            }
            (None, Some(l)) => {
                // caption goes right under the table, other settings at the end
                let at = if key == "[caption]" {
                    self.table_end
                } else {
                    self.end
                };
                lines.insert(at, l);
                self.end += 1;
            }
            (None, None) => {}
        }
    }

    /// Current chart: (kind, horizontal, stacked); None = plain table.
    pub fn chart(&self, lines: &[String]) -> Option<(ChartKind, bool, bool)> {
        let v = self
            .get(lines, "chart")
            .or_else(|| self.get(lines, "type"))?;
        let mut kind = None;
        let (mut h, mut s) = (false, false);
        for w in v
            .split(|c: char| c == ',' || c.is_whitespace() || c == '-')
            .filter(|w| !w.is_empty())
        {
            match w.to_ascii_lowercase().as_str() {
                "horizontal" => h = true,
                "stacked" => s = true,
                o => {
                    if kind.is_none() {
                        kind = ChartKind::from_word(o)
                    }
                }
            }
        }
        if let Some(v) = self.get(lines, "bar-type") {
            s = v.eq_ignore_ascii_case("stacked");
        }
        if let Some(v) = self.get(lines, "orientation") {
            h = v.eq_ignore_ascii_case("horizontal");
        }
        Some((kind.unwrap_or(ChartKind::Bar), h, s))
    }

    pub fn set_chart(&mut self, lines: &mut Vec<String>, chart: Option<(ChartKind, bool, bool)>) {
        // Bars keep stacking and orientation on their own lines
        // (`Bar-Type:`, `Orientation:`), like iA Presenter writes them.
        let bar = chart.filter(|c| c.0 == ChartKind::Bar);
        self.set(
            lines,
            "bar-type",
            "Bar-Type",
            bar.filter(|c| c.2).map(|_| "stacked"),
        );
        self.set(
            lines,
            "orientation",
            "Orientation",
            bar.filter(|c| c.1).map(|_| "horizontal"),
        );
        if chart.is_none() {
            self.set(lines, "color-type", "Color-Type", None);
        }
        let value = chart.map(|(k, _, s)| {
            let mut v = kind_word(k).to_string();
            if s && k == ChartKind::Area {
                v.push_str(", Stacked");
            }
            v
        });
        if self.find_meta(lines, "type").is_some() && self.find_meta(lines, "chart").is_none() {
            self.set(lines, "type", "Type", value.as_deref());
        } else {
            self.set(lines, "chart", "Chart", value.as_deref());
        }
    }

    fn cells(&self, lines: &[String]) -> (Vec<Vec<String>>, Vec<String>) {
        let mut rows = Vec::new();
        let mut aligns = Vec::new();
        for (k, l) in lines[self.start..self.table_end].iter().enumerate() {
            let t = strip(l).trim();
            let t = t.strip_prefix('|').unwrap_or(t);
            let t = t.strip_suffix('|').unwrap_or(t);
            let cells: Vec<String> = t.split('|').map(|c| c.trim().to_string()).collect();
            if k == 1
                && cells
                    .iter()
                    .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, ':' | '-' | ' ')))
            {
                aligns = cells;
                continue;
            }
            rows.push(cells);
        }
        (rows, aligns)
    }

    fn write(&mut self, lines: &mut Vec<String>, rows: Vec<Vec<String>>, aligns: Vec<String>) {
        let n = rows
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(0)
            .max(aligns.len())
            .max(1);
        let mut widths = vec![3usize; n];
        for r in &rows {
            for (j, c) in r.iter().enumerate() {
                widths[j] = widths[j].max(c.chars().count());
            }
        }
        let pre = if self.tab { "\t" } else { "" };
        let mut out = Vec::new();
        for (k, r) in rows.iter().enumerate() {
            let mut l = format!("{pre}|");
            for (j, w) in widths.iter().enumerate() {
                let c = r.get(j).map(|s| s.as_str()).unwrap_or("");
                l.push_str(&format!(" {c:<w$} |"));
            }
            out.push(l);
            if k == 0 {
                let mut a = format!("{pre}|");
                for (j, w) in widths.iter().enumerate() {
                    let spec = aligns.get(j).map(|s| s.as_str()).unwrap_or(":--");
                    let left = spec.starts_with(':');
                    let right = spec.ends_with(':') && spec.len() > 1;
                    let dashes = w - left as usize - right as usize;
                    a.push_str(&format!(
                        " {}{}{} |",
                        if left { ":" } else { "" },
                        "-".repeat(dashes.max(1)),
                        if right { ":" } else { "" }
                    ));
                }
                out.push(a);
            }
        }
        let old = self.table_end - self.start;
        let new = out.len();
        lines.splice(self.start..self.table_end, out);
        self.table_end = self.start + new;
        self.end = self.end + new - old;
    }

    /// Lines up the columns.
    pub fn reformat(&mut self, lines: &mut Vec<String>) {
        let (rows, aligns) = self.cells(lines);
        self.write(lines, rows, aligns);
    }

    /// Swaps rows and columns (the data itself is kept).
    pub fn transpose(&mut self, lines: &mut Vec<String>) {
        let (rows, _) = self.cells(lines);
        let n = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let t: Vec<Vec<String>> = (0..n)
            .map(|j| {
                rows.iter()
                    .map(|r| r.get(j).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();
        self.write(lines, t, Vec::new());
    }

    pub fn to_csv(&self, lines: &[String]) -> String {
        let (rows, _) = self.cells(lines);
        rows.iter()
            .map(|r| {
                r.iter()
                    .map(|c| {
                        if c.contains(',') {
                            format!("\"{c}\"")
                        } else {
                            c.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn kind_word(k: ChartKind) -> &'static str {
    match k {
        ChartKind::Bar => "Bar",
        ChartKind::Line => "Line",
        ChartKind::Spline => "Spline",
        ChartKind::Area => "Area",
        ChartKind::Step => "Step",
        ChartKind::Pie => "Pie",
        ChartKind::Donut => "Donut",
        ChartKind::Radar => "Radar",
        ChartKind::Gauge => "Gauge",
        ChartKind::Bubble => "Bubble",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_type_keeps_data() {
        let text = "### T\n| | A |\n|:--|:--|\n| x | 1 |\nChart: Bar\nyLabel: U\n\nnote";
        let mut lines = lines_of(text);
        let mut b = find(&lines, 3).unwrap();
        assert_eq!((b.start, b.table_end, b.end), (1, 4, 6));
        b.set_chart(&mut lines, Some((ChartKind::Line, false, false)));
        assert_eq!(lines[4], "Chart: Line");
        assert_eq!(lines[3], "| x | 1 |");
        b.set_chart(&mut lines, None);
        assert_eq!(lines[4], "yLabel: U");
        b.set_chart(&mut lines, Some((ChartKind::Bar, true, true)));
        assert_eq!(b.chart(&lines), Some((ChartKind::Bar, true, true)));
        assert!(lines.contains(&"Bar-Type: stacked".to_string()));
        assert!(lines.contains(&"Orientation: horizontal".to_string()));
        b.set_chart(&mut lines, Some((ChartKind::Bar, false, false)));
        assert!(!lines.iter().any(|l| l.starts_with("Bar-Type") || l.starts_with("Orientation")));
        b.set_chart(&mut lines, None);
        b.set(&mut lines, "[caption]", "", Some("Cap"));
        assert_eq!(lines[4], "[Cap]");
        b.transpose(&mut lines);
        assert!(lines[1].contains('x'));
    }
}
