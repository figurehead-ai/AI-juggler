//! What a person reads when something fails (D-25 amendment AJ-E1025,
//! §10): plain words, in ONE place, for every [`AppError`] that reaches
//! the interface. Never an id ("account a1b2 not found"), a tool's id
//! ("no adapter for harness made-up-cli"), a code ("invalid replay: …")
//! or an OS string ("No such file or directory (os error 2)"). The
//! technical text stays in `Display`, which goes to the error log
//! ([`crate::error_log`]) and from there into Export diagnostics.

use juggler_adapter::AdapterError;
use juggler_core::CoreError;
use juggler_supervisor::SupervisorError;

use crate::error::AppError;

/// Something that can't be said better: nothing changed, try again.
const TRY_AGAIN: &str = "Something went wrong, so nothing changed. Try again.";
/// A file AI Juggler needed couldn't be read or written.
const FILE: &str = "AI Juggler couldn't read or save a file it needed. Try again.";

/// The words for `error`.
#[must_use]
pub fn user_words(error: &AppError) -> String {
    match error {
        AppError::Core(core) => core_words(core),
        AppError::Supervisor(supervisor) => supervisor_words(supervisor),
        AppError::Adapter(adapter) => adapter_words(adapter),
        AppError::NotFound { entity, .. } => gone_words(entity),
        AppError::UnknownHarness(_) => {
            "AI Juggler doesn't support that tool yet, so nothing was started.".to_owned()
        }
        AppError::AuditUnavailable => {
            "Nothing was stopped: AI Juggler couldn't write the record it \
                                       keeps of every stop. Try again in a moment."
                .to_owned()
        }
        AppError::Validation { message, .. } => refusal_words(message),
        // Many commands explain a refusal here in the app's own words
        // ("This session has no conversation to replay yet."); an OS
        // string ("… (os error 2)") never shows.
        AppError::Io { message, .. } if !looks_internal(message) && !message.trim().is_empty() => {
            sentence(message)
        }
        AppError::Io { .. } => FILE.to_owned(),
        AppError::StatePoisoned => {
            "Part of AI Juggler stopped working. Quit and reopen AI Juggler to fix it.".to_owned()
        }
        // Written for people already (AJ-E1001, AJ-E307, AJ-E1501).
        AppError::IdentityMismatch { .. }
        | AppError::AlreadyImported { .. }
        | AppError::Refused(_) => error.to_string(),
        // The failure's own words (`SwitchFailure::words`), the accounts
        // named; its detail is the `Display` the log keeps.
        AppError::Switch { words, .. } => words.clone(),
    }
}

fn core_words(error: &CoreError) -> String {
    match error {
        CoreError::SchemaTooNew { .. } => {
            "This data was saved by a newer AI Juggler. Update AI Juggler to open it.".to_owned()
        }
        CoreError::NotADatabase { .. } => "That file isn't AI Juggler's data.".to_owned(),
        CoreError::NotFound { entity, .. } => gone_words(entity),
        CoreError::Validation { message, .. } => refusal_words(message),
        // The storage's class, as usage statistics read it (never its text).
        _ => match error.stats_code() {
            Some("db_locked") => {
                "AI Juggler's data is busy right now. Try again in a moment.".to_owned()
            }
            Some("permission_denied") => {
                "AI Juggler isn't allowed to save its data in its folder.".to_owned()
            }
            Some("db_corrupt") => {
                "AI Juggler's data couldn't be read: it may be damaged. Try again.".to_owned()
            }
            _ => "AI Juggler couldn't read or save its data. Try again.".to_owned(),
        },
    }
}

fn supervisor_words(error: &SupervisorError) -> String {
    match error {
        SupervisorError::Spawn { .. } => {
            "The tool didn't start. Check that it's installed, then try again.".to_owned()
        }
        SupervisorError::UnknownSession(_) => "That session isn't running any more.".to_owned(),
        SupervisorError::WrongMachine { .. } => {
            "That runs on another computer, so AI Juggler won't touch it from here.".to_owned()
        }
        SupervisorError::Unverifiable { .. } => "Nothing was stopped: that process may not be the \
                                                 one AI Juggler started any more."
            .to_owned(),
        SupervisorError::LedgerDegraded => "AI Juggler can't keep track of processes right now, so \
                                            it won't stop any. Quit and reopen AI Juggler to fix it."
            .to_owned(),
        SupervisorError::NotATty { .. } => "That session has no terminal to type into.".to_owned(),
        SupervisorError::Journal { .. } => {
            "AI Juggler couldn't save the session's record. Try again.".to_owned()
        }
        _ => FILE.to_owned(),
    }
}

fn adapter_words(error: &AdapterError) -> String {
    match error {
        AdapterError::NotFound(_) => {
            "AI Juggler can't find that tool on this computer. Install it, then try again."
                .to_owned()
        }
        AdapterError::VersionDrift { version, .. } => {
            format!("This version of the tool ({version}) hasn't been checked with AI Juggler yet.")
        }
        // Adapters write a plain reason here ("not a Codex home");
        // an OS string never shows.
        AdapterError::AccountDir { message, .. } | AdapterError::Unsupported(message)
            if !looks_internal(message) =>
        {
            sentence(message)
        }
        AdapterError::AccountDir { .. } => {
            "AI Juggler couldn't set up the account's folder. Try again.".to_owned()
        }
        AdapterError::InvalidLaunch(message) => refusal_words(message),
        AdapterError::Unsupported(_) => "That tool can't do this.".to_owned(),
        AdapterError::Watch(_) => {
            "AI Juggler couldn't follow the tool's files. Try again.".to_owned()
        }
        _ => FILE.to_owned(),
    }
}

/// Something referred to is gone: what it was, in the app's words.
fn gone_words(entity: &str) -> String {
    let (thing, plural) = match entity {
        "account" => ("That account", false),
        "profile" | "launch settings" => ("Those launch settings", true),
        "session" | "instance" | "parked session" => ("That session", false),
        "project" => ("That project", false),
        "paired machine" => ("That computer", false),
        "work item" => ("That work", false),
        "switch edge" | "switch destination" | "cross-machine switch" | "divergence" => {
            ("That switch", false)
        }
        "ledger row" => ("That process's record", false),
        "cap" => ("That limit", false),
        _ => ("That", false),
    };
    let verb = if plural { "aren't" } else { "isn't" };
    format!("{thing} {verb} here any more: it may have been removed. Nothing changed.")
}

/// A refusal written by a command ("only a session that has ended can be
/// archived — stop it first"): said as it is, as a sentence. A message
/// that reads like a program's ("unknown mode \"x\"") never shows.
fn refusal_words(message: &str) -> String {
    if looks_internal(message) || message.trim().is_empty() {
        TRY_AGAIN.to_owned()
    } else {
        sentence(message)
    }
}

/// Capitalised, ending in a full stop.
fn sentence(message: &str) -> String {
    let text = message.trim();
    let mut chars = text.chars();
    let mut out: String = chars
        .next()
        .map(|first| first.to_uppercase().collect())
        .unwrap_or_default();
    out.push_str(chars.as_str());
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// Words from a program, not a person: quoting, code marks, task ids,
/// or a `snake_case` word (a file's path is fine: the user picked it).
#[must_use]
pub fn looks_internal(text: &str) -> bool {
    text.contains(['`', '{', '}', '"', '§'])
        || text.contains("::")
        || text.contains("AJ-E")
        || text.contains("os error")
        || text.contains("error:")
        || text.starts_with("unknown ")
        || text
            .split_whitespace()
            .filter(|word| !word.contains(['/', '~', '.']))
            .any(|word| {
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .split('_')
                    .filter(|part| !part.is_empty())
                    .count()
                    > 1
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_handover::pipeline::{PreflightStop, SwitchFailure};

    /// What a person never reads (D-25): ids, codes and OS strings.
    const INTERNAL: &[&str] = &[
        "not found",
        "invalid ",
        "harness",
        "adapter",
        "os error",
        "internal state",
        "no audit, no kill",
        "made-up-cli",
        "acct-7f3a",
        "five_hour",
        "AJ-E",
    ];

    fn plain(text: &str) -> bool {
        !INTERNAL.iter().any(|token| text.contains(token))
    }

    fn every_error() -> Vec<AppError> {
        let io = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No such file or directory (os error 2)",
        );
        vec![
            AppError::NotFound {
                entity: "account",
                id: "acct-7f3a".into(),
            },
            AppError::NotFound {
                entity: "launch settings",
                id: "acct-7f3a".into(),
            },
            AppError::NotFound {
                entity: "startup recovery pass (AJ-E510 §11.2)",
                id: "acct-7f3a".into(),
            },
            AppError::UnknownHarness("made-up-cli".into()),
            AppError::AuditUnavailable,
            AppError::Validation {
                subject: "replay",
                message: "it is still live — its terminal shows it as it runs".into(),
            },
            AppError::Validation {
                subject: "usage stats",
                message: "unknown mode \"five_hour\"".into(),
            },
            AppError::Io {
                context: "replay transcript",
                message: io.to_string(),
            },
            AppError::StatePoisoned,
            AppError::Core(CoreError::NotFound {
                entity: "session",
                id: "acct-7f3a".into(),
            }),
            AppError::Core(CoreError::Validation {
                subject: "x",
                message: "use 1–64 characters".into(),
            }),
            AppError::Core(CoreError::Io("filesystem: os error 28".into())),
            AppError::Supervisor(SupervisorError::LedgerDegraded),
            AppError::Supervisor(SupervisorError::Io(io)),
            AppError::Adapter(AdapterError::NotFound("made-up-cli".into())),
            AppError::Adapter(AdapterError::InvalidLaunch("model_id is missing".into())),
            AppError::switch(
                SwitchFailure::LaunchFailed("terminal spawn failed".into()),
                "Work",
                "Personal",
            ),
        ]
    }

    #[test]
    fn every_error_reads_in_plain_words() {
        for error in every_error() {
            let words = user_words(&error);
            assert!(
                plain(&words),
                "an error's words carry an id, a code or an OS string"
            );
            assert!(
                words.ends_with(['.', '!', '?']),
                "an error's words are a sentence"
            );
            assert_eq!(
                error.wire().user_message,
                words,
                "the wire carries the words"
            );
        }
    }

    #[test]
    fn a_command_s_own_refusal_is_said_as_written() {
        let error = AppError::Validation {
            subject: "archive",
            message: "only a session that has ended can be archived — stop it first".into(),
        };
        assert_eq!(
            user_words(&error),
            "Only a session that has ended can be archived — stop it first."
        );
    }

    #[test]
    fn a_program_s_words_are_told_apart_from_a_person_s() {
        assert!(looks_internal("unknown answer \"x\""));
        assert!(looks_internal("model_id is missing"));
        assert!(!looks_internal(
            "/Users/me/my_backup.zip already exists — choose a new file name"
        ));
        assert!(!looks_internal("That machine isn't connected right now."));
    }

    /// Every switch failure, as the switch dialog gets it (Work → Personal).
    fn switch_failures() -> Vec<(&'static str, SwitchFailure)> {
        vec![
            ("unresolved-source", SwitchFailure::UnresolvedSource),
            ("torn-tail", SwitchFailure::TornTail),
            (
                "collision-forked",
                SwitchFailure::CollisionForked("sess-9f3a".into()),
            ),
            (
                "staging-aborted",
                SwitchFailure::StagingAborted("No space left on device (os error 28)".into()),
            ),
            (
                "launch-failed",
                SwitchFailure::LaunchFailed("terminal spawn failed".into()),
            ),
            ("prime-quota-death", SwitchFailure::PrimeQuotaDeath(12_400)),
            ("original-session-gone", SwitchFailure::OriginalSessionGone),
            ("merge-conflict-open", SwitchFailure::MergeConflictOpen),
            ("hop-cost", SwitchFailure::HopCost(4, 380_000)),
            (
                "preflight-target-exhausted",
                SwitchFailure::Preflight(PreflightStop::TargetExhausted),
            ),
            (
                "preflight-version-drift",
                SwitchFailure::Preflight(PreflightStop::VersionDrift),
            ),
            (
                "adapter",
                SwitchFailure::Adapter(AdapterError::InvalidLaunch("model_id is missing".into())),
            ),
        ]
    }

    #[test]
    fn a_switch_failure_says_what_happened_and_the_log_keeps_the_detail() {
        let error = AppError::switch(
            SwitchFailure::LaunchFailed("terminal spawn failed".into()),
            "Work",
            "Personal",
        );
        assert_eq!(
            error.wire().user_message,
            "The new session on Personal didn't start, so nothing moved."
        );
        assert_eq!(error.wire().code, "switch_failed");
        assert!(error.wire().retryable);
        let logged = error.to_string();
        assert!(
            logged.contains("terminal spawn failed") && logged.contains("S-F5"),
            "the log keeps the failure's detail and its code"
        );
    }

    /// The words every switch failure reaches the screen with, committed
    /// for the interface's guard (`ui/src/test/switch-failure-words.json`,
    /// read by `plain-words-on-screen.test.tsx`); regenerate with
    /// `UPDATE_GOLDENS=1`.
    #[test]
    fn switch_failure_words_are_committed_for_the_interface() {
        let words: Vec<serde_json::Value> = switch_failures()
            .into_iter()
            .map(|(kind, failure)| {
                let error = AppError::switch(failure, "Work", "Personal");
                serde_json::json!({ "kind": kind, "words": error.wire().user_message })
            })
            .collect();
        let mut snapshot = serde_json::to_string_pretty(&words).unwrap();
        snapshot.push('\n');
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/test/switch-failure-words.json");
        if std::env::var("UPDATE_GOLDENS").is_ok() {
            std::fs::write(&path, &snapshot).unwrap();
        }
        let committed = std::fs::read_to_string(&path).expect("run with UPDATE_GOLDENS=1 once");
        assert_eq!(
            committed, snapshot,
            "a switch failure's words changed: regenerate the interface's copy"
        );
        for (_, failure) in switch_failures() {
            let words = AppError::switch(failure, "Work", "Personal")
                .wire()
                .user_message;
            assert!(
                !looks_internal(&words),
                "a switch failure's words look internal"
            );
        }
    }
}
