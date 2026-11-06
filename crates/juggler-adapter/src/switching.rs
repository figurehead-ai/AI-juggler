//! ADR-0028: what a switch must do, capability by capability, and each
//! adapter's answer for its tool.
//!
//! Switching is why people use AI Juggler, so no adapter may lose a
//! capability by leaving it out. Every adapter answers every capability
//! below through [`crate::HarnessAdapter::switch_capabilities`]:
//! [`SwitchAnswer::Supported`], naming how; or
//! [`SwitchAnswer::NotSupported`], with a reason a user can read (the
//! switch screen says it). [`SwitchAnswer::Unanswered`] is only the
//! trait's default, and the conformance kit (check 14) refuses an adapter
//! that leaves any capability on it. A supported capability is then
//! proved by the kit's shared family fixtures, the same for every tool.

/// One thing a switch must do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwitchCapability {
    /// A copied conversation's history is byte-identical, and a source
    /// still being written passes the prefix check.
    HistoryExact,
    /// Every subagent the conversation ran inside itself arrives, and
    /// resumes within it.
    OwnSubagents,
    /// The conversations it started (and theirs) are found, each once,
    /// and move with it.
    Family,
    /// After the move, each child's parent on the target is the one it
    /// had (the tool's own tree, not only AI Juggler's view of it).
    FamilyLinks,
    /// Every moved conversation resumes on the target by its own id.
    Resumable,
    /// The source conversation itself never changes: its files are only
    /// read. The one exception is what [`Self::AutomationHandover`]
    /// changes in the source app's own record of it, reversibly.
    SourceUntouched,
    /// What can't move (pages published under the account and the like)
    /// is listed before anything moves.
    LeftBehindNamed,
    /// Switching back carries only what happened away, for every member.
    SwitchBackDelta,
    /// What wakes the conversation by itself in its app (watching a pull
    /// request to fix it, resuming when usage resets, …) is named before
    /// the move, taken over by the copy, and turned off on the original,
    /// so two copies never work at once and nobody tidies up by hand;
    /// switching back reverses it.
    AutomationHandover,
}

impl SwitchCapability {
    /// Every capability, in the order the contract lists them.
    pub const ALL: [Self; 9] = [
        Self::HistoryExact,
        Self::OwnSubagents,
        Self::Family,
        Self::FamilyLinks,
        Self::Resumable,
        Self::SourceUntouched,
        Self::LeftBehindNamed,
        Self::SwitchBackDelta,
        Self::AutomationHandover,
    ];

    /// The capability's name in the contract (ADR-0028, SPEC §7.9).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::HistoryExact => "history_exact",
            Self::OwnSubagents => "own_subagents",
            Self::Family => "family",
            Self::FamilyLinks => "family_links",
            Self::Resumable => "resumable",
            Self::SourceUntouched => "source_untouched",
            Self::LeftBehindNamed => "left_behind_named",
            Self::SwitchBackDelta => "switch_back_delta",
            Self::AutomationHandover => "automation_handover",
        }
    }

    /// Whether every adapter whose conversations can be switched at all
    /// must support it: these are the floor of an exact move, never an
    /// exception.
    #[must_use]
    pub fn required(self) -> bool {
        matches!(
            self,
            Self::HistoryExact | Self::Resumable | Self::SourceUntouched | Self::LeftBehindNamed
        )
    }
}

/// An adapter's answer for one capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchAnswer {
    /// The trait's default: not answered. Never shipped (check 14).
    Unanswered,
    /// The tool can't, and why, in words the switch screen shows.
    NotSupported(&'static str),
    /// Supported, and how (for the manifest: "the transcript and its
    /// folder, copied whole", …).
    Supported(&'static str),
}

impl SwitchAnswer {
    /// Whether it is supported.
    #[must_use]
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Supported(_))
    }
}

/// An adapter's answers, one per capability.
pub type SwitchCapabilities = fn(SwitchCapability) -> SwitchAnswer;

/// The trait's default: every capability unanswered.
#[must_use]
pub fn unanswered(_: SwitchCapability) -> SwitchAnswer {
    SwitchAnswer::Unanswered
}

/// The answers of a tool whose conversations can't be moved into another
/// account at all: a switch starts fresh there with a handover note.
#[must_use]
pub fn fresh_start_only(_: SwitchCapability) -> SwitchAnswer {
    SwitchAnswer::NotSupported(
        "this tool can't take a conversation from another account: a switch starts a new one there, with a summary of the old",
    )
}

/// What check 14 finds wrong with `answers`, as sentences (empty when
/// the adapter keeps the contract). `switchable` is whether the adapter
/// moves conversations at all (it has handover support).
#[must_use]
pub fn contract_breaches(answers: SwitchCapabilities, switchable: bool) -> Vec<String> {
    let mut breaches = Vec::new();
    for capability in SwitchCapability::ALL {
        match answers(capability) {
            SwitchAnswer::Unanswered => breaches.push(format!(
                "{} is unanswered: say Supported (how) or NotSupported (why)",
                capability.name()
            )),
            SwitchAnswer::NotSupported(why) if why.trim().len() < 12 => breaches.push(format!(
                "{}: the reason must be a sentence a user can read, not \"{why}\"",
                capability.name()
            )),
            SwitchAnswer::NotSupported(_) if switchable && capability.required() => {
                breaches.push(format!(
                    "{} is required of every adapter that moves conversations",
                    capability.name()
                ));
            }
            SwitchAnswer::Supported(how) if how.trim().is_empty() => {
                breaches.push(format!("{}: say how it is supported", capability.name()));
            }
            SwitchAnswer::Supported(_) if !switchable => breaches.push(format!(
                "{} can't be supported without handover support",
                capability.name()
            )),
            _ => {}
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::{SwitchAnswer, SwitchCapability, contract_breaches, fresh_start_only, unanswered};

    #[test]
    fn the_default_is_refused_for_every_capability() {
        let breaches = contract_breaches(unanswered, true);
        assert_eq!(breaches.len(), SwitchCapability::ALL.len());
        assert!(breaches[0].contains("history_exact is unanswered"));
    }

    #[test]
    fn a_tool_that_cant_move_conversations_answers_with_a_reason() {
        assert!(contract_breaches(fresh_start_only, false).is_empty());
    }

    #[test]
    fn a_switchable_tool_must_support_the_floor() {
        fn no_history(capability: SwitchCapability) -> SwitchAnswer {
            match capability {
                SwitchCapability::HistoryExact => {
                    SwitchAnswer::NotSupported("history can't be copied here")
                }
                _ => SwitchAnswer::Supported("by the fixture"),
            }
        }
        let breaches = contract_breaches(no_history, true);
        assert_eq!(
            breaches,
            vec!["history_exact is required of every adapter that moves conversations".to_owned()]
        );
    }

    #[test]
    fn a_reason_is_a_sentence() {
        fn terse(_: SwitchCapability) -> SwitchAnswer {
            SwitchAnswer::NotSupported("no")
        }
        assert!(
            contract_breaches(terse, false)
                .iter()
                .all(|b| b.contains("a sentence a user can read"))
        );
    }
}
