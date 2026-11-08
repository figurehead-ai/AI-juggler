//! The Floor's textures, painted once into one atlas: each area's floor
//! and wall, screens, signs, book spines, posters, rugs, boards and the
//! skyline's windows. Models are flat-coloured; textures carry only what
//! geometry can't (patterns and text). A second small atlas holds the
//! overlay's glyphs and balloon icons. Nothing is bundled.

use std::collections::HashMap;

use crate::colour::{Rgba, hex, mix, ramp};
use crate::kit::{FloorMat, WallFinish};
use crate::paint::{Img, draw_text, noise2, text_width};
use crate::rng::Rng;

/// Texture pixels per metre.
pub const PX_PER_M: f32 = 24.0;

/// A texture's place in an atlas: `(u, v, du, dv)`.
pub type Uv = [f32; 4];

/// The atlas has no room left for a texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Full;

/// Many textures in one image, packed in shelves.
#[derive(Clone, Debug)]
pub struct Atlas {
    /// The image.
    pub img: Img,
    x: i32,
    y: i32,
    shelf: i32,
    map: HashMap<String, Uv>,
}

impl Atlas {
    /// An empty atlas.
    #[must_use]
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            img: Img::new(w, h),
            x: 0,
            y: 0,
            shelf: 0,
            map: HashMap::new(),
        }
    }

    /// Adds `im` under `key` (once); its place, inset half a texel so
    /// tiling never bleeds.
    ///
    /// # Errors
    ///
    /// [`Full`] when it doesn't fit.
    pub fn add(&mut self, key: &str, im: &Img) -> Result<Uv, Full> {
        if let Some(r) = self.map.get(key) {
            return Ok(*r);
        }
        if self.x + im.w + 2 > self.img.w {
            self.x = 0;
            self.y += self.shelf + 2;
            self.shelf = 0;
        }
        if self.y + im.h + 2 > self.img.h || im.w + 2 > self.img.w {
            return Err(Full);
        }
        let (x, y) = (self.x + 1, self.y + 1);
        self.img.blit(im, x, y);
        self.x += im.w + 2;
        self.shelf = self.shelf.max(im.h);
        let (aw, ah) = (self.img.w as f32, self.img.h as f32);
        let r = [
            (x as f32 + 0.5) / aw,
            (y as f32 + 0.5) / ah,
            (im.w as f32 - 1.0) / aw,
            (im.h as f32 - 1.0) / ah,
        ];
        self.map.insert(key.to_owned(), r);
        Ok(r)
    }

    /// Adds `im` addressed to its edges (glyphs and icons, never tiled).
    ///
    /// # Errors
    ///
    /// [`Full`] when it doesn't fit.
    pub fn add_exact(&mut self, key: &str, im: &Img) -> Result<Uv, Full> {
        let r = self.add(key, im)?;
        let (aw, ah) = (self.img.w as f32, self.img.h as f32);
        Ok([
            r[0] - 0.5 / aw,
            r[1] - 0.5 / ah,
            r[2] + 1.0 / aw,
            r[3] + 1.0 / ah,
        ])
    }

    /// The rows in use: everything added sits above this.
    #[must_use]
    pub fn used(&self) -> i32 {
        (self.y + self.shelf + 2).min(self.img.h)
    }

    /// The image cut to the rows in use, rounded up to a power of two
    /// (so the places it gave, scaled by the factor returned, stay
    /// exact), and that factor: a place's v and height times it.
    #[must_use]
    pub fn trimmed(self) -> (Img, f32) {
        let h = u32::try_from(self.used().max(1))
            .unwrap_or(1)
            .next_power_of_two()
            .min(u32::try_from(self.img.h).unwrap_or(1));
        let h = i32::try_from(h).unwrap_or(self.img.h);
        let factor = self.img.h as f32 / h as f32;
        let mut img = self.img;
        img.px.truncate((img.w * h * 4) as usize);
        img.h = h;
        (img, factor)
    }

    /// The place of `key`, when added.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Uv> {
        self.map.get(key).copied()
    }
}

/// How far a floor material's pattern repeats, metres.
#[must_use]
pub fn floor_tile(m: &FloorMat) -> f32 {
    match m {
        FloorMat::Concrete { .. } => 6.0,
        FloorMat::Carpet { .. } | FloorMat::Terrazzo { .. } => 4.0,
        FloorMat::Planks { .. } | FloorMat::Parquet { .. } | FloorMat::Rubber { .. } => 4.8,
    }
}

/// One tile of a floor material.
#[must_use]
pub fn floor_img(m: &FloorMat, seed: u32) -> Img {
    let n = (floor_tile(m) * PX_PER_M) as i32;
    let mut im = Img::new(n, n);
    for y in 0..n {
        for x in 0..n {
            im.blend(
                x,
                y,
                floor_colour(m, x as f32 / PX_PER_M, y as f32 / PX_PER_M, seed),
            );
        }
    }
    im
}

/// One tile (4.8 × 3 m) of a wall finish; `accent` for the stripes.
#[must_use]
pub fn wall_img(w: &WallFinish, seed: u32, accent: Rgba) -> Img {
    let (wm, hm) = (4.8, 3.0);
    let (nw, nh) = ((wm * PX_PER_M) as i32, (hm * PX_PER_M) as i32);
    let mut im = Img::new(nw, nh);
    for y in 0..nh {
        for x in 0..nw {
            im.blend(
                x,
                y,
                wall_colour(
                    w,
                    x as f32 / PX_PER_M,
                    hm - y as f32 / PX_PER_M,
                    seed,
                    accent,
                ),
            );
        }
    }
    im
}

fn grain(x: f32, z: f32, k: f32, seed: u32) -> f32 {
    noise2((x * k) as i32, (z * k) as i32, seed) - 0.5
}

fn floor_colour(m: &FloorMat, x: f32, z: f32, seed: u32) -> Rgba {
    match *m {
        FloorMat::Planks { wood, width } => {
            let row = (z / width).floor() as i32;
            let len = 1.6;
            let u = (x + row.rem_euclid(3) as f32 * len / 3.0) / len;
            let tone = (noise2(row, (u.floor() as i32).rem_euclid(3), seed) - 0.5) * 0.22;
            let seam = u.fract() < 0.02 || (z / width).fract() < 0.07;
            let fibre = (noise2((x * 40.0) as i32, row, seed ^ 9) - 0.5) * 0.06;
            ramp(wood, if seam { -0.32 } else { tone + fibre })
        }
        FloorMat::Parquet { wood, block } => {
            let (i, j) = ((x / block).floor() as i32, (z / block).floor() as i32);
            let (u, v) = ((x / block).fract(), (z / block).fract());
            let along_x = (i + j) % 2 == 0;
            let strip = if along_x { v } else { u } * 4.0;
            let seam = strip.fract() < 0.1 || u < 0.03 || v < 0.03;
            let tone = (noise2(i * 7 + strip.floor() as i32, j, seed) - 0.5) * 0.25;
            ramp(
                wood,
                if seam {
                    -0.3
                } else {
                    tone + if along_x { 0.05 } else { -0.05 }
                },
            )
        }
        FloorMat::Carpet { a, b, tile } => {
            let (i, j) = ((x / tile).floor() as i32, (z / tile).floor() as i32);
            let (u, v) = ((x / tile).fract(), (z / tile).fract());
            let even = (i + j) % 2 == 0;
            let line = if even {
                (u * 9.0).fract() < 0.3
            } else {
                (v * 9.0).fract() < 0.3
            };
            ramp(
                if even { a } else { b },
                if line { -0.08 } else { 0.0 } + grain(x, z, 30.0, seed) * 0.1,
            )
        }
        FloorMat::Concrete { tone, seam } => {
            let blot = (noise2((x / 2.0).floor() as i32, (z / 2.0).floor() as i32, seed) - 0.5)
                * 0.1
                + grain(x, z, 2.0, seed ^ 3) * 0.06;
            let speck = noise2((x * 45.0) as i32, (z * 45.0) as i32, seed ^ 5) > 0.93;
            let joint = (x / seam).fract() < 0.01 || (z / seam).fract() < 0.012;
            ramp(
                tone,
                if joint {
                    -0.35
                } else {
                    blot + if speck { -0.12 } else { 0.0 }
                },
            )
        }
        FloorMat::Terrazzo { base, chips } => {
            let (i, j) = ((x / 0.05).floor() as i32, (z / 0.05).floor() as i32);
            let n = noise2(i, j, seed);
            if n > 0.86 {
                chips[((n * 100.0) as usize) % 3]
            } else if n > 0.8 {
                ramp(base, -0.25)
            } else {
                ramp(base, (noise2(i / 8, j / 8, seed ^ 2) - 0.5) * 0.06)
            }
        }
        FloorMat::Rubber { c } => {
            let (u, v) = ((x / 0.12).fract(), (z / 0.12).fract());
            let stud = (u - 0.5).powi(2) + (v - 0.5).powi(2) < 0.06;
            ramp(
                c,
                if stud {
                    0.12
                } else {
                    grain(x, z, 20.0, seed) * 0.05
                },
            )
        }
    }
}

fn wall_colour(w: &WallFinish, x: f32, y: f32, seed: u32, accent: Rgba) -> Rgba {
    match *w {
        WallFinish::Paint { c, dado } => {
            if y < 1.0 {
                ramp(dado, if (y - 0.98).abs() < 0.03 { 0.2 } else { -0.05 })
            } else {
                ramp(c, grain(x, y, 8.0, seed) * 0.03)
            }
        }
        WallFinish::Brick { c, mortar } => {
            let row = (y / 0.075).floor() as i32;
            let u = (x + if row % 2 == 0 { 0.0 } else { 0.11 }) / 0.22;
            if (y / 0.075).fract() < 0.18 || u.fract() < 0.06 {
                mortar
            } else {
                ramp(c, (noise2(u.floor() as i32, row, seed) - 0.5) * 0.3)
            }
        }
        WallFinish::Slats { wood, back } => {
            let u = (x / 0.12).fract();
            if u < 0.28 {
                back
            } else {
                let tone = (noise2((x / 0.12).floor() as i32, 3, seed) - 0.5) * 0.2;
                ramp(wood, tone + if u > 0.9 { -0.15 } else { 0.0 })
            }
        }
        WallFinish::Tiles { c, grout } => {
            let (u, v) = ((x / 0.6).fract(), (y / 0.6).fract());
            if u < 0.04 || v < 0.04 {
                grout
            } else {
                ramp(
                    c,
                    (noise2((x / 0.6) as i32, (y / 0.6) as i32, seed) - 0.5) * 0.1,
                )
            }
        }
        WallFinish::Stripes { a, b } => {
            let band = ((x * 0.35 + y) / 1.2).rem_euclid(3.0);
            if band < 0.9 {
                b
            } else if band < 1.0 {
                accent
            } else {
                a
            }
        }
        WallFinish::Acoustic { a, b } => {
            let (i, j) = ((x / 0.6).floor() as i32, (y / 0.6).floor() as i32);
            let (u, v) = ((x / 0.6).fract(), (y / 0.6).fract());
            if u < 0.06 || v < 0.06 {
                ramp(a, -0.4)
            } else {
                ramp(if (i + j) % 3 == 0 { b } else { a }, (v - 0.5) * 0.15)
            }
        }
    }
}

/// What a screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenPic {
    /// Code scrolling, in the App's colours (working).
    Code,
    /// A chat (waiting on you).
    Chat,
    /// An error (crashed).
    Error,
    /// A frozen frame (stuck).
    Frozen,
    /// Off (out of usage).
    Off,
    /// A green tick (done).
    Done,
}

fn lines(
    im: &mut Img,
    r: &mut Rng,
    (from, to): (i32, i32),
    (base, spread): (usize, usize),
    c: Rgba,
) {
    for y in (from..to).step_by(3) {
        im.rect(3, y, (r.int(spread) + base) as i32, 1, c);
    }
}

/// A screen's picture (40 × 26; code is twice as tall, so it can scroll).
#[must_use]
pub fn screen_img(pic: ScreenPic, accent: Rgba, seed: u32) -> Img {
    let (w, h) = (40, 26);
    let mut im = Img::new(w, h);
    let mut r = Rng(seed | 1);
    match pic {
        ScreenPic::Off => {
            im.rect(0, 0, w, h, hex("#15161b"));
            im.rect(2, 2, 6, 1, hex("#2a2c34"));
        }
        ScreenPic::Error => {
            im.rect(0, 0, w, h, hex("#5a1418"));
            lines(&mut im, &mut r, (3, h - 2), (8, 20), hex("#ff8a80"));
            im.rect(w / 2 - 4, h / 2 - 5, 8, 10, hex("#ffdedb"));
            im.rect(w / 2 - 1, h / 2 - 3, 2, 4, hex("#c62828"));
            im.rect(w / 2 - 1, h / 2 + 2, 2, 2, hex("#c62828"));
        }
        ScreenPic::Frozen => {
            im.rect(0, 0, w, h, hex("#3a3c44"));
            lines(&mut im, &mut r, (3, h - 2), (6, 22), hex("#6e717c"));
            im.rect(w - 9, h - 7, 6, 4, hex("#e0b84a"));
        }
        ScreenPic::Chat => {
            im.rect(0, 0, w, h, hex("#f4efe6"));
            let mut y = 2;
            while y < h - 4 {
                let right = r.chance(0.5);
                let l = r.int(14) as i32 + 10;
                let bubble = if right {
                    mix(accent, hex("#ffffff"), 0.35)
                } else {
                    hex("#dcd6cc")
                };
                im.rect(if right { w - 2 - l } else { 2 }, y, l, 3, bubble);
                y += 5;
            }
        }
        ScreenPic::Done => {
            im.rect(0, 0, w, h, hex("#16261d"));
            lines(&mut im, &mut r, (3, h - 2), (6, 20), hex("#5fae7d"));
            im.rect(w - 12, 4, 8, 8, hex("#5fd38d"));
        }
        ScreenPic::Code => {
            im = Img::new(w, h * 2);
            im.rect(0, 0, w, h * 2, hex("#1b1d24"));
            im.rect(0, 0, 3, h * 2, hex("#262833"));
            let cols = [
                mix(accent, hex("#ffffff"), 0.3),
                hex("#e6c07b"),
                hex("#98c379"),
                hex("#c8ccd4"),
                hex("#61afef"),
            ];
            for y in (1..h * 2 - 1).step_by(2) {
                let mut x = 5 + (r.int(4) * 2) as i32;
                for _ in 0..=r.int(3) {
                    let l = r.int(8) as i32 + 2;
                    if x + l > w - 2 {
                        break;
                    }
                    im.rect(x, y, l, 1, *r.pick(&cols));
                    x += l + 2;
                }
            }
        }
    }
    im
}

/// How a sign is lit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignStyle {
    /// Light letters on dark.
    Dark,
    /// Dark letters on light.
    Light,
    /// Glowing letters.
    Neon,
}

/// An area's sign: its mark's letters in the App's colour, then its name.
#[must_use]
pub fn sign_img(mono: &str, name: &str, accent: Rgba, style: SignStyle) -> Img {
    let (w, h) = (text_width(name, 1) + 22, 13);
    let mut im = Img::new(w, h);
    let (bg, fg) = match style {
        SignStyle::Neon => (hex("#141420"), mix(accent, hex("#ffffff"), 0.45)),
        SignStyle::Light => (hex("#f6f1e8"), hex("#2a2530")),
        SignStyle::Dark => (hex("#26222e"), hex("#f4efe6")),
    };
    im.rect(0, 0, w, h, bg);
    im.rect(0, h - 2, w, 2, accent);
    im.rect(2, 2, 15, 9, accent);
    draw_text(&mut im, 4, 3, mono, hex("#ffffff"), 1);
    draw_text(&mut im, 19, 3, name, fg, 1);
    im
}

/// The floor logo the Floor writes itself until the main window hands it
/// the tool's own (ADR-0026 §7): `mark`'s letters with the Account's
/// initials, in a ring of the App's colour.
#[must_use]
pub fn logo_img(mark: &str, initials: &str, accent: Rgba) -> Img {
    let n = 112;
    let mut im = Img::new(n, n);
    let c = n as f32 / 2.0;
    im.disc(c, c, 54.0, accent);
    im.disc(c, c, 48.0, mix(accent, hex("#ffffff"), 0.82));
    im.disc(c, c, 44.0, accent);
    draw_text(
        &mut im,
        (n - text_width(mark, 5)) / 2,
        22,
        mark,
        hex("#ffffff"),
        5,
    );
    let k = 3;
    let iw = text_width(initials, k);
    let (px, py) = ((n - iw) / 2 - 5, 66);
    if !initials.is_empty() {
        im.rect(px, py, iw + 10, 7 * k + 8, hex("#ffffff"));
        draw_text(&mut im, px + 5, py + 4, initials, accent, k);
    }
    im
}

/// A block of text lines on a background (the lobby's boards).
#[must_use]
pub fn text_img(rows: &[(&str, Rgba)], bg: Rgba, pad: i32) -> Img {
    let w = rows
        .iter()
        .map(|(s, _)| text_width(s, 1))
        .max()
        .unwrap_or(0)
        + pad * 2;
    let h = i32::try_from(rows.len()).unwrap_or(0) * 9 + pad * 2 - 2;
    let mut im = Img::new(w, h);
    im.rect(0, 0, w, h, bg);
    for (i, (s, c)) in (0_i32..).zip(rows) {
        draw_text(&mut im, pad, pad + i * 9, s, *c, 1);
    }
    im
}

/// A shelf's book spines.
#[must_use]
pub fn books_img(seed: u32) -> Img {
    let mut im = Img::new(64, 16);
    let mut r = Rng(seed | 1);
    let cols = [
        "#a4543a", "#3a5a7a", "#d9b44a", "#4a7a5a", "#7a3b5a", "#e6dccc", "#2f2f3a", "#c86f4f",
    ];
    let mut x = 0;
    while x < 64 {
        let (w, h) = (2 + r.int(3) as i32, 10 + r.int(6) as i32);
        let c = hex(r.pick(&cols));
        im.rect(x, 16 - h, w, h, c);
        im.rect(x, 16 - h + 2, w, 1, ramp(c, 0.25));
        x += w;
    }
    im
}

/// A poster, in one of three compositions.
#[must_use]
pub fn poster_img(seed: u32, accent: Rgba) -> Img {
    let mut im = Img::new(24, 32);
    let mut r = Rng(seed | 1);
    let pal = [
        hex("#f2e8d5"),
        hex("#e07a5f"),
        hex("#3d405b"),
        hex("#81b29a"),
        hex("#f2cc8f"),
        accent,
    ];
    im.rect(0, 0, 24, 32, pal[0]);
    match r.int(3) {
        0 => {
            for i in 0..4 {
                im.disc(
                    6.0 + i as f32 * 4.0,
                    10.0 + i as f32 * 4.0,
                    5.0,
                    pal[1 + i % 5],
                );
            }
        }
        1 => {
            for i in 0..5 {
                im.rect(
                    3,
                    4 + i * 5,
                    6 + r.int(14) as i32,
                    3,
                    pal[1 + i as usize % 5],
                );
            }
        }
        _ => {
            im.disc(12.0, 13.0, 8.0, pal[4]);
            im.rect(0, 20, 24, 12, pal[2]);
            im.rect(4, 24, 16, 2, pal[5]);
        }
    }
    im.outline_rect(0, 0, 24, 32, hex("#2a2530"));
    im
}

/// A rug in the App's colours.
#[must_use]
pub fn rug_img(c: Rgba, accent: Rgba, seed: u32) -> Img {
    let mut im = Img::new(48, 36);
    let mut r = Rng(seed | 1);
    im.rect(0, 0, 48, 36, c);
    im.outline_rect(2, 2, 44, 32, ramp(c, 0.25));
    match r.int(3) {
        0 => {
            for i in 0..5 {
                im.rect(
                    6,
                    7 + i * 5,
                    36,
                    2,
                    if i % 2 == 0 { ramp(c, -0.2) } else { accent },
                );
            }
        }
        1 => {
            for y in 5..32 {
                for x in 5..44 {
                    if (x / 6 + y / 6) % 2 == 0 {
                        im.blend(x, y, [0, 0, 0, 30]);
                    }
                }
            }
        }
        _ => {
            im.disc(24.0, 18.0, 10.0, ramp(c, 0.2));
            im.disc(24.0, 18.0, 5.0, accent);
        }
    }
    im
}

/// The skyline's lit and dark windows.
#[must_use]
pub fn skyline_img() -> Img {
    let mut im = Img::new(16, 32);
    im.rect(0, 0, 16, 32, hex("#2c2a4a"));
    for y in (2..32).step_by(4) {
        for x in (2..16).step_by(4) {
            let lit = noise2(x, y, 11) > 0.55;
            im.rect(
                x,
                y,
                2,
                2,
                if lit { hex("#ffd58a") } else { hex("#3d3a62") },
            );
        }
    }
    im
}

/// A whiteboard with scribbles and notes.
#[must_use]
pub fn board_img(seed: u32) -> Img {
    let mut im = Img::new(48, 28);
    let mut r = Rng(seed | 1);
    im.rect(0, 0, 48, 28, hex("#f4f4f0"));
    for _ in 0..6 {
        let (x, y, l) = (
            r.int(36) as i32 + 4,
            r.int(20) as i32 + 3,
            r.int(10) as i32 + 4,
        );
        let pen = *r.pick(&[
            hex("#2f6fbf"),
            hex("#c0392b"),
            hex("#2e8b57"),
            hex("#333333"),
        ]);
        im.rect(x, y, l, 1, pen);
    }
    for _ in 0..3 {
        let (x, y) = (r.int(38) as i32 + 4, r.int(18) as i32 + 3);
        let note = *r.pick(&[hex("#ffe27a"), hex("#ffb3c7"), hex("#a8e6cf")]);
        im.rect(x, y, 5, 5, note);
    }
    im.outline_rect(0, 0, 48, 28, hex("#9aa0a8"));
    im
}

/// Pegboard.
#[must_use]
pub fn pegboard_img() -> Img {
    let mut im = Img::new(24, 24);
    im.rect(0, 0, 24, 24, hex("#c9a46c"));
    for y in (2..24).step_by(4) {
        for x in (2..24).step_by(4) {
            im.rect(x, y, 1, 1, hex("#6b5638"));
        }
    }
    im
}

/// A server rack's front.
#[must_use]
pub fn rack_img() -> Img {
    let mut im = Img::new(20, 40);
    im.rect(0, 0, 20, 40, hex("#1c1d22"));
    for y in (2..38).step_by(5) {
        im.rect(2, y, 16, 4, hex("#2a2c34"));
        im.rect(3, y + 1, 2, 1, hex("#5fd38d"));
        im.rect(6, y + 1, 1, 1, hex("#e0b84a"));
    }
    im
}

/// The overlay's characters.
pub const GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789·-.,!?:/%+ ";

/// A balloon over a desk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    /// `!`: asks you.
    Asks,
    /// `?`: stuck.
    Stuck,
    /// A tick: done.
    Done,
    /// `X`: crashed.
    Error,
    /// `Z`: out of usage.
    Sleep,
    /// A heart (the cat).
    Heart,
    /// An empty gauge: out of room (flags only).
    Out,
    /// A gauge a quarter full: low on room (flags only).
    Low,
    /// `!` on orange: the machine runs hot (flags only).
    Hot,
}

/// The overlay's atlas: glyphs, balloons and a white texel.
#[derive(Clone, Debug)]
pub struct Ui {
    /// The image.
    pub img: Img,
    /// Each glyph's place and width.
    pub glyph: HashMap<char, (Uv, i32)>,
    /// Each balloon's place and size.
    pub icon: HashMap<Icon, (Uv, i32, i32)>,
    /// AJ-E3515: each flag's badge (a square with the balloon's mark),
    /// its place; 9 × 9.
    pub badge: HashMap<Icon, Uv>,
    /// A plain white texel, for flat shapes.
    pub white: Uv,
}

fn balloon(mark: &str, bg: Rgba, fg: Rgba) -> Img {
    let ink = hex("#1d1a24");
    let mut im = Img::new(13, 15);
    im.rect(1, 0, 11, 11, ink);
    im.rect(0, 1, 13, 9, ink);
    im.rect(2, 1, 9, 9, bg);
    im.rect(1, 2, 11, 7, bg);
    im.rect(5, 11, 3, 2, ink);
    im.rect(6, 13, 1, 2, ink);
    im.rect(6, 11, 1, 1, bg);
    draw_text(&mut im, (13 - text_width(mark, 1)) / 2, 2, mark, fg, 1);
    im
}

/// Paints the overlay's atlas.
///
/// # Errors
///
/// [`Full`] if it outgrows its 512 × 128 image (a test holds it).
pub fn ui_atlas() -> Result<Ui, Full> {
    let mut a = Atlas::new(512, 128);
    let mut glyph = HashMap::new();
    for c in GLYPHS.chars() {
        let w = text_width(&c.to_string(), 1).max(3);
        let mut im = Img::new(w, 7);
        draw_text(&mut im, 0, 0, &c.to_string(), [255; 4], 1);
        glyph.insert(c, (a.add_exact(&format!("g{c}"), &im)?, w));
    }
    let mut white = Img::new(4, 4);
    white.rect(0, 0, 4, 4, [255; 4]);
    let white = a.add_exact("white", &white)?;
    let mut icon = HashMap::new();
    for (k, mark, bg, fg) in MARKS
        .into_iter()
        .chain([(Icon::Heart, "+", "#ff8fb1", "#401020")])
    {
        let im = balloon(mark, hex(bg), hex(fg));
        icon.insert(k, (a.add_exact(&format!("{k:?}"), &im)?, im.w, im.h));
    }
    let mut tick = balloon(" ", hex("#5fd38d"), hex("#0c2a18"));
    for (x, y) in [(3, 5), (4, 6), (5, 7), (6, 6), (7, 5), (8, 4), (9, 3)] {
        tick.rect(x, y, 1, 2, hex("#0c2a18"));
    }
    icon.insert(Icon::Done, (a.add_exact("tick", &tick)?, tick.w, tick.h));
    let mut badge = HashMap::new();
    for (k, mark, bg, fg) in MARKS.into_iter().chain([
        (Icon::Hot, "!", "#ff7a3d", "#2a0c00"),
        (Icon::Done, " ", "#5fd38d", "#0c2a18"),
        (Icon::Out, " ", "#f87171", "#2a0808"),
        (Icon::Low, " ", "#f59e0b", "#2a1600"),
    ]) {
        let mut im = Img::new(9, 9);
        im.rect(0, 0, 9, 9, hex(bg));
        if matches!(k, Icon::Out | Icon::Low) {
            // the app's room mark: a gauge's outline, empty when out, a
            // quarter full when low
            im.rect(1, 3, 7, 1, hex(fg));
            im.rect(1, 6, 7, 1, hex(fg));
            im.rect(1, 3, 1, 4, hex(fg));
            im.rect(7, 3, 1, 4, hex(fg));
            if k == Icon::Low {
                im.rect(2, 4, 2, 2, hex(fg));
            }
        } else if k == Icon::Done {
            for (x, y) in [(1, 4), (2, 5), (3, 6), (4, 5), (5, 4), (6, 3), (7, 2)] {
                im.rect(x, y, 1, 2, hex(fg));
            }
        } else {
            draw_text(&mut im, (9 - text_width(mark, 1)) / 2, 1, mark, hex(fg), 1);
        }
        badge.insert(k, a.add_exact(&format!("badge{k:?}"), &im)?);
    }
    Ok(Ui {
        img: a.img,
        glyph,
        icon,
        badge,
        white,
    })
}

/// The marks a balloon and a badge share: its icon, the letter, and the
/// background and letter colours. Sleep is asleep over the desk: out of
/// room, in its danger red.
const MARKS: [(Icon, &str, &str, &str); 4] = [
    (Icon::Asks, "!", "#ffd34d", "#2a2000"),
    (Icon::Stuck, "?", "#f0a640", "#2a1600"),
    (Icon::Error, "X", "#ef5350", "#ffffff"),
    (Icon::Sleep, "Z", "#f87171", "#2a0808"),
];
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{
        Atlas, Full, ScreenPic, SignStyle, floor_img, screen_img, sign_img, ui_atlas, wall_img,
    };
    use crate::colour::hex;
    use crate::family::Family;
    use crate::kit::{choose, kit_for};
    use crate::paint::Img;

    #[test]
    fn the_atlas_packs_once_per_key_and_says_when_full() {
        let mut a = Atlas::new(64, 64);
        let one = a.add("one", &Img::new(30, 30)).unwrap();
        assert_eq!(a.add("one", &Img::new(30, 30)).unwrap(), one);
        assert!(a.add("two", &Img::new(30, 30)).is_ok());
        assert_eq!(a.add("three", &Img::new(30, 40)), Err(Full));
        assert!(a.get("two").is_some());
        assert_eq!(a.get("nope"), None);
    }

    #[test]
    fn every_familys_floor_and_wall_paint_and_the_overlay_fits() {
        for family in Family::ALL {
            let kit = kit_for(family, choose(family, "app", 1), 2);
            let floor = floor_img(&kit.floor, 7);
            let wall = wall_img(&kit.wall, 7, kit.accent);
            assert!(
                floor.px.chunks(4).all(|p| p[3] == 255),
                "{family:?}: the floor is opaque"
            );
            assert!(
                wall.px.chunks(4).all(|p| p[3] == 255),
                "{family:?}: the wall is opaque"
            );
        }
        let ui = ui_atlas().unwrap();
        assert_eq!(ui.icon.len(), 6);
        assert!(ui.glyph.contains_key(&'A') && ui.glyph.contains_key(&'%'));
        for pic in [
            ScreenPic::Code,
            ScreenPic::Chat,
            ScreenPic::Error,
            ScreenPic::Frozen,
            ScreenPic::Off,
            ScreenPic::Done,
        ] {
            assert_eq!(screen_img(pic, hex("#4a7aff"), 3).w, 40);
        }
        assert!(sign_img("CC", "WORK", hex("#4a7aff"), SignStyle::Neon).w > 22);
    }
}
