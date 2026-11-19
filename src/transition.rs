//! Slide animations played while presenting: one choice for the whole
//! deck (Design tab), saved in `info.json`.
//!
//! - Fade: the next slide dissolves in over the current one.
//! - Slide: slides push sideways, right to left going forward and left to
//!   right going back.
//! - Motion: like Keynote's Magic Move. A picture or line of text that is on
//!   both slides glides and resizes from its old place to its new one; the
//!   rest of the next slide fades in and its content rises in one by one.

use crate::parse::ImageSpec;
use egui::{pos2, Color32, Galley, Rect};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transition {
    #[default]
    None,
    Fade,
    Slide,
    Motion,
}

impl Transition {
    pub const ALL: [Transition; 4] = [Self::None, Self::Fade, Self::Slide, Self::Motion];

    pub fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Fade => "Fade In/Out",
            Self::Slide => "Slide",
            Self::Motion => "Motion",
        }
    }

    /// The word stored in `info.json`.
    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fade => "fade",
            Self::Slide => "slide",
            Self::Motion => "motion",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|t| t.key().eq_ignore_ascii_case(s.trim()))
    }

    /// How long the whole animation runs, in seconds.
    pub fn duration(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Fade => 0.5,
            Self::Slide => 0.55,
            Self::Motion => MOTION_TOTAL,
        }
    }
}

/// Motion: how long shared pieces take to move to their new places.
pub const MOTION_MOVE: f32 = 0.9;
/// Motion: how long the next slide's background takes to fade in.
pub const MOTION_FADE: f32 = 0.5;
/// Motion: when the first piece of content starts rising.
const REVEAL_START: f32 = 0.12;
/// Motion: the delay between one piece of content and the next.
const REVEAL_STAGGER: f32 = 0.07;
/// Motion: how long each piece takes to rise in.
const REVEAL_LEN: f32 = 0.55;
/// Pieces after this many all start together, so long slides don't drag.
const REVEAL_MAX_STEPS: u32 = 8;
const MOTION_TOTAL: f32 = {
    let reveal = REVEAL_START + REVEAL_STAGGER * REVEAL_MAX_STEPS as f32 + REVEAL_LEN;
    if reveal > MOTION_MOVE {
        reveal
    } else {
        MOTION_MOVE
    }
};

/// A transition in progress, from one slide to another.
#[derive(Clone, Copy, Debug)]
pub struct Anim {
    pub kind: Transition,
    pub from: usize,
    pub to: usize,
    /// `ctx.input(|i| i.time)` when it started.
    pub start: f64,
}

impl Anim {
    /// Seconds since the start.
    pub fn elapsed(&self, now: f64) -> f32 {
        (now - self.start).max(0.0) as f32
    }

    pub fn done(&self, now: f64) -> bool {
        self.elapsed(now) >= self.kind.duration()
    }

    /// +1 going forward, -1 going back.
    pub fn dir(&self) -> f32 {
        if self.to >= self.from {
            1.0
        } else {
            -1.0
        }
    }
}

/// How each of the two slides is drawn at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub opacity: f32,
    /// Shift as a share of the slide's width.
    pub shift: f32,
}

impl Pose {
    pub const REST: Pose = Pose {
        opacity: 1.0,
        shift: 0.0,
    };
}

/// The outgoing and incoming slides' poses `t` seconds in.
pub fn poses(kind: Transition, t: f32, dir: f32) -> (Pose, Pose) {
    let rest = Pose::REST;
    match kind {
        Transition::None => (Pose { opacity: 0.0, ..rest }, rest),
        Transition::Fade => {
            let p = ease_in_out_sine(t / kind.duration());
            // The old slide stays solid underneath, so nothing shows through.
            (rest, Pose { opacity: p, ..rest })
        }
        Transition::Slide => {
            let p = ease_in_out_cubic(t / kind.duration());
            (
                Pose {
                    shift: -dir * p,
                    ..rest
                },
                Pose {
                    shift: dir * (1.0 - p),
                    ..rest
                },
            )
        }
        // Drawn by piece instead: see `begin_measure`.
        Transition::Motion => (rest, Pose {
            opacity: ease_out_cubic(t / MOTION_FADE),
            ..rest
        }),
    }
}

// ---------- content reveal (Motion) ----------

thread_local! {
    /// Seconds into a Motion reveal and the next piece's number, while one
    /// is being drawn.
    static REVEAL: Cell<Option<(f32, u32)>> = const { Cell::new(None) };
}

/// The slide drawn next has its content rise in, `t` seconds in.
pub fn begin_reveal(t: f32) {
    REVEAL.with(|r| r.set(Some((t, 0))));
}

pub fn end_reveal() {
    REVEAL.with(|r| r.set(None));
}

/// For the next piece of content: its opacity and how far below its place
/// it sits, as a share of `lift`. Outside a reveal: fully shown, in place.
pub fn reveal_step() -> (f32, f32) {
    REVEAL.with(|r| match r.get() {
        None => (1.0, 0.0),
        Some((t, n)) => {
            r.set(Some((t, n + 1)));
            let start = REVEAL_START + REVEAL_STAGGER * n.min(REVEAL_MAX_STEPS) as f32;
            let p = ((t - start) / REVEAL_LEN).clamp(0.0, 1.0);
            (ease_out_cubic(p * 1.4), 1.0 - ease_out_quint(p))
        }
    })
}

/// A painter for one piece of content during a reveal, and how far down to
/// draw it (`lift` points at most).
pub fn reveal_painter(p: &egui::Painter, lift: f32) -> (egui::Painter, f32) {
    let (opacity, down) = reveal_step();
    let mut p = p.clone();
    if opacity < 1.0 {
        p.multiply_opacity(opacity);
    }
    (p, down * lift)
}

// ---------- shared pieces (Motion) ----------

/// Something drawn on a slide that can move to the next one.
#[derive(Clone)]
pub enum Piece {
    Image {
        spec: ImageSpec,
        contain: Option<bool>,
    },
    Text {
        galley: Arc<Galley>,
        fallback: Color32,
    },
}

/// A piece, where it was drawn, and what it is (`key`, and which of the
/// pieces with that key it is, in drawing order).
#[derive(Clone)]
pub struct Element {
    pub key: String,
    pub nth: u32,
    pub rect: Rect,
    pub piece: Piece,
}

enum Mode {
    /// Record every piece, draw nothing.
    Measure(Vec<Element>),
    /// Skip these pieces; draw the rest.
    Hide(HashSet<(String, u32)>),
}

struct Pieces {
    mode: Mode,
    seen: HashMap<String, u32>,
}

thread_local! {
    static PIECES: RefCell<Option<Pieces>> = const { RefCell::new(None) };
}

/// Until `end_measure`, pieces are recorded and not drawn.
pub fn begin_measure() {
    PIECES.with(|p| {
        *p.borrow_mut() = Some(Pieces {
            mode: Mode::Measure(Vec::new()),
            seen: HashMap::new(),
        })
    });
}

pub fn end_measure() -> Vec<Element> {
    PIECES.with(|p| match p.borrow_mut().take() {
        Some(Pieces {
            mode: Mode::Measure(found),
            ..
        }) => found,
        _ => Vec::new(),
    })
}

/// Until `end_hide`, the listed pieces are not drawn.
pub fn begin_hide(hide: HashSet<(String, u32)>) {
    PIECES.with(|p| {
        *p.borrow_mut() = Some(Pieces {
            mode: Mode::Hide(hide),
            seen: HashMap::new(),
        })
    });
}

pub fn end_hide() {
    PIECES.with(|p| *p.borrow_mut() = None);
}

/// Called by the slide drawing code for each piece. True means: don't draw it.
pub fn skip_piece(key: impl FnOnce() -> String, rect: Rect, piece: impl FnOnce() -> Piece) -> bool {
    PIECES.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut() else {
            return false;
        };
        let key = key();
        let n = p.seen.entry(key.clone()).or_insert(0);
        let nth = *n;
        *n += 1;
        match &mut p.mode {
            Mode::Measure(found) => {
                found.push(Element {
                    key,
                    nth,
                    rect,
                    piece: piece(),
                });
                true
            }
            Mode::Hide(hide) => hide.contains(&(key, nth)),
        }
    })
}

/// Pieces found on both slides, as (old, new) pairs.
pub fn pair_up(old: Vec<Element>, new: &[Element]) -> Vec<(Element, Element)> {
    let mut by_key: HashMap<(String, u32), Element> = old
        .into_iter()
        .map(|e| ((e.key.clone(), e.nth), e))
        .collect();
    new.iter()
        .filter_map(|n| {
            by_key
                .remove(&(n.key.clone(), n.nth))
                .map(|o| (o, n.clone()))
        })
        .collect()
}

pub fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let l = |x: f32, y: f32| x + (y - x) * t;
    Rect::from_min_max(
        pos2(l(a.min.x, b.min.x), l(a.min.y, b.min.y)),
        pos2(l(a.max.x, b.max.x), l(a.max.y, b.max.y)),
    )
}

// ---------- easing ----------

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn ease_out_cubic(x: f32) -> f32 {
    let x = clamp01(x);
    1.0 - (1.0 - x).powi(3)
}

pub fn ease_out_quint(x: f32) -> f32 {
    let x = clamp01(x);
    1.0 - (1.0 - x).powi(5)
}

pub fn ease_in_out_cubic(x: f32) -> f32 {
    let x = clamp01(x);
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

pub fn ease_in_out_sine(x: f32) -> f32 {
    let x = clamp01(x);
    -((std::f32::consts::PI * x).cos() - 1.0) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for t in Transition::ALL {
            assert_eq!(Transition::from_key(t.key()), Some(t));
        }
        assert_eq!(Transition::from_key("MOTION"), Some(Transition::Motion));
        assert_eq!(Transition::from_key("spin"), None);
    }

    #[test]
    fn poses_end_at_rest() {
        for kind in [Transition::Fade, Transition::Slide, Transition::Motion] {
            for dir in [1.0, -1.0] {
                let (_, inn) = poses(kind, kind.duration(), dir);
                assert!((inn.opacity - 1.0).abs() < 1e-3, "{kind:?}");
                assert!(inn.shift.abs() < 1e-3, "{kind:?}");
            }
        }
    }

    #[test]
    fn slide_direction_follows_navigation() {
        let (out, inn) = poses(Transition::Slide, 0.0, 1.0);
        assert_eq!(out.shift, 0.0);
        assert_eq!(inn.shift, 1.0); // next slide comes in from the right
        let (_, inn) = poses(Transition::Slide, 0.0, -1.0);
        assert_eq!(inn.shift, -1.0); // previous slide comes in from the left
    }

    #[test]
    fn shared_pieces_pair_up_in_order() {
        let el = |key: &str, nth: u32, x: f32| Element {
            key: key.into(),
            nth,
            rect: Rect::from_min_size(pos2(x, 0.0), egui::vec2(10.0, 10.0)),
            piece: Piece::Image {
                spec: crate::parse::new_image(key.into(), None),
                contain: None,
            },
        };
        let old = vec![el("img:a.jpg", 0, 0.0), el("txt:Hi", 0, 5.0), el("txt:Hi", 1, 9.0)];
        let new = vec![el("img:a.jpg", 0, 50.0), el("txt:Hi", 0, 60.0), el("txt:Other", 0, 1.0)];
        let pairs = pair_up(old, &new);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0.rect.min.x, 0.0);
        assert_eq!(pairs[0].1.rect.min.x, 50.0);
        assert_eq!(pairs[1].0.key, "txt:Hi");
        assert_eq!(pairs[1].0.rect.min.x, 5.0);
    }

    #[test]
    fn measuring_records_and_hiding_skips() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(1.0, 1.0));
        let img = || Piece::Image {
            spec: crate::parse::new_image("a.jpg".into(), None),
            contain: None,
        };
        assert!(!skip_piece(|| "img:a.jpg".into(), r, img));
        begin_measure();
        assert!(skip_piece(|| "img:a.jpg".into(), r, img));
        assert_eq!(end_measure().len(), 1);
        begin_hide([("img:a.jpg".to_string(), 1)].into());
        assert!(!skip_piece(|| "img:a.jpg".into(), r, img));
        assert!(skip_piece(|| "img:a.jpg".into(), r, img));
        end_hide();
    }

    #[test]
    fn reveal_staggers_and_finishes() {
        begin_reveal(0.0);
        let (a, d) = reveal_step();
        assert_eq!(a, 0.0);
        assert_eq!(d, 1.0);
        end_reveal();
        begin_reveal(MOTION_TOTAL);
        for _ in 0..20 {
            assert_eq!(reveal_step(), (1.0, 0.0));
        }
        end_reveal();
        assert_eq!(reveal_step(), (1.0, 0.0));
    }
}
