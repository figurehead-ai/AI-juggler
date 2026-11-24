//! AJ-E2324: the files a move touches, made sure of before it backs them
//! up and before it moves them; moved out of `moving.rs` for its size.
//!
//! - **Holding still** ([`hold_still`]): every file at or under the
//!   touched paths holds still for [`QUIET`] (no write, size or time of
//!   last write changing), woken by the files' own events
//!   ([`MoveSystem::quiet_for`]: the kernel's, on the real system), at
//!   most [`STILL_CAP`]. Before, a walk every 1.5 s, at least once: a
//!   1.5 s floor on every move and up to 9 s of walks.
//! - **The open-file guard** ([`open_writers`], the review's B3): right
//!   before the backup and again right before anything moves, a program
//!   that has one of those files open to write it (a sync tool, an
//!   editor, another app: writers no tool's adapter names) stops the move
//!   by name. Readers never do.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{MoveSystem, Sides, Writing, place_name, quit};
use crate::error::{MoveHolder, MoveStop, StopClass};
use crate::state::AppState;

/// How long every file the move touches must hold still, once every
/// writer ended, before it is backed up.
pub(super) const QUIET: Duration = Duration::from_millis(300);
/// The longest a move waits for them.
pub(super) const STILL_CAP: Duration = Duration::from_secs(9);

/// Waits until every file at or under `paths` holds still for [`QUIET`],
/// at most [`STILL_CAP`]. Each window is woken by the files' events
/// ([`MoveSystem::quiet_for`]); after it, their sizes and times of last
/// write decide (what the events don't watch included). Returns the files
/// still changing then.
pub(super) fn hold_still(paths: &[PathBuf], system: &dyn MoveSystem) -> Result<(), Vec<PathBuf>> {
    use juggler_handover::moving::state::{changed, marks};
    let until = Instant::now() + STILL_CAP;
    let looks = STILL_CAP.as_millis() / QUIET.as_millis();
    let mut last = marks(paths);
    let mut moving = Vec::new();
    for _ in 0..looks {
        let left = until.saturating_duration_since(Instant::now()).max(QUIET);
        let calm = system.quiet_for(paths, QUIET, left);
        let now = marks(paths);
        moving = changed(&last, &now);
        if calm && moving.is_empty() {
            return Ok(());
        }
        if Instant::now() >= until {
            break;
        }
        last = now;
    }
    Err(if moving.is_empty() {
        paths.to_vec()
    } else {
        moving
    })
}

/// The open-file guard: the programs that have a file at or under
/// `paths` open to write it stop the move, by name ("Dropbox (process
/// 512) has this conversation's files open to write them"): the
/// person's to close. `None`: none does.
pub(super) fn open_writers(paths: &[PathBuf], system: &dyn MoveSystem) -> Option<MoveStop> {
    let mut found = system.open_writers(paths);
    if found.is_empty() {
        return None;
    }
    found.sort();
    found.dedup();
    let named: Vec<String> = found
        .iter()
        .map(|(pid, name)| format!("{name} (process {pid})"))
        .collect();
    let words = format!(
        "{} {} this conversation's files open to write them",
        juggler_core::words::and_list(&named),
        if found.len() == 1 { "has" } else { "have" },
    );
    let detail = format!("open for writing: {}", named.join(", "));
    let holders = found
        .into_iter()
        .map(|(_, name)| MoveHolder {
            name,
            place_id: None,
            ..MoveHolder::default()
        })
        .collect();
    Some(MoveStop::new(StopClass::NeedsYou, words, detail).held_by(holders))
}

/// Who was still writing `path`, in words: the writers the tool's adapter
/// names now (AJ-E2318, the review's E16: "claude (process 4242) was
/// still writing this conversation"); else, the backup for writers no
/// hook names, the profile whose folders hold it ("Claude app · Work was
/// still writing this conversation as it closed"), else the account whose
/// sessions write there.
pub(super) fn still_writing(
    state: &Arc<AppState>,
    sides: &Sides,
    path: &Path,
    system: &dyn MoveSystem,
    writing: &Writing<'_>,
) -> (String, Vec<MoveHolder>) {
    let table = system.table(state);
    let named: Vec<(String, String)> = quit::writers_now(&table, writing)
        .iter()
        .filter_map(|writer| {
            let name = table.get(&writer.pid)?.exe.as_deref()?.file_name()?;
            let name = name.to_string_lossy().into_owned();
            Some((format!("{name} (process {})", writer.pid), name))
        })
        .collect();
    if !named.is_empty() {
        let words = format!(
            "{} {} still writing this conversation",
            named
                .iter()
                .map(|(said, _)| said.as_str())
                .collect::<Vec<_>>()
                .join(" and "),
            if named.len() == 1 { "was" } else { "were" }
        );
        let holders = named
            .into_iter()
            .map(|(_, name)| MoveHolder {
                name,
                place_id: None,
                ..MoveHolder::default()
            })
            .collect();
        return (words, holders);
    }
    let both = [
        (
            &sides.request.source,
            &sides.source_place,
            &sides.source_label,
        ),
        (
            &sides.request.target,
            &sides.target_place,
            &sides.target_label,
        ),
    ];
    for (side, place, label) in both {
        let holds = path.starts_with(&side.tool_dir)
            || side
                .host_dir
                .as_deref()
                .is_some_and(|dir| path.starts_with(dir));
        if holds {
            return match place {
                Some((id, place)) => {
                    let name = place_name(state, place);
                    (
                        format!("{name} was still writing this conversation as it closed"),
                        vec![MoveHolder {
                            name,
                            place_id: Some(id.clone()),
                            ..MoveHolder::default()
                        }],
                    )
                }
                None => (
                    format!("a session on {label} was still writing this conversation"),
                    Vec::new(),
                ),
            };
        }
    }
    (
        "another app was still writing this conversation".to_owned(),
        Vec::new(),
    )
}

/// A size in words ("406 MB"), as the switch screen says sizes.
pub(super) fn size_words(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss, reason = "a size in words")]
    let bytes = bytes as f64;
    if bytes >= 1e9 {
        format!("{:.1} GB", bytes / 1e9)
    } else if bytes >= 1e6 {
        format!("{:.0} MB", bytes / 1e6)
    } else {
        format!("{:.0} KB", (bytes / 1e3).max(1.0))
    }
}
