//! Kawaii faces shared by starters, customers and Toasty the oven.
//!
//! # Face grammar
//! Everything is measured in `s`, the distance across the face.
//! * **Eyes** sit at ±0.25s, a hair above the face centre: tall key-ink ovals with a paper
//!   sparkle at the upper left (the game's light comes from the upper left, like the glass
//!   shines on jars and Toasty). Closed eyes are single strokes.
//! * **Blush** is a halftone oval under and outside each eye (±0.39s, +0.13s). Very small
//!   faces (board cards, far-away jars) print it as a flat tint so the screen dots don't
//!   swallow the features.
//! * **Mouths** are small and sit 0.13s below the centre. Open mouths are key ink with a
//!   paper-backed pink tongue.
//! * **Brows** only appear when they carry the emotion: worried (Hungry), lifted (Wow,
//!   Excited), smug (Proud) and one-up-one-down (Hmm).
//!
//! Each expression has a unique eye + mouth pair so they read apart even at thumbnail size:
//!
//! | Expr    | eyes                    | mouth                 | brows          |
//! |---------|-------------------------|-----------------------|----------------|
//! | Content | sparkle ovals           | little "w"            | –              |
//! | Happy   | closed arcs ^ ^         | small open smile      | –              |
//! | Excited | big starry eyes         | big open smile        | lifted         |
//! | Wow     | round eyes              | "o"                   | high arches    |
//! | Proud   | closed smiles ‿ ‿       | wide grin + dimples   | confident      |
//! | Hungry  | big glossy eyes         | wobbly line + drool   | worried        |
//! | Sleepy  | droopy lids with lashes | tiny slack "o"        | –              |
//! | Hmm     | one open, one squint    | flat, off-centre      | one up, one down |

use crate::draw::{DrawList, Paint, Screen};
use crate::geom::{V2, arc, circle, ellipse, quad_bezier, soft_star, v2};
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

impl Expr {
    pub const ALL: [Expr; 8] = [
        Expr::Content,
        Expr::Happy,
        Expr::Excited,
        Expr::Wow,
        Expr::Proud,
        Expr::Hungry,
        Expr::Sleepy,
        Expr::Hmm,
    ];

    /// Eyes drawn as closed strokes (no sparkle to knock out).
    pub fn eyes_closed(self) -> bool {
        matches!(self, Expr::Happy | Expr::Proud | Expr::Sleepy)
    }
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
    // Blush first so the key lines sit on top.
    if parts.blush {
        blush(d, c, s, expr);
    }
    if parts.eyes {
        eyes(d, c, s, expr, look);
    }
    if parts.mouth {
        mouth(d, c + v2(0.0, s * 0.13), s, expr);
    }
}

/// Face stroke weight for a face of size `s`, never thinner than a printable hairline.
pub fn stroke_w(d: &DrawList, s: f32) -> f32 {
    let k = d.xf().width_scale().max(0.05);
    (s * 0.046).max(1.5 / k)
}

/// True when a face of size `s` prints too small for halftone blush and tiny sparkles.
pub fn is_tiny(d: &DrawList, s: f32) -> bool {
    s * d.xf().width_scale() < 46.0
}

/// Both eyes (and brows when the expression has them), centred on the face centre `c`.
pub fn eyes(d: &mut DrawList, c: V2, s: f32, expr: Expr, look: V2) {
    let lw = stroke_w(d, s);
    let off = look * (s * 0.04);
    for sx in [-1.0f32, 1.0] {
        let e = c + v2(sx * s * 0.25, -s * 0.02) + off;
        eye(d, e, s, expr, sx, lw);
    }
    brows(d, c + off * 0.5, s, expr);
}

/// One eye centred on `e`; `side` is -1 for the viewer's left eye, +1 for the right.
pub fn eye(d: &mut DrawList, e: V2, s: f32, expr: Expr, side: f32, lw: f32) {
    let tiny = is_tiny(d, s);
    match expr {
        Expr::Content => oval_eye(d, e, s * 0.073, s * 0.095, true, tiny),
        Expr::Hungry => {
            // Big, glossy, pleading: the lower catch-light swells into a wet shine.
            oval_eye(d, e + v2(0.0, -s * 0.004), s * 0.083, s * 0.102, true, tiny);
            if !tiny {
                d.knock(&circle(e + v2(s * 0.03, s * 0.042), s * 0.016));
            }
        }
        Expr::Excited => {
            let r = v2(s * 0.094, s * 0.114);
            d.fill(Ink::Key, 1.0, &ellipse(e, r.x, r.y, 0.0));
            // Starry catch-light: the "love" sparkle.
            d.knock(&soft_star(e + v2(-s * 0.024, -s * 0.032), s * 0.062, s * 0.019, 4, 0.0));
            if !tiny {
                d.knock(&circle(e + v2(s * 0.034, s * 0.046), s * 0.017));
            }
        }
        Expr::Wow => {
            let c = e + v2(0.0, -s * 0.012);
            d.fill(Ink::Key, 1.0, &circle(c, s * 0.077));
            d.knock(&circle(c + v2(-s * 0.028, -s * 0.028), s * 0.026));
        }
        Expr::Happy => {
            d.line(Ink::Key, lw, &arc(e + v2(0.0, s * 0.042), s * 0.077, PI * 1.13, PI * 1.87));
        }
        Expr::Proud => {
            d.line(Ink::Key, lw, &arc(e + v2(0.0, -s * 0.04), s * 0.075, PI * 0.14, PI * 0.86));
        }
        Expr::Sleepy => {
            // Heavy lid drooping towards the outer corner, with two little lashes.
            let outer = e + v2(side * s * 0.08, s * 0.024);
            let inner = e + v2(-side * s * 0.07, s * 0.004);
            let lid = quad_bezier(inner, e + v2(0.0, s * 0.03), outer, 8);
            d.line(Ink::Key, lw, &lid);
            for (t, len) in [(0.45f32, 0.034f32), (0.8, 0.04)] {
                let p = inner.lerp(outer, t) + v2(0.0, s * 0.02);
                d.line(Ink::Key, lw * 0.6, &[p, p + v2(side * s * len * 0.35, s * len)]);
            }
        }
        Expr::Hmm => {
            if side < 0.0 {
                // The appraising eye: open, a little narrowed.
                oval_eye(d, e, s * 0.07, s * 0.078, true, tiny);
            } else {
                // The squint.
                let a = e + v2(-s * 0.068, s * 0.004);
                let b = e + v2(s * 0.068, -s * 0.004);
                d.line(Ink::Key, lw, &quad_bezier(a, e + v2(0.0, -s * 0.014), b, 6));
            }
        }
    }
}

fn oval_eye(d: &mut DrawList, e: V2, rx: f32, ry: f32, sparkle: bool, tiny: bool) {
    d.fill(Ink::Key, 1.0, &ellipse(e, rx, ry, 0.0));
    if sparkle {
        let r = rx * if tiny { 0.46 } else { 0.4 };
        d.knock(&circle(e + v2(-rx * 0.3, -ry * 0.36), r));
        if !tiny {
            // A second, tiny catch-light low on the far side keeps the eye wet and alive.
            d.knock(&circle(e + v2(rx * 0.38, ry * 0.46), rx * 0.16));
        }
    }
}

/// Brows for the expressions that need them (no-op otherwise).
pub fn brows(d: &mut DrawList, c: V2, s: f32, expr: Expr) {
    let lw = stroke_w(d, s) * 0.86;
    for sx in [-1.0f32, 1.0] {
        let e = c + v2(sx * s * 0.25, -s * 0.02);
        match expr {
            Expr::Hungry => {
                // Worried: inner ends lifted.
                let b = e + v2(0.0, -s * 0.16);
                let outer = b + v2(sx * s * 0.068, s * 0.026);
                let inner = b + v2(-sx * s * 0.05, -s * 0.018);
                d.line(Ink::Key, lw, &quad_bezier(outer, b + v2(sx * s * 0.01, -s * 0.004), inner, 6));
            }
            Expr::Wow => {
                d.line(Ink::Key, lw, &arc(e + v2(0.0, -s * 0.1), s * 0.074, PI * 1.22, PI * 1.78));
            }
            Expr::Excited => {
                d.line(Ink::Key, lw, &arc(e + v2(0.0, -s * 0.09), s * 0.06, PI * 1.28, PI * 1.72));
            }
            Expr::Proud => {
                // Confident: lifted and tilted up at the outer end.
                let b = e + v2(0.0, -s * 0.14);
                let outer = b + v2(sx * s * 0.066, -s * 0.014);
                let inner = b + v2(-sx * s * 0.052, s * 0.012);
                d.line(Ink::Key, lw, &quad_bezier(inner, b + v2(0.0, -s * 0.022), outer, 6));
            }
            Expr::Hmm => {
                if sx < 0.0 {
                    // Raised in appraisal.
                    d.line(Ink::Key, lw, &arc(e + v2(0.0, -s * 0.1), s * 0.07, PI * 1.2, PI * 1.72));
                } else {
                    // Lowered over the squint, sloping down towards the nose.
                    let outer = e + v2(s * 0.07, -s * 0.11);
                    let inner = e + v2(-s * 0.056, -s * 0.074);
                    d.line(Ink::Key, lw, &[outer, inner]);
                }
            }
            _ => {}
        }
    }
}

/// A mouth centred on `m` for a face of size `s`.
pub fn mouth(d: &mut DrawList, m: V2, s: f32, expr: Expr) {
    let lw = stroke_w(d, s) * 0.88;
    match expr {
        Expr::Content => w_mouth(d, m, s * 0.047, lw),
        Expr::Happy => open_mouth(d, &smile_disc(m + v2(0.0, -s * 0.012), s * 0.076, s * 0.07), lw),
        Expr::Excited => open_mouth(d, &smile_disc(m + v2(0.0, -s * 0.018), s * 0.108, s * 0.1), lw),
        Expr::Wow => {
            let o = ellipse(m + v2(0.0, s * 0.012), s * 0.05, s * 0.064, 0.0);
            open_mouth(d, &o, lw);
        }
        Expr::Proud => {
            // Wide, satisfied grin with dimples.
            let w = s * 0.108;
            let grin = {
                let mut p =
                    quad_bezier(m + v2(-w, -s * 0.018), m + v2(0.0, -s * 0.006), m + v2(w, -s * 0.018), 10);
                p.extend(quad_bezier(
                    m + v2(w, -s * 0.018),
                    m + v2(0.0, s * 0.11),
                    m + v2(-w, -s * 0.018),
                    12,
                ));
                p
            };
            open_mouth(d, &grin, lw);
            for sx in [-1.0f32, 1.0] {
                let a = m + v2(sx * (w + s * 0.012), -s * 0.036);
                d.line(
                    Ink::Key,
                    lw * 0.7,
                    &quad_bezier(a, a + v2(sx * s * 0.02, s * 0.02), a + v2(0.0, s * 0.04), 5),
                );
            }
        }
        Expr::Hungry => {
            let pts: Vec<V2> = (0..=14)
                .map(|i| {
                    let t = i as f32 / 14.0;
                    m + v2((t - 0.5) * s * 0.22, (t * PI * 3.0).sin() * s * 0.018)
                })
                .collect();
            d.line(Ink::Key, lw, &pts);
            if !is_tiny(d, s) {
                // A hopeful drip of drool at the corner.
                let top = m + v2(s * 0.085, s * 0.012);
                let drop = drip(top, s * 0.024, s * 0.07);
                d.backing(&drop);
                d.fill(Ink::Blue, 0.55, &drop);
                d.knock(&circle(top + v2(-s * 0.006, s * 0.05), s * 0.007));
                d.outline(Ink::Key, lw * 0.6, &drop);
            }
        }
        Expr::Sleepy => {
            let o = ellipse(m + v2(0.0, s * 0.01), s * 0.028, s * 0.034, 0.0);
            d.fill(Ink::Key, 1.0, &o);
        }
        Expr::Hmm => {
            let a = m + v2(-s * 0.035, s * 0.01);
            let b = m + v2(s * 0.075, -s * 0.008);
            d.line(Ink::Key, lw, &quad_bezier(a, a.lerp(b, 0.5) + v2(0.0, s * 0.006), b, 6));
        }
    }
}

/// The classic cat-mouth "w" centred on `m`, each lobe `r` wide.
pub fn w_mouth(d: &mut DrawList, m: V2, r: f32, lw: f32) {
    let mut w = arc(m + v2(-r, 0.0), r, PI * 0.05, PI * 0.95);
    w.reverse();
    w.extend(arc(m + v2(r, 0.0), r, PI * 0.05, PI * 0.95).into_iter().rev());
    d.line(Ink::Key, lw, &w);
}

/// Lower half-disc smile with a slightly curved top edge.
fn smile_disc(c: V2, rx: f32, ry: f32) -> Vec<V2> {
    let mut p = quad_bezier(c + v2(-rx, 0.0), c + v2(0.0, ry * 0.12), c + v2(rx, 0.0), 8);
    p.extend((1..16).map(|i| {
        let a = PI * i as f32 / 16.0;
        c + v2(a.cos() * rx, a.sin() * ry)
    }));
    p
}

/// An open mouth: key-ink cavity, paper-backed pink tongue, key outline.
pub fn open_mouth(d: &mut DrawList, poly: &[V2], lw: f32) {
    let Some(b) = crate::geom::Rect::of_points(poly) else { return };
    d.backing(poly);
    d.fill(Ink::Key, 1.0, poly);
    let tongue = ellipse(v2(b.center().x, b.y + b.h * 0.98), b.w * 0.34, b.h * 0.5, 0.0);
    d.clipped(poly, |d| {
        d.knock(&tongue);
        d.fill(Ink::Pink, 0.9, &tongue);
    });
    d.outline(Ink::Key, lw, poly);
}

/// A teardrop hanging from `top`: radius `r`, total length `len`.
fn drip(top: V2, r: f32, len: f32) -> Vec<V2> {
    let c = top + v2(0.0, len - r);
    let mut p = vec![top];
    p.extend(arc(c, r, -PI * 0.12, PI * 1.12));
    p
}

/// Halftone cheeks (flat tint on tiny faces). Flustered expressions add blush strokes.
pub fn blush(d: &mut DrawList, c: V2, s: f32, expr: Expr) {
    let tiny = is_tiny(d, s);
    for sx in [-1.0f32, 1.0] {
        let bc = c + v2(sx * s * 0.39, s * 0.13);
        let b = ellipse(bc, s * 0.12, s * 0.075, 0.0);
        // Thin the yellow and blue plates under the cheek so the pink prints clean pink,
        // not coral or mauve, whatever the face is printed on.
        d.knock_p(0.45, Screen::Solid, 0b0110, &b);
        if tiny {
            d.fill_p(
                Paint { ink: Ink::Pink, tone: 0.5, screen: Screen::Solid, mode: crate::draw::Mode::Add },
                &b,
            );
        } else {
            d.ht_add(Ink::Pink, 0.62, &b);
        }
        if matches!(expr, Expr::Excited | Expr::Proud) && !tiny {
            for k in 0..3 {
                let x = (k as f32 - 1.0) * s * 0.05;
                let p = bc + v2(x, 0.0);
                d.line(Ink::Pink, s * 0.02, &[p + v2(s * 0.016, -s * 0.03), p + v2(-s * 0.014, s * 0.03)]);
            }
        }
    }
}
