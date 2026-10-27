//! The subagents' wire types (AJ-E1520).

use juggler_adapters::{SubtaskLog, SubtaskState};
use serde::Serialize;
use ts_rs::TS;

/// A session's subagents, for the inspector.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubtasksInfo {
    /// Every subagent, in the order they were started.
    pub items: Vec<SubtaskInfo>,
    /// Whether the session is still running. Once it ended, a subagent
    /// the transcript never saw end is shown as `unfinished`.
    pub live: bool,
}

/// One subagent.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubtaskInfo {
    /// The call that started it.
    pub id: String,
    /// What it was asked to do, on one line.
    pub label: String,
    /// The kind of agent, when the call named one.
    pub kind: Option<String>,
    /// Where it stands.
    pub state: SubtaskStateInfo,
}

/// Where a subagent stands, as the inspector shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum SubtaskStateInfo {
    /// Still working.
    Running,
    /// Started in the background; the transcript doesn't say when it ends.
    Background,
    /// Finished, or closed by its session.
    Done,
    /// Ended with an error, or never started.
    Failed,
    /// Stopped before it finished.
    Stopped,
    /// Its session ended before the transcript said it had.
    Unfinished,
}

impl SubtasksInfo {
    pub(super) fn new(log: Option<&SubtaskLog>, live: bool) -> Self {
        let items = log
            .map(SubtaskLog::subtasks)
            .unwrap_or_default()
            .iter()
            .map(|item| SubtaskInfo {
                id: item.id.clone(),
                label: item.label.clone(),
                kind: item.kind.clone(),
                state: match (item.state, live) {
                    (SubtaskState::Running, true) => SubtaskStateInfo::Running,
                    (SubtaskState::Running, false) => SubtaskStateInfo::Unfinished,
                    (SubtaskState::Background, _) => SubtaskStateInfo::Background,
                    (SubtaskState::Done, _) => SubtaskStateInfo::Done,
                    (SubtaskState::Failed, _) => SubtaskStateInfo::Failed,
                    (SubtaskState::Stopped, _) => SubtaskStateInfo::Stopped,
                },
            })
            .collect();
        Self { items, live }
    }
}
