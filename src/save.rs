//! Simple binary world saves: level data + modified chunks (RLE compressed).
//! Saves live in ~/.mcrust/saves/<folder>/.

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::game::{Game, Player};
use crate::item::ItemStack;
use crate::math::v3;
use crate::world::{BlockEntity, Chunk, World, CHUNK_H};

pub struct WorldInfo {
    pub folder: String,
    pub name: String,
    #[allow(dead_code)]
    pub seed: u64,
    pub last_played: u64,
    pub time: i64,
}

pub fn saves_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.mcrust/saves")
}

struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn stack(&mut self, s: &ItemStack) {
        self.u16(s.id);
        self.u8(s.count);
        self.u16(s.damage);
    }
    fn rle(&mut self, data: &[u8]) {
        let mut out = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let v = data[i];
            let mut n = 1;
            while i + n < data.len() && data[i + n] == v && n < 255 {
                n += 1;
            }
            out.push(n as u8);
            out.push(v);
            i += n;
        }
        self.u32(out.len() as u32);
        self.0.extend_from_slice(&out);
    }
}

struct R<'a> {
    d: &'a [u8],
    p: usize,
}
impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.p + n > self.d.len() {
            return None;
        }
        let s = &self.d[self.p..self.p + n];
        self.p += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn str(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        Some(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
    fn stack(&mut self) -> Option<ItemStack> {
        Some(ItemStack { id: self.u16()?, count: self.u8()?, damage: self.u16()? })
    }
    fn rle(&mut self, out: &mut [u8]) -> Option<()> {
        let n = self.u32()? as usize;
        let data = self.take(n)?;
        let mut o = 0;
        for pair in data.chunks(2) {
            let (cnt, v) = (pair[0] as usize, pair[1]);
            for _ in 0..cnt {
                if o < out.len() {
                    out[o] = v;
                }
                o += 1;
            }
        }
        Some(())
    }
}

const MAGIC: &[u8; 4] = b"MCRS";
const VERSION: u32 = 1;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn folder_name(name: &str) -> String {
    let mut s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.trim().is_empty() {
        s = "World".into();
    }
    let base = s.trim().to_string();
    let mut candidate = base.clone();
    let mut n = 1;
    while std::path::Path::new(&format!("{}/{}", saves_dir(), candidate)).exists() {
        candidate = format!("{base} ({n})");
        n += 1;
    }
    candidate
}

pub fn save_game(g: &Game) -> std::io::Result<()> {
    if g.world_folder.is_empty() {
        return Ok(());
    }
    let dir = format!("{}/{}", saves_dir(), g.world_folder);
    std::fs::create_dir_all(&dir)?;
    // ---- level ----
    let mut w = W(Vec::new());
    w.0.extend_from_slice(MAGIC);
    w.u32(VERSION);
    w.str(&g.world_name);
    w.u64(g.world.seed);
    w.u64(g.world.time as u64);
    w.u64(now_secs());
    let p = &g.player;
    for v in [p.body.pos.x, p.body.pos.y, p.body.pos.z, p.yaw, p.pitch, p.health, p.saturation, p.spawn.x, p.spawn.y, p.spawn.z] {
        w.f32(v);
    }
    w.i32(p.food);
    w.i32(p.air);
    w.u8(p.inv.selected as u8);
    for s in &p.inv.slots {
        w.stack(s);
    }
    // block entities
    w.u32(g.world.block_entities.len() as u32);
    for (&(x, y, z), be) in &g.world.block_entities {
        w.i32(x);
        w.i32(y);
        w.i32(z);
        match be {
            BlockEntity::Furnace(f) => {
                w.u8(0);
                w.stack(&f.input);
                w.stack(&f.fuel);
                w.stack(&f.output);
                w.i32(f.burn_time);
                w.i32(f.burn_total);
                w.i32(f.cook_time);
            }
            BlockEntity::Chest(v) => {
                w.u8(1);
                w.u32(v.len() as u32);
                for s in v {
                    w.stack(s);
                }
            }
        }
    }
    std::fs::File::create(format!("{dir}/level.dat"))?.write_all(&w.0)?;

    // ---- chunks: every modified chunk, loaded or unloaded ----
    let mut c = W(Vec::new());
    c.0.extend_from_slice(MAGIC);
    c.u32(VERSION);
    let mut list: Vec<&Chunk> = g.world.saved.values().map(|b| &**b).collect();
    list.extend(g.world.chunks.values().filter(|ch| ch.modified).map(|b| &**b));
    c.u32(list.len() as u32);
    for ch in list {
        c.i32(ch.cx);
        c.i32(ch.cz);
        c.rle(&ch.blocks);
        c.rle(&ch.meta);
    }
    std::fs::File::create(format!("{dir}/chunks.dat"))?.write_all(&c.0)?;
    Ok(())
}

pub fn read_info(folder: &str) -> Option<WorldInfo> {
    let mut d = Vec::new();
    std::fs::File::open(format!("{}/{}/level.dat", saves_dir(), folder)).ok()?.read_to_end(&mut d).ok()?;
    let mut r = R { d: &d, p: 0 };
    if r.take(4)? != MAGIC {
        return None;
    }
    let _v = r.u32()?;
    let name = r.str()?;
    let seed = r.u64()?;
    let time = r.u64()? as i64;
    let last_played = r.u64()?;
    Some(WorldInfo { folder: folder.to_string(), name, seed, last_played, time })
}

pub fn list_worlds() -> Vec<WorldInfo> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(saves_dir()) {
        for e in rd.flatten() {
            if let Some(name) = e.file_name().to_str() {
                if let Some(info) = read_info(name) {
                    v.push(info);
                }
            }
        }
    }
    v.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    v
}

pub fn delete_world(folder: &str) {
    if folder.is_empty() || folder.contains("..") || folder.contains('/') {
        return;
    }
    let _ = std::fs::remove_dir_all(format!("{}/{}", saves_dir(), folder));
}

/// Load a saved world into the game. Returns false on failure.
pub fn load_game(g: &mut Game, folder: &str) -> bool {
    let dir = format!("{}/{}", saves_dir(), folder);
    let mut d = Vec::new();
    if std::fs::File::open(format!("{dir}/level.dat")).and_then(|mut f| f.read_to_end(&mut d)).is_err() {
        return false;
    }
    let mut r = R { d: &d, p: 0 };
    let Some(parsed) = (|| -> Option<()> {
        if r.take(4)? != MAGIC {
            return None;
        }
        let _v = r.u32()?;
        let name = r.str()?;
        let seed = r.u64()?;
        let time = r.u64()? as i64;
        let _last = r.u64()?;
        let mut f = [0f32; 10];
        for v in f.iter_mut() {
            *v = r.f32()?;
        }
        let food = r.i32()?;
        let air = r.i32()?;
        let sel = r.u8()?;
        let mut slots = [ItemStack::EMPTY; 36];
        for s in slots.iter_mut() {
            *s = r.stack()?;
        }
        let n_be = r.u32()?;
        let mut bes = HashMap::new();
        for _ in 0..n_be {
            let pos = (r.i32()?, r.i32()?, r.i32()?);
            match r.u8()? {
                0 => {
                    let mut fs = crate::inventory::FurnaceState::new();
                    fs.input = r.stack()?;
                    fs.fuel = r.stack()?;
                    fs.output = r.stack()?;
                    fs.burn_time = r.i32()?;
                    fs.burn_total = r.i32()?;
                    fs.cook_time = r.i32()?;
                    bes.insert(pos, BlockEntity::Furnace(fs));
                }
                _ => {
                    let n = r.u32()? as usize;
                    let mut v = Vec::with_capacity(n);
                    for _ in 0..n {
                        v.push(r.stack()?);
                    }
                    bes.insert(pos, BlockEntity::Chest(v));
                }
            }
        }
        // build world
        g.reset_world(World::new(seed));
        g.world.time = time;
        g.world.block_entities = bes;
        g.world_name = name;
        g.world_folder = folder.to_string();
        let mut p = Player::new(v3(f[0], f[1], f[2]));
        p.yaw = f[3];
        p.pitch = f[4];
        p.health = f[5];
        p.saturation = f[6];
        p.spawn = v3(f[7], f[8], f[9]);
        p.food = food;
        p.air = air;
        p.inv.selected = (sel as usize).min(8);
        p.inv.slots = slots;
        g.player = p;
        Some(())
    })() else {
        return false;
    };
    let _ = parsed;
    // chunks
    let mut cd = Vec::new();
    if std::fs::File::open(format!("{dir}/chunks.dat")).and_then(|mut f| f.read_to_end(&mut cd)).is_ok() {
        let mut r = R { d: &cd, p: 0 };
        let _ = (|| -> Option<()> {
            if r.take(4)? != MAGIC {
                return None;
            }
            let _v = r.u32()?;
            let n = r.u32()?;
            for _ in 0..n {
                let cx = r.i32()?;
                let cz = r.i32()?;
                let mut ch = g.world.generator.generate(cx, cz);
                r.rle(&mut ch.blocks)?;
                r.rle(&mut ch.meta)?;
                ch.light.iter_mut().for_each(|l| *l = 0);
                ch.compute_heightmap();
                ch.compute_initial_light();
                ch.modified = true;
                g.world.saved.insert((cx, cz), Box::new(ch));
            }
            Some(())
        })();
    }
    let _ = CHUNK_H;
    true
}
