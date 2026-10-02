//! Item icon textures: hand-authored 16x16 pixel art in the style of the vanilla
//! Minecraft item sprites (dark outlines, limited palettes, transparent background).
//!
//! Each sprite is a grid of 16 strings of 16 chars plus a palette mapping chars to
//! colours; `.` (and any char missing from the palette) is transparent.
//! Tools share one template per tool shape and swap in a per-material palette.
use crate::image::{rgb, Image};

pub const NAMES: &[&str] = &[
    "stick",
    "coal",
    "charcoal",
    "iron_ingot",
    "gold_ingot",
    "diamond",
    "wheat_seeds",
    "wheat",
    "bread",
    "apple",
    "porkchop",
    "cooked_porkchop",
    "beef",
    "cooked_beef",
    "chicken",
    "cooked_chicken",
    "rotten_flesh",
    "leather",
    "feather",
    "bone",
    "string",
    "gunpowder",
    "flint",
    "arrow",
    "bow",
    "bow_pulling_0",
    "bow_pulling_1",
    "bow_pulling_2",
    "bowl",
    "mushroom_stew",
    "sugar",
    "egg",
    "bucket",
    "water_bucket",
    "lava_bucket",
    "paper",
    "wooden_pickaxe",
    "wooden_axe",
    "wooden_shovel",
    "wooden_sword",
    "wooden_hoe",
    "stone_pickaxe",
    "stone_axe",
    "stone_shovel",
    "stone_sword",
    "stone_hoe",
    "iron_pickaxe",
    "iron_axe",
    "iron_shovel",
    "iron_sword",
    "iron_hoe",
    "golden_pickaxe",
    "golden_axe",
    "golden_shovel",
    "golden_sword",
    "golden_hoe",
    "diamond_pickaxe",
    "diamond_axe",
    "diamond_shovel",
    "diamond_sword",
    "diamond_hoe",
    "oak_door",
    "red_bed",
];

type Grid = [&'static str; 16];
type Pal<'a> = &'a [(char, u32)];

/// Render a char grid with a palette. Unknown chars / '.' are transparent.
fn sprite(grid: &Grid, pal: Pal) -> Image {
    let mut img = Image::new(16, 16);
    for (y, row) in grid.iter().enumerate() {
        debug_assert!(row.chars().count() == 16, "sprite row {y} is not 16 wide: {row:?}");
        for (x, ch) in row.chars().enumerate().take(16) {
            if let Some(&(_, c)) = pal.iter().find(|(k, _)| *k == ch) {
                img.set(x, y, rgb(c));
            }
        }
    }
    img
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------
// Template chars: K outline, X darkest, D dark, M mid, L light, H highlight (head
// material); o stick outline, t stick light, s stick dark.

const STICK_PAL: [(char, u32); 3] = [('o', 0x281E0B), ('t', 0x896727), ('s', 0x5E4520)];

const PICKAXE: Grid = [
    ".....KKKKKKKK...",
    "...KKHHHHHLLLK..",
    "..KHLMMMMMMMLMK.",
    ".KHMDKKKKDDMMMDK",
    "KHDKK....KDDLMDK",
    ".KK......oKLMMDK",
    "........otsKLMDK",
    ".......otso.KLDK",
    "......otso..KLDK",
    ".....otso...KLDK",
    "....otso....KLDK",
    "...otso....KLMK.",
    "..otso.....KLDK.",
    ".otso.....KLDK..",
    "otso......KDK...",
    ".oo........K....",
];

const AXE: Grid = [
    ".....KKKK.......",
    "....KHLLLKK..oo.",
    "...KHLMMMLLKotso",
    "...KLMMMMMMotso.",
    "...KLMMMMDotso..",
    "...KLMMMDotso...",
    "...KLMMDotso....",
    "...KMDDotso.....",
    "....KKotso......",
    ".....otso.......",
    "....otso........",
    "...otso.........",
    "..otso..........",
    ".otso...........",
    "otso............",
    ".oo.............",
];

const SHOVEL: Grid = [
    "...........KKKK.",
    "..........KHLLLK",
    ".........KHLLLMK",
    "........KHLLMMDK",
    "........KLLMMDDK",
    "........KLMMDDK.",
    "........KMMDDK..",
    ".......otKKKK...",
    "......otso......",
    ".....otso.......",
    "....otso........",
    "...otso.........",
    "..otso..........",
    ".otso...........",
    "otso............",
    ".oo.............",
];

const SWORD: Grid = [
    "............KKK.",
    "...........KLHHK",
    "..........KLHMDK",
    ".........KLHMDK.",
    "........KLHMDK..",
    ".......KLHMDK...",
    "..KK..KLHMDK....",
    ".KDXKKLHMDK.....",
    "..KDXKHMDK......",
    "...KDXKDK.......",
    "....oKDXK.......",
    "...otsKDXK......",
    "..otso.KK.......",
    ".otso...........",
    "KDXo............",
    "KKK.............",
];

const HOE: Grid = [
    ".....KKKKKKK....",
    "....KHLLLLLMKoo.",
    "....KMDDDDDDotso",
    "....KDKKKKKotso.",
    "....KK....otso..",
    ".........otso...",
    "........otso....",
    ".......otso.....",
    "......otso......",
    ".....otso.......",
    "....otso........",
    "...otso.........",
    "..otso..........",
    ".otso...........",
    "otso............",
    ".oo.............",
];

/// [K, X, D, M, L, H]
fn material(m: &str) -> Option<[u32; 6]> {
    Some(match m {
        "wooden" => [0x281E0B, 0x493615, 0x6B511F, 0x896727, 0xA8834A, 0xC29D62],
        "stone" => [0x1E1E1E, 0x404040, 0x5A5A5A, 0x747474, 0x8F8F8F, 0xA9A9A9],
        "iron" => [0x2A2A2A, 0x4E4E4E, 0x727272, 0xA8A8A8, 0xD8D8D8, 0xFFFFFF],
        "golden" => [0x3D2A04, 0x8A5A0E, 0xD2981A, 0xEAC72A, 0xFEF34A, 0xFFFFB5],
        "diamond" => [0x0A3530, 0x0E6D63, 0x1AAA95, 0x2BC7AC, 0x4AEDD9, 0xC8FFF4],
        _ => return None,
    })
}

fn tool(mat: &str, kind: &str) -> Option<Image> {
    let grid = match kind {
        "pickaxe" => &PICKAXE,
        "axe" => &AXE,
        "shovel" => &SHOVEL,
        "sword" => &SWORD,
        "hoe" => &HOE,
        _ => return None,
    };
    let [k, x, d, m, l, h] = material(mat)?;
    let pal = [
        ('K', k),
        ('X', x),
        ('D', d),
        ('M', m),
        ('L', l),
        ('H', h),
        STICK_PAL[0],
        STICK_PAL[1],
        STICK_PAL[2],
    ];
    Some(sprite(grid, &pal))
}

// ---------------------------------------------------------------------------
// Materials & misc
// ---------------------------------------------------------------------------

const STICK: Grid = [
    "................",
    ".............oo.",
    "............otso",
    "...........otso.",
    "..........otso..",
    ".........otso...",
    "........otso....",
    ".......otso.....",
    "......otso......",
    ".....otso.......",
    "....otso........",
    "...otso.........",
    "..otso..........",
    ".otso...........",
    "..oo............",
    "................",
];

const COAL: Grid = [
    "................",
    "................",
    ".....KK.........",
    "....KLdKK.KK....",
    "...KLddLdKdLK...",
    "..KLdddDdddddK..",
    "..KdLddddLddDK..",
    ".KLddDddddddDK..",
    ".KddddddLdDDdDK.",
    "..KdLddddddddDK.",
    "..KddDdddLdDDK..",
    ".KdddddDdddDK...",
    ".KDddDDddDDK....",
    "..KKDDKKDDK.....",
    "....KK..KK......",
    "................",
];

const INGOT: Grid = [
    "................",
    "................",
    "................",
    "................",
    "........KKKKKK..",
    ".......KHHHHHLK.",
    "......KHLLLLLMK.",
    ".....KHLLLLLMDK.",
    "....KHLLLLLMDDK.",
    "...KKKKKKKKKDDK.",
    "...KMMMMMMMKDK..",
    "...KDDDDDDDKK...",
    "...KKKKKKKKK....",
    "................",
    "................",
    "................",
];

const DIAMOND: Grid = [
    "................",
    "................",
    ".....KKKKKK.....",
    "....KHHLLLMK....",
    "...KHLHLLMLMK...",
    "..KHLLLHLLMMDK..",
    "..KLLLLMLMMMDK..",
    "...KLLLMLMMDK...",
    "....KLLMMMDK....",
    ".....KLMMDK.....",
    "......KMDK......",
    ".......KK.......",
    "................",
    "................",
    "................",
    "................",
];

const WHEAT_SEEDS: Grid = [
    "................",
    "................",
    "................",
    "................",
    ".......KK.......",
    "......KGgK..KK..",
    ".......KK..KGgK.",
    "..KK........KK..",
    ".KGgK...KK......",
    "..KK...KGgK.....",
    "........KK...KK.",
    "....KK......KGgK",
    "...KGgK......KK.",
    "....KK..........",
    "................",
    "................",
];

const WHEAT: Grid = [
    "........KK......",
    ".......KYyK.KK..",
    ".......KydKKYyK.",
    "......KYyKKYydK.",
    "......KydKydK...",
    ".....KYyKKYyK.KK",
    ".....KydKydK.KYK",
    ".....KsKYyK.KYyK",
    "....KsKsKKKKyYdK",
    "....KsKsKKyYdKK.",
    "...KssKKsYdKK...",
    "...KssssKKK.....",
    "..KsssKK........",
    "..KttKK.........",
    ".KstK...........",
    "KsKK............",
];

const BREAD: Grid = [
    "................",
    "................",
    "................",
    "..........KKK...",
    "........KKbBbK..",
    "......KKbBbbbbK.",
    ".....KbBbbcbbbK.",
    "....KbBbbcbbbdK.",
    "...KbbbcbbbbdK..",
    "..KbBbcbbbbddK..",
    "..KbbcbbbbddK...",
    ".KbbbbbbdddK....",
    ".KbbbbdddKK.....",
    "..KddddKK.......",
    "...KKKK.........",
    "................",
];

const APPLE: Grid = [
    "................",
    "........K.......",
    ".......KsK.KK...",
    ".......KsKKgGK..",
    "...KKKKKsKggK...",
    "..KrRRrrKKKrrK..",
    ".KrRWRrrrrrrrdK.",
    ".KrRRrrrrrrrrdK.",
    ".KrRrrrrrrrrrdK.",
    ".KrrrrrrrrrrddK.",
    ".KrrrrrrrrrrddK.",
    "..KrrrrrrrrddK..",
    "..KdrrrrrrdddK..",
    "...KddKKKddK....",
    "....KK...KK.....",
    "................",
];

const PORKCHOP: Grid = [
    "................",
    "................",
    "................",
    "........KKKK....",
    "......KKpPPpKK..",
    ".....KpPWWPppK..",
    "....KpPWPPppppK.",
    "...KpPPPpppppdK.",
    "...KpPpppppppdK.",
    "..KpPppppppdddK.",
    "..KppppppppdddK.",
    ".KWWppppppddK...",
    ".KWWWpppdddK....",
    "..KWWWddKKK.....",
    "...KKKKK........",
    "................",
];

const BEEF: Grid = [
    "................",
    "................",
    "................",
    ".....KKKKK......",
    "....KrrRrrKKK...",
    "...KrRfRrrrrrK..",
    "..KrrfrrrRrrrrK.",
    "..KrRrrrfrrrRrK.",
    ".KrrrrRfrrrrrdK.",
    ".KrfrrrrrrRrrdK.",
    ".KrrrRrrfrrrddK.",
    "..KrrrrrfrrddK..",
    "..KdrrrrrrddK...",
    "...KddddddKK....",
    "....KKKKKK......",
    "................",
];

const CHICKEN: Grid = [
    "................",
    ".KK..KK.........",
    "KwwKKwwK........",
    "KwwKKwwK........",
    ".KbK.KbK........",
    "..KbK.KbK.......",
    "...KbKKbKKKK....",
    "...KpPPpPPppKK..",
    "..KpPPpPpppppPK.",
    "..KpPppppppppppK",
    "..KpppppppppppdK",
    "...KpppppppppddK",
    "...KdpppppppddK.",
    "....KddppppddK..",
    ".....KKdddddK...",
    "......KKKKKK....",
];

const ROTTEN_FLESH: Grid = [
    "................",
    "................",
    "......KKK.......",
    "....KKgGgKK.....",
    "...KgGgrgggKK...",
    "..KggrrgGgggdK..",
    "..KgGgggrgggdK..",
    ".KggggGggrrgddK.",
    ".KgrrgggggggdK..",
    "..KggggGgrgddK..",
    "..KgGgggggdddK..",
    "...KgggrgddK....",
    "....KdddddK.....",
    ".....KKKKK......",
    "................",
    "................",
];

const LEATHER: Grid = [
    "................",
    "................",
    "...KKK....KKK...",
    "..KbbbKKKKbbbK..",
    "..KbBbbbbbbBbK..",
    "...KbbbBbbbbK...",
    "...KbBbbbbbbK...",
    "..KbbbbbBbbbbK..",
    "..KbBbbbbbbbbK..",
    "..KbbbbbbbBbdK..",
    "...KbbBbbbbdK...",
    "...KbbbbbbbdK...",
    "..KbbbdKKKbddK..",
    "..KddKK...KKdK..",
    "...KK.......K...",
    "................",
];

const FEATHER: Grid = [
    "...........KKKK.",
    "..........KwwwgK",
    ".........KwwwglK",
    "........KwwwglK.",
    ".......KwwwglK..",
    "......KwwwglK...",
    ".....KwwwglK....",
    "....KwwwglK.....",
    "...KwwwglK......",
    "...KwwglK.......",
    "...KwglK........",
    "...KglK.........",
    "..KgK...........",
    ".KgK............",
    "KgK.............",
    "K...............",
];

const BONE: Grid = [
    "...........KK...",
    "..........KwwK..",
    "..........KwwwKK",
    "........KKwwwwwK",
    ".......KwwlKKwlK",
    "......KwwlK..KK.",
    ".....KwwlK......",
    "....KwwlK.......",
    "...KwwlK........",
    "..KwwlK.........",
    ".KwwlK..........",
    "KK.KlK..........",
    "KwKwlK..........",
    "KwwwlK..........",
    ".KwwK...........",
    "..KK............",
];

const STRING: Grid = [
    "................",
    "................",
    "..........gww...",
    ".........w...w..",
    "........w.....w.",
    ".......w......w.",
    "......w......w..",
    ".....w.....ww...",
    "....w...ww......",
    "...w..ww........",
    "..w.ww..........",
    ".w.w............",
    "w..w............",
    "...w............",
    "....w...........",
    "................",
];

const GUNPOWDER: Grid = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "......K.K.......",
    ".....KgKgK..K...",
    "...K.KgGgKKgK...",
    "..KgKgGgGgggKK..",
    ".KgGgggdgGgdgK..",
    ".KgdgGgggdgggdK.",
    "KggGgdggGggdgggK",
    "KgdgggdgggggdggK",
    ".KKKKKKKKKKKKKK.",
    "................",
];

const FLINT: Grid = [
    "................",
    "................",
    ".......KK.......",
    "......KhlK......",
    ".....KhlllK.....",
    "....KhllldlK....",
    "....KlhllddlK...",
    "...KhlldlllddK..",
    "...KllddlldddK..",
    "..KhlldddlddddK.",
    "..KlldddddddddK.",
    "..KldddddddddK..",
    "...KdddddddKK...",
    "....KKKKKKK.....",
    "................",
    "................",
];

const ARROW: Grid = [
    "..........KKKKK.",
    "..........KhhhgK",
    "...........KhhgK",
    "..........sbKhgK",
    ".........sb..KgK",
    "........sb....K.",
    ".......sb.......",
    "......sb........",
    ".....sb.........",
    "..w.sb..........",
    ".wwsb...........",
    ".wsbw...........",
    ".sbww...........",
    "s.gw............",
    "..g.............",
    "................",
];

const BOW: Grid = [
    "...KKK..........",
    "..KBBBKK........",
    "..KbbbBBK.......",
    "...KsKbbBKK.....",
    ".....sKKbBBK....",
    "......s.KgBBK...",
    ".......sKGgBK...",
    "........sKKbBK..",
    ".........s.KbBK.",
    "..........sKbBK.",
    "...........sKbBK",
    "............sbBK",
    "............KbBK",
    ".............KK.",
    "................",
    "................",
];

/// Bow being drawn (vanilla `bow_pulling_0..2`): same limb as BOW, the string pulled
/// back into a V towards the bottom-left with an arrow nocked along the diagonal.
/// Extra chars: t/d arrow shaft light/dark, h/i/k arrowhead light/dark/outline, w fletching.
const BOW_PULLING: [Grid; 3] = [
    [
        "...KKK..........",
        "..KBBBKK...kkk..",
        "..KbbbBBK.khhhk.",
        "...KsKbbBKKkhik.",
        "....s.KKbBBtdik.",
        ".....s..KgtdKk..",
        ".....s..KtdBK...",
        ".....sw.tdKbBK..",
        ".....wstd..KbBK.",
        "......tdw..KbBK.",
        ".....tdwsss.KbBK",
        "...........ssbBK",
        "............KbBK",
        ".............KK.",
        "................",
        "................",
    ],
    [
        "...KKK..........",
        "..KBBBKK........",
        "..KbbbBBK.kkk...",
        "...KsKbbBKhhhk..",
        "....s.KKbBBhik..",
        "....s...Kgtdik..",
        "....s...KtdBK...",
        ".....s..tdKbBK..",
        ".....w.td..KbBK.",
        "....wstd...KbBK.",
        ".....tdws...KbBK",
        "....tdw..ssssbBK",
        "............KbBK",
        ".............KK.",
        "................",
        "................",
    ],
    [
        "...KKK..........",
        "..KBBBKK........",
        "..KbbbBBK.......",
        "...KsKbbBKKk....",
        "....s.KKbhhhk...",
        "....s...KghiK...",
        "....s...KtdiK...",
        "....s...tdKbBK..",
        "....s..td..KbBK.",
        "....w.td...KbBK.",
        "...wstd.....KbBK",
        "....tdwssssssbBK",
        "...tdw......KbBK",
        ".............KK.",
        "................",
        "................",
    ],
];

const BOW_PAL: [(char, u32); 12] = [
    ('k', 0x2A2A2A),
    ('K', 0x281E0B),
    ('B', 0x6B511F),
    ('b', 0x9C7A3C),
    ('g', 0x8C8C8C),
    ('G', 0x5A5A5A),
    ('s', 0xD8D8D8),
    ('t', 0x896727),
    ('d', 0x4A3618),
    ('h', 0xD0D0D0),
    ('i', 0x8A8A8A),
    ('w', 0xEDEDED),
];

const BOWL: Grid = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "..KKKKKKKKKKKK..",
    ".KBbijiiijiiibK.",
    "KBbbbiijiiiibbdK",
    "KBbbbbbbbbbbbbdK",
    ".KBbbbbbbbbbbdK.",
    "..KbbbbbbbbbddK.",
    "...KbbbbbbbddK..",
    "....KKddddddK...",
    ".....KKKKKK.....",
    "................",
];

const SUGAR: Grid = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    ".......KK.......",
    "......KwwK......",
    ".....KwwgwK.....",
    "....KwgwwwwK....",
    "...KwwwwgwwgK...",
    "..KwgwwwwwwwwK..",
    ".KwwwwgwwwgwwwK.",
    "KwwgwwwwwwwwgwwK",
    "KgwwwwgwwgwwwwgK",
    ".KKKKKKKKKKKKKK.",
];

const EGG: Grid = [
    "................",
    "................",
    "......KKKK......",
    ".....KwweeK.....",
    "....KwEeeeeK....",
    "....KEeeeseK....",
    "...KEeeeeeedK...",
    "...KEeseeeedK...",
    "...KeeeseeedK...",
    "...KeeeeeeedK...",
    "...KeseeeeddK...",
    "....KeeeeddK....",
    "....KdeeeddK....",
    ".....KddddK.....",
    "......KKKK......",
    "................",
];

const BUCKET: Grid = [
    "................",
    "................",
    "................",
    "....KKKKKKKK....",
    "..KKHHLLLLLMKK..",
    ".KHLiijiiiiiMMK.",
    ".KLiijiiiijiiMK.",
    ".KHLiiiiijiiMDK.",
    ".KHHLLLLLLMMMDK.",
    "..KHLLLLLLMMDK..",
    "..KHLLLLLMMMDK..",
    "..KHLLLLLMMDDK..",
    "...KHLLLMMMDK...",
    "...KHLLMMMDDK...",
    "....KKKKKKKK....",
    "................",
];

const PAPER: Grid = [
    "................",
    "................",
    "..KKKKKKKKKK....",
    "..KwwwwwwwwwKK..",
    ".KwwwwwwwwwwwwK.",
    ".KwwwwwwwwwwwgK.",
    ".KwwwwwwwwwwwgK.",
    ".KwwwwwwwwwwwgK.",
    "KwwwwwwwwwwwwgK.",
    "KwwwwwwwwwwwgK..",
    "KwwwwwwwwwwwgK..",
    "KwwwwwwwwwwwgK..",
    ".KKgwwwwwwwwgK..",
    "...KKKgggggggK..",
    "......KKKKKKK...",
    "................",
];

const OAK_DOOR: Grid = [
    "................",
    "....KKKKKKKK....",
    "....KhlllllK....",
    "....Kl..m..K....",
    "....Kl..m..K....",
    "....Kl..m..K....",
    "....KlmmmmdK....",
    "....KlmdmmdK....",
    "....KlmdmmdK....",
    "....KlmdmmiK....",
    "....KlmdmmIK....",
    "....KlmdmmdK....",
    "....KlmdmmdK....",
    "....KlmdmmdK....",
    "....KlddddDK....",
    "....KKKKKKKK....",
];

const RED_BED: Grid = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "......KKKKKKKKK.",
    ".....KWWWHRRRRK.",
    "....KWWwHRRRRdK.",
    "...KwwgHRRRRddK.",
    "..KgggrrrrrrddK.",
    "..KgggSSSSSSddK.",
    "..KbbbbbbbbbBBK.",
    "..KbKKKKKKKbKbK.",
    "..KKK.....KKKKK.",
    "................",
    "................",
];

pub fn get(name: &str) -> Option<Image> {
    if let Some((mat, kind)) = name.rsplit_once('_') {
        if let Some(img) = tool(mat, kind) {
            return Some(img);
        }
    }
    let img = match name {
        "stick" => sprite(&STICK, &STICK_PAL),
        "coal" => sprite(
            &COAL,
            &[('K', 0x0A0A0A), ('D', 0x1A1A1A), ('d', 0x2C2C2C), ('L', 0x555555)],
        ),
        "charcoal" => sprite(
            &COAL,
            &[('K', 0x120E08), ('D', 0x2B2219), ('d', 0x3F3427), ('L', 0x66553F)],
        ),
        "iron_ingot" => sprite(
            &INGOT,
            &[
                ('K', 0x353535),
                ('D', 0x7A7A7A),
                ('M', 0xA8A8A8),
                ('L', 0xD8D8D8),
                ('H', 0xFFFFFF),
            ],
        ),
        "gold_ingot" => sprite(
            &INGOT,
            &[
                ('K', 0x5A3A06),
                ('D', 0xB57312),
                ('M', 0xDC9D1A),
                ('L', 0xFADD4C),
                ('H', 0xFFFFB5),
            ],
        ),
        "diamond" => sprite(
            &DIAMOND,
            &[
                ('K', 0x0C3B33),
                ('D', 0x1A9C8B),
                ('M', 0x2BC7AC),
                ('L', 0x4AEDD9),
                ('H', 0xD5FFF6),
            ],
        ),
        "wheat_seeds" => sprite(
            &WHEAT_SEEDS,
            &[('K', 0x23400F), ('g', 0x3F7A1F), ('G', 0x7FC04A)],
        ),
        "wheat" => sprite(
            &WHEAT,
            &[
                ('K', 0x3D2D0C),
                ('Y', 0xE6C65A),
                ('y', 0xC9A23A),
                ('d', 0x9C7A22),
                ('s', 0x8C8A2E),
                ('t', 0xB59A4A),
            ],
        ),
        "bread" => sprite(
            &BREAD,
            &[
                ('K', 0x3D2309),
                ('b', 0xA9692B),
                ('B', 0xC98A43),
                ('c', 0xE6BB79),
                ('d', 0x7A4815),
            ],
        ),
        "apple" => sprite(
            &APPLE,
            &[
                ('K', 0x3A0808),
                ('s', 0x5E3B12),
                ('g', 0x2F7A16),
                ('G', 0x5DB52A),
                ('r', 0xD8202B),
                ('R', 0xF05A5A),
                ('W', 0xFFD8D8),
                ('d', 0x99121C),
            ],
        ),
        "porkchop" => sprite(
            &PORKCHOP,
            &[
                ('K', 0x8A3A3A),
                ('p', 0xEC8F8F),
                ('P', 0xF6B6B6),
                ('W', 0xFFE4E4),
                ('d', 0xCC6464),
            ],
        ),
        "cooked_porkchop" => sprite(
            &PORKCHOP,
            &[
                ('K', 0x4A2A13),
                ('p', 0xB5784A),
                ('P', 0xD29C6A),
                ('W', 0xE7C393),
                ('d', 0x8A5530),
            ],
        ),
        "beef" => sprite(
            &BEEF,
            &[
                ('K', 0x4F0D0D),
                ('r', 0xC8302C),
                ('R', 0xE0574F),
                ('f', 0xF3B6A8),
                ('d', 0x8E1D1D),
            ],
        ),
        "cooked_beef" => sprite(
            &BEEF,
            &[
                ('K', 0x2A160B),
                ('r', 0x7A4A2C),
                ('R', 0x96603A),
                ('f', 0xB4824F),
                ('d', 0x522E19),
            ],
        ),
        "chicken" => sprite(
            &CHICKEN,
            &[
                ('K', 0x7A4A3C),
                ('p', 0xF2C4B3),
                ('P', 0xFDE0D3),
                ('d', 0xD89A88),
                ('b', 0xE7B09C),
                ('w', 0xF4F4F4),
            ],
        ),
        "cooked_chicken" => sprite(
            &CHICKEN,
            &[
                ('K', 0x4A2A12),
                ('p', 0xC88A4A),
                ('P', 0xE0A865),
                ('d', 0x8D5427),
                ('b', 0xB27740),
                ('w', 0xF0E6D0),
            ],
        ),
        "rotten_flesh" => sprite(
            &ROTTEN_FLESH,
            &[
                ('K', 0x2F1D0E),
                ('g', 0x9A5E36),
                ('G', 0xBA7C4E),
                ('r', 0x6E8A2E),
                ('d', 0x5C3A1D),
            ],
        ),
        "leather" => sprite(
            &LEATHER,
            &[('K', 0x45200F), ('b', 0xB4572E), ('B', 0xD06D3D), ('d', 0x7F3A1D)],
        ),
        "feather" => sprite(
            &FEATHER,
            &[('K', 0x5A5A5A), ('w', 0xF4F4F4), ('g', 0xA8A8A8), ('l', 0xD0D0D0)],
        ),
        "bone" => sprite(&BONE, &[('K', 0x5E5A4E), ('w', 0xF4F1E6), ('l', 0xC9C3AE)]),
        "string" => sprite(&STRING, &[('w', 0xEDEDED), ('g', 0xB0B0B0)]),
        "gunpowder" => sprite(
            &GUNPOWDER,
            &[('K', 0x262626), ('g', 0x5E5E5E), ('G', 0x8E8E8E), ('d', 0x3C3C3C)],
        ),
        "flint" => sprite(
            &FLINT,
            &[('K', 0x0E0E0E), ('h', 0x8C8C8C), ('l', 0x5A5A5A), ('d', 0x2F2F2F)],
        ),
        "arrow" => sprite(
            &ARROW,
            &[
                ('K', 0x2A2A2A),
                ('h', 0xB8B8B8),
                ('g', 0x6E6E6E),
                ('s', 0x896727),
                ('b', 0x4A3618),
                ('w', 0xEDEDED),
            ],
        ),
        "bow" => sprite(&BOW, &BOW_PAL),
        "bow_pulling_0" => sprite(&BOW_PULLING[0], &BOW_PAL),
        "bow_pulling_1" => sprite(&BOW_PULLING[1], &BOW_PAL),
        "bow_pulling_2" => sprite(&BOW_PULLING[2], &BOW_PAL),
        "bowl" => sprite(
            &BOWL,
            &[
                ('K', 0x2E1E0C),
                ('B', 0xA07A45),
                ('b', 0x7E5A2C),
                ('d', 0x5A3F1C),
                ('i', 0x3E2A12),
                ('j', 0x3E2A12),
            ],
        ),
        "mushroom_stew" => sprite(
            &BOWL,
            &[
                ('K', 0x2E1E0C),
                ('B', 0xA07A45),
                ('b', 0x7E5A2C),
                ('d', 0x5A3F1C),
                ('i', 0xC69B6D),
                ('j', 0x8E5B3A),
            ],
        ),
        "sugar" => sprite(&SUGAR, &[('K', 0xA8A8A8), ('w', 0xFFFFFF), ('g', 0xDADADA)]),
        "egg" => sprite(
            &EGG,
            &[
                ('K', 0x6B5A33),
                ('w', 0xFFFDF2),
                ('E', 0xF7ECC8),
                ('e', 0xE6D5A4),
                ('s', 0xB59D64),
                ('d', 0xC7B17C),
            ],
        ),
        "bucket" | "water_bucket" | "lava_bucket" => {
            let (i, j) = match name {
                "water_bucket" => (0x2B4CD8, 0x5577F0),
                "lava_bucket" => (0xE05A0C, 0xFFB52A),
                _ => (0x3C3C3C, 0x3C3C3C),
            };
            sprite(
                &BUCKET,
                &[
                    ('K', 0x2A2A2A),
                    ('D', 0x6E6E6E),
                    ('M', 0x969696),
                    ('L', 0xC4C4C4),
                    ('H', 0xE8E8E8),
                    ('i', i),
                    ('j', j),
                ],
            )
        }
        "paper" => sprite(&PAPER, &[('K', 0x8F8F80), ('w', 0xF2F2E6), ('g', 0xC8C8B8)]),
        "oak_door" => sprite(
            &OAK_DOOR,
            &[
                ('K', 0x3A2A14),
                ('D', 0x5E4729),
                ('d', 0x7A5F38),
                ('m', 0x9C7F4E),
                ('l', 0xB8945F),
                ('h', 0xC9A66E),
                ('I', 0x2A2A2A),
                ('i', 0x8A8A8A),
            ],
        ),
        "red_bed" => sprite(
            &RED_BED,
            &[
                ('K', 0x2A0A08),
                ('W', 0xFFFFFF),
                ('w', 0xE0E0E0),
                ('g', 0xB8B8B8),
                ('H', 0xD8524A),
                ('R', 0xC23A33),
                ('r', 0x9A221D),
                ('S', 0x8A1C18),
                ('d', 0x6A1410),
                ('b', 0xAD8A55),
                ('B', 0x7A5F38),
            ],
        ),
        _ => return None,
    };
    Some(img)
}
