//! Full-screen backdrops (720×1280 reference canvas).

use super::LINE;
use crate::draw::DrawList;
use crate::geom::{V2, arc, circle, rect, rect_poly, rounded_rect, scallop, soft_star, v2};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::PI;

pub const W: f32 = 720.0;
pub const H: f32 = 1280.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backdrop {
    /// Morning: kitchen wall with a sunrise window.
    Bakehouse,
    /// Shop: striped awning window over the counter.
    Shopfront,
    /// Evening: lamp-lit shelf wall with a moon window.
    Pantry,
}

fn wall(d: &mut DrawList, ink: Ink, tone: f32, stripe: Option<Ink>) {
    let all = rect_poly(rect(-20.0, -20.0, W + 40.0, H + 40.0));
    d.fill(ink, tone, &all);
    if let Some(s) = stripe {
        for i in 0..12 {
            let x = i as f32 * 64.0 + 16.0;
            d.ht_add(s, 0.14, &rect_poly(rect(x, -20.0, 30.0, H + 40.0)));
        }
    }
}

fn window(d: &mut DrawList, r: crate::geom::Rect, sky: impl FnOnce(&mut DrawList, &[V2])) {
    let mut outline = vec![v2(r.x, r.y + r.h)];
    outline.extend(arc(
        v2(r.x + r.w * 0.5, r.y + r.w * 0.5),
        r.w * 0.5,
        PI,
        PI * 2.0,
    ));
    outline.push(v2(r.x + r.w, r.y + r.h));
    d.knock(&outline);
    d.clipped(&outline.clone(), |d| sky(d, &outline));
    let cx = r.x + r.w * 0.5;
    d.line(Ink::Key, 7.0, &[v2(cx, r.y), v2(cx, r.y + r.h)]);
    d.line(
        Ink::Key,
        7.0,
        &[v2(r.x, r.y + r.h * 0.62), v2(r.x + r.w, r.y + r.h * 0.62)],
    );
    d.outline(Ink::Key, 9.0, &outline);
    let sill = rounded_rect(rect(r.x - 26.0, r.y + r.h - 4.0, r.w + 52.0, 24.0), 8.0);
    d.backing(&sill);
    d.fill(Ink::Yellow, 0.7, &sill);
    d.ht(Ink::Pink, 0.35, &sill);
    d.outline(Ink::Key, LINE, &sill);
}

fn cloud(d: &mut DrawList, c: V2, s: f32) {
    let p = scallop(c, s, 7, 0.22);
    d.knock(&p);
    d.outline(Ink::Key, 3.5, &p);
}

fn counter(d: &mut DrawList, top: f32, front: Ink) {
    let slab = rounded_rect(rect(-30.0, top, W + 60.0, 44.0), 14.0);
    let body = rect_poly(rect(-30.0, top + 30.0, W + 60.0, H - top));
    d.backing(&body);
    d.fill(front, 0.55, &body);
    for i in 0..5 {
        let x = 30.0 + i as f32 * 140.0;
        let panel = rounded_rect(rect(x, top + 80.0, 110.0, 200.0), 18.0);
        d.ht_add(Ink::Key, 0.18, &panel);
        d.stroke_p(crate::draw::Paint::solid(Ink::Key, 0.8), 3.0, &panel, true);
    }
    d.backing(&slab);
    d.fill(Ink::Yellow, 0.8, &slab);
    d.ht(Ink::Pink, 0.45, &slab);
    d.outline(Ink::Key, LINE + 1.0, &slab);
}

fn plant(d: &mut DrawList, c: V2, s: f32) {
    for (i, a) in [-0.9f32, -0.4, 0.1, 0.6, 1.0].iter().enumerate() {
        let dir = V2::from_angle(-PI * 0.5 + a * 0.8);
        let leaf = crate::geom::lens_along(
            &[c, c + dir * (s * (0.9 + 0.2 * (i % 2) as f32))],
            s * 0.36,
            0.0,
        );
        d.fill(Ink::Blue, 0.55, &leaf);
        d.fill(Ink::Yellow, 0.8, &leaf);
        d.outline(Ink::Key, 3.0, &leaf);
    }
    let pot = vec![
        c + v2(-s * 0.42, 0.0),
        c + v2(s * 0.42, 0.0),
        c + v2(s * 0.32, s * 0.55),
        c + v2(-s * 0.32, s * 0.55),
    ];
    d.backing(&pot);
    d.fill(Ink::Pink, 0.75, &pot);
    d.outline(Ink::Key, LINE, &pot);
}

fn shelf(d: &mut DrawList, y: f32, x0: f32, x1: f32) {
    let board = rounded_rect(rect(x0, y, x1 - x0, 20.0), 6.0);
    d.backing(&board);
    d.fill(Ink::Yellow, 0.75, &board);
    d.ht(Ink::Pink, 0.4, &board);
    d.outline(Ink::Key, LINE, &board);
    for x in [x0 + 40.0, x1 - 40.0] {
        let br = vec![
            v2(x - 10.0, y + 20.0),
            v2(x + 10.0, y + 20.0),
            v2(x, y + 50.0),
        ];
        d.fill(Ink::Key, 0.85, &br);
    }
}

/// Draw a full-screen backdrop.
pub fn backdrop(d: &mut DrawList, b: Backdrop) {
    match b {
        Backdrop::Bakehouse => {
            wall(d, Ink::Yellow, 0.16, Some(Ink::Pink));
            window(d, rect(190.0, 90.0, 340.0, 330.0), |d, o| {
                d.fill(Ink::Pink, 0.3, o);
                d.ht(Ink::Yellow, 0.6, o);
                let sun = circle(v2(360.0, 330.0), 86.0);
                d.knock(&sun);
                d.fill(Ink::Yellow, 1.0, &sun);
                d.ht(Ink::Pink, 0.3, &sun);
                d.outline(Ink::Key, 4.0, &sun);
                cloud(d, v2(270.0, 250.0), 36.0);
                cloud(d, v2(470.0, 190.0), 28.0);
                let hill = crate::geom::ellipse(v2(360.0, 470.0), 260.0, 110.0, 0.0);
                d.fill(Ink::Yellow, 0.7, &hill);
                d.fill(Ink::Blue, 0.5, &hill);
                d.outline(Ink::Key, 4.0, &hill);
            });
            plant(d, v2(610.0, 360.0), 70.0);
            shelf(d, 520.0, 40.0, 300.0);
            counter(d, 1010.0, Ink::Blue);
        }
        Backdrop::Shopfront => {
            wall(d, Ink::Pink, 0.12, None);
            for i in 0..9 {
                let y = 60.0 + i as f32 * 150.0;
                for j in 0..6 {
                    let x = 40.0 + j as f32 * 130.0 + if i % 2 == 1 { 65.0 } else { 0.0 };
                    d.ht_add(Ink::Pink, 0.3, &soft_star(v2(x, y), 12.0, 5.0, 4, 0.0));
                }
            }
            window(d, rect(120.0, 150.0, 480.0, 360.0), |d, o| {
                d.fill(Ink::Blue, 0.28, o);
                cloud(d, v2(250.0, 300.0), 40.0);
                cloud(d, v2(470.0, 250.0), 32.0);
                let street = rect_poly(rect(100.0, 420.0, 520.0, 120.0));
                d.fill(Ink::Yellow, 0.5, &street);
                d.ht(Ink::Pink, 0.2, &street);
                for i in 0..4 {
                    let tree = circle(v2(170.0 + i as f32 * 130.0, 420.0), 34.0);
                    d.fill(Ink::Yellow, 0.9, &tree);
                    d.fill(Ink::Blue, 0.6, &tree);
                    d.outline(Ink::Key, 3.5, &tree);
                }
            });
            // Scalloped awning.
            let top = 100.0;
            for i in 0..8 {
                let x = 100.0 + i as f32 * 65.0;
                let mut stripe = vec![v2(x, top), v2(x + 65.0, top), v2(x + 65.0, top + 70.0)];
                stripe.extend(arc(v2(x + 32.5, top + 70.0), 32.5, 0.0, PI));
                d.backing(&stripe);
                if i % 2 == 0 {
                    d.fill(Ink::Pink, 0.85, &stripe);
                }
                d.outline(Ink::Key, LINE, &stripe);
            }
            counter(d, 930.0, Ink::Yellow);
        }
        Backdrop::Pantry => {
            wall(d, Ink::Blue, 0.2, None);
            d.ht_add(Ink::Yellow, 0.35, &circle(v2(560.0, 170.0), 260.0));
            window(d, rect(60.0, 110.0, 280.0, 300.0), |d, o| {
                d.fill(Ink::Blue, 0.85, o);
                d.fill(Ink::Pink, 0.25, o);
                let moon = circle(v2(210.0, 250.0), 50.0);
                d.knock(&moon);
                d.fill(Ink::Yellow, 0.8, &moon);
                d.knock(&circle(v2(234.0, 236.0), 44.0));
                d.fill(Ink::Blue, 0.85, &circle(v2(234.0, 236.0), 44.0));
                d.fill(Ink::Pink, 0.25, &circle(v2(234.0, 236.0), 44.0));
                for i in 0..14 {
                    let p = v2(80.0 + hash01(1, i) * 240.0, 130.0 + hash01(2, i) * 260.0);
                    d.knock(&soft_star(p, 6.0 + 4.0 * hash01(3, i), 2.4, 4, 0.0));
                }
            });
            // Hanging lamp.
            d.line(Ink::Key, 4.0, &[v2(560.0, -10.0), v2(560.0, 110.0)]);
            let shade = vec![
                v2(510.0, 150.0),
                v2(610.0, 150.0),
                v2(580.0, 100.0),
                v2(540.0, 100.0),
            ];
            d.backing(&shade);
            d.fill(Ink::Pink, 0.8, &shade);
            d.outline(Ink::Key, LINE, &shade);
            let bulb = circle(v2(560.0, 160.0), 16.0);
            d.backing(&bulb);
            d.fill(Ink::Yellow, 1.0, &bulb);
            d.outline(Ink::Key, 3.0, &bulb);
            shelf(d, 700.0, 20.0, 700.0);
            counter(d, 1030.0, Ink::Pink);
        }
    }
}
