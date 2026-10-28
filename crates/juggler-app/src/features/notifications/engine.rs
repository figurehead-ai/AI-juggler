//! AJ-E1510: which system notifications are due, as plain data. The
//! engine sees each session's state as it changes and This Mac's room;
//! the caller says what the user allows now (the kinds turned on, quiet
//! hours, accounts left out, AI Juggler in front). Nothing here talks to
//! the system.

use std::collections::HashMap;

/// What a notification is about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// A session waits for the user.
    Waiting,
    /// A session looks stuck.
    Stuck,
    /// A session hit its usage limit.
    Limit,
    /// A session finished on its own.
    Finished,
    /// This Mac is getting tight or under pressure.
    MacTight,
    /// AJ-E2405: a session ended and left processes running.
    LeftBehind,
    /// AJ-E2501: an account is almost out of room.
    Quota,
    /// AJ-E2501: a session crashed and was started again.
    Crashed,
    /// AJ-E2501: an account has room again (its window reset).
    Reset,
}

impl Kind {
    /// The kind a session status enters, if any (`SessionStatus::as_str`).
    #[must_use]
    pub fn of_status(status: &str) -> Option<Self> {
        match status {
            "waiting_user" => Some(Self::Waiting),
            "stuck" => Some(Self::Stuck),
            "quota_blocked" => Some(Self::Limit),
            "finished" => Some(Self::Finished),
            _ => None,
        }
    }

    /// How soon the same kind may come again for the same thing after it
    /// was shown and left: a session flickering between working and
    /// waiting asks once, and This Mac's room as often as its own alerts.
    const fn rearm_ms(self) -> i64 {
        match self {
            Self::MacTight | Self::Quota => 30 * 60_000,
            _ => 5 * 60_000,
        }
    }
}

/// What a click opens.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Target {
    /// A session, by id.
    Session(String),
    /// The This Mac page.
    ThisMac,
    /// AJ-E2501: an account, by id.
    Account(String),
}

/// One notification due now.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Due {
    /// What it says.
    pub kind: Kind,
    /// What a click opens.
    pub target: Target,
}

/// What the user allows at this moment.
pub struct Gate<'a> {
    /// The kinds turned on in Settings › Notifications.
    pub allowed: &'a dyn Fn(Kind) -> bool,
    /// Inside the quiet hours (an unknown hour counts as quiet, as the
    /// digest does).
    pub quiet: bool,
    /// AI Juggler's window is in front: the user sees it there.
    pub in_front: bool,
    /// The session's account is left out of the digest and notifications.
    pub muted: &'a dyn Fn(&Target) -> bool,
}

#[derive(Default)]
struct Track {
    /// The kind the target is in now (`None`: nothing to tell).
    now: Option<Kind>,
    /// Not yet shown for the current state (held through quiet hours).
    pending: bool,
    /// When each kind was last shown.
    shown_ms: HashMap<Kind, i64>,
}

/// Every target's state and what has been shown.
#[derive(Default)]
pub struct Engine {
    tracks: HashMap<Target, Track>,
}

impl Engine {
    /// The target's state now: `Some(kind)` for a state worth telling,
    /// `None` for any other. Each entry into a state can be told once.
    pub fn observe(&mut self, target: Target, kind: Option<Kind>, now_ms: i64) {
        let track = self.tracks.entry(target).or_default();
        if track.now == kind {
            return;
        }
        track.now = kind;
        track.pending = kind.is_some_and(|kind| {
            track
                .shown_ms
                .get(&kind)
                .is_none_or(|at| now_ms - at >= kind.rearm_ms())
        });
    }

    /// The notifications to show now. A state the user doesn't want told
    /// (its kind off, its account left out, AI Juggler in front) is
    /// dropped; quiet hours hold it until they end, if it still holds.
    pub fn due(&mut self, gate: &Gate<'_>, now_ms: i64) -> Vec<Due> {
        let mut due = Vec::new();
        for (target, track) in &mut self.tracks {
            let Some(kind) = track.now.filter(|_| track.pending) else {
                continue;
            };
            if !(gate.allowed)(kind) || gate.in_front || (gate.muted)(target) {
                track.pending = false;
                continue;
            }
            if gate.quiet {
                continue;
            }
            track.pending = false;
            track.shown_ms.insert(kind, now_ms);
            due.push(Due {
                kind,
                target: target.clone(),
            });
        }
        // Gone quiet long enough that nothing would hold it back: forgotten.
        self.tracks.retain(|_, track| {
            track.now.is_some()
                || track
                    .shown_ms
                    .iter()
                    .any(|(kind, at)| now_ms - at < kind.rearm_ms())
        });
        due.sort_by(|a, b| a.target.cmp(&b.target));
        due
    }

    /// Whether anything waits to be shown (quiet hours, or not yet asked).
    #[must_use]
    pub fn pending(&self) -> bool {
        self.tracks.values().any(|track| track.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    fn open(_: Kind) -> bool {
        true
    }
    fn nobody(_: &Target) -> bool {
        false
    }
    fn gate() -> Gate<'static> {
        Gate {
            allowed: &open,
            quiet: false,
            in_front: false,
            muted: &nobody,
        }
    }
    fn s(id: &str) -> Target {
        Target::Session(id.into())
    }

    #[test]
    fn each_state_is_told_once() {
        let mut engine = Engine::default();
        engine.observe(s("a"), Some(Kind::Waiting), 0);
        assert_eq!(engine.due(&gate(), 0).len(), 1);
        // Still waiting: nothing more.
        engine.observe(s("a"), Some(Kind::Waiting), MIN);
        assert!(engine.due(&gate(), MIN).is_empty());
        // A different state is its own news.
        engine.observe(s("a"), Some(Kind::Finished), 2 * MIN);
        assert_eq!(
            engine.due(&gate(), 2 * MIN),
            vec![Due {
                kind: Kind::Finished,
                target: s("a")
            }]
        );
    }

    #[test]
    fn a_session_flickering_back_to_waiting_asks_once_in_five_minutes() {
        let mut engine = Engine::default();
        engine.observe(s("a"), Some(Kind::Waiting), 0);
        assert_eq!(engine.due(&gate(), 0).len(), 1);
        engine.observe(s("a"), None, MIN);
        engine.observe(s("a"), Some(Kind::Waiting), 2 * MIN);
        assert!(engine.due(&gate(), 2 * MIN).is_empty());
        engine.observe(s("a"), None, 3 * MIN);
        engine.observe(s("a"), Some(Kind::Waiting), 6 * MIN);
        assert_eq!(engine.due(&gate(), 6 * MIN).len(), 1);
    }

    #[test]
    fn quiet_hours_hold_it_and_it_comes_after_if_it_still_holds() {
        let mut engine = Engine::default();
        let quiet = Gate {
            quiet: true,
            ..gate()
        };
        engine.observe(s("a"), Some(Kind::Waiting), 0);
        engine.observe(s("b"), Some(Kind::Stuck), 0);
        assert!(engine.due(&quiet, 0).is_empty());
        assert!(engine.due(&quiet, MIN).is_empty());
        // b got going again during the night: only a is still news.
        engine.observe(s("b"), None, 2 * MIN);
        assert_eq!(
            engine.due(&gate(), 3 * MIN),
            vec![Due {
                kind: Kind::Waiting,
                target: s("a")
            }]
        );
        assert!(engine.due(&gate(), 4 * MIN).is_empty());
    }

    #[test]
    fn a_kind_turned_off_an_account_left_out_or_the_app_in_front_drops_it() {
        let mut engine = Engine::default();
        let no_finished = |kind: Kind| kind != Kind::Finished;
        let off = Gate {
            allowed: &no_finished,
            ..gate()
        };
        engine.observe(s("a"), Some(Kind::Finished), 0);
        assert!(engine.due(&off, 0).is_empty());
        // Turned on later: that finish isn't told late.
        assert!(engine.due(&gate(), MIN).is_empty());

        let left_out = |target: &Target| *target == s("b");
        let muted = Gate {
            muted: &left_out,
            ..gate()
        };
        engine.observe(s("b"), Some(Kind::Waiting), 0);
        engine.observe(s("c"), Some(Kind::Waiting), 0);
        assert_eq!(
            engine.due(&muted, 0),
            vec![Due {
                kind: Kind::Waiting,
                target: s("c")
            }]
        );

        let front = Gate {
            in_front: true,
            ..gate()
        };
        engine.observe(s("d"), Some(Kind::Limit), 0);
        assert!(engine.due(&front, 0).is_empty());
        assert!(engine.due(&gate(), MIN).is_empty());
    }

    #[test]
    fn this_mac_is_told_again_only_after_half_an_hour() {
        let mut engine = Engine::default();
        engine.observe(Target::ThisMac, Some(Kind::MacTight), 0);
        assert_eq!(engine.due(&gate(), 0).len(), 1);
        engine.observe(Target::ThisMac, None, 5 * MIN);
        engine.observe(Target::ThisMac, Some(Kind::MacTight), 10 * MIN);
        assert!(engine.due(&gate(), 10 * MIN).is_empty());
        engine.observe(Target::ThisMac, None, 20 * MIN);
        engine.observe(Target::ThisMac, Some(Kind::MacTight), 31 * MIN);
        assert_eq!(engine.due(&gate(), 31 * MIN).len(), 1);
    }

    #[test]
    fn only_the_four_session_states_are_news() {
        for (status, kind) in [
            ("waiting_user", Some(Kind::Waiting)),
            ("stuck", Some(Kind::Stuck)),
            ("quota_blocked", Some(Kind::Limit)),
            ("finished", Some(Kind::Finished)),
            ("running", None),
            ("failed", None),
            ("stopped", None),
            ("handed_over", None),
        ] {
            assert_eq!(Kind::of_status(status), kind, "{status}");
        }
    }
}
