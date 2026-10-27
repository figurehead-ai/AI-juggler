//! E19 integration (AJ-E1901..E1904, E1906), on fake agents only.
//!
//! - Claims: N fake agents claim one task at once and exactly one wins.
//! - "Ready" is honest: a task an open task blocks can't be claimed, and
//!   is ready again once its blocker is done.
//! - Permissions (D-28): each level decides every task call (audited);
//!   a change above it is held for the user's OK in Needs you, and
//!   happens only if they allow it (once, or always: the second time).
//! - The rules: the lead plans epics and stories and gives tasks out; a
//!   member works what it holds, and gives it back with a note.
//! - D-31: an agent can only ask for a new session; only the user starts
//!   one, on the launch settings the user picks, and the task goes to it.
//! - Tasks follow the work: a card shows its member's task, and a real
//!   switch keeps the task (the switch history shows each session's).
//! - The issue a task comes from is looked up only on the user's click,
//!   through their own `gh`, and offline says so.
//!
//! D-44: the app writes no task, so these tests' tasks are written by the
//! pool's planner (a lead agent), as its `task_create` would.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

mod common;
#[path = "common/messages_lab.rs"]
#[allow(dead_code)]
mod lab;

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_agentbridge::wire::{
    Args, Call, Refusal, Response, TaskArgs, TaskAssignArgs, TaskCreateArgs, TaskUpdateArgs,
    TasksListArgs,
};
use juggler_app::features::pools;
use juggler_app::features::tasks::{apply, issues, launch, needs, views};
use juggler_app::paging::PageRequest;
use juggler_core::model::SessionId;
use lab::{Lab, field, planner, seed_task};

fn first_page() -> PageRequest {
    PageRequest::first(50)
}

fn create(lab: &Lab, key: &str, title: &str, kind: Option<&str>) -> Response {
    lab.call(
        key,
        Call::TaskCreate,
        Args {
            new_task: Some(TaskCreateArgs {
                title: title.into(),
                kind: kind.map(str::to_owned),
                ..TaskCreateArgs::default()
            }),
            ..Args::default()
        },
    )
}

fn claim(lab: &Lab, key: &str, task: &str) -> Response {
    lab.call(
        key,
        Call::TaskClaim,
        Args {
            task: Some(TaskArgs { task: task.into() }),
            ..Args::default()
        },
    )
}

fn update(lab: &Lab, key: &str, change: TaskUpdateArgs) -> Response {
    lab.call(
        key,
        Call::TaskUpdate,
        Args {
            task_change: Some(change),
            ..Args::default()
        },
    )
}

fn assign(lab: &Lab, key: &str, task: &str, to: &str) -> Response {
    lab.call(
        key,
        Call::TaskAssign,
        Args {
            assign: Some(TaskAssignArgs {
                task: task.into(),
                to: to.into(),
                note: None,
            }),
            ..Args::default()
        },
    )
}

fn list(lab: &Lab, key: &str, args: TasksListArgs) -> Vec<serde_json::Value> {
    let response = lab.call(
        key,
        Call::TasksList,
        Args {
            tasks: Some(args),
            ..Args::default()
        },
    );
    let shown = format!("{response:?}");
    response.result.unwrap_or_else(|| panic!("{shown}"))["tasks"]
        .as_array()
        .unwrap()
        .clone()
}

/// The planner adds a task to the lab's pool; its id.
fn task(lab: &Lab, title: &str, blocked_by: &[&str]) -> String {
    seed_task(
        &lab.state,
        &lab.pool,
        TaskCreateArgs {
            title: title.into(),
            acceptance: Some("- tests pass".into()),
            blocked_by: blocked_by.iter().map(|id| (*id).to_owned()).collect(),
            ..TaskCreateArgs::default()
        },
    )
}

/// The planner adds an item of `kind` (under `parent`); its id.
fn item(lab: &Lab, title: &str, kind: &str, parent: Option<&str>) -> String {
    seed_task(
        &lab.state,
        &lab.pool,
        TaskCreateArgs {
            title: title.into(),
            kind: Some(kind.into()),
            parent: parent.map(str::to_owned),
            ..TaskCreateArgs::default()
        },
    )
}

fn status_of(lab: &Lab, id: &str) -> String {
    views::detail(&lab.state, id, None).unwrap().task.status
}

#[test]
fn n_fake_agents_claim_one_task_and_exactly_one_wins() {
    let lab = Arc::new(Lab::new());
    let keys: Vec<String> = (0..6)
        .map(|n| lab.agent(&format!("s-{n}"), &format!("Builder {n}")).1)
        .collect();
    let id = task(&lab, "Build the API", &[]);
    let start = Arc::new(std::sync::Barrier::new(keys.len()));
    let handles: Vec<_> = keys
        .into_iter()
        .map(|key| {
            let (lab, id, start) = (Arc::clone(&lab), id.clone(), Arc::clone(&start));
            std::thread::spawn(move || {
                start.wait();
                claim(&lab, &key, &id)
            })
        })
        .collect();
    let outcomes: Vec<Response> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let won: Vec<&Response> = outcomes.iter().filter(|r| r.result.is_some()).collect();
    assert_eq!(won.len(), 1, "{outcomes:?}");
    assert_eq!(won[0].result.as_ref().unwrap()["claimed"], true);
    for lost in outcomes.iter().filter(|r| r.result.is_none()) {
        assert_eq!(lost.refused, Some(Refusal::Deny));
        assert!(
            lost.reason.as_deref().unwrap().contains("already"),
            "{lost:?}"
        );
    }
    let detail = views::detail(&lab.state, &id, None).unwrap();
    assert_eq!(detail.task.status, "in_progress");
    assert_eq!(
        detail
            .history
            .iter()
            .filter(|e| e.what == "took it")
            .count(),
        1,
        "one claim in the history"
    );
}

#[test]
fn ready_means_no_open_item_blocks_it() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Builder");
    let first = task(&lab, "Schema", &[]);
    let second = task(&lab, "Endpoints", &[&first]);
    let ready = list(
        &lab,
        &a,
        TasksListArgs {
            ready: true,
            ..TasksListArgs::default()
        },
    );
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0]["id"], first.as_str());
    assert_eq!(
        status_of(&lab, &second),
        "blocked",
        "honest: its blocker is open"
    );
    let refused = claim(&lab, &a, &second);
    assert_eq!(refused.refused, Some(Refusal::Deny));
    assert!(refused.reason.unwrap().contains(&first));
    // The first is done: the second is ready, and claimable.
    assert!(claim(&lab, &a, &first).result.is_some());
    let done = update(
        &lab,
        &a,
        TaskUpdateArgs {
            task: first.clone(),
            status: Some("done".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert!(done.result.is_some(), "{done:?}");
    assert_eq!(status_of(&lab, &second), "ready");
    assert!(claim(&lab, &a, &second).result.is_some());
    // The whole list puts ready work first.
    let all = list(&lab, &a, TasksListArgs::default());
    assert_eq!(all[0]["status"], "in_progress");
    assert_eq!(all.last().unwrap()["status"], "done");
}

#[test]
fn ready_lists_only_tasks_never_epics_or_stories() {
    // Lab 2026-10-26, bug 1: an epic or story that is Ready isn't work an
    // agent takes, so `ready: true` lists only the task.
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Builder");
    for kind in ["epic", "story"] {
        item(&lab, kind, kind, None);
    }
    let only = task(&lab, "Write hello.txt", &[]);
    let ready = list(
        &lab,
        &a,
        TasksListArgs {
            ready: true,
            ..TasksListArgs::default()
        },
    );
    let ids: Vec<_> = ready.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [only.as_str()]);
    assert_eq!(
        list(&lab, &a, TasksListArgs::default()).len(),
        3,
        "the whole list still has all three"
    );
}

#[test]
fn each_level_decides_and_a_change_above_it_waits_on_the_user() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Planner");
    pools::access::set_level_impl(&lab.state, &lab.pool, "read", &[]).unwrap();
    // Read: looking is fine; adding a task waits on the user.
    assert!(list(&lab, &a, TasksListArgs::default()).is_empty());
    let held = create(&lab, &a, "Build the API", None);
    assert_eq!(held.refused, Some(Refusal::Ask));
    assert!(held.reason.unwrap().contains("held for the user's OK"));
    let waiting = needs::needs(&lab.state).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].kind, "ask");
    assert!(waiting[0].title.contains("Planner"), "{:?}", waiting[0]);
    assert!(!waiting[0].offer_always, "not before the second approval");
    assert!(
        views::list(&lab.state, &lab.pool, None, None, &first_page())
            .unwrap()
            .items
            .is_empty(),
        "nothing happens before the user's OK"
    );
    needs::answer(&lab.state, &waiting[0].id, "once").unwrap();
    let tasks = views::list(&lab.state, &lab.pool, None, None, &first_page())
        .unwrap()
        .items;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].written_by, "Planner", "made as the agent's own");
    // The second ask offers a rule; Always allow makes one.
    create(&lab, &a, "Build the cart", None);
    let second = needs::needs(&lab.state).unwrap();
    assert!(second[0].offer_always);
    needs::answer(&lab.state, &second[0].id, "always").unwrap();
    assert!(
        create(&lab, &a, "Build checkout", None).result.is_some(),
        "the rule allows it now"
    );
    // A no leaves the task as it was.
    let id = tasks[0].id.clone();
    assert_eq!(claim(&lab, &a, &id).refused, Some(Refusal::Ask));
    let ask = needs::needs(&lab.state).unwrap().remove(0);
    needs::answer(&lab.state, &ask.id, "deny").unwrap();
    assert_eq!(status_of(&lab, &id), "ready");
    assert!(needs::needs(&lab.state).unwrap().is_empty());
    // Every call reached the decision point and is in the audit.
    let audit = lab
        .state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .audit(100)
        .unwrap();
    let ours: Vec<_> = audit.iter().filter(|r| r.actor == "session:s-a").collect();
    for action in ["read_tasks", "create_task", "claim_task"] {
        assert!(
            ours.iter().any(|r| r.action == action),
            "{action}: {ours:?}"
        );
    }
    assert!(
        ours.iter()
            .any(|r| r.action == "create_task" && r.decision == "ask")
    );
    assert!(
        ours.iter()
            .any(|r| r.action == "create_task" && r.decision == "allow")
    );
}

#[test]
fn the_lead_plans_and_gives_tasks_out_and_members_work_what_they_hold() {
    let lab = Lab::new();
    let (lead_id, lead) = lab.agent("s-lead", "Planner");
    let (builder_id, builder) = lab.agent("s-b", "Builder");
    pools::pool_set_role_impl(&lab.state, lead_id.as_str(), "lead").unwrap();
    // Only the lead plans epics and stories.
    let refused = create(&lab, &builder, "Checkout", Some("epic"));
    assert_eq!(refused.refused, Some(Refusal::Deny));
    assert!(refused.reason.unwrap().contains("lead"));
    let epic = field(&create(&lab, &lead, "Checkout", Some("epic")), "/task/id");
    assert!(epic.starts_with("aj-"), "{epic}");
    let made = create(&lab, &builder, "Build the API", None);
    let id = field(&made, "/task/id");
    // Giving it out is the lead's, and asks at every named level.
    let not_lead = assign(&lab, &builder, &id, lead_id.as_str());
    assert_eq!(not_lead.refused, Some(Refusal::Deny));
    let held = assign(&lab, &lead, &id, builder_id.as_str());
    assert_eq!(held.refused, Some(Refusal::Ask), "{held:?}");
    let ask = needs::needs(&lab.state).unwrap().remove(0);
    // The held assignment names the member, never its session's id (lab
    // 2026-10-26, bug 2), in Needs you and in the task's history.
    assert!(
        ask.title.ends_with("to Builder") && !ask.title.contains(builder_id.as_str()),
        "the ask names the member"
    );
    let named = format!("give {id} to Builder");
    let history = views::detail(&lab.state, &id, None).unwrap().history;
    assert!(
        history.iter().any(|e| e.what.contains(&named)),
        "the history names the member"
    );
    assert!(
        !history.iter().any(|e| e.what.contains(builder_id.as_str())),
        "no session id in the history"
    );
    let stranger = assign(&lab, &lead, &id, "s-nobody-here");
    assert_eq!(stranger.refused, Some(Refusal::Deny), "not in the pool");
    assert_eq!(
        needs::needs(&lab.state).unwrap().len(),
        1,
        "nothing more held"
    );
    needs::answer(&lab.state, &ask.id, "once").unwrap();
    let mine = list(
        &lab,
        &builder,
        TasksListArgs {
            mine: true,
            ..TasksListArgs::default()
        },
    );
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["yours"], true);
    // The builder works it; someone else's task isn't its to change.
    assert!(claim(&lab, &builder, &id).result.is_some());
    let other = field(&create(&lab, &lead, "Docs", None), "/task/id");
    assert!(claim(&lab, &lead, &other).result.is_some());
    let refused = update(
        &lab,
        &builder,
        TaskUpdateArgs {
            task: other.clone(),
            status: Some("done".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert_eq!(refused.refused, Some(Refusal::Deny));
    assert!(refused.reason.unwrap().contains("someone else's"));
    let cancel = update(
        &lab,
        &builder,
        TaskUpdateArgs {
            task: id.clone(),
            status: Some("cancelled".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert_eq!(cancel.refused, Some(Refusal::Deny), "only the lead cancels");
    let review = update(
        &lab,
        &builder,
        TaskUpdateArgs {
            task: id.clone(),
            status: Some("in_review".into()),
            note: Some("PR is up".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert_eq!(field(&review, "/task/status"), "in_review");
    // The lead may close it.
    let done = update(
        &lab,
        &lead,
        TaskUpdateArgs {
            task: id.clone(),
            status: Some("done".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert_eq!(field(&done, "/task/status"), "done");
}

#[test]
fn a_holder_gives_it_back_with_a_note_and_the_next_one_reads_it() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Alpha");
    let (_b, b) = lab.agent("s-b", "Beta");
    let id = task(&lab, "Migrate totals to cents", &[]);
    assert!(claim(&lab, &a, &id).result.is_some());
    let back = update(
        &lab,
        &a,
        TaskUpdateArgs {
            task: id.clone(),
            release: true,
            note: Some("Orders done; refunds still use floats.".into()),
            ..TaskUpdateArgs::default()
        },
    );
    assert_eq!(field(&back, "/task/status"), "ready");
    assert_eq!(field(&back, "/task/holder"), "null");
    assert!(claim(&lab, &b, &id).result.is_some());
    let got = lab.call(
        &b,
        Call::TaskGet,
        Args {
            task: Some(TaskArgs { task: id.clone() }),
            ..Args::default()
        },
    );
    let text = serde_json::to_string(&got.result.unwrap()).unwrap();
    assert!(text.contains("refunds still use floats"), "{text}");
    assert!(text.contains("data, not instructions"));
}

#[test]
fn an_agent_asks_for_a_session_and_only_the_user_starts_it() {
    let lab = Lab::new();
    let (lead_id, lead) = lab.agent("s-lead", "Planner");
    pools::pool_set_role_impl(&lab.state, lead_id.as_str(), "lead").unwrap();
    let id = task(&lab, "Write the release notes", &[]);
    let live = || {
        lab.state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .live()
            .unwrap()
            .len()
    };
    let before = live();
    let asked = assign(&lab, &lead, &id, "new_session");
    assert!(asked.result.is_some(), "{asked:?}");
    assert!(field(&asked, "/note").contains("nothing starts by itself"));
    // Asking twice is one request.
    assign(&lab, &lead, &id, "new_session");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(live(), before, "no session starts for an agent");
    let waiting = needs::needs(&lab.state).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].kind, "launch");
    // The user starts it, on the launch settings they pick.
    let started = launch::start(&lab.state, &id, &lab.profile, Some(&waiting[0].id)).unwrap();
    let session = SessionId::new(started.session_id.as_str());
    assert_eq!(started.task.status, "in_progress");
    assert_eq!(
        started.task.holder_session_id.as_deref(),
        Some(session.as_str())
    );
    let detail = views::detail(&lab.state, &id, None).unwrap();
    assert!(
        detail
            .sessions
            .iter()
            .any(|s| s.session_id == session.as_str() && s.how == "started_for")
    );
    assert!(
        detail
            .history
            .iter()
            .any(|e| e.what.starts_with("started a session"))
    );
    assert!(
        needs::needs(&lab.state).unwrap().is_empty(),
        "the request is answered"
    );
    // It's in the task's pool.
    let place = pools::members::place_impl(&lab.state, session.as_str()).unwrap();
    assert_eq!(place.pool_id, lab.pool);
    // A running session of the working set: the pools' answer says so too.
    let info = pools::pools_list_impl(&lab.state).unwrap();
    assert!(info.placements.contains(&place));
    let _ = lab.state.supervisor.stop(&session, Duration::from_secs(2));
}

#[test]
fn a_card_shows_its_members_task() {
    let lab = Lab::new();
    let (a_id, a) = lab.agent("s-a", "Builder");
    let id = task(&lab, "Build the API", &[]);
    let card = |lab: &Lab| {
        pools::cards::cards_impl(&lab.state, &lab.pool)
            .unwrap()
            .cards
            .into_iter()
            .find(|c| c.session_id == a_id.as_str())
            .unwrap()
            .task
    };
    assert_eq!(card(&lab).value.as_deref(), Some("None"));
    assert!(claim(&lab, &a, &id).result.is_some());
    assert_eq!(
        card(&lab).value.as_deref(),
        Some(format!("{id} · Build the API (in progress)").as_str())
    );
    let mine = views::of_session(&lab.state, a_id.as_str(), &first_page()).unwrap();
    assert_eq!(mine.items[0].id, id);
}

#[test]
fn the_lead_plans_an_epic_with_stories_and_the_user_sees_the_tree() {
    let lab = Lab::new();
    let epic = item(&lab, "Checkout redesign", "epic", None);
    let mut stories = Vec::new();
    for title in ["Address step", "Shipping step", "Payment step"] {
        stories.push(item(&lab, title, "story", Some(&epic)));
    }
    item(&lab, "Validate postcodes", "task", Some(&stories[0]));
    assert!(
        apply::create(
            &lab.state,
            &planner(),
            &lab.pool,
            &TaskCreateArgs {
                title: "Wrong".into(),
                kind: Some("story".into()),
                parent: Some(stories[0].clone()),
                ..TaskCreateArgs::default()
            },
        )
        .is_err(),
        "a story goes under an epic"
    );
    let plan = views::plan(&lab.state, &lab.pool, &first_page()).unwrap();
    assert_eq!(plan.items.len(), 4);
    let detail = views::detail(&lab.state, &epic, None).unwrap();
    assert_eq!(detail.children.len(), 3);
    assert_eq!((detail.task.items, detail.task.items_done), (3, 0));
    let under = views::children(&lab.state, &lab.pool, &stories[0], &first_page()).unwrap();
    assert_eq!(under.items.len(), 1);
}

#[test]
fn two_hundred_tasks_list_fast_and_page() {
    let lab = Lab::new();
    for n in 0..230 {
        task(&lab, &format!("Task {n}"), &[]);
    }
    let started = Instant::now();
    let first = views::list(&lab.state, &lab.pool, None, None, &PageRequest::first(200)).unwrap();
    let took = started.elapsed();
    assert_eq!(first.items.len(), 200);
    assert!(took < Duration::from_millis(100), "{took:?}");
    // Keyset pages (paging.rs): every task once, none twice.
    let mut seen: std::collections::HashSet<String> =
        first.items.iter().map(|t| t.id.clone()).collect();
    let mut next = first.next_cursor;
    while let Some(cursor) = next {
        let page = views::list(
            &lab.state,
            &lab.pool,
            None,
            None,
            &PageRequest::after(&cursor, 50),
        )
        .unwrap();
        for task in &page.items {
            assert!(seen.insert(task.id.clone()), "{} twice", task.id);
        }
        next = page.next_cursor;
    }
    assert_eq!(seen.len(), 230);
    let counts = views::counts(&lab.state, &lab.pool).unwrap();
    assert_eq!((counts.ready, counts.done), (230, 0));
}

/// A stand-in for the user's `gh`, printing what GitHub would.
fn fake_gh(dir: &std::path::Path, out: &str, code: i32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("gh");
    std::fs::write(&path, format!("#!/bin/sh\nprintf '{out}'\nexit {code}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn the_issue_is_looked_up_only_on_a_click_and_offline_says_so() {
    let lab = Lab::new();
    let dir = tempfile::tempdir().unwrap();
    let id = seed_task(
        &lab.state,
        &lab.pool,
        TaskCreateArgs {
            title: "Fix the cart".into(),
            url: Some("https://github.com/figurehead/shop/issues/42".into()),
            ..TaskCreateArgs::default()
        },
    );
    // The planner points it elsewhere, as its `task_update` would.
    let relink = |url: &str| {
        apply::update(
            &lab.state,
            &planner(),
            &lab.pool,
            &TaskUpdateArgs {
                task: id.clone(),
                url: Some(url.into()),
                ..TaskUpdateArgs::default()
            },
        )
    };
    // Nothing is looked up by itself.
    let detail = views::detail(&lab.state, &id, None).unwrap();
    let issue = detail.issue.unwrap();
    assert_eq!(issue.label.as_deref(), Some("figurehead/shop#42"));
    assert_eq!((issue.title, issue.checked_at_ms), (None, None));
    // Offline (gh fails): it says so, and keeps the link.
    lab.state.tasks.set_gh(Some(fake_gh(dir.path(), "", 1)));
    let offline = issues::look_up(&lab.state, &id).unwrap_err().to_string();
    assert!(offline.contains("offline"), "{offline}");
    assert_eq!(
        views::detail(&lab.state, &id, None)
            .unwrap()
            .issue
            .unwrap()
            .title,
        None
    );
    // Online: its title and state, kept with when.
    let online = tempfile::tempdir().unwrap();
    lab.state.tasks.set_gh(Some(fake_gh(
        online.path(),
        "Cart drops items on refresh\\topen\\n",
        0,
    )));
    let detail = issues::look_up(&lab.state, &id).unwrap();
    let issue = detail.issue.unwrap();
    assert_eq!(issue.title.as_deref(), Some("Cart drops items on refresh"));
    assert_eq!(issue.state.as_deref(), Some("open"));
    assert!(issue.checked_at_ms.is_some());
    assert_eq!(issue.look_up_note, None);
    // A gh that's there but that others can change isn't run, and the
    // words say why, never that gh is missing (lab 2026-10-26, bug 3).
    use std::os::unix::fs::PermissionsExt;
    let shared = tempfile::tempdir().unwrap();
    let unsafe_gh = fake_gh(shared.path(), "", 0);
    std::fs::set_permissions(shared.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    lab.state.tasks.set_gh(Some(unsafe_gh));
    let refused = issues::look_up(&lab.state, &id).unwrap_err().to_string();
    assert!(refused.contains("won't run it") && refused.contains("other users can change"));
    assert!(!refused.contains(issues::NO_GH), "it isn't missing");
    let issue = views::detail(&lab.state, &id, None).unwrap().issue.unwrap();
    assert!(!issue.can_look_up);
    assert!(issue.look_up_note.unwrap().contains("won't run it"));
    std::fs::set_permissions(shared.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    // No gh at all says so.
    lab.state
        .tasks
        .set_gh(Some(shared.path().join("not-installed")));
    let missing = issues::look_up(&lab.state, &id).unwrap_err().to_string();
    assert!(missing.contains(issues::NO_GH));
    // A Linear link names its issue from the link, and isn't looked up.
    relink("https://linear.app/acme/issue/ENG-123/cart").unwrap();
    let issue = views::detail(&lab.state, &id, None).unwrap().issue.unwrap();
    assert_eq!(issue.label.as_deref(), Some("ENG-123"));
    assert!(!issue.can_look_up);
    assert!(issues::look_up(&lab.state, &id).is_err());
    // Only https links are kept.
    assert!(relink("javascript:alert(1)").is_err());
}
