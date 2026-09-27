//! Momo the frog: always up for an adventure. Polka-dot tee, backpack straps, a big leaf
//! poking out of the pack, and sticky toe-pads on the counter. Surprise me!

use super::kit::*;
use crate::art::Expr;
use crate::art::face;
use crate::draw::{DrawList, Screen};
use crate::geom::{V2, arc, circle, ellipse, quad_bezier, rect, rounded_rect, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const SKIN: Coat = Coat::new(0.86, 0.0, 0.56, 0.0);
const SKIN_PALE: Coat = Coat::new(0.52, 0.0, 0.16, 0.0);
const SPOT: Coat = Coat::new(0.9, 0.0, 0.82, 0.0);
const TEE: Coat = Coat::new(0.42, 0.48, 0.0, 0.0);
const STRAP: Coat = Coat::new(0.0, 0.0, 0.74, 0.08);
const LEAF: Coat = Coat::new(0.9, 0.0, 0.36, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let head = ellipse(h + v2(0.0, 14.0), 94.0, 56.0, 0.0);
    let bump_c = [h + v2(-46.0, -30.0), h + v2(46.0, -30.0)];
    let bumps: Vec<Vec<V2>> = bump_c.iter().map(|c| circle(*c, 31.0)).collect();

    let torso = r.breathe(&mound(&[
        v2(0.0, -122.0),
        v2(36.0, -118.0),
        v2(60.0, -108.0),
        v2(78.0, -103.0),
        v2(94.0, -85.0),
        v2(101.0, -54.0),
        v2(104.0, 0.0),
    ]));
    let paths: Vec<Vec<V2>> = [-1.0f32, 1.0].iter().map(|&sx| r.arm_path(sx, 92.0, 48.0)).collect();
    let arms: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(p, 16.5, 13.5)).collect();
    let sleeves: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(&p[..6], 19.0, 18.0)).collect();
    let hands: Vec<(V2, f32)> = [-1.0f32, 1.0].iter().map(|&sx| r.paw_place(sx, 44.0, 92.0, 2.0)).collect();
    let palms: Vec<Vec<V2>> =
        hands.iter().map(|(c, tilt)| rot(&paw_shape(*c + v2(0.0, -4.0), 34.0, 22.0), *c, *tilt)).collect();
    let pads: Vec<Vec<Vec<V2>>> = hands
        .iter()
        .enumerate()
        .map(|(i, (c, tilt))| toe_pads(*c, *tilt, r.raised(if i == 0 { -1.0 } else { 1.0 })))
        .collect();
    let leaf_base = v2(-70.0, -100.0);
    let leaf_tip = v2(-112.0, -184.0);
    let leaf = r.breathe(&soft_spike(leaf_base, leaf_tip, 56.0, -9.0));

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head, &leaf];
        sil.extend(bumps.iter().map(|b| b.as_slice()));
        sil.extend(sleeves.iter().map(|s| s.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &leaf);
        r.halo(d, &torso);
        for (a, _) in arms.iter().chain(sleeves.iter()) {
            r.halo(d, a);
        }
        for p in &palms {
            r.halo(d, p);
        }
        for pad in pads.iter().flatten() {
            r.halo(d, pad);
        }
    }
    for b in &bumps {
        r.halo(d, b);
    }
    r.halo(d, &head);

    if r.body {
        // The leaf poking out of the backpack, behind everything.
        r.part_with(d, &leaf, |d, p| {
            LEAF.ink(d, p);
            r.rim_shade(d, p, v2(1.0, 0.2), 8.0, Ink::Blue, 0.3);
        });
        let lt = r.breathe(&[leaf_base, leaf_tip]);
        let (base, tip) = (lt[0], lt[1]);
        let rib = bow(base, tip.lerp(base, 0.1), -5.0, 10);
        r.seam(d, &rib);
        if !r.small() {
            let dir = (tip - base).norm();
            for i in 1..5 {
                let p = crate::geom::resample(&rib, 6)[i];
                for sx in [-1.0f32, 1.0] {
                    let q = p + dir * 9.0 + dir.perp() * (sx * 15.0);
                    r.detail_line(d, &bow(p, q, sx * 2.5, 4));
                }
            }
        }
        r.part_with(d, &torso, |d, p| {
            TEE.ink(d, p);
            dots(r, d, p);
            r.rim_shade(d, p, v2(1.0, 0.3), 12.0, Ink::Pink, 0.3);
        });
        neck_shadow(r, d, &torso, 94.0, 56.0, 14.0);
        straps(r, d);
    }
    let limbs = |d: &mut DrawList| {
        for (a, open) in &arms {
            d.backing(a);
            SKIN.ink(d, a);
            d.line(Ink::Key, r.inner(), open);
        }
        for (s, open) in &sleeves {
            d.backing(s);
            TEE.ink(d, s);
            dots(r, d, s);
            d.line(Ink::Key, r.inner(), open);
        }
        for (i, (palm, hand_pads)) in palms.iter().zip(pads.iter()).enumerate() {
            r.part(d, palm, SKIN);
            for pad in hand_pads {
                r.part(d, pad, SKIN_PALE);
            }
            r.wave_marks(d, hands[i].0 + v2(0.0, -30.0), if i == 0 { -1.0 } else { 1.0 });
        }
    };
    if r.body && !r.limbs_in_front() {
        limbs(d);
    }

    // Head: bumps and face merge into one silhouette without seams.
    for b in &bumps {
        d.backing(b);
        SKIN.ink(d, b);
    }
    d.backing(&head);
    SKIN.ink(d, &head);
    // Pale throat that puffs out in a little "gulp" once per idle loop.
    let gulp = if matches!(r.pose.frame, 4 | 5) { 1.08 } else { 1.0 };
    d.clipped(&head, |d| {
        SKIN_PALE.ink(d, &ellipse(h + v2(0.0, 56.0), 60.0 * gulp, 24.0 * gulp, 0.0));
    });
    r.rim_shade(d, &head, v2(0.4, 1.0), 9.0, Ink::Blue, 0.22);
    let parts: Vec<&[V2]> = vec![&head, &bumps[0], &bumps[1]];
    r.union_outline(d, &parts);
    if !r.small() {
        for (o, rad) in
            [(v2(-14.0, -8.0), 6.5), (v2(6.0, -16.0), 4.5), (v2(20.0, -4.0), 3.6), (v2(-62.0, 4.0), 4.0)]
        {
            let s = circle(h + o, rad);
            SPOT.ink(d, &s);
        }
    }
    for sx in [-1.0f32, 1.0] {
        d.fill(Ink::Key, 0.9, &ellipse(h + v2(sx * 9.0, 10.0), 2.4, 1.8, 0.0));
    }
    r.clear_cheeks(d, h + v2(0.0, 2.0), 150.0, 0.8);
    face::blush(d, h + v2(0.0, 2.0), 150.0, r.expr);
    for (i, c) in bump_c.iter().enumerate() {
        frog_eye(r, d, *c + v2(0.0, -2.0), if i == 0 { -1.0 } else { 1.0 });
    }
    frog_mouth(r, d, h + v2(0.0, 30.0));
    if r.limbs_in_front() {
        limbs(d);
    }
}

/// Three sticky toe pads at the fingertips: along the counter edge, or up top when the hand
/// is `raised`.
fn toe_pads(c: V2, tilt: f32, raised: bool) -> Vec<Vec<V2>> {
    [(-13.0, -1.5), (0.0, 1.0), (13.0, -1.5)]
        .iter()
        .map(|(x, y)| {
            let p = if raised { c + v2(*x * 0.9, -28.0 - *y) } else { c + v2(*x, *y - 5.0) };
            circle(c + (p - c).rotate(tilt), 7.2)
        })
        .collect()
}

fn dots(r: &Rig, d: &mut DrawList, clip: &[V2]) {
    if r.small() {
        return;
    }
    d.clipped(clip, |d| {
        for c in dot_grid(clip, 26.0, 22.0, 5.0) {
            // Knock only the pink plate: yellow dots on coral.
            d.knock_p(1.0, Screen::Solid, 0b0001, &circle(c, 5.0));
        }
    });
}

fn straps(r: &Rig, d: &mut DrawList) {
    for sx in [-1.0f32, 1.0] {
        let s = r.breathe(&[
            v2(sx * 44.0, -122.0),
            v2(sx * 62.0, -118.0),
            v2(sx * 58.0, -60.0),
            v2(sx * 56.0, 4.0),
            v2(sx * 40.0, 4.0),
            v2(sx * 42.0, -60.0),
        ]);
        r.part(d, &s, STRAP);
        r.stitches(d, &inset(&s[1..5], 1.0, false), 3.0, 3.0);
        let b = r.breathe(&[v2(sx * 50.0, -60.0)])[0];
        let buckle = rounded_rect(rect(b.x - 11.0, b.y - 6.0, 22.0, 12.0), 3.0);
        r.part(d, &buckle, Coat::new(0.92, 0.12, 0.0, 0.0));
        r.detail_line(d, &[b + v2(-4.0, -3.0), b + v2(-4.0, 3.0)]);
    }
    // Chest strap with a clip.
    let y = r.breathe(&[v2(0.0, -72.0)])[0].y;
    let band = rounded_rect(rect(-44.0, y - 4.5, 88.0, 9.0), 4.0);
    r.part(d, &band, STRAP);
    let clip = rounded_rect(rect(-10.0, y - 7.0, 20.0, 14.0), 4.0);
    r.part(d, &clip, Coat::new(0.3, 0.1, 0.1, 0.1));
}

fn frog_eye(r: &Rig, d: &mut DrawList, c: V2, side: f32) {
    let lw = face::stroke_w(d, 100.0);
    let white = circle(c, 21.0);
    let closed = r.expr.eyes_closed() || r.blinking();
    if closed && r.expr != Expr::Sleepy {
        // Lid shut: a stroke across the dome.
        let arc_pts = if r.expr == Expr::Happy {
            arc(c + v2(0.0, 10.0), 13.0, PI * 1.12, PI * 1.88)
        } else {
            arc(c + v2(0.0, -6.0), 13.0, PI * 0.14, PI * 0.86)
        };
        d.line(Ink::Key, lw, &arc_pts);
        return;
    }
    d.backing(&white);
    d.outline(Ink::Key, r.inner(), &white);
    let (pr, off, stars) = match r.expr {
        Expr::Excited => (13.5, v2(0.0, 1.0), true),
        Expr::Hungry => (12.0, v2(0.0, -2.0), false),
        Expr::Wow => (7.0, v2(0.0, 0.0), false),
        Expr::Hmm => (9.5, v2(6.0, 2.0), false),
        _ => (10.5, v2(-side, 2.0), false),
    };
    let p = c + off;
    d.fill(Ink::Key, 1.0, &ellipse(p, pr, pr * 1.12, 0.0));
    if stars {
        d.knock(&crate::geom::soft_star(p + v2(-3.5, -4.5), 7.5, 2.4, 4, 0.0));
    } else {
        d.knock(&circle(p + v2(-pr * 0.34, -pr * 0.4), pr * 0.34));
    }
    if r.expr == Expr::Hungry {
        d.knock(&circle(p + v2(pr * 0.35, pr * 0.4), pr * 0.18));
    }
    // Lids for sleepy and the critic's squint.
    let lid_to = match r.expr {
        Expr::Sleepy => Some(0.1),
        Expr::Hmm if side > 0.0 => Some(-0.1),
        _ => None,
    };
    if let Some(t) = lid_to {
        let y = c.y + 21.0 * t;
        let lid = crate::geom::rect_poly(rect(c.x - 30.0, c.y - 40.0, 60.0, y - (c.y - 40.0)));
        d.clipped(&white, |d| {
            d.backing(&lid);
            SKIN.ink(d, &lid);
        });
        r.seam(d, &quad_bezier(v2(c.x - 20.0, y), v2(c.x, y + 3.0), v2(c.x + 20.0, y), 8));
        d.outline(Ink::Key, r.inner(), &white);
    }
    // Brows ride on the domes for worried / wow faces.
    let brow = match r.expr {
        Expr::Hungry => Some((v2(-side * 8.0, -32.0), v2(side * 12.0, -26.0))),
        Expr::Wow | Expr::Excited => Some((v2(-12.0, -31.0), v2(12.0, -31.0))),
        _ => None,
    };
    if let Some((a, b)) = brow {
        r.seam(d, &bow(c + a, c + b, if r.expr == Expr::Hungry { 0.0 } else { 4.0 }, 6));
    }
}

fn frog_mouth(r: &Rig, d: &mut DrawList, m: V2) {
    let lw = face::stroke_w(d, 110.0);
    match r.expr {
        Expr::Content => {
            r.seam(d, &quad_bezier(m + v2(-44.0, -4.0), m + v2(0.0, 16.0), m + v2(44.0, -4.0), 16))
        }
        Expr::Happy | Expr::Excited => {
            let (w, dep) = if r.expr == Expr::Excited { (34.0, 26.0) } else { (26.0, 17.0) };
            let mut p = quad_bezier(m + v2(-w, -2.0), m + v2(0.0, 4.0), m + v2(w, -2.0), 10);
            p.extend(
                quad_bezier(m + v2(w, -2.0), m + v2(0.0, dep * 1.7), m + v2(-w, -2.0), 12)
                    .into_iter()
                    .skip(1),
            );
            face::open_mouth(d, &p, lw);
            for sx in [-1.0f32, 1.0] {
                r.detail_line(d, &bow(m + v2(sx * (w + 2.0), -2.0), m + v2(sx * (w + 12.0), -8.0), 0.0, 2));
            }
        }
        Expr::Proud => {
            let mut p = quad_bezier(m + v2(-38.0, -4.0), m + v2(0.0, 2.0), m + v2(38.0, -4.0), 10);
            p.extend(
                quad_bezier(m + v2(38.0, -4.0), m + v2(0.0, 18.0), m + v2(-38.0, -4.0), 12)
                    .into_iter()
                    .skip(1),
            );
            face::open_mouth(d, &p, lw);
        }
        Expr::Wow => face::mouth(d, m + v2(0.0, -4.0), 130.0, Expr::Wow),
        Expr::Hungry => face::mouth(d, m + v2(0.0, -2.0), 170.0, Expr::Hungry),
        Expr::Sleepy => face::mouth(d, m + v2(0.0, -2.0), 120.0, Expr::Sleepy),
        Expr::Hmm => r.seam(d, &[m + v2(-20.0, 2.0), m + v2(26.0, -3.0)]),
    }
}
