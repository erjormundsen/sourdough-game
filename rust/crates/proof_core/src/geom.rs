//! Tiny 2D geometry kit: vectors, transforms, polylines and shape tessellation.
//!
//! Everything in Proof's art is built from polygons and polylines in "reference units"
//! (the 720-wide portrait canvas), so the rasteriser only ever sees straight segments.

use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);

    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }
    pub fn len_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V2) -> f32 {
        self.x * o.y - self.y * o.x
    }
    /// Unit vector (zero stays zero).
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 { self / l } else { V2::ZERO }
    }
    /// Rotated 90° counter-clockwise (in y-down screen space this points "left" of travel).
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        self + (o - self) * t
    }
    pub fn rotate(self, a: f32) -> V2 {
        let (s, c) = a.sin_cos();
        v2(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    pub fn from_angle(a: f32) -> V2 {
        v2(a.cos(), a.sin())
    }
    pub fn mul_v(self, o: V2) -> V2 {
        v2(self.x * o.x, self.y * o.y)
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        *self = *self + o;
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, o: V2) {
        *self = *self - o;
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        v2(self.x * s, self.y * s)
    }
}
impl Div<f32> for V2 {
    type Output = V2;
    fn div(self, s: f32) -> V2 {
        v2(self.x / s, self.y / s)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}

/// Axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub const fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

impl Rect {
    pub fn center(&self) -> V2 {
        v2(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.x + self.w && p.y <= self.y + self.h
    }
    pub fn grow(&self, m: f32) -> Rect {
        rect(self.x - m, self.y - m, self.w + 2.0 * m, self.h + 2.0 * m)
    }
    pub fn union(&self, o: &Rect) -> Rect {
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        rect(x0, y0, x1 - x0, y1 - y0)
    }
    pub fn of_points(pts: &[V2]) -> Option<Rect> {
        let first = pts.first()?;
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for p in pts {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        Some(rect(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Similarity transform: scale, then rotate, then translate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    pub pos: V2,
    pub scale: V2,
    pub rot: f32,
}

impl Default for Xf {
    fn default() -> Self {
        Xf::IDENTITY
    }
}

impl Xf {
    pub const IDENTITY: Xf = Xf {
        pos: V2::ZERO,
        scale: v2(1.0, 1.0),
        rot: 0.0,
    };

    pub fn at(pos: V2) -> Xf {
        Xf {
            pos,
            ..Xf::IDENTITY
        }
    }
    pub fn scaled(self, s: f32) -> Xf {
        Xf {
            scale: self.scale * s,
            ..self
        }
    }
    pub fn scaled_xy(self, sx: f32, sy: f32) -> Xf {
        Xf {
            scale: self.scale.mul_v(v2(sx, sy)),
            ..self
        }
    }
    pub fn rotated(self, r: f32) -> Xf {
        Xf {
            rot: self.rot + r,
            ..self
        }
    }
    pub fn apply(&self, p: V2) -> V2 {
        p.mul_v(self.scale).rotate(self.rot) + self.pos
    }
    /// Compose: `self` applied after `inner`.
    pub fn then(&self, inner: &Xf) -> Xf {
        Xf {
            pos: self.apply(inner.pos),
            scale: self.scale.mul_v(inner.scale),
            rot: self.rot + inner.rot,
        }
    }
    /// Uniform-ish scale used for stroke widths.
    pub fn width_scale(&self) -> f32 {
        (self.scale.x.abs() * self.scale.y.abs()).sqrt()
    }
}

// ---------------------------------------------------------------------------
// Polylines
// ---------------------------------------------------------------------------

pub fn polyline_len(pts: &[V2]) -> f32 {
    pts.windows(2).map(|w| w[0].dist(w[1])).sum()
}

/// Resample a polyline into exactly `n` points evenly spaced along its length.
pub fn resample(pts: &[V2], n: usize) -> Vec<V2> {
    if pts.is_empty() || n == 0 {
        return Vec::new();
    }
    if pts.len() == 1 || n == 1 {
        return vec![pts[0]; n];
    }
    let total = polyline_len(pts);
    if total <= 1e-6 {
        return vec![pts[0]; n];
    }
    let step = total / (n - 1) as f32;
    let mut out = Vec::with_capacity(n);
    out.push(pts[0]);
    let mut seg = 0usize;
    let mut seg_start = 0.0f32;
    for i in 1..n - 1 {
        let target = step * i as f32;
        while seg < pts.len() - 2 && seg_start + pts[seg].dist(pts[seg + 1]) < target {
            seg_start += pts[seg].dist(pts[seg + 1]);
            seg += 1;
        }
        let sl = pts[seg].dist(pts[seg + 1]).max(1e-6);
        let t = ((target - seg_start) / sl).clamp(0.0, 1.0);
        out.push(pts[seg].lerp(pts[seg + 1], t));
    }
    out.push(*pts.last().unwrap());
    out
}

/// Chaikin corner cutting; keeps endpoints for open curves.
pub fn chaikin(pts: &[V2], iterations: usize, closed: bool) -> Vec<V2> {
    let mut cur = pts.to_vec();
    for _ in 0..iterations {
        if cur.len() < 3 {
            break;
        }
        let mut next = Vec::with_capacity(cur.len() * 2);
        let n = cur.len();
        if !closed {
            next.push(cur[0]);
        }
        let count = if closed { n } else { n - 1 };
        for i in 0..count {
            let a = cur[i];
            let b = cur[(i + 1) % n];
            next.push(a.lerp(b, 0.25));
            next.push(a.lerp(b, 0.75));
        }
        if !closed {
            next.push(cur[n - 1]);
        }
        cur = next;
    }
    cur
}

/// Quadratic bezier sampled into `n` segments.
pub fn quad_bezier(a: V2, c: V2, b: V2, n: usize) -> Vec<V2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            a * (u * u) + c * (2.0 * u * t) + b * (t * t)
        })
        .collect()
}

/// Cubic bezier sampled into `n` segments.
pub fn cubic_bezier(a: V2, c1: V2, c2: V2, b: V2, n: usize) -> Vec<V2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            a * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + b * (t * t * t)
        })
        .collect()
}

/// Point-in-polygon (even-odd).
pub fn point_in_poly(p: V2, poly: &[V2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Distance from `p` to segment `ab`.
pub fn dist_to_segment(p: V2, a: V2, b: V2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.len_sq().max(1e-9)).clamp(0.0, 1.0);
    p.dist(a + ab * t)
}

pub fn dist_to_polyline(p: V2, pts: &[V2]) -> f32 {
    match pts.len() {
        0 => f32::INFINITY,
        1 => p.dist(pts[0]),
        _ => pts
            .windows(2)
            .map(|w| dist_to_segment(p, w[0], w[1]))
            .fold(f32::INFINITY, f32::min),
    }
}

// ---------------------------------------------------------------------------
// Shapes (all return closed polygons without a repeated end point)
// ---------------------------------------------------------------------------

fn seg_count(r: f32) -> usize {
    ((r.abs() * 0.9).sqrt() * 6.0).clamp(40.0, 128.0) as usize
}

pub fn ellipse(c: V2, rx: f32, ry: f32, rot: f32) -> Vec<V2> {
    let n = seg_count(rx.max(ry));
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            c + v2(a.cos() * rx, a.sin() * ry).rotate(rot)
        })
        .collect()
}

pub fn circle(c: V2, r: f32) -> Vec<V2> {
    ellipse(c, r, r, 0.0)
}

/// Arc from angle `a0` to `a1` (radians, y-down so positive is clockwise on screen).
pub fn arc(c: V2, r: f32, a0: f32, a1: f32) -> Vec<V2> {
    let n = ((seg_count(r) as f32) * ((a1 - a0).abs() / TAU))
        .ceil()
        .max(4.0) as usize;
    (0..=n)
        .map(|i| c + V2::from_angle(a0 + (a1 - a0) * i as f32 / n as f32) * r)
        .collect()
}

pub fn rounded_rect(r: Rect, radius: f32) -> Vec<V2> {
    let rad = radius.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    let mut pts = Vec::new();
    let corners = [
        (v2(r.x + r.w - rad, r.y + rad), -PI * 0.5),
        (v2(r.x + r.w - rad, r.y + r.h - rad), 0.0),
        (v2(r.x + rad, r.y + r.h - rad), PI * 0.5),
        (v2(r.x + rad, r.y + rad), PI),
    ];
    for (c, start) in corners {
        if rad <= 0.01 {
            pts.push(c);
        } else {
            let a = arc(c, rad, start, start + PI * 0.5);
            pts.extend_from_slice(&a);
        }
    }
    pts
}

pub fn rect_poly(r: Rect) -> Vec<V2> {
    vec![
        v2(r.x, r.y),
        v2(r.x + r.w, r.y),
        v2(r.x + r.w, r.y + r.h),
        v2(r.x, r.y + r.h),
    ]
}

/// Squircle-ish soft blob: circle with a few low-frequency harmonics (deterministic by `seed`).
pub fn blob(c: V2, rx: f32, ry: f32, wobble: f32, seed: u32) -> Vec<V2> {
    let n = seg_count(rx.max(ry)) + 8;
    let s = seed as f32 * 0.6180339;
    let p1 = (s * 7.1).fract() * TAU;
    let p2 = (s * 13.7).fract() * TAU;
    let p3 = (s * 3.3).fract() * TAU;
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let k = 1.0
                + wobble
                    * (0.5 * (2.0 * a + p1).sin()
                        + 0.3 * (3.0 * a + p2).sin()
                        + 0.2 * (5.0 * a + p3).sin());
            c + v2(a.cos() * rx * k, a.sin() * ry * k)
        })
        .collect()
}

/// Five-point star (or `points`-pointed).
pub fn star(c: V2, r_out: f32, r_in: f32, points: usize, rot: f32) -> Vec<V2> {
    let n = points * 2;
    (0..n)
        .map(|i| {
            let r = if i % 2 == 0 { r_out } else { r_in };
            let a = rot - PI * 0.5 + TAU * i as f32 / n as f32;
            c + V2::from_angle(a) * r
        })
        .collect()
}

/// Rounded star: star with its corners softened (cute sparkle).
pub fn soft_star(c: V2, r_out: f32, r_in: f32, points: usize, rot: f32) -> Vec<V2> {
    chaikin(&star(c, r_out, r_in, points, rot), 2, true)
}

/// Heart centred on `c`, roughly `size` wide.
pub fn heart(c: V2, size: f32) -> Vec<V2> {
    let n = 64;
    (0..n)
        .map(|i| {
            let t = TAU * i as f32 / n as f32;
            let x = 16.0 * t.sin().powi(3);
            let y =
                -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
            c + v2(x, y + 1.5) * (size / 34.0)
        })
        .collect()
}

/// Capsule (stadium) from `a` to `b` with radius `r`.
pub fn capsule(a: V2, b: V2, r: f32) -> Vec<V2> {
    let d = (b - a).norm();
    let ang = d.angle();
    let mut pts = arc(b, r, ang - PI * 0.5, ang + PI * 0.5);
    pts.extend(arc(a, r, ang + PI * 0.5, ang + PI * 1.5));
    pts
}

/// Lens (vesica) along a polyline: width tapers to zero at both ends. `bias` in [-1, 1]
/// pushes the opening towards one side (the "ear").
pub fn lens_along(pts: &[V2], width: f32, bias: f32) -> Vec<V2> {
    let path = resample(pts, 24);
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
        let w = width * (PI * t).sin().max(0.0).powf(0.8);
        left.push(path[i] + nrm * (w * (0.5 + 0.5 * bias)));
        right.push(path[i] - nrm * (w * (0.5 - 0.5 * bias)));
    }
    right.reverse();
    left.extend(right);
    left
}

/// Scalloped circle (cloud / sheep wool / cake-cup edge).
pub fn scallop(c: V2, r: f32, bumps: usize, depth: f32) -> Vec<V2> {
    let n = bumps * 10;
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let k = 1.0 + depth * ((a * bumps as f32 * 0.5).sin().abs() - 0.6);
            c + V2::from_angle(a) * (r * k)
        })
        .collect()
}

/// Zig-zag edge between two points (ticket bottoms, "z" letters).
pub fn zigzag(a: V2, b: V2, teeth: usize, amp: f32) -> Vec<V2> {
    let d = b - a;
    let nrm = d.norm().perp();
    (0..=teeth * 2)
        .map(|i| {
            let t = i as f32 / (teeth * 2) as f32;
            a + d * t + nrm * if i % 2 == 1 { amp } else { 0.0 }
        })
        .collect()
}

pub fn translate(poly: &[V2], d: V2) -> Vec<V2> {
    poly.iter().map(|p| *p + d).collect()
}

pub fn transform(poly: &[V2], xf: &Xf) -> Vec<V2> {
    poly.iter().map(|p| xf.apply(*p)).collect()
}

/// Signed area (positive = clockwise in y-down space).
pub fn signed_area(poly: &[V2]) -> f32 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].cross(poly[(i + 1) % n]))
        .sum::<f32>()
        * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_endpoints_and_spacing() {
        let pts = vec![v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)];
        let r = resample(&pts, 5);
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], v2(0.0, 0.0));
        assert_eq!(r[4], v2(10.0, 10.0));
        assert!((r[2].dist(v2(10.0, 0.0))) < 1e-4);
    }

    #[test]
    fn xf_compose_matches_sequential_apply() {
        let outer = Xf::at(v2(5.0, 3.0)).scaled(2.0).rotated(0.3);
        let inner = Xf::at(v2(-1.0, 2.0)).scaled(0.5).rotated(-0.1);
        let p = v2(1.5, -0.7);
        let a = outer.apply(inner.apply(p));
        let b = outer.then(&inner).apply(p);
        assert!(a.dist(b) < 1e-4, "{a:?} vs {b:?}");
    }

    #[test]
    fn heart_and_star_are_closed_polys_inside_bounds() {
        let h = heart(v2(0.0, 0.0), 34.0);
        let b = Rect::of_points(&h).unwrap();
        assert!(b.w > 30.0 && b.w < 38.0, "{b:?}");
        assert!(point_in_poly(v2(0.0, 0.0), &h));
        let s = star(v2(0.0, 0.0), 10.0, 4.0, 5, 0.0);
        assert_eq!(s.len(), 10);
        assert!(point_in_poly(v2(0.0, 0.0), &s));
    }

    #[test]
    fn lens_is_thin_at_ends() {
        let l = lens_along(&[v2(0.0, 0.0), v2(100.0, 0.0)], 20.0, 0.0);
        let b = Rect::of_points(&l).unwrap();
        assert!((b.h - 20.0).abs() < 1.0);
        assert!(l[0].dist(v2(0.0, 0.0)) < 1e-3);
    }
}
