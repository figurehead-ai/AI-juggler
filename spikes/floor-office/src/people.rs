//! People, painted in art pixels: about 12 wide and 40 tall standing (33 seated), built from parts
//! (skin, hair style and colour, top, trousers, shoes, glasses, headphones, beard) with hue-shifted
//! shading and a coloured outline. A small lanyard badge carries the App's colour, so a team reads
//! as a team. Each person is drawn once per pose they need and cached in the atlas.

use crate::paint::{hex, ramp, Img, Rgba, Rng};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hair { Short, Long, Bun, Curly, Pony, Buzz, Bob, Scarf, Bald }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Top { Tee, Hoodie, Shirt, Blazer, Sweater, Dress }

#[derive(Clone, Copy)]
pub struct Look {
    pub skin: Rgba,
    pub hair: Rgba,
    pub hair_style: Hair,
    pub top: Rgba,
    pub top2: Rgba,
    pub top_kind: Top,
    pub pants: Rgba,
    pub shoes: Rgba,
    pub glasses: bool,
    pub phones: bool,
    pub beard: bool,
    pub badge: Rgba,
}

impl Look {
    pub fn random(r: &mut Rng, badge: Rgba) -> Look {
        let skins = ["#f7d7c0", "#eec09e", "#d9a27a", "#c08558", "#9a643f", "#734a2e", "#55361f"];
        let hairs = ["#2a1c14", "#3d2a1d", "#5b3a22", "#8a5a2e", "#b98a4a", "#d9b56a", "#e8e2d6", "#8e2f24", "#1f1f25"];
        let fabrics = ["#3f5f8a", "#7a3b4a", "#3f6b55", "#b8743a", "#5c4a7a", "#c9b28a", "#2f3a4a", "#a8584a", "#4a7a8a", "#d9c7a0", "#6b6f78", "#8a8f5a"];
        let pants = ["#2b3448", "#3a3a40", "#6b5a44", "#3f5a7a", "#25252b", "#5a4a3a"];
        let shoes = ["#1d1b20", "#5a3a2a", "#e6e2da", "#7a2f2f", "#3a3f4a"];
        let style = *r.pick(&[Hair::Short, Hair::Long, Hair::Bun, Hair::Curly, Hair::Pony, Hair::Buzz, Hair::Bob, Hair::Scarf, Hair::Bald, Hair::Short, Hair::Long]);
        let hair = if style == Hair::Scarf { hex(r.pick(&["#7a3b6a", "#2f6b6b", "#b8743a", "#3f4a7a", "#8a2f3a"])) } else { hex(r.pick(&hairs)) };
        Look {
            skin: hex(r.pick(&skins)),
            hair,
            hair_style: style,
            top: hex(r.pick(&fabrics)),
            top2: hex(r.pick(&fabrics)),
            top_kind: *r.pick(&[Top::Tee, Top::Hoodie, Top::Shirt, Top::Blazer, Top::Sweater, Top::Dress, Top::Tee]),
            pants: hex(r.pick(&pants)),
            shoes: hex(r.pick(&shoes)),
            glasses: r.chance(0.3),
            phones: r.chance(0.16),
            beard: style != Hair::Scarf && r.chance(0.18),
            badge,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pose { SitFront(u8), SitBack(u8), Wave(u8), WalkDown(u8), WalkUp(u8), WalkRight(u8), Sleep, Scratch, Lean }

/// Frame size, including a 2-pixel margin for the outline.
pub const PW: i32 = 16;
pub const PH: i32 = 44;
/// Where the feet are inside a frame (anchor).
pub const FOOT_X: i32 = 8;
pub const FOOT_Y_STAND: i32 = 41;
pub const FOOT_Y_SIT: i32 = 36;

pub fn paint(lk: &Look, pose: Pose) -> Img {
    let mut im = Img::new(PW, PH);
    let (ox, oy) = (2, 2);
    let sitting = matches!(pose, Pose::SitFront(_) | Pose::SitBack(_) | Pose::Sleep | Pose::Scratch | Pose::Lean);
    let front = matches!(pose, Pose::SitFront(_) | Pose::Wave(_) | Pose::WalkDown(_));
    let side = matches!(pose, Pose::WalkRight(_));
    // A seated person sits lower in the frame, so the feet anchor stays at the chair.
    let dy = if sitting { 5 } else { 0 };
    let sh = |c: Rgba, t: f32| ramp(c, t);
    {
        let mut r = |x: i32, y: i32, w: i32, h: i32, c: Rgba| im.rect(ox + x, oy + y + dy, w, h, c);
        let (top_l, top_d) = (sh(lk.top, 0.18), sh(lk.top, -0.3));
        let (skin_d, pants_d) = (sh(lk.skin, -0.25), sh(lk.pants, -0.3));
        // ---- legs ----
        if sitting {
            r(2, 24, 8, 3, lk.pants);
            r(2, 26, 8, 1, pants_d);
            r(3, 27, 2, 5, pants_d);
            r(7, 27, 2, 5, pants_d);
            r(2, 32, 3, 2, lk.shoes);
            r(7, 32, 3, 2, lk.shoes);
        } else if side {
            let Pose::WalkRight(f) = pose else { unreachable!() };
            if f == 0 {
                r(3, 24, 3, 14, pants_d);
                r(6, 24, 3, 14, lk.pants);
                r(2, 38, 4, 2, lk.shoes);
                r(6, 38, 5, 2, lk.shoes);
            } else {
                r(4, 24, 4, 14, lk.pants);
                r(4, 38, 5, 2, lk.shoes);
            }
        } else {
            let step = match pose { Pose::WalkDown(f) | Pose::WalkUp(f) => f as i32 + 1, _ => 0 };
            r(3, 24, 3, if step == 1 { 13 } else { 14 }, lk.pants);
            r(6, 24, 3, if step == 2 { 13 } else { 14 }, pants_d);
            r(2, if step == 1 { 37 } else { 38 }, 4, 2, lk.shoes);
            r(6, if step == 2 { 37 } else { 38 }, 4, 2, lk.shoes);
        }
        // ---- torso ----
        if side {
            r(3, 11, 6, 13, lk.top);
            r(3, 11, 1, 13, top_d);
            r(8, 11, 1, 13, top_l);
        } else {
            r(2, 11, 8, 13, lk.top);
            r(2, 11, 1, 13, top_l);
            r(8, 11, 2, 13, top_d);
            r(2, 23, 8, 1, top_d);
            if lk.top_kind == Top::Dress && !sitting {
                r(1, 21, 10, 7, lk.top);
                r(8, 21, 3, 7, top_d);
            }
            if front {
                match lk.top_kind {
                    Top::Tee | Top::Dress => r(5, 11, 2, 1, lk.skin),
                    Top::Hoodie => { r(3, 11, 6, 2, top_d); r(5, 13, 1, 3, hex("#efeae0")); r(6, 13, 1, 3, hex("#efeae0")); r(4, 18, 4, 3, top_d); }
                    Top::Shirt => { r(4, 11, 1, 2, hex("#f4f1ea")); r(7, 11, 1, 2, hex("#f4f1ea")); for k in 0..4 { r(6, 13 + k * 3, 1, 1, top_d); } }
                    Top::Blazer => { r(4, 11, 4, 12, lk.top2); r(4, 11, 4, 1, sh(lk.top2, 0.2)); }
                    Top::Sweater => { r(2, 22, 8, 1, top_d); r(2, 15, 8, 1, top_d); r(5, 11, 2, 1, hex("#f4f1ea")); }
                }
                // the team lanyard and badge in the App's colour
                r(4, 12, 1, 3, sh(lk.badge, -0.2));
                r(7, 14, 2, 3, lk.badge);
                r(7, 14, 2, 1, sh(lk.badge, 0.35));
            } else if lk.top_kind == Top::Hoodie {
                r(3, 11, 6, 3, top_d);
            }
        }
        // ---- arms ----
        let arm = |r: &mut dyn FnMut(i32, i32, i32, i32, Rgba), x: i32, y: i32, h: i32| {
            r(x, y, 2, h, top_d);
            r(x, y + h, 2, 2, lk.skin);
        };
        match pose {
            Pose::SitFront(f) => {
                let (a, b) = if f == 0 { (0, 1) } else { (1, 0) };
                r(0, 12, 2, 6, top_d); r(10, 12, 2, 6, top_d);
                r(1, 18 + a, 3, 2, top_d); r(8, 18 + b, 3, 2, top_d);
                r(4, 18 + a, 2, 2, lk.skin); r(6, 18 + b, 2, 2, lk.skin);
            }
            Pose::SitBack(f) => {
                let (a, b) = if f == 0 { (0, 1) } else { (1, 0) };
                arm(&mut r, 0, 12 + a, 7); arm(&mut r, 10, 12 + b, 7);
            }
            Pose::Wave(f) => {
                arm(&mut r, 0, 12, 9);
                r(10, 6, 2, 6, top_d);
                if f == 0 { r(11, 2, 2, 4, lk.skin); } else { r(9, 1, 2, 4, lk.skin); }
            }
            Pose::WalkDown(f) | Pose::WalkUp(f) => {
                let s = f as i32;
                arm(&mut r, 0, 12 + s, 9); arm(&mut r, 10, 13 - s, 9);
            }
            Pose::WalkRight(f) => { let x = if f == 0 { 4 } else { 6 }; arm(&mut r, x, 12, 9); }
            Pose::Sleep => { r(0, 9, 3, 4, top_d); r(9, 9, 3, 4, top_d); }
            Pose::Scratch => { arm(&mut r, 0, 12, 7); r(10, 4, 2, 8, top_d); r(9, 2, 2, 2, lk.skin); }
            Pose::Lean => { r(1, 3, 2, 9, top_d); r(9, 3, 2, 9, top_d); r(2, 3, 2, 2, lk.skin); r(8, 3, 2, 2, lk.skin); }
        }
        // ---- head ----
        let hy = if pose == Pose::Sleep { 4 } else { 0 };
        let hd = sh(lk.hair, -0.3);
        let hl = sh(lk.hair, 0.35);
        if side {
            r(3, 2 + hy, 6, 8, lk.skin);
            r(8, 6 + hy, 1, 1, hex("#1d1726"));
            r(9, 7 + hy, 1, 1, skin_d);
            match lk.hair_style {
                Hair::Bald => {}
                Hair::Scarf => { r(2, 1 + hy, 7, 4, lk.hair); r(2, 5 + hy, 3, 7, lk.hair); }
                Hair::Long | Hair::Bob => { r(2, 1 + hy, 7, 3, lk.hair); r(2, 4 + hy, 3, if lk.hair_style == Hair::Long { 9 } else { 5 }, lk.hair); }
                Hair::Pony => { r(2, 1 + hy, 7, 3, lk.hair); r(1, 4 + hy, 2, 5, lk.hair); }
                _ => { r(2, 1 + hy, 7, 3, lk.hair); r(2, 4 + hy, 2, 3, lk.hair); }
            }
            r(3, 1 + hy, 3, 1, hl);
        } else if front {
            r(2, 2, 8, 8, lk.skin);
            r(9, 3, 1, 7, skin_d);
            // ears
            r(1, 5, 1, 2, skin_d); r(10, 5, 1, 2, skin_d);
            let eye = hex("#1d1726");
            r(4, 6, 1, 1, eye); r(7, 6, 1, 1, eye);
            r(4, 5, 1, 1, hd); r(7, 5, 1, 1, hd);
            r(6, 7, 1, 1, skin_d);
            let mouth = if matches!(pose, Pose::Wave(_)) { hex("#7a2e35") } else { sh(lk.skin, -0.35) };
            r(5, 8, 2, 1, mouth);
            match lk.hair_style {
                Hair::Short | Hair::Pony => { r(2, 1, 8, 3, lk.hair); r(2, 4, 1, 2, lk.hair); r(9, 4, 1, 2, lk.hair); r(4, 1, 3, 1, hl); }
                Hair::Long => { r(2, 1, 8, 3, lk.hair); r(1, 3, 2, 10, lk.hair); r(9, 3, 2, 10, lk.hair); r(4, 1, 3, 1, hl); }
                Hair::Bun => { r(2, 1, 8, 3, lk.hair); r(4, -1, 4, 2, lk.hair); r(5, -1, 2, 1, hl); }
                Hair::Curly => { for k in 0..10 { r(1 + k, if k % 2 == 0 { 0 } else { 1 }, 1, 4, if k % 3 == 0 { hd } else { lk.hair }); } r(1, 4, 1, 3, lk.hair); r(10, 4, 1, 3, lk.hair); }
                Hair::Buzz => { r(2, 1, 8, 2, hd); }
                Hair::Bob => { r(1, 1, 10, 3, lk.hair); r(1, 4, 2, 6, lk.hair); r(9, 4, 2, 6, lk.hair); r(4, 1, 3, 1, hl); }
                Hair::Scarf => { r(1, 1, 10, 4, lk.hair); r(1, 5, 2, 7, lk.hair); r(9, 5, 2, 7, lk.hair); r(3, 10, 6, 2, lk.hair); r(3, 1, 4, 1, hl); }
                Hair::Bald => { r(3, 2, 5, 1, sh(lk.skin, 0.25)); }
            }
            if lk.glasses { r(3, 6, 6, 1, hex("#2a2530")); r(4, 6, 1, 1, hex("#a9d8ff")); r(7, 6, 1, 1, hex("#a9d8ff")); }
            if lk.beard { r(3, 8, 6, 2, lk.hair); r(2, 7, 1, 2, lk.hair); r(9, 7, 1, 2, lk.hair); r(5, 8, 2, 1, sh(lk.hair, -0.4)); }
            if lk.phones { r(2, 0, 8, 1, hex("#25252b")); r(1, 4, 1, 3, hex("#25252b")); r(10, 4, 1, 3, hex("#25252b")); }
        } else {
            // seen from behind
            r(2, 1 + hy, 8, 9, lk.hair);
            r(8, 2 + hy, 2, 8, hd);
            r(3, 1 + hy, 3, 1, hl);
            match lk.hair_style {
                Hair::Bald => { r(2, 1 + hy, 8, 9, lk.skin); r(8, 2 + hy, 2, 8, skin_d); r(3, 2 + hy, 3, 1, sh(lk.skin, 0.25)); }
                Hair::Buzz => { r(2, 1 + hy, 8, 9, hd); r(5, 10 + hy, 2, 1, lk.skin); }
                Hair::Short => { r(5, 10 + hy, 2, 1, skin_d); }
                Hair::Long => { r(1, 3 + hy, 10, 11, lk.hair); r(8, 4 + hy, 3, 10, hd); }
                Hair::Bob => { r(1, 2 + hy, 10, 9, lk.hair); r(8, 3 + hy, 3, 8, hd); }
                Hair::Pony => { r(5, 10 + hy, 2, 5, lk.hair); }
                Hair::Bun => { r(4, -1 + hy, 4, 3, lk.hair); r(5, -1 + hy, 2, 1, hl); }
                Hair::Curly => { r(1, 0 + hy, 10, 10, lk.hair); for k in 0..5 { r(2 + k * 2, 1 + hy + (k % 2), 1, 1, hd); } }
                Hair::Scarf => { r(1, 1 + hy, 10, 12, lk.hair); r(8, 2 + hy, 3, 11, hd); }
            }
            if lk.phones { r(2, 0 + hy, 8, 1, hex("#25252b")); r(1, 4 + hy, 1, 3, hex("#25252b")); r(10, 4 + hy, 1, 3, hex("#25252b")); }
        }
    }
    im.outline();
    im
}

pub fn paint_mirrored(lk: &Look, pose: Pose) -> Img {
    paint(lk, pose).mirrored()
}
