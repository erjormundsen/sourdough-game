//! Pip the duckling: puddle-hopper in a sou'wester and a toggle raincoat. Loves sour.

use super::kit::*;
use crate::art::Expr;
use crate::draw::{DrawList, PLATES_COLOR, Screen};
use crate::geom::{V2, capsule, circle, ellipse, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

const DOWN: Coat = Coat::new(0.95, 0.06, 0.0, 0.0);
const BILL: Coat = Coat::new(0.95, 0.52, 0.0, 0.0);
const OILSKIN: Coat = Coat::new(0.0, 0.0, 0.62, 0.04);
const TOGGLE: Coat = Coat::new(0.9, 0.16, 0.0, 0.0);

pub fn draw(d: &mut DrawList, r: &Rig) {
    let h = r.h;
    let head = furry_oval(h + v2(0.0, 2.0), 72.0, 67.0, &[(0.5, 0.3, 5.0, 2), (PI - 0.5, 0.3, 5.0, 2)]);
    // Portraits sit the hat a touch lower so it fits the frame.
    let (hx, hk) = if r.portrait { (0.9, 0.9) } else { (1.0, 1.0) };
    let hat = |p: V2| h + v2(p.x * hx, -36.0 + (p.y + 36.0) * hk);
    let crown = blobby(
        &[
            v2(-54.0, -38.0),
            v2(-50.0, -72.0),
            v2(-28.0, -94.0),
            v2(0.0, -99.0),
            v2(28.0, -94.0),
            v2(50.0, -72.0),
            v2(54.0, -38.0),
            v2(0.0, -34.0),
        ]
        .map(hat),
    );
    let brim = blobby(
        &[
            v2(-97.0, -18.0),
            v2(-86.0, -40.0),
            v2(-56.0, -55.0),
            v2(0.0, -60.0),
            v2(56.0, -55.0),
            v2(86.0, -40.0),
            v2(97.0, -18.0),
            v2(80.0, -15.0),
            v2(48.0, -30.0),
            v2(0.0, -38.0),
            v2(-48.0, -30.0),
            v2(-80.0, -15.0),
        ]
        .map(hat),
    );

    let torso = r.breathe(&mound(&[
        v2(0.0, -126.0),
        v2(28.0, -122.0),
        v2(48.0, -113.0),
        v2(64.0, -108.0),
        v2(80.0, -90.0),
        v2(86.0, -60.0),
        v2(90.0, 0.0),
    ]));
    let paths: Vec<Vec<V2>> = [-1.0f32, 1.0].iter().map(|&sx| r.arm_path(sx, 84.0, 44.0)).collect();
    let arms: Vec<(Vec<V2>, Vec<V2>)> = paths.iter().map(|p| tube_parts(p, 17.0, 14.0)).collect();
    let wings: Vec<(V2, Vec<V2>)> = [-1.0f32, 1.0]
        .iter()
        .map(|&sx| {
            let c = v2(sx * 39.0, REST_Y + 4.0);
            (c, wing_tip(c, sx))
        })
        .collect();

    if r.body {
        let mut sil: Vec<&[V2]> = vec![&torso, &head, &crown, &brim];
        sil.extend(arms.iter().map(|a| a.0.as_slice()));
        r.wall_shadow(d, &sil);
        r.halo(d, &torso);
        for (a, _) in &arms {
            r.halo(d, a);
        }
        for (_, w) in &wings {
            r.halo(d, w);
        }
    }
    r.halo(d, &head);
    r.halo(d, &crown);
    r.halo(d, &brim);

    if r.body {
        r.part_with(d, &torso, |d, p| {
            OILSKIN.ink(d, p);
            r.rim_shade(d, p, v2(1.0, 0.3), 12.0, Ink::Key, 0.2);
        });
        neck_shadow(r, d, &torso, 72.0, 67.0, 10.0);
        // Big rounded raincoat collar.
        for sx in [-1.0f32, 1.0] {
            let c = r.breathe(&blobby(&[
                v2(sx * 2.0, -120.0),
                v2(sx * 36.0, -118.0),
                v2(sx * 56.0, -105.0),
                v2(sx * 52.0, -89.0),
                v2(sx * 29.0, -85.0),
                v2(sx * 6.0, -96.0),
            ]));
            r.part_with(d, &c, |d, p| {
                Coat::new(0.0, 0.0, 0.5, 0.0).ink(d, p);
                r.rim_shade(d, p, v2(0.3, 1.0), 5.0, Ink::Key, 0.18);
            });
            r.stitches(d, &inset(&c, 0.8, true), 3.0, 3.0);
        }
        // Front seam and rope toggles.
        let top = r.breathe(&[v2(0.0, -96.0)])[0];
        r.seam(d, &[top, v2(0.0, 4.0)]);
        for y in [-72.0, -48.0] {
            let c = r.breathe(&[v2(0.0, y)])[0];
            r.detail_line(d, &bow(c + v2(-13.0, 0.0), c + v2(0.0, 0.0), 5.0, 6));
            let t = capsule(c + v2(-1.0, 0.0), c + v2(12.0, 0.0), 4.2);
            d.backing(&t);
            TOGGLE.ink(d, &t);
            d.outline(Ink::Key, r.detail(), &t);
        }
        // Raindrops beading on the oilskin.
        if !r.small() {
            for (x, y, s) in [(-52.0, -62.0, 1.0), (24.0, -44.0, 0.8), (56.0, -76.0, 0.9)] {
                drop(r, d, r.breathe(&[v2(x, y)])[0], 5.0 * s);
            }
        }
        for (i, (poly, open)) in arms.iter().enumerate() {
            d.backing(poly);
            OILSKIN.ink(d, poly);
            r.rim_shade(d, poly, v2(1.0, 0.5), 8.0, Ink::Key, 0.2);
            d.line(Ink::Key, r.inner(), open);
            elbow_creases(r, d, poly, &paths[i], 15.0, false);
        }
        for (i, (c, w)) in wings.iter().enumerate() {
            let sx = if i == 0 { -1.0 } else { 1.0 };
            r.part(d, w, DOWN);
            if !r.small() {
                // Feather separations running back from the fingers.
                for (y, len) in [(-15.0f32, 14.0f32), (-6.0, 12.0)] {
                    let a = *c + v2(-sx * 12.0, y);
                    r.detail_line(d, &bow(a, a + v2(sx * len, y * 0.15 - 3.0), sx * 1.5, 4));
                }
            }
        }
    }

    // Head, feather bangs, then the hat.
    r.part(d, &head, DOWN);
    for (a, b) in [(v2(-6.0, -40.0), v2(-16.0, -25.0)), (v2(4.0, -40.0), v2(10.0, -24.0))] {
        let f = spike(h + a, h + b, 11.0, 2.0);
        r.part(d, &f, DOWN);
    }
    r.part_with(d, &crown, |d, p| {
        OILSKIN.ink(d, p);
        d.clipped(p, |d| {
            let band =
                crate::geom::rect_poly(crate::geom::rect(h.x - 80.0, hat(v2(0.0, -56.0)).y, 160.0, 14.0));
            d.fill(Ink::Blue, 0.85, &band);
            d.fill(Ink::Key, 0.18, &band);
        });
        r.rim_shade(d, p, v2(1.0, 0.2), 10.0, Ink::Key, 0.18);
    });
    r.detail_line(d, &bow(hat(v2(0.0, -98.0)), hat(v2(2.0, -58.0)), -3.0, 6));
    r.seam(d, &bow(hat(v2(-53.0, -56.0)), hat(v2(53.0, -56.0)), 4.0, 12));
    if !r.small() {
        d.knock_p(
            0.7,
            Screen::Solid,
            PLATES_COLOR,
            &capsule(hat(v2(-34.0, -82.0)), hat(v2(-22.0, -90.0)), 4.0),
        );
    }
    r.part_with(d, &brim, |d, p| {
        OILSKIN.ink(d, p);
        r.rim_shade(d, p, v2(0.0, 1.0), 6.0, Ink::Key, 0.22);
    });
    r.stitches(d, &inset(&brim, 0.86, true), 3.5, 3.0);
    if !r.small() {
        drop(r, d, hat(v2(66.0, -38.0)), 4.2);
    }

    // Face: eyes and blush from the grammar, then the bill (which carries the mouth).
    r.clear_cheeks(d, h + v2(0.0, -2.0), 100.0, 0.75);
    r.face(d, h + v2(0.0, -2.0), 100.0, V2::ZERO, true, None);
    bill(r, d, h + v2(0.0, 6.0));
}

/// The bill: upper and lower mandibles that part with the expression.
fn bill(r: &Rig, d: &mut DrawList, o: V2) {
    let gape = match r.expr {
        Expr::Excited => 13.0,
        Expr::Happy | Expr::Wow => 9.0,
        Expr::Proud => 6.0,
        Expr::Hungry => 4.0,
        _ => 0.0,
    };
    let tilt = if r.expr == Expr::Hmm { 0.12 } else { 0.0 };
    let upper = rot(
        &blobby(
            &[
                v2(-31.0, 14.0),
                v2(-18.0, 5.0),
                v2(0.0, 3.0),
                v2(18.0, 5.0),
                v2(31.0, 14.0),
                v2(24.0, 22.0),
                v2(0.0, 24.0),
                v2(-24.0, 22.0),
            ]
            .map(|p| o + p),
        ),
        o + v2(0.0, 16.0),
        tilt,
    );
    let lower = blobby(
        &[v2(-24.0, 20.0), v2(0.0, 22.0), v2(24.0, 20.0), v2(19.0, 30.0), v2(0.0, 34.0), v2(-19.0, 30.0)]
            .map(|p| o + p + v2(0.0, gape)),
    );
    if gape > 0.0 {
        let cav = ellipse(o + v2(0.0, 22.0 + gape * 0.5), 21.0, 3.5 + gape * 0.5, 0.0);
        d.backing(&cav);
        d.fill(Ink::Key, 1.0, &cav);
        if gape > 5.0 {
            let tongue = ellipse(o + v2(0.0, 24.0 + gape), 11.0, gape * 0.45, 0.0);
            d.clipped(&cav, |d| {
                d.knock(&tongue);
                d.fill(Ink::Pink, 0.9, &tongue);
            });
        }
        d.outline(Ink::Key, r.inner(), &cav);
    }
    r.part(d, &lower, BILL);
    r.part_with(d, &upper, |d, p| {
        BILL.ink(d, p);
        if !r.small() {
            d.knock_p(0.6, Screen::Solid, PLATES_COLOR, &capsule(o + v2(-16.0, 9.0), o + v2(-6.0, 7.0), 2.6));
        }
    });
    for sx in [-1.0f32, 1.0] {
        d.fill(Ink::Key, 0.9, &ellipse(o + v2(sx * 7.0, 11.0), 2.2, 1.6, 0.0));
    }
    if gape == 0.0 {
        // Closed: the smile line between the mandibles.
        let line = rot(&bow(o + v2(-24.0, 20.0), o + v2(24.0, 20.0), -4.0, 10), o + v2(0.0, 16.0), tilt);
        r.seam(d, &line);
    }
    if r.expr == Expr::Hungry && !r.small() {
        // A hopeful drip at the corner of the bill.
        drop(r, d, o + v2(21.0, 36.0), 3.6);
    }
}

/// A duckling wing tip resting on the counter: rounded at the sleeve, ending in three
/// feather fingers that point inwards.
fn wing_tip(c: V2, sx: f32) -> Vec<V2> {
    let body = [c + v2(sx * 15.0, -26.0), c + v2(sx * 0.0, -27.0), c + v2(-sx * 10.0, -22.0)];
    // Three feather tips along the inner end, stepping down to the counter.
    let tips = [
        c + v2(-sx * 20.0, -19.0),
        c + v2(-sx * 13.0, -13.0),
        c + v2(-sx * 22.0, -8.0),
        c + v2(-sx * 14.0, -3.0),
        c + v2(-sx * 20.0, 3.0),
    ];
    let mut pts: Vec<V2> = body.to_vec();
    pts.extend(tips);
    pts.extend([c + v2(sx * 6.0, 4.0), c + v2(sx * 19.0, -8.0)]);
    blobby(&pts)
}

/// A bead of rain: a paper teardrop with a key rim.
fn drop(r: &Rig, d: &mut DrawList, c: V2, s: f32) {
    let mut p = vec![c + v2(0.0, -s * 1.6)];
    p.extend(crate::geom::arc(c, s, -PI * 0.15, PI * 1.15));
    d.knock_p(1.0, Screen::Solid, PLATES_COLOR, &p);
    d.fill(Ink::Blue, 0.22, &p);
    d.outline(Ink::Key, r.detail() * 0.8, &p);
    d.knock_p(1.0, Screen::Solid, PLATES_COLOR, &circle(c + v2(-s * 0.3, -s * 0.2), s * 0.28));
}
