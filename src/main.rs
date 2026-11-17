use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// City constants. Everything is built from a 1 m cell grid.
//   road 8 | sidewalk 4 | two 8x8 lots | sidewalk 4 | road ...   (pitch 32)
// ---------------------------------------------------------------------------
const P: usize = 32; // block pitch (road + block)
const NB: usize = 9; // blocks per side
const GW: usize = NB * P + 8; // grid width in cells (296 m)

const CRATE_H: f32 = 0.5;
const CAR_H: f32 = 1.3;
const PLAYER_R: f32 = 0.2;
const MAX_DIST: f32 = 150.0;
const MAG: i32 = 12;
const GRAVITY: f32 = 11.5;
const JUMP_V: f32 = 4.6;
const WALK_SPEED: f32 = 2.8;
const SPRINT_SPEED: f32 = 5.2;
const SNEAK_SPEED: f32 = 1.5;
const SPRINT_TILT: f32 = 0.9; // joystick pushed this far = running

// texture ids (1-based, 0 = untextured)
const T_ASPHALT: usize = 1;
const T_SIDEWALK: usize = 2;
const T_CONCRETE: usize = 3;
const T_GRASS: usize = 4;
const T_FAC0: usize = 5; // concrete facade
const T_CRATE: usize = 8;

fn height_at(x: f32, z: f32) -> f32 {
    if x < 0.0 || z < 0.0 {
        return 99.0;
    }
    let (ix, iz) = (x as usize, z as usize);
    if ix >= GW || iz >= GW {
        return 99.0;
    }
    city().h[iz * GW + ix]
}

// ---------------------------------------------------------------------------
// Tiny deterministic RNG + tileable noise (no extra crates needed)
// ---------------------------------------------------------------------------
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / 16777216.0
    }
}

fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ seed.wrapping_mul(2246822519);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) & 0xFFFF) as f32 / 65535.0
}

// value noise that wraps every px / py lattice cells, so textures tile
fn vn(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let xf = x.floor();
    let yf = y.floor();
    let fx = x - xf;
    let fy = y - yf;
    let (xi, yi) = (xf as i32, yf as i32);
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let g = |a: i32, b: i32| hash2(a.rem_euclid(px), b.rem_euclid(py), seed);
    let (a, b, c, d) = (g(xi, yi), g(xi + 1, yi), g(xi, yi + 1), g(xi + 1, yi + 1));
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

fn fbm(u: f32, v: f32, f: i32, oct: u32, seed: u32) -> f32 {
    let mut s = 0.0;
    let mut a = 0.5;
    let mut norm = 0.0;
    let mut fr = f;
    for o in 0..oct {
        s += a * vn(u * fr as f32, v * fr as f32, fr, fr, seed + o * 17);
        norm += a;
        a *= 0.5;
        fr *= 2;
    }
    s / norm
}

fn speck(u: f32, v: f32, n: i32, seed: u32) -> f32 {
    hash2((u * n as f32) as i32, (v * n as f32) as i32, seed)
}
fn fr(x: f32) -> f32 {
    x - x.floor()
}
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
fn sc(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

// ---------------------------------------------------------------------------
// Procedural textures (generated at start-up, no asset files needed)
// ---------------------------------------------------------------------------
fn make_tex(size: usize, f: impl Fn(f32, f32) -> [f32; 3]) -> Texture2D {
    let mut bytes = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let c = f((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
            bytes.push((c[0].clamp(0.0, 1.0) * 255.0) as u8);
            bytes.push((c[1].clamp(0.0, 1.0) * 255.0) as u8);
            bytes.push((c[2].clamp(0.0, 1.0) * 255.0) as u8);
            bytes.push(255);
        }
    }
    let img = Image { bytes, width: size as u16, height: size as u16 };
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    t
}

fn tex_asphalt() -> Texture2D {
    make_tex(256, |u, v| {
        let m = fbm(u, v, 4, 4, 11);
        let s = speck(u, v, 256, 3);
        let mut g = 0.15 + 0.10 * m + (s - 0.5) * 0.09;
        if s > 0.985 {
            g += 0.16; // loose gravel
        }
        let st = fbm(u, v, 3, 3, 5); // oil / wear patches
        if st > 0.60 {
            g *= 1.0 - (st - 0.60) * 1.8;
        }
        let cr = (fbm(u, v, 6, 3, 9) - 0.5).abs(); // hairline cracks
        if cr < 0.01 {
            g *= 0.45;
        }
        [g * 1.02, g, g * 1.05]
    })
}

fn tex_sidewalk() -> Texture2D {
    make_tex(256, |u, v| {
        let (su, sv) = (u * 2.0, v * 2.0);
        let tint = hash2(su.floor() as i32, sv.floor() as i32, 21);
        let m = fbm(u, v, 4, 4, 22);
        let s = speck(u, v, 256, 23);
        let mut g = 0.50 + 0.10 * m + (tint - 0.5) * 0.07 + (s - 0.5) * 0.06;
        let (fu, fv) = (fr(su), fr(sv));
        let j = fu.min(1.0 - fu).min(fv).min(1.0 - fv);
        if j < 0.015 {
            g *= 0.55;
        } else if j < 0.03 {
            g *= 0.9;
        }
        let st = fbm(u, v, 3, 3, 24);
        if st > 0.6 {
            g *= 1.0 - (st - 0.6) * 1.4;
        }
        [g, g * 0.99, g * 0.96]
    })
}

fn tex_concrete() -> Texture2D {
    make_tex(256, |u, v| {
        let m = fbm(u, v, 5, 4, 51);
        let s = speck(u, v, 256, 52);
        let mut g = 0.44 + 0.12 * m + (s - 0.5) * 0.07;
        let e = u.min(1.0 - u).min(v).min(1.0 - v);
        if e < 0.008 {
            g *= 0.6;
        }
        let st = fbm(u, v, 3, 3, 53);
        if st > 0.58 {
            g *= 1.0 - (st - 0.58) * 1.6;
        }
        [g * 0.98, g, g * 1.03]
    })
}

fn tex_grass() -> Texture2D {
    make_tex(256, |u, v| {
        let m = fbm(u, v, 4, 4, 31);
        let blade = vn(u * 128.0, v * 16.0, 128, 16, 32);
        let s = speck(u, v, 256, 33);
        let dry = fbm(u, v, 2, 3, 34);
        let base = mix([0.12, 0.25, 0.07], [0.30, 0.33, 0.12], dry * 0.9);
        sc(base, 0.95 + 0.35 * (m - 0.5) + 0.45 * (blade - 0.5) + 0.30 * (s - 0.5))
    })
}

fn tex_crate() -> Texture2D {
    make_tex(256, |u, v| {
        let grain = |seed: u32, k: f32| vn(u * 6.0, v * 48.0 + k, 6, 48, seed);
        let s = speck(u, v, 256, 61);
        let border = u < 0.10 || u > 0.90 || v < 0.10 || v > 0.90;
        let brace = ((u - v).abs() < 0.05 || (u + v - 1.0).abs() < 0.05) && !border;
        let mut c;
        if border || brace {
            c = sc([0.44, 0.29, 0.15], 0.8 + 0.4 * grain(62, 0.0) + (s - 0.5) * 0.1);
        } else {
            let pi = (u * 5.0).floor();
            let tone = hash2(pi as i32, 0, 63);
            c = sc([0.58, 0.40, 0.21], 0.75 + 0.35 * tone + 0.35 * grain(64, pi * 7.0) + (s - 0.5) * 0.1);
            if fr(u * 5.0) < 0.035 {
                c = sc(c, 0.35);
            }
        }
        // nails
        for (nx, ny) in [(0.05f32, 0.05f32), (0.95, 0.05), (0.05, 0.95), (0.95, 0.95)] {
            if (u - nx).powi(2) + (v - ny).powi(2) < 0.0003 {
                c = [0.25, 0.25, 0.27];
            }
        }
        c
    })
}

// style 0 = concrete, 1 = brick, 2 = glass curtain wall. One tile = 8 m x 8 m
// (2 bays x 2 floors), anchored at the bottom so floors line up on tall walls.
fn tex_facade(style: u8) -> Texture2D {
    make_tex(512, move |u, v| {
        let xm = u * 8.0;
        let ym = (1.0 - v) * 8.0;
        let idx = (u * 2.0).floor() as i32;
        let lu = fr(u * 2.0);
        let fy = (ym / 4.0).floor() as i32;
        let lv = fr(ym / 4.0); // 0 = floor, 1 = ceiling
        let id = hash2(idx, fy, 40 + style as u32);
        let s = speck(u, v, 512, 41);

        if style == 2 {
            if lv < 0.16 {
                let n = fbm(u, v, 8, 3, 42);
                return sc([0.30, 0.36, 0.38], 0.85 + 0.3 * n + (s - 0.5) * 0.08);
            }
            let pu = fr(u * 4.0);
            if pu < 0.025 || pu > 0.975 {
                return [0.58, 0.60, 0.61];
            }
            let wv = ((lv - 0.16) / 0.84).clamp(0.0, 1.0);
            let cl = fbm(u, v, 6, 4, 43);
            let sky = mix([0.45, 0.60, 0.72], [0.80, 0.87, 0.93], cl);
            let mut c = mix([0.07, 0.12, 0.15], sky, ((wv * 1.1).min(1.0)) * 0.85 * (0.5 + cl * 0.6));
            if id > 0.9 {
                c = mix(c, [0.9, 0.75, 0.45], 0.55);
            }
            return sc(c, 0.95 + (s - 0.5) * 0.06);
        }

        // wall surface
        let mut c;
        if style == 1 {
            let row = (ym / 0.1).floor() as i32;
            let off = if row % 2 == 0 { 0.0 } else { 0.16 };
            let bu = (xm + off) / 0.32;
            let col = (bu.floor() as i32).rem_euclid(25);
            let (fu, fvv) = (fr(bu), fr(ym / 0.1));
            if fu < 0.07 || fvv < 0.12 {
                c = sc([0.60, 0.58, 0.54], 0.9 + 0.2 * s);
            } else {
                let tone = hash2(col, row.rem_euclid(80), 44);
                c = sc(
                    mix([0.50, 0.22, 0.16], [0.66, 0.34, 0.24], tone),
                    0.88 + 0.2 * fbm(u, v, 16, 2, 45) + (s - 0.5) * 0.12,
                );
            }
        } else {
            let m = fbm(u, v, 8, 4, 46);
            let mut g = 0.60 + 0.12 * (m - 0.5) + (s - 0.5) * 0.05;
            if lv < 0.02 || lu < 0.008 || lu > 0.992 {
                g *= 0.8; // panel joints
            }
            if lu > 0.2 && lu < 0.8 && lv < 0.28 {
                let st = vn(u * 128.0, v * 4.0, 128, 4, 47); // rain streaks under the sill
                g *= 1.0 - 0.28 * st * (1.0 - lv / 0.28);
            }
            c = [g, g * 0.99, g * 0.95];
        }

        // punched window
        let wu = (lu - 0.20) / 0.60;
        let wt = (0.84 - lv) / 0.56; // 0 = top, 1 = bottom
        if wu >= 0.0 && wu <= 1.0 && wt >= 0.0 && wt <= 1.0 {
            if wu < 0.07 || wu > 0.93 || wt < 0.07 || wt > 0.93 || (wu - 0.5).abs() < 0.025 {
                return [0.80, 0.80, 0.77];
            }
            let mut g = mix([0.55, 0.68, 0.80], [0.06, 0.08, 0.11], (wt * 1.2).min(1.0));
            if fr((wu * 0.9 - wt * 0.6) * 2.0) < 0.12 {
                g = sc(g, 1.25); // glare streak
            }
            if id > 0.82 {
                g = mix(g, [0.88, 0.68, 0.38], 0.75 * (1.0 - wt * 0.4)); // lights on
            } else if id > 0.58 && wt < 0.35 + 0.4 * id {
                g = [0.66, 0.62, 0.55]; // blinds half drawn
            }
            if wt < 0.14 {
                g = sc(g, 0.7); // recess shadow
            }
            return g;
        }
        if lv >= 0.22 && lv < 0.28 && wu > -0.06 && wu < 1.06 {
            return [0.78, 0.77, 0.74]; // sill
        }
        if style == 1 && lv >= 0.84 && lv < 0.89 && wu > -0.06 && wu < 1.06 {
            return [0.70, 0.69, 0.66]; // stone lintel
        }
        c
    })
}

fn make_textures() -> Vec<Texture2D> {
    vec![
        tex_asphalt(),
        tex_sidewalk(),
        tex_concrete(),
        tex_grass(),
        tex_facade(0),
        tex_facade(1),
        tex_facade(2),
        tex_crate(),
    ]
}

// ---------------------------------------------------------------------------
// The city: collision heights, ground kinds and a list of things to draw
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
enum Prop {
    Building { x0: f32, z0: f32, x1: f32, z1: f32, h: f32, style: u8 },
    Tree { x: f32, z: f32, s: f32 },
    Car { x: f32, z: f32, along_z: bool, col: u8 },
    Crate { x: f32, z: f32 },
    Lamp { x: f32, z: f32 },
}

struct City {
    h: Vec<f32>,
    kind: Vec<u8>, // 0 asphalt, 1 sidewalk, 2 concrete, 3 grass
    props: Vec<Prop>,
    spots: Vec<(f32, f32)>,
}

static CITY: OnceLock<City> = OnceLock::new();
fn city() -> &'static City {
    CITY.get_or_init(build_city)
}

fn mark(h: &mut [f32], x0: usize, z0: usize, w: usize, d: usize, v: f32) {
    for z in z0..(z0 + d).min(GW) {
        for x in x0..(x0 + w).min(GW) {
            if h[z * GW + x] < v {
                h[z * GW + x] = v;
            }
        }
    }
}

fn build_city() -> City {
    let mut rng = Rng(0x2545F4914F6CDD1D);
    let mut h = vec![0.0f32; GW * GW];
    let mut kind = vec![2u8; GW * GW];
    let mut props: Vec<Prop> = Vec::new();
    let gwf = GW as f32;
    let cen = gwf * 0.5;

    for z in 0..GW {
        for x in 0..GW {
            let (lx, lz) = (x % P, z % P);
            kind[z * GW + x] = if lx < 8 || lz < 8 {
                0
            } else if lx - 8 < 4 || lx - 8 >= 20 || lz - 8 < 4 || lz - 8 >= 20 {
                1
            } else {
                2
            };
            if x == 0 || z == 0 || x == GW - 1 || z == GW - 1 {
                h[z * GW + x] = 9.0; // city boundary wall
            }
        }
    }
    for (x0, z0, x1, z1) in [(0.0, 0.0, gwf, 1.0), (0.0, gwf - 1.0, gwf, gwf), (0.0, 0.0, 1.0, gwf), (gwf - 1.0, 0.0, gwf, gwf)] {
        props.push(Prop::Building { x0, z0, x1, z1, h: 9.0, style: 0 });
    }

    for bx in 0..NB {
        for bz in 0..NB {
            let ox = (bx * P + 8) as f32;
            let oz = (bz * P + 8) as f32;
            let d = (((ox + 12.0 - cen).powi(2) + (oz + 12.0 - cen).powi(2)).sqrt() / cen).min(1.0);
            let center = bx == NB / 2 && bz == NB / 2;

            for i in 0..2 {
                for j in 0..2 {
                    let lx0 = ox + 4.0 + i as f32 * 8.0;
                    let lz0 = oz + 4.0 + j as f32 * 8.0;
                    let roll = rng.f();
                    let lot = if center {
                        1
                    } else if d < 0.35 {
                        if roll < 0.85 { 0 } else if roll < 0.92 { 1 } else { 2 }
                    } else if roll < 0.62 {
                        0
                    } else if roll < 0.77 {
                        1
                    } else if roll < 0.90 {
                        2
                    } else {
                        3
                    };
                    let (ix, iz) = (lx0 as usize, lz0 as usize);
                    match lot {
                        0 => {
                            let mut inset = || if rng.f() < 0.35 { 1.0f32 } else { 0.0 };
                            let (a, b, c, e) = (inset(), inset(), inset(), inset());
                            let (x0, x1, z0, z1) = (lx0 + a, lx0 + 8.0 - b, lz0 + c, lz0 + 8.0 - e);
                            let hb = 8.0 + (1.0 - d).powf(1.6) * 58.0 * (0.25 + 0.75 * rng.f()) + rng.f() * 8.0;
                            let bh = ((hb / 4.0).round() * 4.0).max(8.0);
                            let style = if bh > 32.0 && rng.f() < 0.7 {
                                2
                            } else if rng.f() < 0.5 {
                                1
                            } else {
                                0
                            };
                            mark(&mut h, x0 as usize, z0 as usize, (x1 - x0) as usize, (z1 - z0) as usize, bh);
                            props.push(Prop::Building { x0, z0, x1, z1, h: bh, style });
                        }
                        1 => {
                            for zz in 0..8 {
                                for xx in 0..8 {
                                    kind[(iz + zz) * GW + ix + xx] = 3;
                                }
                            }
                            let n = if center { 9 } else { 4 + (rng.f() * 3.0) as usize };
                            for _ in 0..n {
                                let tx = ix + 1 + (rng.f() * 6.0) as usize;
                                let tz = iz + 1 + (rng.f() * 6.0) as usize;
                                if h[tz * GW + tx] == 0.0 {
                                    h[tz * GW + tx] = 1.0;
                                    props.push(Prop::Tree { x: tx as f32 + 0.5, z: tz as f32 + 0.5, s: 0.8 + 0.5 * rng.f() });
                                }
                            }
                        }
                        2 => {
                            let n = 3 + (rng.f() * 4.0) as usize;
                            for _ in 0..n {
                                let tx = ix + (rng.f() * 8.0) as usize;
                                let tz = iz + (rng.f() * 8.0) as usize;
                                if h[tz * GW + tx] == 0.0 {
                                    h[tz * GW + tx] = CRATE_H;
                                    props.push(Prop::Crate { x: tx as f32 + 0.5, z: tz as f32 + 0.5 });
                                }
                            }
                        }
                        _ => {
                            for zz in 0..8 {
                                for xx in 0..8 {
                                    kind[(iz + zz) * GW + ix + xx] = 0;
                                }
                            }
                            for k in 0..2usize {
                                if rng.f() < 0.75 {
                                    let xs = ix + 1 + k * 4;
                                    let zs = iz + 1 + (rng.f() * 2.0) as usize;
                                    mark(&mut h, xs, zs, 2, 5, CAR_H);
                                    props.push(Prop::Car { x: xs as f32 + 1.0, z: zs as f32 + 2.5, along_z: true, col: (rng.next() % 8) as u8 });
                                }
                            }
                        }
                    }
                }
            }

            // street lamps on the sidewalks
            for t in [6.0f32, 18.0] {
                props.push(Prop::Lamp { x: ox + 0.7, z: oz + t });
                props.push(Prop::Lamp { x: ox + 23.3, z: oz + t });
                props.push(Prop::Lamp { x: ox + t, z: oz + 0.7 });
                props.push(Prop::Lamp { x: ox + t, z: oz + 23.3 });
            }
        }
    }

    // parked cars along the curb lanes of every road (centre lanes stay clear)
    for r in 0..=NB {
        let base = r * P;
        for seg in 0..NB {
            let s0 = seg * P + 8;
            for side in 0..2usize {
                if (r == 0 && side == 0) || (r == NB && side == 1) {
                    continue;
                }
                let off = if side == 0 { 0 } else { 6 };
                for k in 0..3usize {
                    let t0 = s0 + 4 + k * 5;
                    let col = (rng.next() % 8) as u8;
                    if rng.f() >= 0.45 {
                        mark(&mut h, base + off, t0, 2, 5, CAR_H);
                        props.push(Prop::Car { x: (base + off) as f32 + 1.0, z: t0 as f32 + 2.5, along_z: true, col });
                    }
                    if rng.f() >= 0.45 {
                        mark(&mut h, t0, base + off, 5, 2, CAR_H);
                        props.push(Prop::Car { x: t0 as f32 + 2.5, z: (base + off) as f32 + 1.0, along_z: false, col: (col + 3) % 8 });
                    }
                }
            }
        }
    }

    // shooting-range targets scattered along the streets
    let mut spots = Vec::new();
    for _ in 0..36 {
        let r = ((rng.f() * (NB + 1) as f32) as usize).min(NB);
        let seg = ((rng.f() * NB as f32) as usize).min(NB - 1);
        let t = (seg * P + 8) as f32 + 3.0 + rng.f() * 18.0;
        let c = (r * P) as f32 + 4.0;
        if rng.f() < 0.5 {
            spots.push((c, t));
        } else {
            spots.push((t, c));
        }
    }

    City { h, kind, props, spots }
}

// ---------------------------------------------------------------------------
// Batched 3D drawing: boxes / quads collected into few meshes, with texture
// switching and distance fade (the sky shows through, which reads as haze).
// ---------------------------------------------------------------------------
struct Batch {
    v: Vec<Vertex>,
    i: Vec<u16>,
    tex: usize,
    texs: Vec<Texture2D>,
    cam: Vec3,
    fs: f32, // fade start distance
    fe: f32, // fully faded distance
}

impl Batch {
    fn new(texs: Vec<Texture2D>, cam: Vec3) -> Self {
        Self { v: Vec::new(), i: Vec::new(), tex: 0, texs, cam, fs: 1.0e6, fe: 2.0e6 }
    }

    fn flush(&mut self) {
        if self.v.is_empty() {
            return;
        }
        let texture = if self.tex == 0 { None } else { Some(self.texs[self.tex - 1].clone()) };
        let mesh = Mesh { vertices: std::mem::take(&mut self.v), indices: std::mem::take(&mut self.i), texture };
        draw_mesh(&mesh);
    }

    fn set_tex(&mut self, t: usize) {
        if t != self.tex {
            self.flush();
            self.tex = t;
        }
    }

    fn fog(&self, p: Vec3) -> f32 {
        let d = (p - self.cam).length();
        1.0 - ((d - self.fs) / (self.fe - self.fs)).clamp(0.0, 1.0)
    }

    fn tquad(&mut self, tex: usize, p: [Vec3; 4], uv: [(f32, f32); 4], col: Color) {
        let f = [self.fog(p[0]), self.fog(p[1]), self.fog(p[2]), self.fog(p[3])];
        if f.iter().all(|a| *a < 0.01) {
            return;
        }
        self.set_tex(tex);
        let base = self.v.len() as u16;
        for k in 0..4 {
            let c = Color::new(col.r, col.g, col.b, col.a * f[k]);
            self.v.push(Vertex::new(p[k].x, p[k].y, p[k].z, uv[k].0, uv[k].1, c));
        }
        self.i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if self.v.len() > 6000 {
            self.flush();
        }
    }

    fn quad(&mut self, p: [Vec3; 4], col: Color) {
        self.tquad(0, p, [(0.0, 0.0); 4], col);
    }

    // Box from three half-extent axis vectors (so it can be rotated / tilted),
    // with simple fake lighting per face.
    fn boxed(&mut self, c: Vec3, ax: Vec3, ay: Vec3, az: Vec3, col: Color) {
        let light = vec3(0.39, 0.87, 0.29);
        let axes = [ax, ay, az];
        for k in 0..3 {
            let n = axes[k];
            let u = axes[(k + 1) % 3];
            let v = axes[(k + 2) % 3];
            for s in [-1.0f32, 1.0] {
                let fc = c + n * s;
                let nn = (n * s).normalize_or_zero();
                let shade = 0.55 + 0.45 * nn.dot(light).max(0.0);
                let fcol = Color::new(col.r * shade, col.g * shade, col.b * shade, col.a);
                self.quad([fc - u - v, fc + u - v, fc + u + v, fc - u + v], fcol);
            }
        }
    }

    fn aabb(&mut self, c: Vec3, half: Vec3, col: Color) {
        self.boxed(c, vec3(half.x, 0.0, 0.0), vec3(0.0, half.y, 0.0), vec3(0.0, 0.0, half.z), col);
    }

    // Box rotated around the Y axis. hf = half size forward, hu = up, hr = right.
    fn obox(&mut self, c: Vec3, yaw: f32, hf: f32, hu: f32, hr: f32, col: Color) {
        let f = vec3(yaw.cos(), 0.0, yaw.sin());
        let r = vec3(-yaw.sin(), 0.0, yaw.cos());
        self.boxed(c, f * hf, vec3(0.0, hu, 0.0), r * hr, col);
    }

    // Square-ish bar between two points (arms, etc).
    fn limb(&mut self, a: Vec3, b: Vec3, t: f32, col: Color) {
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
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------
struct Target {
    x0: f32,
    x: f32,
    z: f32,
    alive: bool,
    phase: f32,
    respawn: f32,
}

impl Target {
    fn new(x: f32, z: f32, phase: f32) -> Self {
        Self { x0: x, x, z, alive: true, phase, respawn: 0.0 }
    }
}

struct Game {
    x: f32,
    y: f32,
    z: f32,
    vy: f32,
    grounded: bool,
    yaw: f32,
    pitch: f32, // positive = looking up
    zoom: f32,  // 0 = normal, 1 = fully scoped
    boost: f32, // 0..1, widens the view while running / sliding
    health: i32,
    ammo: i32,
    reserve: i32,
    reloading: f32,
    fire_cd: f32,
    scoped: bool,
    sneak_on: bool,
    sneaking: bool,
    sprint: bool,
    sliding: bool,
    slide_t: f32,
    slide_dur: f32,
    slide_cd: f32,
    slide_dir: Vec2,
    walk: f32,
    moving: f32,
    kills: u32,
    targets: Vec<Target>,
    tracer: Option<(Vec3, Vec3)>,
    tracer_t: f32,
    hit_t: f32,
}

impl Game {
    fn new() -> Self {
        Self {
            x: 132.0,
            y: 0.0,
            z: 132.0,
            vy: 0.0,
            grounded: true,
            yaw: -0.5,
            pitch: 0.0,
            zoom: 0.0,
            boost: 0.0,
            health: 100,
            ammo: MAG,
            reserve: 60,
            reloading: 0.0,
            fire_cd: 0.0,
            scoped: false,
            sneak_on: false,
            sneaking: false,
            sprint: false,
            sliding: false,
            slide_t: 0.0,
            slide_dur: 0.6,
            slide_cd: 0.0,
            slide_dir: Vec2::ZERO,
            walk: 0.0,
            moving: 0.0,
            kills: 0,
            targets: city()
                .spots
                .iter()
                .enumerate()
                .map(|(i, &(x, z))| Target::new(x, z, i as f32 * 1.3))
                .collect(),
            tracer: None,
            tracer_t: 0.0,
            hit_t: 0.0,
        }
    }

    // body height factor (squashed when sneaking / sliding)
    fn k(&self) -> f32 {
        if self.sliding {
            0.5
        } else if self.sneaking {
            0.78
        } else {
            1.0
        }
    }

    fn blocked(&self, x: f32, z: f32) -> bool {
        let r = PLAYER_R;
        [(-r, -r), (r, -r), (-r, r), (r, r)]
            .iter()
            .any(|(ox, oz)| height_at(x + ox, z + oz) > self.y + 0.05)
    }

    fn move_by(&mut self, dx: f32, dz: f32) {
        if !self.blocked(self.x + dx, self.z) {
            self.x += dx;
        }
        if !self.blocked(self.x, self.z + dz) {
            self.z += dz;
        }
    }

    // Height of the floor / crate top the player can stand on.
    fn ground(&self, ref_y: f32) -> f32 {
        let r = PLAYER_R;
        let mut best = 0.0f32;
        for (ox, oz) in [(-r, -r), (r, -r), (-r, r), (r, r)] {
            let hh = height_at(self.x + ox, self.z + oz);
            if hh <= ref_y + 0.05 && hh > best {
                best = hh;
            }
        }
        best
    }

    fn aim_dir(&self) -> Vec3 {
        let cp = self.pitch.cos();
        vec3(cp * self.yaw.cos(), self.pitch.sin(), cp * self.yaw.sin())
    }

    fn right(&self) -> Vec3 {
        vec3(-self.yaw.sin(), 0.0, self.yaw.cos())
    }

    fn pivot(&self) -> Vec3 {
        let hgt = if self.sliding {
            0.40
        } else if self.sneaking {
            0.62
        } else {
            0.78
        };
        vec3(self.x, self.y + hgt, self.z)
    }

    fn gun_center(&self) -> Vec3 {
        let f = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        vec3(self.x, self.y + 0.56 * self.k(), self.z) + f * 0.38 + self.right() * 0.20
    }

    // Over-the-shoulder camera that slides in when a wall is behind you.
    fn cam_pos(&self) -> Vec3 {
        let d = self.aim_dir();
        let r = self.right();
        let dist = 2.4 + (0.9 - 2.4) * self.zoom;
        let shoulder = 0.35 + (0.28 - 0.35) * self.zoom;
        let pivot = self.pivot();
        let want = pivot - d * dist + r * shoulder + vec3(0.0, 0.15, 0.0);
        let mut best = want;
        let steps = 24;
        for i in 1..=steps {
            let f = i as f32 / steps as f32;
            let p = pivot + (want - pivot) * f;
            if height_at(p.x, p.z) > p.y {
                let back = (i - 1) as f32 / steps as f32;
                best = pivot + (want - pivot) * back;
                break;
            }
        }
        if best.y < 0.12 {
            best.y = 0.12;
        }
        best
    }

    fn start_reload(&mut self) {
        if self.reloading <= 0.0 && self.ammo < MAG && self.reserve > 0 {
            self.reloading = 1.25;
        }
    }

    fn shoot(&mut self) {
        if self.fire_cd > 0.0 || self.reloading > 0.0 {
            return;
        }
        if self.ammo <= 0 {
            if self.reserve <= 0 {
                self.reserve = 60; // training yard resupply
            }
            self.start_reload();
            return;
        }
        self.ammo -= 1;
        self.fire_cd = 0.18;

        // Hitscan from the camera through the crosshair.
        let o = self.cam_pos();
        let d = self.aim_dir();
        let mut end = o + d * MAX_DIST;
        let mut hit: Option<usize> = None;
        let mut t = 0.0f32;
        'march: while t < MAX_DIST {
            t += 0.08;
            let p = o + d * t;
            if p.y < 0.0 || height_at(p.x, p.z) > p.y {
                end = p;
                break;
            }
            for (i, tg) in self.targets.iter().enumerate() {
                if !tg.alive {
                    continue;
                }
                let dx = p.x - tg.x;
                let dz = p.z - tg.z;
                if dx * dx + dz * dz < 0.09 && p.y <= 1.05 {
                    hit = Some(i);
                    end = p;
                    break 'march;
                }
            }
        }
        if let Some(i) = hit {
            self.targets[i].alive = false;
            self.targets[i].respawn = 4.0;
            self.kills += 1;
            self.hit_t = 0.25;
        }

        let muzzle = self.gun_center() + d * 0.46;
        self.tracer = Some((muzzle, end));
        self.tracer_t = 0.07;
    }
}

// ---------------------------------------------------------------------------
// HUD layout (matches the reference picture) + user customisation
// ---------------------------------------------------------------------------
const REF_W: f32 = 2576.0;
const REF_H: f32 = 1450.0;
// (anchored to right edge, x, y, radius) in reference-picture pixels
const BASE: [(bool, f32, f32, f32); 6] = [
    (false, 315.0, 1148.0, 223.0),  // joystick
    (true, 2093.0, 750.0, 90.0),    // scope
    (true, 2415.0, 750.0, 115.0),   // jump
    (true, 2175.0, 1067.0, 175.0),  // fire
    (true, 1798.0, 1153.0, 79.0),   // reload
    (true, 2432.0, 1277.0, 94.0),   // sneak / slide
];
const NUDGE: f32 = 25.0;

#[derive(Clone, Copy, PartialEq)]
enum Elem {
    Joy = 0,
    Scope = 1,
    Jump = 2,
    Fire = 3,
    Reload = 4,
    Snk = 5,
}

const ELEMS: [Elem; 6] = [Elem::Joy, Elem::Scope, Elem::Jump, Elem::Fire, Elem::Reload, Elem::Snk];

impl Elem {
    fn name(self) -> &'static str {
        match self {
            Elem::Joy => "Joystick",
            Elem::Scope => "Scope",
            Elem::Jump => "Jump",
            Elem::Fire => "Fire",
            Elem::Reload => "Reload",
            Elem::Snk => "Snk/Slide",
        }
    }
}

#[derive(Clone, Copy)]
struct ElemCfg {
    dx: f32,
    dy: f32,
    scale: f32,
    opacity: f32,
}

impl ElemCfg {
    fn new() -> Self {
        Self { dx: 0.0, dy: 0.0, scale: 1.0, opacity: 0.8 }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Play,
    Settings,
    Edit,
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Joy,
    Look,
    Btn,
    Win,
    Drag,
}

struct Pad {
    c: Vec2,
    r: f32,
    a: f32,
}

struct Layout {
    u: f32,
    s: f32,
    w: f32,
    h: f32,
    pads: [Pad; 6],
    settings: Pad,
}

impl Layout {
    fn new(w: f32, h: f32, cfg: &[ElemCfg; 6]) -> Self {
        let u = (h / 720.0).clamp(0.75, 1.8);
        let s = h / REF_H;
        let mk = |i: usize| -> Pad {
            let (right, x, y, r) = BASE[i];
            let cx = if right { w - (REF_W - x) * s } else { x * s };
            let c = cfg[i];
            Pad { c: vec2(cx + c.dx * s, y * s + c.dy * s), r: r * s * c.scale, a: c.opacity }
        };
        let st_r = (60.0 * s).max(34.0 * u);
        let st_x = (w - (REF_W - 2508.0) * s).min(w - st_r - 6.0 * u);
        let st_y = (65.0 * s).max(st_r + 6.0 * u);
        Self {
            u,
            s,
            w,
            h,
            pads: [mk(0), mk(1), mk(2), mk(3), mk(4), mk(5)],
            settings: Pad { c: vec2(st_x, st_y), r: st_r, a: 0.8 },
        }
    }

    // buttons only (not the joystick)
    fn hit(&self, p: Vec2) -> Option<Elem> {
        let mut best = None;
        let mut best_d = f32::MAX;
        for i in 1..6 {
            let pad = &self.pads[i];
            let d = p.distance(pad.c);
            if d <= pad.r * 1.15 && d < best_d {
                best = Some(ELEMS[i]);
                best_d = d;
            }
        }
        best
    }

    // buttons + joystick (used by the HUD editor)
    fn hit_any(&self, p: Vec2) -> Option<Elem> {
        if let Some(e) = self.hit(p) {
            return Some(e);
        }
        if p.distance(self.pads[0].c) <= self.pads[0].r {
            return Some(Elem::Joy);
        }
        None
    }

    // where element i sits with no customisation
    fn base_c(&self, i: usize) -> Vec2 {
        let (right, x, y, _) = BASE[i];
        vec2(if right { self.w - (REF_W - x) * self.s } else { x * self.s }, y * self.s)
    }

    // put element i's centre at c (kept fully on screen)
    fn place(&self, i: usize, c: Vec2, cfg: &mut [ElemCfg; 6]) {
        let r = self.pads[i].r;
        let c = vec2(c.x.clamp(r, (self.w - r).max(r)), c.y.clamp(r, (self.h - r).max(r)));
        let b = self.base_c(i);
        cfg[i].dx = (c.x - b.x) / self.s;
        cfg[i].dy = (c.y - b.y) / self.s;
    }

    fn settings_hit(&self, p: Vec2) -> bool {
        p.distance(self.settings.c) <= self.settings.r * 1.1
    }

    // Keep the floating joystick base fully on screen.
    fn clamp_origin(&self, p: Vec2, w: f32, h: f32) -> Vec2 {
        let m = self.pads[0].r + 10.0 * self.u;
        vec2(p.x.clamp(m, (w - m).max(m)), p.y.clamp(m, (h - m).max(m)))
    }
}

fn draw_center(text: &str, cx: f32, cy: f32, size: f32, col: Color) {
    let m = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - m.width / 2.0, cy + m.height / 2.0, size, col);
}

fn draw_pad(p: &Pad, label: &str, active: bool, u: f32) {
    let fill = if active {
        Color::new(0.23, 0.62, 0.78, 0.85 * p.a + 0.1)
    } else {
        Color::new(0.02, 0.03, 0.05, 0.85 * p.a)
    };
    draw_circle(p.c.x, p.c.y, p.r, fill);
    draw_circle_lines(p.c.x, p.c.y, p.r, 2.5 * u, Color::new(0.85, 0.94, 1.0, p.a));
    let size = (p.r * 0.36).max(11.0 * u);
    draw_center(label, p.c.x, p.c.y, size, Color::new(1.0, 1.0, 1.0, (p.a + 0.2).min(1.0)));
}

fn draw_joystick(home: &Pad, origin: Vec2, knob: Vec2, u: f32) {
    draw_circle(origin.x, origin.y, home.r, Color::new(0.02, 0.03, 0.05, 0.55 * home.a));
    draw_circle_lines(origin.x, origin.y, home.r, 2.5 * u, Color::new(0.85, 0.94, 1.0, home.a));
    draw_circle(knob.x, knob.y, home.r * 0.42, Color::new(0.8, 0.92, 1.0, 0.85 * home.a));
}

fn ui_button(r: Rect, label: &str, active: bool, u: f32) {
    let fill = if active {
        Color::new(0.23, 0.62, 0.78, 0.95)
    } else {
        Color::new(0.13, 0.18, 0.25, 0.95)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.8, 0.92, 1.0, 0.8));
    draw_center(label, r.x + r.w / 2.0, r.y + r.h / 2.0, 17.0 * u, WHITE);
}

fn win_size(screen: Screen, u: f32) -> Vec2 {
    match screen {
        Screen::Settings => vec2(430.0 * u, 222.0 * u),
        Screen::Edit => vec2(430.0 * u, 330.0 * u),
        Screen::Play => Vec2::ZERO,
    }
}

// position (clamped to the screen) and size of the floating window
fn win_geom(screen: Screen, win_pos: Option<Vec2>, w: f32, h: f32, u: f32) -> (Vec2, Vec2) {
    let ws = win_size(screen, u);
    let d = win_pos.unwrap_or(vec2((w - 430.0 * u) * 0.5, 14.0 * u));
    let wp = vec2(
        d.x.clamp(0.0, (w - ws.x).max(0.0)),
        d.y.clamp(0.0, (h - ws.y).max(0.0)),
    );
    (wp, ws)
}

// ---------------------------------------------------------------------------
// Drawing the city
// ---------------------------------------------------------------------------
fn flat(b: &mut Batch, x: f32, z: f32, w: f32, d: f32, y: f32, col: Color) {
    b.quad([vec3(x, y, z + d), vec3(x + w, y, z + d), vec3(x + w, y, z), vec3(x, y, z)], col);
}

fn tcube(b: &mut Batch, tex: usize, c: Vec3, hx: f32, hy: f32, hz: f32) {
    let (x0, x1, y0, y1, z0, z1) = (c.x - hx, c.x + hx, c.y - hy, c.y + hy, c.z - hz, c.z + hz);
    let uv = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
    let sh = |k: f32| Color::new(k, k, k, 1.0);
    b.tquad(tex, [vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x0, y1, z0)], uv, sh(0.72));
    b.tquad(tex, [vec3(x1, y0, z1), vec3(x0, y0, z1), vec3(x0, y1, z1), vec3(x1, y1, z1)], uv, sh(0.84));
    b.tquad(tex, [vec3(x0, y0, z1), vec3(x0, y0, z0), vec3(x0, y1, z0), vec3(x0, y1, z1)], uv, sh(0.72));
    b.tquad(tex, [vec3(x1, y0, z0), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x1, y1, z0)], uv, sh(0.88));
    b.tquad(tex, [vec3(x0, y1, z0), vec3(x1, y1, z0), vec3(x1, y1, z1), vec3(x0, y1, z1)], uv, sh(1.0));
}

fn draw_building(b: &mut Batch, x0: f32, z0: f32, x1: f32, z1: f32, h: f32, style: u8, cam: Vec3) {
    let tex = T_FAC0 + style as usize;
    let light = vec3(0.39, 0.87, 0.29);
    let up = vec3(0.0, 1.0, 0.0);
    // (visible from camera, start corner, direction along the wall, length, outward normal)
    let faces = [
        (cam.z < z0, vec3(x0, 0.0, z0), vec3(1.0, 0.0, 0.0), x1 - x0, vec3(0.0, 0.0, -1.0)),
        (cam.z > z1, vec3(x1, 0.0, z1), vec3(-1.0, 0.0, 0.0), x1 - x0, vec3(0.0, 0.0, 1.0)),
        (cam.x < x0, vec3(x0, 0.0, z1), vec3(0.0, 0.0, -1.0), z1 - z0, vec3(-1.0, 0.0, 0.0)),
        (cam.x > x1, vec3(x1, 0.0, z0), vec3(0.0, 0.0, 1.0), z1 - z0, vec3(1.0, 0.0, 0.0)),
    ];
    for (vis, o, dir, len, n) in faces {
        if !vis {
            continue;
        }
        let shade = 0.72 + 0.4 * n.dot(light).max(0.0);
        let col = Color::new(shade, shade, shade, 1.0);
        let mut pos = 0.0f32;
        while pos < len {
            let w = (len - pos).min(8.0);
            let a = o + dir * pos;
            let e = o + dir * (pos + w);
            let mid = (a + e) * 0.5;
            let dd = vec2(mid.x - cam.x, mid.z - cam.z).length();
            if dd < b.fe + 8.0 {
                let uw = w / 8.0;
                let mut y = 0.0f32;
                while y < h {
                    let hh = (h - y).min(8.0);
                    let vt = 1.0 - hh / 8.0;
                    b.tquad(
                        tex,
                        [a + up * y, e + up * y, e + up * (y + hh), a + up * (y + hh)],
                        [(0.0, 1.0), (uw, 1.0), (uw, vt), (0.0, vt)],
                        col,
                    );
                    y += 8.0;
                }
            }
            pos += 8.0;
        }
    }
}

fn draw_tree(b: &mut Batch, x: f32, z: f32, s: f32) {
    let t = hash2((x * 10.0) as i32, (z * 10.0) as i32, 77);
    let g = |k: f32| Color::new(0.10 + 0.06 * t, 0.26 + 0.12 * t * k, 0.08 + 0.04 * t, 1.0);
    b.obox(vec3(x, 0.9 * s, z), 0.0, 0.12, 0.9 * s, 0.12, Color::new(0.25, 0.17, 0.10, 1.0));
    b.obox(vec3(x, 2.0 * s, z), 0.3, 0.85 * s, 0.6 * s, 0.85 * s, g(0.8));
    b.obox(vec3(x, 2.8 * s, z), 0.9, 0.65 * s, 0.5 * s, 0.65 * s, g(1.0));
    b.obox(vec3(x, 3.4 * s, z), 0.5, 0.38 * s, 0.4 * s, 0.38 * s, g(1.2));
}

fn draw_car(b: &mut Batch, x: f32, z: f32, along_z: bool, col: u8) {
    const COLS: [(f32, f32, f32); 8] = [
        (0.65, 0.08, 0.08),
        (0.10, 0.20, 0.50),
        (0.80, 0.80, 0.82),
        (0.08, 0.08, 0.09),
        (0.45, 0.47, 0.50),
        (0.12, 0.38, 0.20),
        (0.75, 0.60, 0.15),
        (0.55, 0.30, 0.12),
    ];
    let (r, g, bl) = COLS[(col % 8) as usize];
    let body = Color::new(r, g, bl, 1.0);
    let yaw = if along_z { std::f32::consts::FRAC_PI_2 } else { 0.0 };
    let f = vec3(yaw.cos(), 0.0, yaw.sin());
    let rt = vec3(-yaw.sin(), 0.0, yaw.cos());
    let c = vec3(x, 0.0, z);
    let up = vec3(0.0, 1.0, 0.0);
    let glass = Color::new(0.10, 0.14, 0.18, 1.0);
    let tire = Color::new(0.05, 0.05, 0.06, 1.0);
    b.obox(c + up * 0.30, yaw, 2.15, 0.12, 0.88, Color::new(0.07, 0.07, 0.08, 1.0));
    b.obox(c + up * 0.58, yaw, 2.2, 0.26, 0.9, body);
    b.obox(c + up * 0.98 - f * 0.1, yaw, 1.1, 0.26, 0.80, glass);
    b.obox(c + up * 1.23 - f * 0.1, yaw, 1.0, 0.04, 0.78, body);
    for sx in [-1.3f32, 1.3] {
        for sz in [-0.88f32, 0.88] {
            b.obox(c + f * sx + rt * sz + up * 0.3, yaw, 0.32, 0.3, 0.1, tire);
        }
    }
    for sz in [-0.65f32, 0.65] {
        b.obox(c - f * 2.2 + rt * sz + up * 0.65, yaw, 0.02, 0.06, 0.16, Color::new(0.75, 0.05, 0.05, 1.0));
        b.obox(c + f * 2.2 + rt * sz + up * 0.65, yaw, 0.02, 0.06, 0.16, Color::new(0.95, 0.92, 0.75, 1.0));
    }
}

fn draw_city(b: &mut Batch, cam: Vec3) {
    let c = city();

    // ---- ground tiles (4 m) ----
    b.fs = 45.0;
    b.fe = 95.0;
    let r = 95.0f32;
    let gt = (GW / 4) as f32;
    let tx0 = ((cam.x - r) / 4.0).floor().max(0.0) as usize;
    let tx1 = ((cam.x + r) / 4.0).ceil().min(gt) as usize;
    let tz0 = ((cam.z - r) / 4.0).floor().max(0.0) as usize;
    let tz1 = ((cam.z + r) / 4.0).ceil().min(gt) as usize;
    for tz in tz0..tz1 {
        for tx in tx0..tx1 {
            let x = (tx * 4) as f32;
            let z = (tz * 4) as f32;
            let (dx, dz) = (x + 2.0 - cam.x, z + 2.0 - cam.z);
            if dx * dx + dz * dz > (r + 3.0) * (r + 3.0) {
                continue;
            }
            let tex = match c.kind[tz * 4 * GW + tx * 4] {
                0 => T_ASPHALT,
                1 => T_SIDEWALK,
                2 => T_CONCRETE,
                _ => T_GRASS,
            };
            b.tquad(
                tex,
                [vec3(x, 0.0, z + 4.0), vec3(x + 4.0, 0.0, z + 4.0), vec3(x + 4.0, 0.0, z), vec3(x, 0.0, z)],
                [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
                WHITE,
            );
        }
    }

    // ---- road markings ----
    let paint = Color::new(0.88, 0.86, 0.80, 1.0);
    let ym = 0.06;
    let mr = 60.0f32;
    for rd in 0..=NB {
        let base = (rd * P) as f32;
        let along = |v: f32| (v - mr).max(0.0).div_euclid(4.0) * 4.0;
        // centre dashes
        if (base + 4.0 - cam.x).abs() < mr {
            let mut z = along(cam.z);
            while z < (cam.z + mr).min(GW as f32) {
                let m = (z as usize) % P;
                if m >= 10 && m <= 29 {
                    flat(b, base + 3.9, z, 0.2, 2.4, ym, paint);
                }
                z += 4.0;
            }
        }
        if (base + 4.0 - cam.z).abs() < mr {
            let mut x = along(cam.x);
            while x < (cam.x + mr).min(GW as f32) {
                let m = (x as usize) % P;
                if m >= 10 && m <= 29 {
                    flat(b, x, base + 3.9, 2.4, 0.2, ym, paint);
                }
                x += 4.0;
            }
        }
        // zebra crossings at both ends of each road segment
        for seg in 0..NB {
            let s0 = (seg * P + 8) as f32;
            for zc in [s0 + 1.0, s0 + 21.0] {
                if (base + 4.0 - cam.x).abs() < mr && (zc - cam.z).abs() < mr {
                    for k in 0..7 {
                        flat(b, base + 0.5 + k as f32, zc, 0.5, 1.5, ym, paint);
                    }
                }
                if (base + 4.0 - cam.z).abs() < mr && (zc - cam.x).abs() < mr {
                    for k in 0..7 {
                        flat(b, zc, base + 0.5 + k as f32, 1.5, 0.5, ym, paint);
                    }
                }
            }
        }
    }

    // ---- buildings and street props ----
    for p in &c.props {
        match *p {
            Prop::Building { x0, z0, x1, z1, h, style } => {
                let dx = (x0 - cam.x).max(0.0).max(cam.x - x1);
                let dz = (z0 - cam.z).max(0.0).max(cam.z - z1);
                if dx * dx + dz * dz < 135.0 * 135.0 {
                    b.fs = 70.0;
                    b.fe = 130.0;
                    draw_building(b, x0, z0, x1, z1, h, style, cam);
                }
            }
            Prop::Tree { x, z, s } => {
                if (x - cam.x).powi(2) + (z - cam.z).powi(2) < 85.0 * 85.0 {
                    b.fs = 50.0;
                    b.fe = 90.0;
                    draw_tree(b, x, z, s);
                }
            }
            Prop::Car { x, z, along_z, col } => {
                if (x - cam.x).powi(2) + (z - cam.z).powi(2) < 85.0 * 85.0 {
                    b.fs = 50.0;
                    b.fe = 90.0;
                    draw_car(b, x, z, along_z, col);
                }
            }
            Prop::Crate { x, z } => {
                if (x - cam.x).powi(2) + (z - cam.z).powi(2) < 85.0 * 85.0 {
                    b.fs = 50.0;
                    b.fe = 90.0;
                    tcube(b, T_CRATE, vec3(x, CRATE_H * 0.5, z), 0.44, CRATE_H * 0.5, 0.44);
                }
            }
            Prop::Lamp { x, z } => {
                if (x - cam.x).powi(2) + (z - cam.z).powi(2) < 80.0 * 80.0 {
                    b.fs = 50.0;
                    b.fe = 90.0;
                    b.aabb(vec3(x, 2.6, z), vec3(0.05, 2.6, 0.05), Color::new(0.22, 0.24, 0.26, 1.0));
                    b.aabb(vec3(x, 5.25, z), vec3(0.22, 0.08, 0.22), Color::new(0.92, 0.88, 0.70, 1.0));
                }
            }
        }
    }
}

fn draw_target(b: &mut Batch, tg: &Target, px: f32, pz: f32) {
    let yaw = (pz - tg.z).atan2(px - tg.x); // face the player
    let p = vec3(tg.x, 0.0, tg.z);
    let up = vec3(0.0, 1.0, 0.0);
    let f = vec3(yaw.cos(), 0.0, yaw.sin());
    let r = vec3(-yaw.sin(), 0.0, yaw.cos());
    let red = Color::new(0.72, 0.18, 0.15, 1.0);
    let dark_red = Color::new(0.45, 0.10, 0.09, 1.0);
    let white = Color::new(0.95, 0.95, 0.92, 1.0);
    let metal = Color::new(0.35, 0.36, 0.39, 1.0);
    let skin = Color::new(0.88, 0.62, 0.38, 1.0);

    // base plate + legs
    b.obox(p + up * 0.02, yaw, 0.20, 0.02, 0.20, Color::new(0.22, 0.23, 0.25, 1.0));
    b.obox(p + r * 0.07 + up * 0.19, yaw, 0.055, 0.15, 0.055, metal);
    b.obox(p - r * 0.07 + up * 0.19, yaw, 0.055, 0.15, 0.055, metal);
    // body
    b.obox(p + up * 0.52, yaw, 0.11, 0.20, 0.20, red);
    b.obox(p + up * 0.34, yaw, 0.115, 0.025, 0.205, Color::new(0.10, 0.10, 0.12, 1.0)); // belt
    // bullseye on the chest
    let c = p + up * 0.54;
    b.obox(c + f * 0.112, yaw, 0.004, 0.14, 0.14, white);
    b.obox(c + f * 0.116, yaw, 0.004, 0.10, 0.10, dark_red);
    b.obox(c + f * 0.120, yaw, 0.004, 0.06, 0.06, white);
    b.obox(c + f * 0.124, yaw, 0.004, 0.025, 0.025, dark_red);
    // arms
    b.obox(p + r * 0.25 + up * 0.50, yaw, 0.05, 0.16, 0.05, dark_red);
    b.obox(p - r * 0.25 + up * 0.50, yaw, 0.05, 0.16, 0.05, dark_red);
    // head, helmet, visor
    b.obox(p + up * 0.83, yaw, 0.085, 0.085, 0.085, skin);
    b.obox(p + up * 0.90, yaw, 0.10, 0.045, 0.10, dark_red);
    b.obox(p + up * 0.83 + f * 0.088, yaw, 0.01, 0.025, 0.07, Color::new(0.08, 0.09, 0.12, 1.0));
}

fn draw_player(b: &mut Batch, g: &Game) {
    let k = g.k();
    let yaw = g.yaw;
    let base = vec3(g.x, g.y, g.z);
    let up = vec3(0.0, 1.0, 0.0);
    let f = vec3(yaw.cos(), 0.0, yaw.sin());
    let r = vec3(-yaw.sin(), 0.0, yaw.cos());
    let d = g.aim_dir();
    let u2 = r.cross(d);

    let boot = Color::new(0.09, 0.09, 0.11, 1.0);
    let pants = Color::new(0.17, 0.20, 0.26, 1.0);
    let belt = Color::new(0.08, 0.08, 0.09, 1.0);
    let shirt = Color::new(0.22, 0.55, 0.48, 1.0);
    let vest = Color::new(0.15, 0.26, 0.25, 1.0);
    let patch = Color::new(0.80, 0.68, 0.30, 1.0);
    let pack = Color::new(0.55, 0.45, 0.30, 1.0);
    let skin = Color::new(0.92, 0.72, 0.56, 1.0);
    let helmet = Color::new(0.26, 0.36, 0.30, 1.0);
    let visor = Color::new(0.10, 0.26, 0.36, 1.0);
    let glove = Color::new(0.10, 0.10, 0.12, 1.0);
    let gun_dark = Color::new(0.12, 0.13, 0.15, 1.0);
    let gun_mid = Color::new(0.30, 0.32, 0.35, 1.0);
    let gun_wood = Color::new(0.35, 0.24, 0.14, 1.0);

    // soft shadow so jumps read clearly
    let sy = g.ground(g.y) + 0.012;
    b.quad(
        [
            vec3(g.x - 0.3, sy, g.z + 0.3),
            vec3(g.x + 0.3, sy, g.z + 0.3),
            vec3(g.x + 0.3, sy, g.z - 0.3),
            vec3(g.x - 0.3, sy, g.z - 0.3),
        ],
        Color::new(0.0, 0.0, 0.0, 0.35),
    );

    // legs + boots (swing while walking)
    let sw = (g.walk * 2.0).sin() * 0.14 * g.moving.min(1.0);
    let leg_top = 0.40 * k;
    let ch = (leg_top - 0.10) * 0.5;
    for s in [-1.0f32, 1.0] {
        let off = r * (0.095 * s) + f * (sw * s);
        b.obox(base + off + up * 0.05 + f * 0.03, yaw, 0.10, 0.05, 0.07, boot);
        b.obox(base + off + up * (0.10 + ch), yaw, 0.065, ch, 0.07, pants);
    }

    // belt + pouches
    b.obox(base + up * leg_top, yaw, 0.115, 0.025, 0.20, belt);
    b.obox(base + up * leg_top + r * 0.19 + f * 0.03, yaw, 0.04, 0.035, 0.04, pack);
    b.obox(base + up * leg_top - r * 0.19 + f * 0.03, yaw, 0.04, 0.035, 0.04, pack);

    // torso, plate carrier, backpack, chest patch
    b.obox(base + up * (0.59 * k), yaw, 0.10, 0.19 * k, 0.19, shirt);
    b.obox(base + up * (0.60 * k) + f * 0.05, yaw, 0.06, 0.15 * k, 0.17, vest);
    b.obox(base + up * (0.60 * k) - f * 0.07, yaw, 0.04, 0.15 * k, 0.16, vest);
    b.obox(base + up * (0.66 * k) + f * 0.112, yaw, 0.012, 0.035 * k, 0.07, patch);
    b.obox(base + up * (0.62 * k) - f * 0.17, yaw, 0.06, 0.14 * k, 0.13, pack);

    // neck, head, helmet, visor
    b.obox(base + up * (0.80 * k), yaw, 0.04, 0.025, 0.04, skin);
    let hy = 0.93 * k;
    b.obox(base + up * hy, yaw, 0.085, 0.09, 0.085, skin);
    b.obox(base + up * (hy + 0.05), yaw, 0.095, 0.05, 0.095, helmet);
    b.obox(base + up * (hy + 0.14), yaw, 0.02, 0.012, 0.05, Color::new(0.20, 0.28, 0.24, 1.0));
    b.obox(base + up * hy + f * 0.088, yaw, 0.01, 0.03, 0.075, visor);

    // gun
    let gc = g.gun_center();
    b.boxed(gc, d * 0.17, u2 * 0.035, r * 0.03, gun_dark); // receiver
    b.boxed(gc + d * 0.17, d * 0.07, u2 * 0.028, r * 0.026, gun_mid); // handguard
    b.boxed(gc + d * 0.31, d * 0.10, u2 * 0.013, r * 0.013, gun_dark); // barrel
    b.boxed(gc - d * 0.22, d * 0.08, u2 * 0.035, r * 0.025, gun_wood); // stock
    b.boxed(gc + d * 0.03 - u2 * 0.075, d * 0.02, u2 * 0.05, r * 0.02, gun_dark); // magazine
    b.boxed(gc + u2 * 0.05 + d * 0.04, d * 0.05, u2 * 0.015, r * 0.012, gun_dark); // sight

    // arms reach the gun
    let sh_r = base + up * (0.74 * k) + r * 0.24;
    let sh_l = base + up * (0.74 * k) - r * 0.24;
    let hand_r = gc - d * 0.05 - u2 * 0.045;
    let hand_l = gc + d * 0.17 - u2 * 0.04;
    for (sh, hand) in [(sh_r, hand_r), (sh_l, hand_l)] {
        let mid = sh + (hand - sh) * 0.62;
        b.limb(sh, mid, 0.05, shirt);
        b.limb(mid, hand, 0.04, glove);
        b.aabb(sh, vec3(0.06, 0.05, 0.06), helmet); // shoulder pad
    }

    if g.tracer_t > 0.04 {
        b.aabb(gc + d * 0.50, vec3(0.06, 0.06, 0.06), Color::new(1.0, 0.85, 0.3, 1.0));
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------
#[macroquad::main("Marikina FPS")]
async fn main() {
    clear_background(Color::new(0.08, 0.10, 0.14, 1.0));
    draw_text("Building city...", 30.0, screen_height() * 0.5, 40.0, WHITE);
    next_frame().await;
    let texs = make_textures();
    let _ = city();

    let mut g = Game::new();
    let mut roles: HashMap<u64, Role> = HashMap::new();
    let mut last: HashMap<u64, Vec2> = HashMap::new();
    let mut joy_origin: Option<Vec2> = None;
    let mut touch_seen = false;

    let mut screen = Screen::Play;
    let mut slide_hold = false; // false = tap to slide, true = hold to slide
    let mut cfg = [ElemCfg::new(); 6];
    let mut sel: usize = 3;
    let mut win_pos: Option<Vec2> = None;
    let mut win_off = Vec2::ZERO;
    let mut drag_off: HashMap<u64, Vec2> = HashMap::new();
    let mut mouse_drag: Option<(usize, Vec2)> = None;

    loop {
        let dt = get_frame_time().min(0.05);
        let w = screen_width();
        let h = screen_height();
        let ui = Layout::new(w, h, &cfg);
        let u = ui.u;
        let joy_home = ui.pads[0].c;
        let joy_r = ui.pads[0].r;

        let touch_list = touches();
        if !touch_list.is_empty() {
            touch_seen = true;
        }
        let mobile = cfg!(target_os = "android") || touch_seen;

        // forget fingers that are gone
        roles.retain(|id, _| touch_list.iter().any(|t| t.id == *id));
        last.retain(|id, _| touch_list.iter().any(|t| t.id == *id));
        if !roles.values().any(|r| *r == Role::Joy) {
            joy_origin = None;
        }

        let (wp, ws) = win_geom(screen, win_pos, w, h, u);
        let title = Rect::new(wp.x, wp.y, ws.x, 36.0 * u);

        let mut mv = Vec2::ZERO; // x = strafe right, y = forward
        let mut look = Vec2::ZERO; // screen-style delta (down = positive)
        let mut joy_mag = 0.0f32;
        let mut fire = false;
        let mut jump = false;
        let mut reload = false;
        let mut scope_tap = false;
        let mut snk_tap = false;
        let mut snk_held = false;
        let mut open_settings = false;
        let mut joy_knob: Option<Vec2> = None;
        let mut taps: Vec<Vec2> = Vec::new();

        for t in &touch_list {
            let p = t.position;

            // menus: only taps + dragging the floating window
            if screen != Screen::Play {
                match t.phase {
                    TouchPhase::Started => {
                        taps.push(p);
                        if title.contains(p) {
                            roles.insert(t.id, Role::Win);
                            win_off = p - wp;
                        } else if screen == Screen::Edit && !Rect::new(wp.x, wp.y, ws.x, ws.y).contains(p) {
                            // grab a button / the joystick to drag it anywhere
                            if let Some(e) = ui.hit_any(p) {
                                sel = e as usize;
                                roles.insert(t.id, Role::Drag);
                                drag_off.insert(t.id, ui.pads[sel].c - p);
                            }
                        }
                    }
                    TouchPhase::Moved => {
                        if roles.get(&t.id) == Some(&Role::Win) {
                            win_pos = Some(p - win_off);
                        } else if roles.get(&t.id) == Some(&Role::Drag) {
                            if let Some(off) = drag_off.get(&t.id) {
                                ui.place(sel, p + *off, &mut cfg);
                            }
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        roles.remove(&t.id);
                    }
                    _ => {}
                }
                continue;
            }

            match t.phase {
                TouchPhase::Started => {
                    let role = if ui.settings_hit(p) {
                        open_settings = true;
                        Role::Btn
                    } else if let Some(e) = ui.hit(p) {
                        match e {
                            Elem::Jump => jump = true,
                            Elem::Reload => reload = true,
                            Elem::Scope => scope_tap = true,
                            Elem::Snk => snk_tap = true,
                            _ => {}
                        }
                        Role::Btn
                    } else if p.x < w * 0.42 && p.y > h * 0.25 && !roles.values().any(|r| *r == Role::Joy) {
                        joy_origin = Some(ui.clamp_origin(p, w, h));
                        Role::Joy
                    } else {
                        Role::Look
                    };
                    roles.insert(t.id, role);
                    last.insert(t.id, p);
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if roles.remove(&t.id) == Some(Role::Joy) {
                        joy_origin = None;
                    }
                    last.remove(&t.id);
                    continue;
                }
                _ => {}
            }

            match roles.get(&t.id).copied().unwrap_or(Role::Look) {
                Role::Joy => {
                    let origin = joy_origin.unwrap_or(joy_home);
                    let mut v = (p - origin) / joy_r;
                    if v.length() > 1.0 {
                        v = v.normalize();
                    }
                    let dz = if v.length() < 0.1 { Vec2::ZERO } else { v };
                    mv = vec2(dz.x, -dz.y);
                    joy_mag = dz.length();
                    joy_knob = Some(origin + v * joy_r);
                }
                Role::Look => {
                    if let Some(prev) = last.get(&t.id) {
                        look += p - *prev;
                    }
                    last.insert(t.id, p);
                }
                Role::Btn => match ui.hit(p) {
                    Some(Elem::Fire) => fire = true,
                    Some(Elem::Snk) => snk_held = true,
                    _ => {}
                },
                Role::Win | Role::Drag => {}
            }
        }

        // Mouse clicks (desktop): settings button / menu taps
        if !mobile && is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let mp = vec2(mx, my);
            if screen == Screen::Play {
                if ui.settings_hit(mp) {
                    open_settings = true;
                }
            } else {
                taps.push(mp);
            }
        }

        // Mouse drag of HUD elements (desktop editor)
        if !mobile && screen == Screen::Edit {
            let (mx, my) = mouse_position();
            let mp = vec2(mx, my);
            if is_mouse_button_pressed(MouseButton::Left) && !Rect::new(wp.x, wp.y, ws.x, ws.y).contains(mp) {
                if let Some(e) = ui.hit_any(mp) {
                    sel = e as usize;
                    mouse_drag = Some((sel, ui.pads[sel].c - mp));
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                if let Some((i, off)) = mouse_drag {
                    ui.place(i, mp + off, &mut cfg);
                }
            } else {
                mouse_drag = None;
            }
        }

        // Keyboard + mouse (desktop)
        let mut kb_sprint = false;
        if screen == Screen::Play {
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                mv.y += 1.0;
            }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                mv.y -= 1.0;
            }
            if is_key_down(KeyCode::D) {
                mv.x += 1.0;
            }
            if is_key_down(KeyCode::A) {
                mv.x -= 1.0;
            }
            if is_key_pressed(KeyCode::Space) {
                jump = true;
            }
            if is_key_pressed(KeyCode::R) {
                reload = true;
            }
            if is_key_pressed(KeyCode::F) {
                scope_tap = true;
            }
            if is_key_pressed(KeyCode::C) {
                snk_tap = true;
            }
            if is_key_down(KeyCode::C) {
                snk_held = true;
            }
            kb_sprint = is_key_down(KeyCode::LeftShift);
            if !mobile {
                if is_mouse_button_down(MouseButton::Left) {
                    fire = true;
                }
                if is_mouse_button_down(MouseButton::Right) {
                    let d = mouse_delta_position();
                    look += vec2(-d.x * w * 0.5, d.y * h * 0.5);
                }
            }
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
        }

        // nothing reaches the game while a menu is open
        if screen != Screen::Play || open_settings {
            mv = Vec2::ZERO;
            look = Vec2::ZERO;
            joy_mag = 0.0;
            fire = false;
            jump = false;
            reload = false;
            scope_tap = false;
            snk_tap = false;
            snk_held = false;
            kb_sprint = false;
        }

        // ------------------------------------------------ menus
        let mut new_screen = screen;
        if open_settings {
            new_screen = Screen::Settings;
        }
        {
            let tapped = |r: Rect| taps.iter().any(|p| r.contains(*p));
            // button rect on row i, x offset and width in window units
            let btn = |i: usize, x: f32, bw: f32| {
                Rect::new(wp.x + x * u, wp.y + (48.0 + 56.0 * i as f32) * u, bw * u, 46.0 * u)
            };
            match screen {
                Screen::Settings => {
                    if tapped(btn(0, 150.0, 120.0)) {
                        slide_hold = false;
                    }
                    if tapped(btn(0, 282.0, 120.0)) {
                        slide_hold = true;
                    }
                    if tapped(btn(1, 12.0, 406.0)) {
                        new_screen = Screen::Edit;
                    }
                    if tapped(btn(2, 12.0, 406.0)) {
                        new_screen = Screen::Play;
                    }
                }
                Screen::Edit => {
                    let win_rect = Rect::new(wp.x, wp.y, ws.x, ws.y);
                    for p in &taps {
                        if !win_rect.contains(*p) {
                            if let Some(e) = ui.hit_any(*p) {
                                sel = e as usize;
                            }
                        }
                    }
                    if tapped(btn(0, 12.0, 56.0)) {
                        sel = (sel + 5) % 6;
                    }
                    if tapped(btn(0, 362.0, 56.0)) {
                        sel = (sel + 1) % 6;
                    }
                    if tapped(btn(1, 150.0, 56.0)) {
                        cfg[sel].scale = (((cfg[sel].scale - 0.1) * 10.0).round() / 10.0).max(0.5);
                    }
                    if tapped(btn(1, 286.0, 56.0)) {
                        cfg[sel].scale = (((cfg[sel].scale + 0.1) * 10.0).round() / 10.0).min(2.0);
                    }
                    if tapped(btn(2, 150.0, 56.0)) {
                        cfg[sel].opacity = (((cfg[sel].opacity - 0.1) * 10.0).round() / 10.0).max(0.2);
                    }
                    if tapped(btn(2, 286.0, 56.0)) {
                        cfg[sel].opacity = (((cfg[sel].opacity + 0.1) * 10.0).round() / 10.0).min(1.0);
                    }
                    if tapped(btn(3, 150.0, 56.0)) {
                        cfg[sel].dx = (cfg[sel].dx - NUDGE).max(-3000.0);
                    }
                    if tapped(btn(3, 214.0, 56.0)) {
                        cfg[sel].dy = (cfg[sel].dy - NUDGE).max(-3000.0);
                    }
                    if tapped(btn(3, 278.0, 56.0)) {
                        cfg[sel].dy = (cfg[sel].dy + NUDGE).min(3000.0);
                    }
                    if tapped(btn(3, 342.0, 56.0)) {
                        cfg[sel].dx = (cfg[sel].dx + NUDGE).min(3000.0);
                    }
                    if tapped(btn(4, 12.0, 190.0)) {
                        cfg[sel] = ElemCfg::new();
                    }
                    if tapped(btn(4, 228.0, 190.0)) {
                        new_screen = Screen::Settings;
                    }
                }
                Screen::Play => {}
            }
        }
        if new_screen != screen {
            screen = new_screen;
            roles.clear();
            last.clear();
            joy_origin = None;
        }

        // ------------------------------------------------ actions
        if scope_tap {
            g.scoped = !g.scoped;
        }
        g.sneaking = g.sneak_on || (screen == Screen::Play && is_key_down(KeyCode::LeftControl));
        g.sprint = (joy_mag >= SPRINT_TILT || kb_sprint)
            && mv.length() > 0.1
            && !g.sneaking
            && !g.scoped
            && !g.sliding;

        // one button: Slide while running, Snk otherwise
        let slide_label = g.sprint || g.sliding;
        if snk_tap {
            if slide_label {
                if g.grounded && !g.sliding && g.slide_cd <= 0.0 {
                    let sf = vec2(g.yaw.cos(), g.yaw.sin());
                    let sr = vec2(-g.yaw.sin(), g.yaw.cos());
                    let sv = sf * mv.y + sr * mv.x;
                    g.slide_dir = if sv.length() > 0.1 { sv.normalize() } else { sf };
                    g.sliding = true;
                    g.slide_t = 0.0;
                    g.slide_dur = if slide_hold { 1.4 } else { 0.6 };
                }
            } else {
                g.sneak_on = !g.sneak_on;
                g.sneaking = g.sneak_on;
            }
        }
        if g.sliding {
            g.slide_t += dt;
            let released = slide_hold && !snk_held && g.slide_t > 0.2;
            if g.slide_t >= g.slide_dur || released || jump {
                g.sliding = false;
                g.slide_cd = 0.4;
            }
        }
        g.slide_cd = (g.slide_cd - dt).max(0.0);

        if reload {
            g.start_reload();
        }
        if jump && g.grounded {
            g.vy = JUMP_V;
            g.grounded = false;
        }
        if fire {
            g.shoot();
        }

        // Look (drag up = look up)
        let sens = if g.zoom > 0.5 { 0.5 } else { 1.0 };
        g.yaw = (g.yaw + look.x / u * 0.0050 * sens).rem_euclid(TAU);
        g.pitch = (g.pitch - look.y / u * 0.0035 * sens).clamp(-0.55, 0.5);

        // Move relative to where the camera faces
        if mv.length() > 1.0 {
            mv = mv.normalize();
        }
        let f = vec2(g.yaw.cos(), g.yaw.sin());
        let r = vec2(-g.yaw.sin(), g.yaw.cos());
        let wish = f * mv.y + r * mv.x;
        let (dir, speed) = if g.sliding {
            (g.slide_dir, 2.0 + 6.0 * (1.0 - (g.slide_t / g.slide_dur).min(1.0)))
        } else if g.sneaking {
            (wish, SNEAK_SPEED)
        } else if g.sprint {
            (wish.normalize_or_zero(), SPRINT_SPEED)
        } else {
            (wish, WALK_SPEED)
        };
        g.move_by(dir.x * speed * dt, dir.y * speed * dt);
        g.moving = if g.sliding { 0.0 } else { mv.length() };
        g.walk += g.moving * speed * dt * 3.0;

        // Vertical physics (jump, crates)
        let prev_y = g.y;
        g.vy -= GRAVITY * dt;
        g.y += g.vy * dt;
        let floor = g.ground(prev_y);
        if g.y <= floor {
            g.y = floor;
            g.vy = 0.0;
        }
        g.grounded = g.y <= floor + 0.001;

        // Timers
        g.fire_cd = (g.fire_cd - dt).max(0.0);
        g.tracer_t = (g.tracer_t - dt).max(0.0);
        g.hit_t = (g.hit_t - dt).max(0.0);
        if g.reloading > 0.0 {
            g.reloading -= dt;
            if g.reloading <= 0.0 {
                let n = (MAG - g.ammo).min(g.reserve);
                g.ammo += n;
                g.reserve -= n;
                g.reloading = 0.0;
            }
        }
        let zoom_goal = if g.scoped { 1.0 } else { 0.0 };
        g.zoom += (zoom_goal - g.zoom) * (dt * 12.0).min(1.0);
        let boost_goal = if g.sprint || g.sliding { 1.0 } else { 0.0 };
        g.boost += (boost_goal - g.boost) * (dt * 8.0).min(1.0);
        for t in &mut g.targets {
            t.phase += dt;
            if t.alive {
                t.x = t.x0 + (t.phase * 0.9).sin() * 0.9;
            } else {
                t.respawn -= dt;
                if t.respawn <= 0.0 {
                    t.alive = true;
                }
            }
        }

        // ------------------------------------------------------------ render
        clear_background(Color::new(0.76, 0.82, 0.88, 1.0));
        {
            let hz = (h * 0.5 + g.pitch * h * 0.9).clamp(h * 0.1, h);
            let top = [0.28f32, 0.46, 0.74];
            let haze = [0.76f32, 0.82, 0.88];
            let n = 24;
            for i in 0..n {
                let t0 = i as f32 / n as f32;
                let y0 = hz * t0;
                let y1 = hz * (i + 1) as f32 / n as f32;
                let k = t0.powf(1.6);
                let c = Color::new(
                    top[0] + (haze[0] - top[0]) * k,
                    top[1] + (haze[1] - top[1]) * k,
                    top[2] + (haze[2] - top[2]) * k,
                    1.0,
                );
                draw_rectangle(0.0, y0, w, y1 - y0 + 1.0, c);
            }
        }

        let cam = g.cam_pos();
        let d = g.aim_dir();
        set_camera(&Camera3D {
            position: cam,
            target: cam + d * 10.0,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 1.05 - 0.6 * g.zoom + 0.08 * g.boost,
            ..Default::default()
        });

        let mut b = Batch::new(texs.clone(), cam);
        draw_city(&mut b, cam);
        b.fs = 50.0;
        b.fe = 95.0;
        for tg in &g.targets {
            if tg.alive && (tg.x - g.x).abs() < 95.0 && (tg.z - g.z).abs() < 95.0 {
                draw_target(&mut b, tg, g.x, g.z);
            }
        }
        b.fs = 1.0e6;
        b.fe = 2.0e6;
        draw_player(&mut b, &g);
        if let Some((_, end)) = g.tracer {
            if g.tracer_t > 0.0 {
                b.aabb(end, vec3(0.05, 0.05, 0.05), Color::new(1.0, 0.8, 0.3, 1.0));
            }
        }
        b.flush();
        if let Some((from, to)) = g.tracer {
            if g.tracer_t > 0.0 {
                draw_line_3d(from, to, Color::new(1.0, 0.9, 0.4, 1.0));
            }
        }

        // ------------------------------------------------------------ HUD
        set_default_camera();
        let cx = w * 0.5;
        let cy = h * 0.5;

        if g.zoom > 0.4 {
            let a = ((g.zoom - 0.4) / 0.6).clamp(0.0, 1.0);
            let s = h.min(w) * 0.92;
            let x0 = (w - s) / 2.0;
            let y0 = (h - s) / 2.0;
            let dark = Color::new(0.0, 0.0, 0.0, 0.82 * a);
            draw_rectangle(0.0, 0.0, x0, h, dark);
            draw_rectangle(x0 + s, 0.0, w - x0 - s, h, dark);
            draw_rectangle(x0, 0.0, s, y0, dark);
            draw_rectangle(x0, y0 + s, s, h - y0 - s, dark);
            let line = Color::new(0.75, 0.82, 0.83, 0.7 * a);
            draw_circle_lines(cx, cy, s * 0.5, 3.0, Color::new(0.55, 0.66, 0.7, a));
            draw_line(cx - s * 0.5, cy, cx + s * 0.5, cy, 1.0, line);
            draw_line(cx, cy - s * 0.5, cx, cy + s * 0.5, 1.0, line);
        } else {
            let gap = 5.0 * u;
            let len = 11.0 * u;
            draw_line(cx - gap - len, cy, cx - gap, cy, 2.0, WHITE);
            draw_line(cx + gap, cy, cx + gap + len, cy, 2.0, WHITE);
            draw_line(cx, cy - gap - len, cx, cy - gap, 2.0, WHITE);
            draw_line(cx, cy + gap, cx, cy + gap + len, 2.0, WHITE);
        }
        if g.hit_t > 0.0 {
            let s = 12.0 * u;
            let c = Color::new(1.0, 0.35, 0.3, 1.0);
            draw_line(cx - s, cy - s, cx - s * 0.4, cy - s * 0.4, 3.0, c);
            draw_line(cx + s, cy - s, cx + s * 0.4, cy - s * 0.4, 3.0, c);
            draw_line(cx - s, cy + s, cx - s * 0.4, cy + s * 0.4, 3.0, c);
            draw_line(cx + s, cy + s, cx + s * 0.4, cy + s * 0.4, 3.0, c);
        }

        let pad = 14.0 * u;
        draw_text("MARIKINA FPS  /  CITY", pad, 27.0 * u, 22.0 * u, WHITE);
        draw_text(
            &format!("HP {}     {} / {}", g.health, g.ammo, g.reserve),
            pad,
            55.0 * u,
            21.0 * u,
            Color::new(0.65, 0.94, 0.79, 1.0),
        );
        draw_text(
            &format!("TARGETS {}     {}", g.kills, g.targets.iter().filter(|t| t.alive).count()),
            pad,
            82.0 * u,
            17.0 * u,
            Color::new(0.82, 0.88, 0.92, 1.0),
        );
        if g.reloading > 0.0 {
            draw_text("RELOADING...", cx - 70.0 * u, cy + h * 0.22, 26.0 * u, WHITE);
        }

        // on-screen controls
        if mobile || screen == Screen::Edit {
            let origin = joy_origin.unwrap_or(joy_home);
            let knob = joy_knob.unwrap_or(origin);
            draw_joystick(&ui.pads[0], origin, knob, u);
            draw_pad(&ui.pads[1], "Scope", g.scoped, u);
            draw_pad(&ui.pads[2], "Jump", !g.grounded, u);
            draw_pad(&ui.pads[3], "Fire", fire, u);
            draw_pad(&ui.pads[4], "Rel", g.reloading > 0.0, u);
            let label = if g.sprint || g.sliding { "Slide" } else { "Snk" };
            let active = g.sliding || (g.sneaking && !g.sprint);
            draw_pad(&ui.pads[5], label, active, u);
        }
        if screen == Screen::Edit {
            let p = &ui.pads[sel];
            draw_circle_lines(p.c.x, p.c.y, p.r + 6.0 * u, 3.0 * u, Color::new(1.0, 0.85, 0.2, 1.0));
        }

        // settings button (octagon, top right)
        {
            let s = &ui.settings;
            draw_poly(s.c.x, s.c.y, 8, s.r, 22.5, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_poly_lines(s.c.x, s.c.y, 8, s.r, 22.5, 2.0 * u, Color::new(0.85, 0.94, 1.0, 0.8));
            draw_center("Settings", s.c.x, s.c.y, 13.0 * u, WHITE);
        }

        // floating window
        if screen != Screen::Play {
            let (wp, ws) = win_geom(screen, win_pos, w, h, u);
            let row_y = |i: usize| wp.y + (48.0 + 56.0 * i as f32) * u;
            let btn = |i: usize, x: f32, bw: f32| Rect::new(wp.x + x * u, row_y(i), bw * u, 46.0 * u);
            let txt = Color::new(0.85, 0.94, 1.0, 1.0);
            draw_rectangle(wp.x, wp.y, ws.x, ws.y, Color::new(0.05, 0.08, 0.12, 0.94));
            draw_rectangle_lines(wp.x, wp.y, ws.x, ws.y, 2.0, Color::new(0.8, 0.92, 1.0, 0.8));
            draw_rectangle(wp.x, wp.y, ws.x, 36.0 * u, Color::new(0.14, 0.22, 0.30, 1.0));
            match screen {
                Screen::Settings => {
                    draw_text("SETTINGS   (drag here to move)", wp.x + 12.0 * u, wp.y + 25.0 * u, 18.0 * u, WHITE);
                    draw_text("Slide", wp.x + 14.0 * u, row_y(0) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(0, 150.0, 120.0), "TAP", !slide_hold, u);
                    ui_button(btn(0, 282.0, 120.0), "HOLD", slide_hold, u);
                    ui_button(btn(1, 12.0, 406.0), "CUSTOMIZE HUD", false, u);
                    ui_button(btn(2, 12.0, 406.0), "CLOSE", false, u);
                }
                Screen::Edit => {
                    draw_text("HUD EDITOR  (drag buttons anywhere)", wp.x + 12.0 * u, wp.y + 25.0 * u, 18.0 * u, WHITE);
                    ui_button(btn(0, 12.0, 56.0), "<", false, u);
                    ui_button(btn(0, 362.0, 56.0), ">", false, u);
                    draw_center(ELEMS[sel].name(), wp.x + 215.0 * u, row_y(0) + 23.0 * u, 22.0 * u, WHITE);

                    draw_text("Size", wp.x + 14.0 * u, row_y(1) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(1, 150.0, 56.0), "-", false, u);
                    ui_button(btn(1, 286.0, 56.0), "+", false, u);
                    let pct = (cfg[sel].scale * 100.0).round() as i32;
                    draw_center(&format!("{}%", pct), wp.x + 246.0 * u, row_y(1) + 23.0 * u, 20.0 * u, WHITE);

                    draw_text("Opacity", wp.x + 14.0 * u, row_y(2) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(2, 150.0, 56.0), "-", false, u);
                    ui_button(btn(2, 286.0, 56.0), "+", false, u);
                    let op = (cfg[sel].opacity * 100.0).round() as i32;
                    draw_center(&format!("{}%", op), wp.x + 246.0 * u, row_y(2) + 23.0 * u, 20.0 * u, WHITE);

                    draw_text("Nudge", wp.x + 14.0 * u, row_y(3) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(3, 150.0, 56.0), "<", false, u);
                    ui_button(btn(3, 214.0, 56.0), "^", false, u);
                    ui_button(btn(3, 278.0, 56.0), "v", false, u);
                    ui_button(btn(3, 342.0, 56.0), ">", false, u);

                    ui_button(btn(4, 12.0, 190.0), "RESET", false, u);
                    ui_button(btn(4, 228.0, 190.0), "DONE", false, u);
                }
                Screen::Play => {}
            }
        }

        next_frame().await
    }
}
