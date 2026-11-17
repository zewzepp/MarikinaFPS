use macroquad::prelude::*;

pub const REF_W: f32 = 2576.0;
pub const REF_H: f32 = 1450.0;
// (anchored to right edge, x, y, radius) in reference-picture pixels
pub const BASE: [(bool, f32, f32, f32); 6] = [
    (false, 315.0, 1148.0, 223.0), // joystick
    (true, 2093.0, 750.0, 90.0),   // scope
    (true, 2415.0, 750.0, 115.0),  // jump
    (true, 2175.0, 1067.0, 175.0), // fire
    (true, 1798.0, 1153.0, 79.0),  // reload
    (true, 2432.0, 1277.0, 94.0),  // sneak / slide
];
pub const NUDGE_PX: f32 = 6.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Elem {
    Joy = 0,
    Scope = 1,
    Jump = 2,
    Fire = 3,
    Reload = 4,
    Snk = 5,
}

pub const ELEMS: [Elem; 6] = [Elem::Joy, Elem::Scope, Elem::Jump, Elem::Fire, Elem::Reload, Elem::Snk];

impl Elem {
    pub fn name(self) -> &'static str {
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

// pos is the centre of the button as a fraction of the screen, so a layout
// survives rotation / different screen sizes.
#[derive(Clone, Copy)]
pub struct ElemCfg {
    pub pos: Vec2,
    pub scale: f32,
    pub opacity: f32,
}

pub fn default_pos(i: usize, w: f32, h: f32) -> Vec2 {
    let s = h / REF_H;
    let (right, x, y, _) = BASE[i];
    let cx = if right { w - (REF_W - x) * s } else { x * s };
    vec2(cx / w, y * s / h)
}

pub fn default_cfg(i: usize, w: f32, h: f32) -> ElemCfg {
    ElemCfg { pos: default_pos(i, w, h), scale: 1.0, opacity: 0.8 }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Play,
    Settings,
    Edit,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Joy,
    Look,
    Btn,
    Win,
    Drag(usize),
}

pub struct Pad {
    pub c: Vec2,
    pub r: f32,
    pub a: f32,
}

pub struct Layout {
    pub u: f32,
    pub pads: [Pad; 6],
    pub settings: Pad,
}

impl Layout {
    pub fn new(w: f32, h: f32, cfg: &[ElemCfg; 6]) -> Self {
        let u = (h / 720.0).clamp(0.75, 1.8);
        let s = h / REF_H;
        let mk = |i: usize| -> Pad {
            let c = cfg[i];
            Pad { c: vec2(c.pos.x * w, c.pos.y * h), r: BASE[i].3 * s * c.scale, a: c.opacity }
        };
        let st_r = (60.0 * s).max(34.0 * u);
        let st_x = (w - (REF_W - 2508.0) * s).min(w - st_r - 6.0 * u);
        let st_y = (65.0 * s).max(st_r + 6.0 * u);
        Self {
            u,
            pads: [mk(0), mk(1), mk(2), mk(3), mk(4), mk(5)],
            settings: Pad { c: vec2(st_x, st_y), r: st_r, a: 0.8 },
        }
    }

    // buttons only (not the joystick)
    pub fn hit(&self, p: Vec2) -> Option<Elem> {
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
    pub fn hit_any(&self, p: Vec2) -> Option<Elem> {
        if let Some(e) = self.hit(p) {
            return Some(e);
        }
        if p.distance(self.pads[0].c) <= self.pads[0].r {
            return Some(Elem::Joy);
        }
        None
    }

    pub fn settings_hit(&self, p: Vec2) -> bool {
        p.distance(self.settings.c) <= self.settings.r * 1.1
    }

    // Keep the floating joystick base fully on screen.
    pub fn clamp_origin(&self, p: Vec2, w: f32, h: f32) -> Vec2 {
        let m = self.pads[0].r + 10.0 * self.u;
        vec2(p.x.clamp(m, (w - m).max(m)), p.y.clamp(m, (h - m).max(m)))
    }

    // keep a dragged element's centre fully on screen
    pub fn clamp_pos(&self, i: usize, p: Vec2, w: f32, h: f32) -> Vec2 {
        let r = self.pads[i].r;
        vec2(p.x.clamp(r, (w - r).max(r)) / w, p.y.clamp(r, (h - r).max(r)) / h)
    }
}

// small fixed +/- pads shown beside the scope for changing magnification
pub fn zoom_pads(w: f32, h: f32, u: f32) -> [Pad; 2] {
    let r = 30.0 * u;
    let x = (w * 0.5 + h * 0.58).min(w - r - 8.0 * u);
    [
        Pad { c: vec2(x, h * 0.5 - r * 1.4), r, a: 0.8 },
        Pad { c: vec2(x, h * 0.5 + r * 1.4), r, a: 0.8 },
    ]
}

pub fn draw_center(text: &str, cx: f32, cy: f32, size: f32, col: Color) {
    let m = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - m.width / 2.0, cy + m.height / 2.0, size, col);
}

pub fn draw_pad(p: &Pad, label: &str, active: bool, u: f32) {
    let fill = if active {
        Color::new(0.23, 0.62, 0.78, 0.85 * p.a + 0.1)
    } else {
        Color::new(0.02, 0.03, 0.05, 0.85 * p.a)
    };
    draw_circle(p.c.x, p.c.y, p.r, fill);
    draw_circle_lines(p.c.x, p.c.y, p.r, 2.5 * u, Color::new(0.85, 0.94, 1.0, p.a));
    draw_circle_lines(p.c.x, p.c.y, p.r * 0.86, 1.0 * u, Color::new(0.85, 0.94, 1.0, p.a * 0.35));
    let size = (p.r * 0.36).max(11.0 * u);
    draw_center(label, p.c.x, p.c.y, size, Color::new(1.0, 1.0, 1.0, (p.a + 0.2).min(1.0)));
}

pub fn draw_joystick(home: &Pad, origin: Vec2, knob: Vec2, u: f32) {
    draw_circle(origin.x, origin.y, home.r, Color::new(0.02, 0.03, 0.05, 0.55 * home.a));
    draw_circle_lines(origin.x, origin.y, home.r, 2.5 * u, Color::new(0.85, 0.94, 1.0, home.a));
    draw_circle(knob.x, knob.y, home.r * 0.42, Color::new(0.8, 0.92, 1.0, 0.85 * home.a));
}

pub fn ui_button(r: Rect, label: &str, active: bool, u: f32) {
    let fill = if active {
        Color::new(0.23, 0.62, 0.78, 0.95)
    } else {
        Color::new(0.13, 0.18, 0.25, 0.95)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.8, 0.92, 1.0, 0.8));
    draw_center(label, r.x + r.w / 2.0, r.y + r.h / 2.0, 17.0 * u, WHITE);
}

pub fn win_size(screen: Screen, u: f32) -> Vec2 {
    match screen {
        Screen::Settings => vec2(430.0 * u, 278.0 * u),
        Screen::Edit => vec2(430.0 * u, 330.0 * u),
        Screen::Play => Vec2::ZERO,
    }
}

// position (clamped to the screen) and size of the floating window
pub fn win_geom(screen: Screen, win_pos: Option<Vec2>, w: f32, h: f32, u: f32) -> (Vec2, Vec2) {
    let ws = win_size(screen, u);
    let d = win_pos.unwrap_or(vec2((w - 430.0 * u) * 0.5, 14.0 * u));
    let wp = vec2(d.x.clamp(0.0, (w - ws.x).max(0.0)), d.y.clamp(0.0, (h - ws.y).max(0.0)));
    (wp, ws)
}
