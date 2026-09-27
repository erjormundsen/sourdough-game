//! Press tools: raster timing and image inspection (owned by the press area).
//!
//! Usage:
//! * `press bench [runs]` — time rasterising a full backdrop at device scale 1.5 and a loaf.
//! * `press zoom in.png out.png x y w h [k]` — crop a region and enlarge it k× (nearest).

use proof_core::art::bread::{LoafView, loaf_top};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::scenes::{Backdrop, backdrop, counter_front, counter_top};
use proof_core::art::{Expr, face};
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, circle, v2};
use proof_core::ink::{Edition, Ink};
use proof_raster::{Canvas, CompositeStyle, RasterConfig, rasterize};
use std::fs::File;
use std::time::Instant;

fn load(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let dec = png::Decoder::new(std::io::BufReader::new(File::open(path).ok()?));
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let ch = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        _ => return None,
    };
    let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
    for px in buf[..info.buffer_size()].chunks(ch) {
        rgba.extend_from_slice(&[px[0], px[1], px[2], if ch == 4 { px[3] } else { 255 }]);
    }
    Some((info.width, info.height, rgba))
}

fn time_ms(runs: u32, mut f: impl FnMut()) -> (f64, f64) {
    let mut best = f64::MAX;
    let mut total = 0.0;
    for _ in 0..runs {
        let t = Instant::now();
        f();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        best = best.min(ms);
        total += ms;
    }
    (best, total / runs as f64)
}

/// Raster budgets at device scale 1.5 (release build): a full 720×1280 backdrop ≤ 150 ms,
/// a loaf ≤ 20 ms. Screen maps and press textures are built once per scale and cached.
fn bench(runs: u32) {
    let cfg = RasterConfig { scale: 1.5, ..RasterConfig::default() };
    let t = Instant::now();
    std::hint::black_box(proof_raster::press_textures(1.5));
    println!("press textures (once per device scale)   {:7.2} ms", t.elapsed().as_secs_f64() * 1000.0);
    let t = Instant::now();
    let mut warm = DrawList::new();
    warm.fill(Ink::Pink, 0.3, &circle(V2::ZERO, 10.0));
    warm.ht(Ink::Pink, 0.3, &circle(V2::ZERO, 10.0));
    std::hint::black_box(rasterize(&warm, &cfg, None));
    println!("screen maps (once per device scale)      {:7.2} ms", t.elapsed().as_secs_f64() * 1000.0);
    let mut worst_backdrop = 0.0f64;
    for (name, bd) in
        [("bakehouse", Backdrop::Bakehouse), ("shopfront", Backdrop::Shopfront), ("pantry", Backdrop::Pantry)]
    {
        let mut d = DrawList::new();
        backdrop(&mut d, bd, 1280.0);
        let (best, avg) = time_ms(runs, || {
            let pl = rasterize(&d, &cfg, Some(proof_core::geom::rect(0.0, 0.0, 720.0, 1280.0)));
            std::hint::black_box(pl);
        });
        worst_backdrop = worst_backdrop.max(best);
        println!("backdrop {name:10} {:5} cmds  best {best:7.2} ms  avg {avg:7.2} ms", d.cmds.len());
        let mut f = DrawList::new();
        counter_front(&mut f, bd, 1280.0);
        let (best, avg) = time_ms(runs, || {
            std::hint::black_box(rasterize(&f, &cfg, None));
        });
        println!("counter  {name:10} {:5} cmds  best {best:7.2} ms  avg {avg:7.2} ms", f.cmds.len());
    }
    let mut loaf80 = 0.0;
    for r in [80.0, 150.0] {
        let mut d = DrawList::new();
        loaf_top(&mut d, &LoafView { r, ..LoafView::default() });
        let (best, avg) = time_ms(runs, || {
            std::hint::black_box(rasterize(&d, &cfg, None));
        });
        if r == 80.0 {
            loaf80 = best;
        }
        println!("loaf r={r:<5}          {:5} cmds  best {best:7.2} ms  avg {avg:7.2} ms", d.cmds.len());
    }
    let ok = |v: f64, b: f64| if v <= b { "ok" } else { "OVER BUDGET" };
    println!(
        "budget: backdrop {worst_backdrop:.1}/150 ms {}, loaf {loaf80:.1}/20 ms {}",
        ok(worst_backdrop, 150.0),
        ok(loaf80, 20.0)
    );
}

fn zoom(args: &[String]) {
    let [src, dst, x, y, w, h, rest @ ..] = args else {
        eprintln!("usage: press zoom in.png out.png x y w h [k]");
        return;
    };
    let p = |s: &String| s.parse::<u32>().unwrap_or(0);
    let (x, y, w, h) = (p(x), p(y), p(w), p(h));
    let k = rest.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(4).max(1);
    let Some((iw, ih, px)) = load(src) else {
        eprintln!("cannot read {src}");
        return;
    };
    let (w, h) = (w.min(iw.saturating_sub(x)), h.min(ih.saturating_sub(y)));
    let mut out = vec![0u8; (w * k * h * k * 4) as usize];
    for oy in 0..h * k {
        for ox in 0..w * k {
            let s = (((y + oy / k) * iw + x + ox / k) * 4) as usize;
            let d = ((oy * w * k + ox) * 4) as usize;
            out[d..d + 4].copy_from_slice(&px[s..s + 4]);
        }
    }
    proof_tools::write_png(std::path::Path::new(dst), w * k, h * k, &out).unwrap();
    println!("wrote {dst} ({}x{})", w * k, h * k);
}

/// Print whole screens (backdrop, counter, a loaf, a jar, a face) on the CPU press at a device
/// scale, like the game does on a phone. `press scene out.png [scale] [grain] [height]`;
/// the backdrop alone at the tour's size (e.g. `1.5 0.85 1560`) matches the game pixel for pixel.
fn scene(args: &[String]) {
    let out = args.first().cloned().unwrap_or_else(|| "../out/after/press_scene.png".into());
    let scale: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.5);
    let grain: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(RasterConfig::default().grain);
    let height: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1280.0);
    let only_backdrop = args.get(3).is_some();
    let cfg = RasterConfig { scale, grain, ..RasterConfig::default() };
    let style = CompositeStyle::default();
    let mut sheets = Vec::new();
    for (bd, ed) in [
        (Backdrop::Bakehouse, Edition::Dawn),
        (Backdrop::Shopfront, Edition::Daylight),
        (Backdrop::Pantry, Edition::Dusk),
    ] {
        let pal = ed.palette();
        let mut c = Canvas::paper_styled(720.0, height, scale, &pal, &style);
        let mut put = |d: &DrawList| {
            let pl = rasterize(d, &cfg, None);
            c.draw(&pl, V2::ZERO, &pal, &style);
        };
        let mut d = DrawList::new();
        backdrop(&mut d, bd, height);
        put(&d);
        if !only_backdrop {
            let top = counter_top(bd, height);
            let mut d = DrawList::new();
            d.with(Xf::at(v2(360.0, top - 150.0)), |d| {
                d.fill(Ink::Yellow, 0.42, &circle(V2::ZERO, 110.0));
                face(d, v2(0.0, -6.0), 128.0, Expr::Happy, V2::ZERO);
            });
            put(&d);
            let mut d = DrawList::new();
            counter_front(&mut d, bd, height);
            put(&d);
            let mut d = DrawList::new();
            d.with(Xf::at(v2(170.0, top + 70.0)), |d| {
                loaf_top(d, &LoafView { r: 80.0, ..LoafView::default() })
            });
            d.with(Xf::at(v2(560.0, top + 20.0)).scaled(0.62), |d| jar(d, &JarView::default()));
            put(&d);
        }
        sheets.push(c);
    }
    let (w, h) = (sheets[0].width, sheets[0].height);
    let mut rgba = Vec::with_capacity((w * 3 * h * 4) as usize);
    for y in 0..h {
        for c in &sheets {
            let row = &c.rgb[(y * w) as usize..((y + 1) * w) as usize];
            for p in row {
                let [r, g, b] = p.to_u8();
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    proof_tools::write_png(std::path::Path::new(&out), w * 3, h, &rgba).unwrap();
    println!("wrote {out}");
}

/// Box-filter an image down by an integer factor: `press shrink in.png out.png k`. A phone
/// screenshot shrunk 2× on a desktop monitor is roughly what the eye gets from the phone.
fn shrink(args: &[String]) {
    let [src, dst, k, ..] = args else {
        eprintln!("usage: press shrink in.png out.png k");
        return;
    };
    let k: u32 = k.parse().unwrap_or(2).max(1);
    let Some((w, h, px)) = load(src) else {
        eprintln!("cannot read {src}");
        return;
    };
    let (ow, oh) = (w / k, h / k);
    let mut out = vec![255u8; (ow * oh * 4) as usize];
    for y in 0..oh {
        for x in 0..ow {
            let mut acc = [0u32; 3];
            for dy in 0..k {
                for dx in 0..k {
                    let i = (((y * k + dy) * w + x * k + dx) * 4) as usize;
                    for c in 0..3 {
                        acc[c] += px[i + c] as u32;
                    }
                }
            }
            let o = ((y * ow + x) * 4) as usize;
            for c in 0..3 {
                out[o + c] = (acc[c] / (k * k)) as u8;
            }
        }
    }
    proof_tools::write_png(std::path::Path::new(dst), ow, oh, &out).unwrap();
    println!("wrote {dst} ({ow}x{oh})");
}

/// Compare the same region of two images (e.g. a game screenshot and `press scene` output):
/// `press diff a.png ax ay b.png bx by w h` prints the mean absolute difference per channel.
fn diff(args: &[String]) {
    let [a, ax, ay, b, bx, by, w, h, ..] = args else {
        eprintln!("usage: press diff a.png ax ay b.png bx by w h");
        return;
    };
    let p = |s: &String| s.parse::<u32>().unwrap_or(0);
    let (Some((aw, _, apx)), Some((bw, _, bpx))) = (load(a), load(b)) else {
        eprintln!("cannot read inputs");
        return;
    };
    let (ax, ay, bx, by, w, h) = (p(ax), p(ay), p(bx), p(by), p(w), p(h));
    let mut sum = [0u64; 3];
    let mut mean = [[0u64; 3]; 2];
    for y in 0..h {
        for x in 0..w {
            let ia = (((ay + y) * aw + ax + x) * 4) as usize;
            let ib = (((by + y) * bw + bx + x) * 4) as usize;
            for c in 0..3 {
                sum[c] += (apx[ia + c] as i32 - bpx[ib + c] as i32).unsigned_abs() as u64;
                mean[0][c] += apx[ia + c] as u64;
                mean[1][c] += bpx[ib + c] as u64;
            }
        }
    }
    let n = (w * h) as f64;
    println!(
        "mean |a-b| rgb = {:.2} {:.2} {:.2}; mean a = {:.1} {:.1} {:.1}; mean b = {:.1} {:.1} {:.1}",
        sum[0] as f64 / n,
        sum[1] as f64 / n,
        sum[2] as f64 / n,
        mean[0][0] as f64 / n,
        mean[0][1] as f64 / n,
        mean[0][2] as f64 / n,
        mean[1][0] as f64 / n,
        mean[1][1] as f64 / n,
        mean[1][2] as f64 / n
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("bench") => bench(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5)),
        Some("zoom") => zoom(&args[1..]),
        Some("scene") => scene(&args[1..]),
        Some("diff") => diff(&args[1..]),
        Some("shrink") => shrink(&args[1..]),
        _ => eprintln!(
            "usage: press bench [runs] | press zoom in.png out.png x y w h [k] | \
             press scene out.png [scale] [grain] [height] | press diff a.png ax ay b.png bx by w h | \
             press shrink in.png out.png k"
        ),
    }
}
