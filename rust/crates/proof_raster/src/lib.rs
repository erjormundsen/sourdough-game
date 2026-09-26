//! Rasterise [`proof_core::draw::DrawList`]s into four-ink riso plates with tiny-skia.
//!
//! Output is a single RGBA8 buffer where each channel holds one ink's coverage
//! (R = pink, G = yellow, B = blue, A = key). The Godot shader (and [`composite`] on the CPU)
//! multiplies the four inks over paper, applies grain and per-ink misregistration.

mod grain;
mod halftone;

pub use grain::{GRAIN_SIZE, grain_texture};

use proof_core::draw::{DrawList, Mode, Op, Screen, Shape};
use proof_core::geom::{Rect, V2, v2};
use proof_core::ink::{Palette, Rgb};
use tiny_skia::{
    BlendMode, Color, FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Shader,
    Stroke, Transform,
};

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
}

impl Default for RasterConfig {
    fn default() -> Self {
        RasterConfig {
            scale: 1.0,
            pitch: 4.6,
            // Classic riso-ish screen angles so overlapping halftones don't moiré.
            angles: [
                15f32.to_radians(),
                0.0,
                75f32.to_radians(),
                45f32.to_radians(),
            ],
            pad: 10.0,
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

fn path_from(pts: &[V2], closed: bool) -> Option<Path> {
    let mut pb = PathBuilder::with_capacity(pts.len() + 1, pts.len() + 1);
    let first = pts.first()?;
    pb.move_to(first.x, first.y);
    for p in &pts[1..] {
        pb.line_to(p.x, p.y);
    }
    if closed {
        pb.close();
    }
    pb.finish()
}

fn path_multi(polys: &[Vec<V2>]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    for poly in polys {
        let Some(first) = poly.first() else { continue };
        pb.move_to(first.x, first.y);
        for p in &poly[1..] {
            pb.line_to(p.x, p.y);
        }
        pb.close();
    }
    pb.finish()
}

enum Geo {
    Fill(Path, FillRule),
    Stroke(Path, Stroke),
}

fn geo_of(shape: &Shape) -> Option<Geo> {
    match shape {
        Shape::Poly(p) => path_from(p, true).map(|p| Geo::Fill(p, FillRule::Winding)),
        Shape::PolysEo(ps) => path_multi(ps).map(|p| Geo::Fill(p, FillRule::EvenOdd)),
        Shape::Line { pts, width, closed } => {
            // A zero-length line still deserves a round dot.
            let pts = if pts.len() == 2 && pts[0].dist(pts[1]) < 0.01 {
                vec![pts[0], pts[0] + v2(0.01, 0.0)]
            } else {
                pts.clone()
            };
            let stroke = Stroke {
                width: width.max(0.01),
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            path_from(&pts, *closed).map(|p| Geo::Stroke(p, stroke))
        }
    }
}

fn draw_geo(pm: &mut Pixmap, geo: &Geo, paint: &Paint, ts: Transform, mask: Option<&Mask>) {
    match geo {
        Geo::Fill(path, rule) => pm.fill_path(path, paint, *rule, ts, mask),
        Geo::Stroke(path, stroke) => pm.stroke_path(path, paint, stroke, ts, mask),
    }
}

fn mask_geo(mask: &mut Mask, geo: &Geo, ts: Transform, first: bool) {
    let (path, rule) = match geo {
        Geo::Fill(p, r) => (p.clone(), *r),
        Geo::Stroke(p, s) => match p.stroke(s, 1.0) {
            Some(sp) => (sp, FillRule::Winding),
            None => return,
        },
    };
    if first {
        mask.fill_path(&path, rule, true, ts);
    } else {
        mask.intersect_path(&path, rule, true, ts);
    }
}

/// Rasterise `list` into plates. `bounds` (reference units) defaults to the list's ink bounds.
pub fn rasterize(list: &DrawList, cfg: &RasterConfig, bounds: Option<Rect>) -> Plates {
    let b = bounds
        .or_else(|| list.bounds())
        .unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        })
        .grow(cfg.pad);
    let s = cfg.scale;
    let ox = (b.x * s).floor();
    let oy = (b.y * s).floor();
    let w = ((b.x + b.w) * s).ceil() as i64 - ox as i64;
    let h = ((b.y + b.h) * s).ceil() as i64 - oy as i64;
    let (w, h) = (w.clamp(1, 4096) as u32, h.clamp(1, 4096) as u32);
    let ts = Transform::from_row(s, 0.0, 0.0, s, -ox, -oy);

    let mut plates: Vec<Pixmap> = (0..4).map(|_| Pixmap::new(w, h).expect("plate")).collect();
    let mut backing: Option<Pixmap> = None;
    let mut clips: Vec<Mask> = Vec::new();
    let mut tiles = halftone::TileCache::default();

    for cmd in &list.cmds {
        match &cmd.op {
            Op::ClipPop => {
                clips.pop();
                continue;
            }
            Op::ClipPush => {
                if let Some(geo) = geo_of(&cmd.shape) {
                    let mut m = match clips.last() {
                        Some(top) => top.clone(),
                        None => Mask::new(w, h).expect("mask"),
                    };
                    mask_geo(&mut m, &geo, ts, clips.is_empty());
                    clips.push(m);
                }
                continue;
            }
            _ => {}
        }
        let Some(geo) = geo_of(&cmd.shape) else {
            continue;
        };
        let clip = clips.last();
        match &cmd.op {
            Op::Ink(p) => {
                let shader = shader_for(&mut tiles, cfg, p.ink.idx(), p.tone, p.screen, ox, oy);
                let paint = Paint {
                    shader,
                    blend_mode: match p.mode {
                        Mode::Over => BlendMode::Source,
                        Mode::Add => BlendMode::SourceOver,
                    },
                    anti_alias: true,
                    ..Paint::default()
                };
                draw_geo(&mut plates[p.ink.idx()], &geo, &paint, ts, clip);
            }
            Op::Knock {
                tone,
                screen,
                plates: mask_bits,
            } => {
                for (i, plate) in plates.iter_mut().enumerate() {
                    if mask_bits & (1 << i) == 0 {
                        continue;
                    }
                    let shader = shader_for(&mut tiles, cfg, i, *tone, *screen, ox, oy);
                    let paint = Paint {
                        shader,
                        blend_mode: BlendMode::DestinationOut,
                        anti_alias: true,
                        ..Paint::default()
                    };
                    draw_geo(plate, &geo, &paint, ts, clip);
                }
            }
            Op::Backing => {
                let solid = Paint {
                    shader: Shader::SolidColor(Color::BLACK),
                    blend_mode: BlendMode::DestinationOut,
                    anti_alias: true,
                    ..Paint::default()
                };
                for plate in plates.iter_mut() {
                    draw_geo(plate, &geo, &solid, ts, clip);
                }
                let bk = backing.get_or_insert_with(|| Pixmap::new(w, h).expect("backing"));
                let paint = Paint {
                    blend_mode: BlendMode::SourceOver,
                    ..solid
                };
                draw_geo(bk, &geo, &paint, ts, clip);
            }
            Op::ClipPush | Op::ClipPop => unreachable!(),
        }
    }

    let n = (w * h) as usize;
    let mut rgba = vec![0u8; n * 4];
    for (k, plate) in plates.iter().enumerate() {
        let data = plate.data();
        for i in 0..n {
            rgba[i * 4 + k] = data[i * 4 + 3];
        }
    }
    let backing = backing.map(|bk| bk.data().chunks_exact(4).map(|px| px[3]).collect());
    Plates {
        width: w,
        height: h,
        origin: v2(ox / s, oy / s),
        scale: s,
        rgba,
        backing,
    }
}

fn shader_for<'a>(
    tiles: &'a mut halftone::TileCache,
    cfg: &RasterConfig,
    ink: usize,
    tone: f32,
    screen: Screen,
    ox: f32,
    oy: f32,
) -> Shader<'a> {
    let tone = tone.clamp(0.0, 1.0);
    match screen {
        Screen::Solid => Shader::SolidColor(Color::from_rgba(0.0, 0.0, 0.0, tone).unwrap()),
        Screen::Halftone | Screen::Coarse => {
            if tone >= 0.995 {
                return Shader::SolidColor(Color::BLACK);
            }
            let pitch = cfg.pitch * cfg.scale * if screen == Screen::Coarse { 2.0 } else { 1.0 };
            tiles.pattern(tone, pitch, cfg.angles[ink], ox, oy)
        }
    }
}

/// Settings for turning plates into colour (CPU preview of the Godot shader).
#[derive(Clone, Copy, Debug)]
pub struct CompositeStyle {
    /// Per-ink misregistration offsets in reference units.
    pub offsets: [V2; 4],
    /// Fine grain strength (0..1).
    pub grain: f32,
    /// Low-frequency ink starvation strength (0..1).
    pub starve: f32,
}

impl Default for CompositeStyle {
    fn default() -> Self {
        CompositeStyle {
            offsets: [v2(1.2, -0.8), v2(-0.9, 0.6), v2(0.7, 1.1), v2(0.0, 0.0)],
            grain: 0.22,
            starve: 0.12,
        }
    }
}

/// Sample one plate channel with bilinear filtering at a fractional pixel position.
fn sample(pl: &Plates, ch: usize, x: f32, y: f32) -> f32 {
    let (w, h) = (pl.width as i64, pl.height as i64);
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let at = |xx: i64, yy: i64| -> f32 {
        if xx < 0 || yy < 0 || xx >= w || yy >= h {
            0.0
        } else {
            pl.rgba[((yy * w + xx) * 4) as usize + ch] as f32 / 255.0
        }
    };
    let a = at(x0, y0) + (at(x0 + 1, y0) - at(x0, y0)) * fx;
    let b = at(x0, y0 + 1) + (at(x0 + 1, y0 + 1) - at(x0, y0 + 1)) * fx;
    a + (b - a) * fy
}

/// An RGB canvas used by tools and tests to preview composition without Godot.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// Device pixels per reference unit.
    pub scale: f32,
    pub rgb: Vec<Rgb>,
    grain: Vec<u8>,
}

impl Canvas {
    /// A sheet of paper `w`×`h` reference units.
    pub fn paper(w: f32, h: f32, scale: f32, palette: &Palette) -> Canvas {
        let (pw, ph) = ((w * scale).round() as u32, (h * scale).round() as u32);
        let grain = grain_texture();
        let mut c = Canvas {
            width: pw,
            height: ph,
            scale,
            rgb: vec![palette.paper; (pw * ph) as usize],
            grain,
        };
        // Paper tooth: faint fibres from the grain texture.
        for y in 0..ph {
            for x in 0..pw {
                let (g, coarse) = c.grain_at(x as f32, y as f32);
                let k = 1.0 - 0.035 * g - 0.03 * coarse;
                let p = &mut c.rgb[(y * pw + x) as usize];
                *p = Rgb(p.0 * k, p.1 * k, p.2 * (k - 0.004));
            }
        }
        c
    }

    fn grain_at(&self, x: f32, y: f32) -> (f32, f32) {
        let gx = (x as i64).rem_euclid(GRAIN_SIZE as i64) as usize;
        let gy = (y as i64).rem_euclid(GRAIN_SIZE as i64) as usize;
        let i = (gy * GRAIN_SIZE + gx) * 4;
        (
            self.grain[i] as f32 / 255.0,
            self.grain[i + 1] as f32 / 255.0,
        )
    }

    /// Multiply-composite plates onto the canvas at their reference position (+ `offset`).
    pub fn draw(&mut self, pl: &Plates, offset: V2, palette: &Palette, style: &CompositeStyle) {
        let rel = pl.scale / self.scale;
        let r = pl.ref_rect();
        let x0 = (((r.x + offset.x) * self.scale).floor() as i64).max(0);
        let y0 = (((r.y + offset.y) * self.scale).floor() as i64).max(0);
        let x1 = (((r.x + r.w + offset.x) * self.scale).ceil() as i64).min(self.width as i64);
        let y1 = (((r.y + r.h + offset.y) * self.scale).ceil() as i64).min(self.height as i64);
        for y in y0..y1 {
            for x in x0..x1 {
                // Canvas pixel centre → plate pixel.
                let rx = (x as f32 + 0.5) / self.scale - offset.x;
                let ry = (y as f32 + 0.5) / self.scale - offset.y;
                let (g, coarse) = self.grain_at(x as f32, y as f32);
                let mut cov = [0.0f32; 4];
                for (k, c) in cov.iter_mut().enumerate() {
                    let o = style.offsets[k];
                    let px = (rx - o.x - pl.origin.x) * pl.scale - 0.5;
                    let py = (ry - o.y - pl.origin.y) * pl.scale - 0.5;
                    let v = sample(pl, k, px, py);
                    // Key ink gets less grain so faces stay crisp.
                    let gk = if k == 3 { 0.4 } else { 1.0 };
                    *c = v * (1.0 - style.grain * gk * g) * (1.0 - style.starve * coarse);
                }
                let idx = (y as u32 * self.width + x as u32) as usize;
                let mut base = self.rgb[idx];
                if let Some(bk) = &pl.backing {
                    let px = ((rx - pl.origin.x) * pl.scale - 0.5) as i64;
                    let py = ((ry - pl.origin.y) * pl.scale - 0.5) as i64;
                    if px >= 0 && py >= 0 && (px as u32) < pl.width && (py as u32) < pl.height {
                        let a = bk[(py as u32 * pl.width + px as u32) as usize] as f32 / 255.0;
                        let paper =
                            palette
                                .paper
                                .mul(Rgb(1.0 - 0.03 * g, 1.0 - 0.03 * g, 1.0 - 0.035 * g));
                        base = base.lerp(paper, a);
                    }
                }
                self.rgb[idx] = palette.composite(base, cov);
                let _ = rel;
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

/// Convenience: composite one list on its own sheet of paper.
pub fn preview(
    list: &DrawList,
    cfg: &RasterConfig,
    palette: &Palette,
    style: &CompositeStyle,
) -> Canvas {
    let pl = rasterize(list, cfg, None);
    let r = pl.ref_rect();
    let mut c = Canvas::paper(r.w, r.h, cfg.scale, palette);
    c.draw(&pl, v2(-r.x, -r.y), palette, style);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use proof_core::draw::Paint as DPaint;
    use proof_core::geom::{circle, rect, rect_poly};
    use proof_core::ink::Ink;

    fn at(pl: &Plates, ch: usize, x: f32, y: f32) -> u8 {
        let px = ((x - pl.origin.x) * pl.scale) as u32;
        let py = ((y - pl.origin.y) * pl.scale) as u32;
        pl.rgba[((py * pl.width + px) * 4) as usize + ch]
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
        let big = rect_poly(rect(0.0, 0.0, 60.0, 60.0));
        let small = rect_poly(rect(20.0, 20.0, 20.0, 20.0));
        let mut d = DrawList::new();
        d.fill(Ink::Pink, 1.0, &big);
        d.fill(Ink::Pink, 0.2, &small);
        let pl = rasterize(&d, &RasterConfig::default(), None);
        let v = at(&pl, 0, 30.0, 30.0);
        assert!((45..=57).contains(&v), "last draw wins: {v}");

        let mut d = DrawList::new();
        d.fill(Ink::Pink, 1.0, &big);
        d.fill_p(DPaint::solid(Ink::Pink, 0.2).add(), &small);
        let pl = rasterize(&d, &RasterConfig::default(), None);
        assert_eq!(at(&pl, 0, 30.0, 30.0), 255);
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
        let cfg = RasterConfig {
            scale: 2.0,
            ..RasterConfig::default()
        };
        let pl = rasterize(&d, &cfg, None);
        let mut sum = 0.0;
        let mut n = 0.0;
        for y in 40..360u32 {
            for x in 40..360u32 {
                let px = x + (cfg.pad * cfg.scale) as u32;
                let py = y + (cfg.pad * cfg.scale) as u32;
                sum += pl.rgba[((py * pl.width + px) * 4 + 2) as usize] as f32 / 255.0;
                n += 1.0;
            }
        }
        let avg = sum / n;
        assert!((avg - 0.4).abs() < 0.05, "avg {avg}");
    }

    #[test]
    fn clip_limits_drawing() {
        let mut d = DrawList::new();
        d.clipped(&rect_poly(rect(0.0, 0.0, 20.0, 20.0)), |d| {
            d.fill(Ink::Pink, 1.0, &rect_poly(rect(0.0, 0.0, 60.0, 60.0)));
        });
        let pl = rasterize(
            &d,
            &RasterConfig::default(),
            Some(rect(0.0, 0.0, 60.0, 60.0)),
        );
        assert_eq!(at(&pl, 0, 10.0, 10.0), 255);
        assert_eq!(at(&pl, 0, 40.0, 40.0), 0);
    }

    #[test]
    fn deterministic_output() {
        let mut d = DrawList::new();
        d.ht(Ink::Pink, 0.3, &circle(v2(0.0, 0.0), 40.0));
        d.line(Ink::Key, 3.0, &[v2(-30.0, 0.0), v2(30.0, 10.0)]);
        let a = rasterize(&d, &RasterConfig::default(), None);
        let b = rasterize(&d, &RasterConfig::default(), None);
        assert_eq!(a.rgba, b.rgba);
    }
}
