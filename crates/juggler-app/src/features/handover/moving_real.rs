//! The real process side of a move ([`super::MoveSystem`]); moved out of
//! `moving.rs` for its size.

use std::sync::Arc;

use super::{KnownPlace, Member, MoveProgressInfo, MoveSystem, place_name, tree};
use crate::state::AppState;

/// Hands each step of a move's progress on (to the switch screen).
pub type Tell = Box<dyn Fn(&MoveProgressInfo) + Send + Sync>;

/// The real system.
pub struct Real {
    tell: Option<Tell>,
}

impl Real {
    /// The real system, telling no one how a move goes (the offer, Undo).
    #[must_use]
    pub fn quiet() -> Self {
        Self { tell: None }
    }

    /// The real system, telling `tell` how the move goes.
    #[must_use]
    pub fn telling(tell: Tell) -> Self {
        Self { tell: Some(tell) }
    }
}

impl MoveSystem for Real {
    fn tree(&self, state: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        tree::now(state, place, roots)
    }

    fn elsewhere(&self, state: &Arc<AppState>, place: &KnownPlace, left: &[Member]) -> Vec<Member> {
        tree::elsewhere_now(state, place, left)
    }

    fn quits_juggler(
        &self,
        state: &Arc<AppState>,
        place: &KnownPlace,
        pid: u32,
        action: &str,
    ) -> Option<String> {
        tree::quits_juggler_now(state, place, pid, &place_name(state, place), action)
    }

    fn tell(&self, progress: &MoveProgressInfo) {
        if let Some(tell) = &self.tell {
            tell(progress);
        }
    }

    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<(u32, Option<i64>)> {
        crate::features::places::running_now(state, place_id)
    }

    fn quit(&self, pid: u32) -> Result<(), String> {
        use crate::features::places::start::Starter as _;
        crate::features::places::start::System.quit(pid)
    }

    fn alive(&self, pid: u32, start: Option<i64>) -> bool {
        use crate::features::places::start::Starter as _;
        crate::features::places::start::System.alive(pid, start)
    }

    // Started and handed links exactly as a Desktop hand-off is.
    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        use crate::features::places::handoff::System as _;
        crate::features::places::handoff::Real.start(state, place)
    }

    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        use crate::features::places::handoff::System as _;
        crate::features::places::handoff::Real.send(pid, link)
    }

    fn working(&self, state: &Arc<AppState>, pid: u32) -> usize {
        crate::features::places::working_in(state, pid)
    }

    fn open_elsewhere(&self, state: &Arc<AppState>, accounts: &[String], hosts: &[u32]) -> usize {
        state
            .conversations
            .live_model()
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .account_id
                    .as_ref()
                    .is_some_and(|a| accounts.contains(a))
            })
            .filter(|entry| {
                !entry.places.iter().any(|place| {
                    place
                        .host
                        .as_ref()
                        .is_some_and(|host| hosts.contains(&host.pid))
                })
            })
            .count()
    }
}
