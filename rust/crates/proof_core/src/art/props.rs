//! Printed props: tickets, rubber stamps, buttons, bubbles, icons, flour bags, bannetons.

use super::{LINE, shadow, twinkle};
use crate::content::{Flour, Pattern, Shape, Stencil, Topping};
use crate::draw::{DrawList, Paint, Screen};
use crate::geom::{
    V2, arc, capsule, chaikin, circle, ellipse, heart, quad_bezier, rect, rect_poly, rounded_rect, soft_star,
    v2, zigzag,
};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::{PI, TAU};

/// An order ticket centred on the origin, `w`×`h`, with a zig-zag tear-off bottom.
pub fn ticket(d: &mut DrawList, w: f32, h: f32, header: Ink) {
    let (x0, y0) = (-w * 0.5, -h * 0.5);
    let bottom = y0 + h - 7.0;
    let mut p = vec![v2(x0, y0), v2(x0 + w, y0), v2(x0 + w, bottom)];
    p.extend(zigzag(v2(x0 + w, bottom), v2(x0, bottom), (w / 18.0) as usize, -7.0).into_iter().skip(1));
    d.ht(Ink::Blue, 0.3, &crate::geom::translate(&p, v2(5.0, 6.0)));
    d.backing(&p);
    d.fill(Ink::Yellow, 0.08, &p);
    d.ht(header, 0.45, &rect_poly(rect(x0, y0, w, 26.0)));
    d.outline(Ink::Key, 3.0, &p);
    let hole = circle(v2(0.0, y0 + 13.0), 5.5);
    d.backing(&hole);
    d.outline(Ink::Key, 2.5, &hole);
}

/// A rubber stamp mark: ring, stars for the grade, distressed ink.
pub fn stamp(d: &mut DrawList, r: f32, stars: u32, ink: Ink, seed: u32) {
    let ring = circle(V2::ZERO, r);
    let inner = circle(V2::ZERO, r * 0.8);
    d.fill_eo(Paint::solid(ink, 0.95), &[ring.clone(), inner.clone()]);
    d.stroke_p(Paint::solid(ink, 0.95), r * 0.035, &circle(V2::ZERO, r * 0.72), true);
    let n = stars.clamp(1, 3);
    for i in 0..n {
        let x = (i as f32 - (n - 1) as f32 * 0.5) * r * 0.42;
        d.fill(ink, 0.95, &soft_star(v2(x, -r * 0.3), r * 0.18, r * 0.08, 5, 0.0));
    }
    d.fill(ink, 0.95, &rect_poly(rect(-r * 0.62, r * 0.06, r * 1.24, r * 0.1)));
    d.fill(ink, 0.95, &rect_poly(rect(-r * 0.62, r * 0.42, r * 1.24, r * 0.05)));
    // Distress: speckles of missing ink.
    for i in 0..(r * 1.4) as u32 {
        let a = hash01(seed, i * 2) * TAU;
        let rr = hash01(seed, i * 2 + 1).sqrt() * r;
        let s = 0.8 + 2.4 * hash01(seed ^ 7, i);
        d.knock_p(0.9, Screen::Solid, 0b1111, &circle(V2::from_angle(a) * rr, s));
    }
}

/// A chunky pill button (label drawn by the engine), centred.
pub fn button(d: &mut DrawList, w: f32, h: f32, ink: Ink, pressed: bool) {
    let y = if pressed { 5.0 } else { 0.0 };
    let r = rect(-w * 0.5, -h * 0.5 + y, w, h);
    if !pressed {
        d.fill(Ink::Key, 0.9, &rounded_rect(rect(-w * 0.5, -h * 0.5 + 6.0, w, h), h * 0.5));
    }
    let body = rounded_rect(r, h * 0.5);
    d.backing(&body);
    d.fill(ink, 0.9, &body);
    d.knock_p(
        0.55,
        Screen::Solid,
        0b0111,
        &capsule(v2(-w * 0.5 + h * 0.45, -h * 0.22 + y), v2(-w * 0.5 + h * 0.8, -h * 0.22 + y), h * 0.08),
    );
    d.outline(Ink::Key, LINE - 0.5, &body);
}

/// A round icon button (for tools), centred.
pub fn round_button(d: &mut DrawList, r: f32, ink: Ink, selected: bool) {
    if selected {
        d.fill(Ink::Yellow, 1.0, &circle(V2::ZERO, r + 9.0));
        d.outline(Ink::Key, 2.5, &circle(V2::ZERO, r + 9.0));
    }
    d.fill(Ink::Key, 0.9, &circle(v2(0.0, 5.0), r));
    let c = circle(V2::ZERO, r);
    d.backing(&c);
    d.fill(ink, 0.35, &c);
    d.outline(Ink::Key, LINE - 0.5, &c);
}

/// Speech bubble centred at origin with a tail pointing to `tail` (relative).
pub fn bubble(d: &mut DrawList, w: f32, h: f32, tail: V2) {
    let body = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 26.0);
    let base = v2(tail.x.clamp(-w * 0.3, w * 0.3), h * 0.5 - 4.0);
    let t = vec![base + v2(-16.0, 0.0), tail, base + v2(16.0, 0.0)];
    d.ht(Ink::Blue, 0.3, &crate::geom::translate(&body, v2(5.0, 6.0)));
    d.backing(&body);
    d.backing(&t);
    d.outline(Ink::Key, LINE - 0.5, &body);
    d.knock(&[base + v2(-13.0, -6.0), base + v2(13.0, -6.0), base + v2(13.0, 2.0), base + v2(-13.0, 2.0)]);
    d.backing(&[base + v2(-13.0, -6.0), base + v2(13.0, -6.0), base + v2(13.0, 2.0), base + v2(-13.0, 2.0)]);
    d.line(Ink::Key, LINE - 0.5, &[base + v2(-16.0, 0.0), tail, base + v2(16.0, 0.0)]);
}

pub fn heart_icon(d: &mut DrawList, c: V2, s: f32) {
    let h = heart(c, s);
    d.fill(Ink::Pink, 1.0, &h);
    d.knock_p(0.8, Screen::Solid, 0b0111, &ellipse(c + v2(-s * 0.2, -s * 0.12), s * 0.09, s * 0.06, -0.6));
    d.outline(Ink::Key, (s * 0.09).max(2.0), &h);
}

pub fn coin(d: &mut DrawList, c: V2, r: f32) {
    d.fill(Ink::Yellow, 1.0, &circle(c, r));
    d.ht(Ink::Pink, 0.35, &circle(c + v2(r * 0.12, r * 0.12), r * 0.8));
    d.fill(Ink::Yellow, 1.0, &circle(c, r * 0.62));
    d.knock_p(0.3, Screen::Solid, 0b0101, &circle(c, r * 0.62));
    d.outline(Ink::Key, (r * 0.14).max(2.0), &circle(c, r));
    d.stroke_p(Paint::solid(Ink::Key, 0.8), (r * 0.08).max(1.5), &circle(c, r * 0.62), true);
    d.fill(Ink::Key, 0.85, &soft_star(c, r * 0.36, r * 0.16, 5, 0.0));
}

/// A tied flour sack. `flour` picks the label ink recipe.
pub fn flour_bag(d: &mut DrawList, flour: Flour) {
    shadow(d, v2(0.0, 2.0), 62.0, 9.0);
    let sack = chaikin(
        &[
            v2(-46.0, 0.0),
            v2(-54.0, -70.0),
            v2(-34.0, -112.0),
            v2(34.0, -112.0),
            v2(54.0, -70.0),
            v2(46.0, 0.0),
        ],
        3,
        true,
    );
    d.backing(&sack);
    d.fill(Ink::Yellow, 0.18, &sack);
    d.outline(Ink::Key, LINE, &sack);
    let top = chaikin(
        &[v2(-30.0, -112.0), v2(-40.0, -140.0), v2(0.0, -126.0), v2(40.0, -140.0), v2(30.0, -112.0)],
        2,
        true,
    );
    d.backing(&top);
    d.fill(Ink::Yellow, 0.18, &top);
    d.outline(Ink::Key, LINE, &top);
    d.line(Ink::Key, 4.0, &quad_bezier(v2(-34.0, -112.0), v2(0.0, -104.0), v2(34.0, -112.0), 8));
    let label = rounded_rect(rect(-34.0, -78.0, 68.0, 52.0), 10.0);
    match flour {
        Flour::White => {
            d.knock(&label);
            d.ht(Ink::Blue, 0.2, &label);
        }
        Flour::Wheat => {
            d.fill(Ink::Yellow, 0.85, &label);
            d.ht(Ink::Pink, 0.3, &label);
        }
        Flour::Rye => {
            d.fill(Ink::Pink, 0.55, &label);
            d.ht(Ink::Key, 0.3, &label);
        }
    }
    d.outline(Ink::Key, 3.0, &label);
    // Wheat sprig on the label.
    let base = v2(0.0, -34.0);
    d.line(Ink::Key, 2.5, &[base, base + v2(0.0, -36.0)]);
    for i in 0..4 {
        let y = -12.0 - i as f32 * 7.0;
        for sx in [-1.0, 1.0] {
            d.fill(Ink::Key, 1.0, &ellipse(base + v2(sx * 5.0, y), 3.2, 5.5, sx * 0.5));
        }
    }
    // Flour dust puff.
    for i in 0..6 {
        let p = v2(-60.0 + hash01(3, i) * 120.0, -8.0 + hash01(4, i) * 10.0);
        d.knock_p(0.6, Screen::Solid, 0b0111, &circle(p, 2.0 + 3.0 * hash01(5, i)));
    }
}

/// A banneton proofing basket, top view, with the dough ball tucked in when `full`.
pub fn banneton(d: &mut DrawList, shape: Shape, r: f32, full: bool) {
    let (rx, ry) = shape.radii();
    let outer = ellipse(V2::ZERO, rx * r, ry * r, 0.0);
    shadow(d, v2(0.0, ry * r * 0.85), rx * r, ry * r * 0.25);
    d.backing(&outer);
    d.fill(Ink::Yellow, 0.85, &outer);
    d.fill(Ink::Pink, 0.45, &outer);
    d.ht(Ink::Key, 0.2, &outer);
    for i in 1..5 {
        let k = 1.0 - i as f32 * 0.07;
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 2.0, &ellipse(V2::ZERO, rx * r * k, ry * r * k, 0.0), true);
    }
    d.outline(Ink::Key, LINE, &outer);
    let inner = ellipse(V2::ZERO, rx * r * 0.7, ry * r * 0.7, 0.0);
    if full {
        let dough = crate::geom::blob(V2::ZERO, rx * r * 0.74, ry * r * 0.74, 0.02, 3);
        d.knock(&dough);
        d.fill(Ink::Yellow, 0.25, &dough);
        for i in 1..5 {
            let k = 0.74 * i as f32 / 5.0;
            d.knock_line(0.9, 3.0, &ellipse(V2::ZERO, rx * r * k, ry * r * k, 0.0), true);
        }
        d.outline(Ink::Key, LINE - 1.0, &dough);
    } else {
        d.knock(&inner);
        d.fill(Ink::Yellow, 0.3, &inner);
        d.ht(Ink::Pink, 0.25, &inner);
        d.outline(Ink::Key, 3.0, &inner);
    }
}

/// Mini loaf silhouette icon with a scoring pattern (order bubbles, guide buttons).
pub fn pattern_icon(d: &mut DrawList, c: V2, s: f32, p: Pattern) {
    let body = ellipse(c, s, s * 0.8, 0.0);
    d.fill(Ink::Yellow, 0.9, &body);
    d.ht(Ink::Pink, 0.4, &body);
    d.outline(Ink::Key, (s * 0.08).max(2.0), &body);
    for stroke in crate::scoring::template(p, Shape::Boule) {
        let pts: Vec<V2> = stroke.iter().map(|q| c + v2(q.x * s * 0.95, q.y * s * 0.78)).collect();
        d.knock_line(1.0, (s * 0.14).max(2.5), &pts, false);
        d.stroke_p(Paint::solid(Ink::Key, 1.0), (s * 0.05).max(1.4), &pts, false);
    }
}

pub fn stencil_icon(d: &mut DrawList, c: V2, s: f32, st: Stencil) {
    let body = circle(c, s);
    d.fill(Ink::Yellow, 0.9, &body);
    d.ht(Ink::Pink, 0.45, &body);
    for poly in crate::art::bread::stencil_shape(st) {
        let p: Vec<V2> = poly.iter().map(|q| c + *q * (s * 1.7)).collect();
        d.knock(&p);
    }
    d.outline(Ink::Key, (s * 0.08).max(2.0), &body);
}

pub fn topping_icon(d: &mut DrawList, c: V2, s: f32, t: Topping) {
    let bowl = {
        let mut p = arc(c + v2(0.0, -s * 0.1), s, 0.0, PI);
        p.push(c + v2(-s, -s * 0.1));
        p
    };
    for i in 0..9 {
        let p = c + v2((hash01(9, i) - 0.5) * s * 1.3, -s * 0.2 - hash01(8, i) * s * 0.35);
        let a = hash01(7, i) * PI;
        match t {
            Topping::Sesame => {
                d.knock(&ellipse(p, s * 0.13, s * 0.08, a));
                d.stroke_p(Paint::solid(Ink::Key, 1.0), 1.5, &ellipse(p, s * 0.13, s * 0.08, a), true);
            }
            Topping::Oats => {
                d.fill(Ink::Yellow, 0.4, &ellipse(p, s * 0.2, s * 0.13, a));
                d.stroke_p(Paint::solid(Ink::Key, 1.0), 1.5, &ellipse(p, s * 0.2, s * 0.13, a), true);
            }
            Topping::Poppy => d.fill(Ink::Key, 1.0, &circle(p, s * 0.07)),
        }
    }
    d.fill(Ink::Blue, 0.55, &bowl);
    d.outline(Ink::Key, (s * 0.08).max(2.0), &bowl);
}

/// A cheeky lemon: the "tangy" order icon.
pub fn lemon_icon(d: &mut DrawList, c: V2, s: f32) {
    let body = chaikin(
        &[
            c + v2(-s, 0.0),
            c + v2(-s * 0.55, -s * 0.62),
            c + v2(s * 0.55, -s * 0.62),
            c + v2(s, 0.0),
            c + v2(s * 0.55, s * 0.62),
            c + v2(-s * 0.55, s * 0.62),
        ],
        3,
        true,
    );
    d.fill(Ink::Yellow, 1.0, &body);
    d.outline(Ink::Key, (s * 0.09).max(2.0), &body);
    crate::art::face(d, c + v2(0.0, -s * 0.05), s * 0.9, crate::art::Expr::Hmm, V2::ZERO);
}

/// A soft milk-cloud: the "mild" order icon.
pub fn cloud_icon(d: &mut DrawList, c: V2, s: f32) {
    let body = crate::geom::scallop(c, s, 7, 0.16);
    d.backing(&body);
    d.ht(Ink::Blue, 0.18, &body);
    d.outline(Ink::Key, (s * 0.09).max(2.0), &body);
    crate::art::face(d, c + v2(0.0, s * 0.05), s * 0.85, crate::art::Expr::Happy, V2::ZERO);
}

/// Crust-shade swatch: a little loaf with 0 (blonde) .. 1 (bold) browning.
pub fn crust_icon(d: &mut DrawList, c: V2, s: f32, crust: f32) {
    let body = ellipse(c, s, s * 0.72, 0.0);
    d.fill(Ink::Yellow, 0.95, &body);
    d.ht(Ink::Pink, 0.15 + 0.6 * crust, &body);
    if crust > 0.5 {
        d.ht(Ink::Key, (crust - 0.5) * 0.7, &body);
    }
    d.line(
        Ink::Key,
        (s * 0.07).max(1.8),
        &quad_bezier(c + v2(-s * 0.5, s * 0.05), c + v2(0.0, -s * 0.25), c + v2(s * 0.5, s * 0.05), 8),
    );
    d.outline(Ink::Key, (s * 0.08).max(2.0), &body);
}

/// "Surprise me!" gift box.
pub fn gift_icon(d: &mut DrawList, c: V2, s: f32) {
    let b = rounded_rect(rect(c.x - s * 0.8, c.y - s * 0.45, s * 1.6, s * 1.2), s * 0.12);
    d.fill(Ink::Blue, 0.6, &b);
    d.fill(Ink::Pink, 0.9, &rect_poly(rect(c.x - s * 0.16, c.y - s * 0.45, s * 0.32, s * 1.2)));
    d.outline(Ink::Key, (s * 0.08).max(2.0), &b);
    for sx in [-1.0, 1.0] {
        let loop_ = ellipse(c + v2(sx * s * 0.3, -s * 0.62), s * 0.3, s * 0.18, sx * 0.4);
        d.fill(Ink::Pink, 0.9, &loop_);
        d.outline(Ink::Key, (s * 0.07).max(1.8), &loop_);
    }
    twinkle(d, c + v2(s * 0.95, -s * 0.8), s * 0.3, Ink::Yellow);
}

/// Masking tape strip (labels).
pub fn tape(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let mut p = zigzag(v2(-w * 0.5, -h * 0.5), v2(-w * 0.5, h * 0.5), 3, -3.0);
    p.extend(zigzag(v2(w * 0.5, h * 0.5), v2(w * 0.5, -h * 0.5), 3, -3.0));
    d.backing(&p);
    d.fill(ink, 0.45, &p);
}

/// A sparkle burst of little stars around the origin.
pub fn burst(d: &mut DrawList, r: f32, n: u32, seed: u32) {
    for i in 0..n {
        let a = TAU * i as f32 / n as f32 + hash01(seed, i) * 0.4;
        let rr = r * (0.75 + 0.35 * hash01(seed, i + 50));
        let ink = if i % 2 == 0 { Ink::Yellow } else { Ink::Pink };
        twinkle(d, V2::from_angle(a) * rr, 9.0 + 7.0 * hash01(seed, i + 99), ink);
    }
}
