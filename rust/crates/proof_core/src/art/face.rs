//! Kawaii faces shared by starters, customers and Toasty the oven.

use crate::draw::DrawList;
use crate::geom::{V2, arc, circle, ellipse, v2};
use crate::ink::Ink;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    /// Dot eyes, little "w" mouth.
    Content,
    /// Closed happy arcs ^ ^.
    Happy,
    /// Big sparkly eyes, open smile.
    Excited,
    /// Wide eyes, small "o".
    Wow,
    /// Sleepy lines, drool-free.
    Sleepy,
    /// Wobbly mouth, eyebrows up: "feed me?"
    Hungry,
    /// Eyes closed, big smile (served perfectly).
    Proud,
    /// Squinty critic, flat mouth.
    Hmm,
}

/// Which parts of a face to draw (ducks have beaks, frogs have their own eyes...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceParts {
    pub eyes: bool,
    pub mouth: bool,
    pub blush: bool,
}

impl FaceParts {
    pub const ALL: FaceParts = FaceParts { eyes: true, mouth: true, blush: true };
}

/// Draw a face centred on `c`. `s` ≈ distance across the whole face; `look` nudges the eyes.
pub fn face(d: &mut DrawList, c: V2, s: f32, expr: Expr, look: V2) {
    face_parts(d, c, s, expr, look, FaceParts::ALL);
}

/// Like [`face`], choosing which parts to draw.
pub fn face_parts(d: &mut DrawList, c: V2, s: f32, expr: Expr, look: V2, parts: FaceParts) {
    let ex = s * 0.25;
    let ey = -s * 0.02;
    let eyes = [c + v2(-ex, ey) + look * (s * 0.04), c + v2(ex, ey) + look * (s * 0.04)];
    let lw = (s * 0.05).max(1.6);

    // Blush first so the key lines sit on top.
    if parts.blush {
        for sx in [-1.0, 1.0] {
            d.ht_add(Ink::Pink, 0.62, &ellipse(c + v2(sx * s * 0.39, s * 0.13), s * 0.12, s * 0.075, 0.0));
        }
    }

    if parts.eyes {
        eyes_for(d, eyes, s, expr, lw);
    }
    if parts.mouth {
        mouth_for(d, c, s, expr, lw);
    }
}

fn eyes_for(d: &mut DrawList, eyes: [V2; 2], s: f32, expr: Expr, lw: f32) {
    match expr {
        Expr::Content | Expr::Hungry | Expr::Hmm => {
            for e in eyes {
                let ry = if expr == Expr::Hmm { s * 0.045 } else { s * 0.085 };
                d.fill(Ink::Key, 1.0, &ellipse(e, s * 0.066, ry, 0.0));
                if expr != Expr::Hmm {
                    d.knock(&circle(e + v2(s * 0.022, -s * 0.03), s * 0.024));
                }
            }
            if expr == Expr::Hungry {
                for (i, e) in eyes.iter().enumerate() {
                    let sx = if i == 0 { -1.0 } else { 1.0 };
                    let b = *e + v2(0.0, -s * 0.15);
                    d.line(
                        Ink::Key,
                        lw * 0.8,
                        &[b + v2(-sx * s * 0.07, s * 0.03), b + v2(sx * s * 0.06, -s * 0.01)],
                    );
                }
            }
        }
        Expr::Happy | Expr::Proud => {
            for e in eyes {
                d.line(Ink::Key, lw, &arc(e + v2(0.0, s * 0.03), s * 0.07, PI * 1.1, PI * 1.9));
            }
        }
        Expr::Excited | Expr::Wow => {
            for e in eyes {
                d.fill(Ink::Key, 1.0, &ellipse(e, s * 0.09, s * 0.11, 0.0));
                d.knock(&circle(e + v2(s * 0.03, -s * 0.04), s * 0.034));
                d.knock(&circle(e + v2(-s * 0.03, s * 0.04), s * 0.016));
            }
        }
        Expr::Sleepy => {
            for e in eyes {
                d.line(Ink::Key, lw, &arc(e + v2(0.0, -s * 0.03), s * 0.065, PI * 0.15, PI * 0.85));
            }
        }
    }
}

fn mouth_for(d: &mut DrawList, c: V2, s: f32, expr: Expr, lw: f32) {
    let m = c + v2(0.0, s * 0.13);
    match expr {
        Expr::Content | Expr::Happy => {
            let r = s * 0.05;
            let mut w = arc(m + v2(-r, 0.0), r, PI * 0.05, PI * 0.95);
            w.reverse();
            w.extend(arc(m + v2(r, 0.0), r, PI * 0.05, PI * 0.95).into_iter().rev());
            d.line(Ink::Key, lw * 0.9, &w);
        }
        Expr::Excited | Expr::Proud => {
            let mouth = {
                let mut p = arc(m + v2(0.0, -s * 0.02), s * 0.1, 0.0, PI);
                p.push(m + v2(-s * 0.1, -s * 0.02));
                p
            };
            d.fill(Ink::Pink, 1.0, &mouth);
            d.fill(Ink::Key, 0.25, &mouth);
            d.outline(Ink::Key, lw * 0.9, &mouth);
        }
        Expr::Wow => {
            let o = ellipse(m + v2(0.0, s * 0.02), s * 0.05, s * 0.065, 0.0);
            d.fill(Ink::Pink, 1.0, &o);
            d.outline(Ink::Key, lw * 0.9, &o);
        }
        Expr::Sleepy => {
            d.outline(Ink::Key, lw * 0.8, &ellipse(m + v2(0.0, s * 0.01), s * 0.028, s * 0.034, 0.0));
        }
        Expr::Hungry => {
            let pts: Vec<V2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    m + v2((t - 0.5) * s * 0.2, (t * PI * 3.0).sin() * s * 0.018)
                })
                .collect();
            d.line(Ink::Key, lw * 0.85, &pts);
        }
        Expr::Hmm => {
            d.line(Ink::Key, lw * 0.9, &[m + v2(-s * 0.07, s * 0.01), m + v2(s * 0.07, -s * 0.005)]);
        }
    }
}
