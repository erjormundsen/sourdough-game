//! Dough and loaves, seen from above (for scoring) and in cross-section (crumb shots).

use super::{LINE, shadow};
use crate::content::{Inclusion, Recipe, Shape, Stencil, Topping};
use crate::draw::{DrawList, Paint, Screen};
use crate::geom::{
    V2, blob, chaikin, circle, ellipse, heart, lens_along, point_in_poly, resample, soft_star, v2,
};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::{PI, TAU};

/// One scored cut, in loaf-normalised coordinates (unit radius, y down).
#[derive(Clone, Debug, PartialEq)]
pub struct CutView {
    pub pts: Vec<V2>,
    /// 0..1 how far the cut tore open in the oven.
    pub bloom: f32,
    /// 0..1 how much it lifted into an ear on one side.
    pub ear: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoafView {
    pub shape: Shape,
    pub recipe: Recipe,
    /// Radius in reference units.
    pub r: f32,
    /// 0 = raw proofed dough, 1 = fully baked.
    pub bake: f32,
    /// Oven spring 0..1 (size + bloom).
    pub spring: f32,
    /// Crust shade 0 (blonde) .. 1 (bold).
    pub crust: f32,
    pub cuts: Vec<CutView>,
    pub stencil: Option<Stencil>,
    pub topping: Option<Topping>,
    pub seed: u32,
}

impl Default for LoafView {
    fn default() -> Self {
        LoafView {
            shape: Shape::Boule,
            recipe: Recipe::Country,
            r: 150.0,
            bake: 1.0,
            spring: 0.8,
            crust: 0.5,
            cuts: Vec::new(),
            stencil: None,
            topping: None,
            seed: 7,
        }
    }
}

/// Stencil outline in unit space (fits in radius ~0.45).
pub fn stencil_shape(s: Stencil) -> Vec<Vec<V2>> {
    match s {
        Stencil::Heart => vec![heart(v2(0.0, 0.02), 0.78)],
        Stencil::Star => vec![soft_star(v2(0.0, 0.03), 0.46, 0.2, 5, 0.0)],
        Stencil::Sun => {
            let mut v = vec![circle(v2(0.0, 0.0), 0.2)];
            for i in 0..8 {
                let a = TAU * i as f32 / 8.0;
                let dir = V2::from_angle(a);
                let p = dir.perp() * 0.045;
                v.push(vec![dir * 0.27 + p, dir * 0.43, dir * 0.27 - p]);
            }
            v
        }
        Stencil::Bunny => {
            let mut v = vec![ellipse(v2(0.0, 0.1), 0.26, 0.22, 0.0)];
            v.push(ellipse(v2(-0.11, -0.2), 0.07, 0.2, -0.15));
            v.push(ellipse(v2(0.11, -0.2), 0.07, 0.2, 0.15));
            v
        }
    }
}

fn outline_of(v: &LoafView, grow: f32) -> Vec<V2> {
    let (rx, ry) = v.shape.radii();
    blob(V2::ZERO, rx * grow, ry * grow, 0.025, v.seed)
}

/// Loaf size multiplier from spring + bake.
pub fn loaf_scale(v: &LoafView) -> f32 {
    1.0 + 0.12 * v.spring * v.bake
}

/// Draw a loaf (or raw dough when `bake` = 0) centred at the origin.
pub fn loaf_top(d: &mut DrawList, v: &LoafView) {
    let k = v.r * loaf_scale(v);
    let (rx, ry) = v.shape.radii();
    shadow(d, v2(0.0, ry * k * 0.9), rx * k * 1.02, ry * k * 0.28);

    d.push(crate::geom::Xf::IDENTITY.scaled(k));
    let body = outline_of(v, 1.0);
    let b = v.bake.clamp(0.0, 1.0);

    // Base: pale dough → golden crust (knock first so the shadow never shows through).
    let yellow = 0.22 + 0.72 * b;
    d.backing(&body);
    d.fill(Ink::Yellow, yellow, &body);
    d.clipped(&body, |d| {
        // Browning: pink halftone deepening towards the rim, key specks for bold.
        let brown = b * (0.04 + 0.6 * v.crust);
        let rye = if v.recipe == Recipe::DarkRye { 0.14 } else { 0.0 };
        d.ht(Ink::Pink, (brown + 0.12 * b).min(0.95), &body);
        d.ht(Ink::Pink, brown * 0.75, &outline_of(v, 0.78));
        d.ht(Ink::Pink, brown * 0.55, &outline_of(v, 0.5));
        let key = b * ((v.crust - 0.45).max(0.0) * 0.45 + rye);
        if key > 0.01 {
            d.ht(Ink::Key, key, &body);
            d.ht(Ink::Key, key * 0.5, &outline_of(v, 0.72));
        }
        // Whole wheat bran specks.
        if matches!(v.recipe, Recipe::WholeWheat | Recipe::DarkRye) {
            for i in 0..70 {
                let p = scatter(v.seed ^ 0x33, i, rx, ry);
                d.fill(Ink::Key, 0.75, &ellipse(p, 0.012, 0.007, hash01(v.seed, i) * PI));
            }
        }
        // Banneton flour rings (survive the bake as faint lines).
        let ring_tone = 0.9 - 0.45 * b;
        for i in 1..6 {
            let g = i as f32 / 6.0;
            d.knock_line(ring_tone, 0.022, &outline_of(v, g), true);
        }
        // Inclusions peeking through.
        if let Some(inc) = v.recipe.inclusion() {
            for i in 0..9 {
                let p = scatter(v.seed ^ 0x77, i, rx * 0.85, ry * 0.85);
                inclusion_bit(d, inc, p, 0.05 + 0.02 * hash01(v.seed, i + 40), hash01(v.seed, i + 90) * PI);
            }
        }
    });

    // Flour stencil (bright paper).
    if let Some(st) = v.stencil {
        for poly in stencil_shape(st) {
            d.knock_p(0.93, Screen::Solid, 0b0111, &poly);
            d.knock_p(0.35, Screen::Halftone, 0b1000, &poly);
        }
    }

    // Cuts: raw → thin key slashes; baked → blooming lens with an ear.
    for cut in &v.cuts {
        if cut.pts.len() < 2 {
            continue;
        }
        let pts = chaikin(&resample(&cut.pts, 12), 2, false);
        let open = b * cut.bloom;
        if open < 0.05 {
            d.stroke_p(Paint::solid(Ink::Key, 0.95), 0.022, &pts, false);
            continue;
        }
        let width = 0.2 * open;
        let bias = 0.75 * cut.ear;
        // Ear: a raised, darker lip hugging one side of the opening.
        if cut.ear > 0.15 {
            let lip = lens_along(&pts, width * (1.25 + 0.6 * cut.ear), (bias + 0.35).min(1.0));
            d.ht(Ink::Pink, (0.4 + 0.5 * v.crust).min(0.92), &lip);
        }
        // Crumb: pale, a whisper of halftone.
        let lens = lens_along(&pts, width, bias);
        let half = lens.len() / 2;
        d.knock(&lens);
        d.fill(Ink::Yellow, 0.4, &lens);
        d.ht(Ink::Pink, 0.1 + 0.15 * v.crust * b, &lens);
        d.stroke_p(Paint::solid(Ink::Key, 1.0), 0.026, &lens[..half], false);
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 0.016, &lens[half..], false);
    }

    // Toppings.
    if let Some(t) = v.topping {
        let n = match t {
            Topping::Sesame => 70,
            Topping::Oats => 26,
            Topping::Poppy => 140,
        };
        for i in 0..n {
            let p = scatter(v.seed ^ 0x51, i, rx * 0.93, ry * 0.93);
            if v.cuts.iter().any(|c| crate::geom::dist_to_polyline(p, &c.pts) < 0.09 * b.max(0.3)) {
                continue;
            }
            let a = hash01(v.seed ^ 0x9, i) * PI;
            match t {
                Topping::Sesame => {
                    let s = ellipse(p, 0.03, 0.017, a);
                    d.knock(&s);
                    d.fill(Ink::Yellow, 0.3, &s);
                    d.stroke_p(Paint::solid(Ink::Key, 0.8), 0.006, &s, true);
                }
                Topping::Oats => {
                    let s = ellipse(p, 0.05, 0.032, a);
                    d.knock(&s);
                    d.fill(Ink::Yellow, 0.22, &s);
                    d.ht(Ink::Pink, 0.12 * b, &s);
                    d.stroke_p(Paint::solid(Ink::Key, 0.8), 0.008, &s, true);
                }
                Topping::Poppy => {
                    d.fill(Ink::Key, 0.95, &circle(p, 0.011));
                }
            }
        }
    }

    d.outline(Ink::Key, LINE / k * 1.1, &body);
    d.pop();
}

/// Deterministic scatter inside an ellipse.
fn scatter(seed: u32, i: u32, rx: f32, ry: f32) -> V2 {
    let a = hash01(seed, i * 2) * TAU;
    let r = hash01(seed, i * 2 + 1).sqrt();
    v2(a.cos() * rx * r, a.sin() * ry * r)
}

fn inclusion_bit(d: &mut DrawList, inc: Inclusion, p: V2, s: f32, a: f32) {
    match inc {
        Inclusion::Olive => {
            let o = ellipse(p, s, s * 0.7, a);
            d.knock(&o);
            d.fill(Ink::Blue, 0.75, &o);
            d.fill(Ink::Pink, 0.7, &o);
            d.fill(Ink::Key, 0.35, &o);
            d.knock_color(&circle(p + v2(s * 0.2, -s * 0.2), s * 0.22));
        }
        Inclusion::Cranberry => {
            let o = blob(p, s * 0.8, s * 0.7, 0.12, (a * 100.0) as u32);
            d.knock(&o);
            d.fill(Ink::Pink, 1.0, &o);
            d.fill(Ink::Key, 0.18, &o);
        }
        Inclusion::Cheddar => {
            let o = blob(p, s * 1.3, s * 0.9, 0.2, (a * 100.0) as u32);
            d.fill(Ink::Yellow, 1.0, &o);
            d.ht(Ink::Pink, 0.55, &o);
            d.stroke_p(Paint::solid(Ink::Key, 0.5), 0.008, &o, true);
        }
    }
}

/// A cross-section crumb shot. `openness` 0..1 comes from starter pep.
pub fn crumb_slice(d: &mut DrawList, w: f32, h: f32, openness: f32, crust: f32, seed: u32) {
    let mut top = crate::geom::arc(v2(0.0, 0.0), 1.0, PI, TAU);
    for p in top.iter_mut() {
        *p = v2(p.x * w * 0.5, p.y * h * 0.8);
    }
    let mut outline = top;
    outline.push(v2(w * 0.5, h * 0.12));
    outline.extend(crate::geom::quad_bezier(
        v2(w * 0.46, h * 0.2),
        v2(0.0, h * 0.26),
        v2(-w * 0.46, h * 0.2),
        10,
    ));
    outline.push(v2(-w * 0.5, h * 0.12));
    let outline = chaikin(&outline, 2, true);
    shadow(d, v2(0.0, h * 0.26), w * 0.5, h * 0.07);
    d.backing(&outline);
    d.fill(Ink::Yellow, 0.95, &outline);
    d.ht(Ink::Pink, 0.25 + 0.5 * crust, &outline);
    if crust > 0.5 {
        d.ht(Ink::Key, (crust - 0.5) * 0.5, &outline);
    }
    let inner: Vec<V2> = outline.iter().map(|p| v2(p.x * 0.9, p.y * 0.86 + h * 0.012)).collect();
    d.knock(&inner);
    d.fill(Ink::Yellow, 0.3, &inner);
    d.clipped(&inner, |d| {
        let n = (18.0 + 40.0 * openness) as u32;
        for i in 0..n {
            let p = v2((hash01(seed, i * 3) - 0.5) * w * 0.85, -h * 0.7 + hash01(seed, i * 3 + 1) * h * 0.85);
            let big = hash01(seed, i * 3 + 2).powf(2.5);
            let r = (2.0 + big * 14.0 * (0.3 + openness)) * (w / 220.0);
            let hole = ellipse(p, r * 1.2, r, hash01(seed, i) * 0.8 - 0.4);
            if !point_in_poly(p, &inner) {
                continue;
            }
            d.knock(&hole);
            d.ht(Ink::Yellow, 0.45, &ellipse(p + v2(0.0, -r * 0.35), r * 1.1, r * 0.55, 0.0));
            d.stroke_p(Paint::solid(Ink::Key, 0.45), 1.0, &hole, true);
        }
    });
    d.outline(Ink::Key, LINE, &outline);
}
