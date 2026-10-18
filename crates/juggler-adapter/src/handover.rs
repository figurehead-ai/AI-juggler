//! Handover support contract (AJ-E204; deep/09 §H1, H-1).
//!
//! An OPTIONAL sub-trait of [`crate::adapter::HarnessAdapter`] so the
//! semver-protected main trait is unchanged (ADR-0002) and adapters
//! opt in incrementally. A provider without `HandoverSupport` is
//! replay/pack-only (T3/T4). Alongside the trait live the pure
//! transcript-format helpers (sibling-set resolution, tail
//! verification, session-id forking, embedded-`cwd` reading) —
//! byte-level logic shared by every implementing adapter and unit
//! tested here, per the H-2/H-3 mandates: resolve via
//! `SessionRecord.transcript_ref` (never newest-mtime), copy the full
//! sibling set (never one file), and discover — never compute — the
//! target directory from `cwd`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::adapter::HarnessAdapter;
use crate::error::AdapterError;
use crate::model::{Account, HarnessId};
use juggler_core::model::{Timestamp, TranscriptRef};

/// sha256 hex of `bytes` (the hash every manifest row and
/// copy-never-move invariant uses).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// One file inside a [`SessionBundle`] (bytes in memory; bundles are
/// bounded by the session set, streamed by callers when huge).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BundleFile {
    /// File name (not a deep path — the sibling set is flat).
    pub name: String,
    /// The file's bytes.
    pub bytes: Vec<u8>,
}

/// The bundle manifest: identity + fidelity evidence for what was
/// copied (deep/09 §H1 shape).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BundleManifest {
    /// Manifest schema (1).
    pub schema: u32,
    /// The harness this bundle belongs to.
    pub harness: HarnessId,
    /// The vendor-native session id.
    pub native_id: String,
    /// The working directory the session ran in (read from the
    /// transcript's embedded `cwd` — the discovery key).
    pub cwd: PathBuf,
    /// Harness version observed at bundle time, when known.
    pub harness_version: Option<String>,
    /// When the bundle was taken.
    pub created_at: Timestamp,
    /// The fidelity honestly attributable to this bundle.
    pub fidelity: Fidelity,
    /// Whether any redaction was applied (pack policy, AJ-E208).
    pub redacted: bool,
}

/// Fidelity evidence carried on a bundle.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Fidelity {
    /// The F-label (`F0`–`F4`).
    pub level: String,
    /// Caps applied (e.g. `unvalidated-pair`, `target-dir-fallback`).
    pub caps_applied: Vec<String>,
}

/// A full session bundle: the main transcript plus every sibling
/// file (subagent transcripts, spill, file-history inventory) — a
/// single-file bundle is not constructible through the honest path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SessionBundle {
    /// Identity + fidelity.
    pub manifest: BundleManifest,
    /// The `<session>*` sibling set (main file included).
    pub files: Vec<BundleFile>,
}

/// The prime-prompt input: distilled turns (T3 replay).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TurnText {
    /// Message uuid, when the source carried one.
    pub uuid: Option<String>,
    /// Who spoke (`user` / `assistant`).
    pub role: &'static str,
    /// The distilled text.
    pub text: String,
}

/// The install result: where the session landed plus the honesty
/// flags the manifest/UI must carry (H-3's flagged fallback, F-H-03's
/// fork notice).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InstallOutcome {
    /// The installed transcript's reference.
    pub reference: TranscriptRef,
    /// The native id forked (collision on target).
    pub forked: bool,
    /// The target dir came from the flagged mangle fallback, NOT
    /// discovery — surfaced, never hidden.
    pub target_dir_fallback: bool,
}

/// Optional handover capability (T2 transplant + T3 prime). The
/// engine orchestrates; the adapter performs harness-specific I/O.
pub trait HandoverSupport: HarnessAdapter {
    /// Bundles the session at `reference` — the FULL sibling set,
    /// copy-never-move (the source stays byte-identical).
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] semantics for a missing file;
    /// other [`AdapterError`] variants for read failures or a torn
    /// tail (F-H-02).
    fn bundle_session(&self, reference: &TranscriptRef) -> Result<SessionBundle, AdapterError>;

    /// Installs a bundle under the target account, DISCOVERING the
    /// target project directory by embedded `cwd` (H-3; the naive
    /// mangle is a flagged fallback, never the primary). Returns the
    /// installed transcript's reference (forked id on collision,
    /// F-H-03).
    ///
    /// # Errors
    ///
    /// [`AdapterError::AccountDir`] for path/write failures.
    fn install_bundle(
        &self,
        bundle: &SessionBundle,
        target: &Account,
    ) -> Result<InstallOutcome, AdapterError>;

    /// Builds the prime prompt for T3 replay from distilled turns.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] when the harness has no prime
    /// path.
    fn prime_prompt(&self, turns: &[TurnText]) -> Result<String, AdapterError>;
}

/// Lists the `<stem>*` sibling set in `dir` (the main file plus
/// subagent transcripts, spill, file-history inventory) — F-H-04's
/// honest inventory. Sorted for determinism.
#[must_use]
pub fn sibling_paths(dir: &Path, stem: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(stem))
        })
        .collect();
    out.sort();
    out
}

/// Whether the last non-empty line parses as JSON — the F-H-02 torn
/// tail check (a source still appending produces a partial line).
#[must_use]
pub fn tail_parses(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
}

/// Reads the embedded `"cwd"` from the first lines of a Claude JSONL
/// (the discovery key — decoding the mangled directory name is
/// ambiguous on dots/dashes, H-3).
#[must_use]
pub fn embedded_cwd(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines().take(25) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(cwd) = value.get("cwd").and_then(serde_json::Value::as_str) {
            return Some(cwd.to_owned());
        }
    }
    None
}

/// Rewrites the top-level `sessionId` on every line to `new_id`
/// (F-H-03 collision fork): a NEW native id, the `uuid`/`parentUuid`
/// tree preserved line-by-line. Lines that fail to parse pass
/// through byte-identical (never dropped).
#[must_use]
pub fn fork_session_id(bytes: &[u8], new_id: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push('\n');
            continue;
        }
        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "sessionId".to_owned(),
                    serde_json::Value::String(new_id.to_owned()),
                );
            }
            out.push_str(&value.to_string());
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.into_bytes()
}

/// The set of message `uuid`s in a transcript (the first-wins dedupe
/// key — H2.1's cycle-safety).
#[must_use]
pub fn uuid_set(bytes: &[u8]) -> BTreeSet<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(uuid) = value.get("uuid").and_then(serde_json::Value::as_str) {
            out.insert(uuid.to_owned());
        }
    }
    out
}

/// The naive vendor mangle (`/` → `-`) — ONLY a flagged fallback
/// when discovery finds no target dir (H-3); documented as
/// collision-prone (`/a/b.c` vs `/a/b-c`).
#[must_use]
pub fn mangled_dir(cwd: &str) -> String {
    cwd.replace('/', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(uuid: &str, parent: Option<&str>, session: &str, cwd: &str) -> String {
        serde_json::json!({
            "uuid": uuid,
            "parentUuid": parent,
            "sessionId": session,
            "cwd": cwd,
            "type": "user",
            "message": {"role": "user", "content": "hi"},
        })
        .to_string()
    }

    #[test]
    fn tail_parses_rejects_a_torn_line() {
        let whole = format!("{}\n", line("u1", None, "s1", "/w"));
        assert!(tail_parses(whole.as_bytes()));
        let torn = format!("{whole}{{\"uuid\": \"u2\", \"pare");
        assert!(!tail_parses(torn.as_bytes()));
    }

    #[test]
    fn embedded_cwd_reads_from_the_first_lines() {
        let bytes = format!("{}\n", line("u1", None, "s1", "/Users/dev/weird dir")).into_bytes();
        assert_eq!(
            embedded_cwd(&bytes),
            Some("/Users/dev/weird dir".to_owned())
        );
        assert_eq!(embedded_cwd(b"not json\n"), None);
    }

    #[test]
    fn fork_preserves_the_tree_and_rewrites_the_id() {
        let body = format!(
            "{}\n{}\n",
            line("u1", None, "s-old", "/w"),
            line("u2", Some("u1"), "s-old", "/w")
        );
        let forked = fork_session_id(body.as_bytes(), "s-new");
        let forked_text = String::from_utf8_lossy(&forked).into_owned();
        assert!(forked_text.contains("\"sessionId\":\"s-new\""));
        assert!(!forked_text.contains("s-old"));
        assert!(
            forked_text.contains("\"parentUuid\":\"u1\""),
            "tree preserved"
        );
        // Unparseable lines pass through byte-identical.
        let mixed = format!("{body}{{torn");
        let forked_mixed = fork_session_id(mixed.as_bytes(), "s-new");
        assert!(String::from_utf8_lossy(&forked_mixed).contains("{torn"));
    }

    #[test]
    fn uuid_set_collects_ids() {
        let body = format!(
            "{}\n{}\n",
            line("u1", None, "s", "/w"),
            line("u2", Some("u1"), "s", "/w")
        );
        let set = uuid_set(body.as_bytes());
        assert!(set.contains("u1") && set.contains("u2"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn mangle_is_the_flagged_fallback_only() {
        assert_eq!(mangled_dir("/a/b.c"), "-a-b.c");
        // Fallback status is structural: discovery is the primary
        // path and flags its use — this helper never decides alone.
    }
}
