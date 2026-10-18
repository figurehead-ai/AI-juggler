//! Pack redaction policy and detection (AJ-E208; D-14; deep/09
//! §H5.4). The policy is ALWAYS the user's remembered choice — a
//! global with per-Provider override, never an imposed product
//! default (deep/09's local-off/export-on defaults are superseded:
//! they may only appear as options the user picks). Detection is a
//! hand-rolled pass per pattern (no regex dependency). Secret
//! VALUES never leave this module: the public surface carries
//! kinds, counts, and masked previews only.
//!
//! Index contract: detections are numbered over SEGMENTS (turn
//! texts in order, then the extra files' bytes) — preflight lists
//! and pack-apply strip the same segments, so an un-redact index
//! picked in the preview addresses the exact item the pack build
//! keeps.

use std::collections::{BTreeMap, BTreeSet};

use crate::pack::{ConversationMode, GeneratedPack, PackInput, RedactionCount};

/// The user's secret-handling choice (wire names for settings).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RedactionPolicy {
    /// Replace detected values with `•••`.
    Strip,
    /// Keep values in the pack (it stays on this Mac).
    Keep,
    /// Ask per switch; the per-switch choice is recorded, counts
    /// still land in the manifest.
    AskEachTime,
}

impl RedactionPolicy {
    /// Settings wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strip => "strip",
            Self::Keep => "keep",
            Self::AskEachTime => "ask",
        }
    }

    /// Parses a settings wire name (`None` on anything else — an
    /// unknown value resolves to unset, never guessed).
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "strip" => Some(Self::Strip),
            "keep" => Some(Self::Keep),
            "ask" => Some(Self::AskEachTime),
            _ => None,
        }
    }

    /// The plain-words picker label (same words in Settings →
    /// Privacy and the first-run chooser — no dark pattern).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Strip => "Strip — replace keys with •••",
            Self::Keep => "Keep — pack stays on this Mac",
            Self::AskEachTime => "Ask each time",
        }
    }
}

/// Resolution per D-14: override-if-set, else global; unset stays
/// unset (it BLOCKS pack generation until the user chooses — no
/// code path may treat None as a default).
#[must_use]
pub fn resolve_policy(
    global: Option<RedactionPolicy>,
    provider_override: Option<RedactionPolicy>,
) -> Option<RedactionPolicy> {
    provider_override.or(global)
}

/// Where the effective policy came from (the confirm line names it:
/// "(your global choice)" vs "(override)").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicySource {
    /// The per-Provider override.
    Override,
    /// The user's global choice.
    Global,
}

/// One listed redaction for the preview UI: the kind, the masked
/// value (recognisable, never usable), and which segment it was
/// found in (conversation turn vs pack extra file). The raw value
/// never crosses this type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ListedDetection {
    /// Detection kind (`sk-key` | `bearer` | `env-value` |
    /// `private-key` | `zcode-rollout`).
    pub kind: &'static str,
    /// Masked preview (first char + `•••` + last 4).
    pub preview: String,
    /// Segment ordinal (turns first, then extra files).
    pub segment: usize,
}

/// The raw detection internals (module-private: kind + range in
/// ONE segment's bytes).
#[derive(Clone, PartialEq, Eq, Debug)]
struct Detection {
    kind: &'static str,
    start: usize,
    end: usize,
}

/// The masked preview (first char + … + last 4 — enough to
/// recognise, never enough to use).
#[must_use]
pub fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 5 {
        return "•".repeat(chars.len().max(3));
    }
    let head: String = chars.iter().take(1).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}•••{tail}")
}

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'~' | b'+' | b'/' | b'=')
}

/// Scans forward over token chars from `at`.
fn token_end(bytes: &[u8], at: usize) -> usize {
    let mut end = at;
    while end < bytes.len() && is_token_char(bytes[end]) {
        end += 1;
    }
    end
}

fn scan_sk(text: &str, out: &mut Vec<Detection>) {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = text[at..].find("sk-") {
        let prefix = at + found;
        at = prefix + 3;
        let end = token_end(bytes, at);
        if end - at >= 8 {
            out.push(Detection {
                kind: "sk-key",
                start: at,
                end,
            });
        }
    }
}

fn scan_bearer(text: &str, out: &mut Vec<Detection>) {
    let bytes = text.as_bytes();
    let lower = text.to_ascii_lowercase();
    let mut at = 0;
    while let Some(found) = lower[at..].find("bearer ") {
        let after = at + found + "bearer ".len();
        at = after;
        let end = token_end(bytes, after);
        if end - after >= 8 {
            out.push(Detection {
                kind: "bearer",
                start: after,
                end,
            });
        }
    }
}

/// `.env`-style assignments: `NAME=value` — the NAME must look
/// credential-like, the value ≥ 8 chars.
fn scan_env_value(text: &str, out: &mut Vec<Detection>) {
    for line in text.split('\n') {
        // Diff hunks prefix assignments (`+KEY=…`); strip the marker
        // for the NAME check while keeping byte offsets true.
        let marker_len = line
            .find(|c: char| !matches!(c, '+' | '-' | ' '))
            .unwrap_or(line.len());
        let body = &line[marker_len..];
        let Some((name, value)) = body.split_once('=') else {
            continue;
        };
        let trimmed = name.trim();
        if trimmed.is_empty()
            || !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        let credential_word = trimmed
            .split('_')
            .filter(|word| !word.is_empty())
            .any(|word| {
                matches!(
                    word.to_ascii_uppercase().as_str(),
                    "KEY"
                        | "KEYS"
                        | "TOKEN"
                        | "TOKENS"
                        | "SECRET"
                        | "SECRETS"
                        | "PASSWORD"
                        | "PASSWD"
                        | "PASS"
                        | "APIKEY"
                        | "API"
                        | "AUTH"
                        | "CREDENTIAL"
                        | "CREDENTIALS"
                        | "PRIVATE"
                )
            });
        if !credential_word {
            continue;
        }
        let mut start = value.as_ptr() as usize - line.as_ptr() as usize;
        let mut end = start + value.len();
        let mut value = value;
        if let Some(stripped) = value.strip_prefix('"') {
            value = stripped.strip_suffix('"').unwrap_or(stripped);
            start += 1;
            end -= 1;
        } else if let Some(stripped) = value.strip_prefix('\'') {
            value = stripped.strip_suffix('\'').unwrap_or(stripped);
            start += 1;
            end -= 1;
        }
        let _ = &mut value;
        if end.saturating_sub(start) >= 8 && end <= line.len() {
            let line_start = line.as_ptr() as usize - text.as_ptr() as usize;
            out.push(Detection {
                kind: "env-value",
                start: line_start + start,
                end: line_start + end,
            });
        }
    }
}

fn scan_private_key(text: &str, out: &mut Vec<Detection>) {
    let mut at = 0;
    while let Some(found) = text[at..].find("-----BEGIN") {
        let block_start = at + found;
        let Some(end_rel) = text[block_start..].find("-----END") else {
            break;
        };
        let tail = block_start + end_rel;
        let close = tail + text[tail..].find("-----").unwrap_or(5) + 5;
        // The whole block is the secret — headers included.
        out.push(Detection {
            kind: "private-key",
            start: block_start,
            end: close.min(text.len()),
        });
        at = close;
    }
}

/// `ZCode` rollout headers (`docs/research/zcode.md:64`): model-IO
/// JSON carries request headers; authorization/x-api-key/api-key
/// entries are the secrets.
fn scan_zcode_rollout(text: &str, out: &mut Vec<Detection>) {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    for needle in ["authorization", "x-api-key", "api-key"] {
        let mut at = 0;
        while let Some(found) = lower[at..].find(needle) {
            let name_start = at + found;
            let name_end = name_start + needle.len();
            at = name_end;
            // `api-key` inside `x-api-key` is the same header — the
            // x- variant already scanned it.
            if needle == "api-key" && name_start > 0 && bytes[name_start - 1] == b'-' {
                continue;
            }
            let mut cursor = name_end;
            // JSON keys close with a quote before the colon.
            while cursor < bytes.len()
                && (bytes[cursor] == b'"' || bytes[cursor].is_ascii_whitespace())
            {
                cursor += 1;
            }
            let is_colon = cursor < bytes.len() && bytes[cursor] == b':';
            if !is_colon {
                continue;
            }
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                cursor += 1;
            }
            let end = token_end(bytes, cursor);
            if end.saturating_sub(cursor) >= 8 {
                out.push(Detection {
                    kind: "zcode-rollout",
                    start: cursor,
                    end,
                });
            }
        }
    }
}

/// Scans ONE segment for all five kinds, sorted by byte start then
/// kind (deterministic order).
#[must_use]
fn scan_segment(text: &str) -> Vec<Detection> {
    let mut out = Vec::new();
    scan_sk(text, &mut out);
    scan_bearer(text, &mut out);
    scan_env_value(text, &mut out);
    scan_private_key(text, &mut out);
    scan_zcode_rollout(text, &mut out);
    out.sort_by(|a, b| (a.start, a.kind).cmp(&(b.start, b.kind)));
    out
}

/// Lists detections over the SEGMENTS (turns in order, then extra
/// files) with GLOBAL indices — the preview UI's item list. Values
/// are masked in here; they never leave the engine.
#[must_use]
pub fn list_segments(segments: &[&str]) -> Vec<ListedDetection> {
    let mut out = Vec::new();
    for (segment, text) in segments.iter().enumerate() {
        for detection in scan_segment(text) {
            let value = &text[detection.start..detection.end];
            out.push(ListedDetection {
                kind: detection.kind,
                preview: mask(value),
                segment,
            });
        }
    }
    out
}

/// Strips ONE segment's non-kept values (local keep indices).
fn strip_one(text: &str, keep_local: &BTreeSet<usize>) -> String {
    let detections = scan_segment(text);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (index, detection) in detections.iter().enumerate() {
        if keep_local.contains(&index) {
            continue;
        }
        if detection.start >= detection.end || detection.start < cursor {
            continue;
        }
        out.push_str(&text[cursor..detection.start]);
        out.push_str("•••");
        cursor = detection.end;
    }
    out.push_str(&text[cursor.min(text.len())..]);
    out
}

/// Strips the SEGMENTS under the policy, returning the rewritten
/// segments plus per-kind counts of ALL detections (kept and
/// stripped — the manifest records what was detected and how the
/// policy treated it; under Keep the counts still land).
#[must_use]
pub fn strip_segments(
    segments: &[String],
    strip: bool,
    keep_global: &BTreeSet<usize>,
) -> (Vec<String>, Vec<RedactionCount>) {
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut base = 0;
    let mut out = Vec::with_capacity(segments.len());
    for text in segments {
        let local = scan_segment(text);
        for detection in &local {
            *counts.entry(detection.kind).or_insert(0) += 1;
        }
        let keep_local: BTreeSet<usize> = keep_global
            .range(base..base + local.len())
            .map(|global| global - base)
            .collect();
        out.push(if strip {
            strip_one(text, &keep_local)
        } else {
            text.clone()
        });
        base += local.len();
    }
    let rows = counts
        .into_iter()
        .map(|(kind, count)| RedactionCount {
            kind: kind.to_owned(),
            count,
        })
        .collect();
    (out, rows)
}

/// What a pack generation pass applies (the user's resolved will:
/// strip or keep, plus per-item un-redact indices for THIS pack
/// only — never a remembered-policy change).
#[derive(Clone, Debug, Default)]
pub struct RedactionApply {
    /// Strip the values (false = Keep — counts still recorded).
    pub strip: bool,
    /// Detection indices the user un-redacted for this pack only.
    pub keep: BTreeSet<usize>,
}

/// Generates the pack under the redaction policy: turn texts and
/// extra-file bytes are stripped BEFORE rendering (the rendered
/// conversation and every staged extra carry no secret bytes), and
/// the manifest state records the per-kind counts.
#[must_use]
pub fn redacted_pack(
    input: &PackInput,
    mode: ConversationMode,
    apply: &RedactionApply,
) -> GeneratedPack {
    let mut segments: Vec<String> = input.turns.iter().map(|turn| turn.text.clone()).collect();
    segments.extend(
        input
            .staged_files
            .iter()
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned()),
    );
    let (stripped, rows) = strip_segments(&segments, apply.strip, &apply.keep);
    let mut stripped_input = input.clone();
    for (turn, text) in stripped_input.turns.iter_mut().zip(&stripped) {
        turn.text.clone_from(text);
    }
    let extras: Vec<(String, Vec<u8>)> = stripped_input
        .staged_files
        .iter()
        .cloned()
        .zip(stripped.iter().skip(stripped_input.turns.len()))
        .map(|((path, _), text)| (path, text.clone().into_bytes()))
        .collect();
    stripped_input.staged_files = extras;

    // The rows ride the manifest state at GENERATION time (the
    // manifest hashes the final bytes — patching after would break
    // verification).
    crate::pack::generate_pack_with_redactions(&stripped_input, mode, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_is_override_then_global_then_unset() {
        let strip = Some(RedactionPolicy::Strip);
        let keep = Some(RedactionPolicy::Keep);
        assert_eq!(resolve_policy(strip, None), strip);
        assert_eq!(resolve_policy(strip, keep), keep, "override wins");
        assert_eq!(resolve_policy(None, keep), keep);
        assert_eq!(
            resolve_policy(None, None),
            None,
            "unset stays unset — no default"
        );
        assert_eq!(
            RedactionPolicy::from_wire("strip"),
            Some(RedactionPolicy::Strip)
        );
        assert_eq!(RedactionPolicy::from_wire("bogus"), None);
    }

    #[test]
    fn all_five_kinds_detect() {
        let text = "key sk-abc123def456ghi here\nBearer abcdef1234567890 now\nAPI_TOKEN=\"abcdefgh12345678\"\n-----BEGIN RSA PRIVATE KEY-----\nzzzz\n-----END PRIVATE KEY-----\n{\"headers\":{\"authorization\":\"zzzz1234ffff5678\"}}\n";
        let detections = scan_segment(text);
        let kinds: Vec<&str> = detections.iter().map(|d| d.kind).collect();
        for expected in [
            "sk-key",
            "bearer",
            "env-value",
            "private-key",
            "zcode-rollout",
        ] {
            assert!(kinds.contains(&expected), "missing {expected}: {kinds:?}");
        }
        // The plain Bearer line is its own detection; the JSON
        // header value (no Bearer prefix) is the rollout kind.
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "bearer").count(),
            1,
            "{kinds:?}"
        );
        assert_eq!(kinds.len(), 5, "{kinds:?}");
    }

    #[test]
    fn strip_removes_every_value_and_counts() {
        let text = "sk-abc123def456ghi and Bearer abcd1234efgh5678";
        let (stripped, counts) = strip_segments(&[text.to_owned()], true, &BTreeSet::new());
        assert!(!stripped[0].contains("abc123def456ghi"));
        assert!(!stripped[0].contains("abcd1234efgh5678"));
        assert_eq!(stripped[0].matches("•••").count(), 2);
        assert_eq!(counts.len(), 2);
        assert_eq!(
            counts[0],
            RedactionCount {
                kind: "bearer".into(),
                count: 1
            }
        );
        assert_eq!(
            counts[1],
            RedactionCount {
                kind: "sk-key".into(),
                count: 1
            }
        );
    }

    #[test]
    fn keep_records_counts_without_stripping() {
        let text = "sk-abc123def456ghi";
        let (kept, counts) = strip_segments(&[text.to_owned()], false, &BTreeSet::new());
        assert_eq!(kept[0], text, "keep leaves bytes alone");
        assert_eq!(
            counts,
            vec![RedactionCount {
                kind: "sk-key".into(),
                count: 1
            }]
        );
    }

    #[test]
    fn un_redact_keeps_exactly_that_item_for_this_pass() {
        let segments = vec![
            "sk-first12345678".to_owned(),
            "sk-second87654321".to_owned(),
        ];
        let keep: BTreeSet<usize> = [1].into_iter().collect();
        let (stripped, counts) = strip_segments(&segments, true, &keep);
        assert!(stripped[0].contains("•••"), "item 0 still strips");
        assert_eq!(
            stripped[1], "sk-second87654321",
            "un-redacted item survives"
        );
        assert_eq!(
            counts,
            vec![RedactionCount {
                kind: "sk-key".into(),
                count: 2
            }]
        );
    }

    #[test]
    fn global_indices_span_segments() {
        let segments = ["sk-aaaa11112222".to_owned(), "sk-bbbb33334444".to_owned()];
        let listed = list_segments(&[&segments[0], &segments[1]]);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].segment, 0);
        assert_eq!(listed[1].segment, 1);
        // The detected value is the part after `sk-`.
        assert!(listed[0].preview.starts_with('a'));
        assert!(listed[0].preview.ends_with("2222"));
    }

    #[test]
    fn mask_never_reveals_enough_to_use() {
        assert_eq!(mask("sk-abcdefgh12345678"), "s•••5678");
        assert_eq!(mask("short").chars().count(), 5);
    }

    #[test]
    fn plain_words_are_not_secrets() {
        let text = "MAX_CONNECTIONS=32\nsk-short\nthe task-mask is fine\nskippingalong";
        assert!(scan_segment(text).is_empty(), "{text}");
    }
}
