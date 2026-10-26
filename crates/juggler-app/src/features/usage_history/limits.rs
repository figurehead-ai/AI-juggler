//! A profile's limit windows over time, and one session's usage turn by
//! turn with its share of those windows.

use std::sync::Arc;

use juggler_core::quota::{QuotaSample, WindowKind};
use juggler_core::storage::thin;

use super::dto::{
    UsageLimitsInfo, UsageSessionDetailInfo, UsageShareInfo, UsageTotalsInfo, UsageTurnInfo,
    UsageWindowHistoryInfo, UsageWindowPeakInfo, UsageWindowShareInfo,
};
use super::fold::io;
use crate::commands::{TokenCountsInfo, lock_storage_arc};
use crate::error::AppResult;
use crate::state::AppState;

/// The most readings a range's window line carries.
const MAX_POINTS: usize = 400;
/// The most readings read for one window in one range.
const MAX_SAMPLES: usize = 200_000;
/// The most turns a session's page draws.
const MAX_TURNS: usize = 2_000;
/// Two reset times within this are the same reset (vendors round).
const SAME_RESET_MS: i64 = 120_000;
/// A fall of this many points with no reset time is a new window.
const FALL_PCT: f64 = 5.0;

const FIVE_HOURS_MS: i64 = 5 * 3_600_000;
const WEEK_MS: i64 = 7 * 86_400_000;

fn weekly() -> WindowKind {
    WindowKind::Weekly { anchor_ms: None }
}

/// The windows a run of readings saw: a new one wherever the reset time
/// moves on, or (with no reset time) the reading falls back.
fn windows_of(samples: &[QuotaSample]) -> Vec<UsageWindowPeakInfo> {
    let mut out: Vec<UsageWindowPeakInfo> = Vec::new();
    let mut last: Option<&QuotaSample> = None;
    for sample in samples {
        let fresh = match (last, out.last()) {
            (None, _) | (_, None) => true,
            (Some(previous), Some(open)) => match (open.resets_ms, sample.resets_at) {
                (Some(was), Some(now)) => (now - was).abs() > SAME_RESET_MS,
                _ => sample.used_pct + FALL_PCT < previous.used_pct,
            },
        };
        if fresh {
            out.push(UsageWindowPeakInfo {
                first_ms: sample.ts,
                resets_ms: sample.resets_at,
                peak_pct: sample.used_pct,
            });
        } else if let Some(open) = out.last_mut() {
            open.peak_pct = open.peak_pct.max(sample.used_pct);
            if open.resets_ms.is_none() {
                open.resets_ms = sample.resets_at;
            }
        }
        last = Some(sample);
    }
    out
}

fn history(samples: Vec<QuotaSample>) -> UsageWindowHistoryInfo {
    let windows = windows_of(&samples);
    let count = u32::try_from(samples.len()).unwrap_or(u32::MAX);
    UsageWindowHistoryInfo {
        points: thin(samples, MAX_POINTS)
            .into_iter()
            .map(|sample| (sample.ts, sample.used_pct))
            .collect(),
        windows,
        samples: count,
    }
}

/// A profile's 5-hour and weekly windows over `[start, end)`.
///
/// # Errors
///
/// Storage errors.
pub fn limits_impl(
    state: &Arc<AppState>,
    profile: &str,
    start_ms: i64,
    end_ms: i64,
) -> AppResult<UsageLimitsInfo> {
    let storage = lock_storage_arc(state)?;
    let repo = storage.usage_history();
    Ok(UsageLimitsInfo {
        profile: profile.to_owned(),
        five_hour: history(repo.samples_between(
            profile,
            WindowKind::FiveHour,
            start_ms,
            end_ms,
            MAX_SAMPLES,
        )?),
        weekly: history(repo.samples_between(profile, weekly(), start_ms, end_ms, MAX_SAMPLES)?),
    })
}

fn tokens_info(tokens: juggler_core::model::TokenCounts) -> TokenCountsInfo {
    TokenCountsInfo {
        input: tokens.input,
        output: tokens.output,
        cache_read: tokens.cache_read,
        cache_write: tokens.cache_write,
        reasoning: tokens.reasoning,
    }
}

/// One session's usage turn by turn, by model, and its share of the
/// profile's windows now.
///
/// # Errors
///
/// Storage errors.
pub fn session_impl(
    state: &Arc<AppState>,
    profile: &str,
    conversation: &str,
) -> AppResult<UsageSessionDetailInfo> {
    let storage = lock_storage_arc(state)?;
    let catalog = crate::commands::pricing_catalog(&storage);
    let repo = storage.usage_history();
    let (rows, truncated) = repo.session_turns(profile, conversation, MAX_TURNS)?;
    let turns = rows
        .into_iter()
        .map(|row| UsageTurnInfo {
            at_ms: row.ts,
            usd: row
                .model
                .as_deref()
                .and_then(|model| catalog.price_tokens(model, row.ts, row.tokens)),
            model: row.model,
            tokens: tokens_info(row.tokens),
        })
        .collect();
    let mut totals = UsageTotalsInfo {
        sessions: 1,
        ..UsageTotalsInfo::default()
    };
    let mut all = juggler_core::model::TokenCounts::default();
    let mut usd = 0.0;
    let mut priced = false;
    let mut by_model = Vec::new();
    for row in repo.session_by_model(profile, conversation)? {
        juggler_core::storage::add_tokens(&mut all, row.tokens);
        totals.messages = totals.messages.saturating_add(row.messages);
        let cost = row
            .model
            .as_deref()
            .and_then(|model| catalog.price_tokens(model, row.first_ms, row.tokens));
        match cost {
            Some(value) => {
                usd += value;
                priced = true;
            }
            None => totals.unpriced = true,
        }
        let key = row.model.clone().unwrap_or_default();
        by_model.push(UsageShareInfo {
            label: super::fold::label_of("model", &key, &super::fold::Lookups::default()),
            key,
            io: io(&row.tokens),
            cache: row.tokens.cache_read.saturating_add(row.tokens.cache_write),
            usd: cost,
            sessions: 1,
            profiles: Vec::new(),
        });
    }
    by_model.sort_by_key(|share| std::cmp::Reverse(share.io));
    totals.tokens = tokens_info(all);
    totals.usd = priced.then_some(usd);
    let now = juggler_core::model::Timestamp::now().millis();
    let mut windows = Vec::new();
    for (name, kind, length) in [
        ("five_hour", WindowKind::FiveHour, FIVE_HOURS_MS),
        ("weekly", weekly(), WEEK_MS),
    ] {
        // The vendor's reset time places the window; without one, the
        // last 5 hours or 7 days.
        let reset = repo
            .latest_sample(profile, kind, now)?
            .and_then(|sample| sample.resets_at)
            .filter(|resets| *resets > now && *resets - length <= now);
        let (since_ms, basis) = match reset {
            Some(resets) => (resets - length, "window"),
            None => (now - length, "rolling"),
        };
        let (profile_tokens, session_tokens) = repo.share_since(profile, conversation, since_ms)?;
        windows.push(UsageWindowShareInfo {
            window: name.to_owned(),
            since_ms,
            basis: basis.to_owned(),
            session_io: io(&session_tokens),
            profile_io: io(&profile_tokens),
        });
    }
    Ok(UsageSessionDetailInfo {
        turns,
        truncated,
        totals,
        by_model,
        windows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: i64, used_pct: f64, resets_at: Option<i64>) -> QuotaSample {
        QuotaSample {
            ts,
            used_pct,
            resets_at,
            confidence: juggler_core::model::Confidence::Measured,
        }
    }

    #[test]
    fn a_new_reset_time_starts_a_new_window_and_each_keeps_its_peak() {
        let hour = 3_600_000;
        let samples = [
            sample(0, 10.0, Some(5 * hour)),
            sample(hour, 40.0, Some(5 * hour + 30_000)),
            sample(2 * hour, 35.0, Some(5 * hour)),
            sample(6 * hour, 5.0, Some(11 * hour)),
            sample(7 * hour, 22.0, Some(11 * hour)),
        ];
        let windows = windows_of(&samples);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].peak_pct, 40.0);
        assert_eq!(windows[0].resets_ms, Some(5 * hour));
        assert_eq!(windows[1].first_ms, 6 * hour);
        assert_eq!(windows[1].peak_pct, 22.0);
    }

    #[test]
    fn without_reset_times_a_fall_starts_a_new_window() {
        let samples = [
            sample(0, 30.0, None),
            sample(1, 80.0, None),
            sample(2, 78.0, None),
            sample(3, 4.0, None),
        ];
        let windows = windows_of(&samples);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].peak_pct, 80.0);
        assert_eq!(windows[1].peak_pct, 4.0);
    }
}
