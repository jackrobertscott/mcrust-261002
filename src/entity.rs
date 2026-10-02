//! Mobs (with simple vanilla-like AI), dropped items, falling blocks, arrows and particles.

use crate::block::{self, *};
use crate::item::{self, ItemStack};
use crate::math::{v3, wrap_deg, Vec3};
use crate::models::ModelKind;
use crate::noise::Random;
use crate::physics::{Aabb, Body};
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobKind {
    Pig,
    Cow,
    Sheep,
    Chicken,
    Zombie,
    Skeleton,
    Creeper,
    Spider,
}

impl MobKind {
    pub fn hostile(self) -> bool {
        matches!(self, MobKind::Zombie | MobKind::Skeleton | MobKind::Creeper | MobKind::Spider)
    }
    pub fn max_health(self) -> f32 {
        match self {
            MobKind::Pig | MobKind::Cow => 10.0,
            MobKind::Sheep => 8.0,
            MobKind::Chicken => 4.0,
            MobKind::Spider => 16.0,
            _ => 20.0,
        }
    }
    /// (half width, height)
    pub fn size(self) -> (f32, f32) {
        match self {
            MobKind::Pig => (0.45, 0.9),
            MobKind::Cow => (0.45, 1.4),
            MobKind::Sheep => (0.45, 1.3),
            MobKind::Chicken => (0.2, 0.7),
            MobKind::Zombie | MobKind::Skeleton => (0.3, 1.95),
            MobKind::Creeper => (0.3, 1.7),
            MobKind::Spider => (0.7, 0.9),
        }
    }
    pub fn speed(self) -> f32 {
        match self {
            MobKind::Zombie => 0.23,
            MobKind::Spider => 0.3,
            MobKind::Chicken => 0.25,
            MobKind::Cow => 0.2,
            _ => 0.25,
        }
    }
    pub fn model(self) -> ModelKind {
        match self {
            MobKind::Pig => ModelKind::Pig,
            MobKind::Cow => ModelKind::Cow,
            MobKind::Sheep => ModelKind::Sheep,
            MobKind::Chicken => ModelKind::Chicken,
            MobKind::Zombie => ModelKind::Biped,
            MobKind::Skeleton => ModelKind::Skeleton,
            MobKind::Creeper => ModelKind::Creeper,
            MobKind::Spider => ModelKind::Spider,
        }
    }
    pub fn skin(self) -> &'static str {
        match self {
            MobKind::Pig => "pig",
            MobKind::Cow => "cow",
            MobKind::Sheep => "sheep",
            MobKind::Chicken => "chicken",
            MobKind::Zombie => "zombie",
            MobKind::Skeleton => "skeleton",
            MobKind::Creeper => "creeper",
            MobKind::Spider => "spider",
        }
    }
    pub fn eye_height(self) -> f32 {
        self.size().1 * 0.85
    }
}

pub struct Mob {
    pub kind: MobKind,
    pub body: Body,
    pub prev_pos: Vec3,
    pub yaw: f32,
    pub prev_yaw: f32,
    pub head_yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub hurt_time: i32,
    pub invuln: i32,
    pub death_time: i32,
    pub limb_swing: f32,
    pub limb_amount: f32,
    pub prev_limb_amount: f32,
    pub age: i32,
    pub target: Option<Vec3>,
    pub wander_cooldown: i32,
    pub panic: i32,
    pub attack_cooldown: i32,
    pub fuse: i32,
    pub prev_fuse: i32,
    pub fire: i32,
    pub sheared: bool,
    pub persistent: bool,
    pub swing: f32,
    pub forward: f32,
    pub jump: bool,
    pub egg_timer: i32,
}

/// Something a mob did that the game must handle.
pub enum MobEvent {
    AttackPlayer { damage: f32, from: Vec3 },
    Explode { pos: Vec3, power: f32 },
    ShootArrow { from: Vec3, dir: Vec3 },
    LayEgg { pos: Vec3 },
}

impl Mob {
    pub fn new(kind: MobKind, pos: Vec3, rng: &mut Random) -> Mob {
        let (hw, h) = kind.size();
        let yaw = rng.uniform(0.0, 360.0);
        Mob {
            kind,
            body: Body::new(pos, hw, h),
            prev_pos: pos,
            yaw,
            prev_yaw: yaw,
            head_yaw: yaw,
            pitch: 0.0,
            health: kind.max_health(),
            hurt_time: 0,
            invuln: 0,
            death_time: 0,
            limb_swing: 0.0,
            limb_amount: 0.0,
            prev_limb_amount: 0.0,
            age: 0,
            target: None,
            wander_cooldown: rng.range(100),
            panic: 0,
            attack_cooldown: 0,
            fuse: 0,
            prev_fuse: 0,
            fire: 0,
            sheared: false,
            persistent: false,
            swing: 0.0,
            forward: 0.0,
            jump: false,
            egg_timer: 6000 + rng.range(6000),
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn eye(&self) -> Vec3 {
        self.body.pos + v3(0.0, self.kind.eye_height(), 0.0)
    }

    pub fn aabb(&self) -> Aabb {
        self.body.aabb()
    }

    /// Apply damage with knockback. Returns true if it took damage.
    pub fn hurt(&mut self, dmg: f32, from: Vec3, rng: &mut Random) -> bool {
        if self.invuln > 0 || !self.alive() {
            return false;
        }
        self.health -= dmg;
        self.hurt_time = 10;
        self.invuln = 10;
        let d = self.body.pos - from;
        let l = (d.x * d.x + d.z * d.z).sqrt().max(0.01);
        self.body.vel.x = self.body.vel.x * 0.5 + d.x / l * 0.4;
        self.body.vel.z = self.body.vel.z * 0.5 + d.z / l * 0.4;
        self.body.vel.y = (self.body.vel.y * 0.5 + 0.4).min(0.4);
        if !self.kind.hostile() {
            self.panic = 100;
            self.target = None;
        }
        let _ = rng;
        true
    }

    pub fn drops(&self, rng: &mut Random) -> Vec<ItemStack> {
        let mut v = Vec::new();
        let mut add = |id, lo: i32, hi: i32, rng: &mut Random| {
            let n = lo + rng.range(hi - lo + 1);
            if n > 0 {
                v.push(ItemStack::new(id, n as u8));
            }
        };
        let fire = self.fire > 0;
        match self.kind {
            MobKind::Pig => add(if fire { item::COOKED_PORKCHOP } else { item::PORKCHOP }, 1, 3, rng),
            MobKind::Cow => {
                add(item::LEATHER, 0, 2, rng);
                add(if fire { item::COOKED_BEEF } else { item::BEEF }, 1, 3, rng);
            }
            MobKind::Sheep => {
                if !self.sheared {
                    add(block::WOOL as u16, 1, 1, rng);
                }
            }
            MobKind::Chicken => {
                add(item::FEATHER, 0, 2, rng);
                add(if fire { item::COOKED_CHICKEN } else { item::CHICKEN }, 1, 1, rng);
            }
            MobKind::Zombie => add(item::ROTTEN_FLESH, 0, 2, rng),
            MobKind::Skeleton => {
                add(item::BONE, 0, 2, rng);
                add(item::ARROW, 0, 2, rng);
            }
            MobKind::Creeper => add(item::GUNPOWDER, 0, 2, rng),
            MobKind::Spider => add(item::STRING, 0, 2, rng),
        }
        v
    }

    /// One game tick of AI + physics.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(&mut self, world: &World, player_pos: Vec3, player_alive: bool, skydark: f32, raining: bool, rng: &mut Random, events: &mut Vec<MobEvent>) {
        self.prev_pos = self.body.pos;
        self.prev_yaw = self.yaw;
        self.prev_limb_amount = self.limb_amount;
        self.prev_fuse = self.fuse;
        self.age += 1;
        if self.hurt_time > 0 {
            self.hurt_time -= 1;
        }
        if self.invuln > 0 {
            self.invuln -= 1;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
        if self.swing > 0.0 {
            self.swing += 1.0 / 6.0;
            if self.swing >= 1.0 {
                self.swing = 0.0;
            }
        }
        if !self.alive() {
            self.death_time += 1;
            self.body.vel.x *= 0.5;
            self.body.vel.z *= 0.5;
            self.physics(world);
            return;
        }

        // Burning in daylight (undead)
        if matches!(self.kind, MobKind::Zombie | MobKind::Skeleton) && skydark < 4.0 && !raining && !self.body.in_water && !self.persistent {
            let (x, y, z) = self.eye().floor();
            if world.sky_light(x, y, z) == 15 && rng.chance(0.03) {
                self.fire = self.fire.max(160);
            }
        }
        if self.fire > 0 {
            self.fire -= 1;
            if self.body.in_water {
                self.fire = 0;
            }
            if self.fire % 20 == 0 {
                self.health -= 1.0;
                self.hurt_time = 10;
            }
        }
        if self.body.in_lava {
            self.fire = 300;
            if self.invuln == 0 {
                self.health -= 4.0;
                self.hurt_time = 10;
                self.invuln = 10;
            }
        }

        self.forward = 0.0;
        self.jump = false;
        let to_player = player_pos - self.body.pos;
        let dist_p = to_player.len();
        let dark = skydark >= 4.0;
        let aggressive = self.kind.hostile() && player_alive && dist_p < 16.0 && (self.kind != MobKind::Spider || dark || self.hurt_time > 0);
        let mut look_at: Option<Vec3> = None;

        if aggressive {
            look_at = Some(player_pos + v3(0.0, 1.6, 0.0));
            match self.kind {
                MobKind::Skeleton => {
                    // keep distance, shoot
                    let desired = 8.0;
                    self.face(player_pos, 30.0);
                    if dist_p > desired + 2.0 {
                        self.forward = 1.0;
                    } else if dist_p < desired - 3.0 {
                        self.forward = -0.6;
                    }
                    if self.attack_cooldown == 0 && dist_p < 15.0 && self.can_see(world, player_pos + v3(0.0, 1.5, 0.0)) {
                        self.attack_cooldown = 40 + rng.range(20);
                        let from = self.eye();
                        let mut dir = (player_pos + v3(0.0, 1.2, 0.0)) - from;
                        let horiz = (dir.x * dir.x + dir.z * dir.z).sqrt();
                        dir.y += horiz * 0.2;
                        events.push(MobEvent::ShootArrow { from, dir: dir.norm() });
                        self.swing = 0.01;
                    }
                }
                MobKind::Creeper => {
                    self.face(player_pos, 30.0);
                    if dist_p < 3.0 || (self.fuse > 0 && dist_p < 7.0) {
                        self.fuse += 1;
                        if self.fuse >= 30 {
                            events.push(MobEvent::Explode { pos: self.body.pos + v3(0.0, 0.8, 0.0), power: 3.0 });
                            self.health = 0.0;
                            self.death_time = 100; // vanish
                        }
                    } else {
                        self.fuse = (self.fuse - 1).max(0);
                        self.forward = 1.0;
                    }
                }
                _ => {
                    self.face(player_pos, 30.0);
                    self.forward = 1.0;
                    let reach = self.body.half_w * 2.0 + 0.7;
                    let horiz = (to_player.x * to_player.x + to_player.z * to_player.z).sqrt();
                    if horiz < reach + 0.3 && to_player.y.abs() < 2.0 && self.attack_cooldown == 0 {
                        self.attack_cooldown = 20;
                        self.swing = 0.01;
                        let dmg = match self.kind {
                            MobKind::Zombie => 3.0,
                            MobKind::Spider => 2.0,
                            _ => 2.0,
                        };
                        events.push(MobEvent::AttackPlayer { damage: dmg, from: self.body.pos });
                    }
                    if self.kind == MobKind::Spider && dist_p > 2.0 && dist_p < 6.0 && self.body.on_ground && rng.chance(0.1) {
                        // leap
                        let d = to_player.norm();
                        self.body.vel.x = d.x * 0.4;
                        self.body.vel.z = d.z * 0.4;
                        self.body.vel.y = 0.4;
                    }
                }
            }
        } else {
            if self.kind == MobKind::Creeper {
                self.fuse = (self.fuse - 1).max(0);
            }
            // Wander / panic
            if self.panic > 0 {
                self.panic -= 1;
                if self.target.is_none() || rng.chance(0.05) {
                    let a = rng.uniform(0.0, std::f32::consts::TAU);
                    self.target = Some(self.body.pos + v3(a.cos() * 8.0, 0.0, a.sin() * 8.0));
                }
            } else if self.wander_cooldown > 0 {
                self.wander_cooldown -= 1;
            } else {
                self.wander_cooldown = 80 + rng.range(120);
                if rng.chance(0.6) {
                    let a = rng.uniform(0.0, std::f32::consts::TAU);
                    let r = rng.uniform(3.0, 9.0);
                    self.target = Some(self.body.pos + v3(a.cos() * r, 0.0, a.sin() * r));
                } else {
                    self.target = None;
                }
            }
            if let Some(t) = self.target {
                let d = t - self.body.pos;
                if d.x * d.x + d.z * d.z < 1.0 {
                    self.target = None;
                } else {
                    self.face(t, 20.0);
                    self.forward = if self.panic > 0 { 1.6 } else { 0.8 };
                    // avoid walking off big drops / into water when calm
                    if self.panic == 0 && !self.safe_ahead(world) {
                        self.target = None;
                        self.forward = 0.0;
                    }
                }
            }
            // Passive mobs look at nearby player
            if !self.kind.hostile() && dist_p < 6.0 && self.target.is_none() {
                look_at = Some(player_pos + v3(0.0, 1.6, 0.0));
            }
        }

        // Head orientation
        if let Some(l) = look_at {
            let d = l - self.eye();
            let target_yaw = (-d.x).atan2(d.z).to_degrees();
            let diff = wrap_deg(target_yaw - self.yaw).clamp(-75.0, 75.0);
            self.head_yaw = self.yaw + diff;
            let h = (d.x * d.x + d.z * d.z).sqrt();
            self.pitch = (-d.y).atan2(h).to_degrees().clamp(-40.0, 40.0);
        } else {
            self.head_yaw += wrap_deg(self.yaw - self.head_yaw) * 0.3;
            self.pitch *= 0.8;
        }

        // Jump when blocked or in water
        if self.forward != 0.0 && self.body.collided_h && self.body.on_ground {
            self.jump = true;
        }
        if self.body.in_water || self.body.in_lava {
            self.jump = rng.chance(0.8);
        }
        if self.kind == MobKind::Spider && self.body.collided_h && aggressive {
            self.body.vel.y = 0.2; // climb walls
        }

        // Chickens lay eggs
        if self.kind == MobKind::Chicken {
            self.egg_timer -= 1;
            if self.egg_timer <= 0 {
                self.egg_timer = 6000 + rng.range(6000);
                events.push(MobEvent::LayEgg { pos: self.body.pos });
            }
        }

        // Movement
        let speed = self.kind.speed() * if self.kind == MobKind::Creeper && self.fuse > 0 { 0.0 } else { 1.0 };
        let yaw_r = self.yaw.to_radians();
        let dir = v3(-yaw_r.sin(), 0.0, yaw_r.cos());
        let accel = if self.body.on_ground { speed * 0.4 } else { 0.02 };
        let fwd = self.forward.clamp(-1.0, 1.6);
        self.body.vel.x += dir.x * fwd * accel * 0.5;
        self.body.vel.z += dir.z * fwd * accel * 0.5;
        if self.jump && self.body.on_ground {
            self.body.vel.y = 0.42;
        } else if self.jump && (self.body.in_water || self.body.in_lava) {
            self.body.vel.y += 0.04;
        }
        self.physics(world);

        // limb animation
        let dx = self.body.pos.x - self.prev_pos.x;
        let dz = self.body.pos.z - self.prev_pos.z;
        let moved = ((dx * dx + dz * dz).sqrt() * 4.0).min(1.0);
        self.limb_amount += (moved - self.limb_amount) * 0.4;
        self.limb_swing += self.limb_amount;
    }

    fn face(&mut self, p: Vec3, max_turn: f32) {
        let d = p - self.body.pos;
        if d.x.abs() < 1e-4 && d.z.abs() < 1e-4 {
            return;
        }
        let target = (-d.x).atan2(d.z).to_degrees();
        let diff = wrap_deg(target - self.yaw).clamp(-max_turn, max_turn);
        self.yaw = wrap_deg(self.yaw + diff);
    }

    fn safe_ahead(&self, world: &World) -> bool {
        let yaw_r = self.yaw.to_radians();
        let ahead = self.body.pos + v3(-yaw_r.sin() * 1.0, 0.0, yaw_r.cos() * 1.0);
        let (x, y, z) = ahead.floor();
        for dy in 0..4 {
            let b = world.get(x, y - 1 - dy, z);
            if b == WATER || b == LAVA {
                return false;
            }
            if block::is_solid(b) {
                return true;
            }
        }
        false
    }

    fn can_see(&self, world: &World, target: Vec3) -> bool {
        let from = self.eye();
        let d = target - from;
        let n = (d.len() * 2.0) as i32 + 1;
        for i in 1..n {
            let p = from + d * (i as f32 / n as f32);
            let (x, y, z) = p.floor();
            if block::is_opaque(world.get(x, y, z)) {
                return false;
            }
        }
        true
    }

    fn physics(&mut self, world: &World) {
        let b = &mut self.body;
        if b.in_water || b.in_lava {
            let drag = if b.in_water { 0.8 } else { 0.5 };
            let v = b.vel;
            b.move_by(world, v, false);
            b.vel.x *= drag;
            b.vel.z *= drag;
            b.vel.y = b.vel.y * drag - 0.02;
        } else {
            let v = b.vel;
            b.move_by(world, v, false);
            let fr = if b.on_ground { 0.546 } else { 0.91 };
            b.vel.x *= fr;
            b.vel.z *= fr;
            b.vel.y = (b.vel.y - 0.08) * 0.98;
            if self.kind == MobKind::Chicken && !b.on_ground && b.vel.y < 0.0 {
                b.vel.y *= 0.6; // flutter
            }
        }
    }
}

// ---------------- Item entities ----------------

pub struct ItemEntity {
    pub body: Body,
    pub prev_pos: Vec3,
    pub stack: ItemStack,
    pub age: i32,
    pub pickup_delay: i32,
    pub bob: f32,
}

impl ItemEntity {
    pub fn new(pos: Vec3, stack: ItemStack, rng: &mut Random) -> ItemEntity {
        let mut body = Body::new(pos, 0.125, 0.25);
        body.vel = v3(rng.uniform(-0.1, 0.1), 0.2, rng.uniform(-0.1, 0.1));
        body.step_height = 0.0;
        ItemEntity { body, prev_pos: pos, stack, age: 0, pickup_delay: 10, bob: rng.uniform(0.0, 6.28) }
    }
    pub fn tick(&mut self, world: &World) {
        self.prev_pos = self.body.pos;
        self.age += 1;
        if self.pickup_delay > 0 {
            self.pickup_delay -= 1;
        }
        let b = &mut self.body;
        // push out of solid blocks
        let (x, y, z) = (b.pos + v3(0.0, 0.1, 0.0)).floor();
        if block::is_solid(world.get(x, y, z)) {
            b.vel.y = 0.2;
            b.pos.y += 0.05;
        }
        if b.in_water {
            b.vel.y += 0.01;
            b.vel.y *= 0.9;
        } else {
            b.vel.y -= 0.04;
        }
        let v = b.vel;
        b.move_by(world, v, false);
        let fr = if b.on_ground { 0.588 } else { 0.98 };
        b.vel.x *= fr;
        b.vel.z *= fr;
        b.vel.y *= 0.98;
        if b.on_ground {
            b.vel.y *= -0.5;
        }
    }
}

// ---------------- Falling blocks ----------------

pub struct FallingBlock {
    pub body: Body,
    pub prev_pos: Vec3,
    pub block: u8,
    pub age: i32,
}

impl FallingBlock {
    pub fn new(x: i32, y: i32, z: i32, b: u8) -> FallingBlock {
        let pos = v3(x as f32 + 0.5, y as f32, z as f32 + 0.5);
        let mut body = Body::new(pos, 0.49, 0.98);
        body.step_height = 0.0;
        FallingBlock { body, prev_pos: pos, block: b, age: 0 }
    }
    /// Returns Some(position) when landed.
    pub fn tick(&mut self, world: &World) -> Option<(i32, i32, i32)> {
        self.prev_pos = self.body.pos;
        self.age += 1;
        self.body.vel.y = (self.body.vel.y - 0.04) * 0.98;
        let v = self.body.vel;
        self.body.move_by(world, v, false);
        if self.body.on_ground || self.age > 600 {
            let p = self.body.pos + v3(0.0, 0.5, 0.0);
            return Some(p.floor());
        }
        None
    }
}

// ---------------- Arrows ----------------

pub struct Arrow {
    pub pos: Vec3,
    pub prev_pos: Vec3,
    pub vel: Vec3,
    pub stuck: bool,
    pub age: i32,
    pub from_player: bool,
    pub damage: f32,
}

impl Arrow {
    pub fn yaw_pitch(&self) -> (f32, f32) {
        let v = self.vel;
        let h = (v.x * v.x + v.z * v.z).sqrt();
        ((-v.x).atan2(v.z).to_degrees(), (-v.y).atan2(h).to_degrees())
    }
}

// ---------------- Particles ----------------

#[derive(Clone, Copy)]
pub struct Particle {
    pub pos: Vec3,
    pub prev_pos: Vec3,
    pub vel: Vec3,
    pub age: i32,
    pub life: i32,
    pub size: f32,
    /// uv rect in block atlas (u0, v0, u1, v1); None = untextured
    pub uv: [f32; 4],
    pub color: [u8; 4],
    pub gravity: f32,
    pub textured: bool,
    pub emissive: bool,
}

impl Particle {
    pub fn tick(&mut self, world: &World) -> bool {
        self.prev_pos = self.pos;
        self.age += 1;
        if self.age >= self.life {
            return false;
        }
        self.vel.y -= self.gravity;
        // move one axis at a time against the particle's own extent, so chips
        // land and rest on top of surfaces instead of sinking into them
        let h = self.size.min(0.1);
        let solid = |p: Vec3| {
            let (x, y, z) = p.floor();
            block::is_solid(world.get(x, y, z))
        };
        let mut on_ground = false;
        let ny = self.pos.y + self.vel.y;
        let edge = ny + if self.vel.y < 0.0 { -h } else { h };
        if solid(v3(self.pos.x, edge, self.pos.z)) {
            if self.vel.y < 0.0 {
                on_ground = true;
                self.pos.y = edge.floor() + 1.0 + h;
            }
            self.vel.y = 0.0;
        } else {
            self.pos.y = ny;
        }
        let nx = self.pos.x + self.vel.x;
        if solid(v3(nx + h * self.vel.x.signum(), self.pos.y, self.pos.z)) {
            self.vel.x = 0.0;
        } else {
            self.pos.x = nx;
        }
        let nz = self.pos.z + self.vel.z;
        if solid(v3(self.pos.x, self.pos.y, nz + h * self.vel.z.signum())) {
            self.vel.z = 0.0;
        } else {
            self.pos.z = nz;
        }
        self.vel = self.vel * 0.98;
        if on_ground {
            self.vel.x *= 0.7;
            self.vel.z *= 0.7;
        }
        true
    }
}
