//! Tile screenshots into one contact sheet for quick review.
//!
//! Usage: `cargo run -p proof_tools --bin contact -- out.png <cols> <scale> a.png b.png ...`

use std::fs::File;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = &args[1];
    let cols: u32 = args[2].parse().unwrap();
    let scale: f32 = args[3].parse().unwrap();
    let imgs: Vec<_> = args[4..].iter().filter_map(|p| load(p)).collect();
    if imgs.is_empty() {
        eprintln!("no images");
        return;
    }
    let (w0, h0) = (imgs[0].0, imgs[0].1);
    let (tw, th) = ((w0 as f32 * scale) as u32, (h0 as f32 * scale) as u32);
    let rows = (imgs.len() as u32).div_ceil(cols);
    let (w, h) = (tw * cols + 8 * (cols + 1), th * rows + 8 * (rows + 1));
    let mut sheet = vec![40u8; (w * h * 4) as usize];
    for (i, (iw, ih, px)) in imgs.iter().enumerate() {
        let (cx, cy) = (8 + (i as u32 % cols) * (tw + 8), 8 + (i as u32 / cols) * (th + 8));
        for y in 0..th {
            for x in 0..tw {
                let sx = ((x as f32 / scale) as u32).min(iw - 1);
                let sy = ((y as f32 / scale) as u32).min(ih - 1);
                let s = ((sy * iw + sx) * 4) as usize;
                let d = (((cy + y) * w + cx + x) * 4) as usize;
                sheet[d..d + 4].copy_from_slice(&px[s..s + 4]);
            }
        }
    }
    proof_tools::write_png(std::path::Path::new(out), w, h, &sheet).unwrap();
    println!("wrote {out} ({} images)", imgs.len());
}
