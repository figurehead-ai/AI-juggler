//! D-76 (AJ-E211): leaving out the start of a very large Codex rollout
//! when switching. A Codex turn starts with its `turn_context` line, and
//! a compaction writes a `compacted` line (Codex's own fresh start with
//! its summary). The cut keeps the first line (`session_meta`: the
//! thread's id, folder and version, which `codex resume` reads) and the
//! lines from the cut's turn on; nothing in them changes.

use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde_json::Value;

pub use crate::cut::{Cut, CutPlan, CutWritten, TurnMark, plan};

/// Finds each turn and compaction in a rollout, and its length.
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
        let maybe = memchr::memmem::find(&buf, b"\"turn_context\"").is_some()
            || memchr::memmem::find(&buf, b"\"compacted\"").is_some();
        if maybe && let Ok(value) = serde_json::from_slice::<Value>(&buf) {
            let kind = value.get("type").and_then(Value::as_str);
            if matches!(kind, Some("turn_context" | "compacted")) {
                marks.push(TurnMark {
                    line,
                    offset,
                    at_ms: value
                        .get("timestamp")
                        .and_then(Value::as_str)
                        .and_then(juggler_core::time::timestamp_ms),
                    compaction: kind == Some("compacted"),
                });
            }
        }
        offset += n as u64;
        line += 1;
    }
    Ok((marks, offset))
}

/// What `cut` would leave out of the rollout at `path`.
///
/// # Errors
///
/// The read's IO error.
pub fn preview(path: &Path, cut: Cut) -> std::io::Result<juggler_adapter::handover::CutPreview> {
    crate::cut::preview(path, cut, turn_marks)
}

/// Writes the rollout's first line and its lines from `plan` to `dst`.
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
    let mut head = BufReader::new(std::fs::File::open(src)?);
    let mut first = Vec::new();
    head.read_until(b'\n', &mut first)?;
    if plan.from_offset > 0 {
        out.write_all(&first)?;
        written.lines += 1;
        written.bytes += first.len() as u64;
    }
    let mut file = std::fs::File::open(src)?;
    file.seek(SeekFrom::Start(plan.from_offset))?;
    let mut tail = BufReader::with_capacity(1 << 20, file);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = tail.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf)?;
        written.lines += 1;
        written.bytes += n as u64;
    }
    out.flush()?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Cut, plan, turn_marks, write_cut};

    fn rollout() -> String {
        [
            r#"{"timestamp":"2026-10-01T10:00:00Z","type":"session_meta","payload":{"id":"t-1","cwd":"/w"}}"#,
            r#"{"timestamp":"2026-10-01T10:00:01Z","type":"turn_context","payload":{"cwd":"/w"}}"#,
            r#"{"timestamp":"2026-10-01T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"first"}]}}"#,
            r#"{"timestamp":"2026-10-02T10:00:01Z","type":"turn_context","payload":{"cwd":"/w"}}"#,
            r#"{"timestamp":"2026-10-02T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"second"}]}}"#,
            r#"{"timestamp":"2026-10-03T10:00:01Z","type":"turn_context","payload":{"cwd":"/w"}}"#,
            r#"{"timestamp":"2026-10-03T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"third"}]}}"#,
        ]
        .join("\n")
            + "\n"
    }

    #[test]
    fn a_rollout_keeps_its_first_line_and_the_turns_from_the_cut() {
        let text = rollout();
        let (marks, total) = turn_marks(text.as_bytes()).unwrap();
        assert_eq!(
            marks.iter().map(|m| m.line).collect::<Vec<_>>(),
            vec![1, 3, 5]
        );
        let cut = plan(&marks, total, Cut::KeepLast(2)).unwrap();
        assert_eq!((cut.from_line, cut.turns_left_out), (3, 1));
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("r.jsonl"), dir.path().join("cut.jsonl"));
        std::fs::write(&src, &text).unwrap();
        let wrote = write_cut(&src, &cut, &dst).unwrap();
        let out = std::fs::read_to_string(&dst).unwrap();
        assert!(out.starts_with(r#"{"timestamp":"2026-10-01T10:00:00Z","type":"session_meta""#));
        assert!(!out.contains("first") && out.contains("second") && out.contains("third"));
        assert_eq!(wrote.lines, 5);
    }
}
