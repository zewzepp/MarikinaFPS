use macroquad::prelude::*;
use std::f32::consts::TAU;

pub const ATLAS: usize = 512;
pub const TILE: usize = 128;
pub const NTILES: usize = 16;

pub const T_WHITE: usize = 0;
pub const T_ASPHALT: usize = 1;
pub const T_ASPH_LINE: usize = 2;
pub const T_ZEBRA: usize = 3;
pub const T_SIDEWALK: usize = 4;
pub const T_GRASS: usize = 5;
pub const T_PAVING: usize = 6;
pub const T_CONCRETE: usize = 7;
pub const T_FAC_RES: usize = 8;
pub const T_FAC_OFFICE: usize = 9;
pub const T_FAC_SHOP: usize = 10;
pub const T_ROOF: usize = 11;
pub const T_BRICK: usize = 12;
pub const T_WOOD: usize = 13;
pub const T_METAL: usize = 14;
pub const T_COURT: usize = 15;

pub const FOG: [f32; 3] = [0.70, 0.79, 0.89];
pub const FOG_START: f32 = 38.0;
pub const FOG_END: f32 = 150.0;
pub const LOD_DIST: f32 = 70.0;
pub const VIEW_DIST: f32 = 150.0;

pub fn sun_dir() -> Vec3 {
    vec3(0.55, 0.75, 0.35).normalize()
}

pub fn fog_color() -> Color {
    Color::new(FOG[0], FOG[1], FOG[2], 1.0)
}

// uv inside an atlas tile; a,b in 0..1 (b = 0 is the top of the tile)
pub fn tile_uv(tile: usize, a: f32, b: f32) -> Vec2 {
    let ts = TILE as f32 / ATLAS as f32;
    let inset = 0.6 / ATLAS as f32;
    let tx = (tile % 4) as f32;
    let ty = (tile / 4) as f32;
    vec2(
        tx * ts + inset + a * (ts - 2.0 * inset),
        ty * ts + inset + b * (ts - 2.0 * inset),
    )
}

// Cool sky ambient + warm sun. vis = 0..1 sun visibility (shadow).
pub fn light(n: Vec3, vis: f32) -> [f32; 3] {
    let l = sun_dir();
    let a = 0.40 + 0.14 * (n.y * 0.5 + 0.5);
    let d = 0.62 * n.dot(l).max(0.0) * vis;
    [a * 0.86 + d * 1.00, a * 0.93 + d * 0.95, a * 1.00 + d * 0.84]
}

#[derive(Clone, Copy)]
pub struct SV {
    pub p: Vec3,
    pub uv: Vec2,
    pub c: Color,
}

#[derive(Clone, Copy)]
pub struct Quad {
    pub v: [SV; 4],
    pub n: Vec3, // outward normal, ZERO = draw from both sides
    pub tile: usize,
}

impl Quad {
    // Untextured far-away version: flat colour = tile average x vertex colour.
    pub fn to_lod(&self, avg: &[[f32; 3]; NTILES]) -> Quad {
        let a = avg[self.tile];
        let uv = tile_uv(T_WHITE, 0.5, 0.5);
        let mut q = *self;
        for s in q.v.iter_mut() {
            s.uv = uv;
            s.c = Color::new(s.c.r * a[0], s.c.g * a[1], s.c.b * a[2], s.c.a);
        }
        q.tile = T_WHITE;
        q
    }
}

// ---------------------------------------------------------------------------
// Batched mesh with per-vertex fog + sun lighting for dynamic boxes.
// macroquad limits a draw call to ~10000 vertices / 5000 indices, so flush early.
// ---------------------------------------------------------------------------
pub struct Batch {
    v: Vec<Vertex>,
    i: Vec<u16>,
    tex: Texture2D,
    pub cam: Vec3,
    pub vis: f32,
    white: Vec2,
    xo: Vec3,
    xs: f32,
    pub nofog: bool,
}

impl Batch {
    pub fn new(tex: &Texture2D, cam: Vec3) -> Self {
        Self {
            v: Vec::new(),
            i: Vec::new(),
            tex: tex.clone(),
            cam,
            vis: 1.0,
            white: tile_uv(T_WHITE, 0.5, 0.5),
            xo: Vec3::ZERO,
            xs: 1.0,
            nofog: false,
        }
    }

    // scale everything drawn next about `origin` (used to size character models)
    pub fn set_xform(&mut self, origin: Vec3, s: f32) {
        self.xo = origin;
        self.xs = s;
    }

    pub fn clear_xform(&mut self) {
        self.xs = 1.0;
    }

    fn xf(&self, p: Vec3) -> Vec3 {
        self.xo + (p - self.xo) * self.xs
    }

    pub fn flush(&mut self) {
        if self.v.is_empty() {
            return;
        }
        let mesh = Mesh {
            vertices: std::mem::take(&mut self.v),
            indices: std::mem::take(&mut self.i),
            texture: Some(self.tex.clone()),
        };
        draw_mesh(&mesh);
    }

    fn fogc(&self, p: Vec3, c: Color) -> Color {
        if self.nofog {
            return c;
        }
        let d = (p - self.cam).length();
        let t = ((d - FOG_START) / (FOG_END - FOG_START)).clamp(0.0, 1.0);
        if t <= 0.0 {
            return c;
        }
        let f = t * t * (3.0 - 2.0 * t);
        Color::new(
            c.r + (FOG[0] - c.r) * f,
            c.g + (FOG[1] - c.g) * f,
            c.b + (FOG[2] - c.b) * f,
            c.a,
        )
    }

    fn raw(&mut self, p: [Vec3; 4], uv: [Vec2; 4], c: [Color; 4]) {
        let base = self.v.len() as u16;
        for k in 0..4 {
            let pk = self.xf(p[k]);
            let col = self.fogc(pk, c[k]);
            self.v.push(Vertex::new(pk.x, pk.y, pk.z, uv[k].x, uv[k].y, col));
        }
        self.i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if self.v.len() > 2800 || self.i.len() > 4000 {
            self.flush();
        }
    }

    // pre-built static quad (with back-face culling)
    pub fn push(&mut self, q: &Quad) {
        if q.n != Vec3::ZERO && q.n.dot(self.cam - q.v[0].p) <= 0.0 {
            return;
        }
        if q.v[2].p == q.v[3].p {
            // triangle stored as a degenerate quad: emit 3 verts / 3 indices
            let base = self.v.len() as u16;
            for k in 0..3 {
                let pk = self.xf(q.v[k].p);
                let col = self.fogc(pk, q.v[k].c);
                self.v.push(Vertex::new(pk.x, pk.y, pk.z, q.v[k].uv.x, q.v[k].uv.y, col));
            }
            self.i.extend_from_slice(&[base, base + 1, base + 2]);
            if self.i.len() > 4000 {
                self.flush();
            }
            return;
        }
        self.raw(
            [q.v[0].p, q.v[1].p, q.v[2].p, q.v[3].p],
            [q.v[0].uv, q.v[1].uv, q.v[2].uv, q.v[3].uv],
            [q.v[0].c, q.v[1].c, q.v[2].c, q.v[3].c],
        );
    }

    pub fn quad(&mut self, p: [Vec3; 4], col: Color) {
        let w = self.white;
        self.raw(p, [w, w, w, w], [col, col, col, col]);
    }

    // Box from three half-extent axis vectors, lit by the sun.
    pub fn boxed(&mut self, c: Vec3, ax: Vec3, ay: Vec3, az: Vec3, col: Color) {
        let axes = [ax, ay, az];
        for k in 0..3 {
            let n = axes[k];
            let u = axes[(k + 1) % 3];
            let v = axes[(k + 2) % 3];
            for s in [-1.0f32, 1.0] {
                let fc = c + n * s;
                let nn = (n * s).normalize_or_zero();
                if nn.dot(self.cam - self.xf(fc)) <= 0.0 {
                    continue;
                }
                let l = light(nn, self.vis);
                let fcol = Color::new(col.r * l[0], col.g * l[1], col.b * l[2], col.a);
                self.quad([fc - u - v, fc + u - v, fc + u + v, fc - u + v], fcol);
            }
        }
    }

    pub fn aabb(&mut self, c: Vec3, half: Vec3, col: Color) {
        self.boxed(c, vec3(half.x, 0.0, 0.0), vec3(0.0, half.y, 0.0), vec3(0.0, 0.0, half.z), col);
    }

    // Box rotated around Y. hf = half size forward, hu = up, hr = right.
    pub fn obox(&mut self, c: Vec3, yaw: f32, hf: f32, hu: f32, hr: f32, col: Color) {
        let f = vec3(yaw.cos(), 0.0, yaw.sin());
        let r = vec3(-yaw.sin(), 0.0, yaw.cos());
        self.boxed(c, f * hf, vec3(0.0, hu, 0.0), r * hr, col);
    }

    // Bar between two points (arms, legs).
    pub fn limb(&mut self, a: Vec3, b: Vec3, t: f32, col: Color) {
        let dv = b - a;
        let len = dv.length();
        if len < 0.001 {
            return;
        }
        let n = dv / len;
        let helper = if n.y.abs() > 0.9 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
        let u = n.cross(helper).normalize() * t;
        let v = n.cross(u).normalize() * t;
        self.boxed((a + b) * 0.5, dv * 0.5, u, v, col);
    }

    // camera-facing flat quad (particles, flashes)
    pub fn billboard(&mut self, c: Vec3, right: Vec3, up: Vec3, s: f32, col: Color) {
        let r = right * s;
        let u = up * s;
        self.quad([c - r - u, c + r - u, c + r + u, c - r + u], col);
    }
}

// ---------------------------------------------------------------------------
// Sky dome with horizon haze that matches the fog, sun and soft clouds.
// ---------------------------------------------------------------------------
fn add_quad(v: &mut Vec<Vertex>, i: &mut Vec<u16>, p: [Vec3; 4], col: Color, uv: Vec2) {
    let base = v.len() as u16;
    for q in p.iter() {
        v.push(Vertex::new(q.x, q.y, q.z, uv.x, uv.y, col));
    }
    i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn draw_sky(tex: &Texture2D, cam: Vec3, t: f32) {
    let white = tile_uv(T_WHITE, 0.5, 0.5);
    let zen = [0.18f32, 0.42, 0.80];
    let rings: [f32; 9] = [-0.6, -0.15, 0.0, 0.08, 0.2, 0.4, 0.7, 1.1, 1.5707];
    let segs = 20usize;
    let rad = 420.0f32;
    let mut v: Vec<Vertex> = Vec::new();
    let mut idx: Vec<u16> = Vec::new();
    for el in rings.iter() {
        let col = if *el <= 0.0 {
            fog_color()
        } else {
            let k = (*el / 1.5707).powf(0.5);
            Color::new(
                FOG[0] + (zen[0] - FOG[0]) * k,
                FOG[1] + (zen[1] - FOG[1]) * k,
                FOG[2] + (zen[2] - FOG[2]) * k,
                1.0,
            )
        };
        for s in 0..=segs {
            let az = s as f32 / segs as f32 * TAU;
            let dir = vec3(el.cos() * az.cos(), el.sin(), el.cos() * az.sin());
            let p = cam + dir * rad;
            v.push(Vertex::new(p.x, p.y, p.z, white.x, white.y, col));
        }
    }
    for ri in 0..rings.len() - 1 {
        for s in 0..segs {
            let a = (ri * (segs + 1) + s) as u16;
            let b = a + 1;
            let c = a + (segs + 1) as u16;
            let d = c + 1;
            idx.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: Some(tex.clone()) });

    // sun + clouds
    let mut v2: Vec<Vertex> = Vec::new();
    let mut i2: Vec<u16> = Vec::new();
    let sd = sun_dir();
    let sp = cam + sd * 400.0;
    let rr = sd.cross(Vec3::Y).normalize_or_zero();
    let uu = rr.cross(sd);
    let glow = 70.0f32;
    add_quad(
        &mut v2,
        &mut i2,
        [sp - rr * glow - uu * glow, sp + rr * glow - uu * glow, sp + rr * glow + uu * glow, sp - rr * glow + uu * glow],
        Color::new(1.0, 0.95, 0.80, 0.16),
        white,
    );
    let disc = 15.0f32;
    add_quad(
        &mut v2,
        &mut i2,
        [sp - rr * disc - uu * disc, sp + rr * disc - uu * disc, sp + rr * disc + uu * disc, sp - rr * disc + uu * disc],
        Color::new(1.0, 0.98, 0.88, 1.0),
        white,
    );
    for k in 0..14 {
        let kf = k as f32;
        let az = kf * 2.399 + t * 0.003;
        let el = 0.18 + ((k * 37) % 11) as f32 * 0.055;
        let sz = 38.0 + ((k * 53) % 7) as f32 * 6.0;
        let dir = vec3(el.cos() * az.cos(), el.sin(), el.cos() * az.sin());
        let center = cam + dir * 405.0;
        let cr = dir.cross(Vec3::Y).normalize_or_zero();
        let cu = cr.cross(dir).normalize_or_zero();
        for sub in 0..3 {
            let off = (sub as f32 - 1.0) * sz * 0.9;
            let c0 = center + cr * off + cu * (sz * 0.12 * (sub as f32 - 1.0).abs());
            let base = v2.len() as u16;
            v2.push(Vertex::new(c0.x, c0.y, c0.z, white.x, white.y, Color::new(1.0, 1.0, 1.0, 0.62)));
            for r in 0..10 {
                let a = r as f32 / 10.0 * TAU;
                let pp = c0 + cr * (a.cos() * sz * 1.7) + cu * (a.sin() * sz * 0.65);
                v2.push(Vertex::new(pp.x, pp.y, pp.z, white.x, white.y, Color::new(1.0, 1.0, 1.0, 0.0)));
            }
            for r in 0..10u16 {
                let a = base + 1 + r;
                let b = base + 1 + (r + 1) % 10;
                i2.extend_from_slice(&[base, a, b]);
            }
        }
    }
    draw_mesh(&Mesh { vertices: v2, indices: i2, texture: Some(tex.clone()) });
}
