//! Simple RGBA image type plus a tiny dependency-free PNG encoder (used for
//! debugging / dumping generated assets).

#[derive(Clone)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    /// RGBA8, row-major, top row first.
    pub data: Vec<[u8; 4]>,
}

impl Image {
    pub fn new(w: usize, h: usize) -> Image {
        Image { w, h, data: vec![[0, 0, 0, 0]; w * h] }
    }
    pub fn filled(w: usize, h: usize, c: [u8; 4]) -> Image {
        Image { w, h, data: vec![c; w * h] }
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> [u8; 4] {
        self.data[y * self.w + x]
    }
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, c: [u8; 4]) {
        if x < self.w && y < self.h {
            self.data[y * self.w + x] = c;
        }
    }
    /// Signed-coordinate set that silently ignores out-of-bounds pixels.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x >= 0 && y >= 0 {
            self.set(x as usize, y as usize, c);
        }
    }
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, c);
            }
        }
    }
    /// Copy `src` into self at (dx,dy) (no blending).
    pub fn blit(&mut self, src: &Image, dx: i32, dy: i32) {
        for y in 0..src.h {
            for x in 0..src.w {
                self.put(dx + x as i32, dy + y as i32, src.get(x, y));
            }
        }
    }
    /// Copy `src` into self at (dx,dy) using alpha blending.
    pub fn blend(&mut self, src: &Image, dx: i32, dy: i32) {
        for y in 0..src.h {
            for x in 0..src.w {
                let s = src.get(x, y);
                let (tx, ty) = (dx + x as i32, dy + y as i32);
                if tx < 0 || ty < 0 || tx as usize >= self.w || ty as usize >= self.h {
                    continue;
                }
                let d = self.get(tx as usize, ty as usize);
                let a = s[3] as u32;
                let ia = 255 - a;
                let o = [
                    ((s[0] as u32 * a + d[0] as u32 * ia) / 255) as u8,
                    ((s[1] as u32 * a + d[1] as u32 * ia) / 255) as u8,
                    ((s[2] as u32 * a + d[2] as u32 * ia) / 255) as u8,
                    (a + d[3] as u32 * ia / 255).min(255) as u8,
                ];
                self.set(tx as usize, ty as usize, o);
            }
        }
    }
    /// Scale up by an integer factor (nearest neighbour) - handy for previews.
    pub fn scaled(&self, f: usize) -> Image {
        let mut o = Image::new(self.w * f, self.h * f);
        for y in 0..o.h {
            for x in 0..o.w {
                o.data[y * o.w + x] = self.get(x / f, y / f);
            }
        }
        o
    }

    pub fn to_png(&self) -> Vec<u8> {
        encode_png(self.w, self.h, &self.data)
    }
    pub fn save_png(&self, path: &str) -> std::io::Result<()> {
        std::fs::write(path, self.to_png())
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[n as usize] = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &d in data {
        a = (a + d as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, ty: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(4 + data.len());
    c.extend_from_slice(ty);
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&crc32(&c).to_be_bytes());
}

pub fn encode_png(w: usize, h: usize, px: &[[u8; 4]]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(h * (w * 4 + 1));
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.extend_from_slice(&px[y * w + x]);
        }
    }
    // zlib stream with uncompressed (stored) deflate blocks
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    if raw.is_empty() {
        z.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(c) = chunks.next() {
        let last = chunks.peek().is_none();
        z.push(if last { 1 } else { 0 });
        let len = c.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Small deterministic RNG helpers for texture generation.
pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x1234567))
    }
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    /// Uniform in [0,1)
    pub fn f(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform integer in [0,n)
    pub fn range(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
}

/// Hex colour helper: rgb(0x7F7F7F) -> opaque RGBA
pub const fn rgb(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]
}
