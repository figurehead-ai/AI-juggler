//! AJ-E1529: an account's conversations as its tool keeps them on disk —
//! the ones AI Juggler started and the ones it didn't. Each adapter knows
//! where its tool writes them and how to read one; the app only indexes
//! what the adapter returns (ADR-0019: no tool's layout in shared code).
//! Reading is read-only: nothing is ever written into a setup.

use std::path::PathBuf;

use juggler_core::model::TokenCounts;

/// What one conversation file says about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationSummary {
    /// The tool's id for it (Claude's `sessionId`, Codex's thread id):
    /// what `--resume` takes.
    pub id: String,
    /// The folder it ran in, as the tool recorded it.
    pub cwd: Option<PathBuf>,
    /// The git branch it last recorded, when it records one.
    pub git_branch: Option<String>,
    /// A title: the tool's own summary, else the first thing the user
    /// asked (one line, trimmed).
    pub title: Option<String>,
    /// First and last activity, epoch ms, from the lines' timestamps.
    pub started_ms: Option<i64>,
    /// Last activity.
    pub last_ms: Option<i64>,
    /// Tokens, each message counted once.
    pub tokens: TokenCounts,
}

/// A title from free text: its first non-empty line, at most `max` chars.
#[must_use]
pub fn title_from(text: &str, max: usize) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let mut out: String = line.chars().take(max).collect();
    if line.chars().count() > max {
        out.push('…');
    }
    Some(out)
}

/// RFC 3339 (`2026-10-21T19:18:59.391Z`) to epoch ms; `None` if not that
/// shape. Only the UTC `Z` form the tools write is read.
#[must_use]
pub fn timestamp_ms(raw: &str) -> Option<i64> {
    let bytes = raw.as_bytes();
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let num = |from: usize, to: usize| raw.get(from..to)?.parse::<i64>().ok();
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let millis = raw
        .get(19..)
        .and_then(|rest| rest.strip_prefix('.'))
        .map_or(0, |rest| {
            let digits: String = rest
                .chars()
                .take_while(char::is_ascii_digit)
                .take(3)
                .collect();
            format!("{digits:0<3}").parse::<i64>().unwrap_or(0)
        });
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis)
}

/// Where an adapter's conversation files are, and the newest-write order
/// isn't required: the index sorts.
pub type ConversationFiles = Vec<PathBuf>;

/// A running process that is one of the tool's sessions (not a helper,
/// and not a desktop app's own internals).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionProcess {
    /// The conversation its arguments resume, when they name one.
    pub resumes: Option<String>,
}

/// The value `flag` takes in `args`: the word after it (`--resume <id>`,
/// when that isn't another option), or after its `=` (`--resume=<id>`,
/// how the Claude app passes it).
#[must_use]
pub fn value_after(args: &[String], flags: &[&str]) -> Option<String> {
    args.iter().enumerate().find_map(|(index, arg)| {
        if let Some((flag, value)) = arg.split_once('=')
            && flags.contains(&flag)
        {
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if !flags.contains(&arg.as_str()) {
            return None;
        }
        args.get(index + 1)
            .filter(|next| !next.starts_with('-'))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resume_id_reads_as_two_words_or_one() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let flags = ["--resume", "-r"];
        assert_eq!(
            value_after(&args(&["claude", "--resume", "abc"]), &flags).as_deref(),
            Some("abc")
        );
        // The Claude app's own Code sessions pass it as one word.
        assert_eq!(
            value_after(&args(&["claude", "--resume=abc"]), &flags).as_deref(),
            Some("abc")
        );
        assert_eq!(
            value_after(&args(&["claude", "-r", "abc"]), &flags).as_deref(),
            Some("abc")
        );
        assert_eq!(
            value_after(&args(&["claude", "--resume", "--verbose"]), &flags),
            None
        );
        assert_eq!(value_after(&args(&["claude", "--resume="]), &flags), None);
        assert_eq!(
            value_after(&args(&["claude", "--model=opus"]), &flags),
            None
        );
        assert_eq!(
            value_after(&args(&["codex", "resume", "0199"]), &["resume"]).as_deref(),
            Some("0199")
        );
    }

    #[test]
    fn timestamps_read_as_utc_epoch_ms() {
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
        assert_eq!(timestamp_ms("yesterday"), None);
    }

    #[test]
    fn a_title_is_the_first_line_trimmed() {
        assert_eq!(
            title_from("\n  Fix the login test\nmore", 80).as_deref(),
            Some("Fix the login test")
        );
        assert_eq!(title_from("abcdef", 3).as_deref(), Some("abc…"));
        assert_eq!(title_from("   \n ", 10), None);
    }
}
