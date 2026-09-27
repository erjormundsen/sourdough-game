//! Otto the otter: sleek, wiggly and snack-motivated. A Breton-striped shirt, puffy whisker
//! pads under a big nose, and his favourite pebble held in both paws.

use super::kit::*;
use crate::draw::{DrawList, PLATES_COLOR, Screen};
use crate::geom::{V2, capsule, circle, ellipse, quad_bezier, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const FUR: Coat = Coat::new(0.8, 0.6, 0.44, 0.0);
const MASK: Coat = Coat::new(0.26, 0.08, 0.0, 0.0);
const PAD: Coat = Coat::new(0.2, 0.06, 0.0, 0.0);
const STRIPE: f32 = 0.72;
const STONE: Coat = Coat::new(0.08, 0.04, 0.3, 0.2);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let hc = h + v2(0.0, 2.0);
    // Wide and flat-topped: a sleek otter skull, not a round bear head.
    let head = blobby(
        &[
            v2(0.0, -56.0),
            v2(46.0, -52.0),
            v2(76.0, -30.0),
            v2(87.0, 0.0),
            v2(78.0, 34.0),
            v2(48.0, 56.0),
            v2(0.0, 62.0),
            v2(-48.0, 56.0),
            v2(-78.0, 34.0),
            v2(-87.0, 0.0),
            v2(-76.0, -30.0),
            v2(-46.0, -52.0),
        ]
        .map(|p| hc + p),
    );
    // Tiny ears set low on the sides.
    let ears: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = hc + v2(sx * 82.0, -14.0);
            let tw = if sx > 0.0 { r.pose.twitch * 0.3 } else { 0.0 };
            let pivot = c + v2(-sx * 8.0, 6.0);
            (rot(&circle(c, 11.5), pivot, tw), rot(&circle(c + v2(-sx * 1.0, 1.0), 5.5), pivot, tw))
        })
        .collect();
    let mask = blobby(
        &[
            v2(0.0, -2.0),
            v2(24.0, -6.0),
            v2(50.0, 4.0),
            v2(66.0, 24.0),
            v2(58.0, 44.0),
            v2(32.0, 57.0),
            v2(0.0, 61.0),
            v2(-32.0, 57.0),
            v2(-58.0, 44.0),
            v2(-66.0, 24.0),
            v2(-50.0, 4.0),
            v2(-24.0, -6.0),
        ]
        .map(|p| hc + p),
    );
    let pads: Vec<Vec<V2>> =
        [-1.0f32, 1.0].iter().map(|&sx| ellipse(hc + v2(sx * 19.0, 25.0), 22.0, 17.0, sx * 0.12)).collect();

    let torso = r.breathe(&mound(&[
        v2(0.0, -126.0),
        v2(26.0, -122.0),
        v2(46.0, -110.0),
        v2(64.0, -101.0),
        v2(80.0, -85.0),
        v2(86.0, -54.0),
        v2(88.0, 0.0),
    ]));
    // Resting: paws round the pebble on the counter. Cheering: he hugs it up under his chin.
    let hug = r.cheering();
    let paths: Vec<Vec<V2>> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            if hug {
                let ctrl =
                    [v2(sx * 73.0, -92.0), v2(sx * 88.0, -70.0), v2(sx * 70.0, -70.0), v2(sx * 40.0, -84.0)];
                r.breathe(&spline(&ctrl, false, 6))
            } else {
                r.arm_path(sx, 84.0, 34.0)
            }
        })
        .collect();
    let arms: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(p, 17.0, 14.0)).collect();
    let paws: Vec<(V2, f32, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let (c, tilt) = if hug {
                (v2(sx * 27.0, -80.0), -sx * 0.5)
            } else if r.raised(sx) {
                r.paw_place(sx, 26.0, 84.0, 3.0)
            } else {
                (v2(sx * 26.0, REST_Y + 3.0), sx * 0.25)
            };
            (c, tilt, rot(&paw_shape(c, 26.0, 24.0), c, tilt))
        })
        .collect();
    let pebble_c = if hug { v2(0.0, -94.0) } else { v2(0.0, REST_Y - 11.0) };
    let pebble = ellipse(pebble_c, 22.0, 15.0, 0.08);

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head];
        sil.extend(arms.iter().map(|a| a.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        r.halo(d, &pebble);
        for (_, _, p) in &paws {
            r.halo(d, p);
        }
    }
    for (e, _) in &ears {
        r.halo(d, e);
    }
    r.halo(d, &head);

    if r.body {
        // Fur torso (a sleek neck above a wide boat neckline), pale throat, striped shirt.
        r.part(d, &torso, FUR);
        let throat = r.breathe(&ellipse(v2(0.0, -102.0), 42.0, 26.0, 0.0));
        d.clipped(&torso, |d| {
            MASK.ink_clean(d, &throat);
            d.outline(Ink::Key, r.detail(), &throat);
        });
        let neck = r.breathe(&quad_bezier(v2(-66.0, -100.0), v2(0.0, -76.0), v2(66.0, -100.0), 12));
        let mut shirt = neck.clone();
        shirt.extend([v2(96.0, -96.0), v2(96.0, 2.0), v2(-96.0, 2.0), v2(-96.0, -96.0)]);
        d.clipped(&torso, |d| {
            d.backing(&shirt);
            stripes(r, d, &shirt, V2::ZERO, 0.0);
            r.rim_shade(d, &shirt, v2(1.0, 0.3), 12.0, Ink::Blue, 0.2);
        });
        d.clipped(&torso, |d| r.seam(d, &neck));
        d.outline(Ink::Key, r.inner(), &torso);
        neck_shadow(r, d, &torso, 82.0, 60.0, 8.0);
    }
    let limbs = |d: &mut DrawList| {
        for (i, (a, open)) in arms.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            d.backing(a);
            stripes(r, d, a, paths[i][paths[i].len() / 2], sx * 0.22);
            r.rim_shade(d, a, v2(1.0, 0.5), 7.0, Ink::Blue, 0.2);
            d.line(Ink::Key, r.inner(), open);
            elbow_creases(r, d, a, &paths[i], 15.0, false);
        }
        // The pebble, held in both paws.
        r.part_with(d, &pebble, |d, p| {
            STONE.ink(d, p);
            r.rim_shade(d, p, v2(0.6, 1.0), 5.0, Ink::Key, 0.3);
            if !r.small() {
                for (x, y) in [(-9.0, -3.0), (6.0, 4.0), (12.0, -5.0), (-2.0, 7.0)] {
                    d.fill(Ink::Key, 0.5, &circle(pebble_c + v2(x, y), 1.3));
                }
                d.knock_p(
                    1.0,
                    Screen::Solid,
                    PLATES_COLOR,
                    &ellipse(pebble_c + v2(-8.0, -8.0), 5.0, 2.6, -0.2),
                );
            }
        });
        for (i, (c, tilt, p)) in paws.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part(d, p, FUR);
            toes_at(r, d, *c, 26.0, 24.0, 3, *tilt, hug || r.raised(sx));
            r.wave_marks(d, *c + v2(0.0, -24.0), sx);
        }
    };
    if r.body && !r.limbs_in_front() {
        limbs(d);
    }

    for (e, inner) in &ears {
        r.part(d, e, FUR);
        MASK.ink_clean(d, inner);
    }
    r.part_with(d, &head, |d, p| {
        FUR.ink(d, p);
        // Sleek, glossy fur: a paper sheen across the crown.
        if !r.small() {
            d.knock_p(
                0.5,
                Screen::Solid,
                PLATES_COLOR,
                &capsule(hc + v2(-44.0, -34.0), hc + v2(-20.0, -45.0), 4.5),
            );
            d.knock_p(0.5, Screen::Solid, PLATES_COLOR, &circle(hc + v2(-8.0, -48.0), 3.0));
        }
    });
    r.part(d, &mask, MASK);
    // Puffy whisker pads, freckled with whisker dots.
    for p in &pads {
        r.part(d, p, PAD);
    }
    if !r.small() {
        for sx in [-1.0f32, 1.0] {
            for (x, y) in [(15.0, 24.0), (24.0, 20.0), (25.0, 30.0), (33.0, 25.0)] {
                d.fill(Ink::Key, 0.75, &circle(hc + v2(sx * x, y), 1.7));
            }
        }
    }
    // Broad nose sitting on top of the pads.
    let nose_c = hc + v2(0.0, 10.0);
    let nose = blobby(&[
        nose_c + v2(-17.0, -7.0),
        nose_c + v2(0.0, -9.0),
        nose_c + v2(17.0, -7.0),
        nose_c + v2(12.0, 4.0),
        nose_c + v2(0.0, 9.0),
        nose_c + v2(-12.0, 4.0),
    ]);
    d.fill(Ink::Key, 1.0, &nose);
    d.knock(&ellipse(nose_c + v2(-6.0, -4.0), 4.5, 2.2, -0.15));
    let m = hc + v2(0.0, 45.0);
    r.clear_cheeks(d, hc + v2(0.0, -14.0), 92.0, 0.5);
    r.face(d, hc + v2(0.0, -14.0), 92.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, m, 80.0, r.expr);
    // Whiskers sprout from the pads.
    if !r.small() {
        for sx in [-1.0f32, 1.0] {
            for (i, (y0, y1)) in [(20.0f32, 10.0f32), (26.0, 25.0), (32.0, 40.0)].iter().enumerate() {
                let a = hc + v2(sx * 36.0, *y0);
                let b = hc + v2(sx * (106.0 - i as f32 * 6.0), *y1 + r.pose.twitch * 3.0);
                r.detail_line(d, &bow(a, b, sx * 2.5, 8));
            }
        }
    }
    if r.limbs_in_front() {
        limbs(d);
    }
}

/// Breton stripes that curve with the body (or run around an arm when `tilt` is set).
fn stripes(r: &Rig, d: &mut DrawList, clip: &[V2], pivot: V2, tilt: f32) {
    let Some(b) = crate::geom::Rect::of_points(clip) else { return };
    let pitch = 17.0;
    let (ink_t, w) = if r.small() { (STRIPE * 0.7, pitch * 0.5) } else { (STRIPE, pitch * 0.42) };
    d.clipped(clip, |d| {
        if tilt == 0.0 {
            // Stripes sag a little towards the middle, following the chest.
            let (x0, x1) = (b.x - 4.0, b.x + b.w + 4.0);
            let j0 = ((b.y - 14.0) / pitch).floor() as i32;
            let j1 = ((b.y + b.h + 4.0) / pitch).ceil() as i32;
            for j in j0..=j1 {
                let y = j as f32 * pitch - 6.0;
                let top = quad_bezier(v2(x0, y - 6.0), v2((x0 + x1) * 0.5, y + 8.0), v2(x1, y - 6.0), 10);
                let mut p = top.clone();
                p.extend(top.iter().rev().map(|q| *q + v2(0.0, w)));
                d.fill(Ink::Blue, ink_t, &p);
            }
        } else {
            for band in bands(clip, PI * 0.5 + tilt, pitch, w, pivot.y) {
                d.fill(Ink::Blue, ink_t, &band);
            }
        }
    });
}
