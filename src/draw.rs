//! Drawing the game scene: camera, sky, chunks, entities, held item, particles and the HUD.

use std::collections::HashMap;

use crate::block::{self, *};
use crate::entity::{MobKind, Particle};
use crate::game::{Game, Menu, Target};
use crate::gl::{self, Vertex};
use crate::item::{self, ItemId};
use crate::math::{lerp, v3, Mat4, Vec3};
use crate::models::{self, ModelKind, Pose, Shade};
use crate::render::Renderer;
use crate::ui::{Ui, WHITE};
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Atlas {
    Blocks,
    Items,
}

/// Cached geometry for items (extruded sprites / cubes), in a unit box.
pub struct ItemModels {
    cache: HashMap<ItemId, (Vec<Vertex>, Atlas)>,
}

impl ItemModels {
    pub fn new() -> ItemModels {
        ItemModels { cache: HashMap::new() }
    }

    pub fn get(&mut self, r: &Renderer, id: ItemId) -> &(Vec<Vertex>, Atlas) {
        if !self.cache.contains_key(&id) {
            let m = build_item_model(r, id, None);
            self.cache.insert(id, m);
        }
        &self.cache[&id]
    }

    /// Model for a specific item texture (e.g. bow pulling stages), cached under a pseudo id.
    pub fn get_tex(&mut self, r: &Renderer, key: ItemId, tex: &'static str) -> &(Vec<Vertex>, Atlas) {
        if !self.cache.contains_key(&key) {
            let t = if r.item_tile.contains_key(tex) { Some(tex) } else { None };
            let m = build_item_model(r, item::BOW, t);
            self.cache.insert(key, m);
        }
        &self.cache[&key]
    }
}

/// Is this item drawn as a 3D cube (vs a flat sprite)?
pub fn is_cube_item(id: ItemId) -> bool {
    item::texture(id).is_none() && id < 256
}

fn build_item_model(r: &Renderer, id: ItemId, tex_override: Option<&str>) -> (Vec<Vertex>, Atlas) {
    if is_cube_item(id) && tex_override.is_none() {
        return (cube_model(r, id as u8), Atlas::Blocks);
    }
    let tex = tex_override.unwrap_or_else(|| item::texture(id).unwrap_or("stick"));
    let (img, atlas, (u0, v0, s)) = if let Some(bn) = tex.strip_prefix("b:") {
        let t = *r.block_tile.get(bn).unwrap_or(&0);
        (&r.block_atlas_img, Atlas::Blocks, r.tiles.uv(t))
    } else {
        let t = *r.item_tile.get(tex).unwrap_or(&0);
        (&r.item_atlas_img, Atlas::Items, r.tiles_uv_items(t))
    };
    let px0 = (u0 * img.w as f32).round() as usize;
    let py0 = (v0 * img.h as f32).round() as usize;
    let opaque = |x: i32, y: i32| -> bool {
        if !(0..16).contains(&x) || !(0..16).contains(&y) {
            return false;
        }
        img.get(px0 + x as usize, py0 + y as usize)[3] > 16
    };
    let mut v = Vec::new();
    let t = 1.0 / 32.0; // half thickness
    let p = 1.0 / 16.0;
    let mk = |pos: [f32; 3], uv: [f32; 2], shade: u8| Vertex { pos, uv, color: [255; 4], light: [255, 255, shade, 0] };
    let quad = |v: &mut Vec<Vertex>, q: [[f32; 3]; 4], uv: [[f32; 2]; 4], shade: u8| {
        let a = [mk(q[0], uv[0], shade), mk(q[1], uv[1], shade), mk(q[2], uv[2], shade), mk(q[3], uv[3], shade)];
        v.extend_from_slice(&[a[0], a[1], a[2], a[0], a[2], a[3]]);
    };
    // front (+z) and back (-z)
    quad(&mut v, [[0.0, 0.0, t], [1.0, 0.0, t], [1.0, 1.0, t], [0.0, 1.0, t]], [[u0, v0 + s], [u0 + s, v0 + s], [u0 + s, v0], [u0, v0]], 255);
    quad(&mut v, [[1.0, 0.0, -t], [0.0, 0.0, -t], [0.0, 1.0, -t], [1.0, 1.0, -t]], [[u0 + s, v0 + s], [u0, v0 + s], [u0, v0], [u0 + s, v0]], 200);
    // edges per pixel
    for y in 0..16i32 {
        for x in 0..16i32 {
            if !opaque(x, y) {
                continue;
            }
            let (fx0, fx1) = (x as f32 * p, (x + 1) as f32 * p);
            let (fy1, fy0) = (1.0 - y as f32 * p, 1.0 - (y + 1) as f32 * p);
            let uc = u0 + (x as f32 + 0.5) / 16.0 * s;
            let vc = v0 + (y as f32 + 0.5) / 16.0 * s;
            let uv = [[uc, vc]; 4];
            if !opaque(x - 1, y) {
                quad(&mut v, [[fx0, fy0, -t], [fx0, fy0, t], [fx0, fy1, t], [fx0, fy1, -t]], uv, 150);
            }
            if !opaque(x + 1, y) {
                quad(&mut v, [[fx1, fy0, t], [fx1, fy0, -t], [fx1, fy1, -t], [fx1, fy1, t]], uv, 150);
            }
            if !opaque(x, y - 1) {
                quad(&mut v, [[fx0, fy1, t], [fx1, fy1, t], [fx1, fy1, -t], [fx0, fy1, -t]], uv, 230);
            }
            if !opaque(x, y + 1) {
                quad(&mut v, [[fx0, fy0, -t], [fx1, fy0, -t], [fx1, fy0, t], [fx0, fy0, t]], uv, 120);
            }
        }
    }
    (v, atlas)
}

/// Unit cube with block textures (for held / dropped blocks).
pub fn cube_model(r: &Renderer, b: u8) -> Vec<Vertex> {
    let d = block::def(b);
    let tiles = r.tiles.blocks[b as usize];
    let tint = match d.tint {
        Tint::Grass => 0x91BD59,
        Tint::Foliage => 0x77AB2F,
        Tint::Fixed(c) => c,
        Tint::None => 0xFFFFFF,
    };
    let tc = [(tint >> 16) as u8, (tint >> 8) as u8, tint as u8, 255];
    let h = match d.shape {
        Shape::Layer => 2.0 / 16.0,
        _ => 1.0,
    };
    let faces: [([[f32; 3]; 4], u16, u8); 6] = [
        ([[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, h, 1.0], [0.0, h, 0.0]], tiles[2], 153),
        ([[1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, h, 0.0], [1.0, h, 1.0]], tiles[2], 153),
        ([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], tiles[1], 128),
        ([[0.0, h, 1.0], [1.0, h, 1.0], [1.0, h, 0.0], [0.0, h, 0.0]], tiles[0], 255),
        ([[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, h, 0.0], [1.0, h, 0.0]], tiles[3], 204),
        ([[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, h, 1.0], [0.0, h, 1.0]], tiles[2], 204),
    ];
    let mut v = Vec::new();
    for (i, (q, tile, shade)) in faces.iter().enumerate() {
        let (u0, v0, s) = r.tiles.uv(*tile);
        let vt = if i == 2 || i == 3 { 0.0 } else { (1.0 - h) * s };
        let uv = [[u0, v0 + s], [u0 + s, v0 + s], [u0 + s, v0 + vt], [u0, v0 + vt]];
        let col = if d.tint != Tint::None && (b != GRASS || i == 3) { tc } else { [255; 4] };
        let mk = |k: usize| Vertex { pos: q[k], uv: uv[k], color: col, light: [255, 255, *shade, 0] };
        v.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
        if b == GRASS && i != 2 && i != 3 {
            let (u0, v0, s) = r.tiles.uv(r.tiles.grass_overlay);
            let uv = [[u0, v0 + s], [u0 + s, v0 + s], [u0 + s, v0], [u0, v0]];
            let mk = |k: usize| Vertex { pos: q[k], uv: uv[k], color: tc, light: [255, 255, *shade, 0] };
            v.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
        }
    }
    v
}

fn transform_verts(src: &[Vertex], m: &Mat4, sky: u8, blk: u8, out: &mut Vec<Vertex>) {
    for vt in src {
        let p = m.transform(v3(vt.pos[0], vt.pos[1], vt.pos[2]));
        out.push(Vertex { pos: [p.x, p.y, p.z], uv: vt.uv, color: vt.color, light: [sky, blk, vt.light[2], 0] });
    }
}

pub struct Camera {
    pub pos: Vec3,
    pub view: Mat4,
    pub proj: Mat4,
    pub yaw: f32,
    pub pitch: f32,
}

pub struct Scene {
    pub items: ItemModels,
}

fn sky_colors(g: &Game, cam_pos: Vec3) -> ([f32; 3], [f32; 3], Option<[f32; 4]>, f32) {
    let b = g.sky_brightness();
    let biome = g.world.biome(cam_pos.x.floor() as i32, cam_pos.z.floor() as i32);
    let base = match biome {
        crate::worldgen::Biome::Desert | crate::worldgen::Biome::Savanna => [0.43, 0.63, 1.0],
        b if b.is_snowy() => [0.50, 0.66, 1.0],
        _ => [0.47, 0.65, 1.0],
    };
    let sky = [base[0] * b, base[1] * b, base[2] * b];
    let fb = b * 0.94 + 0.06;
    let mut fog = [0.75 * fb, 0.85 * fb, 1.0 * fb];
    // sunrise / sunset
    let a = g.celestial();
    let c = (a * std::f32::consts::TAU).cos();
    let mut sunrise = None;
    if c > -0.4 && c < 0.4 {
        let f = c / 0.4 * 0.5 + 0.5;
        let mut alpha = 1.0 - (1.0 - (f * std::f32::consts::PI).sin()) * 0.99;
        alpha *= alpha;
        let col = [f * 0.3 + 0.7, f * f * 0.7 + 0.2, 0.2, alpha];
        // blend fog toward sunrise colour (as vanilla does when looking at the sun)
        let k = alpha * 0.4;
        fog = [fog[0] * (1.0 - k) + col[0] * k, fog[1] * (1.0 - k) + col[1] * k, fog[2] * (1.0 - k) + col[2] * k];
        sunrise = Some(col);
    }
    let star = ((1.0 - ((a * std::f32::consts::TAU).cos() * 2.0 + 0.25).clamp(0.0, 1.0)).powi(2)) * 0.5 * (1.0 - g.rain);
    let mut sky = sky;
    if g.rain > 0.0 {
        let r = g.rain;
        let gray = (sky[0] * 0.3 + sky[1] * 0.59 + sky[2] * 0.11) * 0.6;
        for c in sky.iter_mut() {
            *c = *c * (1.0 - r * 0.75) + gray * r * 0.75;
        }
        fog = [fog[0] * (1.0 - r * 0.5), fog[1] * (1.0 - r * 0.5), fog[2] * (1.0 - r * 0.4)];
    }
    let sunrise = if g.rain > 0.5 { None } else { sunrise };
    (sky, fog, sunrise, star)
}

impl Scene {
    pub fn new() -> Scene {
        Scene { items: ItemModels::new() }
    }

    pub fn camera(&self, g: &Game, aspect: f32, alpha: f32, panorama: bool) -> Camera {
        let p = &g.player;
        let pos = p.prev_pos.lerp(p.body.pos, alpha);
        let mut eye = pos + v3(0.0, p.eye_height(), 0.0);
        let mut yaw = p.yaw;
        let mut pitch = p.pitch;
        if panorama {
            yaw = g.panorama_angle;
            pitch = 12.0 + (g.panorama_angle * 0.05).sin() * 4.0;
        }
        let mut fov = g.options.fov;
        if p.sprinting && !panorama {
            fov *= 1.1;
        }
        if p.body.in_water && !panorama {
            fov *= 0.85;
        }
        let mut view = Mat4::identity();
        if !panorama && g.third_person == 0 {
            // hurt tilt
            let ht = p.hurt_time as f32 - alpha;
            if ht > 0.0 {
                let f = (ht / 10.0).powi(4);
                view = Mat4::rot_z((-(f * std::f32::consts::PI).sin() * 14.0).to_radians()) * view;
            }
            if p.dead {
                view = Mat4::rot_z(40.0f32.to_radians() * (p.death_time as f32 / 20.0).min(1.0)) * view;
            }
            if g.options.view_bobbing {
                let wd = lerp(p.prev_walk_dist, p.walk_dist, alpha);
                let bob = lerp(p.prev_bob, p.bob, alpha);
                let f = wd * std::f32::consts::PI;
                view = view
                    * Mat4::translate((f.sin() * bob * 0.5) as f32, -(f.cos() * bob).abs(), 0.0)
                    * Mat4::rot_z((f.sin() * bob * 3.0).to_radians())
                    * Mat4::rot_x(((f - 0.2).cos() * bob).abs().mul_add(5.0, 0.0).to_radians());
            }
        }
        if g.screen_shake > 0.0 {
            let s = g.screen_shake * 0.05;
            view = Mat4::translate((g.tick_count as f32 * 3.7).sin() * s, (g.tick_count as f32 * 5.3).cos() * s, 0.0) * view;
        }
        if g.third_person > 0 && !panorama {
            let back = if g.third_person == 1 { -1.0 } else { 1.0 };
            let (yr, pr) = (yaw.to_radians(), pitch.to_radians());
            let dir = v3(-yr.sin() * pr.cos(), -pr.sin(), yr.cos() * pr.cos()) * back;
            // pull back, avoiding walls
            let mut dist = 4.0;
            if let Some(h) = crate::physics::raycast(&g.world, eye, dir, 4.0) {
                dist = (h.dist - 0.2).max(0.5);
            }
            eye = eye + dir * dist;
            if g.third_person == 2 {
                yaw += 180.0;
                pitch = -pitch;
            }
        }
        let view = view * Mat4::rot_x(pitch.to_radians()) * Mat4::rot_y((yaw + 180.0).to_radians());
        let proj = Mat4::perspective(fov.to_radians(), aspect, 0.05, 1000.0);
        Camera { pos: eye, view, proj, yaw, pitch }
    }

    /// Render the 3D world (used both in-game and for the title panorama).
    pub fn draw_world(&mut self, g: &Game, r: &mut Renderer, alpha: f32, panorama: bool) {
        let aspect = r.width as f32 / r.height.max(1) as f32;
        let cam = self.camera(g, aspect, alpha, panorama);
        let (sky, mut fog, sunrise, star) = sky_colors(g, cam.pos);
        let rd = (g.options.render_distance * 16) as f32;
        let mut fog_range = (rd * 0.55, rd * 0.95);
        let (cx, cy, cz) = cam.pos.floor();
        let cam_block = g.world.get(cx, cy, cz);
        let underwater = cam_block == WATER;
        let in_lava = cam_block == LAVA;
        if underwater {
            let b = g.sky_brightness() * 0.8 + 0.2;
            fog = [0.02 * b, 0.1 * b, 0.45 * b];
            fog_range = (0.0, 24.0);
        } else if in_lava {
            fog = [0.6, 0.1, 0.0];
            fog_range = (0.0, 1.5);
        }
        unsafe {
            gl::glViewport(0, 0, r.width as i32, r.height as i32);
            gl::glClearColor(fog[0], fog[1], fog[2], 1.0);
            gl::glClear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::glEnable(gl::DEPTH_TEST);
            gl::glDepthFunc(gl::LEQUAL);
            gl::glEnable(gl::CULL_FACE);
            gl::glCullFace(gl::BACK);
            gl::glFrontFace(gl::CCW);
        }
        let mvp = cam.proj * cam.view;
        if !underwater && !in_lava {
            let celestial = g.celestial() * std::f32::consts::TAU;
            r.draw_sky(&mvp, sky, fog, celestial, star, sunrise);
        }
        let skydark = g.skydark();
        r.setup_world_shader(&mvp, fog, fog_range, skydark);
        r.draw_chunks(cam.pos, &mvp, false);

        // ---- entities ----
        self.draw_entities(g, r, &cam, &mvp, alpha, panorama);
        r.setup_world_shader(&mvp, fog, fog_range, skydark);

        // ---- block outline & cracks ----
        if !panorama && g.menu != Menu::Death && !g.hide_hud {
            if let Target::Block(h) = g.target {
                self.draw_selection(g, r, &cam, &mvp, h.x, h.y, h.z);
            }
        }
        // ---- particles ----
        self.draw_particles(g, r, &cam, &mvp, alpha, fog, fog_range, skydark);

        // ---- clouds ----
        if g.options.clouds && !underwater {
            let b = g.sky_brightness();
            let mut cc = [b * 0.9 + 0.1, b * 0.9 + 0.1, b * 0.85 + 0.15];
            if g.rain > 0.0 {
                let gray = (cc[0] * 0.3 + cc[1] * 0.59 + cc[2] * 0.11) * 0.6;
                for c in cc.iter_mut() {
                    *c = *c * (1.0 - g.rain * 0.95) + gray * g.rain * 0.95;
                }
            }
            let time = g.world.time as f64 + alpha as f64;
            r.draw_clouds(cam.pos, &mvp, time, cc, rd);
        }
        // ---- translucent ----
        r.setup_world_shader(&mvp, fog, fog_range, skydark);
        unsafe {
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::glDisable(gl::CULL_FACE);
        }
        r.world_shader.set_f("u_alpha_ref", 0.01);
        r.draw_chunks(cam.pos, &mvp, true);
        unsafe {
            gl::glDisable(gl::BLEND);
            gl::glEnable(gl::CULL_FACE);
        }

        // ---- rain / snow ----
        if !panorama || g.rain > 0.0 {
            self.draw_weather(g, r, &cam, &mvp, alpha, fog, fog_range, skydark);
        }

        // ---- first person hand ----
        if !panorama && g.third_person == 0 && !g.hide_hud && !g.player.dead {
            self.draw_hand(g, r, alpha, aspect, fog);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_weather(&mut self, g: &Game, r: &mut Renderer, cam: &Camera, mvp: &Mat4, alpha: f32, fog: [f32; 3], fog_range: (f32, f32), skydark: f32) {
        let strength = lerp(g.prev_rain, g.rain, alpha);
        if strength <= 0.0 {
            return;
        }
        let (cx, cy, cz) = cam.pos.floor();
        let radius = 10;
        let t = (g.world.time as f32 + alpha) / 20.0;
        let mut rain_v: Vec<Vertex> = Vec::new();
        let mut snow_v: Vec<Vertex> = Vec::new();
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dz * dz;
                if d2 > radius * radius {
                    continue;
                }
                let (x, z) = (cx + dx, cz + dz);
                if !g.world.is_loaded(x, z) {
                    continue;
                }
                let biome = g.world.biome(x, z);
                if matches!(biome, crate::worldgen::Biome::Desert | crate::worldgen::Biome::Savanna) {
                    continue;
                }
                let snow = biome.is_snowy() || (biome == crate::worldgen::Biome::Mountains && cy > 100);
                let ground = g.world.surface_height(x, z);
                let y0 = ground.max(cy - radius);
                let y1 = (cy + radius).max(ground);
                if y0 >= y1 {
                    continue;
                }
                let centre = v3(x as f32 + 0.5 - cam.pos.x, 0.0, z as f32 + 0.5 - cam.pos.z);
                let len = (centre.x * centre.x + centre.z * centre.z).sqrt().max(0.01);
                // quad perpendicular to the view direction, 1 block wide
                let (px, pz) = (-centre.z / len * 0.5, centre.x / len * 0.5);
                let dist = len / radius as f32;
                let a = ((1.0 - dist * dist) * 0.5 + 0.5) * strength * 0.8;
                let l = g.world.light(x, y0.max(cy), z);
                let light = [(l >> 4) * 17, (l & 15) * 17, 255, 0];
                let (fy0, fy1) = (y0 as f32 - cam.pos.y, y1 as f32 - cam.pos.y);
                let hash = ((x.wrapping_mul(3121) ^ z.wrapping_mul(45238971)) & 255) as f32 / 255.0;
                let (scroll, uscroll, tile_h, out) = if snow {
                    (t * 0.25 + hash, (t * 0.03 + hash).sin() * 0.3, 4.0, &mut snow_v)
                } else {
                    (t * 2.2 + hash * 3.0, hash, 8.0, &mut rain_v)
                };
                let v_top = fy1 / tile_h + scroll;
                let v_bot = fy0 / tile_h + scroll;
                let col = [255, 255, 255, (a * 255.0) as u8];
                let mk = |p: [f32; 3], u: f32, v: f32| Vertex { pos: p, uv: [u, -v], color: col, light };
                let (ax, az) = (centre.x - px, centre.z - pz);
                let (bx, bz) = (centre.x + px, centre.z + pz);
                let q = [
                    mk([ax, fy0, az], uscroll, v_bot),
                    mk([bx, fy0, bz], uscroll + 1.0, v_bot),
                    mk([bx, fy1, bz], uscroll + 1.0, v_top),
                    mk([ax, fy1, az], uscroll, v_top),
                ];
                out.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
            }
        }
        r.setup_world_shader(mvp, fog, fog_range, skydark);
        r.world_shader.set_f("u_alpha_ref", 0.01);
        unsafe {
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::glDisable(gl::CULL_FACE);
            gl::glDepthMask(0);
        }
        r.rain_tex.bind();
        r.draw_stream(&rain_v);
        r.snow_tex.bind();
        r.draw_stream(&snow_v);
        unsafe {
            gl::glDepthMask(1);
            gl::glEnable(gl::CULL_FACE);
            gl::glDisable(gl::BLEND);
        }
    }

    fn light_at(w: &World, p: Vec3) -> (u8, u8) {
        let (x, y, z) = p.floor();
        let mut l = w.light(x, y, z);
        if block::is_opaque(w.get(x, y, z)) {
            l = w.light(x, y + 1, z);
        }
        ((l >> 4) * 17, (l & 15) * 17)
    }

    fn draw_entities(&mut self, g: &Game, r: &mut Renderer, cam: &Camera, mvp: &Mat4, alpha: f32, panorama: bool) {
        let s = r.world_shader;
        s.bind();
        s.set_mat4("u_mvp", &mvp.0);
        unsafe { gl::glDisable(gl::CULL_FACE) };
        let mut verts: Vec<Vertex> = Vec::new();
        let frustum = crate::math::Frustum::from_matrix(mvp);
        // ---- mobs ----
        for m in &g.mobs {
            let pos = m.prev_pos.lerp(m.body.pos, alpha);
            let rel = pos - cam.pos;
            let bb_min = rel - v3(m.body.half_w + 1.0, 0.5, m.body.half_w + 1.0);
            let bb_max = rel + v3(m.body.half_w + 1.0, m.body.height + 0.5, m.body.half_w + 1.0);
            if !frustum.aabb_visible(bb_min, bb_max) || rel.len() > 128.0 {
                continue;
            }
            let (sky, blk) = Self::light_at(&g.world, pos + v3(0.0, m.body.height * 0.5, 0.0));
            let yaw = lerp_angle(m.prev_yaw, m.yaw, alpha);
            let kind = m.kind.model();
            let mut parts = models::build(kind);
            let la = lerp(m.prev_limb_amount, m.limb_amount, alpha);
            let pose = Pose {
                limb_swing: m.limb_swing - m.limb_amount * (1.0 - alpha),
                limb_amount: la,
                head_yaw: crate::math::wrap_deg(m.head_yaw - yaw).to_radians() * -1.0,
                head_pitch: m.pitch.to_radians(),
                age: m.age as f32 + alpha,
                arms_forward: m.kind == MobKind::Zombie || (m.kind == MobKind::Skeleton && m.attack_cooldown > 0) || (m.kind == MobKind::Chicken && !m.body.on_ground),
                swing: m.swing,
                sneak: false,
            };
            models::animate(kind, &mut parts, &pose);
            let mut scale = 1.0;
            let mut root = models::entity_root(rel, yaw, 1.0);
            let mut flash = [0.0f32; 4];
            if m.kind == MobKind::Creeper && m.fuse > 0 {
                let f = (lerp(m.prev_fuse as f32, m.fuse as f32, alpha) / 28.0).clamp(0.0, 1.0);
                let wobble = 1.0 + (f * 100.0).sin() * f * 0.01;
                let ff = f * f * f * f;
                let sx = (1.0 + ff * 0.4) * wobble;
                let sy = (1.0 + ff * 0.1) / wobble;
                root = Mat4::translate(rel.x, rel.y, rel.z) * Mat4::scale(sx, sy, sx) * Mat4::translate(-rel.x, -rel.y, -rel.z) * root;
                if ((f * 10.0) as i32) % 2 == 1 {
                    flash = [1.0, 1.0, 1.0, 0.5];
                }
                scale = sx;
            }
            let _ = scale;
            if !m.alive() {
                let f = ((m.death_time as f32 + alpha - 1.0) / 20.0 * 1.6).max(0.0).sqrt().min(1.0);
                root = Mat4::translate(rel.x, rel.y, rel.z)
                    * Mat4::rot_y((180.0 - yaw).to_radians())
                    * Mat4::rot_z(-(f * 90.0f32).to_radians())
                    * Mat4::rot_y(-(180.0 - yaw).to_radians())
                    * Mat4::translate(-rel.x, -rel.y, -rel.z)
                    * root;
            }
            if m.hurt_time > 0 || !m.alive() {
                flash = [1.0, 0.0, 0.0, 0.35];
            }
            let shade = Shade { sky, block: blk, color: [255; 4] };
            verts.clear();
            models::emit(&mut verts, &parts, &root, models::texture_size(kind), shade);
            if let Some(t) = r.skins.get(m.kind.skin()) {
                t.bind();
            }
            s.set_v4("u_flash", flash);
            r.draw_stream(&verts);
            if m.kind == MobKind::Sheep && !m.sheared {
                let mut fur = models::build(ModelKind::SheepFur);
                models::animate(ModelKind::SheepFur, &mut fur, &pose);
                verts.clear();
                models::emit(&mut verts, &fur, &root, (64.0, 32.0), shade);
                if let Some(t) = r.skins.get("sheep_fur") {
                    t.bind();
                }
                r.draw_stream(&verts);
            }
            // burning: flame particles drawn as overlay quads are skipped; tint orange
        }
        s.set_v4("u_flash", [0.0; 4]);

        // ---- player in third person ----
        if !panorama && g.third_person > 0 {
            let p = &g.player;
            let pos = p.prev_pos.lerp(p.body.pos, alpha);
            let rel = pos - cam.pos;
            let (sky, blk) = Self::light_at(&g.world, pos + v3(0.0, 1.0, 0.0));
            let mut parts = models::build(ModelKind::Biped);
            let sw = if p.swinging { (lerp(p.prev_swing, p.swing, alpha)).max(0.0) } else { 0.0 };
            let pose = Pose {
                limb_swing: p.limb_swing,
                limb_amount: p.limb_amount,
                head_yaw: -crate::math::wrap_deg(p.yaw - p.body_yaw).to_radians(),
                head_pitch: p.pitch.to_radians(),
                age: g.tick_count as f32,
                arms_forward: false,
                swing: sw,
                sneak: p.sneaking,
            };
            models::animate(ModelKind::Biped, &mut parts, &pose);
            let root = models::entity_root(rel - v3(0.0, if p.sneaking { 0.125 } else { 0.0 }, 0.0), p.body_yaw, 0.9375);
            verts.clear();
            models::emit(&mut verts, &parts, &root, (64.0, 64.0), Shade { sky, block: blk, color: [255; 4] });
            if let Some(t) = r.skins.get("steve") {
                t.bind();
            }
            if p.hurt_time > 0 {
                s.set_v4("u_flash", [1.0, 0.0, 0.0, 0.35]);
            }
            r.draw_stream(&verts);
            s.set_v4("u_flash", [0.0; 4]);
            // held item in right hand
            let held = p.inv.held();
            if !held.is_empty() {
                let arm = &parts[2];
                let arm_m = root
                    * Mat4::translate(arm.pivot[0] / 16.0, arm.pivot[1] / 16.0, arm.pivot[2] / 16.0)
                    * Mat4::rot_z(arm.rot[2])
                    * Mat4::rot_y(arm.rot[1])
                    * Mat4::rot_x(arm.rot[0]);
                let cube = is_cube_item(held.id);
                let m = if cube {
                    arm_m * Mat4::translate(-1.0 / 16.0, 10.0 / 16.0, -2.0 / 16.0) * Mat4::rot_x(-0.4) * Mat4::rot_y(0.785) * Mat4::scale(0.375, -0.375, 0.375) * Mat4::translate(-0.5, -0.5, -0.5)
                } else {
                    arm_m * Mat4::translate(-1.0 / 16.0, 10.0 / 16.0, -1.0 / 16.0) * Mat4::rot_x(-1.5708) * Mat4::rot_y(1.5708) * Mat4::rot_z(0.785) * Mat4::scale(0.6, -0.6, 0.6) * Mat4::translate(-0.5, -0.2, 0.0)
                };
                let (mv, atlas) = self.items.get(r, held.id).clone();
                verts.clear();
                transform_verts(&mv, &m, sky, blk, &mut verts);
                if atlas == Atlas::Blocks { r.block_atlas.bind() } else { r.item_atlas.bind() }
                r.draw_stream(&verts);
            }
        }

        // ---- dropped items ----
        let mut by_atlas: [Vec<Vertex>; 2] = [Vec::new(), Vec::new()];
        for it in &g.items {
            let pos = it.prev_pos.lerp(it.body.pos, alpha);
            let rel = pos - cam.pos;
            if rel.len() > 64.0 {
                continue;
            }
            let (sky, blk) = Self::light_at(&g.world, pos + v3(0.0, 0.25, 0.0));
            let t = it.age as f32 + alpha;
            let bob = ((t / 10.0 + it.bob).sin() * 0.1 + 0.1).max(0.0);
            let spin = (t / 20.0 + it.bob) * 57.3;
            let cube = is_cube_item(it.stack.id);
            let n = if it.stack.count > 32 { 4 } else if it.stack.count > 16 { 3 } else if it.stack.count > 1 { 2 } else { 1 };
            let (mv, atlas) = self.items.get(r, it.stack.id).clone();
            for k in 0..n {
                let off = v3(k as f32 * 0.05 - 0.03, k as f32 * 0.04, k as f32 * 0.03);
                let m = if cube {
                    let sc = 0.25;
                    Mat4::translate(rel.x + off.x, rel.y + bob + 0.125 + off.y, rel.z + off.z) * Mat4::rot_y(spin.to_radians()) * Mat4::scale(sc, sc, sc) * Mat4::translate(-0.5, -0.5, -0.5)
                } else {
                    let sc = 0.5;
                    Mat4::translate(rel.x + off.x * 0.5, rel.y + bob + 0.25 + off.y * 0.3, rel.z + off.z * 2.0) * Mat4::rot_y(spin.to_radians()) * Mat4::scale(sc, sc, sc) * Mat4::translate(-0.5, -0.5, 0.0)
                };
                let i = if atlas == Atlas::Blocks { 0 } else { 1 };
                transform_verts(&mv, &m, sky, blk, &mut by_atlas[i]);
            }
        }
        // ---- falling blocks ----
        for f in &g.falling {
            let pos = f.prev_pos.lerp(f.body.pos, alpha);
            let rel = pos - cam.pos;
            let (sky, blk) = Self::light_at(&g.world, pos + v3(0.0, 0.5, 0.0));
            let mv = cube_model(r, f.block);
            let m = Mat4::translate(rel.x - 0.5, rel.y, rel.z - 0.5);
            transform_verts(&mv, &m, sky, blk, &mut by_atlas[0]);
        }
        r.block_atlas.bind();
        let v0 = std::mem::take(&mut by_atlas[0]);
        r.draw_stream(&v0);
        r.item_atlas.bind();
        let v1 = std::mem::take(&mut by_atlas[1]);
        r.draw_stream(&v1);

        // ---- arrows ----
        verts.clear();
        for a in &g.arrows {
            let pos = a.prev_pos.lerp(a.pos, alpha);
            let rel = pos - cam.pos;
            let (sky, blk) = Self::light_at(&g.world, pos);
            let (yaw, pitch) = a.yaw_pitch();
            let m = Mat4::translate(rel.x, rel.y, rel.z) * Mat4::rot_y((-yaw).to_radians()) * Mat4::rot_x(pitch.to_radians());
            let shaft = [(-0.02, -0.02, -0.25), (0.02, 0.02, 0.25)];
            push_box(&mut verts, &m, shaft[0], shaft[1], [110, 80, 40, 255], sky, blk);
            push_box(&mut verts, &m, (-0.04, -0.04, -0.28), (0.04, 0.04, -0.2), [230, 230, 230, 255], sky, blk);
            push_box(&mut verts, &m, (-0.03, -0.03, 0.25), (0.03, 0.03, 0.3), [140, 140, 140, 255], sky, blk);
        }
        r.white.bind();
        r.draw_stream(&verts);
        unsafe { gl::glEnable(gl::CULL_FACE) };
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_particles(&mut self, g: &Game, r: &mut Renderer, cam: &Camera, mvp: &Mat4, alpha: f32, fog: [f32; 3], fog_range: (f32, f32), skydark: f32) {
        if g.particles.is_empty() {
            return;
        }
        let (yr, pr) = (cam.yaw.to_radians(), cam.pitch.to_radians());
        // camera right & up vectors
        let right = v3(yr.cos(), 0.0, yr.sin());
        let fwd = v3(-yr.sin() * pr.cos(), -pr.sin(), yr.cos() * pr.cos());
        let up = right.cross(fwd).norm() * -1.0;
        let mut tex_v = Vec::new();
        let mut flat_v = Vec::new();
        let mut glow_v = Vec::new();
        for p in &g.particles {
            let pos = p.prev_pos.lerp(p.pos, alpha) - cam.pos;
            let (sky, blk) = Self::light_at(&g.world, p.pos);
            let s = p.size * if p.textured { 1.0 } else { 1.0 - (p.age as f32 / p.life as f32) * 0.6 };
            let (a, b) = (right * s, up * s);
            let uv = if p.textured { particle_uv(r, p) } else { [0.5, 0.5, 0.5, 0.5] };
            let c = p.color;
            let mk = |q: Vec3, u: f32, v: f32| Vertex { pos: [q.x, q.y, q.z], uv: [u, v], color: c, light: [sky, blk, 255, 0] };
            let q = [
                mk(pos - a - b, uv[0], uv[3]),
                mk(pos + a - b, uv[2], uv[3]),
                mk(pos + a + b, uv[2], uv[1]),
                mk(pos - a + b, uv[0], uv[1]),
            ];
            let list = if p.textured { &mut tex_v } else if p.emissive { &mut glow_v } else { &mut flat_v };
            list.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
        }
        r.setup_world_shader(mvp, fog, fog_range, skydark);
        unsafe { gl::glDisable(gl::CULL_FACE) };
        r.block_atlas.bind();
        r.draw_stream(&tex_v);
        r.white.bind();
        r.draw_stream(&flat_v);
        r.world_shader.set_f("u_fixed_light", 1.0);
        r.draw_stream(&glow_v);
        r.world_shader.set_f("u_fixed_light", -1.0);
        unsafe { gl::glEnable(gl::CULL_FACE) };
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_selection(&mut self, g: &Game, r: &mut Renderer, cam: &Camera, mvp: &Mat4, x: i32, y: i32, z: i32) {
        let b = g.world.get(x, y, z);
        let (mn, mx) = block::selection_box(b, g.world.get_meta(x, y, z));
        let e = 0.002;
        let o = v3(x as f32, y as f32, z as f32) - cam.pos;
        let lo = v3(o.x + mn[0] - e, o.y + mn[1] - e, o.z + mn[2] - e);
        let hi = v3(o.x + mx[0] + e, o.y + mx[1] + e, o.z + mx[2] + e);
        // crack overlay
        if g.player.breaking == Some((x, y, z)) && g.player.break_progress > 0.0 {
            let stage = ((g.player.break_progress * 10.0) as i32).clamp(0, 9);
            let (u0, v0, s) = r.block_uv(&format!("destroy_stage_{stage}"));
            let mut v = Vec::new();
            let faces: [[Vec3; 4]; 6] = [
                [v3(lo.x, lo.y, lo.z), v3(lo.x, lo.y, hi.z), v3(lo.x, hi.y, hi.z), v3(lo.x, hi.y, lo.z)],
                [v3(hi.x, lo.y, hi.z), v3(hi.x, lo.y, lo.z), v3(hi.x, hi.y, lo.z), v3(hi.x, hi.y, hi.z)],
                [v3(lo.x, lo.y, lo.z), v3(hi.x, lo.y, lo.z), v3(hi.x, lo.y, hi.z), v3(lo.x, lo.y, hi.z)],
                [v3(lo.x, hi.y, hi.z), v3(hi.x, hi.y, hi.z), v3(hi.x, hi.y, lo.z), v3(lo.x, hi.y, lo.z)],
                [v3(hi.x, lo.y, lo.z), v3(lo.x, lo.y, lo.z), v3(lo.x, hi.y, lo.z), v3(hi.x, hi.y, lo.z)],
                [v3(lo.x, lo.y, hi.z), v3(hi.x, lo.y, hi.z), v3(hi.x, hi.y, hi.z), v3(lo.x, hi.y, hi.z)],
            ];
            for q in faces {
                let uv = [[u0, v0 + s], [u0 + s, v0 + s], [u0 + s, v0], [u0, v0]];
                let mk = |k: usize| Vertex { pos: [q[k].x, q[k].y, q[k].z], uv: uv[k], color: [255; 4], light: [255, 255, 255, 0] };
                v.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
            }
            r.world_shader.set_f("u_fixed_light", 1.0);
            r.block_atlas.bind();
            unsafe {
                gl::glEnable(gl::BLEND);
                gl::glBlendFunc(gl::DST_COLOR, gl::SRC_COLOR);
                gl::glEnable(gl::POLYGON_OFFSET_FILL);
                gl::glPolygonOffset(-3.0, -3.0);
                gl::glDepthMask(0);
            }
            r.draw_stream(&v);
            unsafe {
                gl::glDisable(gl::POLYGON_OFFSET_FILL);
                gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::glDisable(gl::BLEND);
                gl::glDepthMask(1);
            }
            r.world_shader.set_f("u_fixed_light", -1.0);
        }
        // outline
        let c = [0, 0, 0, 102];
        let corners = [
            v3(lo.x, lo.y, lo.z),
            v3(hi.x, lo.y, lo.z),
            v3(hi.x, lo.y, hi.z),
            v3(lo.x, lo.y, hi.z),
            v3(lo.x, hi.y, lo.z),
            v3(hi.x, hi.y, lo.z),
            v3(hi.x, hi.y, hi.z),
            v3(lo.x, hi.y, hi.z),
        ];
        let edges = [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 7), (7, 4), (0, 4), (1, 5), (2, 6), (3, 7)];
        let mut v = Vec::new();
        for (a, b) in edges {
            for p in [corners[a], corners[b]] {
                v.push(Vertex { pos: [p.x, p.y, p.z], uv: [0.5, 0.5], color: c, light: [255, 255, 255, 0] });
            }
        }
        r.world_shader.bind();
        r.world_shader.set_mat4("u_mvp", &mvp.0);
        r.world_shader.set_f("u_fixed_light", 1.0);
        r.white.bind();
        unsafe {
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::glLineWidth(2.0 * r.gui_scale as f32 / 2.0);
        }
        r.draw_stream_lines(&v);
        unsafe { gl::glDisable(gl::BLEND) };
        r.world_shader.set_f("u_fixed_light", -1.0);
    }

    fn draw_hand(&mut self, g: &Game, r: &mut Renderer, alpha: f32, aspect: f32, fog: [f32; 3]) {
        let p = &g.player;
        unsafe { gl::glClear(gl::DEPTH_BUFFER_BIT) };
        let proj = Mat4::perspective(70f32.to_radians(), aspect, 0.05, 10.0);
        // bobbing like the camera
        let mut base = Mat4::identity();
        if g.options.view_bobbing {
            let wd = lerp(p.prev_walk_dist, p.walk_dist, alpha);
            let bob = lerp(p.prev_bob, p.bob, alpha);
            let f = wd * std::f32::consts::PI;
            base = Mat4::translate(f.sin() * bob * 0.5, -(f.cos() * bob).abs(), 0.0) * Mat4::rot_z((f.sin() * bob * 3.0).to_radians()) * Mat4::rot_x(((f - 0.2).cos() * bob).abs() * 5.0f32.to_radians());
        }
        let (sky, blk) = Self::light_at(&g.world, p.eye());
        let swing = if p.swinging { lerp(p.prev_swing, p.swing, alpha).max(0.0) } else { 0.0 };
        let equip = 1.0 - lerp(p.prev_equip, p.equip, alpha);
        let held = p.last_held;
        let s = r.world_shader;
        s.bind();
        s.set_mat4("u_mvp", &(proj).0);
        s.set_v3("u_fog_color", fog);
        unsafe { gl::glUniform2f(s.loc("u_fog"), 100.0, 200.0) };
        s.set_f("u_skydark", g.skydark());
        s.set_f("u_fixed_light", -1.0);
        s.set_v4("u_flash", [0.0; 4]);
        let pi = std::f32::consts::PI;
        let sq = swing.sqrt();
        let mut verts = Vec::new();
        // eating animation
        let eat = if p.eating > 0 { (p.eating as f32 + alpha) / 32.0 } else { 0.0 };
        if held.is_empty() {
            // Steve's right arm, pointing from the lower right towards the centre
            let ap = [0.52, -0.27, -0.86, -0.45, 0.7, -0.55];
            let hand = v3(ap[0] - 0.35 * (sq * pi).sin(), ap[1] + 0.25 * (sq * pi * 2.0).sin() - equip * 0.6, ap[2] - 0.25 * (swing * pi).sin());
            let dir = v3(ap[3] + 0.2 * (sq * pi).sin(), ap[4] + 0.3 * (sq * pi).sin(), ap[5]).norm();
            let side = dir.cross(v3(0.0, 1.0, 0.0)).norm();
            let zb = side.cross(dir).norm();
            // model y (down the arm) -> dir; model x -> side; model z -> zb
            let m = base * Mat4::from_basis(side * -1.0, dir, zb, hand) * Mat4::scale(1.0, 1.0, 1.0) * Mat4::translate(1.0 / 16.0, -10.0 / 16.0, 0.0);
            let parts = models::build(ModelKind::Biped);
            let arm = &parts[2];
            let am = m;
            for b in &arm.boxes {
                models::emit_box(&mut verts, b, &am, (64.0, 64.0), Shade { sky, block: blk, color: [255; 4] });
            }
            if let Some(t) = r.skins.get("steve") {
                t.bind();
            }
            unsafe { gl::glDisable(gl::CULL_FACE) };
            r.draw_stream(&verts);
            unsafe { gl::glEnable(gl::CULL_FACE) };
            return;
        }
        let cube = is_cube_item(held.id);
        let mut m = base * Mat4::translate(0.56, -0.52 - equip * 0.6, -0.72);
        if eat > 0.0 {
            let e = eat.min(1.0);
            let shake = if e > 0.2 { ((e * 32.0) / 4.0 * pi).cos().abs() * 0.1 } else { 0.0 };
            m = m * Mat4::translate(-0.3 * e.min(0.25) * 4.0 * 0.5, shake + 0.2 * e.min(0.25) * 4.0, 0.0) * Mat4::rot_y((-40.0 * e.min(0.25) * 4.0f32).to_radians());
        } else {
            m = m * Mat4::translate(-0.4 * (sq * pi).sin(), 0.2 * (sq * pi * 2.0).sin(), -0.2 * (swing * pi).sin());
            m = m
                * Mat4::rot_y((45.0 + (swing * swing * pi).sin() * -20.0f32).to_radians())
                * Mat4::rot_z(((sq * pi).sin() * -20.0f32).to_radians())
                * Mat4::rot_x(((sq * pi).sin() * -80.0f32).to_radians())
                * Mat4::rot_y((-45f32).to_radians());
        }
        let model_m = if cube {
            m * Mat4::rot_y(45f32.to_radians()) * Mat4::scale(0.4, 0.4, 0.4) * Mat4::translate(-0.5, -0.5, -0.5)
        } else {
            let tool = item::tool_info(held.id).is_some() || held.id == item::STICK || held.id == item::BONE;
            if tool {
                m * Mat4::translate(1.13 / 16.0, 3.2 / 16.0, 1.13 / 16.0) * Mat4::rot_y((-90f32).to_radians()) * Mat4::rot_z(25f32.to_radians()) * Mat4::scale(0.68, 0.68, 0.68) * Mat4::translate(-0.5, -0.5, 0.0)
            } else {
                m * Mat4::translate(1.13 / 16.0, 3.2 / 16.0, 1.13 / 16.0) * Mat4::rot_y((-90f32).to_radians()) * Mat4::rot_z(25f32.to_radians()) * Mat4::scale(0.68, 0.68, 0.68) * Mat4::translate(-0.5, -0.5, 0.0)
            }
        };
        let (mv, atlas) = if held.id == item::BOW && p.bow_charge > 0 {
            let stage = if p.bow_charge >= 18 { 2 } else if p.bow_charge > 13 { 1 } else { 0 };
            let tex = ["bow_pulling_0", "bow_pulling_1", "bow_pulling_2"][stage];
            self.items.get_tex(r, 60000 + stage as u16, tex).clone()
        } else {
            self.items.get(r, held.id).clone()
        };
        transform_verts(&mv, &model_m, sky, blk, &mut verts);
        if atlas == Atlas::Blocks { r.block_atlas.bind() } else { r.item_atlas.bind() }
        unsafe { gl::glDisable(gl::CULL_FACE) };
        r.draw_stream(&verts);
        unsafe { gl::glEnable(gl::CULL_FACE) };
    }

    // ---------------- HUD ----------------

    pub fn draw_hud(&mut self, g: &Game, r: &mut Renderer, ui: &mut Ui, fps: f32) {
        let p = &g.player;
        let (w, h) = (ui.w, ui.h);
        // screen overlays: underwater tint / pumpkin etc.
        let (cx, cy, cz) = p.eye().floor();
        if g.world.get(cx, cy, cz) == WATER {
            ui.rect(r, 0.0, 0.0, w, h, [20, 40, 120, 70]);
        }
        if p.fire > 0 && !p.body.in_water {
            ui.gradient(r, 0.0, h * 0.5, w, h * 0.5, [255, 120, 0, 0], [255, 100, 0, 110]);
        }
        if g.hide_hud {
            return;
        }
        // crosshair (inverted colours)
        ui.flush();
        unsafe { gl::glBlendFunc(gl::ONE_MINUS_DST_COLOR, gl::ONE_MINUS_SRC_COLOR) };
        let (cw, ch) = ui.sprite_size(r, "crosshair");
        ui.sprite(r, "crosshair", ((w - cw) / 2.0).floor(), ((h - ch) / 2.0).floor());
        ui.flush();
        unsafe { gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA) };

        // hotbar
        let hx = (w / 2.0 - 91.0).floor();
        ui.sprite(r, "hotbar", hx, h - 22.0);
        ui.sprite(r, "hotbar_selection", hx - 1.0 + p.inv.selected as f32 * 20.0, h - 23.0);
        for i in 0..9 {
            let st = p.inv.slots[i];
            ui.item(r, &st, hx + 3.0 + i as f32 * 20.0, h - 19.0);
        }
        // experience bar (empty) like vanilla
        let xb = hx;
        let yb = h - 29.0;
        ui.rect(r, xb, yb, 182.0, 5.0, [0, 0, 0, 255]);
        ui.rect(r, xb + 1.0, yb + 1.0, 180.0, 3.0, [30, 44, 24, 255]);
        for k in 1..18 {
            ui.rect(r, xb + k as f32 * 10.1, yb + 1.0, 1.0, 3.0, [0, 0, 0, 255]);
        }
        // hearts
        let hy = h - 39.0;
        let blink = p.invuln > 0 && (p.invuln / 3) % 2 == 1;
        let low = p.health <= 4.0;
        for i in 0..10 {
            let x = hx + i as f32 * 8.0;
            let mut y = hy;
            if low {
                y += ((g.tick_count as i64 * 31 + i as i64 * 7) % 3 - 1) as f32;
            }
            ui.sprite_c(r, "heart_container", x, y, if blink { [255, 255, 255, 255] } else { [255, 255, 255, 255] });
            let hp = p.health.ceil() as i32;
            if hp >= i * 2 + 2 {
                ui.sprite(r, "heart_full", x, y);
            } else if hp == i * 2 + 1 {
                ui.sprite(r, "heart_half", x, y);
            }
            if blink {
                let ph = p.prev_health.ceil() as i32;
                if ph > hp && ph >= i * 2 + 1 && hp < i * 2 + 2 {
                    ui.sprite_c(r, "heart_full", x, y, [255, 255, 255, 120]);
                }
            }
        }
        // hunger
        for i in 0..10 {
            let x = hx + 182.0 - 9.0 - i as f32 * 8.0;
            let mut y = hy;
            if p.saturation <= 0.0 && g.tick_count % ((p.food as u64) * 3 + 1) == 0 {
                y += ((g.tick_count as i64 * 13 + i as i64 * 5) % 3 - 1) as f32;
            }
            ui.sprite(r, "food_empty", x, y);
            if p.food >= i * 2 + 2 {
                ui.sprite(r, "food_full", x, y);
            } else if p.food == i * 2 + 1 {
                ui.sprite(r, "food_half", x, y);
            }
        }
        // air
        if g.world.get(cx, cy, cz) == WATER || p.air < 300 {
            let full = ((p.air.max(0) as f32 - 2.0) * 10.0 / 300.0).ceil() as i32;
            let partial = (p.air.max(0) as f32 * 10.0 / 300.0).ceil() as i32 - full;
            for i in 0..(full + partial) {
                let x = hx + 182.0 - 9.0 - i as f32 * 8.0;
                ui.sprite(r, "air_bubble", x, hy - 10.0);
            }
        }
        // held item name
        if g.held_name_timer > 0 {
            let st = p.inv.held();
            if !st.is_empty() {
                let a = ((g.held_name_timer as f32 * 256.0 / 10.0).min(255.0)) as u8;
                let name = item::name(st.id);
                ui.text_centered(r, name, w / 2.0, h - 59.0, [255, 255, 255, a]);
            }
        }
        if let Some((m, t)) = &g.action_msg {
            let a = ((*t as f32 * 255.0 / 10.0).min(255.0)) as u8;
            ui.text_centered(r, m, w / 2.0, h - 68.0, [255, 255, 255, a]);
        }
        if p.sleeping > 0 {
            let a = ((p.sleeping as f32 / 70.0).min(1.0) * 230.0) as u8;
            ui.rect(r, 0.0, 0.0, w, h, [16, 16, 32, a]);
        }
        if g.show_debug {
            self.draw_debug(g, r, ui, fps);
        }
    }

    fn draw_debug(&mut self, g: &Game, r: &mut Renderer, ui: &mut Ui, fps: f32) {
        let p = &g.player;
        let pos = p.body.pos;
        let (bx, by, bz) = pos.floor();
        let facing = {
            let y = crate::math::wrap_deg(p.yaw);
            if (-45.0..45.0).contains(&y) {
                "south (Towards positive Z)"
            } else if (45.0..135.0).contains(&y) {
                "west (Towards negative X)"
            } else if (-135.0..-45.0).contains(&y) {
                "east (Towards positive X)"
            } else {
                "north (Towards negative Z)"
            }
        };
        let l = g.world.light(bx, by, bz);
        let day = g.world.time / 24000;
        let lines = vec![
            format!("Minecraft Rust Edition ({:.0} fps, {} chunk updates)", fps, r.chunks_rendered),
            format!("C: {} meshes, {} loaded chunks", r.sections.len(), g.world.chunks.len()),
            format!("E: {} mobs, {} items, {} particles", g.mobs.len(), g.items.len(), g.particles.len()),
            String::new(),
            format!("XYZ: {:.3} / {:.5} / {:.3}", pos.x, pos.y, pos.z),
            format!("Block: {} {} {}", bx, by, bz),
            format!("Chunk: {} {} {} in {} {}", bx & 15, by & 15, bz & 15, bx >> 4, bz >> 4),
            format!("Facing: {} ({:.1} / {:.1})", facing, crate::math::wrap_deg(p.yaw), p.pitch),
            format!("Light: {} ({} sky, {} block)", (l >> 4).max(l & 15), l >> 4, l & 15),
            format!("Biome: {}", g.world.biome(bx, bz).name()),
            format!("Local Difficulty: {} // Day {}", g.options.difficulty.name(), day),
            format!("Seed: {}", g.world.seed as i64),
        ];
        let mut y = 2.0;
        for line in &lines {
            if !line.is_empty() {
                let wdt = ui.text_width(r, line);
                ui.rect(r, 1.0, y - 1.0, wdt + 1.0, 9.0, [80, 80, 80, 144]);
                ui.text_no_shadow(r, line, 2.0, y, [224, 224, 224, 255]);
            }
            y += 9.0;
        }
        let _ = WHITE;
    }
}

fn particle_uv(r: &Renderer, p: &Particle) -> [f32; 4] {
    let b = p.uv[0] as usize;
    let tile = r.tiles.blocks.get(b).map(|t| t[2]).unwrap_or(0);
    let tile = if b == GRASS as usize { r.tiles.blocks[DIRT as usize][2] } else { tile };
    let (u0, v0, s) = r.tiles.uv(tile);
    let su = u0 + p.uv[1] * s;
    let sv = v0 + p.uv[2] * s;
    [su, sv, su + s * 0.25, sv + s * 0.25]
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + crate::math::wrap_deg(b - a) * t
}

fn push_box(out: &mut Vec<Vertex>, m: &Mat4, lo: (f32, f32, f32), hi: (f32, f32, f32), c: [u8; 4], sky: u8, blk: u8) {
    let (x0, y0, z0) = lo;
    let (x1, y1, z1) = hi;
    let faces: [([[f32; 3]; 4], u8); 6] = [
        ([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], 153),
        ([[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]], 153),
        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], 128),
        ([[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], 255),
        ([[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]], 204),
        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], 204),
    ];
    for (q, sh) in faces {
        let pts: Vec<Vec3> = q.iter().map(|p| m.transform(v3(p[0], p[1], p[2]))).collect();
        let mk = |k: usize| Vertex { pos: [pts[k].x, pts[k].y, pts[k].z], uv: [0.5, 0.5], color: c, light: [sky, blk, sh, 0] };
        out.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
    }
}
