use crate::audio::Snd;
use crate::world::*;
use macroquad::prelude::*;
use std::f32::consts::TAU;

pub const S: f32 = 1.7; // model scale: 1.0 model unit = 1.7 world units
pub const PR: f32 = 0.32;
pub const STEP: f32 = 0.38;
pub const GRAVITY: f32 = 20.0;
pub const JUMP_V: f32 = 6.4;
pub const WALK: f32 = 3.4;
pub const SPRINT: f32 = 6.2;
pub const SNEAK: f32 = 1.8;
pub const MAG: i32 = 30;
pub const MAX_DIST: f32 = 150.0;
pub const SPRINT_TILT: f32 = 0.9;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Team { Blue, Red }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weapon { Rifle, Knife }

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub mv: Vec2,
    pub look: Vec2,
    pub joy_mag: f32,
    pub kb_sprint: bool,
    pub ctrl: bool,
    pub jump: bool,
    pub fire: bool,
    pub switch_weapon: bool,
    pub reload: bool,
    pub scope_tap: bool,
    pub snk_tap: bool,
    pub snk_held: bool,
    pub slide_hold: bool,
    pub zoom_step: i32,
}

pub struct Target {
    pub team: Team,
    pub hp: i32,
    pub fire_cd: f32,
    pub muzzle_flash: f32,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub x0: f32,
    pub z0: f32,
    pub hd: f32,    // wander heading
    pub timer: f32, // time until next random decision
    pub idle: f32,  // standing still for this long
    pub spd: f32,
    pub yaw: f32,
    pub alive: bool,
    pub respawn: f32,
    pub walk: f32,
    // look
    pub skin: Color,
    pub hair: Color,
    pub shirt: Color,
    pub pants: Color,
    pub shoes: Color,
    pub accent: Color,
    pub hair_style: u8, // 0 short, 1 long, 2 cap, 3 ponytail
    pub short_sleeve: bool,
    pub shorts: bool,
    pub pack: bool,
    pub bag: bool,
}

fn pick(list: &[[f32; 3]], r: f32) -> Color {
    let c = list[((r * list.len() as f32) as usize).min(list.len() - 1)];
    Color::new(c[0], c[1], c[2], 1.0)
}

impl Target {
    fn new(x: f32, z: f32, k: usize, enemy: bool) -> Self {
        use crate::tex::hash;
        let h = |n: u32| hash(k as i32 * 7 + 3, n as i32, 4141 + n);
        let skins = [[0.93, 0.74, 0.58], [0.84, 0.60, 0.40], [0.72, 0.50, 0.32], [0.58, 0.38, 0.24], [0.96, 0.80, 0.66]];
        let hairs = [[0.07, 0.06, 0.06], [0.16, 0.10, 0.07], [0.30, 0.18, 0.10], [0.55, 0.55, 0.55], [0.40, 0.14, 0.08]];
        let shirts = [
            [0.78, 0.20, 0.17],
            [0.20, 0.40, 0.78],
            [0.92, 0.78, 0.20],
            [0.22, 0.62, 0.34],
            [0.93, 0.93, 0.90],
            [0.92, 0.50, 0.15],
            [0.50, 0.28, 0.70],
            [0.15, 0.58, 0.60],
            [0.90, 0.45, 0.60],
            [0.45, 0.47, 0.50],
        ];
        let pants = [[0.20, 0.28, 0.46], [0.10, 0.10, 0.12], [0.58, 0.50, 0.36], [0.28, 0.29, 0.32], [0.30, 0.34, 0.20]];
        let shoes = [[0.08, 0.08, 0.09], [0.38, 0.22, 0.12], [0.92, 0.92, 0.90], [0.70, 0.15, 0.12], [0.55, 0.36, 0.20]];
        let team = if enemy { Team::Red } else { Team::Blue };
        Self {
            team,
            hp: 100,
            fire_cd: 0.4 + h(16) * 1.4,
            muzzle_flash: 0.0,
            x,
            z,
            vx: 0.0,
            vz: 0.0,
            x0: x,
            z0: z,
            hd: h(1) * TAU,
            timer: h(2) * 2.0,
            idle: 0.0,
            spd: 0.9 + h(3) * 0.9,
            yaw: h(1) * TAU,
            alive: true,
            respawn: 0.0,
            walk: h(4) * 20.0,
            skin: pick(&skins, h(5)),
            hair: pick(&hairs, h(6)),
            shirt: if enemy { Color::new(0.72, 0.12, 0.10, 1.0) } else { Color::new(0.10, 0.34, 0.78, 1.0) },
            pants: pick(&pants, h(8)),
            shoes: pick(&shoes, h(9)),
            accent: pick(&shirts, h(10)),
            hair_style: (h(11) * 3.999) as u8,
            short_sleeve: h(12) > 0.4,
            shorts: h(13) > 0.78,
            pack: h(14) > 0.7,
            bag: h(15) > 0.8,
        }
    }
}

pub struct Prop {
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub y0: f32,
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
    pub along_x: bool,
    pub seed: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub respawn: f32,
    pub shake: f32,
    pub col: Color,
    pub col2: Color,
}

impl Prop {
    fn make(sp: &PropSpawn) -> Self {
        use crate::tex::hash;
        let h = |n: i32| hash(sp.seed as i32, n, 991);
        let c = |r: f32, g: f32, b: f32| Color::new(r, g, b, 1.0);
        let (hx, hy, hz, hp, col, col2) = match sp.kind {
            P_CRATE => {
                let k = 0.85 + h(1) * 0.3;
                (0.44 * k, 0.45, 0.44 * k, 3, c(0.62 + h(2) * 0.1, 0.45, 0.26), c(0.38, 0.26, 0.14))
            }
            P_BENCH => {
                if sp.along_x {
                    (0.8, 0.33, 0.22, 5, c(0.55, 0.38, 0.22), c(0.35, 0.36, 0.38))
                } else {
                    (0.22, 0.33, 0.8, 5, c(0.55, 0.38, 0.22), c(0.35, 0.36, 0.38))
                }
            }
            P_CONE => (0.2, 0.33, 0.2, 1, c(0.95, 0.42, 0.08), c(0.95, 0.95, 0.92)),
            P_BIN => (0.3, 0.45, 0.3, 2, c(0.20, 0.45 + h(3) * 0.2, 0.30), c(0.14, 0.30, 0.20)),
            _ => (0.38, 0.55, 0.38, 4, c(0.75 - h(4) * 0.5, 0.20 + h(4) * 0.3, 0.18 + h(4) * 0.45), c(0.15, 0.15, 0.17)),
        };
        Self {
            kind: sp.kind,
            x: sp.x,
            z: sp.z,
            y0: sp.y0,
            hx,
            hy,
            hz,
            along_x: sp.along_x,
            seed: sp.seed,
            hp,
            max_hp: hp,
            alive: true,
            respawn: 0.0,
            shake: 0.0,
            col,
            col2,
        }
    }

    // cones and bins are light: they do not block movement, they get knocked over
    pub fn solid(&self) -> bool {
        self.kind != P_CONE && self.kind != P_BIN
    }
}

pub struct PropSet {
    pub list: Vec<Prop>,
    grid: Vec<Vec<usize>>,
}

impl PropSet {
    fn new(spawns: &[PropSpawn]) -> Self {
        let list: Vec<Prop> = spawns.iter().map(Prop::make).collect();
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); GN * GN];
        for (i, p) in list.iter().enumerate() {
            let g0x = (((p.x - p.hx) / GC).floor().max(0.0) as usize).min(GN - 1);
            let g1x = (((p.x + p.hx) / GC).floor().max(0.0) as usize).min(GN - 1);
            let g0z = (((p.z - p.hz) / GC).floor().max(0.0) as usize).min(GN - 1);
            let g1z = (((p.z + p.hz) / GC).floor().max(0.0) as usize).min(GN - 1);
            for gz in g0z..=g1z {
                for gx in g0x..=g1x {
                    grid[gz * GN + gx].push(i);
                }
            }
        }
        Self { list, grid }
    }

    fn cell(&self, x: f32, z: f32) -> Option<&Vec<usize>> {
        if x < 0.0 || z < 0.0 || x >= SIZE || z >= SIZE {
            return None;
        }
        let gx = ((x / GC) as usize).min(GN - 1);
        let gz = ((z / GC) as usize).min(GN - 1);
        Some(&self.grid[gz * GN + gx])
    }
}

// combined ground height: static city + solid, unbroken props
fn hh(w: &World, ps: &PropSet, x: f32, z: f32) -> f32 {
    let mut h = w.height_at(x, z);
    if let Some(c) = ps.cell(x, z) {
        for &i in c {
            let p = &ps.list[i];
            if p.alive && p.solid() && (x - p.x).abs() <= p.hx && (z - p.z).abs() <= p.hz {
                h = h.max(p.y0 + p.hy * 2.0);
            }
        }
    }
    h
}

pub struct Debris {
    pub p: Vec3,
    pub v: Vec3,
    pub q: Quat,
    pub w: Vec3,
    pub half: Vec3,
    pub col: Color,
    pub life: f32,
    pub casing: bool,
    pub bounces: u8,
}

pub struct Part {
    pub p: Vec3,
    pub v: Vec3,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub col: Color,
    pub grav: f32,
}

pub struct Game {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub ys: f32,
    pub vel: Vec2,
    pub vy: f32,
    pub grounded: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub sway: Vec2,
    pub zoom: f32,
    pub boost: f32,
    pub mag: f32,
    pub ammo: i32,
    pub health: i32,
    pub weapon: Weapon,
    pub reserve: i32,
    pub reloading: f32,
    pub fire_cd: f32,
    pub scoped: bool,
    pub sneak_on: bool,
    pub sneaking: bool,
    pub sprint: bool,
    pub sliding: bool,
    pub slide_t: f32,
    pub slide_dur: f32,
    pub slide_cd: f32,
    pub slide_dir: Vec2,
    pub slide_spd: f32,
    pub slide_a: f32,
    pub walk: f32,
    pub moving: f32,
    pub step_acc: f32,
    pub land_dip: f32,
    pub shake: f32,
    pub flash: f32,
    pub kills: u32,
    pub heads: u32,
    pub targets: Vec<Target>,
    pub props: PropSet,
    pub debris: Vec<Debris>,
    pub parts: Vec<Part>,
    pub tracer: Option<(Vec3, Vec3)>,
    pub tracer_t: f32,
    pub hit_t: f32,
    pub head_t: f32,
    pub range: f32,
    pub t: f32,
    pub ev: Vec<(Snd, f32)>,
    rng: u32,
}

fn samples(x: f32, z: f32) -> [(f32, f32); 9] {
    let r = PR;
    let d = r * 0.7071;
    [
        (x, z),
        (x + r, z),
        (x - r, z),
        (x, z + r),
        (x, z - r),
        (x + d, z + d),
        (x - d, z + d),
        (x + d, z - d),
        (x - d, z - d),
    ]
}

impl Game {
    pub fn new(w: &World) -> Self {
        let targets = w.spawns.iter().enumerate().map(|(k, s)| Target::new(s.0, s.1, k, s.2)).collect();
        Self {
            x: SIZE * 0.5,
            y: 0.0,
            z: 6.0,
            ys: 0.0,
            vel: Vec2::ZERO,
            vy: 0.0,
            grounded: true,
            yaw: std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            sway: Vec2::ZERO,
            zoom: 0.0,
            boost: 0.0,
            mag: 4.0,
            ammo: MAG,
            health: 100,
            weapon: Weapon::Rifle,
            reserve: 120,
            reloading: 0.0,
            fire_cd: 0.0,
            scoped: false,
            sneak_on: false,
            sneaking: false,
            sprint: false,
            sliding: false,
            slide_t: 0.0,
            slide_dur: 0.9,
            slide_cd: 0.0,
            slide_dir: Vec2::ZERO,
            slide_spd: 0.0,
            slide_a: 0.0,
            walk: 0.0,
            moving: 0.0,
            step_acc: 0.0,
            land_dip: 0.0,
            shake: 0.0,
            flash: 0.0,
            kills: 0,
            heads: 0,
            targets,
            props: PropSet::new(&w.props),
            debris: Vec::new(),
            parts: Vec::new(),
            tracer: None,
            tracer_t: 0.0,
            hit_t: 0.0,
            head_t: 0.0,
            range: 0.0,
            t: 0.0,
            ev: Vec::new(),
            rng: 0x1234_5678,
        }
    }

    fn rf(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xFFFFFF) as f32 / 16777215.0
    }

    pub fn k(&self) -> f32 {
        let kb = if self.sneaking { 0.78 } else { 1.0 };
        kb + (0.5 - kb) * self.slide_a
    }

    pub fn feet(&self) -> Vec3 {
        vec3(self.x, self.y, self.z)
    }

    fn blocked(&self, w: &World, x: f32, z: f32) -> bool {
        // Invisible collision planes keep the player inside the compact arena.
        if x < PR || z < PR || x > SIZE - PR || z > SIZE - PR {
            return true;
        }
        samples(x, z).iter().any(|(sx, sz)| hh(w, &self.props, *sx, *sz) > self.y + STEP)
    }

    fn ground(&self, w: &World, ref_y: f32) -> f32 {
        let mut best = 0.0f32;
        for (sx, sz) in samples(self.x, self.z) {
            let h = hh(w, &self.props, sx, sz);
            if h <= ref_y + STEP && h > best {
                best = h;
            }
        }
        best
    }

    pub fn ground_here(&self, w: &World) -> f32 {
        self.ground(w, self.y)
    }

    pub fn aim_dir(&self) -> Vec3 {
        let yw = self.yaw + self.sway.x;
        let pt = self.pitch + self.sway.y;
        let cp = pt.cos();
        vec3(cp * yw.cos(), pt.sin(), cp * yw.sin())
    }

    pub fn right(&self) -> Vec3 {
        vec3(-self.yaw.sin(), 0.0, self.yaw.cos())
    }

    pub fn eye(&self) -> Vec3 {
        vec3(self.x, self.ys + (0.93 * self.k() + 0.06) * S, self.z)
    }

    // gun position in model space (before the S scale about the feet)
    pub fn gun_model(&self) -> Vec3 {
        let f = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        vec3(self.x, self.y + 0.56 * self.k(), self.z) + f * 0.38 + self.right() * 0.20
    }

    pub fn to_world(&self, p: Vec3) -> Vec3 {
        let o = self.feet();
        o + (p - o) * S
    }

    pub fn cam_pos(&self, w: &World) -> Vec3 {
        let d = self.aim_dir();
        let r = self.right();
        let z = self.zoom;
        let hb = if self.sneaking { 0.62 } else { 0.78 };
        let hgt = hb + (0.40 - hb) * self.slide_a;
        let third = vec3(self.x, self.ys + hgt * S, self.z);
        let pivot = third + (self.eye() - third) * z;
        let dist = 4.0 * (1.0 - z);
        let shoulder = 0.65 * (1.0 - z);
        let want = pivot - d * dist + r * shoulder + vec3(0.0, 0.3 * (1.0 - z), 0.0);
        let mut best = want;
        let steps = 28;
        for i in 1..=steps {
            let f = i as f32 / steps as f32;
            let p = pivot + (want - pivot) * f;
            if hh(w, &self.props, p.x, p.z) > p.y - 0.15 {
                let back = (i - 1) as f32 / steps as f32;
                best = pivot + (want - pivot) * back;
                break;
            }
        }
        best.y = (best.y - self.land_dip).max(0.3);
        if self.shake > 0.0 {
            best.y += (self.t * 90.0).sin() * self.shake * 0.05;
            best.x += (self.t * 77.0).cos() * self.shake * 0.03;
        }
        best
    }

    fn start_reload(&mut self) {
        if self.reloading <= 0.0 && self.ammo < MAG && self.reserve > 0 {
            self.reloading = 1.3;
            self.ev.push((Snd::Reload, 0.8));
        }
    }

    // March a ray; returns (distance, target index, headshot)
    pub fn raycast(&self, w: &World, o: Vec3, d: Vec3) -> (f32, Option<(usize, bool)>) {
        let mut t = 0.2f32;
        while t < MAX_DIST {
            t += 0.1;
            let p = o + d * t;
            if p.y < 0.0 || w.height_at(p.x, p.z) > p.y {
                return (t, None);
            }
            if p.y < 1.95 {
                for (i, tg) in self.targets.iter().enumerate() {
                    if !tg.alive || tg.team == Team::Blue {
                        continue;
                    }
                    let dx = p.x - tg.x;
                    let dz = p.z - tg.z;
                    let r2 = if p.y > 1.45 { 0.06 } else { 0.18 };
                    if dx * dx + dz * dz < r2 {
                        return (t, Some((i, p.y > 1.45)));
                    }
                }
            }
        }
        (MAX_DIST, None)
    }

    fn burst(&mut self, p: Vec3, n: Vec3, count: usize, col: Color, speed: f32, size: f32) {
        for _ in 0..count {
            let v = (n + vec3(self.rf() - 0.5, self.rf() - 0.5, self.rf() - 0.5) * 1.6) * speed * (0.4 + self.rf());
            let life = 0.3 + self.rf() * 0.5;
            let sz = size * (0.6 + self.rf() * 0.8);
            self.parts.push(Part { p, v, life, max: life, size: sz, col, grav: 9.0 });
        }
        if self.parts.len() > 400 {
            let n = self.parts.len() - 400;
            self.parts.drain(0..n);
        }
    }

    fn kill_target(&mut self, i: usize, dir: Vec3, head: bool) {
        let (tx, tz, yaw) = (self.targets[i].x, self.targets[i].z, self.targets[i].yaw);
        let (skin, hair, shirt, pants, shoes) = {
            let t = &self.targets[i];
            (t.skin, t.hair, t.shirt, t.pants, t.shoes)
        };
        let sleeve = if self.targets[i].short_sleeve { skin } else { shirt };
        let leg2 = if self.targets[i].shorts { skin } else { pants };
        self.targets[i].alive = false;
        self.targets[i].respawn = 6.0;
        let r = vec3(-yaw.sin(), 0.0, yaw.cos());
        let f = vec3(yaw.cos(), 0.0, yaw.sin());
        // (offset, half extents, colour) in world units around the body
        let pieces: [(Vec3, Vec3, Color); 11] = [
            (r * 0.10 + vec3(0.0, 0.62, 0.0), vec3(0.09, 0.22, 0.09), pants),
            (r * -0.10 + vec3(0.0, 0.62, 0.0), vec3(0.09, 0.22, 0.09), pants),
            (r * 0.10 + vec3(0.0, 0.16, 0.0), vec3(0.10, 0.14, 0.09), leg2),
            (r * -0.10 + vec3(0.0, 0.16, 0.0), vec3(0.10, 0.14, 0.09), leg2),
            (vec3(0.0, 1.05, 0.0), vec3(0.14, 0.32, 0.29), shirt),
            (vec3(0.0, 1.50, 0.0), vec3(0.14, 0.14, 0.14), skin),
            (vec3(0.0, 1.66, 0.0), vec3(0.16, 0.06, 0.16), hair),
            (r * 0.40 + vec3(0.0, 1.08, 0.0), vec3(0.08, 0.28, 0.08), sleeve),
            (r * -0.40 + vec3(0.0, 1.08, 0.0), vec3(0.08, 0.28, 0.08), sleeve),
            (r * 0.10 + f * 0.04 + vec3(0.0, 0.05, 0.0), vec3(0.14, 0.06, 0.09), shoes),
            (r * -0.10 + f * 0.04 + vec3(0.0, 0.05, 0.0), vec3(0.14, 0.06, 0.09), shoes),
        ];
        let vx = self.targets[i].vx;
        let vz = self.targets[i].vz;
        for (off, half, col) in pieces {
            let kick = 3.5 + self.rf() * 4.0;
            let v = dir * kick + vec3(vx, 2.0 + self.rf() * 3.0, vz) + (r * (self.rf() - 0.5) + f * (self.rf() - 0.5)) * 2.5;
            let w = vec3(self.rf() - 0.5, self.rf() - 0.5, self.rf() - 0.5) * 16.0;
            self.debris.push(Debris {
                p: vec3(tx, 0.0, tz) + off,
                v,
                q: Quat::from_rotation_y(-yaw),
                w,
                half,
                col,
                life: 9.0,
                casing: false,
                bounces: 0,
            });
        }
        if self.debris.len() > 260 {
            let n = self.debris.len() - 260;
            self.debris.drain(0..n);
        }
        self.kills += 1;
        self.hit_t = 0.3;
        if head {
            self.heads += 1;
            self.head_t = 1.2;
            self.ev.push((Snd::Ding, 0.7));
        }
        self.ev.push((Snd::Hit, 0.8));
        let p = vec3(tx, 1.2, tz);
        self.burst(p, vec3(0.0, 1.0, 0.0) - dir * 0.3, 14, Color::new(0.9, 0.2, 0.15, 1.0), 3.0, 0.07);
    }

    // first smashable prop along a ray (within tmax)
    fn raycast_prop(&self, o: Vec3, d: Vec3, tmax: f32) -> Option<(f32, usize)> {
        let mut t = 0.2f32;
        while t < tmax {
            t += 0.08;
            let p = o + d * t;
            if let Some(c) = self.props.cell(p.x, p.z) {
                for &i in c {
                    let pr = &self.props.list[i];
                    if pr.alive
                        && (p.x - pr.x).abs() <= pr.hx
                        && (p.z - pr.z).abs() <= pr.hz
                        && p.y >= pr.y0
                        && p.y <= pr.y0 + pr.hy * 2.0
                    {
                        return Some((t, i));
                    }
                }
            }
        }
        None
    }

    fn debris_piece(&mut self, p: Vec3, half: Vec3, col: Color, v: Vec3) {
        let w = vec3(self.rf() - 0.5, self.rf() - 0.5, self.rf() - 0.5) * 18.0;
        let q = Quat::from_rotation_y(self.rf() * TAU);
        self.debris.push(Debris { p, v, q, w, half, col, life: 14.0, casing: false, bounces: 0 });
    }

    fn hit_prop(&mut self, i: usize, dir: Vec3, at: Vec3) {
        self.props.list[i].hp -= 1;
        if self.props.list[i].hp <= 0 {
            self.break_prop(i, dir);
            return;
        }
        let (kind, col) = (self.props.list[i].kind, self.props.list[i].col);
        self.props.list[i].shake = 0.3;
        self.burst(at, -dir + vec3(0.0, 0.5, 0.0), 6, col, 2.5, 0.05);
        let snd = if kind == P_BARREL || kind == P_BIN { Snd::Clang } else { Snd::Impact };
        self.ev.push((snd, 0.5));
    }

    fn break_prop(&mut self, i: usize, dir: Vec3) {
        let (kind, x, z, y0, hx, hy, hz, ax, col, col2) = {
            let p = &self.props.list[i];
            (p.kind, p.x, p.z, p.y0, p.hx, p.hy, p.hz, p.along_x, p.col, p.col2)
        };
        self.props.list[i].alive = false;
        self.props.list[i].respawn = 25.0;
        let c = vec3(x, y0 + hy, z);
        let dark = |c: Color, k: f32| Color::new(c.r * k, c.g * k, c.b * k, 1.0);
        let mut pieces: Vec<(Vec3, Vec3, Color)> = Vec::new(); // offset, half, colour
        match kind {
            P_CRATE => {
                for k in 0..9 {
                    let off = vec3((self.rf() - 0.5) * hx * 1.5, (self.rf() - 0.5) * hy * 1.7, (self.rf() - 0.5) * hz * 1.5);
                    let half = if k % 2 == 0 { vec3(hx * 0.9, 0.05, 0.065) } else { vec3(0.065, 0.05, hz * 0.9) };
                    let shade = 0.8 + self.rf() * 0.35;
                    pieces.push((off, half, dark(col, shade)));
                }
                for sx in [-1.0f32, 1.0] {
                    pieces.push((vec3(sx * hx * 0.8, 0.0, 0.0), vec3(0.06, hy * 0.9, 0.06), col2));
                }
            }
            P_BENCH => {
                for k in 0..3 {
                    let o = (k as f32 - 1.0) * 0.14;
                    let (off, half) = if ax {
                        (vec3(0.0, 0.0, o), vec3(hx * 0.95, 0.035, 0.055))
                    } else {
                        (vec3(o, 0.0, 0.0), vec3(0.055, 0.035, hz * 0.95))
                    };
                    pieces.push((off, half, dark(col, 0.85 + 0.1 * k as f32)));
                }
                let (lo, lh) = if ax { (vec3(hx * 0.8, -0.15, 0.0), vec3(0.05, 0.15, hz * 0.9)) } else { (vec3(0.0, -0.15, hz * 0.8), vec3(hx * 0.9, 0.15, 0.05)) };
                pieces.push((lo, lh, col2));
                pieces.push((-lo * vec3(1.0, 0.0, 1.0) + vec3(0.0, -0.15, 0.0), lh, col2));
                let (bo, bh) = if ax { (vec3(0.0, 0.2, -hz * 0.85), vec3(hx * 0.95, 0.05, 0.02)) } else { (vec3(-hx * 0.85, 0.2, 0.0), vec3(0.02, 0.05, hz * 0.95)) };
                pieces.push((bo, bh, dark(col, 0.9)));
            }
            P_CONE => {
                pieces.push((vec3(0.0, -hy + 0.04, 0.0), vec3(0.2, 0.03, 0.2), dark(col, 0.8)));
                pieces.push((vec3(0.0, -0.1, 0.0), vec3(0.14, 0.16, 0.14), col));
                pieces.push((vec3(0.0, 0.1, 0.0), vec3(0.11, 0.05, 0.11), col2));
                pieces.push((vec3(0.0, 0.24, 0.0), vec3(0.07, 0.1, 0.07), col));
            }
            P_BIN => {
                pieces.push((vec3(0.0, -0.05, 0.0), vec3(hx * 0.9, hy * 0.85, hz * 0.9), col));
                pieces.push((vec3(0.0, hy * 0.95, 0.0), vec3(hx * 1.05, 0.04, hz * 1.05), col2));
                for _ in 0..6 {
                    let off = vec3((self.rf() - 0.5) * 0.3, 0.1 + self.rf() * 0.3, (self.rf() - 0.5) * 0.3);
                    let tc = Color::new(0.4 + self.rf() * 0.5, 0.4 + self.rf() * 0.5, 0.4 + self.rf() * 0.5, 1.0);
                    pieces.push((off, vec3(0.05, 0.04, 0.05), tc));
                }
            }
            _ => {
                pieces.push((vec3(0.0, -hy * 0.5, 0.0), vec3(hx * 0.92, hy * 0.5, hz * 0.92), col));
                pieces.push((vec3(0.0, hy * 0.5, 0.0), vec3(hx * 0.92, hy * 0.5, hz * 0.92), col));
                pieces.push((vec3(0.0, 0.0, 0.0), vec3(hx * 1.0, 0.03, hz * 1.0), col2));
                pieces.push((vec3(0.0, hy * 0.95, 0.0), vec3(hx * 0.7, 0.02, hz * 0.7), col2));
            }
        }
        let strong = if kind == P_CONE { 1.6 } else { 1.0 };
        for (off, half, pc) in pieces {
            let kick = (2.5 + self.rf() * 3.5) * strong;
            let v = dir * kick + vec3((self.rf() - 0.5) * 3.0, 2.0 + self.rf() * 3.0, (self.rf() - 0.5) * 3.0);
            self.debris_piece(c + off, half, pc, v);
        }
        if self.debris.len() > 320 {
            let n = self.debris.len() - 320;
            self.debris.drain(0..n);
        }
        let (snd, v) = match kind {
            P_BARREL | P_BIN => (Snd::Clang, 0.8),
            P_CONE => (Snd::Impact, 0.6),
            _ => (Snd::Smash, 0.9),
        };
        self.ev.push((snd, v));
        self.burst(c, vec3(0.0, 1.0, 0.0) + dir * 0.3, 12, dark(col, 1.1), 3.5, 0.06);
        self.burst(c, vec3(0.0, 0.6, 0.0), 6, Color::new(0.75, 0.72, 0.66, 1.0), 1.5, 0.14);
    }

    fn update_props(&mut self, dt: f32) {
        let pp = vec2(self.x, self.z);
        let vel = self.vel;
        let sp = vel.length();
        let dir = if sp > 0.1 { vec3(vel.x / sp, 0.0, vel.y / sp) } else { vec3(0.0, 0.0, 0.0) };
        let mut knock: Vec<usize> = Vec::new();
        for (i, p) in self.props.list.iter_mut().enumerate() {
            p.shake = (p.shake - dt).max(0.0);
            if !p.alive {
                p.respawn -= dt;
                if p.respawn <= 0.0 && vec2(p.x, p.z).distance(pp) > 6.0 {
                    p.alive = true;
                    p.hp = p.max_hp;
                }
                continue;
            }
            // walking or sliding into light props knocks them over
            if !p.solid() && sp > 2.3 && self.y < p.y0 + p.hy * 2.0 {
                let d = vec2(p.x, p.z).distance(pp);
                if d < PR + p.hx.max(p.hz) + 0.05 {
                    knock.push(i);
                }
            }
        }
        for i in knock {
            self.break_prop(i, dir + vec3(0.0, 0.2, 0.0));
        }
    }

    fn shoot(&mut self, w: &World) {
        if self.fire_cd > 0.0 || self.reloading > 0.0 {
            return;
        }
        if self.weapon == Weapon::Knife {
            self.fire_cd = 0.42;
            self.ev.push((Snd::Clang, 0.25));
            let f = vec2(self.yaw.cos(), self.yaw.sin());
            let mut best: Option<(usize, f32)> = None;
            for (idx, bot) in self.targets.iter().enumerate() {
                if !bot.alive || bot.team != Team::Red { continue; }
                let delta = vec2(bot.x - self.x, bot.z - self.z);
                let dist = delta.length();
                if dist <= 2.2 && dist > 0.01 && delta.normalize().dot(f) > 0.25 && best.map_or(true, |(_, d)| dist < d) {
                    best = Some((idx, dist));
                }
            }
            if let Some((idx, _)) = best { self.kill_target(idx, vec3(f.x, 0.0, f.y), false); }
            return;
        }
        if self.ammo <= 0 {
            self.fire_cd = 0.3;
            if self.reserve > 0 {
                self.start_reload();
            } else {
                self.reserve = 120; // street resupply
                self.ev.push((Snd::Empty, 0.8));
            }
            return;
        }
        self.ammo -= 1;
        self.fire_cd = 0.14;
        self.ev.push((Snd::Shot, 1.0));

        let o = self.cam_pos(w);
        let d = self.aim_dir();
        let (mut dist, mut hit) = self.raycast(w, o, d);
        let mut prop: Option<usize> = None;
        if let Some((pt, pi)) = self.raycast_prop(o, d, dist) {
            if pt < dist {
                dist = pt;
                hit = None;
                prop = Some(pi);
            }
        }
        let end = o + d * dist;
        if let Some(pi) = prop {
            self.hit_prop(pi, d, end);
        } else if let Some((i, head)) = hit {
            self.kill_target(i, d, head);
        } else if dist < MAX_DIST {
            let back = -d;
            self.burst(end, back + vec3(0.0, 0.6, 0.0), 9, Color::new(1.0, 0.85, 0.4, 1.0), 4.0, 0.045);
            self.burst(end, back * 0.4 + vec3(0.0, 0.5, 0.0), 6, Color::new(0.75, 0.72, 0.66, 1.0), 1.2, 0.12);
            self.ev.push((Snd::Impact, (0.35 * (1.0 - dist / MAX_DIST)).max(0.08)));
        }
        let muzzle = self.to_world(self.gun_model() + d * 0.46);
        self.tracer = Some((muzzle, end));
        self.tracer_t = 0.06;
        self.flash = 0.05;

        // recoil kick + shell casing
        let k = 1.0 - self.zoom * 0.5;
        self.pitch += (0.010 + self.rf() * 0.005) * k;
        self.yaw += (self.rf() - 0.5) * 0.007 * k;
        self.shake = 0.12;
        let r = self.right();
        let up = vec3(0.0, 1.0, 0.0);
        let cp = self.to_world(self.gun_model());
        let v = r * (1.8 + self.rf()) + up * (2.2 + self.rf()) - d * 0.6;
        let wv = vec3(self.rf() - 0.5, self.rf() - 0.5, self.rf() - 0.5) * 30.0;
        self.debris.push(Debris {
            p: cp,
            v,
            q: Quat::IDENTITY,
            w: wv,
            half: vec3(0.025, 0.012, 0.012),
            col: Color::new(0.95, 0.75, 0.25, 1.0),
            life: 5.0,
            casing: true,
            bounces: 0,
        });
    }

    pub fn update(&mut self, w: &World, i: &Input, dt: f32) {
        self.t += dt;
        if i.switch_weapon {
            self.weapon = if self.weapon == Weapon::Rifle { Weapon::Knife } else { Weapon::Rifle };
            if self.weapon == Weapon::Knife { self.scoped = false; self.zoom = 0.0; }
            self.ev.push((Snd::Ui, 0.65));
        }
        if i.scope_tap {
            if self.weapon == Weapon::Rifle { self.scoped = !self.scoped; }
            self.ev.push((Snd::Scope, 0.5));
        }
        if i.zoom_step != 0 {
            self.mag = (self.mag + i.zoom_step as f32 * 1.5).clamp(2.0, 9.0);
            self.ev.push((Snd::Ui, 0.4));
        }
        let mut mv = i.mv;
        if mv.length() > 1.0 {
            mv = mv.normalize();
        }
        let forward_ok = mv.y > 0.3;
        self.sneaking = self.sneak_on || i.ctrl;
        self.sprint = (i.joy_mag >= SPRINT_TILT || i.kb_sprint) && forward_ok && !self.sneaking && !self.scoped && !self.sliding;

        let f = vec2(self.yaw.cos(), self.yaw.sin());
        let r = vec2(-self.yaw.sin(), self.yaw.cos());
        let wish = f * mv.y + r * mv.x;

        // one button: Slide while running forward, Snk otherwise
        let slide_label = self.sprint || self.sliding;
        if i.snk_tap {
            if slide_label {
                let dir = wish.normalize_or_zero();
                // sliding backward is not allowed: only forward-ish directions
                if self.grounded && !self.sliding && self.slide_cd <= 0.0 && forward_ok && dir.dot(f) > 0.55 {
                    self.sliding = true;
                    self.slide_t = 0.0;
                    self.slide_dir = dir;
                    self.slide_spd = self.vel.length().max(SPRINT) * 1.25;
                    self.slide_dur = if i.slide_hold { 2.6 } else { 1.0 };
                    self.ev.push((Snd::Slide, 0.7));
                }
            } else {
                self.sneak_on = !self.sneak_on;
                self.sneaking = self.sneak_on;
            }
        }
        if self.sliding {
            self.slide_t += dt;
            // smooth exponential glide, with gentle steering toward the stick
            let k = if i.slide_hold { 0.5 } else { 1.15 };
            self.slide_spd = (self.slide_spd * (-k * dt).exp() - 0.8 * dt).max(0.0);
            if mv.length() > 0.3 {
                let wd = wish.normalize_or_zero();
                if wd.dot(self.slide_dir) > -0.2 {
                    self.slide_dir = (self.slide_dir + wd * dt * 2.2).normalize_or_zero();
                }
            }
            let released = i.slide_hold && !i.snk_held && self.slide_t > 0.2;
            // turning the camera away from the slide, or pushing the stick against it, cancels instantly
            let turned_back = self.slide_dir.dot(f) < 0.3;
            let pulled_back = mv.length() > 0.3 && wish.normalize_or_zero().dot(self.slide_dir) < -0.2;
            if self.slide_t >= self.slide_dur || self.slide_spd < 2.3 || released || i.jump || turned_back || pulled_back {
                if turned_back || pulled_back {
                    self.vel = self.slide_dir * self.slide_spd * 0.35; // drop out of the slide, don't keep gliding
                } else {
                    self.vel = self.slide_dir * self.slide_spd;
                }
                self.sliding = false;
                self.slide_cd = 0.4;
            }
        }
        self.slide_cd = (self.slide_cd - dt).max(0.0);

        if i.reload {
            self.start_reload();
        }
        if i.jump && self.grounded {
            self.vy = JUMP_V;
            self.grounded = false;
            self.ev.push((Snd::Jump, 0.5));
        }
        if i.fire {
            self.shoot(w);
        }

        // look
        let sens = 1.0 - self.zoom + self.zoom / (self.mag * 0.55);
        self.yaw = (self.yaw + i.look.x * 0.0050 * sens).rem_euclid(TAU);
        self.pitch = (self.pitch - i.look.y * 0.0035 * sens).clamp(-0.7, 1.1);

        // ---- horizontal physics: momentum, acceleration, friction
        let mut speed = if self.sneaking {
            SNEAK
        } else if self.sprint {
            SPRINT
        } else {
            WALK
        };
        if mv.y < -0.1 {
            speed *= 0.65; // backpedal is slower
        }
        if self.scoped {
            speed *= 0.6;
        }
        if self.sliding {
            self.vel = self.slide_dir * self.slide_spd;
        } else {
            let target = if self.sprint { wish.normalize_or_zero() * speed } else { wish * speed };
            let acc = if self.grounded {
                if mv.length() > 0.1 {
                    26.0
                } else {
                    32.0
                }
            } else {
                4.0
            };
            let dv = target - self.vel;
            let dl = dv.length();
            let step = acc * dt;
            if dl <= step {
                self.vel = target;
            } else {
                self.vel += dv / dl * step;
            }
        }
        let dx = self.vel.x * dt;
        let dz = self.vel.y * dt;
        if !self.blocked(w, self.x + dx, self.z) {
            self.x += dx;
        } else {
            self.vel.x = 0.0;
            if self.sliding {
                self.slide_spd *= 0.4;
            }
        }
        if !self.blocked(w, self.x, self.z + dz) {
            self.z += dz;
        } else {
            self.vel.y = 0.0;
            if self.sliding {
                self.slide_spd *= 0.4;
            }
        }
        let sp = self.vel.length();
        self.moving = if self.sliding || !self.grounded { 0.0 } else { (sp / WALK).min(1.6) };
        self.walk += sp * dt * 2.2;
        if self.grounded && !self.sliding && sp > 1.0 {
            self.step_acc += sp * dt;
            let stride = if self.sprint { 2.7 } else { 1.9 };
            if self.step_acc > stride {
                self.step_acc = 0.0;
                let v = if self.sneaking {
                    0.12
                } else if self.sprint {
                    0.55
                } else {
                    0.34
                };
                self.ev.push((Snd::Step, v));
            }
        }

        // ---- vertical physics
        let prev_y = self.y;
        self.vy -= GRAVITY * dt;
        self.y += self.vy * dt;
        let floor = self.ground(w, prev_y);
        if self.y <= floor {
            if self.vy < -5.0 {
                self.ev.push((Snd::Land, (-self.vy / 14.0).min(1.0)));
                self.land_dip = (-self.vy * 0.018).min(0.25);
                self.shake = self.shake.max(0.08);
            }
            self.y = floor;
            self.vy = 0.0;
        }
        self.grounded = self.y <= floor + 0.001;
        self.ys += (self.y - self.ys) * (dt * 16.0).min(1.0);
        self.land_dip = (self.land_dip - dt * 0.9).max(0.0);

        // ---- timers
        self.fire_cd = (self.fire_cd - dt).max(0.0);
        self.tracer_t = (self.tracer_t - dt).max(0.0);
        self.hit_t = (self.hit_t - dt).max(0.0);
        self.head_t = (self.head_t - dt).max(0.0);
        self.flash = (self.flash - dt).max(0.0);
        self.shake = (self.shake - dt * 0.8).max(0.0);
        if self.reloading > 0.0 {
            self.reloading -= dt;
            if self.reloading <= 0.0 {
                let n = (MAG - self.ammo).min(self.reserve);
                self.ammo += n;
                self.reserve -= n;
                self.reloading = 0.0;
            }
        }
        let zoom_goal = if self.scoped { 1.0 } else { 0.0 };
        self.zoom += (zoom_goal - self.zoom) * (dt * 10.0).min(1.0);
        let sa_goal = if self.sliding { 1.0 } else { 0.0 };
        self.slide_a += (sa_goal - self.slide_a) * (dt * 11.0).min(1.0);
        if self.sliding && self.slide_spd > 2.5 {
            let back = vec3(-self.slide_dir.x, 0.35, -self.slide_dir.y);
            let p = vec3(self.x, self.y + 0.08, self.z);
            if self.rf() < 0.7 {
                self.burst(p, back, 1, Color::new(0.72, 0.70, 0.64, 1.0), 1.4, 0.12);
            }
        }
        let boost_goal = if self.sprint || self.sliding { 1.0 } else { 0.0 };
        self.boost += (boost_goal - self.boost) * (dt * 8.0).min(1.0);

        // scope sway (breathing + movement), steadier when crouched
        let a = if self.zoom > 0.1 {
            0.0016 * (0.6 + self.mag * 0.12) * (if self.sneaking { 0.45 } else { 1.0 }) * (1.0 + sp * 0.25)
        } else {
            0.0
        };
        self.sway = vec2(
            ((self.t * 1.3).sin() + (self.t * 2.9).sin() * 0.4) * a,
            ((self.t * 1.7).cos() * 0.8 + (self.t * 3.3).sin() * 0.3) * a,
        );
        if self.zoom > 0.5 {
            let o = self.cam_pos(w);
            let (dist, _) = self.raycast(w, o, self.aim_dir());
            self.range = if dist >= MAX_DIST { 0.0 } else { dist };
        }

        self.update_props(dt);
        self.update_targets(w, dt);
        self.update_debris(w, dt);
        for p in &mut self.parts {
            p.life -= dt;
            p.v.y -= p.grav * dt;
            p.p += p.v * dt;
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    fn update_targets(&mut self, w: &World, dt: f32) {
        use std::f32::consts::PI;
        let player_pos = vec2(self.x, self.z);
        for i in 0..self.targets.len() {
            let (r1, r2, r3) = (self.rf(), self.rf(), self.rf());
            let (team, alive, pos, home, fire_cd) = {
                let t=&self.targets[i]; (t.team,t.alive,vec2(t.x,t.z),vec2(t.x0,t.z0),t.fire_cd)
            };
            if !alive {
                let t=&mut self.targets[i]; t.respawn-=dt;
                if t.respawn<=0.0 { t.alive=true; t.hp=100; t.x=t.x0; t.z=t.z0; t.vx=0.0; t.vz=0.0; t.hd=r1*TAU; t.timer=0.5+r2; t.idle=0.0; }
                continue;
            }
            let mut target: Option<(usize, Vec2, f32)> = None;
            if team == Team::Red {
                let d=player_pos.distance(pos); target=Some((usize::MAX,player_pos,d));
            } else {
                for (j, other) in self.targets.iter().enumerate() {
                    if other.alive && other.team==Team::Red {
                        let p=vec2(other.x,other.z); let d=p.distance(pos);
                        if target.map_or(true,|(_,_,best)| d<best) { target=Some((j,p,d)); }
                    }
                }
            }
            let mut fire_at: Option<(usize, Vec2, f32)> = None;
            let mut desired_heading=None;
            let mut facing_heading=None;
            if let Some((j,aim,distance))=target {
                let delta=aim-pos;
                let angle=delta.y.atan2(delta.x);
                facing_heading=Some(angle);
                desired_heading=Some(if distance>11.0 {angle} else {angle+PI*0.5*(if i%2==0{1.0}else{-1.0})});
                if distance<34.0 && fire_cd<=dt {
                    let from=vec3(pos.x,1.25,pos.y); let to=vec3(aim.x,1.0,aim.y);
                    let dist=(to-from).length(); let steps=(dist*4.0).ceil() as i32;
                    let visible=(1..steps).all(|step| {let p=from.lerp(to,step as f32/steps.max(1) as f32); w.height_at(p.x,p.z)<=p.y});
                    if visible { fire_at=Some((j,aim,distance)); }
                }
            }
            let mut hit_player=false;
            let mut hit_bot=None;
            {
            let t=&mut self.targets[i];
            t.fire_cd=(t.fire_cd-dt).max(0.0); t.timer-=dt; if t.idle>0.0 {t.idle-=dt;}
            if let Some(hd)=desired_heading {t.hd=hd; t.spd=if target.unwrap().2>11.0 {2.1} else {0.9};}
            if let Some(face)=facing_heading {t.yaw=face;}
            else if t.timer<=0.0 {t.hd+=(r2-0.5)*2.6; t.spd=0.8+r3*1.2; t.timer=1.2+r3*3.5;}
            let hx=home.x-t.x; let hz=home.y-t.z;
            if hx*hx+hz*hz>25.0*25.0 {t.hd=(hz.atan2(hx));}
            let (c,sn)=(t.hd.cos(),t.hd.sin());
            if hh(w,&self.props,t.x+c*0.9,t.z+sn*0.9)>0.3 {t.hd+=PI*(0.55+r2*0.9)*if r1<0.5{1.0}else{-1.0};t.timer=0.8+r3;}
            let go=if t.idle>0.0 {0.0}else{t.spd}; let gx=t.hd.cos()*go; let gz=t.hd.sin()*go;
            t.vx+=(gx-t.vx)*(dt*4.0).min(1.0); t.vz+=(gz-t.vz)*(dt*4.0).min(1.0);
            let nx=t.x+t.vx*dt; let nz=t.z+t.vz*dt;
            if nx>PR && nz>PR && nx<SIZE-PR && nz<SIZE-PR && hh(w,&self.props,nx,nz)<=0.3 {t.x=nx;t.z=nz;} else {t.vx=0.0;t.vz=0.0;t.hd+=PI*0.7;}
            let sp=(t.vx*t.vx+t.vz*t.vz).sqrt();t.walk+=sp*dt*2.6;
            if facing_heading.is_none() && sp>0.25 {t.yaw=t.vz.atan2(t.vx);}
            if let Some((j,aim,_))=fire_at { t.fire_cd=0.8+r1*0.6; t.muzzle_flash=0.08; t.yaw=(aim.y-t.z).atan2(aim.x-t.x); if j==usize::MAX {hit_player=true;} else {hit_bot=Some(j);} }
            t.muzzle_flash=(t.muzzle_flash-dt).max(0.0);
            }
            if hit_player { self.health=(self.health-6).max(0); self.hit_t=0.08; }
            if let Some(j)=hit_bot { let other=&mut self.targets[j]; other.hp-=14; if other.hp<=0 { other.alive=false; other.respawn=6.0; } }
        }
        if self.health<=0 {self.health=100;self.x=SIZE*0.5;self.z=6.0;self.vel=Vec2::ZERO;}
    }

    fn update_debris(&mut self, w: &World, dt: f32) {
        let mut evs: Vec<(Snd, f32)> = Vec::new();
        for d in &mut self.debris {
            d.life -= dt;
            d.v.y -= 16.0 * dt;
            let nx = d.p.x + d.v.x * dt;
            let nz = d.p.z + d.v.z * dt;
            let rad = (d.half.min_element() * 1.5).max(0.03);
            if hh(w, &self.props, nx, d.p.z) > d.p.y - rad * 0.5 {
                d.v.x = -d.v.x * 0.4;
            }
            if hh(w, &self.props, d.p.x, nz) > d.p.y - rad * 0.5 {
                d.v.z = -d.v.z * 0.4;
            }
            d.p.x += d.v.x * dt;
            d.p.z += d.v.z * dt;
            d.p.y += d.v.y * dt;
            let fl = hh(w, &self.props, d.p.x, d.p.z);
            let fl = if fl > d.p.y + 0.6 { 0.0 } else { fl };
            if d.p.y - rad < fl {
                d.p.y = fl + rad;
                if d.v.y < -1.2 {
                    d.bounces = d.bounces.saturating_add(1);
                    if d.casing && d.bounces < 3 {
                        evs.push((Snd::Ding, 0.10));
                    }
                    d.v.y = -d.v.y * 0.38;
                } else {
                    d.v.y = 0.0;
                }
                let fr = (1.0 - 6.0 * dt).max(0.0);
                d.v.x *= fr;
                d.v.z *= fr;
                d.w *= fr;
            }
            let ang = d.w.length() * dt;
            if ang > 1e-5 {
                d.q = (Quat::from_axis_angle(d.w.normalize(), ang) * d.q).normalize();
            }
        }
        self.debris.retain(|d| d.life > 0.0);
        if evs.len() > 2 {
            evs.truncate(2);
        }
        self.ev.extend(evs);
    }
}
