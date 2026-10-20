//! AJ-E1106: the usage-statistics guardrails that need the whole app
//! (the rest live beside the code they guard; the AJ-E1106 manifest
//! maps every promise to its test).
//!
//! - Nothing the user owns reaches a report — or even the stats store:
//!   a week seeded with an Account label, a project under `$HOME`, a
//!   private model name, a secret-shaped error carrying the home path,
//!   and the user name, driven through the real counting paths, then
//!   closed and assembled; every byte of the stats folder is checked.
//! - Default silence: with no choice made, counting, the interface
//!   path and a tick create nothing and make no request.
//! - Stats code names no vendor credential or identity file.

#![cfg(feature = "stats")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::path::Path;
use std::sync::Arc;

use juggler_account::{RecordingTransport, Transport};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::usage_stats;
use juggler_core::model::{Account, AccountId, HarnessId, LaunchRecipe, ProfileId};

fn state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn all_bytes(dir: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.extend(bytes);
            }
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

#[test]
fn nothing_the_user_owns_reaches_the_stats_store_or_a_report() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("fake-home-x7k3");
    let state_dir = home.join(".claude-acme");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join(".credentials.json"),
        r#"{"oauth":"sk-ant-oat01-never-read"}"#,
    )
    .unwrap();
    let state = state(dir.path());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-acme"),
                harness: HarnessId::new("claude-code"),
                label: "Acme Legal Team".into(),
                plan: None,
                state_dir: state_dir.clone(),
                cleanup_opt_in: false,
                renewal_date: Some("2026-11-02".into()),
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-acme"),
                name: "Acme main".into(),
                account: AccountId::new("acc-acme"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .projects()
            .touch(&home.join("clients/acme-merger"), None)
            .unwrap();
    }
    usage_stats::usage_stats_set_impl(&state, "share").unwrap();
    let user = std::env::var("USER").unwrap_or_else(|_| "someone".into());
    let secret = "sk-ant-api03-ZXhhbXBsZS1zZWNyZXQ";
    let home_text = home.display().to_string();

    // The real paths, fed what a careless site might pass.
    usage_stats::count(
        "instance.running_model",
        &[
            ("provider", usage_stats::provider("claude-code")),
            (
                "model_family",
                usage_stats::model_family("acme-private-model-x9"),
            ),
        ],
    );
    usage_stats::count(
        "leftover.detected",
        &[(
            "category",
            usage_stats::leftover_category(
                &format!("{home_text}/bin/acme-agent"),
                &format!("--token {secret}"),
            ),
        )],
    );
    // An error crossing to the interface carries a secret and the home
    // path in its message; only its class may be counted.
    let error = juggler_app::error::AppError::Io {
        context: "hand over",
        message: format!("{secret} failed at {home_text}/clients/acme-merger for {user}"),
    };
    let _ = serde_json::to_string(&error).unwrap();
    // The interface tries values outside the schema.
    let dims = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    usage_stats::usage_stats_ui_event_impl(
        "ui.screen_opened",
        &dims(&[("screen", &home_text)]),
        None,
    );
    usage_stats::usage_stats_ui_event_impl(
        "onboarding.login",
        &dims(&[("provider", "Acme Legal Team"), ("outcome", "completed")]),
        None,
    );
    usage_stats::usage_stats_ui_gauge_impl("ui.theme", "Acme Legal Team");

    // Close this week and assemble it (gauges read the Account, the
    // recipe and the project).
    let now = juggler_core::model::Timestamp::now().millis();
    let week = juggler_stats::week::label(now);
    let closed = juggler_stats::week::end_ms(&week).unwrap();
    let recorder = RecordingTransport::default();
    let transport = usage_stats::enabled::StatsTransport(Arc::new(recorder) as Arc<dyn Transport>);
    let out = usage_stats::enabled::tick(
        &state,
        &juggler_stats::Environment::default(),
        &usage_stats::enabled::envelope(),
        &transport,
        closed,
    )
    .unwrap();
    assert_eq!(out.assembled.len(), 1, "{out:?}");

    let stats_dir = dir.path().join("app-data/stats");
    let report = std::fs::read_to_string(stats_dir.join(format!("skipped/{week}.json")))
        .or_else(|_| std::fs::read_to_string(stats_dir.join(format!("pending/{week}.json"))))
        .unwrap();
    assert!(report.contains(r#""m":"accounts.configured""#), "{report}");
    assert!(report.contains(r#""m":"error.class""#), "{report}");
    let everything = all_bytes(&stats_dir);
    let mut forbidden = vec![
        "Acme Legal Team",
        "Acme main",
        "acme-merger",
        "clients",
        "fake-home-x7k3",
        ".claude-acme",
        "acme-private-model",
        "acme-agent",
        "sk-ant",
        "never-read",
        "2026-11-02",
    ];
    if user.len() >= 3 {
        forbidden.push(&user);
    }
    for needle in forbidden {
        assert!(
            !contains(&everything, needle),
            "{needle:?} reached the stats folder"
        );
    }
    usage_stats::delete_local(&state).unwrap();
}

#[test]
fn without_a_choice_nothing_is_created_and_nothing_is_sent() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    usage_stats::count("app.launch", &[]);
    usage_stats::usage_stats_ui_event_impl(
        "ui.screen_opened",
        &[("screen".to_owned(), "fleet".to_owned())]
            .into_iter()
            .collect(),
        None,
    );
    let recorder = Arc::new(RecordingTransport::default());
    let transport = usage_stats::enabled::StatsTransport(recorder.clone() as Arc<dyn Transport>);
    let now = juggler_core::model::Timestamp::now().millis();
    usage_stats::enabled::tick(
        &state,
        &juggler_stats::Environment::default(),
        &usage_stats::enabled::envelope(),
        &transport,
        now,
    )
    .unwrap();
    let overview = usage_stats::usage_stats_overview_impl(&state).unwrap();
    assert!(overview.rows.is_empty() && overview.reports.is_empty());
    assert!(!dir.path().join("app-data/stats").exists());
    assert_eq!(recorder.count(), 0);
}

#[test]
fn stats_code_names_no_vendor_credential_or_identity_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("src/usage_stats.rs")];
    for entry in std::fs::read_dir(root.join("../juggler-stats/src"))
        .unwrap()
        .flatten()
    {
        files.push(entry.path());
    }
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for marker in [
            ".credentials",
            "auth.json",
            "/.claude",
            "/.codex",
            "/.zcode",
            "find-generic-password",
            "oauth",
            "id_token",
        ] {
            assert!(
                !text.contains(marker),
                "{} mentions {marker:?}",
                file.display()
            );
        }
    }
}
