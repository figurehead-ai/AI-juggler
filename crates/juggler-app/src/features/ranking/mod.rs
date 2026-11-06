//! AJ-E611: ranking the accounts a session could move to (the switch
//! candidates), with the reasons in words.

pub mod commands;
pub mod dto;

use juggler_core::model::SessionStatus;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::account::login_state_of;
use crate::features::pricing::pricing_catalog;
use crate::features::quota::{QuotaDisplayInfo, quota_display_inner};
use crate::features::ranking::dto::RankedRowInfo;
use crate::features::switch::preflight::project_account_counts;
use crate::features::validation::{dogfood_cells_of, dogfood_key};

/// The display's forecast TTE, when the pill can show one.
pub(crate) fn quota_tte(display: &QuotaDisplayInfo) -> Option<f64> {
    display.forecast.tte_min
}

/// AJ-E611 Req.1–3: the explainable destination ranking — hard filters
/// first (logged-out, wrong Provider, exhausted with no reset before
/// need-by), survivors scored with the ONE set of core weights.
/// Suggest-only: the rows sort and explain; the user clicks.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage failures.
#[allow(
    clippy::too_many_lines,
    reason = "one assembly pass — the weights live in core"
)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "its signature as it was in commands.rs; AJ-E2017 moved it unchanged"
)]
pub fn ranking_rows_inner(
    state: &std::sync::Arc<crate::state::AppState>,
    project: Option<String>,
) -> AppResult<Vec<RankedRowInfo>> {
    rank_candidates_for(state, project.as_deref(), None)
}

/// AJ-E209: the switch-side inputs for §Q6 — the source Provider
/// (pair fidelity + replay cost), the source's minutes-to-exhaustion
/// (return bonus), the returnable originals, and the source's
/// measured replay floor.
pub(crate) struct SwitchRankContext {
    /// The switching-away Provider id.
    pub(crate) source_provider: String,
    /// Its side of the pair (AJ-E1601: the tool, or the tool through
    /// another service).
    pub(crate) source_side: String,
    /// The SOURCE account's TTE minutes (None = unknown).
    pub(crate) source_tte_min: Option<f64>,
    /// Accounts that are returnable originals (open excursions).
    pub(crate) returnable_accounts: std::collections::BTreeSet<String>,
    /// The source conversation's measured replay floor (tokens).
    pub(crate) source_replay_tokens: Option<u64>,
}

/// AJ-E209: the priced replay cost for a cross-Provider pair
/// (same-Provider transplants pay nothing into costFit — the §Q6
/// extension prices REPLAY, per pass2/02 §7). Returns 0.0 when
/// unpriced (neutral, exactly like an unpriced launch).
fn replay_cost_for(
    context: &SwitchRankContext,
    account: &juggler_core::model::Account,
    catalog: &juggler_core::cost_ledger::PricingCatalog,
    now: i64,
) -> f64 {
    if context.source_provider.as_str() == account.harness.as_str() {
        return 0.0;
    }
    // Cross-Provider ⇒ replay today; price the measured floor at
    // the target's bundled rate.
    let rate = catalog
        .bundled
        .first()
        .map(|row| row.model.clone())
        .and_then(|model| {
            catalog.price_tokens(
                &model,
                now,
                juggler_core::model::TokenCounts {
                    input: 1_000_000,
                    ..juggler_core::model::TokenCounts::default()
                },
            )
        });
    juggler_handover::ranking::switch_cost_usd(true, context.source_replay_tokens, rate)
        .unwrap_or(0.0)
}

/// The §Q6 candidate builder shared by the launch ranking (no
/// switch context) and the switch entry point (E209: real pair
/// fidelity, priced switch cost, return likelihood).
#[allow(
    clippy::too_many_lines,
    reason = "one assembly pass over the candidates"
)]
pub(crate) fn rank_candidates_for(
    state: &std::sync::Arc<crate::state::AppState>,
    project: Option<&str>,
    switch: Option<&SwitchRankContext>,
) -> AppResult<Vec<RankedRowInfo>> {
    // 0.4: the folders a desktop app profile signs in for, read before
    // storage is locked (the book loads from storage).
    let through_app = crate::features::places::handoff::folders_signed_in_through_apps(state);
    let storage = lock_storage_arc(state)?;
    let accounts = storage.accounts().list()?;
    let now = juggler_core::model::Timestamp::now().millis();
    let sessions = storage.sessions().list(None).unwrap_or_default();
    let recipes: Vec<juggler_core::model::LaunchRecipe> = storage
        .accounts()
        .list()
        .unwrap_or_default()
        .iter()
        .flat_map(|account| {
            storage
                .profiles()
                .list_by_account(&account.id)
                .unwrap_or_default()
        })
        .collect();
    let project_sessions: Vec<String> = project
        .map(|project_id| {
            sessions
                .iter()
                .filter(|record| {
                    record
                        .project
                        .as_ref()
                        .is_some_and(|p| p.as_str() == project_id)
                })
                .map(|record| record.id.as_str().to_owned())
                .collect()
        })
        .unwrap_or_default();
    let project_counts = project.map(|project_id| {
        project_account_counts(&storage, &juggler_core::model::ProjectId::new(project_id))
    });

    let mut candidates: Vec<juggler_core::ranking::Candidate> = Vec::new();
    let mut tte_by_account: std::collections::BTreeMap<String, Option<f64>> =
        std::collections::BTreeMap::new();
    // AJ-E306: the per-account admission verdict (SPEC §5.2) — warn,
    // never block; None when headroom or burn is unknown.
    let mut admission_by_account: std::collections::BTreeMap<String, Option<String>> =
        std::collections::BTreeMap::new();
    let dogfood = dogfood_cells_of(&storage);
    for account in &accounts {
        let display = quota_display_inner(&storage, account, now);
        tte_by_account.insert(account.id.as_str().to_owned(), quota_tte(&display));
        let headroom_pct = [display.five_hour.as_ref(), display.weekly.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|window| window.best_estimate().map(|pct| 100.0 - pct))
            .reduce(f64::min);
        let confidence = match display
            .five_hour
            .as_ref()
            .and_then(|window| window.badge.as_deref())
            .or_else(|| {
                display
                    .weekly
                    .as_ref()
                    .and_then(|window| window.badge.as_deref())
            }) {
            Some("measured") => juggler_core::model::Confidence::Measured,
            Some("computed") => juggler_core::model::Confidence::Computed,
            _ => juggler_core::model::Confidence::Estimated,
        };
        // F17: the project's share, from the same count as the usual
        // account; none without a project (no folder picked).
        let affinity = project_counts.as_ref().map(|counts| {
            let total: u64 = counts.values().sum();
            let own = counts.get(&account.id).copied().unwrap_or(0);
            #[allow(clippy::cast_precision_loss, reason = "session counts, far below 2^52")]
            let share = if total == 0 {
                0.0
            } else {
                own as f64 / total as f64
            };
            share
        });
        let events = storage
            .usage()
            .events_for_account(account.id.as_str(), now - juggler_core::quota::WEEK_MS)
            .unwrap_or_default();
        let session_ids: Vec<&str> = project_sessions.iter().map(String::as_str).collect();
        let median_burn = juggler_core::quota_forecast::median_session_burn(&events, &session_ids);
        let headroom_tokens = [display.five_hour.as_ref(), display.weekly.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|window| window.headroom_tokens)
            .reduce(f64::min);
        admission_by_account.insert(
            account.id.as_str().to_owned(),
            juggler_core::quota_forecast::admission_verdict(median_burn, headroom_tokens).map(
                |verdict| {
                    match verdict {
                        juggler_core::quota_forecast::AdmissionVerdict::Ok => "ok",
                        juggler_core::quota_forecast::AdmissionVerdict::Tight => "tight",
                        juggler_core::quota_forecast::AdmissionVerdict::No => "no",
                    }
                    .to_owned()
                },
            ),
        );
        let catalog = pricing_catalog(&storage);
        let projected_usd = median_burn.and_then(|tokens| {
            let counts = juggler_core::model::TokenCounts {
                input: tokens,
                ..juggler_core::model::TokenCounts::default()
            };
            catalog
                .bundled
                .first()
                .map(|row| row.model.clone())
                .and_then(|model| catalog.price_tokens(&model, now, counts))
        });
        let active_sessions = sessions
            .iter()
            .filter(|record| {
                record.status == SessionStatus::Running
                    && recipes.iter().any(|recipe| {
                        recipe.id == record.profile
                            && recipe.account.as_str() == account.id.as_str()
                    })
            })
            .count();
        let active_sessions = u32::try_from(active_sessions).unwrap_or(u32::MAX);
        // AJ-E209/E210: with a switch context the pair's ADVERTISED
        // fidelity (the E203 ceiling, lowered one level per FAILED
        // live cell — D-04) rides the 0.15 slot, a replay pair pays
        // its priced switch cost into costFit, and returnable
        // originals carry the source's TTE for the return bonus.
        // AJ-E1601: a profile through another service is its own side of
        // a pair, and is ready when its key is in the Keychain.
        let side = crate::features::service_accounts::switch_side_in(&storage, account);
        let (fidelity, switch_cost) = match switch {
            Some(context) => (
                Some(juggler_handover::validation::advertised_ceiling_number(
                    &context.source_side,
                    &side,
                    dogfood.get(&dogfood_key(&context.source_side, &side)),
                )),
                replay_cost_for(context, account, &catalog, now),
            ),
            None => (None, 0.0), // launch-neutral, documented (§Q6)
        };
        let projected_usd = projected_usd.map(|usd| usd + switch_cost);
        let ready = crate::features::service_accounts::ready_in(state, &storage, account);
        candidates.push(juggler_core::ranking::Candidate {
            account: account.id.as_str().to_owned(),
            logged_out: ready.map_or_else(
                || {
                    // 0.4: a Claude app profile used in its Code tab signs
                    // in through the app; a switch continues there.
                    login_state_of(&state.registry, account)
                        == juggler_adapters::LoginState::LoggedOut
                        && !through_app.contains(account.id.as_str())
                },
                |ready| !ready,
            ),
            wrong_provider: false, // request-restriction surface rides E210's pair gating
            exhausted_no_reset: display.exhausted_no_reset(),
            // 0.3.2: a used-up window its credits carry on past.
            credits_cover: display.using_credits().is_some(),
            headroom_pct,
            headroom_confidence: confidence,
            affinity,
            fidelity,
            projected_usd,
            active_sessions,
            soft_cap: 4,
            returnable: switch
                .is_some_and(|context| context.returnable_accounts.contains(account.id.as_str())),
            source_resets_min: switch.and_then(|context| context.source_tte_min),
        });
    }

    let ranked = juggler_core::ranking::rank(&candidates);
    Ok(ranked
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let tte = tte_by_account.get(&row.account).copied().flatten();
            let account_label = accounts
                .iter()
                .find(|account| account.id.as_str() == row.account)
                .map_or_else(|| row.account.clone(), |account| account.label.clone());
            let admission = admission_by_account.get(&row.account).cloned().flatten();
            RankedRowInfo {
                account: row.account,
                account_label,
                score: row.score,
                filtered_reason: row.filtered_reason,
                top_factors: row.top_factors,
                factors: row.factors,
                headroom_unknown: row.headroom_unknown,
                confidence: match row.confidence {
                    juggler_core::model::Confidence::Measured => "Measured".to_owned(),
                    juggler_core::model::Confidence::Computed => "Computed".to_owned(),
                    juggler_core::model::Confidence::Estimated => "Estimate".to_owned(),
                },
                tte_min: tte,
                suggested: index == 0 && row.score.is_some(),
                admission,
            }
        })
        .collect())
}
