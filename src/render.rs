//! Rendering resources and world rendering (chunks, sky, clouds, overlays).

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::assets;
use crate::block::{self, NUM_BLOCKS};
use crate::gl::{self, Mesh, Shader, Texture, Vertex};
use crate::image::Image;
use crate::math::{Frustum, Mat4, Vec3};
use crate::mesher::{self, MeshInput, TileTable};
use crate::world::{World, SECTIONS};

const WORLD_VS: &str = r#"
#version 120
attribute vec3 a_pos;
attribute vec2 a_uv;
attribute vec4 a_color;
attribute vec4 a_light;
uniform mat4 u_mvp;
uniform vec3 u_offset;
uniform float u_skydark;   // 0 (day) .. 11 (night)
uniform float u_fixed_light; // >=0: override light level (0..1) for gui/held items
varying vec2 v_uv;
varying vec4 v_color;
varying vec3 v_light;
varying float v_dist;

float curve(float l) {
    float f = l / (4.0 - 3.0 * l);
    float g = 1.0 - pow(1.0 - f, 4.0);
    return mix(f, g, 0.45) * 0.96 + 0.04;
}

void main() {
    vec3 wp = a_pos + u_offset;
    gl_Position = u_mvp * vec4(wp, 1.0);
    v_dist = length(wp);
    v_uv = a_uv;
    v_color = a_color;
    float sky = clamp((a_light.x * 15.0 - u_skydark) / 15.0, 0.0, 1.0);
    float blk = a_light.y;
    vec3 skyc = vec3(curve(sky));
    // slight blue tint at night
    skyc *= mix(vec3(1.0), vec3(0.75, 0.8, 1.0), u_skydark / 11.0);
    vec3 blkc = vec3(curve(blk)) * vec3(1.0, 0.93, 0.82);
    vec3 l = max(skyc, blkc);
    if (u_fixed_light >= 0.0) l = vec3(u_fixed_light);
    v_light = l * a_light.z;
}
"#;

const WORLD_FS: &str = r#"
#version 120
uniform sampler2D u_tex;
uniform vec3 u_fog_color;
uniform vec2 u_fog;
uniform float u_alpha_ref;
uniform vec4 u_flash;
uniform vec4 u_color_mul;
varying vec2 v_uv;
varying vec4 v_color;
varying vec3 v_light;
varying float v_dist;
void main() {
    vec4 t = texture2D(u_tex, v_uv) * v_color * u_color_mul;
    if (t.a < u_alpha_ref) discard;
    vec3 c = t.rgb * v_light;
    c = mix(c, u_flash.rgb, u_flash.a);
    float f = clamp((u_fog.y - v_dist) / (u_fog.y - u_fog.x), 0.0, 1.0);
    gl_FragColor = vec4(mix(u_fog_color, c, f), t.a);
}
"#;

const GUI_VS: &str = r#"
#version 120
attribute vec3 a_pos;
attribute vec2 a_uv;
attribute vec4 a_color;
uniform mat4 u_mvp;
varying vec2 v_uv;
varying vec4 v_color;
void main() {
    gl_Position = u_mvp * vec4(a_pos, 1.0);
    v_uv = a_uv;
    v_color = a_color;
}
"#;

const GUI_FS: &str = r#"
#version 120
uniform sampler2D u_tex;
varying vec2 v_uv;
varying vec4 v_color;
void main() {
    vec4 t = texture2D(u_tex, v_uv) * v_color;
    if (t.a < 0.004) discard;
    gl_FragColor = t;
}
"#;

pub struct SectionMesh {
    pub solid: Mesh,
    pub trans: Mesh,
}


/// Rectangle in an atlas, in pixels.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Renderer {
    pub world_shader: Shader,
    pub gui_shader: Shader,
    pub block_atlas: Texture,
    pub block_atlas_img: Image,
    pub item_atlas: Texture,
    pub item_atlas_img: Image,
    pub tiles: Arc<TileTable>,
    pub block_tile: HashMap<String, u16>,
    pub item_tile: HashMap<String, u16>,
    pub sections: HashMap<(i32, i32, i32), SectionMesh>,
    section_gen: HashMap<(i32, i32, i32), u64>,
    gen_counter: u64,
    mesh_tx: Sender<(Box<MeshInput>, u64)>,
    mesh_rx: Receiver<((i32, i32, i32), u64, Vec<Vertex>, Vec<Vertex>)>,
    in_flight: usize,
    pub stream: Mesh,
    pub stream2: Mesh,
    pub skins: HashMap<&'static str, Texture>,
    pub sun: Texture,
    pub moon: Texture,
    pub clouds_img: Image,
    pub clouds_mesh: Mesh,
    clouds_origin: (i32, i32),
    pub white: Texture,
    pub gui_atlas: Texture,
    pub gui_rects: HashMap<&'static str, Rect>,
    pub font: Texture,
    pub glyph_w: [u32; 256],
    pub stars: Mesh,
    pub rain_tex: Texture,
    pub snow_tex: Texture,
    pub width: u32,
    pub height: u32,
    pub gui_scale: u32,
    pub chunks_rendered: usize,
}

fn pack_tiles(names: &[(String, Image)]) -> (Image, HashMap<String, u16>) {
    let per_row = 16usize;
    let rows = names.len().div_ceil(per_row).max(16);
    let size = 16 * per_row;
    assert!(rows <= 16, "atlas overflow");
    let mut img = Image::new(size, size);
    let mut map = HashMap::new();
    for (i, (n, t)) in names.iter().enumerate() {
        let x = (i % per_row) * 16;
        let y = (i / per_row) * 16;
        img.blit(t, x as i32, y as i32);
        map.insert(n.clone(), i as u16);
    }
    (img, map)
}

impl Renderer {
    pub fn new() -> Renderer {
        let world_shader = Shader::new(WORLD_VS, WORLD_FS);
        let gui_shader = Shader::new(GUI_VS, GUI_FS);

        // ---- block atlas ----
        let mut list: Vec<(String, Image)> = Vec::new();
        for n in assets::blocks::NAMES {
            if let Some(img) = assets::blocks::get(n) {
                list.push((n.to_string(), img));
            }
        }
        let (block_atlas_img, block_tile) = pack_tiles(&list);
        let block_atlas = Texture::from_image(&block_atlas_img);
        let t = |n: &str| -> u16 { *block_tile.get(n).unwrap_or_else(|| block_tile.get("stone").unwrap()) };
        let mut blocks = vec![[0u16; 4]; NUM_BLOCKS];
        for (i, d) in block::BLOCKS.iter().enumerate() {
            let f = d.front.unwrap_or(d.tex[2]);
            blocks[i] = [t(d.tex[0]), t(d.tex[1]), t(d.tex[2]), t(f)];
        }
        blocks[block::FARMLAND as usize][3] = t("farmland_moist");
        let tiles = Arc::new(TileTable {
            blocks,
            grass_overlay: t("grass_side_overlay"),
            grass_snowed: t("grass_side_snowed"),
            wheat: std::array::from_fn(|i| t(&format!("wheat_stage{i}"))),
            bed: [t("bed_head_top"), t("bed_foot_top"), t("bed_head_side"), t("bed_foot_side"), t("bed_head_end"), t("bed_foot_end")],
            tiles_per_row: 16,
        });

        // ---- item atlas ----
        let mut ilist: Vec<(String, Image)> = Vec::new();
        for n in assets::items::NAMES {
            if let Some(img) = assets::items::get(n) {
                ilist.push((n.to_string(), img));
            }
        }
        let (item_atlas_img, item_tile) = pack_tiles(&ilist);
        let item_atlas = Texture::from_image(&item_atlas_img);

        // ---- mesh workers ----
        let (mesh_tx, job_rx) = channel::<(Box<MeshInput>, u64)>();
        let (res_tx, mesh_rx) = channel();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(2).clamp(1, 4);
        for _ in 0..n_threads {
            let rx = job_rx.clone();
            let tx = res_tx.clone();
            let tiles = tiles.clone();
            std::thread::spawn(move || loop {
                let job = {
                    let l = rx.lock().unwrap();
                    l.recv()
                };
                let Ok((inp, g)) = job else { break };
                let (s, t) = mesher::mesh_section(&inp, &tiles);
                if tx.send(((inp.cx, inp.sy, inp.cz), g, s, t)).is_err() {
                    break;
                }
            });
        }

        // ---- other textures ----
        let mut skins = HashMap::new();
        for n in ["pig", "cow", "sheep", "sheep_fur", "chicken", "zombie", "steve", "skeleton", "creeper", "spider"] {
            if let Some(img) = assets::mobs::get(n) {
                skins.insert(n, Texture::from_image(&img));
            }
        }
        let sun = Texture::from_image(&assets::mobs::get("sun").unwrap_or_else(|| Image::filled(32, 32, [255, 255, 200, 255])));
        let moon = Texture::from_image(&assets::mobs::get("moon").unwrap_or_else(|| Image::filled(32, 32, [200, 200, 200, 255])));
        let clouds_img = assets::mobs::get("clouds").unwrap_or_else(|| Image::new(256, 256));
        let white = Texture::from_image(&Image::filled(4, 4, [255, 255, 255, 255]));
        // precipitation textures (vanilla-like streaks and flakes)
        let mut rimg = Image::new(16, 128);
        let mut rr = crate::image::Rng::new(77);
        for _ in 0..7 {
            let x = rr.range(16) as i32;
            let y = rr.range(128) as i32;
            let len = 8 + rr.range(12) as i32;
            for k in 0..len {
                let a = 60 + (k * 110 / len) as u8;
                rimg.put(x, (y + k) % 128, [200, 215, 255, a]);
            }
        }
        let rain_tex = Texture::from_image(&rimg);
        rain_tex.set_repeat();
        let mut simg = Image::new(16, 64);
        for _ in 0..9 {
            let x = rr.range(15) as i32;
            let y = rr.range(63) as i32;
            simg.put(x, y, [255, 255, 255, 230]);
            if rr.range(2) == 0 {
                simg.put(x + 1, y, [240, 240, 250, 200]);
                simg.put(x, y + 1, [240, 240, 250, 200]);
                simg.put(x + 1, y + 1, [230, 230, 240, 170]);
            }
        }
        let snow_tex = Texture::from_image(&simg);
        snow_tex.set_repeat();

        // ---- GUI atlas (shelf packing) ----
        let mut gui_imgs: Vec<(&'static str, Image)> = Vec::new();
        for n in assets::gui::NAMES {
            if *n == "font" {
                continue;
            }
            if let Some(img) = assets::gui::get(n) {
                gui_imgs.push((n, img));
            }
        }
        gui_imgs.sort_by_key(|(_, i)| std::cmp::Reverse(i.h));
        let atlas_w = 1024;
        let mut gimg = Image::new(atlas_w, 1024);
        let mut gui_rects = HashMap::new();
        let (mut x, mut y, mut row_h) = (0usize, 0usize, 0usize);
        for (n, img) in &gui_imgs {
            if x + img.w + 1 > atlas_w {
                x = 0;
                y += row_h + 1;
                row_h = 0;
            }
            gimg.blit(img, x as i32, y as i32);
            gui_rects.insert(*n, Rect { x: x as f32, y: y as f32, w: img.w as f32, h: img.h as f32 });
            x += img.w + 1;
            row_h = row_h.max(img.h);
        }
        let gui_atlas = Texture::from_image(&gimg);
        let font_img = assets::gui::get("font").unwrap_or_else(|| Image::new(128, 128));
        let font = Texture::from_image(&font_img);
        let mut glyph_w = [6u32; 256];
        for (c, w) in glyph_w.iter_mut().enumerate().take(128) {
            *w = assets::gui::glyph_width(c as u8);
        }

        let mut r = Renderer {
            world_shader,
            gui_shader,
            block_atlas,
            block_atlas_img,
            item_atlas,
            item_atlas_img,
            tiles,
            block_tile,
            item_tile,
            sections: HashMap::new(),
            section_gen: HashMap::new(),
            gen_counter: 0,
            mesh_tx,
            mesh_rx,
            in_flight: 0,
            stream: Mesh::new(),
            stream2: Mesh::new(),
            skins,
            sun,
            moon,
            clouds_img,
            clouds_mesh: Mesh::new(),
            clouds_origin: (i32::MIN, i32::MIN),
            white,
            gui_atlas,
            gui_rects,
            font,
            glyph_w,
            stars: Mesh::new(),
            rain_tex,
            snow_tex,
            width: 1,
            height: 1,
            gui_scale: 2,
            chunks_rendered: 0,
        };
        r.build_stars();
        gl::enable_vertex_attribs();
        r
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
        // Vanilla "auto" GUI scale: largest scale where the screen is >= 320x240 in GUI pixels
        let mut s = 1;
        while (w / (s + 1)) >= 320 && (h / (s + 1)) >= 240 && s < 4 {
            s += 1;
        }
        self.gui_scale = s;
    }

    fn build_stars(&mut self) {
        let mut rng = crate::noise::Random::new(10842);
        let mut v = Vec::new();
        for _ in 0..1500 {
            let d = Vec3 { x: rng.uniform(-1.0, 1.0), y: rng.uniform(-1.0, 1.0), z: rng.uniform(-1.0, 1.0) };
            let l = d.len();
            if !(0.01..=1.0).contains(&l) {
                continue;
            }
            let d = d.norm();
            let size = 0.15 + rng.next_f32() * 0.1;
            let p = d * 100.0;
            // build a small quad facing the origin
            let up = if d.y.abs() > 0.9 { Vec3 { x: 1.0, y: 0.0, z: 0.0 } } else { Vec3 { x: 0.0, y: 1.0, z: 0.0 } };
            let a = d.cross(up).norm() * size;
            let b = d.cross(a).norm() * size;
            let c = [255, 255, 255, 255];
            let mk = |q: Vec3| Vertex { pos: [q.x, q.y, q.z], uv: [0.5, 0.5], color: c, light: [255, 255, 255, 0] };
            let (p0, p1, p2, p3) = (p - a - b, p + a - b, p + a + b, p - a + b);
            v.extend_from_slice(&[mk(p0), mk(p1), mk(p2), mk(p0), mk(p2), mk(p3)]);
        }
        self.stars.upload(&v, gl::STATIC_DRAW);
    }

    // ---------------- chunk meshes ----------------

    /// Submit dirty sections for meshing; apply finished meshes.
    pub fn update_chunks(&mut self, world: &mut World, cam: Vec3, render_dist: i32) {
        // Apply results
        while let Ok((key, g, s, t)) = self.mesh_rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            if self.section_gen.get(&key) != Some(&g) {
                continue;
            }
            self.apply_mesh(key, &s, &t);
        }
        let pcx = (cam.x.floor() as i32) >> 4;
        let pcz = (cam.z.floor() as i32) >> 4;
        // Collect dirty sections, nearest first
        let mut dirty: Vec<(i32, i32, i32, i32)> = Vec::new();
        for (&(cx, cz), ch) in world.chunks.iter() {
            if (cx - pcx).abs() > render_dist || (cz - pcz).abs() > render_dist {
                continue;
            }
            // Only mesh when all 4 neighbours are present (avoids walls at the edge)
            if ch.dirty.iter().any(|d| *d) {
                let ok = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().all(|(dx, dz)| world.chunks.contains_key(&(cx + dx, cz + dz)));
                if !ok {
                    continue;
                }
                for sy in 0..SECTIONS as i32 {
                    if ch.dirty[sy as usize] {
                        let d = (cx - pcx).pow(2) + (cz - pcz).pow(2);
                        dirty.push((d, cx, sy, cz));
                    }
                }
            }
        }
        dirty.sort();
        let mut sync_budget = 8;
        for (d, cx, sy, cz) in dirty {
            let near = d <= 2;
            if !near && self.in_flight > 64 {
                break;
            }
            if let Some(ch) = world.chunks.get_mut(&(cx, cz)) {
                ch.dirty[sy as usize] = false;
            }
            self.gen_counter += 1;
            let g = self.gen_counter;
            self.section_gen.insert((cx, sy, cz), g);
            let inp = MeshInput::build(world, cx, sy, cz);
            if near && sync_budget > 0 {
                sync_budget -= 1;
                let (s, t) = mesher::mesh_section(&inp, &self.tiles);
                self.apply_mesh((cx, sy, cz), &s, &t);
            } else {
                self.in_flight += 1;
                let _ = self.mesh_tx.send((Box::new(inp), g));
            }
        }
    }

    fn apply_mesh(&mut self, key: (i32, i32, i32), s: &[Vertex], t: &[Vertex]) {
        if s.is_empty() && t.is_empty() {
            self.sections.remove(&key);
            return;
        }
        let e = self.sections.entry(key).or_insert_with(|| SectionMesh { solid: Mesh::new(), trans: Mesh::new() });
        e.solid.upload(s, gl::STATIC_DRAW);
        e.trans.upload(t, gl::STATIC_DRAW);
    }

    pub fn drop_chunk_meshes(&mut self, cx: i32, cz: i32) {
        for sy in 0..SECTIONS as i32 {
            self.sections.remove(&(cx, sy, cz));
            self.section_gen.remove(&(cx, sy, cz));
        }
    }

    /// Set the common world-shader uniforms.
    pub fn setup_world_shader(&self, mvp: &Mat4, fog_color: [f32; 3], fog: (f32, f32), skydark: f32) {
        let s = self.world_shader;
        s.bind();
        s.set_mat4("u_mvp", &mvp.0);
        s.set_v3("u_fog_color", fog_color);
        unsafe {
            gl::glUniform2f(s.loc("u_fog"), fog.0, fog.1);
        }
        s.set_f("u_skydark", skydark);
        s.set_f("u_alpha_ref", 0.1);
        s.set_v4("u_flash", [0.0; 4]);
        s.set_v4("u_color_mul", [1.0; 4]);
        s.set_f("u_fixed_light", -1.0);
        s.set_v3("u_offset", [0.0; 3]);
        s.set_i("u_tex", 0);
    }

    pub fn draw_chunks(&mut self, cam: Vec3, mvp: &Mat4, translucent: bool) {
        let frustum = Frustum::from_matrix(mvp);
        let s = self.world_shader;
        self.block_atlas.bind();
        let mut list: Vec<(f32, (i32, i32, i32))> = Vec::new();
        for (&(cx, sy, cz), m) in self.sections.iter() {
            let mesh = if translucent { &m.trans } else { &m.solid };
            if mesh.count == 0 {
                continue;
            }
            let min = Vec3 { x: (cx * 16) as f32 - cam.x, y: (sy * 16) as f32 - cam.y, z: (cz * 16) as f32 - cam.z };
            let max = min + Vec3 { x: 16.0, y: 16.0, z: 16.0 };
            if !frustum.aabb_visible(min, max) {
                continue;
            }
            let c = min + Vec3 { x: 8.0, y: 8.0, z: 8.0 };
            list.push((c.dot(c), (cx, sy, cz)));
        }
        if translucent {
            list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        } else {
            list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
        if !translucent {
            self.chunks_rendered = list.len();
        }
        for (_, key) in list {
            let m = &self.sections[&key];
            let (cx, sy, cz) = key;
            s.set_v3("u_offset", [(cx * 16) as f32 - cam.x, (sy * 16) as f32 - cam.y, (cz * 16) as f32 - cam.z]);
            if translucent {
                m.trans.draw();
            } else {
                m.solid.draw();
            }
        }
        s.set_v3("u_offset", [0.0; 3]);
    }

    /// Upload vertices to the stream buffer and draw them with the current shader.
    pub fn draw_stream(&mut self, v: &[Vertex]) {
        if v.is_empty() {
            return;
        }
        self.stream.upload(v, gl::STREAM_DRAW);
        self.stream.draw();
    }
    pub fn draw_stream_lines(&mut self, v: &[Vertex]) {
        if v.is_empty() {
            return;
        }
        self.stream2.upload(v, gl::STREAM_DRAW);
        self.stream2.draw_mode(gl::LINES);
    }

    // ---------------- sky ----------------

    /// Draws sky plane, sun, moon and stars. `view_rot` is the rotation-only view matrix * projection.
    pub fn draw_sky(&mut self, rot_proj: &Mat4, sky_color: [f32; 3], fog_color: [f32; 3], celestial: f32, star_brightness: f32, sunrise: Option<[f32; 4]>) {
        let s = self.world_shader;
        s.bind();
        s.set_mat4("u_mvp", &rot_proj.0);
        s.set_f("u_fixed_light", 1.0);
        s.set_v3("u_fog_color", fog_color);
        unsafe {
            gl::glUniform2f(s.loc("u_fog"), 0.0, 1.0e6);
            gl::glDepthMask(0);
            gl::glDisable(gl::CULL_FACE);
        }
        self.white.bind();
        // Sky plane above, fog-blended at distance to horizon colour
        unsafe { gl::glUniform2f(s.loc("u_fog"), 10.0, 300.0) };
        let sc = [(sky_color[0] * 255.0) as u8, (sky_color[1] * 255.0) as u8, (sky_color[2] * 255.0) as u8, 255];
        let mut v = Vec::new();
        let r = 384.0;
        let h = 16.0;
        let n = 16;
        for i in 0..n {
            for j in 0..n {
                let x0 = -r + 2.0 * r * i as f32 / n as f32;
                let x1 = -r + 2.0 * r * (i + 1) as f32 / n as f32;
                let z0 = -r + 2.0 * r * j as f32 / n as f32;
                let z1 = -r + 2.0 * r * (j + 1) as f32 / n as f32;
                let mk = |x: f32, z: f32| Vertex { pos: [x, h, z], uv: [0.5, 0.5], color: sc, light: [255, 255, 255, 0] };
                v.extend_from_slice(&[mk(x0, z0), mk(x1, z0), mk(x1, z1), mk(x0, z0), mk(x1, z1), mk(x0, z1)]);
            }
        }
        self.draw_stream(&v);

        unsafe {
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::glUniform2f(s.loc("u_fog"), 0.0, 1.0e6);
        }
        // Sunrise/sunset glow fan
        if let Some(c) = sunrise {
            let mut v = Vec::new();
            let side = if celestial.sin() > 0.0 { 1.0 } else { -1.0 };
            let ctr = [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, (c[3] * 255.0) as u8];
            let edge = [ctr[0], ctr[1], ctr[2], 0];
            let segs = 16;
            for i in 0..segs {
                let a0 = i as f32 / segs as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / segs as f32 * std::f32::consts::TAU;
                let p = |a: f32| -> [f32; 3] { [side * 100.0, a.cos() * 40.0 * c[3] + 0.0, a.sin() * 120.0] };
                let mk = |pos: [f32; 3], col: [u8; 4]| Vertex { pos, uv: [0.5, 0.5], color: col, light: [255, 255, 255, 0] };
                v.extend_from_slice(&[mk([side * 100.0, 0.0, 0.0], ctr), mk(p(a0), edge), mk(p(a1), edge)]);
            }
            self.draw_stream(&v);
        }

        // Sun & moon (additive)
        unsafe { gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE) };
        let rot = Mat4::rot_x(0.0) * Mat4::rot_z(celestial);
        let quad = |size: f32, y: f32, flip: bool| -> Vec<Vertex> {
            let s2 = size;
            let pts = [[-s2, y, -s2], [s2, y, -s2], [s2, y, s2], [-s2, y, s2]];
            let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            let mut out = Vec::new();
            let order: [usize; 6] = if flip { [0, 2, 1, 0, 3, 2] } else { [0, 1, 2, 0, 2, 3] };
            for &i in &order {
                let p = rot.transform(Vec3 { x: pts[i][0], y: pts[i][1], z: pts[i][2] });
                out.push(Vertex { pos: [p.x, p.y, p.z], uv: uvs[i], color: [255, 255, 255, 255], light: [255, 255, 255, 0] });
            }
            out
        };
        self.sun.bind();
        let v = quad(30.0, 100.0, false);
        self.draw_stream(&v);
        self.moon.bind();
        let v = quad(20.0, -100.0, true);
        self.draw_stream(&v);
        // Stars
        if star_brightness > 0.0 {
            self.white.bind();
            s.set_v4("u_color_mul", [1.0, 1.0, 1.0, star_brightness]);
            let m = *rot_proj * rot;
            s.set_mat4("u_mvp", &m.0);
            self.stars.draw();
            s.set_v4("u_color_mul", [1.0; 4]);
        }
        unsafe {
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::glDisable(gl::BLEND);
            gl::glDepthMask(1);
            gl::glEnable(gl::CULL_FACE);
        }
        s.set_f("u_fixed_light", -1.0);
    }

    /// 3D clouds (vanilla "fancy" style boxes, 12x4x12 per texel).
    pub fn draw_clouds(&mut self, cam: Vec3, mvp: &Mat4, time: f64, color: [f32; 3], fog_far: f32) {
        let cell = 12.0f32;
        let cloud_y = 108.0f32;
        let drift = (time * 0.03) as f32; // blocks moved along +x
        // cloud-space coordinates of camera
        let cxw = cam.x + drift;
        let ox = (cxw / cell).floor() as i32;
        let oz = (cam.z / cell).floor() as i32;
        let radius = 24;
        if (ox, oz) != self.clouds_origin {
            self.clouds_origin = (ox, oz);
            let img = &self.clouds_img;
            let at = |x: i32, z: i32| -> bool {
                let px = x.rem_euclid(img.w as i32) as usize;
                let pz = z.rem_euclid(img.h as i32) as usize;
                img.get(px, pz)[3] > 128
            };
            let mut v = Vec::new();
            let shade = [[0.9, 0.9, 0.9], [0.9, 0.9, 0.9], [0.7, 0.7, 0.7], [1.0, 1.0, 1.0], [0.8, 0.8, 0.8], [0.8, 0.8, 0.8]];
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let (x, z) = (ox + dx, oz + dz);
                    if !at(x, z) {
                        continue;
                    }
                    let x0 = dx as f32 * cell;
                    let z0 = dz as f32 * cell;
                    let (x1, z1) = (x0 + cell, z0 + cell);
                    let (y0, y1) = (0.0, 4.0);
                    let faces: [([[f32; 3]; 4], usize, bool); 6] = [
                        ([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], 0, !at(x - 1, z)),
                        ([[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]], 1, !at(x + 1, z)),
                        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], 2, true),
                        ([[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], 3, true),
                        ([[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]], 4, !at(x, z - 1)),
                        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], 5, !at(x, z + 1)),
                    ];
                    for (q, f, vis) in faces {
                        if !vis {
                            continue;
                        }
                        let s = shade[f];
                        let c = [(s[0] * 255.0) as u8, (s[1] * 255.0) as u8, (s[2] * 255.0) as u8, 204];
                        let mk = |p: [f32; 3]| Vertex { pos: p, uv: [0.5, 0.5], color: c, light: [255, 255, 255, 0] };
                        v.extend_from_slice(&[mk(q[0]), mk(q[1]), mk(q[2]), mk(q[0]), mk(q[2]), mk(q[3])]);
                    }
                }
            }
            self.clouds_mesh.upload(&v, gl::STATIC_DRAW);
        }
        let s = self.world_shader;
        s.bind();
        s.set_mat4("u_mvp", &mvp.0);
        s.set_f("u_fixed_light", 1.0);
        s.set_v4("u_color_mul", [color[0], color[1], color[2], 1.0]);
        unsafe { gl::glUniform2f(s.loc("u_fog"), fog_far * 0.8, fog_far * 2.2) };
        let off = [ox as f32 * cell - cxw, cloud_y - cam.y, oz as f32 * cell - cam.z];
        s.set_v3("u_offset", off);
        self.white.bind();
        unsafe {
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            // depth pre-pass so overlapping faces don't double-blend
            gl::glColorMask(0, 0, 0, 0);
            self.clouds_mesh.draw();
            gl::glColorMask(1, 1, 1, 1);
            gl::glDepthFunc(gl::LEQUAL);
            self.clouds_mesh.draw();
            gl::glDisable(gl::BLEND);
        }
        s.set_v3("u_offset", [0.0; 3]);
        s.set_v4("u_color_mul", [1.0; 4]);
        s.set_f("u_fixed_light", -1.0);
    }

    pub fn block_uv(&self, name: &str) -> (f32, f32, f32) {
        let t = *self.block_tile.get(name).unwrap_or(&0);
        self.tiles.uv(t)
    }
    pub fn block_tile_uv(&self, tile: u16) -> (f32, f32, f32) {
        self.tiles.uv(tile)
    }
}
