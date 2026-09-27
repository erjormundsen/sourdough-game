//! Screening: how a riso master turns a tone into ink.
//!
//! A riso drum prints 1-bit: every tint is made of ink dots. Screens here are threshold
//! maps `T(x, y)` — a pixel is inked when `tone > T` — evaluated with anti-aliased dot
//! edges so dots never look jaggy. Maps are calibrated so the inked fraction of a large area
//! equals the tone (art stays balanced).
//!
//! * **FM / grain touch** ([`Screen::Solid`] tints): a tileable band-passed noise map. Dots
//!   are small clusters of varying size, evenly spread (blue-noise-like), like Riso's
//!   "grain touch" mode or film grain.
//! * **AM halftone** ([`Screen::Halftone`], [`Screen::Coarse`]): round dots on a grid at the
//!   ink's screen angle. Dots vary slightly in size from cell to cell, the smallest dots drop
//!   out (the master cannot hold them) and the smallest holes fill in — stochastically, so
//!   the average tone is preserved.

use crate::noise::{Cache, Field, floor_i, hash3, rand01};
use proof_core::draw::Screen;
use std::sync::{Arc, OnceLock};

/// Tones at or above this print as solid ink (no screen).
pub const SOLID: f32 = 0.992;

/// Tileable FM threshold map at device resolution.
pub struct FmMap {
    pub n: usize,
    mask: i64,
    /// Lowest threshold inside each pixel's footprint (bilinear field) ...
    lo: Vec<f32>,
    /// ... and the inverse of its range: coverage ramps from `lo` to `lo + 1/inv`.
    inv: Vec<f32>,
    /// Tone → raw threshold so the mean coverage equals the tone.
    lut: Vec<f32>,
}

const LUT: usize = 1024;

impl FmMap {
    fn build(grain_px: f32) -> FmMap {
        FmMap::build_with(grain_px, 0.6, 0.45, 2.4, 0.85)
    }

    pub fn build_with(grain_px: f32, k1: f32, min1: f32, k2: f32, band_k: f32) -> FmMap {
        let n = 256usize;
        // Band-pass white noise: the low-pass sets the grain size, subtracting a wider blur
        // removes clumping so the tint stays even (a blue-noise-like "green noise").
        let s1 = (grain_px * k1).max(min1);
        let s2 = s1 * k2;
        let w = Field::white(n, 0x6a1f);
        let band = w.blur(s1).add_scaled(&w.blur(s2), -band_k);
        let t = band.rank_equalized();
        // Footprint range of the bilinear field over each pixel: its extremes lie on the
        // pixel's corners and edge midpoints (averages of neighbouring centres).
        let mut lo = vec![0.0f32; n * n];
        let mut inv = vec![0.0f32; n * n];
        let tv = &t.v;
        for y in 0..n {
            let rows = [(y + n - 1) % n, y, (y + 1) % n];
            for x in 0..n {
                let cols = [(x + n - 1) % n, x, (x + 1) % n];
                let at = |i: usize, j: usize| tv[rows[j] * n + cols[i]];
                let c = at(1, 1);
                let (mut mn, mut mx) = (c, c);
                for (sx, sy) in [(0, 0), (2, 0), (0, 2), (2, 2)] {
                    let (h, v, d) = (at(sx, 1), at(1, sy), at(sx, sy));
                    for q in [(c + h) * 0.5, (c + v) * 0.5, (c + h + v + d) * 0.25] {
                        mn = mn.min(q);
                        mx = mx.max(q);
                    }
                }
                lo[y * n + x] = mn;
                inv[y * n + x] = 1.0 / (mx - mn).max(1e-4);
            }
        }
        let mut m = FmMap { n, mask: n as i64 - 1, lo, inv, lut: Vec::new() };
        // Calibrate: mean coverage for raw thresholds on a grid (a quarter of the pixels is
        // plenty), then invert.
        let steps = 96;
        let sample: Vec<usize> =
            (0..n * n).filter(|j| (j / n).is_multiple_of(2) && j.is_multiple_of(2)).collect();
        let cal: Vec<f32> = (0..=steps)
            .map(|i| {
                let raw = i as f32 / steps as f32;
                let sum: f64 = sample.iter().map(|j| m.cov_raw(*j, raw) as f64).sum();
                (sum / sample.len() as f64) as f32
            })
            .collect();
        m.lut = (0..=LUT).map(|i| inverse(&cal, i as f32 / LUT as f32)).collect();
        m
    }

    #[inline]
    fn cov_raw(&self, i: usize, raw: f32) -> f32 {
        ((raw - self.lo[i]) * self.inv[i]).clamp(0.0, 1.0)
    }

    /// The raw threshold that makes the average coverage equal `tone`.
    #[inline]
    pub fn raw(&self, tone: f32) -> f32 {
        let x = tone.clamp(0.0, 1.0) * LUT as f32;
        let i = (x as usize).min(LUT - 1);
        let f = x - i as f32;
        self.lut[i] + (self.lut[i + 1] - self.lut[i]) * f
    }

    /// Coverage at absolute device pixel (x, y) for a raw threshold.
    #[inline]
    pub fn cov(&self, x: i64, y: i64, raw: f32) -> f32 {
        let i = ((y & self.mask) as usize) * self.n + (x & self.mask) as usize;
        self.cov_raw(i, raw)
    }

    /// Coverage of pixels `x0..x0 + out.len()` of row `y`.
    fn row(&self, x0: i64, y: i64, raw: f32, out: &mut [f32]) {
        let n = self.n;
        let base = ((y & self.mask) as usize) * n;
        let (lo, inv) = (&self.lo[base..base + n], &self.inv[base..base + n]);
        let mut x = (x0 & self.mask) as usize;
        for o in out.iter_mut() {
            *o = ((raw - lo[x]) * inv[x]).clamp(0.0, 1.0);
            x = (x + 1) & (n - 1);
        }
    }
}

/// Invert a monotonic table `cal[i] = f(i / (len - 1))`.
fn inverse(cal: &[f32], y: f32) -> f32 {
    let last = cal.len() - 1;
    if y <= cal[0] {
        return 0.0;
    }
    if y >= cal[last] {
        return 1.0;
    }
    let i = cal.partition_point(|v| *v < y).clamp(1, last);
    let (a, b) = (cal[i - 1], cal[i]);
    let f = if b - a > 1e-9 { (y - a) / (b - a) } else { 0.0 };
    (i as f32 - 1.0 + f) / last as f32
}

/// Shared FM map for a grain size (device pixels), cached.
pub fn fm_map(grain_px: f32) -> Arc<FmMap> {
    static CACHE: Cache<FmMap> = Cache::new();
    let key = (grain_px * 16.0).round().max(1.0) as u32;
    CACHE.get(key, || FmMap::build(key as f32 / 16.0))
}

/// One halftone cell's threshold table (cell = unit square), cached.
pub struct AmCell {
    n: usize,
    t: Vec<f32>,
    /// Inverse threshold gradient in cells per threshold unit.
    inv_g: Vec<f32>,
}

fn am_cell() -> &'static AmCell {
    static CELL: OnceLock<AmCell> = OnceLock::new();
    CELL.get_or_init(|| {
        let n = 64usize;
        // Cosine spot: round dots that grow into a checkerboard at 50%, then round holes.
        let mut spot = Field::zeros(n);
        for y in 0..n {
            for x in 0..n {
                let u = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let s = (std::f32::consts::PI * u).cos() + (std::f32::consts::PI * v).cos();
                // Tiny deterministic tie-break keeps ranks stable.
                spot.v[y * n + x] = -s + (y * n + x) as f32 * 1e-7;
            }
        }
        let t = spot.rank_equalized();
        let g = t.gradient_len();
        let inv_g = g.v.iter().map(|g| 1.0 / (g * n as f32).max(0.05)).collect();
        AmCell { n, t: t.v, inv_g }
    })
}

/// How far into highlights/shadows a screen stays regular before dots drop out / holes fill.
const DROP_HALFTONE: f32 = 0.075;
const DROP_COARSE: f32 = 0.04;
/// Per-cell dot size variation (tone units) where dots stand alone.
const JITTER: f32 = 0.045;

/// Pattern phase: the base screen, or a shifted copy so a layer of the same ink on a
/// different footing (an `Add` over another screen, a knockout) lands independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Base,
    Add,
    Knock,
}

/// A screen ready to evaluate for one command.
pub enum Screener {
    Solid,
    Fm { map: Arc<FmMap>, raw: f32, dx: i64, dy: i64 },
    Am(Am),
}

/// Anti-aliased coverage of one cell's dot at a raw threshold.
#[inline]
fn dot(cell: &AmCell, pitch: f32, fu: f32, fv: f32, raw: f32) -> f32 {
    let n = cell.n;
    let ix = ((fu * n as f32) as usize).min(n - 1);
    let iy = ((fv * n as f32) as usize).min(n - 1);
    let i = iy * n + ix;
    ((raw - cell.t[i]) * cell.inv_g[i] * pitch + 0.5).clamp(0.0, 1.0)
}

/// Tone calibration of AM dots at one pitch: the one-pixel anti-aliasing ramp over-covers
/// tiny dots (and tiny holes), so map the wanted tone to the raw dot threshold.
pub struct AmCal {
    lut: Vec<f32>,
}

impl AmCal {
    fn build(pitch: f32) -> AmCal {
        let cell = am_cell();
        // Pixel centres of a rotated screen land uniformly across the cell, so the mean
        // coverage of a large area is the mean over the cell table.
        let steps = 96;
        let cal: Vec<f32> = (0..=steps)
            .map(|k| {
                let raw = k as f32 / steps as f32;
                let sum: f64 = (0..cell.n * cell.n)
                    .map(|i| ((raw - cell.t[i]) * cell.inv_g[i] * pitch + 0.5).clamp(0.0, 1.0) as f64)
                    .sum();
                (sum / (cell.n * cell.n) as f64) as f32
            })
            .collect();
        AmCal { lut: (0..=LUT).map(|i| inverse(&cal, i as f32 / LUT as f32)).collect() }
    }

    #[inline]
    fn raw(&self, tone: f32) -> f32 {
        let x = tone.clamp(0.0, 1.0) * LUT as f32;
        let i = (x as usize).min(LUT - 1);
        self.lut[i] + (self.lut[i + 1] - self.lut[i]) * (x - i as f32)
    }
}

fn am_cal(pitch: f32) -> Arc<AmCal> {
    static CACHE: Cache<AmCal> = Cache::new();
    let key = (pitch * 8.0).round().max(8.0) as u32;
    CACHE.get(key, || AmCal::build(key as f32 / 8.0))
}

pub struct Am {
    /// (cos, sin) of the angle divided by the pitch in device px.
    c: f32,
    s: f32,
    pu: f32,
    pv: f32,
    pitch: f32,
    tone: f32,
    /// Calibrated thresholds for `tone`, the minimum dot and the minimum hole.
    raw: f32,
    raw_lo: f32,
    raw_hi: f32,
    t0: f32,
    salt: u32,
    cell: &'static AmCell,
    cal: Arc<AmCal>,
}

/// What one halftone cell prints.
#[derive(Clone, Copy)]
enum CellInk {
    Paper,
    Ink,
    /// A dot (or hole) at this raw threshold.
    Dot(f32),
}

impl Am {
    #[inline]
    fn cell_ink(&self, iu: i32, iv: i32, tone: f32, raw: f32) -> CellInk {
        let t0 = self.t0;
        let (t, mut r) = if tone < t0 {
            // Highlight dots too small for the master: keep a random subset at minimum size.
            if rand01(iu, iv, self.salt) * t0 >= tone {
                return CellInk::Paper;
            }
            (t0, self.raw_lo)
        } else if tone > 1.0 - t0 {
            if rand01(iu, iv, self.salt) * t0 >= 1.0 - tone {
                return CellInk::Ink;
            }
            (1.0 - t0, self.raw_hi)
        } else {
            (tone, raw)
        };
        // Dots that stand alone vary a little in size, like real ink on a real master.
        let alone = ((t - 0.5).abs() - 0.12) * (1.0 / 0.06);
        if alone > 0.0 {
            let j = (hash3(iu, iv, self.salt ^ 0x51f1) >> 8) as f32 * (1.0 / 16_777_216.0) - 0.5;
            r += j * 2.0 * JITTER * alone.min(1.0) * (t.min(1.0 - t) * 4.0).min(1.0);
        }
        CellInk::Dot(r)
    }

    #[inline]
    fn uv(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.c + y * self.s + self.pu, y * self.c - x * self.s + self.pv)
    }

    #[inline]
    fn cov(&self, x: f32, y: f32, tone: f32) -> f32 {
        let (u, v) = self.uv(x, y);
        let (iu, iv) = (floor_i(u), floor_i(v));
        match self.cell_ink(iu, iv, tone, self.cal.raw(tone)) {
            CellInk::Paper => 0.0,
            CellInk::Ink => 1.0,
            CellInk::Dot(r) => dot(self.cell, self.pitch, u - iu as f32, v - iv as f32, r),
        }
    }

    /// Coverage of pixels `x0..x0 + out.len()` on row `y` at this screen's tone.
    fn row(&self, x0: i64, y: i64, out: &mut [f32]) {
        let py = y as f32 + 0.5;
        let mut cell = (i32::MIN, i32::MIN, CellInk::Paper);
        for (j, o) in out.iter_mut().enumerate() {
            let (u, v) = self.uv((x0 + j as i64) as f32 + 0.5, py);
            let (iu, iv) = (floor_i(u), floor_i(v));
            if iu != cell.0 || iv != cell.1 {
                cell = (iu, iv, self.cell_ink(iu, iv, self.tone, self.raw));
            }
            *o = match cell.2 {
                CellInk::Paper => 0.0,
                CellInk::Ink => 1.0,
                CellInk::Dot(r) => dot(self.cell, self.pitch, u - iu as f32, v - iv as f32, r),
            };
        }
    }
}

/// Screen parameters shared by every command of one raster pass.
pub struct Screens {
    fm: Arc<FmMap>,
    /// Halftone pitch in device px.
    pitch: f32,
    angles: [f32; 4],
}

impl Screens {
    pub fn new(scale: f32, pitch_units: f32, grain_units: f32, angles: [f32; 4]) -> Screens {
        Screens { fm: fm_map((grain_units * scale).max(0.8)), pitch: pitch_units * scale, angles }
    }

    /// Prepare the screen for `tone` on plate `ink`.
    pub fn prepare(&self, ink: usize, screen: Screen, tone: f32, phase: Phase) -> Screener {
        let tone = tone.clamp(0.0, 1.0);
        if tone >= SOLID {
            return Screener::Solid;
        }
        let ph = match phase {
            Phase::Base => 0,
            Phase::Add => 1,
            Phase::Knock => 2,
        };
        match screen {
            Screen::Solid => {
                let n = self.fm.n as i64;
                // Each drum (and each phase) reads a different part of the map.
                let dx = ink as i64 * 101 + ph * (n / 2 + 37);
                let dy = ink as i64 * 59 + ph * (n / 3 + 71);
                Screener::Fm { raw: self.fm.raw(tone), map: self.fm.clone(), dx, dy }
            }
            Screen::Halftone | Screen::Coarse => {
                let coarse = screen == Screen::Coarse;
                let pitch = self.pitch * if coarse { 2.0 } else { 1.0 };
                let a = self.angles[ink];
                let (pu, pv) = match phase {
                    Phase::Base => (0.0, 0.0),
                    Phase::Add => (0.5, 0.5),
                    Phase::Knock => (0.5, 0.0),
                };
                let cal = am_cal(pitch);
                let t0 = if coarse { DROP_COARSE } else { DROP_HALFTONE };
                Screener::Am(Am {
                    c: a.cos() / pitch,
                    s: a.sin() / pitch,
                    pu,
                    pv,
                    pitch,
                    tone,
                    raw: cal.raw(tone),
                    raw_lo: cal.raw(t0),
                    raw_hi: cal.raw(1.0 - t0),
                    t0,
                    salt: 0x1234_5678 ^ ((ink as u32) << 8) ^ ph as u32 ^ ((coarse as u32) << 12),
                    cell: am_cell(),
                    cal,
                })
            }
        }
    }
}

impl Screener {
    /// Ink coverage at absolute device pixel (x, y) (pixel centre at +0.5).
    #[inline]
    pub fn cov(&self, x: i64, y: i64) -> f32 {
        match self {
            Screener::Solid => 1.0,
            Screener::Fm { map, raw, dx, dy } => map.cov(x + dx, y + dy, *raw),
            Screener::Am(am) => am.cov(x as f32 + 0.5, y as f32 + 0.5, am.tone),
        }
    }

    /// Coverage of the pixels `x0..x0 + out.len()` of row `y` (much faster than `cov`).
    pub fn row(&self, x0: i64, y: i64, out: &mut [f32]) {
        match self {
            Screener::Solid => out.fill(1.0),
            Screener::Fm { map, raw, dx, dy } => map.row(x0 + dx, y + dy, *raw, out),
            Screener::Am(am) => am.row(x0, y, out),
        }
    }

    /// Coverage at (x, y) for a different tone on the same screen and phase (used to
    /// re-screen a flattened union in place).
    #[inline]
    pub fn cov_tone(&self, x: i64, y: i64, tone: f32) -> f32 {
        if tone >= SOLID {
            return 1.0;
        }
        match self {
            Screener::Solid => 1.0,
            Screener::Fm { map, dx, dy, .. } => map.cov(x + dx, y + dy, map.raw(tone)),
            Screener::Am(am) => am.cov(x as f32 + 0.5, y as f32 + 0.5, tone),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean(s: &Screener, n: i64) -> f32 {
        let mut sum = 0.0f64;
        for y in 0..n {
            for x in 0..n {
                sum += s.cov(x, y) as f64;
            }
        }
        (sum / (n * n) as f64) as f32
    }

    #[test]
    fn every_screen_averages_to_its_tone() {
        for scale in [1.0, 1.5, 2.0] {
            let scr = Screens::new(scale, 4.6, 1.3, [0.26, 0.0, 1.31, 0.785]);
            for screen in [Screen::Solid, Screen::Halftone, Screen::Coarse] {
                for tone in [0.02, 0.05, 0.1, 0.2, 0.35, 0.5, 0.65, 0.8, 0.9, 0.97] {
                    for ink in 0..4 {
                        let s = scr.prepare(ink, screen, tone, Phase::Base);
                        let m = mean(&s, 600);
                        let tol = if !(0.06..=0.94).contains(&tone) { 0.012 } else { 0.02 };
                        assert!(
                            (m - tone).abs() < tol,
                            "{screen:?} ink {ink} scale {scale} tone {tone}: mean {m}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fm_grain_is_crisp_not_hazy() {
        // At a light tint most pixels are bare paper and most ink sits in dots (pixels at
        // least half covered), not in a faint haze of barely-inked pixels.
        let scr = Screens::new(1.5, 4.6, 1.3, [0.0; 4]);
        let s = scr.prepare(0, Screen::Solid, 0.15, Phase::Base);
        let (mut paper, mut dense, mut total) = (0, 0.0f32, 0.0f32);
        for y in 0..300 {
            for x in 0..300 {
                let v = s.cov(x, y);
                if v <= 0.0 {
                    paper += 1;
                }
                if v >= 0.5 {
                    dense += v;
                }
                total += v;
            }
        }
        assert!(paper > 90_000 * 60 / 100, "paper {paper}");
        assert!(dense > total * 0.6, "ink in dots {dense} of {total}");
    }

    #[test]
    fn shifted_phase_is_independent() {
        let scr = Screens::new(1.5, 4.6, 1.3, [0.26, 0.0, 1.31, 0.785]);
        for screen in [Screen::Solid, Screen::Halftone] {
            let a = scr.prepare(2, screen, 0.3, Phase::Base);
            let b = scr.prepare(2, screen, 0.3, Phase::Knock);
            let mut sum = 0.0f64;
            for y in 0..400 {
                for x in 0..400 {
                    let (ca, cb) = (a.cov(x, y), b.cov(x, y));
                    sum += (ca + cb * (1.0 - ca)) as f64;
                }
            }
            let union = (sum / 160_000.0) as f32;
            // Unrelated layers union to well above either (nested ones would stay 0.3).
            assert!(union > 0.44, "{screen:?} union {union}");
        }
    }
}
