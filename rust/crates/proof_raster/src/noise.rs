//! Deterministic, tileable noise fields for the press (paper, ink film, screens).
//!
//! Everything here is generated once per device scale and cached, so it is written for
//! clarity first; the loops are still cheap (a few ms for a 512² field in release).

use std::sync::{Arc, Mutex};

/// A tiny keyed cache of shared, build-once values (keyed by a quantised scale or pitch).
pub struct Cache<T>(Mutex<Vec<(u32, Arc<T>)>>);

impl<T> Cache<T> {
    pub const fn new() -> Self {
        Cache(Mutex::new(Vec::new()))
    }

    pub fn get(&self, key: u32, make: impl FnOnce() -> T) -> Arc<T> {
        let mut c = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, v)) = c.iter().find(|(k, _)| *k == key) {
            return v.clone();
        }
        let v = Arc::new(make());
        c.push((key, v.clone()));
        v
    }
}

/// A good 32-bit integer hash (lowbias32).
#[inline]
pub fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// Hash of a lattice point and a seed.
#[inline]
pub fn hash3(x: i32, y: i32, seed: u32) -> u32 {
    hash_u32((x as u32) ^ hash_u32((y as u32) ^ hash_u32(seed.wrapping_add(0x9e37_79b9))))
}

/// Uniform [0, 1) from a lattice point and a seed.
#[inline]
pub fn rand01(x: i32, y: i32, seed: u32) -> f32 {
    (hash3(x, y, seed) >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// A square, tileable (wrap-around) field of values.
#[derive(Clone, Debug)]
pub struct Field {
    pub n: usize,
    pub v: Vec<f32>,
}

impl Field {
    pub fn zeros(n: usize) -> Field {
        Field { n, v: vec![0.0; n * n] }
    }

    /// Uniform white noise in [-0.5, 0.5).
    pub fn white(n: usize, seed: u32) -> Field {
        let mut f = Field::zeros(n);
        for y in 0..n {
            for x in 0..n {
                f.v[y * n + x] = rand01(x as i32, y as i32, seed) - 0.5;
            }
        }
        f
    }

    /// Separable gaussian blur with wrap-around (keeps the field tileable).
    /// `sx`/`sy` are the standard deviations along each axis, in texels.
    pub fn blur_xy(&self, sx: f32, sy: f32) -> Field {
        let n = self.n;
        let kernel = |s: f32| -> Vec<f32> {
            if s < 0.05 {
                return vec![1.0];
            }
            let r = ((s * 3.0).ceil() as usize).min(n / 2);
            let mut k: Vec<f32> =
                (0..=2 * r).map(|i| (-((i as f32 - r as f32).powi(2)) / (2.0 * s * s)).exp()).collect();
            let sum: f32 = k.iter().sum();
            k.iter_mut().for_each(|v| *v /= sum);
            k
        };
        // Convolve one wrapped line (padded copy, so the inner loop has no index maths).
        let line = |src: &mut dyn Iterator<Item = f32>, k: &[f32], pad: &mut Vec<f32>, out: &mut Vec<f32>| {
            let r = k.len() / 2;
            let row: Vec<f32> = src.collect();
            pad.clear();
            pad.extend_from_slice(&row[n - r..]);
            pad.extend_from_slice(&row);
            pad.extend_from_slice(&row[..r]);
            out.clear();
            for x in 0..n {
                let win = &pad[x..x + k.len()];
                out.push(win.iter().zip(k).map(|(a, b)| a * b).sum());
            }
        };
        let (kx, ky) = (kernel(sx), kernel(sy));
        let (mut pad, mut buf) = (Vec::new(), Vec::new());
        let mut tmp = Field::zeros(n);
        for y in 0..n {
            line(&mut self.v[y * n..(y + 1) * n].iter().copied(), &kx, &mut pad, &mut buf);
            tmp.v[y * n..(y + 1) * n].copy_from_slice(&buf);
        }
        let mut out = Field::zeros(n);
        for x in 0..n {
            line(&mut (0..n).map(|y| tmp.v[y * n + x]), &ky, &mut pad, &mut buf);
            for (y, v) in buf.iter().enumerate() {
                out.v[y * n + x] = *v;
            }
        }
        out
    }

    pub fn blur(&self, s: f32) -> Field {
        self.blur_xy(s, s)
    }

    pub fn mean_std(&self) -> (f32, f32) {
        let n = self.v.len() as f64;
        let mean = self.v.iter().map(|v| *v as f64).sum::<f64>() / n;
        let var = self.v.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>() / n;
        (mean as f32, var.sqrt() as f32)
    }

    /// Rescale to the given mean and standard deviation.
    pub fn normalized(mut self, mean: f32, std: f32) -> Field {
        let (m, s) = self.mean_std();
        let k = if s > 1e-9 { std / s } else { 0.0 };
        self.v.iter_mut().for_each(|v| *v = mean + (*v - m) * k);
        self
    }

    /// Replace every value by its rank, mapped to (0, 1): the result is exactly uniformly
    /// distributed while keeping the field's spatial structure (a threshold map).
    pub fn rank_equalized(&self) -> Field {
        let mut idx: Vec<u32> = (0..self.v.len() as u32).collect();
        idx.sort_by(|a, b| self.v[*a as usize].total_cmp(&self.v[*b as usize]));
        let total = self.v.len() as f32;
        let mut out = Field::zeros(self.n);
        for (rank, i) in idx.into_iter().enumerate() {
            out.v[i as usize] = (rank as f32 + 0.5) / total;
        }
        out
    }

    pub fn add_scaled(mut self, o: &Field, k: f32) -> Field {
        self.v.iter_mut().zip(&o.v).for_each(|(a, b)| *a += b * k);
        self
    }

    /// Gradient magnitude (central differences, wrapped), in value units per texel.
    pub fn gradient_len(&self) -> Field {
        let n = self.n;
        let mut out = Field::zeros(n);
        for y in 0..n {
            let (up, dn) = ((y + n - 1) % n, (y + 1) % n);
            for x in 0..n {
                let (l, r) = ((x + n - 1) % n, (x + 1) % n);
                let gx = (self.v[y * n + r] - self.v[y * n + l]) * 0.5;
                let gy = (self.v[dn * n + x] - self.v[up * n + x]) * 0.5;
                out.v[y * n + x] = (gx * gx + gy * gy).sqrt();
            }
        }
        out
    }

    /// Quantise to a byte channel (values clamped to 0..1).
    pub fn to_u8(&self) -> Vec<u8> {
        self.v.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect()
    }
}

/// Interleave four byte channels into RGBA8.
pub fn pack_rgba(r: &[u8], g: &[u8], b: &[u8], a: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(r.len() * 4);
    for i in 0..r.len() {
        out.extend_from_slice(&[r[i], g[i], b[i], a[i]]);
    }
    out
}

/// `floor` without a libm call (baseline x86-64 has no rounding instruction); |x| < 2³¹.
#[inline]
pub fn floor_i(x: f32) -> i32 {
    let i = x as i32;
    if (i as f32) > x { i - 1 } else { i }
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_equalized_is_uniform_and_tileable_blur_keeps_mean() {
        let f = Field::white(64, 3).blur(1.5);
        let (m, _) = f.mean_std();
        assert!(m.abs() < 0.02, "blur keeps the mean ~0: {m}");
        let r = f.rank_equalized();
        let below = r.v.iter().filter(|v| **v < 0.25).count() as f32 / r.v.len() as f32;
        assert!((below - 0.25).abs() < 0.001, "uniform ranks: {below}");
        // Wrap-around: the seam is as smooth as the interior.
        let seam = (f.v[10 * 64 + 63] - f.v[10 * 64]).abs();
        assert!(seam < 0.2, "tileable seam {seam}");
    }
}
