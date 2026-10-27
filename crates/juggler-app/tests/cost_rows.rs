//! 0.1.8: Usage › Prices and details showed raw session ids, one
//! near-duplicate row per model, and no total once any model had no list
//! price. A row is now one session's day, named as the rail names it
//! (the user's label, else the conversation's title, with its project),
//! its models folded together with each one's part alongside; the total
//! sums every priced model and names the ones without a price.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::conversations::commands::list_impl;
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    AccountId, Confidence, ConversationId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp, TokenCounts,
};
use juggler_core::quota::{UsageEvent, UsageSource};

/// 2026-10-21T00:00:00Z.
const OCT_21: i64 = 1_792_540_800_000;
const DAY: i64 = 86_400_000;

fn transcript(config: &Path, cwd: &Path, id: &str, ask: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"gitBranch\":\"main\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{ask}\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

fn event(
    account: &str,
    session: &str,
    ts: i64,
    model: &str,
    input: u64,
    output: u64,
) -> UsageEvent {
    UsageEvent {
        account: account.to_owned(),
        session: Some(session.to_owned()),
        ts,
        model: Some(model.to_owned()),
        tokens: TokenCounts {
            input,
            output,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: Some(format!("{session}-{ts}-{model}")),
    }
}

#[test]
fn a_row_is_a_named_session_day_and_the_total_counts_every_priced_model() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1", "Fix the login test");

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    commands::projects_backfill_impl(&state).unwrap();
    // Usage reads the conversations first; the cost table names from them.
    list_impl(&state).unwrap();

    let noon = OCT_21 + DAY / 2;
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_events(&[
            // c-1 on the 21st: a priced model and one with no list price.
            event(&account, "c-1", noon, "claude-opus-4-6", 1000, 1000),
            event(&account, "c-1", noon + 1, "codex-auto-review", 500, 0),
            // c-1 the day before, and a session no conversation knows.
            event(&account, "c-1", noon - DAY, "claude-opus-4-6", 2000, 0),
            event(&account, "c-unknown", noon, "claude-opus-4-6", 1000, 0),
        ])
        .unwrap();

    let first = commands::cost_rows_page_impl(
        &state,
        None,
        &PageRequest {
            cursor: None,
            limit: Some(2),
        },
    )
    .unwrap();
    assert_eq!(first.total, Some(3), "one row per session and day");
    assert_eq!(first.items.len(), 2);
    let row = &first.items[0];
    assert_eq!(row.session, "c-1");
    assert_eq!(row.day, "2026-10-21");
    assert_eq!(
        row.title.as_deref(),
        Some("Fix the login test"),
        "its title, never its id"
    );
    assert_eq!(row.project.as_deref(), Some("app"));
    // $5 in and $25 out per million: 1000 of each is $0.03. The model with
    // no list price adds nothing and is never $0.
    assert!((row.usd.unwrap() - 0.03).abs() < 1e-9, "{:?}", row.usd);
    assert_eq!(row.tokens.input, 1500);
    assert_eq!(row.models.len(), 2);
    assert_eq!(
        row.models[0].model.as_deref(),
        Some("claude-opus-4-6"),
        "most tokens first"
    );
    assert_eq!(row.models[1].model.as_deref(), Some("codex-auto-review"));
    assert_eq!(row.models[1].usd.usd, None);
    let unknown = &first.items[1];
    assert_eq!(unknown.session, "c-unknown");
    assert_eq!(
        (unknown.title.as_deref(), unknown.project.as_deref()),
        (None, None)
    );

    let second = commands::cost_rows_page_impl(
        &state,
        None,
        &PageRequest {
            cursor: first.next_cursor.clone(),
            limit: Some(2),
        },
    )
    .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].day, "2026-10-20", "newest day first");
    assert!(second.next_cursor.is_none());

    // The total sums every priced model on every day (it was the first
    // model's first day only) and names what it leaves out.
    let rollups = commands::cost_rollups_impl(&state, None);
    assert!(
        (rollups.total_usd.unwrap() - 0.045).abs() < 1e-9,
        "{:?}",
        rollups.total_usd
    );
    assert_eq!(
        rollups.unpriced_models,
        vec![Some("codex-auto-review".to_owned())]
    );
    assert_eq!(rollups.subscription.api_equivalent_usd, rollups.total_usd);

    // The user's own label for the session wins, as on the rail.
    {
        let storage = state.storage.lock().unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-work"),
                name: "Main".into(),
                account: AccountId::new(account.as_str()),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new("s-1"),
                conversation: Some(ConversationId::new("c-1")),
                profile: ProfileId::new("rec-work"),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: repo.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::from_millis(noon),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        storage
            .sessions()
            .set_label(&SessionId::new("s-1"), Some("Login fix"))
            .unwrap();
    }
    let relabelled = commands::cost_rows_page_impl(&state, None, &PageRequest::default()).unwrap();
    assert_eq!(relabelled.items[0].title.as_deref(), Some("Login fix"));
}
