//! Procedural textures, painted once at start into one atlas: each area's floor and wall material,
//! screens, signs, book spines, posters, rugs and the skyline's windows. Models are flat-coloured;
//! textures carry only what geometry can't (patterns and text). A second small atlas holds the
//! overlay's glyphs and balloon icons.

use crate::kit::{FloorMat, WallFinish};
use crate::paint::{draw_text, hex, mix, noise2, ramp, text_width, Img, Rgba, Rng};
use std::collections::HashMap;

pub const PX_PER_M: f32 = 24.0;

pub struct Atlas { pub img: Img, x: i32, y: i32, shelf: i32, map: HashMap<String, [f32; 4]> }

impl Atlas {
    pub fn new(w: i32, h: i32) -> Self { Atlas { img: Img::new(w, h), x: 0, y: 0, shelf: 0, map: HashMap::new() } }
    /// Adds an image; returns its UV rect (u, v, du, dv), inset half a texel so tiling never bleeds.
    pub fn add(&mut self, key: &str, im: &Img) -> [f32; 4] {
        if let Some(r) = self.map.get(key) { return *r; }
        if self.x + im.w + 2 > self.img.w { self.x = 0; self.y += self.shelf + 2; self.shelf = 0; }
        assert!(self.y + im.h < self.img.h, "texture atlas full");
        let (x, y) = (self.x + 1, self.y + 1);
        self.img.blit(im, x, y);
        self.x += im.w + 2;
        self.shelf = self.shelf.max(im.h);
        let (aw, ah) = (self.img.w as f32, self.img.h as f32);
        let r = [(x as f32 + 0.5) / aw, (y as f32 + 0.5) / ah, (im.w as f32 - 1.0) / aw, (im.h as f32 - 1.0) / ah];
        self.map.insert(key.to_string(), r);
        r
    }
    /// Adds an image addressed exactly (for glyphs and icons, which are never tiled).
    pub fn add_exact(&mut self, key: &str, im: &Img) -> [f32; 4] {
        let r = self.add(key, im);
        let (aw, ah) = (self.img.w as f32, self.img.h as f32);
        [r[0] - 0.5 / aw, r[1] - 0.5 / ah, r[2] + 1.0 / aw, r[3] + 1.0 / ah]
    }
    pub fn get(&self, key: &str) -> [f32; 4] { *self.map.get(key).unwrap_or_else(|| panic!("no texture {key}")) }
}

/// The repeat size of a floor material, in metres, chosen so its pattern tiles.
pub fn floor_tile(m: &FloorMat) -> f32 {
    match m { FloorMat::Concrete { .. } => 6.0, FloorMat::Carpet { .. } | FloorMat::Terrazzo { .. } => 4.0, _ => 4.8 }
}

pub fn floor_img(m: &FloorMat, seed: u32) -> Img {
    let t = floor_tile(m);
    let n = (t * PX_PER_M) as i32;
    let mut im = Img::new(n, n);
    for y in 0..n { for x in 0..n {
        let (wx, wz) = (x as f32 / PX_PER_M, y as f32 / PX_PER_M);
        im.blend(x, y, floor_colour(m, wx, wz, seed));
    } }
    im
}

pub fn wall_img(w: &WallFinish, seed: u32, accent: Rgba) -> Img {
    let (wm, hm) = (4.8, 3.0);
    let (nw, nh) = ((wm * PX_PER_M) as i32, (hm * PX_PER_M) as i32);
    let mut im = Img::new(nw, nh);
    for y in 0..nh { for x in 0..nw {
        im.blend(x, y, wall_colour(w, x as f32 / PX_PER_M, hm - y as f32 / PX_PER_M, seed, accent));
    } }
    im
}

fn floor_colour(m: &FloorMat, x: f32, z: f32, seed: u32) -> Rgba {
    match *m {
        FloorMat::Planks { wood, width } => {
            let row = (z / width).floor() as i32;
            let len = 1.6;
            let off = (row.rem_euclid(3)) as f32 * len / 3.0;
            let u = (x + off) / len;
            let plank = u.floor() as i32;
            let tone = (noise2(row, plank.rem_euclid(3), seed) - 0.5) * 0.22;
            let seam = u.fract() < 0.02 || (z / width).fract() < 0.07;
            let grain = (noise2((x * 40.0) as i32, row, seed ^ 9) - 0.5) * 0.06;
            ramp(wood, if seam { -0.32 } else { tone + grain })
        }
        FloorMat::Parquet { wood, block } => {
            let (i, j) = ((x / block).floor() as i32, (z / block).floor() as i32);
            let (u, v) = ((x / block).fract(), (z / block).fract());
            let along_x = (i + j) % 2 == 0;
            let strip = if along_x { v } else { u } * 4.0;
            let k = strip.floor() as i32;
            let seam = strip.fract() < 0.1 || u < 0.03 || v < 0.03;
            let tone = (noise2(i * 7 + k, j, seed) - 0.5) * 0.25;
            ramp(wood, if seam { -0.3 } else { tone + if along_x { 0.05 } else { -0.05 } })
        }
        FloorMat::Carpet { a, b, tile } => {
            let (i, j) = ((x / tile).floor() as i32, (z / tile).floor() as i32);
            let (u, v) = ((x / tile).fract(), (z / tile).fract());
            let c = if (i + j) % 2 == 0 { a } else { b };
            let line = if (i + j) % 2 == 0 { (u * 9.0).fract() < 0.3 } else { (v * 9.0).fract() < 0.3 };
            let n = (noise2((x * 30.0) as i32, (z * 30.0) as i32, seed) - 0.5) * 0.1;
            ramp(c, if line { -0.08 } else { 0.0 } + n)
        }
        FloorMat::Concrete { tone, seam } => {
            let blot = (noise2((x / 2.0).floor() as i32, (z / 2.0).floor() as i32, seed) - 0.5) * 0.1 + (noise2((x * 2.0) as i32, (z * 2.0) as i32, seed ^ 3) - 0.5) * 0.06;
            let speck = noise2((x * 45.0) as i32, (z * 45.0) as i32, seed ^ 5) > 0.93;
            let s = (x / seam).fract() < 0.01 || (z / seam).fract() < 0.012;
            ramp(tone, if s { -0.35 } else { blot + if speck { -0.12 } else { 0.0 } })
        }
        FloorMat::Terrazzo { base, chips } => {
            let (i, j) = ((x / 0.05).floor() as i32, (z / 0.05).floor() as i32);
            let n = noise2(i, j, seed);
            if n > 0.86 { chips[((n * 100.0) as usize) % 3] } else if n > 0.8 { ramp(base, -0.25) } else { ramp(base, (noise2(i / 8, j / 8, seed ^ 2) - 0.5) * 0.06) }
        }
        FloorMat::Rubber { c } => {
            let (u, v) = ((x / 0.12).fract(), (z / 0.12).fract());
            let stud = (u - 0.5).powi(2) + (v - 0.5).powi(2) < 0.06;
            ramp(c, if stud { 0.12 } else { (noise2((x * 20.0) as i32, (z * 20.0) as i32, seed) - 0.5) * 0.05 })
        }
    }
}

fn wall_colour(w: &WallFinish, x: f32, y: f32, seed: u32, accent: Rgba) -> Rgba {
    match *w {
        WallFinish::Paint { c, dado } => {
            if y < 1.0 { ramp(dado, if (y - 0.98).abs() < 0.03 { 0.2 } else { -0.05 }) } else { ramp(c, (noise2((x * 8.0) as i32, (y * 8.0) as i32, seed) - 0.5) * 0.03) }
        }
        WallFinish::Brick { c, mortar } => {
            let row = (y / 0.075).floor() as i32;
            let off = if row % 2 == 0 { 0.0 } else { 0.11 };
            let u = (x + off) / 0.22;
            let mort = (y / 0.075).fract() < 0.18 || u.fract() < 0.06;
            if mort { mortar } else { ramp(c, (noise2(u.floor() as i32, row, seed) - 0.5) * 0.3) }
        }
        WallFinish::Slats { wood, back } => {
            let u = (x / 0.12).fract();
            if u < 0.28 { back } else { ramp(wood, (noise2((x / 0.12).floor() as i32, 3, seed) - 0.5) * 0.2 + if u > 0.9 { -0.15 } else { 0.0 }) }
        }
        WallFinish::Tiles { c, grout } => {
            let (u, v) = ((x / 0.6).fract(), (y / 0.6).fract());
            if u < 0.04 || v < 0.04 { grout } else { ramp(c, (noise2((x / 0.6) as i32, (y / 0.6) as i32, seed) - 0.5) * 0.1) }
        }
        WallFinish::Stripes { a, b } => {
            let d = x * 0.35 + y;
            let band = (d / 1.2).rem_euclid(3.0);
            if band < 0.9 { b } else if band < 1.0 { accent } else { a }
        }
        WallFinish::Acoustic { a, b } => {
            let (i, j) = ((x / 0.6).floor() as i32, (y / 0.6).floor() as i32);
            let (u, v) = ((x / 0.6).fract(), (y / 0.6).fract());
            if u < 0.06 || v < 0.06 { ramp(a, -0.4) } else if (i + j) % 3 == 0 { ramp(b, (v - 0.5) * 0.15) } else { ramp(a, (v - 0.5) * 0.15) }
        }
    }
}

/// A screen's picture: code in the tool's colours, a chat, an error, a frozen frame, or off.
pub fn screen_img(kind: &str, accent: Rgba, seed: u32) -> Img {
    let (w, h) = (40, 26);
    let mut im = Img::new(w, h);
    let mut r = Rng(seed | 1);
    match kind {
        "off" => { im.rect(0, 0, w, h, hex("#15161b")); im.rect(2, 2, 6, 1, hex("#2a2c34")); }
        "error" => {
            im.rect(0, 0, w, h, hex("#5a1418"));
            for y in (3..h - 2).step_by(3) { let l = r.int(20) as i32 + 8; im.rect(3, y, l, 1, hex("#ff8a80")); }
            im.rect(w / 2 - 4, h / 2 - 5, 8, 10, hex("#ffdedb"));
            im.rect(w / 2 - 1, h / 2 - 3, 2, 4, hex("#c62828")); im.rect(w / 2 - 1, h / 2 + 2, 2, 2, hex("#c62828"));
        }
        "frozen" => {
            im.rect(0, 0, w, h, hex("#3a3c44"));
            for y in (3..h - 2).step_by(3) { let l = r.int(22) as i32 + 6; im.rect(3, y, l, 1, hex("#6e717c")); }
            im.rect(w - 9, h - 7, 6, 4, hex("#e0b84a"));
        }
        "chat" => {
            im.rect(0, 0, w, h, hex("#f4efe6"));
            let mut y = 2;
            while y < h - 4 {
                let right = r.chance(0.5);
                let l = r.int(14) as i32 + 10;
                let x = if right { w - 2 - l } else { 2 };
                im.rect(x, y, l, 3, if right { mix(accent, hex("#ffffff"), 0.35) } else { hex("#dcd6cc") });
                y += 5;
            }
        }
        "done" => {
            im.rect(0, 0, w, h, hex("#16261d"));
            for y in (3..h - 2).step_by(3) { let l = r.int(20) as i32 + 6; im.rect(3, y, l, 1, hex("#5fae7d")); }
            im.rect(w - 12, 4, 8, 8, hex("#5fd38d"));
        }
        _ => {
            // code: a dark editor with indented coloured lines (twice the height, so it can scroll)
            im = Img::new(w, h * 2);
            im.rect(0, 0, w, h * 2, hex("#1b1d24"));
            im.rect(0, 0, 3, h * 2, hex("#262833"));
            let cols = [mix(accent, hex("#ffffff"), 0.3), hex("#e6c07b"), hex("#98c379"), hex("#c8ccd4"), hex("#61afef")];
            let mut y = 1;
            while y < h * 2 - 1 {
                let indent = (r.int(4) * 2) as i32;
                let mut x = 5 + indent;
                for _ in 0..(1 + r.int(3)) {
                    let l = r.int(8) as i32 + 2;
                    if x + l > w - 2 { break; }
                    im.rect(x, y, l, 1, *r.pick(&cols));
                    x += l + 2;
                }
                y += 2;
            }
        }
    }
    im
}

pub fn sign_img(mono: &str, name: &str, accent: Rgba, style: &str) -> Img {
    let tw = text_width(name, 1);
    let (w, h) = (tw + 22, 13);
    let mut im = Img::new(w, h);
    let (bg, fg) = match style { "neon" => (hex("#141420"), mix(accent, hex("#ffffff"), 0.45)), "light" => (hex("#f6f1e8"), hex("#2a2530")), _ => (hex("#26222e"), hex("#f4efe6")) };
    im.rect(0, 0, w, h, bg);
    im.rect(0, h - 2, w, 2, accent);
    im.rect(2, 2, 15, 9, accent);
    draw_text(&mut im, 4, 3, mono, hex("#ffffff"), 1);
    draw_text(&mut im, 19, 3, name, fg, 1);
    im
}

pub fn text_img(lines: &[(&str, Rgba)], bg: Rgba, pad: i32) -> Img {
    let w = lines.iter().map(|(s, _)| text_width(s, 1)).max().unwrap_or(0) + pad * 2;
    let h = lines.len() as i32 * 9 + pad * 2 - 2;
    let mut im = Img::new(w, h);
    im.rect(0, 0, w, h, bg);
    for (i, (s, c)) in lines.iter().enumerate() { draw_text(&mut im, pad, pad + i as i32 * 9, s, *c, 1); }
    im
}

pub fn books_img(seed: u32) -> Img {
    let mut im = Img::new(64, 16);
    let mut r = Rng(seed | 1);
    let cols = ["#a4543a", "#3a5a7a", "#d9b44a", "#4a7a5a", "#7a3b5a", "#e6dccc", "#2f2f3a", "#c86f4f"];
    let mut x = 0;
    while x < 64 {
        let w = 2 + r.int(3) as i32;
        let h = 10 + r.int(6) as i32;
        let c = hex(r.pick(&cols));
        im.rect(x, 16 - h, w, h, c);
        im.rect(x, 16 - h + 2, w, 1, ramp(c, 0.25));
        x += w;
    }
    im
}

pub fn poster_img(seed: u32, accent: Rgba) -> Img {
    let mut im = Img::new(24, 32);
    let mut r = Rng(seed | 1);
    let pal = [hex("#f2e8d5"), hex("#e07a5f"), hex("#3d405b"), hex("#81b29a"), hex("#f2cc8f"), accent];
    im.rect(0, 0, 24, 32, pal[0]);
    match r.int(3) {
        0 => { for i in 0..4 { im.disc(6.0 + i as f32 * 4.0, 10.0 + i as f32 * 4.0, 5.0, pal[1 + i % 5]); } }
        1 => { for i in 0..5 { im.rect(3, 4 + i * 5, 6 + r.int(14) as i32, 3, pal[1 + (i as usize) % 5]); } }
        _ => { im.disc(12.0, 13.0, 8.0, pal[4]); im.rect(0, 20, 24, 12, pal[2]); im.rect(4, 24, 16, 2, pal[5]); }
    }
    im.outline_rect(0, 0, 24, 32, hex("#2a2530"));
    im
}

pub fn rug_img(c: Rgba, accent: Rgba, seed: u32) -> Img {
    let mut im = Img::new(48, 36);
    let mut r = Rng(seed | 1);
    im.rect(0, 0, 48, 36, c);
    im.outline_rect(2, 2, 44, 32, ramp(c, 0.25));
    match r.int(3) {
        0 => { for i in 0..5 { im.rect(6, 7 + i * 5, 36, 2, if i % 2 == 0 { ramp(c, -0.2) } else { accent }); } }
        1 => { for y in 0..36 { for x in 0..48 { if (x / 6 + y / 6) % 2 == 0 && x > 4 && x < 44 && y > 4 && y < 32 { im.blend(x, y, [0, 0, 0, 30]); } } } }
        _ => { im.disc(24.0, 18.0, 10.0, ramp(c, 0.2)); im.disc(24.0, 18.0, 5.0, accent); }
    }
    im
}

pub fn skyline_img() -> Img {
    let mut im = Img::new(16, 32);
    im.rect(0, 0, 16, 32, hex("#2c2a4a"));
    for y in (2..32).step_by(4) { for x in (2..16).step_by(4) {
        let lit = noise2(x, y, 11) > 0.55;
        im.rect(x, y, 2, 2, if lit { hex("#ffd58a") } else { hex("#3d3a62") });
    } }
    im
}

pub fn board_img(seed: u32) -> Img {
    let mut im = Img::new(48, 28);
    let mut r = Rng(seed | 1);
    im.rect(0, 0, 48, 28, hex("#f4f4f0"));
    for _ in 0..6 { let (x, y) = (r.int(36) as i32 + 4, r.int(20) as i32 + 3); im.rect(x, y, r.int(10) as i32 + 4, 1, *r.pick(&[hex("#2f6fbf"), hex("#c0392b"), hex("#2e8b57"), hex("#333333")])); }
    for _ in 0..3 { let (x, y) = (r.int(38) as i32 + 4, r.int(18) as i32 + 3); im.rect(x, y, 5, 5, *r.pick(&[hex("#ffe27a"), hex("#ffb3c7"), hex("#a8e6cf")])); }
    im.outline_rect(0, 0, 48, 28, hex("#9aa0a8"));
    im
}

pub fn pegboard_img() -> Img {
    let mut im = Img::new(24, 24);
    im.rect(0, 0, 24, 24, hex("#c9a46c"));
    for y in (2..24).step_by(4) { for x in (2..24).step_by(4) { im.rect(x, y, 1, 1, hex("#6b5638")); } }
    im
}

pub fn rack_img() -> Img {
    let mut im = Img::new(20, 40);
    im.rect(0, 0, 20, 40, hex("#1c1d22"));
    for y in (2..38).step_by(5) { im.rect(2, y, 16, 4, hex("#2a2c34")); im.rect(3, y + 1, 2, 1, hex("#5fd38d")); im.rect(6, y + 1, 1, 1, hex("#e0b84a")); }
    im
}

// ---------- the overlay's atlas: glyphs and balloon icons ----------
pub const GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789·-.!?:/%+ ";

pub struct Ui { pub img: Img, pub glyph: HashMap<char, ([f32; 4], i32)>, pub icon: HashMap<&'static str, ([f32; 4], i32, i32)>, pub white: [f32; 4] }

pub fn ui_atlas() -> Ui {
    let mut a = Atlas::new(512, 128);
    let mut glyph = HashMap::new();
    for c in GLYPHS.chars() {
        let w = text_width(&c.to_string(), 1).max(3);
        let mut im = Img::new(w, 7);
        draw_text(&mut im, 0, 0, &c.to_string(), [255, 255, 255, 255], 1);
        glyph.insert(c, (a.add_exact(&format!("g{c}"), &im), w));
    }
    let mut white = Img::new(4, 4);
    white.rect(0, 0, 4, 4, [255, 255, 255, 255]);
    let wr = a.add_exact("white", &white);
    let mut icon = HashMap::new();
    let balloon = |mark: &str, bg: Rgba, fg: Rgba| -> Img {
        let mut im = Img::new(13, 15);
        im.rect(1, 0, 11, 11, hex("#1d1a24"));
        im.rect(0, 1, 13, 9, hex("#1d1a24"));
        im.rect(2, 1, 9, 9, bg);
        im.rect(1, 2, 11, 7, bg);
        im.rect(5, 11, 3, 2, hex("#1d1a24")); im.rect(6, 13, 1, 2, hex("#1d1a24"));
        im.rect(6, 11, 1, 1, bg);
        let w = text_width(mark, 1);
        draw_text(&mut im, (13 - w) / 2, 2, mark, fg, 1);
        im
    };
    for (k, mark, bg, fg) in [("asks", "!", "#ffd34d", "#2a2000"), ("stuck", "?", "#f0a640", "#2a1600"), ("done", "+", "#5fd38d", "#0c2a18"), ("error", "X", "#ef5350", "#ffffff"), ("sleep", "Z", "#9fb4e8", "#1a2240"), ("heart", "+", "#ff8fb1", "#401020")] {
        let im = balloon(mark, hex(bg), hex(fg));
        icon.insert(k, (a.add_exact(k, &im), im.w, im.h));
    }
    // a tick for "done"
    let mut tick = balloon(" ", hex("#5fd38d"), hex("#0c2a18"));
    for (x, y) in [(3, 5), (4, 6), (5, 7), (6, 6), (7, 5), (8, 4), (9, 3)] { tick.rect(x, y, 1, 1, hex("#0c2a18")); tick.rect(x, y + 1, 1, 1, hex("#0c2a18")); }
    icon.insert("done", (a.add_exact("tick", &tick), tick.w, tick.h));
    Ui { img: a.img, glyph, icon, white: wr }
}
