//! The company at one moment (AJ-E3505, ADR-0026 §5): what the Floor
//! draws and what the main window's Floor list shows, from one build, so
//! the two never disagree. Only what the backend knows; every state comes
//! from [`crate::state`], the rail's own rules.
//!
//! The backend (`juggler-app`'s `features/floor`) builds it; the render
//! thread and the UI read it. Nothing here reads a file or a clock.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::state::FloorState;

/// How an App is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum FloorAppKind {
    /// A command-line tool on a folder.
    Cli,
    /// A desktop app, or one of its profiles.
    Desktop,
}

/// One session at a desk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorSession {
    /// The rail's key: `s:<id>` for AI Juggler's own, `c:<account>/<id>`
    /// for another app's. A click opens it.
    pub key: String,
    /// Its title, as the rail says it.
    pub title: String,
    /// How the Floor draws it.
    pub state: FloorState,
    /// Its state in words ("Probably stuck").
    pub word: String,
    /// Subagents running.
    pub subagents: u32,
    /// Its project's name, when it has one.
    pub project: Option<String>,
    /// Its open tasks.
    pub tasks: u32,
    /// Its processes' CPU, percent of one core (the fire goes on the
    /// busiest desk).
    pub cpu_pct: f32,
    /// When it started (epoch ms): desks go in this order.
    #[ts(type = "number")]
    pub started_ms: i64,
    /// Its last activity (epoch ms).
    #[ts(type = "number")]
    pub last_ms: i64,
}

/// One App's area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorApp {
    /// The App's id (D-62).
    pub id: String,
    /// Its name ("Claude Code · Work Max").
    pub name: String,
    /// Its tool, for its mark (drawn by the UI; never branched on).
    pub tool: String,
    /// A desktop App's host app (the registry's id), for its own icon.
    pub host_app: Option<String>,
    /// The print of its floor logo kept now (AJ-E3508), when one is.
    pub logo: Option<String>,
    /// How it's used.
    pub kind: FloorAppKind,
    /// Its tool's style family, by word.
    pub family: String,
    /// Its colour slot (`MARK_COLOURS`).
    pub colour_slot: u8,
    /// Its Account, when it has one.
    pub account_id: Option<String>,
    /// Its Account's initials, for the floor logo.
    pub initials: String,
    /// Open: running now, or with a live session.
    pub open: bool,
    /// Its last activity (epoch ms), for "unused".
    #[ts(type = "number | null")]
    pub last_active_ms: Option<i64>,
    /// Its processes' CPU, in percent of one core.
    pub cpu_pct: f32,
    /// Since when it has been over the overload threshold (epoch ms).
    #[ts(type = "number | null")]
    pub hot_since_ms: Option<i64>,
    /// Its sessions, live ones first, then by start.
    pub sessions: Vec<FloorSession>,
}

/// One Account's usage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorAccount {
    /// Its id.
    pub id: String,
    /// Its name.
    pub label: String,
    /// What's left of its tightest limit, 0–100, when known.
    pub left_pct: Option<f32>,
    /// When that limit resets (epoch ms), when known.
    #[ts(type = "number | null")]
    pub resets_ms: Option<i64>,
    /// Its last use (epoch ms).
    #[ts(type = "number | null")]
    pub last_used_ms: Option<i64>,
    /// Out: at a limit now.
    pub out: bool,
}

/// One pool and the sessions in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorPool {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Sealed: its sessions are in no other pool.
    pub sealed: bool,
    /// Its sessions, by the rail's key.
    pub sessions: Vec<String>,
}

/// This Mac's health.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorMachine {
    /// Its load, 0–1 (the rack's lights).
    pub load: f32,
    /// Since when its overload alert has been on (epoch ms).
    #[ts(type = "number | null")]
    pub alert_since_ms: Option<i64>,
    /// The alert in words ("CPU 96% for 6 minutes").
    pub alert: Option<String>,
}

/// The company at one moment.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FloorSnapshot {
    /// When it was built (epoch ms).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Every App the Floor shows, in no particular order (the office
    /// model orders them).
    pub apps: Vec<FloorApp>,
    /// Every Account.
    pub accounts: Vec<FloorAccount>,
    /// Every pool.
    pub pools: Vec<FloorPool>,
    /// This Mac.
    pub machine: FloorMachine,
    /// Unread notifications, for the lobby's board.
    pub notices: u32,
}

impl FloorSnapshot {
    /// Sessions waiting on the user (a question, a permission, stuck, out),
    /// for N (the next one) and the list's count.
    #[must_use]
    pub fn waiting(&self) -> Vec<&FloorSession> {
        self.apps
            .iter()
            .flat_map(|app| &app.sessions)
            .filter(|s| {
                matches!(
                    s.state,
                    FloorState::Asks | FloorState::Stuck | FloorState::Limit
                )
            })
            .collect()
    }
}
