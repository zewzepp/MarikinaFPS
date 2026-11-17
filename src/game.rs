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

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub mv: Vec2,
    pub look: Vec2,
    pub joy_mag: f32,
    pub kb_sprint: bool,
    pub ctrl: bool,
    pub jump: bool,
    pub fire: bool,
    pub reload: bool,
    pub scope_tap: bool,
    pub snk_tap: bool,
    pub snk_held: bool,
    pub slide_hold: bool,
    pub zoom_step: i32,
}

pub struct Target {
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub x0: f32,
    pub z0: f32,
    pub ax: bool,
    pub dir: f32,
    pub yaw: f32,
    pub alive: bool,
    pub respawn: f32,
    pub walk: f32,
}

impl Target {
    fn new(x: f32, z: f32, ax: bool, k: usize) -> Self {
        Self {
            x,
            z,
            vx: 0.0,
            vz: 0.0,
            x0: x,
            z0: z,
            ax,
            dir: if k % 2 == 0 { 1.0 } else { -1.0 },
            yaw: 0.0,
            alive: true,
            respawn: 0.0,
            walk: k as f32,
        }
    }
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
    pub walk: f32,
    pub moving: f32,
    pub step_acc: f32,
    pub land_dip: f32,
    pub shake: f32,
    pub flash: f32,
    pub kills: u32,
    pub heads: u32,
    pub targets: Vec<Target>,
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
        let targets = w.spawns.iter().enumerate().map(|(k, s)| Target::new(s.0, s.1, s.2, k)).collect();
        Self {
            x: SIZE * 0.5,
            y: 0.0,
            z: SIZE * 0.5 + 2.0,
            ys: 0.0,
            vel: Vec2::ZERO,
            vy: 0.0,
            grounded: true,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            sway: Vec2::ZERO,
            zoom: 0.0,
            boost: 0.0,
            mag: 4.0,
            ammo: MAG,
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
            walk: 0.0,
            moving: 0.0,
            step_acc: 0.0,
            land_dip: 0.0,
            shake: 0.0,
            flash: 0.0,
            kills: 0,
            heads: 0,
            targets,
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
        if self.sliding {
            0.5
        } else if self.sneaking {
            0.78
        } else {
            1.0
        }
    }

    pub fn feet(&self) -> Vec3 {
        vec3(self.x, self.y, self.z)
    }

    fn blocked(&self, w: &World, x: f32, z: f32) -> bool {
        samples(x, z).iter().any(|(sx, sz)| w.height_at(*sx, *sz) > self.y + STEP)
    }

    fn ground(&self, w: &World, ref_y: f32) -> f32 {
        let mut best = 0.0f32;
        for (sx, sz) in samples(self.x, self.z) {
            let h = w.height_at(sx, sz);
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
        let hgt = if self.sliding {
            0.40
        } else if self.sneaking {
            0.62
        } else {
            0.78
        };
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
            if w.height_at(p.x, p.z) > p.y - 0.15 {
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
                    if !tg.alive {
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
        self.targets[i].alive = false;
        self.targets[i].respawn = 6.0;
        let r = vec3(-yaw.sin(), 0.0, yaw.cos());
        let f = vec3(yaw.cos(), 0.0, yaw.sin());
        let red = Color::new(0.72, 0.18, 0.15, 1.0);
        let dred = Color::new(0.45, 0.10, 0.09, 1.0);
        let skin = Color::new(0.88, 0.62, 0.38, 1.0);
        let dark = Color::new(0.14, 0.15, 0.18, 1.0);
        let pieces: [(Vec3, Vec3, Color); 7] = [
            (r * 0.13 + vec3(0.0, 0.42, 0.0), vec3(0.10, 0.42, 0.10), dark),
            (r * -0.13 + vec3(0.0, 0.42, 0.0), vec3(0.10, 0.42, 0.10), dark),
            (vec3(0.0, 1.05, 0.0), vec3(0.20, 0.32, 0.30), red),
            (vec3(0.0, 1.55, 0.0), vec3(0.14, 0.14, 0.14), skin),
            (vec3(0.0, 1.70, 0.0), vec3(0.17, 0.07, 0.17), dred),
            (r * 0.42 + vec3(0.0, 1.05, 0.0), vec3(0.08, 0.28, 0.08), dred),
            (r * -0.42 + vec3(0.0, 1.05, 0.0), vec3(0.08, 0.28, 0.08), dred),
        ];
        let vx = self.targets[i].vx;
        let vz = self.targets[i].vz;
        for (off, half, col) in pieces {
            let kick = 3.5 + self.rf() * 4.0;
            let v = dir * kick + vec3(vx, 2.0 + self.rf() * 3.0, vz) + (r * (self.rf() - 0.5) + f * (self.rf() - 0.5)) * 2.0;
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
        if self.debris.len() > 220 {
            let n = self.debris.len() - 220;
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

    fn shoot(&mut self, w: &World) {
        if self.fire_cd > 0.0 || self.reloading > 0.0 {
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
        let (dist, hit) = self.raycast(w, o, d);
        let end = o + d * dist;
        if let Some((i, head)) = hit {
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
        if i.scope_tap {
            self.scoped = !self.scoped;
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
                if self.grounded && !self.sliding && self.slide_cd <= 0.0 && forward_ok && dir.dot(f) > 0.35 {
                    self.sliding = true;
                    self.slide_t = 0.0;
                    self.slide_dir = dir;
                    self.slide_spd = SPRINT * 1.3;
                    self.slide_dur = if i.slide_hold { 2.5 } else { 0.9 };
                    self.ev.push((Snd::Slide, 0.7));
                }
            } else {
                self.sneak_on = !self.sneak_on;
                self.sneaking = self.sneak_on;
            }
        }
        if self.sliding {
            self.slide_t += dt;
            let fr = if i.slide_hold { 2.4 } else { 5.0 };
            self.slide_spd -= (fr + self.slide_spd * 0.5) * dt;
            let released = i.slide_hold && !i.snk_held && self.slide_t > 0.2;
            let turned_back = self.slide_dir.dot(f) < -0.2;
            if self.slide_t >= self.slide_dur || self.slide_spd < 2.3 || released || i.jump || turned_back {
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
        for t in &mut self.targets {
            if !t.alive {
                t.respawn -= dt;
                if t.respawn <= 0.0 {
                    t.alive = true;
                    t.x = t.x0;
                    t.z = t.z0;
                    t.vx = 0.0;
                    t.vz = 0.0;
                }
                continue;
            }
            let (ux, uz) = if t.ax { (1.0, 0.0) } else { (0.0, 1.0) };
            let off = (t.x - t.x0) * ux + (t.z - t.z0) * uz;
            if off * t.dir > 4.0 {
                t.dir = -t.dir;
            }
            if w.height_at(t.x + ux * t.dir * 0.8, t.z + uz * t.dir * 0.8) > 0.3 {
                t.dir = -t.dir;
            }
            let gx = ux * t.dir * 1.3;
            let gz = uz * t.dir * 1.3;
            t.vx += (gx - t.vx) * (dt * 4.0).min(1.0);
            t.vz += (gz - t.vz) * (dt * 4.0).min(1.0);
            t.x += t.vx * dt;
            t.z += t.vz * dt;
            t.walk += (t.vx.abs() + t.vz.abs()) * dt * 2.4;
            let want = t.vz.atan2(t.vx);
            let mut dy = want - t.yaw;
            while dy > std::f32::consts::PI {
                dy -= TAU;
            }
            while dy < -std::f32::consts::PI {
                dy += TAU;
            }
            t.yaw += dy * (dt * 6.0).min(1.0);
        }
    }

    fn update_debris(&mut self, w: &World, dt: f32) {
        let mut evs: Vec<(Snd, f32)> = Vec::new();
        for d in &mut self.debris {
            d.life -= dt;
            d.v.y -= 16.0 * dt;
            let nx = d.p.x + d.v.x * dt;
            let nz = d.p.z + d.v.z * dt;
            let rad = d.half.max_element().max(0.03);
            if w.height_at(nx, d.p.z) > d.p.y - rad * 0.5 {
                d.v.x = -d.v.x * 0.4;
            }
            if w.height_at(d.p.x, nz) > d.p.y - rad * 0.5 {
                d.v.z = -d.v.z * 0.4;
            }
            d.p.x += d.v.x * dt;
            d.p.z += d.v.z * dt;
            d.p.y += d.v.y * dt;
            let fl = w.height_at(d.p.x, d.p.z);
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
