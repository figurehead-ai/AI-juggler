//! What an import tells the user afterwards (AJ-E2006): the links it left
//! outside the folder, and what looks like a secret in what it copied.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_core::model::Account;

use crate::state::AppState;

/// The imported Account settled (its cleanup default, its record read
/// back) and, AJ-E2006, what the import left outside the folder and what
/// looks like a secret in what it copied, told once as a notice.
pub fn settled(
    state: &Arc<AppState>,
    account: Account,
    outcome: &juggler_adapter::ImportOutcome,
) -> Account {
    crate::settings_policy::apply_cleanup_default(state, account.id.as_str());
    let account = crate::commands::find_account_arc(state, account.id.as_str()).unwrap_or(account);
    let secrets = secrets_in(&outcome.to_scan, &account.state_dir);
    if let Some(notice) = crate::features::notifications::sources::import_notice(
        account.id.as_str(),
        &account.label,
        &outcome.left_outside,
        &secrets,
    ) && let Ok(storage) = crate::commands::lock_storage_arc(state)
    {
        let _ = crate::features::notifications::centre::record(
            &storage,
            &notice,
            crate::commands::now_millis(),
        );
    }
    account
}

/// The largest copied file the import's redaction scan reads
/// (transcripts, the bulk of a setup, are larger and hold conversations).
const SCANNED_UP_TO: u64 = 4 * 1024 * 1024;

/// AJ-E2006: what looks like a secret in the copied `files`, masked, each
/// named by its path under `root`.
fn secrets_in(files: &[PathBuf], root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    for file in files {
        if std::fs::metadata(file).map_or(true, |m| m.len() > SCANNED_UP_TO) {
            continue;
        }
        let Ok(bytes) = std::fs::read(file) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let name = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string();
        found.extend(
            juggler_handover::redaction::list_segments(&[text.as_ref()])
                .into_iter()
                .map(|segment| format!("{name}: {} ({})", segment.preview, segment.kind)),
        );
    }
    found
}
