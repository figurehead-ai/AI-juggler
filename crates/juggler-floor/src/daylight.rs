//! Day and night (0.4.2, `docs/research/floor-04x.md` §6): where the sun
//! is for the Mac's own place and clock, and the light that gives the
//! office. Pure: the window side reads the time zone's coordinates (the
//! system's `zone.tab`) and the clock, and asks again every
//! [`STEP_MS`]; nothing here animates, so a still Floor draws no frames.
//!
//! The sun's position follows NOAA's "General Solar Position
//! Calculations" (declination and the equation of time from the
//! fractional year, then the hour angle): within a fraction of a degree,
//! far finer than the light needs.

use crate::math::{V3, norm};

/// How often the light is worked out again (one frame each time).
pub const STEP_MS: i64 = 10 * 60 * 1000;

/// The light the office is drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Towards the sun (or the moon at night), in the scene's axes.
    pub sun_dir: V3,
    /// The sun's colour.
    pub sun_col: V3,
    /// The sun's strength.
    pub sun_k: f32,
    /// The sky at the top of the window, and at its foot.
    pub top: V3,
    /// The sky at the window's foot.
    pub bottom: V3,
    /// Ambient light from above, and from below.
    pub hemi_sky: V3,
    /// Ambient light from below.
    pub hemi_ground: V3,
    /// 0 by day, 1 at night: lamps and screens carry the room.
    pub night: f32,
}

/// The scene's fixed sun in the classic look (0.4.0's).
pub const CLASSIC_SUN: V3 = [-0.42, 0.82, 0.38];

impl Sky {
    /// 0.4.0's light, unchanged: the golden frames are drawn in it.
    #[must_use]
    pub fn classic() -> Self {
        Self {
            sun_dir: norm(CLASSIC_SUN),
            sun_col: [1.0, 0.9, 0.76],
            sun_k: 0.62,
            top: [0.13, 0.11, 0.24],
            bottom: [0.62, 0.36, 0.42],
            hemi_sky: [0.66, 0.64, 0.76],
            hemi_ground: [0.34, 0.3, 0.38],
            night: 0.0,
        }
    }

    /// The light for the sun at `elevation` and `azimuth` (degrees;
    /// azimuth clockwise from north). The office faces south-east, so
    /// the morning sun comes in from the left.
    #[must_use]
    pub fn at(elevation: f32, azimuth: f32) -> Self {
        let day = Self::day_at(elevation.max(6.0), azimuth);
        let gold = Self::golden(elevation.clamp(0.0, 6.0), azimuth);
        let dusk = Self::dusk(azimuth);
        let night = Self::night(azimuth);
        if elevation >= 6.0 {
            day
        } else if elevation >= 0.0 {
            mix(&gold, &day, elevation / 6.0)
        } else if elevation >= -6.0 {
            mix(&dusk, &gold, (elevation + 6.0) / 6.0)
        } else if elevation >= -12.0 {
            mix(&night, &dusk, (elevation + 12.0) / 6.0)
        } else {
            night
        }
    }

    /// Always day: the sun high in the early afternoon.
    #[must_use]
    pub fn always_day() -> Self {
        Self::at(48.0, 200.0)
    }

    /// Always night.
    #[must_use]
    pub fn always_night() -> Self {
        Self::at(-30.0, 200.0)
    }

    fn day_at(elevation: f32, azimuth: f32) -> Self {
        let warm = 1.0 - ((elevation - 6.0) / 40.0).clamp(0.0, 1.0);
        Self {
            sun_dir: sun_vector(elevation, azimuth),
            sun_col: lerp3([1.0, 0.92, 0.78], [1.0, 0.8, 0.6], warm),
            sun_k: 0.7,
            top: [0.36, 0.55, 0.86],
            bottom: [0.78, 0.86, 0.95],
            hemi_sky: [0.5, 0.56, 0.72],
            hemi_ground: [0.32, 0.27, 0.28],
            night: 0.0,
        }
    }

    fn golden(elevation: f32, azimuth: f32) -> Self {
        Self {
            sun_dir: sun_vector(elevation.max(3.0) + 9.0, azimuth),
            sun_col: [1.0, 0.66, 0.4],
            sun_k: 0.6,
            top: [0.3, 0.32, 0.6],
            bottom: [0.98, 0.62, 0.42],
            hemi_sky: [0.56, 0.5, 0.62],
            hemi_ground: [0.4, 0.28, 0.26],
            night: 0.1,
        }
    }

    fn dusk(azimuth: f32) -> Self {
        Self {
            sun_dir: sun_vector(10.0, azimuth),
            sun_col: [0.7, 0.55, 0.85],
            sun_k: 0.22,
            top: [0.1, 0.09, 0.26],
            bottom: [0.52, 0.3, 0.46],
            hemi_sky: [0.42, 0.4, 0.58],
            hemi_ground: [0.22, 0.18, 0.26],
            night: 0.6,
        }
    }

    fn night(azimuth: f32) -> Self {
        // The moon: low, cool and faint, across from the sun.
        Self {
            sun_dir: sun_vector(34.0, azimuth + 180.0),
            sun_col: [0.6, 0.7, 1.0],
            sun_k: 0.14,
            top: [0.02, 0.03, 0.09],
            bottom: [0.09, 0.09, 0.2],
            hemi_sky: [0.3, 0.32, 0.46],
            hemi_ground: [0.14, 0.13, 0.2],
            night: 1.0,
        }
    }
}

fn lerp3(a: V3, b: V3, k: f32) -> V3 {
    [
        a[0] + (b[0] - a[0]) * k,
        a[1] + (b[1] - a[1]) * k,
        a[2] + (b[2] - a[2]) * k,
    ]
}

/// `a` towards `b` by `k` (0–1).
#[must_use]
pub fn mix(a: &Sky, b: &Sky, k: f32) -> Sky {
    let k = k.clamp(0.0, 1.0);
    Sky {
        sun_dir: norm(lerp3(a.sun_dir, b.sun_dir, k)),
        sun_col: lerp3(a.sun_col, b.sun_col, k),
        sun_k: a.sun_k + (b.sun_k - a.sun_k) * k,
        top: lerp3(a.top, b.top, k),
        bottom: lerp3(a.bottom, b.bottom, k),
        hemi_sky: lerp3(a.hemi_sky, b.hemi_sky, k),
        hemi_ground: lerp3(a.hemi_ground, b.hemi_ground, k),
        night: a.night + (b.night - a.night) * k,
    }
}

/// The scene's direction towards a sun at `elevation`, `azimuth`
/// (degrees). The office faces the sun's path: the morning light comes
/// in from the front left, noon's from the front, the evening's from the
/// front right (the azimuth's swing halved and kept to the front), so a
/// low sun never puts the whole floor in the back wall's shadow.
#[must_use]
pub fn sun_vector(elevation: f32, azimuth: f32) -> V3 {
    let swing = ((azimuth.rem_euclid(360.0) - 180.0) * 0.5).clamp(-70.0, 70.0);
    let (a, e) = (swing.to_radians(), elevation.max(14.0).to_radians());
    norm([a.sin() * e.cos(), e.sin(), a.cos() * e.cos()])
}

/// The sun's elevation and azimuth (degrees) at `lat`, `lon` (degrees,
/// east positive) at `unix_ms` (UTC), by NOAA's equations.
#[must_use]
pub fn solar(lat: f32, lon: f32, unix_ms: i64) -> (f32, f32) {
    let days = unix_ms as f64 / 86_400_000.0;
    // Day of the year and the hour (UTC), from the days since 1970.
    let (year_day, hour) = day_of_year(days);
    let gamma = 2.0 * std::f64::consts::PI / 365.0 * (year_day - 1.0 + (hour - 12.0) / 24.0);
    let eqtime = 229.18
        * (0.000_075 + 0.001_868 * gamma.cos()
            - 0.032_077 * gamma.sin()
            - 0.014_615 * (2.0 * gamma).cos()
            - 0.040_849 * (2.0 * gamma).sin());
    let decl = 0.006_918 - 0.399_912 * gamma.cos() + 0.070_257 * gamma.sin()
        - 0.006_758 * (2.0 * gamma).cos()
        + 0.000_907 * (2.0 * gamma).sin()
        - 0.002_697 * (3.0 * gamma).cos()
        + 0.001_48 * (3.0 * gamma).sin();
    let true_min = hour * 60.0 + eqtime + 4.0 * f64::from(lon);
    let ha = (true_min / 4.0 - 180.0).to_radians();
    let phi = f64::from(lat).to_radians();
    let cos_zen = (phi.sin() * decl.sin() + phi.cos() * decl.cos() * ha.cos()).clamp(-1.0, 1.0);
    let zen = cos_zen.acos();
    let elevation = 90.0 - zen.to_degrees();
    let az = {
        let s = zen.sin().max(1e-6);
        let c = ((phi.sin() * zen.cos() - decl.sin()) / (phi.cos() * s)).clamp(-1.0, 1.0);
        let a = 180.0 - c.acos().to_degrees();
        if ha > 0.0 { 360.0 - a } else { a }
    };
    (elevation as f32, (az.rem_euclid(360.0)) as f32)
}

/// The day of the year (1-based, with its fraction dropped) and the hour
/// of the day (UTC) for `days` since 1970-01-01.
fn day_of_year(days: f64) -> (f64, f64) {
    let whole = days.floor();
    let hour = (days - whole) * 24.0;
    // Civil date from days (Howard Hinnant's algorithm), then its ordinal.
    let z = whole as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy_march = doe - (365 * yoe + yoe / 4 - yoe / 100);
    // doy_march counts from 1 March; turn it into a count from 1 January.
    let y = yoe + era * 400;
    let mp = (5 * doy_march + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let jan_mar = if leap { 60 } else { 59 };
    let ordinal = if month <= 2 {
        doy_march - 306 + 1
    } else {
        doy_march + jan_mar + 1
    };
    (ordinal as f64, hour)
}

/// A time zone's coordinates from a `zone.tab` line's ISO 6709 field
/// ("+4352+01825" or "+404251-0740023"): (lat, lon) in degrees.
#[must_use]
pub fn iso6709(field: &str) -> Option<(f32, f32)> {
    let split = field[1..].find(['+', '-'])? + 1;
    let (la, lo) = field.split_at(split);
    let deg = |s: &str, d: usize| -> Option<f32> {
        let sign = if s.starts_with('-') { -1.0 } else { 1.0 };
        let digits = &s[1..];
        let whole: f32 = digits.get(..d)?.parse().ok()?;
        let min: f32 = digits.get(d..d + 2)?.parse().ok()?;
        let sec: f32 = digits
            .get(d + 2..)
            .filter(|s| !s.is_empty())
            .map_or(Some(0.0), |s| s.parse().ok())?;
        Some(sign * (whole + min / 60.0 + sec / 3600.0))
    };
    Some((deg(la, 2)?, deg(lo, 3)?))
}

/// The coordinates of time zone `zone` (`Europe/Sarajevo`) in a
/// `zone.tab` text.
#[must_use]
pub fn zone_coordinates(zone_tab: &str, zone: &str) -> Option<(f32, f32)> {
    zone_tab
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .find(|f| f.get(2) == Some(&zone))
        .and_then(|f| iso6709(f.get(1)?))
}

/// The sky for a place and moment: NOAA's sun, then [`Sky::at`].
#[must_use]
pub fn sky_for(lat: f32, lon: f32, unix_ms: i64) -> Sky {
    let (e, a) = solar(lat, lon, unix_ms);
    Sky::at(e, a)
}

#[cfg(test)]
mod tests {
    use super::{Sky, iso6709, solar, zone_coordinates};

    // 2026-06-21 12:00 UTC and 2026-12-21 12:00 UTC.
    const JUNE: i64 = 1_782_043_200_000;
    const DEC: i64 = 1_797_854_400_000;

    #[test]
    fn noon_on_the_equator_at_the_equinox_is_overhead_and_the_solstices_tilt() {
        // Greenwich, midsummer noon: the sun at 90 − 51.5 + 23.4 ≈ 62°.
        let (e, a) = solar(51.48, 0.0, JUNE);
        assert!((e - 62.0).abs() < 1.0, "{e}");
        assert!((a - 180.0).abs() < 5.0, "due south: {a}");
        // Midwinter noon: ≈ 15°.
        let (e, _) = solar(51.48, 0.0, DEC);
        assert!((e - 15.0).abs() < 1.0, "{e}");
        // Midnight is night.
        let (e, _) = solar(51.48, 0.0, JUNE - 12 * 3_600_000);
        assert!(e < 0.0, "{e}");
    }

    #[test]
    fn the_light_moves_from_day_through_dusk_to_night() {
        let day = Sky::at(40.0, 180.0);
        let dusk = Sky::at(-4.0, 270.0);
        let night = Sky::at(-20.0, 0.0);
        assert!(day.night < 0.01 && night.night > 0.99);
        assert!(dusk.night > 0.1 && dusk.night < 0.9);
        assert!(day.sun_k > night.sun_k);
        assert!(day.top[2] > night.top[2], "a blue day sky, a dark night");
        // Continuous at the joins.
        for e in [6.0f32, 0.0, -6.0, -12.0] {
            let (a, b) = (Sky::at(e + 0.01, 90.0), Sky::at(e - 0.01, 90.0));
            assert!((a.sun_k - b.sun_k).abs() < 0.02, "at {e}");
        }
    }

    #[test]
    fn a_zone_has_its_coordinates() {
        assert_eq!(
            iso6709("+4352+01825").map(|(a, b)| (a.round(), b.round())),
            Some((44.0, 18.0))
        );
        let (la, lo) = iso6709("+404251-0740023").unwrap_or_default();
        assert!((la - 40.714).abs() < 0.01 && (lo + 74.006).abs() < 0.01);
        let tab = "# comment\nBA\t+4352+01825\tEurope/Sarajevo\nUS\t+404251-0740023\tAmerica/New_York\tEastern (most areas)\n";
        assert!(zone_coordinates(tab, "America/New_York").is_some());
        assert_eq!(zone_coordinates(tab, "Mars/Base"), None);
    }

    #[test]
    fn classic_is_today_s_light() {
        let c = Sky::classic();
        assert!((c.sun_k - 0.62).abs() < 1e-6);
        assert_eq!(c.night, 0.0);
    }
}
