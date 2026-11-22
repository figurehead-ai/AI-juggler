//! AJ-E1610 (D-66, D-84, D-105, D-106): the AI desktop apps, as data.
//!
//! One table, `desktop_apps.json`, describes each tool's own desktop app:
//! how it is found (its bundle, its bundle id, and for an app with no
//! published id the folder it keeps its own state in), what links into it
//! are sound, what is track-only and why, and whether its tool may be
//! driven. Each adapter's [`juggler_adapter::ToolDiscovery::host_apps`]
//! returns its own rows ([`for_tool`]); the rows of a tool whose adapter
//! isn't built yet (Cursor, Kiro, Antigravity) are still Apps through the
//! host registry, which takes every row ([`all`]).
//!
//! The facts come from the research notes (`docs/research/desktop-apps.md`
//! and the desktop sections of `cursor.md`, `kiro.md`, `antigravity.md`).
//! A row whose `desktop.seen` is `source` was never read from an installed
//! bundle: its bundle id is from the vendor's docs or source (Cursor's,
//! Kiro's and Antigravity's from their Homebrew casks; OpenCode's from its
//! build config; Hermes' from its docs) and is to be confirmed on a real
//! install (AJ-E1612). The Claude app's and the Codex app's rows were
//! read from installed bundles. Kimi Code publishes no bundle id: its app is known
//! by its own state folder, `kimi-code-app`.
//!
//! Nothing here reads inside an app's data folder or its sign-in: the
//! install folder is only looked at, and sessions are read by each tool's
//! own adapter, from the files it already reads.

use std::path::Path;

use juggler_adapter::live::HostApp;

/// The table (data, not branches; ADR-0019).
const TABLE: &str = include_str!("desktop_apps.json");

/// Every desktop app in the table, read once a run (the host registry is
/// built on every scan). A table that doesn't read is a build bug the
/// tests catch; here it reads as none, never a crash.
#[must_use]
pub fn all() -> Vec<HostApp> {
    static ROWS: std::sync::OnceLock<Vec<HostApp>> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| serde_json::from_str(TABLE).unwrap_or_default())
        .clone()
}

/// The desktop apps of tool `tool` (its adapter's id).
#[must_use]
pub fn for_tool(tool: &str) -> Vec<HostApp> {
    all()
        .into_iter()
        .filter(|app| app.desktop.tool.as_deref() == Some(tool))
        .collect()
}

/// Whether `app` is installed for the user whose home is `home`, by the
/// folder it keeps its own state in (an app with no published bundle id):
/// the folder is looked at, never read or written. `false` for an app
/// that names none.
#[must_use]
pub fn installed_by_folder(app: &HostApp, home: &Path) -> bool {
    app.desktop
        .install_folder
        .as_deref()
        .is_some_and(|folder| home.join(folder).is_dir())
}

#[cfg(test)]
mod tests {
    use juggler_adapter::desktop_app::{RouteAction, TrackPart};
    use juggler_adapter::live::HostKind;

    use super::*;

    #[test]
    fn the_table_reads_and_every_id_is_once() {
        let apps = all();
        assert_eq!(apps.len(), 9, "{apps:?}");
        let mut ids: Vec<&str> = apps.iter().map(|app| app.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), apps.len(), "an id twice");
        assert!(apps.iter().all(HostApp::is_app), "each is an App");
        assert!(
            apps.iter().all(|app| app.desktop.tool.is_some()),
            "each names its tool"
        );
    }

    #[test]
    fn each_app_is_found_by_its_bundle_id_or_its_own_folder() {
        for app in all() {
            assert!(
                app.bundle_id.is_some() || app.desktop.install_folder.is_some(),
                "{}: no way to find it",
                app.id
            );
            assert!(app.bundle.ends_with(".app"), "{}", app.id);
        }
        let ids: Vec<(String, Option<String>)> = all()
            .into_iter()
            .map(|app| (app.id, app.bundle_id))
            .collect();
        for (id, bundle_id) in [
            ("claude-app", Some("com.anthropic.claudefordesktop")),
            ("codex-app", Some("com.openai.codex")),
            ("cursor", Some("com.todesktop.230313mzl4w4u92")),
            ("kiro-ide", Some("dev.kiro.desktop")),
            ("antigravity-app", Some("com.google.antigravity")),
            ("antigravity-ide", Some("com.google.antigravity-ide")),
            ("opencode-app", Some("ai.opencode.desktop")),
            ("kimi-code-app", None),
            ("hermes-app", Some("com.nousresearch.hermes")),
        ] {
            assert!(
                ids.contains(&(id.to_owned(), bundle_id.map(str::to_owned))),
                "{id}"
            );
        }
        let opencode = all()
            .into_iter()
            .find(|app| app.id == "opencode-app")
            .unwrap();
        assert!(opencode.has_bundle_id("ai.opencode.desktop.beta"));
        assert!(!opencode.has_bundle_id("ai.opencode"));
    }

    #[test]
    fn only_the_claude_app_opens_per_account_and_the_rest_say_why() {
        for app in all() {
            if app.id == "claude-app" {
                assert_eq!(app.instance_arg.as_deref(), Some("--user-data-dir"));
                assert_eq!(app.desktop.why_not(TrackPart::Account), None);
            } else {
                assert_eq!(app.instance_arg, None, "{}", app.id);
                assert!(
                    app.desktop.why_not(TrackPart::Account).is_some(),
                    "{}",
                    app.id
                );
            }
        }
    }

    #[test]
    fn the_claude_apps_undocumented_links_are_declared_but_never_sent() {
        let claude = for_tool("claude-code").remove(0);
        for action in [RouteAction::ContinueSession, RouteAction::NeedsInput] {
            assert!(claude.desktop.route(action).is_some(), "{action:?}");
            assert_eq!(
                claude.route_link(action, Some("local_abc"), None),
                None,
                "{action:?}"
            );
        }
        assert_eq!(
            claude
                .route_link(RouteAction::NewChat, None, None)
                .as_deref(),
            Some("claude://claude.ai/new")
        );
        assert_eq!(
            claude
                .route_link(RouteAction::OpenChat, Some("1a2b-3c"), None)
                .as_deref(),
            Some("claude://claude.ai/chat/1a2b-3c")
        );
        assert_eq!(
            claude
                .route_link(
                    RouteAction::NewInFolder,
                    None,
                    Some(Path::new("/Users/you/my app"))
                )
                .as_deref(),
            Some("claude://code/new?folder=/Users/you/my%20app")
        );
        assert_eq!(
            claude.route_link(RouteAction::NewInFolder, None, Some(Path::new("rel"))),
            None,
            "a folder is always absolute"
        );
        assert_eq!(
            claude.route_link(RouteAction::OpenChat, Some("a/../b"), None),
            None
        );
        // Its session link was proved live (AJ-E2102): still sent.
        assert_eq!(
            claude.session_link_for("local_abc").as_deref(),
            Some("claude://claude.ai/epitaxy/local_abc")
        );
    }

    #[test]
    fn replies_go_only_through_an_adapter_and_never_install_anything() {
        for app in all() {
            for route in &app.desktop.routes {
                if matches!(route.action, RouteAction::Reply | RouteAction::ReadSessions) {
                    assert_eq!(route.link, None, "{}: through its adapter only", app.id);
                }
                let link = route.link.as_deref().unwrap_or_default();
                assert!(
                    !link.contains("install") && !link.contains("plugin"),
                    "{}: {link}",
                    app.id
                );
                assert!(!route.source.is_empty(), "{}: unsourced", app.id);
            }
            let reply = app.desktop.usable(RouteAction::Reply).is_some();
            assert_eq!(
                reply,
                app.id == "hermes-app",
                "{}: only Hermes' gateway is a sound reply route",
                app.id
            );
            assert_eq!(
                reply,
                app.desktop.why_not(TrackPart::Reply).is_none(),
                "{}: a reply route or a reason, never both or neither",
                app.id
            );
        }
    }

    #[test]
    fn every_track_only_reason_is_one_plain_line() {
        for app in all() {
            for track in &app.desktop.track_only {
                let why = &track.why;
                assert!(why.ends_with('.') && !why.contains('\n'), "{why}");
                assert!(why.len() <= 140, "{}: {why}", app.id);
                for word in ["adapter", "track-only", "route", "AJ-E", "D-1"] {
                    assert!(!why.contains(word), "{}: {why} says {word}", app.id);
                }
            }
        }
    }

    #[test]
    fn antigravity_is_never_driven_on_a_consumer_plan() {
        let apps = for_tool("antigravity");
        assert_eq!(apps.len(), 2);
        for app in &apps {
            assert!(!app.desktop.may_drive(true), "{}", app.id);
            assert!(!app.desktop.may_switch_automatically(true), "{}", app.id);
            assert!(app.desktop.may_drive(false), "{}", app.id);
        }
        // No other tool's app is held back by it.
        for app in all()
            .iter()
            .filter(|app| app.desktop.tool.as_deref() != Some("antigravity"))
        {
            assert!(app.desktop.may_drive(true), "{}", app.id);
        }
    }

    #[test]
    fn an_editor_app_keeps_its_folder_link_and_a_desktop_app_its_kind() {
        let cursor = all().into_iter().find(|app| app.id == "cursor").unwrap();
        assert_eq!(cursor.kind, HostKind::Editor);
        assert!(cursor.desktop.editor_app && cursor.is_app());
        assert_eq!(
            cursor.folder_link.as_deref(),
            Some("cursor://file/{folder}")
        );
    }

    #[test]
    fn kimi_code_is_found_by_its_own_folder_which_is_never_written() {
        let kimi = for_tool("kimi-code").remove(0);
        let home = tempfile::tempdir().unwrap();
        assert!(!installed_by_folder(&kimi, home.path()));
        let folder = home
            .path()
            .join("Library/Application Support/kimi-code-app");
        std::fs::create_dir_all(&folder).unwrap();
        let before = std::fs::read_dir(&folder).unwrap().count();
        assert!(installed_by_folder(&kimi, home.path()));
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), before);
        let claude = for_tool("claude-code").remove(0);
        assert!(
            !installed_by_folder(&claude, home.path()),
            "no folder named"
        );
    }
}
