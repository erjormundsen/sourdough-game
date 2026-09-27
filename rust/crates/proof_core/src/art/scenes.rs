//! Full-screen backdrops (720-wide reference canvas, any height) and the counters drawn in
//! front of the gameplay layer.
//!
//! Each scene is built back to front — far view through a window, the wall and its
//! furniture, then the surface the game happens on — and is *composed around the gameplay*:
//! busy detail lives at the edges and up high, the play areas stay calm. Screens place
//! objects on the surfaces returned by the layout functions below ([`bench_top`],
//! [`jar_shelf`], [`jar_slots`], [`counter_top`], [`case_rows`], [`case_cols`]), so every
//! jar, loaf and customer stands on something real.
//!
//! Heights: every function takes the *effective* height `h` of the area below the safe-area
//! inset (screens draw the backdrop at `y = top` with `h = screen_h - top`). Walls extend
//! well above `y = 0` so a notch never shows bare paper.

use super::props::{clip_polyline_convex, clip_rect, glow, ramp, ramp_p, ring_poly, wood};
use super::style::{DETAIL, INNER, OUTER, contact_shadow};
use crate::draw::{DrawList, PLATES_COLOR, Paint, Screen};
use crate::geom::{
    Rect, V2, arc, capsule, chaikin, circle, ellipse, quad_bezier, rect, rect_poly, rounded_rect, scallop,
    soft_star, translate, v2,
};
use crate::ink::Ink;
use crate::rng::hash01;
use std::f32::consts::{PI, TAU};

pub const W: f32 = 720.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backdrop {
    /// Morning: the bakehouse — sunrise window, tiled backsplash, marble bench.
    Bakehouse,
    /// Shop: the shop floor — big street window, lamps, chalkboard, glass display case.
    Shopfront,
    /// Evening: the pantry — moonlit window, lamp-lit starter shelf, prep counter.
    Pantry,
}

/// State-dependent dressing: what stands where, so décor never collides with it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dressing {
    /// Centres of the starter jars standing on this scene's jar shelf.
    pub jars: Vec<f32>,
}

// =========================================================================================
// Layout contracts
// =========================================================================================

/// Extra height over the 1280 reference (tall phones).
fn extra(h: f32) -> f32 {
    (h - 1280.0).max(0.0)
}

/// Bakehouse: where the wall meets the marble bench (the bench fills everything below).
/// The bench's work area is anchored to the bottom of the screen; taller phones get a
/// taller wall (more scene), never an empty gap.
pub fn bench_top(h: f32) -> f32 {
    bake_layout(h).dough.y - 210.0 - extra(h) * 0.05
}

/// The surface starter jars stand on (Bakehouse: the wall shelf left of the window;
/// Pantry: the long lamp-lit shelf). Shopfront has none and returns its counter top.
pub fn jar_shelf(b: Backdrop, h: f32) -> f32 {
    match b {
        Backdrop::Bakehouse => bench_top(h) - 46.0,
        Backdrop::Shopfront => counter_top(b, h),
        Backdrop::Pantry => 742.0 + extra(h) * 0.72,
    }
}

/// Jar scale used on each scene's shelf.
pub fn jar_scale(b: Backdrop) -> f32 {
    match b {
        Backdrop::Bakehouse => 0.4,
        _ => 0.8,
    }
}

/// Centres for `n` jars on the scene's jar shelf.
pub fn jar_slots(b: Backdrop, n: usize) -> Vec<f32> {
    match b {
        Backdrop::Bakehouse => [56.0, 130.0, 204.0].iter().take(n.clamp(1, 3)).copied().collect(),
        _ => match n {
            0 | 1 => vec![360.0],
            2 => vec![250.0, 470.0],
            _ => vec![150.0, 360.0, 570.0],
        },
    }
}

/// Where each backdrop's counter top sits for an area `h` tall. Bakehouse: the bench's
/// front lip (the bottom action bar sits on the apron below it); Shopfront: the display
/// case top; Pantry: the prep counter top.
pub fn counter_top(b: Backdrop, h: f32) -> f32 {
    match b {
        Backdrop::Bakehouse => h - 132.0,
        Backdrop::Shopfront => h - 368.0 - extra(h) * 0.22,
        Backdrop::Pantry => h - 326.0,
    }
}

/// Display-case shelf board, below a row's centre line.
pub const CASE_SHELF: f32 = 54.0;
/// Loaf radius that sits on a display-case shelf (its shadow lands on the board).
pub const CASE_LOAF_R: f32 = 58.0;
/// Treat size for display-case stacks.
pub const CASE_TREAT: f32 = 112.0;

/// Shopfront: the y centres of the two display-case shelves' goods (loaves of radius
/// [`CASE_LOAF_R`] sit with their base on the shelf board, [`CASE_SHELF`] below the centre).
pub fn case_rows(h: f32) -> [f32; 2] {
    let top = counter_top(Backdrop::Shopfront, h);
    let gap = 142.0 + extra(h) * 0.12;
    let r1 = top + 108.0 + extra(h) * 0.04;
    [r1, r1 + gap]
}

/// Shopfront: the x centres of the four display-case columns.
pub fn case_cols() -> [f32; 4] {
    [100.0, 273.0, 447.0, 620.0]
}

/// The Morning stage on the Bakehouse bench: where the dough, trays, bannetons, Toasty and
/// the crust gauge go. Extra height on tall phones is shared out between the gaps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakeLayout {
    /// Centre of the dough and its bread board.
    pub dough: V2,
    pub board_r: f32,
    /// Centres of the Dress and Score trays.
    pub trays: [f32; 2],
    /// Centre line of the waiting bannetons.
    pub bannetons: f32,
    /// Toasty's feet (bottom centre), on the marble.
    pub oven: V2,
    /// Centre of the crust gauge above Toasty.
    pub gauge: V2,
    /// Centre line of the bottom action bar (on the bench apron).
    pub bar: f32,
}

pub fn bake_layout(h: f32) -> BakeLayout {
    let e = extra(h);
    let slab = counter_top(Backdrop::Bakehouse, h);
    let t2 = slab - 74.0 - e * 0.15;
    let t1 = t2 - 102.0;
    let dough = v2(360.0, t1 - 272.0 - e * 0.1);
    let bt = dough.y - 210.0 - e * 0.05;
    // Toasty (drawn at 1.15×) stands near the front of the bench; the crust gauge hangs
    // just above its chimney steam.
    let oven = v2(360.0, slab - 34.0);
    let gauge_y = (bt + 86.0).max(oven.y - super::oven::OVEN_H * 1.15 - 160.0);
    BakeLayout {
        dough,
        board_r: 190.0,
        trays: [t1, t2],
        bannetons: dough.y,
        oven,
        gauge: v2(360.0, gauge_y),
        bar: slab + 72.0,
    }
}

/// Positions for `n` waiting bannetons: a column (or two) on the bench left of the board.
pub fn banneton_slots(h: f32, n: usize) -> (Vec<V2>, f32) {
    let c = bake_layout(h).dough;
    if n <= 3 {
        let ys = [-112.0, 0.0, 112.0];
        let start = (3 - n.max(1)) as f32 * 0.5;
        ((0..n).map(|i| v2(84.0, c.y + ys[0] + (start + i as f32) * 112.0)).collect(), 30.0)
    } else {
        let rows = n.div_ceil(2);
        let top = c.y - (rows as f32 - 1.0) * 0.5 * 104.0;
        (
            (0..n).map(|i| v2(if i % 2 == 0 { 54.0 } else { 122.0 }, top + (i / 2) as f32 * 104.0)).collect(),
            26.0,
        )
    }
}

/// Pantry: the prep counter's work surface (objects stand on its back half).
pub fn counter_surface(h: f32) -> f32 {
    counter_top(Backdrop::Pantry, h) + 44.0
}

// =========================================================================================
// Shared scenery pieces
// =========================================================================================

/// The wall from just above the top of the area down to `bottom` (walls stop where a
/// bench, wainscot or counter takes over, so nothing is painted twice).
fn wall_to(bottom: f32) -> Rect {
    rect(-12.0, -40.0, W + 24.0, bottom + 40.0)
}

/// Bunting swag from `a` to `b` sagging by `sag`, flags cycling through `inks`.
fn bunting(d: &mut DrawList, a: V2, b: V2, sag: f32, flags: usize, inks: &[Ink], seed: u32) {
    let mid = a.lerp(b, 0.5) + v2(0.0, sag * 2.0);
    let string = quad_bezier(a, mid, b, 24);
    d.line(Ink::Key, 1.8, &string);
    for i in 0..flags {
        let t0 = (i as f32 + 0.12) / flags as f32;
        let t1 = (i as f32 + 0.88) / flags as f32;
        let p0 = bez(a, mid, b, t0);
        let p1 = bez(a, mid, b, t1);
        let tip = p0.lerp(p1, 0.5) + v2(0.0, 30.0) + v2((hash01(seed, i as u32) - 0.5) * 3.0, 0.0);
        let flag = vec![p0, p1, tip];
        let ink = inks[i % inks.len()];
        d.backing(&flag);
        d.fill(ink, 0.82, &flag);
        if i % 2 == 0 {
            for k in 0..2 {
                let c = p0.lerp(p1, 0.35 + 0.3 * k as f32).lerp(tip, 0.3);
                d.knock_color(&circle(c, 2.2));
            }
        } else {
            d.ht(Ink::Key, 0.14, &flag);
        }
        d.stroke_p(Paint::solid(Ink::Key, 0.9), DETAIL, &flag, true);
    }
}

fn bez(a: V2, c: V2, b: V2, t: f32) -> V2 {
    let u = 1.0 - t;
    a * (u * u) + c * (2.0 * u * t) + b * (t * t)
}

/// An arched window opening: returns the glass outline for `r`.
fn arch(r: Rect) -> Vec<V2> {
    let rad = r.w * 0.5;
    let mut p = vec![v2(r.x, r.y + r.h)];
    p.extend(arc(v2(r.x + rad, r.y + rad), rad, PI, TAU));
    p.push(v2(r.x + r.w, r.y + r.h));
    p
}

/// Frame, mullions and sill around a window opening `glass` (already filled with a view).
fn window_frame(
    d: &mut DrawList,
    glass: &[V2],
    r: Rect,
    frame_dark: f32,
    mullion: bool,
    transom: Option<f32>,
) {
    let cx = r.x + r.w * 0.5;
    // Frame band: a thick outline in wood, drawn as a stroke with a key edge on both sides.
    d.stroke_p(Paint::solid(Ink::Key, 1.0), 20.0, glass, true);
    d.knock_line(1.0, 14.0, glass, true);
    if frame_dark < 0.35 {
        // Painted frame: pale blue-white instead of wood.
        d.stroke_p(Paint::solid(Ink::Yellow, 0.12), 14.0, glass, true);
        d.stroke_p(Paint::solid(Ink::Blue, 0.3), 14.0, glass, true);
    } else {
        d.stroke_p(Paint::solid(Ink::Yellow, 0.5 + 0.3 * frame_dark), 14.0, glass, true);
        d.stroke_p(Paint::solid(Ink::Pink, 0.18 + 0.3 * frame_dark), 14.0, glass, true);
    }
    d.clipped(glass, |d| {
        if mullion {
            let bar = rect_poly(rect(cx - 5.0, r.y - 10.0, 10.0, r.h + 20.0));
            d.fill(Ink::Key, 1.0, &rect_poly(rect(cx - 6.5, r.y - 10.0, 13.0, r.h + 20.0)));
            d.knock(&bar);
            d.fill(Ink::Yellow, 0.5 + 0.3 * frame_dark, &bar);
            d.fill(Ink::Pink, 0.18 + 0.3 * frame_dark, &bar);
        }
        if let Some(ty) = transom {
            let bar = rect_poly(rect(r.x - 10.0, ty - 5.0, r.w + 20.0, 10.0));
            d.fill(Ink::Key, 1.0, &rect_poly(rect(r.x - 10.0, ty - 6.5, r.w + 20.0, 13.0)));
            d.knock(&bar);
            if frame_dark < 0.35 {
                d.fill(Ink::Blue, 0.3, &bar);
                d.fill(Ink::Yellow, 0.12, &bar);
            } else {
                d.fill(Ink::Yellow, 0.5 + 0.3 * frame_dark, &bar);
                d.fill(Ink::Pink, 0.18 + 0.3 * frame_dark, &bar);
            }
        }
        // Glass glints.
        for (i, k) in [0.18f32, 0.3].iter().enumerate() {
            let x = r.x + r.w * k;
            let streak = vec![
                v2(x, r.y + r.h * 0.25),
                v2(x + 16.0 + i as f32 * 8.0, r.y + r.h * 0.25),
                v2(x - 30.0 + i as f32 * 8.0, r.y + r.h * 0.62),
                v2(x - 46.0, r.y + r.h * 0.62),
            ];
            d.knock_p(0.45, Screen::Solid, PLATES_COLOR, &streak);
        }
    });
}

/// A wall shelf board with a front edge; objects stand on `y`.
fn shelf_board(d: &mut DrawList, y: f32, x0: f32, x1: f32, seed: u32) {
    contact_shadow(d, v2((x0 + x1) * 0.5, y + 26.0), (x1 - x0) * 0.48, 7.0);
    let top = rect_poly(rect(x0, y - 7.0, x1 - x0, 7.0));
    let front = rounded_rect(rect(x0, y, x1 - x0, 15.0), 3.0);
    d.backing(&top);
    wood(d, &top, 0.3, 0.0, seed);
    d.backing(&front);
    wood(d, &front, 0.6, 0.0, seed + 1);
    d.outline(Ink::Key, INNER, &front);
    d.stroke_p(
        Paint::solid(Ink::Key, 1.0),
        INNER,
        &[v2(x0, y - 7.0), v2(x1, y - 7.0), v2(x1, y), v2(x0, y), v2(x0, y - 7.0)],
        true,
    );
}

/// A carved wooden bracket under a shelf at `x` (shelf front at `y`).
fn corbel(d: &mut DrawList, x: f32, y: f32, s: f32) {
    let p = chaikin(
        &[
            v2(x - 7.0 * s, y),
            v2(x + 7.0 * s, y),
            v2(x + 7.0 * s, y + 18.0 * s),
            v2(x, y + 44.0 * s),
            v2(x - 7.0 * s, y + 38.0 * s),
        ],
        2,
        true,
    );
    d.backing(&p);
    wood(d, &p, 0.62, PI * 0.5, 7);
    d.outline(Ink::Key, DETAIL + 0.8, &p);
}

/// A curly iron shelf bracket at `x` under a shelf front at `y`.
fn iron_bracket(d: &mut DrawList, x: f32, y: f32, dir: f32) {
    let pts = [v2(x, y), v2(x, y + 30.0), v2(x + dir * 26.0, y)];
    d.line(Ink::Key, 3.0, &[pts[0], pts[1]]);
    d.line(Ink::Key, 2.4, &quad_bezier(pts[1], v2(x + dir * 4.0, y + 4.0), pts[2], 8));
    let curl = arc(v2(x + dir * 9.0, y + 12.0), 5.0, 0.0, TAU * 0.8);
    d.line(Ink::Key, 1.8, &curl);
}

/// A potted plant standing at `base` (bottom centre), `s` ≈ height.
fn potted_plant(d: &mut DrawList, base: V2, s: f32, pot: Ink, seed: u32) {
    contact_shadow(d, base + v2(0.0, 1.0), s * 0.34, s * 0.06);
    let n = 7;
    for i in 0..n {
        let a = -PI * 0.5 + (i as f32 - (n - 1) as f32 * 0.5) * 0.3 + (hash01(seed, i) - 0.5) * 0.2;
        let len = s * (0.62 + 0.3 * hash01(seed ^ 3, i)) * (1.0 - 0.25 * ((i as f32 - 3.0).abs() / 3.0));
        let root = base + v2(0.0, -s * 0.36);
        let tip = root + V2::from_angle(a) * len;
        let leaf = crate::geom::lens_along(
            &[root, root.lerp(tip, 0.5) + V2::from_angle(a + 0.3) * (s * 0.04), tip],
            s * 0.2,
            0.15,
        );
        d.backing(&leaf);
        d.fill(Ink::Yellow, 0.8, &leaf);
        d.fill(Ink::Blue, 0.62, &leaf);
        if i % 2 == 0 {
            d.ht(Ink::Key, 0.14, &leaf);
        }
        d.stroke_p(Paint::solid(Ink::Key, 1.0), DETAIL, &leaf, true);
        d.stroke_p(Paint::solid(Ink::Key, 0.5), 1.2, &[root.lerp(tip, 0.15), root.lerp(tip, 0.8)], false);
    }
    let pot_poly = chaikin(
        &[
            base + v2(-s * 0.24, -s * 0.38),
            base + v2(s * 0.24, -s * 0.38),
            base + v2(s * 0.19, 0.0),
            base + v2(-s * 0.19, 0.0),
        ],
        1,
        true,
    );
    d.backing(&pot_poly);
    d.fill(pot, 0.75, &pot_poly);
    ramp(d, Ink::Key, &pot_poly, base + v2(-s * 0.24, 0.0), base + v2(s * 0.24, 0.0), 0.0, 0.2, 3);
    d.outline(Ink::Key, INNER, &pot_poly);
    let lip = rounded_rect(rect(base.x - s * 0.27, base.y - s * 0.43, s * 0.54, s * 0.1), 3.0);
    d.backing(&lip);
    d.fill(pot, 0.85, &lip);
    d.outline(Ink::Key, DETAIL + 0.8, &lip);
}

/// A tin canister with a lid and a round label, base at `base`.
fn canister(d: &mut DrawList, base: V2, w: f32, h: f32, ink: Ink) {
    contact_shadow(d, base + v2(0.0, 1.0), w * 0.6, 6.0);
    let body = rounded_rect(rect(base.x - w * 0.5, base.y - h, w, h), 6.0);
    d.backing(&body);
    d.fill(ink, 0.62, &body);
    ramp(d, Ink::Key, &body, base + v2(-w * 0.5, 0.0), base + v2(w * 0.5, 0.0), 0.0, 0.28, 4);
    d.knock_p(
        0.6,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(base + v2(-w * 0.3, -h * 0.8), base + v2(-w * 0.3, -h * 0.25), w * 0.05),
    );
    d.outline(Ink::Key, INNER, &body);
    let lid = rounded_rect(rect(base.x - w * 0.56, base.y - h - 10.0, w * 1.12, 13.0), 5.0);
    d.backing(&lid);
    d.fill(ink, 0.8, &lid);
    d.ht(Ink::Key, 0.3, &lid);
    d.outline(Ink::Key, DETAIL + 0.8, &lid);
    let knob = circle(base + v2(0.0, -h - 13.0), 4.5);
    d.backing(&knob);
    d.fill(Ink::Yellow, 0.9, &knob);
    d.outline(Ink::Key, DETAIL, &knob);
    let label = circle(base + v2(0.0, -h * 0.5), w * 0.3);
    d.knock(&label);
    d.fill(Ink::Yellow, 0.12, &label);
    d.stroke_p(Paint::solid(Ink::Key, 0.9), DETAIL * 0.9, &label, true);
    let lc = base + v2(0.0, -h * 0.5);
    d.line(Ink::Key, 1.6, &[lc + v2(0.0, w * 0.18), lc + v2(0.0, -w * 0.18)]);
    for k in 0..3 {
        for sx in [-1.0f32, 1.0] {
            d.fill(Ink::Key, 1.0, &ellipse(lc + v2(sx * 3.0, w * 0.06 - k as f32 * 4.6), 2.0, 3.4, sx * 0.5));
        }
    }
}

/// A short stack of books lying down, base at `base`.
fn books(d: &mut DrawList, base: V2, seed: u32) {
    contact_shadow(d, base + v2(0.0, 1.0), 44.0, 6.0);
    let inks = [Ink::Pink, Ink::Blue, Ink::Yellow];
    let mut y = base.y;
    for i in 0..3u32 {
        let w = 72.0 - i as f32 * 8.0 + hash01(seed, i) * 6.0;
        let hgt = 15.0 + 4.0 * hash01(seed ^ 2, i);
        let x = base.x - w * 0.5 + (hash01(seed ^ 4, i) - 0.5) * 8.0;
        let book = rounded_rect(rect(x, y - hgt, w, hgt), 3.0);
        d.backing(&book);
        d.fill(inks[(i + seed) as usize % 3], 0.7, &book);
        d.ht(Ink::Key, 0.12, &book);
        d.knock_color(&rect_poly(rect(x + w - 10.0, y - hgt + 3.0, 6.0, hgt - 6.0)));
        d.stroke_p(
            Paint::solid(Ink::Key, 1.0),
            DETAIL,
            &[v2(x + 8.0, y - hgt * 0.5), v2(x + w * 0.45, y - hgt * 0.5)],
            false,
        );
        d.outline(Ink::Key, DETAIL + 0.8, &book);
        y -= hgt;
    }
}

/// A glass milk bottle, base at `base`.
fn bottle(d: &mut DrawList, base: V2) {
    contact_shadow(d, base + v2(0.0, 1.0), 20.0, 5.0);
    let body = chaikin(
        &[
            base + v2(-16.0, 0.0),
            base + v2(-17.0, -46.0),
            base + v2(-9.0, -62.0),
            base + v2(-8.0, -78.0),
            base + v2(8.0, -78.0),
            base + v2(9.0, -62.0),
            base + v2(17.0, -46.0),
            base + v2(16.0, 0.0),
        ],
        2,
        true,
    );
    d.backing(&body);
    d.fill(Ink::Blue, 0.12, &body);
    let milk = clip_rect(&body, rect(base.x - 30.0, base.y - 40.0, 60.0, 40.0));
    d.knock(&milk);
    d.stroke_p(Paint::solid(Ink::Key, 0.5), 1.2, &[base + v2(-15.0, -40.0), base + v2(15.0, -40.0)], false);
    d.knock_p(
        0.8,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(base + v2(-9.0, -54.0), base + v2(-10.0, -12.0), 2.4),
    );
    d.outline(Ink::Key, DETAIL + 0.8, &body);
    let cap = rounded_rect(rect(base.x - 9.5, base.y - 84.0, 19.0, 8.0), 3.0);
    d.backing(&cap);
    d.fill(Ink::Pink, 0.85, &cap);
    d.outline(Ink::Key, DETAIL, &cap);
}

/// A stoneware crock with a wooden spoon, base at `base`.
fn crock(d: &mut DrawList, base: V2, ink: Ink) {
    contact_shadow(d, base + v2(0.0, 1.0), 30.0, 5.0);
    let spoon = capsule(base + v2(6.0, -52.0), base + v2(20.0, -84.0), 3.2);
    d.backing(&spoon);
    wood(d, &spoon, 0.4, -1.2, 3);
    d.outline(Ink::Key, DETAIL, &spoon);
    let body = chaikin(
        &[
            base + v2(-26.0, 0.0),
            base + v2(-30.0, -40.0),
            base + v2(-24.0, -52.0),
            base + v2(24.0, -52.0),
            base + v2(30.0, -40.0),
            base + v2(26.0, 0.0),
        ],
        2,
        true,
    );
    d.backing(&body);
    d.fill(Ink::Yellow, 0.3, &body);
    d.fill(ink, 0.12, &body);
    let band = clip_rect(&body, rect(base.x - 40.0, base.y - 40.0, 80.0, 9.0));
    d.fill(ink, 0.75, &band);
    ramp(d, Ink::Key, &body, base + v2(-30.0, 0.0), base + v2(30.0, 0.0), 0.0, 0.22, 4);
    d.outline(Ink::Key, INNER, &body);
}

/// Fill the free spans of a shelf with décor, keeping clear of `occupied` (x, half-width).
fn dress_shelf(
    d: &mut DrawList,
    y: f32,
    x0: f32,
    x1: f32,
    occupied: &[(f32, f32)],
    kit: &[(f32, u8)],
    seed: u32,
) {
    let mut spans = vec![(x0, x1)];
    for (cx, hw) in occupied {
        let (a, b) = (cx - hw - 14.0, cx + hw + 14.0);
        let mut next = Vec::new();
        for (s0, s1) in spans {
            if b <= s0 || a >= s1 {
                next.push((s0, s1));
            } else {
                if a > s0 {
                    next.push((s0, a));
                }
                if b < s1 {
                    next.push((b, s1));
                }
            }
        }
        spans = next;
    }
    let mut pool: Vec<(f32, u8)> = kit.to_vec();
    let rot = seed as usize % pool.len().max(1);
    pool.rotate_left(rot);
    let mut k = seed;
    for (s0, s1) in spans {
        // Take the items that fit (each kind once), then centre the group in the span.
        let mut items: Vec<(f32, u8)> = Vec::new();
        let mut used = 0.0;
        let mut rest = Vec::new();
        for it in pool.drain(..) {
            let need = it.0 + if items.is_empty() { 0.0 } else { 18.0 };
            if used + need <= (s1 - s0) - 8.0 && items.len() < 3 {
                used += need;
                items.push(it);
            } else {
                rest.push(it);
            }
        }
        pool = rest;
        let mut x = (s0 + s1) * 0.5 - used * 0.5;
        for (w, kind) in items {
            let base = v2(x + w * 0.5, y);
            draw_decor(d, base, kind, k);
            x += w + 18.0;
            k += 1;
        }
    }
}

fn draw_decor(d: &mut DrawList, base: V2, kind: u8, seed: u32) {
    match kind {
        0 => canister(d, base, 50.0, 74.0, Ink::Pink),
        1 => canister(d, base, 42.0, 56.0, Ink::Blue),
        2 => books(d, base, seed),
        3 => potted_plant(d, base, 86.0, Ink::Pink, seed),
        4 => bottle(d, base),
        5 => crock(d, base, Ink::Blue),
        _ => potted_plant(d, base, 64.0, Ink::Blue, seed),
    }
}

// =========================================================================================
// The Bakehouse (Dawn)
// =========================================================================================

fn bakehouse(d: &mut DrawList, h: f32, dress: &Dressing) {
    let bt = bench_top(h);
    let js = jar_shelf(Backdrop::Bakehouse, h);
    let tile_top = js + 16.0;
    let wall = rect_poly(wall_to(tile_top));
    // Warm plaster, lit from the window.
    d.fill(Ink::Yellow, 0.2, &wall);
    d.fill(Ink::Pink, 0.06, &wall);
    let wt = 196.0 + (bt - 490.0).max(0.0) * 0.45;
    let win = rect(250.0, wt, 220.0, js - wt);
    // A soft warm deepening toward the ceiling (a halftone ramp, no hard edge).
    let upper = rect_poly(rect(-12.0, -40.0, W + 24.0, win.y + win.h * 0.55 + 40.0));
    ramp(d, Ink::Pink, &upper, v2(0.0, 90.0), v2(0.0, win.y + win.h * 0.55), 0.16, 0.0, 6);

    // Backsplash tiles below the ledge line.
    let splash = rect(-12.0, tile_top, W + 24.0, bt - tile_top);
    d.fill(Ink::Blue, 0.1, &rect_poly(splash));
    let mut row = 0;
    let mut y = tile_top;
    while y < bt {
        d.stroke_p(Paint::solid(Ink::Blue, 0.6), 1.5, &[v2(-12.0, y), v2(W + 12.0, y)], false);
        let off = if row % 2 == 0 { 0.0 } else { 18.0 };
        let mut x = -22.0 + off;
        let hgt = (bt - y).min(18.0);
        while x < W + 12.0 {
            d.stroke_p(Paint::solid(Ink::Blue, 0.6), 1.5, &[v2(x, y), v2(x, y + hgt)], false);
            if hgt > 10.0 {
                d.knock_p(
                    0.8,
                    Screen::Solid,
                    PLATES_COLOR,
                    &capsule(v2(x + 6.0, y + 4.5), v2(x + 13.0, y + 4.5), 1.3),
                );
            }
            x += 36.0;
        }
        y += 18.0;
        row += 1;
    }

    // Bunting swags either side of the window.
    let by = 166.0 + (bt - 490.0).max(0.0) * 0.1;
    bunting(d, v2(-12.0, by), v2(226.0, by + 6.0), 11.0, 6, &[Ink::Pink, Ink::Yellow, Ink::Blue], 3);
    bunting(d, v2(494.0, by + 6.0), v2(732.0, by), 11.0, 6, &[Ink::Blue, Ink::Pink, Ink::Yellow], 5);

    // The sunrise window.
    let glass = arch(win);
    d.knock(&glass);
    let hz = win.y + win.h * 0.72;
    d.clipped(&glass, |d| {
        let sky = rect_poly(rect(win.x - 20.0, win.y - 20.0, win.w + 40.0, win.h + 40.0));
        ramp(d, Ink::Pink, &sky, v2(0.0, win.y), v2(0.0, hz), 0.6, 0.1, 9);
        ramp(d, Ink::Yellow, &sky, v2(0.0, win.y), v2(0.0, hz), 0.1, 0.95, 9);
        let sun_c = v2(360.0, hz + 6.0);
        for i in 0..9 {
            let a = PI + PI * (i as f32 + 0.5) / 9.0;
            let ray = vec![
                sun_c + V2::from_angle(a - 0.06) * 54.0,
                sun_c + V2::from_angle(a) * 130.0,
                sun_c + V2::from_angle(a + 0.06) * 54.0,
            ];
            d.knock_p(0.5, Screen::Solid, 0b0001, &ray);
        }
        let sun = circle(sun_c, 44.0);
        d.knock(&sun);
        d.fill(Ink::Yellow, 1.0, &sun);
        d.fill(Ink::Pink, 0.26, &sun);
        d.knock_p(0.8, Screen::Solid, 0b0001, &circle(sun_c + v2(-12.0, -14.0), 20.0));
        d.stroke_p(Paint::solid(Ink::Key, 0.8), DETAIL, &sun, true);
        // Distant hill with a windmill, then the near hill.
        let far = chaikin(
            &[
                v2(win.x - 20.0, hz + 4.0),
                v2(win.x + 40.0, hz - 22.0),
                v2(win.x + 110.0, hz - 12.0),
                v2(win.x + 180.0, hz - 32.0),
                v2(win.x + win.w + 20.0, hz - 6.0),
                v2(win.x + win.w + 20.0, win.y + win.h + 20.0),
                v2(win.x - 20.0, win.y + win.h + 20.0),
            ],
            3,
            true,
        );
        d.knock(&far);
        d.fill(Ink::Blue, 0.34, &far);
        d.fill(Ink::Yellow, 0.34, &far);
        d.stroke_p(Paint::solid(Ink::Key, 0.6), 1.4, &far, true);
        let mill = v2(win.x + 162.0, hz - 26.0);
        d.fill(
            Ink::Key,
            0.75,
            &[mill + v2(-5.5, 0.0), mill + v2(5.5, 0.0), mill + v2(3.0, -24.0), mill + v2(-3.0, -24.0)],
        );
        for k in 0..4 {
            let a = 0.4 + k as f32 * PI * 0.5;
            d.stroke_p(
                Paint::solid(Ink::Key, 0.75),
                2.0,
                &[mill + v2(0.0, -24.0), mill + v2(0.0, -24.0) + V2::from_angle(a) * 16.0],
                false,
            );
        }
        let near = chaikin(
            &[
                v2(win.x - 20.0, hz + 14.0),
                v2(win.x + 70.0, hz + 2.0),
                v2(win.x + 150.0, hz + 18.0),
                v2(win.x + win.w + 20.0, hz + 6.0),
                v2(win.x + win.w + 20.0, win.y + win.h + 20.0),
                v2(win.x - 20.0, win.y + win.h + 20.0),
            ],
            3,
            true,
        );
        d.knock(&near);
        d.fill(Ink::Yellow, 0.85, &near);
        d.fill(Ink::Blue, 0.6, &near);
        ramp(d, Ink::Blue, &near, v2(0.0, hz), v2(0.0, win.y + win.h), 0.6, 0.8, 3);
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 1.6, &near, true);
        // Hedgerow dots on the near hill.
        for i in 0..7u32 {
            let p = v2(win.x + 14.0 + i as f32 * 30.0 + hash01(81, i) * 8.0, hz + 16.0 + hash01(82, i) * 8.0);
            let bush = circle(p, 6.0 + 3.0 * hash01(83, i));
            d.fill(Ink::Blue, 0.9, &bush);
            d.stroke_p(Paint::solid(Ink::Key, 0.6), 1.2, &bush, true);
        }
        for (c, sz) in [(v2(win.x + 50.0, win.y + 70.0), 18.0), (v2(win.x + 160.0, win.y + 52.0), 14.0)] {
            let cl = scallop(c, sz, 7, 0.22);
            let cl: Vec<V2> = cl.iter().map(|p| v2(p.x, c.y + (p.y - c.y) * 0.62)).collect();
            d.knock(&cl);
            d.fill(Ink::Pink, 0.18, &clip_rect(&cl, rect(c.x - 30.0, c.y + 2.0, 60.0, 20.0)));
            d.stroke_p(Paint::solid(Ink::Key, 0.7), 1.5, &cl, true);
        }
        for (c, sz) in [(v2(win.x + 104.0, win.y + 96.0), 6.0), (v2(win.x + 120.0, win.y + 106.0), 4.5)] {
            d.line(Ink::Key, 1.5, &[c + v2(-sz, -sz * 0.5), c, c + v2(sz, -sz * 0.6)]);
        }
    });
    window_frame(d, &glass, win, 0.42, true, Some(win.y + win.w * 0.5 + 10.0));
    // Curtain rod and slim tie-back gingham curtains.
    let rod_y = win.y - 16.0;
    d.line(Ink::Key, 4.0, &[v2(win.x - 36.0, rod_y), v2(win.x + win.w + 36.0, rod_y)]);
    for x in [win.x - 40.0, win.x + win.w + 40.0] {
        let f = circle(v2(x, rod_y), 6.0);
        d.backing(&f);
        d.fill(Ink::Pink, 0.8, &f);
        d.outline(Ink::Key, DETAIL, &f);
    }
    for sx in [-1.0f32, 1.0] {
        let edge = if sx < 0.0 { win.x } else { win.x + win.w };
        let x_out = edge + sx * 30.0;
        let x_in = edge - sx * 22.0;
        let tie_y = win.y + win.h * 0.6;
        let tie_x = edge + sx * 6.0;
        let bottom = win.y + win.h - 4.0;
        let mut c = vec![v2(x_out, rod_y), v2(x_in, rod_y)];
        c.extend(quad_bezier(v2(x_in, rod_y), v2(x_in + sx * 2.0, tie_y - 50.0), v2(tie_x, tie_y), 10));
        c.extend(quad_bezier(
            v2(tie_x, tie_y),
            v2(tie_x - sx * 10.0, tie_y + 30.0),
            v2(tie_x - sx * 16.0, bottom),
            8,
        ));
        c.push(v2(x_out + sx * 6.0, bottom));
        d.backing(&c);
        d.fill(Ink::Yellow, 0.12, &c);
        d.clipped(&c, |d| {
            let x0 = x_out.min(x_in) - 12.0;
            for i in 0..7 {
                d.fill(
                    Ink::Pink,
                    0.34,
                    &rect_poly(rect(x0 + i as f32 * 12.0, rod_y, 6.0, bottom - rod_y + 10.0)),
                );
            }
            let mut y = rod_y + 3.0;
            while y < bottom + 10.0 {
                d.fill_p(Paint::solid(Ink::Pink, 0.34).add(), &rect_poly(rect(x0, y, 90.0, 6.0)));
                y += 12.0;
            }
            for k in 0..2 {
                let x = x_out + (x_in - x_out) * (0.3 + 0.35 * k as f32);
                d.stroke_p(
                    Paint::solid(Ink::Key, 0.35),
                    1.4,
                    &[v2(x, rod_y + 6.0), v2(x + (tie_x - x) * 0.7, tie_y - 8.0)],
                    false,
                );
            }
        });
        d.outline(Ink::Key, INNER, &c);
        let tie = capsule(v2(tie_x - 11.0, tie_y), v2(tie_x + 11.0, tie_y + 2.0), 4.5);
        d.backing(&tie);
        d.fill(Ink::Blue, 0.75, &tie);
        d.outline(Ink::Key, DETAIL, &tie);
    }

    // The ledge line: jar shelf (left), window sill, décor shelf (right), all at `js`.
    let sill = rounded_rect(rect(win.x - 18.0, js - 2.0, win.w + 36.0, 16.0), 5.0);
    d.backing(&sill);
    wood(d, &sill, 0.45, 0.0, 61);
    d.outline(Ink::Key, INNER, &sill);
    potted_plant(d, v2(win.x + win.w - 24.0, js - 2.0), 46.0, Ink::Blue, 9);
    shelf_board(d, js, 8.0, 234.0, 71);
    iron_bracket(d, 36.0, js + 15.0, 1.0);
    iron_bracket(d, 206.0, js + 15.0, -1.0);
    let occ: Vec<(f32, f32)> = dress.jars.iter().map(|x| (*x, 36.0)).collect();
    dress_shelf(d, js - 7.0, 14.0, 228.0, &occ, &[(56.0, 5), (44.0, 6), (34.0, 4)], 1);
    shelf_board(d, js, 486.0, 712.0, 73);
    iron_bracket(d, 514.0, js + 15.0, 1.0);
    iron_bracket(d, 684.0, js + 15.0, -1.0);
    crock(d, v2(530.0, js - 7.0), Ink::Pink);
    bowl_stack(d, v2(610.0, js - 7.0));
    d.with(crate::geom::Xf::at(v2(680.0, js - 7.0)).scaled(0.42), |d| {
        super::props::flour_bag(d, crate::content::Flour::Rye)
    });

    // Utensil rail (right) with hanging tools.
    let ry = win.y + 24.0;
    d.line(Ink::Key, 4.5, &[v2(506.0, ry), v2(706.0, ry)]);
    for x in [506.0, 706.0] {
        let cap = circle(v2(x, ry), 5.5);
        d.backing(&cap);
        d.fill(Ink::Yellow, 0.8, &cap);
        d.outline(Ink::Key, DETAIL, &cap);
    }
    utensils(d, ry);
    // A little framed loaf print on the upper left wall.
    frame_print(d, rect(46.0, by + 64.0, 104.0, 84.0));

    // The marble bench.
    let bench_r = rect(-12.0, bt, W + 24.0, counter_top(Backdrop::Bakehouse, h) + 20.0 - bt);
    marble(d, bench_r, 7);
    // Shadow where the bench meets the wall, and the bench's back edge.
    ramp(
        d,
        Ink::Key,
        &rect_poly(rect(-12.0, bt, W + 24.0, 24.0)),
        v2(0.0, bt),
        v2(0.0, bt + 24.0),
        0.26,
        0.0,
        4,
    );
    d.line(Ink::Key, INNER, &[v2(-12.0, bt), v2(W + 12.0, bt)]);
    // A flour dredger standing on the bench, right of the board.
    let lay = bake_layout(h);
    dredger(d, v2(662.0, lay.dough.y - 118.0));
}

/// A little stack of mixing bowls, base at `base`.
fn bowl_stack(d: &mut DrawList, base: V2) {
    contact_shadow(d, base + v2(0.0, 1.0), 40.0, 5.0);
    for (i, (w, ink)) in [(70.0f32, Ink::Blue), (58.0, Ink::Pink), (46.0, Ink::Yellow)].iter().enumerate() {
        let y = base.y - i as f32 * 16.0;
        let mut b = arc(v2(base.x, y - 22.0), w * 0.5, 0.0, PI);
        b.push(v2(base.x - w * 0.5, y - 22.0));
        let b: Vec<V2> = b.iter().map(|p| v2(p.x, y - 22.0 + (p.y - (y - 22.0)) * 0.72)).collect();
        d.backing(&b);
        d.fill(*ink, if *ink == Ink::Yellow { 0.8 } else { 0.6 }, &b);
        ramp(d, Ink::Key, &b, v2(base.x - w * 0.5, 0.0), v2(base.x + w * 0.5, 0.0), 0.0, 0.2, 3);
        d.outline(Ink::Key, DETAIL + 0.8, &b);
    }
}

/// A small framed print of a loaf hanging on a nail.
fn frame_print(d: &mut DrawList, r: Rect) {
    let nail = v2(r.x + r.w * 0.5, r.y - 22.0);
    d.line(Ink::Key, 1.4, &[v2(r.x + 16.0, r.y + 2.0), nail, v2(r.x + r.w - 16.0, r.y + 2.0)]);
    d.fill(Ink::Key, 1.0, &circle(nail, 3.0));
    let frame = rounded_rect(r, 4.0);
    d.ht(Ink::Key, 0.25, &translate(&frame, v2(4.0, 6.0)));
    d.backing(&frame);
    wood(d, &frame, 0.55, 0.0, 17);
    d.outline(Ink::Key, INNER, &frame);
    let pic = rect_poly(rect(r.x + 10.0, r.y + 10.0, r.w - 20.0, r.h - 20.0));
    d.knock(&pic);
    d.fill(Ink::Blue, 0.16, &pic);
    let c = v2(r.x + r.w * 0.5, r.y + r.h * 0.56);
    let loaf = ellipse(c, r.w * 0.26, r.h * 0.2, 0.0);
    d.fill(Ink::Yellow, 0.9, &loaf);
    d.fill(Ink::Pink, 0.45, &loaf);
    d.knock_line(
        1.0,
        3.0,
        &quad_bezier(c + v2(-r.w * 0.13, r.h * 0.1), c + v2(0.0, 0.0), c + v2(r.w * 0.13, -r.h * 0.1), 6),
        false,
    );
    d.stroke_p(Paint::solid(Ink::Key, 1.0), DETAIL, &loaf, true);
    d.stroke_p(Paint::solid(Ink::Key, 0.7), DETAIL * 0.8, &pic, true);
}

/// A flour dredger (shaker) standing at `base`.
fn dredger(d: &mut DrawList, base: V2) {
    contact_shadow(d, base + v2(0.0, 1.0), 34.0, 6.0);
    let body = rounded_rect(rect(base.x - 22.0, base.y - 58.0, 44.0, 58.0), 6.0);
    d.backing(&body);
    d.fill(Ink::Blue, 0.3, &body);
    ramp(d, Ink::Key, &body, base + v2(-22.0, 0.0), base + v2(22.0, 0.0), 0.0, 0.3, 4);
    d.knock_p(
        0.7,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(base + v2(-12.0, -48.0), base + v2(-12.0, -10.0), 2.5),
    );
    let band = rect_poly(rect(base.x - 22.0, base.y - 38.0, 44.0, 12.0));
    d.fill(Ink::Pink, 0.8, &band);
    d.outline(Ink::Key, INNER, &body);
    let mut lid = arc(v2(base.x, base.y - 58.0), 24.0, PI, TAU);
    lid.push(v2(base.x + 24.0, base.y - 56.0));
    lid.push(v2(base.x - 24.0, base.y - 56.0));
    let lid: Vec<V2> = lid.iter().map(|p| v2(p.x, base.y - 58.0 + (p.y - (base.y - 58.0)) * 0.7)).collect();
    d.backing(&lid);
    d.fill(Ink::Blue, 0.4, &lid);
    d.ht(Ink::Key, 0.2, &lid);
    for k in 0..5 {
        d.knock_color(&circle(
            v2(base.x - 12.0 + k as f32 * 6.0, base.y - 66.0 + (k as f32 - 2.0).abs() * 1.5),
            1.5,
        ));
    }
    d.outline(Ink::Key, INNER, &lid);
    let knob = circle(v2(base.x, base.y - 77.0), 4.0);
    d.backing(&knob);
    d.fill(Ink::Blue, 0.5, &knob);
    d.outline(Ink::Key, DETAIL, &knob);
    // A little drift of flour around its foot.
    for i in 0..6u32 {
        let p = base + v2(-40.0 + hash01(91, i) * 80.0, -3.0 + hash01(92, i) * 10.0);
        d.fill(Ink::Blue, 0.18, &circle(p, 3.0 + 3.0 * hash01(93, i)));
    }
}

/// Hanging utensils from a rail at `ry` (right side of the bakehouse).
fn utensils(d: &mut DrawList, ry: f32) {
    let hooks = [524.0, 568.0, 614.0, 660.0];
    for x in hooks {
        d.line(Ink::Key, 2.0, &arc(v2(x, ry + 6.0), 5.0, -PI * 0.5, PI * 0.6));
    }
    // Whisk.
    let x = hooks[0];
    let handle = capsule(v2(x, ry + 12.0), v2(x, ry + 48.0), 5.5);
    let top = ry + 46.0;
    for k in 0..5 {
        let dx = (k as f32 - 2.0) * 6.5;
        let loop_ = quad_bezier(v2(x - 2.0, top), v2(x + dx * 2.2, top + 70.0), v2(x + 2.0, top), 12);
        d.stroke_p(Paint::solid(Ink::Key, 0.85), 1.5, &loop_, false);
    }
    d.backing(&handle);
    wood(d, &handle, 0.5, PI * 0.5, 2);
    d.outline(Ink::Key, DETAIL, &handle);
    // Ladle.
    let x = hooks[1];
    let stem = capsule(v2(x, ry + 12.0), v2(x, ry + 92.0), 3.2);
    d.backing(&stem);
    d.fill(Ink::Blue, 0.35, &stem);
    d.outline(Ink::Key, DETAIL, &stem);
    let bowl = {
        let mut p = arc(v2(x, ry + 92.0), 17.0, 0.0, PI);
        p.push(v2(x - 17.0, ry + 92.0));
        p
    };
    d.backing(&bowl);
    d.fill(Ink::Blue, 0.4, &bowl);
    ramp(d, Ink::Key, &bowl, v2(x - 17.0, 0.0), v2(x + 17.0, 0.0), 0.0, 0.3, 3);
    d.outline(Ink::Key, DETAIL + 0.6, &bowl);
    // Wooden spoon.
    let x = hooks[2];
    let st = capsule(v2(x, ry + 12.0), v2(x, ry + 70.0), 3.6);
    let head = ellipse(v2(x, ry + 86.0), 11.0, 18.0, 0.0);
    d.backing(&st);
    d.backing(&head);
    wood(d, &st, 0.45, PI * 0.5, 4);
    wood(d, &head, 0.45, PI * 0.5, 5);
    d.outline(Ink::Key, DETAIL, &st);
    d.outline(Ink::Key, DETAIL + 0.6, &head);
    // Spatula.
    let x = hooks[3];
    let st = capsule(v2(x, ry + 12.0), v2(x, ry + 58.0), 3.4);
    let blade = rounded_rect(rect(x - 13.0, ry + 56.0, 26.0, 36.0), 6.0);
    d.backing(&st);
    d.fill(Ink::Pink, 0.75, &st);
    d.outline(Ink::Key, DETAIL, &st);
    d.backing(&blade);
    d.fill(Ink::Pink, 0.6, &blade);
    for k in 0..2 {
        d.knock_color(&capsule(
            v2(x - 4.0 + k as f32 * 8.0, ry + 64.0),
            v2(x - 4.0 + k as f32 * 8.0, ry + 82.0),
            2.0,
        ));
    }
    d.outline(Ink::Key, DETAIL + 0.6, &blade);
}

/// Pale marble: soft mottling and fine veins in the cool ink.
fn marble(d: &mut DrawList, r: Rect, seed: u32) {
    let poly = rect_poly(r);
    // A gentle darkening toward the viewer, then soft mottling and fine veins.
    ramp_p(
        d,
        Paint::solid(Ink::Blue, 1.0),
        &poly,
        v2(0.0, r.y + r.h * 0.45),
        v2(0.0, r.y + r.h),
        0.05,
        0.1,
        3,
    );
    for i in 0..9u32 {
        let c = v2(r.x + hash01(seed, i) * r.w, r.y + hash01(seed ^ 1, i) * r.h);
        // Soft, diagonal clouds of colour (flat tints: a screen here reads as stains).
        let blob = crate::geom::blob(
            c,
            70.0 + 90.0 * hash01(seed ^ 2, i),
            22.0 + 20.0 * hash01(seed ^ 3, i),
            0.25,
            i,
        );
        let blob: Vec<V2> = blob.iter().map(|q| c + (*q - c).rotate(-0.45)).collect();
        d.fill_p(Paint::solid(Ink::Blue, 0.03 + 0.03 * hash01(seed ^ 4, i)).add(), &clip_rect(&blob, r));
    }
    for i in 0..9u32 {
        let mut p = v2(r.x + hash01(seed ^ 5, i) * r.w, r.y + hash01(seed ^ 6, i) * r.h);
        let mut a = hash01(seed ^ 7, i) * PI - PI * 0.5 + 0.4;
        let mut pts = vec![p];
        for k in 0..14u32 {
            a += (hash01(seed ^ 8, i * 31 + k) - 0.5) * 0.9;
            p += V2::from_angle(a) * (16.0 + 10.0 * hash01(seed ^ 9, i * 31 + k));
            pts.push(p);
        }
        // Keep veins on the slab (they're thin, so clamping the points is enough).
        let pts: Vec<V2> =
            pts.iter().map(|q| v2(q.x.clamp(r.x, r.x + r.w), q.y.clamp(r.y + 3.0, r.y + r.h))).collect();
        let w = 0.8 + 1.0 * hash01(seed ^ 10, i);
        d.stroke_p(Paint::solid(Ink::Blue, 0.38), w, &chaikin(&pts, 2, false), false);
        if i % 3 == 0 {
            d.stroke_p(Paint::solid(Ink::Key, 0.2), 0.9, &chaikin(&pts[..pts.len() / 2], 2, false), false);
        }
    }
}

fn bakehouse_front(d: &mut DrawList, h: f32) {
    let top = counter_top(Backdrop::Bakehouse, h);
    // The marble slab's rounded front edge, then the wooden apron below it.
    let slab = rounded_rect(rect(-12.0, top, W + 24.0, 20.0), 8.0);
    let apron = rect_poly(rect(-12.0, top + 16.0, W + 24.0, h - top + 44.0));
    d.backing(&apron);
    wood(d, &apron, 0.62, 0.0, 83);
    ramp(d, Ink::Key, &apron, v2(0.0, top + 16.0), v2(0.0, top + 40.0), 0.3, 0.0, 3);
    // Plank seams.
    for i in 1..6 {
        let x = i as f32 * W / 6.0 + (hash01(85, i) - 0.5) * 30.0;
        d.stroke_p(Paint::solid(Ink::Key, 0.5), DETAIL, &[v2(x, top + 20.0), v2(x, h + 58.0)], false);
    }
    d.outline(Ink::Key, INNER, &apron);
    d.backing(&slab);
    d.fill(Ink::Blue, 0.1, &slab);
    ramp(d, Ink::Key, &slab, v2(0.0, top), v2(0.0, top + 20.0), 0.0, 0.2, 3);
    d.knock_p(0.7, Screen::Solid, PLATES_COLOR, &capsule(v2(40.0, top + 5.0), v2(W - 40.0, top + 5.0), 1.6));
    d.outline(Ink::Key, OUTER, &slab);
    // A gingham tea towel hanging over the edge, folded once.
    let tx = 640.0;
    let y0 = top + 6.0;
    let towel =
        vec![v2(tx - 30.0, y0), v2(tx + 30.0, y0), v2(tx + 33.0, y0 + 84.0), v2(tx - 27.0, y0 + 88.0)];
    d.ht(Ink::Key, 0.25, &translate(&towel, v2(5.0, 6.0)));
    d.backing(&towel);
    d.fill(Ink::Yellow, 0.12, &towel);
    d.clipped(&towel, |d| {
        for i in 0..6 {
            let x = tx - 40.0 + i as f32 * 14.0;
            d.fill(Ink::Blue, 0.42, &rect_poly(rect(x, y0, 7.0, 100.0)));
        }
        let mut y = y0 + 4.0;
        while y < y0 + 96.0 {
            d.fill_p(Paint::solid(Ink::Blue, 0.42).add(), &rect_poly(rect(tx - 50.0, y, 100.0, 7.0)));
            y += 14.0;
        }
        // A hem stripe near the bottom.
        d.fill(Ink::Pink, 0.8, &rect_poly(rect(tx - 50.0, y0 + 68.0, 100.0, 6.0)));
    });
    d.outline(Ink::Key, INNER, &towel);
    // The fold over the slab edge.
    let fold = rounded_rect(rect(tx - 33.0, top - 4.0, 66.0, 14.0), 6.0);
    d.backing(&fold);
    d.fill(Ink::Yellow, 0.12, &fold);
    d.fill(Ink::Blue, 0.3, &fold);
    d.outline(Ink::Key, INNER, &fold);
}

// =========================================================================================
// The Shopfront (Daylight)
// =========================================================================================

fn shopfront(d: &mut DrawList, h: f32) {
    let ct = counter_top(Backdrop::Shopfront, h);
    let rail = ct - 150.0 - extra(h) * 0.1;
    let wall = rect_poly(wall_to(rail));
    d.fill(Ink::Yellow, 0.22, &wall);
    // A soft sprig wallpaper: tiny coarse dots in a diamond lattice.
    let mut j = 0;
    let mut y = 120.0;
    while y < rail - 8.0 {
        let off = if j % 2 == 0 { 0.0 } else { 30.0 };
        let mut x = -30.0 + off;
        while x < W + 30.0 {
            d.fill(Ink::Pink, 0.4, &soft_star(v2(x, y), 5.0, 2.2, 4, 0.0));
            x += 60.0;
        }
        y += 44.0;
        j += 1;
    }
    // Wainscot rail and lower panelling.
    let low = rect_poly(rect(-12.0, rail, W + 24.0, ct - rail + 30.0));
    d.fill(Ink::Pink, 0.2, &low);
    d.fill(Ink::Yellow, 0.3, &low);
    for i in 0..13 {
        let x = -12.0 + i as f32 * 60.0;
        let panel = rounded_rect(rect(x, rail + 22.0, 46.0, ct - rail), 6.0);
        d.stroke_p(Paint::solid(Ink::Key, 0.35), DETAIL, &panel, true);
    }
    let moulding = rect_poly(rect(-12.0, rail - 4.0, W + 24.0, 12.0));
    d.backing(&moulding);
    wood(d, &moulding, 0.45, 0.0, 91);
    d.outline(Ink::Key, DETAIL + 0.8, &moulding);

    // The big shop window.
    let wy = 150.0 + extra(h) * 0.1;
    let win = rect(138.0, wy, 444.0, ct - wy + 30.0);
    let glass = rounded_rect(win, 16.0);
    d.knock(&glass);
    let transom = wy + 64.0;
    d.clipped(&glass, |d| {
        let sky = rect_poly(rect(win.x, win.y, win.w, win.h));
        ramp(d, Ink::Blue, &sky, v2(0.0, win.y), v2(0.0, win.y + win.h * 0.7), 0.42, 0.1, 8);
        for (c, s) in [(v2(win.x + 110.0, transom + 60.0), 30.0), (v2(win.x + 330.0, transom + 36.0), 24.0)] {
            let cl = scallop(c, s, 7, 0.2);
            let cl: Vec<V2> = cl.iter().map(|p| v2(p.x, c.y + (p.y - c.y) * 0.6)).collect();
            d.knock(&cl);
            d.stroke_p(Paint::solid(Ink::Key, 0.5), 1.4, &cl, true);
        }
        // Across the street: a row of little shops, far and pale.
        let street_y = win.y + win.h * 0.62;
        let mut x = win.x - 10.0;
        let mut i = 0u32;
        while x < win.x + win.w + 10.0 {
            let bw = 70.0 + 30.0 * hash01(201, i);
            let bh = 110.0 + 60.0 * hash01(202, i);
            let house = rect_poly(rect(x, street_y - bh, bw, bh + 80.0));
            d.knock(&house);
            let ink = [Ink::Pink, Ink::Yellow, Ink::Blue][i as usize % 3];
            d.fill(ink, 0.22, &house);
            d.fill(Ink::Blue, 0.1, &house);
            let roof = vec![
                v2(x - 4.0, street_y - bh),
                v2(x + bw * 0.5, street_y - bh - 26.0),
                v2(x + bw + 4.0, street_y - bh),
            ];
            d.knock(&roof);
            d.fill(ink, 0.45, &roof);
            d.fill(Ink::Blue, 0.2, &roof);
            d.stroke_p(Paint::solid(Ink::Key, 0.45), 1.3, &roof, true);
            for k in 0..2 {
                let wx = x + 14.0 + k as f32 * (bw - 40.0);
                let wwin = rounded_rect(rect(wx, street_y - bh + 30.0, 16.0, 22.0), 3.0);
                d.fill(Ink::Blue, 0.35, &wwin);
                d.stroke_p(Paint::solid(Ink::Key, 0.4), 1.2, &wwin, true);
            }
            d.stroke_p(Paint::solid(Ink::Key, 0.45), 1.3, &house, true);
            x += bw;
            i += 1;
        }
        // A tree and a lamp post on the pavement.
        let tree_c = v2(win.x + 88.0, street_y - 70.0);
        d.fill(Ink::Key, 0.6, &rect_poly(rect(tree_c.x - 4.0, tree_c.y + 20.0, 8.0, 60.0)));
        let crown = crate::geom::blob(tree_c, 48.0, 44.0, 0.12, 4);
        d.knock(&crown);
        d.fill(Ink::Yellow, 0.7, &crown);
        d.fill(Ink::Blue, 0.5, &crown);
        d.ht(Ink::Key, 0.12, &crate::geom::blob(tree_c + v2(10.0, 14.0), 34.0, 24.0, 0.2, 5));
        d.stroke_p(Paint::solid(Ink::Key, 0.6), 1.6, &crown, true);
        let lamp_x = win.x + win.w - 70.0;
        d.stroke_p(
            Paint::solid(Ink::Key, 0.7),
            4.0,
            &[v2(lamp_x, street_y + 60.0), v2(lamp_x, street_y - 110.0)],
            false,
        );
        let lamp = rounded_rect(rect(lamp_x - 10.0, street_y - 132.0, 20.0, 24.0), 5.0);
        d.knock(&lamp);
        d.fill(Ink::Yellow, 0.8, &lamp);
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 1.6, &lamp, true);
        // Pavement.
        let pave = rect_poly(rect(win.x, street_y + 60.0, win.w, 200.0));
        d.knock(&pave);
        d.fill(Ink::Yellow, 0.3, &pave);
        d.fill(Ink::Pink, 0.1, &pave);
        d.stroke_p(
            Paint::solid(Ink::Key, 0.5),
            1.4,
            &[v2(win.x, street_y + 60.0), v2(win.x + win.w, street_y + 60.0)],
            false,
        );
        // The awning outside: its scalloped valance hangs across the top of the view.
        let aw_y = transom + 8.0;
        let n = 9;
        let sw = win.w / n as f32;
        for k in 0..n {
            let x0 = win.x + k as f32 * sw;
            let mut st = vec![v2(x0, aw_y - 20.0), v2(x0 + sw, aw_y - 20.0), v2(x0 + sw, aw_y + 18.0)];
            st.extend(arc(v2(x0 + sw * 0.5, aw_y + 18.0), sw * 0.5, 0.0, PI));
            d.knock(&st);
            if k % 2 == 0 {
                d.fill(Ink::Pink, 0.85, &st);
            } else {
                d.fill(Ink::Yellow, 0.1, &st);
            }
            d.ht(Ink::Key, 0.1, &st);
            d.stroke_p(Paint::solid(Ink::Key, 0.85), DETAIL, &st, true);
        }
    });
    window_frame(d, &glass, win, 0.3, false, Some(transom));

    // Pendant lamps.
    for (x, drop) in [(74.0, 214.0), (646.0, 214.0)] {
        pendant(d, v2(x, drop + extra(h) * 0.12));
    }
    // Left: a hanging plant; right: the chalkboard menu. Below them, wicker bread
    // baskets hang on the wall, full of baguettes.
    hanging_plant(d, v2(70.0, 330.0 + extra(h) * 0.2));
    chalkboard(d, rect(598.0, 326.0 + extra(h) * 0.2, 112.0, 176.0));
    let by = rail - 150.0;
    wall_basket(d, v2(70.0, by), 3, 1);
    wall_basket(d, v2(652.0, by + 12.0), 2, 2);
}

/// A wicker basket hanging from a peg at `peg`, holding `n` baguettes.
fn wall_basket(d: &mut DrawList, peg: V2, n: u32, seed: u32) {
    let rim_y = peg.y + 46.0;
    let (w, depth) = (104.0, 50.0);
    // Rope loop and peg.
    d.stroke_p(
        Paint::solid(Ink::Key, 0.85),
        1.6,
        &[v2(peg.x - w * 0.42, rim_y), peg, v2(peg.x + w * 0.42, rim_y)],
        false,
    );
    let knob = circle(peg, 5.0);
    d.backing(&knob);
    wood(d, &knob, 0.6, 0.0, 3);
    d.outline(Ink::Key, DETAIL, &knob);
    // Shadow on the wall behind the basket.
    let body = {
        let mut b = arc(v2(peg.x, rim_y), w * 0.5, 0.0, PI);
        for q in b.iter_mut() {
            *q = v2(q.x, rim_y + (q.y - rim_y) * (depth / (w * 0.5)));
        }
        b.push(v2(peg.x - w * 0.5, rim_y));
        b
    };
    d.ht(Ink::Key, 0.22, &translate(&body, v2(6.0, 8.0)));
    // Back rim.
    let rim = ellipse(v2(peg.x, rim_y), w * 0.5, 9.0, 0.0);
    d.backing(&rim);
    d.fill(Ink::Yellow, 0.55, &rim);
    d.fill(Ink::Pink, 0.45, &rim);
    d.fill(Ink::Key, 0.18, &rim);
    // Baguettes, fanned out of the basket.
    for i in 0..n {
        let t = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
        let ang = -PI * 0.5 + t * 0.7 + (hash01(seed, i) - 0.5) * 0.12;
        let root = v2(peg.x + t * 40.0, rim_y + 26.0);
        let len = 96.0 + 14.0 * hash01(seed ^ 3, i);
        let tip = root + V2::from_angle(ang) * len;
        let loaf = capsule(root, tip, 10.0);
        d.backing(&loaf);
        d.fill(Ink::Yellow, 0.9, &loaf);
        d.fill(Ink::Pink, 0.42, &loaf);
        let dir = V2::from_angle(ang);
        let side = dir.perp();
        for k in 0..4 {
            let c = root + dir * (len * (0.4 + 0.17 * k as f32));
            let slash = [c - side * 6.0 - dir * 5.0, c + side * 6.0 + dir * 5.0];
            d.knock_line(1.0, 4.0, &slash, false);
            d.stroke_p(Paint::solid(Ink::Yellow, 0.5), 2.2, &slash, false);
        }
        d.outline(Ink::Key, DETAIL + 0.8, &loaf);
    }
    // Basket front with a woven texture.
    d.backing(&body);
    d.fill(Ink::Yellow, 0.62, &body);
    d.fill(Ink::Pink, 0.3, &body);
    for row in 0..4 {
        let y = rim_y + 8.0 + row as f32 * 10.0;
        let mut x = peg.x - w * 0.5 + if row % 2 == 0 { 0.0 } else { 7.0 };
        while x < peg.x + w * 0.5 {
            let seg = [v2(x, y), v2(x + 9.0, y + 6.0)];
            for piece in clip_polyline_convex(&seg, &body) {
                d.stroke_p(Paint::solid(Ink::Pink, 0.75), 2.0, &piece, false);
            }
            x += 14.0;
        }
    }
    d.outline(Ink::Key, INNER, &body);
    let lip = capsule(v2(peg.x - w * 0.5, rim_y), v2(peg.x + w * 0.5, rim_y), 4.5);
    d.backing(&lip);
    wood(d, &lip, 0.45, 0.0, 7);
    d.outline(Ink::Key, DETAIL + 0.8, &lip);
}

fn pendant(d: &mut DrawList, at: V2) {
    d.line(Ink::Key, 2.0, &[v2(at.x, -200.0), at + v2(0.0, -30.0)]);
    glow(d, Ink::Yellow, at + v2(0.0, 22.0), 110.0, 90.0, 0.02, 0.35, 6, None);
    let shade = {
        let mut p = arc(at + v2(0.0, 6.0), 38.0, PI, TAU);
        p.push(at + v2(38.0, 12.0));
        p.push(at + v2(-38.0, 12.0));
        p
    };
    let cap = rounded_rect(rect(at.x - 8.0, at.y - 42.0, 16.0, 12.0), 3.0);
    d.backing(&cap);
    d.fill(Ink::Key, 0.8, &cap);
    let bulb = circle(at + v2(0.0, 16.0), 11.0);
    d.backing(&bulb);
    d.fill(Ink::Yellow, 0.95, &bulb);
    d.knock_p(0.6, Screen::Solid, 0b0010, &circle(at + v2(-3.0, 13.0), 4.0));
    d.outline(Ink::Key, DETAIL, &bulb);
    d.backing(&shade);
    d.fill(Ink::Blue, 0.7, &shade);
    ramp(d, Ink::Key, &shade, at + v2(-38.0, 0.0), at + v2(38.0, 0.0), 0.0, 0.35, 4);
    d.knock_p(
        0.6,
        Screen::Solid,
        PLATES_COLOR,
        &crate::art::props::arc_band(at + v2(0.0, 6.0), 28.0, PI * 1.15, PI * 1.4, 4.0),
    );
    d.outline(Ink::Key, INNER, &shade);
}

fn hanging_plant(d: &mut DrawList, at: V2) {
    // Macramé hanger.
    for dx in [-18.0f32, 0.0, 18.0] {
        d.stroke_p(Paint::solid(Ink::Key, 0.8), 1.4, &[v2(at.x, -200.0 + 0.0), at + v2(dx, -6.0)], false);
    }
    // Trailing vines.
    for (k, dx) in [-30.0f32, -12.0, 14.0, 30.0].iter().enumerate() {
        let len = 90.0 + 50.0 * hash01(211, k as u32);
        let pts: Vec<V2> = (0..=10)
            .map(|i| {
                let t = i as f32 / 10.0;
                at + v2(dx + (t * 5.0 + k as f32).sin() * 6.0, 20.0 + t * len)
            })
            .collect();
        d.stroke_p(Paint::solid(Ink::Key, 0.7), 1.4, &pts, false);
        for (i, p) in pts.iter().enumerate().skip(1).step_by(2) {
            let side = if i % 4 == 1 { 1.0 } else { -1.0 };
            let leaf = crate::geom::lens_along(&[*p, *p + v2(side * 12.0, 6.0)], 8.0, 0.0);
            d.backing(&leaf);
            d.fill(Ink::Yellow, 0.8, &leaf);
            d.fill(Ink::Blue, 0.6, &leaf);
            d.stroke_p(Paint::solid(Ink::Key, 0.9), 1.3, &leaf, true);
        }
    }
    let pot = chaikin(
        &[at + v2(-34.0, -8.0), at + v2(34.0, -8.0), at + v2(26.0, 30.0), at + v2(-26.0, 30.0)],
        1,
        true,
    );
    d.backing(&pot);
    d.fill(Ink::Pink, 0.75, &pot);
    d.ht(Ink::Key, 0.12, &pot);
    d.knock_color(&rect_poly(rect(at.x - 30.0, at.y + 4.0, 60.0, 5.0)));
    d.outline(Ink::Key, INNER, &pot);
}

fn chalkboard(d: &mut DrawList, r: Rect) {
    d.ht(Ink::Key, 0.25, &translate(&rounded_rect(r, 8.0), v2(5.0, 7.0)));
    let frame = rounded_rect(r, 8.0);
    d.backing(&frame);
    wood(d, &frame, 0.6, PI * 0.5, 95);
    d.outline(Ink::Key, INNER, &frame);
    let slate = rounded_rect(rect(r.x + 9.0, r.y + 9.0, r.w - 18.0, r.h - 18.0), 4.0);
    d.fill(Ink::Key, 0.84, &slate);
    d.fill(Ink::Blue, 0.3, &slate);
    // Chalk: a loaf doodle, dotted price lines, a heart. (Knock-outs = chalk on slate.)
    let c = v2(r.x + r.w * 0.5, r.y + 44.0);
    let loaf = ellipse(c, 26.0, 16.0, 0.0);
    d.knock_line(0.9, 2.2, &loaf, true);
    d.knock_line(0.9, 2.0, &quad_bezier(c + v2(-13.0, 7.0), c + v2(0.0, 0.0), c + v2(13.0, -7.0), 6), false);
    let mut y = r.y + 82.0;
    for k in 0..4 {
        let w = 34.0 + 18.0 * hash01(221, k);
        d.knock_line(0.85, 2.2, &[v2(r.x + 20.0, y), v2(r.x + 20.0 + w, y)], false);
        let mut x = r.x + 26.0 + w;
        while x < r.x + r.w - 30.0 {
            d.knock_p(0.8, Screen::Solid, 0b1111, &circle(v2(x, y), 1.1));
            x += 6.0;
        }
        d.knock_line(0.85, 2.2, &[v2(r.x + r.w - 26.0, y), v2(r.x + r.w - 18.0, y)], false);
        y += 20.0;
    }
    d.knock_line(0.8, 2.0, &crate::geom::heart(v2(r.x + r.w - 26.0, r.y + r.h - 26.0), 16.0), true);
    // Chalk dust at the tray.
    let tray = rounded_rect(rect(r.x + 4.0, r.y + r.h - 4.0, r.w - 8.0, 8.0), 3.0);
    d.backing(&tray);
    wood(d, &tray, 0.5, 0.0, 97);
    d.outline(Ink::Key, DETAIL, &tray);
    let chalk = capsule(v2(r.x + 26.0, r.y + r.h - 6.0), v2(r.x + 44.0, r.y + r.h - 6.0), 3.0);
    d.knock(&chalk);
    d.outline(Ink::Key, 1.2, &chalk);
}

/// The glass display case, drawn in front of the customer. Goods sit on its shelves.
fn shopfront_front(d: &mut DrawList, h: f32) {
    let ct = counter_top(Backdrop::Shopfront, h);
    let rows = case_rows(h);
    // Case body: wooden frame around glass.
    let body = rect_poly(rect(-12.0, ct + 20.0, W + 24.0, h - ct + 40.0));
    d.backing(&body);
    d.fill(Ink::Blue, 0.1, &body);
    d.fill(Ink::Yellow, 0.1, &body);
    // Back wall of the case (slightly darker, gives depth behind the goods).
    let back = rect_poly(rect(18.0, ct + 34.0, W - 36.0, h - ct - 80.0));
    d.fill(Ink::Pink, 0.14, &back);
    ramp(d, Ink::Key, &back, v2(0.0, ct + 34.0), v2(0.0, ct + 70.0), 0.2, 0.0, 3);
    // Shelves: a top surface and a front edge, just under each row's goods.
    for (i, cy) in rows.iter().enumerate() {
        let sy = cy + CASE_SHELF;
        let surf = rect_poly(rect(18.0, sy - 16.0, W - 36.0, 16.0));
        d.fill(Ink::Yellow, 0.4, &surf);
        d.fill(Ink::Pink, 0.16, &surf);
        let edge = rect_poly(rect(14.0, sy, W - 28.0, 13.0));
        d.backing(&edge);
        wood(d, &edge, 0.55, 0.0, 301 + i as u32);
        d.outline(Ink::Key, DETAIL + 0.8, &edge);
        d.stroke_p(
            Paint::solid(Ink::Key, 0.6),
            DETAIL * 0.8,
            &[v2(18.0, sy - 16.0), v2(W - 18.0, sy - 16.0)],
            false,
        );
        // Paper doilies under each column (things sit on something).
        for x in case_cols() {
            let doily = ellipse(v2(x, sy - 6.0), 58.0, 9.0, 0.0);
            let lace = scallop(v2(x, sy - 6.0), 58.0, 14, 0.07);
            let lace: Vec<V2> = lace.iter().map(|p| v2(p.x, sy - 6.0 + (p.y - (sy - 6.0)) * 0.16)).collect();
            d.knock(&lace);
            d.stroke_p(Paint::solid(Ink::Key, 0.45), 1.2, &lace, true);
            d.stroke_p(Paint::solid(Ink::Pink, 0.5), 1.0, &doily, true);
        }
    }
    // Glass: reflections, and a frame.
    for (x, w) in [(40.0, 22.0), (72.0, 8.0), (W - 120.0, 16.0)] {
        let streak =
            vec![v2(x, ct + 40.0), v2(x + w, ct + 40.0), v2(x + w - 30.0, h - 60.0), v2(x - 30.0, h - 60.0)];
        d.knock_p(
            0.35,
            Screen::Solid,
            PLATES_COLOR,
            &clip_rect(&streak, rect(18.0, ct + 34.0, W - 36.0, h - ct - 80.0)),
        );
    }
    for x in [0.0, W - 18.0] {
        let post = rect_poly(rect(x, ct + 20.0, 18.0, h - ct));
        d.backing(&post);
        wood(d, &post, 0.62, PI * 0.5, 311);
        d.outline(Ink::Key, INNER, &post);
    }
    d.stroke_p(
        Paint::solid(Ink::Key, 1.0),
        INNER,
        &rect_poly(rect(18.0, ct + 34.0, W - 36.0, h - ct - 80.0)),
        true,
    );
    // Kick plate.
    let kick = rect_poly(rect(-12.0, h - 34.0, W + 24.0, 94.0));
    d.backing(&kick);
    d.fill(Ink::Pink, 0.72, &kick);
    d.ht(Ink::Key, 0.2, &kick);
    for i in 0..24 {
        let x = i as f32 * 32.0 + 8.0;
        d.knock_p(0.7, Screen::Solid, PLATES_COLOR, &circle(v2(x, h - 17.0), 3.0));
    }
    d.outline(Ink::Key, INNER, &kick);
    // The counter top: a thick butcher-block slab.
    let slab = rounded_rect(rect(-12.0, ct, W + 24.0, 34.0), 10.0);
    d.ht(Ink::Key, 0.3, &rect_poly(rect(-12.0, ct + 30.0, W + 24.0, 12.0)));
    d.backing(&slab);
    wood(d, &slab, 0.48, 0.0, 321);
    let top_face = rect_poly(rect(-12.0, ct, W + 24.0, 12.0));
    d.fill(Ink::Yellow, 0.62, &top_face);
    d.fill(Ink::Pink, 0.2, &top_face);
    d.knock_p(0.6, Screen::Solid, PLATES_COLOR, &capsule(v2(30.0, ct + 5.0), v2(W - 30.0, ct + 5.0), 1.5));
    d.line(Ink::Key, DETAIL, &[v2(-30.0, ct + 12.0), v2(W + 30.0, ct + 12.0)]);
    d.outline(Ink::Key, OUTER, &slab);
    // On the counter: a cake stand under a glass dome (left).
    cake_dome(d, v2(96.0, ct + 4.0));
}

fn cake_dome(d: &mut DrawList, base: V2) {
    contact_shadow(d, base + v2(0.0, -1.0), 58.0, 6.0);
    let foot = rounded_rect(rect(base.x - 26.0, base.y - 10.0, 52.0, 10.0), 4.0);
    let stem = rect_poly(rect(base.x - 7.0, base.y - 34.0, 14.0, 26.0));
    let plate = rounded_rect(rect(base.x - 56.0, base.y - 42.0, 112.0, 10.0), 5.0);
    for p in [&foot, &stem] {
        d.backing(p);
        d.fill(Ink::Pink, 0.55, p);
        d.outline(Ink::Key, DETAIL + 0.6, p);
    }
    // A little layer cake.
    let cake = rounded_rect(rect(base.x - 38.0, base.y - 94.0, 76.0, 52.0), 8.0);
    d.backing(&cake);
    d.fill(Ink::Yellow, 0.5, &cake);
    d.fill(Ink::Pink, 0.14, &cake);
    let icing = rect_poly(rect(base.x - 38.0, base.y - 72.0, 76.0, 8.0));
    d.fill(Ink::Pink, 0.8, &icing);
    let top_icing = chaikin(
        &[
            v2(base.x - 40.0, base.y - 96.0),
            v2(base.x + 40.0, base.y - 96.0),
            v2(base.x + 40.0, base.y - 84.0),
            v2(base.x + 26.0, base.y - 80.0),
            v2(base.x + 12.0, base.y - 86.0),
            v2(base.x - 4.0, base.y - 79.0),
            v2(base.x - 20.0, base.y - 86.0),
            v2(base.x - 40.0, base.y - 81.0),
        ],
        1,
        true,
    );
    d.backing(&top_icing);
    d.fill(Ink::Pink, 0.4, &top_icing);
    d.outline(Ink::Key, DETAIL + 0.6, &top_icing);
    d.outline(Ink::Key, DETAIL + 0.6, &cake);
    let cherry = circle(v2(base.x, base.y - 104.0), 7.0);
    d.backing(&cherry);
    d.fill(Ink::Pink, 1.0, &cherry);
    d.knock_p(0.8, Screen::Solid, PLATES_COLOR, &circle(v2(base.x - 2.0, base.y - 106.0), 2.0));
    d.outline(Ink::Key, DETAIL, &cherry);
    d.backing(&plate);
    d.fill(Ink::Pink, 0.55, &plate);
    d.outline(Ink::Key, DETAIL + 0.6, &plate);
    // Glass dome (just an outline + glints: it's transparent).
    let mut dome = arc(v2(base.x, base.y - 70.0), 52.0, PI, TAU);
    dome.insert(0, v2(base.x - 52.0, base.y - 42.0));
    dome.push(v2(base.x + 52.0, base.y - 42.0));
    d.stroke_p(Paint::solid(Ink::Key, 1.0), INNER, &dome, false);
    d.knock_p(
        0.8,
        Screen::Solid,
        PLATES_COLOR,
        &crate::art::props::arc_band(v2(base.x, base.y - 70.0), 44.0, PI * 1.12, PI * 1.35, 5.0),
    );
    let knob = circle(v2(base.x, base.y - 126.0), 6.0);
    d.backing(&knob);
    d.fill(Ink::Blue, 0.5, &knob);
    d.outline(Ink::Key, DETAIL, &knob);
}

// =========================================================================================
// The Pantry (Dusk)
// =========================================================================================

fn pantry(d: &mut DrawList, h: f32, dress: &Dressing) {
    let js = jar_shelf(Backdrop::Pantry, h);
    let ct = counter_top(Backdrop::Pantry, h);
    let wain = js + 72.0;
    let wall_r = wall_to(wain);
    let wall = rect_poly(wall_r);
    // Dusky violet plaster with a warm lamp pool around the starter shelf: the dusk inks
    // thin out ring by ring toward the lamp, and lamp-yellow dots warm it.
    d.fill(Ink::Blue, 0.42, &wall);
    d.fill(Ink::Pink, 0.14, &wall);
    let lc = v2(360.0, js - 110.0);
    let steps = 7;
    for i in 0..steps {
        let k = 1.0 - i as f32 / steps as f32;
        let keep = 0.86f32.powi(i + 1);
        let e = if i + 1 < steps {
            ring_poly(lc, 440.0 * k, 320.0 * k, (k - 1.0 / steps as f32) / k)
        } else {
            ellipse(lc, 440.0 * k, 320.0 * k, 0.0)
        };
        let e = clip_rect(&e, wall_r);
        d.fill(Ink::Blue, 0.42 * keep, &e);
        d.fill(Ink::Pink, 0.14 * keep, &e);
    }
    glow(d, Ink::Yellow, lc, 500.0, 360.0, 0.04, 0.6, 9, Some(wall_r));
    // Vignette toward the top corners.
    let top_band = rect_poly(rect(-12.0, -40.0, W + 24.0, 400.0));
    ramp(d, Ink::Key, &top_band, v2(0.0, -40.0), v2(0.0, 360.0), 0.18, 0.0, 5);

    // Wainscot below the shelf: beadboard panelling and a rail.
    let low = rect_poly(rect(-12.0, wain, W + 24.0, ct - wain + 30.0));
    d.fill(Ink::Blue, 0.42, &low);
    d.fill(Ink::Pink, 0.16, &low);
    let mut x = 12.0;
    while x < W {
        d.stroke_p(Paint::solid(Ink::Key, 0.3), 1.6, &[v2(x, wain + 10.0), v2(x, ct)], false);
        d.knock_p(
            0.35,
            Screen::Solid,
            PLATES_COLOR,
            &rect_poly(rect(x + 2.0, wain + 10.0, 2.0, ct - wain - 10.0)),
        );
        x += 30.0;
    }
    let rail = rect_poly(rect(-12.0, wain - 2.0, W + 24.0, 12.0));
    d.backing(&rail);
    wood(d, &rail, 0.55, 0.0, 401);
    d.outline(Ink::Key, DETAIL + 0.8, &rail);
    // Tall walls: a peg rail on the panelling with the baker's apron and a tote.
    if ct - wain > 170.0 {
        let py = wain + 40.0;
        let peg_rail = rect_poly(rect(26.0, py - 6.0, 200.0, 12.0));
        d.backing(&peg_rail);
        wood(d, &peg_rail, 0.6, 0.0, 405);
        d.outline(Ink::Key, DETAIL + 0.6, &peg_rail);
        apron(d, v2(78.0, py), 1);
        tote(d, v2(178.0, py));
    }

    // The moonlit window (left).
    let win = rect(36.0, 170.0 + extra(h) * 0.08, 256.0, 300.0 + extra(h) * 0.22);
    let glass = arch(win);
    d.knock(&glass);
    d.clipped(&glass, |d| {
        let sky = rect_poly(rect(win.x - 20.0, win.y - 20.0, win.w + 40.0, win.h + 40.0));
        d.fill(Ink::Blue, 0.9, &sky);
        ramp(d, Ink::Pink, &sky, v2(0.0, win.y), v2(0.0, win.y + win.h), 0.05, 0.4, 7);
        ramp(d, Ink::Key, &sky, v2(0.0, win.y), v2(0.0, win.y + win.h * 0.7), 0.3, 0.0, 5);
        // Moon with a soft halo.
        let mc = v2(win.x + win.w * 0.62, win.y + win.w * 0.42);
        glow(d, Ink::Yellow, mc, 90.0, 90.0, 0.04, 0.3, 5, None);
        let moon = circle(mc, 34.0);
        d.knock(&moon);
        d.fill(Ink::Yellow, 0.85, &moon);
        let bite = circle(mc + v2(15.0, -11.0), 29.0);
        d.knock(&bite);
        d.fill(Ink::Blue, 0.9, &bite);
        d.fill(Ink::Pink, 0.12, &bite);
        d.ht(Ink::Key, 0.18, &bite);
        for i in 0..16u32 {
            let p = v2(
                win.x + 14.0 + hash01(11, i) * (win.w - 28.0),
                win.y + 16.0 + hash01(12, i) * win.h * 0.55,
            );
            if p.dist(mc) < 50.0 {
                continue;
            }
            let s = 3.0 + 5.0 * hash01(13, i);
            d.knock(&soft_star(p, s, s * 0.36, 4, 0.0));
            if s > 6.0 {
                d.fill(Ink::Yellow, 0.4, &soft_star(p, s, s * 0.36, 4, 0.0));
            }
        }
        // Sleepy rooftops with a few lit windows.
        let base = win.y + win.h;
        let mut x = win.x - 10.0;
        let mut i = 0u32;
        while x < win.x + win.w + 10.0 {
            let bw = 48.0 + 28.0 * hash01(21, i);
            let bh = 50.0 + 60.0 * hash01(22, i);
            let mut house = vec![
                v2(x, base + 20.0),
                v2(x, base - bh),
                v2(x + bw * 0.5, base - bh - 22.0),
                v2(x + bw, base - bh),
                v2(x + bw, base + 20.0),
            ];
            if i % 3 == 1 {
                house = vec![
                    v2(x, base + 20.0),
                    v2(x, base - bh),
                    v2(x + bw, base - bh),
                    v2(x + bw, base + 20.0),
                ];
            }
            d.fill(Ink::Key, 0.62, &house);
            d.fill(Ink::Blue, 0.9, &house);
            if hash01(23, i) > 0.35 {
                let lw = rounded_rect(rect(x + bw * 0.35, base - bh + 16.0, 12.0, 14.0), 2.0);
                d.knock(&lw);
                d.fill(Ink::Yellow, 0.9, &lw);
            }
            x += bw - 2.0;
            i += 1;
        }
    });
    window_frame(d, &glass, win, 0.6, true, Some(win.y + win.w * 0.5 + 16.0));
    let sill = rounded_rect(rect(win.x - 20.0, win.y + win.h - 2.0, win.w + 40.0, 16.0), 5.0);
    d.backing(&sill);
    wood(d, &sill, 0.55, 0.0, 411);
    d.outline(Ink::Key, INNER, &sill);
    // A candle on the sill.
    let cb = v2(win.x + 40.0, win.y + win.h - 2.0);
    glow(d, Ink::Yellow, cb + v2(0.0, -40.0), 44.0, 44.0, 0.05, 0.45, 4, None);
    let candle = rounded_rect(rect(cb.x - 8.0, cb.y - 30.0, 16.0, 30.0), 3.0);
    d.backing(&candle);
    d.fill(Ink::Yellow, 0.2, &candle);
    d.outline(Ink::Key, DETAIL, &candle);
    let flame = chaikin(
        &[cb + v2(0.0, -48.0), cb + v2(6.0, -36.0), cb + v2(0.0, -31.0), cb + v2(-6.0, -36.0)],
        2,
        true,
    );
    d.backing(&flame);
    d.fill(Ink::Yellow, 1.0, &flame);
    d.fill(Ink::Pink, 0.3, &flame);
    d.outline(Ink::Key, 1.6, &flame);

    // Fairy lights draped under the headline, across the top of the wall.
    let ly = 150.0 + extra(h) * 0.04;
    fairy_lights(d, v2(-10.0, ly), v2(730.0, ly), 30.0, 4);

    // The starter shelf.
    shelf_board(d, js, 8.0, W - 8.0, 431);
    corbel(d, 60.0, js + 15.0, 1.0);
    corbel(d, W - 60.0, js + 15.0, 1.0);
    let occ: Vec<(f32, f32)> = dress.jars.iter().map(|x| (*x, 72.0)).collect();
    dress_shelf(
        d,
        js - 7.0,
        18.0,
        W - 18.0,
        &occ,
        &[(50.0, 0), (72.0, 2), (60.0, 3), (34.0, 4), (42.0, 1), (60.0, 5)],
        2,
    );
}

/// A gingham baker's apron hanging from a peg at `peg`.
fn apron(d: &mut DrawList, peg: V2, seed: u32) {
    let top = peg.y + 4.0;
    let body = chaikin(
        &[
            v2(peg.x - 22.0, top + 10.0),
            v2(peg.x + 22.0, top + 10.0),
            v2(peg.x + 26.0, top + 52.0),
            v2(peg.x + 44.0, top + 58.0),
            v2(peg.x + 40.0, top + 136.0),
            v2(peg.x - 40.0, top + 136.0),
            v2(peg.x - 44.0, top + 58.0),
            v2(peg.x - 26.0, top + 52.0),
        ],
        2,
        true,
    );
    d.ht(Ink::Key, 0.22, &translate(&body, v2(5.0, 7.0)));
    // Neck loop.
    d.line(
        Ink::Key,
        2.2,
        &quad_bezier(v2(peg.x - 18.0, top + 12.0), v2(peg.x, top - 26.0), v2(peg.x + 18.0, top + 12.0), 10),
    );
    d.backing(&body);
    d.fill(Ink::Yellow, 0.12, &body);
    for i in 0..7 {
        let x = peg.x - 48.0 + i as f32 * 14.0;
        let stripe = crate::geom::rect_poly(rect(x, top, 7.0, 150.0));
        let stripe = super::props::clip_convex(&stripe, &body);
        d.fill_p(Paint::solid(Ink::Pink, 0.4), &stripe);
    }
    let mut y = top + 12.0;
    while y < top + 140.0 {
        let band =
            super::props::clip_convex(&crate::geom::rect_poly(rect(peg.x - 50.0, y, 100.0, 7.0)), &body);
        d.fill_p(Paint::solid(Ink::Pink, 0.4).add(), &band);
        y += 14.0;
    }
    // Pocket and ties.
    let pocket = rounded_rect(rect(peg.x - 18.0, top + 84.0, 36.0, 28.0), 5.0);
    d.backing(&pocket);
    d.fill(Ink::Blue, 0.55, &pocket);
    d.outline(Ink::Key, DETAIL, &pocket);
    d.outline(Ink::Key, INNER, &body);
    for sx in [-1.0f32, 1.0] {
        let tie = quad_bezier(
            v2(peg.x + sx * 42.0, top + 60.0),
            v2(peg.x + sx * 50.0, top + 74.0),
            v2(peg.x + sx * 46.0, top + 96.0),
            8,
        );
        d.line(Ink::Key, 2.0, &tie);
    }
    let knob = circle(peg, 5.5);
    d.backing(&knob);
    wood(d, &knob, 0.7, 0.0, seed);
    d.outline(Ink::Key, DETAIL, &knob);
}

/// A canvas tote bag with a loaf poking out, hanging from a peg at `peg`.
fn tote(d: &mut DrawList, peg: V2) {
    let top = peg.y + 30.0;
    let bag = rounded_rect(rect(peg.x - 34.0, top, 68.0, 76.0), 6.0);
    d.ht(Ink::Key, 0.22, &translate(&bag, v2(5.0, 7.0)));
    d.line(
        Ink::Key,
        2.2,
        &quad_bezier(v2(peg.x - 22.0, top + 2.0), v2(peg.x, peg.y - 30.0), v2(peg.x + 22.0, top + 2.0), 10),
    );
    // A baguette peeking out.
    let loaf = capsule(v2(peg.x - 6.0, top + 20.0), v2(peg.x + 20.0, top - 30.0), 9.0);
    d.backing(&loaf);
    d.fill(Ink::Yellow, 0.9, &loaf);
    d.fill(Ink::Pink, 0.42, &loaf);
    d.knock_line(1.0, 3.0, &[v2(peg.x + 4.0, top - 6.0), v2(peg.x + 16.0, top - 12.0)], false);
    d.outline(Ink::Key, DETAIL + 0.6, &loaf);
    d.backing(&bag);
    d.fill(Ink::Yellow, 0.24, &bag);
    d.fill(Ink::Pink, 0.06, &bag);
    let heart = crate::geom::heart(v2(peg.x, top + 40.0), 24.0);
    d.fill(Ink::Pink, 0.8, &heart);
    d.outline(Ink::Key, INNER, &bag);
    let knob = circle(peg, 5.5);
    d.backing(&knob);
    wood(d, &knob, 0.7, 0.0, 9);
    d.outline(Ink::Key, DETAIL, &knob);
}

fn fairy_lights(d: &mut DrawList, a: V2, b: V2, sag: f32, swags: usize) {
    for s in 0..swags {
        let p0 = a.lerp(b, s as f32 / swags as f32);
        let p1 = a.lerp(b, (s + 1) as f32 / swags as f32);
        let mid = p0.lerp(p1, 0.5) + v2(0.0, sag * 2.0);
        let wire = quad_bezier(p0, mid, p1, 18);
        d.stroke_p(Paint::solid(Ink::Key, 0.8), 1.5, &wire, false);
        for k in 0..5 {
            let t = (k as f32 + 0.5) / 5.0;
            let c = bez(p0, mid, p1, t) + v2(0.0, 7.0);
            glow(d, Ink::Yellow, c, 20.0, 20.0, 0.1, 0.55, 3, None);
            let bulb = ellipse(c, 4.5, 6.0, 0.0);
            d.backing(&bulb);
            d.fill(Ink::Yellow, 1.0, &bulb);
            if (k + s) % 2 == 0 {
                d.fill(Ink::Pink, 0.35, &bulb);
            }
            d.outline(Ink::Key, 1.4, &bulb);
            d.fill(Ink::Key, 0.85, &rect_poly(rect(c.x - 2.5, c.y - 9.0, 5.0, 4.0)));
        }
    }
}

/// The prep counter: a butcher-block top (objects stand on its back half) over cabinets.
fn pantry_front(d: &mut DrawList, h: f32) {
    let ct = counter_top(Backdrop::Pantry, h);
    let depth = 80.0;
    let top = rect_poly(rect(-12.0, ct, W + 24.0, depth));
    d.backing(&top);
    wood(d, &top, 0.4, 0.0, 441);
    // Butcher-block strips.
    for i in 1..9 {
        let y = ct + i as f32 * depth / 9.0;
        d.stroke_p(Paint::solid(Ink::Pink, 0.35), 1.2, &[v2(-30.0, y), v2(W + 30.0, y)], false);
    }
    ramp(d, Ink::Key, &top, v2(0.0, ct), v2(0.0, ct + 18.0), 0.28, 0.0, 3);
    d.line(Ink::Key, INNER, &[v2(-30.0, ct), v2(W + 30.0, ct)]);
    // Front edge.
    let edge = rounded_rect(rect(-12.0, ct + depth, W + 24.0, 18.0), 6.0);
    d.backing(&edge);
    wood(d, &edge, 0.62, 0.0, 443);
    d.knock_p(
        0.55,
        Screen::Solid,
        PLATES_COLOR,
        &capsule(v2(20.0, ct + depth + 5.0), v2(W - 20.0, ct + depth + 5.0), 1.5),
    );
    d.outline(Ink::Key, OUTER, &edge);
    // Cabinet doors.
    let cab_top = ct + depth + 18.0;
    let body = rect_poly(rect(-12.0, cab_top, W + 24.0, h - cab_top + 60.0));
    d.backing(&body);
    d.fill(Ink::Pink, 0.62, &body);
    d.fill(Ink::Blue, 0.22, &body);
    ramp(d, Ink::Key, &body, v2(0.0, cab_top), v2(0.0, cab_top + 30.0), 0.35, 0.0, 3);
    for i in 0..3 {
        let x = 16.0 + i as f32 * 232.0;
        let door = rounded_rect(rect(x, cab_top + 16.0, 224.0, h - cab_top + 60.0), 12.0);
        d.stroke_p(Paint::solid(Ink::Key, 0.8), INNER, &door, true);
        let inset = rounded_rect(rect(x + 14.0, cab_top + 30.0, 196.0, h - cab_top + 60.0), 8.0);
        d.ht(Ink::Key, 0.12, &inset);
        d.stroke_p(Paint::solid(Ink::Key, 0.5), DETAIL, &inset, true);
        let knob = circle(v2(x + if i == 0 { 200.0 } else { 24.0 }, cab_top + 48.0), 6.0);
        d.backing(&knob);
        d.fill(Ink::Yellow, 0.9, &knob);
        d.outline(Ink::Key, DETAIL, &knob);
    }
    d.outline(Ink::Key, INNER, &body);
}

// =========================================================================================
// Entry points
// =========================================================================================

/// The counter in front of everything (drawn as its own layer).
pub fn counter_front(d: &mut DrawList, b: Backdrop, h: f32) {
    match b {
        Backdrop::Bakehouse => bakehouse_front(d, h),
        Backdrop::Shopfront => shopfront_front(d, h),
        Backdrop::Pantry => pantry_front(d, h),
    }
}

/// Draw a full-screen backdrop (everything behind the gameplay) for an area `h` tall.
pub fn backdrop(d: &mut DrawList, b: Backdrop, h: f32) {
    backdrop_dressed(d, b, h, &Dressing::default());
}

/// [`backdrop`] with state-dependent dressing (keeps décor clear of the jars).
pub fn backdrop_dressed(d: &mut DrawList, b: Backdrop, h: f32, dress: &Dressing) {
    match b {
        Backdrop::Bakehouse => bakehouse(d, h, dress),
        Backdrop::Shopfront => shopfront(d, h),
        Backdrop::Pantry => pantry(d, h, dress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Shape as S;

    fn weight(d: &DrawList) -> (usize, usize) {
        let pts = d
            .cmds
            .iter()
            .map(|c| match &c.shape {
                S::Poly(p) => p.len(),
                S::PolysEo(ps) => ps.iter().map(|p| p.len()).sum(),
                S::Line { pts, .. } => pts.len(),
            })
            .sum();
        (d.cmds.len(), pts)
    }

    /// Backdrops are rasterised on phones at every screen change: keep them bounded.
    #[test]
    fn backdrops_stay_within_budget() {
        for h in [1280.0, 1560.0, 1800.0] {
            for b in [Backdrop::Bakehouse, Backdrop::Shopfront, Backdrop::Pantry] {
                let mut d = DrawList::new();
                let dress = Dressing { jars: jar_slots(b, 3) };
                backdrop_dressed(&mut d, b, h, &dress);
                let (cmds, pts) = weight(&d);
                assert!(cmds < 2500 && pts < 80_000, "{b:?} at {h}: {cmds} cmds, {pts} pts");
                let mut f = DrawList::new();
                counter_front(&mut f, b, h);
                let (cmds, pts) = weight(&f);
                assert!(cmds < 1200 && pts < 40_000, "{b:?} front at {h}: {cmds} cmds, {pts} pts");
            }
        }
    }

    /// Layout contracts: surfaces are ordered top to bottom and stay on screen.
    #[test]
    fn layout_contracts_are_ordered() {
        for h in [1280.0, 1400.0, 1560.0, 1800.0] {
            let l = bake_layout(h);
            assert!(bench_top(h) < l.dough.y - l.board_r, "board overlaps the wall at {h}");
            assert!(l.dough.y + l.board_r < l.trays[0] - 44.0, "board overlaps the trays at {h}");
            assert!(
                l.trays[1] + 44.0 < counter_top(Backdrop::Bakehouse, h),
                "trays hang off the bench at {h}"
            );
            assert!(l.bar < h, "bar off screen at {h}");
            let rows = case_rows(h);
            assert!(rows[0] > counter_top(Backdrop::Shopfront, h) + CASE_LOAF_R);
            assert!(rows[1] + CASE_SHELF + 16.0 < h - 30.0, "case shelf under the kick plate at {h}");
            assert!(jar_shelf(Backdrop::Pantry, h) + 60.0 < counter_top(Backdrop::Pantry, h));
        }
    }
}
