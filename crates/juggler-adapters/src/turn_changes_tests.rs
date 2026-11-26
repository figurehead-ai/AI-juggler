#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test: a failed step fails the test"
)]

use juggler_adapter::subagents::LinesChanged;
use serde_json::json;

use super::{TurnChanges, edit_lines, turn_changes};

const T0: &str = "2026-11-26T10:00:00.000Z";
const T1: &str = "2026-11-26T10:01:00.000Z";
const T2: &str = "2026-11-26T10:02:00.000Z";
const T3: &str = "2026-11-26T10:03:00.000Z";

fn asked(at: &str, words: &str) -> String {
    json!({"type": "user", "timestamp": at, "message": {"role": "user", "content": words}})
        .to_string()
}

fn call(at: &str, id: &str, name: &str, input: serde_json::Value) -> String {
    json!({"type": "assistant", "timestamp": at, "message": {"role": "assistant", "content": [
        {"type": "tool_use", "id": id, "name": name, "input": input}
    ]}})
    .to_string()
}

fn result(at: &str, id: &str, described: serde_json::Value) -> String {
    json!({"type": "user", "timestamp": at, "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": id, "content": "ok"}
    ]}, "toolUseResult": described})
    .to_string()
}

fn read(lines: &[String], until: Option<i64>) -> Option<TurnChanges> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    turn_changes(&path, until).unwrap()
}

fn lines(added: u64, removed: u64) -> Option<LinesChanged> {
    Some(LinesChanged { added, removed })
}

#[test]
fn an_edit_counts_the_lines_it_changed_not_the_lines_it_quoted() {
    let changed = edit_lines(
        "fn a() {\n    1\n}\nfn b() {}",
        "fn a() {\n    2\n    3\n}\nfn b() {}",
    );
    assert_eq!(Some(changed), lines(2, 1));
    let turn = read(
        &[
            asked(T0, "fix the sum"),
            call(
                T0,
                "e1",
                "Edit",
                json!({"file_path": "/Users/someone/shop/src/calc.py", "old_string": "a\nb\nc", "new_string": "a\nB\nc\nd"}),
            ),
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 1);
    assert_eq!(turn.names, ["src/calc.py"]);
    assert_eq!(turn.lines, lines(2, 1));
}

#[test]
fn a_multi_edit_sums_its_edits() {
    let turn = read(
        &[
            asked(T0, "rename it"),
            call(
                T0,
                "m1",
                "MultiEdit",
                json!({"file_path": "/w/app/main.rs", "edits": [
                    {"old_string": "let x = 1;", "new_string": "let total = 1;"},
                    {"old_string": "x + 1", "new_string": "total + 1\n// done"},
                ]}),
            ),
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 1);
    assert_eq!(turn.lines, lines(3, 2));
}

#[test]
fn a_write_of_a_new_file_counts_its_lines_by_its_result() {
    let content = "def mul(a, b):\n    return a * b\n";
    let mut transcript = vec![
        asked(T0, "add mul"),
        call(
            T0,
            "w1",
            "Write",
            json!({"file_path": "/w/shop/src/mul.py", "content": content}),
        ),
    ];
    // Before its result is read, the call can't say what the file held.
    let unread = read(&transcript, None).unwrap();
    assert_eq!(unread.files, 1);
    assert_eq!(unread.lines, None, "files only, never a guess");
    transcript.push(result(
        T0,
        "w1",
        json!({"type": "create", "filePath": "/w/shop/src/mul.py", "content": content, "structuredPatch": []}),
    ));
    let turn = read(&transcript, None).unwrap();
    assert_eq!(turn.lines, lines(2, 0));
}

#[test]
fn a_write_over_a_file_and_a_replace_all_count_by_their_patch() {
    let turn = read(
        &[
            asked(T0, "tidy"),
            call(T0, "w2", "Write", json!({"file_path": "/w/a.txt", "content": "x\ny\n"})),
            result(
                T0,
                "w2",
                json!({"type": "update", "filePath": "/w/a.txt", "structuredPatch": [
                    {"oldStart": 1, "oldLines": 3, "newStart": 1, "newLines": 2, "lines": [" x", "-old", "-older", "+y"]}
                ]}),
            ),
            call(
                T0,
                "r1",
                "Edit",
                json!({"file_path": "/w/b.txt", "old_string": "a", "new_string": "b", "replace_all": true}),
            ),
            result(
                T0,
                "r1",
                json!({"filePath": "/w/b.txt", "structuredPatch": [
                    {"lines": ["-a", "+b"]}, {"lines": ["-a", "+b"]}, {"lines": ["-a", "+b"]}
                ]}),
            ),
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 2);
    assert_eq!(turn.lines, lines(4, 5));
}

#[test]
fn a_file_touched_twice_is_one_file_with_both_changes() {
    let turn = read(
        &[
            asked(T0, "two passes"),
            call(T0, "a", "Edit", json!({"file_path": "/w/shop/src/calc.py", "old_string": "1", "new_string": "2"})),
            call(T0, "b", "Edit", json!({"file_path": "/w/shop/src/calc.py", "old_string": "x", "new_string": "y\nz"})),
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 1);
    assert_eq!(turn.names, ["src/calc.py"]);
    assert_eq!(turn.lines, lines(3, 2));
}

#[test]
fn a_call_that_cant_count_shows_the_files_only() {
    let turn = read(
        &[
            asked(T0, "update the notebook and the readme"),
            call(
                T0,
                "n1",
                "NotebookEdit",
                json!({"notebook_path": "/w/lab/data.ipynb", "new_source": "x = 1"}),
            ),
            call(
                T0,
                "e1",
                "Edit",
                json!({"file_path": "/w/lab/README.md", "old_string": "a", "new_string": "b"}),
            ),
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 2);
    assert_eq!(turn.names, ["lab/data.ipynb", "lab/README.md"]);
    assert_eq!(
        turn.lines, None,
        "a sum leaving the notebook out would claim less"
    );
}

#[test]
fn only_the_last_turn_by_its_time_counts_and_a_failed_call_changed_nothing() {
    let failed = json!({"type": "user", "timestamp": T2, "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "f", "content": "String not found", "is_error": true}
    ]}, "toolUseResult": "Error: String not found"})
    .to_string();
    let transcript = [
        asked(T0, "first"),
        call(
            T0,
            "a",
            "Edit",
            json!({"file_path": "/w/one.rs", "old_string": "1", "new_string": "2"}),
        ),
        asked(T1, "second"),
        call(
            T1,
            "b",
            "Edit",
            json!({"file_path": "/w/two.rs", "old_string": "1", "new_string": "2\n3"}),
        ),
        call(
            T2,
            "f",
            "Edit",
            json!({"file_path": "/w/three.rs", "old_string": "nope", "new_string": "x"}),
        ),
        failed,
        asked(T3, "third: nothing changed"),
    ];
    // The end: the third turn changed nothing.
    assert_eq!(read(&transcript, None), None);
    // The second turn, as a feed item at its end tells it.
    let at = juggler_core::time::timestamp_ms(T2).unwrap();
    let turn = read(&transcript, Some(at)).unwrap();
    assert_eq!(turn.names, ["w/two.rs"]);
    let second = transcript[..2]
        .iter()
        .map(|line| line.len() as u64 + 1)
        .sum::<u64>();
    assert_eq!(turn.start, second, "where the second turn's ask starts");
    assert_eq!(turn.lines, lines(2, 1));
}

#[test]
fn a_stop_a_note_and_a_subagents_line_are_not_a_new_turn() {
    let interrupted =
        json!({"type": "user", "timestamp": T1, "message": {"role": "user", "content": [
            {"type": "text", "text": "[Request interrupted by user for tool use]"}
        ]}})
        .to_string();
    let meta = json!({"type": "user", "isMeta": true, "timestamp": T1, "message": {"role": "user", "content": "<command-name>/clear</command-name>"}}).to_string();
    let side = json!({"type": "user", "isSidechain": true, "timestamp": T1, "message": {"role": "user", "content": "the subagent's task"}}).to_string();
    let turn = read(
        &[
            asked(T0, "go"),
            call(
                T0,
                "a",
                "Edit",
                json!({"file_path": "/w/a.rs", "old_string": "1", "new_string": "2"}),
            ),
            interrupted,
            meta,
            side,
        ],
        None,
    )
    .unwrap();
    assert_eq!(turn.files, 1);
}

#[test]
fn a_codex_patch_counts_its_lines_file_by_file_and_a_delete_shows_files_only() {
    let line = |at: &str, kind: &str, payload: serde_json::Value| {
        json!({"timestamp": at, "type": kind, "payload": payload}).to_string()
    };
    let patch = "*** Begin Patch\n*** Update File: /w/shop/src/calc.py\n@@ def add\n-    return a\n+    return a + b\n*** Add File: /w/shop/src/mul.py\n+def mul(a, b):\n+    return a * b\n*** End Patch";
    let mut rollout = vec![
        line(T0, "session_meta", json!({"id": "x"})),
        line(
            T0,
            "event_msg",
            json!({"type": "user_message", "message": "add mul"}),
        ),
        line(
            T0,
            "response_item",
            json!({"type": "custom_tool_call", "name": "apply_patch", "call_id": "p", "input": patch}),
        ),
    ];
    let turn = read(&rollout, None).unwrap();
    assert_eq!(turn.files, 2);
    assert_eq!(turn.names, ["src/calc.py", "src/mul.py"]);
    assert_eq!(turn.lines, lines(3, 1));
    rollout.push(line(
        T0,
        "response_item",
        json!({"type": "custom_tool_call", "name": "apply_patch", "call_id": "q", "input": "*** Begin Patch\n*** Delete File: /w/shop/old.py\n*** End Patch"}),
    ));
    let turn = read(&rollout, None).unwrap();
    assert_eq!(turn.files, 3);
    assert_eq!(turn.lines, None);
}

#[test]
fn a_turn_whose_start_is_out_of_reach_is_not_told() {
    // One turn longer than the longest window: its first calls can't be
    // read, so no count is given (it would be too low).
    let mut transcript = vec![asked(T0, "a long one")];
    let big = "x".repeat(64 * 1024);
    for n in 0..140 {
        transcript.push(call(
            T0,
            &format!("c{n}"),
            "Edit",
            json!({"file_path": format!("/w/f{n}.rs"), "old_string": "", "new_string": big}),
        ));
    }
    assert_eq!(read(&transcript, None), None);
}

/// AJ-E3713 R6a: the stat comes from the transcript alone: nothing in
/// the reader starts a process (never git in the user's projects).
#[test]
fn the_reader_never_runs_git_or_any_command() {
    let source = include_str!("turn_changes.rs");
    for word in [
        "process::Command",
        "Command::new",
        "\"git\"",
        "std::process",
    ] {
        assert!(!source.contains(word), "{word} in turn_changes.rs");
    }
}
