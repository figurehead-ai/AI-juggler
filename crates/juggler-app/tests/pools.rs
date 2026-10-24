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
use juggler_app::features::pools::{self, PoolsInfo};
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

    // All sessions: shop (work and personal together, D-33) and blog.
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools[0].scopes, 2);

    // A user pool with two projects' work: two, which the UI names.
    let info = pools::pool_create_impl(&state, "Mixed", None).unwrap();
    let mixed = pool_id(&info, "Mixed");
    pools::pool_move_impl(&state, w1.as_str(), &mixed).unwrap();
    let info = pools::pools_list_impl(&state).unwrap();
    assert_eq!(info.pools[1].scopes, 1);
    let info = pools::pool_move_impl(&state, b1.as_str(), &mixed).unwrap();
    assert_eq!(info.pools[1].scopes, 2);
    assert_eq!(info.pools[0].scopes, 1);
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
    assert!(!info.pools[0].spans_macs, "another pool unchanged");
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
    for pool in [DEFAULT_POOL_ID, team.as_str()] {
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
            DEFAULT_POOL_ID,
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
        ["read_roster", "read_cards", "read_tasks"]
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
    let _ = pools::pool_delete_impl(&state, DEFAULT_POOL_ID);
    let _ = pools::access::set_level_impl(&state, &team, "custom", &["read_roster".to_owned()]);
    let _ = pools::access::answer_impl(&state, &team, "deliver_to_session", "always");
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
