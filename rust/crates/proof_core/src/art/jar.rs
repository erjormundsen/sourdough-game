//! Starter jars — the pets. The microbiome is drawn literally: pink round Yeasties and blue
//! bean-shaped Lactos swim in creamy starter; their mix *is* the tang dial.

use super::{Expr, LINE, face, shadow, zzz};
use crate::draw::{DrawList, Screen};
use crate::geom::{V2, capsule, circle, ellipse, quad_bezier, rect, rounded_rect, v2, zigzag};
use crate::ink::Ink;
use crate::rng::hash01;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JarView {
    /// Liveliness 0..1: number of microbes, bubbles and how domed the top is.
    pub pep: f32,
    /// 0 = mild (all Yeasties) .. 1 = tangy (mostly Lactos).
    pub tang: f32,
    /// Fill height 0..1.
    pub rise: f32,
    /// Rubber band height 0..1 (level right after the last feed).
    pub band: f32,
    pub hooch: bool,
    pub expr: Expr,
    /// Animation phase (seconds). Quantise before caching.
    pub t: f32,
    pub seed: u32,
    /// Ink of the gingham cloth cap.
    pub cloth: Ink,
}

impl Default for JarView {
    fn default() -> Self {
        JarView {
            pep: 0.7,
            tang: 0.35,
            rise: 0.55,
            band: 0.35,
            hooch: false,
            expr: Expr::Content,
            t: 0.0,
            seed: 1,
            cloth: Ink::Pink,
        }
    }
}

pub const JAR_W: f32 = 170.0;
pub const JAR_H: f32 = 250.0;

fn level_y(rise: f32) -> f32 {
    -(26.0 + rise.clamp(0.0, 1.0) * 150.0)
}

/// A Yeastie: round pink blob with a tiny face.
pub fn yeastie(d: &mut DrawList, c: V2, r: f32) {
    d.fill(Ink::Pink, 0.95, &circle(c, r));
    d.fill(Ink::Key, 1.0, &circle(c + v2(-r * 0.32, -r * 0.1), r * 0.16));
    d.fill(Ink::Key, 1.0, &circle(c + v2(r * 0.32, -r * 0.1), r * 0.16));
}

/// A Lacto: blue capsule with a tiny face.
pub fn lacto(d: &mut DrawList, c: V2, r: f32, ang: f32) {
    let dir = V2::from_angle(ang) * (r * 1.1);
    d.fill(Ink::Blue, 0.9, &capsule(c - dir, c + dir, r * 0.72));
    let n = V2::from_angle(ang).perp();
    let _ = n;
    d.fill(Ink::Key, 1.0, &circle(c + v2(-r * 0.3, -r * 0.12), r * 0.15));
    d.fill(Ink::Key, 1.0, &circle(c + v2(r * 0.3, -r * 0.12), r * 0.15));
}

/// Draw a starter jar with its bottom centre at the origin.
pub fn jar(d: &mut DrawList, v: &JarView) {
    let body = rounded_rect(rect(-85.0, -200.0, 170.0, 200.0), 42.0);
    let neck = rounded_rect(rect(-63.0, -222.0, 126.0, 34.0), 12.0);
    let inner = rounded_rect(rect(-80.0, -195.0, 160.0, 190.0), 38.0);
    let hooch = v.hooch;
    let rise = if hooch { v.rise.min(0.3) } else { v.rise };
    let y_top = level_y(rise);

    shadow(d, v2(0.0, 4.0), 104.0, 16.0);

    // Glass (opaque paper behind so the jar never looks see-through).
    d.backing(&neck);
    d.fill(Ink::Blue, 0.13, &neck);
    d.outline(Ink::Key, LINE, &neck);
    d.backing(&body);
    d.fill(Ink::Blue, 0.13, &body);

    // Starter inside the glass.
    d.clipped(&inner, |d| {
        let dome = if hooch { 0.0 } else { 6.0 + 12.0 * v.pep };
        let wob = v.t * 1.7;
        let mut goop = vec![v2(-100.0, 10.0)];
        for i in 0..=24 {
            let x = -90.0 + 180.0 * i as f32 / 24.0;
            let u = x / 85.0;
            let wave = (x * 0.09 + wob).sin() * 1.6 * v.pep;
            goop.push(v2(x, y_top - dome * (1.0 - u * u).max(0.0) + wave));
        }
        goop.push(v2(100.0, 10.0));
        d.fill(Ink::Yellow, 0.42, &goop);
        d.knock_p(1.0, Screen::Solid, 1 << 2, &goop);
        // A slightly deeper band at the bottom: the starter settles.
        d.ht_add(Ink::Yellow, 0.35, &rounded_rect(rect(-90.0, -34.0, 180.0, 40.0), 10.0));

        // Bubbles.
        let bubbles = (4.0 + 18.0 * v.pep) as u32;
        for i in 0..bubbles {
            let hx = hash01(v.seed, i * 3);
            let hy = hash01(v.seed, i * 3 + 1);
            let hr = hash01(v.seed, i * 3 + 2);
            let x = -70.0 + 140.0 * hx;
            let y = y_top + 10.0 + (-12.0 - y_top - 10.0) * hy;
            let r = 1.8 + 4.5 * hr * hr * (0.5 + v.pep);
            d.knock_color(&circle(v2(x, y), r));
            if r > 3.5 {
                d.stroke_p(crate::draw::Paint::solid(Ink::Key, 0.55), 1.2, &circle(v2(x, y), r), true);
            }
        }

        // Microbes: count grows with pep, mix follows tang.
        let n = (3.0 + 9.0 * v.pep).round() as u32;
        let n_lacto = (n as f32 * (0.12 + 0.76 * v.tang)).round() as u32;
        let span = (-18.0 - (y_top + 16.0)).max(8.0);
        for i in 0..n {
            let hx = hash01(v.seed ^ 0xabc, i * 5);
            let hy = hash01(v.seed ^ 0xabc, i * 5 + 1);
            let ph = hash01(v.seed ^ 0xabc, i * 5 + 2) * std::f32::consts::TAU;
            let bob = (v.t * (1.4 + v.pep) + ph).sin() * (1.5 + 2.5 * v.pep);
            let c = v2(-62.0 + 124.0 * hx, y_top + 16.0 + span * hy + bob);
            // Keep the face area clear.
            let face_c = v2(0.0, ((y_top - 8.0).max(-150.0) * 0.5).min(-36.0));
            if (c.x - face_c.x).abs() < 50.0 && (c.y - face_c.y).abs() < 30.0 {
                continue;
            }
            if i < n_lacto {
                lacto(d, c, 7.0, ph);
            } else {
                yeastie(d, c, 7.2);
            }
        }

        if hooch {
            let top = y_top - 16.0;
            let layer = rounded_rect(rect(-90.0, top, 180.0, 18.0), 2.0);
            d.fill(Ink::Blue, 0.3, &layer);
            d.fill(Ink::Yellow, 0.15, &layer);
            d.ht(Ink::Key, 0.14, &layer);
        }

        let face_y = ((y_top - 8.0).max(-150.0) * 0.5).min(-36.0);
        face(d, v2(0.0, face_y), 82.0, v.expr, V2::ZERO);
    });

    // Glass shine.
    d.knock_p(0.85, Screen::Solid, 0b0111, &capsule(v2(-61.0, -172.0), v2(-61.0, -92.0), 5.5));
    d.knock_p(0.85, Screen::Solid, 0b0111, &circle(v2(-61.0, -76.0), 4.2));

    d.outline(Ink::Key, LINE + 0.5, &body);

    // Rubber band: marks the level right after the last feed.
    let yb = level_y(v.band);
    d.line(Ink::Pink, 7.0, &quad_bezier(v2(-86.0, yb - 2.0), v2(0.0, yb + 9.0), v2(86.0, yb - 2.0), 16));

    // Gingham cloth cap + string bow.
    let cloth = {
        let mut p = vec![v2(-72.0, -250.0), v2(72.0, -250.0)];
        p.extend(quad_bezier(v2(84.0, -234.0), v2(92.0, -214.0), v2(78.0, -206.0), 6));
        p.extend(zigzag(v2(78.0, -206.0), v2(-78.0, -206.0), 7, -5.0));
        p.extend(quad_bezier(v2(-78.0, -206.0), v2(-92.0, -214.0), v2(-84.0, -234.0), 6));
        crate::geom::chaikin(&p, 2, true)
    };
    d.backing(&cloth);
    d.clipped(&cloth, |d| {
        for i in -5..=5 {
            let x = i as f32 * 22.0;
            d.fill(v.cloth, 0.42, &crate::geom::rect_poly(rect(x - 5.5, -260.0, 11.0, 60.0)));
        }
        for j in 0..4 {
            let y = -252.0 + j as f32 * 22.0;
            d.fill_p(
                crate::draw::Paint::solid(v.cloth, 0.42).add(),
                &crate::geom::rect_poly(rect(-100.0, y - 5.5, 200.0, 11.0)),
            );
        }
    });
    d.outline(Ink::Key, LINE, &cloth);
    d.line(Ink::Key, 3.2, &quad_bezier(v2(-66.0, -214.0), v2(0.0, -208.0), v2(66.0, -214.0), 10));
    let bow = v2(40.0, -212.0);
    d.outline(Ink::Key, 3.0, &ellipse(bow + v2(-9.0, -5.0), 9.0, 5.5, 0.5));
    d.outline(Ink::Key, 3.0, &ellipse(bow + v2(9.0, -5.0), 9.0, 5.5, -0.5));
    d.line(Ink::Key, 3.0, &[bow, bow + v2(-6.0, 12.0)]);
    d.line(Ink::Key, 3.0, &[bow, bow + v2(7.0, 11.0)]);

    // Masking-tape name label (text is drawn by the engine on top).
    let tape = {
        let mut p = zigzag(v2(-52.0, -172.0), v2(-52.0, -150.0), 3, -3.0);
        p.extend(zigzag(v2(52.0, -150.0), v2(52.0, -172.0), 3, -3.0));
        p
    };
    d.with(crate::geom::Xf::IDENTITY.rotated(-0.035), |d| {
        d.knock_color(&tape);
        d.fill(Ink::Yellow, 0.5, &tape);
        d.ht_add(Ink::Pink, 0.1, &tape);
    });

    if hooch {
        zzz(d, v2(70.0, -262.0), 14.0);
    }
}

/// Where the engine should place the name label (reference units, jar-local).
pub const TAPE_CENTER: V2 = v2(0.0, -161.0);
