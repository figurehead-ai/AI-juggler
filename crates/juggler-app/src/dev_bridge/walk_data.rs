//! The 0.2 walk (F19): a fresh AI Juggler Dev data folder filled with the
//! setups found on this computer, safely, for a walk on real data through
//! the bridge (`just dev-bridge-data`, `docs/qa/DEV-BRIDGE.md`).
//!
//! - It finds them with the scan that starts no tool (the adapters'
//!   `scan_environment`, files only: never a `--version`, which first
//!   run's full scan runs), and brings each in the way first run's "Use
//!   them where they are" does (ADR-0018: nothing is copied, marked or
//!   written in the tool's folder). A setup that can't be used where it
//!   is is left out, never copied.
//! - Accounts are named "Account A", "Account B", … so captures carry no
//!   real labels.
//! - `limits_probe_allowed` is `off` (D-39 runs the tool's own `/usage`
//!   headless by default; a walk starts no tool).
//! - It refuses a folder that already holds AI Juggler's data.

use std::sync::Arc;

use crate::commands::{self, ImportRequest};
use crate::state::AppState;

/// One setup brought in: its walk label and its tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filled {
    /// "Account A", "Account B", …
    pub label: String,
    /// The tool's id.
    pub tool: String,
}

/// What [`fill`] did: the setups brought in, and how many were left out
/// (they can't be used where they are, or failed).
#[derive(Debug, Default)]
pub struct Outcome {
    /// Brought in, in scan order.
    pub filled: Vec<Filled>,
    /// Found but not brought in.
    pub left_out: usize,
}

/// The walk label of the `index`th setup: "Account A" … "Account Z",
/// then "Account 27", ….
#[must_use]
pub fn label_at(index: usize) -> String {
    match u8::try_from(index).ok().filter(|index| *index < 26) {
        Some(letter) => format!("Account {}", char::from(b'A' + letter)),
        None => format!("Account {}", index + 1),
    }
}

/// Whether `data_dir` already holds AI Juggler's data (its store).
#[must_use]
pub fn holds_data(data_dir: &std::path::Path) -> bool {
    data_dir.join("juggler.sqlite").exists()
}

/// Brings in every setup the no-tool scan finds that can be used where it
/// is, under walk labels, and turns the limits probe off.
///
/// # Errors
///
/// Storage failures (the probe setting).
pub fn fill(state: &Arc<AppState>) -> Result<Outcome, crate::error::AppError> {
    // A walk starts no tool: off before anything else.
    commands::lock_storage_arc(state)?
        .app_settings()
        .set(crate::features::quota::probe::ALLOWED_KEY, "off")?;
    let mut outcome = Outcome::default();
    for found in commands::environment_scan_impl(state) {
        let in_place = state
            .registry
            .get(&juggler_core::model::HarnessId::new(found.harness.as_str()))
            .is_some_and(|adapter| adapter.can_use_in_place());
        if found.managed || !in_place {
            outcome.left_out += 1;
            continue;
        }
        let label = label_at(outcome.filled.len());
        let request = ImportRequest {
            origin: Some(found.origin.clone()),
            in_place: true,
            ..ImportRequest::default()
        };
        match commands::account_import_mode_impl(
            state,
            &found.harness,
            &found.path,
            &label,
            request,
        ) {
            Ok(_) => outcome.filled.push(Filled {
                label,
                tool: found.harness,
            }),
            Err(_) => outcome.left_out += 1,
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn labels_carry_no_real_names() {
        assert_eq!(label_at(0), "Account A");
        assert_eq!(label_at(6), "Account G");
        assert_eq!(label_at(25), "Account Z");
        assert_eq!(label_at(26), "Account 27");
    }

    #[test]
    fn fills_a_fresh_folder_in_place_with_the_probe_off_and_writes_no_tool_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let home = root.join("home");
        for name in [".codex", ".codex-work"] {
            let codex = home.join(name);
            std::fs::create_dir_all(codex.join("sessions")).unwrap();
            std::fs::write(codex.join("config.toml"), "model = \"x\"\n").unwrap();
        }
        let listing = |path: &std::path::Path| {
            let mut out: Vec<_> = walk(path);
            out.sort();
            out
        };
        let before = listing(&home);
        let mut registry = juggler_adapter::AdapterRegistry::new();
        registry.register(Arc::new(
            juggler_adapters::CodexAdapter::with_bin("/usr/bin/false").with_home(&home),
        ));
        let data = root.join("AI Juggler Dev");
        assert!(!holds_data(&data));
        let state = Arc::new(AppState::init_with_registry(data.clone(), registry).unwrap());
        let outcome = fill(&state).unwrap();

        let labels: Vec<&str> = outcome.filled.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, ["Account A", "Account B"], "{outcome:?}");
        let storage = state.storage.lock().unwrap();
        for account in storage.accounts().list().unwrap() {
            assert!(account.in_place, "{}: used where it is", account.label);
            assert!(account.label.starts_with("Account "));
        }
        assert_eq!(
            storage
                .app_settings()
                .get(crate::features::quota::probe::ALLOWED_KEY)
                .unwrap()
                .as_deref(),
            Some("off")
        );
        drop(storage);
        assert!(holds_data(&data));
        assert_eq!(
            listing(&home),
            before,
            "nothing written in the tool's folders"
        );
    }

    fn walk(path: &std::path::Path) -> Vec<(std::path::PathBuf, u64)> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push((path.clone(), 0));
                out.extend(walk(&path));
            } else {
                out.push((path.clone(), std::fs::metadata(&path).unwrap().len()));
            }
        }
        out
    }
}
