//! Mimi the bunny: sweet and gentle, in a polka-dot dress with puff sleeves, a Peter Pan
//! collar and a ribbon on her floppy ear.

use super::kit::*;
use crate::draw::{DrawList, PLATES_COLOR, Screen};
use crate::geom::{V2, circle, ellipse, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const FUR: Coat = Coat::new(0.04, 0.02, 0.0, 0.0);
const EAR_IN: Coat = Coat::new(0.08, 0.5, 0.0, 0.0);
const DRESS: Coat = Coat::new(0.06, 0.46, 0.0, 0.0);
const RIBBON: Coat = Coat::new(0.0, 0.08, 0.55, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let head_c = h + v2(0.0, 4.0);
    let head = furry_oval(head_c, 75.0, 65.0, &[(0.4, 0.34, 6.0, 2), (PI - 0.4, 0.34, 6.0, 2)]);

    // Ears: the left stands tall (and twitches); the right folds over at the tip.
    let k = if r.portrait { 0.62 } else { 1.0 };
    let tall_base = h + v2(-24.0, -44.0);
    let tall_path = rot(
        &spline(&[tall_base, h + v2(-31.0, -44.0 - 56.0 * k), h + v2(-35.0, -44.0 - 96.0 * k)], false, 6),
        tall_base,
        -0.06 + r.pose.twitch * 0.08 - if r.portrait { 0.18 } else { 0.0 },
    );
    let fold_base = h + v2(24.0, -44.0);
    let bend = h + v2(35.0, -44.0 - 78.0 * k);
    let stand_path = spline(&[fold_base, h + v2(30.0, -44.0 - 42.0 * k), bend], false, 6);
    // The folded-over tip: a soft droopy flap hinged at the bend.
    let flop = blobby(&[
        bend + v2(-15.0, 2.0),
        bend + v2(-6.0, -12.0),
        bend + v2(14.0, -15.0),
        bend + v2(34.0, -5.0),
        bend + v2(46.0, 13.0),
        bend + v2(44.0, 25.0),
        bend + v2(30.0, 23.0),
        bend + v2(12.0, 12.0),
    ]);
    let tall = tube(&tall_path, 15.0, 19.0);
    let stand = tube(&stand_path, 15.0, 17.0);
    let tall_in = tube(&tall_path[1..tall_path.len() - 2], 6.5, 9.0);
    let stand_in = tube(&stand_path[1..], 6.5, 8.5);

    // Body.
    let torso = r.breathe(&mound(&[
        v2(0.0, -126.0),
        v2(28.0, -122.0),
        v2(48.0, -113.0),
        v2(62.0, -108.0),
        v2(76.0, -92.0),
        v2(82.0, -62.0),
        v2(84.0, 0.0),
    ]));
    let arms: Vec<(Vec<V2>, Vec<V2>)> =
        [-1.0f32, 1.0].iter().map(|&sx| tube_parts(&r.arm_path(sx, 76.0, 40.0), 12.0, 10.5)).collect();
    let puffs: Vec<Vec<V2>> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = v2(sx * 66.0, -97.0);
            r.breathe(&blobby(&[
                c + v2(sx * -20.0, -6.0),
                c + v2(sx * -4.0, -18.0),
                c + v2(sx * 14.0, -14.0),
                c + v2(sx * 21.0, 3.0),
                c + v2(sx * 13.0, 17.0),
                c + v2(sx * -4.0, 18.0),
                c + v2(sx * -17.0, 8.0),
            ]))
        })
        .collect();
    let paws: Vec<(V2, f32, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let (c, tilt) = r.paw_place(sx, 35.0, 76.0, 4.0);
            (c, tilt, rot(&paw_shape(c, 30.0, 25.0), c, tilt))
        })
        .collect();

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head, &tall, &stand, &flop];
        sil.extend(puffs.iter().map(|p| p.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for p in &puffs {
            r.halo(d, p);
        }
        for (_, _, p) in &paws {
            r.halo(d, p);
        }
    }
    for e in [&tall, &stand, &flop] {
        r.halo(d, e);
    }
    r.halo(d, &head);

    if r.body {
        r.part_with(d, &torso, |d, p| {
            DRESS.ink(d, p);
            polka(r, d, p, 20.0, 3.3);
            r.rim_shade(d, p, v2(1.0, 0.4), 10.0, Ink::Pink, 0.3);
        });
        neck_shadow(r, d, &torso, 75.0, 65.0, 9.0);
        // Peter Pan collar: two round lobes meeting under the chin.
        for sx in [-1.0f32, 1.0] {
            let c = r.breathe(&blobby(&[
                v2(sx * 2.0, -120.0),
                v2(sx * 32.0, -118.0),
                v2(sx * 48.0, -104.0),
                v2(sx * 42.0, -89.0),
                v2(sx * 22.0, -85.0),
                v2(sx * 5.0, -95.0),
            ]));
            r.part(d, &c, Coat::PAPER);
            r.stitches(d, &inset(&c, 0.8, true), 3.0, 3.0);
        }
        for y in [-72.0, -52.0] {
            let b = r.breathe(&[v2(0.0, y)])[0];
            button(r, d, b, 4.6, Coat::new(0.0, 0.12, 0.0, 0.0));
        }
    }
    // Arms, puff sleeves and paws: on the counter, or (cheering) up at the cheeks.
    let limbs = |d: &mut DrawList| {
        for (a, open) in &arms {
            d.backing(a);
            FUR.ink(d, a);
            d.line(Ink::Key, r.inner(), open);
        }
        for (i, p) in puffs.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part_with(d, p, |d, p| {
                DRESS.ink(d, p);
                polka(r, d, p, 20.0, 3.3);
                r.rim_shade(d, p, v2(0.4, 1.0), 6.0, Ink::Pink, 0.35);
            });
            // Soft gathers fanning into a little cuff band.
            let c = r.breathe(&[v2(sx * 66.0, -97.0)])[0];
            let band = bow(c + v2(-sx * 12.0, 14.0), c + v2(sx * 16.0, 10.0), -3.0 * sx, 6);
            if !r.small() {
                for (k, t) in [0.3f32, 0.7].iter().enumerate() {
                    let a = crate::geom::resample(&band, 11)[(t * 10.0) as usize];
                    let top = c + v2(sx * (k as f32 * 10.0 - 4.0), -6.0);
                    r.detail_line(d, &bow(a + v2(0.0, -2.0), top, sx * 2.0, 5));
                }
            }
            r.seam(d, &band);
        }
        for (i, (c, tilt, p)) in paws.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part(d, p, FUR);
            toes_at(r, d, *c, 30.0, 25.0, 3, *tilt, r.raised(sx));
            r.wave_marks(d, *c + v2(0.0, -25.0), sx);
        }
    };
    if r.body && !r.limbs_in_front() {
        limbs(d);
    }

    // Ears (behind the head): tall ear, then the folded ear with its flap on top.
    r.part(d, &tall, FUR);
    d.clipped(&tall, |d| EAR_IN.ink(d, &tall_in));
    r.part(d, &stand, FUR);
    d.clipped(&stand, |d| EAR_IN.ink(d, &stand_in));
    r.shade(d, &stand, &shift(&flop, v2(3.0, 7.0)), Ink::Blue, 0.34);
    r.part(d, &flop, FUR);

    r.part(d, &head, FUR);

    // Face: pink nose, whisker freckles, the shared grammar.
    let nose_c = h + v2(0.0, 18.0);
    let nose = blobby(&[nose_c + v2(-7.5, -3.5), nose_c + v2(7.5, -3.5), nose_c + v2(0.0, 5.0)]);
    d.backing(&nose);
    d.fill(Ink::Pink, 0.95, &nose);
    d.outline(Ink::Key, r.detail(), &nose);
    if !r.small() {
        for sx in [-1.0f32, 1.0] {
            for (dx, dy) in [(15.0, 22.0), (21.0, 18.0), (21.0, 27.0)] {
                d.fill(Ink::Key, 0.8, &circle(h + v2(sx * dx, dy), 1.5));
            }
        }
    }
    let m = h + v2(0.0, 28.0);
    r.detail_line(d, &[nose_c + v2(0.0, 4.0), m + v2(0.0, -1.5)]);
    r.face(d, h + v2(0.0, 5.0), 100.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, m, 84.0, r.expr);

    ribbon(r, d, fold_base + v2(4.0, -6.0));
    if r.limbs_in_front() {
        limbs(d);
    }
}

/// Paper polka dots knocked out of the dress (the key plate survives).
fn polka(r: &Rig, d: &mut DrawList, clip: &[V2], pitch: f32, rad: f32) {
    if r.small() {
        return;
    }
    let dots: Vec<Vec<V2>> =
        dot_grid(clip, pitch, pitch * 0.86, rad).iter().map(|c| circle(*c, rad)).collect();
    d.clipped(clip, |d| knock_many(d, PLATES_COLOR, &dots));
}

/// A blue ribbon bow with paper dots, tied at the base of the floppy ear.
fn ribbon(r: &Rig, d: &mut DrawList, c: V2) {
    let loops: Vec<Vec<V2>> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            blobby(&[
                c + v2(sx * 3.0, -2.0),
                c + v2(sx * 15.0, -14.0),
                c + v2(sx * 25.0, -11.0),
                c + v2(sx * 26.0, 4.0),
                c + v2(sx * 15.0, 8.0),
                c + v2(sx * 3.0, 3.0),
            ])
        })
        .collect();
    let tails: Vec<Vec<V2>> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            blobby(&[
                c + v2(sx * 1.0, 2.0),
                c + v2(sx * 11.0, 10.0),
                c + v2(sx * 16.0, 19.0),
                c + v2(sx * 10.0, 17.0),
                c + v2(sx * 5.0, 20.0),
                c + v2(sx * 1.0, 8.0),
            ])
        })
        .collect();
    let knot = ellipse(c + v2(0.0, 0.5), 6.5, 7.5, 0.0);
    for p in tails.iter().chain(loops.iter()) {
        r.halo(d, p);
    }
    r.halo(d, &knot);
    for t in &tails {
        r.part(d, t, RIBBON);
    }
    for l in &loops {
        r.part_with(d, l, |d, p| {
            RIBBON.ink(d, p);
            if !r.small() {
                d.clipped(p, |d| {
                    for (dx, dy) in [(-17.0, -6.0), (-9.0, 1.0), (11.0, -8.0), (19.0, 0.0)] {
                        d.knock_p(1.0, Screen::Solid, PLATES_COLOR, &circle(c + v2(dx, dy), 2.1));
                    }
                });
            }
        });
    }
    r.part(d, &knot, Coat::new(0.0, 0.08, 0.72, 0.0));
    for sx in [-1.0f32, 1.0] {
        r.detail_line(d, &bow(c + v2(sx * 8.0, -1.0), c + v2(sx * 17.0, -5.0), sx * -2.0, 4));
    }
}
