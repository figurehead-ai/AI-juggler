//! AJ-E1603 integration: Kimi Code is a registered Provider — listed, an
//! Account gets its own home and passes the isolation checklist, its
//! limits wait for `kimi web` until it reports them and then show the
//! windows it named, labelled experimental (R3, D-106), its sessions are listed and
//! become projects, its default home is used where it is unwritten, and a
//! membership account that crashes is never started again by itself
//! (its plans are for interactive use only). Synthetic files only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/kimi_fixture/mod.rs"]
mod kimi_fixture;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{KIMI_CODE_ID, KimiCodeAdapter};
use juggler_app::features::sessions::restart::{self, Ending};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

/// A `kimi` that prints its version, and crashes when run otherwise.
fn crashing_kimi(dir: &Path) -> PathBuf {
    let path = dir.join("kimi");
    stand_in::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 2.3.0; exit 0; fi\necho 'panicked' >&2\nexit 3\n",
    );
    path
}

/// The app with Kimi Code's home in the sandbox.
fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        KimiCodeAdapter::with_bin(crashing_kimi(dir)).with_home(dir.join("home")),
    ));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let meta = entry.metadata().unwrap();
            out.push((entry.path(), meta.len(), meta.modified().unwrap()));
            if meta.is_dir() {
                walk(&entry.path(), out);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

#[test]
fn kimi_code_is_a_listed_provider() {
    let providers = tauri::async_runtime::block_on(
        juggler_app::features::harnesses::commands::harnesses_list(),
    );
    let kimi = providers
        .iter()
        .find(|p| p.id == KIMI_CODE_ID)
        .expect("listed");
    assert_eq!(kimi.name, "Kimi Code");
}

#[test]
fn a_kimi_account_has_its_own_home_and_its_limits_wait_for_kimi_web() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    assert_eq!(created.login_state, "logged_out");
    let rows = juggler_app::features::isolation::isolation_check_impl(&state, &created.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    let expert = juggler_app::features::expert::expert_setup_info_impl(
        &state,
        KIMI_CODE_ID,
        Some(&created.id),
        None,
    )
    .unwrap();
    let snippet = expert.shell_snippet.unwrap();
    assert!(
        snippet.contains(&format!("KIMI_CODE_HOME='{}'", dir.display())),
        "{snippet}"
    );
    assert!(snippet.ends_with(" kimi"), "{snippet}");

    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == created.id)
        .unwrap();
    let display =
        juggler_app::features::quota::quota_display_inner(&storage, &account, 1_792_580_400_000);
    assert_eq!(display.unavailable_reason, None);
    assert!(display.five_hour.is_none() && display.weekly.is_none());
    assert_eq!(
        display.five_hour_omitted_reason.as_deref(),
        Some("Kimi Code hasn't reported its limits yet — they show after you use Kimi Code")
    );
}

/// A stand-in for the user's own `kimi web` on 127.0.0.1: answers healthz
/// and the usage, and returns the requests it got.
fn kimi_web(usage: &'static str) -> (u16, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = Vec::new();
            let mut buffer = [0_u8; 2048];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..read]);
            }
            let request = String::from_utf8(raw).unwrap();
            let body = if request.starts_with("GET /api/v1/healthz ") {
                r#"{"code":0,"data":{"ok":true}}"#
            } else {
                usage
            };
            let answer = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(answer.as_bytes()).unwrap();
            seen.push(request);
        }
        seen
    });
    (port, handle)
}

#[test]
fn measure_now_reads_the_windows_from_kimi_web_labelled_experimental() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    // A membership sign-in, and the user's own `kimi web` running for it.
    kimi_fixture::sign_in_membership(&dir, "made-up-sign-in");
    let (port, server) = kimi_web(
        r#"{"code":0,"data":{"kind":"ok","quota":{"usages":{"limit5h":{"usedRatio":0.42,"resetAt":"2030-01-01T00:00:00Z"},"monthTotal":{"usedRatio":0.2},"monthCode":{"usedRatio":0.1}},"extraUsage":null}}}"#,
    );
    let now = juggler_core::model::Timestamp::now().millis();
    kimi_fixture::write(
        &dir.join("server/instances/01TEST.json"),
        &format!(
            r#"{{"server_id":"01TEST","pid":{},"host":"127.0.0.1","port":{port},"started_at":1,"heartbeat_at":{now}}}"#,
            std::process::id()
        ),
    );
    kimi_fixture::write(&dir.join("server.token"), "made-up-server-token\n");

    let status = juggler_app::features::quota::probe::run_now_impl(&state, &created.id).unwrap();
    assert!(status.offered);
    assert_eq!(status.last_outcome.as_deref(), Some("ran"));
    let seen = server.join().unwrap();
    assert!(seen[1].contains("Authorization: Bearer made-up-server-token\r\n"));

    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == created.id)
        .unwrap();
    let display =
        juggler_app::features::quota::quota_display_inner(&storage, &account, now + 1_000);
    let five = display.five_hour.expect("the 5-hour window it named");
    assert_eq!(five.percentage.map(f64::round), Some(42.0));
    assert_eq!(five.source.as_deref(), Some("local_server"));
    assert_eq!(
        five.source_words.as_deref(),
        Some("Kimi Code's own local server (experimental)")
    );
    // The week it left out isn't there, and isn't guessed.
    assert!(display.weekly.is_none());
    assert_eq!(
        display.weekly_omitted_reason.as_deref(),
        Some("Kimi Code reports no weekly limit for this account")
    );
    // The month is kept as its own window.
    let month = storage
        .usage()
        .samples_since(&created.id, juggler_core::quota::WindowKind::Monthly, 0)
        .unwrap();
    assert_eq!(month.len(), 1);
    assert!((month[0].used_pct - 20.0).abs() < 1e-9);
}

#[test]
fn an_api_key_account_is_never_asked() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    kimi_fixture::sign_in_api_key(&dir);
    let status = juggler_app::features::quota::probe::status_impl(&state, &created.id).unwrap();
    assert!(!status.offered);
    assert!(juggler_app::features::quota::probe::run_now_impl(&state, &created.id).is_err());
}

#[test]
fn its_sessions_are_listed_and_their_folders_become_projects() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    let project = work
        .path()
        .join("projects/shop")
        .canonicalize()
        .unwrap_or_else(|_| {
            std::fs::create_dir_all(work.path().join("projects/shop")).unwrap();
            work.path().join("projects/shop").canonicalize().unwrap()
        });
    kimi_fixture::add_session(&dir, "ses-1", &project, "Fix the cart");

    assert!(
        juggler_app::features::projects::backfill::projects_backfill_impl(&state).unwrap() >= 1
    );
    let projects = juggler_app::features::projects::list::projects_list_impl(&state).unwrap();
    let shop = projects
        .iter()
        .find(|p| Path::new(&p.root) == project)
        .expect("the session's folder is a project");
    assert!(shop.accounts_used.contains(&"Kimi".to_owned()));
    let listed = state.conversations.all().unwrap();
    let conversation = listed
        .iter()
        .find(|c| c.summary.id == "ses-1")
        .expect("the session is listed");
    assert_eq!(conversation.tool, KIMI_CODE_ID);
    assert_eq!(conversation.summary.title.as_deref(), Some("Fix the cart"));
    let line =
        juggler_app::features::conversations::resume_command_impl(&state, &created.id, "ses-1")
            .unwrap();
    assert!(line.ends_with("kimi '--session' 'ses-1'"), "{line}");
}

#[test]
fn the_default_home_is_used_where_it_is_and_never_written() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let source = work.path().join("home/.kimi-code");
    kimi_fixture::settings(&source, "made-up");
    kimi_fixture::sign_in_membership(&source, "made-up");
    kimi_fixture::add_session(&source, "ses-1", Path::new("/w/app"), "old");
    let before = snapshot(&source);
    let result = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        KIMI_CODE_ID,
        source.to_str().unwrap(),
        "Kimi",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            origin: Some("vendor-default".into()),
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    assert_eq!(result.account.login_state, "signed_in");
    juggler_app::usage_pipeline::ingest_pass(&state, 0);
    let _ = juggler_app::features::projects::backfill::projects_backfill_impl(&state);
    let rows =
        juggler_app::features::isolation::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    assert_eq!(snapshot(&source), before, "nothing in the home changed");
}

/// Waits for `session`'s exit and stores it as the forwarder would.
fn crashed(state: &Arc<AppState>, session: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "{session} never exited");
        if let Some(event) = state.supervisor.recv_timeout(Duration::from_millis(200)) {
            let exited = matches!(&event, juggler_supervisor::LifecycleEvent::Exited { session: id, .. } if id.as_str() == session);
            let storage = state.storage.lock().unwrap();
            juggler_app::features::sessions::attention::apply_lifecycle_event(
                &storage, event, false, None,
            );
            if exited {
                break;
            }
        }
    }
    restart::after_exit(state, &SessionId::new(session), Ending::Crashed);
}

#[test]
fn a_membership_account_that_crashes_is_never_started_again_by_itself() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    kimi_fixture::sign_in_membership(&dir, "made-up");
    let profile = juggler_app::features::profile::create_profile(
        &state,
        &created.id,
        "Daily",
        None,
        Some(work.path().to_string_lossy().into_owned()),
    )
    .unwrap()
    .id;
    let prefs = juggler_app::settings_policy::SessionPrefsInfo {
        restart_on_crash: true,
        ..juggler_app::settings_policy::session_prefs_get_impl(&state).unwrap()
    };
    juggler_app::settings_policy::session_prefs_set_impl(&state, &prefs).unwrap();

    let first = juggler_app::features::launch::launch_profile(&state, &profile, None).unwrap();
    crashed(&state, &first.session_id);
    std::thread::sleep(Duration::from_secs(4));
    let storage = state.storage.lock().unwrap();
    assert_eq!(
        storage.sessions().list(None).unwrap().len(),
        1,
        "nothing was started again"
    );
    let history: Vec<String> = storage
        .sessions()
        .lifecycle_events(&SessionId::new(first.session_id.as_str()))
        .unwrap()
        .into_iter()
        .map(|(event, _)| event)
        .collect();
    assert!(
        history
            .iter()
            .any(|event| event.contains("not started again after a crash")
                && event.contains("interactive use only")),
        "{history:?}"
    );
}
