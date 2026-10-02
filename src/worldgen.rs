//! Terrain generation: biomes, height, caves, ores, trees and plants.
//! Fully deterministic per chunk (trees that overhang chunk borders are
//! generated from neighbouring chunks' seeds), so chunks can be generated
//! in parallel on worker threads.

use crate::block::*;
use crate::noise::{hash3, Octaves, Perlin, Random};
use crate::world::{Chunk, CHUNK_H, SEA_LEVEL};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Biome {
    Ocean = 0,
    DeepOcean,
    FrozenOcean,
    Beach,
    SnowyBeach,
    Plains,
    Forest,
    BirchForest,
    Taiga,
    SnowyTaiga,
    SnowyTundra,
    Desert,
    Savanna,
    Jungle,
    Swamp,
    Mountains,
    SnowyMountains,
    River,
}

impl Biome {
    pub fn from_u8(v: u8) -> Biome {
        if v <= Biome::River as u8 { unsafe { std::mem::transmute::<u8, Biome>(v) } } else { Biome::Plains }
    }
    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "Ocean",
            Biome::DeepOcean => "Deep Ocean",
            Biome::FrozenOcean => "Frozen Ocean",
            Biome::Beach => "Beach",
            Biome::SnowyBeach => "Snowy Beach",
            Biome::Plains => "Plains",
            Biome::Forest => "Forest",
            Biome::BirchForest => "Birch Forest",
            Biome::Taiga => "Taiga",
            Biome::SnowyTaiga => "Snowy Taiga",
            Biome::SnowyTundra => "Snowy Tundra",
            Biome::Desert => "Desert",
            Biome::Savanna => "Savanna",
            Biome::Jungle => "Jungle",
            Biome::Swamp => "Swamp",
            Biome::Mountains => "Mountains",
            Biome::SnowyMountains => "Snowy Mountains",
            Biome::River => "River",
        }
    }
    /// (grass colour, foliage colour)
    pub fn colors(self) -> (u32, u32) {
        match self {
            Biome::Plains | Biome::Beach => (0x91BD59, 0x77AB2F),
            Biome::Ocean | Biome::DeepOcean | Biome::River => (0x8EB971, 0x71A74D),
            Biome::Forest => (0x79C05A, 0x59AE30),
            Biome::BirchForest => (0x88BB67, 0x6BA941),
            Biome::Taiga => (0x86B783, 0x68A464),
            Biome::SnowyTaiga | Biome::SnowyTundra | Biome::SnowyBeach | Biome::FrozenOcean | Biome::SnowyMountains => (0x80B497, 0x60A17B),
            Biome::Desert | Biome::Savanna => (0xBFB755, 0xAEA42A),
            Biome::Jungle => (0x59C93C, 0x30BB0B),
            Biome::Swamp => (0x6A7039, 0x6A7039),
            Biome::Mountains => (0x8AB689, 0x6DA36B),
        }
    }
    pub fn is_snowy(self) -> bool {
        matches!(self, Biome::FrozenOcean | Biome::SnowyBeach | Biome::SnowyTaiga | Biome::SnowyTundra | Biome::SnowyMountains)
    }
    pub fn is_ocean(self) -> bool {
        matches!(self, Biome::Ocean | Biome::DeepOcean | Biome::FrozenOcean)
    }
}

pub struct Column {
    pub height: i32,
    pub biome: Biome,
    pub temp: f64,
    pub humid: f64,
}

pub struct Generator {
    pub seed: u64,
    continent: Octaves,
    erosion: Octaves,
    peaks: Octaves,
    detail: Octaves,
    temp: Octaves,
    humid: Octaves,
    river: Octaves,
    cave1: Perlin,
    cave2: Perlin,
    cave3: Perlin,
    cheese: Octaves,
    surface: Perlin,
    misc: Perlin,
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Generator {
    pub fn new(seed: u64) -> Generator {
        Generator {
            seed,
            continent: Octaves::new(seed ^ 0x1111, 5),
            erosion: Octaves::new(seed ^ 0x2222, 4),
            peaks: Octaves::new(seed ^ 0x3333, 5),
            detail: Octaves::new(seed ^ 0x4444, 4),
            temp: Octaves::new(seed ^ 0x5555, 3),
            humid: Octaves::new(seed ^ 0x6666, 3),
            river: Octaves::new(seed ^ 0x7777, 4),
            cave1: Perlin::new(seed ^ 0x8888),
            cave2: Perlin::new(seed ^ 0x9999),
            cave3: Perlin::new(seed ^ 0xAAAA),
            cheese: Octaves::new(seed ^ 0xBBBB, 2),
            surface: Perlin::new(seed ^ 0xCCCC),
            misc: Perlin::new(seed ^ 0xDDDD),
        }
    }

    /// Climate parameters for a column: (temperature, humidity) roughly in [-1,1].
    fn climate(&self, x: f64, z: f64) -> (f64, f64) {
        let t = self.temp.fbm2(x / 1400.0, z / 1400.0) * 2.4 + self.detail.fbm2(x / 40.0, z / 40.0) * 0.03;
        let h = self.humid.fbm2(x / 1100.0 + 100.0, z / 1100.0) * 2.4 + self.detail.fbm2(z / 40.0, x / 40.0) * 0.03;
        (t, h)
    }

    pub fn column(&self, x: i32, z: i32) -> Column {
        let (fx, fz) = (x as f64, z as f64);
        let cont = self.continent.fbm2(fx / 900.0, fz / 900.0) * 1.5 + 0.12;
        let ero = self.erosion.fbm2(fx / 450.0, fz / 450.0) * 1.5;
        let pk = self.peaks.fbm2(fx / 160.0, fz / 160.0);
        let det = self.detail.fbm2(fx / 70.0, fz / 70.0);
        let (temp, humid) = self.climate(fx, fz);

        // Base land height from continentalness.
        let sea = SEA_LEVEL as f64;
        let mut h;
        if cont < -0.15 {
            // Ocean floor
            let t = smoothstep(-0.15, -0.55, cont);
            h = sea - 4.0 - t * 22.0 + det * 4.0;
        } else {
            let land = smoothstep(-0.15, 0.25, cont);
            h = sea - 3.0 + land * 7.0 + det * (3.0 + land * 5.0);
            // Hills / mountains where erosion is low.
            let m = smoothstep(0.05, 0.6, -ero) * land;
            let ridge = 1.0 - pk.abs() * 2.0; // ridges in [-1,1]
            h += m * (18.0 + ridge.max(-0.3) * 28.0 + det * 10.0);
            // Gentle rolling hills elsewhere
            h += (1.0 - m) * smoothstep(-0.2, 0.4, ero) * det.max(0.0) * 6.0;
        }
        // Rivers: carve channels where river noise crosses zero.
        let rv = self.river.fbm2(fx / 700.0, fz / 700.0).abs();
        let mut is_river = false;
        if cont > -0.05 && rv < 0.022 {
            let depth = 1.0 - rv / 0.022;
            let target = sea - 3.0;
            if h > target {
                h = h - (h - target) * smoothstep(0.0, 0.6, depth);
            }
            if h < sea - 0.5 {
                is_river = true;
            }
        }
        // Swamps are flat near sea level.
        let height = (h.round() as i32).clamp(4, CHUNK_H as i32 - 8);

        let biome = self.pick_biome(height, temp, humid, cont, ero, is_river);
        let mut height = height;
        if biome == Biome::Swamp {
            height = height.min(SEA_LEVEL + 1).max(SEA_LEVEL - 1);
        }
        Column { height, biome, temp, humid }
    }

    fn pick_biome(&self, height: i32, t: f64, h: f64, cont: f64, ero: f64, river: bool) -> Biome {
        let cold = t < -0.38;
        if height < SEA_LEVEL - 1 && !river {
            if cold {
                return Biome::FrozenOcean;
            }
            return if height < SEA_LEVEL - 16 { Biome::DeepOcean } else { Biome::Ocean };
        }
        if river {
            return if cold { Biome::FrozenOcean } else { Biome::River };
        }
        if height <= SEA_LEVEL + 1 && cont < 0.0 && ero > -0.3 {
            return if cold { Biome::SnowyBeach } else if t > 0.4 && h < 0.0 { Biome::Desert } else { Biome::Beach };
        }
        if height > SEA_LEVEL + 26 {
            return if cold || height > SEA_LEVEL + 44 { Biome::SnowyMountains } else { Biome::Mountains };
        }
        if cold {
            return if h > 0.0 { Biome::SnowyTaiga } else { Biome::SnowyTundra };
        }
        if t < -0.12 {
            return Biome::Taiga;
        }
        if t > 0.38 {
            if h < -0.05 {
                return Biome::Desert;
            }
            if h < 0.3 {
                return Biome::Savanna;
            }
            return Biome::Jungle;
        }
        if h > 0.42 && height <= SEA_LEVEL + 3 {
            return Biome::Swamp;
        }
        if h < -0.2 {
            return Biome::Plains;
        }
        if h > 0.22 {
            return Biome::BirchForest;
        }
        Biome::Forest
    }

    fn is_cave(&self, x: i32, y: i32, z: i32, surface: i32) -> bool {
        if y <= 0 {
            return false;
        }
        let (fx, fy, fz) = (x as f64, y as f64, z as f64);
        // Spaghetti tunnels: intersection of two noise "zero sheets".
        let a = self.cave1.noise3(fx / 48.0, fy / 32.0, fz / 48.0);
        let b = self.cave2.noise3(fx / 48.0, fy / 32.0, fz / 48.0);
        let w = 0.055 + (0.02 * self.cave3.noise3(fx / 30.0, fy / 30.0, fz / 30.0));
        let tunnel = a * a + b * b < w * w;
        // Cheese caverns deep down.
        let cheese = if y < 50 {
            let c = self.cheese.fbm3(fx / 70.0, fy / 40.0, fz / 70.0);
            c > 0.42 + (y as f64 / 50.0) * 0.1
        } else {
            false
        };
        if !(tunnel || cheese) {
            return false;
        }
        // Don't break through the ocean floor / under water.
        if surface < SEA_LEVEL && y > surface - 6 {
            return false;
        }
        true
    }

    /// Climate-blended grass/foliage colours for a column.
    fn blended_colors(&self, x: i32, z: i32) -> (u32, u32) {
        let mut acc = [0u32; 6];
        let mut n = 0;
        for dx in [-6, 0, 6] {
            for dz in [-6, 0, 6] {
                let c = self.column_biome_cheap(x + dx, z + dz);
                let (g, f) = c.colors();
                acc[0] += (g >> 16) & 255;
                acc[1] += (g >> 8) & 255;
                acc[2] += g & 255;
                acc[3] += (f >> 16) & 255;
                acc[4] += (f >> 8) & 255;
                acc[5] += f & 255;
                n += 1;
            }
        }
        let g = ((acc[0] / n) << 16) | ((acc[1] / n) << 8) | (acc[2] / n);
        let f = ((acc[3] / n) << 16) | ((acc[4] / n) << 8) | (acc[5] / n);
        (g, f)
    }

    fn column_biome_cheap(&self, x: i32, z: i32) -> Biome {
        self.column(x, z).biome
    }

    pub fn generate(&self, cx: i32, cz: i32) -> Chunk {
        let mut ch = Chunk::new(cx, cz);
        let bx = cx * 16;
        let bz = cz * 16;
        let mut cols: Vec<Column> = Vec::with_capacity(256);
        for lz in 0..16 {
            for lx in 0..16 {
                cols.push(self.column(bx + lx, bz + lz));
            }
        }
        // Biome colours: sample on a coarse grid and interpolate for speed.
        let mut grid = [[(0u32, 0u32); 3]; 3];
        for (gz, row) in grid.iter_mut().enumerate() {
            for (gx, cell) in row.iter_mut().enumerate() {
                *cell = self.blended_colors(bx + gx as i32 * 8, bz + gz as i32 * 8);
            }
        }
        let lerp_col = |a: u32, b: u32, t: f32| -> u32 {
            let mut o = 0;
            for s in [16, 8, 0] {
                let ca = ((a >> s) & 255) as f32;
                let cb = ((b >> s) & 255) as f32;
                o |= ((ca + (cb - ca) * t) as u32) << s;
            }
            o
        };
        for lz in 0..16 {
            for lx in 0..16 {
                let (gx, tx) = ((lx / 8) as usize, (lx % 8) as f32 / 8.0);
                let (gz, tz) = ((lz / 8) as usize, (lz % 8) as f32 / 8.0);
                let mut res = [0u32; 2];
                for k in 0..2 {
                    let get = |a: usize, b: usize| if k == 0 { grid[b][a].0 } else { grid[b][a].1 };
                    let top = lerp_col(get(gx, gz), get(gx + 1, gz), tx);
                    let bot = lerp_col(get(gx, gz + 1), get(gx + 1, gz + 1), tx);
                    res[k] = lerp_col(top, bot, tz);
                }
                let i = (lz * 16 + lx) as usize;
                ch.grass_color[i] = res[0];
                ch.foliage_color[i] = res[1];
                ch.biomes[i] = cols[i].biome as u8;
            }
        }

        let mut rng = Random::new(hash3(self.seed, cx, 0, cz));

        // ---- Terrain ----
        for lz in 0..16i32 {
            for lx in 0..16i32 {
                let col = &cols[(lz * 16 + lx) as usize];
                let (wx, wz) = (bx + lx, bz + lz);
                let h = col.height;
                let biome = col.biome;
                let sn = self.surface.noise2(wx as f64 / 8.0, wz as f64 / 8.0);
                let soil_depth = 3 + (sn * 1.5 + 1.0) as i32;
                let (top, filler) = match biome {
                    Biome::Desert => (SAND, SAND),
                    Biome::Beach | Biome::SnowyBeach => (SAND, SAND),
                    Biome::Ocean | Biome::FrozenOcean | Biome::River => {
                        if h < SEA_LEVEL - 6 || sn > 0.3 { (GRAVEL, GRAVEL) } else if sn < -0.35 { (CLAY, CLAY) } else { (SAND, SAND) }
                    }
                    Biome::DeepOcean => (GRAVEL, GRAVEL),
                    Biome::Mountains | Biome::SnowyMountains => {
                        if h > SEA_LEVEL + 36 + (sn * 6.0) as i32 { (STONE, STONE) } else if sn > 0.45 { (GRAVEL, STONE) } else { (GRASS, DIRT) }
                    }
                    _ => (GRASS, DIRT),
                };
                for y in 0..CHUNK_H as i32 {
                    let b = if y == 0 {
                        BEDROCK
                    } else if y < 5 && (hash3(self.seed, wx, y, wz) % 5) as i32 >= y {
                        BEDROCK
                    } else if y < h - soil_depth {
                        STONE
                    } else if y < h {
                        if filler == SAND && y < h - 3 { SANDSTONE } else { filler }
                    } else if y == h {
                        if top == GRASS && h < SEA_LEVEL { DIRT } else { top }
                    } else if y <= SEA_LEVEL {
                        WATER
                    } else {
                        AIR
                    };
                    if b != AIR {
                        ch.set(lx, y, lz, b);
                    }
                }
                // Desert: sandstone below sand
                if top == SAND {
                    for y in (h - soil_depth - 3).max(1)..(h - soil_depth) {
                        if ch.get(lx, y, lz) == STONE {
                            ch.set(lx, y, lz, SANDSTONE);
                        }
                    }
                }
                // Frozen surfaces
                if biome.is_snowy() && h < SEA_LEVEL {
                    ch.set(lx, SEA_LEVEL, lz, ICE);
                }
            }
        }

        // ---- Caves ----
        for lz in 0..16i32 {
            for lx in 0..16i32 {
                let col = &cols[(lz * 16 + lx) as usize];
                let (wx, wz) = (bx + lx, bz + lz);
                let top = col.height.min(CHUNK_H as i32 - 2);
                for y in 1..=top {
                    let b = ch.get(lx, y, lz);
                    if b == BEDROCK || b == WATER || b == AIR {
                        continue;
                    }
                    if self.is_cave(wx, y, wz, col.height) {
                        // Don't open a hole right under water
                        if ch.get(lx, y + 1, lz) == WATER {
                            continue;
                        }
                        ch.set(lx, y, lz, if y <= 10 { LAVA } else { AIR });
                        // keep grass on top of dirt that loses its cover
                        if y > 1 && ch.get(lx, y - 1, lz) == DIRT && y - 1 == col.height - 1 {
                            ch.set(lx, y - 1, lz, GRASS);
                        }
                    }
                }
            }
        }

        // ---- Ores ----
        let ores: [(u8, i32, i32, i32); 7] = [
            // block, veins per chunk, max y, vein size
            (COAL_ORE, 20, 128, 12),
            (IRON_ORE, 18, 64, 8),
            (GOLD_ORE, 2, 32, 8),
            (REDSTONE_ORE, 8, 16, 7),
            (DIAMOND_ORE, 1, 16, 7),
            (LAPIS_ORE, 1, 32, 6),
            (GRAVEL, 6, 100, 24),
        ];
        for &(ore, count, max_y, size) in &ores {
            for _ in 0..count {
                let mut x = rng.range(16) as f32;
                let mut y = rng.range(max_y) as f32;
                let mut z = rng.range(16) as f32;
                let (dx, dy, dz) = (rng.uniform(-1.0, 1.0), rng.uniform(-0.5, 0.5), rng.uniform(-1.0, 1.0));
                for _ in 0..size {
                    let (ix, iy, iz) = (x as i32, y as i32, z as i32);
                    for (ox, oy, oz) in [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)] {
                        let (px, py, pz) = (ix + ox, iy + oy, iz + oz);
                        if (0..16).contains(&px) && (1..CHUNK_H as i32).contains(&py) && (0..16).contains(&pz) && ch.get(px, py, pz) == STONE && rng.chance(0.7) {
                            ch.set(px, py, pz, ore);
                        }
                    }
                    x += dx;
                    y += dy;
                    z += dz;
                }
            }
        }

        // ---- Surface plants (inside this chunk only) ----
        for lz in 0..16i32 {
            for lx in 0..16i32 {
                let col = &cols[(lz * 16 + lx) as usize];
                let h = col.height;
                if h + 1 >= CHUNK_H as i32 || h < SEA_LEVEL {
                    continue;
                }
                let ground = ch.get(lx, h, lz);
                if ch.get(lx, h + 1, lz) != AIR {
                    continue;
                }
                let r = rng.next_f32();
                let biome = col.biome;
                if ground == GRASS {
                    let (grass_p, flower_p, fern_p) = match biome {
                        Biome::Plains => (0.22, 0.03, 0.0),
                        Biome::Forest | Biome::BirchForest => (0.10, 0.02, 0.0),
                        Biome::Taiga | Biome::SnowyTaiga => (0.06, 0.0, 0.08),
                        Biome::Jungle => (0.30, 0.01, 0.12),
                        Biome::Savanna => (0.30, 0.0, 0.0),
                        Biome::Swamp => (0.10, 0.01, 0.0),
                        Biome::SnowyTundra => (0.01, 0.0, 0.0),
                        _ => (0.05, 0.005, 0.0),
                    };
                    if r < grass_p {
                        ch.set(lx, h + 1, lz, SHORT_GRASS);
                    } else if r < grass_p + fern_p {
                        ch.set(lx, h + 1, lz, FERN);
                    } else if r < grass_p + fern_p + flower_p {
                        let f = if biome == Biome::Swamp { BLUE_ORCHID } else if rng.chance(0.5) { DANDELION } else { POPPY };
                        ch.set(lx, h + 1, lz, f);
                    } else if r > 0.9993 && biome != Biome::SnowyTundra {
                        ch.set(lx, h + 1, lz, PUMPKIN);
                        ch.set_meta(lx, h + 1, lz, rng.range(4) as u8);
                    }
                } else if ground == SAND && biome == Biome::Desert {
                    if r < 0.006 && lx > 0 && lx < 15 && lz > 0 && lz < 15 {
                        let hgt = 1 + rng.range(3);
                        for i in 1..=hgt {
                            ch.set(lx, h + i, lz, CACTUS);
                        }
                    } else if r > 0.992 {
                        ch.set(lx, h + 1, lz, DEAD_BUSH);
                    }
                }
                // Sugar cane next to water
                if (ground == GRASS || ground == SAND || ground == DIRT) && h == SEA_LEVEL && rng.chance(0.12) {
                    let near_water = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
                        let (nx, nz) = (lx + dx, lz + dz);
                        (0..16).contains(&nx) && (0..16).contains(&nz) && ch.get(nx, h, nz) == WATER
                    });
                    if near_water {
                        let hgt = 1 + rng.range(3);
                        for i in 1..=hgt {
                            ch.set(lx, h + i, lz, SUGAR_CANE);
                        }
                    }
                }
            }
        }

        // ---- Trees (from this chunk and neighbours that may overhang) ----
        for ncz in cz - 1..=cz + 1 {
            for ncx in cx - 1..=cx + 1 {
                self.place_trees(&mut ch, ncx, ncz);
            }
        }

        // ---- Snow cover ----
        for lz in 0..16i32 {
            for lx in 0..16i32 {
                let i = (lz * 16 + lx) as usize;
                let biome = Biome::from_u8(ch.biomes[i]);
                let snowy = biome.is_snowy() || (biome == Biome::Mountains && cols[i].height > SEA_LEVEL + 38);
                if !snowy {
                    continue;
                }
                for y in (1..CHUNK_H as i32 - 1).rev() {
                    let b = ch.get(lx, y, lz);
                    if b == AIR {
                        continue;
                    }
                    if b == SHORT_GRASS || b == FERN {
                        ch.set(lx, y, lz, SNOW_LAYER);
                    } else if (is_opaque(b) || is_leaves(b)) && b != ICE {
                        ch.set(lx, y + 1, lz, SNOW_LAYER);
                    }
                    break;
                }
            }
        }

        ch.compute_heightmap();
        ch
    }

    fn place_trees(&self, ch: &mut Chunk, tcx: i32, tcz: i32) {
        let mut rng = Random::new(hash3(self.seed ^ 0x7EE5, tcx, 1, tcz));
        // Biome at chunk centre decides density
        let centre = self.column(tcx * 16 + 8, tcz * 16 + 8);
        let (count, extra_p) = match centre.biome {
            Biome::Forest => (8, 0.0),
            Biome::BirchForest => (8, 0.0),
            Biome::Taiga => (8, 0.0),
            Biome::SnowyTaiga => (7, 0.0),
            Biome::Jungle => (18, 0.0),
            Biome::Swamp => (2, 0.0),
            Biome::Savanna => (1, 0.5),
            Biome::Plains => (0, 0.15),
            Biome::SnowyTundra => (0, 0.1),
            Biome::Mountains => (1, 0.3),
            _ => (0, 0.0),
        };
        let mut n = count;
        if rng.chance(extra_p) {
            n += 1;
        }
        for _ in 0..n {
            let lx = rng.range(16);
            let lz = rng.range(16);
            let kind_r = rng.next_f32();
            let tree_seed = rng.next_u64();
            let wx = tcx * 16 + lx;
            let wz = tcz * 16 + lz;
            let col = self.column(wx, wz);
            let h = col.height;
            if h < SEA_LEVEL || h > CHUNK_H as i32 - 20 {
                continue;
            }
            if !matches!(
                col.biome,
                Biome::Forest | Biome::BirchForest | Biome::Taiga | Biome::SnowyTaiga | Biome::Jungle | Biome::Swamp | Biome::Savanna | Biome::Plains | Biome::SnowyTundra | Biome::Mountains
            ) {
                continue;
            }
            // Skip if the base was carved out by a cave (check same rule)
            if self.is_cave(wx, h, wz, h) {
                continue;
            }
            let mut tr = Random::new(tree_seed);
            match col.biome {
                Biome::Forest => {
                    if kind_r < 0.2 {
                        self.tree_oak(ch, wx, h + 1, wz, &mut tr, BIRCH_LOG, BIRCH_LEAVES, 5)
                    } else if kind_r < 0.3 {
                        self.tree_big_oak(ch, wx, h + 1, wz, &mut tr)
                    } else {
                        self.tree_oak(ch, wx, h + 1, wz, &mut tr, OAK_LOG, OAK_LEAVES, 4)
                    }
                }
                Biome::BirchForest => self.tree_oak(ch, wx, h + 1, wz, &mut tr, BIRCH_LOG, BIRCH_LEAVES, 5),
                Biome::Taiga | Biome::SnowyTaiga | Biome::SnowyTundra | Biome::Mountains => {
                    if col.biome == Biome::Mountains && kind_r < 0.5 {
                        self.tree_oak(ch, wx, h + 1, wz, &mut tr, OAK_LOG, OAK_LEAVES, 4)
                    } else {
                        self.tree_spruce(ch, wx, h + 1, wz, &mut tr)
                    }
                }
                Biome::Jungle => {
                    if kind_r < 0.12 {
                        self.tree_jungle_big(ch, wx, h + 1, wz, &mut tr)
                    } else if kind_r < 0.55 {
                        self.tree_bush(ch, wx, h + 1, wz, &mut tr)
                    } else {
                        self.tree_oak(ch, wx, h + 1, wz, &mut tr, JUNGLE_LOG, JUNGLE_LEAVES, 6)
                    }
                }
                Biome::Swamp => self.tree_oak(ch, wx, h + 1, wz, &mut tr, OAK_LOG, OAK_LEAVES, 5),
                Biome::Savanna => self.tree_acacia(ch, wx, h + 1, wz, &mut tr),
                Biome::Plains => self.tree_oak(ch, wx, h + 1, wz, &mut tr, OAK_LOG, OAK_LEAVES, 4),
                _ => {}
            }
        }
    }

    // --- tree helpers (world coordinates; writes only within `ch`) ---
    fn put(ch: &mut Chunk, x: i32, y: i32, z: i32, b: u8, force: bool) {
        let lx = x - ch.cx * 16;
        let lz = z - ch.cz * 16;
        if !(0..16).contains(&lx) || !(0..16).contains(&lz) || y < 1 || y >= CHUNK_H as i32 {
            return;
        }
        let cur = ch.get(lx, y, lz);
        if force || cur == AIR || cur == SHORT_GRASS || cur == FERN || cur == SNOW_LAYER || (is_leaves(cur) && is_log(b)) {
            ch.set(lx, y, lz, b);
        }
    }
    fn leaf(ch: &mut Chunk, x: i32, y: i32, z: i32, b: u8) {
        Self::put(ch, x, y, z, b, false);
    }
    fn trunk(ch: &mut Chunk, x: i32, y0: i32, z: i32, h: i32, log: u8) {
        // dirt under trunk
        Self::put(ch, x, y0 - 1, z, DIRT, true);
        for y in y0..y0 + h {
            Self::put(ch, x, y, z, log, true);
        }
    }

    fn tree_oak(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random, log: u8, leaves: u8, base_h: i32) {
        let h = base_h + r.range(3);
        // Leaves: two wide layers + two narrow
        for dy in -3..=0i32 {
            let ly = y + h + dy;
            let rad: i32 = if dy >= -1 { 1 } else { 2 };
            for dx in -rad..=rad {
                for dz in -rad..=rad {
                    let corner = dx.abs() == rad && dz.abs() == rad;
                    if corner && (dy == 0 || r.chance(0.5)) {
                        continue;
                    }
                    Self::leaf(ch, x + dx, ly, z + dz, leaves);
                }
            }
        }
        Self::trunk(ch, x, y, z, h, log);
    }

    fn tree_big_oak(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random) {
        let h = 7 + r.range(4);
        for dy in -3..=1i32 {
            let ly = y + h + dy - 1;
            let rad = if dy == 1 { 2 } else if dy == -3 { 2 } else { 3 };
            for dx in -rad..=rad {
                for dz in -rad..=rad {
                    if dx * dx + dz * dz > rad * rad + 1 {
                        continue;
                    }
                    if r.chance(0.08) {
                        continue;
                    }
                    Self::leaf(ch, x + dx, ly, z + dz, OAK_LEAVES);
                }
            }
        }
        Self::trunk(ch, x, y, z, h, OAK_LOG);
        // A couple of branches
        for _ in 0..2 {
            let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][r.range(4) as usize];
            let by = y + h - 3 - r.range(2);
            Self::put(ch, x + dx, by, z + dz, OAK_LOG, true);
        }
    }

    fn tree_spruce(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random) {
        let h = 7 + r.range(4);
        let leaf_start = 2 + r.range(2);
        let mut rad: i32 = 0;
        let mut max_rad = 1;
        // From top down: cone of alternating radii
        Self::leaf(ch, x, y + h, z, SPRUCE_LEAVES);
        Self::leaf(ch, x, y + h + 1, z, SPRUCE_LEAVES);
        for ly in (y + leaf_start..y + h + 1).rev() {
            for dx in -rad..=rad {
                for dz in -rad..=rad {
                    if rad > 0 && dx.abs() == rad && dz.abs() == rad {
                        continue;
                    }
                    Self::leaf(ch, x + dx, ly, z + dz, SPRUCE_LEAVES);
                }
            }
            if rad >= max_rad {
                rad = 0;
                max_rad = (max_rad + 1).min(3);
            } else {
                rad += 1;
            }
        }
        Self::trunk(ch, x, y, z, h, SPRUCE_LOG);
    }

    fn tree_acacia(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random) {
        let h = 5 + r.range(2);
        Self::put(ch, x, y - 1, z, DIRT, true);
        let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][r.range(4) as usize];
        let bend = h - 2 - r.range(2);
        let (mut px, mut pz) = (x, z);
        for i in 0..h {
            if i >= bend {
                px += dx;
                pz += dz;
            }
            Self::put(ch, px, y + i, pz, ACACIA_LOG, true);
        }
        let top = y + h;
        for (rad, ly) in [(3i32, top - 1), (2i32, top)] {
            for ox in -rad..=rad {
                for oz in -rad..=rad {
                    if ox.abs() == rad && oz.abs() == rad {
                        continue;
                    }
                    if ly == top - 1 && (ox.abs() + oz.abs() > 4) {
                        continue;
                    }
                    Self::leaf(ch, px + ox, ly, pz + oz, ACACIA_LEAVES);
                }
            }
        }
        // second small canopy
        let (sx, sz) = (x - dx, z - dz);
        Self::put(ch, sx, y + bend, sz, ACACIA_LOG, true);
        Self::put(ch, sx - dx, y + bend + 1, sz - dz, ACACIA_LOG, true);
        for ox in -2..=2i32 {
            for oz in -2..=2i32 {
                if ox.abs() == 2 && oz.abs() == 2 {
                    continue;
                }
                Self::leaf(ch, sx - dx + ox, y + bend + 2, sz - dz + oz, ACACIA_LEAVES);
            }
        }
    }

    fn tree_bush(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random) {
        Self::put(ch, x, y, z, JUNGLE_LOG, true);
        for dy in 0..=2i32 {
            let rad = 2 - dy;
            for dx in -rad..=rad {
                for dz in -rad..=rad {
                    if dx.abs() + dz.abs() > rad + 1 || r.chance(0.1) {
                        continue;
                    }
                    Self::leaf(ch, x + dx, y + dy, z + dz, OAK_LEAVES);
                }
            }
        }
    }

    fn tree_jungle_big(&self, ch: &mut Chunk, x: i32, y: i32, z: i32, r: &mut Random) {
        let h = 14 + r.range(10);
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            Self::trunk(ch, x + ox, y, z + oz, h, JUNGLE_LOG);
        }
        for dy in -2..=1i32 {
            let rad = if dy == 1 { 2 } else { 4 - dy.abs().min(1) };
            for dx in -rad..=rad + 1 {
                for dz in -rad..=rad + 1 {
                    let cx = dx as f32 - 0.5;
                    let cz = dz as f32 - 0.5;
                    if cx * cx + cz * cz > (rad as f32 + 0.6).powi(2) {
                        continue;
                    }
                    Self::leaf(ch, x + dx, y + h + dy, z + dz, JUNGLE_LEAVES);
                }
            }
        }
        // side branches with small leaf blobs
        for i in 0..3 {
            let by = y + h / 2 + i * 3 + r.range(2);
            if by >= y + h - 2 {
                break;
            }
            let (dx, dz) = [(2, 0), (-1, 0), (0, 2), (0, -1)][r.range(4) as usize];
            Self::put(ch, x + dx, by, z + dz, JUNGLE_LOG, true);
            for ox in -1..=1 {
                for oz in -1..=1 {
                    Self::leaf(ch, x + dx + ox, by + 1, z + dz + oz, JUNGLE_LEAVES);
                }
            }
        }
    }
}

/// Used by the game to pick a spawn point.
pub fn find_spawn(generator: &Generator) -> (i32, i32, i32) {
    for r in 0..200 {
        for i in 0..8 {
            let a = i as f32 / 8.0 * std::f32::consts::TAU;
            let x = (a.cos() * r as f32 * 16.0) as i32;
            let z = (a.sin() * r as f32 * 16.0) as i32;
            let c = generator.column(x, z);
            if c.height > SEA_LEVEL && !c.biome.is_ocean() && c.biome != Biome::River && c.height < SEA_LEVEL + 20 {
                return (x, c.height + 1, z);
            }
        }
    }
    (0, 100, 0)
}

/// Debug: render a top-down biome/height map to a PNG.
pub fn debug_map(seed: u64, path: &str, size: usize, step: i32) {
    let g = Generator::new(seed);
    let mut img = crate::image::Image::new(size, size);
    for py in 0..size {
        for px in 0..size {
            let x = (px as i32 - size as i32 / 2) * step;
            let z = (py as i32 - size as i32 / 2) * step;
            let c = g.column(x, z);
            let base: u32 = match c.biome {
                Biome::Ocean => 0x2040C0,
                Biome::DeepOcean => 0x102080,
                Biome::FrozenOcean => 0x8080E0,
                Biome::River => 0x3060FF,
                Biome::Beach => 0xE8DCA0,
                Biome::SnowyBeach => 0xF0F0E0,
                Biome::Plains => 0x8DB360,
                Biome::Forest => 0x056621,
                Biome::BirchForest => 0x307444,
                Biome::Taiga => 0x0B6659,
                Biome::SnowyTaiga => 0x31554A,
                Biome::SnowyTundra => 0xFFFFFF,
                Biome::Desert => 0xFA9418,
                Biome::Savanna => 0xBDB25F,
                Biome::Jungle => 0x537B09,
                Biome::Swamp => 0x07F9B2,
                Biome::Mountains => 0x606060,
                Biome::SnowyMountains => 0xA0A0A0,
            };
            let shade = 0.7 + (c.height - SEA_LEVEL) as f32 / 120.0;
            let f = |v: u32| ((v & 255) as f32 * shade).clamp(0.0, 255.0) as u8;
            img.set(px, py, [f(base >> 16), f(base >> 8), f(base), 255]);
        }
    }
    let _ = img.save_png(path);
}
