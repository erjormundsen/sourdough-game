//! Hazel the hedgehog: a kindly baker's friend in a gingham apron, a daisy tucked in her
//! spines and a sprig of wheat in her pocket. Her prickles are for show.

use super::kit::*;
use crate::draw::{DrawList, Paint};
use crate::geom::{V2, circle, ellipse, quad_bezier, rect, rounded_rect, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const SPINE_BACK: Coat = Coat::new(0.56, 0.56, 0.52, 0.0);
const SPINE_FRONT: Coat = Coat::new(0.72, 0.48, 0.3, 0.0);
const SPINE_TIP: Coat = Coat::new(0.34, 0.16, 0.05, 0.0);
const FACE: Coat = Coat::new(0.3, 0.1, 0.0, 0.0);
const BODY: Coat = Coat::new(0.62, 0.42, 0.2, 0.0);
const PAW: Coat = Coat::new(0.4, 0.34, 0.04, 0.0);
const EAR_IN: Coat = Coat::new(0.15, 0.45, 0.0, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let sc = h + v2(0.0, 2.0);
    let k = if r.portrait { 0.84 } else { 1.0 };
    // Two layers of quills: a darker back crown and a lighter, offset front crown.
    let (back, back_tips) = quill_crown(sc, 70.0, 62.0, -PI * 1.1, PI * 0.1, 13, 32.0 * k, 11);
    let (front, front_tips) =
        quill_crown(sc + v2(0.0, 4.0), 64.0, 56.0, -PI * 1.05 + 0.13, PI * 0.05 + 0.13, 11, 26.0 * k, 23);
    let face = blobby(
        &[
            v2(0.0, -38.0),
            v2(38.0, -33.0),
            v2(58.0, -6.0),
            v2(52.0, 24.0),
            v2(30.0, 46.0),
            v2(0.0, 56.0),
            v2(-30.0, 46.0),
            v2(-52.0, 24.0),
            v2(-58.0, -6.0),
            v2(-38.0, -33.0),
        ]
        .map(|p| h + v2(0.0, 12.0) + p),
    );
    // Hairline: the quills come down over the brow in a little serrated fringe.
    let fringe = {
        let mut p = vec![h + v2(-50.0, -14.0)];
        p.extend(
            quad_bezier(h + v2(-50.0, -14.0), h + v2(0.0, -64.0), h + v2(50.0, -14.0), 12)
                .into_iter()
                .skip(1),
        );
        let n = 5;
        for i in 0..=2 * n {
            let t = i as f32 / (2 * n) as f32;
            let x = 50.0 - 100.0 * t;
            let base_y = -16.0 - 12.0 * (1.0 - (x / 50.0).powi(2));
            // Teeth lean outwards from the parting, like brushed-down quills.
            let y = if i % 2 == 1 { base_y + 15.0 } else { base_y };
            let lean = if i % 2 == 1 { x.signum() * 3.0 } else { 0.0 };
            p.push(h + v2(x + lean, y));
        }
        p
    };
    let ears: Vec<(Vec<V2>, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = h + v2(sx * 57.0, -16.0);
            let tw = if sx < 0.0 { r.pose.twitch * 0.25 } else { 0.0 };
            let pivot = c + v2(-sx * 8.0, 8.0);
            (rot(&circle(c, 13.0), pivot, tw), rot(&circle(c + v2(-sx * 1.5, 1.5), 7.0), pivot, tw))
        })
        .collect();

    let torso = r.breathe(&mound(&[
        v2(0.0, -126.0),
        v2(34.0, -122.0),
        v2(58.0, -112.0),
        v2(74.0, -106.0),
        v2(90.0, -88.0),
        v2(96.0, -56.0),
        v2(98.0, 0.0),
    ]));
    // Her quilled back rises over the shoulders.
    let (back_quills, _) = quill_crown(v2(0.0, -86.0), 86.0, 34.0, -PI, 0.0, 9, 17.0, 5);
    let back_quills = r.breathe(&back_quills);
    let arms: Vec<(Vec<V2>, Vec<V2>)> =
        [-1.0f32, 1.0].iter().map(|&sx| tube_parts(&r.arm_path(sx, 90.0, 44.0), 18.0, 14.5)).collect();
    let paws: Vec<(V2, f32, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let (c, tilt) = r.paw_place(sx, 41.0, 90.0, 4.0);
            (c, tilt, rot(&paw_shape(c, 33.0, 26.0), c, tilt))
        })
        .collect();

    if r.body {
        // A calm, rounded shadow plate: tracing every quill would fuzz the silhouette.
        let crown_shadow = ellipse(sc + v2(0.0, -8.0), 92.0, 84.0, 0.0);
        let sil: Vec<&[V2]> = vec![&torso, &crown_shadow];
        r.wall_shadow(d, &sil);
        r.halo(d, &back_quills);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for (_, _, p) in &paws {
            r.halo(d, p);
        }
    }
    r.halo(d, &back);

    if r.body {
        r.part(d, &back_quills, SPINE_BACK);
        r.part(d, &torso, BODY);
        apron(r, d);
        neck_shadow(r, d, &torso, 72.0, 64.0, 10.0);
    }
    let limbs = |d: &mut DrawList| {
        for (poly, open) in &arms {
            d.backing(poly);
            BODY.ink(d, poly);
            d.line(Ink::Key, r.inner(), open);
        }
        for (i, (c, tilt, p)) in paws.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part(d, p, PAW);
            toes_at(r, d, *c, 33.0, 26.0, 3, *tilt, r.raised(sx));
            r.wave_marks(d, *c + v2(0.0, -26.0), sx);
        }
    };
    if r.body && !r.limbs_in_front() {
        limbs(d);
    }

    // Quills: dark back crown with pale tips, lighter front crown, a few quill strokes.
    r.part_with(d, &back, |d, p| {
        SPINE_BACK.ink(d, p);
        pale_tips(d, p, &back_tips, sc, 13.0);
    });
    r.part_with(d, &front, |d, p| {
        SPINE_FRONT.ink(d, p);
        pale_tips(d, p, &front_tips, sc, 10.0);
        r.rim_shade(d, p, v2(0.8, 1.0), 8.0, Ink::Blue, 0.22);
    });
    if !r.small() {
        for (i, t) in front_tips.iter().enumerate() {
            if i % 2 == 0 {
                let base = sc + (*t - sc) * 0.62;
                r.detail_line(d, &[base, sc + (*t - sc) * 0.86]);
            }
        }
    }
    for (e, inner) in &ears {
        r.part(d, e, FACE);
        EAR_IN.ink_clean(d, inner);
    }
    r.part(d, &face, FACE);
    r.part_with(d, &fringe, |d, p| {
        SPINE_FRONT.ink(d, p);
        r.rim_shade(d, p, v2(0.0, -1.0), 7.0, Ink::Blue, 0.2);
    });

    // Snout, nose and the shared face.
    let nose_c = h + v2(0.0, 36.0);
    let nose = ellipse(nose_c, 9.5, 7.5, 0.0);
    d.fill(Ink::Key, 1.0, &nose);
    d.knock(&ellipse(nose_c + v2(-3.5, -2.5), 3.0, 2.0, -0.3));
    r.face(d, h + v2(0.0, 13.0), 96.0, V2::ZERO, true, None);
    crate::art::face::mouth(d, h + v2(0.0, 48.0), 76.0, r.expr);

    daisy(r, d, h + v2(-50.0, -46.0));
    if r.limbs_in_front() {
        limbs(d);
    }
}

/// Pale banded tips on each quill (clipped to the crown).
fn pale_tips(d: &mut DrawList, crown: &[V2], tips: &[V2], c: V2, size: f32) {
    d.clipped(crown, |d| {
        for t in tips {
            let dir = (*t - c).norm();
            SPINE_TIP.ink(d, &ellipse(*t - dir * (size * 0.2), size * 0.55, size, dir.angle() + PI * 0.5));
        }
    });
}

fn apron(r: &Rig, d: &mut DrawList) {
    let bib = r.breathe(&blobby(&[
        v2(0.0, -104.0),
        v2(34.0, -103.0),
        v2(40.0, -80.0),
        v2(48.0, -40.0),
        v2(52.0, 6.0),
        v2(-52.0, 6.0),
        v2(-48.0, -40.0),
        v2(-40.0, -80.0),
        v2(-34.0, -103.0),
    ]));
    // Straps over the shoulders.
    for sx in [-1.0f32, 1.0] {
        let strap = r.breathe(&[
            v2(sx * 28.0, -100.0),
            v2(sx * 44.0, -120.0),
            v2(sx * 58.0, -116.0),
            v2(sx * 40.0, -97.0),
        ]);
        r.part(d, &strap, Coat::new(0.0, 0.5, 0.0, 0.0));
    }
    r.part_with(d, &bib, |d, p| {
        d.clipped(p, |d| {
            if r.small() {
                d.fill(Ink::Pink, 0.28, p);
                return;
            }
            for axis in [0.0, PI * 0.5] {
                for b in bands(p, axis, 24.0, 12.0, 0.0) {
                    d.fill_p(Paint::solid(Ink::Pink, 0.28).add(), &b);
                }
            }
        });
        r.rim_shade(d, p, v2(1.0, 0.2), 8.0, Ink::Blue, 0.18);
    });
    r.stitches(d, &inset(&bib, 0.9, true), 3.0, 3.0);
    // Pocket with a sprig of wheat.
    let pc = r.breathe(&[v2(-2.0, -62.0)])[0];
    wheat(r, d, pc + v2(6.0, -8.0));
    let pocket = rounded_rect(rect(pc.x - 19.0, pc.y - 11.0, 38.0, 26.0), 6.0);
    r.part(d, &pocket, Coat::new(0.08, 0.45, 0.0, 0.0));
    r.stitches(d, &[pc + v2(-15.0, -5.0), pc + v2(15.0, -5.0)], 2.5, 2.5);
}

fn wheat(r: &Rig, d: &mut DrawList, c: V2) {
    let base = c + v2(0.0, 8.0);
    let top = c + v2(9.0, -30.0);
    d.line(Ink::Key, r.detail() * 1.7, &[base, top]);
    d.line(Ink::Yellow, r.detail() * 0.7, &[base, top]);
    for i in 0..3 {
        let p = base.lerp(top, 0.5 + i as f32 * 0.17);
        for sx in [-1.0f32, 1.0] {
            let g = ellipse(p + v2(sx * 3.2, 0.0), 2.4, 4.4, sx * 0.55);
            d.backing(&g);
            d.fill(Ink::Yellow, 0.9, &g);
            d.fill(Ink::Pink, 0.22, &g);
            d.outline(Ink::Key, r.detail() * 0.7, &g);
        }
    }
    let g = ellipse(top + v2(0.4, -3.0), 2.4, 4.6, 0.2);
    d.backing(&g);
    d.fill(Ink::Yellow, 0.9, &g);
    d.outline(Ink::Key, r.detail() * 0.7, &g);
    if !r.small() {
        for sx in [-1.0f32, 1.0] {
            r.detail_line(d, &[top + v2(0.0, -6.0), top + v2(sx * 5.0, -14.0)]);
        }
    }
}

fn daisy(r: &Rig, d: &mut DrawList, c: V2) {
    let petals: Vec<Vec<V2>> = (0..8)
        .map(|i| {
            let a = i as f32 / 8.0 * PI * 2.0 + 0.2;
            ellipse(c + V2::from_angle(a) * 11.0, 9.0, 5.0, a)
        })
        .collect();
    for p in &petals {
        r.halo(d, p);
    }
    for p in &petals {
        d.backing(p);
        d.outline(Ink::Key, r.detail(), p);
    }
    let mid = circle(c, 7.5);
    d.backing(&mid);
    d.fill(Ink::Yellow, 1.0, &mid);
    d.fill(Ink::Pink, 0.25, &mid);
    r.tex(d, &mid, Ink::Pink, 0.4);
    d.outline(Ink::Key, r.detail(), &mid);
}
