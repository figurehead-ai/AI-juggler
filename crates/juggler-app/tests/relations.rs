//! AJ-E3102: every page's counts from the relation-count cache.
//!
//! On sessions AI Juggler ran and conversations another app started (a
//! real Claude Code folder's transcripts), over projects, folders, Apps
//! and Accounts:
//! - K2: every count equals the list it heads (the sessions page's total,
//!   for all, open, ended and archived) and a recount from the rows;
//! - a change no event announced still shows in the next count (the
//!   database's stamps), and a conversation read from the files too;
//! - X1: Clear cache mid-walk: the next count is built again and equal;
//!   Settings names the cache and its size;
//! - K2 (AJ-E3103): every list opens on what needs you, then what's
//!   working, then what's open, then by last activity; each filter its
//!   list offers narrows it to as many rows as the filter says, each one
//!   of the list's, and its heading agrees;
//! - the budgets: 10 000 sessions rebuilt cold in ≤ 1 s, any count read
//!   from the cache in ≤ 5 ms (each read prints a `[perf]` line). This
//!   binary runs alone (`.config/nextest.toml`, "Wall-clock budgets").

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::cache;
use juggler_app::features::conversations;
use juggler_app::features::relations::{self, EntityKind, EntityRef, RelationCountsInfo};
use juggler_app::features::sessions::facets::sessions_facets_impl;
use juggler_app::features::sessions::rows::{
    Rank, SessionRowInfo, SessionsQuery, sessions_counts_impl, sessions_counts_scanned,
    sessions_page_impl,
};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};
use juggler_core::storage::InstanceColumn;

const PROJECTS: usize = 12;
const FOLDERS: usize = 4;

struct Fixture {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    config: PathBuf,
    repos: Vec<PathBuf>,
    projects: Vec<String>,
    /// The imported Claude Code folder whose files hold the outside ones.
    outside_folder: String,
}

fn transcript(config: &Path, cwd: &Path, id: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"gitBranch\":\"main\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Work on it\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

fn folder(id: &str, dir: &Path) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
        label: id.to_owned(),
        plan: None,
        state_dir: dir.join(id),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

fn record(n: usize, project: &str, status: SessionStatus) -> InstanceRecord {
    let at = 1_760_000_000_000 + i64::try_from(n).unwrap() * 60_000;
    InstanceRecord {
        id: SessionId::new(format!("s-{n:05}")),
        conversation: None,
        profile: ProfileId::new(format!("rec-{}", n % FOLDERS)),
        machine: MachineId::new(if n.is_multiple_of(7) {
            "mac-other"
        } else {
            "mac-here"
        }),
        kind: InstanceKind::Terminal,
        project: (!n.is_multiple_of(9)).then(|| ProjectId::new(project)),
        cwd: PathBuf::from("/nowhere"),
        status,
        started_at: Timestamp::from_millis(at),
        last_activity: Some(Timestamp::from_millis(at + 1_000)),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

/// `own` sessions AI Juggler ran over [`FOLDERS`] folders and
/// [`PROJECTS`] projects (one in 25 open, one in 40 archived, one in 9
/// with no project, one in 7 on another Mac), and `outside` conversations
/// in a Claude Code folder's files over the same projects. Folders 0 and
/// 1 and the Claude Code one are signed in to one Account, 2 to another,
/// 3 to none.
fn fixture(own: usize, outside: usize) -> Fixture {
    // The system's temporary folder, never the build's: a build folder
    // inside a checkout (preflight's `target/`) folds every folder here
    // into that repository's project, the real one.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let repos: Vec<PathBuf> = (0..PROJECTS)
        .map(|n| {
            let repo = root.join(format!("Dev/repo-{n:02}"));
            std::fs::create_dir_all(repo.join(".git")).unwrap();
            assert_eq!(
                juggler_core::storage::project_root(&repo),
                repo,
                "the fixture's repositories must be their own projects, inside no other"
            );
            repo
        })
        .collect();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    for n in 0..outside {
        transcript(&config, &repos[n % PROJECTS], &format!("c-{n:05}"));
    }
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    cache::register_defaults(&state);
    let outside_folder = commands::account_import_mode_impl(
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
    // The tools' files read, and their folders made projects.
    // The tools' files read (each repository is made a project below;
    // the backfill would read the real home's worktree managers).
    conversations::refresh_impl(&state).unwrap();
    let started = Instant::now();
    let projects: Vec<String> = {
        let storage = state.storage.lock().unwrap();
        let projects: Vec<String> = repos
            .iter()
            .map(|repo| storage.projects().touch(repo, None).unwrap().id.to_string())
            .collect();
        for n in 0..FOLDERS {
            let id = format!("acc-{n}");
            storage.accounts().insert(&folder(&id, &root)).unwrap();
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new(format!("rec-{n}")),
                    name: "Main".into(),
                    account: AccountId::new(id.as_str()),
                    settings_overlay: SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
        }
        let a = storage.subscriptions().upsert("anthropic", "a", 1).unwrap();
        let b = storage.subscriptions().upsert("anthropic", "b", 1).unwrap();
        for (folder, account) in [
            ("acc-0", &a),
            ("acc-1", &a),
            ("acc-2", &b),
            (outside_folder.as_str(), &a),
        ] {
            storage
                .subscriptions()
                .set_folder(folder, Some(account))
                .unwrap();
        }
        for n in 0..own {
            let status = if n.is_multiple_of(25) {
                SessionStatus::Running
            } else {
                SessionStatus::Finished
            };
            storage
                .sessions()
                .insert(&record(n, &projects[n % PROJECTS], status))
                .unwrap();
        }
        for n in (1..own).step_by(40) {
            storage
                .sessions()
                .record_lifecycle(&SessionId::new(format!("s-{n:05}")), "archived")
                .unwrap();
        }
        projects
    };
    eprintln!(
        "[perf] fixture: {own} sessions, {outside} outside conversations in {:?}",
        started.elapsed()
    );
    Fixture {
        _dir: dir,
        state,
        config,
        repos,
        projects,
        outside_folder,
    }
}

fn entity(kind: EntityKind, id: &str) -> EntityRef {
    EntityRef {
        kind,
        id: id.to_owned(),
    }
}

/// The query whose list an entity's Sessions tab shows.
fn query_of(entity: &EntityRef) -> SessionsQuery {
    let mut query = SessionsQuery::default();
    let id = Some(entity.id.clone());
    match entity.kind {
        EntityKind::All => {}
        EntityKind::Project => query.project_id = id,
        EntityKind::Folder => query.account_id = id,
        EntityKind::App => query.app_id = id,
        EntityKind::Subscription => query.subscription_id = id,
        other => panic!("{other:?} heads no sessions list yet"),
    }
    query
}

/// Every page that heads a list of sessions in the fixture.
fn pages(fx: &Fixture) -> Vec<EntityRef> {
    let mut pages = vec![entity(EntityKind::All, "")];
    pages.extend(fx.projects.iter().map(|id| entity(EntityKind::Project, id)));
    let folders: Vec<String> = (0..FOLDERS)
        .map(|n| format!("acc-{n}"))
        .chain([fx.outside_folder.clone()])
        .collect();
    for id in &folders {
        pages.push(entity(EntityKind::Folder, id));
        pages.push(entity(
            EntityKind::App,
            &juggler_core::storage::cli_app_id(id),
        ));
    }
    let accounts: std::collections::BTreeSet<String> = fx
        .state
        .storage
        .lock()
        .unwrap()
        .subscriptions()
        .folders()
        .unwrap()
        .into_iter()
        .filter_map(|(_, account)| account)
        .collect();
    pages.extend(
        accounts
            .iter()
            .map(|id| entity(EntityKind::Subscription, id)),
    );
    pages
}

/// How many rows the list `query` narrowed to `which` holds, page by page.
fn listed(state: &Arc<AppState>, query: &SessionsQuery, which: Option<&str>) -> u64 {
    let query = SessionsQuery {
        which: which.map(str::to_owned),
        ..query.clone()
    };
    rows_of(state, &query, 200).len() as u64
}

/// Every row of `query`'s list, in order, `limit` a page.
fn rows_of(state: &Arc<AppState>, query: &SessionsQuery, limit: u32) -> Vec<SessionRowInfo> {
    let mut rows = Vec::new();
    let mut cursor = None;
    loop {
        let page = sessions_page_impl(
            state,
            query,
            &PageRequest {
                cursor,
                limit: Some(limit),
            },
        )
        .unwrap();
        rows.extend(page.items);
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return rows,
        }
    }
}

fn assert_agrees(state: &Arc<AppState>, page: &EntityRef, counts: &RelationCountsInfo) {
    let query = query_of(page);
    let reference = sessions_counts_scanned(state, &query).unwrap();
    assert_eq!(counts.sessions, reference, "{page:?}: the recount");
    assert_eq!(
        sessions_counts_impl(state, &query).unwrap(),
        reference,
        "{page:?}: the list's heading"
    );
    assert_eq!(
        listed(state, &query, None),
        counts.sessions.all,
        "{page:?}: all"
    );
    for (which, n) in [
        ("open", counts.sessions.open),
        ("ended", counts.sessions.ended),
        ("archived", counts.sessions.archived),
    ] {
        assert_eq!(listed(state, &query, Some(which)), n, "{page:?}: {which}");
    }
}

/// K2 (L2): every page's count equals the list it heads and a recount
/// from the rows, AI Juggler's sessions and the outside ones together.
#[test]
fn every_count_equals_the_list_it_heads_and_a_recount() {
    let fx = fixture(600, 60);
    let state = &fx.state;
    let pages = pages(&fx);
    let counts = relations::counts_many(state, &pages).unwrap();
    assert_eq!(counts.len(), pages.len());
    for (page, counts) in pages.iter().zip(&counts) {
        assert_agrees(state, page, counts);
    }
    // The whole: 600 of AI Juggler's and 60 outside, one in 25 open, one
    // in 40 archived.
    let all = &counts[0];
    assert_eq!(all.sessions.all, 660);
    assert_eq!(all.sessions.open, 24);
    assert_eq!(all.sessions.archived, 15);
    assert_eq!(all.projects, PROJECTS as u64);
    assert_eq!(all.folders, FOLDERS as u64 + 1);
    assert_eq!(all.subscriptions, 2);
    assert_eq!(all.machines, 3, "two Macs AI Juggler ran on, and this one");
    // A project's relations: each folder and its App (sessions n with
    // n % 12 == p meet folders n % 4), and the outside folder.
    let p0 = &counts[1];
    assert_eq!(p0.apps, 2, "folder 0's App and the outside one");
    assert_eq!(p0.folders, 2);
    assert_eq!(p0.tools, 1);
    // The cheaper kinds against the storage's own indexes.
    {
        let storage = state.storage.lock().unwrap();
        let relations_repo = storage.relations();
        for n in 0..FOLDERS {
            let id = format!("rec-{n}");
            let page = entity(EntityKind::LaunchSettings, &id);
            assert_eq!(
                relations::counts(state, &page).unwrap().sessions.all,
                relations_repo
                    .count_where(InstanceColumn::LaunchSettings, &id)
                    .unwrap()
            );
        }
        let other = relations::counts(state, &entity(EntityKind::Machine, "mac-other")).unwrap();
        assert_eq!(
            other.sessions.all,
            relations_repo
                .count_where(InstanceColumn::Machine, "mac-other")
                .unwrap()
        );
    }
    let tool = relations::counts(state, &entity(EntityKind::Tool, "claude-code")).unwrap();
    assert_eq!(tool.sessions.all, 660);
    // The conversation index finds the outside ones by project and tool.
    let index = relations::conversation_index(state).unwrap();
    assert_eq!(index.facets().len(), 60);
    assert_eq!(index.of_project(&fx.projects[0]).count(), 5);
    assert_eq!(index.of_tool("claude-code").count(), 60);
}

/// A change shows in the next count: one no event announced (straight to
/// the database), and a conversation the next pass reads from the files.
#[test]
fn a_change_shows_in_the_next_count() {
    let fx = fixture(100, 12);
    let state = &fx.state;
    let project = entity(EntityKind::Project, &fx.projects[3]);
    let before = relations::counts(state, &project).unwrap();
    let builds = state.relations.builds();
    // Read again with nothing changed: from the cache, nothing built.
    assert_eq!(relations::counts(state, &project).unwrap(), before);
    assert_eq!(state.relations.builds(), builds);

    // A session started and another archived, told to no one.
    {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&record(9_001, &fx.projects[3], SessionStatus::Running))
            .unwrap();
        storage
            .sessions()
            .record_lifecycle(&SessionId::new("s-00003"), "archived")
            .unwrap();
    }
    let after = relations::counts(state, &project).unwrap();
    assert_eq!(after.sessions.all, before.sessions.all + 1);
    assert_eq!(after.sessions.open, before.sessions.open + 1);
    assert_eq!(after.sessions.archived, before.sessions.archived + 1);
    assert_agrees(state, &project, &after);

    // A conversation written in the project's folder, read by the pass.
    transcript(&fx.config, &fx.repos[3], "c-new");
    conversations::refresh_impl(state).unwrap();
    let read = relations::counts(state, &project).unwrap();
    assert_eq!(read.sessions.all, after.sessions.all + 1);
    assert_agrees(state, &project, &read);
}

/// X1: Clear cache mid-walk. Settings names the cache with its size; the
/// clear empties it; the next count is built again and the same.
#[test]
fn a_cache_cleared_mid_walk_comes_back_the_same() {
    let fx = fixture(300, 24);
    let state = &fx.state;
    let pages = pages(&fx);
    let walked: Vec<RelationCountsInfo> = pages
        .iter()
        .take(4)
        .map(|page| relations::counts(state, page).unwrap())
        .collect();
    let row = |info: &juggler_app::features::cache::dto::CacheInfo| {
        info.caches
            .iter()
            .find(|row| row.key == relations::cache::ID)
            .cloned()
            .expect("the counts are one of the local caches")
    };
    let kept = row(&cache::status_impl(state).unwrap());
    assert_eq!(kept.name, "Counts on pages");
    assert!(kept.bytes > 0 && kept.entries == 1, "{kept:?}");
    let cleared = row(&cache::clear_impl(state).unwrap());
    assert_eq!((cleared.bytes, cleared.entries), (0, 0));
    let builds = state.relations.builds();
    // The walk goes on: each count comes back, built once, the same.
    for (page, was) in pages.iter().take(4).zip(&walked) {
        assert_eq!(&relations::counts(state, page).unwrap(), was);
    }
    assert_eq!(state.relations.builds(), builds + 2, "both parts, once");
    for page in pages.iter().skip(4) {
        let counts = relations::counts(state, page).unwrap();
        assert_agrees(state, page, &counts);
    }
    // Off: nothing is kept, and the counts are still right.
    cache::set_on_impl(state, false).unwrap();
    assert!(state.relations.kept().is_none());
    assert_eq!(&relations::counts(state, &pages[0]).unwrap(), &walked[0]);
    assert!(state.relations.kept().is_none(), "nothing kept while off");
}

/// The budgets: 10 000 sessions (9 000 of AI Juggler's, 1 000 outside)
/// rebuilt cold within 1 s; any count from the cache within 5 ms.
#[test]
fn ten_thousand_sessions_count_cold_within_a_second_and_cached_within_5_ms() {
    let fx = fixture(9_000, 1_000);
    let state = &fx.state;
    let pages = pages(&fx);
    let started = Instant::now();
    let all = relations::counts(state, &pages[0]).unwrap();
    let cold = started.elapsed();
    eprintln!("[perf] cold rebuild of 10 000 sessions: {cold:?}");
    assert_eq!(all.sessions.all, 10_000);
    assert!(cold < Duration::from_secs(1), "cold rebuild took {cold:?}");
    // Before: a project's heading counted every row on each ask.
    let started = Instant::now();
    sessions_counts_scanned(state, &query_of(&pages[1])).unwrap();
    eprintln!(
        "[perf] before: one project's counts from every row: {:?}",
        started.elapsed()
    );
    let mut slowest = Duration::ZERO;
    for _ in 0..20 {
        for page in &pages {
            let started = Instant::now();
            let counts = relations::counts(state, page).unwrap();
            slowest = slowest.max(started.elapsed());
            assert!(counts.sessions.all > 0, "{page:?}");
        }
    }
    eprintln!(
        "[perf] slowest of {} cached counts: {slowest:?}",
        20 * pages.len()
    );
    assert!(
        slowest < Duration::from_millis(5),
        "a cached count took {slowest:?}"
    );
    // One ask for every page at once, from the cache too.
    let started = Instant::now();
    let many = relations::counts_many(state, &pages).unwrap();
    let took = started.elapsed();
    eprintln!("[perf] {} pages' counts in one ask: {took:?}", pages.len());
    assert_eq!(many.len(), pages.len());
    assert!(took < Duration::from_millis(5), "one ask took {took:?}");
    // AJ-E3103: a project's list's first page, and its filters' values.
    let project = query_of(&pages[1]);
    let started = Instant::now();
    sessions_page_impl(
        state,
        &project,
        &PageRequest {
            cursor: None,
            limit: Some(50),
        },
    )
    .unwrap();
    eprintln!(
        "[perf] a project's first page of 50, of 10 000: {:?}",
        started.elapsed()
    );
    let started = Instant::now();
    sessions_facets_impl(state, &project).unwrap();
    eprintln!(
        "[perf] a project's filters' values, of 10 000: {:?}",
        started.elapsed()
    );
    // A change: only the stale part is built again.
    let builds = state.relations.builds();
    relations::changed(state, relations::Change::ConversationsLive);
    let started = Instant::now();
    relations::counts(state, &pages[0]).unwrap();
    eprintln!("[perf] the outside part rebuilt: {:?}", started.elapsed());
    assert_eq!(state.relations.builds(), builds + 1);
}

/// A row's place in the order, by what it shows (the UI's bands), and
/// when it was last active.
fn order_of(row: &SessionRowInfo) -> (Rank, i64) {
    if let Some(session) = &row.session {
        let status = SessionStatus::parse(&session.status).unwrap();
        let runtime = session
            .runtime
            .as_ref()
            .and_then(|info| info.state.as_deref());
        let last = session.last_activity_ms.unwrap_or(session.started_at_ms);
        return (Rank::of_session(status, runtime), last);
    }
    let conversation = row.conversation.as_ref().unwrap();
    let last = conversation
        .last_activity_ms
        .or(conversation.started_ms)
        .unwrap_or_default();
    (
        Rank::of_conversation(conversation.live, conversation.state.as_deref()),
        last,
    )
}

/// K2 (AJ-E3103): every page's list opens on what needs you, then what's
/// working, then what's open, then the rest by last activity, across its
/// pages; every value each of its filters offers narrows it to that many
/// rows, all of them the list's own, with a heading that agrees; "No
/// project", one Mac and "since" narrow it too.
#[test]
fn every_list_opens_on_what_needs_you_and_every_filter_narrows_it() {
    let fx = fixture(240, 24);
    let state = &fx.state;
    {
        let storage = state.storage.lock().unwrap();
        for (n, status) in [
            (9_100, SessionStatus::WaitingUser),
            (9_101, SessionStatus::Stuck),
            (9_102, SessionStatus::Starting),
        ] {
            storage
                .sessions()
                .insert(&record(n, &fx.projects[0], status))
                .unwrap();
        }
    }
    let mut pages = pages(&fx);
    // The whole list, a few projects, the folders, Apps and Accounts.
    pages.retain(|page| page.kind != EntityKind::Project || fx.projects[..3].contains(&page.id));
    let mut checked = 0;
    for page in &pages {
        let query = query_of(page);
        // Pages of 7, so the order holds across a page's end.
        let rows = rows_of(state, &query, 7);
        let order: Vec<(Rank, i64)> = rows.iter().map(order_of).collect();
        for pair in order.windows(2) {
            assert!(
                pair[0].0 < pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 >= pair[1].1),
                "{page:?}: {:?} before {:?}",
                pair[0],
                pair[1]
            );
        }
        let keys: std::collections::HashSet<&str> =
            rows.iter().map(|row| row.key.as_str()).collect();
        assert_eq!(keys.len(), rows.len(), "{page:?}: no row twice");
        let facets = sessions_facets_impl(state, &query).unwrap();
        let narrowings: [(
            &str,
            &Vec<juggler_app::features::sessions::facets::FacetInfo>,
        ); 4] = [
            ("project", &facets.projects),
            ("app", &facets.apps),
            ("account", &facets.subscriptions),
            ("machine", &facets.machines),
        ];
        for (kind, values) in narrowings {
            // The page's own kind is no filter of its list.
            let own = matches!(
                (kind, page.kind),
                ("project", EntityKind::Project)
                    | ("app", EntityKind::App)
                    | ("account", EntityKind::Subscription)
            );
            if own {
                assert!(values.len() <= 1, "{page:?}: one {kind}, its own");
                continue;
            }
            if kind != "account" {
                assert_eq!(
                    values.iter().map(|facet| facet.count).sum::<u64>(),
                    rows.len() as u64,
                    "{page:?}: every row has one {kind}"
                );
            }
            for facet in values {
                let mut narrowed = query.clone();
                let id = Some(facet.id.clone());
                match kind {
                    "project" => narrowed.project_id = id,
                    "app" => narrowed.app_id = id,
                    "account" => narrowed.subscription_id = id,
                    _ => narrowed.machine_id = id,
                }
                let shown = rows_of(state, &narrowed, 200);
                assert_eq!(
                    shown.len() as u64,
                    facet.count,
                    "{page:?}: {kind} {}",
                    facet.id
                );
                assert_eq!(
                    sessions_counts_impl(state, &narrowed).unwrap().all,
                    facet.count,
                    "{page:?}: {kind} {}: the heading",
                    facet.id
                );
                for row in &shown {
                    assert!(
                        keys.contains(row.key.as_str()),
                        "{page:?}: {} is the list's",
                        row.key
                    );
                    let project = row
                        .session
                        .as_ref()
                        .map(|s| s.project_id.clone())
                        .or_else(|| row.conversation.as_ref().map(|c| c.project_id.clone()))
                        .flatten();
                    if kind == "project" {
                        assert_eq!(
                            project.unwrap_or_default(),
                            facet.id,
                            "{page:?}: {}",
                            row.key
                        );
                    }
                    if kind == "machine"
                        && let Some(session) = &row.session
                    {
                        assert_eq!(session.machine_id, facet.id);
                    }
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 40, "{checked} filters checked");
    // "No project" is one of the whole list's: the one in nine with none.
    let all = sessions_facets_impl(state, &SessionsQuery::default()).unwrap();
    let none = all
        .projects
        .iter()
        .find(|facet| facet.id.is_empty())
        .expect("some have no project");
    assert_eq!(none.count, (0..240).filter(|n| n % 9 == 0).count() as u64);
    assert_eq!(
        all.machines.len(),
        3,
        "two Macs AI Juggler ran on, and this one"
    );
    // Since: only what was active then or later, and its heading agrees.
    let since = 1_760_000_000_000 + 200 * 60_000;
    let recent = SessionsQuery {
        since_ms: Some(since),
        which: Some("ended".into()),
        ..SessionsQuery::default()
    };
    let shown = rows_of(state, &recent, 50);
    assert!(!shown.is_empty());
    assert!(
        shown
            .iter()
            .all(|row| order_of(row).1 >= since && order_of(row).0 == Rank::Ended)
    );
    assert_eq!(
        sessions_counts_impl(state, &recent).unwrap().ended,
        shown.len() as u64
    );
}

/// Contention: another thread holds the app's database connection (a
/// long write, the warm-up build). With nothing changed, a count comes
/// from the cache without waiting for it; a write nothing announced,
/// committed meanwhile by another connection, is never served stale:
/// the count waits for the connection and counts it.
#[test]
fn a_count_is_checked_even_while_the_database_is_busy() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let fx = fixture(100, 12);
    let state = &fx.state;
    let project = entity(EntityKind::Project, &fx.projects[3]);
    let before = relations::counts(state, &project).unwrap();
    let released = Arc::new(AtomicBool::new(false));
    let (held, wait) = std::sync::mpsc::channel();
    let (release, until) = std::sync::mpsc::channel::<()>();
    let holder = {
        let state = Arc::clone(state);
        let released = Arc::clone(&released);
        std::thread::spawn(move || {
            let storage = state.storage.lock().unwrap();
            held.send(()).unwrap();
            // Bounded: a count that wrongly waits fails, never hangs.
            let _ = until.recv_timeout(Duration::from_secs(20));
            released.store(true, Ordering::SeqCst);
            drop(storage);
        })
    };
    wait.recv().unwrap();
    assert_eq!(relations::counts(state, &project).unwrap(), before);
    assert!(
        !released.load(Ordering::SeqCst),
        "with nothing changed, the cached count didn't wait for the connection"
    );
    // Another writer commits a session in the project, telling no one.
    let other = rusqlite::Connection::open(state.data_dir.join("juggler.sqlite")).unwrap();
    other.busy_timeout(Duration::from_secs(5)).unwrap();
    other
        .execute(
            "INSERT INTO instances (id, launch_recipe_id, machine, kind, project, cwd, status, \
             started_at, usage_json) SELECT 's-elsewhere', 'rec-3', 'mac-here', 'terminal', ?1, \
             '/nowhere', 'running', 1, usage_json FROM instances LIMIT 1",
            [&fx.projects[3]],
        )
        .unwrap();
    let builds = state.relations.builds();
    let counting = {
        let state = Arc::clone(state);
        let project = project.clone();
        std::thread::spawn(move || relations::counts(&state, &project).unwrap())
    };
    // The count found its part stale and is building it, held up by the
    // busy connection (never answering from the cache meanwhile).
    let deadline = Instant::now() + Duration::from_secs(20);
    while state.relations.builds() == builds && !counting.is_finished() {
        assert!(
            Instant::now() < deadline,
            "the count neither built nor answered"
        );
        // poll: bounded by the deadline above; until the count starts its build.
        std::thread::sleep(Duration::from_millis(5));
    }
    release.send(()).unwrap();
    let after = counting.join().unwrap();
    holder.join().unwrap();
    assert_eq!(after.sessions.all, before.sessions.all + 1);
    assert_eq!(after.sessions.open, before.sessions.open + 1);
    assert_agrees(state, &project, &after);
}
