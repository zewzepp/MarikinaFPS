use crate::gfx::*;
use crate::tex::hash;
use macroquad::prelude::*;
use std::collections::HashMap;

pub const ROAD: f32 = 8.0;
pub const PERIOD: f32 = 44.0;
pub const NB: usize = 1;
pub const SIZE: f32 = 52.0; // compact cargo arena
pub const CURB: f32 = 0.12;
const CH: f32 = 32.0;
pub const GC: f32 = 8.0;
pub const GN: usize = 7;

#[derive(Clone, Copy)]
pub struct BoxC {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
    pub h: f32,
}

// kinds of smashable props
pub const P_CRATE: u8 = 0;
pub const P_BENCH: u8 = 1;
pub const P_CONE: u8 = 2;
pub const P_BIN: u8 = 3;
pub const P_BARREL: u8 = 4;

#[derive(Clone, Copy)]
pub struct PropSpawn {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub y0: f32,
    pub along_x: bool,
    pub seed: u32,
}

pub struct Chunk {
    pub c: Vec3,
    pub quads: Vec<Quad>,
}

pub struct World {
    pub boxes: Vec<BoxC>,
    grid: Vec<Vec<usize>>,
    pub chunks: Vec<Chunk>,
    pub spawns: Vec<(f32, f32, bool)>, // x, z, enemy team
    pub props: Vec<PropSpawn>,
}

impl World {
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        if x < 0.0 || z < 0.0 || x >= SIZE || z >= SIZE {
            return 99.0;
        }
        let gx = ((x / GC) as usize).min(GN - 1);
        let gz = ((z / GC) as usize).min(GN - 1);
        let mut h = 0.0f32;
        for &i in &self.grid[gz * GN + gx] {
            let b = &self.boxes[i];
            if x >= b.x0 && x <= b.x1 && z >= b.z0 && z <= b.z1 && b.h > h {
                h = b.h;
            }
        }
        h
    }

    pub fn draw(&self, b: &mut Batch, cam: Vec3, fwd: Vec3) {
        for ch in &self.chunks {
            let d = ch.c - cam;
            if d.length() > VIEW_DIST + 30.0 {
                continue;
            }
            if d.dot(fwd) < -30.0 {
                continue;
            }
            for q in &ch.quads {
                b.push(q);
            }
        }
    }
}

struct B {
    q: HashMap<(i32, i32), Vec<Quad>>,
    boxes: Vec<BoxC>,
    spawns: Vec<(f32, f32, bool)>,
    props: Vec<PropSpawn>,
}

fn tint(r: f32, g: f32, b: f32) -> Color {
    Color::new(r, g, b, 1.0)
}

impl B {
    fn push(&mut self, p: [Vec3; 4], n: Vec3, uv: [Vec2; 4], tile: usize, cols: [Color; 4]) {
        let cx = (p[0].x + p[2].x) * 0.5;
        let cz = (p[0].z + p[2].z) * 0.5;
        let key = ((cx / CH).floor() as i32, (cz / CH).floor() as i32);
        let mk = |k: usize| SV { p: p[k], uv: uv[k], c: cols[k] };
        self.q.entry(key).or_default().push(Quad { v: [mk(0), mk(1), mk(2), mk(3)], n, tile });
    }

    fn solid(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, h: f32) {
        self.boxes.push(BoxC { x0, z0, x1, z1, h });
    }

    fn wall(&mut self, ax: f32, az: f32, bx: f32, bz: f32, y0: f32, y1: f32, tile: usize, t: Color, n: Vec3) {
        let len = ((bx - ax).powi(2) + (bz - az).powi(2)).sqrt();
        let nx = ((len / 4.0).ceil() as i32).max(1);
        let ny = (((y1 - y0) / 4.0).ceil() as i32).max(1);
        let l = light(n, 1.0);
        for i in 0..nx {
            for j in 0..ny {
                let t0 = i as f32 / nx as f32;
                let t1 = (i + 1) as f32 / nx as f32;
                let h0 = y0 + (y1 - y0) * j as f32 / ny as f32;
                let h1 = y0 + (y1 - y0) * (j + 1) as f32 / ny as f32;
                let px = |s: f32| ax + (bx - ax) * s;
                let pz = |s: f32| az + (bz - az) * s;
                let col = |y: f32| {
                    let ao = 0.70 + 0.30 * (y / 3.0).min(1.0);
                    Color::new(t.r * l[0] * ao, t.g * l[1] * ao, t.b * l[2] * ao, 1.0)
                };
                let p = [vec3(px(t0), h0, pz(t0)), vec3(px(t1), h0, pz(t1)), vec3(px(t1), h1, pz(t1)), vec3(px(t0), h1, pz(t0))];
                let uv = [tile_uv(tile, 0.0, 1.0), tile_uv(tile, 1.0, 1.0), tile_uv(tile, 1.0, 0.0), tile_uv(tile, 0.0, 0.0)];
                let c0 = col(h0);
                let c1 = col(h1);
                self.push(p, n, uv, tile, [c0, c0, c1, c1]);
            }
        }
    }

    fn floor(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, tile: usize, t: Color, rot: bool) {
        let l = light(Vec3::Y, 1.0);
        let c = Color::new(t.r * l[0], t.g * l[1], t.b * l[2], 1.0);
        let uv = if rot {
            [tile_uv(tile, 0.0, 0.0), tile_uv(tile, 0.0, 1.0), tile_uv(tile, 1.0, 1.0), tile_uv(tile, 1.0, 0.0)]
        } else {
            [tile_uv(tile, 0.0, 0.0), tile_uv(tile, 1.0, 0.0), tile_uv(tile, 1.0, 1.0), tile_uv(tile, 0.0, 1.0)]
        };
        let p = [vec3(x0, y, z0), vec3(x1, y, z0), vec3(x1, y, z1), vec3(x0, y, z1)];
        self.push(p, Vec3::Y, uv, tile, [c, c, c, c]);
    }

    // textured box prop standing on y0, full height hy, half extents hx/hz
    fn prop(&mut self, cx: f32, cz: f32, y0: f32, hx: f32, hy: f32, hz: f32, tile: usize, t: Color) {
        let y1 = y0 + hy;
        self.wall(cx - hx, cz - hz, cx + hx, cz - hz, y0, y1, tile, t, vec3(0.0, 0.0, -1.0));
        self.wall(cx + hx, cz + hz, cx - hx, cz + hz, y0, y1, tile, t, vec3(0.0, 0.0, 1.0));
        self.wall(cx - hx, cz + hz, cx - hx, cz - hz, y0, y1, tile, t, vec3(-1.0, 0.0, 0.0));
        self.wall(cx + hx, cz - hz, cx + hx, cz + hz, y0, y1, tile, t, vec3(1.0, 0.0, 0.0));
        self.floor(cx - hx, cz - hz, cx + hx, cz + hz, y1, tile, t, false);
    }

    fn building(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, floors: i32, kind: u8, seed: u32) {
        let palette = [
            tint(1.0, 1.0, 1.0),
            tint(1.0, 0.92, 0.82),
            tint(0.88, 0.95, 1.0),
            tint(0.95, 1.0, 0.88),
            tint(1.0, 0.88, 0.88),
            tint(0.92, 0.92, 0.98),
        ];
        let tn = palette[(hash(seed as i32, 3, 77) * 5.99) as usize];
        let h = floors as f32 * 4.0;
        let top = CURB + h;
        let upper = match kind {
            0 => T_FAC_OFFICE,
            _ => {
                if hash(seed as i32, 9, 5) > 0.55 {
                    T_BRICK
                } else {
                    T_FAC_RES
                }
            }
        };
        let faces: [(f32, f32, f32, f32, Vec3); 4] = [
            (x0, z0, x1, z0, vec3(0.0, 0.0, -1.0)),
            (x1, z1, x0, z1, vec3(0.0, 0.0, 1.0)),
            (x0, z1, x0, z0, vec3(-1.0, 0.0, 0.0)),
            (x1, z0, x1, z1, vec3(1.0, 0.0, 0.0)),
        ];
        for (ax, az, bx, bz, n) in faces {
            if kind == 1 {
                self.wall(ax, az, bx, bz, CURB, CURB + 4.0, T_FAC_SHOP, tn, n);
                self.wall(ax, az, bx, bz, CURB + 4.0, top, upper, tn, n);
            } else {
                self.wall(ax, az, bx, bz, CURB, top, upper, tn, n);
            }
        }
        self.floor(x0, z0, x1, z1, top, T_CONCRETE, tint(0.9, 0.9, 0.9), false);
        let (cx, cz) = ((x0 + x1) * 0.5, (z0 + z1) * 0.5);
        let (hx, hz) = ((x1 - x0) * 0.5, (z1 - z0) * 0.5);
        let pc = tint(0.8, 0.8, 0.78);
        self.prop(cx, z0 + 0.12, top, hx, 0.6, 0.12, T_CONCRETE, pc);
        self.prop(cx, z1 - 0.12, top, hx, 0.6, 0.12, T_CONCRETE, pc);
        self.prop(x0 + 0.12, cz, top, 0.12, 0.6, hz, T_CONCRETE, pc);
        self.prop(x1 - 0.12, cz, top, 0.12, 0.6, hz, T_CONCRETE, pc);
        if hx > 3.0 {
            self.prop(cx + hx * 0.4, cz - hz * 0.3, top, 1.0, 1.0, 0.8, T_METAL, tint(0.9, 0.9, 0.9));
            self.prop(cx - hx * 0.4, cz + hz * 0.3, top, 0.7, 0.9, 0.7, T_METAL, tint(0.8, 0.85, 0.9));
        }
        self.solid(x0, z0, x1, z1, top);
    }

    fn car(&mut self, cx: f32, cz: f32, along_x: bool, col: Color) {
        let (hx, hz) = if along_x { (2.1, 0.92) } else { (0.92, 2.1) };
        let (cxa, cza) = if along_x { (1.1, 0.8) } else { (0.8, 1.1) };
        self.prop(cx, cz, 0.30, hx, 0.62, hz, T_WHITE, col);
        self.prop(cx, cz, 0.92, cxa, 0.46, cza, T_WHITE, tint(0.16, 0.22, 0.30));
        let dark = tint(0.06, 0.06, 0.07);
        for sx in [-1.0f32, 1.0] {
            for sz in [-1.0f32, 1.0] {
                let (ox, oz) = if along_x { (sx * 1.35, sz * hz) } else { (sx * hx, sz * 1.35) };
                self.prop(cx + ox, cz + oz, 0.0, 0.32, 0.55, 0.32, T_WHITE, dark);
            }
        }
        self.solid(cx - hx, cz - hz, cx + hx, cz + hz, 1.35);
    }

    fn lamp(&mut self, x: f32, z: f32) {
        self.prop(x, z, CURB, 0.07, 5.0, 0.07, T_METAL, tint(0.55, 0.58, 0.6));
        self.prop(x, z, CURB + 4.9, 0.28, 0.12, 0.28, T_WHITE, tint(1.4, 1.3, 1.0));
        self.solid(x - 0.1, z - 0.1, x + 0.1, z + 0.1, 5.0);
    }

    fn tree(&mut self, x: f32, z: f32, s: u32) {
        let k = 0.8 + hash(s as i32, 1, 21) * 0.6;
        self.prop(x, z, CURB, 0.14, 2.2 * k, 0.14, T_WOOD, tint(0.8, 0.7, 0.6));
        let g = tint(0.5 + hash(s as i32, 2, 3) * 0.2, 0.9, 0.5);
        self.prop(x, z, CURB + 1.8 * k, 1.3 * k, 1.2 * k, 1.3 * k, T_GRASS, g);
        self.prop(x, z, CURB + 2.8 * k, 0.9 * k, 1.0 * k, 0.9 * k, T_GRASS, g);
        self.solid(x - 0.16, z - 0.16, x + 0.16, z + 0.16, 3.0);
    }

    fn crate_(&mut self, x: f32, z: f32, s: u32) {
        self.props.push(PropSpawn { kind: P_CRATE, x, z, y0: CURB, along_x: false, seed: s });
    }

    fn bench(&mut self, x: f32, z: f32, along_x: bool) {
        self.props.push(PropSpawn { kind: P_BENCH, x, z, y0: CURB, along_x, seed: 0 });
    }

    fn smash(&mut self, kind: u8, x: f32, z: f32, s: u32) {
        self.props.push(PropSpawn { kind, x, z, y0: CURB, along_x: false, seed: s });
    }

    fn block(&mut self, bi: usize, bj: usize) {
        let ox = bj as f32 * PERIOD + ROAD;
        let oz = bi as f32 * PERIOD + ROAD;
        let seed = (bi * 17 + bj * 31) as u32;
        let kind = ((bi * 5 + bj * 3) as u32 + (hash(bi as i32, bj as i32, 9) * 6.0) as u32) % 6;
        let kind = if bi == 2 && bj == 2 { 4 } else { kind };
        let inner_tile = match kind {
            2 | 3 => T_GRASS,
            1 => T_CONCRETE,
            _ => T_PAVING,
        };
        let sw = tint(1.0, 1.0, 1.0);
        // raised pad (curb + sidewalk ring + inner ground)
        self.solid(ox, oz, ox + 24.0, oz + 24.0, CURB);
        for cz in 0..6 {
            for cx in 0..6 {
                let border = cx == 0 || cz == 0 || cx == 5 || cz == 5;
                let tile = if border { T_SIDEWALK } else { inner_tile };
                let x = ox + cx as f32 * 4.0;
                let z = oz + cz as f32 * 4.0;
                self.floor(x, z, x + 4.0, z + 4.0, CURB, tile, sw, false);
            }
        }
        let cc = tint(0.85, 0.85, 0.83);
        self.wall(ox, oz, ox + 24.0, oz, 0.0, CURB, T_CONCRETE, cc, vec3(0.0, 0.0, -1.0));
        self.wall(ox + 24.0, oz + 24.0, ox, oz + 24.0, 0.0, CURB, T_CONCRETE, cc, vec3(0.0, 0.0, 1.0));
        self.wall(ox, oz + 24.0, ox, oz, 0.0, CURB, T_CONCRETE, cc, vec3(-1.0, 0.0, 0.0));
        self.wall(ox + 24.0, oz, ox + 24.0, oz + 24.0, 0.0, CURB, T_CONCRETE, cc, vec3(1.0, 0.0, 0.0));

        for (lx, lz) in [(2.0, 2.0), (22.0, 2.0), (2.0, 22.0), (22.0, 22.0)] {
            self.lamp(ox + lx, oz + lz);
        }
        for (k, (tx, tz)) in [(12.0f32, 1.8f32), (12.0, 22.2), (1.8, 12.0), (22.2, 12.0)].iter().enumerate() {
            if hash(seed as i32, k as i32, 4) > 0.45 {
                self.tree(ox + tx, oz + tz, seed + k as u32);
            }
        }

        // street clutter on the sidewalk ring: bins and rows of traffic cones
        if hash(seed as i32, 7, 61) > 0.35 {
            self.smash(P_BIN, ox + 3.3, oz + 1.4, seed + 100);
        }
        if hash(seed as i32, 8, 62) > 0.35 {
            self.smash(P_BIN, ox + 20.7, oz + 22.6, seed + 101);
        }
        if hash(seed as i32, 9, 63) > 0.4 {
            for k in 0..4 {
                self.smash(P_CONE, ox + 6.0 + k as f32 * 1.4, oz + 21.8, seed + 110 + k as u32);
            }
        }
        if hash(seed as i32, 10, 64) > 0.5 {
            for k in 0..3 {
                self.smash(P_CONE, ox + 1.8, oz + 6.0 + k as f32 * 1.5, seed + 120 + k as u32);
            }
        }

        let (ix0, iz0, ix1, iz1) = (ox + 4.0, oz + 4.0, ox + 20.0, oz + 20.0);
        let fl = |lo: i32, hi: i32, s: i32| lo + (hash(seed as i32, s, 12) * (hi - lo + 1) as f32) as i32;
        match kind {
            0 => {
                self.building(ix0 + 1.0, iz0 + 1.0, ix1 - 1.0, iz1 - 1.0, fl(4, 6, 1), 0, seed);
            }
            1 => {
                self.building(ix0, iz0 + 1.0, ix0 + 7.5, iz1 - 1.0, fl(2, 3, 2), 1, seed + 1);
                self.building(ix0 + 8.5, iz0 + 1.0, ix1, iz1 - 1.0, fl(2, 4, 3), 1, seed + 2);
            }
            2 => {
                for (k, (hx, hz)) in [(0.0f32, 0.0f32), (9.0, 0.0), (0.0, 9.0), (9.0, 9.0)].iter().enumerate() {
                    self.building(ix0 + hx, iz0 + hz, ix0 + hx + 7.0, iz0 + hz + 7.0, fl(2, 3, 4 + k as i32), 2, seed + 3 + k as u32);
                }
            }
            3 => {
                self.floor(ix0 + 7.0, iz0, ix0 + 9.0, iz1, CURB + 0.05, T_PAVING, sw, false);
                self.floor(ix0, iz0 + 7.0, ix1, iz0 + 9.0, CURB + 0.05, T_PAVING, sw, false);
                if hash(seed as i32, 5, 5) > 0.5 {
                    self.floor(ix0 + 2.0, iz0 + 2.0, ix1 - 2.0, iz1 - 2.0, CURB + 0.07, T_COURT, sw, false);
                    for zz in [iz0 + 2.0, iz1 - 2.0] {
                        self.prop((ix0 + ix1) * 0.5, zz, CURB, 0.06, 3.0, 0.06, T_METAL, tint(0.7, 0.7, 0.72));
                        self.prop((ix0 + ix1) * 0.5, zz, CURB + 2.6, 0.9, 0.55, 0.04, T_WHITE, tint(0.95, 0.95, 0.95));
                        self.solid((ix0 + ix1) * 0.5 - 0.1, zz - 0.1, (ix0 + ix1) * 0.5 + 0.1, zz + 0.1, 3.0);
                    }
                } else {
                    for k in 0..7 {
                        let fx = ix0 + 1.5 + hash(seed as i32, k, 31) * 13.0;
                        let fz = iz0 + 1.5 + hash(seed as i32, k, 32) * 13.0;
                        if (fx - (ix0 + 8.0)).abs() > 1.6 && (fz - (iz0 + 8.0)).abs() > 1.6 {
                            self.tree(fx, fz, seed + 40 + k as u32);
                        }
                    }
                    self.bench(ix0 + 5.0, iz0 + 6.2, true);
                    self.bench(ix0 + 11.0, iz0 + 9.8, true);
                    self.bench(ix0 + 6.2, iz0 + 11.0, false);
                }
            }
            4 => {
                let (cx, cz) = ((ix0 + ix1) * 0.5, (iz0 + iz1) * 0.5);
                self.prop(cx, cz, CURB, 3.0, 0.6, 3.0, T_CONCRETE, tint(0.9, 0.9, 0.9));
                self.prop(cx, cz, CURB + 0.5, 2.6, 0.1, 2.6, T_WHITE, tint(0.25, 0.5, 0.75));
                self.prop(cx, cz, CURB, 0.4, 2.0, 0.4, T_CONCRETE, tint(0.85, 0.85, 0.85));
                self.solid(cx - 3.0, cz - 3.0, cx + 3.0, cz + 3.0, CURB + 0.6);
                for (k, (kx, kz)) in [(2.0f32, 2.0f32), (14.0, 2.0), (2.0, 14.0), (14.0, 14.0)].iter().enumerate() {
                    self.prop(ix0 + kx, iz0 + kz, CURB, 1.0, 2.3, 1.0, T_WOOD, tint(1.0, 0.95, 0.9));
                    self.prop(ix0 + kx, iz0 + kz, CURB + 2.3, 1.3, 0.15, 1.3, T_METAL, tint(0.9, 0.35 + 0.15 * k as f32, 0.3));
                    self.solid(ix0 + kx - 1.0, iz0 + kz - 1.0, ix0 + kx + 1.0, iz0 + kz + 1.0, CURB + 2.45);
                }
            }
            _ => {
                for k in 0..3 {
                    let sx = ix0 + 2.0 + k as f32 * 5.0;
                    self.prop(sx, iz0 + 1.6, CURB, 1.4, 0.9, 0.8, T_WOOD, tint(1.0, 0.95, 0.9));
                    self.prop(sx, iz0 + 1.6, CURB + 2.2, 1.7, 0.12, 1.2, T_METAL, tint(0.35 + 0.3 * k as f32, 0.6, 0.85 - 0.25 * k as f32));
                    self.solid(sx - 1.4, iz0 + 0.8, sx + 1.4, iz0 + 2.4, CURB + 0.9);
                }
                let cols = [tint(0.8, 0.25, 0.2), tint(0.2, 0.4, 0.75), tint(0.85, 0.7, 0.2)];
                for k in 0..2 {
                    let cz = iz1 - 1.4 - k as f32 * 3.2;
                    self.prop(ix0 + 4.0, cz, CURB, 3.0, 2.6, 1.2, T_METAL, cols[(k + bi) % 3]);
                    self.solid(ix0 + 1.0, cz - 1.2, ix0 + 7.0, cz + 1.2, CURB + 2.6);
                }
                for k in 0..4 {
                    self.smash(P_BARREL, ox + 21.6, oz + 5.5 + k as f32 * 1.3, seed + 140 + k as u32);
                }
                for k in 0..9 {
                    let fx = ix0 + 9.0 + hash(seed as i32, k, 51) * 6.0;
                    let fz = iz0 + 5.0 + hash(seed as i32, k, 52) * 9.0;
                    self.crate_(fx.floor() + 0.5, fz.floor() + 0.5, seed + 60 + k as u32);
                }
            }
        }
    }

    fn roads(&mut self) {
        let asphalt = tint(1.0, 1.0, 1.0);
        let car_cols = [
            tint(0.75, 0.15, 0.15),
            tint(0.15, 0.30, 0.65),
            tint(0.85, 0.85, 0.85),
            tint(0.12, 0.12, 0.13),
            tint(0.80, 0.65, 0.15),
            tint(0.20, 0.50, 0.30),
        ];
        for cz in 0..GN as i32 {
            for cx in 0..GN as i32 {
                let road_x = cz % 4 == 0; // road runs along x
                let road_z = cx % 4 == 0; // road runs along z
                if !road_x && !road_z {
                    continue;
                }
                let x = cx as f32 * ROAD;
                let z = cz as f32 * ROAD;
                if road_x && road_z {
                    self.floor(x, z, x + ROAD, z + ROAD, 0.0, T_ASPHALT, asphalt, false);
                    continue;
                }
                let along_x = road_x;
                let k = if along_x { cx % 4 } else { cz % 4 };
                let (tile, rot) = if k == 2 {
                    (T_ASPH_LINE, along_x)
                } else {
                    (T_ZEBRA, !along_x)
                };
                self.floor(x, z, x + ROAD, z + ROAD, 0.0, tile, asphalt, rot);
                let hs = hash(cx, cz, 91);
                if k == 2 && hs < 0.55 {
                    let side = if hash(cx, cz, 92) > 0.5 { 1.6 } else { ROAD - 1.6 };
                    let col = car_cols[(hash(cx, cz, 93) * 5.99) as usize];
                    if along_x {
                        self.car(x + ROAD * 0.5, z + side, true, col);
                    } else {
                        self.car(x + side, z + ROAD * 0.5, false, col);
                    }
                }
                if (cx * 3 + cz * 5) % 5 == 0 {
                    self.spawns.push((x + ROAD * 0.5, z + ROAD * 0.5, along_x));
                }
            }
        }
    }
}

pub fn build_world() -> World {
    let mut b = B { q: HashMap::new(), boxes: Vec::new(), spawns: Vec::new(), props: Vec::new() };
    let data = include_bytes!("../assets/dockyard/dockyard.meshbin");
    let triangles = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let white_uv = tile_uv(T_WHITE, 0.5, 0.5);
    for raw in data[4..].chunks_exact(40).take(triangles) {
        let f = |n: usize| f32::from_le_bytes([raw[n], raw[n + 1], raw[n + 2], raw[n + 3]]);
        let col = Color::new(raw[36] as f32 / 255.0, raw[37] as f32 / 255.0, raw[38] as f32 / 255.0, raw[39] as f32 / 255.0);
        let vertex = |n: usize| SV { p: vec3(f(n), f(n + 1), f(n + 2)), uv: white_uv, c: col };
        let a = vertex(0);
        let c = vertex(6);
        b.push(&Quad { v: [a, vertex(3), c, c], n: Vec3::ZERO, tile: T_WHITE });
    }
    for line in include_str!("../assets/dockyard/dockyard-collisions.txt").lines() {
        let values: Vec<f32> = line.split(',').filter_map(|v| v.parse().ok()).collect();
        if values.len() == 5 {
            b.solid(values[0], values[1], values[2], values[3], values[4]);
        }
    }

    // Invisible wall-foot bounds follow the cargo perimeter and join at corners.
    // The player radius is handled by movement collision, so the planes stay flush
    // with the 52 m arena edge without creating visible geometry.
    let edge = 1.25;
    let wall_height = 8.0;
    b.solid(0.0, 0.0, edge, SIZE, wall_height);
    b.solid(SIZE - edge, 0.0, SIZE, SIZE, wall_height);
    b.solid(0.0, 0.0, SIZE, edge, wall_height);
    b.solid(0.0, SIZE - edge, SIZE, SIZE, wall_height);

    // Five armed red bots, plus four blue teammates beside the player spawn.
    let blue = [(23.1, 6.0), (24.7, 6.0), (27.3, 6.0), (28.9, 6.0)];
    for (x, z) in blue { b.spawns.push((x, z, false)); }
    let red = [(23.1, 46.0), (24.7, 46.0), (26.0, 46.0), (27.3, 46.0), (28.9, 46.0)];
    for (x, z) in red { b.spawns.push((x, z, true)); }

    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); GN * GN];
    for (i, bx) in b.boxes.iter().enumerate() {
        let gx0 = ((bx.x0 / GC).floor().max(0.0) as usize).min(GN - 1);
        let gx1 = ((bx.x1 / GC).floor().max(0.0) as usize).min(GN - 1);
        let gz0 = ((bx.z0 / GC).floor().max(0.0) as usize).min(GN - 1);
        let gz1 = ((bx.z1 / GC).floor().max(0.0) as usize).min(GN - 1);
        for gz in gz0..=gz1 {
            for gx in gx0..=gx1 {
                grid[gz * GN + gx].push(i);
            }
        }
    }
    let mut chunks = Vec::new();
    for ((kx, kz), quads) in b.q.into_iter() {
        let c = vec3((kx as f32 + 0.5) * CH, 8.0, (kz as f32 + 0.5) * CH);
        chunks.push(Chunk { c, quads });
    }
    World { boxes: b.boxes, grid, chunks, spawns: b.spawns, props: b.props }
}
