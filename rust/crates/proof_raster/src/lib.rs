//! Rasterise [`proof_core::draw::DrawList`]s into four riso masters ("plates").
//!
//! Output is a single RGBA8 buffer where each channel holds one ink drum's master
//! (R = pink, G = yellow, B = blue, A = key). Like a real risograph master every tint is
//! *screened* (see [`screen`]): flat tints become FM "grain touch" dots and halftones become
//! AM dots at the ink's screen angle, all anti-aliased. The press ([`press`], and its GPU twin
//! `riso_press.gdshaderinc`) then prints the plates: per-drum misregistration, ink spread,
//! mottled ink film, pinholes, paper tooth, and multiplying overprints.
//!
//! tiny-skia is used only to scan-convert shapes into coverage masks; screening and the
//! "last draw wins" plate logic are done here per pixel.

mod noise;
pub mod press;
pub mod screen;

pub use press::{Drum, Param, PressStyle, PressTextures, Tex, press_textures};

/// The press settings used to composite plates (the CPU preview of the riso shaders).
pub type CompositeStyle = PressStyle;

use proof_core::draw::{DrawList, Mode, Op, Shape};
use proof_core::geom::{Rect, V2, v2};
use proof_core::ink::{Palette, Rgb};
use screen::{Phase, Screens};
use std::sync::Arc;
use tiny_skia::{FillRule, LineCap, LineJoin, Mask, Path, PathBuilder, Stroke, Transform};

/// How to rasterise a list.
#[derive(Clone, Copy, Debug)]
pub struct RasterConfig {
    /// Device pixels per reference unit.
    pub scale: f32,
    /// Halftone cell size in reference units.
    pub pitch: f32,
    /// Screen angle per ink (radians).
    pub angles: [f32; 4],
    /// Transparent margin around the art, in reference units (room for misregistration).
    pub pad: f32,
    /// FM grain size for flat tints, in reference units.
    pub grain: f32,
}

impl Default for RasterConfig {
    fn default() -> Self {
        RasterConfig {
            scale: 1.0,
            pitch: 4.6,
            // Classic screen angles (C 15°, Y 0°, M 75°, K 45°) so overlapping dots don't moiré.
            angles: [15f32.to_radians(), 0.0, 75f32.to_radians(), 45f32.to_radians()],
            pad: 10.0,
            grain: 0.85,
        }
    }
}

/// Four ink plates packed into RGBA8, plus an optional opaque paper backing mask.
#[derive(Clone, Debug)]
pub struct Plates {
    pub width: u32,
    pub height: u32,
    /// Reference-space position of pixel (0, 0).
    pub origin: V2,
    pub scale: f32,
    /// R = pink, G = yellow, B = blue, A = key coverage.
    pub rgba: Vec<u8>,
    /// One byte per pixel, present when the list used `backing()`.
    pub backing: Option<Vec<u8>>,
}

impl Plates {
    /// Reference-space rectangle covered by the plates.
    pub fn ref_rect(&self) -> Rect {
        Rect {
            x: self.origin.x,
            y: self.origin.y,
            w: self.width as f32 / self.scale,
            h: self.height as f32 / self.scale,
        }
    }
}

fn path_from(pts: impl Iterator<Item = V2>, closed: bool) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let mut first = true;
    for p in pts {
        if first {
            pb.move_to(p.x, p.y);
            first = false;
        } else {
            pb.line_to(p.x, p.y);
        }
    }
    if closed {
        pb.close();
    }
    pb.finish()
}

/// Scan-convertible outline of a shape in device pixels.
fn device_path(shape: &Shape, s: f32, ox: f32, oy: f32) -> Option<(Path, FillRule)> {
    let dev = move |p: &V2| v2(p.x * s - ox, p.y * s - oy);
    match shape {
        Shape::Poly(p) if p.len() >= 3 => path_from(p.iter().map(dev), true).map(|p| (p, FillRule::Winding)),
        Shape::Poly(_) => None,
        Shape::PolysEo(ps) => {
            let mut pb = PathBuilder::new();
            for poly in ps.iter().filter(|p| p.len() >= 3) {
                let q = dev(&poly[0]);
                pb.move_to(q.x, q.y);
                for p in &poly[1..] {
                    let q = dev(p);
                    pb.line_to(q.x, q.y);
                }
                pb.close();
            }
            pb.finish().map(|p| (p, FillRule::EvenOdd))
        }
        Shape::Line { pts, width, closed } => {
            if pts.len() < 2 {
                return None;
            }
            // A zero-length line still deserves a round dot.
            let pts: Vec<V2> = if pts.len() == 2 && pts[0].dist(pts[1]) < 0.01 {
                vec![dev(&pts[0]), dev(&pts[0]) + v2(0.01, 0.0)]
            } else {
                pts.iter().map(dev).collect()
            };
            let stroke = Stroke {
                width: (width * s).max(0.01),
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            let line = path_from(pts.into_iter(), *closed)?;
            line.stroke(&stroke, 1.0).map(|p| (p, FillRule::Winding))
        }
    }
}

/// Pixel box touched by a device path (grown for anti-aliasing), clamped to the plate.
fn pixel_box(path: &Path, w: u32, h: u32) -> Option<(usize, usize, usize, usize)> {
    let b = path.bounds();
    let x0 = (b.left().floor() as i64 - 2).max(0);
    let y0 = (b.top().floor() as i64 - 2).max(0);
    let x1 = (b.right().ceil() as i64 + 2).min(w as i64);
    let y1 = (b.bottom().ceil() as i64 + 2).min(h as i64);
    (x1 > x0 && y1 > y0).then_some((x0 as usize, y0 as usize, x1 as usize, y1 as usize))
}

const INV255: f32 = 1.0 / 255.0;

#[inline]
fn to_u8(v: f32) -> u8 {
    (v * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// Walk the scratch mask inside `bb` row by row, clearing it as we go. For every row with
/// coverage, `f(y, x, cov)` gets the first covered column and the coverage of the span up to
/// the last covered column (clip applied; 0 where uncovered).
#[inline]
fn spans(
    scratch: &mut [u8],
    clip: Option<&[u8]>,
    w: usize,
    bb: (usize, usize, usize, usize),
    cov: &mut Vec<f32>,
    mut f: impl FnMut(usize, usize, &[f32]),
) {
    let (x0, y0, x1, y1) = bb;
    for y in y0..y1 {
        let row = &mut scratch[y * w + x0..y * w + x1];
        let Some(a) = row.iter().position(|c| *c != 0) else { continue };
        let b = row.iter().rposition(|c| *c != 0).map_or(a, |b| b) + 1;
        cov.clear();
        match clip {
            Some(cl) => {
                let cr = &cl[y * w + x0 + a..y * w + x0 + b];
                cov.extend(
                    row[a..b].iter().zip(cr).map(|(c, k)| (*c as u32 * *k as u32) as f32 * (1.0 / 65025.0)),
                );
            }
            None => cov.extend(row[a..b].iter().map(|c| *c as f32 * INV255)),
        }
        row[a..b].fill(0);
        f(y, x0 + a, cov);
    }
}

/// Rasterise `list` into plates. `bounds` (reference units) defaults to the list's ink bounds.
pub fn rasterize(list: &DrawList, cfg: &RasterConfig, bounds: Option<Rect>) -> Plates {
    let b = bounds.or_else(|| list.bounds()).unwrap_or(Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }).grow(cfg.pad);
    let s = cfg.scale;
    let ox = (b.x * s).floor();
    let oy = (b.y * s).floor();
    let w = ((b.x + b.w) * s).ceil() as i64 - ox as i64;
    let h = ((b.y + b.h) * s).ceil() as i64 - oy as i64;
    let (w, h) = (w.clamp(1, 4096) as u32, h.clamp(1, 4096) as u32);
    let (wu, n) = (w as usize, (w * h) as usize);
    let (oxi, oyi) = (ox as i64, oy as i64);

    let screens = Screens::new(s, cfg.pitch, cfg.grain, cfg.angles);
    let mut rgba = vec![0u8; n * 4];
    let mut backing: Option<Vec<u8>> = None;
    let mut scratch = Mask::new(w, h).expect("mask");
    let mut clips: Vec<Mask> = Vec::new();
    let mut covbuf: Vec<f32> = Vec::with_capacity(wu);
    let mut sbuf: Vec<f32> = vec![0.0; wu];
    // Plates that receive an `Add` remember the (tone, screen) each pixel was last printed
    // with, so adding the same ink on the same screen is flattened like separation software
    // does: the union tone is re-screened in place (dots grow) instead of doubling the grid.
    let mut rec: [Option<Vec<u8>>; 4] = std::array::from_fn(|k| {
        list.cmds
            .iter()
            .any(|c| matches!(&c.op, Op::Ink(p) if p.mode == Mode::Add && p.ink.idx() == k))
            .then(|| vec![0u8; n * 2])
    });

    for cmd in &list.cmds {
        match &cmd.op {
            Op::ClipPop => {
                clips.pop();
                continue;
            }
            Op::ClipPush => {
                let mut m = match clips.last() {
                    Some(top) => top.clone(),
                    None => Mask::new(w, h).expect("mask"),
                };
                match device_path(&cmd.shape, s, ox, oy) {
                    Some((path, rule)) if clips.is_empty() => {
                        m.fill_path(&path, rule, true, Transform::identity())
                    }
                    Some((path, rule)) => m.intersect_path(&path, rule, true, Transform::identity()),
                    // An empty clip hides everything until it is popped.
                    None => m.clear(),
                }
                clips.push(m);
                continue;
            }
            _ => {}
        }
        let Some((path, rule)) = device_path(&cmd.shape, s, ox, oy) else { continue };
        let Some(bb) = pixel_box(&path, w, h) else { continue };
        scratch.fill_path(&path, rule, true, Transform::identity());
        let clip = clips.last().map(|m| m.data());
        let sc = scratch.data_mut();
        match &cmd.op {
            Op::Ink(p) => {
                let k = p.ink.idx();
                let sid = 1 + p.screen as u8;
                let tone = p.tone.clamp(0.0, 1.0);
                let mut rk = rec[k].as_deref_mut();
                if p.mode == Mode::Over {
                    let scr = screens.prepare(k, p.screen, tone, Phase::Base);
                    let t8 = to_u8(tone);
                    spans(sc, clip, wu, bb, &mut covbuf, |y, xa, cov| {
                        let sv = &mut sbuf[..cov.len()];
                        scr.row(xa as i64 + oxi, y as i64 + oyi, sv);
                        let base = y * wu + xa;
                        for (j, (&c, &v)) in cov.iter().zip(sv.iter()).enumerate() {
                            if c <= 0.0 {
                                continue;
                            }
                            let i = base + j;
                            let o = &mut rgba[i * 4 + k];
                            let old = *o as f32 * INV255;
                            *o = to_u8(old + (v - old) * c);
                            if c > 0.5
                                && let Some(r) = rk.as_deref_mut()
                            {
                                r[i * 2] = t8;
                                r[i * 2 + 1] = sid;
                            }
                        }
                    });
                } else {
                    let flat = screens.prepare(k, p.screen, tone, Phase::Base);
                    let free = screens.prepare(k, p.screen, tone, Phase::Add);
                    spans(sc, clip, wu, bb, &mut covbuf, |y, xa, cov| {
                        let sv = &mut sbuf[..cov.len()];
                        free.row(xa as i64 + oxi, y as i64 + oyi, sv);
                        let base = y * wu + xa;
                        for (j, (&c, &v)) in cov.iter().zip(sv.iter()).enumerate() {
                            if c <= 0.0 {
                                continue;
                            }
                            let i = base + j;
                            let o = &mut rgba[i * 4 + k];
                            let old = *o as f32 * INV255;
                            match rk.as_deref_mut() {
                                Some(r) if r[i * 2 + 1] == sid && r[i * 2] > 0 => {
                                    // Same ink, same screen: print the flattened union tone.
                                    let t0 = r[i * 2] as f32 * INV255;
                                    let u = t0 + tone - t0 * tone;
                                    let v = flat.cov_tone((xa + j) as i64 + oxi, y as i64 + oyi, u);
                                    *o = to_u8(old + (v - old) * c);
                                    if c > 0.5 {
                                        r[i * 2] = to_u8(u);
                                    }
                                }
                                _ => *o = to_u8(old + v * c * (1.0 - old)),
                            }
                        }
                    });
                }
            }
            Op::Knock { tone, screen, plates: bits } => {
                let scr: Vec<(usize, screen::Screener)> = (0..4)
                    .filter(|k| bits & (1 << k) != 0)
                    .map(|k| (k, screens.prepare(k, *screen, *tone, Phase::Knock)))
                    .collect();
                spans(sc, clip, wu, bb, &mut covbuf, |y, xa, cov| {
                    let base = y * wu + xa;
                    for (k, s) in &scr {
                        let sv = &mut sbuf[..cov.len()];
                        s.row(xa as i64 + oxi, y as i64 + oyi, sv);
                        for (j, (&c, &v)) in cov.iter().zip(sv.iter()).enumerate() {
                            if c <= 0.0 {
                                continue;
                            }
                            let i = base + j;
                            let o = &mut rgba[i * 4 + k];
                            *o = to_u8(*o as f32 * INV255 * (1.0 - v * c));
                            if c > 0.5
                                && let Some(r) = rec[*k].as_deref_mut()
                            {
                                r[i * 2 + 1] = 0;
                            }
                        }
                    }
                });
            }
            Op::Backing => {
                let bk = backing.get_or_insert_with(|| vec![0u8; n]);
                spans(sc, clip, wu, bb, &mut covbuf, |y, xa, cov| {
                    let base = y * wu + xa;
                    for (j, &c) in cov.iter().enumerate() {
                        if c <= 0.0 {
                            continue;
                        }
                        let i = base + j;
                        for (k, r) in rec.iter_mut().enumerate() {
                            let o = &mut rgba[i * 4 + k];
                            *o = to_u8(*o as f32 * INV255 * (1.0 - c));
                            if c > 0.5
                                && let Some(r) = r.as_deref_mut()
                            {
                                r[i * 2 + 1] = 0;
                            }
                        }
                        let o = &mut bk[i];
                        *o = to_u8(*o as f32 * INV255 + c * (1.0 - *o as f32 * INV255));
                    }
                });
            }
            Op::ClipPush | Op::ClipPop => unreachable!(),
        }
    }
    Plates { width: w, height: h, origin: v2(ox / s, oy / s), scale: s, rgba, backing }
}

/// Sample one plate channel with bilinear filtering at a fractional pixel position
/// (pixel centres at +0.5, like a GPU texture).
#[inline]
fn sample(pl: &Plates, ch: usize, x: f32, y: f32) -> f32 {
    let (w, h) = (pl.width as i64, pl.height as i64);
    let (x, y) = (x - 0.5, y - 0.5);
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let at = |xx: i64, yy: i64| -> f32 {
        if xx < 0 || yy < 0 || xx >= w || yy >= h {
            0.0
        } else {
            pl.rgba[((yy * w + xx) * 4) as usize + ch] as f32 * INV255
        }
    };
    let a = at(x0, y0) + (at(x0 + 1, y0) - at(x0, y0)) * fx;
    let b = at(x0, y0 + 1) + (at(x0 + 1, y0 + 1) - at(x0, y0 + 1)) * fx;
    a + (b - a) * fy
}

/// An RGB canvas used by tools and tests to preview printing without Godot.
/// Its pixels play the role of the phone screen: the page centre (for drum rotation) is
/// the canvas centre and paper/ink textures are laid in canvas pixels.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// Device pixels per reference unit.
    pub scale: f32,
    pub rgb: Vec<Rgb>,
    press: Arc<PressTextures>,
}

impl Canvas {
    /// A sheet of paper `w`×`h` reference units.
    pub fn paper(w: f32, h: f32, scale: f32, palette: &Palette) -> Canvas {
        Canvas::paper_styled(w, h, scale, palette, &PressStyle::default())
    }

    pub fn paper_styled(w: f32, h: f32, scale: f32, palette: &Palette, style: &PressStyle) -> Canvas {
        let (pw, ph) = ((w * scale).round().max(1.0) as u32, (h * scale).round().max(1.0) as u32);
        let press = press_textures(scale);
        let mut rgb = vec![palette.paper; (pw * ph) as usize];
        let cpx = press.coarse_px();
        let csz = press.coarse.size as f32;
        for y in 0..ph {
            for x in 0..pw {
                let f = press.fine.texel(x as i64, y as i64);
                let c = press.coarse.bilinear((x as f32 + 0.5) / cpx * csz, (y as f32 + 0.5) / cpx * csz);
                rgb[(y * pw + x) as usize] = style.paper(palette.paper, &f, &c);
            }
        }
        Canvas { width: pw, height: ph, scale, rgb, press }
    }

    /// Print plates onto the canvas at their reference position (+ `offset`).
    pub fn draw(&mut self, pl: &Plates, offset: V2, palette: &Palette, style: &CompositeStyle) {
        let s = self.scale;
        let press = self.press.clone();
        let r = pl.ref_rect();
        // Room for the drums' displacement.
        let m = 12.0;
        let x0 = (((r.x + offset.x - m) * s).floor() as i64).max(0);
        let y0 = (((r.y + offset.y - m) * s).floor() as i64).max(0);
        let x1 = (((r.x + r.w + offset.x + m) * s).ceil() as i64).min(self.width as i64);
        let y1 = (((r.y + r.h + offset.y + m) * s).ceil() as i64).min(self.height as i64);
        let centre = v2(self.width as f32 * 0.5, self.height as f32 * 0.5);
        let drums: [(V2, [f32; 4]); 4] = std::array::from_fn(|k| {
            let (sh, a) = style.drums[k].affine(style.offsets[k], style.kick);
            (sh * s, a)
        });
        let warp = style.warp * s;
        let cpx = press.coarse_px();
        let csz = press.coarse.size as f32;
        // Canvas px → plate px.
        let k = pl.scale / s;
        let (px0, py0) = ((offset.x + pl.origin.x) * s, (offset.y + pl.origin.y) * s);
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = v2(x as f32 + 0.5, y as f32 + 0.5);
                let f = press.fine.texel(x, y);
                let mt = press.mid.texel(x, y);
                let d = p - centre;
                let wv = v2((mt[0] - 0.5) * 2.0 * warp, (mt[1] - 0.5) * 2.0 * warp);
                // Each drum reads the warp turned a quarter further, so edges rag independently.
                let warps = [wv, v2(-wv.y, wv.x), v2(wv.y, -wv.x), wv * -0.6];
                let mut v = [0.0f32; 4];
                let mut any = 0.0;
                for (ch, ((sh, a), wk)) in drums.iter().zip(warps).enumerate() {
                    let disp = *sh + v2(a[0] * d.x + a[1] * d.y, a[2] * d.x + a[3] * d.y);
                    let q = p - disp + wk;
                    v[ch] = sample(pl, ch, (q.x - px0) * k, (q.y - py0) * k);
                    any += v[ch];
                }
                let idx = (y as u32 * self.width + x as u32) as usize;
                let mut base = self.rgb[idx];
                let mut backed = false;
                if let Some(bk) = &pl.backing {
                    let bx = ((p.x - px0) * k).floor() as i64;
                    let by = ((p.y - py0) * k).floor() as i64;
                    if bx >= 0 && by >= 0 && (bx as u32) < pl.width && (by as u32) < pl.height {
                        let a = bk[(by as u32 * pl.width + bx as u32) as usize] as f32 * INV255;
                        if a > 0.0 {
                            let c = press.coarse.bilinear(p.x / cpx * csz, p.y / cpx * csz);
                            base = base.lerp(style.paper(palette.paper, &f, &c), a);
                            backed = true;
                        }
                    }
                }
                if any < 0.002 {
                    if backed {
                        self.rgb[idx] = base;
                    }
                    continue;
                }
                let c = press.coarse.bilinear(p.x / cpx * csz, p.y / cpx * csz);
                let cov = style.ink(v, &f, &mt, &c, v2(p.x * inv_w, p.y * inv_h));
                self.rgb[idx] = palette.composite(base, cov);
            }
        }
    }

    pub fn to_rgba8(&self) -> Vec<u8> {
        self.rgb
            .iter()
            .flat_map(|c| {
                let [r, g, b] = c.to_u8();
                [r, g, b, 255]
            })
            .collect()
    }
}

/// Convenience: print one list on its own sheet of paper.
pub fn preview(list: &DrawList, cfg: &RasterConfig, palette: &Palette, style: &CompositeStyle) -> Canvas {
    let pl = rasterize(list, cfg, None);
    let r = pl.ref_rect();
    let mut c = Canvas::paper_styled(r.w, r.h, cfg.scale, palette, style);
    c.draw(&pl, v2(-r.x, -r.y), palette, style);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use proof_core::draw::{Paint as DPaint, Screen};
    use proof_core::geom::{circle, rect, rect_poly};
    use proof_core::ink::{Edition, Ink};

    fn at(pl: &Plates, ch: usize, x: f32, y: f32) -> u8 {
        let px = ((x - pl.origin.x) * pl.scale) as u32;
        let py = ((y - pl.origin.y) * pl.scale) as u32;
        pl.rgba[((py * pl.width + px) * 4) as usize + ch]
    }

    /// Mean plate coverage over a reference-space rectangle.
    fn mean(pl: &Plates, ch: usize, r: Rect) -> f32 {
        let (x0, y0) = (((r.x - pl.origin.x) * pl.scale) as u32, ((r.y - pl.origin.y) * pl.scale) as u32);
        let (x1, y1) = (x0 + (r.w * pl.scale) as u32, y0 + (r.h * pl.scale) as u32);
        let mut sum = 0.0f64;
        for y in y0..y1 {
            for x in x0..x1 {
                sum += pl.rgba[((y * pl.width + x) * 4) as usize + ch] as f64 / 255.0;
            }
        }
        (sum / ((x1 - x0) * (y1 - y0)) as f64) as f32
    }

    #[test]
    fn solid_fill_lands_on_its_plate_only() {
        let mut d = DrawList::new();
        d.fill(Ink::Yellow, 1.0, &rect_poly(rect(0.0, 0.0, 50.0, 50.0)));
        let pl = rasterize(&d, &RasterConfig::default(), None);
        assert_eq!(at(&pl, 1, 25.0, 25.0), 255);
        assert_eq!(at(&pl, 0, 25.0, 25.0), 0);
        assert_eq!(at(&pl, 3, 25.0, 25.0), 0);
    }

    #[test]
    fn over_replaces_and_add_unions() {
        let big = rect_poly(rect(0.0, 0.0, 100.0, 100.0));
        let small = rect_poly(rect(20.0, 20.0, 60.0, 60.0));
        let inner = rect(25.0, 25.0, 50.0, 50.0);
        let cfg = RasterConfig { scale: 2.0, ..RasterConfig::default() };
        let mut d = DrawList::new();
        d.fill(Ink::Pink, 1.0, &big);
        d.fill(Ink::Pink, 0.2, &small);
        let pl = rasterize(&d, &cfg, None);
        let v = mean(&pl, 0, inner);
        assert!((v - 0.2).abs() < 0.03, "last draw wins: {v}");
        assert_eq!(at(&pl, 0, 10.0, 10.0), 255);

        let mut d = DrawList::new();
        d.fill(Ink::Pink, 1.0, &big);
        d.fill_p(DPaint::solid(Ink::Pink, 0.2).add(), &small);
        let pl = rasterize(&d, &cfg, None);
        assert_eq!(at(&pl, 0, 50.0, 50.0), 255);
    }

    #[test]
    fn screened_tints_average_to_their_tone() {
        let cfg = RasterConfig { scale: 1.5, ..RasterConfig::default() };
        let area = rect(0.0, 0.0, 240.0, 240.0);
        let inner = rect(20.0, 20.0, 200.0, 200.0);
        for screen in [Screen::Solid, Screen::Halftone, Screen::Coarse] {
            for ink in Ink::ALL {
                for tone in [0.04, 0.15, 0.3, 0.5, 0.7, 0.9] {
                    let mut d = DrawList::new();
                    d.fill_p(DPaint { ink, tone, screen, mode: Mode::Over }, &rect_poly(area));
                    let pl = rasterize(&d, &cfg, None);
                    let v = mean(&pl, ink.idx(), inner);
                    assert!((v - tone).abs() < 0.025, "{screen:?} {ink:?} {tone}: {v}");
                }
            }
        }
    }

    #[test]
    fn tints_are_screened_not_flat() {
        let cfg = RasterConfig { scale: 1.5, ..RasterConfig::default() };
        for screen in [Screen::Solid, Screen::Halftone] {
            let mut d = DrawList::new();
            d.fill_p(
                DPaint { ink: Ink::Blue, tone: 0.4, screen, mode: Mode::Over },
                &rect_poly(rect(0.0, 0.0, 100.0, 100.0)),
            );
            let pl = rasterize(&d, &cfg, None);
            let px: Vec<u8> = pl.rgba.chunks_exact(4).map(|p| p[2]).filter(|v| *v > 0).collect();
            let solid = px.iter().filter(|v| **v > 230).count() as f32 / px.len() as f32;
            let paper = pl.rgba.chunks_exact(4).filter(|p| p[2] < 25).count();
            assert!(solid > 0.2, "{screen:?}: dots are mostly full ink ({solid})");
            assert!(paper > 1000, "{screen:?}: paper shows between dots");
        }
    }

    #[test]
    fn add_on_the_same_screen_still_darkens() {
        let cfg = RasterConfig { scale: 1.5, ..RasterConfig::default() };
        let area = rect_poly(rect(0.0, 0.0, 200.0, 200.0));
        for screen in [Screen::Solid, Screen::Halftone] {
            let mut d = DrawList::new();
            d.fill_p(DPaint { ink: Ink::Blue, tone: 0.34, screen, mode: Mode::Over }, &area);
            d.fill_p(DPaint { ink: Ink::Blue, tone: 0.3, screen, mode: Mode::Add }, &area);
            let pl = rasterize(&d, &cfg, None);
            let v = mean(&pl, 2, rect(20.0, 20.0, 160.0, 160.0));
            // Flattened intent: 0.34 + 0.3 − 0.34·0.3 = 0.54.
            assert!((v - 0.54).abs() < 0.06, "{screen:?}: {v}");
        }
    }

    #[test]
    fn knockout_clears_every_plate_and_backing_is_recorded() {
        let mut d = DrawList::new();
        let big = rect_poly(rect(0.0, 0.0, 60.0, 60.0));
        d.fill(Ink::Pink, 1.0, &big);
        d.fill(Ink::Key, 1.0, &big);
        d.knock(&circle(v2(30.0, 30.0), 10.0));
        d.backing(&rect_poly(rect(0.0, 50.0, 10.0, 10.0)));
        let pl = rasterize(&d, &RasterConfig::default(), None);
        assert_eq!(at(&pl, 0, 30.0, 30.0), 0);
        assert_eq!(at(&pl, 3, 30.0, 30.0), 0);
        assert_eq!(at(&pl, 3, 5.0, 5.0), 255);
        assert_eq!(at(&pl, 3, 5.0, 55.0), 0, "backing knocks earlier ink");
        assert!(pl.backing.is_some());
    }

    #[test]
    fn halftone_average_matches_tone() {
        let mut d = DrawList::new();
        d.ht(Ink::Blue, 0.4, &rect_poly(rect(0.0, 0.0, 200.0, 200.0)));
        let cfg = RasterConfig { scale: 2.0, ..RasterConfig::default() };
        let pl = rasterize(&d, &cfg, None);
        let avg = mean(&pl, 2, rect(20.0, 20.0, 160.0, 160.0));
        assert!((avg - 0.4).abs() < 0.03, "avg {avg}");
    }

    #[test]
    fn clip_limits_drawing() {
        let mut d = DrawList::new();
        d.clipped(&rect_poly(rect(0.0, 0.0, 20.0, 20.0)), |d| {
            d.fill(Ink::Pink, 1.0, &rect_poly(rect(0.0, 0.0, 60.0, 60.0)));
        });
        let pl = rasterize(&d, &RasterConfig::default(), Some(rect(0.0, 0.0, 60.0, 60.0)));
        assert_eq!(at(&pl, 0, 10.0, 10.0), 255);
        assert_eq!(at(&pl, 0, 40.0, 40.0), 0);
    }

    #[test]
    fn deterministic_output() {
        let mut d = DrawList::new();
        d.ht(Ink::Pink, 0.3, &circle(v2(0.0, 0.0), 40.0));
        d.fill(Ink::Yellow, 0.45, &circle(v2(10.0, 0.0), 30.0));
        d.line(Ink::Key, 3.0, &[v2(-30.0, 0.0), v2(30.0, 10.0)]);
        let a = rasterize(&d, &RasterConfig::default(), None);
        let b = rasterize(&d, &RasterConfig::default(), None);
        assert_eq!(a.rgba, b.rgba);
    }

    #[test]
    fn printed_solids_keep_their_ink_colour() {
        // A solid pink patch prints close to (but a little lighter and textured than) the ink.
        let pal = Edition::Daylight.palette();
        let mut d = DrawList::new();
        d.fill(Ink::Pink, 1.0, &rect_poly(rect(0.0, 0.0, 120.0, 120.0)));
        let c = preview(
            &d,
            &RasterConfig { scale: 1.5, ..RasterConfig::default() },
            &pal,
            &CompositeStyle::default(),
        );
        let target = pal.paper.times(pal.inks[0]);
        let (mut g, mut n) = (0.0f32, 0.0f32);
        for y in 40..160u32 {
            for x in 40..160u32 {
                g += c.rgb[(y * c.width + x) as usize].1;
                n += 1.0;
            }
        }
        let g = g / n;
        assert!(g > target.1 && g < target.1 + 0.2, "green of printed pink {g} vs ink {}", target.1);
    }
}
