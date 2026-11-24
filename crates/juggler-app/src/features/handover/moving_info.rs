//! ADR-0031: what a move tells the window (the switch screen's offer,
//! progress, outcomes and Undo), split from `moving.rs` by feature.

use juggler_adapter::moving::{Refusal, RefusalKind};

/// How a move is going, for the switch screen while it runs (0.4.9: a
/// long backup never looks frozen).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveProgressInfo {
    /// `closing`, `still`, `backing_up`, `moving`, `starting` or
    /// `checking`.
    pub stage: String,
    /// What it is doing, in words ("Waiting for Claude app · Work to
    /// finish closing…").
    pub words: String,
    /// Bytes done of `total` (both 0 where there's nothing to count).
    #[ts(type = "number")]
    pub done: u64,
    /// See `done`.
    #[ts(type = "number")]
    pub total: u64,
}

impl MoveProgressInfo {
    /// A stage with nothing to count.
    pub(crate) fn step(stage: &str, words: String) -> Self {
        Self {
            stage: stage.to_owned(),
            words,
            done: 0,
            total: 0,
        }
    }
}

/// One conversation of the family, as the switch screen lists it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveMemberInfo {
    /// The app's title, else the conversation's id.
    pub title: String,
    /// Archived in the app (it stays archived).
    pub archived: bool,
    /// It moves (`moves`), or it is already in the target and is linked
    /// to the moved one (`relinked`).
    pub side: String,
    /// Its working folder is gone (its history is kept).
    pub folder_missing: bool,
}

/// Whether the user can clear what stands in Move's way (D-127).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum MoveBlockKind {
    /// The user can: work to finish, a session to end, a profile to sign
    /// in to, room to free. Move is shown, waiting, with the how.
    UserCanClear,
    /// The user can't (the tool's data in a shape the move doesn't know,
    /// the disks, the kinds of the two sides, one folder for both): Move
    /// isn't shown, and Copy is the switch.
    Setup,
}

/// One thing in Move's way.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveBlockInfo {
    /// Whether the user can clear it.
    pub kind: MoveBlockKind,
    /// In words; for one the user can clear, what to do ("…; Move becomes
    /// available once they finish").
    pub words: String,
}

impl From<Refusal> for MoveBlockInfo {
    fn from(refusal: Refusal) -> Self {
        Self {
            kind: match refusal.kind {
                RefusalKind::UserCanClear => MoveBlockKind::UserCanClear,
                RefusalKind::Setup => MoveBlockKind::Setup,
            },
            words: refusal.words,
        }
    }
}

/// Whether and how a conversation can be moved, for the switch screen.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveOfferInfo {
    /// The tool can plan a move between these two accounts at all.
    pub applies: bool,
    /// What stands in the way now (empty: Move can happen, the default).
    /// A `setup` one hides Move; one the user can clear shows it waiting.
    pub blocks: Vec<MoveBlockInfo>,
    /// 0.5.1 F72: what holds the files while one the user can clear
    /// stands in the way, each with its door (AJ-E2319's holders).
    pub holders: Vec<crate::error::MoveHolder>,
    /// The family, the moved conversation first.
    pub members: Vec<MoveMemberInfo>,
    /// The profiles that quit and start again ("Claude app · Work").
    pub restarts: Vec<String>,
    /// What the user should know (a kept branch, a missing folder).
    pub notes: Vec<String>,
    /// The project's notes differ in the two accounts (the user may take
    /// the conversation's).
    pub notes_conflict: Option<String>,
    /// Output folders that stay where old messages name them.
    pub kept_outputs: usize,
    /// The family's bytes (moved, not copied: no extra room).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Desktop app profiles (true) or tool folders in a terminal (false).
    pub app: bool,
    /// AJ-E2328: what moves with it beside its messages, in words ("Its
    /// worktree fix-login (branch main): Claude app · Work owns it
    /// after the move"); empty when there is nothing.
    pub carried: Vec<String>,
}

/// One of a move's four outcomes.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveOutcomeInfo {
    /// `verified`, `failed`, or `pending` (not something AI Juggler can
    /// check by itself).
    pub state: String,
    /// In words.
    pub detail: String,
}

/// What an Undo did (R4, the 0.5.0 re-walk): its words, never a path, and
/// where what changed since the move is kept, for Copy details only.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveUndoneInfo {
    /// "Undone: the conversation is back where it was. …".
    pub words: String,
    /// The folder that keeps what was done since the move.
    pub kept: String,
    /// What of the profiles kept running when they quit (a server they
    /// started), in words: as the move itself says it (AJ-E2319 R5).
    pub notes: Vec<String>,
}

/// What a move did.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveDoneInfo {
    /// The move's id (for Undo).
    pub move_id: String,
    /// The conversation's files: every byte where the plan put it, before
    /// and after the apps started again.
    pub payload: MoveOutcomeInfo,
    /// The app's list: every member listed under the target account,
    /// linked as it was, no record left on the source.
    pub catalog: MoveOutcomeInfo,
    /// Opened in the target profile, by its own process.
    pub opened: MoveOutcomeInfo,
    /// A reply from the model on the target account: only the user can
    /// check it (a move never sends a prompt).
    pub model: MoveOutcomeInfo,
    /// What the user should know.
    pub notes: Vec<String>,
    /// AJ-E2322: the App to offer Open in, when the conversation isn't
    /// seen open there (`None`: it is, or it moved to a terminal folder).
    pub open_place_id: Option<String>,
    /// The moved conversation's id (what Open opens).
    pub conversation: String,
}
