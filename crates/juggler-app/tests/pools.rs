//! AJ-E1702: pools, the default pool, roles and permissions.
//!
//! - "All sessions" always exists; it can't be renamed or deleted, and
//!   every session no other pool holds is in it, as a member.
//! - The user story: a planner and two builders in "Checkout redesign",
//!   the planner as lead.
//! - Membership hangs off the Work item, so a switch keeps it; a switch
//!   from a session starts that session's own Work item.
//! - A pool for one project refuses a session from another, in words.
//! - Grants: the plan doc's table by default; the user's changes stick;
//!   the two fixed rows can't be changed.
//! - The guardrail: pool operations never start, stop or type into a
//!   session — asserted on a real running process, and on the source.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::error::AppError;
use juggler_app::pools::{self, PoolsInfo};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, Epoch, EpochId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe,
    MachineId, ProfileId, ProjectId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::storage::DEFAULT_POOL_ID;

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

fn session(
    state: &Arc<AppState>,
    id: &str,
    account: &str,
    project: Option<&ProjectId>,
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
            project: project.cloned(),
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

fn place<'a>(info: &'a PoolsInfo, session: &str) -> (&'a str, &'a str) {
    let row = info
        .members
        .iter()
        .find(|row| row.session_id == session)
        .unwrap_or_else(|| panic!("{session} has no place"));
    (row.pool_id.as_str(), row.role.as_str())
}

fn pool_id(info: &PoolsInfo, name: &str) -> String {
    info.pools
        .iter()
        .find(|pool| pool.name == name)
        .unwrap_or_else(|| panic!("no pool {name}"))
        .id
        .clone()
}

fn refused(result: Result<PoolsInfo, AppError>) -> String {
    match result {
        Err(AppError::Refused(words)) => words,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Records what a switch records: the source's epoch and the new
/// session's, in the Work item the switch continues.
fn switch(state: &Arc<AppState>, project: &ProjectId, from: &SessionId, to: &SessionId) {
    let storage = state.storage.lock().unwrap();
    let graph = storage.switch_graph();
    let work = graph.work_for_switch(project, from, "Work").unwrap();
    for (n, session) in [from, to].into_iter().enumerate() {
        graph
            .insert_epoch(&Epoch {
                id: EpochId::new(format!("ep-{}-{n}", to.as_str())),
                work: work.id.clone(),
                session_ref: Some(session.clone()),
                native_id: None,
                account: AccountId::new("work"),
                provider: HarnessId::new("claude-code"),
                resumed_from_epoch: None,
                machine: None,
            })
            .unwrap();
    }
}

#[test]
fn the_default_pool_always_exists_and_holds_every_other_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let a = session(&state, "s-a", "work", None, dir.path());

    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools.len(), 1);
    assert_eq!(info.pools[0].id, DEFAULT_POOL_ID);
    assert_eq!(info.pools[0].name, "All sessions");
    assert_eq!(info.pools[0].kind, "default");
    assert_eq!(place(&info, a.as_str()), (DEFAULT_POOL_ID, "member"));

    let words = refused(pools::pool_delete_impl(&state, DEFAULT_POOL_ID));
    assert!(words.contains("can't be deleted"), "{words}");
    let words = refused(pools::pool_rename_impl(&state, DEFAULT_POOL_ID, "Mine"));
    assert!(words.contains("keeps its name"), "{words}");
    // Its name is taken, whatever the case.
    let words = refused(pools::pool_create_impl(&state, " all SESSIONS ", None));
    assert!(
        words.contains("already a pool called «All sessions»"),
        "{words}"
    );

    // Still there after a restart.
    drop(state);
    let state = app_state(dir.path());
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools[0].name, "All sessions");
}

#[test]
fn a_planner_and_two_builders_in_checkout_redesign_with_the_planner_as_lead() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let planner = session(&state, "s-planner", "work", None, dir.path());
    let builder1 = session(&state, "s-b1", "work", None, dir.path());
    let builder2 = session(&state, "s-b2", "work", None, dir.path());
    let other = session(&state, "s-other", "work", None, dir.path());

    let info = pools::pool_create_impl(&state, "  Checkout redesign ", None).unwrap();
    let checkout = pool_id(&info, "Checkout redesign");
    assert_eq!(info.pools[1].kind, "user");
    for member in [&planner, &builder1, &builder2] {
        pools::pool_move_impl(&state, member.as_str(), &checkout).unwrap();
    }
    let info = pools::pool_set_role_impl(&state, planner.as_str(), "lead").unwrap();
    assert_eq!(place(&info, planner.as_str()), (checkout.as_str(), "lead"));
    assert_eq!(
        place(&info, builder1.as_str()),
        (checkout.as_str(), "member")
    );
    assert_eq!(
        place(&info, builder2.as_str()),
        (checkout.as_str(), "member")
    );
    assert_eq!(place(&info, other.as_str()), (DEFAULT_POOL_ID, "member"));

    // An observer, then back.
    let info = pools::pool_set_role_impl(&state, builder2.as_str(), "observer").unwrap();
    assert_eq!(place(&info, builder2.as_str()).1, "observer");
    assert!(matches!(
        pools::pool_set_role_impl(&state, builder2.as_str(), "owner"),
        Err(AppError::Validation { .. })
    ));

    // Rename: a taken name is refused; the members stay.
    pools::pool_create_impl(&state, "Docs", None).unwrap();
    let words = refused(pools::pool_rename_impl(&state, &checkout, "docs"));
    assert!(words.contains("«Docs»"), "{words}");
    let info = pools::pool_rename_impl(&state, &checkout, "Checkout v2").unwrap();
    assert_eq!(pool_id(&info, "Checkout v2"), checkout);
    assert_eq!(place(&info, planner.as_str()), (checkout.as_str(), "lead"));
    assert_eq!(
        refused(pools::pool_create_impl(&state, "   ", None)),
        "Give the pool a name"
    );
    let long = "x".repeat(49);
    assert!(refused(pools::pool_create_impl(&state, &long, None)).contains("at most 48"));

    // Moving to another pool makes it a member there.
    let docs = pool_id(&info, "Docs");
    let info = pools::pool_move_impl(&state, planner.as_str(), &docs).unwrap();
    assert_eq!(place(&info, planner.as_str()), (docs.as_str(), "member"));

    // Delete: its sessions go back to All sessions; the pool is gone.
    let info = pools::pool_delete_impl(&state, &checkout).unwrap();
    assert!(info.pools.iter().all(|pool| pool.id != checkout));
    assert_eq!(place(&info, builder1.as_str()), (DEFAULT_POOL_ID, "member"));
    assert_eq!(place(&info, builder2.as_str()), (DEFAULT_POOL_ID, "member"));
    assert_eq!(place(&info, planner.as_str()), (docs.as_str(), "member"));
    assert_eq!(
        refused(pools::pool_move_impl(&state, builder1.as_str(), &checkout)),
        "That pool isn't there any more"
    );
    // The name is free again.
    pools::pool_create_impl(&state, "Checkout redesign", None).unwrap();
}

#[test]
fn membership_hangs_off_the_work_item_so_a_switch_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let root = dir.path().join("shop");
    let shop = project(&state, &root, "shop");
    let a = session(&state, "s-a", "work", Some(&shop), &root);
    let other = session(&state, "s-other", "work", Some(&shop), &root);
    let info = pools::pool_create_impl(&state, "Checkout", None).unwrap();
    let checkout = pool_id(&info, "Checkout");
    pools::pool_move_impl(&state, a.as_str(), &checkout).unwrap();
    pools::pool_set_role_impl(&state, a.as_str(), "lead").unwrap();

    // Another session in the same project switched first: its Work item
    // is its own, and it stays out of the pool.
    let other_next = session(&state, "s-other-2", "work", Some(&shop), &root);
    switch(&state, &shop, &other, &other_next);

    // a switches to a new session: the pool and the role follow the work.
    let b = session(&state, "s-b", "work", Some(&shop), &root);
    switch(&state, &shop, &a, &b);
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(place(&info, b.as_str()), (checkout.as_str(), "lead"));
    assert_eq!(place(&info, a.as_str()), (checkout.as_str(), "lead"));
    assert_eq!(place(&info, other.as_str()), (DEFAULT_POOL_ID, "member"));
    assert_eq!(
        place(&info, other_next.as_str()),
        (DEFAULT_POOL_ID, "member")
    );

    // And again from b: same Work item, same pool.
    let c = session(&state, "s-c", "work", Some(&shop), &root);
    switch(&state, &shop, &b, &c);
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(place(&info, c.as_str()), (checkout.as_str(), "lead"));

    // A change made after the switch is one change, not two rows that
    // disagree: moving c moves the whole line of work.
    let info = pools::pool_move_impl(&state, c.as_str(), DEFAULT_POOL_ID).unwrap();
    for session in [&a, &b, &c] {
        assert_eq!(place(&info, session.as_str()), (DEFAULT_POOL_ID, "member"));
    }
    let storage = state.storage.lock().unwrap();
    let active = storage.pools().members().unwrap();
    assert_eq!(active.len(), 1, "{active:?}");
}

#[test]
fn a_switch_from_a_session_starts_its_own_work_item() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let root = dir.path().join("shop");
    let shop = project(&state, &root, "shop");
    let storage = state.storage.lock().unwrap();
    let graph = storage.switch_graph();
    let one = graph
        .work_for_switch(&shop, &SessionId::new("s-1"), "Work")
        .unwrap();
    graph
        .insert_epoch(&Epoch {
            id: EpochId::new("ep-1"),
            work: one.id.clone(),
            session_ref: Some(SessionId::new("s-1")),
            native_id: None,
            account: AccountId::new("work"),
            provider: HarnessId::new("claude-code"),
            resumed_from_epoch: None,
            machine: None,
        })
        .unwrap();
    // Before: every switch in a project went into its oldest Work item.
    let two = graph
        .work_for_switch(&shop, &SessionId::new("s-2"), "Work")
        .unwrap();
    assert_ne!(one.id, two.id);
    let again = graph
        .work_for_switch(&shop, &SessionId::new("s-1"), "Work")
        .unwrap();
    assert_eq!(one.id, again.id);
}

#[test]
fn a_pool_for_one_project_refuses_a_session_from_another_in_words() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let shop_root = dir.path().join("shop");
    let blog_root = dir.path().join("blog");
    let shop = project(&state, &shop_root, "shop");
    let blog = project(&state, &blog_root, "blog");
    let in_shop = session(&state, "s-shop", "work", Some(&shop), &shop_root);
    let in_blog = session(&state, "s-blog", "work", Some(&blog), &blog_root);
    let loose = session(&state, "s-loose", "work", None, dir.path());

    let info = pools::pool_create_impl(&state, "Shop team", Some(shop.as_str())).unwrap();
    let team = pool_id(&info, "Shop team");
    assert_eq!(info.pools[1].project_id.as_deref(), Some(shop.as_str()));
    pools::pool_move_impl(&state, in_shop.as_str(), &team).unwrap();
    let words = refused(pools::pool_move_impl(&state, in_blog.as_str(), &team));
    assert_eq!(
        words,
        "«Shop team» is for shop only — this session works somewhere else"
    );
    refused(pools::pool_move_impl(&state, loose.as_str(), &team));
    let words = refused(pools::pool_create_impl(&state, "Ghost", Some("proj-gone")));
    assert!(words.contains("isn't there any more"), "{words}");
}

#[test]
fn scopes_count_projects_and_accounts_d33() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    account(&state, "personal");
    let shop_root = dir.path().join("shop");
    let shop = project(&state, &shop_root, "shop");
    let blog_root = dir.path().join("blog");
    let blog = project(&state, &blog_root, "blog");
    let w1 = session(&state, "s-w1", "work", Some(&shop), &shop_root);
    session(&state, "s-w2", "work", Some(&shop), &shop_root);
    let p1 = session(&state, "s-p1", "personal", Some(&shop), &shop_root);
    session(&state, "s-b1", "work", Some(&blog), &blog_root);

    // All sessions: shop · work, shop · personal, blog · work.
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools[0].scopes, 3);

    // A user pool with work and personal sessions together: two groups,
    // which the UI names as a warning.
    let info = pools::pool_create_impl(&state, "Mixed", None).unwrap();
    let mixed = pool_id(&info, "Mixed");
    pools::pool_move_impl(&state, w1.as_str(), &mixed).unwrap();
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools[1].scopes, 1);
    let info = pools::pool_move_impl(&state, p1.as_str(), &mixed).unwrap();
    assert_eq!(info.pools[1].scopes, 2);
    assert_eq!(info.pools[0].scopes, 2);
}

#[test]
fn grants_follow_the_plan_table_and_only_change_where_the_user_changes_them() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let info = pools::pool_create_impl(&state, "Team", None).unwrap();
    let team = pool_id(&info, "Team");
    let grants = |info: &PoolsInfo, pool: &str| -> Vec<(String, bool, bool, bool)> {
        info.pools
            .iter()
            .find(|row| row.id == pool)
            .unwrap()
            .grants
            .iter()
            .map(|grant| {
                (
                    grant.capability.clone(),
                    grant.lead,
                    grant.member,
                    grant.observer,
                )
            })
            .collect()
    };
    // pools-messaging-tasks.md §2.3, the grants table (owner = the user).
    let table: Vec<(String, bool, bool, bool)> = [
        ("roster", true, true, true),
        ("own_inbox", true, true, true),
        ("message", true, true, false),
        ("broadcast", true, false, false),
        ("share_doc", true, true, false),
        ("create_task", true, true, false),
        ("create_plan", true, false, false),
        ("assign", true, false, false),
        ("claim", true, true, false),
        ("update_own", true, true, false),
        ("close_others", true, false, false),
    ]
    .into_iter()
    .map(|(cap, lead, member, observer)| (cap.to_owned(), lead, member, observer))
    .collect();
    assert_eq!(grants(&info, &team), table);
    assert_eq!(grants(&info, DEFAULT_POOL_ID), table);

    // "Message one member: lead-only (option)".
    let info = pools::pool_set_grant_impl(&state, &team, "message", "member", false).unwrap();
    assert!(!grants(&info, &team)[2].2);
    assert!(grants(&info, DEFAULT_POOL_ID)[2].2, "another pool changed");
    // Back to the default: the override row goes.
    let info = pools::pool_set_grant_impl(&state, &team, "message", "member", true).unwrap();
    assert_eq!(grants(&info, &team), table);
    {
        let storage = state.storage.lock().unwrap();
        let rows: i64 = storage
            .raw_conn()
            .query_row("SELECT COUNT(*) FROM pool_grants", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }
    // An observer may be let in.
    let info = pools::pool_set_grant_impl(&state, &team, "claim", "observer", true).unwrap();
    assert!(grants(&info, &team)[8].3);
    // It sticks across a restart.
    drop(state);
    let state = app_state(dir.path());
    let info = pools::pools_list_impl(&state).unwrap();
    assert!(grants(&info, &team)[8].3);

    // The fixed rows can't be changed; unknown words are refused.
    let words = refused(pools::pool_set_grant_impl(
        &state, &team, "roster", "observer", false,
    ));
    assert!(words.contains("always see the pool"), "{words}");
    assert!(matches!(
        pools::pool_set_grant_impl(&state, &team, "drop_tables", "lead", true),
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        pools::pool_set_grant_impl(&state, &team, "assign", "owner", true),
        Err(AppError::Validation { .. })
    ));
}

/// What a running session would show if anything touched it.
struct Watched {
    session: SessionId,
    pid: u32,
    typed: std::path::PathBuf,
    signals: std::path::PathBuf,
}

fn running_session(state: &Arc<AppState>, dir: &Path) -> Watched {
    let session = session(state, "s-live", "work", None, dir);
    let typed = dir.join("typed.log");
    let signals = dir.join("signals.log");
    let script = format!(
        "trap 'echo TERM >> {s}; exit 0' TERM; trap 'echo HUP >> {s}' HUP; trap 'echo INT >> {s}' INT; \
         : > {t}; while IFS= read -r line; do echo \"$line\" >> {t}; done",
        s = signals.display(),
        t = typed.display()
    );
    let spawned = state
        .supervisor
        .spawn(
            &session,
            &LaunchSpec {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), script],
                env: EnvMap::new(),
                cwd: dir.to_path_buf(),
                needs_pty: true,
            },
            &juggler_supervisor::tree::SpawnContext {
                provider: "claude-code".into(),
                account_id: Some("work".into()),
                project_id: None,
                machine: commands::local_machine_id(state),
                role: "root",
            },
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !typed.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(typed.exists(), "the watched session didn't start");
    Watched {
        session,
        pid: spawned.pid,
        typed,
        signals,
    }
}

fn alive(pid: u32) -> bool {
    // Signal 0: is it there? (Sends nothing.)
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn pool_operations_never_start_stop_or_type_into_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let watched = running_session(&state, dir.path());
    // The seam proves itself: what's typed shows.
    state
        .supervisor
        .write_input(&watched.session, b"probe\n")
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::fs::read_to_string(&watched.typed)
        .unwrap_or_default()
        .is_empty()
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(std::fs::read_to_string(&watched.typed).unwrap(), "probe\n");

    let before_spawns = state.supervisor.spawn_contexts_snapshot().len();
    let ledger_rows = |state: &Arc<AppState>| -> i64 {
        state
            .storage
            .lock()
            .unwrap()
            .raw_conn()
            .query_row("SELECT COUNT(*) FROM process_ledger", [], |row| row.get(0))
            .unwrap()
    };
    let before_ledger = ledger_rows(&state);
    let before_status = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&watched.session)
        .unwrap()
        .status;

    // Every pool operation, the refused ones too.
    let id = watched.session.as_str();
    let info = pools::pool_create_impl(&state, "Team", None).unwrap();
    let team = pool_id(&info, "Team");
    pools::pool_move_impl(&state, id, &team).unwrap();
    pools::pool_set_role_impl(&state, id, "lead").unwrap();
    pools::pool_set_role_impl(&state, id, "observer").unwrap();
    pools::pool_set_grant_impl(&state, &team, "broadcast", "member", true).unwrap();
    pools::pool_rename_impl(&state, &team, "Team two").unwrap();
    pools::pools_list_impl(&state).unwrap();
    let _ = pools::pool_delete_impl(&state, DEFAULT_POOL_ID);
    let _ = pools::pool_set_grant_impl(&state, &team, "roster", "lead", false);
    pools::pool_move_impl(&state, id, DEFAULT_POOL_ID).unwrap();
    pools::pool_move_impl(&state, id, &team).unwrap();
    pools::pool_delete_impl(&state, &team).unwrap();
    std::thread::sleep(Duration::from_millis(300));

    assert!(alive(watched.pid), "a pool operation stopped the session");
    assert_eq!(
        std::fs::read_to_string(&watched.typed).unwrap(),
        "probe\n",
        "a pool operation typed into the session"
    );
    assert_eq!(
        std::fs::read_to_string(&watched.signals).unwrap_or_default(),
        "",
        "a pool operation signalled the session"
    );
    assert_eq!(
        state.supervisor.spawn_contexts_snapshot().len(),
        before_spawns,
        "a pool operation started a session"
    );
    assert_eq!(
        ledger_rows(&state),
        before_ledger,
        "a pool operation launched something"
    );
    assert_eq!(
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&watched.session)
            .unwrap()
            .status,
        before_status
    );
    state
        .supervisor
        .stop(&watched.session, Duration::from_secs(2))
        .unwrap();
}

/// The same law on the source: the pool code has no way to reach a
/// session. A new call to the supervisor, a launch or a terminal write in
/// it fails here even where the behaviour test above doesn't exercise it.
#[test]
fn the_pool_code_has_no_path_to_a_session() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in [
        root.join("src/pools.rs"),
        root.join("../juggler-core/src/storage/pools.rs"),
    ] {
        let source = std::fs::read_to_string(&file).unwrap();
        // Doc comments may say what the code never does.
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "supervisor",
            "spawn",
            "write_input",
            "terminal",
            "launch",
            ".stop(",
            "kill",
            "signal",
            "Command::new",
            "LaunchSpec",
        ] {
            assert!(
                !code.contains(needle),
                "{} mentions `{needle}`: pool operations must never start, stop or type into a session",
                file.display()
            );
        }
    }
}
