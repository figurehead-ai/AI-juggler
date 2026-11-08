//! The Floor's materials (0.4.1 second pass, `docs/research/floor-04x.md`
//! §12): wood, fabric, carpet, metal, plastic, leaves, skin, hair, cloth,
//! denim, leather, glaze, plaster, rubber, paper and soil, each a small
//! tileable map **generated at run time** (nothing is bundled): how much
//! lighter or darker the surface is than its part's colour, how rough it
//! is, and which way its surface leans (a normal from a generated height).
//!
//! A part keeps its own colour; its material only adds the surface. The
//! maps are made once when the reworked look is first drawn, in one
//! texture array with every mip level, and shaded triplanar in the part's
//! own metric space, so a texture sticks to what it's on and keeps its
//! size however the part is scaled.

use std::f32::consts::{PI, TAU};

/// A material's number in an [`crate::models::Inst`]'s `mat` (0: none,
/// the flat colour as before).
pub type Mat = u32;

/// No material: the part's flat colour.
pub const NONE: Mat = 0;
/// Wood, the grain along x.
pub const WOOD: Mat = 1;
/// Woven upholstery.
pub const FABRIC: Mat = 2;
/// Loop-pile carpet.
pub const CARPET: Mat = 3;
/// Brushed metal.
pub const METAL: Mat = 4;
/// Satin plastic, laminate.
pub const PLASTIC: Mat = 5;
/// A leaf: midrib and veins (in the leaf's own unit space).
pub const LEAF: Mat = 6;
/// Skin.
pub const SKIN: Mat = 7;
/// Hair, the strands along the part's y.
pub const HAIR: Mat = 8;
/// Cotton jersey with folds (tops).
pub const CLOTH: Mat = 9;
/// Denim twill (trousers).
pub const DENIM: Mat = 10;
/// Leather.
pub const LEATHER: Mat = 11;
/// Glazed ceramic.
pub const CERAMIC: Mat = 12;
/// Plaster and paint (walls).
pub const PLASTER: Mat = 13;
/// Rubber.
pub const RUBBER: Mat = 14;
/// Paper and card.
pub const PAPER: Mat = 15;
/// Potting soil.
pub const SOIL: Mat = 16;
/// A face (eyes, brows, mouth drawn by the shader on skin; the part's
/// `tex` carries the expression, [`Face`]).
pub const FACE: Mat = 17;

/// How many materials have a map (all but [`NONE`] and [`FACE`]).
pub const LAYERS: u32 = 16;
/// A map's side, texels.
pub const SIZE: u32 = 256;
/// Mip levels down to 1 × 1.
pub const LEVELS: u32 = 9;

/// How a material is shaded, beside its map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kind {
    /// Metres one map covers (the leaf: its own length).
    pub tile: f32,
    /// Metal (its colour tints its reflection).
    pub metal: f32,
    /// Light wrapping round (skin, leaves: light through them).
    pub wrap: f32,
    /// A soft sheen at grazing angles (cloth).
    pub sheen: f32,
    /// Highlights along strands (hair, brushed metal).
    pub aniso: f32,
    /// Mapped in the part's unit space, not metres (a leaf, a face).
    pub unit: bool,
}

const fn k(tile: f32, metal: f32, wrap: f32, sheen: f32, aniso: f32, unit: bool) -> Kind {
    Kind {
        tile,
        metal,
        wrap,
        sheen,
        aniso,
        unit,
    }
}

/// Each material's kind, by number (0 and [`FACE`] included).
pub const KINDS: [Kind; 18] = [
    k(1.0, 0.0, 0.0, 0.0, 0.0, false),   // none
    k(0.9, 0.0, 0.0, 0.0, 0.0, false),   // wood
    k(0.06, 0.0, 0.0, 0.5, 0.0, false),  // fabric
    k(0.3, 0.0, 0.0, 0.35, 0.0, false),  // carpet
    k(0.5, 1.0, 0.0, 0.0, 0.6, false),   // metal
    k(0.25, 0.0, 0.0, 0.0, 0.0, false),  // plastic
    k(1.0, 0.0, 0.45, 0.0, 0.0, true),   // leaf
    k(0.12, 0.0, 0.35, 0.0, 0.0, false), // skin
    k(0.22, 0.0, 0.2, 0.0, 1.0, false),  // hair
    k(0.7, 0.0, 0.0, 0.45, 0.0, false),  // cloth
    k(0.1, 0.0, 0.0, 0.3, 0.0, false),   // denim
    k(0.18, 0.0, 0.0, 0.1, 0.0, false),  // leather
    k(0.35, 0.0, 0.0, 0.0, 0.0, false),  // ceramic
    k(1.2, 0.0, 0.0, 0.0, 0.0, false),   // plaster
    k(0.12, 0.0, 0.0, 0.0, 0.0, false),  // rubber
    k(0.25, 0.0, 0.1, 0.0, 0.0, false),  // paper
    k(0.3, 0.0, 0.0, 0.0, 0.0, false),   // soil
    k(1.0, 0.0, 0.35, 0.0, 0.0, true),   // face
];

/// The map layer a material reads (`None`: no map).
#[must_use]
pub const fn layer(m: Mat) -> Option<u32> {
    match m {
        1..=16 => Some(m - 1),
        FACE => Some(SKIN - 1),
        _ => None,
    }
}

/// A face's expression, packed into the part's `tex` and `misc.xy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// How open the eyes are (0 closed, 1 open, more: wide).
    pub open: f32,
    /// Brows' tilt: + worried (inner ends up), − cross.
    pub tilt: f32,
    /// Brows raised.
    pub raise: f32,
    /// Smile (+) or frown (−).
    pub smile: f32,
    /// The mouth open (0–1).
    pub mouth: f32,
    /// Eyes closed happily (arcs), not asleep.
    pub happy: bool,
}

impl Face {
    /// The face's `(tex, misc.xy)`.
    #[must_use]
    pub fn pack(self) -> ([f32; 4], [f32; 2]) {
        (
            [self.open, self.tilt, self.raise, self.smile],
            [self.mouth, if self.happy { 1.0 } else { 0.0 }],
        )
    }
}

// ---------------------------------------------------------------- noise

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(0x8da6_b343)
        .wrapping_add((y as u32).wrapping_mul(0xd816_3841))
        .wrapping_add(seed.wrapping_mul(0xcb1a_b31f));
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / 16_777_216.0
}

fn wrap(i: i32, p: i32) -> i32 {
    i.rem_euclid(p.max(1))
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gradient noise, periodic every `(px, py)` cells; about −1–1.
fn perlin(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let g = |i: i32, j: i32, dx: f32, dy: f32| {
        let a = unit(hash(wrap(i, px), wrap(j, py), seed)) * TAU;
        a.cos() * dx + a.sin() * dy
    };
    let (u, v) = (fade(fx), fade(fy));
    let n00 = g(ix, iy, fx, fy);
    let n10 = g(ix + 1, iy, fx - 1.0, fy);
    let n01 = g(ix, iy + 1, fx, fy - 1.0);
    let n11 = g(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
    let a = n00 + (n10 - n00) * u;
    let b = n01 + (n11 - n01) * u;
    (a + (b - a) * v) * 1.41
}

/// Fractal noise over `(u, v)` in 0–1, `(fx, fy)` cells at the first
/// octave, periodic over the tile; about −1–1.
fn fbm(u: f32, v: f32, fx: i32, fy: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm) = (0.0, 1.0, 0.0);
    let (mut cx, mut cy) = (fx, fy);
    for o in 0..octaves {
        sum += amp
            * perlin(
                u * cx as f32,
                v * cy as f32,
                cx,
                cy,
                seed.wrapping_add(o * 97),
            );
        norm += amp;
        amp *= 0.5;
        cx *= 2;
        cy *= 2;
    }
    sum / norm
}

/// Cellular noise: the distances to the nearest and second-nearest
/// points, `n` cells across the tile (one point each), in cell units.
fn worley(u: f32, v: f32, n: i32, seed: u32) -> (f32, f32) {
    let (x, y) = (u * n as f32, v * n as f32);
    let (cx, cy) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2) = (9.0f32, 9.0f32);
    for j in -1..=1 {
        for i in -1..=1 {
            let (gx, gy) = (cx + i, cy + j);
            let h = hash(wrap(gx, n), wrap(gy, n), seed);
            let px = gx as f32 + unit(h);
            let py = gy as f32 + unit(h.wrapping_mul(0x9e37_79b9));
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A round yarn's height across it, 0–1 over one thread.
fn yarn(f: f32) -> f32 {
    (1.0 - (2.0 * f - 1.0).powi(2)).max(0.0).sqrt()
}

// ------------------------------------------------------------ surfaces

/// One texel before its normal: height (0–1), colour multiplier, rough.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Texel {
    h: f32,
    alb: f32,
    rough: f32,
}

const fn tx(h: f32, alb: f32, rough: f32) -> Texel {
    Texel { h, alb, rough }
}

/// How steep a material's height reads (its bumps' strength).
const fn relief(m: Mat) -> f32 {
    match m {
        FABRIC | CARPET | HAIR => 5.0,
        SOIL => 6.0,
        DENIM => 4.5,
        LEAF => 3.0,
        LEATHER => 2.5,
        WOOD | PLASTER => 2.0,
        CLOTH => 1.7,
        RUBBER => 1.5,
        SKIN | PAPER => 1.2,
        METAL => 0.8,
        PLASTIC => 0.6,
        CERAMIC => 0.4,
        _ => 1.0,
    }
}

#[allow(clippy::too_many_lines, reason = "a recipe a material")]
fn texel(m: Mat, u: f32, v: f32) -> Texel {
    let s = m * 7919;
    match m {
        WOOD => {
            // Flat-sawn: rings as long stripes along the grain (x), gently
            // arched, fibres and open pores along them.
            let warp = fbm(u, v, 1, 2, 3, s) * 0.6 + fbm(u, v, 3, 6, 2, s + 4) * 0.12;
            let ring = (v * 11.0 + warp + (u * TAU).sin() * 0.12).rem_euclid(1.0);
            let late = smooth(0.55, 0.85, ring) * (1.0 - smooth(0.93, 1.0, ring));
            let fibre = fbm(u, v, 2, 110, 3, s + 1);
            let pore = smooth(0.5, 0.8, perlin(u * 16.0, v * 220.0, 16, 220, s + 2));
            let tone = fbm(u, v, 1, 2, 2, s + 3) * 0.07;
            tx(
                0.55 - 0.14 * late + 0.12 * fibre - 0.12 * pore,
                1.04 - 0.16 * late + 0.07 * fibre - 0.06 * pore + tone,
                0.5 + 0.14 * late + 0.1 * pore,
            )
        }
        FABRIC => {
            // A plain weave, 20 threads across, each yarn a little uneven.
            let n = 20.0;
            let (tu, tv) = (u * n, v * n);
            let (cu, cv) = (tu.floor() as i32, tv.floor() as i32);
            let (fu, fv) = (tu - cu as f32, tv - cv as f32);
            let warp_up = (cu + cv) & 1 == 0;
            let bend = |f: f32| 0.75 + 0.25 * (PI * (2.0 * f - 1.0)).cos();
            let warp = yarn(fu) * if warp_up { bend(fv) } else { 0.45 };
            let weft = yarn(fv) * if warp_up { 0.45 } else { bend(fu) };
            let fuzz = fbm(u, v, 40, 40, 2, s);
            let slub = (unit(hash(cu, 0, s)) - 0.5) * 0.1 + (unit(hash(0, cv, s + 1)) - 0.5) * 0.1;
            let h = warp.max(weft);
            tx(
                h * 0.85 + fuzz * 0.08,
                0.78 + 0.26 * h + slub + fuzz * 0.04,
                0.86 - 0.06 * h,
            )
        }
        CARPET => {
            let (f1, _) = worley(u, v, 44, s);
            let tuft = 1.0 - smooth(0.0, 0.62, f1);
            let blot = fbm(u, v, 3, 3, 3, s + 1);
            let fine = fbm(u, v, 64, 64, 1, s + 2);
            tx(
                tuft * 0.8 + fine * 0.12,
                0.8 + 0.22 * tuft + 0.07 * blot,
                0.95,
            )
        }
        METAL => {
            let streak = fbm(u, v, 2, 160, 3, s);
            let scratch = smooth(0.82, 0.95, perlin(u * 3.0, v * 60.0, 3, 60, s + 1).abs());
            tx(
                0.5 + 0.12 * streak - 0.15 * scratch,
                0.97 + 0.05 * streak + 0.06 * scratch,
                0.3 + 0.08 * streak - 0.06 * scratch,
            )
        }
        PLASTIC => {
            let peel = fbm(u, v, 20, 20, 3, s);
            let speck = smooth(0.6, 0.8, perlin(u * 64.0, v * 64.0, 64, 64, s + 1));
            tx(0.5 + 0.08 * peel, 1.0 - 0.03 * speck, 0.4 + 0.05 * peel)
        }
        LEAF => {
            // Across the leaf u (its midrib at 0.5), along it v.
            let d = (u - 0.5).abs();
            let midrib = 1.0 - smooth(0.0, 0.03, d);
            let sv = (v * 6.0 - d * 5.0 + d * d * 6.0).rem_euclid(1.0);
            let vein = (1.0 - smooth(0.0, 0.07, sv.min(1.0 - sv))) * smooth(0.03, 0.07, d);
            let (cell, _) = worley(u, v, 18, s);
            let areole = smooth(0.2, 0.6, cell);
            tx(
                0.45 + 0.4 * midrib + 0.18 * vein - 0.03 * areole,
                1.0 + 0.2 * midrib + 0.1 * vein - 0.02 * areole,
                0.42 + 0.1 * areole,
            )
        }
        SKIN => {
            let (f1, _) = worley(u, v, 70, s);
            let pore = 1.0 - smooth(0.0, 0.25, f1);
            let mottle = fbm(u, v, 4, 4, 3, s + 1);
            tx(
                0.5 - 0.14 * pore + 0.05 * mottle,
                1.0 + 0.035 * mottle - 0.03 * pore,
                0.56 + 0.06 * pore,
            )
        }
        HAIR => {
            let strand = fbm(u, v, 70, 3, 3, s);
            let clump = fbm(u, v, 9, 1, 2, s + 1);
            let h = 0.5 + 0.3 * strand + 0.2 * clump;
            tx(h, 0.82 + 0.3 * strand + 0.12 * clump, 0.42 - 0.1 * strand)
        }
        CLOTH => {
            // Folds hanging mostly downwards (along y), and a fine knit.
            let warp = fbm(u, v, 2, 2, 3, s) * 0.45;
            let drape = 0.5 + 0.5 * (TAU * (2.0 * u + warp)).sin();
            let crease = 1.0 - smooth(0.0, 0.5, ((TAU * (2.0 * u + 1.0 * v + warp)).sin()).abs());
            let knit = yarn((u * 110.0).fract()) * 0.5
                + yarn((v * 80.0 + (u * 110.0).floor() * 0.5).fract()) * 0.5;
            let h = 0.62 * drape - 0.07 * crease + 0.08 * knit;
            tx(h, 0.84 + 0.2 * drape - 0.03 * crease + 0.03 * knit, 0.86)
        }
        DENIM => {
            // A 3/1 twill: diagonal ribs; slubby indigo, white weft flecks.
            let n = 36.0;
            let d = ((u * n + v * n) / 3.0).fract();
            let rib = yarn(d);
            let slub = fbm(u, v, 3, 40, 3, s);
            let fleck = smooth(
                0.82,
                0.95,
                unit(hash((u * n) as i32, (v * n) as i32, s + 1)),
            );
            tx(
                0.4 + 0.45 * rib + 0.06 * slub,
                0.84 + 0.16 * rib + 0.14 * slub + 0.16 * fleck,
                0.86,
            )
        }
        LEATHER => {
            // Pebble grain: soft domes with fine creases between them.
            let (f1, f2) = worley(u, v, 34, s);
            let pebble = 1.0 - smooth(0.0, 0.75, f1);
            let crease = 1.0 - smooth(0.0, 0.08, f2 - f1);
            let fine = fbm(u, v, 30, 30, 2, s + 1);
            tx(
                0.35 + 0.45 * pebble - 0.08 * crease + 0.04 * fine,
                0.94 + 0.08 * pebble - 0.05 * crease + 0.03 * fine,
                0.5 - 0.12 * pebble,
            )
        }
        CERAMIC => {
            let (f1, f2) = worley(u, v, 6, s);
            let craze = 1.0 - smooth(0.0, 0.012, f2 - f1);
            let flow = fbm(u, v, 3, 3, 3, s + 1);
            tx(
                0.5 + 0.05 * flow - 0.02 * craze,
                1.0 + 0.04 * flow - 0.015 * craze,
                0.14 + 0.04 * craze,
            )
        }
        PLASTER => {
            let n = fbm(u, v, 6, 6, 5, s);
            let trowel = fbm(u, v, 2, 2, 2, s + 1);
            tx(
                0.5 + 0.22 * n + 0.1 * trowel,
                0.97 + 0.05 * n + 0.03 * trowel,
                0.88,
            )
        }
        RUBBER => {
            let n = fbm(u, v, 24, 24, 3, s);
            tx(0.5 + 0.15 * n, 1.0 + 0.04 * n, 0.9)
        }
        PAPER => {
            let fibres = fbm(u, v, 48, 12, 3, s) + fbm(u, v, 12, 48, 3, s + 1);
            tx(0.5 + 0.12 * fibres, 1.0 + 0.03 * fibres, 0.9)
        }
        SOIL => {
            let (f1, _) = worley(u, v, 22, s);
            let crumb = 1.0 - smooth(0.0, 0.55, f1);
            let fine = fbm(u, v, 40, 40, 2, s + 1);
            tx(
                0.4 * crumb + 0.15 * fine + 0.3,
                0.62 + 0.55 * crumb + 0.1 * fine,
                0.97,
            )
        }
        _ => tx(0.5, 1.0, 0.5),
    }
}

/// Every map, ready for a texture array: layer-major (each layer, then
/// its levels), as the GPU upload wants it. A texel is RGBA8: colour
/// multiplier ÷ 1.5, roughness, the normal's x and y.
#[derive(Clone, Debug, PartialEq)]
pub struct Maps {
    /// Layers × levels, layer-major.
    pub data: Vec<u8>,
}

/// A preview of one material's map lit from the upper left, `colour`
/// its part's colour (for the design proposal and by-eye checks).
#[must_use]
pub fn preview(m: Mat, colour: [f32; 3]) -> crate::paint::Img {
    let l = bake_layer(m);
    let s = l.side;
    let mut img = crate::paint::Img::new(s as i32, s as i32);
    let light = {
        let v = [-0.5f32, -0.6, 0.62];
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / n, v[1] / n, v[2] / n]
    };
    for q in 0..s * s {
        let n = l.n[q];
        let d = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0);
        let h = {
            let hv = [light[0], light[1], light[2] + 1.0];
            let hl = (hv[0] * hv[0] + hv[1] * hv[1] + hv[2] * hv[2]).sqrt();
            ((n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2]) / hl).max(0.0)
        };
        let r = l.rough[q].max(0.08);
        let spec = h.powf(2.0 / (r * r * r * r) - 2.0).min(1.0) * (1.0 - r) * 0.6;
        let lit = 0.35 + 0.75 * d;
        let c: Vec<u8> = colour
            .iter()
            .map(|c| byte(c * l.alb[q] * lit + spec))
            .collect();
        let o = q * 4;
        img.px[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    }
    img
}

/// One layer's float texels: colour multiplier, roughness, normal.
struct Layer {
    side: usize,
    alb: Vec<f32>,
    rough: Vec<f32>,
    n: Vec<[f32; 3]>,
}

fn bake_layer(m: Mat) -> Layer {
    let side = SIZE as usize;
    let mut texels = Vec::with_capacity(side * side);
    for j in 0..side {
        for i in 0..side {
            let (u, v) = (
                (i as f32 + 0.5) / side as f32,
                (j as f32 + 0.5) / side as f32,
            );
            texels.push(texel(m, u, v));
        }
    }
    let at = |i: usize, j: usize| texels[(j % side) * side + (i % side)].h;
    // the height's slope per texel, scaled so a bump reads the same at
    // any map size
    let k = relief(m) * side as f32 / 64.0;
    let mut n = Vec::with_capacity(side * side);
    for j in 0..side {
        for i in 0..side {
            let dx = at(i + 1, j) - at(i + side - 1, j);
            let dy = at(i, j + 1) - at(i, j + side - 1);
            let v = [-dx * k * 0.5, -dy * k * 0.5, 1.0];
            let l = (v[0] * v[0] + v[1] * v[1] + 1.0).sqrt();
            n.push([v[0] / l, v[1] / l, v[2] / l]);
        }
    }
    Layer {
        side,
        alb: texels.iter().map(|t| t.alb).collect(),
        rough: texels.iter().map(|t| t.rough.clamp(0.04, 1.0)).collect(),
        n,
    }
}

/// Halves a layer: colours and roughness averaged; normals averaged and
/// what the average loses added to the roughness (Toksvig), so a bumpy
/// surface far away turns matt instead of sparkling.
fn halve(l: &Layer) -> Layer {
    let side = (l.side / 2).max(1);
    let mut out = Layer {
        side,
        alb: Vec::with_capacity(side * side),
        rough: Vec::with_capacity(side * side),
        n: Vec::with_capacity(side * side),
    };
    for j in 0..side {
        for i in 0..side {
            let idx = [
                (2 * j) * l.side + 2 * i,
                (2 * j) * l.side + (2 * i + 1).min(l.side - 1),
                (2 * j + 1).min(l.side - 1) * l.side + 2 * i,
                (2 * j + 1).min(l.side - 1) * l.side + (2 * i + 1).min(l.side - 1),
            ];
            let mut a = 0.0;
            let mut r2 = 0.0;
            let mut nn = [0.0f32; 3];
            for &q in &idx {
                a += l.alb[q];
                r2 += l.rough[q] * l.rough[q];
                for (acc, v) in nn.iter_mut().zip(l.n[q]) {
                    *acc += v;
                }
            }
            let len = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2]).sqrt() / 4.0;
            let var = ((1.0 - len) / len.max(1e-3)).max(0.0);
            out.alb.push(a / 4.0);
            out.rough.push((r2 / 4.0 + var).sqrt().min(1.0));
            let l3 = (len * 4.0).max(1e-6);
            out.n.push([nn[0] / l3, nn[1] / l3, nn[2] / l3]);
        }
    }
    out
}

fn byte(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn pack(l: &Layer, out: &mut Vec<u8>) {
    for q in 0..l.side * l.side {
        out.extend_from_slice(&[
            byte(l.alb[q] / 1.5),
            byte(l.rough[q]),
            byte(l.n[q][0] * 0.5 + 0.5),
            byte(l.n[q][1] * 0.5 + 0.5),
        ]);
    }
}

/// One material's map with every level, packed.
fn bake_one(m: Mat) -> Vec<u8> {
    let mut out = Vec::new();
    let mut l = bake_layer(m);
    for _ in 0..LEVELS {
        pack(&l, &mut out);
        l = halve(&l);
    }
    out
}

/// Every material's map, made on as many threads as there are layers
/// (one map is about a millisecond's work on each).
#[must_use]
pub fn bake() -> Maps {
    let parts: Vec<Vec<u8>> = std::thread::scope(|sc| {
        let handles: Vec<_> = (1..=LAYERS)
            .map(|m| sc.spawn(move || bake_one(m)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    });
    Maps {
        data: parts.concat(),
    }
}

/// The bytes the maps take on the GPU.
#[must_use]
pub fn gpu_bytes() -> u64 {
    let mut level = u64::from(SIZE) * u64::from(SIZE) * 4;
    let mut sum = 0;
    for _ in 0..LEVELS {
        sum += level;
        level /= 4;
    }
    sum * u64::from(LAYERS)
}

/// Each material's kind for the shader, as WGSL: a private table indexed
/// by the material's number.
#[must_use]
pub fn wgsl_table() -> String {
    let kinds = KINDS.iter().map(|kd| {
        format!(
            "vec4<f32>({:.4}, {:.3}, {:.3}, {:.3})",
            if kd.unit { -kd.tile } else { kd.tile },
            kd.metal,
            kd.wrap,
            kd.sheen
        )
    });
    let aniso = KINDS.iter().map(|kd| format!("{:.3}", kd.aniso));
    wgsl_array("MKIND", "vec4<f32>", kinds) + &wgsl_array("MANISO", "f32", aniso)
}

/// One private WGSL array of `ty` named `name`, with its values in order.
fn wgsl_array(name: &str, ty: &str, values: impl ExactSizeIterator<Item = String>) -> String {
    let n = values.len();
    let body: Vec<String> = values.collect();
    format!(
        "var<private> {name}: array<{ty}, {n}> = array<{ty}, {n}>({});\n",
        body.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_map_tiles_without_a_seam() {
        // the last column meets the first as smoothly as any two others
        for m in 1..=LAYERS {
            if m == LEAF {
                continue; // one leaf a map, never tiled
            }
            let l = bake_layer(m);
            let s = l.side;
            let step = |a: usize, b: usize| (l.alb[a] - l.alb[b]).abs();
            let (mut inner, mut seam) = (0.0, 0.0);
            for j in 0..s {
                seam += step(j * s + s - 1, j * s);
                inner += step(j * s + s / 2 - 1, j * s + s / 2);
            }
            assert!(
                seam < inner * 2.5 + 0.5,
                "material {m}: seam {seam} vs {inner}"
            );
        }
    }

    #[test]
    fn maps_are_the_same_every_time_and_sized_as_promised() {
        let a = bake();
        let b = bake();
        assert_eq!(a, b);
        assert_eq!(a.data.len() as u64, gpu_bytes());
        // 16 layers of 256², every level: about 5.6 MB on the GPU
        assert!(gpu_bytes() < 6 * 1024 * 1024);
    }

    #[test]
    fn normals_are_unit_and_roughness_in_range() {
        for m in [WOOD, FABRIC, CLOTH, HAIR, LEAF] {
            let l = bake_layer(m);
            for n in &l.n {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-3 && n[2] > 0.0);
            }
            assert!(l.rough.iter().all(|r| (0.04..=1.0).contains(r)));
            // a material is a surface, not a flat colour: it varies
            let mean = l.alb.iter().sum::<f32>() / l.alb.len() as f32;
            let var = l.alb.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / l.alb.len() as f32;
            assert!(var > 1e-4, "material {m} is flat");
        }
    }

    #[test]
    fn a_far_bumpy_surface_turns_matt() {
        let mut l = bake_layer(CARPET);
        let r0 = l.rough.iter().sum::<f32>() / l.rough.len() as f32;
        for _ in 0..4 {
            l = halve(&l);
        }
        let r4 = l.rough.iter().sum::<f32>() / l.rough.len() as f32;
        assert!(r4 >= r0 - 1e-3);
        let mut l = bake_layer(FABRIC);
        let flat0 = l.n.iter().map(|n| n[2]).sum::<f32>() / l.n.len() as f32;
        for _ in 0..5 {
            l = halve(&l);
        }
        let flat5 = l.n.iter().map(|n| n[2]).sum::<f32>() / l.n.len() as f32;
        assert!(flat5 > flat0, "far away the weave's bumps average out");
    }

    #[test]
    fn the_shader_table_names_every_material() {
        let t = wgsl_table();
        assert!(t.contains("array<vec4<f32>, 18>"));
        assert_eq!(t.matches("vec4<f32>(").count(), 18);
        assert_eq!(layer(FACE), Some(SKIN - 1));
        assert_eq!(layer(NONE), None);
        assert_eq!(layer(SOIL), Some(LAYERS - 1));
    }
}
