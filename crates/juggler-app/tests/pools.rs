//! AJ-E1702: pools, the default pool, roles and permissions.
//!
//! - D-33: each project has its own default pool, holding every session
//!   of it (any account) that no other pool holds, as a member; sessions
//!   with no project have theirs. A default pool can't be renamed or
//!   deleted, and takes only its own project's sessions.
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
use juggler_app::features::pools::{self, PoolsInfo};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, Epoch, EpochId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe,
    MachineId, ProfileId, ProjectId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::storage::{NO_PROJECT_POOL_ID, default_pool_id};

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

/// A session's pool and role.
#[derive(Debug)]
struct Place(String, String);

impl PartialEq<(&str, &str)> for Place {
    fn eq(&self, other: &(&str, &str)) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

/// A session's place as `pools_place` reads it; the pools' answer `info`
/// agrees (these sessions run, so they're in the working set): it names
/// the place unless it's the project's default pool, as a member.
fn place(state: &Arc<AppState>, info: &PoolsInfo, session: &str) -> Place {
    let read = pools::members::place_impl(state, session).unwrap();
    let project = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session))
        .unwrap()
        .project;
    let default = read.pool_id == default_pool_id(project.as_ref()) && read.role == "member";
    let named = info.placements.iter().find(|row| row.session_id == session);
    assert_eq!(
        named.is_none(),
        default,
        "the answer names a place exactly when it isn't the default"
    );
    if let Some(row) = named {
        assert_eq!(row, &read, "the answer and the read agree");
    }
    Place(read.pool_id, read.role)
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
fn each_project_has_its_own_default_pool_holding_all_its_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    account(&state, "personal");
    let shop_root = dir.path().join("shop");
    let blog_root = dir.path().join("blog");
    let shop = project(&state, &shop_root, "shop");
    let blog = project(&state, &blog_root, "blog");
    let w = session(&state, "s-w", "work", Some(&shop), &shop_root);
    let p = session(&state, "s-p", "personal", Some(&shop), &shop_root);
    let b = session(&state, "s-b", "work", Some(&blog), &blog_root);
    let loose = session(&state, "s-loose", "work", None, dir.path());
    let shop_pool = default_pool_id(Some(&shop));
    let blog_pool = default_pool_id(Some(&blog));

    let info = pools::pools_list_impl(&state).unwrap();
    let names: Vec<(&str, &str)> = info
        .pools
        .iter()
        .map(|pool| (pool.name.as_str(), pool.kind.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            ("blog · all sessions", "default"),
            ("shop · all sessions", "default"),
            ("No project · all sessions", "default"),
        ]
    );
    // Every session of a project, on any account, is in its pool.
    assert_eq!(
        place(&state, &info, w.as_str()),
        (shop_pool.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, p.as_str()),
        (shop_pool.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, b.as_str()),
        (blog_pool.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, loose.as_str()),
        (NO_PROJECT_POOL_ID, "member")
    );

    for default in [shop_pool.as_str(), NO_PROJECT_POOL_ID] {
        let words = refused(pools::pool_delete_impl(&state, default));
        assert!(words.contains("can't be deleted"), "{words}");
        let words = refused(pools::pool_rename_impl(&state, default, "Mine"));
        assert!(words.contains("keeps its name"), "{words}");
    }
    // Its name is taken, whatever the case.
    let words = refused(pools::pool_create_impl(
        &state,
        " SHOP · all sessions ",
        None,
    ));
    assert!(words.contains("«shop · all sessions»"), "{words}");
    // A default pool takes only its own project's sessions.
    let words = refused(pools::pool_move_impl(&state, b.as_str(), &shop_pool));
    assert!(words.contains("is for shop only"), "{words}");
    refused(pools::pool_move_impl(&state, loose.as_str(), &shop_pool));
    let words = refused(pools::pool_move_impl(
        &state,
        w.as_str(),
        NO_PROJECT_POOL_ID,
    ));
    assert!(words.contains("sessions with no project"), "{words}");

    // A renamed project renames its pool; a new project gets its own at
    // once; all of it is still there after a restart.
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute(
            "UPDATE projects SET name = 'storefront' WHERE id = ?1",
            [shop.as_str()],
        )
        .unwrap();
    let docs_root = dir.path().join("docs");
    let docs = project(&state, &docs_root, "docs");
    drop(state);
    let state = app_state(dir.path());
    let info = pools::pools_list_impl(&state).unwrap();
    let pool = |id: &str| {
        info.pools
            .iter()
            .find(|pool| pool.id == id)
            .map(|pool| pool.name.clone())
    };
    assert_eq!(
        pool(&shop_pool).as_deref(),
        Some("storefront · all sessions")
    );
    assert_eq!(
        pool(&default_pool_id(Some(&docs))).as_deref(),
        Some("docs · all sessions")
    );
}

/// D-33 with D-28: each project's default pool has its own level.
/// The 0.2 walk (F10): two projects with one name (two folders called
/// `app`) have default pools told apart by the folder each is in, in the
/// list and on the pool's own page alike.
#[test]
fn two_projects_with_one_name_have_pools_told_apart() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let work_root = dir.path().join("work").join("app");
    let home_root = dir.path().join("home").join("app");
    let work = project(&state, &work_root, "app");
    let home = project(&state, &home_root, "app");
    session(&state, "s-w", "work", Some(&work), &work_root);
    session(&state, "s-h", "work", Some(&home), &home_root);

    let info = pools::pools_list_impl(&state).unwrap();
    let mut names: Vec<&str> = info
        .pools
        .iter()
        .filter(|pool| pool.project_id.is_some())
        .map(|pool| pool.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "app · all sessions (in home)",
            "app · all sessions (in work)"
        ]
    );
    let storage = state.storage.lock().unwrap();
    let (_, name) = pools::named_pool(&storage, default_pool_id(Some(&work)).as_str()).unwrap();
    assert_eq!(name, "app · all sessions (in work)");
}

/// Worktrees of two repositories sit in folders of one name (`.wt`): the
/// names go up as many folders as tell them apart.
#[test]
fn pools_in_folders_of_one_name_go_up_until_told_apart() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let a_root = dir.path().join("shop").join(".wt").join("egret");
    let b_root = dir.path().join("blog").join(".wt").join("egret");
    let a = project(&state, &a_root, "egret");
    let b = project(&state, &b_root, "egret");
    session(&state, "s-a", "work", Some(&a), &a_root);
    session(&state, "s-b", "work", Some(&b), &b_root);
    let info = pools::pools_list_impl(&state).unwrap();
    let mut names: Vec<&str> = info
        .pools
        .iter()
        .filter(|pool| pool.project_id.is_some())
        .map(|pool| pool.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "egret · all sessions (in blog/.wt)",
            "egret · all sessions (in shop/.wt)"
        ]
    );
}

#[test]
fn two_projects_default_pools_have_their_own_levels() {
    use juggler_core::agent_access::{AgentAction, Decision};

    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let a_root = dir.path().join("a");
    let b_root = dir.path().join("b");
    let a = project(&state, &a_root, "a");
    let b = project(&state, &b_root, "b");
    let in_a = session(&state, "s-a", "work", Some(&a), &a_root);
    let in_b = session(&state, "s-b", "work", Some(&b), &b_root);
    let (pool_a, pool_b) = (default_pool_id(Some(&a)), default_pool_id(Some(&b)));
    pools::access::set_level_impl(&state, &pool_a, "full", &[]).unwrap();
    pools::access::set_level_impl(&state, &pool_b, "read", &[]).unwrap();
    let deliver = AgentAction::DeliverToSession;
    assert_eq!(
        pools::access::allowed(&state, in_a.as_str(), &pool_a, deliver, Some(0)),
        Decision::Allow
    );
    assert!(matches!(
        pools::access::allowed(
            &state,
            in_b.as_str(),
            &pool_b,
            AgentAction::PostMessage,
            None
        ),
        Decision::Ask { .. }
    ));
    // Each only in its own project's pool.
    assert_eq!(
        pools::access::allowed(
            &state,
            in_a.as_str(),
            &pool_b,
            AgentAction::ReadRoster,
            None
        ),
        Decision::Deny
    );
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
    assert_eq!(info.pools[0].kind, "user", "your pools first");
    for member in [&planner, &builder1, &builder2] {
        pools::pool_move_impl(&state, member.as_str(), &checkout).unwrap();
    }
    let info = pools::pool_set_role_impl(&state, planner.as_str(), "lead").unwrap();
    assert_eq!(
        place(&state, &info, planner.as_str()),
        (checkout.as_str(), "lead")
    );
    assert_eq!(
        place(&state, &info, builder1.as_str()),
        (checkout.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, builder2.as_str()),
        (checkout.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, other.as_str()),
        (NO_PROJECT_POOL_ID, "member")
    );

    // An observer, then back.
    let info = pools::pool_set_role_impl(&state, builder2.as_str(), "observer").unwrap();
    assert_eq!(place(&state, &info, builder2.as_str()).1, "observer");
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
    assert_eq!(
        place(&state, &info, planner.as_str()),
        (checkout.as_str(), "lead")
    );
    assert_eq!(
        refused(pools::pool_create_impl(&state, "   ", None)),
        "Give the pool a name"
    );
    let long = "x".repeat(49);
    assert!(refused(pools::pool_create_impl(&state, &long, None)).contains("at most 48"));

    // Moving to another pool makes it a member there.
    let docs = pool_id(&info, "Docs");
    let info = pools::pool_move_impl(&state, planner.as_str(), &docs).unwrap();
    assert_eq!(
        place(&state, &info, planner.as_str()),
        (docs.as_str(), "member")
    );

    // Delete: its sessions go back to their default pool; the pool is gone.
    let info = pools::pool_delete_impl(&state, &checkout).unwrap();
    assert!(info.pools.iter().all(|pool| pool.id != checkout));
    assert_eq!(
        place(&state, &info, builder1.as_str()),
        (NO_PROJECT_POOL_ID, "member")
    );
    assert_eq!(
        place(&state, &info, builder2.as_str()),
        (NO_PROJECT_POOL_ID, "member")
    );
    assert_eq!(
        place(&state, &info, planner.as_str()),
        (docs.as_str(), "member")
    );
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
    let shop_pool = default_pool_id(Some(&shop));
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
    assert_eq!(
        place(&state, &info, b.as_str()),
        (checkout.as_str(), "lead")
    );
    assert_eq!(
        place(&state, &info, a.as_str()),
        (checkout.as_str(), "lead")
    );
    assert_eq!(
        place(&state, &info, other.as_str()),
        (shop_pool.as_str(), "member")
    );
    assert_eq!(
        place(&state, &info, other_next.as_str()),
        (shop_pool.as_str(), "member")
    );

    // And again from b: same Work item, same pool.
    let c = session(&state, "s-c", "work", Some(&shop), &root);
    switch(&state, &shop, &b, &c);
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(
        place(&state, &info, c.as_str()),
        (checkout.as_str(), "lead")
    );

    // A change made after the switch is one change, not two rows that
    // disagree: moving c moves the whole line of work.
    let info = pools::pool_move_impl(&state, c.as_str(), shop_pool.as_str()).unwrap();
    for session in [&a, &b, &c] {
        assert_eq!(
            place(&state, &info, session.as_str()),
            (shop_pool.as_str(), "member")
        );
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
    let row = info.pools.iter().find(|pool| pool.id == team).unwrap();
    assert_eq!(row.project_id.as_deref(), Some(shop.as_str()));
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
fn scopes_count_projects_whatever_the_account_d33() {
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
    session(&state, "s-p1", "personal", Some(&shop), &shop_root);
    let b1 = session(&state, "s-b1", "work", Some(&blog), &blog_root);

    // shop's pool: work and personal together, one project (D-33).
    let scopes =
        |info: &PoolsInfo, id: &str| info.pools.iter().find(|pool| pool.id == id).unwrap().scopes;
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(scopes(&info, &default_pool_id(Some(&shop))), 1);

    // A user pool with two projects' work: two, which the UI names.
    let info = pools::pool_create_impl(&state, "Mixed", None).unwrap();
    let mixed = pool_id(&info, "Mixed");
    pools::pool_move_impl(&state, w1.as_str(), &mixed).unwrap();
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(scopes(&info, &mixed), 1);
    let info = pools::pool_move_impl(&state, b1.as_str(), &mixed).unwrap();
    assert_eq!(scopes(&info, &mixed), 2);
    assert_eq!(scopes(&info, &default_pool_id(Some(&blog))), 0);
}

/// I6: the pools' answer never lists the history. Each pool says how many
/// sessions it holds (archived ones aside); the working set's places are
/// named only where they aren't the default; a pool's members come a page
/// at a time, its lead first.
#[test]
fn a_pools_members_come_a_page_at_a_time_its_lead_first() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let root = dir.path().join("shop");
    let shop = project(&state, &root, "shop");
    let shop_pool = default_pool_id(Some(&shop));
    let sessions: Vec<SessionId> = (0..7)
        .map(|n| session(&state, &format!("s-{n}"), "work", Some(&shop), &root))
        .collect();
    let info = pools::pool_create_impl(&state, "Team", None).unwrap();
    let team = pool_id(&info, "Team");
    for member in &sessions[..3] {
        pools::pool_move_impl(&state, member.as_str(), &team).unwrap();
    }
    pools::pool_set_role_impl(&state, sessions[2].as_str(), "lead").unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .record_lifecycle(&sessions[6], "archived")
        .unwrap();

    let info = pools::pools_list_impl(&state).unwrap();
    let members = |id: &str| {
        info.pools
            .iter()
            .find(|pool| pool.id == id)
            .unwrap()
            .members
    };
    assert_eq!(members(&team), 3);
    assert_eq!(members(&shop_pool), 3, "the archived one isn't counted");
    let named: Vec<(&str, &str)> = info
        .placements
        .iter()
        .map(|row| (row.session_id.as_str(), row.role.as_str()))
        .collect();
    assert_eq!(
        named,
        [("s-0", "member"), ("s-1", "member"), ("s-2", "lead")],
        "only the places that aren't the default"
    );

    let first = pools::members::members_page(&state, &team, &PageRequest::first(2)).unwrap();
    assert_eq!(first.total, Some(3));
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.items[0].session.id, "s-2", "the lead first");
    assert_eq!(first.items[0].card.role, "lead");
    let cursor = first.next_cursor.clone().unwrap();
    let rest =
        pools::members::members_page(&state, &team, &PageRequest::after(&cursor, 2)).unwrap();
    assert_eq!(rest.items.len(), 1);
    assert_eq!(rest.next_cursor, None, "the last page says so");
    let mut seen: Vec<&str> = first
        .items
        .iter()
        .chain(&rest.items)
        .map(|row| row.session.id.as_str())
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, ["s-0", "s-1", "s-2"]);
    for row in first.items.iter().chain(&rest.items) {
        assert_eq!(row.card.session_id, row.session.id);
    }

    let default =
        pools::members::members_page(&state, &shop_pool, &PageRequest::default()).unwrap();
    assert_eq!(default.total, Some(3));
    assert!(
        default.items.iter().all(|row| row.session.id != "s-6"),
        "an archived session is history"
    );
    assert_eq!(
        pools::members::members_page(&state, "pool-gone", &PageRequest::default())
            .err()
            .map(|error| matches!(error, AppError::Refused(_))),
        Some(true)
    );
}

/// D-32: a pool spans the user's paired Macs only when the user says so.
#[test]
fn a_pool_spans_paired_macs_only_when_the_user_turns_it_on() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let info = pools::pool_create_impl(&state, "Team", None).unwrap();
    let team = pool_id(&info, "Team");
    assert!(
        info.pools.iter().all(|pool| !pool.spans_macs),
        "off by default"
    );
    let info = pools::pool_set_spans_macs_impl(&state, &team, true).unwrap();
    assert!(
        info.pools
            .iter()
            .find(|pool| pool.id == team)
            .unwrap()
            .spans_macs
    );
    assert!(
        info.pools
            .iter()
            .filter(|pool| pool.id != team)
            .all(|pool| !pool.spans_macs),
        "another pool unchanged"
    );
    drop(state);
    let state = app_state(dir.path());
    let info = pools::pools_list_impl(&state).unwrap();
    assert!(
        info.pools
            .iter()
            .find(|pool| pool.id == team)
            .unwrap()
            .spans_macs
    );
    pools::pool_delete_impl(&state, &team).unwrap();
    assert!(matches!(
        pools::pool_set_spans_macs_impl(&state, &team, false),
        Err(AppError::Refused(_))
    ));
}

/// D-31: what decides pools and agent access never reads quota, so no
/// pool, level or rule can route work by an account's remaining limit.
#[test]
fn pools_and_agent_access_never_read_quota() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("src/features/pools"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.extend([
        root.join("../juggler-core/src/storage/pools.rs"),
        root.join("../juggler-core/src/storage/agent_access.rs"),
        root.join("../juggler-core/src/agent_access.rs"),
    ]);
    for file in files {
        let code = std::fs::read_to_string(&file).unwrap().to_lowercase();
        for needle in ["quota", "runway", "ranking"] {
            assert!(
                !code.contains(needle),
                "{} mentions {needle}",
                file.display()
            );
        }
    }
}

#[test]
fn access_levels_are_the_users_and_the_decision_point_asks_above_them() {
    use juggler_core::agent_access::{AgentAction, Decision};

    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    account(&state, "work");
    let planner = session(&state, "s-planner", "work", None, dir.path());
    let watcher = session(&state, "s-watcher", "work", None, dir.path());
    let info = pools::pool_create_impl(&state, "Team", None).unwrap();
    let team = pool_id(&info, "Team");
    pools::pool_move_impl(&state, planner.as_str(), &team).unwrap();
    pools::pool_move_impl(&state, watcher.as_str(), &team).unwrap();
    pools::pool_set_role_impl(&state, watcher.as_str(), "observer").unwrap();
    let level = |info: &PoolsInfo, pool: &str| {
        let row = info.pools.iter().find(|row| row.id == pool).unwrap();
        (row.access.level.clone(), row.access.actions.clone())
    };

    // Every pool starts at Work together, the default pool too.
    let info = pools::pools_list_impl(&state).unwrap();
    for pool in [NO_PROJECT_POOL_ID, team.as_str()] {
        let (word, actions) = level(&info, pool);
        assert_eq!(word, "work_together");
        assert!(actions.contains(&"post_message".to_owned()));
        assert!(!actions.contains(&"deliver_to_session".to_owned()));
    }
    let decide = |session: &SessionId, action| {
        pools::access::allowed(&state, session.as_str(), &team, action, None)
    };
    assert_eq!(decide(&planner, AgentAction::PostMessage), Decision::Allow);
    assert_eq!(
        decide(&planner, AgentAction::DeliverToSession),
        Decision::Ask {
            offer_always: false
        }
    );
    // An observer reads freely and asks for the rest.
    assert_eq!(decide(&watcher, AgentAction::ReadCards), Decision::Allow);
    assert!(matches!(
        decide(&watcher, AgentAction::PostMessage),
        Decision::Ask { .. }
    ));
    // Another pool's agent is denied here.
    assert_eq!(
        pools::access::allowed(
            &state,
            planner.as_str(),
            NO_PROJECT_POOL_ID,
            AgentAction::ReadRoster,
            None
        ),
        Decision::Deny
    );

    // Full lets messages into sessions; Read takes posting away.
    pools::access::set_level_impl(&state, &team, "full", &[]).unwrap();
    assert_eq!(
        decide(&planner, AgentAction::DeliverToSession),
        Decision::Allow
    );
    let info = pools::access::set_level_impl(&state, &team, "read", &[]).unwrap();
    assert_eq!(
        level(&info, &team).1,
        ["read_roster", "read_cards", "read_tasks", "read_messages"]
    );
    assert!(matches!(
        decide(&planner, AgentAction::PostMessage),
        Decision::Ask { .. }
    ));
    // Custom is the user's own set.
    let info = pools::access::set_level_impl(
        &state,
        &team,
        "custom",
        &["read_roster".to_owned(), "assign_task".to_owned()],
    )
    .unwrap();
    assert_eq!(
        level(&info, &team),
        (
            "custom".to_owned(),
            vec!["read_roster".to_owned(), "assign_task".to_owned()]
        )
    );
    assert_eq!(decide(&planner, AgentAction::AssignTask), Decision::Allow);

    // Allow once twice: the second ask offers Always allow; the rule
    // allows, and taking it back asks again on the very next call.
    pools::access::set_level_impl(&state, &team, "work_together", &[]).unwrap();
    // Staging a launch is Work together's (the Needs you card's click is
    // the gate); giving work to someone else still asks.
    assert_eq!(
        decide(&planner, AgentAction::PrepareLaunch),
        Decision::Allow
    );
    pools::access::answer_impl(&state, &team, "assign_task", "once").unwrap();
    assert_eq!(
        decide(&planner, AgentAction::AssignTask),
        Decision::Ask { offer_always: true }
    );
    let info = pools::access::answer_impl(&state, &team, "assign_task", "always").unwrap();
    let rule = info.pools.iter().find(|row| row.id == team).unwrap().rules[0].clone();
    assert_eq!(
        (rule.action.as_str(), rule.made_by.as_str()),
        ("assign_task", "user")
    );
    assert_eq!(decide(&planner, AgentAction::AssignTask), Decision::Allow);
    pools::access::revoke_impl(&state, rule.id).unwrap();
    assert!(matches!(
        decide(&planner, AgentAction::AssignTask),
        Decision::Ask { .. }
    ));
    assert!(matches!(
        pools::access::revoke_impl(&state, rule.id),
        Err(AppError::Refused(_))
    ));

    // D-29: delivery pauses at the hop limit (5 until the user changes
    // it), and the stop-all denies it.
    pools::access::set_level_impl(&state, &team, "full", &[]).unwrap();
    let deliver = |hop| {
        pools::access::allowed(
            &state,
            planner.as_str(),
            &team,
            AgentAction::DeliverToSession,
            Some(hop),
        )
    };
    assert_eq!(deliver(4), Decision::Allow);
    assert_eq!(
        deliver(5),
        Decision::Ask {
            offer_always: false
        }
    );
    let info = pools::access::set_delivery_impl(&state, &team, 8, true).unwrap();
    let row = info.pools.iter().find(|row| row.id == team).unwrap();
    assert_eq!(
        (row.access.hop_limit, row.access.delivery_stopped),
        (8, true)
    );
    assert_eq!(deliver(0), Decision::Deny);
    pools::access::set_delivery_impl(&state, &team, 8, false).unwrap();
    assert_eq!(deliver(7), Decision::Allow);
    assert!(matches!(
        pools::access::set_delivery_impl(&state, &team, 0, false),
        Err(AppError::Refused(_))
    ));
    pools::access::set_level_impl(&state, &team, "work_together", &[]).unwrap();

    // It sticks across a restart; unknown words are refused.
    drop(state);
    let state = app_state(dir.path());
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(level(&info, &team).0, "work_together");
    for (level, actions) in [
        ("everything", vec![]),
        ("custom", vec!["change_pool".to_owned()]),
    ] {
        assert!(matches!(
            pools::access::set_level_impl(&state, &team, level, &actions),
            Err(AppError::Validation { .. })
        ));
    }
    assert!(matches!(
        pools::access::answer_impl(&state, &team, "read_roster", "forever"),
        Err(AppError::Validation { .. })
    ));
    // Every request was audited.
    let audited = state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .audit(100)
        .unwrap();
    assert_eq!(audited.len(), 16);
}

/// The hard lines (D-28): the agent action catalog names no way to change
/// a pool, a level or a rule, start a session, pick an account or read a
/// credential; and the decision point only reads and audits.
#[test]
fn no_level_or_rule_can_reach_a_hard_line() {
    use juggler_core::agent_access::AgentAction;

    for action in AgentAction::ALL {
        let word = action.as_str();
        for line in [
            "pool",
            "level",
            "rule",
            "grant",
            "account",
            "credential",
            "token",
            "secret",
            "start",
            "spawn",
            "permission",
            "approve",
        ] {
            assert!(
                !word.contains(line),
                "the agent action {word} reaches a hard line ({line})"
            );
        }
    }
    // prepare_launch only stages a launch for the user's click: Work
    // together and Full allow it, Read never does; assigning work to
    // someone else is in no named level.
    use juggler_core::agent_access::AccessLevel;
    let none = Default::default();
    assert!(
        !AccessLevel::Read
            .actions(&none)
            .contains(&AgentAction::PrepareLaunch)
    );
    for level in [AccessLevel::WorkTogether, AccessLevel::Full] {
        assert!(level.actions(&none).contains(&AgentAction::PrepareLaunch));
        assert!(!level.actions(&none).contains(&AgentAction::AssignTask));
    }
    // The decision point writes nothing but the audit.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/features/pools/access.rs")).unwrap();
    let decision = &source[source.find("pub fn allowed(").unwrap()..];
    for call in [
        "set_level",
        "always_allow",
        "revoke(",
        "allow_once",
        "place(",
        "create(",
        "delete(",
        "rename(",
        "Keychain",
        "credential",
    ] {
        assert!(!decision.contains(call), "the decision point calls {call}");
    }
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
    pools::access::set_level_impl(&state, &team, "full", &[]).unwrap();
    pools::pool_rename_impl(&state, &team, "Team two").unwrap();
    pools::pools_list_impl(&state).unwrap();
    let _ = pools::pool_delete_impl(&state, NO_PROJECT_POOL_ID);
    let _ = pools::access::set_level_impl(&state, &team, "custom", &["read_roster".to_owned()]);
    let _ = pools::access::answer_impl(&state, &team, "deliver_to_session", "always");
    pools::pool_move_impl(&state, id, NO_PROJECT_POOL_ID).unwrap();
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
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("src/features/pools"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.push(root.join("../juggler-core/src/storage/pools.rs"));
    assert!(files.len() >= 5, "the pools module's files: {files:?}");
    for file in files {
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
