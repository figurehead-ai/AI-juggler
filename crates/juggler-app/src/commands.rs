//! Tauri commands exposed to the webview, with TypeScript types generated
//! from the Rust definitions (`ts-rs`) — single source of truth for both
//! sides of the bridge.
//!
//! Commands take `tauri::State` by value per the framework's convention.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use juggler_adapter::LaunchMode;
use juggler_core::CoreError;
use juggler_core::model::{
    Account, AccountId, MachineId, Profile, ProfileId, SessionId, SessionRecord, SessionStatus,
    SettingsOverlay, Timestamp,
};
use juggler_core::storage::Storage;
use juggler_supervisor::{ExitKind, LifecycleEvent};

use crate::error::{AppError, AppResult};
use crate::focus;
use crate::state::AppState;

/// Static application information served to the UI at startup.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppInfo {
    /// Application version from the crate manifest.
    pub version: String,
    /// Machine-friendly app name.
    pub name: String,
}

impl AppInfo {
    /// Builds the info from compile-time crate metadata.
    #[must_use]
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: "AI Juggler".to_string(),
        }
    }
}

/// AJ-E609: the account's cost rollups (session/account/project/day
/// dimensions share one shape — the UI matrix and the CSV export read
/// the SAME rows, so reconciliation is exact) plus the dual
/// subscription view and the pricing-staleness flag.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct CostRollupsInfo {
    /// Day-bucketed rollup rows (`key` = session id in the session
    /// dimension).
    pub rows: Vec<juggler_core::cost_ledger::CostRollup>,
    /// "≈$X at API prices · you pay flat" — ratio only when a fee
    /// exists (AJ-E610's form owns entry).
    pub subscription: juggler_core::cost_ledger::SubscriptionView,
    /// The bundled pricing table is >30 d old (inline warn).
    pub pricing_stale: bool,
    /// The bundled table's `last synced`, epoch ms.
    pub pricing_last_synced_ms: Option<i64>,
    /// The grand-total row (ALL bucket) for the scope.
    pub total_usd: Option<f64>,
    /// The scope the query ran over (account id or `ALL`).
    pub scope: String,
}

/// The catalog as loaded: bundled seed + user overrides + staleness.
#[must_use]
fn pricing_catalog(
    storage: &juggler_core::storage::Storage,
) -> juggler_core::cost_ledger::PricingCatalog {
    use juggler_core::cost_ledger::PricingCatalog;
    let mut catalog = PricingCatalog {
        bundled: juggler_core::cost_ledger::load_seed_rows(),
        overrides: storage.usage().pricing_overrides().unwrap_or_default(),
        last_synced_ms: None,
    };
    catalog.last_synced_ms = storage
        .app_settings()
        .get("pricing_synced_at")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok());
    catalog
}

/// The scoped events (one account, or every account with usage) plus
/// the account's optional user-entered flat fee.
fn scoped_cost_events(
    storage: &juggler_core::storage::Storage,
    account: Option<&str>,
) -> (String, Vec<juggler_core::quota::UsageEvent>, Option<f64>) {
    let account = account.map_or_else(|| "ALL".to_owned(), str::to_owned);
    let usage = storage.usage();
    let mut events = Vec::new();
    if account == "ALL" {
        for account_id in usage.usage_accounts().unwrap_or_default() {
            events.extend(usage.events_for_account(&account_id, 0).unwrap_or_default());
        }
    } else {
        events = usage.events_for_account(&account, 0).unwrap_or_default();
    }
    let fee = storage
        .app_settings()
        .get(&format!("subscription_fee:{account}"))
        .ok()
        .flatten()
        .and_then(|value| value.parse::<f64>().ok());
    (account, events, fee)
}

/// AJ-E609 Req.3: the per-session cost matrix (session × model × day).
#[tauri::command]
#[must_use]
pub fn cost_rollups(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: Option<String>,
) -> CostRollupsInfo {
    let Ok(storage) = state.storage.lock() else {
        return CostRollupsInfo::default_cost();
    };
    let catalog = pricing_catalog(&storage);
    let (scope, events, fee) = scoped_cost_events(&storage, account.as_deref());
    let rows = juggler_core::cost_ledger::rollup(
        &events,
        &catalog,
        juggler_core::cost_ledger::RollupDimension::Session,
    );
    let all = juggler_core::cost_ledger::rollup(
        &events,
        &catalog,
        juggler_core::cost_ledger::RollupDimension::All,
    );
    let total_usd = all.first().and_then(|row| row.usd.usd);
    let view = juggler_core::cost_ledger::subscription_view(total_usd, fee);
    let now = juggler_core::model::Timestamp::now().millis();
    CostRollupsInfo {
        rows,
        subscription: view,
        pricing_stale: catalog.is_stale(now),
        pricing_last_synced_ms: catalog.last_synced_ms,
        total_usd,
        scope,
    }
}

impl CostRollupsInfo {
    /// The poison/error fallback: an empty, honest matrix.
    #[must_use]
    fn default_cost() -> Self {
        Self {
            rows: Vec::new(),
            subscription: juggler_core::cost_ledger::subscription_view(None, None),
            pricing_stale: false,
            pricing_last_synced_ms: None,
            total_usd: None,
            scope: String::new(),
        }
    }
}

/// AJ-E609 Req.6: streams the CSV to a temp file (chunked month
/// windows — never a full-table read) and returns its path. The rows
/// are the SAME rollup shape the UI displays.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage or I/O failure.
#[tauri::command]
pub fn cost_csv_export(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: Option<String>,
) -> AppResult<String> {
    let storage = lock_storage_arc(&state)?;
    let catalog = pricing_catalog(&storage);
    let usage = storage.usage();
    let accounts: Vec<String> = match account.as_deref() {
        Some(named) => vec![named.to_owned()],
        None => usage.usage_accounts()?,
    };
    let month_ms = 30 * 24 * 60 * 60 * 1000_i64;
    let now = juggler_core::model::Timestamp::now().millis();
    let path = std::env::temp_dir().join(format!("ai-juggler-costs-{now}.csv"));
    let mut file =
        std::fs::File::create(&path).map_err(|error| AppError::io("create cost csv", &error))?;
    // Header + chunked month windows, merged into the day map as we
    // go (bounded memory: the map is day-bucketed, the events chunk
    // never accumulates).
    let mut merged: Vec<juggler_core::cost_ledger::CostRollup> = Vec::new();
    for account_id in &accounts {
        let mut hi = now;
        loop {
            let lo = hi.saturating_sub(month_ms);
            let chunk = usage.events_between(account_id, lo, hi)?;
            if !chunk.is_empty() {
                let mut rows = juggler_core::cost_ledger::rollup(
                    &chunk,
                    &catalog,
                    juggler_core::cost_ledger::RollupDimension::Session,
                );
                merged.append(&mut rows);
            }
            if lo <= 0 {
                break;
            }
            hi = lo;
        }
    }
    juggler_core::cost_ledger::write_csv(&merged, &mut file)
        .map_err(|error| AppError::io("write cost csv", &error))?;
    Ok(path.to_string_lossy().to_string())
}

/// AJ-E609 Req.1: set one user pricing override (the editor shows
/// `source_url` + effective date before save — AJ-E610's surface).
///
/// # Errors
///
/// Returns the typed [`AppError`] on validation or storage failure.
#[tauri::command]
pub fn pricing_override_set(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    row: juggler_core::cost_ledger::LedgerPriceRow,
) -> AppResult<()> {
    if row.model.trim().is_empty() || row.source_url.trim().is_empty() {
        return Err(AppError::Validation {
            subject: "pricing override",
            message: "model and source_url are required (auditable pricing law)".into(),
        });
    }
    let storage = lock_storage_arc(&state)?;
    storage
        .usage()
        .pricing_override_upsert(&row)
        .map_err(AppError::from)?;
    Ok(())
}

/// AJ-E609: clear one override (history reprices back to bundled).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub fn pricing_override_clear(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    model: String,
    effective_from_ms: i64,
) -> AppResult<()> {
    let storage = lock_storage_arc(&state)?;
    storage
        .usage()
        .pricing_override_delete(&model, effective_from_ms)
        .map_err(AppError::from)?;
    Ok(())
}

/// AJ-E609: catalog metadata for the pricing editor (staleness, last
/// synced, the bundled + override rows).
#[tauri::command]
#[must_use]
pub fn pricing_catalog_info(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> juggler_core::cost_ledger::PricingCatalog {
    match state.storage.lock() {
        Ok(storage) => pricing_catalog(&storage),
        Err(_) => juggler_core::cost_ledger::PricingCatalog::default(),
    }
}

/// AJ-E610: one window clock's rendered state — the typed shape every
/// quota surface reads (rail chips, Inspector `Usage` tab, `NewSession`
/// rows). `percentage: None` in Unknown/Learning — a missing number is
/// the honest state, never 0%.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct WindowDisplay {
    /// The §Q8 machine state (`unknown|learning|tracking|stale|exhausted`).
    pub state: String,
    /// Percent used, ONLY when the state carries one.
    pub percentage: Option<f64>,
    /// The badge (`Measured|Computed|Estimate`), adjacent on screen.
    pub badge: Option<String>,
    /// Staleness note ("last measured Nh ago") when Measured went quiet.
    pub stale_note_ms: Option<i64>,
    /// Vendor reset instant, epoch ms, when known.
    pub resets_at_ms: Option<i64>,
    /// Absolute burn inside the window (the honest floor).
    pub burned_tokens: u64,
    /// Burn in vendor credits, when the provider bills credits.
    pub burned_credits: Option<f64>,
    /// Headroom in tokens (cap − burned), ONLY when the cap is
    /// known — the admission verdict's input (AJ-E306); unknown
    /// stays None, never a fabricated number.
    pub headroom_tokens: Option<f64>,
}

/// AJ-E610: the subscription clock's rendered state — a calendar-style
/// date line, never merged with the window clock.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct RenewalDisplay {
    /// `none|set|approaching|overdue`.
    pub state: String,
    /// The exact copy (`Renews 3 Oct` / `Renewal: unknown — add date`).
    pub label: String,
    /// Entered by the user (D-12 provenance).
    pub entered_by_user: bool,
}

/// AJ-E610: the forecast pill (deep/10 §1 states; data only — the
/// pill component renders them).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct ForecastDisplay {
    /// `learning|wont-exhaust|exhausts|no-headroom`.
    pub state: String,
    /// Point TTE minutes (Measured renders the point).
    pub tte_min: Option<f64>,
    /// Cone lower bound (Estimated renders the range).
    pub range_min: Option<f64>,
    /// Cone upper bound.
    pub range_max: Option<f64>,
    /// Inherited confidence badge.
    pub badge: String,
}

/// AJ-E610: the two-clock display for one account — both windows (a
/// provider/plan may omit one, with reason), the separate subscription
/// clock, the forecast, and any provider-level unavailability.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct QuotaDisplayInfo {
    /// Account id.
    pub account: String,
    /// The 5 h window, when the plan has one.
    pub five_hour: Option<WindowDisplay>,
    /// Why the 5 h ring is omitted (e.g. Codex Pro has none).
    pub five_hour_omitted_reason: Option<String>,
    /// The weekly window, when the plan has one.
    pub weekly: Option<WindowDisplay>,
    /// Why the weekly ring is omitted.
    pub weekly_omitted_reason: Option<String>,
    /// The subscription clock (D-12).
    pub renewal: RenewalDisplay,
    /// The burn forecast.
    pub forecast: ForecastDisplay,
    /// Provider-level unavailability (e.g. Gemini pre-verification).
    pub unavailable_reason: Option<String>,
}

/// Evaluates one account's two-clock display from the engine (E606
/// evaluate + E608 forecast + E411 renewal + E607 staleness).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn quota_display_inner(
    storage: &juggler_core::storage::Storage,
    account: &juggler_core::model::Account,
    now_ms: i64,
) -> QuotaDisplayInfo {
    use juggler_core::quota::{
        self, AccountEvaluation, QuotaCap, QuotaState, SubscriptionClock, WindowKind,
    };
    use juggler_core::quota_forecast::{self, ForecastState, RateUnits};

    let account_id = account.id.as_str();
    let settings = storage.app_settings();
    let usage = storage.usage();
    let events = usage
        .events_for_account(account_id, now_ms - quota::WEEK_MS)
        .unwrap_or_default();
    let anchor = settings
        .get(&juggler_core::storage::anchor_settings_key(account_id))
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<i64>().ok());
    let samples_five_hour = usage
        .samples_for_account(account_id, juggler_core::quota::WindowKind::FiveHour)
        .unwrap_or_default();
    let samples_weekly = usage
        .samples_for_account(
            account_id,
            juggler_core::quota::WindowKind::Weekly { anchor_ms: anchor },
        )
        .unwrap_or_default();
    let cap = |kind: WindowKind| -> QuotaCap {
        settings
            .get(&juggler_core::storage::cap_settings_key(account_id, kind))
            .ok()
            .flatten()
            .map(|raw| QuotaCap::decode(&raw))
            .unwrap_or_default()
    };
    let clock = SubscriptionClock {
        renewal_date: account.renewal_date.clone(),
        entered_by_user: account.renewal_date.is_some(),
        from_vendor_feed: false,
    };
    let today = epoch_today(now_ms);
    let evaluation = quota::evaluate_account(AccountEvaluation {
        events: &events,
        samples_five_hour: &samples_five_hour,
        samples_weekly: &samples_weekly,
        account: account_id,
        cap_five_hour: cap(WindowKind::FiveHour),
        cap_weekly: cap(WindowKind::Weekly { anchor_ms: anchor }),
        weekly_anchor_ms: anchor,
        subscription: &clock,
        now_ms,
        today,
    });

    // Staleness (Q-1): the freshest Measured sample's age.
    let last_measured = samples_five_hour
        .iter()
        .chain(samples_weekly.iter())
        .filter(|sample| sample.confidence == juggler_core::model::Confidence::Measured)
        .map(|sample| sample.ts)
        .max();
    let stale_note_ms =
        last_measured.filter(|ts| now_ms.saturating_sub(*ts) > quota::STALE_AFTER_MS);

    let window_display = |reading: &quota::WindowReading, cap: QuotaCap| WindowDisplay {
        state: match reading.state {
            QuotaState::Unknown => "unknown".to_owned(),
            QuotaState::Learning => "learning".to_owned(),
            QuotaState::Tracking => "tracking".to_owned(),
            QuotaState::Stale => "stale".to_owned(),
            QuotaState::Exhausted => "exhausted".to_owned(),
        },
        percentage: reading.percentage,
        badge: reading
            .percentage
            .map(|_| confidence_name(reading.confidence).to_owned()),
        stale_note_ms,
        resets_at_ms: reading.resets_at,
        burned_tokens: reading
            .burn
            .tokens
            .input
            .saturating_add(reading.burn.tokens.output)
            .saturating_add(reading.burn.tokens.cache_read)
            .saturating_add(reading.burn.tokens.cache_write)
            .saturating_add(reading.burn.tokens.reasoning),
        burned_credits: (reading.burn.credits > 0.0).then_some(reading.burn.credits),
        headroom_tokens: cap.tokens.map(|tokens| {
            #[allow(clippy::cast_precision_loss)]
            let room = tokens as f64;
            #[allow(clippy::cast_precision_loss)]
            let burned = reading.burn.tokens.input as f64
                + reading.burn.tokens.output as f64
                + reading.burn.tokens.cache_read as f64
                + reading.burn.tokens.cache_write as f64
                + reading.burn.tokens.reasoning as f64;
            (room - burned).max(0.0)
        }),
    };

    // The forecast runs on the unit the provider bills (ZCode credits).
    let units = if account.harness.as_str() == "zcode" {
        RateUnits::Credits
    } else {
        RateUnits::Tokens
    };
    let forecast_cap = cap(WindowKind::FiveHour);
    let resets_at = evaluation.five_hour.resets_at;
    let fc = quota_forecast::forecast(
        &events,
        account_id,
        WindowKind::FiveHour,
        forecast_cap,
        units,
        resets_at,
        now_ms,
    );
    let forecast = ForecastDisplay {
        state: match fc.state {
            ForecastState::Learning => "learning".to_owned(),
            ForecastState::WontExhaust => "wont-exhaust".to_owned(),
            ForecastState::Exhausts { .. } => "exhausts".to_owned(),
            ForecastState::NoHeadroom => "no-headroom".to_owned(),
        },
        tte_min: match fc.state {
            ForecastState::Exhausts { tte_min, .. } => Some(tte_min),
            _ => None,
        },
        range_min: match fc.state {
            ForecastState::Exhausts { range_low_min, .. } => Some(range_low_min),
            _ => None,
        },
        range_max: match fc.state {
            ForecastState::Exhausts { range_high_min, .. } => Some(range_high_min),
            _ => None,
        },
        badge: confidence_name(fc.confidence).to_owned(),
    };

    // Renewal (D-12 via E411's label machine; approaching threshold
    // user-tunable 1–30 days, default 7).
    let warn_days: i64 = settings
        .get("renewal_warn_days")
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(7)
        .clamp(1, 30);
    let renewal = match clock.evaluate(today) {
        juggler_core::quota::RenewalState::Unknown => RenewalDisplay {
            state: "none".to_owned(),
            label: "Renewal: unknown — add date".to_owned(),
            entered_by_user: false,
        },
        juggler_core::quota::RenewalState::RenewIn { days, .. } => {
            let (state, label) = if days < 0 {
                ("overdue", "Renewal date passed".to_owned())
            } else if days <= warn_days {
                (
                    "approaching",
                    format!("Renews in {days} days — check your plan"),
                )
            } else {
                (
                    "set",
                    format!(
                        "Renews {}",
                        account.renewal_date.clone().unwrap_or_default()
                    ),
                )
            };
            RenewalDisplay {
                state: state.to_owned(),
                label,
                entered_by_user: true,
            }
        }
    };

    // Provider/plan window shapes (Req.5): Pro renders weekly; plans
    // without a 5 h window omit that ring WITH reason, never empty.
    let provider = account.harness.as_str();
    let plan = account.plan.as_ref().map(ToString::to_string);
    let mut unavailable_reason = None;
    let (five_hour, five_hour_reason, weekly, weekly_reason) = match provider {
        "gemini" | "antigravity" => {
            unavailable_reason = Some("No verified quota source for this Provider yet".to_owned());
            (None, None, None, None)
        }
        // Codex Pro has no 5 h limit (AJ-E603): omit that ring with
        // its reason; every other shape renders both windows.
        "codex" if plan.as_deref() == Some("pro") => (
            None,
            Some("Pro plans have no five-hour limit — weekly only".to_owned()),
            Some(window_display(
                &evaluation.weekly,
                cap(WindowKind::Weekly { anchor_ms: anchor }),
            )),
            None,
        ),
        _ => (
            Some(window_display(
                &evaluation.five_hour,
                cap(WindowKind::FiveHour),
            )),
            None,
            Some(window_display(
                &evaluation.weekly,
                cap(WindowKind::Weekly { anchor_ms: anchor }),
            )),
            None,
        ),
    };
    QuotaDisplayInfo {
        account: account_id.to_owned(),
        five_hour,
        five_hour_omitted_reason: five_hour_reason,
        weekly,
        weekly_omitted_reason: weekly_reason,
        renewal,
        forecast,
        unavailable_reason,
    }
}

/// AJ-E611: one ranked (or filtered) row on the wire — greyed rows
/// carry their filter reason; survivors carry the score + their
/// explanation (top-2 factors in words; the full five in `factors`).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct RankedRowInfo {
    /// Account id.
    pub account: String,
    /// The weighted score, when the account survived the hard filters.
    pub score: Option<f64>,
    /// The hard filter's reason (greyed row copy).
    pub filtered_reason: Option<String>,
    /// Top-2 contributing factors in words.
    pub top_factors: Vec<String>,
    /// All five named inputs (the "Why this order?" expander).
    pub factors: Vec<(String, f64)>,
    /// Unknown headroom ranked without quota.
    pub headroom_unknown: bool,
    /// The confidence badge copy.
    pub confidence: String,
    /// TTE minutes for the pill, when a forecast exists.
    pub tte_min: Option<f64>,
    /// True for the top row (badged "Suggested").
    pub suggested: bool,
    /// The launch admission verdict (`ok|tight|no`, SPEC §5.2) for
    /// this account on the requested project — None when headroom or
    /// burn is unknown (rendered as "headroom unknown", never a
    /// guess). Warns; never blocks (AJ-E306 Req.7).
    pub admission: Option<String>,
}

/// The display's forecast TTE, when the pill can show one.
fn quota_tte(display: &QuotaDisplayInfo) -> Option<f64> {
    display.forecast.tte_min
}

/// AJ-E611 Req.1–3: the explainable destination ranking — hard filters
/// first (logged-out, wrong Provider, exhausted with no reset before
/// need-by), survivors scored with the ONE set of core weights.
/// Suggest-only: the rows sort and explain; the user clicks.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
#[allow(clippy::too_many_lines)] // one assembly pass — the weights live in core
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
struct SwitchRankContext {
    /// The switching-away Provider id.
    source_provider: String,
    /// The SOURCE account's TTE minutes (None = unknown).
    source_tte_min: Option<f64>,
    /// Accounts that are returnable originals (open excursions).
    returnable_accounts: std::collections::BTreeSet<String>,
    /// The source conversation's measured replay floor (tokens).
    source_replay_tokens: Option<u64>,
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
#[allow(clippy::too_many_lines)]
fn rank_candidates_for(
    state: &std::sync::Arc<crate::state::AppState>,
    project: Option<&str>,
    switch: Option<&SwitchRankContext>,
) -> AppResult<Vec<RankedRowInfo>> {
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
    let total_project_sessions = project_sessions.len().max(1);

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
            .filter_map(|window| window.percentage.map(|pct| 100.0 - pct))
            .reduce(f64::min);
        let exhausted_no_reset = [display.five_hour.as_ref(), display.weekly.as_ref()]
            .into_iter()
            .flatten()
            .any(|window| window.state == "exhausted" && window.resets_at_ms.is_none());
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
        let affinity = if project.is_some() {
            let own = sessions
                .iter()
                .filter(|record| {
                    record
                        .project
                        .as_ref()
                        .is_some_and(|p| p.as_str() == project.unwrap_or_default())
                        && recipes.iter().any(|recipe| {
                            recipe.id == record.profile
                                && recipe.account.as_str() == account.id.as_str()
                        })
                })
                .count();
            f64::from(u32::try_from(own).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_project_sessions).unwrap_or(u32::MAX))
        } else {
            0.5
        };
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
        let (fidelity, switch_cost) = match switch {
            Some(context) => (
                juggler_handover::validation::advertised_ceiling_number(
                    &context.source_provider,
                    account.harness.as_str(),
                    dogfood.get(&dogfood_key(
                        context.source_provider.as_str(),
                        account.harness.as_str(),
                    )),
                ),
                replay_cost_for(context, account, &catalog, now),
            ),
            None => (0.5, 0.0), // launch-neutral, documented (§Q6)
        };
        let projected_usd = projected_usd.map(|usd| usd + switch_cost);
        candidates.push(juggler_core::ranking::Candidate {
            account: account.id.as_str().to_owned(),
            logged_out: login_state_of(account) == juggler_adapters::LoginState::LoggedOut,
            wrong_provider: false, // request-restriction surface rides E210's pair gating
            exhausted_no_reset,
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
            let admission = admission_by_account.get(&row.account).cloned().flatten();
            RankedRowInfo {
                account: row.account,
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

/// AJ-E611: the explainable ranking command (thin wrapper over
/// [`ranking_rows_inner`] — suggest-only, the user clicks).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
#[tauri::command]
pub fn ranking_rows(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    project: Option<String>,
) -> AppResult<Vec<RankedRowInfo>> {
    ranking_rows_inner(&state, project)
}

/// AJ-E611: one emitted alert on the wire (the notification region's
/// payload — polite live-region semantics client-side).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct AlertEventInfo {
    /// `tray-amber|actionable|exhausted|reset-reached`.
    pub kind: String,
    /// Account the alert belongs to.
    pub account: String,
    /// The copy (badge-carrying: Estimated reads differently).
    pub text: String,
    /// Emitted at, epoch ms.
    pub at_ms: i64,
}

/// The per-(account, window) persisted FSM states (`alert_fsms` KV).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct AlertFsmState {
    #[serde(default)]
    five_hour: juggler_core::alerts::AlertFsm,
    #[serde(default)]
    weekly: juggler_core::alerts::AlertFsm,
}

const ALERT_FSMS_KEY: &str = "alert_fsms";
const ALERT_DIGEST_KEY: &str = "alert_digest";
/// AJ-E206 §8 telemetry: local-only door counters (never networked) —
/// `switch_oneclick` vs `switch_wizard{steps_shown}` proves the fast
/// path is earned, not assumed.
const SWITCH_DOOR_COUNTERS_KEY: &str = "switch_door_counters";
const ALERT_MUTES_KEY: &str = "alert_mutes";

/// AJ-E611: one account's two window FSMs + its renewal-approaching
/// digest line (Req.5 — the subscription clock NEVER steps a window
/// machine). Pure with respect to Provider surfaces; mutates only the
/// persisted machine entry and the digest buffer.
fn step_account_alerts(
    storage: &juggler_core::storage::Storage,
    account: &juggler_core::model::Account,
    now: i64,
    entry: &mut AlertFsmState,
    digest: &mut juggler_core::alerts::Digest,
) -> Vec<AlertEventInfo> {
    use juggler_core::alerts::{AlertEmission, WindowObservation};

    let mut out = Vec::new();
    let id = account.id.as_str().to_owned();
    let display = quota_display_inner(storage, account, now);
    let confidence = match display.forecast.badge.as_str() {
        "measured" => juggler_core::model::Confidence::Measured,
        "computed" => juggler_core::model::Confidence::Computed,
        _ => juggler_core::model::Confidence::Estimated,
    };
    // One buffered line per distinct copy — the label changes as the
    // date nears, which is the natural dedupe key.
    if display.renewal.state == "approaching"
        && !digest
            .entries
            .iter()
            .any(|line| line.key == id && line.text == display.renewal.label)
    {
        *digest = juggler_core::alerts::digest_add(digest, &id, &display.renewal.label);
    }
    for (slot, window) in [
        ("five_hour", &display.five_hour),
        ("weekly", &display.weekly),
    ] {
        let Some(window) = window else { continue };
        let observation = WindowObservation {
            tte_min: display.forecast.tte_min,
            used_pct: window.percentage,
            confidence,
        };
        let machine = if slot == "five_hour" {
            &entry.five_hour
        } else {
            &entry.weekly
        };
        let (next, emission) = juggler_core::alerts::step(machine, observation, now);
        if slot == "five_hour" {
            entry.five_hour = next;
        } else {
            entry.weekly = next;
        }
        let event = match emission {
            AlertEmission::TrayAmber { .. } => Some((
                "tray-amber",
                format!(
                    "{} ≈{} min left ({}), {} window",
                    short_label(&account.label),
                    display
                        .forecast
                        .tte_min
                        .map_or_else(|| "—".into(), |tte| format!("{tte:.0}")),
                    badge_word(confidence),
                    slot
                ),
            )),
            AlertEmission::Actionable { .. } => Some((
                "actionable",
                format!(
                    "{} at {}{} used ({}), {} window — hand over or queue at reset",
                    short_label(&account.label),
                    window
                        .percentage
                        .map_or_else(String::new, |pct| format!("{pct:.0}%")),
                    if window.percentage.is_some() {
                        ""
                    } else {
                        "critical TTE"
                    },
                    badge_word(confidence),
                    slot
                ),
            )),
            AlertEmission::ExhaustedNotification => Some((
                "exhausted",
                format!("{} {} window exhausted", short_label(&account.label), slot),
            )),
            AlertEmission::None | AlertEmission::ResetReached => None,
        };
        if let Some((kind, text)) = event {
            out.push(AlertEventInfo {
                kind: kind.to_owned(),
                account: id.clone(),
                text,
                at_ms: now,
            });
        }
    }
    out
}

/// AJ-E611 Req.4: one alert evaluation pass — steps every account's
/// two window FSMs from the engine's current readings, emits AT MOST
/// the §Q5 notifications, and persists the machines. Non-actionable
/// crossings (Req.5: renewal `approaching`) batch into the hourly
/// digest; quiet hours defer without dropping. Passive by
/// construction: nothing here launches, signals, or writes any
/// Provider surface (the guardrail test pins it). `local_hour` comes
/// from the polling UI (the webview knows wall-clock local time; the
/// backend carries no tz database) — `None` defers the flush.
#[must_use]
pub fn alerts_tick_inner(
    state: &std::sync::Arc<crate::state::AppState>,
    local_hour: Option<u32>,
) -> Vec<AlertEventInfo> {
    let mut out = Vec::new();
    let now = juggler_core::model::Timestamp::now().millis();
    let Ok(storage) = state.storage.lock() else {
        return out;
    };
    let accounts = storage.accounts().list().unwrap_or_default();
    let mut machines: std::collections::BTreeMap<String, AlertFsmState> = storage
        .app_settings()
        .get(ALERT_FSMS_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    let mut digest: juggler_core::alerts::Digest = storage
        .app_settings()
        .get(ALERT_DIGEST_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();

    for account in &accounts {
        let entry = machines.entry(account.id.as_str().to_owned()).or_default();
        out.extend(step_account_alerts(
            &storage,
            account,
            now,
            entry,
            &mut digest,
        ));
    }
    if let Ok(blob) = serde_json::to_string(&machines) {
        let _ = storage.app_settings().set(ALERT_FSMS_KEY, &blob);
    }

    out.extend(queue_and_digest_events(&storage, now, local_hour, digest));
    out
}

/// AJ-E611 Req.4/6 tail of the alert pass: reset-queue notifications
/// (due items notify ONCE — `notified`; broken lineage goes `stale`,
/// explained) plus the hourly digest flush (muted keys drop; quiet
/// hours — a missing local hour counts as quiet — defer without
/// dropping). The next digest state is persisted here; the caller
/// receives only the events.
fn queue_and_digest_events(
    storage: &juggler_core::storage::Storage,
    now: i64,
    local_hour: Option<u32>,
    digest: juggler_core::alerts::Digest,
) -> Vec<AlertEventInfo> {
    let mut out = Vec::new();
    let session_ids: Vec<String> = storage
        .sessions()
        .list(None)
        .unwrap_or_default()
        .iter()
        .map(|record| record.id.as_str().to_owned())
        .collect();
    let usage = storage.usage();
    for (id, session, account, _resume_at) in usage.pending_resumes(now).unwrap_or_default() {
        if !session_ids.contains(&session) {
            let _ = usage.set_reset_state(&id, "stale");
            continue;
        }
        out.push(AlertEventInfo {
            kind: "reset-reached".to_owned(),
            account,
            text: format!("reset reached — resume {session}?"),
            at_ms: now,
        });
        let _ = usage.set_reset_state(&id, "notified");
    }

    let muted: Vec<String> = storage
        .app_settings()
        .get(ALERT_MUTES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    let quiet_hour = local_hour.is_none_or(|hour| !(7..22).contains(&hour));
    let (next_digest, flushed) = if quiet_hour {
        (digest.clone(), Vec::new())
    } else {
        juggler_core::alerts::digest_flush(&digest, now, &muted, local_hour.unwrap_or(12))
    };
    for entry in flushed {
        out.push(AlertEventInfo {
            kind: "digest".to_owned(),
            account: entry.key.clone(),
            text: entry.text,
            at_ms: now,
        });
    }
    if let Ok(blob) = serde_json::to_string(&next_digest) {
        let _ = storage.app_settings().set(ALERT_DIGEST_KEY, &blob);
    }
    out
}

fn short_label(label: &str) -> String {
    label.chars().take(18).collect()
}

fn badge_word(confidence: juggler_core::model::Confidence) -> &'static str {
    match confidence {
        juggler_core::model::Confidence::Measured => "Measured",
        juggler_core::model::Confidence::Computed => "Computed",
        juggler_core::model::Confidence::Estimated => "estimate",
    }
}

/// AJ-E611: the alert evaluation pass (UI polls this on its own
/// cadence; emissions are at-most-once by the machines). The UI passes
/// its wall-clock local hour — the quiet-hours gate for the digest
/// flush.
#[tauri::command]
#[must_use]
pub fn alerts_tick(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    local_hour: Option<u32>,
) -> Vec<AlertEventInfo> {
    alerts_tick_inner(&state, local_hour)
}

/// AJ-E611: one reset-queue row.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct ResetQueueInfo {
    /// Queue entry id.
    pub id: String,
    /// The session that hit the wall.
    pub session: String,
    /// Its account.
    pub account: String,
    /// Resume-at instant, epoch ms.
    pub resume_at_ms: i64,
    /// `queued|notified|ready|resumed|dismissed|stale`.
    pub state: String,
}

/// AJ-E611 Req.6: queues a resume for an exhausted session — a
/// PASSIVE timer + notification; the resume fires only from the
/// notification/row click (`reset_queue_resume`).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub fn reset_queue_queue(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    session: String,
    account: String,
    resume_at_ms: i64,
) -> AppResult<()> {
    let storage = lock_storage_arc(&state)?;
    let id = format!("rq-{session}");
    storage
        .usage()
        .queue_reset(&id, &session, &account, resume_at_ms)
        .map_err(AppError::from)?;
    Ok(())
}

/// AJ-E611: the queue rows (countdown/ready/stale explained).
#[tauri::command]
#[must_use]
pub fn reset_queue_list(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Vec<ResetQueueInfo> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    let now = juggler_core::model::Timestamp::now().millis();
    storage
        .usage()
        .pending_resumes(now + 365 * 24 * 3_600_000)
        .unwrap_or_default()
        .into_iter()
        .map(|(id, session, account, resume_at)| ResetQueueInfo {
            id,
            session,
            account,
            resume_at_ms: resume_at,
            state: if resume_at <= now { "ready" } else { "queued" }.to_owned(),
        })
        .collect()
}

/// AJ-E611: dismiss a queued resume (explicit user action).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub fn reset_queue_dismiss(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    id: String,
) -> AppResult<()> {
    let storage = lock_storage_arc(&state)?;
    storage
        .usage()
        .set_reset_state(&id, "dismissed")
        .map_err(AppError::from)?;
    Ok(())
}

/// AJ-E610 Req.3/E609 Req.4: the user-entered flat subscription fee
/// (USD/month) behind the dual view's ratio. Explicit set/clear only;
/// no plan-tier inference ever writes this.
///
/// # Errors
///
/// Returns the typed [`AppError`] on validation or storage failure.
#[tauri::command]
pub fn account_set_fee(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: String,
    fee_usd: Option<f64>,
) -> AppResult<()> {
    if let Some(fee) = fee_usd
        && !(0.0..=100_000.0).contains(&fee)
    {
        return Err(AppError::Validation {
            subject: "subscription fee",
            message: "must be between 0 and 100000 USD/month".into(),
        });
    }
    let storage = lock_storage_arc(&state)?;
    let key = format!("subscription_fee:{account}");
    match fee_usd {
        Some(fee) => storage
            .app_settings()
            .set(&key, &fee.to_string())
            .map_err(AppError::from)?,
        None => {
            storage
                .app_settings()
                .set(&key, "")
                .map_err(AppError::from)?;
        }
    }
    Ok(())
}

/// AJ-E610: the two-clock display for one account (by id).
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the account is unknown.
#[tauri::command]
pub fn quota_display(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: String,
) -> AppResult<QuotaDisplayInfo> {
    let storage = lock_storage_arc(&state)?;
    let record = storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(account.clone()))?;
    Ok(quota_display_inner(
        &storage,
        &record,
        juggler_core::model::Timestamp::now().millis(),
    ))
}

/// AJ-E610: the two-clock display for EVERY account (the rail chip /
/// `NewSession` row source; one query, no per-row round trips).
#[tauri::command]
#[must_use]
pub fn quota_display_all(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Vec<QuotaDisplayInfo> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    let now = juggler_core::model::Timestamp::now().millis();
    storage
        .accounts()
        .list()
        .unwrap_or_default()
        .iter()
        .map(|account| quota_display_inner(&storage, account, now))
        .collect()
}

/// AJ-E607 Req.5: every ingest source's observable health — the typed
/// state the AJ-E610 "Feed stale" banner binds to. A source silent
/// for more than 10 min carries `last_event_at` older than that (the
/// ENGINE demotes its Measured readings to Stale; data is never
/// deleted).
#[tauri::command]
#[must_use]
pub fn usage_source_health(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Vec<crate::usage_pipeline::SourceHealthInfo> {
    crate::usage_pipeline::source_health(&state)
}

#[tauri::command]
#[must_use]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn app_info() -> AppInfo {
    AppInfo::current()
}

/// A Provider the app can manage (ADR-0008; AJ-E107).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProviderInfo {
    /// Provider id, e.g. `claude-code`.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// Deprecated alias (one version). AJ-E107.
pub type HarnessInfo = ProviderInfo;

/// An account row for the UI.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountInfo {
    /// Account id.
    pub id: String,
    /// Provider id (wire field `harness` aliased one version; AJ-E107).
    #[serde(alias = "harness")]
    pub provider: String,
    /// User label.
    pub label: String,
    /// Plan tier, if set.
    pub plan: Option<String>,
    /// Login state: `logged_in` or `logged_out`.
    pub login_state: String,
    /// D-08 Tier-1 cleanup opt-in (off by default — notify-only tier).
    pub cleanup_opt_in: bool,
    /// Optional user-entered renewal date, local calendar `YYYY-MM-DD`
    /// (D-12); `null` renders `Expiry: unknown` — never inferred.
    pub renewal_date: Option<String>,
    /// AJ-E415 per-Account live-preview switch (off by default; the
    /// enable is the Screen-Recording consent moment).
    pub preview_opt_in: bool,
    /// AJ-E307 Req.1/2: import provenance (source canonical path /
    /// origin / imported-at). `None` = provenance-unknown (pre-E307
    /// accounts — never guessed).
    #[ts(optional = nullable)]
    pub import_source_path: Option<String>,
    /// The discovery origin (`cc-switch|t3|alias|home-pattern|manual`).
    #[ts(optional = nullable)]
    pub import_origin: Option<String>,
    /// Import completion time, epoch ms (None = never imported).
    #[ts(optional = nullable)]
    pub imported_at: Option<i64>,
}

/// A Launch settings row for the UI (AJ-E107).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LaunchRecipeInfo {
    /// Profile id.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Owning account id.
    pub account_id: String,
    /// Owning Account label (denormalized for the list).
    pub account_label: String,
    /// Provider id (wire field `harness` aliased one version; AJ-E107).
    #[serde(alias = "harness")]
    pub provider: String,
    /// Preferred model from the overlay, if any.
    pub model: Option<String>,
    /// Default working directory, if any.
    pub default_cwd: Option<String>,
}

/// Deprecated alias (one version). AJ-E107.
pub type ProfileInfo = LaunchRecipeInfo;

/// I-05: quota window snapshot (SPEC §6). `null` when no feed exists for
/// the Provider — never a fabricated percentage (AJ-E401 honesty contract).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QuotaSnapshotInfo {
    /// Percent of the window's allowance already used (0–100).
    pub used_percentage: f64,
    /// When the window resets, epoch milliseconds, if known.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// Trust level of the number: `measured` | `computed` | `estimated`.
    pub confidence: String,
}

/// I-06: token counters (SPEC §6). `null` until usage has actually been
/// observed — a never-sampled Instance reports no numbers, not zeros.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TokenCountsInfo {
    /// Prompt tokens consumed.
    #[ts(type = "number")]
    pub input: u64,
    /// Completion tokens consumed (thinking included, billed as output).
    #[ts(type = "number")]
    pub output: u64,
    /// Tokens served from the prompt cache.
    #[ts(type = "number")]
    pub cache_read: u64,
    /// Tokens written to the prompt cache.
    #[ts(type = "number")]
    pub cache_write: u64,
    /// Reasoning tokens (subset semantics per harness; kept separate for
    /// analytics).
    #[ts(type = "number")]
    pub reasoning: u64,
}

/// AJ-E405: the structured attention reason carried on the row — the
/// UI renders it verbatim (rule + measured inputs + since); it never
/// re-derives the reason from the status alone.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AttentionInfo {
    /// Rule id: `stuck_no_output` | `waiting_prompt` | `quota_exit`.
    pub rule: String,
    /// Human-readable measured inputs (`no output 6m`, `cpu 0.3%`).
    pub inputs: Vec<String>,
    /// When the condition began, epoch milliseconds.
    #[ts(type = "number")]
    pub since_ms: i64,
}

/// AJ-E409: the Quality/Performance signal set (D-07) — individually
/// labelled signals with tiers; NO composite score exists (grep gate).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QualitySignalsInfo {
    /// tokens/min: five 1-min bars (oldest first) or `null` = no data.
    #[ts(type = "Array<number> | null")]
    pub tokens_bars: Option<Vec<u64>>,
    /// Trailing-5-min average tokens/min (a real 0 when idle).
    #[ts(type = "number")]
    pub avg_tokens_per_min: f64,
    /// `2 errors / 41 turns` (exact rendered string) or the
    /// provider-specific not-measurable reason.
    pub error_label: String,
    /// True when the provider has no structured error feed.
    pub errors_not_measurable: bool,
    /// Context tokens in use, when the provider reports them.
    #[ts(type = "number | null")]
    pub context_used: Option<u64>,
    /// Provider-reported context window size, when known.
    #[ts(type = "number | null")]
    pub context_window: Option<u64>,
    /// Cumulative session tokens (labelled `cumulative, not context`).
    pub cumulative: TokenCountsInfo,
    /// Session cost USD from the versioned pricing table (`Est ·
    /// value-equiv.`); `null` when the model has no pricing row.
    #[ts(type = "number | null")]
    pub session_cost_usd: Option<f64>,
    /// The model the cost was priced against (null = unpriced).
    pub priced_model: Option<String>,
    /// Quota forecast RANGE label (`~13-27m at current rate · Est`) or
    /// `not burning`.
    pub quota_forecast: String,
    /// The AJ-E405 stuck rule + inputs, when tripped (permanent `?`).
    pub stuck: Option<AttentionInfo>,
}

/// I-10: why an Instance needs its user (`null` when it does not).
/// Derived from the lifecycle status here; AJ-E405 replaces the
/// derivation with structured heuristic reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum NeedsAttention {
    /// A permission/wait prompt is alive.
    WaitingUser,
    /// No-output heuristics tripped (rendered `Stuck?` — heuristic).
    Stuck,
    /// Rate-limit signature on exit.
    QuotaBlocked,
    /// Non-zero, non-user exit.
    Failed,
}

/// I-07/I-08 delta payload of the 5 s resource tick (`session://resources`,
/// AJ-E401 Req.4). One emission per tick carries every live Instance —
/// coalesced by construction.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceResourcesInfo {
    /// Instance id.
    pub id: String,
    /// CPU utilization summed over the process tree, percent; `null`
    /// until two table samples exist.
    pub cpu_pct: Option<f64>,
    /// Resident memory summed over the process tree, bytes; `null`
    /// until sampled.
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// Live descendant count; `null` until sampled.
    #[ts(type = "number | null")]
    pub child_count: Option<u32>,
}

/// I-05 delta payload of the 30 s quota tick (`session://quota`,
/// AJ-E401 Req.4). `quota` is `null` while no feed exists for the
/// Provider (the two-clock engine, AJ-E606+, populates it later).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceQuotaInfo {
    /// Instance id.
    pub id: String,
    /// Quota window snapshot, or `null` when unknown.
    pub quota: Option<QuotaSnapshotInfo>,
}

/// An Instance row for the UI (the conversation lives inside; AJ-E107).
/// One payload carries every indicator-row and inspector field I-01..I-11
/// (AJ-E401): absent data is `null`, never `0`, `""`, or a guessed name.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceInfo {
    /// Instance id.
    pub id: String,
    /// Launch settings id it runs under.
    pub launch_recipe_id: String,
    /// Provider id (wire field `harness` aliased one version); `unknown`
    /// when the launch settings row is gone — never blank.
    #[serde(alias = "harness")]
    pub provider: String,
    /// Owning account id; `null` when the launch settings row is gone.
    pub account_id: Option<String>,
    /// Owning Account label for display (wire field `label` aliased one
    /// version); `unknown` when unresolvable — never blank.
    #[serde(alias = "label")]
    pub account_label: String,
    /// I-03 display value: the RUNNING model once observed, else the
    /// launch overlay, else `default model` — never a guessed name.
    pub model: Option<String>,
    /// The launch-overlay model (intent, not telemetry; AJ-E412).
    pub launch_model: Option<String>,
    /// The transcript-OBSERVED running model; `null` until observed —
    /// never filled from the overlay.
    pub running_model: Option<String>,
    /// How `model` is known: `launch` (overlay only, not verified
    /// live), `in-session` (observed, differs from the overlay),
    /// `transcript` (observed), `unknown` (neither).
    pub model_source: String,
    /// I-04: lifecycle status string (storage vocabulary).
    pub status: String,
    /// I-05: quota window snapshot; `null` when no feed exists.
    pub quota: Option<QuotaSnapshotInfo>,
    /// I-06: token counters; `null` until usage has been observed.
    pub tokens: Option<TokenCountsInfo>,
    /// I-07: CPU percent summed over the process tree; `null` until the
    /// resource tick has two samples.
    pub cpu_pct: Option<f64>,
    /// I-07: resident bytes summed over the process tree; `null` until
    /// sampled (a finished Instance is not measurable — not zero).
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// I-08: live descendant count; `null` until sampled, `0` after exit
    /// (nothing can be running then).
    #[ts(type = "number | null")]
    pub child_count: Option<u32>,
    /// I-08: subtask count; `null` — no transcript source exists yet
    /// (AJ-E409 lands the signals; never a guessed count).
    #[ts(type = "number | null")]
    pub subtask_count: Option<u64>,
    /// I-09: git branch of the working directory, parsed from
    /// `.git/HEAD` (no subprocess); `null` for non-git or detached.
    pub git_branch: Option<String>,
    /// I-09: dirty file count; `null` — requires a `git status` walk that
    /// must never run per-row on the hot path (collector comes later).
    #[ts(type = "number | null")]
    pub git_dirty_count: Option<u64>,
    /// I-10: why this Instance needs its user; `null` when it does not.
    pub needs_attention: Option<NeedsAttention>,
    /// AJ-E405: the structured reason (rule + measured inputs + since)
    /// behind `needs_attention`; `null` when healthy or pre-E405 rows.
    pub attention: Option<AttentionInfo>,
    /// Start time, epoch milliseconds.
    #[ts(type = "number")]
    pub started_at_ms: i64,
    /// I-11: last observed activity, epoch milliseconds; `null` when
    /// never observed.
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// Fleet dimension (D-13): machine the Instance runs on.
    pub machine_id: String,
    /// Working directory (AJ-E407 Overview: copyable project + cwd).
    pub cwd: String,
    /// Instance kind: `terminal` | `desktop`.
    pub kind: String,
    /// True when crash recovery re-adopted this Instance's verified
    /// root (AJ-E510): `Running — re-adopted`, terminal detached
    /// (output from before the crash is unavailable), stop via the
    /// AJ-E507 sheet.
    pub adopted: bool,
    /// Owning project id, when the cwd resolved to a tracked project.
    pub project_id: Option<String>,
    /// The native conversation inside this Instance, once discovered
    /// (AJ-E104: Instance row → conversation inside; None while Starting).
    pub conversation_id: Option<String>,
    /// AJ-E413 Req.1: the per-Provider isolation claim for desktop
    /// Instances, verbatim (`Single-account app — Juggler tracks,
    /// cannot isolate` for single-account apps); `null` for terminal
    /// Instances and when the provider's adapter is not registered.
    pub isolation_label: Option<String>,
}

/// Deprecated alias (one version). AJ-E107.
pub type SessionInfo = InstanceInfo;

/// A tracked project for the UI — the juggling anchor (ADR-0007).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProjectInfo {
    /// Project id.
    pub id: String,
    /// Canonical root directory.
    pub root: String,
    /// Display name.
    pub name: String,
    /// Last activity, epoch milliseconds (when known).
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// The instance currently managing this project, when live.
    pub active_instance: Option<SessionInfo>,
    /// Account labels that have ever run in this project (AI Juggler runs
    /// plus imported pre-existing history).
    pub accounts_used: Vec<String>,
    /// Total instances run through AI Juggler in this project.
    #[ts(type = "number")]
    pub instance_count: u64,
    /// Sessions discovered in imported transcripts (pre-AI-Juggler work).
    #[ts(type = "number")]
    pub history_sessions: u64,
    /// True when the root directory no longer exists on disk (AJ-E408
    /// Overview danger banner + `Relocate…`).
    pub missing: bool,
}

/// Dashboard counts.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StatsInfo {
    /// Managed accounts.
    #[ts(type = "number")]
    pub accounts: u64,
    /// Configured profiles.
    #[ts(type = "number")]
    pub profiles: u64,
    /// Sessions not in a terminal state.
    #[ts(type = "number")]
    pub active_sessions: u64,
    /// AJ-E501: a configured process ledger failed — cleanup is
    /// read-only until restart (the degraded banner's data).
    pub ledger_degraded: bool,
}

/// Result of launching a session.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LaunchResult {
    /// The new session id.
    pub session_id: String,
    /// OS process id.
    pub pid: u32,
}

/// Result of a desktop launch (AJ-E413 Req.1): the tracked Instance id,
/// the OS pid, and the bundle identifier when the platform has one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DesktopLaunchInfo {
    /// The new (desktop-kind) Instance id.
    pub session_id: String,
    /// OS process id.
    pub pid: u32,
    /// Bundle identifier (macOS), when known.
    pub bundle_id: Option<String>,
    /// The per-Provider isolation claim, shown verbatim (Req.1): e.g.
    /// `Single-account app — Juggler tracks, cannot isolate`.
    pub isolation_label: String,
}

/// An existing harness setup found by the onboarding environment scan
/// (spec §12). Metadata-only discovery; the wizard shows `notes`
/// verbatim.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiscoveredInfo {
    /// Harness id.
    pub harness: String,
    /// Config dir path.
    pub path: String,
    /// `vendor-default` | `env-override` | `home-pattern`.
    pub origin: String,
    /// `logged_in` | `logged_out`.
    pub login: String,
    /// Approximate bytes on disk (caches excluded).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Most recent activity, unix seconds (when known).
    #[ts(type = "number")]
    pub last_used_secs: Option<u64>,
    /// True when already managed by AI Juggler.
    pub managed: bool,
    /// Human-facing hints shown by the wizard.
    pub notes: Vec<String>,
    /// AJ-E303: the inferred identity (purpose/login/activity with
    /// Found/Likely/Guessed confidence words — never scores).
    pub inference: InferenceInfo,
    /// AJ-E303: every signal that found this dir (merged duplicates:
    /// alias + home-pattern → ONE candidate with both signals).
    pub signals: Vec<String>,
}

/// AJ-E303: the inference block on one candidate (rule-table fields,
/// §4.1 — deterministic, offline, first match wins per field).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InferenceInfo {
    /// Inferred account purpose ("Work", "Personal", humanised name).
    pub purpose: String,
    /// found | likely | guessed.
    pub purpose_confidence: String,
    /// signed-in | maybe-expired | not-signed-in.
    pub login: String,
    /// found | likely | guessed.
    pub login_confidence: String,
    /// active | dormant | archive.
    pub activity: String,
    /// found | likely | guessed.
    pub activity_confidence: String,
    /// Human words for the activity tier ("Dormant 40 d").
    pub activity_words: String,
    /// True when the login lives in the OS keychain (the re-login
    /// caveat copy applies after import).
    pub keychain_bound: bool,
}

/// Result of importing a discovered setup as a managed account.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ImportResult {
    /// The created account.
    pub account: AccountInfo,
    /// Files copied into the managed dir.
    #[ts(type = "number")]
    pub files_copied: u64,
    /// Bytes copied into the managed dir.
    #[ts(type = "number")]
    pub bytes_copied: u64,
    /// Cache/noise entries skipped, human-readable.
    pub skipped: Vec<String>,
    /// Login state of the imported account: `logged_in` | `logged_out`.
    pub login: String,
    /// AJ-E307 Req.3: per top level entry bytes, largest first — the
    /// copy receipt's breakdown.
    pub folders: Vec<FolderSizeInfo>,
    /// True when this was the settings-only degraded import.
    pub degraded: bool,
    /// When login did not carry over, the plain-language reason and next
    /// step (e.g. macOS keychain path-locking).
    pub login_note: Option<String>,
}

// ---------------------------------------------------------------------------
// commands

/// Lists the Providers this build can manage (canonical name; the
/// legacy `harnesses_list` alias is registered in the handler).
#[tauri::command(rename = "providers_list")]
#[must_use]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn harnesses_list() -> Vec<ProviderInfo> {
    vec![ProviderInfo {
        id: juggler_adapters::CLAUDE_CODE_ID.to_string(),
        name: "Claude Code".to_string(),
    }]
}

/// Lists accounts with their login state.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn accounts_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<AccountInfo>> {
    let storage = lock_storage(&state)?;
    let mut out = Vec::new();
    for account in storage.accounts().list()? {
        out.push(account_info_of(&account));
    }
    Ok(out)
}

/// Creates a managed account: prepares the isolated harness state dir and
/// persists the account.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_create(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    label: String,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    create_account(state.inner(), &harness, &label, plan)
}

/// Core implementation shared with integration tests (commands need a
/// Tauri runtime; this does not).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn create_account(
    state: &Arc<AppState>,
    harness: &str,
    label: &str,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    let id = AccountId::new(format!("{harness}-{}", unique_suffix()));
    let state_dir = state.accounts_dir().join(harness).join(id.as_str());
    adapter.prepare_account(&state_dir)?;
    let account = Account {
        id,
        harness: juggler_core::model::HarnessId::new(harness),
        label: label.to_string(),
        plan: plan.map(juggler_core::model::PlanTier::new),
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    };
    lock_storage_arc(state)?.accounts().insert(&account)?;
    Ok(account_info_of(&account))
}

/// Re-probes an account's login state (metadata-only; ADR-0003).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_login_state(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<String> {
    let account = find_account(&state, &account_id)?;
    Ok(login_state_name(login_state_of(&account)).to_string())
}

/// The copyable shell command that runs the vendor's own login against this
/// account's isolated config dir (external-terminal fallback, spec §4.1).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_login_command(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<String> {
    let account = find_account(&state, &account_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    juggler_adapter::adapter::shell_snippet(
        adapter.as_ref(),
        &account.state_dir,
        &std::collections::BTreeMap::new(),
    )
    .ok_or_else(|| AppError::Validation {
        subject: "sign-in",
        message: format!("{} has no terminal sign-in command", account.harness),
    })
}

/// Scans the machine for existing harness setups the onboarding wizard can
/// import (metadata-only; ADR-0006). Never fails — an empty list is a
/// valid "nothing found".
#[tauri::command]
#[must_use]
pub fn environment_scan(state: tauri::State<'_, Arc<AppState>>) -> Vec<DiscoveredInfo> {
    environment_scan_impl(state.inner())
}

/// The stable LOCAL machine id (AJ-E501 Req.6, D-13): generated once,
/// persisted in app settings — never a hard-coded `"this-mac"` literal
/// at call sites.
pub fn local_machine_id(state: &Arc<AppState>) -> MachineId {
    let stored = {
        let storage = lock_storage_arc(state);
        storage
            .ok()
            .and_then(|storage| storage.app_settings().get("local_machine_id").ok())
            .flatten()
    };
    if let Some(id) = stored {
        return MachineId::new(id);
    }
    let generated = format!("machine-{}", unique_suffix());
    if let Ok(storage) = lock_storage_arc(state) {
        let _ = storage.app_settings().set("local_machine_id", &generated);
    }
    MachineId::new(generated)
}

/// The ledger spawn context for an account launch (root role).
fn ledger_ctx(
    state: &Arc<AppState>,
    account: &Account,
    project_id: Option<String>,
) -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: account.harness.to_string(),
        account_id: Some(account.id.to_string()),
        project_id,
        machine: local_machine_id(state),
        role: juggler_core::storage::ROLE_ROOT,
    }
}

/// Core implementation shared with integration tests.
#[must_use]
pub fn environment_scan_impl(state: &Arc<AppState>) -> Vec<DiscoveredInfo> {
    let mut out = Vec::new();
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        for found in adapter.scan_environment() {
            out.push(discovered_info_of(found));
        }
    }
    out
}

/// AJ-E307 Req.3: one breakdown row (top-level entry + bytes).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FolderSizeInfo {
    /// Top-level folder (with `/`) or file name.
    pub name: String,
    /// Importable bytes under it.
    #[ts(type = "number")]
    pub bytes: u64,
}

/// AJ-E307 Req.3: the pre-copy source inspection on the wire.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
// A flat one-row wire payload for one panel — the honest shape here
// is four independent flags, not a state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct SourceInspectInfo {
    /// Total importable bytes.
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// Importable file count.
    #[ts(type = "number")]
    pub file_count: u64,
    /// Per-entry breakdown, largest first.
    pub folders: Vec<FolderSizeInfo>,
    /// The bounded walk hit its file cap (sizes are a floor).
    pub truncated: bool,
    /// The copy ceiling in bytes — the UI offers the degraded path
    /// from `total_bytes > ceiling_bytes`, never a raw ceiling string.
    #[ts(type = "number")]
    pub ceiling_bytes: u64,
    /// Config evidence — false renders the plain-language rejection.
    pub is_config: bool,
    /// Already managed by AI Juggler.
    pub already_managed: bool,
    /// Source login state: `logged_in` | `logged_out`.
    pub login: String,
    /// Keychain-bound login (signed in, no credentials file) — the
    /// re-login note shows at Confirm-source time (Req.8).
    pub keychain_bound: bool,
}

/// AJ-E309: the install moment the funnel measures from — written
/// once, on the first funnel event this data dir ever records.
const FUNNEL_INSTALL_AT_KEY: &str = "funnel_install_at_ms";

/// AJ-E309: set once the first Instance launch has been recorded
/// (`first_launch` is a once-per-install event).
const FUNNEL_FIRST_LAUNCH_KEY: &str = "funnel_first_launch_recorded";

/// Maps a core validation rejection to the coded `validation` wire
/// (not retryable); everything else stays a storage failure.
fn core_to_app(error: juggler_core::CoreError) -> AppError {
    match error {
        juggler_core::CoreError::Validation { subject, message } => {
            AppError::Validation { subject, message }
        }
        other => AppError::Core(other),
    }
}

/// AJ-E309 Req.6: appends one funnel event to the LOCAL store. Runs
/// on a blocking worker — never on the UI thread — and the UI never
/// awaits it on a user path.
///
/// # Errors
///
/// Returns the typed [`AppError`] on a schema violation (coded
/// `validation`) or a storage/I/O failure.
#[tauri::command]
pub async fn funnel_event(
    state: tauri::State<'_, Arc<AppState>>,
    kind: String,
    fields: std::collections::BTreeMap<String, String>,
) -> AppResult<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        funnel_event_impl(
            &state,
            &kind,
            fields,
            juggler_core::model::Timestamp::now().millis(),
        )
    })
    .await
    .map_err(|error| AppError::Io {
        context: "funnel event",
        message: error.to_string(),
    })?
}

/// Core implementation shared with integration tests (`now_ms` is
/// injected so the dry-run export is deterministic).
///
/// The first event ever recorded stamps the install moment;
/// `first_launch` is recorded once per install, with
/// `seconds_since_install` computed HERE (the UI never supplies it).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`funnel_event`].
pub fn funnel_event_impl(
    state: &Arc<AppState>,
    kind: &str,
    fields: std::collections::BTreeMap<String, String>,
    now_ms: i64,
) -> AppResult<()> {
    let mut event = juggler_core::funnel_events::FunnelEvent {
        kind: kind.to_owned(),
        at_ms: now_ms,
        fields,
    };
    juggler_core::funnel_events::validate(&event).map_err(core_to_app)?;
    let first_launch = kind == "first_launch";
    {
        let storage = lock_storage_arc(state)?;
        let settings = storage.app_settings();
        let stored = settings
            .get(FUNNEL_INSTALL_AT_KEY)?
            .and_then(|raw| raw.parse::<i64>().ok());
        let install_at = if let Some(at) = stored {
            at
        } else {
            settings.set(FUNNEL_INSTALL_AT_KEY, &now_ms.to_string())?;
            now_ms
        };
        if first_launch {
            if settings.get(FUNNEL_FIRST_LAUNCH_KEY)?.is_some() {
                return Ok(());
            }
            event.fields.clear();
            event.fields.insert(
                "seconds_since_install".to_owned(),
                ((now_ms - install_at).max(0) / 1000).to_string(),
            );
        }
    }
    juggler_core::funnel_events::FunnelLog::open(&state.data_dir)
        .append(&event)
        .map_err(core_to_app)?;
    if first_launch {
        lock_storage_arc(state)?
            .app_settings()
            .set(FUNNEL_FIRST_LAUNCH_KEY, "1")?;
    }
    Ok(())
}

/// AJ-E309 Req.7: the LOCAL funnel export — counts per kind, flow
/// steps reached, policy choices, time to first Instance, and the
/// per-field inference-override rates (> 30% flagged). Markdown text
/// returned to the webview, which saves it on the user's click; no
/// network call exists.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub fn funnel_export(state: tauri::State<'_, Arc<AppState>>) -> AppResult<String> {
    funnel_export_impl(state.inner())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
pub fn funnel_export_impl(state: &Arc<AppState>) -> AppResult<String> {
    let events = juggler_core::funnel_events::FunnelLog::open(&state.data_dir)
        .read()
        .map_err(core_to_app)?;
    Ok(juggler_core::funnel_events::report(&events))
}

/// AJ-E309 Req.5: what the [Expert setup…] panel shows for one
/// Provider — or one Account (and optionally one of its Launch
/// settings). Every path comes from AI Juggler's own state, never
/// from the caller.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ExpertSetupInfo {
    /// The Provider id.
    pub provider: String,
    /// The Account this describes; `null` = the Provider's folder
    /// (before any Account exists in this flow).
    pub account_id: Option<String>,
    /// The managed state folder (read-only display).
    pub state_dir: String,
    /// Whether that folder exists yet.
    pub exists: bool,
    /// The copyable shell line reproducing the setup (isolation env +
    /// Launch-settings overrides + the vendor's command); `null`
    /// before an Account exists or for Providers without a CLI.
    pub shell_snippet: Option<String>,
    /// Environment keys Launch-settings overrides may never set.
    pub protected_env_keys: Vec<String>,
    /// One plain sentence on what the folder is.
    pub note: String,
}

/// AJ-E309 Req.5: the Expert-setup facts.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown Provider, Account,
/// or Launch settings (or Launch settings of another Account).
#[tauri::command]
pub fn expert_setup_info(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    account_id: Option<String>,
    recipe_id: Option<String>,
) -> AppResult<ExpertSetupInfo> {
    expert_setup_info_impl(
        state.inner(),
        &provider,
        account_id.as_deref(),
        recipe_id.as_deref(),
    )
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`expert_setup_info`].
pub fn expert_setup_info_impl(
    state: &Arc<AppState>,
    provider: &str,
    account_id: Option<&str>,
    recipe_id: Option<&str>,
) -> AppResult<ExpertSetupInfo> {
    let Some(account_id) = account_id else {
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(provider))
            .ok_or_else(|| AppError::UnknownHarness(provider.to_owned()))?;
        let dir = state.accounts_dir().join(provider);
        return Ok(ExpertSetupInfo {
            provider: provider.to_owned(),
            account_id: None,
            exists: dir.is_dir(),
            state_dir: dir.to_string_lossy().into_owned(),
            shell_snippet: None,
            protected_env_keys: adapter
                .isolation_env_keys()
                .iter()
                .map(|key| (*key).to_owned())
                .collect(),
            note: "Each new account gets its own isolated folder in here — the shell line \
                   appears once the account exists."
                .to_owned(),
        });
    };
    let account = find_account_arc(state, account_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let extra_env = match recipe_id {
        Some(recipe_id) => {
            let (recipe, owner) = find_profile_with_account_arc(state, recipe_id)?;
            if owner.id != account.id {
                return Err(AppError::Validation {
                    subject: "launch settings",
                    message: "those Launch settings belong to another account".to_owned(),
                });
            }
            recipe.settings_overlay.env
        }
        None => std::collections::BTreeMap::new(),
    };
    Ok(ExpertSetupInfo {
        provider: account.harness.to_string(),
        account_id: Some(account.id.to_string()),
        exists: account.state_dir.is_dir(),
        state_dir: account.state_dir.to_string_lossy().into_owned(),
        shell_snippet: juggler_adapter::adapter::shell_snippet(
            adapter.as_ref(),
            &account.state_dir,
            &extra_env,
        ),
        protected_env_keys: adapter
            .isolation_env_keys()
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        note: format!(
            "“{}” keeps all of its state here — isolated from your other accounts and \
             from your own install.",
            account.label
        ),
    })
}

/// AJ-E309 Req.5: [Reveal in Finder] for a managed folder. The path
/// is resolved from AI Juggler's own state (an Account's folder, or
/// the Provider's accounts folder) — never caller-supplied — and the
/// nearest existing folder inside the data dir is revealed when the
/// target does not exist yet. Read-only: nothing is created.
///
/// # Errors
///
/// Returns the typed [`AppError`] for unknown ids or when Finder
/// cannot be opened.
#[tauri::command]
pub fn expert_reveal(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    account_id: Option<String>,
) -> AppResult<()> {
    let target = expert_reveal_target(state.inner(), &provider, account_id.as_deref())?;
    app.opener()
        .reveal_item_in_dir(&target)
        .map_err(|error| AppError::Io {
            context: "reveal in Finder",
            message: error.to_string(),
        })
}

/// The folder [`expert_reveal`] would show (shared with tests): the
/// requested managed folder, or its nearest existing ancestor inside
/// the data dir.
///
/// # Errors
///
/// Returns the typed [`AppError`] for unknown ids.
pub fn expert_reveal_target(
    state: &Arc<AppState>,
    provider: &str,
    account_id: Option<&str>,
) -> AppResult<std::path::PathBuf> {
    let info = expert_setup_info_impl(state, provider, account_id, None)?;
    let mut target = std::path::PathBuf::from(info.state_dir);
    while !target.is_dir() && target != state.data_dir {
        if !target.pop() {
            break;
        }
    }
    if !target.starts_with(&state.data_dir) {
        target.clone_from(&state.data_dir);
    }
    Ok(target)
}

/// `_`-delimited key segments that mark an environment key as
/// credential-shaped (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`)
/// — whole segments, so `CLAUDE_CODE_MAX_OUTPUT_TOKENS` stays allowed.
const CREDENTIAL_KEY_SEGMENTS: &[&str] = &[
    "KEY",
    "APIKEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "CREDENTIALS",
    "AUTH",
    "OAUTH",
    "BEARER",
    "COOKIE",
];

/// The most overrides one Launch settings may carry.
const ENV_OVERLAY_MAX: usize = 32;

/// AJ-E309: validates a Launch-settings environment overlay. Keys are
/// shell-style (`[A-Z_][A-Z0-9_]*`, ≤ 64); values are one line
/// (≤ 1024 bytes). Isolation keys are refused (they would repoint the
/// Account's folder), and so are credential-shaped keys — vendor
/// credentials are never stored by AI Juggler; the vendor's own
/// sign-in owns them.
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming the first offending key.
pub fn validate_env_overlay(
    protected: &[&str],
    env: &std::collections::BTreeMap<String, String>,
) -> AppResult<()> {
    let refuse = |message: String| AppError::Validation {
        subject: "environment override",
        message,
    };
    if env.len() > ENV_OVERLAY_MAX {
        return Err(refuse(format!(
            "at most {ENV_OVERLAY_MAX} overrides per Launch settings"
        )));
    }
    for (key, value) in env {
        let mut bytes = key.bytes();
        let shaped = matches!(bytes.next(), Some(b'A'..=b'Z' | b'_'))
            && key.len() <= 64
            && bytes.all(|b| matches!(b, b'A'..=b'Z' | b'0'..=b'9' | b'_'));
        if !shaped {
            return Err(refuse(format!(
                "“{key}” isn't a valid variable name — use capitals, digits and _"
            )));
        }
        if protected.contains(&key.as_str()) {
            return Err(refuse(format!(
                "{key} is managed by AI Juggler — it keeps this account isolated"
            )));
        }
        if key
            .split('_')
            .any(|segment| CREDENTIAL_KEY_SEGMENTS.contains(&segment))
        {
            return Err(refuse(format!(
                "{key} looks like a credential — sign in with the vendor's own flow instead; \
                 AI Juggler never stores credentials"
            )));
        }
        if value.len() > 1024 || value.contains(['\0', '\n', '\r']) {
            return Err(refuse(format!(
                "the value for {key} must be a single line under 1 KB"
            )));
        }
    }
    Ok(())
}

/// AJ-E309 Req.5: one Launch settings' environment overrides.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown id.
#[tauri::command]
pub fn launch_recipe_env_get(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    let (recipe, _) = find_profile_with_account_arc(state.inner(), &recipe_id)?;
    Ok(recipe.settings_overlay.env)
}

/// AJ-E309 Req.5: replaces one Launch settings' environment
/// overrides (the env overlay editor's Save). Validated first —
/// nothing is written on a refusal.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown id, a refused
/// overlay, or a storage failure.
#[tauri::command]
pub fn launch_recipe_env_set(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
    env: std::collections::BTreeMap<String, String>,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    launch_recipe_env_set_impl(state.inner(), &recipe_id, env)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`launch_recipe_env_set`].
pub fn launch_recipe_env_set_impl(
    state: &Arc<AppState>,
    recipe_id: &str,
    env: std::collections::BTreeMap<String, String>,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    let (mut recipe, account) = find_profile_with_account_arc(state, recipe_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    validate_env_overlay(adapter.isolation_env_keys(), &env)?;
    recipe.settings_overlay.env = env;
    lock_storage_arc(state)?.profiles().update(&recipe)?;
    Ok(recipe.settings_overlay.env)
}

/// AJ-E308 Req.7: one named isolation-checklist row.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct IsolationRowInfo {
    /// Stable row id (`managed_dir_private`, …).
    pub id: &'static str,
    /// The user-facing line (exactly what was checked).
    pub label: String,
    /// True when the check passed; a fail BLOCKS \[Launch\], not \[Done\].
    pub pass: bool,
    /// Evidence/note copy (the failing line names itself).
    pub detail: String,
}

/// AJ-E308 Req.7: the post-import/create isolation checklist — six
/// named checks, metadata-only, run automatically; any fail blocks
/// \[Launch\] with the failing line named.
///
/// # Errors
///
/// Returns the typed [`AppError`] (unknown account / storage).
#[tauri::command]
pub fn isolation_check(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<Vec<IsolationRowInfo>> {
    isolation_check_impl(state.inner(), &account_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on lookup/storage failures.
// One assembly pass over six named checks; splitting it would hide
// the checklist's shape.
#[allow(clippy::too_many_lines)]
pub fn isolation_check_impl(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<Vec<IsolationRowInfo>> {
    let account = {
        let storage = lock_storage_arc(state)?;
        storage
            .accounts()
            .get(&AccountId::new(account_id.to_owned()))?
    };
    let dir = account.state_dir.clone();
    let mut rows = Vec::new();

    // 1. The managed dir is owner-private (0700). The mask IS the
    // check (no group/other bits set) — a trailing-zeros rewrite
    // would obscure it.
    #[cfg(unix)]
    let private_pass = {
        use std::os::unix::fs::PermissionsExt;
        #[allow(clippy::verbose_bit_mask)]
        let check = |mode: u32| mode & 0o077 == 0;
        std::fs::metadata(&dir).is_ok_and(|meta| check(meta.permissions().mode()))
    };
    #[cfg(not(unix))]
    let private_pass = std::fs::metadata(&dir).is_ok();
    rows.push(IsolationRowInfo {
        id: "managed_dir_private",
        label: "Account folder is owner-only (0700)".into(),
        pass: private_pass,
        detail: format!("{}", dir.display()),
    });

    // 2. The managed marker is present (this dir is unambiguously ours).
    let marker_pass = dir
        .join(juggler_adapters::claude_code::MANAGED_MARKER)
        .is_file();
    rows.push(IsolationRowInfo {
        id: "marker_present",
        label: "The managed-folder marker is present".into(),
        pass: marker_pass,
        detail: "the folder identifies itself as AI Juggler-managed".into(),
    });

    // 3. The launch environment points ONLY at this managed dir.
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.as_str().to_owned()))?;
    let env_ok = adapter
        .launch_spec(
            &account,
            &Profile {
                id: ProfileId::new("check"),
                name: "check".into(),
                account: account.id.clone(),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            },
            std::path::Path::new("/"),
            &juggler_adapter::LaunchMode::Interactive,
        )
        .is_ok_and(|spec| {
            spec.env
                .get("CLAUDE_CONFIG_DIR")
                .is_some_and(|value| std::path::Path::new(value) == dir)
        });
    rows.push(IsolationRowInfo {
        id: "launch_env_managed_only",
        label: "Launch environment points only at this account's folder".into(),
        pass: env_ok,
        detail: "CLAUDE_CONFIG_DIR resolves to the managed dir".into(),
    });

    // 4. The import SOURCE carries no AI Juggler writes (no marker,
    //    nothing of ours inside it) — metadata-only.
    let source_clean = account.import_source_path.as_deref().is_none_or(|source| {
        !std::path::Path::new(source)
            .join(juggler_adapters::claude_code::MANAGED_MARKER)
            .is_file()
    });
    rows.push(IsolationRowInfo {
        id: "source_free_of_juggler_writes",
        label: "The imported-from folder is untouched (no AI Juggler writes)".into(),
        pass: source_clean,
        detail: account
            .import_source_path
            .clone()
            .unwrap_or_else(|| "created fresh — no import source".into()),
    });

    // 5. Per-dir probe independence: this dir's login classification
    //    reads ONLY this dir (structural: single-path probe).
    let probe_state = login_state_of(&account);
    rows.push(IsolationRowInfo {
        id: "probe_independence",
        label: "Login state reads only this account's folder".into(),
        pass: !matches!(probe_state, juggler_adapters::LoginState::Unknown),
        detail: format!(
            "probe classified this dir as {}",
            login_state_name(probe_state)
        ),
    });

    // 6. No stray files directly under the harness accounts root —
    //    writes outside managed dirs would land there.
    let parent_clean = dir.parent().is_none_or(|parent| {
        std::fs::read_dir(parent)
            .is_ok_and(|read| read.flatten().all(|entry| entry.path().is_dir()))
    });

    rows.push(IsolationRowInfo {
        id: "no_writes_outside_managed",
        label: "No stray files outside managed folders".into(),
        pass: parent_clean,
        detail: "the accounts root holds only account folders".into(),
    });

    Ok(rows)
}

/// Inspects a candidate import source BEFORE any copy (metadata-only).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] (unknown harness / unreadable path).
pub fn source_inspect(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    source_path: String,
) -> AppResult<SourceInspectInfo> {
    source_inspect_impl(state.inner(), &harness, &source_path)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on failure.
pub fn source_inspect_impl(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
) -> AppResult<SourceInspectInfo> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    let inspect = adapter.inspect_source(std::path::Path::new(source_path))?;
    // Keychain-bound: signed in with NO credentials file (the login
    // cannot move — E303's caveat, surfaced at Confirm-source).
    let keychain_bound = inspect.login == juggler_adapter::environment::LoginProbe::LoggedIn
        && !std::path::Path::new(source_path)
            .join(".credentials.json")
            .exists();
    Ok(SourceInspectInfo {
        total_bytes: inspect.total_bytes,
        file_count: inspect.file_count,
        folders: inspect
            .folders
            .into_iter()
            .map(|(name, bytes)| FolderSizeInfo { name, bytes })
            .collect(),
        truncated: inspect.truncated,
        ceiling_bytes: inspect.ceiling_bytes,
        is_config: inspect.is_config,
        already_managed: inspect.already_managed,
        login: login_probe_name(inspect.login),
        keychain_bound,
    })
}

/// Imports a discovered setup as a new managed account: byte-copies the
/// config into an isolated dir (source untouched), creates a default
/// "Main" profile so the account is immediately launchable.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_import(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    source_path: String,
    label: String,
    plan: Option<String>,
    origin: Option<String>,
    degraded: Option<bool>,
) -> AppResult<ImportResult> {
    account_import_impl(
        state.inner(),
        &harness,
        &source_path,
        &label,
        plan,
        origin,
        degraded.unwrap_or(false),
    )
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn account_import_impl(
    state: &Arc<AppState>,
    harness: &str,
    source_path: &str,
    label: &str,
    plan: Option<String>,
    origin: Option<String>,
    degraded: bool,
) -> AppResult<ImportResult> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    // AJ-E307 Req.2: duplicate protection — a provenance-matched
    // source is blocked with the existing account's label, before
    // anything is written.
    let canonical_source = match std::path::Path::new(source_path).canonicalize() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => source_path.to_owned(),
    };
    {
        let storage = lock_storage_arc(state)?;
        for existing in storage.accounts().list()? {
            if existing.import_source_path.as_deref() == Some(canonical_source.as_str()) {
                return Err(AppError::AlreadyImported {
                    label: existing.label.clone(),
                });
            }
        }
    }
    let id = AccountId::new(format!("{harness}-{}", unique_suffix()));
    let state_dir = state.accounts_dir().join(harness).join(id.as_str());
    let outcome = if degraded {
        adapter.import_account_degraded(std::path::Path::new(source_path), &state_dir)?
    } else {
        adapter.import_account(std::path::Path::new(source_path), &state_dir)?
    };
    let account = Account {
        id,
        harness: juggler_core::model::HarnessId::new(harness),
        label: label.to_string(),
        plan: plan.map(juggler_core::model::PlanTier::new),
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        // AJ-E307 Req.1: the import's provenance, written at import
        // time — canonical path + origin + completion stamp.
        import_source_path: Some(canonical_source),
        import_origin: origin,
        imported_at: Some(juggler_core::model::Timestamp::now().millis()),
    };
    {
        let storage = lock_storage_arc(state)?;
        storage.accounts().insert(&account)?;
        // A default profile makes the imported account launchable in one
        // click — the wizard's promise.
        storage.profiles().insert(&Profile {
            id: ProfileId::new(format!("prof-{}", unique_suffix())),
            name: "Main".to_string(),
            account: account.id.clone(),
            settings_overlay: SettingsOverlay::default(),
            default_cwd: None,
        })?;
    }
    Ok(ImportResult {
        account: account_info_of(&account),
        files_copied: outcome.files_copied,
        bytes_copied: outcome.bytes_copied,
        skipped: outcome.skipped,
        login: login_probe_name(outcome.login),
        login_note: outcome.login_note,
        folders: outcome
            .folders
            .into_iter()
            .map(|(name, bytes)| FolderSizeInfo { name, bytes })
            .collect(),
        degraded: outcome.degraded,
    })
}

/// Spawns the guided login for an account inside an embedded terminal.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn terminal_spawn_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<crate::terminal::TerminalInfo> {
    crate::terminal::spawn_login_terminal_impl(&app, state.inner(), &account_id)
}

/// Starts streaming a supervised session into the terminal dock
/// (idempotent per terminal).
#[tauri::command]
#[must_use]
pub fn terminal_attach(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> crate::terminal::TerminalInfo {
    let info = crate::terminal::TerminalInfo { id: id.clone() };
    crate::terminal::attach_forwarder(
        &app,
        state.inner(),
        &juggler_core::model::SessionId::new(id),
    );
    info
}

/// Feeds keystrokes to a terminal.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn terminal_write(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    data: String,
) -> AppResult<()> {
    crate::terminal::terminal_write_impl(state.inner(), &id, &data)
}

/// Resizes a terminal.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn terminal_resize(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    cols: u16,
    rows: u16,
) -> AppResult<()> {
    crate::terminal::terminal_resize_impl(state.inner(), &id, cols, rows)
}

/// Stops a terminal's process (graceful ladder).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn terminal_kill(state: tauri::State<'_, Arc<AppState>>, id: String) -> AppResult<()> {
    crate::terminal::terminal_kill_impl(state.inner(), &id)
}

/// The retained output tail (base64) for replay on panel reopen.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn terminal_snapshot(state: tauri::State<'_, Arc<AppState>>, id: String) -> AppResult<String> {
    crate::terminal::terminal_snapshot_impl(state.inner(), &id)
}

/// Opens the vendor's own web sign-in page in the system browser — the
/// maximum login aid allowed (ADR-0003): the user completes the vendor's
/// flow; credentials never touch AI Juggler.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn login_open_web(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
) -> AppResult<()> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness.as_str()))
        .ok_or_else(|| AppError::UnknownHarness(harness.clone()))?;
    let Some(url) = adapter.login_url() else {
        return Err(AppError::Io {
            context: "open login page",
            message: format!("{harness} has no web sign-in page"),
        });
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Io {
            context: "open login page",
            message: e.to_string(),
        })
}

/// Lists Launch settings (canonical name `launch_recipes_list`;
/// legacy alias registered in the handler).
#[tauri::command(rename = "launch_recipes_list")]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn profiles_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<LaunchRecipeInfo>> {
    let storage = lock_storage(&state)?;
    profile_infos(&storage)
}

/// Creates a profile under an account.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn profile_create(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    name: String,
    model: Option<String>,
    default_cwd: Option<String>,
) -> AppResult<ProfileInfo> {
    create_profile(state.inner(), &account_id, &name, model, default_cwd)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn create_profile(
    state: &Arc<AppState>,
    account_id: &str,
    name: &str,
    model: Option<String>,
    default_cwd: Option<String>,
) -> AppResult<ProfileInfo> {
    if name.trim().is_empty() {
        return Err(AppError::Io {
            context: "create profile",
            message: "profile name must not be empty".into(),
        });
    }
    let storage = lock_storage_arc(state)?;
    let account = storage
        .accounts()
        .get(&AccountId::new(account_id))
        .map_err(|error| not_found_or_core(error, "account", account_id))?;
    let profile = Profile {
        id: ProfileId::new(format!("prof-{}", unique_suffix())),
        name: name.trim().to_string(),
        account: account.id,
        settings_overlay: SettingsOverlay {
            model,
            env: std::collections::BTreeMap::default(),
        },
        default_cwd: default_cwd.clone().map(std::path::PathBuf::from),
    };
    storage.profiles().insert(&profile)?;
    Ok(ProfileInfo {
        id: profile.id.to_string(),
        name: profile.name,
        account_id: profile.account.to_string(),
        account_label: account.label,
        provider: account.harness.to_string(),
        model: profile.settings_overlay.model,
        default_cwd,
    })
}

/// Deletes a profile (cascades to its session records).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn profile_delete(state: tauri::State<'_, Arc<AppState>>, profile_id: String) -> AppResult<()> {
    lock_storage(&state)?
        .profiles()
        .delete(&ProfileId::new(profile_id))?;
    Ok(())
}

/// Launches a session for a profile through the supervisor (one click on
/// the UI side; the only writer of harness state is the harness itself,
/// inside its managed dir).
/// The account's usage snapshot from the two-clock engine (AJ-E606):
/// the single producer of `UsageSnapshot.window` — every session
/// insert site calls this, so quota state has exactly one
/// representation. Until ingest (AJ-E607) feeds the store this is the
/// honest zero (`window: None`, default confidence) — never an
/// invented 0%. Errors degrade to the same honest zero (a broken
/// quota read must never block launching).
fn engine_usage_snapshot(
    storage: &juggler_core::storage::Storage,
    profile: &juggler_core::model::ProfileId,
) -> juggler_core::model::UsageSnapshot {
    use juggler_core::quota::{self, QuotaCap, SubscriptionClock, WindowKind};
    use juggler_core::storage::{anchor_settings_key, cap_settings_key};

    let account = match storage.profiles().get(profile) {
        Ok(recipe) => recipe.account,
        Err(_) => return juggler_core::model::UsageSnapshot::default(),
    };
    let account = account.as_str().to_string();
    let Ok(account_row) = storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(account.clone()))
    else {
        return juggler_core::model::UsageSnapshot::default();
    };
    let now_ms = Timestamp::now().millis();
    let usage_repo = storage.usage();
    let Ok(events) = usage_repo.events_for_account(&account, now_ms - quota::WEEK_MS) else {
        return juggler_core::model::UsageSnapshot::default();
    };
    let settings = storage.app_settings();
    let anchor = settings
        .get(&anchor_settings_key(&account))
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<i64>().ok());
    let samples_five_hour = usage_repo
        .samples_for_account(&account, WindowKind::FiveHour)
        .unwrap_or_default();
    let samples_weekly = usage_repo
        .samples_for_account(&account, WindowKind::Weekly { anchor_ms: anchor })
        .unwrap_or_default();
    let cap = |kind: WindowKind| -> QuotaCap {
        settings
            .get(&cap_settings_key(&account, kind))
            .ok()
            .flatten()
            .map(|raw| QuotaCap::decode(&raw))
            .unwrap_or_default()
    };
    let subscription = SubscriptionClock {
        renewal_date: account_row.renewal_date.clone(),
        entered_by_user: account_row.renewal_date.is_some(),
        from_vendor_feed: false,
    };
    let evaluation = quota::evaluate_account(quota::AccountEvaluation {
        events: &events,
        samples_five_hour: &samples_five_hour,
        samples_weekly: &samples_weekly,
        account: &account,
        cap_five_hour: cap(WindowKind::FiveHour),
        cap_weekly: cap(WindowKind::Weekly { anchor_ms: anchor }),
        weekly_anchor_ms: anchor,
        subscription: &subscription,
        now_ms,
        today: epoch_today(now_ms),
    });
    // The session snapshot carries the 5 h pool (the pool a session
    // draws down first); the weekly clock + renewal stay account-level
    // surfaces (AccountInfo / AJ-E610). Unknown/Learning carry no
    // percentage — the window stays absent rather than reading 0%.
    let five_hour = evaluation.five_hour;
    match five_hour.percentage {
        Some(used) => juggler_core::model::UsageSnapshot {
            tokens: juggler_core::model::TokenCounts::default(),
            window: Some(juggler_core::model::QuotaWindow {
                used_percentage: used,
                resets_at: five_hour
                    .resets_at
                    .map(juggler_core::model::Timestamp::from_millis),
            }),
            confidence: five_hour.confidence,
        },
        None => juggler_core::model::UsageSnapshot::default(),
    }
}

/// Today as a UTC calendar date (the engine's injected day; renewal
/// rendering stays a UI concern per AJ-E411).
fn epoch_today(now_ms: i64) -> juggler_core::renewal::CalendarDate {
    civil_from_days(now_ms.div_euclid(24 * 60 * 60 * 1000))
}

/// Howard Hinnant's `civil_from_days` — the day count back to a
/// proleptic-Gregorian date (mirrors renewal.rs's test-side inverse).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn civil_from_days(z: i64) -> juggler_core::renewal::CalendarDate {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    juggler_core::renewal::CalendarDate {
        year: if m <= 2 { y + 1 } else { y },
        month: m as u32,
        day: d as u32,
    }
}

#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn launch_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    cwd: Option<String>,
) -> AppResult<LaunchResult> {
    let result = launch_profile(state.inner(), &profile_id, cwd)?;
    emit_sessions(&app, state.inner())?;
    Ok(result)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn launch_profile(
    state: &Arc<AppState>,
    profile_id: &str,
    cwd: Option<String>,
) -> AppResult<LaunchResult> {
    use juggler_adapter::LaunchMode;

    let (profile, account) = find_profile_with_account_arc(state, profile_id)?;
    if profile.settings_overlay.model.is_some()
        && account.harness.as_str() == juggler_adapters::CLAUDE_CODE_ID
    {
        juggler_adapters::claude_code::write_settings_overlay(
            &account.state_dir,
            &profile.settings_overlay,
        )?;
    }
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let effective_cwd = cwd
        .or_else(|| {
            profile
                .default_cwd
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let spec = adapter.launch_spec(&account, &profile, &effective_cwd, &LaunchMode::Interactive)?;
    let session_id = SessionId::new(format!("sess-{}", unique_suffix()));
    // The juggling anchor: this launch's cwd becomes (or refreshes) the
    // tracked project, and the session is linked to it (ADR-0007).
    // Resolved BEFORE spawn so the ledger Intent row is attributable.
    let canonical_cwd = spec.cwd.canonicalize().unwrap_or_else(|_| spec.cwd.clone());
    let project_id = {
        let storage = lock_storage_arc(state)?;
        storage
            .projects()
            .touch(&canonical_cwd, None)?
            .id
            .to_string()
    };
    let spawned = state.supervisor.spawn(
        &session_id,
        &spec,
        &ledger_ctx(state, &account, Some(project_id.clone())),
    )?;
    {
        // The machine identity locks storage itself — resolve it
        // BEFORE the insert-scope lock (non-reentrant mutex).
        let machine = local_machine_id(state);
        let storage = lock_storage_arc(state)?;
        let project = storage.projects().touch(&canonical_cwd, None)?;
        let usage = engine_usage_snapshot(&storage, &profile.id);
        storage.sessions().insert(&SessionRecord {
            id: session_id.clone(),
            conversation: None,
            profile: profile.id,
            machine: machine.clone(),
            kind: juggler_core::model::InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: spec.cwd.clone(),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage,
            attention: None,
            running: None,
            // Terminal Instances own no window identity (AJ-E414).
            window: None,
            transcript_ref: None,
        })?;
    };
    Ok(LaunchResult {
        session_id: session_id.to_string(),
        pid: spawned.pid,
    })
}

/// Launches the Provider's desktop app under an Account and tracks it
/// as a desktop-kind Instance in the same ledger (AJ-E413). Capability-
/// gated: providers without `DESKTOP_APP` are refused by name.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the provider ships no
/// desktop app or the process cannot start.
#[tauri::command]
pub fn desktop_launch(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<DesktopLaunchInfo> {
    let result = desktop_launch_arc(state.inner(), &account_id)?;
    emit_sessions(&app, state.inner())?;
    Ok(result)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the provider ships no
/// desktop app, the account lacks launch settings (the owning recipe is
/// the storage anchor), or the process cannot start.
pub fn desktop_launch_arc(state: &Arc<AppState>, account_id: &str) -> AppResult<DesktopLaunchInfo> {
    let (account, recipe_id) = {
        let storage = lock_storage_arc(state)?;
        let account = storage
            .accounts()
            .get(&AccountId::new(account_id.to_string()))?;
        // Desktop instances anchor to the account's launch settings row
        // (the storage FK); the overlay itself is not applied — env
        // isolation is what the DESKTOP adapter declares, honestly.
        let recipe = storage
            .profiles()
            .list_by_account(&account.id)?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound {
                entity: "launch settings",
                id: account.id.to_string(),
            })?;
        (account, recipe.id)
    };
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    if !adapter
        .capabilities()
        .contains(juggler_adapter::Capabilities::DESKTOP_APP)
    {
        return Err(AppError::Validation {
            subject: "desktop launch",
            message: format!("provider {} ships no desktop app", account.harness.as_str()),
        });
    }
    // Idempotent account-dir preparation (the same contract terminal
    // launches honour) — the launch cwd must exist before spawn.
    adapter.prepare_account(&account.state_dir)?;
    let launch = adapter.desktop_launch(&account)?;
    let session_id = SessionId::new(format!("sess-{}", unique_suffix()));
    let project_id = {
        let storage = lock_storage_arc(state)?;
        storage.projects().touch(&launch.cwd, None)?.id.to_string()
    };
    let spawned = state.supervisor.spawn_desktop(
        &session_id,
        &launch,
        &ledger_ctx(state, &account, Some(project_id)),
    )?;
    {
        // The machine identity locks storage itself — resolve it
        // BEFORE the insert-scope lock (non-reentrant mutex).
        let machine = local_machine_id(state);
        let storage = lock_storage_arc(state)?;
        let project = storage.projects().touch(&launch.cwd, None)?;
        let usage = engine_usage_snapshot(&storage, &recipe_id);
        storage.sessions().insert(&SessionRecord {
            id: session_id.clone(),
            conversation: None,
            profile: recipe_id,
            machine: machine.clone(),
            kind: juggler_core::model::InstanceKind::Desktop,
            project: Some(project.id.clone()),
            cwd: launch.cwd.clone(),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage,
            attention: None,
            running: None,
            // Window identity recorded at launch (AJ-E414 Req.2): the
            // owning PID + bundle id; the title is adopted at first
            // sight (first focus attempt) and re-discovered thereafter.
            window: Some(juggler_core::model::WindowIdentity {
                pid: spawned.pid,
                bundle_id: launch.bundle_id.clone(),
                title: None,
                cgwindow_id: None,
            }),
            transcript_ref: None,
        })?;
    }
    Ok(DesktopLaunchInfo {
        session_id: session_id.to_string(),
        pid: spawned.pid,
        bundle_id: launch.bundle_id,
        isolation_label: launch.isolation_label.to_string(),
    })
}

/// A focus attempt's honest result (AJ-E414 Req.5): the copy-exact
/// outcome label, whether the Accessibility grant was missing (the
/// attempt degraded to app activation), and the window title the
/// attempt targeted (after first-sight adoption / re-discovery).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FocusOutcomeInfo {
    /// `Focused window` | `App opened — window not found` |
    /// `App not running — Relaunch?` | platform-unavailable note.
    pub outcome: String,
    /// True when tiers (b)/(c) were impossible without the
    /// Accessibility permission — the UI shows its explainer.
    pub permission_missing: bool,
    /// The window title this attempt targeted, once known.
    pub window_title: Option<String>,
}

/// The window identity for a desktop Instance: recorded at launch, or
/// synthesized for pre-v10 rows (bundle id from the adapter spec; pid 0
/// forces re-discovery). Shared by the focus tiers and the preview.
fn desktop_window_identity(
    state: &Arc<AppState>,
    record: &juggler_core::model::InstanceRecord,
) -> AppResult<juggler_core::model::WindowIdentity> {
    if let Some(identity) = record.window.clone() {
        return Ok(identity);
    }
    let bundle_id = {
        let storage = lock_storage_arc(state)?;
        let account = storage
            .accounts()
            .get(&storage.profiles().get(&record.profile)?.account)?;
        state
            .registry
            .get(&account.harness)
            .and_then(|adapter| adapter.desktop_launch(&account).ok())
            .and_then(|launch| launch.bundle_id)
    };
    Ok(juggler_core::model::WindowIdentity {
        pid: 0,
        bundle_id,
        title: None,
        cgwindow_id: None,
    })
}

/// Runs the AJ-E414 focus tiers for one desktop Instance: activate the
/// app, match the recorded window identity, clear `AXMinimized`, then
/// `AXRaise` (see `focus.rs` for the mechanism pin). The outcome says
/// exactly which tier ran — app activation alone is never reported as
/// window focus success.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Instance is unknown
/// or not desktop-kind (focus owns windows, terminals own panels).
pub fn desktop_focus(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<FocusOutcomeInfo> {
    desktop_focus_with(
        state.inner(),
        &session_id,
        std::sync::Arc::new(focus::OsascriptRunner),
    )
}

/// The testable core (tests inject a scripted runner).
///
/// # Errors
///
/// Same as [`desktop_focus`].
pub fn desktop_focus_with(
    state: &Arc<AppState>,
    session_id: &str,
    runner: std::sync::Arc<dyn focus::ScriptRunner>,
) -> AppResult<FocusOutcomeInfo> {
    let mut record = {
        let storage = lock_storage_arc(state)?;
        storage
            .sessions()
            .get(&SessionId::new(session_id.to_string()))?
    };
    if record.kind != juggler_core::model::InstanceKind::Desktop {
        return Err(AppError::Validation {
            subject: "desktop focus",
            message: "window focus applies to desktop instances".into(),
        });
    }
    if is_terminal(record.status) {
        return Ok(FocusOutcomeInfo {
            outcome: focus::OUTCOME_NOT_RUNNING.to_string(),
            permission_missing: false,
            window_title: record.window.as_ref().and_then(|w| w.title.clone()),
        });
    }
    let identity = desktop_window_identity(state, &record)?;
    // The tier engine is platform-independent over the runner seam
    // (CI drives it everywhere); the platform gate lives in the
    // command wrapper above.
    let attempt = focus::FocusService::new(runner).attempt(&identity);
    // Persist the (possibly enriched) identity — first-sight titles and
    // re-discovered PIDs survive the attempt (Req.2).
    if attempt.identity != record.window {
        record.window.clone_from(&attempt.identity);
        let storage = lock_storage_arc(state)?;
        storage.sessions().update(&record)?;
    }
    Ok(FocusOutcomeInfo {
        outcome: attempt.outcome.to_string(),
        permission_missing: attempt.permission_missing,
        window_title: attempt
            .identity
            .as_ref()
            .and_then(|identity| identity.title.clone()),
    })
}

/// Wire payload for one preview frame (AJ-E415): a base64 PNG of the
/// Instance's window plus its capture time (frame age) and sequence.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PreviewFrameInfo {
    /// Owning Instance id.
    pub session_id: String,
    /// Base64-encoded PNG bytes of the captured window.
    pub png_b64: String,
    /// Capture time, epoch milliseconds (the panel labels frame age).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Monotonic frame sequence.
    #[ts(type = "number")]
    pub seq: u64,
}

/// Wire payload for a preview phase change (AJ-E415): the phase name
/// (`live|paused|denied|unsupported`) and the copy-exact panel text.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PreviewStateInfo {
    /// Owning Instance id.
    pub session_id: String,
    /// `live` | `paused` | `denied` | `unsupported`.
    pub phase: String,
    /// Copy-exact text the panel renders for the phase (empty for live).
    pub text: String,
}

/// The one live preview session (Req.5: one at a time — the selected
/// Instance). Stop is a flag + a done channel (bounded join).
pub struct PreviewSession {
    /// The Instance whose window is mirrored.
    pub session_id: String,
    /// Stop flag (the loop polls it every tick).
    pub stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Completes when the loop thread ended.
    pub done: std::sync::mpsc::Receiver<()>,
}

/// Translates one driver event into the wire events (`preview://frame`
/// / `preview://state`) with the copy-exact phase texts.
fn emit_preview_event(app: &tauri::AppHandle, event: crate::preview::PreviewEvent) {
    use tauri::Emitter;
    match event {
        crate::preview::PreviewEvent::Frame {
            session_id,
            png_b64,
            at_ms,
            seq,
        } => {
            let _ = app.emit(
                crate::EVENT_PREVIEW_FRAME,
                PreviewFrameInfo {
                    session_id,
                    png_b64,
                    at_ms,
                    seq,
                },
            );
        }
        crate::preview::PreviewEvent::Phase { session_id, phase } => {
            let text = match phase {
                crate::preview::PreviewPhase::Live => String::new(),
                crate::preview::PreviewPhase::Paused => {
                    crate::preview::PAUSED_NOT_VISIBLE.to_string()
                }
                crate::preview::PreviewPhase::Denied => {
                    "Screen Recording permission needed — grant it in System Settings, then re-enable preview"
                        .to_string()
                }
                crate::preview::PreviewPhase::Unsupported => {
                    // D-17 unblocked macOS; this phase is now only the
                    // honest non-macOS answer.
                    "Live preview is not supported on this platform".to_string()
                }
            };
            let _ = app.emit(
                crate::EVENT_PREVIEW_STATE,
                PreviewStateInfo {
                    session_id,
                    phase: phase.as_str().to_string(),
                    text,
                },
            );
        }
    }
}

/// Stops any running preview and waits ≤900 ms for the thread (the
/// poll cadence bounds the wait; a late exit leaks nothing — one
/// capture per poll, nothing persistent).
fn stop_preview_session(state: &Arc<AppState>) {
    let session = state
        .preview
        .lock()
        .map_err(|_| AppError::StatePoisoned)
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(session) = session {
        session
            .stop
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if session
            .done
            .recv_timeout(Duration::from_millis(900))
            .is_err()
        {
            eprintln!(
                "ai-juggler: preview thread for {} outlived the stop budget",
                session.session_id
            );
        }
    }
}

/// Starts the read-only live preview for one desktop Instance
/// (AJ-E415): strictly one at a time (the previous stream stops
/// first); requires the Account's explicit preview opt-in; frames and
/// phase changes arrive as `preview://frame` / `preview://state`
/// events. Read-only by construction — pixels flow app → panel only.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message for a non-desktop Instance,
/// a missing opt-in, or an unknown id.
pub fn preview_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;

    let (record, identity) = {
        let storage = lock_storage(&state)?;
        let record = storage
            .sessions()
            .get(&SessionId::new(session_id.clone()))?;
        if record.kind != juggler_core::model::InstanceKind::Desktop {
            return Err(AppError::Validation {
                subject: "live preview",
                message: "preview mirrors desktop instance windows".into(),
            });
        }
        if is_terminal(record.status) {
            return Ok(()); // nothing to mirror; the panel renders status only
        }
        let identity = desktop_window_identity(state.inner(), &record)?;
        (record, identity)
    };
    // The Account's explicit consent gate (Req.3) — default off.
    {
        let storage = lock_storage(&state)?;
        let recipe = storage.profiles().get(&record.profile)?;
        let account = storage.accounts().get(&recipe.account)?;
        if !account.preview_opt_in {
            return Err(AppError::Validation {
                subject: "live preview",
                message: format!(
                    "preview is off for {} — enable it in Account Actions",
                    account.label
                ),
            });
        }
    }
    stop_preview_session(state.inner());

    let (done_tx, done_rx) = mpsc::channel();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let session = session_id.clone();
    let live_id = SessionId::new(session_id.clone());
    let state_for_thread = Arc::clone(state.inner());
    let app = app.clone();
    std::thread::spawn(move || {
        let emit = move |event: crate::preview::PreviewEvent| {
            emit_preview_event(&app, event);
        };
        // D-17: the REAL capture source on macOS — the audited init
        // shim (crates/juggler-scap-init) satisfies the binding's init
        // requirement inside the workspace's unsafe-free law. Other
        // platforms keep the honest unsupported source.
        #[cfg(target_os = "macos")]
        let source: std::sync::Arc<dyn crate::preview::CaptureSource> =
            std::sync::Arc::new(crate::preview::ScreenshotSource);
        #[cfg(not(target_os = "macos"))]
        let source: std::sync::Arc<dyn crate::preview::CaptureSource> =
            std::sync::Arc::new(crate::preview::UnsupportedSource);
        let driver = crate::preview::PreviewDriver {
            session_id: session.as_str(),
            identity,
            source: source.as_ref(),
            is_live: &|| state_for_thread.supervisor.is_supervised(&live_id),
            emit: &emit,
            stopped: stop_flag.as_ref(),
        };
        driver.run();
        let _ = done_tx.send(());
    });
    state
        .preview
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .replace(PreviewSession {
            session_id,
            stop,
            done: done_rx,
        });
    Ok(())
}

/// Stops the live preview (panel close / selection switch). Bounded
/// teardown: the stop flag ends the loop within one poll interval.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on state poisoning.
pub fn preview_stop(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    stop_preview_session(state.inner());
    Ok(())
}

/// Sets the per-Account live-preview opt-in (Req.3's permanent off
/// switch; enabling is the Screen-Recording consent moment — the first
/// start triggers the system prompt at that point, never before).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown.
pub fn account_set_preview_opt_in(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    let account = {
        let storage = lock_storage(&state)?;
        let mut account = storage
            .accounts()
            .get(&AccountId::new(account_id.clone()))?;
        account.preview_opt_in = opt_in;
        storage.accounts().update(&account)?;
        account
    };
    Ok(account_info_of(&account))
}

/// Reads one app-level setting (schema v10 kv); `None` = never set.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub fn app_setting_get(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
) -> AppResult<Option<String>> {
    let storage = lock_storage(&state)?;
    storage.app_settings().get(&key).map_err(AppError::from)
}

/// Writes one app-level setting (empty value = reset to default).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub fn app_setting_set(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
    value: String,
) -> AppResult<()> {
    let storage = lock_storage(&state)?;
    storage
        .app_settings()
        .set(&key, &value)
        .map_err(AppError::from)
}

/// Opens the system Accessibility privacy pane (the deep link behind
/// the AJ-E414 permission explainer; the link is fixed — never a
/// caller-supplied URL).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the pane cannot open.
pub fn open_accessibility_settings(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            None::<&str>,
        )
        .map_err(|e| AppError::Io {
            context: "open Accessibility settings",
            message: e.to_string(),
        })
}

/// Rebinds the global "Go to instance…" hotkey (AJ-E414 Req.4). The
/// new binding is registered with the OS FIRST; only a successful
/// registration persists it — and a failed registration restores the
/// previous one, so the user is never left without a hotkey.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message for an empty binding or an
/// OS registration failure (conflict, unparseable accelerator).
pub fn go_to_hotkey_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    hotkey: String,
) -> AppResult<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let hotkey = hotkey.trim().to_string();
    if hotkey.is_empty() {
        return Err(AppError::Validation {
            subject: "hotkey",
            message: "empty binding — the default is restored by setting it explicitly".into(),
        });
    }
    let previous = {
        let storage = lock_storage(&state)?;
        storage
            .app_settings()
            .get(crate::GO_TO_HOTKEY_KEY)
            .map_err(AppError::from)?
    };
    let shortcuts = app.global_shortcut();
    if let Some(previous) = &previous {
        let _ = shortcuts.unregister(previous.as_str());
    }
    match shortcuts.register(hotkey.as_str()) {
        Ok(()) => {
            let storage = lock_storage(&state)?;
            storage
                .app_settings()
                .set(crate::GO_TO_HOTKEY_KEY, &hotkey)
                .map_err(AppError::from)?;
            Ok(())
        }
        Err(error) => {
            // Best-effort restore of the previous binding.
            if let Some(previous) = &previous {
                let _ = shortcuts.register(previous.as_str());
            }
            Err(AppError::Io {
                context: "register hotkey",
                message: format!("{hotkey}: {error}"),
            })
        }
    }
}

/// Lists Instances (board feed), newest first (canonical name
/// `instances_list`; legacy alias registered in the handler).
#[tauri::command(rename = "instances_list")]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn sessions_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<InstanceInfo>> {
    // Collector lock first and alone: the resource tick shares it, and no
    // code path ever holds the storage and collector locks together.
    let live = live_stats_of(state.inner());
    let labels = desktop_isolation_labels(&state.registry);
    let adopted = state.supervisor.alive_adopted_sessions();
    let storage = lock_storage(&state)?;
    session_infos(&storage, &live, &labels, &adopted)
}

/// Stops a session via the supervisor ladder (grace, then kill).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn stop_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    state
        .supervisor
        .stop(&SessionId::new(session_id), Duration::from_secs(5))?;
    emit_sessions(&app, state.inner())?;
    Ok(())
}

/// Dashboard counts.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn dashboard_stats(state: tauri::State<'_, Arc<AppState>>) -> AppResult<StatsInfo> {
    let ledger_degraded = state.supervisor.ledger_degraded();
    let storage = lock_storage(&state)?;
    let accounts = storage.accounts().list()?;
    let mut profiles = 0_u64;
    for account in &accounts {
        profiles += u64::try_from(storage.profiles().list_by_account(&account.id)?.len())
            .unwrap_or_default();
    }
    let active = storage
        .sessions()
        .list(None)?
        .into_iter()
        .filter(|record| {
            matches!(
                record.status,
                SessionStatus::Starting | SessionStatus::Running | SessionStatus::WaitingUser
            )
        })
        .count();
    Ok(StatsInfo {
        accounts: u64::try_from(accounts.len()).unwrap_or_default(),
        profiles,
        active_sessions: u64::try_from(active).unwrap_or_default(),
        ledger_degraded,
    })
}

// ---------------------------------------------------------------------------
// lifecycle forwarding

/// Applies a supervisor lifecycle event to storage (status transitions).
/// Returns the updated record when one matched.
pub fn apply_lifecycle_event(
    storage: &Storage,
    event: LifecycleEvent,
    stopped_by_user: bool,
    quota_blocked: Option<&juggler_core::model::AttentionReason>,
) -> Option<SessionRecord> {
    let session_id = match &event {
        LifecycleEvent::Started { session, .. }
        | LifecycleEvent::Exited { session, .. }
        | LifecycleEvent::HelperOrphaned { session, .. } => session,
    };
    let mut record = storage.sessions().get(session_id).ok()?;
    match event {
        LifecycleEvent::Started { .. } => {
            record.status = SessionStatus::Running;
            record.attention = None;
        }
        // AJ-E504: helper-level delta — the SESSION record does not
        // change (the root still owns the row state); the Cleanup
        // Centre (AJ-E506) consumes these via the ledger.
        LifecycleEvent::HelperOrphaned { .. } => return None,
        LifecycleEvent::Exited { kind, .. } => {
            record.status = if stopped_by_user {
                // The stop ladder (PTY exits surface kills as plain
                // non-zero codes) — user intent, not a failure.
                SessionStatus::Stopped
            } else if let Some(reason) = quota_blocked {
                // AJ-E405: non-zero exit with a rate-limit signature —
                // QuotaBlocked (immediate, no debounce) with the reason.
                record.attention = Some(reason.clone());
                SessionStatus::QuotaBlocked
            } else {
                match kind {
                    kind if kind.is_success() => SessionStatus::Finished,
                    juggler_supervisor::ExitKind::Signaled(_) => SessionStatus::Stopped,
                    juggler_supervisor::ExitKind::Code(_) => SessionStatus::Failed,
                }
            };
            if record.status != SessionStatus::QuotaBlocked {
                record.attention = None;
            }
        }
    }
    record.last_activity = Some(Timestamp::now());
    storage.sessions().update(&record).ok()?;
    Some(record)
}

/// Records one heuristic transition (status + structured reason) and
/// returns the updated record. No action ever follows (SPEC §7).
pub fn apply_attention_transition(
    storage: &Storage,
    session: &SessionId,
    transition: &juggler_supervisor::AttentionTransition,
) -> Option<SessionRecord> {
    let mut record = storage.sessions().get(session).ok()?;
    record.status = transition.status;
    record.attention = Some(transition.reason.clone());
    storage.sessions().update(&record).ok()?;
    Some(record)
}

/// Clears attention and returns the record to `Running` — the recovery
/// path (output resumed; AJ-E405 Req.5).
pub fn apply_attention_recovery(storage: &Storage, session: &SessionId) -> Option<SessionRecord> {
    let mut record = storage.sessions().get(session).ok()?;
    if !matches!(
        record.status,
        SessionStatus::Stuck | SessionStatus::WaitingUser | SessionStatus::QuotaBlocked
    ) {
        return None;
    }
    record.status = SessionStatus::Running;
    record.attention = None;
    record.last_activity = Some(Timestamp::now());
    storage.sessions().update(&record).ok()?;
    Some(record)
}

/// One 15 s heuristic cycle (AJ-E405): evaluates every active Instance
/// against the supervisor's measured inputs and records + emits any
/// confirmed transitions. Purely passive — no action follows.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn heuristic_tick(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    let engine = &state.heuristics;
    let live = live_stats_of(state);
    let mut changed = false;
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        for record in storage.sessions().list(None)? {
            if !is_active(record.status) {
                engine.forget(&record.id);
                continue;
            }
            let now = Timestamp::now().millis();
            let transcript_mtime_ms = record.transcript_ref.as_ref().and_then(|tr| {
                std::fs::metadata(&tr.path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
            });
            let tail = state
                .supervisor
                .output_tail(&record.id, 2048)
                .unwrap_or_default();
            let inputs = juggler_supervisor::HeuristicInputs {
                last_output_ms: state.supervisor.last_output_ms(&record.id),
                cpu_pct: live
                    .get(&record.id.to_string())
                    .and_then(|stats| stats.cpu_pct),
                transcript_mtime_ms,
                prompt_alive: juggler_supervisor::prompt_alive(&tail),
                now_ms: now,
            };
            if let Some(transition) = engine.evaluate(&record.id, &inputs) {
                apply_attention_transition(&storage, &record.id, &transition);
                changed = true;
            }
        }
    }
    if changed {
        emit_sessions(app, state)?;
    }
    Ok(())
}

/// Event-driven recovery drain (Req.5, <300 ms): every session with
/// fresh output that currently sits in a heuristic state returns to
/// `Running` and the UI is told immediately.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn attention_recovery_drain(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    let mut changed = false;
    let drained: Vec<SessionId> = state.supervisor.drain_activity();
    for session in &drained {
        if let Some(running) = state.heuristics.recover(session) {
            debug_assert_eq!(running, SessionStatus::Running);
            {
                let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
                if apply_attention_recovery(&storage, session).is_some() {
                    changed = true;
                }
            }
        }
    }
    if changed {
        emit_sessions(app, state)?;
    }
    // AJ-E412 Req.2: the same fresh-output signal drives transcript
    // tail-scans for the running model (only the active Instances) —
    // the event path keeps the <300 ms budget; `detect_running_models`
    // emits `sessions_updated` itself when a model changed.
    if !drained.is_empty() {
        detect_running_models(app, state, Some(&drained));
    }
    Ok(())
}

/// The model a transcript line records for its turn — top-level
/// `model` or nested `message.model` (both shapes ship in the wild).
fn line_model(value: &serde_json::Value) -> Option<String> {
    value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            value
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(serde_json::Value::as_str)
        })
        .map(str::to_string)
}

/// Aggregated transcript facts feeding the quality signals.
struct TranscriptScan {
    events: Vec<juggler_core::quality::UsageEvent>,
    errors: u64,
    turns: u64,
    context_used: Option<u64>,
    context_window: Option<u64>,
}

/// Scans the native transcript JSONL for per-turn usage events,
/// timestamps, error markers and the last reported context pair.
/// Fixture-compatible shapes; Claude transcripts carry `timestamp` +
/// `usage` on assistant lines.
fn scan_transcript(record: &SessionRecord, now: i64) -> TranscriptScan {
    let mut scan = TranscriptScan {
        events: Vec::new(),
        errors: 0,
        turns: 0,
        context_used: None,
        context_window: None,
    };
    let bytes = record
        .transcript_ref
        .as_ref()
        .and_then(|transcript| std::fs::read(&transcript.path).ok());
    let Some(bytes) = bytes else {
        return scan;
    };
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("usage").is_none() {
            continue;
        }
        scan.turns += 1;
        let usage = &value["usage"];
        let tokens = juggler_core::model::TokenCounts {
            input: usage["input_tokens"].as_u64().unwrap_or(0),
            output: usage["output_tokens"].as_u64().unwrap_or(0),
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        };
        // Wall clock: RFC3339 `timestamp` when present, else `now`
        // (honest single-point fallback → the per-minute bars degrade
        // to one bucket, never fabricated).
        let at_ms = value["timestamp"]
            .as_str()
            .and_then(chrono_like_ms)
            .unwrap_or(now);
        scan.events.push(juggler_core::quality::UsageEvent {
            at_ms,
            tokens,
            model: line_model(&value),
        });
        if value.get("error").is_some() {
            scan.errors += 1;
        }
        if let Some(used) = value["context_tokens"].as_u64() {
            scan.context_used = Some(used);
            scan.context_window = value["context_window"].as_u64();
        }
    }
    scan
}

/// Computes the D-07 quality signals for one Instance (AJ-E409): reads
/// the native transcript when present (per-turn usage + timestamps +
/// error markers); every field is a labelled signal or its honest
/// empty — never a composite score.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn instance_quality(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<QualitySignalsInfo> {
    let storage = lock_storage(&state)?;
    let record = storage
        .sessions()
        .get(&SessionId::new(session_id.clone()))
        .map_err(AppError::from)?;
    let now = Timestamp::now().millis();
    let scan = scan_transcript(&record, now);

    let tpm = juggler_core::quality::signals::tokens_per_min(&scan.events, now);
    let error_label = if scan.turns == 0 {
        "Not measurable (interactive)".to_string()
    } else {
        format!("{} errors / {} turns", scan.errors, scan.turns)
    };
    let table = juggler_core::quality::load_pricing_table();
    let recipe = storage.profiles().get(&record.profile).ok();
    let provider = storage
        .accounts()
        .list()
        .ok()
        .and_then(|accounts| {
            accounts
                .iter()
                .find(|a| recipe.as_ref().is_some_and(|r| r.account == a.id))
                .map(|a| a.harness.to_string())
        })
        .unwrap_or_default();
    // AJ-E412 Req.4: price per turn where the transcript records the
    // model; turns without one stay in the token totals but are
    // excluded from $. Falls back to tokens-only when no turn was
    // priceable (never to the launch overlay — that is intent, not
    // telemetry).
    let (cost, priced_model) = match juggler_core::quality::pricing::session_cost_per_turn(
        &table,
        &provider,
        &scan.events,
    ) {
        Some((total, models)) => (Some(total), Some(models.join(", "))),
        None => (None, None),
    };
    // Forecast: from quota deltas — none are observable yet, so idle
    // honestly until the usage engine (AJ-E606+) feeds burn rates.
    let forecast = juggler_core::quality::quota_forecast_label(
        record
            .usage
            .window
            .as_ref()
            .map_or(0.0, |w| w.used_percentage),
        0.0,
    );
    Ok(QualitySignalsInfo {
        tokens_bars: tpm.bars.map(|bars| bars.to_vec()),
        avg_tokens_per_min: tpm.avg_per_min,
        error_label,
        errors_not_measurable: scan.turns == 0,
        context_used: scan.context_used,
        context_window: scan.context_window,
        cumulative: TokenCountsInfo {
            input: record.usage.tokens.input,
            output: record.usage.tokens.output,
            cache_read: record.usage.tokens.cache_read,
            cache_write: record.usage.tokens.cache_write,
            reasoning: record.usage.tokens.reasoning,
        },
        session_cost_usd: cost,
        priced_model: priced_model.filter(|_| cost.is_some()),
        quota_forecast: forecast,
        stuck: record.attention.as_ref().map(|reason| AttentionInfo {
            rule: reason.rule.clone(),
            inputs: reason.inputs.clone(),
            since_ms: reason.since_ms,
        }),
    })
}

/// RFC3339-ish timestamp → epoch ms without pulling chrono (the
/// transcripts use `2026-10-13T12:34:56.789Z`-style stamps).
// Fractional seconds land in f64; the ms truncation is below the
// per-minute bar resolution we compute from it.
#[allow(clippy::cast_possible_truncation)]
fn chrono_like_ms(raw: &str) -> Option<i64> {
    let date_time = raw.split('T').nth(1)?.split(['Z', '+']).next()?;
    let mut parts = date_time.split(':');
    let h: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    let day_ms = raw.strip_suffix('Z').map(str::to_string).and_then(|s| {
        let date = s.split('T').next()?.to_string();
        let mut d = date.split('-');
        let y: i64 = d.next()?.parse().ok()?;
        let mo: i64 = d.next()?.parse().ok()?;
        let da: i64 = d.next()?.parse().ok()?;
        Some(days_from_civil(y, mo, da) * 86_400_000)
    })?;
    Some(day_ms + h * 3_600_000 + m * 60_000 + (sec * 1000.0) as i64)
}

/// Days since epoch (Howard Hinnant's civil-from-days inverse).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Emits the refreshed session list to the UI.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn emit_sessions(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    use tauri::Emitter;
    let live = live_stats_of(state);
    let sessions = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        session_infos(
            &storage,
            &live,
            &desktop_isolation_labels(&state.registry),
            &state.supervisor.alive_adopted_sessions(),
        )?
    };
    let _ = app.emit(crate::EVENT_SESSIONS_UPDATED, sessions);
    Ok(())
}

/// One batched sample of live process stats for every supervised session
/// (locks only the collector — never together with the storage lock).
fn live_stats_of(state: &Arc<AppState>) -> LiveStatsMap {
    let Ok(mut collector) = state.resources.lock() else {
        // Poisoned collector: report nothing rather than fabricated zeros.
        return LiveStatsMap::new();
    };
    collector
        .refresh(&state.supervisor)
        .into_iter()
        .map(|(session, stats)| (session.to_string(), stats))
        .collect()
}

/// Emits the 5 s resource delta (I-07/I-08) on `session://resources`:
/// one coalesced payload per tick carrying every active Instance, and no
/// emission at all while nothing is live (idle costs zero wakeups for
/// the UI, AJ-E401 Req.4).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn emit_resources(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    use tauri::Emitter;
    let live = live_stats_of(state);
    let payload: Vec<InstanceResourcesInfo> = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| is_active(record.status))
            .filter_map(|record| {
                // Unsampled rows are omitted — the UI keeps its honest
                // `null` instead of inheriting a zero.
                let sample = live.get(&record.id.to_string()).copied()?;
                Some(InstanceResourcesInfo {
                    id: record.id.to_string(),
                    cpu_pct: sample.cpu_pct,
                    rss_bytes: Some(sample.rss_bytes),
                    child_count: Some(sample.child_count),
                })
            })
            .collect()
    };
    if payload.is_empty() {
        return Ok(());
    }
    let _ = app.emit(crate::EVENT_SESSIONS_RESOURCES, payload);
    Ok(())
}

/// Emits the 30 s quota delta (I-05) on `session://quota`: one coalesced
/// payload per tick; per-Instance `quota` is `null` until a feed exists
/// (the two-clock engine, AJ-E606+, populates it later — the tick itself
/// is the contract this task ships).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn emit_quota(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    use tauri::Emitter;
    let payload: Vec<InstanceQuotaInfo> = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| is_active(record.status))
            .map(|record| InstanceQuotaInfo {
                id: record.id.to_string(),
                quota: quota_snapshot_of(&record.usage),
            })
            .collect()
    };
    if payload.is_empty() {
        return Ok(());
    }
    let _ = app.emit(crate::EVENT_SESSIONS_QUOTA, payload);
    Ok(())
}

// ---------------------------------------------------------------------------
// helpers

fn lock_storage_arc(state: &Arc<AppState>) -> Result<std::sync::MutexGuard<'_, Storage>, AppError> {
    state.storage.lock().map_err(|_| AppError::StatePoisoned)
}

fn lock_storage<'a>(
    state: &'a tauri::State<'_, Arc<AppState>>,
) -> Result<std::sync::MutexGuard<'a, Storage>, AppError> {
    state.storage.lock().map_err(|_| AppError::StatePoisoned)
}

fn account_info_of(account: &Account) -> AccountInfo {
    AccountInfo {
        login_state: login_state_name(login_state_of(account)).to_string(),
        id: account.id.to_string(),
        provider: account.harness.to_string(),
        label: account.label.clone(),
        plan: account.plan.as_ref().map(ToString::to_string),
        cleanup_opt_in: account.cleanup_opt_in,
        renewal_date: account.renewal_date.clone(),
        preview_opt_in: account.preview_opt_in,
        import_source_path: account.import_source_path.clone(),
        import_origin: account.import_origin.clone(),
        imported_at: account.imported_at,
    }
}

/// Wire-format name for a scan/import login probe.
fn login_probe_name(probe: juggler_adapter::LoginProbe) -> String {
    match probe {
        juggler_adapter::LoginProbe::LoggedIn => "logged_in",
        juggler_adapter::LoginProbe::LoggedOut => "logged_out",
    }
    .to_string()
}

/// Wire-format name for a login state — the AJ-E308 Req.1 taxonomy
/// (`signed_in|partial|expired|sso_pending|api_key|logged_out|unknown`).
/// `unknown` renders "sign in when online", NEVER as logged out.
fn login_state_name(state: juggler_adapters::LoginState) -> &'static str {
    match state {
        juggler_adapters::LoginState::SignedIn => "signed_in",
        juggler_adapters::LoginState::Partial => "partial",
        juggler_adapters::LoginState::Expired => "expired",
        juggler_adapters::LoginState::SsoPending => "sso_pending",
        juggler_adapters::LoginState::ApiKey => "api_key",
        juggler_adapters::LoginState::LoggedOut => "logged_out",
        juggler_adapters::LoginState::Unknown => "unknown",
    }
}

fn login_state_of(account: &Account) -> juggler_adapters::LoginState {
    if account.harness.as_str() == juggler_adapters::CLAUDE_CODE_ID {
        return juggler_adapters::ClaudeCodeAdapter::new().login_state(&account.state_dir);
    }
    juggler_adapters::LoginState::LoggedOut
}

/// Profile rows with denormalized account labels/harnesses.
fn profile_infos(storage: &Storage) -> AppResult<Vec<ProfileInfo>> {
    let accounts: std::collections::BTreeMap<String, (String, String)> = storage
        .accounts()
        .list()?
        .into_iter()
        .map(|account| {
            (
                account.id.to_string(),
                (account.label, account.harness.to_string()),
            )
        })
        .collect();
    let mut out = Vec::new();
    for (account_id, (label, harness)) in &accounts {
        for profile in storage
            .profiles()
            .list_by_account(&AccountId::new(account_id.as_str()))?
        {
            out.push(ProfileInfo {
                id: profile.id.to_string(),
                name: profile.name.clone(),
                account_id: profile.account.to_string(),
                account_label: label.clone(),
                provider: harness.clone(),
                model: profile.settings_overlay.model,
                default_cwd: profile
                    .default_cwd
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned()),
            });
        }
    }
    Ok(out)
}

/// Live supervisor stats keyed by Instance id (empty until the collector
/// has sampled; historical rows never have entries).
pub type LiveStatsMap = std::collections::HashMap<String, juggler_supervisor::LiveProcessStats>;

/// I-05 conversion shared by the row payload and the 30 s quota tick.
fn quota_snapshot_of(usage: &juggler_core::model::UsageSnapshot) -> Option<QuotaSnapshotInfo> {
    usage.window.map(|window| QuotaSnapshotInfo {
        used_percentage: window.used_percentage,
        resets_at_ms: window.resets_at.map(Timestamp::millis),
        confidence: confidence_name(usage.confidence).to_string(),
    })
}

/// Wire name for a usage-confidence level (SPEC §6 honesty rule).
fn confidence_name(confidence: juggler_core::model::Confidence) -> &'static str {
    match confidence {
        juggler_core::model::Confidence::Measured => "measured",
        juggler_core::model::Confidence::Computed => "computed",
        juggler_core::model::Confidence::Estimated => "estimated",
    }
}

/// Wire value for the I-10 attention reason derived from lifecycle status
/// (AJ-E405 replaces the derivation with structured heuristic reasons).
fn needs_attention_of(status: SessionStatus) -> Option<NeedsAttention> {
    match status {
        SessionStatus::WaitingUser => Some(NeedsAttention::WaitingUser),
        SessionStatus::Stuck => Some(NeedsAttention::Stuck),
        SessionStatus::QuotaBlocked => Some(NeedsAttention::QuotaBlocked),
        SessionStatus::Failed => Some(NeedsAttention::Failed),
        SessionStatus::Starting
        | SessionStatus::Running
        | SessionStatus::Finished
        | SessionStatus::Stopped
        | SessionStatus::HandedOver => None,
    }
}

/// I-09: branch name read from `<cwd>/.git/HEAD` — one file read, never a
/// `git` subprocess on the hot path (AJ-E401 performance budget). Detached
/// heads and worktree/gitfile layouts return `None`: unknown, not guessed.
fn git_branch_of(cwd: &std::path::Path) -> Option<String> {
    let head = std::fs::read_to_string(cwd.join(".git").join("HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

/// A terminal status: the process is gone, so nothing about it is live.
fn is_terminal(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Finished
            | SessionStatus::Stopped
            | SessionStatus::Failed
            | SessionStatus::HandedOver
    )
}

/// An active status: the row carries live-state expectations (resource
/// ticks and quota ticks only ever emit for these).
fn is_active(status: SessionStatus) -> bool {
    !is_terminal(status)
}

/// Resolves the account/provider join for one Instance row. When the
/// launch-recipe or account row is gone (defensive only — storage FKs
/// prevent it today), the Provider and label degrade to a generic
/// `unknown` marker, never a blank string, and the account id to `null`.
fn resolved_join(
    recipe_index: &std::collections::BTreeMap<String, (String, String, Option<String>)>,
    account_index: &std::collections::BTreeMap<String, (String, String)>,
    recipe_id: &str,
) -> (Option<String>, String, String, Option<String>) {
    let (account_id, provider, model) = recipe_index.get(recipe_id).cloned().unwrap_or_default();
    let account_label = account_index
        .get(&account_id)
        .map_or_else(|| "unknown".to_string(), |(label, _)| label.clone());
    let provider = if provider.is_empty() {
        "unknown".to_string()
    } else {
        provider
    };
    (
        if account_id.is_empty() {
            None
        } else {
            Some(account_id)
        },
        account_label,
        provider,
        model,
    )
}

/// AJ-E412 Req.1 classification: the I-03 display model prefers the
/// OBSERVED running model over the launch intent; the source names how
/// it is known (`in-session` = observed drift from the overlay).
fn display_model(
    running: Option<&juggler_core::model::RunningModel>,
    launch: Option<&String>,
) -> (Option<String>, String) {
    let observed = running.map(|entry| entry.model.clone());
    let source = match (&observed, launch) {
        (Some(running_model), Some(launch_model)) if running_model != launch_model => "in-session",
        (Some(_), _) => "transcript",
        (None, Some(_)) => "launch",
        (None, None) => "unknown",
    };
    (observed.or_else(|| launch.cloned()), source.to_string())
}

/// Per-Provider desktop isolation claims (AJ-E413 Req.1): provider id →
/// the label shown verbatim on that provider's desktop Instances.
/// Providers without a desktop app contribute nothing.
fn desktop_isolation_labels(
    registry: &juggler_adapter::AdapterRegistry,
) -> std::collections::BTreeMap<String, String> {
    registry
        .ids()
        .into_iter()
        .filter_map(|harness| {
            registry
                .get(&harness)
                .and_then(|adapter| adapter.desktop_isolation_label())
                .map(|label| (harness.to_string(), label.to_string()))
        })
        .collect()
}

/// Session rows with denormalized account/provider joins and the full
/// I-01..I-11 read model (AJ-E401). One storage pass per collection
/// (accounts, launch settings, instances) joined in memory — no per-row
/// storage reads; absent values serialise as `null`, never `0` or `""`.
/// Test/inspection surface: session rows with an empty live-stats map.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub fn session_infos_snapshot(storage: &Storage) -> AppResult<Vec<InstanceInfo>> {
    session_infos(
        storage,
        &LiveStatsMap::new(),
        &std::collections::BTreeMap::new(),
        &[],
    )
}

/// account id → (label, provider).
type AccountIndex = std::collections::BTreeMap<String, (String, String)>;
/// launch-recipe id → (account id, provider, overlay model).
type RecipeIndex = std::collections::BTreeMap<String, (String, String, Option<String>)>;

/// Join indexes for the read model: account id → (label, provider) and
/// launch-recipe id → (account id, provider, overlay model). One
/// storage pass per collection, joined in memory — no per-row reads.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
fn session_join_indexes(storage: &Storage) -> AppResult<(AccountIndex, RecipeIndex)> {
    let account_index: AccountIndex = storage
        .accounts()
        .list()?
        .into_iter()
        .map(|account| {
            (
                account.id.to_string(),
                (account.label, account.harness.to_string()),
            )
        })
        .collect();
    let mut recipe_index: RecipeIndex = std::collections::BTreeMap::new();
    for (account_id, (_, harness)) in &account_index {
        for recipe in storage
            .profiles()
            .list_by_account(&AccountId::new(account_id.as_str()))?
        {
            recipe_index.insert(
                recipe.id.to_string(),
                (
                    recipe.account.to_string(),
                    harness.clone(),
                    recipe.settings_overlay.model,
                ),
            );
        }
    }
    Ok((account_index, recipe_index))
}

fn session_infos(
    storage: &Storage,
    live: &LiveStatsMap,
    desktop_labels: &std::collections::BTreeMap<String, String>,
    adopted_ids: &[juggler_core::model::SessionId],
) -> AppResult<Vec<InstanceInfo>> {
    let (account_index, recipe_index) = session_join_indexes(storage)?;
    Ok(storage
        .sessions()
        .list(None)?
        .into_iter()
        .map(|record| {
            let (account_id, account_label, provider, launch_model) =
                resolved_join(&recipe_index, &account_index, &record.profile.to_string());
            let isolation_label = if record.kind == juggler_core::model::InstanceKind::Desktop {
                desktop_labels.get(&provider).cloned()
            } else {
                None
            };
            let (model, model_source) =
                display_model(record.running.as_ref(), launch_model.as_ref());
            let terminal = is_terminal(record.status);
            let stats = if terminal {
                None
            } else {
                live.get(&record.id.to_string()).copied()
            };
            InstanceInfo {
                adopted: adopted_ids.contains(&record.id),
                id: record.id.to_string(),
                launch_recipe_id: record.profile.to_string(),
                provider,
                account_id,
                account_label,
                model,
                launch_model,
                running_model: record
                    .running
                    .as_ref()
                    .map(|observed| observed.model.clone()),
                model_source,
                status: record.status.as_str().to_string(),
                quota: quota_snapshot_of(&record.usage),
                tokens: if record.usage.tokens == juggler_core::model::TokenCounts::default() {
                    None
                } else {
                    let t = record.usage.tokens;
                    Some(TokenCountsInfo {
                        input: t.input,
                        output: t.output,
                        cache_read: t.cache_read,
                        cache_write: t.cache_write,
                        reasoning: t.reasoning,
                    })
                },
                cpu_pct: stats.and_then(|s| s.cpu_pct),
                rss_bytes: stats.map(|s| s.rss_bytes),
                child_count: if terminal {
                    Some(0)
                } else {
                    stats.map(|s| s.child_count)
                },
                // No transcript-derived source exists yet (AJ-E409) —
                // shipped nullable, never backfilled with a guess.
                subtask_count: None,
                git_branch: git_branch_of(&record.cwd),
                // No subprocess-free dirty-count source; collector lands
                // with the E4 signals work. Null = unavailable, not 0.
                git_dirty_count: None,
                needs_attention: needs_attention_of(record.status),
                attention: record.attention.as_ref().map(|reason| AttentionInfo {
                    rule: reason.rule.clone(),
                    inputs: reason.inputs.clone(),
                    since_ms: reason.since_ms,
                }),
                started_at_ms: record.started_at.millis(),
                last_activity_ms: record.last_activity.as_ref().map(|t| t.millis()),
                machine_id: record.machine.to_string(),
                cwd: record.cwd.to_string_lossy().into_owned(),
                kind: record.kind.as_str().to_string(),
                project_id: record
                    .project
                    .as_ref()
                    .map(std::string::ToString::to_string),
                conversation_id: record.conversation.as_ref().map(|c| c.as_str().to_string()),
                isolation_label,
            }
        })
        .collect())
}

/// Scans imported transcripts and backfills the projects rail with the
/// user's REAL history: every project directory found in any account's
/// `projects/` transcripts (cwd read from the transcript itself, never
/// guessed from the encoded dir name), with per-account history merged.
/// Idempotent; safe to run at every startup.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn projects_backfill(state: tauri::State<'_, Arc<AppState>>) -> AppResult<u32> {
    projects_backfill_impl(state.inner())
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn projects_backfill_impl(state: &Arc<AppState>) -> AppResult<u32> {
    use std::collections::BTreeMap;

    // root → (labels, native session count)
    let mut found: BTreeMap<PathBuf, (Vec<String>, u64)> = BTreeMap::new();
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    for account in &accounts {
        let projects_root = account.state_dir.join("projects");
        let Ok(entries) = std::fs::read_dir(&projects_root) else {
            continue;
        };
        for project_dir in entries.flatten().filter(|e| e.path().is_dir()) {
            let jsonl = newest_jsonl(&project_dir.path());
            let Some(jsonl) = jsonl else { continue };
            let Some(cwd) = transcript_cwd(&jsonl) else {
                continue; // cannot decode the dir name unambiguously
            };
            let entry = found
                .entry(PathBuf::from(&cwd))
                .or_insert_with(|| (Vec::new(), 0));
            if !entry.0.contains(&account.label) {
                entry.0.push(account.label.clone());
            }
            entry.1 += count_jsonl(&project_dir.path());
        }
    }

    let storage = lock_storage_arc(state)?;
    let mut enriched = 0;
    for (root, (labels, sessions)) in found {
        let record = storage.projects().touch(&root, None)?;
        storage.projects().merge_history(
            &record.id,
            &juggler_core::model::ProjectHistory {
                accounts: labels,
                native_sessions: sessions,
            },
        )?;
        enriched += 1;
    }
    Ok(enriched)
}

/// Newest `.jsonl` in a project transcript dir by mtime.
fn newest_jsonl(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "jsonl"))
        .max_by_key(|entry| entry.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|entry| entry.path())
}

fn count_jsonl(dir: &std::path::Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|e| e == "jsonl"))
                .count() as u64
        })
        .unwrap_or_default()
}

/// Reads a transcript's true working directory from its first lines
/// (`"cwd"` is recorded on message records) — the only unambiguous decode.
fn transcript_cwd(jsonl: &std::path::Path) -> Option<String> {
    use std::io::BufRead;
    let file = std::fs::File::open(jsonl).ok()?;
    for line in std::io::BufReader::new(file).lines().take(25).flatten() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
            && let Some(cwd) = value.get("cwd").and_then(|v| v.as_str())
            && !cwd.is_empty()
        {
            return Some(cwd.to_string());
        }
    }
    None
}

/// The juggle (spec §7 T1/T2 v0): move a live session's transcript to
/// another account and resume it there — same project, same context, one
/// click. User-initiated only (automation policy).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn session_handover(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<LaunchResult> {
    let result = session_handover_impl(state.inner(), &session_id, &target_profile_id)?;
    emit_sessions(&app, state.inner())?;
    Ok(result)
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// The hand-over source row + its owning account (the read-side
/// preamble of [`session_handover_impl`]).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the source session or
/// its account cannot be resolved.
fn handover_source(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<(juggler_core::model::InstanceRecord, Account)> {
    let storage = lock_storage_arc(state)?;
    let source = storage
        .sessions()
        .get(&SessionId::new(session_id))
        .map_err(|error| not_found_or_core(error, "session", session_id))?;
    let source_profile = storage
        .profiles()
        .get(&source.profile)
        .map_err(|error| not_found_or_core(error, "profile", &source.profile.to_string()))?;
    let account_a = storage
        .accounts()
        .get(&source_profile.account)
        .map_err(|error| {
            not_found_or_core(error, "account", &source_profile.account.to_string())
        })?;
    Ok((source, account_a))
}

/// Resumes the session under the target account (see the module docs).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the source session, its
/// transcript, or the target profile cannot be resolved, or on copy or
/// launch failure.
pub fn session_handover_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<LaunchResult> {
    // AJ-E205: the pipeline owns execution (freeze → stage → verify →
    // launch → record); the default freeze choice waits for the turn
    // boundary. The UI calls `switch_execute` with the user's choice.
    switch_execute_impl(
        state,
        session_id,
        target_profile_id,
        "finish-turn",
        None,
        &[],
    )
    .map(|switched| LaunchResult {
        session_id: switched.session_id,
        pid: switched.pid,
    })
}

/// AJ-E205: the preflight verdict for one switch (rendered by the
/// freeze dialog / door before anything executes).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct PreflightInfo {
    /// Mechanism wire name (`native-resume|transplant|replay|pack`).
    pub mechanism: String,
    /// The F-label shown (F0–F4).
    pub fidelity: String,
    /// The capped score (internal honesty; the label is the UI).
    pub score: f64,
    /// §7.1 warning copy (verbatim table renders).
    pub warnings: Vec<String>,
    /// Zombie children listed at freeze (no action taken).
    pub zombies: Vec<String>,
    /// Whether an uncommitted diff snapshot rides the pack.
    pub diff_snapshotted: bool,
    /// Whether the source instance is live (freeze dialog needed).
    pub source_live: bool,
    /// The F1-experimental transplant copy (until E210 proves pairs).
    pub experimental_copy: String,
    /// AJ-E206 §8 door verdict: `one-click` or `wizard`.
    pub door: String,
    /// Wizard step titles in §8 order (empty for 1-click; only the
    /// non-trivial steps ever appear — never phantom steps).
    pub wizard_steps: Vec<String>,
    /// Why the wizard opened (one line per failed §8 condition;
    /// empty for 1-click).
    pub wizard_reasons: Vec<String>,
    /// The target is the project's dominant account (the usual
    /// test; unique max by session count, ties are not habits).
    pub usual_account: bool,
    /// The carry-list the door renders (carried / rebuilt / dropped
    /// names — the popover one-liner and the wizard's preview).
    pub carry: CarryListInfo,
    /// AJ-E208: the destination Provider's redaction policy (the
    /// verbatim confirm line; unset ⇒ the first-run chooser).
    pub redaction: RedactionPolicyInfo,
    /// AJ-E208: the source conversation's detected secrets (masked)
    /// — the per-item un-redact list.
    pub redaction_items: Vec<RedactionItemInfo>,
}

/// The carry-list grouped by disposition (AJ-E206; every name is a
/// concrete artifact — "some context" is forbidden copy).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize)]
pub struct CarryListInfo {
    /// Artifacts that move or copy.
    pub carried: Vec<String>,
    /// Artifacts the target rebuilds (replay turn / re-derived).
    pub regenerated: Vec<String>,
    /// Artifacts that do not carry — must be listed (§8 1-click
    /// blocker; untranslated translates score here too, §H2).
    pub dropped: Vec<String>,
}

/// AJ-E205: preflight for a switch — ordered checks stop BEFORE any
/// byte moves; warnings render for the door (E206).
///
/// # Errors
///
/// Returns the typed [`AppError`] when resolution fails or a hard
/// gate stops the switch.
#[tauri::command]
pub fn switch_preflight(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<PreflightInfo> {
    switch_preflight_impl(state.inner(), &session_id, &target_profile_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on resolution failures or hard
/// preflight stops.
pub fn switch_preflight_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<PreflightInfo> {
    use juggler_handover::pipeline::{self, CopySituation};

    let (_profile_b, account_b) = find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = handover_source(state, session_id)?;
    let reference = source.transcript_ref.clone().ok_or_else(|| AppError::Io {
        context: "hand over",
        message: "Couldn't tell which file is this session — pick it".into(),
    })?;

    // Compaction scan from the source transcript (the F2 signal).
    let compacted = std::fs::read(&reference.path).is_ok_and(|bytes| {
        juggler_adapters::claude_transcript::parse_transcript(&bytes).has_compact_boundary()
    });

    // Target quota gate + the usual-account test (one storage pass).
    let now = Timestamp::now().millis();
    let storage = lock_storage_arc(state)?;
    let (exhausted, tight) = quota_gate_of(&storage, &account_b, now);
    // AJ-E206 §8: the "usual account" test — the project's dominant
    // account by session count (a tie is not a habit).
    let usual_account = source
        .project
        .as_ref()
        .is_some_and(|project| usual_account_for_project(&storage, project, &account_b));
    drop(storage);

    // One disposition pass feeds both the fidelity engine and the
    // door's carry-list (same names, same truth).
    let dispositions = juggler_handover::fidelity::base_dispositions(
        account_a.harness.as_str(),
        account_b.harness.as_str(),
    );
    let carry = carry_lists_of(&dispositions);

    let input = pipeline::PreflightInput {
        fidelity: dispositions,
        compacted,
        gates: pipeline::PreflightGates {
            target_exhausted_no_reset: exhausted,
            version_drift: false, // per-version pinning rides E210's matrix
        },
        target_quota_tight: tight,
        git: pipeline::GitState::default(),
        hop_count: 0,
        replay_tokens_spent: 0,
    };
    let report = pipeline::preflight(&input).map_err(|failure| AppError::Io {
        context: "hand over",
        message: failure.to_string(),
    })?;
    let warnings: Vec<String> = report
        .warnings
        .iter()
        .map(|situation| pipeline::user_copy(situation.clone()))
        .collect();
    let source_live = matches!(source.status.as_str(), "running" | "starting");

    // AJ-E206 §8: choose the door from the same preflight facts.
    // Headroom ≥ Tight holds unless the tight warn fired (exhausted
    // already stopped above at the hard gate); conflicts arrive with
    // E207's reconciliation and rules divergence has no source yet.
    let door_inputs = juggler_handover::door::DoorInputs {
        usual_account,
        mechanism: Some(report.mechanism),
        fidelity: Some(report.pair.label),
        compacted,
        conflicts: false,
        drops_listed: !carry.dropped.is_empty(),
        target_headroom_ok: tight.is_none(),
        rules_diverged: false,
    };
    let (door, wizard_steps, wizard_reasons) =
        door_verdict(state, &door_inputs, &carry, report.pair.label);
    let (redaction, redaction_items) = {
        let storage = lock_storage_arc(state)?;
        let policy = RedactionPolicyInfo::from_parts(
            &account_b.harness,
            redaction_global_of(&storage),
            redaction_override_of(&storage, account_b.harness.as_str()),
        );
        (policy, redaction_items_of(&source))
    };

    Ok(PreflightInfo {
        mechanism: match report.mechanism {
            pipeline::Mechanism::NativeResume => "native-resume".to_owned(),
            pipeline::Mechanism::Transplant => "transplant".to_owned(),
            pipeline::Mechanism::Replay => "replay".to_owned(),
            pipeline::Mechanism::PackOnly => "pack".to_owned(),
        },
        fidelity: report.pair.label.as_str().to_owned(),
        score: report.pair.score,
        warnings,
        zombies: report.zombie_children.clone(),
        diff_snapshotted: report.diff_snapshotted,
        source_live,
        experimental_copy: pipeline::user_copy(CopySituation::TransplantUnproven {
            account_a: account_a.label.clone(),
        }),
        door,
        wizard_steps,
        wizard_reasons,
        usual_account,
        carry,
        redaction,
        redaction_items,
    })
}

/// Target quota gate from the E606/E610 engine (hard filter =
/// exhausted with no reset; tight = the warn copy).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // display-copy rounding, clamped to [0, u32::MAX) before the cast
fn quota_gate_of(storage: &Storage, account: &Account, now: i64) -> (bool, Option<(u32, u32)>) {
    let target_display = quota_display_inner(storage, account, now);
    match target_display.five_hour.as_ref().map(|w| w.state.as_str()) {
        Some("exhausted") => (true, None),
        Some(_) => {
            let used = target_display
                .five_hour
                .as_ref()
                .and_then(|window| window.percentage)
                .unwrap_or(0.0);
            let tight = target_display
                .forecast
                .tte_min
                .filter(|tte| *tte < 45.0)
                .map(|tte| {
                    (
                        (100.0 - used).round().clamp(0.0, 99.0) as u32,
                        tte.round().clamp(0.0, 999.0) as u32,
                    )
                });
            (false, tight)
        }
        None => (false, None),
    }
}

/// AJ-E206 §8: chooses the door from the preflight facts and counts
/// the choice (local telemetry). Returns the wire name, the wizard
/// step titles (empty for 1-click — never phantom steps), and the
/// wizard's opening lines.
fn door_verdict(
    state: &Arc<AppState>,
    inputs: &juggler_handover::door::DoorInputs,
    carry: &CarryListInfo,
    label: juggler_handover::fidelity::FidelityLabel,
) -> (String, Vec<String>, Vec<String>) {
    let door = juggler_handover::door::choose_door(inputs);
    record_door_counter(state, &door);
    match &door {
        juggler_handover::door::Door::OneClick => ("one-click".to_owned(), Vec::new(), Vec::new()),
        juggler_handover::door::Door::Wizard { steps } => (
            "wizard".to_owned(),
            steps.iter().map(|step| step.title().to_owned()).collect(),
            wizard_reasons_of(inputs, carry, label),
        ),
    }
}

/// AJ-E206 §8: whether `target` is the project's dominant account —
/// strictly the unique maximum by occurrence count (live session
/// records by account, plus one occurrence per imported-history
/// label). No history ⇒ nobody is usual ⇒ the wizard opens.
fn usual_account_for_project(
    storage: &Storage,
    project: &juggler_core::model::ProjectId,
    target: &Account,
) -> bool {
    let mut counts: std::collections::BTreeMap<juggler_core::model::AccountId, u64> =
        std::collections::BTreeMap::new();
    let sessions = storage.sessions().list(None).unwrap_or_default();
    for record in sessions
        .iter()
        .filter(|row| row.project.as_ref() == Some(project))
    {
        if let Ok(recipe) = storage.profiles().get(&record.profile) {
            *counts.entry(recipe.account).or_default() += 1;
        }
    }
    // Imported history speaks in labels, not ids (ADR-0008 E1 rename
    // kept `ProjectHistory.accounts` as labels) — resolve via accounts.
    if let Ok(record) = storage.projects().get(project) {
        let accounts = storage.accounts().list().unwrap_or_default();
        for label in &record.history.accounts {
            if let Some(account) = accounts.iter().find(|row| &row.label == label) {
                *counts.entry(account.id.clone()).or_default() += 1;
            }
        }
    }
    let Some(target_count) = counts.get(&target.id).copied().filter(|count| *count > 0) else {
        return false;
    };
    let others_max = counts
        .iter()
        .filter(|(id, _)| *id != &target.id)
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(0);
    target_count > others_max
}

/// Groups the pair's dispositions into the door's carry-list names.
fn carry_lists_of(input: &juggler_handover::fidelity::FidelityInput) -> CarryListInfo {
    use juggler_handover::fidelity::{Disposition, FidelityInput};
    let FidelityInput { dispositions, .. } = input;
    let mut carry = CarryListInfo::default();
    for (kind, disposition) in dispositions {
        match disposition {
            Disposition::Copy => carry.carried.push(kind.label().to_owned()),
            Disposition::Regenerate => carry.regenerated.push(kind.label().to_owned()),
            // A translate without a written mapping (or with dropped
            // keys) is honestly a Drop (§H2) — it carries nothing the
            // user can rely on, so it lists on the drop side.
            Disposition::Translate {
                untranslated,
                keys_dropped,
            } => {
                if *untranslated || *keys_dropped {
                    carry.dropped.push(kind.label().to_owned());
                } else {
                    carry.carried.push(kind.label().to_owned());
                }
            }
            Disposition::Drop => carry.dropped.push(kind.label().to_owned()),
        }
    }
    carry
}

/// AJ-E206 §5: the wizard's opening lines — one human sentence per
/// failed §8 condition (never a generic "some context may be lost").
fn wizard_reasons_of(
    inputs: &juggler_handover::door::DoorInputs,
    carry: &CarryListInfo,
    label: juggler_handover::fidelity::FidelityLabel,
) -> Vec<String> {
    use juggler_handover::fidelity::FidelityLabel;
    use juggler_handover::pipeline::Mechanism;
    let mut reasons = Vec::new();
    if !inputs.usual_account {
        reasons.push("This isn't the usual account for this project.".to_owned());
    }
    match inputs.mechanism {
        Some(Mechanism::Replay) => {
            reasons.push("This switch replays — the target can't resume natively.".to_owned());
        }
        Some(Mechanism::PackOnly) => {
            reasons.push("Only a pack can move — the target can't resume or replay.".to_owned());
        }
        _ => {}
    }
    if matches!(inputs.fidelity, Some(shown) if shown < FidelityLabel::F3) {
        reasons.push(format!(
            "Fidelity is {}: something would translate or drop.",
            label.as_str()
        ));
    }
    if inputs.compacted {
        reasons.push("The source was compacted — the tail may not carry.".to_owned());
    }
    if !carry.dropped.is_empty() {
        reasons.push(format!(
            "The carry-list drops: {}.",
            carry.dropped.join(", ")
        ));
    }
    if !inputs.target_headroom_ok {
        reasons.push("The target is running low on quota.".to_owned());
    }
    reasons
}

/// AJ-E206 §5 telemetry: count the door the preflight chose — local
/// KV diagnostics only (never networked); a failed read or write
/// never blocks the switch.
fn record_door_counter(state: &Arc<AppState>, door: &juggler_handover::door::Door) {
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let settings = storage.app_settings();
    let mut counters: std::collections::BTreeMap<String, u64> = settings
        .get(SWITCH_DOOR_COUNTERS_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    *counters.entry(door.telemetry_kind()).or_insert(0) += 1;
    if let Ok(blob) = serde_json::to_string(&counters) {
        let _ = settings.set(SWITCH_DOOR_COUNTERS_KEY, &blob);
    }
}

/// AJ-E206: the executed switch's handles — the new instance plus
/// the graph edge that `switch_undo` compensates.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct SwitchExecutedInfo {
    /// The continuation instance's session id.
    pub session_id: String,
    /// The continuation instance's process id.
    pub pid: u32,
    /// The recorded edge (the undo handle).
    pub edge_id: String,
    /// The resting source session (what undo resumes).
    pub source_session_id: String,
}

/// AJ-E205: executes a switch with the user's freeze choice — the
/// pipeline stages + verifies + launches (via the supervisor), and
/// every attempt records its edge in the Switch Graph (E201).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure; the source
/// is exactly resumable in all of them (S-F contract).
#[tauri::command]
pub fn switch_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    freeze_choice: String,
    redaction_choice: Option<String>,
    redaction_unredact: Option<Vec<u32>>,
) -> AppResult<SwitchExecutedInfo> {
    switch_execute_impl(
        state.inner(),
        &session_id,
        &target_profile_id,
        &freeze_choice,
        redaction_choice.as_deref(),
        &redaction_unredact.unwrap_or_default(),
    )
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure.
pub fn switch_execute_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
) -> AppResult<SwitchExecutedInfo> {
    use juggler_handover::pipeline::{self, FreezeChoice, Mechanism};

    let freeze = if freeze_choice == "stop-now" {
        FreezeChoice::StopNowAndSwitch
    } else {
        FreezeChoice::FinishTurnThenSwitch
    };
    let (profile_b, account_b) = find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = handover_source(state, session_id)?;
    if source.project.is_none() {
        return Err(AppError::Io {
            context: "hand over",
            message: "session has no project link".into(),
        });
    }
    let reference = source.transcript_ref.clone().ok_or_else(|| AppError::Io {
        context: "hand over",
        message: "Couldn't tell which file is this session — pick it".into(),
    })?;
    let support = state
        .registry
        .handover(&account_b.harness)
        .ok_or_else(|| AppError::Io {
            context: "hand over",
            message: format!(
                "{} has no transplant support — replay/pack only",
                account_b.harness
            ),
        })?;

    // Record (E201): the Staged edge exists BEFORE anything moves;
    // failures transition it, history is never rewritten.
    let (work_id, from_epoch, edge_id) =
        record_edge_staged(state, &source, &reference, &account_a, &account_b)?;

    // The pipeline: snapshot → stage → verify → launch (the app's
    // supervisor hook) → the copy-never-move invariant.
    let adapter = state
        .registry
        .get(&account_b.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_b.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let mut launch_hook = make_launch_hook(
        state,
        adapter,
        &account_b,
        &profile_b,
        &source,
        &new_session,
    );
    let outcome = pipeline::execute(
        support.as_ref(),
        &reference,
        &account_b,
        freeze,
        Mechanism::Transplant,
        &mut launch_hook,
    );
    let (record, pid) = match outcome {
        Ok(value) => value,
        Err(failure) => {
            record_edge_failed(state, &edge_id, &failure);
            return Err(AppError::Io {
                context: "hand over",
                message: failure.copy(&account_a.label),
            });
        }
    };
    record_edge_succeeded(
        state,
        &source,
        &profile_b,
        &record,
        &account_b,
        &EdgeCtx {
            work: work_id,
            from_epoch,
            edge: edge_id.clone(),
            session: new_session.clone(),
        },
    )?;

    // AJ-E208: the switch's redaction decision rides the edge.
    record_redaction_decision(
        state,
        &edge_id,
        &account_b.harness,
        redaction_choice,
        redaction_unredact,
    );

    Ok(SwitchExecutedInfo {
        session_id: new_session.to_string(),
        pid,
        edge_id: edge_id.to_string(),
        source_session_id: source.id.to_string(),
    })
}

/// AJ-E206 §8.2: undoes a completed switch — stops the continuation
/// instance, resumes the source account's native session, and records
/// the compensating edge (`parent_edge_a` = the original). User-click
/// only; the graph keeps both edges so the travel stays explainable.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the edge cannot be compensated
/// (not confirmed, missing epochs/native ids) or stop/resume fails —
/// a failed stop aborts BEFORE the source resumes (never two live
/// instances on one work item).
#[tauri::command]
pub fn switch_undo(
    state: tauri::State<'_, Arc<AppState>>,
    edge_id: String,
) -> AppResult<LaunchResult> {
    switch_undo_impl(state.inner(), &edge_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_undo_impl(state: &Arc<AppState>, edge_id: &str) -> AppResult<LaunchResult> {
    let context = undo_edge_context(state, edge_id)?;
    stop_continuation(state, context.target_session.clone())?;
    undo_resume_source(state, &context)
}

/// The read-side of a switch to undo (AJ-E206).
struct UndoContext {
    /// The work item both epochs belong to.
    work: juggler_core::model::WorkItemId,
    /// The source epoch (what undo resumes).
    from_epoch: juggler_core::model::Epoch,
    /// The confirmed switch's to-epoch.
    to_epoch: juggler_core::model::EpochId,
    /// The continuation instance (what undo stops), if any.
    target_session: Option<SessionId>,
    /// The original edge (the compensating edge's parent).
    edge: juggler_core::model::SwitchEdge,
    /// The source's native session id (the resume target).
    native_id: String,
}

/// Loads and validates the edge to undo: confirmed, both epochs
/// present, source session + native id resolvable.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the edge cannot be compensated.
fn undo_edge_context(state: &Arc<AppState>, edge_id: &str) -> AppResult<UndoContext> {
    use juggler_core::model::{EdgeState, EpochId, SwitchEdgeId};
    let undo_of = SwitchEdgeId::new(edge_id.to_owned());
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let edge = graph.edge(&undo_of).map_err(|_| AppError::Io {
        context: "undo switch",
        message: "That switch isn't in the graph — nothing to undo".to_owned(),
    })?;
    let not_landed = || AppError::Io {
        context: "undo switch",
        message: "Only a completed switch can be undone — this one never landed".to_owned(),
    };
    if !matches!(edge.state, EdgeState::Confirmed) {
        return Err(not_landed());
    }
    let Some(to_epoch_id) = edge.to_epoch.clone() else {
        return Err(not_landed());
    };
    let epochs = graph.epochs_for_work(&edge.work)?;
    let find = |id: &EpochId| epochs.iter().find(|epoch| &epoch.id == id);
    let from_epoch = find(&edge.from_epoch)
        .ok_or_else(|| AppError::Io {
            context: "undo switch",
            message: "The switch's source epoch is missing from the graph".to_owned(),
        })?
        .clone();
    let target_session = find(&to_epoch_id).and_then(|epoch| epoch.session_ref.clone());
    if from_epoch.session_ref.is_none() {
        return Err(AppError::Io {
            context: "undo switch",
            message: "The switch's source session is missing — resume it from Sessions".to_owned(),
        });
    }
    let Some(native_id) = from_epoch.native_id.clone().filter(|id| !id.is_empty()) else {
        return Err(AppError::Io {
            context: "undo switch",
            message: "The source has no native session to resume".to_owned(),
        });
    };
    Ok(UndoContext {
        work: edge.work.clone(),
        from_epoch,
        to_epoch: to_epoch_id,
        target_session,
        edge,
        native_id,
    })
}

/// Stops the continuation instance. Already-gone is fine (the stop
/// goal already holds); a REAL stop failure aborts before the source
/// resumes — never two live instances on one work item.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the stop ladder fails.
fn stop_continuation(state: &Arc<AppState>, target: Option<SessionId>) -> AppResult<()> {
    let Some(target_session) = target else {
        return Ok(());
    };
    let stopped = state
        .supervisor
        .stop(&target_session, Duration::from_secs(5));
    match stopped {
        Ok(()) | Err(juggler_supervisor::SupervisorError::UnknownSession(_)) => Ok(()),
        Err(error) => Err(AppError::Io {
            context: "undo switch",
            message: format!("Couldn't stop the continuation instance: {error}"),
        }),
    }
}

/// Resumes the source: its account, its recipe, its own native id —
/// then records the compensating edge (`parent_edge_a` = the original;
/// to-epoch on the source account; launched+confirmed in one motion,
/// the supervisor accepting the spawn being the proof we can record
/// today).
///
/// # Errors
///
/// Returns the typed [`AppError`] when resolution, spawn, or graph
/// recording fails.
fn undo_resume_source(state: &Arc<AppState>, context: &UndoContext) -> AppResult<LaunchResult> {
    let UndoContext {
        from_epoch,
        native_id,
        ..
    } = context;
    let Some(source_session_ref) = from_epoch.session_ref.clone() else {
        return Err(AppError::Io {
            context: "undo switch",
            message: "The switch's source session is missing — resume it from Sessions".to_owned(),
        });
    };

    let (source, account_a, profile_a) = {
        let storage = lock_storage_arc(state)?;
        let source = storage
            .sessions()
            .get(&source_session_ref)
            .map_err(|error| {
                not_found_or_core(error, "session", &source_session_ref.to_string())
            })?;
        let account_a = storage
            .accounts()
            .get(&from_epoch.account)
            .map_err(|error| {
                not_found_or_core(error, "account", &from_epoch.account.to_string())
            })?;
        let profile_a = storage
            .profiles()
            .get(&source.profile)
            .map_err(|error| not_found_or_core(error, "profile", &source.profile.to_string()))?;
        (source, account_a, profile_a)
    };
    let adapter = state
        .registry
        .get(&account_a.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_a.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let spec = adapter.launch_spec(
        &account_a,
        &profile_a,
        &source.cwd,
        &LaunchMode::Resume {
            session_id: native_id.clone(),
            delta_prompt: None,
        },
    )?;
    let spawned = state.supervisor.spawn(
        &new_session,
        &spec,
        &ledger_ctx(
            state,
            &account_a,
            source.project.as_ref().map(ToString::to_string),
        ),
    )?;

    record_compensating_edge(state, context, &new_session, &profile_a.id, &source)?;

    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid: spawned.pid,
    })
}

/// Records the undo: the resumed source instance row, its epoch, and
/// the compensating edge (`parent_edge_a` = the original; launched +
/// confirmed in one motion — the supervisor accepting the spawn is
/// the proof we can record today).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage or graph writes fail.
fn record_compensating_edge(
    state: &Arc<AppState>,
    context: &UndoContext,
    new_session: &SessionId,
    profile: &juggler_core::model::LaunchRecipeId,
    source: &juggler_core::model::InstanceRecord,
) -> AppResult<()> {
    use juggler_core::model::{
        ConversationId, Epoch, EpochId, InstanceKind, InstanceRecord, SwitchEdge, SwitchEdgeId,
        SwitchMechanism,
    };
    let UndoContext {
        work,
        to_epoch,
        edge,
        native_id,
        ..
    } = context;
    let machine = local_machine_id(state);
    let storage = lock_storage_arc(state)?;
    let usage = engine_usage_snapshot(&storage, profile);
    storage.sessions().insert(&InstanceRecord {
        id: new_session.clone(),
        conversation: Some(ConversationId::new(native_id.to_owned())),
        profile: profile.clone(),
        machine,
        kind: InstanceKind::Terminal,
        project: source.project.clone(),
        cwd: source.cwd.clone(),
        status: SessionStatus::Starting,
        started_at: Timestamp::now(),
        last_activity: None,
        usage,
        attention: None,
        running: None,
        window: None,
        transcript_ref: source.transcript_ref.clone(),
    })?;
    let graph = storage.switch_graph();
    let epoch_undo = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: epoch_undo.clone(),
        work: work.clone(),
        session_ref: Some(new_session.clone()),
        native_id: Some(native_id.to_owned()),
        account: edge.from_account.clone(),
        provider: edge.from_provider.clone(),
        resumed_from_epoch: Some(to_epoch.clone()),
    })?;
    let undo_edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph.append_edge(&SwitchEdge {
        id: undo_edge_id.clone(),
        work: work.clone(),
        from_epoch: to_epoch.clone(),
        // Filled by mark_launched below (S-G1: append never sets it).
        to_epoch: None,
        from_account: edge.to_account.clone(),
        to_account: edge.from_account.clone(),
        from_provider: edge.to_provider.clone(),
        to_provider: edge.from_provider.clone(),
        // Same account, same native file: T1 native resume.
        mechanism: SwitchMechanism::NativeResume,
        pack_id: None,
        fidelity_shown: Some("F4".to_owned()),
        manifest_sha256: None,
        state: juggler_core::model::EdgeState::Staged,
        actor: juggler_core::model::EdgeActor::User,
        parent_edge_a: Some(edge.id.clone()),
        parent_edge_b: None,
        created_at: Timestamp::now(),
    })?;
    graph.mark_launched(&undo_edge_id, &epoch_undo)?;
    graph.mark_confirmed(&undo_edge_id)?;
    Ok(())
}

/// AJ-E208: records the switch's redaction decision on the edge —
/// the effective policy, the per-switch choice (Ask), and how many
/// items the user un-redacted for THIS switch only (never a
/// remembered-policy change). A storage failure never fails the
/// switch (the decision is diagnostics; the switch already landed).
fn record_redaction_decision(
    state: &Arc<AppState>,
    edge_id: &juggler_core::model::SwitchEdgeId,
    provider: &juggler_core::model::HarnessId,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
) {
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let policy = {
        let global = redaction_global_of(&storage);
        let r#override = redaction_override_of(&storage, provider.as_str());
        juggler_handover::redaction::resolve_policy(global, r#override)
    };
    let chosen = redaction_choice.and_then(juggler_handover::redaction::RedactionPolicy::from_wire);
    let applied = chosen
        .or(policy)
        .map_or_else(|| "unset".to_owned(), |parsed| parsed.as_str().to_owned());
    let rows = vec![
        juggler_core::model::ManifestEntry {
            rel_path: "redaction/policy".to_owned(),
            sha256: String::new(),
            bytes: 0,
            kind: "redaction".to_owned(),
            disposition: applied,
        },
        juggler_core::model::ManifestEntry {
            rel_path: "redaction/unredacted".to_owned(),
            sha256: String::new(),
            bytes: u64::try_from(redaction_unredact.len()).unwrap_or_default(),
            kind: "redaction-unredact".to_owned(),
            disposition: "this-switch-only".to_owned(),
        },
    ];
    let _ = storage.switch_graph().append_manifest(edge_id, &rows);
}

/// The supervisor launch hook: builds the resume spec for the
/// INSTALLED native id (a collision fork changes it) and spawns under
/// the app's supervisor — the only spawn path (suggest-only law).
fn make_launch_hook<'a>(
    state: &'a Arc<AppState>,
    adapter: juggler_adapter::SharedAdapter,
    account_b: &'a juggler_core::model::Account,
    profile_b: &'a juggler_core::model::LaunchRecipe,
    source: &'a juggler_core::model::InstanceRecord,
    new_session: &'a SessionId,
) -> impl FnMut(&juggler_adapter::handover::InstallOutcome) -> Result<u32, String> + 'a {
    move |install: &juggler_adapter::handover::InstallOutcome| -> Result<u32, String> {
        let spec = adapter
            .launch_spec(
                account_b,
                profile_b,
                &source.cwd,
                &LaunchMode::Resume {
                    session_id: install.reference.native_id.clone().unwrap_or_default(),
                    delta_prompt: None,
                },
            )
            .map_err(|error| error.to_string())?;
        let spawned = state
            .supervisor
            .spawn(
                new_session,
                &spec,
                &ledger_ctx(
                    state,
                    account_b,
                    source.project.as_ref().map(ToString::to_string),
                ),
            )
            .map_err(|error| error.to_string())?;
        Ok(spawned.pid)
    }
}

/// Records the pre-execution graph state: the work item (created on
/// first switch), the SOURCE epoch, and the Staged edge. Returns
/// `(work, from_epoch, edge)` ids.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
fn record_edge_staged(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    reference: &juggler_core::model::TranscriptRef,
    account_a: &juggler_core::model::Account,
    account_b: &juggler_core::model::Account,
) -> AppResult<(
    juggler_core::model::WorkItemId,
    juggler_core::model::EpochId,
    juggler_core::model::SwitchEdgeId,
)> {
    use juggler_core::model::{Epoch, EpochId, SwitchEdge, SwitchEdgeId, SwitchMechanism};

    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let Some(project) = source.project.as_ref() else {
        return Err(AppError::Io {
            context: "hand over",
            message: "session has no project link".into(),
        });
    };
    let work = match graph.works_for_project(project)?.into_iter().next_back() {
        Some(existing) => existing,
        None => graph
            .create_work(project, "Work")
            .map_err(|error| AppError::Io {
                context: "hand over",
                message: error.to_string(),
            })?,
    };
    let from = Epoch {
        id: EpochId::new(format!("ep-{}", unique_suffix())),
        work: work.id.clone(),
        session_ref: Some(source.id.clone()),
        native_id: reference.native_id.clone(),
        account: account_a.id.clone(),
        provider: account_a.harness.clone(),
        resumed_from_epoch: None,
    };
    graph.insert_epoch(&from).map_err(|error| AppError::Io {
        context: "hand over",
        message: error.to_string(),
    })?;
    let edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph
        .append_edge(&SwitchEdge {
            id: edge_id.clone(),
            work: work.id.clone(),
            from_epoch: from.id.clone(),
            to_epoch: None,
            from_account: account_a.id.clone(),
            to_account: account_b.id.clone(),
            from_provider: account_a.harness.clone(),
            to_provider: account_b.harness.clone(),
            mechanism: SwitchMechanism::Transplant,
            pack_id: None,
            fidelity_shown: Some("F1".to_owned()),
            manifest_sha256: None,
            state: juggler_core::model::EdgeState::Staged,
            actor: juggler_core::model::EdgeActor::User,
            parent_edge_a: None,
            parent_edge_b: None,
            created_at: Timestamp::now(),
        })
        .map_err(|error| AppError::Io {
            context: "hand over",
            message: error.to_string(),
        })?;
    Ok((work.id.clone(), from.id.clone(), edge_id))
}

/// Transitions the edge to `Failed{stage, reason}` — the same edge,
/// never a rewrite (S-G1); the source is safe by the pipeline's
/// contract.
fn record_edge_failed(
    state: &Arc<AppState>,
    edge: &juggler_core::model::SwitchEdgeId,
    failure: &juggler_handover::pipeline::SwitchFailure,
) {
    if let Ok(storage) = lock_storage_arc(state) {
        let _ = storage
            .switch_graph()
            .mark_failed(edge, failure.stage(), &failure.to_string());
    }
}

/// The graph ids one executing switch carries through recording.
struct EdgeCtx {
    /// The work item.
    work: juggler_core::model::WorkItemId,
    /// The source epoch.
    from_epoch: juggler_core::model::EpochId,
    /// The staged edge.
    edge: juggler_core::model::SwitchEdgeId,
    /// The continuation Instance id.
    session: SessionId,
}

/// Records the succeeded switch: the source rests (`HandedOver`), the
/// continuation Instance starts, the TARGET epoch is created, the
/// edge launches + confirms (the supervisor accepting the spawn is
/// the confirmation provable today), and the manifest rows land.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
fn record_edge_succeeded(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    profile_b: &juggler_core::model::LaunchRecipe,
    record: &juggler_handover::pipeline::SwitchRecord,
    target: &juggler_core::model::Account,
    ctx: &EdgeCtx,
) -> AppResult<()> {
    use juggler_core::model::{Epoch, EpochId};

    let native_id = record
        .install
        .reference
        .native_id
        .clone()
        .unwrap_or_default();
    let machine = local_machine_id(state);
    let storage = lock_storage_arc(state)?;
    let mut handed = source.clone();
    handed.status = SessionStatus::HandedOver;
    storage.sessions().update(&handed)?;
    let usage = engine_usage_snapshot(&storage, &profile_b.id);
    storage.sessions().insert(&SessionRecord {
        id: ctx.session.clone(),
        conversation: Some(juggler_core::model::ConversationId::new(native_id.clone())),
        profile: profile_b.id.clone(),
        machine,
        kind: juggler_core::model::InstanceKind::Terminal,
        project: source.project.clone(),
        cwd: source.cwd.clone(),
        status: SessionStatus::Starting,
        started_at: Timestamp::now(),
        last_activity: None,
        usage,
        attention: None,
        running: None,
        window: None,
        transcript_ref: Some(juggler_core::model::TranscriptRef {
            path: record.install.reference.path.clone(),
            native_id: Some(native_id.clone()),
        }),
    })?;
    let graph = storage.switch_graph();
    let to_epoch = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: to_epoch.clone(),
        work: ctx.work.clone(),
        session_ref: Some(ctx.session.clone()),
        native_id: Some(native_id),
        account: target.id.clone(),
        provider: target.harness.clone(),
        resumed_from_epoch: Some(ctx.from_epoch.clone()),
    })?;
    graph.mark_launched(&ctx.edge, &to_epoch)?;
    graph.mark_confirmed(&ctx.edge)?;
    graph.append_manifest(
        &ctx.edge,
        &record
            .source_hashes
            .iter()
            .map(|row| juggler_core::model::ManifestEntry {
                rel_path: row.rel_path.clone(),
                sha256: row.sha256.clone(),
                bytes: row.bytes,
                kind: row.kind.clone(),
                disposition: row.disposition.clone(),
            })
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}

/// Lists tracked projects with their live instance, account history, and
/// totals — the juggling view (ADR-0007).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn projects_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<ProjectInfo>> {
    projects_list_impl(state.inner())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn projects_list_impl(state: &Arc<AppState>) -> AppResult<Vec<ProjectInfo>> {
    // Collector first (never locked together with storage), then the
    // storage guard for the whole join below.
    let live_stats = live_stats_of(state);
    let storage = lock_storage_arc(state)?;
    let sessions = session_infos(
        &storage,
        &live_stats,
        &desktop_isolation_labels(&state.registry),
        &state.supervisor.alive_adopted_sessions(),
    )?;
    // Statuses that mean "occupying the project right now".
    let live = |status: &str| matches!(status, "starting" | "running");
    Ok(storage
        .projects()
        .list()?
        .into_iter()
        .map(|project| {
            let id = project.id.to_string();
            let owned: Vec<&SessionInfo> = sessions
                .iter()
                .filter(|s| s.project_id.as_deref() == Some(id.as_str()))
                .collect();
            let active_instance = owned.iter().find(|s| live(&s.status)).map(|s| (*s).clone());
            let mut accounts_used: Vec<String> = owned
                .iter()
                .map(|s| s.account_label.clone())
                .filter(|label| !label.is_empty())
                .collect();
            accounts_used.extend(project.history.accounts.iter().cloned());
            accounts_used.sort();
            accounts_used.dedup();
            ProjectInfo {
                id,
                root: project.root.to_string_lossy().into_owned(),
                name: project.name,
                last_activity_ms: project.last_activity.map(juggler_core::Timestamp::millis),
                active_instance,
                accounts_used,
                instance_count: u64::try_from(owned.len()).unwrap_or_default(),
                history_sessions: project.history.native_sessions,
                missing: !project.root.exists(),
            }
        })
        .collect())
}

fn find_account(
    state: &tauri::State<'_, Arc<AppState>>,
    account_id: &str,
) -> Result<Account, AppError> {
    find_account_arc(state.inner(), account_id)
}

/// Core account lookup shared with the terminal module and tests.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the account does not exist.
pub fn find_account_arc(state: &Arc<AppState>, account_id: &str) -> Result<Account, AppError> {
    lock_storage_arc(state)?
        .accounts()
        .get(&AccountId::new(account_id))
        .map_err(|error| not_found_or_core(error, "account", account_id))
}

fn find_profile_with_account_arc(
    state: &Arc<AppState>,
    profile_id: &str,
) -> Result<(Profile, Account), AppError> {
    let storage = lock_storage_arc(state)?;
    let profile = storage
        .profiles()
        .get(&ProfileId::new(profile_id))
        .map_err(|error| not_found_or_core(error, "profile", profile_id))?;
    let account = storage
        .accounts()
        .get(&profile.account)
        .map_err(|error| not_found_or_core(error, "account", &profile.account.to_string()))?;
    Ok((profile, account))
}

fn not_found_or_core(error: juggler_core::CoreError, entity: &'static str, id: &str) -> AppError {
    match error {
        juggler_core::CoreError::NotFound { .. } => AppError::NotFound {
            entity,
            id: id.to_string(),
        },
        other => AppError::Core(other),
    }
}

pub(crate) fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
        % 1_000_000_000_000
}

// ---------------------------------------------------------------------------
// AJ-E410: the Processes tab data, the orphan contract, and the Tier-1
// auto-cleanup engine (D-08).
// ---------------------------------------------------------------------------

/// One live process row for the Processes tab (PID / name / CPU / RAM /
/// age / child count / depth).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessRowInfo {
    /// OS process id.
    pub pid: u32,
    /// Parent pid, when readable.
    #[ts(type = "number | null")]
    pub parent_pid: Option<u32>,
    /// Process name.
    pub name: String,
    /// CPU utilization, percent; `null` until two table samples.
    pub cpu_pct: Option<f64>,
    /// Resident memory, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// Process age, ms.
    #[ts(type = "number")]
    pub age_ms: u64,
    /// Direct children within the tree.
    #[ts(type = "number")]
    pub child_count: u32,
    /// Tree depth (root = 0) — the UI indent level.
    pub depth: u8,
}

/// One I-07 sparkline point (2 s sampling, 5 min retained).
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TreeSampleInfo {
    /// Sample time, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Tree CPU%, percent; `null` before two samples.
    pub cpu_pct: Option<f64>,
    /// Tree RSS sum, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
}

/// One confirmed orphan row (`Orphaned - left behind (N)` section).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OrphanInfo {
    /// Instance the process was left behind by.
    pub session_id: String,
    /// Orphan process id.
    pub pid: u32,
    /// Process name.
    pub name: String,
    /// Resident memory, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// Label of the dead parent (its program name).
    pub parent_label: String,
    /// Age since the parent exited, ms.
    #[ts(type = "number")]
    pub age_since_parent_exit_ms: i64,
    /// Last stop state for this pid (`gone` / `still alive - escalated to
    /// SIGKILL` / …), when one exists.
    pub stop_state: Option<String>,
}

/// The Processes tab payload for one Instance (AJ-E410 Req.2/3).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessesInfo {
    /// Live tree rows (root first, breadth-first descendants).
    pub rows: Vec<ProcessRowInfo>,
    /// 5-minute 2 s resource ring (the I-07 sparkline source).
    pub spark: Vec<TreeSampleInfo>,
    /// Confirmed orphans for this Instance.
    pub orphans: Vec<OrphanInfo>,
    /// Honest unavailability note (adopted/deskto-kind without a tree):
    /// `resources unavailable`, never flat-zero rows.
    pub unavailable_reason: Option<String>,
}

/// Outcome payload for orphan stop actions.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StopOutcomeInfo {
    /// The pid the action targeted.
    pub pid: u32,
    /// UI state: `gone`, `still alive - escalated to SIGKILL`, or the
    /// refusal reason.
    pub state: String,
}

/// `Verify owner` payload: the re-checked ppid chain evidence.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OrphanVerifyInfo {
    /// The pid examined.
    pub pid: u32,
    /// True when the process is alive and non-zombie.
    pub alive: bool,
    /// Current parent pid, when readable.
    #[ts(type = "number | null")]
    pub parent_pid: Option<u32>,
    /// Human note (e.g. `re-parented to init/launchd`).
    pub note: String,
}

/// The live Processes tab payload for one Instance (rows + spark +
/// orphans); rollups above Instance level aggregate several of these
/// read-only in the UI (Req.2).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Instance is unknown.
#[tauri::command]
pub fn instance_processes(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<ProcessesInfo> {
    instance_processes_arc(&state, &session_id)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Instance is unknown.
pub fn instance_processes_arc(state: &Arc<AppState>, session_id: &str) -> AppResult<ProcessesInfo> {
    let (record_exists, supervised) = {
        let storage = lock_storage_arc(state)?;
        let exists = storage
            .sessions()
            .get(&SessionId::new(session_id.to_string()))
            .is_ok();
        (
            exists,
            state
                .supervisor
                .is_supervised(&SessionId::new(session_id.to_string())),
        )
    };
    if !record_exists {
        return Err(AppError::from(CoreError::NotFound {
            entity: "instance",
            id: session_id.to_string(),
        }));
    }
    let session = SessionId::new(session_id.to_string());
    let session_id = session_id.to_string();
    let (rows, spark, unavailable) = {
        let Ok(collector) = state.resources.lock() else {
            return Ok(unavailable_processes("resources unavailable"));
        };
        let rows = collector.rows(&session).unwrap_or_default();
        let spark = collector.ring(&session);
        // Adopted/desktop-kind sessions this app never supervised have
        // no tree to read: honest unavailability, never flat-zero rows.
        let unavailable =
            (!supervised && rows.is_empty()).then(|| "resources unavailable".to_string());
        (rows, spark, unavailable)
    };
    let now = Timestamp::now().millis();
    let orphans = state
        .orphans
        .orphans_of(&session)
        .into_iter()
        .map(|orphan| OrphanInfo {
            session_id: session_id.clone(),
            pid: orphan.pid,
            name: orphan.name,
            rss_bytes: orphan.rss_bytes,
            parent_label: orphan.parent_label,
            age_since_parent_exit_ms: now - orphan.parent_exit_ms,
            stop_state: state
                .orphans
                .stop_state(orphan.pid)
                .map(|outcome| outcome.state_label().to_string()),
        })
        .collect();
    Ok(ProcessesInfo {
        rows: rows
            .into_iter()
            .map(|row| ProcessRowInfo {
                pid: row.pid,
                parent_pid: row.parent_pid,
                name: row.name,
                cpu_pct: row.cpu_pct,
                rss_bytes: row.rss_bytes,
                age_ms: row.age_ms,
                child_count: row.child_count,
                depth: row.depth,
            })
            .collect(),
        spark: spark
            .into_iter()
            .map(|sample| TreeSampleInfo {
                at_ms: sample.at_ms,
                cpu_pct: sample.cpu_pct,
                rss_bytes: sample.rss_bytes,
            })
            .collect(),
        orphans,
        unavailable_reason: unavailable,
    })
}

/// An honest `resources unavailable` payload (Req.7 — never flat zeros).
fn unavailable_processes(reason: &str) -> ProcessesInfo {
    ProcessesInfo {
        rows: Vec::new(),
        spark: Vec::new(),
        orphans: Vec::new(),
        unavailable_reason: Some(reason.to_string()),
    }
}

// --- AJ-E505: the port occupancy map ----------------------------------

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
        .unwrap_or_default()
}

/// One port-inventory row (the API DTO; `NotManaged` rows carry
/// `killable: false` and NO stop/kill command ids — the F3 guard).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct PortRowInfo {
    /// The listening port (the headline).
    pub port: u16,
    /// The owning process id (the evidence).
    pub pid: u32,
    /// `managed-orphan` | `managed-running` | `re-adopted` |
    /// `not-managed`.
    pub class: String,
    /// Owning Instance, when managed.
    pub session: Option<String>,
    /// Provider id, when managed.
    pub provider: Option<String>,
    /// Account id, when managed.
    pub account: Option<String>,
    /// Project id, when managed.
    pub project: Option<String>,
    /// Listener age from the kernel process start, ms.
    pub listener_age_ms: Option<i64>,
    /// Process display name, when managed.
    pub name: Option<String>,
    /// The well-known dev-port hint (a LABEL, never identity).
    pub hint: Option<String>,
    /// False exactly for `not-managed` rows: no kill affordance, ever.
    pub killable: bool,
}

/// One sustained-breach hog fact (Req.5 — the Cleanup Centre's input).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct HogFactInfo {
    /// The hog pid (helper for RSS rules, the root for tree rules).
    pub pid: u32,
    /// Owning Instance.
    pub session: String,
    /// Provider id.
    pub provider: String,
    /// Live (or last-seen) RSS, bytes.
    pub rss_bytes: u64,
    /// Live CPU%, when sampled.
    pub cpu_pct: Option<f64>,
    /// Which rule fired: `helper-rss>2GiB` | `tree-cpu>90%` |
    /// `tree-rss>p95(provider)`.
    pub rule: String,
}

/// The inventory response: rows + the honest probe note + hog facts.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default)]
pub struct PortInventoryInfo {
    /// Rows sorted by port.
    pub rows: Vec<PortRowInfo>,
    /// `Some` when the platform has no probe or the OS call failed —
    /// the honest note, never an empty map presented as truth.
    pub note: Option<String>,
    /// Sustained-breach hogs (60 s window).
    pub hogs: Vec<HogFactInfo>,
}

/// The pre-launch collision warning (advisor informs, never blocks).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct PortWarningInfo {
    /// The contested port.
    pub port: u16,
    /// The orphan holding it.
    pub pid: u32,
    /// The Instance label (`Provider (id-prefix)`).
    pub instance: String,
    /// How long the holder has lived, ms.
    pub age_ms: i64,
    /// The copy-exact warning (Req.4).
    pub message: String,
}

/// The port inventory (Req.3): every listener classified as exactly
/// one of managed-orphan / managed-running / re-adopted /
/// not-managed; unmanaged rows are display-only.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn port_inventory(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PortInventoryInfo> {
    Ok(port_inventory_inner(&state))
}

/// The pre-launch warning (Req.4): a project's expected dev port
/// (learned from its instances' observed listeners) held by a managed
/// orphan. Launch itself is NEVER blocked — the UI offers `Launch
/// anyway` beside `Free this port`.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn launch_port_warning(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: Option<String>,
) -> AppResult<Option<PortWarningInfo>> {
    Ok(launch_port_warning_inner(&state, project_id))
}

/// Humanized age for the copy-exact warning: `45s` / `12m` / `3h` /
/// `2d`.
fn human_age(ms: i64) -> String {
    let secs = ms.div_euclid(1000).max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3_600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

/// The real probe for THIS platform (query-time freshness; the tick
/// join stores the durable view).
fn os_listeners() -> Result<Vec<juggler_supervisor::Listener>, String> {
    use juggler_supervisor::PortProbe as _;
    #[cfg(unix)]
    let probe = juggler_supervisor::NetstatProbe;
    #[cfg(not(unix))]
    let probe = juggler_supervisor::UnsupportedProbe;
    probe.listeners().map_err(|error| error.to_string())
}

/// Every ledger row of the local machine's sessions (all states —
/// dead roots matter for classification).
/// EVERY ledger row, all machines (AJ-E511: the "All machines"
/// inventory scope — foreign rows are display-only in the UI). Walks
/// the composite keys, then reads each key — the same session on two
/// machines stays two groups.
fn all_ledger_rows(state: &Arc<AppState>) -> Vec<juggler_core::storage::LedgerRow> {
    use juggler_supervisor::tree::LedgerStore as _;
    let mut rows = Vec::new();
    for (session, machine) in state.ledger.live_session_machines().unwrap_or_default() {
        if let Ok(mut key_rows) = state
            .ledger
            .rows_for_session_on_machine(&SessionId::new(session), &MachineId::new(machine))
        {
            rows.append(&mut key_rows);
        }
    }
    rows
}

fn local_ledger_rows(state: &Arc<AppState>) -> Vec<juggler_core::storage::LedgerRow> {
    use juggler_supervisor::tree::LedgerStore as _;
    let machine = local_machine_id(state);
    let mut rows = Vec::new();
    let mut sessions = std::collections::HashSet::new();
    if let Ok(candidates) = state.ledger.candidates(&machine) {
        for row in candidates {
            sessions.insert(row.session_id.clone());
        }
    }
    for session in sessions {
        // COMPOSITE key (AJ-E511): a session may exist on two
        // machines — session-only reads would leak foreign rows into
        // this node's scope (the fleet-scale false-kill trap).
        if let Ok(mut session_rows) = state
            .ledger
            .rows_for_session_on_machine(&SessionId::new(session), &machine)
        {
            rows.append(&mut session_rows);
        }
    }
    rows
}

// --- AJ-E506: the Cleanup Centre (read-only, Tier 0) -------------------

/// One inventory row's verification badge (AJ-E503 at query time).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct VerificationBadge {
    /// `verified` | `unverifiable` | `unknown` (no recorded facts).
    pub verdict: String,
    /// The facts behind the verdict (the expansion's check result).
    pub detail: String,
}

/// The ownership evidence chain (Req.4 — `Why does Juggler think this
/// is mine?`).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct EvidenceChain {
    /// The Instance's root pid (spawned-by).
    pub root_pid: Option<u32>,
    /// First-seen parent pid (the ppid at record time).
    pub parent_pid: Option<u32>,
    /// The row's PGID.
    pub pgid: Option<u32>,
    /// True when the PGID left the root's group (E502's flag).
    pub escaped_group: bool,
    /// The exe/start-time check result (AJ-E503 at query time).
    pub verification: VerificationBadge,
    /// First seen, epoch ms.
    pub first_seen: i64,
    /// Last seen, epoch ms.
    pub last_seen: i64,
}

/// One read-only row (the shared `ProcessRow`'s data). NO kill fields
/// exist on this DTO by construction (Req.7).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupRow {
    /// The machine that owns this row (AJ-E511) — `local_machine`
    /// on the inventory names this node; foreign rows render
    /// display-only (no kill affordance).
    pub machine_id: String,
    /// OS process id.
    pub pid: u32,
    /// Process-group id.
    pub pgid: Option<u32>,
    /// `root` | `helper`.
    pub role: String,
    /// Ledger state (`running` | `orphaned` | `unverifiable` | `exited`).
    pub state: String,
    /// The pass2 §2 label, VERBATIM — except down-period orphans,
    /// which carry the AJ-E510 label `left behind while Juggler was
    /// closed` (their root died while the app was not running).
    pub state_label: String,
    /// True for helpers of roots that died while Juggler was closed
    /// (AJ-E510 Req.2) — the distinct down-period marker.
    pub down_period: bool,
    /// Role guess — always labelled a guess (E504 note).
    pub role_guess: String,
    /// Executable path (truncated for display, full on expand).
    pub exe_path: String,
    /// Truncated command line.
    pub cmdline: String,
    /// Age from the kernel start time, ms.
    pub age_ms: Option<i64>,
    /// Live RSS bytes (frozen at last tick for exited rows).
    pub rss_bytes: Option<u64>,
    /// Live CPU%.
    pub cpu_pct: Option<f64>,
    /// Ports held (csv) — the collision evidence.
    pub ports: Option<String>,
    /// True when a sustained hog rule fired (E505).
    pub hog: bool,
    /// Which hog rule(s) fired.
    pub hog_rules: Vec<String>,
    /// The verification badge + evidence chain (expansion).
    pub evidence: EvidenceChain,
}

/// One Instance group (healthy trees collapsed by default).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupGroup {
    /// The Instance (session) id.
    pub session: String,
    /// Provider id.
    pub provider: String,
    /// Account id.
    pub account: Option<String>,
    /// Project id.
    pub project: Option<String>,
    /// Machine id (AJ-E511).
    pub machine: String,
    /// True when the tree is healthy (collapse by default).
    pub healthy: bool,
    /// The rows.
    pub rows: Vec<CleanupRow>,
}

/// One Cleanup Centre history row (AJ-E509: the audit log's read
/// shape, newest first).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupHistoryInfo {
    /// The audit history (7 days), newest first.
    pub rows: Vec<juggler_core::storage::AuditOutcomeInfo>,
    /// The copy-exact empty note when there are no records.
    pub empty_note: Option<String>,
}

/// The Cleanup Centre response.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default)]
pub struct CleanupInventoryInfo {
    /// Groups matching the filter.
    pub groups: Vec<CleanupGroup>,
    /// The executing node's machine id (AJ-E511): rows whose
    /// `machine_id` equals this are `This Mac`; anything else is a
    /// future-synced foreign row — display-only.
    pub local_machine: String,
    /// Instances fully clean (the empty-state copy's N).
    pub clean_instances: usize,
    /// True when the ledger is degraded (AJ-E501 banner + read-only).
    pub ledger_degraded: bool,
    /// The History filter's audit rows (AJ-E509 Req.4; `None` for
    /// every other filter).
    pub history: Option<CleanupHistoryInfo>,
}

/// `pass2` §2 state labels, VERBATIM (Req.2).
#[must_use]
pub fn cleanup_state_label(state: &str) -> &'static str {
    match state {
        "orphaned" => "Still running after its Instance closed",
        "re-adopted" | "adopted" => "Running — re-adopted",
        "unverifiable" => "Unverifiable — will not be touched",
        "running" => "Running",
        "exited" => "Exited",
        "starting" => "Starting",
        _ => "Unknown",
    }
}

/// The read-only Cleanup Centre inventory (Tier 0 — zero signalling:
/// this command registers no kill surface and its DTOs carry none).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn cleanup_inventory(
    state: tauri::State<'_, Arc<AppState>>,
    filter: String,
    machine_scope: Option<String>,
) -> AppResult<CleanupInventoryInfo> {
    Ok(cleanup_inventory_inner(
        &state,
        &filter,
        machine_scope.as_deref() == Some("all"),
    ))
}

/// `cleanup_inventory` sans-Tauri (tests + dogfood evidence).
#[allow(clippy::similar_names)] // row/root_row read naturally here
#[allow(clippy::too_many_lines)] // one pass per filter; splitting hides the grouping rules
pub fn cleanup_inventory_inner(
    state: &Arc<AppState>,
    filter: &str,
    all_machines: bool,
) -> CleanupInventoryInfo {
    let now = now_millis();
    let degraded = state.supervisor.ledger_degraded();
    // History (7 days) reads the AUDIT LOG (AJ-E509 Req.4) — the
    // outcome vocabulary is one language with AJ-E507's.
    if filter == "history" {
        let since = now - 7 * 24 * 3_600_000;
        let rows = state.audit.history(since).unwrap_or_default();
        let empty_note = rows
            .is_empty()
            .then(|| "No cleanups yet — automatic cleanup is off by default.".to_string());
        return CleanupInventoryInfo {
            groups: Vec::new(),
            local_machine: local_machine_id(state).as_str().to_string(),
            clean_instances: 0,
            ledger_degraded: degraded,
            history: Some(CleanupHistoryInfo { rows, empty_note }),
        };
    }
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let hogs = hog_facts(state, now);
    let live_rows: std::collections::HashMap<u32, juggler_supervisor::ProcessRow> = state
        .resources
        .lock()
        .ok()
        .map(|collector| {
            collector
                .stats_snapshot()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|session| {
                    collector
                        .rows(&session)
                        .into_iter()
                        .flatten()
                        .map(|row| (row.pid, row))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .unwrap_or_default();
    let table = state.supervisor.snapshot_table();

    let mut groups: Vec<CleanupGroup> = Vec::new();
    let mut clean = 0_usize;
    // Group rows per (session, machine) — the D-13 composite key: the
    // same session may exist on two machines and MUST render as two
    // groups (attribution stays joinable, never merged). The default
    // scope reads THIS machine only; "all" adds future-synced
    // foreign rows (display-only in the UI).
    let mut by_key: std::collections::HashMap<
        (String, String),
        Vec<juggler_core::storage::LedgerRow>,
    > = std::collections::HashMap::new();
    for row in if all_machines {
        all_ledger_rows(state)
    } else {
        local_ledger_rows(state)
    } {
        by_key
            .entry((row.session_id.clone(), row.machine_id.clone()))
            .or_default()
            .push(row);
    }
    let mut session_order: Vec<(String, String)> = by_key.keys().cloned().collect();
    session_order.sort();
    for (session, row_machine) in session_order {
        let rows_raw = &by_key[&(session.clone(), row_machine.clone())];
        let provider = rows_raw
            .first()
            .map(|row| row.provider.clone())
            .unwrap_or_default();
        let account = rows_raw.first().and_then(|row| row.account_id.clone());
        let project = rows_raw.first().and_then(|row| row.project_id.clone());
        let machine = rows_raw
            .first()
            .map(|row| row.machine_id.clone())
            .unwrap_or_default();
        let root_row = rows_raw.iter().find(|row| row.role == "root");
        let root_pid = root_row.and_then(|row| row.pid);
        let root_alive = root_row.is_some_and(|row| row.state == "running");
        let session_adopted = adopted.contains(&session);

        let mut rows: Vec<CleanupRow> = Vec::new();
        let mut any_attention = false;
        for row in rows_raw {
            let pid = row.pid.unwrap_or_default();
            if pid == 0 {
                continue;
            }
            // History filter: exited rows within 7 days only.
            if filter == "history" && row.state != "exited" {
                continue;
            }
            if filter != "history" && row.state == "exited" {
                continue;
            }
            // Query-time verification (E503): table facts vs record.
            let verification = if row.state == "unverifiable" {
                VerificationBadge {
                    verdict: "unverifiable".into(),
                    detail: "identity check failed — permanently untouched".into(),
                }
            } else if let Some(facts) = table.as_ref().and_then(|t| t.facts.get(&pid)) {
                let start_ok = row
                    .started_at
                    .is_none_or(|start| start == 0 || start == facts.start_time);
                let exe_ok = !row.exe_path.starts_with('/')
                    || facts.exe.as_deref().is_none_or(|exe| exe == row.exe_path);
                if start_ok && exe_ok {
                    VerificationBadge {
                        verdict: "verified".into(),
                        detail: format!(
                            "start-time and exe match the live process (start {}, exe {:?})",
                            facts.start_time,
                            facts.exe.as_deref().unwrap_or("?")
                        ),
                    }
                } else {
                    VerificationBadge {
                        verdict: "unverifiable".into(),
                        detail: "live start-time or exe differs from the record".into(),
                    }
                }
            } else if row.state == "exited" {
                VerificationBadge {
                    verdict: "unknown".into(),
                    detail: "process exited — no live check applies".into(),
                }
            } else {
                VerificationBadge {
                    verdict: "unknown".into(),
                    detail: "not in the current process table".into(),
                }
            };

            let row_hogs: Vec<&HogFactInfo> = hogs
                .iter()
                .filter(|hog| hog.pid == pid || (hog.pid == 0 && hog.session == session))
                .collect();
            let port_collision = row.state == "orphaned" && row.listens_on.is_some();
            let effective_state = if session_adopted && row.state == "running" {
                "adopted"
            } else {
                row.state.as_str()
            };
            let needs_attention = row.state == "orphaned" || !row_hogs.is_empty() || port_collision;
            if needs_attention {
                any_attention = true;
            }
            // Filter sets.
            let matches = match filter {
                "needs-attention" => needs_attention,
                "re-adopted" => session_adopted,
                _ => true, // all-managed + history (its row gate is above)
            };
            if !matches {
                continue;
            }
            let live = live_rows.get(&pid);
            // AJ-E510 Req.2: the durable down-period marker (set by
            // crash recovery) — distinct from live-session orphans.
            let down_period = effective_state == "orphaned" && row.down_period;
            let state_label = if down_period {
                "left behind while Juggler was closed".to_string()
            } else {
                cleanup_state_label(effective_state).to_string()
            };
            rows.push(CleanupRow {
                machine_id: row.machine_id.clone(),
                pid,
                pgid: row.pgid,
                role: row.role.clone(),
                state: effective_state.to_string(),
                state_label,
                down_period,
                role_guess: format!("{} (guess)", row.exe_path.rsplit('/').next().unwrap_or("?")),
                exe_path: row.exe_path.clone(),
                cmdline: row.cmdline_trunc.clone(),
                age_ms: row
                    .started_at
                    .map(|start| (now - start.saturating_mul(1000)).max(0)),
                rss_bytes: live
                    .map(|l| l.rss_bytes)
                    .or(row.peak_rss.map(i64::unsigned_abs)),
                cpu_pct: live.and_then(|l| l.cpu_pct).or(row.peak_cpu),
                ports: row.listens_on.clone(),
                hog: !row_hogs.is_empty(),
                hog_rules: row_hogs.iter().map(|hog| hog.rule.clone()).collect(),
                evidence: EvidenceChain {
                    root_pid,
                    parent_pid: row.parent_pid,
                    pgid: row.pgid,
                    escaped_group: row.escaped_group,
                    verification,
                    first_seen: row.first_seen,
                    last_seen: row.last_seen,
                },
            });
        }
        if rows.is_empty() {
            continue;
        }
        let healthy = !any_attention && root_alive;
        if healthy && filter == "needs-attention" {
            // Healthy trees hold no attention rows by construction.
        }
        if healthy
            && root_alive
            && rows_raw
                .iter()
                .all(|row| row.state == "running" || row.state == "exited")
        {
            clean += 1;
        }
        groups.push(CleanupGroup {
            session: session.clone(),
            provider,
            account,
            project,
            machine,
            healthy,
            rows,
        });
    }
    CleanupInventoryInfo {
        groups,
        local_machine: local_machine_id(state).as_str().to_string(),
        clean_instances: clean,
        ledger_degraded: degraded,
        history: None,
    }
}

/// Exports the 7-day cleanup audit as JSONL (AJ-E509 Req.4). The
/// first line is the redaction note (itself valid JSON): cmdlines are
/// hashed; full command lines never persisted. User-initiated only.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn cleanup_audit_export(state: tauri::State<'_, Arc<AppState>>) -> AppResult<String> {
    Ok(cleanup_audit_export_inner(&state))
}

/// `cleanup_audit_export` sans-Tauri (tests + evidence).
pub fn cleanup_audit_export_inner(state: &Arc<AppState>) -> String {
    let since = now_millis() - 7 * 24 * 3_600_000;
    let rows = state.audit.rows_since(since).unwrap_or_default();
    let mut lines = vec![r#"{"note":"cleanup audit export — command lines are hashed and truncated; full cmdlines are never stored"}"#.to_string()];
    for row in rows {
        if let Ok(line) = serde_json::to_string(&row) {
            lines.push(line);
        }
    }
    lines.join("\n")
}

/// `port_inventory` sans-Tauri (tests + evidence runs).
pub fn port_inventory_inner(state: &Arc<AppState>) -> PortInventoryInfo {
    let now = now_millis();
    let listeners = match os_listeners() {
        Ok(listeners) => listeners,
        Err(note) => {
            return PortInventoryInfo {
                rows: Vec::new(),
                note: Some(note),
                hogs: Vec::new(),
            };
        }
    };
    let rows = local_ledger_rows(state);
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let inventory = juggler_supervisor::classify_listeners(&listeners, &rows, &adopted, now);
    PortInventoryInfo {
        rows: inventory
            .into_iter()
            .map(|row| PortRowInfo {
                port: row.port,
                pid: row.pid,
                class: row.class.as_str().to_string(),
                session: row.session,
                provider: row.provider,
                account: row.account,
                project: row.project,
                listener_age_ms: row.listener_age_ms,
                name: row.name,
                hint: row.hint.map(str::to_string),
                killable: row.class != juggler_supervisor::ListenerClass::NotManaged,
            })
            .collect(),
        note: state
            .resources
            .lock()
            .ok()
            .and_then(|collector| collector.ports_note()),
        hogs: hog_facts(state, now),
    }
}

/// Sustained-breach hog facts (Req.5): a helper >2 GiB, a tree >90%
/// of one core, or a tree above its Provider's learned p95 — each
/// sustained over the 60 s window of the collector's rings. The p95
/// rule needs ≥3 Provider trees to compare against (an honest floor;
/// smaller populations never fire it).
#[allow(clippy::similar_names)] // state/stats read naturally here
fn hog_facts(state: &Arc<AppState>, now_ms: i64) -> Vec<HogFactInfo> {
    const HOG_WINDOW_MS: i64 = 60_000;
    // AJ-E512: the thresholds are settings-backed (defaults per SPEC
    // §10) so tests can lower them without faking gigabytes; the
    // production values never change here.
    let helper_rss_limit = setting_u64(state, "hog_helper_rss_limit_mb", 2_048) * 1024 * 1024;
    let cpu_pct_limit = setting_f64(state, "hog_cpu_pct", 90.0);
    let Some(collector) = state.resources.lock().ok() else {
        return Vec::new();
    };
    let stats = collector.stats_snapshot();
    let contexts: Vec<(SessionId, String)> = state
        .supervisor
        .spawn_contexts_snapshot()
        .into_iter()
        .map(|(session, ctx)| (session, ctx.provider.clone()))
        .collect();
    let mut facts = Vec::new();
    for (session, provider) in &contexts {
        let Some(tree_stats) = stats.get(session) else {
            continue;
        };
        // Sustained = every ring sample in the window breaches.
        let ring = collector.ring(session);
        let in_window: Vec<_> = ring
            .iter()
            .filter(|sample| now_ms - sample.at_ms <= HOG_WINDOW_MS)
            .collect();
        if in_window.len() < 2 {
            continue; // not enough samples to call anything sustained
        }
        let cpu_sustained = in_window
            .iter()
            .all(|sample| sample.cpu_pct.is_some_and(|cpu| cpu > cpu_pct_limit));
        if cpu_sustained {
            facts.push(HogFactInfo {
                pid: 0, // tree-level: the Cleanup Centre resolves the
                // Instance's live root pid at render time
                session: session.as_str().to_string(),
                provider: provider.clone(),
                rss_bytes: tree_stats.rss_bytes,
                cpu_pct: tree_stats.cpu_pct,
                rule: "tree-cpu>90%".to_string(),
            });
        }
        // The per-Provider p95 population: the OTHER trees of this
        // Provider (self-excluded), needing at least three peers.
        let peers: Vec<u64> = contexts
            .iter()
            .filter(|(peer, peer_provider)| peer_provider == provider && peer != session)
            .filter_map(|(peer, _)| stats.get(peer).map(|s| s.rss_bytes))
            .collect();
        if peers.len() >= 3 {
            let mut sorted = peers.clone();
            sorted.sort_unstable();
            let p95 = sorted[sorted.len() * 95 / 100];
            let rss_sustained = in_window.iter().all(|sample| sample.rss_bytes > p95);
            if rss_sustained && tree_stats.rss_bytes > p95 {
                facts.push(HogFactInfo {
                    pid: 0,
                    session: session.as_str().to_string(),
                    provider: provider.clone(),
                    rss_bytes: tree_stats.rss_bytes,
                    cpu_pct: tree_stats.cpu_pct,
                    rule: "tree-rss>p95(provider)".to_string(),
                });
            }
        }
        // Single-helper RSS: the live rows of the tree (root excluded).
        for row in collector.rows(session).into_iter().flatten().skip(1) {
            if row.rss_bytes > helper_rss_limit {
                facts.push(HogFactInfo {
                    pid: row.pid,
                    session: session.as_str().to_string(),
                    provider: provider.clone(),
                    rss_bytes: row.rss_bytes,
                    cpu_pct: row.cpu_pct,
                    rule: "helper-rss>2GiB".to_string(),
                });
            }
        }
    }
    facts
}

/// `launch_port_warning` sans-Tauri: the learned expected ports of
/// `project_id`'s instances (their observed `listens_on` history,
/// any row state) crossed with CURRENT orphan-held listeners.
pub fn launch_port_warning_inner(
    state: &Arc<AppState>,
    project_id: Option<String>,
) -> Option<PortWarningInfo> {
    let now = now_millis();
    let expected: std::collections::HashSet<u16> = local_ledger_rows(state)
        .into_iter()
        .filter(|row| {
            project_id
                .as_deref()
                .is_none_or(|project| row.project_id.as_deref() == Some(project))
        })
        .filter_map(|row| row.listens_on)
        .flat_map(|ports| {
            ports
                .split(',')
                .filter_map(|port| port.parse::<u16>().ok())
                .collect::<Vec<_>>()
        })
        .collect();
    if expected.is_empty() {
        return None;
    }
    let listeners = os_listeners().ok()?;
    let rows = local_ledger_rows(state);
    let adopted: std::collections::HashSet<String> = state
        .supervisor
        .alive_adopted_sessions()
        .into_iter()
        .map(|session| session.as_str().to_string())
        .collect();
    let inventory = juggler_supervisor::classify_listeners(&listeners, &rows, &adopted, now);
    inventory
        .into_iter()
        .find(|row| {
            expected.contains(&row.port)
                && row.class == juggler_supervisor::ListenerClass::ManagedOrphan
        })
        .map(|row| {
            let row_port = row.port;
            let instance = format!(
                "{} ({})",
                row.provider.as_deref().unwrap_or("instance"),
                row.session
                    .as_deref()
                    .map(str::chars)
                    .map(|mut chars| chars.by_ref().take(8).collect::<String>())
                    .unwrap_or_default()
            );
            let age_ms = row.listener_age_ms.unwrap_or_default();
            let message = format!(
                ":{} is already held by an orphan from {instance} ({}).",
                row_port,
                human_age(age_ms)
            );
            PortWarningInfo {
                port: row_port,
                pid: row.pid,
                instance,
                age_ms,
                message,
            }
        })
}

/// Stops one orphan: the allow-list guard refuses PIDs that were not
/// recorded as descendants (fail closed, PROPOSAL.md:47), then the
/// TERM→KILL ladder runs with the post-stop re-read built in (Req.6).
/// Every kill lands in the journal's audit trail (D-08) AND the
/// append-only cleanup audit (AJ-E509: intent before signal).
///
/// # Errors
///
/// [`AppError::AuditUnavailable`] when the audit intent fails (no
/// audit, no kill); otherwise the typed stop outcome.
#[tauri::command]
pub fn orphan_stop(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
) -> AppResult<StopOutcomeInfo> {
    orphan_stop_inner(&state, &SessionId::new(session_id), pid)
}

/// Stops every confirmed orphan of one Instance (the confirm popover
/// lists all PIDs before this runs; each kill is audited).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn orphan_stop_all(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Vec<StopOutcomeInfo>> {
    let session = SessionId::new(session_id);
    let mut out = Vec::new();
    for orphan in state.orphans.orphans_of(&session) {
        // One kill at a time: a failed audit intent stands the run
        // down HERE (typed error out) — never silently continue.
        out.push(orphan_stop_inner(&state, &session, orphan.pid)?);
    }
    Ok(out)
}

/// Shared stop path for the command and `Stop all` (guard → audit
/// intent → ladder → tracker).
///
/// # Errors
///
/// [`AppError::AuditUnavailable`] when the audit intent fails (no
/// audit, no kill).
pub fn orphan_stop_inner(
    state: &Arc<AppState>,
    session: &SessionId,
    pid: u32,
) -> Result<StopOutcomeInfo, AppError> {
    orphan_stop_audited(state, state.audit.as_ref(), session, pid)
}

/// `orphan_stop_inner` with an injected audit writer (the
/// fault-injection gate's seam; production passes the real repo).
///
/// # Errors
///
/// [`AppError::AuditUnavailable`] when the audit intent fails (no
/// audit, no kill); otherwise the typed stop outcome.
pub fn orphan_stop_audited(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    session: &SessionId,
    pid: u32,
) -> Result<StopOutcomeInfo, AppError> {
    if !state.orphans.allows(session, pid) {
        let outcome = juggler_supervisor::StopOutcome::RefusedNotRecorded;
        state.orphans.record_stop(pid, outcome.clone());
        return Ok(StopOutcomeInfo {
            pid,
            state: outcome.state_label().to_string(),
        });
    }
    // AJ-E509: NO AUDIT, NO KILL — the intent row is written BEFORE
    // the first signal; a failed write refuses the kill with the
    // typed error (never a silent `let _ =`), and the ladder runs
    // only behind a recorded intent.
    let (intent_id, _ports_before) = audit_kill_intent(state, audit, session, pid)?;
    let outcome = juggler_supervisor::stop_pid_ladder(pid, Duration::from_secs(2), |pid| {
        juggler_supervisor::process_alive(pid).is_some()
    });
    state.orphans.record_stop(pid, outcome.clone());
    if let Some(journal) = state.supervisor.journal_ref() {
        let _ = journal.record_killed(session, pid, "manual", outcome.state_label());
    }
    // Complete the audit with the per-PID outcome and the honest
    // post-ladder port state (a still-held port is a fact, never a
    // success claim).
    let ports_after = ports_held_by(pid);
    if let Some(id) = intent_id {
        // Best-effort completion: the intent (the accountability
        // floor) already exists; a failed completion degrades to the
        // truthful pending record, read as `outcome unknown`.
        let _ = audit.complete(id, outcome.state_label(), ports_after.as_deref());
    }
    Ok(StopOutcomeInfo {
        pid,
        state: outcome.state_label().to_string(),
    })
}

/// The audit-writer seam (AJ-E509 Req.3's fault-injection gate):
/// production is the repo; tests inject failures to prove NO AUDIT,
/// NO KILL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The audit-writer failure (there is nothing to distinguish — any
/// failure refuses the kill).
pub struct AuditWriteError;

impl std::fmt::Display for AuditWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cleanup audit write failed")
    }
}

/// The audit-writer seam (production = the repo; tests inject
/// failures for the NO-AUDIT-NO-KILL gate).
pub trait AuditWriter {
    /// Writes the intent row; `Err` refuses the kill.
    ///
    /// # Errors
    ///
    /// [`AuditWriteError`] when the store fails — NO AUDIT, NO KILL.
    fn intent(
        &self,
        record: &juggler_core::storage::CleanupAuditRecord,
    ) -> Result<i64, AuditWriteError>;

    /// Completes the record with the ladder outcome (best-effort).
    ///
    /// # Errors
    ///
    /// [`AuditWriteError`] when the store fails.
    fn complete(
        &self,
        id: i64,
        outcome: &str,
        ports_after: Option<&str>,
    ) -> Result<(), AuditWriteError>;
}

impl AuditWriter for juggler_core::storage::CleanupAuditRepo {
    fn intent(
        &self,
        record: &juggler_core::storage::CleanupAuditRecord,
    ) -> Result<i64, AuditWriteError> {
        juggler_core::storage::CleanupAuditRepo::intent(self, record).map_err(|_| AuditWriteError)
    }

    fn complete(
        &self,
        id: i64,
        outcome: &str,
        ports_after: Option<&str>,
    ) -> Result<(), AuditWriteError> {
        juggler_core::storage::CleanupAuditRepo::complete(
            self,
            id,
            outcome,
            ports_after,
            now_millis(),
        )
        .map_err(|_| AuditWriteError)
    }
}

/// Writes the audit intent for a manual kill (AJ-E509 Req.2/3) and
/// captures the pre-ladder ports. `Err(AppError::AuditUnavailable)`
/// REFUSES the kill — the caller surfaces it.
///
/// # Errors
///
/// [`AppError::AuditUnavailable`] when the intent write fails.
fn audit_kill_intent(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    session: &SessionId,
    pid: u32,
) -> Result<(Option<i64>, Option<String>), AppError> {
    use juggler_core::storage::{ACTOR_PREFIX_USER, CleanupAuditRecord};
    let now = now_millis();
    let ports_before = ports_held_by(pid);
    let record = CleanupAuditRecord {
        id: 0,
        at_ms: now,
        actor: format!("{ACTOR_PREFIX_USER}orphan_stop"),
        pid,
        pgid: None,
        exe_path: format!("pid:{pid}"),
        cmdline_hash: format!("pid:{pid}"),
        cmdline_trunc: format!("PID {pid} (recorded orphan)"),
        session_id: session.as_str().to_string(),
        provider: "unknown".into(),
        account_id: None,
        project_id: None,
        machine_id: local_machine_id(state).as_str().to_string(),
        identity: "allow-list guard (journal-recorded descendant)".into(),
        scope: "pid".into(),
        outcome: String::new(),
        ports_before: ports_before.clone(),
        ports_after: None,
    };
    match audit.intent(&record) {
        Ok(id) => Ok((Some(id), ports_before)),
        Err(AuditWriteError) => Err(AppError::AuditUnavailable),
    }
}

/// The ports a pid holds RIGHT NOW (the audit's before/after facts).
fn ports_held_by(pid: u32) -> Option<String> {
    let listeners = os_listeners().ok()?;
    let ports: Vec<String> = listeners
        .iter()
        .filter(|listener| listener.pid == pid)
        .map(|listener| listener.port.to_string())
        .collect();
    (!ports.is_empty()).then(|| ports.join(","))
}

// --- AJ-E507: verify-before-kill sheet + manual TERM→KILL ladder ---

/// The kill sheet's default grace (pass2 §9.3 — matches the
/// `stop_session` grace).
pub const KILL_GRACE_SECS: u64 = 5;

/// The exact refusal copy when click-time verification fails
/// (Req.2 — the sheet renders it with the kill button REMOVED).
pub const KILL_REFUSAL_COPY: &str =
    "This PID now belongs to a different program — Juggler will not touch it.";
/// The AJ-E511 foreign-machine refusal (Req.2): typed
/// [`juggler_supervisor::SupervisorError::WrongMachine`] under the
/// hood — this is its sheet-facing copy.
pub const KILL_WRONG_MACHINE_COPY: &str =
    "This process record belongs to another machine — Juggler will not touch it.";

/// The exact unsaved-state warning copy (Req.5).
fn unsaved_state_copy(role: &str) -> String {
    format!("Includes a {role} that may hold unsaved state.")
}

/// Role guess, pass2 §5 vocabulary — inferred from exe/cmdline, always
/// shown as a guess, never as fact. Pure (unit-tested).
#[must_use]
pub fn guess_role(exe: &str, cmdline: &str) -> &'static str {
    let exe_name = exe.rsplit('/').next().unwrap_or(exe).to_ascii_lowercase();
    let line = cmdline.to_ascii_lowercase();
    let hay = format!("{exe_name} {line}");
    let contains_any = |needles: &[&str]| needles.iter().any(|n| hay.contains(n));
    if contains_any(&["mcp-server", "mcp_server", "/mcp ", "mcp.json"]) {
        "McpServer"
    } else if contains_any(&[
        "postgres",
        "mysqld",
        "mariadbd",
        "redis-server",
        "mongod",
        "memcached",
        "sqlite3",
    ]) {
        "Database"
    } else if contains_any(&[
        "vite",
        "webpack",
        "esbuild",
        "next dev",
        "--watch-tags",
        "dev-server",
        "serve -s",
        "http.server",
    ]) {
        "DevServer"
    } else if contains_any(&["watchman", "fswatch", "chokidar", "nodemon", "--watch"]) {
        "Watcher"
    } else if contains_any(&[
        "cargo build",
        "make -",
        "tsc ",
        "rustc",
        "clang",
        "esbuild --build",
    ]) {
        "BuildTool"
    } else if exe_name == "sh"
        || exe_name == "bash"
        || exe_name == "zsh"
        || exe_name == "fish"
        || exe_name.ends_with("node") && line.contains("-e")
    {
        "TerminalShell"
    } else if contains_any(&["claude", "codex", "gemini", "copilot"]) {
        "Harness"
    } else {
        "Unknown"
    }
}

/// One PGID member in the verify sheet's collateral list (Req.5).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct KillMemberInfo {
    /// OS process id.
    pub pid: u32,
    /// The member's live PGID (`None` when unreadable).
    pub pgid: Option<u32>,
    /// Executable basename (display).
    pub exe: String,
    /// Truncated command line (display).
    pub cmdline: String,
    /// pass2 §5 role guess — always labelled a guess in the UI.
    pub role_guess: &'static str,
    /// The exact Req.5 warning when the role may hold unsaved state.
    pub unsaved_state_warning: Option<String>,
    /// True when this pid has a Juggler ledger row (attributed).
    pub from_ledger: bool,
}

/// The verify sheet's data (step 1): click-time identity verdict +
/// the re-read-now facts + the PGID collateral list.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct KillVerifyInfo {
    /// The machine that owns the target (AJ-E511: every kill-sheet
    /// payload names its machine — `This Mac` locally).
    pub machine_id: String,
    /// OS process id.
    pub pid: u32,
    /// Owning Instance.
    pub session: String,
    /// True only when click-time E503 verification SUCCEEDED.
    pub verified: bool,
    /// The exact refusal copy when `!verified`.
    pub refusal: Option<String>,
    /// Kernel-reported exe NOW.
    pub exe_path: String,
    /// Full command line NOW (recorded value when unreadable).
    pub cmdline: String,
    /// Working directory NOW (recorded value when unreadable).
    pub cwd: String,
    /// Ports the pid holds NOW.
    pub ports: Vec<u16>,
    /// The live PGID.
    pub pgid: Option<u32>,
    /// True when the row left the root's group (E502).
    pub escaped_group: bool,
    /// Every live PGID member (self included) with role guesses.
    pub members: Vec<KillMemberInfo>,
    /// The whole-Instance tree's live pids (scope 3's enumeration).
    pub tree_pids: Vec<u32>,
    /// `TERM → grace → KILL` | `TerminateProcess only` (verbatim).
    pub platform: String,
    /// The ladder grace, seconds.
    pub grace_secs: u64,
    /// Attribution.
    pub provider: String,
    /// Attribution.
    pub account_id: Option<String>,
    /// Attribution.
    pub project_id: Option<String>,
}

/// The verify sheet's step-1 command: re-runs the E503 identity check
/// NOW (not from the last tick), re-reads exe/cmdline/cwd/ports, and
/// enumerates the PGID collateral (Req.1/Req.5). Zero signalling.
///
/// # Errors
///
/// [`AppError::NotFound`] when the pid has no ledger row for the
/// session (the UI never offers a kill on unmanaged rows — Req.2 —
/// so this is a contract breach, not a UI state).
#[tauri::command]
pub fn kill_verify(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
) -> AppResult<KillVerifyInfo> {
    kill_verify_inner(&state, &session_id, pid)
}

/// Click-time identity verification — ONE rule shared by the sheet
/// (step 1) and the execute gate (step 3) so they can never
/// disagree. Helpers carry the kernel-recorded exe (the full
/// boot/start-time/exe triple); LIVE-supervised roots verify via the
/// handle's settled identity with start-time only — the exe moves
/// legitimately across `exec` (E503/E504's `/bin/sh`-vs-`/bin/bash`
/// aliasing rule). Anything else (no record, probe error) fails
/// CLOSED.
fn verify_kill_target(
    state: &Arc<AppState>,
    session: &SessionId,
    row: &juggler_core::storage::LedgerRow,
) -> bool {
    let Some(pid) = row.pid else {
        return false;
    };
    // AJ-E511 Req.2 — the machine gate BEFORE any kernel probe: a
    // foreign-machine row is refused by the TYPED WrongMachine path
    // (kill_verify_inner / kill_execute_as); the sweep already skips
    // foreign rows at candidate read (audit-note-only, never a
    // signal).
    if row.machine_id != local_machine_id(state).as_str() {
        return false;
    }
    if row.role == "root"
        && let Some(identity) = state.supervisor.root_identity(session)
    {
        return juggler_supervisor::tree::verify_start_time(pid, &identity)
            == juggler_supervisor::tree::Verification::Verified;
    }
    row.started_at.is_some_and(|start_time| {
        let identity = juggler_supervisor::tree::ProcessIdentity {
            pid,
            pgid: row.pgid,
            start_time,
            exe: std::path::PathBuf::from(&row.exe_path),
            boot_id: row.boot_id.clone(),
        };
        juggler_supervisor::tree::verify(pid, &identity)
            == juggler_supervisor::tree::Verification::Verified
    })
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// [`AppError::NotFound`] when no ledger row exists for
/// `(session, pid)`.
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
pub fn kill_verify_inner(
    state: &Arc<AppState>,
    session_id: &str,
    pid: u32,
) -> Result<KillVerifyInfo, AppError> {
    use juggler_supervisor::tree::LedgerStore as _;
    let session = SessionId::new(session_id.to_string());
    let rows = state.ledger.session_rows(&session).unwrap_or_default();

    let row = rows
        .iter()
        .find(|row| row.pid == Some(pid))
        .ok_or_else(|| AppError::NotFound {
            entity: "ledger row",
            id: format!("{session_id}/{pid}"),
        })?;
    // AJ-E511 Req.2 — the typed machine gate first: a foreign row
    // gets its OWN refusal copy (not the identity one) and never
    // reaches a kernel probe or a member listing.
    if row.machine_id != local_machine_id(state).as_str() {
        return Ok(KillVerifyInfo {
            machine_id: row.machine_id.clone(),
            pid,
            session: session_id.to_string(),
            verified: false,
            refusal: Some(KILL_WRONG_MACHINE_COPY.to_string()),
            exe_path: row.exe_path.clone(),
            cmdline: truncate_cmdline(&row.cmdline_trunc),
            cwd: row.cwd.clone(),
            ports: Vec::new(),
            pgid: None,
            escaped_group: row.escaped_group,
            members: Vec::new(),
            tree_pids: Vec::new(),
            platform: juggler_supervisor::ladder_platform().to_string(),
            grace_secs: KILL_GRACE_SECS,
            provider: row.provider.clone(),
            account_id: row.account_id.clone(),
            project_id: row.project_id.clone(),
        });
    }
    // Click-time verification: identity against the kernel NOW —
    // the shared rule (`verify_kill_target`). Unverifiable rows
    // (state `unverifiable`, a missing start-time, a probe error)
    // fail CLOSED — the sheet explains.
    let verified = verify_kill_target(state, &session, row);
    // Re-read NOW (the live facts; recorded values when unreadable).
    let (cmdline, cwd) = juggler_supervisor::live_details(pid).map_or_else(
        || (row.cmdline_trunc.clone(), row.cwd.clone()),
        |(cmd, cwd)| {
            (
                if cmd.is_empty() {
                    row.cmdline_trunc.clone()
                } else {
                    cmd
                },
                if cwd.is_empty() { row.cwd.clone() } else { cwd },
            )
        },
    );
    let ports = os_listeners()
        .map(|listeners| {
            listeners
                .iter()
                .filter(|listener| listener.pid == pid)
                .map(|listener| listener.port)
                .collect::<Vec<u16>>()
        })
        .unwrap_or_default();
    let pgid = juggler_supervisor::group::pgid_of(pid).ok();
    let (members, tree_pids) = if verified {
        kill_collateral(&rows, pgid)
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(KillVerifyInfo {
        machine_id: row.machine_id.clone(),
        pid,
        session: session_id.to_string(),
        verified,
        refusal: (!verified).then(|| KILL_REFUSAL_COPY.to_string()),
        exe_path: row.exe_path.clone(),
        cmdline: truncate_cmdline(&cmdline),
        cwd,
        ports,
        pgid,
        escaped_group: row.escaped_group,
        members,
        tree_pids,
        platform: juggler_supervisor::ladder_platform().to_string(),
        grace_secs: KILL_GRACE_SECS,
        provider: row.provider.clone(),
        account_id: row.account_id.clone(),
        project_id: row.project_id.clone(),
    })
}

/// The PGID collateral list + the whole-tree enumeration (the verify
/// sheet's scope data). Ledger rows give attribution; strangers are
/// named as strangers (Req.5's shared-group warning data).
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
fn kill_collateral(
    rows: &[juggler_core::storage::LedgerRow],
    pgid: Option<u32>,
) -> (Vec<KillMemberInfo>, Vec<u32>) {
    let ledger_by_pid: std::collections::HashMap<u32, &juggler_core::storage::LedgerRow> = rows
        .iter()
        .filter_map(|row| row.pid.map(|pid| (pid, row)))
        .collect();
    let mut members = Vec::new();
    if let Some(pgid) = pgid {
        for member_pid in juggler_supervisor::pgid_members(pgid) {
            let ledger_row = ledger_by_pid.get(&member_pid).copied();
            let (member_exe, member_cmd) = ledger_row
                .map(|row| (row.exe_path.clone(), row.cmdline_trunc.clone()))
                .or_else(|| {
                    juggler_supervisor::live_details(member_pid)
                        .map(|(cmd, _)| ("(unattributed)".into(), cmd))
                })
                .unwrap_or_else(|| ("(unattributed)".into(), String::new()));
            let role = guess_role(&member_exe, &member_cmd);
            members.push(KillMemberInfo {
                pid: member_pid,
                pgid: juggler_supervisor::group::pgid_of(member_pid).ok(),
                exe: member_exe.rsplit('/').next().unwrap_or("?").to_string(),
                cmdline: truncate_cmdline(&member_cmd),
                role_guess: role,
                unsaved_state_warning: matches!(role, "McpServer" | "TerminalShell")
                    .then(|| unsaved_state_copy(role)),
                from_ledger: ledger_row.is_some(),
            });
        }
    }
    // Scope 3's enumeration: the Instance tree's live pids.
    let tree_pids = rows
        .iter()
        .filter(|row| row.pid.is_some() && (row.state == "running" || row.state == "orphaned"))
        .filter_map(|row| row.pid)
        .collect();
    (members, tree_pids)
}

/// Display truncation for command lines (200 chars, the ledger's own
/// convention).
fn truncate_cmdline(cmdline: &str) -> String {
    if cmdline.chars().count() > 200 {
        let truncated: String = cmdline.chars().take(197).collect();
        format!("{truncated}...")
    } else {
        cmdline.to_string()
    }
}

/// One still-held port after the ladder (the honest post-kill report,
/// FMEA C2 — never a success claim).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct StillHoldingInfo {
    /// The port still held.
    pub port: u16,
    /// The pid holding it NOW (may differ from the killed pid).
    pub pid: u32,
    /// The holder's exe (basename).
    pub exe: String,
}

/// The ladder's per-process report (Req.6's vocabulary, one label per
/// member).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct KillMemberOutcome {
    /// OS process id.
    pub pid: u32,
    /// `exited on TERM` | `needed KILL` | `already gone` | `refused` |
    /// `TERM'd — outcome unknown`.
    pub outcome: String,
}

/// The kill outcome (step 3's report): per-process outcomes, the
/// honest still-holding list, and — when an escapee holds the port —
/// the pid-only re-offer data.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct KillExecuteInfo {
    /// The exact refusal copy when the same-tick verification failed
    /// (zero signals were sent).
    pub refusal: Option<String>,
    /// Per-process outcomes.
    pub members: Vec<KillMemberOutcome>,
    /// True when the user cancelled mid-grace (audit left pending).
    pub cancelled: bool,
    /// True when every member succeeded (`already gone` included).
    pub all_succeeded: bool,
    /// Ports still held after the ladder (by anyone).
    pub still_holding: Vec<StillHoldingInfo>,
    /// The pid-only re-offer when an ESCAPED member still holds a
    /// killed target's port (fresh verification is the UI's next
    /// step, never an automatic signal).
    pub reoffer_pid: Option<u32>,
}

/// The kill sheet's step 3: the audited ladder. Guardrails, in
/// order: (1) same-tick re-verification of EVERY signalled pid's
/// anchor — a reused PID gets ZERO signals and the exact refusal
/// copy; (2) the AJ-E509 audit intent BEFORE the first signal (no
/// audit, no kill); (3) the ladder — TERM, cancellable
/// grace, KILL only survivors; (4) the honest post-ladder port
/// re-probe (still-held is a fact, never a failure to hide).
///
/// # Errors
///
/// [`AppError::NotFound`] when no ledger row exists; validation when
/// `scope` is not one of `pid`|`pgid`|`tree`;
/// [`AppError::AuditUnavailable`] when the audit intent fails (the
/// kill is refused, zero signals sent).
#[tauri::command]
pub fn kill_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
    scope: String,
    attempt: String,
    goal_port: Option<u16>,
) -> AppResult<KillExecuteInfo> {
    let flag = state.register_kill_cancel(&attempt);
    let result = kill_execute_audited(
        &state,
        state.audit.as_ref(),
        &session_id,
        pid,
        &scope,
        &flag,
        goal_port,
    );
    state.unregister_kill_cancel(&attempt);
    result
}

/// Cancels a running ladder's grace window (the UI's Cancel button).
/// Idempotent; unknown attempts are no-ops (the ladder already
/// finished).
#[tauri::command]
pub fn kill_cancel(state: tauri::State<'_, Arc<AppState>>, attempt: String) {
    state.flip_kill_cancel(&attempt);
}

/// The grace window's per-process exit ticks (Req.3): liveness of the
/// ladder's known members, probed cheaply (one targeted check per
/// pid — no scans). The sheet polls this while the ladder runs; the
/// AUTHORITATIVE outcomes still come from `kill_execute`'s report.
#[tauri::command]
#[must_use]
pub fn kill_liveness(pids: Vec<u32>) -> Vec<bool> {
    pids.iter()
        .map(|pid| juggler_supervisor::process_alive(*pid).is_some())
        .collect()
}

/// `kill_execute` with an injected audit writer (the guardrail
/// gate's seam; production passes the real repo).
///
/// # Errors
///
/// As [`kill_execute`].
pub fn kill_execute_audited(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    session_id: &str,
    pid: u32,
    scope: &str,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    goal_port: Option<u16>,
) -> Result<KillExecuteInfo, AppError> {
    kill_execute_as(
        state,
        audit,
        "user:kill_sheet",
        session_id,
        pid,
        scope,
        cancel_flag,
        goal_port,
    )
}

/// `kill_execute` with an explicit audit ACTOR — the automatic tiers'
/// entry (AJ-E508): actor `auto:tier-1` / `auto:tier-2` per the E509
/// vocabulary. Same guardrails, same ladder, same audit floor.
///
/// # Errors
///
/// As [`kill_execute`].
#[allow(clippy::too_many_arguments)] // the audit actor rides the E507 kill path's shape
pub fn kill_execute_as(
    state: &Arc<AppState>,
    audit: &dyn AuditWriter,
    actor: &str,
    session_id: &str,
    pid: u32,
    scope: &str,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    goal_port: Option<u16>,
) -> Result<KillExecuteInfo, AppError> {
    use juggler_supervisor::tree::LedgerStore as _;
    let session = SessionId::new(session_id.to_string());
    let rows = state.ledger.session_rows(&session).unwrap_or_default();
    let row = rows
        .iter()
        .find(|row| row.pid == Some(pid))
        .ok_or_else(|| AppError::NotFound {
            entity: "ledger row",
            id: format!("{session_id}/{pid}"),
        })?;
    // Guardrail 0 — the machine gate (AJ-E511 Req.2): a foreign row
    // gets the TYPED WrongMachine refusal with zero signals, before
    // any identity probe (which would misleadingly read as a
    // PID-reuse refusal).
    if row.machine_id != local_machine_id(state).as_str() {
        return Ok(KillExecuteInfo {
            refusal: Some(KILL_WRONG_MACHINE_COPY.to_string()),
            members: vec![KillMemberOutcome {
                pid,
                outcome: "refused".into(),
            }],
            cancelled: false,
            all_succeeded: false,
            still_holding: Vec::new(),
            reoffer_pid: None,
        });
    }
    // Guardrail 1 — SAME-TICK re-verification (E503 against the
    // kernel NOW, not the sheet's earlier verdict; the SAME shared
    // rule the sheet used): a mismatched or unverifiable anchor gets
    // ZERO signals and the exact copy.
    let verified = verify_kill_target(state, &session, row);
    if !verified {
        return Ok(KillExecuteInfo {
            refusal: Some(KILL_REFUSAL_COPY.to_string()),
            members: vec![KillMemberOutcome {
                pid,
                outcome: "refused".into(),
            }],
            cancelled: false,
            all_succeeded: false,
            still_holding: Vec::new(),
            reoffer_pid: None,
        });
    }
    // Scope enumeration (Req.1's three scopes; the UI defaults to
    // the narrowest that covers the goal).
    let members = scope_members(scope, &rows, pid)?;
    // The port facts: the goal port when the user stated one (the
    // port that opened the sheet — it may be held by an ESCAPED
    // sibling, which is exactly the honest-still-holding case),
    // else whatever the target itself holds.
    let member_ports = ports_held_by(pid);
    let ports_before = match goal_port {
        Some(port) => Some(port.to_string()),
        None => member_ports.clone(),
    };
    // Guardrail 2 — NO AUDIT, NO KILL: the intent row names the real
    // target (scope, pgid, attribution), written BEFORE any signal.
    let record = kill_intent_record(
        state,
        row,
        session_id,
        pid,
        scope,
        ports_before.as_deref(),
        member_ports.as_deref(),
        actor,
    );
    let intent_id = audit
        .intent(&record)
        .map_err(|_| AppError::AuditUnavailable)?;
    // Guardrail 3 — the ladder itself (cancellable through grace).
    let cancel = std::sync::Arc::clone(cancel_flag);
    let outcome = juggler_supervisor::terminate_group(
        // AJ-E512: settings-backed (default the spec's 5 s) so the
        // collateral suites can widen it on loaded machines — a starved
        // fixture's TERM handler must land inside the grace window.
        std::time::Duration::from_secs(setting_u64(state, "kill_grace_secs", KILL_GRACE_SECS)),
        move || members,
        move || cancel.load(std::sync::atomic::Ordering::Relaxed),
    );
    // The honest post-ladder port re-probe (C2): report what is
    // STILL held and by whom — including an escaped member that keeps
    // the port (then re-offer pid-only after fresh verification).
    let (still_holding, reoffer_pid) =
        still_holding_report(ports_before.as_deref(), pid, outcome.cancelled);
    // Audit completion: per-pid vocabulary joined; a CANCELLED
    // ladder leaves the intent pending ("TERM'd, outcome unknown").
    if !outcome.cancelled {
        let vocabulary = outcome
            .members
            .iter()
            .map(|(pid, outcome)| format!("pid {pid}: {}", outcome.as_str()))
            .collect::<Vec<_>>()
            .join("; ");
        let ports_after = still_holding
            .iter()
            .map(|held| held.port.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let ports_after = (!ports_after.is_empty()).then_some(ports_after);
        // Best-effort (E509's floor is the intent row itself).
        let _ = audit.complete(intent_id, &vocabulary, ports_after.as_deref());
    }
    let members = outcome
        .members
        .iter()
        .map(|(pid, label)| KillMemberOutcome {
            pid: *pid,
            outcome: label.as_str().to_string(),
        })
        .collect();
    Ok(KillExecuteInfo {
        refusal: None,
        members,
        cancelled: outcome.cancelled,
        all_succeeded: outcome.all_succeeded(),
        still_holding,
        reoffer_pid,
    })
}

/// The kill sheet's audit intent row (Req.6 / AJ-E509 Req.2): the
/// full attribution from the ledger row, the verified-at-click-time
/// identity note, and the goal-port-aware before/after facts.
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
#[allow(clippy::too_many_arguments)] // mirrors the audit record's own columns
fn kill_intent_record(
    state: &Arc<AppState>,
    row: &juggler_core::storage::LedgerRow,
    session_id: &str,
    pid: u32,
    scope: &str,
    ports_before: Option<&str>,
    member_ports: Option<&str>,
    actor: &str,
) -> juggler_core::storage::CleanupAuditRecord {
    juggler_core::storage::CleanupAuditRecord {
        id: 0,
        at_ms: now_millis(),
        actor: actor.to_string(),
        pid,
        pgid: juggler_supervisor::group::pgid_of(pid).ok(),
        exe_path: row.exe_path.clone(),
        cmdline_hash: row.cmdline_hash.clone(),
        cmdline_trunc: truncate_cmdline(&row.cmdline_trunc),
        session_id: session_id.to_string(),
        provider: row.provider.clone(),
        account_id: row.account_id.clone(),
        project_id: row.project_id.clone(),
        machine_id: local_machine_id(state).as_str().to_string(),
        identity: format!(
            "E503 verified at click time (start {}, exe {:?})",
            row.started_at.map_or("?".to_string(), |t| t.to_string()),
            row.exe_path
        ),
        scope: scope.to_string(),
        outcome: String::new(),
        ports_before: ports_before
            .map(str::to_string)
            .or_else(|| member_ports.map(str::to_string)),
        ports_after: None,
    }
}

/// The three scopes' member enumeration (Req.1): `pid` — the one
/// verified process; `pgid` — every live member of its process group;
/// `tree` — the Instance's live ledger rows.
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown scope, or for `pgid` when
/// the group is unreadable (the honest narrower-scope prompt).
#[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
fn scope_members(
    scope: &str,
    rows: &[juggler_core::storage::LedgerRow],
    pid: u32,
) -> Result<Vec<u32>, AppError> {
    match scope {
        "pid" => Ok(vec![pid]),
        "pgid" => {
            let pgid =
                juggler_supervisor::group::pgid_of(pid).map_err(|_| AppError::Validation {
                    subject: "scope",
                    message: "the process group is unreadable — choose `This process only`".into(),
                })?;
            Ok(juggler_supervisor::pgid_members(pgid))
        }
        "tree" => Ok(rows
            .iter()
            .filter(|row| row.pid.is_some() && (row.state == "running" || row.state == "orphaned"))
            .filter_map(|row| row.pid)
            .collect()),
        other => Err(AppError::Validation {
            subject: "scope",
            message: format!("unknown scope {other:?} (pid | pgid | tree)"),
        }),
    }
}

/// The post-ladder port re-probe (FMEA C2): who STILL holds each of
/// the target's ports, and the pid-only re-offer when an escapee —
/// not the killed pid — keeps one. Never a success claim.
fn still_holding_report(
    ports_before: Option<&str>,
    pid: u32,
    cancelled: bool,
) -> (Vec<StillHoldingInfo>, Option<u32>) {
    let target_ports: Vec<u16> = ports_before
        .unwrap_or("")
        .split(',')
        .filter_map(|port| port.parse::<u16>().ok())
        .collect();
    let mut still_holding = Vec::new();
    if let Ok(listeners) = os_listeners() {
        for port in &target_ports {
            for listener in listeners.iter().filter(|l| l.port == *port) {
                let exe = holder_exe(listener.pid);
                still_holding.push(StillHoldingInfo {
                    port: *port,
                    pid: listener.pid,
                    exe,
                });
            }
        }
    }
    // The re-offer only after a COMPLETED ladder (a cancelled one
    // reports the facts and leaves the choice entirely to the user).
    let reoffer_pid = if cancelled {
        None
    } else {
        still_holding
            .iter()
            .find(|held| held.pid != pid)
            .map(|held| held.pid)
    };
    (still_holding, reoffer_pid)
}

/// `Verify owner`: re-reads the ppid chain for one orphan pid (manual,
/// honest evidence — never a silent re-check).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
#[tauri::command]
pub fn orphan_verify(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
) -> AppResult<OrphanVerifyInfo> {
    orphan_verify_arc(&state, &session_id, pid)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on probe failure.
pub fn orphan_verify_arc(
    _state: &Arc<AppState>,
    _session_id: &str,
    pid: u32,
) -> AppResult<OrphanVerifyInfo> {
    // The probe itself is per-pid (the session scopes it in the UI).
    match juggler_supervisor::verify_owner(pid) {
        juggler_supervisor::OwnerState::Gone => Ok(OrphanVerifyInfo {
            pid,
            alive: false,
            parent_pid: None,
            note: "process is gone".to_string(),
        }),
        juggler_supervisor::OwnerState::Alive { ppid } => Ok(OrphanVerifyInfo {
            pid,
            alive: true,
            parent_pid: ppid,
            note: owner_note(ppid),
        }),
    }
}

/// Human note for a live pid's parent state.
fn owner_note(ppid: Option<u32>) -> String {
    match ppid {
        Some(1 | 0) => "alive — re-parented to init/launchd (its parent exited)".to_string(),
        Some(parent) => format!("alive — parent pid {parent}"),
        None => "alive — parent not readable".to_string(),
    }
}

/// Sets the per-Account Tier-1 cleanup opt-in (D-08: off by default;
/// the toggle is the only way automatic kills ever happen).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown.
#[tauri::command]
pub fn account_set_cleanup(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    account_set_cleanup_arc(&state, &account_id, opt_in)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown.
pub fn account_set_cleanup_arc(
    state: &Arc<AppState>,
    account_id: &str,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    let account = {
        let storage = lock_storage_arc(state)?;
        let mut account = storage
            .accounts()
            .get(&AccountId::new(account_id.to_string()))?;
        account.cleanup_opt_in = opt_in;
        storage.accounts().update(&account)?;
        account
    };
    Ok(account_info_of(&account))
}

/// Sets or clears the optional user-entered renewal date (AJ-E411,
/// D-12). `date` must be a valid local calendar `YYYY-MM-DD` (strictly
/// validated — honest rejection, never a repair); `None` clears it.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown
/// or the date format is invalid.
#[tauri::command]
pub fn account_set_renewal(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    date: Option<String>,
) -> AppResult<AccountInfo> {
    account_set_renewal_arc(&state, &account_id, date.as_deref())
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown
/// or the date format is invalid.
pub fn account_set_renewal_arc(
    state: &Arc<AppState>,
    account_id: &str,
    date: Option<&str>,
) -> AppResult<AccountInfo> {
    if let Some(raw) = date
        && juggler_core::renewal::CalendarDate::parse(raw).is_none()
    {
        return Err(AppError::Validation {
            subject: "renewal date",
            message: format!("must be YYYY-MM-DD (got {raw:?})"),
        });
    }
    let account = {
        let storage = lock_storage_arc(state)?;
        let mut account = storage
            .accounts()
            .get(&AccountId::new(account_id.to_string()))?;
        account.renewal_date = date.map(ToString::to_string);
        storage.accounts().update(&account)?;
        account
    };
    Ok(account_info_of(&account))
}

/// The exit-snapshot hook (AJ-E410 Req.3): freezes the pre-exit
/// descendant rows into the tracker and journals the durable
/// allow-list. Called by the lifecycle forwarder on every Exited event
/// (clean exits are recorded too — they simply never surface orphans).
pub fn record_exit_snapshot(
    state: &Arc<AppState>,
    session: &SessionId,
    root_pid: u32,
    kind: &ExitKind,
) {
    let (rows, root_program) = snapshot_descendants(state, session, root_pid);
    if let Some(journal) = state.supervisor.journal_ref() {
        let _ = journal.record_descendants(session, root_pid, kind.is_success(), &rows);
    }
    state.orphans.record_exit(juggler_supervisor::ExitSnapshot {
        session: session.clone(),
        root_pid,
        root_program,
        clean: kind.is_success(),
        taken_ms: Timestamp::now().millis(),
        rows,
    });
}

/// The frozen pre-exit rows, split into (descendants, root program name).
/// The root is dead already — its name survives only in the collector's
/// last tick while it was alive, which is exactly what this reads.
fn snapshot_descendants(
    state: &Arc<AppState>,
    session: &SessionId,
    root_pid: u32,
) -> (Vec<juggler_supervisor::DescendantRow>, String) {
    let Some(collector) = state.resources.lock().ok() else {
        return (Vec::new(), "the ended instance".to_string());
    };
    let frozen = collector.rows(session).unwrap_or_default();
    let root_program = frozen
        .iter()
        .find(|row| row.pid == root_pid)
        .map_or_else(|| "the ended instance".to_string(), |row| row.name.clone());
    let rows = frozen
        .into_iter()
        .filter(|row| row.pid != root_pid)
        .map(|row| juggler_supervisor::DescendantRow {
            pid: row.pid,
            name: row.name,
            rss_bytes: row.rss_bytes,
        })
        .collect();
    (rows, root_program)
}

/// The 2 s sampling tick (AJ-E410 Req.1): collector tick (targeted
/// refresh + periodic discovery), orphan grace poll with fresh liveness
/// probes, then the Tier-1 auto-cleanup sweep for opted-in Accounts
/// (60 s grace, recorded-descendant PIDs only — D-08).
pub fn processes_tick(state: &Arc<AppState>) {
    let now = Timestamp::now().millis();
    {
        let Ok(mut collector) = state.resources.lock() else {
            return;
        };
        collector.tick(&state.supervisor, now);
        // The E410 tracker's own confirmations advance here (the
        // Processes tab's orphan section); the AJ-E508 sweep reads
        // the LEDGER's orphan verdicts instead — the tracker
        // deliberately drops clean exits and cannot serve Tier 1.
        let tracked = state.orphans.tracked_pids();
        if !tracked.is_empty() {
            collector.refresh_pids(&tracked);
            let _ = state.orphans.poll(now, |pid| collector.lookup(pid));
        }
    }
    auto_cleanup_sweep(state, now);
}

/// One live Tier-2 notice (AJ-E508 Req.1): the crash-orphan's Undo
/// window. Exactly one per candidate; `keep` (Undo / Keep running)
/// dismisses it forever for that pid.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Tier2Notice {
    /// The candidate pid.
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
    /// Truncated command line (the notification names the process).
    pub cmdline_trunc: String,
    /// When the notice was shown, epoch ms.
    pub shown_at_ms: i64,
    /// The window's end, epoch ms (≥30 s after `shown_at_ms`; no
    /// signal before it).
    pub deadline_ms: i64,
    /// True once the user chose Undo / Keep running.
    pub keep: bool,
}

/// The Undo window's length (Req.1: ≥30 s — never shorter).
pub const TIER2_UNDO_WINDOW_MS: i64 = 30_000;
/// Tier 2's crash grace before the notification (Req.1: 5 min).
pub const TIER2_CRASH_GRACE_MS: i64 = 5 * 60_000;

/// The automatic tiers' sweep (AJ-E508; replaces the E410 prototype).
/// Tier 0 by default: without BOTH the Account opt-in and the tier
/// evaluator's eligibility, nothing is ever signalled. The candidate
/// source is the LEDGER's orphaned rows (the E504 reconciler's
/// verdict — the same authority the Cleanup Centre lists; the E410
/// tracker deliberately drops clean exits and cannot serve Tier 1).
/// Gates, in order: the global pause; the 24 h crash suspension; the
/// per-candidate evaluator (roles/roots/containment/machine/exit
/// kind); pacing (the auto grace; Tier 2's 5 min + ONE notification +
/// the Undo window); same-tick re-verification (a failure DEMOTES to
/// Tier 0 with an audit note); then the E507 ladder behind an E509
/// audit intent (actor `auto:tier-N`), via [`kill_execute_as`] — the
/// only kill surface, manual or automatic.
#[allow(clippy::too_many_lines)] // one pass per gate; splitting hides the ordering that IS the law
pub fn auto_cleanup_sweep(state: &Arc<AppState>, now: i64) {
    if cleanup_paused(state) || cleanup_suspended_until(state).is_some() {
        return;
    }
    let machine = local_machine_id(state);
    // AJ-E512 Req.3: a hog is NEVER auto-signalled at any tier —
    // Tier 3 is alert + staged sheet only (E508), so the sweep stands
    // down for hog-flagged pids (and hog-flagged trees) entirely; the
    // manual E507 sheet stays the only path.
    // Hog facts ride the REAL clock (`now` here is the sweep's
    // SCHEDULE clock — often minutes in the future in tests, which
    // would empty the 60 s ring window and un-hog everything).
    let hog_facts_now = hog_facts(state, now_millis());
    let hog_pids: std::collections::HashSet<u32> = hog_facts_now
        .iter()
        .filter(|hog| hog.pid != 0)
        .map(|hog| hog.pid)
        .collect();
    let hog_sessions: std::collections::HashSet<String> = hog_facts_now
        .iter()
        .map(|hog| hog.session.clone())
        .collect();
    // One candidates read: every ORPHANED row of this machine's
    // sessions, grouped by session with its root row (the exit-kind
    // source and the pacing clock).
    let mut by_session: std::collections::HashMap<
        String,
        (
            Vec<juggler_core::storage::LedgerRow>,
            Option<juggler_core::storage::LedgerRow>,
        ),
    > = std::collections::HashMap::new();
    for row in local_ledger_rows(state) {
        if row.machine_id != machine.as_str() {
            continue;
        }
        let entry = by_session.entry(row.session_id.clone()).or_default();
        if row.role == "root" {
            entry.1 = Some(row);
        } else if row.state == "orphaned" {
            entry.0.push(row);
        }
    }
    for (session_id, (candidates, root)) in by_session {
        let Some(root_row) = root else { continue };
        // The root's last tick while alive ≈ its exit time (its
        // last_seen stops refreshing at death).
        let parent_exit_ms = root_row.last_seen;
        if now - parent_exit_ms < juggler_supervisor::AUTO_CLEANUP_GRACE_MS {
            continue;
        }
        // The root's exit kind decides the TIER, never the
        // candidate's presence: code 0 → Tier 1's premise; anything
        // else → crash (Tier 2's); unknown → observe only (fail
        // closed).
        let root_exit_clean = root_row.exit_kind.as_deref().map(|kind| {
            match kind.strip_prefix("code:") {
                Some(code) => code == "0",
                // A signal death or an unparsable kind is a crash.
                None => false,
            }
        });
        let settings = account_tier_settings(state, &SessionId::new(session_id.clone()));
        for row in candidates {
            let Some(orphan_pid) = row.pid else { continue };
            if hog_pids.contains(&orphan_pid) || hog_sessions.contains(&session_id) {
                continue; // hog: alert + staged sheet only, never a signal
            }
            let candidate = juggler_supervisor::tree::TierCandidate {
                pid: orphan_pid,
                role_guess: guess_role(&row.exe_path, &row.cmdline_trunc).to_string(),
                is_root: false,
                state: row.state.clone(),
                escaped_group: row.escaped_group,
                root_exit_clean,
                machine_local: true,
                tier1_opt_in: settings.tier1_opt_in,
                tier2_opt_in: settings.tier2_opt_in,
            };
            let tier = match juggler_supervisor::tree::evaluate_tier(&candidate) {
                juggler_supervisor::tree::TierVerdict::Signal(tier) => tier,
                juggler_supervisor::tree::TierVerdict::ObserveOnly(_) => continue,
            };
            // Tier 2 pacing: 5 min after the crash AND one
            // notification AND an elapsed Undo window.
            if tier == 2 {
                if now - parent_exit_ms < TIER2_CRASH_GRACE_MS {
                    continue;
                }
                let orphan = juggler_supervisor::Orphan {
                    session: SessionId::new(session_id.clone()),
                    root_pid: root_row.pid.unwrap_or(0),
                    parent_label: root_row.exe_path.clone(),
                    pid: orphan_pid,
                    name: row.exe_path.clone(),
                    rss_bytes: 0,
                    parent_exit_ms,
                };
                match tier2_window(state, &orphan, &row, now) {
                    Tier2Window::Open | Tier2Window::Kept => continue,
                    Tier2Window::Elapsed => (),
                }
            }
            // Same-tick re-verification — a failure DEMOTES to Tier 0
            // (list-only) with an audit note, never a signal.
            let session = SessionId::new(session_id.clone());
            if !verify_kill_target(state, &session, &row) {
                demote_with_audit_note(state, &row, tier);
                continue;
            }
            let actor = if tier == 1 {
                juggler_core::storage::ACTOR_AUTO_TIER1
            } else {
                juggler_core::storage::ACTOR_AUTO_TIER2
            };
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let outcome = kill_execute_as(
                state,
                state.audit.as_ref(),
                actor,
                session_id.as_str(),
                orphan_pid,
                "pid",
                &flag,
                None,
            );
            if let Ok(outcome) = outcome
                && let Some(journal) = state.supervisor.journal_ref()
            {
                let _ = journal.record_killed(
                    &session,
                    orphan_pid,
                    if tier == 1 {
                        "auto-tier1"
                    } else {
                        "auto-tier2"
                    },
                    &outcome
                        .members
                        .iter()
                        .map(|member| member.outcome.as_str())
                        .collect::<Vec<_>>()
                        .join("; "),
                );
            }
        }
    }
}

enum Tier2Window {
    /// Created now or still counting — no signal this tick.
    Open,
    /// The user chose Undo / Keep running — never signalled.
    Kept,
    /// The window elapsed without an Undo — eligible to signal.
    Elapsed,
}

fn tier2_window(
    state: &Arc<AppState>,
    orphan: &juggler_supervisor::Orphan,
    row: &juggler_core::storage::LedgerRow,
    now: i64,
) -> Tier2Window {
    let Ok(mut pending) = state.tier2_pending.lock() else {
        return Tier2Window::Open; // fail closed: no notice, no signal
    };
    match pending.get(&orphan.pid) {
        Some(notice) if notice.keep => Tier2Window::Kept,
        Some(notice) => {
            if now < notice.deadline_ms {
                Tier2Window::Open
            } else {
                pending.remove(&orphan.pid);
                Tier2Window::Elapsed
            }
        }
        None => {
            // Exactly one notification per crash-orphan (Req.1).
            pending.insert(
                orphan.pid,
                Tier2Notice {
                    pid: orphan.pid,
                    session: orphan.session.as_str().to_string(),
                    cmdline_trunc: truncate_cmdline(&row.cmdline_trunc),
                    shown_at_ms: now,
                    deadline_ms: now + TIER2_UNDO_WINDOW_MS,
                    keep: false,
                },
            );
            Tier2Window::Open
        }
    }
}

/// A failed same-tick re-verification: the candidate is silently
/// demoted to Tier 0 (list-only) WITH an audit note — the E509 log
/// carries intent + `demoted` completion so the History tells the
/// truth about what was considered and why nothing was signalled.
fn demote_with_audit_note(state: &Arc<AppState>, row: &juggler_core::storage::LedgerRow, tier: u8) {
    let actor = if tier == 1 {
        juggler_core::storage::ACTOR_AUTO_TIER1
    } else {
        juggler_core::storage::ACTOR_AUTO_TIER2
    };
    let record = kill_intent_record(
        state,
        row,
        &row.session_id,
        row.pid.unwrap_or(0),
        "pid",
        None,
        None,
        actor,
    );
    if let Ok(id) = state.audit.intent(&record) {
        let _ = state.audit.complete(
            id,
            "demoted — re-verification failed, listed only (Tier 0)",
            None,
            now_millis(),
        );
    }
}

/// True when the global `Pause automatic cleanup` kill-switch is on
/// (Req.4): the sweep stands down within one tick of the flag.
fn cleanup_paused(state: &Arc<AppState>) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return true; // a lock failure pauses — never signals
    };
    storage
        .app_settings()
        .get("cleanup_paused")
        .ok()
        .flatten()
        .is_some_and(|value| value == "1")
}

/// The 24 h crash-suspension deadline (Req.5), if active: Tiers 1-2
/// stand down until the user re-confirms in the Cleanup Centre.
fn cleanup_suspended_until(state: &Arc<AppState>) -> Option<i64> {
    let storage = state.storage.lock().ok()?;
    let until = storage
        .app_settings()
        .get("cleanup_suspended_until")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())?;
    (until > now_millis()).then_some(until)
}

/// Resolves an Instance's Account tier settings (session → launch
/// settings → account). Unknown links read as both-off (fail closed).
fn account_tier_settings(
    state: &Arc<AppState>,
    session: &SessionId,
) -> juggler_core::storage::CleanupTierSettings {
    let account = (|| {
        let storage = state.storage.lock().ok()?;
        let record = storage.sessions().get(session).ok()?;
        let recipe = storage.profiles().get(&record.profile).ok()?;
        storage.accounts().get(&recipe.account).ok()
    })();
    let account_id = account.as_ref().map(|a| a.id.as_str().to_string());
    match account_id {
        Some(account_id) => state.cleanup_settings.get(&account_id).unwrap_or_default(),
        None => juggler_core::storage::CleanupTierSettings::default(),
    }
}

/// One live Tier-2 notice as the UI reads it.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct Tier2NoticeInfo {
    /// The candidate pid.
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
    /// Truncated command line.
    pub cmdline_trunc: String,
    /// The window's end, epoch ms.
    pub deadline_ms: i64,
}

/// Reads a u64 app setting with a default (AJ-E512 test seams for
/// the hog/idle thresholds; production defaults are the arguments).
fn setting_u64(state: &Arc<AppState>, key: &str, default: u64) -> u64 {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

/// Reads an f64 app setting with a default (same seam as
/// [`setting_u64`]).
fn setting_f64(state: &Arc<AppState>, key: &str, default: f64) -> f64 {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

/// One passive Cleanup Centre alert (AJ-E512 Req.3). `role=status`
/// surfaces in the UI — never a focus-stealing dialog.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupAlertInfo {
    /// `hog` (resource alert, staged-sheet eligible) or `idle` (the
    /// once-per-orphan nudge).
    pub kind: String,
    /// The exact copy (byte-asserted in tests; one language with the
    /// Rust suite).
    pub text: String,
    /// The helper pid the alert names (0 when unresolvable).
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
}

/// The passive alert set: hog alerts (exact template) + idle-orphan
/// nudges (fired at most ONCE per orphan — the once-flag is persisted
/// the moment the nudge is computed).
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default)]
pub struct CleanupAlertsInfo {
    /// Hog alerts first (actionable), then idle nudges.
    pub alerts: Vec<CleanupAlertInfo>,
}

/// Formats bytes the way the alert copy needs (GiB one decimal, MiB
/// whole).
#[must_use]
pub fn rss_text(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)] // display rounding, not math
    let gib = bytes as f64 / 1_073_741_824.0;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{gib:.1} GiB")
    } else {
        format!("{} MiB", bytes / (1024 * 1024))
    }
}

/// Formats an age the way the nudge copy needs.
fn age_text(ms: i64) -> String {
    let mins = ms / 60_000;
    if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h", mins / 60)
    }
}

/// Computes the passive alerts (AJ-E512 Req.3): for every hog-flagged
/// tree, ONE alert naming its TOP-RSS helper (NN/g: name the helper,
/// not just the Instance), and — once per orphan — an idle nudge
/// (CPU≈0 + age only). Purely passive: no alert carries a signal
/// path; the hog alert's pid routes to the STAGED E507 sheet
/// (app prepares, user executes).
pub fn cleanup_alerts_inner(state: &Arc<AppState>) -> CleanupAlertsInfo {
    let now = now_millis();
    let machine = local_machine_id(state);
    let mut alerts = Vec::new();

    // Hog alerts: top-RSS live helper per hog-flagged session.
    let facts = hog_facts(state, now);
    let mut hog_sessions: Vec<String> = facts.iter().map(|f| f.session.clone()).collect();
    hog_sessions.sort();
    hog_sessions.dedup();
    if !hog_sessions.is_empty() {
        let Some(collector) = state.resources.lock().ok() else {
            return CleanupAlertsInfo { alerts };
        };
        for session in hog_sessions {
            let top = collector
                .rows(&SessionId::new(session.clone()))
                .into_iter()
                .flatten()
                .skip(1) // the root
                .max_by_key(|row| row.rss_bytes);
            let Some(top) = top else { continue };
            let helper = top.name.rsplit('/').next().unwrap_or(&top.name).to_string();
            let instance = instance_label_for(state, &session);
            alerts.push(CleanupAlertInfo {
                kind: "hog".into(),
                text: format!(
                    "{helper} under {instance} holds {} — review in Cleanup Centre.",
                    rss_text(top.rss_bytes)
                ),
                pid: top.pid,
                session: session.clone(),
            });
        }
    }

    // Idle nudges: orphaned helpers, CPU≈0, aged past the threshold —
    // fired at most ONCE per orphan (persisted flag).
    let min_age = setting_u64(state, "idle_nudge_min_age_ms", 3_600_000).cast_signed();
    for row in local_ledger_rows(state) {
        if row.state != "orphaned" || row.machine_id != machine.as_str() || row.role != "helper" {
            continue;
        }
        let Some(pid) = row.pid else { continue };
        let cpu_ok = state
            .resources
            .lock()
            .ok()
            .and_then(|collector| {
                collector
                    .rows(&SessionId::new(row.session_id.clone()))
                    .into_iter()
                    .flatten()
                    .find(|live| live.pid == pid)
                    .map(|live| live.cpu_pct.is_none_or(|cpu| cpu < 1.0))
            })
            .unwrap_or(true);
        if !cpu_ok {
            continue;
        }
        let age = now - row.started_at.map(|s| s * 1000).unwrap_or_default();
        if age < min_age {
            continue;
        }
        let flag = format!("idle_nudge:{}:{pid}", row.session_id);
        let already = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(&flag).ok().flatten())
            .is_some();
        if already {
            continue;
        }
        if let Ok(storage) = state.storage.lock() {
            let _ = storage.app_settings().set(&flag, "1");
        }
        let helper = row
            .exe_path
            .rsplit('/')
            .next()
            .unwrap_or(&row.exe_path)
            .to_string();
        alerts.push(CleanupAlertInfo {
            kind: "idle".into(),
            text: format!(
                "{helper} has been idle for {} — review it in the Cleanup Centre.",
                age_text(age)
            ),
            pid,
            session: row.session_id.clone(),
        });
    }
    CleanupAlertsInfo { alerts }
}

/// The Instance's display label (account label → provider → session).
fn instance_label_for(state: &Arc<AppState>, session: &str) -> String {
    let lookup = (|| {
        let storage = state.storage.lock().ok()?;
        let record = storage
            .sessions()
            .get(&SessionId::new(session.to_string()))
            .ok()?;
        let recipe = storage.profiles().get(&record.profile).ok()?;
        let account = storage.accounts().get(&recipe.account).ok()?;
        Some(account.label.clone())
    })();
    lookup.unwrap_or_else(|| "its Instance".to_string())
}

/// Reads the passive Cleanup Centre alerts (AJ-E512 Req.3).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
#[tauri::command]
pub fn cleanup_alerts(state: tauri::State<'_, Arc<AppState>>) -> AppResult<CleanupAlertsInfo> {
    Ok(cleanup_alerts_inner(&state))
}

/// The cleanup automation's status (the centre's banners + the
/// Settings surface read THIS — one truth).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupStatusInfo {
    /// True while `Pause automatic cleanup` is on.
    pub paused: bool,
    /// True while the 24 h crash suspension is active.
    pub suspended: bool,
    /// The suspension's deadline, epoch ms (when `suspended`).
    pub suspended_until_ms: Option<i64>,
    /// The live Tier-2 notices (open Undo windows).
    pub tier2_pending: Vec<Tier2NoticeInfo>,
}

/// The one-time recovery summary (AJ-E510 Req.3): present exactly
/// once per crash — the second relaunch (no new findings) re-shows
/// nothing.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct RecoverySummaryInfo {
    /// The exact template text.
    pub summary: String,
    /// `{a}` — Instances re-adopted (stoppable through the E507 path).
    pub adopted: usize,
    /// `{b}` — helpers left behind while Juggler was closed.
    pub left_behind: usize,
    /// `{c}` — processes that could not be verified (left alone).
    pub unverifiable: usize,
}

/// Reads the unconsumed recovery summary, if one is pending (shown
/// once, then dismissed forever until a new recovery finds something).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
#[tauri::command]
pub fn recovery_summary_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Option<RecoverySummaryInfo>> {
    recovery_summary_get_arc(&state)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
pub fn recovery_summary_get_arc(state: &Arc<AppState>) -> AppResult<Option<RecoverySummaryInfo>> {
    let storage = lock_storage_arc(state)?;
    let Some(summary) = storage
        .app_settings()
        .get("recovery_summary")
        .map_err(AppError::from)?
        .filter(|text| !text.is_empty())
    else {
        return Ok(None);
    };
    let consumed = storage
        .app_settings()
        .get("recovery_summary_consumed")
        .map_err(AppError::from)?
        .is_some_and(|value| value == "1");
    if consumed {
        return Ok(None);
    }
    // Counts ride the text's shape (written by the same template).
    let count = |part: Option<&str>| {
        part.and_then(|text| text.split_whitespace().next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or_default()
    };
    let mut fields = summary.split(',');
    let adopted = count(fields.next());
    let left_behind = count(fields.next());
    let unverifiable = count(fields.next());
    Ok(Some(RecoverySummaryInfo {
        summary,
        adopted,
        left_behind,
        unverifiable,
    }))
}

/// Dismisses the summary (the only way it goes away; no permanent
/// dismiss shortcut exists by design).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
#[tauri::command]
pub fn recovery_summary_dismiss(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    let storage = lock_storage_arc(&state)?;
    storage
        .app_settings()
        .set("recovery_summary_consumed", "1")
        .map_err(AppError::from)?;
    Ok(())
}

/// Reads the automation status (Tier 0 is always the fallback — this
/// surface never signals).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
#[tauri::command]
pub fn cleanup_status(state: tauri::State<'_, Arc<AppState>>) -> AppResult<CleanupStatusInfo> {
    cleanup_status_inner(&state)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
pub fn cleanup_status_inner(state: &Arc<AppState>) -> AppResult<CleanupStatusInfo> {
    Ok(cleanup_status_of(state))
}

/// The status read (shared by the command and the sweep's tests).
fn cleanup_status_of(state: &Arc<AppState>) -> CleanupStatusInfo {
    let tier2_pending = state
        .tier2_pending
        .lock()
        .map(|pending| {
            pending
                .values()
                .filter(|notice| !notice.keep)
                .map(|notice| Tier2NoticeInfo {
                    pid: notice.pid,
                    session: notice.session.clone(),
                    cmdline_trunc: notice.cmdline_trunc.clone(),
                    deadline_ms: notice.deadline_ms,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    CleanupStatusInfo {
        paused: cleanup_paused(state),
        suspended: cleanup_suspended_until(state).is_some(),
        suspended_until_ms: cleanup_suspended_until(state),
        tier2_pending,
    }
}

/// Sets the global `Pause automatic cleanup` kill-switch (Req.4):
/// takes effect within one tick (the sweep reads the flag first).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
#[tauri::command]
pub fn cleanup_pause_set(
    state: tauri::State<'_, Arc<AppState>>,
    paused: bool,
) -> AppResult<CleanupStatusInfo> {
    cleanup_pause_set_arc(&state, paused)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_pause_set_arc(state: &Arc<AppState>, paused: bool) -> AppResult<CleanupStatusInfo> {
    {
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set("cleanup_paused", if paused { "1" } else { "0" })
            .map_err(AppError::from)?;
    }
    Ok(cleanup_status_of(state))
}

/// The user's Tier-2 decision on one notice (Req.1): `keep` = Undo /
/// Keep running — the candidate is never signalled and the notice is
/// dismissed (exactly one notification per crash-orphan, by construction).
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
#[tauri::command]
pub fn cleanup_tier2_decide(
    state: tauri::State<'_, Arc<AppState>>,
    pid: u32,
    keep: bool,
) -> AppResult<CleanupStatusInfo> {
    cleanup_tier2_decide_arc(&state, pid, keep)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
pub fn cleanup_tier2_decide_arc(
    state: &Arc<AppState>,
    pid: u32,
    keep: bool,
) -> AppResult<CleanupStatusInfo> {
    if let Ok(mut pending) = state.tier2_pending.lock()
        && let Some(notice) = pending.get_mut(&pid)
    {
        notice.keep = keep;
    }
    Ok(cleanup_status_of(state))
}

/// Re-confirms automatic cleanup after the 24 h crash suspension
/// (Req.5 — the Cleanup Centre's button; shown, never silent).
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
#[tauri::command]
pub fn cleanup_resume_after_crash(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CleanupStatusInfo> {
    cleanup_resume_after_crash_arc(&state)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_resume_after_crash_arc(state: &Arc<AppState>) -> AppResult<CleanupStatusInfo> {
    {
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set("cleanup_suspended_until", "0")
            .map_err(AppError::from)?;
    }
    Ok(cleanup_status_of(state))
}

/// Reads one Account's two tier opt-ins (absent rows = both off).
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
#[tauri::command]
pub fn cleanup_tiers_get(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    cleanup_tiers_get_arc(&state, &account_id)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
pub fn cleanup_tiers_get_arc(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    Ok(state.cleanup_settings.get(account_id).unwrap_or_default())
}

/// The Account launch-settings area's two SEPARATE toggles (Req.4,
/// exact labels in the UI). Also mirrors Tier 1 onto the legacy
/// `accounts.cleanup_opt_in` column so older readers stay truthful.
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
#[tauri::command]
pub fn cleanup_tiers_set(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    tier1_opt_in: bool,
    tier2_opt_in: bool,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    cleanup_tiers_set_arc(&state, &account_id, tier1_opt_in, tier2_opt_in)
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_tiers_set_arc(
    state: &Arc<AppState>,
    account_id: &str,
    tier1_opt_in: bool,
    tier2_opt_in: bool,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    let settings = juggler_core::storage::CleanupTierSettings {
        tier1_opt_in,
        tier2_opt_in,
    };
    state
        .cleanup_settings
        .set(account_id, &settings)
        .map_err(AppError::from)?;
    // Mirror Tier 1 onto the prototype column (one honest source:
    // the tier table; the account row follows).
    account_set_cleanup_arc(state, account_id, tier1_opt_in).ok();
    Ok(settings)
}

// ---------------------------------------------------------------------------
// AJ-E412: running-model telemetry — the transcript-observed model.
// ---------------------------------------------------------------------------

/// Scans NEW transcript bytes (tracked per-Instance parse offset) for
/// the newest recorded turn model, and persists + classifies it on
/// change (`transcript` when it matches the launch overlay,
/// `in-session` when it differs — an observed `/model` change). Only
/// transcript lines are read: no Provider config files, no network
/// (Req.2/Req.7 — local-only by construction).
///
/// Returns true when any Instance's model changed (the caller emits
/// `sessions_updated`).
pub fn detect_running_models(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    only: Option<&[SessionId]>,
) -> bool {
    let changed = detect_running_models_core(state, only);
    if changed {
        let _ = emit_sessions(app, state);
    }
    changed
}

/// The storage-side core (tests call this directly — no app handle).
pub fn detect_running_models_core(state: &Arc<AppState>, only: Option<&[SessionId]>) -> bool {
    let sessions: Vec<(SessionId, Option<String>, std::path::PathBuf)> = {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        storage
            .sessions()
            .list(None)
            .unwrap_or_default()
            .into_iter()
            .filter(|record| !is_terminal(record.status))
            .filter(|record| only.is_none_or(|ids| ids.contains(&record.id)))
            .filter_map(|record| {
                let overlay = storage
                    .profiles()
                    .get(&record.profile)
                    .ok()
                    .and_then(|recipe| recipe.settings_overlay.model);
                let path = record.transcript_ref.as_ref()?.path.clone();
                Some((record.id, overlay, path))
            })
            .collect()
    };
    let mut changed = false;
    for (session, launch_model, path) in sessions {
        if scan_one_transcript(state, &session, launch_model.as_ref(), &path) {
            changed = true;
        }
    }
    changed
}

/// Tail-scans one Instance's transcript for the newest observed model;
/// true when the stored observation changed. Incremental by byte offset
/// (truncation resets to zero — re-parse, never assume).
fn scan_one_transcript(
    state: &Arc<AppState>,
    session: &SessionId,
    launch_model: Option<&String>,
    path: &std::path::Path,
) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(metadata) = std::fs::metadata(path) else {
        return false; // no transcript yet: not observed, never guessed
    };
    let len = metadata.len();
    let offset = state
        .model_offsets
        .lock()
        .ok()
        .and_then(|offsets| offsets.get(session).copied())
        .unwrap_or(0);
    if len < offset {
        // Truncated/rotated: reset and re-parse from the start.
        if let Ok(mut offsets) = state.model_offsets.lock() {
            offsets.insert(session.clone(), 0);
        }
        return scan_one_transcript(state, session, launch_model, path);
    }
    if len == offset {
        return false; // nothing new — one metadata read, no parse
    }
    // Read only the NEW bytes (plus a carry for the half-parsed line):
    // seek past the consumed prefix, never re-parse the head.
    let read_from = offset.saturating_sub(1);
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.seek(SeekFrom::Start(read_from)).is_err() {
        return false;
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len - read_from).unwrap_or(0));
    if file
        .by_ref()
        .take(len - read_from)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return false;
    }
    if let Ok(mut offsets) = state.model_offsets.lock() {
        offsets.insert(session.clone(), len);
    }
    // The newest model recorded on a complete line wins.
    let mut observed: Option<String> = None;
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(model) = line_model(&value) {
            observed = Some(model);
        }
    }
    let Some(model) = observed else {
        return false;
    };
    let source = if launch_model == Some(&model) {
        juggler_core::model::ModelSource::Transcript
    } else {
        juggler_core::model::ModelSource::InSession
    };
    let next = juggler_core::model::RunningModel { model, source };
    persist_running_model(state, session, &next)
}

/// Persists an observed running model; false when unchanged (no write,
/// no emit) or when the row is gone.
fn persist_running_model(
    state: &Arc<AppState>,
    session: &SessionId,
    next: &juggler_core::model::RunningModel,
) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    let Ok(mut record) = storage.sessions().get(session) else {
        return false;
    };
    if record.running.as_ref() == Some(next) {
        return false; // unchanged: no write, no emit
    }
    record.running = Some(next.clone());
    storage.sessions().update(&record).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        AppInfo, InstanceInfo, LiveStatsMap, NeedsAttention, QuotaSnapshotInfo, TokenCountsInfo,
        git_branch_of, needs_attention_of, session_infos,
    };
    use juggler_core::model::{
        Account, AccountId, Confidence, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
        ProfileId, QuotaWindow, SessionId, SessionStatus, SettingsOverlay, Timestamp, TokenCounts,
        UsageSnapshot,
    };
    use juggler_core::storage::Storage;

    #[test]
    fn date_compact_matches_known_civil_dates() {
        use super::date_compact_from_millis;
        let day = |days: i64| days * 86_400_000;
        assert_eq!(date_compact_from_millis(0), "19700101");
        assert_eq!(date_compact_from_millis(day(20_738)), "20261012");
        assert_eq!(date_compact_from_millis(day(20_739)), "20261013");
        // Leap-year boundary: 2024-02-29 is day 19782.
        assert_eq!(date_compact_from_millis(day(19_782)), "20240229");
    }
    use juggler_supervisor::LiveProcessStats;

    #[test]
    fn app_info_is_populated() {
        let info = AppInfo::current();
        assert_eq!(info.name, "AI Juggler");
        assert!(!info.version.is_empty());
    }

    /// A sacrificial git cwd: `<tmp>/.git/HEAD` with a branch ref. Never a
    /// real repository — fixtures only (workspace safety rule). Every call
    /// gets a unique directory: tests run in parallel and a shared path
    /// would let one test's `fs::write` truncate while another reads.
    fn git_cwd(branch: &str) -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "aij-e401-{}-{}-{n}",
            std::process::id(),
            branch.replace('/', "-")
        ));
        let git = dir.join(".git");
        std::fs::create_dir_all(&git).expect("fixture .git dir");
        std::fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n"))
            .expect("fixture HEAD");
        dir
    }

    fn record(
        id: &str,
        recipe: &str,
        status: SessionStatus,
        cwd: std::path::PathBuf,
    ) -> InstanceRecord {
        InstanceRecord {
            id: SessionId::new(id),
            conversation: None,
            profile: ProfileId::new(recipe),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd,
            status,
            started_at: Timestamp::from_millis(1_700_000_000_000),
            last_activity: Some(Timestamp::from_millis(1_700_000_060_000)),
            usage: UsageSnapshot::default(),
            attention: None,
            running: None,
            window: None,
            transcript_ref: None,
        }
    }

    /// Storage seeded with one account, two launch recipes (with and
    /// without a model) and the four AJ-E401 fixture rows.
    fn fixture_storage() -> Storage {
        let storage = Storage::open_in_memory().expect("in-memory storage");
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-1"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: std::env::temp_dir().join("aij-e401-account-state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .expect("seed account");
        for (id, model) in [("rec-1", Some("opus")), ("rec-2", None)] {
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new(id),
                    name: "fixture".into(),
                    account: AccountId::new("acc-1"),
                    settings_overlay: SettingsOverlay {
                        model: model.map(String::from),
                        env: Default::default(),
                    },
                    default_cwd: None,
                })
                .expect("seed recipe");
        }

        // Full row: every source populated.
        let mut full = record(
            "sess-full",
            "rec-1",
            SessionStatus::Running,
            git_cwd("feat/read-model"),
        );
        full.usage = UsageSnapshot {
            tokens: TokenCounts {
                input: 12_345,
                output: 6_789,
                cache_read: 100,
                cache_write: 200,
                reasoning: 42,
            },
            window: Some(QuotaWindow {
                used_percentage: 42.5,
                resets_at: Some(Timestamp::from_millis(1_700_010_000_000)),
            }),
            confidence: Confidence::Computed,
        };
        storage.sessions().insert(&full).expect("seed full");

        // Quota-less row: no feed, tokens never observed.
        storage
            .sessions()
            .insert(&record(
                "sess-no-quota",
                "rec-1",
                SessionStatus::Running,
                std::env::temp_dir(),
            ))
            .expect("seed quota-less");

        // Model-less row, terminal: launch settings carry no model; a
        // finished Instance is not resource-measurable.
        storage
            .sessions()
            .insert(&record(
                "sess-no-model",
                "rec-2",
                SessionStatus::Finished,
                std::env::temp_dir(),
            ))
            .expect("seed model-less");

        // Dangling recipe rows cannot exist in storage (the sessions table
        // foreign-keys launch_recipe_id), so the join's generic-marker
        // fallback is covered by `unjoinable_rows_render_generic_markers`
        // below instead of an insert.

        // Waiting row with a resolvable account: attention reason present.
        storage
            .sessions()
            .insert(&record(
                "sess-waiting",
                "rec-1",
                SessionStatus::WaitingUser,
                std::env::temp_dir(),
            ))
            .expect("seed waiting");
        storage
    }

    #[allow(clippy::panic)] // fixture lookup: panicking names the missing row
    fn row<'a>(rows: &'a [InstanceInfo], id: &str) -> &'a InstanceInfo {
        rows.iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("row {id} missing"))
    }

    /// AJ-E401 fixtures: full row, quota-less row, model-less row — absent
    /// data serialises as `null`, never `0`, `""`, or a guessed name.
    #[test]
    fn session_info_full_row_carries_every_field() {
        let storage = fixture_storage();
        let live: LiveStatsMap = [(
            "sess-full".to_string(),
            LiveProcessStats {
                child_count: 2,
                cpu_pct: Some(3.5),
                rss_bytes: 123_456,
            },
        )]
        .into_iter()
        .collect();
        let rows =
            session_infos(&storage, &live, &std::collections::BTreeMap::new(), &[]).expect("rows");
        let full = row(&rows, "sess-full");

        assert_eq!(full.account_id.as_deref(), Some("acc-1"));
        assert_eq!(full.account_label, "Work Max");
        assert_eq!(full.provider, "claude-code");
        assert_eq!(full.model.as_deref(), Some("opus"));
        let quota = full.quota.as_ref().expect("quota present");
        assert_eq!(
            quota,
            &QuotaSnapshotInfo {
                used_percentage: 42.5,
                resets_at_ms: Some(1_700_010_000_000),
                confidence: "computed".into(),
            }
        );
        assert_eq!(
            full.tokens,
            Some(TokenCountsInfo {
                input: 12_345,
                output: 6_789,
                cache_read: 100,
                cache_write: 200,
                reasoning: 42,
            })
        );
        assert_eq!(full.cpu_pct, Some(3.5));
        assert_eq!(full.rss_bytes, Some(123_456));
        assert_eq!(full.child_count, Some(2));
        assert_eq!(full.git_branch.as_deref(), Some("feat/read-model"));
        assert_eq!(full.needs_attention, None);
        assert_eq!(full.last_activity_ms, Some(1_700_000_060_000));
        assert_eq!(full.machine_id, "this-mac");
    }

    #[test]
    fn session_info_absent_data_is_null_not_zero() {
        let storage = fixture_storage();
        let rows = session_infos(
            &storage,
            &LiveStatsMap::new(),
            &std::collections::BTreeMap::new(),
            &[],
        )
        .expect("rows");

        // Quota-less: no feed, no observed tokens, never sampled.
        let no_quota = row(&rows, "sess-no-quota");
        assert_eq!(no_quota.quota, None);
        assert_eq!(no_quota.tokens, None);
        assert_eq!(no_quota.cpu_pct, None);
        assert_eq!(no_quota.rss_bytes, None);
        assert_eq!(no_quota.child_count, None);
        // Non-git cwd: the field is omitted entirely, not blank.
        assert_eq!(no_quota.git_branch, None);

        // Model-less: null, never a guessed default model name.
        let no_model = row(&rows, "sess-no-model");
        assert_eq!(no_model.model, None);
        // Terminal status: nothing is live; child_count 0 is a fact, cpu
        // and rss are unmeasurable — null, not zero.
        assert_eq!(no_model.child_count, Some(0));
        assert_eq!(no_model.cpu_pct, None);
        assert_eq!(no_model.rss_bytes, None);
        assert_eq!(no_model.needs_attention, None);

        // Waiting: structured attention reason, not a re-derived guess.
        assert_eq!(
            row(&rows, "sess-waiting").needs_attention,
            Some(NeedsAttention::WaitingUser)
        );

        // Wire honesty: the serialised payload shows null, not zeroed
        // objects or empty strings.
        for id in ["sess-no-quota", "sess-no-model"] {
            let json = serde_json::to_string(row(&rows, id)).expect("serialize");
            assert!(json.contains(r#""quota":null"#), "{id}: {json}");
            assert!(json.contains(r#""tokens":null"#), "{id}: {json}");
            assert!(
                json.contains(r#""account_label":"Work Max""#),
                "{id}: {json}"
            );
        }
    }

    /// AJ-E413 Req.1: desktop rows carry the per-Provider isolation
    /// claim verbatim; terminal rows and unregistered providers stay
    /// `null` — the panel never guesses a limit.
    #[test]
    fn session_info_desktop_isolation_label_maps_per_provider() {
        let storage = fixture_storage();
        let mut desktop = record(
            "sess-desk",
            "rec-1",
            SessionStatus::Running,
            git_cwd("feat/x"),
        );
        desktop.kind = InstanceKind::Desktop;
        storage.sessions().insert(&desktop).expect("seed desktop");

        let labels = std::collections::BTreeMap::from([(
            "claude-code".to_string(),
            "Single-account app — Juggler tracks, cannot isolate".to_string(),
        )]);
        let rows = session_infos(&storage, &LiveStatsMap::new(), &labels, &[]).expect("rows");
        assert_eq!(
            row(&rows, "sess-desk").isolation_label.as_deref(),
            Some("Single-account app — Juggler tracks, cannot isolate")
        );
        // Terminal instances never carry a desktop claim.
        assert_eq!(row(&rows, "sess-full").isolation_label, None);
        // Desktop row whose provider has no registered label: null, not
        // an inherited or default claim.
        let bare = session_infos(
            &storage,
            &LiveStatsMap::new(),
            &std::collections::BTreeMap::new(),
            &[],
        )
        .expect("rows");
        assert_eq!(row(&bare, "sess-desk").isolation_label, None);
    }

    /// AJ-E401 performance budget: the in-memory join (storage reads +
    /// per-row derivation, excluding the batched process-table refresh)
    /// stays under the 20 ms p95 budget at the 50-Instance target scale.
    #[test]
    fn session_info_join_p95_under_budget_at_50_instances() {
        let storage = Storage::open_in_memory().expect("storage");
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-bench"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Bench".into(),
                plan: None,
                state_dir: std::env::temp_dir().join("aij-e401-bench"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .expect("seed account");
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-bench"),
                name: "bench".into(),
                account: AccountId::new("acc-bench"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .expect("seed recipe");
        for n in 0..50 {
            let mut record = record(
                &format!("sess-bench-{n}"),
                "rec-bench",
                SessionStatus::Running,
                std::env::temp_dir(),
            );
            record.usage.tokens = TokenCounts {
                input: n,
                ..TokenCounts::default()
            };
            storage.sessions().insert(&record).expect("seed row");
        }
        let mut samples: Vec<u128> = Vec::new();
        for _ in 0..100 {
            let start = std::time::Instant::now();
            let rows = session_infos(
                &storage,
                &LiveStatsMap::new(),
                &std::collections::BTreeMap::new(),
                &[],
            )
            .expect("rows");
            assert_eq!(rows.len(), 50);
            samples.push(start.elapsed().as_micros());
        }
        samples.sort_unstable();
        let p95 = samples[94]; // 95th of 100
        assert!(p95 < 20_000, "join p95 {p95}µs exceeds the 20 ms budget");
    }

    /// Unknown statuses cannot reach the read model: storage rejects them
    /// at row-mapping time (degrade-to-error, never a panic), so the wire
    /// only ever carries the nine vocabulary strings — verified by
    /// round-tripping every status through the mapper.
    #[test]
    fn session_info_status_vocabulary_is_the_nine_wire_strings() {
        let storage = fixture_storage();
        let rows = session_infos(
            &storage,
            &LiveStatsMap::new(),
            &std::collections::BTreeMap::new(),
            &[],
        )
        .expect("rows");
        let vocabulary: Vec<&str> = rows.iter().map(|row| row.status.as_str()).collect();
        for wire in [
            "starting",
            "running",
            "waiting_user",
            "stuck",
            "quota_blocked",
            "finished",
            "stopped",
            "failed",
            "handed_over",
        ] {
            assert!(
                SessionStatus::parse(wire).is_some(),
                "wire status {wire} must parse"
            );
        }
        assert!(
            vocabulary
                .iter()
                .all(|status| SessionStatus::parse(status).is_some())
        );
        assert_eq!(
            needs_attention_of(SessionStatus::parse("quota_blocked").unwrap()),
            Some(NeedsAttention::QuotaBlocked)
        );
    }

    /// Defensive join fallback (storage FKs make this unreachable today):
    /// an unjoinable row renders generic `unknown` markers — account id
    /// `null`, provider/label never blank (AJ-E401 Req.2).
    #[test]
    fn unjoinable_rows_render_generic_markers() {
        use super::resolved_join;
        use std::collections::BTreeMap;
        let (account_id, account_label, provider, model) =
            resolved_join(&BTreeMap::new(), &BTreeMap::new(), "rec-gone");
        assert_eq!(account_id, None);
        assert_eq!(account_label, "unknown");
        assert_eq!(provider, "unknown");
        assert_eq!(model, None);
    }

    /// I-09 branch parsing: ref rows parse, detached and missing `.git`
    /// stay `None` (unknown, not a guess).
    #[test]
    fn git_branch_parses_refs_only() {
        let cwd = git_cwd("main");
        assert_eq!(git_branch_of(&cwd), Some("main".to_string()));
        assert_eq!(git_branch_of(&std::env::temp_dir()), None);
        let detached =
            std::env::temp_dir().join(format!("aij-e401-{}-detached", std::process::id()));
        std::fs::create_dir_all(detached.join(".git")).expect("fixture");
        std::fs::write(
            detached.join(".git/HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .expect("fixture");
        assert_eq!(git_branch_of(&detached), None);
        std::fs::remove_dir_all(detached).ok();
        std::fs::remove_dir_all(cwd.join(".git")).ok();
    }

    /// AJ-E401 Req.5: the committed ts-rs output contains every new read
    /// model field (regenerated by `cargo test`; committed with the code).
    #[test]
    #[allow(clippy::panic)] // failure message names the missing file
    fn generated_ts_contains_read_model_fields() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/types/generated");
        let read = |name: &str| {
            std::fs::read_to_string(format!("{dir}/{name}.ts"))
                .unwrap_or_else(|error| panic!("generated {name}.ts: {error}"))
        };
        let instance = read("InstanceInfo");
        for field in [
            "account_id",
            "account_label",
            "model",
            "quota",
            "tokens",
            "cpu_pct",
            "rss_bytes",
            "child_count",
            "subtask_count",
            "git_branch",
            "git_dirty_count",
            "needs_attention",
            "last_activity_ms",
            "machine_id",
        ] {
            assert!(
                instance.contains(&format!("{field}:")),
                "InstanceInfo.ts is missing `{field}`"
            );
        }
        for (name, fields) in [
            (
                "QuotaSnapshotInfo",
                &["used_percentage", "resets_at_ms", "confidence"][..],
            ),
            (
                "TokenCountsInfo",
                &["input", "output", "cache_read", "cache_write", "reasoning"][..],
            ),
            (
                "InstanceResourcesInfo",
                &["id", "cpu_pct", "rss_bytes", "child_count"][..],
            ),
            ("InstanceQuotaInfo", &["id", "quota"][..]),
        ] {
            let file = read(name);
            for field in fields {
                assert!(
                    file.contains(&format!("{field}:")),
                    "{name}.ts is missing `{field}`"
                );
            }
        }
        let needs = read("NeedsAttention");
        for variant in ["waiting_user", "stuck", "quota_blocked", "failed"] {
            assert!(
                needs.contains(variant),
                "NeedsAttention.ts is missing `{variant}`"
            );
        }
    }
}

/// One-version IPC aliases (AJ-E107 Req.2): legacy command names keep
/// deserialising while UI call sites migrate to the canonical names.
pub mod legacy_aliases {
    use super::{
        AppResult, InstanceInfo, LaunchRecipeInfo, ProviderInfo, harnesses_list, profiles_list,
        sessions_list,
    };
    use crate::state::AppState;
    use std::sync::Arc;

    /// Legacy alias of `providers_list`.
    #[tauri::command]
    #[must_use]
    pub fn harnesses_list_alias() -> Vec<ProviderInfo> {
        harnesses_list()
    }

    /// Legacy alias of `launch_recipes_list`.
    #[tauri::command]
    ///
    /// # Errors
    ///
    /// Returns the parent command's typed error on failure.
    pub fn profiles_list_alias(
        state: tauri::State<'_, Arc<AppState>>,
    ) -> AppResult<Vec<LaunchRecipeInfo>> {
        profiles_list(state)
    }

    /// Legacy alias of `instances_list`.
    #[tauri::command]
    ///
    /// # Errors
    ///
    /// Returns the parent command's typed error on failure.
    pub fn sessions_list_alias(
        state: tauri::State<'_, Arc<AppState>>,
    ) -> AppResult<Vec<InstanceInfo>> {
        sessions_list(state)
    }

    // Silence unused-import when only aliases reference these.
}

/// The still-holding report's holder exe (basename of the live
/// command's first token — `Listeners` carry no exe by design).
fn holder_exe(pid: u32) -> String {
    juggler_supervisor::live_details(pid).map_or_else(
        || "?".to_string(),
        |(cmdline, _)| {
            cmdline
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .rsplit('/')
                .next()
                .unwrap_or("?")
                .to_string()
        },
    )
}

// ---------------------------------------------------------------------------
// AJ-E207: switch-back — resume the original native session plus ONE
// delta turn; never a copy-back (D-05; pass2/02 §6.1-6.2).

/// One listed conflict for the review UI (three radio choices each,
/// keyboard-completable — never auto-merged).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct FileConflictInfo {
    /// Pack-relative path (the carry-list name).
    pub rel_path: String,
    /// Fork sha256, when the base manifest tracked it.
    pub base_sha: Option<String>,
    /// Away-side sha256 today (`null` = deleted there).
    pub away_sha: Option<String>,
    /// Original-side sha256 today (`null` = deleted there).
    pub original_sha: Option<String>,
}

impl FileConflictInfo {
    fn of(conflict: &juggler_handover::reconcile::FileConflict) -> Self {
        Self {
            rel_path: conflict.rel_path.clone(),
            base_sha: conflict.base_sha.clone(),
            away_sha: conflict.away_sha.clone(),
            original_sha: conflict.original_sha.clone(),
        }
    }
}

/// The switch-back plan the suggestion card / review modal renders
/// (suggest-only; the user clicks — SPEC:26,263).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct SwitchBackPlanInfo {
    /// The verbatim quota-reset suggestion (Req.3).
    pub suggestion: String,
    /// The original account's label (where work returns).
    pub original_account: String,
    /// The away account's label.
    pub away_account: String,
    /// The native id the return resumes (empty when S-F7).
    pub original_native_id: String,
    /// The original transcript still exists.
    pub original_alive: bool,
    /// S-F7 fallback wire name ("" | "nearest-epoch" | "fresh-f0").
    pub sf7_fallback: String,
    /// §7.1 S-F7 copy when the original is gone.
    pub sf7_copy: String,
    /// The away instance is still live (freeze-or-fork offer).
    pub away_live: bool,
    /// The both-live warning copy (§6.2 — silence is how
    /// wrong-account billing happens).
    pub both_live_copy: String,
    /// The divergence headline (§7.1 shape: counts, never "some
    /// context"; nothing merged silently).
    pub divergence: String,
    /// The rendered "While you were away" delta turn (preview).
    pub delta_preview: String,
    /// Away-only turn count (the delta body).
    pub away_only_turns: u32,
    /// Original-only turn count (already native; no-op).
    pub original_only_turns: u32,
    /// Shared (fork) turn count.
    pub base_turns: u32,
    /// Tracked-artifact conflicts needing a choice.
    pub conflicts: Vec<FileConflictInfo>,
}

/// One user resolution from the review UI.
#[derive(Clone, PartialEq, Eq, Debug, serde::Deserialize)]
pub struct SwitchBackChoiceIn {
    /// The conflicting path.
    pub rel_path: String,
    /// `keep-original|take-away|keep-both`.
    pub choice: String,
}

/// The lineage facts every switch-back path walks once.
struct BackLineage {
    /// The work both epochs belong to.
    work: juggler_core::model::WorkItemId,
    /// The away side's epoch (what the excursion became).
    away_epoch: juggler_core::model::Epoch,
    /// The original side's epoch at the fork.
    original_epoch: juggler_core::model::Epoch,
    /// The confirmed fork edge (switch A→B).
    fork_edge: juggler_core::model::SwitchEdge,
    /// The away instance's session row.
    away_session: juggler_core::model::InstanceRecord,
    /// The original session row, when it still exists.
    original_session: Option<juggler_core::model::InstanceRecord>,
}

/// Walks the graph from an away instance to its fork: the confirmed
/// edge that created it, both epochs, and both session rows.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the session has no switch
/// lineage (never switched away) or the graph is torn.
fn back_lineage(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<(
    BackLineage,
    juggler_core::model::Account,
    juggler_core::model::Account,
)> {
    use juggler_core::model::{EdgeState, SessionId};

    let storage = lock_storage_arc(state)?;
    let away_session = storage
        .sessions()
        .get(&SessionId::new(session_id))
        .map_err(|error| not_found_or_core(error, "session", session_id))?;
    let Some(_project) = away_session.project.clone() else {
        return Err(AppError::Io {
            context: "switch back",
            message: "session has no project link".into(),
        });
    };
    let graph = storage.switch_graph();
    let mut works = Vec::new();
    for record in storage.projects().list()? {
        works.extend(graph.works_for_project(&record.id)?);
    }
    let mut lineage: Option<BackLineage> = None;
    for work in &works {
        let epochs = graph.epochs_for_work(&work.id)?;
        let Some(away_epoch) = epochs
            .iter()
            .find(|epoch| {
                epoch
                    .session_ref
                    .as_ref()
                    .is_some_and(|id| id.as_str() == session_id)
            })
            .cloned()
        else {
            continue;
        };
        let Some(fork_edge) = graph.edges_for_work(&work.id)?.into_iter().find(|edge| {
            edge.to_epoch.as_ref() == Some(&away_epoch.id)
                && matches!(edge.state, EdgeState::Confirmed)
        }) else {
            continue;
        };
        let original_epoch = graph
            .epochs_for_work(&work.id)?
            .into_iter()
            .find(|epoch| epoch.id == fork_edge.from_epoch)
            .ok_or_else(|| AppError::Io {
                context: "switch back",
                message: "The switch's source epoch is missing from the graph".to_owned(),
            })?;
        let original_session = original_epoch
            .session_ref
            .as_ref()
            .and_then(|id| storage.sessions().get(id).ok());
        lineage = Some(BackLineage {
            work: work.id.clone(),
            away_epoch,
            original_epoch,
            fork_edge,
            away_session,
            original_session,
        });
        break;
    }
    let Some(lineage) = lineage else {
        return Err(AppError::Io {
            context: "switch back",
            message: "This session never switched away — nothing to return to".to_owned(),
        });
    };
    let away_account = storage
        .accounts()
        .get(&lineage.away_epoch.account)
        .map_err(|error| {
            not_found_or_core(error, "account", &lineage.away_epoch.account.to_string())
        })?;
    let original_account = storage
        .accounts()
        .get(&lineage.original_epoch.account)
        .map_err(|error| {
            not_found_or_core(
                error,
                "account",
                &lineage.original_epoch.account.to_string(),
            )
        })?;
    Ok((lineage, away_account, original_account))
}

/// One side's conversation facts: the uuid set (reconciliation) and
/// the user/assistant turns in file order (the delta body).
fn transcript_side(
    path: &std::path::Path,
) -> Option<(
    std::collections::HashSet<String>,
    Vec<juggler_handover::pack::IrTurn>,
)> {
    use juggler_adapters::claude_transcript::{MessageRole, parse_transcript};
    let bytes = std::fs::read(path).ok()?;
    let parsed = parse_transcript(&bytes);
    let mut uuids = std::collections::HashSet::new();
    let mut turns = Vec::new();
    for message in parsed.messages {
        if let Some(uuid) = &message.uuid {
            uuids.insert(uuid.clone());
        }
        let role = match message.role {
            MessageRole::User => juggler_handover::pack::IrRole::User,
            MessageRole::Assistant => juggler_handover::pack::IrRole::Assistant,
            MessageRole::Other => continue,
        };
        turns.push(juggler_handover::pack::IrTurn {
            uuid: message.uuid,
            role,
            text: message.text,
            at_ms: None,
            tokens: None,
        });
    }
    Some((uuids, turns))
}

/// Hashes the fork manifest's FILE-domain rows on one side now
/// (transcript-kind rows are the conversation domain — uuid sets,
/// never file choices; that split is how the copy-back ban stays
/// structural).
fn file_domain_hashes_now(
    fork_rows: &[juggler_core::model::ManifestEntry],
    side_transcript: Option<&juggler_core::model::TranscriptRef>,
) -> juggler_handover::reconcile::ArtifactHashes {
    let mut hashes = juggler_handover::reconcile::ArtifactHashes::new();
    let Some(reference) = side_transcript else {
        return hashes;
    };
    let Some(dir) = reference.path.parent() else {
        return hashes;
    };
    for row in fork_rows.iter().filter(|row| row.kind != "transcript") {
        if let Ok(bytes) = std::fs::read(dir.join(&row.rel_path)) {
            hashes.insert(
                row.rel_path.clone(),
                juggler_handover::manifest::sha256_hex(&bytes),
            );
        }
    }
    hashes
}

/// Assembles the plan facts shared by plan/execute (one walk, one
/// reconcile — the same truth both halves render and act on).
#[allow(clippy::type_complexity)]
fn back_facts(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<(
    BackLineage,
    juggler_core::model::Account,
    juggler_core::model::Account,
    juggler_handover::switch_back::DeltaInput,
    Vec<juggler_handover::reconcile::FileConflict>,
    bool,
    juggler_handover::switch_back::OriginalSession,
)> {
    use juggler_handover::reconcile::{reconcile_conversation, reconcile_files};

    let (lineage, away_account, original_account) = back_lineage(state, session_id)?;
    let away_reference = lineage.away_session.transcript_ref.clone();
    let original_reference = lineage
        .original_session
        .as_ref()
        .and_then(|row| row.transcript_ref.clone());
    let original_alive = original_reference
        .as_ref()
        .is_some_and(|reference| reference.path.is_file());

    // S-F7: never fabricate — nearest surviving epoch on the
    // ORIGINAL account whose transcript still exists.
    let nearest_surviving = {
        let storage = lock_storage_arc(state)?;
        let graph = storage.switch_graph();
        graph
            .epochs_for_work(&lineage.work)?
            .into_iter()
            .filter(|epoch| {
                epoch.account == original_account.id && epoch.id != lineage.original_epoch.id
            })
            .find_map(|epoch| {
                let row = epoch
                    .session_ref
                    .as_ref()
                    .and_then(|id| storage.sessions().get(id).ok())?;
                let reference = row.transcript_ref?;
                reference.native_id.filter(|_| reference.path.is_file())
            })
    };
    let verdict = juggler_handover::switch_back::original_session_verdict(
        original_alive,
        lineage
            .original_epoch
            .native_id
            .as_deref()
            .filter(|id| !id.is_empty()),
        nearest_surviving.as_deref(),
    );

    // Conversation: base = intersection (B began as A's bytes);
    // away-only renders the delta; original-only is a no-op.
    let away_side = away_reference
        .as_ref()
        .and_then(|reference| transcript_side(&reference.path));
    let original_side = original_reference
        .as_ref()
        .and_then(|reference| transcript_side(&reference.path));
    let empty = std::collections::HashSet::new();
    let (away_uuids, away_turns) = away_side.unwrap_or_else(|| (empty.clone(), Vec::new()));
    let (original_uuids, _) = original_side.unwrap_or_else(|| (empty, Vec::new()));
    let conversation = reconcile_conversation(&away_uuids, &original_uuids);

    // File domain: the fork manifest's non-transcript rows, hashed
    // on both sides now (three-way; conflicts never auto-merge).
    let fork_rows = {
        let storage = lock_storage_arc(state)?;
        storage
            .switch_graph()
            .manifest_for(&lineage.fork_edge.id)
            .unwrap_or_default()
    };
    let base: juggler_handover::reconcile::ArtifactHashes = fork_rows
        .iter()
        .filter(|row| row.kind != "transcript")
        .map(|row| (row.rel_path.clone(), row.sha256.clone()))
        .collect();
    let away_files = file_domain_hashes_now(&fork_rows, away_reference.as_ref());
    let original_files = file_domain_hashes_now(&fork_rows, original_reference.as_ref());
    let conflicts = reconcile_files(&base, &away_files, &original_files);

    let delta_input = juggler_handover::switch_back::DeltaInput {
        away_account: away_account.label.clone(),
        original_account: original_account.label.clone(),
        away_turns,
        away_only_uuids: conversation.away_only_uuids.iter().cloned().collect(),
        conversation: conversation.clone(),
        diff_stat: None, // the away pack's freeze-time stat rides E209's pack surfaces
        todo_note: None,
    };
    let away_live = matches!(
        lineage.away_session.status.as_str(),
        "running" | "starting" | "waiting_user"
    );
    Ok((
        lineage,
        away_account,
        original_account,
        delta_input,
        conflicts,
        away_live,
        verdict,
    ))
}

/// AJ-E207: the switch-back plan for one away instance — verbatim
/// suggestion, S-F7 verdict, both-live warning, divergence counts,
/// the delta preview, and the conflict list. Suggest-only.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the lineage walk fails.
#[tauri::command]
pub fn switch_back_plan(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<SwitchBackPlanInfo> {
    switch_back_plan_impl(state.inner(), &session_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_back_plan_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<SwitchBackPlanInfo> {
    use juggler_handover::switch_back::{
        OriginalSession, Sf7Fallback, both_live_copy, delta_turn_text, divergence_copy,
        suggestion_copy,
    };

    let (_lineage, _away_account, original_account, delta_input, conflicts, away_live, verdict) =
        back_facts(state, session_id)?;
    let away_only_turns =
        u32::try_from(juggler_handover::switch_back::delta_turns(&delta_input).len())
            .unwrap_or(u32::MAX);
    let (original_native_id, sf7_fallback, sf7_copy) = match &verdict {
        OriginalSession::Alive { native_id } => (native_id.clone(), String::new(), String::new()),
        OriginalSession::Gone { fallback } => (
            String::new(),
            match fallback {
                Sf7Fallback::NearestEpoch { native_id } => {
                    format!("nearest-epoch:{native_id}")
                }
                Sf7Fallback::FreshPackF0 => "fresh-f0".to_owned(),
            },
            format!(
                "{}",
                juggler_handover::pipeline::SwitchFailure::OriginalSessionGone
            ),
        ),
    };
    Ok(SwitchBackPlanInfo {
        suggestion: suggestion_copy(&original_account.label, &delta_input.away_account),
        original_account: original_account.label.clone(),
        away_account: delta_input.away_account.clone(),
        original_native_id,
        original_alive: matches!(verdict, OriginalSession::Alive { .. }),
        sf7_fallback,
        sf7_copy,
        away_live,
        both_live_copy: both_live_copy(&original_account.label, &delta_input.away_account),
        divergence: divergence_copy(away_only_turns as usize, conflicts.len()),
        delta_preview: delta_turn_text(&delta_input),
        away_only_turns,
        original_only_turns: u32::try_from(delta_input.conversation.original_only_uuids)
            .unwrap_or(u32::MAX),
        base_turns: u32::try_from(delta_input.conversation.base_uuids).unwrap_or(u32::MAX),
        conflicts: conflicts.iter().map(FileConflictInfo::of).collect(),
    })
}

/// AJ-E207: executes the switch-back — applies the user's conflict
/// choices, stops-or-keeps the away instance per the explicit
/// choice, resumes the ORIGINAL native session with the one delta
/// turn, and records the switch-back edge (`resumed_from_epoch` =
/// the original epoch; parent = the fork; choices in the manifest).
/// The original transcript is never written (copy-back has no code
/// path).
///
/// # Errors
///
/// Returns the typed [`AppError`] on unresolved conflicts (S-F8 —
/// the work stays forked, both sides resumable), spawn failure, or
/// graph write failure.
#[tauri::command]
pub fn switch_back_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    choices: Vec<SwitchBackChoiceIn>,
    keep_away_running: bool,
) -> AppResult<LaunchResult> {
    switch_back_execute_impl(state.inner(), &session_id, &choices, keep_away_running)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_back_execute_impl(
    state: &Arc<AppState>,
    session_id: &str,
    choices: &[SwitchBackChoiceIn],
    keep_away_running: bool,
) -> AppResult<LaunchResult> {
    use juggler_handover::switch_back::{plan_resolutions, unresolved};

    let (lineage, away_account, original_account, delta_input, conflicts, away_live, verdict) =
        back_facts(state, session_id)?;
    // S-F8: every conflict resolved or the work stays forked — an
    // error, never a guess.
    let resolutions = parse_switch_back_choices(choices);
    if !unresolved(&conflicts, &resolutions).is_empty() {
        return Err(AppError::Io {
            context: "switch back",
            message: format!(
                "{}",
                juggler_handover::pipeline::SwitchFailure::MergeConflictOpen
            ),
        });
    }

    // Apply the file plan (file domain only — the transcript is not
    // an artifact, so no path here can touch it).
    let actions = plan_resolutions(&conflicts, &resolutions, &chrono_date_compact());
    apply_conflict_choices(&lineage, &actions);

    // The away instance: stop per the explicit choice, or keep it
    // running (legal — the plan's both-live copy already said so).
    if !keep_away_running && away_live {
        stop_away_instance(state, &lineage.away_session.id)?;
    }

    resume_original_with_delta(
        state,
        &lineage,
        &away_account,
        original_account,
        &delta_input,
        &verdict,
        &actions,
    )
}

/// Parses the UI's wire choices into resolutions (unknown wire
/// names are dropped — an unresolved conflict then stops the
/// execute, never guessed).
fn parse_switch_back_choices(
    choices: &[SwitchBackChoiceIn],
) -> Vec<juggler_handover::switch_back::ConflictResolution> {
    use juggler_handover::reconcile::ConflictChoice;
    choices
        .iter()
        .filter_map(|row| {
            let choice = match row.choice.as_str() {
                "keep-original" => ConflictChoice::KeepOriginal,
                "take-away" => ConflictChoice::TakeAway,
                "keep-both" => ConflictChoice::KeepBoth,
                _ => return None,
            };
            Some(juggler_handover::switch_back::ConflictResolution {
                rel_path: row.rel_path.clone(),
                choice,
            })
        })
        .collect()
}

/// Applies the chosen file actions between the two transcript
/// directories (file domain only — the transcript itself is never
/// an action target, which keeps the copy-back ban structural).
fn apply_conflict_choices(
    lineage: &BackLineage,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) {
    use juggler_handover::reconcile::FileAction;
    let away_dir = lineage
        .away_session
        .transcript_ref
        .as_ref()
        .and_then(|reference| reference.path.parent().map(std::path::Path::to_path_buf));
    let original_dir = lineage
        .original_session
        .as_ref()
        .and_then(|row| row.transcript_ref.as_ref())
        .and_then(|reference| reference.path.parent().map(std::path::Path::to_path_buf));
    for (resolution, action) in actions {
        let (Some(away_dir), Some(original_dir)) = (away_dir.as_ref(), original_dir.as_ref())
        else {
            continue;
        };
        let source = away_dir.join(&resolution.rel_path);
        match action {
            FileAction::ReplaceWithAway => {
                let _ = std::fs::copy(&source, original_dir.join(&resolution.rel_path));
            }
            FileAction::SaveAwaySibling { sibling_rel_path } => {
                let _ = std::fs::copy(&source, original_dir.join(sibling_rel_path));
            }
            FileAction::Unchanged => {}
        }
    }
}

/// Stops the away instance (freeze-or-fork's stop side). Already-
/// gone is fine; a REAL stop failure aborts before the resume.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the stop ladder fails.
fn stop_away_instance(state: &Arc<AppState>, session: &SessionId) -> AppResult<()> {
    match state.supervisor.stop(session, Duration::from_secs(5)) {
        Ok(()) | Err(juggler_supervisor::SupervisorError::UnknownSession(_)) => Ok(()),
        Err(error) => Err(AppError::Io {
            context: "switch back",
            message: format!("Couldn't stop the away instance: {error}"),
        }),
    }
}

/// Spawns the return (resume + delta, or the honest S-F7 fallback)
/// and records the switch-back edge: resumes the ORIGINAL epoch,
/// parent = the fork; the conflict choices ride the manifest (E201).
///
/// # Errors
///
/// Returns the typed [`AppError`] on resolution, spawn, or graph
/// write failure — the original transcript is never written on any
/// path (copy-back has no code path).
#[allow(clippy::too_many_arguments)]
fn resume_original_with_delta(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away_account: &juggler_core::model::Account,
    original_account: juggler_core::model::Account,
    delta_input: &juggler_handover::switch_back::DeltaInput,
    verdict: &juggler_handover::switch_back::OriginalSession,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) -> AppResult<LaunchResult> {
    use juggler_handover::switch_back::{OriginalSession, Sf7Fallback, delta_turn_text};

    // What resumes: the original native id, or the S-F7 fallback
    // (never fabricated). Fresh-f0 = a fresh interactive start
    // honestly labelled F0 (the pack carries the detail; nothing is
    // primed silently); mechanism + fidelity are decided at the
    // edge (record_switch_back_edge).
    let (resume_id, fresh) = match verdict {
        OriginalSession::Alive { native_id }
        | OriginalSession::Gone {
            fallback: Sf7Fallback::NearestEpoch { native_id },
        } => (native_id.clone(), false),
        OriginalSession::Gone {
            fallback: Sf7Fallback::FreshPackF0,
        } => (String::new(), true),
    };
    let mode = if fresh {
        LaunchMode::Interactive
    } else {
        LaunchMode::Resume {
            session_id: resume_id.clone(),
            delta_prompt: Some(delta_turn_text(delta_input)),
        }
    };

    let original_session = lineage
        .original_session
        .clone()
        .ok_or_else(|| AppError::Io {
            context: "switch back",
            message: "The original session row is missing — resume it from the graph".to_owned(),
        })?;
    let (profile_a, account_a) = {
        let storage = lock_storage_arc(state)?;
        let profile = storage
            .profiles()
            .get(&original_session.profile)
            .map_err(|error| {
                not_found_or_core(error, "profile", &original_session.profile.to_string())
            })?;
        (profile, original_account)
    };
    let adapter = state
        .registry
        .get(&account_a.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_a.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let spec = adapter.launch_spec(&account_a, &profile_a, &original_session.cwd, &mode)?;
    let spawned = state.supervisor.spawn(
        &new_session,
        &spec,
        &ledger_ctx(
            state,
            &account_a,
            original_session.project.as_ref().map(ToString::to_string),
        ),
    )?;

    record_switch_back_edge(
        state,
        lineage,
        away_account,
        &account_a,
        &profile_a,
        &original_session,
        &new_session,
        &resume_id,
        fresh,
        actions,
    )?;

    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid: spawned.pid,
    })
}

/// Records the return: the resumed instance row, its epoch
/// (`resumed_from_epoch` = the ORIGINAL epoch), the switch-back
/// edge (parent = the fork; two-parent shape ready for merges),
/// and the per-conflict choices as manifest rows (E201).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage or graph writes fail.
#[allow(clippy::too_many_arguments)]
fn record_switch_back_edge(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away_account: &juggler_core::model::Account,
    account_a: &juggler_core::model::Account,
    profile_a: &juggler_core::model::LaunchRecipe,
    original_session: &juggler_core::model::InstanceRecord,
    new_session: &SessionId,
    resume_id: &str,
    fresh: bool,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) -> AppResult<()> {
    use juggler_core::model::{
        ConversationId, EdgeActor, EdgeState, Epoch, EpochId, InstanceKind, InstanceRecord,
        ManifestEntry, SwitchEdge, SwitchEdgeId, SwitchMechanism,
    };
    let machine = local_machine_id(state);
    let storage = lock_storage_arc(state)?;
    let usage = engine_usage_snapshot(&storage, &profile_a.id);
    storage.sessions().insert(&InstanceRecord {
        id: new_session.clone(),
        conversation: Some(ConversationId::new(resume_id.to_owned())),
        profile: profile_a.id.clone(),
        machine,
        kind: InstanceKind::Terminal,
        project: original_session.project.clone(),
        cwd: original_session.cwd.clone(),
        status: SessionStatus::Starting,
        started_at: Timestamp::now(),
        last_activity: None,
        usage,
        attention: None,
        running: None,
        window: None,
        transcript_ref: original_session.transcript_ref.clone(),
    })?;
    let graph = storage.switch_graph();
    let epoch_id = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: epoch_id.clone(),
        work: lineage.work.clone(),
        session_ref: Some(new_session.clone()),
        native_id: Some(resume_id.to_owned()),
        account: lineage.original_epoch.account.clone(),
        provider: lineage.original_epoch.provider.clone(),
        resumed_from_epoch: Some(lineage.original_epoch.id.clone()),
    })?;
    let edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph.append_edge(&SwitchEdge {
        id: edge_id.clone(),
        work: lineage.work.clone(),
        from_epoch: lineage.away_epoch.id.clone(),
        to_epoch: None, // filled by mark_launched (S-G1)
        from_account: away_account.id.clone(),
        to_account: account_a.id.clone(),
        from_provider: away_account.harness.clone(),
        to_provider: account_a.harness.clone(),
        // Fresh-f0 is a pack-labelled fresh start; a resume is T1.
        mechanism: if fresh {
            SwitchMechanism::Pack
        } else {
            SwitchMechanism::NativeResume
        },
        pack_id: None,
        fidelity_shown: Some(if fresh { "F0" } else { "F4" }.to_owned()),
        manifest_sha256: None,
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: Some(lineage.fork_edge.id.clone()),
        parent_edge_b: None,
        created_at: Timestamp::now(),
    })?;
    graph.mark_launched(&edge_id, &epoch_id)?;
    graph.mark_confirmed(&edge_id)?;
    let choice_rows: Vec<ManifestEntry> = actions
        .iter()
        .map(|(resolution, _)| ManifestEntry {
            rel_path: resolution.rel_path.clone(),
            sha256: String::new(),
            bytes: 0,
            kind: "conflict-choice".to_owned(),
            disposition: resolution.choice.as_str().to_owned(),
        })
        .collect();
    if !choice_rows.is_empty() {
        graph.append_manifest(&edge_id, &choice_rows)?;
    }
    Ok(())
}

/// Today's date as YYYYMMDD (the Keep-both sibling suffix).
fn chrono_date_compact() -> String {
    date_compact_from_millis(Timestamp::now().millis())
}

/// Civil-from-days (Howard Hinnant) — one suffix, no calendar
/// dependency; pinned by test against known dates.
fn date_compact_from_millis(millis: i64) -> String {
    let days = millis.div_euclid(86_400_000);
    // Civil-from-days (Howard Hinnant's algorithm) — no calendar
    // dependency for one suffix.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    let day = doy - (153 * mp + 2) / 5 + 1;
    format!("{year:04}{month:02}{day:02}")
}

/// One suggested return (suggest-only — the user clicks).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct SwitchBackSuggestionInfo {
    /// The away instance to return FROM (the card's target).
    pub session_id: String,
    /// The original account's label (where work returns).
    pub original_account: String,
    /// The away account's label.
    pub away_account: String,
    /// The verbatim suggestion copy (Req.3).
    pub copy: String,
}

/// AJ-E207 Req.3: switch-back suggestions — one open excursion whose
/// ORIGINAL account has quota headroom again (the reset trigger),
/// each with the verbatim copy. Suggests only; nothing executes.
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
#[tauri::command]
pub fn switch_back_suggestions(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<SwitchBackSuggestionInfo>> {
    switch_back_suggestions_impl(state.inner())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
pub fn switch_back_suggestions_impl(
    state: &Arc<AppState>,
) -> AppResult<Vec<SwitchBackSuggestionInfo>> {
    use juggler_core::model::EdgeState;

    let now = Timestamp::now().millis();
    let storage = lock_storage_arc(state)?;
    let accounts = storage.accounts().list()?;
    let graph = storage.switch_graph();
    let mut works = Vec::new();
    for project in storage.projects().list()? {
        works.extend(graph.works_for_project(&project.id)?);
    }
    let mut out = Vec::new();
    for work in &works {
        let edges = graph.edges_for_work(&work.id)?;
        let epochs = graph.epochs_for_work(&work.id)?;
        let epoch_account = |id: &juggler_core::model::EpochId| {
            epochs
                .iter()
                .find(|epoch| &epoch.id == id)
                .map(|epoch| epoch.account.clone())
        };
        for edge in edges
            .iter()
            .filter(|edge| matches!(edge.state, EdgeState::Confirmed))
        {
            // Open excursion: no confirmed edge returns from this
            // away epoch yet.
            let returned = edge.to_epoch.as_ref().is_some_and(|away_epoch| {
                edges.iter().any(|other| {
                    matches!(other.state, EdgeState::Confirmed) && &other.from_epoch == away_epoch
                })
            });
            if returned || edge.to_epoch.is_none() {
                continue;
            }
            // A parent-linked edge is a return/compensation, not a
            // fresh excursion — suggesting its undo would bounce the
            // user straight back.
            if edge.parent_edge_a.is_some() {
                continue;
            }
            let Some(original_account_id) = epoch_account(&edge.from_epoch) else {
                continue;
            };
            let Some(original_account) = accounts
                .iter()
                .find(|account| account.id == original_account_id)
            else {
                continue;
            };
            // The reset trigger: the original account has headroom
            // again (not exhausted, not tight).
            let display = quota_display_inner(&storage, original_account, now);
            let headroom = match display.five_hour.as_ref().map(|w| w.state.as_str()) {
                Some("exhausted") => false,
                Some(_) => display.forecast.tte_min.is_none_or(|tte| tte >= 45.0),
                None => true,
            };
            if !headroom {
                continue;
            }
            let Some(away_session) = edge
                .to_epoch
                .as_ref()
                .and_then(|id| epochs.iter().find(|epoch| &epoch.id == id))
                .and_then(|epoch| epoch.session_ref.clone())
            else {
                continue;
            };
            let away_label = accounts
                .iter()
                .find(|account| account.id == edge.to_account)
                .map(|account| account.label.clone())
                .unwrap_or_default();
            out.push(SwitchBackSuggestionInfo {
                session_id: away_session.to_string(),
                original_account: original_account.label.clone(),
                copy: juggler_handover::switch_back::suggestion_copy(
                    &original_account.label,
                    &away_label,
                ),
                away_account: away_label,
            });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// AJ-E208: pack redaction policy — global choice + per-Provider
// override (D-14). Always the user's remembered choice; unset BLOCKS
// pack generation (no code path applies an unchosen default).

/// Settings KV keys (schema-less strings — persisted across
/// restarts; round-trip pinned by test).
const REDACTION_GLOBAL_KEY: &str = "redaction_global";
const REDACTION_OVERRIDES_KEY: &str = "redaction_overrides";

/// The resolved policy a switch surface shows (the confirm line is
/// verbatim per Req.2).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct RedactionPolicyInfo {
    /// The user's global choice wire name (`strip|keep|ask`).
    pub global: Option<String>,
    /// This Provider's override wire name, when set.
    pub r#override: Option<String>,
    /// override-if-set else global; `null` = unset (the first pack
    /// is blocked until the user chooses).
    pub effective: Option<String>,
    /// `global` | `override` | `unset`.
    pub source: String,
    /// The verbatim confirm line ("Redaction for {Provider}: …").
    pub line: String,
}

impl RedactionPolicyInfo {
    /// Builds the info block (line verbatim per Req.2).
    #[must_use]
    pub fn from_parts(
        provider: &juggler_core::model::HarnessId,
        global: Option<juggler_handover::redaction::RedactionPolicy>,
        r#override: Option<juggler_handover::redaction::RedactionPolicy>,
    ) -> Self {
        let effective = juggler_handover::redaction::resolve_policy(global, r#override);
        Self {
            global: global.map(|policy| policy.as_str().to_owned()),
            r#override: r#override.map(|policy| policy.as_str().to_owned()),
            effective: effective.map(|policy| policy.as_str().to_owned()),
            source: match (&global, &r#override) {
                (_, Some(_)) => "override".to_owned(),
                (Some(_), None) => "global".to_owned(),
                (None, None) => "unset".to_owned(),
            },
            line: redaction_line(provider.as_str(), global, r#override),
        }
    }
}

/// The stored global choice.
fn redaction_global_of(storage: &Storage) -> Option<juggler_handover::redaction::RedactionPolicy> {
    storage
        .app_settings()
        .get(REDACTION_GLOBAL_KEY)
        .ok()
        .flatten()
        .and_then(|wire| juggler_handover::redaction::RedactionPolicy::from_wire(&wire))
}

/// The stored override for one Provider.
fn redaction_override_of(
    storage: &Storage,
    provider: &str,
) -> Option<juggler_handover::redaction::RedactionPolicy> {
    let overrides: std::collections::BTreeMap<String, String> = storage
        .app_settings()
        .get(REDACTION_OVERRIDES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    overrides
        .get(provider)
        .and_then(|wire| juggler_handover::redaction::RedactionPolicy::from_wire(wire))
}

/// Builds the confirm line verbatim (Req.2) — "(your global
/// choice)" vs "(override)"; empty when unset (the UI shows the
/// first-run chooser instead).
#[must_use]
fn redaction_line(
    provider: &str,
    global: Option<juggler_handover::redaction::RedactionPolicy>,
    r#override: Option<juggler_handover::redaction::RedactionPolicy>,
) -> String {
    let effective = juggler_handover::redaction::resolve_policy(global, r#override);
    let Some(policy) = effective else {
        return String::new();
    };
    let source = if r#override.is_some() {
        "override"
    } else {
        "your global choice"
    };
    format!(
        "Redaction for {provider}: {} ({source})",
        strip_label(policy)
    )
}

/// The Strip label words the picker uses ("replace keys with •••");
/// Keep/Ask use their own plain words.
fn strip_label(policy: juggler_handover::redaction::RedactionPolicy) -> String {
    match policy {
        juggler_handover::redaction::RedactionPolicy::Strip => {
            "Strip — replace keys with •••".to_owned()
        }
        juggler_handover::redaction::RedactionPolicy::Keep => {
            "Keep — pack stays on this Mac".to_owned()
        }
        juggler_handover::redaction::RedactionPolicy::AskEachTime => "Ask each time".to_owned(),
    }
}

/// AJ-E208: the redaction policy for one Provider (settings surface
/// + the switch confirm line).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
#[tauri::command]
pub fn redaction_policy_get(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
) -> AppResult<RedactionPolicyInfo> {
    redaction_policy_get_impl(state.inner(), &provider)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
pub fn redaction_policy_get_impl(
    state: &Arc<AppState>,
    provider: &str,
) -> AppResult<RedactionPolicyInfo> {
    let storage = lock_storage_arc(state)?;
    let global = redaction_global_of(&storage);
    let r#override = redaction_override_of(&storage, provider);
    let effective = juggler_handover::redaction::resolve_policy(global, r#override);
    Ok(RedactionPolicyInfo {
        global: global.map(|policy| policy.as_str().to_owned()),
        r#override: r#override.map(|policy| policy.as_str().to_owned()),
        effective: effective.map(|policy| policy.as_str().to_owned()),
        source: match (&global, &r#override) {
            (_, Some(_)) => "override".to_owned(),
            (Some(_), None) => "global".to_owned(),
            (None, None) => "unset".to_owned(),
        },
        line: redaction_line(provider, global, r#override),
    })
}

/// AJ-E208: sets (or clears) the GLOBAL policy — Settings →
/// Privacy. `None` clears (back to unset — the first pack blocks).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage writes fail.
#[tauri::command]
pub fn redaction_policy_set_global(
    state: tauri::State<'_, Arc<AppState>>,
    policy: Option<String>,
) -> AppResult<()> {
    redaction_policy_set_global_impl(state.inner(), policy.as_deref())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on an unknown wire name or a
/// storage write failure (an unknown name is refused, never
/// guessed).
pub fn redaction_policy_set_global_impl(
    state: &Arc<AppState>,
    policy: Option<&str>,
) -> AppResult<()> {
    let parsed = policy.and_then(juggler_handover::redaction::RedactionPolicy::from_wire);
    if policy.is_some() && parsed.is_none() {
        return Err(AppError::Io {
            context: "redaction policy",
            message: format!("Unknown policy {policy:?} — strip, keep, or ask"),
        });
    }
    let storage = lock_storage_arc(state)?;
    let settings = storage.app_settings();
    match parsed {
        Some(parsed) => {
            settings.set(REDACTION_GLOBAL_KEY, parsed.as_str())?;
        }
        None => {
            settings.set(REDACTION_GLOBAL_KEY, "")?;
        }
    }
    Ok(())
}

/// AJ-E208: sets (or clears) one Provider's override — Provider
/// settings.
///
/// # Errors
///
/// Returns the typed [`AppError`] on an unknown wire name or a
/// storage write failure.
#[tauri::command]
pub fn redaction_policy_set_override(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    policy: Option<String>,
) -> AppResult<()> {
    redaction_policy_set_override_impl(state.inner(), &provider, policy.as_deref())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn redaction_policy_set_override_impl(
    state: &Arc<AppState>,
    provider: &str,
    policy: Option<&str>,
) -> AppResult<()> {
    let parsed = policy.and_then(juggler_handover::redaction::RedactionPolicy::from_wire);
    if policy.is_some() && parsed.is_none() {
        return Err(AppError::Io {
            context: "redaction policy",
            message: format!("Unknown policy {policy:?} — strip, keep, or ask"),
        });
    }
    let storage = lock_storage_arc(state)?;
    let settings = storage.app_settings();
    let mut overrides: std::collections::BTreeMap<String, String> = settings
        .get(REDACTION_OVERRIDES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    match parsed {
        Some(parsed) => {
            overrides.insert(provider.to_owned(), parsed.as_str().to_owned());
        }
        None => {
            overrides.remove(provider);
        }
    }
    let blob = serde_json::to_string(&overrides).map_err(|error| AppError::Io {
        context: "redaction policy",
        message: error.to_string(),
    })?;
    settings.set(REDACTION_OVERRIDES_KEY, &blob)?;
    Ok(())
}

/// One listed secret for the preview UI (masked — values never
/// cross the bridge).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct RedactionItemInfo {
    /// Detection kind (`sk-key`, `bearer`, `env-value`,
    /// `private-key`, `zcode-rollout`).
    pub kind: String,
    /// Masked preview (first char + `•••` + last 4).
    pub preview: String,
    /// Segment ordinal (conversation turns first).
    pub segment: u32,
}

/// Lists the source conversation's secrets (masked) for the
/// preview; the un-redact indices the user picks address these.
fn redaction_items_of(source: &juggler_core::model::InstanceRecord) -> Vec<RedactionItemInfo> {
    let Some(reference) = source.transcript_ref.as_ref() else {
        return Vec::new();
    };
    let Some((_uuids, turns)) = transcript_side(&reference.path) else {
        return Vec::new();
    };
    let segments: Vec<String> = turns.iter().map(|turn| turn.text.clone()).collect();
    let refs: Vec<&str> = segments.iter().map(String::as_str).collect();
    juggler_handover::redaction::list_segments(&refs)
        .into_iter()
        .map(|listed| RedactionItemInfo {
            kind: listed.kind.to_owned(),
            preview: listed.preview,
            segment: u32::try_from(listed.segment).unwrap_or(u32::MAX),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// AJ-E209: the badged switch entry point, the Switch Graph, and the
// per-node actions (Resume / Export pack) — ranking suggests only.

/// One ranked, badged switch destination (the handover section's
/// rows). Suggest-only: the click is the user's.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct SwitchTargetInfo {
    /// The launch recipe the switch would run under.
    pub recipe_id: String,
    /// Its account id.
    pub account_id: String,
    /// Its account label.
    pub account_label: String,
    /// The ordered-pair badge wire name ("F1 · Beta" — never
    /// "lossless" below F4; cross-Provider always carries Beta).
    pub badge: String,
    /// Validated state (false until E210's fixtures flip cells).
    pub validated: bool,
    /// Cross-Provider pair (badge + Drop list are mandatory).
    pub cross_provider: bool,
    /// The pair's Drop list (the badge's honest companion).
    pub drops: Vec<String>,
    /// The §Q6 score (`None` = hard-filtered, reason alongside).
    pub score: Option<f64>,
    /// Hard-filter reason, when filtered.
    pub filtered_reason: Option<String>,
    /// Top-2 contributing factors in words.
    pub top_factors: Vec<String>,
    /// All inputs for the "Why this order?" expander.
    pub factors: Vec<(String, f64)>,
    /// Confidence badge (Measured/Computed/Estimate).
    pub confidence: String,
    /// Minutes-to-exhaustion, when known.
    pub tte_min: Option<f64>,
    /// The top survivor (suggest-only).
    pub suggested: bool,
}

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
    let source_replay_tokens = source.transcript_ref.as_ref().and_then(|reference| {
        std::fs::read(&reference.path).ok().map(|bytes| {
            let parsed = juggler_adapters::claude_transcript::parse_transcript(&bytes);
            parsed.usage.input.saturating_add(parsed.usage.output)
        })
    });
    Ok(SwitchRankContext {
        source_provider: source_account.harness.to_string(),
        source_tte_min,
        returnable_accounts: returnable,
        source_replay_tokens,
    })
}

/// AJ-E209: the ranked + badged destinations for one source session
/// (§Q6 with the switch extensions; D-04/D-11 badges; suggest-only).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the source session or ranking
/// inputs cannot be resolved.
#[tauri::command]
pub fn switch_targets(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Vec<SwitchTargetInfo>> {
    switch_targets_impl(state.inner(), &session_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
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
    let storage = lock_storage_arc(state)?;
    let dogfood = dogfood_cells_of(&storage);
    let mut out = Vec::new();
    for account in storage.accounts().list()? {
        let Some(row) = by_account.get(account.id.as_str()) else {
            continue;
        };
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
            // (a passing cell validates only real-CI rows — D-04).
            let cell = dogfood.get(&dogfood_key(
                context.source_provider.as_str(),
                account.harness.as_str(),
            ));
            let matrix_row = juggler_handover::validation::pair_cell(
                context.source_provider.as_str(),
                account.harness.as_str(),
            )
            .map(|row| match cell {
                Some(record) => juggler_handover::validation::with_dogfood(row, record.clone()),
                None => row,
            });
            out.push(SwitchTargetInfo {
                recipe_id: recipe.id.to_string(),
                account_id: account.id.to_string(),
                account_label: account.label.clone(),
                badge: juggler_handover::validation::advertised_badge(
                    context.source_provider.as_str(),
                    account.harness.as_str(),
                    cell,
                ),
                validated: matrix_row
                    .as_ref()
                    .is_some_and(juggler_handover::validation::PairCell::validated),
                cross_provider: cross,
                drops: carry.dropped.clone(),
                score: row.score,
                filtered_reason: row.filtered_reason.clone(),
                top_factors: row.top_factors.clone(),
                factors: row.factors.clone(),
                confidence: row.confidence.clone(),
                tte_min: row.tte_min,
                suggested: row.suggested,
            });
        }
    }
    Ok(out)
}

/// One epoch node on the timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct EpochNodeInfo {
    /// Epoch id.
    pub id: String,
    /// Provider id (the icon).
    pub provider: String,
    /// Account label.
    pub account_label: String,
    /// Instance-kind badge.
    pub kind: String,
    /// A native session exists AND its transcript survives (the
    /// Resume action's honest gate).
    pub resumable: bool,
    /// The live session id, when one is attached.
    pub session_id: Option<String>,
}

/// One labelled edge on the timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct EdgeInfo {
    /// From epoch id.
    pub from: String,
    /// To epoch id (None until launched).
    pub to: Option<String>,
    /// The arrow's label ("transplant F1 · 12:04" / "replay F1 ·
    /// ≈41k tok" / "switch-back ↺").
    pub label: String,
    /// The §9 reason, when the edge failed (dashed red + recovery).
    pub failed_reason: Option<String>,
    /// Switch-back edge (renders ↺).
    pub switch_back: bool,
}

/// One work item's timeline.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct WorkGraphInfo {
    /// Work title.
    pub title: String,
    /// Epochs newer than the last fork/merge, in order.
    pub epochs: Vec<EpochNodeInfo>,
    /// Their edges.
    pub edges: Vec<EdgeInfo>,
    /// Older epochs collapsed away ("+n switches" — never unbounded).
    pub collapsed: u32,
    /// The S-F9 hop line (inform, never block).
    pub hop_note: Option<String>,
}

/// AJ-E209 Req.4: the per-project Switch Graph — the timeline that
/// answers "how did this work travel?".
///
/// # Errors
///
/// Returns the typed [`AppError`] when the project or graph reads
/// fail.
#[tauri::command]
pub fn switch_graph_overview(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<Vec<WorkGraphInfo>> {
    switch_graph_overview_impl(state.inner(), &project_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_graph_overview_impl(
    state: &Arc<AppState>,
    project_id: &str,
) -> AppResult<Vec<WorkGraphInfo>> {
    graph_works_for(state, project_id)
}

/// The graph walk behind [`switch_graph_overview_impl`].
fn graph_works_for(state: &Arc<AppState>, project_id: &str) -> AppResult<Vec<WorkGraphInfo>> {
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let accounts: std::collections::BTreeMap<_, _> = storage
        .accounts()
        .list()?
        .into_iter()
        .map(|account| (account.id.clone(), account.label.clone()))
        .collect();
    let works =
        graph.works_for_project(&juggler_core::model::ProjectId::new(project_id.to_owned()))?;
    let mut out = Vec::new();
    for work in works {
        out.push(work_graph_info(&storage, &graph, &accounts, &work)?);
    }
    Ok(out)
}

/// Maps one work item to its timeline (collapse + nodes + edges +
/// the S-F9 hop note).
fn work_graph_info(
    storage: &juggler_core::storage::Storage,
    graph: &juggler_core::storage::SwitchGraphRepo<'_>,
    accounts: &std::collections::BTreeMap<juggler_core::model::AccountId, String>,
    work: &juggler_core::model::WorkItem,
) -> AppResult<WorkGraphInfo> {
    let epochs = graph.epochs_for_work(&work.id)?;
    let edges = graph.edges_for_work(&work.id)?;
    let has_branch = epochs.iter().any(|epoch| {
        epochs
            .iter()
            .filter(|other| other.resumed_from_epoch == Some(epoch.id.clone()))
            .count()
            > 1
    });
    // Collapse epochs older than the last fork/merge point.
    let cut = if has_branch {
        epochs
            .iter()
            .filter(|epoch| {
                epochs
                    .iter()
                    .filter(|other| other.resumed_from_epoch == Some(epoch.id.clone()))
                    .count()
                    > 1
            })
            .rev()
            .map(|epoch| epoch.id.clone())
            .next()
    } else {
        None
    };
    let keep_from = cut.or_else(|| epochs.first().map(|epoch| epoch.id.clone()));
    let visible_start = keep_from
        .and_then(|id| epochs.iter().position(|epoch| epoch.id == id))
        .unwrap_or(0);
    let collapsed = u32::try_from(visible_start).unwrap_or_default();
    let mut nodes = Vec::new();
    for epoch in &epochs[visible_start.min(epochs.len())..] {
        let resumable = epoch.native_id.as_deref().is_some_and(|id| !id.is_empty())
            && epoch
                .session_ref
                .as_ref()
                .and_then(|session| storage.sessions().get(session).ok())
                .and_then(|record| {
                    record
                        .transcript_ref
                        .as_ref()
                        .map(|reference| reference.path.is_file())
                })
                .unwrap_or(false);
        nodes.push(EpochNodeInfo {
            id: epoch.id.to_string(),
            provider: epoch.provider.to_string(),
            account_label: accounts
                .get(&epoch.account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_owned()),
            kind: "terminal".to_owned(),
            resumable,
            session_id: epoch.session_ref.as_ref().map(ToString::to_string),
        });
    }
    let (wire_edges, hops, tokens) = graph_edge_summary(&edges);

    Ok(WorkGraphInfo {
        title: work.title.clone(),
        epochs: nodes,
        edges: wire_edges,
        collapsed,
        hop_note: juggler_handover::ranking::hop_note(hops, tokens),
    })
}

/// Wires one work's edges for the timeline: labels (mechanism +
/// fidelity shown, switch-back ↺, failed reason) and the hop/token
/// counts feeding the S-F9 note.
fn graph_edge_summary(edges: &[juggler_core::model::SwitchEdge]) -> (Vec<EdgeInfo>, u32, u64) {
    use juggler_core::model::EdgeState;
    let mut wire_edges = Vec::new();
    for edge in edges {
        let back = edge.parent_edge_a.is_some();
        let label = match &edge.state {
            EdgeState::Failed { reason, .. } => format!("failed · {reason}"),
            _ if back => format!(
                "switch-back ↺ · {}",
                edge.fidelity_shown.as_deref().unwrap_or("F4")
            ),
            _ => {
                let mech = match edge.mechanism {
                    juggler_core::model::SwitchMechanism::NativeResume => "native-resume",
                    juggler_core::model::SwitchMechanism::Transplant => "transplant",
                    juggler_core::model::SwitchMechanism::Replay => "replay",
                    juggler_core::model::SwitchMechanism::Pack => "pack",
                };
                format!("{mech} {}", edge.fidelity_shown.as_deref().unwrap_or("F0"))
            }
        };
        wire_edges.push(EdgeInfo {
            from: edge.from_epoch.to_string(),
            to: edge.to_epoch.as_ref().map(ToString::to_string),
            label,
            failed_reason: match &edge.state {
                EdgeState::Failed { reason, .. } => Some(reason.clone()),
                _ => None,
            },
            switch_back: back,
        });
    }
    let hops = u32::try_from(
        edges
            .iter()
            .filter(|edge| matches!(edge.state, EdgeState::Confirmed))
            .count(),
    )
    .unwrap_or_default();
    let tokens = edges
        .iter()
        .filter_map(|edge| edge.manifest_sha256.as_ref().map(|_| 0_u64))
        .sum::<u64>();
    (wire_edges, hops, tokens)
}

/// Finds one epoch (any project) with its session row + account —
/// the graph actions' shared read side.
fn find_epoch_with_session(
    state: &Arc<AppState>,
    epoch_id: &str,
) -> AppResult<(
    juggler_core::model::Epoch,
    juggler_core::model::InstanceRecord,
    juggler_core::model::Account,
)> {
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let wanted = juggler_core::model::EpochId::new(epoch_id.to_owned());
    let mut found = None;
    for project in storage.projects().list()? {
        for work in graph.works_for_project(&project.id)? {
            if let Some(epoch) = graph
                .epochs_for_work(&work.id)?
                .into_iter()
                .find(|row| row.id == wanted)
            {
                found = Some(epoch);
            }
        }
    }
    let epoch = found.ok_or_else(|| AppError::Io {
        context: "graph action",
        message: "That epoch isn't in the graph".to_owned(),
    })?;
    let session = epoch
        .session_ref
        .as_ref()
        .and_then(|id| storage.sessions().get(id).ok())
        .ok_or_else(|| AppError::Io {
            context: "graph action",
            message: "The epoch has no session row".to_owned(),
        })?;
    let account = storage
        .accounts()
        .get(&epoch.account)
        .map_err(|error| not_found_or_core(error, "account", &epoch.account.to_string()))?;
    Ok((epoch, session, account))
}

/// AJ-E209 Req.5: resumes an epoch's native session (the graph's
/// Resume action — suggest-chosen, the user's click).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the epoch or its account/
/// recipe cannot be resolved or the spawn fails.
#[tauri::command]
pub fn epoch_resume(
    state: tauri::State<'_, Arc<AppState>>,
    epoch_id: String,
) -> AppResult<LaunchResult> {
    epoch_resume_impl(state.inner(), &epoch_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn epoch_resume_impl(state: &Arc<AppState>, epoch_id: &str) -> AppResult<LaunchResult> {
    let (epoch, session_row, account) = find_epoch_with_session(state, epoch_id)?;
    let native_id = epoch
        .native_id
        .clone()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::Io {
            context: "resume epoch",
            message: "The epoch has no native session to resume".to_owned(),
        })?;
    let profile = {
        let storage = lock_storage_arc(state)?;
        storage
            .profiles()
            .get(&session_row.profile)
            .map_err(|error| {
                not_found_or_core(error, "profile", &session_row.profile.to_string())
            })?
    };
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let spec = adapter.launch_spec(
        &account,
        &profile,
        &session_row.cwd,
        &LaunchMode::Resume {
            session_id: native_id.clone(),
            delta_prompt: None,
        },
    )?;
    let spawned = state.supervisor.spawn(
        &new_session,
        &spec,
        &ledger_ctx(
            state,
            &account,
            session_row.project.as_ref().map(ToString::to_string),
        ),
    )?;
    {
        let machine = local_machine_id(state);
        let storage = lock_storage_arc(state)?;
        let usage = engine_usage_snapshot(&storage, &profile.id);
        storage
            .sessions()
            .insert(&juggler_core::model::InstanceRecord {
                id: new_session.clone(),
                conversation: Some(juggler_core::model::ConversationId::new(native_id)),
                profile: profile.id.clone(),
                machine,
                kind: juggler_core::model::InstanceKind::Terminal,
                project: session_row.project.clone(),
                cwd: session_row.cwd.clone(),
                status: SessionStatus::Starting,
                started_at: Timestamp::now(),
                last_activity: None,
                usage,
                attention: None,
                running: None,
                window: None,
                transcript_ref: session_row.transcript_ref.clone(),
            })?;
    }
    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid: spawned.pid,
    })
}

/// AJ-E209 Req.5: exports an epoch's pack (the graph's Export action)
/// — conversation turns rendered through the AJ-E208 redaction
/// policy (the GLOBAL choice — a standalone export has no
/// destination Provider), written under `<app-data>/exports/`.
/// Returns the pack directory for the toast.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the epoch cannot be resolved,
/// the policy is unset (exports block like switches), or the write
/// fails.
#[tauri::command]
pub fn pack_export(state: tauri::State<'_, Arc<AppState>>, epoch_id: String) -> AppResult<String> {
    pack_export_impl(state.inner(), &epoch_id)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn pack_export_impl(state: &Arc<AppState>, epoch_id: &str) -> AppResult<String> {
    use juggler_handover::pack::{ConversationMode, IrTurn, PackInput, PackSource};
    use juggler_handover::redaction::{RedactionApply, RedactionPolicy, redacted_pack};

    let (epoch, session_row, account) = find_epoch_with_session(state, epoch_id)?;
    // The GLOBAL redaction policy governs standalone exports
    // (unset blocks, exactly like switches — no default is applied).
    let policy = {
        let storage = lock_storage_arc(state)?;
        juggler_handover::redaction::resolve_policy(redaction_global_of(&storage), None)
            .ok_or_else(|| AppError::Io {
                context: "export pack",
                message: "Choose how secrets are handled in packs first (Accounts → Pack secrets)"
                    .to_owned(),
            })?
    };
    let reference = session_row
        .transcript_ref
        .clone()
        .ok_or_else(|| AppError::Io {
            context: "export pack",
            message: "The epoch has no transcript to pack".to_owned(),
        })?;
    let turns: Vec<IrTurn> = transcript_side(&reference.path)
        .map(|(_, turns)| turns)
        .unwrap_or_default();
    let input = PackInput {
        pack_id: format!("pack-{}", unique_suffix()),
        created_at_ms: Timestamp::now().millis(),
        work_id: epoch.work.to_string(),
        edge_id: None,
        source: PackSource {
            provider: account.harness.to_string(),
            account_label: account.label.clone(),
            profile_name: None,
            native_session_id: epoch.native_id.clone().unwrap_or_default(),
            harness_version: None,
            epoch_id: epoch.id.to_string(),
        },
        target_hint: None,
        project: juggler_handover::pack::PackProject {
            root: session_row.cwd.to_string_lossy().into_owned(),
            project_id: session_row
                .project
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        },
        git: juggler_handover::pack::PackGit {
            branch: "unknown".to_owned(),
            head_sha: None,
            dirty: false,
            diff_stat: None,
            worktree_path: None,
        },
        turns,
        compacted: false,
        artifacts: juggler_handover::pack::PackArtifacts::default(),
        fidelity: juggler_handover::pack::PackFidelity {
            level: "F1".to_owned(),
            score: 0.5,
            caps_applied: vec!["unvalidated-pair".to_owned()],
        },
        quota_at_creation: None,
        lineage: juggler_handover::pack::PackLineage::default(),
        staged_files: Vec::new(),
    };
    let apply = RedactionApply {
        strip: policy == RedactionPolicy::Strip,
        keep: std::collections::BTreeSet::new(),
    };
    let pack = redacted_pack(&input, ConversationMode::Distilled { last_n: 12 }, &apply);
    let dir = state
        .data_dir
        .join("exports")
        .join(format!("pack-{}", unique_suffix()));
    std::fs::create_dir_all(&dir).map_err(|error| AppError::Io {
        context: "export pack",
        message: format!("could not create {}: {error}", dir.display()),
    })?;
    juggler_handover::pack::write_pack(&dir, &pack).map_err(|error| AppError::Io {
        context: "export pack",
        message: error.to_string(),
    })?;
    Ok(dir.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// AJ-E210: the ordered-pair validation matrix (D-04) — the source of
// truth that graduates pairs out of "Beta". Read side renders in
// Settings → Beta; the record side stores HUMAN-run live dogfood
// cells (the app never runs vendor binaries itself).

/// KV key for the recorded live dogfood cells (JSON map
/// "from→to" → [`juggler_handover::validation::DogfoodCell`]).
const DOGFOOD_CELLS_KEY: &str = "dogfood_cells";

/// The map key for one pair's recorded cell.
fn dogfood_key(from: &str, to: &str) -> String {
    format!("{from}\u{2192}{to}")
}

/// The recorded live dogfood cells (empty until a human records one).
fn dogfood_cells_of(
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

/// One Settings → Beta matrix row (the same data the badges derive
/// from — one source, no drift).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct ValidationCellInfo {
    /// Source provider id.
    pub from: String,
    /// Target provider id.
    pub to: String,
    /// The switch mechanism at the ceiling (transplant/replay/pack-only).
    pub mechanism: String,
    /// The researched ceiling (E203), wire name.
    pub researched_ceiling: String,
    /// The advertised ceiling after validation gating (a FAILED live
    /// cell lowers it one level until fixed).
    pub advertised: String,
    /// The wire badge ("F1 · Beta" / "F3 · Validated").
    pub badge: String,
    /// Validated = real-CI row green AND a passing live cell.
    pub validated: bool,
    /// The pair's shape is proven over the fake harness in CI.
    pub mechanism_green: bool,
    /// Both sides' REAL adapters have green fixture rows.
    pub ci_green: bool,
    /// The recorded live cell, when one exists (dates + verdict +
    /// hashes — never secrets).
    pub dogfood: Option<DogfoodCellInfo>,
    /// One-line status ("CI fixtures green · live dogfood pending —
    /// open beta debt").
    pub status: String,
    /// Honest notes (blocks, pending adapters), verbatim.
    pub notes: Vec<String>,
}

/// The recorded live cell as the UI reads it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct DogfoodCellInfo {
    /// ISO date the cell ran.
    pub at: String,
    /// Project shape.
    pub project_shape: String,
    /// pass | fail.
    pub verdict: String,
    /// sha256 of the source transcript before the switch.
    pub source_sha256: String,
    /// sha256 of the target transcript after the resume.
    pub target_sha256: String,
    /// The continuity note (pass) or the failure reason (fail).
    pub note: String,
}

/// AJ-E210: the validation matrix (static researched rows merged with
/// any recorded live cells). Read-only.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
#[tauri::command]
pub fn validation_matrix(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ValidationCellInfo>> {
    validation_matrix_impl(state.inner())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn validation_matrix_impl(state: &Arc<AppState>) -> AppResult<Vec<ValidationCellInfo>> {
    let storage = lock_storage_arc(state)?;
    let recorded = dogfood_cells_of(&storage);
    Ok(juggler_handover::validation::validation_matrix()
        .into_iter()
        .map(|row| {
            let merged = match recorded.get(&dogfood_key(&row.from, &row.to)) {
                Some(cell) => juggler_handover::validation::with_dogfood(row.clone(), cell.clone()),
                None => row.clone(),
            };
            ValidationCellInfo {
                from: row.from.clone(),
                to: row.to.clone(),
                mechanism: match row.mechanism {
                    juggler_handover::validation::PairMechanism::Transplant => "transplant",
                    juggler_handover::validation::PairMechanism::Replay => "replay",
                    juggler_handover::validation::PairMechanism::PackOnly => "pack-only",
                }
                .to_owned(),
                researched_ceiling: row.researched_ceiling.as_str().to_owned(),
                advertised: merged.advertised_ceiling().as_str().to_owned(),
                badge: merged.badge(),
                validated: merged.validated(),
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

/// AJ-E210: records a live dogfood cell for one ordered pair — a
/// HUMAN ran the pair on real binaries and reports the outcome (the
/// app never launches vendor binaries for validation). Recording a
/// PASS on a real-CI-green pair flips its badge to "· Validated"; a
/// FAIL lowers the advertised ceiling one level until fixed.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the pair is not in the
/// researched matrix (never invented) or the store fails.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn validation_dogfood_record(
    state: tauri::State<'_, Arc<AppState>>,
    from: String,
    to: String,
    verdict: String,
    at: String,
    project_shape: String,
    source_sha256: String,
    target_sha256: String,
    note: String,
) -> AppResult<()> {
    validation_dogfood_record_impl(
        state.inner(),
        &from,
        &to,
        &verdict,
        &at,
        &project_shape,
        &source_sha256,
        &target_sha256,
        &note,
    )
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
#[allow(clippy::too_many_arguments)]
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
            context: "record dogfood cell",
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
                context: "record dogfood cell",
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
        context: "record dogfood cell",
        message: error.to_string(),
    })?;
    storage
        .app_settings()
        .set(DOGFOOD_CELLS_KEY, &blob)
        .map_err(AppError::Core)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// AJ-E301: the wizard's Step-0 install gate — one row per Provider
// from the adapters' own `discover()`, probed in PARALLEL off the UI
// thread; a missing binary is caught before any account is created.

/// Vendor display metadata for one Provider (names + install pages;
/// no binary knowledge — that is `discover()`'s job).
fn provider_display(provider: &str) -> (String, &'static str) {
    let name = match provider {
        "claude-code" => "Claude Code".to_owned(),
        "codex" => "Codex".to_owned(),
        "zcode" => "ZCode".to_owned(),
        "fake" | "fake-desktop" => "Fake harness (test)".to_owned(),
        other => other.to_owned(),
    };
    let url = match provider {
        "claude-code" => "https://claude.com/download",
        "codex" => "https://developers.openai.com/codex",
        "zcode" => "https://z.ai",
        _ => "about:blank",
    };
    (name, url)
}

/// One Step-0 row (AJ-E301 Req.2): `Installed vX.Y` / `Not installed`
/// / `Installed (vX, unverified)`.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct ProviderInstallInfo {
    /// Provider id (the wizard's `provider` parameter).
    pub provider: String,
    /// Display name ("Claude Code").
    pub display_name: String,
    /// Vendor install page.
    pub install_url: String,
    /// `installed` | `not_installed` | `unverified` | `probe_failed`.
    pub status: String,
    /// The reported version, when a binary was found.
    pub version: Option<String>,
    /// Resolved binary path, when found.
    pub binary: Option<String>,
    /// Discovery notes (drift details), verbatim.
    pub notes: Vec<String>,
    /// AJ-E306 flow (c): the verified version range(s) — the card's
    /// badge; empty renders "unverified" honestly.
    pub verified_against: Vec<String>,
    /// `terminal` or `desktop` (the kind badge, Req.3).
    pub kind: String,
    /// The single-instance desktop limit, stated BEFORE selection
    /// (Req.3) — the adapter's isolation claim verbatim.
    pub single_instance_note: Option<String>,
}

/// AJ-E301: probes every registered Provider in parallel (thread per
/// adapter — `--version` subprocesses never touch the UI thread; the
/// 2s-per-provider hard cap is the subprocess timeout inside each
/// adapter's `discover()`).
///
/// # Errors
///
/// Returns the typed [`AppError`] on registry failures (never for a
/// missing binary — that is a ROW, not an error).
#[tauri::command]
pub fn providers_discover(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ProviderInstallInfo>> {
    providers_discover_impl(state.inner())
}

/// Core implementation shared with integration tests (the no-binary
/// Step-0 fixture).
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn providers_discover_impl(state: &Arc<AppState>) -> AppResult<Vec<ProviderInstallInfo>> {
    // Tauri dispatches commands off the UI thread; the per-adapter
    // probes additionally run on their OWN threads (parallel, one
    // subprocess each — the ≤800 ms p95 budget for 3 Providers).
    {
        let registry = &state.registry;
        let ids = registry.ids();
        let reports: Vec<juggler_adapter::discovery::DiscoveryReport> =
            std::thread::scope(|scope| {
                let handles: Vec<_> = ids
                    .iter()
                    .map(|id| {
                        let adapter = registry.get(id);
                        scope.spawn(move || {
                            adapter.map_or_else(
                                || juggler_adapter::discovery::DiscoveryReport {
                                    adapter: id.clone(),
                                    install: juggler_adapter::discovery::InstallStatus::NotFound,
                                    verification:
                                        juggler_adapter::discovery::VerificationStatus::Unverified,
                                    binary: None,
                                    notes: Vec::new(),
                                },
                                |adapter| adapter.discover(),
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .filter_map(|handle| handle.join().ok())
                    .collect()
            });
        Ok(reports
            .into_iter()
            .map(|report| {
                let (display_name, install_url) = provider_display(report.adapter.as_str());
                let (status, version) = match &report.install {
                    juggler_adapter::discovery::InstallStatus::Installed { version } => {
                        let verified = matches!(
                            report.verification,
                            juggler_adapter::discovery::VerificationStatus::Verified
                        );
                        (
                            if verified { "installed" } else { "unverified" },
                            Some(version.clone()),
                        )
                    }
                    juggler_adapter::discovery::InstallStatus::NotFound => ("not_installed", None),
                    juggler_adapter::discovery::InstallStatus::Failed { .. } => {
                        ("probe_failed", None)
                    }
                };
                let adapter_meta = registry.get(&juggler_core::model::HarnessId::new(
                    report.adapter.as_str(),
                ));
                let (verified_against, kind, single_instance_note) = match adapter_meta {
                    Some(adapter) => (
                        adapter
                            .verified_against()
                            .iter()
                            .map(|v| (*v).to_owned())
                            .collect(),
                        if adapter.desktop_isolation_label().is_some() {
                            "desktop".to_owned()
                        } else {
                            "terminal".to_owned()
                        },
                        adapter.desktop_isolation_label().map(str::to_owned),
                    ),
                    None => (Vec::new(), "terminal".to_owned(), None),
                };
                ProviderInstallInfo {
                    provider: report.adapter.to_string(),
                    display_name: display_name.clone(),
                    install_url: install_url.to_owned(),
                    status: status.to_owned(),
                    version,
                    binary: report
                        .binary
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned()),
                    notes: report.notes,
                    verified_against,
                    kind,
                    single_instance_note,
                }
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// AJ-E302: the 10-signal scan (pass2/03 §3 S1–S10) — parallel probes
// off the UI thread, per-probe health, progress events, and a cancel
// flag. Every probe is metadata-only; the guardrail test pins that.

use std::sync::atomic::{AtomicBool, Ordering};

/// The scan's cancellation flag (one scan at a time in practice; the
/// wizard sets it on close/unmount).
static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

/// One probe's result as the UI reads it (Req.4 — health rides along).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct ProbeInfo {
    /// Probe id (`s1-config-dirs` … `s10-desktop`).
    pub id: String,
    /// Human label ("Config directories").
    pub label: String,
    /// `ok` | `partial` | `denied` | `cancelled`.
    pub health: String,
    /// The reason, when not ok.
    pub reason: Option<String>,
    /// Metadata-only detail (counts, names).
    pub detail: String,
}

/// The widened scan report (AJ-E302): the candidate installs plus the
/// per-probe health, project seeds (S9), and desktop apps (S10).
#[derive(Clone, Debug, serde::Serialize)]
pub struct ScanReportInfo {
    /// Candidate setups (S1/S2/S3/S5 origins).
    pub installs: Vec<DiscoveredInfo>,
    /// All ten probes with health (Req.4).
    pub probes: Vec<ProbeInfo>,
    /// Project seed names, dirs only (S9).
    pub seeds: Vec<String>,
    /// Desktop apps present, names only (S10).
    pub desktop_apps: Vec<String>,
    /// True when the scan was cancelled partway (the UI says so —
    /// never "no setups found").
    pub cancelled: bool,
}

/// The ten probe labels (progress: "n of 10 checks").
const PROBE_LABELS: [(&str, &str); 10] = [
    ("s1-config-dirs", "Config directories"),
    ("s2-env-overrides", "Environment overrides"),
    ("s3-manager-t3", "t3 profiles"),
    ("s4-manager-cc-switch", "cc-switch database"),
    ("s5-shell-aliases", "Shell aliases"),
    ("s6-installed-clis", "Installed CLIs"),
    ("s7-login-metadata", "Login metadata"),
    ("s8-usage-recency", "Usage recency"),
    ("s9-projects", "Projects"),
    ("s10-desktop", "Desktop apps"),
];

fn probe(id: &str, health: juggler_adapter::environment::ScanHealth, detail: String) -> ProbeInfo {
    ProbeInfo {
        id: id.to_owned(),
        label: PROBE_LABELS
            .iter()
            .find(|(pid, _)| *pid == id)
            .map_or_else(|| id.to_owned(), |(_, label)| (*label).to_owned()),
        health: match &health {
            juggler_adapter::environment::ScanHealth::Ok => "ok".to_owned(),
            juggler_adapter::environment::ScanHealth::Partial(_) => "partial".to_owned(),
            juggler_adapter::environment::ScanHealth::Denied(_) => "denied".to_owned(),
        },
        reason: health.reason().map(ToOwned::to_owned),
        detail,
    }
}

/// AJ-E302: the full 10-signal scan. Probes run in PARALLEL (thread
/// per probe group — metadata walks and `--version` subprocesses
/// never block the UI thread); progress streams as `n of 10` events;
/// the cancel flag stops between probe groups and the report carries
/// `cancelled` (an empty-but-cancelled scan never reads as "nothing
/// found").
///
/// # Errors
///
/// Returns the typed [`AppError`] on registry failures (a denied or
/// partial probe is a ROW, never an error).
#[tauri::command]
pub fn environment_scan_full(
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> AppResult<ScanReportInfo> {
    // Progress streams to the webview as `scan-progress` events
    // ("Checking… n of 10" — H1 visibility, WCAG-safe text).
    let sink = move |done: u32, total: u32| {
        let _ = tauri::Emitter::emit(
            &app,
            "scan-progress",
            serde_json::json!({ "done": done, "total": total }),
        );
    };
    environment_scan_full_impl(state.inner(), Some(&sink))
}

/// Core implementation shared with integration tests. `progress` is
/// the optional UI sink (done, total) — the command layer wires it to
/// Tauri events.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn environment_scan_full_impl(
    state: &Arc<AppState>,
    progress: Option<&dyn Fn(u32, u32)>,
) -> AppResult<ScanReportInfo> {
    SCAN_CANCEL.store(false, Ordering::SeqCst);
    let total = u32::try_from(PROBE_LABELS.len()).unwrap_or_default();
    let mut done = 0_u32;
    let tick = |done: &mut u32| {
        *done += 1;
        if let Some(sink) = progress {
            sink(*done, total);
        }
    };

    // The parallel walk feeds S1/S2/S3/S6/S7/S8; S5/S4/S9/S10 are the
    // folds below. Cancellation stops between groups.
    let (found_installs, alias_rows, managers, discoveries) = scan_walk(state);
    let cancelled = SCAN_CANCEL.load(Ordering::SeqCst);
    tick(&mut done); // S1 — the walk
    tick(&mut done); // S2 — env overrides (app env, cheap)
    tick(&mut done); // S7 — login metadata
    tick(&mut done); // S8 — usage recency
    tick(&mut done); // S3 — manager t3

    let mut installs: Vec<DiscoveredInfo> =
        found_installs.into_iter().map(discovered_info_of).collect();

    // S5: assignments whose target exists become installs (a
    // forgotten manual setup); dangling ones stay hints.
    let (alias_installs, _alias_health, dangling) = alias_installs_of(&alias_rows, &mut installs);
    installs.extend(alias_installs);
    tick(&mut done); // S5

    // S4: the cc-switch health rides its manager hint.
    let cc_health = managers
        .iter()
        .find(|manager| manager.manager == "cc-switch")
        .map_or_else(
            || juggler_adapter::environment::ScanHealth::Ok,
            |manager| manager.health.clone(),
        );
    tick(&mut done); // S4
    tick(&mut done); // S6 (the --version runs happened in the walk)

    let config_dirs: Vec<std::path::PathBuf> = installs
        .iter()
        .map(|install| std::path::PathBuf::from(&install.path))
        .collect();
    let seeds = juggler_adapters::scan_signals::project_seed_names(&config_dirs);
    tick(&mut done); // S9
    let desktop_apps =
        juggler_adapters::scan_signals::desktop_apps_present(std::path::Path::new("/Applications"));
    tick(&mut done); // S10

    let probes = scan_probe_rows(
        &installs,
        &alias_rows,
        dangling,
        &managers,
        cc_health,
        &discoveries,
        &seeds,
        &desktop_apps,
        cancelled,
    );

    Ok(ScanReportInfo {
        installs,
        probes,
        seeds,
        desktop_apps,
        cancelled,
    })
}

/// The parallel metadata walk: per adapter, the config-dir scan, the
/// rc assignments, the manager hints, and the `--version` probe all
/// run on their own threads (off the UI thread; ≤ one subprocess per
/// adapter).
/// The parallel walk's collected parts: installs, alias hints,
/// manager hints, and per-adapter CLI presence.
type ScanWalk = (
    Vec<juggler_adapter::environment::DiscoveredInstall>,
    Vec<juggler_adapter::environment::AliasHint>,
    Vec<juggler_adapter::environment::ManagerHint>,
    Vec<(String, bool)>,
);

fn scan_walk(state: &Arc<AppState>) -> ScanWalk {
    let mut found_installs = Vec::new();
    let mut alias_rows = Vec::new();
    let mut managers = Vec::new();
    let mut discoveries = Vec::new();
    let registry = &state.registry;
    let ids = registry.ids();
    std::thread::scope(|scope| {
        let handles: Vec<_> = ids
            .iter()
            .map(|id| {
                let adapter = registry.get(id);
                scope.spawn(move || {
                    let mut installs = Vec::new();
                    let mut aliases = Vec::new();
                    let mut manager_hints = Vec::new();
                    let mut discovery = None;
                    if let Some(adapter) = adapter {
                        installs = adapter.scan_environment();
                        aliases = adapter.scan_shell_aliases();
                        manager_hints = adapter.scan_managers();
                        discovery =
                            Some((adapter.id().to_string(), adapter.discover().is_installed()));
                    }
                    (installs, aliases, manager_hints, discovery)
                })
            })
            .collect();
        for handle in handles {
            if let Ok((installs, aliases, mgrs, discovery)) = handle.join() {
                found_installs.extend(installs);
                alias_rows.extend(aliases);
                managers.extend(mgrs);
                if let Some(entry) = discovery {
                    discoveries.push(entry);
                }
            }
        }
    });
    (found_installs, alias_rows, managers, discoveries)
}

/// The wire form of one discovered install.
fn discovered_info_of(found: juggler_adapter::environment::DiscoveredInstall) -> DiscoveredInfo {
    let credentials_len = std::fs::metadata(found.path.join(".credentials.json"))
        .ok()
        .map(|meta| meta.len());
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or_default();
    let idle_secs = found
        .last_used_secs
        .map(|last| now_secs.saturating_sub(last));
    DiscoveredInfo {
        inference: inference_info_of(
            &found.path,
            None,
            matches!(
                found.login,
                juggler_adapter::environment::LoginProbe::LoggedIn
            ),
            credentials_len,
            idle_secs,
            found.managed,
        ),
        signals: vec![found.origin.as_str().to_owned()],
        harness: found.harness.as_str().to_string(),
        path: found.path.to_string_lossy().into_owned(),
        origin: found.origin.as_str().to_string(),
        login: login_probe_name(found.login),
        bytes: found.bytes,
        last_used_secs: found.last_used_secs,
        managed: found.managed,
        notes: found.notes,
    }
}

/// The wire inference block for one candidate (the one extra read is
/// a credentials-file SIZE — metadata, never contents).
fn inference_info_of(
    path: &std::path::Path,
    manager_name: Option<&str>,
    probe_logged_in: bool,
    credentials_len: Option<u64>,
    idle_secs: Option<u64>,
    managed: bool,
) -> InferenceInfo {
    let inference =
        juggler_adapters::inference::infer_candidate(&juggler_adapters::inference::InferInput {
            path,
            manager_name,
            probe_logged_in,
            credentials_len,
            idle_secs,
            managed,
        });
    InferenceInfo {
        purpose: inference.purpose.value,
        purpose_confidence: inference.purpose.confidence.as_str().to_owned(),
        login: inference.login.value.as_str().to_owned(),
        login_confidence: inference.login.confidence.as_str().to_owned(),
        activity: inference.activity.value.as_str().to_owned(),
        activity_confidence: inference.activity.confidence.as_str().to_owned(),
        activity_words: inference.activity.value.words(),
        keychain_bound: inference.keychain_bound,
    }
}

/// S5 + the duplicate merge (Req.2): an alias whose target exists
/// MERGES into the known candidate as an extra signal (alias +
/// home-pattern → ONE candidate, signals listed); only a target no
/// other signal found becomes a new candidate. Dangling rows are
/// counted for the probe health.
pub fn alias_installs_of(
    alias_rows: &[juggler_adapter::environment::AliasHint],
    known: &mut [DiscoveredInfo],
) -> (
    Vec<DiscoveredInfo>,
    juggler_adapter::environment::ScanHealth,
    u32,
) {
    let mut out = Vec::new();
    let mut dangling = 0_u32;
    for hint in alias_rows {
        let path = hint.path.to_string_lossy().into_owned();
        let signal = format!("{} in {}:{}", hint.variable, hint.source_file, hint.line);
        if !hint.target_exists {
            dangling += 1;
            continue;
        }
        // The duplicate merge: an existing candidate for the same dir
        // gains the alias as a signal — never a second card.
        if let Some(existing) = known.iter_mut().find(|install| install.path == path) {
            if !existing.signals.contains(&signal) {
                existing.signals.push(signal);
            }
            continue;
        }
        let credentials_len = std::fs::metadata(hint.path.join(".credentials.json"))
            .ok()
            .map(|meta| meta.len());
        out.push(DiscoveredInfo {
            inference: inference_info_of(&hint.path, None, false, credentials_len, None, false),
            signals: vec!["shell-alias".to_owned()],
            harness: "claude-code".to_owned(),
            path,
            origin: "shell-alias".to_owned(),
            login: "logged_out".to_owned(),
            bytes: 0,
            last_used_secs: None,
            managed: false,
            notes: vec![format!(
                "Set by {} in your shell — {}:{}.",
                hint.variable, hint.source_file, hint.line
            )],
        });
    }
    let health = if dangling > 0 {
        juggler_adapter::environment::ScanHealth::Partial(format!(
            "{dangling} assignment(s) point at directories that no longer exist"
        ))
    } else {
        juggler_adapter::environment::ScanHealth::Ok
    };
    (out, health, dangling)
}

/// Builds the ten probe rows (S1–S10) with their health.
#[allow(clippy::too_many_arguments)]
fn scan_probe_rows(
    installs: &[DiscoveredInfo],
    alias_rows: &[juggler_adapter::environment::AliasHint],
    dangling: u32,
    managers: &[juggler_adapter::environment::ManagerHint],
    cc_health: juggler_adapter::environment::ScanHealth,
    discoveries: &[(String, bool)],
    seeds: &[String],
    desktop_apps: &[String],
    cancelled: bool,
) -> Vec<ProbeInfo> {
    let installs_health = if cancelled {
        juggler_adapter::environment::ScanHealth::Partial("cancelled partway".to_owned())
    } else {
        juggler_adapter::environment::ScanHealth::Ok
    };
    let count = |rows: &[DiscoveredInfo]| u32::try_from(rows.len()).unwrap_or_default();
    vec![
        probe(
            "s1-config-dirs",
            installs_health.clone(),
            format!("{} candidate setup(s)", count(installs)),
        ),
        probe(
            "s2-env-overrides",
            juggler_adapter::environment::ScanHealth::Ok,
            env_override_detail(),
        ),
        probe(
            "s3-manager-t3",
            juggler_adapter::environment::ScanHealth::Ok,
            manager_detail(managers, "t3"),
        ),
        probe(
            "s4-manager-cc-switch",
            cc_health,
            manager_detail(managers, "cc-switch"),
        ),
        probe(
            "s5-shell-aliases",
            if dangling > 0 {
                juggler_adapter::environment::ScanHealth::Partial(format!(
                    "{dangling} assignment(s) point at directories that no longer exist"
                ))
            } else {
                juggler_adapter::environment::ScanHealth::Ok
            },
            format!(
                "{} assignment(s){}",
                alias_rows.len(),
                if dangling > 0 {
                    format!(" ({dangling} dangling)")
                } else {
                    String::new()
                }
            ),
        ),
        probe(
            "s6-installed-clis",
            juggler_adapter::environment::ScanHealth::Ok,
            discoveries
                .iter()
                .map(|(id, installed)| {
                    format!("{id}: {}", if *installed { "installed" } else { "missing" })
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
        probe("s7-login-metadata", installs_health.clone(), {
            let signed = installs
                .iter()
                .filter(|install| install.login == "logged_in")
                .count();
            format!("{signed} of {} signed in", installs.len())
        }),
        probe("s8-usage-recency", installs_health.clone(), {
            let active = installs
                .iter()
                .filter(|install| install.last_used_secs.is_some())
                .count();
            format!("{active} setup(s) with recent activity")
        }),
        probe(
            "s9-projects",
            juggler_adapter::environment::ScanHealth::Ok,
            format!("{} project name(s)", seeds.len()),
        ),
        probe(
            "s10-desktop",
            juggler_adapter::environment::ScanHealth::Ok,
            if desktop_apps.is_empty() {
                "none of the known desktop apps present".to_owned()
            } else {
                desktop_apps.join(", ")
            },
        ),
    ]
}

/// S2 detail: which of the two known override names are set in the
/// app's environment (name match only).
fn env_override_detail() -> String {
    let set: Vec<&str> = ["CLAUDE_CONFIG_DIR", "CODEX_HOME"]
        .iter()
        .filter(|name| std::env::var_os(*name).is_some())
        .copied()
        .collect();
    if set.is_empty() {
        "none set in this app's environment".to_owned()
    } else {
        format!("set: {}", set.join(", "))
    }
}

/// S3/S4 detail: one manager's metadata line ("not present" when
/// absent).
fn manager_detail(managers: &[juggler_adapter::environment::ManagerHint], manager: &str) -> String {
    managers
        .iter()
        .find(|hint| hint.manager == manager)
        .map_or_else(
            || "not present".to_owned(),
            |hint| {
                if hint.present {
                    hint.detail.clone()
                } else {
                    "not present".to_owned()
                }
            },
        )
}

/// AJ-E302 Req.5: cancels a running scan (the wizard calls it on
/// close/unmount; the report that returns carries `cancelled`).
#[tauri::command]
pub fn scan_cancel() {
    SCAN_CANCEL.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// AJ-E304: the recommendation read side — the scan inventory becomes
// one set-level proposal. STAGING ONLY: no account, login, or file
// copy happens here (the accept click executes, in the wizard).

/// AJ-E304: builds the recommendation from the current scan (the
/// engine itself is pure in `juggler_core::recommendation`).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the scan fails.
#[tauri::command]
pub fn recommendation_for(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<juggler_core::recommendation::Recommendation> {
    recommendation_for_impl(state.inner())
}

/// Core implementation shared with integration tests (the
/// zero-writes-before-accept guardrail).
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn recommendation_for_impl(
    state: &Arc<AppState>,
) -> AppResult<juggler_core::recommendation::Recommendation> {
    let report = environment_scan_full_impl(state, None)?;
    let candidates: Vec<juggler_core::recommendation::RecCandidate> = report
        .installs
        .iter()
        .map(|install| juggler_core::recommendation::RecCandidate {
            provider: install.harness.clone(),
            purpose: install.inference.purpose.clone(),
            signed_in: install.login == "logged_in",
            dormant: matches!(install.inference.activity.as_str(), "dormant" | "archive"),
            managed: install.managed,
            source_path: Some(install.path.clone()),
        })
        .collect();
    // Installed-but-unconfigured Providers: discover says installed,
    // the scan found no candidate there.
    let configured: std::collections::BTreeSet<String> = report
        .installs
        .iter()
        .map(|install| install.harness.clone())
        .collect();
    let gaps = providers_discover_impl(state)?
        .into_iter()
        .filter(|row| row.status != "not_installed" && !configured.contains(&row.provider))
        .map(|row| juggler_core::recommendation::RecProviderGap {
            provider: row.provider,
            display_name: row.display_name,
        })
        .collect::<Vec<_>>();
    Ok(juggler_core::recommendation::recommend(
        &candidates,
        &gaps,
        report.seeds.first().cloned(),
    ))
}
