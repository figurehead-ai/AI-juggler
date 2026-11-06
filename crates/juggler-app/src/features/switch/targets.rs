//! AJ-E209: the badged switch entry point: each target's verdict and rank.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_core::model::Timestamp;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::handover::source::handover_source;
use crate::features::providers::{installed_versions_for, warm_provider_versions};
use crate::features::quota::quota_display_inner;
use crate::features::ranking::dto::RankedRowInfo;
use crate::features::ranking::{SwitchRankContext, quota_tte, rank_candidates_for};
use crate::features::switch::dto::{HeldBack, SwitchTargetInfo};
use crate::features::switch::preflight::carry_lists_of;
use crate::features::validation::{dogfood_cells_of, dogfood_key};
use crate::state::AppState;

/// The §Q6 switch context for one source: its TTE (return-bonus
/// input), the returnable originals (accounts an OPEN excursion leads
/// back to), and the source's measured replay floor (its transcript
/// usage).
fn switch_rank_context_of(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    source_account: &juggler_core::model::Account,
) -> AppResult<SwitchRankContext> {
    let storage = lock_storage_arc(state)?;
    let now = Timestamp::now().millis();
    let source_tte_min = quota_tte(&quota_display_inner(&storage, source_account, now));
    let mut returnable = std::collections::BTreeSet::new();
    let graph = storage.switch_graph();
    for project in storage.projects().list()? {
        for work in graph.works_for_project(&project.id)? {
            let edges = graph.edges_for_work(&work.id)?;
            for edge in edges.iter().filter(|edge| {
                matches!(edge.state, juggler_core::model::EdgeState::Confirmed)
                    && edge.parent_edge_a.is_none()
            }) {
                let Some(to_epoch) = &edge.to_epoch else {
                    continue;
                };
                let returned = edges.iter().any(|other| {
                    matches!(other.state, juggler_core::model::EdgeState::Confirmed)
                        && &other.from_epoch == to_epoch
                });
                if !returned {
                    returnable.insert(edge.from_account.to_string());
                }
            }
        }
    }
    // What the usage pipeline already counted for it; the transcript is
    // read (once per size and time) only when it counted nothing (0.1.5
    // review: a session's page read its whole transcript twice on open).
    let counted = source
        .usage
        .tokens
        .input
        .saturating_add(source.usage.tokens.output);
    let source_replay_tokens = if counted > 0 {
        Some(counted)
    } else {
        source
            .transcript_ref
            .as_ref()
            .and_then(|reference| transcript_tokens_once(&reference.path))
    };
    Ok(SwitchRankContext {
        source_provider: source_account.harness.to_string(),
        source_side: crate::features::service_accounts::switch_side_in(&storage, source_account),
        source_tte_min,
        returnable_accounts: returnable,
        source_replay_tokens,
    })
}

/// A transcript's input + output tokens, read again only when its size or
/// time changed (the last few kept).
fn transcript_tokens_once(path: &std::path::Path) -> Option<u64> {
    type Seen =
        std::collections::HashMap<PathBuf, (u64, Option<std::time::SystemTime>, Option<u64>)>;
    static SEEN: std::sync::LazyLock<std::sync::Mutex<Seen>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(Seen::new()));
    let meta = std::fs::metadata(path).ok()?;
    let stamp = (meta.len(), meta.modified().ok());
    if let Ok(seen) = SEEN.lock()
        && let Some((len, modified, tokens)) = seen.get(path)
        && (*len, *modified) == stamp
    {
        return *tokens;
    }
    let tokens = juggler_adapters::transcript_tokens(path)
        .map(|tokens| tokens.input.saturating_add(tokens.output));
    if let Ok(mut seen) = SEEN.lock() {
        if seen.len() >= 64 {
            seen.clear();
        }
        seen.insert(path.to_path_buf(), (stamp.0, stamp.1, tokens));
    }
    tokens
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn switch_targets_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<Vec<SwitchTargetInfo>> {
    let (source, source_account) = handover_source(state, session_id)?;
    let context = switch_rank_context_of(state, &source, &source_account)?;
    let ranked = rank_candidates_for(
        state,
        source.project.as_ref().map(ToString::to_string).as_deref(),
        Some(&context),
    )?;
    let by_account: std::collections::BTreeMap<String, &RankedRowInfo> = ranked
        .iter()
        .map(|row| (row.account.clone(), row))
        .collect();
    // AJ-E808: any discovery the register needs runs before storage is
    // held (it spawns the Providers' version probes).
    warm_provider_versions(state);
    let storage = lock_storage_arc(state)?;
    let dogfood = dogfood_cells_of(&storage);
    let mut out = Vec::new();
    // In the ranking's order (the 0.2 walk, F1: they came in the order the
    // accounts were added).
    let mut accounts = storage.accounts().list()?;
    let place: std::collections::BTreeMap<&str, usize> = ranked
        .iter()
        .enumerate()
        .map(|(index, row)| (row.account.as_str(), index))
        .collect();
    accounts.sort_by_key(|account| {
        place
            .get(account.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    for account in accounts {
        let Some(row) = by_account.get(account.id.as_str()) else {
            continue;
        };
        // 0.3.4: the switch's own launch check, before it's offered.
        let blocker = crate::features::handover::access::launch_blocker(state, &account);
        let set_up = matches!(
            blocker,
            Some(juggler_adapter::readiness::NotReady::NotSetUp(_))
        );
        // 0.3.5: a first run the switch finishes never holds a target back.
        let first_run = blocker == Some(juggler_adapter::readiness::NotReady::FirstRun);
        let reason = row.filtered_reason.clone().or_else(|| {
            blocker
                .as_ref()
                .filter(|_| !first_run)
                .map(crate::features::handover::access::blocker_words)
        });
        for recipe in storage.profiles().list_by_account(&account.id)? {
            if recipe.id == source.profile {
                continue; // the source itself is not a destination
            }
            let cross = account.harness.as_str() != context.source_provider;
            let carry = carry_lists_of(&juggler_handover::fidelity::base_dispositions(
                context.source_provider.as_str(),
                account.harness.as_str(),
            ));
            // AJ-E210: the badge and validated state come from the
            // validation matrix merged with any recorded live cells
            // (a passing cell validates only real-CI rows — D-04). A
            // profile through another service is its own side (AJ-E1601).
            let side = crate::features::service_accounts::switch_side_in(&storage, &account);
            let cell = dogfood.get(&dogfood_key(context.source_side.as_str(), &side));
            let matrix_row = juggler_handover::validation::pair_cell(
                context.source_side.as_str(),
                &side,
            )
            .map(|row| match cell {
                Some(record) => juggler_handover::validation::with_dogfood(row, record.clone()),
                None => row,
            });
            let versions = installed_versions_for(
                state,
                context.source_provider.as_str(),
                account.harness.as_str(),
            );
            let installed = |provider: &str| versions.get(provider).cloned();
            out.push(SwitchTargetInfo {
                recipe_id: recipe.id.to_string(),
                provider: account.harness.to_string(),
                account_id: account.id.to_string(),
                account_label: account.label.clone(),
                badge: matrix_row.as_ref().map_or_else(
                    || "Handover notes · Beta".to_owned(),
                    |row| row.badge_for(&installed, true),
                ),
                validated: matrix_row
                    .as_ref()
                    .is_some_and(|row| row.validated_for(&installed, true)),
                cross_provider: cross,
                drops: carry.dropped.clone(),
                score: row.score,
                filtered_reason: reason.clone(),
                held_back: (row.filtered_reason.is_none() && set_up).then_some(HeldBack::SetUp),
                first_run,
                top_factors: row.top_factors.clone(),
                factors: row.factors.clone(),
                confidence: row.confidence.clone(),
                tte_min: row.tte_min,
                suggested: row.suggested && reason.is_none(),
            });
        }
    }
    Ok(out)
}
