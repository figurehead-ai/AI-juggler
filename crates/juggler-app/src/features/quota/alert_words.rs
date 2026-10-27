//! A quota alert in plain words (D-25 amendment AJ-E1025; wireframe 27):
//! "Mia has 7% left in its 5-hour window, until it resets in 1 h 10 min" —
//! never a window id (`five_hour`), a tier tag ("(Measured)") or a
//! session's id. The numbers ride along so this Mac's banner can say the
//! reset in local time and offer Hand over and Queue for the reset.

use juggler_core::model::{AccountId, SessionId};
use juggler_core::storage::Storage;

/// The numbers behind a quota alert.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct AlertQuotaInfo {
    /// The account's name.
    pub account_label: String,
    /// The window (`five_hour|weekly`): data for the UI, never shown.
    pub window: String,
    /// Percent left, when measured.
    pub left_pct: Option<f64>,
    /// Minutes of work left at the current pace, when forecast.
    pub minutes_left: Option<f64>,
    /// When the window resets, epoch ms.
    pub resets_at_ms: Option<i64>,
}

/// An account's name, short enough for a banner line.
fn short_label(label: &str) -> String {
    label.chars().take(18).collect()
}

/// A window, as people say it.
fn window_words(slot: &str) -> &'static str {
    if slot == "weekly" {
        "weekly window"
    } else {
        "5-hour window"
    }
}

/// A span, as people say it: "45 min", "2 h", "1 h 10 min".
fn span_words(ms: i64) -> String {
    let minutes = (ms.max(0) + 59_999) / 60_000;
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (hours, rest) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, rest) => format!("{hours} h {rest} min"),
    }
}

/// A quota alert (`tray-amber|actionable|exhausted`) in plain words, for
/// every surface that shows the text as it is (a paired console, the
/// digest). This Mac's banner words it again with the reset in local
/// time (the backend carries no time zone).
#[must_use]
pub fn quota_alert_words(kind: &str, quota: &AlertQuotaInfo, now: i64) -> String {
    let who = short_label(&quota.account_label);
    let window = window_words(&quota.window);
    let reset = quota
        .resets_at_ms
        .filter(|at| *at > now)
        .map(|at| span_words(at - now));
    match kind {
        "tray-amber" => match quota.minutes_left {
            #[allow(clippy::cast_possible_truncation)] // minutes, rounded
            Some(minutes) => format!(
                "{who} has about {} of work left in its {window}",
                span_words((minutes.max(1.0) * 60_000.0).round() as i64)
            ),
            None => format!("{who} is running low in its {window}"),
        },
        "actionable" => {
            let head = match quota.left_pct {
                Some(left) => format!("{who} has {left:.0}% left in its {window}"),
                None => format!("{who} is about to run out of its {window}"),
            };
            match reset {
                Some(reset) => format!("{head}, until it resets in {reset}"),
                None => head,
            }
        }
        _ => match reset {
            Some(reset) => format!("{who} has used up its {window}; it resets in {reset}"),
            None => format!("{who} has used up its {window}"),
        },
    }
}

/// A queued resume came due, in plain words (never the session's id):
/// its account and the folder it works in.
#[must_use]
pub fn reset_reached_words(storage: &Storage, account: &str, session: &str) -> String {
    let label = storage
        .accounts()
        .get(&AccountId::new(account))
        .map_or_else(|_| "Its account".to_owned(), |account| account.label);
    let folder = storage
        .sessions()
        .get(&SessionId::new(session))
        .ok()
        .and_then(|record| {
            record
                .cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty());
    match folder {
        Some(folder) => format!("{label} has reset: the session in {folder} can pick up again"),
        None => format!("{label} has reset: its session can pick up again"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    fn quota(
        window: &str,
        left: Option<f64>,
        minutes: Option<f64>,
        reset_in_min: Option<i64>,
    ) -> AlertQuotaInfo {
        AlertQuotaInfo {
            account_label: "Mia".to_owned(),
            window: window.to_owned(),
            left_pct: left,
            minutes_left: minutes,
            resets_at_ms: reset_in_min.map(|min| NOW + min * 60_000),
        }
    }

    /// Words a person never says: window ids and tier tags.
    const INTERNAL: &[&str] = &[
        "five_hour",
        "(Measured)",
        "(Computed)",
        "(estimate)",
        "Measured",
        "Computed",
        "TTE",
        "exhausted",
    ];

    fn plain(text: &str) -> bool {
        !INTERNAL.iter().any(|token| text.contains(token))
    }

    #[test]
    fn an_actionable_alert_says_what_is_left_and_until_when() {
        let text = quota_alert_words(
            "actionable",
            &quota("five_hour", Some(7.0), None, Some(70)),
            NOW,
        );
        assert_eq!(
            text,
            "Mia has 7% left in its 5-hour window, until it resets in 1 h 10 min"
        );
    }

    #[test]
    fn every_alert_is_in_plain_words() {
        for kind in ["tray-amber", "actionable", "exhausted"] {
            for window in ["five_hour", "weekly"] {
                for (left, minutes, reset) in
                    [(Some(7.0), Some(40.0), Some(90)), (None, None, None)]
                {
                    let text = quota_alert_words(kind, &quota(window, left, minutes, reset), NOW);
                    assert!(plain(&text), "an alert names an internal id or tag");
                    assert!(text.starts_with("Mia "), "an alert leads with the account");
                }
            }
        }
    }

    #[test]
    fn a_used_up_window_says_when_it_comes_back() {
        assert_eq!(
            quota_alert_words(
                "exhausted",
                &quota("weekly", Some(0.0), None, Some(45)),
                NOW
            ),
            "Mia has used up its weekly window; it resets in 45 min"
        );
        assert_eq!(
            quota_alert_words(
                "tray-amber",
                &quota("five_hour", None, Some(40.0), None),
                NOW
            ),
            "Mia has about 40 min of work left in its 5-hour window"
        );
    }
}
