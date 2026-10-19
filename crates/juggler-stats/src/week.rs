//! ISO-8601 weeks in UTC (`2026-W42`), computed from epoch milliseconds
//! without a calendar dependency.

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian.
#[must_use]
pub fn civil(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's days-from-civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// (year, month, day) → days since 1970-01-01.
#[must_use]
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(month);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// ISO weekday, Monday = 1 … Sunday = 7.
fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7) + 1 // 1970-01-01 was a Thursday
}

/// The ISO week (`2026-W42`) of a UTC instant.
#[must_use]
pub fn label(at_ms: i64) -> String {
    let days = at_ms.div_euclid(86_400_000);
    let (year, week) = iso_week(days);
    format!("{year}-W{week:02}")
}

fn iso_week(days: i64) -> (i64, i64) {
    // The Thursday of this week decides the ISO year.
    let thursday = days - weekday(days) + 4;
    let (year, _, _) = civil(thursday);
    let jan1 = days_from_civil(year, 1, 1);
    (year, (thursday - jan1) / 7 + 1)
}

/// Monday 00:00 UTC that starts `label`'s week, in epoch ms (`None` for a
/// malformed label).
#[must_use]
pub fn start_ms(label: &str) -> Option<i64> {
    let (year, week) = label.split_once("-W")?;
    let year: i64 = year.parse().ok()?;
    let week: i64 = week.parse().ok()?;
    if !(1..=53).contains(&week) {
        return None;
    }
    // Week 1 holds January 4th.
    let jan4 = days_from_civil(year, 1, 4);
    let monday = jan4 - (weekday(jan4) - 1) + (week - 1) * 7;
    Some(monday * 86_400_000)
}

/// When `label`'s week closes (the next Monday 00:00 UTC), epoch ms.
#[must_use]
pub fn end_ms(label: &str) -> Option<i64> {
    start_ms(label).map(|start| start + 7 * 86_400_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(year: i64, month: u32, day: u32) -> i64 {
        days_from_civil(year, month, day) * 86_400_000
    }

    #[test]
    fn iso_weeks_match_known_dates() {
        assert_eq!(label(ms(2026, 10, 18)), "2026-W42");
        assert_eq!(label(ms(2026, 10, 19)), "2026-W43"); // a Monday
        assert_eq!(label(ms(2021, 1, 3)), "2020-W53"); // ISO year differs
        assert_eq!(label(ms(2024, 12, 30)), "2025-W01");
        assert_eq!(label(ms(1970, 1, 1)), "1970-W01");
    }

    #[test]
    fn a_week_runs_monday_to_monday() {
        let start = start_ms("2026-W42").unwrap();
        assert_eq!(start, ms(2026, 10, 12));
        assert_eq!(end_ms("2026-W42").unwrap(), ms(2026, 10, 19));
        assert_eq!(label(start), "2026-W42");
        assert_eq!(label(start - 1), "2026-W41");
        assert_eq!(start_ms("2020-W53").unwrap(), ms(2020, 12, 28));
        assert!(start_ms("garbage").is_none());
    }

    #[test]
    fn civil_round_trips() {
        for days in [-1_000_000, -1, 0, 1, 19_000, 20_744, 2_000_000] {
            let (y, m, d) = civil(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }
}
