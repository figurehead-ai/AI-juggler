//! AJ-E1703: context cards — honest fields only (D-06).
//!
//! - A card says only what AI Juggler knows: name, place, tool, state and
//!   last activity from the read model; the files changed and the step in
//!   progress from the session's own transcript, read on the tick.
//! - What it doesn't know says "Unavailable" and why: before the tick has
//!   read a transcript, for a session with none, and for tasks (E19).
//! - What the agent says is its own words, never a fact AI Juggler made.
//! - A card's place is the rows' place line (project · app · profile,
//!   D-36); it names its account by the profile's label only, never an
//!   email or a plan (D-33; it is what agents will be shown).
//! - The tick reads only what was appended; a session that ended keeps
//!   what was read while it ran.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::error::AppError;
use juggler_app::features::pools::{self, ContextCard, cards};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectId, SessionId, SessionStatus, Timestamp, TranscriptRef,
};
use juggler_core::storage::NO_PROJECT_POOL_ID;

const ACCOUNT_LABEL: &str = "Work Max";
const PLAN: &str = "Max 20x plan";

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn account(state: &Arc<AppState>) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-work"),
            harness: HarnessId::new("fake"),
            label: ACCOUNT_LABEL.into(),
            plan: Some(juggler_core::model::PlanTier::new(PLAN)),
            state_dir: state.accounts_dir().join("acc-work"),
            cleanup_opt_in: false,
            renewal_date: None,
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
            id: ProfileId::new("rec-work"),
            name: "Main".into(),
            account: AccountId::new("acc-work"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

fn project(state: &Arc<AppState>, root: &Path) -> ProjectId {
    std::fs::create_dir_all(root).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(root, Some("shop"))
        .unwrap()
        .id
}

fn session(
    state: &Arc<AppState>,
    id: &str,
    project: &ProjectId,
    cwd: &Path,
    transcript: Option<&Path>,
    status: SessionStatus,
) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.clone()),
            cwd: cwd.to_path_buf(),
            status,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: transcript.map(|path| TranscriptRef {
                path: path.to_path_buf(),
                native_id: None,
            }),
            window: None,
        })
        .unwrap();
    session
}

fn call(name: &str, input: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"t\",\"name\":\"{name}\",\"input\":{input}}}]}}}}\n"
    )
}

fn card<'a>(cards: &'a [ContextCard], id: &str) -> &'a ContextCard {
    cards
        .iter()
        .find(|card| card.session_id == id)
        .unwrap_or_else(|| panic!("no card for {id}"))
}

/// A planner with a transcript, a builder without one, in one pool.
fn checkout(dir: &Path) -> (Arc<AppState>, String, std::path::PathBuf) {
    let state = app_state(dir);
    account(&state);
    let root = dir.join("shop");
    let shop = project(&state, &root);
    let transcript = dir.join("planner.jsonl");
    std::fs::write(
        &transcript,
        call("Read", r#"{"file_path":"/w/shop/README.md"}"#),
    )
    .unwrap();
    let planner = session(
        &state,
        "s-planner",
        &shop,
        &root,
        Some(&transcript),
        SessionStatus::Running,
    );
    let builder = session(
        &state,
        "s-builder",
        &shop,
        &root,
        None,
        SessionStatus::Running,
    );
    let info = pools::pool_create_impl(&state, "Checkout redesign", None).unwrap();
    let pool = info.pools[0].id.clone();
    pools::pool_move_impl(&state, planner.as_str(), &pool).unwrap();
    pools::pool_move_impl(&state, builder.as_str(), &pool).unwrap();
    pools::pool_set_role_impl(&state, planner.as_str(), "lead").unwrap();
    (state, pool, transcript)
}

#[test]
fn a_card_says_what_ai_juggler_knows_and_unavailable_for_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool, _) = checkout(dir.path());

    let result = cards::cards_impl(&state, &pool).unwrap();
    assert_eq!(result.pool_id, pool);
    assert_eq!(result.cards.len(), 2);
    assert_eq!(result.cards[0].session_id, "s-planner", "the lead first");
    let planner = card(&result.cards, "s-planner");
    assert_eq!(planner.role, "lead");
    // D-36: the rows' place line, project · app · profile.
    assert_eq!(
        planner.place,
        format!("shop · AI Juggler · {ACCOUNT_LABEL}")
    );
    assert_eq!(
        planner.name, "shop",
        "no label, title or branch: its folder"
    );
    assert_eq!(planner.state, "running");
    assert_eq!(
        planner.tool, "fake",
        "no adapter: the tool's id, never a made-up name"
    );
    assert!(planner.last_activity_ms.is_some());

    // Nothing read yet: every activity field says so, none is empty-as-known.
    assert_eq!(planner.said.value, None);
    assert_eq!(
        planner.said.unavailable.as_deref(),
        Some("Unavailable — read only while it runs in AI Juggler")
    );
    assert_eq!(planner.files, Vec::<String>::new());
    assert!(
        planner
            .files_unavailable
            .as_deref()
            .unwrap()
            .starts_with("Unavailable — ")
    );
    // Tasks are E19: never a guess.
    for card in &result.cards {
        assert_eq!(card.task.value, None);
        assert_eq!(
            card.task.unavailable.as_deref(),
            Some("Unavailable — AI Juggler doesn't keep tasks yet")
        );
    }
}

#[test]
fn the_tick_reads_the_transcript_and_the_card_says_whose_words_they_are() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool, transcript) = checkout(dir.path());

    // Read, but nothing changed and no task list: known empty, and "hasn't said".
    cards::tick(&state);
    let result = cards::cards_impl(&state, &pool).unwrap();
    let planner = card(&result.cards, "s-planner");
    assert_eq!(planner.files, Vec::<String>::new());
    assert_eq!(planner.files_unavailable, None, "read: no file changed yet");
    assert_eq!(
        planner.said.unavailable.as_deref(),
        Some("Unavailable — it hasn't said")
    );
    // The builder has no transcript: still unavailable.
    let builder = card(&result.cards, "s-builder");
    assert!(builder.files_unavailable.is_some());

    // It works: an edit, a new file, and its own task list.
    let mut more = std::fs::read_to_string(&transcript).unwrap();
    more.push_str(&call("Edit", r#"{"file_path":"/w/shop/src/cart.ts"}"#));
    more.push_str(&call("Write", r#"{"file_path":"/w/shop/src/pay.ts"}"#));
    more.push_str(&call(
        "TodoWrite",
        r#"{"todos":[{"content":"Plan the cart","status":"completed"},{"content":"Split checkout into steps","status":"in_progress"}]}"#,
    ));
    std::fs::write(&transcript, more).unwrap();
    cards::tick(&state);
    let result = cards::cards_impl(&state, &pool).unwrap();
    let planner = card(&result.cards, "s-planner");
    assert_eq!(
        planner.files,
        ["src/pay.ts", "src/cart.ts"],
        "newest first, last two parts"
    );
    assert_eq!(
        planner.said.value.as_deref(),
        Some("Split checkout into steps")
    );

    // It ended: what was read while it ran stays.
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&SessionId::new("s-planner"))
            .unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
    }
    cards::tick(&state);
    let result = cards::cards_impl(&state, &pool).unwrap();
    assert_eq!(card(&result.cards, "s-planner").files.len(), 2);
}

/// D-33: a card names the account by its label only — never its id, an
/// email or its plan — and not at all when the label is an email.
#[test]
fn a_card_carries_the_account_label_and_never_an_email_or_plan() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool, _) = checkout(dir.path());
    cards::tick(&state);
    // shop's default pool: the project's own (D-33).
    let shop_pool = pools::pools_list_impl(&state)
        .unwrap()
        .pools
        .into_iter()
        .find(|row| row.kind == "default" && row.project_id.is_some())
        .unwrap()
        .id;
    for pool_id in [pool.as_str(), shop_pool.as_str()] {
        let result = cards::cards_impl(&state, pool_id).unwrap();
        for card in &result.cards {
            assert_eq!(card.account.as_deref(), Some(ACCOUNT_LABEL));
        }
        let json = serde_json::to_string(&result).unwrap();
        for leak in ["acc-work", PLAN, "rec-work", "@", "state_dir"] {
            assert!(!json.contains(leak), "a card carries {leak:?}: {json}");
        }
    }
    // An account labelled with its email address: no account on the card.
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute("UPDATE accounts SET label = 'someone@example.com'", [])
        .unwrap();
    let result = cards::cards_impl(&state, &pool).unwrap();
    assert!(result.cards.iter().all(|card| card.account.is_none()));
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains('@'), "{json}");
}

#[test]
fn a_missing_pool_is_refused_in_words() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    match cards::cards_impl(&state, "pool-gone") {
        Err(AppError::Refused(words)) => assert!(words.contains("isn't there"), "{words}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    // The default pool always has cards to give (none yet).
    assert!(
        cards::cards_impl(&state, NO_PROJECT_POOL_ID)
            .unwrap()
            .cards
            .is_empty()
    );
}
