//! Starter jars — the pets. The microbiome is drawn literally: pink round Yeasties and blue
//! bean-shaped Lactos swim in creamy starter; their mix *is* the tang dial.
//!
//! Anatomy (bottom centre at the origin, light from the top-left): a thick-bottomed glass
//! jar with rounded shoulders; creamy starter inside with bubbles pressed against the glass;
//! a face printed on the glass in the clear zone beside the rubber band (the band never
//! crosses it); a torn masking-tape name label; and a gingham cloth cap tied with string.
//!
//! Clip rule (the rasteriser's "Over" fills zero out masked-off pixels in the same span):
//! everything printed inside a clip that may overhang it is added (`.add()`) or knocked.

use super::style::{DETAIL, OUTER, contact_shadow};
use super::{Expr, face, zzz};
use crate::draw::{DrawList, PLATES_ALL, PLATES_COLOR, Paint, Screen};
use crate::geom::{V2, capsule, chaikin, circle, cubic_bezier, ellipse, quad_bezier, resample, v2};
use crate::ink::Ink;
use crate::rng::{Rng, hash01};
use std::f32::consts::{PI, TAU};

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

/// Footprint contract (origin = bottom centre of the jar).
pub const JAR_W: f32 = 170.0;
pub const JAR_H: f32 = 250.0;

/// Where the engine should place the name label (reference units, jar-local).
pub const TAPE_CENTER: V2 = v2(0.0, -157.0);

/// Half-width of the glass body.
const HALF: f32 = 85.0;
/// Half-width of the neck.
const NECK: f32 = 63.0;
/// Where the straight sides turn into the shoulders.
const SHOULDER: f32 = -176.0;
/// Where the shoulders meet the neck.
const NECK_Y: f32 = -200.0;
/// Glass thickness at the walls and at the (thick) base.
const WALL: f32 = 5.5;
const BASE: f32 = 13.0;
/// The masking tape's size.
const TAPE_W: f32 = 138.0;
const TAPE_H: f32 = 36.0;

fn level_y(rise: f32) -> f32 {
    -(26.0 + rise.clamp(0.0, 1.0) * 150.0)
}

/// Right half of the glass outline from the bottom centre up to the neck top, inset by `k`.
fn half_outline(k: f32) -> Vec<V2> {
    let x = HALF - k;
    let r = 34.0 - k * 0.5;
    let y0 = if k > 0.0 { -BASE } else { 0.0 };
    let mut p = vec![v2(0.0, y0), v2(x - r, y0)];
    p.extend(quad_bezier(v2(x - r, y0), v2(x, y0), v2(x, y0 - r), 8).into_iter().skip(1));
    // Sides swell a hair; shoulders roll in to the neck.
    p.extend(quad_bezier(v2(x, y0 - r), v2(x + 2.0, -100.0), v2(x, SHOULDER), 10).into_iter().skip(1));
    let nx = NECK - k;
    p.extend(
        cubic_bezier(
            v2(x, SHOULDER),
            v2(x, SHOULDER - 18.0),
            v2(nx + 12.0, NECK_Y + 1.0),
            v2(nx, NECK_Y),
            10,
        )
        .into_iter()
        .skip(1),
    );
    p.push(v2(nx, -216.0));
    p
}

/// A closed outline mirrored from [`half_outline`].
fn outline(k: f32) -> Vec<V2> {
    let right = half_outline(k);
    let mut p: Vec<V2> = right.iter().rev().map(|q| v2(-q.x, q.y)).collect();
    p.extend(right.iter().skip(1));
    p.pop();
    p
}

/// Where the face sits, printed on the glass: above the rubber band when there is room
/// (leaving the settled starter below the band to the microbes), otherwise below it; never
/// crossed by the band and clear of the tape. Returns (centre, size).
fn face_spot(band_y: f32) -> (V2, f32) {
    let lo = -BASE - 6.0;
    let hi = TAPE_CENTER.y + TAPE_H * 0.5 + 6.0;
    let above = (band_y - 10.0) - hi;
    let below = lo - (band_y + 10.0);
    let (c, room) = if above >= 40.0 || above >= below {
        ((band_y - 10.0 + hi) * 0.5, above)
    } else {
        ((lo + band_y + 10.0) * 0.5, below)
    };
    (v2(0.0, c), (room * 1.7).clamp(60.0, 86.0))
}

/// A tapered filled stroke along `pts` with half-width `hw(t)`.
fn taper_line(pts: &[V2], hw: impl Fn(f32) -> f32) -> Vec<V2> {
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let mut left = Vec::with_capacity(n * 2);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let a = pts[i.saturating_sub(1)];
        let b = pts[(i + 1).min(n - 1)];
        let nrm = (b - a).norm().perp();
        let w = hw(i as f32 / (n - 1) as f32);
        left.push(pts[i] + nrm * w);
        right.push(pts[i] - nrm * w);
    }
    right.reverse();
    left.extend(right);
    left
}

fn arch(t: f32, p: f32) -> f32 {
    (PI * t.clamp(0.0, 1.0)).sin().max(0.0).powf(p)
}

/// A Yeastie: round pink blob budding a baby, with a tiny face.
pub fn yeastie(d: &mut DrawList, c: V2, r: f32) {
    let lw = (r * 0.11).max(0.8);
    let bud_dir = V2::from_angle(-0.5 + hash01((c.x * 7.0 + 999.0) as u32, (c.y * 3.0 + 999.0) as u32) * 1.0);
    let bud = circle(c + bud_dir * (r * 0.95), r * 0.4);
    let body = circle(c, r);
    // Printed on paper (knocked) so the pink stays pink over the creamy starter.
    d.knock_color(&bud);
    d.fill(Ink::Pink, 0.8, &bud);
    d.outline(Ink::Key, lw, &bud);
    d.knock(&body);
    d.fill(Ink::Pink, 0.8, &body);
    d.outline(Ink::Key, lw, &body);
    d.knock_p(
        0.6,
        Screen::Solid,
        PLATES_COLOR,
        &ellipse(c + v2(-r * 0.36, -r * 0.44), r * 0.28, r * 0.17, -0.6),
    );
    for sx in [-1.0, 1.0] {
        d.fill(Ink::Key, 1.0, &circle(c + v2(sx * r * 0.33, r * 0.02), r * 0.15));
    }
    d.line(
        Ink::Key,
        lw * 0.9,
        &quad_bezier(c + v2(-r * 0.13, r * 0.3), c + v2(0.0, r * 0.42), c + v2(r * 0.13, r * 0.3), 5),
    );
}

/// A Lacto: blue bean-shaped rod with a tiny face.
pub fn lacto(d: &mut DrawList, c: V2, r: f32, ang: f32) {
    let lw = (r * 0.11).max(0.8);
    // Mostly horizontal so they read as beans, with a gentle tilt.
    let ang = (ang.sin() * 0.45).clamp(-0.5, 0.5);
    let dir = V2::from_angle(ang);
    let n = dir.perp();
    let spine: Vec<V2> = (0..=8)
        .map(|i| {
            let t = i as f32 / 8.0 * 2.0 - 1.0;
            c + dir * (t * r * 1.2) - n * ((1.0 - t * t) * r * 0.16)
        })
        .collect();
    let w = r * 0.68;
    let side = |s: f32| -> Vec<V2> {
        spine
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let t = i as f32 / 8.0;
                let k = (1.0 - (2.0 * t - 1.0).powi(6)).max(0.0).sqrt();
                *p + n * (s * w * k)
            })
            .collect()
    };
    let mut body = side(1.0);
    body.extend(side(-1.0).into_iter().rev());
    let body = chaikin(&body, 2, true);
    d.knock(&body);
    d.fill(Ink::Blue, 0.6, &body);
    d.outline(Ink::Key, lw, &body);
    d.knock_p(
        0.6,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(c - dir * (r * 0.7) - n * (w * 0.42), c - dir * (r * 0.05) - n * (w * 0.5), r * 0.11),
    );
    for sx in [-1.0, 1.0] {
        d.fill(Ink::Key, 1.0, &circle(c + dir * (sx * r * 0.34) + n * (r * 0.06), r * 0.14));
    }
}

/// Draw a starter jar with its bottom centre at the origin.
pub fn jar(d: &mut DrawList, v: &JarView) {
    let body = outline(0.0);
    let cavity = outline(WALL);
    let rise = if v.hooch { v.rise.min(0.3) } else { v.rise.clamp(0.0, 1.0) };
    let y_top = level_y(rise);
    let band_y = level_y(v.band);
    let (face_c, face_s) = face_spot(band_y);

    contact_shadow(d, v2(4.0, 2.0), 100.0, 14.0);

    // Glass: opaque paper behind, the faintest blue in the body.
    d.backing(&body);
    d.fill(Ink::Blue, 0.05, &body);
    d.clipped(&cavity, |d| starter(d, v, y_top, face_c, face_s, band_y));
    glass(d, &body, &cavity);
    face(d, face_c, face_s, v.expr, V2::ZERO);
    d.outline(Ink::Key, OUTER, &body);

    rubber_band(d, band_y);
    tape(d, v.seed);
    cloth_cap(d, v);

    if v.hooch {
        zzz(d, v2(72.0, -262.0), 14.0);
    } else if v.pep > 0.85 {
        // Peak pep: sparkles above the cap.
        for (i, p) in [v2(-74.0, -252.0), v2(80.0, -238.0)].iter().enumerate() {
            let s = 6.0 + 2.0 * (v.t * 2.0 + i as f32).sin().abs();
            super::twinkle(d, *p, s, Ink::Yellow);
        }
    }
}

/// The starter: creamy, domed at peak pep, bubbles against the glass, microbes, hooch.
fn starter(d: &mut DrawList, v: &JarView, y_top: f32, face_c: V2, face_s: f32, band_y: f32) {
    let pep = v.pep.clamp(0.0, 1.0);
    let hooch = v.hooch;
    let mut rng = Rng::new(v.seed as u64 * 131 + 7);
    let dome = if hooch { 0.0 } else { 3.0 + 12.0 * pep };
    let wob = v.t * 1.7;
    // The surface: domed in the middle, lumpy with pep, creeping up the glass at the edges.
    let surface: Vec<V2> = (0..=36)
        .map(|i| {
            let x = -HALF + 2.0 * HALF * i as f32 / 36.0;
            let u = x / (HALF - WALL);
            let lump = ((x * 0.13 + wob).sin() * 1.2 + (x * 0.31 - wob * 0.6).sin() * 0.7) * (0.2 + pep);
            let creep = (u.abs() - 0.86).max(0.0) * 26.0;
            v2(x, y_top - dome * (1.0 - u * u).max(0.0) + lump - creep)
        })
        .collect();
    let mut goop = surface.clone();
    goop.push(v2(HALF + 5.0, 10.0));
    goop.push(v2(-HALF - 5.0, 10.0));

    // Creamy starter (flat tints: a screen reads as cork at this size), settling denser
    // towards the bottom and a touch warmer against the glass.
    d.fill_p(Paint::solid(Ink::Yellow, 0.36).add(), &goop);
    d.fill_p(Paint::solid(Ink::Pink, 0.05).add(), &goop);
    let depth = -y_top - BASE;
    // Each layer adds a little more density on top of the one above it.
    for (h, y, p) in [(0.5f32, 0.11f32, 0.021f32), (0.25, 0.12, 0.022)] {
        let top = -BASE - depth * h;
        let layer = chaikin(
            &[v2(-HALF, top + 4.0), v2(0.0, top - 3.0), v2(HALF, top + 4.0), v2(HALF, 10.0), v2(-HALF, 10.0)],
            2,
            true,
        );
        d.fill_p(Paint::solid(Ink::Yellow, y).add(), &layer);
        d.fill_p(Paint::solid(Ink::Pink, p).add(), &layer);
    }
    // The top: a darker, glossier skin under the surface line.
    let skin: Vec<V2> = surface.iter().map(|p| *p + v2(0.0, 4.5)).collect();
    d.stroke_p(Paint::solid(Ink::Yellow, 0.25).add(), 7.0, &skin, false);
    d.stroke_p(Paint::solid(Ink::Pink, 0.05).add(), 7.0, &skin, false);
    d.stroke_p(Paint::solid(Ink::Pink, 0.45).add(), 2.0, &surface, false);
    d.stroke_p(Paint::solid(Ink::Key, 0.35).add(), 1.2, &surface, false);

    // A soft film left on the glass where the starter peaked and fell back.
    if hooch || v.band > v.rise + 0.05 {
        let peak = level_y((v.rise + 0.22).min(1.0));
        let mut film: Vec<V2> = surface.iter().map(|p| *p + v2(0.0, 2.0)).collect();
        let crest: Vec<V2> = (0..=18)
            .rev()
            .map(|i| {
                let x = -HALF + 2.0 * HALF * i as f32 / 18.0;
                v2(x, peak + 7.0 * vnoise1(x * 0.07, v.seed) + 4.0 * (x * 0.21).sin())
            })
            .collect();
        film.extend(crest);
        d.fill_p(Paint::solid(Ink::Yellow, 0.14).add(), &film);
        for i in 0..7 {
            let x = -64.0 + i as f32 * 21.0 + rng.range(-6.0, 6.0);
            let y1 = peak + rng.range(4.0, 14.0);
            let y0 = y_top - rng.range(2.0, 8.0);
            if y1 < y0 {
                let drip = vec![v2(x, y1), v2(x + rng.range(-1.5, 1.5), y0)];
                d.stroke_p(Paint::solid(Ink::Yellow, 0.22).add(), rng.range(3.0, 6.0), &drip, false);
            }
        }
    }

    // Bubbles press against the glass but stay inside it (clear of the rounded corners).
    let cavity = outline(WALL);
    let mut ring = cavity.clone();
    ring.push(cavity[0]);
    let within = |p: V2, margin: f32| {
        crate::geom::point_in_poly(p, &cavity) && crate::geom::dist_to_polyline(p, &ring) >= margin
    };
    // Keep-out zones: the face, the band and the tape.
    let clear = |p: V2, r: f32| -> bool {
        // The face spans the eyes (a little above centre) down to the blush and mouth.
        let dy = p.y - face_c.y;
        let in_face =
            (p.x - face_c.x).abs() < face_s * 0.53 + r && dy > -face_s * 0.16 - r && dy < face_s * 0.24 + r;
        let in_band = (p.y - band_y).abs() < 7.0 + r;
        let in_tape = (p.y - TAPE_CENTER.y).abs() < TAPE_H * 0.5 + r && p.x.abs() < TAPE_W * 0.5 + r;
        !(in_face || in_band || in_tape)
    };

    // Bubbles pressed against the glass: bigger and busier with pep.
    let span = (-BASE - 4.0) - (y_top + 6.0);
    if !hooch && span > 8.0 {
        let n = (6.0 + 22.0 * pep) as usize;
        let mut rings = Vec::new();
        let mut dots = Vec::new();
        for i in 0..n {
            let hx = hash01(v.seed, i as u32 * 3);
            let hy = hash01(v.seed, i as u32 * 3 + 1);
            let hr = hash01(v.seed, i as u32 * 3 + 2);
            // Bubbles gather towards the top where the action is, and at the glass.
            let y = y_top + 8.0 + span * hy.powf(1.7);
            let edge = if i % 3 == 0 { hx.round() * 2.0 - 1.0 } else { hx * 2.0 - 1.0 };
            let x = edge * (HALF - 16.0);
            let r = 1.6 + 5.4 * hr * hr * (0.45 + pep);
            let p = v2(x, y);
            if !clear(p, r) || !within(p, r + 1.5) {
                continue;
            }
            if r > 3.0 {
                rings.push((p, r));
            } else {
                dots.push(circle(p, r));
            }
        }
        for (p, r) in &rings {
            let c = circle(*p, *r);
            d.knock_p(0.85, Screen::Solid, PLATES_COLOR, &c);
            d.fill_p(Paint::solid(Ink::Yellow, 0.1).add(), &c);
            d.stroke_p(Paint::solid(Ink::Key, 0.55).add(), DETAIL * 0.5, &c, true);
            d.stroke_p(
                Paint::solid(Ink::Pink, 0.35).add(),
                DETAIL * 0.9,
                &circle(*p + v2(0.4, 0.6), *r),
                true,
            );
            d.knock(&circle(*p + v2(-r * 0.35, -r * 0.35), r * 0.3));
        }
        for c in &dots {
            d.knock_p(0.8, Screen::Solid, PLATES_COLOR, c);
        }
    }

    // Microbes: count grows with pep, the mix follows tang.
    let n = (3.0 + 8.0 * pep).round() as u32;
    let n_lacto = (n as f32 * (0.1 + 0.8 * v.tang.clamp(0.0, 1.0))).round() as u32;
    let top = y_top + 14.0;
    let bottom = -BASE - 12.0;
    if bottom > top {
        let mut placed: Vec<V2> = Vec::new();
        let mut i = 0;
        let mut tries = 0;
        while i < n && tries < 90 {
            tries += 1;
            let ph = rng.range(0.0, TAU);
            let x = rng.range(-HALF + 25.0, HALF - 25.0);
            let y = rng.range(top, bottom);
            let bob = (v.t * (1.4 + pep) + ph).sin() * (1.5 + 2.5 * pep);
            let c = v2(x, y + bob);
            let r = 9.5;
            if !clear(c, r + 2.0) || placed.iter().any(|q| q.dist(c) < 2.5 * r) {
                continue;
            }
            placed.push(c);
            // Interleave the two kinds so a crowded jar still shows the mix.
            let is_lacto = ((i + 1) * n_lacto) / n > (i * n_lacto) / n;
            if is_lacto {
                lacto(d, c, r * 0.95, ph);
            } else {
                yeastie(d, c, r);
            }
            i += 1;
        }
    }

    if hooch {
        // A grey-brown layer of hooch pooled on the flat, tired starter.
        let top = y_top - 18.0;
        let layer = vec![v2(-HALF, top), v2(HALF, top), v2(HALF, y_top + 2.0), v2(-HALF, y_top + 2.0)];
        d.knock(&layer);
        d.fill_p(Paint::solid(Ink::Blue, 0.2).add(), &layer);
        d.fill_p(Paint::solid(Ink::Key, 0.12).add(), &layer);
        d.fill_p(Paint::solid(Ink::Yellow, 0.22).add(), &layer);
        let lower = vec![
            v2(-HALF, y_top - 7.0),
            v2(HALF, y_top - 7.0),
            v2(HALF, y_top + 2.0),
            v2(-HALF, y_top + 2.0),
        ];
        d.fill_p(Paint::solid(Ink::Key, 0.1).add(), &lower);
        d.stroke_p(Paint::solid(Ink::Key, 1.0).add(), DETAIL * 0.8, &[v2(-HALF, top), v2(HALF, top)], false);
        d.knock_p(0.8, Screen::Solid, PLATES_ALL, &capsule(v2(-54.0, top + 5.0), v2(6.0, top + 5.0), 1.6));
    }
}

/// Smooth 1D value noise in [-0.5, 0.5).
fn vnoise1(x: f32, seed: u32) -> f32 {
    let x0 = x.floor();
    let f = x - x0;
    let s = f * f * (3.0 - 2.0 * f);
    let h = |i: f32| hash01(seed, (i as i32 + 4096) as u32);
    h(x0) + (h(x0 + 1.0) - h(x0)) * s - 0.5
}

/// Glass: blue at the grazing edges, a thick base, long reflections on both sides.
fn glass(d: &mut DrawList, body: &[V2], cavity: &[V2]) {
    d.clipped(body, |d| {
        // The walls seen at a grazing angle: bluer towards the silhouette.
        for (sx, k) in [(-1.0f32, 1.0f32), (1.0, 1.2)] {
            for (w, tone) in [(16.0f32, 0.08f32), (8.0, 0.14)] {
                let x0 = sx * (HALF + 2.0);
                let wall = vec![
                    v2(x0, -24.0),
                    v2(x0 - sx * w, -34.0),
                    v2(x0 - sx * w, SHOULDER + 4.0),
                    v2(x0, SHOULDER - 8.0),
                ];
                d.fill_p(Paint::solid(Ink::Blue, tone * k).add(), &chaikin(&wall, 2, true));
            }
        }
        // The thick base: a lens of glass under the starter.
        let base = chaikin(
            &[
                v2(-HALF + 20.0, -1.0),
                v2(HALF - 20.0, -1.0),
                v2(HALF - 14.0, -BASE + 1.0),
                v2(0.0, -BASE - 2.0),
                v2(-HALF + 14.0, -BASE + 1.0),
            ],
            2,
            true,
        );
        d.knock_p(0.7, Screen::Solid, PLATES_COLOR, &base);
        d.fill_p(Paint::solid(Ink::Blue, 0.24).add(), &base);
        d.knock_p(0.9, Screen::Solid, PLATES_ALL, &capsule(v2(-44.0, -6.5), v2(10.0, -7.0), 1.8));
    });
    // Where the glass wall meets the inside.
    d.stroke_p(Paint::solid(Ink::Key, 0.25), DETAIL * 0.5, cavity, true);
    // Reflections: a long window-shaped glint on the left, a slim one on the right.
    let left = resample(&chaikin(&[v2(-67.0, -40.0), v2(-70.0, -110.0), v2(-67.0, -166.0)], 2, false), 16);
    d.knock_p(0.92, Screen::Solid, PLATES_ALL, &taper_line(&left, |t| 5.6 * arch(t, 0.45)));
    let left2 = resample(&chaikin(&[v2(-57.0, -60.0), v2(-59.0, -105.0), v2(-57.0, -140.0)], 2, false), 12);
    d.knock_p(0.7, Screen::Solid, PLATES_ALL, &taper_line(&left2, |t| 1.6 * arch(t, 0.6)));
    d.knock(&circle(v2(-66.0, -29.0), 3.0));
    let right = resample(&chaikin(&[v2(71.0, -62.0), v2(73.0, -112.0), v2(70.0, -150.0)], 2, false), 12);
    d.knock_p(0.85, Screen::Solid, PLATES_ALL, &taper_line(&right, |t| 2.4 * arch(t, 0.6)));
    // The shoulders catch the light too.
    let sh = resample(
        &chaikin(&[v2(-73.0, SHOULDER - 6.0), v2(-63.0, SHOULDER - 18.0), v2(-50.0, NECK_Y + 2.0)], 2, false),
        10,
    );
    d.knock_p(0.85, Screen::Solid, PLATES_ALL, &taper_line(&sh, |t| 3.2 * arch(t, 0.6)));
}

/// The rubber band: a fat glossy elastic hugging the glass, dipping towards us.
fn rubber_band(d: &mut DrawList, y: f32) {
    let front = quad_bezier(v2(-HALF - 1.0, y - 2.5), v2(0.0, y + 6.5), v2(HALF + 1.0, y - 2.5), 24);
    let strap = taper_line(&front, |t| 3.9 - 0.6 * (2.0 * t - 1.0).powi(2));
    d.backing(&strap);
    d.fill(Ink::Pink, 1.0, &strap);
    d.fill(Ink::Key, 0.12, &strap);
    d.outline(Ink::Key, 1.6, &strap);
    // Glossy top edge, a darker underside.
    let hl: Vec<V2> = front.iter().map(|p| *p + v2(0.0, -1.5)).collect();
    d.knock_p(0.75, Screen::Solid, PLATES_ALL, &taper_line(&hl[3..14], |t| 0.9 * arch(t, 0.7)));
    let under: Vec<V2> = front.iter().map(|p| *p + v2(0.0, 2.0)).collect();
    d.fill_p(Paint::solid(Ink::Key, 0.3).add(), &taper_line(&under, |t| 1.1 * arch(t, 0.3)));
}

/// The masking-tape label (the engine prints the name on it): torn ends, a slight tilt.
fn tape(d: &mut DrawList, seed: u32) {
    let mut rng = Rng::new(seed as u64 * 7 + 3);
    let (hw, hh) = (TAPE_W * 0.5, TAPE_H * 0.5);
    let torn = |x: f32, rng: &mut Rng, down: bool| -> Vec<V2> {
        let n = 8;
        (0..=n)
            .map(|i| {
                let t = i as f32 / n as f32;
                let y = if down { -hh + 2.0 * hh * t } else { hh - 2.0 * hh * t };
                let j = if i == 0 || i == n { 0.3 } else { 1.0 };
                v2(x + rng.range(-3.0, 3.0) * j, y)
            })
            .collect()
    };
    // The tape wraps the cylinder: its long edges bow gently.
    let mut p: Vec<V2> = quad_bezier(v2(-hw, -hh), v2(0.0, -hh + 3.0), v2(hw, -hh), 12);
    p.extend(torn(hw, &mut rng, true).into_iter().skip(1));
    p.extend(quad_bezier(v2(hw, hh), v2(0.0, hh + 3.0), v2(-hw, hh), 12).into_iter().skip(1));
    p.extend(torn(-hw, &mut rng, false).into_iter().skip(1));
    d.with(crate::geom::Xf::at(TAPE_CENTER).rotated(-0.03), |d| {
        let shadow: Vec<V2> = p.iter().map(|q| *q + v2(1.4, 2.0)).collect();
        d.fill_p(Paint::solid(Ink::Key, 0.12).add(), &shadow);
        d.knock_color(&p);
        d.fill(Ink::Yellow, 0.26, &p);
        d.fill(Ink::Pink, 0.05, &p);
        // Crepe-paper texture: faint fibres and a slightly darker, sticky edge.
        for i in 0..4 {
            let y = -hh + 7.0 + i as f32 * 7.5;
            d.stroke_p(
                Paint::solid(Ink::Yellow, 0.4),
                0.9,
                &[v2(-hw + 5.0, y), v2(hw - 5.0, y + 0.8)],
                false,
            );
        }
        d.stroke_p(Paint::solid(Ink::Yellow, 0.5), 3.0, &p, true);
        d.stroke_p(Paint::solid(Ink::Key, 0.4), DETAIL * 0.55, &p, true);
    });
}

/// Gingham cloth over the lid: a puffy bun of cloth, a flared skirt with soft folds, tied
/// with string.
fn cloth_cap(d: &mut DrawList, v: &JarView) {
    let ink = v.cloth;
    let tie_y = NECK_Y - 13.0;
    let puff_w = NECK + 12.0;
    let equator = tie_y - 9.0;
    let crown = -JAR_H;
    // The puff: (u across -1..1, s from the crown 0 to the tie 1). Meridians converge on the
    // crown; parallels bow towards us because we look down on it a little.
    let puff_at = |u: f32, s: f32| -> V2 {
        if s < 0.8 {
            let phi = s / 0.8 * PI * 0.5;
            let x = u * puff_w * phi.sin();
            let bow = 7.0 * (1.0 - u * u).max(0.0).sqrt() * phi.sin();
            v2(x, equator - (equator - crown) * phi.cos() + bow)
        } else {
            let t = (s - 0.8) / 0.2;
            let w = puff_w + (NECK + 4.0 - puff_w) * t * t;
            let bow = 7.0 * (1.0 - u * u).max(0.0).sqrt() * (1.0 - 0.6 * t);
            v2(u * w, equator + (tie_y - equator) * t + bow)
        }
    };
    // The skirt, gathered at the tie and flaring into soft folds.
    let folds = 6.0;
    let skirt_at = |u: f32, s: f32| -> V2 {
        let flare = NECK + 4.0 + 13.0 * s;
        let wave = (u * folds * PI + 0.5).sin();
        let bow = 5.0 * (1.0 - u * u).max(0.0).sqrt();
        v2(u * flare + wave * 2.0 * s, tie_y + bow * 0.6 + s * (15.0 + 3.0 * wave * s))
    };
    let skirt = {
        let mut p: Vec<V2> = (0..=16).map(|i| skirt_at(-1.0 + 2.0 * i as f32 / 16.0, 0.0)).collect();
        // A pinked hem: zig-zag teeth along the bottom.
        p.extend((0..=60).rev().map(|i| {
            let u = -1.0 + 2.0 * i as f32 / 60.0;
            skirt_at(u, 1.0) + v2(0.0, if i % 2 == 0 { 0.0 } else { 2.6 })
        }));
        p
    };
    let puff = {
        let mut p: Vec<V2> = (0..=24).map(|i| puff_at(-1.0, i as f32 / 24.0)).collect();
        p.extend((1..24).map(|i| puff_at(-1.0 + 2.0 * i as f32 / 24.0, 1.0)));
        p.extend((0..=24).rev().map(|i| puff_at(1.0, i as f32 / 24.0)));
        p
    };

    // Gingham woven through (u, s) space so the stripes follow the cloth.
    let gingham = |d: &mut DrawList, f: &dyn Fn(f32, f32) -> V2, cols: usize, rows: usize| {
        let tone = 0.4;
        for c in (0..cols).step_by(2) {
            let (u0, u1) = (-1.0 + 2.0 * c as f32 / cols as f32, -1.0 + 2.0 * (c + 1) as f32 / cols as f32);
            let mut p: Vec<V2> = (0..=14).map(|i| f(u0, i as f32 / 14.0)).collect();
            p.extend((0..=14).rev().map(|i| f(u1, i as f32 / 14.0)));
            d.fill_p(Paint::solid(ink, tone).add(), &p);
        }
        for r in (0..rows).step_by(2) {
            let (s0, s1) = (r as f32 / rows as f32, (r + 1) as f32 / rows as f32);
            let mut p: Vec<V2> = (0..=20).map(|i| f(-1.0 + 2.0 * i as f32 / 20.0, s0)).collect();
            p.extend((0..=20).rev().map(|i| f(-1.0 + 2.0 * i as f32 / 20.0, s1)));
            d.fill_p(Paint::solid(ink, tone).add(), &p);
        }
    };

    d.backing(&skirt);
    d.clipped(&skirt, |d| {
        gingham(d, &skirt_at, 14, 2);
        // Folds: the faces turned away from the light fall into soft shade.
        for k in 0..(folds as usize) {
            let u = -1.0 + (2.0 * k as f32 + 1.0) / folds;
            let mut p: Vec<V2> = (0..=6).map(|i| skirt_at(u + 0.02, i as f32 / 6.0)).collect();
            p.extend((0..=6).rev().map(|i| skirt_at(u + 0.13, i as f32 / 6.0)));
            d.fill_p(Paint::solid(Ink::Key, 0.13).add(), &p);
        }
        // Shade tucked under the puff.
        let tuck: Vec<V2> = (0..=16).map(|i| skirt_at(-1.0 + 2.0 * i as f32 / 16.0, 0.05)).collect();
        d.stroke_p(Paint::solid(Ink::Key, 0.2).add(), 7.0, &tuck, false);
    });
    d.outline(Ink::Key, DETAIL * 1.2, &skirt);

    d.backing(&puff);
    d.clipped(&puff, |d| {
        gingham(d, &puff_at, 12, 7);
        // Light from the top-left: a sheen up there, shade to the right and under the belly.
        let shade = chaikin(
            &[
                v2(26.0, tie_y + 6.0),
                v2(puff_w + 8.0, tie_y + 6.0),
                v2(puff_w + 8.0, crown - 4.0),
                v2(40.0, crown - 4.0),
            ],
            2,
            true,
        );
        d.fill_p(Paint::solid(Ink::Key, 0.11).add(), &shade);
        let belly: Vec<V2> = (0..=20).map(|i| puff_at(-1.0 + 2.0 * i as f32 / 20.0, 0.93)).collect();
        d.stroke_p(Paint::solid(Ink::Key, 0.16).add(), 7.0, &belly, false);
        d.knock_p(0.5, Screen::Solid, PLATES_COLOR, &ellipse(v2(-30.0, crown + 12.0), 22.0, 7.0, -0.25));
        d.knock_p(0.3, Screen::Solid, PLATES_COLOR, &ellipse(v2(-34.0, crown + 20.0), 30.0, 11.0, -0.3));
    });
    d.outline(Ink::Key, OUTER * 0.9, &puff);

    // String wrapped twice round the neck, and a bow.
    let wrap = quad_bezier(v2(-NECK - 5.0, tie_y), v2(0.0, tie_y + 6.0), v2(NECK + 5.0, tie_y), 16);
    d.line(Ink::Key, 3.2, &wrap);
    let wrap2: Vec<V2> = wrap.iter().map(|p| *p + v2(0.0, -3.6)).collect();
    d.stroke_p(Paint::solid(Ink::Key, 0.75), 2.0, &wrap2, false);
    let bow = v2(36.0, tie_y + 3.5);
    for sx in [-1.0f32, 1.0] {
        let lp = chaikin(
            &[bow, bow + v2(sx * 7.0, -10.0), bow + v2(sx * 15.0, -6.0), bow + v2(sx * 12.5, 2.0), bow],
            2,
            true,
        );
        d.backing(&lp);
        d.fill(Ink::Yellow, 0.22, &lp);
        d.outline(Ink::Key, 2.4, &lp);
        let tail = quad_bezier(bow, bow + v2(sx * 4.0, 8.0), bow + v2(sx * 7.0 + 2.0, 15.0), 6);
        d.line(Ink::Key, 2.4, &tail);
    }
    d.fill(Ink::Key, 1.0, &circle(bow, 3.2));
}
