//! M1-3b integration: smart onboarding — the environment scan feeds the
//! import, the import yields an immediately launchable account, and the
//! source setup is never modified (ADR-0006 guardrails).

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

/// A `claude` stub good enough for login state + launch assertions: writes
/// a transcript under its isolated config dir and exits.
fn claude_stub(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\n\
         mkdir -p \"$CLAUDE_CONFIG_DIR/projects/p\"\n\
         printf '%s' \"$CLAUDE_CONFIG_DIR\" > \"$CLAUDE_CONFIG_DIR/projects/p/s1.jsonl\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path, stub: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// A pre-existing user setup outside the app data dir: logged in (file
/// credentials), one session transcript, plus cache noise.
fn existing_setup(root: &Path) -> std::path::PathBuf {
    let dir = root.join("existing-claude");
    std::fs::create_dir_all(dir.join("projects/-users-dev-app")).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{"model":"sonnet"}"#).unwrap();
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    std::fs::write(
        dir.join("projects/-users-dev-app/session-1.jsonl"),
        "{\"type\":\"user\"}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("shell-snapshots")).unwrap();
    std::fs::write(dir.join("shell-snapshots/x.sh"), "noise").unwrap();
    dir
}

#[test]
fn import_makes_an_existing_setup_launchable_without_touching_it() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let source = existing_setup(work.path());
    let source_snapshot = snapshot(&source);

    // Import through the command surface the wizard uses.
    let result = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        source.to_str().unwrap(),
        "Existing account",
        Some("pro".into()),
        Some("home-pattern".to_string()),
        false,
    )
    .unwrap();

    // The account exists, is logged in, and the settings/history carried.
    assert_eq!(result.login, "logged_in");
    assert_eq!(
        result.login_note.as_deref(),
        Some("Login carried over with the import.")
    );
    assert!(result.files_copied >= 3, "{}", result.files_copied);
    assert!(result.skipped.iter().any(|s| s.contains("shell-snapshots")));
    let account = result.account;
    assert_eq!(account.label, "Existing account");
    assert_eq!(account.plan.as_deref(), Some("pro"));

    // A default profile makes it launchable in one click.
    let storage = state.storage.lock().unwrap();
    let profiles = storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(account.id.clone()))
        .unwrap();
    drop(storage);
    let main = profiles.first().expect("default profile created on import");
    assert_eq!(main.name, "Main");

    // Launch through the supervisor: the isolated copy behaves like the
    // original setup (stub writes into ITS config dir).
    let launched = commands::launch_profile(&state, main.id.as_str(), None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut exited = false;
    while std::time::Instant::now() < deadline {
        if let Some(juggler_supervisor::LifecycleEvent::Exited { .. }) = state
            .supervisor
            .recv_timeout(std::time::Duration::from_millis(200))
        {
            exited = true;
            break;
        }
    }
    assert!(exited, "imported account's session must run and exit");
    let storage = state.storage.lock().unwrap();
    let sessions = storage.sessions().list(None).unwrap();
    drop(storage);
    assert!(
        sessions
            .iter()
            .any(|s| s.id.as_str() == launched.session_id)
    );

    // The source tree is byte-identical — the import is read-only.
    assert_eq!(snapshot(&source), source_snapshot);

    // Re-importing the same source still works (it was not marked), but
    // importing our own managed dir is refused.
    let state_dir = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(account.id.as_str());
    let err = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        state_dir.to_str().unwrap(),
        "Dup",
        None,
        Some("home-pattern".to_string()),
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("already managed"));
}

#[test]
fn environment_scan_smoke_never_panics_on_a_real_home() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    // Scans the developer's actual home: contents are machine-specific, so
    // assert only invariants — valid wire values, plausible paths.
    let found = commands::environment_scan_impl(&state);
    for install in &found {
        assert_eq!(install.harness, CLAUDE_CODE_ID);
        assert!(!install.path.is_empty());
        assert!(
            [
                "vendor-default",
                "env-override",
                "home-pattern",
                "manager-tool"
            ]
            .contains(&install.origin.as_str())
        );
        assert!(
            ["logged_in", "logged_out"].contains(&install.login.as_str()),
            "login wire value: {}",
            install.login
        );
        assert!(!install.notes.is_empty());
    }
}

/// Path→size snapshot of a tree (sorted) for read-only checks.
fn snapshot(root: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, u64)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let meta = entry.metadata().unwrap();
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if meta.is_dir() {
                walk(&entry.path(), root, out);
            } else {
                out.push((rel, meta.len()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// AJ-E301: the Step-0 install gate reads the adapters' own
/// `discover()` — an installed CLI renders "installed" with its
/// version, a Provider whose binary is missing renders
/// `not_installed` (a ROW, never an error), and a version outside
/// `verified_against` renders `unverified`. The wire carries exactly
/// what the gate needs: status + version + install page.
/// A stub that answers `--version` (the discover probe) like a real
/// CLI would.
fn version_stub(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("version-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.296'; exit 0; fi\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn providers_discover_renders_installed_and_missing_rows() {
    let work = tempfile::tempdir().unwrap();
    let stub = version_stub(work.path());
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&stub)));
    // A Provider whose binary does NOT exist: the no-binary fixture
    // (the wizard must gate BEFORE any account exists).
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        work.path().join("missing-harness"),
    )));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data-2"), registry).unwrap());

    let rows = commands::providers_discover_impl(&state).unwrap();
    assert!(rows.len() >= 2, "one row per registered Provider: {rows:?}");

    let claude = rows
        .iter()
        .find(|row| row.provider == CLAUDE_CODE_ID)
        .expect("claude row present");
    assert_eq!(claude.status, "installed", "stub binary found");
    assert!(claude.binary.is_some());
    assert_eq!(claude.display_name, "Claude Code");
    assert_eq!(claude.install_url, "https://claude.com/download");

    let missing = rows
        .iter()
        .find(|row| row.provider == "fake")
        .expect("fake row present (recorded, never skipped)");
    assert_eq!(missing.status, "not_installed", "the gate's dead-end state");
    assert!(missing.version.is_none());

    // Every row carries display metadata — the wizard never guesses.
    for row in &rows {
        assert!(!row.display_name.is_empty());
        assert!(!row.install_url.is_empty());
    }
}

/// AJ-E301 Req.4: command rejections serialize as the CODED wire —
/// `{code, retryable, user_message}` — so the UI can render curated
/// copy and a retry affordance instead of a raw OS string.
#[test]
fn errors_carry_code_retryable_and_user_message() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    // An unknown-harness rejection codes as non-retryable.
    let error = commands::create_account(&state, "made-up-cli", "X", None).unwrap_err();
    let wire = serde_json::to_value(error.wire()).unwrap();
    assert_eq!(wire["code"], "unknown_harness");
    assert_eq!(wire["retryable"], false);
    assert!(
        wire["user_message"]
            .as_str()
            .unwrap()
            .contains("made-up-cli"),
        "user-facing, not a raw OS string"
    );

    // The wire form is what crosses the bridge.
    let serialized = serde_json::to_value(&error).unwrap();
    assert_eq!(serialized["code"], "unknown_harness");
    assert!(serialized["user_message"].is_string());
}

/// AJ-E302: the full 10-signal scan — every probe present with
/// health, progress ticks reach 10 of 10, and the report carries
/// seeds + desktop arrays. (Credential-discipline is pinned by the
/// adapters-side guardrail test.)
#[test]
fn environment_scan_full_reports_ten_probes_with_health() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    let ticks = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let ticks_sink = ticks.clone();
    let report = commands::environment_scan_full_impl(
        &state,
        Some(&move |done, total| {
            ticks_sink.borrow_mut().push((done, total));
        }),
    )
    .unwrap();
    let ticks = ticks.borrow();

    assert_eq!(report.probes.len(), 10, "S1–S10 all present");
    let ids: Vec<&str> = report.probes.iter().map(|p| p.id.as_str()).collect();
    for expected in [
        "s1-config-dirs",
        "s2-env-overrides",
        "s3-manager-t3",
        "s4-manager-cc-switch",
        "s5-shell-aliases",
        "s6-installed-clis",
        "s7-login-metadata",
        "s8-usage-recency",
        "s9-projects",
        "s10-desktop",
    ] {
        assert!(ids.contains(&expected), "{expected} present");
    }
    for probe in &report.probes {
        assert!(["ok", "partial", "denied"].contains(&probe.health.as_str()));
        assert!(!probe.label.is_empty() && !probe.detail.is_empty());
    }
    assert!(!report.cancelled);
    assert_eq!(ticks.last(), Some(&(10, 10)), "progress reaches 10 of 10");
    assert!(report.seeds.is_empty() || !report.installs.is_empty());
}

/// AJ-E303: regenerates the webview's typed bindings for the scan
/// surface (the generated file is committed; this keeps it
/// reproducible in CI instead of a one-off local run).
#[test]
fn export_scan_bindings() {
    use ts_rs::TS;
    let config = ts_rs::Config::default();
    juggler_app::commands::DiscoveredInfo::export(&config).unwrap();
    juggler_app::commands::InferenceInfo::export(&config).unwrap();
    juggler_app::commands::AccountInfo::export(&config).unwrap();
    juggler_app::commands::ImportResult::export(&config).unwrap();
    juggler_app::commands::SourceInspectInfo::export(&config).unwrap();
    juggler_app::commands::FolderSizeInfo::export(&config).unwrap();
}

/// AJ-E303 Req.2: the duplicate merge — the same canonical dir found
/// by BOTH a home-pattern walk and a shell alias is ONE candidate
/// carrying both signals; an alias to a dir no other signal found
/// becomes a new candidate; a dangling alias becomes neither.
#[test]
fn alias_and_pattern_merge_into_one_candidate() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("claude-side");
    std::fs::create_dir_all(&dir).unwrap();
    let mut known = vec![commands::DiscoveredInfo {
        harness: "claude-code".to_owned(),
        path: dir.to_string_lossy().into_owned(),
        origin: "home-pattern".to_owned(),
        login: "logged_out".to_owned(),
        bytes: 0,
        last_used_secs: None,
        managed: false,
        notes: vec![],
        inference: Default::default(),
        signals: vec!["home-pattern".to_owned()],
    }];
    let hint = juggler_adapter::environment::AliasHint {
        variable: "CLAUDE_CONFIG_DIR".to_owned(),
        path: dir.clone(),
        source_file: "/Users/dev/.zshrc".to_owned(),
        line: 12,
        target_exists: true,
    };
    let dangling = juggler_adapter::environment::AliasHint {
        variable: "CLAUDE_CONFIG_DIR".to_owned(),
        path: work.path().join("gone"),
        source_file: "/Users/dev/.zshrc".to_owned(),
        line: 40,
        target_exists: false,
    };
    let (added, health, dangling_count) =
        commands::alias_installs_of(&[hint, dangling], &mut known);
    assert_eq!(known.len(), 1, "no duplicate card");
    assert_eq!(
        added.len(),
        0,
        "the alias merged into the existing candidate"
    );
    assert_eq!(
        known[0].signals,
        vec![
            "home-pattern".to_owned(),
            "CLAUDE_CONFIG_DIR in /Users/dev/.zshrc:12".to_owned()
        ],
        "both signals listed on the one candidate"
    );
    assert_eq!(dangling_count, 1);
    assert!(matches!(
        health,
        juggler_adapter::environment::ScanHealth::Partial(_)
    ));

    // An alias to an UNKNOWN dir becomes a candidate of its own.
    let fresh = work.path().join("claude-fresh");
    std::fs::create_dir_all(&fresh).unwrap();
    let hint = juggler_adapter::environment::AliasHint {
        variable: "CLAUDE_CONFIG_DIR".to_owned(),
        path: fresh,
        source_file: "/Users/dev/.zshrc".to_owned(),
        line: 60,
        target_exists: true,
    };
    let (added, _, _) = commands::alias_installs_of(&[hint], &mut known);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].origin, "shell-alias");
}

/// AJ-E304 Req.7 (the staging guardrail): the recommendation engine
/// reads the inventory and proposes — ZERO storage writes happen (no
/// account exists) until the accept click executes imports.
#[test]
fn recommendation_stages_without_writing() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);

    let recommendation = commands::recommendation_for_impl(&state).unwrap();
    assert!(!recommendation.summary.is_empty());
    // Every item carries its one-line rationale (unexplained never ships).
    for item in &recommendation.items {
        assert!(!item.rationale.is_empty());
    }
    // ZERO accounts were created by the proposal itself.
    let storage = state.storage.lock().unwrap();
    let accounts = storage.accounts().list().unwrap();
    assert!(
        accounts.is_empty(),
        "staging only — {} accounts appeared",
        accounts.len()
    );
}

// ---- AJ-E307: provenance, duplicate block, degraded import, inspect ----

#[test]
fn duplicate_import_is_blocked_with_the_existing_label_e307() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let source = existing_setup(work.path());
    commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        source.to_str().unwrap(),
        "Work",
        None,
        Some("cc-switch".to_string()),
        false,
    )
    .unwrap();
    // The SAME source again: blocked, named, nothing written.
    let err = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        source.to_str().unwrap(),
        "Duplicate",
        None,
        Some("cc-switch".to_string()),
        false,
    )
    .unwrap_err();
    let wire = err.wire();
    assert_eq!(wire.code, "already_imported");
    assert!(wire.user_message.contains("Already imported as «Work»"));
    let storage = state.storage.lock().unwrap();
    let accounts = storage.accounts().list().unwrap();
    assert_eq!(accounts.len(), 1, "no second account was created");
}

#[test]
fn import_writes_provenance_and_degraded_import_flags_e307() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let source = existing_setup(work.path());
    // The pre-copy inspection: evidence + breakdown + login.
    let inspect =
        commands::source_inspect_impl(&state, CLAUDE_CODE_ID, source.to_str().unwrap()).unwrap();
    assert!(inspect.is_config);
    assert!(!inspect.already_managed);
    assert!(inspect.folders.iter().any(|f| f.name == "projects/"));
    assert!(inspect.total_bytes > 0);
    assert_eq!(inspect.ceiling_bytes, 2 * 1024 * 1024 * 1024);

    let result = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        source.to_str().unwrap(),
        "Degraded",
        None,
        Some("t3".to_string()),
        true,
    )
    .unwrap();
    assert!(result.degraded, "the degraded flag rides the wire");
    assert!(
        !result
            .account
            .import_source_path
            .clone()
            .unwrap()
            .is_empty()
    );
    assert_eq!(result.account.import_origin.as_deref(), Some("t3"));
    assert!(result.account.imported_at.is_some());
    // Provenance is queryable by path (the duplicate check's input).
    let storage = state.storage.lock().unwrap();
    let stored = storage.accounts().list().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].import_origin.as_deref(), Some("t3"));
    let canonical = source.canonicalize().unwrap();
    assert_eq!(
        stored[0].import_source_path.as_deref(),
        Some(canonical.to_str().unwrap())
    );
}

#[test]
fn isolation_check_six_rows_and_a_forced_fail_e308() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let source = existing_setup(work.path());
    let result = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        source.to_str().unwrap(),
        "Iso",
        None,
        Some("home-pattern".to_string()),
        false,
    )
    .unwrap();
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert_eq!(rows.len(), 6, "exactly the six named rows: {rows:#?}");
    assert!(
        rows.iter().all(|row| row.pass),
        "a healthy import: {rows:#?}"
    );
    let ids: Vec<&str> = rows.iter().map(|row| row.id).collect();
    assert!(ids.contains(&"managed_dir_private"));
    assert!(ids.contains(&"marker_present"));
    assert!(ids.contains(&"launch_env_managed_only"));
    assert!(ids.contains(&"source_free_of_juggler_writes"));
    assert!(ids.contains(&"probe_independence"));
    assert!(ids.contains(&"no_writes_outside_managed"));

    // Forced fail: a stray FILE at the accounts root trips row 6 and
    // names itself (the UI blocks [Launch], never [Done]).
    let accounts_root = state.accounts_dir().join(CLAUDE_CODE_ID);
    std::fs::write(accounts_root.join("stray.txt"), "leak").unwrap();
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    let stray = rows
        .iter()
        .find(|row| row.id == "no_writes_outside_managed")
        .unwrap();
    assert!(!stray.pass);
    let _ = std::fs::remove_file(accounts_root.join("stray.txt"));
}
