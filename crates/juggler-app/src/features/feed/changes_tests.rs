#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test: a failed step fails the test"
)]

//! AJ-E3713 R6a: a post's diff stat comes from its session's own
//! transcript, through an adapter that reads its tool's work; a tool
//! whose transcripts aren't read carries none.

use std::sync::Arc;

use juggler_core::model::{SessionStatus, TranscriptRef};
use serde_json::json;

use super::attach;
use crate::features::feed::dto::{FeedAuthorInfo, FeedItemInfo};
use crate::features::notifications::centre::NoticeTarget;

fn state(dir: &std::path::Path) -> Arc<crate::state::AppState> {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        "/nowhere",
    )));
    Arc::new(crate::state::AppState::init_with_registry(dir.join("data"), registry).unwrap())
}

fn item(session: &str, provider: &str, kind: &str, at_ms: i64) -> FeedItemInfo {
    FeedItemInfo {
        id: format!("l:{session}"),
        at_ms,
        kind: kind.to_owned(),
        category: "finished".to_owned(),
        level: 1,
        mine: false,
        coalesce: None,
        rollup: None,
        rollup_window_ms: 0,
        scope: None,
        author: FeedAuthorInfo {
            kind: "app".to_owned(),
            provider: Some(provider.to_owned()),
            ..FeedAuthorInfo::default()
        },
        subject: "calc".to_owned(),
        facts: crate::features::feed::dto::FeedFactsInfo::default(),
        target: NoticeTarget {
            session_id: Some(session.to_owned()),
            ..NoticeTarget::default()
        },
        digest: None,
    }
}

#[test]
fn a_finished_turn_carries_what_its_calls_changed_and_another_tool_carries_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    let transcript = dir.path().join("s1.jsonl");
    let lines = [
        json!({"type": "user", "timestamp": "2026-11-26T10:00:00Z", "message": {"role": "user", "content": "add mul"}}),
        json!({"type": "assistant", "timestamp": "2026-11-26T10:00:10Z", "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": "e", "name": "Edit", "input": {"file_path": "/w/shop/src/calc.py", "old_string": "a", "new_string": "a\nb"}}
        ]}}),
    ];
    std::fs::write(
        &transcript,
        lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        storage
            .raw_conn()
            .execute_batch(
                "INSERT INTO accounts (id, provider_id, label, state_dir, created_at) VALUES ('acc', 'fake', 'Lab', '/x', 0);
                 INSERT INTO launch_recipes (id, account_id, settings_json, created_at) VALUES ('prof', 'acc', '{}', 0);",
            )
            .unwrap();
        for id in ["s1", "s2"] {
            let mut record = crate::test_support::instance(
                id,
                "prof",
                SessionStatus::Finished,
                "/w/shop".into(),
            );
            record.transcript_ref = Some(TranscriptRef {
                path: transcript.clone(),
                native_id: None,
            });
            storage.sessions().insert(&record).unwrap();
        }
    }
    let at = juggler_core::time::timestamp_ms("2026-11-26T10:00:20Z").unwrap();
    let mut items = vec![
        item("s1", juggler_adapters::FAKE_HARNESS_ID, "finished", at),
        // A tool with no adapter that reads its work: no stat.
        item("s2", "grok-build", "finished", at),
        // Not a turn's end: no stat.
        item("s1", juggler_adapters::FAKE_HARNESS_ID, "restarted", at),
    ];
    attach(&state, &mut items);
    let changes = items[0].facts.changes.as_ref().expect("the turn's stat");
    assert_eq!(changes.files, 1);
    assert_eq!(changes.names, ["src/calc.py"]);
    assert_eq!((changes.added, changes.removed), (Some(1), Some(0)));
    assert_eq!(items[1].facts.changes, None);
    assert_eq!(items[2].facts.changes, None);
    // Read again: the same, from what was kept.
    let mut again = vec![item(
        "s1",
        juggler_adapters::FAKE_HARNESS_ID,
        "finished",
        at,
    )];
    attach(&state, &mut again);
    assert_eq!(again[0].facts.changes.as_ref(), Some(changes));
}
