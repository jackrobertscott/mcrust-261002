//! gui textures (procedurally generated pixel art): bitmap font, widgets,
//! HUD icons, container backgrounds and the title logo.
use crate::image::{rgb, Image, Rng};

pub const NAMES: &[&str] = &[
    "font",
    "widgets_button",
    "widgets_button_hover",
    "widgets_button_disabled",
    "hotbar",
    "hotbar_selection",
    "crosshair",
    "heart_container",
    "heart_full",
    "heart_half",
    "food_empty",
    "food_full",
    "food_half",
    "air_bubble",
    "inventory",
    "crafting_table",
    "furnace",
    "chest",
    "furnace_flame",
    "furnace_arrow",
    "logo",
    "background_tile",
];

pub fn get(name: &str) -> Option<Image> {
    Some(match name {
        "font" => font(),
        "widgets_button" => button(0),
        "widgets_button_hover" => button(1),
        "widgets_button_disabled" => button(2),
        "hotbar" => hotbar(),
        "hotbar_selection" => hotbar_selection(),
        "crosshair" => crosshair(),
        "heart_container" => heart(0),
        "heart_full" => heart(2),
        "heart_half" => heart(1),
        "food_empty" => food(0),
        "food_full" => food(2),
        "food_half" => food(1),
        "air_bubble" => air_bubble(),
        "inventory" => inventory(),
        "crafting_table" => crafting_table(),
        "furnace" => furnace(),
        "chest" => chest(),
        "furnace_flame" => furnace_flame(),
        "furnace_arrow" => furnace_arrow(),
        "logo" => logo(),
        "background_tile" => background_tile(),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Font
// ---------------------------------------------------------------------------

/// Glyph bitmaps: rows (top to bottom, starting at cell row 0) separated by
/// spaces, '#' = pixel. Lowercase letters start on row 2 (x-height 5),
/// descenders reach row 7. Recreates vanilla `ascii.png` shapes.
const GLYPHS: &[(u8, &str)] = &[
    (b'!', "# # # # # . #"),
    (b'"', "#.# #.# #.#"),
    (b'#', ".#.#. .#.#. ##### .#.#. ##### .#.#. .#.#."),
    (b'$', "..#.. .#### #.... .###. ....# ####. ..#.."),
    (b'%', "#...# #..#. ...#. ..#.. .#... .#..# #...#"),
    (b'&', "..#.. .#.#. ..#.. .##.# #..#. #..#. .##.#"),
    (b'\'', "# # #"),
    (b'(', "..## .#.. #... #... #... .#.. ..##"),
    (b')', "##.. ..#. ...# ...# ...# ..#. ##.."),
    (b'*', ". . #..# .##. #..#"),
    (b'+', ". ..#.. ..#.. ##### ..#.. ..#.."),
    (b',', ". . . . . # # #"),
    (b'-', ". . . #####"),
    (b'.', ". . . . . # #"),
    (b'/', "....# ...#. ...#. ..#.. .#... .#... #...."),
    (b'0', ".###. #...# #..## #.#.# ##..# #...# .###."),
    (b'1', "..#.. .##.. ..#.. ..#.. ..#.. ..#.. #####"),
    (b'2', ".###. #...# ....# ..##. .#... #...# #####"),
    (b'3', ".###. #...# ....# ..##. ....# #...# .###."),
    (b'4', "...## ..#.# .#..# #...# ##### ....# ....#"),
    (b'5', "##### #.... ####. ....# ....# #...# .###."),
    (b'6', "..##. .#... #.... ####. #...# #...# .###."),
    (b'7', "##### #...# ....# ...#. ..#.. ..#.. ..#.."),
    (b'8', ".###. #...# #...# .###. #...# #...# .###."),
    (b'9', ".###. #...# #...# .#### ....# ...#. .##.."),
    (b':', ". # # . . # #"),
    (b';', ". # # . . # # #"),
    (b'<', "...# ..#. .#.. #... .#.. ..#. ...#"),
    (b'=', ". . ##### . . #####"),
    (b'>', "#... .#.. ..#. ...# ..#. .#.. #..."),
    (b'?', ".###. #...# ....# ...#. ..#.. . ..#.."),
    (b'@', ".####. #....# #.##.# #.##.# #.#### #..... .####."),
    (b'A', ".###. #...# ##### #...# #...# #...# #...#"),
    (b'B', "####. #...# ####. #...# #...# #...# ####."),
    (b'C', ".###. #...# #.... #.... #.... #...# .###."),
    (b'D', "####. #...# #...# #...# #...# #...# ####."),
    (b'E', "##### #.... ###.. #.... #.... #.... #####"),
    (b'F', "##### #.... ###.. #.... #.... #.... #...."),
    (b'G', ".#### #.... #..## #...# #...# #...# .###."),
    (b'H', "#...# #...# ##### #...# #...# #...# #...#"),
    (b'I', "### .#. .#. .#. .#. .#. ###"),
    (b'J', "....# ....# ....# ....# ....# #...# .###."),
    (b'K', "#...# #..#. ###.. #..#. #...# #...# #...#"),
    (b'L', "#.... #.... #.... #.... #.... #.... #####"),
    (b'M', "#...# ##.## #.#.# #...# #...# #...# #...#"),
    (b'N', "#...# ##..# #.#.# #..## #...# #...# #...#"),
    (b'O', ".###. #...# #...# #...# #...# #...# .###."),
    (b'P', "####. #...# ####. #.... #.... #.... #...."),
    (b'Q', ".###. #...# #...# #...# #...# #..#. .##.#"),
    (b'R', "####. #...# ####. #...# #...# #...# #...#"),
    (b'S', ".#### #.... .###. ....# ....# #...# .###."),
    (b'T', "##### ..#.. ..#.. ..#.. ..#.. ..#.. ..#.."),
    (b'U', "#...# #...# #...# #...# #...# #...# .###."),
    (b'V', "#...# #...# #...# #...# .#.#. .#.#. ..#.."),
    (b'W', "#...# #...# #...# #...# #.#.# ##.## #...#"),
    (b'X', "#...# .#.#. ..#.. .#.#. #...# #...# #...#"),
    (b'Y', "#...# .#.#. ..#.. ..#.. ..#.. ..#.. ..#.."),
    (b'Z', "##### ....# ...#. ..#.. .#... #.... #####"),
    (b'[', "### #.. #.. #.. #.. #.. ###"),
    (b'\\', "#.... .#... .#... ..#.. ...#. ...#. ....#"),
    (b']', "### ..# ..# ..# ..# ..# ###"),
    (b'^', "..#.. .#.#. #...#"),
    (b'_', ". . . . . . . #####"),
    (b'`', "#. .#"),
    (b'a', ". . .###. ....# .#### #...# .####"),
    (b'b', "#.... #.... #.##. ##..# #...# #...# ####."),
    (b'c', ". . .###. #...# #.... #...# .###."),
    (b'd', "....# ....# .##.# #..## #...# #...# .####"),
    (b'e', ". . .###. #...# ##### #.... .####"),
    (b'f', "..## .#.. #### .#.. .#.. .#.. .#.."),
    (b'g', ". . .#### #...# #...# .#### ....# ####."),
    (b'h', "#.... #.... #.##. ##..# #...# #...# #...#"),
    (b'i', "# . # # # # #"),
    (b'j', "....# . ....# ....# ....# #...# #...# .###."),
    (b'k', "#... #... #..# #.#. ##.. #.#. #..#"),
    (b'l', "#. #. #. #. #. #. .#"),
    (b'm', ". . ##.#. #.#.# #.#.# #...# #...#"),
    (b'n', ". . ####. #...# #...# #...# #...#"),
    (b'o', ". . .###. #...# #...# #...# .###."),
    (b'p', ". . #.##. ##..# #...# ####. #.... #...."),
    (b'q', ". . .##.# #..## #...# .#### ....# ....#"),
    (b'r', ". . #.##. ##..# #.... #.... #...."),
    (b's', ". . .#### #.... .###. ....# ####."),
    (b't', ".#. .#. ### .#. .#. .#. ..#"),
    (b'u', ". . #...# #...# #...# #...# .####"),
    (b'v', ". . #...# #...# #...# .#.#. ..#.."),
    (b'w', ". . #...# #...# #.#.# #.#.# .####"),
    (b'x', ". . #...# .#.#. ..#.. .#.#. #...#"),
    (b'y', ". . #...# #...# #...# .#### ....# ####."),
    (b'z', ". . ##### ...#. ..#.. .#... #####"),
    (b'{', "..## .#.. .#.. #... .#.. .#.. ..##"),
    (b'|', "# # # # # # # #"),
    (b'}', "##.. ..#. ..#. ...# ..#. ..#. ##.."),
    (b'~', ".##..# #..##."),
];

fn glyph(c: u8) -> Option<&'static str> {
    GLYPHS.iter().find(|g| g.0 == c).map(|g| g.1)
}

/// Advance width in pixels including the 1px spacing (vanilla rules: computed
/// from the rightmost glyph pixel, space = 4).
pub fn glyph_width(c: u8) -> u32 {
    if c == b' ' {
        return 4;
    }
    match glyph(c) {
        Some(g) => {
            let w = g
                .split_whitespace()
                .map(|r| r.rfind('#').map(|i| i + 1).unwrap_or(0))
                .max()
                .unwrap_or(0);
            w as u32 + 1
        }
        None => {
            if c < 32 || c == 127 {
                0
            } else {
                6
            }
        }
    }
}

fn font() -> Image {
    let mut img = Image::new(128, 128);
    let white = [255, 255, 255, 255];
    for &(c, g) in GLYPHS {
        let cx = (c as usize % 16) * 8;
        let cy = (c as usize / 16) * 8;
        for (y, row) in g.split_whitespace().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '#' && x < 8 && y < 8 {
                    img.set(cx + x, cy + y, white);
                }
            }
        }
    }
    img
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Draw a char-grid sprite using a palette; chars not in the palette are skipped.
fn draw_grid(img: &mut Image, x0: i32, y0: i32, rows: &[&str], pal: &[(char, [u8; 4])]) {
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some(p) = pal.iter().find(|p| p.0 == ch) {
                img.put(x0 + x as i32, y0 + y as i32, p.1);
            }
        }
    }
}

const PANEL: [u8; 4] = rgb(0xC6C6C6);
const SLOT_FILL: [u8; 4] = rgb(0x8B8B8B);
const SLOT_DARK: [u8; 4] = rgb(0x373737);
const WHITE: [u8; 4] = rgb(0xFFFFFF);
const BLACK: [u8; 4] = rgb(0x000000);
const SHADOW: [u8; 4] = rgb(0x555555);

/// Vanilla container panel: #C6C6C6 fill, 2px white top-left highlight,
/// 2px #555555 bottom-right shadow, rounded black outline.
fn panel(w: usize, h: usize) -> Image {
    let mut img = Image::new(w, h);
    let (wi, hi) = (w as i32, h as i32);
    for y in 0..hi {
        for x in 0..wi {
            // distance to each edge
            let (l, t, r, b) = (x, y, wi - 1 - x, hi - 1 - y);
            let near_h = l.min(r);
            let near_v = t.min(b);
            // rounded corners: corner distance sum
            if near_h + near_v < 2 {
                continue; // transparent
            }
            if near_h + near_v == 2 && near_h <= 1 && near_v <= 1 || near_h == 0 || near_v == 0 {
                img.put(x, y, BLACK);
                continue;
            }
            // inner pixel: distances from the inner edge (0 = first inner px)
            let (il, it, ir, ib) = (l - 1, t - 1, r - 1, b - 1);
            let hl = il <= 1 || it <= 1;
            let sh = ir <= 1 || ib <= 1;
            let c = if hl && sh {
                // junction at top-right / bottom-left corners
                let a = it.min(il);
                let bb = ir.min(ib);
                if a < bb {
                    WHITE
                } else if a > bb {
                    SHADOW
                } else {
                    PANEL
                }
            } else if hl {
                WHITE
            } else if sh {
                SHADOW
            } else {
                PANEL
            };
            img.put(x, y, c);
        }
    }
    img
}

/// Recessed frame of size w x h at (x,y): dark top-left, white bottom-right.
fn inset(img: &mut Image, x: i32, y: i32, w: i32, h: i32, fill: [u8; 4]) {
    img.fill_rect(x, y, w, h, fill);
    img.fill_rect(x, y, w - 1, 1, SLOT_DARK);
    img.fill_rect(x, y, 1, h - 1, SLOT_DARK);
    img.fill_rect(x + 1, y + h - 1, w - 1, 1, WHITE);
    img.fill_rect(x + w - 1, y + 1, 1, h - 1, WHITE);
}

/// Standard 18x18 slot for an item drawn at (x,y).
fn slot(img: &mut Image, x: i32, y: i32) {
    inset(img, x - 1, y - 1, 18, 18, SLOT_FILL);
}

/// Big 26x26 output slot for an item drawn at (x,y).
fn big_slot(img: &mut Image, x: i32, y: i32) {
    inset(img, x - 5, y - 5, 26, 26, SLOT_FILL);
}

fn player_inventory(img: &mut Image) {
    for r in 0..3 {
        for c in 0..9 {
            slot(img, 8 + c * 18, 84 + r * 18);
        }
    }
    for c in 0..9 {
        slot(img, 8 + c * 18, 142);
    }
}

/// Right-pointing arrow mask of total size w x h (h odd); `head` = head width.
fn arrow_mask(w: i32, h: i32, head: i32, shaft_half: i32) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    let cy = h / 2;
    for x in 0..w {
        for y in 0..h {
            let d = (y - cy).abs();
            let inside = if x >= w - head {
                let k = x - (w - head);
                d <= (head - 1 - k).min(cy)
            } else {
                d <= shaft_half
            };
            if inside {
                v.push((x, y));
            }
        }
    }
    v
}

fn draw_arrow(img: &mut Image, x0: i32, y0: i32, w: i32, h: i32, head: i32, sh: i32, c: [u8; 4]) {
    for (x, y) in arrow_mask(w, h, head, sh) {
        img.put(x0 + x, y0 + y, c);
    }
}

const FLAME: [&str; 14] = [
    "......#.......",
    "......#.......",
    ".....##...#...",
    ".....###..#...",
    "....####.##...",
    "...#####.###..",
    "...#########..",
    "..###########.",
    "..###########.",
    ".#############",
    ".#############",
    ".############.",
    "..##########..",
    "...########...",
];

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

fn inventory() -> Image {
    let mut img = panel(176, 166);
    for i in 0..4 {
        slot(&mut img, 8, 8 + i * 18);
    }
    // player preview
    inset(&mut img, 25, 7, 52, 72, BLACK);
    for (x, y) in [(98, 18), (116, 18), (98, 36), (116, 36), (154, 28)] {
        slot(&mut img, x, y);
    }
    draw_arrow(&mut img, 135, 30, 16, 13, 7, 1, SLOT_FILL);
    player_inventory(&mut img);
    img
}

fn crafting_table() -> Image {
    let mut img = panel(176, 166);
    for r in 0..3 {
        for c in 0..3 {
            slot(&mut img, 30 + c * 18, 17 + r * 18);
        }
    }
    big_slot(&mut img, 124, 35);
    draw_arrow(&mut img, 89, 35, 22, 15, 8, 2, SLOT_FILL);
    player_inventory(&mut img);
    img
}

fn furnace() -> Image {
    let mut img = panel(176, 166);
    slot(&mut img, 56, 17);
    slot(&mut img, 56, 53);
    big_slot(&mut img, 116, 35);
    draw_grid(&mut img, 56, 36, &FLAME, &[('#', SLOT_FILL)]);
    draw_arrow(&mut img, 79, 34, 24, 17, 9, 2, SLOT_FILL);
    player_inventory(&mut img);
    img
}

fn chest() -> Image {
    let mut img = panel(176, 166);
    for r in 0..3 {
        for c in 0..9 {
            slot(&mut img, 8 + c * 18, 18 + r * 18);
        }
    }
    player_inventory(&mut img);
    img
}

fn furnace_flame() -> Image {
    let mut img = Image::new(14, 14);
    let inside = |x: i32, y: i32| {
        x >= 0 && y >= 0 && x < 14 && y < 14 && FLAME[y as usize].as_bytes()[x as usize] == b'#'
    };
    for y in 0..14 {
        for x in 0..14 {
            if !inside(x, y) {
                continue;
            }
            // distance to the flame edge (0 = outer pixel)
            let mut d = 0;
            'o: for r in 1..4 {
                for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
                    if !inside(x + dx, y + dy) {
                        break 'o;
                    }
                }
                d = r;
            }
            let c = match d {
                0 => {
                    if y < 6 {
                        rgb(0xC8380E)
                    } else {
                        rgb(0xE05A10)
                    }
                }
                1 => rgb(0xF89A1C),
                2 => rgb(0xFFD02E),
                _ => rgb(0xFFF59A),
            };
            img.put(x, y, c);
        }
    }
    img
}

fn furnace_arrow() -> Image {
    let mut img = Image::new(24, 17);
    draw_arrow(&mut img, 0, 0, 24, 17, 9, 2, WHITE);
    img
}

// ---------------------------------------------------------------------------
// Widgets / HUD
// ---------------------------------------------------------------------------

fn button(kind: u8) -> Image {
    // (fill shades, highlight, shadow (right / bottom), deepest shadow, outline)
    let (fills, hi, sh, sh2, outline): ([u32; 3], u32, u32, u32, u32) = match kind {
        0 => ([0x6F6F6F, 0x737373, 0x7A7A7A], 0xAAAAAA, 0x565656, 0x4A4A4A, 0x000000),
        1 => ([0x7F89C2, 0x848EC7, 0x8B95CC], 0xBDC6FF, 0x5A6499, 0x4E5888, 0xFFFFFF),
        _ => ([0x2C2C2C, 0x2E2E2E, 0x313131], 0x3C3C3C, 0x252525, 0x222222, 0x000000),
    };
    let mut img = Image::filled(200, 20, rgb(outline));
    let mut rng = Rng::new(0xB077);
    for y in 1..19 {
        for x in 1..199 {
            let r = rng.range(10);
            let mut c = rgb(fills[if r < 5 { 0 } else if r < 8 { 1 } else { 2 }]);
            if y == 1 || x == 1 {
                c = rgb(hi);
            }
            if x == 198 || y == 17 {
                c = rgb(sh);
            }
            if y == 18 {
                c = rgb(sh2);
            }
            img.put(x, y, c);
        }
    }
    img
}

fn hotbar() -> Image {
    let mut img = Image::new(182, 22);
    let outer = [0x10, 0x10, 0x10, 0xE0];
    img.fill_rect(0, 0, 182, 22, outer);
    for i in 0..9 {
        let x0 = 1 + i * 20;
        // gray frame ring of the cell
        img.fill_rect(x0, 1, 20, 20, rgb(0x8B8B8B));
        img.fill_rect(x0, 1, 20, 1, rgb(0xA5A5A5));
        img.fill_rect(x0, 20, 20, 1, rgb(0x6B6B6B));
        img.fill_rect(x0, 2, 1, 18, rgb(0x9A9A9A));
        img.fill_rect(x0 + 19, 2, 1, 18, rgb(0x747474));
        // translucent slot interior
        img.fill_rect(x0 + 1, 2, 18, 18, [0x2A, 0x2A, 0x2A, 0x9C]);
        img.fill_rect(x0 + 1, 2, 18, 1, [0x10, 0x10, 0x10, 0xB0]);
        img.fill_rect(x0 + 1, 2, 1, 18, [0x10, 0x10, 0x10, 0xB0]);
    }
    img
}

fn hotbar_selection() -> Image {
    let mut img = Image::new(24, 24);
    let ring = |img: &mut Image, o: i32, c: [u8; 4]| {
        let s = 24 - 2 * o;
        img.fill_rect(o, o, s, 1, c);
        img.fill_rect(o, o + s - 1, s, 1, c);
        img.fill_rect(o, o, 1, s, c);
        img.fill_rect(o + s - 1, o, 1, s, c);
    };
    ring(&mut img, 0, [0, 0, 0, 0xC0]);
    ring(&mut img, 1, rgb(0xFFFFFF));
    ring(&mut img, 2, rgb(0xC6C6C6));
    ring(&mut img, 3, [0, 0, 0, 0x90]);
    // round the outermost corners
    for (x, y) in [(0, 0), (23, 0), (0, 23), (23, 23)] {
        img.put(x, y, [0, 0, 0, 0]);
    }
    img
}

fn crosshair() -> Image {
    let mut img = Image::new(15, 15);
    img.fill_rect(7, 0, 1, 15, WHITE);
    img.fill_rect(0, 7, 15, 1, WHITE);
    img
}

const HEART: [&str; 9] = [
    ".KKK.KKK.",
    "KphrKrrrK",
    "KhrrrrrrK",
    "KrrrrrrdK",
    ".KrrrrdK.",
    "..KrrdK..",
    "...KdK...",
    "....K....",
    ".........",
];

/// level: 0 = empty container, 1 = half, 2 = full.
fn heart(level: u8) -> Image {
    let mut img = Image::new(9, 9);
    let empty = [0x22, 0x0E, 0x0E, 0x80];
    let pal_full = [
        ('K', BLACK),
        ('r', rgb(0xE51D1D)),
        ('h', rgb(0xFFFFFF)),
        ('p', rgb(0xF6A0A0)),
        ('d', rgb(0xA80F0F)),
    ];
    let pal_empty = [('K', BLACK), ('r', empty), ('h', empty), ('p', empty), ('d', empty)];
    draw_grid(&mut img, 0, 0, &HEART, &pal_empty);
    if level > 0 {
        let mut full = Image::new(9, 9);
        draw_grid(&mut full, 0, 0, &HEART, &pal_full);
        let max_x = if level == 2 { 9 } else { 4 };
        for y in 0..9 {
            for x in 0..max_x {
                let c = full.get(x, y);
                if c[3] > 0 {
                    img.set(x, y, c);
                }
            }
        }
    }
    img
}

const SHANK: [&str; 9] = [
    "....KKKK.",
    "...KmllmK",
    "..KmlmmmK",
    "..KmmmmdK",
    "..KmmmddK",
    ".KbKmddK.",
    "KwbbKKK..",
    "KbbK.....",
    ".KK......",
];

fn food(level: u8) -> Image {
    let mut img = Image::new(9, 9);
    let empty = [0x20, 0x14, 0x0A, 0x80];
    let pal_full = [
        ('K', rgb(0x1E0F05)),
        ('m', rgb(0xB0602A)),
        ('l', rgb(0xDB8C4C)),
        ('d', rgb(0x7A3C14)),
        ('b', rgb(0xD8CFC0)),
        ('w', rgb(0xFFFFFF)),
    ];
    let pal_empty = [
        ('K', rgb(0x1E0F05)),
        ('m', empty),
        ('l', empty),
        ('d', empty),
        ('b', empty),
        ('w', empty),
    ];
    draw_grid(&mut img, 0, 0, &SHANK, &pal_empty);
    if level > 0 {
        let mut full = Image::new(9, 9);
        draw_grid(&mut full, 0, 0, &SHANK, &pal_full);
        let min_x = if level == 2 { 0 } else { 4 };
        for y in 0..9 {
            for x in min_x..9 {
                let c = full.get(x, y);
                if c[3] > 0 {
                    img.set(x, y, c);
                }
            }
        }
    }
    img
}

fn air_bubble() -> Image {
    let mut img = Image::new(9, 9);
    let rows = [
        "..BBBBB..",
        ".BiiiiiB.",
        "BiwwiiiiB",
        "BiwiiiiiB",
        "BiiiiiiiB",
        "BiiiiiilB",
        "BiiiiillB",
        ".BiiillB.",
        "..BBBBB..",
    ];
    draw_grid(
        &mut img,
        0,
        0,
        &rows,
        &[
            ('B', rgb(0x2D5FC4)),
            ('i', [0x9C, 0xC4, 0xF5, 0x90]),
            ('l', [0x5A, 0x8C, 0xE0, 0xC0]),
            ('w', rgb(0xFFFFFF)),
        ],
    );
    img
}

// ---------------------------------------------------------------------------
// Title logo
// ---------------------------------------------------------------------------

const LOGO: [&str; 5] = [
    "*   * * *   * *** *** *** *** *** ***",
    "** ** * **  * *   *   * * * * *    * ",
    "* * * * * * * **  *   **  *** **   * ",
    "*   * * *  ** *   *   * * * * *    * ",
    "*   * * *   * *** *** * * * * *    * ",
];

fn logo() -> Image {
    const W: usize = 274;
    const H: usize = 44;
    const S: i32 = 7; // pixels per logo block
    const DEPTH: i32 = 4;
    let cols = LOGO[0].len() as i32;
    let ox = (W as i32 - (cols * S + DEPTH / 2 + 2)) / 2 + 1;
    let oy = 1;
    let face = |x: i32, y: i32| -> bool {
        let (bx, by) = ((x - ox).div_euclid(S), (y - oy).div_euclid(S));
        if x < ox || y < oy || by >= 5 || bx >= cols {
            return false;
        }
        LOGO[by as usize].as_bytes().get(bx as usize) == Some(&b'*')
    };
    // extrusion layer index (1..=DEPTH) or 0
    let extr = |x: i32, y: i32| -> i32 {
        for d in 1..=DEPTH {
            if face(x - d / 2, y - d) {
                return d;
            }
        }
        0
    };
    let mut img = Image::new(W, H);
    let mut rng = Rng::new(0x10_60);
    // coarse 2x2 noise grid for a rough stone look
    let gw = W / 2 + 1;
    let noise: Vec<f32> = (0..gw * (H / 2 + 1)).map(|_| rng.f()).collect();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if face(x, y) {
                let n = noise[(y as usize / 2) * gw + x as usize / 2] * 0.6 + rng.f() * 0.4;
                let mut v: i32 = if n < 0.25 {
                    0x6C
                } else if n < 0.55 {
                    0x80
                } else if n < 0.8 {
                    0x93
                } else {
                    0xA6
                };
                if !face(x, y - 1) {
                    v = 0xD8; // lit top edge
                } else if !face(x - 1, y) || !face(x, y - 2) {
                    v = (v + 0x30).min(0xC4);
                } else if !face(x + 1, y) || !face(x, y + 1) {
                    v -= 0x18;
                }
                let v = v as u8;
                img.put(x, y, [v, v, v, 255]);
            } else {
                let d = extr(x, y);
                if d > 0 {
                    let below_face = face(x, y - d);
                    let v: u8 = if below_face { 0x46 - (d as u8) * 4 } else { 0x2E };
                    img.put(x, y, [v, v, v, 255]);
                }
            }
        }
    }
    // black outline around everything
    let src = img.clone();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if src.get(x as usize, y as usize)[3] != 0 {
                continue;
            }
            let mut near = false;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < W as i32 && ny < H as i32 && src.get(nx as usize, ny as usize)[3] != 0 {
                    near = true;
                }
            }
            if near {
                img.put(x, y, BLACK);
            }
        }
    }
    img
}

// ---------------------------------------------------------------------------
// Menu background
// ---------------------------------------------------------------------------

fn background_tile() -> Image {
    let mut img = Image::new(16, 16);
    let mut rng = Rng::new(0xD1A7);
    let pal = [0x593D29, 0x6C4A30, 0x79553A, 0x866043, 0x966C4A, 0xB9855C];
    for y in 0..16 {
        for x in 0..16 {
            let r = rng.range(100);
            let i = match r {
                0..=7 => 0,
                8..=24 => 1,
                25..=49 => 2,
                50..=79 => 3,
                80..=95 => 4,
                _ => 5,
            };
            img.set(x, y, rgb(pal[i]));
        }
    }
    // a few small darker pebbles and light specks like vanilla dirt
    for _ in 0..6 {
        let (x, y) = (rng.range(16) as i32, rng.range(16) as i32);
        img.put(x, y, rgb(0x4F3524));
        img.put((x + 1) % 16, y, rgb(0x593D29));
    }
    for _ in 0..4 {
        let (x, y) = (rng.range(16) as i32, rng.range(16) as i32);
        img.put(x, y, rgb(0xB9855C));
    }
    img
}
