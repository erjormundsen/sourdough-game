//! Secondary goods: discard muffins, cinnamon buns and bagels.
//!
//! All three are seen in a gentle 3/4 view with the house light from the top-left, and the
//! raw versions share the finished silhouettes: batter in the paper cup, a pale coil of
//! cinnamon dough, a matte shaped ring. The finishing swipe bakes, glazes or seeds them.
//!
//! Clip rule (the rasteriser's "Over" fills zero out masked-off pixels in the same span):
//! inside a clip, anything that may overhang the clip is added (`.add()`) or knocked, and
//! gradients are nested copies of the clip shape itself.

use super::style::{DETAIL, INNER, OUTER, contact_shadow};
use crate::content::Treat;
use crate::draw::{DrawList, PLATES_ALL, PLATES_COLOR, Paint, Screen};
use crate::geom::{V2, chaikin, circle, ellipse, quad_bezier, v2};
use crate::ink::Ink;
use crate::rng::Rng;
use std::f32::consts::{PI, TAU};

/// Line-weight factor for a treat `s` wide (thinner on small treats, never hairline).
fn lwk(s: f32) -> f32 {
    (s / 130.0).clamp(0.45, 1.3).sqrt()
}

fn arch(t: f32, p: f32) -> f32 {
    (PI * t.clamp(0.0, 1.0)).sin().max(0.0).powf(p)
}

/// A filled tapered stroke along `pts` with half-width `hw(t)`.
fn taper(pts: &[V2], hw: impl Fn(f32) -> f32) -> Vec<V2> {
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

/// Lumpy closed outline around `c` (radii `rx`, `ry`) with low harmonics.
fn lumpy(c: V2, rx: f32, ry: f32, wob: f32, seed: u32, n: usize) -> Vec<V2> {
    let mut rng = Rng::new(seed as u64 * 17 + 5);
    let ph: Vec<f32> = (0..3).map(|_| rng.range(0.0, TAU)).collect();
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let k = 1.0
                + wob
                    * (0.5 * (3.0 * a + ph[0]).sin()
                        + 0.3 * (5.0 * a + ph[1]).sin()
                        + 0.2 * (7.0 * a + ph[2]).sin());
            c + v2(a.cos() * rx * k, a.sin() * ry * k)
        })
        .collect()
}

/// A 3/4-view silhouette: the `top` outline (centred on `c`) plus its side wall `side` deep.
fn with_side(top: &[V2], c: V2, rx: f32, side: f32) -> Vec<V2> {
    let mut upper: Vec<V2> = top.iter().filter(|q| q.y <= c.y).cloned().collect();
    upper.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut lower: Vec<V2> = top.iter().filter(|q| q.y > c.y).map(|q| *q + v2(0.0, side)).collect();
    lower.sort_by(|a, b| b.x.total_cmp(&a.x));
    let mut out = upper;
    out.push(v2(rx * 1.01, c.y + side * 0.5));
    out.extend(lower);
    out.push(v2(-rx * 1.01, c.y + side * 0.5));
    chaikin(&out, 2, true)
}

/// An unfinished treat on the tray (before the finishing swipe).
pub fn treat_raw(d: &mut DrawList, t: Treat, s: f32, seed: u32) {
    contact_shadow(d, v2(s * 0.02, s * 0.37), s * 0.46, s * 0.075);
    match t {
        Treat::Muffin => muffin(d, s, seed, false),
        Treat::CinnamonBun => bun(d, s, seed, false),
        Treat::Bagel => bagel(d, s, seed, false),
    }
}

/// Draw one treat centred at the origin, roughly `s` wide. `seed` varies toppings.
pub fn treat(d: &mut DrawList, t: Treat, s: f32, seed: u32) {
    contact_shadow(d, v2(s * 0.02, s * 0.37), s * 0.46, s * 0.075);
    match t {
        Treat::Muffin => muffin(d, s, seed, true),
        Treat::CinnamonBun => bun(d, s, seed, true),
        Treat::Bagel => bagel(d, s, seed, true),
    }
}

// ---------------------------------------------------------------------------------------
// Muffin
// ---------------------------------------------------------------------------------------

fn muffin(d: &mut DrawList, s: f32, seed: u32, baked: bool) {
    let k = lwk(s);
    let mut rng = Rng::new(seed as u64 * 31 + 1);
    // The pleated paper cup: wide at the rim, tucked at the base, seen a touch from above.
    let (rim_y, base_y) = (-0.02 * s, 0.36 * s);
    let (rim_w, base_w) = (0.36 * s, 0.27 * s);
    let rim_front = quad_bezier(v2(-rim_w, rim_y), v2(0.0, rim_y + 0.07 * s), v2(rim_w, rim_y), 16);
    let base_front = quad_bezier(v2(base_w, base_y), v2(0.0, base_y + 0.05 * s), v2(-base_w, base_y), 16);
    let mut cup = rim_front.clone();
    cup.extend(base_front.iter().cloned());
    d.backing(&cup);
    d.fill(Ink::Pink, 0.46, &cup);
    let pleats = 11;
    d.clipped(&cup, |d| {
        for i in 0..pleats {
            let u0 = i as f32 / pleats as f32;
            let u1 = (i as f32 + 0.5) / pleats as f32;
            let at = |u: f32, top: bool| {
                let x = -1.0 + 2.0 * u;
                if top {
                    v2(x * rim_w, rim_y + 0.07 * s * (1.0 - x * x))
                } else {
                    v2(x * base_w, base_y + 0.05 * s * (1.0 - x * x))
                }
            };
            // Every other pleat faces away from the light.
            let q = vec![at(u0, true), at(u1, true), at(u1, false), at(u0, false)];
            d.fill_p(Paint::solid(Ink::Key, 0.1 + 0.08 * u0).add(), &q);
            d.stroke_p(
                Paint::solid(Ink::Key, 0.5).add(),
                DETAIL * 0.55 * k,
                &[at(u0, true), at(u0, false)],
                false,
            );
        }
        // Lit on the left, rolling into shade on the right.
        d.knock_p(
            0.3,
            Screen::Solid,
            PLATES_COLOR,
            &[
                v2(-rim_w, rim_y),
                v2(-rim_w * 0.55, rim_y),
                v2(-base_w * 0.6, base_y + 0.1 * s),
                v2(-base_w, base_y + 0.1 * s),
            ],
        );
        d.fill_p(
            Paint::ht(Ink::Key, 0.3).add(),
            &[
                v2(rim_w * 0.55, rim_y),
                v2(rim_w, rim_y),
                v2(base_w, base_y + 0.1 * s),
                v2(base_w * 0.6, base_y + 0.1 * s),
            ],
        );
    });
    d.outline(Ink::Key, INNER * k, &cup);

    if baked {
        // The muffin top: a crumbly golden mushroom cap spilling over the rim.
        let base = rim_y + 0.03 * s;
        let dome = {
            let mut p = Vec::new();
            for i in 0..=48 {
                let a = PI + PI * i as f32 / 48.0;
                let (sa, ca) = a.sin_cos();
                let full = sa.abs().powf(0.8) * sa.signum();
                let bumps = 0.022 * ((a * 13.0 + seed as f32).sin() + 0.7 * (a * 21.0 + 1.3).sin());
                let r = 1.0 + bumps;
                p.push(v2(ca * 0.47 * s * r, base + full * 0.4 * s * r));
            }
            // The underside of the cap tucks back to the paper rim.
            p.extend(quad_bezier(
                v2(0.47 * s, base),
                v2(0.46 * s, base + 0.06 * s),
                v2(0.34 * s, base + 0.055 * s),
                8,
            ));
            p.extend(quad_bezier(
                v2(0.34 * s, base + 0.055 * s),
                v2(0.0, base + 0.09 * s),
                v2(-0.34 * s, base + 0.055 * s),
                10,
            ));
            p.extend(quad_bezier(
                v2(-0.34 * s, base + 0.055 * s),
                v2(-0.46 * s, base + 0.06 * s),
                v2(-0.47 * s, base),
                8,
            ));
            chaikin(&p, 2, true)
        };
        // Its shadow on the cup.
        let shade: Vec<V2> = rim_front.iter().map(|p| *p + v2(0.0, 0.06 * s)).collect();
        d.fill_p(Paint::ht(Ink::Key, 0.5).add(), &taper(&shade, |t| 0.035 * s * arch(t, 0.4)));
        d.backing(&dome);
        d.fill(Ink::Yellow, 0.9, &dome);
        d.clipped(&dome, |d| {
            // A lit dome: pale gold on the top-left crown, toasty at the rim and underside.
            let crown = v2(-0.1 * s, base - 0.26 * s);
            let steps = [(1.0f32, 0.6f32), (0.84, 0.48), (0.68, 0.36), (0.5, 0.26), (0.32, 0.17)];
            for (sc, tone) in steps {
                let ring: Vec<V2> = dome.iter().map(|q| crown + (*q - crown) * sc).collect();
                d.ht(Ink::Pink, tone, &ring);
            }
            d.ht_add(Ink::Key, 0.16, &ellipse(v2(0.08 * s, base + 0.05 * s), 0.52 * s, 0.075 * s, 0.0));
            // Crackled top: the crust split into plates showing lighter crumb.
            let cracks = [
                vec![v2(-0.22, -0.26), v2(-0.1, -0.23), v2(0.0, -0.3), v2(0.14, -0.26), v2(0.24, -0.3)],
                vec![v2(-0.02, -0.24), v2(0.01, -0.16), v2(-0.05, -0.09)],
                vec![v2(0.1, -0.33), v2(0.16, -0.38)],
                vec![v2(-0.31, -0.12), v2(-0.22, -0.18), v2(-0.13, -0.15)],
            ];
            for c in cracks.iter() {
                let pts: Vec<V2> = chaikin(&c.iter().map(|p| *p * s).collect::<Vec<_>>(), 2, false);
                let gap = taper(&pts, |t| 0.014 * s * arch(t, 0.6));
                d.knock(&gap);
                d.fill_p(Paint::solid(Ink::Yellow, 0.55).add(), &gap);
                d.fill_p(Paint::ht(Ink::Pink, 0.15).add(), &gap);
                let lip: Vec<V2> = pts.iter().map(|p| *p + v2(0.004, 0.011) * s).collect();
                d.fill_p(Paint::solid(Ink::Key, 0.55).add(), &taper(&lip, |t| 0.006 * s * arch(t, 0.6)));
            }
            // Sugar sparkles.
            for _ in 0..(12.0 + 14.0 * k) as usize {
                let p = v2(rng.range(-0.34, 0.34), rng.range(-0.36, -0.04)) * s;
                d.knock(&ellipse(p, 0.009 * s, 0.006 * s, rng.range(0.0, PI)));
            }
        });
        // Blueberries, half sunk into the crumb, one burst.
        let spots = [
            v2(-0.26, -0.1),
            v2(0.04, -0.33),
            v2(0.23, -0.14),
            v2(-0.07, -0.05),
            v2(0.33, 0.0),
            v2(-0.34, 0.01),
            v2(-0.16, -0.27),
        ];
        for (i, p) in spots.iter().enumerate() {
            let jitter = v2(rng.range(-0.025, 0.025), rng.range(-0.02, 0.02));
            berry(d, (*p + jitter) * s, s * rng.range(0.036, 0.052), i == 2, k);
        }
        d.outline(Ink::Key, OUTER * 0.85 * k, &dome);
    } else {
        // Glossy batter domed just over the rim, berries peeking through.
        let batter = {
            let mut p = Vec::new();
            for i in 0..=28 {
                let a = PI + PI * i as f32 / 28.0;
                p.push(v2(a.cos() * rim_w * 1.03, rim_y + 0.035 * s + a.sin() * 0.17 * s));
            }
            p.extend(rim_front.iter().rev().map(|q| *q + v2(0.0, 0.006 * s)));
            chaikin(&p, 2, true)
        };
        d.backing(&batter);
        d.fill(Ink::Yellow, 0.42, &batter);
        d.fill(Ink::Pink, 0.06, &batter);
        d.clipped(&batter, |d| {
            d.fill_p(
                Paint::solid(Ink::Pink, 0.12).add(),
                &ellipse(v2(0.08 * s, rim_y + 0.07 * s), 0.36 * s, 0.07 * s, 0.0),
            );
            d.knock_p(
                0.75,
                Screen::Solid,
                PLATES_COLOR,
                &ellipse(v2(-0.13 * s, rim_y - 0.06 * s), 0.13 * s, 0.035 * s, -0.12),
            );
            d.knock_p(
                0.9,
                Screen::Solid,
                PLATES_COLOR,
                &ellipse(v2(-0.19 * s, rim_y - 0.075 * s), 0.04 * s, 0.014 * s, -0.12),
            );
        });
        for (p, r) in [
            (v2(-0.2, -0.04), 0.042),
            (v2(0.1, -0.08), 0.038),
            (v2(0.25, -0.01), 0.04),
            (v2(-0.02, 0.0), 0.034),
        ] {
            berry(d, p * s, s * r, false, k);
        }
        d.outline(Ink::Key, INNER * k, &batter);
    }
}

/// A blueberry: purple (blue over pink), a crown dimple and a highlight.
fn berry(d: &mut DrawList, c: V2, r: f32, burst: bool, k: f32) {
    if burst {
        d.fill_p(Paint::ht(Ink::Blue, 0.55).add(), &ellipse(c + v2(r * 0.4, r * 0.6), r * 1.7, r * 1.1, 0.3));
        d.fill_p(Paint::ht(Ink::Pink, 0.55).add(), &ellipse(c + v2(r * 0.4, r * 0.6), r * 1.7, r * 1.1, 0.3));
    }
    let b = circle(c, r);
    d.knock(&b);
    d.fill(Ink::Blue, 1.0, &b);
    d.fill(Ink::Pink, 0.78, &b);
    d.fill(Ink::Key, 0.18, &b);
    d.fill_p(Paint::solid(Ink::Key, 0.25).add(), &ellipse(c + v2(r * 0.3, r * 0.35), r * 0.75, r * 0.6, 0.4));
    d.knock_p(0.8, Screen::Solid, PLATES_ALL, &ellipse(c + v2(-r * 0.35, -r * 0.35), r * 0.3, r * 0.2, -0.6));
    d.outline(Ink::Key, (DETAIL * 0.8 * k).max(0.8), &b);
    let crown = c + v2(r * 0.15, -r * 0.05);
    d.line(Ink::Key, (r * 0.18).max(0.7), &[crown + v2(-r * 0.2, 0.0), crown + v2(r * 0.2, 0.0)]);
    d.line(Ink::Key, (r * 0.18).max(0.7), &[crown + v2(0.0, -r * 0.2), crown + v2(0.0, r * 0.2)]);
}

// ---------------------------------------------------------------------------------------
// Cinnamon bun
// ---------------------------------------------------------------------------------------

fn bun(d: &mut DrawList, s: f32, seed: u32, baked: bool) {
    let k = lwk(s);
    let mut rng = Rng::new(seed as u64 * 13 + 7);
    let c = v2(0.0, -0.09 * s);
    let (rx, ry) = (0.44 * s, 0.27 * s);
    let side = 0.14 * s;
    let top = lumpy(c, rx, ry, 0.02, seed, 72);
    let body = with_side(&top, c, rx, side);
    let (dough_y, dough_p) = if baked { (0.88, 0.3) } else { (0.4, 0.06) };
    // Where the rolled strip ends: the spiral's outer end, and its seam down the side.
    let turns = 2.4;
    let phase = 0.4 + rng.range(-0.3, 0.3);
    let end_a = TAU * turns + phase;
    d.backing(&body);
    d.fill(Ink::Yellow, dough_y, &body);
    // The side wall: the outer strip of dough, browner towards the base, with its seam.
    d.clipped(&body, |d| {
        d.ht(Ink::Pink, dough_p + 0.3, &body);
        d.ht(Ink::Key, if baked { 0.2 } else { 0.05 }, &body);
        let base: Vec<V2> = top.iter().filter(|q| q.y > c.y).map(|q| *q + v2(0.0, side)).collect();
        if base.len() > 2 {
            let mut band = base.clone();
            band.sort_by(|a, b| a.x.total_cmp(&b.x));
            d.stroke_p(Paint::ht(Ink::Key, if baked { 0.35 } else { 0.1 }).add(), 0.05 * s, &band, false);
        }
        // The seam sits where the strip's end wraps round the front.
        let sx = (end_a.cos() * rx * 0.95).clamp(-0.3 * s, 0.3 * s);
        let yl = c.y + ry * (1.0 - (sx / rx).powi(2)).max(0.0).sqrt();
        let seam =
            quad_bezier(v2(sx, yl), v2(sx + 0.03 * s, yl + side * 0.5), v2(sx + 0.015 * s, yl + side), 6);
        d.stroke_p(
            Paint::solid(Ink::Pink, if baked { 0.6 } else { 0.35 }).add(),
            DETAIL * 2.0 * k,
            &seam,
            false,
        );
        d.stroke_p(
            Paint::solid(Ink::Key, if baked { 0.5 } else { 0.3 }).add(),
            DETAIL * 0.7 * k,
            &seam,
            false,
        );
        // Lit on the left flank.
        d.knock_p(
            0.3,
            Screen::Solid,
            PLATES_COLOR,
            &ellipse(v2(-0.36 * s, c.y + side * 0.8), 0.06 * s, 0.1 * s, 0.2),
        );
    });
    // The top face.
    d.knock(&top);
    d.fill(Ink::Yellow, dough_y, &top);
    d.ht(Ink::Pink, dough_p, &top);
    // The swirl: coil ridges lit on their top-left, cinnamon deep in the grooves.
    let spiral = |t: f32, off: f32| -> V2 {
        let a = t * TAU * turns + phase;
        let r = (0.1 + 0.9 * t) * (1.0 + off);
        c + v2(a.cos() * rx * 0.95 * r, a.sin() * ry * 0.95 * r)
    };
    let n = 160;
    let groove: Vec<V2> = (0..=n).map(|i| spiral(i as f32 / n as f32, 0.0)).collect();
    d.clipped(&top, |d| {
        let flank: Vec<V2> = (0..=n).map(|i| spiral(i as f32 / n as f32, -0.08)).collect();
        d.stroke_p(Paint::ht(Ink::Pink, if baked { 0.8 } else { 0.35 }).add(), 0.055 * s, &flank, false);
        let ridge: Vec<V2> = (0..=n).map(|i| spiral(i as f32 / n as f32, -0.22)).collect();
        d.stroke_p(Paint::solid(Ink::Yellow, if baked { 0.55 } else { 0.25 }).add(), 0.03 * s, &ridge, false);
        d.knock_p(0.45, Screen::Solid, PLATES_COLOR, &taper(&ridge, |t| 0.008 * s * arch(t, 0.3)));
        d.stroke_p(Paint::solid(Ink::Key, if baked { 0.66 } else { 0.48 }).add(), 0.026 * s, &groove, false);
        d.stroke_p(Paint::solid(Ink::Pink, 0.8).add(), 0.026 * s, &groove, false);
    });
    d.outline(Ink::Key, DETAIL * 1.1 * k, &top);

    if baked {
        // Glaze piped from a spoon in switchbacks across the swirl: long passes with
        // rounded turns near the edge, a thick glossy ribbon that pools where it turns, and
        // one drip running over the front rim.
        let passes = 4;
        let tilt = -0.1;
        let level = |i: usize| (-0.62 + 1.24 * i as f32 / (passes - 1) as f32) * ry;
        let half = |dy: f32| rx * 0.8 * (1.0 - (dy / ry).powi(2)).max(0.0).sqrt();
        let mut pts: Vec<V2> = Vec::new();
        let r = (level(1) - level(0)) * 0.5;
        for i in 0..passes {
            let dy = level(i);
            // Each pass stops a turn's radius short of the edge, so the turn stays on top.
            let hw = (half(dy) - r * 0.9).max(r) * rng.range(0.88, 1.0);
            let (x0, x1) = if i % 2 == 0 { (-hw, hw) } else { (hw, -hw) };
            for j in 0..=10 {
                let u = j as f32 / 10.0;
                let x = x0 + (x1 - x0) * u;
                pts.push(v2(x, dy + (u * PI * 2.0 + i as f32).sin() * 0.01 * s));
            }
            if i + 1 < passes {
                // A round turn down to the next pass.
                let sgn = if i % 2 == 0 { 1.0 } else { -1.0 };
                for j in 1..12 {
                    let a = -PI * 0.5 + PI * j as f32 / 12.0;
                    pts.push(v2(x1 + sgn * a.cos() * r, dy + r + a.sin() * r));
                }
            }
        }
        let path: Vec<V2> = crate::geom::resample(&chaikin(&pts, 2, false), 120)
            .into_iter()
            .map(|p| c + p.rotate(tilt))
            .collect();
        // Thicker where the spoon slowed down to turn.
        let turn_at: Vec<f32> = (0..path.len())
            .map(|i| {
                let a = path[i.saturating_sub(2)];
                let m = path[i];
                let b = path[(i + 2).min(path.len() - 1)];
                let (u, v) = ((m - a).norm(), (b - m).norm());
                (1.0 - u.dot(v)).clamp(0.0, 1.0)
            })
            .collect();
        let np = path.len();
        let pool = |t: f32| {
            let i = ((t * (np - 1) as f32).round() as usize).min(np - 1);
            1.0 + 0.9 * turn_at[i].sqrt()
        };
        let w = 0.024 * s;
        let ribbon = taper(&path, |t| w * pool(t) * (0.55 + 0.45 * arch(t, 0.25)));
        // The drip: from the lowest swing near the front, over the rim and down the side.
        let lowest =
            path.iter().filter(|p| p.x.abs() < rx * 0.4).fold(path[0], |a, p| if p.y > a.y { *p } else { a });
        let rim = c.y + ry * (1.0 - (lowest.x / rx).powi(2)).max(0.0).sqrt();
        let len = side * rng.range(0.55, 0.8);
        let hw = w * 1.15;
        let drip = chaikin(
            &[
                lowest + v2(-hw * 1.2, -w),
                lowest + v2(hw * 1.2, -w),
                v2(lowest.x + hw, rim),
                v2(lowest.x + hw * 0.8, rim + len * 0.7),
                v2(lowest.x + hw * 0.9, rim + len),
                v2(lowest.x, rim + len + hw * 0.9),
                v2(lowest.x - hw * 0.9, rim + len),
                v2(lowest.x - hw * 0.8, rim + len * 0.7),
                v2(lowest.x - hw, rim),
            ],
            3,
            true,
        );
        let cap0 = circle(path[0], w * pool(0.0) * 0.55);
        let cap1 = circle(path[path.len() - 1], w * pool(1.0) * 0.55);
        let parts = [&ribbon, &drip, &cap0, &cap1];
        for p in parts {
            let shadow: Vec<V2> = p.iter().map(|q| *q + v2(0.007, 0.013) * s).collect();
            d.fill_p(Paint::ht(Ink::Key, 0.45).add(), &shadow);
        }
        // Outlines first, then the glaze over them: one seamless pour.
        for p in parts {
            d.stroke_p(Paint::solid(Ink::Key, 0.75), DETAIL * 1.6 * k, p, true);
        }
        for p in parts {
            d.knock(p);
            d.fill_p(Paint::solid(Ink::Yellow, 0.1).add(), p);
            d.fill_p(Paint::solid(Ink::Blue, 0.05).add(), p);
        }
        // Gloss: a bright streak along the ribbon's upper edge, cool shade underneath.
        let n = path.len();
        let nrm: Vec<V2> = (0..n)
            .map(|i| {
                let a = path[i.saturating_sub(1)];
                let b = path[(i + 1).min(n - 1)];
                let q = (b - a).norm().perp();
                if q.y > 0.0 { -q } else { q }
            })
            .collect();
        let under: Vec<V2> =
            (0..n).map(|i| path[i] - nrm[i] * (w * pool(i as f32 / (n - 1) as f32) * 0.45)).collect();
        d.fill_p(Paint::ht(Ink::Blue, 0.32).add(), &taper(&under, |t| w * 0.4 * pool(t) * arch(t, 0.3)));
        let over: Vec<V2> =
            (0..n).map(|i| path[i] + nrm[i] * (w * pool(i as f32 / (n - 1) as f32) * 0.3)).collect();
        for seg in over.chunks(9).step_by(2) {
            if seg.len() >= 3 {
                d.knock(&taper(seg, |t| w * 0.22 * arch(t, 0.8)));
            }
        }
        d.knock(&ellipse(v2(lowest.x - hw * 0.3, rim + len * 0.75), hw * 0.28, hw * 0.45, 0.0));
    }
    d.outline(Ink::Key, OUTER * 0.85 * k, &body);
}

// ---------------------------------------------------------------------------------------
// Bagel
// ---------------------------------------------------------------------------------------

fn bagel(d: &mut DrawList, s: f32, seed: u32, baked: bool) {
    let k = lwk(s);
    let mut rng = Rng::new(seed as u64 * 19 + 3);
    let c = v2(0.0, -0.04 * s);
    let (rx, ry) = (0.45 * s, 0.31 * s);
    let side = 0.085 * s;
    let outer = lumpy(c, rx, ry, 0.018, seed, 72);
    let hole_c = c + v2(0.0, -0.025 * s);
    let hole = ellipse(hole_c, 0.14 * s, 0.08 * s, 0.0);
    let body = with_side(&outer, c, rx, side);
    let (y, p, key): (f32, f32, f32) = if baked { (0.92, 0.5, 0.12) } else { (0.36, 0.06, 0.02) };
    d.backing(&body);
    d.fill(Ink::Yellow, y, &body);
    d.ht(Ink::Pink, (p + 0.22).min(0.95), &body);
    d.ht(Ink::Key, key + 0.14, &body);
    // The top of the ring, lit from the top-left; it rolls down into the hole and the side.
    d.knock(&outer);
    d.fill(Ink::Yellow, y, &outer);
    d.clipped(&outer, |d| {
        let lit = c + v2(-0.14 * s, -0.12 * s);
        for (sc, tone) in [(1.0f32, p + 0.12), (0.8, p), (0.6, p - 0.1)] {
            let ring: Vec<V2> = outer.iter().map(|q| lit + (*q - lit) * sc).collect();
            d.ht(Ink::Pink, tone.max(0.02), &ring);
        }
        d.ht(Ink::Key, key, &outer);
        // The ring rounds down into the hole: a soft shaded collar.
        d.ht_add(
            Ink::Pink,
            (p + 0.25).min(0.95),
            &ellipse(hole_c + v2(0.0, 0.01 * s), 0.2 * s, 0.13 * s, 0.0),
        );
        if baked {
            // Glossy boiled crust: a hard specular arc and a softer sheen.
            let arc: Vec<V2> = (0..=18)
                .map(|i| {
                    let a = PI * (1.02 + 0.5 * i as f32 / 18.0);
                    c + v2(a.cos() * rx * 0.7, a.sin() * ry * 0.66)
                })
                .collect();
            d.knock_p(0.9, Screen::Solid, PLATES_ALL, &taper(&arc, |t| 0.022 * s * arch(t, 0.7)));
            d.knock_p(
                0.35,
                Screen::Solid,
                PLATES_COLOR,
                &ellipse(c + v2(-0.17 * s, -0.12 * s), 0.12 * s, 0.05 * s, -0.4),
            );
            d.fill_p(
                Paint::ht(Ink::Key, 0.22).add(),
                &ellipse(c + v2(0.22 * s, 0.14 * s), 0.28 * s, 0.13 * s, 0.3),
            );
        } else {
            // A soft satin sheen and a dusting of flour on the raw dough.
            d.knock_p(
                0.3,
                Screen::Solid,
                PLATES_COLOR,
                &ellipse(c + v2(-0.14 * s, -0.1 * s), 0.18 * s, 0.07 * s, -0.3),
            );
            d.knock_p(
                0.25,
                Screen::Halftone,
                PLATES_COLOR,
                &ellipse(c + v2(0.06 * s, 0.02 * s), 0.34 * s, 0.2 * s, 0.1),
            );
        }
    });
    // The hole: its far inner wall in shade, the table showing below.
    d.knock(&hole);
    d.clipped(&hole, |d| {
        d.fill(Ink::Yellow, y, &hole);
        d.ht(Ink::Pink, (p + 0.28).min(0.95), &hole);
        d.ht(Ink::Key, key + 0.3, &hole);
        d.knock(&ellipse(hole_c + v2(0.0, 0.045 * s), 0.15 * s, 0.08 * s, 0.0));
        d.ht_add(Ink::Blue, 0.34, &ellipse(hole_c + v2(0.02 * s, 0.05 * s), 0.13 * s, 0.05 * s, 0.0));
    });
    d.outline(Ink::Key, INNER * 0.85 * k, &hole);
    d.stroke_p(Paint::solid(Ink::Key, 0.4), DETAIL * 0.7 * k, &outer, true);

    if baked {
        // Seeds: sesame and poppy on the top of the ring (not in the hole).
        let mut sesame = Vec::new();
        let mut poppy = Vec::new();
        let mut placed: Vec<V2> = Vec::new();
        let mut tries = 0;
        while (sesame.len() + poppy.len()) < 30 && tries < 500 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = rng.range(0.44, 0.9);
            let q = c + v2(a.cos() * rx * r, a.sin() * ry * r);
            if placed.iter().any(|o| o.dist(q) < 0.05 * s) {
                continue;
            }
            placed.push(q);
            if rng.chance(0.68) {
                sesame.push(ellipse(q, 0.03 * s, 0.016 * s, a + PI * 0.5 + rng.range(-0.6, 0.6)));
            } else {
                poppy.push(circle(q, 0.011 * s));
            }
        }
        for sd in &sesame {
            let sh: Vec<V2> = sd.iter().map(|q| *q + v2(0.005, 0.007) * s).collect();
            d.fill_p(Paint::solid(Ink::Key, 0.45).add(), &sh);
            d.knock(sd);
            d.fill(Ink::Yellow, 0.26, sd);
            d.stroke_p(Paint::solid(Ink::Key, 0.6), (DETAIL * 0.45 * k).max(0.6), sd, true);
        }
        for pp in &poppy {
            d.fill(Ink::Key, 0.95, pp);
            d.fill(Ink::Blue, 0.6, pp);
        }
    }
    d.outline(Ink::Key, OUTER * 0.85 * k, &body);
}
