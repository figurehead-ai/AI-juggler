//! AJ-E1509 — AI Juggler itself on This Mac: the app's own memory and its
//! crash watchdog's (a process of AI Juggler, never a stray one), and
//! whether it opens at login.

use juggler_login_item::LoginItem;

use crate::state::AppState;

/// This Mac's "AI Juggler itself" line.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OwnInfo {
    /// Memory of the app and its watchdog, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// The crash watchdog runs (AJ-E1509 Req.4).
    pub watchdog: bool,
    /// AI Juggler opens at login.
    pub opens_at_login: bool,
}

/// AI Juggler's own processes now: this one and its watchdog.
#[must_use]
pub fn measure(state: &AppState) -> OwnInfo {
    let watchdog = state
        .watchdog
        .lock()
        .ok()
        .and_then(|child| child.as_ref().map(std::process::Child::id));
    let pids: Vec<sysinfo::Pid> = std::iter::once(std::process::id())
        .chain(watchdog)
        .map(sysinfo::Pid::from_u32)
        .collect();
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&pids), true);
    OwnInfo {
        rss_bytes: pids
            .iter()
            .filter_map(|pid| system.process(*pid))
            .map(sysinfo::Process::memory)
            .sum(),
        watchdog: watchdog.is_some(),
        opens_at_login: super::login_item::status() == LoginItem::On,
    }
}
