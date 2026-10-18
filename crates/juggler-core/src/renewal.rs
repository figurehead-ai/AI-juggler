//! Subscription-expiry warnings (AJ-E411, D-12): the optional
//! user-entered renewal date per Account, its threshold label, and the
//! gated research seam for any future sanctioned source.
//!
//! D-12 (law): renewal dates are **user-entered only**. Inferring or
//! guessing from plan tier, usage, or billing cadence is PROHIBITED —
//! with no date, expiry is honestly `unknown`. Any future Provider
//! billing/renewal source must be sanctioned, read-only, and verified
//! live before it may populate the field; unproven sources stay
//! `unknown` (see [`RenewalSource`] and
//! `docs/research/subscription-expiry-sources.md`).
//!
//! Dates are local calendar dates in `YYYY-MM-DD` form — no timezone
//! conversion is ever applied to the stored value (Req.7). A past date
//! entered today warns immediately; nothing is "corrected".

/// A local calendar date (no time, no timezone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalendarDate {
    /// Year (proleptic Gregorian).
    pub year: i64,
    /// Month, 1..=12.
    pub month: u32,
    /// Day, 1..=31 (not month-validated beyond February's leap rule).
    pub day: u32,
}

impl CalendarDate {
    /// Parses `YYYY-MM-DD` (zero-padded); anything else is `None` —
    /// honest rejection, never a repair.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let mut parts = raw.split('-');
        let year = parts.next()?.parse::<i64>().ok()?;
        let month = parts.next()?.parse::<u32>().ok()?;
        let day = parts.next()?.parse::<u32>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Days from the epoch-anchored civil count (Howard Hinnant's
    /// `days_from_civil`): positive after 1970-01-01.
    #[must_use]
    pub fn civil_days(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Whole days from `today` until this date (negative = past).
    #[must_use]
    pub fn days_until(&self, today: CalendarDate) -> i64 {
        self.civil_days() - today.civil_days()
    }
}

/// Days in `month` of `year` (leap-aware February).
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Proleptic Gregorian leap rule.
fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Howard Hinnant's `days_from_civil` — days since 1970-01-01.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The threshold tier a renewal date falls into (Req.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryTier {
    /// More than 14 days out: the date alone.
    DateOnly,
    /// 8-14 days: `Renews in N days`.
    Countdown,
    /// 7 days or fewer: warning chip (amber) — text, never colour-only.
    Warning,
    /// Due date passed: danger chip.
    Danger,
}

/// A rendered expiry label (tier + the exact copy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryLabel {
    /// The threshold tier.
    pub tier: ExpiryTier,
    /// The exact display copy.
    pub text: String,
}

/// Computes the expiry label for a stored renewal date against a fixed
/// `today` (both local calendar dates — pure, so tests pin the clock).
/// A missing or malformed date yields `None`: callers render
/// `Expiry: unknown` (never a guess, D-12).
#[must_use]
pub fn renewal_label(renewal_date: Option<&str>, today: CalendarDate) -> Option<ExpiryLabel> {
    let raw = renewal_date?;
    let date = CalendarDate::parse(raw)?;
    let days = date.days_until(today);
    let (tier, text) = if days > 14 {
        (ExpiryTier::DateOnly, format!("Renews {raw}"))
    } else if days >= 8 {
        (ExpiryTier::Countdown, format!("Renews in {days} days"))
    } else if days >= 1 {
        (ExpiryTier::Warning, format!("Renews in {days} days"))
    } else {
        (ExpiryTier::Danger, "Renewal date passed".to_string())
    };
    Some(ExpiryLabel { tier, text })
}

/// Whether the tier surfaces on the Account row / participates in
/// Needs-me ranking (Req.2/Req.5): only at 7 days or fewer.
#[must_use]
pub fn is_row_visible(tier: ExpiryTier) -> bool {
    matches!(tier, ExpiryTier::Warning | ExpiryTier::Danger)
}

/// The D-12 research seam: a future sanctioned renewal-date source
/// (billing API, receipt…) implements this. The only shipped
/// implementation is [`UnknownRenewalSource`] — nothing is populated
/// until a source is sanctioned, read-only, AND verified live
/// (see `docs/research/subscription-expiry-sources.md`; a sanctioned
/// source is a NEW task, never a silent change here).
pub trait RenewalSource: Send + Sync {
    /// Looks up the renewal date for a provider account; `None` =
    /// unknown. `Err` = the source failed (never a guess fallback).
    ///
    /// # Errors
    ///
    /// Returns a message describing why the source failed; callers
    /// surface it, they never fall back to a guess.
    fn lookup(&self, provider: &str, account_label: &str) -> Result<Option<String>, String>;
}

/// The shipped source: always `Unknown` (D-12 default).
pub struct UnknownRenewalSource;

impl RenewalSource for UnknownRenewalSource {
    fn lookup(&self, _provider: &str, _account_label: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: CalendarDate = CalendarDate {
        year: 2026,
        month: 10,
        day: 14,
    };

    fn date(days_from_today: i64) -> String {
        // Anchor: 2026-10-14 + offset via civil arithmetic.
        let anchor = TODAY.civil_days() + days_from_today;
        let (y, m, d) = civil_from_days(anchor);
        format!("{y:04}-{m:02}-{d:02}")
    }

    /// Inverse of `days_from_civil` for test-date construction.
    fn civil_from_days(z: i64) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    #[test]
    fn renewal_no_date_is_unknown_never_guessed() {
        assert!(renewal_label(None, TODAY).is_none());
        assert!(renewal_label(Some("not-a-date"), TODAY).is_none());
    }

    #[test]
    fn renewal_far_future_is_date_only() {
        let label = renewal_label(Some(&date(15)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::DateOnly);
        assert_eq!(label.text, format!("Renews {}", date(15)));
        assert!(!is_row_visible(label.tier));
    }

    #[test]
    fn renewal_fourteen_days_is_countdown() {
        let label = renewal_label(Some(&date(14)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Countdown);
        assert_eq!(label.text, "Renews in 14 days");
        assert!(!is_row_visible(label.tier));
    }

    #[test]
    fn renewal_eight_days_is_countdown() {
        let label = renewal_label(Some(&date(8)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Countdown);
        assert_eq!(label.text, "Renews in 8 days");
    }

    #[test]
    fn renewal_seven_days_is_warning_chip() {
        let label = renewal_label(Some(&date(7)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Warning);
        assert_eq!(label.text, "Renews in 7 days");
        assert!(is_row_visible(label.tier));
    }

    #[test]
    fn renewal_one_day_is_warning_chip() {
        let label = renewal_label(Some(&date(1)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Warning);
        assert_eq!(label.text, "Renews in 1 days");
    }

    #[test]
    fn renewal_today_is_danger() {
        let label = renewal_label(Some(&date(0)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Danger);
        assert_eq!(label.text, "Renewal date passed");
        assert!(is_row_visible(label.tier));
    }

    #[test]
    fn renewal_yesterday_is_danger_and_never_corrected() {
        let label = renewal_label(Some(&date(-1)), TODAY).unwrap();
        assert_eq!(label.tier, ExpiryTier::Danger);
        assert_eq!(label.text, "Renewal date passed");
    }

    #[test]
    fn renewal_calendar_dates_parse_strictly() {
        assert!(CalendarDate::parse("2026-02-29").is_none()); // not a leap year
        assert!(CalendarDate::parse("2028-02-29").is_some()); // leap year
        assert!(CalendarDate::parse("2026-13-01").is_none());
        assert!(CalendarDate::parse("2026-10-14T00:00").is_none());
        assert!(CalendarDate::parse("2026-10-14").is_some());
    }

    #[test]
    fn renewal_source_seam_returns_unknown_today() {
        let source = UnknownRenewalSource;
        assert_eq!(source.lookup("claude-code", "Work Max"), Ok(None));
    }
}
