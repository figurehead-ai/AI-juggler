//! AJ-E1610: every AI desktop app of the shared table, on fixtures only.
//! Each is found running by its bundle, or by the bundle id its own
//! `Info.plist` declares when the folder was renamed (a beta build), and
//! Kimi Code by its bundle's name with its own state folder as the
//! install sign; seen running, it is offered and never added (D-84).
//! Added, it opens (its own bundle, through Launch Services: never a tool
//! run beside it), keeps alive and comes forward like every App, and
//! "Open here" sends only a documented link, to its own process. Nothing
//! writes an app's data folder. No real app is ever started: the starter
//! records, the desktop is a fake, and the processes are made up.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::live::HostApp;

use super::*;
use crate::features::conversations::places::{HostRegistry, running_apps};
use crate::features::conversations::procs::{Proc, Table};
use crate::features::places::fakes::{FakeDesktop, Recorder};

/// A made-up app bundle `name` in `root`, declaring `bundle_id`, with its
/// main program; returns that program.
fn bundle(root: &Path, name: &str, bundle_id: Option<&str>) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(dir.join("Contents/MacOS")).unwrap();
    if let Some(id) = bundle_id {
        std::fs::write(
            dir.join("Contents/Info.plist"),
            format!(
                "<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict>\n<key>CFBundleIdentifier</key>\n<string>{id}</string>\n</dict></plist>\n"
            ),
        )
        .unwrap();
    }
    let exe = dir.join("Contents/MacOS/Main");
    std::fs::write(&exe, "").unwrap();
    exe
}

/// The table's apps that run one instance (every one but the Claude app,
/// whose profiles the D-84 tests cover), each as a bundle on disk: a
/// renamed folder where it has a bundle id (found by the id alone), its
/// own name where it has none (Kimi Code).
fn fixtures(root: &Path) -> Vec<(HostApp, PathBuf)> {
    juggler_adapters::desktop_apps::all()
        .into_iter()
        .filter(|app| app.instance_arg.is_none())
        .map(|app| {
            let exe = match &app.bundle_id {
                Some(id) => bundle(root, &format!("{} Beta.app", app.id), Some(id)),
                None => bundle(root, &app.bundle, None),
            };
            (app, exe)
        })
        .collect()
}

/// Each fixture's main process, with a helper inside its bundle that
/// carries no weight of its own.
fn table(fixtures: &[(HostApp, PathBuf)]) -> Table {
    let mut table = Table::new();
    for (at, (_, exe)) in fixtures.iter().enumerate() {
        let pid = 5000 + u32::try_from(at).unwrap() * 10;
        table.insert(
            pid,
            Proc {
                pid,
                ppid: Some(1),
                exe: Some(exe.clone()),
                args: vec![exe.to_string_lossy().into_owned()],
                start: Some(i64::from(pid)),
            },
        );
        let helper = exe
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Frameworks/Helper.app/Contents/MacOS/Helper");
        table.insert(
            pid + 1,
            Proc {
                pid: pid + 1,
                ppid: Some(pid),
                exe: Some(helper),
                args: Vec::new(),
                start: Some(i64::from(pid) + 1),
            },
        );
    }
    table
}

/// The registry, and every fixture app (made in `root`) as the scan
/// finds it running.
fn seen(
    root: &Path,
) -> (
    HostRegistry,
    Vec<crate::features::conversations::places::RunningApp>,
) {
    let hosts = HostRegistry::new(Vec::new());
    let running = running_apps(&table(&fixtures(root)), &hosts, root);
    (hosts, running)
}

/// A state whose tools look only in a made-up home under `root` (Add
/// links an app's folder through its adapter: never the real home's).
fn state(root: &Path) -> Arc<AppState> {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(
        juggler_adapters::OpenCodeAdapter::new().with_home(&home),
    ));
    registry.register(Arc::new(
        juggler_adapters::KimiCodeAdapter::new().with_home(&home),
    ));
    registry.register(Arc::new(
        juggler_adapters::HermesAdapter::new().with_home(&home),
    ));
    crate::features::places::fakes::on_made_up_mac(
        AppState::init_with_registry(root.join("data"), registry).unwrap(),
        root,
    )
}

/// Every file under `dir`, with its bytes.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(at) = todo.pop() {
        for entry in std::fs::read_dir(&at).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                todo.push(path);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap_or_default()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn each_desktop_app_is_found_by_its_bundle_or_its_bundle_id() {
    let root = tempfile::tempdir().unwrap();
    let fixtures = fixtures(root.path());
    assert_eq!(fixtures.len(), 8, "every app but the Claude app");
    let hosts = HostRegistry::new(Vec::new());
    let found = running_apps(&table(&fixtures), &hosts, root.path());
    let mut ids: Vec<&str> = found.iter().map(|app| app.app_id.as_str()).collect();
    ids.sort_unstable();
    let mut want: Vec<&str> = fixtures.iter().map(|(app, _)| app.id.as_str()).collect();
    want.sort_unstable();
    assert_eq!(ids, want, "each once, by its main process alone");
    for app in &found {
        assert_eq!(app.instance, None, "{}: one instance", app.app_id);
        assert_eq!(app.given, None, "{}: no folder given", app.app_id);
    }
    // A bundle that declares no id the table knows is no App.
    let other = bundle(root.path(), "Other.app", Some("com.example.other"));
    let mut lone = Table::new();
    lone.insert(
        9,
        Proc {
            pid: 9,
            ppid: Some(1),
            exe: Some(other),
            args: Vec::new(),
            start: Some(9),
        },
    );
    assert!(running_apps(&lone, &hosts, root.path()).is_empty());
}

#[test]
fn kimi_code_is_known_installed_by_its_own_folder_alone() {
    let home = tempfile::tempdir().unwrap();
    let kimi = juggler_adapters::desktop_apps::all()
        .into_iter()
        .find(|app| app.bundle_id.is_none())
        .unwrap();
    assert!(!juggler_adapters::desktop_apps::installed_by_folder(
        &kimi,
        home.path()
    ));
    std::fs::create_dir_all(
        home.path()
            .join("Library/Application Support/kimi-code-app"),
    )
    .unwrap();
    assert!(juggler_adapters::desktop_apps::installed_by_folder(
        &kimi,
        home.path()
    ));
}

#[test]
fn each_desktop_app_seen_running_is_offered_never_added_and_once_added_opens_and_keeps_alive() {
    let root = tempfile::tempdir().unwrap();
    let (hosts, running) = seen(root.path());
    let state = state(root.path());
    let bundles_before = snapshot(root.path());
    observe(&state, &running, &[]);

    let offered: Vec<String> = offers(&state)
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let apps: Vec<String> = crate::features::apps::list_impl(&state)
        .unwrap()
        .into_iter()
        .map(|app| app.id)
        .collect();
    for app in &running {
        let id = book::place_id(&app.app_id, None);
        assert!(offered.contains(&id), "{id}: offered");
        assert!(!apps.contains(&id), "{id}: never added by itself");
        assert!(
            set_keep_alive(&state, &id, true).is_err(),
            "{id}: an offer isn't kept alive"
        );
    }

    for app in &running {
        let id = book::place_id(&app.app_id, None);
        answer_offer(&state, &id, true).unwrap();
        // Its tool is the one the table names, adapter or not.
        let stored = crate::features::apps::list_impl(&state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id);
        assert!(stored.is_some(), "{id}: an App once added");
        let host = hosts.by_id(&app.app_id).unwrap();
        assert_eq!(
            stored.map(|row| row.tool).as_deref(),
            host.desktop.tool.as_deref(),
            "{id}"
        );
        set_keep_alive(&state, &id, true).unwrap();
        // Open while it isn't running: its own bundle, started as itself.
        if let Ok(mut watch) = state.places.watch.lock() {
            watch.forget_pid(&id);
        }
        let starter = Arc::new(Recorder::default());
        let said = open(
            &state,
            &id,
            None,
            Arc::new(FakeDesktop::default()),
            starter.clone(),
            None,
        )
        .unwrap();
        assert_ne!(said.reached, "none", "{id}: {}", said.outcome);
        let starts = starter.starts.lock().unwrap().clone();
        assert_eq!(starts.len(), 1, "{id}: one start");
        let (bundle, how, args) = &starts[0];
        assert_eq!(bundle, &app.bundle, "{id}: its own bundle");
        assert!(
            args.is_empty(),
            "{id}: no profile switch it doesn't document: {args:?}"
        );
        assert!(!how.new_instance, "{id}: one instance");
        assert!(
            starter.envs.lock().unwrap().iter().all(Vec::is_empty),
            "{id}: no variable passed"
        );
        // Nothing but the app: never a tool run beside it (agy -p, agy acp).
        let all = format!("{args:?} {:?}", starter.envs.lock().unwrap());
        for word in ["agy", "acp", " -p", "kiro-cli", "hermes", "opencode serve"] {
            assert!(!all.contains(word), "{id}: {word} in {all}");
        }
        // Keep alive starts the same bundle again, behind.
        let kept = super::start::plan(
            &with_book(&state, |book| book.places.get(&id).cloned())
                .unwrap()
                .unwrap(),
            &hosts,
            None,
            true,
        )
        .unwrap();
        assert_eq!(&kept.bundle, &app.bundle);
        assert!(kept.how.background && kept.args.is_empty());
    }
    // Read-only: no bundle, plist or app folder was written.
    let bundles_after: Vec<(PathBuf, Vec<u8>)> = snapshot(root.path())
        .into_iter()
        .filter(|(path, _)| !path.starts_with(root.path().join("data")))
        .collect();
    let bundles_before: Vec<(PathBuf, Vec<u8>)> = bundles_before
        .into_iter()
        .filter(|(path, _)| !path.starts_with(root.path().join("data")))
        .collect();
    assert_eq!(
        bundles_after, bundles_before,
        "no app's bundle or folder written"
    );
}

#[test]
fn open_here_sends_only_a_documented_link_to_the_apps_own_process() {
    let root = tempfile::tempdir().unwrap();
    let (_, running) = seen(root.path());
    let state = state(root.path());
    observe(&state, &running, &[]);
    let kimi = running
        .iter()
        .find(|app| app.app_id == "kimi-code-app")
        .unwrap();
    let cursor = running.iter().find(|app| app.app_id == "cursor").unwrap();
    let (kimi_id, cursor_id) = (
        book::place_id(&kimi.app_id, None),
        book::place_id(&cursor.app_id, None),
    );
    let desktop = FakeDesktop {
        sends_links: true,
        ..FakeDesktop::default()
    };
    // An offer isn't the user's: nothing is sent.
    assert!(reach::open_route(&state, &kimi_id, "new_in_folder", Some("/w/p"), &desktop).is_err());
    answer_offer(&state, &kimi_id, true).unwrap();
    answer_offer(&state, &cursor_id, true).unwrap();
    // Running as this test's process (never signalled).
    let me = std::process::id();
    let start = crate::features::conversations::procs::some(&[me])
        .get(&me)
        .and_then(|process| process.start);
    for id in [&kimi_id, &cursor_id] {
        let mut watch = state.places.watch.lock().unwrap();
        watch.found(id, me, start);
        watch.note_exe(id, &std::env::current_exe().unwrap());
    }

    let said = reach::open_route(
        &state,
        &kimi_id,
        "new_in_folder",
        Some("/w/my app"),
        &desktop,
    )
    .unwrap();
    assert_eq!(said.reached, "thread", "{}", said.outcome);
    assert_eq!(
        desktop.sent.lock().unwrap().clone(),
        [(me, "kimi-code://open?root=/w/my%20app&new=1".to_owned())]
    );
    // Kimi Code has no new-chat link; a relative folder never goes.
    assert!(reach::open_route(&state, &kimi_id, "new_chat", None, &desktop).is_err());
    assert!(reach::open_route(&state, &kimi_id, "new_in_folder", Some("w/p"), &desktop).is_err());
    // Cursor has none at all; an unproved or session action is never sent.
    let none = reach::open_route(&state, &cursor_id, "new_chat", None, &desktop).unwrap_err();
    assert!(none.to_string().contains("no link"), "{none}");
    for action in ["continue_session", "needs_input", "open_session", "reply"] {
        assert!(reach::open_route(&state, &kimi_id, action, Some("/w"), &desktop).is_err());
    }
    assert_eq!(
        desktop.sent.lock().unwrap().len(),
        1,
        "only the one documented link"
    );
    // The App's page reads what it allows and why not.
    let rows = commands::known(&state).unwrap();
    let row = rows.iter().find(|row| row.id == kimi_id).unwrap();
    let reach = row.reach.as_ref().unwrap();
    assert_eq!(reach.open_here, ["new_in_folder"]);
    assert!(reach.track_only.iter().any(|track| track.part == "reply"));
    let row = rows.iter().find(|row| row.id == cursor_id).unwrap();
    assert_eq!(row.reach.as_ref().unwrap().track_only.len(), 4);
}

#[test]
fn the_claude_apps_unproved_links_fall_back_to_plain_focus() {
    let claude = juggler_adapters::claude_live::host_apps().remove(0);
    let hosts = HostRegistry::new(vec![claude.clone()]);
    let host = hosts.by_id(&claude.id).unwrap();
    assert_eq!(
        host.route_link(
            juggler_adapter::desktop_app::RouteAction::ContinueSession,
            Some("local_1"),
            None
        ),
        None
    );
    // With its session route marked unproved, Go to it's link isn't sent:
    // the app is brought forward and the user picks the session.
    let mut unproved = claude;
    for route in &mut unproved.desktop.routes {
        if route.action == juggler_adapter::desktop_app::RouteAction::OpenSession {
            route.status = juggler_adapter::desktop_app::RouteStatus::Unverified;
        }
    }
    assert_eq!(unproved.session_link_for("local_1"), None);
}
