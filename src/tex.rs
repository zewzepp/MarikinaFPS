use crate::gfx::*;
use macroquad::prelude::*;

pub fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ s.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

fn noise(x: i32, y: i32, s: u32) -> f32 {
    hash(x, y, s) - 0.5
}

fn put(px: &mut [u8], tile: usize, x: usize, y: usize, c: [f32; 3]) {
    let ox = (tile % 4) * TILE + x;
    let oy = (tile / 4) * TILE + y;
    let i = (oy * ATLAS + ox) * 4;
    px[i] = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
    px[i + 1] = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
    px[i + 2] = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
    px[i + 3] = 255;
}

fn mulc(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

// Returns (pixels, average colour per tile)
pub fn build_atlas() -> (Vec<u8>, [[f32; 3]; NTILES]) {
    let mut px = vec![255u8; ATLAS * ATLAS * 4];
    let mut sum = [[0.0f32; 3]; NTILES];
    for tile in 0..NTILES {
        for y in 0..TILE {
            for x in 0..TILE {
                let (xi, yi) = (x as i32, y as i32);
                let n = noise(xi, yi, tile as u32);
                let n2 = noise(xi / 4, yi / 4, tile as u32 + 50);
                let fx = x as f32 / TILE as f32;
                let fy = y as f32 / TILE as f32;
                let c: [f32; 3] = match tile {
                    T_ASPHALT => mulc([0.23, 0.24, 0.26], 1.0 + n * 0.35 + n2 * 0.25),
                    T_ASPH_LINE => {
                        let base = mulc([0.23, 0.24, 0.26], 1.0 + n * 0.35 + n2 * 0.25);
                        if (fx - 0.5).abs() < 0.035 && fy > 0.12 && fy < 0.88 {
                            [0.88, 0.80, 0.30]
                        } else {
                            base
                        }
                    }
                    T_ZEBRA => {
                        let base = mulc([0.23, 0.24, 0.26], 1.0 + n * 0.3);
                        if ((fy * 8.0) as i32) % 2 == 0 {
                            mulc([0.88, 0.88, 0.85], 1.0 + n * 0.25)
                        } else {
                            base
                        }
                    }
                    T_SIDEWALK => {
                        let g = (fx * 4.0).fract() < 0.03 || (fy * 4.0).fract() < 0.03;
                        mulc([0.62, 0.61, 0.58], (if g { 0.78 } else { 1.0 }) + n * 0.12 + n2 * 0.1)
                    }
                    T_GRASS => {
                        let k = 1.0 + n * 0.5 + n2 * 0.3;
                        [0.22 * k, 0.40 * k, 0.16 * k]
                    }
                    T_PAVING => {
                        let row = (fy * 8.0) as i32;
                        let off = if row % 2 == 0 { 0.0 } else { 0.5 };
                        let gx = ((fx * 6.0 + off).fract() < 0.05) || (fy * 8.0).fract() < 0.07;
                        let tint = 0.9 + hash((fx * 6.0 + off) as i32, row, 7) * 0.2;
                        mulc([0.55, 0.40, 0.33], (if gx { 0.6 } else { tint }) + n * 0.1)
                    }
                    T_CONCRETE => mulc([0.64, 0.64, 0.62], 1.0 + n * 0.18 + n2 * 0.16),
                    T_FAC_RES | T_FAC_OFFICE | T_FAC_SHOP => {
                        let wall = match tile {
                            T_FAC_RES => [0.86, 0.78, 0.62],
                            T_FAC_OFFICE => [0.58, 0.64, 0.70],
                            _ => [0.80, 0.55, 0.42],
                        };
                        // window grid: 2 columns x 2 rows per tile
                        let wx = (fx * 2.0).fract();
                        let wy = (fy * 2.0).fract();
                        let inw = wx > 0.22 && wx < 0.78 && wy > 0.2 && wy < 0.75;
                        if inw {
                            let lit = hash((fx * 2.0) as i32, (fy * 2.0) as i32, tile as u32) > 0.7;
                            let k = 1.0 - (wy - 0.2) * 0.8;
                            if lit && tile != T_FAC_SHOP {
                                [0.95 * k + 0.05, 0.85 * k, 0.55 * k]
                            } else {
                                [0.12 + 0.18 * k, 0.20 + 0.22 * k, 0.28 + 0.26 * k]
                            }
                        } else if (wx > 0.18 && wx < 0.82 && wy > 0.75 && wy < 0.82)
                            || ((wx > 0.18 && wx < 0.22 || wx > 0.78 && wx < 0.82) && wy > 0.2 && wy < 0.8)
                        {
                            mulc(wall, 0.72)
                        } else {
                            mulc(wall, 1.0 + n * 0.14 + n2 * 0.12)
                        }
                    }
                    T_ROOF => {
                        let ridge = (fy * 10.0).fract();
                        mulc([0.45, 0.46, 0.47], (0.85 + ridge * 0.2) + n * 0.1)
                    }
                    T_BRICK => {
                        let row = (fy * 10.0) as i32;
                        let off = if row % 2 == 0 { 0.0 } else { 0.5 };
                        let mortar = (fy * 10.0).fract() < 0.12 || (fx * 5.0 + off).fract() < 0.08;
                        if mortar {
                            [0.68, 0.66, 0.62]
                        } else {
                            mulc([0.60, 0.28, 0.20], 0.9 + hash((fx * 5.0 + off) as i32, row, 3) * 0.25 + n * 0.1)
                        }
                    }
                    T_WOOD => {
                        let grain = ((fy * 30.0 + n2 * 3.0).sin() * 0.5 + 0.5) * 0.18;
                        let plank = (fx * 4.0).fract() < 0.04;
                        mulc([0.55, 0.38, 0.20], (if plank { 0.55 } else { 0.9 }) + grain + n * 0.06)
                    }
                    T_METAL => {
                        let rib = ((fx * 16.0).sin() * 0.5 + 0.5) * 0.18;
                        mulc([0.55, 0.58, 0.60], 0.82 + rib + n * 0.06)
                    }
                    T_COURT => {
                        let line = (fx - 0.5).abs() < 0.02 || fx < 0.03 || fx > 0.97 || fy < 0.03 || fy > 0.97;
                        if line {
                            [0.95, 0.95, 0.95]
                        } else {
                            mulc([0.25, 0.45, 0.35], 1.0 + n * 0.15)
                        }
                    }
                    _ => [1.0, 1.0, 1.0],
                };
                put(&mut px, tile, x, y, c);
                for k in 0..3 {
                    sum[tile][k] += c[k];
                }
            }
        }
        let cnt = (TILE * TILE) as f32;
        for k in 0..3 {
            sum[tile][k] /= cnt;
        }
    }
    (px, sum)
}

pub fn make_texture() -> (Texture2D, [[f32; 3]; NTILES]) {
    let (px, avg) = build_atlas();
    let img = Image { bytes: px, width: ATLAS as u16, height: ATLAS as u16 };
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    (t, avg)
}
