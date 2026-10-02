//! Converts 16x16x16 chunk sections into vertex buffers.
//! Runs on worker threads from a padded snapshot of the world.

use crate::block::{self, *};
use crate::gl::Vertex;
use crate::world::{World, CHUNK_H};

pub const P: i32 = 18; // padded size

/// Atlas tile indices needed by the mesher.
#[derive(Clone)]
pub struct TileTable {
    /// per block: top, bottom, side, front
    pub blocks: Vec<[u16; 4]>,
    pub grass_overlay: u16,
    pub grass_snowed: u16,
    pub wheat: [u16; 8],
    pub tiles_per_row: u16,
}

impl TileTable {
    pub fn uv(&self, tile: u16) -> (f32, f32, f32) {
        let n = self.tiles_per_row as f32;
        let s = 1.0 / n;
        ((tile % self.tiles_per_row) as f32 * s, (tile / self.tiles_per_row) as f32 * s, s)
    }
}

pub struct MeshInput {
    pub cx: i32,
    pub sy: i32,
    pub cz: i32,
    pub blocks: Vec<u8>,
    pub meta: Vec<u8>,
    pub light: Vec<u8>,
    /// 18x18 columns
    pub grass: Vec<u32>,
    pub foliage: Vec<u32>,
}

#[inline]
fn pidx(x: i32, y: i32, z: i32) -> usize {
    (((y + 1) * P + (z + 1)) * P + (x + 1)) as usize
}

impl MeshInput {
    /// Snapshot a section plus a one-block border from the world.
    pub fn build(world: &World, cx: i32, sy: i32, cz: i32) -> MeshInput {
        let n = (P * P * P) as usize;
        let mut inp = MeshInput {
            cx,
            sy,
            cz,
            blocks: vec![0; n],
            meta: vec![0; n],
            light: vec![0xF0; n],
            grass: vec![0x91BD59; (P * P) as usize],
            foliage: vec![0x77AB2F; (P * P) as usize],
        };
        let y0 = sy * 16;
        for dcz in -1..=1 {
            for dcx in -1..=1 {
                let Some(ch) = world.chunk(cx + dcx, cz + dcz) else { continue };
                // range of local x/z in this neighbour that map into the padded box
                let (xs, xe) = match dcx { -1 => (15, 16), 0 => (0, 16), _ => (0, 1) };
                let (zs, ze) = match dcz { -1 => (15, 16), 0 => (0, 16), _ => (0, 1) };
                for lz in zs..ze {
                    for lx in xs..xe {
                        let px = lx + dcx * 16;
                        let pz = lz + dcz * 16;
                        let ci = (lz * 16 + lx) as usize;
                        let ti = ((pz + 1) * P + (px + 1)) as usize;
                        inp.grass[ti] = ch.grass_color[ci];
                        inp.foliage[ti] = ch.foliage_color[ci];
                        for py in -1..17 {
                            let wy = y0 + py;
                            if wy < 0 || wy >= CHUNK_H as i32 {
                                let i = pidx(px, py, pz);
                                inp.light[i] = if wy < 0 { 0 } else { 0xF0 };
                                continue;
                            }
                            let si = ((wy as usize * 16 + lz as usize) * 16) + lx as usize;
                            let i = pidx(px, py, pz);
                            inp.blocks[i] = ch.blocks[si];
                            inp.meta[i] = ch.meta[si];
                            inp.light[i] = ch.light[si];
                        }
                    }
                }
            }
        }
        inp
    }
    #[inline]
    fn b(&self, x: i32, y: i32, z: i32) -> u8 {
        self.blocks[pidx(x, y, z)]
    }
    #[inline]
    fn m(&self, x: i32, y: i32, z: i32) -> u8 {
        self.meta[pidx(x, y, z)]
    }
    #[inline]
    fn l(&self, x: i32, y: i32, z: i32) -> u8 {
        self.light[pidx(x, y, z)]
    }
}

// Face definitions: normal, and 4 corner offsets (CCW when viewed from outside).
// Faces: 0 -X, 1 +X, 2 -Y, 3 +Y, 4 -Z, 5 +Z
const NORMALS: [(i32, i32, i32); 6] = [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];
const FACE_SHADE: [f32; 6] = [0.6, 0.6, 0.5, 1.0, 0.8, 0.8];

/// Corner positions for each face (unit cube), in CCW order from outside,
/// starting at the vertex that maps to uv (0,1) (bottom-left of the texture).
const FACE_VERTS: [[[f32; 3]; 4]; 6] = [
    // -X : looking from -x towards +x; left = -z? (texture u goes +z)
    [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]],
    // +X : u goes -z
    [[1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]],
    // -Y
    [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
    // +Y
    [[0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
    // -Z : u goes -x
    [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
    // +Z : u goes +x
    [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]],
];
const FACE_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub fn face_dir_from_meta(meta: u8) -> usize {
    // meta facing: 0 north(-z), 1 south(+z), 2 west(-x), 3 east(+x)
    match meta & 3 {
        0 => 4,
        1 => 5,
        2 => 0,
        _ => 1,
    }
}

fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

pub struct Builder<'a> {
    pub tiles: &'a TileTable,
    pub solid: Vec<Vertex>,
    pub trans: Vec<Vertex>,
}

impl<'a> Builder<'a> {
    #[allow(clippy::too_many_arguments)]
    fn quad(out: &mut Vec<Vertex>, p: [[f32; 3]; 4], uv: [[f32; 2]; 4], color: [u8; 4], lights: [[u8; 3]; 4], flip: bool) {
        let v = |i: usize| Vertex { pos: p[i], uv: uv[i], color, light: [lights[i][0], lights[i][1], lights[i][2], 0] };
        if flip {
            out.extend_from_slice(&[v(1), v(2), v(3), v(1), v(3), v(0)]);
        } else {
            out.extend_from_slice(&[v(0), v(1), v(2), v(0), v(2), v(3)]);
        }
    }
}

fn tile_uv(t: &TileTable, tile: u16, u: f32, v: f32) -> [f32; 2] {
    let (u0, v0, s) = t.uv(tile);
    [u0 + u * s, v0 + v * s]
}

pub fn mesh_section(inp: &MeshInput, tiles: &TileTable) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut b = Builder { tiles, solid: Vec::with_capacity(4096), trans: Vec::new() };
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                let id = inp.b(x, y, z);
                if id == AIR {
                    continue;
                }
                let d = block::def(id);
                match d.shape {
                    Shape::Cube => {
                        if id == CHEST {
                            mesh_box(inp, &mut b, x, y, z, id, [1.0 / 16.0, 0.0, 1.0 / 16.0], [15.0 / 16.0, 14.0 / 16.0, 15.0 / 16.0]);
                        } else {
                            mesh_cube(inp, &mut b, x, y, z, id);
                        }
                    }
                    Shape::Liquid => mesh_liquid(inp, &mut b, x, y, z, id),
                    Shape::Cross => mesh_cross(inp, &mut b, x, y, z, id),
                    Shape::Crop => mesh_crop(inp, &mut b, x, y, z),
                    Shape::Torch => mesh_torch(inp, &mut b, x, y, z),
                    Shape::Layer => mesh_box(inp, &mut b, x, y, z, id, [0.0; 3], [1.0, 2.0 / 16.0, 1.0]),
                    Shape::Farmland => mesh_box(inp, &mut b, x, y, z, id, [0.0; 3], [1.0, 15.0 / 16.0, 1.0]),
                    Shape::Cactus => mesh_cactus(inp, &mut b, x, y, z),
                    Shape::None => {}
                }
            }
        }
    }
    (b.solid, b.trans)
}

fn tint_for(inp: &MeshInput, id: u8, x: i32, z: i32) -> [u8; 3] {
    match block::def(id).tint {
        Tint::None => [255, 255, 255],
        Tint::Grass => rgb(inp.grass[((z + 1) * P + (x + 1)) as usize]),
        Tint::Foliage => rgb(inp.foliage[((z + 1) * P + (x + 1)) as usize]),
        Tint::Fixed(c) => rgb(c),
    }
}

/// Smooth light + AO for one face corner. Returns (sky, block, ao-shade) as 0..255.
fn corner_light(inp: &MeshInput, x: i32, y: i32, z: i32, face: usize, corner: [f32; 3]) -> ([u8; 3], u8) {
    let (nx, ny, nz) = NORMALS[face];
    // q = cell in front of the face
    let (qx, qy, qz) = (x + nx, y + ny, z + nz);
    // tangent offsets: for each axis not equal to normal axis, step towards the corner
    let mut t = [(0, 0, 0); 2];
    let mut k = 0;
    for axis in 0..3 {
        let n_axis = match axis { 0 => nx, 1 => ny, _ => nz };
        if n_axis != 0 {
            continue;
        }
        let s = if corner[axis] > 0.5 { 1 } else { -1 };
        t[k] = match axis { 0 => (s, 0, 0), 1 => (0, s, 0), _ => (0, 0, s) };
        k += 1;
    }
    let c1 = (qx + t[0].0, qy + t[0].1, qz + t[0].2);
    let c2 = (qx + t[1].0, qy + t[1].1, qz + t[1].2);
    let c3 = (qx + t[0].0 + t[1].0, qy + t[0].1 + t[1].1, qz + t[0].2 + t[1].2);
    let op = |c: (i32, i32, i32)| block::is_opaque(inp.b(c.0, c.1, c.2));
    let s1 = op(c1);
    let s2 = op(c2);
    let sc = op(c3);
    let ao = if s1 && s2 { 0 } else { 3 - (s1 as u8 + s2 as u8 + sc as u8) };
    let mut sky = 0u32;
    let mut blk = 0u32;
    let mut n = 0u32;
    let q = inp.l(qx, qy, qz);
    sky += (q >> 4) as u32;
    blk += (q & 15) as u32;
    n += 1;
    for (c, solid) in [(c1, s1), (c2, s2), (c3, sc && !(s1 && s2))] {
        if !solid && !(s1 && s2 && c == c3) {
            let l = inp.l(c.0, c.1, c.2);
            sky += (l >> 4) as u32;
            blk += (l & 15) as u32;
            n += 1;
        }
    }
    let sky = (sky * 17 / n).min(255) as u8;
    let blk = (blk * 17 / n).min(255) as u8;
    let ao_f = [0.5, 0.65, 0.82, 1.0][ao as usize];
    let shade = (FACE_SHADE[face] * ao_f * 255.0) as u8;
    ([sky, blk, shade], ao)
}

fn face_visible(inp: &MeshInput, id: u8, x: i32, y: i32, z: i32, face: usize) -> bool {
    let (nx, ny, nz) = NORMALS[face];
    let nb = inp.b(x + nx, y + ny, z + nz);
    if block::is_opaque(nb) {
        return false;
    }
    let d = block::def(id);
    if nb == id && (id == GLASS || id == ICE) {
        return false;
    }
    if d.layer == Layer::Opaque || block::is_leaves(id) {
        return true;
    }
    true
}

fn mesh_cube(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32, id: u8) {
    let d = block::def(id);
    let tiles = b.tiles.blocks[id as usize];
    let tint = tint_for(inp, id, x, z);
    let meta = inp.m(x, y, z);
    let front_face = if d.front.is_some() { face_dir_from_meta(meta) } else { 99 };
    let snowy = id == GRASS && matches!(inp.b(x, y + 1, z), SNOW_LAYER | SNOW_BLOCK);
    for face in 0..6 {
        if !face_visible(inp, id, x, y, z, face) {
            continue;
        }
        let mut tile = match face {
            3 => tiles[0],
            2 => tiles[1],
            _ => tiles[2],
        };
        if face == front_face {
            tile = tiles[3];
        }
        let mut color = [255u8, 255, 255, 255];
        if d.tint != Tint::None && (id != GRASS || face == 3) {
            color = [tint[0], tint[1], tint[2], 255];
        }
        if snowy && face != 3 && face != 2 {
            tile = b.tiles.grass_snowed;
        }
        let mut pos = [[0.0; 3]; 4];
        let mut lights = [[0u8; 3]; 4];
        let mut aos = [0u8; 4];
        let mut uvs = [[0.0; 2]; 4];
        for i in 0..4 {
            let c = FACE_VERTS[face][i];
            pos[i] = [x as f32 + c[0], y as f32 + c[1], z as f32 + c[2]];
            let (l, ao) = corner_light(inp, x, y, z, face, c);
            lights[i] = l;
            aos[i] = ao;
            uvs[i] = tile_uv(b.tiles, tile, FACE_UVS[i][0], FACE_UVS[i][1]);
        }
        let flip = aos[0] as u32 + aos[2] as u32 > aos[1] as u32 + aos[3] as u32;
        let out = if d.layer == Layer::Translucent { &mut b.trans } else { &mut b.solid };
        Builder::quad(out, pos, uvs, color, lights, flip);
        // Grass side overlay
        if id == GRASS && face != 3 && face != 2 && !snowy {
            let uvs2: [[f32; 2]; 4] = std::array::from_fn(|i| tile_uv(b.tiles, b.tiles.grass_overlay, FACE_UVS[i][0], FACE_UVS[i][1]));
            // offset slightly outwards to avoid z-fighting
            let (nx, ny, nz) = NORMALS[face];
            let e = 0.001;
            let pos2: [[f32; 3]; 4] = std::array::from_fn(|i| [pos[i][0] + nx as f32 * e, pos[i][1] + ny as f32 * e, pos[i][2] + nz as f32 * e]);
            Builder::quad(&mut b.solid, pos2, uvs2, [tint[0], tint[1], tint[2], 255], lights, flip);
        }
    }
}

/// Flat-lit box (used for partial blocks). min/max in block-local units.
fn mesh_box(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32, id: u8, min: [f32; 3], max: [f32; 3]) {
    let d = block::def(id);
    let tiles = b.tiles.blocks[id as usize];
    let meta = inp.m(x, y, z);
    let front_face = if d.front.is_some() { face_dir_from_meta(meta) } else { 99 };
    let tint = tint_for(inp, id, x, z);
    for face in 0..6 {
        let (nx, ny, nz) = NORMALS[face];
        // cull only faces that touch the cell boundary
        let on_edge = match face {
            0 => min[0] == 0.0,
            1 => max[0] == 1.0,
            2 => min[1] == 0.0,
            3 => max[1] == 1.0,
            4 => min[2] == 0.0,
            _ => max[2] == 1.0,
        };
        if on_edge && block::is_opaque(inp.b(x + nx, y + ny, z + nz)) {
            continue;
        }
        let mut tile = match face {
            3 => tiles[0],
            2 => tiles[1],
            _ => tiles[2],
        };
        if face == front_face {
            tile = tiles[3];
        }
        if id == FARMLAND && face == 3 && meta > 0 {
            tile = tiles[3];
        }
        let l = if on_edge { inp.l(x + nx, y + ny, z + nz) } else { inp.l(x, y, z) };
        let l = if face == 2 && on_edge && block::is_opaque(inp.b(x, y - 1, z)) { inp.l(x, y, z) } else { l };
        let light = [(l >> 4) * 17, (l & 15) * 17, (FACE_SHADE[face] * 255.0) as u8];
        let mut pos = [[0.0; 3]; 4];
        let mut uvs = [[0.0; 2]; 4];
        for i in 0..4 {
            let c = FACE_VERTS[face][i];
            let p = [
                if c[0] > 0.5 { max[0] } else { min[0] },
                if c[1] > 0.5 { max[1] } else { min[1] },
                if c[2] > 0.5 { max[2] } else { min[2] },
            ];
            pos[i] = [x as f32 + p[0], y as f32 + p[1], z as f32 + p[2]];
            // UV from box coordinates (vanilla-style cropping)
            let (u, v) = match face {
                0 => (p[2], 1.0 - p[1]),
                1 => (1.0 - p[2], 1.0 - p[1]),
                2 => (p[0], p[2]),
                3 => (p[0], p[2]),
                4 => (1.0 - p[0], 1.0 - p[1]),
                _ => (p[0], 1.0 - p[1]),
            };
            uvs[i] = tile_uv(b.tiles, tile, u, v);
        }
        let color = if d.tint != Tint::None { [tint[0], tint[1], tint[2], 255] } else { [255; 4] };
        Builder::quad(&mut b.solid, pos, uvs, color, [light; 4], false);
    }
}

fn mesh_cactus(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32) {
    let tiles = b.tiles.blocks[CACTUS as usize];
    let e = 1.0 / 16.0;
    let l = inp.l(x, y, z);
    for face in 0..6 {
        let (nx, ny, nz) = NORMALS[face];
        if (face == 2 || face == 3) && block::is_opaque(inp.b(x + nx, y + ny, z + nz)) {
            continue;
        }
        if (face == 2 || face == 3) && inp.b(x + nx, y + ny, z + nz) == CACTUS {
            continue;
        }
        let tile = match face {
            3 => tiles[0],
            2 => tiles[1],
            _ => tiles[2],
        };
        let mut pos = [[0.0; 3]; 4];
        let mut uvs = [[0.0; 2]; 4];
        for i in 0..4 {
            let mut c = FACE_VERTS[face][i];
            match face {
                0 => c[0] = e,
                1 => c[0] = 1.0 - e,
                4 => c[2] = e,
                5 => c[2] = 1.0 - e,
                _ => {}
            }
            pos[i] = [x as f32 + c[0], y as f32 + c[1], z as f32 + c[2]];
            uvs[i] = tile_uv(b.tiles, tile, FACE_UVS[i][0], FACE_UVS[i][1]);
        }
        let ll = if face == 3 { inp.l(x, y + 1, z) } else { l };
        let light = [(ll >> 4) * 17, (ll & 15) * 17, (FACE_SHADE[face] * 255.0) as u8];
        Builder::quad(&mut b.solid, pos, uvs, [255; 4], [light; 4], false);
    }
}

fn double_quad(out: &mut Vec<Vertex>, pos: [[f32; 3]; 4], uvs: [[f32; 2]; 4], color: [u8; 4], light: [u8; 3]) {
    Builder::quad(out, pos, uvs, color, [light; 4], false);
    let rp = [pos[1], pos[0], pos[3], pos[2]];
    let ru = [uvs[1], uvs[0], uvs[3], uvs[2]];
    Builder::quad(out, rp, ru, color, [light; 4], false);
}

fn mesh_cross(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32, id: u8) {
    let tile = b.tiles.blocks[id as usize][2];
    let tint = tint_for(inp, id, x, z);
    let l = inp.l(x, y, z);
    let light = [(l >> 4) * 17, (l & 15) * 17, 255];
    // small random offset like vanilla for grass/flowers
    let h = (x.wrapping_mul(3129871) ^ z.wrapping_mul(116129781) ^ y) as u32;
    let h = h.wrapping_mul(h).wrapping_mul(42317861).wrapping_add(h.wrapping_mul(11));
    let (ox, oz) = if id != SUGAR_CANE {
        ((((h >> 16) & 15) as f32 / 15.0 - 0.5) * 0.4, (((h >> 24) & 15) as f32 / 15.0 - 0.5) * 0.4)
    } else {
        (0.0, 0.0)
    };
    let (fx, fy, fz) = (x as f32 + ox, y as f32, z as f32 + oz);
    let a = 0.5 - 0.45;
    let c = 0.5 + 0.45;
    let uv: [[f32; 2]; 4] = std::array::from_fn(|i| tile_uv(b.tiles, tile, FACE_UVS[i][0], FACE_UVS[i][1]));
    let color = [tint[0], tint[1], tint[2], 255];
    let q1 = [[fx + a, fy, fz + a], [fx + c, fy, fz + c], [fx + c, fy + 1.0, fz + c], [fx + a, fy + 1.0, fz + a]];
    let q2 = [[fx + a, fy, fz + c], [fx + c, fy, fz + a], [fx + c, fy + 1.0, fz + a], [fx + a, fy + 1.0, fz + c]];
    double_quad(&mut b.solid, q1, uv, color, light);
    double_quad(&mut b.solid, q2, uv, color, light);
}

fn mesh_crop(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32) {
    let stage = inp.m(x, y, z).min(7) as usize;
    let tile = b.tiles.wheat[stage];
    let l = inp.l(x, y, z);
    let light = [(l >> 4) * 17, (l & 15) * 17, 255];
    let (fx, fy, fz) = (x as f32, y as f32 - 1.0 / 16.0, z as f32);
    let uv: [[f32; 2]; 4] = std::array::from_fn(|i| tile_uv(b.tiles, tile, FACE_UVS[i][0], FACE_UVS[i][1]));
    for o in [0.25, 0.75] {
        let qx = [[fx + o, fy, fz], [fx + o, fy, fz + 1.0], [fx + o, fy + 1.0, fz + 1.0], [fx + o, fy + 1.0, fz]];
        let qz = [[fx, fy, fz + o], [fx + 1.0, fy, fz + o], [fx + 1.0, fy + 1.0, fz + o], [fx, fy + 1.0, fz + o]];
        double_quad(&mut b.solid, qx, uv, [255; 4], light);
        double_quad(&mut b.solid, qz, uv, [255; 4], light);
    }
}

/// Torch: a 2x10 pixel stick; wall torches lean away from the wall.
pub fn torch_geometry(meta: u8) -> ([f32; 3], [f32; 3]) {
    // returns base centre and top offset (shear) in block coords
    let p = 1.0 / 16.0;
    match meta {
        1 => ([0.5, 3.5 * p, 1.5 * p], [0.0, 0.0, 0.4]),
        2 => ([0.5, 3.5 * p, 1.0 - 1.5 * p], [0.0, 0.0, -0.4]),
        3 => ([1.5 * p, 3.5 * p, 0.5], [0.4, 0.0, 0.0]),
        4 => ([1.0 - 1.5 * p, 3.5 * p, 0.5], [-0.4, 0.0, 0.0]),
        _ => ([0.5, 0.0, 0.5], [0.0, 0.0, 0.0]),
    }
}

pub fn emit_torch(out: &mut Vec<Vertex>, tiles: &TileTable, origin: [f32; 3], meta: u8, light: [u8; 3]) {
    let tile = tiles.blocks[TORCH as usize][2];
    let p = 1.0 / 16.0;
    let (base, lean) = torch_geometry(meta);
    let h = 10.0 * p;
    let w = 1.0 * p;
    // the stick: 4 sides + top
    let pt = |dx: f32, dy: f32, dz: f32| -> [f32; 3] {
        let t = dy / h;
        [origin[0] + base[0] + dx + lean[0] * t, origin[1] + base[1] + dy, origin[2] + base[2] + dz + lean[2] * t]
    };
    // side faces use texture columns 7..9, rows 6..16
    let uvr = |u0: f32, v0: f32, u1: f32, v1: f32| -> [[f32; 2]; 4] {
        [tile_uv(tiles, tile, u0, v1), tile_uv(tiles, tile, u1, v1), tile_uv(tiles, tile, u1, v0), tile_uv(tiles, tile, u0, v0)]
    };
    let side_uv = uvr(7.0 * p, 6.0 * p, 9.0 * p, 1.0);
    let faces: [([[f32; 3]; 4], usize); 4] = [
        ([pt(-w, 0.0, -w), pt(-w, 0.0, w), pt(-w, h, w), pt(-w, h, -w)], 0),
        ([pt(w, 0.0, w), pt(w, 0.0, -w), pt(w, h, -w), pt(w, h, w)], 1),
        ([pt(w, 0.0, -w), pt(-w, 0.0, -w), pt(-w, h, -w), pt(w, h, -w)], 4),
        ([pt(-w, 0.0, w), pt(w, 0.0, w), pt(w, h, w), pt(-w, h, w)], 5),
    ];
    for (q, f) in faces {
        let l = [light[0], light[1], (FACE_SHADE[f] * 255.0) as u8];
        Builder::quad(out, q, side_uv, [255; 4], [l; 4], false);
    }
    let top = [pt(-w, h, w), pt(w, h, w), pt(w, h, -w), pt(-w, h, -w)];
    Builder::quad(out, top, uvr(7.0 * p, 6.0 * p, 9.0 * p, 8.0 * p), [255; 4], [[light[0], light[1], 255]; 4], false);
    let bot = [pt(-w, 0.0, -w), pt(w, 0.0, -w), pt(w, 0.0, w), pt(-w, 0.0, w)];
    Builder::quad(out, bot, uvr(7.0 * p, 14.0 * p, 9.0 * p, 1.0), [255; 4], [[light[0], light[1], 128]; 4], false);
}

fn mesh_torch(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32) {
    let l = inp.l(x, y, z);
    let light = [(l >> 4) * 17, (l & 15) * 17, 255];
    let meta = inp.m(x, y, z);
    let tiles = b.tiles;
    emit_torch(&mut b.solid, tiles, [x as f32, y as f32, z as f32], meta, light);
}

fn mesh_liquid(inp: &MeshInput, b: &mut Builder, x: i32, y: i32, z: i32, id: u8) {
    let tile = b.tiles.blocks[id as usize][0];
    let above = inp.b(x, y + 1, z);
    let top_h = if above == id { 1.0 } else { 14.0 / 16.0 };
    let translucent = id == WATER;
    for face in 0..6 {
        let (nx, ny, nz) = NORMALS[face];
        let nb = inp.b(x + nx, y + ny, z + nz);
        if nb == id || (face != 3 && block::is_opaque(nb)) {
            continue;
        }
        if face == 3 && block::is_opaque(nb) && top_h >= 1.0 {
            continue;
        }
        if id == WATER && nb == ICE {
            continue;
        }
        let l = if block::is_opaque(nb) { inp.l(x, y, z) } else { inp.l(x + nx, y + ny, z + nz) };
        let l = l.max(inp.l(x, y, z));
        let light = [(l >> 4) * 17, (l & 15) * 17, (FACE_SHADE[face] * 255.0) as u8];
        let mut pos = [[0.0; 3]; 4];
        let mut uvs = [[0.0; 2]; 4];
        for i in 0..4 {
            let c = FACE_VERTS[face][i];
            let cy = if c[1] > 0.5 { top_h } else { 0.0 };
            pos[i] = [x as f32 + c[0], y as f32 + cy, z as f32 + c[2]];
            let v = if face == 3 || face == 2 { FACE_UVS[i][1] } else { 1.0 - cy };
            uvs[i] = tile_uv(b.tiles, tile, FACE_UVS[i][0], v);
        }
        let out = if translucent { &mut b.trans } else { &mut b.solid };
        let color = [255u8, 255, 255, 255];
        Builder::quad(out, pos, uvs, color, [light; 4], false);
        if translucent && face == 3 {
            // underside of the water surface, visible from below
            let rp = [pos[1], pos[0], pos[3], pos[2]];
            let ru = [uvs[1], uvs[0], uvs[3], uvs[2]];
            Builder::quad(out, rp, ru, color, [light; 4], false);
        }
    }
}
