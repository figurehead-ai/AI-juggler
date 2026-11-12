//! AJ-E3505: the Floor's snapshot, from what the app already keeps.
//!
//! - [`gather`] reads the sources the rail and the pages read: AI
//!   Juggler's sessions from the last sample (never a new one), other
//!   apps' sessions from the conversation index, the stored Apps and
//!   Accounts, the pools' placements, each folder's usage, the process
//!   graph, This Mac's cached health and the notification summary. It
//!   starts nothing and writes nothing.
//! - [`assemble`] turns them into a [`FloorSnapshot`], purely, so its
//!   rules are tested without an app.
//!
//! The rail's snooze is the window's own (it lives in the window's
//! storage), so a snoozed ask still stands and waves on the Floor.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use juggler_core::storage::{App, cli_app_id};
use juggler_floor::snapshot::{
    FloorAccount, FloorApp, FloorAppKind, FloorMachine, FloorPool, FloorSession, FloorSnapshot,
};
use juggler_floor::state::{FloorState, other_session, own_session};

use crate::commands::{InstanceInfo, NeedsAttention, lock_storage_arc};
use crate::error::AppResult;
use crate::features::conversations::dto::ConversationInfo;
use crate::state::AppState;

/// An App is hot after its share of the Mac's CPU has stayed at or over
/// this for [`HOT_AFTER_MS`] (SPEC §9.7: a fire on its busiest desk).
pub const HOT_SHARE: f64 = 0.5;
/// How long an App's CPU must stay high before it's hot.
pub const HOT_AFTER_MS: i64 = 2 * 60 * 1000;

/// One folder's usage: what's left of its tightest limit, when it resets,
/// and whether it's out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FolderUsage {
    /// What's left, 0–100, when known.
    pub left_pct: Option<f32>,
    /// When that limit resets (epoch ms).
    pub resets_ms: Option<i64>,
    /// At a limit now.
    pub out: bool,
}

/// What the Floor needs of one of AI Juggler's sessions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnRow {
    /// Its id.
    pub id: String,
    /// Its status.
    pub status: String,
    /// Its attention record, by word (`waiting_user`).
    pub needs_attention: Option<&'static str>,
    /// The tool's own word for its state.
    pub runtime: Option<String>,
    /// Its title, as the rail says it.
    pub title: String,
    /// Its folder.
    pub folder: Option<String>,
    /// Its project.
    pub project_id: Option<String>,
    /// Subagents running.
    pub subagents: u32,
    /// When it started, epoch ms.
    pub started_ms: i64,
    /// Its last activity, epoch ms.
    pub last_ms: Option<i64>,
}

impl OwnRow {
    /// The row's facts the Floor uses.
    #[must_use]
    pub fn of(row: &InstanceInfo) -> Self {
        let title = seat_name(
            row.label.as_deref(),
            row.title.as_deref(),
            row.git_branch.as_deref(),
            folder_of(&row.cwd),
            Some(row.started_at_ms),
            crate::commands::now_millis(),
        );
        Self {
            id: row.id.clone(),
            status: row.status.clone(),
            needs_attention: row.needs_attention.map(needs_word),
            runtime: row.runtime.as_ref().and_then(|r| r.state.clone()),
            title,
            folder: row.account_id.clone(),
            project_id: row.project_id.clone(),
            subagents: u32::try_from(row.subtask_count.unwrap_or(0)).unwrap_or(u32::MAX),
            started_ms: row.started_at_ms,
            last_ms: row.last_activity_ms,
        }
    }
}

/// What the snapshot is made from.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    /// The stored Apps (hidden ones left out).
    pub apps: Vec<App>,
    /// Each App's colour slot.
    pub colour_slots: BTreeMap<String, u8>,
    /// Each tool's name, by id.
    pub tool_names: BTreeMap<String, String>,
    /// Each tool's desktop app as its spec names it (`Claude`), by id.
    pub desktop_apps: BTreeMap<String, String>,
    /// Each desktop App's app word as the places book shows it ("Claude
    /// app"), by App id: the rail's name, never the tool's.
    pub desktop_words: BTreeMap<String, String>,
    /// ADR-0032: each of the user's desktop Apps' one identity, by App id
    /// (`places::identities`): the rail's own name for it.
    pub identities: BTreeMap<String, crate::features::presence::AppIdentity>,
    /// ADR-0032: the desktop Apps running now (`places::running_at_last_look`):
    /// their offices are lit, sessions or not, as Open now lists them.
    pub running: BTreeSet<String>,
    /// Each tool's Floor family word, for its (CLI, desktop) Apps.
    pub families: BTreeMap<String, (Option<String>, Option<String>)>,
    /// Each folder's name.
    pub folder_names: BTreeMap<String, String>,
    /// Each folder's Account.
    pub folder_accounts: BTreeMap<String, String>,
    /// Each Account's name.
    pub account_names: BTreeMap<String, String>,
    /// Each folder's usage.
    pub usage: BTreeMap<String, FolderUsage>,
    /// Each project's name.
    pub projects: BTreeMap<String, String>,
    /// AI Juggler's sessions, each with its App.
    pub own: Vec<(OwnRow, String)>,
    /// Other apps' sessions, each with its App.
    pub other: Vec<(ConversationInfo, String)>,
    /// Open tasks, by AI Juggler session.
    pub tasks: BTreeMap<String, u32>,
    /// The pools, and their sessions by the rail's key.
    pub pools: Vec<FloorPool>,
    /// CPU, percent of one core, by the rail's session key.
    pub cpu: BTreeMap<String, f64>,
    /// The Mac's logical cores.
    pub cores: u32,
    /// This Mac.
    pub machine: FloorMachine,
    /// Unread notifications.
    pub notices: u32,
    /// Needs you's one count (AJ-E2502: Home's, the bell's).
    pub needs: u32,
    /// The print of each App's kept floor logo.
    pub logos: BTreeMap<String, String>,
}

fn needs_word(attention: NeedsAttention) -> &'static str {
    match attention {
        NeedsAttention::WaitingUser => "waiting_user",
        NeedsAttention::Stuck => "stuck",
        NeedsAttention::QuotaBlocked => "quota_blocked",
        NeedsAttention::Failed => "failed",
    }
}

/// A seat's name: the rail's one rule (`naming::short_name`); untitled,
/// its folder and when it started (D-96) — its App is the area it sits in.
fn seat_name(
    label: Option<&str>,
    title: Option<&str>,
    branch: Option<&str>,
    folder: &str,
    started_ms: Option<i64>,
    now: i64,
) -> String {
    let started = juggler_core::naming::named_by_facts(label, title, branch)
        .then(|| started_ms.filter(|ms| *ms > 0))
        .flatten()
        .and_then(|ms| juggler_platform::clock::started_words(ms, now));
    juggler_core::naming::short_name(
        label,
        title,
        branch,
        &juggler_core::naming::Facts {
            folder: Some(folder),
            app: None,
            started: started.as_deref(),
        },
    )
}

fn folder_of(cwd: &str) -> &str {
    std::path::Path::new(cwd)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

/// Up to two initials from a name ("Work Max" → "WM").
#[must_use]
pub fn initials(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric())
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

#[allow(clippy::cast_possible_truncation)] // percent of a core: small
fn cpu_of(inputs: &Inputs, key: &str) -> f32 {
    inputs.cpu.get(key).copied().unwrap_or(0.0) as f32
}

fn own_seat(row: &OwnRow, inputs: &Inputs) -> FloorSession {
    let seen = own_session(
        &row.status,
        row.needs_attention,
        row.runtime.as_deref(),
        false,
    );
    FloorSession {
        key: format!("s:{}", row.id),
        title: row.title.clone(),
        state: seen.floor(),
        word: seen.word(),
        subagents: row.subagents,
        project: row
            .project_id
            .as_ref()
            .and_then(|p| inputs.projects.get(p).cloned()),
        tasks: inputs.tasks.get(&row.id).copied().unwrap_or(0),
        cpu_pct: cpu_of(inputs, &format!("s:{}", row.id)),
        started_ms: row.started_ms,
        last_ms: row.last_ms.unwrap_or(row.started_ms),
        // AI Juggler's own: it is the app
        go_to: None,
    }
}

fn other_seat(c: &ConversationInfo, inputs: &Inputs, now: i64) -> FloorSession {
    let last = c.last_activity_ms.unwrap_or(0);
    let looked = c.viewed_ms.unwrap_or(0) >= last;
    let seen = other_session(c.live, c.state.as_deref(), c.probably, looked);
    let folder = folder_of(c.cwd.as_deref().unwrap_or_default());
    // Its own title whole, as the rail shows one (never cut to its first words).
    let title = c.title.clone().unwrap_or_else(|| {
        seat_name(
            None,
            None,
            c.git_branch.as_deref(),
            folder,
            c.started_ms,
            now,
        )
    });
    FloorSession {
        key: format!("c:{}/{}", c.account_id, c.id),
        title,
        state: seen.floor(),
        word: seen.word(),
        subagents: u32::try_from(c.subtask_count.unwrap_or(0)).unwrap_or(u32::MAX),
        project: c
            .project_id
            .as_ref()
            .and_then(|p| inputs.projects.get(p).cloned())
            .or_else(|| (!folder.is_empty()).then(|| folder.to_owned())),
        tasks: 0,
        cpu_pct: cpu_of(inputs, &format!("c:{}/{}", c.account_id, c.id)),
        started_ms: c.started_ms.unwrap_or(last),
        last_ms: last,
        // open in an app: Go to it brings that app forward to it
        go_to: c
            .places
            .first()
            .filter(|_| c.live)
            .map(|p| {
                if p.shown.is_empty() {
                    p.app.clone()
                } else {
                    p.shown.clone()
                }
            })
            .filter(|name| !name.is_empty()),
    }
}

/// An App's name, the rail's (`ui/src/lib/app-name.ts`): a desktop App
/// is its app ("Claude app", from the places book, else from the tool's
/// spec) and its profile; a CLI App is its tool and its folder.
fn app_name(app: &App, inputs: &Inputs) -> String {
    use crate::features::presence::{Naming, identity};
    let named = app.label.as_deref().filter(|l| !l.trim().is_empty());
    let desktop = app.kind == "desktop";
    // ADR-0032: a desktop App the user has is named as the rail names it
    if let Some(known) = inputs.identities.get(&app.id).filter(|_| desktop) {
        return named.map_or_else(|| known.label.clone(), str::to_owned);
    }
    // a CLI App, or a desktop one the places book doesn't hold yet: the
    // same rule, from what the App says
    let tool = inputs
        .tool_names
        .get(&app.tool)
        .cloned()
        .unwrap_or_else(|| app.tool.clone());
    let word = if desktop {
        desktop_word(app, inputs, &tool)
    } else {
        tool
    };
    let folder = app
        .folder_id
        .as_ref()
        .and_then(|f| inputs.folder_names.get(f))
        .map(String::as_str)
        .filter(|_| !desktop);
    identity(Naming {
        id: &app.id,
        desktop,
        app: &word,
        instance_dir: app.instance_dir.as_deref().map(std::path::Path::new),
        default_instance: false,
        folder_label: folder,
        folder_shared: false,
        named,
    })
    .label
}

/// A desktop App's app word: the places book's ("Claude app"); without
/// one, the tool's desktop app, with "app" when the tool's own name
/// starts with it (`desktopAppWord` in the UI); never the tool's name
/// alone, which reads as its CLI.
fn desktop_word(app: &App, inputs: &Inputs, tool: &str) -> String {
    if let Some(word) = inputs.desktop_words.get(&app.id).filter(|w| !w.is_empty()) {
        return word.clone();
    }
    match inputs.desktop_apps.get(&app.tool) {
        Some(desktop) if tool.starts_with(desktop.as_str()) => format!("{desktop} app"),
        Some(desktop) => desktop.clone(),
        None => format!("{tool} app"),
    }
}

/// D-84: the Apps the Floor shows are the rail's (`apps::list_impl`).
pub use crate::features::apps::users_apps;

/// The snapshot at `now` (epoch ms). `hot` holds since when each App has
/// been busy; it is updated here.
#[must_use]
pub fn assemble(inputs: &Inputs, now: i64, hot: &mut BTreeMap<String, i64>) -> FloorSnapshot {
    let mut seats: BTreeMap<&str, Vec<FloorSession>> = BTreeMap::new();
    for (row, app) in &inputs.own {
        seats.entry(app).or_default().push(own_seat(row, inputs));
    }
    // 0.4.3: one its desktop App's records don't list sits in no App.
    for (c, app) in inputs.other.iter().filter(|(c, _)| !c.app_unlisted) {
        seats
            .entry(app)
            .or_default()
            .push(other_seat(c, inputs, now));
    }
    let cores = f64::from(inputs.cores.max(1));
    let mut apps = Vec::with_capacity(inputs.apps.len());
    let mut busy = BTreeSet::new();
    for app in &inputs.apps {
        let mut sessions = seats.remove(app.id.as_str()).unwrap_or_default();
        sessions.sort_by(|a, b| {
            (a.state == FloorState::Ended)
                .cmp(&(b.state == FloorState::Ended))
                .then(a.started_ms.cmp(&b.started_ms))
                .then_with(|| a.key.cmp(&b.key))
        });
        let cpu_pct: f64 = sessions.iter().filter_map(|s| inputs.cpu.get(&s.key)).sum();
        let hot_since_ms = (cpu_pct / (cores * 100.0) >= HOT_SHARE).then(|| {
            busy.insert(app.id.clone());
            *hot.entry(app.id.clone()).or_insert(now)
        });
        let account_id = app
            .folder_id
            .as_ref()
            .and_then(|f| inputs.folder_accounts.get(f).cloned());
        let account_name = account_id
            .as_ref()
            .and_then(|a| inputs.account_names.get(a))
            .or_else(|| {
                app.folder_id
                    .as_ref()
                    .and_then(|f| inputs.folder_names.get(f))
            });
        let kind = if app.kind == "desktop" {
            FloorAppKind::Desktop
        } else {
            FloorAppKind::Cli
        };
        let family = inputs
            .families
            .get(&app.tool)
            .and_then(|(cli, desktop)| match kind {
                FloorAppKind::Cli => cli.clone(),
                FloorAppKind::Desktop => desktop.clone().or_else(|| cli.clone()),
            });
        let last_session = sessions.iter().map(|s| s.last_ms).max();
        apps.push(FloorApp {
            id: app.id.clone(),
            name: app_name(app, inputs),
            tool: app.tool.clone(),
            host_app: app.host_app_id.clone(),
            logo: inputs.logos.get(&app.id).cloned(),
            kind,
            family: family.unwrap_or_else(|| "neutral".to_owned()),
            colour_slot: inputs.colour_slots.get(&app.id).copied().unwrap_or(0),
            account_id,
            initials: account_name.map(|n| initials(n)).unwrap_or_default(),
            open: inputs.running.contains(&app.id)
                || sessions.iter().any(|s| s.state != FloorState::Ended),
            last_active_ms: last_session.max(Some(app.last_seen)).filter(|ms| *ms > 0),
            #[allow(clippy::cast_possible_truncation)] // percent of a core: small
            cpu_pct: cpu_pct as f32,
            hot_since_ms: hot_since_ms.filter(|since| now - since >= HOT_AFTER_MS),
            sessions,
        });
    }
    hot.retain(|id, _| busy.contains(id));
    FloorSnapshot {
        at_ms: now,
        apps,
        accounts: accounts(inputs, now),
        pools: inputs.pools.clone(),
        machine: inputs.machine.clone(),
        notices: inputs.notices,
        needs: inputs.needs,
    }
}

/// Each Account: its tightest folder's usage, and its last use (the
/// latest activity of a session on any of its folders).
fn accounts(inputs: &Inputs, now: i64) -> Vec<FloorAccount> {
    let mut last: BTreeMap<&str, i64> = BTreeMap::new();
    let folder_last = inputs
        .own
        .iter()
        .filter_map(|(row, _)| Some((row.folder.as_deref()?, row.last_ms?)))
        .chain(
            inputs
                .other
                .iter()
                .filter_map(|(c, _)| Some((c.account_id.as_str(), c.last_activity_ms?))),
        );
    for (folder, ms) in folder_last {
        if let Some(account) = inputs.folder_accounts.get(folder) {
            let at = last.entry(account).or_insert(ms);
            *at = (*at).max(ms);
        }
    }
    inputs
        .account_names
        .iter()
        .map(|(id, label)| {
            let usage: Vec<&FolderUsage> = inputs
                .folder_accounts
                .iter()
                .filter(|(_, account)| *account == id)
                .filter_map(|(folder, _)| inputs.usage.get(folder))
                .collect();
            let tightest = usage
                .iter()
                .filter(|u| u.left_pct.is_some())
                .min_by(|a, b| {
                    a.left_pct
                        .unwrap_or(100.0)
                        .total_cmp(&b.left_pct.unwrap_or(100.0))
                });
            FloorAccount {
                id: id.clone(),
                label: label.clone(),
                left_pct: tightest.and_then(|u| u.left_pct),
                resets_ms: tightest.and_then(|u| u.resets_ms),
                last_used_ms: last.get(id.as_str()).copied(),
                out: usage.iter().any(|u| u.out),
                resets_words: tightest
                    .and_then(|u| u.resets_ms)
                    .and_then(|ms| juggler_platform::clock::when_words(ms, now)),
            }
        })
        .collect()
}

/// Other apps' sessions (the ones AI Juggler didn't start), each with its
/// running subagents as the rail's count last read them (FL1 step 4,
/// rc32: the index's rows carry none, so a terminal session's helpers
/// were never drawn while its page said "Subagents running · 2").
pub(crate) fn others(state: &Arc<AppState>, rows: Vec<ConversationInfo>) -> Vec<ConversationInfo> {
    crate::features::subagents::count::counted_conversations(
        state,
        rows.into_iter()
            .filter(|c| c.session_id.is_none())
            .collect(),
    )
}

/// Reads every source now.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn gather(state: &Arc<AppState>, now: i64) -> AppResult<Inputs> {
    let own_rows = crate::commands::instances_shared_snapshot(state)?;
    let other_rows = others(
        state,
        crate::features::conversations::commands::list_indexed(state)?,
    );
    let desktop = crate::features::places::desktop_apps(state);
    let mut inputs = Inputs {
        cores: 1,
        ..Inputs::default()
    };
    for id in state.registry.ids() {
        if let Some(adapter) = state.registry.get(&id) {
            let spec = adapter.spec();
            inputs.families.insert(
                spec.id.clone(),
                (spec.floor.cli.clone(), spec.floor.desktop.clone()),
            );
            if let Some(desktop) = spec.desktop_app.clone() {
                inputs.desktop_apps.insert(spec.id.clone(), desktop);
            }
            inputs.tool_names.insert(spec.id, spec.display_name);
        }
    }
    // The desktop Apps' words, as the rail names them (bug (a), 0.4.1).
    if let Ok(words) = crate::features::places::with_book(state, |book| {
        book.places
            .iter()
            .map(|(id, place)| (id.clone(), place.name.clone()))
            .collect::<BTreeMap<_, _>>()
    }) {
        inputs.desktop_words = words;
    }
    // ADR-0032: the rail's names and what runs, without a process scan
    inputs.identities = crate::features::places::commands::identities(state);
    inputs.running = crate::features::places::running_at_last_look(state);
    // The Apps list's own rule (rc22, A3): a profile's folder is a CLI
    // App while something counts under it.
    let counted = crate::features::apps::counted_profile_folders(state)?;
    {
        let storage = lock_storage_arc(state)?;
        read_entities(&storage, &counted, &mut inputs)?;
        read_usage(state, &storage, now, &mut inputs)?;
        read_work(&storage, &own_rows, &mut inputs)?;
    }
    for c in other_rows {
        let app = c.owner(&desktop);
        inputs.other.push((c, app));
    }
    read_machine(state, &mut inputs);
    let summary = crate::features::notifications::views::summary_now(state);
    inputs.notices = summary.unread;
    inputs.needs = summary.count;
    let ids: Vec<&str> = inputs.apps.iter().map(|a| a.id.as_str()).collect();
    inputs.logos = super::logos::prints(&state.cache.data_dir().join(super::logos::DIR), &ids);
    Ok(inputs)
}

/// The Apps, their colours, the Accounts and their folders, the projects.
fn read_entities(
    storage: &juggler_core::storage::Storage,
    counted: &BTreeSet<String>,
    inputs: &mut Inputs,
) -> AppResult<()> {
    let in_use: BTreeSet<String> = storage
        .accounts()
        .list()?
        .into_iter()
        .map(|folder| folder.id.as_str().to_owned())
        .collect();
    inputs.apps = users_apps(storage.apps().list()?, &in_use, counted);
    inputs.colour_slots = storage.apps().colour_slots()?;
    inputs.folder_accounts = storage
        .subscriptions()
        .folders()?
        .into_iter()
        .filter_map(|(folder, account)| Some((folder, account?)))
        .collect();
    for subscription in storage.subscriptions().list()? {
        if subscription.removed_at.is_none() {
            let name = subscription
                .name
                .clone()
                .unwrap_or_else(|| subscription.vendor.clone());
            inputs.account_names.insert(subscription.id, name);
        }
    }
    for project in storage.projects().list()? {
        inputs.projects.insert(project.id.to_string(), project.name);
    }
    Ok(())
}

/// Each folder's name and usage, as the Usage page shows it.
fn read_usage(
    state: &AppState,
    storage: &juggler_core::storage::Storage,
    now: i64,
    inputs: &mut Inputs,
) -> AppResult<()> {
    for folder in storage.accounts().list()? {
        let shown = crate::features::quota::shown(state, storage, &folder, now);
        let windows = [shown.five_hour.as_ref(), shown.weekly.as_ref()];
        let tightest = windows
            .into_iter()
            .flatten()
            .filter(|w| w.percentage.is_some())
            .max_by(|a, b| {
                a.percentage
                    .unwrap_or(0.0)
                    .total_cmp(&b.percentage.unwrap_or(0.0))
            });
        #[allow(clippy::cast_possible_truncation)] // a percentage
        let usage = FolderUsage {
            left_pct: tightest
                .and_then(|w| w.percentage)
                .map(|used| (100.0 - used).clamp(0.0, 100.0) as f32),
            resets_ms: tightest.and_then(|w| w.resets_at_ms),
            out: windows
                .into_iter()
                .flatten()
                .any(|w| w.state == "exhausted"),
        };
        inputs
            .folder_names
            .insert(folder.id.to_string(), folder.label.clone());
        inputs.usage.insert(folder.id.to_string(), usage);
    }
    Ok(())
}

/// AI Juggler's sessions with their Apps, their open tasks, and the pools.
fn read_work(
    storage: &juggler_core::storage::Storage,
    own_rows: &[InstanceInfo],
    inputs: &mut Inputs,
) -> AppResult<()> {
    let ids: Vec<String> = own_rows.iter().map(|row| row.id.clone()).collect();
    for (session, _, _, status) in storage.tasks().of_sessions(&ids)? {
        if status.is_open() {
            *inputs.tasks.entry(session).or_insert(0) += 1;
        }
    }
    let facts = crate::features::pools::members::facts(storage)?;
    for row in own_rows {
        let app = facts
            .get(&juggler_core::model::SessionId::new(row.id.clone()))
            .and_then(|f| f.app.clone())
            .unwrap_or_else(|| cli_app_id(row.account_id.as_deref().unwrap_or_default()));
        inputs.own.push((OwnRow::of(row), app));
    }
    let pools = crate::features::pools::pools_info(storage)?;
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for placement in pools.placements {
        members
            .entry(placement.pool_id)
            .or_default()
            .push(format!("s:{}", placement.session_id));
    }
    inputs.pools = pools
        .pools
        .into_iter()
        .map(|pool| FloorPool {
            sessions: members.remove(&pool.id).unwrap_or_default(),
            id: pool.id,
            name: pool.name,
            sealed: pool.sealed,
        })
        .collect();
    Ok(())
}

/// Each session's CPU from the process graph, and This Mac's health (both
/// as last sampled).
fn read_machine(state: &AppState, inputs: &mut Inputs) {
    use crate::features::processes::model::Origin;
    let graph = state.processes.graph();
    for node in graph.apps.iter().flat_map(|app| &app.sessions) {
        let key = match node.origin {
            Origin::Ours => format!("s:{}", node.session_id),
            Origin::Theirs => format!(
                "c:{}/{}",
                node.account_id.as_deref().unwrap_or_default(),
                node.session_id
            ),
        };
        *inputs.cpu.entry(key).or_insert(0.0) += node.rollup.cpu_pct;
    }
    if let Some(mac) = state.notifier.this_mac() {
        inputs.cores = mac.machine.cores.max(1);
        // the Mac chip's alert: a critical one first, then the oldest
        let critical =
            |a: &crate::machine_alerts::OverloadAlertInfo| alert_critical(&a.kind, &mac.machine);
        let shown = mac.alerts.iter().min_by_key(|a| (!critical(a), a.since_ms));
        inputs.machine = machine_of(
            mac.machine.cpu_pct,
            shown.map(|a| (a.since_ms, a.title.as_str())).into_iter(),
        );
        inputs.machine.alert_kind = shown.map(|a| a.kind.clone());
        inputs.machine.alert_critical = shown.is_some_and(critical);
        inputs.machine.room = Some(mac.machine.summary.words.clone());
    }
}

/// An alert of `kind` is critical on this Mac: macOS's critical memory
/// pressure or heat, or a disk out of room (its alert only comes then).
/// The Floor's chip is red for these, amber for the rest.
#[must_use]
pub fn alert_critical(kind: &str, machine: &crate::machine_health::MachineHealthInfo) -> bool {
    match kind {
        "memory_pressure" => machine.pressure.as_deref() == Some("critical"),
        "thermal" => machine.thermal.as_deref() == Some("critical"),
        "disk" => true,
        _ => false,
    }
}

/// This Mac on the Floor: its load as a fraction, and the overload alert
/// that has been on longest (`(since_ms, title)` each), with since when.
pub fn machine_of<'a>(
    cpu_pct: Option<f64>,
    alerts: impl Iterator<Item = (i64, &'a str)>,
) -> FloorMachine {
    let alert = alerts.min_by_key(|(since, _)| *since);
    #[allow(clippy::cast_possible_truncation)] // a fraction
    let load = cpu_pct.map_or(0.0, |pct| (pct / 100.0).clamp(0.0, 1.0) as f32);
    FloorMachine {
        load,
        alert_since_ms: alert.map(|(since, _)| since),
        alert: alert.map(|(_, title)| title.to_owned()),
        room: None,
        alert_kind: None,
        alert_critical: false,
    }
}
