//! AJ-E3301: builds a 0.4 scenario's starting state (its `setup`, from
//! `scripts/qa/scenario.py seed ID`) in a lab's AI Juggler data folder, so
//! L2's harness and L4's hand walk start from the same place. Opt-in and
//! ignored by default; `scripts/qa/lab_scenario.sh` runs it:
//!
//!   AIJ_QA_SEED=seed.json AIJ_QA_DATA=/tmp/aij-lab-…/data \
//!     cargo test -p juggler-app --test qa_scenario_seed -- --ignored
//!
//! It refuses any data folder that isn't a lab's or the replica's, and
//! writes only there: Apps a CLI tool runs become AI Juggler's own managed
//! folders (no vendor folder is touched); a desktop app's profile is left
//! out with a note (its place comes from the running app).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use juggler_app::state::{AppState, default_registry};
use juggler_core::agent_access::AccessLevel;
use juggler_core::model::{
    InstanceKind, InstanceRecord, ProfileId, ProjectId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::storage::{MemberLevel, MemberRef, PoolRole, Scope};
use serde_json::Value;

fn lab_path(var: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var(var).unwrap_or_else(|_| panic!("{var} isn't set")));
    let text = path.to_string_lossy();
    assert!(
        text.contains("aij-lab") || text.contains("aij-replica"),
        "{var}={text}: refusing a folder that isn't a lab's or the replica's"
    );
    path
}

fn status_of(state: &str) -> SessionStatus {
    match state {
        "working" => SessionStatus::Running,
        "needs-you" | "idle" => SessionStatus::WaitingUser,
        "stuck" => SessionStatus::Stuck,
        "ended" => SessionStatus::Finished,
        "crashed" => SessionStatus::Failed,
        // Walk K3: a session that hit its usage limit.
        "limit" => SessionStatus::QuotaBlocked,
        other => panic!("unknown session state {other:?}"),
    }
}

/// Folder `id`'s own folder.
fn dir_of(state: &Arc<AppState>, id: &str) -> PathBuf {
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&juggler_core::model::AccountId::new(id.to_owned()))
        .unwrap()
        .state_dir
}

/// A lab's room: folder `id`'s 5-hour window `used`% full, as its tool's
/// own statusline would have said a minute ago, resetting in two hours
/// (`room_used` on a scenario App; walk A5's "out of room").
fn lab_room(state: &Arc<AppState>, id: &str, used: f64) {
    use juggler_core::model::Confidence;
    use juggler_core::quota::{QuotaSample, UsageSource, WindowKind};
    let now = Timestamp::now().millis();
    let sample = QuotaSample {
        ts: now - 60_000,
        used_pct: used,
        resets_at: Some(now + 2 * 3_600_000),
        confidence: Confidence::Measured,
    };
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_sourced_samples(&[(
            id.to_owned(),
            WindowKind::FiveHour,
            sample,
            UsageSource::Statusline,
        )])
        .unwrap();
}

/// A lab sign-in for a Claude Code folder on scenario Account `name`:
/// made-up ids in `.claude.json`'s `oauthAccount` (what AI Juggler reads
/// to know the Account; folders on one name share it, so they group) and
/// a `.credentials.json` that only exists (AI Juggler checks presence and
/// size, never opens it). Nothing in it signs anything in.
fn lab_sign_in(dir: &std::path::Path, name: &str) {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let state = juggler_adapters::claude_code::claude_state_file(dir);
    let mut value: Value = std::fs::read(&state)
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    value["oauthAccount"] = serde_json::json!({
        "accountUuid": format!("lab-account-{slug}"),
        "emailAddress": format!("{slug}@example.invalid"),
    });
    std::fs::write(&state, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    std::fs::write(
        dir.join(".credentials.json"),
        b"{\"lab\": \"not a sign-in\"}",
    )
    .unwrap();
}

fn level_of(word: &str) -> Option<AccessLevel> {
    AccessLevel::parse(&word.replace('-', "_"))
}

#[test]
#[ignore = "opt-in: seeds a lab data folder (scripts/qa/lab_scenario.sh)"]
fn seed_scenario() {
    let Ok(seed_path) = std::env::var("AIJ_QA_SEED") else {
        return;
    };
    let data = lab_path("AIJ_QA_DATA");
    let home = lab_path("AIJ_QA_HOME");
    let seed: Value = serde_json::from_str(&std::fs::read_to_string(&seed_path).unwrap()).unwrap();
    let state = Arc::new(AppState::init_with_registry(data, default_registry()).unwrap());
    // This Mac's own id, as the app's own launches record it (walk K2: a
    // hard-coded "this-mac" made the lab's launches look like a second Mac).
    let machine = juggler_app::commands::local_machine_id(&state);
    let mut notes: Vec<String> = Vec::new();

    // Projects: a folder each under the lab HOME.
    let mut projects: HashMap<String, ProjectId> = HashMap::new();
    let names = seed["projects"].as_array().cloned().unwrap_or_default();
    let worktrees = seed["worktrees"].as_array().cloned().unwrap_or_default();
    for name in names
        .iter()
        .chain(worktrees.iter())
        .filter_map(Value::as_str)
    {
        let root = home.join("repos").join(name);
        std::fs::create_dir_all(&root).unwrap();
        let record = state
            .storage
            .lock()
            .unwrap()
            .projects()
            .touch(&root, Some(name))
            .unwrap();
        projects.insert(name.to_owned(), record.id);
    }

    // Apps: a managed folder per CLI App, with its launch settings.
    let mut recipes: HashMap<String, (String, String)> = HashMap::new();
    // The desktop Apps by the scenario's id: their sessions run in them.
    let mut desktop_apps: HashMap<String, String> = HashMap::new();
    // A scenario's Accounts: its Claude folders get a lab sign-in on their
    // `account` (unless `signed_in = false`), and the Account its name.
    let mut account_names: HashMap<String, String> = HashMap::new();
    fn signed_in(app: &Value) -> Option<&str> {
        app["account"]
            .as_str()
            .filter(|name| !name.is_empty() && app["signed_in"].as_bool() != Some(false))
    }
    let adopt = std::env::var("AIJ_LAB_ADOPT").ok();
    let mut adopted_once = false;
    for app in seed["apps"].as_array().cloned().unwrap_or_default() {
        let (id, tool) = (app["id"].as_str().unwrap(), app["tool"].as_str().unwrap());
        if tool == "claude-desktop" {
            // D-84: an App because the user added it. A Claude app profile:
            // its folder under the lab's ClaudeProfiles, the Claude Code
            // folder its Code sessions run on (its Account), and the
            // desktop App linking them, closed, with how Open starts it.
            let profile = app["profile"].as_str().unwrap_or(id);
            // D-84: a profile only on disk (`added = false`), for Find
            // them to offer; nothing of AI Juggler's knows it yet.
            if app["added"].as_bool() == Some(false) {
                std::fs::create_dir_all(
                    home.join("Library/Application Support/ClaudeProfiles")
                        .join(profile),
                )
                .unwrap();
                notes.push(format!("{id}: only its folder, for Find them to offer"));
                continue;
            }
            let label = match app["account"].as_str().unwrap_or("") {
                "" => profile.to_owned(),
                account => format!("{account} · {profile}"),
            };
            let Ok(account) =
                juggler_app::features::account::create_account(&state, "claude-code", &label, None)
            else {
                notes.push(format!(
                    "{id}: its Claude Code folder can't be made; left out"
                ));
                continue;
            };
            if let Some(name) = signed_in(&app) {
                lab_sign_in(&dir_of(&state, &account.id), name);
                account_names.insert(account.id.clone(), name.to_owned());
            }
            if let Some(used) = app["room_used"].as_f64() {
                lab_room(&state, &account.id, used);
            }
            let dir = home
                .join("Library/Application Support/ClaudeProfiles")
                .join(profile);
            std::fs::create_dir_all(&dir).unwrap();
            let app_id = juggler_app::features::places::book::place_id("claude-app", Some(&dir));
            let bundle = PathBuf::from("/Applications/Claude.app");
            let launch = serde_json::json!({
                "name": "Claude app",
                "instance": profile,
                "bundle": bundle,
                "exe": bundle.join("Contents/MacOS/Claude"),
                "given": dir,
            });
            let now = Timestamp::now().millis();
            {
                let storage = state.storage.lock().unwrap();
                storage
                    .apps()
                    .put_desktop(&juggler_core::storage::App {
                        id: app_id.clone(),
                        kind: "desktop".to_owned(),
                        tool: "claude-code".to_owned(),
                        host_app_id: Some("claude-app".to_owned()),
                        instance_dir: Some(dir.to_string_lossy().into_owned()),
                        folder_id: Some(account.id.clone()),
                        label: None,
                        hidden: false,
                        keep_alive: false,
                        paused_json: None,
                        launch_json: Some(launch.to_string()),
                        sessions_json: None,
                        first_seen: now,
                        last_seen: now,
                        added: true,
                    })
                    .unwrap();
                storage.apps().add(&app_id).unwrap();
            }
            if app["running"].as_bool() == Some(true) {
                notes.push(format!(
                    "{id}: seeded closed; the walk opens it (Open starts the Claude app on its lab folder)"
                ));
            }
            let recipe = juggler_app::features::profile::create_profile(
                &state,
                &account.id,
                "Main",
                None,
                None,
            )
            .unwrap();
            desktop_apps.insert(id.to_owned(), app_id);
            recipes.insert(id.to_owned(), (account.id, recipe.id));
            continue;
        }
        // A tool with no Account (OpenCode in P8) is named by its profile
        // alone, not " · default".
        let profile = app["profile"].as_str().unwrap_or(id);
        let label = match app["account"].as_str().unwrap_or("") {
            "" => profile.to_owned(),
            account => format!("{account} · {profile}"),
        };
        // ADR-0027 condition 7 (AIJ_LAB_ADOPT): the tool's real default
        // folder, used where it is, for that tool's Apps (its real sign-in
        // runs small tasks in the lab's folders). Its other Apps of that
        // tool can't have another sign-in: one App only.
        let account = if adopt.as_deref() == Some(tool) {
            if adopted_once {
                notes.push(format!(
                    "{id}: one {tool} App uses the real folder; a second would need another sign-in, left out"
                ));
                continue;
            }
            adopted_once = true;
            let real = real_default(&state, tool, &home);
            let request = juggler_app::features::account::import::ImportRequest {
                in_place: true,
                ..Default::default()
            };
            match juggler_app::features::account::import::account_import_mode_impl(
                &state,
                tool,
                &real.to_string_lossy(),
                &label,
                request,
            ) {
                Ok(result) => result.account,
                Err(error) => {
                    notes.push(format!(
                        "{id}: the real {tool} folder can't be used: {error}"
                    ));
                    continue;
                }
            }
        } else {
            let Ok(account) =
                juggler_app::features::account::create_account(&state, tool, &label, None)
            else {
                notes.push(format!(
                    "{id}: {tool} isn't a tool this build knows; left out"
                ));
                continue;
            };
            if tool == "claude-code"
                && let Some(name) = signed_in(&app)
            {
                lab_sign_in(&dir_of(&state, &account.id), name);
                account_names.insert(account.id.clone(), name.to_owned());
            }
            if let Some(used) = app["room_used"].as_f64() {
                lab_room(&state, &account.id, used);
            }
            account
        };
        let profile =
            juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
                .unwrap();
        recipes.insert(id.to_owned(), (account.id, profile.id));
    }

    // Sessions: rows in the states the scenario starts from.
    for session in seed["sessions"].as_array().cloned().unwrap_or_default() {
        let id = session["id"].as_str().unwrap();
        let Some((_, recipe)) = recipes.get(session["app"].as_str().unwrap()) else {
            notes.push(format!("{id}: its App isn't seeded; left out"));
            continue;
        };
        let project = session["project"]
            .as_str()
            .and_then(|name| projects.get(name))
            .cloned();
        let cwd = session["project"]
            .as_str()
            .map_or_else(|| home.clone(), |name| home.join("repos").join(name));
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new(id),
                conversation: None,
                profile: ProfileId::new(recipe.as_str()),
                machine: machine.clone(),
                // A session AI Juggler started in a desktop App runs there.
                kind: if desktop_apps.contains_key(session["app"].as_str().unwrap()) {
                    InstanceKind::Desktop
                } else {
                    InstanceKind::Terminal
                },
                project,
                cwd,
                status: status_of(session["state"].as_str().unwrap_or("idle")),
                started_at: Timestamp::now(),
                last_activity: Some(Timestamp::now()),
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        if let Some(app_id) = desktop_apps.get(session["app"].as_str().unwrap()) {
            state
                .storage
                .lock()
                .unwrap()
                .apps()
                .set_session(id, app_id)
                .unwrap();
        }
    }

    // `ended_sessions = N` (the K walks: paging a long history): N ended
    // sessions over the scenario's terminal Apps and projects, one every
    // 37 minutes back from now, oldest last.
    let bulk = seed["ended_sessions"].as_u64().unwrap_or(0);
    let mut terminal: Vec<(&String, &(String, String))> = recipes
        .iter()
        .filter(|(app, _)| !desktop_apps.contains_key(*app))
        .collect();
    terminal.sort_by(|a, b| a.0.cmp(b.0));
    let mut names: Vec<&String> = projects.keys().collect();
    names.sort();
    if bulk > 0 && !terminal.is_empty() {
        let now = Timestamp::now().millis();
        let storage = state.storage.lock().unwrap();
        for i in 0..bulk {
            let (_, (_, recipe)) = terminal[usize::try_from(i).unwrap() % terminal.len()];
            let name = names
                .get(usize::try_from(i).unwrap() % names.len().max(1))
                .copied();
            let started = now - i64::try_from(i + 1).unwrap() * 37 * 60_000;
            storage
                .sessions()
                .insert(&InstanceRecord {
                    id: SessionId::new(format!("ended-{i:04}")),
                    conversation: None,
                    profile: ProfileId::new(recipe.as_str()),
                    machine: machine.clone(),
                    kind: InstanceKind::Terminal,
                    project: name.and_then(|name| projects.get(name)).cloned(),
                    cwd: name.map_or_else(|| home.clone(), |name| home.join("repos").join(name)),
                    status: SessionStatus::Finished,
                    started_at: Timestamp::from_millis(started),
                    last_activity: Some(Timestamp::from_millis(started + 20 * 60_000)),
                    usage: Default::default(),
                    attention: None,
                    running: None,
                    transcript_ref: None,
                    window: None,
                })
                .unwrap();
        }
        notes.push(format!(
            "{bulk} ended sessions, back to {} days ago",
            bulk * 37 / 60 / 24
        ));
    }

    // Pools, their scope, level and seal; then their members.
    let mut pools: HashMap<String, String> = HashMap::new();
    for pool in seed["pools"].as_array().cloned().unwrap_or_default() {
        let name = pool["name"].as_str().unwrap();
        let scope_word = pool["scope"].as_str().unwrap_or("every");
        let scope = if scope_word == "every" {
            Scope::Every
        } else {
            Scope::These(
                scope_word
                    .split(',')
                    .map(|name| projects.get(name.trim()).cloned())
                    .collect::<BTreeSet<_>>(),
            )
        };
        let storage = state.storage.lock().unwrap();
        let made = storage.pools().create_scoped(name, &scope).unwrap();
        if pool["sealed"].as_bool() == Some(true) {
            storage.pools().set_sealed(&made.id, true).unwrap();
        }
        if let Some(level) = pool["level"].as_str().and_then(level_of) {
            storage
                .agent_access()
                .set_level(&made.id, level, &BTreeSet::new())
                .unwrap();
        }
        if let Some(template) = pool["template"].as_str() {
            notes.push(format!(
                "{name}: made as the {template} template would; its talk grid is the default"
            ));
        }
        pools.insert(name.to_owned(), made.id);
    }
    for member in seed["members"].as_array().cloned().unwrap_or_default() {
        let pool = &pools[member["pool"].as_str().unwrap()];
        let reference = member["ref"].as_str().unwrap();
        let member_ref = match member["kind"].as_str().unwrap() {
            "session" => MemberRef::Session(SessionId::new(reference)),
            "app" => match recipes.get(reference) {
                // An App member is the App's own id: `cli:<folder>` for
                // a CLI App (the P3 walk: the bare folder showed "An App").
                Some((account, _)) => MemberRef::App(format!("cli:{account}")),
                None => {
                    notes.push(format!(
                        "{reference}: an App member whose App isn't seeded; left out"
                    ));
                    continue;
                }
            },
            "account" => MemberRef::Account(reference.to_owned()),
            "tool" => MemberRef::Tool(reference.to_owned()),
            "mac" => MemberRef::Mac(reference.to_owned()),
            "everyone" => MemberRef::Everyone,
            other => panic!("unknown member kind {other:?}"),
        };
        let role = PoolRole::parse(member["role"].as_str().unwrap_or("member")).unwrap();
        let level = member["level"].as_str().map(|word| MemberLevel {
            level: word.replace('-', "_"),
            custom_actions: String::new(),
        });
        state
            .storage
            .lock()
            .unwrap()
            .pools()
            .join(pool, &member_ref, &[], role, level.as_ref())
            .unwrap();
    }
    // Each signed-in Account named as the scenario names it ("Work").
    let mut named = BTreeSet::new();
    for account in juggler_app::features::subscriptions::list_impl(&state).unwrap() {
        let Some(id) = account.account_id.clone() else {
            continue;
        };
        if let Some(name) = account
            .profiles
            .iter()
            .find_map(|profile| account_names.get(&profile.id))
        {
            juggler_app::features::subscriptions::rename_impl(&state, &id, Some(name)).unwrap();
            named.insert(name.clone());
        }
    }
    if !named.is_empty() {
        notes.push(format!(
            "Accounts signed in with a lab sign-in: {}",
            named.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for note in &notes {
        println!("seed: {note}");
    }
    println!(
        "seed: {} projects, {} Apps, {} pools",
        projects.len(),
        recipes.len(),
        pools.len()
    );
}

/// A tool's default folder in the real home: the adapter's (under the
/// lab's HOME) moved to `AIJ_QA_REAL_HOME`. The lab bridge checks it
/// again, from the user record, before anything runs on it.
fn real_default(state: &Arc<AppState>, tool: &str, lab: &std::path::Path) -> std::path::PathBuf {
    let real_home =
        std::path::PathBuf::from(std::env::var("AIJ_QA_REAL_HOME").expect("AIJ_QA_REAL_HOME"));
    let default = state
        .registry
        .get(&juggler_core::model::HarnessId::new(tool))
        .and_then(|adapter| adapter.vendor_default_dir())
        .expect("a tool with a default folder");
    real_home.join(
        default
            .strip_prefix(lab)
            .expect("the default folder is under the lab's HOME"),
    )
}
