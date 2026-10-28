//! The switch dialog's words, decided here and not in the UI (the
//! maintainer's walk of 0.1.8's switch, 2026-10-27): what goes along, in
//! plain words with a line on each; the positive line the Target step
//! opens with; and whether the keys-and-passwords question applies at
//! all. It applies only when the conversation's text goes somewhere new
//! — another tool (and the company behind it) reads it, or it is written
//! down to hand over. A same-tool switch on this Mac copies the session
//! file as it is, so it never asks.

use juggler_handover::fidelity::{ArtifactKind, Disposition, FidelityInput};
use juggler_handover::pipeline::Mechanism;

/// One thing a switch moves, recreates or leaves behind.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct CarryItemInfo {
    /// What it is ("The whole conversation").
    pub label: String,
    /// One line on what that means for this switch.
    pub hint: String,
    /// `moves` | `recreated` | `stays`.
    pub how: String,
    /// What it is, for the dialog to join with the set-up check (`mcp`,
    /// `skills`, …): one source for what the other account has.
    pub kind: String,
}

/// The kind's wire name.
fn kind_name(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Chat => "chat",
        ArtifactKind::Rules => "rules",
        ArtifactKind::GitDiff => "changes",
        ArtifactKind::Todos => "tasks",
        ArtifactKind::Mcp => "mcp",
        ArtifactKind::Skills => "skills",
        ArtifactKind::EnvModel => "model",
        ArtifactKind::Subagents => "subagents",
    }
}

/// Whether the switch asks about keys and passwords, and who would read
/// the conversation.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct SecretsInfo {
    /// The question applies: the conversation's text goes to another
    /// tool or into a handover file.
    pub asks: bool,
    /// Who reads it then (`Codex (OpenAI)`).
    pub reader: String,
}

/// Whether the conversation's words leave as text (a replay to another
/// tool, or only notes to read), so keys in it would be seen elsewhere.
#[must_use]
pub fn secrets_question_applies(mechanism: Mechanism) -> bool {
    matches!(mechanism, Mechanism::Replay | Mechanism::PackOnly)
}

/// A tool, as the switch names it: `Codex (OpenAI)`.
#[must_use]
pub fn reader_words(name: &str, vendor: &str) -> String {
    if vendor.is_empty() || vendor == name {
        name.to_owned()
    } else {
        format!("{name} ({vendor})")
    }
}

fn hint(kind: ArtifactKind, mechanism: Mechanism) -> &'static str {
    let same_folder = matches!(mechanism, Mechanism::Transplant | Mechanism::NativeResume);
    match kind {
        ArtifactKind::Chat if same_folder => {
            "Every message from the start, copied as the tool keeps it; the new account picks it up where it stopped."
        }
        ArtifactKind::Chat => {
            "The other tool reads the conversation, newest first as much as fits, to pick up the work."
        }
        ArtifactKind::Rules => {
            "The rule files in the project folder (such as CLAUDE.md or AGENTS.md); they stay there and are read again."
        }
        ArtifactKind::GitDiff if same_folder => {
            "Changes you haven't committed stay in the folder; the new session works in the same folder."
        }
        ArtifactKind::GitDiff => {
            "Changes you haven't committed stay in the folder; a summary of them goes along for the other tool to read."
        }
        ArtifactKind::Todos => "What the session planned to do next, from its latest version.",
        // What the other account has set up is the set-up check's to say
        // (the 0.2 walk, F12: this line said they connect again while the
        // check listed 9 that aren't there): the dialog joins the two.
        ArtifactKind::Mcp => {
            "The servers that give the tool extra abilities. The new session uses the ones set up on the other account."
        }
        ArtifactKind::Skills => {
            "Saved instructions and slash commands. The new session uses the ones set up on the other account."
        }
        ArtifactKind::EnvModel => "Which model the session uses.",
        ArtifactKind::Subagents => "Work done by helper agents inside the session.",
    }
}

/// The carry list, one line each, in plain words (never "pack", never
/// "rebuilt by the target").
#[must_use]
pub fn carry_items(input: &FidelityInput, mechanism: Mechanism) -> Vec<CarryItemInfo> {
    input
        .dispositions
        .iter()
        .map(|(kind, disposition)| {
            let how = match disposition {
                Disposition::Copy
                | Disposition::Translate {
                    untranslated: false,
                    keys_dropped: false,
                } => "moves",
                Disposition::Regenerate => "recreated",
                Disposition::Translate { .. } | Disposition::Drop => "stays",
            };
            let hint = if how == "stays" {
                "It doesn't go along: the new session starts without it.".to_owned()
            } else {
                hint(*kind, mechanism).to_owned()
            };
            CarryItemInfo {
                label: kind.label().to_owned(),
                hint,
                how: how.to_owned(),
                kind: kind_name(*kind).to_owned(),
            }
        })
        .collect()
}

/// The line the Target step opens with: what goes along, said positively.
#[must_use]
pub fn move_line(mechanism: Mechanism, messages: usize, tool_b: &str) -> String {
    let count = if messages == 1 {
        "its 1 message".to_owned()
    } else if messages > 0 {
        format!(
            "all {} messages",
            juggler_handover::pipeline::thousands(u64::try_from(messages).unwrap_or(u64::MAX))
        )
    } else {
        "every message".to_owned()
    };
    // Never "moves" (deep/11 §4.9): the conversation is carried over, and
    // the original stays.
    match mechanism {
        Mechanism::Transplant | Mechanism::NativeResume => format!(
            "The whole conversation is carried over — {count}, its subagents and file history. The original stays as it was."
        ),
        Mechanism::Replay => format!(
            "{tool_b} starts fresh and reads the conversation — {count}, newest first as much as fits — to pick up the work. The original stays as it was."
        ),
        Mechanism::PackOnly => format!(
            "{tool_b} starts fresh from notes about the work: what it is, what was decided and what's left. The original stays as it was."
        ),
    }
}

/// What the switch dialog says, from the preflight's facts.
pub struct PreflightWords {
    /// [`carry_items`].
    pub carry_items: Vec<CarryItemInfo>,
    /// [`move_line`].
    pub move_line: String,
    /// The keys question.
    pub secrets: SecretsInfo,
}

/// The dialog's words for a switch of `source` to `account_b`.
#[must_use]
pub fn preflight_words(
    state: &std::sync::Arc<crate::state::AppState>,
    account_b: &juggler_core::model::Account,
    mechanism: Mechanism,
    input: &FidelityInput,
    messages: usize,
) -> PreflightWords {
    let spec = state
        .registry
        .get(&account_b.harness)
        .map(|adapter| adapter.spec());
    let name = spec.as_ref().map_or_else(
        || account_b.harness.to_string(),
        |spec| spec.display_name.clone(),
    );
    let vendor = spec.map(|spec| spec.vendor).unwrap_or_default();
    PreflightWords {
        carry_items: carry_items(input, mechanism),
        move_line: move_line(mechanism, messages, &name),
        secrets: SecretsInfo {
            asks: secrets_question_applies(mechanism),
            reader: reader_words(&name, &vendor),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_question_applies_only_when_the_words_go_somewhere_new() {
        assert!(
            !secrets_question_applies(Mechanism::Transplant),
            "a same-tool switch copies the file as it is"
        );
        assert!(!secrets_question_applies(Mechanism::NativeResume));
        assert!(
            secrets_question_applies(Mechanism::Replay),
            "another tool reads it"
        );
        assert!(secrets_question_applies(Mechanism::PackOnly));
    }

    #[test]
    fn the_carry_list_is_in_plain_words() {
        let input = juggler_handover::fidelity::base_dispositions("claude-code", "claude-code");
        for item in carry_items(&input, Mechanism::Transplant) {
            let words = format!("{} {}", item.label, item.hint).to_lowercase();
            for token in [
                "pack",
                "redact",
                "target",
                "transcript",
                "git diff",
                "state",
            ] {
                assert!(
                    !words.contains(token),
                    "a carry line uses a product-internal word"
                );
            }
        }
    }

    #[test]
    fn the_target_step_says_what_moves() {
        assert_eq!(
            move_line(Mechanism::Transplant, 42, "Claude Code"),
            "The whole conversation is carried over — all 42 messages, its subagents and file history. The original stays as it was."
        );
        // The 0.2 walk (F12): "all 31009 messages", and "moves".
        let long = move_line(Mechanism::Replay, 31_009, "Codex");
        assert!(long.contains("all 31,009 messages"), "{long}");
        for mechanism in [
            Mechanism::Transplant,
            Mechanism::NativeResume,
            Mechanism::Replay,
            Mechanism::PackOnly,
        ] {
            let words = move_line(mechanism, 3, "Codex").to_lowercase();
            for jargon in ["moves", "natively", "replay"] {
                assert!(!words.contains(jargon), "{jargon} in {words}");
            }
        }
        assert_eq!(reader_words("Codex", "OpenAI"), "Codex (OpenAI)");
    }
}
