//! 0.4.4: a limit hit in a conversation moved to another Account (ADR-0031,
//! D-93) is credited to the Account it happened on, by the move journal.
//! A move renames the conversation's history into the target's folder,
//! whole; read there, a hit from before the move made the target read as
//! out of room the moment the user moved to it to carry on. And (D-103.10)
//! the hits an earlier version stored on the wrong Account go back, once,
//! at start. Every test runs on a throwaway data folder.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step is the failure it reports"
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapter::moving::{MovePlan, MoveRequest, MoveSide, MoveStep, NotesChoice};
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::handover::moved_lines::Moves;
use juggler_app::features::quota::past_hits::{self, Outcome, RECORD_KEY};
use juggler_app::state::AppState;
use juggler_app::usage_pipeline;
use juggler_core::quota::UsageSource;
use juggler_handover::moving::{FORMAT, Manifest, Phase, VERSION};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

fn now() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

/// RFC 3339 for `ms` (UTC), as a transcript line carries it.
fn iso(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(mo <= 2);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// The app on the data folder `data`, as it starts.
fn start(data: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin("/bin/true").with_overlays_root(data.join("overlays")),
    ));
    Arc::new(AppState::init_with_registry(data.to_path_buf(), registry).unwrap())
}

/// A Claude profile used where it is, signed in as `subject`.
fn profile(state: &Arc<AppState>, root: &Path, name: &str, subject: &str) -> (String, PathBuf) {
    let dir = root.join("ClaudeCodeProfiles").join(name);
    std::fs::create_dir_all(dir.join("projects/-w-p")).unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        format!(
            r#"{{"oauthAccount":{{"accountUuid":"{subject}","organizationUuid":"org-{subject}","emailAddress":"planted@example.invalid"}}}}"#
        ),
    )
    .unwrap();
    let id = juggler_app::features::account::import::account_import_mode_impl(
        state,
        CLAUDE_CODE_ID,
        dir.to_str().unwrap(),
        name,
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    (id, dir)
}

/// A move's record, as the journal keeps it: `history` renamed from
/// `from` to `to` at `at_ms`, at `phase`.
fn journal(
    state: &Arc<AppState>,
    id: &str,
    sides: [(&str, &Path); 2],
    step: (&Path, &Path),
    at_ms: i64,
    phase: Phase,
) {
    let side = |dir: &Path| MoveSide {
        tool_dir: dir.to_path_buf(),
        host_dir: None,
        version: None,
    };
    let manifest = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        id: id.into(),
        tool: CLAUDE_CODE_ID.into(),
        created_ms: at_ms,
        request: MoveRequest {
            conversation: "s-1".into(),
            source: side(sides[0].1),
            target: side(sides[1].1),
            notes: NotesChoice::KeepTarget,
        },
        plan: MovePlan {
            conversation: "s-1".into(),
            resume: "s-1".into(),
            steps: vec![MoveStep::Rename {
                from: step.0.to_path_buf(),
                to: step.1.to_path_buf(),
                history: true,
            }],
            ..MovePlan::default()
        },
        phase,
        entries: Vec::new(),
        previous: None,
        problem: None,
        places: Vec::new(),
        accounts: vec![sides[0].0.to_owned(), sides[1].0.to_owned()],
        preserved: None,
    };
    let dir = state
        .data_dir
        .join(juggler_handover::moving::FOLDER)
        .join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

fn hit(at_ms: i64, resets_ms: i64) -> String {
    format!(
        r#"{{"type":"assistant","sessionId":"s-1","timestamp":"{}","message":{{"content":[{{"type":"text","text":"PLANTED"}}]}},"error":"rate_limit","isApiErrorMessage":true,"quotaLimits":{{"status":"rejected","rateLimitType":"five_hour","resetsAt":{}}}}}"#,
        iso(at_ms),
        resets_ms / 1000
    )
}

fn five_hour(state: &Arc<AppState>, id: &str) -> Option<(String, Option<i64>)> {
    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(id))
        .unwrap();
    juggler_app::features::quota::quota_display_inner(&storage, &account, now())
        .five_hour
        .map(|window| (window.state, window.resets_at_ms))
}

#[test]
fn a_hit_before_a_move_is_the_accounts_it_moved_from() {
    let dir = tempfile::tempdir().unwrap();
    let state = start(&dir.path().join("app-data"));
    let (work, work_dir) = profile(&state, dir.path(), "work", "acct-w");
    let (side, side_dir) = profile(&state, dir.path(), "side", "acct-s");
    let from = work_dir.join("projects/-w-p/s-1.jsonl");
    let to = side_dir.join("projects/-w-p/s-1.jsonl");
    // Work hit its five-hour limit 30 minutes ago; the conversation moved
    // to Side 10 minutes ago, and Side's own hit came after.
    let moved = now() - 10 * MIN;
    let work_reset = (now() + 2 * HOUR) / 1000 * 1000;
    journal(
        &state,
        "m-1",
        [(&work, &work_dir), (&side, &side_dir)],
        (&from, &to),
        moved,
        Phase::Committed,
    );
    std::fs::write(&to, hit(now() - 30 * MIN, work_reset) + "\n").unwrap();
    usage_pipeline::ingest_pass(&state, 1);
    assert_eq!(
        five_hour(&state, &work),
        Some(("exhausted".to_owned(), Some(work_reset))),
        "the hit is Work's"
    );
    assert_ne!(
        five_hour(&state, &side).map(|(state, _)| state).as_deref(),
        Some("exhausted"),
        "Side has room: the hit wasn't its"
    );
    // A hit after the move is Side's own.
    let side_reset = (now() + 3 * HOUR) / 1000 * 1000;
    let mut lines = std::fs::read_to_string(&to).unwrap();
    lines.push_str(&(hit(now() - MIN, side_reset) + "\n"));
    std::fs::write(&to, lines).unwrap();
    usage_pipeline::ingest_pass(&state, 2);
    assert_eq!(
        five_hour(&state, &side),
        Some(("exhausted".to_owned(), Some(side_reset)))
    );
}

#[test]
fn an_undone_or_unapplied_move_leaves_a_hit_where_it_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = start(&dir.path().join("app-data"));
    let (work, work_dir) = profile(&state, dir.path(), "work", "acct-w");
    let (side, side_dir) = profile(&state, dir.path(), "side", "acct-s");
    let from = work_dir.join("projects/-w-p/s-1.jsonl");
    let to = side_dir.join("projects/-w-p/s-1.jsonl");
    let reset = (now() + 2 * HOUR) / 1000 * 1000;
    journal(
        &state,
        "m-1",
        [(&work, &work_dir), (&side, &side_dir)],
        (&from, &to),
        now() - 10 * MIN,
        Phase::Undone,
    );
    journal(
        &state,
        "m-2",
        [(&work, &work_dir), (&side, &side_dir)],
        (&from, &to),
        now() - 5 * MIN,
        Phase::BackedUp,
    );
    std::fs::write(&to, hit(now() - 30 * MIN, reset) + "\n").unwrap();
    usage_pipeline::ingest_pass(&state, 1);
    assert_eq!(
        five_hour(&state, &side),
        Some(("exhausted".to_owned(), Some(reset)))
    );
    assert_ne!(
        five_hour(&state, &work).map(|(state, _)| state).as_deref(),
        Some("exhausted")
    );
}

/// As a version before 0.4.4 stored them: every hit in `file` on
/// `account`, the one whose folder it was read in.
fn stored_where_read(state: &Arc<AppState>, account: &str, file: &Path) {
    let hits: Vec<_> = juggler_adapters::claude_limits::limit_hits_in(file)
        .into_iter()
        .map(|(kind, hit)| (account.to_owned(), kind, hit, UsageSource::ApiRetry))
        .collect();
    let storage = state.storage.lock().unwrap();
    storage.usage().insert_sourced_samples(&hits).unwrap();
}

/// The Accounts holding each hit in `file`, in its order.
fn holders(state: &Arc<AppState>, file: &Path) -> Vec<Vec<String>> {
    let storage = state.storage.lock().unwrap();
    juggler_adapters::claude_limits::limit_hits_in(file)
        .iter()
        .map(|(kind, hit)| storage.usage().limit_hit_holders(*kind, hit).unwrap())
        .collect()
}

fn give_back(state: &Arc<AppState>) -> Outcome {
    let storage = state.storage.lock().unwrap();
    past_hits::give_back(&storage, &Moves::read(state), &state.registry).unwrap()
}

/// A stored reading, whole: its Account, window and instant, the span it
/// held and its source.
type Reading = (String, String, i64, Option<i64>, Option<String>);

/// Every stored reading.
fn readings(state: &Arc<AppState>) -> Vec<Reading> {
    let storage = state.storage.lock().unwrap();
    let mut stmt = storage
        .raw_conn()
        .prepare(
            "SELECT q.account, q.window, q.ts, q.last_seen, s.source FROM quota_samples q
             LEFT JOIN quota_sample_sources s
               ON s.account = q.account AND s.window = q.window AND s.ts = q.ts
             ORDER BY 1, 2, 3",
        )
        .unwrap();
    stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

#[test]
fn past_hits_go_back_to_the_account_they_happened_on_and_again_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = start(&dir.path().join("app-data"));
    let (work, work_dir) = profile(&state, dir.path(), "work", "acct-w");
    let (side, side_dir) = profile(&state, dir.path(), "side", "acct-s");
    let from = work_dir.join("projects/-w-p/s-1.jsonl");
    let to = side_dir.join("projects/-w-p/s-1.jsonl");
    journal(
        &state,
        "m-1",
        [(&work, &work_dir), (&side, &side_dir)],
        (&from, &to),
        now() - 10 * MIN,
        Phase::Committed,
    );
    // The moved conversation: Work's hit from before the move, and Side's
    // own after it. Another of Side's, never moved, with its own hit.
    let work_reset = (now() + 2 * HOUR) / 1000 * 1000;
    let side_reset = (now() + 3 * HOUR) / 1000 * 1000;
    std::fs::write(
        &to,
        [
            hit(now() - 30 * MIN, work_reset),
            hit(now() - 5 * MIN, side_reset),
        ]
        .join("\n")
            + "\n",
    )
    .unwrap();
    let other = side_dir.join("projects/-w-p/s-2.jsonl");
    std::fs::write(&other, hit(now() - 40 * MIN, side_reset) + "\n").unwrap();
    // As 0.4.3 stored them: all three Side's.
    stored_where_read(&state, &side, &to);
    stored_where_read(&state, &side, &other);
    assert_eq!(holders(&state, &to), [[side.as_str()], [side.as_str()]]);

    let outcome = give_back(&state);
    assert_eq!(
        outcome,
        Outcome {
            wrong_before: 1,
            moved: 1,
            left: 1,
            ..Outcome::default()
        }
    );
    assert_eq!(
        holders(&state, &to),
        [[work.as_str()], [side.as_str()]],
        "Work's hit is Work's"
    );
    assert_eq!(
        holders(&state, &other),
        [[side.as_str()]],
        "not moved: it stays"
    );
    assert_eq!(
        five_hour(&state, &work),
        Some(("exhausted".to_owned(), Some(work_reset)))
    );

    // Again: it finds nothing held wrongly and changes nothing.
    let after = readings(&state);
    assert_eq!(
        give_back(&state),
        Outcome {
            left: 2,
            ..Outcome::default()
        }
    );
    assert_eq!(readings(&state), after);
    // At start it runs once: this data's first start ran it and kept it.
    let storage = state.storage.lock().unwrap();
    assert!(storage.app_settings().get(RECORD_KEY).unwrap().is_some());
    assert_eq!(
        past_hits::once_at_start(&storage, &state.data_dir, &state.registry),
        None
    );
}

#[test]
fn at_start_a_hit_whose_account_is_gone_is_no_ones_and_the_counts_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    let state = start(&data);
    let (side, side_dir) = profile(&state, dir.path(), "side", "acct-s");
    // The journal names an Account that is no longer there.
    let gone_dir = dir.path().join("ClaudeCodeProfiles/gone");
    let from = gone_dir.join("projects/-w-p/s-1.jsonl");
    let to = side_dir.join("projects/-w-p/s-1.jsonl");
    journal(
        &state,
        "m-1",
        [("acct-gone", &gone_dir), (&side, &side_dir)],
        (&from, &to),
        now() - 10 * MIN,
        Phase::Committed,
    );
    let reset = (now() + 2 * HOUR) / 1000 * 1000;
    std::fs::write(&to, hit(now() - 30 * MIN, reset) + "\n").unwrap();
    stored_where_read(&state, &side, &to);
    assert_eq!(
        five_hour(&state, &side),
        Some(("exhausted".to_owned(), Some(reset)))
    );
    // A 0.4.3 database, opened by 0.4.4: nothing kept yet.
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(RECORD_KEY, "")
        .unwrap();
    drop(state);

    let state = start(&data);
    assert_eq!(holders(&state, &to), [Vec::<String>::new()], "never Side's");
    assert_ne!(
        five_hour(&state, &side).map(|(state, _)| state).as_deref(),
        Some("exhausted"),
        "Side has room"
    );
    let kept = state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .get(RECORD_KEY)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Outcome>(&kept).unwrap(),
        Outcome {
            wrong_before: 1,
            cleared: 1,
            ..Outcome::default()
        }
    );
}
