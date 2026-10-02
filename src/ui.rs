//! Immediate-mode 2D drawing in GUI-pixel coordinates (vanilla-style scaled GUI).

use crate::block::{self, Shape};
use crate::gl::{self, Mesh, Vertex};
use crate::item::{self, ItemStack};
use crate::math::Mat4;
use crate::render::Renderer;

pub const WHITE: [u8; 4] = [255, 255, 255, 255];

pub struct Ui {
    verts: Vec<Vertex>,
    tex: u32,
    tex_size: (f32, f32),
    mesh: Mesh,
    /// GUI-space size
    pub w: f32,
    pub h: f32,
    pub scale: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

impl Ui {
    pub fn new() -> Ui {
        Ui { verts: Vec::new(), tex: 0, tex_size: (1.0, 1.0), mesh: Mesh::new(), w: 1.0, h: 1.0, scale: 1.0, mouse_x: 0.0, mouse_y: 0.0 }
    }

    pub fn begin(&mut self, r: &Renderer, mouse_px: (f32, f32)) {
        self.scale = r.gui_scale as f32;
        self.w = (r.width as f32 / self.scale).ceil();
        self.h = (r.height as f32 / self.scale).ceil();
        self.mouse_x = mouse_px.0 / self.scale;
        self.mouse_y = mouse_px.1 / self.scale;
        let s = &r.gui_shader;
        s.bind();
        let m = Mat4::ortho(0.0, r.width as f32 / self.scale, r.height as f32 / self.scale, 0.0, -100.0, 100.0);
        s.set_mat4("u_mvp", &m.0);
        s.set_i("u_tex", 0);
        unsafe {
            gl::glViewport(0, 0, r.width as i32, r.height as i32);
            gl::glDisable(gl::DEPTH_TEST);
            gl::glDisable(gl::CULL_FACE);
            gl::glEnable(gl::BLEND);
            gl::glBlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.verts.clear();
        self.tex = 0;
    }

    pub fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        unsafe { gl::glBindTexture(gl::TEXTURE_2D, self.tex) };
        self.mesh.upload(&self.verts, gl::STREAM_DRAW);
        self.mesh.draw();
        self.verts.clear();
    }

    pub fn end(&mut self) {
        self.flush();
        unsafe {
            gl::glDisable(gl::BLEND);
            gl::glEnable(gl::DEPTH_TEST);
            gl::glEnable(gl::CULL_FACE);
        }
    }

    fn use_tex(&mut self, id: u32, w: u32, h: u32) {
        if self.tex != id {
            self.flush();
            self.tex = id;
            self.tex_size = (w as f32, h as f32);
        }
    }

    /// Raw textured quad with UVs in texture pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn quad_px(&mut self, x: f32, y: f32, w: f32, h: f32, u: f32, v: f32, uw: f32, vh: f32, c: [u8; 4]) {
        let (tw, th) = self.tex_size;
        let (u0, v0, u1, v1) = (u / tw, v / th, (u + uw) / tw, (v + vh) / th);
        self.quad_uv(x, y, w, h, [u0, v0, u1, v1], c);
    }

    pub fn quad_uv(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], c: [u8; 4]) {
        let mk = |px: f32, py: f32, u: f32, v: f32| Vertex { pos: [px, py, 0.0], uv: [u, v], color: c, light: [0; 4] };
        let a = mk(x, y, uv[0], uv[1]);
        let b = mk(x + w, y, uv[2], uv[1]);
        let cc = mk(x + w, y + h, uv[2], uv[3]);
        let d = mk(x, y + h, uv[0], uv[3]);
        self.verts.extend_from_slice(&[a, b, cc, a, cc, d]);
    }

    pub fn rect(&mut self, r: &Renderer, x: f32, y: f32, w: f32, h: f32, c: [u8; 4]) {
        self.use_tex(r.white.id, 4, 4);
        self.quad_uv(x, y, w, h, [0.25, 0.25, 0.75, 0.75], c);
    }

    pub fn gradient(&mut self, r: &Renderer, x: f32, y: f32, w: f32, h: f32, top: [u8; 4], bot: [u8; 4]) {
        self.use_tex(r.white.id, 4, 4);
        let mk = |px: f32, py: f32, c: [u8; 4]| Vertex { pos: [px, py, 0.0], uv: [0.5, 0.5], color: c, light: [0; 4] };
        let (a, b, c, d) = (mk(x, y, top), mk(x + w, y, top), mk(x + w, y + h, bot), mk(x, y + h, bot));
        self.verts.extend_from_slice(&[a, b, c, a, c, d]);
    }

    pub fn sprite(&mut self, r: &Renderer, name: &str, x: f32, y: f32) {
        self.sprite_c(r, name, x, y, WHITE);
    }
    pub fn sprite_c(&mut self, r: &Renderer, name: &str, x: f32, y: f32, c: [u8; 4]) {
        if let Some(rc) = r.gui_rects.get(name).copied() {
            self.use_tex(r.gui_atlas.id, r.gui_atlas.w, r.gui_atlas.h);
            self.quad_px(x, y, rc.w, rc.h, rc.x, rc.y, rc.w, rc.h, c);
        }
    }
    /// Part of a GUI sprite: (sx,sy,sw,sh) relative to the sprite.
    #[allow(clippy::too_many_arguments)]
    pub fn sprite_part(&mut self, r: &Renderer, name: &str, x: f32, y: f32, sx: f32, sy: f32, sw: f32, sh: f32) {
        if let Some(rc) = r.gui_rects.get(name).copied() {
            self.use_tex(r.gui_atlas.id, r.gui_atlas.w, r.gui_atlas.h);
            self.quad_px(x, y, sw, sh, rc.x + sx, rc.y + sy, sw, sh, WHITE);
        }
    }
    pub fn sprite_size(&self, r: &Renderer, name: &str) -> (f32, f32) {
        r.gui_rects.get(name).map(|rc| (rc.w, rc.h)).unwrap_or((0.0, 0.0))
    }

    /// Tiled background texture (from block atlas) darkened like the vanilla options screen.
    pub fn dirt_background(&mut self, r: &Renderer, brightness: u8) {
        let (u0, v0, s) = r.block_uv("dirt");
        self.use_tex(r.block_atlas.id, r.block_atlas.w, r.block_atlas.h);
        let tile = 32.0; // vanilla draws the 16px texture at 2x GUI scale... 32 gui px per tile
        let c = [brightness, brightness, brightness, 255];
        let nx = (self.w / tile).ceil() as i32 + 1;
        let ny = (self.h / tile).ceil() as i32 + 1;
        for j in 0..ny {
            for i in 0..nx {
                self.quad_uv(i as f32 * tile, j as f32 * tile, tile, tile, [u0, v0, u0 + s, v0 + s], c);
            }
        }
    }

    // ---------------- text ----------------

    pub fn text_width(&self, r: &Renderer, s: &str) -> f32 {
        let mut w = 0;
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '§' {
                chars.next();
                continue;
            }
            let i = (c as u32).min(255) as usize;
            w += r.glyph_w[i];
        }
        w as f32
    }

    fn glyphs(&mut self, r: &Renderer, s: &str, x: f32, y: f32, c: [u8; 4], scale: f32) {
        self.use_tex(r.font.id, r.font.w, r.font.h);
        let mut cx = x;
        let mut col = c;
        let mut chars = s.chars();
        while let Some(ch) = chars.next() {
            if ch == '§' {
                if let Some(code) = chars.next() {
                    col = format_color(code, c);
                }
                continue;
            }
            let code = if (ch as u32) < 128 { ch as u32 } else { '?' as u32 };
            let gx = (code % 16) as f32 * 8.0;
            let gy = (code / 16) as f32 * 8.0;
            if ch != ' ' {
                self.quad_px(cx, y, 8.0 * scale, 8.0 * scale, gx, gy, 8.0, 8.0, col);
            }
            cx += r.glyph_w[code as usize] as f32 * scale;
        }
    }

    /// Text with the vanilla drop shadow (offset 1px, colour/4).
    pub fn text(&mut self, r: &Renderer, s: &str, x: f32, y: f32, c: [u8; 4]) {
        let sh = [c[0] / 4, c[1] / 4, c[2] / 4, c[3]];
        // Shadow uses darkened format colours too
        let shadow: String = s.to_string();
        self.glyphs_shadow(r, &shadow, x + 1.0, y + 1.0, sh, 1.0);
        self.glyphs(r, s, x, y, c, 1.0);
    }
    fn glyphs_shadow(&mut self, r: &Renderer, s: &str, x: f32, y: f32, c: [u8; 4], scale: f32) {
        self.use_tex(r.font.id, r.font.w, r.font.h);
        let mut cx = x;
        let mut col = c;
        let mut chars = s.chars();
        while let Some(ch) = chars.next() {
            if ch == '§' {
                if let Some(code) = chars.next() {
                    let f = format_color(code, [c[0] * 4, c[1] * 4, c[2] * 4, c[3]]);
                    col = [f[0] / 4, f[1] / 4, f[2] / 4, f[3]];
                }
                continue;
            }
            let code = if (ch as u32) < 128 { ch as u32 } else { '?' as u32 };
            if ch != ' ' {
                self.quad_px(cx, y, 8.0 * scale, 8.0 * scale, (code % 16) as f32 * 8.0, (code / 16) as f32 * 8.0, 8.0, 8.0, col);
            }
            cx += r.glyph_w[code as usize] as f32 * scale;
        }
    }
    pub fn text_no_shadow(&mut self, r: &Renderer, s: &str, x: f32, y: f32, c: [u8; 4]) {
        self.glyphs(r, s, x, y, c, 1.0);
    }
    pub fn text_centered(&mut self, r: &Renderer, s: &str, cx: f32, y: f32, c: [u8; 4]) {
        let w = self.text_width(r, s);
        self.text(r, s, (cx - w / 2.0).floor(), y, c);
    }
    pub fn text_scaled(&mut self, r: &Renderer, s: &str, x: f32, y: f32, c: [u8; 4], scale: f32) {
        let sh = [c[0] / 4, c[1] / 4, c[2] / 4, c[3]];
        self.glyphs_shadow(r, s, x + scale, y + scale, sh, scale);
        self.glyphs(r, s, x, y, c, scale);
    }

    // ---------------- vanilla widgets ----------------

    /// Vanilla button. Returns true if hovered.
    pub fn button(&mut self, r: &Renderer, label: &str, x: f32, y: f32, w: f32, h: f32, enabled: bool) -> bool {
        let hovered = enabled && self.hit(x, y, w, h);
        let name = if !enabled { "widgets_button_disabled" } else if hovered { "widgets_button_hover" } else { "widgets_button" };
        // draw left half and right half so any width works (like vanilla)
        let hw = (w / 2.0).floor();
        let sh = h.min(20.0);
        self.sprite_part(r, name, x, y, 0.0, 0.0, hw, sh / 2.0);
        self.sprite_part(r, name, x + hw, y, 200.0 - (w - hw), 0.0, w - hw, sh / 2.0);
        self.sprite_part(r, name, x, y + sh / 2.0, 0.0, 20.0 - sh / 2.0, hw, sh / 2.0);
        self.sprite_part(r, name, x + hw, y + sh / 2.0, 200.0 - (w - hw), 20.0 - sh / 2.0, w - hw, sh / 2.0);
        let col = if !enabled { [160, 160, 160, 255] } else if hovered { [255, 255, 160, 255] } else { [224, 224, 224, 255] };
        self.text_centered(r, label, x + w / 2.0, y + (h - 8.0) / 2.0, col);
        hovered
    }

    pub fn hit(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.mouse_x >= x && self.mouse_x < x + w && self.mouse_y >= y && self.mouse_y < y + h
    }

    // ---------------- items ----------------

    pub fn item(&mut self, r: &Renderer, st: &ItemStack, x: f32, y: f32) {
        if st.is_empty() {
            return;
        }
        self.item_icon(r, st.id, x, y);
        // durability bar
        let maxd = item::max_damage(st.id);
        if maxd > 0 && st.damage > 0 {
            let frac = 1.0 - st.damage as f32 / maxd as f32;
            let w = (13.0 * frac).round();
            let hue = frac / 3.0; // red..green
            let c = hsv(hue, 1.0, 1.0);
            self.rect(r, x + 2.0, y + 13.0, 13.0, 2.0, [0, 0, 0, 255]);
            self.rect(r, x + 2.0, y + 13.0, w, 1.0, c);
        }
        if st.count > 1 {
            let s = st.count.to_string();
            let w = self.text_width(r, &s);
            self.text(r, &s, x + 17.0 - w, y + 9.0, WHITE);
        }
    }

    pub fn item_icon(&mut self, r: &Renderer, id: item::ItemId, x: f32, y: f32) {
        if let Some(tex) = item::texture(id) {
            if let Some(bn) = tex.strip_prefix("b:") {
                let (u0, v0, s) = r.block_uv(bn);
                self.use_tex(r.block_atlas.id, r.block_atlas.w, r.block_atlas.h);
                self.quad_uv(x, y, 16.0, 16.0, [u0, v0, u0 + s, v0 + s], WHITE);
            } else {
                let t = *r.item_tile.get(tex).unwrap_or(&0);
                let (u0, v0, s) = r.tiles_uv_items(t);
                self.use_tex(r.item_atlas.id, r.item_atlas.w, r.item_atlas.h);
                self.quad_uv(x, y, 16.0, 16.0, [u0, v0, u0 + s, v0 + s], WHITE);
            }
            return;
        }
        if id < 256 {
            self.block_icon(r, id as u8, x, y);
        }
    }

    /// Isometric 3D block icon as drawn in vanilla inventories.
    pub fn block_icon(&mut self, r: &Renderer, b: u8, x: f32, y: f32) {
        let d = block::def(b);
        let t = &r.tiles.blocks[b as usize];
        let (top_t, side_t, front_t) = (t[0], t[2], t[3]);
        self.use_tex(r.block_atlas.id, r.block_atlas.w, r.block_atlas.h);
        let tint = match d.tint {
            block::Tint::Grass => 0x91BD59,
            block::Tint::Foliage => 0x77AB2F,
            block::Tint::Fixed(c) => c,
            block::Tint::None => 0xFFFFFF,
        };
        let tc = |m: f32, tinted: bool| -> [u8; 4] {
            let c = if tinted { tint } else { 0xFFFFFF };
            [(((c >> 16) & 255) as f32 * m) as u8, (((c >> 8) & 255) as f32 * m) as u8, ((c & 255) as f32 * m) as u8, 255]
        };
        let leaf_tint = d.tint != block::Tint::None;
        let h = match d.shape {
            Shape::Layer => 2.0 / 16.0,
            Shape::Farmland => 15.0 / 16.0,
            _ => 1.0,
        };
        // Isometric projection: cube corners
        let cx = x + 8.0;
        let top_y = y + 1.0;
        let sx = 7.0; // half width
        let sy = 3.6; // quarter-height of top rhombus
        let vh = 8.4 * h; // side height
        let ty = top_y + (1.0 - h) * 8.4;
        // top face points: back, right, front, left
        let pb = (cx, ty);
        let pr = (cx + sx, ty + sy);
        let pf = (cx, ty + 2.0 * sy);
        let pl = (cx - sx, ty + sy);
        let face = |me: &mut Ui, tile: u16, pts: [(f32, f32); 4], c: [u8; 4], vfrac: f32| {
            let (u0, v0, s) = r.block_tile_uv(tile);
            let uvs = [[u0, v0 + s * (1.0 - vfrac)], [u0 + s, v0 + s * (1.0 - vfrac)], [u0 + s, v0 + s], [u0, v0 + s]];
            let mk = |i: usize| Vertex { pos: [pts[i].0, pts[i].1, 0.0], uv: uvs[i], color: c, light: [0; 4] };
            me.verts.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
        };
        let grass = b == block::GRASS;
        // left face (front-left), slightly darker
        let left_tile = if d.front.is_some() { front_t } else { side_t };
        face(self, left_tile, [pl, pf, (pf.0, pf.1 + vh), (pl.0, pl.1 + vh)], tc(0.8, leaf_tint && !grass), h);
        face(self, side_t, [pf, pr, (pr.0, pr.1 + vh), (pf.0, pf.1 + vh)], tc(0.6, leaf_tint && !grass), h);
        if grass {
            let ov = r.tiles.grass_overlay;
            face(self, ov, [pl, pf, (pf.0, pf.1 + vh), (pl.0, pl.1 + vh)], tc(0.8, true), h);
            face(self, ov, [pf, pr, (pr.0, pr.1 + vh), (pf.0, pf.1 + vh)], tc(0.6, true), h);
        }
        // top
        let (u0, v0, s) = r.block_tile_uv(top_t);
        let c = tc(1.0, leaf_tint);
        let uvs = [[u0, v0], [u0 + s, v0], [u0 + s, v0 + s], [u0, v0 + s]];
        let pts = [pl, pb, pr, pf];
        let mk = |i: usize| Vertex { pos: [pts[i].0, pts[i].1, 0.0], uv: uvs[i], color: c, light: [0; 4] };
        self.verts.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
    }

    /// Tooltip box (vanilla style: dark purple-bordered).
    pub fn tooltip(&mut self, r: &Renderer, text: &str, mx: f32, my: f32) {
        let w = self.text_width(r, text);
        let x = (mx + 12.0).min(self.w - w - 4.0);
        let y = my - 12.0;
        let bg = [16, 0, 16, 240];
        self.rect(r, x - 3.0, y - 4.0, w + 6.0, 1.0, bg);
        self.rect(r, x - 3.0, y + 9.0, w + 6.0, 1.0, bg);
        self.rect(r, x - 3.0, y - 3.0, w + 6.0, 12.0, bg);
        self.rect(r, x - 4.0, y - 3.0, 1.0, 12.0, bg);
        self.rect(r, x + w + 3.0, y - 3.0, 1.0, 12.0, bg);
        let b1 = [80, 0, 255, 80];
        let b2 = [40, 0, 127, 80];
        self.gradient(r, x - 3.0, y - 2.0, 1.0, 10.0, b1, b2);
        self.gradient(r, x + w + 2.0, y - 2.0, 1.0, 10.0, b1, b2);
        self.rect(r, x - 3.0, y - 3.0, w + 6.0, 1.0, b1);
        self.rect(r, x - 3.0, y + 8.0, w + 6.0, 1.0, b2);
        self.text(r, text, x, y, WHITE);
    }
}

pub fn hsv(h: f32, s: f32, v: f32) -> [u8; 4] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]
}

/// Vanilla § formatting colours.
pub fn format_color(code: char, default: [u8; 4]) -> [u8; 4] {
    let c = match code.to_ascii_lowercase() {
        '0' => 0x000000,
        '1' => 0x0000AA,
        '2' => 0x00AA00,
        '3' => 0x00AAAA,
        '4' => 0xAA0000,
        '5' => 0xAA00AA,
        '6' => 0xFFAA00,
        '7' => 0xAAAAAA,
        '8' => 0x555555,
        '9' => 0x5555FF,
        'a' => 0x55FF55,
        'b' => 0x55FFFF,
        'c' => 0xFF5555,
        'd' => 0xFF55FF,
        'e' => 0xFFFF55,
        'f' => 0xFFFFFF,
        _ => return default,
    };
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, default[3]]
}

impl Renderer {
    pub fn tiles_uv_items(&self, t: u16) -> (f32, f32, f32) {
        let s = 1.0 / 16.0;
        ((t % 16) as f32 * s, (t / 16) as f32 * s, s)
    }
}
