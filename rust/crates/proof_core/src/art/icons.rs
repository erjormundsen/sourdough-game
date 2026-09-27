//! Small icons for buttons, order bubbles, boards and tickets (drawn around a centre point).
//!
//! Icons are drawn for legibility at 18–45 units of radius: flat spot tints with a two-tone
//! crescent shadow (never halftone — at this size a screen reads as polka dots), one key
//! weight from [`lw`], a paper glint, and silhouettes that stay distinct in a row.

use super::props::{self, arc_band, clip_convex, shade};
use crate::content::{Flour, Pattern, Recipe, Shape, Stencil, Topping, Treat};
use crate::customer::Want;
use crate::draw::{DrawList, PLATES_COLOR, Paint, Screen};
use crate::geom::{
    V2, arc, capsule, chaikin, circle, ellipse, lens_along, quad_bezier, rect, rect_poly, rounded_rect,
    soft_star, v2,
};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Icon {
    Stencil(Stencil),
    Topping(Topping),
    Pattern(Pattern),
    Freehand,
    None,
    Flour(Flour),
    Recipe(Recipe),
    Treat(Treat),
    Want(Want),
    Shape(Shape),
    Undo,
    Check,
    Coin,
    Heart,
    Star,
    Moon,
    Book,
    Back,
    Bowl,
    Oven,
    Shop,
    Jar,
    Fridge,
    /// Morning / Dawn edition.
    Sun,
    /// Settings: sound.
    Sound,
    /// Settings: haptics.
    Buzz,
    /// Settings: calm print (reduced motion).
    Calm,
}

/// Key line weight for an icon of radius `s`.
pub fn lw(s: f32) -> f32 {
    (s * 0.09).clamp(2.0, 4.2)
}

/// Where light comes from (top-left): the offset that keeps the lit part of a shape.
fn light(s: f32) -> V2 {
    v2(-s * 0.16, -s * 0.2)
}

/// Draw `icon` centred on `c`, roughly `s` in radius.
pub fn icon(d: &mut DrawList, icon: Icon, c: V2, s: f32) {
    let w = lw(s);
    match icon {
        Icon::Stencil(st) => stencil(d, c, s * 0.9, st),
        Icon::Topping(t) => topping(d, c + v2(0.0, s * 0.12), s * 0.82, t),
        Icon::Pattern(p) => pattern(d, c, s * 0.88, p),
        Icon::Freehand => lame(d, c, s),
        Icon::None => {
            loaf(d, c, s * 0.8, s * 0.64, 0.35);
            d.outline(Ink::Key, w, &ellipse(c, s * 0.8, s * 0.64, 0.0));
            let b = c + v2(s * 0.52, -s * 0.42);
            let badge = circle(b, s * 0.34);
            d.backing(&badge);
            d.fill(Ink::Pink, 0.25, &badge);
            d.outline(Ink::Key, w * 0.9, &badge);
            d.line(Ink::Key, w * 0.9, &[b + v2(-s * 0.2, s * 0.2), b + v2(s * 0.2, -s * 0.2)]);
        }
        Icon::Flour(f) => sack(d, c, s, f),
        Icon::Recipe(r) => recipe(d, r, c, s),
        Icon::Treat(t) => treat(d, t, c, s),
        Icon::Want(wt) => want(d, wt, c, s),
        Icon::Shape(sh) => {
            let (rx, ry) = sh.radii();
            let (ax, ay) = (s * 0.66 * rx, s * 0.66 * ry);
            loaf(d, c, ax, ay, 0.45);
            scored(d, c, ax, ay, if sh == Shape::Batard { 3 } else { 2 }, w);
            d.outline(Ink::Key, w, &ellipse(c, ax, ay, 0.0));
        }
        Icon::Undo => {
            // A curl from the lower right, over the top, ending at the left pointing down.
            let o = c + v2(s * 0.06, s * 0.1);
            let rr = s * 0.48;
            let a = arc(o, rr, PI * 0.2, -PI * 0.92);
            d.line(Ink::Key, w * 1.5, &a);
            let end = *a.last().unwrap_or(&o);
            let ang = -PI * 0.92;
            let dir = v2(ang.sin(), -ang.cos());
            let side = dir.perp();
            let head = vec![
                end + dir * (s * 0.3),
                end - dir * (s * 0.06) + side * (s * 0.26),
                end - dir * (s * 0.06) - side * (s * 0.26),
            ];
            let head = chaikin(&head, 1, true);
            d.fill(Ink::Key, 1.0, &head);
            d.outline(Ink::Key, w * 0.7, &head);
        }
        Icon::Check => {
            let pts = [c + v2(-s * 0.48, s * 0.02), c + v2(-s * 0.12, s * 0.38), c + v2(s * 0.52, -s * 0.42)];
            d.line(Ink::Key, w * 2.4, &pts);
            d.stroke_p(Paint::solid(Ink::Pink, 1.0), w * 1.1, &pts, false);
        }
        Icon::Coin => props::coin(d, c, s * 0.7),
        Icon::Heart => props::heart_icon(d, c, s * 1.2),
        Icon::Star => {
            let st = soft_star(c, s * 0.82, s * 0.38, 5, 0.0);
            d.backing(&st);
            d.fill(Ink::Yellow, 1.0, &st);
            let inner = soft_star(c + v2(-s * 0.05, -s * 0.07), s * 0.52, s * 0.24, 5, 0.0);
            d.fill(Ink::Pink, 0.3, &st);
            d.knock_p(1.0, Screen::Solid, 1 << Ink::Pink.idx(), &inner);
            d.outline(Ink::Key, w, &st);
        }
        Icon::Moon => {
            let m = circle(c, s * 0.7);
            let bite = circle(c + v2(s * 0.34, -s * 0.24), s * 0.56);
            d.backing(&m);
            d.fill(Ink::Yellow, 0.95, &m);
            d.knock(&bite);
            d.outline(Ink::Key, w, &m);
            d.knock_line(
                1.0,
                w * 2.6,
                &arc(c + v2(s * 0.34, -s * 0.24), s * 0.56, PI * 0.42, PI * 1.28),
                false,
            );
            d.line(Ink::Key, w, &arc(c + v2(s * 0.34, -s * 0.24), s * 0.56, PI * 0.47, PI * 1.23));
            d.line(
                Ink::Key,
                w * 0.8,
                &quad_bezier(
                    c + v2(-s * 0.42, s * 0.12),
                    c + v2(-s * 0.32, s * 0.2),
                    c + v2(-s * 0.22, s * 0.12),
                    6,
                ),
            );
            d.fill_p(
                Paint::solid(Ink::Pink, 0.7),
                &ellipse(c + v2(-s * 0.2, s * 0.32), s * 0.08, s * 0.05, 0.0),
            );
            super::twinkle(d, c + v2(s * 0.62, s * 0.42), s * 0.22, Ink::Yellow);
        }
        Icon::Sun => {
            for i in 0..8 {
                let a = TAU * i as f32 / 8.0 + PI / 8.0;
                let ray =
                    capsule(c + V2::from_angle(a) * (s * 0.62), c + V2::from_angle(a) * (s * 0.86), s * 0.08);
                d.backing(&ray);
                d.fill(Ink::Yellow, 1.0, &ray);
                d.fill(Ink::Pink, 0.3, &ray);
                d.outline(Ink::Key, w * 0.7, &ray);
            }
            let disc = circle(c, s * 0.5);
            d.backing(&disc);
            d.fill(Ink::Yellow, 1.0, &disc);
            shade(d, &disc, light(s * 0.6), Ink::Pink, 0.12, 0.38);
            d.outline(Ink::Key, w, &disc);
            super::face(d, c + v2(0.0, s * 0.04), s * 0.6, super::Expr::Happy, V2::ZERO);
        }
        Icon::Book => {
            for sx in [-1.0f32, 1.0] {
                let page = vec![
                    c + v2(0.0, -s * 0.48),
                    c + v2(sx * s * 0.82, -s * 0.6),
                    c + v2(sx * s * 0.82, s * 0.48),
                    c + v2(0.0, s * 0.62),
                ];
                d.backing(&page);
                d.fill(if sx < 0.0 { Ink::Pink } else { Ink::Blue }, 0.32, &page);
                for k in 0..3 {
                    let y = -s * 0.28 + k as f32 * s * 0.24;
                    d.line(
                        Ink::Key,
                        w * 0.55,
                        &[c + v2(sx * s * 0.18, y), c + v2(sx * s * 0.62, y - s * 0.06)],
                    );
                }
                d.outline(Ink::Key, w, &page);
            }
        }
        Icon::Back => {
            d.line(
                Ink::Key,
                w * 1.7,
                &[c + v2(s * 0.3, -s * 0.52), c + v2(-s * 0.26, 0.0), c + v2(s * 0.3, s * 0.52)],
            );
        }
        Icon::Bowl => {
            let dough = super::super::geom::blob(c + v2(0.0, -s * 0.16), s * 0.62, s * 0.36, 0.05, 2);
            d.backing(&dough);
            d.fill(Ink::Yellow, 0.4, &dough);
            d.outline(Ink::Key, w * 0.85, &dough);
            let mut b = arc(c + v2(0.0, -s * 0.08), s * 0.86, 0.0, PI);
            b.push(c + v2(-s * 0.86, -s * 0.08));
            d.backing(&b);
            d.fill(Ink::Blue, 0.55, &b);
            shade(d, &b, light(s), Ink::Blue, 0.55, 0.8);
            for k in 0..3 {
                let p = c + v2(-s * 0.4 + k as f32 * s * 0.4, s * 0.28);
                d.fill(Ink::Pink, 0.9, &circle(p, s * 0.07));
            }
            d.outline(Ink::Key, w, &b);
        }
        Icon::Oven => {
            let body = rounded_rect(rect(c.x - s * 0.82, c.y - s * 0.7, s * 1.64, s * 1.34), s * 0.3);
            d.backing(&body);
            d.fill(Ink::Pink, 0.6, &body);
            shade(d, &body, light(s), Ink::Pink, 0.6, 0.8);
            d.outline(Ink::Key, w, &body);
            let win = rounded_rect(rect(c.x - s * 0.52, c.y - s * 0.12, s * 1.04, s * 0.56), s * 0.14);
            d.knock(&win);
            d.fill(Ink::Yellow, 0.9, &win);
            d.fill(Ink::Pink, 0.2, &win);
            d.outline(Ink::Key, w * 0.85, &win);
            for sx in [-1.0f32, 1.0] {
                let k = circle(c + v2(sx * s * 0.55, -s * 0.44), s * 0.1);
                d.backing(&k);
                d.fill(Ink::Yellow, 1.0, &k);
                d.outline(Ink::Key, w * 0.7, &k);
            }
            super::face(d, c + v2(0.0, -s * 0.42), s * 0.52, super::Expr::Content, V2::ZERO);
            for sx in [-1.0f32, 1.0] {
                d.fill(
                    Ink::Key,
                    0.9,
                    &rounded_rect(
                        rect(c.x + sx * s * 0.55 - s * 0.1, c.y + s * 0.62, s * 0.2, s * 0.14),
                        s * 0.05,
                    ),
                );
            }
        }
        Icon::Shop => {
            let body = rect_poly(rect(c.x - s * 0.72, c.y - s * 0.18, s * 1.44, s * 0.88));
            d.backing(&body);
            d.fill(Ink::Yellow, 0.35, &body);
            d.outline(Ink::Key, w, &body);
            let door = rounded_rect(rect(c.x + s * 0.12, c.y + s * 0.08, s * 0.36, s * 0.62), s * 0.06);
            d.backing(&door);
            d.fill(Ink::Blue, 0.5, &door);
            d.outline(Ink::Key, w * 0.8, &door);
            let window = rounded_rect(rect(c.x - s * 0.52, c.y + s * 0.08, s * 0.48, s * 0.34), s * 0.05);
            d.backing(&window);
            d.fill(Ink::Blue, 0.22, &window);
            d.outline(Ink::Key, w * 0.8, &window);
            for i in 0..4 {
                let x = c.x - s * 0.8 + i as f32 * s * 0.4;
                let mut aw = vec![v2(x, c.y - s * 0.62), v2(x + s * 0.4, c.y - s * 0.62)];
                aw.extend(arc(v2(x + s * 0.2, c.y - s * 0.26), s * 0.2, 0.0, PI));
                d.backing(&aw);
                if i % 2 == 0 {
                    d.fill(Ink::Pink, 0.9, &aw);
                }
                d.outline(Ink::Key, w * 0.75, &aw);
            }
        }
        Icon::Jar => jar_icon(d, c, s),
        Icon::Fridge => {
            let (ax, ay) = (s * 0.86, s * 0.7);
            let outer = ellipse(c, ax, ay, 0.0);
            d.backing(&outer);
            d.fill(Ink::Yellow, 0.8, &outer);
            d.fill(Ink::Pink, 0.45, &outer);
            for k in [0.86f32, 0.72] {
                d.stroke_p(Paint::solid(Ink::Key, 0.6), w * 0.5, &ellipse(c, ax * k, ay * k, 0.0), true);
            }
            let dough = ellipse(c + v2(0.0, -ay * 0.04), ax * 0.6, ay * 0.6, 0.0);
            d.knock(&dough);
            d.fill(Ink::Yellow, 0.25, &dough);
            d.knock_line(1.0, w * 0.8, &ellipse(c + v2(0.0, -ay * 0.04), ax * 0.34, ay * 0.34, 0.0), true);
            d.outline(Ink::Key, w * 0.8, &dough);
            d.outline(Ink::Key, w, &outer);
        }
        Icon::Sound => {
            let body = vec![
                c + v2(-s * 0.7, -s * 0.22),
                c + v2(-s * 0.36, -s * 0.22),
                c + v2(s * 0.04, -s * 0.58),
                c + v2(s * 0.04, s * 0.58),
                c + v2(-s * 0.36, s * 0.22),
                c + v2(-s * 0.7, s * 0.22),
            ];
            let body = chaikin(&body, 1, true);
            d.backing(&body);
            d.fill(Ink::Blue, 0.55, &body);
            d.outline(Ink::Key, w, &body);
            for k in 0..2 {
                d.line(
                    Ink::Key,
                    w,
                    &arc(c + v2(s * 0.06, 0.0), s * (0.34 + 0.26 * k as f32), -PI * 0.3, PI * 0.3),
                );
            }
        }
        Icon::Buzz => {
            let body = rounded_rect(rect(c.x - s * 0.34, c.y - s * 0.62, s * 0.68, s * 1.24), s * 0.14);
            d.backing(&body);
            d.fill(Ink::Pink, 0.5, &body);
            d.outline(Ink::Key, w, &body);
            let screen = rounded_rect(rect(c.x - s * 0.22, c.y - s * 0.46, s * 0.44, s * 0.74), s * 0.06);
            d.knock(&screen);
            d.fill(Ink::Yellow, 0.3, &screen);
            d.outline(Ink::Key, w * 0.6, &screen);
            for sx in [-1.0f32, 1.0] {
                for k in 0..2 {
                    let x = c.x + sx * s * (0.52 + 0.16 * k as f32);
                    d.line(
                        Ink::Key,
                        w * 0.8,
                        &[v2(x, c.y - s * 0.22), v2(x + sx * s * 0.06, c.y), v2(x, c.y + s * 0.22)],
                    );
                }
            }
        }
        Icon::Calm => {
            // Three soft printed dots settling into register.
            for (i, ink) in [Ink::Pink, Ink::Yellow, Ink::Blue].iter().enumerate() {
                let a = -PI * 0.5 + i as f32 * TAU / 3.0;
                let dot = circle(c + V2::from_angle(a) * (s * 0.24), s * 0.42);
                d.fill_p(Paint::solid(*ink, 0.7).add(), &dot);
            }
            let ring = circle(c, s * 0.72);
            d.outline(Ink::Key, w, &ring);
        }
    }
}

// =========================================================================================
// Building blocks
// =========================================================================================

/// A flat golden loaf body (top view) with a crescent shadow and a glint. No outline.
fn loaf(d: &mut DrawList, c: V2, rx: f32, ry: f32, crust: f32) {
    let body = ellipse(c, rx, ry, 0.0);
    let s = rx.min(ry);
    let pink = 0.2 + 0.46 * crust;
    d.backing(&body);
    d.fill(Ink::Yellow, 0.92, &body);
    shade(d, &body, light(s), Ink::Pink, pink, (pink + 0.24).min(0.95));
    if crust > 0.62 {
        shade(d, &body, light(s), Ink::Key, (crust - 0.62) * 0.35, (crust - 0.62) * 0.7);
    }
    d.knock_p(0.75, Screen::Solid, PLATES_COLOR, &arc_band(c, s * 0.7, PI * 1.12, PI * 1.36, s * 0.09));
}

/// `n` short parallel scores across a loaf of radii `rx`,`ry` (the way a bâtard is cut):
/// unmistakably bread at any size, never a face or a "no" sign.
fn scores(c: V2, rx: f32, ry: f32, n: usize) -> Vec<Vec<V2>> {
    let pitch = rx * if n >= 3 { 0.42 } else { 0.5 };
    (0..n)
        .map(|i| {
            let x = (i as f32 - (n as f32 - 1.0) * 0.5) * pitch;
            let m = c + v2(x, 0.0);
            let (a, b) = (m + v2(-rx * 0.15, ry * 0.36), m + v2(rx * 0.15, -ry * 0.36));
            quad_bezier(a, a.lerp(b, 0.5) + v2(-rx * 0.04, -ry * 0.04), b, 6)
        })
        .collect()
}

/// Draw [`scores`] as opened cuts.
fn scored(d: &mut DrawList, c: V2, rx: f32, ry: f32, n: usize, w: f32) {
    for pts in scores(c, rx, ry, n) {
        cut(d, &pts, (rx * 0.13).max(2.4), w);
    }
}

/// An open score: paper edges, pale crumb, a key hairline.
fn cut(d: &mut DrawList, pts: &[V2], width: f32, w: f32) {
    d.knock_line(1.0, width, pts, false);
    d.fill(Ink::Yellow, 0.42, &lens_along(pts, width * 0.62, 0.0));
    d.stroke_p(Paint::solid(Ink::Key, 1.0), (w * 0.55).max(1.2), pts, false);
}

fn pattern(d: &mut DrawList, c: V2, s: f32, p: Pattern) {
    let w = lw(s);
    let (rx, ry) = (s, s * 0.8);
    loaf(d, c, rx, ry, 0.42);
    for stroke in crate::scoring::template(p, Shape::Boule) {
        let pts: Vec<V2> = stroke.iter().map(|q| c + v2(q.x * rx * 0.95, q.y * ry * 0.95)).collect();
        cut(d, &pts, (s * 0.15).max(2.8), w);
    }
    d.outline(Ink::Key, w, &ellipse(c, rx, ry, 0.0));
}

fn stencil(d: &mut DrawList, c: V2, s: f32, st: Stencil) {
    let w = lw(s);
    loaf(d, c, s, s, 0.45);
    for poly in super::bread::stencil_shape(st) {
        let p: Vec<V2> = poly.iter().map(|q| c + *q * (s * 1.7)).collect();
        d.knock(&p);
        d.stroke_p(Paint::solid(Ink::Key, 0.35), (w * 0.4).max(1.0), &p, true);
    }
    // A few flour specks around the stencil.
    for i in 0..8u32 {
        let a = hash01(31, i) * TAU;
        let p = c + V2::from_angle(a) * (s * (0.55 + 0.25 * hash01(32, i)));
        d.knock_p(0.9, Screen::Solid, PLATES_COLOR, &circle(p, (s * 0.035).max(0.9)));
    }
    d.outline(Ink::Key, w, &circle(c, s));
}

fn topping(d: &mut DrawList, c: V2, s: f32, t: Topping) {
    let w = lw(s);
    // Seeds heaped above the rim.
    let n = 12;
    for i in 0..n {
        let u = (hash01(9, i) - 0.5) * 2.0;
        let p = c + v2(u * s * 0.68, -s * 0.14 - hash01(8, i) * s * 0.42 * (1.0 - u * u * 0.8));
        let a = hash01(7, i) * PI;
        match t {
            Topping::Sesame => {
                let e = ellipse(p, s * 0.15, s * 0.09, a);
                d.backing(&e);
                d.fill(Ink::Yellow, 0.3, &e);
                d.stroke_p(Paint::solid(Ink::Key, 1.0), (w * 0.45).max(1.0), &e, true);
            }
            Topping::Oats => {
                let e = ellipse(p, s * 0.2, s * 0.13, a);
                d.backing(&e);
                d.fill(Ink::Yellow, 0.55, &e);
                d.fill(Ink::Pink, 0.18, &e);
                d.stroke_p(Paint::solid(Ink::Key, 1.0), (w * 0.45).max(1.0), &e, true);
            }
            Topping::Poppy => {
                let e = circle(p, s * 0.085);
                d.backing(&e);
                d.fill(Ink::Key, 0.95, &e);
            }
        }
    }
    let mut bowl = arc(c + v2(0.0, -s * 0.1), s, 0.0, PI);
    bowl.push(c + v2(-s, -s * 0.1));
    d.backing(&bowl);
    d.fill(Ink::Blue, 0.5, &bowl);
    shade(d, &bowl, light(s), Ink::Blue, 0.5, 0.78);
    d.knock_p(
        0.75,
        Screen::Solid,
        PLATES_COLOR,
        &arc_band(c + v2(0.0, -s * 0.1), s * 0.78, PI * 0.62, PI * 0.8, s * 0.07),
    );
    d.outline(Ink::Key, w, &bowl);
}

/// A bread lame: a little wooden handle with a curved razor blade.
fn lame(d: &mut DrawList, c: V2, s: f32) {
    let w = lw(s);
    let a = c + v2(-s * 0.62, s * 0.62);
    let b = c + v2(s * 0.3, -s * 0.3);
    let handle = capsule(a, b, s * 0.13);
    d.backing(&handle);
    d.fill(Ink::Yellow, 0.7, &handle);
    d.fill(Ink::Pink, 0.35, &handle);
    d.outline(Ink::Key, w, &handle);
    let blade = chaikin(
        &[
            b + v2(-s * 0.1, -s * 0.02),
            b + v2(s * 0.34, -s * 0.44),
            b + v2(s * 0.5, -s * 0.3),
            b + v2(s * 0.12, s * 0.08),
        ],
        1,
        true,
    );
    d.backing(&blade);
    d.fill(Ink::Blue, 0.35, &blade);
    d.knock_p(
        0.8,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(b + v2(s * 0.1, -s * 0.12), b + v2(s * 0.3, -s * 0.33), s * 0.04),
    );
    d.outline(Ink::Key, w * 0.9, &blade);
    // A freehand swoosh.
    let sw = quad_bezier(
        c + v2(-s * 0.8, -s * 0.1),
        c + v2(-s * 0.55, -s * 0.72),
        c + v2(-s * 0.05, -s * 0.6),
        10,
    );
    d.stroke_p(Paint::solid(Ink::Pink, 0.9), w * 0.9, &sw, false);
}

/// A flour sack icon: kraft sack, rolled rim, flour peeking out, coloured roundel.
fn sack(d: &mut DrawList, c: V2, s: f32, f: Flour) {
    let w = lw(s);
    let body = chaikin(
        &[
            c + v2(-s * 0.58, s * 0.8),
            c + v2(-s * 0.66, s * 0.1),
            c + v2(-s * 0.52, -s * 0.42),
            c + v2(s * 0.52, -s * 0.42),
            c + v2(s * 0.66, s * 0.1),
            c + v2(s * 0.58, s * 0.8),
        ],
        2,
        true,
    );
    d.backing(&body);
    d.fill(Ink::Yellow, 0.4, &body);
    shade(d, &body, light(s), Ink::Pink, 0.14, 0.32);
    d.outline(Ink::Key, w, &body);
    let mound = {
        let mut p = quad_bezier(
            c + v2(-s * 0.46, -s * 0.44),
            c + v2(0.0, -s * 0.98),
            c + v2(s * 0.46, -s * 0.44),
            10,
        );
        p.push(c + v2(0.0, -s * 0.4));
        p
    };
    d.backing(&mound);
    match f {
        Flour::White => {}
        Flour::Wheat => {
            d.fill(Ink::Yellow, 0.6, &mound);
            d.fill(Ink::Pink, 0.3, &mound);
        }
        Flour::Rye => {
            d.fill(Ink::Pink, 0.4, &mound);
            d.fill(Ink::Blue, 0.35, &mound);
            d.fill(Ink::Key, 0.15, &mound);
        }
    }
    d.outline(Ink::Key, w * 0.85, &mound);
    let rim = chaikin(
        &[
            c + v2(-s * 0.6, -s * 0.52),
            c + v2(s * 0.6, -s * 0.52),
            c + v2(s * 0.56, -s * 0.34),
            c + v2(-s * 0.56, -s * 0.34),
        ],
        1,
        true,
    );
    d.backing(&rim);
    d.fill(Ink::Yellow, 0.5, &rim);
    d.fill(Ink::Pink, 0.22, &rim);
    d.outline(Ink::Key, w * 0.85, &rim);
    let lc = c + v2(0.0, s * 0.24);
    let label = circle(lc, s * 0.3);
    let (ink, tone) = match f {
        Flour::White => (Ink::Blue, 0.6),
        Flour::Wheat => (Ink::Yellow, 0.95),
        Flour::Rye => (Ink::Pink, 0.75),
    };
    d.knock(&label);
    d.fill(ink, tone, &label);
    d.outline(Ink::Key, w * 0.8, &label);
    d.line(Ink::Key, (w * 0.6).max(1.2), &[lc + v2(0.0, s * 0.18), lc + v2(0.0, -s * 0.18)]);
    for k in 0..3 {
        for sx in [-1.0f32, 1.0] {
            d.fill(
                Ink::Key,
                1.0,
                &ellipse(
                    lc + v2(sx * s * 0.055, s * 0.07 - k as f32 * s * 0.08),
                    s * 0.04,
                    s * 0.065,
                    sx * 0.5,
                ),
            );
        }
    }
}

fn recipe(d: &mut DrawList, r: Recipe, c: V2, s: f32) {
    let w = lw(s);
    let (rx, ry) = (s * 0.86, s * 0.76);
    let crust = match r {
        Recipe::Country => 0.42,
        Recipe::WholeWheat => 0.6,
        Recipe::DarkRye => 0.92,
        Recipe::Olive => 0.38,
        Recipe::CranberryWalnut => 0.5,
        Recipe::Cheddar => 0.46,
    };
    loaf(d, c, rx, ry, crust);
    let body = ellipse(c, rx, ry, 0.0);
    match r {
        Recipe::WholeWheat => {
            for i in 0..14u32 {
                let p = c + v2((hash01(41, i) - 0.5) * rx * 1.4, (hash01(42, i) - 0.5) * ry * 1.3);
                let e = ellipse(p, s * 0.045, s * 0.025, hash01(43, i) * PI);
                if clip_convex(&e, &body).len() >= 3 {
                    d.fill(Ink::Key, 0.7, &e);
                }
            }
        }
        Recipe::DarkRye => {
            // Crackled rye top.
            for i in 0..6u32 {
                let a = hash01(44, i) * TAU;
                let p0 = c + V2::from_angle(a) * (s * 0.12);
                let p1 = c + V2::from_angle(a + 0.3) * (s * 0.62);
                d.knock_line(
                    0.9,
                    (s * 0.05).max(1.2),
                    &[p0, p0.lerp(p1, 0.5) + V2::from_angle(a + 1.2) * (s * 0.06), p1],
                    false,
                );
            }
        }
        Recipe::Olive => {
            for i in 0..5u32 {
                let p = c + v2((hash01(45, i) - 0.5) * rx * 1.2, (hash01(46, i) - 0.5) * ry * 1.1);
                let o = ellipse(p, s * 0.1, s * 0.075, 0.3);
                d.backing(&o);
                d.fill(Ink::Yellow, 0.7, &o);
                d.fill(Ink::Blue, 0.7, &o);
                d.outline(Ink::Key, w * 0.55, &o);
                d.knock_color(&circle(p, s * 0.028));
            }
        }
        Recipe::CranberryWalnut => {
            for i in 0..7u32 {
                let p = c + v2((hash01(47, i) - 0.5) * rx * 1.25, (hash01(48, i) - 0.5) * ry * 1.15);
                let o = circle(p, s * 0.075);
                d.backing(&o);
                if i % 3 == 2 {
                    d.fill(Ink::Yellow, 0.6, &o);
                    d.fill(Ink::Pink, 0.55, &o);
                    d.fill(Ink::Key, 0.3, &o);
                } else {
                    d.fill(Ink::Pink, 1.0, &o);
                    d.fill(Ink::Key, 0.15, &o);
                }
                d.outline(Ink::Key, w * 0.5, &o);
            }
        }
        Recipe::Cheddar => {
            for i in 0..4u32 {
                let p = c + v2((hash01(49, i) - 0.5) * rx * 1.1, (hash01(50, i) - 0.5) * ry * 1.0);
                let o = super::super::geom::blob(p, s * 0.2, s * 0.13, 0.25, i + 2);
                let o = clip_convex(&o, &body);
                if o.len() < 3 {
                    continue;
                }
                d.fill(Ink::Yellow, 1.0, &o);
                d.fill(Ink::Pink, 0.6, &o);
                d.stroke_p(Paint::solid(Ink::Key, 0.8), w * 0.45, &o, true);
            }
        }
        Recipe::Country => {}
    }
    if r != Recipe::DarkRye {
        scored(d, c, rx, ry, 3, w);
    }
    d.outline(Ink::Key, w, &body);
}

fn treat(d: &mut DrawList, t: Treat, c: V2, s: f32) {
    let w = lw(s);
    match t {
        Treat::Muffin => {
            let cup = vec![
                c + v2(-s * 0.58, -s * 0.02),
                c + v2(s * 0.58, -s * 0.02),
                c + v2(s * 0.44, s * 0.74),
                c + v2(-s * 0.44, s * 0.74),
            ];
            d.backing(&cup);
            d.fill(Ink::Pink, 0.55, &cup);
            shade(d, &cup, light(s), Ink::Pink, 0.55, 0.8);
            for k in 1..6 {
                let t = k as f32 / 6.0;
                let top = c + v2(-s * 0.58 + t * s * 1.16, -s * 0.02);
                let bot = c + v2(-s * 0.44 + t * s * 0.88, s * 0.74);
                d.line(Ink::Key, w * 0.55, &[top, bot]);
            }
            d.outline(Ink::Key, w, &cup);
            let top = chaikin(
                &[
                    c + v2(-s * 0.72, s * 0.06),
                    c + v2(-s * 0.7, -s * 0.46),
                    c + v2(-s * 0.3, -s * 0.78),
                    c + v2(s * 0.3, -s * 0.78),
                    c + v2(s * 0.7, -s * 0.46),
                    c + v2(s * 0.72, s * 0.06),
                ],
                2,
                true,
            );
            d.backing(&top);
            d.fill(Ink::Yellow, 0.9, &top);
            d.fill(Ink::Pink, 0.35, &top);
            for (x, y) in [(-0.3f32, -0.3f32), (0.22, -0.5), (0.34, -0.12), (-0.05, -0.08)] {
                let b = circle(c + v2(x * s, y * s), s * 0.085);
                d.fill(Ink::Blue, 0.85, &b);
                d.fill(Ink::Pink, 0.4, &b);
            }
            d.knock_p(
                0.8,
                Screen::Solid,
                PLATES_COLOR,
                &arc_band(c + v2(0.0, -s * 0.1), s * 0.52, PI * 1.15, PI * 1.4, s * 0.07),
            );
            d.outline(Ink::Key, w, &top);
        }
        Treat::CinnamonBun => {
            let body = super::super::geom::blob(c, s * 0.8, s * 0.72, 0.03, 3);
            d.backing(&body);
            d.fill(Ink::Yellow, 0.88, &body);
            shade(d, &body, light(s), Ink::Pink, 0.4, 0.62);
            let spiral: Vec<V2> = (0..70)
                .map(|i| {
                    let t = i as f32 / 70.0;
                    let p = V2::from_angle(t * TAU * 2.3 + 0.6) * (s * 0.68 * t);
                    c + v2(p.x, p.y * 0.9)
                })
                .collect();
            d.stroke_p(Paint::solid(Ink::Pink, 0.95), w * 1.1, &spiral, false);
            d.stroke_p(Paint::solid(Ink::Key, 0.7), w * 0.45, &spiral, false);
            let icing: Vec<V2> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.0;
                    c + v2(-s * 0.62 + t * s * 1.24, -s * 0.08 + if i % 2 == 0 { -s * 0.22 } else { s * 0.2 })
                })
                .collect();
            d.knock_line(1.0, w * 1.3, &chaikin(&icing, 2, false), false);
            d.outline(Ink::Key, w, &body);
        }
        Treat::Bagel => {
            let outer = ellipse(c, s * 0.82, s * 0.72, 0.0);
            let hole = ellipse(c + v2(0.0, -s * 0.02), s * 0.22, s * 0.17, 0.0);
            d.backing(&outer);
            d.fill(Ink::Yellow, 0.92, &outer);
            shade(d, &outer, light(s), Ink::Pink, 0.42, 0.66);
            d.knock(&hole);
            for i in 0..12u32 {
                let a = hash01(51, i) * TAU;
                let rr = 0.42 + 0.26 * hash01(52, i);
                let p = c + v2(a.cos() * s * rr, a.sin() * s * rr * 0.86);
                let e = ellipse(p, s * 0.07, s * 0.04, hash01(53, i) * PI);
                d.knock_color(&e);
                d.stroke_p(Paint::solid(Ink::Key, 0.8), (w * 0.35).max(0.9), &e, true);
            }
            d.outline(Ink::Key, w, &outer);
            d.outline(Ink::Key, w * 0.9, &hole);
        }
    }
}

fn want(d: &mut DrawList, wt: Want, c: V2, s: f32) {
    let w = lw(s);
    match wt {
        Want::Recipe(r) => recipe(d, r, c, s),
        Want::Shape(sh) => icon(d, Icon::Shape(sh), c, s),
        Want::Tangy => props::lemon_icon(d, c, s * 0.88),
        Want::Mild => props::cloud_icon(d, c, s * 0.85),
        Want::Crust(cl) => {
            let (rx, ry) = (s * 0.86, s * 0.68);
            loaf(d, c, rx, ry, cl.shade());
            scored(d, c, rx, ry, 3, w);
            d.outline(Ink::Key, w, &ellipse(c, rx, ry, 0.0));
        }
        Want::Pattern(p) => pattern(d, c, s * 0.88, p),
        Want::Stencil(st) => stencil(d, c, s * 0.88, st),
        Want::Topping(t) => topping(d, c + v2(0.0, s * 0.12), s * 0.82, t),
        Want::Treat(t) => treat(d, t, c, s),
        Want::BigEar => {
            let (rx, ry) = (s * 0.86, s * 0.7);
            loaf(d, c, rx, ry, 0.5);
            let pts = quad_bezier(
                c + v2(-rx * 0.66, ry * 0.3),
                c + v2(-rx * 0.1, -ry * 0.55),
                c + v2(rx * 0.66, -ry * 0.18),
                12,
            );
            cut(d, &pts, (s * 0.2).max(3.4), w);
            // The ear: a lifted lip along the cut.
            let ear = lens_along(&pts, (s * 0.16).max(3.0), -0.9);
            let ear: Vec<V2> = ear.iter().map(|p| *p + v2(0.0, -s * 0.07)).collect();
            d.backing(&ear);
            d.fill(Ink::Yellow, 0.95, &ear);
            d.fill(Ink::Pink, 0.62, &ear);
            d.outline(Ink::Key, w * 0.8, &ear);
            d.outline(Ink::Key, w, &ellipse(c, rx, ry, 0.0));
            super::twinkle(d, c + v2(s * 0.72, -s * 0.62), s * 0.3, Ink::Yellow);
        }
        Want::Surprise => props::gift_icon(d, c, s * 0.8),
        Want::AnyLoaf => {
            let (rx, ry) = (s * 0.86, s * 0.68);
            loaf(d, c, rx, ry, 0.45);
            scored(d, c, rx, ry, 3, w);
            d.outline(Ink::Key, w, &ellipse(c, rx, ry, 0.0));
            props::heart_icon(d, c + v2(s * 0.62, -s * 0.5), s * 0.5);
        }
    }
}

/// A tiny starter jar: glass, bubbly starter with a face, gingham cap.
fn jar_icon(d: &mut DrawList, c: V2, s: f32) {
    let w = lw(s);
    let body = rounded_rect(rect(c.x - s * 0.56, c.y - s * 0.56, s * 1.12, s * 1.38), s * 0.26);
    d.backing(&body);
    d.fill(Ink::Blue, 0.14, &body);
    let goo = clip_convex(&rect_poly(rect(c.x - s, c.y - s * 0.12, s * 2.0, s * 2.0)), &body);
    d.fill(Ink::Yellow, 0.55, &goo);
    for (x, y) in [(-0.3f32, 0.46f32), (0.26, 0.2), (0.34, 0.56)] {
        d.knock_color(&circle(c + v2(x * s, y * s), s * 0.06));
    }
    d.fill(Ink::Pink, 0.9, &circle(c + v2(-s * 0.28, s * 0.14), s * 0.07));
    d.fill(Ink::Blue, 0.8, &capsule(c + v2(s * 0.12, s * 0.5), c + v2(s * 0.24, s * 0.46), s * 0.045));
    super::face(d, c + v2(0.0, s * 0.3), s * 0.62, super::Expr::Happy, V2::ZERO);
    d.knock_p(
        0.8,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(c + v2(-s * 0.36, -s * 0.36), c + v2(-s * 0.36, -s * 0.02), s * 0.05),
    );
    d.outline(Ink::Key, w, &body);
    let cap = rounded_rect(rect(c.x - s * 0.66, c.y - s * 0.86, s * 1.32, s * 0.34), s * 0.12);
    d.backing(&cap);
    d.fill(Ink::Yellow, 0.1, &cap);
    for k in 0..4 {
        let x = c.x - s * 0.62 + k as f32 * s * 0.36;
        d.fill_p(
            Paint::solid(Ink::Pink, 0.55).add(),
            &clip_convex(&rect_poly(rect(x, c.y - s * 0.9, s * 0.16, s * 0.4)), &cap),
        );
    }
    d.fill_p(
        Paint::solid(Ink::Pink, 0.55).add(),
        &clip_convex(&rect_poly(rect(c.x - s, c.y - s * 0.75, s * 2.0, s * 0.12)), &cap),
    );
    d.outline(Ink::Key, w * 0.9, &cap);
}
