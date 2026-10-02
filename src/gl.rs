//! Minimal OpenGL 2.1 bindings (macOS OpenGL.framework exports these directly)
//! plus small helpers for shaders, textures and vertex buffers.
#![allow(non_snake_case, dead_code, clippy::missing_safety_doc)]

use std::ffi::{c_char, c_void, CString};

pub type GLenum = u32;
pub type GLuint = u32;
pub type GLint = i32;
pub type GLsizei = i32;
pub type GLfloat = f32;
pub type GLboolean = u8;
pub type GLbitfield = u32;

pub const DEPTH_BUFFER_BIT: u32 = 0x0100;
pub const COLOR_BUFFER_BIT: u32 = 0x4000;
pub const TRIANGLES: u32 = 0x0004;
pub const LINES: u32 = 0x0001;
pub const DEPTH_TEST: u32 = 0x0B71;
pub const CULL_FACE: u32 = 0x0B44;
pub const BLEND: u32 = 0x0BE2;
pub const BACK: u32 = 0x0405;
pub const FRONT: u32 = 0x0404;
pub const CCW: u32 = 0x0901;
pub const LEQUAL: u32 = 0x0203;
pub const LESS: u32 = 0x0201;
pub const ALWAYS: u32 = 0x0207;
pub const SRC_ALPHA: u32 = 0x0302;
pub const ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
pub const ONE: u32 = 1;
pub const ZERO: u32 = 0;
pub const ONE_MINUS_DST_COLOR: u32 = 0x0307;
pub const ONE_MINUS_SRC_COLOR: u32 = 0x0301;
pub const DST_COLOR: u32 = 0x0306;
pub const SRC_COLOR: u32 = 0x0300;
pub const TEXTURE_2D: u32 = 0x0DE1;
pub const TEXTURE0: u32 = 0x84C0;
pub const RGBA: u32 = 0x1908;
pub const UNSIGNED_BYTE: u32 = 0x1401;
pub const FLOAT: u32 = 0x1406;
pub const UNSIGNED_SHORT: u32 = 0x1403;
pub const UNSIGNED_INT: u32 = 0x1405;
pub const TEXTURE_MAG_FILTER: u32 = 0x2800;
pub const TEXTURE_MIN_FILTER: u32 = 0x2801;
pub const TEXTURE_WRAP_S: u32 = 0x2802;
pub const TEXTURE_WRAP_T: u32 = 0x2803;
pub const NEAREST: i32 = 0x2600;
pub const LINEAR: i32 = 0x2601;
pub const NEAREST_MIPMAP_LINEAR: i32 = 0x2702;
pub const NEAREST_MIPMAP_NEAREST: i32 = 0x2700;
pub const REPEAT: i32 = 0x2901;
pub const CLAMP_TO_EDGE: i32 = 0x812F;
pub const TEXTURE_MAX_LEVEL: u32 = 0x813D;
pub const GENERATE_MIPMAP: u32 = 0x8191;
pub const ARRAY_BUFFER: u32 = 0x8892;
pub const ELEMENT_ARRAY_BUFFER: u32 = 0x8893;
pub const STATIC_DRAW: u32 = 0x88E4;
pub const DYNAMIC_DRAW: u32 = 0x88E8;
pub const STREAM_DRAW: u32 = 0x88E0;
pub const VERTEX_SHADER: u32 = 0x8B31;
pub const FRAGMENT_SHADER: u32 = 0x8B30;
pub const COMPILE_STATUS: u32 = 0x8B81;
pub const LINK_STATUS: u32 = 0x8B82;
pub const POLYGON_OFFSET_FILL: u32 = 0x8037;
pub const LINE_SMOOTH: u32 = 0x0B20;
pub const SCISSOR_TEST: u32 = 0x0C11;

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    pub fn glClear(mask: GLbitfield);
    pub fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    pub fn glEnable(cap: GLenum);
    pub fn glDisable(cap: GLenum);
    pub fn glViewport(x: GLint, y: GLint, w: GLsizei, h: GLsizei);
    pub fn glScissor(x: GLint, y: GLint, w: GLsizei, h: GLsizei);
    pub fn glBlendFunc(s: GLenum, d: GLenum);
    pub fn glDepthFunc(f: GLenum);
    pub fn glDepthMask(f: GLboolean);
    pub fn glColorMask(r: GLboolean, g: GLboolean, b: GLboolean, a: GLboolean);
    pub fn glCullFace(m: GLenum);
    pub fn glFrontFace(m: GLenum);
    pub fn glLineWidth(w: f32);
    pub fn glPolygonOffset(f: f32, u: f32);
    pub fn glGetError() -> GLenum;
    pub fn glFlush();
    pub fn glGenTextures(n: GLsizei, t: *mut GLuint);
    pub fn glDeleteTextures(n: GLsizei, t: *const GLuint);
    pub fn glBindTexture(target: GLenum, t: GLuint);
    pub fn glActiveTexture(t: GLenum);
    pub fn glTexImage2D(target: GLenum, level: GLint, internal: GLint, w: GLsizei, h: GLsizei, border: GLint, format: GLenum, ty: GLenum, data: *const c_void);
    pub fn glTexSubImage2D(target: GLenum, level: GLint, x: GLint, y: GLint, w: GLsizei, h: GLsizei, format: GLenum, ty: GLenum, data: *const c_void);
    pub fn glTexParameteri(target: GLenum, p: GLenum, v: GLint);
    pub fn glGenBuffers(n: GLsizei, b: *mut GLuint);
    pub fn glDeleteBuffers(n: GLsizei, b: *const GLuint);
    pub fn glBindBuffer(target: GLenum, b: GLuint);
    pub fn glBufferData(target: GLenum, size: isize, data: *const c_void, usage: GLenum);
    pub fn glBufferSubData(target: GLenum, offset: isize, size: isize, data: *const c_void);
    pub fn glCreateShader(ty: GLenum) -> GLuint;
    pub fn glShaderSource(s: GLuint, n: GLsizei, src: *const *const c_char, len: *const GLint);
    pub fn glCompileShader(s: GLuint);
    pub fn glGetShaderiv(s: GLuint, p: GLenum, out: *mut GLint);
    pub fn glGetShaderInfoLog(s: GLuint, max: GLsizei, len: *mut GLsizei, log: *mut c_char);
    pub fn glCreateProgram() -> GLuint;
    pub fn glAttachShader(p: GLuint, s: GLuint);
    pub fn glLinkProgram(p: GLuint);
    pub fn glGetProgramiv(p: GLuint, pname: GLenum, out: *mut GLint);
    pub fn glGetProgramInfoLog(p: GLuint, max: GLsizei, len: *mut GLsizei, log: *mut c_char);
    pub fn glUseProgram(p: GLuint);
    pub fn glBindAttribLocation(p: GLuint, idx: GLuint, name: *const c_char);
    pub fn glGetUniformLocation(p: GLuint, name: *const c_char) -> GLint;
    pub fn glUniform1i(l: GLint, v: GLint);
    pub fn glUniform1f(l: GLint, v: f32);
    pub fn glUniform2f(l: GLint, a: f32, b: f32);
    pub fn glUniform3f(l: GLint, a: f32, b: f32, c: f32);
    pub fn glUniform4f(l: GLint, a: f32, b: f32, c: f32, d: f32);
    pub fn glUniformMatrix4fv(l: GLint, n: GLsizei, transpose: GLboolean, v: *const f32);
    pub fn glVertexAttribPointer(idx: GLuint, size: GLint, ty: GLenum, norm: GLboolean, stride: GLsizei, ptr: *const c_void);
    pub fn glEnableVertexAttribArray(idx: GLuint);
    pub fn glDisableVertexAttribArray(idx: GLuint);
    pub fn glDrawArrays(mode: GLenum, first: GLint, count: GLsizei);
    pub fn glDrawElements(mode: GLenum, count: GLsizei, ty: GLenum, idx: *const c_void);
    pub fn glReadPixels(x: GLint, y: GLint, w: GLsizei, h: GLsizei, format: GLenum, ty: GLenum, data: *mut c_void);
    pub fn glFinish();
}

/// Vertex format shared by every mesh in the game.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    /// Tint colour (rgb) + alpha.
    pub color: [u8; 4],
    /// [sky light 0-255, block light 0-255, shade 0-255, unused]
    pub light: [u8; 4],
}

pub const ATTR_POS: u32 = 0;
pub const ATTR_UV: u32 = 1;
pub const ATTR_COLOR: u32 = 2;
pub const ATTR_LIGHT: u32 = 3;

#[derive(Clone, Copy)]
pub struct Shader {
    pub prog: GLuint,
}

impl Shader {
    pub fn new(vs: &str, fs: &str) -> Shader {
        unsafe {
            let compile = |ty: GLenum, src: &str| -> GLuint {
                let s = glCreateShader(ty);
                let c = CString::new(src).unwrap();
                let p = c.as_ptr();
                glShaderSource(s, 1, &p, std::ptr::null());
                glCompileShader(s);
                let mut ok = 0;
                glGetShaderiv(s, COMPILE_STATUS, &mut ok);
                if ok == 0 {
                    let mut buf = vec![0u8; 4096];
                    let mut len = 0;
                    glGetShaderInfoLog(s, 4096, &mut len, buf.as_mut_ptr() as *mut c_char);
                    panic!("shader compile error: {}", String::from_utf8_lossy(&buf[..len as usize]));
                }
                s
            };
            let v = compile(VERTEX_SHADER, vs);
            let f = compile(FRAGMENT_SHADER, fs);
            let prog = glCreateProgram();
            glAttachShader(prog, v);
            glAttachShader(prog, f);
            for (i, n) in ["a_pos", "a_uv", "a_color", "a_light"].iter().enumerate() {
                let c = CString::new(*n).unwrap();
                glBindAttribLocation(prog, i as u32, c.as_ptr());
            }
            glLinkProgram(prog);
            let mut ok = 0;
            glGetProgramiv(prog, LINK_STATUS, &mut ok);
            if ok == 0 {
                let mut buf = vec![0u8; 4096];
                let mut len = 0;
                glGetProgramInfoLog(prog, 4096, &mut len, buf.as_mut_ptr() as *mut c_char);
                panic!("shader link error: {}", String::from_utf8_lossy(&buf[..len as usize]));
            }
            Shader { prog }
        }
    }
    pub fn bind(&self) {
        unsafe { glUseProgram(self.prog) }
    }
    pub fn loc(&self, name: &str) -> GLint {
        let c = CString::new(name).unwrap();
        unsafe { glGetUniformLocation(self.prog, c.as_ptr()) }
    }
    pub fn set_mat4(&self, name: &str, m: &[f32; 16]) {
        unsafe { glUniformMatrix4fv(self.loc(name), 1, 0, m.as_ptr()) }
    }
    pub fn set_f(&self, name: &str, v: f32) {
        unsafe { glUniform1f(self.loc(name), v) }
    }
    pub fn set_i(&self, name: &str, v: i32) {
        unsafe { glUniform1i(self.loc(name), v) }
    }
    pub fn set_v3(&self, name: &str, v: [f32; 3]) {
        unsafe { glUniform3f(self.loc(name), v[0], v[1], v[2]) }
    }
    pub fn set_v4(&self, name: &str, v: [f32; 4]) {
        unsafe { glUniform4f(self.loc(name), v[0], v[1], v[2], v[3]) }
    }
}

pub struct Texture {
    pub id: GLuint,
    pub w: u32,
    pub h: u32,
}

impl Texture {
    pub fn from_image(img: &crate::image::Image) -> Texture {
        let mut id = 0;
        unsafe {
            glGenTextures(1, &mut id);
            glBindTexture(TEXTURE_2D, id);
            glTexParameteri(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST);
            glTexParameteri(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST);
            glTexParameteri(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE);
            glTexParameteri(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE);
            glTexImage2D(TEXTURE_2D, 0, RGBA as i32, img.w as i32, img.h as i32, 0, RGBA, UNSIGNED_BYTE, img.data.as_ptr() as *const c_void);
        }
        Texture { id, w: img.w as u32, h: img.h as u32 }
    }
    pub fn set_repeat(&self) {
        unsafe {
            glBindTexture(TEXTURE_2D, self.id);
            glTexParameteri(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT);
            glTexParameteri(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT);
        }
    }
    pub fn update(&self, img: &crate::image::Image) {
        unsafe {
            glBindTexture(TEXTURE_2D, self.id);
            glTexSubImage2D(TEXTURE_2D, 0, 0, 0, img.w as i32, img.h as i32, RGBA, UNSIGNED_BYTE, img.data.as_ptr() as *const c_void);
        }
    }
    pub fn bind(&self) {
        unsafe { glBindTexture(TEXTURE_2D, self.id) }
    }
}

/// A vertex buffer holding triangles in the shared `Vertex` format.
pub struct Mesh {
    pub vbo: GLuint,
    pub count: i32,
    capacity: usize,
}

impl Mesh {
    pub fn new() -> Mesh {
        let mut vbo = 0;
        unsafe { glGenBuffers(1, &mut vbo) };
        Mesh { vbo, count: 0, capacity: 0 }
    }
    pub fn upload(&mut self, verts: &[Vertex], usage: GLenum) {
        unsafe {
            glBindBuffer(ARRAY_BUFFER, self.vbo);
            let bytes = std::mem::size_of_val(verts);
            if bytes > self.capacity || usage == STATIC_DRAW {
                glBufferData(ARRAY_BUFFER, bytes as isize, verts.as_ptr() as *const c_void, usage);
                self.capacity = bytes;
            } else {
                // orphan + reupload for streaming
                glBufferData(ARRAY_BUFFER, self.capacity as isize, std::ptr::null(), usage);
                glBufferSubData(ARRAY_BUFFER, 0, bytes as isize, verts.as_ptr() as *const c_void);
            }
        }
        self.count = verts.len() as i32;
    }
    pub fn draw(&self) {
        self.draw_mode(TRIANGLES);
    }
    pub fn draw_mode(&self, mode: GLenum) {
        if self.count == 0 {
            return;
        }
        unsafe {
            glBindBuffer(ARRAY_BUFFER, self.vbo);
            let stride = std::mem::size_of::<Vertex>() as i32;
            glVertexAttribPointer(ATTR_POS, 3, FLOAT, 0, stride, 0 as *const c_void);
            glVertexAttribPointer(ATTR_UV, 2, FLOAT, 0, stride, 12 as *const c_void);
            glVertexAttribPointer(ATTR_COLOR, 4, UNSIGNED_BYTE, 1, stride, 20 as *const c_void);
            glVertexAttribPointer(ATTR_LIGHT, 4, UNSIGNED_BYTE, 1, stride, 24 as *const c_void);
            glDrawArrays(mode, 0, self.count);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe { glDeleteBuffers(1, &self.vbo) };
    }
}

pub fn enable_vertex_attribs() {
    unsafe {
        for i in 0..4 {
            glEnableVertexAttribArray(i);
        }
    }
}

/// Read back the current framebuffer as an image (top row first).
pub fn screenshot(w: u32, h: u32) -> crate::image::Image {
    let mut px = vec![[0u8; 4]; (w * h) as usize];
    unsafe {
        glFinish();
        glReadPixels(0, 0, w as i32, h as i32, RGBA, UNSIGNED_BYTE, px.as_mut_ptr() as *mut c_void);
    }
    let mut img = crate::image::Image::new(w as usize, h as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let mut c = px[(h as usize - 1 - y) * w as usize + x];
            c[3] = 255;
            img.data[y * w as usize + x] = c;
        }
    }
    img
}
