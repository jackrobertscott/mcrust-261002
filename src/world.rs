//! Chunk storage, block access, light propagation and background chunk generation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::block::{self, *};
use crate::worldgen::{Biome, Generator};

pub const CHUNK_H: usize = 128;
pub const SECTIONS: usize = CHUNK_H / 16;
pub const SEA_LEVEL: i32 = 62;
const CHUNK_VOL: usize = 16 * 16 * CHUNK_H;

#[inline]
fn idx(x: i32, y: i32, z: i32) -> usize {
    ((y as usize * 16 + z as usize) * 16) + x as usize
}

pub struct Chunk {
    pub cx: i32,
    pub cz: i32,
    pub blocks: Vec<u8>,
    pub meta: Vec<u8>,
    /// sky light in high nibble, block light in low nibble
    pub light: Vec<u8>,
    /// y of highest block that blocks sky light, + 1
    pub heightmap: [u8; 256],
    pub biomes: [u8; 256],
    pub grass_color: [u32; 256],
    pub foliage_color: [u32; 256],
    /// sections that need re-meshing
    pub dirty: [bool; SECTIONS],
    pub modified: bool,
}

impl Chunk {
    pub fn new(cx: i32, cz: i32) -> Chunk {
        Chunk {
            cx,
            cz,
            blocks: vec![0; CHUNK_VOL],
            meta: vec![0; CHUNK_VOL],
            light: vec![0; CHUNK_VOL],
            heightmap: [0; 256],
            biomes: [0; 256],
            grass_color: [0x91BD59; 256],
            foliage_color: [0x77AB2F; 256],
            dirty: [true; SECTIONS],
            modified: false,
        }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 || y >= CHUNK_H as i32 {
            return AIR;
        }
        self.blocks[idx(x, y, z)]
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: u8) {
        if y < 0 || y >= CHUNK_H as i32 {
            return;
        }
        let i = idx(x, y, z);
        self.blocks[i] = b;
        self.meta[i] = 0;
    }
    #[inline]
    pub fn get_meta(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 || y >= CHUNK_H as i32 {
            return 0;
        }
        self.meta[idx(x, y, z)]
    }
    #[inline]
    pub fn set_meta(&mut self, x: i32, y: i32, z: i32, m: u8) {
        if y < 0 || y >= CHUNK_H as i32 {
            return;
        }
        self.meta[idx(x, y, z)] = m;
    }
    #[inline]
    pub fn light_at(&self, x: i32, y: i32, z: i32) -> u8 {
        if y >= CHUNK_H as i32 {
            return 0xF0;
        }
        if y < 0 {
            return 0;
        }
        self.light[idx(x, y, z)]
    }
    pub fn biome(&self, x: i32, z: i32) -> Biome {
        Biome::from_u8(self.biomes[(z * 16 + x) as usize])
    }

    pub fn compute_heightmap(&mut self) {
        for z in 0..16 {
            for x in 0..16 {
                let mut h = 0;
                for y in (0..CHUNK_H as i32).rev() {
                    if def(self.get(x, y, z)).light_opacity > 0 {
                        h = y + 1;
                        break;
                    }
                }
                self.heightmap[(z * 16 + x) as usize] = h as u8;
            }
        }
    }

    /// Initial lighting computed entirely inside the chunk (worker thread).
    pub fn compute_initial_light(&mut self) {
        let mut queue: VecDeque<(i32, i32, i32)> = VecDeque::new();
        // Sky light: straight down columns
        for z in 0..16 {
            for x in 0..16 {
                let mut level: i32 = 15;
                for y in (0..CHUNK_H as i32).rev() {
                    let op = def(self.get(x, y, z)).light_opacity as i32;
                    if op > 0 {
                        level = (level - op).max(0);
                    }
                    let i = idx(x, y, z);
                    self.light[i] = (level as u8) << 4 | (self.light[i] & 0x0F);
                    if level == 0 {
                        break;
                    }
                }
            }
        }
        // Seed horizontal spreading from cells with sky light next to darker cells
        for y in 0..CHUNK_H as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let i = idx(x, y, z);
                    let b = self.blocks[i];
                    let emit = def(b).light_emit;
                    if emit > 0 {
                        self.light[i] = (self.light[i] & 0xF0) | emit;
                        queue.push_back((x, y, z));
                        continue;
                    }
                    let s = self.light[i] >> 4;
                    if s > 1 {
                        // only push if a neighbour is darker
                        let mut needs = false;
                        for (dx, dy, dz) in DIRS {
                            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                            if !(0..16).contains(&nx) || !(0..16).contains(&nz) || ny < 0 || ny >= CHUNK_H as i32 {
                                continue;
                            }
                            if (self.light[idx(nx, ny, nz)] >> 4) + 1 < s && def(self.blocks[idx(nx, ny, nz)]).light_opacity < 15 {
                                needs = true;
                                break;
                            }
                        }
                        if needs {
                            queue.push_back((x, y, z));
                        }
                    }
                }
            }
        }
        // BFS for both channels inside the chunk
        while let Some((x, y, z)) = queue.pop_front() {
            let l = self.light[idx(x, y, z)];
            let (s, bl) = ((l >> 4) as i32, (l & 15) as i32);
            for (dx, dy, dz) in DIRS {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                if !(0..16).contains(&nx) || !(0..16).contains(&nz) || ny < 0 || ny >= CHUNK_H as i32 {
                    continue;
                }
                let ni = idx(nx, ny, nz);
                let op = (def(self.blocks[ni]).light_opacity as i32).max(1);
                if op >= 15 {
                    continue;
                }
                let nl = self.light[ni];
                let (ns, nb) = ((nl >> 4) as i32, (nl & 15) as i32);
                let mut changed = false;
                let mut new_s = ns;
                let mut new_b = nb;
                if s - op > ns {
                    new_s = s - op;
                    changed = true;
                }
                if bl - op > nb {
                    new_b = bl - op;
                    changed = true;
                }
                if changed {
                    self.light[ni] = ((new_s as u8) << 4) | new_b as u8;
                    queue.push_back((nx, ny, nz));
                }
            }
        }
    }
}

pub const DIRS: [(i32, i32, i32); 6] = [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];

#[derive(Clone, Debug)]
pub enum BlockEntity {
    Furnace(crate::inventory::FurnaceState),
    Chest(Vec<crate::item::ItemStack>),
}

pub struct World {
    pub chunks: HashMap<(i32, i32), Box<Chunk>>,
    pub gen: Arc<Generator>,
    pub seed: u64,
    pending: HashSet<(i32, i32)>,
    job_tx: Sender<(i32, i32)>,
    result_rx: Receiver<Chunk>,
    /// Ticks since world creation. 24000 per day.
    pub time: i64,
    pub block_entities: HashMap<(i32, i32, i32), BlockEntity>,
    /// Saved (modified) chunks that were unloaded
    pub saved: HashMap<(i32, i32), Box<Chunk>>,
}

impl World {
    pub fn new(seed: u64) -> World {
        let gen = Arc::new(Generator::new(seed));
        let (job_tx, job_rx) = channel::<(i32, i32)>();
        let (result_tx, result_rx) = channel::<Chunk>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).clamp(1, 6);
        for _ in 0..threads {
            let rx = job_rx.clone();
            let tx = result_tx.clone();
            let g = gen.clone();
            std::thread::spawn(move || loop {
                let job = {
                    let lock = rx.lock().unwrap();
                    lock.recv()
                };
                let Ok((cx, cz)) = job else { break };
                let mut ch = g.generate(cx, cz);
                ch.compute_initial_light();
                if tx.send(ch).is_err() {
                    break;
                }
            });
        }
        World {
            chunks: HashMap::new(),
            gen,
            seed,
            pending: HashSet::new(),
            job_tx,
            result_rx,
            time: 1000,
            block_entities: HashMap::new(),
            saved: HashMap::new(),
        }
    }

    /// Ask for a chunk to be generated (non-blocking).
    pub fn request(&mut self, cx: i32, cz: i32) {
        if self.chunks.contains_key(&(cx, cz)) || self.pending.contains(&(cx, cz)) {
            return;
        }
        if let Some(ch) = self.saved.remove(&(cx, cz)) {
            self.insert_chunk(ch);
            return;
        }
        self.pending.insert((cx, cz));
        let _ = self.job_tx.send((cx, cz));
    }
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Receive finished chunks from the workers. Returns number inserted.
    pub fn receive_chunks(&mut self, max: usize) -> usize {
        let mut n = 0;
        while n < max {
            match self.result_rx.try_recv() {
                Ok(ch) => {
                    let key = (ch.cx, ch.cz);
                    if !self.pending.remove(&key) {
                        continue; // no longer wanted
                    }
                    self.insert_chunk(Box::new(ch));
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Cancel pending requests outside the given radius.
    pub fn retain_pending(&mut self, f: impl Fn(i32, i32) -> bool) {
        self.pending.retain(|&(x, z)| f(x, z));
    }

    fn insert_chunk(&mut self, mut ch: Box<Chunk>) {
        let (cx, cz) = (ch.cx, ch.cz);
        ch.dirty = [true; SECTIONS];
        self.chunks.insert((cx, cz), ch);
        // Mark neighbours dirty (their border faces/light may change)
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if let Some(n) = self.chunks.get_mut(&(cx + dx, cz + dz)) {
                n.dirty = [true; SECTIONS];
            }
        }
        // Propagate light across the borders with loaded neighbours.
        let mut q: VecDeque<(i32, i32, i32)> = VecDeque::new();
        let bx = cx * 16;
        let bz = cz * 16;
        for y in 0..CHUNK_H as i32 {
            for i in 0..16 {
                // this chunk's edges
                q.push_back((bx + i, y, bz));
                q.push_back((bx + i, y, bz + 15));
                q.push_back((bx, y, bz + i));
                q.push_back((bx + 15, y, bz + i));
                // neighbours' facing edges
                q.push_back((bx + i, y, bz - 1));
                q.push_back((bx + i, y, bz + 16));
                q.push_back((bx - 1, y, bz + i));
                q.push_back((bx + 16, y, bz + i));
            }
        }
        self.propagate_increase(q, true);
    }

    pub fn unload_chunk(&mut self, cx: i32, cz: i32) {
        if let Some(ch) = self.chunks.remove(&(cx, cz)) {
            if ch.modified {
                self.saved.insert((cx, cz), ch);
            }
        }
    }

    #[inline]
    pub fn chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        self.chunks.get(&(cx, cz)).map(|b| &**b)
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 || y >= CHUNK_H as i32 {
            return AIR;
        }
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.blocks[idx(x & 15, y, z & 15)],
            None => AIR,
        }
    }
    pub fn is_loaded(&self, x: i32, z: i32) -> bool {
        self.chunks.contains_key(&(x >> 4, z >> 4))
    }
    pub fn get_meta(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 || y >= CHUNK_H as i32 {
            return 0;
        }
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.meta[idx(x & 15, y, z & 15)],
            None => 0,
        }
    }
    /// Raw light byte (sky<<4 | block)
    pub fn light(&self, x: i32, y: i32, z: i32) -> u8 {
        if y >= CHUNK_H as i32 {
            return 0xF0;
        }
        if y < 0 {
            return 0;
        }
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.light[idx(x & 15, y, z & 15)],
            None => 0xF0,
        }
    }
    pub fn sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.light(x, y, z) >> 4
    }
    pub fn block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.light(x, y, z) & 15
    }
    fn set_light_raw(&mut self, x: i32, y: i32, z: i32, v: u8) {
        if let Some(c) = self.chunks.get_mut(&(x >> 4, z >> 4)) {
            c.light[idx(x & 15, y, z & 15)] = v;
            c.dirty[(y / 16) as usize] = true;
            // light at section borders affects neighbouring sections' meshes
            let ly = y & 15;
            if ly == 0 && y > 0 {
                c.dirty[(y / 16 - 1) as usize] = true;
            }
            if ly == 15 && (y / 16 + 1) < SECTIONS as i32 {
                c.dirty[(y / 16 + 1) as usize] = true;
            }
            let (lx, lz) = (x & 15, z & 15);
            let sec = (y / 16) as usize;
            let mut mark = |dx: i32, dz: i32| {
                if let Some(n) = self.chunks.get_mut(&((x >> 4) + dx, (z >> 4) + dz)) {
                    n.dirty[sec] = true;
                }
            };
            if lx == 0 {
                mark(-1, 0);
            }
            if lx == 15 {
                mark(1, 0);
            }
            if lz == 0 {
                mark(0, -1);
            }
            if lz == 15 {
                mark(0, 1);
            }
        }
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.heightmap[((z & 15) * 16 + (x & 15)) as usize] as i32,
            None => 0,
        }
    }

    pub fn biome(&self, x: i32, z: i32) -> Biome {
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.biome(x & 15, z & 15),
            None => Biome::Plains,
        }
    }

    pub fn set_meta(&mut self, x: i32, y: i32, z: i32, m: u8) {
        if y < 0 || y >= CHUNK_H as i32 {
            return;
        }
        if let Some(c) = self.chunks.get_mut(&(x >> 4, z >> 4)) {
            c.meta[idx(x & 15, y, z & 15)] = m;
            c.dirty[(y / 16) as usize] = true;
            c.modified = true;
        }
    }

    /// Change a block, updating light, heightmap and dirty flags.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, b: u8, meta: u8) {
        if y < 0 || y >= CHUNK_H as i32 {
            return;
        }
        let (cx, cz) = (x >> 4, z >> 4);
        let (lx, lz) = (x & 15, z & 15);
        let old;
        {
            let Some(c) = self.chunks.get_mut(&(cx, cz)) else { return };
            let i = idx(lx, y, lz);
            old = c.blocks[i];
            c.blocks[i] = b;
            c.meta[i] = meta;
            c.modified = true;
            let sec = (y / 16) as usize;
            c.dirty[sec] = true;
            if y & 15 == 0 && sec > 0 {
                c.dirty[sec - 1] = true;
            }
            if y & 15 == 15 && sec + 1 < SECTIONS {
                c.dirty[sec + 1] = true;
            }
            // heightmap
            let hi = (lz * 16 + lx) as usize;
            let h = c.heightmap[hi] as i32;
            if def(b).light_opacity > 0 && y + 1 > h {
                c.heightmap[hi] = (y + 1) as u8;
            } else if def(b).light_opacity == 0 && y + 1 == h {
                let mut nh = 0;
                for yy in (0..y).rev() {
                    if def(c.blocks[idx(lx, yy, lz)]).light_opacity > 0 {
                        nh = yy + 1;
                        break;
                    }
                }
                c.heightmap[hi] = nh as u8;
            }
        }
        // neighbour chunk meshes
        for (dx, dz, cond) in [(-1, 0, lx == 0), (1, 0, lx == 15), (0, -1, lz == 0), (0, 1, lz == 15)] {
            if cond {
                if let Some(n) = self.chunks.get_mut(&(cx + dx, cz + dz)) {
                    n.dirty[(y / 16) as usize] = true;
                }
            }
        }
        if old == FURNACE || old == FURNACE_LIT || old == CHEST {
            if !(b == FURNACE || b == FURNACE_LIT) || old == CHEST {
                // removed by caller (drops handled by game)
            }
        }
        self.update_light(x, y, z, old, b);
    }

    fn update_light(&mut self, x: i32, y: i32, z: i32, old: u8, new: u8) {
        let od = def(old);
        let nd = def(new);
        // ---- block light ----
        if od.light_emit != nd.light_emit || od.light_opacity != nd.light_opacity {
            let cur = self.block_light(x, y, z);
            let mut inc = VecDeque::new();
            if cur > 0 {
                let mut dec = VecDeque::new();
                let l = self.light(x, y, z);
                self.set_light_raw(x, y, z, l & 0xF0);
                dec.push_back((x, y, z, cur));
                self.propagate_decrease(dec, false, &mut inc);
            }
            if nd.light_emit > 0 {
                let l = self.light(x, y, z);
                self.set_light_raw(x, y, z, (l & 0xF0) | nd.light_emit);
                inc.push_back((x, y, z));
            }
            // pull light from neighbours into a now-transparent block
            for (dx, dy, dz) in DIRS {
                inc.push_back((x + dx, y + dy, z + dz));
            }
            self.propagate_increase(inc, false);
        }
        // ---- sky light ----
        if od.light_opacity != nd.light_opacity {
            let cur = self.sky_light(x, y, z);
            let mut inc = VecDeque::new();
            if cur > 0 {
                let mut dec = VecDeque::new();
                let l = self.light(x, y, z);
                self.set_light_raw(x, y, z, l & 0x0F);
                dec.push_back((x, y, z, cur));
                self.propagate_decrease(dec, true, &mut inc);
            }
            for (dx, dy, dz) in DIRS {
                inc.push_back((x + dx, y + dy, z + dz));
            }
            if y + 1 >= CHUNK_H as i32 || self.sky_light(x, y + 1, z) == 15 {
                // directly exposed to sky
                if nd.light_opacity == 0 {
                    let l = self.light(x, y, z);
                    self.set_light_raw(x, y, z, (l & 0x0F) | 0xF0);
                    inc.push_back((x, y, z));
                }
            }
            self.propagate_increase(inc, true);
        }
    }

    fn get_ch(&self, l: u8, sky: bool) -> i32 {
        if sky { (l >> 4) as i32 } else { (l & 15) as i32 }
    }

    fn propagate_increase(&mut self, mut q: VecDeque<(i32, i32, i32)>, sky: bool) {
        let mut guard = 0usize;
        while let Some((x, y, z)) = q.pop_front() {
            guard += 1;
            if guard > 2_000_000 {
                break;
            }
            if y < 0 || y >= CHUNK_H as i32 || !self.is_loaded(x, z) {
                continue;
            }
            let lvl = self.get_ch(self.light(x, y, z), sky);
            if lvl <= 1 {
                continue;
            }
            for (dx, dy, dz) in DIRS {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                if ny < 0 || ny >= CHUNK_H as i32 || !self.is_loaded(nx, nz) {
                    continue;
                }
                let nb = self.get(nx, ny, nz);
                let op = def(nb).light_opacity as i32;
                if op >= 15 {
                    continue;
                }
                let newl = if sky && dy == -1 && lvl == 15 && op == 0 { 15 } else { lvl - op.max(1) };
                let nl = self.light(nx, ny, nz);
                if newl > self.get_ch(nl, sky) {
                    let v = if sky { (nl & 0x0F) | ((newl as u8) << 4) } else { (nl & 0xF0) | newl as u8 };
                    self.set_light_raw(nx, ny, nz, v);
                    q.push_back((nx, ny, nz));
                }
            }
        }
    }

    fn propagate_decrease(&mut self, mut q: VecDeque<(i32, i32, i32, u8)>, sky: bool, inc: &mut VecDeque<(i32, i32, i32)>) {
        while let Some((x, y, z, lvl)) = q.pop_front() {
            for (dx, dy, dz) in DIRS {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                if ny < 0 || ny >= CHUNK_H as i32 || !self.is_loaded(nx, nz) {
                    continue;
                }
                let nl = self.light(nx, ny, nz);
                let n = self.get_ch(nl, sky) as u8;
                if n == 0 {
                    continue;
                }
                let emit = if sky { 0 } else { def(self.get(nx, ny, nz)).light_emit };
                if (n < lvl || (sky && dy == -1 && lvl == 15 && n == 15)) && emit == 0 {
                    let v = if sky { nl & 0x0F } else { nl & 0xF0 };
                    self.set_light_raw(nx, ny, nz, v);
                    q.push_back((nx, ny, nz, n));
                } else {
                    inc.push_back((nx, ny, nz));
                }
            }
        }
    }

    pub fn grass_color(&self, x: i32, z: i32) -> u32 {
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(c) => c.grass_color[((z & 15) * 16 + (x & 15)) as usize],
            None => 0x91BD59,
        }
    }

    pub fn is_solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        block::is_solid(self.get(x, y, z))
    }
}
