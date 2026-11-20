//! D-140: the hop limit counts per chain, not per thread.
//!
//! A turn an agent's message woke carries that message's count: anything
//! the woken agent writes, in a new thread too, counts on from it. A
//! person's prompt, click or message starts the count again. Before, a
//! woken agent that wrote in a new thread started at 1, so a pool set to
//! ask after 2 let 4 through (the soak's P11 walk, SK-15).
//!
//! The count lives here, for the turn: it is set when AI Juggler wakes a
//! session (a delivered turn, a typed prompt, a desktop member's wake) and
//! cleared by the person's word to it. A hook's `UserPromptSubmit` that
//! follows a wake is the wake's own prompt; any other is the person's. A
//! desktop member's prompts can't be told from its wakes for sure, so its
//! count is kept only for the loop check's window.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use juggler_core::messaging;

/// How a session was woken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Woken {
    /// A turn AI Juggler started: driven, or typed into its terminal.
    Turn,
    /// A desktop member, through its app's connector.
    Desktop,
}

/// What a woken turn carries.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Carry {
    /// The agent messages in a row behind the turn.
    hop: u32,
    /// The wake's own prompt hasn't been reported by a hook yet.
    prompt_due: bool,
    /// When it lapses (a desktop member's).
    until: Option<Instant>,
}

/// The counts woken turns carry, by session.
#[derive(Debug, Default)]
pub struct Chains {
    carries: Mutex<HashMap<String, Carry>>,
}

/// How long a desktop member's wake carries its count: the loop check's
/// window (`messaging::PING_PONG_MS`).
fn desktop_window() -> Duration {
    Duration::from_millis(u64::try_from(messaging::PING_PONG_MS).unwrap_or(600_000))
}

impl Chains {
    /// `session` is woken by agent messages `hop` in a row (0: the
    /// person's own word is among what woke it, so the count starts
    /// again). Returns what it carried before, to put back should the
    /// wake not happen ([`Chains::put_back`]).
    pub(crate) fn woke(&self, session: &str, hop: u32, how: Woken) -> Option<Carry> {
        let Ok(mut carries) = self.carries.lock() else {
            return None;
        };
        if hop == 0 {
            return carries.remove(session);
        }
        carries.insert(
            session.to_owned(),
            Carry {
                hop,
                prompt_due: true,
                until: (how == Woken::Desktop).then(|| Instant::now() + desktop_window()),
            },
        )
    }

    /// A wake that didn't happen: `session` carries what it did before.
    pub(crate) fn put_back(&self, session: &str, before: Option<Carry>) {
        if let Ok(mut carries) = self.carries.lock() {
            match before {
                Some(carry) => carries.insert(session.to_owned(), carry),
                None => carries.remove(session),
            };
        }
    }

    /// The person spoke to `session` (a prompt, or their message to it):
    /// its count starts again.
    pub(crate) fn person(&self, session: &str) {
        if let Ok(mut carries) = self.carries.lock() {
            carries.remove(session);
        }
    }

    /// `session`'s hook reported a submitted prompt: the wake's own, the
    /// first time after a wake; else the person's.
    pub(crate) fn prompt_submitted(&self, session: &str) {
        let Ok(mut carries) = self.carries.lock() else {
            return;
        };
        match carries.get_mut(session) {
            Some(carry) if carry.prompt_due => carry.prompt_due = false,
            Some(_) => {
                carries.remove(session);
            }
            None => {}
        }
    }

    /// The agent messages in a row behind `session`'s turn now (0 when
    /// the person started it, or nothing woke it).
    #[must_use]
    pub(crate) fn carried(&self, session: &str) -> u32 {
        let Ok(mut carries) = self.carries.lock() else {
            return 0;
        };
        match carries.get(session) {
            Some(carry) if carry.until.is_some_and(|until| Instant::now() >= until) => {
                carries.remove(session);
                0
            }
            Some(carry) => carry.hop,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Chains, Woken};

    #[test]
    fn a_woken_turn_carries_its_count_until_the_person_speaks() {
        let chains = Chains::default();
        assert_eq!(chains.carried("s"), 0, "nothing woke it");
        chains.woke("s", 2, Woken::Turn);
        assert_eq!(chains.carried("s"), 2);
        // The wake's own prompt, as its hook reports it, keeps the count.
        chains.prompt_submitted("s");
        assert_eq!(chains.carried("s"), 2);
        // The next prompt is the person's.
        chains.prompt_submitted("s");
        assert_eq!(chains.carried("s"), 0);
        chains.woke("s", 3, Woken::Turn);
        chains.person("s");
        assert_eq!(chains.carried("s"), 0, "the person's own word");
        chains.woke("s", 3, Woken::Turn);
        chains.woke("s", 0, Woken::Turn);
        assert_eq!(chains.carried("s"), 0, "woken with the person's word");
    }

    #[test]
    fn a_wake_that_didnt_happen_puts_the_count_back() {
        let chains = Chains::default();
        let before = chains.woke("s", 1, Woken::Turn);
        chains.prompt_submitted("s");
        let again = chains.woke("s", 4, Woken::Turn);
        chains.put_back("s", again);
        assert_eq!(chains.carried("s"), 1);
        chains.put_back("s", before);
        assert_eq!(chains.carried("s"), 0);
    }
}
