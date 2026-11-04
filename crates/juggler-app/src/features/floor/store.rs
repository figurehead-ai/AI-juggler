//! The office as saved (AJ-E3503 req. 3, schema v46): each App's place in
//! the order, family, look, size and shrink clock in `floor_areas`; the
//! office seed and the Floor's settings in `app_settings` under `floor.*`.
//! The conversions here are pure, so the rules are tested without a
//! window.

use juggler_core::storage::{FLOOR_SEED_KEY, FloorArea, Storage};
use juggler_floor::driver::Motion;
use juggler_floor::family::{Family, Layout};
use juggler_floor::kit::Choice;
use juggler_floor::layout::{AreaState, DEFAULT_PER_FLOOR, Office};

use crate::error::AppResult;

/// Apps a floor holds (4–16).
pub const PER_FLOOR_KEY: &str = "floor.apps_per_floor";
/// How much moves: `system`, `full`, `calm` or `off`.
pub const MOTION_KEY: &str = "floor.motion";
/// Show unused Apps' decay: `on` or `off`.
pub const DECAY_KEY: &str = "floor.decay";
/// Open the Floor when AI Juggler starts: `on` or `off`.
pub const AT_START_KEY: &str = "floor.open_at_start";
/// Keep the Floor on top: `on` or `off`.
pub const ON_TOP_KEY: &str = "floor.on_top";

/// The Floor's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Apps a floor holds.
    pub per_floor: usize,
    /// How much moves.
    pub motion: Motion,
    /// Show unused Apps' decay.
    pub decay: bool,
    /// Open at start.
    pub at_start: bool,
    /// Keep on top.
    pub on_top: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            per_floor: DEFAULT_PER_FLOOR,
            motion: Motion::Full,
            decay: true,
            at_start: false,
            on_top: false,
        }
    }
}

impl Settings {
    /// The settings from their stored words (defaults for the rest).
    #[must_use]
    pub fn from_words(get: impl Fn(&str) -> Option<String>) -> Self {
        let d = Self::default();
        let on = |key: &str, default: bool| get(key).map_or(default, |v| v == "on");
        Self {
            per_floor: get(PER_FLOOR_KEY)
                .and_then(|v| v.parse().ok())
                .unwrap_or(d.per_floor)
                .clamp(4, 16),
            motion: match get(MOTION_KEY).as_deref() {
                Some("calm") => Motion::Calm,
                Some("off") => Motion::Off,
                _ => Motion::Full,
            },
            decay: on(DECAY_KEY, d.decay),
            at_start: on(AT_START_KEY, d.at_start),
            on_top: on(ON_TOP_KEY, d.on_top),
        }
    }

    /// The settings as stored.
    ///
    /// # Errors
    ///
    /// Storage errors.
    pub fn read(storage: &Storage) -> AppResult<Self> {
        let settings = storage.app_settings();
        let mut words = std::collections::BTreeMap::new();
        for key in [
            PER_FLOOR_KEY,
            MOTION_KEY,
            DECAY_KEY,
            AT_START_KEY,
            ON_TOP_KEY,
        ] {
            if let Some(v) = settings.get(key)? {
                words.insert(key, v);
            }
        }
        Ok(Self::from_words(|k| words.get(k).cloned()))
    }
}

/// The office from its saved rows (hidden Apps left out) and seed.
#[must_use]
pub fn office_from(rows: &[FloorArea], seed: u32, per_floor: usize) -> Office {
    let mut office = Office::new(seed, per_floor);
    for row in rows.iter().filter(|r| !r.hidden) {
        let family = Family::of(Some(&row.family));
        let layout = Layout::parse(&row.layout)
            .filter(|l| family.layouts().contains(l))
            .unwrap_or(family.layouts()[0]);
        office.apps.insert(
            row.app_id.clone(),
            AreaState {
                order: row.order,
                family,
                choice: Choice {
                    layout,
                    seed: row.seed,
                },
                desks: (row.desks as usize).max(6),
                fits_smaller_since: row.fits_smaller_since,
            },
        );
    }
    office
}

/// The rows to save for `office` (every App it has seen).
#[must_use]
pub fn rows_of(office: &Office) -> Vec<FloorArea> {
    office
        .apps
        .iter()
        .map(|(id, a)| FloorArea {
            app_id: id.clone(),
            order: a.order,
            family: a.family.as_str().to_owned(),
            layout: a.choice.layout.as_str().to_owned(),
            seed: a.choice.seed,
            desks: u32::try_from(a.desks).unwrap_or(u32::MAX),
            fits_smaller_since: a.fits_smaller_since,
            hidden: false,
        })
        .collect()
}

/// The saved office, a new seed saved the first time; the hidden Apps.
///
/// # Errors
///
/// Storage errors.
pub fn load(
    storage: &Storage,
    per_floor: usize,
    fresh_seed: u32,
) -> AppResult<(Office, Vec<String>)> {
    let settings = storage.app_settings();
    let saved = settings.get(FLOOR_SEED_KEY)?.and_then(|v| v.parse().ok());
    let seed = if let Some(seed) = saved {
        seed
    } else {
        settings.set(FLOOR_SEED_KEY, &fresh_seed.to_string())?;
        fresh_seed
    };
    let rows = storage.floor().areas()?;
    let hidden = rows
        .iter()
        .filter(|r| r.hidden)
        .map(|r| r.app_id.clone())
        .collect();
    Ok((office_from(&rows, seed, per_floor), hidden))
}

/// Saves what the office learnt (Apps whose row the store refuses, such as
/// one removed meanwhile, are left out).
///
/// # Errors
///
/// Storage errors.
pub fn save(storage: &Storage, office: &Office) -> AppResult<()> {
    let known: std::collections::BTreeSet<String> =
        storage.apps().list()?.into_iter().map(|a| a.id).collect();
    let rows: Vec<FloorArea> = rows_of(office)
        .into_iter()
        .filter(|r| known.contains(&r.app_id))
        .collect();
    storage.floor().save(&rows)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use juggler_floor::driver::Motion;
    use juggler_floor::family::Family;

    use super::{DECAY_KEY, MOTION_KEY, PER_FLOOR_KEY, Settings, office_from, rows_of};
    use juggler_floor::layout::Office;

    #[test]
    fn settings_read_their_words_with_defaults_and_limits() {
        assert_eq!(Settings::from_words(|_| None), Settings::default());
        let words: BTreeMap<&str, &str> = BTreeMap::from([
            (PER_FLOOR_KEY, "40"),
            (MOTION_KEY, "calm"),
            (DECAY_KEY, "off"),
        ]);
        let s = Settings::from_words(|k| words.get(k).map(|v| (*v).to_owned()));
        assert_eq!((s.per_floor, s.motion, s.decay), (16, Motion::Calm, false));
    }

    #[test]
    fn the_office_round_trips_through_its_rows_and_hidden_apps_stay_out() {
        let mut office = Office::new(9, 10);
        office.observe(
            &[
                ("a".into(), Family::Industrial, 3),
                ("b".into(), Family::Studio, 14),
            ],
            1,
        );
        let mut rows = rows_of(&office);
        let back = office_from(&rows, 9, 10);
        assert_eq!(back.apps, office.apps);
        rows[0].hidden = true;
        rows[1].layout = "a layout this build doesn't know".into();
        let partial = office_from(&rows, 9, 10);
        assert_eq!(partial.apps.len(), 1, "a hidden App stays off the Floor");
        let b = &partial.apps["b"];
        assert!(
            Family::Studio.layouts().contains(&b.choice.layout),
            "an unknown layout falls back to the family's first"
        );
    }
}
