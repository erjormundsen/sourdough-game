//! Secondary goods: discard muffins, cinnamon buns and bagels.

use super::{LINE, shadow};
use crate::content::Treat;
use crate::draw::{DrawList, Paint};
use crate::geom::{V2, chaikin, circle, ellipse, scallop, v2};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::TAU;

/// Draw one treat centred at the origin, roughly `s` wide. `seed` varies toppings.
pub fn treat(d: &mut DrawList, t: Treat, s: f32, seed: u32) {
    shadow(d, v2(0.0, s * 0.38), s * 0.5, s * 0.09);
    match t {
        Treat::Muffin => muffin(d, s, seed),
        Treat::CinnamonBun => bun(d, s, seed),
        Treat::Bagel => bagel(d, s, seed),
    }
}

fn muffin(d: &mut DrawList, s: f32, seed: u32) {
    let cup = vec![
        v2(-s * 0.36, -s * 0.05),
        v2(s * 0.36, -s * 0.05),
        v2(s * 0.28, s * 0.38),
        v2(-s * 0.28, s * 0.38),
    ];
    d.fill(Ink::Pink, 0.55, &cup);
    for i in 0..7 {
        let x = -s * 0.3 + i as f32 * s * 0.1;
        d.line(Ink::Key, 1.8, &[v2(x, -s * 0.03), v2(x * 0.8, s * 0.36)]);
    }
    d.outline(Ink::Key, LINE - 1.0, &cup);
    let top = chaikin(
        &[
            v2(-s * 0.44, 0.0),
            v2(-s * 0.4, -s * 0.3),
            v2(0.0, -s * 0.44),
            v2(s * 0.4, -s * 0.3),
            v2(s * 0.44, 0.0),
        ],
        3,
        true,
    );
    d.fill(Ink::Yellow, 0.9, &top);
    d.ht(Ink::Pink, 0.4, &top);
    for i in 0..9 {
        let p = v2(
            (hash01(seed, i) - 0.5) * s * 0.6,
            -s * 0.05 - hash01(seed, i + 20) * s * 0.3,
        );
        d.fill(Ink::Blue, 0.9, &circle(p, s * 0.035));
        d.fill(Ink::Pink, 0.6, &circle(p, s * 0.035));
    }
    d.outline(Ink::Key, LINE - 1.0, &top);
}

fn bun(d: &mut DrawList, s: f32, seed: u32) {
    let body = crate::geom::blob(V2::ZERO, s * 0.46, s * 0.4, 0.04, seed);
    d.fill(Ink::Yellow, 0.9, &body);
    d.ht(Ink::Pink, 0.45, &body);
    let spiral: Vec<V2> = (0..90)
        .map(|i| {
            let t = i as f32 / 90.0;
            let a = t * TAU * 2.6;
            V2::from_angle(a) * (s * 0.4 * t) + v2(0.0, 0.0)
        })
        .map(|p| v2(p.x, p.y * 0.88))
        .collect();
    d.stroke_p(Paint::ht(Ink::Key, 0.55), s * 0.05, &spiral, false);
    d.line(Ink::Key, 2.0, &spiral);
    // Icing drizzle: a loose zig-zag of paper-white.
    let mut drizzle = Vec::new();
    for i in 0..7 {
        let x = -s * 0.32 + i as f32 * s * 0.105;
        let y = if i % 2 == 0 { -s * 0.26 } else { s * 0.2 };
        drizzle.push(v2(x + (hash01(seed, i) - 0.5) * s * 0.05, y));
    }
    let drizzle = chaikin(&drizzle, 3, false);
    d.knock_line(1.0, s * 0.05, &drizzle, false);
    d.stroke_p(Paint::solid(Ink::Key, 0.35), 1.2, &drizzle, false);
    d.outline(Ink::Key, LINE - 1.0, &body);
}

fn bagel(d: &mut DrawList, s: f32, seed: u32) {
    let outer = crate::geom::blob(V2::ZERO, s * 0.46, s * 0.4, 0.03, seed);
    let hole = ellipse(v2(0.0, -s * 0.02), s * 0.11, s * 0.08, 0.0);
    d.fill_eo(
        Paint::solid(Ink::Yellow, 0.95),
        &[outer.clone(), hole.clone()],
    );
    d.fill_eo(Paint::ht(Ink::Pink, 0.55), &[outer.clone(), hole.clone()]);
    d.fill_eo(Paint::ht(Ink::Key, 0.12), &[outer.clone(), hole.clone()]);
    for i in 0..26 {
        let a = hash01(seed, i) * TAU;
        let r = s * (0.18 + 0.2 * hash01(seed, i + 40));
        let p = V2::from_angle(a) * r;
        let seedp = ellipse(v2(p.x, p.y * 0.87), s * 0.022, s * 0.013, a);
        d.knock(&seedp);
        d.stroke_p(Paint::solid(Ink::Key, 0.8), 1.0, &seedp, true);
    }
    d.outline(Ink::Key, LINE - 1.0, &outer);
    d.outline(Ink::Key, LINE - 1.5, &hole);
    let _ = scallop;
}
