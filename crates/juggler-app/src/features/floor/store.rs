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
/// D-80: how it's drawn: `pixel` (Pixel art, the default) or `full`
/// (Full resolution).
pub const DETAIL_KEY: &str = "floor.detail";
/// AJ-E3520: the light: `clock` (follow my clock, the default), `day` or
/// `night`.
pub const LIGHT_KEY: &str = "floor.light";
/// AJ-E3519: which Apps show: `all` (the default) or `active`.
pub const SHOW_KEY: &str = "floor.show";
/// AJ-E3519: how areas are arranged: `kept` (their kept places, the
/// default) or `packed` (active Apps together; a view only).
pub const ARRANGE_KEY: &str = "floor.arrange";
/// AJ-E3515, AJ-E3519: which labels show: `smart` (by zoom, the
/// default), `attention` (only what needs you) or `names` (every area's
/// name).
pub const LABELS_KEY: &str = "floor.labels";

/// AJ-E3520: the office's light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Light {
    /// The sun where the Mac's time zone is, now.
    #[default]
    Clock,
    /// Always day.
    Day,
    /// Always night.
    Night,
}

/// AJ-E3515: which labels show.
pub use juggler_floor::flags::Labels;

/// AJ-E3519: which Apps show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    /// Every App the Floor has.
    #[default]
    All,
    /// Only the Apps in use (unused ones out of view).
    Active,
}

/// AJ-E3519: how the areas are arranged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Arrange {
    /// Each in its kept place.
    #[default]
    Kept,
    /// Active Apps packed together (a view only: nothing saved).
    Packed,
}

/// A setting's stored word and back, for the ones with a few choices.
pub trait Word: Sized + Copy + 'static {
    /// Every choice with its word, the default first.
    const WORDS: &'static [(Self, &'static str)];

    /// The stored word.
    fn word(self) -> &'static str
    where
        Self: PartialEq,
    {
        Self::WORDS
            .iter()
            .find(|(v, _)| *v == self)
            .map_or(Self::WORDS[0].1, |(_, w)| w)
    }

    /// The choice for a stored word (the default for anything else).
    fn parse(word: Option<&str>) -> Self {
        Self::WORDS
            .iter()
            .find(|(_, w)| Some(*w) == word)
            .map_or(Self::WORDS[0].0, |(v, _)| *v)
    }
}

impl Word for Light {
    const WORDS: &'static [(Self, &'static str)] = &[
        (Self::Clock, "clock"),
        (Self::Day, "day"),
        (Self::Night, "night"),
    ];
}

impl Word for Show {
    const WORDS: &'static [(Self, &'static str)] = &[(Self::All, "all"), (Self::Active, "active")];
}

impl Word for Arrange {
    const WORDS: &'static [(Self, &'static str)] =
        &[(Self::Kept, "kept"), (Self::Packed, "packed")];
}

impl Word for Labels {
    const WORDS: &'static [(Self, &'static str)] = &[
        (Self::Smart, "smart"),
        (Self::Attention, "attention"),
        (Self::Names, "names"),
    ];
}

/// D-80: how the Floor is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Detail {
    /// Pixel art: the scene at its pixel size, scaled up (the approved
    /// design).
    #[default]
    Pixel,
    /// Full resolution: the scene at the screen's own pixels, smoothed.
    Full,
}

impl Detail {
    /// Full resolution.
    #[must_use]
    pub const fn full(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// How much moves, as chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionChoice {
    /// Full, or Off while the system's Reduce motion is on.
    #[default]
    System,
    /// Always the chosen one.
    Fixed(Motion),
}

impl MotionChoice {
    /// The motion now, given whether the system asks for less.
    #[must_use]
    pub const fn resolve(self, reduce: bool) -> Motion {
        match self {
            Self::System if reduce => Motion::Off,
            Self::System => Motion::Full,
            Self::Fixed(m) => m,
        }
    }
}

/// The Floor's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Apps a floor holds.
    pub per_floor: usize,
    /// How much moves.
    pub motion: MotionChoice,
    /// Show unused Apps' decay.
    pub decay: bool,
    /// Open at start.
    pub at_start: bool,
    /// Keep on top.
    pub on_top: bool,
    /// D-80: Pixel art or Full resolution.
    pub detail: Detail,
    /// AJ-E3520: the light.
    pub light: Light,
    /// AJ-E3519: which Apps show.
    pub show: Show,
    /// AJ-E3519: how the areas are arranged.
    pub arrange: Arrange,
    /// AJ-E3515: which labels show.
    pub labels: Labels,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            per_floor: DEFAULT_PER_FLOOR,
            motion: MotionChoice::System,
            decay: true,
            at_start: false,
            on_top: false,
            detail: Detail::Pixel,
            light: Light::Clock,
            show: Show::All,
            arrange: Arrange::Kept,
            labels: Labels::Smart,
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
                Some("full") => MotionChoice::Fixed(Motion::Full),
                Some("calm") => MotionChoice::Fixed(Motion::Calm),
                Some("off") => MotionChoice::Fixed(Motion::Off),
                _ => MotionChoice::System,
            },
            decay: on(DECAY_KEY, d.decay),
            at_start: on(AT_START_KEY, d.at_start),
            on_top: on(ON_TOP_KEY, d.on_top),
            detail: if get(DETAIL_KEY).as_deref() == Some("full") {
                Detail::Full
            } else {
                Detail::Pixel
            },
            light: Light::parse(get(LIGHT_KEY).as_deref()),
            show: Show::parse(get(SHOW_KEY).as_deref()),
            arrange: Arrange::parse(get(ARRANGE_KEY).as_deref()),
            labels: Labels::parse(get(LABELS_KEY).as_deref()),
        }
    }

    /// The motion now (the system's Reduce motion read for Follow the
    /// system).
    #[must_use]
    pub fn motion_now(&self) -> Motion {
        self.motion.resolve(juggler_window_events::reduce_motion())
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
            DETAIL_KEY,
            LIGHT_KEY,
            SHOW_KEY,
            ARRANGE_KEY,
            LABELS_KEY,
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

    use super::{
        ARRANGE_KEY, Arrange, DECAY_KEY, DETAIL_KEY, Detail, LABELS_KEY, LIGHT_KEY, Labels, Light,
        MOTION_KEY, MotionChoice, PER_FLOOR_KEY, SHOW_KEY, Settings, Show, Word, office_from,
        rows_of,
    };
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
        assert_eq!(
            (s.per_floor, s.motion, s.decay),
            (16, MotionChoice::Fixed(Motion::Calm), false)
        );
    }

    #[test]
    fn pixel_art_is_the_default_and_full_resolution_is_kept_as_chosen() {
        assert_eq!(Settings::default().detail, Detail::Pixel);
        let full = Settings::from_words(|k| (k == DETAIL_KEY).then(|| "full".to_owned()));
        assert_eq!(full.detail, Detail::Full);
        assert!(full.detail.full());
        let odd = Settings::from_words(|k| (k == DETAIL_KEY).then(|| "huge".to_owned()));
        assert_eq!(odd.detail, Detail::Pixel, "an unknown word is Pixel art");
    }

    #[test]
    fn the_view_menu_s_choices_read_their_words_with_defaults() {
        let d = Settings::from_words(|_| None);
        assert_eq!(
            (d.light, d.show, d.arrange, d.labels),
            (Light::Clock, Show::All, Arrange::Kept, Labels::Smart)
        );
        let words: BTreeMap<&str, &str> = [
            (LIGHT_KEY, "night"),
            (SHOW_KEY, "active"),
            (ARRANGE_KEY, "packed"),
            (LABELS_KEY, "attention"),
        ]
        .into();
        let s = Settings::from_words(|k| words.get(k).map(|v| (*v).to_owned()));
        assert_eq!(
            (s.light, s.show, s.arrange, s.labels),
            (
                Light::Night,
                Show::Active,
                Arrange::Packed,
                Labels::Attention
            )
        );
        for l in [Light::Clock, Light::Day, Light::Night] {
            assert_eq!(Light::parse(Some(l.word())), l);
        }
        assert_eq!(Light::parse(Some("dusk")), Light::Clock);
    }

    #[test]
    fn follow_the_system_stops_motion_when_reduce_motion_is_on() {
        let s = Settings::default();
        assert_eq!(s.motion, MotionChoice::System);
        assert_eq!(s.motion.resolve(false), Motion::Full);
        assert_eq!(s.motion.resolve(true), Motion::Off);
        assert_eq!(
            MotionChoice::Fixed(Motion::Calm).resolve(true),
            Motion::Calm,
            "a chosen motion stays as chosen"
        );
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
