//! AJ-E1605 R3 (D-106, keyless): a `ZCode` model call refused at the
//! plan's limit, from the App's own session store (`model_usage`).
//!
//! The engine records every model call it makes; a call Z.ai refused at
//! the GLM Coding Plan's limit is a row with `status = 'error'`,
//! `error_type = 'rate_limited'` and Z.ai's own message in
//! `error_message` (its `error_code` is empty). The engine passes the
//! message on unchanged: its own code (0.16.9) only knows the codes, 1308
//! and 1310, as "stop, quota" (non-retryable). The message's shape was
//! measured on a real store on 2026-11-13 by pattern counts only (no text
//! read: 236 such rows, every one of them this shape, 116 characters):
//!
//! - `[1308][Usage limit reached for 5 hour. Your limit will reset at
//!   YYYY-MM-DD HH:MM:SS][<request id>]` — the 5-hour window;
//! - `[1310][Weekly/Monthly Limit Exhausted. Your limit will reset at
//!   YYYY-MM-DD HH:MM:SS][<request id>]` — the longer window, read as the
//!   week. **UNCONFIRMED**: Z.ai's words name "Weekly/Monthly"; every reset
//!   on the measured store fell 5.6–6.5 days after its hit, which fits a
//!   week, but no monthly case was seen.
//!
//! The request id is 30 lowercase hex characters on the measured store;
//! any length up to 64 is taken, and it is dropped as parsed (never kept).
//! Anything else — another code, other words, a code with the other
//! window's words, a reset that isn't a valid time — is no hit: a window is
//! never guessed.
//!
//! **The reset's time zone is UNCONFIRMED.** Z.ai writes no zone. Read as
//! UTC+8 (Beijing time, the zone of Z.ai's published peak hours), every one
//! of the 138 measured 5-hour resets fell 0–4.5 hours after its hit, as a
//! 5-hour window must; read as UTC they fell 8–12.5 hours after, which a
//! 5-hour window can't. So it is read as UTC+8 until the lab check
//! (AJ-E1612) sees one live.
//!
//! A hit holds for a session while its last model call is that refusal:
//! any later call (the model replied, or another refusal) decides again.

use std::path::Path;

use juggler_adapter::live::LimitHit;
use juggler_core::quota::WindowKind;

/// The zone Z.ai's resets are read in (UTC+8; see the module docs).
pub const RESET_OFFSET_MS: i64 = 8 * 3_600_000;
/// The longest request id taken.
const ID_MAX: usize = 64;
/// The words between the window and the time.
const RESETS: &str = " Your limit will reset at ";

/// Z.ai's codes and words, by the window each names.
const MESSAGES: &[(&str, &str, WindowKind)] = &[
    (
        "1308",
        "Usage limit reached for 5 hour.",
        WindowKind::FiveHour,
    ),
    (
        "1310",
        "Weekly/Monthly Limit Exhausted.",
        WindowKind::Weekly { anchor_ms: None },
    ),
];

/// What a refusal says: the window and its reset (epoch ms).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The window that ran out.
    pub window: WindowKind,
    /// When Z.ai says it resets, epoch ms (read as UTC+8).
    pub resets_at_ms: i64,
}

/// What `message` says, when it is exactly one of Z.ai's limit messages.
#[must_use]
pub fn read_message(message: &str) -> Option<Refusal> {
    MESSAGES.iter().find_map(|(code, words, window)| {
        let rest = message
            .strip_prefix('[')?
            .strip_prefix(code)?
            .strip_prefix("][")?
            .strip_prefix(words)?
            .strip_prefix(RESETS)?;
        let (time, rest) = rest.split_at_checked(19)?;
        let id = rest.strip_prefix("][")?.strip_suffix(']')?;
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if id.is_empty() || id.len() > ID_MAX || !id.chars().all(hex) {
            return None;
        }
        Some(Refusal {
            window: *window,
            resets_at_ms: utc8_ms(time)?,
        })
    })
}

/// `YYYY-MM-DD HH:MM:SS` read in UTC+8, as epoch ms; `None` for anything
/// else (another shape, a date that doesn't exist).
fn utc8_ms(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    let shape = b.len() == 19
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b' '
        && b[13] == b':'
        && b[16] == b':';
    if !shape {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        let part = text.get(from..to)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let month_days = *days_in.get(usize::try_from(month - 1).ok()?)?;
    if !(1..=month_days).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    // Days from 1970-01-01 (Howard Hinnant's days_from_civil).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let local = ((days * 24 + hour) * 60 + minute) * 60 + second;
    Some(local * 1_000 - RESET_OFFSET_MS)
}

/// The hit `session` holds in the store at `db`: its last model call
/// (by start) was refused at the plan's limit. Read-only, as the usage
/// read opens it.
///
/// # Errors
///
/// The store can't be read in the shape read on 0.16.9.
pub fn session_hit(db: &Path, session: &str) -> Result<Option<LimitHit>, String> {
    let conn = crate::common::sqlite::open_tool_store(db, "ZCode")?;
    let last = conn.query_row(
        "SELECT status, error_type, error_message, started_at FROM model_usage
         WHERE session_id = ?1 ORDER BY started_at DESC, rowid DESC LIMIT 1",
        [session],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        },
    );
    let (status, kind, message, at) = match last {
        Ok(row) => row,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(error) => return Err(format!("zcode model_usage limit read: {error}")),
    };
    if status.as_deref() != Some("error") || kind.as_deref() != Some("rate_limited") {
        return Ok(None);
    }
    Ok(message
        .as_deref()
        .and_then(read_message)
        .filter(|refusal| refusal.resets_at_ms > at)
        .map(|refusal| LimitHit {
            at_ms: at,
            resets_at_ms: Some(refusal.resets_at_ms),
            window: Some(refusal.window),
        }))
}

/// [`session_hit`] for a session's link (`<db>#<id>`): `None` for a link
/// that isn't one (the caller reads a tail), `Some(None)` when no hit
/// holds or the store can't be read.
#[must_use]
pub fn session_limit_hit(transcript: &Path) -> Option<Option<LimitHit>> {
    let (db, id) =
        crate::common::sqlite::store_ref_parts(transcript, crate::zcode_sessions::DB_FILE)?;
    Some(session_hit(&db, &id).ok().flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0123456789abcdef0123456789abcd";

    #[test]
    fn zai_messages_name_their_window_and_reset_in_utc8() {
        let five = format!(
            "[1308][Usage limit reached for 5 hour. Your limit will reset at 2026-10-21 18:30:00][{ID}]"
        );
        assert_eq!(five.len(), 116, "the measured length");
        // 2026-10-21 18:30:00 UTC+8 is 10:30:00 UTC.
        assert_eq!(
            read_message(&five),
            Some(Refusal {
                window: WindowKind::FiveHour,
                resets_at_ms: 1_792_578_600_000,
            })
        );
        let week = format!(
            "[1310][Weekly/Monthly Limit Exhausted. Your limit will reset at 2028-02-29 00:00:00][{ID}]"
        );
        assert_eq!(
            read_message(&week).map(|r| r.window),
            Some(WindowKind::Weekly { anchor_ms: None })
        );
    }

    #[test]
    fn anything_else_is_refused() {
        let time = "2026-10-21 18:30:00";
        for message in [
            String::new(),
            "rate limited".to_owned(),
            // The codes swapped with the other window's words.
            format!(
                "[1310][Usage limit reached for 5 hour. Your limit will reset at {time}][{ID}]"
            ),
            format!(
                "[1308][Weekly/Monthly Limit Exhausted. Your limit will reset at {time}][{ID}]"
            ),
            // Another code, other words, no id, a bad id, more after.
            format!(
                "[1305][Usage limit reached for 5 hour. Your limit will reset at {time}][{ID}]"
            ),
            format!("[1308][使用上限已达到. Your limit will reset at {time}][{ID}]"),
            format!("[1308][Usage limit reached for 5 hour. Your limit will reset at {time}]"),
            format!("[1308][Usage limit reached for 5 hour. Your limit will reset at {time}][XYZ]"),
            format!(
                "[1308][Usage limit reached for 5 hour. Your limit will reset at {time}][{ID}] x"
            ),
            // Times that aren't.
            format!(
                "[1308][Usage limit reached for 5 hour. Your limit will reset at 2026-02-30 10:00:00][{ID}]"
            ),
            format!(
                "[1308][Usage limit reached for 5 hour. Your limit will reset at 2026-10-21 24:00:00][{ID}]"
            ),
            format!(
                "[1308][Usage limit reached for 5 hour. Your limit will reset at 2026-10-21T18:30:00][{ID}]"
            ),
            format!(
                "[1308][Usage limit reached for 5 hour. Your limit will reset at +026-10-21 18:30:00][{ID}]"
            ),
        ] {
            assert_eq!(read_message(&message), None, "{message}");
        }
    }
}
