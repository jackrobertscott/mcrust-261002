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
