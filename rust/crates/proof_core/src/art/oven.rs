//! Toasty, the bakery's pink oven (with a face, obviously).
//!
//! A retro countertop appliance in pink enamel: a streamlined body with bevelled edges and
//! chrome trim, a control panel with the face between two ticked dials and a pilot lamp, a
//! bottom-hinged door with a chrome-framed window (the loaves glow inside on a wire rack), a
//! chrome handle, vent slots, bullet feet and a little chimney that puffs steam. Light comes
//! from the top-left.

use super::style::{DETAIL, INNER, OUTER, contact_shadow};
use super::{Expr, face};
use crate::draw::{DrawList, PLATES_ALL, PLATES_COLOR, Paint, Screen};
use crate::geom::{V2, arc, capsule, chaikin, circle, ellipse, rect, rounded_rect, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OvenView {
    /// 0..1 heat glow in the window.
    pub glow: f32,
    /// 0 = closed .. 1 = door fully down.
    pub open: f32,
    pub expr: Expr,
    /// Steam puffs from the chimney (0..1).
    pub steam: f32,
    pub t: f32,
}

impl Default for OvenView {
    fn default() -> Self {
        OvenView { glow: 0.0, open: 0.0, expr: Expr::Content, steam: 0.0, t: 0.0 }
    }
}

/// Footprint contract (origin = bottom centre between the feet).
pub const OVEN_W: f32 = 330.0;
/// Body top (chimney and steam rise above this).
pub const OVEN_H: f32 = 318.0;

/// Oven window rectangle in oven-local coordinates (for placing loaves inside).
pub const WINDOW: crate::geom::Rect = rect(-104.0, -206.0, 208.0, 118.0);

const BODY_BOTTOM: f32 = -20.0;
const DOOR_TOP: f32 = -240.0;
const HINGE: f32 = -44.0;
const DOOR_HALF: f32 = 136.0;

/// A rounded box with separate top and bottom corner radii.
fn capsule_box(x0: f32, y0: f32, x1: f32, y1: f32, rt: f32, rb: f32) -> Vec<V2> {
    let mut p = Vec::new();
    p.extend(arc(v2(x1 - rt, y0 + rt), rt, -PI * 0.5, 0.0));
    p.extend(arc(v2(x1 - rb, y1 - rb), rb, 0.0, PI * 0.5));
    p.extend(arc(v2(x0 + rb, y1 - rb), rb, PI * 0.5, PI));
    p.extend(arc(v2(x0 + rt, y0 + rt), rt, PI, PI * 1.5));
    p
}

/// Chrome: cool grey with a hard highlight and a dark reflected band.
fn chrome(d: &mut DrawList, poly: &[V2], hl: &[V2], dark: &[V2], line: f32) {
    d.backing(poly);
    d.fill(Ink::Blue, 0.22, poly);
    d.fill(Ink::Key, 0.14, poly);
    d.clipped(poly, |d| {
        if dark.len() >= 3 {
            d.fill(Ink::Key, 0.42, dark);
        }
        if hl.len() >= 3 {
            d.knock(hl);
        }
    });
    d.outline(Ink::Key, line, poly);
}

/// Draw Toasty with the bottom centre at the origin. `inside` draws the window contents
/// (already clipped to the glass).
pub fn oven(d: &mut DrawList, v: &OvenView, inside: impl FnOnce(&mut DrawList)) {
    let open = v.open.clamp(0.0, 1.0);
    let glow = v.glow.clamp(0.0, 1.0);
    contact_shadow(d, v2(6.0, 2.0), 178.0, 17.0);

    // Bullet feet, tucked under the body.
    for x in [-118.0f32, 118.0] {
        let foot = chaikin(
            &[v2(x - 19.0, -26.0), v2(x + 19.0, -26.0), v2(x + 13.0, 0.0), v2(x - 13.0, 0.0)],
            2,
            true,
        );
        chrome(
            d,
            &foot,
            &capsule(v2(x - 9.0, -16.0), v2(x - 7.0, -6.0), 2.2),
            &[v2(x + 4.0, -26.0), v2(x + 19.0, -26.0), v2(x + 13.0, 0.0), v2(x + 3.0, 0.0)],
            INNER,
        );
    }

    // The chimney (behind the body's top).
    let pipe = rounded_rect(rect(90.0, -350.0, 34.0, 44.0), 6.0);
    chrome(
        d,
        &pipe,
        &capsule(v2(97.0, -342.0), v2(97.0, -318.0), 2.4),
        &[v2(112.0, -350.0), v2(124.0, -350.0), v2(124.0, -306.0), v2(112.0, -306.0)],
        INNER,
    );
    let cap = rounded_rect(rect(84.0, -360.0, 46.0, 13.0), 6.0);
    chrome(d, &cap, &capsule(v2(92.0, -355.0), v2(108.0, -355.0), 1.8), &[], INNER);

    // Body: streamlined pink enamel.
    let body = capsule_box(-165.0, -OVEN_H, 165.0, BODY_BOTTOM, 58.0, 34.0);
    d.backing(&body);
    d.fill(Ink::Pink, 0.58, &body);
    d.clipped(&body, |d| {
        // Bevel: the top edge catches the light, the belly rolls into shade.
        d.knock_p(
            0.28,
            Screen::Solid,
            PLATES_COLOR,
            &capsule_box(-150.0, -OVEN_H + 8.0, 150.0, -OVEN_H + 30.0, 20.0, 10.0),
        );
        d.ht_add(Ink::Pink, 0.35, &rounded_rect(rect(-170.0, -48.0, 340.0, 40.0), 12.0));
        d.ht_add(Ink::Key, 0.14, &rounded_rect(rect(-170.0, -36.0, 340.0, 30.0), 10.0));
        // Right flank in shade, left flank lit.
        d.ht_add(
            Ink::Key,
            0.12,
            &chaikin(&[v2(140.0, -330.0), v2(180.0, -330.0), v2(180.0, 0.0), v2(146.0, 0.0)], 2, true),
        );
        d.knock_p(0.8, Screen::Solid, PLATES_ALL, &capsule(v2(-148.0, -262.0), v2(-148.0, -120.0), 3.2));
        d.knock_p(0.8, Screen::Solid, PLATES_ALL, &circle(v2(-148.0, -104.0), 2.6));
    });
    d.outline(Ink::Key, OUTER + 1.0, &body);
    // The front panel's inset edge.
    let panel = capsule_box(-154.0, -OVEN_H + 10.0, 154.0, BODY_BOTTOM - 8.0, 48.0, 26.0);
    d.stroke_p(Paint::solid(Ink::Key, 0.45), DETAIL, &panel, true);

    controls(d, v);

    // Vent slots under the door.
    for i in 0..7 {
        let x = -48.0 + i as f32 * 16.0;
        let slot = capsule(v2(x, -34.0), v2(x, -26.0), 3.0);
        d.fill(Ink::Key, 0.8, &slot);
        d.knock_p(0.5, Screen::Solid, PLATES_ALL, &capsule(v2(x - 0.8, -31.0), v2(x - 0.8, -28.0), 0.9));
    }

    if open > 0.01 {
        cavity(d, glow, open);
        open_door(d, open);
    } else {
        door(d, glow, v.t, inside);
    }

    steam(d, v);
}

/// The control panel: the face between two ticked chrome dials, and a pilot lamp.
fn controls(d: &mut DrawList, v: &OvenView) {
    let cy = -279.0;
    for (i, x) in [-120.0f32, 120.0].iter().enumerate() {
        let c = v2(*x, cy);
        // Ticks around the dial (a 240° scale).
        for k in 0..9 {
            let a = PI * (0.83 + 1.33 * k as f32 / 8.0);
            let dir = V2::from_angle(a);
            let (r0, r1) = if k % 4 == 0 { (26.0, 33.0) } else { (27.0, 31.0) };
            d.line(Ink::Key, if k % 4 == 0 { 2.6 } else { 1.8 }, &[c + dir * r0, c + dir * r1]);
        }
        let knob = circle(c, 21.0);
        d.backing(&knob);
        d.fill(Ink::Blue, 0.2, &knob);
        d.fill(Ink::Key, 0.12, &knob);
        d.clipped(&knob, |d| {
            d.fill(Ink::Key, 0.38, &ellipse(c + v2(7.0, 9.0), 20.0, 15.0, 0.6));
            d.knock(&ellipse(c + v2(-8.0, -9.0), 9.0, 5.0, -0.7));
        });
        // Knurled grip ring.
        for k in 0..16 {
            let a = k as f32 / 16.0 * PI * 2.0;
            let dir = V2::from_angle(a);
            d.line(Ink::Key, 1.3, &[c + dir * 17.5, c + dir * 20.5]);
        }
        d.outline(Ink::Key, INNER, &knob);
        // The pointer: temperature follows the glow, the timer ticks along.
        let turn = if i == 0 { -0.9 + 1.6 * v.glow } else { -0.6 + (v.t * 0.35).fract() * 1.8 };
        let dir = V2::from_angle(-PI * 0.5 + turn);
        let cap = circle(c, 7.0);
        d.fill(Ink::Pink, 0.75, &cap);
        d.outline(Ink::Key, DETAIL, &cap);
        d.line(Ink::Key, 3.4, &[c + dir * 7.0, c + dir * 16.0]);
    }
    // Pilot lamp: glows while baking.
    let lc = v2(-82.0, cy + 22.0);
    let lamp = circle(lc, 5.0);
    if v.glow > 0.3 {
        d.ht_add(Ink::Yellow, 0.5 * v.glow, &circle(lc, 11.0));
    }
    d.backing(&lamp);
    d.fill(Ink::Yellow, 0.35 + 0.65 * v.glow, &lamp);
    d.fill(Ink::Pink, 0.1 + 0.5 * v.glow, &lamp);
    d.outline(Ink::Key, DETAIL, &lamp);
    d.knock(&circle(lc + v2(-1.5, -1.5), 1.4));
    face(d, v2(0.0, cy + 3.0), 124.0, v.expr, V2::ZERO);
}

/// The closed door: pink panel, chrome-framed window with the glowing oven inside.
fn door(d: &mut DrawList, glow: f32, t: f32, inside: impl FnOnce(&mut DrawList)) {
    let door = rounded_rect(rect(-DOOR_HALF, DOOR_TOP, DOOR_HALF * 2.0, HINGE - DOOR_TOP), 30.0);
    d.backing(&door);
    d.fill(Ink::Pink, 0.44, &door);
    d.clipped(&door, |d| {
        d.knock_p(
            0.3,
            Screen::Solid,
            PLATES_COLOR,
            &rounded_rect(rect(-DOOR_HALF + 6.0, DOOR_TOP + 5.0, DOOR_HALF * 2.0 - 12.0, 10.0), 5.0),
        );
        d.ht_add(Ink::Pink, 0.3, &rounded_rect(rect(-DOOR_HALF, HINGE - 18.0, DOOR_HALF * 2.0, 20.0), 6.0));
    });
    d.outline(Ink::Key, INNER + 0.6, &door);

    // Chrome window frame.
    let frame = rounded_rect(WINDOW.grow(9.0), 30.0);
    let glass = rounded_rect(WINDOW, 22.0);
    chrome(
        d,
        &frame,
        &capsule(v2(-100.0, WINDOW.y - 5.0), v2(40.0, WINDOW.y - 5.0), 1.6),
        &rounded_rect(rect(WINDOW.x - 12.0, WINDOW.y + WINDOW.h - 2.0, WINDOW.w + 24.0, 14.0), 6.0),
        INNER,
    );

    // Inside: a warm glow (brightest behind the loaves), the heating element and the rack.
    d.knock(&glass);
    d.clipped(&glass, |d| {
        let (x0, y0, w, h) = (WINDOW.x, WINDOW.y, WINDOW.w, WINDOW.h);
        let dark = 1.0 - glow;
        let base_key = 0.22 + 0.42 * dark;
        d.fill(Ink::Key, base_key, &glass);
        d.fill(Ink::Blue, 0.14 * dark, &glass);
        d.fill(Ink::Yellow, 0.25 * glow, &glass);
        if glow > 0.02 {
            for (i, k) in [1.0f32, 0.78, 0.56, 0.36].iter().enumerate() {
                let e = ellipse(v2(0.0, y0 + h * 0.62), w * 0.62 * k, h * 0.7 * k, 0.0);
                let f = (i + 1) as f32 / 4.0;
                d.fill(Ink::Yellow, (glow * (0.45 + 0.5 * f)).min(1.0), &e);
                d.fill(Ink::Pink, glow * (0.34 - 0.2 * f), &e);
                let kt = base_key * (1.0 - glow * f);
                if kt > 0.01 {
                    d.fill(Ink::Key, kt, &e);
                } else {
                    d.knock_p(1.0, Screen::Solid, 1 << Ink::Key.idx(), &e);
                }
            }
        }
        // Back-wall seams for depth.
        d.stroke_p(Paint::solid(Ink::Key, 0.25), 2.0, &[v2(x0 + 18.0, y0), v2(x0 + 30.0, y0 + h)], false);
        d.stroke_p(
            Paint::solid(Ink::Key, 0.25),
            2.0,
            &[v2(x0 + w - 18.0, y0), v2(x0 + w - 30.0, y0 + h)],
            false,
        );
        // The heating element: a coil glowing across the top.
        let coil: Vec<V2> = (0..=40)
            .map(|i| {
                let u = i as f32 / 40.0;
                v2(x0 + 16.0 + (w - 32.0) * u, y0 + 12.0 + (u * PI * 14.0).sin() * 3.0)
            })
            .collect();
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 4.0, &coil, false);
        if glow > 0.05 {
            d.stroke_p(Paint::solid(Ink::Pink, glow), 3.0, &coil, false);
            d.stroke_p(Paint::solid(Ink::Yellow, glow), 3.0, &coil, false);
            d.stroke_p(Paint::ht(Ink::Yellow, 0.5 * glow).add(), 14.0, &coil, false);
        }
        // Wire rack the loaves sit on.
        let rack_y = y0 + h - 14.0;
        d.line(Ink::Key, 2.6, &[v2(x0, rack_y), v2(x0 + w, rack_y)]);
        d.line(Ink::Key, 1.6, &[v2(x0, rack_y + 7.0), v2(x0 + w, rack_y + 7.0)]);
        for i in 0..9 {
            let x = x0 + 12.0 + i as f32 * (w - 24.0) / 8.0;
            d.line(Ink::Key, 1.4, &[v2(x, rack_y), v2(x, rack_y + 7.0)]);
        }
        inside(d);
        // Heat shimmer over the loaves and a cool glare on the glass.
        if glow > 0.3 {
            for i in 0..3 {
                let x = -60.0 + i as f32 * 60.0 + (t * 2.0 + i as f32).sin() * 4.0;
                let wave: Vec<V2> = (0..=8)
                    .map(|k| v2(x + (k as f32 * 1.3 + t * 3.0).sin() * 3.0, y0 + 36.0 - k as f32 * 3.0))
                    .collect();
                d.stroke_p(Paint::solid(Ink::Yellow, 0.5 * glow).add(), 1.8, &wave, false);
            }
        }
        let glare = |x: f32, w: f32| {
            vec![
                v2(x, y0 - 4.0),
                v2(x + w, y0 - 4.0),
                v2(x + w - 50.0, y0 + h + 4.0),
                v2(x - 50.0, y0 + h + 4.0),
            ]
        };
        d.knock_p(0.35, Screen::Solid, PLATES_ALL, &glare(-40.0, 26.0));
        d.knock_p(0.3, Screen::Solid, PLATES_ALL, &glare(-4.0, 9.0));
    });
    d.outline(Ink::Key, INNER, &glass);

    // Chrome handle across the top of the door, standing off on two brackets.
    let hy = DOOR_TOP + 11.0;
    for x in [-62.0f32, 62.0] {
        d.fill(Ink::Key, 0.35, &ellipse(v2(x + 2.0, hy + 5.0), 8.0, 3.0, 0.0));
        let bracket = rounded_rect(rect(x - 5.0, hy - 2.0, 10.0, 9.0), 3.0);
        chrome(d, &bracket, &[], &[], DETAIL);
    }
    d.ht_add(Ink::Key, 0.3, &capsule(v2(-70.0, hy + 6.0), v2(78.0, hy + 6.0), 5.0));
    let handle = capsule(v2(-74.0, hy - 3.0), v2(74.0, hy - 3.0), 6.5);
    chrome(
        d,
        &handle,
        &capsule(v2(-66.0, hy - 5.5), v2(20.0, hy - 5.5), 1.6),
        &rounded_rect(rect(-80.0, hy - 1.0, 160.0, 6.0), 3.0),
        INNER,
    );

    // A little badge under the window.
    let badge = rounded_rect(rect(-26.0, WINDOW.y + WINDOW.h + 17.0, 52.0, 15.0), 7.5);
    chrome(
        d,
        &badge,
        &capsule(v2(-18.0, WINDOW.y + WINDOW.h + 21.0), v2(4.0, WINDOW.y + WINDOW.h + 21.0), 1.2),
        &[],
        DETAIL,
    );
    d.fill(Ink::Pink, 0.9, &crate::geom::heart(v2(0.0, WINDOW.y + WINDOW.h + 24.5), 9.0));
}

/// The open oven: the cavity in perspective, glowing, with its rack.
fn cavity(d: &mut DrawList, glow: f32, open: f32) {
    let (x0, x1, y0, y1) = (-DOOR_HALF + 4.0, DOOR_HALF - 4.0, DOOR_TOP + 4.0, HINGE - 2.0);
    let mouth = rounded_rect(rect(x0, y0, x1 - x0, y1 - y0), 24.0);
    d.backing(&mouth);
    d.fill(Ink::Key, 0.72, &mouth);
    d.clipped(&mouth, |d| {
        // The back wall, smaller and lit from within.
        let back = rounded_rect(rect(x0 + 34.0, y0 + 26.0, x1 - x0 - 68.0, y1 - y0 - 50.0), 12.0);
        d.fill(Ink::Key, 0.5, &back);
        d.fill(Ink::Yellow, 0.25 + 0.6 * glow, &back);
        d.fill(Ink::Pink, 0.3 * glow, &back);
        // Side walls converge on it.
        for sx in [-1.0f32, 1.0] {
            let wall = vec![
                v2(sx * (x1 - 2.0), y0),
                v2(sx * (x1 - 34.0), y0 + 26.0),
                v2(sx * (x1 - 34.0), y1 - 24.0),
                v2(sx * (x1 - 2.0), y1),
            ];
            d.fill(Ink::Key, if sx < 0.0 { 0.55 } else { 0.8 }, &wall);
            d.fill(Ink::Yellow, 0.3 * glow, &wall);
        }
        // Floor, rack rails and the glowing element.
        let floor = vec![v2(x0, y1), v2(x0 + 34.0, y1 - 24.0), v2(x1 - 34.0, y1 - 24.0), v2(x1, y1)];
        d.fill(Ink::Key, 0.65, &floor);
        let coil: Vec<V2> = (0..=30)
            .map(|i| {
                let u = i as f32 / 30.0;
                v2(x0 + 44.0 + (x1 - x0 - 88.0) * u, y0 + 34.0 + (u * PI * 10.0).sin() * 2.5)
            })
            .collect();
        d.stroke_p(Paint::solid(Ink::Pink, 0.4 + 0.6 * glow), 3.0, &coil, false);
        d.stroke_p(Paint::solid(Ink::Yellow, 0.3 + 0.7 * glow), 3.0, &coil, false);
        let rack_y = y1 - 44.0;
        d.stroke_p(
            Paint::solid(Ink::Yellow, 0.35),
            2.6,
            &[v2(x0 + 30.0, rack_y), v2(x1 - 30.0, rack_y)],
            false,
        );
        for i in 0..8 {
            let x = x0 + 44.0 + i as f32 * (x1 - x0 - 88.0) / 7.0;
            d.stroke_p(
                Paint::solid(Ink::Yellow, 0.3),
                1.6,
                &[v2(x, rack_y), v2(x * 1.12, rack_y + 18.0)],
                false,
            );
        }
        d.stroke_p(
            Paint::solid(Ink::Yellow, 0.35),
            2.6,
            &[v2(x0 + 12.0, rack_y + 18.0), v2(x1 - 12.0, rack_y + 18.0)],
            false,
        );
    });
    d.outline(Ink::Key, INNER, &mouth);
    let _ = open;
}

/// The door swung down towards us, showing its inner face in perspective.
fn open_door(d: &mut DrawList, open: f32) {
    let theta = open * PI * 0.5;
    let h = HINGE - DOOR_TOP;
    // Seen from a little above: the free edge sweeps down past the hinge.
    let far = HINGE - h * theta.cos() + h * 0.36 * theta.sin();
    let spread = 1.0 + 0.1 * theta.sin();
    let (y_a, y_b) = if far < HINGE { (far, HINGE) } else { (HINGE, far) };
    let w_far = DOOR_HALF * spread;
    let slab = if far >= HINGE {
        vec![v2(-DOOR_HALF, HINGE), v2(DOOR_HALF, HINGE), v2(w_far, far), v2(-w_far, far)]
    } else {
        vec![v2(-w_far, far), v2(w_far, far), v2(DOOR_HALF, HINGE), v2(-DOOR_HALF, HINGE)]
    };
    let slab = chaikin(&slab, 1, true);
    d.backing(&slab);
    d.fill(Ink::Pink, 0.5, &slab);
    d.ht_add(Ink::Key, 0.18, &slab);
    // The inner glass, foreshortened.
    let inset = (y_b - y_a) * 0.16;
    let glass = vec![
        v2(-DOOR_HALF * 0.74, y_a + inset),
        v2(DOOR_HALF * 0.74, y_a + inset),
        v2(DOOR_HALF * 0.78, y_b - inset * 1.6),
        v2(-DOOR_HALF * 0.78, y_b - inset * 1.6),
    ];
    if (y_b - y_a) > 12.0 {
        d.fill(Ink::Key, 0.45, &glass);
        d.fill(Ink::Blue, 0.2, &glass);
        d.knock_p(
            0.3,
            Screen::Solid,
            PLATES_ALL,
            &[glass[0], glass[0] + v2(30.0, 0.0), glass[3] + v2(20.0, 0.0), glass[3]],
        );
        d.outline(Ink::Key, DETAIL, &glass);
    }
    d.outline(Ink::Key, INNER + 0.6, &slab);
    // The handle now hangs at the free edge.
    let hy = far + if far >= HINGE { 6.0 } else { -6.0 };
    let handle = capsule(v2(-74.0 * spread, hy), v2(74.0 * spread, hy), 6.0);
    chrome(d, &handle, &capsule(v2(-60.0, hy - 2.5), v2(10.0, hy - 2.5), 1.5), &[], INNER);
}

/// Steam puffs from the chimney: soft clouds that rise, grow and fade.
fn steam(d: &mut DrawList, v: &OvenView) {
    if v.steam <= 0.02 {
        return;
    }
    for i in 0..3 {
        let k = i as f32;
        let rise = (v.t * 0.7 + k * 0.33).fract();
        let c = v2(106.0 + (rise * 6.0 + k).sin() * 10.0 + rise * 14.0, -372.0 - rise * 84.0 - k * 3.0);
        let r = (10.0 + rise * 16.0) * v.steam;
        if r < 2.0 {
            continue;
        }
        let fade = (1.0 - rise).powf(0.7);
        let lobes =
            [(v2(-0.55, 0.15), 0.62), (v2(0.5, 0.2), 0.58), (v2(0.0, -0.3), 0.72), (v2(0.05, 0.3), 0.6)];
        let mut puff: Vec<Vec<V2>> = Vec::new();
        for (o, s) in lobes {
            puff.push(circle(c + o * r, r * s));
        }
        for p in &puff {
            d.backing(p);
        }
        for p in &puff {
            d.fill_p(
                Paint::ht(Ink::Blue, 0.16 * fade).add(),
                &ellipse(centroid(p) + v2(r * 0.12, r * 0.18), r * 0.5, r * 0.4, 0.0),
            );
        }
        for p in &puff {
            d.stroke_p(Paint::solid(Ink::Key, 0.75 * fade), 2.2, p, true);
        }
        // Erase the inner overlaps so the puff reads as one cloud.
        for p in &puff {
            let inner: Vec<V2> = p.iter().map(|q| centroid(p) + (*q - centroid(p)) * 0.9).collect();
            d.knock_p(1.0, Screen::Solid, 1 << Ink::Key.idx(), &inner);
        }
        d.knock_p(
            0.6,
            Screen::Solid,
            PLATES_ALL,
            &ellipse(c + v2(-r * 0.3, -r * 0.35), r * 0.25, r * 0.14, -0.5),
        );
    }
}

fn centroid(p: &[V2]) -> V2 {
    p.iter().fold(V2::ZERO, |a, q| a + *q) / p.len().max(1) as f32
}
