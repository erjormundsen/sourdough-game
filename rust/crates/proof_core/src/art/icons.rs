//! Small icons for buttons, order bubbles and tickets (drawn around a centre point).

use super::props;
use crate::bake::CrustLevel;
use crate::content::{Flour, Pattern, Recipe, Shape, Stencil, Topping, Treat};
use crate::customer::Want;
use crate::draw::{DrawList, Paint};
use crate::geom::{V2, arc, circle, ellipse, quad_bezier, rect, rounded_rect, soft_star, v2};
use crate::ink::Ink;
use std::f32::consts::PI;

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
}

/// Draw `icon` centred on `c`, roughly `s` in radius.
pub fn icon(d: &mut DrawList, icon: Icon, c: V2, s: f32) {
    let lw = (s * 0.1).max(2.2);
    match icon {
        Icon::Stencil(st) => props::stencil_icon(d, c, s * 0.85, st),
        Icon::Topping(t) => props::topping_icon(d, c + v2(0.0, s * 0.15), s * 0.8, t),
        Icon::Pattern(p) => props::pattern_icon(d, c, s * 0.85, p),
        Icon::Freehand => {
            let body = ellipse(c, s * 0.85, s * 0.68, 0.0);
            d.fill(Ink::Yellow, 0.3, &body);
            d.outline(Ink::Key, lw, &body);
            let blade = [c + v2(-s * 0.55, s * 0.5), c + v2(s * 0.5, -s * 0.55)];
            d.line(Ink::Key, lw * 1.6, &blade);
            d.fill(
                Ink::Blue,
                0.8,
                &rounded_rect(rect(c.x + s * 0.2, c.y - s * 0.85, s * 0.5, s * 0.3), s * 0.08),
            );
            d.outline(
                Ink::Key,
                lw * 0.7,
                &rounded_rect(rect(c.x + s * 0.2, c.y - s * 0.85, s * 0.5, s * 0.3), s * 0.08),
            );
        }
        Icon::None => {
            let o = circle(c, s * 0.6);
            d.outline(Ink::Key, lw, &o);
            d.line(Ink::Key, lw, &[c + v2(-s * 0.42, s * 0.42), c + v2(s * 0.42, -s * 0.42)]);
        }
        Icon::Flour(f) => {
            d.with(crate::geom::Xf::at(c + v2(0.0, s * 0.95)).scaled(s / 95.0), |d| props::flour_bag(d, f));
        }
        Icon::Recipe(r) => recipe_icon(d, r, c, s),
        Icon::Treat(t) => {
            d.with(crate::geom::Xf::at(c), |d| super::treats::treat(d, t, s * 1.7, 5));
        }
        Icon::Want(w) => want_icon(d, w, c, s),
        Icon::Shape(sh) => {
            let (rx, ry) = sh.radii();
            let body = ellipse(c, s * 0.7 * rx, s * 0.7 * ry, 0.0);
            d.fill(Ink::Yellow, 0.9, &body);
            d.ht(Ink::Pink, 0.35, &body);
            d.outline(Ink::Key, lw, &body);
        }
        Icon::Undo => {
            let a = arc(c + v2(0.0, s * 0.1), s * 0.5, PI * 1.05, PI * 2.3);
            d.line(Ink::Key, lw * 1.3, &a);
            let tip = a[0];
            d.fill(
                Ink::Key,
                1.0,
                &[tip + v2(-s * 0.28, -s * 0.05), tip + v2(s * 0.18, -s * 0.2), tip + v2(0.0, s * 0.3)],
            );
        }
        Icon::Check => {
            d.line(
                Ink::Key,
                lw * 1.6,
                &[c + v2(-s * 0.45, 0.0), c + v2(-s * 0.1, s * 0.35), c + v2(s * 0.5, -s * 0.4)],
            );
        }
        Icon::Coin => props::coin(d, c, s * 0.7),
        Icon::Heart => props::heart_icon(d, c, s * 1.2),
        Icon::Star => {
            let st = soft_star(c, s * 0.8, s * 0.36, 5, 0.0);
            d.fill(Ink::Yellow, 1.0, &st);
            d.outline(Ink::Key, lw, &st);
        }
        Icon::Moon => {
            let m = circle(c, s * 0.7);
            d.fill(Ink::Yellow, 0.9, &m);
            d.knock(&circle(c + v2(s * 0.35, -s * 0.25), s * 0.58));
            d.outline(Ink::Key, lw, &m);
            d.knock_line(
                1.0,
                lw * 2.5,
                &arc(c + v2(s * 0.35, -s * 0.25), s * 0.58, PI * 0.45, PI * 1.25),
                false,
            );
            d.line(Ink::Key, lw, &arc(c + v2(s * 0.35, -s * 0.25), s * 0.58, PI * 0.5, PI * 1.2));
        }
        Icon::Book => {
            for sx in [-1.0, 1.0] {
                let page = vec![
                    c + v2(0.0, -s * 0.5),
                    c + v2(sx * s * 0.8, -s * 0.62),
                    c + v2(sx * s * 0.8, s * 0.5),
                    c + v2(0.0, s * 0.62),
                ];
                d.knock(&page);
                d.fill(if sx < 0.0 { Ink::Pink } else { Ink::Blue }, 0.35, &page);
                d.outline(Ink::Key, lw, &page);
            }
        }
        Icon::Back => {
            d.line(
                Ink::Key,
                lw * 1.5,
                &[c + v2(s * 0.4, -s * 0.5), c + v2(-s * 0.3, 0.0), c + v2(s * 0.4, s * 0.5)],
            );
        }
        Icon::Bowl => {
            let mut b = arc(c + v2(0.0, -s * 0.1), s * 0.85, 0.0, PI);
            b.push(c + v2(-s * 0.85, -s * 0.1));
            let dough = super::super::geom::blob(c + v2(0.0, -s * 0.2), s * 0.6, s * 0.35, 0.05, 2);
            d.fill(Ink::Yellow, 0.35, &dough);
            d.outline(Ink::Key, lw * 0.8, &dough);
            d.knock(&b);
            d.fill(Ink::Blue, 0.6, &b);
            d.outline(Ink::Key, lw, &b);
        }
        Icon::Oven => {
            let body = rounded_rect(rect(c.x - s * 0.8, c.y - s * 0.7, s * 1.6, s * 1.4), s * 0.25);
            d.fill(Ink::Pink, 0.62, &body);
            d.outline(Ink::Key, lw, &body);
            let win = rounded_rect(rect(c.x - s * 0.5, c.y - s * 0.2, s, s * 0.6), s * 0.12);
            d.fill(Ink::Yellow, 0.8, &win);
            d.outline(Ink::Key, lw * 0.8, &win);
        }
        Icon::Jar => {
            let v = super::jar::JarView { expr: super::Expr::Happy, ..Default::default() };
            d.with(crate::geom::Xf::at(c + v2(0.0, s * 0.95)).scaled(s / 140.0), |d| super::jar::jar(d, &v));
        }
        Icon::Fridge => {
            d.with(crate::geom::Xf::at(c), |d| props::banneton(d, Shape::Boule, s * 0.8, true));
        }
        Icon::Shop => {
            let body = rect(c.x - s * 0.75, c.y - s * 0.2, s * 1.5, s * 0.9);
            d.knock(&crate::geom::rect_poly(body));
            d.outline(Ink::Key, lw, &crate::geom::rect_poly(body));
            for i in 0..4 {
                let x = c.x - s * 0.8 + i as f32 * s * 0.4;
                let mut aw = vec![v2(x, c.y - s * 0.6), v2(x + s * 0.4, c.y - s * 0.6)];
                aw.extend(arc(v2(x + s * 0.2, c.y - s * 0.25), s * 0.2, 0.0, PI));
                d.fill(if i % 2 == 0 { Ink::Pink } else { Ink::Yellow }, 0.9, &aw);
                d.outline(Ink::Key, lw * 0.7, &aw);
            }
        }
    }
}

fn recipe_icon(d: &mut DrawList, r: Recipe, c: V2, s: f32) {
    let v = super::bread::LoafView {
        recipe: r,
        r: s * 0.9,
        crust: if r == Recipe::DarkRye { 0.75 } else { 0.5 },
        cuts: vec![super::bread::CutView {
            pts: vec![v2(-0.55, 0.25), v2(0.0, -0.15), v2(0.55, 0.15)],
            bloom: 0.8,
            ear: 0.5,
        }],
        seed: r as u32 + 3,
        ..Default::default()
    };
    d.with(crate::geom::Xf::at(c), |d| super::bread::loaf_top(d, &v));
}

fn want_icon(d: &mut DrawList, w: Want, c: V2, s: f32) {
    match w {
        Want::Recipe(r) => recipe_icon(d, r, c, s),
        Want::Shape(sh) => icon(d, Icon::Shape(sh), c, s),
        Want::Tangy => props::lemon_icon(d, c, s * 0.9),
        Want::Mild => props::cloud_icon(d, c, s * 0.85),
        Want::Crust(cl) => props::crust_icon(d, c, s * 0.9, cl.shade()),
        Want::Pattern(p) => props::pattern_icon(d, c, s * 0.85, p),
        Want::Stencil(st) => props::stencil_icon(d, c, s * 0.85, st),
        Want::Topping(t) => props::topping_icon(d, c + v2(0.0, s * 0.15), s * 0.8, t),
        Want::Treat(t) => icon(d, Icon::Treat(t), c, s),
        Want::BigEar => {
            props::pattern_icon(d, c, s * 0.8, Pattern::Ear);
            super::twinkle(d, c + v2(s * 0.7, -s * 0.6), s * 0.3, Ink::Yellow);
        }
        Want::Surprise => props::gift_icon(d, c, s * 0.8),
        Want::AnyLoaf => {
            let body = ellipse(c, s * 0.85, s * 0.65, 0.0);
            d.fill(Ink::Yellow, 0.9, &body);
            d.ht(Ink::Pink, 0.4, &body);
            d.stroke_p(
                Paint::solid(Ink::Key, 1.0),
                (s * 0.08).max(2.0),
                &quad_bezier(c + v2(-s * 0.45, 0.0), c + v2(0.0, -s * 0.2), c + v2(s * 0.45, 0.0), 8),
                false,
            );
            d.outline(Ink::Key, (s * 0.08).max(2.0), &body);
        }
    }
    let _ = CrustLevel::Golden;
}
