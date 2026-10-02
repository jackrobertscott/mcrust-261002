//! Axis-aligned collision physics shared by the player and mobs (vanilla-like).

use crate::block::{self, LAVA, WATER};
use crate::math::{v3, Vec3};
use crate::world::World;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }
    pub fn intersects(&self, o: &Aabb) -> bool {
        self.min.x < o.max.x && self.max.x > o.min.x && self.min.y < o.max.y && self.max.y > o.min.y && self.min.z < o.max.z && self.max.z > o.min.z
    }
    pub fn offset(&self, d: Vec3) -> Aabb {
        Aabb { min: self.min + d, max: self.max + d }
    }
    pub fn expand(&self, d: Vec3) -> Aabb {
        let mut a = *self;
        if d.x < 0.0 { a.min.x += d.x } else { a.max.x += d.x }
        if d.y < 0.0 { a.min.y += d.y } else { a.max.y += d.y }
        if d.z < 0.0 { a.min.z += d.z } else { a.max.z += d.z }
        a
    }
    pub fn grow(&self, g: f32) -> Aabb {
        Aabb { min: self.min - v3(g, g, g), max: self.max + v3(g, g, g) }
    }
    fn clip_x(&self, o: &Aabb, mut d: f32) -> f32 {
        if o.max.y <= self.min.y || o.min.y >= self.max.y || o.max.z <= self.min.z || o.min.z >= self.max.z {
            return d;
        }
        if d > 0.0 && o.max.x <= self.min.x {
            d = d.min(self.min.x - o.max.x);
        } else if d < 0.0 && o.min.x >= self.max.x {
            d = d.max(self.max.x - o.min.x);
        }
        d
    }
    fn clip_y(&self, o: &Aabb, mut d: f32) -> f32 {
        if o.max.x <= self.min.x || o.min.x >= self.max.x || o.max.z <= self.min.z || o.min.z >= self.max.z {
            return d;
        }
        if d > 0.0 && o.max.y <= self.min.y {
            d = d.min(self.min.y - o.max.y);
        } else if d < 0.0 && o.min.y >= self.max.y {
            d = d.max(self.max.y - o.min.y);
        }
        d
    }
    fn clip_z(&self, o: &Aabb, mut d: f32) -> f32 {
        if o.max.x <= self.min.x || o.min.x >= self.max.x || o.max.y <= self.min.y || o.min.y >= self.max.y {
            return d;
        }
        if d > 0.0 && o.max.z <= self.min.z {
            d = d.min(self.min.z - o.max.z);
        } else if d < 0.0 && o.min.z >= self.max.z {
            d = d.max(self.max.z - o.min.z);
        }
        d
    }
    /// Ray intersection; returns (t, face) where face is 0..6 like mesher normals.
    pub fn ray(&self, o: Vec3, d: Vec3) -> Option<(f32, usize)> {
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;
        let mut face = 0;
        let axes = [(o.x, d.x, self.min.x, self.max.x, 0usize), (o.y, d.y, self.min.y, self.max.y, 2), (o.z, d.z, self.min.z, self.max.z, 4)];
        for (oo, dd, mn, mx, f) in axes {
            if dd.abs() < 1e-9 {
                if oo < mn || oo > mx {
                    return None;
                }
                continue;
            }
            let t1 = (mn - oo) / dd;
            let t2 = (mx - oo) / dd;
            let (tn, tf, fn_) = if t1 < t2 { (t1, t2, f) } else { (t2, t1, f + 1) };
            if tn > tmin {
                tmin = tn;
                face = fn_;
            }
            tmax = tmax.min(tf);
        }
        if tmin > tmax || tmax < 0.0 {
            return None;
        }
        Some((tmin.max(0.0), face))
    }
}

pub fn block_boxes(world: &World, area: &Aabb) -> Vec<Aabb> {
    let mut out = Vec::new();
    let (x0, y0, z0) = (area.min.x.floor() as i32, area.min.y.floor() as i32 - 1, area.min.z.floor() as i32);
    let (x1, y1, z1) = (area.max.x.floor() as i32, area.max.y.floor() as i32, area.max.z.floor() as i32);
    for y in y0..=y1 {
        for z in z0..=z1 {
            for x in x0..=x1 {
                if !world.is_loaded(x, z) {
                    // treat unloaded chunks as solid walls so nothing falls out of the world
                    out.push(Aabb::new(v3(x as f32, y as f32, z as f32), v3(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0)));
                    continue;
                }
                let b = world.get(x, y, z);
                if let Some((mn, mx)) = block::collision_box(b) {
                    out.push(Aabb::new(
                        v3(x as f32 + mn[0], y as f32 + mn[1], z as f32 + mn[2]),
                        v3(x as f32 + mx[0], y as f32 + mx[1], z as f32 + mx[2]),
                    ));
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct Body {
    /// feet centre
    pub pos: Vec3,
    pub vel: Vec3,
    pub half_w: f32,
    pub height: f32,
    pub on_ground: bool,
    pub collided_h: bool,
    pub collided_v: bool,
    pub in_water: bool,
    pub in_lava: bool,
    pub fall_distance: f32,
    pub step_height: f32,
}

impl Body {
    pub fn new(pos: Vec3, half_w: f32, height: f32) -> Body {
        Body { pos, vel: Vec3::ZERO, half_w, height, on_ground: false, collided_h: false, collided_v: false, in_water: false, in_lava: false, fall_distance: 0.0, step_height: 0.6 }
    }
    pub fn aabb(&self) -> Aabb {
        Aabb::new(v3(self.pos.x - self.half_w, self.pos.y, self.pos.z - self.half_w), v3(self.pos.x + self.half_w, self.pos.y + self.height, self.pos.z + self.half_w))
    }

    /// Move with collision. `sneak` prevents walking off edges. Returns fall distance on landing.
    pub fn move_by(&mut self, world: &World, mut d: Vec3, sneak: bool) -> Option<f32> {
        let orig = d;
        let bb = self.aabb();
        // Sneaking: don't step off edges
        if sneak && self.on_ground {
            let step = 0.05;
            let has_floor = |dx: f32, dz: f32| -> bool {
                let t = bb.offset(v3(dx, -0.6, dz));
                block_boxes(world, &t).iter().any(|b| b.intersects(&t))
            };
            while d.x != 0.0 && !has_floor(d.x, 0.0) {
                d.x = if d.x.abs() < step { 0.0 } else { d.x - step * d.x.signum() };
            }
            while d.z != 0.0 && !has_floor(0.0, d.z) {
                d.z = if d.z.abs() < step { 0.0 } else { d.z - step * d.z.signum() };
            }
            while d.x != 0.0 && d.z != 0.0 && !has_floor(d.x, d.z) {
                d.x = if d.x.abs() < step { 0.0 } else { d.x - step * d.x.signum() };
                d.z = if d.z.abs() < step { 0.0 } else { d.z - step * d.z.signum() };
            }
        }
        let wanted = d;
        let boxes = block_boxes(world, &bb.expand(d).grow(0.01));
        let mut b = bb;
        let mut dy = d.y;
        for o in &boxes {
            dy = o.clip_y(&b, dy);
        }
        b = b.offset(v3(0.0, dy, 0.0));
        let mut dx = d.x;
        for o in &boxes {
            dx = o.clip_x(&b, dx);
        }
        b = b.offset(v3(dx, 0.0, 0.0));
        let mut dz = d.z;
        for o in &boxes {
            dz = o.clip_z(&b, dz);
        }
        b = b.offset(v3(0.0, 0.0, dz));

        // Step up (e.g. onto slabs/farmland) if horizontal movement was blocked
        let on_ground_now = self.on_ground || (dy != wanted.y && wanted.y < 0.0);
        if self.step_height > 0.0 && on_ground_now && (dx != wanted.x || dz != wanted.z) {
            let boxes2 = block_boxes(world, &bb.expand(v3(wanted.x, self.step_height, wanted.z)).grow(0.01));
            let mut b2 = bb;
            let mut sy = self.step_height;
            for o in &boxes2 {
                sy = o.clip_y(&b2, sy);
            }
            b2 = b2.offset(v3(0.0, sy, 0.0));
            let mut sx = wanted.x;
            for o in &boxes2 {
                sx = o.clip_x(&b2, sx);
            }
            b2 = b2.offset(v3(sx, 0.0, 0.0));
            let mut sz = wanted.z;
            for o in &boxes2 {
                sz = o.clip_z(&b2, sz);
            }
            b2 = b2.offset(v3(0.0, 0.0, sz));
            let mut down = -sy;
            for o in &boxes2 {
                down = o.clip_y(&b2, down);
            }
            b2 = b2.offset(v3(0.0, down, 0.0));
            if sx * sx + sz * sz > dx * dx + dz * dz + 1e-6 {
                b = b2;
                dx = sx;
                dz = sz;
                dy = b2.min.y - bb.min.y;
            }
        }

        self.pos = v3((b.min.x + b.max.x) / 2.0, b.min.y, (b.min.z + b.max.z) / 2.0);
        self.collided_h = dx != wanted.x || dz != wanted.z;
        self.collided_v = dy != wanted.y;
        let was_on_ground = self.on_ground;
        self.on_ground = wanted.y < 0.0 && dy != wanted.y;
        if dx != wanted.x {
            self.vel.x = 0.0;
        }
        if dz != wanted.z {
            self.vel.z = 0.0;
        }
        if dy != wanted.y {
            self.vel.y = 0.0;
        }
        let _ = orig;
        // fall distance tracking
        let mut landed = None;
        if self.on_ground {
            if !was_on_ground || self.fall_distance > 0.0 {
                landed = Some(self.fall_distance);
            }
            self.fall_distance = 0.0;
        } else if dy < 0.0 {
            self.fall_distance -= dy;
        }
        self.update_fluids(world);
        if self.in_water {
            self.fall_distance = 0.0;
        }
        landed
    }

    pub fn update_fluids(&mut self, world: &World) {
        let bb = self.aabb().grow(-0.001);
        let (x0, y0, z0) = (bb.min.x.floor() as i32, (bb.min.y + 0.4).floor() as i32, bb.min.z.floor() as i32);
        let (x1, y1, z1) = (bb.max.x.floor() as i32, (bb.max.y - 0.4).floor() as i32, bb.max.z.floor() as i32);
        self.in_water = false;
        self.in_lava = false;
        for y in y0..=y1.max(y0) {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    match world.get(x, y, z) {
                        WATER => self.in_water = true,
                        LAVA => self.in_lava = true,
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Result of a block ray cast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: usize,
    pub dist: f32,
}

/// Cast a ray through the world against block selection boxes (ignores fluids).
pub fn raycast(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
    let d = dir.norm();
    let (mut x, mut y, mut z) = origin.floor();
    let step = |v: f32| if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 };
    let (sx, sy, sz) = (step(d.x), step(d.y), step(d.z));
    let next = |p: f32, i: i32, s: i32| -> f32 { if s > 0 { (i + 1) as f32 - p } else { p - i as f32 } };
    let inv = |v: f32| if v.abs() < 1e-9 { f32::INFINITY } else { 1.0 / v.abs() };
    let (dtx, dty, dtz) = (inv(d.x), inv(d.y), inv(d.z));
    let mut tx = if sx != 0 { next(origin.x, x, sx) * dtx } else { f32::INFINITY };
    let mut ty = if sy != 0 { next(origin.y, y, sy) * dty } else { f32::INFINITY };
    let mut tz = if sz != 0 { next(origin.z, z, sz) * dtz } else { f32::INFINITY };
    let mut t = 0.0;
    for _ in 0..256 {
        if t > max {
            break;
        }
        let b = world.get(x, y, z);
        if b != block::AIR && b != WATER && b != LAVA {
            let (mn, mx) = block::selection_box(b, world.get_meta(x, y, z));
            let bb = Aabb::new(v3(x as f32 + mn[0], y as f32 + mn[1], z as f32 + mn[2]), v3(x as f32 + mx[0], y as f32 + mx[1], z as f32 + mx[2]));
            if let Some((th, face)) = bb.ray(origin, d) {
                if th <= max {
                    return Some(Hit { x, y, z, face, dist: th });
                }
            }
        }
        if tx < ty && tx < tz {
            t = tx;
            x += sx;
            tx += dtx;
        } else if ty < tz {
            t = ty;
            y += sy;
            ty += dty;
        } else {
            t = tz;
            z += sz;
            tz += dtz;
        }
    }
    None
}
