//! 0.3.9: the disk space a switch uses, said before it's confirmed. A
//! conversation can be many gigabytes (7.6 GB on the maintainer's Mac);
//! the switch screen says how big it is, how much disk space the copy
//! takes (almost none when it's a clone), how much is free, and, when
//! that's short or low, what to do. The copy itself still checks as it
//! goes ([`juggler_adapter::handover::ensure_room`]).

use std::sync::Arc;

use juggler_adapter::handover::{DISK_RESERVE, DISK_RESERVE_WORDS, SwitchRoom};
use juggler_core::model::TranscriptRef;
use juggler_supervisor::machine::bytes_words;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Under this much free after the copy, the screen suggests making room.
const LOW_AFTER: u64 = 10 * 1000 * 1000 * 1000;

/// The disk space a switch uses, in words for the switch screen.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchSpaceInfo {
    /// `plenty`, `low` (it fits, leaving little), `short` (it doesn't
    /// fit: the switch can't start) or `none` (nothing to say: a switch
    /// to another tool writes only a short handover).
    pub kind: String,
    /// What it uses, in one sentence.
    pub words: String,
    /// What to do about it, when it's low or short.
    pub advice: Option<String>,
    /// The conversation's size.
    #[ts(type = "number")]
    pub carried_bytes: u64,
    /// What the copy writes on disk.
    #[ts(type = "number")]
    pub writes_bytes: u64,
    /// Free on the target's disk, when the system says.
    #[ts(type = "number | null")]
    pub free_bytes: Option<u64>,
}

/// The disk space switching `session_id` to `target_profile_id` uses.
///
/// # Errors
///
/// A session or profile that isn't there; a file whose size can't be read.
pub fn space_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<SwitchSpaceInfo> {
    let sides = crate::features::switch_setup::sides(state, session_id, target_profile_id)?;
    let (Some(transcript), true) = (
        sides.transcript,
        sides.source.harness == sides.target.harness,
    ) else {
        return Ok(nothing());
    };
    let Some(support) = state.registry.handover(&sides.target.harness) else {
        return Ok(nothing());
    };
    let reference = TranscriptRef {
        native_id: transcript
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned()),
        path: transcript,
    };
    let room = support
        .switch_room(&reference, &sides.target)
        .map_err(AppError::Adapter)?;
    let free = sides
        .target
        .state_dir
        .ancestors()
        .find(|dir| dir.is_dir())
        .and_then(|dir| juggler_platform::room::free_space(dir).ok());
    Ok(space(room, free))
}

fn nothing() -> SwitchSpaceInfo {
    SwitchSpaceInfo {
        kind: "none".to_owned(),
        words: String::new(),
        advice: None,
        carried_bytes: 0,
        writes_bytes: 0,
        free_bytes: None,
    }
}

/// The words for `room` with `free` bytes free on the target's disk.
#[must_use]
pub fn space(room: SwitchRoom, free: Option<u64>) -> SwitchSpaceInfo {
    let size = bytes_words(room.carried);
    let uses = if room.writes == 0 {
        "it's copied as a clone, so it takes almost no disk space".to_owned()
    } else if room.unpacks {
        format!(
            "the copy unpacks to its full size as it's made (at least {})",
            bytes_words(room.writes)
        )
    } else {
        format!(
            "the copy takes about {} of disk space",
            bytes_words(room.writes)
        )
    };
    let free_words = free.map_or_else(String::new, |free| format!(" ({} free)", bytes_words(free)));
    let mut info = SwitchSpaceInfo {
        kind: "plenty".to_owned(),
        words: format!("This conversation is {size}; {uses}{free_words}."),
        advice: None,
        carried_bytes: room.carried,
        writes_bytes: room.writes,
        free_bytes: free,
    };
    let Some(free) = free else {
        return info;
    };
    let needed = room.writes.saturating_add(DISK_RESERVE);
    let make_room = "empty the Trash, or remove large files you don't need";
    if free < needed {
        "short".clone_into(&mut info.kind);
        info.words = format!(
            "Not enough disk space: the copy needs about {} and AI Juggler always leaves {} free, but {} is free.",
            bytes_words(room.writes),
            DISK_RESERVE_WORDS,
            bytes_words(free)
        );
        info.advice = Some(format!(
            "Free about {} ({make_room}), then switch. Nothing has moved.",
            bytes_words(needed - free)
        ));
    } else if room.writes > 0 && free - room.writes < LOW_AFTER {
        "low".clone_into(&mut info.kind);
        info.advice = Some(format!(
            "That leaves about {} free. If you can, {make_room} first.",
            bytes_words(free - room.writes)
        ));
    }
    info
}

/// The disk space a switch uses, for the switch screen.
///
/// # Errors
///
/// As [`space_impl`].
#[tauri::command]
pub async fn switch_space_check(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<SwitchSpaceInfo> {
    crate::ipc::blocking(state.inner(), "check the disk space", move |state| {
        space_impl(state, &session_id, &target_profile_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1000 * 1000 * 1000;

    fn room(carried: u64, writes: u64, unpacks: bool) -> SwitchRoom {
        SwitchRoom {
            carried,
            writes,
            unpacks,
        }
    }

    #[test]
    fn a_clone_says_it_takes_almost_nothing() {
        let info = space(room(8 * GB, 0, false), Some(49 * GB));
        assert_eq!(info.kind, "plenty");
        assert_eq!(
            info.words,
            "This conversation is 8.0 GB; it's copied as a clone, so it takes almost no disk space (49 GB free)."
        );
        assert_eq!(info.advice, None);
    }

    #[test]
    fn a_full_copy_says_how_much_and_what_is_free() {
        let info = space(room(800_000_000, 800_000_000, false), Some(49 * GB));
        assert_eq!(info.kind, "plenty");
        assert!(
            info.words
                .contains("the copy takes about 800 MB of disk space (49 GB free)"),
            "{}",
            info.words
        );
    }

    #[test]
    fn little_left_after_says_so_and_how_to_make_room() {
        let info = space(room(8 * GB, 8 * GB, false), Some(16 * GB));
        assert_eq!(info.kind, "low");
        assert_eq!(
            info.advice.as_deref(),
            Some(
                "That leaves about 8.0 GB free. If you can, empty the Trash, or remove large files you don't need first."
            )
        );
    }

    #[test]
    fn not_enough_stops_it_and_says_how_much_to_free() {
        let info = space(room(8 * GB, 8 * GB, false), Some(10 * GB));
        assert_eq!(info.kind, "short");
        assert!(
            info.words
                .starts_with("Not enough disk space: the copy needs about 8.0 GB"),
            "{}",
            info.words
        );
        assert!(
            info.advice
                .as_deref()
                .unwrap()
                .starts_with("Free about 2.0 GB"),
            "{:?}",
            info.advice
        );
    }

    #[test]
    fn a_packed_rollout_says_it_unpacks() {
        let info = space(room(500_000_000, 500_000_000, true), Some(49 * GB));
        assert!(
            info.words
                .contains("unpacks to its full size as it's made (at least 500 MB)"),
            "{}",
            info.words
        );
    }

    #[test]
    fn an_unknown_free_space_still_says_the_size() {
        let info = space(room(8 * GB, 0, false), None);
        assert_eq!(info.kind, "plenty");
        assert!(
            info.words.ends_with("almost no disk space."),
            "{}",
            info.words
        );
    }
}
