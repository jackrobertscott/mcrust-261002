//! Block textures: 16x16 pixel art, hand-authored grids plus seeded noise.
//!
//! Tinted textures (grass_top, grass_side_overlay, *_leaves, short_grass, fern)
//! are grayscale; the engine multiplies them by a biome colour.
use crate::image::{rgb, Image, Rng};

type C = [u8; 4];

pub const NAMES: &[&str] = &[
    "stone", "dirt", "grass_top", "grass_side_overlay", "grass_side_snowed", "snow", "ice",
    "sand", "sandstone_top", "sandstone_side", "sandstone_bottom", "gravel", "clay", "bedrock",
    "cobblestone", "mossy_cobblestone", "bricks", "obsidian",
    "oak_log", "oak_log_top", "oak_planks", "oak_leaves",
    "birch_log", "birch_log_top", "birch_planks", "birch_leaves",
    "spruce_log", "spruce_log_top", "spruce_planks", "spruce_leaves",
    "jungle_log", "jungle_log_top", "jungle_planks", "jungle_leaves",
    "acacia_log", "acacia_log_top", "acacia_planks", "acacia_leaves",
    "coal_ore", "iron_ore", "gold_ore", "diamond_ore", "redstone_ore", "lapis_ore",
    "coal_block", "iron_block", "gold_block", "diamond_block", "glass",
    "crafting_table_top", "crafting_table_side", "crafting_table_front",
    "furnace_front", "furnace_front_on", "furnace_side", "furnace_top",
    "chest_front", "chest_side", "chest_top", "torch", "water_still", "lava_still",
    "farmland", "farmland_moist",
    "wheat_stage0", "wheat_stage1", "wheat_stage2", "wheat_stage3",
    "wheat_stage4", "wheat_stage5", "wheat_stage6", "wheat_stage7",
    "short_grass", "fern", "dandelion", "poppy", "blue_orchid", "dead_bush", "sugar_cane",
    "oak_sapling", "birch_sapling", "spruce_sapling", "jungle_sapling", "acacia_sapling",
    "cactus_side", "cactus_top", "cactus_bottom", "white_wool", "bookshelf",
    "pumpkin_side", "pumpkin_top", "tnt_side", "tnt_top", "tnt_bottom",
    "destroy_stage_0", "destroy_stage_1", "destroy_stage_2", "destroy_stage_3",
    "destroy_stage_4", "destroy_stage_5", "destroy_stage_6", "destroy_stage_7",
    "destroy_stage_8", "destroy_stage_9",
    "oak_door_top", "oak_door_bottom", "ladder",
    "bed_head_top", "bed_foot_top", "bed_head_side", "bed_foot_side",
    "bed_head_end", "bed_foot_end",
];

pub fn get(name: &str) -> Option<Image> {
    let img = match name {
        "stone" => stone(),
        "dirt" => dirt(21),
        "grass_top" => grass_top(31),
        "grass_side_overlay" => grass_side_overlay(),
        "grass_side_snowed" => grass_side_snowed(),
        "snow" => snow(41),
        "ice" => ice(),
        "sand" => sand(),
        "sandstone_top" => sandstone_top(),
        "sandstone_side" => sandstone_side(),
        "sandstone_bottom" => sandstone_bottom(),
        "gravel" => gravel(),
        "clay" => clay(),
        "bedrock" => bedrock(),
        "cobblestone" => cobblestone(91),
        "mossy_cobblestone" => mossy_cobblestone(),
        "bricks" => bricks(),
        "obsidian" => obsidian(),
        "coal_ore" => ore(&ORE_A, [0x1A1A1A, 0x2E2E2E, 0x474747], false),
        "iron_ore" => ore(&ORE_B, [0xAF8E77, 0xD8AF93, 0xEDD2BE], false),
        "gold_ore" => ore(&ORE_B, [0xD49A1A, 0xFCEE4B, 0xFFFFB5], true),
        "diamond_ore" => ore(&ORE_C, [0x1BA6A0, 0x5DECF5, 0xD5FFF6], false),
        "redstone_ore" => ore(&ORE_C, [0x8A0000, 0xFF0000, 0xFF8080], true),
        "lapis_ore" => ore(&ORE_A, [0x0F2D78, 0x1D47A6, 0x4A78D8], true),
        "coal_block" => coal_block(),
        "iron_block" => metal_block(0xDCDCDC, 0xFFFFFF, 0xC4C4C4, 0x9A9A9A, 501),
        "gold_block" => metal_block(0xF9D849, 0xFFFFA8, 0xE9B820, 0xB98A10, 502),
        "diamond_block" => diamond_block(),
        "glass" => glass(),
        "crafting_table_top" => crafting_table_top(),
        "crafting_table_side" => crafting_table_side(),
        "crafting_table_front" => crafting_table_front(),
        "furnace_front" => furnace_front(false),
        "furnace_front_on" => furnace_front(true),
        "furnace_side" => furnace_side(),
        "furnace_top" => furnace_top(),
        "chest_front" => chest_front(),
        "chest_side" => chest_side(),
        "chest_top" => chest_top(),
        "torch" => torch(),
        "water_still" => water(),
        "lava_still" => lava(),
        "farmland" => farmland(false),
        "farmland_moist" => farmland(true),
        "short_grass" => short_grass(),
        "fern" => fern(),
        "dandelion" => dandelion(),
        "poppy" => poppy(),
        "blue_orchid" => blue_orchid(),
        "dead_bush" => dead_bush(),
        "sugar_cane" => sugar_cane(),
        "oak_sapling" => oak_sapling(),
        "birch_sapling" => birch_sapling(),
        "spruce_sapling" => spruce_sapling(),
        "jungle_sapling" => jungle_sapling(),
        "acacia_sapling" => acacia_sapling(),
        "cactus_side" => cactus_side(),
        "cactus_top" => cactus_top(false),
        "cactus_bottom" => cactus_top(true),
        "white_wool" => white_wool(),
        "bookshelf" => bookshelf(),
        "pumpkin_side" => pumpkin_side(),
        "pumpkin_top" => pumpkin_top(),
        "tnt_side" => tnt_side(),
        "tnt_top" => tnt_end(true),
        "tnt_bottom" => tnt_end(false),
        "oak_door_top" => oak_door_top(),
        "oak_door_bottom" => oak_door_bottom(),
        "ladder" => ladder(),
        "bed_head_top" => bed_head_top(),
        "bed_foot_top" => bed_foot_top(),
        "bed_head_side" => bed_side(613),
        "bed_foot_side" => bed_side(614),
        "bed_head_end" => bed_head_end(),
        "bed_foot_end" => bed_foot_end(),
        _ => {
            if let Some(s) = name.strip_prefix("wheat_stage") {
                let s: u32 = s.parse().ok()?;
                if s > 7 {
                    return None;
                }
                return Some(wheat(s));
            }
            if let Some(s) = name.strip_prefix("destroy_stage_") {
                let s: usize = s.parse().ok()?;
                if s > 9 {
                    return None;
                }
                return Some(destroy(s));
            }
            return wood(name);
        }
    };
    Some(img)
}

// ---------------------------------------------------------------------------
// helpers

fn rgba(c: u32, a: u8) -> C {
    let mut x = rgb(c);
    x[3] = a;
    x
}

fn gray(v: u8) -> C {
    [v, v, v, 255]
}

fn shade(c: C, f: f32) -> C {
    let m = |v: u8| (v as f32 * f).round().clamp(0.0, 255.0) as u8;
    [m(c[0]), m(c[1]), m(c[2]), c[3]]
}

fn lerp(a: u32, b: u32, t: f32) -> C {
    let (a, b) = (rgb(a), rgb(b));
    let l = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [l(0), l(1), l(2), 255]
}

fn white(seed: u64) -> Vec<f32> {
    let mut r = Rng::new(seed);
    for _ in 0..4 {
        r.next_u32();
    }
    (0..256).map(|_| r.f()).collect()
}

/// Tileable [1 2 1] blur, `hx` horizontal passes and `vy` vertical passes.
fn blur(v: &[f32], hx: usize, vy: usize) -> Vec<f32> {
    let mut a = v.to_vec();
    for _ in 0..hx {
        let mut b = vec![0.0; 256];
        for y in 0..16 {
            for x in 0..16 {
                b[y * 16 + x] =
                    (a[y * 16 + (x + 15) % 16] + 2.0 * a[y * 16 + x] + a[y * 16 + (x + 1) % 16]) / 4.0;
            }
        }
        a = b;
    }
    for _ in 0..vy {
        let mut b = vec![0.0; 256];
        for y in 0..16 {
            for x in 0..16 {
                b[y * 16 + x] =
                    (a[(y + 15) % 16 * 16 + x] + 2.0 * a[y * 16 + x] + a[(y + 1) % 16 * 16 + x]) / 4.0;
            }
        }
        a = b;
    }
    a
}

/// Rank-equalise so values are uniformly distributed in (0,1).
fn equalize(v: &[f32]) -> Vec<f32> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].partial_cmp(&v[b]).unwrap());
    let mut o = vec![0.0; v.len()];
    for (rank, &i) in idx.iter().enumerate() {
        o[i] = (rank as f32 + 0.5) / v.len() as f32;
    }
    o
}

/// Tileable noise: smooth (blurred) component mixed with `detail` of per-pixel noise.
fn noise(seed: u64, hx: usize, vy: usize, detail: f32) -> Vec<f32> {
    let s = equalize(&blur(&white(seed), hx, vy));
    let w = white(seed ^ 0x5DEECE66D);
    let m: Vec<f32> = s.iter().zip(&w).map(|(a, b)| a * (1.0 - detail) + b * detail).collect();
    equalize(&m)
}

/// Weighted palette lookup (palette ordered dark -> light).
fn wpick(t: f32, pal: &[(u32, u32)]) -> u32 {
    let total: u32 = pal.iter().map(|p| p.1).sum();
    let t = t * total as f32;
    let mut acc = 0.0;
    for &(c, w) in pal {
        acc += w as f32;
        if t < acc {
            return c;
        }
    }
    pal[pal.len() - 1].0
}

fn wquant(v: &[f32], pal: &[(u32, u32)]) -> Image {
    let mut img = Image::new(16, 16);
    for i in 0..256 {
        img.data[i] = rgb(wpick(v[i], pal));
    }
    img
}

fn grid(rows: &[&str], pal: &[(char, C)]) -> Image {
    let mut img = Image::new(16, 16);
    paint(&mut img, rows, pal);
    img
}

/// Paint characters found in `pal`; other characters leave the image untouched.
fn paint(img: &mut Image, rows: &[&str], pal: &[(char, C)]) {
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some(&(_, c)) = pal.iter().find(|p| p.0 == ch) {
                img.set(x, y, c);
            }
        }
    }
}

/// Jittered-grid tileable voronoi: returns cell id per pixel.
fn voronoi(seed: u64, gx: usize, gy: usize, jitter: f32) -> Vec<usize> {
    let mut r = Rng::new(seed);
    let (cw, ch) = (16.0 / gx as f32, 16.0 / gy as f32);
    let mut pts = Vec::new();
    for j in 0..gy {
        for i in 0..gx {
            let ox = 0.5 + (r.f() - 0.5) * jitter;
            let oy = 0.5 + (r.f() - 0.5) * jitter;
            let sh = if j % 2 == 1 { 0.5 } else { 0.0 };
            pts.push(((i as f32 + ox + sh) * cw % 16.0, (j as f32 + oy) * ch));
        }
    }
    let mut ids = vec![0; 256];
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut best = (f32::MAX, 0);
            for (i, &(qx, qy)) in pts.iter().enumerate() {
                let mut dx = (px - qx).abs();
                if dx > 8.0 {
                    dx = 16.0 - dx;
                }
                let mut dy = (py - qy).abs();
                if dy > 8.0 {
                    dy = 16.0 - dy;
                }
                let d = dx * dx + dy * dy + 0.6 * dx.max(dy) * dx.max(dy);
                if d < best.0 {
                    best = (d, i);
                }
            }
            ids[y * 16 + x] = best.1;
        }
    }
    ids
}

// ---------------------------------------------------------------------------
// natural blocks

fn stone() -> Image {
    wquant(
        &noise(11, 2, 1, 0.45),
        &[(0x666666, 2), (0x6F6F6F, 3), (0x777777, 4), (0x7F7F7F, 6), (0x888888, 3), (0x949494, 1)],
    )
}

fn dirt(seed: u64) -> Image {
    wquant(
        &noise(seed, 1, 1, 0.7),
        &[(0x593D29, 2), (0x6C4A32, 3), (0x79553A, 5), (0x866043, 6), (0x966C4A, 3), (0xB9855C, 1)],
    )
}

const GRASS_PAL: [(u32, u32); 6] =
    [(0x7A7A7A, 1), (0x8B8B8B, 3), (0x999999, 5), (0xA6A6A6, 5), (0xB3B3B3, 3), (0xC4C4C4, 1)];

fn grass_top(seed: u64) -> Image {
    wquant(&noise(seed, 1, 1, 0.65), &GRASS_PAL)
}

fn grass_side_overlay() -> Image {
    let g = grass_top(33);
    let depth = [4, 3, 3, 4, 5, 4, 3, 3, 4, 6, 4, 3, 3, 4, 3, 5];
    let mut img = Image::new(16, 16);
    for x in 0..16 {
        for y in 0..depth[x] {
            let mut c = g.get(x, y);
            if y == depth[x] - 1 {
                c = shade(c, 0.82);
            }
            img.set(x, y, c);
        }
    }
    img
}

fn snow(seed: u64) -> Image {
    wquant(&noise(seed, 1, 1, 0.5), &[(0xD9E6E6, 1), (0xE6F0F0, 3), (0xF2FAFA, 5), (0xFFFFFF, 6)])
}

fn grass_side_snowed() -> Image {
    let mut img = dirt(22);
    let s = snow(43);
    let depth = [3, 3, 4, 3, 2, 3, 3, 4, 5, 3, 3, 2, 3, 4, 3, 3];
    for x in 0..16 {
        for y in 0..depth[x] {
            let mut c = s.get(x, y);
            if y == depth[x] - 1 {
                c = shade(c, 0.9);
            }
            img.set(x, y, c);
        }
    }
    img
}

fn ice() -> Image {
    let n = noise(45, 3, 1, 0.35);
    let mut img = wquant(&n, &[(0x7096E0, 2), (0x7DA6F0, 5), (0x8DB5F8, 4), (0xA8C8FF, 2), (0xC8DCFF, 1)]);
    for p in img.data.iter_mut() {
        p[3] = 185;
    }
    // a few bright streaks
    for &(x, y) in &[(3, 2), (4, 2), (5, 2), (9, 6), (10, 6), (11, 6), (12, 6), (2, 11), (3, 11), (11, 13), (12, 13)] {
        img.set(x, y, rgba(0xE4F0FF, 200));
    }
    img
}

fn sand() -> Image {
    wquant(
        &noise(51, 1, 1, 0.75),
        &[(0xC7BD86, 1), (0xD2C892, 3), (0xDBD3A0, 6), (0xE1D9AA, 3), (0xE9E2BA, 1)],
    )
}

fn sandstone_top() -> Image {
    wquant(&noise(52, 2, 2, 0.3), &[(0xD2C892, 1), (0xD9D09E, 4), (0xE0D8AA, 4), (0xE8E1B9, 1)])
}

fn sandstone_bottom() -> Image {
    wquant(
        &noise(53, 1, 1, 0.6),
        &[(0xC2B67F, 1), (0xCFC48F, 3), (0xD8CE9C, 5), (0xE0D8AA, 3), (0xE8E1B9, 1)],
    )
}

fn sandstone_side() -> Image {
    let mid = wquant(&noise(54, 3, 0, 0.3), &[(0xCFC48F, 2), (0xD8CE9C, 5), (0xDFD6A6, 3)]);
    let mut img = mid.clone();
    let mut r = Rng::new(55);
    for x in 0..16 {
        img.set(x, 0, rgb(if r.range(3) == 0 { 0xE0D8AA } else { 0xE8E1B9 }));
        img.set(x, 1, rgb(if r.range(3) == 0 { 0xDCD3A2 } else { 0xE2DAAC }));
        img.set(x, 2, rgb(if r.range(4) == 0 { 0xD8CE9C } else { 0xDDD5A5 }));
        img.set(x, 3, rgb(if r.range(4) == 0 { 0xB8AB78 } else { 0xC4B784 }));
        img.set(x, 11, rgb(if r.range(4) == 0 { 0xBFB27E } else { 0xC9BC88 }));
        for y in 12..15 {
            let c = if (x + (y - 12) * 2) % 4 < 2 { 0xD6CB97 } else { 0xCBBF8A };
            img.set(x, y, rgb(c));
        }
        img.set(x, 15, rgb(if r.range(3) == 0 { 0xB8AB78 } else { 0xC1B480 }));
    }
    img
}

fn gravel() -> Image {
    let ids = voronoi(61, 5, 5, 0.9);
    let mut r = Rng::new(62);
    let tones: Vec<u32> = (0..25)
        .map(|_| [0x5E5A5A, 0x6E6A69, 0x7F7C7B, 0x8E8887, 0x9A9393, 0xA9A3A2, 0x7A7470, 0x8C7F79][r.range(8) as usize])
        .collect();
    let px = white(63);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let id = ids[y * 16 + x];
            let rid = ids[y * 16 + (x + 1) % 16];
            let did = ids[(y + 1) % 16 * 16 + x];
            let lid = ids[y * 16 + (x + 15) % 16];
            let uid = ids[(y + 15) % 16 * 16 + x];
            let mut c = rgb(tones[id]);
            if rid != id || did != id {
                c = shade(c, 0.68);
            } else if lid != id || uid != id {
                c = shade(c, 1.12);
            }
            c = shade(c, 0.94 + px[y * 16 + x] * 0.12);
            img.set(x, y, c);
        }
    }
    img
}

fn clay() -> Image {
    wquant(&noise(71, 1, 1, 0.5), &[(0x9298A6, 1), (0x9BA1AE, 3), (0xA0A6B3, 5), (0xA8AEBB, 3), (0xB2B7C3, 1)])
}

fn bedrock() -> Image {
    wquant(
        &noise(81, 1, 1, 0.5),
        &[(0x1F1F1F, 1), (0x333333, 2), (0x494949, 3), (0x575757, 3), (0x6B6B6B, 2), (0x868686, 1), (0x9E9E9E, 1)],
    )
}

fn cobblestone(seed: u64) -> Image {
    let ids = voronoi(seed, 3, 3, 0.7);
    let mut r = Rng::new(seed + 1);
    let tones: Vec<u32> = (0..9).map(|_| [0x6E6E6E, 0x7A7A7A, 0x878787, 0x929292][r.range(4) as usize]).collect();
    let px = white(seed + 2);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let id = ids[y * 16 + x];
            let rid = ids[y * 16 + (x + 1) % 16];
            let did = ids[(y + 1) % 16 * 16 + x];
            let lid = ids[y * 16 + (x + 15) % 16];
            let uid = ids[(y + 15) % 16 * 16 + x];
            let d2 = ids[(y + 2) % 16 * 16 + x];
            let r2 = ids[y * 16 + (x + 2) % 16];
            let n = px[y * 16 + x];
            let c = if rid != id || did != id {
                gray(if n < 0.4 { 0x3C } else if n < 0.85 { 0x4D } else { 0x5A })
            } else if lid != id || uid != id {
                shade(rgb(tones[id]), 1.22 + n * 0.08)
            } else if d2 != id || r2 != id {
                shade(rgb(tones[id]), 0.86 + n * 0.06)
            } else {
                shade(rgb(tones[id]), 0.95 + n * 0.12)
            };
            img.set(x, y, c);
        }
    }
    img
}

fn mossy_cobblestone() -> Image {
    let mut img = cobblestone(91);
    let m = noise(95, 1, 1, 0.35);
    let tone = white(96);
    for i in 0..256 {
        if m[i] < 0.4 {
            let base = img.data[i];
            let dark = base[0] < 0x60;
            let c = if dark {
                0x3F5428
            } else {
                wpick(tone[i], &[(0x4A6A2B, 2), (0x587A31, 3), (0x678C3A, 2), (0x7A9E45, 1)])
            };
            img.data[i] = rgb(c);
        }
    }
    img
}

fn bricks() -> Image {
    let mut img = Image::new(16, 16);
    let mut r = Rng::new(97);
    let px = white(98);
    let mut tone = [[0u32; 3]; 4];
    for row in tone.iter_mut() {
        for t in row.iter_mut() {
            *t = [0x8F4E3C, 0x9A5745, 0xA45E4B, 0x96513F][r.range(4) as usize];
        }
    }
    for y in 0..16 {
        let br = y / 4;
        let ry = y % 4;
        let off = if br % 2 == 0 { 0 } else { 4 };
        for x in 0..16 {
            let bx = (x + off) % 16;
            let n = px[y * 16 + x];
            let c = if ry == 3 || bx % 8 == 7 {
                rgb(if n < 0.3 { 0x8A837C } else if n < 0.8 { 0x9C958D } else { 0xADA69E })
            } else {
                let b = rgb(tone[br][(bx / 8 + br) % 3]);
                let f = match ry {
                    0 => 1.12,
                    2 => 0.88,
                    _ => 1.0,
                };
                let f = if bx % 8 == 6 { f * 0.92 } else if bx % 8 == 0 { f * 1.05 } else { f };
                shade(b, f * (0.93 + n * 0.12))
            };
            img.set(x, y, c);
        }
    }
    img
}

fn obsidian() -> Image {
    wquant(
        &noise(101, 2, 1, 0.4),
        &[(0x060508, 2), (0x0F0B19, 5), (0x1B1229, 4), (0x2A1D3E, 2), (0x3B2754, 1), (0x4F3A6E, 1)],
    )
}

fn water() -> Image {
    let mut img = wquant(
        &noise(111, 3, 1, 0.3),
        &[(0x2C55C4, 2), (0x3260D0, 4), (0x386ADA, 4), (0x4677E2, 3), (0x5E8EEA, 1)],
    );
    for p in img.data.iter_mut() {
        p[3] = 180;
    }
    img
}

fn lava() -> Image {
    wquant(
        &noise(121, 1, 1, 0.3),
        &[(0xB8400A, 1), (0xD0530E, 3), (0xE36A14, 4), (0xF08C20, 3), (0xF9B23A, 2), (0xFFD76A, 1)],
    )
}

fn farmland(moist: bool) -> Image {
    let pal: &[(u32, u32)] = if moist {
        &[(0x2E1B0E, 1), (0x3B2414, 3), (0x4A2E1B, 5), (0x553620, 4), (0x623F26, 1)]
    } else {
        &[(0x5A3D27, 1), (0x6E4B30, 3), (0x7C5638, 5), (0x8A6040, 4), (0x9A6C4A, 1)]
    };
    let mut img = wquant(&noise(if moist { 131 } else { 132 }, 2, 0, 0.6), pal);
    let dark = rgb(if moist { 0x24150A } else { 0x4A3220 });
    for i in 0..16 {
        img.set(i, 0, dark);
        img.set(0, i, dark);
        img.set(i, 15, shade(dark, 0.85));
        img.set(15, i, shade(dark, 0.85));
    }
    for y in [4, 8, 12] {
        for x in 1..15 {
            let c = img.get(x, y);
            img.set(x, y, shade(c, 0.78));
            let c = img.get(x, y - 1);
            img.set(x, y - 1, shade(c, 1.08));
        }
    }
    img
}

// ---------------------------------------------------------------------------
// ores and mineral blocks

const ORE_A: [&str; 16] = [
    "................",
    "..do............",
    ".dooh.....dh....",
    ".doho....dooh...",
    "..dd.....dooo...",
    "..........dd....",
    "....dho.........",
    "...doohh........",
    "...dooo....do...",
    "....dd....dooh..",
    "..........dohd..",
    ".do........dd...",
    "dooh.....dho....",
    "dohd....doooh...",
    ".dd......ddo....",
    "................",
];

const ORE_B: [&str; 16] = [
    "................",
    "...oh...........",
    "..doho....oh....",
    "..ddo....doho...",
    "........ddoo....",
    ".....oh..dd.....",
    "....dooh........",
    "....ddoo....oh..",
    ".....dd....doho.",
    "...........ddo..",
    "..oh............",
    ".doho....oh.....",
    ".ddoo...doho....",
    "..dd....ddoo....",
    ".........dd.....",
    "................",
];

const ORE_C: [&str; 16] = [
    "................",
    "..hd........ho..",
    "..ooo......hoo..",
    "...od.......od..",
    "................",
    "......oh........",
    ".....hood.......",
    "......od........",
    "..........ho....",
    "..oh.....hood...",
    ".hoo......od....",
    "..od............",
    "......ho....oh..",
    ".....hood...od..",
    "......od........",
    "................",
];

fn ore(layout: &[&str; 16], cols: [u32; 3], flip: bool) -> Image {
    let mut img = stone();
    for (y, row) in layout.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let xx = if flip { 15 - x } else { x };
            let c = match ch {
                'd' => cols[0],
                'o' => cols[1],
                'h' => cols[2],
                _ => continue,
            };
            img.set(xx, y, rgb(c));
        }
    }
    img
}

fn coal_block() -> Image {
    wquant(
        &noise(141, 1, 1, 0.6),
        &[(0x0B0B0B, 3), (0x141414, 5), (0x1D1D1D, 4), (0x282828, 2), (0x363636, 1)],
    )
}

fn metal_block(base: u32, hi: u32, lo: u32, dark: u32, seed: u64) -> Image {
    let n = noise(seed, 3, 0, 0.4);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let t = n[y * 16 + x];
            let c = if x == 0 || y == 0 {
                rgb(hi)
            } else if x == 15 || y == 15 {
                rgb(dark)
            } else if x == 1 || y == 1 {
                lerp(base, hi, 0.5)
            } else if x == 14 || y == 14 {
                rgb(lo)
            } else if t < 0.18 {
                rgb(lo)
            } else if t > 0.88 {
                lerp(base, hi, 0.6)
            } else {
                rgb(base)
            };
            img.set(x, y, c);
        }
    }
    img
}

fn diamond_block() -> Image {
    let img = metal_block(0x62DCD5, 0xD6FFF8, 0x3EC1B9, 0x24918A, 503);
    img
}

fn glass() -> Image {
    let rows = [
        "wWwwwwWwwwwWwwwb",
        "W..............b",
        "w...x..........b",
        "w..x...........w",
        "w.x..x.........b",
        "w...x..........b",
        "W..x...........b",
        "w..............w",
        "w..............b",
        "w...........x..b",
        "W..........x...b",
        "w.........x....w",
        "w..............b",
        "w..............b",
        "w..............b",
        "bbbwbbbbbwbbbbbb",
    ];
    grid(
        &rows,
        &[('W', rgba(0xFFFFFF, 255)), ('w', rgba(0xD9EEF3, 255)), ('b', rgba(0xA6CCD5, 255)), ('x', rgba(0xFFFFFF, 170))],
    )
}

// ---------------------------------------------------------------------------
// wood

struct WoodPal {
    bark: [u32; 4],         // dark -> light
    planks: [u32; 5],       // seam, dark, mid, light, highlight
    ring: [u32; 3],         // dark, mid, light (log top)
    leaves: ([u8; 4], f32, u64, usize), // gray shades, hole fraction, seed, blur
}

fn wood_pal(kind: &str) -> Option<WoodPal> {
    Some(match kind {
        "oak" => WoodPal {
            bark: [0x3F3020, 0x523F25, 0x6B5232, 0x7E6644],
            planks: [0x6A5230, 0x8C6E42, 0x9C7F4E, 0xAD8A55, 0xB8945F],
            ring: [0x8E6E40, 0xA4834F, 0xB8945F],
            leaves: ([0x5A, 0x7C, 0x9A, 0xB8], 0.2, 201, 1),
        },
        "birch" => WoodPal {
            bark: [0x3A3934, 0x5F5E58, 0xD8D7D2, 0xE8E7E2],
            planks: [0x9C8A58, 0xB8A46C, 0xC4B07A, 0xCFBC85, 0xD7C792],
            ring: [0xB09F6C, 0xC4B07A, 0xD7C792],
            leaves: ([0x60, 0x80, 0x9E, 0xBA], 0.18, 202, 1),
        },
        "spruce" => WoodPal {
            bark: [0x24170A, 0x2F1F0D, 0x3B2812, 0x4C3519],
            planks: [0x4A3420, 0x654A2C, 0x735531, 0x7E5E37, 0x886540],
            ring: [0x5F4426, 0x735531, 0x8A6539],
            leaves: ([0x40, 0x5E, 0x7C, 0x98], 0.12, 203, 0),
        },
        "jungle" => WoodPal {
            bark: [0x3A2B0E, 0x4A3712, 0x574119, 0x6E5523],
            planks: [0x6E4A2C, 0x8C6440, 0xA0734D, 0xAD7F57, 0xB88A60],
            ring: [0x8C6440, 0xA0734D, 0xB88764],
            leaves: ([0x48, 0x6C, 0x90, 0xB4], 0.18, 204, 2),
        },
        "acacia" => WoodPal {
            bark: [0x47433D, 0x5B5650, 0x676157, 0x7A7468],
            planks: [0x7F4127, 0xA05330, 0xAD5D32, 0xBA6337, 0xC6703F],
            ring: [0x984F29, 0xAD5D32, 0xC26D3F],
            leaves: ([0x50, 0x70, 0x90, 0xAC], 0.26, 205, 0),
        },
        _ => return None,
    })
}

fn wood(name: &str) -> Option<Image> {
    let (kind, part) = name.split_once('_')?;
    let p = wood_pal(kind)?;
    let seed = kind.len() as u64 * 7919 + kind.as_bytes()[0] as u64;
    Some(match part {
        "log" => log_side(kind, &p, seed),
        "log_top" => log_top(kind, &p, seed),
        "planks" => planks(&p.planks, seed + 3),
        "leaves" => leaves(&p),
        _ => return None,
    })
}

fn log_side(kind: &str, p: &WoodPal, seed: u64) -> Image {
    if kind == "birch" {
        return birch_side(seed);
    }
    let mut img = Image::new(16, 16);
    let mut r = Rng::new(seed);
    // column base tones: crevices (dark) and ridges (light)
    let mut base = [0i32; 16];
    let mut b = 2i32;
    for x in 0..16 {
        let roll = r.range(10);
        b = if roll < 3 { 1 } else if roll < 4 { 0 } else if roll < 7 { 2 } else { 3 };
        base[x] = b;
    }
    let _ = b;
    for x in 0..16 {
        let mut y = 0;
        let start = r.range(16) as usize;
        while y < 16 {
            let len = 2 + r.range(6) as usize;
            let d = r.range(5) as i32;
            let off = if d == 0 { -1 } else if d == 4 { 1 } else { 0 };
            let idx = (base[x] + off).clamp(0, 3) as usize;
            for k in 0..len {
                if y + k < 16 {
                    img.set(x, (y + k + start) % 16, rgb(p.bark[idx]));
                }
            }
            y += len;
        }
    }
    img
}

fn birch_side(seed: u64) -> Image {
    let mut img = wquant(&noise(seed, 0, 3, 0.4), &[(0xC8C7C0, 2), (0xD8D7D2, 5), (0xE4E3DE, 4), (0xF0EFEA, 1)]);
    let mut r = Rng::new(seed + 1);
    for _ in 0..9 {
        let y = r.range(16) as usize;
        let x0 = r.range(16) as usize;
        let len = 2 + r.range(4) as usize;
        for k in 0..len {
            let x = (x0 + k) % 16;
            let c = if k == 0 || k == len - 1 { 0x5F5E58 } else { 0x2E2D29 };
            img.set(x, y, rgb(c));
            if r.range(3) == 0 {
                img.set(x, (y + 1) % 16, rgb(0x5F5E58));
            }
        }
    }
    img
}

fn log_top(kind: &str, p: &WoodPal, seed: u64) -> Image {
    let bark = log_side(kind, p, seed);
    let jit = white(seed + 5);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            if x == 0 || y == 0 || x == 15 || y == 15 {
                img.set(x, y, bark.get(x, (y + x) % 16));
                continue;
            }
            let dx = (x as f32 - 7.5).abs();
            let dy = (y as f32 - 7.5).abs();
            let d = dx.max(dy) * 0.75 + (dx * dx + dy * dy).sqrt() * 0.25 + jit[y * 16 + x] * 0.5;
            let ring = (d / 1.75) as i32;
            let c = if ring == 0 {
                p.ring[1]
            } else if ring % 2 == 1 {
                p.ring[2]
            } else {
                p.ring[0]
            };
            img.set(x, y, rgb(c));
        }
    }
    // inner bark shadow
    for i in 1..15 {
        for &(x, y) in &[(i, 1), (1, i), (i, 14), (14, i)] {
            let c = img.get(x, y);
            img.set(x, y, shade(c, 0.85));
        }
    }
    img
}

fn planks(pal: &[u32; 5], seed: u64) -> Image {
    let mut img = Image::new(16, 16);
    let mut r = Rng::new(seed);
    let seams = [3usize, 11, 7, 15];
    for b in 0..4 {
        for ry in 0..4 {
            let y = b * 4 + ry;
            if ry == 3 {
                for x in 0..16 {
                    img.set(x, y, rgb(pal[0]));
                }
                continue;
            }
            let mut x = 0;
            while x < 16 {
                let len = 2 + r.range(5) as usize;
                let roll = r.range(8);
                let idx = if roll < 2 { 1 } else if roll < 6 { 2 } else if roll < 7 { 3 } else { 4 };
                let idx = if ry == 0 && idx == 1 { 2 } else { idx };
                for k in 0..len {
                    if x + k < 16 {
                        img.set(x + k, y, rgb(pal[idx]));
                    }
                }
                x += len;
            }
        }
        let sx = seams[b];
        for ry in 0..3 {
            img.set(sx, b * 4 + ry, rgb(pal[0]));
            img.set((sx + 1) % 16, b * 4 + ry, rgb(pal[3]));
        }
    }
    img
}

fn leaves(p: &WoodPal) -> Image {
    let (shades, hole, seed, bl) = p.leaves;
    let n = noise(seed, bl, bl, 0.55);
    let h = noise(seed + 50, 1, 1, 0.45);
    let mut img = Image::new(16, 16);
    for i in 0..256 {
        if h[i] < hole {
            continue;
        }
        let v = shades[((n[i] * 4.0) as usize).min(3)];
        img.data[i] = gray(v);
    }
    // darken pixels just below holes and lighten those above (leaf depth)
    let src = img.clone();
    for y in 0..16 {
        for x in 0..16 {
            let c = src.get(x, y);
            if c[3] == 0 {
                continue;
            }
            let above = src.get(x, (y + 15) % 16)[3] == 0;
            let below = src.get(x, (y + 1) % 16)[3] == 0;
            if above {
                img.set(x, y, shade(c, 0.75));
            } else if below {
                img.set(x, y, shade(c, 1.12));
            }
        }
    }
    img
}

fn crafting_table_top() -> Image {
    let p = wood_pal("oak").unwrap();
    let mut img = planks(&p.planks, 301);
    let dark = rgb(0x4E3A20);
    let line = rgb(0x6A5230);
    let light = rgb(0xC29D62);
    for i in 0..16 {
        img.set(i, 0, dark);
        img.set(0, i, dark);
        img.set(i, 15, dark);
        img.set(15, i, dark);
        if i > 0 && i < 15 {
            img.set(i, 1, light);
            img.set(1, i, light);
            img.set(i, 14, line);
            img.set(14, i, line);
        }
    }
    for &k in &[5usize, 10] {
        for i in 2..14 {
            img.set(k, i, line);
            img.set(i, k, line);
            img.set(k + 1, i, shade(img.get(k + 1, i), 1.1));
        }
    }
    img
}

fn table_band(img: &mut Image) {
    for x in 0..16 {
        img.set(x, 0, rgb(if x % 5 == 2 { 0xA4834F } else { 0xB8945F }));
        img.set(x, 1, rgb(if x % 7 == 3 { 0x8C6E42 } else { 0x9C7F4E }));
        img.set(x, 2, rgb(0x4E3A20));
    }
    for y in 3..16 {
        img.set(0, y, rgb(0x5A4428));
        img.set(15, y, rgb(0x5A4428));
    }
}

fn crafting_table_front() -> Image {
    let p = wood_pal("oak").unwrap();
    let mut img = planks(&p.planks, 302);
    table_band(&mut img);
    let rows = [
        "................",
        "................",
        "................",
        "................",
        "..hhh......MMM..",
        ".h.hSSSSS..MmM..",
        ".h.hSSSSSs..h...",
        ".hhhSSSSsT..h...",
        "...hSSSsT...h...",
        "...hSssT....h...",
        "....TTT.....h...",
        "............h...",
        "............H...",
        "................",
        "................",
        "................",
    ];
    paint(
        &mut img,
        &rows,
        &[
            ('h', rgb(0x3A2A14)),
            ('S', rgb(0xB4B4B4)),
            ('s', rgb(0x8A8A8A)),
            ('T', rgb(0x6A6A6A)),
            ('M', rgb(0x9A9A9A)),
            ('m', rgb(0x5E5E5E)),
            ('H', rgb(0x2A1E0E)),
        ],
    );
    img
}

fn crafting_table_side() -> Image {
    let p = wood_pal("oak").unwrap();
    let mut img = planks(&p.planks, 303);
    table_band(&mut img);
    let rows = [
        "................",
        "................",
        "................",
        "................",
        "..MMMM....SSS...",
        ".MmmmmM...SsS...",
        ".M.hh.M...SSS...",
        "...hh......h....",
        "...hh......h....",
        "...hh......h....",
        "...hh......h....",
        "...hh......h....",
        "...HH......H....",
        "................",
        "................",
        "................",
    ];
    paint(
        &mut img,
        &rows,
        &[
            ('h', rgb(0x5E4426)),
            ('H', rgb(0x3A2A14)),
            ('S', rgb(0xB4B4B4)),
            ('s', rgb(0x7A7A7A)),
            ('M', rgb(0x9A9A9A)),
            ('m', rgb(0xC8C8C8)),
        ],
    );
    img
}

// ---------------------------------------------------------------------------
// furnace / chest / bookshelf

fn furnace_stone(seed: u64) -> Image {
    wquant(
        &noise(seed, 1, 1, 0.55),
        &[(0x575757, 1), (0x6A6A6A, 3), (0x7A7A7A, 5), (0x888888, 4), (0x979797, 2), (0xA8A8A8, 1)],
    )
}

fn furnace_side() -> Image {
    let mut img = furnace_stone(401);
    let band = smooth_stone(405);
    for y in 0..3 {
        for x in 0..16 {
            img.set(x, y, band.get(x, y));
        }
    }
    for x in 0..16 {
        img.set(x, 3, rgb(0x4A4A4A));
        img.set(x, 15, rgb(0x505050));
    }
    img
}

fn smooth_stone(seed: u64) -> Image {
    wquant(&noise(seed, 2, 1, 0.3), &[(0x9A9A9A, 2), (0xA6A6A6, 5), (0xB0B0B0, 3)])
}

fn furnace_top() -> Image {
    let mut img = smooth_stone(402);
    for i in 0..16 {
        img.set(i, 0, rgb(0x6E6E6E));
        img.set(0, i, rgb(0x6E6E6E));
        img.set(i, 15, rgb(0x5A5A5A));
        img.set(15, i, rgb(0x5A5A5A));
    }
    img
}

fn furnace_front(lit: bool) -> Image {
    let mut img = furnace_side();
    let rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "...kkkkkkkkkk...",
        "...kbbbbbbbbk...",
        "...LLLLLLLLLL...",
        "................",
        "..kkkkkkkkkkkk..",
        "..kbbbbbbbbbbk..",
        "..kbbbbbbbbbbk..",
        "..kbbbbbbbbbbk..",
        "..kbbbbbbbbbbk..",
        "..kbbbbbbbbbbk..",
        "..LLLLLLLLLLLL..",
    ];
    paint(&mut img, &rows, &[('k', rgb(0x3A3A3A)), ('b', rgb(0x141414)), ('L', rgb(0x9A9A9A))]);
    if lit {
        let fire = [
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "....oooooooo....",
            "................",
            "................",
            "................",
            "...........r....",
            "....r...r..f....",
            "...rfr.rfr.fr...",
            "...fyfrfyfffy...",
            "...yWyyfyWyyy...",
            "................",
        ];
        paint(
            &mut img,
            &fire,
            &[
                ('r', rgb(0xB8330C)),
                ('f', rgb(0xF07A1A)),
                ('y', rgb(0xFFC23A)),
                ('W', rgb(0xFFF2A0)),
                ('o', rgb(0x7A3A10)),
            ],
        );
    }
    img
}

const CHEST_PL: [u32; 5] = [0x5C3D16, 0x8A5A22, 0xA1692C, 0xAE7432, 0xBC7F38];

fn chest_base(seed: u64, lid: bool) -> Image {
    let mut img = planks(&CHEST_PL, seed);
    let edge = rgb(0x2E1E0A);
    for i in 0..16 {
        img.set(i, 0, edge);
        img.set(0, i, edge);
        img.set(i, 15, edge);
        img.set(15, i, edge);
    }
    if lid {
        for x in 1..15 {
            img.set(x, 5, edge);
            img.set(x, 6, rgb(0x6E4718));
            img.set(x, 4, shade(img.get(x, 4), 0.88));
        }
    }
    img
}

fn chest_front() -> Image {
    let mut img = chest_base(411, true);
    let rows = [
        "................",
        "................",
        "................",
        "......kkkk......",
        "......kWLk......",
        "......kLGk......",
        "......kLGk......",
        "......kGGk......",
        "......kkkk......",
    ];
    paint(&mut img, &rows, &[('k', rgb(0x1E1E1E)), ('W', rgb(0xF0F0F0)), ('L', rgb(0xC8C8C8)), ('G', rgb(0x8E8E8E))]);
    img
}

fn chest_side() -> Image {
    chest_base(412, true)
}

fn chest_top() -> Image {
    chest_base(413, false)
}

fn bookshelf() -> Image {
    let p = wood_pal("oak").unwrap();
    let mut img = planks(&p.planks, 421);
    let back = rgb(0x2B1E0E);
    let cols = [0x8E2C24, 0x2E4F8F, 0x356E2C, 0x7A5230, 0x5E2F7A, 0xB59A5A, 0x2E6E70, 0xA34F1F];
    let mut r = Rng::new(422);
    for &(top, bot) in &[(1usize, 6usize), (9, 14)] {
        for y in top..=bot {
            for x in 1..15 {
                img.set(x, y, back);
            }
        }
        let mut x = 1;
        while x < 15 {
            if r.range(9) == 0 {
                x += 1;
                continue;
            }
            let w = (1 + r.range(2) as usize).min(15 - x);
            let h = 4 + r.range(3) as usize;
            let c = rgb(cols[r.range(cols.len() as u32) as usize]);
            for k in 0..w {
                for y in (bot + 1 - h)..=bot {
                    let mut cc = c;
                    if y == bot + 1 - h {
                        cc = shade(c, 1.25);
                    } else if k == 0 && w == 2 {
                        cc = shade(c, 1.1);
                    } else if k == w - 1 {
                        cc = shade(c, 0.8);
                    }
                    if y == bot - 1 && h >= 5 {
                        cc = rgb(0xD8C070);
                    }
                    img.set(x + k, y, cc);
                }
            }
            x += w;
        }
    }
    for x in 0..16 {
        for &y in &[0usize, 7, 8, 15] {
            let c = img.get(x, y);
            let f = if y == 7 || y == 0 { 1.05 } else { 0.8 };
            img.set(x, y, shade(c, f));
        }
    }
    for y in 0..16 {
        img.set(0, y, shade(rgb(p.planks[2]), 0.85));
        img.set(15, y, shade(rgb(p.planks[2]), 0.75));
    }
    img
}

// ---------------------------------------------------------------------------
// misc blocks

fn white_wool() -> Image {
    let mut img = wquant(&noise(431, 1, 0, 0.6), &[(0xC9CDCD, 1), (0xDADDDD, 3), (0xE9ECEC, 6), (0xF4F6F6, 2)]);
    for y in 0..16 {
        for x in 0..16 {
            if (x + y * 3) % 7 == 0 {
                let c = img.get(x, y);
                img.set(x, y, shade(c, 0.95));
            }
        }
    }
    img
}

fn rib_cols(seed: u64, pal: [u32; 4]) -> Image {
    // pal: dark, mid, light, highlight; vertical ribs with period 5
    let pat = [0usize, 1, 2, 3, 2, 1, 0, 1, 2, 3, 2, 1, 0, 1, 2, 1];
    let n = white(seed);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let mut i = pat[x] as i32;
            let t = n[y * 16 + x];
            if t < 0.12 {
                i -= 1;
            } else if t > 0.9 {
                i += 1;
            }
            img.set(x, y, rgb(pal[i.clamp(0, 3) as usize]));
        }
    }
    img
}

fn cactus_side() -> Image {
    let mut img = rib_cols(441, [0x0E4F17, 0x157624, 0x1E8C2E, 0x3FA03A]);
    for &(x, y) in &[(0, 2), (6, 4), (12, 1), (0, 9), (6, 11), (12, 7), (12, 13), (6, 15), (0, 14)] {
        img.set(x, y, rgb(0x1C1C10));
        img.set(x + 1, y, rgb(0xCFE0A8));
    }
    img
}

fn cactus_top(bottom: bool) -> Image {
    let mut img = Image::new(16, 16);
    let n = white(if bottom { 452 } else { 451 });
    for y in 0..16 {
        for x in 0..16 {
            let dx = (x as f32 - 7.5).abs();
            let dy = (y as f32 - 7.5).abs();
            let d = dx.max(dy);
            let t = n[y * 16 + x];
            let c = if d > 7.0 {
                0x0E4F17
            } else if d > 6.0 {
                if t < 0.5 { 0x157624 } else { 0x1E8C2E }
            } else if bottom {
                if t < 0.3 { 0x2E8A2E } else { 0x3FA03A }
            } else if (dx < 1.0 || dy < 1.0) && d < 5.0 {
                0x6CBF52
            } else if d < 2.0 {
                0x5AB548
            } else if t < 0.3 {
                0x2E8A2E
            } else {
                0x3FA03A
            };
            img.set(x, y, rgb(c));
        }
    }
    img
}

fn pumpkin_side() -> Image {
    let mut img = rib_cols(461, [0xB35F0E, 0xD07A15, 0xE38A1D, 0xF2A33A]);
    for x in 0..16 {
        let c = img.get(x, 0);
        img.set(x, 0, shade(c, 0.82));
        let c = img.get(x, 15);
        img.set(x, 15, shade(c, 0.78));
        let c = img.get(x, 14);
        img.set(x, 14, shade(c, 0.9));
    }
    img
}

fn pumpkin_top() -> Image {
    let n = white(471);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
            let d = fx.abs().max(fy.abs());
            let spoke = (fx.abs() - fy.abs()).abs() < 0.6 || fx.abs() < 0.6 || fy.abs() < 0.6;
            let mut c = if spoke && d > 1.5 { rgb(0xC06A12) } else { rgb(0xE38A1D) };
            if d > 6.0 {
                c = shade(c, 0.88);
            }
            if n[y * 16 + x] > 0.88 {
                c = shade(c, 1.1);
            }
            img.set(x, y, c);
        }
    }
    let stem = ["......", "..ss..", ".sggs.", ".sgGs.", "..ss..", "......"];
    for (dy, row) in stem.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let c = match ch {
                's' => 0x5A3E14,
                'g' => 0x6E7A28,
                'G' => 0x8C9A3A,
                _ => continue,
            };
            img.set(5 + dx, 5 + dy, rgb(c));
        }
    }
    img
}

fn tnt_side() -> Image {
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let c = match x % 4 {
                0 => 0x8E1C08,
                1 => 0xDB3A12,
                2 => 0xD02E0C,
                _ => 0xA82210,
            };
            let mut c = rgb(c);
            if y == 0 || y == 15 {
                c = shade(c, 0.8);
            }
            img.set(x, y, c);
        }
    }
    for x in 0..16 {
        img.set(x, 4, rgb(0xB8B8B8));
        img.set(x, 11, rgb(0xB8B8B8));
        for y in 5..11 {
            img.set(x, y, rgb(0xF4F4F4));
        }
    }
    let letters = [
        "..KKK.K..K.KKK..",
        "...K..KK.K..K...",
        "...K..K.KK..K...",
        "...K..K..K..K...",
    ];
    for (dy, row) in letters.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == 'K' {
                img.set(x, 6 + dy, rgb(0x181818));
            }
        }
    }
    img
}

fn tnt_end(top: bool) -> Image {
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let (cx, cy) = (x % 4, y % 4);
            let edge = cx == 0 || cy == 0 || cx == 3 || cy == 3;
            let corner = (cx == 0 || cx == 3) && (cy == 0 || cy == 3);
            let c = if corner {
                0x6E1606
            } else if edge {
                if cx == 0 || cy == 0 { 0xC23010 } else { 0x8E1C08 }
            } else {
                if cx == 1 && cy == 1 { 0xD0D0D0 } else { 0xA8A8A8 }
            };
            img.set(x, y, rgb(c));
        }
    }
    if top {
        let f = [(7, 6, 0x2A2A2A), (8, 6, 0x3A3A3A), (7, 7, 0x404040), (8, 7, 0x2A2A2A), (7, 8, 0x1A1A1A), (8, 8, 0x303030), (7, 9, 0x1A1A1A), (8, 9, 0x1A1A1A), (6, 5, 0x555555), (8, 5, 0x555555)];
        for (x, y, c) in f {
            img.set(x, y, rgb(c));
        }
    }
    img
}

fn torch() -> Image {
    let rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        ".......WY.......",
        ".......YO.......",
        ".......lD.......",
        ".......lD.......",
        ".......lD.......",
        ".......lD.......",
        ".......lD.......",
        ".......lD.......",
        ".......lD.......",
        ".......dD.......",
    ];
    grid(
        &rows,
        &[
            ('W', rgb(0xFFFBD0)),
            ('Y', rgb(0xFFD84A)),
            ('O', rgb(0xF59A2A)),
            ('l', rgb(0x8E6B3A)),
            ('D', rgb(0x654A26)),
            ('d', rgb(0x5A4222)),
        ],
    )
}

// ---------------------------------------------------------------------------
// plants

/// Grayscale plant with lighter tops (for tinted foliage).
fn gray_plant(rows: &[&str], seed: u64) -> Image {
    let n = white(seed);
    let mut img = Image::new(16, 16);
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let base = match ch {
                'g' => 0.0,
                'd' => -26.0,
                'l' => 22.0,
                _ => continue,
            };
            let v = 0xB4 as f32 - y as f32 * 3.5 + base + (n[y * 16 + x] - 0.5) * 24.0;
            img.set(x, y, gray(v.clamp(40.0, 230.0) as u8));
        }
    }
    img
}

fn short_grass() -> Image {
    gray_plant(
        &[
            "................",
            "................",
            "................",
            "...........l....",
            "..l........g....",
            "..g....l...g..l.",
            "..g....g..g...g.",
            ".gd....g..g..g..",
            ".g.g...gd.g..g..",
            ".g.g.l..g.gd.g.l",
            "gd.g.g..g.g.g..g",
            "g..dg.g.ggd.g.gd",
            "g..gg.g.gg..gdg.",
            ".g.dg.gg.gd.dg..",
            ".gd.gdg.dg.ggd..",
            "..gd.gd.dgdgd...",
        ],
        481,
    )
}

fn fern() -> Image {
    gray_plant(
        &[
            "................",
            "................",
            ".........l......",
            "........lg..l...",
            "..l....g.g.g....",
            "...g.gg..gg..l..",
            ".l..gd..gg..g...",
            "..g..gg.gd.gg...",
            "....dgggg.g..g..",
            ".lg...dggdg.g...",
            "...gg..gdggg....",
            "..l..gggdg..gl..",
            "...ggd.dgg.dg...",
            ".....dgdgdgd....",
            "......dgdd......",
            ".......d........",
        ],
        482,
    )
}

const STEM: [(char, C); 2] = [('g', [0x3E, 0x8D, 0x1F, 255]), ('G', [0x5D, 0xA8, 0x2E, 255])];

fn flower(rows: &[&str], pal: &[(char, C)]) -> Image {
    let mut img = grid(rows, &STEM);
    paint(&mut img, rows, pal);
    img
}

fn dandelion() -> Image {
    flower(
        &[
            "................",
            "................",
            "................",
            "................",
            "................",
            "......yYy.......",
            ".....yYOYy......",
            ".....YOOOY......",
            ".....yYOYy......",
            "......yYy.......",
            ".......g........",
            ".......g..gG....",
            "..Gg...g.gG.....",
            "...Gg..ggg......",
            "....GgGg........",
            ".......g........",
        ],
        &[('Y', rgb(0xFFEC4F)), ('y', rgb(0xF2C21E)), ('O', rgb(0xE8A50E))],
    )
}

fn poppy() -> Image {
    flower(
        &[
            "................",
            "................",
            "................",
            "................",
            "......rR.R......",
            ".....RRRRRr.....",
            "....rRRkRRRr....",
            ".....RkKkRR.....",
            ".....rRRkRr.....",
            "......rRRr......",
            ".......g........",
            "...g...g........",
            "...Gg..g..gG....",
            "....Gg.g.gG.....",
            ".....GgggG......",
            ".......g........",
        ],
        &[('R', rgb(0xED302C)), ('r', rgb(0xB01E18)), ('k', rgb(0x3A1A10)), ('K', rgb(0x1E2A10))],
    )
}

fn blue_orchid() -> Image {
    flower(
        &[
            "................",
            "................",
            "..bB.....bB.....",
            ".bBLb...bBLb....",
            "..bBd....bBd....",
            "...g......g.....",
            "...g...bB.g.....",
            "....g.bBLbg.....",
            "....g..bBd......",
            ".....g..g.g.....",
            ".....g..gg......",
            "......g.g.......",
            "..G...ggg...G...",
            "...GG..g...GG...",
            "....Gg.g..gG....",
            "......ggg.......",
        ],
        &[('b', rgb(0x2B9FD4)), ('B', rgb(0x4FC2EE)), ('L', rgb(0xA8E6FA)), ('d', rgb(0x1A6E9E))],
    )
}

fn dead_bush() -> Image {
    grid(
        &[
            "................",
            "..b.........b...",
            "...B...b...B....",
            ".b..b..B..b..b..",
            "..b..B.b.B..B...",
            "...B..bbb..b....",
            "b...b..B..b...b.",
            ".B...b.b.B...B..",
            "..bB..bBb..Bb...",
            "....b..b..b.....",
            ".....B.b.B......",
            "......bBb.......",
            ".......b........",
            ".......B........",
            ".......b........",
            ".......b........",
        ],
        &[('b', rgb(0x6B4A20)), ('B', rgb(0x946428))],
    )
}

fn sugar_cane() -> Image {
    let mut img = Image::new(16, 16);
    let stalks = [(3usize, 1usize), (7, 3), (12, 0)];
    let (l, m, d, j) = (rgb(0xAEDB7A), rgb(0x8EC25E), rgb(0x6A9A3F), rgb(0xD6EEAA));
    for &(x, off) in &stalks {
        for y in 0..16 {
            let joint = (y + off) % 5 == 0;
            img.set(x, y, if joint { j } else { l });
            img.set(x + 1, y, if joint { m } else { d });
        }
        // leaves
        let ly = (off * 3 + 4) % 14;
        img.set(x + 2, ly, m);
        img.set(x + 3, ly - 1, l);
        if x > 2 {
            img.set(x - 1, ly + 5, m);
            img.set(x - 2, ly + 4, l);
        }
    }
    img
}

/// Saplings (cross-model plants, final colours). Grid chars: k/g/l/h leaves dark ->
/// highlight, b/B stem dark/light.
fn sapling(rows: &[&str], leaves: [u32; 4], stem: [u32; 2]) -> Image {
    let [k, g, l, h] = leaves;
    grid(
        rows,
        &[
            ('k', rgb(k)),
            ('g', rgb(g)),
            ('l', rgb(l)),
            ('h', rgb(h)),
            ('b', rgb(stem[0])),
            ('B', rgb(stem[1])),
        ],
    )
}

fn oak_sapling() -> Image {
    sapling(
        &[
            "................",
            "................",
            "......kgk.......",
            "....kgglhgk.k...",
            "...kglhlggkglk..",
            "..kglggkgglhlgk.",
            "..kgggkglgkggk..",
            ".kglhgkgBggkgk..",
            "..kgggkgBkglgk..",
            "...kgk.kBgkgk...",
            "....k...Bk.k....",
            "......kgB.......",
            ".......kB.......",
            "........B.......",
            "........b.......",
            "........b.......",
        ],
        [0x23500F, 0x3A7A1A, 0x55A128, 0x76BE3E],
        [0x4F3818, 0x7A5A2C],
    )
}

fn birch_sapling() -> Image {
    sapling(
        &[
            "................",
            "................",
            ".......kk.......",
            ".....kkglk......",
            "....kglhlgkk....",
            "...kglggglhgk...",
            "..kglgkglgggk...",
            "...kgkgBkglgk...",
            "..kglgkBgkgk....",
            "...kgk.Bk.k.....",
            "....k..B........",
            ".......Bkg......",
            ".......B.k......",
            ".......B........",
            ".......b........",
            ".......b........",
        ],
        [0x3D5C22, 0x5E8A38, 0x7EAA52, 0xA0C674],
        [0x9E9A8E, 0xE4E1D6],
    )
}

fn spruce_sapling() -> Image {
    sapling(
        &[
            "................",
            "........l.......",
            ".......kgl......",
            "......kgglk.....",
            ".......kgk......",
            "......kgglk.....",
            ".....kgglggk....",
            "....kgkgBglgk...",
            "......kgBgk.....",
            ".....kggBlgk....",
            "....kgglBgglgk..",
            "...kgkgkBgkgkgk.",
            "......k.Bb.k....",
            "........Bb......",
            "........bb......",
            "........bb......",
        ],
        [0x163220, 0x274D30, 0x3A6842, 0x52855A],
        [0x3B2812, 0x5A3E1F],
    )
}

fn jungle_sapling() -> Image {
    sapling(
        &[
            "................",
            "................",
            "....kk....kk....",
            "...kglk..klgk...",
            "..kglhgkkghlgk..",
            "..kgllgkkgllgk..",
            ".kglggk..kgglgk.",
            ".kgk.kglgk..kgk.",
            "....kglhlgk.....",
            "...kgk.Bkgk.....",
            "..kgk..B..kgk...",
            ".......Bkglgk...",
            "....kgkB..kk....",
            ".......B........",
            ".......b........",
            ".......b........",
        ],
        [0x1B4A0C, 0x2D7414, 0x45991F, 0x63B834],
        [0x4A3712, 0x6E5523],
    )
}

fn acacia_sapling() -> Image {
    sapling(
        &[
            "................",
            "................",
            "....kggk........",
            "..kgglhgk..kgk..",
            ".kglhlgglk.kglgk",
            ".kgglglggkkglhgk",
            "..kgkgkgBkkgglgk",
            "...k.kkB..kgkk..",
            ".......B..kBk...",
            "........B.B.....",
            "........BB......",
            "........B.......",
            "........B.......",
            "........B.......",
            "........b.......",
            "........b.......",
        ],
        [0x4A5E14, 0x6C8424, 0x8EA634, 0xB0C64E],
        [0x5B5650, 0x7A7468],
    )
}

fn wheat(stage: u32) -> Image {
    let mut img = Image::new(16, 16);
    // (x, height factor, lean every n px (0 = straight), lean dir)
    let stalks: [(i32, f32, i32, i32); 8] = [
        (1, 0.8, 5, 1),
        (3, 1.0, 0, 0),
        (5, 0.75, 6, -1),
        (7, 0.95, 0, 0),
        (9, 0.85, 5, 1),
        (11, 1.0, 0, 0),
        (13, 0.8, 6, -1),
        (14, 0.6, 0, 0),
    ];
    let t = (stage as f32 / 6.0).min(1.0);
    let stem = lerp(0x2F8A12, 0x8CA830, t);
    let stem_d = shade(stem, 0.75);
    let ripe = stage == 7;
    for (i, &(x0, f, every, dir)) in stalks.iter().enumerate() {
        let h = (((stage + 1) as f32 / 8.0) * 15.0 * f).round().max(1.0) as i32;
        let head = if ripe { 6 } else if stage >= 5 { 4 } else { 0 };
        for k in 0..h {
            let y = 15 - k;
            let x = x0 + if every > 0 { dir * (k / every) } else { 0 };
            let from_top = h - 1 - k;
            let c = if ripe {
                if from_top < head {
                    if (from_top + i as i32) % 2 == 0 { rgb(0xDCBB65) } else { rgb(0xB8952F) }
                } else if k % 3 == 0 {
                    rgb(0x8F7A2A)
                } else {
                    rgb(0xA8903A)
                }
            } else if from_top < head {
                if from_top % 2 == 0 { lerp(0x8CB840, 0xC8C860, t) } else { lerp(0x6E9A2E, 0xA8A440, t) }
            } else if k % 3 == 1 {
                stem_d
            } else {
                stem
            };
            img.put(x, y, c);
            if from_top < head && from_top > 0 && from_top % 2 == 1 {
                let side = if i % 2 == 0 { 1 } else { -1 };
                img.put(x + side, y, shade(c, 0.85));
            }
            if from_top >= head && k % 4 == 2 && h > 3 {
                let side = if (k / 4 + i as i32) % 2 == 0 { 1 } else { -1 };
                let lc = if ripe { rgb(0x9A8434) } else { stem_d };
                img.put(x + side, y - 1, lc);
            }
        }
    }
    img
}

// ---------------------------------------------------------------------------
// destroy stages

fn crack_order() -> Vec<(i32, i32)> {
    let mut r = Rng::new(9001);
    let mut seen = [[false; 16]; 16];
    let mut out = Vec::new();
    let mut tips: Vec<(i32, i32, i32, i32)> = vec![(7, 8, 1, -1), (8, 7, -1, 1), (7, 7, -1, -1), (8, 8, 1, 1)];
    for &(x, y, _, _) in &tips {
        seen[y as usize][x as usize] = true;
        out.push((x, y));
    }
    let mut iter = 0;
    while out.len() < 150 && !tips.is_empty() && iter < 5000 {
        iter += 1;
        let i = r.range(tips.len() as u32) as usize;
        let (x, y, dx, dy) = tips[i];
        let roll = r.range(4);
        let (sx, sy) = match roll {
            0 | 1 => (dx, dy),
            2 => (dx, if dx == 0 { dy } else { 0 }),
            _ => (if dy == 0 { dx } else { 0 }, dy),
        };
        let (nx, ny) = (x + sx, y + sy);
        if !(0..16).contains(&nx) || !(0..16).contains(&ny) {
            tips.remove(i);
            continue;
        }
        if !seen[ny as usize][nx as usize] {
            seen[ny as usize][nx as usize] = true;
            out.push((nx, ny));
        }
        tips[i] = (nx, ny, dx, dy);
        if r.range(9) == 0 && tips.len() < 12 {
            // branch with rotated direction
            let (bx, by) = if r.range(2) == 0 { (dx, -dy) } else { (-dx, dy) };
            let (bx, by) = if bx == 0 && by == 0 { (1, 0) } else { (bx, by) };
            tips.push((nx, ny, bx, by));
        }
    }
    out
}

fn destroy(stage: usize) -> Image {
    let order = crack_order();
    let counts = [6, 12, 20, 30, 42, 55, 70, 88, 108, 130];
    let n = counts[stage].min(order.len());
    let mut img = Image::new(16, 16);
    for (k, &(x, y)) in order[..n].iter().enumerate() {
        let a = if k % 5 == 3 { 150 } else { 215 };
        img.put(x, y, [0x18, 0x18, 0x18, a]);
    }
    img
}

// ---------------------------------------------------------------------------
// door / ladder / bed

/// Oak planks rotated 90 degrees: vertical boards 4 px wide (door panels).
fn vplanks(seed: u64) -> Image {
    let p = wood_pal("oak").unwrap();
    let h = planks(&p.planks, seed);
    let mut img = Image::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            img.set(x, y, h.get(y, x));
        }
    }
    img
}

/// Door palette: frame outline, dark frame, window shadow / highlight lips,
/// iron hinge and handle, transparent window.
const DOOR_PAL: [(char, C); 7] = [
    ('D', rgb(0x4C3A21)),
    ('d', rgb(0x7A5F38)),
    ('s', rgb(0x5E4729)),
    ('h', rgb(0xC4A06A)),
    ('.', [0, 0, 0, 0]),
    ('I', rgb(0x3A3A3A)),
    ('i', rgb(0x8A8A8A)),
];

fn oak_door_top() -> Image {
    let mut img = vplanks(601);
    let rows = [
        "DDDDDDDDDDDDDDDD",
        "DddddddddddddddD",
        "DdssssssssssssdD",
        "Dds....hs....hdD",
        "Dds....hs....hdD",
        "Dds....hs....hdD",
        "Dds....hs....hdD",
        "DdhhhhhhhhhhhhdD",
        "Dd____________dD",
        "Dd____________dD",
        "Dd____________dD",
        "Dd____________dD",
        "Dd____________dD",
        "Ddi___________dD",
        "DdI___________dD",
        "Dd____________dD",
    ];
    paint(&mut img, &rows, &DOOR_PAL);
    img
}

fn oak_door_bottom() -> Image {
    let mut img = vplanks(602);
    let rows = [
        "Dd____________dD",
        "Ddi___________dD",
        "DdI_________iidD",
        "Dd__________IIdD",
        "Dd____________dD",
        "Dd____________dD",
        "DdssssssssssssdD",
        "DdhhhhhhhhhhhhdD",
        "Dd____________dD",
        "Dd____________dD",
        "Dd____________dD",
        "Ddi___________dD",
        "DdI___________dD",
        "Dd____________dD",
        "DddddddddddddddD",
        "DDDDDDDDDDDDDDDD",
    ];
    paint(&mut img, &rows, &DOOR_PAL);
    img
}

fn ladder() -> Image {
    let rows = [
        ".ln..........ln.",
        ".lnHHlHHHHlHHln.",
        ".lnKKKKKKKKKKln.",
        ".ln..........lK.",
        ".lK..........ln.",
        ".lnHHHlHHHHHHln.",
        ".lnKKKKKKKKKKln.",
        ".mn..........ln.",
        ".ln..........mn.",
        ".lnHHHHHlHHHHln.",
        ".lnKKKKKKKKKKln.",
        ".ln..........ln.",
        ".lK..........ln.",
        ".lnHlHHHHHHlHln.",
        ".lnKKKKKKKKKKln.",
        ".ln..........lK.",
    ];
    grid(
        &rows,
        &[
            ('H', rgb(0xB8945F)),
            ('l', rgb(0xAD8A55)),
            ('m', rgb(0x8C6E42)),
            ('n', rgb(0x6A5230)),
            ('K', rgb(0x3F3020)),
        ],
    )
}

const BED_RED: [(u32, u32); 4] = [(0x7E1A16, 1), (0x9A221D, 3), (0xAE2B25, 5), (0xC23A33, 2)];

const BED_PAL: [(char, C); 12] = [
    ('R', rgb(0xD0463E)), // blanket fold highlight
    ('r', rgb(0x8E1F1A)), // blanket shade
    ('k', rgb(0x5E1310)), // blanket hem / deep shadow
    ('W', rgb(0xFFFFFF)),
    ('w', rgb(0xE6E6E6)),
    ('g', rgb(0xC4C4C4)),
    ('G', rgb(0x9A9A9A)),
    ('b', rgb(0xAD8A55)), // oak frame light
    ('m', rgb(0x9C7F4E)),
    ('n', rgb(0x8C6E42)),
    ('D', rgb(0x6A5230)),
    ('.', [0, 0, 0, 0]),
];

fn blanket(seed: u64) -> Image {
    wquant(&noise(seed, 1, 0, 0.5), &BED_RED)
}

fn bed_head_top() -> Image {
    let mut img = blanket(611);
    let rows = [
        "gwwwwwwwwwwwwwwg",
        "gWWWWWWWWWWWWWwG",
        "gWWWWWWWWWWWWwwG",
        "gWWWWWwWWWWWWwwG",
        "gwWWWWWWWWWWwwgG",
        "gwwwwwwwwwwwwggG",
        "ggggggggggggggGG",
        "RRRRRRRRRRRRRRRR",
        "rrrrrrrrrrrrrrrr",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
    ];
    paint(&mut img, &rows, &BED_PAL);
    img
}

fn bed_foot_top() -> Image {
    let mut img = blanket(612);
    let rows = [
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "r______________r",
        "rrrrrrrrrrrrrrrr",
        "RRRRRRRRRRRRRRRR",
        "R______________R",
        "rrrrrrrrrrrrrrrr",
        "kkkkkkkkkkkkkkkk",
    ];
    paint(&mut img, &rows, &BED_PAL);
    img
}

/// Side / end view: 9 px of content in rows 7..15, transparent above.
/// `band` overlays the 5 blanket rows (7..11), `wood` the 4 frame rows (12..15).
fn bed_profile(seed: u64, band: &[&str; 5], wood: &[&str; 4]) -> Image {
    let b = blanket(seed);
    let p = wood_pal("oak").unwrap();
    let w = planks(&p.planks, seed + 1);
    let mut img = Image::new(16, 16);
    for y in 7..12 {
        for x in 0..16 {
            img.set(x, y, b.get(x, y));
        }
    }
    for y in 12..16 {
        for x in 0..16 {
            // rows 4..6 are one board's grain, without its seam row
            img.set(x, y, w.get(x, (y - 12) % 3 + 4));
        }
    }
    let mut rows = vec!["................"; 7];
    rows.extend_from_slice(band);
    rows.extend_from_slice(wood);
    paint(&mut img, &rows, &BED_PAL);
    img
}

const BED_WOOD: [&str; 4] = [
    "DDDDDDDDDDDDDDDD",
    "Db____________bD",
    "Dm____________nD",
    "DDDDDDDDDDDDDDDD",
];

fn bed_side(seed: u64) -> Image {
    bed_profile(
        seed,
        &[
            "RRRRRRRRRRRRRRRR",
            "________________",
            "________________",
            "rrrrrrrrrrrrrrrr",
            "kkkkkkkkkkkkkkkk",
        ],
        &BED_WOOD,
    )
}

fn bed_head_end() -> Image {
    bed_profile(
        615,
        &[
            "RRgwwwwwwwwwwgRR",
            "__gWWWWWWWWWwg__",
            "_rgwwwwwwwwwwgr_",
            "rrGggggggggggGrr",
            "kkkkkkkkkkkkkkkk",
        ],
        &BED_WOOD,
    )
}

fn bed_foot_end() -> Image {
    bed_profile(
        616,
        &[
            "RRRRRRRRRRRRRRRR",
            "R______________R",
            "rrrrrrrrrrrrrrrr",
            "r______________r",
            "kkkkkkkkkkkkkkkk",
        ],
        &BED_WOOD,
    )
}
