//! AJ-E3520: the office's light follows the user's own day. The sun's
//! place comes from the system's time zone (its coordinates in the zone
//! table, so no Location permission) and the clock, by NOAA's equations
//! (`juggler_floor::daylight`). The light steps every ten minutes, one
//! frame each time, so a still Floor still draws nothing in between.

use juggler_floor::daylight::{STEP_MS, Sky, sky_for, zone_coordinates};

use super::store::Light;

/// Where the system's time zone is: latitude and longitude, degrees;
/// none where the system names no zone (the Floor then keeps the day's
/// light).
#[must_use]
pub fn place() -> Option<(f32, f32)> {
    let zone = juggler_platform::system::time_zone()?;
    zone_coordinates(&juggler_platform::system::zone_table()?, &zone)
}

/// The step `now_ms` falls in: the light changes only from one to the
/// next.
#[must_use]
pub const fn step_of(now_ms: i64) -> i64 {
    now_ms.div_euclid(STEP_MS)
}

/// Milliseconds until the next step (at least one).
#[must_use]
pub fn until_next_step(now_ms: i64) -> u64 {
    u64::try_from(STEP_MS - now_ms.rem_euclid(STEP_MS))
        .unwrap_or(1)
        .max(1)
}

/// The sky for `light` at the step holding `now_ms`, over `place`.
#[must_use]
pub fn sky(light: Light, place: Option<(f32, f32)>, now_ms: i64) -> Sky {
    match (light, place) {
        (Light::Night, _) => Sky::always_night(),
        (Light::Clock, Some((lat, lon))) => sky_for(lat, lon, step_of(now_ms) * STEP_MS),
        (Light::Day | Light::Clock, _) => Sky::always_day(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Light, sky, step_of, until_next_step};
    use juggler_floor::daylight::STEP_MS;

    // 2026-06-21 12:00 UTC.
    const NOON: i64 = 1_782_043_200_000;
    const GREENWICH: Option<(f32, f32)> = Some((51.48, 0.0));

    #[test]
    fn the_light_follows_the_clock_and_steps_every_ten_minutes() {
        let day = sky(Light::Clock, GREENWICH, NOON);
        let night = sky(Light::Clock, GREENWICH, NOON - 12 * 3_600_000);
        assert!(day.night < 0.01 && night.night > 0.99);
        // the same within a step, so nothing redraws in between
        assert_eq!(
            sky(Light::Clock, GREENWICH, NOON + 1_000),
            sky(Light::Clock, GREENWICH, NOON + STEP_MS - 1_000)
        );
        assert_eq!(step_of(NOON + STEP_MS) - step_of(NOON), 1);
        assert_eq!(until_next_step(NOON), u64::try_from(STEP_MS).unwrap());
        assert_eq!(until_next_step(NOON + STEP_MS - 5), 5);
    }

    #[test]
    fn always_day_and_night_ignore_the_clock_and_no_zone_keeps_the_day() {
        assert_eq!(
            sky(Light::Day, GREENWICH, NOON - 12 * 3_600_000),
            juggler_floor::daylight::Sky::always_day()
        );
        assert_eq!(
            sky(Light::Night, None, NOON),
            juggler_floor::daylight::Sky::always_night()
        );
        assert_eq!(
            sky(Light::Clock, None, NOON - 12 * 3_600_000),
            juggler_floor::daylight::Sky::always_day()
        );
    }
}
