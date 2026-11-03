//! Pixel painting: an RGBA image with a scanline rasteriser, the fixed perspective camera (with
//! unprojection, so floors and walls can be shaded per pixel in perspective), hue-shifted colour
//! ramps, seeded randomness and a small pixel font for signs.

pub type Rgba = [u8; 4];

#[derive(Clone)]
pub struct Img {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

impl Img {
    pub fn new(w: i32, h: i32) -> Self {
        Img { w, h, px: vec![0; (w.max(1) * h.max(1) * 4) as usize] }
    }
    pub fn get(&self, x: i32, y: i32) -> Rgba {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return [0; 4];
        }
        let i = ((y * self.w + x) * 4) as usize;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }
    pub fn blend(&mut self, x: i32, y: i32, c: Rgba) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || c[3] == 0 {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        let a = c[3] as u32;
        if a == 255 {
            self.px[i..i + 4].copy_from_slice(&c);
            return;
        }
        for k in 0..3 {
            self.px[i + k] = ((c[k] as u32 * a + self.px[i + k] as u32 * (255 - a)) / 255) as u8;
        }
        self.px[i + 3] = (a + self.px[i + 3] as u32 * (255 - a) / 255).min(255) as u8;
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.blend(xx, yy, c);
            }
        }
    }
    /// Fills a polygon, asking `f` for the colour of each covered pixel.
    pub fn poly_with(&mut self, pts: &[(f32, f32)], mut f: impl FnMut(i32, i32) -> Option<Rgba>) {
        if pts.len() < 3 {
            return;
        }
        let miny = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        let maxy = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let y0 = (miny.floor() as i32).max(0);
        let y1 = (maxy.ceil() as i32).min(self.h - 1);
        let mut xs: Vec<f32> = Vec::with_capacity(8);
        for y in y0..=y1 {
            let sy = y as f32 + 0.5;
            xs.clear();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let xa = (pair[0] - 0.5).ceil() as i32;
                let xb = (pair[1] - 0.5).ceil() as i32;
                for x in xa.max(0)..xb.min(self.w) {
                    if let Some(c) = f(x, y) {
                        self.blend(x, y, c);
                    }
                }
            }
        }
    }
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Rgba) {
        self.poly_with(pts, |_, _| Some(c));
    }
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgba) {
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.blend(x, y, c);
                }
            }
        }
    }
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
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), c: Rgba) {
        let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs())).ceil().max(1.0) as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.blend((a.0 + (b.0 - a.0) * t).round() as i32, (a.1 + (b.1 - a.1) * t).round() as i32, c);
        }
    }
    /// A one-pixel outline in a darker, cooler version of the neighbouring colour ("selective outline").
    pub fn outline(&mut self) {
        let src = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if src.get(x, y)[3] > 0 {
                    continue;
                }
                let mut best: Option<Rgba> = None;
                for (dx, dy) in [(0, 1), (1, 0), (-1, 0), (0, -1)] {
                    let n = src.get(x + dx, y + dy);
                    if n[3] > 200 {
                        best = Some(n);
                        break;
                    }
                }
                if let Some(n) = best {
                    let o = ramp(n, -0.62);
                    self.blend(x, y, [o[0], o[1], o[2], 255]);
                }
            }
        }
    }
    pub fn blit(&mut self, src: &Img, dx: i32, dy: i32) {
        for y in 0..src.h {
            for x in 0..src.w {
                self.blend(dx + x, dy + y, src.get(x, y));
            }
        }
    }
    pub fn mirrored(&self) -> Img {
        let mut m = Img::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                let c = self.get(x, y);
                let i = ((y * self.w + (self.w - 1 - x)) * 4) as usize;
                m.px[i..i + 4].copy_from_slice(&c);
            }
        }
        m
    }
}

// ---------- colour ----------
pub fn hex(s: &str) -> Rgba {
    let n = u32::from_str_radix(&s[1..], 16).unwrap();
    [(n >> 16) as u8, (n >> 8) as u8, n as u8, 255]
}
pub fn alpha(c: Rgba, a: u8) -> Rgba {
    [c[0], c[1], c[2], a]
}
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        a[3],
    ]
}
pub fn mul(c: Rgba, k: [f32; 3]) -> Rgba {
    [(c[0] as f32 * k[0]).min(255.0) as u8, (c[1] as f32 * k[1]).min(255.0) as u8, (c[2] as f32 * k[2]).min(255.0) as u8, c[3]]
}

fn to_hsl(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    if mx == mn {
        return (0.0, 0.0, l);
    }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h * 60.0, s, l)
}
pub fn hsl(h: f32, s: f32, l: f32) -> Rgba {
    let h = h.rem_euclid(360.0);
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h / 30.0) % 12.0;
        l - a * (k - 3.0).min(9.0 - k).min(1.0).max(-1.0)
    };
    [(f(0.0) * 255.0).round() as u8, (f(8.0) * 255.0).round() as u8, (f(4.0) * 255.0).round() as u8, 255]
}
/// Steps along a hue-shifted ramp: darker steps lean cool (toward blue-violet) and lose a little
/// saturation; lighter steps lean warm (toward yellow). `t` is -1 (darkest) .. +1 (lightest).
pub fn ramp(c: Rgba, t: f32) -> Rgba {
    let (h, s, l) = to_hsl(c);
    let target = if t < 0.0 { 245.0 } else { 55.0 };
    let mut dh = target - h;
    if dh > 180.0 { dh -= 360.0 }
    if dh < -180.0 { dh += 360.0 }
    let shift = dh.clamp(-22.0, 22.0) * t.abs() * if s < 0.08 { 0.3 } else { 1.0 };
    let nl = if t < 0.0 { l * (1.0 + t * 0.62) } else { l + (1.0 - l) * t * 0.55 };
    let ns = (s * (1.0 - t.abs() * 0.18)).clamp(0.0, 1.0);
    let mut o = hsl(h + shift, ns, nl.clamp(0.0, 1.0));
    o[3] = c[3];
    o
}

/// OKLCH (as in mark-hues.ts) to sRGB.
pub fn oklch(l: f32, c: f32, h_deg: f32) -> Rgba {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    let r = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let bl = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;
    let enc = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.0031308 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (v * 255.0).round() as u8
    };
    [enc(r), enc(g), enc(bl), 255]
}

// ---------- seeded randomness ----------
pub fn hash(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}
#[derive(Clone)]
pub struct Rng(pub u32);
impl Rng {
    pub fn of(s: &str) -> Rng {
        Rng(hash(s))
    }
    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f32) / 4294967296.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
    pub fn int(&mut self, n: usize) -> usize {
        ((self.next() * n as f32) as usize).min(n.saturating_sub(1))
    }
    pub fn pick<'a, T>(&mut self, a: &'a [T]) -> &'a T {
        &a[self.int(a.len())]
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.next() < p
    }
}
/// Deterministic noise in 0..1 for a lattice point.
pub fn noise2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

// ---------- the fixed camera ----------
pub type V3 = [f32; 3];
pub fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
pub fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn norm(a: V3) -> V3 { let l = dot(a, a).sqrt(); [a[0] / l, a[1] / l, a[2] / l] }

pub struct Cam {
    pub eye: V3,
    r: V3,
    u: V3,
    f: V3,
    focal: f32,
    cx: f32,
    cy: f32,
}
impl Cam {
    /// Looks at `target` from above at `el_deg`, `dist` away, onto an image `w` x `h`.
    pub fn new(target: V3, el_deg: f32, dist: f32, fov_deg: f32, w: i32, h: i32) -> Cam {
        let el = el_deg.to_radians();
        let eye = [target[0], target[1] + dist * el.sin(), target[2] + dist * el.cos()];
        let f = norm(sub(target, eye));
        let r = norm(cross(f, [0.0, 1.0, 0.0]));
        let u = cross(r, f);
        Cam { eye, r, u, f, focal: (h as f32 / 2.0) / (fov_deg.to_radians() / 2.0).tan(), cx: w as f32 / 2.0, cy: h as f32 / 2.0 }
    }
    /// Moves the picture within the image.
    pub fn shift(&mut self, dx: f32, dy: f32) {
        self.cx += dx;
        self.cy += dy;
    }
    pub fn p(&self, w: V3) -> (f32, f32) {
        let d = sub(w, self.eye);
        let (x, y, z) = (dot(d, self.r), dot(d, self.u), dot(d, self.f));
        (self.cx + x / z * self.focal, self.cy - y / z * self.focal)
    }
    pub fn quad(&self, q: [V3; 4]) -> [(f32, f32); 4] {
        [self.p(q[0]), self.p(q[1]), self.p(q[2]), self.p(q[3])]
    }
    fn ray(&self, px: f32, py: f32) -> V3 {
        let (a, b) = ((px - self.cx) / self.focal, -(py - self.cy) / self.focal);
        [self.f[0] + self.r[0] * a + self.u[0] * b, self.f[1] + self.r[1] * a + self.u[1] * b, self.f[2] + self.r[2] * a + self.u[2] * b]
    }
    /// Where the pixel's ray meets the horizontal plane y = `y`.
    pub fn on_plane_y(&self, px: f32, py: f32, y: f32) -> Option<(f32, f32)> {
        let d = self.ray(px, py);
        if d[1].abs() < 1e-6 { return None; }
        let t = (y - self.eye[1]) / d[1];
        (t > 0.0).then(|| (self.eye[0] + d[0] * t, self.eye[2] + d[2] * t))
    }
    /// Where the pixel's ray meets the vertical plane z = `z`.
    pub fn on_plane_z(&self, px: f32, py: f32, z: f32) -> Option<(f32, f32)> {
        let d = self.ray(px, py);
        if d[2].abs() < 1e-6 { return None; }
        let t = (z - self.eye[2]) / d[2];
        (t > 0.0).then(|| (self.eye[0] + d[0] * t, self.eye[1] + d[1] * t))
    }
    /// Where the pixel's ray meets the vertical plane x = `x`.
    pub fn on_plane_x(&self, px: f32, py: f32, x: f32) -> Option<(f32, f32)> {
        let d = self.ray(px, py);
        if d[0].abs() < 1e-6 { return None; }
        let t = (x - self.eye[0]) / d[0];
        (t > 0.0).then(|| (self.eye[1] + d[1] * t, self.eye[2] + d[2] * t))
    }
    /// Art pixels per metre at a floor point (for sizing sprites by depth).
    pub fn scale_at(&self, w: V3) -> f32 {
        let a = self.p(w);
        let b = self.p([w[0] + 1.0, w[1], w[2]]);
        (b.0 - a.0).abs()
    }
}

/// Draws the camera-facing faces of an axis-aligned box. `shade` gives each face's colour from its normal.
pub fn draw_box(img: &mut Img, cam: &Cam, off: (f32, f32), lo: V3, hi: V3, base: Rgba) {
    draw_box_faces(img, cam, off, lo, hi, |n| match n {
        1 => ramp(base, 0.18),           // top: lit
        2 => ramp(base, -0.22),          // front (+z): toward the camera
        3 => ramp(base, -0.42),          // sides
        _ => ramp(base, -0.5),
    });
}
pub fn draw_box_faces(img: &mut Img, cam: &Cam, off: (f32, f32), lo: V3, hi: V3, colour: impl Fn(u8) -> Rgba) {
    let o = |q: [(f32, f32); 4]| q.map(|(x, y)| (x - off.0, y - off.1));
    let faces: [(V3, V3, [V3; 4], u8); 5] = [
        ([0.0, 1.0, 0.0], [lo[0], hi[1], lo[2]], [[lo[0], hi[1], lo[2]], [hi[0], hi[1], lo[2]], [hi[0], hi[1], hi[2]], [lo[0], hi[1], hi[2]]], 1),
        ([0.0, 0.0, 1.0], [lo[0], lo[1], hi[2]], [[lo[0], lo[1], hi[2]], [hi[0], lo[1], hi[2]], [hi[0], hi[1], hi[2]], [lo[0], hi[1], hi[2]]], 2),
        ([0.0, 0.0, -1.0], [lo[0], lo[1], lo[2]], [[lo[0], lo[1], lo[2]], [hi[0], lo[1], lo[2]], [hi[0], hi[1], lo[2]], [lo[0], hi[1], lo[2]]], 4),
        ([1.0, 0.0, 0.0], [hi[0], lo[1], lo[2]], [[hi[0], lo[1], lo[2]], [hi[0], lo[1], hi[2]], [hi[0], hi[1], hi[2]], [hi[0], hi[1], lo[2]]], 3),
        ([-1.0, 0.0, 0.0], [lo[0], lo[1], lo[2]], [[lo[0], lo[1], lo[2]], [lo[0], lo[1], hi[2]], [lo[0], hi[1], hi[2]], [lo[0], hi[1], lo[2]]], 3),
    ];
    for (n, pt, q, id) in faces {
        if dot(n, sub(cam.eye, pt)) > 0.0 {
            img.poly(&o(cam.quad(q)), colour(id));
        }
    }
}
/// Art-pixel bounds of world points (with a margin), as (x, y, w, h).
pub fn bounds(cam: &Cam, pts: &[V3], margin: i32) -> (i32, i32, i32, i32) {
    let ps: Vec<(f32, f32)> = pts.iter().map(|p| cam.p(*p)).collect();
    let x0 = ps.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32 - margin;
    let y0 = ps.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32 - margin;
    let x1 = ps.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32 + margin;
    let y1 = ps.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32 + margin;
    (x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
}
pub fn corners(lo: V3, hi: V3) -> Vec<V3> {
    let mut v = Vec::with_capacity(8);
    for &x in &[lo[0], hi[0]] {
        for &y in &[lo[1], hi[1]] {
            for &z in &[lo[2], hi[2]] {
                v.push([x, y, z]);
            }
        }
    }
    v
}

// ---------- a 5x7 pixel font for signs (upper case, digits, a few marks) ----------
fn glyph(c: char) -> [&'static str; 7] {
    match c {
        'A' => [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'B' => ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
        'C' => [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
        'D' => ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
        'E' => ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
        'F' => ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
        'G' => [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
        'H' => ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'I' => ["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"],
        'J' => ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
        'K' => ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
        'L' => ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
        'M' => ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'N' => ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'P' => ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
        'Q' => [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
        'R' => ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'S' => [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'W' => ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."],
        'X' => ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
        'Y' => ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
        '0' => [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."],
        '1' => [".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"],
        '2' => [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
        '3' => ["####.", "....#", "....#", ".###.", "....#", "....#", "####."],
        '4' => ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
        '6' => [".###.", "#....", "#....", "####.", "#...#", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
        '8' => [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
        '9' => [".###.", "#...#", "#...#", ".####", "....#", "....#", ".###."],
        '·' => [".", ".", ".", "#", ".", ".", "."],
        '-' => ["...", "...", "...", "###", "...", "...", "..."],
        '.' => [".", ".", ".", ".", ".", ".", "#"],
        '!' => ["#", "#", "#", "#", "#", ".", "#"],
        '?' => [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
        ':' => [".", "#", ".", ".", ".", "#", "."],
        '/' => ["....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#...."],
        '%' => ["##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##"],
        _ => ["..", "..", "..", "..", "..", "..", ".."],
    }
}
pub fn text_width(s: &str, scale: i32) -> i32 {
    let mut w = 0;
    for c in s.chars() {
        let g = glyph(c.to_ascii_uppercase());
        w += (g[0].len() as i32 + 1) * scale;
    }
    (w - scale).max(0)
}
pub fn draw_text(img: &mut Img, x: i32, y: i32, s: &str, col: Rgba, scale: i32) {
    let mut cx = x;
    for c in s.chars() {
        let g = glyph(c.to_ascii_uppercase());
        for (row, line) in g.iter().enumerate() {
            for (k, ch) in line.chars().enumerate() {
                if ch == '#' {
                    img.rect(cx + k as i32 * scale, y + row as i32 * scale, scale, scale, col);
                }
            }
        }
        cx += (g[0].len() as i32 + 1) * scale;
    }
}
