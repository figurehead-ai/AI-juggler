//! P19 (QA walk batch 21): a same-tool switch copies the conversation into
//! the target account with its original lines, ids and times, and the
//! target's usage counted that history again (×1.41–1.48 on the lab's
//! conversations). The carried lines belong to the account they were
//! spent on: in the target's copy, every line older than the switch is
//! left out, and rows an earlier version stored for them are cleared by
//! the one-time re-read.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_app::state::AppState;
use juggler_app::usage_pipeline;
use juggler_core::model::{
    Account, AccountId, Confidence, EdgeActor, EdgeState, Epoch, EpochId, FailureStage, HarnessId,
    SwitchEdge, SwitchEdgeId, SwitchMechanism, Timestamp, TokenCounts,
};
use juggler_core::quota::{UsageEvent, UsageSource};

/// 2026-10-22 10:00 UTC, and the switch five minutes later.
const T0: i64 = 1_792_663_200_000;
const SWITCH_MIN: i64 = 5;

/// A usage line at `minute` past 10:00 on 2026-10-22.
fn line(id: &str, session: &str, minute: i64, output: u64) -> String {
    let ts = format!("2026-10-22T10:{minute:02}:00.000Z");
    format!(
        "{{\"type\":\"assistant\",\"uuid\":\"u-{id}\",\"sessionId\":\"{session}\",\"requestId\":\"r-{id}\",\
         \"timestamp\":\"{ts}\",\"message\":{{\"id\":\"m-{id}\",\"role\":\"assistant\",\"model\":\"claude-opus-4\",\
         \"usage\":{{\"input_tokens\":10,\"output_tokens\":{output}}}}}}}\n"
    )
}

/// `ms` as the transcripts write it (UTC, milliseconds).
fn iso(ms: i64) -> String {
    let secs = ms / 1000;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// A usage line at `ms`, for switches staged just now.
fn line_at(id: &str, session: &str, ms: i64, output: u64) -> String {
    format!(
        "{{\"type\":\"assistant\",\"uuid\":\"u-{id}\",\"sessionId\":\"{session}\",\"requestId\":\"r-{id}\",\
         \"timestamp\":\"{}\",\"message\":{{\"id\":\"m-{id}\",\"role\":\"assistant\",\"model\":\"claude-opus-4\",\
         \"usage\":{{\"input_tokens\":10,\"output_tokens\":{output}}}}}}}\n",
        iso(ms)
    )
}

fn account(state: &Arc<AppState>, id: &str, dir: &Path) {
    std::fs::create_dir_all(dir.join("projects/lab")).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: id.into(),
            plan: None,
            state_dir: dir.to_path_buf(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: true,
        })
        .unwrap();
}

/// How far a switch got.
#[derive(Clone, Copy)]
enum Outcome<'a> {
    /// Launched, its copy installed under this id (the source's, or a new
    /// one beside a copy the target already had).
    Launched(&'a str),
    /// Staged only: the launch isn't recorded yet.
    Staged,
    /// Failed: nothing was carried.
    Failed,
}

/// A transplant of conversation `native` from `from` to `to` at `at`.
fn switched(state: &Arc<AppState>, native: &str, from: &str, to: &str, at: i64, outcome: Outcome) {
    let storage = state.storage.lock().unwrap();
    let project = storage
        .projects()
        .touch(Path::new("/work/lab"), None)
        .unwrap()
        .id;
    let graph = storage.switch_graph();
    let work = graph.create_work(&project, native).unwrap();
    let epoch = |id: &str, account: &str, native: &str| Epoch {
        id: EpochId::new(id),
        work: work.id.clone(),
        session_ref: None,
        native_id: Some(native.to_owned()),
        account: AccountId::new(account),
        provider: HarnessId::new("claude-code"),
        resumed_from_epoch: None,
        machine: None,
    };
    let (from_epoch, edge) = (format!("{native}-{at}-from"), format!("{native}-{at}-edge"));
    graph
        .insert_epoch(&epoch(&from_epoch, from, native))
        .unwrap();
    graph
        .append_edge(&SwitchEdge {
            id: SwitchEdgeId::new(edge.as_str()),
            work: work.id.clone(),
            from_epoch: EpochId::new(from_epoch.as_str()),
            to_epoch: None,
            from_account: AccountId::new(from),
            to_account: AccountId::new(to),
            from_provider: HarnessId::new("claude-code"),
            to_provider: HarnessId::new("claude-code"),
            mechanism: SwitchMechanism::Transplant,
            pack_id: None,
            fidelity_shown: None,
            manifest_sha256: None,
            state: match outcome {
                Outcome::Failed => EdgeState::Failed {
                    stage: FailureStage::Preflight,
                    reason: "no".into(),
                },
                _ => EdgeState::Staged,
            },
            actor: EdgeActor::User,
            parent_edge_a: None,
            parent_edge_b: None,
            created_at: Timestamp::from_millis(at),
            to_machine: None,
        })
        .unwrap();
    if let Outcome::Launched(installed) = outcome {
        let to_epoch = format!("{native}-{at}-to");
        graph
            .insert_epoch(&epoch(&to_epoch, to, installed))
            .unwrap();
        graph
            .mark_launched(
                &SwitchEdgeId::new(edge.as_str()),
                &EpochId::new(to_epoch.as_str()),
            )
            .unwrap();
    }
}

fn outputs(state: &Arc<AppState>, account: &str) -> Vec<(Option<String>, u64)> {
    let mut rows: Vec<(Option<String>, u64)> = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .events_for_account(account, 0)
        .unwrap()
        .into_iter()
        .map(|event| (event.msg_uuid, event.tokens.output))
        .collect();
    rows.sort();
    rows
}

fn write(path: PathBuf, lines: &[String]) {
    std::fs::write(path, lines.concat()).unwrap();
}

#[test]
fn a_switched_conversations_history_counts_only_where_it_was_spent() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    let (b, c) = (dir.path().join("b"), dir.path().join("c"));
    account(&state, "acc-b", &b);
    account(&state, "acc-c", &c);

    // "conv": two turns on B, switched to C, one more turn there.
    let history = [line("1", "conv", 0, 100), line("2", "conv", 1, 200)];
    write(b.join("projects/lab/conv.jsonl"), &history);
    let mut copy = history.to_vec();
    copy.push(line("3", "conv", SWITCH_MIN + 1, 300));
    write(c.join("projects/lab/conv.jsonl"), &copy);
    // "quiet": switched to C and not used there yet: its copy is all history.
    let quiet = [line("q", "quiet", 0, 50)];
    write(b.join("projects/lab/quiet.jsonl"), &quiet);
    write(c.join("projects/lab/quiet.jsonl"), &quiet);
    let at = T0 + SWITCH_MIN * 60_000;
    switched(
        &state,
        "conv",
        "acc-b",
        "acc-c",
        at,
        Outcome::Launched("conv"),
    );
    switched(
        &state,
        "quiet",
        "acc-b",
        "acc-c",
        at,
        Outcome::Launched("quiet"),
    );

    // What an earlier version stored for C: the carried history too.
    let stored = |id: &str, session: &str, output: u64| UsageEvent {
        account: "acc-c".into(),
        session: Some(session.into()),
        ts: T0,
        model: Some("claude-opus-4".into()),
        tokens: TokenCounts {
            input: 10,
            output,
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: Some(format!("m-{id}/r-{id}")),
    };
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_events(&[
            stored("1", "conv", 100),
            stored("2", "conv", 200),
            stored("q", "quiet", 50),
        ])
        .unwrap();

    usage_pipeline::ingest_pass(&state, 0);

    assert_eq!(
        outputs(&state, "acc-b"),
        vec![
            (Some("m-1/r-1".into()), 100),
            (Some("m-2/r-2".into()), 200),
            (Some("m-q/r-q".into()), 50),
        ],
        "the history stays with the account it was spent on"
    );
    assert_eq!(
        outputs(&state, "acc-c"),
        vec![(Some("m-3/r-3".into()), 300)],
        "the target counts only what was done there"
    );

    // A later turn on C, in a pass that isn't a re-read: still only C's.
    copy.push(line("4", "conv", SWITCH_MIN + 2, 400));
    write(c.join("projects/lab/conv.jsonl"), &copy);
    usage_pipeline::ingest_pass(&state, 1);
    assert_eq!(
        outputs(&state, "acc-c"),
        vec![(Some("m-3/r-3".into()), 300), (Some("m-4/r-4".into()), 400)]
    );
    assert_eq!(outputs(&state, "acc-b").len(), 3);
}

fn state_with(dir: &Path, accounts: &[&str]) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    for id in accounts {
        account(&state, id, &dir.join(id));
    }
    state
}

#[test]
fn a_copy_installed_beside_the_targets_own_keeps_the_targets_usage() {
    // Review of #132 (H2): A → B, back to A, then A → B again. B already
    // has "conv", so the copy is installed as "conv-fork". B's own "conv"
    // is B's real work and keeps all of it; only the new copy's history
    // is A's.
    let dir = tempfile::tempdir().unwrap();
    let state = state_with(dir.path(), &["acc-a", "acc-b"]);
    let b = dir.path().join("acc-b");
    write(
        b.join("projects/lab/conv.jsonl"),
        &[line("1", "conv", 0, 100), line("2", "conv", 1, 200)],
    );
    write(
        b.join("projects/lab/conv-fork.jsonl"),
        &[
            line("f1", "conv-fork", 2, 50),
            line("f2", "conv-fork", SWITCH_MIN + 1, 70),
        ],
    );
    let at = T0 + SWITCH_MIN * 60_000;
    switched(
        &state,
        "conv",
        "acc-a",
        "acc-b",
        at,
        Outcome::Launched("conv-fork"),
    );

    usage_pipeline::ingest_pass(&state, 0);

    assert_eq!(
        outputs(&state, "acc-b"),
        vec![
            (Some("m-1/r-1".into()), 100),
            (Some("m-2/r-2".into()), 200),
            (Some("m-f2/r-f2".into()), 70),
        ],
        "B's own conversation keeps its usage; the new copy's history is A's"
    );
}

#[test]
fn a_failed_or_long_staged_switch_carried_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with(dir.path(), &["acc-a", "acc-b"]);
    let b = dir.path().join("acc-b");
    write(
        b.join("projects/lab/failed.jsonl"),
        &[line("x", "failed", 0, 10)],
    );
    write(
        b.join("projects/lab/stale.jsonl"),
        &[line("y", "stale", 0, 20)],
    );
    let at = T0 + SWITCH_MIN * 60_000;
    switched(&state, "failed", "acc-a", "acc-b", at, Outcome::Failed);
    // Staged on 2026-10-22 and never launched: long past the window in
    // which a usage pass can read a copy before its launch is recorded.
    switched(&state, "stale", "acc-a", "acc-b", at, Outcome::Staged);

    usage_pipeline::ingest_pass(&state, 0);

    assert_eq!(
        outputs(&state, "acc-b"),
        vec![(Some("m-x/r-x".into()), 10), (Some("m-y/r-y".into()), 20)]
    );
}

#[test]
fn a_copy_read_before_its_launch_is_recorded_is_still_left_out() {
    // The few seconds between the install and the launch record: the
    // source's id stands in for a switch staged just now.
    let dir = tempfile::tempdir().unwrap();
    let state = state_with(dir.path(), &["acc-a", "acc-b"]);
    let now = Timestamp::now().millis();
    write(
        dir.path().join("acc-b/projects/lab/live.jsonl"),
        &[line_at("old", "live", now - 120_000, 5)],
    );
    switched(
        &state,
        "live",
        "acc-a",
        "acc-b",
        now - 60_000,
        Outcome::Staged,
    );

    usage_pipeline::ingest_pass(&state, 0);

    assert!(
        outputs(&state, "acc-b").is_empty(),
        "the copy's history is A's"
    );
}

#[test]
fn no_pass_reads_a_copy_while_its_switch_runs() {
    // Re-review of #132 (Low): between the install and the launch record
    // a back-and-forth's copy is still keyed by the source's id, so a pass
    // then would drop B's own conversation's history, for good (offsets
    // are saved). No pass runs while a switch holds its slot.
    let dir = tempfile::tempdir().unwrap();
    let state = state_with(dir.path(), &["acc-a", "acc-b"]);
    let now = Timestamp::now().millis();
    let b = dir.path().join("acc-b/projects/lab");
    write(
        b.join("conv.jsonl"),
        &[line_at("own", "conv", now - 120_000, 5)],
    );
    write(
        b.join("conv-fork.jsonl"),
        &[line_at("carried", "conv-fork", now - 120_000, 7)],
    );
    switched(
        &state,
        "conv",
        "acc-a",
        "acc-b",
        now - 60_000,
        Outcome::Staged,
    );
    state
        .switching
        .lock()
        .unwrap()
        .insert("sess-switching".into());

    usage_pipeline::ingest_pass(&state, 0);
    assert!(
        outputs(&state, "acc-b").is_empty(),
        "nothing is read mid-switch"
    );

    // The launch records the copy's own id, and the switch ends.
    {
        let storage = state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let edge = format!("conv-{}-edge", now - 60_000);
        let from = graph
            .edge(&SwitchEdgeId::new(edge.as_str()))
            .unwrap()
            .from_epoch;
        let work = graph.epoch(&from).unwrap().unwrap().work;
        graph
            .insert_epoch(&Epoch {
                id: EpochId::new("conv-to"),
                work,
                session_ref: None,
                native_id: Some("conv-fork".into()),
                account: AccountId::new("acc-b"),
                provider: HarnessId::new("claude-code"),
                resumed_from_epoch: None,
                machine: None,
            })
            .unwrap();
        graph
            .mark_launched(&SwitchEdgeId::new(edge.as_str()), &EpochId::new("conv-to"))
            .unwrap();
    }
    state.switching.lock().unwrap().clear();

    usage_pipeline::ingest_pass(&state, 1);
    assert_eq!(
        outputs(&state, "acc-b"),
        vec![(Some("m-own/r-own".into()), 5)],
        "B's own conversation keeps its usage; the copy's history is A's"
    );
}
