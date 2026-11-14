//! AJ-E1608: the Muse Code adapter on fixtures — discovery, each account a
//! home of its own, the one-per-Mac subscription sign-in, the terminal
//! launch, sessions from Muse's own index, and its inventory. Nothing here
//! runs Muse or touches a Keychain (`AIJ_NO_KEYCHAIN` is honoured by the
//! keychain seam; these homes aren't the user's anyway).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2)"
)]

mod muse_fixture;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::prelude::*;
use juggler_adapter::{AdapterError, InstallStatus, LaunchMode, LoginState};
use juggler_adapters::MuseAdapter;
use juggler_adapters::muse::{
    ENV_API_KEY, ENV_CLAUDE, ENV_CODEX, ENV_CONFIG, ENV_CREDENTIAL_BACKEND, ENV_DATA, ENV_HOME,
    ENV_NO_UPDATE, FILE_BACKEND, ONE_SIGN_IN,
};
use juggler_core::model::{Account, AccountId, Profile, ProfileId, SettingsOverlay};

struct Kit {
    _root: tempfile::TempDir,
    home: PathBuf,
    bin: PathBuf,
    project: PathBuf,
}

/// A user's home with Muse's launcher and its version record, as the
/// installer leaves them (the launcher a stand-in that prints a version).
fn kit() -> Kit {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let home = base.join("home");
    let bin_dir = home.join(".local/bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let bin = bin_dir.join("muse");
    std::fs::write(&bin, "#!/bin/sh\necho 'Muse Code 9.9.9'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(bin_dir.join(".muse-version"), "1.4.13-R6996.1\n").unwrap();
    let project = base.join("work/project");
    std::fs::create_dir_all(&project).unwrap();
    Kit {
        home,
        bin,
        project,
        _root: root,
    }
}

fn adapter(kit: &Kit) -> MuseAdapter {
    MuseAdapter::new().with_home(&kit.home)
}

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("muse-a"),
        harness: juggler_core::model::HarnessId::new(juggler_adapters::MUSE_ID),
        label: "Muse".into(),
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
        account: AccountId::new("muse-a"),
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

const KEY: &str = "made-up-meta-key-for-a-test";

#[test]
fn muse_is_found_by_its_launcher_and_its_version_record() {
    let kit = kit();
    let report = adapter(&kit).discover();
    assert_eq!(report.binary.as_deref(), Some(kit.bin.as_path()));
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "1.4.13".into()
        },
        "the launcher's own record, not a run"
    );
    // Without the record, Muse itself is asked (with updates off).
    std::fs::remove_file(kit.bin.parent().unwrap().join(".muse-version")).unwrap();
    assert_eq!(
        adapter(&kit).discover().install,
        InstallStatus::Installed {
            version: "9.9.9".into()
        }
    );
}

#[test]
fn each_account_is_a_home_of_its_own_and_the_users_own_runs_as_muse_does() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.home.parent().unwrap().join("accounts/a");
    adapter.prepare_account(&dir).unwrap();
    let spec = adapter
        .launch_spec(
            &account(&dir),
            &profile(&[(ENV_API_KEY, KEY), ("META_EXTRA", "x")], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    let env = |key: &str| spec.env.get(key).cloned();
    assert_eq!(env(ENV_HOME), Some(dir.to_string_lossy().into_owned()));
    assert_eq!(
        env(ENV_CONFIG),
        Some(dir.join(".config").to_string_lossy().into_owned())
    );
    assert_eq!(
        env(ENV_DATA),
        Some(dir.join(".local/share").to_string_lossy().into_owned())
    );
    for (key, sentinel) in [(ENV_CODEX, "no-codex"), (ENV_CLAUDE, "no-claude")] {
        let folder = PathBuf::from(env(key).unwrap());
        assert!(
            folder.starts_with(&dir) && folder.ends_with(sentinel),
            "{key}"
        );
        assert!(
            std::fs::read_dir(&folder).unwrap().next().is_none(),
            "{key} points at an empty folder"
        );
    }
    assert_eq!(
        env(ENV_NO_UPDATE).as_deref(),
        Some("1"),
        "never self-updates"
    );
    assert_eq!(
        env(ENV_API_KEY).as_deref(),
        Some(KEY),
        "the account's own key"
    );
    assert_eq!(env(ENV_CREDENTIAL_BACKEND).as_deref(), Some(ENV_UNSET));
    assert_eq!(env("META_EXTRA").as_deref(), Some("x"));
    assert!(dir.join(".config/muse").is_dir() && dir.join(".local/share/muse").is_dir());

    // The user's own Muse: the XDG variables unset; HOME, CODEX_HOME and
    // CLAUDE_CONFIG_DIR as they are.
    let own = kit.home.join(".config/muse");
    std::fs::create_dir_all(&own).unwrap();
    assert_eq!(adapter.vendor_default_dir().as_deref(), Some(own.as_path()));
    let spec = adapter
        .launch_spec(
            &account(&own),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    for key in [ENV_CONFIG, ENV_DATA] {
        assert_eq!(
            spec.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
    }
    for key in [ENV_HOME, ENV_CODEX, ENV_CLAUDE] {
        assert!(!spec.env.contains_key(key), "{key} is the user's own");
    }
    assert_eq!(
        spec.env.get(ENV_API_KEY).map(String::as_str),
        Some(ENV_UNSET),
        "an inherited key never reaches it"
    );
}

#[test]
fn a_second_subscription_account_is_refused_saying_why() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.home.parent().unwrap().join("accounts/b");
    adapter.prepare_account(&dir).unwrap();
    for mode in [
        LaunchMode::Interactive,
        LaunchMode::Resume {
            session_id: "0198f0aa-1111-7000-8000-0000000000aa".into(),
            delta_prompt: None,
        },
    ] {
        match adapter.launch_spec(&account(&dir), &profile(&[], None), &kit.project, &mode) {
            Err(AdapterError::Unsupported(why)) => assert_eq!(why, ONE_SIGN_IN),
            other => panic!("{mode:?}: {other:?}"),
        }
    }
    match adapter.login_spec(&dir) {
        Err(AdapterError::Unsupported(why)) => assert_eq!(why, ONE_SIGN_IN),
        other => panic!("the sign-in ran: {other:?}"),
    }
    assert_eq!(
        adapter.login_state(&dir),
        LoginState::Unknown,
        "its key is in its Launch settings, not read here"
    );
    assert!(
        !adapter.plan_usage_offered(&dir),
        "a key has no plan windows"
    );

    // The user's own Muse signs in with Muse's own `muse login`.
    let own = kit.home.join(".config/muse");
    std::fs::create_dir_all(&own).unwrap();
    let login = adapter.login_spec(&own).unwrap();
    assert_eq!(login.args, ["login"]);
    assert_eq!(adapter.login_state(&own), LoginState::LoggedOut);
    muse_fixture::sign_in(&own, "made-up");
    assert_eq!(adapter.login_state(&own), LoginState::SignedIn);
    assert!(adapter.plan_usage_offered(&own));
}

#[test]
fn with_separate_sign_ins_on_each_folder_keeps_its_own_in_a_file() {
    let kit = kit();
    let adapter = adapter(&kit).with_separate_sign_ins(true);
    let dir = kit.home.parent().unwrap().join("accounts/c");
    adapter.prepare_account(&dir).unwrap();
    let spec = adapter
        .launch_spec(
            &account(&dir),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(
        spec.env.get(ENV_CREDENTIAL_BACKEND).map(String::as_str),
        Some(FILE_BACKEND)
    );
    let login = adapter.login_spec(&dir).unwrap();
    assert_eq!(
        login.env.get(ENV_CREDENTIAL_BACKEND).map(String::as_str),
        Some(FILE_BACKEND)
    );
    assert_eq!(adapter.login_state(&dir), LoginState::LoggedOut);
    muse_fixture::sign_in(&dir.join(".config/muse"), "made-up");
    assert_eq!(adapter.login_state(&dir), LoginState::SignedIn);
    // The user's own keeps the Keychain.
    let own = kit.home.join(".config/muse");
    std::fs::create_dir_all(&own).unwrap();
    let spec = adapter
        .launch_spec(
            &account(&own),
            &profile(&[], None),
            &kit.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(
        spec.env.get(ENV_CREDENTIAL_BACKEND).map(String::as_str),
        Some(ENV_UNSET)
    );
}

#[test]
fn the_terminal_takes_a_first_message_a_model_and_a_resume() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.home.parent().unwrap().join("accounts/d");
    adapter.prepare_account(&dir).unwrap();
    let recipe = profile(&[(ENV_API_KEY, KEY)], Some("muse-spark-1.3"));
    let primed = adapter
        .launch_spec(
            &account(&dir),
            &recipe,
            &kit.project,
            &LaunchMode::Primed {
                prompt: "--read the handover".into(),
                read_dirs: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(
        primed.args,
        ["--model", "muse-spark-1.3", "--", "--read the handover"],
        "the message is never read as an option"
    );
    assert!(primed.needs_pty);
    let resumed = adapter
        .launch_spec(
            &account(&dir),
            &recipe,
            &kit.project,
            &LaunchMode::Resume {
                session_id: "0198f0aa-1111-7000-8000-0000000000aa".into(),
                delta_prompt: None,
            },
        )
        .unwrap();
    assert_eq!(
        resumed.args,
        [
            "--model",
            "muse-spark-1.3",
            "resume",
            "0198f0aa-1111-7000-8000-0000000000aa"
        ]
    );
    assert!(matches!(
        adapter.launch_spec(
            &account(&dir),
            &recipe,
            &kit.project,
            &LaunchMode::Headless { prompt: "x".into() }
        ),
        Err(AdapterError::Unsupported(_))
    ));
    assert_eq!(
        adapter.resume_args("abc"),
        Some(vec!["resume".to_owned(), "abc".to_owned()])
    );
}

#[test]
fn sessions_come_from_muses_index_and_an_unknown_format_is_refused() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.home.parent().unwrap().join("accounts/e");
    adapter.prepare_account(&dir).unwrap();
    let now = muse_fixture::now_us();
    muse_fixture::add_session_at(&dir, "0198-root", &kit.project, "Fix the tests", now, None);
    muse_fixture::add_session_at(
        &dir,
        "0198-child",
        &kit.project,
        "sub-agent",
        now,
        Some("0198-root"),
    );
    let stored = adapter.stored_conversations(&dir);
    assert_eq!(stored.len(), 1, "a sub-agent's session isn't listed");
    assert_eq!(stored[0].summary.id, "0198-root");
    assert_eq!(
        stored[0].summary.cwd.as_deref(),
        Some(kit.project.as_path())
    );
    let found = adapter
        .locate_transcript(&dir, &kit.project, now / 1_000 - 1_000, &[])
        .unwrap();
    assert_eq!(found.native_id.as_deref(), Some("0198-root"));

    // A schema Muse may move to is refused whole, never half read.
    let other = kit.home.parent().unwrap().join("accounts/f");
    adapter.prepare_account(&other).unwrap();
    muse_fixture::create_with(&other, "2");
    muse_fixture::add_session(&other, "0198-new", &kit.project, "new");
    let conn = muse_fixture::create_with(&other, "2");
    drop(conn);
    let index = muse_fixture::index(&other);
    let error = juggler_adapters::muse_sessions::sessions(&index).unwrap_err();
    assert!(error.contains("format"), "{error}");
    assert!(adapter.stored_conversations(&other).is_empty());
}

#[test]
fn its_inventory_is_the_accounts_own_config() {
    let kit = kit();
    let adapter = adapter(&kit);
    let dir = kit.home.parent().unwrap().join("accounts/g");
    adapter.prepare_account(&dir).unwrap();
    muse_fixture::settings(&dir.join(".config/muse"));
    let names: Vec<String> = adapter
        .inventory(&dir, None)
        .into_iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(names, ["files", "release"]);
}

#[test]
fn only_the_users_own_setup_is_used_where_it_is_and_none_is_copied() {
    let kit = kit();
    let adapter = adapter(&kit);
    let own = kit.home.join(".config/muse");
    muse_fixture::settings(&own);
    assert!(adapter.can_use_in_place());
    assert!(adapter.adopt_account(&own).is_ok());
    let elsewhere = kit.home.join("other-muse");
    muse_fixture::settings(&elsewhere);
    assert!(adapter.adopt_account(&elsewhere).is_err());
    let dest = kit.home.parent().unwrap().join("accounts/copy");
    assert!(matches!(
        adapter.import_account(&own, &dest),
        Err(AdapterError::Unsupported(_))
    ));
    assert!(!dest.exists());
    let found = adapter.scan_environment();
    assert!(found.iter().any(|install| install.path == own));
}
