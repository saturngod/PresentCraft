//! Slide animations played while presenting: one choice for the whole
//! deck (Design tab), saved in `info.json`.
//!
//! - Fade: the next slide dissolves in over the current one.
//! - Slide: slides push sideways, right to left going forward and left to
//!   right going back.
//! - Motion: the current slide sinks back and fades while the next one
//!   settles in, and its text, pictures and charts rise in one after another.

use egui::emath::TSTransform;
use egui::{vec2, Rect};
use std::cell::Cell;

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

/// Motion: the outgoing slide's part.
const MOTION_OUT: f32 = 0.45;
/// Motion: the incoming slide's background.
const MOTION_IN: f32 = 0.6;
/// Motion: when the first piece of content starts rising.
const REVEAL_START: f32 = 0.12;
/// Motion: the delay between one piece of content and the next.
const REVEAL_STAGGER: f32 = 0.07;
/// Motion: how long each piece takes to rise in.
const REVEAL_LEN: f32 = 0.55;
/// Pieces after this many all start together, so long slides don't drag.
const REVEAL_MAX_STEPS: u32 = 8;
const MOTION_TOTAL: f32 =
    REVEAL_START + REVEAL_STAGGER * REVEAL_MAX_STEPS as f32 + REVEAL_LEN;

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
    /// Scale about the slide's centre.
    pub scale: f32,
    /// Shift as a share of the slide's width.
    pub shift: f32,
}

impl Pose {
    pub const REST: Pose = Pose {
        opacity: 1.0,
        scale: 1.0,
        shift: 0.0,
    };

    /// The layer transform that places a slide drawn in `rect`.
    pub fn transform(&self, rect: Rect) -> TSTransform {
        let c = rect.center().to_vec2();
        let offset = vec2(self.shift * rect.width(), 0.0);
        TSTransform::new(c - c * self.scale + offset, self.scale)
    }
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
        Transition::Motion => {
            let out = ease_out_cubic(t / MOTION_OUT);
            let inn = ease_out_expo(t / MOTION_IN);
            (
                Pose {
                    opacity: 1.0 - out,
                    scale: 1.0 - 0.07 * out,
                    shift: -dir * 0.025 * out,
                },
                Pose {
                    opacity: ease_out_cubic(t / (MOTION_IN * 0.7)),
                    scale: 0.955 + 0.045 * inn,
                    shift: dir * 0.02 * (1.0 - inn),
                },
            )
        }
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

pub fn ease_out_expo(x: f32) -> f32 {
    let x = clamp01(x);
    if x >= 1.0 {
        1.0
    } else {
        1.0 - 2f32.powf(-10.0 * x)
    }
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
                assert!((inn.scale - 1.0).abs() < 1e-3, "{kind:?}");
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
