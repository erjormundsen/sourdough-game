//! Dough and loaves, seen from above (for scoring) and in cross-section (crumb shots).
//!
//! The house light comes from the top-left. A loaf is printed back to front:
//! cast shadow → paper backing → crust (a lit dome: halftone browning that deepens towards
//! the lower-right rim) → recipe character (bran, rye crackle, inclusions) → banneton flour
//! coil → stencil flour with grainy edges → cuts (grooves in raw dough; torn blooms with a
//! lifted, shadow-casting ear once baked) → toppings (clustered, oriented, pushed aside by
//! the blooms) → key contour.
//!
//! `bake` (oven) and `bloom`/`ear` (reveal) are animated on screen, so every pass
//! interpolates smoothly between raw dough and a finished loaf.
//!
//! Clip rule: the rasteriser's "Over" fills zero out masked-off pixels in the same span, so
//! everything printed inside a clip (the cuts pass, and the whole loaf when Toasty draws it
//! inside his window) knocks to paper and then *adds* ink, which prints the same colour and
//! is always clip-safe. Passes outside the clip stay within the silhouette by construction.

use super::style::{DETAIL, INNER, OUTER};
use crate::content::{Inclusion, Recipe, Shape, Stencil, Topping};
use crate::draw::{Cmd, DrawList, Op, PLATES_ALL, PLATES_COLOR, Paint, Screen};

/// Pink + yellow plates.
const PY: u8 = 0b0011;
/// Pink + key plates (lifting these leaves a golden highlight).
const PK: u8 = 0b1001;
/// Pink + yellow + key plates (everything a crust prints with).
const PYK: u8 = 0b1011;
use crate::geom::{
    V2, Xf, chaikin, circle, ellipse, heart, point_in_poly, polyline_len, resample, soft_star, v2,
};
use crate::ink::Ink;
use crate::rng::{Rng, hash01};
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
                // Rounded rays (a stencil can't cut needle points).
                v.push(chaikin(&[dir * 0.27 + p, dir * 0.43, dir * 0.27 - p], 2, true));
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

/// Loaf size multiplier from spring + bake.
pub fn loaf_scale(v: &LoafView) -> f32 {
    1.0 + 0.12 * v.spring.clamp(0.0, 1.0) * v.bake.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------------------
// Small drawing helpers (multi-polygon commands keep hundreds of specks to one command).
// ---------------------------------------------------------------------------------------

/// Direction towards the house light (top-left).
const LIGHT: V2 = v2(-0.5547, -0.8321);

fn tx_polys(d: &DrawList, polys: &[Vec<V2>]) -> Vec<Vec<V2>> {
    let xf = d.xf();
    polys.iter().filter(|p| p.len() >= 3).map(|p| p.iter().map(|q| xf.apply(*q)).collect()).collect()
}

/// Knock many small shapes in one command (even-odd, so keep them from overlapping).
fn knock_many(d: &mut DrawList, tone: f32, screen: Screen, plates: u8, polys: &[Vec<V2>]) {
    let ps = tx_polys(d, polys);
    if ps.is_empty() || tone <= 0.0 {
        return;
    }
    d.cmds.push(Cmd { op: Op::Knock { tone, screen, plates }, shape: crate::draw::Shape::PolysEo(ps) });
}

/// Ink many small shapes in one command (even-odd, so keep them from overlapping).
fn fill_many(d: &mut DrawList, paint: Paint, polys: &[Vec<V2>]) {
    let ps: Vec<Vec<V2>> = polys.iter().filter(|p| p.len() >= 3).cloned().collect();
    if ps.is_empty() || paint.tone <= 0.0 {
        return;
    }
    d.fill_eo(paint, &ps);
}

/// Unit normals along an open polyline.
fn normals(path: &[V2]) -> Vec<V2> {
    let n = path.len();
    (0..n)
        .map(|i| {
            let a = path[i.saturating_sub(1)];
            let b = path[(i + 1).min(n - 1)];
            (b - a).norm().perp()
        })
        .collect()
}

/// A filled calligraphic stroke along `path` with half-width `hw(t)`, t = 0..1.
fn taper(path: &[V2], hw: impl Fn(f32) -> f32) -> Vec<V2> {
    let n = path.len();
    if n < 2 {
        return Vec::new();
    }
    let nrm = normals(path);
    let mut left = Vec::with_capacity(n * 2);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let w = hw(t);
        left.push(path[i] + nrm[i] * w);
        right.push(path[i] - nrm[i] * w);
    }
    right.reverse();
    left.extend(right);
    left
}

/// Irregular rounded blob with a few low harmonics, stretched along `rot`.
fn lump(c: V2, rx: f32, ry: f32, rot: f32, wobble: f32, seed: u32, n: usize) -> Vec<V2> {
    let p1 = hash01(seed, 11) * TAU;
    let p2 = hash01(seed, 12) * TAU;
    let p3 = hash01(seed, 13) * TAU;
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let k = 1.0
                + wobble
                    * (0.55 * (2.0 * a + p1).sin()
                        + 0.3 * (3.0 * a + p2).sin()
                        + 0.15 * (5.0 * a + p3).sin());
            c + v2(a.cos() * rx * k, a.sin() * ry * k).rotate(rot)
        })
        .collect()
}

/// A 0 → 1 → 0 arch over t in [0, 1] (sin^p, safe at the ends: sin(π) is a hair negative).
fn arch(t: f32, p: f32) -> f32 {
    (PI * t.clamp(0.0, 1.0)).sin().max(0.0).powf(p)
}

fn translate_poly(poly: &[V2], o: V2) -> Vec<V2> {
    poly.iter().map(|p| *p + o).collect()
}

/// A small ellipse as an `n`-gon (specks, seeds and grains don't need 40 points).
fn speck(c: V2, rx: f32, ry: f32, rot: f32, n: usize) -> Vec<V2> {
    let (sr, cr) = rot.sin_cos();
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            c + v2(x * cr - y * sr, x * sr + y * cr)
        })
        .collect()
}

/// Smooth value noise in [0, 1) (bilinear over a hashed lattice).
fn vnoise(p: V2, seed: u32) -> f32 {
    let (x0, y0) = (p.x.floor(), p.y.floor());
    let (fx, fy) = (p.x - x0, p.y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let h = |x: f32, y: f32| {
        hash01(seed, ((x as i32 + 4096) as u32).wrapping_mul(7919) ^ (y as i32 + 4096) as u32)
    };
    let a = h(x0, y0) + (h(x0 + 1.0, y0) - h(x0, y0)) * sx;
    let b = h(x0, y0 + 1.0) + (h(x0 + 1.0, y0 + 1.0) - h(x0, y0 + 1.0)) * sx;
    a + (b - a) * sy
}

/// Off-centre radial gradient printed last-wins: nested copies of `body` shrinking towards
/// `focus`, tone running from `rim` to `core`. The rings are gently warped so the steps read
/// as an organic halftone gradient rather than a target.
#[allow(clippy::too_many_arguments)]
fn dome(
    d: &mut DrawList,
    ink: Ink,
    screen: Screen,
    body: &[V2],
    focus: V2,
    inner: f32,
    rim: f32,
    core: f32,
    steps: usize,
    curve: f32,
    seed: u32,
    seam: f32,
) {
    let steps = steps.max(2);
    if std::env::var("NO_DOME").is_ok() {
        return;
    }
    let rings: Vec<Vec<V2>> = (0..steps)
        .map(|i| {
            let t = i as f32 / (steps - 1) as f32;
            let s = 1.0 - (1.0 - inner) * t;
            let ph = hash01(seed, i as u32) * TAU;
            body.iter()
                .map(|p| {
                    let q = *p - focus;
                    let a = q.y.atan2(q.x);
                    let wob =
                        1.0 + 0.05 * t * (0.6 * (3.0 * a + ph).sin() + 0.4 * (5.0 * a - 1.7 * ph).sin());
                    focus + q * (s * wob)
                })
                .collect()
        })
        .collect();
    // Each step paints an annulus: ring i minus ring i+1 pulled in by `seam`. The next step
    // repaints that thin overlap completely, so no anti-aliased seam survives, yet every
    // pixel is painted about once rather than once per enclosing ring.
    let pull = |ring: &[V2]| -> Vec<V2> {
        ring.iter()
            .map(|p| {
                let q = *p - focus;
                let l = q.len();
                focus + q * ((l - seam).max(l * 0.5) / l.max(1e-6))
            })
            .collect()
    };
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let tone = (rim + (core - rim) * t.powf(curve)).clamp(0.0, 1.0);
        let polys: Vec<Vec<V2>> =
            if i + 1 < steps { vec![rings[i].clone(), pull(&rings[i + 1])] } else { vec![rings[i].clone()] };
        if tone < 0.01 {
            knock_many(d, 1.0, Screen::Solid, 1 << ink.idx(), &polys);
        } else {
            fill_many(d, Paint { ink, tone, screen, mode: crate::draw::Mode::Over }, &polys);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Loaf geometry
// ---------------------------------------------------------------------------------------

/// Point on the unit silhouette at angle `a` (no wobble).
fn sil_at(shape: Shape, a: f32) -> V2 {
    let (rx, ry) = shape.radii();
    let (s, c) = a.sin_cos();
    match shape {
        Shape::Boule => v2(c * rx, s * ry),
        // A bâtard is a torpedo: full through the middle, easing into blunt rounded ends.
        Shape::Batard => {
            let e = 2.0 / 2.25;
            v2(c.signum() * c.abs().powf(e) * rx, s.signum() * s.abs().powf(e) * ry * (1.0 - 0.1 * c * c))
        }
    }
}

fn silhouette(shape: Shape, seed: u32) -> Vec<V2> {
    let n = 132;
    let p1 = hash01(seed, 1) * TAU;
    let p2 = hash01(seed, 2) * TAU;
    let p3 = hash01(seed, 3) * TAU;
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let wob = 1.0
                + 0.011 * (2.0 * a + p1).sin()
                + 0.008 * (3.0 * a + p2).sin()
                + 0.006 * (5.0 * a + p3).sin();
            sil_at(shape, a) * wob
        })
        .collect()
}

/// One cut's opening, in loaf units.
struct Bloom {
    path: Vec<V2>,
    /// Unit normals pointing to the ear side (towards the light).
    nrm: Vec<V2>,
    len: f32,
    /// bake × bloom.
    open: f32,
    /// ear × open.
    ear: f32,
    /// Full opening width at the widest point.
    width: f32,
    /// Lens profile 0..1 per path point.
    prof: Vec<f32>,
    /// Half-widths towards the ear side / far side per path point.
    we: Vec<f32>,
    wf: Vec<f32>,
    ear_edge: Vec<V2>,
    far_edge: Vec<V2>,
    /// The opening: ear edge forward, far edge back.
    lens: Vec<V2>,
}

impl Bloom {
    fn new(cut: &CutView, bake: f32, seed: u32) -> Option<Bloom> {
        if cut.pts.len() < 2 {
            return None;
        }
        let smooth = chaikin(&resample(&cut.pts, 12), 2, false);
        let len = polyline_len(&smooth);
        if len < 0.02 {
            return None;
        }
        let n = ((len * 44.0) as usize).clamp(10, 64);
        let path = resample(&smooth, n);
        let mut nrm = normals(&path);
        let avg = nrm.iter().fold(V2::ZERO, |a, q| a + *q);
        if avg.dot(LIGHT) < 0.0 {
            nrm.iter_mut().for_each(|q| *q = -*q);
        }
        let open = (bake.clamp(0.0, 1.0) * cut.bloom.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let ear = cut.ear.clamp(0.0, 1.0) * open;
        // Raw grooves are a slit; blooms open wider on long confident cuts.
        let width = 0.036 + open * (0.07 + 0.17 * (len / 1.2).min(1.0)) * (1.0 + 0.3 * ear);
        let skew = 0.85 + 0.3 * hash01(seed, 5);
        let mut rng = Rng::new(seed as u64 * 31 + 7);
        let mut we = Vec::with_capacity(n);
        let mut wf = Vec::with_capacity(n);
        let mut profs = Vec::with_capacity(n);
        let tear = (open - 0.12).clamp(0.0, 1.0);
        // Where the crust tore in chunks: a couple of notches along the far edge.
        let notches: Vec<(f32, f32)> = (0..2).map(|_| (rng.range(0.2, 0.8), rng.range(0.04, 0.09))).collect();
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let prof = (PI * t.powf(skew)).sin().max(0.0).powf(0.72);
            let rough = (vnoise(v2(t * len * 22.0, 0.5), seed) - 0.5) * 0.34
                + (vnoise(v2(t * len * 60.0, 2.5), seed ^ 7) - 0.5) * 0.18;
            let notch: f32 = notches.iter().map(|(c, w)| -0.28 * (-((t - c) / w).powi(2)).exp()).sum();
            let serr = 1.0 + tear * 0.05 * (vnoise(v2(t * len * 40.0, 7.5), seed ^ 3) - 0.5);
            profs.push(prof);
            we.push(width * prof * (0.5 - 0.2 * ear) * serr);
            wf.push(width * prof * (0.5 + 0.22 * ear) * (1.0 + tear * (rough + notch)));
        }
        let ear_edge: Vec<V2> = (0..n).map(|i| path[i] + nrm[i] * we[i]).collect();
        let far_edge: Vec<V2> = (0..n).map(|i| path[i] - nrm[i] * wf[i]).collect();
        let mut lens = ear_edge.clone();
        lens.extend(far_edge.iter().rev().skip(1).take(n.saturating_sub(2)));
        Some(Bloom { path, nrm, len, open, ear, width, prof: profs, we, wf, ear_edge, far_edge, lens })
    }

    /// A band inside the opening between fractions `a` and `b` of the way across it
    /// (0 = the ear-side edge, 1 = the torn far edge).
    fn strip(&self, a: f32, b: f32) -> Vec<V2> {
        let n = self.path.len();
        let at = |i: usize, f: f32| self.path[i] + self.nrm[i] * (self.we[i] - (self.we[i] + self.wf[i]) * f);
        let mut p: Vec<V2> = (0..n).map(|i| at(i, a)).collect();
        p.extend((0..n).rev().map(|i| at(i, b)));
        p
    }

    /// The opening grown by `g` on both sides (and a little past the ends).
    fn grown(&self, g: f32) -> Vec<V2> {
        let n = self.path.len();
        let tan0 = (self.path[0] - self.path[1.min(n - 1)]).norm();
        let tan1 = (self.path[n - 1] - self.path[n.saturating_sub(2)]).norm();
        let mut left = vec![self.path[0] + tan0 * g * 0.8];
        let mut right = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let taperk = (PI * t).sin().max(0.0).powf(0.35);
            left.push(self.path[i] + self.nrm[i] * (self.we[i] + g * taperk));
            right.push(self.path[i] - self.nrm[i] * (self.wf[i] + g * taperk));
        }
        left.push(self.path[n - 1] + tan1 * g * 0.8);
        right.reverse();
        left.extend(right);
        left
    }

    /// Signed distance of `p` across the cut (positive towards the ear side) and the
    /// nearest path index, if `p` is alongside the cut.
    fn across(&self, p: V2) -> Option<(f32, usize)> {
        let mut best: Option<(f32, f32, usize)> = None;
        for (i, q) in self.path.iter().enumerate() {
            let dd = (p - *q).len_sq();
            if best.is_none_or(|b| dd < b.0) {
                best = Some((dd, (p - *q).dot(self.nrm[i]), i));
            }
        }
        let (_, s, i) = best?;
        let n = self.path.len();
        if i == 0 || i == n - 1 {
            // Beyond the ends: only count points that sit right beside the tip.
            let tan = (self.path[i] - self.path[if i == 0 { 1 } else { n - 2 }]).norm();
            if (p - self.path[i]).dot(tan) > 0.0 {
                return None;
            }
        }
        Some((s, i))
    }
}

/// Everything the loaf passes share.
struct Loaf<'a> {
    v: &'a LoafView,
    /// Reference units per loaf unit (radius × spring).
    k: f32,
    rx: f32,
    ry: f32,
    b: f32,
    crust: f32,
    /// Level of detail: 0 (icon) .. 1 (hero).
    lod: f32,
    body: Vec<V2>,
    blooms: Vec<Bloom>,
}

impl Loaf<'_> {
    /// Overlap between gradient rings: ~2.5 reference units, in loaf units.
    fn seam(&self) -> f32 {
        2.5 / self.k
    }

    /// Line width in loaf units for a stroke that should print `w` reference units wide at
    /// hero size (thinner on small loaves, never hairline).
    fn lw(&self, w: f32) -> f32 {
        let s = (self.v.r / 150.0).clamp(0.3, 1.25);
        (w * (0.45 + 0.55 * s)).max(0.9) / self.k
    }

    /// Where a crust point ends up once the blooms have pushed the crust apart.
    fn displaced(&self, p: V2, margin: f32) -> Option<V2> {
        let mut q = p;
        for bl in &self.blooms {
            if let Some((s, i)) = bl.across(q) {
                let (we, wf) = (bl.we[i] + margin, bl.wf[i] + margin);
                if s >= 0.0 && s < we {
                    // Riding up onto the ear side.
                    q = bl.path[i] + bl.nrm[i] * (we + s * 0.35);
                } else if s < 0.0 && -s < wf {
                    q = bl.path[i] - bl.nrm[i] * (wf - s * 0.35);
                }
            }
        }
        let inside = (q.x / self.rx).powi(2) + (q.y / self.ry).powi(2) < 0.96;
        inside.then_some(q)
    }
}

/// Draw a loaf (or raw dough when `bake` = 0) centred at the origin.
///
/// Footprint contract: a boule spans ±`r` (bâtard ±`r`·[`Shape::radii`]), growing by up to
/// 12% with oven spring; cut points are in loaf-normalised units (see `scoring`).
pub fn loaf_top(d: &mut DrawList, v: &LoafView) {
    let k = v.r * loaf_scale(v);
    let (rx, ry) = v.shape.radii();
    let b = v.bake.clamp(0.0, 1.0);
    let blooms: Vec<Bloom> = v
        .cuts
        .iter()
        .enumerate()
        .filter_map(|(i, c)| Bloom::new(c, b, v.seed.wrapping_mul(97).wrapping_add(i as u32 * 13)))
        .collect();
    let l = Loaf {
        v,
        k,
        rx,
        ry,
        b,
        crust: v.crust.clamp(0.0, 1.0),
        lod: ((v.r - 34.0) / 96.0).clamp(0.0, 1.0),
        body: silhouette(v.shape, v.seed),
        blooms,
    };

    cast_shadow(d, &l);
    d.push(Xf::IDENTITY.scaled(k));
    d.backing(&l.body);
    // Everything up to the stencil stays inside the silhouette by construction and prints
    // unclipped (a clip mask costs a pass per command); cuts can run off the loaf with a
    // freehand swipe, so they and what sits on them are clipped.
    crust(d, &l);
    character(d, &l);
    banneton(d, &l);
    if let Some(st) = v.stencil {
        stencil_flour(d, &l, st);
    }
    rim_light(d, &l);
    d.clipped(&l.body, |d| {
        browning(d, &l);
        cuts(d, &l);
        if let Some(inc) = v.recipe.inclusion() {
            inclusions_in_blooms(d, &l, inc);
        }
        if let Some(t) = v.topping {
            toppings(d, &l, t);
        }
    });
    d.outline(Ink::Key, l.lw(OUTER * 1.02), &l.body);
    d.pop();
}

/// Soft drop shadow falling to the lower-right, with a denser core where the loaf sits.
fn cast_shadow(d: &mut DrawList, l: &Loaf) {
    let k = l.k;
    let lift = 0.45 + 0.55 * l.b.max(0.3);
    let off = v2(0.05, 0.1) * k * lift;
    // Only the crescent that peeks out from under the loaf is printed (the loaf's backing
    // would knock the rest anyway).
    let under: Vec<V2> = l.body.iter().map(|p| *p * (k * 0.97)).collect();
    let outer: Vec<V2> = l.body.iter().map(|p| *p * (k * 1.03) + off).collect();
    d.fill_eo(Paint::ht(Ink::Blue, 0.3), &[outer, under.clone()]);
    let core: Vec<V2> = l.body.iter().map(|p| *p * (k * 0.99) + off * 0.45).collect();
    d.fill_eo(Paint::solid(Ink::Blue, 0.22).add(), &[core, under]);
}

/// Extra browning a recipe's crust takes on the (pink, key) plates.
fn recipe_tint(r: Recipe) -> (f32, f32) {
    match r {
        // Rye bakes to a cocoa brown rather than a red-gold.
        Recipe::DarkRye => (-0.12, 0.3),
        Recipe::WholeWheat => (0.04, 0.1),
        Recipe::Cheddar => (0.05, 0.0),
        _ => (0.0, 0.0),
    }
}

/// The crust: a lit dome. Yellow warms with the bake; pink browns, deepest to the
/// lower-right; blue and key add depth to bold bakes and rye.
fn crust(d: &mut DrawList, l: &Loaf) {
    let v = l.v;
    let (b, c) = (l.b, l.crust);
    let raw = 1.0 - b;
    let steps = (4.0 + 6.0 * l.lod) as usize;
    let focus = v2(-0.3 * l.rx, -0.34 * l.ry);
    let (pe, ke) = recipe_tint(v.recipe);

    // Warm cream dough → gold.
    let y_rim = 0.58 * raw + b * (0.95 - 0.08 * ke);
    let y_core = 0.4 * raw + b * (0.84 - 0.1 * c - 0.1 * ke);
    dome(
        d,
        Ink::Yellow,
        Screen::Solid,
        &l.body,
        focus,
        0.3,
        y_rim,
        y_core,
        3 + (l.lod > 0.5) as usize,
        1.0,
        v.seed,
        l.seam(),
    );

    // Browning (a warm shade on the unlit side of raw dough).
    let p_rim = 0.13 * raw + b * (0.18 + 0.62 * c + pe);
    let p_core = 0.02 * raw + b * (0.02 + 0.36 * c + pe * 0.8);
    // Raw dough takes a soft flat-tint blush (fine dots would read as speckle on dough);
    // once it starts to bake the browning prints as a halftone.
    let pink_screen = if b < 0.12 { Screen::Solid } else { Screen::Halftone };
    dome(
        d,
        Ink::Pink,
        pink_screen,
        &l.body,
        focus,
        0.16,
        p_rim,
        p_core,
        steps,
        0.85,
        v.seed ^ 0x51,
        l.seam(),
    );

    // Depth: the key plate browns the unlit rim into golden-brown; bold and rye go deeper.
    let key_rim = 0.045 * raw + b * (0.08 + 0.42 * c + ke);
    let key_core = b * ((c - 0.6).max(0.0) * 0.3 + ke * 0.4);
    if key_rim > 0.06 {
        dome(
            d,
            Ink::Key,
            Screen::Halftone,
            &l.body,
            focus,
            0.6,
            key_rim,
            key_core,
            (steps * 2).div_ceil(3),
            1.2,
            v.seed ^ 0x99,
            l.seam(),
        );
    }

    // Raw dough has a soft satin sheen towards the light.
    if raw > 0.05 {
        for (i, (sx, tone)) in [(0.36f32, 0.14f32), (0.24, 0.2), (0.13, 0.26)].iter().enumerate() {
            let hl = lump(focus * 0.85, sx * l.rx, sx * 0.72 * l.ry, -0.6, 0.1, v.seed ^ i as u32, 48);
            d.knock_p(tone * raw, Screen::Solid, PY, &hl);
        }
    }

    // Blisters on a well-baked crust: tiny dark specks, more towards the lower-right.
    if b > 0.4 && c > 0.3 && l.lod > 0.2 {
        let mut rng = Rng::new(v.seed as u64 ^ 0xB115);
        let n = (70.0 * l.lod * (c - 0.25) * b) as usize;
        let mut specks = Vec::new();
        let s0 = (0.9 / l.k).max(0.005);
        for _ in 0..n {
            let a = rng.range(0.0, TAU);
            let r = rng.f32().sqrt() * 0.94;
            let p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
            if p.dot(LIGHT) > 0.1 && rng.chance(0.75) {
                continue;
            }
            let s = s0 * rng.range(0.9, 1.9);
            specks.push(speck(p, s, s * 0.8, rng.range(0.0, PI), 8));
        }
        fill_many(d, Paint::solid(Ink::Key, 0.4 + 0.3 * b), &specks);
    }
}

/// The crust browns hardest along every opening.
fn browning(d: &mut DrawList, l: &Loaf) {
    let (b, c) = (l.b, l.crust);
    let (pe, ke) = recipe_tint(l.v.recipe);
    let opened: Vec<&Bloom> = l.blooms.iter().filter(|bl| bl.open >= 0.03).collect();
    if !opened.is_empty() {
        let open = opened.iter().map(|bl| bl.open).sum::<f32>() / opened.len() as f32;
        let ridges: Vec<Vec<V2>> = opened.iter().map(|bl| bl.grown(0.02 + 0.05 * bl.open)).collect();
        let tone = ((0.42 + 0.5 * c + pe) * b * open.sqrt() + 0.04).min(0.96);
        fill_many(d, Paint::ht(Ink::Pink, tone).add(), &ridges);
        let scorch: Vec<Vec<V2>> = opened.iter().map(|bl| bl.grown((0.02 + 0.05 * bl.open) * 0.55)).collect();
        let kt = ((0.1 + 0.34 * c + ke) * b * open).min(0.6);
        fill_many(d, Paint::solid(Ink::Key, kt).add(), &scorch);
    }
}

/// Bran flecks, rye crackle, herb flecks and inclusions sitting in the crust.
fn character(d: &mut DrawList, l: &Loaf) {
    let v = l.v;
    let b = l.b;
    let mut rng = Rng::new(v.seed as u64 ^ 0xC4A2);
    let (rye, wheat) = (v.recipe == Recipe::DarkRye, v.recipe == Recipe::WholeWheat);

    if rye && l.lod > 0.15 {
        crackle(d, l);
    }

    // Bran: short dark flecks all over whole wheat and rye.
    if rye || wheat {
        let n = (20.0 + 70.0 * l.lod) as usize;
        let s0 = (1.3 / l.k).max(0.007);
        let mut flecks = Vec::new();
        for _ in 0..n {
            let a = rng.range(0.0, TAU);
            let r = rng.f32().sqrt() * 0.95;
            let p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
            let s = s0 * rng.range(0.8, 1.6);
            flecks.push(speck(p, s * 1.7, s * 0.75, rng.range(0.0, PI), 8));
        }
        fill_many(d, Paint::solid(Ink::Key, 0.5 + 0.25 * b), &flecks);
        fill_many(d, Paint::solid(Ink::Pink, 0.8).add(), &flecks);
    }

    // Herbs: green flecks (yellow over blue) for the olive loaf.
    if v.recipe == Recipe::Olive {
        let n = (14.0 + 30.0 * l.lod) as usize;
        let s0 = (1.6 / l.k).max(0.008);
        let mut herbs = Vec::new();
        for _ in 0..n {
            let a = rng.range(0.0, TAU);
            let r = rng.f32().sqrt() * 0.92;
            let p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
            let s = s0 * rng.range(0.9, 1.8);
            herbs.push(lump(p, s * 1.8, s, rng.range(0.0, PI), 0.25, rng.next_u32(), 10));
        }
        fill_many(d, Paint::solid(Ink::Blue, 0.9), &herbs);
        fill_many(d, Paint::solid(Ink::Yellow, 1.0), &herbs);
        fill_many(d, Paint::solid(Ink::Key, 0.25 + 0.2 * b), &herbs);
    }

    // Inclusions peeking through the crust (half-buried: a dough skin hugs each one).
    if let Some(inc) = v.recipe.inclusion() {
        let n = (5.0 + 5.0 * l.lod) as usize;
        let mut placed: Vec<V2> = Vec::new();
        let mut bits = Vec::new();
        let mut tries = 0;
        while bits.len() < n && tries < 200 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = 0.18 + rng.f32().sqrt() * 0.68;
            let p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
            if placed.iter().any(|q| q.dist(p) < 0.2) {
                continue;
            }
            let Some(p) = l.displaced(p, 0.03) else { continue };
            placed.push(p);
            bits.push((p, rng.range(0.042, 0.062), rng.range(0.0, PI), rng.next_u32()));
        }
        inclusion_bits(d, l, inc, &bits, true);
    }
}

/// A rye crackle: the floury skin splits along meandering, branching fissures.
fn crackle(d: &mut DrawList, l: &Loaf) {
    let v = l.v;
    let mut rng = Rng::new(v.seed as u64 ^ 0xC2AC);
    // Flour dusting first (soft-edged, heaviest on the crown): the fissures show darker
    // crust through it.
    for (i, k) in [1.0f32, 0.84, 0.68, 0.52].iter().enumerate() {
        let dust = lump(
            v2(-0.06 * l.rx, -0.08 * l.ry),
            0.7 * k * l.rx,
            0.64 * k * l.ry,
            0.3,
            0.1,
            v.seed ^ i as u32,
            64,
        );
        d.knock_p(0.16 * l.b, Screen::Solid, PY, &dust);
    }
    let dust = lump(v2(-0.06 * l.rx, -0.08 * l.ry), 0.78 * l.rx, 0.72 * l.ry, 0.3, 0.1, v.seed ^ 77, 64);
    d.knock_p(0.2 * l.b, Screen::Solid, PY, &dust);
    let mut cracks: Vec<Vec<V2>> = Vec::new();
    let n = (5.0 + 5.0 * l.lod) as usize;
    for _ in 0..n {
        let a = rng.range(0.0, TAU);
        let r = rng.f32().sqrt() * 0.55;
        let mut p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
        let mut dir = rng.range(0.0, TAU);
        let steps = rng.range(6.0, 13.0) as usize;
        let mut pts = vec![p];
        for _ in 0..steps {
            dir += rng.range(-0.6, 0.6);
            p += V2::from_angle(dir) * 0.04;
            if (p.x / l.rx).powi(2) + (p.y / l.ry).powi(2) > 0.72 {
                break;
            }
            pts.push(p);
            if rng.chance(0.12) {
                let bd = dir + rng.range(0.7, 1.3) * if rng.chance(0.5) { 1.0 } else { -1.0 };
                let mut q = p;
                let mut br = vec![q];
                for _ in 0..rng.range(2.0, 5.0) as usize {
                    q += V2::from_angle(bd + rng.range(-0.4, 0.4)) * 0.035;
                    br.push(q);
                }
                cracks.push(br);
            }
        }
        cracks.push(pts);
    }
    let w = l.lw(2.6);
    let mut polys = Vec::new();
    let mut lips = Vec::new();
    for c in cracks.iter().filter(|c| c.len() >= 2) {
        let c = chaikin(c, 2, false);
        polys.push(taper(&c, |t| w * (0.15 + 0.85 * arch(t, 0.7))));
        let nrm = normals(&c);
        let lip: Vec<V2> = c.iter().zip(&nrm).map(|(p, q)| *p + *q * (w * 1.1)).collect();
        lips.push(taper(&lip, |t| w * 0.45 * arch(t, 1.0)));
    }
    for poly in &polys {
        d.fill_p(Paint::solid(Ink::Key, 0.5 * l.b).add(), poly);
        d.fill_p(Paint::solid(Ink::Pink, 0.6 * l.b).add(), poly);
    }
    knock_many(d, 0.45 * l.b, Screen::Solid, PY, &lips);
}

/// Draw inclusion bits at (pos, size, angle, seed). `buried` bits get a dough skin.
fn inclusion_bits(d: &mut DrawList, l: &Loaf, inc: Inclusion, bits: &[(V2, f32, f32, u32)], buried: bool) {
    if bits.is_empty() {
        return;
    }
    let b = l.b;
    let mut shapes = Vec::new();
    let mut sockets = Vec::new();
    let mut shines = Vec::new();
    let mut skins = Vec::new();
    for &(p, s, a, sd) in bits {
        let shape = match inc {
            Inclusion::Olive => lump(p, s, s * 0.72, a, 0.06, sd, 28),
            Inclusion::Cranberry => lump(p, s * 0.8, s * 0.62, a, 0.2, sd, 24),
            Inclusion::Cheddar => lump(p, s * (1.0 + 0.3 * b), s * (0.7 + 0.25 * b), a, 0.3, sd, 30),
        };
        sockets.push(shape.iter().map(|q| p + (*q - p) * 1.28 + v2(0.004, 0.006)).collect::<Vec<V2>>());
        shines.push(speck(p + v2(-0.3, -0.34) * s, s * 0.26, s * 0.15, -0.6, 10));
        if buried {
            // A crescent of dough over the lower-right half of the bit.
            let dir = V2::from_angle(a + 0.8);
            let skin: Vec<V2> = shape
                .iter()
                .filter(|q| (**q - p).dot(dir) > -s * 0.1)
                .map(|q| *q + dir * (s * 0.12))
                .collect();
            if skin.len() >= 3 {
                skins.push(skin);
            }
        }
        shapes.push(shape);
    }
    // The crust puckers around each bit.
    fill_many(d, Paint::solid(Ink::Pink, (0.2 + 0.4 * l.crust) * b + 0.06).add(), &sockets);
    knock_many(d, 1.0, Screen::Solid, PLATES_ALL, &shapes);
    match inc {
        Inclusion::Olive => {
            fill_many(d, Paint::solid(Ink::Blue, 0.85).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Pink, 0.55).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Key, 0.5).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Yellow, 0.35).add(), &shapes);
        }
        Inclusion::Cranberry => {
            fill_many(d, Paint::solid(Ink::Pink, 1.0).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Blue, 0.22).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Key, 0.2 + 0.1 * b).add(), &shapes);
        }
        Inclusion::Cheddar => {
            fill_many(d, Paint::solid(Ink::Yellow, 1.0).add(), &shapes);
            fill_many(d, Paint::solid(Ink::Pink, 0.18 + 0.4 * b).add(), &shapes);
            // Crisp frico edges once baked.
            if b > 0.3 {
                for sh in &shapes {
                    d.stroke_p(Paint::solid(Ink::Key, 0.35 * b).add(), l.lw(2.4), sh, true);
                }
            }
        }
    }
    knock_many(d, 0.9, Screen::Solid, PLATES_COLOR, &shines);
    if !skins.is_empty() {
        knock_many(d, 1.0, Screen::Solid, PLATES_ALL, &skins);
        fill_many(d, Paint::solid(Ink::Yellow, 0.3 + 0.6 * b).add(), &skins);
        fill_many(d, Paint::solid(Ink::Pink, 0.03 + b * (0.08 + 0.36 * l.crust)).add(), &skins);
    }
    for sh in &shapes {
        d.stroke_p(Paint::solid(Ink::Key, 0.8).add(), l.lw(DETAIL * 0.8), sh, true);
    }
}

/// Banneton flour: a broken coil of flour ridges (spiral on a boule, loops on a bâtard).
fn banneton(d: &mut DrawList, l: &Loaf) {
    if l.lod < 0.08 {
        return;
    }
    let v = l.v;
    let b = l.b;
    let turns = match v.shape {
        Shape::Boule => 6.2,
        Shape::Batard => 5.4,
    };
    let u0 = 0.12;
    let total = TAU * turns;
    let mut rng = Rng::new(v.seed as u64 ^ 0xBA77);
    let mut theta = rng.range(0.0, 1.0);
    let base = rng.range(0.0, TAU);
    let mut buckets: [Vec<Vec<V2>>; 2] = [Vec::new(), Vec::new()];
    let wscale = l.lw(2.6).max(0.007);
    while theta < total {
        let dash = rng.range(0.5, 1.9) / (0.3 + theta / total);
        let gap = rng.range(0.12, 0.55) / (0.3 + theta / total);
        let t1 = (theta + dash).min(total);
        let mut pts = Vec::new();
        let steps = ((t1 - theta) * 8.0).ceil().max(2.0) as usize;
        for s in 0..=steps {
            let th = theta + (t1 - theta) * s as f32 / steps as f32;
            let u = u0 + (0.95 - u0) * th / total;
            let wob = 1.0 + 0.025 * vnoise(v2(th * 0.7, 3.1), v.seed);
            pts.push(sil_at(v.shape, th + base) * (u * wob));
        }
        theta = t1 + gap;
        if pts.len() < 2 {
            continue;
        }
        let w = wscale * rng.range(0.55, 1.25);
        let poly = taper(&pts, |t| w * (0.25 + 0.75 * arch(t, 0.6)));
        buckets[rng.below(2) as usize].push(poly);
    }
    for (i, polys) in buckets.iter().enumerate() {
        let tone = (0.64 + 0.2 * i as f32) * (1.0 - b) + (0.3 + 0.16 * i as f32) * b;
        knock_many(d, tone, Screen::Solid, PYK, polys);
    }
    // A light haze of flour between the ridges on raw dough.
    if b < 0.9 {
        let haze = lump(v2(-0.05, -0.08), 0.7 * l.rx, 0.66 * l.ry, 0.4, 0.14, v.seed ^ 0x4A2E, 64);
        d.knock_p(0.14 * (1.0 - b), Screen::Solid, PY, &haze);
    }
}

/// Flour dusted through a stencil: a bright core, a dotted fringe and loose grains.
fn stencil_flour(d: &mut DrawList, l: &Loaf, st: Stencil) {
    let v = l.v;
    let shapes = stencil_shape(st);
    let mut rng = Rng::new(v.seed as u64 ^ 0x57E4);
    let b = l.b;
    // The fringe: whole shape knocked with a dotted screen…
    for poly in &shapes {
        d.knock_p(0.72, Screen::Solid, PYK, poly);
    }
    // …and a slightly inset core knocked bright.
    for poly in &shapes {
        let c = poly.iter().fold(V2::ZERO, |a, p| a + *p) / poly.len() as f32;
        let inset: Vec<V2> = poly.iter().map(|p| c + (*p - c) * 0.95).collect();
        d.knock_p(0.97, Screen::Solid, PYK, &inset);
    }
    // Loose grains drifting over the edge.
    if l.lod > 0.15 {
        let mut grains = Vec::new();
        let mut placed: Vec<V2> = Vec::new();
        let gs = (0.9 / l.k).max(0.0035);
        for poly in &shapes {
            let per = polyline_len(poly) / 0.012 * (0.4 + 0.6 * l.lod);
            let ring = resample(
                &{
                    let mut p = poly.clone();
                    p.push(poly[0]);
                    p
                },
                per as usize + 3,
            );
            let nrm = normals(&ring);
            // Outward = away from the shape's centre.
            let c = poly.iter().fold(V2::ZERO, |a, p| a + *p) / poly.len() as f32;
            for (i, p) in ring.iter().enumerate() {
                let mut n = nrm[i];
                if (*p - c).dot(n) < 0.0 {
                    n = -n;
                }
                for _ in 0..2 {
                    let off = rng.range(-0.01, 0.045) * if rng.chance(0.7) { 1.0 } else { 0.5 };
                    let q = *p + n * off + n.perp() * rng.range(-0.008, 0.008);
                    if placed.iter().rev().take(24).any(|o| o.dist(q) < gs * 2.4) {
                        continue;
                    }
                    placed.push(q);
                    let s = gs * rng.range(0.6, 1.5);
                    grains.push(speck(q, s, s * 0.8, rng.range(0.0, PI), 7));
                }
            }
        }
        knock_many(d, 0.95, Screen::Solid, PYK, &grains);
    }
    // Flour toasts a touch at its fringe in the oven.
    if b > 0.2 {
        for poly in &shapes {
            d.stroke_p(Paint::solid(Ink::Yellow, 0.28 * b).add(), l.lw(3.0), poly, true);
        }
    }
}

/// Scored cuts: grooves in raw dough that tear open into blooms with a lifted ear.
fn cuts(d: &mut DrawList, l: &Loaf) {
    if l.blooms.is_empty() {
        return;
    }
    let (b, c) = (l.b, l.crust);
    // No clip masks here (they cost a full-size mask pass per cut): every cut's edges are
    // printed first, just outside its own opening, and then all the openings, so a cut
    // that crosses a neighbour's opening is naturally hidden by it.
    //
    // 1. The ears: flaps of crust lifted towards the light (one even-odd command for all
    //    cuts; where two flaps cross, the openings printed later cover the overlap).
    let eared: Vec<&Bloom> = l.blooms.iter().filter(|bl| bl.ear > 0.12 && l.lod > 0.15).collect();
    if !eared.is_empty() {
        let ear = eared.iter().map(|bl| bl.ear).sum::<f32>() / eared.len() as f32;
        let band = |bl: &Bloom, from: f32, to: f32| -> Vec<V2> {
            let n = bl.path.len();
            let fw = |i: usize| bl.width * bl.prof[i] * (0.22 + 0.5 * bl.ear);
            let mut p: Vec<V2> = (0..n).map(|i| bl.ear_edge[i] + bl.nrm[i] * (fw(i) * from)).collect();
            p.extend((0..n).rev().map(|i| bl.ear_edge[i] + bl.nrm[i] * (fw(i) * to)));
            p
        };
        // Brightest just behind the lip, easing back into the crust at the hinge.
        let outer: Vec<Vec<V2>> = eared.iter().map(|bl| band(bl, 0.08, 1.0)).collect();
        knock_many(d, 0.12 + 0.2 * ear, Screen::Solid, PK, &outer);
        let inner: Vec<Vec<V2>> = eared.iter().map(|bl| band(bl, 0.1, 0.5)).collect();
        knock_many(d, 0.15 + 0.35 * ear, Screen::Solid, PK, &inner);
    }
    // 2. Edges of the baked openings, just outside them: a fine torn far edge and the
    //    ear's crisp dark lip, which catches a glint of light along its crest.
    for bl in &l.blooms {
        if bl.open <= 0.05 {
            continue;
        }
        let o = bl.open.min(1.0);
        let n = bl.path.len();
        let w = l.lw(DETAIL) * (0.4 + 0.6 * o);
        let far: Vec<V2> = (0..n).map(|j| bl.far_edge[j] - bl.nrm[j] * (w * 0.35)).collect();
        {
            // Always drawn: on small loaves it is what makes a cut read as an opening.
            d.stroke_p(Paint::solid(Ink::Key, 0.85).add(), w, &far, false);
        }
        let lip_w = l.lw(INNER * (0.55 + 0.75 * bl.ear)) * (0.4 + 0.6 * o);
        let lip_at: Vec<V2> =
            (0..n).map(|j| bl.ear_edge[j] + bl.nrm[j] * (lip_w * 0.4 * bl.prof[j].sqrt())).collect();
        d.fill_p(Paint::solid(Ink::Key, 1.0).add(), &taper(&lip_at, |t| lip_w * 0.5 * arch(t, 0.5)));
        if bl.ear > 0.15 && l.lod > 0.3 {
            let crest: Vec<V2> =
                (0..n).map(|j| bl.ear_edge[j] + bl.nrm[j] * (lip_w * 1.05 * bl.prof[j].sqrt())).collect();
            let hw = lip_w * 0.28 * bl.ear;
            d.knock_p(0.85, Screen::Solid, PYK, &taper(&crest, |t| hw * arch(t, 1.2)));
        }
    }
    // 3. Openings: a pale, rough grigne (or wet dough in a fresh groove).
    for bl in &l.blooms {
        let o = bl.open.min(1.0);
        let raw = 1.0 - o;
        // Clear what the crust printed there, then print the grigne (see the clip rule).
        d.knock(&bl.lens);
        // Small loaves keep their openings paler so the cut still reads at icon size.
        let pink = (0.3 * raw + o * (0.06 + 0.26 * c * b)) * (0.55 + 0.45 * l.lod.min(1.0) * 2.0).min(1.0);
        let yellow = 0.75 * raw + (0.72 + 0.14 * b) * o;
        d.fill_p(Paint::solid(Ink::Yellow, yellow).add(), &bl.lens);
        d.fill_p(Paint::ht(Ink::Pink, pink).add(), &bl.lens);
        if raw > 0.05 {
            d.fill_p(Paint::solid(Ink::Key, 0.1 * raw).add(), &bl.lens);
        }
        if o > 0.12 && l.lod > 0.2 {
            // The torn far lip is crust, browned; the heart of the opening stays paler.
            d.fill_p(Paint::solid(Ink::Pink, (pink + 0.24 * o).min(0.7)).add(), &bl.strip(0.8, 1.02));
            d.fill_p(Paint::solid(Ink::Key, (0.08 + 0.24 * c) * b * o).add(), &bl.strip(0.9, 1.02));
            let heart = bl.strip(0.28, 0.72);
            let paler = (1.0 - (0.58 + 0.1 * b) / yellow.max(0.01)).clamp(0.0, 1.0);
            d.knock_p(paler, Screen::Solid, 1 << Ink::Yellow.idx(), &heart);
            d.knock_p(0.65, Screen::Solid, 1 << Ink::Pink.idx(), &heart);
            if l.lod > 0.3 && bl.len * l.k > 40.0 {
                fibres(d, l, bl);
            }
        }
        // The ear's shadow: a dark band tucked under the lip.
        if bl.ear > 0.04 {
            let e = bl.ear;
            if l.lod > 0.2 {
                d.fill_p(
                    Paint::solid(Ink::Pink, (0.28 + 0.16 * c).min(0.6)).add(),
                    &bl.strip(0.0, 0.12 + 0.22 * e),
                );
            }
            d.fill_p(Paint::solid(Ink::Key, 0.18 + 0.28 * e).add(), &bl.strip(0.0, 0.07 + 0.16 * e));
        }
    }
    // 4. Fresh grooves in raw dough: the blade's slit, a lit lip on the light side.
    for bl in &l.blooms {
        // The slit closes over as the cut blooms: gone once the opening shows.
        let raw = (1.0 - bl.open / 0.35).clamp(0.0, 1.0);
        if raw <= 0.02 {
            continue;
        }
        let n = bl.path.len();
        let slit_w = l.lw(DETAIL * 1.35);
        let slit = taper(&bl.path, |t| slit_w * arch(t, 0.45) * (0.4 + 0.6 * raw));
        d.fill_p(Paint::solid(Ink::Key, 0.9 * raw.sqrt()).add(), &slit);
        let lip: Vec<V2> = (0..n).map(|j| bl.ear_edge[j] + bl.nrm[j] * (slit_w * 0.45)).collect();
        d.knock_p(0.9 * raw, Screen::Solid, PYK, &taper(&lip, |t| slit_w * 0.5 * arch(t, 0.8)));
        let shade: Vec<V2> = (0..n).map(|j| bl.far_edge[j] - bl.nrm[j] * (slit_w * 0.4)).collect();
        let sh = taper(&shade, |t| slit_w * 0.55 * arch(t, 0.8));
        d.fill_p(Paint::solid(Ink::Key, 0.3 * raw).add(), &sh);
        d.fill_p(Paint::solid(Ink::Pink, 0.3 * raw).add(), &sh);
    }
}

/// Torn, stretched gluten across an opening: pale strands with darker gaps, and pores.
fn fibres(d: &mut DrawList, l: &Loaf, bl: &Bloom) {
    let n = bl.path.len();
    let mut rng = Rng::new((bl.len * 1000.0) as u64 ^ l.v.seed as u64);
    let count = ((bl.len * 26.0) * l.lod * bl.open) as usize;
    let w = l.lw(1.3);
    // Gluten stretched across the tear: pale strands leaning the same way, fanning a little,
    // with a soft shadow on their far side.
    let lean_dir = if hash01(l.v.seed, n as u32) < 0.5 { -1.0 } else { 1.0 };
    let mut light = Vec::new();
    let mut shade = Vec::new();
    for _ in 0..count {
        let i = (rng.range(0.12, 0.88) * (n - 1) as f32) as usize;
        let span = bl.we[i] + bl.wf[i];
        if span < 0.025 {
            continue;
        }
        let f0 = rng.range(0.18, 0.35);
        let f1 = rng.range(0.62, 0.86);
        let at = |f: f32| bl.path[i] + bl.nrm[i] * (bl.we[i] - span * f);
        let tan = (bl.path[(i + 1).min(n - 1)] - bl.path[i.saturating_sub(1)]).norm();
        let lean = tan * (span * lean_dir * rng.range(0.1, 0.3));
        let pts = vec![at(f0), at((f0 + f1) * 0.5) + lean * 0.4, at(f1) + lean];
        let tw = w * rng.range(0.7, 1.2);
        shade.push(taper(&pts.iter().map(|p| *p + tan * (tw * 1.1 * lean_dir)).collect::<Vec<V2>>(), |t| {
            tw * 0.6 * arch(t, 0.8)
        }));
        light.push(taper(&pts, |t| tw * (0.2 + 0.8 * arch(t, 0.8))));
    }
    fill_many(d, Paint::solid(Ink::Pink, 0.22).add(), &shade);
    knock_many(d, 0.5, Screen::Solid, PY, &light);
    // Tiny pores in the torn face.
    let mut pores = Vec::new();
    for _ in 0..(count / 2) {
        let i = (rng.range(0.15, 0.85) * (n - 1) as f32) as usize;
        let span = bl.we[i] + bl.wf[i];
        let p = bl.path[i] + bl.nrm[i] * (bl.we[i] - span * rng.range(0.3, 0.8));
        let s = (1.2 / l.k).max(0.004) * rng.range(0.8, 1.6);
        pores.push(speck(p, s * 1.4, s, rng.range(0.0, PI), 8));
    }
    fill_many(d, Paint::solid(Ink::Pink, 0.4).add(), &pores);
    fill_many(d, Paint::solid(Ink::Key, 0.2).add(), &pores);
}

/// Inclusions exposed inside the blooms.
fn inclusions_in_blooms(d: &mut DrawList, l: &Loaf, inc: Inclusion) {
    let mut bits = Vec::new();
    for (j, bl) in l.blooms.iter().enumerate() {
        if bl.open < 0.35 || bl.len < 0.3 {
            continue;
        }
        let n = bl.path.len();
        let count = ((bl.len * 2.2) as usize).clamp(1, 3);
        for m in 0..count {
            let h = hash01(l.v.seed ^ 0x1C, (j * 7 + m) as u32);
            let i = ((0.2 + 0.6 * h) * (n - 1) as f32) as usize;
            let span = bl.we[i] + bl.wf[i];
            let s = (span * 0.32).min(0.05);
            if s < 0.012 {
                continue;
            }
            let p = bl.path[i] - bl.nrm[i] * (bl.wf[i] * 0.35);
            bits.push((p, s, hash01(l.v.seed, (j * 5 + m) as u32) * PI, (j * 31 + m) as u32));
        }
    }
    inclusion_bits(d, l, inc, &bits, false);
}

/// Seeds and flakes: clustered, oriented along a gentle flow and pushed aside by blooms.
fn toppings(d: &mut DrawList, l: &Loaf, t: Topping) {
    let v = l.v;
    let b = l.b;
    let mut rng = Rng::new(v.seed as u64 ^ 0x70FF);
    // Seed size in loaf units: never smaller than a printable speck.
    let (len, target) = match t {
        Topping::Sesame => ((6.6 / l.k).max(0.05), 120.0),
        Topping::Oats => ((11.5 / l.k).max(0.082), 34.0),
        Topping::Poppy => ((3.5 / l.k).max(0.026), 230.0),
    };
    let base_len = match t {
        Topping::Sesame => 0.05,
        Topping::Oats => 0.082,
        Topping::Poppy => 0.026,
    };
    let n = (target * (base_len / len).powi(2) * (0.45 + 0.55 * l.lod)) as usize;
    // A few clumps where the seeds landed thickest.
    let clumps: Vec<(V2, f32)> = (0..4)
        .map(|_| {
            let a = rng.range(0.0, TAU);
            let r = rng.range(0.1, 0.6);
            (v2(a.cos() * l.rx * r, a.sin() * l.ry * r), rng.range(0.18, 0.32))
        })
        .collect();
    let min_d = len * 0.88;
    let mut pts: Vec<(V2, f32)> = Vec::new();
    let mut tries = 0;
    while pts.len() < n && tries < n * 30 {
        tries += 1;
        let a = rng.range(0.0, TAU);
        let r = rng.f32().sqrt() * 0.94;
        let p = v2(a.cos() * l.rx * r, a.sin() * l.ry * r);
        let dens = 0.35 + clumps.iter().map(|(c, s)| (-(p.dist(*c) / s).powi(2)).exp()).sum::<f32>();
        if !rng.chance(dens.min(1.0) * (1.0 - 0.35 * r * r)) {
            continue;
        }
        let Some(q) = l.displaced(p, len * 0.5) else { continue };
        if pts.iter().any(|(o, _)| o.dist(q) < min_d) {
            continue;
        }
        let flow = vnoise(q * 2.2, v.seed ^ 0xF10) * TAU * 1.2 + rng.range(-0.4, 0.4);
        pts.push((q, flow));
    }
    match t {
        Topping::Poppy => {
            let dots: Vec<Vec<V2>> =
                pts.iter().map(|(p, a)| speck(*p, len * 0.55, len * 0.45, *a, 10)).collect();
            let shadows: Vec<Vec<V2>> = dots.iter().map(|p| translate_poly(p, v2(0.3, 0.45) * len)).collect();
            fill_many(d, Paint::solid(Ink::Key, 0.3).add(), &shadows);
            fill_many(d, Paint::solid(Ink::Key, 0.95).add(), &dots);
            fill_many(d, Paint::solid(Ink::Blue, 0.7).add(), &dots);
            let glints: Vec<Vec<V2>> = pts
                .iter()
                .map(|(p, _)| speck(*p + v2(-0.15, -0.18) * len, len * 0.14, len * 0.14, 0.0, 6))
                .collect();
            knock_many(d, 0.7, Screen::Solid, PLATES_ALL, &glints);
        }
        Topping::Sesame | Topping::Oats => {
            let sesame = t == Topping::Sesame;
            let mut bodies = Vec::new();
            let mut shadows = Vec::new();
            let mut shines = Vec::new();
            for (p, a) in &pts {
                let (w, h) = if sesame { (len * 0.5, len * 0.3) } else { (len * 0.5, len * 0.36) };
                let body: Vec<V2> = if sesame {
                    // A teardrop: pointed at one end.
                    (0..16)
                        .map(|i| {
                            let u = TAU * i as f32 / 16.0;
                            let pinch = 1.0 - 0.35 * (u.cos() * 0.5 + 0.5).powi(3);
                            *p + v2(u.cos() * w, u.sin() * h * pinch).rotate(*a)
                        })
                        .collect()
                } else {
                    lump(*p, w, h, *a, 0.12, (p.x * 1e4 + 5e4) as u32 ^ (p.y * 1e4 + 5e4) as u32, 18)
                };
                shadows.push(body.iter().map(|q| *q + v2(0.35, 0.55) * (h * 0.7)).collect::<Vec<V2>>());
                shines.push(speck(*p + v2(-0.25, -0.35) * h, w * 0.4, h * 0.3, *a, 8));
                bodies.push(body);
            }
            fill_many(d, Paint::solid(Ink::Key, 0.42).add(), &shadows);
            knock_many(d, 1.0, Screen::Solid, PLATES_ALL, &bodies);
            if sesame {
                fill_many(d, Paint::solid(Ink::Yellow, 0.18 + 0.2 * b).add(), &bodies);
                fill_many(d, Paint::solid(Ink::Pink, 0.03 + 0.1 * b * (0.4 + l.crust)).add(), &bodies);
            } else {
                fill_many(d, Paint::solid(Ink::Yellow, 0.24 + 0.2 * b).add(), &bodies);
                fill_many(d, Paint::solid(Ink::Pink, 0.05 + 0.14 * b * (0.5 + l.crust)).add(), &bodies);
            }
            knock_many(d, 0.9, Screen::Solid, PY, &shines);
            let w = l.lw(1.1);
            let mut rings = Vec::with_capacity(bodies.len() * 2);
            for body in &bodies {
                let c = body.iter().fold(V2::ZERO, |a, q| a + *q) / body.len() as f32;
                let r = body.iter().map(|q| q.dist(c)).fold(0.0f32, f32::max).max(1e-4);
                rings.push(body.iter().map(|q| c + (*q - c) * (1.0 + w * 0.5 / r)).collect::<Vec<V2>>());
                rings.push(body.iter().map(|q| c + (*q - c) * (1.0 - w * 0.5 / r).max(0.2)).collect());
            }
            fill_many(d, Paint::solid(Ink::Key, 0.7).add(), &rings);
        }
    }
}

/// A thin cool reflected light along the shadow-side rim gives the dome its roundness.
fn rim_light(d: &mut DrawList, l: &Loaf) {
    if l.lod < 0.2 {
        return;
    }
    let n = l.body.len();
    let arc: Vec<V2> = (0..n)
        .filter_map(|i| {
            let p = l.body[i];
            let facing = -(p.norm().dot(LIGHT));
            (facing > 0.35).then_some(p * 0.965)
        })
        .collect();
    if arc.len() < 3 {
        return;
    }
    // `arc` may wrap around the start of the loop; order it from the lowest angle.
    let mut arc = arc;
    let start = arc
        .iter()
        .enumerate()
        .map(|(i, p)| (i, (arc[(i + arc.len() - 1) % arc.len()]).dist(*p)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    arc.rotate_left(start);
    let w = l.lw(3.4);
    let band = taper(&arc, |t| w * arch(t, 0.8));
    d.knock_p(0.16 + 0.08 * l.b, Screen::Solid, PY, &band);
}

// ---------------------------------------------------------------------------------------
// Crumb shot
// ---------------------------------------------------------------------------------------

/// Far intersection of the ray `c + u·t` with an ellipse (centre `e`, radii, rotation).
fn ray_ellipse(c: V2, u: V2, e: V2, rx: f32, ry: f32, rot: f32) -> Option<f32> {
    // Into the ellipse's frame, scaled to a unit circle.
    let to = |p: V2| {
        let q = p.rotate(-rot);
        v2(q.x / rx, q.y / ry)
    };
    let o = to(c - e);
    let du = to(u);
    let a = du.len_sq();
    let b = 2.0 * o.dot(du);
    let cc = o.len_sq() - 1.0;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 || a < 1e-12 {
        return None;
    }
    let t = (-b + disc.sqrt()) / (2.0 * a);
    (t > 0.0).then_some(t)
}

/// Outline of a union of overlapping ellipses, star-shaped around `c` (coalesced alveoli).
fn ellipse_union(c: V2, parts: &[(V2, f32, f32, f32)], wobble: f32, seed: u32, n: usize) -> Vec<V2> {
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let u = V2::from_angle(a);
            let t = parts
                .iter()
                .filter_map(|(e, rx, ry, rot)| ray_ellipse(c, u, *e, *rx, *ry, *rot))
                .fold(0.0f32, f32::max);
            let k = 1.0 + wobble * (vnoise(v2(a * 1.9, 0.5), seed) - 0.5);
            c + u * (t * k)
        })
        .collect()
}

/// Offset a closed outline inwards by `t(p)` along its normals.
fn inset(poly: &[V2], t: impl Fn(V2) -> f32) -> Vec<V2> {
    let n = poly.len();
    let area = crate::geom::signed_area(poly);
    (0..n)
        .map(|i| {
            let a = poly[(i + n - 1) % n];
            let b = poly[(i + 1) % n];
            let mut nrm = (b - a).norm().perp();
            // Clockwise in y-down (positive area): `perp` already points inwards.
            if area < 0.0 {
                nrm = -nrm;
            }
            poly[i] + nrm * t(poly[i])
        })
        .collect()
}

/// A cross-section crumb shot. `openness` 0..1 comes from starter pep.
///
/// Footprint: `w` wide, the dome rising to `-0.8·h` above the origin and the flat base
/// sitting at `+0.22·h` (the ear may lift a touch above the dome).
pub fn crumb_slice(d: &mut DrawList, w: f32, h: f32, openness: f32, crust: f32, seed: u32) {
    let open = openness.clamp(0.0, 1.0);
    let crust = crust.clamp(0.0, 1.0);
    let hw = w * 0.5;
    let base = h * 0.22;
    let height = h * 1.02;
    let lw = (w / 200.0).clamp(0.6, 1.8);
    let mut rng = Rng::new(seed as u64 * 977 + 13);

    // Profile: widest a little above the base, a full rounded crown, and tucked under
    // where it sat on the stone.
    let waist = base - h * 0.2;
    let crown = base - height;
    let dome_y = |x: f32| -> f32 {
        let u = (x / hw).clamp(-1.0, 1.0).abs();
        waist - (waist - crown) * (1.0 - u.powf(2.3)).max(0.0).powf(0.5)
    };
    // The score: a shallow valley (the grigne) with the ear rooted on its left.
    let ear_x = hw * (0.06 + 0.14 * hash01(seed, 1));
    let gap = w * (0.1 + 0.05 * open);
    let dip = h * (0.035 + 0.04 * open);
    let surface = |x: f32| -> f32 {
        let t = (x - ear_x) / gap;
        let valley = if (0.0..1.0).contains(&t) { arch(t, 0.8) * dip } else { 0.0 };
        dome_y(x) + valley
    };
    let n = 72;
    let top: Vec<V2> = (0..=n)
        .map(|i| {
            let x = -hw + w * i as f32 / n as f32;
            v2(x, surface(x))
        })
        .collect();
    let close = |top: &[V2]| -> Vec<V2> {
        let mut p = top.to_vec();
        let tuck =
            crate::geom::quad_bezier(v2(hw, waist), v2(hw * 1.01, base), v2(hw * 0.84, base + h * 0.01), 8);
        p.extend(tuck.iter().skip(1).cloned());
        p.extend(crate::geom::quad_bezier(
            v2(hw * 0.78, base + h * 0.013),
            v2(0.0, base + h * 0.03),
            v2(-hw * 0.78, base + h * 0.013),
            12,
        ));
        p.extend(tuck.iter().rev().take(8).map(|q| v2(-q.x, q.y)));
        chaikin(&p, 2, true)
    };
    // The plain dome (no ear) shapes the crumb.
    let plain = close(&top);

    // The ear: a flap of crust rising from the valley's left rim and curling over it,
    // folded into the silhouette so a single contour runs up its back and under its lip.
    let root_x = ear_x - gap * 0.55;
    let lift = h * (0.06 + 0.08 * open);
    let tip = v2(ear_x + gap * (0.62 + 0.18 * open), surface(ear_x) - lift);
    let back0 = v2(root_x, surface(root_x));
    let back = crate::geom::cubic_bezier(
        back0,
        back0 + v2(gap * 0.3, -lift * 0.35),
        tip + v2(-gap * 0.55, -lift * 0.35),
        tip,
        12,
    );
    let lip_r = h * 0.018;
    let under_end = v2(ear_x + gap * 0.12, surface(ear_x + gap * 0.12) - h * 0.004);
    let under = crate::geom::cubic_bezier(
        tip + v2(lip_r * 0.3, lip_r * 1.6),
        tip + v2(-gap * 0.2, lip_r * 3.0),
        under_end + v2(gap * 0.1, -lift * 0.35),
        under_end,
        10,
    );
    let mut with_ear: Vec<V2> = top.iter().filter(|p| p.x < root_x).cloned().collect();
    with_ear.extend(back.iter().cloned());
    with_ear.extend(under.iter().cloned());
    with_ear.extend(top.iter().filter(|p| p.x > under_end.x).cloned());
    let body = close(&with_ear);

    // Shadow, then paper under the whole silhouette.
    super::style::contact_shadow(d, v2(w * 0.02, base + h * 0.03), hw * 1.02, h * 0.075);
    d.backing(&body);

    // Crust: a caramelised shell, darkest on the crown and the ear.
    d.fill(Ink::Yellow, 0.95, &body);
    d.ht(Ink::Pink, (0.38 + 0.42 * crust).min(0.95), &body);
    d.ht(Ink::Key, (0.18 + 0.42 * crust).min(0.8), &body);
    let belly: Vec<V2> = under.iter().map(|p| *p + v2(0.0, -h * 0.008)).collect();
    let band = taper(&belly, |t| h * 0.012 * arch(t, 0.6));
    d.fill_p(Paint::ht(Ink::Pink, 0.45), &band);
    d.knock_p(1.0, Screen::Solid, 1 << Ink::Key.idx(), &band);

    // The crumb: inset from the crust, which is thickest on the crown.
    let thick = |p: V2| -> f32 {
        let up = ((base - p.y) / height).clamp(0.0, 1.0);
        let valley = if p.x > ear_x + gap * 0.15 && p.x < ear_x + gap * 1.1 { 0.6 } else { 1.0 };
        h * (0.042 + 0.04 * up) * valley
    };
    // A darker skin just under the outline: the crust is most caramelised outside.
    let skin = chaikin(&inset(&body, |_| h * 0.022), 1, true);
    let mut ring_eo = vec![body.clone()];
    ring_eo.push(skin);
    d.fill_eo(Paint::ht(Ink::Key, (0.3 + 0.4 * crust).min(0.85)).add(), &ring_eo);
    // The valley floor (the grigne) bakes paler and thinner.
    let valley: Vec<V2> =
        top.iter().filter(|p| p.x > under_end.x - gap * 0.02 && p.x < ear_x + gap * 1.02).cloned().collect();
    if valley.len() >= 2 {
        let band = taper(&valley, |t| h * 0.034 * arch(t, 0.35));
        d.clipped(&body, |d| {
            d.knock(&band);
            d.fill_p(Paint::solid(Ink::Yellow, 0.85).add(), &band);
            d.fill_p(Paint::ht(Ink::Pink, 0.28 + 0.2 * crust).add(), &band);
            d.fill_p(Paint::ht(Ink::Key, 0.08 + 0.12 * crust).add(), &band);
        });
    }
    let crumb = chaikin(&inset(&plain, thick), 1, true);
    d.clip_push(&body);
    d.knock(&crumb);
    // Denser, warmer crumb against the crust; pale and glossy in the heart (flat tints:
    // a screen would be too coarse for crumb this fine). The crumb runs under the ear's
    // notch, which the body clip cuts away, so it prints knock-then-add / knock-to-lighten.
    d.fill_p(Paint::solid(Ink::Yellow, 0.4).add(), &crumb);
    d.fill_p(Paint::solid(Ink::Pink, 0.07).add(), &crumb);
    let ring = chaikin(&inset(&crumb, |_| h * 0.03), 1, true);
    d.knock_p(1.0 - 0.31 / 0.4, Screen::Solid, 1 << Ink::Yellow.idx(), &ring);
    d.knock_p(0.5, Screen::Solid, 1 << Ink::Pink.idx(), &ring);
    let heart = chaikin(&inset(&ring, |_| h * 0.06), 1, true);
    d.knock_p(1.0 - 0.25 / 0.31, Screen::Solid, 1 << Ink::Yellow.idx(), &heart);
    d.knock_p(1.0, Screen::Solid, 1 << Ink::Pink.idx(), &heart);

    // Alveoli: coalesced, irregular, stretched along the rise; big in the upper heart,
    // tight against the crust and the base.
    let crumb_pts: Vec<V2> = crumb.iter().step_by(2).cloned().collect();
    let edge_dist = |p: V2| crumb_pts.iter().map(|q| q.dist(p)).fold(f32::INFINITY, f32::min);
    let big = w * (0.022 + 0.075 * open * open);
    let small = w * (0.006 + 0.003 * open);
    let mut placed: Vec<(V2, f32)> = Vec::new();
    let mut holes: Vec<(V2, Vec<V2>, f32)> = Vec::new();
    let target = (90.0 + 70.0 * (1.0 - open)) as usize;
    let mut tries = 0;
    while holes.len() < target && tries < 5000 {
        tries += 1;
        let p = v2(rng.range(-hw, hw), rng.range(base - height, base));
        if !point_in_poly(p, &crumb) {
            continue;
        }
        let edge = edge_dist(p);
        let heartk = (edge / (h * 0.3)).clamp(0.0, 1.0).powf(0.8);
        let upk = 1.0 - ((p.y - (base - height * 0.58)) / height).abs() * 1.3;
        let size = small + (big - small) * rng.f32().powf(2.6 - 1.2 * open) * heartk * upk.clamp(0.25, 1.0);
        if edge < size * 1.3 + h * 0.018 {
            continue;
        }
        if placed.iter().any(|(q, r)| q.dist(p) < (r + size) * 1.08 + w * 0.005) {
            continue;
        }
        placed.push((p, size));
        let rise = (p - v2(0.0, base + h * 0.5)).angle();
        let stretch = rng.range(1.1, 1.45 + 0.45 * open);
        // Bigger holes coalesce from two or three bubbles.
        let mut parts = vec![(p, size * stretch, size / stretch.sqrt(), rise)];
        if size > w * 0.02 {
            for _ in 0..(1 + rng.below(2)) {
                let dir = V2::from_angle(rise + rng.range(-1.2, 1.2));
                let r2 = size * rng.range(0.45, 0.75);
                parts.push((
                    p + dir * (size * rng.range(0.5, 0.9)),
                    r2 * 1.2,
                    r2,
                    rise + rng.range(-0.6, 0.6),
                ));
            }
        }
        let poly = ellipse_union(p, &parts, 0.16 + 0.14 * open, rng.next_u32(), 28);
        holes.push((p, poly, size));
    }
    // Each cavity: warm shade, a deep shadow under its upper wall, a glossy lit floor.
    let mut bodies = Vec::new();
    let mut shades = Vec::new();
    let mut floors = Vec::new();
    let mut specks = Vec::new();
    for (c, poly, size) in &holes {
        if *size < w * 0.009 {
            specks.push(poly.clone());
            continue;
        }
        let down = size * (0.32 + 0.1 * open);
        let lower: Vec<V2> =
            poly.iter().map(|q| *c + (*q - *c) * (1.0 - down / (size * 1.6)) + v2(0.0, down)).collect();
        shades.push(poly.clone());
        shades.push(lower);
        if *size > w * 0.016 {
            floors.push(speck(*c + v2(-size * 0.1, size * 0.55), size * 0.55, size * 0.17, 0.0, 14));
        }
        bodies.push(poly.clone());
    }
    knock_many(d, 1.0, Screen::Solid, PLATES_ALL, &bodies);
    fill_many(d, Paint::solid(Ink::Yellow, 0.62).add(), &bodies);
    fill_many(d, Paint::solid(Ink::Pink, 0.22 + 0.06 * crust).add(), &bodies);
    fill_many(d, Paint::solid(Ink::Key, 0.12).add(), &bodies);
    fill_many(d, Paint::solid(Ink::Key, 0.25).add(), &shades);
    fill_many(d, Paint::solid(Ink::Pink, 0.16).add(), &shades);
    knock_many(d, 0.5, Screen::Solid, PLATES_COLOR, &floors);
    knock_many(d, 1.0, Screen::Solid, PLATES_ALL, &specks);
    fill_many(d, Paint::solid(Ink::Pink, 0.3).add(), &specks);
    fill_many(d, Paint::solid(Ink::Key, 0.3).add(), &specks);
    fill_many(d, Paint::solid(Ink::Yellow, 0.6).add(), &specks);
    // Fine pores everywhere between the cavities.
    let mut pores = Vec::new();
    let pr = (w * 0.0035).max(0.7);
    for _ in 0..(160.0 * (w / 200.0).clamp(0.6, 2.2)) as usize {
        let p = v2(rng.range(-hw, hw), rng.range(base - height, base));
        if !point_in_poly(p, &crumb) || placed.iter().any(|(q, r)| q.dist(p) < r * 1.3 + pr) {
            continue;
        }
        pores.push(speck(p, pr * rng.range(0.8, 1.6), pr * rng.range(0.6, 1.0), rng.range(0.0, PI), 8));
    }
    fill_many(d, Paint::solid(Ink::Key, 0.22).add(), &pores);
    fill_many(d, Paint::solid(Ink::Pink, 0.2).add(), &pores);
    // A faint hairline on the biggest cavities' upper rims.
    for (c, poly, size) in &holes {
        if *size > w * 0.03 {
            let rim: Vec<V2> = poly.iter().filter(|q| q.y < c.y - size * 0.1).cloned().collect();
            if rim.len() >= 3 {
                d.stroke_p(Paint::solid(Ink::Key, 0.6).add(), DETAIL * 0.55 * lw, &rim, false);
            }
        }
    }

    // Flour on the base; a lit highlight along the crown's crust.
    let bottom = taper(
        &crate::geom::quad_bezier(
            v2(hw * 0.86, base + h * 0.01),
            v2(0.0, base + h * 0.03),
            v2(-hw * 0.86, base + h * 0.01),
            12,
        ),
        |t| h * 0.014 * arch(t, 0.3),
    );
    d.knock_p(0.6, Screen::Halftone, PLATES_COLOR, &bottom);
    let crown: Vec<V2> = top
        .iter()
        .filter(|p| p.x < ear_x - gap * 0.3 && p.x > -hw * 0.75)
        .map(|p| *p + v2(0.0, h * 0.012))
        .collect();
    if crown.len() >= 2 {
        let hl = taper(&crown, |t| h * 0.008 * arch(t, 0.7));
        d.knock_p(0.55, Screen::Halftone, PLATES_COLOR, &hl);
    }

    d.clip_pop();
    d.outline(Ink::Key, OUTER * lw * 0.85, &body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Pattern;
    use crate::draw::Shape as S;
    use crate::scoring::template;

    /// (commands, points): the rasteriser's workload.
    fn budget(d: &DrawList) -> (usize, usize) {
        let pts = d
            .cmds
            .iter()
            .map(|c| match &c.shape {
                S::Poly(p) => p.len(),
                S::PolysEo(ps) => ps.iter().map(Vec::len).sum(),
                S::Line { pts, .. } => pts.len(),
            })
            .sum();
        (d.cmds.len(), pts)
    }

    fn finite(d: &DrawList) -> bool {
        d.cmds.iter().all(|c| match &c.shape {
            S::Poly(p) => p.iter().all(|q| q.x.is_finite() && q.y.is_finite()),
            S::PolysEo(ps) => ps.iter().flatten().all(|q| q.x.is_finite() && q.y.is_finite()),
            S::Line { pts, width, .. } => {
                width.is_finite() && pts.iter().all(|q| q.x.is_finite() && q.y.is_finite())
            }
        })
    }

    /// Every animated state (bake 0→1 in the oven, bloom 0→1 in the reveal) at every size
    /// the screens use must stay finite (a NaN silently drops a whole shape) and cheap.
    #[test]
    fn loaves_are_finite_and_within_budget_in_every_state() {
        let mut worst = (0, 0);
        for (i, p) in Pattern::ALL.iter().enumerate() {
            for shape in [Shape::Boule, Shape::Batard] {
                for r in [27.0, 40.0, 62.0, 118.0, 165.0] {
                    for step in 0..=4 {
                        let t = step as f32 / 4.0;
                        let v = LoafView {
                            shape,
                            recipe: Recipe::ALL[(i + step) % Recipe::ALL.len()],
                            r,
                            bake: t,
                            spring: t,
                            crust: t,
                            cuts: template(*p, shape)
                                .into_iter()
                                .map(|pts| CutView { pts, bloom: t, ear: t })
                                .collect(),
                            stencil: Some(Stencil::ALL[(i + step) % Stencil::ALL.len()]),
                            topping: Some(Topping::ALL[(i + step) % Topping::ALL.len()]),
                            seed: i as u32 * 7 + step as u32,
                        };
                        let mut d = DrawList::new();
                        loaf_top(&mut d, &v);
                        assert!(finite(&d), "{p:?} {shape:?} r={r} t={t}");
                        let b = budget(&d);
                        worst = (worst.0.max(b.0), worst.1.max(b.1));
                    }
                }
            }
        }
        eprintln!("loaf worst budget (cmds, points): {worst:?}");
        assert!(worst.0 < 1500 && worst.1 < 120_000, "loaf budget {worst:?}");
        for open in [0.0, 0.5, 1.0] {
            let mut d = DrawList::new();
            crumb_slice(&mut d, 200.0, 130.0, open, open, 3);
            assert!(finite(&d));
            let b = budget(&d);
            assert!(b.0 < 600 && b.1 < 60_000, "crumb budget {b:?}");
        }
    }

    #[test]
    fn freehand_scribbles_stay_finite() {
        // Degenerate player input: a dot, a back-and-forth scrub and an out-of-bounds swipe.
        let cuts = vec![
            CutView { pts: vec![v2(0.1, 0.1), v2(0.1, 0.1)], bloom: 1.0, ear: 1.0 },
            CutView {
                pts: vec![v2(-0.3, 0.0), v2(0.3, 0.0), v2(-0.3, 0.01), v2(0.3, 0.02)],
                bloom: 1.0,
                ear: 1.0,
            },
            CutView { pts: vec![v2(-1.4, -1.2), v2(1.3, 1.1)], bloom: 1.0, ear: 1.0 },
        ];
        for bake in [0.0, 0.5, 1.0] {
            let mut d = DrawList::new();
            loaf_top(&mut d, &LoafView { cuts: cuts.clone(), bake, ..LoafView::default() });
            assert!(finite(&d));
        }
    }

    /// The rasteriser's "Over" fills zero out masked-off pixels in the same span, so an
    /// Over fill inside a clip must stay within it (see the module docs). Walk every bakery
    /// asset's clip stack and flag Over ink that overhangs the active clip.
    fn over_violations(d: &DrawList) -> Vec<String> {
        use crate::draw::Mode;
        use crate::geom::{dist_to_polyline, point_in_poly};
        let mut clips: Vec<Vec<V2>> = Vec::new();
        let mut bad = Vec::new();
        let inside = |p: V2, clip: &[V2], margin: f32| -> bool {
            let mut ring = clip.to_vec();
            ring.push(clip[0]);
            let dist = dist_to_polyline(p, &ring);
            if point_in_poly(p, clip) { dist >= margin - 0.75 } else { dist <= 0.75 && margin <= 0.0 }
        };
        for (i, c) in d.cmds.iter().enumerate() {
            match &c.op {
                Op::ClipPush => {
                    let poly = match &c.shape {
                        S::Poly(p) => p.clone(),
                        S::PolysEo(ps) => ps.first().cloned().unwrap_or_default(),
                        S::Line { pts, .. } => pts.clone(),
                    };
                    clips.push(poly);
                }
                Op::ClipPop => {
                    clips.pop();
                }
                Op::Ink(p) if p.mode == Mode::Over && !clips.is_empty() => {
                    let (pts, margin): (Vec<V2>, f32) = match &c.shape {
                        S::Poly(p) => (p.clone(), 0.0),
                        S::PolysEo(ps) => (ps.iter().flatten().cloned().collect(), 0.0),
                        S::Line { pts, width, .. } => (pts.clone(), width * 0.5),
                    };
                    for clip in clips.iter().filter(|c| c.len() >= 3) {
                        if let Some(q) = pts.iter().find(|q| !inside(**q, clip, margin)) {
                            bad.push(format!("#{i} {:?} at {q:?}", c.op));
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        bad
    }

    #[test]
    fn over_fills_stay_inside_their_clips() {
        use crate::art::{jar, oven, treats};
        use crate::content::Treat;
        let mut all = Vec::new();
        // Loaves in every state, including a freehand swipe that runs off the edge, drawn
        // standalone and inside Toasty's window (which clips them).
        let off_edge =
            CutView { pts: vec![v2(-1.3, 0.6), v2(0.0, 0.2), v2(1.3, -0.5)], bloom: 1.0, ear: 1.0 };
        for (i, p) in Pattern::ALL.iter().enumerate() {
            for t in [0.0, 0.5, 1.0] {
                let mut cuts: Vec<CutView> = template(*p, Shape::Boule)
                    .into_iter()
                    .map(|pts| CutView { pts, bloom: t, ear: t })
                    .collect();
                cuts.push(CutView { bloom: t, ear: t, ..off_edge.clone() });
                let v = LoafView {
                    recipe: Recipe::ALL[i % Recipe::ALL.len()],
                    bake: t,
                    spring: t,
                    crust: t,
                    cuts,
                    stencil: Some(Stencil::ALL[i % Stencil::ALL.len()]),
                    topping: Some(Topping::ALL[i % Topping::ALL.len()]),
                    seed: i as u32,
                    ..LoafView::default()
                };
                let mut d = DrawList::new();
                loaf_top(&mut d, &v);
                all.extend(over_violations(&d).into_iter().map(|e| format!("loaf {p:?} t={t}: {e}")));
                let mut d = DrawList::new();
                let ov = oven::OvenView { glow: t, open: 0.0, steam: t, ..Default::default() };
                oven::oven(&mut d, &ov, |d| {
                    d.with(crate::geom::Xf::at(v2(-46.0, -150.0)), |d| {
                        loaf_top(d, &LoafView { r: 40.0, ..v.clone() })
                    })
                });
                all.extend(over_violations(&d).into_iter().map(|e| format!("oven {p:?} t={t}: {e}")));
            }
        }
        for open in [0.3, 1.0] {
            let mut d = DrawList::new();
            oven::oven(&mut d, &oven::OvenView { open, glow: 0.5, ..Default::default() }, |_| {});
            all.extend(over_violations(&d).into_iter().map(|e| format!("open oven: {e}")));
        }
        let mut d = DrawList::new();
        crumb_slice(&mut d, 200.0, 130.0, 0.9, 0.6, 3);
        all.extend(over_violations(&d).into_iter().map(|e| format!("crumb: {e}")));
        for (i, (hooch, rise, band)) in
            [(false, 0.55, 0.35), (false, 1.0, 0.28), (true, 0.2, 0.3)].iter().enumerate()
        {
            let mut d = DrawList::new();
            let v = jar::JarView {
                hooch: *hooch,
                rise: *rise,
                band: *band,
                pep: 0.9,
                seed: i as u32,
                ..Default::default()
            };
            jar::jar(&mut d, &v);
            all.extend(over_violations(&d).into_iter().map(|e| format!("jar {i}: {e}")));
        }
        for t in Treat::ALL {
            for raw in [false, true] {
                let mut d = DrawList::new();
                if raw {
                    treats::treat_raw(&mut d, t, 130.0, 1);
                } else {
                    treats::treat(&mut d, t, 130.0, 1);
                }
                all.extend(over_violations(&d).into_iter().map(|e| format!("{t:?} raw={raw}: {e}")));
            }
        }
        assert!(all.is_empty(), "{} Over fills overhang their clip:\n{}", all.len(), all.join("\n"));
    }
}
