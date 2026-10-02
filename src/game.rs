//! Game state and simulation (20 ticks per second, like vanilla).

use std::collections::{HashSet, VecDeque};

use crate::block::{self, *};
use crate::entity::*;
use crate::inventory::{FurnaceState, Inventory};
use crate::item::{self, ItemStack};
use crate::math::{v3, Vec3};
use crate::noise::Random;
use crate::physics::{self, Aabb, Body, Hit};
use crate::platform::{key, Event, Window};
use crate::world::{BlockEntity, World, CHUNK_H, DIRS};
use crate::worldgen::Biome;

pub const REACH: f32 = 4.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Menu {
    None,
    Title,
    CreateWorld,
    Options,
    Controls,
    Pause,
    Death,
    Inventory,
    Crafting,
    Furnace,
    Chest,
    Loading,
    SelectWorld,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "Peaceful",
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }
    pub fn next(self) -> Difficulty {
        match self {
            Difficulty::Peaceful => Difficulty::Easy,
            Difficulty::Easy => Difficulty::Normal,
            Difficulty::Normal => Difficulty::Hard,
            Difficulty::Hard => Difficulty::Peaceful,
        }
    }
}

pub struct Options {
    pub render_distance: i32,
    pub fov: f32,
    pub sensitivity: f32,
    pub view_bobbing: bool,
    pub difficulty: Difficulty,
    pub gui_scale: u32, // 0 = auto
    pub clouds: bool,
}

impl Options {
    pub fn path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.mcrust_options.txt")
    }
    pub fn load() -> Options {
        let mut o = Options { render_distance: 8, fov: 70.0, sensitivity: 0.5, view_bobbing: true, difficulty: Difficulty::Normal, gui_scale: 0, clouds: true };
        if let Ok(s) = std::fs::read_to_string(Self::path()) {
            for line in s.lines() {
                let mut it = line.splitn(2, ':');
                let (Some(k), Some(v)) = (it.next(), it.next()) else { continue };
                match k {
                    "renderDistance" => o.render_distance = v.parse().unwrap_or(8),
                    "fov" => o.fov = v.parse().unwrap_or(70.0),
                    "mouseSensitivity" => o.sensitivity = v.parse().unwrap_or(0.5),
                    "bobView" => o.view_bobbing = v == "true",
                    "guiScale" => o.gui_scale = v.parse().unwrap_or(0),
                    "clouds" => o.clouds = v == "true",
                    "difficulty" => {
                        o.difficulty = match v {
                            "0" => Difficulty::Peaceful,
                            "1" => Difficulty::Easy,
                            "3" => Difficulty::Hard,
                            _ => Difficulty::Normal,
                        }
                    }
                    _ => {}
                }
            }
        }
        o.render_distance = o.render_distance.clamp(2, 16);
        o
    }
    pub fn save(&self) {
        let d = match self.difficulty {
            Difficulty::Peaceful => 0,
            Difficulty::Easy => 1,
            Difficulty::Normal => 2,
            Difficulty::Hard => 3,
        };
        let s = format!(
            "renderDistance:{}\nfov:{}\nmouseSensitivity:{}\nbobView:{}\nguiScale:{}\nclouds:{}\ndifficulty:{}\n",
            self.render_distance, self.fov, self.sensitivity, self.view_bobbing, self.gui_scale, self.clouds, d
        );
        let _ = std::fs::write(Self::path(), s);
    }
}

pub struct Player {
    pub body: Body,
    pub prev_pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub prev_health: f32,
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub food_timer: i32,
    pub air: i32,
    pub hurt_time: i32,
    pub invuln: i32,
    pub inv: Inventory,
    pub cursor: ItemStack,
    pub craft2: [ItemStack; 4],
    pub sprinting: bool,
    pub sneaking: bool,
    pub dead: bool,
    pub death_time: i32,
    pub swing: f32,
    pub prev_swing: f32,
    pub swinging: bool,
    pub eating: i32,
    pub breaking: Option<(i32, i32, i32)>,
    pub break_progress: f32,
    pub break_cooldown: i32,
    pub use_cooldown: i32,
    pub walk_dist: f32,
    pub prev_walk_dist: f32,
    pub bob: f32,
    pub prev_bob: f32,
    pub spawn: Vec3,
    pub fire: i32,
    pub equip: f32,
    pub prev_equip: f32,
    pub last_held: ItemStack,
    pub sprint_tap: i32,
    pub limb_swing: f32,
    pub limb_amount: f32,
    pub body_yaw: f32,
    pub bow_charge: i32,
    pub sleeping: i32,
    pub on_ladder: bool,
}

impl Player {
    pub fn new(pos: Vec3) -> Player {
        Player {
            body: Body::new(pos, 0.3, 1.8),
            prev_pos: pos,
            yaw: 0.0,
            pitch: 0.0,
            health: 20.0,
            prev_health: 20.0,
            food: 20,
            saturation: 5.0,
            exhaustion: 0.0,
            food_timer: 0,
            air: 300,
            hurt_time: 0,
            invuln: 0,
            inv: Inventory::new(),
            cursor: ItemStack::EMPTY,
            craft2: [ItemStack::EMPTY; 4],
            sprinting: false,
            sneaking: false,
            dead: false,
            death_time: 0,
            swing: 0.0,
            prev_swing: 0.0,
            swinging: false,
            eating: 0,
            breaking: None,
            break_progress: 0.0,
            break_cooldown: 0,
            use_cooldown: 0,
            walk_dist: 0.0,
            prev_walk_dist: 0.0,
            bob: 0.0,
            prev_bob: 0.0,
            spawn: pos,
            fire: 0,
            equip: 1.0,
            prev_equip: 1.0,
            last_held: ItemStack::EMPTY,
            sprint_tap: 0,
            limb_swing: 0.0,
            limb_amount: 0.0,
            body_yaw: 0.0,
            bow_charge: 0,
            sleeping: 0,
            on_ladder: false,
        }
    }
    pub fn eye_height(&self) -> f32 {
        if self.sleeping > 0 {
            0.3
        } else if self.sneaking {
            1.54
        } else {
            1.62
        }
    }
    pub fn eye(&self) -> Vec3 {
        self.body.pos + v3(0.0, self.eye_height(), 0.0)
    }
    pub fn look_dir(&self) -> Vec3 {
        let (y, p) = (self.yaw.to_radians(), self.pitch.to_radians());
        v3(-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos())
    }
    pub fn add_exhaustion(&mut self, e: f32) {
        self.exhaustion = (self.exhaustion + e).min(40.0);
    }
    pub fn start_swing(&mut self) {
        if !self.swinging || self.swing >= 0.5 || self.swing < 0.0 {
            self.swing = -1.0 / 6.0;
            self.swinging = true;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    None,
    Block(Hit),
    Mob(usize),
}

pub struct Game {
    pub menu: Menu,
    pub world: World,
    pub player: Player,
    pub mobs: Vec<Mob>,
    pub items: Vec<ItemEntity>,
    pub falling: Vec<FallingBlock>,
    pub arrows: Vec<Arrow>,
    pub particles: Vec<Particle>,
    pub rng: Random,
    pub tick_count: u64,
    pub target: Target,
    pub options: Options,
    pub craft3: [ItemStack; 9],
    pub open_pos: (i32, i32, i32),
    pub show_debug: bool,
    pub hide_hud: bool,
    pub third_person: u8,
    pub fluid_queue: VecDeque<(i32, i32, i32, u64)>,
    fluid_set: HashSet<(i32, i32, i32)>,
    pub spawned_chunks: HashSet<(i32, i32)>,
    pub in_world: bool,
    pub world_name: String,
    pub seed_text: String,
    pub name_text: String,
    pub focused_field: u8,
    pub panorama_angle: f32,
    pub held_name_timer: i32,
    pub loading_ticks: i32,
    pub sounds: Vec<SoundEvent>,
    pub splash: String,
    pub dragging_slider: Option<u8>,
    pub menu_return: Menu,
    pub screen_shake: f32,
    pub world_folder: String,
    pub world_list: Vec<crate::save::WorldInfo>,
    pub selected_world: Option<usize>,
    pub settle_on_load: bool,
    pub confirm_delete: bool,
    pub was_in_water: bool,
    pub scroll_acc: f32,
    pub raining: bool,
    pub rain: f32,
    pub prev_rain: f32,
    pub weather_timer: i32,
    pub action_msg: Option<(String, i32)>,
}

pub const TITLE_SEED: u64 = 59;

pub fn title_seed() -> u64 {
    std::env::var("MCRUST_TITLE_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(TITLE_SEED)
}

#[derive(Clone, Copy, Debug)]
pub struct SoundEvent {
    pub name: &'static str,
    pub pos: Option<Vec3>,
    pub volume: f32,
    pub pitch: f32,
}

const SPLASHES: &[&str] = &[
    "Made in Rust!",
    "Zero dependencies!",
    "100% pixels!",
    "Also try Terraria!",
    "Now with biomes!",
    "Punch wood!",
    "Creeper? Aww man!",
    "Blocks all the way down!",
    "Hand-made pixels!",
    "Fearless concurrency!",
    "Borrow checked!",
    "cargo run --release!",
    "Look out for creepers!",
    "Sunrise and sunset!",
    "Works on my machine!",
];

impl Game {
    pub fn new() -> Game {
        let world = World::new(title_seed());
        let mut rng = Random::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1));
        let splash = SPLASHES[rng.range(SPLASHES.len() as i32) as usize].to_string();
        let (sx, sy, sz) = crate::worldgen::find_spawn(&world.generator);
        let p = Player::new(v3(sx as f32 + 0.5, sy as f32 + 10.0, sz as f32 + 0.5));
        Game {
            menu: Menu::Title,
            world,
            player: p,
            mobs: Vec::new(),
            items: Vec::new(),
            falling: Vec::new(),
            arrows: Vec::new(),
            particles: Vec::new(),
            rng,
            tick_count: 0,
            target: Target::None,
            options: Options::load(),
            craft3: [ItemStack::EMPTY; 9],
            open_pos: (0, 0, 0),
            show_debug: false,
            hide_hud: false,
            third_person: 0,
            fluid_queue: VecDeque::new(),
            fluid_set: HashSet::new(),
            spawned_chunks: HashSet::new(),
            in_world: false,
            world_name: "New World".into(),
            seed_text: String::new(),
            name_text: "New World".into(),
            focused_field: 0,
            panorama_angle: 0.0,
            held_name_timer: 0,
            loading_ticks: 0,
            sounds: Vec::new(),
            splash,
            dragging_slider: None,
            menu_return: Menu::Title,
            screen_shake: 0.0,
            world_folder: String::new(),
            world_list: Vec::new(),
            selected_world: None,
            settle_on_load: true,
            confirm_delete: false,
            was_in_water: false,
            scroll_acc: 0.0,
            raining: false,
            rain: 0.0,
            prev_rain: 0.0,
            weather_timer: 12000,
            action_msg: None,
        }
    }

    /// Horizontal direction the player is looking: 0 N(-z), 1 S(+z), 2 W(-x), 3 E(+x).
    pub fn player_facing(&self) -> u8 {
        let yaw = crate::math::wrap_deg(self.player.yaw);
        if (-45.0..45.0).contains(&yaw) {
            1
        } else if (45.0..135.0).contains(&yaw) {
            2
        } else if (-135.0..-45.0).contains(&yaw) {
            3
        } else {
            0
        }
    }

    pub fn message(&mut self, m: &str) {
        self.action_msg = Some((m.to_string(), 60));
    }

    pub fn sound(&mut self, name: &'static str, pos: Option<Vec3>, volume: f32) {
        let pitch = 0.9 + self.rng.next_f32() * 0.2;
        self.sounds.push(SoundEvent { name, pos, volume, pitch });
    }
    pub fn sound_at_block(&mut self, name: &'static str, x: i32, y: i32, z: i32, volume: f32) {
        self.sound(name, Some(v3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5)), volume);
    }

    // ---------------- world lifecycle ----------------

    pub fn start_new_world(&mut self) {
        let seed: u64 = if self.seed_text.trim().is_empty() {
            self.rng.next_u64()
        } else if let Ok(n) = self.seed_text.trim().parse::<i64>() {
            n as u64
        } else {
            // string seeds hash like Java's String.hashCode
            let mut h: i32 = 0;
            for c in self.seed_text.trim().chars() {
                h = h.wrapping_mul(31).wrapping_add(c as i32);
            }
            h as i64 as u64
        };
        self.reset_world(World::new(seed));
        self.world_name = if self.name_text.trim().is_empty() { "New World".into() } else { self.name_text.trim().to_string() };
        self.world_folder = crate::save::folder_name(&self.world_name);
        let (sx, sy, sz) = crate::worldgen::find_spawn(&self.world.generator);
        self.player = Player::new(v3(sx as f32 + 0.5, sy as f32 + 0.5, sz as f32 + 0.5));
        self.player.yaw = self.rng.uniform(0.0, 360.0);
        self.settle_on_load = true;
        self.in_world = true;
        self.menu = Menu::Loading;
        self.loading_ticks = 0;
    }

    /// Replace the world and clear all transient state.
    pub fn reset_world(&mut self, w: World) {
        self.world = w;
        self.mobs.clear();
        self.items.clear();
        self.falling.clear();
        self.arrows.clear();
        self.particles.clear();
        self.fluid_queue.clear();
        self.fluid_set.clear();
        self.spawned_chunks.clear();
        self.craft3 = [ItemStack::EMPTY; 9];
        self.target = Target::None;
        self.raining = false;
        self.rain = 0.0;
        self.prev_rain = 0.0;
        self.weather_timer = 12000;
    }

    pub fn open_saved_world(&mut self, folder: &str) -> bool {
        if crate::save::load_game(self, folder) {
            self.in_world = true;
            self.settle_on_load = false;
            self.menu = Menu::Loading;
            self.loading_ticks = 0;
            true
        } else {
            false
        }
    }

    pub fn save(&self) {
        if self.in_world {
            if let Err(e) = crate::save::save_game(self) {
                eprintln!("failed to save world: {e}");
            }
        }
    }

    pub fn quit_to_title(&mut self) {
        self.close_container_silent();
        self.save();
        self.in_world = false;
        let seed = title_seed();
        self.reset_world(World::new(seed));
        self.menu = Menu::Title;
        let (sx, sy, sz) = crate::worldgen::find_spawn(&self.world.generator);
        self.player = Player::new(v3(sx as f32 + 0.5, sy as f32 + 10.0, sz as f32 + 0.5));
    }

    /// Load / unload chunks around a centre.
    pub fn stream_chunks(&mut self, center: Vec3) -> Vec<(i32, i32)> {
        let rd = self.options.render_distance;
        let pcx = (center.x.floor() as i32) >> 4;
        let pcz = (center.z.floor() as i32) >> 4;
        // request nearest first
        let mut want: Vec<(i32, i32, i32)> = Vec::new();
        for dz in -rd - 1..=rd + 1 {
            for dx in -rd - 1..=rd + 1 {
                if dx * dx + dz * dz <= (rd + 1) * (rd + 1) + 1 {
                    want.push((dx * dx + dz * dz, pcx + dx, pcz + dz));
                }
            }
        }
        want.sort();
        for (_, x, z) in want {
            if self.world.pending_count() > 48 {
                break;
            }
            self.world.request(x, z);
        }
        let lim = rd + 2;
        self.world.retain_pending(|x, z| (x - pcx).abs() <= lim && (z - pcz).abs() <= lim);
        let mut unload = Vec::new();
        for &(x, z) in self.world.chunks.keys() {
            if (x - pcx).abs() > lim + 1 || (z - pcz).abs() > lim + 1 {
                unload.push((x, z));
            }
        }
        for &(x, z) in &unload {
            self.world.unload_chunk(x, z);
        }
        self.world.receive_chunks(8);
        unload
    }

    // ---------------- input ----------------

    pub fn handle_game_input(&mut self, win: &mut Window) {
        let events = win.events.clone();
        for e in events {
            match e {
                Event::KeyDown(k, rep) => {
                    if rep && k != key::Q {
                        continue;
                    }
                    match k {
                        key::ESCAPE => {
                            self.menu = Menu::Pause;
                        }
                        key::E => self.open_inventory(),
                        key::Q => {
                            if !rep || self.tick_count % 2 == 0 {
                                let all = win.key(key::COMMAND) || win.key(key::CONTROL);
                                self.drop_held(all);
                            }
                        }
                        key::F1 => self.hide_hud = !self.hide_hud,
                        key::F3 => self.show_debug = !self.show_debug,
                        key::F5 => self.third_person = (self.third_person + 1) % 3,
                        key::W => {
                            if self.player.sprint_tap > 0 && self.player.food > 6 {
                                self.player.sprinting = true;
                            }
                            self.player.sprint_tap = 7;
                        }
                        _ => {}
                    }
                    let slots = [key::N1, key::N2, key::N3, key::N4, key::N5, key::N6, key::N7, key::N8, key::N9];
                    if let Some(i) = slots.iter().position(|&s| s == k) {
                        self.select_slot(i);
                    }
                }
                Event::Scroll(d) => {
                    // accumulate (trackpads send many small deltas)
                    if d.signum() != self.scroll_acc.signum() {
                        self.scroll_acc = 0.0;
                    }
                    self.scroll_acc += d;
                    while self.scroll_acc.abs() >= 1.0 {
                        let s = self.player.inv.selected as i32;
                        let n = if self.scroll_acc > 0.0 { s - 1 } else { s + 1 };
                        self.select_slot(n.rem_euclid(9) as usize);
                        self.scroll_acc -= self.scroll_acc.signum();
                    }
                }
                Event::MouseDown(0) => {
                    self.player.start_swing();
                    if let Target::Mob(i) = self.target {
                        self.attack_mob(i);
                    }
                }
                Event::MouseDown(1) => {
                    self.player.use_cooldown = 0;
                }
                _ => {}
            }
        }
    }

    fn select_slot(&mut self, i: usize) {
        if self.player.inv.selected != i {
            self.player.inv.selected = i;
            self.held_name_timer = 40;
            self.player.eating = 0;
            self.player.breaking = None;
            self.player.break_progress = 0.0;
        }
    }

    pub fn open_inventory(&mut self) {
        self.menu = Menu::Inventory;
    }

    pub fn close_container(&mut self) {
        if self.menu == Menu::Chest {
            let (x, y, z) = self.open_pos;
            self.sound_at_block("chest.close", x, y, z, 0.6);
        }
        // return crafting grid and cursor contents
        let mut ret: Vec<ItemStack> = Vec::new();
        for s in self.player.craft2.iter_mut() {
            if !s.is_empty() {
                ret.push(*s);
            }
            *s = ItemStack::EMPTY;
        }
        for s in self.craft3.iter_mut() {
            if !s.is_empty() {
                ret.push(*s);
            }
            *s = ItemStack::EMPTY;
        }
        if !self.player.cursor.is_empty() {
            ret.push(self.player.cursor);
            self.player.cursor = ItemStack::EMPTY;
        }
        for st in ret {
            let left = self.player.inv.add(st);
            if left > 0 {
                let mut s = st;
                s.count = left;
                self.throw_item(s);
            }
        }
        self.menu = Menu::None;
    }

    fn drop_held(&mut self, all: bool) {
        let held = self.player.inv.held();
        if held.is_empty() {
            return;
        }
        let n = if all { held.count } else { 1 };
        let mut st = held;
        st.count = n;
        self.player.inv.consume_held(n);
        self.throw_item(st);
        self.player.start_swing();
    }

    pub fn throw_item(&mut self, st: ItemStack) {
        let eye = self.player.eye() - v3(0.0, 0.3, 0.0);
        let mut e = ItemEntity::new(eye, st, &mut self.rng);
        let d = self.player.look_dir();
        e.body.vel = v3(d.x * 0.3, d.y * 0.3 + 0.1, d.z * 0.3);
        e.pickup_delay = 40;
        self.items.push(e);
    }

    pub fn spawn_item(&mut self, pos: Vec3, st: ItemStack) {
        let e = ItemEntity::new(pos, st, &mut self.rng);
        self.items.push(e);
    }

    // ---------------- targeting ----------------

    pub fn update_target(&mut self) {
        let eye = self.player.eye();
        let dir = self.player.look_dir();
        let hit = physics::raycast(&self.world, eye, dir, REACH);
        let block_dist = hit.map(|h| h.dist).unwrap_or(REACH);
        let mut best: Option<(f32, usize)> = None;
        for (i, m) in self.mobs.iter().enumerate() {
            if !m.alive() {
                continue;
            }
            if let Some((t, _)) = m.aabb().grow(0.1).ray(eye, dir) {
                if t <= 3.0 && t < block_dist && best.is_none_or(|b| t < b.0) {
                    best = Some((t, i));
                }
            }
        }
        self.target = if let Some((_, i)) = best {
            Target::Mob(i)
        } else if let Some(h) = hit {
            Target::Block(h)
        } else {
            Target::None
        };
    }

    fn attack_mob(&mut self, i: usize) {
        let held = self.player.inv.held();
        let dmg = item::tool_info(held.id).map(|t| t.attack_damage()).unwrap_or(1.0);
        // critical hit when falling
        let crit = self.player.body.vel.y < -0.05 && !self.player.body.on_ground && !self.player.body.in_water;
        let dmg = if crit { dmg * 1.5 } else { dmg };
        let from = self.player.body.pos;
        let sprinting = self.player.sprinting;
        let m = &mut self.mobs[i];
        let mpos = m.body.pos;
        let kind = m.kind;
        if m.hurt(dmg, from, &mut self.rng) {
            let pitch = if m.alive() { 0.9 + self.rng.next_f32() * 0.2 } else { 0.8 };
            self.sounds.push(SoundEvent { name: mob_hurt_sound(kind), pos: Some(mpos), volume: 1.0, pitch });
            let m = &mut self.mobs[i];
            if sprinting {
                let d = self.player.look_dir();
                m.body.vel.x += d.x * 0.5;
                m.body.vel.z += d.z * 0.5;
                self.player.sprinting = false;
            }
            if crit {
                let c = m.body.pos + v3(0.0, m.body.height * 0.6, 0.0);
                for _ in 0..12 {
                    let v = v3(self.rng.uniform(-0.3, 0.3), self.rng.uniform(0.0, 0.3), self.rng.uniform(-0.3, 0.3));
                    self.particles.push(Particle { pos: c, prev_pos: c, vel: v, age: 0, life: 15, size: 0.08, uv: [0.0; 4], color: [255, 255, 255, 255], gravity: 0.02, textured: false, emissive: true });
                }
            }
            self.player.add_exhaustion(0.1);
            if let Some(t) = item::tool_info(held.id) {
                let wear = if t.kind == Tool::Sword { 1 } else { 2 };
                self.player.inv.damage_held(wear);
            }
        }
    }

    // ---------------- main tick ----------------

    pub fn tick(&mut self, win: &Window) {
        self.tick_count += 1;
        if !self.in_world {
            return;
        }
        self.world.time += 1;
        if self.tick_count % 6000 == 0 {
            self.save();
        }
        if self.held_name_timer > 0 {
            self.held_name_timer -= 1;
        }
        if let Some((_, t)) = &mut self.action_msg {
            *t -= 1;
            if *t <= 0 {
                self.action_msg = None;
            }
        }
        if self.player.sleeping > 0 {
            self.player.sleeping += 1;
            if self.player.sleeping >= 100 {
                self.player.sleeping = 0;
                // wake up beside the bed
                let (bx, by, bz) = self.player.body.pos.floor();
                'find: for r in 1..=2 {
                    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                        let (x, z) = (bx + dx * r, bz + dz * r);
                        if !block::is_solid(self.world.get(x, by, z)) && !block::is_solid(self.world.get(x, by + 1, z)) && block::is_solid(self.world.get(x, by - 1, z)) {
                            self.player.body.pos = v3(x as f32 + 0.5, by as f32, z as f32 + 0.5);
                            self.player.prev_pos = self.player.body.pos;
                            break 'find;
                        }
                    }
                }
                let day = self.world.time.div_euclid(24000) + 1;
                self.world.time = day * 24000;
                self.raining = false;
                self.rain = 0.0;
                self.prev_rain = 0.0;
                self.weather_timer = 6000 + self.rng.range(30000);
                self.save();
            }
        }
        if self.screen_shake > 0.0 {
            self.screen_shake = (self.screen_shake - 0.1).max(0.0);
        }
        let playing = self.menu == Menu::None;
        self.tick_player(win, playing);
        self.tick_mobs();
        self.tick_items();
        self.tick_falling();
        self.tick_arrows();
        self.particles.retain_mut(|p| p.tick(&self.world));
        self.tick_fluids();
        self.tick_weather();
        self.random_ticks();
        self.tick_furnaces();
        self.spawn_mobs();
        self.display_ticks();
    }

    fn tick_furnaces(&mut self) {
        let mut changes = Vec::new();
        for (pos, be) in self.world.block_entities.iter_mut() {
            if let BlockEntity::Furnace(f) = be {
                let burning = f.tick();
                changes.push((*pos, burning));
            }
        }
        for ((x, y, z), burning) in changes {
            let b = self.world.get(x, y, z);
            let meta = self.world.get_meta(x, y, z);
            if burning && b == FURNACE {
                self.world.set_block(x, y, z, FURNACE_LIT, meta);
            } else if !burning && b == FURNACE_LIT {
                self.world.set_block(x, y, z, FURNACE, meta);
            }
            if burning && self.rng.chance(0.1) {
                let p = v3(x as f32 + 0.5, y as f32 + 0.4, z as f32 + 0.5);
                self.flame_particle(p);
            }
        }
    }

    /// Random "display ticks" near the player: torch flames & smoke, lava pops.
    fn display_ticks(&mut self) {
        let (px, py, pz) = self.player.body.pos.floor();
        for _ in 0..800 {
            let x = px + self.rng.range(33) - 16;
            let y = py + self.rng.range(33) - 16;
            let z = pz + self.rng.range(33) - 16;
            let b = self.world.get(x, y, z);
            if b == TORCH {
                let meta = self.world.get_meta(x, y, z);
                let (base, lean) = crate::mesher::torch_geometry(meta);
                let p = v3(x as f32 + base[0] + lean[0], y as f32 + base[1] + 0.68, z as f32 + base[2] + lean[2]);
                self.flame_particle(p);
                self.particles.push(Particle { pos: p, prev_pos: p, vel: v3(0.0, 0.01, 0.0), age: 0, life: 20 + self.rng.range(10), size: 0.04, uv: [0.0; 4], color: [60, 60, 60, 255], gravity: -0.001, textured: false, emissive: false });
            } else if b == LAVA && self.world.get(x, y + 1, z) == AIR && self.rng.chance(0.05) {
                let p = v3(x as f32 + self.rng.next_f32(), y as f32 + 1.0, z as f32 + self.rng.next_f32());
                let v = v3(self.rng.uniform(-0.05, 0.05), 0.15, self.rng.uniform(-0.05, 0.05));
                self.particles.push(Particle { pos: p, prev_pos: p, vel: v, age: 0, life: 30, size: 0.05, uv: [0.0; 4], color: [255, 150, 40, 255], gravity: 0.01, textured: false, emissive: true });
            }
        }
    }

    fn flame_particle(&mut self, p: Vec3) {
        self.particles.push(Particle { pos: p, prev_pos: p, vel: v3(0.0, 0.004, 0.0), age: 0, life: 20 + self.rng.range(20), size: 0.06, uv: [0.0; 4], color: [255, 180, 60, 255], gravity: -0.0005, textured: false, emissive: true });
    }

    pub fn skydark(&self) -> f32 {
        (1.0 - self.sky_brightness()) * 11.0
    }

    /// Celestial angle in [0,1), 0 = noon (vanilla formula).
    pub fn celestial(&self) -> f32 {
        let t = (self.world.time % 24000) as f32 / 24000.0 - 0.25;
        let f = t.rem_euclid(1.0);
        let f2 = 0.5 - (f * std::f32::consts::PI).cos() / 2.0;
        f + (f2 - f) / 3.0
    }

    pub fn sky_brightness(&self) -> f32 {
        let a = self.celestial();
        ((a * std::f32::consts::TAU).cos() * 2.0 + 0.5).clamp(0.0, 1.0) * (1.0 - self.rain * 5.0 / 16.0)
    }

    fn tick_weather(&mut self) {
        self.weather_timer -= 1;
        if self.weather_timer <= 0 {
            self.raining = !self.raining;
            self.weather_timer = if self.raining { 3000 + self.rng.range(9000) } else { 6000 + self.rng.range(30000) };
        }
        let target = if self.raining { 1.0 } else { 0.0 };
        self.prev_rain = self.rain;
        if self.rain < target {
            self.rain = (self.rain + 0.01).min(1.0);
        } else if self.rain > target {
            self.rain = (self.rain - 0.01).max(0.0);
        }
        // rain sound when the player is near open sky in a wet biome
        if self.rain > 0.1 && self.tick_count % 24 == 0 {
            let (x, y, z) = self.player.eye().floor();
            let biome = self.world.biome(x, z);
            let dry = matches!(biome, Biome::Desert | Biome::Savanna);
            if !dry && !biome.is_snowy() {
                let exposed = self.world.surface_height(x, z) <= y + 8;
                let vol = self.rain * if exposed { 0.35 } else { 0.08 };
                self.sounds.push(SoundEvent { name: "rain", pos: None, volume: vol, pitch: 0.9 + self.rng.next_f32() * 0.2 });
            }
        }
    }

    fn tick_player(&mut self, win: &Window, playing: bool) {
        let p = &mut self.player;
        p.prev_pos = p.body.pos;
        p.prev_health = p.health;
        p.prev_walk_dist = p.walk_dist;
        p.prev_bob = p.bob;
        p.prev_swing = p.swing;
        p.prev_equip = p.equip;
        if p.sprint_tap > 0 {
            p.sprint_tap -= 1;
        }
        if p.hurt_time > 0 {
            p.hurt_time -= 1;
        }
        if p.invuln > 0 {
            p.invuln -= 1;
        }
        if p.use_cooldown > 0 {
            p.use_cooldown -= 1;
        }
        if p.break_cooldown > 0 {
            p.break_cooldown -= 1;
        }
        if p.swinging {
            p.swing += 1.0 / 6.0;
            if p.swing >= 1.0 {
                p.swing = 0.0;
                p.swinging = false;
            }
        }
        // Equip animation (lower then raise when held item changes)
        let held = p.inv.held();
        if held.id != p.last_held.id {
            p.equip = (p.equip - 0.4).max(0.0);
            if p.equip <= 0.0 {
                p.last_held = held;
            }
        } else {
            p.last_held = held;
            p.equip = (p.equip + 0.4).min(1.0);
        }

        if p.dead {
            p.death_time += 1;
            return;
        }
        if p.sleeping > 0 {
            return;
        }

        // ---- movement input ----
        let (mut fwd, mut strafe, mut jump) = (0.0f32, 0.0f32, false);
        let mut sneak = false;
        let mut sprint_key = false;
        if playing {
            if win.key(key::W) {
                fwd += 1.0;
            }
            if win.key(key::S) {
                fwd -= 1.0;
            }
            if win.key(key::A) {
                strafe += 1.0;
            }
            if win.key(key::D) {
                strafe -= 1.0;
            }
            jump = win.key(key::SPACE);
            sneak = win.key(key::SHIFT);
            sprint_key = win.key(key::CONTROL) || win.key(key::OPTION);
        }
        if p.eating > 0 {
            fwd *= 0.2;
            strafe *= 0.2;
        }
        p.sneaking = sneak;
        if sneak {
            fwd *= 0.3;
            strafe *= 0.3;
        }
        if sprint_key && fwd > 0.0 && p.food > 6 && !sneak {
            p.sprinting = true;
        }
        if fwd <= 0.0 || p.body.collided_h || p.food <= 6 || sneak {
            p.sprinting = false;
        }
        let len = (fwd * fwd + strafe * strafe).sqrt();
        if len > 1.0 {
            fwd /= len;
            strafe /= len;
        }
        fwd *= 0.98;
        strafe *= 0.98;

        let yaw_r = p.yaw.to_radians();
        let (s, c) = yaw_r.sin_cos();
        let world = &self.world;
        // ladders
        p.on_ladder = {
            let bb = p.body.aabb();
            let (x0, y0, z0) = bb.min.floor();
            let (x1, y1, z1) = bb.max.floor();
            let mut on = false;
            for y in y0..=y1 {
                for z in z0..=z1 {
                    for x in x0..=x1 {
                        if world.get(x, y, z) == LADDER {
                            on = true;
                        }
                    }
                }
            }
            on
        };
        let was_on_ground = p.body.on_ground;
        let b = &mut p.body;
        let move_relative = |b: &mut Body, accel: f32| {
            let mx = strafe * c - fwd * s;
            let mz = fwd * c + strafe * s;
            b.vel.x += mx * accel;
            b.vel.z += mz * accel;
        };
        let landed;
        if b.in_water || b.in_lava {
            let y0 = b.pos.y;
            move_relative(b, 0.02);
            let v = b.vel;
            landed = b.move_by(world, v, false);
            let drag = if b.in_water { 0.8 } else { 0.5 };
            b.vel.x *= drag;
            b.vel.z *= drag;
            b.vel.y = b.vel.y * drag - 0.02;
            if jump {
                b.vel.y += 0.04;
            }
            // jump out of water onto a ledge
            if b.collided_h {
                let test = b.aabb().offset(v3(b.vel.x, b.vel.y + 0.6 - b.pos.y + y0, b.vel.z));
                if physics::block_boxes(world, &test).iter().all(|o| !o.intersects(&test)) {
                    b.vel.y = 0.3;
                }
            }
        } else {
            let slip = if b.on_ground {
                let (x, y, z) = (b.pos - v3(0.0, 0.5, 0.0)).floor();
                let below = world.get(x, y, z);
                if below == ICE { 0.98 } else { 0.6 }
            } else {
                1.0
            };
            let friction = slip * 0.91;
            let speed = 0.1 * if p.sprinting { 1.3 } else { 1.0 };
            let accel = if b.on_ground { speed * (0.16277136 / (friction * friction * friction)) } else if p.sprinting { 0.026 } else { 0.02 };
            if jump && b.on_ground && p.use_cooldown >= 0 {
                b.vel.y = 0.42;
                if p.sprinting {
                    b.vel.x -= s * 0.2;
                    b.vel.z += c * 0.2;
                }
            }
            move_relative(b, accel);
            if p.on_ladder {
                b.vel.x = b.vel.x.clamp(-0.15, 0.15);
                b.vel.z = b.vel.z.clamp(-0.15, 0.15);
                b.vel.y = b.vel.y.max(-0.15);
                b.fall_distance = 0.0;
                if sneak && b.vel.y < 0.0 {
                    b.vel.y = 0.0;
                }
            }
            let v = b.vel;
            landed = b.move_by(world, v, sneak);
            if p.on_ladder && (b.collided_h || jump) {
                b.vel.y = 0.2;
            }
            b.vel.y = (b.vel.y - 0.08) * 0.98;
            b.vel.x *= friction;
            b.vel.z *= friction;
        }
        if jump && was_on_ground && !b.on_ground && b.vel.y > 0.0 {
            p.exhaustion += if p.sprinting { 0.2 } else { 0.05 };
        }
        // Walk distance & bobbing
        let dx = p.body.pos.x - p.prev_pos.x;
        let dz = p.body.pos.z - p.prev_pos.z;
        let dist = (dx * dx + dz * dz).sqrt();
        let prev_step = p.walk_dist.floor();
        p.walk_dist += dist * 0.6;
        let stepped = p.walk_dist.floor() > prev_step && p.body.on_ground && !p.sneaking;
        let target_bob = if p.body.on_ground { dist.min(0.1) } else { 0.0 };
        p.bob += (target_bob - p.bob) * 0.4;
        p.limb_amount += ((dist * 4.0).min(1.0) - p.limb_amount) * 0.4;
        p.limb_swing += p.limb_amount;
        if p.sprinting {
            p.add_exhaustion(0.1 * dist);
        }
        // body yaw follows movement / head
        let diff = crate::math::wrap_deg(p.yaw - p.body_yaw);
        if dist > 0.01 || diff.abs() > 50.0 {
            p.body_yaw += diff * 0.3;
        }

        // ---- footstep / landing / splash sounds ----
        let feet = self.player.body.pos;
        let (fx, fy, fz) = (feet - v3(0.0, 0.2, 0.0)).floor();
        let below = self.world.get(fx, fy, fz);
        if stepped && below != AIR && below != WATER {
            self.sound(crate::audio::step_sound(below), Some(feet), 0.3);
        }
        if let Some(fd) = landed {
            if fd > 1.0 && below != AIR {
                self.sound(crate::audio::step_sound(below), Some(feet), 0.5);
            }
        }
        let in_water = self.player.body.in_water;
        if in_water && !self.was_in_water && self.player.body.vel.y < -0.1 {
            self.sound("splash", Some(feet), 0.4);
        }
        self.was_in_water = in_water;
        // ---- fall damage ----
        if let Some(fd) = landed {
            let dmg = (fd - 3.0).ceil();
            if dmg > 0.0 {
                self.damage_player(dmg, None);
            }
        }
        let p = &mut self.player;
        // ---- drowning ----
        let (ex, ey, ez) = p.eye().floor();
        let head_block = self.world.get(ex, ey, ez);
        if head_block == WATER {
            p.air -= 1;
            if p.air <= -20 {
                p.air = 0;
                self.damage_player(2.0, None);
            }
        } else {
            p.air = (p.air + 5).min(300);
        }
        let p = &mut self.player;
        // ---- lava / fire / cactus ----
        if p.body.in_lava {
            p.fire = 300;
            self.damage_player(4.0, None);
        }
        let p = &mut self.player;
        if p.body.in_water {
            p.fire = 0;
        }
        if p.fire > 0 {
            p.fire -= 1;
            if p.fire % 20 == 0 {
                self.damage_player(1.0, None);
            }
        }
        let bb = self.player.body.aabb().grow(0.01);
        let touching_cactus = {
            let (x0, y0, z0) = bb.min.floor();
            let (x1, y1, z1) = bb.max.floor();
            let mut t = false;
            for y in y0..=y1 {
                for z in z0..=z1 {
                    for x in x0..=x1 {
                        if self.world.get(x, y, z) == CACTUS {
                            t = true;
                        }
                    }
                }
            }
            t
        };
        if touching_cactus {
            self.damage_player(1.0, None);
        }
        if self.player.body.pos.y < -64.0 {
            self.damage_player(4.0, None);
        }

        // ---- hunger ----
        self.tick_hunger();

        // ---- item pickup ----
        let pbb = self.player.body.aabb().grow(1.0);
        let mut picked = false;
        for it in self.items.iter_mut() {
            if it.pickup_delay > 0 || it.stack.count == 0 {
                continue;
            }
            if pbb.intersects(&it.body.aabb()) {
                let left = self.player.inv.add(it.stack);
                if left < it.stack.count {
                    picked = true;
                }
                it.stack.count = left;
            }
        }
        if picked {
            let pitch = 1.4 + self.rng.next_f32() * 0.8;
            let pos = Some(self.player.body.pos);
            self.sounds.push(SoundEvent { name: "pop", pos, volume: 0.25, pitch });
        }
        self.items.retain(|i| i.stack.count > 0);

        if playing {
            self.update_target();
            self.tick_mining(win);
            self.tick_use(win);
        } else {
            self.player.breaking = None;
            self.player.break_progress = 0.0;
            self.player.eating = 0;
        }
    }

    fn tick_hunger(&mut self) {
        let p = &mut self.player;
        let diff = self.options.difficulty;
        if diff == Difficulty::Peaceful {
            p.food = 20;
            if self.tick_count % 20 == 0 && p.health < 20.0 {
                p.health = (p.health + 1.0).min(20.0);
            }
            return;
        }
        if p.exhaustion > 4.0 {
            p.exhaustion -= 4.0;
            if p.saturation > 0.0 {
                p.saturation = (p.saturation - 1.0).max(0.0);
            } else {
                p.food = (p.food - 1).max(0);
            }
        }
        if p.food >= 18 && p.health < 20.0 {
            p.food_timer += 1;
            if p.food_timer >= 80 {
                p.health = (p.health + 1.0).min(20.0);
                p.add_exhaustion(6.0);
                p.food_timer = 0;
            }
        } else if p.food <= 0 {
            p.food_timer += 1;
            if p.food_timer >= 80 {
                p.food_timer = 0;
                let min = match diff {
                    Difficulty::Easy => 10.0,
                    Difficulty::Normal => 1.0,
                    _ => 0.0,
                };
                if p.health > min {
                    self.damage_player(1.0, None);
                }
            }
        } else {
            p.food_timer = 0;
        }
    }

    pub fn damage_player(&mut self, dmg: f32, from: Option<Vec3>) {
        let p = &mut self.player;
        if p.dead || p.invuln > 0 || dmg <= 0.0 {
            return;
        }
        p.health -= dmg;
        p.hurt_time = 10;
        p.invuln = 10;
        p.add_exhaustion(0.1);
        if let Some(f) = from {
            let d = p.body.pos - f;
            let l = (d.x * d.x + d.z * d.z).sqrt().max(0.01);
            p.body.vel.x = p.body.vel.x * 0.5 + d.x / l * 0.4;
            p.body.vel.z = p.body.vel.z * 0.5 + d.z / l * 0.4;
            p.body.vel.y = 0.36;
        }
        self.sounds.push(SoundEvent { name: "hurt", pos: None, volume: 0.8, pitch: 1.0 });
        let p = &mut self.player;
        if p.health <= 0.0 {
            p.health = 0.0;
            p.dead = true;
            p.death_time = 0;
            // drop everything
            let mut drops = Vec::new();
            for s in p.inv.slots.iter_mut() {
                if !s.is_empty() {
                    drops.push(*s);
                    *s = ItemStack::EMPTY;
                }
            }
            let pos = p.body.pos + v3(0.0, 1.0, 0.0);
            for d in drops {
                let mut e = ItemEntity::new(pos, d, &mut self.rng);
                e.body.vel = v3(self.rng.uniform(-0.3, 0.3), self.rng.uniform(0.1, 0.4), self.rng.uniform(-0.3, 0.3));
                e.pickup_delay = 40;
                self.items.push(e);
            }
            self.close_container_silent();
            self.menu = Menu::Death;
        }
    }

    fn close_container_silent(&mut self) {
        self.player.craft2 = [ItemStack::EMPTY; 4];
        self.craft3 = [ItemStack::EMPTY; 9];
        self.player.cursor = ItemStack::EMPTY;
    }

    pub fn respawn(&mut self) {
        let spawn = self.player.spawn;
        let inv = std::mem::replace(&mut self.player.inv, Inventory::new());
        self.player = Player::new(spawn);
        self.player.inv = inv;
        // make sure spawn is not inside blocks
        let (x, _, z) = spawn.floor();
        if self.world.is_loaded(x, z) {
            let mut y = spawn.y.floor() as i32;
            while y < CHUNK_H as i32 - 2 && (block::is_solid(self.world.get(x, y, z)) || block::is_solid(self.world.get(x, y + 1, z))) {
                y += 1;
            }
            self.player.body.pos.y = y as f32;
        }
        self.menu = Menu::None;
    }

    // ---------------- mining ----------------

    fn break_speed(&self, b: u8) -> f32 {
        let d = block::def(b);
        if d.hardness < 0.0 {
            return 0.0;
        }
        if d.hardness == 0.0 {
            return 1.0;
        }
        let held = self.player.inv.held();
        let tool = item::tool_info(held.id);
        let mut speed = 1.0;
        let mut can_harvest = d.tier == 0;
        if held.id == item::SHEARS {
            if block::is_leaves(b) {
                speed = 15.0;
            } else if b == WOOL {
                speed = 5.0;
            }
        }
        if let Some(t) = tool {
            let effective = t.kind == d.tool || (t.kind == Tool::Sword && (block::is_leaves(b) || b == PUMPKIN)) || (t.kind == Tool::Hoe && block::is_leaves(b));
            if effective {
                speed = if t.kind == Tool::Sword { 1.5 } else { t.speed() };
            }
            if t.kind == Tool::Pickaxe && d.tool == Tool::Pickaxe && t.level() >= d.tier {
                can_harvest = true;
            }
        }
        if !self.player.body.on_ground {
            speed /= 5.0;
        }
        let (ex, ey, ez) = self.player.eye().floor();
        if self.world.get(ex, ey, ez) == WATER {
            speed /= 5.0;
        }
        speed / d.hardness / if can_harvest { 30.0 } else { 100.0 }
    }

    pub fn can_harvest(&self, b: u8) -> bool {
        let d = block::def(b);
        if d.tier == 0 {
            return true;
        }
        match item::tool_info(self.player.inv.held().id) {
            Some(t) => t.kind == Tool::Pickaxe && t.level() >= d.tier,
            None => false,
        }
    }

    fn tick_mining(&mut self, win: &Window) {
        let lmb = win.buttons[0];
        let Target::Block(hit) = self.target else {
            self.player.breaking = None;
            self.player.break_progress = 0.0;
            return;
        };
        if !lmb {
            self.player.breaking = None;
            self.player.break_progress = 0.0;
            return;
        }
        let pos = (hit.x, hit.y, hit.z);
        if self.player.breaking != Some(pos) {
            self.player.breaking = Some(pos);
            self.player.break_progress = 0.0;
        }
        if self.player.break_cooldown > 0 {
            return;
        }
        self.player.start_swing();
        let b = self.world.get(hit.x, hit.y, hit.z);
        let speed = self.break_speed(b);
        self.player.break_progress += speed;
        if self.tick_count % 4 == 0 {
            self.block_hit_particles(hit);
            let pos = v3(hit.x as f32 + 0.5, hit.y as f32 + 0.5, hit.z as f32 + 0.5);
            self.sounds.push(SoundEvent { name: crate::audio::step_sound(b), pos: Some(pos), volume: 0.3, pitch: 0.5 });
        }
        if self.player.break_progress >= 1.0 {
            self.player.break_progress = 0.0;
            self.player.breaking = None;
            self.player.break_cooldown = 5;
            self.break_block(hit.x, hit.y, hit.z, true);
            self.player.add_exhaustion(0.005);
            let held = self.player.inv.held();
            if held.id == item::SHEARS && (block::is_leaves(b) || b == WOOL || b == SHORT_GRASS || b == FERN) {
                self.player.inv.damage_held(1);
            }
            if let Some(t) = item::tool_info(held.id) {
                if block::def(b).hardness > 0.0 {
                    self.player.inv.damage_held(if t.kind == Tool::Sword { 2 } else { 1 });
                }
            }
        }
    }

    fn block_hit_particles(&mut self, hit: Hit) {
        let b = self.world.get(hit.x, hit.y, hit.z);
        let (nx, ny, nz) = DIRS[hit.face];
        let c = v3(hit.x as f32 + 0.5 + nx as f32 * 0.6, hit.y as f32 + 0.5 + ny as f32 * 0.6, hit.z as f32 + 0.5 + nz as f32 * 0.6);
        let jitter = v3(if nx == 0 { self.rng.uniform(-0.4, 0.4) } else { 0.0 }, if ny == 0 { self.rng.uniform(-0.4, 0.4) } else { 0.0 }, if nz == 0 { self.rng.uniform(-0.4, 0.4) } else { 0.0 });
        // small chips that pop off the face and fall (vanilla: power 0.2, scale 0.6)
        let v = self.particle_velocity(Vec3::ZERO);
        let v = v3(v.x * 0.2, (v.y - 0.1) * 0.2 + 0.1, v.z * 0.2);
        self.block_particle(b, c + jitter, v, 0.6);
    }

    /// Vanilla particle launch: the base direction plus random spread, at a
    /// random speed, with a little upward kick.
    fn particle_velocity(&mut self, base: Vec3) -> Vec3 {
        let r = &mut self.rng;
        let d = base + v3(r.uniform(-0.4, 0.4), r.uniform(-0.4, 0.4), r.uniform(-0.4, 0.4));
        let speed = (r.next_f32() + r.next_f32() + 1.0) * 0.15 * 0.4;
        let l = d.len().max(1e-4);
        v3(d.x / l * speed, d.y / l * speed + 0.1, d.z / l * speed)
    }

    /// A terrain debris particle: a random 4x4-pixel chip of the block's
    /// texture, darkened to 60% like vanilla's.
    pub fn block_particle(&mut self, b: u8, pos: Vec3, vel: Vec3, scale: f32) {
        // pick a random 4x4 sub-region of the block's side texture; uv filled by renderer via tile index
        let su = self.rng.range(12) as f32 / 16.0;
        let sv = self.rng.range(12) as f32 / 16.0;
        let tint = match block::def(b).tint {
            Tint::Grass | Tint::Foliage => {
                let c = self.world.grass_color(pos.x.floor() as i32, pos.z.floor() as i32);
                if b == GRASS { 0xFFFFFF } else { c }
            }
            Tint::Fixed(c) => c,
            Tint::None => 0xFFFFFF,
        };
        let dark = |c: u32| (c as f32 * 0.6) as u8;
        self.particles.push(Particle {
            pos,
            prev_pos: pos,
            vel,
            age: 0,
            life: (4.0 / (self.rng.next_f32() * 0.9 + 0.1)) as i32,
            size: (0.05 + self.rng.next_f32() * 0.05) * scale,
            // encode: u0 = block id, v0 = sub offsets (resolved at render time)
            uv: [b as f32, su, sv, 0.0],
            color: [dark((tint >> 16) & 255), dark((tint >> 8) & 255), dark(tint & 255), 255],
            gravity: 0.04,
            textured: true,
            emissive: false,
        });
    }

    /// Break a block, spawning drops and particles.
    pub fn break_block(&mut self, x: i32, y: i32, z: i32, by_player: bool) {
        let b = self.world.get(x, y, z);
        if b == AIR || b == BEDROCK {
            return;
        }
        let meta = self.world.get_meta(x, y, z);
        // particles: a 4x4x4 burst of chips flying outward from the centre (as in vanilla)
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    let d = v3((i as f32 + 0.5) / 4.0, (j as f32 + 0.5) / 4.0, (k as f32 + 0.5) / 4.0);
                    let p = v3(x as f32, y as f32, z as f32) + d;
                    let v = self.particle_velocity(d - v3(0.5, 0.5, 0.5));
                    self.block_particle(b, p, v, 1.0);
                }
            }
        }
        self.sound_at_block(crate::audio::dig_sound(b), x, y, z, 1.0);
        // drops
        let harvest = !by_player || self.can_harvest(b);
        if harvest {
            for st in self.block_drops(b, meta) {
                let p = v3(x as f32 + 0.5, y as f32 + 0.3, z as f32 + 0.5);
                self.spawn_item(p, st);
            }
        }
        // block entity contents
        if let Some(be) = self.world.block_entities.remove(&(x, y, z)) {
            let p = v3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
            match be {
                BlockEntity::Furnace(f) => {
                    for s in [f.input, f.fuel, f.output] {
                        if !s.is_empty() {
                            self.spawn_item(p, s);
                        }
                    }
                }
                BlockEntity::Chest(v) => {
                    for s in v {
                        if !s.is_empty() {
                            self.spawn_item(p, s);
                        }
                    }
                }
            }
        }
        let replacement = if b == ICE && by_player && self.world.get(x, y - 1, z) != AIR { WATER } else { AIR };
        self.set_block_updated(x, y, z, replacement, 0);
        // remove the other half of doors / beds
        if b == OAK_DOOR {
            let oy = if meta & 8 != 0 { y - 1 } else { y + 1 };
            if self.world.get(x, oy, z) == OAK_DOOR {
                self.set_block_updated(x, oy, z, AIR, 0);
            }
        } else if b == BED {
            let (dx, dz) = block::facing_step(meta & 3);
            let (ox, oz) = if meta & 4 != 0 { (x - dx, z - dz) } else { (x + dx, z + dz) };
            if self.world.get(ox, y, oz) == BED {
                self.set_block_updated(ox, y, oz, AIR, 0);
            }
        }
    }

    pub fn block_drops(&mut self, b: u8, meta: u8) -> Vec<ItemStack> {
        let one = |id: u16| vec![ItemStack::new(id, 1)];
        let r = &mut self.rng;
        match b {
            STONE => one(COBBLESTONE as u16),
            GRASS | FARMLAND => one(DIRT as u16),
            COAL_ORE => one(item::COAL),
            DIAMOND_ORE => one(item::DIAMOND),
            GRAVEL => {
                if r.chance(0.1) {
                    one(item::FLINT)
                } else {
                    one(GRAVEL as u16)
                }
            }
            SHORT_GRASS | FERN if self.player.inv.held().id == item::SHEARS => one(b as u16),
            SHORT_GRASS | FERN => {
                if r.chance(0.125) {
                    one(item::WHEAT_SEEDS)
                } else {
                    vec![]
                }
            }
            DEAD_BUSH => {
                if r.chance(0.5) {
                    vec![ItemStack::new(item::STICK, 1 + r.range(2) as u8)]
                } else {
                    vec![]
                }
            }
            WHEAT => {
                if meta >= 7 {
                    vec![ItemStack::new(item::WHEAT, 1), ItemStack::new(item::WHEAT_SEEDS, 1 + r.range(3) as u8)]
                } else {
                    one(item::WHEAT_SEEDS)
                }
            }
            OAK_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES | JUNGLE_LEAVES | ACACIA_LEAVES if self.player.inv.held().id == item::SHEARS => one(b as u16),
            OAK_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES | JUNGLE_LEAVES | ACACIA_LEAVES => {
                let mut v = vec![];
                let sap = match b {
                    BIRCH_LEAVES => BIRCH_SAPLING,
                    SPRUCE_LEAVES => SPRUCE_SAPLING,
                    JUNGLE_LEAVES => JUNGLE_SAPLING,
                    ACACIA_LEAVES => ACACIA_SAPLING,
                    _ => OAK_SAPLING,
                };
                if self.rng.chance(if b == JUNGLE_LEAVES { 0.025 } else { 0.05 }) {
                    v.push(ItemStack::new(sap as u16, 1));
                }
                if self.rng.chance(0.02) {
                    v.push(ItemStack::new(item::STICK, 1 + self.rng.range(2) as u8));
                }
                if b == OAK_LEAVES && self.rng.chance(0.005) {
                    v.push(ItemStack::new(item::APPLE, 1));
                }
                v
            }
            GLASS | ICE | AIR | WATER | LAVA | SNOW_LAYER => {
                if b == SNOW_LAYER && item::tool_info(self.player.inv.held().id).is_some_and(|t| t.kind == Tool::Shovel) {
                    one(SNOW_LAYER as u16)
                } else {
                    vec![]
                }
            }
            SNOW_BLOCK => vec![ItemStack::new(SNOW_LAYER as u16, 4)],
            CLAY => one(CLAY as u16),
            BOOKSHELF => vec![ItemStack::new(item::PAPER, 3)],
            FURNACE_LIT => one(FURNACE as u16),
            OAK_DOOR => one(item::OAK_DOOR_ITEM),
            BED => one(item::BED_ITEM),
            BEDROCK => vec![],
            _ => one(b as u16),
        }
    }

    /// Set a block and trigger neighbour updates (falling blocks, fluids, plant support).
    pub fn set_block_updated(&mut self, x: i32, y: i32, z: i32, b: u8, meta: u8) {
        self.world.set_block(x, y, z, b, meta);
        self.neighbor_changed(x, y, z);
        for (dx, dy, dz) in DIRS {
            self.neighbor_changed(x + dx, y + dy, z + dz);
        }
    }

    fn neighbor_changed(&mut self, x: i32, y: i32, z: i32) {
        let b = self.world.get(x, y, z);
        // fluids
        if b == WATER || b == LAVA {
            self.schedule_fluid(x, y, z);
        } else if b == AIR || block::def(b).replaceable {
            for (dx, dy, dz) in DIRS {
                if dy == -1 {
                    continue;
                }
                let nb = self.world.get(x + dx, y + dy, z + dz);
                if nb == WATER || nb == LAVA {
                    self.schedule_fluid(x + dx, y + dy, z + dz);
                }
            }
        }
        // gravity blocks
        if (b == SAND || b == GRAVEL) && y > 0 {
            let below = self.world.get(x, y - 1, z);
            if below == AIR || below == WATER || below == LAVA || block::def(below).replaceable {
                self.world.set_block(x, y, z, AIR, 0);
                self.falling.push(FallingBlock::new(x, y, z, b));
                self.neighbor_changed(x, y + 1, z);
            }
        }
        // plants need support
        if block::needs_support(b) {
            let below = self.world.get(x, y - 1, z);
            let ok = match b {
                WHEAT => below == FARMLAND,
                CACTUS => below == SAND || below == CACTUS,
                SUGAR_CANE => matches!(below, GRASS | DIRT | SAND | SUGAR_CANE),
                DEAD_BUSH => matches!(below, SAND | DIRT | GRASS),
                SNOW_LAYER => block::is_opaque(below) || block::is_leaves(below),
                _ => matches!(below, GRASS | DIRT | FARMLAND),
            };
            if !ok {
                self.break_block(x, y, z, false);
            }
        }
        if b == OAK_DOOR {
            let meta = self.world.get_meta(x, y, z);
            if meta & 8 == 0 && !block::is_opaque(self.world.get(x, y - 1, z)) {
                self.break_block(x, y, z, false);
                return;
            }
        }
        if b == LADDER {
            let meta = self.world.get_meta(x, y, z);
            let (wx, wz) = match meta & 3 {
                0 => (x, z - 1),
                1 => (x, z + 1),
                2 => (x - 1, z),
                _ => (x + 1, z),
            };
            if !block::is_opaque(self.world.get(wx, y, wz)) {
                self.break_block(x, y, z, false);
                return;
            }
        }
        if b == TORCH {
            let meta = self.world.get_meta(x, y, z);
            let (sx, sy, sz) = match meta {
                1 => (x, y, z - 1),
                2 => (x, y, z + 1),
                3 => (x - 1, y, z),
                4 => (x + 1, y, z),
                _ => (x, y - 1, z),
            };
            if !block::is_opaque(self.world.get(sx, sy, sz)) && self.world.get(sx, sy, sz) != FARMLAND {
                self.break_block(x, y, z, false);
            }
        }
        // farmland under a solid block turns to dirt
        if b == FARMLAND && block::is_opaque(self.world.get(x, y + 1, z)) {
            self.world.set_block(x, y, z, DIRT, 0);
        }
        // grass under an opaque block turns to dirt
        if b == GRASS && block::is_opaque(self.world.get(x, y + 1, z)) {
            self.world.set_block(x, y, z, DIRT, 0);
        }
    }

    // ---------------- using items / placing ----------------

    fn tick_use(&mut self, win: &Window) {
        let rmb = win.buttons[1];
        let held = self.player.inv.held();
        // Bow: hold to draw, release to shoot
        if held.id == item::BOW {
            let has_arrow = self.player.inv.slots.iter().any(|s| s.id == item::ARROW && s.count > 0);
            if rmb && has_arrow {
                self.player.bow_charge += 1;
                return;
            }
            if !rmb && self.player.bow_charge > 0 {
                let charge = self.player.bow_charge;
                self.player.bow_charge = 0;
                let mut f = charge as f32 / 20.0;
                f = (f * f + f * 2.0) / 3.0;
                if f < 0.1 {
                    return;
                }
                let f = f.min(1.0);
                if let Some(i) = self.player.inv.slots.iter().position(|s| s.id == item::ARROW && s.count > 0) {
                    self.player.inv.slots[i].count -= 1;
                    if self.player.inv.slots[i].count == 0 {
                        self.player.inv.slots[i] = ItemStack::EMPTY;
                    }
                }
                let d = self.player.look_dir();
                let from = self.player.eye() - v3(0.0, 0.1, 0.0);
                let dmg = if f >= 1.0 { 9.0 } else { 6.0 * f };
                self.arrows.push(Arrow { pos: from, prev_pos: from, vel: d * (f * 3.0), stuck: false, age: 0, from_player: true, damage: dmg.max(1.0) });
                self.player.inv.damage_held(1);
                self.sound("bow", Some(from), 0.8);
                return;
            }
            self.player.bow_charge = 0;
        } else {
            self.player.bow_charge = 0;
        }
        // Eating
        if let Some((hunger, sat)) = item::food(held.id) {
            if rmb && (self.player.food < 20 || self.options.difficulty == Difficulty::Peaceful && false) {
                // interacting with a block takes priority (e.g. furnace)
                let interact = matches!(self.target, Target::Block(h) if matches!(self.world.get(h.x, h.y, h.z), CRAFTING_TABLE | FURNACE | FURNACE_LIT | CHEST)) && !self.player.sneaking;
                if !interact {
                    self.player.eating += 1;
                    if self.player.eating % 4 == 0 {
                        self.sound("eat", None, 0.5);
                        // food particles
                        let p = self.player.eye() + self.player.look_dir() * 0.5 - v3(0.0, 0.2, 0.0);
                        if item::texture(held.id).is_some() {
                            // crumbs: small chips of the food's own sprite (as in vanilla)
                            for _ in 0..3 {
                                let v = v3(self.rng.uniform(-0.05, 0.05), 0.1, self.rng.uniform(-0.05, 0.05));
                                let (su, sv) = (self.rng.range(12) as f32 / 16.0, self.rng.range(12) as f32 / 16.0);
                                let size = 0.04 + self.rng.next_f32() * 0.03;
                                self.particles.push(Particle { pos: p, prev_pos: p, vel: v, age: 0, life: 15, size, uv: [held.id as f32, su, sv, 1.0], color: [255; 4], gravity: 0.04, textured: true, emissive: false });
                            }
                        }
                    }
                    if self.player.eating >= 32 {
                        self.player.eating = 0;
                        self.player.food = (self.player.food + hunger).min(20);
                        self.player.saturation = (self.player.saturation + sat).min(self.player.food as f32);
                        self.player.inv.consume_held(1);
                        if held.id == item::ROTTEN_FLESH && self.rng.chance(0.8) {
                            self.player.saturation = 0.0;
                        }
                        self.sound("burp", None, 0.5);
                    }
                    return;
                }
            } else {
                self.player.eating = 0;
            }
        } else {
            self.player.eating = 0;
        }
        if !rmb || self.player.use_cooldown > 0 {
            return;
        }
        self.player.use_cooldown = 4;
        // Buckets work on fluids, so raycast including fluids
        if held.id == item::BUCKET {
            if let Some((x, y, z)) = self.fluid_raycast() {
                let b = self.world.get(x, y, z);
                if self.world.get_meta(x, y, z) == 0 {
                    self.world.set_block(x, y, z, AIR, 0);
                    self.neighbor_changed(x, y, z);
                    for (dx, dy, dz) in DIRS {
                        self.neighbor_changed(x + dx, y + dy, z + dz);
                    }
                    let filled = if b == WATER { item::WATER_BUCKET } else { item::LAVA_BUCKET };
                    self.sound_at_block(if b == WATER { "bucket.fill" } else { "bucket.fill_lava" }, x, y, z, 1.0);
                    self.player.inv.consume_held(1);
                    let st = ItemStack::new(filled, 1);
                    if self.player.inv.held().is_empty() {
                        *self.player.inv.held_mut() = st;
                    } else if self.player.inv.add(st) > 0 {
                        self.throw_item(st);
                    }
                    self.player.start_swing();
                }
            }
            return;
        }
        if let Target::Mob(i) = self.target {
            if held.id == item::SHEARS && self.mobs[i].kind == MobKind::Sheep && !self.mobs[i].sheared && self.mobs[i].alive() {
                self.mobs[i].sheared = true;
                let pos = self.mobs[i].body.pos + v3(0.0, 1.0, 0.0);
                let n = 1 + self.rng.range(3) as u8;
                for _ in 0..n {
                    self.spawn_item(pos, ItemStack::new(WOOL as u16, 1));
                }
                self.player.inv.damage_held(1);
                self.player.start_swing();
                self.sound("shear", Some(pos), 1.0);
            }
            return;
        }
        let Target::Block(hit) = self.target else { return };
        let tb = self.world.get(hit.x, hit.y, hit.z);
        // Interactions
        if !self.player.sneaking || held.is_empty() {
            match tb {
                CRAFTING_TABLE => {
                    self.menu = Menu::Crafting;
                    self.open_pos = (hit.x, hit.y, hit.z);
                    return;
                }
                FURNACE | FURNACE_LIT => {
                    self.open_pos = (hit.x, hit.y, hit.z);
                    self.world.block_entities.entry(self.open_pos).or_insert_with(|| BlockEntity::Furnace(FurnaceState::new()));
                    self.menu = Menu::Furnace;
                    return;
                }
                OAK_DOOR => {
                    let m = self.world.get_meta(hit.x, hit.y, hit.z);
                    let lower_y = if m & 8 != 0 { hit.y - 1 } else { hit.y };
                    let lm = self.world.get_meta(hit.x, lower_y, hit.z) ^ 4;
                    self.world.set_meta(hit.x, lower_y, hit.z, lm);
                    if self.world.get(hit.x, lower_y + 1, hit.z) == OAK_DOOR {
                        self.world.set_meta(hit.x, lower_y + 1, hit.z, lm | 8);
                    }
                    self.player.start_swing();
                    self.sound_at_block(if lm & 4 != 0 { "door.open" } else { "door.close" }, hit.x, hit.y, hit.z, 0.8);
                    return;
                }
                BED => {
                    self.try_sleep(hit.x, hit.y, hit.z);
                    return;
                }
                CHEST => {
                    self.open_pos = (hit.x, hit.y, hit.z);
                    self.world.block_entities.entry(self.open_pos).or_insert_with(|| BlockEntity::Chest(vec![ItemStack::EMPTY; 27]));
                    self.menu = Menu::Chest;
                    self.sound_at_block("chest.open", hit.x, hit.y, hit.z, 0.6);
                    return;
                }
                TNT => {
                    if held.id == item::FLINT {
                        // (no flint & steel) - ignore
                    }
                }
                _ => {}
            }
        }
        if held.is_empty() {
            return;
        }
        // Hoe: till
        if let Some(t) = item::tool_info(held.id) {
            if t.kind == Tool::Hoe && (tb == GRASS || tb == DIRT) && hit.face != 2 && self.world.get(hit.x, hit.y + 1, hit.z) == AIR {
                self.world.set_block(hit.x, hit.y, hit.z, FARMLAND, 0);
                self.player.inv.damage_held(1);
                self.player.start_swing();
                self.sound_at_block("till", hit.x, hit.y, hit.z, 1.0);
                if tb == GRASS && self.rng.chance(0.1) {
                    let p = v3(hit.x as f32 + 0.5, hit.y as f32 + 1.1, hit.z as f32 + 0.5);
                    self.spawn_item(p, ItemStack::new(item::WHEAT_SEEDS, 1));
                }
                return;
            }
            return;
        }
        // Seeds
        if held.id == item::WHEAT_SEEDS {
            if tb == FARMLAND && hit.face == 3 && self.world.get(hit.x, hit.y + 1, hit.z) == AIR {
                self.world.set_block(hit.x, hit.y + 1, hit.z, WHEAT, 0);
                self.player.inv.consume_held(1);
                self.player.start_swing();
            }
            return;
        }
        // Water / lava bucket
        if held.id == item::WATER_BUCKET || held.id == item::LAVA_BUCKET {
            let (nx, ny, nz) = DIRS[hit.face];
            let (px, py, pz) = if block::def(tb).replaceable { (hit.x, hit.y, hit.z) } else { (hit.x + nx, hit.y + ny, hit.z + nz) };
            let cur = self.world.get(px, py, pz);
            if cur == AIR || block::def(cur).replaceable {
                let fl = if held.id == item::WATER_BUCKET { WATER } else { LAVA };
                self.set_block_updated(px, py, pz, fl, 0);
                self.sound_at_block(if fl == WATER { "bucket.empty" } else { "bucket.empty_lava" }, px, py, pz, 1.0);
                self.schedule_fluid(px, py, pz);
                *self.player.inv.held_mut() = ItemStack::new(item::BUCKET, 1);
                self.player.start_swing();
            }
            return;
        }
        // Doors and beds occupy two blocks
        if held.id == item::OAK_DOOR_ITEM || held.id == item::BED_ITEM {
            let (nx, ny, nz) = DIRS[hit.face];
            let (px, py, pz) = if block::def(tb).replaceable && tb != WATER && tb != LAVA { (hit.x, hit.y, hit.z) } else { (hit.x + nx, hit.y + ny, hit.z + nz) };
            let free = |g: &Game, x: i32, y: i32, z: i32| {
                let b = g.world.get(x, y, z);
                (b == AIR || block::def(b).replaceable) && b != WATER && b != LAVA && y < CHUNK_H as i32
            };
            let facing = self.player_facing();
            let pbb = self.player.body.aabb();
            let hits_player = |x: i32, y: i32, z: i32| pbb.intersects(&Aabb::new(v3(x as f32, y as f32, z as f32), v3(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0)));
            if held.id == item::OAK_DOOR_ITEM {
                if free(self, px, py, pz) && free(self, px, py + 1, pz) && block::is_opaque(self.world.get(px, py - 1, pz)) {
                    self.set_block_updated(px, py, pz, OAK_DOOR, facing);
                    self.world.set_block(px, py + 1, pz, OAK_DOOR, facing | 8);
                    self.player.inv.consume_held(1);
                    self.player.start_swing();
                    self.sound_at_block("dig.wood", px, py, pz, 0.9);
                }
            } else {
                let (dx, dz) = block::facing_step(facing);
                let (hx, hz) = (px + dx, pz + dz);
                if free(self, px, py, pz) && free(self, hx, py, hz) && block::is_opaque(self.world.get(px, py - 1, pz)) && block::is_opaque(self.world.get(hx, py - 1, hz)) && !hits_player(px, py, pz) && !hits_player(hx, py, hz) {
                    self.set_block_updated(px, py, pz, BED, facing);
                    self.set_block_updated(hx, py, hz, BED, facing | 4);
                    self.player.inv.consume_held(1);
                    self.player.start_swing();
                    self.sound_at_block("dig.cloth", px, py, pz, 0.9);
                }
            }
            return;
        }
        // Block placement
        if !item::is_block(held.id) {
            return;
        }
        let pb = held.id as u8;
        let (nx, ny, nz) = DIRS[hit.face];
        let (mut px, mut py, mut pz) = (hit.x + nx, hit.y + ny, hit.z + nz);
        if block::def(tb).replaceable && tb != WATER && tb != LAVA {
            px = hit.x;
            py = hit.y;
            pz = hit.z;
        }
        if py < 0 || py >= CHUNK_H as i32 {
            return;
        }
        let cur = self.world.get(px, py, pz);
        if !(cur == AIR || block::def(cur).replaceable) {
            return;
        }
        // validity
        let below = self.world.get(px, py - 1, pz);
        let mut meta = 0u8;
        match pb {
            TORCH => {
                // attach to the clicked face
                meta = match hit.face {
                    3 => 0,
                    4 => 2, // clicked north face of block -> torch on its north side, wall to the south
                    5 => 1,
                    0 => 4,
                    1 => 3,
                    _ => return,
                };
                if meta == 0 && !block::is_opaque(below) && below != FARMLAND {
                    return;
                }
                if meta != 0 && !block::is_opaque(tb) {
                    return;
                }
            }
            SHORT_GRASS | FERN | DANDELION | POPPY | BLUE_ORCHID | OAK_SAPLING | BIRCH_SAPLING | SPRUCE_SAPLING | JUNGLE_SAPLING | ACACIA_SAPLING => {
                if !matches!(below, GRASS | DIRT | FARMLAND) {
                    return;
                }
            }
            DEAD_BUSH => {
                if !matches!(below, SAND | DIRT | GRASS) {
                    return;
                }
            }
            SUGAR_CANE => {
                let ok = below == SUGAR_CANE
                    || (matches!(below, GRASS | DIRT | SAND)
                        && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| self.world.get(px + dx, py - 1, pz + dz) == WATER));
                if !ok {
                    return;
                }
            }
            CACTUS => {
                if !(below == SAND || below == CACTUS) {
                    return;
                }
            }
            SNOW_LAYER => {
                if !block::is_opaque(below) {
                    return;
                }
            }
            WHEAT => return,
            LADDER => {
                meta = match hit.face {
                    4 => 1,
                    5 => 0,
                    0 => 3,
                    1 => 2,
                    _ => return,
                };
                if !block::is_opaque(tb) {
                    return;
                }
            }
            _ => {}
        }
        if block::def(pb).front.is_some() {
            // face the player
            let yaw = crate::math::wrap_deg(self.player.yaw);
            meta = if (-45.0..45.0).contains(&yaw) {
                0 // player looks south(+z) -> front faces north(-z)
            } else if (45.0..135.0).contains(&yaw) {
                3 // player looks west -> faces east
            } else if !(-135.0..135.0).contains(&yaw) {
                1
            } else {
                2
            };
        }
        // entity collision
        if let Some((mn, mx)) = block::collision_box(pb) {
            let bb = Aabb::new(v3(px as f32 + mn[0], py as f32 + mn[1], pz as f32 + mn[2]), v3(px as f32 + mx[0], py as f32 + mx[1], pz as f32 + mx[2]));
            if self.player.body.aabb().intersects(&bb) {
                return;
            }
            if self.mobs.iter().any(|m| m.alive() && m.aabb().intersects(&bb)) {
                return;
            }
        }
        self.set_block_updated(px, py, pz, pb, meta);
        if pb == FURNACE {
            self.world.block_entities.insert((px, py, pz), BlockEntity::Furnace(FurnaceState::new()));
        }
        if pb == CHEST {
            self.world.block_entities.insert((px, py, pz), BlockEntity::Chest(vec![ItemStack::EMPTY; 27]));
        }
        if block::is_leaves(pb) {
            self.world.set_meta(px, py, pz, 1); // player-placed: no decay
        }
        self.player.inv.consume_held(1);
        self.player.start_swing();
        self.sound_at_block(crate::audio::place_sound(pb), px, py, pz, 0.9);
    }

    fn fluid_raycast(&self) -> Option<(i32, i32, i32)> {
        let eye = self.player.eye();
        let d = self.player.look_dir();
        let mut t = 0.0;
        while t < REACH {
            let p = eye + d * t;
            let (x, y, z) = p.floor();
            let b = self.world.get(x, y, z);
            if b == WATER || b == LAVA {
                return Some((x, y, z));
            }
            if block::is_solid(b) {
                return None;
            }
            t += 0.05;
        }
        None
    }

    fn try_sleep(&mut self, x: i32, y: i32, z: i32) {
        if self.skydark() < 4.0 {
            self.message("You can only sleep at night");
            return;
        }
        let bp = v3(x as f32 + 0.5, y as f32, z as f32 + 0.5);
        if (self.player.body.pos - bp).len() > 3.5 {
            self.message("You may not rest now; the bed is too far away");
            return;
        }
        let monsters = self.mobs.iter().any(|m| {
            let d = m.body.pos - bp;
            m.alive() && m.kind.hostile() && d.x.abs() < 8.0 && d.z.abs() < 8.0 && d.y.abs() < 5.0
        });
        if monsters {
            self.message("You may not rest now; there are monsters nearby");
            return;
        }
        // lie on the bed, set spawn
        let meta = self.world.get_meta(x, y, z);
        let (dx, dz) = block::facing_step(meta & 3);
        let (hx, hz) = if meta & 4 != 0 { (x, z) } else { (x + dx, z + dz) };
        self.player.spawn = v3(x as f32 + 0.5, y as f32 + 0.6, z as f32 + 0.5);
        self.player.body.pos = v3(hx as f32 + 0.5, y as f32 + 0.5625, hz as f32 + 0.5);
        self.player.body.vel = Vec3::ZERO;
        self.player.prev_pos = self.player.body.pos;
        self.player.sleeping = 1;
        self.message("Respawn point set");
    }

    // ---------------- fluids ----------------

    pub fn schedule_fluid(&mut self, x: i32, y: i32, z: i32) {
        if self.fluid_set.insert((x, y, z)) {
            let b = self.world.get(x, y, z);
            let delay = if b == LAVA { 30 } else { 5 };
            self.fluid_queue.push_back((x, y, z, self.tick_count + delay));
        }
    }

    fn tick_fluids(&mut self) {
        let mut budget = 400;
        while let Some(&(x, y, z, due)) = self.fluid_queue.front() {
            if due > self.tick_count || budget == 0 {
                break;
            }
            budget -= 1;
            self.fluid_queue.pop_front();
            self.fluid_set.remove(&(x, y, z));
            if !self.world.is_loaded(x, z) {
                continue;
            }
            self.update_fluid(x, y, z);
        }
    }

    fn update_fluid(&mut self, x: i32, y: i32, z: i32) {
        let b = self.world.get(x, y, z);
        if b != WATER && b != LAVA {
            return;
        }
        let meta = self.world.get_meta(x, y, z);
        let step: u8 = if b == WATER { 1 } else { 2 };
        let max_level: u8 = 7;
        // lava/water interaction
        if b == LAVA {
            for (dx, dy, dz) in DIRS {
                if dy == -1 {
                    continue;
                }
                if self.world.get(x + dx, y + dy, z + dz) == WATER {
                    let nb = if meta == 0 { OBSIDIAN } else { COBBLESTONE };
                    self.set_block_updated(x, y, z, nb, 0);
                    self.sound_at_block("fizz", x, y, z, 0.5);
                    return;
                }
            }
        }
        let level_of = |w: &World, x: i32, y: i32, z: i32| -> Option<u8> {
            if w.get(x, y, z) == b {
                let m = w.get_meta(x, y, z);
                Some(if m >= 8 { 0 } else { m })
            } else {
                None
            }
        };
        let mut level = meta;
        if meta != 0 {
            // flowing: recompute support
            let above = self.world.get(x, y + 1, z) == b;
            let mut min_n = 99u8;
            let mut sources = 0;
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if let Some(l) = level_of(&self.world, x + dx, y, z + dz) {
                    min_n = min_n.min(l);
                    if self.world.get_meta(x + dx, y, z + dz) == 0 {
                        sources += 1;
                    }
                }
            }
            let new_level = if b == WATER && sources >= 2 && {
                let bl = self.world.get(x, y - 1, z);
                block::is_solid(bl) || (bl == WATER && self.world.get_meta(x, y - 1, z) == 0)
            } {
                0
            } else if above {
                8
            } else if min_n < 99 && min_n + step <= max_level {
                min_n + step
            } else {
                255
            };
            if new_level == 255 {
                self.set_block_updated(x, y, z, AIR, 0);
                return;
            }
            if new_level != meta {
                self.world.set_meta(x, y, z, new_level);
                for (dx, dy, dz) in DIRS {
                    self.schedule_fluid(x + dx, y + dy, z + dz);
                }
                self.schedule_fluid(x, y, z);
                level = new_level;
            }
        }
        // spread down
        let below = self.world.get(x, y - 1, z);
        if y > 0 && (below == AIR || (block::def(below).replaceable && below != WATER && below != LAVA)) {
            if below != AIR {
                self.break_block(x, y - 1, z, false);
            }
            self.world.set_block(x, y - 1, z, b, 8);
            self.schedule_fluid(x, y - 1, z);
            return;
        }
        if y > 0 && below == LAVA && b == WATER {
            let m = self.world.get_meta(x, y - 1, z);
            self.set_block_updated(x, y - 1, z, if m == 0 { OBSIDIAN } else { COBBLESTONE }, 0);
            return;
        }
        let blocked_below = block::is_solid(below) || (below == b && self.world.get_meta(x, y - 1, z) == 0) || (below != b && below != AIR && !block::def(below).replaceable);
        if !blocked_below {
            return;
        }
        let eff = if level >= 8 { 0 } else { level };
        let nl = eff + step;
        if nl > max_level {
            return;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (x + dx, z + dz);
            if !self.world.is_loaded(nx, nz) {
                continue;
            }
            let nb = self.world.get(nx, y, nz);
            if nb == AIR || (block::def(nb).replaceable && nb != WATER && nb != LAVA) {
                if nb != AIR {
                    self.break_block(nx, y, nz, false);
                }
                self.world.set_block(nx, y, nz, b, nl);
                self.schedule_fluid(nx, y, nz);
            } else if nb == b {
                let m = self.world.get_meta(nx, y, nz);
                if m != 0 && m < 8 && m > nl {
                    self.world.set_meta(nx, y, nz, nl);
                    self.schedule_fluid(nx, y, nz);
                }
            } else if (nb == WATER && b == LAVA) || (nb == LAVA && b == WATER) {
                self.schedule_fluid(nx, y, nz);
            }
        }
    }

    // ---------------- random ticks ----------------

    fn random_ticks(&mut self) {
        let (px, _, pz) = self.player.body.pos.floor();
        let (pcx, pcz) = (px >> 4, pz >> 4);
        let keys: Vec<(i32, i32)> = self.world.chunks.keys().copied().filter(|&(x, z)| (x - pcx).abs() <= 6 && (z - pcz).abs() <= 6).collect();
        for (cx, cz) in keys {
            for sy in 0..(CHUNK_H as i32 / 16) {
                for _ in 0..3 {
                    let r = self.rng.next_u64();
                    let x = cx * 16 + (r & 15) as i32;
                    let z = cz * 16 + ((r >> 4) & 15) as i32;
                    let y = sy * 16 + ((r >> 8) & 15) as i32;
                    self.random_tick(x, y, z);
                }
            }
        }
    }

    fn random_tick(&mut self, x: i32, y: i32, z: i32) {
        let b = self.world.get(x, y, z);
        match b {
            WHEAT => {
                let m = self.world.get_meta(x, y, z);
                let light = self.world.sky_light(x, y + 1, z).max(self.world.block_light(x, y + 1, z));
                if m < 7 && light >= 9 {
                    let moist = self.world.get_meta(x, y - 1, z) > 0;
                    if self.rng.chance(if moist { 0.33 } else { 0.15 }) {
                        self.world.set_meta(x, y, z, m + 1);
                    }
                }
            }
            FARMLAND => {
                let mut water = false;
                'o: for dx in -4..=4 {
                    for dz in -4..=4 {
                        for dy in 0..=1 {
                            if self.world.get(x + dx, y + dy, z + dz) == WATER {
                                water = true;
                                break 'o;
                            }
                        }
                    }
                }
                let m = self.world.get_meta(x, y, z);
                if water && m == 0 {
                    self.world.set_meta(x, y, z, 1);
                } else if !water {
                    if m > 0 {
                        self.world.set_meta(x, y, z, 0);
                    } else if self.world.get(x, y + 1, z) != WHEAT && self.rng.chance(0.2) {
                        self.set_block_updated(x, y, z, DIRT, 0);
                    }
                }
            }
            GRASS => {
                if block::is_opaque(self.world.get(x, y + 1, z)) {
                    self.world.set_block(x, y, z, DIRT, 0);
                }
            }
            DIRT => {
                // grass spreads from nearby grass
                let light = self.world.sky_light(x, y + 1, z);
                if light >= 9 && !block::is_opaque(self.world.get(x, y + 1, z)) && self.world.get(x, y + 1, z) != WATER {
                    for _ in 0..4 {
                        let dx = self.rng.range(3) - 1;
                        let dy = self.rng.range(5) - 3;
                        let dz = self.rng.range(3) - 1;
                        if self.world.get(x + dx, y + dy, z + dz) == GRASS {
                            self.world.set_block(x, y, z, GRASS, 0);
                            break;
                        }
                    }
                }
            }
            SUGAR_CANE | CACTUS => {
                if self.world.get(x, y + 1, z) == AIR {
                    let mut h = 1;
                    while self.world.get(x, y - h, z) == b {
                        h += 1;
                    }
                    if h < 3 && self.rng.chance(0.1) {
                        self.set_block_updated(x, y + 1, z, b, 0);
                    }
                }
            }
            OAK_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES | JUNGLE_LEAVES | ACACIA_LEAVES => {
                if self.world.get_meta(x, y, z) != 0 {
                    return;
                }
                let mut found = false;
                'f: for dy in -4..=4i32 {
                    for dz in -4..=4i32 {
                        for dx in -4..=4i32 {
                            if dx.abs() + dy.abs() + dz.abs() > 5 {
                                continue;
                            }
                            if block::is_log(self.world.get(x + dx, y + dy, z + dz)) {
                                found = true;
                                break 'f;
                            }
                        }
                    }
                }
                if !found {
                    self.break_block(x, y, z, false);
                }
            }
            OAK_SAPLING | BIRCH_SAPLING | SPRUCE_SAPLING | JUNGLE_SAPLING | ACACIA_SAPLING => {
                let light = self.world.sky_light(x, y + 1, z).max(self.world.block_light(x, y + 1, z));
                if light >= 9 && self.rng.chance(1.0 / 7.0) {
                    let blocks = self.world.generator.grow_tree(b, x, y, z, self.rng.next_u64());
                    // need room: every log position must be free (air, plant or the sapling itself)
                    let ok = blocks.iter().all(|&(bx, by, bz, nb)| {
                        let cur = self.world.get(bx, by, bz);
                        by < CHUNK_H as i32 && (!block::is_log(nb) || cur == AIR || block::is_leaves(cur) || block::def(cur).replaceable || (bx, by, bz) == (x, y, z) || block::is_plant(cur))
                    });
                    if ok {
                        for (bx, by, bz, nb) in blocks {
                            let cur = self.world.get(bx, by, bz);
                            if block::is_log(nb) || cur == AIR || block::def(cur).replaceable || block::is_sapling(cur) {
                                if nb == DIRT && block::is_solid(cur) {
                                    continue;
                                }
                                self.world.set_block(bx, by, bz, nb, 0);
                            }
                        }
                    }
                }
            }
            ICE => {
                if self.world.block_light(x, y + 1, z) > 11 {
                    self.set_block_updated(x, y, z, WATER, 0);
                }
            }
            _ => {}
        }
    }

    // ---------------- entities ----------------

    fn tick_mobs(&mut self) {
        let mut events = Vec::new();
        let ppos = self.player.body.pos;
        let alive = !self.player.dead;
        let skydark = self.skydark();
        let peaceful = self.options.difficulty == Difficulty::Peaceful;
        let mut amb = Vec::new();
        let mut eat_grass = Vec::new();
        for m in self.mobs.iter_mut() {
            let (x, _, z) = m.body.pos.floor();
            if !self.world.is_loaded(x, z) {
                continue;
            }
            let fuse_before = m.fuse;
            m.tick(&self.world, ppos, alive && !peaceful, skydark, self.rain > 0.2, &mut self.rng, &mut events);
            if m.alive() && self.rng.chance(1.0 / 240.0) {
                if let Some(name) = mob_sound(m.kind) {
                    amb.push((name, m.body.pos));
                }
            }
            if m.kind == MobKind::Sheep && m.sheared && m.alive() && self.rng.chance(1.0 / 600.0) {
                let (x, y, z) = m.body.pos.floor();
                if self.world.get(x, y - 1, z) == GRASS {
                    eat_grass.push((x, y - 1, z));
                    m.sheared = false;
                }
            }
            if m.kind == MobKind::Creeper && fuse_before == 0 && m.fuse > 0 {
                amb.push(("fuse", m.body.pos));
            }
        }
        for (n, p) in amb {
            self.sound(n, Some(p), 0.6);
        }
        for (x, y, z) in eat_grass {
            self.world.set_block(x, y, z, DIRT, 0);
        }
        // simple separation between mobs
        let n = self.mobs.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (self.mobs[i].body.pos, self.mobs[j].body.pos);
                let d = v3(a.x - b.x, 0.0, a.z - b.z);
                let min = self.mobs[i].body.half_w + self.mobs[j].body.half_w;
                let l = d.len();
                if l < min && l > 1e-4 && (a.y - b.y).abs() < 1.5 {
                    let push = d * (0.05 / l);
                    self.mobs[i].body.vel += push;
                    self.mobs[j].body.vel -= push;
                }
            }
        }
        for ev in events {
            match ev {
                MobEvent::AttackPlayer { damage, from } => {
                    let mult = match self.options.difficulty {
                        Difficulty::Peaceful => 0.0,
                        Difficulty::Easy => 0.5,
                        Difficulty::Normal => 1.0,
                        Difficulty::Hard => 1.5,
                    };
                    self.damage_player(damage * mult, Some(from));
                }
                MobEvent::Explode { pos, power } => self.explode(pos, power),
                MobEvent::ShootArrow { from, dir } => {
                    let d = dir + v3(self.rng.uniform(-0.05, 0.05), self.rng.uniform(-0.05, 0.05), self.rng.uniform(-0.05, 0.05));
                    self.arrows.push(Arrow { pos: from, prev_pos: from, vel: d.norm() * 1.6, stuck: false, age: 0, from_player: false, damage: 3.0 });
                    self.sound("bow", Some(from), 0.8);
                }
                MobEvent::LayEgg { pos } => {
                    self.spawn_item(pos, ItemStack::new(item::EGG, 1));
                }
            }
        }
        // deaths: drops + poof particles
        let mut drops = Vec::new();
        for m in self.mobs.iter() {
            if !m.alive() && m.death_time == 20 {
                drops.push((m.body.pos, m.drops(&mut self.rng.clone())));
            }
        }
        for (pos, ds) in drops {
            for d in ds {
                self.spawn_item(pos + v3(0.0, 0.5, 0.0), d);
            }
            for _ in 0..20 {
                let p = pos + v3(self.rng.uniform(-0.5, 0.5), self.rng.uniform(0.0, 1.0), self.rng.uniform(-0.5, 0.5));
                let v = v3(self.rng.uniform(-0.03, 0.03), self.rng.uniform(0.0, 0.05), self.rng.uniform(-0.03, 0.03));
                self.particles.push(Particle { pos: p, prev_pos: p, vel: v, age: 0, life: 20 + self.rng.range(10), size: 0.07 + self.rng.next_f32() * 0.05, uv: [0.0; 4], color: [235, 235, 235, 255], gravity: -0.002, textured: false, emissive: false });
            }
            self.rng.next_u64();
        }
        // despawn
        let pp = self.player.body.pos;
        let rng = &mut self.rng;
        self.mobs.retain(|m| {
            if !m.alive() && m.death_time >= 20 {
                return false;
            }
            let d = (m.body.pos - pp).len();
            if m.kind.hostile() && !m.persistent {
                if d > 128.0 {
                    return false;
                }
                if d > 32.0 && rng.chance(1.0 / 800.0) {
                    return false;
                }
            }
            if peaceful && m.kind.hostile() {
                return false;
            }
            d < 200.0
        });
    }

    pub fn explode(&mut self, pos: Vec3, power: f32) {
        self.sound("explode", Some(pos), 1.0);
        self.screen_shake = 1.0;
        let r = power.ceil() as i32 + 1;
        let (cx, cy, cz) = pos.floor();
        let mut destroyed = Vec::new();
        for dy in -r..=r {
            for dz in -r..=r {
                for dx in -r..=r {
                    let d = ((dx * dx + dy * dy + dz * dz) as f32).sqrt();
                    let strength = power * (0.7 + self.rng.next_f32() * 0.6);
                    if d > strength {
                        continue;
                    }
                    let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                    let b = self.world.get(x, y, z);
                    if b == AIR || b == BEDROCK || b == OBSIDIAN || b == WATER || b == LAVA {
                        continue;
                    }
                    destroyed.push((x, y, z, b));
                }
            }
        }
        for &(x, y, z, b) in &destroyed {
            let meta = self.world.get_meta(x, y, z);
            if self.rng.chance(0.3) {
                for st in self.block_drops(b, meta) {
                    let p = v3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                    self.spawn_item(p, st);
                }
            }
            self.world.block_entities.remove(&(x, y, z));
            self.world.set_block(x, y, z, AIR, 0);
        }
        for &(x, y, z, _) in &destroyed {
            for (dx, dy, dz) in DIRS {
                self.neighbor_changed(x + dx, y + dy, z + dz);
            }
        }
        // particles
        for _ in 0..40 {
            let p = pos + v3(self.rng.uniform(-2.0, 2.0), self.rng.uniform(-1.0, 2.0), self.rng.uniform(-2.0, 2.0));
            let v = v3(self.rng.uniform(-0.1, 0.1), self.rng.uniform(0.0, 0.1), self.rng.uniform(-0.1, 0.1));
            let g = 200 + self.rng.range(55) as u8;
            self.particles.push(Particle { pos: p, prev_pos: p, vel: v, age: 0, life: 20 + self.rng.range(20), size: 0.3 + self.rng.next_f32() * 0.3, uv: [0.0; 4], color: [g, g, g, 255], gravity: -0.001, textured: false, emissive: true });
        }
        // damage entities
        let diff_mult = match self.options.difficulty {
            Difficulty::Peaceful => 0.0,
            Difficulty::Easy => 0.5,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.5,
        };
        let pp = self.player.body.pos + v3(0.0, 0.9, 0.0);
        let d = (pp - pos).len();
        if d < power * 2.0 {
            let impact = 1.0 - d / (power * 2.0);
            let dmg = ((impact * impact + impact) / 2.0 * 7.0 * power * 2.0 + 1.0) * diff_mult;
            self.damage_player(dmg, Some(pos));
            let push = (pp - pos).norm() * impact;
            self.player.body.vel += push;
        }
        for m in self.mobs.iter_mut() {
            let d = (m.body.pos - pos).len();
            if d < power * 2.0 && m.alive() {
                let impact = 1.0 - d / (power * 2.0);
                let dmg = (impact * impact + impact) / 2.0 * 7.0 * power * 2.0 + 1.0;
                m.invuln = 0;
                m.hurt(dmg, pos, &mut self.rng);
            }
        }
    }

    fn tick_items(&mut self) {
        for it in self.items.iter_mut() {
            it.tick(&self.world);
        }
        // merge nearby identical stacks
        let n = self.items.len();
        if self.tick_count % 10 == 0 {
            for i in 0..n {
                for j in i + 1..n {
                    if self.items[i].stack.count == 0 || self.items[j].stack.count == 0 {
                        continue;
                    }
                    if self.items[i].stack.can_stack_with(&self.items[j].stack) && (self.items[i].body.pos - self.items[j].body.pos).len() < 0.6 {
                        let max = item::max_stack(self.items[i].stack.id);
                        let total = self.items[i].stack.count as u32 + self.items[j].stack.count as u32;
                        if total <= max as u32 {
                            self.items[i].stack.count = total as u8;
                            self.items[j].stack.count = 0;
                        }
                    }
                }
            }
        }
        self.items.retain(|i| i.age < 6000 && i.stack.count > 0 && i.body.pos.y > -64.0 && !(i.body.in_lava));
    }

    fn tick_falling(&mut self) {
        let mut landed = Vec::new();
        for (i, f) in self.falling.iter_mut().enumerate() {
            if let Some(p) = f.tick(&self.world) {
                landed.push((i, p, f.block));
            }
        }
        for &(i, (x, y, z), b) in landed.iter().rev() {
            self.falling.remove(i);
            let cur = self.world.get(x, y, z);
            if cur == AIR || block::def(cur).replaceable {
                if cur != AIR && cur != WATER && cur != LAVA {
                    self.break_block(x, y, z, false);
                }
                self.set_block_updated(x, y, z, b, 0);
            } else {
                let p = v3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                self.spawn_item(p, ItemStack::new(b as u16, 1));
            }
        }
    }

    fn tick_arrows(&mut self) {
        let mut hits_player = Vec::new();
        let mut sounds: Vec<(&'static str, Vec3, f32)> = Vec::new();
        for a in self.arrows.iter_mut() {
            a.prev_pos = a.pos;
            a.age += 1;
            if a.stuck {
                continue;
            }
            let steps = 4;
            for _ in 0..steps {
                let np = a.pos + a.vel * (1.0 / steps as f32);
                let (x, y, z) = np.floor();
                if block::is_solid(self.world.get(x, y, z)) {
                    a.stuck = true;
                    sounds.push(("arrow.hit", a.pos, 1.0));
                    break;
                }
                a.pos = np;
                if a.from_player {
                    let pt = Aabb::new(a.pos, a.pos);
                    if let Some(m) = self.mobs.iter_mut().find(|m| m.alive() && m.aabb().grow(0.15).intersects(&pt)) {
                        let speed = a.vel.len();
                        let dmg = (a.damage * speed / 3.0).max(1.0).ceil();
                        m.invuln = 0;
                        let from = a.pos - a.vel;
                        if m.hurt(dmg, from, &mut self.rng) {
                            sounds.push((mob_hurt_sound(m.kind), m.body.pos, if m.alive() { 1.0 } else { 0.8 }));
                        }
                        sounds.push(("arrow.hit", a.pos, 1.0));
                        a.age = 10000;
                        break;
                    }
                }
                if !a.from_player {
                    let pb = self.player.body.aabb();
                    if pb.grow(0.1).intersects(&Aabb::new(a.pos, a.pos)) && !self.player.dead {
                        hits_player.push(a.damage);
                        a.age = 10000;
                        break;
                    }
                }
            }
            a.vel = a.vel * 0.99;
            a.vel.y -= 0.05;
        }
        for (name, pos, pitch) in sounds {
            self.sounds.push(SoundEvent { name, pos: Some(pos), volume: 0.8, pitch: pitch * (0.9 + self.rng.next_f32() * 0.2) });
        }
        for (i, dmg) in hits_player.into_iter().enumerate() {
            let _ = i;
            let from = self.player.body.pos - v3(0.0, 0.0, 0.0);
            self.damage_player(dmg, Some(from));
        }
        // pick up stuck player arrows
        let pbb = self.player.body.aabb().grow(0.6);
        let mut got = 0;
        for a in self.arrows.iter_mut() {
            if a.stuck && a.from_player && a.age > 5 && a.age < 1200 && pbb.intersects(&Aabb::new(a.pos, a.pos)) {
                if self.player.inv.add(ItemStack::new(item::ARROW, 1)) == 0 {
                    a.age = 10000;
                    got += 1;
                }
            }
        }
        if got > 0 {
            let pos = Some(self.player.body.pos);
            self.sounds.push(SoundEvent { name: "pop", pos, volume: 0.25, pitch: 1.6 });
        }
        self.arrows.retain(|a| a.age < 1200);
    }

    fn spawn_mobs(&mut self) {
        // Passive animals on newly generated chunks
        let keys: Vec<(i32, i32)> = self.world.chunks.keys().copied().filter(|k| !self.spawned_chunks.contains(k)).collect();
        for (cx, cz) in keys {
            self.spawned_chunks.insert((cx, cz));
            if !self.rng.chance(0.1) {
                continue;
            }
            let x = cx * 16 + 2 + self.rng.range(12);
            let z = cz * 16 + 2 + self.rng.range(12);
            let biome = self.world.biome(x, z);
            let kind = match biome {
                Biome::Desert | Biome::Beach | Biome::Ocean | Biome::DeepOcean | Biome::River | Biome::FrozenOcean | Biome::SnowyBeach => continue,
                _ => match self.rng.range(4) {
                    0 => MobKind::Pig,
                    1 => MobKind::Cow,
                    2 => MobKind::Sheep,
                    _ => MobKind::Chicken,
                },
            };
            let count = 2 + self.rng.range(3);
            for _ in 0..count {
                let sx = x + self.rng.range(5) - 2;
                let sz = z + self.rng.range(5) - 2;
                let y = self.world.surface_height(sx, sz);
                if y <= 0 || self.world.get(sx, y - 1, sz) != GRASS {
                    continue;
                }
                let m = Mob::new(kind, v3(sx as f32 + 0.5, y as f32, sz as f32 + 0.5), &mut self.rng);
                self.mobs.push(m);
            }
        }
        // Hostile mobs in the dark
        if self.options.difficulty == Difficulty::Peaceful || self.tick_count % 10 != 0 {
            return;
        }
        let hostile = self.mobs.iter().filter(|m| m.kind.hostile()).count();
        if hostile >= 30 {
            return;
        }
        let skydark = self.skydark();
        for _ in 0..3 {
            let a = self.rng.uniform(0.0, std::f32::consts::TAU);
            let r = self.rng.uniform(24.0, 56.0);
            let pp = self.player.body.pos;
            let x = (pp.x + a.cos() * r).floor() as i32;
            let z = (pp.z + a.sin() * r).floor() as i32;
            if !self.world.is_loaded(x, z) {
                continue;
            }
            // pick a random y: surface or underground
            let top = self.world.surface_height(x, z);
            let y = if self.rng.chance(0.5) { top } else { self.rng.range(top.max(2)) };
            if y < 1 {
                continue;
            }
            if !block::is_opaque(self.world.get(x, y - 1, z)) || block::is_solid(self.world.get(x, y, z)) || block::is_solid(self.world.get(x, y + 1, z)) {
                continue;
            }
            if self.world.get(x, y, z) == WATER || self.world.get(x, y, z) == LAVA {
                continue;
            }
            let sky = (self.world.sky_light(x, y, z) as f32 - skydark).max(0.0);
            let bl = self.world.block_light(x, y, z) as f32;
            if sky.max(bl) > 7.0 || bl > 0.0 {
                continue;
            }
            let kind = match self.rng.range(100) {
                0..=34 => MobKind::Zombie,
                35..=59 => MobKind::Skeleton,
                60..=84 => MobKind::Creeper,
                _ => MobKind::Spider,
            };
            let m = Mob::new(kind, v3(x as f32 + 0.5, y as f32, z as f32 + 0.5), &mut self.rng);
            let bb = m.aabb();
            if !physics::block_boxes(&self.world, &bb).iter().any(|b| b.intersects(&bb)) {
                self.mobs.push(m);
            }
        }
    }
}

/// Ambient sound a mob makes now and then (creepers are silent, as in vanilla).
pub fn mob_sound(k: MobKind) -> Option<&'static str> {
    Some(match k {
        MobKind::Pig => "pig",
        MobKind::Cow => "cow",
        MobKind::Sheep => "sheep",
        MobKind::Chicken => "chicken",
        MobKind::Zombie => "zombie",
        MobKind::Skeleton => "skeleton",
        MobKind::Spider => "spider",
        MobKind::Creeper => return None,
    })
}

pub fn mob_hurt_sound(k: MobKind) -> &'static str {
    match k {
        MobKind::Pig => "pig.hurt",
        MobKind::Cow => "cow.hurt",
        MobKind::Sheep => "sheep.hurt",
        MobKind::Chicken => "chicken.hurt",
        MobKind::Zombie => "zombie.hurt",
        MobKind::Skeleton => "skeleton.hurt",
        MobKind::Spider => "spider.hurt",
        MobKind::Creeper => "creeper.hurt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with(item: u16) -> Game {
        let mut g = Game::new();
        g.player.inv.slots[0] = ItemStack::new(item, 1);
        g.player.inv.selected = 0;
        g.player.body.on_ground = true;
        g
    }

    #[test]
    fn harvest_levels() {
        let g = game_with(0);
        assert!(g.can_harvest(DIRT));
        assert!(!g.can_harvest(STONE));
        let g = game_with(item::WOODEN_PICKAXE);
        assert!(g.can_harvest(STONE));
        assert!(g.can_harvest(COAL_ORE));
        assert!(!g.can_harvest(IRON_ORE));
        let g = game_with(item::STONE_PICKAXE);
        assert!(g.can_harvest(IRON_ORE));
        assert!(!g.can_harvest(DIAMOND_ORE));
        let g = game_with(item::IRON_PICKAXE);
        assert!(g.can_harvest(DIAMOND_ORE));
        assert!(g.can_harvest(GOLD_ORE));
        assert!(!g.can_harvest(OBSIDIAN));
        let g = game_with(item::DIAMOND_PICKAXE);
        assert!(g.can_harvest(OBSIDIAN));
    }

    #[test]
    fn break_times_match_vanilla() {
        // ticks to break = 1 / speed
        let ticks = |item: u16, b: u8| (1.0 / game_with(item).break_speed(b)).ceil() as i32;
        assert_eq!(ticks(0, DIRT), 15); // 0.75 s by hand
        assert_eq!(ticks(0, OAK_LOG), 60); // 3 s by hand
        assert_eq!(ticks(item::WOODEN_PICKAXE, STONE), 23); // ~1.15 s
        assert_eq!(ticks(item::STONE_PICKAXE, STONE), 12);
        assert!(ticks(0, STONE) > 100); // 7.5 s without a pickaxe
        assert_eq!(ticks(item::DIAMOND_SHOVEL, DIRT), 2);
    }

    #[test]
    fn drops() {
        let mut g = game_with(0);
        assert_eq!(g.block_drops(STONE, 0)[0].id, COBBLESTONE as u16);
        assert_eq!(g.block_drops(GRASS, 0)[0].id, DIRT as u16);
        assert_eq!(g.block_drops(COAL_ORE, 0)[0].id, item::COAL);
        assert_eq!(g.block_drops(DIAMOND_ORE, 0)[0].id, item::DIAMOND);
        let ripe = g.block_drops(WHEAT, 7);
        assert!(ripe.iter().any(|s| s.id == item::WHEAT));
        assert_eq!(g.block_drops(WHEAT, 3)[0].id, item::WHEAT_SEEDS);
    }
}
