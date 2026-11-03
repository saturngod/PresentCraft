//! Find bar for the Markdown editor (⌘F / Ctrl+F): case-insensitive search in
//! the open document, one match at a time. The found match flashes yellow
//! before it settles into the selection colour.

use eframe::egui;
use egui::text::LayoutJob;
use egui::Color32;

/// How long the current match flashes before it looks selected.
pub const FLASH_SECS: f64 = 0.9;

/// A match: char range and byte range in the editor text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub chars: (usize, usize),
    bytes: (usize, usize),
}

/// Search runs only when the query changes or the user steps to the next
/// or previous match, never every frame.
#[derive(Default)]
pub struct Find {
    pub open: bool,
    pub query: String,
    pub current: Option<Hit>,
    /// The last search found nothing.
    pub not_found: bool,
    /// When the current match last changed (starts the flash).
    pub flash_since: f64,
    /// Put the keyboard in the search field (and select its text) next frame.
    pub focus_field: bool,
    /// Scroll the editor to the current match next frame.
    pub scroll: bool,
    /// The search field or editor had the keyboard last frame (egui drops
    /// focus on Esc before the frame runs, so Esc is checked against this).
    pub had_focus: bool,
    searched: String,
}

impl Find {
    /// Called each frame while the bar is open. Searches forward from
    /// `cursor` only when the query changed; otherwise just drops the
    /// current match if editing changed the text under it.
    pub fn refresh(&mut self, text: &str, cursor: usize, now: f64) {
        if self.query != self.searched {
            self.searched = self.query.clone();
            self.current = None;
            self.go(search(text, &self.query, cursor, true), now);
        } else if let Some(h) = self.current {
            let still = text
                .get(h.bytes.0..h.bytes.1)
                .is_some_and(|s| same(s, &self.query));
            if !still {
                self.current = None;
            }
        }
    }

    /// Find the next (or previous) match after the current one, wrapping
    /// around the document. With no current match, start at `cursor`.
    pub fn step(&mut self, text: &str, cursor: usize, forward: bool, now: f64) {
        let from = match self.current {
            Some(h) if forward => h.chars.1,
            Some(h) => h.chars.0,
            None => cursor,
        };
        self.go(search(text, &self.query, from, forward), now);
    }

    fn go(&mut self, hit: Option<Hit>, now: f64) {
        self.not_found = hit.is_none() && !self.query.is_empty();
        if hit.is_some() {
            self.current = hit;
            self.flash_since = now;
            self.scroll = true;
        }
    }

    pub fn current_range(&self) -> Option<(usize, usize)> {
        self.current.map(|h| h.chars)
    }

    /// Highlight for the current match only: (start char, end char, colour).
    /// It flashes yellow, then settles into the selection colour.
    pub fn marks(&self, now: f64, selection: Color32) -> Vec<(usize, usize, Color32)> {
        let Some(h) = self.current.filter(|_| self.open) else {
            return Vec::new();
        };
        let flash = Color32::from_rgb(255, 222, 40);
        let t = ((now - self.flash_since) / FLASH_SECS).clamp(0.0, 1.0) as f32;
        // hold the yellow, then ease into the selection colour
        let k = ((t - 0.45) / 0.55).clamp(0.0, 1.0);
        let k = k * k * (3.0 - 2.0 * k);
        vec![(h.chars.0, h.chars.1, lerp_color(flash, selection, k))]
    }

    pub fn flashing(&self, now: f64) -> bool {
        self.open && self.current.is_some() && now - self.flash_since < FLASH_SECS
    }
}

fn lerp_color(a: Color32, b: Color32, k: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgba_unmultiplied(
        m(a.r(), b.r()),
        m(a.g(), b.g()),
        m(a.b(), b.b()),
        m(a.a(), b.a()),
    )
}

fn fold(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

fn same(a: &str, query: &str) -> bool {
    a.chars().map(fold).eq(query.chars().map(fold))
}

/// The nearest case-insensitive match of `query` starting at or after char
/// `from` (forward) or before it (backward), wrapping around the text.
pub fn search(text: &str, query: &str, from: usize, forward: bool) -> Option<Hit> {
    let q: Vec<char> = query.chars().map(fold).collect();
    if q.is_empty() {
        return None;
    }
    // (byte, folded char), plus the end of the text
    let t: Vec<(usize, char)> = text.char_indices().map(|(b, c)| (b, fold(c))).collect();
    if t.len() < q.len() {
        return None;
    }
    let last = t.len() - q.len();
    let at = |i: usize| t[i..i + q.len()].iter().map(|x| x.1).eq(q.iter().copied());
    let from = from.min(t.len());
    let found = if forward {
        (from..=last).chain(0..from.min(last + 1)).find(|&i| at(i))
    } else {
        (0..from.min(last + 1))
            .rev()
            .chain((from..=last).rev())
            .find(|&i| at(i))
    }?;
    let end = found + q.len();
    let end_byte = t.get(end).map_or(text.len(), |x| x.0);
    Some(Hit {
        chars: (found, end),
        bytes: (t[found].0, end_byte),
    })
}

/// Give the given char ranges a background colour in `job`, splitting
/// sections where needed. Ranges must be sorted and not overlap. Positions
/// are chars, mapped through the job's own text: the editor shows some
/// characters differently (tabs as arrows, `---` as long dashes), so its
/// bytes don't line up with the source text.
pub fn apply_marks(job: &mut LayoutJob, marks: &[(usize, usize, Color32)]) {
    if marks.is_empty() {
        return;
    }
    let text = job.text.clone();
    let mut bytes: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    bytes.push(text.len());
    let ranges: Vec<(usize, usize, Color32)> = marks
        .iter()
        .filter_map(|&(a, b, c)| Some((*bytes.get(a)?, *bytes.get(b)?, c)))
        .collect();
    let mut out = Vec::with_capacity(job.sections.len() + ranges.len() * 2);
    let mut r = 0usize;
    for sec in std::mem::take(&mut job.sections) {
        let (s0, s1) = (sec.byte_range.start.0, sec.byte_range.end.0);
        let mut pos = s0;
        while pos < s1 {
            while r < ranges.len() && ranges[r].1 <= pos {
                r += 1;
            }
            let (end, bg) = match ranges.get(r) {
                Some(&(a, b, c)) if a <= pos => (b.min(s1), Some(c)),
                Some(&(a, _, _)) => (a.min(s1), None),
                None => (s1, None),
            };
            let mut piece = sec.clone();
            piece.byte_range = egui::text::ByteIndex(pos)..egui::text::ByteIndex(end);
            if pos != s0 {
                piece.leading_space = 0.0;
            }
            if let Some(c) = bg {
                piece.format.background = c;
            }
            out.push(piece);
            pos = end;
        }
        if s0 == s1 {
            out.push(sec);
        }
    }
    job.sections = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_forward_and_back_with_wrap() {
        let r = |h: Option<Hit>| h.map(|h| h.chars);
        assert_eq!(r(search("Table tAbLe", "table", 1, true)), Some((6, 11)));
        assert_eq!(r(search("Table tAbLe", "table", 7, true)), Some((0, 5)));
        assert_eq!(r(search("Table tAbLe", "table", 6, false)), Some((0, 5)));
        assert_eq!(r(search("Table tAbLe", "table", 0, false)), Some((6, 11)));
        assert_eq!(r(search("ကခ table", "TABLE", 0, true)), Some((3, 8)));
        assert!(search("abc", "", 0, true).is_none());
        assert!(search("abc", "x", 0, true).is_none());
    }

    #[test]
    fn marks_split_sections() {
        let text = "hello world";
        let mut job = LayoutJob::single_section(text.into(), Default::default());
        apply_marks(&mut job, &[(6, 11, Color32::RED)]);
        assert_eq!(job.sections.len(), 2);
        assert_eq!(job.sections[1].byte_range.start.0, 6);
        assert_eq!(job.sections[1].byte_range.end.0, 11);
        assert_eq!(job.sections[1].format.background, Color32::RED);

        // shown text with wider characters than the source still lines up by char
        let mut job = LayoutJob::single_section("———\nslide".into(), Default::default());
        apply_marks(&mut job, &[(4, 9, Color32::RED)]);
        assert_eq!(job.sections[1].byte_range.start.0, "———\n".len());
    }

    #[test]
    fn query_change_picks_match_after_cursor() {
        let text = "a b a b a";
        let mut f = Find {
            query: "a".into(),
            ..Default::default()
        };
        f.refresh(text, 3, 0.0);
        assert_eq!(f.current_range(), Some((4, 5)));
        f.step(text, 0, true, 1.0);
        assert_eq!(f.current_range(), Some((8, 9)));
        f.step(text, 0, true, 2.0);
        assert_eq!(f.current_range(), Some((0, 1)));
        f.step(text, 0, false, 3.0);
        assert_eq!(f.current_range(), Some((8, 9)));
        // editing the text under the match drops it
        f.refresh("a b a b x", 0, 4.0);
        assert_eq!(f.current_range(), None);
    }
}
