//! The real process side of a move ([`super::MoveSystem`]); moved out of
//! `moving.rs` for its size.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{KnownPlace, Member, MoveProgressInfo, MoveSystem, ProcId, Table, place_name, tree};
use crate::error::MoveHolder;
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

    // AJ-E2317: AJ-E2327's evidence-first link check.
    fn link_mismatch(
        &self,
        state: &Arc<AppState>,
        place_id: &str,
        _: &KnownPlace,
        _: &Path,
    ) -> Option<PathBuf> {
        super::pick::link_contradiction(state, place_id)
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

    // Only through the process's identity, checked right before
    // (`ownership::signal::quit_checked`).
    fn quit(&self, pid: u32) -> Result<(), String> {
        juggler_platform::apps::quit(pid).map_err(|error| error.to_string())
    }

    fn identify(&self, pids: &[u32]) -> HashMap<u32, ProcId> {
        crate::features::ownership::signal::identify(pids)
    }

    fn table(&self, state: &Arc<AppState>) -> Table {
        state.places.table()
    }

    // Started as a Desktop hand-off is, by the move that holds it closed
    // (every other start refuses a held profile).
    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        crate::features::places::start_held(state, place, &crate::features::places::start::System)
    }

    // AJ-E2322 R2: woken by the moved files' folders' events (a look
    // every second in case one is lost), capped.
    fn wait_open(
        &self,
        folders: &[PathBuf],
        cap: std::time::Duration,
        seen: &mut dyn FnMut() -> Option<String>,
    ) -> Option<String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut wake = juggler_adapters::folder_wake::FolderWake::new(move |_| {
            let _ = tx.send(());
        });
        wake.watch(&folders.iter().cloned().collect());
        super::timeline::wait_on(&rx, cap, seen)
    }

    // AJ-E2324: woken by the kernel when one ends; a second at most, so a
    // pid given to another program since is looked at again.
    fn wait_exit(&self, pids: &[u32], cap: std::time::Duration) {
        let _ = juggler_platform::wait::any_exit(pids, cap.min(std::time::Duration::from_secs(1)));
    }

    fn quiet_for(
        &self,
        paths: &[PathBuf],
        window: std::time::Duration,
        cap: std::time::Duration,
    ) -> bool {
        juggler_platform::wait::quiet_for(paths, window, cap) != juggler_platform::wait::Quiet::Busy
    }

    // AJ-E2324 R2: a system that can't tell says so in the log; the
    // move's other checks still hold.
    fn open_writers(&self, paths: &[PathBuf]) -> Vec<(u32, String)> {
        match juggler_platform::open_files::writers(paths) {
            Ok(found) => found.into_iter().map(|w| (w.pid, w.name)).collect(),
            Err(why) => {
                tracing::warn!(target: "juggler::move", "the open-file check couldn't run: {why}");
                Vec::new()
            }
        }
    }

    // 0.5.1 F93: never a sheet mid-move (both Apps closed, the move
    // held): `execute` and `undo` asked it before anything quit; declined,
    // the link isn't sent and the move offers Open.
    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        use juggler_platform::os_prompt::{self, Gated, Presence};
        match os_prompt::send_link(
            crate::os_prompts::registry::APP_LINKS,
            Presence::Background,
            pid,
            link,
        ) {
            Gated::Called { value, .. } => value.map_err(|error| error.to_string()),
            Gated::NotNow | Gated::Waiting => {
                Err("not sent: AI Juggler may not send links to apps yet".to_owned())
            }
        }
    }

    fn working(&self, state: &Arc<AppState>, pid: u32) -> usize {
        crate::features::places::working_in(state, pid)
    }

    // 0.5.1 F72: each by its title and where it's open, with its Go to.
    fn open_elsewhere(
        &self,
        state: &Arc<AppState>,
        accounts: &[String],
        hosts: &[u32],
    ) -> Vec<MoveHolder> {
        let open: Vec<_> = state
            .conversations
            .live_model()
            .entries
            .into_iter()
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
            .collect();
        if open.is_empty() {
            return Vec::new();
        }
        // Read only when something is open elsewhere (rare).
        let index = state.conversations.all().unwrap_or_default();
        open.into_iter()
            .map(|entry| {
                let title = entry.app_title.clone().or_else(|| {
                    index
                        .iter()
                        .find(|c| {
                            Some(&c.account_id) == entry.account_id.as_ref()
                                && c.summary.id == entry.session_id
                        })
                        .and_then(|c| c.summary.title.clone())
                });
                let host = entry
                    .places
                    .iter()
                    .find_map(|place| place.host.as_ref())
                    .map(|host| host.shown.clone());
                MoveHolder {
                    name: super::elsewhere_name(title.as_deref(), host.as_deref()),
                    place_id: None,
                    account_id: entry.account_id.clone(),
                    conversation: Some(entry.session_id.clone()),
                }
            })
            .collect()
    }
}
