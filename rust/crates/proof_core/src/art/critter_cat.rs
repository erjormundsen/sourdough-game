//! Sir Whiskers: the critic. Blue-grey tom in a tailcoat with a monocle on a gold chain,
//! notebook in one paw and pen in the other. Impress him.

use super::kit::*;
use crate::art::Expr;
use crate::draw::{DrawList, PLATES_COLOR, Screen};
use crate::geom::{V2, Xf, capsule, circle, ellipse, rect, rounded_rect, soft_star, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const FUR: Coat = Coat::new(0.0, 0.05, 0.36, 0.12);
const EAR_IN: Coat = Coat::new(0.08, 0.5, 0.0, 0.0);
const COAT: Coat = Coat::new(0.0, 0.0, 0.28, 0.72);
const LAPEL: Coat = Coat::new(0.0, 0.0, 0.45, 0.6);
const BOW: Coat = Coat::new(0.08, 0.95, 0.0, 0.0);
const GOLD: Coat = Coat::new(0.9, 0.22, 0.0, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let head = furry_oval(h + v2(0.0, 4.0), 86.0, 67.0, &[(0.2, 0.5, 11.0, 3), (PI - 0.2, 0.5, 11.0, 3)]);
    let ears: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let pivot = h + v2(sx * 50.0, -44.0);
            let tw = if sx > 0.0 { r.pose.twitch * 0.2 } else { 0.0 };
            let outer = blobby(&[
                h + v2(sx * 18.0, -50.0),
                h + v2(sx * 50.0, -92.0),
                h + v2(sx * 64.0, -108.0),
                h + v2(sx * 72.0, -104.0),
                h + v2(sx * 80.0, -62.0),
                h + v2(sx * 82.0, -24.0),
                h + v2(sx * 50.0, -30.0),
            ]);
            let inner = blobby(&[
                h + v2(sx * 34.0, -52.0),
                h + v2(sx * 56.0, -86.0),
                h + v2(sx * 64.0, -94.0),
                h + v2(sx * 68.0, -84.0),
                h + v2(sx * 70.0, -46.0),
            ]);
            (rot(&outer, pivot, tw * sx), rot(&inner, pivot, tw * sx))
        })
        .collect();

    let torso = r.breathe(&mound(&[
        v2(0.0, -130.0),
        v2(30.0, -126.0),
        v2(52.0, -118.0),
        v2(70.0, -115.0),
        v2(86.0, -96.0),
        v2(92.0, -60.0),
        v2(94.0, 0.0),
    ]));
    let paths: Vec<Vec<V2>> = [-1.0f32, 1.0].iter().map(|&sx| r.arm_path(sx, 88.0, 44.0)).collect();
    let arms: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(p, 18.5, 15.0)).collect();
    let paws: Vec<(V2, f32, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let (c, tilt) = r.paw_place(sx, 40.0, 88.0, 4.0);
            (c, tilt, rot(&paw_shape(c, 31.0, 26.0), c, tilt))
        })
        .collect();

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head];
        sil.extend(arms.iter().map(|a| a.0.as_slice()));
        sil.extend(ears.iter().map(|e| e.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for (_, _, p) in &paws {
            r.halo(d, p);
        }
    }
    for (e, _) in &ears {
        r.halo(d, e);
    }
    r.halo(d, &head);

    let limbs = |d: &mut DrawList| {
        for (i, (poly, open)) in arms.iter().enumerate() {
            d.backing(poly);
            COAT.ink(d, poly);
            r.rim_shade(d, poly, v2(1.0, 0.4), 8.0, Ink::Key, 0.25);
            d.line(Ink::Key, r.inner(), open);
            elbow_creases(r, d, poly, &paths[i], 16.0, true);
        }
        if !r.limbs_in_front() {
            notebook(r, d);
            pen(r, d, v2(44.0, -40.0), v2(10.0, -70.0));
        }
        for (i, (c, tilt, p)) in paws.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part(d, p, Coat::PAPER);
            toes_at(r, d, *c, 32.0, 26.0, 3, *tilt, r.raised(sx));
            r.wave_marks(d, *c + v2(0.0, -26.0), sx);
        }
    };
    if r.body {
        tailcoat(r, d, &torso);
        if r.limbs_in_front() {
            // The notebook stays propped on the counter while his paws are busy.
            notebook(r, d);
        } else {
            limbs(d);
        }
    }

    for (e, inner) in &ears {
        r.part(d, e, FUR);
        EAR_IN.ink_clean(d, inner);
    }
    if !r.small() {
        for (i, (_, inner)) in ears.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            let base = inner[0].lerp(inner[inner.len() / 2], 0.5) + v2(sx * 10.0, 16.0);
            fur_flick(r, d, base, v2(sx * 0.35, -1.0).norm(), 14.0, 3);
        }
    }
    r.part(d, &head, FUR);

    // White muzzle and chin.
    let muzzle = blobby(
        &[
            v2(-31.0, 13.0),
            v2(-14.0, 7.0),
            v2(0.0, 12.0),
            v2(14.0, 7.0),
            v2(31.0, 13.0),
            v2(37.0, 29.0),
            v2(22.0, 43.0),
            v2(0.0, 47.0),
            v2(-22.0, 43.0),
            v2(-37.0, 29.0),
        ]
        .map(|p| h + p),
    );
    r.part(d, &muzzle, Coat::PAPER);
    let nose_c = h + v2(0.0, 16.5);
    let nose = blobby(&[nose_c + v2(-7.0, -3.5), nose_c + v2(7.0, -3.5), nose_c + v2(0.0, 4.5)]);
    d.fill(Ink::Pink, 0.95, &nose);
    d.outline(Ink::Key, r.detail(), &nose);
    let m = h + v2(0.0, 27.0);
    r.detail_line(d, &[nose_c + v2(0.0, 4.0), m + v2(0.0, -1.5)]);

    let fc = h + v2(0.0, 2.0);
    r.clear_cheeks(d, fc, 104.0, 0.85);
    r.face(d, fc, 104.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, m, 84.0, r.expr);
    if r.expr == Expr::Content && !r.blinking() {
        // A permanently raised brow over the monocle.
        let b = fc + v2(27.0, -30.0);
        d.line(Ink::Key, r.detail() * 1.4, &crate::geom::arc(b + v2(0.0, 8.0), 11.0, PI * 1.22, PI * 1.78));
    }

    if r.limbs_in_front() {
        limbs(d);
    }

    // Whiskers (they twitch).
    if !r.small() {
        for sx in [-1.0f32, 1.0] {
            for (i, (y0, y1)) in [(22.0f32, 12.0f32), (28.0, 28.0), (34.0, 43.0)].iter().enumerate() {
                let a = h + v2(sx * 34.0, *y0);
                let b = h + v2(sx * (104.0 - i as f32 * 5.0), *y1 + r.pose.twitch * 3.0 * sx.max(0.0));
                r.detail_line(d, &bow(a, b, sx * 3.0, 8));
            }
        }
    }

    monocle(r, d, fc + v2(26.0, -2.0));
}

fn tailcoat(r: &Rig, d: &mut DrawList, torso: &[V2]) {
    r.part_with(d, torso, |d, p| {
        COAT.ink(d, p);
        r.rim_shade(d, p, v2(1.0, 0.3), 12.0, Ink::Key, 0.22);
    });
    // Shirt front in the V of the lapels.
    let shirt = r.breathe(&[v2(-28.0, -126.0), v2(28.0, -126.0), v2(5.0, -36.0), v2(-5.0, -36.0)]);
    r.part(d, &shirt, Coat::PAPER);
    r.stitches(d, &r.breathe(&[v2(0.0, -80.0), v2(0.0, -40.0)]), 2.5, 5.0);
    neck_shadow(r, d, &shirt, 86.0, 67.0, 9.0);
    // Peak lapels.
    for sx in [-1.0f32, 1.0] {
        let l = r.breathe(&[
            v2(sx * 21.0, -128.0),
            v2(sx * 5.0, -36.0),
            v2(sx * 26.0, -58.0),
            v2(sx * 48.0, -92.0),
            v2(sx * 36.0, -96.0),
            v2(sx * 44.0, -110.0),
            v2(sx * 34.0, -128.0),
        ]);
        r.part_with(d, &l, |d, p| {
            LAPEL.ink(d, p);
            r.rim_shade(d, p, v2(-sx, 0.4), 4.0, Ink::Key, 0.3);
        });
    }
    // Welt pocket with a yellow pocket square.
    let pk = r.breathe(&[v2(54.0, -70.0)])[0];
    let sq = blobby(&[
        pk + v2(-12.0, 1.0),
        pk + v2(-9.0, -9.0),
        pk + v2(-4.0, -4.0),
        pk + v2(1.0, -13.0),
        pk + v2(6.0, -5.0),
        pk + v2(12.0, -9.0),
        pk + v2(12.0, 1.0),
    ]);
    r.part(d, &sq, Coat::new(0.9, 0.1, 0.0, 0.0));
    r.seam(d, &[pk + v2(-17.0, 1.0), pk + v2(17.0, -1.0)]);
    // Bow tie tucked under the chin.
    let c = r.breathe(&[v2(0.0, -91.0)])[0];
    for sx in [-1.0f32, 1.0] {
        let wing = blobby(&[
            c + v2(sx * 2.0, 0.0),
            c + v2(sx * 20.0, -10.0),
            c + v2(sx * 23.0, 0.0),
            c + v2(sx * 20.0, 10.0),
        ]);
        r.part(d, &wing, BOW);
        r.detail_line(d, &bow(c + v2(sx * 8.0, -3.0), c + v2(sx * 16.0, -5.0), 0.0, 2));
    }
    let knot = ellipse(c, 6.5, 7.5, 0.0);
    r.part(d, &knot, Coat::new(0.1, 1.0, 0.0, 0.12));
}

/// The critic's notepad, propped on the counter: spiral top, a three-star rating.
fn notebook(r: &Rig, d: &mut DrawList) {
    let at = r.breathe(&[v2(-36.0, -52.0)])[0];
    d.with(Xf::at(at).rotated(0.1), |d| {
        let cover = rounded_rect(rect(-21.0, -26.0, 42.0, 54.0), 5.0);
        r.halo(d, &cover);
        r.part(d, &cover, Coat::new(0.22, 0.7, 0.0, 0.0));
        let page = rounded_rect(rect(-17.0, -20.0, 34.0, 46.0), 3.0);
        r.part(d, &page, Coat::PAPER);
        if !r.small() {
            for i in 0..3 {
                let s = soft_star(v2(-10.0 + i as f32 * 10.0, -8.0), 5.5, 2.4, 5, 0.0);
                if i < 2 {
                    d.fill(Ink::Yellow, 1.0, &s);
                }
                d.outline(Ink::Key, r.detail() * 0.7, &s);
            }
            for y in [4.0, 11.0] {
                r.detail_line(d, &[v2(-11.0, y), v2(10.0 - y * 0.3, y)]);
            }
            for i in 0..5 {
                let x = -14.0 + i as f32 * 7.0;
                d.outline(Ink::Key, r.detail() * 0.8, &ellipse(v2(x, -22.0), 2.2, 4.0, 0.0));
            }
        }
    });
}

fn pen(r: &Rig, d: &mut DrawList, grip: V2, tip: V2) {
    let (grip, tip) = {
        let p = r.breathe(&[grip, tip]);
        (p[0], p[1])
    };
    let dir = (tip - grip).norm();
    let body = capsule(grip - dir * 10.0, tip - dir * 9.0, 3.8);
    r.halo(d, &body);
    r.part(d, &body, Coat::new(0.0, 0.0, 0.7, 0.2));
    let nib = vec![tip - dir * 9.0 + dir.perp() * 3.6, tip, tip - dir * 9.0 - dir.perp() * 3.6];
    r.halo(d, &nib);
    r.part(d, &nib, GOLD);
    let clip = capsule(grip - dir * 6.0 + dir.perp() * 3.0, grip + dir * 8.0 + dir.perp() * 3.0, 1.2);
    d.fill(Ink::Yellow, 0.9, &clip);
}

/// Gold-rimmed monocle over the right eye, chain looping down to the lapel.
fn monocle(r: &Rig, d: &mut DrawList, c: V2) {
    let ring = circle(c, 21.0);
    // Glass: lifts the fur behind it a touch and adds a cool, smooth sheen.
    d.knock_p(0.3, Screen::Solid, PLATES_COLOR, &ring);
    d.fill_p(crate::draw::Paint::solid(Ink::Blue, 0.1).add(), &ring);
    if !r.small() {
        d.knock_line(0.8, 2.6, &crate::geom::arc(c, 14.5, PI * 1.08, PI * 1.42), false);
    }
    d.outline(Ink::Key, r.inner() * 1.55, &ring);
    d.stroke_p(crate::draw::Paint::solid(Ink::Yellow, 1.0), r.inner() * 0.62, &ring, true);
    d.stroke_p(crate::draw::Paint::solid(Ink::Pink, 0.2), r.inner() * 0.62, &ring, true);
    // Chain: drapes from the rim over the jaw to a lapel buttonhole (or a short loop).
    let a = c + V2::from_angle(PI * 0.36) * 21.0;
    let chain = if r.body {
        let end = r.breathe(&[v2(40.0, -101.0)])[0];
        bow(a, end, -7.0, 14)
    } else {
        bow(a, a + v2(8.0, 34.0), -8.0, 10)
    };
    d.line(Ink::Key, r.detail() * 1.45, &chain);
    if !r.small() {
        for p in crate::geom::resample(&chain, 11).iter().skip(1) {
            d.fill(Ink::Yellow, 1.0, &circle(*p, r.detail() * 0.62));
        }
    }
    if r.body {
        let end = *chain.last().unwrap_or(&a);
        d.fill(Ink::Key, 1.0, &ellipse(end, r.detail() * 1.4, r.detail() * 2.2, 0.3));
    }
}
