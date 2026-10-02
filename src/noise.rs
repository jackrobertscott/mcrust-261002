//! Seeded gradient (Perlin) noise and hashing utilities for world generation.
#![allow(dead_code)]

#[derive(Clone)]
pub struct Perlin {
    perm: [u8; 512],
    ox: f64,
    oy: f64,
    oz: f64,
}

pub fn hash64(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

/// Deterministic hash of integer coordinates + seed.
pub fn hash3(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    hash64(seed ^ (x as u32 as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as u32 as u64).wrapping_mul(0xC2B2AE3D27D4EB4F).rotate_left(17)
        ^ (z as u32 as u64).wrapping_mul(0x165667B19E3779F9).rotate_left(31))
}

/// Simple seeded PRNG (splitmix64).
#[derive(Clone)]
pub struct Random(pub u64);
impl Random {
    pub fn new(seed: u64) -> Random {
        Random(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        hash64(self.0)
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, n: i32) -> i32 {
        if n <= 0 { 0 } else { (self.next_u64() % n as u64) as i32 }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }
    /// Uniform in [a,b)
    pub fn uniform(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}
fn grad(h: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = h & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

impl Perlin {
    pub fn new(seed: u64) -> Perlin {
        let mut r = Random::new(seed);
        let mut p = [0u8; 256];
        for (i, v) in p.iter_mut().enumerate() {
            *v = i as u8;
        }
        for i in (1..256).rev() {
            let j = (r.next_u64() % (i as u64 + 1)) as usize;
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Perlin { perm, ox: r.next_f32() as f64 * 256.0, oy: r.next_f32() as f64 * 256.0, oz: r.next_f32() as f64 * 256.0 }
    }

    pub fn noise3(&self, x: f64, y: f64, z: f64) -> f64 {
        let (x, y, z) = (x + self.ox, y + self.oy, z + self.oz);
        let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
        let xi = (fx as i64 & 255) as usize;
        let yi = (fy as i64 & 255) as usize;
        let zi = (fz as i64 & 255) as usize;
        let (x, y, z) = (x - fx, y - fy, z - fz);
        let (u, v, w) = (fade(x), fade(y), fade(z));
        let p = &self.perm;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;
        lerp(
            w,
            lerp(
                v,
                lerp(u, grad(p[aa], x, y, z), grad(p[ba], x - 1.0, y, z)),
                lerp(u, grad(p[ab], x, y - 1.0, z), grad(p[bb], x - 1.0, y - 1.0, z)),
            ),
            lerp(
                v,
                lerp(u, grad(p[aa + 1], x, y, z - 1.0), grad(p[ba + 1], x - 1.0, y, z - 1.0)),
                lerp(u, grad(p[ab + 1], x, y - 1.0, z - 1.0), grad(p[bb + 1], x - 1.0, y - 1.0, z - 1.0)),
            ),
        )
    }

    pub fn noise2(&self, x: f64, y: f64) -> f64 {
        self.noise3(x, y, 0.5)
    }
}

/// Fractal (fBm) noise built from several octaves of Perlin noise.
#[derive(Clone)]
pub struct Octaves {
    layers: Vec<Perlin>,
}

impl Octaves {
    pub fn new(seed: u64, n: usize) -> Octaves {
        Octaves { layers: (0..n).map(|i| Perlin::new(hash64(seed.wrapping_add(i as u64 * 7919)))).collect() }
    }
    /// Result roughly in [-1,1].
    pub fn fbm2(&self, x: f64, y: f64) -> f64 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for l in &self.layers {
            sum += l.noise2(x * freq, y * freq) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
    pub fn fbm3(&self, x: f64, y: f64, z: f64) -> f64 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for l in &self.layers {
            sum += l.noise3(x * freq, y * freq, z * freq) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}
