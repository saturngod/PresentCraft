//! Presenter view: while the presentation window is open, the editor
//! window shows the talk as a list of notes in large text. Each slide's
//! visible text is one note and each speaker-note paragraph is another.
//! ↑/↓ move between notes; moving onto another slide's note changes the
//! slide in the presentation window, and the other way round.

use crate::parse::{Block, Slide};
use crate::{icon_button, slide_color, theme, App, Icon, ACCENT_BLUE, TOP_BAR};
use eframe::egui;
use egui::{pos2, vec2, Color32, FontId, Key, Modifiers, Rect, Sense, Stroke};

/// One line of a slide's visible text.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub bold: bool,
    /// A stand-in for something that isn't text (a picture, a table).
    pub placeholder: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NoteKind {
    /// What the audience sees on the slide.
    Slide(Vec<Line>),
    /// A paragraph of speaker notes.
    Speaker(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub slide: usize,
    pub kind: NoteKind,
}

fn line(text: impl Into<String>, bold: bool) -> Line {
    Line {
        text: text.into(),
        bold,
        placeholder: false,
    }
}

fn placeholder(text: &str) -> Line {
    Line {
        text: text.into(),
        bold: false,
        placeholder: true,
    }
}

fn slide_lines(slide: &Slide) -> Vec<Line> {
    let mut out = Vec::new();
    for block in slide.groups.iter().flatten() {
        match block {
            Block::Heading { level, text } => out.push(line(text.clone(), *level <= 2)),
            Block::Paragraph(t) | Block::Quote(t) => out.extend(
                t.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| line(l, false)),
            ),
            Block::List(items) => {
                for it in items {
                    let mark = match (&it.number, it.task) {
                        (Some(n), _) => format!("{n}. "),
                        (None, Some(true)) => "☑ ".into(),
                        (None, Some(false)) => "☐ ".into(),
                        (None, None) => "• ".into(),
                    };
                    let indent = "    ".repeat(it.indent);
                    out.push(line(format!("{indent}{mark}{}", it.text), false));
                }
            }
            Block::Image(_) => out.push(placeholder("Picture")),
            Block::Table(_) => out.push(placeholder("Table")),
            Block::Chart(_) => out.push(placeholder("Chart")),
            Block::Code(..) => out.push(placeholder("Code")),
            Block::Math(m) => out.push(line(crate::math::plain(m), false)),
        }
    }
    if out.is_empty() {
        out.push(placeholder(if slide.backgrounds.is_empty() {
            "Empty slide"
        } else {
            "Picture"
        }));
    }
    out
}

/// Every slide becomes a note, followed by one note per speaker-note
/// paragraph, so each slide can be reached even without notes.
pub fn build_notes(slides: &[Slide]) -> Vec<Note> {
    let mut notes = Vec::new();
    for (i, s) in slides.iter().enumerate() {
        notes.push(Note {
            slide: i,
            kind: NoteKind::Slide(slide_lines(s)),
        });
        for n in &s.notes {
            notes.push(Note {
                slide: i,
                kind: NoteKind::Speaker(n.clone()),
            });
        }
    }
    notes
}

/// Index of the first note of `slide`.
pub fn first_note_of(notes: &[Note], slide: usize) -> usize {
    notes.iter().position(|n| n.slide == slide).unwrap_or(0)
}

/// Talk time as HH:MM:SS.
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

const GUTTER: f32 = 150.0;
const COLUMN_MAX: f32 = 1100.0;
const PAD: f32 = 22.0;

impl App {
    /// Seconds the talk has been running (pauses excluded).
    fn talk_seconds(&self, now: f64) -> f64 {
        self.talk_elapsed + self.talk_since.map_or(0.0, |t| now - t)
    }

    fn select_note(&mut self, notes: &[Note], i: usize) {
        if notes.is_empty() {
            return;
        }
        self.present_note = i.min(notes.len() - 1);
        self.current = notes[self.present_note].slide;
        self.note_scroll = true;
    }

    fn select_slide_note(&mut self, notes: &[Note], slide: usize) {
        let last = self.slides.len().saturating_sub(1);
        let i = first_note_of(notes, slide.min(last));
        self.select_note(notes, i);
    }

    /// The editor window while presenting.
    pub(crate) fn presenter_view(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let pal = crate::settings::pal();
        let notes = build_notes(&self.slides);
        if notes.is_empty() {
            return;
        }
        // Follow slide changes made in the presentation window.
        self.present_note = self.present_note.min(notes.len() - 1);
        if notes[self.present_note].slide != self.current {
            let slide = self.current;
            self.select_slide_note(&notes, slide);
        }

        let (down, up, next, prev, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::ArrowRight)
                    || i.consume_key(Modifiers::NONE, Key::PageDown),
                i.consume_key(Modifiers::NONE, Key::ArrowLeft)
                    || i.consume_key(Modifiers::NONE, Key::PageUp),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if esc {
            self.stop_presenting(ctx);
            return;
        }
        if down && self.present_note + 1 < notes.len() {
            self.select_note(&notes, self.present_note + 1);
        }
        if up && self.present_note > 0 {
            self.select_note(&notes, self.present_note - 1);
        }
        if next {
            let s = self.current + 1;
            self.select_slide_note(&notes, s);
        }
        if prev && self.current > 0 {
            let s = self.current - 1;
            self.select_slide_note(&notes, s);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(pal.editor_bg))
            .show(ui, |ui| {
                let full = ui.max_rect();
                let top = Rect::from_min_size(full.min, vec2(full.width(), TOP_BAR));
                self.presenter_top_bar(ui, top);
                ui.painter().line_segment(
                    [
                        pos2(full.left(), top.bottom() - 0.5),
                        pos2(full.right(), top.bottom() - 0.5),
                    ],
                    Stroke::new(1.0_f32, pal.divider.gamma_multiply(0.6)),
                );
                let body = Rect::from_min_max(pos2(full.left(), top.bottom()), full.max);
                self.presenter_notes(ui, body, &notes);
                self.presenter_nav(ui, body, &notes);
            });
        if self.talk_since.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn presenter_top_bar(&mut self, ui: &mut egui::Ui, top: Rect) {
        let pal = crate::settings::pal();
        let p = ui.painter().clone();
        p.text(
            top.center(),
            egui::Align2::CENTER_CENTER,
            self.doc_name(),
            FontId::new(13.5, theme::family(false, true, false)),
            pal.title_grey,
        );

        // Stop: the primary action, a blue circle like Present.
        let stop =
            Rect::from_center_size(pos2(top.right() - 30.0, top.center().y), vec2(34.0, 34.0));
        let resp = ui
            .interact(stop, egui::Id::unique("presenter-stop"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("Stop Presenting ({})", crate::kbd("⌘↵")));
        let fill = if resp.hovered() {
            crate::hex(0x4C80FF)
        } else {
            ACCENT_BLUE
        };
        p.circle_filled(stop.center(), 17.0, fill);
        p.rect_filled(
            Rect::from_center_size(stop.center(), vec2(11.0, 11.0)),
            2.5,
            Color32::WHITE,
        );
        if resp.clicked() {
            let ctx = ui.ctx().clone();
            self.stop_presenting(&ctx);
            return;
        }

        // Pause / resume the timer.
        let now = ui.input(|i| i.time);
        let pause =
            Rect::from_center_size(pos2(stop.left() - 22.0, top.center().y), vec2(30.0, 26.0));
        let running = self.talk_since.is_some();
        let (icon, tip) = if running {
            (Icon::Pause, "Pause Timer")
        } else {
            (Icon::Play, "Resume Timer")
        };
        if icon_button(ui, pause, icon, tip).clicked() {
            if let Some(t) = self.talk_since.take() {
                self.talk_elapsed += now - t;
            } else {
                self.talk_since = Some(now);
            }
        }

        // Elapsed time pill.
        let label = clock(self.talk_seconds(now));
        let g = p.layout_no_wrap(label, FontId::proportional(13.0), pal.text_hi);
        let pill = Rect::from_min_size(
            pos2(
                pause.left() - 10.0 - g.size().x - 28.0,
                top.center().y - 15.0,
            ),
            vec2(g.size().x + 28.0, 30.0),
        );
        p.rect_filled(pill, 15.0, pal.pill);
        p.rect_stroke(
            pill,
            15.0,
            Stroke::new(1.0_f32, pal.border),
            egui::StrokeKind::Inside,
        );
        let color = if running { pal.text_hi } else { pal.text_dim };
        p.galley(pill.center() - g.size() / 2.0, g, color);
        ui.interact(pill, egui::Id::unique("presenter-clock"), Sense::hover())
            .on_hover_text(if running {
                "Talk time"
            } else {
                "Talk time (paused)"
            });
    }

    fn presenter_notes(&mut self, ui: &mut egui::Ui, body: Rect, notes: &[Note]) {
        let pal = crate::settings::pal();
        let dark = crate::settings::is_dark();
        let col_w = (body.width() - GUTTER - 60.0).clamp(240.0, COLUMN_MAX);
        let size = (col_w / 36.0).clamp(18.0, 30.0);
        let total = GUTTER + col_w;
        let left_off = ((body.width() - total) / 2.0).max(16.0);
        let sel = self.present_note;
        let scroll = std::mem::take(&mut self.note_scroll);

        let mut clicked = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
        egui::ScrollArea::vertical()
            .id_salt("presenter-notes")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(24.0);
                for (i, note) in notes.iter().enumerate() {
                    let is_sel = i == sel;
                    let on_slide = note.slide == self.current;
                    // Nearby notes read clearly, farther ones fade.
                    let d = i.abs_diff(sel) as f32;
                    let fade = if d <= 2.0 {
                        1.0
                    } else {
                        (1.0 - (d - 2.0) * 0.12).max(0.45)
                    };
                    let text_color = if is_sel {
                        pal.text_hi
                    } else {
                        pal.text_dim.gamma_multiply(fade)
                    };
                    let style = |bold: bool, placeholder: bool| crate::inline::Style {
                        size,
                        color: if placeholder {
                            text_color.gamma_multiply(0.7)
                        } else {
                            text_color
                        },
                        face: crate::fonts::Face::SANS,
                        bold,
                        italic: placeholder,
                        accent: if is_sel { ACCENT_BLUE } else { text_color },
                        line_height: 1.45,
                    };
                    let wrap = col_w - 2.0 * PAD;
                    let galleys: Vec<_> =
                        match &note.kind {
                            NoteKind::Slide(lines) => lines
                                .iter()
                                .map(|l| {
                                    ui.painter().layout_job(crate::inline::job(
                                        &l.text,
                                        &style(l.bold, l.placeholder),
                                        wrap,
                                    ))
                                })
                                .collect(),
                            NoteKind::Speaker(t) => vec![ui
                                .painter()
                                .layout_job(crate::inline::job(t, &style(false, false), wrap))],
                        };
                    let text_h: f32 = galleys.iter().map(|g| g.size().y).sum();
                    let is_slide = matches!(note.kind, NoteKind::Slide(_));
                    ui.add_space(if is_slide && i > 0 { 44.0 } else { 6.0 });
                    let h = text_h + if is_slide { 2.0 * PAD - 6.0 } else { 12.0 };
                    let (row, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
                    let x0 = row.left() + left_off;
                    let card = Rect::from_min_size(pos2(x0 + GUTTER, row.top()), vec2(col_w, h));
                    let p = ui.painter();

                    if is_slide {
                        // number badge and a small thumbnail in the gutter
                        let c = pos2(x0 + 18.0, card.top() + 16.0);
                        let (fill, num) = if on_slide {
                            (slide_color(note.slide), Color32::WHITE)
                        } else {
                            (pal.button, pal.text_dim)
                        };
                        p.circle_filled(c, 11.0, fill);
                        p.text(
                            c,
                            egui::Align2::CENTER_CENTER,
                            format!("{}", note.slide + 1),
                            FontId::new(
                                if note.slide + 1 >= 10 { 10.5 } else { 11.0 },
                                theme::family(false, true, false),
                            ),
                            num,
                        );
                        let tw = GUTTER - 56.0;
                        let thumb = Rect::from_min_size(
                            pos2(x0 + 38.0, card.top() + 6.0),
                            vec2(tw, tw / crate::render::ASPECT),
                        );
                        if ui.is_rect_visible(thumb) {
                            let tp = ui.painter().with_clip_rect(thumb.intersect(ui.clip_rect()));
                            self.paint_slide(&tp, thumb, note.slide);
                            ui.painter().rect_stroke(
                                thumb,
                                2.0,
                                Stroke::new(1.0_f32, pal.border),
                                egui::StrokeKind::Outside,
                            );
                        }
                        let p = ui.painter();
                        let fill = if is_sel {
                            ACCENT_BLUE.gamma_multiply(if dark { 0.28 } else { 0.30 })
                        } else if on_slide {
                            ACCENT_BLUE.gamma_multiply(if dark { 0.12 } else { 0.12 })
                        } else {
                            pal.sidebar_bg
                        };
                        p.rect_filled(card, 8.0, fill);
                        if is_sel {
                            p.rect_stroke(
                                card,
                                8.0,
                                Stroke::new(2.5_f32, ACCENT_BLUE),
                                egui::StrokeKind::Inside,
                            );
                        }
                    } else if is_sel {
                        p.rect_filled(
                            Rect::from_min_size(card.min, vec2(3.0, card.height())),
                            1.5,
                            ACCENT_BLUE,
                        );
                    }
                    let mut y = card.top() + if is_slide { PAD - 3.0 } else { 6.0 };
                    for g in galleys {
                        let gh = g.size().y;
                        ui.painter()
                            .galley(pos2(card.left() + PAD, y), g, text_color);
                        y += gh;
                    }
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                    if resp.hovered() && !is_sel {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if is_sel && scroll {
                        ui.scroll_to_rect(card.expand2(vec2(0.0, 28.0)), Some(egui::Align::Min));
                    }
                }
                // room to bring the last notes up to the top
                ui.add_space(body.height() * 0.6);
            });
        if let Some(i) = clicked {
            self.select_note(notes, i);
        }
    }

    /// Floating note and slide steppers at the bottom corners.
    fn presenter_nav(&mut self, ui: &mut egui::Ui, body: Rect, notes: &[Note]) {
        let pal = crate::settings::pal();
        let font = FontId::proportional(12.5);
        let pill = |ui: &mut egui::Ui, r: Rect| {
            let p = ui.painter();
            p.rect_filled(
                r.translate(vec2(0.0, 2.0)),
                17.0,
                Color32::from_black_alpha(18),
            );
            p.rect_filled(r, 17.0, pal.pill);
            p.rect_stroke(
                r,
                17.0,
                Stroke::new(1.0_f32, pal.border),
                egui::StrokeKind::Inside,
            );
        };
        let y = body.bottom() - 20.0 - 34.0;

        // Notes: ↑ Note n of N ↓
        let r = Rect::from_min_size(pos2(body.left() + 20.0, y), vec2(200.0, 34.0));
        pill(ui, r);
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            format!("Note {} of {}", self.present_note + 1, notes.len()),
            font.clone(),
            pal.text_hi,
        );
        let up = Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(28.0, 26.0));
        let dn = Rect::from_center_size(pos2(r.right() - 22.0, r.center().y), vec2(28.0, 26.0));
        if nav_button(
            ui,
            up,
            Icon::ChevronUp,
            "Previous Note (↑)",
            self.present_note > 0,
        ) {
            self.select_note(notes, self.present_note - 1);
        }
        if nav_button(
            ui,
            dn,
            Icon::ChevronDown,
            "Next Note (↓)",
            self.present_note + 1 < notes.len(),
        ) {
            self.select_note(notes, self.present_note + 1);
        }

        // Slides: ‹ Slide n of N ›
        let n = self.slides.len();
        let r = Rect::from_min_size(pos2(body.right() - 20.0 - 200.0, y), vec2(200.0, 34.0));
        pill(ui, r);
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            format!("Slide {} of {}", self.current + 1, n),
            font,
            pal.text_hi,
        );
        let lt = Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(28.0, 26.0));
        let rt = Rect::from_center_size(pos2(r.right() - 22.0, r.center().y), vec2(28.0, 26.0));
        if nav_button(
            ui,
            lt,
            Icon::ChevronLeft,
            "Previous Slide (←)",
            self.current > 0,
        ) {
            let s = self.current - 1;
            self.select_slide_note(notes, s);
        }
        if nav_button(
            ui,
            rt,
            Icon::ChevronRight,
            "Next Slide (→)",
            self.current + 1 < n,
        ) {
            let s = self.current + 1;
            self.select_slide_note(notes, s);
        }
    }
}

/// An icon button that is drawn dimmed and ignores clicks when disabled.
fn nav_button(ui: &mut egui::Ui, r: Rect, icon: Icon, tip: &str, enabled: bool) -> bool {
    if enabled {
        icon_button(ui, r, icon, tip).clicked()
    } else {
        let pal = crate::settings::pal();
        crate::paint_icon(ui.painter(), r, icon, pal.text_dim.gamma_multiply(0.55));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn slides_and_speaker_notes_become_notes() {
        let doc = "# Title\n\tShown text\nSpoken one.\n\nSpoken two.\n---\n## Second\n";
        let notes = build_notes(&parse_document(doc));
        assert_eq!(notes.len(), 4);
        assert_eq!(notes[0].slide, 0);
        match &notes[0].kind {
            NoteKind::Slide(lines) => {
                assert_eq!(lines[0], line("Title", true));
                assert_eq!(lines[1].text, "Shown text");
            }
            k => panic!("expected slide note, got {k:?}"),
        }
        assert_eq!(notes[1].kind, NoteKind::Speaker("Spoken one.".into()));
        assert_eq!(notes[2].kind, NoteKind::Speaker("Spoken two.".into()));
        assert_eq!(notes[3].slide, 1);
        assert_eq!(first_note_of(&notes, 1), 3);
    }

    #[test]
    fn empty_slides_still_get_a_note() {
        let notes = build_notes(&parse_document("Only spoken.\n---\n"));
        assert!(matches!(&notes[0].kind, NoteKind::Slide(l) if l[0].placeholder));
        assert_eq!(notes[1].kind, NoteKind::Speaker("Only spoken.".into()));
        assert_eq!(notes.iter().filter(|n| n.slide == 1).count(), 1);
    }

    #[test]
    fn clock_formats_hours_minutes_seconds() {
        assert_eq!(clock(0.0), "00:00:00");
        assert_eq!(clock(3725.9), "01:02:05");
    }
}
