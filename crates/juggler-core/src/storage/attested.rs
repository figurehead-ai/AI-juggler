//! AJ-E3013 (D-116, ADR-0020's D-116 addendum): the desktop sessions the
//! agent bridge attested, kept in memory so every pool decision sees them
//! as it sees a session AI Juggler started.
//!
//! - A desktop app's session (the Claude app's Code tab) has no row in
//!   `instances`: AI Juggler didn't start it. When its bridge's call is
//!   fully attested (its process chain, the tool's session registry and
//!   the app's own record; `juggler-app`'s `features/agent_bridge/attest`),
//!   its facts are kept here under a minted id, [`DESKTOP_PREFIX`] and its
//!   conversation, which no real session can have
//!   ([`super::SessionsRepo::insert`] refuses it).
//! - [`super::SessionsRepo::pool_facts_of`] reads them for such an id, so
//!   `allowed`, the resolver, `refs_of` and `role_in` treat the session as
//!   they treat any other. `pool_facts()` (every session) doesn't list them.
//! - An entry lives [`TTL`] after the last attested call that put it; a
//!   read never extends it. The attestation path drops one at once when a
//!   later call finds its session's record stale ([`AttestedSessions::forget`]).
//! - Only the attestation path writes here: a guard test in `juggler-app`
//!   holds every `attested().put(` to that one file.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::model::SessionId;
use crate::pools::SessionFacts;

/// The prefix of every attested desktop session's id.
pub const DESKTOP_PREFIX: &str = "desktop:";

/// How long an entry lives after the last attested call.
pub const TTL: Duration = Duration::from_mins(10);

/// Whether `id` is an attested desktop session's (minted) id.
#[must_use]
pub fn is_desktop_id(id: &str) -> bool {
    id.starts_with(DESKTOP_PREFIX)
}

/// The id an attested desktop session gets for `conversation`.
#[must_use]
pub fn desktop_id(conversation: &str) -> SessionId {
    SessionId::new(format!("{DESKTOP_PREFIX}{conversation}"))
}

/// The attested desktop sessions, in memory only.
#[derive(Debug, Default)]
pub struct AttestedSessions {
    entries: Mutex<HashMap<SessionId, (SessionFacts, Instant)>>,
}

impl AttestedSessions {
    /// Keeps `facts` (their session id must be a desktop one) as seen
    /// now. Only the attestation path calls this, after a full
    /// attestation on the same call.
    pub fn put(&self, facts: SessionFacts) {
        self.put_at(facts, Instant::now());
    }

    fn put_at(&self, facts: SessionFacts, at: Instant) {
        if !is_desktop_id(facts.session.as_str()) {
            return;
        }
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.retain(|_, (_, seen)| at.saturating_duration_since(*seen) < TTL);
        entries.insert(facts.session.clone(), (facts, at));
    }

    /// The facts kept for `id`, while they're fresh. Never extends them.
    #[must_use]
    pub fn get(&self, id: &SessionId) -> Option<SessionFacts> {
        self.get_at(id, Instant::now())
    }

    fn get_at(&self, id: &SessionId, now: Instant) -> Option<SessionFacts> {
        if !is_desktop_id(id.as_str()) {
            return None;
        }
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match entries.get(id) {
            Some((facts, seen)) if now.saturating_duration_since(*seen) < TTL => {
                Some(facts.clone())
            }
            Some(_) => {
                entries.remove(id);
                None
            }
            None => None,
        }
    }

    /// When `id` was last attested, while it's fresh.
    #[must_use]
    pub fn seen(&self, id: &SessionId) -> Option<Instant> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries
            .get(id)
            .map(|(_, seen)| *seen)
            .filter(|seen| seen.elapsed() < TTL)
    }

    /// Forgets `id` (its session's record went stale).
    pub fn forget(&self, id: &SessionId) {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(id: &str) -> SessionFacts {
        SessionFacts::bare(SessionId::new(id), None)
    }

    #[test]
    fn only_desktop_ids_are_kept_and_reads_never_extend_them() {
        let kept = AttestedSessions::default();
        let start = Instant::now();
        kept.put_at(facts("sess-1"), start);
        assert_eq!(kept.get_at(&SessionId::new("sess-1"), start), None);
        let id = desktop_id("c-1");
        kept.put_at(facts(id.as_str()), start);
        let half = start + TTL / 2;
        assert!(kept.get_at(&id, half).is_some());
        // The read at half-time didn't extend it.
        assert_eq!(kept.get_at(&id, start + TTL), None);
        assert_eq!(
            kept.get_at(&id, start + TTL / 4),
            None,
            "dropped, not hidden"
        );
    }

    #[test]
    fn a_new_attestation_refreshes_and_forget_drops() {
        let kept = AttestedSessions::default();
        let start = Instant::now();
        let id = desktop_id("c-2");
        kept.put_at(facts(id.as_str()), start);
        kept.put_at(facts(id.as_str()), start + TTL / 2);
        assert!(kept.get_at(&id, start + TTL).is_some());
        kept.forget(&id);
        assert_eq!(kept.get_at(&id, start + TTL / 2), None);
    }
}
