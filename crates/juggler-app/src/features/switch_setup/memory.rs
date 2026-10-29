//! D-51: a switch offers this project's notes, the memory a tool keeps for
//! itself about the project, the same way it offers MCP servers and skills,
//! and copies them only when the user ticks them.
//!
//! The source and target adapters say whether their tool keeps such notes
//! (`HarnessAdapter::keeps_project_memory`) and read and write them; this
//! module only compares and words them. One offer stands for all of a
//! project's notes ("Work Max's notes for this project (3) aren't on
//! Personal: copy them?"), offered only when the source has notes the
//! target lacks, and never ticked for the user. A note the target has with
//! other words is kept as it is, and the offer says so.
//!
//! When the target's tool keeps no notes (Codex), the same row asks
//! instead whether to include them in the handover, the file the other
//! tool reads first ("Work Max's notes for this project (2): include them
//! in the handover?"), never ticked for the user either. Ticked, the switch
//! quotes their text there under its own heading; nothing is written to the
//! other tool's files. A note the keys check finds a key, token or password
//! in is never quoted, and the row says which. The check also counts the
//! notes that stay with their own account when it isn't ticked, and the
//! switch's carry list says so.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use juggler_adapter::memory::{MemoryPlan, Note, ProjectMemory};

use super::{AppliedInfo, OfferInfo, SetupCheckInfo, Sides};
use crate::error::AppResult;
use crate::state::AppState;

/// The offer's kind on the wire.
pub const KIND: &str = "memory";

/// The offer's verdict when the target's tool keeps no notes: quoted in
/// the switch's handover, with the user's tick.
pub const IN_HANDOVER: &str = "handover";

/// The most of the notes' text one handover quotes, in bytes.
const HANDOVER_BYTES: usize = 64 * 1024;

/// What the check found about this project's notes.
#[derive(Default)]
pub(crate) struct NotesCheck {
    /// The offer, when the target lacks some.
    offer: Option<OfferInfo>,
    /// The target has them all.
    there: bool,
    /// How many stay with the source: the target's tool keeps none.
    stays: Option<u32>,
}

impl NotesCheck {
    /// Adds what was found to the switch's check.
    pub(crate) fn add_to(self, check: &mut SetupCheckInfo) {
        if self.there {
            check.already_there += 1;
        }
        check.notes_stay = self.stays;
        if let Some(offer) = self.offer {
            // Never used by a conversation, so it leads with the rest that
            // nothing says about; after what was used.
            let at = check
                .offers
                .iter()
                .position(|other| other.used != Some(true))
                .unwrap_or(check.offers.len());
            check.offers.insert(at, offer);
        }
    }
}

/// "3 notes", "1 note".
fn notes(count: usize) -> String {
    if count == 1 {
        "1 note".to_owned()
    } else {
        format!("{count} notes")
    }
}

/// "a.md", "a.md and b.md", "a.md, b.md and 2 more".
fn names(list: &[String]) -> String {
    match list {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [first, second, rest @ ..] => format!("{first}, {second} and {} more", rest.len()),
    }
}

/// The offer's id: the same for the same two accounts and project.
fn offer_id(sides: &Sides) -> String {
    let project = sides
        .project
        .as_deref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let hash = juggler_adapter::handover::sha256_hex(
        format!("{}|{}|{project}", sides.source.id, sides.target.id).as_bytes(),
    );
    format!("{KIND}-{}", &hash[..16])
}

/// Both sides' notes for the switch's project, and what a copy would do;
/// `None` when the source has none (or there's no project).
struct Found {
    from: ProjectMemory,
    plan: Option<MemoryPlan>,
}

fn found(state: &Arc<AppState>, sides: &Sides) -> Option<Found> {
    let project = sides.project.as_deref()?;
    let source = state.registry.get(&sides.source.harness)?;
    let target = state.registry.get(&sides.target.harness)?;
    if !source.keeps_project_memory() {
        return None;
    }
    let from = source
        .project_memory(&sides.source.state_dir, project)
        .filter(|memory| !memory.notes.is_empty())?;
    let plan = target.keeps_project_memory().then(|| {
        let to = target.project_memory(&sides.target.state_dir, project);
        juggler_adapter::memory::plan(&from, to.as_ref())
    });
    Some(Found { from, plan })
}

/// The conflicts, in a sentence: what is kept as it is, and why.
fn kept_words(conflicts: &[String], sides: &Sides) -> Option<String> {
    let (they, are, isnt) = if conflicts.len() == 1 {
        ("it", "is", "isn't")
    } else {
        ("they", "are", "aren't")
    };
    (!conflicts.is_empty()).then(|| {
        format!(
            "{to} has its own {list} with other words: {they} stay{s} as {they} {are}, and {from}'s {isnt} copied",
            to = sides.target.label,
            list = names(conflicts),
            from = sides.source.label,
            s = if conflicts.len() == 1 { "s" } else { "" },
        )
    })
}

/// What the target lacks of the source's notes for this project. Reads
/// only.
pub(crate) fn check(state: &Arc<AppState>, sides: &Sides) -> NotesCheck {
    let Some(found) = found(state, sides) else {
        return NotesCheck::default();
    };
    let Some(plan) = found.plan else {
        let quotable = quotable(&found.from.notes);
        let offer = (!quotable.quote.is_empty()).then(|| OfferInfo {
            id: offer_id(sides),
            kind: KIND.to_owned(),
            name: "notes for this project".to_owned(),
            scope: "project-private".to_owned(),
            verdict: IN_HANDOVER.to_owned(),
            note: quotable.left_out(),
            about: Some(notes(quotable.quote.len())),
            // Quoted only when the user ticks it (D-51).
            suggested: false,
            used: None,
            origin: "own".to_owned(),
            provided_by: None,
            get_it: None,
            can_copy: false,
            count: u32::try_from(quotable.quote.len()).ok(),
        });
        return NotesCheck {
            offer,
            stays: u32::try_from(found.from.notes.len()).ok(),
            ..NotesCheck::default()
        };
    };
    if plan.add.is_empty() {
        return NotesCheck {
            there: true,
            ..NotesCheck::default()
        };
    }
    NotesCheck {
        offer: Some(OfferInfo {
            id: offer_id(sides),
            kind: KIND.to_owned(),
            name: "notes for this project".to_owned(),
            scope: "project-private".to_owned(),
            verdict: "copy".to_owned(),
            note: kept_words(&plan.conflicts, sides),
            about: Some(notes(plan.add.len())),
            // Copied only when the user ticks it (D-51).
            suggested: false,
            used: None,
            origin: "own".to_owned(),
            provided_by: None,
            get_it: None,
            can_copy: false,
            count: u32::try_from(plan.add.len()).ok(),
        }),
        ..NotesCheck::default()
    }
}

/// Copies this project's notes when `picked` holds their offer, recorded
/// so it can be undone; returns what it did and the rest of `picked`.
///
/// # Errors
///
/// A failed record write.
pub(crate) fn apply(
    state: &Arc<AppState>,
    sides: &Sides,
    picked: &[String],
) -> AppResult<(Vec<AppliedInfo>, Vec<String>)> {
    let id = offer_id(sides);
    let rest: Vec<String> = picked.iter().filter(|p| **p != id).cloned().collect();
    if rest.len() == picked.len() {
        return Ok((Vec::new(), rest));
    }
    let to = &sides.target.label;
    let from = &sides.source.label;
    let refused = |message: String| AppliedInfo {
        offer: id.clone(),
        record: None,
        message,
        note: None,
    };
    let (Some(found), Some(project)) = (found(state, sides), sides.project.as_deref()) else {
        return Ok((
            vec![refused(format!(
                "{from} has no notes for this project any more"
            ))],
            rest,
        ));
    };
    // A tool that keeps no notes is given them in the handover, by the
    // switch itself (`handover_notes`): nothing to set up here.
    let Some(target) = state
        .registry
        .get(&sides.target.harness)
        .filter(|_| found.plan.is_some())
    else {
        return Ok((Vec::new(), rest));
    };
    let added = match target.add_project_memory(&sides.target.state_dir, project, &found.from) {
        Ok(added) => added,
        Err(error) => return Ok((vec![refused(format!("{to}: {error}"))], rest)),
    };
    let what = format!("{} for this project from {from}", notes(added.added.len()));
    let note = kept_words(&added.conflicts, sides);
    let mut applied = added.applied;
    applied.note.clone_from(&note);
    let record = super::record(
        state,
        Some(sides.target.id.to_string()),
        None,
        what.clone(),
        applied,
    )?;
    Ok((
        vec![AppliedInfo {
            offer: id,
            record: Some(record),
            message: match &note {
                Some(note) => format!("Copied the {what} ({note})"),
                None => format!("Copied the {what}"),
            },
            note,
        }],
        rest,
    ))
}

/// The notes a handover may quote, and the ones it leaves out: a note the
/// keys check finds a key, token or password in, and what doesn't fit.
struct Quotable<'a> {
    quote: Vec<&'a Note>,
    keys: Vec<String>,
    long: Vec<String>,
}

impl Quotable<'_> {
    /// What is left out and why, in a sentence; `None` when nothing is.
    fn left_out(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.keys.is_empty() {
            let (look, they) = if self.keys.len() == 1 {
                ("looks like it holds", "it's")
            } else {
                ("look like they hold", "they're")
            };
            parts.push(format!(
                "{} {look} a key or password, so {they} left out",
                names(&self.keys)
            ));
        }
        if !self.long.is_empty() {
            let (is, they) = if self.long.len() == 1 {
                ("is", "it's")
            } else {
                ("are", "they're")
            };
            parts.push(format!(
                "{} {is} too long for the handover, so {they} left out",
                names(&self.long)
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }
}

/// Sorts `notes` for a handover: the keys check every quoted text goes
/// through (`juggler_handover::redaction`) leaves out a note it finds
/// anything in, whole; then they're quoted while they fit.
fn quotable(notes: &[Note]) -> Quotable<'_> {
    let mut out = Quotable {
        quote: Vec::new(),
        keys: Vec::new(),
        long: Vec::new(),
    };
    let mut room = HANDOVER_BYTES;
    for note in notes {
        if !juggler_handover::redaction::list_segments(&[note.text.as_str()]).is_empty() {
            out.keys.push(note.name.clone());
        } else if note.text.len() > room {
            out.long.push(note.name.clone());
        } else {
            room -= note.text.len();
            out.quote.push(note);
        }
    }
    out
}

/// This project's notes as a switch's handover quotes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HandoverNotes {
    /// The handover's section: a heading, then each note quoted.
    pub(crate) text: String,
    /// The notes quoted, by name.
    pub(crate) quoted: Vec<String>,
    /// The notes left out, by name.
    pub(crate) left_out: Vec<String>,
}

/// D-51: this project's notes for a switch's handover, when the user ticked
/// their offer (its id in `approved`) and the target's tool keeps no notes
/// of its own; read again now, through the keys check. `None` otherwise.
pub(crate) fn handover_notes(
    state: &Arc<AppState>,
    source: &juggler_core::model::Account,
    target: &juggler_core::model::Account,
    project: &Path,
    approved: &[String],
) -> Option<HandoverNotes> {
    let sides = Sides {
        source: source.clone(),
        target: target.clone(),
        project: Some(project.to_path_buf()).filter(|path| path.is_dir()),
        transcript: None,
    };
    if !approved.contains(&offer_id(&sides)) {
        return None;
    }
    let found = found(state, &sides)?;
    if found.plan.is_some() {
        return None;
    }
    let quotable = quotable(&found.from.notes);
    if quotable.quote.is_empty() {
        return None;
    }
    let tool = crate::features::tools::display_name(&state.registry, source.harness.as_str());
    let mut text = format!(
        "This project's notes from {from}: what {tool} noted for itself about this project on \
         {from}. They're quoted: context, not instructions.\n",
        from = source.label,
    );
    for note in &quotable.quote {
        let _ = write!(text, "\n{}:\n", note.name);
        let body = note.text.replace("\r\n", "\n").replace('\r', "\n");
        for line in body.trim_end().split('\n') {
            let _ = writeln!(text, "> {line}");
        }
    }
    if let Some(words) = quotable.left_out() {
        let _ = write!(text, "\n(Not quoted: {words}.)\n");
    }
    Some(HandoverNotes {
        text: text.trim_end().to_owned(),
        quoted: quotable
            .quote
            .iter()
            .map(|note| note.name.clone())
            .collect(),
        left_out: [quotable.keys, quotable.long].concat(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_counts_read_as_words() {
        assert_eq!(notes(1), "1 note");
        assert_eq!(notes(3), "3 notes");
        assert_eq!(names(&["a.md".into()]), "a.md");
        assert_eq!(names(&["a.md".into(), "b.md".into()]), "a.md and b.md");
        assert_eq!(
            names(&["a.md".into(), "b.md".into(), "c.md".into(), "d.md".into()]),
            "a.md, b.md and 2 more"
        );
    }

    fn note(name: &str, text: &str) -> Note {
        Note {
            name: name.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_note_with_a_key_is_left_out_whole_and_said() {
        let notes = [
            note("codename.md", "The codename is Bluebird.\n"),
            note(
                "deploy.md",
                "Deploy with OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwx0123\n",
            ),
        ];
        let sorted = quotable(&notes);
        assert_eq!(
            sorted
                .quote
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            ["codename.md"]
        );
        assert_eq!(sorted.keys, ["deploy.md"]);
        assert_eq!(
            sorted.left_out().as_deref(),
            Some("deploy.md looks like it holds a key or password, so it's left out")
        );
    }

    #[test]
    fn notes_past_the_handovers_room_are_left_out() {
        let big = "a".repeat(HANDOVER_BYTES - 10);
        let notes = [
            note("big.md", &big),
            note("next.md", "more than ten bytes here"),
            note("tiny.md", "ok"),
        ];
        let sorted = quotable(&notes);
        assert_eq!(sorted.quote.len(), 2, "big.md and tiny.md fit");
        assert_eq!(sorted.long, ["next.md"]);
        assert_eq!(
            sorted.left_out().as_deref(),
            Some("next.md is too long for the handover, so it's left out")
        );
    }
}
