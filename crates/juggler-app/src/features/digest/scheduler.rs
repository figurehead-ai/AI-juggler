//! The digest's scheduler (ADR-0036 §1.6): event-driven, pure, on the
//! caller's clock. Nothing changed, no digest and no timer. With changed
//! agents, the next digest is due after an interval that shortens as more
//! agents work; a notable event brings it forward to a minute later (a
//! burst is one digest), never sooner than five minutes after the last;
//! while the user is away nothing is due, and on return one digest covers
//! the whole window at once. A skip (the battery) keeps what changed for
//! the next one.

use std::collections::BTreeMap;

/// A notable event's digest waits this long, so a burst becomes one.
pub const NOTABLE_DELAY_MS: i64 = 60_000;
/// Never a digest sooner than this after the previous one.
pub const MIN_GAP_MS: i64 = 5 * 60_000;
/// An agent changed this recently counts as active.
pub const ACTIVE_MS: i64 = 30 * 60_000;
/// The first digest's window reaches back at least this far.
pub const FIRST_WINDOW_MS: i64 = 30 * 60_000;
/// The compiled bounds of the interval, minutes (the remote param
/// `digest_interval_min` may narrow them, ADR-0035 §2).
pub const INTERVAL_BOUNDS_MIN: (u32, u32) = (10, 60);

/// The interval for this many active agents: 30 minutes for 1, 25 for 2,
/// 20 for 3, 15 for 4 or more, within `bounds` (minutes).
#[must_use]
pub fn interval_ms(active: usize, bounds: (u32, u32)) -> i64 {
    let minutes: u32 = match active {
        0 | 1 => 30,
        2 => 25,
        3 => 20,
        _ => 15,
    };
    let (low, high) = (bounds.0.min(bounds.1), bounds.0.max(bounds.1));
    i64::from(minutes.clamp(low, high)) * 60_000
}

/// A digest to make now: its window and the agents that changed in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    /// The window's start, epoch ms.
    pub start_ms: i64,
    /// Its end (now).
    pub end_ms: i64,
    /// The agents that changed, by session id, in order.
    pub agents: Vec<String>,
}

/// The scheduler's state.
#[derive(Clone, Debug)]
pub struct Scheduler {
    bounds: (u32, u32),
    last_end_ms: Option<i64>,
    changed: BTreeMap<String, i64>,
    first_change_ms: Option<i64>,
    notable_ms: Option<i64>,
    away: bool,
    returned_ms: Option<i64>,
    not_before_ms: Option<i64>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(INTERVAL_BOUNDS_MIN)
    }
}

impl Scheduler {
    /// A scheduler whose interval stays within `bounds`, minutes.
    #[must_use]
    pub const fn new(bounds: (u32, u32)) -> Self {
        Self {
            bounds,
            last_end_ms: None,
            changed: BTreeMap::new(),
            first_change_ms: None,
            notable_ms: None,
            away: false,
            returned_ms: None,
            not_before_ms: None,
        }
    }

    /// Something happened to `agent` at `now_ms`; `notable` for a new
    /// need, a session's ending or a crash.
    pub fn heard(&mut self, agent: &str, notable: bool, now_ms: i64) {
        self.changed.insert(agent.to_owned(), now_ms);
        self.first_change_ms.get_or_insert(now_ms);
        if notable {
            self.notable_ms.get_or_insert(now_ms);
        }
    }

    /// The user went away (no digest is written for no one), or came back
    /// (one digest covers the whole window, at once).
    pub fn set_away(&mut self, away: bool, now_ms: i64) {
        if self.away && !away {
            self.returned_ms = Some(now_ms);
        }
        self.away = away;
    }

    /// When the next digest is due; `None` with nothing changed or while
    /// the user is away (no timer then).
    #[must_use]
    pub fn due_at(&self) -> Option<i64> {
        if self.away || self.changed.is_empty() {
            return None;
        }
        let newest = self.changed.values().copied().max()?;
        let active = self
            .changed
            .values()
            .filter(|at| newest - **at <= ACTIVE_MS)
            .count();
        let since = self.last_end_ms.or(self.first_change_ms)?;
        let mut due = since + interval_ms(active, self.bounds);
        let gap = self.last_end_ms.map_or(i64::MIN, |end| end + MIN_GAP_MS);
        if let Some(notable) = self.notable_ms {
            due = due.min((notable + NOTABLE_DELAY_MS).max(gap));
        }
        if let Some(back) = self.returned_ms {
            due = due.min(back);
        }
        if let Some(not_before) = self.not_before_ms {
            due = due.max(not_before);
        }
        Some(due)
    }

    /// The digest due by `now_ms`, if one is: its window runs from the
    /// previous digest's end (the first reaches back at least 30 minutes
    /// and to its first change). What changes from here goes to the next.
    pub fn take(&mut self, now_ms: i64) -> Option<Due> {
        if now_ms < self.due_at()? {
            return None;
        }
        let start_ms = self.last_end_ms.unwrap_or_else(|| {
            let first = self.first_change_ms.unwrap_or(now_ms);
            first.min(now_ms - FIRST_WINDOW_MS)
        });
        let agents = std::mem::take(&mut self.changed).into_keys().collect();
        self.first_change_ms = None;
        self.notable_ms = None;
        self.returned_ms = None;
        self.not_before_ms = None;
        self.last_end_ms = Some(now_ms);
        Some(Due {
            start_ms,
            end_ms: now_ms,
            agents,
        })
    }

    /// No digest now (the battery is critical): what changed is kept and
    /// the next try waits five minutes.
    pub fn defer(&mut self, now_ms: i64) {
        self.not_before_ms = Some(now_ms + MIN_GAP_MS);
    }

    /// The agents changed since the last digest.
    #[must_use]
    pub fn changed(&self) -> usize {
        self.changed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn nothing_changed_nothing_due_and_no_timer() {
        let mut s = Scheduler::default();
        assert_eq!(s.due_at(), None);
        assert_eq!(s.take(10 * 60 * MIN), None);
    }

    #[test]
    fn the_interval_shortens_as_more_agents_work() {
        let mut one = Scheduler::default();
        one.heard("a", false, 0);
        assert_eq!(one.due_at(), Some(30 * MIN));
        let mut four = Scheduler::default();
        for agent in ["a", "b", "c", "d"] {
            four.heard(agent, false, MIN);
        }
        assert_eq!(
            four.due_at(),
            Some(16 * MIN),
            "15 minutes after the first change"
        );
        assert_eq!(four.take(15 * MIN), None, "not before it is due");
        let due = four.take(16 * MIN).unwrap();
        assert_eq!(due.agents, ["a", "b", "c", "d"]);
        assert_eq!(due.start_ms, -14 * MIN, "the first reaches back 30 minutes");
        assert_eq!(due.end_ms, 16 * MIN);
        assert_eq!(four.due_at(), None, "quiet again: nothing armed");
        assert_eq!(interval_ms(2, INTERVAL_BOUNDS_MIN), 25 * MIN);
        assert_eq!(interval_ms(3, INTERVAL_BOUNDS_MIN), 20 * MIN);
        assert_eq!(
            interval_ms(9, (20, 60)),
            20 * MIN,
            "the remote bounds narrow it"
        );
    }

    #[test]
    fn a_notable_burst_is_one_digest_a_minute_later_never_within_five_of_the_last() {
        let mut s = Scheduler::default();
        s.heard("a", true, 0);
        s.heard("b", true, 20_000);
        s.heard("c", true, 40_000);
        assert_eq!(
            s.due_at(),
            Some(MIN),
            "a minute after the first: one digest"
        );
        let first = s.take(MIN).unwrap();
        assert_eq!(first.agents.len(), 3);
        // Another notable right after: held to five minutes past the last.
        s.heard("d", true, 2 * MIN);
        assert_eq!(s.due_at(), Some(MIN + MIN_GAP_MS));
        let second = s.take(MIN + MIN_GAP_MS).unwrap();
        assert_eq!(second.start_ms, MIN, "it covers from the last digest's end");
    }

    #[test]
    fn away_stops_the_cadence_and_the_return_makes_one_digest_at_once() {
        let mut s = Scheduler::default();
        s.heard("a", false, 0);
        s.set_away(true, MIN);
        s.heard("b", true, 2 * MIN);
        s.heard("c", false, 3 * 60 * MIN);
        assert_eq!(s.due_at(), None, "no digest is written for no one");
        s.set_away(false, 4 * 60 * MIN);
        assert_eq!(s.due_at(), Some(2 * MIN + NOTABLE_DELAY_MS), "already due");
        let due = s.take(4 * 60 * MIN).unwrap();
        assert_eq!(due.agents, ["a", "b", "c"], "the whole window in one");
        assert_eq!(
            due.start_ms, 0,
            "from the first change, at least 30 minutes back"
        );
    }

    #[test]
    fn a_skip_keeps_what_changed_for_the_next_digest() {
        let mut s = Scheduler::default();
        s.heard("a", true, 0);
        s.defer(MIN);
        assert_eq!(s.due_at(), Some(MIN + MIN_GAP_MS));
        assert_eq!(s.take(2 * MIN), None);
        s.heard("b", false, 3 * MIN);
        let due = s.take(MIN + MIN_GAP_MS).unwrap();
        assert_eq!(due.agents, ["a", "b"]);
    }

    #[test]
    fn a_change_while_one_is_made_goes_to_the_next() {
        let mut s = Scheduler::default();
        s.heard("a", true, 0);
        let first = s.take(MIN).unwrap();
        assert_eq!(first.agents, ["a"]);
        s.heard("a", false, MIN + 1);
        assert_eq!(s.changed(), 1);
        assert_eq!(s.due_at(), Some(MIN + 30 * MIN));
    }
}
