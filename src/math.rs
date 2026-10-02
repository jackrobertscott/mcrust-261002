//! Small linear algebra helpers.
#![allow(dead_code)]

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const ZERO: Vec3 = v3(0.0, 0.0, 0.0);
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> Vec3 {
        let l = self.len();
        if l > 1e-6 { self * (1.0 / l) } else { self }
    }
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
    pub fn floor(self) -> (i32, i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

/// Column-major 4x4 matrix (OpenGL convention).
#[derive(Clone, Copy, Debug)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub fn identity() -> Mat4 {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        Mat4(m)
    }
    pub fn perspective(fovy_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fovy_rad / 2.0).tan();
        let mut m = [0.0; 16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) / (near - far);
        m[11] = -1.0;
        m[14] = 2.0 * far * near / (near - far);
        Mat4(m)
    }
    pub fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Mat4 {
        let mut m = Mat4::identity().0;
        m[0] = 2.0 / (r - l);
        m[5] = 2.0 / (t - b);
        m[10] = -2.0 / (f - n);
        m[12] = -(r + l) / (r - l);
        m[13] = -(t + b) / (t - b);
        m[14] = -(f + n) / (f - n);
        Mat4(m)
    }
    pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Mat4::identity();
        m.0[12] = x;
        m.0[13] = y;
        m.0[14] = z;
        m
    }
    pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Mat4::identity();
        m.0[0] = x;
        m.0[5] = y;
        m.0[10] = z;
        m
    }
    pub fn rot_x(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut m = Mat4::identity();
        m.0[5] = c;
        m.0[6] = s;
        m.0[9] = -s;
        m.0[10] = c;
        m
    }
    pub fn rot_y(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut m = Mat4::identity();
        m.0[0] = c;
        m.0[2] = -s;
        m.0[8] = s;
        m.0[10] = c;
        m
    }
    pub fn rot_z(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut m = Mat4::identity();
        m.0[0] = c;
        m.0[1] = s;
        m.0[4] = -s;
        m.0[5] = c;
        m
    }
    pub fn transform(&self, p: Vec3) -> Vec3 {
        let m = &self.0;
        v3(
            m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
            m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
            m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
        )
    }
    /// Transform returning homogeneous w as well.
    pub fn transform4(&self, p: Vec3) -> [f32; 4] {
        let m = &self.0;
        [
            m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
            m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
            m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
            m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15],
        ]
    }
}

impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let a = &self.0;
        let b = &o.0;
        let mut r = [0.0; 16];
        for c in 0..4 {
            for row in 0..4 {
                let mut s = 0.0;
                for k in 0..4 {
                    s += a[k * 4 + row] * b[c * 4 + k];
                }
                r[c * 4 + row] = s;
            }
        }
        Mat4(r)
    }
}

/// Frustum planes extracted from a view-projection matrix.
pub struct Frustum {
    planes: [[f32; 4]; 6],
}

impl Frustum {
    pub fn from_matrix(m: &Mat4) -> Frustum {
        let m = &m.0;
        let row = |i: usize| [m[i], m[4 + i], m[8 + i], m[12 + i]];
        let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
        let mut planes = [[0.0; 4]; 6];
        for k in 0..4 {
            planes[0][k] = r3[k] + r0[k];
            planes[1][k] = r3[k] - r0[k];
            planes[2][k] = r3[k] + r1[k];
            planes[3][k] = r3[k] - r1[k];
            planes[4][k] = r3[k] + r2[k];
            planes[5][k] = r3[k] - r2[k];
        }
        Frustum { planes }
    }
    pub fn aabb_visible(&self, min: Vec3, max: Vec3) -> bool {
        for p in &self.planes {
            let x = if p[0] >= 0.0 { max.x } else { min.x };
            let y = if p[1] >= 0.0 { max.y } else { min.y };
            let z = if p[2] >= 0.0 { max.z } else { min.z };
            if p[0] * x + p[1] * y + p[2] * z + p[3] < 0.0 {
                return false;
            }
        }
        true
    }
}

pub fn clamp(v: f32, a: f32, b: f32) -> f32 {
    v.max(a).min(b)
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
/// Wrap an angle in degrees into [-180,180)
pub fn wrap_deg(a: f32) -> f32 {
    let mut a = a % 360.0;
    if a >= 180.0 {
        a -= 360.0;
    }
    if a < -180.0 {
        a += 360.0;
    }
    a
}
