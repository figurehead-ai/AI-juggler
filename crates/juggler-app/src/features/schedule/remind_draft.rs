//! Bot's "remind me at…" (ADR-0039 §4, §11.1): code reads the request's
//! time and words into a typed draft; no model picks anything, and only
//! the user's click on the draft sets the reminder (the watches' draft and
//! confirm). What it reads, English and German:
//!
//! - **When:** "in 20 minutes", "in 2 hours", "in half an hour", "in an
//!   hour"; "at 5 pm", "at 17:30", "um 17 Uhr", each the next such time
//!   (today, else tomorrow), or "tomorrow at 9"; no time said leaves it
//!   for the user to pick.
//! - **What:** the words after "remind me to" ("check the deploy"), else
//!   what it is about ("Look at “api”"), else "Your reminder".

use jiff::civil::Time;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use super::trigger::{self, CalendarRule, ClockTime};
use crate::features::watches::draft::{number_words, request_times, words};
use crate::features::watches::model::WatchTarget;

/// The longest title a draft takes from the request.
const TITLE_MAX: usize = 120;

/// The reminder Bot drafted: the user checks it and clicks to set it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReminderDraftInfo {
    /// When, epoch ms; none when the request said no time.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// Its words.
    pub title: String,
    /// What it is about.
    #[ts(optional = nullable)]
    pub about: Option<WatchTarget>,
    /// The Bot turn it came from (set on the turn).
    #[ts(optional = nullable)]
    pub chat_turn: Option<String>,
}

/// The words ask for a reminder ("remind me…", "erinnere mich…").
#[must_use]
pub fn is_reminder_request(text: &str) -> bool {
    words(text)
        .iter()
        .any(|w| w.starts_with("remind") || w.starts_with("erinner"))
}

fn minutes_word(w: &str) -> Option<i64> {
    match w {
        "m" | "min" | "mins" | "minute" | "minutes" | "minute(s)" | "minuten" | "minute." => {
            Some(1)
        }
        "h" | "hr" | "hrs" | "hour" | "hours" | "stunde" | "stunden" => Some(60),
        _ => None,
    }
}

/// A number the request says: digits, or the watches' number words.
fn number(w: &str) -> Option<i64> {
    w.parse::<i64>().ok().or_else(|| {
        number_words()
            .iter()
            .find(|(k, _)| *k == w || (w == "einer" && *k == "eins"))
            .map(|(_, n)| i64::from(*n))
    })
}

/// A delay in minutes, in bounds (a minute to 30 days).
fn bounded(minutes: i64) -> Option<i64> {
    (1..=60 * 24 * 30).contains(&minutes).then_some(minutes)
}

/// "20min": digits and the unit in one word.
fn glued(word: &str) -> Option<i64> {
    let digits: String = word.chars().take_while(char::is_ascii_digit).collect();
    bounded(digits.parse::<i64>().ok()? * minutes_word(&word[digits.len()..])?)
}

/// "in 20 minutes", "in 2 hours", "in half an hour", "in an hour" (and
/// German): the delay in minutes.
#[must_use]
pub fn delay_minutes(text: &str) -> Option<i64> {
    let w = words(text);
    for (i, x) in w.iter().enumerate() {
        if x != "in" {
            continue;
        }
        let rest: Vec<&str> = w[i + 1..].iter().take(4).map(String::as_str).collect();
        match rest.as_slice() {
            ["half", "an", "hour", ..] | ["einer", "halben", "stunde", ..] => return Some(30),
            ["an", "hour", ..] | ["einer", "stunde", ..] => return Some(60),
            [n, unit, ..] => {
                if let Some(minutes) = number(n)
                    .zip(minutes_word(unit))
                    .and_then(|(n, per)| bounded(n * per))
                    .or_else(|| glued(n))
                {
                    return Some(minutes);
                }
            }
            [n] => {
                if let Some(minutes) = glued(n) {
                    return Some(minutes);
                }
            }
            _ => {}
        }
    }
    None
}

/// When the request asks to be reminded, from `now_ms` in `zone`.
#[must_use]
pub fn when(text: &str, now_ms: i64, zone: &TimeZone) -> Option<i64> {
    if let Some(minutes) = delay_minutes(text) {
        return Some(now_ms + minutes * trigger::MINUTE_MS);
    }
    let (h, m) = request_times(text)
        .first()
        .and_then(|hm| hm.split_once(':'))
        .and_then(|(h, m)| Some((h.parse::<u8>().ok()?, m.parse::<u8>().ok()?)))?;
    let w = words(text);
    let tomorrow = w.iter().any(|x| x == "tomorrow" || x == "morgen");
    if tomorrow {
        let day = trigger::local(now_ms, zone)?.date().tomorrow().ok()?;
        let time = Time::new(i8::try_from(h).ok()?, i8::try_from(m).ok()?, 0, 0).ok()?;
        return trigger::instant_of(day.to_datetime(time), zone);
    }
    trigger::calendar_after(
        &CalendarRule::Daily {
            at: ClockTime::new(h, m),
        },
        now_ms,
        zone,
    )
}

/// Where the request's time phrase starts (" at 5", " in 20", " in an
/// hour", " tomorrow", " um 17", " morgen", " today"), so "look at api"
/// keeps its own "at".
fn time_phrase_at(lower: &str) -> Option<usize> {
    let numbered = |cue: &str| {
        lower.match_indices(cue).find_map(|(at, _)| {
            let next = &lower[at + cue.len()..];
            let timed = next.starts_with(|c: char| c.is_ascii_digit())
                || [
                    "an ", "a ", "half ", "einer ", "one ", "two ", "ten ", "twenty ", "thirty ",
                ]
                .iter()
                .any(|w| next.starts_with(w));
            timed.then_some(at)
        })
    };
    let plain = |cue: &str| lower.find(cue);
    [
        numbered(" at "),
        numbered(" in "),
        numbered(" um "),
        plain(" tomorrow"),
        plain(" morgen"),
        plain(" today"),
        plain(" heute"),
    ]
    .into_iter()
    .flatten()
    .min()
}

/// The reminder's words: what follows "remind me to", with its time
/// phrase left out; `None` when the request names nothing to do.
#[must_use]
pub fn what(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let start = [
        "remind me to ",
        "remind me about ",
        "erinnere mich daran, ",
        "erinnere mich an ",
    ]
    .iter()
    .find_map(|cue| lower.find(cue).map(|at| at + cue.len()))?;
    let mut rest = text.get(start..)?.trim().to_owned();
    let lowered = rest.to_lowercase();
    let cut = time_phrase_at(&lowered);
    if let Some(cut) = cut.filter(|at| rest.is_char_boundary(*at)) {
        rest.truncate(cut);
    }
    let rest = rest.trim().trim_end_matches(['.', '!', '?', ',']).trim();
    if rest.is_empty() {
        return None;
    }
    let mut chars = rest.chars();
    let first = chars.next()?.to_uppercase().collect::<String>();
    let title: String = (first + chars.as_str()).chars().take(TITLE_MAX).collect();
    Some(title)
}

/// The draft for a reminder request: its time, its words, what it is
/// about (`about`, with its name).
#[must_use]
pub fn draft(
    text: &str,
    now_ms: i64,
    zone: &TimeZone,
    about: Option<(WatchTarget, String)>,
) -> ReminderDraftInfo {
    let title = what(text).unwrap_or_else(|| {
        about.as_ref().map_or_else(
            || "Your reminder".to_owned(),
            |(_, name)| format!("Look at “{name}”"),
        )
    });
    ReminderDraftInfo {
        at_ms: when(text, now_ms, zone).filter(|at| *at > now_ms),
        title,
        about: about.map(|(target, _)| target),
        chat_turn: None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

    use jiff::civil::date;

    use super::*;

    fn berlin() -> TimeZone {
        TimeZone::get("Europe/Berlin").unwrap()
    }

    fn at(h: i8, mi: i8, day: i8) -> i64 {
        date(2026, 11, day)
            .at(h, mi, 0, 0)
            .to_zoned(berlin())
            .unwrap()
            .timestamp()
            .as_millisecond()
    }

    #[test]
    fn a_reminder_is_asked_in_its_own_words() {
        assert!(is_reminder_request("Remind me at 5 pm to check the deploy"));
        assert!(is_reminder_request("erinnere mich um 17 Uhr"));
        assert!(!is_reminder_request("tell me when api finishes"));
    }

    #[test]
    fn the_time_is_read_by_code() {
        let now = at(14, 10, 26);
        let z = berlin();
        assert_eq!(
            when("remind me in 20 minutes", now, &z),
            Some(now + 20 * 60_000)
        );
        assert_eq!(
            when("remind me in 2 hours", now, &z),
            Some(now + 120 * 60_000)
        );
        assert_eq!(
            when("remind me in half an hour", now, &z),
            Some(now + 30 * 60_000)
        );
        assert_eq!(when("remind me in 45min", now, &z), Some(now + 45 * 60_000));
        assert_eq!(
            when("erinnere mich in einer Stunde", now, &z),
            Some(now + 60 * 60_000)
        );
        assert_eq!(when("remind me at 5 pm", now, &z), Some(at(17, 0, 26)));
        assert_eq!(
            when("remind me at 9:30", now, &z),
            Some(at(9, 30, 27)),
            "passed today: tomorrow"
        );
        assert_eq!(
            when("remind me tomorrow at 9 am", now, &z),
            Some(at(9, 0, 27))
        );
        assert_eq!(
            when("erinnere mich morgen um 8 Uhr", now, &z),
            Some(at(8, 0, 27))
        );
        assert_eq!(
            when("remind me about api", now, &z),
            None,
            "no time: the user picks"
        );
    }

    #[test]
    fn the_words_are_the_users_own() {
        assert_eq!(
            what("Remind me to check the deploy at 5 pm").as_deref(),
            Some("Check the deploy")
        );
        assert_eq!(
            what("remind me to look at api in 20 minutes.").as_deref(),
            Some("Look at api")
        );
        assert_eq!(what("remind me at 5"), None);
        let d = draft(
            "remind me at 5 pm",
            at(14, 10, 26),
            &berlin(),
            Some((WatchTarget::Session("s1".into()), "api".into())),
        );
        assert_eq!(d.title, "Look at “api”");
        assert_eq!(d.at_ms, Some(at(17, 0, 26)));
        assert_eq!(d.about, Some(WatchTarget::Session("s1".into())));
        assert_eq!(
            draft("remind me", 0, &berlin(), None).title,
            "Your reminder"
        );
    }
}
