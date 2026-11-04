//! Colours: sRGB bytes, OKLCH, and the app's mark colours (one per App
//! and per Account, given once and stored, AJ-E3206).

/// An sRGB colour with alpha, 0–255 each.
pub type Rgba = [u8; 4];

/// The app's mark colours as OKLCH `(l, c, h°)`, slot by slot: the Rust
/// copy of `MARK_COLOURS` in `ui/src/lib/mark-hues.ts` (a test reads that
/// file, so the two can't drift).
pub const MARK_COLOURS: [(f32, f32, f32); 8] = [
    (0.627, 0.16, 255.0),
    (0.549, 0.143, 130.0),
    (0.629, 0.16, 345.0),
    (0.562, 0.16, 320.0),
    (0.571, 0.16, 280.0),
    (0.619, 0.136, 115.0),
    (0.645, 0.16, 325.0),
    (0.564, 0.16, 345.0),
];

/// The mark colour of colour slot `slot` (it wraps).
#[must_use]
pub fn mark_colour(slot: usize) -> Rgba {
    let (l, c, h) = MARK_COLOURS[slot % MARK_COLOURS.len()];
    oklch(l, c, h)
}

/// A `#rrggbb` colour, opaque; black for anything else.
#[must_use]
pub fn hex(text: &str) -> Rgba {
    let n = text
        .strip_prefix('#')
        .and_then(|digits| u32::from_str_radix(digits, 16).ok())
        .unwrap_or(0);
    [(n >> 16) as u8, (n >> 8) as u8, n as u8, 255]
}

/// `a` towards `b` by `t` (0–1); `a`'s alpha.
#[must_use]
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let channel =
        |k: usize| (f32::from(a[k]) + (f32::from(b[k]) - f32::from(a[k])) * t).round() as u8;
    [channel(0), channel(1), channel(2), a[3]]
}

/// An OKLCH colour as sRGB (gamut-clipped).
#[must_use]
#[allow(clippy::many_single_char_names)] // OKLab's own names.
pub fn oklch(l: f32, c: f32, h_deg: f32) -> Rgba {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = l + 0.396_337_8 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_35 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_4 * s3;
    let bl = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
    let encode = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (v * 255.0).round() as u8
    };
    [encode(r), encode(g), encode(bl), 255]
}

#[cfg(test)]
mod tests {
    use super::{MARK_COLOURS, hex, mix, oklch};

    #[test]
    fn the_mark_colours_match_the_main_windows() {
        let ts = include_str!("../../../ui/src/lib/mark-hues.ts");
        let start = ts.find("export const MARK_COLOURS").unwrap_or(0);
        let table = &ts[start..ts[start..].find("];").map_or(ts.len(), |end| start + end)];
        let rows: Vec<(f32, f32, f32)> = table
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let l = line.strip_prefix("{ l: ")?;
                let (l, rest) = l.split_once(", c: ")?;
                let (c, rest) = rest.split_once(", h: ")?;
                let h = rest.trim_end_matches(" },").trim_end_matches(" }");
                Some((l.parse().ok()?, c.parse().ok()?, h.parse().ok()?))
            })
            .collect();
        assert_eq!(
            rows.as_slice(),
            MARK_COLOURS.as_slice(),
            "mark-hues.ts and colour.rs differ"
        );
    }

    #[test]
    fn colours_read_and_mix() {
        assert_eq!(hex("#ff8000"), [255, 128, 0, 255]);
        assert_eq!(hex("nonsense"), [0, 0, 0, 255]);
        assert_eq!(
            mix([0, 0, 0, 9], [200, 100, 50, 255], 0.5),
            [100, 50, 25, 9]
        );
        let white = oklch(1.0, 0.0, 0.0);
        assert!(white[0] >= 254 && white[1] >= 254 && white[2] >= 254);
    }
}
