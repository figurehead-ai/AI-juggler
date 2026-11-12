//! 0.4.4: a limit hit in a conversation moved to another Account (ADR-0031,
//! D-93) is credited to the Account it happened on, by the move journal.
//! A move renames the conversation's history into the target's folder,
//! whole; read there, a hit from before the move made the target read as
//! out of room the moment the user moved to it to carry on.

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
use juggler_app::state::AppState;
use juggler_app::usage_pipeline;
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
    let data = dir.path().join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin("/bin/true").with_overlays_root(data.join("overlays")),
    ));
    let state = Arc::new(AppState::init_with_registry(data, registry).unwrap());
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
    let data = dir.path().join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin("/bin/true").with_overlays_root(data.join("overlays")),
    ));
    let state = Arc::new(AppState::init_with_registry(data, registry).unwrap());
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
