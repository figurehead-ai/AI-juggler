//! Kimi Code adapter tests (AJ-E1603): discovery (and the old Python CLI
//! told apart), `KIMI_CODE_HOME` isolation, launches and the
//! interactive-only guardrail, the sign-in probe by presence, sessions
//! from the documented layout, and bringing a home in. Every home here is
//! synthetic (`kimi_fixture`); no Kimi Code was run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod kimi_fixture;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{AdapterError, HarnessAdapter, InstallStatus, LaunchMode};
use juggler_adapters::kimi_code::{is_kimi_code, parse_version};
use juggler_adapters::{KIMI_CODE_ID, KimiCodeAdapter, LoginState};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
use kimi_fixture::write;

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("kimi-acct"),
        harness: HarnessId::new(KIMI_CODE_ID),
        label: "Kimi".into(),
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

fn profile(model: Option<&str>) -> Profile {
    Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: AccountId::new("kimi-acct"),
        settings_overlay: SettingsOverlay {
            model: model.map(str::to_owned),
            env: BTreeMap::from([("KIMI_CODE_HOME".to_owned(), "/elsewhere".to_owned())]),
            ..SettingsOverlay::default()
        },
        default_cwd: None,
    }
}

/// A stub `kimi` that prints `output` for `--version`.
fn stub(dir: &Path, output: &str) -> PathBuf {
    let path = dir.join("kimi");
    write(&path, &format!("#!/bin/sh\necho {output:?}\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
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

#[test]
fn versions_parse_and_the_old_python_cli_is_told_apart() {
    assert_eq!(parse_version("2.3.0\n").as_deref(), Some("2.3.0"));
    assert_eq!(parse_version("kimi-code v2.3.0").as_deref(), Some("2.3.0"));
    assert_eq!(
        parse_version("kimi, version 1.52.0").as_deref(),
        Some("1.52.0")
    );
    assert_eq!(parse_version("usage: kimi"), None);
    assert!(is_kimi_code("2.3.0") && is_kimi_code("10.0.0"));
    assert!(!is_kimi_code("1.52.0") && !is_kimi_code("0.9"));

    let dir = tempfile::tempdir().unwrap();
    let found = KimiCodeAdapter::with_bin(stub(dir.path(), "2.3.0")).discover();
    assert_eq!(
        found.install,
        InstallStatus::Installed {
            version: "2.3.0".into()
        }
    );
    assert_eq!(
        found.verification,
        juggler_adapter::VerificationStatus::Unverified,
        "no live check yet"
    );
    let old = tempfile::tempdir().unwrap();
    let legacy = KimiCodeAdapter::with_bin(stub(old.path(), "kimi, version 1.52.0")).discover();
    assert_eq!(legacy.install, InstallStatus::NotFound);
    assert!(
        legacy.notes[0].contains("old Kimi CLI"),
        "{:?}",
        legacy.notes
    );
    let missing = KimiCodeAdapter::with_bin("/nonexistent/kimi").discover();
    assert_eq!(missing.install, InstallStatus::NotFound);
}

#[test]
fn each_account_has_its_own_home_and_the_default_runs_as_a_plain_kimi() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let adapter = KimiCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let a = work.path().join("accounts/a");
    let env = adapter.prepare_account(&a).unwrap();
    assert_eq!(
        env.get("KIMI_CODE_HOME").map(String::as_str),
        Some(a.to_str().unwrap())
    );
    assert!(juggler_adapters::claude_code::is_managed_dir(&a));
    let spec = adapter
        .launch_spec(&account(&a), &profile(None), &a, &LaunchMode::Interactive)
        .unwrap();
    assert_eq!(
        spec.env.get("KIMI_CODE_HOME").map(String::as_str),
        Some(a.to_str().unwrap()),
        "an override never moves the home"
    );
    assert!(
        spec.env
            .keys()
            .all(|key| !key.starts_with("KIMI_CODE_IDENTITY")),
        "its client identity is never set"
    );
    let default = home.join(".kimi-code");
    let spec = adapter
        .launch_spec(
            &account(&default),
            &profile(None),
            &a,
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(
        spec.env.get("KIMI_CODE_HOME").map(String::as_str),
        Some(ENV_UNSET)
    );
}

#[test]
fn launches_per_mode_and_headless_only_on_an_api_key() {
    let work = tempfile::tempdir().unwrap();
    let adapter = KimiCodeAdapter::with_bin("/bin/echo").with_home(work.path().join("home"));
    let home = work.path().join("accounts/a");
    adapter.prepare_account(&home).unwrap();
    let launch = |mode: LaunchMode, model: Option<&str>| {
        adapter.launch_spec(&account(&home), &profile(model), Path::new("/w/app"), &mode)
    };
    let tui = launch(LaunchMode::Interactive, Some("k3-256k")).unwrap();
    assert_eq!(tui.args, ["-m", "k3-256k"]);
    assert!(tui.needs_pty);
    let resume = launch(
        LaunchMode::Resume {
            session_id: "ses-1".into(),
            delta_prompt: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(resume.args, ["--session", "ses-1"]);
    assert_eq!(resume.cwd, Path::new("/w/app"));
    // No documented way to hand it a message: refused, never dropped.
    for mode in [
        LaunchMode::Resume {
            session_id: "ses-1".into(),
            delta_prompt: Some("go on".into()),
        },
        LaunchMode::Primed {
            prompt: "hi".into(),
            read_dirs: Vec::new(),
        },
    ] {
        assert!(matches!(
            launch(mode, None),
            Err(AdapterError::Unsupported(_))
        ));
    }
    let headless = || {
        launch(
            LaunchMode::Headless {
                prompt: "explain".into(),
            },
            None,
        )
    };
    // Signed out, or on the membership: interactive use only.
    assert!(
        matches!(headless(), Err(AdapterError::Unsupported(why)) if why.contains("interactive use only"))
    );
    kimi_fixture::sign_in_membership(&home, "made-up");
    assert!(adapter.interactive_only(&home).is_some());
    assert!(matches!(headless(), Err(AdapterError::Unsupported(_))));
    // A Kimi Platform API key: its headless runs are allowed.
    std::fs::remove_file(home.join("credentials/kimi-code.json")).unwrap();
    kimi_fixture::sign_in_api_key(&home);
    assert_eq!(adapter.interactive_only(&home), None);
    let run = headless().unwrap();
    assert_eq!(
        run.args,
        ["-p", "explain", "--output-format", "stream-json"]
    );
    assert!(!run.needs_pty);
}

#[test]
fn the_sign_in_probe_reads_which_files_are_there_never_what_they_hold() {
    let work = tempfile::tempdir().unwrap();
    let adapter = KimiCodeAdapter::with_bin("/bin/echo");
    let home = work.path().join("home");
    assert_eq!(adapter.login_state(&home), LoginState::Unknown, "no folder");
    std::fs::create_dir_all(&home).unwrap();
    assert_eq!(adapter.login_state(&home), LoginState::LoggedOut);
    // Contents don't matter: not even valid JSON is needed, none is read.
    write(&home.join("credentials/kimi-code.json"), "not json at all");
    assert_eq!(adapter.login_state(&home), LoginState::SignedIn);
    std::fs::remove_file(home.join("credentials/kimi-code.json")).unwrap();
    write(&home.join("credentials/platform.json"), "x");
    assert_eq!(adapter.login_state(&home), LoginState::ApiKey);
    write(&home.join("credentials/kimi-code.json"), "");
    assert_eq!(
        adapter.login_state(&home),
        LoginState::ApiKey,
        "an empty membership file isn't a sign-in"
    );
}

#[test]
fn sign_in_runs_kimi_login_in_the_accounts_home() {
    let work = tempfile::tempdir().unwrap();
    let adapter = KimiCodeAdapter::with_bin("/bin/echo");
    let home = work.path().join("accounts/a");
    let spec = adapter.login_spec(&home).unwrap();
    assert_eq!(spec.args, ["login"]);
    assert!(spec.needs_pty);
    assert_eq!(
        spec.env.get("KIMI_CODE_HOME").map(String::as_str),
        Some(home.to_str().unwrap())
    );
}

#[test]
fn sessions_are_listed_resumed_and_told_from_helpers() {
    let work = tempfile::tempdir().unwrap();
    let adapter = KimiCodeAdapter::with_bin("/bin/echo");
    let home = work.path().join("home/.kimi-code-work");
    let app = work.path().join("app");
    std::fs::create_dir_all(&app).unwrap();
    let record = kimi_fixture::add_session(&home, "ses-9", &app, "Fix the cart");
    assert_eq!(adapter.conversation_files(&home), vec![record.clone()]);
    let read = adapter.read_conversation(&record).unwrap();
    assert_eq!(read.id, "ses-9");
    assert_eq!(read.title.as_deref(), Some("Fix the cart"));
    assert_eq!(read.cwd.as_deref(), Some(app.as_path()));
    let found = adapter.locate_transcript(&home, &app, 0, &[]).unwrap();
    assert_eq!(found.native_id.as_deref(), Some("ses-9"));
    assert_eq!(found.path, record);
    assert_eq!(
        adapter.resume_args("ses-9"),
        Some(vec!["--session".to_owned(), "ses-9".to_owned()])
    );
    assert_eq!(adapter.resume_args("-c"), None);

    let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let bin = Path::new("/Users/you/.kimi-code/bin/kimi");
    let tui = adapter
        .session_process(Some(bin), &args(&["kimi", "-S", "ses-9"]))
        .expect("the terminal is a session");
    assert_eq!(tui.resumes.as_deref(), Some("ses-9"));
    assert!(
        adapter
            .session_process(
                Some(Path::new("/usr/local/bin/node")),
                &args(&[
                    "node",
                    "/usr/local/lib/node_modules/@moonshot-ai/kimi-code/dist/cli.js"
                ]),
            )
            .is_some()
    );
    for helper in ["web", "login", "acp", "upgrade"] {
        assert!(
            adapter
                .session_process(Some(bin), &args(&["kimi", helper]))
                .is_none(),
            "{helper}"
        );
    }
}

#[test]
fn the_scan_finds_the_default_an_override_and_other_homes_but_not_the_old_cli() {
    let work = tempfile::tempdir().unwrap();
    let canonical = work.path().canonicalize().unwrap();
    let home = canonical.as_path();
    for dir in [".kimi-code", ".kimi-code-work", "xdg/kimi"] {
        kimi_fixture::settings(&home.join(dir), "made-up");
    }
    write(&home.join(".kimi/config.json"), "{}");
    let probe = |_: &Path| juggler_adapter::environment::LoginProbe::LoggedOut;
    let found = juggler_adapters::kimi_environment::scan_kimi_environment(
        home,
        Some(&home.join("xdg/kimi")),
        &probe,
    );
    let mut paths: Vec<String> = found
        .iter()
        .map(|install| {
            install
                .path
                .strip_prefix(home)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    paths.sort();
    assert_eq!(paths, [".kimi-code", ".kimi-code-work", "xdg/kimi"]);
    assert!(
        found
            .iter()
            .all(|install| install.harness.as_str() == KIMI_CODE_ID)
    );
}

#[test]
fn a_copy_keeps_settings_but_never_a_sign_in_a_key_or_the_sessions() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = home.join(".kimi-code");
    let canary = "made-up-sign-in-5e2a";
    kimi_fixture::settings(&source, canary);
    kimi_fixture::sign_in_membership(&source, canary);
    kimi_fixture::add_session(&source, "ses-1", Path::new("/w/app"), "old");
    let before = files(&source);
    let adapter = KimiCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let dest = work.path().join("accounts/kimi-code/copy");

    let outcome = adapter.import_account(&source, &dest).unwrap();

    assert_eq!(files(&source), before, "the original is untouched");
    let copied = files(&dest);
    for (path, bytes) in &copied {
        assert!(
            !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
            "a sign-in or key reached {}",
            path.display()
        );
    }
    for gone in [
        "credentials",
        "server.token",
        "sessions",
        "session_index.jsonl",
        "config.toml",
        "logs",
        "device_id",
    ] {
        assert!(!dest.join(gone).exists(), "{gone} came along");
    }
    for kept in [
        "tui.toml",
        "mcp.json",
        "AGENTS.md",
        "skills/release/SKILL.md",
    ] {
        assert!(dest.join(kept).is_file(), "{kept} stayed behind");
    }
    assert!(
        outcome
            .skipped
            .iter()
            .any(|note| note.starts_with("config.toml: a key is written in it")),
        "{:?}",
        outcome.skipped
    );
    assert_eq!(adapter.login_state(&dest), LoginState::LoggedOut);
}

#[test]
fn a_home_is_used_where_it_is_unwritten_and_its_setup_read_from_it() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = home.join(".kimi-code");
    kimi_fixture::settings(&source, "made-up");
    kimi_fixture::sign_in_membership(&source, "made-up");
    let before = files(&source);
    let adapter = KimiCodeAdapter::with_bin("/bin/echo").with_home(&home);
    let adopted = adapter.adopt_account(&source).unwrap();
    assert_eq!(
        adopted.login,
        juggler_adapter::environment::LoginProbe::LoggedIn
    );
    assert_eq!(adapter.vendor_default_dir(), Some(source.clone()));
    let names: Vec<String> = adapter
        .inventory(&source, None)
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(names, ["files", "release", "AGENTS.md"]);
    assert_eq!(files(&source), before, "nothing was written");
}

#[test]
fn the_spec_describes_kimi_code_as_data() {
    let spec = KimiCodeAdapter::new().spec();
    assert_eq!(spec.id, KIMI_CODE_ID);
    assert_eq!(spec.display_name, "Kimi Code");
    assert_eq!(spec.rules_files, vec!["AGENTS.md".to_owned()]);
    let terms = spec.terms.expect("its terms are named");
    assert!(terms.accounts.contains("interactive use only"));
}
