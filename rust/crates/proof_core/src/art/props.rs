//! Printed props and the UI kit: buttons, trays, tickets, slips, stamps, tape, bubbles,
//! ribbons, the masthead, cards, boards, signs, flour sacks, bannetons and small icon art.
//!
//! Everything here follows the house style (see [`super::style`]): one confident `OUTER`
//! silhouette, `INNER` structure, `DETAIL` texture; flat tints and stepped halftone ramps
//! instead of big solids; paper `backing()` under anything opaque; a printed halftone
//! offset shadow under floating paper and a [`contact_shadow`] under things that rest on a
//! surface. Origins are centred unless a function says otherwise.

use super::style::{DETAIL, INNER, OUTER, contact_shadow};
use super::twinkle;
use crate::content::{Flour, Pattern, Shape, Stencil, Topping};
use crate::draw::{DrawList, PLATES_COLOR, Paint, Screen};
use crate::geom::{
    Rect, V2, arc, capsule, chaikin, circle, ellipse, heart, quad_bezier, rect, rect_poly, rounded_rect,
    soft_star, translate, v2, zigzag,
};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::{PI, TAU};

// =========================================================================================
// Drawing utilities (shared with scenes and icons)
// =========================================================================================

/// Clip a polygon to the half-plane `(p - o)·n >= 0` (Sutherland–Hodgman).
pub fn clip_half(poly: &[V2], o: V2, n: V2) -> Vec<V2> {
    let k = poly.len();
    let mut out = Vec::with_capacity(k + 4);
    for i in 0..k {
        let a = poly[i];
        let b = poly[(i + 1) % k];
        let da = (a - o).dot(n);
        let db = (b - o).dot(n);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push(a.lerp(b, t));
        }
    }
    out
}

/// Clip a polygon to an axis-aligned rectangle.
pub fn clip_rect(poly: &[V2], r: Rect) -> Vec<V2> {
    let p = clip_half(poly, v2(r.x, 0.0), v2(1.0, 0.0));
    let p = clip_half(&p, v2(r.x + r.w, 0.0), v2(-1.0, 0.0));
    let p = clip_half(&p, v2(0.0, r.y), v2(0.0, 1.0));
    clip_half(&p, v2(0.0, r.y + r.h), v2(0.0, -1.0))
}

/// Intersect `poly` with a convex polygon `clip` (either winding).
pub fn clip_convex(poly: &[V2], clip: &[V2]) -> Vec<V2> {
    let n = clip.len();
    if n < 3 {
        return poly.to_vec();
    }
    let area: f32 = (0..n).map(|i| clip[i].cross(clip[(i + 1) % n])).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut out = poly.to_vec();
    for i in 0..n {
        if out.len() < 3 {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % n];
        // Inward normal (y-down, clockwise positive area).
        let e = b - a;
        let nrm = v2(-e.y, e.x) * sign;
        out = clip_half(&out, a, nrm);
    }
    out
}

/// Flat two-tone shading on one ink plate: convex `body` gets `dark`, except the part still
/// covered by `body` shifted by `light`, which keeps `base` (a crescent shadow on the far side).
pub fn shade(d: &mut DrawList, body: &[V2], light: V2, ink: Ink, base: f32, dark: f32) {
    d.fill(ink, dark, body);
    let lit: Vec<V2> = body.iter().map(|p| *p + light).collect();
    let lit = clip_convex(&lit, body);
    if lit.len() >= 3 {
        if base > 0.004 {
            d.fill(ink, base, &lit);
        } else {
            d.knock_p(1.0, Screen::Solid, 1 << ink.idx(), &lit);
        }
    }
}

/// A stepped gradient across `poly` from `a` (tone `t0`) to `b` (tone `t1`): the riso way of
/// shading, one halftone (or tint) band per step. Uses exact band polygons, no clip masks.
#[allow(clippy::too_many_arguments)]
pub fn ramp_p(d: &mut DrawList, base: Paint, poly: &[V2], a: V2, b: V2, t0: f32, t1: f32, steps: usize) {
    let steps = steps.max(1);
    let len = (b - a).len();
    if len < 1e-3 || poly.len() < 3 {
        return;
    }
    let dir = (b - a) / len;
    for i in 0..steps {
        let s0 = i as f32 / steps as f32;
        let s1 = (i + 1) as f32 / steps as f32;
        let mut band = poly.to_vec();
        if i > 0 {
            band = clip_half(&band, a + dir * (len * s0), dir);
        }
        if i + 1 < steps {
            band = clip_half(&band, a + dir * (len * s1), -dir);
        }
        let tone = t0 + (t1 - t0) * (i as f32 + 0.5) / steps as f32;
        if band.len() >= 3 && tone > 0.004 {
            d.fill_p(Paint { tone, ..base }, &band);
        }
    }
}

/// Halftone ramp on one ink (replaces that plate inside `poly`).
#[allow(clippy::too_many_arguments)]
pub fn ramp(d: &mut DrawList, ink: Ink, poly: &[V2], a: V2, b: V2, t0: f32, t1: f32, steps: usize) {
    ramp_p(d, Paint::ht(ink, 1.0), poly, a, b, t0, t1, steps);
}

/// A radial halftone glow: nested ellipses from `t_out` at the rim to `t_in` at the centre,
/// optionally clipped to a rectangle. Unions with the plate (never erases what is there).
#[allow(clippy::too_many_arguments)]
pub fn glow(
    d: &mut DrawList,
    ink: Ink,
    c: V2,
    rx: f32,
    ry: f32,
    t_out: f32,
    t_in: f32,
    steps: usize,
    clip: Option<Rect>,
) {
    // Drawn as nested *rings* (plus a centre disc) so every pixel is written once.
    let steps = steps.max(2);
    for i in 0..steps {
        let k = 1.0 - i as f32 / steps as f32;
        let tone = t_out + (t_in - t_out) * i as f32 / (steps - 1) as f32;
        let mut e = if i + 1 < steps {
            ring_poly(c, rx * k, ry * k, (k - 1.0 / steps as f32) / k)
        } else {
            ellipse(c, rx * k, ry * k, 0.0)
        };
        if let Some(r) = clip {
            e = clip_rect(&e, r);
        }
        if e.len() >= 3 && tone > 0.004 {
            d.fill_p(Paint::ht(ink, tone), &e);
        }
    }
}

/// Densify a polyline so no segment is longer than `step`.
pub fn densify(pts: &[V2], step: f32, closed: bool) -> Vec<V2> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(n * 2);
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let k = ((a.dist(b) / step).ceil() as usize).max(1);
        for j in 0..k {
            out.push(a.lerp(b, j as f32 / k as f32));
        }
    }
    if !closed {
        out.push(pts[n - 1]);
    }
    out
}

/// Nudge a closed outline along its normals with smooth noise: the slightly hand-cut edge
/// of a real stencil/master. `amp` in reference units.
pub fn wobble(poly: &[V2], amp: f32, seed: u32) -> Vec<V2> {
    let p = densify(poly, 6.0, true);
    let n = p.len();
    if n < 3 {
        return p;
    }
    let ph = [hash01(seed, 1) * TAU, hash01(seed, 2) * TAU, hash01(seed, 3) * TAU];
    let mut perim = 0.0;
    (0..n)
        .map(|i| {
            let prev = p[(i + n - 1) % n];
            let next = p[(i + 1) % n];
            perim += p[i].dist(prev);
            let nrm = (next - prev).norm().perp();
            let s = perim * 0.045;
            let k = 0.55 * (s + ph[0]).sin() + 0.3 * (s * 2.3 + ph[1]).sin() + 0.15 * (s * 5.1 + ph[2]).sin();
            p[i] + nrm * (k * amp)
        })
        .collect()
}

/// The printed offset shadow under floating paper (tickets, bubbles, cards).
pub fn offset_shadow(d: &mut DrawList, poly: &[V2], off: V2) {
    d.ht(Ink::Blue, 0.34, &translate(poly, off));
}

/// Scatter of little paper knock-outs (flour dust, chalk, glints) inside `r`.
pub fn dust(d: &mut DrawList, r: Rect, n: u32, size: f32, seed: u32) {
    for i in 0..n {
        let p = v2(r.x + hash01(seed, i * 3) * r.w, r.y + hash01(seed, i * 3 + 1) * r.h);
        let s = size * (0.35 + 0.9 * hash01(seed, i * 3 + 2));
        d.knock_p(0.85, Screen::Solid, PLATES_COLOR, &circle(p, s));
    }
}

/// Warm wood fill for a shape (`dark` 0 = pale birch .. 1 = walnut) with grain lines running
/// along `angle` (radians). Grain is clipped to the shape.
pub fn wood(d: &mut DrawList, poly: &[V2], dark: f32, angle: f32, seed: u32) {
    let dark = dark.clamp(0.0, 1.0);
    d.fill(Ink::Yellow, 0.5 + 0.34 * dark, poly);
    d.fill(Ink::Pink, 0.16 + 0.34 * dark, poly);
    if dark > 0.55 {
        d.ht(Ink::Key, (dark - 0.55) * 0.5, poly);
    }
    if poly.len() < 3 {
        return;
    }
    // Grain runs along `angle` across the shape's real extent, clipped analytically (no
    // clip masks: the shapes are convex).
    let dir = V2::from_angle(angle);
    let nrm = dir.perp();
    let (mut d0, mut d1, mut n0, mut n1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for q in poly {
        let (a, b) = (q.dot(dir), q.dot(nrm));
        d0 = d0.min(a);
        d1 = d1.max(a);
        n0 = n0.min(b);
        n1 = n1.max(b);
    }
    let (len, across) = (d1 - d0, n1 - n0);
    let lines = ((across / 8.0).round() as u32).clamp(1, 28);
    let ink = Paint::solid(Ink::Pink, 0.42 + 0.25 * dark);
    for i in 0..lines {
        if lines > 2 && hash01(seed, i) < 0.3 {
            continue;
        }
        let off = n0 + (i as f32 + 0.5) * across / lines as f32;
        let amp = (1.2 + 2.0 * hash01(seed ^ 9, i)).min(across * 0.2);
        let freq = 0.012 + 0.012 * hash01(seed ^ 3, i);
        let ph = hash01(seed ^ 5, i) * TAU;
        let span = len * (0.45 + 0.55 * hash01(seed ^ 7, i));
        let start = d0 + hash01(seed ^ 11, i) * (len - span);
        let segs = ((span / 24.0) as usize).clamp(3, 12);
        let pts: Vec<V2> = (0..=segs)
            .map(|k| {
                let t = start + span * k as f32 / segs as f32;
                dir * t + nrm * (off + (t * freq + ph).sin() * amp)
            })
            .collect();
        for piece in clip_polyline_convex(&pts, poly) {
            d.stroke_p(ink, 1.2 + 0.8 * hash01(seed ^ 13, i), &piece, false);
        }
    }
    // A knot, when there's room for one.
    if across > 28.0 && len > 80.0 && hash01(seed ^ 17, 0) < 0.6 {
        let t = d0 + len * (0.25 + 0.5 * hash01(seed ^ 19, 0));
        let o = n0 + across * (0.3 + 0.4 * hash01(seed ^ 23, 0));
        let c = dir * t + nrm * o;
        let (rx, ry) = (7.0f32.min(across * 0.2), 3.2f32.min(across * 0.1));
        let outer = ellipse(c, rx * 1.7, ry * 1.7, angle);
        if outer.iter().all(|q| crate::geom::point_in_poly(*q, poly)) {
            d.stroke_p(Paint::solid(Ink::Pink, 0.6), 1.4, &ellipse(c, rx, ry, angle), true);
            d.stroke_p(Paint::solid(Ink::Pink, 0.45), 1.2, &outer, true);
        }
    }
}

/// Clip an open polyline to a convex polygon (either winding); returns the inside pieces.
pub fn clip_polyline_convex(pts: &[V2], poly: &[V2]) -> Vec<Vec<V2>> {
    let n = poly.len();
    if n < 3 || pts.len() < 2 {
        return Vec::new();
    }
    let area: f32 = (0..n).map(|i| poly[i].cross(poly[(i + 1) % n])).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut out: Vec<Vec<V2>> = Vec::new();
    let mut cur: Vec<V2> = Vec::new();
    for w in pts.windows(2) {
        let (p0, p1) = (w[0], w[1]);
        let (mut t0, mut t1) = (0.0f32, 1.0f32);
        let mut inside = true;
        for i in 0..n {
            let a = poly[i];
            let e = poly[(i + 1) % n] - a;
            let nrm = v2(-e.y, e.x) * sign;
            let f0 = (p0 - a).dot(nrm);
            let f1 = (p1 - a).dot(nrm);
            if f0 < 0.0 && f1 < 0.0 {
                inside = false;
                break;
            }
            if f0 < 0.0 {
                t0 = t0.max(f0 / (f0 - f1));
            } else if f1 < 0.0 {
                t1 = t1.min(f0 / (f0 - f1));
            }
            if t0 > t1 {
                inside = false;
                break;
            }
        }
        if !inside {
            if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            continue;
        }
        let a = p0.lerp(p1, t0);
        let b = p0.lerp(p1, t1);
        if cur.last().is_none_or(|l| l.dist(a) > 1e-3) {
            if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            cur.push(a);
        }
        cur.push(b);
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

// =========================================================================================
// Buttons
// =========================================================================================

/// Visual state of a printed control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub pressed: bool,
    pub selected: bool,
    pub disabled: bool,
}

/// How far a pill's face drops when pressed (labels follow it).
pub fn button_lift(h: f32, pressed: bool) -> f32 {
    let depth = (h * 0.1).clamp(5.0, 8.0);
    if pressed { depth - 1.5 } else { 0.0 }
}

/// How far a round button's face drops when pressed (icons follow it).
pub fn round_lift(r: f32, pressed: bool) -> f32 {
    let depth = (r * 0.14).clamp(4.0, 7.0);
    if pressed { depth - 1.0 } else { 0.0 }
}

/// A chunky pill button (label drawn by the engine), centred.
pub fn button(d: &mut DrawList, w: f32, h: f32, ink: Ink, pressed: bool) {
    button_ex(d, w, h, ink, Look { pressed, ..Look::default() });
}

/// A pill button with depth: a darker "side" under a flat glossy face that drops when pressed.
pub fn button_ex(d: &mut DrawList, w: f32, h: f32, ink: Ink, look: Look) {
    let depth = (h * 0.1).clamp(5.0, 8.0);
    let r = h * 0.5;
    let lw = (h * 0.052).clamp(3.0, 4.4);
    let lift = button_lift(h, look.pressed);
    let base = rounded_rect(rect(-w * 0.5, -h * 0.5 + depth, w, h), r);
    let face = rounded_rect(rect(-w * 0.5, -h * 0.5 + lift, w, h), r);
    let ink = if look.selected { Ink::Yellow } else { ink };
    // Resting on the surface.
    d.ht(Ink::Blue, 0.34, &rounded_rect(rect(-w * 0.5 + 8.0, -h * 0.5 + depth + 6.0, w - 8.0, h), r));
    if look.disabled {
        d.backing(&face);
        d.fill(Ink::Key, 0.08, &face);
        d.stroke_p(Paint::solid(Ink::Key, 0.5), lw * 0.8, &face, true);
        return;
    }
    d.backing(&base);
    d.fill(ink, 0.95, &base);
    d.fill(Ink::Key, 0.42, &base);
    d.outline(Ink::Key, lw, &base);
    d.backing(&face);
    d.fill(ink, 0.9, &face);
    let y0 = -h * 0.5 + lift;
    // A soft deeper band along the bottom of the face (same ink, flat).
    let band = clip_convex(&rect_poly(rect(-w, y0 + h * 0.66, w * 2.0, h)), &face);
    if band.len() >= 3 {
        d.fill(ink, 1.0, &band);
        d.fill(Ink::Key, 0.08, &band);
    }
    // Gloss.
    let gy = y0 + h * 0.27;
    d.knock_p(
        0.85,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(v2(-w * 0.5 + r * 0.72, gy), v2(-w * 0.5 + r * 1.3, gy), h * 0.075),
    );
    d.knock_p(0.85, Screen::Solid, PLATES_COLOR, &circle(v2(-w * 0.5 + r * 1.62, gy), h * 0.055));
    d.outline(Ink::Key, lw, &face);
}

/// A round icon button (for tools), centred.
pub fn round_button(d: &mut DrawList, r: f32, ink: Ink, selected: bool) {
    round_button_ex(d, r, ink, Look { selected, ..Look::default() });
}

/// A round token button with depth; the face stays light so icons read on it.
pub fn round_button_ex(d: &mut DrawList, r: f32, ink: Ink, look: Look) {
    let depth = (r * 0.14).clamp(4.0, 7.0);
    let lift = round_lift(r, look.pressed);
    let lw = (r * 0.1).clamp(2.8, 4.2);
    if r >= 40.0 {
        contact_shadow(d, v2(0.0, r + depth - 2.0), r * 0.92, r * 0.2);
    } else {
        d.fill(Ink::Blue, 0.3, &ellipse(v2(3.0, depth + 4.0), r + 2.0, r + 1.0, 0.0));
    }
    if look.selected {
        // A printed halo so the choice reads at a glance.
        let halo = circle(v2(0.0, lift + depth * 0.5), r + 8.0);
        d.backing(&halo);
        d.fill(Ink::Pink, 0.95, &halo);
        d.outline(Ink::Key, lw * 0.8, &halo);
    }
    let base = circle(v2(0.0, depth), r);
    let face = circle(v2(0.0, lift), r);
    if look.disabled {
        d.backing(&face);
        d.fill(Ink::Key, 0.08, &face);
        d.stroke_p(Paint::solid(Ink::Key, 0.5), lw * 0.8, &face, true);
        return;
    }
    d.backing(&base);
    d.fill(ink, 0.95, &base);
    d.fill(Ink::Key, 0.42, &base);
    d.outline(Ink::Key, lw, &base);
    d.backing(&face);
    if look.selected {
        d.fill(Ink::Yellow, 0.34, &face);
    } else {
        d.fill(ink, 0.26, &face);
    }
    shade(d, &face, v2(-r * 0.12, -r * 0.16), ink, if look.selected { 0.0 } else { 0.26 }, 0.42);
    if look.selected {
        d.fill(Ink::Yellow, 0.34, &face);
    }
    d.knock_p(
        0.75,
        Screen::Solid,
        PLATES_COLOR,
        &arc_band(v2(0.0, lift), r * 0.78, PI * 1.12, PI * 1.42, r * 0.09),
    );
    d.outline(Ink::Key, lw, &face);
}

/// A thick arc as a closed polygon (glints, rims).
pub fn arc_band(c: V2, r: f32, a0: f32, a1: f32, w: f32) -> Vec<V2> {
    let mut p = arc(c, r + w * 0.5, a0, a1);
    let mut inner = arc(c, r - w * 0.5, a0, a1);
    inner.reverse();
    p.extend(inner);
    chaikin(&p, 1, true)
}

/// An elliptical annulus (outer radii `rx`,`ry`, inner = outer × `k`) as one non-zero polygon.
pub fn ring_poly(c: V2, rx: f32, ry: f32, k: f32) -> Vec<V2> {
    let mut p = ellipse(c, rx, ry, 0.0);
    let first = p[0];
    let mut inner = ellipse(c, rx * k, ry * k, 0.0);
    inner.reverse();
    let inner_first = inner[inner.len() - 1];
    p.push(first);
    p.push(inner_first);
    p.extend(inner);
    p
}

/// A little wooden sign on a stand, standing on a counter (the "Not today" sign). Origin =
/// bottom centre where the feet touch the surface; the paper face is `w`×`h`.
pub fn sign(d: &mut DrawList, w: f32, h: f32, look: Look) {
    let lift = if look.pressed { 3.0 } else { 0.0 };
    let foot = 16.0;
    contact_shadow(d, v2(0.0, -1.0), w * 0.5, 7.0);
    // Easel legs behind the board.
    for sx in [-1.0f32, 1.0] {
        let leg = capsule(v2(sx * w * 0.3, -h * 0.6), v2(sx * w * 0.38, -2.0), 5.0);
        d.backing(&leg);
        wood(d, &leg, 0.55, PI * 0.5, 3);
        d.outline(Ink::Key, DETAIL + 0.6, &leg);
    }
    let y1 = -foot + lift;
    let frame = rounded_rect(rect(-w * 0.5, y1 - h, w, h), 12.0);
    d.backing(&frame);
    wood(d, &frame, 0.5, 0.0, 11);
    d.outline(Ink::Key, INNER + 0.6, &frame);
    let face = rounded_rect(rect(-w * 0.5 + 9.0, y1 - h + 9.0, w - 18.0, h - 18.0), 7.0);
    d.knock(&face);
    d.fill(Ink::Yellow, 0.1, &face);
    ramp(d, Ink::Blue, &face, v2(0.0, y1 - h * 0.35), v2(0.0, y1 - 9.0), 0.0, 0.12, 3);
    d.stroke_p(Paint::solid(Ink::Key, 0.7), DETAIL, &face, true);
    // Two little nails.
    for sx in [-1.0f32, 1.0] {
        let nail = circle(v2(sx * (w * 0.5 - 16.0), y1 - h + 16.0), 2.6);
        d.fill(Ink::Key, 0.9, &nail);
    }
}

// =========================================================================================
// Surfaces and containers
// =========================================================================================

/// A wooden tray with shallow wells at `slots` (tray-local), centred. Tool buttons sit in it.
pub fn tray(d: &mut DrawList, w: f32, h: f32, slots: &[V2], slot_r: f32) {
    let r = (h * 0.34).min(28.0);
    let outer = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), r);
    // Sits on the bench.
    d.ht(Ink::Blue, 0.32, &rounded_rect(rect(-w * 0.5 + 6.0, -h * 0.5 + 9.0, w - 2.0, h), r));
    d.backing(&outer);
    wood(d, &outer, 0.5, 0.0, 29);
    d.outline(Ink::Key, INNER + 0.8, &outer);
    let inner = rounded_rect(rect(-w * 0.5 + 10.0, -h * 0.5 + 10.0, w - 20.0, h - 20.0), (r - 8.0).max(6.0));
    d.fill(Ink::Yellow, 0.36, &inner);
    d.fill(Ink::Pink, 0.1, &inner);
    // Inner lip shadow along the top edge (flat).
    let lip = clip_convex(&rect_poly(rect(-w, -h * 0.5, w * 2.0, 18.0)), &inner);
    if lip.len() >= 3 {
        d.fill(Ink::Pink, 0.26, &lip);
    }
    d.stroke_p(Paint::solid(Ink::Key, 0.75), DETAIL, &inner, true);
    for s in slots {
        let well = circle(*s + v2(0.0, 3.0), slot_r + 5.0);
        d.fill(Ink::Pink, 0.32, &well);
        d.stroke_p(Paint::solid(Ink::Key, 0.45), DETAIL * 0.8, &well, true);
    }
}

/// Tray geometry for `n` round buttons of radius `r`: the tray width and slot centres
/// (tray-local). `split` inserts an extra gap before the last `split` slots (e.g. Undo).
pub fn tray_layout(n: usize, r: f32, split: usize) -> (f32, Vec<V2>) {
    let pitch = 2.0 * r + 14.0;
    let pad = r + 16.0;
    let extra_gap = if split > 0 && n > split { 18.0 } else { 0.0 };
    let span = (n.max(1) - 1) as f32 * pitch + extra_gap;
    let w = span + 2.0 * pad;
    let slots = (0..n)
        .map(|i| {
            let gap = if split > 0 && i >= n - split { extra_gap } else { 0.0 };
            v2(-span * 0.5 + i as f32 * pitch + gap, 0.0)
        })
        .collect();
    (w, slots)
}

/// A paper tab tucked behind a tray's top edge (for "Dress" / "Score" labels). Origin =
/// bottom-left of the tab where it meets the tray.
pub fn tab(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let mut p = vec![v2(0.0, 12.0)];
    p.extend(arc(v2(10.0, -h + 10.0), 10.0, PI, PI * 1.5));
    p.extend(arc(v2(w - 10.0, -h + 10.0), 10.0, PI * 1.5, PI * 2.0));
    p.push(v2(w, 12.0));
    d.backing(&p);
    d.fill(ink, 0.55, &p);
    d.ht(Ink::Yellow, 0.25, &p);
    d.outline(Ink::Key, INNER, &p);
}

/// A round wooden bread board (top view) dusted with flour, radius `r`, with a handle.
pub fn bread_board(d: &mut DrawList, r: f32) {
    let rim = circle(V2::ZERO, r);
    let dir = V2::from_angle(-PI * 0.25);
    let handle = capsule(dir * (r * 0.8), dir * (r * 1.22), r * 0.13);
    d.ht(Ink::Blue, 0.36, &ellipse(v2(6.0, 12.0), r * 1.01, r * 1.0, 0.0));
    d.ht(Ink::Blue, 0.36, &translate(&handle, v2(6.0, 12.0)));
    d.backing(&handle);
    wood(d, &handle, 0.5, -PI * 0.25, 43);
    d.outline(Ink::Key, INNER + 0.8, &handle);
    let hole = circle(dir * (r * 1.13), r * 0.045);
    d.knock(&hole);
    d.stroke_p(Paint::solid(Ink::Key, 1.0), DETAIL, &hole, true);
    d.backing(&rim);
    wood(d, &rim, 0.42, PI * 0.5, 41);
    // Plank seams.
    d.clipped(&rim, |d| {
        for i in [-1.0f32, 0.0, 1.0] {
            let x = i * r * 0.5 + r * 0.08;
            d.stroke_p(Paint::solid(Ink::Key, 0.4), DETAIL * 0.8, &[v2(x, -r), v2(x, r)], false);
        }
    });
    // Bevelled edge.
    let bevel = ring_poly(V2::ZERO, r, r, 0.93);
    d.fill(Ink::Pink, 0.45, &bevel);
    d.stroke_p(Paint::solid(Ink::Key, 0.55), DETAIL, &circle(V2::ZERO, r * 0.93), true);
    // Flour: soft clouds of paper around where the dough sits, and specks.
    for i in 0..9u32 {
        let a = hash01(77, i) * TAU;
        let rr = r * (0.62 + 0.24 * hash01(78, i));
        let blob = crate::geom::blob(V2::from_angle(a) * rr, r * 0.16, r * 0.1, 0.25, i + 3);
        d.knock_p(0.55, Screen::Solid, PLATES_COLOR, &blob);
    }
    for i in 0..70u32 {
        let a = hash01(79, i) * TAU;
        let rr = r * (0.2 + 0.72 * hash01(80, i).sqrt());
        let s = 0.8 + 2.2 * hash01(81, i);
        d.knock_p(0.9, Screen::Solid, PLATES_COLOR, &circle(V2::from_angle(a) * rr, s));
    }
    d.outline(Ink::Key, OUTER, &rim);
}

/// A rimmed baking sheet lined with parchment (treat trays). Centred.
pub fn sheet_pan(d: &mut DrawList, w: f32, h: f32) {
    let outer = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 18.0);
    contact_shadow(d, v2(0.0, h * 0.5), w * 0.5, 12.0);
    d.backing(&outer);
    d.fill(Ink::Blue, 0.34, &outer);
    d.fill(Ink::Key, 0.1, &outer);
    d.outline(Ink::Key, INNER + 0.8, &outer);
    let inner = rounded_rect(rect(-w * 0.5 + 11.0, -h * 0.5 + 11.0, w - 22.0, h - 22.0), 10.0);
    d.fill(Ink::Blue, 0.2, &inner);
    d.knock_p(1.0, Screen::Solid, 1 << Ink::Key.idx(), &inner);
    d.stroke_p(Paint::solid(Ink::Key, 0.7), DETAIL, &inner, true);
    let paper = crate::geom::blob(V2::ZERO, w * 0.45, h * 0.4, 0.015, 5);
    let paper: Vec<V2> = paper
        .iter()
        .map(|p| v2(p.x.clamp(-w * 0.5 + 16.0, w * 0.5 - 16.0), p.y.clamp(-h * 0.5 + 16.0, h * 0.5 - 16.0)))
        .collect();
    d.knock(&paper);
    d.fill(Ink::Yellow, 0.16, &paper);
    d.stroke_p(Paint::solid(Ink::Key, 0.5), DETAIL * 0.8, &paper, true);
    // Crumpled-parchment creases.
    for i in 0..5u32 {
        let x = -w * 0.35 + w * 0.7 * hash01(91, i);
        let y = -h * 0.3 + h * 0.6 * hash01(92, i);
        d.stroke_p(
            Paint::solid(Ink::Key, 0.25),
            1.2,
            &[v2(x, y), v2(x + 18.0, y + 8.0 * (hash01(93, i) - 0.5))],
            false,
        );
    }
}

/// A wire cooling rack (goods rest on it). Centred.
pub fn cooling_rack(d: &mut DrawList, w: f32, h: f32) {
    contact_shadow(d, v2(0.0, h * 0.5 + 6.0), w * 0.52, 12.0);
    for sx in [-1.0f32, 1.0] {
        let foot = rounded_rect(rect(sx * (w * 0.5 - 22.0) - 8.0, h * 0.5 - 2.0, 16.0, 10.0), 4.0);
        d.fill(Ink::Key, 0.85, &foot);
    }
    let frame = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 16.0);
    d.ht(Ink::Blue, 0.16, &translate(&frame, v2(5.0, 8.0)));
    d.clipped(&frame, |d| {
        let n = (w / 26.0) as i32;
        for i in 0..=n {
            let x = -w * 0.5 + i as f32 * w / n as f32;
            d.stroke_p(Paint::solid(Ink::Key, 0.8), 2.2, &[v2(x, -h * 0.5), v2(x, h * 0.5)], false);
        }
        for j in 1..3 {
            let y = -h * 0.5 + j as f32 * h / 3.0;
            d.stroke_p(Paint::solid(Ink::Key, 0.8), 3.0, &[v2(-w * 0.5, y), v2(w * 0.5, y)], false);
        }
    });
    d.outline(Ink::Key, INNER, &frame);
}

/// A cork board in a wooden frame. Centred.
pub fn cork_board(d: &mut DrawList, w: f32, h: f32) {
    let frame = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 16.0);
    d.ht(Ink::Key, 0.3, &translate(&frame, v2(6.0, 9.0)));
    d.backing(&frame);
    wood(d, &frame, 0.62, 0.0, 51);
    d.outline(Ink::Key, OUTER, &frame);
    let cork = rounded_rect(rect(-w * 0.5 + 14.0, -h * 0.5 + 14.0, w - 28.0, h - 28.0), 8.0);
    d.fill(Ink::Yellow, 0.62, &cork);
    d.fill(Ink::Pink, 0.3, &cork);
    d.clipped(&cork, |d| {
        for i in 0..(w * h / 260.0) as u32 {
            let p = v2(-w * 0.5 + hash01(53, i) * w, -h * 0.5 + hash01(54, i) * h);
            let s = 0.8 + 1.5 * hash01(55, i);
            if hash01(56, i) < 0.5 {
                d.fill(Ink::Key, 0.55, &circle(p, s));
            } else {
                d.knock_p(0.6, Screen::Solid, PLATES_COLOR, &circle(p, s));
            }
        }
    });
    ramp(d, Ink::Key, &cork, v2(0.0, -h * 0.5 + 14.0), v2(0.0, -h * 0.5 + 34.0), 0.25, 0.0, 3);
    d.stroke_p(Paint::solid(Ink::Key, 0.8), INNER * 0.8, &cork, true);
}

/// A push pin (seen from the front), point at `c`.
pub fn pushpin(d: &mut DrawList, c: V2, ink: Ink) {
    d.ht(Ink::Key, 0.35, &ellipse(c + v2(4.0, 4.0), 8.0, 4.0, 0.0));
    let head = circle(c + v2(0.0, -2.0), 8.5);
    d.backing(&head);
    d.fill(ink, 0.95, &head);
    d.ht(Ink::Key, 0.3, &ellipse(c + v2(2.0, 1.0), 6.0, 4.0, 0.0));
    d.knock_p(0.85, Screen::Solid, PLATES_COLOR, &circle(c + v2(-2.8, -4.6), 2.4));
    d.outline(Ink::Key, DETAIL + 0.4, &head);
}

/// A paper card pinned to a board, `w`×`h`, centred. `tint` shades pre-orders.
pub fn pinned_card(d: &mut DrawList, w: f32, h: f32, tint: Option<Ink>, seed: u32) {
    let card = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 6.0);
    offset_shadow(d, &card, v2(4.0, 6.0));
    d.backing(&card);
    if let Some(ink) = tint {
        d.fill(ink, 0.2, &card);
        d.ht(ink, 0.2, &rect_poly(rect(-w * 0.5, -h * 0.5, w, 16.0)));
    } else {
        d.fill(Ink::Yellow, 0.08, &card);
    }
    d.outline(Ink::Key, DETAIL + 0.5, &card);
    let ink = [Ink::Pink, Ink::Blue, Ink::Yellow][(seed % 3) as usize];
    pushpin(d, v2(0.0, -h * 0.5 + 8.0), ink);
}

/// A paper tag hanging on a string from `v2(0, 0)`; the tag body hangs `drop` below.
pub fn hanging_tag(d: &mut DrawList, w: f32, h: f32, drop: f32, ink: Ink) {
    let top = drop;
    d.line(Ink::Key, 1.6, &[v2(0.0, -4.0), v2(0.0, top + 9.0)]);
    let mut body = vec![v2(-w * 0.5 + 10.0, top), v2(w * 0.5 - 10.0, top)];
    body.extend(arc(v2(w * 0.5 - 10.0, top + 10.0), 10.0, -PI * 0.5, 0.0));
    body.extend(arc(v2(w * 0.5 - 8.0, top + h - 8.0), 8.0, 0.0, PI * 0.5));
    body.extend(arc(v2(-w * 0.5 + 8.0, top + h - 8.0), 8.0, PI * 0.5, PI));
    body.extend(arc(v2(-w * 0.5 + 10.0, top + 10.0), 10.0, PI, PI * 1.5));
    offset_shadow(d, &body, v2(3.0, 5.0));
    d.backing(&body);
    d.fill(Ink::Yellow, 0.12, &body);
    d.ht(ink, 0.28, &rect_poly(rect(-w * 0.5, top, w, 7.0)));
    d.outline(Ink::Key, DETAIL + 0.4, &body);
    let hole = circle(v2(0.0, top + 9.0), 3.2);
    d.backing(&hole);
    d.stroke_p(Paint::solid(Ink::Key, 1.0), 1.4, &hole, true);
}

/// A linen proofing cloth (couche) laid on a surface, `w`×`h`, centred: bannetons rest on it.
pub fn couche(d: &mut DrawList, w: f32, h: f32) {
    let cloth = wobble(&rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 10.0), 1.6, 9);
    contact_shadow(d, v2(0.0, h * 0.5 - 2.0), w * 0.5, 8.0);
    d.backing(&cloth);
    d.fill(Ink::Yellow, 0.22, &cloth);
    d.fill(Ink::Blue, 0.07, &cloth);
    // Woven stripes along the edges.
    d.clipped(&cloth, |d| {
        for y in [-h * 0.5 + 10.0, h * 0.5 - 14.0] {
            d.fill(Ink::Blue, 0.45, &rect_poly(rect(-w * 0.5, y, w, 3.5)));
        }
    });
    d.outline(Ink::Key, INNER, &cloth);
}

// =========================================================================================
// Paper: tickets, slips, cards, receipts, ribbons, the masthead
// =========================================================================================

/// An order ticket centred on the origin, `w`×`h`, with a zig-zag tear-off bottom.
pub fn ticket(d: &mut DrawList, w: f32, h: f32, header: Ink) {
    let (x0, y0) = (-w * 0.5, -h * 0.5);
    let bottom = y0 + h - 8.0;
    let teeth = ((w / 22.0) as usize).max(2);
    let mut p = vec![v2(x0, y0 + 6.0)];
    p.extend(arc(v2(x0 + 6.0, y0 + 6.0), 6.0, PI, PI * 1.5));
    p.extend(arc(v2(x0 + w - 6.0, y0 + 6.0), 6.0, PI * 1.5, PI * 2.0));
    p.push(v2(x0 + w, bottom));
    p.extend(zigzag(v2(x0 + w, bottom), v2(x0, bottom), teeth, -8.0).into_iter().skip(1));
    offset_shadow(d, &p, v2(6.0, 8.0));
    d.backing(&p);
    d.fill(Ink::Yellow, 0.07, &p);
    let band = (h * 0.1).clamp(18.0, 30.0);
    d.clipped(&p, |d| {
        d.ht(header, 0.5, &rect_poly(rect(x0, y0, w, band)));
        d.fill(header, 0.25, &rect_poly(rect(x0, y0, w, band)));
    });
    // Perforation under the header.
    dashed(d, v2(x0 + 10.0, y0 + band + 6.0), v2(x0 + w - 10.0, y0 + band + 6.0), 7.0, 5.0, 1.6, 0.6);
    d.outline(Ink::Key, INNER, &p);
    let hole = circle(v2(0.0, y0 + band * 0.5), 5.0);
    d.backing(&hole);
    d.stroke_p(Paint::solid(Ink::Key, 1.0), 2.2, &hole, true);
}

/// A dashed key line from `a` to `b`.
pub fn dashed(d: &mut DrawList, a: V2, b: V2, dash: f32, gap: f32, width: f32, tone: f32) {
    let len = a.dist(b);
    if len < 1.0 {
        return;
    }
    let dir = (b - a) / len;
    let mut t = 0.0;
    while t < len {
        let e = (t + dash).min(len);
        d.stroke_p(Paint::solid(Ink::Key, tone), width, &[a + dir * t, a + dir * e], false);
        t += dash + gap;
    }
}

/// A little die-cut paper sticker (floating words like "+12" and "Loved it!"), centred.
pub fn sticker(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let body = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), h * 0.5);
    offset_shadow(d, &body, v2(3.0, 5.0));
    d.backing(&body);
    d.fill(ink, 0.3, &body);
    let inner = rounded_rect(rect(-w * 0.5 + 5.0, -h * 0.5 + 5.0, w - 10.0, h - 10.0), h * 0.5 - 5.0);
    d.knock_color(&inner);
    d.fill(Ink::Yellow, 0.08, &inner);
    d.outline(Ink::Key, DETAIL + 0.8, &body);
}

/// A toast slip: a paper stub with a coloured tab and a perforation, `w`×`h`, centred.
pub fn slip(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let body = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 12.0);
    offset_shadow(d, &body, v2(5.0, 7.0));
    d.backing(&body);
    d.fill(Ink::Yellow, 0.08, &body);
    let stub_w = h * 0.95;
    d.clipped(&body, |d| {
        let stub = rect_poly(rect(-w * 0.5, -h * 0.5, stub_w, h));
        d.fill(ink, 0.8, &stub);
        d.ht(Ink::Yellow, 0.3, &stub);
    });
    dashed(
        d,
        v2(-w * 0.5 + stub_w, -h * 0.5 + 6.0),
        v2(-w * 0.5 + stub_w, h * 0.5 - 6.0),
        5.0,
        4.0,
        1.8,
        0.8,
    );
    let c = v2(-w * 0.5 + stub_w * 0.5, 0.0);
    let star = soft_star(c, h * 0.26, h * 0.12, 5, 0.0);
    d.knock(&star);
    d.fill(Ink::Yellow, 0.95, &star);
    d.outline(Ink::Key, DETAIL, &star);
    // Half-moon notches where the stub tears off.
    for sy in [-1.0f32, 1.0] {
        let n = circle(v2(-w * 0.5 + stub_w, sy * h * 0.5), 5.0);
        d.knock(&n);
        d.backing(&n);
    }
    d.outline(Ink::Key, INNER, &body);
    for sy in [-1.0f32, 1.0] {
        let a = if sy < 0.0 {
            arc(v2(-w * 0.5 + stub_w, -h * 0.5), 5.0, 0.0, PI)
        } else {
            arc(v2(-w * 0.5 + stub_w, h * 0.5), 5.0, PI, TAU)
        };
        d.line(Ink::Key, INNER, &a);
    }
}

/// A ribbon banner (headline strip) `w`×`h`, centred, with folded tails in `ink`.
pub fn ribbon(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let tail = h * 0.9;
    let drop = h * 0.28;
    for sx in [-1.0f32, 1.0] {
        let x0 = sx * (w * 0.5 - tail * 0.35);
        let x1 = sx * (w * 0.5 + tail);
        let t = vec![
            v2(x0, -h * 0.5 + drop),
            v2(x1, -h * 0.5 + drop),
            v2(x1 - sx * tail * 0.32, drop * 0.5),
            v2(x1, h * 0.5 + drop),
            v2(x0, h * 0.5 + drop),
        ];
        d.backing(&t);
        d.fill(ink, 0.85, &t);
        d.ht(Ink::Key, 0.3, &t);
        d.outline(Ink::Key, INNER, &t);
        // The fold that tucks under the band.
        let fold = vec![v2(sx * w * 0.5, h * 0.5), v2(x0, h * 0.5 + drop), v2(sx * w * 0.5, h * 0.5 + drop)];
        d.backing(&fold);
        d.fill(ink, 0.9, &fold);
        d.ht(Ink::Key, 0.6, &fold);
        d.outline(Ink::Key, DETAIL + 0.5, &fold);
    }
    let band = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 4.0);
    d.backing(&band);
    d.fill(Ink::Yellow, 0.1, &band);
    d.fill(ink, 0.12, &band);
    d.stroke_p(
        Paint::solid(ink, 0.9),
        3.0,
        &[v2(-w * 0.5 + 8.0, -h * 0.5 + 6.0), v2(w * 0.5 - 8.0, -h * 0.5 + 6.0)],
        false,
    );
    d.stroke_p(
        Paint::solid(ink, 0.9),
        3.0,
        &[v2(-w * 0.5 + 8.0, h * 0.5 - 6.0), v2(w * 0.5 - 8.0, h * 0.5 - 6.0)],
        false,
    );
    d.outline(Ink::Key, INNER, &band);
}

/// The masthead band across the top of every screen: `w`×`h` from y = 0, extended `above`
/// units upward to fill a notch. A scalloped awning edge hangs from its bottom.
pub fn masthead(d: &mut DrawList, w: f32, h: f32, above: f32) {
    let band = rect_poly(rect(-40.0, -above - 40.0, w + 80.0, h + above + 40.0));
    d.backing(&band);
    d.fill(Ink::Yellow, 0.72, &band);
    ramp(d, Ink::Pink, &band, v2(0.0, h * 0.35), v2(0.0, h), 0.0, 0.22, 5);
    // Awning scallops.
    let n = 18;
    let sw = w / n as f32;
    let depth = 13.0;
    let mut edge = vec![v2(-40.0, h - 2.0)];
    for i in 0..n {
        let x = i as f32 * sw;
        edge.extend(
            arc(v2(x + sw * 0.5, h - 2.0), sw * 0.5, 0.0, PI)
                .into_iter()
                .map(|p| v2(p.x, h - 2.0 + (p.y - (h - 2.0)) * (depth / (sw * 0.5)))),
        );
    }
    edge.push(v2(w + 40.0, h - 2.0));
    edge.push(v2(w + 40.0, h - 12.0));
    edge.push(v2(-40.0, h - 12.0));
    edge.reverse();
    d.backing(&edge);
    d.fill(Ink::Pink, 0.85, &edge);
    // Every other scallop is paper: knock its own shape (no clip masks needed).
    for i in 0..n {
        if i % 2 == 1 {
            let x = i as f32 * sw;
            let mut sc = vec![v2(x, h - 12.0), v2(x + sw, h - 12.0)];
            sc.extend(
                arc(v2(x + sw * 0.5, h - 2.0), sw * 0.5, 0.0, PI)
                    .into_iter()
                    .map(|p| v2(p.x, h - 2.0 + (p.y - (h - 2.0)) * (depth / (sw * 0.5)))),
            );
            d.knock_color(&sc);
        }
    }
    d.outline(Ink::Key, INNER, &edge);
    d.line(Ink::Key, INNER + 0.8, &[v2(-40.0, h - 12.0), v2(w + 40.0, h - 12.0)]);
}

/// A rounded chip for a counter on the masthead (coins, level). Centred.
pub fn chip(d: &mut DrawList, w: f32, h: f32) {
    let c = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), h * 0.5);
    d.backing(&c);
    d.fill(Ink::Yellow, 0.1, &c);
    ramp(d, Ink::Key, &c, v2(0.0, -h * 0.5), v2(0.0, -h * 0.1), 0.14, 0.0, 3);
    d.outline(Ink::Key, DETAIL + 0.8, &c);
}

/// A rosette badge (level) centred on `c`.
pub fn rosette(d: &mut DrawList, c: V2, r: f32) {
    let s = soft_star(c, r, r * 0.78, 12, 0.0);
    d.backing(&s);
    d.fill(Ink::Yellow, 0.95, &s);
    d.ht(Ink::Pink, 0.3, &s);
    d.outline(Ink::Key, DETAIL + 0.8, &s);
    let inner = circle(c, r * 0.64);
    d.knock_color(&inner);
    d.fill(Ink::Yellow, 0.45, &inner);
    d.stroke_p(Paint::solid(Ink::Key, 0.8), DETAIL, &inner, true);
}

/// A progress bar `w`×`h` (top-left at the origin), filled `frac`.
pub fn meter(d: &mut DrawList, w: f32, h: f32, frac: f32, ink: Ink) {
    let track = rounded_rect(rect(0.0, 0.0, w, h), h * 0.5);
    d.backing(&track);
    d.ht(Ink::Key, 0.12, &track);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.01 {
        let fill = rounded_rect(rect(1.5, 1.5, ((w - 3.0) * f).max(h - 3.0), h - 3.0), (h - 3.0) * 0.5);
        d.fill(ink, 0.92, &fill);
        d.knock_p(
            0.6,
            Screen::Solid,
            PLATES_COLOR,
            &capsule(v2(h * 0.5, h * 0.32), v2(((w - 3.0) * f - h * 0.3).max(h * 0.5), h * 0.32), h * 0.1),
        );
    }
    d.outline(Ink::Key, DETAIL + 0.6, &track);
}

/// Grandma's note: an index card with a header band, ruled lines and taped corners. Centred.
pub fn note_card(d: &mut DrawList, w: f32, h: f32) {
    let card = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), 10.0);
    offset_shadow(d, &card, v2(8.0, 11.0));
    d.backing(&card);
    d.fill(Ink::Yellow, 0.1, &card);
    let head = 92.0;
    d.clipped(&card, |d| {
        let band = rect_poly(rect(-w * 0.5, -h * 0.5, w, head));
        d.fill(Ink::Pink, 0.5, &band);
        d.ht(Ink::Yellow, 0.35, &band);
        // Tiny hearts along the band.
        for i in 0..((w / 46.0) as i32) {
            let c = v2(-w * 0.5 + 23.0 + i as f32 * 46.0, -h * 0.5 + head - 10.0);
            d.knock_color(&heart(c, 9.0));
        }
    });
    d.line(Ink::Key, DETAIL + 0.6, &[v2(-w * 0.5, -h * 0.5 + head), v2(w * 0.5, -h * 0.5 + head)]);
    // Ruled lines and a margin.
    let mut y = -h * 0.5 + head + 44.0;
    while y < h * 0.5 - 20.0 {
        d.stroke_p(
            Paint::solid(Ink::Blue, 0.5),
            1.4,
            &[v2(-w * 0.5 + 16.0, y), v2(w * 0.5 - 16.0, y)],
            false,
        );
        y += 40.0;
    }
    d.stroke_p(
        Paint::solid(Ink::Pink, 0.75),
        1.6,
        &[v2(-w * 0.5 + 52.0, -h * 0.5 + head), v2(-w * 0.5 + 52.0, h * 0.5)],
        false,
    );
    d.outline(Ink::Key, INNER, &card);
    for (sx, rot) in [(-1.0f32, -0.55f32), (1.0, 0.5)] {
        d.with(crate::geom::Xf::at(v2(sx * (w * 0.5 - 18.0), -h * 0.5 + 6.0)).rotated(rot), |d| {
            tape(d, 96.0, 30.0, Ink::Yellow)
        });
    }
}

/// A small portrait of Grandma (bun, round specs, rosy cheeks) centred on `c`, radius `r`.
pub fn grandma(d: &mut DrawList, c: V2, r: f32) {
    let frame = circle(c, r);
    d.backing(&frame);
    d.fill(Ink::Blue, 0.22, &frame);
    d.clipped(&frame, |d| {
        // Cardigan.
        let body = ellipse(c + v2(0.0, r * 1.02), r * 0.78, r * 0.52, 0.0);
        d.backing(&body);
        d.fill(Ink::Pink, 0.6, &body);
        d.ht(Ink::Key, 0.12, &body);
        d.outline(Ink::Key, DETAIL + 0.6, &body);
        let collar = vec![c + v2(-r * 0.2, r * 0.55), c + v2(0.0, r * 0.74), c + v2(r * 0.2, r * 0.55)];
        d.line(Ink::Key, DETAIL, &collar);
        // Bun and hair.
        let bun = circle(c + v2(0.0, -r * 0.58), r * 0.25);
        d.backing(&bun);
        d.fill(Ink::Blue, 0.28, &bun);
        d.ht(Ink::Key, 0.25, &bun);
        d.outline(Ink::Key, DETAIL + 0.6, &bun);
        let head = ellipse(c + v2(0.0, -r * 0.08), r * 0.46, r * 0.44, 0.0);
        d.backing(&head);
        d.fill(Ink::Yellow, 0.28, &head);
        d.fill(Ink::Pink, 0.14, &head);
        let hair = {
            let mut p = arc(c + v2(0.0, -r * 0.08), r * 0.47, PI * 1.02, PI * 1.98);
            p.extend(quad_bezier(
                c + v2(r * 0.46, -r * 0.14),
                c + v2(0.0, -r * 0.36),
                c + v2(-r * 0.46, -r * 0.14),
                8,
            ));
            p
        };
        d.backing(&hair);
        d.fill(Ink::Blue, 0.28, &hair);
        d.ht(Ink::Key, 0.25, &hair);
        d.outline(Ink::Key, DETAIL + 0.6, &hair);
        d.outline(Ink::Key, DETAIL + 0.6, &head);
        // Specs + face.
        for sx in [-1.0f32, 1.0] {
            let lens = circle(c + v2(sx * r * 0.17, -r * 0.03), r * 0.12);
            d.knock_color(&lens);
            d.stroke_p(Paint::solid(Ink::Key, 1.0), DETAIL * 0.85, &lens, true);
            d.line(
                Ink::Key,
                DETAIL * 1.1,
                &arc(c + v2(sx * r * 0.17, -r * 0.0), r * 0.05, PI * 1.15, PI * 1.85),
            );
            d.ht_add(Ink::Pink, 0.6, &ellipse(c + v2(sx * r * 0.3, r * 0.12), r * 0.08, r * 0.05, 0.0));
        }
        d.line(Ink::Key, DETAIL * 0.85, &[c + v2(-r * 0.05, -r * 0.03), c + v2(r * 0.05, -r * 0.03)]);
        d.line(
            Ink::Key,
            DETAIL,
            &quad_bezier(c + v2(-r * 0.08, r * 0.15), c + v2(0.0, r * 0.22), c + v2(r * 0.08, r * 0.15), 6),
        );
    });
    d.outline(Ink::Key, INNER, &frame);
}

/// A till receipt `w`×`h` with torn zig-zag ends, centred.
pub fn receipt(d: &mut DrawList, w: f32, h: f32) {
    let teeth = ((w / 20.0) as usize).max(2);
    let (x0, y0) = (-w * 0.5, -h * 0.5);
    let mut p = zigzag(v2(x0, y0 + 8.0), v2(x0 + w, y0 + 8.0), teeth, 8.0);
    p.extend(zigzag(v2(x0 + w, y0 + h - 8.0), v2(x0, y0 + h - 8.0), teeth, 8.0));
    offset_shadow(d, &p, v2(7.0, 9.0));
    d.backing(&p);
    d.fill(Ink::Yellow, 0.06, &p);
    ramp(d, Ink::Blue, &p, v2(0.0, y0 + h * 0.6), v2(0.0, y0 + h), 0.0, 0.1, 3);
    d.outline(Ink::Key, DETAIL + 0.8, &p);
}

/// A photo print with a white border and two strips of tape; the picture area is inset.
pub fn photo(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let card = rect_poly(rect(-w * 0.5, -h * 0.5, w, h));
    offset_shadow(d, &card, v2(5.0, 7.0));
    d.backing(&card);
    d.fill(Ink::Yellow, 0.06, &card);
    let pic = rect_poly(rect(-w * 0.5 + 10.0, -h * 0.5 + 10.0, w - 20.0, h - 42.0));
    d.fill(ink, 0.2, &pic);
    ramp(d, ink, &pic, v2(0.0, -h * 0.5), v2(0.0, h * 0.5 - 32.0), 0.0, 0.3, 4);
    d.stroke_p(Paint::solid(Ink::Key, 0.6), DETAIL * 0.8, &pic, true);
    d.outline(Ink::Key, DETAIL + 0.5, &card);
    for (sx, rot) in [(-1.0f32, -0.7f32), (1.0, 0.7)] {
        d.with(crate::geom::Xf::at(v2(sx * (w * 0.5 - 6.0), -h * 0.5 + 6.0)).rotated(rot), |d| {
            tape(d, 58.0, 20.0, Ink::Yellow)
        });
    }
}

/// A catalog listing card `w`×`h` (top-left at the origin) with an icon well on the left.
pub fn listing(d: &mut DrawList, w: f32, h: f32, affordable: bool) {
    let card = rounded_rect(rect(0.0, 0.0, w, h), 16.0);
    offset_shadow(d, &card, v2(4.0, 6.0));
    d.backing(&card);
    d.fill(Ink::Yellow, if affordable { 0.12 } else { 0.05 }, &card);
    let well = circle(v2(h * 0.56, h * 0.5), h * 0.36);
    d.fill(Ink::Blue, 0.16, &well);
    d.stroke_p(Paint::solid(Ink::Key, 0.6), DETAIL, &well, true);
    dashed(d, v2(h * 1.08, h * 0.54), v2(w - 180.0, h * 0.54), 3.0, 5.0, 1.4, 0.35);
    d.outline(Ink::Key, DETAIL + 1.0, &card);
}

// =========================================================================================
// Stamps, tape, bubbles
// =========================================================================================

/// A rubber stamp mark: ring, stars for the grade, a wheat sprig, distressed ink.
pub fn stamp(d: &mut DrawList, r: f32, stars: u32, ink: Ink, seed: u32) {
    let paint = Paint::solid(ink, 1.0);
    let ring = wobble(&circle(V2::ZERO, r), r * 0.02, seed);
    let inner = circle(V2::ZERO, r * 0.8);
    d.fill_eo(paint, &[ring.clone(), inner]);
    d.stroke_p(paint, r * 0.03, &circle(V2::ZERO, r * 0.74), true);
    let n = stars.clamp(1, 3);
    for i in 0..n {
        let a = -PI * 0.5 + (i as f32 - (n - 1) as f32 * 0.5) * 0.52;
        let c = V2::from_angle(a) * (r * 0.52);
        d.fill_p(paint, &soft_star(c, r * 0.15, r * 0.066, 5, 0.0));
    }
    // A banner across the middle with a wheat sprig above it.
    let band = rounded_rect(rect(-r * 0.66, r * 0.06, r * 1.32, r * 0.22), r * 0.04);
    d.fill_p(paint, &band);
    d.knock_p(1.0, Screen::Solid, 0b1111, &rect_poly(rect(-r * 0.5, r * 0.155, r * 1.0, r * 0.03)));
    let stem_top = v2(0.0, -r * 0.22);
    d.stroke_p(paint, r * 0.035, &[v2(0.0, r * 0.04), stem_top], false);
    for k in 0..3 {
        let y = -r * 0.03 - k as f32 * r * 0.075;
        for sx in [-1.0f32, 1.0] {
            d.fill_p(paint, &ellipse(v2(sx * r * 0.045, y), r * 0.03, r * 0.055, sx * 0.55));
        }
    }
    for sx in [-1.0f32, 1.0] {
        d.fill_p(paint, &circle(v2(sx * r * 0.6, -r * 0.04), r * 0.04));
    }
    // Distress: uneven ink — a couple of pale patches and speckles of missing ink.
    for i in 0..3u32 {
        let a = hash01(seed ^ 0x51, i) * TAU;
        let c = V2::from_angle(a) * (r * (0.35 + 0.55 * hash01(seed ^ 0x52, i)));
        let blob = crate::geom::blob(c, r * 0.18, r * 0.1, 0.3, seed + i);
        d.knock_p(0.3, Screen::Halftone, 0b1111, &blob);
    }
    for i in 0..(r * 1.1) as u32 {
        let a = hash01(seed, i * 2) * TAU;
        let rr = hash01(seed, i * 2 + 1).sqrt() * r;
        let s = 0.6 + 2.2 * hash01(seed ^ 7, i).powf(2.0);
        d.knock_p(0.95, Screen::Solid, 0b1111, &circle(V2::from_angle(a) * rr, s));
    }
}

/// Masking tape strip (labels), centred. Torn, slightly translucent-looking ends.
pub fn tape(d: &mut DrawList, w: f32, h: f32, ink: Ink) {
    let teeth = ((h / 7.0) as usize).max(2);
    let mut p = Vec::new();
    for (i, q) in zigzag(v2(-w * 0.5, -h * 0.5), v2(-w * 0.5, h * 0.5), teeth, -2.5).into_iter().enumerate() {
        p.push(q + v2(hash01(i as u32, 71) * 2.0 - 1.0, 0.0));
    }
    for (i, q) in zigzag(v2(w * 0.5, h * 0.5), v2(w * 0.5, -h * 0.5), teeth, -2.5).into_iter().enumerate() {
        p.push(q + v2(hash01(i as u32, 73) * 2.0 - 1.0, 0.0));
    }
    d.backing(&p);
    d.fill(ink, 0.42, &p);
    // Paper fibres.
    for i in 0..((w / 14.0) as u32) {
        let x = -w * 0.5 + (i as f32 + hash01(i, 77)) * 14.0;
        d.stroke_p(Paint::solid(ink, 0.6), 0.9, &[v2(x, -h * 0.5 + 2.0), v2(x + 3.0, h * 0.5 - 2.0)], false);
    }
    d.fill_p(Paint::solid(ink, 0.55).add(), &rect_poly(rect(-w * 0.5, -h * 0.5, w, 2.0)));
}

/// Speech bubble centred at origin with a curved tail pointing to `tail` (relative).
pub fn bubble(d: &mut DrawList, w: f32, h: f32, tail: V2) {
    let body = bubble_shape(w, h, tail);
    offset_shadow(d, &body, v2(6.0, 8.0));
    d.backing(&body);
    d.fill(Ink::Yellow, 0.06, &body);
    d.outline(Ink::Key, OUTER - 0.5, &body);
}

/// The bubble outline with its tail merged into the bottom edge (no seam).
pub fn bubble_shape(w: f32, h: f32, tail: V2) -> Vec<V2> {
    let r = (h * 0.22).min(40.0);
    let (x0, y0, x1, y1) = (-w * 0.5, -h * 0.5, w * 0.5, h * 0.5);
    let bx = tail.x.clamp(x0 + r + 34.0, x1 - r - 34.0) * 0.6;
    let half = (w * 0.07).clamp(16.0, 30.0);
    let mut p = Vec::new();
    p.extend(arc(v2(x1 - r, y0 + r), r, -PI * 0.5, 0.0));
    p.extend(arc(v2(x1 - r, y1 - r), r, 0.0, PI * 0.5));
    // Tail: leave the bottom edge, curve out to the tip, curve back.
    let a = v2(bx + half, y1);
    let b = v2(bx - half, y1);
    p.push(a);
    p.extend(quad_bezier(a, v2(bx + half * 0.2, y1 + (tail.y - y1) * 0.35), tail, 8).into_iter().skip(1));
    p.extend(quad_bezier(tail, v2(bx - half * 0.9, y1 + (tail.y - y1) * 0.3), b, 8).into_iter().skip(1));
    p.extend(arc(v2(x0 + r, y1 - r), r, PI * 0.5, PI));
    p.extend(arc(v2(x0 + r, y0 + r), r, PI, PI * 1.5));
    p
}

// =========================================================================================
// Small goods and symbols
// =========================================================================================

pub fn heart_icon(d: &mut DrawList, c: V2, s: f32) {
    let h = heart(c, s);
    d.backing(&h);
    d.fill(Ink::Pink, 0.95, &h);
    d.ht(Ink::Key, 0.18, &heart(c + v2(s * 0.08, s * 0.1), s * 0.8));
    d.fill(Ink::Pink, 0.95, &heart(c + v2(-s * 0.04, -s * 0.04), s * 0.78));
    d.knock_p(
        0.85,
        Screen::Solid,
        PLATES_COLOR,
        &ellipse(c + v2(-s * 0.2, -s * 0.12), s * 0.09, s * 0.06, -0.6),
    );
    d.outline(Ink::Key, (s * 0.08).max(2.0), &h);
}

pub fn coin(d: &mut DrawList, c: V2, r: f32) {
    let rim = circle(c, r);
    d.backing(&rim);
    d.fill(Ink::Yellow, 1.0, &rim);
    d.fill(Ink::Pink, 0.3, &rim);
    let face = circle(c, r * 0.72);
    d.fill(Ink::Pink, 0.1, &face);
    d.knock_p(0.35, Screen::Solid, 0b0001, &face);
    d.stroke_p(Paint::solid(Ink::Key, 0.85), (r * 0.07).max(1.4), &face, true);
    d.fill(Ink::Key, 0.85, &soft_star(c, r * 0.38, r * 0.17, 5, 0.0));
    d.knock_p(0.8, Screen::Solid, PLATES_COLOR, &arc_band(c, r * 0.86, PI * 1.1, PI * 1.45, r * 0.1));
    d.outline(Ink::Key, (r * 0.12).max(2.0), &rim);
}

/// An open flour sack (origin = bottom centre) showing the flour inside: white, speckled
/// whole wheat, or grey-brown rye, with a stamped label. About 120 wide, 160 tall.
pub fn flour_bag(d: &mut DrawList, flour: Flour) {
    contact_shadow(d, v2(0.0, 1.0), 64.0, 9.0);
    // A slumped burlap sack: wide soft bottom, gently pinched below the folded rim.
    let sack = chaikin(
        &[
            v2(-52.0, 0.0),
            v2(-60.0, -30.0),
            v2(-54.0, -78.0),
            v2(-46.0, -104.0),
            v2(46.0, -104.0),
            v2(54.0, -78.0),
            v2(60.0, -30.0),
            v2(52.0, 0.0),
        ],
        3,
        true,
    );
    d.backing(&sack);
    d.fill(Ink::Yellow, 0.46, &sack);
    d.fill(Ink::Pink, 0.16, &sack);
    let body = crate::geom::ellipse(v2(0.0, -52.0), 58.0, 56.0, 0.0);
    let lit = clip_convex(&translate(&body, v2(-14.0, -8.0)), &body);
    // Flat shading: the far side of the sack a step darker.
    d.clipped(&sack, |d| {
        d.fill(Ink::Pink, 0.34, &rect_poly(rect(-70.0, -120.0, 140.0, 130.0)));
        d.fill(Ink::Pink, 0.16, &lit);
        // Burlap weave: sparse stitched lines.
        for i in 0..7 {
            let x = -45.0 + i as f32 * 15.0;
            dashed(d, v2(x, -98.0), v2(x * 1.08, -6.0), 5.0, 6.0, 1.1, 0.22);
        }
        // Folds pooling at the foot.
        for (x, len) in [(-30.0f32, 22.0f32), (6.0, 16.0), (34.0, 20.0)] {
            d.stroke_p(
                Paint::solid(Ink::Key, 0.4),
                1.6,
                &quad_bezier(v2(x, -2.0), v2(x + 4.0, -len * 0.5), v2(x + 2.0, -len), 5),
                false,
            );
        }
    });
    d.outline(Ink::Key, OUTER - 0.5, &sack);
    // The flour heaped in the open mouth.
    let mound = {
        let mut p = quad_bezier(v2(-44.0, -106.0), v2(0.0, -154.0), v2(44.0, -106.0), 12);
        p.push(v2(0.0, -100.0));
        p
    };
    d.backing(&mound);
    match flour {
        Flour::White => {
            d.fill(Ink::Blue, 0.08, &mound);
        }
        Flour::Wheat => {
            d.fill(Ink::Yellow, 0.6, &mound);
            d.fill(Ink::Pink, 0.26, &mound);
            for i in 0..18u32 {
                let p = v2(-34.0 + 68.0 * hash01(61, i), -109.0 - 24.0 * hash01(62, i));
                d.fill(Ink::Key, 0.75, &circle(p, 1.3));
            }
        }
        Flour::Rye => {
            d.fill(Ink::Pink, 0.32, &mound);
            d.fill(Ink::Blue, 0.3, &mound);
            d.fill(Ink::Key, 0.14, &mound);
        }
    }
    d.knock_p(0.7, Screen::Solid, PLATES_COLOR, &crate::geom::ellipse(v2(-12.0, -128.0), 12.0, 5.0, -0.3));
    d.outline(Ink::Key, INNER, &mound);
    // A little wooden scoop resting in it.
    let scoop = capsule(v2(14.0, -124.0), v2(42.0, -150.0), 5.0);
    d.backing(&scoop);
    wood(d, &scoop, 0.4, -0.7, 5);
    d.outline(Ink::Key, DETAIL + 0.6, &scoop);
    // The rim folded down over the mouth: a wavy band, wider than the body.
    let mut rim = quad_bezier(v2(-56.0, -114.0), v2(0.0, -104.0), v2(56.0, -114.0), 10);
    rim.extend(quad_bezier(v2(58.0, -92.0), v2(0.0, -84.0), v2(-58.0, -92.0), 10));
    let rim = chaikin(&rim, 2, true);
    d.backing(&rim);
    d.fill(Ink::Yellow, 0.5, &rim);
    d.fill(Ink::Pink, 0.26, &rim);
    d.outline(Ink::Key, INNER, &rim);
    d.stroke_p(
        Paint::solid(Ink::Key, 0.45),
        1.2,
        &quad_bezier(v2(-50.0, -100.0), v2(0.0, -92.0), v2(50.0, -100.0), 10),
        false,
    );
    // Stamped label: a roundel in the flour's colour with a wheat sprig.
    let lc = v2(0.0, -50.0);
    let label = circle(lc, 26.0);
    d.knock(&label);
    let (lab_ink, tone) = match flour {
        Flour::White => (Ink::Blue, 0.55),
        Flour::Wheat => (Ink::Yellow, 0.95),
        Flour::Rye => (Ink::Pink, 0.7),
    };
    d.fill(lab_ink, tone, &label);
    d.stroke_p(Paint::solid(Ink::Key, 0.9), DETAIL, &circle(lc, 20.5), true);
    d.outline(Ink::Key, DETAIL + 0.8, &label);
    d.line(Ink::Key, 2.2, &[lc + v2(0.0, 14.0), lc + v2(0.0, -14.0)]);
    for i in 0..4 {
        let y = 7.0 - i as f32 * 6.0;
        for sx in [-1.0f32, 1.0] {
            d.fill(Ink::Key, 1.0, &ellipse(lc + v2(sx * 4.4, y), 2.8, 4.8, sx * 0.5));
        }
    }
    // Flour dust at the foot.
    for i in 0..7u32 {
        let p = v2(-60.0 + hash01(3, i) * 120.0, -5.0 + hash01(4, i) * 9.0);
        d.knock_p(0.75, Screen::Solid, PLATES_COLOR, &circle(p, 1.8 + 2.6 * hash01(5, i)));
    }
}

/// A banneton proofing basket, top view, with the dough ball tucked in when `full`.
pub fn banneton(d: &mut DrawList, shape: Shape, r: f32, full: bool) {
    let (rx, ry) = shape.radii();
    let (ax, ay) = (rx * r, ry * r);
    let outer = ellipse(V2::ZERO, ax, ay, 0.0);
    contact_shadow(d, v2(0.0, ay * 0.82), ax * 1.02, ay * 0.3);
    d.backing(&outer);
    d.fill(Ink::Yellow, 0.78, &outer);
    d.fill(Ink::Pink, 0.42, &outer);
    // Coiled cane: rings with little ticks.
    let rings = 5;
    for i in 0..rings {
        let k = 1.0 - (i as f32 + 0.5) * 0.06;
        let e = ellipse(V2::ZERO, ax * k, ay * k, 0.0);
        d.stroke_p(Paint::solid(Ink::Key, 0.55), (r * 0.035).max(1.0), &e, true);
    }
    for i in 0..40u32 {
        let a = TAU * i as f32 / 40.0;
        let p0 = v2(a.cos() * ax * 0.99, a.sin() * ay * 0.99);
        let p1 = v2(a.cos() * ax * 0.73, a.sin() * ay * 0.73);
        if i % 2 == 0 {
            d.stroke_p(Paint::solid(Ink::Pink, 0.5), (r * 0.02).max(0.8), &[p0, p1], false);
        }
    }
    d.outline(Ink::Key, (r * 0.09).clamp(2.2, OUTER), &outer);
    let inner = ellipse(V2::ZERO, ax * 0.7, ay * 0.7, 0.0);
    if full {
        let dough = crate::geom::blob(v2(0.0, -ay * 0.02), ax * 0.74, ay * 0.74, 0.02, 3);
        d.knock(&dough);
        d.fill(Ink::Yellow, 0.26, &dough);
        // The basket's spiral printed in flour on the dough.
        for i in 1..5 {
            let k = 0.74 * i as f32 / 5.0;
            d.knock_line(0.9, (r * 0.05).max(1.5), &ellipse(v2(0.0, -ay * 0.02), ax * k, ay * k, 0.0), true);
        }
        d.ht(Ink::Pink, 0.12, &ellipse(v2(ax * 0.12, ay * 0.14), ax * 0.52, ay * 0.46, 0.0));
        d.outline(Ink::Key, (r * 0.06).clamp(1.6, INNER), &dough);
    } else {
        d.knock(&inner);
        d.fill(Ink::Yellow, 0.3, &inner);
        d.ht(Ink::Pink, 0.2, &inner);
        ramp(d, Ink::Key, &inner, v2(0.0, -ay * 0.7), v2(0.0, -ay * 0.2), 0.2, 0.0, 3);
        d.stroke_p(Paint::solid(Ink::Key, 1.0), (r * 0.05).clamp(1.4, DETAIL + 0.6), &inner, true);
    }
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

/// The oven's crust gauge: blonde → golden → bold swatches with a pointer at `t` (0..1).
/// Centred on the bar; `w` wide.
pub fn crust_gauge(d: &mut DrawList, w: f32, t: f32) {
    let h = 40.0;
    let track = rounded_rect(rect(-w * 0.5, -h * 0.5, w, h), h * 0.5);
    offset_shadow(d, &track, v2(4.0, 6.0));
    d.backing(&track);
    let third = w / 3.0;
    d.clipped(&track, |d| {
        for i in 0..3 {
            let seg = rect_poly(rect(-w * 0.5 + i as f32 * third, -h * 0.5, third + 0.5, h));
            let crust = i as f32 * 0.5;
            d.fill(Ink::Yellow, 0.5 + 0.4 * (i as f32).min(1.0), &seg);
            d.fill(Ink::Pink, 0.08 + 0.34 * crust, &seg);
            if i == 2 {
                d.ht(Ink::Key, 0.28, &seg);
            }
            ramp(d, Ink::Pink, &seg, v2(0.0, -h * 0.5), v2(0.0, h * 0.5), 0.0, 0.2 + 0.2 * crust, 3);
        }
        for i in 1..3 {
            let x = -w * 0.5 + i as f32 * third;
            d.stroke_p(Paint::solid(Ink::Key, 0.8), DETAIL, &[v2(x, -h * 0.5), v2(x, h * 0.5)], false);
        }
        // Tick marks along the bottom.
        for i in 1..24 {
            let x = -w * 0.5 + i as f32 * w / 24.0;
            let len = if i % 4 == 0 { 9.0 } else { 5.0 };
            d.stroke_p(Paint::solid(Ink::Key, 0.6), 1.3, &[v2(x, h * 0.5 - len), v2(x, h * 0.5)], false);
        }
        d.knock_p(
            0.6,
            Screen::Solid,
            PLATES_COLOR,
            &capsule(v2(-w * 0.5 + 18.0, -h * 0.26), v2(w * 0.5 - 18.0, -h * 0.26), 3.0),
        );
    });
    d.outline(Ink::Key, INNER + 0.8, &track);
    // Little loaves above each third.
    for i in 0..3 {
        let c = v2(-w * 0.5 + third * (i as f32 + 0.5), -h * 0.5 - 30.0);
        crust_icon(d, c, 17.0, i as f32 * 0.5);
    }
    let mx = -w * 0.5 + w * t.clamp(0.0, 1.0);
    let needle = capsule(v2(mx, -h * 0.5 - 6.0), v2(mx, h * 0.5 + 6.0), 4.0);
    d.backing(&needle);
    d.fill(Ink::Pink, 1.0, &needle);
    d.outline(Ink::Key, DETAIL + 0.6, &needle);
    let marker = vec![v2(mx, h * 0.5 + 4.0), v2(mx - 13.0, h * 0.5 + 24.0), v2(mx + 13.0, h * 0.5 + 24.0)];
    let marker = chaikin(&marker, 1, true);
    d.backing(&marker);
    d.fill(Ink::Pink, 1.0, &marker);
    d.outline(Ink::Key, INNER, &marker);
}

/// Back half of a mixing bowl (drawn before the dough), rim ellipse centred at origin.
pub fn bowl_back(d: &mut DrawList, r: f32) {
    let rim = ellipse(V2::ZERO, r, r * 0.3, 0.0);
    d.backing(&rim);
    d.fill(Ink::Blue, 0.42, &rim);
    d.ht(Ink::Key, 0.3, &rim);
    d.outline(Ink::Key, INNER, &rim);
}

/// Front of a mixing bowl (drawn after the dough) with the rim lip on top.
pub fn bowl_front(d: &mut DrawList, r: f32) {
    let mut body = arc(V2::ZERO, r, 0.0, PI);
    for p in body.iter_mut() {
        *p = v2(p.x, p.y * 0.95);
    }
    let front_rim = arc(V2::ZERO, r, 0.0, PI).into_iter().map(|p| v2(p.x, p.y * 0.3)).collect::<Vec<_>>();
    let mut shell = body.clone();
    shell.extend(front_rim.iter().rev().copied());
    contact_shadow(d, v2(0.0, r * 0.95), r * 0.72, r * 0.1);
    let foot = rounded_rect(rect(-r * 0.36, r * 0.86, r * 0.72, r * 0.14), 6.0);
    d.backing(&foot);
    d.fill(Ink::Blue, 0.6, &foot);
    d.ht(Ink::Key, 0.4, &foot);
    d.outline(Ink::Key, INNER, &foot);
    d.backing(&shell);
    d.fill(Ink::Blue, 0.62, &shell);
    ramp(d, Ink::Key, &shell, v2(-r, 0.0), v2(r, 0.0), 0.0, 0.3, 6);
    // Painted polka dots.
    for i in 0..9 {
        let a = PI * (0.12 + 0.76 * i as f32 / 8.0);
        let p = v2(a.cos() * r * 0.72, r * 0.3 + a.sin() * r * 0.4);
        d.knock_color(&circle(p, r * 0.045));
        d.fill(Ink::Pink, 0.8, &circle(p, r * 0.045));
    }
    d.knock_p(
        0.7,
        Screen::Solid,
        PLATES_COLOR,
        &arc_band(v2(0.0, 0.0), r * 0.84, PI * 0.72, PI * 0.9, r * 0.05),
    );
    d.outline(Ink::Key, OUTER, &shell);
    d.stroke_p(Paint::solid(Ink::Key, 1.0), INNER, &front_rim, false);
}

// =========================================================================================
// Icon art used in order bubbles, buttons and boards
// =========================================================================================

/// Line weight for icons of radius `s`.
pub fn icon_lw(s: f32) -> f32 {
    (s * 0.085).clamp(2.0, 4.5)
}

/// A baked loaf (top view) as a crisp flat icon: `crust` 0..1 shade, optional cut.
pub fn loaf_icon(d: &mut DrawList, c: V2, rx: f32, ry: f32, crust: f32, cut: bool) {
    let lw = icon_lw(rx);
    let body = ellipse(c, rx, ry, 0.0);
    d.backing(&body);
    d.fill(Ink::Yellow, 0.92, &body);
    d.fill(Ink::Pink, 0.18 + 0.5 * crust, &body);
    if crust > 0.6 {
        d.ht(Ink::Key, (crust - 0.6) * 0.8, &body);
    }
    // Rim shading.
    d.ht(Ink::Pink, 0.2 + 0.4 * crust, &ring_poly(c, rx, ry, 0.84));
    if cut {
        let pts = quad_bezier(
            c + v2(-rx * 0.55, ry * 0.18),
            c + v2(0.0, -ry * 0.42),
            c + v2(rx * 0.55, ry * 0.12),
            10,
        );
        d.knock_line(1.0, (rx * 0.16).max(2.6), &pts, false);
        d.fill(Ink::Yellow, 0.4, &crate::geom::lens_along(&pts, (rx * 0.13).max(2.2), 0.0));
        d.stroke_p(Paint::solid(Ink::Key, 1.0), lw * 0.7, &pts, false);
    }
    d.knock_p(
        0.7,
        Screen::Solid,
        PLATES_COLOR,
        &arc_band(c, rx.min(ry) * 0.72, PI * 1.12, PI * 1.38, rx * 0.07),
    );
    d.outline(Ink::Key, lw, &body);
}

/// Mini loaf silhouette icon with a scoring pattern (order bubbles, guide buttons).
pub fn pattern_icon(d: &mut DrawList, c: V2, s: f32, p: Pattern) {
    super::icons::icon(d, super::icons::Icon::Pattern(p), c, s / 0.88);
}

pub fn stencil_icon(d: &mut DrawList, c: V2, s: f32, st: Stencil) {
    super::icons::icon(d, super::icons::Icon::Stencil(st), c, s / 0.9);
}

pub fn topping_icon(d: &mut DrawList, c: V2, s: f32, t: Topping) {
    super::icons::icon(d, super::icons::Icon::Topping(t), c - v2(0.0, s * 0.12), s / 0.82);
}

/// A cheeky lemon: the "tangy" order icon.
pub fn lemon_icon(d: &mut DrawList, c: V2, s: f32) {
    let lw = icon_lw(s);
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
    let leaf =
        crate::geom::lens_along(&[c + v2(s * 0.3, -s * 0.55), c + v2(s * 0.8, -s * 0.9)], s * 0.26, 0.2);
    d.backing(&leaf);
    d.fill(Ink::Yellow, 0.9, &leaf);
    d.fill(Ink::Blue, 0.7, &leaf);
    d.outline(Ink::Key, lw * 0.8, &leaf);
    d.backing(&body);
    d.fill(Ink::Yellow, 1.0, &body);
    shade(d, &body, v2(-s * 0.16, -s * 0.2), Ink::Pink, 0.0, 0.22);
    d.knock_p(0.8, Screen::Solid, PLATES_COLOR, &arc_band(c, s * 0.62, PI * 1.12, PI * 1.4, s * 0.08));
    d.outline(Ink::Key, lw, &body);
    crate::art::face(d, c + v2(0.0, s * 0.05), s * 0.9, crate::art::Expr::Hmm, V2::ZERO);
}

/// A soft milk-cloud: the "mild" order icon.
pub fn cloud_icon(d: &mut DrawList, c: V2, s: f32) {
    let lw = icon_lw(s);
    let body = crate::geom::scallop(c, s, 7, 0.16);
    d.backing(&body);
    d.fill(Ink::Blue, 0.14, &body);
    let belly = clip_convex(&ellipse(c + v2(0.0, s * 0.62), s * 1.1, s * 0.5, 0.0), &circle(c, s * 1.2));
    let belly = clip_convex(&belly, &ellipse(c, s * 1.05, s * 1.05, 0.0));
    d.fill(Ink::Blue, 0.34, &belly);
    d.outline(Ink::Key, lw, &body);
    crate::art::face(d, c + v2(0.0, s * 0.05), s * 0.85, crate::art::Expr::Happy, V2::ZERO);
}

/// Crust-shade swatch: a little loaf with 0 (blonde) .. 1 (bold) browning.
pub fn crust_icon(d: &mut DrawList, c: V2, s: f32, crust: f32) {
    loaf_icon(d, c, s, s * 0.74, crust, true);
}

/// "Surprise me!" gift box.
pub fn gift_icon(d: &mut DrawList, c: V2, s: f32) {
    let lw = icon_lw(s);
    for sx in [-1.0f32, 1.0] {
        let loop_ = ellipse(c + v2(sx * s * 0.3, -s * 0.6), s * 0.3, s * 0.18, sx * 0.4);
        d.backing(&loop_);
        d.fill(Ink::Pink, 0.9, &loop_);
        d.outline(Ink::Key, lw * 0.8, &loop_);
    }
    let b = rounded_rect(rect(c.x - s * 0.8, c.y - s * 0.45, s * 1.6, s * 1.2), s * 0.12);
    d.backing(&b);
    d.fill(Ink::Blue, 0.55, &b);
    d.fill(Ink::Blue, 0.8, &rect_poly(rect(c.x - s * 0.8, c.y + s * 0.3, s * 1.6, s * 0.45)));
    d.fill(Ink::Pink, 0.9, &rect_poly(rect(c.x - s * 0.16, c.y - s * 0.45, s * 0.32, s * 1.2)));
    let lid = rounded_rect(rect(c.x - s * 0.9, c.y - s * 0.52, s * 1.8, s * 0.3), s * 0.08);
    d.backing(&lid);
    d.fill(Ink::Blue, 0.6, &lid);
    d.fill(Ink::Pink, 0.9, &rect_poly(rect(c.x - s * 0.16, c.y - s * 0.52, s * 0.32, s * 0.3)));
    d.outline(Ink::Key, lw, &b);
    d.outline(Ink::Key, lw, &lid);
    let knot = circle(c + v2(0.0, -s * 0.56), s * 0.12);
    d.backing(&knot);
    d.fill(Ink::Pink, 1.0, &knot);
    d.outline(Ink::Key, lw * 0.8, &knot);
    twinkle(d, c + v2(s * 0.95, -s * 0.8), s * 0.3, Ink::Yellow);
}
