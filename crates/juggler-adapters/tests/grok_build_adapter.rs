//! Grok Build adapter tests (AJ-E1604): discovery by path and kind (the
//! unofficial `grok` script told apart), `GROK_HOME` isolation with the
//! user's Claude and Cursor setups kept out, launches, the sign-in probe
//! by mode, sessions from the documented layout, and bringing a home in.
//! Every home is synthetic (`grok_fixture`); no Grok was run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod grok_fixture;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use grok_fixture::write;
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{HarnessAdapter, InstallStatus, LaunchMode};
use juggler_adapters::grok_build::{self, is_node_script, parse_version};
use juggler_adapters::{GROK_BUILD_ID, GrokBuildAdapter, LoginState};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("grok-acct"),
        harness: HarnessId::new(GROK_BUILD_ID),
        label: "Grok".into(),
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
        account: AccountId::new("grok-acct"),
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

/// A `grok` stand-in (a shell script, not a Node one, so it isn't taken
/// for the unofficial CLI) that prints `version`.
fn program(dir: &Path, version: &str) -> PathBuf {
    let path = dir.join("grok");
    write(&path, &format!("#!/bin/sh\necho {version:?}\n"));
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
fn the_official_grok_is_found_by_path_and_kind_never_the_unofficial_script() {
    assert_eq!(parse_version("grok 1.0.45\n").as_deref(), Some("1.0.45"));
    assert_eq!(parse_version("v1.0.45").as_deref(), Some("1.0.45"));
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("script/grok");
    write(&script, "#!/usr/bin/env node\nrequire('grok-dev')\n");
    assert!(
        is_node_script(&script),
        "the unofficial grok-cli is a Node script"
    );
    let home = dir.path().join("home");
    let installed = program(&home.join(".grok/bin"), "1.0.45");
    assert!(!is_node_script(&installed));
    let adapter = GrokBuildAdapter::new().with_home(&home);
    let found = adapter.discover();
    assert_eq!(found.binary.as_deref(), Some(installed.as_path()));
    assert_eq!(
        found.install,
        InstallStatus::Installed {
            version: "1.0.45".into()
        }
    );
    assert_eq!(
        found.verification,
        juggler_adapter::VerificationStatus::Unverified
    );
    let missing = GrokBuildAdapter::with_bin("/nonexistent/grok").discover();
    assert_eq!(missing.install, InstallStatus::NotFound);
}

#[test]
fn each_account_has_its_own_home_and_never_runs_the_users_claude_setup() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let adapter = GrokBuildAdapter::with_bin("/bin/echo").with_home(&home);
    let a = work.path().join("accounts/a");
    adapter.prepare_account(&a).unwrap();
    // Overrides try to turn Claude's hooks back on and move the sign-in.
    let hostile = profile(&[
        ("GROK_CLAUDE_HOOKS_ENABLED", "1"),
        ("GROK_AUTH_PATH", "/elsewhere/auth"),
        ("GROK_HOME", "/elsewhere"),
        ("XAI_API_KEY", "made-up"),
    ]);
    for mode in [
        LaunchMode::Interactive,
        LaunchMode::Headless {
            prompt: "hi".into(),
        },
        LaunchMode::Resume {
            session_id: "s1".into(),
            delta_prompt: None,
        },
    ] {
        let spec = adapter
            .launch_spec(&account(&a), &hostile, &a, &mode)
            .unwrap();
        assert_eq!(spec.env["GROK_HOME"], a.to_string_lossy());
        for (key, value) in adapter.launch_fixed_env() {
            assert_eq!(
                spec.env.get(*key).map(String::as_str),
                Some(*value),
                "{key}"
            );
        }
        assert_eq!(spec.env["GROK_AUTH_PATH"], ENV_UNSET);
        assert_eq!(spec.env["XAI_API_KEY"], ENV_UNSET);
    }
    let fixed: Vec<&str> = adapter.launch_fixed_env().iter().map(|(k, _)| *k).collect();
    for part in ["AGENTS", "HOOKS", "MCPS", "RULES", "SKILLS"] {
        assert!(fixed.contains(&format!("GROK_CLAUDE_{part}_ENABLED").as_str()));
        assert!(fixed.contains(&format!("GROK_CURSOR_{part}_ENABLED").as_str()));
    }
    // The copyable line keeps them too.
    let line = juggler_adapter::adapter::shell_snippet(&adapter, &a, &BTreeMap::new()).unwrap();
    assert!(line.contains("GROK_CLAUDE_HOOKS_ENABLED='0'"), "{line}");
    assert!(line.contains("-u GROK_AUTH_PATH"), "{line}");
    // The default home runs with GROK_HOME unset.
    let default = home.join(".grok");
    let spec = adapter
        .launch_spec(
            &account(&default),
            &profile(&[]),
            &a,
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(spec.env["GROK_HOME"], ENV_UNSET);
}

#[test]
fn launches_per_mode() {
    let work = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let a = work.path().join("a");
    let launch = |mode: LaunchMode| {
        adapter.launch_spec(&account(&a), &profile(&[]), Path::new("/w/app"), &mode)
    };
    assert!(launch(LaunchMode::Interactive).unwrap().args.is_empty());
    let resume = launch(LaunchMode::Resume {
        session_id: "0199aa00".into(),
        delta_prompt: None,
    })
    .unwrap();
    assert_eq!(resume.args, ["--resume", "0199aa00"]);
    assert_eq!(resume.cwd, Path::new("/w/app"));
    let headless = launch(LaunchMode::Headless {
        prompt: "explain".into(),
    })
    .unwrap();
    assert_eq!(headless.args, ["-p", "explain", "--output-format", "json"]);
    assert!(!headless.needs_pty);
}

#[test]
fn a_first_message_goes_last_after_the_separator_in_new_and_resumed_sessions() {
    let work = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let a = work.path().join("a");
    let mut settings = profile(&[]);
    settings.settings_overlay.model = Some("grok-build-1".into());
    settings.default_cwd = Some("/w/elsewhere".into());
    let launch = |mode: LaunchMode| {
        adapter
            .launch_spec(&account(&a), &settings, Path::new("/w/app"), &mode)
            .unwrap()
    };
    // A switch's pointer: a fresh session in the project's folder.
    let primed = launch(LaunchMode::Primed {
        prompt: "Read the handover file first".into(),
        read_dirs: vec!["/handover".into()],
    });
    assert_eq!(
        primed.args,
        [
            "--model",
            "grok-build-1",
            "--",
            "Read the handover file first"
        ]
    );
    assert_eq!(primed.cwd, Path::new("/w/app"));
    assert!(primed.needs_pty);
    // Text that looks like an option or a subcommand stays a message.
    for text in ["- fix the build", "logout", "--resume x"] {
        let spec = launch(LaunchMode::Primed {
            prompt: text.into(),
            read_dirs: Vec::new(),
        });
        assert_eq!(spec.args.last().map(String::as_str), Some(text));
        assert_eq!(
            spec.args.iter().filter(|arg| *arg == "--").count(),
            1,
            "the message must follow exactly one separator"
        );
    }
    // A resume with a message: the session first, the message last.
    let resumed = launch(LaunchMode::Resume {
        session_id: "0199aa00".into(),
        delta_prompt: Some("go on".into()),
    });
    assert_eq!(
        resumed.args,
        [
            "--resume",
            "0199aa00",
            "--model",
            "grok-build-1",
            "--",
            "go on"
        ]
    );
    // And the running session is still recognised, its message never
    // taken for a helper.
    let args: Vec<String> = std::iter::once("grok".to_owned())
        .chain(resumed.args.iter().cloned())
        .collect();
    let process = grok_build::session_process(Some(Path::new("/u/.grok/bin/grok")), &args)
        .expect("a resumed session with a message is a session");
    assert_eq!(process.resumes.as_deref(), Some("0199aa00"));
    let logout: Vec<String> = ["grok", "--", "logout"].map(str::to_owned).to_vec();
    assert!(grok_build::session_process(Some(Path::new("/u/.grok/bin/grok")), &logout).is_some());
    let helper: Vec<String> = ["grok", "logout"].map(str::to_owned).to_vec();
    assert!(grok_build::session_process(Some(Path::new("/u/.grok/bin/grok")), &helper).is_none());
}

#[test]
fn the_sign_in_probe_reads_modes_never_values() {
    let work = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let home = work.path().join("home");
    assert_eq!(adapter.login_state(&home), LoginState::Unknown);
    std::fs::create_dir_all(&home).unwrap();
    assert_eq!(adapter.login_state(&home), LoginState::LoggedOut);
    grok_fixture::sign_in(&home, "made-up");
    assert_eq!(adapter.login_state(&home), LoginState::SignedIn);
    grok_fixture::sign_in_api_key(&home);
    assert_eq!(adapter.login_state(&home), LoginState::ApiKey);
    write(&home.join("auth.json"), "");
    assert_eq!(adapter.login_state(&home), LoginState::Partial);
    write(&home.join("auth.json"), "not json");
    assert_eq!(adapter.login_state(&home), LoginState::Unknown);
}

#[test]
fn sign_in_runs_grok_login_in_the_accounts_home() {
    let work = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let home = work.path().join("accounts/a");
    let spec = adapter.login_spec(&home).unwrap();
    assert_eq!(spec.args, ["login"]);
    assert_eq!(spec.env["GROK_HOME"], home.to_string_lossy());
    assert_eq!(spec.env["GROK_CLAUDE_MCPS_ENABLED"], "0");
}

#[test]
fn sessions_are_listed_resumed_and_told_from_helpers() {
    let work = tempfile::tempdir().unwrap();
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let home = work.path().join("home/.grok-work");
    let app = work.path().join("my app");
    std::fs::create_dir_all(&app).unwrap();
    let record = grok_fixture::add_session(&home, "0199aa00-1", &app, "Fix the cart");
    assert_eq!(adapter.conversation_files(&home), vec![record.clone()]);
    let read = adapter.read_conversation(&record).unwrap();
    assert_eq!(read.id, "0199aa00-1");
    assert_eq!(read.title.as_deref(), Some("Fix the cart"));
    assert_eq!(read.cwd.as_deref(), Some(app.as_path()));
    let found = adapter.locate_transcript(&home, &app, 0, &[]).unwrap();
    assert_eq!(found.native_id.as_deref(), Some("0199aa00-1"));
    assert_eq!(
        adapter.resume_args("0199aa00-1"),
        Some(vec!["--resume".to_owned(), "0199aa00-1".to_owned()])
    );
    let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let bin = Path::new("/Users/you/.grok/bin/grok");
    let tui = adapter
        .session_process(Some(bin), &args(&["grok", "-r", "0199aa00-1"]))
        .unwrap();
    assert_eq!(tui.resumes.as_deref(), Some("0199aa00-1"));
    for helper in ["login", "update", "mcp", "agent"] {
        assert!(
            adapter
                .session_process(Some(bin), &args(&["grok", helper]))
                .is_none()
        );
    }
}

#[test]
fn a_copy_keeps_settings_and_sessions_but_never_a_sign_in_or_a_key() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = home.join(".grok");
    let canary = "made-up-sign-in-9d4b";
    grok_fixture::settings(&source, canary);
    grok_fixture::sign_in(&source, canary);
    grok_fixture::add_session(&source, "s1", Path::new("/w/app"), "old");
    let before = files(&source);
    let adapter = GrokBuildAdapter::with_bin("/bin/echo").with_home(&home);
    let dest = work.path().join("accounts/grok-build/copy");
    let outcome = adapter.import_account(&source, &dest).unwrap();
    assert_eq!(files(&source), before, "the original is untouched");
    for (path, bytes) in files(&dest) {
        assert!(
            !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
            "a sign-in or key reached {}",
            path.display()
        );
    }
    for gone in [
        "auth.json",
        "mcp_credentials.json",
        "config.toml",
        "bin",
        "downloads",
    ] {
        assert!(!dest.join(gone).exists(), "{gone} came along");
    }
    for kept in ["pager.toml", "rules/style.md"] {
        assert!(dest.join(kept).is_file(), "{kept} stayed behind");
    }
    assert_eq!(adapter.conversation_files(&dest).len(), 1, "sessions came");
    assert!(
        outcome
            .skipped
            .iter()
            .any(|note| note.starts_with("config.toml: a key is written in it"))
    );
    // In place: nothing written, its setup read where it is.
    let adopted = adapter.adopt_account(&source).unwrap();
    assert_eq!(
        adopted.login,
        juggler_adapter::environment::LoginProbe::LoggedIn
    );
    assert_eq!(files(&source), before);
}

#[test]
fn the_scan_finds_homes_but_not_the_unofficial_clis_folder() {
    let work = tempfile::tempdir().unwrap();
    let canonical = work.path().canonicalize().unwrap();
    let home = canonical.as_path();
    grok_fixture::settings(&home.join(".grok-work"), "made-up");
    // The unofficial grok-cli's folder: only its own settings file.
    write(&home.join(".grok/user-settings.json"), "{}");
    let probe = |_: &Path| juggler_adapter::environment::LoginProbe::LoggedOut;
    let found = juggler_adapters::grok_environment::scan_grok_environment(home, None, &probe);
    let paths: Vec<PathBuf> = found.iter().map(|install| install.path.clone()).collect();
    assert_eq!(paths, vec![home.join(".grok-work")]);
}

#[test]
fn the_spec_describes_grok_build_as_data() {
    let spec = GrokBuildAdapter::new().spec();
    assert_eq!(spec.id, GROK_BUILD_ID);
    assert_eq!(spec.display_name, "Grok Build");
    assert!(spec.terms.is_some());
    assert_eq!(spec.update_command.as_deref(), Some("grok update"));
}
