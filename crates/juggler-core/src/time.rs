//! Timestamps as the tools write them (RFC 3339), in epoch milliseconds:
//! one parser for transcripts, rollouts and status lines (AJ-E1530).

use crate::renewal::CalendarDate;

/// Parses `YYYY-MM-DDTHH:MM:SS(.fff)?` with `Z` or a `±HH:MM` offset
/// (Claude's status line writes `2026-10-13T18:00:00-07:00`; transcripts
/// and rollouts `…Z`) into epoch milliseconds. The one parser for every
/// timestamp the tools write (AJ-E1530). Returns `None` for anything else — an unparseable
/// timestamp is honest damage (counted), never a fabricated one.
#[must_use]
pub fn timestamp_ms(raw: &str) -> Option<i64> {
    let date_end = raw.find('T')?;
    let date = CalendarDate::parse(&raw[..date_end])?;
    let time = &raw[date_end + 1..];
    // The offset, east of UTC in minutes: the local time minus it is UTC.
    let (time, offset_minutes) = if let Some(utc) = time.strip_suffix('Z') {
        (utc, 0)
    } else if time.len() > 6
        && matches!(time.as_bytes()[time.len() - 6], b'+' | b'-')
        && time.as_bytes()[time.len() - 3] == b':'
    {
        let (clock, zone) = time.split_at(time.len() - 6);
        let hours: i64 = zone[1..3].parse().ok()?;
        let minutes: i64 = zone[4..6].parse().ok()?;
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        (clock, sign * (hours * 60 + minutes))
    } else {
        (time, 0)
    };
    let (hms, millis) = match time.split_once('.') {
        // A fraction is of a second: ".5" is 500 ms, ".391" 391 ms.
        Some((hms, frac)) if !frac.is_empty() && frac.bytes().all(|b| b.is_ascii_digit()) => (
            hms,
            format!("{:0<3}", &frac[..frac.len().min(3)])
                .parse::<i64>()
                .ok()?,
        ),
        Some(_) => return None,
        None => (time, 0),
    };
    let mut parts = hms.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(0..24).contains(&hours)
        || !(0..60).contains(&minutes)
        || !(0..60).contains(&seconds)
    {
        return None;
    }
    Some(
        date.civil_days() * 86_400_000 + (hours * 3_600 + minutes * 60 + seconds) * 1000 + millis
            - offset_minutes * 60_000,
    )
}

#[cfg(test)]
mod tests {
    use super::timestamp_ms;

    #[test]
    fn reads_z_offsets_and_fractions_and_refuses_the_rest() {
        assert_eq!(timestamp_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            timestamp_ms("2026-10-21T19:18:59.391Z"),
            Some(1_792_610_339_391)
        );
        assert_eq!(
            timestamp_ms("2026-10-21T19:18:59Z"),
            Some(1_792_610_339_000)
        );
        assert_eq!(
            timestamp_ms("2024-02-29T12:00:00.5Z"),
            Some(1_709_208_000_500)
        );
        assert_eq!(
            timestamp_ms("2026-10-14T06:30:00+05:30"),
            timestamp_ms("2026-10-14T01:00:00Z")
        );
        assert_eq!(
            timestamp_ms("2026-10-13T18:00:00-07:00"),
            timestamp_ms("2026-10-14T01:00:00Z")
        );
        assert_eq!(
            timestamp_ms("2026-10-21T19:18:59.391123Z"),
            Some(1_792_610_339_391)
        );
        for bad in [
            "yesterday",
            "2026-13-01T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2026-10-14T01:00:00.Z",
            "2026-10-14T25:00:00Z",
        ] {
            assert_eq!(timestamp_ms(bad), None, "{bad}");
        }
    }
}
