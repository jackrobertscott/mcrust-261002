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
