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
    let red = Color::new(0.72, 0.18, 0.15, 1.0);
    let dark_red = Color::new(0.45, 0.10, 0.09, 1.0);
    let white = Color::new(0.95, 0.95, 0.92, 1.0);
    let pants = Color::new(0.14, 0.15, 0.18, 1.0);
    let skin = Color::new(0.88, 0.62, 0.38, 1.0);
    let sw = (tg.walk * 1.5).sin() * 0.13;

    for s in [-1.0f32, 1.0] {
        let off = r * (0.07 * s) + f * (sw * s);
        b.obox(p + off + up * 0.19, yaw, 0.055, 0.19, 0.055, pants);
        b.obox(p + off + up * 0.03 + f * 0.03, yaw, 0.08, 0.03, 0.06, Color::new(0.07, 0.07, 0.08, 1.0));
    }
    b.obox(p + up * 0.52, yaw, 0.11, 0.20, 0.20, red);
    b.obox(p + up * 0.34, yaw, 0.115, 0.025, 0.205, Color::new(0.10, 0.10, 0.12, 1.0));
    let c = p + up * 0.54;
    b.obox(c + f * 0.112, yaw, 0.004, 0.14, 0.14, white);
    b.obox(c + f * 0.116, yaw, 0.004, 0.10, 0.10, dark_red);
    b.obox(c + f * 0.120, yaw, 0.004, 0.06, 0.06, white);
    b.obox(c + f * 0.124, yaw, 0.004, 0.025, 0.025, dark_red);
    for s in [-1.0f32, 1.0] {
        b.obox(p + r * (0.25 * s) + up * 0.50 + f * (-sw * s * 0.8), yaw, 0.05, 0.16, 0.05, dark_red);
    }
    b.obox(p + up * 0.83, yaw, 0.085, 0.085, 0.085, skin);
    b.obox(p + up * 0.90, yaw, 0.10, 0.045, 0.10, dark_red);
    b.obox(p + up * 0.83 + f * 0.088, yaw, 0.01, 0.025, 0.07, Color::new(0.08, 0.09, 0.12, 1.0));
    b.clear_xform();
}

pub fn draw_player(b: &mut Batch, g: &Game, w: &World) {
    let k = g.k();
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

    // soft shadow
    let sy = base.y + (g.ground_here(w) - g.y + 0.03) / S;
    b.quad(
        [
            vec3(g.x - 0.3, sy, g.z + 0.3),
            vec3(g.x + 0.3, sy, g.z + 0.3),
            vec3(g.x + 0.3, sy, g.z - 0.3),
            vec3(g.x - 0.3, sy, g.z - 0.3),
        ],
        Color::new(0.0, 0.0, 0.0, 0.35),
    );

    // legs: swing with stride, tuck when airborne
    let mut sw = (g.walk * 1.5).sin() * 0.15 * g.moving.min(1.2);
    let tuck = if g.grounded { 0.0 } else { 0.10 };
    if g.sliding {
        sw = 0.0;
    }
    let leg_top = 0.40 * k;
    let ch = (leg_top - 0.10) * 0.5;
    for s in [-1.0f32, 1.0] {
        let lift = if g.grounded { 0.0 } else { tuck * (if s > 0.0 { 1.0 } else { 0.3 }) };
        let off = r * (0.095 * s) + f * (sw * s + if g.sliding { 0.12 * s.max(0.0) } else { 0.0 });
        b.obox(base + off + up * (0.05 + lift) + f * 0.03, yaw, 0.10, 0.05, 0.07, boot);
        b.obox(base + off + up * (0.10 + ch + lift), yaw, 0.065, ch, 0.07, pants);
    }

    b.obox(base + up * leg_top, yaw, 0.115, 0.025, 0.20, belt);
    b.obox(base + up * leg_top + r * 0.19 + f * 0.03, yaw, 0.04, 0.035, 0.04, pack);
    b.obox(base + up * leg_top - r * 0.19 + f * 0.03, yaw, 0.04, 0.035, 0.04, pack);

    b.obox(base + up * (0.59 * k), yaw, 0.10, 0.19 * k, 0.19, shirt);
    b.obox(base + up * (0.60 * k) + f * 0.05, yaw, 0.06, 0.15 * k, 0.17, vest);
    b.obox(base + up * (0.60 * k) - f * 0.07, yaw, 0.04, 0.15 * k, 0.16, vest);
    b.obox(base + up * (0.66 * k) + f * 0.112, yaw, 0.012, 0.035 * k, 0.07, patch);
    b.obox(base + up * (0.62 * k) - f * 0.17, yaw, 0.06, 0.14 * k, 0.13, pack);

    b.obox(base + up * (0.80 * k), yaw, 0.04, 0.025, 0.04, skin);
    let hy = 0.93 * k;
    b.obox(base + up * hy, yaw, 0.085, 0.09, 0.085, skin);
    b.obox(base + up * (hy + 0.05), yaw, 0.095, 0.05, 0.095, helmet);
    b.obox(base + up * (hy + 0.14), yaw, 0.02, 0.012, 0.05, Color::new(0.20, 0.28, 0.24, 1.0));
    b.obox(base + up * hy + f * 0.088, yaw, 0.01, 0.03, 0.075, visor);

    let gc = g.gun_model();
    b.boxed(gc, d * 0.17, u2 * 0.035, r * 0.03, gun_dark);
    b.boxed(gc + d * 0.17, d * 0.07, u2 * 0.028, r * 0.026, gun_mid);
    b.boxed(gc + d * 0.31, d * 0.10, u2 * 0.013, r * 0.013, gun_dark);
    b.boxed(gc - d * 0.22, d * 0.08, u2 * 0.035, r * 0.025, gun_wood);
    b.boxed(gc + d * 0.03 - u2 * 0.075, d * 0.02, u2 * 0.05, r * 0.02, gun_dark);
    b.boxed(gc + u2 * 0.05 + d * 0.04, d * 0.05, u2 * 0.015, r * 0.012, gun_dark);

    let sh_r = base + up * (0.74 * k) + r * 0.24;
    let sh_l = base + up * (0.74 * k) - r * 0.24;
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
