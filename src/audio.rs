use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};
use std::f32::consts::TAU;

const RATE: u32 = 22050;

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn wav(samples: &[f32]) -> Vec<u8> {
    // Output canonical 44.1 kHz / 16-bit / stereo PCM (the most widely supported format);
    // the generators run at RATE and are linearly upsampled here.
    let up = 2usize;
    let out_rate = RATE * up as u32;
    let frames = samples.len() * up;
    let data_len = (frames * 4) as u32;
    let mut b: Vec<u8> = Vec::with_capacity(44 + frames * 4);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&out_rate.to_le_bytes());
    b.extend_from_slice(&(out_rate * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..samples.len() {
        let a = samples[i];
        let c = if i + 1 < samples.len() { samples[i + 1] } else { a };
        for k in 0..up {
            let s = a + (c - a) * (k as f32 / up as f32);
            let v = (s.clamp(-1.0, 1.0) * 30000.0) as i16;
            let le = v.to_le_bytes();
            b.extend_from_slice(&le);
            b.extend_from_slice(&le);
        }
    }
    b
}

fn gen(secs: f32, mut f: impl FnMut(f32, usize) -> f32) -> Vec<u8> {
    let n = (secs * RATE as f32) as usize;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        v.push(f(i as f32 / RATE as f32, i));
    }
    wav(&v)
}

fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        t / attack
    } else {
        (-(t - attack) * decay).exp()
    }
}

fn shot() -> Vec<u8> {
    let mut r = Rng(12345);
    let mut lp = 0.0f32;
    gen(0.45, |t, _| {
        let n = r.next();
        lp += (n - lp) * 0.35;
        let crack = n * env(t, 0.001, 55.0);
        let body = lp * env(t, 0.002, 18.0) * 1.2;
        let thump = (TAU * (150.0 * (-t * 14.0).exp() + 45.0) * t).sin() * env(t, 0.002, 14.0);
        let tail = lp * env(t, 0.01, 7.0) * 0.25;
        (crack * 0.6 + body + thump * 0.9 + tail) * 0.8
    })
}

fn click(freq: f32, secs: f32, vol: f32, seed: u32) -> Vec<u8> {
    let mut r = Rng(seed);
    gen(secs, |t, _| {
        let tone = (TAU * freq * t).sin() * env(t, 0.0005, 160.0);
        (tone * 0.6 + r.next() * env(t, 0.0005, 300.0) * 0.5) * vol
    })
}

fn reload() -> Vec<u8> {
    let mut r = Rng(777);
    gen(1.2, |t, _| {
        let mut s = 0.0;
        for (start, f, v) in [(0.0f32, 900.0f32, 0.9f32), (0.45, 500.0, 0.7), (0.85, 1300.0, 1.0)] {
            if t >= start {
                let tt = t - start;
                s += ((TAU * f * tt).sin() * 0.4 + r.next() * 0.6) * env(tt, 0.0005, 70.0) * v;
            }
        }
        s * 0.6
    })
}

fn step() -> Vec<u8> {
    let mut r = Rng(99);
    let mut lp = 0.0f32;
    gen(0.14, |t, _| {
        lp += (r.next() - lp) * 0.18;
        lp * env(t, 0.003, 32.0) * 1.8
    })
}

fn jump() -> Vec<u8> {
    let mut r = Rng(31);
    let mut lp = 0.0f32;
    gen(0.2, |t, _| {
        lp += (r.next() - lp) * 0.1;
        (lp * 1.4 + (TAU * 110.0 * t).sin() * 0.3) * env(t, 0.01, 14.0)
    })
}

fn land() -> Vec<u8> {
    let mut r = Rng(5);
    let mut lp = 0.0f32;
    gen(0.3, |t, _| {
        lp += (r.next() - lp) * 0.12;
        (lp * 2.0 + (TAU * 70.0 * t).sin() * 0.7) * env(t, 0.002, 16.0) * 0.9
    })
}

fn slide() -> Vec<u8> {
    let mut r = Rng(2024);
    let mut lp = 0.0f32;
    gen(0.6, |t, _| {
        lp += (r.next() - lp) * 0.28;
        lp * (t / 0.08).min(1.0) * (1.0 - t / 0.6).max(0.0) * 1.1
    })
}

fn hitmark() -> Vec<u8> {
    gen(0.16, |t, _| {
        ((TAU * 1800.0 * t).sin() + (TAU * 2700.0 * t).sin() * 0.5) * env(t, 0.001, 38.0) * 0.5
    })
}

fn ding() -> Vec<u8> {
    gen(0.5, |t, _| {
        ((TAU * 1250.0 * t).sin() + (TAU * 1900.0 * t).sin() * 0.4) * env(t, 0.001, 9.0) * 0.5
    })
}

fn impact() -> Vec<u8> {
    let mut r = Rng(4242);
    let mut lp = 0.0f32;
    gen(0.2, |t, _| {
        lp += (r.next() - lp) * 0.4;
        lp * env(t, 0.001, 30.0)
    })
}

fn scope() -> Vec<u8> {
    gen(0.12, |t, _| (TAU * (400.0 + 600.0 * t * 8.0) * t).sin() * env(t, 0.003, 30.0) * 0.4)
}

fn ambient() -> Vec<u8> {
    // looping city rumble: low filtered noise with slow swell (loops cleanly via crossfade of phase)
    let mut r = Rng(8080);
    let mut a = 0.0f32;
    let mut b = 0.0f32;
    let secs = 4.0f32;
    gen(secs, |t, _| {
        let n = r.next();
        a += (n - a) * 0.012;
        b += (n - b) * 0.08;
        let swell = 0.75 + 0.25 * (TAU * t / secs).sin();
        (a * 4.0 + b * 0.25) * swell * 0.5
    })
}

pub struct Sfx {
    shot: Option<Sound>,
    empty: Option<Sound>,
    reload: Option<Sound>,
    step: Option<Sound>,
    jump: Option<Sound>,
    land: Option<Sound>,
    slide: Option<Sound>,
    hit: Option<Sound>,
    ding: Option<Sound>,
    impact: Option<Sound>,
    scope: Option<Sound>,
    ui: Option<Sound>,
    amb: Option<Sound>,
    pub master: f32,
    pub loaded: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Snd {
    Shot,
    Empty,
    Reload,
    Step,
    Jump,
    Land,
    Slide,
    Hit,
    Ding,
    Impact,
    Scope,
    Ui,
}

impl Sfx {
    pub async fn load() -> Sfx {
        async fn ld(b: Vec<u8>) -> Option<Sound> {
            load_sound_from_bytes(&b).await.ok()
        }
        let s = Sfx {
            shot: ld(shot()).await,
            empty: ld(click(2200.0, 0.08, 0.7, 11)).await,
            reload: ld(reload()).await,
            step: ld(step()).await,
            jump: ld(jump()).await,
            land: ld(land()).await,
            slide: ld(slide()).await,
            hit: ld(hitmark()).await,
            ding: ld(ding()).await,
            impact: ld(impact()).await,
            scope: ld(scope()).await,
            ui: ld(click(1500.0, 0.05, 0.5, 3)).await,
            amb: ld(ambient()).await,
            master: 1.0,
            loaded: 0,
        };
        let mut s = s;
        s.loaded = [&s.shot, &s.empty, &s.reload, &s.step, &s.jump, &s.land, &s.slide, &s.hit, &s.ding, &s.impact, &s.scope, &s.ui, &s.amb]
            .iter()
            .filter(|x| x.is_some())
            .count();
        if let Some(a) = &s.amb {
            play_sound(a, PlaySoundParams { looped: true, volume: 0.25 * s.master });
        }
        s
    }

    pub fn play(&self, s: Snd, vol: f32) {
        let snd = match s {
            Snd::Shot => &self.shot,
            Snd::Empty => &self.empty,
            Snd::Reload => &self.reload,
            Snd::Step => &self.step,
            Snd::Jump => &self.jump,
            Snd::Land => &self.land,
            Snd::Slide => &self.slide,
            Snd::Hit => &self.hit,
            Snd::Ding => &self.ding,
            Snd::Impact => &self.impact,
            Snd::Scope => &self.scope,
            Snd::Ui => &self.ui,
        };
        if let Some(snd) = snd {
            play_sound(snd, PlaySoundParams { looped: false, volume: (vol * self.master).clamp(0.0, 1.0) });
        }
    }
}
