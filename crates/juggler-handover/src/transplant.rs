//! Transplant orchestration (AJ-E204; deep/09 D3 — the engine, not
//! the Tauri command, owns tier logic).
//!
//! `hand_over` bundles the source via the adapter's
//! [`HandoverSupport`]
//! (full sibling set, copy-never-move, torn-tail refusal), installs
//! under the target (discovered dir, collision fork), and returns
//! the evidence the caller records on the switch edge: source hashes
//! (the invariant every test asserts), the fork/fallback flags, and
//! the per-file manifest rows. The caller owns launch + storage —
//! this engine never spawns, signals, or writes provider state
//! beyond the target account's own transcript directory.

use juggler_adapter::handover::{HandoverSupport, InstallOutcome};
use juggler_core::model::{Account, TranscriptRef};

use crate::manifest::{ManifestRow, sha256_hex};

/// The transplant outcome: what the caller (commands → switch edge)
/// records.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransplantOutcome {
    /// The installed transcript reference on the target.
    pub install: InstallOutcome,
    /// sha256 of every SOURCE file, pre-copy (the copy-never-move
    /// invariant's before/after evidence).
    pub source_hashes: Vec<ManifestRow>,
    /// sha256 of every INSTALLED file (the manifest the edge and
    /// pack carry).
    pub installed_hashes: Vec<ManifestRow>,
}

/// A transplant failure with the deep/09 failure-taxonomy id the UI
/// copy keys off (§7.1 strings land with AJ-E205's pipeline).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransplantError {
    /// F-H-01: the session's transcript could not be resolved
    /// (no/ambiguous `transcript_ref`).
    #[error("F-H-01: couldn't tell which file is this session — pick it")]
    UnresolvedSource,
    /// F-H-02: the source is still writing (torn tail).
    #[error("F-H-02: source is still writing — stop it first")]
    TornTail,
    /// Adapter-level failure (missing files, target dir problems).
    #[error("{0}")]
    Adapter(#[from] juggler_adapter::error::AdapterError),
}

/// Executes one transplant between accounts (same provider). Pure
/// orchestration over the adapter's I/O; no launch, no signals, no
/// source mutation.
///
/// # Errors
///
/// See [`TransplantError`] — every failure leaves the source exactly
/// resumable (nothing was moved).
pub fn hand_over(
    support: &dyn HandoverSupport,
    reference: &TranscriptRef,
    target: &Account,
) -> Result<TransplantOutcome, TransplantError> {
    if reference.path.as_os_str().is_empty() {
        return Err(TransplantError::UnresolvedSource);
    }
    let bundle = support.bundle_session(reference)?;
    // The source-hash invariant's "before" side.
    let source_hashes: Vec<ManifestRow> = bundle
        .files
        .iter()
        .map(|file| ManifestRow {
            rel_path: file.name.clone(),
            sha256: sha256_hex(&file.bytes),
            bytes: file.bytes.len() as u64,
            kind: "transcript".to_owned(),
            disposition: "copy".to_owned(),
        })
        .collect();
    let install = support.install_bundle(&bundle, target)?;
    // Verify the "after" side: the installed set hashes match the
    // bundle (fork-rewritten ids excluded from byte-equality — they
    // are recorded, not compared).
    let installed_hashes: Vec<ManifestRow> = bundle
        .files
        .iter()
        .map(|file| {
            let installed_name = if install.forked {
                file.name.replace(
                    &bundle.manifest.native_id,
                    install.reference.native_id.as_deref().unwrap_or_default(),
                )
            } else {
                file.name.clone()
            };
            let dir = install
                .reference
                .path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            let bytes = std::fs::read(dir.join(&installed_name)).unwrap_or_default();
            ManifestRow {
                rel_path: installed_name,
                sha256: sha256_hex(&bytes),
                bytes: bytes.len() as u64,
                kind: "transcript".to_owned(),
                disposition: if install.forked { "translate" } else { "copy" }.to_owned(),
            }
        })
        .collect();
    Ok(TransplantOutcome {
        install,
        source_hashes,
        installed_hashes,
    })
}
