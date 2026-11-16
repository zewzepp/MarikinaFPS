use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

// '#' = wall, 'c' = low crate (jump over it or onto it), '.' = floor
const MAP: [&str; 14] = [
    "####################",
    "#..................#",
    "#..cc..........cc..#",
    "#..................#",
    "#.......####.......#",
    "#.......#..........#",
    "#..................#",
    "#.c..............c.#",
    "#..........##......#",
    "#..................#",
    "#..cc..............#",
    "#..................#",
    "#.............cc...#",
    "####################",
];

const WALL_H: f32 = 1.5;
const CRATE_H: f32 = 0.5;
const PLAYER_R: f32 = 0.2;
const MAX_DIST: f32 = 40.0;
const MAG: i32 = 12;
const GRAVITY: f32 = 11.5;
const JUMP_V: f32 = 4.6;

fn cell(x: f32, z: f32) -> u8 {
    if x < 0.0 || z < 0.0 {
        return b'#';
    }
    MAP.get(z as usize)
        .and_then(|row| row.as_bytes().get(x as usize))
        .copied()
        .unwrap_or(b'#')
}

fn height_at(x: f32, z: f32) -> f32 {
    match cell(x, z) {
        b'#' => WALL_H,
        b'c' => CRATE_H,
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Tiny 3D helper: collects boxes/quads into a mesh so we make few draw calls.
// ---------------------------------------------------------------------------
struct Batch {
    v: Vec<Vertex>,
    i: Vec<u16>,
}

impl Batch {
    fn new() -> Self {
        Self { v: Vec::new(), i: Vec::new() }
    }

    fn flush(&mut self) {
        if self.v.is_empty() {
            return;
        }
        let mesh = Mesh {
            vertices: std::mem::take(&mut self.v),
            indices: std::mem::take(&mut self.i),
            texture: None,
        };
        draw_mesh(&mesh);
    }

    fn quad(&mut self, p: [Vec3; 4], col: Color) {
        let base = self.v.len() as u16;
        for q in p {
            self.v.push(Vertex::new(q.x, q.y, q.z, 0.0, 0.0, col));
        }
        self.i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if self.v.len() > 1500 {
            self.flush();
        }
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
    health: i32,
    ammo: i32,
    reserve: i32,
    reloading: f32,
    fire_cd: f32,
    scoped: bool,
    sneak_on: bool,
    sneaking: bool,
    slide: f32,
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
            x: 2.5,
            y: 0.0,
            z: 11.5,
            vy: 0.0,
            grounded: true,
            yaw: -0.5,
            pitch: 0.0,
            zoom: 0.0,
            health: 100,
            ammo: MAG,
            reserve: 60,
            reloading: 0.0,
            fire_cd: 0.0,
            scoped: false,
            sneak_on: false,
            sneaking: false,
            slide: 0.0,
            slide_dir: Vec2::ZERO,
            walk: 0.0,
            moving: 0.0,
            kills: 0,
            targets: vec![
                Target::new(9.5, 2.5, 0.0),
                Target::new(15.5, 6.5, 1.8),
                Target::new(6.5, 11.5, 3.2),
                Target::new(13.5, 9.5, 4.4),
                Target::new(17.5, 11.5, 5.6),
            ],
            tracer: None,
            tracer_t: 0.0,
            hit_t: 0.0,
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
        vec3(self.x, self.y + if self.slide > 0.0 { 0.40 } else if self.sneaking { 0.62 } else { 0.78 }, self.z)
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
            t += 0.04;
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

        let f = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        let muzzle = vec3(self.x, self.y + 0.55, self.z) + f * 0.6 + self.right() * 0.18;
        self.tracer = Some((muzzle, end));
        self.tracer_t = 0.07;
    }
}

// ---------------------------------------------------------------------------
// Touch controls
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq)]
enum Action {
    Fire,
    Jump,
    Reload,
    Scope,
    Sneak,
    Slide,
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Joy,
    Look,
    Btn,
}

struct Pad {
    c: Vec2,
    r: f32,
}

struct Layout {
    u: f32,
    joy_home: Vec2,
    joy_r: f32,
    fire: Pad,
    jump: Pad,
    reload: Pad,
    scope: Pad,
    sneak: Pad,
    slide: Pad,
}

impl Layout {
    fn new(w: f32, h: f32) -> Self {
        let u = (h / 720.0).clamp(0.75, 1.8);
        Self {
            u,
            joy_home: vec2(150.0 * u, h - 150.0 * u),
            joy_r: 85.0 * u,
            fire: Pad { c: vec2(w - 115.0 * u, h - 100.0 * u), r: 58.0 * u },
            jump: Pad { c: vec2(w - 115.0 * u, h - 228.0 * u), r: 44.0 * u },
            reload: Pad { c: vec2(w - 240.0 * u, h - 85.0 * u), r: 40.0 * u },
            scope: Pad { c: vec2(w - 220.0 * u, h - 210.0 * u), r: 40.0 * u },
            sneak: Pad { c: vec2(w - 350.0 * u, h - 95.0 * u), r: 38.0 * u },
            slide: Pad { c: vec2(w - 350.0 * u, h - 215.0 * u), r: 38.0 * u },
        }
    }

    fn pads(&self) -> [(Action, &Pad); 6] {
        [
            (Action::Fire, &self.fire),
            (Action::Jump, &self.jump),
            (Action::Reload, &self.reload),
            (Action::Scope, &self.scope),
            (Action::Sneak, &self.sneak),
            (Action::Slide, &self.slide),
        ]
    }

    fn hit(&self, p: Vec2) -> Option<Action> {
        let mut best = None;
        let mut best_d = f32::MAX;
        for (a, pad) in self.pads() {
            let d = p.distance(pad.c);
            if d <= pad.r * 1.2 && d < best_d {
                best = Some(a);
                best_d = d;
            }
        }
        best
    }

    // Keep the floating joystick base fully on screen.
    fn clamp_origin(&self, p: Vec2, w: f32, h: f32) -> Vec2 {
        let m = self.joy_r + 10.0 * self.u;
        vec2(p.x.clamp(m, (w - m).max(m)), p.y.clamp(m, (h - m).max(m)))
    }
}

fn draw_pad(p: &Pad, label: &str, active: bool, u: f32) {
    let fill = if active {
        Color::new(0.23, 0.62, 0.78, 0.80)
    } else {
        Color::new(0.04, 0.08, 0.12, 0.55)
    };
    draw_circle(p.c.x, p.c.y, p.r, fill);
    draw_circle_lines(p.c.x, p.c.y, p.r, 2.5 * u, Color::new(0.8, 0.92, 1.0, 0.85));
    let size = (p.r * 0.36).max(12.0);
    let m = measure_text(label, None, size as u16, 1.0);
    draw_text(label, p.c.x - m.width / 2.0, p.c.y + m.height * 0.35, size, WHITE);
}

// ---------------------------------------------------------------------------
// Drawing the world
// ---------------------------------------------------------------------------
fn draw_world(b: &mut Batch) {
    for (gz, row) in MAP.iter().enumerate() {
        for (gx, ch) in row.bytes().enumerate() {
            let x = gx as f32;
            let z = gz as f32;
            if ch == b'#' {
                b.aabb(
                    vec3(x + 0.5, WALL_H * 0.5, z + 0.5),
                    vec3(0.5, WALL_H * 0.5, 0.5),
                    Color::new(0.27, 0.48, 0.57, 1.0),
                );
            } else {
                let c = if (gx + gz) % 2 == 0 {
                    Color::new(0.20, 0.25, 0.23, 1.0)
                } else {
                    Color::new(0.16, 0.20, 0.19, 1.0)
                };
                b.quad(
                    [vec3(x, 0.0, z + 1.0), vec3(x + 1.0, 0.0, z + 1.0), vec3(x + 1.0, 0.0, z), vec3(x, 0.0, z)],
                    c,
                );
                if ch == b'c' {
                    b.aabb(
                        vec3(x + 0.5, CRATE_H * 0.5, z + 0.5),
                        vec3(0.46, CRATE_H * 0.5, 0.46),
                        Color::new(0.70, 0.48, 0.26, 1.0),
                    );
                }
            }
        }
    }
}

fn draw_target(b: &mut Batch, tg: &Target, px: f32, pz: f32) {
    let yaw = (pz - tg.z).atan2(px - tg.x); // face the player
    let p = vec3(tg.x, 0.0, tg.z);
    b.obox(p + vec3(0.0, 0.17, 0.0), yaw, 0.07, 0.17, 0.12, Color::new(0.45, 0.46, 0.48, 1.0));
    b.obox(p + vec3(0.0, 0.52, 0.0), yaw, 0.11, 0.20, 0.20, Color::new(0.72, 0.18, 0.15, 1.0));
    b.obox(p + vec3(0.0, 0.86, 0.0), yaw, 0.10, 0.10, 0.10, Color::new(0.88, 0.59, 0.32, 1.0));
}

fn draw_player(b: &mut Batch, g: &Game) {
    let k = if g.slide > 0.0 { 0.5 } else if g.sneaking { 0.78 } else { 1.0 };
    let yaw = g.yaw;
    let base = vec3(g.x, g.y, g.z);
    let f = vec3(yaw.cos(), 0.0, yaw.sin());
    let r = vec3(-yaw.sin(), 0.0, yaw.cos());

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

    // legs (swing while walking)
    let sw = (g.walk * 2.0).sin() * 0.14 * g.moving.min(1.0);
    for s in [-1.0f32, 1.0] {
        let off = r * (0.10 * s) + f * (sw * s);
        b.obox(base + off + vec3(0.0, 0.17 * k, 0.0), yaw, 0.07, 0.17 * k, 0.07, Color::new(0.16, 0.18, 0.22, 1.0));
    }
    // torso, head, visor
    b.obox(base + vec3(0.0, 0.55 * k, 0.0), yaw, 0.11, 0.20 * k, 0.21, Color::new(0.25, 0.60, 0.50, 1.0));
    b.obox(base + vec3(0.0, 0.90 * k, 0.0), yaw, 0.10, 0.10, 0.10, Color::new(0.90, 0.70, 0.55, 1.0));
    b.obox(base + vec3(0.0, 0.92 * k, 0.0) + f * 0.09, yaw, 0.02, 0.04, 0.09, Color::new(0.08, 0.09, 0.12, 1.0));

    // arm + gun, tilted to follow where you aim
    let d = g.aim_dir();
    let up_axis = r.cross(d);
    let chest = base + vec3(0.0, 0.56 * k, 0.0);
    b.obox(chest + f * 0.20 + r * 0.20, yaw, 0.18, 0.05, 0.05, Color::new(0.20, 0.50, 0.42, 1.0));
    let gc = chest + f * 0.38 + r * 0.20;
    b.boxed(gc, d * 0.30, up_axis * 0.045, r * 0.045, Color::new(0.12, 0.13, 0.15, 1.0));
    if g.tracer_t > 0.04 {
        b.aabb(gc + d * 0.36, vec3(0.06, 0.06, 0.06), Color::new(1.0, 0.85, 0.3, 1.0));
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------
#[macroquad::main("Marikina FPS")]
async fn main() {
    let mut g = Game::new();
    let mut roles: HashMap<u64, Role> = HashMap::new();
    let mut last: HashMap<u64, Vec2> = HashMap::new();
    let mut joy_origin: Option<Vec2> = None;
    let mut touch_seen = false;

    loop {
        let dt = get_frame_time().min(0.05);
        let w = screen_width();
        let h = screen_height();
        let ui = Layout::new(w, h);
        let u = ui.u;

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

        let mut mv = Vec2::ZERO; // x = strafe right, y = forward
        let mut look = Vec2::ZERO; // screen-style delta (down = positive)
        let mut fire = false;
        let mut jump = false;
        let mut reload = false;
        let mut scope_tap = false;
        let mut sneak_tap = false;
        let mut slide_tap = false;
        let mut joy_knob: Option<Vec2> = None;

        for t in &touch_list {
            let p = t.position;
            match t.phase {
                TouchPhase::Started => {
                    let role = if let Some(a) = ui.hit(p) {
                        match a {
                            Action::Jump => jump = true,
                            Action::Reload => reload = true,
                            Action::Scope => scope_tap = true,
                            Action::Sneak => sneak_tap = true,
                            Action::Slide => slide_tap = true,
                            Action::Fire => {}
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
                    let origin = joy_origin.unwrap_or(ui.joy_home);
                    let mut v = (p - origin) / ui.joy_r;
                    if v.length() > 1.0 {
                        v = v.normalize();
                    }
                    let dz = if v.length() < 0.1 { Vec2::ZERO } else { v };
                    mv = vec2(dz.x, -dz.y);
                    joy_knob = Some(origin + v * ui.joy_r);
                }
                Role::Look => {
                    if let Some(prev) = last.get(&t.id) {
                        look += p - *prev;
                    }
                    last.insert(t.id, p);
                }
                Role::Btn => {
                    if ui.hit(p) == Some(Action::Fire) {
                        fire = true;
                    }
                }
            }
        }

        // Keyboard + mouse (desktop)
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
            sneak_tap = true;
        }
        if is_key_pressed(KeyCode::LeftShift) {
            slide_tap = true;
        }
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

        // Apply actions
        if scope_tap {
            g.scoped = !g.scoped;
        }
        if sneak_tap {
            g.sneak_on = !g.sneak_on;
        }
        g.sneaking = g.sneak_on || is_key_down(KeyCode::LeftControl);
        if reload {
            g.start_reload();
        }
        if jump && g.grounded {
            g.vy = JUMP_V;
            g.grounded = false;
        }
        if slide_tap && g.grounded && g.slide <= 0.0 {
            let sf = vec2(g.yaw.cos(), g.yaw.sin());
            let sr = vec2(-g.yaw.sin(), g.yaw.cos());
            let sv = sf * mv.y + sr * mv.x;
            g.slide_dir = if sv.length() > 0.1 { sv.normalize() } else { sf };
            g.slide = 0.6;
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
        let (dir, speed) = if g.slide > 0.0 {
            (g.slide_dir, 3.0 + 5.0 * (g.slide / 0.6))
        } else {
            (f * mv.y + r * mv.x, if g.sneaking { 1.5 } else { 3.4 })
        };
        g.move_by(dir.x * speed * dt, dir.y * speed * dt);
        g.moving = if g.slide > 0.0 { 0.0 } else { mv.length() };
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
        g.slide = (g.slide - dt).max(0.0);
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
        clear_background(Color::new(0.36, 0.52, 0.66, 1.0));

        let cam = g.cam_pos();
        let d = g.aim_dir();
        set_camera(&Camera3D {
            position: cam,
            target: cam + d * 10.0,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 1.05 - 0.6 * g.zoom,
            ..Default::default()
        });

        let mut b = Batch::new();
        draw_world(&mut b);
        for tg in &g.targets {
            if tg.alive {
                draw_target(&mut b, tg, g.x, g.z);
            }
        }
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
        draw_text("RUSTY FPS  /  TRAINING YARD", pad, 27.0 * u, 22.0 * u, WHITE);
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

        if mobile {
            // floating joystick
            let origin = joy_origin.unwrap_or(ui.joy_home);
            draw_circle(origin.x, origin.y, ui.joy_r, Color::new(0.04, 0.08, 0.12, 0.42));
            draw_circle_lines(origin.x, origin.y, ui.joy_r, 2.5 * u, Color::new(0.8, 0.92, 1.0, 0.6));
            let knob = joy_knob.unwrap_or(origin);
            draw_circle(knob.x, knob.y, ui.joy_r * 0.42, Color::new(0.8, 0.92, 1.0, 0.65));

            draw_pad(&ui.fire, "FIRE", fire, u);
            draw_pad(&ui.jump, "JUMP", !g.grounded, u);
            draw_pad(&ui.reload, "RELOAD", g.reloading > 0.0, u);
            draw_pad(&ui.scope, "SCOPE", g.scoped, u);
            draw_pad(&ui.sneak, "SNEAK", g.sneaking, u);
            draw_pad(&ui.slide, "SLIDE", g.slide > 0.0, u);
        }

        next_frame().await
    }
}
