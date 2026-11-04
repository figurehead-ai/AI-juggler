//! Pixel painting for the Floor's textures: an RGBA image with a scanline
//! rasteriser, the "selective outline", lattice noise and a 5×7 pixel
//! font for signs. Every texture is painted here at start, from seeds:
//! nothing is bundled (ADR-0026).

use crate::colour::{Rgba, ramp};

/// An RGBA image, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Img {
    /// Width, pixels.
    pub w: i32,
    /// Height, pixels.
    pub h: i32,
    /// The pixels, 4 bytes each.
    pub px: Vec<u8>,
}

impl Img {
    /// A transparent image.
    #[must_use]
    pub fn new(w: i32, h: i32) -> Self {
        let (w, h) = (w.max(1), h.max(1));
        let len = usize::try_from(w * h * 4).unwrap_or(0);
        Self {
            w,
            h,
            px: vec![0; len],
        }
    }

    fn at(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h)
            .then(|| usize::try_from((y * self.w + x) * 4).unwrap_or(0))
    }

    /// The pixel at (`x`, `y`); transparent outside.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Rgba {
        self.at(x, y).map_or([0; 4], |i| {
            [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
        })
    }

    /// Paints `c` over the pixel at (`x`, `y`) by its alpha.
    pub fn blend(&mut self, x: i32, y: i32, c: Rgba) {
        let Some(i) = self.at(x, y) else { return };
        let a = u32::from(c[3]);
        if a == 0 {
            return;
        }
        if a == 255 {
            self.px[i..i + 4].copy_from_slice(&c);
            return;
        }
        for (dst, src) in self.px[i..i + 3].iter_mut().zip(c) {
            *dst = ((u32::from(src) * a + u32::from(*dst) * (255 - a)) / 255) as u8;
        }
        self.px[i + 3] = (a + u32::from(self.px[i + 3]) * (255 - a) / 255).min(255) as u8;
    }

    /// Fills a rectangle.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.blend(xx, yy, c);
            }
        }
    }

    /// A rectangle's one-pixel border.
    pub fn outline_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }

    /// Fills a polygon, asking `f` for each covered pixel's colour.
    pub fn poly_with(&mut self, pts: &[(f32, f32)], mut f: impl FnMut(i32, i32) -> Option<Rgba>) {
        if pts.len() < 3 {
            return;
        }
        let low = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        let high = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let mut xs: Vec<f32> = Vec::with_capacity(8);
        for y in (low.floor() as i32).max(0)..=(high.ceil() as i32).min(self.h - 1) {
            let sy = y as f32 + 0.5;
            xs.clear();
            for (i, a) in pts.iter().enumerate() {
                let b = pts[(i + 1) % pts.len()];
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let (xa, xb) = ((pair[0] - 0.5).ceil() as i32, (pair[1] - 0.5).ceil() as i32);
                for x in xa.max(0)..xb.min(self.w) {
                    if let Some(c) = f(x, y) {
                        self.blend(x, y, c);
                    }
                }
            }
        }
    }

    /// Fills a polygon with one colour.
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Rgba) {
        self.poly_with(pts, |_, _| Some(c));
    }

    /// Fills an ellipse of radii (`rx`, `ry`) round (`cx`, `cy`).
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Rgba) {
        for y in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                if dx * dx + dy * dy <= 1.0 {
                    self.blend(x, y, c);
                }
            }
        }
    }

    /// Fills a disc.
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgba) {
        self.ellipse(cx, cy, r, r, c);
    }

    /// A one-pixel line.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), c: Rgba) {
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.blend(
                (a.0 + (b.0 - a.0) * t).round() as i32,
                (a.1 + (b.1 - a.1) * t).round() as i32,
                c,
            );
        }
    }

    /// The "selective outline": each empty pixel beside a solid one takes a
    /// darker, cooler step of that one's colour.
    pub fn outline(&mut self) {
        let src = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if src.get(x, y)[3] > 0 {
                    continue;
                }
                let solid = [(0, 1), (1, 0), (-1, 0), (0, -1)]
                    .into_iter()
                    .map(|(dx, dy)| src.get(x + dx, y + dy))
                    .find(|n| n[3] > 200);
                if let Some(n) = solid {
                    let o = ramp(n, -0.62);
                    self.blend(x, y, [o[0], o[1], o[2], 255]);
                }
            }
        }
    }

    /// Paints `src` with its top left at (`dx`, `dy`).
    pub fn blit(&mut self, src: &Self, dx: i32, dy: i32) {
        for y in 0..src.h {
            for x in 0..src.w {
                self.blend(dx + x, dy + y, src.get(x, y));
            }
        }
    }

    /// The image mirrored left to right.
    #[must_use]
    pub fn mirrored(&self) -> Self {
        let mut m = Self::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                let c = self.get(x, y);
                if let Some(i) = m.at(self.w - 1 - x, y) {
                    m.px[i..i + 4].copy_from_slice(&c);
                }
            }
        }
        m
    }
}

/// Deterministic noise in 0–1 for a lattice point.
#[must_use]
pub fn noise2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = x.cast_unsigned().wrapping_mul(374_761_393)
        ^ y.cast_unsigned().wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    f32::from(((h ^ (h >> 16)) & 0xffff) as u16) / 65535.0
}

/// The 5×7 sign font: upper case, digits and a few marks.
const FONT: &[(char, [&str; 7])] = &[
    (
        'A',
        [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'B',
        [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
    ),
    (
        'C',
        [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
    ),
    (
        'D',
        [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
    ),
    (
        'E',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
    ),
    (
        'F',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        'G',
        [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####",
        ],
    ),
    (
        'H',
        [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    ('I', ["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"]),
    (
        'J',
        [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
    ),
    (
        'K',
        [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        'L',
        [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
    ),
    (
        'M',
        [
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'N',
        [
            "#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#",
        ],
    ),
    (
        'O',
        [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
    ),
    (
        'P',
        [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        'Q',
        [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
    ),
    (
        'R',
        [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        'S',
        [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        'T',
        [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        'U',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
    ),
    (
        'V',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
    ),
    (
        'W',
        [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#.",
        ],
    ),
    (
        'X',
        [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
    ),
    (
        'Y',
        [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        'Z',
        [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
    ),
    (
        '0',
        [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
    ),
    ('1', [".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"]),
    (
        '2',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
    ),
    (
        '3',
        [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        '4',
        [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
    ),
    (
        '5',
        [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
    ),
    (
        '6',
        [
            ".###.", "#....", "#....", "####.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '7',
        [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
    ),
    (
        '8',
        [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '9',
        [
            ".###.", "#...#", "#...#", ".####", "....#", "....#", ".###.",
        ],
    ),
    ('·', [".", ".", ".", "#", ".", ".", "."]),
    ('-', ["...", "...", "...", "###", "...", "...", "..."]),
    ('.', [".", ".", ".", ".", ".", ".", "#"]),
    ('!', ["#", "#", "#", "#", "#", ".", "#"]),
    (
        '?',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
        ],
    ),
    (':', [".", "#", ".", ".", ".", "#", "."]),
    (
        '/',
        [
            "....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#....",
        ],
    ),
    (
        '+',
        [
            ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....",
        ],
    ),
    (
        '%',
        [
            "##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##",
        ],
    ),
];

/// A character of the sign font (a narrow space for the rest).
fn glyph(c: char) -> [&'static str; 7] {
    FONT.iter()
        .find(|(k, _)| *k == c)
        .map_or(["..", "..", "..", "..", "..", "..", ".."], |(_, g)| *g)
}

fn glyph_width(c: char) -> i32 {
    i32::try_from(glyph(c.to_ascii_uppercase())[0].len()).unwrap_or(0)
}

/// How wide `text` is in the sign font at `scale`.
#[must_use]
pub fn text_width(text: &str, scale: i32) -> i32 {
    let w: i32 = text.chars().map(|c| (glyph_width(c) + 1) * scale).sum();
    (w - scale).max(0)
}

/// Writes `text` in the sign font with its top left at (`x`, `y`).
pub fn draw_text(img: &mut Img, x: i32, y: i32, text: &str, col: Rgba, scale: i32) {
    let mut cx = x;
    for c in text.chars() {
        let g = glyph(c.to_ascii_uppercase());
        for (row, line) in (0_i32..).zip(g.iter()) {
            for (k, ch) in (0_i32..).zip(line.chars()) {
                if ch == '#' {
                    img.rect(cx + k * scale, y + row * scale, scale, scale, col);
                }
            }
        }
        cx += (glyph_width(c) + 1) * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::{Img, draw_text, noise2, text_width};

    #[test]
    fn shapes_fill_inside_and_blend_by_alpha() {
        let mut img = Img::new(10, 10);
        img.poly(
            &[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)],
            [200, 0, 0, 255],
        );
        assert_eq!(img.get(5, 5), [200, 0, 0, 255]);
        assert_eq!(img.get(0, 0), [0, 0, 0, 0]);
        img.blend(5, 5, [0, 0, 200, 128]);
        let mixed = img.get(5, 5);
        assert!(mixed[0] > 80 && mixed[2] > 80);
        img.outline();
        assert_ne!(img.get(0, 5)[3], 0, "the outline rings the shape");
        assert_eq!(img.get(-1, 3), [0; 4], "outside reads transparent");
    }

    #[test]
    fn text_is_as_wide_as_it_says_and_noise_stays_in_range() {
        let mut img = Img::new(40, 10);
        draw_text(&mut img, 0, 0, "AB", [255; 4], 1);
        assert_eq!(text_width("AB", 1), 11);
        assert!((0..11).any(|x| img.get(x, 0)[3] > 0));
        assert!((11..40).all(|x| (0..7).all(|y| img.get(x, y)[3] == 0)));
        for n in 0..200 {
            let v = noise2(n, -n, 7);
            assert!((0.0..=1.0).contains(&v));
        }
        assert_eq!(img.mirrored().mirrored(), img);
    }
}
