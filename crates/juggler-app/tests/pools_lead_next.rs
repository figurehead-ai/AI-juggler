//! 0.4.6 (AJ-E3011): who leads is never a dead end. A pool of Apps with no
//! session open waits for an App's next session to lead; D-92 holds (one
//! session leads).
//!
//! - The wait is set only for an App in the pool, and only while the pool
//!   has no Lead; picking a Lead by hand, or taking the App out, ends it.
//! - The App's next session in the pool's scope takes the Lead, as a
//!   session member, and the wait ends; one outside the scope doesn't.
//! - Two sessions starting at once: only one leads.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::{Arc, Barrier};

use juggler_adapter::AdapterRegistry;
use juggler_app::error::AppError;
use juggler_app::features::pools::{self, PoolsInfo};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectId, SessionId, SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn account(state: &Arc<AppState>, id: &str) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: id.into(),
            plan: None,
            state_dir: state.accounts_dir().join(id),
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
            id: ProfileId::new(format!("rec-{id}")),
            name: "Main".into(),
            account: AccountId::new(id),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

fn project(state: &Arc<AppState>, root: &Path, name: &str) -> ProjectId {
    std::fs::create_dir_all(root).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(root, Some(name))
        .unwrap()
        .id
}

/// A session of `account`'s CLI App (`cli:<account>`) starting in `project`.
fn start(
    state: &Arc<AppState>,
    id: &str,
    account: &str,
    project: &ProjectId,
    cwd: &Path,
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
            profile: ProfileId::new(format!("rec-{account}")),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.clone()),
            cwd: cwd.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    session
}

fn pool_id(info: &PoolsInfo, name: &str) -> String {
    info.pools
        .iter()
        .find(|pool| pool.name == name)
        .unwrap()
        .id
        .clone()
}

fn refused<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Refused(words)) => words,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn leads(state: &Arc<AppState>, pool: &str) -> Vec<String> {
    pools::model::rules_impl(state, pool)
        .unwrap()
        .members
        .into_iter()
        .filter(|row| row.role == "lead")
        .map(|row| row.member)
        .collect()
}

/// "web" with a pool over it holding the App of `work` as a Member.
struct Lab {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    web: ProjectId,
    web_root: std::path::PathBuf,
    pool: String,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    account(&state, "side");
    let web_root = dir.path().join("web");
    let web = project(&state, &web_root, "web");
    let info = pools::pool_create_impl(&state, "Team", Some(web.as_str())).unwrap();
    let pool = pool_id(&info, "Team");
    pools::model::join_impl(&state, &pool, "app:cli:work", "member", None, &[]).unwrap();
    Lab {
        _dir: dir,
        state,
        web,
        web_root,
        pool,
    }
}

#[test]
fn the_apps_next_session_leads_and_the_wait_ends() {
    let lab = lab();
    let waits = pools::lead_next::set_impl(&lab.state, &lab.pool, Some("cli:work")).unwrap();
    assert_eq!(waits.as_deref(), Some("cli:work"));
    assert_eq!(
        pools::model::rules_impl(&lab.state, &lab.pool)
            .unwrap()
            .lead_next
            .as_deref(),
        Some("cli:work")
    );

    // Another App's session in web doesn't take it.
    let side = start(&lab.state, "s-side", "side", &lab.web, &lab.web_root);
    assert!(pools::lead_next::claim(&lab.state, &side).is_empty());

    let first = start(&lab.state, "s-1", "work", &lab.web, &lab.web_root);
    assert_eq!(
        pools::lead_next::claim(&lab.state, &first),
        [lab.pool.clone()]
    );
    assert_eq!(leads(&lab.state, &lab.pool), ["session:s-1"]);
    assert_eq!(
        pools::model::rules_impl(&lab.state, &lab.pool)
            .unwrap()
            .lead_next,
        None
    );

    // The next one joins through its App, as a Member.
    let second = start(&lab.state, "s-2", "work", &lab.web, &lab.web_root);
    assert!(pools::lead_next::claim(&lab.state, &second).is_empty());
    assert_eq!(leads(&lab.state, &lab.pool), ["session:s-1"]);
    let role =
        pools::members::role_in(&lab.state.storage.lock().unwrap(), &second, &lab.pool).unwrap();
    assert_eq!(role.map(|role| role.as_str()), Some("member"));
}

#[test]
fn a_session_outside_the_pools_projects_doesnt_lead() {
    let lab = lab();
    pools::lead_next::set_impl(&lab.state, &lab.pool, Some("cli:work")).unwrap();
    let docs_root = lab._dir.path().join("docs");
    let docs = project(&lab.state, &docs_root, "docs");
    let outside = start(&lab.state, "s-docs", "work", &docs, &docs_root);
    assert!(pools::lead_next::claim(&lab.state, &outside).is_empty());
    assert!(leads(&lab.state, &lab.pool).is_empty());
    assert_eq!(
        pools::model::rules_impl(&lab.state, &lab.pool)
            .unwrap()
            .lead_next
            .as_deref(),
        Some("cli:work"),
        "it still waits"
    );
}

#[test]
fn the_wait_is_only_for_an_app_in_a_pool_with_no_lead() {
    let lab = lab();
    let words = refused(pools::lead_next::set_impl(
        &lab.state,
        &lab.pool,
        Some("cli:side"),
    ));
    assert!(
        words.starts_with("Add the App to the pool first"),
        "{words}"
    );

    let lead = start(&lab.state, "s-lead", "side", &lab.web, &lab.web_root);
    pools::model::join_impl(
        &lab.state,
        &lab.pool,
        &format!("session:{lead}"),
        "lead",
        None,
        &[],
    )
    .unwrap();
    let words = refused(pools::lead_next::set_impl(
        &lab.state,
        &lab.pool,
        Some("cli:work"),
    ));
    assert!(words.starts_with("This pool has a Lead"), "{words}");

    // Cleared at any time.
    assert_eq!(
        pools::lead_next::set_impl(&lab.state, &lab.pool, None).unwrap(),
        None
    );
}

#[test]
fn a_lead_picked_by_hand_or_the_app_leaving_ends_the_wait() {
    let lab = lab();
    pools::lead_next::set_impl(&lab.state, &lab.pool, Some("cli:work")).unwrap();
    let open = start(&lab.state, "s-open", "work", &lab.web, &lab.web_root);
    pools::pool_set_role_impl(&lab.state, open.as_str(), "lead", Some(&lab.pool)).unwrap();
    assert_eq!(
        pools::model::rules_impl(&lab.state, &lab.pool)
            .unwrap()
            .lead_next,
        None
    );

    pools::pool_set_role_impl(&lab.state, open.as_str(), "member", Some(&lab.pool)).unwrap();
    pools::lead_next::set_impl(&lab.state, &lab.pool, Some("cli:work")).unwrap();
    pools::model::leave_impl(&lab.state, &lab.pool, "app:cli:work").unwrap();
    assert_eq!(
        pools::model::rules_impl(&lab.state, &lab.pool)
            .unwrap()
            .lead_next,
        None
    );
}

#[test]
fn two_sessions_starting_at_once_only_one_leads() {
    for _ in 0..10 {
        let lab = lab();
        pools::lead_next::set_impl(&lab.state, &lab.pool, Some("cli:work")).unwrap();
        let a = start(&lab.state, "s-a", "work", &lab.web, &lab.web_root);
        let b = start(&lab.state, "s-b", "work", &lab.web, &lab.web_root);
        let go = Arc::new(Barrier::new(2));
        let claims: Vec<_> = [a, b]
            .into_iter()
            .map(|session| {
                let (state, go) = (Arc::clone(&lab.state), Arc::clone(&go));
                std::thread::spawn(move || {
                    go.wait();
                    pools::lead_next::claim(&state, &session)
                })
            })
            .collect();
        let led: Vec<Vec<String>> = claims
            .into_iter()
            .map(|claim| claim.join().unwrap())
            .collect();
        assert_eq!(
            led.iter().filter(|pools| !pools.is_empty()).count(),
            1,
            "{led:?}"
        );
        assert_eq!(
            leads(&lab.state, &lab.pool).len(),
            1,
            "one session leads (D-92)"
        );
    }
}
