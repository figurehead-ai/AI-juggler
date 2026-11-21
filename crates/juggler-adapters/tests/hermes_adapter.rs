//! AJ-E1607: the Hermes Agent adapter on fixtures — discovery, each account
//! a home of its own, the guardrail that keeps Hermes away from Codex's and
//! Claude's homes, the terminal launch, sessions from Hermes' own store,
//! `hermes usage --json`, and its gateway's life. Nothing here runs Hermes:
//! a stand-in (`hermes_fixture::STUB`) does what its source says it does
//! with those files.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2)"
)]

mod hermes_fixture;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::prelude::*;
use juggler_adapter::{AdapterError, InstallStatus, LaunchMode, LaunchSpec, LoginState};
use juggler_adapters::HermesAdapter;
use juggler_adapters::hermes::{
    ENV_CLAUDE, ENV_CODEX, ENV_HOME, ENV_LOCKS, ENV_POLICY, ENV_SHARED_AUTH,
};
use juggler_adapters::hermes_guard::{NO_CLAUDE, NO_CODEX, POLICY_DIR};
use juggler_core::model::{Account, AccountId, Profile, ProfileId, SettingsOverlay};
use juggler_core::quota::WindowKind;

struct Kit {
    _root: tempfile::TempDir,
    home: PathBuf,
    accounts: PathBuf,
    bin: PathBuf,
    project: PathBuf,
    /// Where an administrator's policy would be (absent unless a test
    /// makes it).
    system_policy: PathBuf,
}

/// A user's home with Hermes' installer wrapper (the stand-in).
fn kit() -> Kit {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let home = base.join("home");
    let bin = home.join(".local/bin/hermes");
    hermes_fixture::stub(&bin);
    let project = base.join("work/project");
    std::fs::create_dir_all(&project).unwrap();
    Kit {
        home,
        accounts: base.join("accounts"),
        bin,
        project,
        system_policy: base.join("etc-hermes"),
        _root: root,
    }
}

fn adapter(kit: &Kit) -> HermesAdapter {
    HermesAdapter::new()
        .with_home(&kit.home)
        .with_system_policy(&kit.system_policy)
}

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("hermes-a"),
        harness: juggler_core::model::HarnessId::new(juggler_adapters::HERMES_ID),
        label: "Hermes".into(),
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

fn profile(env: &[(&str, &str)], model: Option<&str>) -> Profile {
    Profile {
        id: ProfileId::new("p"),
        name: "p".into(),
        account: AccountId::new("hermes-a"),
        settings_overlay: SettingsOverlay {
            model: model.map(str::to_owned),
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<BTreeMap<_, _>>(),
            restart_on_crash: None,
        },
        default_cwd: None,
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Runs a launch as the app would: the user's login environment (here
/// only `HOME` and `PATH`) with the launch's own set and unset on top.
fn run(kit: &Kit, spec: &LaunchSpec) -> std::process::ExitStatus {
    let mut command = std::process::Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .env("HOME", &kit.home)
        .env("PATH", "/usr/bin:/bin");
    for (key, value) in &spec.env {
        if value == ENV_UNSET {
            command.env_remove(key);
        } else {
            command.env(key, value);
        }
    }
    command.status().unwrap()
}

#[test]
fn hermes_is_found_by_its_wrapper_and_its_banner() {
    let kit = kit();
    let report = adapter(&kit).discover();
    assert_eq!(report.binary.as_deref(), Some(kit.bin.as_path()));
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "0.22.3".into()
        }
    );
    assert_eq!(
        report.verification,
        juggler_adapter::VerificationStatus::Unverified,
        "never run live yet"
    );
}

#[test]
fn each_account_is_a_hermes_home_of_its_own_and_the_users_own_runs_as_hermes_does() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    // Launch settings may not move any of them.
    let hostile = [
        (ENV_HOME, "/elsewhere"),
        (ENV_LOCKS, "/elsewhere/locks"),
        (ENV_CODEX, "/elsewhere/.codex"),
        ("HERMES_KANBAN_HOME", "/elsewhere/kanban"),
        ("HERMES_EXTRA", "kept"),
    ];
    let spec = adapter
        .launch_spec(
            &account(&dir),
            &profile(&hostile, None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    let env = |key: &str| spec.env.get(key).cloned();
    assert_eq!(env(ENV_HOME), Some(text(&dir)));
    assert_eq!(
        env(ENV_LOCKS),
        Some(text(&dir.join(".ai-juggler/gateway-locks"))),
        "its own gateway, never the user's"
    );
    assert_eq!(env(ENV_SHARED_AUTH), Some(text(&dir.join("shared"))));
    assert_eq!(env(ENV_POLICY), Some(text(&dir.join(POLICY_DIR))));
    assert_eq!(env(ENV_CODEX), Some(text(&dir.join(NO_CODEX))));
    assert_eq!(env(ENV_CLAUDE), Some(text(&dir.join(NO_CLAUDE))));
    assert_eq!(env("HERMES_KANBAN_HOME").as_deref(), Some(ENV_UNSET));
    assert_eq!(env("HERMES_DATA_DIR_SUFFIX").as_deref(), Some(ENV_UNSET));
    assert_eq!(env("HERMES_EXTRA").as_deref(), Some("kept"));
    assert!(spec.needs_pty);

    // The user's own ~/.hermes: its home, lock and shared sign-in unset;
    // the guard still on.
    let own = kit.home.join(".hermes");
    let spec = adapter
        .launch_spec(
            &account(&own),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    for key in [ENV_HOME, ENV_LOCKS, ENV_SHARED_AUTH] {
        assert_eq!(
            spec.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
    }
    assert_eq!(spec.env.get(ENV_POLICY), Some(&text(&own.join(POLICY_DIR))));
    assert_eq!(spec.env.get(ENV_CODEX), Some(&text(&own.join(NO_CODEX))));
}

/// Req.3: Hermes never imports or edits a managed Codex or Claude home —
/// not the user's own `~/.codex` and `~/.claude`, not an account folder AI
/// Juggler keeps for either — through a session, its sign-in, its usage
/// read or its gateway, even when the account's own `.env` points it at
/// one.
#[test]
fn hermes_never_reads_or_edits_a_codex_or_claude_home() {
    let kit = kit();
    let adapter = adapter(&kit);
    // The user's own Codex and Claude Code sign-ins, and an account folder
    // AI Juggler keeps for Codex, each with a made-up sign-in.
    let codex = kit.home.join(".codex");
    let claude = kit.home.join(".claude");
    let managed_codex = kit.accounts.join("codex-a");
    for (dir, file) in [
        (&codex, "auth.json"),
        (&claude, ".credentials.json"),
        (&managed_codex, "auth.json"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(file), r#"{"tokens":"made-up"}"#).unwrap();
    }
    let snapshot = || -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = Vec::new();
        for dir in [&codex, &claude, &managed_codex] {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            entries.sort();
            for path in entries {
                files.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
        files
    };
    let before = snapshot();

    // A Hermes account whose own `.env` tries to point it at the managed
    // Codex account and the user's Claude home.
    let dir = kit.accounts.join("hermes-a");
    adapter.prepare_account(&dir).unwrap();
    hermes_fixture::sign_in(&dir, "made-up-hermes-token");
    std::fs::write(
        dir.join(".env"),
        format!(
            "CODEX_HOME={}\nCLAUDE_CONFIG_DIR={}\n",
            managed_codex.display(),
            claude.display()
        ),
    )
    .unwrap();
    let acct = account(&dir);
    for mode in [
        LaunchMode::Interactive,
        LaunchMode::Resume {
            session_id: "20261113_abc".into(),
            delta_prompt: None,
        },
        LaunchMode::Primed {
            prompt: "hello".into(),
            read_dirs: Vec::new(),
        },
    ] {
        let spec = adapter
            .launch_spec(&acct, &profile(&[], None), &kit.project, &mode)
            .unwrap();
        assert!(run(&kit, &spec).success());
    }
    let login = adapter.login_spec(&dir).unwrap();
    assert!(run(&kit, &login).success());
    let saw = std::fs::read_to_string(dir.join(".stub-saw")).unwrap();
    assert!(saw.contains("adopt=false"), "{saw}");
    assert!(
        saw.contains(&format!("codex={}", dir.join(NO_CODEX).display())),
        "{saw}"
    );
    assert!(
        saw.contains(&format!("claude={}", dir.join(NO_CLAUDE).display())),
        "{saw}"
    );
    // Its usage read and its gateway run under the same guard.
    let reading = adapter.plan_usage(&acct).unwrap().unwrap();
    assert_eq!(reading.windows.len(), 2);
    let gateway = adapter.start_gateway(&acct).unwrap();
    wait_for(|| juggler_adapters::hermes_gateway::running(&dir).is_some());
    gateway.stop();
    // The user's own home, used where it is, is kept away from them too.
    let own = kit.home.join(".hermes");
    adapter.adopt_account(&own).ok();
    let spec = adapter
        .launch_spec(
            &account(&own),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert!(run(&kit, &spec).success());
    let saw = std::fs::read_to_string(own.join(".stub-saw")).unwrap();
    assert!(saw.contains("adopt=false"), "{saw}");

    assert_eq!(
        snapshot(),
        before,
        "not a byte of a Codex or Claude home changed"
    );
    assert!(
        !codex.join("config.toml").exists() && !managed_codex.join("config.toml").exists(),
        "no MCP server written into a Codex home"
    );
}

/// Without the guard (the stand-in run as a plain `hermes`), the same
/// homes are edited: the test above proves the guard, not the stand-in.
#[test]
fn the_stand_in_does_edit_them_without_the_guard() {
    let kit = kit();
    let codex = kit.home.join(".codex");
    std::fs::create_dir_all(&codex).unwrap();
    std::fs::write(codex.join("auth.json"), r#"{"tokens":"made-up"}"#).unwrap();
    let status = std::process::Command::new(&kit.bin)
        .env_clear()
        .env("HOME", &kit.home)
        .env("PATH", "/usr/bin:/bin")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        std::fs::read_to_string(codex.join("auth.json"))
            .unwrap()
            .contains("by-hermes")
    );
}

#[test]
fn an_administrators_hermes_policy_is_never_replaced() {
    let kit = kit();
    std::fs::create_dir_all(&kit.system_policy).unwrap();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    let refused = adapter
        .launch_spec(
            &account(&dir),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap_err();
    assert!(matches!(refused, AdapterError::Unsupported(ref why) if why.contains("/etc/hermes")));
    assert!(adapter.login_spec(&dir).is_err());
}

#[test]
fn the_terminal_launch_resumes_seeds_and_never_runs_headless() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    let launch = |mode: &LaunchMode, model: Option<&str>| {
        adapter.launch_spec(&account(&dir), &profile(&[], model), &kit.project, mode)
    };
    let resume = LaunchMode::Resume {
        session_id: "20261113_abc".into(),
        delta_prompt: None,
    };
    assert_eq!(
        launch(&resume, Some("nous/x")).unwrap().args,
        ["-m", "nous/x", "--resume", "20261113_abc"]
    );
    let primed = LaunchMode::Primed {
        prompt: "-starts with a dash".into(),
        read_dirs: Vec::new(),
    };
    assert_eq!(
        launch(&primed, None).unwrap().args,
        ["chat", "--query=-starts with a dash"]
    );
    assert!(matches!(
        launch(&LaunchMode::Headless { prompt: "x".into() }, None),
        Err(AdapterError::Unsupported(_))
    ));
    let bad = LaunchMode::Resume {
        session_id: "--help".into(),
        delta_prompt: None,
    };
    assert!(launch(&bad, None).is_err());
    assert_eq!(
        adapter.resume_args("20261113_abc"),
        Some(vec!["--resume".into(), "20261113_abc".into()])
    );
}

#[test]
fn sessions_come_from_hermes_own_store_read_only() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    let now = hermes_fixture::now_s();
    hermes_fixture::add_session_at(&dir, "20261113_root", &kit.project, "", now - 60.0, None);
    hermes_fixture::add_session_at(
        &dir,
        "20261113_named",
        &kit.project,
        "Fix the build",
        now,
        None,
    );
    hermes_fixture::add_session_at(
        &dir,
        "20261113_child",
        &kit.project,
        "delegated",
        now,
        Some("20261113_root"),
    );
    let store = hermes_fixture::store(&dir);
    let listed = adapter.stored_conversations(&dir);
    let ids: Vec<&str> = listed.iter().map(|c| c.summary.id.as_str()).collect();
    assert_eq!(
        ids,
        ["20261113_named", "20261113_root"],
        "roots only, newest first"
    );
    assert_eq!(listed[0].summary.title.as_deref(), Some("Fix the build"));
    assert!(
        listed[1]
            .summary
            .title
            .as_deref()
            .is_some_and(|title| title.contains("first prompt of 20261113_root")),
        "no title: its first prompt"
    );
    assert_eq!(listed[0].summary.tokens.input, 1200);
    assert_eq!(listed[0].summary.tokens.cache_read, 800);
    assert_eq!(
        listed[0].key,
        juggler_adapters::hermes_sessions::session_ref(&store, "20261113_named")
    );
    // The launch's session, found by its folder and time.
    let found = adapter
        .locate_transcript(&dir, &kit.project, ((now - 10.0) * 1000.0) as i64, &[])
        .unwrap();
    assert_eq!(found.native_id.as_deref(), Some("20261113_named"));
}

#[test]
fn a_store_in_another_format_is_refused_whole() {
    let kit = kit();
    let dir = kit.accounts.join("old");
    let conn = hermes_fixture::create_with(&dir, 32);
    drop(conn);
    let error =
        juggler_adapters::hermes_sessions::sessions(&hermes_fixture::store(&dir)).unwrap_err();
    // The error names the store's path: not printed (CodeQL's
    // cleartext-logging rule), only checked.
    assert!(error.contains("schema 32"), "the older schema isn't named");
    let newer = kit.accounts.join("newer");
    let conn = hermes_fixture::create_with(&newer, 40);
    conn.execute_batch("ALTER TABLE sessions DROP COLUMN cwd")
        .unwrap();
    drop(conn);
    let error =
        juggler_adapters::hermes_sessions::sessions(&hermes_fixture::store(&newer)).unwrap_err();
    assert!(
        error.contains("lacks cwd"),
        "the missing column isn't named"
    );
}

#[test]
fn usage_is_hermes_own_report_only_while_signed_in() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    assert_eq!(adapter.login_state(&dir), LoginState::LoggedOut);
    assert!(
        adapter.plan_usage(&account(&dir)).is_none(),
        "nobody to ask"
    );
    hermes_fixture::sign_in(&dir, "made-up");
    assert_eq!(adapter.login_state(&dir), LoginState::SignedIn);
    let reading = adapter.plan_usage(&account(&dir)).unwrap().unwrap();
    assert_eq!(reading.plan.as_deref(), Some("Plus"));
    assert_eq!(reading.windows[0].0, WindowKind::FiveHour);
    assert_eq!(reading.windows[1].0, WindowKind::Weekly { anchor_ms: None });
    let saw = std::fs::read_to_string(dir.join(".stub-saw")).unwrap();
    assert!(saw.contains("args=usage --json"), "{saw}");
    // Signed in only to Nous Portal: Hermes has no usage for it (exit 1).
    std::fs::remove_file(dir.join("auth.json")).unwrap();
    std::fs::create_dir_all(dir.join("shared")).unwrap();
    std::fs::write(dir.join("shared/nous_auth.json"), "{}").unwrap();
    assert_eq!(adapter.login_state(&dir), LoginState::SignedIn);
    let error = adapter.plan_usage(&account(&dir)).unwrap().unwrap_err();
    assert!(error.contains("Nous Portal"), "{error}");
}

fn wait_for(done: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !done() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(done(), "timed out");
}

fn worker(dir: &Path) -> u32 {
    std::fs::read_to_string(dir.join(".worker.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn the_gateway_ai_juggler_starts_is_its_own_and_ends_with_everything_it_forked() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.accounts.join("a");
    adapter.prepare_account(&dir).unwrap();
    let acct = account(&dir);
    let mut gateway = adapter.start_gateway(&acct).unwrap();
    wait_for(|| {
        dir.join(".worker.pid").exists()
            && juggler_adapters::hermes_gateway::running(&dir).is_some()
    });
    let record = juggler_adapters::hermes_gateway::recorded(&dir).unwrap();
    assert_eq!(
        record.pid,
        gateway.pid(),
        "the record names the process AI Juggler started"
    );
    assert_eq!(record.argv, ["hermes", "gateway", "run"]);
    let saw = std::fs::read_to_string(dir.join(".stub-saw")).unwrap();
    assert!(
        saw.contains("args=gateway run --quiet --external-supervisor"),
        "{saw}"
    );
    assert!(
        saw.contains(&format!(
            "locks={}",
            dir.join(".ai-juggler/gateway-locks").display()
        )),
        "{saw}"
    );
    // A second gateway for the same home is refused while it runs.
    assert!(adapter.start_gateway(&acct).is_err());
    let forked = worker(&dir);
    assert!(gateway.running());
    gateway.stop();
    wait_for(|| !juggler_adapter::process::alive(forked));

    // One Hermes started for a client is ended by Hermes' own stop.
    let mut other = std::process::Command::new(&kit.bin)
        .args(["gateway", "run"])
        .env_clear()
        .env("HOME", &kit.home)
        .env("PATH", "/usr/bin:/bin")
        .env(ENV_HOME, &dir)
        .spawn()
        .unwrap();
    wait_for(|| juggler_adapters::hermes_gateway::running(&dir) == Some(other.id()));
    let forked = worker(&dir);
    adapter.stop_gateway(&acct).unwrap();
    other.wait().unwrap();
    wait_for(|| !juggler_adapter::process::alive(forked));
    assert!(juggler_adapters::hermes_gateway::recorded(&dir).is_none());
}

#[test]
fn a_gateway_record_is_read_only_in_the_shapes_hermes_writes() {
    let dir = tempfile::tempdir().unwrap();
    let read = |text: &str| {
        std::fs::write(dir.path().join("gateway.pid"), text).unwrap();
        juggler_adapters::hermes_gateway::recorded(dir.path())
    };
    assert_eq!(
        read("4242\n").map(|r| r.pid),
        Some(4242),
        "an older bare pid"
    );
    assert_eq!(
        read(r#"{"pid": 4243, "kind": "hermes-gateway", "argv": ["hermes"], "start_time": 9}"#)
            .map(|r| r.pid),
        Some(4243)
    );
    assert!(read(r#"{"pid": 4244, "kind": "something-else"}"#).is_none());
    assert!(
        read(r#"{"pid": 1, "kind": "hermes-gateway"}"#).is_none(),
        "never init"
    );
    assert!(read("garbage").is_none());
}

#[test]
fn a_running_hermes_says_whether_it_is_a_session_or_its_gateway() {
    let kit = kit();
    let adapter = adapter(&kit);
    let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    assert!(
        adapter
            .session_process(Some(&kit.bin), &args(&["hermes", "-r", "x"]))
            .is_some()
    );
    assert!(adapter.is_session_server(Some(&kit.bin), &args(&["hermes", "gateway", "run"])));
    assert!(!adapter.is_session_server(Some(&kit.bin), &args(&["hermes"])));
}

#[test]
fn the_users_homes_and_profiles_are_found_and_a_copy_leaves_sign_ins_and_sessions() {
    let kit = kit();
    let adapter = adapter(&kit);
    let own = kit.home.join(".hermes");
    hermes_fixture::settings(&own);
    hermes_fixture::sign_in(&own, "canary-own");
    let work = own.join("profiles/work");
    hermes_fixture::settings(&work);
    hermes_fixture::add_session(&work, "20261113_w", &kit.project, "work");
    let found: Vec<PathBuf> = adapter
        .scan_environment()
        .into_iter()
        .map(|i| i.path)
        .collect();
    assert!(found.contains(&own) && found.contains(&work), "{found:?}");
    let dest = kit.accounts.join("copy");
    adapter.import_account(&own, &dest).unwrap();
    assert!(dest.join("config.yaml").is_file() && dest.join("SOUL.md").is_file());
    for kept in ["auth.json", ".env"] {
        assert!(!dest.join(kept).exists(), "{kept} stays with the original");
    }
    let dest = kit.accounts.join("copy-work");
    adapter.import_account(&work, &dest).unwrap();
    assert!(
        !dest.join("state.db").exists(),
        "sessions stay with the original"
    );
}
