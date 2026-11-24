//! AJ-E1547 (PAT-OSP-1): every system prompt AI Juggler can cause, one
//! entry each, written out in `docs/design/os-prompts.md` (a test holds
//! the two together). Built by code audit and Apple's documentation,
//! never by resetting privacy settings.
//!
//! Each entry's `triggers` name every code path that can make the system
//! ask (`file:function`); a test checks each file holds that function.
//! `never_shown` entries are prompts the code is built never to raise (a
//! keychain call that never asks; an app that never opened is refused,
//! not opened): listed so the walk knows what would be a bug.

use std::sync::Arc;

use juggler_platform::os_prompt::{Folder, Kind, Prompt, Target};

use crate::state::AppState;

/// Accessibility.
pub const ACCESSIBILITY: Prompt = Prompt {
    id: "accessibility",
    kind: Kind::Accessibility,
};

/// System Events' bundle id.
pub const SYSTEM_EVENTS_BUNDLE: &str = "com.apple.systemevents";

/// Automation of System Events (window focus).
pub const SYSTEM_EVENTS: Prompt = Prompt {
    id: "automation-system-events",
    kind: Kind::Automation(Target {
        bundle_id: SYSTEM_EVENTS_BUNDLE,
        name: "System Events",
    }),
};

/// Automation of Terminal (selecting a session's tab).
pub const TERMINAL: Prompt = Prompt {
    id: "automation-terminal",
    kind: Kind::Automation(Target {
        bundle_id: "com.apple.Terminal",
        name: "Terminal",
    }),
};

/// Automation of iTerm (selecting a session's tab).
pub const ITERM: Prompt = Prompt {
    id: "automation-iterm",
    kind: Kind::Automation(Target {
        bundle_id: "com.googlecode.iterm2",
        name: "iTerm",
    }),
};

/// Apple events to a desktop app that keeps a session (its link).
pub const APP_LINKS: Prompt = Prompt {
    id: "automation-app-links",
    kind: Kind::Automation(Target {
        bundle_id: "",
        name: "the app",
    }),
};

/// Notifications.
pub const NOTIFICATIONS: Prompt = Prompt {
    id: "notifications",
    kind: Kind::Notifications,
};

/// The login item's notice.
pub const LOGIN_ITEM: Prompt = Prompt {
    id: "login-item",
    kind: Kind::LoginItem,
};

/// Screen Recording (the live preview).
pub const SCREEN_RECORDING: Prompt = Prompt {
    id: "screen-recording",
    kind: Kind::ScreenRecording,
};

/// The local network (Link).
pub const LOCAL_NETWORK: Prompt = Prompt {
    id: "local-network",
    kind: Kind::LocalNetwork,
};

/// `~/Documents`.
pub const DOCUMENTS: Prompt = Prompt {
    id: "documents",
    kind: Kind::Folder(Folder::Documents),
};

/// `~/Desktop`.
pub const DESKTOP: Prompt = Prompt {
    id: "desktop",
    kind: Kind::Folder(Folder::Desktop),
};

/// `~/Downloads`.
pub const DOWNLOADS: Prompt = Prompt {
    id: "downloads",
    kind: Kind::Folder(Folder::Downloads),
};

/// Another app's data.
pub const APP_DATA: Prompt = Prompt {
    id: "app-data",
    kind: Kind::AppData,
};

/// A helper's first start ("differs from previously opened versions").
pub const HELPER: Prompt = Prompt {
    id: "helper",
    kind: Kind::Differs,
};

/// A keychain item's access (never shown).
pub const KEYCHAIN: Prompt = Prompt {
    id: "keychain",
    kind: Kind::Keychain,
};

/// An app's first open (never shown: refused instead).
pub const FIRST_OPEN: Prompt = Prompt {
    id: "first-open",
    kind: Kind::FirstOpen,
};

/// The registry's id for a protected folder.
#[must_use]
pub const fn folder_id(folder: Folder) -> &'static str {
    match folder {
        Folder::Documents => DOCUMENTS.id,
        Folder::Desktop => DESKTOP.id,
        Folder::Downloads => DOWNLOADS.id,
    }
}

/// One entry of the registry.
pub struct Entry {
    /// The gate's prompt (its short, stable id and kind).
    pub prompt: Prompt,
    /// What it makes possible, as the user would say it.
    pub title: &'static str,
    /// Why AI Juggler needs it, and what it never does.
    pub why: &'static str,
    /// What happens if the user declines.
    pub if_declined: &'static str,
    /// What background work does while it isn't primed.
    pub background: &'static str,
    /// Every code path that can make the system ask (`file:function`).
    pub triggers: &'static [&'static str],
    /// Built never to be shown (listed so a walk knows it's a bug).
    pub never_shown: bool,
    /// Whether this setup can use it (D-127: only those are offered).
    pub needed: fn(&Arc<AppState>) -> bool,
}

const WAITS: &str = "Waits: nothing asks until you've said go.";

/// Every prompt, in the order setup offers them.
pub const ENTRIES: &[Entry] = &[
    Entry {
        prompt: ACCESSIBILITY,
        title: "Jump to a session's own window",
        why: "Brings the exact window of a session to the front when you open it. AI Juggler never types, clicks or reads your screen with it.",
        if_declined: "Opening a session still brings its app forward; you pick the window yourself.",
        background: WAITS,
        triggers: &[
            "crates/juggler-app/src/os_access.rs:os_access_request_impl",
            "crates/juggler-app/src/os_prompts/mod.rs:prime_call",
        ],
        never_shown: false,
        needed: super::needs::on_this_system,
    },
    Entry {
        prompt: SYSTEM_EVENTS,
        title: "Find a session's window among an app's windows",
        why: "Asks the system which windows an app has, so the right one comes forward. Nothing else is controlled.",
        if_declined: "Opening a session brings its app forward without picking the window.",
        background: WAITS,
        triggers: &[
            "crates/juggler-app/src/os_access.rs:os_access_request_impl",
            "crates/juggler-app/src/focus.rs:osascript_with",
            "crates/juggler-app/src/features/conversations/goto.rs:osascript_with",
        ],
        never_shown: false,
        needed: super::needs::on_this_system,
    },
    Entry {
        prompt: TERMINAL,
        title: "Select a session's tab in Terminal",
        why: "Picks the tab a session runs in when you open it. AI Juggler only selects the tab.",
        if_declined: "Terminal comes forward; you pick the tab yourself.",
        background: WAITS,
        triggers: &[
            "crates/juggler-app/src/features/conversations/goto.rs:go",
            "crates/juggler-app/src/os_prompts/mod.rs:prime_call",
        ],
        never_shown: false,
        needed: super::needs::terminal_tabs,
    },
    Entry {
        prompt: ITERM,
        title: "Select a session's tab in iTerm",
        why: "Picks the tab a session runs in when you open it. AI Juggler only selects the tab.",
        if_declined: "iTerm comes forward; you pick the tab yourself.",
        background: WAITS,
        triggers: &[
            "crates/juggler-app/src/features/conversations/goto.rs:go",
            "crates/juggler-app/src/os_prompts/mod.rs:prime_call",
        ],
        never_shown: false,
        needed: super::needs::iterm_tabs,
    },
    Entry {
        prompt: APP_LINKS,
        title: "Open a session in its desktop app",
        why: "Sends the app that keeps a session its own link, so the session itself opens, in the right profile. The system asks once per app.",
        if_declined: "The app comes forward; you pick the session in its sidebar.",
        background: "Never asks: links sent without you are sent quietly or not at all.",
        triggers: &[
            "crates/juggler-app/src/features/conversations/goto.rs:send_link_to",
            "crates/juggler-app/src/features/places/handoff.rs:send",
            "crates/juggler-app/src/features/places/open_session.rs:open",
        ],
        never_shown: false,
        needed: super::needs::desktop_apps,
    },
    Entry {
        prompt: NOTIFICATIONS,
        title: "Tell you when a session needs you",
        why: "Shows a notification when a session waits for you, finishes or hits a limit, never while AI Juggler is in front.",
        if_declined: "Everything still shows in the bell and Needs you inside AI Juggler.",
        background: "Waits: no notification is shown (so the system never asks) until you've said go.",
        triggers: &[
            "crates/juggler-app/src/os_prompts/notify.rs:connect",
            "crates/juggler-app/src/features/notifications/prefs.rs:notifications_set",
            "crates/juggler-app/src/features/notifications/mod.rs:send_test",
        ],
        never_shown: false,
        needed: super::needs::always,
    },
    Entry {
        prompt: LOGIN_ITEM,
        title: "Open AI Juggler at login",
        why: "Keeps your sessions watched from the moment you log in. The system shows a short notice that an item was added.",
        if_declined: "Nothing to decline: turn it off here or in the system's Login Items any time.",
        background: "Never: only your click turns it on.",
        triggers: &["crates/juggler-app/src/features/app/login_item.rs:app_login_item_set"],
        never_shown: false,
        needed: super::needs::login_item,
    },
    Entry {
        prompt: SCREEN_RECORDING,
        title: "See a live preview of each session",
        why: "Shows a session's window inside AI Juggler, on this computer only; nothing is recorded or sent.",
        if_declined: "Sessions show without a live picture of their window.",
        background: "Waits: the preview reads nothing until it's allowed.",
        triggers: &[
            "crates/juggler-app/src/os_access.rs:os_access_request_impl",
            "crates/juggler-app/src/preview.rs:capture",
        ],
        never_shown: false,
        needed: super::needs::preview,
    },
    Entry {
        prompt: LOCAL_NETWORK,
        title: "Find your other computers on this network",
        why: "Link finds and pairs your other computers running AI Juggler on the same network. Nothing leaves your network.",
        if_declined: "Link pairs by address only; computers on this network aren't listed.",
        background: "Waits: Link resumes at launch only once you've said go.",
        triggers: &[
            "crates/juggler-app/src/link.rs:link_set_enabled_impl",
            "crates/juggler-app/src/link.rs:resume_if_enabled",
        ],
        never_shown: false,
        needed: super::needs::link,
    },
    Entry {
        prompt: DOCUMENTS,
        title: "Read your projects in Documents",
        why: "Reads a project's git status (branch, changes) for sessions in your Documents folder. Nothing is changed or sent.",
        if_declined: "Those projects show without their git status.",
        background: "Waits: nothing in Documents is read until you've said go.",
        triggers: &[
            "crates/juggler-app/src/worktree.rs:git",
            "crates/juggler-app/src/worktree.rs:git_bytes",
        ],
        never_shown: false,
        needed: super::needs::documents,
    },
    Entry {
        prompt: DESKTOP,
        title: "Read your projects on the Desktop",
        why: "Reads a project's git status (branch, changes) for sessions on your Desktop. Nothing is changed or sent.",
        if_declined: "Those projects show without their git status.",
        background: "Waits: nothing on the Desktop is read until you've said go.",
        triggers: &[
            "crates/juggler-app/src/worktree.rs:git",
            "crates/juggler-app/src/worktree.rs:git_bytes",
        ],
        never_shown: false,
        needed: super::needs::desktop,
    },
    Entry {
        prompt: DOWNLOADS,
        title: "Read your projects in Downloads",
        why: "Reads a project's git status (branch, changes) for sessions in your Downloads folder. Nothing is changed or sent.",
        if_declined: "Those projects show without their git status.",
        background: "Waits: nothing in Downloads is read until you've said go.",
        triggers: &[
            "crates/juggler-app/src/worktree.rs:git",
            "crates/juggler-app/src/worktree.rs:git_bytes",
        ],
        never_shown: false,
        needed: super::needs::downloads,
    },
    Entry {
        prompt: APP_DATA,
        title: "Read a tool's data kept in its own app's folder",
        why: "Some apps keep their sessions in a folder the system guards for them. No tool AI Juggler supports does today.",
        if_declined: "That tool's sessions aren't read.",
        background: "Waits: such a folder is read only once you've said go.",
        triggers: &["crates/juggler-platform/src/os_prompt/mod.rs:may_read"],
        never_shown: false,
        needed: super::needs::app_data,
    },
    Entry {
        prompt: HELPER,
        title: "Start Bot's AI helper",
        why: "Bot's answers are written by AI on this computer, through a small helper inside AI Juggler. It starts the first time you check or choose Apple's AI (or a downloaded one) in Bot's setup guide.",
        if_declined: "Bot stays on its setup guide (Set up Bot) until its AI can start; nothing else changes.",
        background: "Waits: no helper starts until you've checked or chosen Bot's AI once.",
        triggers: &[
            "crates/juggler-app/src/features/ai/engines/helper.rs:launch_checked",
            "crates/juggler-app/src/os_prompts/mod.rs:prime_call",
        ],
        never_shown: false,
        needed: super::needs::helper,
    },
    Entry {
        prompt: KEYCHAIN,
        title: "Keep your secrets in the keychain",
        why: "AI Juggler's own secrets (the Figurehead device token among them) and the tools' sign-in checks use the keychain with its dialogs turned off: a call that would ask (an item saved by an earlier build with another signature) is left out, never shown.",
        if_declined: "Nothing is asked. A secret the keychain would only give after asking reads as missing: Figurehead asks you to link again. If the dialog ever appears, it's a bug; Always Allow keeps it from coming back.",
        background: "Never asks.",
        triggers: &[
            "crates/juggler-account/src/secrets.rs:quietly",
            "crates/juggler-account/src/secrets.rs:get",
            "crates/juggler-account/src/secrets.rs:set",
            "crates/juggler-account/src/secrets.rs:delete",
            "crates/juggler-platform/src/keychain.rs:item_exists",
        ],
        never_shown: true,
        needed: super::needs::never,
    },
    Entry {
        prompt: FIRST_OPEN,
        title: "Start an app you downloaded",
        why: "AI Juggler never opens an app downloaded from the internet that you haven't opened yourself, so the system's check never comes from us.",
        if_declined: "Open the app yourself once; AI Juggler can start it after that.",
        background: "Never asks: the start is refused and says why.",
        triggers: &["crates/juggler-platform/src/apps.rs:open_app"],
        never_shown: true,
        needed: super::needs::never,
    },
];

/// The entry with `id`.
#[must_use]
pub fn entry(id: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|entry| entry.prompt.id == id)
}
