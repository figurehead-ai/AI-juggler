//! Claude Code transplant mechanics (AJ-E204; deep/09 §H1–§H4).
//!
//! The honest rebuild of v0: the source resolves via
//! `SessionRecord.transcript_ref` (never newest-mtime, F-H-01); the
//! bundle copies the FULL `<session>*` sibling set with a manifest
//! row per file (single-file copy is unreachable, F-H-04); a torn
//! tail refuses the bundle (F-H-02); the target directory is
//! DISCOVERED by scanning the target's `projects/*` and matching the
//! embedded `"cwd"` — the naive mangle is a flagged fallback only
//! (F-H-05/H-3); and an id collision on the target forks a new
//! native id with the `uuid`/`parentUuid` tree preserved (F-H-03).
//! The source is copied, never moved — byte-identical, always.

use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::{
    BundleFile, BundleManifest, Fidelity, InstallOutcome, SessionBundle, TurnText, embedded_cwd,
    fork_session_id, mangled_dir, sibling_paths, tail_parses,
};
use juggler_adapter::model::Account;
use juggler_core::model::{Timestamp, TranscriptRef};

use crate::claude_code::ClaudeCodeAdapter;
use crate::claude_transcript;

impl juggler_adapter::handover::HandoverSupport for ClaudeCodeAdapter {
    fn bundle_session(&self, reference: &TranscriptRef) -> Result<SessionBundle, AdapterError> {
        let path = &reference.path;
        if !path.is_file() {
            return Err(AdapterError::Unsupported(format!(
                "transcript not found: {}",
                path.display()
            )));
        }
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .ok_or_else(|| AdapterError::Unsupported("transcript has no session id".into()))?;
        let main_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        // F-H-04: the FULL sibling set, manifest row per file. A
        // partial read or torn tail refuses the whole bundle.
        let mut files = Vec::new();
        for sibling in sibling_paths(dir, &stem) {
            let bytes = std::fs::read(&sibling)?;
            if sibling
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                == Some(main_name.clone())
                && !tail_parses(&bytes)
            {
                // F-H-02: the source is still writing (or corrupt) —
                // freeze first, never copy torn bytes.
                return Err(AdapterError::Unsupported(
                    "source transcript tail does not parse — stop the session first".into(),
                ));
            }
            files.push(BundleFile {
                name: sibling
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                bytes,
            });
        }
        let main = files
            .iter()
            .find(|file| file.name == main_name)
            .ok_or_else(|| {
                AdapterError::Unsupported("main transcript missing from sibling set".into())
            })?;
        let cwd = embedded_cwd(&main.bytes).map_or_else(|| dir.to_path_buf(), PathBuf::from);
        let native_id = reference.native_id.clone().unwrap_or(stem);
        // §H2.1: a compacted source caps the honest label at F2 (the
        // pre-compaction turns no longer travel). Everything stays
        // "unvalidated-pair" until AJ-E210's live cells graduate it.
        let compacted = claude_transcript::parse_transcript(&main.bytes).has_compact_boundary();
        Ok(SessionBundle {
            manifest: BundleManifest {
                schema: 1,
                harness: self.id(),
                native_id,
                cwd,
                harness_version: None,
                created_at: Timestamp::now(),
                fidelity: Fidelity {
                    level: if compacted { "F2" } else { "F1" }.to_owned(),
                    caps_applied: if compacted {
                        vec!["compact-boundary".to_owned(), "unvalidated-pair".to_owned()]
                    } else {
                        vec!["unvalidated-pair".to_owned()]
                    },
                },
                redacted: false,
            },
            files,
        })
    }

    fn install_bundle(
        &self,
        bundle: &SessionBundle,
        target: &Account,
    ) -> Result<InstallOutcome, AdapterError> {
        let cwd_key = bundle.manifest.cwd.to_string_lossy().into_owned();
        let (dir_b, fallback) = discover_project_dir(&target.state_dir.join("projects"), &cwd_key);

        // F-H-03: fork when the session id already exists on the
        // target (dedupe first-wins by uuid on ingest; the fork
        // keeps the tree and rewrites the id chain).
        let main_name = format!("{}.jsonl", bundle.manifest.native_id);
        let collision = bundle
            .files
            .iter()
            .find(|file| file.name == main_name)
            .is_some_and(|_| dir_b.join(&main_name).exists());
        let installed_id = if collision {
            format!("{}-fork-{}", bundle.manifest.native_id, short_unique())
        } else {
            bundle.manifest.native_id.clone()
        };

        std::fs::create_dir_all(&dir_b).map_err(|error| AdapterError::AccountDir {
            path: dir_b.clone(),
            message: error.to_string(),
        })?;
        // Stage to a temp sibling + atomic rename per file set: a
        // crash leaves the old state, never a half-installed session
        // (F-H-10 — the pipeline in AJ-E205 generalizes this).
        let staging = dir_b.join(format!(".stage-{}-{}", installed_id, std::process::id()));
        std::fs::create_dir_all(&staging).map_err(|error| AdapterError::AccountDir {
            path: staging.clone(),
            message: error.to_string(),
        })?;
        for file in &bundle.files {
            let bytes = if collision && file.name.to_ascii_lowercase().ends_with(".jsonl") {
                fork_session_id(&file.bytes, &installed_id)
            } else {
                file.bytes.clone()
            };
            let name = if collision {
                file.name.replace(&bundle.manifest.native_id, &installed_id)
            } else {
                file.name.clone()
            };
            std::fs::write(staging.join(&name), &bytes).map_err(|error| {
                AdapterError::AccountDir {
                    path: staging.join(&name),
                    message: error.to_string(),
                }
            })?;
        }
        for entry in std::fs::read_dir(&staging).into_iter().flatten().flatten() {
            let from = entry.path();
            let to = dir_b.join(entry.file_name());
            std::fs::rename(&from, &to).map_err(|error| AdapterError::AccountDir {
                path: to,
                message: error.to_string(),
            })?;
        }
        let _ = std::fs::remove_dir_all(&staging);

        Ok(InstallOutcome {
            reference: TranscriptRef {
                path: dir_b.join(format!("{installed_id}.jsonl")),
                native_id: Some(installed_id),
            },
            forked: collision,
            target_dir_fallback: fallback,
        })
    }

    fn prime_prompt(&self, turns: &[TurnText]) -> Result<String, AdapterError> {
        // T3 prime: the distilled replay brief. Deterministic from
        // the turns; no LLM distillation in the deterministic path.
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "You are continuing a session that previously ran under another account."
        );
        let _ = writeln!(
            out,
            "Below is the distilled history — continue from here.\n"
        );
        for turn in turns {
            let _ = writeln!(out, "[{}]: {}", turn.role, turn.text);
            let _ = writeln!(out);
        }
        Ok(out)
    }
}

/// Discovers the target project dir for `cwd`: scan `projects/*`,
/// read one `.jsonl`'s embedded `cwd`, match exactly (H-3
/// discover-don't-compute). Returns `(dir, fallback_used)` — the
/// naive mangle only when nothing matches, FLAGGED so the manifest
/// and UI can say so (never hidden).
fn discover_project_dir(projects_root: &Path, cwd: &str) -> (PathBuf, bool) {
    if let Ok(entries) = std::fs::read_dir(projects_root) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let matched = std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .any(|file| {
                    file.path().extension().is_some_and(|ext| ext == "jsonl")
                        && std::fs::read(file.path())
                            .ok()
                            .and_then(|bytes| embedded_cwd(&bytes))
                            .is_some_and(|found| found == cwd)
                });
            if matched {
                return (dir, false);
            }
        }
    }
    (projects_root.join(mangled_dir(cwd)), true)
}

/// Short unique suffix for fork ids.
fn short_unique() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", Timestamp::now().millis().cast_unsigned(), seq)
}

/// Whether a transcript set contains a `compact_boundary` (re-export
/// of the parser helper for the handover engine's F2 cap).
#[must_use]
pub fn bundle_is_compacted(bundle: &SessionBundle) -> bool {
    bundle
        .files
        .iter()
        .find(|file| file.name.to_ascii_lowercase().ends_with(".jsonl"))
        .is_some_and(|file| claude_transcript::parse_transcript(&file.bytes).has_compact_boundary())
}
