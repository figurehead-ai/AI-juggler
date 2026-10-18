//! AJ-E309 integration: the first-run policy hook stays honest (a
//! fresh install has NO pack policy — nothing backfills one, and
//! "Decide later" therefore stages no pack), the onboarding funnel is
//! a schema-guarded LOCAL store with a deterministic export, and the
//! Expert-setup surface (folder, reveal, shell line, env overrides)
//! never lets an override break Account isolation or store a
//! credential.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::{AdapterRegistry, HarnessAdapter};
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn fields(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn a_fresh_install_has_no_policy_and_onboarding_backfills_none() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    // The onboarding writes (accounts, Launch settings, funnel
    // events) happen; the "Decide later" path writes NO policy.
    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap();
    commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
    commands::funnel_event_impl(
        &state,
        "policy_chosen",
        fields(&[("choice", "decide_later")]),
        1_000,
    )
    .unwrap();
    let info = commands::redaction_policy_get_impl(&state, CLAUDE_CODE_ID).unwrap();
    assert_eq!(info.global, None, "no global is ever backfilled");
    assert_eq!(info.effective, None, "absence of a choice = ask first");
    assert_eq!(info.source, "unset");
    // Unset blocks pack staging at the backend — pinned with a real
    // epoch in switch_graph_targets.rs (`pack_export_impl` refuses
    // with "Choose how secrets are handled in packs first").

    // An explicit choice (the accept click) writes the global once;
    // the restart keeps it.
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    drop(state);
    let reopened = app_state(work.path());
    let info = commands::redaction_policy_get_impl(&reopened, CLAUDE_CODE_ID).unwrap();
    assert_eq!(info.effective.as_deref(), Some("strip"));
    assert_eq!(info.source, "global");
}

#[test]
fn funnel_events_validate_stamp_install_and_dedupe_first_launch() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());

    // Forbidden / unknown / free-text input is refused with the coded
    // `validation` wire (not retryable) and writes nothing.
    for (kind, bad) in [
        ("import_failed", fields(&[("path", "/Users/you/.claude")])),
        ("import_failed", fields(&[("reason_code", "Work Max")])),
        ("scan_completed", fields(&[("label", "work")])),
        ("made_up", fields(&[])),
    ] {
        let error = commands::funnel_event_impl(&state, kind, bad, 5).unwrap_err();
        let wire = error.wire();
        assert_eq!(wire.code, "validation", "{kind}");
        assert!(!wire.retryable);
    }
    assert!(
        !state.data_dir.join("funnel-events.jsonl").exists(),
        "a refused event writes nothing"
    );

    // The first accepted event stamps the install moment.
    commands::funnel_event_impl(
        &state,
        "flow_step",
        fields(&[("flow", "a-first-install"), ("step", "gate")]),
        10_000,
    )
    .unwrap();
    // first_launch: the UI sends NO fields; the backend computes the
    // seconds and records it ONCE per install.
    commands::funnel_event_impl(&state, "first_launch", fields(&[]), 10_000 + 212_000).unwrap();
    commands::funnel_event_impl(&state, "first_launch", fields(&[]), 10_000 + 999_000).unwrap();
    let report = commands::funnel_export_impl(&state).unwrap();
    assert!(report.contains("| first_launch | 1 |"), "{report}");
    assert!(report.contains("- 212 s after install"), "{report}");

    // The raw store never carries a path, label, or credential.
    let raw = std::fs::read_to_string(state.data_dir.join("funnel-events.jsonl")).unwrap();
    for needle in ["/Users", "claude", "Work", "@", "sk-"] {
        assert!(!raw.contains(needle), "{needle} leaked into the store");
    }
}

/// One scripted funnel event: kind, fields, offset (ms) from install.
type Step = (&'static str, Vec<(&'static str, &'static str)>, i64);

/// The dry-run fixture: a realistic first install (flow (a) with two
/// imports, one queued sign-in skipped, one edited inference, a
/// "Strip" choice, then the first launch) emitted through the command
/// layer. The export is compared with the committed dry-run file —
/// `AIJ_BLESS=1` rewrites it after a reviewed change.
#[test]
fn funnel_dry_run_export_matches_the_stored_record() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let t0 = 1_760_000_000_000_i64;
    let script: Vec<Step> = vec![
        (
            "flow_step",
            vec![("flow", "a-first-install"), ("step", "gate")],
            0,
        ),
        ("scan_started", vec![("providers", "1")], 2_000),
        (
            "flow_step",
            vec![("flow", "a-first-install"), ("step", "scan")],
            2_000,
        ),
        (
            "scan_completed",
            vec![("providers", "1"), ("found", "3"), ("health", "ok")],
            3_200,
        ),
        ("inference_shown", vec![("field", "purpose")], 3_300),
        ("inference_shown", vec![("field", "purpose")], 3_300),
        ("inference_shown", vec![("field", "purpose")], 3_300),
        ("proposal_shown", vec![("items", "2")], 3_400),
        ("proposal_customised", vec![], 9_000),
        ("inference_edit", vec![("field", "purpose")], 15_000),
        ("policy_chosen", vec![("choice", "strip")], 21_000),
        (
            "flow_step",
            vec![("flow", "a-first-install"), ("step", "confirm")],
            22_000,
        ),
        ("proposal_accepted", vec![("items", "2")], 22_000),
        ("import_started", vec![], 22_100),
        ("import_completed", vec![("degraded", "false")], 24_900),
        ("import_started", vec![], 25_000),
        (
            "import_failed",
            vec![("reason_code", "already_imported")],
            25_400,
        ),
        ("login_queued", vec![("count", "1")], 25_500),
        (
            "flow_step",
            vec![("flow", "a-first-install"), ("step", "signins")],
            25_500,
        ),
        ("login_skipped", vec![], 31_000),
        (
            "flow_step",
            vec![("flow", "a-first-install"), ("step", "summary")],
            31_000,
        ),
        ("expert_opened", vec![("surface", "wizard")], 33_000),
        ("first_launch", vec![], 48_000),
    ];
    for (kind, pairs, offset) in script {
        commands::funnel_event_impl(&state, kind, fields(&pairs), t0 + offset).unwrap();
    }
    let report = commands::funnel_export_impl(&state).unwrap();
    assert!(report.contains("- purpose: 1/3 overridden (33%) — FLAGGED for retuning"));
    assert!(report.contains("- 48 s after install"));

    let stored = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/qa/e3/AJ-E309/funnel-dry-run.md");
    let header = "<!-- Generated by crates/juggler-app/tests/onboarding_policy_metrics.rs \
                  (funnel_dry_run_export_matches_the_stored_record) — a FIXTURE run through \
                  the real command layer, not live user data. Regenerate with AIJ_BLESS=1. -->\n\n";
    let expected = format!("{header}{report}");
    if std::env::var_os("AIJ_BLESS").is_some() {
        std::fs::write(&stored, &expected).unwrap();
    }
    let committed = std::fs::read_to_string(&stored).expect("the dry-run record is committed");
    assert_eq!(
        committed, expected,
        "the funnel export changed — review, then rerun with AIJ_BLESS=1"
    );
}

#[test]
fn expert_setup_reports_managed_paths_and_a_quoted_shell_line() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());

    // Before any Account: the Provider's folder, no shell line yet.
    let before = commands::expert_setup_info_impl(&state, CLAUDE_CODE_ID, None, None).unwrap();
    assert_eq!(before.account_id, None);
    assert!(
        before
            .state_dir
            .starts_with(state.data_dir.to_str().unwrap())
    );
    assert!(before.shell_snippet.is_none());
    assert_eq!(
        before.protected_env_keys,
        vec!["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"]
    );
    // Reveal never points outside the data dir and never creates.
    let target = commands::expert_reveal_target(&state, CLAUDE_CODE_ID, None).unwrap();
    assert!(target.starts_with(&state.data_dir));
    assert!(target.is_dir());
    assert!(commands::expert_setup_info_impl(&state, "nope", None, None).is_err());

    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap();
    let recipe = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
    commands::launch_recipe_env_set_impl(
        &state,
        &recipe.id,
        fields(&[
            ("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8000"),
            ("NOTE", "it's fine"),
        ]),
    )
    .unwrap();
    let info = commands::expert_setup_info_impl(
        &state,
        CLAUDE_CODE_ID,
        Some(&account.id),
        Some(&recipe.id),
    )
    .unwrap();
    assert!(info.exists);
    let dir = info.state_dir.clone();
    assert!(dir.ends_with(&account.id));
    assert_eq!(
        info.shell_snippet.as_deref(),
        Some(
            format!(
                "CLAUDE_CONFIG_DIR='{dir}' CLAUDE_CODE_MAX_OUTPUT_TOKENS='8000' NOTE='it'\\''s fine' claude"
            )
            .as_str()
        ),
        "isolation first, overrides quoted, the vendor command last"
    );
    assert_eq!(
        commands::expert_reveal_target(&state, CLAUDE_CODE_ID, Some(&account.id)).unwrap(),
        PathBuf::from(&dir)
    );

    // Launch settings of ANOTHER account are refused.
    let other = commands::create_account(&state, CLAUDE_CODE_ID, "Side", None).unwrap();
    let foreign = commands::create_profile(&state, &other.id, "Main", None, None).unwrap();
    let error = commands::expert_setup_info_impl(
        &state,
        CLAUDE_CODE_ID,
        Some(&account.id),
        Some(&foreign.id),
    )
    .unwrap_err();
    assert_eq!(error.wire().code, "validation");
}

#[test]
fn env_overrides_can_never_break_isolation_or_store_a_credential() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap();
    let recipe = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();

    for (key, value, why) in [
        (
            "CLAUDE_CONFIG_DIR",
            "/tmp/elsewhere",
            "managed by AI Juggler",
        ),
        ("AI_JUGGLER_CLAUDE_BIN", "/bin/sh", "managed by AI Juggler"),
        ("ANTHROPIC_API_KEY", "sk-ant-x", "looks like a credential"),
        ("CLAUDE_CODE_OAUTH_TOKEN", "x", "looks like a credential"),
        ("ANTHROPIC_AUTH_TOKEN", "x", "looks like a credential"),
        ("MY_SECRET", "x", "looks like a credential"),
        ("lower_case", "x", "valid variable name"),
        ("1LEADING", "x", "valid variable name"),
        ("MULTI", "a\nb", "single line"),
    ] {
        let error =
            commands::launch_recipe_env_set_impl(&state, &recipe.id, fields(&[(key, value)]))
                .unwrap_err();
        assert!(error.to_string().contains(why), "{key}: {error}");
        assert_eq!(error.wire().code, "validation");
    }
    let too_many: BTreeMap<String, String> = (0..33)
        .map(|i| (format!("VAR_{i}"), "1".to_owned()))
        .collect();
    assert!(commands::launch_recipe_env_set_impl(&state, &recipe.id, too_many).is_err());

    // Nothing was written by any refusal; a valid set round-trips.
    let saved = commands::launch_recipe_env_set_impl(
        &state,
        &recipe.id,
        fields(&[("ANTHROPIC_BASE_URL", "http://127.0.0.1:8080")]),
    )
    .unwrap();
    assert_eq!(saved.len(), 1);

    // The adapter backstop: even an overlay that slipped past the
    // editor can never repoint the config dir at launch.
    let adapter = ClaudeCodeAdapter::with_bin(claude_stub(work.path()));
    let stored = {
        let storage = state.storage.lock().unwrap();
        let mut profile = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(recipe.id.clone()))
            .unwrap();
        profile
            .settings_overlay
            .env
            .insert("CLAUDE_CONFIG_DIR".into(), "/tmp/elsewhere".into());
        profile
    };
    let account_row = commands::find_account_arc(&state, &account.id).unwrap();
    let spec = adapter
        .launch_spec(
            &account_row,
            &stored,
            work.path(),
            &juggler_adapter::LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(
        spec.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(account_row.state_dir.to_str().unwrap()),
        "isolation wins over any overlay"
    );
    assert_eq!(
        spec.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
        Some("http://127.0.0.1:8080")
    );
}
