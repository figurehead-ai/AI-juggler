//! AJ-E211 (D-76): leaving out the start of a very large Claude
//! conversation when switching, on the user's request (the default
//! carries everything).
//!
//! - [`turn_marks`] reads the transcript once, streaming: each person's
//!   turn (a `user` line that isn't a tool's result, a note or a command:
//!   `claude_transcript::user_prompt`) with its byte offset and time, and
//!   each compaction boundary. Only lines that may be one are parsed.
//! - [`plan`] turns a choice (before a date, or the last N turns) into a
//!   cut at a turn's start, moved to a compaction boundary when one lies
//!   within 10 % of the transcript before it (Claude's own fresh start).
//! - [`write_cut`] streams the kept lines to a new file: the first kept
//!   line becomes the chain's root (`parentUuid` cleared), and any kept
//!   line whose parent was left out is cleared too, so `--resume` walks a
//!   whole chain. Title lines (`summary`) are kept. Nothing else changes.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde_json::Value;

use crate::claude_transcript::user_prompt;

pub use crate::cut::{Cut, CutPlan, CutWritten, TurnMark, plan};

/// Finds each turn and compaction boundary in `reader`, and the total
/// length.
///
/// # Errors
///
/// The read's IO error.
pub fn turn_marks(reader: impl Read) -> std::io::Result<(Vec<TurnMark>, u64)> {
    let mut reader = BufReader::with_capacity(1 << 20, reader);
    let mut marks = Vec::new();
    let mut buf = Vec::new();
    let (mut offset, mut line) = (0u64, 0usize);
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        let maybe_turn = memchr::memmem::find(&buf, b"\"user\"").is_some();
        let maybe_boundary = memchr::memmem::find(&buf, b"compact_boundary").is_some();
        if (maybe_turn || maybe_boundary)
            && let Ok(value) = serde_json::from_slice::<Value>(&buf)
        {
            let compaction =
                value.get("subtype").and_then(Value::as_str) == Some("compact_boundary");
            if compaction || user_prompt(&value).is_some() {
                marks.push(TurnMark {
                    line,
                    offset,
                    at_ms: value
                        .get("timestamp")
                        .and_then(Value::as_str)
                        .and_then(juggler_core::time::timestamp_ms),
                    compaction,
                });
            }
        }
        offset += n as u64;
        line += 1;
    }
    Ok((marks, offset))
}

/// What `cut` would leave out of the Claude transcript at `path`.
///
/// # Errors
///
/// The read's IO error.
pub fn preview(path: &Path, cut: Cut) -> std::io::Result<juggler_adapter::handover::CutPreview> {
    crate::cut::preview(path, cut, turn_marks)
}

/// Writes `src`'s lines from `plan` (and its title lines before it) to
/// `dst`, with the chain whole from its new root.
///
/// # Errors
///
/// The IO error of either file.
pub fn write_cut(src: &Path, plan: &CutPlan, dst: &Path) -> std::io::Result<CutWritten> {
    let mut out = BufWriter::new(std::fs::File::create(dst)?);
    let mut written = CutWritten {
        lines: 0,
        bytes: 0,
        rerooted: 0,
    };
    let mut put = |bytes: &[u8], written: &mut CutWritten| -> std::io::Result<()> {
        out.write_all(bytes)?;
        if !bytes.ends_with(b"\n") {
            out.write_all(b"\n")?;
        }
        written.lines += 1;
        written.bytes += bytes.len() as u64;
        Ok(())
    };
    // The title lines before the cut (they name the conversation).
    let mut head = BufReader::with_capacity(1 << 20, std::fs::File::open(src)?);
    let mut buf = Vec::new();
    let mut read = 0u64;
    while read < plan.from_offset {
        buf.clear();
        let n = head.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        read += n as u64;
        if memchr::memmem::find(&buf, b"\"summary\"").is_some()
            && serde_json::from_slice::<Value>(&buf)
                .is_ok_and(|v| v.get("type").and_then(Value::as_str) == Some("summary"))
        {
            put(&buf, &mut written)?;
        }
    }
    let mut file = std::fs::File::open(src)?;
    file.seek(SeekFrom::Start(plan.from_offset))?;
    let mut tail = BufReader::with_capacity(1 << 20, file);
    let mut kept: HashSet<String> = HashSet::new();
    let mut first = true;
    loop {
        buf.clear();
        if tail.read_until(b'\n', &mut buf)? == 0 {
            break;
        }
        let Ok(mut value) = serde_json::from_slice::<Value>(&buf) else {
            // Not a record (a blank line): carried as it is.
            put(&buf, &mut written)?;
            continue;
        };
        let parent = value
            .get("parentUuid")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let orphan = parent.as_ref().is_some_and(|p| !kept.contains(p));
        if let Some(uuid) = value.get("uuid").and_then(Value::as_str) {
            kept.insert(uuid.to_owned());
        }
        if (first || orphan) && parent.is_some() {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("parentUuid".to_owned(), Value::Null);
            }
            written.rerooted += usize::from(!first);
            let mut line = serde_json::to_vec(&value)?;
            line.push(b'\n');
            put(&line, &mut written)?;
        } else {
            put(&buf, &mut written)?;
        }
        first = false;
    }
    out.flush()?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Cut, plan, turn_marks, write_cut};

    fn user(uuid: &str, parent: Option<&str>, at: &str, text: &str) -> String {
        serde_json::json!({
            "type": "user", "uuid": uuid, "parentUuid": parent, "timestamp": at,
            "message": {"role": "user", "content": text},
        })
        .to_string()
    }

    fn reply(uuid: &str, parent: &str, at: &str) -> String {
        serde_json::json!({
            "type": "assistant", "uuid": uuid, "parentUuid": parent, "timestamp": at,
            "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}]},
        })
        .to_string()
    }

    fn tool_result(uuid: &str, parent: &str, at: &str) -> String {
        serde_json::json!({
            "type": "user", "uuid": uuid, "parentUuid": parent, "timestamp": at,
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t", "content": "x"}]},
        })
        .to_string()
    }

    fn conversation() -> String {
        [
            serde_json::json!({"type": "summary", "summary": "Fix login", "leafUuid": "a3"})
                .to_string(),
            user("u1", None, "2026-10-01T10:00:00Z", "first"),
            reply("a1", "u1", "2026-10-01T10:00:05Z"),
            user("u2", Some("a1"), "2026-10-02T10:00:00Z", "second"),
            reply("a2", "u2", "2026-10-02T10:00:05Z"),
            tool_result("r2", "a2", "2026-10-02T10:00:06Z"),
            user("u3", Some("r2"), "2026-10-03T10:00:00Z", "third"),
            reply("a3", "u3", "2026-10-03T10:00:05Z"),
        ]
        .join("\n")
            + "\n"
    }

    #[test]
    fn turns_are_peoples_messages_not_tool_results() {
        let (marks, total) = turn_marks(conversation().as_bytes()).unwrap();
        assert_eq!(
            marks.iter().map(|m| m.line).collect::<Vec<_>>(),
            vec![1, 3, 6]
        );
        assert_eq!(total, conversation().len() as u64);
    }

    #[test]
    fn a_cut_lands_on_a_turn_and_the_chain_resumes_from_it() {
        let text = conversation();
        let (marks, total) = turn_marks(text.as_bytes()).unwrap();
        let cut = plan(&marks, total, Cut::KeepLast(2)).unwrap();
        assert_eq!(
            (cut.from_line, cut.turns_left_out, cut.turns_kept),
            (3, 1, 2)
        );
        let by_date = plan(&marks, total, Cut::Before(1_790_931_600_000)).unwrap();
        assert_eq!(
            by_date.from_line, 3,
            "2026-10-02 09:00: the turn at 10:00 is the first kept"
        );
        assert!(
            plan(&marks, total, Cut::KeepLast(3)).is_none(),
            "nothing left out"
        );
        assert!(
            plan(&marks, total, Cut::Before(i64::MAX)).is_none(),
            "every turn would go"
        );

        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("s.jsonl"), dir.path().join("cut.jsonl"));
        std::fs::write(&src, &text).unwrap();
        let wrote = write_cut(&src, &cut, &dst).unwrap();
        let lines: Vec<serde_json::Value> = std::fs::read_to_string(&dst)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines[0]["type"], "summary", "the title stays");
        assert_eq!(lines[1]["uuid"], "u2");
        assert!(lines[1]["parentUuid"].is_null(), "the new root");
        // Every kept line's parent is kept: the chain is whole.
        let uuids: Vec<&str> = lines.iter().filter_map(|l| l["uuid"].as_str()).collect();
        for l in &lines[1..] {
            if let Some(p) = l["parentUuid"].as_str() {
                assert!(uuids.contains(&p), "{p} is kept");
            }
        }
        assert_eq!((wrote.lines, wrote.rerooted), (6, 0));
        assert!(
            !std::fs::read_to_string(&dst).unwrap().contains("first"),
            "left out"
        );
    }

    #[test]
    fn a_compaction_boundary_near_before_the_cut_is_where_it_lands() {
        let boundary = serde_json::json!({
            "type": "system", "subtype": "compact_boundary", "uuid": "b1", "parentUuid": null,
            "logicalParentUuid": "a2", "timestamp": "2026-10-02T23:00:00Z",
        })
        .to_string();
        let text = [
            user("u1", None, "2026-10-01T10:00:00Z", &"long ".repeat(400)),
            reply("a1", "u1", "2026-10-01T10:00:05Z"),
            user("u2", Some("a1"), "2026-10-02T10:00:00Z", "second"),
            reply("a2", "u2", "2026-10-02T10:00:05Z"),
            boundary,
            user("u3", Some("b1"), "2026-10-03T10:00:00Z", "third"),
            reply("a3", "u3", "2026-10-03T10:00:05Z"),
        ]
        .join("\n")
            + "\n";
        let (marks, total) = turn_marks(text.as_bytes()).unwrap();
        let cut = plan(&marks, total, Cut::KeepLast(1)).unwrap();
        assert!(cut.at_compaction, "{cut:?}");
        assert_eq!((cut.from_line, cut.turns_left_out), (4, 2));
    }

    #[test]
    fn a_kept_line_whose_parent_was_left_out_becomes_a_root() {
        let text = [
            user("u1", None, "2026-10-01T10:00:00Z", "first"),
            user("u2", Some("u1"), "2026-10-02T10:00:00Z", "second"),
            reply("x1", "u1", "2026-10-02T10:00:01Z"),
        ]
        .join("\n")
            + "\n";
        let (marks, total) = turn_marks(text.as_bytes()).unwrap();
        let cut = plan(&marks, total, Cut::KeepLast(1)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("s.jsonl"), dir.path().join("cut.jsonl"));
        std::fs::write(&src, &text).unwrap();
        let wrote = write_cut(&src, &cut, &dst).unwrap();
        assert_eq!(wrote.rerooted, 1, "x1's parent u1 was left out");
    }
}
