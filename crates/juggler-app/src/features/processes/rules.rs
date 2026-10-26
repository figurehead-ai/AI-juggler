//! AJ-E2405: the automatic rules as the page shows them. The tiers stay as
//! they are (ADR-0010, D-08: per account, off by default, only verified
//! dev servers and watchers, never a root, an MCP server, a shell or a
//! database, never another app's process). Here:
//! - a dry run before turning one on: what it would tidy now, what later,
//!   and what never, with the same rule the sweep uses
//!   ([`juggler_supervisor::tree::evaluate_tier`]) and the same exclusions
//!   (heavy, Keep);
//! - the policy across accounts, with pause and the crash hold.

use std::sync::Arc;

use juggler_core::model::SessionId;

use super::dto::{AccountPolicyInfo, CleanupPolicyInfo, RulePreviewInfo, RulePreviewItemInfo};
use super::model::{Origin, ProcState};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One candidate as the dry run decides on it (pure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Its pid.
    pub pid: u32,
    /// Its name with its ports (`vite :5173`).
    pub name: String,
    /// Its session.
    pub session_id: String,
    /// Its role in the cleanup's words (`DevServer`).
    pub role: &'static str,
    /// Its ledger state (`orphaned`).
    pub state: String,
    /// It left its process group.
    pub escaped_group: bool,
    /// Its session's own process ended cleanly (`Some(false)`: a crash).
    pub root_exit_clean: Option<bool>,
    /// When its session's own process ended, epoch ms.
    pub ended_ms: i64,
    /// Heavy (the hog rules never let a rule touch it).
    pub heavy: bool,
    /// The user said Keep.
    pub kept: bool,
}

/// What `tier` would do with each candidate at `now_ms` (pure).
#[must_use]
pub fn preview(candidates: &[Candidate], tier: u8, now_ms: i64) -> Vec<RulePreviewItemInfo> {
    candidates
        .iter()
        .map(|candidate| {
            let verdict =
                juggler_supervisor::tree::evaluate_tier(&juggler_supervisor::tree::TierCandidate {
                    pid: candidate.pid,
                    role_guess: candidate.role.to_owned(),
                    is_root: false,
                    state: candidate.state.clone(),
                    escaped_group: candidate.escaped_group,
                    root_exit_clean: candidate.root_exit_clean,
                    machine_local: true,
                    tier1_opt_in: true,
                    tier2_opt_in: tier >= 2,
                });
            let (when, reason) = if candidate.kept {
                ("never", Some("You chose Keep".to_owned()))
            } else if candidate.heavy {
                (
                    "never",
                    Some("It is heavy: you decide, never a rule".to_owned()),
                )
            } else {
                match verdict {
                    juggler_supervisor::tree::TierVerdict::ObserveOnly(why) => {
                        ("never", Some(plain(&why)))
                    }
                    juggler_supervisor::tree::TierVerdict::Signal(level) => {
                        let wait = if level >= 2 {
                            crate::commands::TIER2_CRASH_GRACE_MS
                        } else {
                            juggler_supervisor::AUTO_CLEANUP_GRACE_MS
                        };
                        let left = candidate.ended_ms + wait - now_ms;
                        if left > 0 {
                            ("later", Some(format!("In {} s", (left + 999) / 1000)))
                        } else {
                            ("now", None)
                        }
                    }
                }
            };
            RulePreviewItemInfo {
                pid: candidate.pid,
                name: candidate.name.clone(),
                session_id: candidate.session_id.clone(),
                when: when.to_owned(),
                reason,
            }
        })
        .collect()
}

/// The evaluator's reason in plain words.
fn plain(why: &str) -> String {
    if why.contains("role guess") {
        "Not a dev server or a watcher".to_owned()
    } else if why.contains("escaped") {
        "It left its session's group".to_owned()
    } else if why.contains("exit kind unknown") {
        "How its session ended isn't known".to_owned()
    } else if why.contains("after crash") {
        "Its session crashed (the second rule)".to_owned()
    } else if why.contains("state") {
        "Not left behind".to_owned()
    } else {
        "Not something a rule tidies".to_owned()
    }
}

/// The sentence for a preview: "Would tidy vite :5173, esbuild now".
#[must_use]
pub fn words(items: &[RulePreviewItemInfo]) -> String {
    let now: Vec<&str> = items
        .iter()
        .filter(|item| item.when == "now")
        .map(|item| item.name.as_str())
        .collect();
    let later = items.iter().filter(|item| item.when == "later").count();
    match (now.is_empty(), later) {
        (true, 0) => "Nothing to tidy now".to_owned(),
        (true, later) => format!("Nothing now; {later} after their wait"),
        (false, 0) => format!("Would tidy {} now", now.join(", ")),
        (false, later) => format!(
            "Would tidy {} now, {later} more after their wait",
            now.join(", ")
        ),
    }
}

/// The dry run for `account_id`'s `tier` (1 or 2): nothing is signalled.
///
/// # Errors
///
/// [`AppError::Validation`] for a tier other than 1 or 2.
pub fn dry_run(state: &Arc<AppState>, account_id: &str, tier: u8) -> AppResult<RulePreviewInfo> {
    use juggler_supervisor::tree::LedgerStore as _;
    if !(1..=2).contains(&tier) {
        return Err(AppError::Validation {
            subject: "tier",
            message: "a rule is tier 1 or 2".into(),
        });
    }
    let graph = state.processes.graph();
    let mut candidates = Vec::new();
    for app in graph.apps.iter().filter(|app| app.ours) {
        for session in app
            .sessions
            .iter()
            .filter(|session| session.account_id.as_deref() == Some(account_id))
        {
            let rows = state
                .ledger
                .session_rows(&SessionId::new(session.session_id.clone()))
                .unwrap_or_default();
            let root = rows.iter().find(|row| row.role == "root");
            let root_exit_clean = root.and_then(|row| {
                row.exit_kind
                    .as_deref()
                    .map(|kind| kind.strip_prefix("code:").is_some_and(|code| code == "0"))
            });
            for node in session.processes.iter().filter(|node| {
                node.origin == Origin::Ours
                    && matches!(
                        node.state,
                        ProcState::LeftBehind | ProcState::LeftBehindClosed
                    )
            }) {
                let row = rows.iter().find(|row| row.pid == Some(node.pid));
                let ports: String = node.ports.iter().fold(String::new(), |mut out, port| {
                    use std::fmt::Write as _;
                    let _ = write!(out, " :{port}");
                    out
                });
                candidates.push(Candidate {
                    pid: node.pid,
                    name: format!("{}{ports}", node.name),
                    session_id: session.session_id.clone(),
                    role: row.map_or("Unknown", |row| {
                        crate::commands::guess_role(&row.exe_path, &row.cmdline_trunc)
                    }),
                    state: row.map_or_else(|| "orphaned".to_owned(), |row| row.state.clone()),
                    escaped_group: row.is_some_and(|row| row.escaped_group),
                    root_exit_clean,
                    ended_ms: root.map_or(0, |row| row.last_seen),
                    heavy: node.heavy,
                    kept: state.processes.is_kept(node.pid, node.start_time),
                });
            }
        }
    }
    let items = preview(&candidates, tier, crate::commands::now_millis());
    Ok(RulePreviewInfo {
        account_id: account_id.to_owned(),
        tier,
        words: words(&items),
        items,
    })
}

/// The automatic cleanup across the user's accounts.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the store's lock is poisoned.
pub fn policy(state: &Arc<AppState>) -> AppResult<CleanupPolicyInfo> {
    let accounts = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.accounts().list().unwrap_or_default()
    };
    let accounts = accounts
        .into_iter()
        .map(|account| {
            let settings = state
                .cleanup_settings
                .get(account.id.as_str())
                .unwrap_or_default();
            AccountPolicyInfo {
                account_id: account.id.as_str().to_owned(),
                label: account.label.clone(),
                tier1: settings.tier1_opt_in,
                tier2: settings.tier2_opt_in,
            }
        })
        .collect();
    Ok(CleanupPolicyInfo {
        accounts,
        paused: crate::commands::cleanup_paused(state),
        suspended_until_ms: crate::commands::cleanup_suspended_until(state),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(pid: u32, role: &'static str) -> Candidate {
        Candidate {
            pid,
            name: format!("p{pid}"),
            session_id: "s".into(),
            role,
            state: "orphaned".into(),
            escaped_group: false,
            root_exit_clean: Some(true),
            ended_ms: 0,
            heavy: false,
            kept: false,
        }
    }

    #[test]
    fn the_dry_run_uses_the_sweeps_own_rule() {
        let mut list = vec![
            candidate(1, "DevServer"),
            candidate(2, "Watcher"),
            candidate(3, "McpServer"),
            candidate(4, "Database"),
            candidate(5, "DevServer"),
            candidate(6, "DevServer"),
        ];
        list[4].kept = true;
        list[5].heavy = true;
        let items = preview(&list, 1, 120_000);
        let when: Vec<&str> = items.iter().map(|item| item.when.as_str()).collect();
        assert_eq!(when, ["now", "now", "never", "never", "never", "never"]);
        assert_eq!(words(&items), "Would tidy p1, p2 now");
        // Within the grace: later, never now.
        let early = preview(&list[..1], 1, 30_000);
        assert_eq!(early[0].when, "later");
        assert_eq!(words(&early), "Nothing now; 1 after their wait");
    }

    #[test]
    fn a_crash_needs_the_second_rule() {
        let mut crashed = candidate(7, "DevServer");
        crashed.root_exit_clean = Some(false);
        assert_eq!(
            preview(std::slice::from_ref(&crashed), 1, 10_000_000)[0].when,
            "never"
        );
        assert_eq!(preview(&[crashed], 2, 10_000_000)[0].when, "now");
    }
}
