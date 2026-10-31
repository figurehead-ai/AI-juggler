//! Keep alive (0.3.3; SPEC §1's third amendment): an app or profile the
//! user turned it on for is started again when it stops running, however
//! it stopped (a crash, ⌘Q, Force Quit), behind the app in front.
//!
//! Gone means its main process missing twice in a row (4 s); then it
//! waits [`WAIT_MS`] and starts it, exactly as it ran (`start::plan`),
//! never on another account. Never while the screen is locked, within a
//! minute of AI Juggler starting or of the computer waking, or once AI
//! Juggler is quitting (it quits with the other apps when the user logs
//! out or shuts down, so the wait leaves it time to stand down). The same
//! limits as a session's restart (AJ-E1509): [`TRIES`] starts in
//! [`WINDOW_MS`], then keep alive pauses and says why. A Stop from AI
//! Juggler pauses it too, until the next Open.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::features::sessions::restart::{TRIES, WINDOW_MS};

/// Misses in a row that mean gone (one tick can miss a process mid-read).
pub const MISSES: u8 = 2;
/// How long after it's gone it's started: a second to quit and reopen it
/// by hand first.
pub const WAIT_MS: i64 = 5_000;
/// Quiet after AI Juggler starts or the computer wakes: apps come back on
/// their own then.
pub const SETTLE_MS: i64 = 60_000;
/// A gap between ticks this long means the computer slept.
pub const SLEPT_MS: i64 = 30_000;
/// How long a start has to show up before it counts as gone again.
pub const STARTING_MS: i64 = 30_000;

/// Set once AI Juggler is quitting: nothing starts after that.
static STANDING_DOWN: AtomicBool = AtomicBool::new(false);

/// AI Juggler is quitting (any way: ⌘Q, a log out, a shut down).
pub fn stand_down() {
    STANDING_DOWN.store(true, Ordering::SeqCst);
}

/// Whether AI Juggler is quitting.
#[must_use]
pub fn standing_down() -> bool {
    STANDING_DOWN.load(Ordering::SeqCst)
}

/// What to do for a kept-alive place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Start it: try `n` of [`TRIES`].
    Start(String, usize),
    /// Pause keep alive for it, and why.
    Pause(String, String),
}

/// One kept-alive place, as the step needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    /// Its id.
    pub id: String,
    /// Running now (after [`Watch::seen`]).
    pub running: bool,
    /// Why its account may only be used by a person at the keyboard.
    pub only_by_you: Option<String>,
}

/// What the watcher remembers between ticks (in memory: an AI Juggler
/// restart counts the tries again, as a session's do).
#[derive(Debug, Default)]
pub struct Watch {
    /// Each place's main process, as last found.
    pids: HashMap<String, (u32, Option<i64>)>,
    misses: HashMap<String, u8>,
    gone_at: HashMap<String, i64>,
    started_at: HashMap<String, i64>,
    tries: HashMap<String, Vec<i64>>,
    quiet_until_ms: i64,
    last_tick_ms: Option<i64>,
    /// When the whole process list was last read to find a place.
    pub searched_ms: Option<i64>,
}

impl Watch {
    /// The process a place runs as, when known.
    #[must_use]
    pub fn pid_of(&self, id: &str) -> Option<(u32, Option<i64>)> {
        self.pids.get(id).copied()
    }

    /// The places whose process is known: `(id, pid, start)`.
    #[must_use]
    pub fn known(&self) -> Vec<(String, u32, Option<i64>)> {
        self.pids
            .iter()
            .map(|(id, (pid, start))| (id.clone(), *pid, *start))
            .collect()
    }

    /// `id` found running as `pid`.
    pub fn found(&mut self, id: &str, pid: u32, start: Option<i64>) {
        self.pids.insert(id.to_owned(), (pid, start));
        self.misses.remove(id);
        self.gone_at.remove(id);
        self.started_at.remove(id);
    }

    /// `id` has no process now (seen by a full scan, not watched).
    pub fn forget_pid(&mut self, id: &str) {
        self.pids.remove(id);
        self.misses.remove(id);
    }

    /// Its known process checked now (`alive`): `true` while it still
    /// counts as running (one miss doesn't end it).
    pub fn seen(&mut self, id: &str, alive: bool) -> bool {
        if alive {
            self.misses.remove(id);
            return true;
        }
        let misses = self.misses.entry(id.to_owned()).or_insert(0);
        *misses = misses.saturating_add(1);
        if *misses < MISSES {
            return true;
        }
        self.pids.remove(id);
        self.misses.remove(id);
        false
    }

    /// The clock moved on to `now_ms`: quiet for [`SETTLE_MS`] after the
    /// first tick (AI Juggler started) and after a sleep.
    pub fn clock(&mut self, now_ms: i64) {
        let slept = self
            .last_tick_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= SLEPT_MS);
        if slept {
            self.quiet_until_ms = now_ms.saturating_add(SETTLE_MS);
        }
        self.last_tick_ms = Some(now_ms);
    }

    /// Test seam (the live walk too): AI Juggler's first minute already
    /// over at `now_ms`.
    #[doc(hidden)]
    pub fn settled_at(&mut self, now_ms: i64) {
        self.last_tick_ms = Some(now_ms);
        self.quiet_until_ms = 0;
    }

    /// Forgets a place (keep alive off, or the place forgotten), and its
    /// tries: turning it on again starts the count again.
    pub fn drop_place(&mut self, id: &str) {
        self.misses.remove(id);
        self.gone_at.remove(id);
        self.started_at.remove(id);
        self.tries.remove(id);
    }

    /// What to do now for the `kept` places. `locked` is asked only when
    /// a start is due (it reads the system).
    pub fn step(&mut self, now_ms: i64, kept: &[Kept], locked: &dyn Fn() -> bool) -> Vec<Act> {
        let mut acts = Vec::new();
        let mut asked: Option<bool> = None;
        for place in kept {
            if place.running {
                self.gone_at.remove(&place.id);
                self.started_at.remove(&place.id);
                continue;
            }
            let gone = *self.gone_at.entry(place.id.clone()).or_insert(now_ms);
            let starting = self
                .started_at
                .get(&place.id)
                .is_some_and(|at| now_ms.saturating_sub(*at) < STARTING_MS);
            if starting
                || now_ms.saturating_sub(gone) < WAIT_MS
                || now_ms < self.quiet_until_ms
                || standing_down()
            {
                continue;
            }
            if *asked.get_or_insert_with(locked) {
                continue;
            }
            let earlier = self.tries.entry(place.id.clone()).or_default();
            earlier.retain(|at| now_ms.saturating_sub(*at) < WINDOW_MS);
            if earlier.len() >= TRIES {
                acts.push(Act::Pause(
                    place.id.clone(),
                    format!("it stopped {TRIES} times in 10 minutes"),
                ));
                continue;
            }
            if let Some(why) = &place.only_by_you {
                acts.push(Act::Pause(place.id.clone(), why.clone()));
                continue;
            }
            earlier.push(now_ms);
            let attempt = earlier.len();
            self.started_at.insert(place.id.clone(), now_ms);
            acts.push(Act::Start(place.id.clone(), attempt));
        }
        acts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(id: &str) -> Kept {
        Kept {
            id: id.into(),
            running: false,
            only_by_you: None,
        }
    }

    fn unlocked() -> bool {
        false
    }

    /// A watch past AI Juggler's first minute.
    fn settled(now: i64) -> Watch {
        let mut watch = Watch::default();
        watch.clock(now - SETTLE_MS - 2_000);
        let mut at = now - SETTLE_MS - 2_000;
        while at < now {
            at += 2_000;
            watch.clock(at);
        }
        watch
    }

    #[test]
    fn one_miss_isnt_gone_two_are() {
        let mut watch = Watch::default();
        watch.found("a", 10, Some(1));
        assert!(watch.seen("a", false), "one miss");
        assert!(watch.seen("a", true));
        assert!(watch.seen("a", false));
        assert!(!watch.seen("a", false), "two in a row");
        assert_eq!(watch.pid_of("a"), None);
    }

    #[test]
    fn a_stopped_app_starts_after_the_wait_and_once() {
        let now = 1_000_000;
        let mut watch = settled(now);
        assert!(watch.step(now, &[down("a")], &unlocked).is_empty(), "it waits first");
        assert!(watch.step(now + 4_000, &[down("a")], &unlocked).is_empty());
        assert_eq!(
            watch.step(now + WAIT_MS, &[down("a")], &unlocked),
            [Act::Start("a".into(), 1)]
        );
        // Still coming up: never a second start meanwhile.
        assert!(watch.step(now + WAIT_MS + 2_000, &[down("a")], &unlocked).is_empty());
    }

    #[test]
    fn three_stops_in_ten_minutes_pause_it() {
        let mut now = 1_000_000;
        let mut watch = settled(now);
        for attempt in 1..=TRIES {
            watch.step(now, &[down("a")], &unlocked);
            let acts = watch.step(now + WAIT_MS, &[down("a")], &unlocked);
            assert_eq!(acts, [Act::Start("a".into(), attempt)]);
            // It came up, then stopped again.
            watch.found("a", 10, None);
            now += STARTING_MS + WAIT_MS;
        }
        watch.step(now, &[down("a")], &unlocked);
        let acts = watch.step(now + WAIT_MS, &[down("a")], &unlocked);
        assert!(matches!(&acts[..], [Act::Pause(id, why)] if id == "a" && why.contains("3 times")));
        // Ten minutes on, the count starts again (once turned back on).
        watch.drop_place("a");
        assert!(!watch.tries.contains_key("a"));
    }

    #[test]
    fn never_when_locked_while_settling_after_a_sleep_or_quitting() {
        let now = 1_000_000;
        let mut watch = settled(now);
        watch.step(now, &[down("a")], &unlocked);
        assert!(watch.step(now + WAIT_MS, &[down("a")], &|| true).is_empty(), "locked");
        // The computer slept: a minute's quiet from waking.
        let woke = now + 3_600_000;
        watch.clock(woke);
        assert!(watch.step(woke, &[down("a")], &unlocked).is_empty());
        assert!(watch.step(woke + SETTLE_MS - 1, &[down("a")], &unlocked).is_empty());
        assert_eq!(watch.step(woke + SETTLE_MS, &[down("a")], &unlocked).len(), 1);
    }

    #[test]
    fn a_fresh_start_waits_a_minute_and_an_interactive_only_account_pauses() {
        let mut watch = Watch::default();
        watch.clock(0);
        assert!(watch.step(WAIT_MS, &[down("a")], &unlocked).is_empty(), "AI Juggler just started");
        let mut only = down("b");
        only.only_by_you = Some("its tool's terms allow interactive use only".into());
        watch.clock(2_000);
        assert!(watch.step(SETTLE_MS, &[only.clone()], &unlocked).is_empty());
        let acts = watch.step(SETTLE_MS + WAIT_MS, &[only], &unlocked);
        assert!(matches!(&acts[..], [Act::Pause(id, _)] if id == "b"));
    }

    #[test]
    fn a_running_app_is_left_alone() {
        let now = 1_000_000;
        let mut watch = settled(now);
        let up = Kept {
            id: "a".into(),
            running: true,
            only_by_you: None,
        };
        let asked = std::cell::Cell::new(false);
        let locked = || {
            asked.set(true);
            false
        };
        assert!(watch.step(now + 60_000, &[up], &locked).is_empty());
        assert!(!asked.get(), "the lock is read only when a start is due");
    }
}
