//! AJ-E2403 (D-41b), the collateral suite extended to observed processes:
//! a leftover of another app's ended session is stopped only by the user's
//! own call, only when the live process is the sighted one and its
//! session's own process has ended, through the audit (`user:observed`,
//! the program's name in place of a command line); every other case sends
//! zero signals; and no automatic rule ever reaches it.
//!
//! Every process here is the test's own `sleep`, reaped by a guard.

#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::processes::stop;
use juggler_app::state::AppState;
use juggler_core::storage::{CleanupTierSettings, Sighting};

/// The test's own processes, ended and reaped however the test ends.
struct Fixtures(Vec<Child>);

impl Fixtures {
    fn sleep(&mut self) -> u32 {
        let child = Command::new("/bin/sleep").arg("300").spawn().unwrap();
        let pid = child.id();
        self.0.push(child);
        pid
    }

    fn alive(&mut self, pid: u32) -> bool {
        let child = self.0.iter_mut().find(|child| child.id() == pid).unwrap();
        child.try_wait().unwrap().is_none()
    }
}

impl Drop for Fixtures {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn start_of(pid: u32) -> i64 {
    stop::read_now(pid).unwrap().start_time
}

fn sighting(state: &Arc<AppState>, pid: u32, start: i64, root: bool) -> Sighting {
    Sighting {
        pid,
        start_time: start,
        boot_id: state.ledger.boot_id().to_owned(),
        exe_name: "sleep".into(),
        cmd_hash: "keyed".into(),
        tool: "tool-a".into(),
        session_id: "conv-ended".into(),
        app: Some("An App".into()),
        session_root: root,
        parent_pid: Some(1),
        first_seen: 1,
        last_seen: 2,
    }
}

fn record(state: &Arc<AppState>, sightings: &[Sighting]) {
    state
        .storage
        .lock()
        .unwrap()
        .sightings()
        .record(sightings)
        .unwrap();
}

fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// The one way it may happen: the sighted leftover of an ended session,
/// by the user's call, audited as `user:observed` with no command line.
#[test]
fn a_verified_leftover_of_an_ended_session_stops_and_is_audited() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut fixtures = Fixtures(Vec::new());
    // The session's own process ran and ended.
    let root = fixtures.sleep();
    let root_start = start_of(root);
    fixtures.0[0].kill().unwrap();
    fixtures.0[0].wait().unwrap();
    let helper = fixtures.sleep();
    let start = start_of(helper);
    record(
        &state,
        &[
            sighting(&state, root, root_start, true),
            sighting(&state, helper, start, false),
        ],
    );
    let sheet = stop::verify(&state, helper, Some(start));
    assert!(sheet.verified, "{:?}", sheet.refusal);
    let outcome =
        stop::execute(&state, state.audit.as_ref(), helper, Some(start), &flag()).unwrap();
    assert!(outcome.refusal.is_none());
    assert!(outcome.all_succeeded, "{:?}", outcome.members);
    // `sleep` exits on TERM; give the kernel a moment to say so.
    let mut gone = false;
    for _ in 0..50 {
        if !fixtures.alive(helper) {
            gone = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(gone, "the leftover stopped");
    let audit = state.audit.history(0).unwrap();
    let row = audit.iter().find(|row| row.pid == helper).unwrap();
    assert_eq!(row.actor, stop::ACTOR_OBSERVED);
    assert_eq!(
        row.cmdline_trunc, "sleep",
        "the program's name, never the line"
    );
    assert!(row.outcome.contains("exited on TERM"), "{}", row.outcome);
}

/// Every refusal sends zero signals: the process is still there after.
#[test]
fn every_refusal_sends_zero_signals() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut fixtures = Fixtures(Vec::new());
    let root = fixtures.sleep();
    let root_start = start_of(root);
    let helper = fixtures.sleep();
    let start = start_of(helper);
    // 1. Its session's own process still runs.
    record(
        &state,
        &[
            sighting(&state, root, root_start, true),
            sighting(&state, helper, start, false),
        ],
    );
    let outcome =
        stop::execute(&state, state.audit.as_ref(), helper, Some(start), &flag()).unwrap();
    assert_eq!(
        outcome.refusal.as_deref(),
        Some(stop::refusal::SESSION_RUNS)
    );
    assert!(fixtures.alive(helper));
    // 2. The session's own process: never, even once ended elsewhere.
    let outcome = stop::execute(
        &state,
        state.audit.as_ref(),
        root,
        Some(root_start),
        &flag(),
    )
    .unwrap();
    assert_eq!(
        outcome.refusal.as_deref(),
        Some(stop::refusal::SESSION_ITSELF)
    );
    assert!(fixtures.alive(root));
    // 3. A sighting of another process under the same pid (start time).
    let other = fixtures.sleep();
    let other_start = start_of(other);
    record(&state, &[sighting(&state, other, other_start - 100, false)]);
    let outcome = stop::execute(
        &state,
        state.audit.as_ref(),
        other,
        Some(other_start - 100),
        &flag(),
    )
    .unwrap();
    assert_eq!(outcome.refusal.as_deref(), Some(stop::refusal::OTHER));
    assert!(fixtures.alive(other));
    // 4. Never seen at all.
    let unseen = fixtures.sleep();
    let outcome = stop::execute(&state, state.audit.as_ref(), unseen, None, &flag()).unwrap();
    assert_eq!(outcome.refusal.as_deref(), Some(stop::refusal::UNSEEN));
    assert!(fixtures.alive(unseen));
    // 5. AI Juggler itself.
    let outcome = stop::execute(
        &state,
        state.audit.as_ref(),
        std::process::id(),
        None,
        &flag(),
    )
    .unwrap();
    assert_eq!(outcome.refusal.as_deref(), Some(stop::refusal::OWN));
    // Nothing was audited: no stop ran.
    assert!(state.audit.history(0).unwrap().is_empty());
}

/// No automatic rule reaches another app's process: with every account's
/// rules on and the sweep run past every grace, the sighted leftover lives.
#[test]
fn no_automatic_rule_ever_stops_another_apps_leftover() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut fixtures = Fixtures(Vec::new());
    let root = fixtures.sleep();
    let root_start = start_of(root);
    fixtures.0[0].kill().unwrap();
    fixtures.0[0].wait().unwrap();
    let helper = fixtures.sleep();
    let start = start_of(helper);
    record(
        &state,
        &[
            sighting(&state, root, root_start, true),
            sighting(&state, helper, start, false),
        ],
    );
    for account in ["acct-a", "acct-b", "tool-a"] {
        state
            .cleanup_settings
            .set(
                account,
                &CleanupTierSettings {
                    tier1_opt_in: true,
                    tier2_opt_in: true,
                },
            )
            .unwrap();
    }
    let later = juggler_core::model::Timestamp::now().millis() + 24 * 3_600_000;
    for step in 0..3 {
        juggler_app::commands::auto_cleanup_sweep(&state, later + step * 600_000);
    }
    let _ = juggler_app::features::processes::tick(&state);
    assert!(
        fixtures.alive(helper),
        "a rule never stops another app's process"
    );
    assert!(state.audit.history(0).unwrap().is_empty());
}
