//! A transcript's searchable lines, and the words around a hit, for both
//! search indexes: AI Juggler's sessions (AJ-E1007) and, when the user
//! turns it on, the sessions started outside it (AJ-E1538).
//!
//! A line's text is [`juggler_core::storage::line_text`]: message text,
//! never ids, paths or signatures. A compressed file (Codex's finished
//! rollouts, `.jsonl.zst`) is read whole, within the adapters' bound, and
//! its offsets are offsets in the decompressed text.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Transcript lines read per indexing step (bounded memory).
const INDEX_CHUNK_BYTES: u64 = 8 * 1024 * 1024;

/// Indexed lines `(offset, len, text)` and the offset consumed up to.
pub(crate) type LineBatch = (Vec<(u64, u64, String)>, u64);

/// Reads complete lines of `path` from `start` (at most one chunk):
/// `(offset, len, text)` for lines with searchable text, and the offset
/// up to which the file was consumed.
pub(crate) fn read_lines(path: &Path, start: u64, size: u64) -> std::io::Result<LineBatch> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let take = (size - start).min(INDEX_CHUNK_BYTES);
    let mut buf = Vec::new();
    file.take(take).read_to_end(&mut buf)?;
    // Only complete lines; a partial last line waits for its newline.
    let Some(end) = buf.iter().rposition(|byte| *byte == b'\n') else {
        // One line longer than a chunk: index the chunk as a whole.
        return Ok(if take == INDEX_CHUNK_BYTES {
            let text = juggler_core::storage::line_text(&String::from_utf8_lossy(&buf));
            (vec![(start, take, text)], start + take)
        } else {
            (Vec::new(), start)
        });
    };
    Ok((lines_of(&buf[..end], start), start + end as u64 + 1))
}

/// The whole lines of `path` from the start of the line `offset` falls in,
/// until they cover at least `len` bytes (a line is never cut, so its text
/// is read as the index reads it; one line counts up to a chunk, as
/// there): `(offset, len, text)` for lines with searchable text, and the
/// bytes those lines cover. The estimate's windows (AJ-E1538).
pub(crate) fn window_lines(path: &Path, offset: u64, len: u64) -> std::io::Result<LineBatch> {
    let size = std::fs::metadata(path)?.len();
    let start = line_start(path, offset.min(size))?;
    let (mut lines, mut next) = read_lines(path, start, size.min(start + len))?;
    if next == start {
        // One line longer than the window: the whole line, as the index
        // takes it.
        (lines, next) = read_lines(path, start, size)?;
    }
    lines.retain(|(_, _, text)| !text.is_empty());
    Ok((lines, next - start))
}

/// Where the line holding `offset` starts: the byte after the newline
/// before it, looking back at most one chunk.
fn line_start(path: &Path, offset: u64) -> std::io::Result<u64> {
    const BLOCK: u64 = 64 * 1024;
    let mut file = std::fs::File::open(path)?;
    let mut end = offset;
    while end > 0 && offset - end < INDEX_CHUNK_BYTES {
        let from = end.saturating_sub(BLOCK);
        file.seek(SeekFrom::Start(from))?;
        let mut buf = Vec::new();
        (&mut file).take(end - from).read_to_end(&mut buf)?;
        if let Some(at) = buf.iter().rposition(|byte| *byte == b'\n') {
            return Ok(from + at as u64 + 1);
        }
        end = from;
    }
    Ok(end)
}

/// Every searchable line of a compressed transcript, decompressed (its
/// offsets are in the decompressed text).
pub(crate) fn compressed_lines(path: &Path) -> std::io::Result<Vec<(u64, u64, String)>> {
    let bytes = juggler_adapters::read_session_file(path)?;
    Ok(lines_of(&bytes, 0))
}

/// `(offset, len, text)` for each line of `bytes` with searchable text,
/// offsets counted from `base`.
fn lines_of(bytes: &[u8], base: u64) -> Vec<(u64, u64, String)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    for line in bytes.split(|byte| *byte == b'\n') {
        let text = juggler_core::storage::line_text(&String::from_utf8_lossy(line));
        if !text.trim().is_empty() {
            out.push((base + offset as u64, line.len() as u64, text));
        }
        offset += line.len() + 1;
    }
    out
}

/// The text around the first query word in a transcript line.
pub(crate) fn snippet(path: &Path, offset: u64, len: u64, query: &str) -> Option<String> {
    let len = len.min(1024 * 1024);
    let buf = if juggler_adapters::is_compressed(path) {
        let bytes = juggler_adapters::read_session_file(path).ok()?;
        let from = usize::try_from(offset).ok()?;
        let to = from
            .checked_add(usize::try_from(len).ok()?)?
            .min(bytes.len());
        bytes.get(from..to)?.to_vec()
    } else {
        let mut file = std::fs::File::open(path).ok()?;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut buf = Vec::new();
        file.take(len).read_to_end(&mut buf).ok()?;
        buf
    };
    let text = juggler_core::storage::line_text(&String::from_utf8_lossy(&buf));
    // Redacted as the session page's conversation is (D-14 patterns), over
    // the whole line before the window is cut, so a key that straddles the
    // window's edge is caught too (spec gap audit 2026-10-27: search
    // results showed credential-like strings).
    let (clean, _) = juggler_handover::redaction::strip_segments(
        std::slice::from_ref(&text),
        true,
        &std::collections::BTreeSet::new(),
    );
    let text = clean.into_iter().next().unwrap_or_default();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = text.to_lowercase();
    let at = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| lower.find(&word.to_lowercase()))
        .min()
        .unwrap_or(0);
    let chars: Vec<char> = text.chars().collect();
    let at_char = text[..at.min(text.len())].chars().count();
    let from = at_char.saturating_sub(60);
    let to = (at_char + 120).min(chars.len());
    let mut out: String = chars[from..to].iter().collect();
    if from > 0 {
        out.insert(0, '…');
    }
    if to < chars.len() {
        out.push('…');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::{compressed_lines, read_lines, snippet};

    const LINES: &str = concat!(
        r#"{"type":"user","message":{"content":"Fix the webhook retry"}}"#,
        "\n",
        r#"{"type":"assistant","id":"x","message":{"content":[{"type":"text","text":"Retried the webhook with backoff"}]}}"#,
        "\n",
    );

    /// AJ-E1538: a compressed rollout's lines carry offsets into its
    /// decompressed text, and a snippet reads them back from there; a
    /// plain file reads the same.
    #[test]
    fn compressed_and_plain_files_give_the_same_lines_and_snippets() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("rollout-1.jsonl");
        std::fs::write(&plain, LINES).unwrap();
        let packed = dir.path().join("rollout-1.jsonl.zst");
        std::fs::write(&packed, zstd::encode_all(LINES.as_bytes(), 3).unwrap()).unwrap();

        let size = std::fs::metadata(&plain).unwrap().len();
        let (from_plain, consumed) = read_lines(&plain, 0, size).unwrap();
        assert_eq!(consumed, size);
        let from_packed = compressed_lines(&packed).unwrap();
        let texts =
            |lines: &[(u64, u64, String)]| lines.iter().map(|l| l.2.clone()).collect::<Vec<_>>();
        assert_eq!(texts(&from_packed), texts(&from_plain));
        assert_eq!(
            texts(&from_packed),
            vec![
                "Fix the webhook retry".to_owned(),
                "Retried the webhook with backoff".to_owned()
            ]
        );
        let (offset, len, _) = from_packed[1];
        assert_eq!(
            snippet(&packed, offset, len, "backoff").as_deref(),
            Some("Retried the webhook with backoff")
        );
        assert_eq!(
            snippet(&plain, offset, len, "backoff"),
            snippet(&packed, offset, len, "backoff")
        );
    }

    /// Spec gap audit, 2026-10-27: a search result showed a transcript
    /// line's credential-like string. Snippets are redacted as the
    /// session page's conversation is.
    #[test]
    fn a_snippet_never_shows_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        let line = serde_json::json!({
            "type": "user",
            "message": {"role": "user", "content": "deploy with sk-FAKE0000NOTAREALKEY111 now please"},
        })
        .to_string();
        std::fs::write(&path, format!("{line}\n")).unwrap();
        let size = std::fs::metadata(&path).unwrap().len();
        let (lines, _) = read_lines(&path, 0, size).unwrap();
        let (offset, len, _) = lines[0];
        let shown = snippet(&path, offset, len, "deploy").unwrap();
        assert!(shown.contains("deploy"), "the match still shows");
        assert!(
            !shown.contains("FAKE0000NOTAREALKEY111"),
            "the key is redacted"
        );
    }
}
