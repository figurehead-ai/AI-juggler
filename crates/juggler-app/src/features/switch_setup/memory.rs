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
//! other words is kept as it is, and the offer says so. When the target's
//! tool keeps no notes, there is no offer: the check counts the notes that
//! stay with their own account, and the switch's carry list says so.

use std::sync::Arc;

use juggler_adapter::memory::{MemoryPlan, ProjectMemory};

use super::{AppliedInfo, OfferInfo, SetupCheckInfo, Sides};
use crate::error::AppResult;
use crate::state::AppState;

/// The offer's kind on the wire.
pub const KIND: &str = "memory";

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
        return NotesCheck {
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
    let Some(target) = state
        .registry
        .get(&sides.target.harness)
        .filter(|_| found.plan.is_some())
    else {
        return Ok((
            vec![refused(format!(
                "{to}'s tool keeps no notes like these, so they stay with {from}"
            ))],
            rest,
        ));
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
}
