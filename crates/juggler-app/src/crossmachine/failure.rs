//! What a person reads when a switch to or from another computer stops
//! (D-25): what happened, then ONE next step, with the other computer
//! named by its label. The switch dialogs show these words under a short
//! title and add nothing, so each thing is said once.
//!
//! The other computer's own words for a refusal are never shown: they
//! speak from its side ("this computer", "here"). Its reason code picks
//! the words here; its text goes to the error log.

/// Which way the work was going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// To the other computer (the switch).
    To,
    /// Back to the original on the other computer (switch-back).
    Back,
}

/// Why a switch to or from another computer stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossFailure {
    // ------------------------------------------- on this computer
    /// The session isn't in a project.
    NoProject,
    /// AI Juggler couldn't tell which file holds the conversation.
    Unresolved,
    /// The handover couldn't be got ready.
    NotReady,
    /// The tool can't move this same conversation.
    CantTransplant,
    /// The destination's tool isn't installed there.
    NotInstalled,
    /// The destination isn't offered any more.
    DestinationGone,
    /// Link is off on this computer.
    LinkOff,
    /// The other computer is offline.
    Offline,
    /// It couldn't be reached while sending.
    Unreachable,
    /// The person stopped sending.
    Cancelled,
    // ------------------------------- the other computer's refusals
    /// It doesn't know this computer (any more).
    NotPaired,
    /// It lets this computer view only.
    ViewOnly,
    /// It runs another AI Juggler release.
    OtherRelease,
    /// It is still taking another handover from this computer.
    Busy,
    /// Switch-back: the original still runs there.
    OriginalRunning,
    /// Too many requests at once.
    TooManyRequests,
    /// It has only just connected (its clock isn't measured yet).
    JustConnected,
    /// The two clocks are too far apart.
    ClocksApart,
    /// The request took too long (or arrived twice).
    Expired,
    /// It couldn't read what was sent.
    Unreadable,
    /// What was sent is over its size limit.
    TooLarge {
        /// The same conversation was sent (a fresh start sends less).
        transplant: bool,
    },
    /// The account's Launch settings or project aren't there any more.
    MissingThere,
    /// Switch-back: the original isn't there any more.
    OriginalMissing,
    /// Switch-back: the original couldn't be read there.
    OriginalUnreadable,
    /// It can't continue this same conversation (another tool there).
    OtherTool,
    /// Its version of the tool differs from this computer's.
    OtherVersion,
    /// It couldn't set the conversation up.
    SetUpFailed,
    /// The session there didn't start.
    StartFailed,
    /// The account there is signed in as someone else.
    SignedInElse,
    /// Anything else that went wrong there.
    WentWrongThere,
}

/// Every kind, for the tests and the interface's committed copy.
#[cfg(test)]
pub const ALL: &[(&str, CrossFailure)] = &[
    ("no-project", CrossFailure::NoProject),
    ("unresolved", CrossFailure::Unresolved),
    ("not-ready", CrossFailure::NotReady),
    ("cant-transplant", CrossFailure::CantTransplant),
    ("not-installed", CrossFailure::NotInstalled),
    ("destination-gone", CrossFailure::DestinationGone),
    ("link-off", CrossFailure::LinkOff),
    ("offline", CrossFailure::Offline),
    ("unreachable", CrossFailure::Unreachable),
    ("cancelled", CrossFailure::Cancelled),
    ("not-paired", CrossFailure::NotPaired),
    ("view-only", CrossFailure::ViewOnly),
    ("other-release", CrossFailure::OtherRelease),
    ("busy", CrossFailure::Busy),
    ("original-running", CrossFailure::OriginalRunning),
    ("too-many-requests", CrossFailure::TooManyRequests),
    ("just-connected", CrossFailure::JustConnected),
    ("clocks-apart", CrossFailure::ClocksApart),
    ("expired", CrossFailure::Expired),
    ("unreadable", CrossFailure::Unreadable),
    (
        "too-large-same-conversation",
        CrossFailure::TooLarge { transplant: true },
    ),
    (
        "too-large-fresh-start",
        CrossFailure::TooLarge { transplant: false },
    ),
    ("missing-there", CrossFailure::MissingThere),
    ("original-missing", CrossFailure::OriginalMissing),
    ("original-unreadable", CrossFailure::OriginalUnreadable),
    ("other-tool", CrossFailure::OtherTool),
    ("other-version", CrossFailure::OtherVersion),
    ("set-up-failed", CrossFailure::SetUpFailed),
    ("start-failed", CrossFailure::StartFailed),
    ("signed-in-else", CrossFailure::SignedInElse),
    ("went-wrong-there", CrossFailure::WentWrongThere),
];

/// The fresh-start mechanism, by its name in the wizard.
const FRESH_START: &str = "Use \u{201c}Full story, fresh start\u{201d} instead.";

impl CrossFailure {
    /// The other computer's refusal, by its reason code (`transplant`:
    /// the same conversation was sent).
    #[must_use]
    pub fn from_code(code: &str, direction: Direction, transplant: bool) -> Self {
        let back = direction == Direction::Back;
        match code {
            "cancelled" | "handover_cancelled" => Self::Cancelled,
            "not_authorised" | "not_paired" | "wrong_console" | "machine_mismatch" => {
                Self::NotPaired
            }
            "permission_denied" | "no_access" => Self::ViewOnly,
            "degraded" | "version" => Self::OtherRelease,
            "busy" if back => Self::OriginalRunning,
            "busy" => Self::Busy,
            "rate_limited" => Self::TooManyRequests,
            "clock_unmeasured" => Self::JustConnected,
            "clock_skew" => Self::ClocksApart,
            "expired" | "replayed" => Self::Expired,
            "malformed" | "corrupt" => Self::Unreadable,
            "too_large" => Self::TooLarge { transplant },
            "unknown_record" if back => Self::OriginalMissing,
            "unknown_record" => Self::MissingThere,
            "original_gone" => Self::OriginalUnreadable,
            "provider_mismatch" | "unsupported" => Self::OtherTool,
            "version_mismatch" => Self::OtherVersion,
            "install_failed" => Self::SetUpFailed,
            "launch_failed" => Self::StartFailed,
            "identity_mismatch" => Self::SignedInElse,
            _ => Self::WentWrongThere,
        }
    }

    /// What a person reads: what happened, then one next step. `machine`
    /// is the other computer's label, `tool` the tool's name.
    #[must_use]
    pub fn words(self, machine: &str, tool: &str) -> String {
        let m = machine;
        match self {
            Self::NoProject => "This session isn't in a project, so nothing moved. Start the work from a project to move it to another computer.".to_owned(),
            Self::Unresolved => "AI Juggler couldn't tell which file holds this conversation, so nothing moved. Resume it on this computer where you left it.".to_owned(),
            Self::NotReady => {
                "AI Juggler couldn't get the handover ready, so nothing was sent. Try again."
                    .to_owned()
            }
            Self::CantTransplant => {
                format!("{tool} can't move this same conversation, so nothing moved. {FRESH_START}")
            }
            Self::NotInstalled => format!(
                "{tool} isn't installed on {m}, so nothing moved. Install it on {m}, then try again."
            ),
            Self::DestinationGone => format!(
                "That account on {m} isn't offered any more, so nothing moved. Pick another destination."
            ),
            Self::LinkOff => "Link is off on this computer, so nothing was sent. Turn Link on in Fleet, then try again.".to_owned(),
            Self::Offline => format!(
                "{m} is offline, so nothing was sent. Check that {m} is on with AI Juggler open, then try again."
            ),
            Self::Unreachable => format!(
                "{m} couldn't be reached, so nothing was sent. Check that {m} is on and connected, then try again."
            ),
            Self::Cancelled => "You stopped sending, so nothing changed on either computer. Switch again when you're ready.".to_owned(),
            Self::NotPaired => format!(
                "{m} doesn't recognise this computer any more, so nothing moved. Pair the two again in Fleet, then try again."
            ),
            Self::ViewOnly => format!(
                "{m} lets this computer view only, so nothing moved. Give this computer View + control in Fleet on {m}, then try again."
            ),
            Self::OtherRelease => format!(
                "{m} runs another release of AI Juggler, so nothing moved. Update AI Juggler on both computers, then try again."
            ),
            Self::Busy => format!(
                "{m} is still taking another handover from this computer, so nothing moved. Wait for it to finish, then try again."
            ),
            Self::OriginalRunning => format!(
                "The original is still running on {m}, so nothing moved. Stop it on {m}, then try again."
            ),
            Self::TooManyRequests => format!(
                "{m} got too many requests at once, so nothing moved. Wait a moment, then try again."
            ),
            Self::JustConnected => format!(
                "{m} has only just connected, so nothing moved. Wait a few seconds, then try again."
            ),
            Self::ClocksApart => format!(
                "The clocks on this computer and {m} are more than 5 minutes apart, so nothing moved. Turn on automatic time on both computers, then try again."
            ),
            Self::Expired => {
                format!("The request took too long to reach {m}, so nothing moved. Try again.")
            }
            Self::Unreadable => {
                format!("{m} couldn't read what was sent, so nothing moved. Try again.")
            }
            Self::TooLarge { transplant: true } => format!(
                "This conversation is too large to send to {m}, so nothing moved. {FRESH_START}"
            ),
            Self::TooLarge { transplant: false } => format!(
                "This conversation is too large to send to {m}, so nothing moved. Start a new session on {m} instead."
            ),
            Self::MissingThere => format!(
                "That account or project isn't on {m} any more, so nothing moved. Pick another destination."
            ),
            Self::OriginalMissing => format!(
                "The original isn't on {m} any more, so nothing moved. Keep working here."
            ),
            Self::OriginalUnreadable => format!(
                "The original on {m} couldn't be read, so nothing moved. Keep working here."
            ),
            Self::OtherTool => format!(
                "{m} can't continue this same conversation, so nothing moved. {FRESH_START}"
            ),
            Self::OtherVersion => format!(
                "{tool} on {m} isn't the same version as on this computer, so nothing moved. {FRESH_START}"
            ),
            Self::SetUpFailed => format!(
                "{m} couldn't set up the conversation, so nothing moved. Try again, or use \u{201c}Full story, fresh start\u{201d} instead."
            ),
            Self::StartFailed => format!(
                "The session on {m} didn't start, so nothing moved. Check that {tool} works on {m}, then try again."
            ),
            Self::SignedInElse => format!(
                "The account on {m} is signed in as someone else now, so nothing moved. Sign it back in on {m}, then try again."
            ),
            Self::WentWrongThere => {
                format!("Something went wrong on {m}, so nothing moved. Try again.")
            }
        }
    }

    /// As the typed error a command returns (its words shown as they are;
    /// a retry can succeed).
    #[must_use]
    pub fn error(self, machine: &str, tool: &str) -> crate::error::AppError {
        crate::error::AppError::Io {
            context: "hand over",
            message: self.words(machine, tool),
        }
    }
}

/// The words a refusal from the other computer reaches the person with;
/// its own text goes to the error log.
#[must_use]
pub fn refusal_words(
    code: &str,
    their_text: &str,
    direction: Direction,
    transplant: bool,
    machine: &str,
    tool: &str,
) -> String {
    crate::error_log::record("hand_over", &format!("{code}: {their_text}"));
    CrossFailure::from_code(code, direction, transplant).words(machine, tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The verbs a next step starts with.
    const NEXT: &[&str] = &[
        "Try", "Check", "Pair", "Give", "Update", "Wait", "Use", "Turn", "Pick", "Sign", "Stop",
        "Install", "Keep", "Resume", "Switch", "Start",
    ];

    /// The sentences of `words` (a sentence ends with a full stop and a
    /// space, or the end).
    fn sentences(words: &str) -> Vec<&str> {
        words
            .split_inclusive(". ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect()
    }

    #[test]
    fn every_failure_says_what_happened_then_one_next_step() {
        for (kind, failure) in ALL {
            let words = failure.words("Studio", "Claude Code");
            let parts = sentences(&words);
            assert_eq!(
                parts.len(),
                2,
                "{kind}: what happened, then one next step: {words:?}"
            );
            assert!(
                parts[0].contains(", so "),
                "{kind}: the first sentence says what happened and what it meant: {words:?}"
            );
            let verb = parts[1].split_whitespace().next().unwrap_or_default();
            assert!(
                NEXT.contains(&verb),
                "{kind}: the second sentence is a next step: {words:?}"
            );
            assert!(words.ends_with('.'), "{kind}: {words:?}");
            assert!(
                !crate::error_words::looks_internal(&words),
                "{kind}: looks internal: {words:?}"
            );
            assert!(
                !words.contains("Mac"),
                "{kind}: computers aren't all Macs: {words:?}"
            );
            assert_eq!(
                failure.error("Studio", "Claude Code").wire().user_message,
                words,
                "{kind}: the error's words are these, as they are"
            );
        }
    }

    #[test]
    fn the_other_computer_is_named_wherever_it_is_meant() {
        for (kind, failure) in ALL {
            let words = failure.words("Studio", "Claude Code");
            let about_here = matches!(
                failure,
                CrossFailure::NoProject
                    | CrossFailure::Unresolved
                    | CrossFailure::NotReady
                    | CrossFailure::CantTransplant
                    | CrossFailure::LinkOff
                    | CrossFailure::Cancelled
            );
            assert_eq!(
                words.contains("Studio"),
                !about_here,
                "{kind}: names the other computer only when it's about it: {words:?}"
            );
        }
    }

    #[test]
    fn every_reason_code_the_other_computer_sends_has_its_words() {
        let to = |code| CrossFailure::from_code(code, Direction::To, true);
        let back = |code| CrossFailure::from_code(code, Direction::Back, false);
        assert_eq!(to("not_authorised"), CrossFailure::NotPaired);
        assert_eq!(to("permission_denied"), CrossFailure::ViewOnly);
        assert_eq!(to("degraded"), CrossFailure::OtherRelease);
        assert_eq!(to("busy"), CrossFailure::Busy);
        assert_eq!(back("busy"), CrossFailure::OriginalRunning);
        assert_eq!(to("clock_skew"), CrossFailure::ClocksApart);
        assert_eq!(to("too_large"), CrossFailure::TooLarge { transplant: true });
        assert_eq!(to("unknown_record"), CrossFailure::MissingThere);
        assert_eq!(back("unknown_record"), CrossFailure::OriginalMissing);
        assert_eq!(back("original_gone"), CrossFailure::OriginalUnreadable);
        assert_eq!(to("version_mismatch"), CrossFailure::OtherVersion);
        assert_eq!(to("launch_failed"), CrossFailure::StartFailed);
        assert_eq!(to("identity_mismatch"), CrossFailure::SignedInElse);
        assert_eq!(to("internal"), CrossFailure::WentWrongThere);
        assert_eq!(
            to("a-code-from-a-newer-release"),
            CrossFailure::WentWrongThere
        );
        // Its own text speaks from its side: never shown.
        let words = refusal_words(
            "permission_denied",
            "View-only pairing — change it in Fleet on this Mac.",
            Direction::To,
            true,
            "Studio",
            "Claude Code",
        );
        assert_eq!(
            words,
            "Studio lets this computer view only, so nothing moved. Give this computer View + control in Fleet on Studio, then try again."
        );
    }

    /// The words of every kind, committed for the interface's guard
    /// (`ui/src/test/cross-switch-failure-words.json`, read by
    /// `plain-words-on-screen.test.tsx`); regenerate with
    /// `UPDATE_GOLDENS=1`.
    #[test]
    fn the_words_are_committed_for_the_interface() {
        let words: Vec<serde_json::Value> = ALL
            .iter()
            .map(|(kind, failure)| {
                serde_json::json!({ "kind": kind, "words": failure.words("Studio", "Claude Code") })
            })
            .collect();
        let mut snapshot = serde_json::to_string_pretty(&words).unwrap();
        snapshot.push('\n');
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/test/cross-switch-failure-words.json");
        if std::env::var("UPDATE_GOLDENS").is_ok() {
            std::fs::write(&path, &snapshot).unwrap();
        }
        let committed = std::fs::read_to_string(&path).expect("run with UPDATE_GOLDENS=1 once");
        assert_eq!(
            committed, snapshot,
            "a cross-machine failure's words changed: regenerate the interface's copy"
        );
    }
}
