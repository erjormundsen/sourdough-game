//! The riso press: how rasterised plates become ink on paper.
//!
//! This is the CPU twin of `godot/shaders/riso_press.gdshaderinc` (used by `riso_ink`,
//! `riso_text`, `riso_backing` and `paper`). Both read the same generated textures and the
//! same [`PressStyle`] numbers (sent to the shaders as uniforms by `proof_gd::riso`), so art
//! previewed on the artboard prints like the game. **Change one, change the other.**
//!
//! The model, per screen pixel `p` (device px, top-left origin):
//! 1. *Drums.* Each ink drum is misregistered by its own affine offset about the page centre
//!    (translation + a hair of rotation/stretch), identical for every sprite on the page.
//!    The print-in kick slams the drums further out of register. Key stays nearly true.
//! 2. *Ink spread.* A small smooth warp and a per-pixel ragged threshold make edges and
//!    screen dots organic; partial coverage swells a touch (dot gain).
//! 3. *Ink film.* Solid ink is never flat: low-frequency mottling per drum, "velvet" where
//!    the paper's tooth peaks skip ink, pinholes, faint streaks along the paper feed,
//!    starvation toward one side, and a later ink trapping unevenly over an earlier one.
//! 4. *Paper.* Warm uncoated stock with tooth, fibres and cloudy formation.
//! 5. *Overprint.* Inks multiply over paper and each other.

use crate::noise::{Cache, Field, pack_rgba, rand01, smoothstep};
use proof_core::geom::{V2, v2};
use proof_core::ink::Rgb;
use std::sync::Arc;

/// One RGBA8 texture.
#[derive(Clone, Debug)]
pub struct Tex {
    pub size: u32,
    pub rgba: Vec<u8>,
}

impl Tex {
    /// Channel values at integer texel (wrapped; sizes are powers of two), 0..1.
    #[inline]
    pub fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let n = self.size as i64;
        let i = (((y & (n - 1)) * n + (x & (n - 1))) * 4) as usize;
        let p = &self.rgba[i..i + 4];
        const K: f32 = 1.0 / 255.0;
        [p[0] as f32 * K, p[1] as f32 * K, p[2] as f32 * K, p[3] as f32 * K]
    }

    /// Bilinear, wrapped sample at texel coordinates (GL convention: centres at +0.5).
    pub fn bilinear(&self, x: f32, y: f32) -> [f32; 4] {
        let (x, y) = (x - 0.5, y - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (ix, iy) = (x0 as i64, y0 as i64);
        let (a, b, c, d) =
            (self.texel(ix, iy), self.texel(ix + 1, iy), self.texel(ix, iy + 1), self.texel(ix + 1, iy + 1));
        let mut out = [0.0; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bot = c[k] + (d[k] - c[k]) * fx;
            out[k] = top + (bot - top) * fy;
        }
        out
    }
}

/// Paper and ink-film textures for one device scale.
///
/// * `fine` (1 texel = 1 px, nearest): R paper tooth, G paper fibres (0.5 = none),
///   B ragged-edge noise, A pinholes.
/// * `mid` (1 texel = 1 px, linear): RG ink-spread warp, B ink-film noise (trapping),
///   A feed streaks (constant down the page).
/// * `coarse` (1 texel = [`COARSE_UNITS`] reference units, linear): RGB mottling of the
///   pink/yellow/blue drums, A key mottling and paper formation.
#[derive(Debug)]
pub struct PressTextures {
    /// Device px per reference unit these were made for.
    pub scale: f32,
    pub fine: Tex,
    pub mid: Tex,
    pub coarse: Tex,
}

/// Reference units per `coarse` texel.
pub const COARSE_UNITS: f32 = 3.0;

impl PressTextures {
    /// Pixels spanned by one tile of the coarse texture at this scale.
    pub fn coarse_px(&self) -> f32 {
        self.coarse.size as f32 * COARSE_UNITS * self.scale
    }

    fn build(scale: f32) -> PressTextures {
        let s = scale;
        // --- fine: 1 px texels ------------------------------------------------------------
        let n = 512usize;
        // Tooth: the paper surface, two octaves (fibre-scale bumps and a softer undulation).
        let tooth = Field::white(n, 11)
            .blur(0.3 * s)
            .normalized(0.0, 1.0)
            .add_scaled(&Field::white(n, 12).blur(0.9 * s).normalized(0.0, 1.0), 0.8)
            .normalized(0.5, 0.16);
        // Fibres: short curly strands, some lighter, some darker than the sheet.
        let fibres = fibre_field(n, s);
        // Ragged-edge noise.
        let edge = Field::white(n, 21).blur(0.33 * s).normalized(0.5, 0.2);
        // Pinholes: sparse specks where ink never lands.
        let pins = pinhole_field(n, s);
        let fine = Tex {
            size: n as u32,
            rgba: pack_rgba(&tooth.to_u8(), &fibres.to_u8(), &edge.to_u8(), &pins.to_u8()),
        };

        // --- mid: 1 px texels -------------------------------------------------------------
        let n = 256usize;
        let wx = Field::white(n, 31).blur(1.1 * s).normalized(0.5, 0.17);
        let wy = Field::white(n, 32).blur(1.1 * s).normalized(0.5, 0.17);
        let film = Field::white(n, 33).blur(1.8 * s).normalized(0.5, 0.18);
        let streak = streak_field(n, s);
        let mid =
            Tex { size: n as u32, rgba: pack_rgba(&wx.to_u8(), &wy.to_u8(), &film.to_u8(), &streak.to_u8()) };

        // --- coarse: COARSE_UNITS per texel ------------------------------------------------
        let n = 256usize;
        let mottle = |seed: u32| {
            Field::white(n, seed)
                .blur(2.2)
                .normalized(0.0, 1.0)
                .add_scaled(&Field::white(n, seed + 1).blur(6.0).normalized(0.0, 1.0), 1.1)
                .normalized(0.5, 0.19)
        };
        let key = Field::white(n, 47)
            .blur(1.4)
            .normalized(0.0, 1.0)
            .add_scaled(&Field::white(n, 48).blur(4.5).normalized(0.0, 1.0), 1.0)
            .normalized(0.5, 0.19);
        let coarse = Tex {
            size: n as u32,
            rgba: pack_rgba(&mottle(41).to_u8(), &mottle(43).to_u8(), &mottle(45).to_u8(), &key.to_u8()),
        };
        PressTextures { scale, fine, mid, coarse }
    }
}

/// Splat soft, curved fibres into a neutral (0.5) field.
fn fibre_field(n: usize, s: f32) -> Field {
    let mut f = Field::zeros(n);
    f.v.iter_mut().for_each(|v| *v = 0.5);
    let area_units = (n as f32 / s).powi(2);
    let count = (area_units * 0.0045) as u32;
    let nn = n as i64;
    for i in 0..count {
        let r = |k: u32| rand01(i as i32, k as i32, 0xf1b3);
        let (x0, y0) = (r(0) * n as f32, r(1) * n as f32);
        let len = (4.0 + 16.0 * r(2).powi(2)) * s;
        let mut ang = r(3) * std::f32::consts::TAU;
        // Gently bent strands (at most ~70° over the whole fibre), not curls.
        let steps_f = (len / 0.5).max(2.0);
        let curl = (r(4) - 0.5) * 2.4 / steps_f;
        let width = (0.3 + 0.4 * r(5)) * s;
        // Mostly light (bleached cellulose catching light), some darker.
        let amp = if r(6) < 0.62 { 0.18 + 0.2 * r(7) } else { -(0.12 + 0.16 * r(7)) };
        let steps = (len / 0.5).max(2.0) as u32;
        let (mut x, mut y) = (x0, y0);
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            // Fibres taper at both ends.
            let taper = (t * (1.0 - t) * 4.0).sqrt();
            let a = amp * taper;
            let rad = width.max(0.5);
            let ri = rad.ceil() as i64 + 1;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let px = x.floor() as i64 + dx;
                    let py = y.floor() as i64 + dy;
                    let d2 = ((px as f32 + 0.5 - x).powi(2) + (py as f32 + 0.5 - y).powi(2)) / (rad * rad);
                    if d2 < 1.0 {
                        let w = (1.0 - d2) * a;
                        let idx = ((py & (nn - 1)) * nn + (px & (nn - 1))) as usize;
                        let cur = f.v[idx] - 0.5;
                        // Keep the strongest mark so crossings don't pile up.
                        if w.abs() > cur.abs() {
                            f.v[idx] = 0.5 + w;
                        }
                    }
                }
            }
            ang += curl;
            x += ang.cos() * 0.5;
            y += ang.sin() * 0.5;
        }
    }
    f
}

/// Sparse soft specks (0 = none, 1 = no ink at all).
fn pinhole_field(n: usize, s: f32) -> Field {
    let mut f = Field::zeros(n);
    let area_units = (n as f32 / s).powi(2);
    let count = (area_units * 0.010) as u32;
    let nn = n as i64;
    for i in 0..count {
        let r = |k: u32| rand01(i as i32, k as i32, 0x9a11);
        let (x, y) = (r(0) * n as f32, r(1) * n as f32);
        let rad = (0.28 + 0.6 * r(2).powi(3)) * s;
        let amp = 0.45 + 0.55 * r(3);
        let ri = rad.ceil() as i64 + 1;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let px = x.floor() as i64 + dx;
                let py = y.floor() as i64 + dy;
                let d = ((px as f32 + 0.5 - x).powi(2) + (py as f32 + 0.5 - y).powi(2)).sqrt();
                let w = (1.0 - smoothstep(rad * 0.6, rad + 0.6, d)) * amp;
                let idx = ((py & (nn - 1)) * nn + (px & (nn - 1))) as usize;
                f.v[idx] = f.v[idx].max(w);
            }
        }
    }
    f
}

/// Faint lines along the paper feed: a function of x only (constant down the tile).
fn streak_field(n: usize, s: f32) -> Field {
    let mut row = vec![0.0f32; n];
    let count = ((n as f32 / s) / 34.0).ceil() as u32;
    for i in 0..count {
        let r = |k: u32| rand01(i as i32, k as i32, 0x57e4);
        let x = r(0) * n as f32;
        let w = (0.5 + 1.8 * r(1)) * s;
        let amp = 0.35 + 0.65 * r(2);
        for (px, v) in row.iter_mut().enumerate() {
            // Wrapped distance so the tile stays seamless.
            let mut d = (px as f32 + 0.5 - x).abs();
            d = d.min(n as f32 - d);
            *v = (*v + amp * (-(d * d) / (2.0 * w * w)).exp()).min(1.0);
        }
    }
    let mut f = Field::zeros(n);
    for y in 0..n {
        f.v[y * n..(y + 1) * n].copy_from_slice(&row);
    }
    f
}

/// Press textures for a device scale (cached; generated once per scale).
pub fn press_textures(scale: f32) -> Arc<PressTextures> {
    static CACHE: Cache<PressTextures> = Cache::new();
    let key = (scale * 20.0).round().max(1.0) as u32;
    CACHE.get(key, || PressTextures::build(key as f32 / 20.0))
}

/// One ink drum's registration on the page beyond its resting translation
/// ([`PressStyle::offsets`]); reference units / radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drum {
    /// Rotation about the page centre (radians, clockwise on screen).
    pub rot: f32,
    /// Fractional stretch per axis (paper grows/shrinks between passes).
    pub stretch: V2,
    /// Extra shift when the page slams into the press (kick = 1).
    pub kick_shift: V2,
    pub kick_rot: f32,
}

impl Drum {
    /// Displacement (reference units) at `d` from the page centre for a resting translation
    /// and a kick: `shift + A·d` with `A = R(rot)·S(1 + stretch) − I`.
    pub fn affine(&self, offset: V2, kick: f32) -> (V2, [f32; 4]) {
        let shift = offset + self.kick_shift * kick;
        let rot = self.rot + self.kick_rot * kick;
        let (c, s) = (rot.cos(), rot.sin());
        let (sx, sy) = (1.0 + self.stretch.x, 1.0 + self.stretch.y);
        // Row-major [a11, a12, a21, a22] of R·S − I.
        (shift, [c * sx - 1.0, -s * sy, s * sx, c * sy - 1.0])
    }
}

/// Everything about how the press prints (the uniforms of the riso shaders).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressStyle {
    /// Resting misregistration of each drum (pink, yellow, blue, key), reference units.
    pub offsets: [V2; 4],
    /// Rotation, stretch and print-in kick of each drum.
    pub drums: [Drum; 4],
    /// Print-in kick (0 = at rest).
    pub kick: f32,
    /// Ink-spread warp amplitude (reference units).
    pub warp: f32,
    /// Edge swelling on partial coverage (dot gain) and ragged-edge strength.
    pub spread: f32,
    pub rough: f32,
    /// Per ink (pink, yellow, blue, key): low-frequency mottling depth.
    pub mottle: [f32; 4],
    /// Ink skipping the paper's tooth peaks.
    pub velvet: [f32; 4],
    /// Pinhole strength.
    pub pinholes: [f32; 4],
    /// Feed-direction streak strength.
    pub streaks: [f32; 4],
    /// Ink starvation across the page (x and y slopes).
    pub starve_x: [f32; 4],
    pub starve_y: [f32; 4],
    /// Uneven trapping of an ink printed over earlier inks.
    pub trap: [f32; 4],
    /// Paper: tooth shading, fibre contrast, formation cloudiness.
    pub tooth: f32,
    pub fibres: f32,
    pub formation: f32,
}

impl Default for PressStyle {
    fn default() -> Self {
        let drum = |rot: f32, tx: f32, ty: f32, kx: f32, ky: f32, kr: f32| Drum {
            rot,
            stretch: v2(tx, ty),
            kick_shift: v2(kx, ky),
            kick_rot: kr,
        };
        PressStyle {
            // Colour drums sit 1.5–2 units out (a well-kept machine on a good day); the key
            // drum, printed last, is nearly true so faces stay crisp.
            offsets: [v2(1.6, 1.15), v2(-1.35, 0.8), v2(0.7, -1.45), v2(0.08, 0.1)],
            // Rotation/stretch add up to ~1.5 units more toward the corners; the kick flings
            // each drum further along its own way (≤ ~9 units, inside the plates' margin).
            drums: [
                drum(0.0014, 0.0008, -0.0005, 3.0, 2.1, 0.0022),
                drum(-0.001, -0.0006, 0.0011, -2.9, 1.7, -0.0018),
                drum(0.0008, 0.0004, 0.0009, 1.5, -3.1, 0.0012),
                drum(0.0001, 0.0, 0.0, 0.4, 0.5, 0.0002),
            ],
            kick: 0.0,
            warp: 0.3,
            spread: 0.05,
            rough: 0.34,
            mottle: [0.11, 0.11, 0.13, 0.06],
            velvet: [0.19, 0.18, 0.22, 0.1],
            pinholes: [0.85, 0.8, 0.85, 0.35],
            streaks: [0.1, 0.08, 0.1, 0.04],
            starve_x: [0.035, -0.025, 0.03, 0.0],
            starve_y: [0.025, 0.035, -0.025, 0.012],
            trap: [0.12, 0.0, 0.14, 0.05],
            tooth: 0.08,
            fibres: 0.18,
            formation: 0.045,
        }
    }
}

/// A named shader parameter value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Param {
    F(f32),
    V2(f32, f32),
    V4([f32; 4]),
}

impl PressStyle {
    /// Uniform values for the Godot shaders at `scale` device px per reference unit.
    /// Names match `riso_press.gdshaderinc` / `riso_ink.gdshader`.
    pub fn shader_params(&self, tex: &PressTextures, scale: f32) -> Vec<(&'static str, Param)> {
        let mut out = vec![
            ("fine_size", Param::F(tex.fine.size as f32)),
            ("mid_size", Param::F(tex.mid.size as f32)),
            ("coarse_px", Param::F(tex.coarse.size as f32 * COARSE_UNITS * scale)),
            ("warp_px", Param::F(self.warp * scale)),
            ("spread", Param::F(self.spread)),
            ("rough", Param::F(self.rough)),
            ("mottle", Param::V4(self.mottle)),
            ("velvet", Param::V4(self.velvet)),
            ("pinholes", Param::V4(self.pinholes)),
            ("streaks", Param::V4(self.streaks)),
            ("starve_x", Param::V4(self.starve_x)),
            ("starve_y", Param::V4(self.starve_y)),
            ("trap", Param::V4(self.trap)),
            ("tooth", Param::F(self.tooth)),
            ("fibres", Param::F(self.fibres)),
            ("formation", Param::F(self.formation)),
        ];
        const SHIFT: [&str; 4] = ["shift0", "shift1", "shift2", "shift3"];
        const LIN: [&str; 4] = ["lin0", "lin1", "lin2", "lin3"];
        for (k, d) in self.drums.iter().enumerate() {
            let (sh, a) = d.affine(self.offsets[k], self.kick);
            out.push((SHIFT[k], Param::V2(sh.x * scale, sh.y * scale)));
            out.push((LIN[k], Param::V4(a)));
        }
        out
    }

    /// Paper colour at a pixel (see `paper_color` in the shader include).
    #[inline]
    pub fn paper(&self, paper: Rgb, f: &[f32; 4], c: &[f32; 4]) -> Rgb {
        let form = c[3] - 0.5;
        let k = 1.0 - self.tooth * (f[0] - 0.5) + self.fibres * (f[1] - 0.5) - self.formation * form;
        Rgb(paper.0 * k, paper.1 * k, paper.2 * k * (1.0 - 0.6 * self.formation * form))
    }

    /// Ink film: turn raw plate coverage `v` (pink, yellow, blue, key) into printed density.
    /// `u` is the pixel's position on the page, 0..1.
    #[inline]
    pub fn ink(&self, v: [f32; 4], f: &[f32; 4], m: &[f32; 4], c: &[f32; 4], u: V2) -> [f32; 4] {
        let n = [f[2] - 0.5, m[2] - 0.5, 0.5 - f[2], f[0] - 0.5];
        let mut e = [0.0f32; 4];
        for k in 0..4 {
            let edge = 4.0 * v[k] * (1.0 - v[k]);
            e[k] = (v[k] + edge * (self.spread + self.rough * 2.0 * n[k])).clamp(0.0, 1.0);
        }
        let peak = smoothstep(0.52, 0.86, f[0]);
        let prior = [e[1], 0.0, (e[0] + e[1]).min(1.0), (e[0] + e[1] + e[2]).min(1.0)];
        let gate = smoothstep(0.3, 0.7, c[3]);
        let mut out = [0.0f32; 4];
        for k in 0..4 {
            let mot = smoothstep(0.32, 0.86, c[k]);
            let ramp = (u.x - 0.5) * self.starve_x[k] + (u.y - 0.5) * self.starve_y[k];
            let body = 1.0
                - self.mottle[k] * mot
                - self.velvet[k] * peak
                - self.pinholes[k] * f[3]
                - self.streaks[k] * m[3] * gate
                - ramp
                - self.trap[k] * prior[k] * m[2];
            out[k] = e[k] * body.clamp(0.0, 1.0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textures_are_deterministic_and_centred() {
        let a = PressTextures::build(1.5);
        let b = PressTextures::build(1.5);
        assert_eq!(a.fine.rgba, b.fine.rgba);
        assert_eq!(a.coarse.rgba, b.coarse.rgba);
        // Channels meant to be neutral around 0.5 are.
        for (tex, ch) in [(&a.fine, 0), (&a.fine, 1), (&a.fine, 2), (&a.mid, 0), (&a.coarse, 1)] {
            let n = (tex.size * tex.size) as f32;
            let mean: f32 = tex.rgba.chunks_exact(4).map(|p| p[ch] as f32 / 255.0).sum::<f32>() / n;
            assert!((mean - 0.5).abs() < 0.03, "channel {ch} mean {mean}");
        }
    }

    #[test]
    fn every_shader_param_is_a_uniform_of_the_riso_shaders() {
        // The Godot press reads these names; a rename on one side only must fail here.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../godot/shaders");
        let src: String = ["riso_press.gdshaderinc", "riso_ink.gdshader"]
            .iter()
            .map(|f| std::fs::read_to_string(dir.join(f)).expect("shader source"))
            .collect();
        let uniforms: Vec<&str> = src
            .lines()
            .filter(|l| l.trim_start().starts_with("uniform "))
            .filter_map(|l| l.split_whitespace().nth(2))
            .map(|w| w.trim_end_matches(';'))
            .collect();
        let tex = press_textures(1.0);
        for (name, _) in PressStyle::default().shader_params(&tex, 1.0) {
            assert!(uniforms.contains(&name), "shader uniform `{name}` missing");
        }
        for name in ["fine_tex", "mid_tex", "coarse_tex", "paper", "ink0", "ink1", "ink2", "ink3"] {
            assert!(uniforms.contains(&name), "shader uniform `{name}` missing");
        }
    }

    #[test]
    fn solid_ink_prints_dense_but_not_flat() {
        let tex = press_textures(1.5);
        let st = PressStyle::default();
        let (mut sum, mut min, mut max) = (0.0f32, 1.0f32, 0.0f32);
        let n = 200;
        for y in 0..n {
            for x in 0..n {
                let f = tex.fine.texel(x, y);
                let m = tex.mid.texel(x, y);
                let c = tex.coarse.bilinear(x as f32 * 0.3, y as f32 * 0.3);
                let out = st.ink([1.0; 4], &f, &m, &c, v2(0.5, 0.5));
                sum += out[0];
                min = min.min(out[0]);
                max = max.max(out[0]);
            }
        }
        let mean = sum / (n * n) as f32;
        assert!(mean > 0.82 && mean < 0.97, "solid density {mean}");
        assert!(max - min > 0.3, "solid ink has texture ({min}..{max})");
    }
}
