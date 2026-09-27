//! Clover the sheep: one big cloud of wool, a hand-knit scarf and a little brass bell.
//! Mild and fluffy, like her favourite loaves.

use super::kit::*;
use crate::draw::{DrawList, PLATES_ALL, Screen};
use crate::geom::{V2, circle, ellipse, quad_bezier, v2};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::PI;

const WOOL: Coat = Coat::new(0.05, 0.02, 0.0, 0.0);
const FACE: Coat = Coat::new(0.32, 0.13, 0.0, 0.0);
const EAR_IN: Coat = Coat::new(0.1, 0.46, 0.0, 0.0);
const KNIT: Coat = Coat::new(0.0, 0.06, 0.56, 0.0);
const HOOF: Coat = Coat::new(0.1, 0.16, 0.1, 0.62);
const BELL: Coat = Coat::new(0.95, 0.2, 0.0, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let wool = r.breathe(&scalloped(
        &mound(&[
            v2(0.0, -128.0),
            v2(36.0, -124.0),
            v2(62.0, -114.0),
            v2(82.0, -104.0),
            v2(97.0, -82.0),
            v2(102.0, -50.0),
            v2(104.0, 0.0),
        ]),
        false,
        24.0,
        7.0,
    ));
    let arms: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let path = r.arm_path(sx, 88.0, 42.0);
            let (poly, open) = tube_parts(&path, 17.0, 14.0);
            let sign = if crate::geom::signed_area(&poly) > 0.0 { -1.0 } else { 1.0 };
            // Fleecy edge on the arm; the shoulder end stays unlined so it melts into the body.
            let fluffy = scalloped_with(&open, false, 14.0, 4.0, sign);
            let mut closed = fluffy.clone();
            closed.extend_from_slice(&poly[open.len()..]);
            (closed, fluffy)
        })
        .collect();
    let hooves: Vec<(V2, f32, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let (c, tilt) = r.paw_place(sx, 38.0, 88.0, 4.0);
            (c, tilt, rot(&paw_shape(c, 26.0, 22.0), c, tilt))
        })
        .collect();

    let ears: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = h + v2(sx * 62.0, 6.0);
            let tw = if sx < 0.0 { -r.pose.twitch * 0.2 } else { 0.0 };
            let pivot = h + v2(sx * 40.0, 0.0);
            (
                rot(&ellipse(c, 26.0, 12.5, sx * 0.42), pivot, tw),
                rot(&ellipse(c + v2(sx * 2.0, 1.0), 16.0, 6.0, sx * 0.42), pivot, tw),
            )
        })
        .collect();
    let face = ellipse(h + v2(0.0, 16.0), 52.0, 60.0, 0.0);
    let k = if r.portrait { 0.92 } else { 1.0 };
    let cap = scalloped(&ellipse(h + v2(0.0, -38.0 * k), 66.0 * k, 36.0 * k, 0.0), true, 22.0, 8.0);

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&wool, &cap, &face];
        sil.extend(arms.iter().map(|a| a.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &wool);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for (_, _, p) in &hooves {
            r.halo(d, p);
        }
    }
    for (e, _) in &ears {
        r.halo(d, e);
    }
    r.halo(d, &face);
    r.halo(d, &cap);

    if r.body {
        r.part_with(d, &wool, |d, p| {
            WOOL.ink(d, p);
            r.rim_shade(d, p, v2(1.0, 0.5), 12.0, Ink::Blue, 0.22);
        });
        curls(r, d, &wool, crate::geom::rect(-44.0, -76.0, 88.0, 30.0), 4, 7);
        scarf(r, d);
    }
    let limbs = |d: &mut DrawList| {
        for (i, (a, open)) in arms.iter().enumerate() {
            d.backing(a);
            WOOL.ink(d, a);
            r.rim_shade(d, a, v2(0.8, 1.0), 7.0, Ink::Blue, 0.22);
            d.line(Ink::Key, r.inner(), open);
            if let Some(b) = crate::geom::Rect::of_points(a) {
                let area =
                    crate::geom::rect(b.x + 8.0, b.y + 10.0, (b.w - 16.0).max(1.0), (b.h * 0.5).max(1.0));
                curls(r, d, a, area, 2, 31 + i as u32);
            }
        }
        for (i, (c, tilt, p)) in hooves.iter().enumerate() {
            r.wave_marks(d, *c + v2(0.0, -22.0), if i == 0 { -1.0 } else { 1.0 });
            r.part(d, p, HOOF);
            if !r.small() {
                let cleft = rot(&[*c + v2(0.0, 1.0), *c + v2(0.0, -10.0)], *c, *tilt);
                d.knock_line(1.0, r.detail(), &cleft, false);
                let glint = rot(&ellipse(*c + v2(-6.0, -15.0), 3.5, 2.2, -0.4), *c, *tilt);
                d.knock_p(0.8, Screen::Solid, PLATES_ALL, &glint);
            }
        }
    };
    if r.body && !r.limbs_in_front() {
        limbs(d);
    }

    for (e, inner) in &ears {
        r.part(d, e, FACE);
        EAR_IN.ink_clean(d, inner);
    }
    r.part(d, &face, FACE);
    r.part_with(d, &cap, |d, p| {
        WOOL.ink(d, p);
        r.rim_shade(d, p, v2(0.7, 1.0), 9.0, Ink::Blue, 0.2);
    });
    let cb = crate::geom::Rect::of_points(&cap).unwrap_or_default();
    curls(r, d, &cap, crate::geom::rect(cb.x + 16.0, cb.y + 12.0, cb.w - 32.0, cb.h * 0.45), 4, 3);

    // Face: tiny nose, the shared grammar.
    let nose_c = h + v2(0.0, 36.0);
    let nose = blobby(&[nose_c + v2(-7.0, -3.0), nose_c + v2(7.0, -3.0), nose_c + v2(0.0, 4.5)]);
    d.fill(Ink::Key, 1.0, &nose);
    let m = h + v2(0.0, 45.0);
    r.detail_line(d, &[nose_c + v2(0.0, 4.0), m + v2(0.0, -1.5)]);
    r.face(d, h + v2(0.0, 16.0), 88.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, m, 76.0, r.expr);
    if r.limbs_in_front() {
        limbs(d);
    }
}

/// Wool curls: little spirals scattered over `area` (x, y, w, h), kept inside `clip`.
fn curls(r: &Rig, d: &mut DrawList, clip: &[V2], area: crate::geom::Rect, n: u32, seed: u32) {
    if r.small() {
        return;
    }
    d.clipped(clip, |d| {
        for i in 0..n {
            let p = v2(area.x + hash01(seed, i) * area.w, area.y + hash01(seed + 1, i) * area.h);
            let a0 = hash01(seed + 2, i) * PI * 2.0;
            let flip = if hash01(seed + 3, i) > 0.5 { 1.0 } else { -1.0 };
            let spiral: Vec<V2> = (0..=14)
                .map(|k| {
                    let t = k as f32 / 14.0;
                    p + V2::from_angle(a0 + flip * t * PI * 1.7) * (6.5 - 3.8 * t)
                })
                .collect();
            r.detail_line(d, &spiral);
        }
    });
}

fn scarf(r: &Rig, d: &mut DrawList) {
    let band = r.breathe(&blobby(&[
        v2(-66.0, -122.0),
        v2(0.0, -128.0),
        v2(66.0, -122.0),
        v2(70.0, -102.0),
        v2(40.0, -92.0),
        v2(0.0, -88.0),
        v2(-40.0, -92.0),
        v2(-70.0, -102.0),
    ]));
    r.part_with(d, &band, |d, p| {
        KNIT.ink(d, p);
        ribbing(r, d, p);
        r.rim_shade(d, p, v2(0.0, 1.0), 6.0, Ink::Key, 0.25);
    });
    // The loose end, hanging down the front with a fringe.
    let end = r.breathe(&[v2(28.0, -100.0), v2(54.0, -98.0), v2(57.0, -64.0), v2(33.0, -62.0)]);
    for i in 0..5 {
        let a = end[3].lerp(end[2], 0.1 + i as f32 * 0.2) + v2(0.0, -1.0);
        r.detail_line(d, &[a, a + v2(0.8, 10.0)]);
    }
    r.part_with(d, &end, |d, p| {
        KNIT.ink(d, p);
        ribbing(r, d, p);
        r.rim_shade(d, p, v2(1.0, 0.0), 6.0, Ink::Key, 0.25);
    });
    // Brass bell on a loop.
    let c = r.breathe(&[v2(-8.0, -80.0)])[0];
    d.outline(Ink::Key, r.detail(), &circle(c + v2(0.0, -12.0), 4.0));
    let bell = blobby(&[
        c + v2(0.0, -10.0),
        c + v2(9.0, -6.0),
        c + v2(11.0, 6.0),
        c + v2(15.0, 12.0),
        c + v2(0.0, 14.0),
        c + v2(-15.0, 12.0),
        c + v2(-11.0, 6.0),
        c + v2(-9.0, -6.0),
    ]);
    r.halo(d, &bell);
    r.part_with(d, &bell, |d, p| {
        BELL.ink(d, p);
        r.rim_shade(d, p, v2(1.0, 0.6), 4.0, Ink::Pink, 0.45);
        if !r.small() {
            d.knock_p(0.9, Screen::Solid, 0b0111, &ellipse(c + v2(-4.5, -1.0), 2.4, 4.5, 0.2));
        }
    });
    r.detail_line(d, &quad_bezier(c + v2(-12.0, 9.0), c + v2(0.0, 12.0), c + v2(12.0, 9.0), 6));
    d.fill(Ink::Key, 1.0, &circle(c + v2(0.0, 14.5), 2.8));
}

/// Knit ribbing: soft vertical ribs clipped to the knitted piece.
fn ribbing(r: &Rig, d: &mut DrawList, clip: &[V2]) {
    if r.small() {
        return;
    }
    let Some(b) = crate::geom::Rect::of_points(clip) else { return };
    d.clipped(clip, |d| {
        let mut x = (b.x / 7.0).floor() * 7.0;
        while x < b.x + b.w {
            let pts = [v2(x, b.y - 1.0), v2(x + 2.0, b.y + b.h + 1.0)];
            d.stroke_p(crate::draw::Paint::solid(Ink::Key, 0.35), r.detail() * 0.8, &pts, false);
            x += 7.0;
        }
    });
}
