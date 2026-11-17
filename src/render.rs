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
    b.clear_xform();
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
    let hip_h = 0.40 * kb * (1.0 - a) + 0.17 * a;
    let hip = base + up * hip_h - f * (0.04 * a);

    // direction the legs point: toward the slide while sliding
    let sd = vec3(g.slide_dir.x, 0.0, g.slide_dir.y);
    let fs = (f * (1.0 - a) + sd * a).normalize_or_zero();
    let fs = if fs.length() < 0.5 { f } else { fs };
    let yaw_s = fs.z.atan2(fs.x);

    // legs (hip -> knee -> foot): stride swing when running, one leg out / one folded when sliding
    let sw = (g.walk * 1.5).sin() * 0.15 * g.moving.min(1.2);
    let tuck = if g.grounded { 0.0 } else { 0.10 };
    for s in [-1.0f32, 1.0] {
        let lift = if g.grounded { 0.0 } else { tuck * (if s > 0.0 { 1.0 } else { 0.3 }) };
        let lat = r * (0.095 * s);
        // upright pose
        let h0 = base + lat + up * (0.40 * kb);
        let f0 = base + lat + f * (sw * s * (1.0 - a) + 0.03) + up * (0.05 + lift);
        let k0 = (h0 + f0) * 0.5 + f * (0.02 + (sw * s).max(0.0) * 0.6);
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
    b.boxed(gc, d * 0.17, u2 * 0.035, r * 0.03, gun_dark);
    b.boxed(gc + d * 0.17, d * 0.07, u2 * 0.028, r * 0.026, gun_mid);
    b.boxed(gc + d * 0.31, d * 0.10, u2 * 0.013, r * 0.013, gun_dark);
    b.boxed(gc - d * 0.22, d * 0.08, u2 * 0.035, r * 0.025, gun_wood);
    b.boxed(gc + d * 0.03 - u2 * 0.075, d * 0.02, u2 * 0.05, r * 0.02, gun_dark);
    b.boxed(gc + u2 * 0.05 + d * 0.04, d * 0.05, u2 * 0.015, r * 0.012, gun_dark);

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
    let a = ((g.zoom - 0.35) / 0.65).clamp(0.0, 1.0);
    let cx = w * 0.5;
    let cy = h * 0.5;
    let s = h.min(w) * 0.92;
    let rad = s * 0.5;
    let x0 = cx - rad;
    let y0 = cy - rad;
    let dark = Color::new(0.0, 0.0, 0.0, 0.97 * a);
    // outside the lens
    draw_rectangle(0.0, 0.0, x0.max(0.0), h, dark);
    draw_rectangle(x0 + s, 0.0, (w - x0 - s).max(0.0), h, dark);
    draw_rectangle(x0, 0.0, s, y0.max(0.0), dark);
    draw_rectangle(x0, y0 + s, s, (h - y0 - s).max(0.0), dark);
    // corners of the square beyond the circle
    let t = s * 0.24;
    draw_poly_lines(cx, cy, 120, rad + t * 0.5 - 1.0, 0.0, t, dark);
    // soft vignette inside the lens
    for i in 0..6 {
        let k = i as f32;
        draw_poly_lines(cx, cy, 120, rad - (k + 0.5) * s * 0.012, 0.0, s * 0.012, Color::new(0.0, 0.0, 0.0, (0.30 - k * 0.05) * a));
    }
    draw_circle(cx, cy, rad, Color::new(0.45, 0.75, 0.85, 0.035 * a));
    draw_poly_lines(cx, cy, 120, rad, 0.0, 4.0 * u, Color::new(0.10, 0.11, 0.12, a));
    draw_poly_lines(cx, cy, 120, rad - 5.0 * u, 0.0, 1.5 * u, Color::new(0.45, 0.5, 0.52, 0.6 * a));

    // reticle: fine centre cross, heavy outer posts, mil dots
    let line = Color::new(0.02, 0.02, 0.02, 0.92 * a);
    let thin = 1.3 * u;
    let post = 5.0 * u;
    let inner = rad * 0.07;
    let mid = rad * 0.30;
    for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        draw_line(cx + dx * inner, cy + dy * inner, cx + dx * mid, cy + dy * mid, thin, line);
        draw_line(cx + dx * mid, cy + dy * mid, cx + dx * rad * 0.97, cy + dy * rad * 0.97, post, line);
    }
    for i in 1..=4 {
        let o = inner + (mid - inner) * i as f32 / 4.0;
        for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            draw_circle(cx + dx * o, cy + dy * o, 2.2 * u, line);
        }
    }
    draw_circle(cx, cy, 1.8 * u, Color::new(0.95, 0.12, 0.10, a));

    let tc = Color::new(0.85, 0.95, 0.9, 0.95 * a);
    draw_center(&format!("{:.1}x", g.mag), cx, cy + rad * 0.80, 22.0 * u, tc);
    let rng = if g.range > 0.0 { format!("RNG {:.0} m", g.range) } else { "RNG ---".to_string() };
    draw_center(&rng, cx, cy + rad * 0.88, 18.0 * u, tc);
}

pub fn draw_minimap(g: &Game, x: f32, y: f32, size: f32) {
    draw_rectangle(x, y, size, size, Color::new(0.05, 0.07, 0.10, 0.65));
    draw_rectangle_lines(x, y, size, size, 2.0, Color::new(0.8, 0.92, 1.0, 0.7));
    let k = size / SIZE;
    for bi in 0..NB {
        for bj in 0..NB {
            let bx = bj as f32 * PERIOD + ROAD;
            let bz = bi as f32 * PERIOD + ROAD;
            draw_rectangle(x + bx * k, y + bz * k, 24.0 * k, 24.0 * k, Color::new(0.35, 0.40, 0.45, 0.85));
        }
    }
    for t in &g.targets {
        if t.alive {
            draw_circle(x + t.x * k, y + t.z * k, 2.0, Color::new(1.0, 0.3, 0.25, 1.0));
        }
    }
    let px = x + g.x * k;
    let py = y + g.z * k;
    let d = vec2(g.yaw.cos(), g.yaw.sin());
    draw_circle(px, py, 3.5, Color::new(0.4, 0.95, 0.8, 1.0));
    draw_line(px, py, px + d.x * 10.0, py + d.y * 10.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.9));
}
