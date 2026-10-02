//! Box models for mobs / the player, using vanilla model coordinates
//! (pixels, Y pointing down, pivots and texture offsets as in Java Edition).

use crate::gl::Vertex;
use crate::math::{v3, Mat4, Vec3};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct MBox {
    pub pos: [f32; 3],
    pub size: [f32; 3],
    pub uv: [f32; 2],
    pub inflate: f32,
    pub mirror: bool,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub pivot: [f32; 3],
    pub rot: [f32; 3],
    pub boxes: Vec<MBox>,
    pub visible: bool,
}

const fn bx(pos: [f32; 3], size: [f32; 3], uv: [f32; 2]) -> MBox {
    MBox { pos, size, uv, inflate: 0.0, mirror: false }
}
fn part(pivot: [f32; 3], boxes: Vec<MBox>) -> Part {
    Part { pivot, rot: [0.0; 3], boxes, visible: true }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ModelKind {
    Pig,
    Cow,
    Sheep,
    SheepFur,
    Chicken,
    Biped, // zombie / player (64x64)
    Skeleton,
    Creeper,
    Spider,
}

pub fn texture_size(k: ModelKind) -> (f32, f32) {
    match k {
        ModelKind::Biped => (64.0, 64.0),
        _ => (64.0, 32.0),
    }
}

/// Part indices for each model are documented in `build`.
pub fn build(k: ModelKind) -> Vec<Part> {
    match k {
        // 0 head, 1 body, 2..6 legs (RB, LB, RF, LF)
        ModelKind::Pig => vec![
            part([0.0, 12.0, -6.0], vec![bx([-4.0, -4.0, -8.0], [8.0, 8.0, 8.0], [0.0, 0.0]), bx([-2.0, 0.0, -9.0], [4.0, 3.0, 1.0], [16.0, 16.0])]),
            Part { rot: [PI / 2.0, 0.0, 0.0], ..part([0.0, 11.0, 2.0], vec![bx([-5.0, -10.0, -7.0], [10.0, 16.0, 8.0], [28.0, 8.0])]) },
            part([-3.0, 18.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([3.0, 18.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([-3.0, 18.0, -5.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([3.0, 18.0, -5.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
        ],
        ModelKind::Cow => vec![
            part(
                [0.0, 4.0, -8.0],
                vec![
                    bx([-4.0, -4.0, -6.0], [8.0, 8.0, 6.0], [0.0, 0.0]),
                    bx([-5.0, -5.0, -4.0], [1.0, 3.0, 1.0], [22.0, 0.0]),
                    bx([4.0, -5.0, -4.0], [1.0, 3.0, 1.0], [22.0, 0.0]),
                ],
            ),
            Part {
                rot: [PI / 2.0, 0.0, 0.0],
                ..part([0.0, 5.0, 2.0], vec![bx([-6.0, -10.0, -7.0], [12.0, 18.0, 10.0], [18.0, 4.0]), bx([-2.0, 2.0, -8.0], [4.0, 6.0, 1.0], [52.0, 0.0])])
            },
            part([-4.0, 12.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([4.0, 12.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([-4.0, 12.0, -6.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([4.0, 12.0, -6.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
        ],
        ModelKind::Sheep => vec![
            part([0.0, 6.0, -8.0], vec![bx([-3.0, -4.0, -6.0], [6.0, 6.0, 8.0], [0.0, 0.0])]),
            Part { rot: [PI / 2.0, 0.0, 0.0], ..part([0.0, 5.0, 2.0], vec![bx([-4.0, -10.0, -7.0], [8.0, 16.0, 6.0], [28.0, 8.0])]) },
            part([-3.0, 12.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([3.0, 12.0, 7.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([-3.0, 12.0, -5.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([3.0, 12.0, -5.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
        ],
        ModelKind::SheepFur => {
            let inf = |mut b: MBox, i: f32| {
                b.inflate = i;
                b
            };
            vec![
                part([0.0, 6.0, -8.0], vec![inf(bx([-3.0, -4.0, -4.0], [6.0, 6.0, 6.0], [0.0, 0.0]), 0.6)]),
                Part { rot: [PI / 2.0, 0.0, 0.0], ..part([0.0, 5.0, 2.0], vec![inf(bx([-4.0, -10.0, -7.0], [8.0, 16.0, 6.0], [28.0, 8.0]), 1.75)]) },
                part([-3.0, 12.0, 7.0], vec![inf(bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0]), 0.5)]),
                part([3.0, 12.0, 7.0], vec![inf(bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0]), 0.5)]),
                part([-3.0, 12.0, -5.0], vec![inf(bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0]), 0.5)]),
                part([3.0, 12.0, -5.0], vec![inf(bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0]), 0.5)]),
            ]
        }
        // 0 head(+beak+wattle), 1 body, 2 right leg, 3 left leg, 4 right wing, 5 left wing
        ModelKind::Chicken => vec![
            part(
                [0.0, 15.0, -4.0],
                vec![
                    bx([-2.0, -6.0, -2.0], [4.0, 6.0, 3.0], [0.0, 0.0]),
                    bx([-2.0, -4.0, -4.0], [4.0, 2.0, 2.0], [14.0, 0.0]),
                    bx([-1.0, -2.0, -3.0], [2.0, 2.0, 2.0], [14.0, 4.0]),
                ],
            ),
            Part { rot: [PI / 2.0, 0.0, 0.0], ..part([0.0, 16.0, 0.0], vec![bx([-3.0, -4.0, -3.0], [6.0, 8.0, 6.0], [0.0, 9.0])]) },
            part([-2.0, 19.0, 1.0], vec![bx([-1.0, 0.0, -3.0], [3.0, 5.0, 3.0], [26.0, 0.0])]),
            part([1.0, 19.0, 1.0], vec![bx([-1.0, 0.0, -3.0], [3.0, 5.0, 3.0], [26.0, 0.0])]),
            part([-4.0, 13.0, 0.0], vec![bx([0.0, 0.0, -3.0], [1.0, 4.0, 6.0], [24.0, 13.0])]),
            part([4.0, 13.0, 0.0], vec![bx([-1.0, 0.0, -3.0], [1.0, 4.0, 6.0], [24.0, 13.0])]),
        ],
        // 0 head, 1 body, 2 right arm, 3 left arm, 4 right leg, 5 left leg
        ModelKind::Biped => vec![
            part([0.0, 0.0, 0.0], vec![bx([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0])]),
            part([0.0, 0.0, 0.0], vec![bx([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], [16.0, 16.0])]),
            part([-5.0, 2.0, 0.0], vec![bx([-3.0, -2.0, -2.0], [4.0, 12.0, 4.0], [40.0, 16.0])]),
            part([5.0, 2.0, 0.0], vec![bx([-1.0, -2.0, -2.0], [4.0, 12.0, 4.0], [32.0, 48.0])]),
            part([-1.9, 12.0, 0.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 16.0])]),
            part([1.9, 12.0, 0.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [16.0, 48.0])]),
        ],
        ModelKind::Skeleton => vec![
            part([0.0, 0.0, 0.0], vec![bx([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0])]),
            part([0.0, 0.0, 0.0], vec![bx([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], [16.0, 16.0])]),
            part([-5.0, 2.0, 0.0], vec![bx([-1.0, -2.0, -1.0], [2.0, 12.0, 2.0], [40.0, 16.0])]),
            part([5.0, 2.0, 0.0], vec![MBox { mirror: true, ..bx([-1.0, -2.0, -1.0], [2.0, 12.0, 2.0], [40.0, 16.0]) }]),
            part([-2.0, 12.0, 0.0], vec![bx([-1.0, 0.0, -1.0], [2.0, 12.0, 2.0], [0.0, 16.0])]),
            part([2.0, 12.0, 0.0], vec![MBox { mirror: true, ..bx([-1.0, 0.0, -1.0], [2.0, 12.0, 2.0], [0.0, 16.0]) }]),
        ],
        // 0 head, 1 body, 2..6 legs
        ModelKind::Creeper => vec![
            part([0.0, 6.0, 0.0], vec![bx([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0])]),
            part([0.0, 6.0, 0.0], vec![bx([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], [16.0, 16.0])]),
            part([-2.0, 18.0, 4.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([2.0, 18.0, 4.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([-2.0, 18.0, -4.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
            part([2.0, 18.0, -4.0], vec![bx([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [0.0, 16.0])]),
        ],
        // 0 head, 1 neck, 2 body, 3..11 legs (1..8)
        ModelKind::Spider => {
            let mut v = vec![
                part([0.0, 15.0, -3.0], vec![bx([-4.0, -4.0, -8.0], [8.0, 8.0, 8.0], [32.0, 4.0])]),
                part([0.0, 15.0, 0.0], vec![bx([-3.0, -3.0, -3.0], [6.0, 6.0, 6.0], [0.0, 0.0])]),
                part([0.0, 15.0, 9.0], vec![bx([-5.0, -4.0, -6.0], [10.0, 8.0, 12.0], [0.0, 12.0])]),
            ];
            for i in 0..8 {
                let right = i % 2 == 0;
                let z = 2.0 - (i / 2) as f32;
                let x = if right { -4.0 } else { 4.0 };
                let b = if right { bx([-15.0, -1.0, -1.0], [16.0, 2.0, 2.0], [18.0, 0.0]) } else { bx([-1.0, -1.0, -1.0], [16.0, 2.0, 2.0], [18.0, 0.0]) };
                v.push(part([x, 15.0, z], vec![b]));
            }
            v
        }
    }
}

/// Animation inputs.
#[derive(Clone, Copy, Default, Debug)]
pub struct Pose {
    pub limb_swing: f32,
    pub limb_amount: f32,
    pub head_yaw: f32,   // radians relative to body
    pub head_pitch: f32, // radians
    pub age: f32,
    pub arms_forward: bool, // zombie
    pub swing: f32,         // attack swing progress 0..1
    pub sneak: bool,
}

pub fn animate(k: ModelKind, parts: &mut [Part], p: &Pose) {
    let ls = p.limb_swing;
    let la = p.limb_amount;
    let walk = |phase: f32| (ls * 0.6662 + phase).cos() * 1.4 * la;
    match k {
        ModelKind::Pig | ModelKind::Cow | ModelKind::Sheep | ModelKind::SheepFur | ModelKind::Creeper => {
            parts[0].rot[0] = p.head_pitch;
            parts[0].rot[1] = p.head_yaw;
            parts[2].rot[0] = walk(0.0);
            parts[3].rot[0] = walk(PI);
            parts[4].rot[0] = walk(PI);
            parts[5].rot[0] = walk(0.0);
        }
        ModelKind::Chicken => {
            parts[0].rot[0] = p.head_pitch;
            parts[0].rot[1] = p.head_yaw;
            parts[2].rot[0] = walk(0.0);
            parts[3].rot[0] = walk(PI);
            // flapping wings when in the air (age used as flap driver)
            let flap = (p.age * 0.9).sin().abs() * if p.arms_forward { 1.0 } else { 0.0 };
            parts[4].rot[2] = flap;
            parts[5].rot[2] = -flap;
        }
        ModelKind::Biped | ModelKind::Skeleton => {
            parts[0].rot[0] = p.head_pitch;
            parts[0].rot[1] = p.head_yaw;
            parts[2].rot = [walk(PI) * 0.5, 0.0, 0.0];
            parts[3].rot = [walk(0.0) * 0.5, 0.0, 0.0];
            parts[4].rot[0] = walk(0.0);
            parts[5].rot[0] = walk(PI);
            if p.arms_forward {
                let bob = (p.age * 0.067).sin() * 0.05;
                parts[2].rot = [-PI / 2.0 + bob, -0.1, (p.age * 0.09).cos() * 0.05 + 0.05];
                parts[3].rot = [-PI / 2.0 - bob, 0.1, -(p.age * 0.09).cos() * 0.05 - 0.05];
            }
            if p.swing > 0.0 {
                let s = p.swing;
                let f = 1.0 - (1.0 - s).powi(4);
                parts[2].rot[0] -= (f * PI).sin() * 1.2 + (s * PI).sin() * 0.4;
                parts[2].rot[1] += (s.sqrt() * PI * 2.0).sin() * 0.2;
            }
            if p.sneak {
                parts[1].rot[0] = 0.5;
                parts[2].rot[0] += 0.4;
                parts[3].rot[0] += 0.4;
                parts[4].pivot[2] = 4.0;
                parts[5].pivot[2] = 4.0;
                parts[4].pivot[1] = 9.0;
                parts[5].pivot[1] = 9.0;
                parts[0].pivot[1] = 1.0;
            }
        }
        ModelKind::Spider => {
            parts[0].rot[0] = p.head_pitch;
            parts[0].rot[1] = p.head_yaw;
            let f = PI / 4.0;
            let zr = [-f, f, -f * 0.74, f * 0.74, -f * 0.74, f * 0.74, -f, f];
            let yr = [0.785, -0.785, 0.3927, -0.3927, -0.3927, 0.3927, -0.785, 0.785];
            for i in 0..8 {
                let pair = i / 2;
                let phase = [0.0, PI, PI / 2.0, PI * 1.5][pair];
                let ysw = -((ls * 0.6662 * 2.0 + phase).cos() * 0.4) * la;
                let zsw = ((ls * 0.6662 + phase).sin() * 0.4).abs() * la;
                let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                parts[3 + i].rot = [0.0, yr[i] + ysw * sign, zr[i] + zsw * sign];
            }
        }
    }
}

/// Lighting/colour info for emitted vertices.
#[derive(Clone, Copy)]
pub struct Shade {
    pub sky: u8,
    pub block: u8,
    pub color: [u8; 4],
}

fn face_shade(n: Vec3) -> f32 {
    let (x2, y2, z2) = (n.x * n.x, n.y * n.y, n.z * n.z);
    let yv = if n.y > 0.0 { 1.0 } else { 0.5 };
    (x2 * 0.6 + y2 * yv + z2 * 0.8).clamp(0.4, 1.0)
}

/// Emit a model's geometry with the given root matrix (entity space -> world).
pub fn emit(out: &mut Vec<Vertex>, parts: &[Part], root: &Mat4, tex: (f32, f32), sh: Shade) {
    for p in parts {
        if !p.visible {
            continue;
        }
        let m = *root
            * Mat4::translate(p.pivot[0] / 16.0, p.pivot[1] / 16.0, p.pivot[2] / 16.0)
            * Mat4::rot_z(p.rot[2])
            * Mat4::rot_y(p.rot[1])
            * Mat4::rot_x(p.rot[0]);
        for b in &p.boxes {
            emit_box(out, b, &m, tex, sh);
        }
    }
}

pub fn emit_box(out: &mut Vec<Vertex>, b: &MBox, m: &Mat4, tex: (f32, f32), sh: Shade) {
    let i = b.inflate;
    let (x0, y0, z0) = (b.pos[0] - i, b.pos[1] - i, b.pos[2] - i);
    let (x1, y1, z1) = (b.pos[0] + b.size[0] + i, b.pos[1] + b.size[1] + i, b.pos[2] + b.size[2] + i);
    let (mut x0, mut x1) = (x0, x1);
    if b.mirror {
        std::mem::swap(&mut x0, &mut x1);
    }
    let s = 1.0 / 16.0;
    let p = |x: f32, y: f32, z: f32| m.transform(v3(x * s, y * s, z * s));
    let v7 = p(x0, y0, z0);
    let v0 = p(x1, y0, z0);
    let v1 = p(x1, y1, z0);
    let v2 = p(x0, y1, z0);
    let v3_ = p(x0, y0, z1);
    let v4 = p(x1, y0, z1);
    let v5 = p(x1, y1, z1);
    let v6 = p(x0, y1, z1);
    let (u, v) = (b.uv[0], b.uv[1]);
    let (w, h, d) = (b.size[0].floor(), b.size[1].floor(), b.size[2].floor());
    let quads: [([Vec3; 4], [f32; 4]); 6] = [
        ([v4, v0, v1, v5], [u + d + w, v + d, u + d + w + d, v + d + h]),
        ([v7, v3_, v6, v2], [u, v + d, u + d, v + d + h]),
        ([v4, v3_, v7, v0], [u + d, v, u + d + w, v + d]),
        ([v1, v2, v6, v5], [u + d + w, v + d, u + d + w + w, v]),
        ([v0, v7, v2, v1], [u + d, v + d, u + d + w, v + d + h]),
        ([v3_, v4, v5, v6], [u + d + w + d, v + d, u + d + w + d + w, v + d + h]),
    ];
    for (q, r) in quads.iter() {
        let (mut u1, v1_, mut u2, v2_) = (r[0] / tex.0, r[1] / tex.1, r[2] / tex.0, r[3] / tex.1);
        if b.mirror {
            std::mem::swap(&mut u1, &mut u2);
        }
        let uvs = [[u2, v1_], [u1, v1_], [u1, v2_], [u2, v2_]];
        let n = (q[1] - q[0]).cross(q[2] - q[1]).norm();
        let shade = (face_shade(n) * 255.0) as u8;
        let mk = |k: usize| Vertex { pos: [q[k].x, q[k].y, q[k].z], uv: uvs[k], color: sh.color, light: [sh.sky, sh.block, shade, 0] };
        out.extend_from_slice(&[mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]);
    }
}

/// Root transform for an entity at `pos` with body yaw (degrees, vanilla convention).
pub fn entity_root(pos: Vec3, yaw_deg: f32, scale: f32) -> Mat4 {
    Mat4::translate(pos.x, pos.y, pos.z)
        * Mat4::rot_y((180.0 - yaw_deg).to_radians())
        * Mat4::scale(-scale, -scale, scale)
        * Mat4::translate(0.0, -1.501, 0.0)
}
