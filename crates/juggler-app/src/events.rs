//! AJ-E2017 R2: every event the backend emits to the UI, as one enum.
//!
//! Emitting goes through [`EmitEvent::emit_event`] with an [`AppEvent`],
//! so a misspelt event name can't compile; [`AppEvent::name`] is the
//! wire string the UI listens to (`ui/src/lib/bridge.ts`'s `EVENTS` and
//! the feature modules' `listen` calls), unchanged by this enum. The
//! dev bridge's forwarding lists name their events through it too.
//!
//! A new event is a new variant here, its wire name in [`AppEvent::name`],
//! and its listener in the UI.

use serde::Serialize;
use tauri::Emitter;

/// An event the backend emits to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppEvent {
    /// The app's setup finished (`app://ready`, carries [`crate::features::app::dto::AppInfo`]).
    AppReady,
    /// The sessions changed; the UI reads its lists again.
    SessionsUpdated,
    /// The 5 s resource tick (I-07/I-08 deltas; AJ-E401).
    SessionsResources,
    /// The 30 s quota tick (I-05 deltas; AJ-E401).
    SessionsQuota,
    /// This Mac's health changed.
    ThisMac,
    /// The global "Go to session" hotkey (AJ-E414).
    GoTo,
    /// A live-preview frame (AJ-E415).
    PreviewFrame,
    /// A live-preview phase change (AJ-E415).
    PreviewState,
    /// A cross-machine switch's push progress (AJ-E706).
    FleetSwitchProgress,
    /// A backup export's progress (AJ-E1009).
    BackupProgress,
    /// The update status changed (AJ-E1010).
    UpdateStatus,
    /// The native window asked to quit; the UI shows the quit plan.
    QuitRequested,
    /// The app menu's Report a bug item.
    ReportBug,
    /// An embedded terminal's output.
    TerminalOutput,
    /// An embedded terminal exited.
    TerminalExit,
    /// The popped-out terminal windows changed.
    TerminalWindows,
    /// The tasks changed.
    TasksChanged,
    /// The messages changed.
    MessagesChanged,
    /// The processes changed.
    ProcessesChanged,
    /// The places (apps and profiles) changed.
    PlacesChanged,
    /// A session's runtime changed.
    RuntimeChanged,
    /// The conversations changed (the index, or a live one).
    ConversationsChanged,
    /// The menu-bar item or the Floor asks the window to open something.
    AppOpen,
    /// The notifications changed.
    NotificationsChanged,
    /// The menu-bar item asks the window to open the notifications.
    NotificationsOpen,
    /// The menu-bar item asks the window for a new session.
    NewSession,
    /// The Floor's snapshot changed.
    FloorChanged,
    /// The Floor's window couldn't open.
    FloorUnavailable,
    /// The Floor's window is asked to open.
    FloorOpenRequest,
}

impl AppEvent {
    /// Every event, in declaration order.
    pub const ALL: [Self; 29] = [
        Self::AppReady,
        Self::SessionsUpdated,
        Self::SessionsResources,
        Self::SessionsQuota,
        Self::ThisMac,
        Self::GoTo,
        Self::PreviewFrame,
        Self::PreviewState,
        Self::FleetSwitchProgress,
        Self::BackupProgress,
        Self::UpdateStatus,
        Self::QuitRequested,
        Self::ReportBug,
        Self::TerminalOutput,
        Self::TerminalExit,
        Self::TerminalWindows,
        Self::TasksChanged,
        Self::MessagesChanged,
        Self::ProcessesChanged,
        Self::PlacesChanged,
        Self::RuntimeChanged,
        Self::ConversationsChanged,
        Self::AppOpen,
        Self::NotificationsChanged,
        Self::NotificationsOpen,
        Self::NewSession,
        Self::FloorChanged,
        Self::FloorUnavailable,
        Self::FloorOpenRequest,
    ];

    /// The wire name the UI listens to.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AppReady => "app://ready",
            Self::SessionsUpdated => "session://updated",
            Self::SessionsResources => "session://resources",
            Self::SessionsQuota => "session://quota",
            Self::ThisMac => "machine://this-mac",
            Self::GoTo => "app://go-to",
            Self::PreviewFrame => "preview://frame",
            Self::PreviewState => "preview://state",
            Self::FleetSwitchProgress => "fleet://switch-progress",
            Self::BackupProgress => "backup://progress",
            Self::UpdateStatus => "update://status",
            Self::QuitRequested => "app://quit-requested",
            Self::ReportBug => "app://report-bug",
            Self::TerminalOutput => "terminal://output",
            Self::TerminalExit => "terminal://exit",
            Self::TerminalWindows => "terminal://windows",
            Self::TasksChanged => "tasks://changed",
            Self::MessagesChanged => "messages://changed",
            Self::ProcessesChanged => "processes://changed",
            Self::PlacesChanged => "places-changed",
            Self::RuntimeChanged => "runtime://changed",
            Self::ConversationsChanged => "conversations://changed",
            Self::AppOpen => "app://open",
            Self::NotificationsChanged => "notifications://changed",
            Self::NotificationsOpen => "notifications://open",
            Self::NewSession => "app://new-session",
            Self::FloorChanged => "floor://changed",
            Self::FloorUnavailable => "floor://unavailable",
            Self::FloorOpenRequest => "floor://open-request",
        }
    }
}

/// Emitting an [`AppEvent`]: `app.emit_event(AppEvent::SessionsUpdated, ())`
/// on anything that can emit (the app handle, a window). A failed emit is
/// dropped, as it always was: the UI reads again on its next event or
/// when a page opens.
pub trait EmitEvent<R: tauri::Runtime>: Emitter<R> {
    /// Emits `event` with `payload` to every window.
    fn emit_event<S: Serialize + Clone>(&self, event: AppEvent, payload: S) {
        let _ = self.emit(event.name(), payload);
    }

    /// Emits `event` with `payload` to one window (`label`).
    fn emit_event_to<S: Serialize + Clone>(&self, label: &str, event: AppEvent, payload: S) {
        let _ = self.emit_to(label, event.name(), payload);
    }
}

impl<R: tauri::Runtime, T: Emitter<R>> EmitEvent<R> for T {}

#[cfg(test)]
mod tests {
    use super::AppEvent;

    #[test]
    fn every_event_has_its_own_wire_name() {
        let mut names: Vec<&str> = AppEvent::ALL.iter().map(|event| event.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two events share a wire name");
    }

    /// Emitting goes through the enum: no `.emit(` or `.emit_to(` with a
    /// string outside this module (the Floor's own files excepted until
    /// they move over).
    #[test]
    fn nothing_emits_by_a_bare_name() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    out.push(path);
                }
            }
        }
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        walk(&src, &mut files);
        let mut bare = Vec::new();
        for file in files {
            let rel = file
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == "events.rs"
                || rel.starts_with("features/floor/")
                || rel == "dev_bridge/tests.rs"
            {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            for (index, line) in text.lines().enumerate() {
                if line.contains(".emit(") || line.contains(".emit_to(") {
                    bare.push(format!("{rel}:{}", index + 1));
                }
            }
        }
        assert!(
            bare.is_empty(),
            "emit through `EmitEvent::emit_event`: {bare:?}"
        );
    }
}
