// Tiny persistent settings store (no extra crates): HUD layout, slide mode, volume.
use crate::ui::ElemCfg;
use macroquad::prelude::*;

const PKG: &str = "com.vantalauncher.marikinafps";

pub struct Saved {
    pub cfg: Option<[ElemCfg; 6]>,
    pub slide_hold: bool,
    pub volume: f32,
}

fn paths() -> Vec<String> {
    vec![
        format!("/data/data/{}/files/settings.txt", PKG),
        format!("/data/user/0/{}/files/settings.txt", PKG),
        "marikinafps_settings.txt".to_string(),
    ]
}

pub fn encode(cfg: &[ElemCfg; 6], slide_hold: bool, volume: f32) -> String {
    let mut s = format!("v=1\nslide_hold={}\nvolume={:.2}\n", slide_hold as i32, volume);
    for (i, c) in cfg.iter().enumerate() {
        s += &format!("e{}={:.5},{:.5},{:.2},{:.2}\n", i, c.pos.x, c.pos.y, c.scale, c.opacity);
    }
    s
}

pub fn save(text: &str) {
    for p in paths() {
        if let Some(dir) = std::path::Path::new(&p).parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        if std::fs::write(&p, text).is_ok() {
            return;
        }
    }
}

pub fn load() -> Saved {
    let mut out = Saved { cfg: None, slide_hold: false, volume: 1.0 };
    let mut text = None;
    for p in paths() {
        if let Ok(t) = std::fs::read_to_string(&p) {
            text = Some(t);
            break;
        }
    }
    let Some(text) = text else { return out };
    let mut els: [Option<ElemCfg>; 6] = [None; 6];
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match k {
            "slide_hold" => out.slide_hold = v.trim() == "1",
            "volume" => out.volume = v.trim().parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0),
            _ if k.starts_with('e') => {
                if let Ok(i) = k[1..].parse::<usize>() {
                    let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if i < 6 && n.len() == 4 {
                        els[i] = Some(ElemCfg {
                            pos: vec2(n[0].clamp(0.0, 1.0), n[1].clamp(0.0, 1.0)),
                            scale: n[2].clamp(0.5, 2.0),
                            opacity: n[3].clamp(0.2, 1.0),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    if els.iter().all(|e| e.is_some()) {
        out.cfg = Some([
            els[0].unwrap(),
            els[1].unwrap(),
            els[2].unwrap(),
            els[3].unwrap(),
            els[4].unwrap(),
            els[5].unwrap(),
        ]);
    }
    out
}
