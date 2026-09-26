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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Pattern, Recipe, Shape, Species, Stencil, Topping, Treat};
    use crate::draw::Shape as S;
    use crate::scoring::template;

    fn all_finite(d: &DrawList) -> bool {
        d.cmds.iter().all(|c| match &c.shape {
            S::Poly(p) => p.iter().all(|q| q.x.is_finite() && q.y.is_finite()),
            S::PolysEo(ps) => ps.iter().flatten().all(|q| q.x.is_finite() && q.y.is_finite()),
            S::Line { pts, width, .. } => {
                width.is_finite() && pts.iter().all(|q| q.x.is_finite() && q.y.is_finite())
            }
        })
    }

    /// A NaN point silently drops a whole shape in the rasteriser, so guard every piece of art.
    #[test]
    fn every_piece_of_art_has_finite_geometry() {
        let mut d = DrawList::new();
        for (i, p) in Pattern::ALL.iter().enumerate() {
            for shape in [Shape::Boule, Shape::Batard] {
                let v = bread::LoafView {
                    shape,
                    recipe: Recipe::ALL[i % Recipe::ALL.len()],
                    cuts: template(*p, shape)
                        .into_iter()
                        .map(|pts| bread::CutView { pts, bloom: 1.0, ear: 1.0 })
                        .collect(),
                    stencil: Some(Stencil::ALL[i]),
                    topping: Some(Topping::ALL[i % 3]),
                    ..Default::default()
                };
                bread::loaf_top(&mut d, &v);
            }
        }
        bread::crumb_slice(&mut d, 200.0, 130.0, 1.0, 1.0, 3);
        for s in Species::ALL {
            for e in [
                Expr::Content,
                Expr::Happy,
                Expr::Excited,
                Expr::Wow,
                Expr::Sleepy,
                Expr::Hungry,
                Expr::Proud,
                Expr::Hmm,
            ] {
                critters::critter(&mut d, &critters::CritterView { species: s, expr: e, t: 0.3 });
            }
        }
        for hooch in [false, true] {
            jar::jar(&mut d, &jar::JarView { hooch, ..Default::default() });
        }
        oven::oven(
            &mut d,
            &oven::OvenView { glow: 1.0, open: 0.5, steam: 1.0, ..Default::default() },
            |_| {},
        );
        for t in Treat::ALL {
            treats::treat(&mut d, t, 120.0, 1);
            treats::treat_raw(&mut d, t, 120.0, 1);
        }
        for b in [scenes::Backdrop::Bakehouse, scenes::Backdrop::Shopfront, scenes::Backdrop::Pantry] {
            scenes::backdrop(&mut d, b, 1560.0);
            scenes::counter_front(&mut d, b, 1560.0);
        }
        props::stamp(&mut d, 60.0, 3, crate::ink::Ink::Pink, 1);
        props::ticket(&mut d, 300.0, 200.0, crate::ink::Ink::Yellow);
        assert!(!d.is_empty());
        assert!(all_finite(&d));
    }
}
