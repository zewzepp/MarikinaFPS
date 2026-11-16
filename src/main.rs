use macroquad::prelude::*;
use std::collections::HashMap;

const MAP: [&str; 12] = [
    "############",
    "#..........#",
    "#..##......#",
    "#..........#",
    "#......#...#",
    "#..........#",
    "#...#......#",
    "#..........#",
    "#......##..#",
    "#..........#",
    "#..........#",
    "############",
];
const FOV: f32 = 1.08;
const MAX_DIST: f32 = 18.0;

#[derive(Clone)]
struct Target { x: f32, y: f32, alive: bool, phase: f32 }

struct Game {
    x: f32, y: f32, angle: f32, pitch: f32,
    health: i32, ammo: i32, reserve: i32, reloading: f32,
    fire_cd: f32, jump: f32, jump_v: f32, slide: f32,
    scoped: bool, sneaking: bool, kills: u32, targets: Vec<Target>,
    touch_prev: HashMap<u64, Vec2>,
}

impl Game {
    fn new() -> Self {
        Self { x: 2.2, y: 2.2, angle: 0.35, pitch: 0.0, health: 100,
            ammo: 12, reserve: 60, reloading: 0.0, fire_cd: 0.0, jump: 0.0,
            jump_v: 0.0, slide: 0.0, scoped: false, sneaking: false, kills: 0,
            targets: vec![Target{x:7.3,y:2.1,alive:true,phase:0.0},Target{x:9.2,y:5.2,alive:true,phase:1.8},Target{x:3.8,y:8.7,alive:true,phase:3.2}],
            touch_prev: HashMap::new() }
    }
    fn wall(x: f32, y: f32) -> bool {
        let xi=x.floor() as isize; let yi=y.floor() as isize;
        xi < 0 || yi < 0 || yi >= MAP.len() as isize || xi >= MAP[0].len() as isize || MAP[yi as usize].as_bytes()[xi as usize] == b'#'
    }
    fn move_by(&mut self, dx: f32, dy: f32) {
        if !Self::wall(self.x+dx, self.y) { self.x += dx; }
        if !Self::wall(self.x, self.y+dy) { self.y += dy; }
    }
    fn shoot(&mut self) {
        if self.fire_cd > 0.0 || self.reloading > 0.0 { return; }
        if self.ammo <= 0 { self.start_reload(); return; }
        self.ammo -= 1; self.fire_cd = 0.20;
        let mut best: Option<(usize,f32)> = None;
        for (i,t) in self.targets.iter().enumerate().filter(|(_,t)|t.alive) {
            let dx=t.x-self.x; let dy=t.y-self.y; let d=(dx*dx+dy*dy).sqrt();
            let a=wrap_angle(dy.atan2(dx)-self.angle).abs();
            if a < if self.scoped {0.045} else {0.12} && d < MAX_DIST && best.map_or(true,|(_,bd)|d<bd) { best=Some((i,d)); }
        }
        if let Some((i,_))=best { self.targets[i].alive=false; self.kills+=1; }
    }
    fn start_reload(&mut self) { if self.reloading<=0.0 && self.ammo<12 && self.reserve>0 {self.reloading=1.25;} }
}

fn wrap_angle(mut a:f32)->f32 { while a>std::f32::consts::PI {a-=2.0*std::f32::consts::PI;} while a < -std::f32::consts::PI {a+=2.0*std::f32::consts::PI;} a }
fn button(rect:Rect,label:&str,active:bool) {
    draw_rectangle(rect.x,rect.y,rect.w,rect.h, if active {Color::new(0.23,0.62,0.78,0.78)} else {Color::new(0.04,0.08,0.12,0.62)});
    draw_rectangle_lines(rect.x,rect.y,rect.w,rect.h,1.5,Color::new(0.8,0.92,1.0,0.75));
    let size=if rect.w<58.0 {12.0}else{15.0}; let m=measure_text(label,None,size as u16,1.0);
    draw_text(label,rect.x+(rect.w-m.width)/2.0,rect.y+(rect.h+m.height*0.55)/2.0,size,WHITE);
}
fn inside(p:Vec2,r:Rect)->bool {p.x>=r.x&&p.x<=r.x+r.w&&p.y>=r.y&&p.y<=r.y+r.h}
fn cast(sx:f32,sy:f32,angle:f32)->(f32,bool) {
    let (rx,ry)=(angle.cos(),angle.sin()); let(mut mx,mut my)=(sx.floor() as i32,sy.floor() as i32);
    let dx=if rx.abs()<0.00001 {1e9}else{(1.0/rx).abs()}; let dy=if ry.abs()<0.00001 {1e9}else{(1.0/ry).abs()};
    let stepx=if rx<0.0 {-1}else{1}; let stepy=if ry<0.0 {-1}else{1};
    let mut side_x=if rx<0.0 {(sx-mx as f32)*dx}else{(mx as f32+1.0-sx)*dx};
    let mut side_y=if ry<0.0 {(sy-my as f32)*dy}else{(my as f32+1.0-sy)*dy};
    for _ in 0..64 { let side=if side_x<side_y {side_x+=dx;mx+=stepx;0}else{side_y+=dy;my+=stepy;1};
        if mx<0||my<0||my>=MAP.len() as i32||mx>=MAP[0].len() as i32 {return (MAX_DIST,true)}
        if MAP[my as usize].as_bytes()[mx as usize]==b'#' {return ((if side==0 {side_x-dx}else{side_y-dy}).max(0.01),side==1)} }
    (MAX_DIST,true)
}

#[macroquad::main("Rusty FPS Sketch")]
async fn main() {
    let mut g=Game::new();
    loop {
        let dt=get_frame_time().min(0.05); let w=screen_width(); let h=screen_height();
        let mobile=cfg!(target_os="android") || w < 900.0 || !touches().is_empty();
        let pad=if mobile {12.0}else{16.0};
        let bw=(w*0.105).clamp(54.0,78.0); let bh=44.0; let gap=7.0;
        let fire=Rect::new(w-pad-bw,h-pad-bh,bw,bh);
        let scope=Rect::new(w-pad-bw,h-pad-bh*2.0-gap,bw,bh);
        let reload=Rect::new(w-pad-bw,h-pad-bh*3.0-gap*2.0,bw,bh);
        let jump=Rect::new(w-pad-bw*2.0-gap,h-pad-bh,bw,bh);
        let slide=Rect::new(w-pad-bw*2.0-gap,h-pad-bh*2.0-gap,bw,bh);
        let sneak=Rect::new(w-pad-bw*2.0-gap,h-pad-bh*3.0-gap*2.0,bw,bh);
        let mut move_x=0.0; let mut move_y=0.0; let mut look_delta=Vec2::ZERO;
        if is_key_down(KeyCode::W)||is_key_down(KeyCode::Up){move_y+=1.0;} if is_key_down(KeyCode::S)||is_key_down(KeyCode::Down){move_y-=1.0;}
        if is_key_down(KeyCode::D){move_x+=1.0;} if is_key_down(KeyCode::A){move_x-=1.0;}
        let mut fire_down=is_mouse_button_down(MouseButton::Left); let mut scope_down=is_mouse_button_down(MouseButton::Right);
        let mut jump_press=is_key_pressed(KeyCode::Space); let mut reload_press=is_key_pressed(KeyCode::R);
        let slide_down=is_key_down(KeyCode::LeftShift); let sneak_down=is_key_down(KeyCode::LeftControl);
        if !mobile && is_mouse_button_down(MouseButton::Left) { let d=mouse_delta_position(); look_delta+=d*Vec2::new(w,h)*-1.0; }
        if mobile {
            let center=Vec2::new(86.0,h-95.0); let radius=62.0;
            draw_circle(center.x,center.y,radius,Color::new(0.04,0.08,0.12,0.43));
            draw_circle_lines(center.x,center.y,radius,2.0,Color::new(0.8,0.92,1.0,0.6));
            let active=g.touch_prev.clone(); let mut next=HashMap::new();
            for t in touches() { next.insert(t.id,t.position); if let Some(prev)=active.get(&t.id) { if prev.x>110.0 && ![fire,scope,reload,jump,slide,sneak].iter().any(|r|inside(t.position,*r)) {look_delta+=t.position-*prev;} } }
            for p in next.values() { if p.x<155.0 && p.y>h-210.0 { let v=(*p-center)/radius; move_x+=v.x.clamp(-1.0,1.0); move_y-=v.y.clamp(-1.0,1.0); draw_circle(center.x+v.x.clamp(-1.0,1.0)*31.0,center.y+v.y.clamp(-1.0,1.0)*31.0,20.0,Color::new(0.8,0.92,1.0,0.65)); }
                fire_down|=inside(*p,fire); scope_down|=inside(*p,scope); if inside(*p,jump){jump_press=true;} if inside(*p,reload){reload_press=true;} }
            g.touch_prev=next;
        }
        if is_key_pressed(KeyCode::Escape) { break; }
        if jump_press && g.jump<=0.0 {g.jump_v=4.6;}
        if reload_press {g.start_reload();}
        if is_key_pressed(KeyCode::F) {g.scoped=!g.scoped;}
        g.scoped=scope_down || (g.scoped && !mobile);
        g.sneaking=sneak_down || touches().iter().any(|t|inside(t.position,sneak));
        if touches().iter().any(|t|inside(t.position,slide)) || slide_down {g.slide=0.55;}
        if fire_down {g.shoot();}
        g.angle=wrap_angle(g.angle+look_delta.x*0.0045); g.pitch=(g.pitch+look_delta.y*0.003).clamp(-0.45,0.45);
        let speed=if g.slide>0.0 {4.8}else if g.sneaking {1.35}else{2.7};
        let len=(move_x*move_x+move_y*move_y).sqrt().max(1.0); move_x/=len; move_y/=len;
        let dx=(g.angle.cos()*move_y-g.angle.sin()*move_x)*speed*dt; let dy=(g.angle.sin()*move_y+g.angle.cos()*move_x)*speed*dt; g.move_by(dx,dy);
        g.slide=(g.slide-dt).max(0.0); g.fire_cd=(g.fire_cd-dt).max(0.0);
        if g.jump_v!=0.0 || g.jump>0.0 {g.jump+=g.jump_v*dt;g.jump_v-=11.5*dt;if g.jump<=0.0 {g.jump=0.0;g.jump_v=0.0;}}
        if g.reloading>0.0 {g.reloading-=dt;if g.reloading<=0.0 {let n=(12-g.ammo).min(g.reserve);g.ammo+=n;g.reserve-=n;g.reloading=0.0;}}
        for t in &mut g.targets {t.phase+=dt; if t.alive {t.x+=t.phase.sin()*dt*0.25;}}

        clear_background(Color::new(0.05,0.09,0.14,1.0));
        let horizon=h*0.49+g.pitch*h*0.45-g.jump*14.0;
        draw_rectangle(0.0,0.0,w,horizon,Color::new(0.11,0.17,0.24,1.0)); draw_rectangle(0.0,horizon,w,h-horizon,Color::new(0.16,0.16,0.15,1.0));
        let cols=(w as usize).clamp(200,1000); let fw=w/cols as f32; let fov=if g.scoped {0.30}else{FOV}; let mut zbuf=vec![MAX_DIST;cols];
        for i in 0..cols {let a=g.angle+((i as f32/cols as f32)-0.5)*fov; let (d,side)=cast(g.x,g.y,a);let corrected=(d*wrap_angle(a-g.angle).cos()).max(0.08);zbuf[i]=corrected;
            let wall_h=(h*0.88/corrected).min(h*2.0);let shade=(1.0-corrected/MAX_DIST).clamp(0.15,0.9)*(if side {0.72}else{1.0});let col=Color::new(0.27*shade,0.48*shade,0.57*shade,1.0);
            draw_rectangle(i as f32*fw,horizon-wall_h*0.5,fw+1.0,wall_h,col);
            let floor_start=(horizon+wall_h*0.5).clamp(horizon,h); if floor_start<h {draw_rectangle(i as f32*fw,floor_start,fw+1.0,h-floor_start,Color::new(0.12,0.13,0.13,1.0));}
        }
        let mut order:Vec<usize>=(0..g.targets.len()).collect();order.sort_by(|a,b|{let da=(g.targets[*b].x-g.x).hypot(g.targets[*b].y-g.y);let db=(g.targets[*a].x-g.x).hypot(g.targets[*a].y-g.y);da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)});
        for i in order {let t=&g.targets[i];if !t.alive {continue;}let dx=t.x-g.x;let dy=t.y-g.y;let d=dx.hypot(dy);let rel=wrap_angle(dy.atan2(dx)-g.angle);if rel.abs()>fov*0.62||d<0.2{continue;}let sx=w*0.5+(rel/fov)*w;let size=(h*0.34/d).clamp(8.0,h*0.55);let ci=((sx/w*cols as f32) as usize).min(cols-1);if zbuf[ci]+0.12<d{continue;}let y=horizon-size*0.45;
            draw_circle(sx,y-size*0.27,size*0.19,Color::new(0.88,0.59,0.32,1.0));draw_rectangle(sx-size*0.20,y-size*0.12,size*0.40,size*0.62,Color::new(0.72,0.18,0.15,1.0));draw_rectangle(sx-size*0.13,y+size*0.42,size*0.10,size*0.3,GRAY);draw_rectangle(sx+size*0.03,y+size*0.42,size*0.10,size*0.3,GRAY);
        }
        // Simple weapon silhouette, lowered while sliding.
        let recoil=if g.fire_cd>0.13 {12.0}else{0.0};let gun_y=h-30.0+(if g.slide>0.0 {42.0}else{0.0})+recoil;
        draw_rectangle(w*0.58,gun_y-72.0,70.0,106.0,Color::new(0.12,0.14,0.16,1.0));draw_rectangle(w*0.60,gun_y-103.0,54.0,46.0,Color::new(0.26,0.29,0.30,1.0));draw_rectangle(w*0.62,gun_y-116.0,12.0,20.0,Color::new(0.1,0.1,0.1,1.0));
        draw_text("RUSTY FPS  /  TRAINING YARD",pad,27.0,22.0,WHITE);draw_text(&format!("HP {}     {} / {}",g.health,g.ammo,g.reserve),pad,55.0,21.0,Color::new(0.65,0.94,0.79,1.0));
        draw_text(&format!("TARGETS {}     {}",g.kills,g.targets.iter().filter(|t|t.alive).count()),pad,82.0,17.0,Color::new(0.82,0.88,0.92,1.0));
        if g.reloading>0.0 {draw_text("RELOADING...",w*0.5-58.0,h*0.73,20.0,WHITE);} if g.sneaking {draw_text("SNEAK",w*0.5-27.0,h-24.0,16.0,WHITE);} if g.slide>0.0 {draw_text("SLIDING",w*0.5-32.0,h-48.0,16.0,WHITE);}
        // Crosshair / scope lens.
        if g.scoped {let r=h.min(w)*0.37;draw_circle(w*0.5,horizon,r,Color::new(0.015,0.02,0.025,0.90));draw_circle_lines(w*0.5,horizon,r,3.0,Color::new(0.55,0.66,0.7,1.0));draw_line(w*0.5-r,horizon,w*0.5+r,horizon,1.0,Color::new(0.75,0.82,0.83,0.7));draw_line(w*0.5,horizon-r,w*0.5,horizon+r,1.0,Color::new(0.75,0.82,0.83,0.7));}else{draw_line(w*0.5-8.0,horizon,w*0.5+8.0,horizon,2.0,WHITE);draw_line(w*0.5,horizon-8.0,w*0.5,horizon+8.0,2.0,WHITE);}
        if mobile {button(fire,"FIRE",fire_down);button(scope,"SCOPE",g.scoped);button(reload,"RELOAD",g.reloading>0.0);button(jump,"JUMP",g.jump>0.0);button(slide,"SLIDE",g.slide>0.0);button(sneak,"SNEAK",g.sneaking);}
        next_frame().await
    }
}
