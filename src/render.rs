use crate::game::*;
use crate::gfx::*;
use crate::ui::draw_center;
use crate::world::*;
use macroquad::prelude::*;

pub fn draw_target(b: &mut Batch, tg: &Target) {
    let yaw = tg.yaw;
    let p = vec3(tg.x, 0.0, tg.z);
    b.set_xform(p, S);
    let up = vec3(0.0, 1.0, 0.0);
    let f = vec3(yaw.cos(), 0.0, yaw.sin());
    let r = vec3(-yaw.sin(), 0.0, yaw.cos());
    let spd = (tg.vx * tg.vx + tg.vz * tg.vz).sqrt();
    let amp = (spd / 1.2).min(1.6);
    let ph = tg.walk * 1.5;
    let sw = ph.sin() * 0.15 * amp;
    let bob = (ph * 2.0).cos().abs() * 0.012 * amp;
    let dark = |c: Color, k: f32| Color::new(c.r * k, c.g * k, c.b * k, 1.0);
    let sole = Color::new(0.06, 0.06, 0.07, 1.0);
    let white = Color::new(0.97, 0.97, 0.97, 1.0);
    let ink = Color::new(0.05, 0.05, 0.07, 1.0);

    // legs: hip -> knee -> ankle, knee bends forward on the forward swing
    let hip_y = 0.40 + bob * 0.5;
    for s in [-1.0f32, 1.0] {
        let swing = sw * s;
        let lift = (swing.max(0.0)) * 0.55;
        let hip = p + r * (0.075 * s) + up * hip_y;
        let foot = p + r * (0.08 * s) + f * swing + up * (0.045 + lift);
        let knee = (hip + foot) * 0.5 + f * (0.02 + swing.max(0.0) * 0.5);
        let shin_c = if tg.shorts { tg.skin } else { tg.pants };
        b.limb(hip, knee, 0.052, tg.pants);
        b.limb(knee, foot + up * 0.03, 0.045, shin_c);
        // shoe: sole + upper + toe cap
        let fc = foot + f * 0.035;
        b.obox(fc + up * 0.012, yaw, 0.085, 0.014, 0.055, sole);
        b.obox(fc + up * 0.04, yaw, 0.075, 0.030, 0.052, tg.shoes);
        b.obox(fc + f * 0.05 + up * 0.035, yaw, 0.03, 0.025, 0.05, dark(tg.shoes, 0.8));
    }

    // pelvis + belt
    b.obox(p + up * (hip_y + 0.01), yaw, 0.088, 0.045, 0.165, tg.pants);
    b.obox(p + up * (hip_y + 0.05), yaw, 0.092, 0.012, 0.168, dark(tg.pants, 0.55));

    // torso with collar, hem stripe and pocket
    let ty = hip_y + 0.18 + bob;
    b.obox(p + up * (ty + 0.04), yaw, 0.083, 0.19, 0.17, tg.shirt);
    b.obox(p + up * (ty - 0.15), yaw, 0.087, 0.018, 0.172, dark(tg.shirt, 0.82));
    b.obox(p + up * (ty + 0.215) + f * 0.01, yaw, 0.07, 0.014, 0.10, dark(tg.shirt, 0.75));
    b.obox(p + up * (ty + 0.10) + f * 0.085 + r * 0.075, yaw, 0.004, 0.035, 0.035, dark(tg.shirt, 0.85));
    // shoulders
    for s in [-1.0f32, 1.0] {
        b.obox(p + up * (ty + 0.20) + r * (0.185 * s), yaw, 0.058, 0.032, 0.045, tg.shirt);
    }

    // backpack
    if tg.pack {
        b.obox(p + up * (ty + 0.04) - f * 0.14, yaw, 0.055, 0.15, 0.12, tg.accent);
        b.obox(p + up * (ty - 0.02) - f * 0.19, yaw, 0.012, 0.07, 0.09, dark(tg.accent, 0.8));
        for s in [-1.0f32, 1.0] {
            b.obox(p + up * (ty + 0.08) + r * (0.075 * s), yaw, 0.09, 0.14, 0.012, dark(tg.accent, 0.7));
        }
    }

    // arms: shoulder -> elbow -> hand, swinging opposite to the legs
    for (k, s) in [-1.0f32, 1.0].iter().enumerate() {
        let s = *s;
        let swing = -sw * s * 1.1;
        let sh = p + up * (ty + 0.19) + r * (0.205 * s);
        let carry = tg.bag && k == 1;
        let hand = if carry {
            sh + r * 0.03 * s + f * 0.05 - up * 0.30
        } else {
            sh + r * 0.02 * s + f * swing - up * (0.30 - swing.abs() * 0.1)
        };
        let elbow = sh + (hand - sh) * 0.5 + f * (0.015 + swing.max(0.0) * 0.5) + r * 0.012 * s;
        let upper_c = tg.shirt;
        let lower_c = if tg.short_sleeve { tg.skin } else { tg.shirt };
        b.limb(sh, elbow, 0.040, upper_c);
        b.limb(elbow, hand, 0.036, lower_c);
        b.obox(hand - up * 0.025, yaw, 0.03, 0.03, 0.026, tg.skin);
        if carry {
            let bc = hand - up * 0.11;
            b.obox(bc, yaw, 0.035, 0.07, 0.055, tg.accent);
            b.obox(bc + up * 0.075, yaw, 0.037, 0.008, 0.057, dark(tg.accent, 0.75));
        }
    }

    // neck + head
    let hy = ty + 0.34 + bob;
    b.obox(p + up * (hy - 0.095), yaw, 0.032, 0.03, 0.032, dark(tg.skin, 0.92));
    let head = p + up * hy;
    b.obox(head, yaw, 0.082, 0.088, 0.078, tg.skin);
    // ears
    for s in [-1.0f32, 1.0] {
        b.obox(head + r * (0.082 * s) - f * 0.005, yaw, 0.014, 0.024, 0.008, dark(tg.skin, 0.93));
    }
    // face: eyes (white + pupil), brows, nose, mouth
    for s in [-1.0f32, 1.0] {
        let e = head + f * 0.080 + r * (0.036 * s) + up * 0.012;
        b.obox(e, yaw, 0.004, 0.015, 0.020, white);
        b.obox(e + f * 0.003 + r * (0.003 * s), yaw, 0.003, 0.012, 0.011, ink);
        b.obox(e + up * 0.03 + f * 0.003, yaw, 0.003, 0.005, 0.024, dark(tg.hair, 0.9));
    }
    b.obox(head + f * 0.085 - up * 0.010, yaw, 0.010, 0.016, 0.010, dark(tg.skin, 0.88));
    b.obox(head + f * 0.080 - up * 0.050, yaw, 0.004, 0.007, 0.026, Color::new(0.55, 0.22, 0.22, 1.0));

    // hair / cap
    match tg.hair_style {
        0 => {
            b.obox(head + up * 0.075, yaw, 0.088, 0.030, 0.084, tg.hair);
            b.obox(head - f * 0.07 + up * 0.01, yaw, 0.020, 0.075, 0.084, tg.hair);
            b.obox(head + f * 0.085 + up * 0.055, yaw, 0.008, 0.022, 0.078, tg.hair);
            for s in [-1.0f32, 1.0] {
                b.obox(head + r * (0.080 * s) + up * 0.035, yaw, 0.07, 0.04, 0.008, tg.hair);
            }
        }
        1 => {
            b.obox(head + up * 0.078, yaw, 0.090, 0.030, 0.086, tg.hair);
            b.obox(head - f * 0.075 - up * 0.05, yaw, 0.022, 0.135, 0.090, tg.hair);
            b.obox(head + f * 0.087 + up * 0.060, yaw, 0.008, 0.020, 0.080, tg.hair);
            for s in [-1.0f32, 1.0] {
                b.obox(head + r * (0.084 * s) - up * 0.02 - f * 0.01, yaw, 0.07, 0.09, 0.008, tg.hair);
            }
        }
        2 => {
            let cap = tg.accent;
            b.obox(head + up * 0.075, yaw, 0.092, 0.036, 0.090, cap);
            b.obox(head + f * 0.125 + up * 0.050, yaw, 0.050, 0.006, 0.075, dark(cap, 0.8));
            b.obox(head - f * 0.07 + up * 0.005, yaw, 0.020, 0.040, 0.082, tg.hair);
        }
        _ => {
            b.obox(head + up * 0.075, yaw, 0.088, 0.030, 0.084, tg.hair);
            b.obox(head - f * 0.07 + up * 0.02, yaw, 0.020, 0.065, 0.084, tg.hair);
            let tail = (ph * 1.0).sin() * 0.05 * amp;
            let root = head - f * 0.095 + up * 0.04;
            b.limb(root, root - f * 0.05 - up * 0.17 + r * tail, 0.025, tg.hair);
            b.obox(root - f * 0.01, yaw, 0.014, 0.014, 0.014, tg.accent);
        }
    }
    // Compact service rifle held at chest height so every combat bot reads as armed.
    let rifle = p + up * (ty + 0.02) + f * 0.20 + r * 0.12;
    let weapon = Color::new(0.10, 0.12, 0.13, 1.0);
    let metal = Color::new(0.31, 0.34, 0.34, 1.0);
    b.obox(rifle, yaw, 0.22, 0.035, 0.035, weapon);
    b.obox(rifle + f * 0.22, yaw, 0.12, 0.022, 0.022, metal);
    b.obox(rifle - f * 0.17 - up * 0.055, yaw, 0.075, 0.065, 0.027, dark(tg.accent, 0.75));
    b.obox(rifle - f * 0.02 - up * 0.075, yaw, 0.035, 0.075, 0.025, weapon);
    b.clear_xform();
}

pub fn draw_prop(b: &mut Batch, p: &Prop, t: f32) {
    let wob = if p.shake > 0.0 { (t * 70.0).sin() * p.shake * 0.05 } else { 0.0 };
    let c = vec3(p.x + wob, p.y0 + p.hy, p.z - wob * 0.6);
    let dark = |col: Color, k: f32| Color::new(col.r * k, col.g * k, col.b * k, 1.0);
    match p.kind {
        P_CRATE => {
            b.aabb(c, vec3(p.hx, p.hy, p.hz), p.col);
            // frame: corner posts + top/bottom bands, so it reads as a wooden crate
            let post = dark(p.col2, 1.0);
            for sx in [-1.0f32, 1.0] {
                for sz in [-1.0f32, 1.0] {
                    b.aabb(c + vec3(sx * (p.hx - 0.035), 0.0, sz * (p.hz - 0.035)), vec3(0.05, p.hy + 0.005, 0.05), post);
                }
            }
            b.aabb(c + vec3(0.0, p.hy - 0.045, 0.0), vec3(p.hx + 0.008, 0.04, p.hz + 0.008), post);
            b.aabb(c - vec3(0.0, p.hy - 0.045, 0.0), vec3(p.hx + 0.008, 0.04, p.hz + 0.008), post);
            b.aabb(c, vec3(p.hx + 0.006, 0.035, p.hz + 0.006), dark(p.col, 0.8));
        }
        P_BENCH => {
            let ax = p.along_x;
            let m = |u: f32, v: f32| if ax { vec3(u, 0.0, v) } else { vec3(v, 0.0, u) };
            let h = |lu: f32, hy: f32, lv: f32| if ax { vec3(lu, hy, lv) } else { vec3(lv, hy, lu) };
            let base = vec3(c.x, p.y0, c.z);
            let (len, wid) = if ax { (p.hx, p.hz) } else { (p.hz, p.hx) };
            for s in [-1.0f32, 1.0] {
                b.aabb(base + m(s * len * 0.82, 0.0) + vec3(0.0, 0.16, 0.0), h(0.04, 0.16, wid * 0.85), p.col2);
            }
            for k in 0..3 {
                let o = (k as f32 - 1.0) * wid * 0.62;
                b.aabb(base + m(0.0, o) + vec3(0.0, 0.33, 0.0), h(len, 0.03, wid * 0.27), dark(p.col, 0.85 + 0.08 * k as f32));
            }
            for k in 0..2 {
                b.aabb(base + m(0.0, -wid * 0.9) + vec3(0.0, 0.50 + 0.1 * k as f32 + 0.04, 0.0), h(len, 0.035, 0.02), dark(p.col, 0.9));
            }
            for s in [-1.0f32, 1.0] {
                b.aabb(base + m(s * len * 0.82, -wid * 0.9) + vec3(0.0, 0.45, 0.0), h(0.03, 0.2, 0.025), p.col2);
            }
        }
        P_CONE => {
            let base = vec3(c.x, p.y0, c.z);
            b.aabb(base + vec3(0.0, 0.03, 0.0), vec3(0.2, 0.03, 0.2), dark(p.col, 0.75));
            b.aabb(base + vec3(0.0, 0.2, 0.0), vec3(0.145, 0.14, 0.145), p.col);
            b.aabb(base + vec3(0.0, 0.40, 0.0), vec3(0.108, 0.055, 0.108), p.col2);
            b.aabb(base + vec3(0.0, 0.54, 0.0), vec3(0.075, 0.09, 0.075), p.col);
        }
        P_BIN => {
            let base = vec3(c.x, p.y0, c.z);
            b.aabb(base + vec3(0.0, p.hy * 0.9, 0.0), vec3(p.hx * 0.9, p.hy * 0.9, p.hz * 0.9), p.col);
            b.aabb(base + vec3(0.0, p.hy * 0.5, 0.0), vec3(p.hx * 0.93, 0.03, p.hz * 0.93), p.col2);
            b.aabb(base + vec3(0.0, p.hy * 1.85, 0.0), vec3(p.hx * 1.04, 0.045, p.hz * 1.04), p.col2);
            b.aabb(base + vec3(0.0, p.hy * 2.0 - 0.01, 0.0), vec3(0.07, 0.03, 0.04), dark(p.col2, 0.7));
        }
        _ => {
            let base = vec3(c.x, p.y0, c.z);
            let a = p.hx * 0.93;
            for yaw in [0.0f32, std::f32::consts::FRAC_PI_4] {
                b.obox(base + vec3(0.0, p.hy, 0.0), yaw, a, p.hy, a, p.col);
            }
            for yy in [0.12f32, p.hy * 2.0 - 0.12] {
                for yaw in [0.0f32, std::f32::consts::FRAC_PI_4] {
                    b.obox(base + vec3(0.0, yy, 0.0), yaw, p.hx * 0.97, 0.03, p.hx * 0.97, p.col2);
                }
            }
            b.obox(base + vec3(0.0, p.hy, 0.0), 0.0, p.hx * 0.96, 0.025, p.hx * 0.96, dark(p.col, 0.8));
            b.obox(base + vec3(0.0, p.hy * 2.0 + 0.005, 0.0), 0.0, p.hx * 0.7, 0.012, p.hx * 0.7, p.col2);
        }
    }
}

pub fn draw_player(b: &mut Batch, g: &Game, w: &World) {
    let kb = if g.sneaking { 0.78 } else { 1.0 };
    let a = g.slide_a; // 0 = upright, 1 = fully in the slide pose
    let yaw = g.yaw;
    let base = g.feet();
    b.set_xform(base, S);
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

    // soft shadow (stretches along the slide)
    let sy = base.y + (g.ground_here(w) - g.y + 0.03) / S;
    let sx = 0.3 + 0.25 * a;
    b.quad(
        [
            vec3(g.x - sx, sy, g.z + 0.3),
            vec3(g.x + sx, sy, g.z + 0.3),
            vec3(g.x + sx, sy, g.z - 0.3),
            vec3(g.x - sx, sy, g.z - 0.3),
        ],
        Color::new(0.0, 0.0, 0.0, 0.35),
    );

    // body lean: back while sliding, slightly forward while sprinting
    let lean = a * 1.05 - g.boost * (1.0 - a) * 0.10;
    let (sl, cl) = lean.sin_cos();
    let dir_t = up * cl - f * sl; // along the spine
    let dd = f * cl + up * sl; // chest depth axis
    let run_ph = g.walk * 1.5;
    let run_amt = if g.grounded && !g.sliding { ((g.moving - 0.9) / 0.6).clamp(0.0, 1.0) } else { 0.0 };
    let run_bob = (run_ph * 2.0).cos().abs() * 0.022 * run_amt;
    let hip_h = 0.40 * kb * (1.0 - a) + 0.17 * a + run_bob;
    let hip = base + up * hip_h - f * (0.04 * a) + r * (run_ph.sin() * 0.012 * run_amt);

    // direction the legs point: toward the slide while sliding
    let sd = vec3(g.slide_dir.x, 0.0, g.slide_dir.y);
    let fs = (f * (1.0 - a) + sd * a).normalize_or_zero();
    let fs = if fs.length() < 0.5 { f } else { fs };
    let yaw_s = fs.z.atan2(fs.x);

    // legs (hip -> knee -> foot): walk / full sprint cycle with knee drive, one leg out / one folded when sliding
    let ph = g.walk * 1.5;
    let run = if g.grounded && !g.sliding { ((g.moving - 0.9) / 0.6).clamp(0.0, 1.0) } else { 0.0 };
    let amp = (0.15 + 0.17 * run) * g.moving.min(1.0);
    let sw = ph.sin() * amp;
    let tuck = if g.grounded { 0.0 } else { 0.10 };
    let bob = (ph * 2.0).cos().abs() * 0.022 * run * (1.0 - a);
    for s in [-1.0f32, 1.0] {
        let lift_air = if g.grounded { 0.0 } else { tuck * (if s > 0.0 { 1.0 } else { 0.3 }) };
        // foot rises while the leg swings forward (recovery), plants while it drives back
        let recover = (ph.cos() * s).max(0.0);
        let lift_run = recover * (0.05 + 0.13 * run) * g.moving.min(1.0);
        let lat = r * (0.095 * s);
        // upright pose
        let h0 = base + lat + up * (0.40 * kb + bob);
        let f0 = base + lat + f * (sw * s * (1.0 - a) + 0.03) + up * (0.05 + lift_air + lift_run);
        let drive = (sw * s).max(0.0);
        let k0 = (h0 + f0) * 0.5 + f * (0.02 + drive * (0.6 + 1.2 * run) + recover * 0.05 * run);
        // slide pose
        let h1 = hip + lat;
        let (f1, k1) = if s > 0.0 {
            let ft = base + lat + fs * 0.30 + up * 0.07;
            (ft, (h1 + ft) * 0.5 + up * 0.015)
        } else {
            let kn = h1 + fs * 0.12 + up * 0.14;
            (kn - fs * 0.05 - up * 0.15 + lat * 0.2, kn)
        };
        let hp = h0 + (h1 - h0) * a;
        let fp = f0 + (f1 - f0) * a;
        let kp = k0 + (k1 - k0) * a;
        let bya = if a > 0.5 { yaw_s } else { yaw };
        b.limb(hp, kp, 0.062, pants);
        b.limb(kp, fp + up * 0.04, 0.058, pants);
        b.obox(fp + fs * (0.02 * a) + f * (0.02 * (1.0 - a)), bya, 0.10, 0.05, 0.07, boot);
        b.obox(fp + up * 0.012 + f * (0.04 * (1.0 - a)) + fs * (0.04 * a), bya, 0.045, 0.02, 0.06, Color::new(0.16, 0.16, 0.18, 1.0));
    }

    // belt + pouches, torso, vest, patch, pack
    b.boxed(hip, dd * 0.115, dir_t * 0.025, r * 0.20, belt);
    b.boxed(hip + r * 0.19 + dd * 0.03, dd * 0.04, dir_t * 0.035, r * 0.04, pack);
    b.boxed(hip - r * 0.19 + dd * 0.03, dd * 0.04, dir_t * 0.035, r * 0.04, pack);

    let tc = hip + dir_t * (0.19 * kb);
    b.boxed(tc, dd * 0.10, dir_t * (0.19 * kb), r * 0.19, shirt);
    b.boxed(tc + dd * 0.05 + dir_t * (0.01 * kb), dd * 0.06, dir_t * (0.15 * kb), r * 0.17, vest);
    b.boxed(tc - dd * 0.07 + dir_t * (0.01 * kb), dd * 0.04, dir_t * (0.15 * kb), r * 0.16, vest);
    b.boxed(tc + dd * 0.112 + dir_t * (0.07 * kb), dd * 0.012, dir_t * (0.035 * kb), r * 0.07, patch);
    b.boxed(tc - dd * 0.17 + dir_t * (0.02 * kb), dd * 0.06, dir_t * (0.14 * kb), r * 0.13, pack);

    // neck + head (head stays more upright than the chest)
    let neck = hip + dir_t * (0.40 * kb);
    b.boxed(neck, dd * 0.04, dir_t * 0.03, r * 0.04, skin);
    let hl = lean * 0.45;
    let dir_h = up * hl.cos() - f * hl.sin();
    let head = hip + dir_h * (0.53 * kb);
    b.obox(head, yaw, 0.085, 0.09, 0.085, skin);
    b.obox(head + up * 0.05, yaw, 0.095, 0.05, 0.095, helmet);
    b.obox(head + up * 0.14, yaw, 0.02, 0.012, 0.05, Color::new(0.20, 0.28, 0.24, 1.0));
    b.obox(head + f * 0.088, yaw, 0.01, 0.03, 0.075, visor);

    let gc = g.gun_model();
    if g.weapon == Weapon::Rifle {
        b.boxed(gc, d * 0.17, u2 * 0.035, r * 0.03, gun_dark);
        b.boxed(gc + d * 0.17, d * 0.07, u2 * 0.028, r * 0.026, gun_mid);
        b.boxed(gc + d * 0.31, d * 0.10, u2 * 0.013, r * 0.013, gun_dark);
        b.boxed(gc - d * 0.22, d * 0.08, u2 * 0.035, r * 0.025, gun_wood);
        b.boxed(gc + d * 0.03 - u2 * 0.075, d * 0.02, u2 * 0.05, r * 0.02, gun_dark);
        b.boxed(gc + u2 * 0.05 + d * 0.04, d * 0.05, u2 * 0.015, r * 0.012, gun_dark);
    } else {
        let grip = gc - d * 0.10;
        b.boxed(grip, d * 0.07, u2 * 0.025, r * 0.025, gun_dark);
        b.boxed(gc + d * 0.13, d * 0.16, u2 * 0.012, r * 0.018, Color::new(0.72, 0.78, 0.78, 1.0));
        b.boxed(gc + d * 0.22, d * 0.04, u2 * 0.025, r * 0.025, gun_mid);
    }

    let sh_r = hip + dir_t * (0.34 * kb) + r * 0.24;
    let sh_l = hip + dir_t * (0.34 * kb) - r * 0.24;
    let hand_r = gc - d * 0.05 - u2 * 0.045;
    let hand_l = gc + d * 0.17 - u2 * 0.04;
    for (sh, hand) in [(sh_r, hand_r), (sh_l, hand_l)] {
        let mid = sh + (hand - sh) * 0.62;
        b.limb(sh, mid, 0.05, shirt);
        b.limb(mid, hand, 0.04, glove);
        b.aabb(sh, vec3(0.06, 0.05, 0.06), helmet);
    }
    b.clear_xform();
}

pub fn draw_fx(b: &mut Batch, g: &Game, right: Vec3, up: Vec3) {
    for d in &g.debris {
        let a = d.q * vec3(d.half.x, 0.0, 0.0);
        let bb = d.q * vec3(0.0, d.half.y, 0.0);
        let c = d.q * vec3(0.0, 0.0, d.half.z);
        b.boxed(d.p, a, bb, c, d.col);
    }
    for p in &g.parts {
        let a = (p.life / p.max).clamp(0.0, 1.0);
        let col = Color::new(p.col.r, p.col.g, p.col.b, a);
        b.billboard(p.p, right, up, p.size * (0.5 + 0.5 * a), col);
    }
    if g.flash > 0.0 && g.zoom < 0.7 {
        let m = g.to_world(g.gun_model() + g.aim_dir() * 0.50);
        let s = 0.16 + g.flash * 2.0;
        b.billboard(m, right, up, s, Color::new(1.0, 0.85, 0.4, 0.95));
        b.billboard(m, right, up, s * 0.5, Color::new(1.0, 1.0, 0.85, 1.0));
        b.billboard(m, right + up, vec3(0.0, 0.0, 0.0) + up - right, s * 0.8, Color::new(1.0, 0.7, 0.25, 0.7));
    }
}

pub fn draw_crosshair(g: &Game, w: f32, h: f32, u: f32) {
    let cx = w * 0.5;
    let cy = h * 0.5;
    let gap = 5.0 * u + g.moving * 2.5 * u + g.shake * 30.0 * u;
    let len = 10.0 * u;
    let sh = Color::new(0.0, 0.0, 0.0, 0.5);
    let wh = Color::new(1.0, 1.0, 1.0, 0.95);
    for (c, t) in [(sh, 4.0), (wh, 2.0)] {
        draw_line(cx - gap - len, cy, cx - gap, cy, t, c);
        draw_line(cx + gap, cy, cx + gap + len, cy, t, c);
        draw_line(cx, cy - gap - len, cx, cy - gap, t, c);
        draw_line(cx, cy + gap, cx, cy + gap + len, t, c);
    }
    draw_circle(cx, cy, 1.6 * u, wh);
}

pub fn draw_scope(g: &Game, w: f32, h: f32, u: f32) {
    if g.zoom < 0.35 {
        return;
    }
    let cx = w * 0.5;
    let cy = h * 0.5;
    let red = Color::new(0.95, 0.18, 0.12, 0.9);
    let shadow = Color::new(0.02, 0.02, 0.02, 0.75);
    let gap = 6.0 * u;
    let len = 12.0 * u;
    for (c, thick) in [(shadow, 3.0 * u), (red, 1.4 * u)] {
        draw_line(cx - gap - len, cy, cx - gap, cy, thick, c);
        draw_line(cx + gap, cy, cx + gap + len, cy, thick, c);
        draw_line(cx, cy - gap - len, cx, cy - gap, thick, c);
        draw_line(cx, cy + gap, cx, cy + gap + len, thick, c);
    }
    draw_circle(cx, cy, 1.5 * u, red);
    draw_text(&format!("{:.1}x", g.mag), cx + 18.0 * u, cy + 30.0 * u, 17.0 * u, WHITE);
    let range = if g.range > 0.0 { format!("{:.0}m", g.range) } else { "--m".to_string() };
    draw_text(&range, cx + 18.0 * u, cy + 49.0 * u, 15.0 * u, WHITE);
}

pub fn draw_minimap(g: &Game, w: &World, x: f32, y: f32, size: f32) {
    draw_rectangle(x, y, size, size, Color::new(0.13, 0.16, 0.17, 0.9));
    draw_rectangle_lines(x, y, size, size, 2.0, Color::new(0.8, 0.92, 1.0, 0.7));
    let k = size / SIZE;
    for v in [SIZE*0.30,SIZE*0.50,SIZE*0.70] {
        draw_line(x+v*k,y+2.0,x+v*k,y+size-2.0,1.0,Color::new(0.8,0.82,0.76,0.16));
        draw_line(x+2.0,y+v*k,x+size-2.0,y+v*k,1.0,Color::new(0.8,0.82,0.76,0.16));
    }
    for bx in &w.boxes {
        draw_rectangle(x+bx.x0*k,y+bx.z0*k,(bx.x1-bx.x0)*k,(bx.z1-bx.z0)*k,Color::new(0.60,0.30,0.10,0.95));
    }
    for t in &g.targets {
        if t.alive {
            let c=if t.team==Team::Red {Color::new(1.0,0.25,0.18,1.0)} else {Color::new(0.16,0.55,1.0,1.0)};
            draw_circle(x + t.x * k, y + t.z * k, 2.3, c);
        }
    }
    let px = x + g.x * k;
    let py = y + g.z * k;
    let d = vec2(g.yaw.cos(), g.yaw.sin());
    draw_circle(px, py, 3.5, Color::new(0.4, 0.95, 0.8, 1.0));
    draw_line(px, py, px + d.x * 10.0, py + d.y * 10.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.9));
}
