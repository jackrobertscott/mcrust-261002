//! All game art is generated in code at startup - there are no external files.
pub mod blocks;
pub mod gui;
pub mod items;
pub mod mobs;

use crate::image::Image;

/// Dump every generated asset as PNG into `dir` (for previewing the art).
pub fn dump_all(dir: &str) {
    let _ = std::fs::create_dir_all(dir);
    let groups: [(&str, &[&str], fn(&str) -> Option<Image>); 4] = [
        ("blocks", blocks::NAMES, blocks::get),
        ("items", items::NAMES, items::get),
        ("mobs", mobs::NAMES, mobs::get),
        ("gui", gui::NAMES, gui::get),
    ];
    for (g, names, f) in groups {
        let _ = std::fs::create_dir_all(format!("{dir}/{g}"));
        for n in names {
            match f(n) {
                Some(img) => {
                    let _ = img.save_png(&format!("{dir}/{g}/{n}.png"));
                }
                None => eprintln!("missing asset {g}/{n}"),
            }
        }
    }
}

/// Build a contact sheet of all 16x16 block and item textures (scaled x4).
pub fn contact_sheet(path: &str) {
    let mut all: Vec<Image> = Vec::new();
    for n in blocks::NAMES {
        if let Some(i) = blocks::get(n) { all.push(i); }
    }
    for n in items::NAMES {
        if let Some(i) = items::get(n) { all.push(i); }
    }
    let cols = 16;
    let rows = all.len().div_ceil(cols);
    let mut sheet = Image::filled(cols * 18, rows * 18, [60, 60, 70, 255]);
    for (k, img) in all.iter().enumerate() {
        sheet.blend(img, (k % cols * 18 + 1) as i32, (k / cols * 18 + 1) as i32);
    }
    let _ = sheet.scaled(3).save_png(path);
}

/// Render an isometric grass block (used as the application icon).
pub fn grass_block_icon(size: usize) -> Image {
    let top = blocks::get("grass_top").unwrap_or_else(|| Image::filled(16, 16, [100, 160, 80, 255]));
    let dirt = blocks::get("dirt").unwrap_or_else(|| Image::filled(16, 16, [130, 90, 60, 255]));
    let over = blocks::get("grass_side_overlay").unwrap_or_else(|| Image::new(16, 16));
    let tint = [0x91 as f32 / 255.0, 0xBD as f32 / 255.0, 0x59 as f32 / 255.0];
    let mut img = Image::new(size, size);
    let s = size as f32;
    let cx = s / 2.0;
    let half_w = s * 0.43;
    let rh = half_w * 0.5; // rhombus half-height
    let top_y = s * 0.06;
    let side_h = s * 0.5;
    let sample = |t: &Image, u: f32, v: f32| t.get(((u * 16.0) as usize).min(15), ((v * 16.0) as usize).min(15));
    for py in 0..size {
        for px in 0..size {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            // top face: inverse of the rhombus mapping
            let dx = (x - cx) / half_w;
            let dy = (y - (top_y + rh)) / rh;
            let u = (dx + dy + 1.0) / 2.0;
            let v = (dy - dx + 1.0) / 2.0;
            let mut col: Option<[f32; 4]> = None;
            if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                let c = sample(&top, u, v);
                col = Some([c[0] as f32 / 255.0 * tint[0], c[1] as f32 / 255.0 * tint[1], c[2] as f32 / 255.0 * tint[2], 1.0]);
            } else {
                // left / right faces
                let left = x < cx;
                let fx = if left { (x - (cx - half_w)) / half_w } else { (x - cx) / half_w };
                if (0.0..1.0).contains(&fx) {
                    let edge_y = if left { top_y + rh + fx * rh } else { top_y + 2.0 * rh - fx * rh };
                    let fy = (y - edge_y) / side_h;
                    if (0.0..1.0).contains(&fy) {
                        let d = sample(&dirt, fx, fy);
                        let o = sample(&over, fx, fy);
                        let a = o[3] as f32 / 255.0;
                        let mut c = [0.0f32; 4];
                        for k in 0..3 {
                            let base = d[k] as f32 / 255.0;
                            let ov = o[k] as f32 / 255.0 * tint[k];
                            c[k] = base * (1.0 - a) + ov * a;
                        }
                        let shade = if left { 0.8 } else { 0.6 };
                        col = Some([c[0] * shade, c[1] * shade, c[2] * shade, 1.0]);
                    }
                }
            }
            if let Some(c) = col {
                img.set(px, py, [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
            }
        }
    }
    img
}
