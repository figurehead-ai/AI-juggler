//! `ZCode` adapter tests (AJ-E1605 Req.1, 2, 3, 7): discovery by its app
//! bundle and its engine, an own tree per App without moving `HOME`, the
//! launch check, launches, the sign-in probe by presence, sessions from
//! the tree's store (read-only, byte for byte), running engines told from
//! helpers, and usage from the App's own store. Every tree and store is
//! synthetic (`zcode_fixture`); no `ZCode` was run and no real `~/.zcode`
//! is opened.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test fails loudly on a broken fixture"
)]

use juggler_adapter::ToolDiscovery;
use juggler_adapter::ToolIdentity;
use juggler_adapter::ToolInventory;
use juggler_adapter::ToolLaunch;
use juggler_adapter::ToolLive;
use juggler_adapter::ToolQuota;
use juggler_adapter::ToolSignIn;
use juggler_adapter::ToolTranscripts;
mod zcode_fixture;

use std::path::{Path, PathBuf};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::runtime::{DrivenLaunch, DrivenRuntime, ImportedHistory, Runtime, Transport};
use juggler_adapter::{AdapterError, Capabilities, InstallStatus, LaunchMode, ToolRuntime};
use juggler_adapters::zcode::{isolation_took, parse_version, session_process};
use juggler_adapters::{LoginState, ZCODE_ID, ZCodeAdapter};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
use zcode_fixture::{Row, write};

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("zcode-acct"),
        harness: HarnessId::new(ZCODE_ID),
        label: "ZCode".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

fn profile(env: &[(&str, &str)]) -> Profile {
    Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: AccountId::new("zcode-acct"),
        settings_overlay: SettingsOverlay {
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            ..SettingsOverlay::default()
        },
        default_cwd: None,
    }
}

/// A stand-in for the app's own runtime (`ZCode.app/Contents/MacOS/ZCode`)
/// that prints `version` only when run as Node.js and asked the engine's
/// `version` (its second word), as `ELECTRON_RUN_AS_NODE=1 ZCode <engine>
/// version` does.
fn runtime(path: &Path, version: &str) -> PathBuf {
    let path = path.to_path_buf();
    write(
        &path,
        &format!(
            "#!/bin/sh\nif [ \"$ELECTRON_RUN_AS_NODE\" = 1 ] && [ \"$2\" = version ]; then echo {version:?}; else exit 3; fi\n"
        ),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// Every file under `dir` with its bytes.
fn files(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A sandbox: a home, an Applications folder with `ZCode.app` and its
/// runtime stand-in.
struct World {
    _root: tempfile::TempDir,
    base: PathBuf,
    home: PathBuf,
    engine: PathBuf,
    runtime: PathBuf,
}

impl World {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let home = base.join("home");
        let apps = base.join("Applications");
        std::fs::create_dir_all(&home).unwrap();
        let engine = zcode_fixture::app(&apps, "dev.zcode.app");
        let runtime = runtime(&apps.join("ZCode.app/Contents/MacOS/ZCode"), "0.16.9");
        Self {
            _root: root,
            base,
            home,
            engine,
            runtime,
        }
    }

    /// The adapter on the app's own runtime, found in its bundle.
    fn adapter(&self) -> ZCodeAdapter {
        ZCodeAdapter::with_engine(&self.engine).with_home(&self.home)
    }

    fn builtin(&self) -> PathBuf {
        self.base
            .join("Applications/ZCode.app/Contents/Resources/config/provider/zcode-builtin.json")
    }
}

/// The app is found where macOS installs apps (`juggler_adapter::apps`;
/// none yet elsewhere).
#[cfg(target_os = "macos")]
#[test]
fn zcode_is_found_by_its_app_bundle_and_versioned_by_its_engine() {
    assert_eq!(parse_version("0.16.9\n").as_deref(), Some("0.16.9"));
    assert_eq!(parse_version("zcode 0.16.9").as_deref(), Some("0.16.9"));
    assert_eq!(parse_version("v0.17.0").as_deref(), Some("0.17.0"));
    assert_eq!(parse_version("zcode\n"), None);

    let world = World::new();
    let adapter = ZCodeAdapter::new()
        .with_app_dirs(vec![
            world.base.join("nowhere"),
            world.base.join("Applications"),
        ])
        .with_home(&world.home);
    assert_eq!(adapter.locate().as_deref(), Some(world.engine.as_path()));
    let found = adapter.discover();
    assert_eq!(found.binary.as_deref(), Some(world.engine.as_path()));
    assert_eq!(
        found.install,
        InstallStatus::Installed {
            version: "0.16.9".into()
        },
        "ELECTRON_RUN_AS_NODE=1 <app runtime> <engine> version"
    );
    assert_eq!(
        found.verification,
        juggler_adapter::VerificationStatus::Unverified,
        "never claimed verified"
    );
    assert_eq!(
        adapter.resolved_binary().as_deref(),
        Some(world.engine.as_path())
    );

    // An app by that name without ZCode's engine in it is not ZCode.
    let other = world.base.join("Other");
    std::fs::create_dir_all(other.join("ZCode.app/Contents/Resources")).unwrap();
    let elsewhere = ZCodeAdapter::new()
        .with_app_dirs(vec![other.clone()])
        .with_home(&world.home);
    assert_eq!(elsewhere.locate(), None);
    assert_eq!(elsewhere.discover().install, InstallStatus::NotFound);

    // A runtime that can't run it, or none, reports the failure; no
    // Node.js of the user's is looked for.
    let broken = ZCodeAdapter::with_engine(&world.engine).with_runtime("/nonexistent/ZCode");
    assert!(matches!(
        broken.discover().install,
        InstallStatus::Failed { .. }
    ));
    std::fs::remove_file(&world.runtime).unwrap();
    assert!(matches!(
        world.adapter().discover().install,
        InstallStatus::Failed { message } if message.contains("no runtime")
    ));
}

#[test]
fn each_app_has_its_own_tree_and_zcodes_own_runs_with_the_variables_unset() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    let env = adapter.prepare_account(&a).unwrap();
    assert!(a.join(".ai-juggler-managed").is_file());
    assert!(a.join(".zcode").is_dir(), "the App's tree is made");
    assert!(
        !a.join(".zcode/cli").exists(),
        "nothing the engine writes is made for it (the launch check reads it)"
    );
    let tree = a.join(".zcode").to_string_lossy().into_owned();
    let store = a
        .join(".zcode/cli/db/db.sqlite")
        .to_string_lossy()
        .into_owned();
    let base = a.to_string_lossy().into_owned();
    assert_eq!(env.get("ZCODE_STORAGE_DIR"), Some(&tree));
    assert_eq!(env.get("ZCODE_DATA_BASE_DIR"), Some(&base));
    assert_eq!(
        env.get("ZCODE_SESSION_DB_PATH"),
        Some(&store),
        "the store's default follows HOME, not ZCODE_STORAGE_DIR: pinned too"
    );

    // ZCode's own ~/.zcode runs with every variable unset.
    let default = world.home.join(".zcode");
    assert_eq!(adapter.vendor_default_dir(), Some(default.clone()));
    assert!(
        adapter
            .pinned_env(&default)
            .iter()
            .all(|(_, value)| value.is_none())
    );

    // Pinned after hostile overrides; inherited movers removed.
    let spec = adapter
        .driven_spec(
            &account(&a),
            &profile(&[
                ("ZCODE_STORAGE_DIR", "/elsewhere"),
                ("ZCODE_DATA_BASE_DIR", "/elsewhere"),
                ("ZCODE_SESSION_DB_PATH", "/elsewhere/db.sqlite"),
                ("ZCODE_SESSION_DB", "/elsewhere/db.sqlite"),
                ("ZCODE_PERSONAL_PROVIDER_CONFIG_FILE", "/elsewhere/p.json"),
                ("KEEP", "kept"),
            ]),
            &world.base,
            &fresh(None),
        )
        .unwrap()
        .launch;
    assert_eq!(spec.env.get("ZCODE_STORAGE_DIR"), Some(&tree));
    assert_eq!(spec.env.get("ZCODE_DATA_BASE_DIR"), Some(&base));
    assert_eq!(spec.env.get("ZCODE_SESSION_DB_PATH"), Some(&store));
    for removed in ["ZCODE_SESSION_DB", "ZCODE_PERSONAL_PROVIDER_CONFIG_FILE"] {
        assert_eq!(
            spec.env.get(removed).map(String::as_str),
            Some(ENV_UNSET),
            "{removed}"
        );
    }
    assert_eq!(spec.env.get("KEEP").map(String::as_str), Some("kept"));
    assert!(
        !spec.env.contains_key("HOME"),
        "HOME is never moved (git identity, SSH keys)"
    );
    let default_spec = adapter
        .driven_spec(
            &account(&default),
            &profile(&[("ZCODE_STORAGE_DIR", "/elsewhere")]),
            &world.base,
            &fresh(None),
        )
        .unwrap()
        .launch;
    for key in [
        "ZCODE_STORAGE_DIR",
        "ZCODE_DATA_BASE_DIR",
        "ZCODE_SESSION_DB_PATH",
    ] {
        assert_eq!(
            default_spec.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
    }

    // No terminal UI: no copyable line to type.
    assert_eq!(
        juggler_adapter::adapter::shell_snippet(&adapter, &a, &std::collections::BTreeMap::new()),
        None
    );
}

/// A driven launch: new, resumed or importing `history`.
fn fresh(resume: Option<&str>) -> DrivenLaunch {
    DrivenLaunch {
        socket: PathBuf::from("/nowhere/kit.sock"),
        resume: resume.map(str::to_owned),
        fresh_id: "0b1c2d3e-0000-4000-8000-0000000000aa".into(),
        history: None,
    }
}

#[test]
fn the_launch_check_refuses_an_app_whose_tree_the_engine_did_not_write() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    let refused = isolation_took(&a).unwrap_err();
    assert!(refused.contains("isn't used"), "{refused}");
    // What the engine writes when it honours the variables.
    drop(zcode_fixture::create(&a));
    assert_eq!(isolation_took(&a), Ok(()));
}

#[test]
fn sessions_run_driven_on_the_apps_own_runtime_and_no_terminal_is_offered() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    let project = world.base.join("work");
    let engine = world.engine.to_string_lossy().into_owned();
    let builtin = world.builtin().to_string_lossy().into_owned();

    // Every terminal launch refuses, saying why: the app ships no TUI.
    for mode in [
        LaunchMode::Interactive,
        LaunchMode::Resume {
            session_id: "sess_0a1b".into(),
            delta_prompt: None,
        },
        LaunchMode::Primed {
            prompt: "hello".into(),
            read_dirs: Vec::new(),
        },
        LaunchMode::Headless {
            prompt: "hello".into(),
        },
    ] {
        match adapter.launch_spec(&account(&a), &profile(&[]), &project, &mode) {
            Err(AdapterError::Unsupported(why)) => assert!(why.contains("no terminal UI"), "{why}"),
            other => panic!("{mode:?}: {other:?}"),
        }
    }
    assert_eq!(adapter.resume_args("sess_0a1b"), None);
    assert!(!adapter.capabilities().contains(Capabilities::HEADLESS));
    assert!(
        adapter
            .capabilities()
            .contains(Capabilities::PROTOCOL_SERVER)
    );
    let driven = adapter.driven().expect("driven is its runtime");
    assert!(!driven.terminal_ui());
    assert!(driven.imports_history());
    assert_eq!(driven.default_runtime(), Runtime::Driven);
    assert_eq!(driven.unavailable(), None);

    // The server, as the desktop app starts it, on the app's runtime.
    let new = driven
        .driven_spec(&account(&a), &profile(&[]), &project, &fresh(None))
        .unwrap();
    assert_eq!(new.launch.program, world.runtime);
    assert_eq!(
        new.launch.args,
        [engine.clone(), "app-server".into(), "--stdio".into()]
    );
    assert!(!new.launch.needs_pty);
    assert_eq!(new.launch.cwd, project);
    assert_eq!(new.transport, Transport::Stdio);
    assert_eq!(new.native_id, None, "the engine names a new session");
    assert_eq!(
        new.launch
            .env
            .get("ELECTRON_RUN_AS_NODE")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        new.launch.env.get("ZCODE_BUILTIN_PROVIDER_CONFIG_FILE"),
        Some(&builtin)
    );
    assert!(!new.launch.env.contains_key("ZCODE_APP_VERSION"));
    // An override can't move the runtime's mode or the provider settings.
    let hostile = driven
        .driven_spec(
            &account(&a),
            &profile(&[
                ("ELECTRON_RUN_AS_NODE", "0"),
                ("ZCODE_BUILTIN_PROVIDER_CONFIG_FILE", "/elsewhere.json"),
            ]),
            &project,
            &fresh(None),
        )
        .unwrap();
    assert_eq!(
        hostile
            .launch
            .env
            .get("ELECTRON_RUN_AS_NODE")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        hostile.launch.env.get("ZCODE_BUILTIN_PROVIDER_CONFIG_FILE"),
        Some(&builtin)
    );

    // A resume names the session over the protocol, never as an option.
    let resumed = driven
        .driven_spec(
            &account(&a),
            &profile(&[]),
            &project,
            &fresh(Some("sess_0a1b")),
        )
        .unwrap();
    assert_eq!(resumed.native_id.as_deref(), Some("sess_0a1b"));
    assert_eq!(resumed.launch.args, new.launch.args);
    for id in ["", "-c", "--continue"] {
        assert!(matches!(
            driven.driven_spec(&account(&a), &profile(&[]), &project, &fresh(Some(id))),
            Err(AdapterError::InvalidLaunch(_))
        ));
    }
    // A pack's conversation comes in under an id of the engine's shape.
    let history = ImportedHistory {
        title: "Handover".into(),
        turns: vec![("user".into(), "the brief".into())],
    };
    let imported = driven
        .driven_spec(
            &account(&a),
            &profile(&[]),
            &project,
            &DrivenLaunch {
                history: Some(history.clone()),
                ..fresh(None)
            },
        )
        .unwrap();
    assert_eq!(
        imported.native_id.as_deref(),
        Some("sess_0b1c2d3e-0000-4000-8000-0000000000aa")
    );
    assert_eq!(imported.history, Some(history.clone()));
    // Not on a resume.
    let not = driven
        .driven_spec(
            &account(&a),
            &profile(&[]),
            &project,
            &DrivenLaunch {
                history: Some(history),
                ..fresh(Some("sess_0a1b"))
            },
        )
        .unwrap();
    assert_eq!(not.history, None);
    // No terminal view either.
    assert!(matches!(
        driven.terminal_view(
            &account(&a),
            &profile(&[]),
            &project,
            &juggler_adapter::runtime::ViewOf {
                native_id: "sess_0a1b".into(),
                socket: None,
            },
        ),
        Err(AdapterError::Unsupported(_))
    ));
    // Without the app's provider settings, it says so.
    std::fs::remove_file(world.builtin()).unwrap();
    assert!(
        driven
            .unavailable()
            .is_some_and(|why| why.contains("provider settings"))
    );
}

#[test]
fn the_sign_in_probe_sees_the_file_never_its_contents_and_login_runs_in_the_tree() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    assert_eq!(
        adapter.login_state(&a),
        LoginState::Unknown,
        "no folder yet"
    );
    adapter.prepare_account(&a).unwrap();
    assert_eq!(adapter.login_state(&a), LoginState::LoggedOut);
    zcode_fixture::sign_in(&a.join(".zcode"), "made-up-token");
    assert_eq!(adapter.login_state(&a), LoginState::SignedIn);
    // ZCode's own tree: the file in it.
    let default = world.home.join(".zcode");
    std::fs::create_dir_all(&default).unwrap();
    assert_eq!(adapter.login_state(&default), LoginState::LoggedOut);
    zcode_fixture::sign_in(&default, "made-up-token");
    assert_eq!(adapter.login_state(&default), LoginState::SignedIn);
    // An empty file is no sign-in.
    write(&a.join(".zcode/v2/credentials.json"), "");
    assert_eq!(adapter.login_state(&a), LoginState::LoggedOut);

    let login = adapter.login_spec(&a).unwrap();
    assert_eq!(login.program, world.runtime);
    assert_eq!(
        login.env.get("ELECTRON_RUN_AS_NODE").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        login.args,
        [world.engine.to_string_lossy().into_owned(), "login".into()]
    );
    assert_eq!(
        login.env.get("ZCODE_DATA_BASE_DIR").map(String::as_str),
        Some(a.to_str().unwrap())
    );
    assert!(login.needs_pty);
}

#[test]
fn sessions_come_from_the_store_read_only_and_byte_for_byte_unchanged() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    let project = world.base.join("work/shop");
    let other = world.base.join("work/other");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let now = zcode_fixture::now_ms();
    {
        let conn = zcode_fixture::create(&a);
        zcode_fixture::add(&conn, &Row::plain("sess_one", &project, now));
        zcode_fixture::add(
            &conn,
            &Row {
                parent: Some("sess_one"),
                ..Row::plain("sess_sub", &project, now)
            },
        );
        zcode_fixture::add(
            &conn,
            &Row {
                archived: Some(now),
                ..Row::plain("sess_old", &project, now)
            },
        );
        zcode_fixture::add(
            &conn,
            &Row {
                task_type: "automation",
                ..Row::plain("sess_auto", &project, now)
            },
        );
        zcode_fixture::add(
            &conn,
            &Row {
                title: "Other work",
                ..Row::plain("sess_two", &other, now - 60_000)
            },
        );
    }
    let store_dir = a.join(".zcode/cli/db");
    assert!(
        !store_dir.join("db.sqlite-wal").exists(),
        "the store is closed, as ZCode leaves it"
    );
    let before = files(&a);

    let listed = adapter.stored_conversations(&a);
    let ids: Vec<&str> = listed.iter().map(|c| c.summary.id.as_str()).collect();
    assert_eq!(
        ids,
        ["sess_one", "sess_two"],
        "interactive, not archived, not a sub-agent's; newest first"
    );
    let one = &listed[0];
    assert_eq!(
        one.summary.cwd.as_deref(),
        Some(project.as_path()),
        "its folder becomes a project"
    );
    assert_eq!(one.summary.title.as_deref(), Some("Fix the cart"));
    assert_eq!(one.summary.started_ms, Some(now));
    assert_eq!(one.summary.last_ms, Some(now + 1_000));
    assert_eq!(one.changed_ms, now + 1_000);
    assert_eq!(
        one.key,
        juggler_adapters::zcode_sessions::session_ref(&store_dir.join("db.sqlite"), "sess_one")
    );

    let found = adapter
        .locate_transcript(&a, &project, now - 1_000, &[])
        .expect("the session started in the folder");
    assert_eq!(found.native_id.as_deref(), Some("sess_one"));
    assert!(
        adapter
            .locate_transcript(&a, &project, now - 1_000, std::slice::from_ref(&found.path))
            .is_none(),
        "already linked"
    );
    assert!(
        adapter
            .locate_transcript(&a, &project, now + 60_000, &[])
            .is_none()
    );

    // The watcher sees them too.
    let mut watcher = adapter.watcher(&a).unwrap();
    let mut seen = Vec::new();
    while let Some(event) = watcher.poll(std::time::Duration::ZERO).unwrap() {
        if let juggler_adapter::SessionEvent::Discovered { native_id, .. } = event {
            seen.push(native_id);
        }
    }
    seen.sort();
    assert_eq!(seen, ["sess_one", "sess_two"]);

    assert_eq!(
        files(&a),
        before,
        "not a byte changed, nothing written beside it"
    );

    // An App with no store has no sessions, and says nothing wrong.
    let b = world.base.join("accounts/b");
    adapter.prepare_account(&b).unwrap();
    assert!(adapter.stored_conversations(&b).is_empty());
    assert!(adapter.locate_transcript(&b, &project, 0, &[]).is_none());
}

#[test]
fn a_running_engine_is_a_session_and_its_helpers_are_not() {
    let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    let engine = "/Applications/ZCode.app/Contents/Resources/glm/zcode.cjs";
    let node = Path::new("/opt/homebrew/Cellar/node/26.10.0_1/bin/node");
    let terminal = session_process(Some(node), &args(&["node", engine])).unwrap();
    assert_eq!(terminal.resumes, None);
    let resumed = session_process(
        Some(node),
        &args(&["node", engine, "--resume", "sess_0a1b"]),
    )
    .unwrap();
    assert_eq!(resumed.resumes.as_deref(), Some("sess_0a1b"));
    assert!(session_process(Some(node), &args(&["node", engine, "-c"])).is_some());
    assert!(session_process(Some(node), &args(&["node", engine, "tui"])).is_some());
    // A prompt that is a helper's name is still a prompt.
    assert!(session_process(Some(node), &args(&["node", engine, "-p", "doctor"])).is_some());
    for helper in [
        "app-server",
        "doctor",
        "version",
        "login",
        "logout",
        "plugins",
        "skills",
        "commands",
        "--version",
        "--help",
    ] {
        assert!(
            session_process(Some(node), &args(&["node", engine, helper])).is_none(),
            "{helper}"
        );
    }
    // The desktop app's own runtime running its server is a helper too.
    let helper_exe = Path::new(
        "/Applications/ZCode.app/Contents/Frameworks/ZCode Helper.app/Contents/MacOS/ZCode Helper",
    );
    assert!(
        session_process(
            Some(helper_exe),
            &args(&["ZCode Helper", engine, "app-server"])
        )
        .is_none()
    );
    // Run by its own `#!` line.
    assert!(session_process(None, &args(&[engine])).is_some());
    // Not ZCode: the desktop app itself, an editor opening the file.
    assert!(
        session_process(
            Some(Path::new("/Applications/ZCode.app/Contents/MacOS/ZCode")),
            &args(&["ZCode"])
        )
        .is_none()
    );
    assert!(
        session_process(
            Some(Path::new("/usr/bin/vim")),
            &args(&["vim", "zcode.cjs"])
        )
        .is_none()
    );
    let adapter = ZCodeAdapter::new();
    assert!(
        adapter
            .session_process(Some(node), &args(&["node", engine]))
            .is_some()
    );
}

#[test]
fn usage_is_read_from_the_apps_own_store() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    let mut record = account(&a);
    assert_eq!(
        adapter.usage_since(&record, 0).unwrap().unwrap(),
        Vec::new(),
        "no store, no usage"
    );
    let at = 1_792_580_400_000; // 2026-10-21
    {
        let conn = zcode_fixture::create(&a);
        zcode_fixture::usage(&conn, "mu_1", "GLM-5.3", "completed", at, 1000);
        zcode_fixture::usage(&conn, "mu_2", "GLM-5.3", "completed", at + 60_000, 400);
        zcode_fixture::usage(&conn, "mu_3", "GLM-5.3", "streaming", at + 90_000, 50);
    }
    record.id = AccountId::new("acct-z");
    let all = adapter.usage_since(&record, 0).unwrap().unwrap();
    assert_eq!(all.len(), 2, "completed rows only");
    assert!(all.iter().all(|event| event.account == "acct-z"));
    assert!(all.iter().all(|event| event.credits.is_some()));
    let later = adapter.usage_since(&record, at + 1).unwrap().unwrap();
    assert_eq!(later.len(), 1, "only rows started since");
    assert_eq!(later[0].tokens.input, 400);
    // Req.4: Computed by the published formula, per session as per account:
    // (fresh × 6.9 + cached × 1.7 + output × 24) / 10 000, half off-peak.
    let first = all.iter().find(|event| event.ts == at).unwrap();
    let raw = (970.0 * 6.9 + 30.0 * 1.7 + 120.0 * 24.0) / 10_000.0;
    let expected = if juggler_adapters::zcode_usage::zcode_is_peak(at) {
        raw
    } else {
        raw * 0.5
    };
    assert!((first.credits.unwrap() - expected).abs() < 1e-9);
    assert_eq!(first.confidence, juggler_core::model::Confidence::Computed);
    assert!(
        all.iter()
            .all(|event| event.session.as_deref() == Some("sess_use"))
    );
    // The conversations list carries each session's own tokens; one with
    // no usage shows none (never invented).
    {
        let conn = zcode_fixture::create(&a);
        zcode_fixture::add(&conn, &Row::plain("sess_use", Path::new("/w"), at));
        zcode_fixture::add(&conn, &Row::plain("sess_idle", Path::new("/w"), at));
    }
    let listed = adapter.stored_conversations(&a);
    let tokens = |id: &str| {
        listed
            .iter()
            .find(|c| c.summary.id == id)
            .unwrap()
            .summary
            .tokens
    };
    assert_eq!(tokens("sess_use").input, 1400, "two completed rows");
    assert_eq!(tokens("sess_use").output, 240);
    assert_eq!(tokens("sess_use").cache_read, 60);
    assert_eq!(
        tokens("sess_idle"),
        juggler_core::model::TokenCounts::default()
    );
}

/// Req.5 (ADR-0018): `~/.zcode` is used where it is and never written; any
/// other tree comes in as a copy into the App's own tree that takes its
/// settings and plugins, never the sign-in, the store, logs, model-call
/// records or each session's files, nor a settings file with a key in it;
/// what the engine reads from the home folder for every App isn't copied.
#[test]
fn a_tree_is_used_in_place_or_copied_without_its_sign_in_store_or_logs() {
    let world = World::new();
    let adapter = world.adapter();
    let own = world.home.join(".zcode");
    zcode_fixture::settings(&own);
    zcode_fixture::sign_in(&own, "made-up-token");
    zcode_fixture::add_session(&own, "sess_old", Path::new("/w"));
    let beta = world.home.join(".zcode-beta");
    zcode_fixture::settings(&beta);
    write(
        &beta.join("v2/provider_config.json"),
        r#"{"providers":{"custom":{"apiKey":"made-up-key-0123456789abcdef"}}}"#,
    );
    let before = (files(&own), files(&beta));

    // The scan finds both, the default first.
    let found = adapter.scan_environment();
    let paths: Vec<&Path> = found.iter().map(|install| install.path.as_path()).collect();
    assert_eq!(paths, [own.as_path(), beta.as_path()]);
    assert_eq!(
        found[0].login,
        juggler_adapter::environment::LoginProbe::LoggedIn
    );

    // Where it is: the default only.
    assert!(adapter.can_use_in_place());
    adapter.adopt_account(&own).unwrap();
    assert!(matches!(
        adapter.adopt_account(&beta),
        Err(AdapterError::AccountDir { .. })
    ));

    // A copy of the default, into the App's own tree.
    let dest = world.base.join("accounts/copy");
    let outcome = adapter.import_account(&own, &dest).unwrap();
    let tree = dest.join(".zcode");
    let copied = files(&tree);
    let names: Vec<String> = copied
        .keys()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    for kept in [
        "v2/setting.json",
        "v2/provider_config.json",
        "cli/plugins/data/notes/state.json",
    ] {
        assert!(names.iter().any(|name| name == kept), "{kept}: {names:?}");
    }
    for left in [
        "credentials.json",
        "db.sqlite",
        "cli/log/",
        "cli/rollout/",
        "tasks-index.sqlite",
        "cli/config.json",
        "AGENTS.md",
        "skills/",
    ] {
        assert!(
            !names.iter().any(|name| name.contains(left)),
            "{left} was copied: {names:?}"
        );
    }
    assert!(
        !copied
            .values()
            .any(|bytes| bytes.windows(13).any(|w| w == b"made-up-token"))
    );
    assert!(
        outcome
            .skipped
            .iter()
            .any(|line| line.contains("your sign-in"))
    );
    assert_eq!(
        adapter.login_state(&dest),
        LoginState::LoggedOut,
        "it signs in once"
    );
    let isolated = adapter.pinned_env(&dest);
    assert!(
        isolated.iter().all(|(_, value)| value.is_some()),
        "{isolated:?}"
    );

    // A key written in a settings file stays behind, named.
    let dest = world.base.join("accounts/beta");
    let outcome = adapter.import_account(&beta, &dest).unwrap();
    assert!(!dest.join(".zcode/v2/provider_config.json").exists());
    assert!(
        outcome
            .skipped
            .iter()
            .any(|line| line.contains("provider_config.json"))
    );
    // A folder that isn't ZCode's is refused, and nothing is made.
    let empty = world.base.join("not-zcode");
    std::fs::create_dir_all(&empty).unwrap();
    let dest = world.base.join("accounts/none");
    assert!(adapter.import_account(&empty, &dest).is_err());
    assert!(!dest.exists());
    assert_eq!(
        (files(&own), files(&beta)),
        before,
        "the originals are unchanged"
    );
}

/// Req.5: every App lists and sets up the one set the engine reads, in the
/// home folder's `~/.zcode`, whichever App asks; the App's own tree is
/// never where they go.
#[test]
fn every_app_shares_the_rules_skills_and_servers_in_the_home_folder() {
    let world = World::new();
    let adapter = world.adapter();
    let own = world.home.join(".zcode");
    zcode_fixture::settings(&own);
    let app = world.base.join("accounts/a");
    adapter.prepare_account(&app).unwrap();
    let names = |state_dir: &Path| -> Vec<String> {
        adapter
            .inventory(state_dir, None)
            .into_iter()
            .map(|item| item.name)
            .collect()
    };
    assert_eq!(names(&app), ["files", "release", "AGENTS.md"]);
    assert_eq!(names(&app), names(&own));
    let item = juggler_adapter::inventory::Item {
        tool: "claude-code".into(),
        kind: juggler_adapter::inventory::ItemKind::McpServer,
        name: "docs".into(),
        scope: juggler_adapter::inventory::Scope::User,
        detail: juggler_adapter::inventory::Detail::Mcp(juggler_adapter::inventory::McpServer {
            transport: juggler_adapter::inventory::McpTransport::Http {
                url: "https://docs.example/mcp".into(),
                headers: Vec::new(),
            },
            oauth: false,
        }),
        origin: juggler_adapter::inventory::Origin::Own,
    };
    adapter.set_up(&app, None, &item).unwrap();
    assert!(names(&own).contains(&"docs".to_owned()));
    assert!(!app.join(".zcode/cli/config.json").exists());
}

/// Req.6: what a ZCode session gives a pack — its chat, read-only from
/// the store: user and assistant text in order, a message's text parts
/// joined, tool calls, reasoning and synthetic text left out; a record in
/// another shape is refused whole; any other link is the caller's to read.
#[test]
fn a_sessions_turns_come_from_the_store_for_a_handover() {
    let world = World::new();
    let adapter = world.adapter();
    let a = world.base.join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    let at = 1_792_580_400_000;
    {
        let conn = zcode_fixture::create(&a);
        zcode_fixture::add(&conn, &Row::plain("sess_1", Path::new("/w"), at));
        zcode_fixture::message(
            &conn,
            "sess_1",
            "msg_1",
            "user",
            at,
            &[
                r#"{"type":"text","text":"Fix the cart total"}"#,
                r#"{"type":"text","text":"<system-reminder>","synthetic":true}"#,
            ],
        );
        zcode_fixture::message(
            &conn,
            "sess_1",
            "msg_2",
            "assistant",
            at + 10,
            &[
                r#"{"type":"reasoning","text":"thinking"}"#,
                r#"{"type":"tool","tool":"read","state":{}}"#,
                r#"{"type":"text","text":"Found it in cart.ts."}"#,
                r#"{"type":"text","text":"Fixed."}"#,
            ],
        );
        zcode_fixture::message(
            &conn,
            "sess_1",
            "msg_3",
            "assistant",
            at + 20,
            &[r#"{"type":"tool","tool":"bash","state":{}}"#],
        );
        zcode_fixture::message(
            &conn,
            "sess_2",
            "msg_4",
            "user",
            at,
            &[r#"{"type":"text","text":"another session"}"#],
        );
    }
    let db = zcode_fixture::db(&a);
    let before = std::fs::read(&db).unwrap();
    let reference = juggler_adapters::zcode_sessions::session_ref(&db, "sess_1");
    let turns = adapter.conversation_turns(&reference).unwrap();
    let said: Vec<(&str, &str)> = turns.iter().map(|t| (t.role, t.text.as_str())).collect();
    assert_eq!(
        said,
        [
            ("user", "Fix the cart total"),
            ("assistant", "Found it in cart.ts.\n\nFixed."),
        ]
    );
    assert_eq!(std::fs::read(&db).unwrap(), before, "read-only");
    assert_eq!(adapter.conversation_turns(Path::new("/w/a.jsonl")), None);
    // A changed shape (a message without its role) is refused whole.
    {
        let conn = zcode_fixture::create(&a);
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data)
             VALUES ('msg_9', 'sess_1', ?1, ?1, '{\"author\":\"user\"}')",
            [at + 30],
        )
        .unwrap();
    }
    assert_eq!(adapter.conversation_turns(&reference), None);
}

#[test]
fn the_spec_describes_zcode_as_data() {
    let spec = ZCodeAdapter::new().spec();
    assert_eq!(spec.id, ZCODE_ID);
    assert_eq!(spec.display_name, "ZCode");
    assert_eq!(spec.vendor, "Z.ai");
    assert_eq!(spec.mark.as_deref(), Some("zcode"), "its letter mark");
    let terms = spec.terms.expect("its terms (AJ-E1518)");
    assert!(terms.url.starts_with("https://zcode.z.ai/"));
    assert!(terms.accounts.contains("GLM Coding Plan"));
    assert_eq!(spec.release_feed, None, "it updates with its app");
    assert_eq!(spec.update_command, None);
    assert!(spec.flags.first_run);
    assert_eq!(ZCodeAdapter::new().verified_against(), &[] as &[&str]);
}
