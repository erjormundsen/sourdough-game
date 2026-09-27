//! Construction kit for the regulars: idle pose, level of detail, silhouette ("halo") inking,
//! smooth outlines, fur, cloth and paws.
//!
//! # How a regular is inked
//! 1. **Halo pass.** Every part that can touch the silhouette (body, sleeves, paws, ears, head,
//!    hats) first gets a thick key stroke along its contour ([`Rig::halo`]).
//! 2. **Part pass, back to front.** Each part lays down opaque paper (`backing`), its ink
//!    recipe, halftone shading, then an `INNER` contour ([`Rig::part`]). The backing of every
//!    later part erases whatever halo or contour of earlier parts falls inside it, so only the
//!    outside of the union survives: the silhouette ends up `OUTER` thick, while the lines where
//!    parts meet or overlap (head over collar, paw over sleeve, ear into head) stay `INNER`.
//! 3. **Details and face** go on last in `DETAIL` / face weights.

use crate::art::face::{self, Expr};
use crate::art::style::{DETAIL, INNER, OUTER};
use crate::draw::{DrawList, Mode, Paint, Screen};
use crate::geom::{V2, circle, ellipse, quad_bezier, v2};
use crate::ink::Ink;
use std::f32::consts::{PI, TAU};

/// Where paws rest, in bust space: the shop counter's top edge (the shop prints busts at
/// 1.3× with their origin 40 units below the counter top, so the edge sits at -40 / 1.3).
pub const REST_Y: f32 = -30.8;

// ---------------------------------------------------------------------------
// Level of detail and idle pose
// ---------------------------------------------------------------------------

/// How big the regular prints, from the list's current transform.
#[derive(Clone, Copy, Debug)]
pub struct Lod {
    /// Print scale (1 = shop size before its 1.3× node scale).
    pub k: f32,
}

impl Lod {
    pub fn of(d: &DrawList) -> Lod {
        Lod { k: d.xf().width_scale().max(0.05) }
    }
    /// Board cards and order notes: halftone swaps for flat tints, hairlines disappear.
    pub fn small(&self) -> bool {
        self.k < 0.45
    }
    /// Line-weight boost so key lines stay printable when the bust is shrunk.
    fn boost(&self) -> f32 {
        if self.k < 0.7 { (0.7 / self.k).powf(0.55).min(1.8) } else { 1.0 }
    }
}

/// A quantised idle frame. The shop redraws three times a second; a 12-frame loop means the
/// texture cache replays the whole idle after the first four seconds instead of rasterising.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub frame: u32,
    /// Head bob (units, negative = up).
    pub bob: f32,
    /// Shoulder rise from breathing (units).
    pub breath: f32,
    /// Ear / whisker flick, -1..1.
    pub twitch: f32,
    /// Content faces blink once per loop.
    pub blink: bool,
}

pub const FRAMES: u32 = 12;

pub fn pose(t: f32) -> Pose {
    let frame = ((t * 3.0).round() as i64).rem_euclid(FRAMES as i64) as u32;
    let ph = frame as f32 / FRAMES as f32 * TAU;
    let q = |v: f32| (v * 4.0).round() / 4.0;
    Pose {
        frame,
        bob: q(-ph.sin() * 3.0),
        breath: q((0.5 - 0.5 * (ph - 0.5).cos()) * 1.6),
        twitch: match frame {
            7 => 1.0,
            8 => -0.45,
            _ => 0.0,
        },
        blink: frame == 10,
    }
}

// ---------------------------------------------------------------------------
// Rig
// ---------------------------------------------------------------------------

/// What the arms are doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmPose {
    /// Both paws on the counter edge.
    Rest,
    /// Both paws pressed to the cheeks ("loved it!").
    Cheer,
    /// One paw up in a little wave ("thanks!").
    Wave,
    /// Paws clasped under the chin ("pretty please?").
    Plead,
}

/// Everything a species needs to draw one frame.
#[derive(Clone, Copy, Debug)]
pub struct Rig {
    /// Head centre (bob included).
    pub h: V2,
    pub expr: Expr,
    pub pose: Pose,
    pub lod: Lod,
    /// Draw shoulders, clothes, arms and props (false for head-only portraits).
    pub body: bool,
    /// Compact ears/hats so a head-only portrait fits its frame.
    pub portrait: bool,
}

impl Rig {
    pub fn outer(&self) -> f32 {
        OUTER * self.lod.boost()
    }
    pub fn inner(&self) -> f32 {
        INNER * self.lod.boost()
    }
    pub fn detail(&self) -> f32 {
        DETAIL * self.lod.boost().powf(1.25)
    }
    pub fn small(&self) -> bool {
        self.lod.small()
    }
    /// Content faces blink once per idle loop.
    pub fn blinking(&self) -> bool {
        self.pose.blink && self.expr == Expr::Content
    }

    /// The shared face grammar with this frame's blink: eyes (+ brows) around `c`, blush,
    /// and — when `mouth_at` is given — the mouth there.
    pub fn face(&self, d: &mut DrawList, c: V2, s: f32, look: V2, blush: bool, mouth_at: Option<V2>) {
        if blush {
            face::blush(d, c, s, self.expr);
        }
        if self.blinking() {
            let lw = face::stroke_w(d, s);
            for sx in [-1.0f32, 1.0] {
                let e = c + v2(sx * s * 0.25, -s * 0.02) + look * (s * 0.04);
                d.line(
                    Ink::Key,
                    lw,
                    &crate::geom::arc(e + v2(0.0, -s * 0.03), s * 0.066, PI * 0.16, PI * 0.84),
                );
            }
        } else {
            face::eyes(d, c, s, self.expr, look);
        }
        if let Some(m) = mouth_at {
            face::mouth(d, m, s, self.expr);
        }
    }

    /// Breathing: points above the paws rise with the shoulders; the paws stay planted.
    pub fn breathe(&self, pts: &[V2]) -> Vec<V2> {
        let a = self.pose.breath;
        pts.iter()
            .map(|p| {
                let k = ((REST_Y - p.y) / 70.0).clamp(0.0, 1.4);
                v2(p.x * (1.0 + a * k * 0.004), p.y - a * k)
            })
            .collect()
    }

    /// Silhouette pre-stroke (see module docs). The stroke leans a touch towards the shadow
    /// side (down-right, away from the upper-left light), so the finished contour swells
    /// where the form turns away from the light and slims where it catches it, like a
    /// brush-inked line.
    pub fn halo(&self, d: &mut DrawList, poly: &[V2]) {
        let lean = v2(0.75, 0.9) * (self.outer() * 0.22);
        d.stroke_p(Paint::solid(Ink::Key, 1.0), 2.0 * self.outer() - self.inner(), &shift(poly, lean), true);
    }

    /// A riso "shadow plate": the silhouette offset down-right (light comes from the upper
    /// left, like every glass shine in the game) in blue halftone. Drawn before the halos so
    /// the character's own backing leaves only the sliver that falls on the wall.
    pub fn wall_shadow(&self, d: &mut DrawList, parts: &[&[V2]]) {
        if !self.body {
            return;
        }
        let off = v2(9.0, 7.0);
        for p in parts {
            self.tex(d, &shift(p, off), Ink::Blue, 0.22);
        }
    }

    /// Riso knockout under the blush: thin out the skin's yellow, blue and key tints where the
    /// cheeks go, so pink prints clean instead of turning muddy on green, gold, grey or brown.
    /// (Faces are drawn afterwards, so no line work is lost.)
    pub fn clear_cheeks(&self, d: &mut DrawList, c: V2, s: f32, strength: f32) {
        for sx in [-1.0f32, 1.0] {
            let b = ellipse(c + v2(sx * s * 0.39, s * 0.13), s * 0.125, s * 0.08, 0.0);
            d.knock_p(strength, Screen::Solid, 0b1110, &b);
        }
    }

    /// Body language for the expression: the shop's three reactions each get a pose.
    pub fn arm_pose(&self) -> ArmPose {
        if !self.body {
            return ArmPose::Rest;
        }
        match self.expr {
            Expr::Excited => ArmPose::Cheer,
            Expr::Happy => ArmPose::Wave,
            Expr::Hungry => ArmPose::Plead,
            _ => ArmPose::Rest,
        }
    }

    /// Excited (a "loved it!" reaction) throws both paws up to the cheeks.
    pub fn cheering(&self) -> bool {
        self.arm_pose() == ArmPose::Cheer
    }

    /// Raised arms cross in front of the chin, so they are drawn after the head.
    pub fn limbs_in_front(&self) -> bool {
        self.arm_pose() != ArmPose::Rest
    }

    /// Whether the arm on side `sx` is lifted off the counter in this pose.
    pub fn raised(&self, sx: f32) -> bool {
        match self.arm_pose() {
            ArmPose::Rest => false,
            ArmPose::Wave => sx > 0.0,
            ArmPose::Cheer | ArmPose::Plead => true,
        }
    }

    /// The shared arm path for side `sx`. Resting: shoulder, down the side, elbow on the
    /// counter, wrist turning in (`w` is the side reach, `wrist` the wrist's x). Raised arms
    /// swing the forearm up from the elbow to the paw placed by [`Rig::paw_place`].
    pub fn arm_path(&self, sx: f32, w: f32, wrist: f32) -> Vec<V2> {
        let sh = v2(sx * w * 0.87, -92.0);
        let ctrl = match (self.arm_pose(), self.raised(sx)) {
            (ArmPose::Cheer, _) => {
                [sh, v2(sx * (w + 3.0), -70.0), v2(sx * w * 0.9, -84.0), v2(sx * w * 0.66, -104.0)]
            }
            (ArmPose::Wave, true) => {
                [sh, v2(sx * (w + 8.0), -80.0), v2(sx * (w + 12.0), -100.0), v2(sx * (w + 4.0), -120.0)]
            }
            (ArmPose::Plead, _) => [sh, v2(sx * w, -62.0), v2(sx * w * 0.66, -58.0), v2(sx * w * 0.3, -68.0)],
            _ => [sh, v2(sx * w, -58.0), v2(sx * w * 0.84, -37.0), v2(sx * wrist, -40.0)],
        };
        self.breathe(&spline(&ctrl, false, 6))
    }

    /// Where a paw goes: its bottom-centre and tilt. Resting paws sit on the counter at `x`;
    /// raised paws land at the end of [`Rig::arm_path`]: against the jaw (cheer), up by the
    /// head (wave), or clasped together under the chin (plead).
    pub fn paw_place(&self, sx: f32, x: f32, w: f32, drop: f32) -> (V2, f32) {
        match (self.arm_pose(), self.raised(sx)) {
            (ArmPose::Cheer, _) => (v2(sx * w * 0.62, -106.0), -sx * 0.38),
            (ArmPose::Wave, true) => (v2(sx * (w + 1.0), -122.0), sx * 0.22),
            (ArmPose::Plead, _) => (v2(sx * 13.0, -70.0), -sx * 0.62),
            _ => (v2(sx * x, REST_Y + drop), 0.0),
        }
    }

    /// Little motion marks beside a waving paw whose top is at `top`.
    pub fn wave_marks(&self, d: &mut DrawList, top: V2, sx: f32) {
        if self.small() || self.arm_pose() != ArmPose::Wave || !self.raised(sx) {
            return;
        }
        for (k, rad) in [(0.0f32, 12.0f32), (1.0, 19.0)] {
            let c = top + v2(sx * (6.0 + k * 3.0), 10.0);
            let a0 = if sx > 0.0 { -PI * 0.42 } else { -PI * 0.58 - PI * 0.25 };
            d.line(Ink::Key, self.detail(), &crate::geom::arc(c, rad, a0, a0 + PI * 0.25));
        }
    }

    /// `INNER` contour around the union of `parts` only: each part's contour is clipped to
    /// the outside of all the others, so merged shapes (eye bumps, wool, spines) get no seams.
    pub fn union_outline(&self, d: &mut DrawList, parts: &[&[V2]]) {
        for (i, p) in parts.iter().enumerate() {
            let mut n = 0;
            for (j, q) in parts.iter().enumerate() {
                if i != j && bounds_touch(p, q) {
                    d.clip_push(&outside(q));
                    n += 1;
                }
            }
            d.outline(Ink::Key, self.inner(), p);
            for _ in 0..n {
                d.clip_pop();
            }
        }
    }

    /// Opaque part: backing, paint, `INNER` contour.
    pub fn part(&self, d: &mut DrawList, poly: &[V2], coat: Coat) {
        d.backing(poly);
        coat.ink(d, poly);
        d.outline(Ink::Key, self.inner(), poly);
    }

    /// Opaque part with a custom paint step between the backing and the contour.
    pub fn part_with(&self, d: &mut DrawList, poly: &[V2], paint: impl FnOnce(&mut DrawList, &[V2])) {
        d.backing(poly);
        paint(d, poly);
        d.outline(Ink::Key, self.inner(), poly);
    }

    pub fn seam(&self, d: &mut DrawList, pts: &[V2]) {
        d.line(Ink::Key, self.inner(), pts);
    }

    pub fn detail_line(&self, d: &mut DrawList, pts: &[V2]) {
        d.line(Ink::Key, self.detail(), pts);
    }

    /// Halftone form shadow `shadow` kept inside `part` (a flat wash when printed small).
    pub fn shade(&self, d: &mut DrawList, part: &[V2], shadow: &[V2], ink: Ink, tone: f32) {
        let small = self.small();
        d.clipped(part, |d| {
            if small {
                d.fill_p(Paint { ink, tone: tone * 0.55, screen: Screen::Solid, mode: Mode::Add }, shadow);
            } else {
                d.ht_add(ink, tone, shadow);
            }
        });
    }

    /// Form shading: a halftone crescent `width` deep along the side of `part` facing away
    /// from the light (`dir` points into the shadow, e.g. down-right).
    pub fn rim_shade(&self, d: &mut DrawList, part: &[V2], dir: V2, width: f32, ink: Ink, tone: f32) {
        if tone <= 0.0 {
            return;
        }
        let lit = shift(part, -dir.norm() * width);
        d.clip_push(&outside(&lit));
        self.shade(d, part, part, ink, tone);
        d.clip_pop();
    }

    /// Halftone texture fill (flat when small), unioned with the plate.
    pub fn tex(&self, d: &mut DrawList, poly: &[V2], ink: Ink, tone: f32) {
        if self.small() {
            d.fill_p(Paint { ink, tone: tone * 0.55, screen: Screen::Solid, mode: Mode::Add }, poly);
        } else {
            d.ht_add(ink, tone, poly);
        }
    }

    /// Dashed stitch line along `pts`.
    pub fn stitches(&self, d: &mut DrawList, pts: &[V2], dash: f32, gap: f32) {
        if self.small() {
            return;
        }
        for seg in dashes(pts, dash, gap) {
            d.line(Ink::Key, self.detail() * 0.8, &seg);
        }
    }
}

// ---------------------------------------------------------------------------
// Ink recipes
// ---------------------------------------------------------------------------

/// A flat overprint recipe: tones for the yellow, pink, blue and key plates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coat {
    pub y: f32,
    pub p: f32,
    pub b: f32,
    pub k: f32,
}

impl Coat {
    pub const fn new(y: f32, p: f32, b: f32, k: f32) -> Coat {
        Coat { y, p, b, k }
    }
    pub const PAPER: Coat = Coat::new(0.0, 0.0, 0.0, 0.0);
    /// Warm cream: muzzles, faces, chest fluff.
    pub const CREAM: Coat = Coat::new(0.3, 0.07, 0.0, 0.0);

    /// Knock the colour plates first so the recipe prints clean over whatever is below.
    pub fn ink_clean(&self, d: &mut DrawList, poly: &[V2]) {
        d.knock_p(1.0, Screen::Solid, crate::draw::PLATES_COLOR, poly);
        self.ink(d, poly);
    }

    pub fn ink(&self, d: &mut DrawList, poly: &[V2]) {
        for (ink, tone) in
            [(Ink::Yellow, self.y), (Ink::Pink, self.p), (Ink::Blue, self.b), (Ink::Key, self.k)]
        {
            if tone > 0.0 {
                d.fill(ink, tone, poly);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Smooth curve through `pts` (centripetal Catmull-Rom), `n` samples per span.
pub fn spline(pts: &[V2], closed: bool, n: usize) -> Vec<V2> {
    let m = pts.len();
    if m < 3 || n == 0 {
        return pts.to_vec();
    }
    let get = |i: isize| -> V2 {
        if closed {
            pts[i.rem_euclid(m as isize) as usize]
        } else if i < 0 {
            pts[0] * 2.0 - pts[1]
        } else if i >= m as isize {
            pts[m - 1] * 2.0 - pts[m - 2]
        } else {
            pts[i as usize]
        }
    };
    let spans = if closed { m } else { m - 1 };
    let mut out = Vec::with_capacity(spans * n + 1);
    for i in 0..spans as isize {
        let (p0, p1, p2, p3) = (get(i - 1), get(i), get(i + 1), get(i + 2));
        let kt = |a: V2, b: V2| a.dist(b).max(1e-3).sqrt();
        let t0 = 0.0;
        let t1 = t0 + kt(p0, p1);
        let t2 = t1 + kt(p1, p2);
        let t3 = t2 + kt(p2, p3);
        for k in 0..n {
            let t = t1 + (t2 - t1) * k as f32 / n as f32;
            let a1 = p0 * ((t1 - t) / (t1 - t0)) + p1 * ((t - t0) / (t1 - t0));
            let a2 = p1 * ((t2 - t) / (t2 - t1)) + p2 * ((t - t1) / (t2 - t1));
            let a3 = p2 * ((t3 - t) / (t3 - t2)) + p3 * ((t - t2) / (t3 - t2));
            let b1 = a1 * ((t2 - t) / (t2 - t0)) + a2 * ((t - t0) / (t2 - t0));
            let b2 = a2 * ((t3 - t) / (t3 - t1)) + a3 * ((t - t1) / (t3 - t1));
            out.push(b1 * ((t2 - t) / (t2 - t1)) + b2 * ((t - t1) / (t2 - t1)));
        }
    }
    if !closed {
        out.push(pts[m - 1]);
    }
    out
}

/// Closed smooth outline through `pts`.
pub fn blobby(pts: &[V2]) -> Vec<V2> {
    spline(pts, true, 8)
}

/// A body mound: `half` runs from the top of the axis down the right side to the bottom-right
/// corner (y = 0). The curve is mirrored and closed with a straight, flat bottom.
pub fn mound(half: &[V2]) -> Vec<V2> {
    let mut ctrl: Vec<V2> = half.iter().rev().filter(|p| p.x.abs() > 1e-3).map(|p| v2(-p.x, p.y)).collect();
    ctrl.extend_from_slice(half);
    let mut out = spline(&ctrl, false, 8);
    for p in &mut out {
        p.y = p.y.min(0.0);
    }
    out
}

/// Shrink (k < 1) or grow a shape about its centroid; closed loops get their first point
/// repeated so they can be stroked as open lines.
pub fn inset(pts: &[V2], k: f32, close: bool) -> Vec<V2> {
    if pts.is_empty() {
        return Vec::new();
    }
    let c = pts.iter().fold(V2::ZERO, |a, p| a + *p) / pts.len() as f32;
    let mut out: Vec<V2> = pts.iter().map(|p| c + (*p - c) * k).collect();
    if close {
        out.push(out[0]);
    }
    out
}

pub fn shift(pts: &[V2], o: V2) -> Vec<V2> {
    pts.iter().map(|p| *p + o).collect()
}

/// Rotate points about `pivot`.
pub fn rot(pts: &[V2], pivot: V2, a: f32) -> Vec<V2> {
    pts.iter().map(|p| pivot + (*p - pivot).rotate(a)).collect()
}

/// Curve from `a` to `b` bowed sideways by `bow` (positive bows to the left of travel).
pub fn bow(a: V2, b: V2, bow: f32, n: usize) -> Vec<V2> {
    let mid = a.lerp(b, 0.5) + (b - a).norm().perp() * bow;
    quad_bezier(a, mid, b, n)
}

/// A tube along `path` whose radius runs from `r0` to `r1`, with round ends.
pub fn tube(path: &[V2], r0: f32, r1: f32) -> Vec<V2> {
    tube_parts(path, r0, r1).0
}

/// [`tube`] plus its contour without the start cap (an open line), for sleeves whose
/// shoulder end melts into the body.
pub fn tube_parts(path: &[V2], r0: f32, r1: f32) -> (Vec<V2>, Vec<V2>) {
    let poly = tube_poly(path, r0, r1);
    let open = if poly.len() > 8 { poly[..poly.len() - 7].to_vec() } else { poly.clone() };
    (poly, open)
}

fn tube_poly(path: &[V2], r0: f32, r1: f32) -> Vec<V2> {
    let n = path.len();
    if n < 2 {
        return Vec::new();
    }
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let prev = path[i.saturating_sub(1)];
        let next = path[(i + 1).min(n - 1)];
        let nrm = (next - prev).norm().perp();
        let r = r0 + (r1 - r0) * t;
        left.push(path[i] + nrm * r);
        right.push(path[i] - nrm * r);
    }
    let end_dir = (path[n - 1] - path[n - 2]).norm();
    let start_dir = (path[1] - path[0]).norm();
    let mut out = left;
    // End cap: from left side round to right side.
    let a_end = end_dir.perp().angle();
    for k in 1..8 {
        let a = a_end - PI * k as f32 / 8.0;
        out.push(path[n - 1] + V2::from_angle(a) * r1);
    }
    out.extend(right.into_iter().rev());
    let a_start = (-start_dir.perp()).angle();
    for k in 1..8 {
        let a = a_start - PI * k as f32 / 8.0;
        out.push(path[0] + V2::from_angle(a) * r0);
    }
    out
}

/// A soft oval whose contour sprouts fur tufts: each `(angle, span, height, teeth)` swaps a
/// stretch of the contour for a little zig-zag of pointed tufts (angles in radians, y-down).
pub fn furry_oval(c: V2, rx: f32, ry: f32, tufts: &[(f32, f32, f32, usize)]) -> Vec<V2> {
    let n = 144;
    let at = |a: f32| c + v2(a.cos() * rx, a.sin() * ry);
    let normal = |a: f32| v2(a.cos() / rx, a.sin() / ry).norm();
    let norm_a = |a: f32| a.rem_euclid(TAU);
    let mut out = Vec::with_capacity(n + 32);
    let in_tuft = |a: f32| {
        tufts.iter().position(|(ta, span, _, _)| {
            let d = (norm_a(a - ta + PI) - PI).abs();
            d < span * 0.5
        })
    };
    let mut emitted = vec![false; tufts.len()];
    for i in 0..n {
        let a = TAU * i as f32 / n as f32;
        match in_tuft(a) {
            None => out.push(at(a)),
            Some(ti) => {
                if emitted[ti] {
                    continue;
                }
                emitted[ti] = true;
                let (ta, span, hgt, teeth) = tufts[ti];
                let a0 = ta - span * 0.5;
                let steps = teeth * 2;
                for s in 0..=steps {
                    let aa = a0 + span * s as f32 / steps as f32;
                    let base = at(aa);
                    if s % 2 == 1 {
                        // Tufts lean a little along the contour, like brushed fur: downwards
                        // on the sides, to the right on the crown.
                        let mut t = v2(-aa.sin() * rx, aa.cos() * ry).norm();
                        if t.y < -0.2 || (t.y.abs() <= 0.2 && t.x < 0.0) {
                            t = -t;
                        }
                        out.push(base + normal(aa) * hgt + t * (hgt * 0.35));
                    } else {
                        out.push(base);
                    }
                }
            }
        }
    }
    out
}

/// A clip region covering everything *except* `poly`: a huge rectangle with `poly` cut out
/// through a zero-width slit (opposite winding, so non-zero fill leaves a hole).
pub fn outside(poly: &[V2]) -> Vec<V2> {
    let big = 5000.0;
    let mut out = vec![v2(-big, -big), v2(big, -big), v2(big, big), v2(-big, big), v2(-big, -big)];
    let mut inner = poly.to_vec();
    if crate::geom::signed_area(&inner) > 0.0 {
        inner.reverse();
    }
    if let Some(first) = inner.first().copied() {
        out.extend(inner);
        out.push(first);
    }
    out
}

fn bounds_touch(a: &[V2], b: &[V2]) -> bool {
    match (crate::geom::Rect::of_points(a), crate::geom::Rect::of_points(b)) {
        (Some(ra), Some(rb)) => {
            let ra = ra.grow(8.0);
            ra.x < rb.x + rb.w && rb.x < ra.x + ra.w && ra.y < rb.y + rb.h && rb.y < ra.y + ra.h
        }
        _ => false,
    }
}

/// Push a contour out into soft round bumps (wool, clouds of fluff). `bump` is the target
/// bump length along the contour, `depth` how far each bump bulges outwards.
pub fn scalloped(pts: &[V2], closed: bool, bump: f32, depth: f32) -> Vec<V2> {
    // Outward = right of travel for clockwise (positive area) outlines in y-down space.
    let sign = if !closed || crate::geom::signed_area(pts) > 0.0 { -1.0 } else { 1.0 };
    scalloped_with(pts, closed, bump, depth, sign)
}

/// [`scalloped`] with an explicit outward side: `sign` = +1 bulges to the left of travel
/// (`perp()`), -1 to the right.
pub fn scalloped_with(pts: &[V2], closed: bool, bump: f32, depth: f32, sign: f32) -> Vec<V2> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut path = pts.to_vec();
    if closed {
        path.push(pts[0]);
    }
    let len = crate::geom::polyline_len(&path);
    let nb = (len / bump).round().max(1.0);
    let n = nb as usize * 12 + 1;
    let fine = crate::geom::resample(&path, n);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = fine[i.saturating_sub(1)];
        let next = fine[(i + 1).min(n - 1)];
        let nrm = (next - prev).norm().perp() * sign;
        let u = (i as f32 / (n - 1) as f32 * nb).fract();
        let k = (PI * u).sin().max(0.0).powf(0.55);
        out.push(fine[i] + nrm * (depth * k));
    }
    if closed {
        out.pop();
    }
    out
}

/// A curved spike (hedgehog spine, feather): `w` wide at `base`, pointed at `tip`,
/// bowed sideways by `bend`.
pub fn spike(base: V2, tip: V2, w: f32, bend: f32) -> Vec<V2> {
    let dir = (tip - base).norm();
    let side = dir.perp();
    let len = base.dist(tip);
    let mid = base.lerp(tip, 0.5) + side * bend;
    let a = base + side * (w * 0.5);
    let b = base - side * (w * 0.5);
    let mut out = quad_bezier(a, mid + side * (w * 0.22), tip, 8);
    out.extend(quad_bezier(tip, mid - side * (w * 0.22), b, 8).into_iter().skip(1));
    // Round the base.
    let back = base - dir * (w * 0.3).min(len * 0.2);
    out.extend(quad_bezier(b, back - side * (w * 0.3), back, 3).into_iter().skip(1));
    out.extend(quad_bezier(back, back + side * (w * 0.3), a, 3).into_iter().skip(1));
    out.pop();
    out
}

/// A soft spike: like [`spike`] but with a rounded tip and plump base (hedgehog quills that
/// read as cute rather than sharp).
pub fn soft_spike(base: V2, tip: V2, w: f32, bend: f32) -> Vec<V2> {
    let dir = (tip - base).norm();
    let side = dir.perp();
    let len = base.dist(tip);
    let mid = base.lerp(tip, 0.45) + side * bend;
    let near_tip = base.lerp(tip, 0.84) + side * (bend * 0.5);
    blobby(&[
        base + side * (w * 0.5),
        mid + side * (w * 0.36),
        near_tip + side * (w * 0.13),
        tip,
        near_tip - side * (w * 0.13),
        mid - side * (w * 0.36),
        base - side * (w * 0.5),
        base - dir * (w * 0.32).min(len * 0.2),
    ])
}

/// A crown of quills around the oval (`c`, `rx`, `ry`): between angles `a0`..`a1` (y-down
/// radians, increasing clockwise) the contour becomes `teeth` pointed quills reaching `len`
/// beyond the oval, swept back away from the face. Returns the outline and the quill tips.
#[allow(clippy::too_many_arguments)]
pub fn quill_crown(
    c: V2,
    rx: f32,
    ry: f32,
    a0: f32,
    a1: f32,
    teeth: usize,
    len: f32,
    seed: u32,
) -> (Vec<V2>, Vec<V2>) {
    let at = |a: f32, k: f32| c + v2(a.cos() * rx * k, a.sin() * ry * k);
    let step = (a1 - a0) / teeth as f32;
    let mut out = Vec::new();
    let mut tips = Vec::new();
    for i in 0..teeth {
        let v0 = a0 + step * i as f32;
        let v1 = v0 + step;
        let mid = v0 + step * 0.5;
        // Sweep: quills on the right lean clockwise, on the left anticlockwise.
        let lean = mid.cos() * step * 0.32;
        let jitter = 1.0 + (crate::rng::hash01(seed, i as u32) - 0.5) * 0.22;
        let rad = |a: f32| v2(a.cos() * rx, a.sin() * ry).len();
        let k_tip = 1.0 + len * jitter / rad(mid);
        let tip = at(mid + lean, k_tip);
        let base0 = at(v0, 1.0);
        let base1 = at(v1, 1.0);
        // Slightly convex flanks read as plump, organic quills.
        let bulge = |a: V2, b: V2| {
            let m = a.lerp(b, 0.5);
            let out_dir = (m - c).norm();
            m + out_dir * (len * 0.08)
        };
        if i == 0 {
            out.push(base0);
        }
        out.extend(quad_bezier(base0, bulge(base0, tip), tip, 5).into_iter().skip(1));
        out.extend(quad_bezier(tip, bulge(tip, base1), base1, 5).into_iter().skip(1));
        tips.push(tip);
    }
    // Close round the rest of the oval.
    let rest = (TAU - (a1 - a0)).max(0.0);
    let n = 40;
    for k in 1..n {
        out.push(at(a1 + rest * k as f32 / n as f32, 1.0));
    }
    (out, tips)
}

/// Parallel bands covering `clip` (and no more: the rasteriser sizes textures from unclipped
/// ink bounds, so patterns must not spill far past their piece). `angle` is the band
/// normal (0 = vertical bands), `pitch` the repeat, `width` the band width; bands are anchored
/// to the origin so neighbouring pieces of the same cloth line up.
pub fn bands(clip: &[V2], angle: f32, pitch: f32, width: f32, phase: f32) -> Vec<Vec<V2>> {
    let Some(b) = crate::geom::Rect::of_points(clip) else { return Vec::new() };
    let c = b.center();
    let rad = 0.5 * b.w.hypot(b.h) + 2.0;
    let n = V2::from_angle(angle);
    let t = n.perp();
    let uc = c.dot(n);
    let k0 = ((uc - rad - phase) / pitch).floor() as i32;
    let k1 = ((uc + rad - phase) / pitch).ceil() as i32;
    let bx = b.grow(1.0);
    (k0..=k1)
        .filter_map(|k| {
            let u = phase + k as f32 * pitch;
            let o = c + n * (u - uc);
            let band = [
                o + n * (-width * 0.5) - t * rad,
                o + n * (width * 0.5) - t * rad,
                o + n * (width * 0.5) + t * rad,
                o + n * (-width * 0.5) + t * rad,
            ];
            let cut = clip_to_rect(&band, bx);
            (cut.len() >= 3).then_some(cut)
        })
        .collect()
}

/// Sutherland–Hodgman: clip a polygon to an axis-aligned rectangle.
pub fn clip_to_rect(poly: &[V2], r: crate::geom::Rect) -> Vec<V2> {
    // Each edge: (axis 0 = x / 1 = y, boundary, keep the side above the boundary?).
    let edges = [(0, r.x, true), (0, r.x + r.w, false), (1, r.y, true), (1, r.y + r.h, false)];
    let mut pts = poly.to_vec();
    for (axis, bound, above) in edges {
        if pts.is_empty() {
            break;
        }
        let coord = |p: V2| if axis == 0 { p.x } else { p.y };
        let inside = |p: V2| if above { coord(p) >= bound } else { coord(p) <= bound };
        let cross = |a: V2, b: V2| a.lerp(b, (bound - coord(a)) / (coord(b) - coord(a)));
        let mut out = Vec::with_capacity(pts.len() + 4);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            match (inside(a), inside(b)) {
                (true, true) => out.push(b),
                (true, false) => out.push(cross(a, b)),
                (false, true) => {
                    out.push(cross(a, b));
                    out.push(b);
                }
                (false, false) => {}
            }
        }
        pts = out;
    }
    pts
}

/// Dot centres of a half-drop grid covering `clip` (anchored to the origin).
pub fn dot_grid(clip: &[V2], pitch: f32, row: f32, margin: f32) -> Vec<V2> {
    let Some(b) = crate::geom::Rect::of_points(clip) else { return Vec::new() };
    let b = b.grow(margin);
    let j0 = (b.y / row).floor() as i32;
    let j1 = ((b.y + b.h) / row).ceil() as i32;
    let mut out = Vec::new();
    for j in j0..=j1 {
        let off = if j.rem_euclid(2) == 1 { pitch * 0.5 } else { 0.0 };
        let i0 = ((b.x - off) / pitch).floor() as i32;
        let i1 = ((b.x + b.w - off) / pitch).ceil() as i32;
        for i in i0..=i1 {
            out.push(v2(i as f32 * pitch + off, j as f32 * row));
        }
    }
    out
}

/// Split a polyline into dashes.
pub fn dashes(pts: &[V2], dash: f32, gap: f32) -> Vec<Vec<V2>> {
    let total = crate::geom::polyline_len(pts);
    if total < 1e-3 {
        return Vec::new();
    }
    let n = (total / 1.5).ceil().max(2.0) as usize;
    let fine = crate::geom::resample(pts, n);
    let step = total / (n - 1) as f32;
    let mut out = Vec::new();
    let mut cur: Vec<V2> = Vec::new();
    for (i, p) in fine.iter().enumerate() {
        let s = i as f32 * step;
        let on = (s % (dash + gap)) < dash;
        if on {
            cur.push(*p);
        } else if cur.len() >= 2 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.clear();
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

// ---------------------------------------------------------------------------
// Shared anatomy
// ---------------------------------------------------------------------------

/// A resting paw (or mitten, wing tip, hoof) whose bottom centre is `c`, `w` wide, `h` tall.
/// The bottom sits a little below `REST_Y` so the counter's edge line doubles as the paw's
/// underside in the shop.
pub fn paw_shape(c: V2, w: f32, h: f32) -> Vec<V2> {
    blobby(&[
        c + v2(0.0, -h),
        c + v2(w * 0.36, -h * 0.94),
        c + v2(w * 0.5, -h * 0.45),
        c + v2(w * 0.42, -h * 0.02),
        c + v2(0.0, h * 0.06),
        c + v2(-w * 0.42, -h * 0.02),
        c + v2(-w * 0.5, -h * 0.45),
        c + v2(-w * 0.36, -h * 0.94),
    ])
}

/// Toe notches on a paw whose bottom-centre is `c`, tilted by `tilt` about it. Resting paws
/// show their toes along the bottom edge; `raised` paws at the fingertips.
#[allow(clippy::too_many_arguments)]
pub fn toes_at(r: &Rig, d: &mut DrawList, c: V2, w: f32, h: f32, n: usize, tilt: f32, raised: bool) {
    if r.small() {
        return;
    }
    for i in 1..n {
        let x = (i as f32 / n as f32 - 0.5) * w * 0.86;
        let (a, b) = if raised {
            (c + v2(x * 0.8, -h * 1.0), c + v2(x * 0.9, -h * 0.62))
        } else {
            (c + v2(x, -h * 0.02), c + v2(x * 0.9, -h * 0.42))
        };
        r.detail_line(d, &rot(&[a, b], c, tilt));
    }
}

/// Fabric creases where a sleeve bends at the elbow: two short folds running in from the
/// inside of the bend, kept inside `sleeve`. `path` is the arm path, `rad` its radius near the
/// elbow. Dark cloth (`light`) gets pale, knocked-out folds instead of key lines.
pub fn elbow_creases(r: &Rig, d: &mut DrawList, sleeve: &[V2], path: &[V2], rad: f32, light: bool) {
    if r.small() || path.len() < 12 {
        return;
    }
    let i = path.len() * 11 / 20;
    let p = path[i];
    let dir = (path[i + 1] - path[i - 1]).norm();
    // The inside of the bend faces the body's centre line.
    let mut n = dir.perp();
    if (p + n).x.abs() > p.x.abs() {
        n = -n;
    }
    let w = r.detail() * 0.85;
    d.clipped(sleeve, |d| {
        for (k, (along, len)) in [(-0.3f32, 0.46f32), (0.28, 0.32)].iter().enumerate() {
            let a = p + n * (rad * 1.05) + dir * (rad * along);
            let b = a - n * (rad * (len + 0.15)) + dir * (rad * 0.16 * (k as f32 - 0.5));
            let crease = bow(a, b, rad * 0.1, 5);
            if light {
                d.knock_line(0.5, w * 1.2, &crease, false);
            } else {
                d.line(Ink::Key, w, &crease);
            }
        }
    });
}

/// Soft halftone shadow cast by the head onto whatever `under` is (neck shadow).
pub fn neck_shadow(r: &Rig, d: &mut DrawList, under: &[V2], head_rx: f32, head_ry: f32, drop: f32) {
    let sh = ellipse(r.h + v2(0.0, drop), head_rx * 0.94, head_ry, 0.0);
    r.shade(d, under, &sh, Ink::Blue, 0.27);
}

/// A shiny round button: fill, paper glint, contour.
pub fn button(r: &Rig, d: &mut DrawList, c: V2, rad: f32, coat: Coat) {
    let b = circle(c, rad);
    d.backing(&b);
    coat.ink(d, &b);
    if !r.small() {
        d.knock(&circle(c + v2(-rad * 0.3, -rad * 0.32), rad * 0.3));
        let hole = rad * 0.22;
        d.fill(Ink::Key, 0.8, &circle(c + v2(-hole, hole * 0.2), hole * 0.45));
        d.fill(Ink::Key, 0.8, &circle(c + v2(hole, hole * 0.2), hole * 0.45));
    }
    d.outline(Ink::Key, r.detail(), &b);
}

/// Little key-ink tuft strokes (fur texture) fanning from `c` along `dir`.
pub fn fur_flick(r: &Rig, d: &mut DrawList, c: V2, dir: V2, len: f32, n: usize) {
    if r.small() {
        return;
    }
    let side = dir.perp();
    for i in 0..n {
        let o = (i as f32 - (n as f32 - 1.0) * 0.5) * len * 0.42;
        let a = c + side * o;
        let b = a + dir * len + side * (o * 0.35);
        r.detail_line(d, &bow(a, b, len * 0.12, 4));
    }
}
