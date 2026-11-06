//! AJ-E210: the ordered-pair validation matrix (D-04) and the human-run
//! dogfood records that graduate pairs out of Beta.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::storage::Storage;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::providers::installed_versions_for;
use crate::features::validation::dto::{DogfoodCellInfo, ValidationCellInfo};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// AJ-E210: the ordered-pair validation matrix (D-04) — the source of
// truth that graduates pairs out of "Beta". Read side renders in
// Settings → Beta; the record side stores HUMAN-run live dogfood
// cells (the app never runs vendor binaries itself).

/// KV key for the recorded live dogfood cells (JSON map
/// "from→to" → [`juggler_handover::validation::DogfoodCell`]).
const DOGFOOD_CELLS_KEY: &str = "dogfood_cells";

/// The map key for one pair's recorded cell.
pub(crate) fn dogfood_key(from: &str, to: &str) -> String {
    format!("{from}\u{2192}{to}")
}

/// The recorded live dogfood cells (empty until a human records one).
pub(crate) fn dogfood_cells_of(
    storage: &Storage,
) -> std::collections::BTreeMap<String, juggler_handover::validation::DogfoodCell> {
    storage
        .app_settings()
        .get(DOGFOOD_CELLS_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default()
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn validation_matrix_impl(state: &Arc<AppState>) -> AppResult<Vec<ValidationCellInfo>> {
    // Released before the rows are built: confirming a register PASS
    // may run Provider discovery (subprocesses).
    let recorded = {
        let storage = lock_storage_arc(state)?;
        dogfood_cells_of(&storage)
    };
    Ok(juggler_handover::validation::validation_matrix()
        .into_iter()
        .map(|row| {
            let merged = match recorded.get(&dogfood_key(&row.from, &row.to)) {
                Some(cell) => juggler_handover::validation::with_dogfood(row.clone(), cell.clone()),
                None => row.clone(),
            };
            let versions = installed_versions_for(state, &row.from, &row.to);
            let installed = |provider: &str| versions.get(provider).cloned();
            ValidationCellInfo {
                from: row.from.clone(),
                to: row.to.clone(),
                from_tool: juggler_handover::validation::side_tool(&row.from).to_owned(),
                to_tool: juggler_handover::validation::side_tool(&row.to).to_owned(),
                from_service: juggler_handover::validation::is_through_service(&row.from),
                to_service: juggler_handover::validation::is_through_service(&row.to),
                mechanism: match row.mechanism {
                    juggler_handover::validation::PairMechanism::Transplant => "transplant",
                    juggler_handover::validation::PairMechanism::Replay => "replay",
                    juggler_handover::validation::PairMechanism::PackOnly => "pack-only",
                }
                .to_owned(),
                researched_ceiling: row.researched_ceiling.as_str().to_owned(),
                advertised: merged.advertised_ceiling().as_str().to_owned(),
                badge: merged.badge_for(&installed, true),
                validated: merged.validated_for(&installed, true),
                mechanism_green: row.mechanism_green,
                ci_green: row.ci_green,
                dogfood: merged.dogfood.as_ref().map(|cell| DogfoodCellInfo {
                    at: cell.at.clone(),
                    project_shape: cell.project_shape.clone(),
                    verdict: match cell.verdict {
                        juggler_handover::validation::DogfoodVerdict::Pass { .. } => "pass",
                        juggler_handover::validation::DogfoodVerdict::Fail { .. } => "fail",
                    }
                    .to_owned(),
                    source_sha256: cell.source_sha256.clone(),
                    target_sha256: cell.target_sha256.clone(),
                    note: match &cell.verdict {
                        juggler_handover::validation::DogfoodVerdict::Pass { continuity_note } => {
                            continuity_note.clone()
                        }
                        juggler_handover::validation::DogfoodVerdict::Fail { reason } => {
                            reason.clone()
                        }
                    },
                }),
                status: merged.status_words(),
                notes: row.notes.clone(),
            }
        })
        .collect())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
#[allow(
    clippy::too_many_arguments,
    reason = "the recorded live test's columns, each its own"
)]
pub fn validation_dogfood_record_impl(
    state: &Arc<AppState>,
    from: &str,
    to: &str,
    verdict: &str,
    at: &str,
    project_shape: &str,
    source_sha256: &str,
    target_sha256: &str,
    note: &str,
) -> AppResult<()> {
    use juggler_handover::validation::{DogfoodCell, DogfoodVerdict};
    if juggler_handover::validation::pair_cell(from, to).is_none() {
        return Err(AppError::Io {
            context: "record a hand check",
            message: format!(
                "{from}→{to} is not a researched pair — cells are recorded, never invented"
            ),
        });
    }
    let verdict = match verdict {
        "pass" => DogfoodVerdict::Pass {
            continuity_note: note.to_owned(),
        },
        "fail" => DogfoodVerdict::Fail {
            reason: note.to_owned(),
        },
        other => {
            return Err(AppError::Io {
                context: "record a hand check",
                message: format!("Unknown verdict {other:?} — pass or fail"),
            });
        }
    };
    let cell = DogfoodCell {
        at: at.to_owned(),
        project_shape: project_shape.to_owned(),
        source_sha256: source_sha256.to_owned(),
        target_sha256: target_sha256.to_owned(),
        verdict,
    };
    let storage = lock_storage_arc(state)?;
    let mut cells = dogfood_cells_of(&storage);
    cells.insert(dogfood_key(from, to), cell);
    let blob = serde_json::to_string(&cells).map_err(|error| AppError::Io {
        context: "record a hand check",
        message: error.to_string(),
    })?;
    storage
        .app_settings()
        .set(DOGFOOD_CELLS_KEY, &blob)
        .map_err(AppError::Core)?;
    Ok(())
}
