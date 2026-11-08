//! ADR-0031 (D-93): a switch by **move**. Where one tool's two accounts
//! keep their conversations on one disk in the same format (two profiles
//! of the Claude app, two `CLAUDE_CONFIG_DIR` folders of Claude Code), a
//! conversation and its whole family can be moved into the other account's
//! folders: the same ids, the full history, its sub-sessions and rewind,
//! opened there under that account's own sign-in. Nothing is rebuilt.
//!
//! The adapter knows the tool's layout, so it **plans**: it reads both
//! sides (never writes) and says, step by step, what moves where, what is
//! put aside, which records are written, and what stays where it is on
//! purpose. AI Juggler's transaction engine (`juggler-handover::moving`)
//! applies the plan with backups, a journal and checks, the same for every
//! tool. No tool's name appears outside its adapter.
//!
//! A move is offered only where it is proven (`in_place_move`,
//! [`crate::switching::SwitchCapability::InPlaceMove`]); anything the
//! adapter can't plan exactly is a refusal in words, and the user still
//! has Copy.

use std::path::{Path, PathBuf};

use crate::error::AdapterError;

/// One side of a move: the account's tool folder and, when the
/// conversation lives in a desktop app, that app instance's folder.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveSide {
    /// The account's tool folder (Claude Code's `CLAUDE_CONFIG_DIR`).
    pub tool_dir: PathBuf,
    /// The desktop app's instance folder (its `--user-data-dir`), when
    /// this side is an app profile.
    pub host_dir: Option<PathBuf>,
    /// The installed version of the app (or, without one, the tool) on
    /// this side, for the adapter's proven-version gate.
    pub version: Option<String>,
}

/// What to do with the project's shared notes when both accounts have
/// their own and they differ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotesChoice {
    /// The target keeps its own notes (nothing project-wide changes).
    #[default]
    KeepTarget,
    /// The conversation's notes go with it; the target's are put aside
    /// (they are shared by every conversation of the project there).
    TakeSource,
}

/// A request to plan a move.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveRequest {
    /// The conversation's id in the tool (Claude Code's session UUID).
    pub conversation: String,
    /// Where it is.
    pub source: MoveSide,
    /// Where it goes.
    pub target: MoveSide,
    /// The notes choice, when the plan names a conflict.
    pub notes: NotesChoice,
}

/// Where a member of the family was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberSide {
    /// In the source account: it moves.
    Source,
    /// In the target account already, under an older record of the same
    /// conversation (an alias): it is re-linked to the moved family.
    Target,
}

/// One conversation of the family.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveMember {
    /// The app's own id for it (Claude app: `local_…`), when it has one.
    /// Not the conversation's id: the two differ.
    pub record_id: Option<String>,
    /// The conversation's id in the tool.
    pub conversation: String,
    /// Its parent's app id, when it was started from another.
    pub parent: Option<String>,
    /// The app's title for it.
    pub title: Option<String>,
    /// Archived in the app (kept as it is).
    pub archived: bool,
    /// Where it was found.
    pub side: MemberSide,
    /// Its history file, where it is now.
    pub transcript: Option<PathBuf>,
    /// Where its history file goes.
    pub transcript_to: Option<PathBuf>,
    /// Its own nested agents' histories.
    pub subagents: usize,
    /// Its bytes in all (history, nested agents, rewind).
    pub bytes: u64,
    /// Its working folder is gone (an archived worktree): its history is
    /// kept, nothing is recreated.
    pub folder_missing: bool,
}

/// Why something is put aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldWhy {
    /// The target's own, different version of the same conversation: kept,
    /// never joined to or overwritten by the moved one.
    DivergentBranch,
    /// The source app's record of a moved conversation, so one account
    /// owns it.
    RetiredRecord,
    /// The target's project notes, replaced by the conversation's (the
    /// user chose it).
    ReplacedNotes,
}

/// One step of a plan. The engine journals, applies and checks each, and
/// can undo every one from its backups.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MoveStep {
    /// Makes a folder (and nothing else) where moved files go.
    MakeDir {
        /// The folder.
        path: PathBuf,
    },
    /// Renames a file or folder to a place on the same disk, keeping its
    /// identity. `history` marks a conversation's history file, whose
    /// first bytes are checked again after the app runs (it may append).
    Rename {
        /// Where it is.
        from: PathBuf,
        /// Where it goes (must not exist).
        to: PathBuf,
        /// It is a history file the app appends to.
        history: bool,
    },
    /// Puts what is at `path` aside in the move's holding place.
    Hold {
        /// What is put aside.
        path: PathBuf,
        /// Why.
        why: HoldWhy,
    },
    /// Writes an app record: a new one, or one whose structured fields
    /// changed (every field the adapter doesn't know is kept as it was).
    WriteRecord {
        /// The record's file.
        path: PathBuf,
        /// Its whole new content.
        value: serde_json::Value,
    },
    /// Clones a file or folder to a new place (the copy is independent).
    Clone {
        /// The original.
        from: PathBuf,
        /// The new place (must not exist).
        to: PathBuf,
    },
}

impl MoveStep {
    /// Every path this step reads or writes.
    #[must_use]
    pub fn paths(&self) -> Vec<&Path> {
        match self {
            Self::MakeDir { path } | Self::Hold { path, .. } | Self::WriteRecord { path, .. } => {
                vec![path]
            }
            Self::Rename { from, to, .. } | Self::Clone { from, to } => vec![from, to],
        }
    }
}

/// A file or folder left where it is on purpose, and why: cleanup must
/// keep it while anything still names it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Retained {
    /// The path.
    pub path: PathBuf,
    /// The conversation that depends on it.
    pub conversation: String,
    /// Why, in words.
    pub why: String,
}

/// A read-only plan: everything a move will do, worked out before
/// anything changes.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovePlan {
    /// The conversation the user moves (its id in the tool).
    pub conversation: String,
    /// What the target opens after the move (its id in the tool).
    pub resume: String,
    /// Every conversation of the family, the moved one first.
    pub members: Vec<MoveMember>,
    /// The steps, in order.
    pub steps: Vec<MoveStep>,
    /// What stays where it is on purpose.
    pub retained: Vec<Retained>,
    /// The folders every step must stay inside (both sides' tool and app
    /// folders): the engine refuses a step outside them.
    pub roots: Vec<PathBuf>,
    /// What the user should know (a missing worktree, a kept branch, a
    /// notes conflict), in words.
    pub notes: Vec<String>,
    /// The project notes differ on the two sides (the user may choose).
    pub notes_conflict: Option<String>,
    /// Why it can't move, in words. A plan with any is never applied.
    pub refusals: Vec<String>,
}

impl MovePlan {
    /// Whether the plan can be applied.
    #[must_use]
    pub fn can_move(&self) -> bool {
        self.refusals.is_empty() && !self.steps.is_empty()
    }

    /// The plan's shape, for the check after the apps quit: the family and
    /// every path a step touches. Two plans of one shape move the same
    /// things (record contents may differ: the app writes at quit).
    #[must_use]
    pub fn shape(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .members
            .iter()
            .map(|m| {
                format!(
                    "member {} {} {:?}",
                    m.record_id.as_deref().unwrap_or("-"),
                    m.conversation,
                    m.side
                )
            })
            .collect();
        for step in &self.steps {
            let kind = match step {
                MoveStep::MakeDir { .. } => "dir",
                MoveStep::Rename { .. } => "rename",
                MoveStep::Hold { .. } => "hold",
                MoveStep::WriteRecord { .. } => "write",
                MoveStep::Clone { .. } => "clone",
            };
            let paths: Vec<String> = step
                .paths()
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            out.push(format!("{kind} {}", paths.join(" -> ")));
        }
        out
    }
}

/// One check the adapter makes of its own records after a move.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveCheck {
    /// What was checked, in words.
    pub what: String,
    /// Whether it holds.
    pub pass: bool,
}

/// What a line the app appended to a moved history after it started is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appended {
    /// The app's own bookkeeping (titles, modes, summaries).
    Bookkeeping,
    /// A notice the app wrote into the history only (a background command
    /// that stopped when the app quit): not a turn.
    SystemNotice,
    /// A turn: someone typed, or the model answered. A move itself never
    /// writes one.
    Turn,
    /// Not a line of a shape the adapter knows.
    Unknown,
}

/// A tool's in-place move ([`crate::ToolHandover::in_place_move`]).
pub trait InPlaceMove: Send + Sync {
    /// Plans a move of `request.conversation` and its whole family from
    /// `request.source` to `request.target`. Reads only. A plan the
    /// adapter can't make exactly carries its refusals.
    ///
    /// # Errors
    ///
    /// A side can't be read at all.
    fn plan(&self, request: &MoveRequest) -> Result<MovePlan, AdapterError>;

    /// The adapter's checks of its own records once `plan` was applied
    /// (before the apps start again): every member is where the plan put
    /// it, linked to the parent it should be, archived as it was, and no
    /// record still claims a moved conversation on the source.
    fn verify(&self, request: &MoveRequest, plan: &MovePlan) -> Vec<MoveCheck>;

    /// What one line the app appended to a moved history is.
    fn appended(&self, line: &[u8]) -> Appended;

    /// The link that opens `conversation` in the target's app, sent to
    /// that app's own process.
    fn resume_link(&self, conversation: &str) -> Option<String>;
}

/// Whether `path` is inside one of `roots` (by its components, never by
/// following links).
#[must_use]
pub fn contained(path: &Path, roots: &[PathBuf]) -> bool {
    let plain = path
        .components()
        .all(|part| !matches!(part, std::path::Component::ParentDir));
    plain && roots.iter().any(|root| path.starts_with(root) && path != root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_outside_the_roots_or_climbing_out_is_not_contained() {
        let roots = vec![PathBuf::from("/a/profile")];
        assert!(contained(Path::new("/a/profile/projects/x.jsonl"), &roots));
        assert!(!contained(Path::new("/a/profile"), &roots));
        assert!(!contained(Path::new("/a/profile-two/x"), &roots));
        assert!(!contained(Path::new("/a/profile/../other/x"), &roots));
    }

    #[test]
    fn a_plans_shape_names_members_and_paths_not_contents() {
        let mut plan = MovePlan {
            members: vec![MoveMember {
                record_id: Some("local_a".into()),
                conversation: "c".into(),
                parent: None,
                title: None,
                archived: false,
                side: MemberSide::Source,
                transcript: None,
                transcript_to: None,
                subagents: 0,
                bytes: 0,
                folder_missing: false,
            }],
            steps: vec![MoveStep::WriteRecord {
                path: "/p/r.json".into(),
                value: serde_json::json!({"a": 1}),
            }],
            ..MovePlan::default()
        };
        let before = plan.shape();
        plan.steps[0] = MoveStep::WriteRecord {
            path: "/p/r.json".into(),
            value: serde_json::json!({"a": 2}),
        };
        assert_eq!(plan.shape(), before);
        assert!(plan.can_move());
        plan.refusals.push("no".into());
        assert!(!plan.can_move());
    }
}
