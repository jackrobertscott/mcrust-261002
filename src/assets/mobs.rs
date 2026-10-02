//! mobs textures (procedurally generated pixel art).
//!
//! Entity skins follow the vanilla Java model UV layouts (see docs/ASSETS.md):
//! every model box is unwrapped with the standard box-unwrap rule and each face
//! is painted with a base colour + seeded noise, then iconic details (faces,
//! patches, bones...) are hand-authored as small character grids.
use crate::image::{rgb, Image, Rng};

pub const NAMES: &[&str] = &[
    "pig", "cow", "sheep", "sheep_fur", "chicken", "zombie", "steve", "skeleton", "creeper",
    "spider", "sun", "moon", "clouds",
];

pub fn get(name: &str) -> Option<Image> {
    Some(match name {
        "pig" => pig(),
        "cow" => cow(),
        "sheep" => sheep(),
        "sheep_fur" => sheep_fur(),
        "chicken" => chicken(),
        "zombie" => zombie(),
        "steve" => steve(),
        "skeleton" => skeleton(),
        "creeper" => creeper(),
        "spider" => spider(),
        "sun" => sun(),
        "moon" => moon(),
        "clouds" => clouds(),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// (x, y, w, h)
type Rect = (i32, i32, i32, i32);

/// The six unwrapped faces of a model box.
#[derive(Clone, Copy)]
struct BoxUv {
    top: Rect,
    bottom: Rect,
    right: Rect,
    front: Rect,
    left: Rect,
    back: Rect,
}

impl BoxUv {
    fn all(&self) -> [Rect; 6] {
        [self.top, self.bottom, self.right, self.front, self.left, self.back]
    }
    /// The four side faces (right, front, left, back) in strip order.
    fn sides(&self) -> [Rect; 4] {
        [self.right, self.front, self.left, self.back]
    }
}

/// Box unwrap for texture offset (u,v) and size (w,h,d).
fn uv(u: i32, v: i32, w: i32, h: i32, d: i32) -> BoxUv {
    BoxUv {
        top: (u + d, v, w, d),
        bottom: (u + d + w, v, w, d),
        right: (u, v + d, d, h),
        front: (u + d, v + d, w, h),
        left: (u + d + w, v + d, d, h),
        back: (u + d + w + d, v + d, w, h),
    }
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Multiply the RGB of `c` by `k`.
fn shade(c: [u8; 4], k: f32) -> [u8; 4] {
    [clamp8(c[0] as f32 * k), clamp8(c[1] as f32 * k), clamp8(c[2] as f32 * k), c[3]]
}

/// Random brightness variation of +-amt.
fn vary(c: [u8; 4], amt: f32, rng: &mut Rng) -> [u8; 4] {
    shade(c, 1.0 + (rng.f() * 2.0 - 1.0) * amt)
}

/// Fill a rect with a base colour plus per-pixel noise.
fn noise_rect(img: &mut Image, r: Rect, base: [u8; 4], amt: f32, rng: &mut Rng) {
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            img.put(x, y, vary(base, amt, rng));
        }
    }
}

fn noise_box(img: &mut Image, b: &BoxUv, base: [u8; 4], amt: f32, rng: &mut Rng) {
    for r in b.all() {
        noise_rect(img, r, base, amt, rng);
    }
}

/// Fill a rect choosing each pixel randomly from a weighted palette.
fn pal_rect(img: &mut Image, r: Rect, pal: &[([u8; 4], u32)], rng: &mut Rng) {
    let total: u32 = pal.iter().map(|p| p.1).sum();
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            let mut k = rng.range(total);
            let mut c = pal[0].0;
            for &(pc, wgt) in pal {
                if k < wgt {
                    c = pc;
                    break;
                }
                k -= wgt;
            }
            img.put(x, y, c);
        }
    }
}

/// Paint a character grid at (x,y). Characters not in the palette are skipped.
fn grid(img: &mut Image, x: i32, y: i32, rows: &[&str], pal: &[(char, [u8; 4])]) {
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            if let Some(&(_, c)) = pal.iter().find(|p| p.0 == ch) {
                img.put(x + i as i32, y + j as i32, c);
            }
        }
    }
}

/// Like `grid`, but each palette colour gets a little noise.
fn grid_n(img: &mut Image, x: i32, y: i32, rows: &[&str], pal: &[(char, [u8; 4])], amt: f32, rng: &mut Rng) {
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            if let Some(&(_, c)) = pal.iter().find(|p| p.0 == ch) {
                img.put(x + i as i32, y + j as i32, vary(c, amt, rng));
            }
        }
    }
}

/// Paint a grid mirrored horizontally.
fn grid_n_flip(img: &mut Image, x: i32, y: i32, rows: &[&str], pal: &[(char, [u8; 4])], amt: f32, rng: &mut Rng) {
    for (j, row) in rows.iter().enumerate() {
        let n = row.chars().count() as i32;
        for (i, ch) in row.chars().enumerate() {
            if let Some(&(_, c)) = pal.iter().find(|p| p.0 == ch) {
                img.put(x + n - 1 - i as i32, y + j as i32, vary(c, amt, rng));
            }
        }
    }
}

/// Copy rect `src` to (dx,dy), optionally flipped horizontally.
fn copy_rect(img: &mut Image, src: Rect, dx: i32, dy: i32, flip: bool) {
    let mut tmp = Vec::new();
    for y in 0..src.3 {
        for x in 0..src.2 {
            tmp.push(img.get((src.0 + x) as usize, (src.1 + y) as usize));
        }
    }
    for y in 0..src.3 {
        for x in 0..src.2 {
            let sx = if flip { src.2 - 1 - x } else { x };
            img.put(dx + x, dy + y, tmp[(y * src.2 + sx) as usize]);
        }
    }
}

/// Paint the mirror image of box `src` into box `dst` (same size): used for
/// the left limbs of 64x64 player-style skins.
fn mirror_box(img: &mut Image, src: &BoxUv, dst: &BoxUv) {
    copy_rect(img, src.top, dst.top.0, dst.top.1, true);
    copy_rect(img, src.bottom, dst.bottom.0, dst.bottom.1, true);
    copy_rect(img, src.front, dst.front.0, dst.front.1, true);
    copy_rect(img, src.back, dst.back.0, dst.back.1, true);
    copy_rect(img, src.left, dst.right.0, dst.right.1, true);
    copy_rect(img, src.right, dst.left.0, dst.left.1, true);
}

/// Fill rows [y0, y1) of every side face of a box (a horizontal "band").
fn band(img: &mut Image, b: &BoxUv, y0: i32, y1: i32, base: [u8; 4], amt: f32, rng: &mut Rng) {
    for r in b.sides() {
        let ys = r.1 + y0;
        let ye = (r.1 + y1).min(r.1 + r.3);
        if ye > ys {
            noise_rect(img, (r.0, ys, r.2, ye - ys), base, amt, rng);
        }
    }
}

/// Random blotches (soft-edged circles) of `c` inside rect `r`.
fn blotches(img: &mut Image, r: Rect, c: [u8; 4], n: u32, rmin: f32, rmax: f32, amt: f32, rng: &mut Rng) {
    for _ in 0..n {
        let cx = r.0 as f32 + rng.f() * r.2 as f32;
        let cy = r.1 as f32 + rng.f() * r.3 as f32;
        let rad = rmin + rng.f() * (rmax - rmin);
        let sx = 0.7 + rng.f() * 0.6;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                let dx = (x as f32 + 0.5 - cx) / sx;
                let dy = (y as f32 + 0.5 - cy) * sx;
                let d = (dx * dx + dy * dy).sqrt();
                if d < rad - 0.3 + rng.f() * 0.6 {
                    img.put(x, y, vary(c, amt, rng));
                }
            }
        }
    }
}

/// Sprinkle pixels of `c` with probability `p`.
fn speckle(img: &mut Image, r: Rect, c: [u8; 4], p: f32, amt: f32, rng: &mut Rng) {
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            if rng.f() < p {
                img.put(x, y, vary(c, amt, rng));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// pig
// ---------------------------------------------------------------------------

fn pig() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(101);
    let pink = rgb(0xF0A5A2);
    let dark = rgb(0xE08C89);

    let head = uv(0, 0, 8, 8, 8);
    noise_box(&mut img, &head, pink, 0.035, &mut rng);
    for r in [head.top, head.right, head.left, head.back] {
        speckle(&mut img, r, dark, 0.12, 0.03, &mut rng);
    }
    let (fx, fy, _, _) = head.front;
    grid(
        &mut img,
        fx,
        fy,
        &[
            "........", "........", "........", //
            "WB....BW", "........", "........", "........", "........",
        ],
        &[('W', rgb(0xFFFFFF)), ('B', rgb(0x000000))],
    );
    // ears darker hint on the head top edges
    speckle(&mut img, head.bottom, dark, 0.3, 0.03, &mut rng);

    let snout = uv(16, 16, 4, 3, 1);
    noise_box(&mut img, &snout, rgb(0xF6B8B5), 0.03, &mut rng);
    let (sx, sy, _, _) = snout.front;
    grid(&mut img, sx, sy, &["....", "N..N", "...."], &[('N', rgb(0x7A3634))]);

    let body = uv(28, 8, 10, 16, 8);
    noise_box(&mut img, &body, pink, 0.035, &mut rng);
    for r in body.all() {
        speckle(&mut img, r, dark, 0.1, 0.04, &mut rng);
    }

    let leg = uv(0, 16, 4, 6, 4);
    noise_box(&mut img, &leg, pink, 0.035, &mut rng);
    band(&mut img, &leg, 5, 6, rgb(0xC97571), 0.04, &mut rng);
    noise_rect(&mut img, leg.bottom, rgb(0x9C5A57), 0.05, &mut rng);
    img
}

// ---------------------------------------------------------------------------
// cow
// ---------------------------------------------------------------------------

fn cow() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(202);
    let dk = rgb(0x45342A);
    let wh = rgb(0xE6E6E6);

    let head = uv(0, 0, 8, 8, 6);
    noise_box(&mut img, &head, dk, 0.08, &mut rng);
    // white blaze on the top of the head continues the face stripe
    let (tx, ty, _, _) = head.top;
    grid_n(&mut img, tx, ty, &["...WW...", "...WW...", "..WWWW..", "..WWW...", "...WW...", "...WW..."], &[('W', wh)], 0.04, &mut rng);
    let (fx, fy, _, _) = head.front;
    grid_n(
        &mut img,
        fx,
        fy,
        &[
            "...WW...", //
            "..WWWW..",
            "...WW...",
            "eE.WW.Ee",
            "...WW...",
            "..MMMM..",
            ".MNMMNM.",
            ".MMMMMM.",
        ],
        &[('W', wh), ('E', rgb(0x111111)), ('e', rgb(0xFFFFFF)), ('M', rgb(0xBDA597)), ('N', rgb(0x5B463C))],
        0.03,
        &mut rng,
    );
    // muzzle wraps a little onto the sides and bottom
    let (rx, ry, rw, _) = head.right;
    noise_rect(&mut img, (rx + rw - 2, ry + 5, 2, 3), rgb(0xBDA597), 0.03, &mut rng);
    let (lx, ly, _, _) = head.left;
    noise_rect(&mut img, (lx, ly + 5, 2, 3), rgb(0xBDA597), 0.03, &mut rng);
    let (bx, by, bw, _) = head.bottom;
    noise_rect(&mut img, (bx + 1, by, bw - 2, 3), rgb(0xBDA597), 0.03, &mut rng);
    // small white patch on the right side of the head
    grid_n(&mut img, rx, ry, &["......", "WW....", "WWW...", "WW...."], &[('W', wh)], 0.04, &mut rng);

    let horn = uv(22, 0, 1, 3, 1);
    noise_box(&mut img, &horn, rgb(0xD8D2C2), 0.04, &mut rng);
    noise_rect(&mut img, horn.top, rgb(0xA8A090), 0.03, &mut rng);

    let body = uv(18, 4, 12, 18, 10);
    noise_box(&mut img, &body, dk, 0.08, &mut rng);
    // spine side (front) & flanks get the iconic white patches
    let (x, y, _, _) = body.front;
    grid_n(
        &mut img,
        x,
        y,
        &[
            "............",
            "WWW.........",
            "WWWW........",
            "WWW.....WW..",
            ".W.....WWWW.",
            "......WWWWWW",
            "......WWWWW.",
            ".......WWW..",
            "............",
            "............",
            "...WWW......",
            "..WWWWW.....",
            "..WWWWWW....",
            "...WWWW.....",
            "....W.....WW",
            "..........WW",
            ".........WWW",
            "............",
        ],
        &[('W', wh)],
        0.04,
        &mut rng,
    );
    let (x, y, _, _) = body.right;
    grid_n(
        &mut img,
        x,
        y,
        &[
            "..........",
            "..WWW.....",
            ".WWWWW....",
            ".WWWWWW...",
            "..WWWW....",
            "..........",
            "..........",
            "......WW..",
            ".....WWWW.",
            ".....WWWWW",
            "......WWW.",
            "..........",
            "..........",
            ".WW.......",
            "WWWW......",
            "WWWWW.....",
            ".WWW......",
            "..........",
        ],
        &[('W', wh)],
        0.04,
        &mut rng,
    );
    let (x, y, _, _) = body.left;
    grid_n(
        &mut img,
        x,
        y,
        &[
            "..........",
            "......WW..",
            ".....WWWW.",
            "....WWWWW.",
            ".....WWW..",
            "..........",
            ".WW.......",
            "WWWW......",
            "WWWWW.....",
            ".WWW......",
            "..........",
            "..........",
            "......WWW.",
            ".....WWWWW",
            "....WWWWWW",
            ".....WWWW.",
            "..........",
            "..........",
        ],
        &[('W', wh)],
        0.04,
        &mut rng,
    );
    // belly (back face): mostly white
    let (x, y, w, h) = body.back;
    noise_rect(&mut img, (x + 1, y + 2, w - 2, h - 5), wh, 0.04, &mut rng);
    blotches(&mut img, (x + 1, y + 2, w - 2, h - 5), dk, 3, 1.0, 2.2, 0.06, &mut rng);
    // rear end (top) has a patch
    let (x, y, _, _) = body.top;
    grid_n(&mut img, x, y, &["............", "....WWW.....", "...WWWWW....", "....WWWW....", ".....WW....."], &[('W', wh)], 0.04, &mut rng);

    let udder = uv(52, 0, 4, 6, 1);
    noise_box(&mut img, &udder, rgb(0xE8A3A0), 0.04, &mut rng);
    let (x, y, _, _) = udder.front;
    grid(&mut img, x, y, &["....", "....", "....", "....", "T..T", "T..T"], &[('T', rgb(0xC97D7A))]);

    let leg = uv(0, 16, 4, 12, 4);
    noise_box(&mut img, &leg, dk, 0.08, &mut rng);
    band(&mut img, &leg, 6, 10, wh, 0.04, &mut rng);
    band(&mut img, &leg, 10, 12, rgb(0x5E5550), 0.05, &mut rng);
    noise_rect(&mut img, leg.bottom, rgb(0x4A423C), 0.05, &mut rng);
    img
}

// ---------------------------------------------------------------------------
// sheep
// ---------------------------------------------------------------------------

fn wool_rect(img: &mut Image, r: Rect, rng: &mut Rng) {
    pal_rect(
        img,
        r,
        &[(rgb(0xEAEAEA), 6), (rgb(0xDEDEDE), 4), (rgb(0xF6F6F6), 3), (rgb(0xCFCFCF), 2)],
        rng,
    );
}

fn sheep() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(303);
    let skin = rgb(0xE0C3A8);

    let head = uv(0, 0, 6, 6, 8);
    noise_box(&mut img, &head, skin, 0.04, &mut rng);
    let (fx, fy, _, _) = head.front;
    grid_n(
        &mut img,
        fx,
        fy,
        &["WWWWWW", "......", "eB..Be", "......", "..NN..", "..mm.."],
        &[('W', rgb(0xE8E8E8)), ('e', rgb(0xFFFFFF)), ('B', rgb(0x000000)), ('N', rgb(0xC7928A)), ('m', rgb(0xBF9C85))],
        0.02,
        &mut rng,
    );
    // ears / wool tufts on the head top
    wool_rect(&mut img, (head.top.0, head.top.1, 6, 3), &mut rng);

    let body = uv(28, 8, 8, 16, 6);
    noise_box(&mut img, &body, skin, 0.04, &mut rng);
    for r in body.all() {
        speckle(&mut img, r, rgb(0xEBD6C4), 0.25, 0.02, &mut rng);
        speckle(&mut img, r, rgb(0xCDAE93), 0.12, 0.02, &mut rng);
    }

    let leg = uv(0, 16, 4, 12, 4);
    noise_box(&mut img, &leg, skin, 0.04, &mut rng);
    band(&mut img, &leg, 10, 12, rgb(0x8F7360), 0.05, &mut rng);
    noise_rect(&mut img, leg.bottom, rgb(0x6F5848), 0.05, &mut rng);
    img
}

fn sheep_fur() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(404);
    for b in [uv(0, 0, 6, 6, 6), uv(28, 8, 8, 16, 6), uv(0, 16, 4, 6, 4)] {
        for r in b.all() {
            wool_rect(&mut img, r, &mut rng);
        }
        // subtle curly clumps
        for r in b.all() {
            speckle(&mut img, r, rgb(0xC4C4C4), 0.06, 0.02, &mut rng);
        }
    }
    img
}

// ---------------------------------------------------------------------------
// chicken
// ---------------------------------------------------------------------------

fn chicken() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(505);
    let white = rgb(0xF4F4F4);
    let shadow = rgb(0xDCDCDC);

    let head = uv(0, 0, 4, 6, 3);
    noise_box(&mut img, &head, white, 0.025, &mut rng);
    let (fx, fy, _, _) = head.front;
    grid(&mut img, fx, fy, &["....", "....", "B..B", "....", "....", "...."], &[('B', rgb(0x000000))]);

    let beak = uv(14, 0, 4, 2, 2);
    noise_box(&mut img, &beak, rgb(0xF9B233), 0.04, &mut rng);
    for r in [beak.front, beak.right, beak.left, beak.back] {
        noise_rect(&mut img, (r.0, r.1 + 1, r.2, 1), rgb(0xDB8A1C), 0.04, &mut rng);
    }

    let wattle = uv(14, 4, 2, 2, 2);
    noise_box(&mut img, &wattle, rgb(0xE01E1E), 0.06, &mut rng);

    let body = uv(0, 9, 6, 8, 6);
    noise_box(&mut img, &body, white, 0.025, &mut rng);
    for r in body.all() {
        speckle(&mut img, r, shadow, 0.15, 0.02, &mut rng);
    }

    let leg = uv(26, 0, 3, 5, 3);
    // vanilla chicken legs: thin orange stalk with a foot at the bottom
    for r in leg.sides() {
        noise_rect(&mut img, (r.0 + r.2 / 2, r.1, 1, r.3 - 1), rgb(0xF9B233), 0.04, &mut rng);
        noise_rect(&mut img, (r.0, r.1 + r.3 - 1, r.2, 1), rgb(0xE59D24), 0.04, &mut rng);
    }
    noise_rect(&mut img, leg.bottom, rgb(0xE59D24), 0.04, &mut rng);

    let wing = uv(24, 13, 1, 4, 6);
    noise_box(&mut img, &wing, white, 0.025, &mut rng);
    for r in [wing.right, wing.left] {
        speckle(&mut img, (r.0, r.1 + 2, r.2, 2), shadow, 0.5, 0.02, &mut rng);
    }
    img
}

// ---------------------------------------------------------------------------
// humanoids: steve & zombie
// ---------------------------------------------------------------------------

struct Biped {
    head: BoxUv,
    body: BoxUv,
    rarm: BoxUv,
    rleg: BoxUv,
    larm: BoxUv,
    lleg: BoxUv,
}

fn biped64() -> Biped {
    Biped {
        head: uv(0, 0, 8, 8, 8),
        body: uv(16, 16, 8, 12, 4),
        rarm: uv(40, 16, 4, 12, 4),
        rleg: uv(0, 16, 4, 12, 4),
        larm: uv(32, 48, 4, 12, 4),
        lleg: uv(16, 48, 4, 12, 4),
    }
}

fn steve() -> Image {
    let mut img = Image::new(64, 64);
    let mut rng = Rng::new(606);
    let b = biped64();
    let skin = rgb(0xB78A72);
    let hair = rgb(0x3A2813);
    let shirt = rgb(0x00ADAD);
    let pants = rgb(0x3D3A9C);
    let shoe = rgb(0x6A6A6A);

    let sp: [(char, [u8; 4]); 9] = [
        ('H', hair),
        ('h', rgb(0x2A1C0C)),
        ('S', skin),
        ('s', rgb(0xA97B64)),
        ('W', rgb(0xFFFFFF)),
        ('E', rgb(0x523D89)),
        ('N', rgb(0x8E5841)),
        ('M', rgb(0x6A4030)),
        ('m', rgb(0x8B5B45)),
    ];
    // head
    noise_rect(&mut img, b.head.top, hair, 0.12, &mut rng);
    noise_rect(&mut img, b.head.bottom, skin, 0.04, &mut rng);
    grid_n(
        &mut img,
        b.head.front.0,
        b.head.front.1,
        &[
            "HHHHHHHH", //
            "HHHHhHHH",
            "HSSSSSSH",
            "SSSSSSSS",
            "SWESSEWS",
            "SSSNNSSS",
            "SSMmmMSS",
            "SSMMMMSS",
        ],
        &sp,
        0.05,
        &mut rng,
    );
    // right side: front edge is on the right
    let side = [
        "HHHHHHHH", //
        "HhHHHHHH",
        "HHHHHHHS",
        "HHHHHSSS",
        "HHHHSSSS",
        "HHHsSSSS",
        "HHHSSSSS",
        "HHSSSSSS",
    ];
    grid_n(&mut img, b.head.right.0, b.head.right.1, &side, &sp, 0.06, &mut rng);
    grid_n_flip(&mut img, b.head.left.0, b.head.left.1, &side, &sp, 0.06, &mut rng);
    noise_rect(&mut img, b.head.back, hair, 0.12, &mut rng);
    speckle(&mut img, b.head.back, rgb(0x2A1C0C), 0.15, 0.05, &mut rng);

    // body
    noise_box(&mut img, &b.body, shirt, 0.04, &mut rng);
    for r in b.body.sides() {
        noise_rect(&mut img, (r.0, r.1 + r.3 - 1, r.2, 1), rgb(0x009A9A), 0.04, &mut rng);
    }
    noise_rect(&mut img, b.body.bottom, pants, 0.05, &mut rng);

    // right arm: sleeve then skin
    noise_box(&mut img, &b.rarm, skin, 0.04, &mut rng);
    noise_rect(&mut img, b.rarm.top, shirt, 0.04, &mut rng);
    band(&mut img, &b.rarm, 0, 4, shirt, 0.04, &mut rng);
    band(&mut img, &b.rarm, 4, 5, rgb(0x00A0A0), 0.03, &mut rng);
    for r in b.rarm.sides() {
        noise_rect(&mut img, (r.0, r.1 + 4, r.2, 1), rgb(0x00A0A0), 0.03, &mut rng);
    }
    // right leg: pants then shoes
    noise_box(&mut img, &b.rleg, pants, 0.05, &mut rng);
    band(&mut img, &b.rleg, 9, 12, shoe, 0.06, &mut rng);
    band(&mut img, &b.rleg, 11, 12, rgb(0x555555), 0.06, &mut rng);
    noise_rect(&mut img, b.rleg.bottom, rgb(0x555555), 0.06, &mut rng);

    mirror_box(&mut img, &b.rarm, &b.larm);
    mirror_box(&mut img, &b.rleg, &b.lleg);
    img
}

fn zombie() -> Image {
    let mut img = Image::new(64, 64);
    let mut rng = Rng::new(707);
    let b = biped64();
    let green = rgb(0x5A8A45);
    let dgreen = rgb(0x3F6B2F);
    let shirt = rgb(0x00A3A3);
    let pants = rgb(0x463E94);

    let zp: [(char, [u8; 4]); 6] = [
        ('G', green),
        ('g', rgb(0x4B7A39)),
        ('D', dgreen),
        ('B', rgb(0x0D140A)),
        ('N', rgb(0x34592A)),
        ('M', rgb(0x2C4A22)),
    ];
    noise_box(&mut img, &b.head, green, 0.06, &mut rng);
    for r in [b.head.top, b.head.back, b.head.right, b.head.left] {
        speckle(&mut img, r, dgreen, 0.2, 0.06, &mut rng);
    }
    // darker crown / scalp on top rows of the sides
    for r in b.head.sides() {
        speckle(&mut img, (r.0, r.1, r.2, 2), dgreen, 0.55, 0.06, &mut rng);
    }
    grid_n(
        &mut img,
        b.head.front.0,
        b.head.front.1,
        &[
            "DgDDgDDg", //
            "gGgGGgGG",
            "GGGGGGGG",
            "GGgGGgGG",
            "GBBGGBBG",
            "GGGNNGGG",
            "GGMMMMGG",
            "GgGGGGgG",
        ],
        &zp,
        0.05,
        &mut rng,
    );

    // torn shirt
    noise_box(&mut img, &b.body, shirt, 0.06, &mut rng);
    speckle(&mut img, b.body.front, rgb(0x008C8C), 0.15, 0.04, &mut rng);
    grid_n(
        &mut img,
        b.body.front.0,
        b.body.front.1,
        &[
            "..GGG...", //
            "...G....", "........", "........", "........", "........", "........", "........",
            "........", "........", "G.....GG", "GG.G.GGG",
        ],
        &zp,
        0.05,
        &mut rng,
    );
    for r in [b.body.right, b.body.left, b.body.back] {
        speckle(&mut img, (r.0, r.1 + r.3 - 2, r.2, 2), green, 0.45, 0.05, &mut rng);
    }
    noise_rect(&mut img, b.body.bottom, pants, 0.05, &mut rng);

    // arms: bare green skin with a ragged sleeve
    noise_box(&mut img, &b.rarm, green, 0.06, &mut rng);
    speckle(&mut img, b.rarm.front, dgreen, 0.12, 0.05, &mut rng);
    noise_rect(&mut img, b.rarm.top, shirt, 0.05, &mut rng);
    band(&mut img, &b.rarm, 0, 3, shirt, 0.05, &mut rng);
    for r in b.rarm.sides() {
        speckle(&mut img, (r.0, r.1 + 3, r.2, 1), shirt, 0.5, 0.05, &mut rng);
    }
    noise_rect(&mut img, b.rarm.bottom, dgreen, 0.06, &mut rng);

    // legs: pants, ragged at the bottom, then dark feet
    noise_box(&mut img, &b.rleg, pants, 0.06, &mut rng);
    band(&mut img, &b.rleg, 10, 12, rgb(0x3A3070), 0.06, &mut rng);
    for r in b.rleg.sides() {
        speckle(&mut img, (r.0, r.1 + 9, r.2, 1), rgb(0x3A3070), 0.5, 0.05, &mut rng);
    }
    noise_rect(&mut img, b.rleg.bottom, rgb(0x2E2758), 0.06, &mut rng);

    mirror_box(&mut img, &b.rarm, &b.larm);
    mirror_box(&mut img, &b.rleg, &b.lleg);
    img
}

// ---------------------------------------------------------------------------
// skeleton
// ---------------------------------------------------------------------------

fn skeleton() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(808);
    let bone = rgb(0xBEBEBE);
    let lite = rgb(0xD2D2D2);
    let dark = rgb(0x4A4A4A);
    let pal = [('L', bone), ('l', lite), ('g', rgb(0x9C9C9C)), ('D', dark), ('B', rgb(0x1E1E1E))];

    let head = uv(0, 0, 8, 8, 8);
    noise_box(&mut img, &head, bone, 0.05, &mut rng);
    for r in head.all() {
        speckle(&mut img, r, rgb(0xA8A8A8), 0.15, 0.04, &mut rng);
    }
    grid_n(
        &mut img,
        head.front.0,
        head.front.1,
        &[
            "lLLlLLLl", //
            "LLLLLLLL",
            "LLLLLLLL",
            "LggLLggL",
            "LBBLLBBL",
            "LLLDDLLL",
            "LDDDDDDL",
            "LLDLLDLL",
        ],
        &pal,
        0.04,
        &mut rng,
    );

    let body = uv(16, 16, 8, 12, 4);
    noise_box(&mut img, &body, bone, 0.05, &mut rng);
    let ribs = [
        "LLLLLLLL", //
        "DDDLLDDD",
        "LLLLLLLL",
        "DDDLLDDD",
        "LLLLLLLL",
        "DDDLLDDD",
        "LLLLLLLL",
        "DDDLLDDD",
        "DDDLLDDD",
        "DDDLLDDD",
        "LLLLLLLL",
        "LLLDDLLL",
    ];
    grid_n(&mut img, body.front.0, body.front.1, &ribs, &pal, 0.05, &mut rng);
    grid_n(&mut img, body.back.0, body.back.1, &ribs, &pal, 0.05, &mut rng);
    let side = [
        "LLLL", "DDDD", "LLLL", "DDDD", "LLLL", "DDDD", "LLLL", "DDDD", "DDDD", "DDDD", "LLLL", "LLLL",
    ];
    grid_n(&mut img, body.right.0, body.right.1, &side, &pal, 0.05, &mut rng);
    grid_n(&mut img, body.left.0, body.left.1, &side, &pal, 0.05, &mut rng);
    noise_rect(&mut img, body.top, dark, 0.05, &mut rng);
    noise_rect(&mut img, (body.top.0 + 3, body.top.1, 2, 4), bone, 0.05, &mut rng);

    for b in [uv(40, 16, 2, 12, 2), uv(0, 16, 2, 12, 2)] {
        noise_box(&mut img, &b, bone, 0.05, &mut rng);
        for r in b.sides() {
            speckle(&mut img, r, rgb(0xA0A0A0), 0.15, 0.04, &mut rng);
            // joint at the middle
            noise_rect(&mut img, (r.0, r.1 + 5, r.2, 1), rgb(0xA6A6A6), 0.04, &mut rng);
        }
    }
    img
}

// ---------------------------------------------------------------------------
// creeper
// ---------------------------------------------------------------------------

fn creeper_rect(img: &mut Image, r: Rect, rng: &mut Rng) {
    pal_rect(
        img,
        r,
        &[
            (rgb(0x5BC74F), 8),
            (rgb(0x0F9A0D), 6),
            (rgb(0x4BAE44), 6),
            (rgb(0x7CD978), 4),
            (rgb(0x2E7D2A), 3),
            (rgb(0xA4E0A1), 2),
            (rgb(0xC8D4C6), 1),
        ],
        rng,
    );
}

fn creeper() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(909);
    for b in [uv(0, 0, 8, 8, 8), uv(16, 16, 8, 12, 4), uv(0, 16, 4, 6, 4)] {
        for r in b.all() {
            creeper_rect(&mut img, r, &mut rng);
        }
    }
    let head = uv(0, 0, 8, 8, 8);
    grid(
        &mut img,
        head.front.0,
        head.front.1,
        &[
            "........", //
            "........",
            ".BBgBBB.",
            ".BBB.BB.",
            "...BB...",
            "..BgBB..",
            "..BBBB..",
            "..B..B..",
        ],
        &[('B', rgb(0x000000)), ('g', rgb(0x1A2A19))],
    );
    // fix: eye shape is two clean 2x2 squares
    grid(&mut img, head.front.0, head.front.1, &["", "", ".BB..BB.", ".BB..BB."], &[('B', rgb(0x000000))]);
    let leg = uv(0, 16, 4, 6, 4);
    noise_rect(&mut img, leg.bottom, rgb(0x2E5A2B), 0.08, &mut rng);
    img
}

// ---------------------------------------------------------------------------
// spider
// ---------------------------------------------------------------------------

fn spider_rect(img: &mut Image, r: Rect, rng: &mut Rng) {
    pal_rect(
        img,
        r,
        &[(rgb(0x2C241E), 10), (rgb(0x342B24), 8), (rgb(0x241D18), 4), (rgb(0x3F352C), 2)],
        rng,
    );
}

fn spider() -> Image {
    let mut img = Image::new(64, 32);
    let mut rng = Rng::new(1010);
    let head = uv(32, 4, 8, 8, 8);
    let neck = uv(0, 0, 6, 6, 6);
    let body = uv(0, 12, 10, 8, 12);
    let leg = uv(18, 0, 16, 2, 2);
    for b in [head, neck, body, leg] {
        for r in b.all() {
            spider_rect(&mut img, r, &mut rng);
        }
    }
    grid(
        &mut img,
        head.front.0,
        head.front.1,
        &[
            "........", //
            "........",
            "r.r..r.r",
            ".RO..OR.",
            ".RR..RR.",
            "........",
            "..F..F..",
            "........",
        ],
        &[('r', rgb(0x9E0F0F)), ('R', rgb(0xC81414)), ('O', rgb(0xFF4A3A)), ('F', rgb(0x6B5D4E))],
    );
    // hairy abdomen pattern: lighter markings on the body's top & sides
    for r in [body.top, body.right, body.left, body.front, body.back] {
        speckle(&mut img, r, rgb(0x4E4337), 0.06, 0.06, &mut rng);
    }
    let (x, y, _, _) = body.top;
    grid_n(
        &mut img,
        x,
        y,
        &[
            "..........",
            "....mm....",
            "...m..m...",
            "....mm....",
            "..........",
            "..m....m..",
            "...m..m...",
            "....mm....",
            "..........",
            "..m....m..",
            "...mmmm...",
            "..........",
        ],
        &[('m', rgb(0x5A4D3F))],
        0.06,
        &mut rng,
    );
    // leg joints
    for r in [leg.top, leg.front, leg.back, leg.bottom] {
        noise_rect(&mut img, (r.0 + 7, r.1, 1, r.2.min(r.3)), rgb(0x564A3E), 0.05, &mut rng);
    }
    img
}

// ---------------------------------------------------------------------------
// environment
// ---------------------------------------------------------------------------

fn sun() -> Image {
    let mut img = Image::new(32, 32);
    // soft glow halo around the square
    for y in 0..32 {
        for x in 0..32 {
            let dx = (x as f32 + 0.5 - 16.0).abs();
            let dy = (y as f32 + 0.5 - 16.0).abs();
            let d = dx.max(dy);
            if d >= 8.0 && d < 12.0 {
                let a = ((12.0 - d) / 4.0 * 110.0) as u8;
                img.set(x, y, [255, 230, 120, a]);
            }
        }
    }
    img.fill_rect(8, 8, 16, 16, rgb(0xFFE84A));
    img.fill_rect(9, 9, 14, 14, rgb(0xFFF27A));
    img.fill_rect(10, 10, 12, 12, rgb(0xFFFAB0));
    img.fill_rect(12, 12, 8, 8, rgb(0xFFFFE0));
    img
}

fn moon() -> Image {
    let mut img = Image::new(32, 32);
    let mut rng = Rng::new(1212);
    let base = rgb(0xDCDCE4);
    // faint glow
    for y in 0..32 {
        for x in 0..32 {
            let dx = (x as f32 + 0.5 - 16.0).abs();
            let dy = (y as f32 + 0.5 - 16.0).abs();
            let d = dx.max(dy);
            if d >= 8.0 && d < 10.0 {
                img.set(x, y, [200, 210, 235, ((10.0 - d) / 2.0 * 60.0) as u8]);
            }
        }
    }
    noise_rect(&mut img, (8, 8, 16, 16), base, 0.03, &mut rng);
    grid(
        &mut img,
        8,
        8,
        &[
            "................",
            "..cc............",
            ".cCCc.....cc....",
            ".cCCc....cCCc...",
            "..cc.....cCCc...",
            ".........cCCc...",
            "....c.....cc....",
            "...cCc..........",
            "....c......ccc..",
            "..........cCCCc.",
            ".cc.......cCCCc.",
            "cCCc......cCCCc.",
            "cCCc.......ccc..",
            ".cc.....c.......",
            ".......cCc......",
            "........c.......",
        ],
        &[('c', rgb(0xB4B4BE)), ('C', rgb(0x9A9AA6))],
    );
    img
}

fn clouds() -> Image {
    const N: usize = 256;
    // tileable value noise, several octaves
    let mut rng = Rng::new(1313);
    let lat: Vec<f32> = (0..64 * 64).map(|_| rng.f()).collect();
    let sample = |x: f32, y: f32, period: usize| -> f32 {
        let xi = x.floor() as i64;
        let yi = y.floor() as i64;
        let fx = x - xi as f32;
        let fy = y - yi as f32;
        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sy = fy * fy * (3.0 - 2.0 * fy);
        let g = |a: i64, b: i64| {
            let a = a.rem_euclid(period as i64) as usize;
            let b = b.rem_euclid(period as i64) as usize;
            lat[b * 64 + a]
        };
        let a = g(xi, yi) + (g(xi + 1, yi) - g(xi, yi)) * sx;
        let b = g(xi, yi + 1) + (g(xi + 1, yi + 1) - g(xi, yi + 1)) * sx;
        a + (b - a) * sy
    };
    let mut vals = vec![0f32; N * N];
    for y in 0..N {
        for x in 0..N {
            let mut v = 0.0;
            let mut amp = 1.0;
            let mut cell = 16.0;
            for _ in 0..3 {
                let period = (N as f32 / cell) as usize;
                v += sample(x as f32 / cell, y as f32 / cell, period) * amp;
                amp *= 0.55;
                cell /= 2.0;
            }
            vals[y * N + x] = v;
        }
    }
    let mut sorted = vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let thresh = sorted[(N * N) * 64 / 100];
    let mut img = Image::new(N, N);
    for y in 0..N {
        for x in 0..N {
            if vals[y * N + x] > thresh {
                img.set(x, y, [255, 255, 255, 255]);
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_assets_build() {
        for n in NAMES {
            let img = get(n).expect(n);
            assert!(img.w > 0 && img.h > 0);
        }
        let c = get("clouds").unwrap();
        let cov = c.data.iter().filter(|p| p[3] > 0).count() as f32 / (c.w * c.h) as f32;
        assert!(cov > 0.28 && cov < 0.42, "cloud coverage {cov}");
    }

    /// Writes scaled previews when MOBS_PREVIEW_DIR is set.
    #[test]
    fn preview() {
        let Ok(dir) = std::env::var("MOBS_PREVIEW_DIR") else { return };
        let _ = std::fs::create_dir_all(&dir);
        for n in NAMES {
            let img = get(n).unwrap();
            let f = if img.w >= 256 { 2 } else { 8 };
            // checker background so transparency is visible
            let mut bg = Image::new(img.w, img.h);
            for y in 0..img.h {
                for x in 0..img.w {
                    let c = if (x + y) % 2 == 0 { 0x40 } else { 0x50 };
                    bg.set(x, y, [c, c, c + 0x20, 255]);
                }
            }
            bg.blend(&img, 0, 0);
            bg.scaled(f).save_png(&format!("{dir}/{n}.png")).unwrap();
        }
        // front views
        let fv = |name: &str, parts: &[(Rect, i32, i32)], w: usize, h: usize| {
            let img = get(name).unwrap();
            let mut o = Image::filled(w, h, [90, 140, 200, 255]);
            for &(r, dx, dy) in parts {
                for y in 0..r.3 {
                    for x in 0..r.2 {
                        let c = img.get((r.0 + x) as usize, (r.1 + y) as usize);
                        if c[3] > 0 {
                            o.put(dx + x, dy + y, c);
                        }
                    }
                }
            }
            o.scaled(10).save_png(&format!("{dir}/front_{name}.png")).unwrap();
        };
        let b = biped64();
        for n in ["steve", "zombie"] {
            fv(
                n,
                &[
                    (b.head.front, 4, 0),
                    (b.body.front, 4, 8),
                    (b.rarm.front, 0, 8),
                    (b.larm.front, 12, 8),
                    (b.rleg.front, 4, 20),
                    (b.lleg.front, 8, 20),
                    (b.head.right, 18, 0),
                    (b.head.back, 28, 0),
                    (b.body.back, 28, 8),
                ],
                38,
                32,
            );
        }
        let h = uv(0, 0, 8, 8, 8);
        let bd = uv(16, 16, 8, 12, 4);
        fv("skeleton", &[(h.front, 2, 0), (bd.front, 2, 8), (uv(40, 16, 2, 12, 2).front, 0, 8), (uv(40, 16, 2, 12, 2).front, 10, 8), (uv(0, 16, 2, 12, 2).front, 3, 20), (uv(0, 16, 2, 12, 2).front, 7, 20)], 12, 32);
        fv("creeper", &[(h.front, 0, 0), (bd.front, 0, 8), (uv(0, 16, 4, 6, 4).front, 0, 20), (uv(0, 16, 4, 6, 4).front, 4, 20)], 8, 26);
        fv("pig", &[(h.front, 0, 0), (uv(16, 16, 4, 3, 1).front, 2, 4), (uv(0, 16, 4, 6, 4).front, 0, 9), (uv(0, 16, 4, 6, 4).front, 4, 9)], 8, 15);
        fv("cow", &[(uv(0, 0, 8, 8, 6).front, 2, 1), (uv(22, 0, 1, 3, 1).front, 1, 0), (uv(22, 0, 1, 3, 1).front, 10, 0), (uv(0, 16, 4, 12, 4).front, 2, 10), (uv(0, 16, 4, 12, 4).front, 6, 10), (uv(18, 4, 12, 18, 10).front, 14, 0)], 28, 22);
        fv("sheep", &[(uv(0, 0, 6, 6, 8).front, 1, 0), (uv(0, 16, 4, 12, 4).front, 0, 7)], 8, 20);
        fv("chicken", &[(uv(0, 0, 4, 6, 3).front, 1, 0), (uv(14, 0, 4, 2, 2).front, 1, 3), (uv(14, 4, 2, 2, 2).front, 2, 5), (uv(0, 9, 6, 8, 6).front, 0, 7), (uv(26, 0, 3, 5, 3).front, 0, 15)], 8, 20);
        fv("spider", &[(uv(32, 4, 8, 8, 8).front, 4, 0), (uv(0, 12, 10, 8, 12).top, 18, 0)], 30, 12);
    }
}
