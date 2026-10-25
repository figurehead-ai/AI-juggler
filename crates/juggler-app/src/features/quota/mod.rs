//! The quota display (AJ-E610, AJ-E611, #120's usage honesty): each
//! account's window clocks, renewal, forecast and credits as every quota
//! surface reads them, and what each tool reports about its limits. A
//! feature module (ADR-0019 §3), moved from `commands.rs` (the size
//! budget: it only shrinks).

#![allow(clippy::wildcard_imports)]

pub mod capture;
pub mod probe;
pub(crate) mod records;
pub mod sources;

use crate::commands::*;

/// AJ-E610: one window clock's rendered state — the typed shape every
/// quota surface reads (rail chips, Inspector `Usage` tab, `NewSession`
/// rows). `percentage: None` in Unknown/Learning — a missing number is
/// the honest state, never 0%.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct WindowDisplay {
    /// The §Q8 machine state (`unknown|learning|tracking|stale|exhausted`),
    /// or `unmeasured`: use was seen but there is no percentage (no
    /// Measured sample, no confirmed cap), or `reset`: the window the
    /// vendor's last reading described has reset since (nothing is known
    /// about the new one). A state that reads as known always carries its
    /// percentage (the 0.0.12 report).
    pub state: String,
    /// Percent used, ONLY when the state carries one.
    pub percentage: Option<f64>,
    /// The badge (`Measured|Computed|Estimate`), adjacent on screen.
    pub badge: Option<String>,
    /// Staleness note ("last measured Nh ago") when Measured went quiet.
    pub stale_note_ms: Option<i64>,
    /// The reset instant, epoch ms, when there is one.
    pub resets_at_ms: Option<i64>,
    /// Where `resets_at_ms` came from: `measured` (the vendor's) or
    /// `calendar` (an anchored weekly phase).
    pub reset_basis: Option<String>,
    /// When the vendor's reading behind this window was taken, epoch ms:
    /// the "as of" every surface shows beside a measured percentage.
    pub measured_at_ms: Option<i64>,
    /// Absolute burn inside the window (the honest floor).
    pub burned_tokens: u64,
    /// Burn in vendor credits, when the provider bills credits.
    pub burned_credits: Option<f64>,
    /// Headroom in tokens (cap − burned), ONLY when the cap is
    /// known — the admission verdict's input (AJ-E306); unknown
    /// stays None, never a fabricated number.
    pub headroom_tokens: Option<f64>,
    /// AJ-E1512: sessions left and the burn-down, from the vendor's own
    /// readings of this window and the account's typical session. Absent
    /// without a Measured reading of a window that hasn't reset.
    pub runway: Option<RunwayDisplay>,
    /// Where the reading behind this window came from (`statusline`,
    /// `rollout`, `desktop_app`, `usage_cache`, `api_retry`), when known.
    pub source: Option<String>,
    /// That source in words, from the tool's spec ("the Claude app").
    pub source_words: Option<String>,
    /// In `reset` only: what the last reading said, as history ("40%
    /// used, 13 h ago; that window has reset since"), never as now.
    pub last_used_pct: Option<f64>,
}

/// AJ-E1512: a window's runway on the wire (`juggler_core::quota_runway`).
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct RunwayDisplay {
    /// The window's readings so far, oldest first: `[epoch ms, percent used]`.
    pub series: Vec<(i64, f64)>,
    /// When the window began.
    pub starts_ms: i64,
    /// When it resets.
    pub resets_at_ms: i64,
    /// Percent used now.
    pub used_pct: f64,
    /// The recent pace, percent of the window per hour.
    pub pct_per_hour: Option<f64>,
    /// When it runs out at that pace, only when before the reset.
    pub exhausts_at_ms: Option<i64>,
    /// A typical session's cost, percent of the window.
    pub session_pct: Option<f64>,
    /// Typical sessions left in the window.
    pub sessions_left: Option<u32>,
    /// The past sessions the typical one is the median of.
    pub sessions_counted: usize,
    /// `measured|computed|estimated`, the lowest of what went in.
    pub badge: String,
}

fn runway_display(
    samples: &[juggler_core::quota::QuotaSample],
    events: &[juggler_core::quota::UsageEvent],
    kind: juggler_core::quota::WindowKind,
    now_ms: i64,
) -> Option<RunwayDisplay> {
    juggler_core::quota_runway::runway(samples, events, kind.width_ms(), now_ms).map(|runway| {
        RunwayDisplay {
            series: runway
                .series
                .iter()
                .map(|point| (point.at_ms, point.used_pct))
                .collect(),
            starts_ms: runway.starts_ms,
            resets_at_ms: runway.resets_at_ms,
            used_pct: runway.used_pct,
            pct_per_hour: runway.pct_per_hour,
            exhausts_at_ms: runway.exhausts_at_ms,
            session_pct: runway.session_pct,
            sessions_left: runway.sessions_left,
            sessions_counted: runway.sessions_counted,
            badge: confidence_name(runway.confidence).to_owned(),
        }
    })
}

/// A window's state on the wire. Tracking or stale with no percentage
/// (activity, but no Measured sample and no confirmed cap) is
/// `unmeasured`: it never reads as a known state (the 0.0.12 report,
/// where every such window showed "100% left").
#[must_use]
pub fn window_state_name(
    state: juggler_core::quota::QuotaState,
    percentage: Option<f64>,
) -> &'static str {
    use juggler_core::quota::QuotaState;
    match (state, percentage) {
        (QuotaState::Unknown, _) => "unknown",
        (QuotaState::Learning, _) => "learning",
        (QuotaState::Tracking | QuotaState::Stale, None) => "unmeasured",
        (QuotaState::Tracking, Some(_)) => "tracking",
        (QuotaState::Stale, Some(_)) => "stale",
        (QuotaState::Exhausted, _) => "exhausted",
        (QuotaState::ResetSinceReading, _) => "reset",
    }
}

/// A Provider's name in words ("Claude Code", "Codex"), from its
/// adapter's `ToolSpec` (ADR-0019; the registered adapters, read once).
pub(crate) fn provider_name(provider: &str) -> String {
    static REGISTRY: std::sync::OnceLock<juggler_adapter::AdapterRegistry> =
        std::sync::OnceLock::new();
    crate::features::tools::display_name(
        REGISTRY.get_or_init(crate::state::default_registry),
        provider,
    )
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
    /// Codex's credits, from its own rollouts (the 0.0.12 report).
    pub credits: Option<CreditsDisplay>,
}

/// An account's credits, as its tool reports them.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct CreditsDisplay {
    /// The plan has credits at all.
    pub has_credits: bool,
    /// Credits are unlimited.
    pub unlimited: bool,
    /// The balance as the tool wrote it.
    pub balance: Option<String>,
    /// When the tool reported it, epoch ms.
    pub as_of_ms: i64,
    /// Who reported it: the tool's name ("Codex").
    pub source: String,
}

/// The account's newest credits snapshot, when its tool reported one.
fn credits_of(
    storage: &juggler_core::storage::Storage,
    account: &juggler_core::model::Account,
) -> Option<CreditsDisplay> {
    let id = account.id.as_str();
    let raw = storage
        .app_settings()
        .get(&format!(
            "{}{id}",
            crate::usage_pipeline::CREDITS_KEY_PREFIX
        ))
        .ok()
        .flatten()?;
    let snapshot: juggler_adapters::CreditsSnapshot = serde_json::from_str(&raw).ok()?;
    Some(CreditsDisplay {
        has_credits: snapshot.has_credits,
        unlimited: snapshot.unlimited,
        balance: snapshot.balance,
        as_of_ms: snapshot.ts,
        source: provider_name(account.harness.as_str()),
    })
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
    use juggler_core::quota::{self, AccountEvaluation, QuotaCap, SubscriptionClock, WindowKind};
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
    // Each sample with where it came from (the reading's source in words).
    let sourced_five_hour = usage
        .sourced_samples_for_account(account_id, juggler_core::quota::WindowKind::FiveHour)
        .unwrap_or_default();
    let sourced_weekly = usage
        .sourced_samples_for_account(
            account_id,
            juggler_core::quota::WindowKind::Weekly { anchor_ms: anchor },
        )
        .unwrap_or_default();
    let samples_five_hour: Vec<_> = sourced_five_hour
        .iter()
        .map(|(sample, _)| *sample)
        .collect();
    let samples_weekly: Vec<_> = sourced_weekly.iter().map(|(sample, _)| *sample).collect();
    let spec = adapter_spec(account.harness.as_str());
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

    let runway_five_hour =
        runway_display(&samples_five_hour, &events, WindowKind::FiveHour, now_ms);
    let runway_weekly = runway_display(
        &samples_weekly,
        &events,
        WindowKind::Weekly { anchor_ms: anchor },
        now_ms,
    );
    let window_display = |reading: &quota::WindowReading,
                          cap: QuotaCap,
                          runway: &Option<RunwayDisplay>| WindowDisplay {
        runway: runway.clone(),
        source: None,
        source_words: None,
        last_used_pct: reading.last_used_pct,
        state: window_state_name(reading.state, reading.percentage).to_owned(),
        percentage: reading.percentage,
        badge: reading
            .percentage
            .map(|_| confidence_name(reading.confidence).to_owned()),
        stale_note_ms,
        resets_at_ms: reading.resets_at,
        measured_at_ms: reading.measured_at,
        reset_basis: reading.reset_basis.map(|basis| {
            match basis {
                quota::ResetBasis::Measured => "measured",
                quota::ResetBasis::Calendar => "calendar",
            }
            .to_owned()
        }),
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

    // Window shapes (Req.5): a window that isn't there is omitted WITH
    // its reason, never an empty ring.
    let provider = account.harness.as_str();
    let mut unavailable_reason = None;
    let (five_hour, five_hour_reason, weekly, weekly_reason) = match provider {
        "gemini" | "antigravity" => {
            unavailable_reason = Some("No verified quota source for this tool yet".to_owned());
            (None, None, None, None)
        }
        // A registered tool whose adapter declares no limits feed: the same
        // capability Home and the provider list read, so every surface
        // says the same thing (QA: Usage said "100% left" for a tool Home
        // said doesn't report).
        _ if !reports_limits(provider) => {
            unavailable_reason = Some(format!(
                "{} doesn't report its limits to AI Juggler yet",
                provider_name(provider)
            ));
            (None, None, None, None)
        }
        // P13: a tool whose feed names its own windows shows the ones it
        // reports, never ones assumed from the tool or plan.
        _ if adapter_has(
            provider,
            juggler_adapter::Capabilities::LIMIT_WINDOWS_REPORTED,
        ) =>
        {
            let (five_reported, weekly_reported) =
                reported_windows(&samples_five_hour, &samples_weekly, now_ms);
            let tool = provider_name(provider);
            let reason = |kind: &str| {
                if five_reported || weekly_reported {
                    format!("{tool} reports no {kind} limit for this account")
                } else {
                    format!(
                        "{tool} hasn't reported its limits yet — they show after you use {tool}"
                    )
                }
            };
            (
                five_reported.then(|| {
                    window_display(
                        &evaluation.five_hour,
                        cap(WindowKind::FiveHour),
                        &runway_five_hour,
                    )
                }),
                (!five_reported).then(|| reason("five-hour")),
                weekly_reported.then(|| {
                    window_display(
                        &evaluation.weekly,
                        cap(WindowKind::Weekly { anchor_ms: anchor }),
                        &runway_weekly,
                    )
                }),
                (!weekly_reported).then(|| reason("weekly")),
            )
        }
        // A plan whose own records leave the week out (a team seat) has
        // none: said, never shown as "not measured".
        _ if sources::week_not_reported(&sourced_five_hour, &sourced_weekly, now_ms) => (
            Some(window_display(
                &evaluation.five_hour,
                cap(WindowKind::FiveHour),
                &runway_five_hour,
            )),
            None,
            None,
            Some(format!(
                "{} reports no weekly limit for this account",
                provider_name(provider)
            )),
        ),
        _ => (
            Some(window_display(
                &evaluation.five_hour,
                cap(WindowKind::FiveHour),
                &runway_five_hour,
            )),
            None,
            Some(window_display(
                &evaluation.weekly,
                cap(WindowKind::Weekly { anchor_ms: anchor }),
                &runway_weekly,
            )),
            None,
        ),
    };
    // The reading behind each window, named with its source.
    let named = |window: Option<WindowDisplay>,
                 sourced: &[(
        juggler_core::quota::QuotaSample,
        Option<juggler_core::quota::UsageSource>,
    )]| {
        window.map(|mut window| {
            let source = sources::reading_source(sourced, window.measured_at_ms);
            window.source = source.map(|source| source.as_str().to_owned());
            window.source_words = source
                .zip(spec.as_ref())
                .and_then(|(source, spec)| sources::source_words(spec, source));
            window
        })
    };
    let five_hour = named(five_hour, &sourced_five_hour);
    let weekly = named(weekly, &sourced_weekly);
    QuotaDisplayInfo {
        account: account_id.to_owned(),
        five_hour,
        five_hour_omitted_reason: five_hour_reason,
        weekly,
        weekly_omitted_reason: weekly_reason,
        renewal,
        forecast,
        unavailable_reason,
        credits: credits_of(storage, account),
    }
}

/// A built-in adapter's capabilities, by tool id (`None` for a tool with
/// no adapter here), read once from the built-in registry: the quota
/// display asks the adapters, never branches on a tool's id.
fn adapter_capabilities(harness: &str) -> Option<juggler_adapter::Capabilities> {
    static CAPS: std::sync::OnceLock<
        std::collections::BTreeMap<String, juggler_adapter::Capabilities>,
    > = std::sync::OnceLock::new();
    CAPS.get_or_init(|| {
        let registry = crate::state::default_registry();
        registry
            .ids()
            .into_iter()
            .filter_map(|id| Some((id.to_string(), registry.get(&id)?.capabilities())))
            .collect()
    })
    .get(harness)
    .copied()
}

/// A built-in adapter's spec, by tool id (`None` for a tool with no
/// adapter here): the words a reading's source is named with.
fn adapter_spec(harness: &str) -> Option<juggler_adapter::ToolSpec> {
    static SPECS: std::sync::OnceLock<
        std::collections::BTreeMap<String, juggler_adapter::ToolSpec>,
    > = std::sync::OnceLock::new();
    SPECS
        .get_or_init(|| {
            let registry = crate::state::default_registry();
            registry
                .ids()
                .into_iter()
                .filter_map(|id| Some((id.to_string(), registry.get(&id)?.spec())))
                .collect()
        })
        .get(harness)
        .cloned()
}

/// True when `harness`'s adapter declares `capability`.
pub(crate) fn adapter_has(harness: &str, capability: juggler_adapter::Capabilities) -> bool {
    adapter_capabilities(harness).is_some_and(|caps| caps.contains(capability))
}

/// True unless `harness` is a registered tool whose adapter declares no
/// limits feed (`USAGE_QUOTA_FEED`) — what `providers_capabilities`
/// reports as `reports_limits`. A tool with no adapter here (`ZCode`'s
/// computed credits) keeps its own rules.
#[must_use]
pub fn reports_limits(harness: &str) -> bool {
    adapter_capabilities(harness)
        .is_none_or(|caps| caps.contains(juggler_adapter::Capabilities::USAGE_QUOTA_FEED))
}

/// Which windows the tool reported in the last week (a Measured sample
/// whose feed named the window, e.g. Codex's `window_minutes`). A window
/// it never reports (`secondary: null`) is not there; one from a plan it
/// left reads as stale until it ages out.
fn reported_windows(
    five_hour: &[juggler_core::quota::QuotaSample],
    weekly: &[juggler_core::quota::QuotaSample],
    now_ms: i64,
) -> (bool, bool) {
    let reported = |samples: &[juggler_core::quota::QuotaSample]| {
        samples.iter().any(|sample| {
            sample.confidence == juggler_core::model::Confidence::Measured
                && now_ms.saturating_sub(sample.ts) <= juggler_core::quota::WEEK_MS
        })
    };
    (reported(five_hour), reported(weekly))
}
