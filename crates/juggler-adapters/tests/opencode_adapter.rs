//! OpenCode adapter tests (AJ-E1602): discovery of v1 and v2, the XDG
//! isolation, launches, the sign-in probe, the session store read
//! read-only, and bringing a setup in. Every database, `auth.json` and
//! home here is synthetic (`opencode_fixture`); no OpenCode was run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod opencode_fixture;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::environment::LoginProbe;
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{
    DiscoveryReport, HarnessAdapter, InstallStatus, LaunchMode, SessionEvent, VerificationStatus,
};
use juggler_adapters::opencode::{Line as OpenCodeLine, line_of, parse_version};
use juggler_adapters::opencode_store::{self, session_of_ref, session_ref};
use juggler_adapters::{LoginState, OPENCODE_ID, OpenCodeAdapter};
use juggler_core::model::{AccountId, Confidence, HarnessId, Profile, ProfileId, SettingsOverlay};
use opencode_fixture::{Assistant, Line};

fn account(state_dir: &Path) -> juggler_core::model::Account {
    juggler_core::model::Account {
        id: AccountId::new("oc-acct"),
        harness: HarnessId::new(OPENCODE_ID),
        label: "OpenCode Go".into(),
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

fn profile(model: Option<&str>, env: BTreeMap<String, String>) -> Profile {
    Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: AccountId::new("oc-acct"),
        settings_overlay: SettingsOverlay {
            model: model.map(str::to_owned),
            env,
            ..SettingsOverlay::default()
        },
        default_cwd: None,
    }
}

/// A stub `opencode` that prints `output` for `--version`.
fn version_stub(dir: &Path, output: &str) -> PathBuf {
    let path = dir.join("opencode");
    std::fs::write(&path, format!("#!/bin/sh\necho {output:?}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// One bounded retry: a freshly written script can fail to execute on a
/// loaded runner (as the Codex and Claude tests do).
fn discover_with_retry(adapter: &OpenCodeAdapter) -> DiscoveryReport {
    let first = adapter.discover();
    if first.is_installed() {
        return first;
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    adapter.discover()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
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
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

// --- discovery ------------------------------------------------------------

#[test]
fn versions_of_both_lines_parse() {
    assert_eq!(parse_version("1.19.3\n").as_deref(), Some("1.19.3"));
    assert_eq!(parse_version("opencode 2.1.2").as_deref(), Some("2.1.2"));
    assert_eq!(parse_version("v2.1.2").as_deref(), Some("2.1.2"));
    assert_eq!(parse_version("opencode\n"), None);
    assert_eq!(parse_version(""), None);
    assert_eq!(line_of("1.19.3"), Some(OpenCodeLine::V1));
    assert_eq!(line_of("2.1.2"), Some(OpenCodeLine::V2));
    assert_eq!(line_of("3.0.0"), None);
}

#[cfg(unix)]
#[test]
fn discovery_tells_the_lines_apart_and_never_claims_a_live_check() {
    for (output, version, note) in [
        ("1.19.3", "1.19.3", "OpenCode 1"),
        ("opencode 2.1.2", "2.1.2", "OpenCode 2"),
        ("3.0.1", "3.0.1", "newer OpenCode"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let report =
            discover_with_retry(&OpenCodeAdapter::with_bin(version_stub(dir.path(), output)));
        assert_eq!(
            report.install,
            InstallStatus::Installed {
                version: version.into()
            }
        );
        // Built from the source, not yet run live (AJ-E1612).
        assert_eq!(report.verification, VerificationStatus::Unverified);
        assert!(
            report.notes.iter().any(|n| n.contains(note)),
            "{output}: {:?}",
            report.notes
        );
        assert_eq!(report.adapter.as_str(), OPENCODE_ID);
    }
}

#[test]
fn a_missing_binary_is_not_found() {
    let report = OpenCodeAdapter::with_bin("/nonexistent/opencode").discover();
    assert_eq!(report.install, InstallStatus::NotFound);
}

// --- isolation --------------------------------------------------------------

#[test]
fn each_account_gets_its_own_xdg_folders() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("accounts/opencode/go");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(dir.path().join("home"));
    let env = adapter.prepare_account(&state).unwrap();
    let at = |relative: &str| state.join(relative).to_string_lossy().into_owned();
    assert_eq!(env.get("XDG_DATA_HOME"), Some(&at("data")));
    assert_eq!(env.get("XDG_CONFIG_HOME"), Some(&at("config")));
    assert_eq!(env.get("XDG_STATE_HOME"), Some(&at("state")));
    assert_eq!(env.get("OPENCODE_CONFIG_DIR"), Some(&at("config/opencode")));
    for folder in ["data/opencode", "config/opencode", "state/opencode"] {
        assert!(state.join(folder).is_dir(), "{folder}");
    }
    let marker = std::fs::read_to_string(state.join(".ai-juggler-managed")).unwrap();
    assert!(marker.contains("harness: opencode"), "{marker}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(state.join("data/opencode"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "sign-ins live here: owner only");
    }
    // The cache is shared (rebuildable packages): never pinned.
    assert!(!env.contains_key("XDG_CACHE_HOME"));
    assert_eq!(adapter.prepare_account(&state).unwrap(), env, "idempotent");
}

#[test]
fn the_default_folder_runs_as_a_plain_opencode() {
    let home = Path::new("/Users/you");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(home);
    let default = home.join(".local/share/opencode");
    assert_eq!(adapter.vendor_default_dir(), Some(default.clone()));
    assert!(
        adapter
            .pinned_env(&default)
            .iter()
            .all(|(_, value)| value.is_none())
    );
    assert_eq!(adapter.db_path(&default), default.join("opencode.db"));
    let snippet =
        juggler_adapter::adapter::shell_snippet(&adapter, &default, &BTreeMap::new()).unwrap();
    assert!(
        snippet.starts_with(
            "env -u XDG_DATA_HOME -u XDG_CONFIG_HOME -u XDG_STATE_HOME -u OPENCODE_CONFIG_DIR -u OPENCODE_DB"
        ),
        "{snippet}"
    );
    assert!(snippet.ends_with(" opencode"), "{snippet}");

    let managed = Path::new("/tmp/acct one");
    let snippet =
        juggler_adapter::adapter::shell_snippet(&adapter, managed, &BTreeMap::new()).unwrap();
    assert!(
        snippet.contains("XDG_DATA_HOME='/tmp/acct one/data' XDG_CONFIG_HOME='/tmp/acct one/config' XDG_STATE_HOME='/tmp/acct one/state' OPENCODE_CONFIG_DIR='/tmp/acct one/config/opencode' opencode"),
        "{snippet}"
    );
    assert_eq!(
        adapter.db_path(managed),
        managed.join("data/opencode/opencode.db")
    );
}

#[test]
fn inherited_sign_ins_and_database_movers_never_reach_a_launch() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("a");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(dir.path());
    adapter.prepare_account(&state).unwrap();
    let spec = adapter
        .launch_spec(
            &account(&state),
            &profile(None, BTreeMap::new()),
            dir.path(),
            &LaunchMode::Interactive,
        )
        .unwrap();
    for key in [
        "OPENCODE_AUTH_CONTENT",
        "OPENCODE_DB",
        "OPENCODE_TEST_HOME",
        "OPENCODE_API_KEY",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
    ] {
        assert_eq!(
            spec.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
    }
}

// --- launches ---------------------------------------------------------------

#[test]
fn launches_per_mode() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("a");
    let adapter = OpenCodeAdapter::with_bin("/usr/local/bin/opencode").with_home(dir.path());
    adapter.prepare_account(&state).unwrap();
    let launch = |mode: LaunchMode, model: Option<&str>| {
        adapter
            .launch_spec(
                &account(&state),
                &profile(model, BTreeMap::new()),
                Path::new("/work/app"),
                &mode,
            )
            .unwrap()
    };

    let tui = launch(LaunchMode::Interactive, None);
    assert!(tui.args.is_empty(), "{:?}", tui.args);
    assert!(tui.needs_pty);
    assert_eq!(tui.cwd, Path::new("/work/app"));

    // A prompt that starts with "-" stays text: the `=` form.
    let primed = launch(
        LaunchMode::Primed {
            prompt: "- fix the build".into(),
            read_dirs: Vec::new(),
        },
        None,
    );
    assert_eq!(primed.args, vec!["--prompt=- fix the build"]);

    let headless = launch(
        LaunchMode::Headless {
            prompt: "-v explain".into(),
        },
        Some("opencode-go/kimi-k3"),
    );
    assert_eq!(
        headless.args,
        vec![
            "run",
            "--format",
            "json",
            "--model=opencode-go/kimi-k3",
            "--",
            "-v explain"
        ]
    );
    assert!(!headless.needs_pty);

    let resumed = launch(
        LaunchMode::Resume {
            session_id: "ses_01J9".into(),
            delta_prompt: Some("carry on".into()),
        },
        None,
    );
    assert_eq!(
        resumed.args,
        vec!["--session", "ses_01J9", "--prompt=carry on"]
    );

    // The TUI's model goes in as inline settings (v2's TUI has no
    // `--model`), merged with any the Launch settings carry.
    let mut env = BTreeMap::new();
    env.insert(
        "OPENCODE_CONFIG_CONTENT".to_owned(),
        r#"{"theme":"system"}"#.to_owned(),
    );
    let spec = adapter
        .launch_spec(
            &account(&state),
            &profile(Some("anthropic/claude-sonnet-5-5"), env),
            Path::new("/work/app"),
            &LaunchMode::Interactive,
        )
        .unwrap();
    let inline: serde_json::Value =
        serde_json::from_str(spec.env.get("OPENCODE_CONFIG_CONTENT").unwrap()).unwrap();
    assert_eq!(inline["model"], "anthropic/claude-sonnet-5-5");
    assert_eq!(inline["theme"], "system");
    assert!(spec.args.is_empty());
}

#[test]
fn sign_in_runs_opencode_auth_login_in_the_accounts_folders() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("a");
    let adapter = OpenCodeAdapter::with_bin("/usr/local/bin/opencode").with_home(dir.path());
    adapter.prepare_account(&state).unwrap();
    let spec = adapter.login_spec(&state).unwrap();
    assert_eq!(spec.args, vec!["auth", "login"]);
    assert!(spec.needs_pty);
    assert_eq!(
        spec.env.get("XDG_DATA_HOME").map(PathBuf::from),
        Some(state.join("data"))
    );
}

// --- sign-in probe ----------------------------------------------------------

#[test]
fn the_sign_in_probe_reads_kinds_never_values() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("a");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(dir.path());
    adapter.prepare_account(&state).unwrap();
    let auth = state.join("data/opencode/auth.json");
    assert_eq!(adapter.login_state(&state), LoginState::LoggedOut);

    for (text, expected) in [
        (
            r#"{"openai":{"type":"oauth","refresh":"r","access":"a","expires":1}}"#,
            LoginState::SignedIn,
        ),
        (
            r#"{"anthropic":{"type":"api","key":"k"}}"#,
            LoginState::ApiKey,
        ),
        // A value of any shape is skipped, not parsed into anything.
        (
            r#"{"x":{"type":"api","key":{"nested":[1,2]}}}"#,
            LoginState::ApiKey,
        ),
        ("{}", LoginState::LoggedOut),
        ("not json", LoginState::Unknown),
        ("", LoginState::Partial),
    ] {
        std::fs::write(&auth, text).unwrap();
        assert_eq!(adapter.login_state(&state), expected, "{text}");
    }
    assert_eq!(
        adapter.login_state(&dir.path().join("missing")),
        LoginState::Unknown,
        "an unreadable folder is unknown, never signed out"
    );
}

#[test]
fn a_v2_sign_in_in_the_database_counts() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("a");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(dir.path());
    adapter.prepare_account(&state).unwrap();
    let db = state.join("data/opencode/opencode.db");
    let conn = opencode_fixture::create(&db, Line::V2);
    opencode_fixture::add_sign_ins(&conn, "made-up", false);
    drop(conn);
    assert_eq!(
        adapter.login_state(&state),
        LoginState::LoggedOut,
        "inactive"
    );
    let conn = opencode_fixture::create(&db, Line::V2);
    conn.execute("UPDATE credential SET active = 1", [])
        .unwrap();
    drop(conn);
    assert_eq!(adapter.login_state(&state), LoginState::SignedIn);
}

// --- the session store ------------------------------------------------------

#[test]
fn sessions_of_both_lines_and_of_an_upgraded_database() {
    let dir = tempfile::tempdir().unwrap();
    for line in [Line::V1, Line::V2] {
        let db = dir.path().join(format!("{line:?}/opencode.db"));
        let conn = opencode_fixture::create(&db, line);
        opencode_fixture::add_session(&conn, line, "ses_a", Path::new("/w/a"), None, 10);
        opencode_fixture::add_session(&conn, line, "ses_b", Path::new("/w/b"), Some("ses_a"), 20);
        drop(conn);
        let sessions = opencode_store::sessions(&db).unwrap();
        assert_eq!(sessions.len(), 2, "{line:?}");
        assert_eq!(sessions[0].id, "ses_b", "newest first");
        assert_eq!(sessions[0].parent_id.as_deref(), Some("ses_a"));
        assert_eq!(sessions[1].directory, "/w/a");
        assert_eq!(sessions[1].title, "Fix the build");
        assert!((sessions[1].cost_usd - 0.42).abs() < 1e-9);
        assert_eq!(sessions[1].tokens.input, 1200);
    }
    // A v1 database v2 upgraded in place: both tables, joined by id.
    let db = dir.path().join("upgraded/opencode.db");
    let conn = opencode_fixture::create(&db, Line::V1);
    opencode_fixture::add_session(&conn, Line::V1, "ses_old", Path::new("/w"), None, 10);
    opencode_fixture::add_session(&conn, Line::V1, "ses_both", Path::new("/w"), None, 11);
    drop(conn);
    let conn = opencode_fixture::create(&db, Line::V2);
    opencode_fixture::add_session(&conn, Line::V2, "ses_both", Path::new("/w"), None, 12);
    opencode_fixture::add_session(&conn, Line::V2, "ses_new", Path::new("/w"), None, 13);
    drop(conn);
    let ids: Vec<String> = opencode_store::sessions(&db)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec!["ses_new", "ses_both", "ses_old"]);

    let not_opencode = dir.path().join("other.db");
    rusqlite::Connection::open(&not_opencode)
        .unwrap()
        .execute_batch("CREATE TABLE t (x)")
        .unwrap();
    assert!(opencode_store::sessions(&not_opencode).is_err());
    assert!(opencode_store::sessions(&dir.path().join("missing.db")).is_err());
}

#[test]
fn usage_reads_completed_assistant_messages_with_their_estimate() {
    let dir = tempfile::tempdir().unwrap();
    for line in [Line::V1, Line::V2] {
        let db = dir.path().join(format!("{line:?}/opencode.db"));
        let conn = opencode_fixture::create(&db, line);
        opencode_fixture::add_session(&conn, line, "ses_1", Path::new("/w"), None, 1_000);
        opencode_fixture::add_user(&conn, line, "msg_u", "ses_1", 1_000);
        for (id, at, completed) in [
            ("msg_1", 1_000, true),
            ("msg_2", 9_000, true),
            ("msg_3", 9_500, false),
        ] {
            opencode_fixture::add_assistant(
                &conn,
                line,
                &Assistant {
                    id,
                    session: "ses_1",
                    created_ms: at,
                    provider: "opencode-go",
                    model: "kimi-k3",
                    input: 500,
                    output: 80,
                    cache_read: 40,
                    cost: 0.0031,
                    completed,
                },
            );
        }
        drop(conn);
        let events = opencode_store::usage_events("acct", &db, 0).unwrap();
        assert_eq!(events.len(), 2, "{line:?}: the running message waits");
        let first = &events[0];
        assert_eq!(first.msg_uuid.as_deref(), Some("opencode:msg_1"));
        assert_eq!(first.session.as_deref(), Some("ses_1"));
        assert_eq!(first.model.as_deref(), Some("opencode-go/kimi-k3"));
        assert_eq!(first.ts, 1_000);
        assert_eq!(first.tokens.input, 500);
        assert_eq!(first.tokens.cache_read, 40);
        assert_eq!(first.tokens.reasoning, 2);
        assert_eq!(first.confidence, Confidence::Measured);
        assert_eq!(first.credits, None, "OpenCode reports no plan credits");
        assert_eq!(first.usd_est, Some(0.0031), "its price is an estimate");
        // Only what changed since the last pass.
        let later = opencode_store::usage_events("acct", &db, 5_000).unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].msg_uuid.as_deref(), Some("opencode:msg_2"));
    }
}

#[test]
fn a_session_link_is_a_reference_no_file_has() {
    let db = Path::new("/acct/data/opencode/opencode.db");
    let reference = session_ref(db, "ses_9");
    assert_eq!(
        reference,
        Path::new("/acct/data/opencode/opencode.db#ses_9")
    );
    assert_eq!(session_of_ref(&reference).as_deref(), Some("ses_9"));
    assert_eq!(session_of_ref(Path::new("/a/b.jsonl")), None);
    assert_eq!(session_of_ref(Path::new("/a/notes#1")), None);
}

#[test]
fn the_watcher_sees_new_and_growing_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data/opencode");
    let db = data.join("opencode.db");
    let conn = opencode_fixture::create(&db, Line::V1);
    opencode_fixture::add_session(&conn, Line::V1, "ses_1", Path::new("/w"), None, 10);
    let adapter = OpenCodeAdapter::with_bin("/bin/echo");
    let mut watcher = adapter.watcher(&data).unwrap();
    let timeout = std::time::Duration::from_millis(1);
    match watcher.poll(timeout).unwrap() {
        Some(SessionEvent::Discovered {
            native_id,
            transcript_path,
        }) => {
            assert_eq!(native_id, "ses_1");
            assert_eq!(transcript_path, session_ref(&db, "ses_1"));
        }
        other => panic!("{other:?}"),
    }
    assert!(watcher.poll(timeout).unwrap().is_none());
    conn.execute("UPDATE session SET time_updated = 99", [])
        .unwrap();
    assert!(matches!(
        watcher.poll(timeout).unwrap(),
        Some(SessionEvent::MessageAppended { native_id, .. }) if native_id == "ses_1"
    ));
}

// --- bringing a setup in ----------------------------------------------------

/// The default setup in `home`: sign-ins, sessions with sign-in tables,
/// logs and snapshots, and settings with an installed plugin package.
fn default_setup(home: &Path, canary: &str) -> PathBuf {
    let data = home.join(".local/share/opencode");
    write(
        &data.join("auth.json"),
        &opencode_fixture::auth_json(canary),
    );
    write(
        &data.join("mcp-auth.json"),
        &format!(r#"{{"gh":"{canary}"}}"#),
    );
    write(&data.join("log/one.log"), "log");
    write(&data.join("snapshot/prj/HEAD"), "ref: x");
    write(&data.join("storage/session/legacy.json"), "{}");
    let conn = opencode_fixture::create(&data.join("opencode.db"), Line::V1);
    opencode_fixture::add_session(&conn, Line::V1, "ses_kept", Path::new("/w"), None, 5);
    opencode_fixture::add_sign_ins(&conn, canary, true);
    drop(conn);
    write(
        &home.join(".config/opencode/opencode.json"),
        r#"{"theme":"system"}"#,
    );
    write(&home.join(".config/opencode/AGENTS.md"), "# rules");
    write(
        &home.join(".config/opencode/node_modules/pkg/index.js"),
        "x",
    );
    data
}

#[test]
fn a_copy_keeps_sessions_and_settings_but_never_a_sign_in() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let canary = "made-up-sign-in-0c1d";
    let source = default_setup(&home, canary);
    let before = files(&home);
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let dest = work.path().join("accounts/opencode/copy");

    let outcome = adapter.import_account(&source, &dest).unwrap();

    assert_eq!(files(&home), before, "the original is untouched");
    assert!(juggler_adapters::claude_code::is_managed_dir(&dest));
    assert!(!dest.join("data/opencode/.ai-juggler-managed").exists());
    let copied = files(&dest);
    for (path, bytes) in &copied {
        assert!(
            !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
            "a sign-in reached {}",
            path.display()
        );
    }
    assert!(!dest.join("data/opencode/auth.json").exists());
    assert!(!dest.join("data/opencode/mcp-auth.json").exists());
    assert!(!dest.join("data/opencode/log").exists());
    assert!(!dest.join("data/opencode/snapshot").exists());
    assert!(
        dest.join("data/opencode/storage/session/legacy.json")
            .is_file()
    );
    assert!(dest.join("config/opencode/opencode.json").is_file());
    assert!(dest.join("config/opencode/AGENTS.md").is_file());
    assert!(!dest.join("config/opencode/node_modules").exists());
    assert!(dest.join("state/opencode").is_dir());

    // The sessions came along; the sign-in tables are empty.
    let db = dest.join("data/opencode/opencode.db");
    let ids: Vec<String> = opencode_store::sessions(&db)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec!["ses_kept"]);
    let conn = rusqlite::Connection::open(&db).unwrap();
    for table in ["credential", "account", "session_share"] {
        let rows: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0, "{table}");
    }
    drop(conn);

    assert_eq!(outcome.login, LoginProbe::LoggedOut);
    assert!(
        outcome
            .login_note
            .as_deref()
            .is_some_and(|note| note.contains("Sign in once")),
        "{:?}",
        outcome.login_note
    );
    assert!(outcome.skipped.iter().any(|s| s.contains("auth.json")));
    assert!(outcome.skipped.iter().any(|s| s.contains("sign-in tables")));
    assert_eq!(adapter.login_state(&dest), LoginState::LoggedOut);
    assert!(
        adapter.import_account(&source, &dest).is_err(),
        "an existing destination is refused"
    );
}

#[test]
fn a_settings_only_copy_leaves_the_history() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = default_setup(&home, "made-up");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let dest = work.path().join("copy");
    let outcome = adapter.import_account_degraded(&source, &dest).unwrap();
    assert!(outcome.degraded);
    assert!(!dest.join("data/opencode/opencode.db").exists());
    assert!(!dest.join("data/opencode/storage").exists());
    assert!(dest.join("config/opencode/opencode.json").is_file());
}

#[test]
fn only_the_default_folder_is_used_where_it_is() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = default_setup(&home, "made-up");
    let before = files(&home);
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let outcome = adapter.adopt_account(&source).unwrap();
    assert_eq!(outcome.login, LoginProbe::LoggedIn);
    assert_eq!(files(&home), before, "nothing written");

    let other = work.path().join("xdg/opencode");
    write(&other.join("auth.json"), "{}");
    let refused = adapter.adopt_account(&other).unwrap_err().to_string();
    assert!(refused.contains("standard folder"), "{refused}");

    // AI Juggler's own copies stay copies.
    let copy = work.path().join("copy");
    adapter.import_account(&source, &copy).unwrap();
    assert!(adapter.adopt_account(&copy).is_err());
}

#[test]
fn the_scan_finds_the_default_and_an_xdg_folder() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    default_setup(&home, "made-up");
    let xdg = work.path().join("xdg");
    let conn = opencode_fixture::create(&xdg.join("opencode/opencode.db"), Line::V2);
    drop(conn);
    let probe = |_: &Path| LoginProbe::LoggedOut;
    let found = juggler_adapters::opencode_environment::scan_opencode_environment(
        &home,
        Some(&xdg),
        &probe,
    );
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0].origin,
        juggler_adapter::environment::InstallOrigin::VendorDefault
    );
    assert!(found[0].path.ends_with(".local/share/opencode"));
    assert_eq!(found[0].harness.as_str(), OPENCODE_ID);
    assert!(found[1].path.ends_with("xdg/opencode"));
    // Nothing found where nothing is.
    let empty = work.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(
        juggler_adapters::opencode_environment::scan_opencode_environment(&empty, None, &probe)
            .is_empty()
    );
}

#[test]
fn inspecting_counts_the_database() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = default_setup(&home, "made-up");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let inspect = adapter.inspect_source(&source).unwrap();
    assert!(inspect.is_config);
    assert!(
        inspect
            .folders
            .iter()
            .any(|(name, bytes)| name == "opencode.db" && *bytes > 0)
    );
}

#[test]
fn a_settings_file_with_a_key_written_in_it_stays_behind_named() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let canary = "made-up-sign-in-0c1d";
    let source = default_setup(&home, canary);
    write(
        &home.join(".config/opencode/opencode.jsonc"),
        "{\n  // a provider with its key written in\n  \"provider\": {\"zen\": {\"options\": {\"apiKey\": \"made-up-key-0123456789abcdef0123456789\"}}},\n}\n",
    );
    // A reference isn't a key: it comes along.
    write(
        &home.join(".config/opencode/tui.json"),
        r#"{"provider":{"go":{"options":{"apiKey":"{env:OPENCODE_API_KEY}"}}}}"#,
    );
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let dest = work.path().join("accounts/opencode/copy");
    let outcome = adapter.import_account(&source, &dest).unwrap();
    assert!(!dest.join("config/opencode/opencode.jsonc").exists());
    assert!(dest.join("config/opencode/tui.json").is_file());
    assert!(dest.join("config/opencode/opencode.json").is_file());
    assert!(
        outcome
            .skipped
            .iter()
            .any(|note| note.starts_with("opencode.jsonc: a key is written in it")),
        "{:?}",
        outcome.skipped
    );
}

#[test]
fn sessions_are_listed_for_the_index_without_sub_agents_or_archived_ones() {
    let work = tempfile::tempdir().unwrap();
    for line in [Line::V1, Line::V2] {
        let state = work.path().join(format!("acct-{line:?}"));
        let db = state.join("data/opencode/opencode.db");
        let conn = opencode_fixture::create(&db, line);
        opencode_fixture::add_session(&conn, line, "ses_top", Path::new("/w/app"), None, 100);
        opencode_fixture::add_session(
            &conn,
            line,
            "ses_child",
            Path::new("/w/app"),
            Some("ses_top"),
            120,
        );
        drop(conn);
        let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(work.path().join("home"));
        let listed = adapter.stored_conversations(&state);
        assert_eq!(listed.len(), 1, "{line:?}: {listed:?}");
        let top = &listed[0];
        assert_eq!(top.summary.id, "ses_top");
        assert_eq!(top.key, session_ref(&db, "ses_top"));
        assert_eq!(top.summary.cwd.as_deref(), Some(Path::new("/w/app")));
        assert_eq!(top.summary.started_ms, Some(100));
        assert!(top.changed_ms >= 100);
        // No file has the key: nothing that opens files misreads it.
        assert!(!top.key.exists());
    }
    // An account with no database yet lists nothing.
    let adapter = OpenCodeAdapter::with_bin("/bin/echo");
    assert!(
        adapter
            .stored_conversations(&work.path().join("none"))
            .is_empty()
    );
}

#[test]
fn a_session_resumes_from_a_terminal_and_a_running_one_is_told_from_helpers() {
    let adapter = OpenCodeAdapter::with_bin("/bin/echo");
    assert_eq!(
        adapter.resume_args("ses_abc"),
        Some(vec!["--session".to_owned(), "ses_abc".to_owned()])
    );
    assert_eq!(adapter.resume_args("--continue"), None);
    let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let bin = Path::new("/opt/homebrew/bin/opencode");
    let tui = adapter
        .session_process(Some(bin), &args(&["opencode", "--session", "ses_abc"]))
        .expect("the TUI is a session");
    assert_eq!(tui.resumes.as_deref(), Some("ses_abc"));
    assert!(
        adapter
            .session_process(Some(bin), &args(&["opencode", "run", "--", "fix it"]))
            .is_some()
    );
    // The npm package, run by node.
    assert!(
        adapter
            .session_process(
                Some(Path::new("/usr/local/bin/node")),
                &args(&[
                    "node",
                    "/usr/local/lib/node_modules/opencode-ai/bin/opencode"
                ]),
            )
            .is_some()
    );
    for helper in ["serve", "auth", "mcp", "upgrade", "stats"] {
        assert!(
            adapter
                .session_process(Some(bin), &args(&["opencode", helper]))
                .is_none(),
            "{helper}"
        );
    }
    assert!(
        adapter
            .session_process(Some(Path::new("/usr/bin/codex")), &args(&["codex"]))
            .is_none()
    );
}

#[test]
fn the_spec_describes_opencode_as_data() {
    let spec = OpenCodeAdapter::new().spec();
    assert_eq!(spec.id, OPENCODE_ID);
    assert_eq!(spec.display_name, "OpenCode");
    assert_eq!(spec.rules_files, vec!["AGENTS.md".to_owned()]);
    assert!(spec.flags.first_run && !spec.flags.primary);
    let terms = spec.terms.expect("its terms are named");
    assert!(terms.url.starts_with("https://opencode.ai/"));
}

#[test]
fn rules_and_servers_are_read_from_the_accounts_own_settings_folder() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(&home);
    // A managed account: its own config folder.
    let managed = work.path().join("accounts/opencode/a");
    write(
        &managed.join("config/opencode/opencode.json"),
        r#"{"mcp":{"files":{"type":"local","command":["files-mcp"]}}}"#,
    );
    write(&managed.join("config/opencode/AGENTS.md"), "# rules");
    let names: Vec<String> = adapter
        .inventory(&managed, None)
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(names, ["files", "AGENTS.md"]);
    // The default folder used in place: the user's own ~/.config/opencode.
    write(
        &home.join(".config/opencode/opencode.json"),
        r#"{"mcp":{"docs":{"type":"remote","url":"https://docs.example/mcp"}}}"#,
    );
    let default = home.join(".local/share/opencode");
    std::fs::create_dir_all(&default).unwrap();
    let names: Vec<String> = adapter
        .inventory(&default, None)
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(names, ["docs"]);
}

/// Every launch of `adapter` against `state`, by mode: the TUI, a primed
/// and a resumed one, a headless run, and the sign-in.
fn every_launch(adapter: &OpenCodeAdapter, state: &Path) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = [
        LaunchMode::Interactive,
        LaunchMode::Primed {
            prompt: "hi".into(),
            read_dirs: Vec::new(),
        },
        LaunchMode::Resume {
            session_id: "ses_01J9".into(),
            delta_prompt: None,
        },
        LaunchMode::Headless {
            prompt: "explain".into(),
        },
    ]
    .iter()
    .map(|mode| {
        adapter
            .launch_spec(
                &account(state),
                &profile(None, BTreeMap::new()),
                Path::new("/work/app"),
                mode,
            )
            .unwrap()
            .args
    })
    .collect();
    out.push(adapter.login_spec(state).unwrap().args);
    out
}

#[test]
fn a_managed_account_on_opencode_2_runs_a_private_server_never_the_shared_service() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let bin = version_stub(work.path(), "2.2.0");
    let adapter = OpenCodeAdapter::with_bin(&bin).with_home(&home);
    assert!(discover_with_retry(&adapter).is_installed());
    let managed = work.path().join("accounts/opencode/a");
    adapter.prepare_account(&managed).unwrap();
    let before = files(&managed);
    let launches = every_launch(&adapter, &managed);
    assert_eq!(launches[0], ["--standalone"]);
    assert_eq!(launches[1], ["--standalone", "--prompt=hi"]);
    assert_eq!(launches[2], ["--standalone", "--session", "ses_01J9"]);
    assert_eq!(
        launches[3],
        ["run", "--standalone", "--format", "json", "--", "explain"]
    );
    assert_eq!(launches[4], ["auth", "login", "--standalone"]);
    // Nothing is written for it: no service settings, no password.
    assert_eq!(files(&managed), before, "a launch wrote to the account");
    assert!(!managed.join("config/opencode/service.json").exists());

    // The default folder used in place keeps the user's shared service, as
    // their own `opencode` does.
    let default = home.join(".local/share/opencode");
    std::fs::create_dir_all(&default).unwrap();
    for args in every_launch(&adapter, &default) {
        assert!(
            !args.iter().any(|arg| arg == "--standalone"),
            "the default folder was given a private server"
        );
    }
}

#[test]
fn opencode_1_and_an_unknown_version_launch_as_before() {
    let work = tempfile::tempdir().unwrap();
    let managed = work.path().join("accounts/opencode/a");
    // OpenCode 1: no service, nothing changes.
    let bin = version_stub(work.path(), "1.19.4");
    let adapter = OpenCodeAdapter::with_bin(&bin).with_home(work.path().join("home"));
    assert!(discover_with_retry(&adapter).is_installed());
    adapter.prepare_account(&managed).unwrap();
    let launches = every_launch(&adapter, &managed);
    assert!(launches[0].is_empty());
    assert_eq!(launches[4], ["auth", "login"]);
    for args in &launches {
        assert!(
            !args.iter().any(|arg| arg == "--standalone"),
            "OpenCode 1 was given a 2.x option"
        );
    }
    // An update to 2.x replaces the binary: its version is asked again.
    let bin = version_stub(work.path(), "opencode 2.2.0");
    assert!(discover_with_retry(&OpenCodeAdapter::with_bin(&bin)).is_installed());
    assert_eq!(every_launch(&adapter, &managed)[0], ["--standalone"]);
    // A version that can't be read: launched as 1.x's, never guessed.
    let echo = OpenCodeAdapter::with_bin("/bin/echo").with_home(work.path().join("home"));
    for args in every_launch(&echo, &managed) {
        assert!(!args.iter().any(|arg| arg == "--standalone"));
    }
}

#[test]
fn a_server_is_set_up_in_the_form_the_installed_opencode_reads() {
    use juggler_adapter::inventory::{Detail, Item, ItemKind, McpServer, McpTransport, Scope};
    let work = tempfile::tempdir().unwrap();
    let item = Item {
        tool: "claude-code".into(),
        kind: ItemKind::McpServer,
        name: "files".into(),
        scope: Scope::User,
        detail: Detail::Mcp(McpServer {
            transport: McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["files-mcp".into()],
                env: Vec::new(),
            },
            oauth: false,
        }),
    };
    for (version, pointer) in [("2.2.0", "/mcp/servers/files"), ("1.19.4", "/mcp/files")] {
        let dir = work.path().join(version);
        std::fs::create_dir_all(&dir).unwrap();
        let bin = version_stub(&dir, version);
        let adapter = OpenCodeAdapter::with_bin(&bin).with_home(work.path().join("home"));
        assert!(discover_with_retry(&adapter).is_installed());
        let managed = dir.join("account");
        adapter.prepare_account(&managed).unwrap();
        adapter.set_up(&managed, None, &item).unwrap();
        let text = std::fs::read_to_string(managed.join("config/opencode/opencode.json")).unwrap();
        let value = juggler_adapter::jsonc::parse(&text).unwrap();
        assert!(
            value
                .pointer(pointer)
                .is_some_and(serde_json::Value::is_object)
        );
        let names: Vec<String> = adapter
            .inventory(&managed, None)
            .into_iter()
            .map(|item| item.name)
            .collect();
        assert_eq!(names, ["files"]);
    }
}
