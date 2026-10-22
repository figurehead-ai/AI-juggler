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
    Account, AccountId, Confidence, EdgeActor, EdgeState, Epoch, EpochId, HarnessId, SwitchEdge,
    SwitchEdgeId, SwitchMechanism, Timestamp, TokenCounts,
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

/// A transplant of conversation `native` from `from` to `to` at `at`.
fn switched(state: &Arc<AppState>, native: &str, from: &str, to: &str, at: i64) {
    let storage = state.storage.lock().unwrap();
    let project = storage
        .projects()
        .touch(Path::new("/work/lab"), None)
        .unwrap()
        .id;
    let graph = storage.switch_graph();
    let work = graph.create_work(&project, native).unwrap();
    let epoch = |id: String, account: &str| Epoch {
        id: EpochId::new(id),
        work: work.id.clone(),
        session_ref: None,
        native_id: Some(native.to_owned()),
        account: AccountId::new(account),
        provider: HarnessId::new("claude-code"),
        resumed_from_epoch: None,
        machine: None,
    };
    graph
        .insert_epoch(&epoch(format!("{native}-from"), from))
        .unwrap();
    graph
        .append_edge(&SwitchEdge {
            id: SwitchEdgeId::new(format!("{native}-edge")),
            work: work.id.clone(),
            from_epoch: EpochId::new(format!("{native}-from")),
            to_epoch: None,
            from_account: AccountId::new(from),
            to_account: AccountId::new(to),
            from_provider: HarnessId::new("claude-code"),
            to_provider: HarnessId::new("claude-code"),
            mechanism: SwitchMechanism::Transplant,
            pack_id: None,
            fidelity_shown: None,
            manifest_sha256: None,
            // Staged: a usage pass can read the copy before the launch is
            // recorded, and must already leave its history out.
            state: EdgeState::Staged,
            actor: EdgeActor::User,
            parent_edge_a: None,
            parent_edge_b: None,
            created_at: Timestamp::from_millis(at),
            to_machine: None,
        })
        .unwrap();
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
    switched(&state, "conv", "acc-b", "acc-c", T0 + SWITCH_MIN * 60_000);
    switched(&state, "quiet", "acc-b", "acc-c", T0 + SWITCH_MIN * 60_000);

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
