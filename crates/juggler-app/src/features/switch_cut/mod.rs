//! D-76 (AJ-E211): leaving out the start of a very large conversation
//! when switching, under the switch's advanced options. By default
//! everything travels; this says, for a choice, what would be left out,
//! what would be carried, and what the new session won't know.

use std::sync::Arc;

use juggler_adapter::handover::{Cut, CutPreview};
use juggler_core::model::TranscriptRef;
use juggler_supervisor::machine::bytes_words;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A choice of what to leave out, as the window sends it: one of the two,
/// or neither (everything travels).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchCutArgs {
    /// Leave out the turns before this time (epoch ms).
    #[ts(type = "number | null")]
    pub before_ms: Option<i64>,
    /// Keep only the last this many turns.
    pub keep_last: Option<u32>,
}

impl SwitchCutArgs {
    /// The cut it asks for; `None`: everything travels.
    #[must_use]
    pub fn cut(self) -> Option<Cut> {
        match (self.before_ms, self.keep_last) {
            (Some(ms), _) => Some(Cut::Before(ms)),
            (None, Some(n)) => Some(Cut::KeepLast(n as usize)),
            (None, None) => None,
        }
    }
}

/// What a choice would leave out and carry, for the switch screen (the
/// window adds the dates, in the user's own format).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchCutInfo {
    /// This switch can leave part out (the same tool on both sides, a
    /// tool that knows how); else the options don't show.
    pub supported: bool,
    /// The person's turns in the conversation.
    pub turns: u32,
    /// When the first and last were written (epoch ms).
    #[ts(type = "number | null")]
    pub first_ms: Option<i64>,
    /// When the last was written.
    #[ts(type = "number | null")]
    pub last_ms: Option<i64>,
    /// The conversation's size.
    #[ts(type = "number")]
    pub bytes: u64,
    /// With the choice: the turns left out (0: nothing would be).
    pub turns_left_out: u32,
    /// The size left out.
    #[ts(type = "number")]
    pub bytes_left_out: u64,
    /// When the first kept turn was written.
    #[ts(type = "number | null")]
    pub from_ms: Option<i64>,
    /// It starts at the tool's own compaction (its summary of before).
    pub at_compaction: bool,
    /// What it carries and leaves out, in one sentence.
    pub words: String,
    /// What the new session won't know, when anything is left out.
    pub loses: Option<String>,
}

fn unsupported() -> SwitchCutInfo {
    SwitchCutInfo {
        supported: false,
        turns: 0,
        first_ms: None,
        last_ms: None,
        bytes: 0,
        turns_left_out: 0,
        bytes_left_out: 0,
        from_ms: None,
        at_compaction: false,
        words: String::new(),
        loses: None,
    }
}

fn turns_words(n: u32) -> String {
    format!(
        "{} {}",
        juggler_handover::pipeline::thousands(u64::from(n)),
        if n == 1 { "turn" } else { "turns" }
    )
}

/// The words for `preview`.
#[must_use]
pub fn info_of(preview: &CutPreview) -> SwitchCutInfo {
    let turns = u32::try_from(preview.turns).unwrap_or(u32::MAX);
    let left = u32::try_from(preview.turns_left_out).unwrap_or(u32::MAX);
    let kept = turns.saturating_sub(left);
    let carried = preview.bytes.saturating_sub(preview.bytes_left_out);
    let (words, loses) = if left == 0 {
        (
            format!(
                "Carries the whole conversation: {} ({}).",
                turns_words(turns),
                bytes_words(preview.bytes)
            ),
            None,
        )
    } else {
        let compaction = if preview.at_compaction {
            " It starts at the summary the tool made when it last compacted the conversation."
        } else {
            ""
        };
        (
            format!(
                "Carries the last {} ({}) and leaves out {} ({}).{compaction}",
                turns_words(kept),
                bytes_words(carried),
                turns_words(left),
                bytes_words(preview.bytes_left_out),
            ),
            Some(format!(
                "The new session won't know what was said or decided before then: the tasks, decisions and file changes from the {} left out. The original keeps everything.",
                turns_words(left)
            )),
        )
    };
    SwitchCutInfo {
        supported: true,
        turns,
        first_ms: preview.first_ms,
        last_ms: preview.last_ms,
        bytes: preview.bytes,
        turns_left_out: left,
        bytes_left_out: preview.bytes_left_out,
        from_ms: preview.from_ms,
        at_compaction: preview.at_compaction,
        words,
        loses,
    }
}

/// What `args` would leave out of `session_id`'s conversation, switched to
/// `target_profile_id`.
///
/// # Errors
///
/// A session or profile that isn't there; a transcript that can't be read.
pub fn preview_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    args: SwitchCutArgs,
) -> AppResult<SwitchCutInfo> {
    let sides = crate::features::switch_setup::sides(state, session_id, target_profile_id)?;
    let (Some(transcript), true) = (
        sides.transcript,
        sides.source.harness == sides.target.harness,
    ) else {
        return Ok(unsupported());
    };
    let Some(support) = state.registry.handover(&sides.target.harness) else {
        return Ok(unsupported());
    };
    let reference = TranscriptRef {
        native_id: transcript
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned()),
        path: transcript,
    };
    let cut = args.cut().unwrap_or(Cut::KeepLast(usize::MAX));
    match support.cut_preview(&reference, cut) {
        Ok(preview) => Ok(info_of(&preview)),
        Err(juggler_adapter::error::AdapterError::Unsupported(_)) => Ok(unsupported()),
        Err(e) => Err(AppError::Adapter(e)),
    }
}

/// D-76: what a choice would leave out of a conversation, before the
/// switch (its turns read once; changing the choice is quick after).
///
/// # Errors
///
/// As [`preview_impl`].
#[tauri::command]
pub async fn switch_cut_preview(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    cut: Option<SwitchCutArgs>,
) -> AppResult<SwitchCutInfo> {
    crate::ipc::blocking(state.inner(), "switch cut", move |state| {
        preview_impl(
            state,
            &session_id,
            &target_profile_id,
            cut.unwrap_or_default(),
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use juggler_adapter::handover::CutPreview;

    use super::{SwitchCutArgs, info_of};

    #[test]
    fn words_say_what_travels_and_what_the_new_session_wont_know() {
        let whole = info_of(&CutPreview {
            turns: 1_200,
            bytes: 7_800_000_000,
            ..CutPreview::default()
        });
        assert!(
            whole
                .words
                .starts_with("Carries the whole conversation: 1,200 turns")
        );
        assert_eq!(whole.loses, None);
        let cut = info_of(&CutPreview {
            turns: 1_200,
            bytes: 7_800_000_000,
            turns_left_out: 800,
            bytes_left_out: 7_600_000_000,
            from_ms: Some(1_790_931_600_000),
            at_compaction: true,
            ..CutPreview::default()
        });
        assert!(
            cut.words.starts_with("Carries the last 400 turns"),
            "{}",
            cut.words
        );
        assert!(cut.words.contains("leaves out 800 turns"));
        assert!(cut.words.contains("summary the tool made"));
        assert!(
            cut.loses
                .as_deref()
                .unwrap()
                .contains("The original keeps everything")
        );
    }

    #[test]
    fn a_choice_is_one_cut_or_none() {
        assert_eq!(SwitchCutArgs::default().cut(), None);
        assert!(matches!(
            SwitchCutArgs {
                before_ms: None,
                keep_last: Some(10)
            }
            .cut(),
            Some(juggler_adapter::handover::Cut::KeepLast(10))
        ));
    }
}
