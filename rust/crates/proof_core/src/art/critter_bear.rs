//! Bruno the bear: big, cosy and flannel-shirted. The crustier, the better.

use super::kit::*;
use crate::draw::{DrawList, Paint};
use crate::geom::{V2, circle, ellipse, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const FUR: Coat = Coat::new(0.82, 0.55, 0.22, 0.0);
const EAR_IN: Coat = Coat::new(0.34, 0.4, 0.0, 0.0);
const FLANNEL: Coat = Coat::new(0.14, 0.04, 0.0, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let head =
        furry_oval(h, 80.0, 71.0, &[(-PI * 0.5, 0.3, 7.0, 2), (0.42, 0.4, 7.5, 2), (PI - 0.42, 0.4, 7.5, 2)]);
    let ears: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = h + v2(sx * 57.0, -52.0);
            let tw = if sx > 0.0 { r.pose.twitch * 0.18 } else { 0.0 };
            let pivot = h + v2(sx * 44.0, -38.0);
            (rot(&circle(c, 25.0), pivot, tw), rot(&circle(c + v2(-sx * 3.0, 3.0), 13.5), pivot, tw))
        })
        .collect();

    let torso = r.breathe(&mound(&[
        v2(0.0, -134.0),
        v2(34.0, -128.0),
        v2(58.0, -118.0),
        v2(78.0, -114.0),
        v2(96.0, -96.0),
        v2(100.0, -60.0),
        v2(102.0, 0.0),
    ]));
    let paths: Vec<Vec<V2>> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            r.breathe(&spline(
                &[v2(sx * 80.0, -92.0), v2(sx * 91.0, -58.0), v2(sx * 78.0, -36.0), v2(sx * 54.0, -38.0)],
                false,
                6,
            ))
        })
        .collect();
    let arms: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(p, 23.0, 17.5)).collect();
    let paws: Vec<(V2, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = v2(sx * 42.0, REST_Y + 5.0);
            (c, paw_shape(c, 42.0, 33.0))
        })
        .collect();

    // Shadow plate and halos.
    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head];
        sil.extend(arms.iter().map(|a| a.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for (_, p) in &paws {
            r.halo(d, p);
        }
    }
    for (e, _) in &ears {
        r.halo(d, e);
    }
    r.halo(d, &head);

    if r.body {
        r.part_with(d, &torso, |d, p| {
            FLANNEL.ink(d, p);
            plaid(r, d, p, V2::ZERO, 0.0);
        });
        // Collar points peeking out under the chin.
        for sx in [-1.0f32, 1.0] {
            let c = r.breathe(&[
                v2(sx * 2.0, -124.0),
                v2(sx * 44.0, -122.0),
                v2(sx * 35.0, -83.0),
                v2(sx * 4.0, -99.0),
            ]);
            r.part_with(d, &c, |d, p| {
                FLANNEL.ink(d, p);
                plaid(r, d, p, v2(sx * 9.0, 4.0), sx * 2.5);
            });
        }
        neck_shadow(r, d, &torso, 80.0, 71.0, 10.0);
        // Placket: a seam, a stitch line and two buttons.
        let top = r.breathe(&[v2(0.0, -100.0)])[0];
        r.seam(d, &[top + v2(-8.0, 0.0), v2(-8.0, 4.0)]);
        r.stitches(d, &[top + v2(8.0, 2.0), v2(8.0, 4.0)], 4.0, 3.5);
        for y in [-78.0, -52.0] {
            let b = r.breathe(&[v2(0.0, y)])[0];
            button(r, d, b, 5.5, Coat::new(0.16, 0.06, 0.0, 0.0));
        }
        // Sleeves with rolled cuffs, then paws.
        for (i, (poly, open)) in arms.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            d.backing(poly);
            FLANNEL.ink(d, poly);
            plaid(r, d, poly, v2(sx * 15.0, 11.0), sx);
            r.rim_shade(d, poly, v2(1.0, 0.6), 9.0, Ink::Blue, 0.3);
            d.line(Ink::Key, r.inner(), open);
            elbow_creases(r, d, poly, &paths[i], 20.0, false);
        }
        for (c, p) in &paws {
            r.part_with(d, p, |d, p| {
                FUR.ink(d, p);
                r.rim_shade(d, p, v2(0.7, 1.0), 6.0, Ink::Blue, 0.26);
            });
            toes(r, d, *c, 42.0, 33.0, 3);
        }
    }

    // Ears behind, then the head.
    for (e, inner) in &ears {
        r.part(d, e, FUR);
        EAR_IN.ink(d, inner);
    }
    r.part(d, &head, FUR);

    // Muzzle, nose, face.
    let muzzle = ellipse(h + v2(0.0, 27.0), 33.0, 25.0, 0.0);
    r.part(d, &muzzle, Coat::CREAM);
    let nose_c = h + v2(0.0, 15.0);
    let nose = blobby(&[
        nose_c + v2(-12.0, -6.0),
        nose_c + v2(0.0, -7.5),
        nose_c + v2(12.0, -6.0),
        nose_c + v2(5.0, 5.0),
        nose_c + v2(0.0, 7.0),
        nose_c + v2(-5.0, 5.0),
    ]);
    d.fill(Ink::Key, 1.0, &nose);
    d.knock(&ellipse(nose_c + v2(-4.0, -3.0), 3.8, 2.2, -0.2));
    let m = h + v2(0.0, 34.0);
    r.detail_line(d, &[nose_c + v2(0.0, 6.0), m + v2(0.0, -2.0)]);
    r.clear_cheeks(d, h + v2(0.0, 1.0), 104.0, 0.45);
    r.face(d, h + v2(0.0, 1.0), 104.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, m, 88.0, r.expr);
}

/// Blue flannel: soft bands that overprint where they cross, pink pinstripes.
fn plaid(r: &Rig, d: &mut DrawList, clip: &[V2], off: V2, tilt: f32) {
    let small = r.small();
    let a = tilt * 0.12;
    let pitch = 30.0;
    let phase = off.x;
    d.clipped(clip, |d| {
        for axis in [a, a + PI * 0.5] {
            for b in bands(clip, axis, pitch, 16.0, phase) {
                d.fill_p(Paint::solid(Ink::Blue, 0.34).add(), &b);
            }
        }
        if !small {
            for axis in [a, a + PI * 0.5] {
                for b in bands(clip, axis, pitch, 2.6, phase + pitch * 0.5) {
                    d.fill_p(Paint::solid(Ink::Pink, 0.7).add(), &b);
                }
            }
        }
    });
}
