//! Toasty, the bakery's pink oven (with a face, obviously).

use super::{Expr, LINE, face, shadow};
use crate::draw::{DrawList, Screen};
use crate::geom::{V2, capsule, circle, rect, rounded_rect, scallop, v2};
use crate::ink::Ink;

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

/// Oven window rectangle in oven-local coordinates (for placing loaves inside).
pub const WINDOW: crate::geom::Rect = rect(-104.0, -206.0, 208.0, 118.0);

/// Draw Toasty with the bottom centre at the origin. `inside` draws the window contents
/// (already clipped to the glass).
pub fn oven(d: &mut DrawList, v: &OvenView, inside: impl FnOnce(&mut DrawList)) {
    shadow(d, v2(0.0, 2.0), 180.0, 18.0);
    // Stubby legs.
    for x in [-118.0, 118.0] {
        let leg = rounded_rect(rect(x - 16.0, -26.0, 32.0, 28.0), 10.0);
        d.backing(&leg);
        d.fill(Ink::Key, 0.9, &leg);
    }
    // Chimney + steam.
    let pipe = rounded_rect(rect(88.0, -352.0, 36.0, 60.0), 8.0);
    d.backing(&pipe);
    d.fill(Ink::Blue, 0.55, &pipe);
    d.outline(Ink::Key, LINE, &pipe);
    if v.steam > 0.02 {
        for i in 0..3 {
            let k = i as f32;
            let rise = (v.t * 0.8 + k * 0.33).fract();
            let c = v2(106.0 + (rise * 6.0).sin() * 8.0, -370.0 - rise * 90.0 - k * 4.0);
            let r = (12.0 + rise * 16.0) * v.steam;
            let puff = scallop(c, r, 7, 0.18);
            d.backing(&puff);
            d.ht(Ink::Blue, 0.2 * (1.0 - rise), &puff);
            d.stroke_p(crate::draw::Paint::solid(Ink::Key, 0.8 * (1.0 - rise)), 2.5, &puff, true);
        }
    }

    let body = rounded_rect(rect(-165.0, -318.0, 330.0, 298.0), 52.0);
    d.backing(&body);
    d.fill(Ink::Pink, 0.62, &body);
    d.ht_add(Ink::Pink, 0.2, &rounded_rect(rect(-165.0, -80.0, 330.0, 60.0), 20.0));
    d.knock_p(0.7, Screen::Solid, 0b0111, &capsule(v2(-138.0, -280.0), v2(-138.0, -240.0), 6.0));
    d.outline(Ink::Key, LINE + 1.5, &body);

    // Control strip with knobs and the face.
    for x in [-118.0, 118.0] {
        let k = circle(v2(x, -276.0), 17.0);
        d.fill(Ink::Yellow, 1.0, &k);
        d.outline(Ink::Key, LINE - 0.5, &k);
        d.line(Ink::Key, 4.0, &[v2(x, -276.0), v2(x + 7.0, -288.0)]);
    }
    face(d, v2(0.0, -272.0), 96.0, v.expr, V2::ZERO);

    // Door (hinged at the bottom, drops towards the viewer as it opens).
    let open = v.open.clamp(0.0, 1.0);
    let door_top = -228.0 + open * 150.0;
    let cavity = rounded_rect(rect(-132.0, -228.0, 264.0, 184.0), 30.0);
    if open > 0.01 {
        d.fill(Ink::Key, 0.72, &cavity);
        d.ht(Ink::Yellow, 0.6 * v.glow, &cavity);
        d.clipped(&cavity, inside_rack);
    }
    let door =
        rounded_rect(rect(-132.0, door_top, 264.0, (-44.0 - door_top).max(20.0)), 30.0 * (1.0 - open * 0.6));
    d.knock(&door);
    d.fill(Ink::Pink, 0.42, &door);
    d.outline(Ink::Key, LINE, &door);
    if open < 0.5 {
        let glass = rounded_rect(WINDOW, 26.0);
        d.knock(&glass);
        d.fill(Ink::Blue, 0.3 * (1.0 - v.glow * 0.7), &glass);
        d.fill(Ink::Key, 0.2 * (1.0 - v.glow * 0.6), &glass);
        d.fill(Ink::Yellow, 0.2 + 0.6 * v.glow, &glass);
        d.ht(Ink::Pink, 0.12 + 0.45 * v.glow, &glass);
        d.clipped(&glass, inside);
        d.knock_p(0.55, Screen::Solid, 0b0111, &capsule(v2(-80.0, -190.0), v2(-40.0, -190.0), 5.0));
        d.outline(Ink::Key, LINE, &glass);
        let handle = capsule(v2(-60.0, -70.0), v2(60.0, -70.0), 8.0);
        d.fill(Ink::Yellow, 1.0, &handle);
        d.outline(Ink::Key, LINE - 0.5, &handle);
    }
}

fn inside_rack(d: &mut DrawList) {
    for y in [-150.0, -90.0] {
        d.line(Ink::Key, 3.0, &[v2(-130.0, y), v2(130.0, y)]);
    }
}
