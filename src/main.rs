mod audio;
mod game;
mod gfx;
mod render;
mod save;
mod tex;
mod ui;
mod world;

use audio::{Sfx, Snd};
use game::*;
use gfx::*;
use macroquad::prelude::*;
use std::collections::HashMap;
use ui::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Marikina City FPS".to_owned(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // loading screen (one frame) while the city, textures and sounds are generated
    clear_background(Color::new(0.05, 0.07, 0.10, 1.0));
    ui::draw_center("Building Marikina City...", screen_width() * 0.5, screen_height() * 0.5, 34.0, WHITE);
    next_frame().await;

    let (atlas, _avg) = tex::make_texture();
    let city = world::build_world();
    let mut g = Game::new(&city);
    let mut sfx = Sfx::load().await;

    let mut roles: HashMap<u64, Role> = HashMap::new();
    let mut last: HashMap<u64, Vec2> = HashMap::new();
    let mut drag_off: HashMap<u64, Vec2> = HashMap::new();
    let mut joy_origin: Option<Vec2> = None;
    let mut touch_seen = false;

    let mut screen = Screen::Play;
    let saved = save::load();
    sfx.set_master(saved.volume);
    let mut slide_hold = saved.slide_hold; // false = tap to slide, true = hold to slide
    let mut cfg: Option<[ElemCfg; 6]> = saved.cfg;
    let mut last_saved = String::new();
    let mut sel: usize = 3;
    let mut win_pos: Option<Vec2> = None;
    let mut win_off = Vec2::ZERO;
    let mut mouse_win: Option<Vec2> = None;
    let mut mouse_drag: Option<(usize, Vec2)> = None;

    loop {
        let dt = get_frame_time().min(0.05);
        let w = screen_width();
        let h = screen_height();
        if cfg.is_none() {
            cfg = Some([
                default_cfg(0, w, h),
                default_cfg(1, w, h),
                default_cfg(2, w, h),
                default_cfg(3, w, h),
                default_cfg(4, w, h),
                default_cfg(5, w, h),
            ]);
        }
        let mut cf = cfg.unwrap();
        let ui = Layout::new(w, h, &cf);
        let u = ui.u;
        let joy_home = ui.pads[0].c;
        let joy_r = ui.pads[0].r;

        let touch_list = touches();
        if !touch_list.is_empty() {
            touch_seen = true;
        }
        let mobile = cfg!(target_os = "android") || touch_seen;

        roles.retain(|id, _| touch_list.iter().any(|t| t.id == *id));
        last.retain(|id, _| touch_list.iter().any(|t| t.id == *id));
        drag_off.retain(|id, _| touch_list.iter().any(|t| t.id == *id));
        if !roles.values().any(|r| *r == Role::Joy) {
            joy_origin = None;
        }

        let (wp, ws) = win_geom(screen, win_pos, w, h, u);
        let title = Rect::new(wp.x, wp.y, ws.x, 36.0 * u);
        let win_rect = Rect::new(wp.x, wp.y, ws.x, ws.y);
        let zp = zoom_pads(w, h, u);

        let mut mv = Vec2::ZERO; // x = strafe right, y = forward
        let mut look = Vec2::ZERO;
        let mut joy_mag = 0.0f32;
        let mut fire = false;
        let mut jump = false;
        let mut reload = false;
        let mut scope_tap = false;
        let mut snk_tap = false;
        let mut snk_held = false;
        let mut open_settings = false;
        let mut zoom_step = 0i32;
        let mut joy_knob: Option<Vec2> = None;
        let mut taps: Vec<Vec2> = Vec::new();

        for t in &touch_list {
            let p = t.position;

            // menus: taps, dragging the floating window, dragging HUD buttons
            if screen != Screen::Play {
                match t.phase {
                    TouchPhase::Started => {
                        taps.push(p);
                        if title.contains(p) {
                            roles.insert(t.id, Role::Win);
                            win_off = p - wp;
                        } else if screen == Screen::Edit && !win_rect.contains(p) {
                            if let Some(e) = ui.hit_any(p) {
                                let i = e as usize;
                                sel = i;
                                roles.insert(t.id, Role::Drag(i));
                                drag_off.insert(t.id, p - ui.pads[i].c);
                            }
                        }
                    }
                    TouchPhase::Moved => match roles.get(&t.id).copied() {
                        Some(Role::Win) => win_pos = Some(p - win_off),
                        Some(Role::Drag(i)) => {
                            let off = drag_off.get(&t.id).copied().unwrap_or(Vec2::ZERO);
                            cf[i].pos = ui.clamp_pos(i, p - off, w, h);
                        }
                        _ => {}
                    },
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        roles.remove(&t.id);
                        drag_off.remove(&t.id);
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
                        // Fire / Slide stay bound to this finger until it lifts, so
                        // other fingers (stick, look, other buttons) can never steal them
                        if e == Elem::Fire || e == Elem::Snk {
                            Role::Hold(e as usize)
                        } else {
                            Role::Btn
                        }
                    } else if g.zoom > 0.5 && p.distance(zp[0].c) <= zp[0].r * 1.2 {
                        zoom_step = 1;
                        Role::Btn
                    } else if g.zoom > 0.5 && p.distance(zp[1].c) <= zp[1].r * 1.2 {
                        zoom_step = -1;
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
                Role::Hold(i) => {
                    let pad = &ui.pads[i];
                    // forgiving: the thumb may wander well outside the circle
                    if p.distance(pad.c) <= pad.r * 2.2 {
                        if i == Elem::Fire as usize {
                            fire = true;
                            // aim while firing with the same thumb
                            if let Some(prev) = last.get(&t.id) {
                                look += p - *prev;
                            }
                            last.insert(t.id, p);
                        } else {
                            snk_held = true;
                        }
                    }
                }
                _ => {}
            }
        }

        // Mouse (desktop): settings button, menu taps, dragging HUD buttons / window
        if !mobile {
            let (mx, my) = mouse_position();
            let mp = vec2(mx, my);
            if is_mouse_button_pressed(MouseButton::Left) {
                if screen == Screen::Play {
                    if ui.settings_hit(mp) {
                        open_settings = true;
                    }
                } else {
                    taps.push(mp);
                    if title.contains(mp) {
                        mouse_win = Some(mp - wp);
                    } else if screen == Screen::Edit && !win_rect.contains(mp) {
                        if let Some(e) = ui.hit_any(mp) {
                            let i = e as usize;
                            sel = i;
                            mouse_drag = Some((i, mp - ui.pads[i].c));
                        }
                    }
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                if let Some(o) = mouse_win {
                    win_pos = Some(mp - o);
                }
                if let Some((i, o)) = mouse_drag {
                    cf[i].pos = ui.clamp_pos(i, mp - o, w, h);
                }
            } else {
                mouse_win = None;
                mouse_drag = None;
            }
        }

        // Keyboard + mouse look (desktop)
        let mut kb_sprint = false;
        let mut ctrl = false;
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
            if is_key_pressed(KeyCode::E) {
                zoom_step = 1;
            }
            if is_key_pressed(KeyCode::Q) {
                zoom_step = -1;
            }
            let wheel = mouse_wheel().1;
            if wheel > 0.0 {
                zoom_step = 1;
            } else if wheel < 0.0 {
                zoom_step = -1;
            }
            kb_sprint = is_key_down(KeyCode::LeftShift);
            ctrl = is_key_down(KeyCode::LeftControl);
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
            ctrl = false;
            zoom_step = 0;
        }

        // ------------------------------------------------ menus
        let mut new_screen = screen;
        let mut left_menu = false;
        if open_settings {
            new_screen = Screen::Settings;
        }
        if !taps.is_empty() {
            sfx.play(Snd::Ui, 0.5);
        }
        {
            let tapped = |r: Rect| taps.iter().any(|p| r.contains(*p));
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
                    if tapped(btn(1, 150.0, 56.0)) {
                        sfx.set_master((((sfx.master - 0.1) * 10.0).round() / 10.0).max(0.0));
                    }
                    if tapped(btn(1, 286.0, 56.0)) {
                        sfx.set_master((((sfx.master + 0.1) * 10.0).round() / 10.0).min(1.0));
                    }
                    if tapped(btn(1, 150.0, 56.0)) || tapped(btn(1, 286.0, 56.0)) {
                        sfx.play(Snd::Shot, 0.6);
                    }
                    if tapped(btn(2, 12.0, 406.0)) {
                        new_screen = Screen::Edit;
                    }
                    if tapped(btn(3, 12.0, 406.0)) {
                        new_screen = Screen::Play;
                    }
                }
                Screen::Edit => {
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
                        cf[sel].scale = (((cf[sel].scale - 0.1) * 10.0).round() / 10.0).max(0.5);
                    }
                    if tapped(btn(1, 286.0, 56.0)) {
                        cf[sel].scale = (((cf[sel].scale + 0.1) * 10.0).round() / 10.0).min(2.0);
                    }
                    if tapped(btn(2, 150.0, 56.0)) {
                        cf[sel].opacity = (((cf[sel].opacity - 0.1) * 10.0).round() / 10.0).max(0.2);
                    }
                    if tapped(btn(2, 286.0, 56.0)) {
                        cf[sel].opacity = (((cf[sel].opacity + 0.1) * 10.0).round() / 10.0).min(1.0);
                    }
                    // nudge: fine tuning only, buttons are placed by dragging
                    let n = vec2(NUDGE_PX / w, NUDGE_PX / h);
                    let mut np = cf[sel].pos;
                    if tapped(btn(3, 150.0, 56.0)) {
                        np.x -= n.x;
                    }
                    if tapped(btn(3, 214.0, 56.0)) {
                        np.y -= n.y;
                    }
                    if tapped(btn(3, 278.0, 56.0)) {
                        np.y += n.y;
                    }
                    if tapped(btn(3, 342.0, 56.0)) {
                        np.x += n.x;
                    }
                    cf[sel].pos = np;
                    if tapped(btn(4, 12.0, 190.0)) {
                        cf[sel] = default_cfg(sel, w, h);
                    }
                    if tapped(btn(4, 228.0, 190.0)) {
                        new_screen = Screen::Settings;
                    }
                }
                Screen::Play => {}
            }
        }
        if new_screen != screen {
            left_menu = true;
            screen = new_screen;
            roles.clear();
            last.clear();
            drag_off.clear();
            mouse_drag = None;
            mouse_win = None;
            joy_origin = None;
        }
        // keep every button on screen (e.g. after a rotation or resize)
        {
            let tmp = Layout::new(w, h, &cf);
            for i in 0..6 {
                let c = vec2(cf[i].pos.x * w, cf[i].pos.y * h);
                cf[i].pos = tmp.clamp_pos(i, c, w, h);
            }
        }
        // persist HUD layout / settings once the finger or mouse is released
        if screen != Screen::Play || left_menu {
            let idle = roles.is_empty() && mouse_drag.is_none() && mouse_win.is_none();
            if idle {
                let txt = save::encode(&cf, slide_hold, sfx.master);
                if txt != last_saved {
                    save::save(&txt);
                    last_saved = txt;
                }
            }
        }
        cfg = Some(cf);

        // ------------------------------------------------ simulate
        let input = Input {
            mv,
            look: look / u,
            joy_mag,
            kb_sprint,
            ctrl,
            jump,
            fire,
            reload,
            scope_tap,
            snk_tap,
            snk_held,
            slide_hold,
            zoom_step,
        };
        g.update(&city, &input, dt);
        for (s, v) in g.ev.drain(..) {
            sfx.play(s, v);
        }

        // ------------------------------------------------------------ render
        clear_background(fog_color());
        let cam = g.cam_pos(&city);
        let d = g.aim_dir();
        let base_fov = 1.05 + 0.08 * g.boost;
        let fov = base_fov * (1.0 - g.zoom) + (1.05 / g.mag) * g.zoom;
        set_camera(&Camera3D {
            position: cam,
            target: cam + d * 10.0,
            up: vec3(0.0, 1.0, 0.0),
            fovy: fov,
            ..Default::default()
        });

        draw_sky(&atlas, cam, g.t);
        let mut b = Batch::new(&atlas, cam);
        city.draw(&mut b, cam, d);
        for tg in &g.targets {
            if tg.alive {
                let dx = tg.x - cam.x;
                let dz = tg.z - cam.z;
                if dx * dx + dz * dz < 120.0 * 120.0 {
                    render::draw_target(&mut b, tg);
                }
            }
        }
        for p in &g.props.list {
            if p.alive {
                let dx = p.x - cam.x;
                let dz = p.z - cam.z;
                if dx * dx + dz * dz < 110.0 * 110.0 {
                    render::draw_prop(&mut b, p, g.t);
                }
            }
        }
        if g.zoom < 0.75 {
            render::draw_player(&mut b, &g, &city);
        }
        let right = d.cross(Vec3::Y).normalize_or_zero();
        let upv = right.cross(d);
        render::draw_fx(&mut b, &g, right, upv);
        b.flush();
        if let Some((from, to)) = g.tracer {
            if g.tracer_t > 0.0 {
                draw_line_3d(from, to, Color::new(1.0, 0.9, 0.5, 1.0));
            }
        }

        // ------------------------------------------------------------ HUD
        set_default_camera();
        let cx = w * 0.5;
        let cy = h * 0.5;

        render::draw_scope(&g, w, h, u);
        if g.zoom < 0.4 {
            render::draw_crosshair(&g, w, h, u);
        }
        if g.hit_t > 0.0 {
            let s = 12.0 * u;
            let c = Color::new(1.0, 0.35, 0.3, 1.0);
            draw_line(cx - s, cy - s, cx - s * 0.4, cy - s * 0.4, 3.0, c);
            draw_line(cx + s, cy - s, cx + s * 0.4, cy - s * 0.4, 3.0, c);
            draw_line(cx - s, cy + s, cx - s * 0.4, cy + s * 0.4, 3.0, c);
            draw_line(cx + s, cy + s, cx + s * 0.4, cy + s * 0.4, 3.0, c);
        }
        if g.head_t > 0.0 {
            let a = (g.head_t / 1.2).min(1.0);
            ui::draw_center("HEADSHOT", cx, cy - h * 0.12, 30.0 * u, Color::new(1.0, 0.8, 0.2, a));
        }

        let pad = 14.0 * u;
        draw_text("MARIKINA CITY  /  DOWNTOWN", pad, 27.0 * u, 22.0 * u, WHITE);
        draw_text(
            &format!("AMMO {} / {}", g.ammo, g.reserve),
            pad,
            55.0 * u,
            21.0 * u,
            Color::new(0.65, 0.94, 0.79, 1.0),
        );
        draw_text(
            &format!("TARGETS {}   HEADSHOTS {}", g.kills, g.heads),
            pad,
            80.0 * u,
            17.0 * u,
            Color::new(0.82, 0.88, 0.92, 1.0),
        );
        if g.zoom < 0.5 {
            render::draw_minimap(&g, pad, 94.0 * u, 110.0 * u);
        }
        if g.reloading > 0.0 {
            ui::draw_center("RELOADING...", cx, cy + h * 0.22, 26.0 * u, WHITE);
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
            if g.zoom > 0.5 && screen == Screen::Play {
                draw_pad(&zp[0], "+", false, u);
                draw_pad(&zp[1], "-", false, u);
            }
        }
        if screen == Screen::Edit {
            let p = &ui.pads[sel];
            draw_circle_lines(p.c.x, p.c.y, p.r + 6.0 * u, 3.0 * u, Color::new(1.0, 0.85, 0.2, 1.0));
            ui::draw_center("Drag any button to place it anywhere", cx, h - 24.0 * u, 20.0 * u, Color::new(1.0, 0.9, 0.5, 1.0));
        }

        // settings button (octagon, top right)
        {
            let s = &ui.settings;
            draw_poly(s.c.x, s.c.y, 8, s.r, 22.5, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_poly_lines(s.c.x, s.c.y, 8, s.r, 22.5, 2.0 * u, Color::new(0.85, 0.94, 1.0, 0.8));
            ui::draw_center("Settings", s.c.x, s.c.y, 13.0 * u, WHITE);
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
                    draw_text(&format!("snd {}/15", sfx.loaded), wp.x + ws.x - 78.0 * u, wp.y + 25.0 * u, 16.0 * u, Color::new(0.7, 0.85, 1.0, 1.0));
                    draw_text("Slide", wp.x + 14.0 * u, row_y(0) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(0, 150.0, 120.0), "TAP", !slide_hold, u);
                    ui_button(btn(0, 282.0, 120.0), "HOLD", slide_hold, u);
                    draw_text("Volume", wp.x + 14.0 * u, row_y(1) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(1, 150.0, 56.0), "-", false, u);
                    ui_button(btn(1, 286.0, 56.0), "+", false, u);
                    let vp = (sfx.master * 100.0).round() as i32;
                    ui::draw_center(&format!("{}%", vp), wp.x + 246.0 * u, row_y(1) + 23.0 * u, 20.0 * u, WHITE);
                    ui_button(btn(2, 12.0, 406.0), "CUSTOMIZE HUD", false, u);
                    ui_button(btn(3, 12.0, 406.0), "CLOSE", false, u);
                }
                Screen::Edit => {
                    let cs = cfg.unwrap()[sel];
                    draw_text("HUD EDITOR   (drag here to move)", wp.x + 12.0 * u, wp.y + 25.0 * u, 18.0 * u, WHITE);
                    ui_button(btn(0, 12.0, 56.0), "<", false, u);
                    ui_button(btn(0, 362.0, 56.0), ">", false, u);
                    ui::draw_center(ELEMS[sel].name(), wp.x + 215.0 * u, row_y(0) + 23.0 * u, 22.0 * u, WHITE);

                    draw_text("Size", wp.x + 14.0 * u, row_y(1) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(1, 150.0, 56.0), "-", false, u);
                    ui_button(btn(1, 286.0, 56.0), "+", false, u);
                    let pct = (cs.scale * 100.0).round() as i32;
                    ui::draw_center(&format!("{}%", pct), wp.x + 246.0 * u, row_y(1) + 23.0 * u, 20.0 * u, WHITE);

                    draw_text("Opacity", wp.x + 14.0 * u, row_y(2) + 30.0 * u, 20.0 * u, txt);
                    ui_button(btn(2, 150.0, 56.0), "-", false, u);
                    ui_button(btn(2, 286.0, 56.0), "+", false, u);
                    let op = (cs.opacity * 100.0).round() as i32;
                    ui::draw_center(&format!("{}%", op), wp.x + 246.0 * u, row_y(2) + 23.0 * u, 20.0 * u, WHITE);

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
