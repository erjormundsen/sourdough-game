//! Procedural riso-kawaii art. Every function draws into a [`DrawList`] in reference units.
//!
//! House rules: dot eyes with a paper sparkle, tiny "w" mouths, halftone blush, rounded key
//! lines, soft blobby silhouettes, and only four inks.

pub mod bread;
pub mod critters;
pub mod face;
pub mod icons;
pub mod jar;
pub mod oven;
pub mod props;
pub mod scenes;
pub mod treats;

pub use face::{Expr, FaceParts, face, face_parts};

use crate::draw::DrawList;
use crate::geom::{V2, ellipse, v2};
use crate::ink::Ink;

/// Standard key-line weight at 1× art scale.
pub const LINE: f32 = 4.5;

/// A soft blue halftone cast shadow under an object.
pub fn shadow(d: &mut DrawList, c: V2, rx: f32, ry: f32) {
    d.ht(Ink::Blue, 0.38, &ellipse(c, rx, ry, 0.0));
}

/// Tiny motion/sparkle marks around a point (4-point twinkle).
pub fn twinkle(d: &mut DrawList, c: V2, s: f32, ink: Ink) {
    let pts = crate::geom::soft_star(c, s, s * 0.34, 4, 0.0);
    d.fill(ink, 1.0, &pts);
    d.outline(Ink::Key, s * 0.16, &pts);
}

/// Three little "z"s rising from `c` (sleepy).
pub fn zzz(d: &mut DrawList, c: V2, s: f32) {
    for (i, k) in [1.0f32, 0.8, 0.62].iter().enumerate() {
        let o = c + v2(i as f32 * s * 0.75, -(i as f32) * s * 0.9);
        let w = s * k;
        d.line(Ink::Key, s * 0.16, &[o, o + v2(w, 0.0), o + v2(0.0, w), o + v2(w, w)]);
    }
}
