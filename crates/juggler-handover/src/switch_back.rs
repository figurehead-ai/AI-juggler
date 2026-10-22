//! Switch-back as a resume, never a copy-back (AJ-E207; D-05;
//! pass2/02 §6.1-6.2). This module owns the pure half: the
//! "While you were away" delta turn (exactly one, bounded), the
//! verbatim suggestion copy (suggest-only — the user clicks), the
//! both-live warning, and the S-F7 fallback verdict (nearest
//! surviving epoch or a pack-primed F0 fresh start — never a
//! fabricated resume).
//!
//! The impure half (graph walks, transcript reads, the spawn) lives
//! in the app layer; there is NO function here that writes a file,
//! and the resume plan carries only the ORIGINAL native id — the
//! away transcript has no path back over the original (the
//! copy-back ban is structural: `reconcile::apply_choice` plans
//! artifact actions, and the transcript is not an artifact).

use std::collections::HashSet;

use crate::pack::IrTurn;
use crate::reconcile::{self, ConflictChoice, ConversationReconciliation, FileConflict};

/// Turns rendered in the delta before the "…and N more" line (the
/// ≤4k-token default: bounded lines × bounded one-liners + the
/// pack pointer for the full detail).
const MAX_RENDERED_TURNS: usize = 40;

/// Characters per rendered turn one-liner.
const MAX_LINE_CHARS: usize = 160;

/// The newest turns rendered in full (bounded by [`FULL_TURN_CHARS`]):
/// the latest decisions are what the returning session most needs, and
/// they are often at the end of a long reply (AJ-E1541, found by the live
/// lab: a name said only in the last reply was cut off at 120 chars).
const FULL_TURNS: usize = 6;

/// Characters kept from each of the [`FULL_TURNS`] newest turns.
const FULL_TURN_CHARS: usize = 1_500;

/// Everything the delta turn renders from (already reconciled).
#[derive(Clone, PartialEq, Debug)]
pub struct DeltaInput {
    /// The away account's label (copy names it).
    pub away_account: String,
    /// The original account's label.
    pub original_account: String,
    /// ALL of the away transcript's turns, in file order (the
    /// engine filters to away-only and dedupes first-wins).
    pub away_turns: Vec<IrTurn>,
    /// The away-only uuid set from [`reconcile::reconcile_conversation`].
    pub away_only_uuids: HashSet<String>,
    /// The reconciliation counts (shared / original-only).
    pub conversation: ConversationReconciliation,
    /// The away pack's diff stat line, when it froze one
    /// (e.g. `+120 −8 across 3 files`).
    pub diff_stat: Option<String>,
    /// The away pack's todo changes line, when any.
    pub todo_note: Option<String>,
    /// AJ-E1541: the hops between the account returned to and the away
    /// one, oldest first (A → B → Codex, returning to A: B's new turns),
    /// each already reduced to what was new on it. Empty for a one-hop
    /// return.
    pub earlier_hops: Vec<DeltaHop>,
    /// The accounts whose conversation couldn't be found (lab 8: another
    /// session in the same folder made it ambiguous). Named in the update,
    /// so a missing hop is never silent.
    pub unread: Vec<String>,
}

/// One account the work continued on before the away one, and what was
/// new there (AJ-E1541).
#[derive(Clone, PartialEq, Debug)]
pub struct DeltaHop {
    /// The account's label.
    pub account: String,
    /// The turns new on that account, in file order.
    pub turns: Vec<IrTurn>,
}

/// The away turns that belong to the delta, in file order, deduped
/// first-wins (Prior-09 §Q3: a re-emitted uuid renders once) —
/// uuid-less lines cannot be classified and are excluded (counted
/// honestly in the input, never silently folded into the delta).
#[must_use]
pub fn delta_turns(input: &DeltaInput) -> Vec<IrTurn> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut turns = Vec::new();
    for turn in &input.away_turns {
        let Some(uuid) = &turn.uuid else {
            continue;
        };
        if !input.away_only_uuids.contains(uuid) {
            continue;
        }
        if seen.insert(uuid.clone()) {
            turns.push(turn.clone());
        }
    }
    turns
}

/// Renders the ONE "While you were away" delta turn (§6.1): what was new
/// on every account since this one, oldest hop first, then the diff stat
/// and todo changes. Bounded: the newest `MAX_RENDERED_TURNS` turns, of
/// which the newest `FULL_TURNS` keep up to `FULL_TURN_CHARS` and the
/// rest are one-liners of `MAX_LINE_CHARS`; older turns are counted.
/// Nothing is promised that isn't in the text (no pack is attached).
#[must_use]
pub fn delta_turn_text(input: &DeltaInput) -> String {
    use std::fmt::Write as _;
    let last = delta_turns(input);
    let mut all: Vec<(&str, IrTurn)> = Vec::new();
    for hop in &input.earlier_hops {
        all.extend(
            hop.turns
                .iter()
                .map(|turn| (hop.account.as_str(), turn.clone())),
        );
    }
    all.extend(
        last.into_iter()
            .map(|turn| (input.away_account.as_str(), turn)),
    );
    // A tool call or its result has no text of its own here: an empty line
    // tells the returning session nothing (found by the live lab).
    all.retain(|(_, turn)| !turn.text.trim().is_empty());
    let mut text = String::new();
    if input.earlier_hops.is_empty() {
        let original_only_note = if input.conversation.original_only_uuids > 0 {
            format!(
                ", {} new here on {}",
                input.conversation.original_only_uuids, input.original_account
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            text,
            "While you were away on {} — {} new turns ({} shared before the switch{}).\n",
            input.away_account,
            all.len(),
            input.conversation.base_uuids,
            original_only_note
        );
    } else {
        let route: Vec<&str> = input
            .earlier_hops
            .iter()
            .map(|hop| hop.account.as_str())
            .chain(std::iter::once(input.away_account.as_str()))
            .collect();
        let _ = writeln!(
            text,
            "While you were away, this work continued on {} — {} new turns.\n",
            route.join(", then "),
            all.len()
        );
    }
    let shown_from = all.len().saturating_sub(MAX_RENDERED_TURNS);
    if shown_from > 0 {
        let _ = writeln!(text, "…{shown_from} earlier turns, not shown.");
    }
    let full_from = all.len().saturating_sub(FULL_TURNS);
    let mut current: Option<&str> = None;
    for (index, (account, turn)) in all.iter().enumerate().skip(shown_from) {
        if !input.earlier_hops.is_empty() && current != Some(*account) {
            let _ = writeln!(text, "On {account}:");
            current = Some(*account);
        }
        let role = match turn.role {
            crate::pack::IrRole::User => "user",
            crate::pack::IrRole::Assistant => "assistant",
        };
        let limit = if index >= full_from {
            FULL_TURN_CHARS
        } else {
            MAX_LINE_CHARS
        };
        let line: String = turn
            .text
            .replace(['\n', '\r'], " ")
            .chars()
            .take(limit)
            .collect();
        let _ = writeln!(text, "- [{role}] {line}");
    }
    for account in &input.unread {
        let _ = writeln!(
            text,
            "On {account}: AI Juggler couldn't find the conversation there, so what happened on it isn't included."
        );
    }
    if let Some(diff) = &input.diff_stat {
        let _ = writeln!(text, "\nDiff while away: {diff}.");
    }
    if let Some(todos) = &input.todo_note {
        let _ = writeln!(text, "Todos changed: {todos}.");
    }
    let _ = write!(
        text,
        "\nThe project's files are as they were left on {}. Pick up from here.",
        input.away_account
    );
    text
}

/// The quota-reset suggestion, VERBATIM (Req.3) — suggested, never
/// executed (automation invariant SPEC:26,263): the UI renders this
/// card and the user clicks.
#[must_use]
pub fn suggestion_copy(original_account: &str, away_account: &str) -> String {
    format!(
        "{original_account} has reset. Resume the original conversation there? Work from \
         {away_account} arrives as one 'while you were away' update."
    )
}

/// The both-live warning (§6.2): resuming A while B runs is legal
/// (parallel accounts are the founding use case) but the UI must
/// SAY it — silence is how wrong-account billing happens.
#[must_use]
pub fn both_live_copy(original_account: &str, away_account: &str) -> String {
    format!(
        "Both accounts are now live on this work item: {original_account} (resumed) and \
         {away_account} (still running). Check which one you type into."
    )
}

/// The divergence headline (§7.1 shape): counts, never "some
/// context"; nothing is merged silently.
#[must_use]
pub fn divergence_copy(new_turns: usize, file_conflicts: usize) -> String {
    format!(
        "Both sides moved since the switch. {new_turns} new turns and {file_conflicts} file \
         conflicts need your choice — nothing is merged silently."
    )
}

/// S-F7 verdict: what switch-back may resume when the original
/// native session is gone. Never fabricates — `ResumeNearest` only
/// when a surviving epoch's native id exists, else the pack-primed
/// fresh start honestly labelled F0 (pack-only fidelity until the
/// pair proves more).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OriginalSession {
    /// The original transcript exists — resume its native id.
    Alive {
        /// The native session id to resume.
        native_id: String,
    },
    /// S-F7: the original is gone — switch-back offers the nearest
    /// surviving epoch, or a pack-primed fresh start, never a
    /// fabricated resume.
    Gone {
        /// What switch-back offers instead (never a fabricated id).
        fallback: Sf7Fallback,
    },
}

/// The honest S-F7 alternatives.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sf7Fallback {
    /// Resume the nearest surviving epoch on the lineage.
    NearestEpoch {
        /// That epoch's native session id.
        native_id: String,
    },
    /// Pack-primed fresh instance — labelled F0 (pack-only) until a
    /// fixture proves more for the pair.
    FreshPackF0,
}

/// Decides the S-F7 verdict from the facts (pure).
#[must_use]
pub fn original_session_verdict(
    original_exists: bool,
    original_native_id: Option<&str>,
    nearest_surviving_native_id: Option<&str>,
) -> OriginalSession {
    if original_exists {
        return OriginalSession::Alive {
            native_id: original_native_id.unwrap_or_default().to_owned(),
        };
    }
    OriginalSession::Gone {
        fallback: match nearest_surviving_native_id.filter(|id| !id.is_empty()) {
            Some(native_id) => Sf7Fallback::NearestEpoch {
                native_id: native_id.to_owned(),
            },
            None => Sf7Fallback::FreshPackF0,
        },
    }
}

/// The resolution record for one conflict — what the merge edge's
/// manifest row states (both parents live on the edge itself,
/// E201).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConflictResolution {
    /// The conflicting path.
    pub rel_path: String,
    /// The user's choice.
    pub choice: ConflictChoice,
}

/// Plans every choice's file action for the merge record (pure;
/// the app executes the plan — see [`reconcile::apply_choice`]).
#[must_use]
pub fn plan_resolutions(
    conflicts: &[FileConflict],
    resolutions: &[ConflictResolution],
    date: &str,
) -> Vec<(ConflictResolution, reconcile::FileAction)> {
    conflicts
        .iter()
        .filter_map(|conflict| {
            let resolution = resolutions
                .iter()
                .find(|row| row.rel_path == conflict.rel_path)?;
            Some((
                resolution.clone(),
                reconcile::apply_choice(conflict, resolution.choice, date),
            ))
        })
        .collect()
}

/// Every conflict must have exactly one resolution before a merge
/// edge may record (S-F8: unresolved ⇒ the work stays forked, both
/// epochs resumable — an error, not a guess).
#[must_use]
pub fn unresolved(conflicts: &[FileConflict], resolutions: &[ConflictResolution]) -> Vec<String> {
    conflicts
        .iter()
        .filter(|conflict| {
            !resolutions
                .iter()
                .any(|row| row.rel_path == conflict.rel_path)
        })
        .map(|conflict| conflict.rel_path.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{IrRole, IrTurn};
    use crate::reconcile::reconcile_conversation;

    fn turn(uuid: Option<&str>, role: IrRole, text: &str) -> IrTurn {
        IrTurn {
            uuid: uuid.map(str::to_owned),
            role,
            text: text.to_owned(),
            at_ms: None,
            tokens: None,
        }
    }

    fn delta_fixture() -> DeltaInput {
        let away_uuids: HashSet<String> = ["u1", "u2", "b1", "b2"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
        let original_uuids: HashSet<String> = ["u1", "u2", "a1"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
        DeltaInput {
            away_account: "Personal".into(),
            original_account: "Work Max".into(),
            away_turns: vec![
                turn(Some("u1"), IrRole::User, "base turn"),
                turn(Some("b1"), IrRole::User, "fixed the login bug"),
                turn(Some("b1"), IrRole::User, "fixed the login bug (re-emitted)"),
                turn(Some("b2"), IrRole::Assistant, "ran the suite — 34 passing"),
                turn(Some("u2"), IrRole::Assistant, "more base"),
                turn(None, IrRole::User, "untracked line"),
            ],
            away_only_uuids: away_uuids.difference(&original_uuids).cloned().collect(),
            conversation: reconcile_conversation(&away_uuids, &original_uuids),
            diff_stat: Some("+120 −8 across 3 files".into()),
            todo_note: Some("\"add retry\" → done".into()),
            earlier_hops: Vec::new(),
            unread: Vec::new(),
        }
    }

    #[test]
    fn an_account_whose_conversation_wasnt_found_is_named() {
        // Lab 8: the Codex hop's conversation couldn't be told apart from
        // another session in the same folder, and the update just left it
        // out. It says so now.
        let mut input = delta_fixture();
        input.unread = vec!["Lab Codex".into()];
        let text = delta_turn_text(&input);
        assert!(
            text.contains(
                "On Lab Codex: AI Juggler couldn't find the conversation there, so what happened on it isn't included."
            ),
            "{text}"
        );
        assert!(!delta_turn_text(&delta_fixture()).contains("couldn't find"));
    }

    #[test]
    fn delta_turns_filter_dedupe_and_skip_untracked() {
        let turns = delta_turns(&delta_fixture());
        // b1 re-emitted renders ONCE (first-wins); base u1/u2 and
        // the uuid-less line are excluded.
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].uuid.as_deref(), Some("b1"));
        assert_eq!(turns[0].text, "fixed the login bug");
        assert_eq!(turns[1].uuid.as_deref(), Some("b2"));
    }

    #[test]
    fn delta_text_is_one_bounded_turn() {
        let text = delta_turn_text(&delta_fixture());
        assert!(text.starts_with("While you were away on Personal"));
        assert!(text.contains("2 new turns (2 shared before the switch, 1 new here on Work Max)"));
        assert!(text.contains("- [user] fixed the login bug"));
        assert!(text.contains("Diff while away: +120 −8 across 3 files."));
        assert!(text.contains("Todos changed: \"add retry\" → done."));
        assert!(text.ends_with("as they were left on Personal. Pick up from here."));
        // Nothing promised that isn't there.
        assert!(!text.contains("pack"));
        // Bounded: one line per turn, each within the full-turn limit.
        assert!(
            text.lines()
                .all(|line| line.chars().count() <= FULL_TURN_CHARS + 20)
        );
    }

    #[test]
    fn many_away_turns_collapse_to_the_pack_pointer() {
        let mut input = delta_fixture();
        input.away_turns = (0..100)
            .map(|index| turn(Some(&format!("b-{index}")), IrRole::User, "away work"))
            .collect();
        input.away_only_uuids = (0..100)
            .map(|index| format!("b-{index}"))
            .collect::<HashSet<_>>();
        input.conversation = ConversationReconciliation {
            base_uuids: 0,
            away_only_uuids: Vec::new(),
            original_only_uuids: 0,
        };
        let text = delta_turn_text(&input);
        assert!(text.contains("100 new turns"));
        assert!(text.contains("…60 earlier turns, not shown."));
        let rendered = text.lines().filter(|line| line.starts_with("- [")).count();
        assert_eq!(rendered, MAX_RENDERED_TURNS);
    }

    #[test]
    fn turns_without_text_are_left_out() {
        let mut input = delta_fixture();
        input.away_turns = vec![
            turn(Some("b1"), IrRole::Assistant, ""),
            turn(Some("b2"), IrRole::User, "  "),
            turn(Some("b3"), IrRole::Assistant, "tests pass"),
        ];
        input.away_only_uuids = ["b1", "b2", "b3"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
        let text = delta_turn_text(&input);
        assert!(text.contains("1 new turns"), "{text}");
        assert_eq!(
            text.lines().filter(|line| line.starts_with("- [")).count(),
            1
        );
    }

    #[test]
    fn the_newest_turns_keep_what_was_said_at_their_end() {
        let mut input = delta_fixture();
        let long = format!("{} The release name is Copper Kettle.", "x".repeat(400));
        input.away_turns = vec![turn(Some("b1"), IrRole::Assistant, &long)];
        let text = delta_turn_text(&input);
        assert!(text.contains("Copper Kettle"), "{text}");
    }

    #[test]
    fn a_return_over_several_hops_names_each_and_keeps_their_turns() {
        let mut input = delta_fixture();
        input.away_account = "Codex".into();
        input.earlier_hops = vec![DeltaHop {
            account: "Personal".into(),
            turns: vec![turn(Some("p1"), IrRole::User, "add sub and name mul times")],
        }];
        let text = delta_turn_text(&input);
        assert!(
            text.starts_with(
                "While you were away, this work continued on Personal, then Codex — 3 new turns."
            ),
            "{text}"
        );
        let personal = text.find("On Personal:").unwrap();
        let codex = text.find("On Codex:").unwrap();
        assert!(personal < codex);
        assert!(text.contains("- [user] add sub and name mul times"));
    }

    #[test]
    fn suggestion_copy_is_verbatim() {
        assert_eq!(
            suggestion_copy("Work Max", "Personal"),
            "Work Max has reset. Resume the original conversation there? Work from Personal \
             arrives as one 'while you were away' update."
        );
    }

    #[test]
    fn sf7_verdict_never_fabricates() {
        assert_eq!(
            original_session_verdict(true, Some("native-1"), Some("native-9")),
            OriginalSession::Alive {
                native_id: "native-1".into()
            }
        );
        assert_eq!(
            original_session_verdict(false, Some("native-1"), Some("native-9")),
            OriginalSession::Gone {
                fallback: Sf7Fallback::NearestEpoch {
                    native_id: "native-9".into()
                }
            }
        );
        assert_eq!(
            original_session_verdict(false, Some("native-1"), None),
            OriginalSession::Gone {
                fallback: Sf7Fallback::FreshPackF0
            }
        );
    }

    #[test]
    fn unresolved_conflicts_are_listed_not_guessed() {
        let conflicts = vec![
            FileConflict {
                rel_path: "todos/list.json".into(),
                base_sha: Some("h0".into()),
                away_sha: Some("h1".into()),
                original_sha: Some("h2".into()),
            },
            FileConflict {
                rel_path: "rules/AGENTS.md".into(),
                base_sha: Some("h0".into()),
                away_sha: Some("h3".into()),
                original_sha: Some("h4".into()),
            },
        ];
        let resolutions = vec![ConflictResolution {
            rel_path: "todos/list.json".into(),
            choice: ConflictChoice::KeepBoth,
        }];
        assert_eq!(
            unresolved(&conflicts, &resolutions),
            vec!["rules/AGENTS.md"]
        );
        let plan = plan_resolutions(&conflicts, &resolutions, "20260101");
        assert_eq!(plan.len(), 1);
        assert!(matches!(
            plan[0].1,
            reconcile::FileAction::SaveAwaySibling { .. }
        ));
    }
}
