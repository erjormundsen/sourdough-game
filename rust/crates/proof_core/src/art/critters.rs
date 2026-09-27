//! The regulars: eight animal customers drawn as shoulder-up busts.
//!
//! Every regular shares one construction so the cast reads as a family: the head centre sits
//! at [`HEAD_C`], faces follow the shared grammar in [`super::face`], shoulders fill about
//! 200–230 units, and both paws (or wings, hooves, webbed hands) rest on the shop counter's
//! edge at [`PAW_REST_Y`]. What changes per species is the silhouette, the ink recipe and
//! the one personal touch each regular carries (Mimi's ribbon, Bruno's flannel, Pip's rain
//! gear, Sir Whiskers' monocle and notebook, Hazel's apron flower, Momo's backpack, Clover's
//! bell, Otto's pebble).
//!
//! Inking follows the kit's halo technique (see `critter_kit.rs`): an `OUTER` silhouette,
//! `INNER` lines where parts meet, `DETAIL` for fur, stitches and whiskers. Skins get a riso
//! knockout under the blush so cheeks print clean pink, clothes carry halftone form shading,
//! and a blue halftone "shadow plate" offsets the silhouette onto the wall behind.
//!
//! # Poses and life
//! The expression picks the body language, so each of the shop's reactions reads at a glance:
//! * **Resting** (Content and the rest): paws on the counter edge.
//! * **Waving** (Happy, "yum, thanks!"): one paw up by the head with little motion marks.
//! * **Cheering** (Excited, "loved it!"): paws (wings, hooves, toe pads) pressed to the
//!   cheeks; Otto hugs his pebble under his chin instead.
//! * **Pleading** (Hungry, sold out / not today): paws clasped under the chin.
//!
//! Raised arms are drawn after the head, so they cross in front of the chin and jaw.
//! * **Idle**: `CritterView::t` drives a 12-frame loop at the shop's three redraws a second (a
//!   bob, breathing shoulders, an ear/whisker flick, a slow blink, Momo's throat gulp). The
//!   frames repeat exactly, so the texture cache serves the loop after its first pass.
//!
//! # Print size
//! Busts check their transform: below ~0.45× (the Tomorrow board) halftones turn into flat
//! tints and hairline details drop out, so small regulars stay clean instead of speckled.

#[path = "critter_kit.rs"]
mod kit;

#[path = "critter_bear.rs"]
mod bear;
#[path = "critter_bunny.rs"]
mod bunny;
#[path = "critter_cat.rs"]
mod cat;
#[path = "critter_duck.rs"]
mod duck;
#[path = "critter_frog.rs"]
mod frog;
#[path = "critter_hedgehog.rs"]
mod hedgehog;
#[path = "critter_otter.rs"]
mod otter;
#[path = "critter_sheep.rs"]
mod sheep;

use super::Expr;
use crate::content::Species;
use crate::draw::DrawList;
use crate::geom::{V2, Xf, v2};
use kit::{Lod, Rig};

/// Footprint contract (reference units, origin = bottom centre of the shoulders):
/// the tallest regular (bunny ears) stays within `BUST_H`, shoulders within `BUST_W`.
pub const BUST_H: f32 = 330.0;
pub const BUST_W: f32 = 230.0;
/// Head centre at rest (screens anchor speech bubbles and hearts from here).
pub const HEAD_C: V2 = v2(0.0, -168.0);
/// Where the paws rest, in bust space: the shop counter's top edge when the shop prints the
/// bust at 1.3× with its origin 40 units below the counter top.
pub const PAW_REST_Y: f32 = kit::REST_Y;
/// Radius that [`critter_head`] portraits fit within (ears and hats included; whiskers may
/// poke a little past it).
pub const HEAD_R: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CritterView {
    pub species: Species,
    pub expr: Expr,
    /// Gentle idle bob phase (seconds).
    pub t: f32,
}

/// Draw a customer with the bottom centre of their shoulders at the origin.
pub fn critter(d: &mut DrawList, v: &CritterView) {
    let pose = kit::pose(v.t);
    let r = Rig {
        h: HEAD_C + v2(0.0, pose.bob),
        expr: v.expr,
        pose,
        lod: Lod::of(d),
        body: true,
        portrait: false,
    };
    if r.small() {
        d.fill(crate::ink::Ink::Blue, 0.18, &crate::geom::ellipse(v2(0.0, 2.0), 100.0, 9.0, 0.0));
    } else {
        super::style::contact_shadow(d, v2(0.0, 2.0), 108.0, 11.0);
    }
    species(d, v.species, &r);
}

/// Head-only portrait centred on the head centre, fitting within [`HEAD_R`] including
/// ears and hats: for the Tomorrow board, order notes and icons.
pub fn critter_head(d: &mut DrawList, species_: Species, expr: Expr) {
    // Tall ears and wide hats shrink the whole portrait a little so everyone fits the frame.
    let fit = match species_ {
        Species::Bunny => 0.78,
        Species::Cat => 0.9,
        Species::Duck => 0.95,
        _ => 1.0,
    };
    d.with(Xf::IDENTITY.scaled(fit), |d| {
        let r = Rig { h: V2::ZERO, expr, pose: kit::pose(0.0), lod: Lod::of(d), body: false, portrait: true };
        species(d, species_, &r);
    });
}

fn species(d: &mut DrawList, s: Species, r: &Rig) {
    match s {
        Species::Bunny => bunny::draw(d, r),
        Species::Bear => bear::draw(d, r),
        Species::Duck => duck::draw(d, r),
        Species::Cat => cat::draw(d, r),
        Species::Hedgehog => hedgehog::draw(d, r),
        Species::Frog => frog::draw(d, r),
        Species::Sheep => sheep::draw(d, r),
        Species::Otter => otter::draw(d, r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Op, Screen};

    fn bust(s: Species, e: Expr, t: f32) -> DrawList {
        let mut d = DrawList::new();
        critter(&mut d, &CritterView { species: s, expr: e, t });
        d
    }

    /// Phones re-rasterise the shop bust a few times a second: keep every frame lean.
    #[test]
    fn busts_stay_within_command_budget() {
        for s in Species::ALL {
            for e in Expr::ALL {
                let n = bust(s, e, 0.0).cmds.len();
                assert!(n < 1600, "{s:?} {e:?}: {n} commands");
                let mut d = DrawList::new();
                critter_head(&mut d, s, e);
                assert!(d.cmds.len() < 900, "{s:?} {e:?} head: {} commands", d.cmds.len());
            }
        }
    }

    /// Every pose (resting, waving, cheering, pleading) through the idle loop stays inside the
    /// footprint, give or take the halo stroke and the offset shadow plate.
    #[test]
    fn busts_respect_their_footprint() {
        for s in Species::ALL {
            for e in Expr::ALL {
                for t in [0.0, 1.0, 2.0, 3.0] {
                    let b = bust(s, e, t).bounds().unwrap();
                    assert!(b.y >= -(BUST_H + 8.0), "{s:?} {e:?} too tall: {b:?}");
                    let half = BUST_W * 0.5;
                    assert!(b.x >= -(half + 12.0) && b.x + b.w <= half + 22.0, "{s:?} {e:?} too wide: {b:?}");
                }
            }
        }
    }

    #[test]
    fn head_portraits_fit_their_frame() {
        for s in Species::ALL {
            let mut d = DrawList::new();
            critter_head(&mut d, s, Expr::Happy);
            let b = d.bounds().unwrap();
            let r = HEAD_R + 14.0;
            assert!(b.x >= -r && b.y >= -r && b.x + b.w <= r && b.y + b.h <= r, "{s:?} portrait: {b:?}");
        }
    }

    /// The idle loop is quantised so the shop's texture cache replays it instead of
    /// rasterising a new frame every third of a second: 12 ticks, six distinct drawings.
    #[test]
    fn idle_loop_repeats_exactly() {
        for s in Species::ALL {
            let loop_at = |t0: f32| -> Vec<u64> {
                (0..kit::FRAMES).map(|i| bust(s, Expr::Content, t0 + i as f32 / 3.0).hash64()).collect()
            };
            let frames = loop_at(0.0);
            assert_eq!(frames, loop_at(28.0), "{s:?} idle loop drifts");
            let distinct = frames.iter().collect::<std::collections::HashSet<_>>().len();
            assert_eq!(distinct, 6, "{s:?}: the idle loop should reuse six drawings");
        }
    }

    #[test]
    fn small_prints_use_flat_tints() {
        let mut d = DrawList::new();
        d.with(Xf::IDENTITY.scaled(0.36), |d| {
            critter(d, &CritterView { species: Species::Bear, expr: Expr::Happy, t: 0.0 })
        });
        let halftones =
            d.cmds.iter().filter(|c| matches!(c.op, Op::Ink(p) if p.screen != Screen::Solid)).count();
        assert_eq!(halftones, 0, "board-sized busts should not print halftone dots");
    }
}
