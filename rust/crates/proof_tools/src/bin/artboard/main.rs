//! Render Proof's procedural art to PNG sheets for review (no Godot needed).
//!
//! Usage: `cargo run --release -p proof_tools --bin artboard -- [out_dir] [--scale S] [--sheet NAME]...`
//!
//! Sheets (each owned by one art area):
//! * `riso`   — press swatches: inks, tints, overprints, screens, edges (proof_raster + inks)
//! * `cast`   — the eight regulars, expressions, small heads (critters + faces)
//! * `bakery` — loaves, dough, crumb, jars, Toasty, treats (bread/jar/oven/treats)
//! * `scenes` — backdrops, props, icons, app icon (scenes/props/icons)

mod bakery;
mod cast;
mod riso;
mod scenes;

use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf};
use proof_core::ink::{Edition, Palette};
use proof_raster::{Canvas, CompositeStyle, RasterConfig, rasterize};
use std::path::{Path, PathBuf};

/// A sheet of paper to compose art on.
pub struct Board {
    pub canvas: Canvas,
    pub cfg: RasterConfig,
    pub palette: Palette,
    pub style: CompositeStyle,
}

impl Board {
    pub fn new(w: f32, h: f32, scale: f32, edition: Edition) -> Board {
        let palette = edition.palette();
        Board {
            canvas: Canvas::paper(w, h, scale, &palette),
            cfg: RasterConfig { scale, ..RasterConfig::default() },
            palette,
            style: CompositeStyle::default(),
        }
    }

    /// Draw art (authored around its own origin) at `at`.
    pub fn put(&mut self, at: V2, f: impl FnOnce(&mut DrawList)) {
        let mut d = DrawList::new();
        d.with(Xf::at(at), f);
        self.put_list(&d);
    }

    /// Draw an already-positioned list.
    pub fn put_list(&mut self, d: &DrawList) {
        let pl = rasterize(d, &self.cfg, None);
        self.canvas.draw(&pl, V2::ZERO, &self.palette, &self.style);
    }

    pub fn save(&self, path: &Path) {
        proof_tools::write_png(path, self.canvas.width, self.canvas.height, &self.canvas.to_rgba8()).unwrap();
        println!("wrote {}", path.display());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = PathBuf::from("../out/artboard");
    let mut scale = 2.0f32;
    let mut sheets: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scale" => {
                scale = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(scale);
                i += 1;
            }
            "--sheet" => {
                if let Some(s) = args.get(i + 1) {
                    sheets.push(s.clone());
                }
                i += 1;
            }
            a if !a.starts_with("--") => out = PathBuf::from(a),
            _ => {}
        }
        i += 1;
    }
    let all = sheets.is_empty();
    let want = |name: &str| all || sheets.iter().any(|s| s == name);
    if want("riso") {
        riso::render(&out, scale);
    }
    if want("cast") {
        cast::render(&out, scale);
    }
    if want("bakery") {
        bakery::render(&out, scale);
    }
    if want("scenes") {
        scenes::render(&out, scale);
    }
}
