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
        held_until: None,
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
    let slug = slug(name);
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

/// A lab sign-in for a Codex home on scenario Account `name`: an
/// `auth.json` of the shape Codex writes for a ChatGPT sign-in, with a
/// made-up account id (what AI Juggler reads to know the Account) and
/// placeholder tokens that sign nothing in.
fn lab_codex_sign_in(dir: &std::path::Path, name: &str) {
    let auth = serde_json::json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "access_token": "lab-not-a-token",
            "refresh_token": "lab-not-a-token",
            "account_id": format!("lab-account-{}", slug(name)),
        },
    });
    std::fs::write(
        dir.join("auth.json"),
        serde_json::to_vec_pretty(&auth).unwrap(),
    )
    .unwrap();
}

/// The P5 rc31 walk: only Claude folders got a lab sign-in, so its two
/// Codex Apps on Personal had no Account. Every App on a scenario
/// Account gets its tool's lab sign-in; `false` for a tool with none (the
/// seeder then says so). Each `"tool" =>` here is a tool that has one
/// (`scenario.py check` reads them).
fn lab_sign_in_for(tool: &str, dir: &std::path::Path, name: &str) -> bool {
    match tool {
        "claude-code" => lab_sign_in(dir, name),
        "codex" => lab_codex_sign_in(dir, name),
        _ => return false,
    }
    true
}

/// A scenario Account's name as its lab ids spell it ("Work Max":
/// "work-max").
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// A10 (`signin = "unreadable"`): the lab sign-in of a Claude Code folder
/// the seeder made, written and then cut short, so its state file can't be
/// read (what a half-written file looks like). Only a folder inside one of
/// `lab` (the lab's HOME and data folder): never an adopted real folder,
/// and walkers never cut a file by hand.
fn lab_unreadable_sign_in(dir: &std::path::Path, name: &str, lab: &[&std::path::Path]) {
    assert!(
        lab.iter().any(|root| dir.starts_with(root)),
        "{}: not one of the lab's folders; its sign-in is left alone",
        dir.display()
    );
    lab_sign_in(dir, name);
    let state = juggler_adapters::claude_code::claude_state_file(dir);
    let whole = std::fs::read(&state).unwrap();
    std::fs::write(&state, &whole[..whole.len() / 2]).unwrap();
}

/// The Apps a scenario seeds with `signin = "unreadable"`: their lab
/// sign-in cut short ([`lab_unreadable_sign_in`]). `recipes` maps an App
/// to its folder; only a Claude Code one has this sign-in.
fn seed_unreadable_sign_ins(
    state: &Arc<AppState>,
    apps: &[Value],
    recipes: &HashMap<String, (String, String)>,
    lab: &[&std::path::Path],
    notes: &mut Vec<String>,
) {
    for app in apps {
        if app["signin"].as_str() != Some("unreadable") {
            continue;
        }
        let id = app["id"].as_str().unwrap();
        let claude = matches!(app["tool"].as_str(), Some("claude-code" | "claude-desktop"));
        let Some((folder, _)) = recipes.get(id).filter(|_| claude) else {
            notes.push(format!(
                "{id}: no lab Claude Code folder; its sign-in left as is"
            ));
            continue;
        };
        let dir = dir_of(state, folder);
        if !lab.iter().any(|root| dir.starts_with(root)) {
            notes.push(format!(
                "{id}: its folder isn't the lab's; its sign-in left as is"
            ));
            continue;
        }
        let name = app["account"].as_str().unwrap_or(id);
        lab_unreadable_sign_in(&dir, name, lab);
        notes.push(format!("{id}: its sign-in can't be read (cut short)"));
    }
}

/// A scenario's project folder under the lab HOME's `repos`. The D5
/// persona walk: a lab's repos weren't git, so New session said "not a
/// git project" and greyed out "Its own worktree". A real user's repo is
/// git: each is one, with one commit, unless the scenario lists it in
/// `plain_folders`.
fn lab_project(
    home: &std::path::Path,
    name: &str,
    plain: &std::collections::HashSet<String>,
) -> PathBuf {
    let root = home.join("repos").join(name);
    std::fs::create_dir_all(&root).unwrap();
    if !plain.contains(name) {
        lab_git_repo(&root);
    }
    root
}

/// Makes `root` a git repository with one commit (a README), as a real
/// user's repo is; one that already is stays as it is. The user's own git
/// config, hooks and signing are never read: the lab's commit is the
/// seeder's alone.
fn lab_git_repo(root: &std::path::Path) {
    if root.join(".git").exists() {
        return;
    }
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "user.name=AI Juggler lab",
                "-c",
                "user.email=lab@aij-lab.invalid",
            ])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?} in a lab repo failed");
    };
    git(&["init", "-q", "-b", "main"]);
    let name = root
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    std::fs::write(root.join("README.md"), format!("# {name}\n")).unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-q", "-m", "Initial commit"]);
}

/// The id of the Account a scenario names `name`: the one its Apps' lab
/// sign-ins (`names`, by folder) are read as, as the Accounts list reads
/// them (which also records it for the pools).
fn account_named(
    state: &Arc<AppState>,
    names: &HashMap<String, String>,
    name: &str,
) -> Option<String> {
    juggler_app::features::subscriptions::list_impl(state)
        .unwrap()
        .into_iter()
        .find(|account| {
            account
                .profiles
                .iter()
                .any(|profile| names.get(&profile.id).is_some_and(|n| n == name))
        })?
        .account_id
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
    // Persona walks (AJ-E3307): a Mac as AI Juggler first sees it.
    if seed["first_run"].as_bool() == Some(true) {
        println!("seed: {}", seed_first_run(&seed, &home));
        return;
    }
    let (_, notes, _) = seed_lab(&seed, data, &home);
    for note in &notes {
        println!("seed: {note}");
    }
}

/// Seeds the lab data folder `data` (its HOME `home`) with scenario
/// `seed`: the seeded state, notes of what it did, and the process groups
/// its sessions run (live ones and leftovers; teardown stops them).
fn seed_lab(
    seed: &Value,
    data: PathBuf,
    home: &std::path::Path,
) -> (Arc<AppState>, Vec<String>, Vec<u32>) {
    let state = Arc::new(AppState::init_with_registry(data, default_registry()).unwrap());
    // This Mac's own id, as the app's own launches record it (walk K2: a
    // hard-coded "this-mac" made the lab's launches look like a second Mac).
    let machine = juggler_app::commands::local_machine_id(&state);
    let mut notes: Vec<String> = Vec::new();

    // Projects: a folder each under the lab HOME.
    let mut projects: HashMap<String, ProjectId> = HashMap::new();
    let names = seed["projects"].as_array().cloned().unwrap_or_default();
    let worktrees = seed["worktrees"].as_array().cloned().unwrap_or_default();
    // Walk K4: projects no session has run in (a picker lists them last).
    let quiet = seed["quiet_projects"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let plain: std::collections::HashSet<String> = seed["plain_folders"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let quiet_names: std::collections::HashSet<String> = quiet
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    for name in names
        .iter()
        .chain(worktrees.iter())
        .chain(quiet.iter())
        .filter_map(Value::as_str)
    {
        let root = lab_project(home, name, &plain);
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
    // A scenario's Accounts: each App's folder gets its tool's lab sign-in
    // on its `account` (unless `signed_in = false`), and the Account its
    // name.
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
            // As Add does (`places::link_its_folder`): the profile
            // launcher's `ClaudeCodeProfiles/<profile>` brought in where it
            // is, named by the profile; never a managed copy, so no second
            // "Claude Code · <account> · <profile>" App (rc22, A3).
            let code = home
                .join("Library/Application Support/ClaudeCodeProfiles")
                .join(profile);
            std::fs::create_dir_all(&code).unwrap();
            // A used folder has its settings (an empty one isn't one).
            if !code.join("settings.json").exists() {
                std::fs::write(code.join("settings.json"), "{}\n").unwrap();
            }
            let Ok(account) = juggler_app::features::account::import::account_import_mode_impl(
                &state,
                "claude-code",
                &code.to_string_lossy(),
                profile,
                juggler_app::features::account::import::ImportRequest {
                    in_place: true,
                    ..Default::default()
                },
            )
            .map(|done| done.account)
            .map_err(|error| {
                notes.push(format!(
                    "{id}: its Claude Code folder can't be brought in ({error}); left out"
                ));
            }) else {
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
            // Its launch settings: the one the import made.
            let recipe = state
                .storage
                .lock()
                .unwrap()
                .profiles()
                .list_by_account(&juggler_core::model::AccountId::new(account.id.clone()))
                .unwrap()
                .remove(0);
            desktop_apps.insert(id.to_owned(), app_id);
            recipes.insert(id.to_owned(), (account.id, recipe.id.to_string()));
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
            let real = real_default(&state, tool, home);
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
            if let Some(name) = signed_in(&app) {
                if lab_sign_in_for(tool, &dir_of(&state, &account.id), name) {
                    account_names.insert(account.id.clone(), name.to_owned());
                } else {
                    notes.push(format!(
                        "{id}: {tool} has no lab sign-in; it's on no Account"
                    ));
                }
            }
            account
        };
        // P8 (the rc26 walk): a scenario that expects "Shell only" says so
        // as data, and the setup fails when the adapter disagrees.
        if let Some(problem) = shell_only_disagrees(&state, &app, &account.id) {
            panic!("{problem}");
        }
        // Its room, on an adopted App too (P1 step 6: the Codex writer).
        if let Some(used) = app["room_used"].as_f64() {
            lab_room(&state, &account.id, used);
        }
        let profile =
            juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
                .unwrap();
        recipes.insert(id.to_owned(), (account.id, profile.id));
    }
    // A10: a sign-in that can't be read, made by the seeder in the lab.
    seed_unreadable_sign_ins(
        &state,
        &seed["apps"].as_array().cloned().unwrap_or_default(),
        &recipes,
        &[home, state.data_dir.as_path()],
        &mut notes,
    );

    // Sessions: rows in the states the scenario starts from; the process
    // groups the lab's sessions run (the caller's to stop).
    let mut groups: Vec<u32> = Vec::new();
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
            .map_or_else(|| home.to_path_buf(), |name| home.join("repos").join(name));
        // Persona D3: `days_ago = N`, a session from N days back (History
        // finds it, the rail's today doesn't); its last word half an hour
        // after it started.
        let ago = session["days_ago"]
            .as_u64()
            .map_or(0, |days| i64::try_from(days).unwrap() * 24 * 3_600_000);
        let started = Timestamp::from_millis(Timestamp::now().millis() - ago);
        let last = Timestamp::from_millis(started.millis() + if ago > 0 { 30 * 60_000 } else { 0 });
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
                started_at: started,
                last_activity: Some(last),
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
        // The M4 walk on P1S6 (rc33): a seeded session had no conversation,
        // so "Continue in another app" rightly refused it. `conversation =
        // true` gives it one in its tool's own format.
        if session["conversation"].as_bool() == Some(true) {
            let (account, _) = &recipes[session["app"].as_str().unwrap()];
            lab_conversation(&state, &session, account, &mut notes);
        }
        // Persona D4: `leftover = true`, a real process left behind by
        // this (ended) session, as AI Juggler's process ledger records one.
        if session["leftover"].as_bool() == Some(true) {
            let (account, _) = &recipes[session["app"].as_str().unwrap()];
            groups.push(lab_leftover(&state, id, account, home, &mut notes));
        }
        // The rc37 persona walks (PD3, PD4, PM4): a session seeded working
        // or waiting had no process, so the lab's first open rightly ended
        // it. One AI Juggler runs in a terminal gets its own, bounded, as
        // the ledger records a root; one in a desktop App runs in that
        // App, which the lab seeds closed.
        let live = matches!(
            status_of(session["state"].as_str().unwrap_or("idle")),
            SessionStatus::Running | SessionStatus::WaitingUser | SessionStatus::Stuck
        );
        if live && desktop_apps.contains_key(session["app"].as_str().unwrap()) {
            notes.push(format!(
                "{id}: runs in its desktop App, seeded closed; it ends when the app opens"
            ));
        } else if live {
            let (account, _) = &recipes[session["app"].as_str().unwrap()];
            groups.push(lab_live(&state, id, account, home, &mut notes));
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
    let mut names: Vec<&String> = projects
        .keys()
        .filter(|name| !quiet_names.contains(*name))
        .collect();
    names.sort();
    // Persona R3: `ended_over_days = N` spreads them evenly over N days.
    let every = match seed["ended_over_days"].as_u64() {
        // (the oldest just inside the N days).
        Some(days) if bulk > 0 => i64::try_from(days * 24 * 3_600_000 / (bulk + 1)).unwrap(),
        _ => 37 * 60_000,
    };
    if bulk > 0 && !terminal.is_empty() {
        let now = Timestamp::now().millis();
        let storage = state.storage.lock().unwrap();
        for i in 0..bulk {
            let (_, (_, recipe)) = terminal[usize::try_from(i).unwrap() % terminal.len()];
            let name = names
                .get(usize::try_from(i).unwrap() % names.len().max(1))
                .copied();
            let started = now - i64::try_from(i + 1).unwrap() * every;
            storage
                .sessions()
                .insert(&InstanceRecord {
                    id: SessionId::new(format!("ended-{i:04}")),
                    conversation: None,
                    profile: ProfileId::new(recipe.as_str()),
                    machine: machine.clone(),
                    kind: InstanceKind::Terminal,
                    project: name.and_then(|name| projects.get(name)).cloned(),
                    cwd: name
                        .map_or_else(|| home.to_path_buf(), |name| home.join("repos").join(name)),
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
            i64::try_from(bulk).unwrap() * every / (24 * 3_600_000)
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
            // An Account member is the Account's own id, read from its
            // Apps' lab sign-ins (P5 rc32: the scenario's name, kept as
            // the id, matched no Account: "Account unknown · 0 members").
            "account" => match account_named(&state, &account_names, reference) {
                Some(id) => MemberRef::Account(id),
                None => {
                    notes.push(format!(
                        "{reference}: an Account member no seeded App signs in to; left out"
                    ));
                    continue;
                }
            },
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
    // X2 (D-62.20): `pool_members = N`: N sessions in the first project,
    // each a member of the first pool (ended: a lab runs no processes; a
    // member listed and resolved is what the load walk measures).
    if let Some(count) = seed["pool_members"].as_u64() {
        let first_pool = seed["pools"]
            .as_array()
            .and_then(|all| all.first())
            .and_then(|pool| pool["name"].as_str())
            .and_then(|name| pools.get(name))
            .cloned();
        if let Some(pool_id) = first_pool {
            let profile = recipes.values().next().map_or_else(
                || {
                    let account = juggler_app::features::account::create_account(
                        &state,
                        "claude-code",
                        "Load",
                        None,
                    )
                    .unwrap();
                    juggler_app::features::profile::create_profile(
                        &state,
                        &account.id,
                        "Main",
                        None,
                        None,
                    )
                    .unwrap()
                    .id
                },
                |(_, profile)| profile.clone(),
            );
            let first = seed["projects"]
                .as_array()
                .and_then(|all| all.first())
                .and_then(Value::as_str);
            let project = first.and_then(|name| projects.get(name)).cloned();
            let cwd =
                first.map_or_else(|| home.to_path_buf(), |name| home.join("repos").join(name));
            for n in 0..count {
                let id = SessionId::new(format!("load-{n:02}"));
                let storage = state.storage.lock().unwrap();
                storage
                    .sessions()
                    .insert(&InstanceRecord {
                        id: id.clone(),
                        conversation: None,
                        profile: ProfileId::new(profile.as_str()),
                        machine: machine.clone(),
                        kind: InstanceKind::Terminal,
                        project: project.clone(),
                        cwd: cwd.clone(),
                        status: status_of("ended"),
                        started_at: Timestamp::now(),
                        last_activity: Some(Timestamp::now()),
                        usage: Default::default(),
                        attention: None,
                        running: None,
                        transcript_ref: None,
                        window: None,
                    })
                    .unwrap();
                storage
                    .pools()
                    .join(
                        &pool_id,
                        &MemberRef::Session(id),
                        &[],
                        PoolRole::Member,
                        None,
                    )
                    .unwrap();
            }
            notes.push(format!("{count} members in its first pool"));
        }
    }
    // P6: `macs = ["mini"]`, Macs paired with this one.
    let macs: Vec<String> = seed["macs"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    seed_paired_macs(&state, &macs, home, &mut notes);
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
    notes.push(format!(
        "{} projects, {} Apps, {} pools",
        projects.len(),
        recipes.len(),
        pools.len()
    ));
    (state, notes, groups)
}

/// A conversation for lab session `id` on folder `account`: a few
/// harmless turns written by the replica's own writer (`replica.py
/// conversation`) in its tool's format under the folder, and the session
/// linked to it, as a session that has talked is. Claude Code's only (a
/// desktop App's sessions run on its Claude Code folder too); another
/// tool's session is left without one, with a note.
fn lab_conversation(
    state: &Arc<AppState>,
    session: &Value,
    account: &str,
    notes: &mut Vec<String>,
) {
    let id = session["id"].as_str().unwrap();
    let (record, tool, dir) = {
        let storage = state.storage.lock().unwrap();
        let record = storage.sessions().get(&SessionId::new(id)).unwrap();
        let found = storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(account.to_owned()))
            .unwrap();
        (record, found.harness.to_string(), found.state_dir)
    };
    if tool != "claude-code" {
        notes.push(format!(
            "{id}: no lab conversation for {tool} yet; it has none"
        ));
        return;
    }
    let native = lab_uuid(id);
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/replica.py"
        ))
        .args(["conversation", "--folder"])
        .arg(&dir)
        .arg("--cwd")
        .arg(&record.cwd)
        .args(["--session", &native])
        // Persona D3: its title and what was said, and when it ended (a
        // session from `days_ago` ended half an hour after it started).
        .args(
            session["title"]
                .as_str()
                .map(|title| ["--topic", title])
                .into_iter()
                .flatten(),
        )
        .args(
            session["said"]
                .as_str()
                .map(|said| ["--said", said])
                .into_iter()
                .flatten(),
        )
        .args([
            "--ago-minutes".to_owned(),
            session["days_ago"]
                .as_u64()
                .map_or(0, |days| (days * 24 * 60).saturating_sub(30))
                .to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "replica.py conversation: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written: Value = serde_json::from_slice(&out.stdout).unwrap();
    let path = PathBuf::from(written["path"].as_str().unwrap());
    let mut linked = record;
    linked.conversation = Some(juggler_core::model::ConversationId::new(native.as_str()));
    linked.transcript_ref = Some(juggler_core::model::TranscriptRef {
        path,
        native_id: Some(native.clone()),
    });
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .update(&linked)
        .unwrap();
    notes.push(match session["title"].as_str() {
        Some(title) => format!("{id}: a conversation of a few turns, \"{title}\""),
        None => format!("{id}: a conversation of a few turns"),
    });
}

/// How long a lab's leftover lives at most (persona D4): long enough for a
/// walk, never longer; teardown stops it before then.
#[cfg(unix)]
const LEFTOVER_SECS: u32 = 1800;

/// How long a lab's live session runs at most: a persona walk and its
/// notes, never longer; teardown stops it before then. When it ends by
/// itself, its session ends, as a session whose process exited does.
#[cfg(unix)]
const LIVE_SECS: u32 = 3600;

/// The lab session `id`'s record and its tool, for its processes.
fn lab_session(
    state: &Arc<AppState>,
    id: &str,
    account: &str,
    home: &std::path::Path,
) -> (InstanceRecord, String) {
    let storage = state.storage.lock().unwrap();
    let record = storage.sessions().get(&SessionId::new(id)).unwrap();
    assert!(
        record.cwd.starts_with(home),
        "{id}: a lab session's processes run only in the lab's folders"
    );
    let tool = storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(account.to_owned()))
        .unwrap()
        .harness
        .to_string();
    (record, tool)
}

/// A bounded process of the lab in `cwd` (under its HOME), in a process
/// group of its own: a shell, named `name`, that holds a trap and a
/// `sleep` of at most `secs`. The trap takes the sleep with the shell
/// (teardown's TERM), and the sleep ends by itself in any case. Returns
/// the shell's pid (its group's id) and the sleep's.
#[cfg(unix)]
fn lab_sleeper(cwd: &std::path::Path, home: &std::path::Path, name: &str, secs: u32) -> (u32, u32) {
    use std::os::unix::process::CommandExt;
    let ready = home.join(format!(".{name}.pid"));
    let mut shell = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "trap 'kill \"$child\" 2>/dev/null; exit 0' TERM INT HUP; sleep {secs} & child=$!; echo \"$child\" > \"$1\"; wait \"$child\""
        ))
        .arg(name)
        .arg(&ready)
        .current_dir(cwd)
        .process_group(0)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let group = shell.id();
    // Reaped here when it ends (teardown's TERM or its bounded sleep), so
    // it never lingers as a zombie of this process; if the seeder exits
    // first, launchd adopts and reaps it.
    std::thread::spawn(move || {
        let _ = shell.wait();
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let pid = loop {
        if let Some(pid) = std::fs::read_to_string(&ready)
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok())
        {
            break pid;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{name}: its process never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    std::fs::remove_file(&ready).unwrap();
    (group, pid)
}

/// Process `pid`'s identity as the ledger keys it: its exe (`fallback`
/// when the OS won't say) and its OS start time.
fn os_identity(pid: u32, fallback: &str) -> (PathBuf, i64) {
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
        sysinfo::ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(sysinfo::UpdateKind::Always),
    );
    let live = system
        .process(sysinfo::Pid::from_u32(pid))
        .expect("the lab's process runs");
    let exe = live
        .exe()
        .map_or_else(|| PathBuf::from(fallback), std::path::Path::to_path_buf);
    (exe, i64::try_from(live.start_time()).unwrap())
}

/// One process of lab session `record` as AI Juggler's process ledger
/// records one it started: `role` at `depth` in group `group`.
#[allow(
    clippy::too_many_arguments,
    reason = "one ledger row: the session, its tool, account and group, and the process"
)]
fn lab_row(
    state: &Arc<AppState>,
    record: &InstanceRecord,
    tool: &str,
    account: &str,
    group: u32,
    (role, pid, depth, root_pid): (&str, Option<u32>, u32, Option<u32>),
    exe: &std::path::Path,
    line: &str,
) -> juggler_core::storage::LedgerRow {
    use juggler_supervisor::tree::LedgerStore;
    let id = record.id.as_str();
    let now = Timestamp::now().millis();
    juggler_core::storage::LedgerRow {
        pid,
        pgid: Some(group),
        started_at: None,
        exe_path: exe.to_string_lossy().into_owned(),
        cmdline_hash: format!("aij-lab-{id}-{role}"),
        cmdline_trunc: line.to_owned(),
        boot_id: LedgerStore::boot_id(state.ledger.as_ref()),
        session_id: id.to_owned(),
        instance_id: None,
        provider: tool.to_owned(),
        account_id: Some(account.to_owned()),
        project_id: record
            .project
            .as_ref()
            .map(|project| project.as_str().to_owned()),
        machine_id: juggler_app::commands::local_machine_id(state)
            .as_str()
            .to_owned(),
        root_pid: (depth > 0).then_some(root_pid).flatten(),
        parent_pid: (depth > 0).then_some(root_pid).flatten(),
        depth,
        role: role.to_owned(),
        state: "starting".to_owned(),
        listens_on: None,
        cwd: record.cwd.to_string_lossy().into_owned(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: now - 3_600_000,
        last_seen: now,
    }
}

/// Persona D4: a process left behind by ended lab session `id`, as AI
/// Juggler's process ledger records one. Its root (a real process that
/// ran in the session's folder and exited cleanly) and a [`lab_sleeper`]
/// of at most [`LEFTOVER_SECS`]: the sleep is the row "Still running
/// after its session closed". Both run in the session's folder under the
/// lab (`lab_scenario.sh teardown` stops what has a file open there).
/// Returns the leftover's process group. Unix only: a process group,
/// `/bin/sh` and its trap (labs run on a Mac).
#[cfg(unix)]
fn lab_leftover(
    state: &Arc<AppState>,
    id: &str,
    account: &str,
    home: &std::path::Path,
    notes: &mut Vec<String>,
) -> u32 {
    use juggler_supervisor::tree::LedgerStore;
    let (record, tool) = lab_session(state, id, account, home);
    // The session's root: a real process that has ended.
    let mut root = std::process::Command::new("/usr/bin/true")
        .current_dir(&record.cwd)
        .spawn()
        .unwrap();
    let root_pid = root.id();
    assert!(root.wait().unwrap().success());
    let (group, pid) = lab_sleeper(
        &record.cwd,
        home,
        &format!("aij-lab-leftover-{id}"),
        LEFTOVER_SECS,
    );
    let (exe, started_at) = os_identity(pid, "/bin/sleep");
    let ledger = &state.ledger;
    let boot = LedgerStore::boot_id(ledger.as_ref());
    let session = SessionId::new(id);
    let row = |role, pid, depth, exe: &std::path::Path, line: &str| {
        lab_row(
            state,
            &record,
            &tool,
            account,
            group,
            (role, pid, depth, Some(root_pid)),
            exe,
            line,
        )
    };
    ledger
        .intent(&row(
            "root",
            None,
            0,
            std::path::Path::new("/usr/bin/true"),
            &tool,
        ))
        .unwrap();
    ledger
        .spawned(&session, root_pid, Some(group), None)
        .unwrap();
    ledger.mark_exited(root_pid, &boot, "code:0", true).unwrap();
    let helper = juggler_core::storage::LedgerRow {
        started_at: Some(started_at),
        ..row(
            "helper",
            Some(pid),
            1,
            &exe,
            &format!("sleep {LEFTOVER_SECS}"),
        )
    };
    ledger.upsert_helpers(&session, &[helper]).unwrap();
    ledger.mark_orphaned(pid, &boot).unwrap();
    notes.push(format!("{id}: a leftover process, sleep {LEFTOVER_SECS} (pid {pid}), its session ended; teardown stops it"));
    group
}

/// Persona D4 elsewhere: a leftover is a Unix process group, so a seed
/// that asks for one says so instead of seeding half of it.
#[cfg(not(unix))]
fn lab_leftover(
    _state: &Arc<AppState>,
    id: &str,
    _account: &str,
    _home: &std::path::Path,
    _notes: &mut Vec<String>,
) -> u32 {
    panic!("{id}: `leftover = true` needs a Unix lab (a process group and /bin/sh)");
}

/// A lab session seeded working, waiting on its user or stuck, really
/// running: its root, a [`lab_sleeper`] of at most [`LIVE_SECS`] in the
/// session's folder under the lab, recorded in AI Juggler's process
/// ledger as the root it started (pid, group, OS start time, running).
/// The app's startup recovery verifies it by boot and start time and
/// picks the session up again, keeping its state, as it does a session
/// that outlived a quit; without it the restart rightly ends the
/// session ("AI Juggler quit while it ran"). Its conversation, if any,
/// stays linked. `lab_scenario.sh teardown` stops it (its folder is the
/// lab's), and stopping the session stops it. Returns its process group.
#[cfg(unix)]
fn lab_live(
    state: &Arc<AppState>,
    id: &str,
    account: &str,
    home: &std::path::Path,
    notes: &mut Vec<String>,
) -> u32 {
    use juggler_supervisor::tree::LedgerStore;
    let (record, tool) = lab_session(state, id, account, home);
    let (group, _) = lab_sleeper(&record.cwd, home, &format!("aij-lab-live-{id}"), LIVE_SECS);
    let (_, started_at) = os_identity(group, "/bin/sh");
    state
        .ledger
        .intent(&lab_row(
            state,
            &record,
            &tool,
            account,
            group,
            ("root", None, 0, None),
            std::path::Path::new("/bin/sh"),
            &tool,
        ))
        .unwrap();
    state
        .ledger
        .spawned(&SessionId::new(id), group, Some(group), Some(started_at))
        .unwrap();
    notes.push(format!(
        "{id}: running (pid {group}, at most {} minutes); teardown stops it",
        LIVE_SECS / 60
    ));
    group
}

/// A live lab session elsewhere: seeded, it ends with the first open.
#[cfg(not(unix))]
fn lab_live(
    _state: &Arc<AppState>,
    id: &str,
    _account: &str,
    _home: &std::path::Path,
    notes: &mut Vec<String>,
) -> u32 {
    notes.push(format!(
        "{id}: no running process outside a Unix lab; it ends when the app opens"
    ));
    0
}

/// A stable, uuid-shaped id for lab session `id` (Claude Code names a
/// conversation's file by one).
fn lab_uuid(id: &str) -> String {
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in id.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    let hex = format!("{hash:032x}");
    format!(
        "{}-{}-4{}-8{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

/// P8 (the rc26 walk): `shell_only = true | false` on an App is what the
/// walk expects its pool member to be labelled, checked against the
/// adapter's own answer for that App's folder
/// (`agent_bridge::tools_why_not`, what the "Shell only" label shows),
/// never a list of tools here. `None` when they agree or nothing is
/// claimed; else what's wrong. P8 said OpenCode was shell only after
/// AJ-E3002 R3 gave it the agent tools.
fn shell_only_disagrees(state: &Arc<AppState>, app: &Value, account_id: &str) -> Option<String> {
    let claimed = app["shell_only"].as_bool()?;
    let account = juggler_app::commands::find_account_arc(state, account_id).unwrap();
    let why = juggler_app::features::agent_bridge::tools_why_not(state, &account);
    let id = app["id"].as_str().unwrap_or_default();
    match (claimed, why) {
        (true, None) => Some(format!(
            "{id}: the scenario expects Shell only, but its adapter gives its launches the agent tools"
        )),
        (false, Some(why)) => Some(format!(
            "{id}: the scenario expects the agent tools, but its adapter says: {why}"
        )),
        _ => None,
    }
}

/// P6: each of `macs` paired with this Mac as the product keeps a
/// pairing: pinned in this Mac's signed peer list (`fleet/peers.json`)
/// under its own key, made in the lab (`<lab>/.aij-lab-macs/<name>`), by
/// its name, View only, never seen. Nothing runs it, so it is a paired
/// Mac that isn't connected: listed in Fleet and in a pool's Macs, and
/// unable to take work (pools across Macs are AJ-E1705's).
fn seed_paired_macs(
    state: &Arc<AppState>,
    macs: &[String],
    home: &std::path::Path,
    notes: &mut Vec<String>,
) {
    for name in macs {
        let keys = home.join(".aij-lab-macs").join(name);
        let other = juggler_link::MachineIdentity::load_or_create(&keys).unwrap();
        juggler_app::fleet::fleet_pin_for_test(
            state,
            juggler_link::Peer {
                machine_id: format!("lab-mac-{name}"),
                label: name.clone(),
                public_key: other.public().to_base64(),
                fingerprint: other.public().fingerprint(),
                permission: juggler_link::Permission::View,
                ask_each_time: false,
                paired_via: juggler_link::PairedVia::Code,
                paired_at_ms: Timestamp::now().millis(),
                last_seen_ms: None,
            },
        )
        .unwrap();
    }
    if !macs.is_empty() {
        notes.push(format!("paired Macs (not connected): {}", macs.join(", ")));
    }
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

/// A Mac as AI Juggler first sees it (the persona walks' first run,
/// AJ-E3307): no AI Juggler data at all; the tools' own folders where
/// they keep them, under the lab's HOME, each Claude Code one with a lab
/// sign-in on its `account` (made-up ids, a presence-only file: nothing
/// signs anything in). A CLI App's folder is `~/.claude` for profile
/// `default` (or none), else `~/.claude-<profile>`; Codex's `~/.codex`
/// (not signed in); a desktop App's its profile under ClaudeProfiles.
/// The projects are folders under `~/repos`.
fn seed_first_run(seed: &Value, home: &std::path::Path) -> String {
    for name in seed["projects"].as_array().cloned().unwrap_or_default() {
        if let Some(name) = name.as_str() {
            std::fs::create_dir_all(home.join("repos").join(name)).unwrap();
        }
    }
    let mut made = Vec::new();
    for app in seed["apps"].as_array().cloned().unwrap_or_default() {
        let tool = app["tool"].as_str().unwrap_or_default();
        let profile = app["profile"].as_str().unwrap_or("default");
        let dir = match tool {
            "claude-code" if profile == "default" => home.join(".claude"),
            "claude-code" => home.join(format!(".claude-{profile}")),
            "codex" => home.join(".codex"),
            "claude-desktop" => home
                .join("Library/Application Support/ClaudeProfiles")
                .join(profile),
            other => panic!("first run: no folder for tool {other:?}"),
        };
        std::fs::create_dir_all(&dir).unwrap();
        if tool == "claude-code"
            && let Some(account) = app["account"].as_str().filter(|name| !name.is_empty())
            && app["signed_in"].as_bool() != Some(false)
        {
            lab_sign_in(&dir, account);
        }
        made.push(dir.strip_prefix(home).unwrap().display().to_string());
    }
    format!(
        "first run: no AI Juggler data; the tools' folders {}",
        made.join(", ")
    )
}

/// A10: `signin = "unreadable"` leaves the lab folder's sign-in cut short,
/// and AI Juggler calls its Account "Account unknown" (its state file
/// couldn't be read), apart from the folder signed in on the same name.
#[test]
fn an_unreadable_sign_in_is_seeded_and_shown_as_account_unknown() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-a10-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let data = lab.path().join("data");
    let state = Arc::new(AppState::init_with_registry(data, default_registry()).unwrap());
    let mut recipes = HashMap::new();
    for id in ["cc-work", "cc-personal"] {
        let account =
            juggler_app::features::account::create_account(&state, "claude-code", id, None)
                .unwrap();
        lab_sign_in(&dir_of(&state, &account.id), "Personal");
        recipes.insert(id.to_owned(), (account.id, String::new()));
    }
    let apps: Vec<Value> = vec![
        serde_json::json!({"id": "cc-work", "tool": "claude-code", "account": "Personal"}),
        serde_json::json!({"id": "cc-personal", "tool": "claude-code", "account": "Personal",
            "signin": "unreadable"}),
    ];
    let mut notes = Vec::new();
    seed_unreadable_sign_ins(
        &state,
        &apps,
        &recipes,
        &[lab.path(), &state.data_dir],
        &mut notes,
    );
    assert_eq!(
        notes,
        ["cc-personal: its sign-in can't be read (cut short)"]
    );
    let damaged = dir_of(&state, &recipes["cc-personal"].0);
    assert!(
        matches!(
            juggler_adapters::claude_code::probe_claude_identity(&damaged),
            juggler_adapter::IdentityProbe::Unverifiable { ref reason }
                if reason == "Claude Code's state file couldn't be read"
        ),
        "the seeded sign-in can't be read"
    );
    let listed = juggler_app::features::subscriptions::list_impl(&state).unwrap();
    let unknown: Vec<_> = listed
        .iter()
        .filter(|account| account.display_name == "Account unknown")
        .collect();
    // The count only: an Account's record is never printed (CodeQL
    // rust/cleartext-logging).
    assert!(unknown.len() == 1, "exactly one Account unknown");
    assert_eq!(unknown[0].profiles.len(), 1);
    assert_eq!(unknown[0].profiles[0].id, recipes["cc-personal"].0);
    // The other folder on that name stays signed in, and its Account
    // counts it alone.
    let signed: Vec<_> = listed
        .iter()
        .filter(|account| account.state == "signed_in")
        .collect();
    assert_eq!(signed.len(), 1);
    assert_eq!(signed[0].profiles.len(), 1);
    assert_eq!(signed[0].profiles[0].id, recipes["cc-work"].0);
}

/// The seeder never cuts a sign-in outside the lab (an adopted real
/// folder, ADR-0027 condition 7): it says so and leaves it.
#[test]
fn an_unreadable_sign_in_is_never_made_outside_the_lab() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-a10-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let elsewhere = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = Arc::new(
        AppState::init_with_registry(lab.path().join("data"), default_registry()).unwrap(),
    );
    let account =
        juggler_app::features::account::create_account(&state, "claude-code", "x", None).unwrap();
    lab_sign_in(&dir_of(&state, &account.id), "Personal");
    let before = std::fs::read(juggler_adapters::claude_code::claude_state_file(&dir_of(
        &state,
        &account.id,
    )))
    .unwrap();
    let recipes = HashMap::from([("x".to_owned(), (account.id.clone(), String::new()))]);
    let apps = vec![serde_json::json!({"id": "x", "tool": "claude-code", "signin": "unreadable"})];
    let mut notes = Vec::new();
    // Its folder is under the lab's data; given only another root, it's
    // not the lab's.
    seed_unreadable_sign_ins(&state, &apps, &recipes, &[elsewhere.path()], &mut notes);
    assert_eq!(
        notes,
        ["x: its folder isn't the lab's; its sign-in left as is"]
    );
    let after = std::fs::read(juggler_adapters::claude_code::claude_state_file(&dir_of(
        &state,
        &account.id,
    )))
    .unwrap();
    assert_eq!(before, after);
}

/// P6: `macs = ["mini"]` pairs a Mac named mini with the lab's: Fleet
/// lists it, and New session (and a pool's Macs) says it can't take work
/// now, as a paired Mac that isn't connected does.
#[test]
fn a_scenarios_macs_are_paired_and_cant_act() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-p6-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let state = Arc::new(
        AppState::init_with_registry(lab.path().join("data"), default_registry()).unwrap(),
    );
    let mut notes = Vec::new();
    seed_paired_macs(&state, &["mini".to_owned()], lab.path(), &mut notes);
    assert_eq!(notes, ["paired Macs (not connected): mini"]);
    // As the app reads it after a restart: from the signed list on disk.
    juggler_app::fleet::fleet_unload_for_test(&state);
    let peers = juggler_app::fleet::fleet_peers_impl(&state).unwrap();
    assert_eq!(
        peers
            .iter()
            .map(|peer| peer.label.as_str())
            .collect::<Vec<_>>(),
        ["mini"]
    );
    assert_eq!(
        juggler_app::fleet::fleet_self_impl(&state).unwrap().trust,
        "trusted"
    );
    let options = juggler_app::features::fleet::launch_options_impl(&state).unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(
        options[0].why_not.as_deref(),
        Some("mini isn't connected right now")
    );
    // Its keys are the lab's.
    assert!(lab.path().join(".aij-lab-macs/mini").is_dir());
}

/// P8 (the rc26 walk): a scenario's `shell_only` must match what the
/// adapter answers for the App (what its pool member's label shows), and
/// the P8 scenario's own Apps do.
#[test]
fn a_scenarios_shell_only_claim_is_the_adapters() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-p8-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let state = Arc::new(
        AppState::init_with_registry(lab.path().join("data"), default_registry()).unwrap(),
    );
    let made = |tool: &str| {
        juggler_app::features::account::create_account(&state, tool, tool, None)
            .unwrap()
            .id
    };
    let (opencode, kimi) = (made("opencode"), made("kimi-code"));
    let claim = |id: &str, tool: &str, shell: bool| serde_json::json!({"id": id, "tool": tool, "shell_only": shell});
    // OpenCode takes the agent tools per launch (AJ-E3002 R3); Kimi Code
    // has none yet.
    assert_eq!(
        shell_only_disagrees(&state, &claim("opencode", "opencode", true), &opencode).as_deref(),
        Some(
            "opencode: the scenario expects Shell only, but its adapter gives its launches the agent tools"
        )
    );
    assert_eq!(
        shell_only_disagrees(&state, &claim("opencode", "opencode", false), &opencode),
        None
    );
    assert_eq!(
        shell_only_disagrees(&state, &claim("kimi", "kimi-code", true), &kimi),
        None
    );
    assert!(shell_only_disagrees(&state, &claim("kimi", "kimi-code", false), &kimi).is_some());
    assert_eq!(
        shell_only_disagrees(
            &state,
            &serde_json::json!({"id": "x", "tool": "opencode"}),
            &opencode
        ),
        None,
        "nothing claimed, nothing checked"
    );
    // The P8 scenario file itself, as the lab seeds it.
    let out = std::process::Command::new("python3")
        .args(["-I", "scripts/qa/scenario.py", "seed", "P8"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .unwrap();
    assert!(out.status.success(), "scenario.py seed P8");
    let seed: Value = serde_json::from_slice(&out.stdout).unwrap();
    let apps = seed["apps"].as_array().unwrap();
    let claimed: Vec<_> = apps
        .iter()
        .filter(|app| app["shell_only"].is_boolean())
        .collect();
    assert!(
        claimed.iter().any(|app| app["shell_only"] == true),
        "P8 has a shell-only scout"
    );
    for app in claimed {
        let account = made(app["tool"].as_str().unwrap());
        assert_eq!(shell_only_disagrees(&state, app, &account), None);
    }
}

/// The D5 persona walk: a scenario's repos are git repositories with one
/// commit (New session offers "Its own worktree" only in one), and making
/// one twice leaves its history alone.
#[test]
fn a_scenarios_repo_is_a_git_repository_with_one_commit() {
    let lab = tempfile::tempdir().unwrap();
    let plain: std::collections::HashSet<String> = ["notes".to_owned()].into();
    let root = lab_project(lab.path(), "billing", &plain);
    // Seeded again (a second setup step): its history is left alone.
    assert_eq!(lab_project(lab.path(), "billing", &plain), root);
    let notes = lab_project(lab.path(), "notes", &plain);
    assert!(
        notes.is_dir() && !notes.join(".git").exists(),
        "a plain folder, as the scenario says"
    );
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "billing is a git repository with a HEAD"
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "1",
        "one commit"
    );
    let dirty = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert!(dirty.stdout.is_empty(), "a clean tree");
}

/// The P5 rc31 walk: only Claude folders got a lab sign-in, so P5's two
/// Codex Apps on Personal were on no Account ("Personal · default needs
/// its sign-in") and its steps couldn't run. Every App of the P5 scenario
/// file, signed in as the seeder signs it in, is on its scenario Account:
/// Personal holds both Codex Apps, Work its Claude Code one.
#[test]
fn every_app_of_a_scenario_is_signed_in_to_its_account() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-p5-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let state = Arc::new(
        AppState::init_with_registry(lab.path().join("data"), default_registry()).unwrap(),
    );
    let out = std::process::Command::new("python3")
        .args(["-I", "scripts/qa/scenario.py", "seed", "P5"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .unwrap();
    assert!(out.status.success(), "scenario.py seed P5");
    let seed: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut on: HashMap<String, Vec<String>> = HashMap::new();
    for app in seed["apps"].as_array().unwrap() {
        let (id, tool) = (app["id"].as_str().unwrap(), app["tool"].as_str().unwrap());
        let name = app["account"].as_str().unwrap();
        let account =
            juggler_app::features::account::create_account(&state, tool, id, None).unwrap();
        assert!(
            lab_sign_in_for(tool, &dir_of(&state, &account.id), name),
            "{id}: {tool} has a lab sign-in"
        );
        on.entry(name.to_owned()).or_default().push(account.id);
    }
    let listed = juggler_app::features::subscriptions::list_impl(&state).unwrap();
    for (name, mut folders) in on {
        folders.sort();
        let found = listed.iter().find(|account| {
            account
                .profiles
                .iter()
                .any(|profile| folders.contains(&profile.id))
        });
        let found = found.unwrap_or_else(|| panic!("{name}: an Account"));
        assert_eq!(found.state, "signed_in", "{name}");
        let mut held: Vec<String> = found.profiles.iter().map(|p| p.id.clone()).collect();
        held.sort();
        assert_eq!(held, folders, "{name}: one Account holds its Apps");
    }
    // Codex's lab sign-in reads as a ChatGPT sign-in with its lab id.
    let codex = juggler_app::features::account::create_account(&state, "codex", "x", None).unwrap();
    lab_codex_sign_in(&dir_of(&state, &codex.id), "Personal");
    assert!(matches!(
        juggler_adapters::codex::probe_codex_identity(&dir_of(&state, &codex.id)),
        juggler_adapter::IdentityProbe::SignedIn { ref subject, .. } if subject == "lab-account-personal"
    ));
}

/// The P5 rc32 walk: Careful's Account member was the scenario's name
/// ("Personal") kept as the Account's id, so it matched no Account
/// ("Account unknown · 0 members") and p1 and p2, on Codex · Personal,
/// joined only the project pool. Seeded as the walk seeds it, Personal is
/// an Account holding its Codex Apps, and a Personal session is in
/// Careful, as Personal's.
#[test]
fn a_scenarios_account_member_is_the_account_its_apps_sign_in_to() {
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-p5-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let out = std::process::Command::new("python3")
        .args(["-I", "scripts/qa/scenario.py", "seed", "P5"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .unwrap();
    assert!(out.status.success(), "scenario.py seed P5");
    let mut seed: Value = serde_json::from_slice(&out.stdout).unwrap();
    // p1, as the walk starts it
    seed["sessions"] = serde_json::json!([
        {"id": "p1", "app": "codex-personal", "project": "web", "state": "idle"}
    ]);
    let home = lab.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let (state, _, groups) = seed_lab(&seed, lab.path().join("data"), &home);
    let _lab = Lab { dir: lab, groups };

    let accounts = juggler_app::features::subscriptions::list_with_apps_impl(&state).unwrap();
    let personal = accounts
        .iter()
        .find(|account| account.name.as_deref() == Some("Personal"))
        .expect("an Account named Personal");
    let apps = personal.apps.clone().unwrap_or_default();
    assert_eq!(apps.len(), 2, "Personal holds both Codex Apps: {apps:?}");
    assert!(apps.iter().all(|app| app.starts_with("cli:")), "{apps:?}");

    let storage = state.storage.lock().unwrap();
    let careful = storage
        .pools()
        .list()
        .unwrap()
        .into_iter()
        .find(|pool| pool.name == "Careful")
        .unwrap();
    let facts = storage
        .sessions()
        .pool_facts_of(&SessionId::new("p1"))
        .unwrap()
        .unwrap();
    assert_eq!(facts.account, personal.account_id, "p1 is Personal's");
    let resolution = storage.memberships().unwrap().resolve(&facts);
    assert!(
        resolution.in_pool(&careful.id).is_some(),
        "p1 is in Careful: {resolution:?}"
    );
    assert!(!resolution.account_unknown);
}

/// The scenario `id`'s setup, as `scripts/qa/scenario.py seed` gives it.
fn scenario_seed(id: &str) -> Value {
    let out = std::process::Command::new("python3")
        .args(["-I", "scripts/qa/scenario.py", "seed", id])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .unwrap();
    assert!(out.status.success(), "scenario.py seed {id}");
    serde_json::from_slice(&out.stdout).unwrap()
}

/// A test's lab: its folder, and the process groups its sessions run,
/// stopped when it goes (as teardown stops a walk's).
struct Lab {
    dir: tempfile::TempDir,
    groups: Vec<u32>,
}

impl Lab {
    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        for group in self.groups.iter().filter(|group| **group > 1) {
            // (One already stopped, as the leftover test's own, is fine.)
            let _ = std::process::Command::new("/bin/kill")
                .args(["-TERM", &format!("-{group}")])
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

/// Scenario `seed` seeded in a fresh lab (kept alive, and its processes
/// running, by the returned [`Lab`]).
fn seeded(seed: &Value, name: &str) -> (Lab, Arc<AppState>) {
    let dir = tempfile::Builder::new()
        .prefix(&format!("aij-lab-{name}-"))
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let (state, _, groups) = seed_lab(seed, dir.path().join("data"), &home);
    (Lab { dir, groups }, state)
}

/// Why the switch refuses a session with nothing to carry yet
/// (`switch::execute::no_conversation_words`), as its targets say it.
fn nothing_to_carry(reason: Option<&str>) -> bool {
    reason.is_some_and(|why| {
        why.starts_with("This session hasn't started a conversation yet")
            || why.starts_with("This session has no turn yet")
    })
}

/// The M4 walk on P1S6 (rc33): the out-of-room writer had no
/// conversation, so "Continue in another app" refused it ("hasn't started
/// a conversation yet") and step 2 couldn't be walked. Seeded now, it has
/// a transcript in Claude Code's own format under its App's folder that
/// the adapter reads as holding turns, and its switch targets no longer
/// refuse it for that.
#[test]
fn p1s6s_writer_is_seeded_with_a_conversation_to_carry() {
    let (_lab, state) = seeded(&scenario_seed("P1S6"), "p1s6");
    let writer = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new("writer"))
        .unwrap();
    let reference = writer.transcript_ref.clone().expect("a transcript");
    assert!(writer.conversation.is_some(), "its conversation's id");
    assert!(
        reference
            .path
            .components()
            .any(|part| part.as_os_str() == "projects")
            && reference.path.extension().is_some_and(|ext| ext == "jsonl"),
        "Claude Code's own place: {}",
        reference.path.display()
    );
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new("claude-code"))
        .unwrap();
    assert_eq!(adapter.holds_a_turn(&reference.path), Some(true));
    let targets =
        juggler_app::features::switch::targets::switch_targets_impl(&state, "writer").unwrap();
    assert!(!targets.is_empty(), "Codex · Personal is offered");
    assert!(
        targets
            .iter()
            .all(|target| !nothing_to_carry(target.filtered_reason.as_deref())),
        "{targets:?}"
    );
}

/// Item 3 of the rc33 M4 walk: "Claude Code · Personal · personal" was
/// never offered because P1S6 has no second Claude (only Codex on
/// Personal). Maya's own lab, PM4, has her two Claude subscriptions: once
/// the writer has turns, her other Claude is offered beside Codex, never
/// refused for the conversation. With no conversation, every target says
/// why, the one reason the page's doors give up front.
#[test]
fn pm4_offers_her_other_claude_once_the_session_has_turns() {
    let seed = scenario_seed("PM4");
    let (_lab, state) = seeded(&seed, "pm4");
    let personal = juggler_app::features::subscriptions::list_with_apps_impl(&state)
        .unwrap()
        .into_iter()
        .find(|account| account.name.as_deref() == Some("Personal"))
        .expect("an Account named Personal");
    let targets =
        juggler_app::features::switch::targets::switch_targets_impl(&state, "writer").unwrap();
    let other_claude: Vec<_> = targets
        .iter()
        .filter(|target| {
            target.provider == "claude-code"
                && personal
                    .profiles
                    .iter()
                    .any(|profile| profile.id == target.account_id)
        })
        .collect();
    assert_eq!(other_claude.len(), 1, "her other Claude: {targets:?}");
    assert!(
        targets
            .iter()
            .all(|target| !nothing_to_carry(target.filtered_reason.as_deref())),
        "{targets:?}"
    );

    // The same lab before its first message: offered, each saying why not.
    let mut fresh = seed;
    fresh["sessions"][0]["conversation"] = Value::Bool(false);
    let (_lab, state) = seeded(&fresh, "pm4-fresh");
    let targets =
        juggler_app::features::switch::targets::switch_targets_impl(&state, "writer").unwrap();
    assert!(
        targets
            .iter()
            .any(|target| target.provider == "claude-code")
    );
    assert!(
        targets
            .iter()
            .all(|target| nothing_to_carry(target.filtered_reason.as_deref())),
        "{targets:?}"
    );
}

/// The rc33 D3 walk: Devon's lab needs fifty-plus sessions over four
/// projects and one, in api days ago, that was fixing the login flow,
/// found by its title or by what was said. Seeded: it ended three days
/// back, and its transcript is Claude Code's own, titled and holding
/// both words.
#[test]
fn pd3s_login_session_is_found_by_its_title_and_what_was_said() {
    let (lab, state) = seeded(&scenario_seed("PD3"), "pd3");
    let sessions = state.storage.lock().unwrap().sessions().list(None).unwrap();
    assert!(sessions.len() >= 50, "{} sessions", sessions.len());
    let projects: BTreeSet<_> = sessions
        .iter()
        .filter_map(|row| row.project.clone())
        .collect();
    assert_eq!(projects.len(), 4);
    let login = sessions
        .iter()
        .find(|row| row.id.as_str() == "login-fix")
        .unwrap();
    assert_eq!(login.status, SessionStatus::Finished);
    assert!(login.cwd.ends_with("repos/api"));
    let days = (Timestamp::now().millis() - login.started_at.millis()) / (24 * 3_600_000);
    assert_eq!(days, 3);
    let reference = login.transcript_ref.clone().expect("a transcript");
    assert!(reference.path.starts_with(lab.path()));
    let text = std::fs::read_to_string(&reference.path).unwrap();
    let summary: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(summary["summary"], "Fix the login flow in the api");
    assert!(text.contains("The session cookie expires"));
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new("claude-code"))
        .unwrap();
    assert_eq!(adapter.holds_a_turn(&reference.path), Some(true));
}

/// Stops a lab leftover's process group however the test ends.
struct StopGroup(Option<u32>);

impl Drop for StopGroup {
    fn drop(&mut self) {
        if let Some(group) = self.0 {
            let _ = std::process::Command::new("/bin/kill")
                .args(["-TERM", &format!("-{group}")])
                .status();
        }
    }
}

/// The rc33 D4 brief: a leftover from an ended session, real and the
/// lab's own. Seeded, its `sleep` runs bounded in its own group, and
/// Cleanup lists it as left behind by its ended session — never a
/// working one; stopping its group (as teardown does) ends it.
#[test]
fn pd4s_leftover_is_listed_as_left_behind_by_its_ended_session() {
    use juggler_supervisor::tree::LedgerStore as _;
    let mut stop = StopGroup(None);
    let (_lab, state) = seeded(&scenario_seed("PD4"), "pd4");
    let rows = state
        .ledger
        .session_rows(&SessionId::new("billing-dev"))
        .unwrap();
    let helper = rows
        .iter()
        .find(|row| row.role == "helper")
        .expect("its leftover");
    stop.0 = helper.pgid;
    let pid = helper.pid.unwrap();
    assert_eq!(helper.state, "orphaned");
    assert!(helper.cmdline_trunc.starts_with("sleep "));
    let root = rows.iter().find(|row| row.role == "root").unwrap();
    assert_eq!(
        (root.state.as_str(), root.exit_kind.as_deref()),
        ("exited", Some("code:0"))
    );
    let alive = |pid: u32| {
        std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .status()
            .unwrap()
            .success()
    };
    assert!(alive(pid), "the leftover runs");
    let info = juggler_app::features::cleanup::inventory::cleanup_inventory_inner(
        &state,
        "needs-attention",
        false,
    );
    let listed: Vec<_> = info
        .groups
        .iter()
        .flat_map(|group| {
            group
                .rows
                .iter()
                .map(move |row| (group.session.as_str(), row.pid))
        })
        .collect();
    assert_eq!(listed, vec![("billing-dev", pid)], "{info:?}");
    drop(stop);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while alive(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "the trap took the sleep with it"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Persona R3: a week of sessions, spread evenly back over seven days.
#[test]
fn pr3s_week_of_sessions_spans_seven_days() {
    let (_lab, state) = seeded(&scenario_seed("PR3"), "pr3");
    let sessions = state.storage.lock().unwrap().sessions().list(None).unwrap();
    let now = Timestamp::now().millis();
    let oldest = sessions
        .iter()
        .map(|row| now - row.started_at.millis())
        .max()
        .unwrap();
    assert!(
        (6 * 24..=7 * 24).contains(&(oldest / 3_600_000)),
        "{oldest} ms back"
    );
    let days: BTreeSet<_> = sessions
        .iter()
        .map(|row| (now - row.started_at.millis()) / (24 * 3_600_000))
        .collect();
    assert_eq!(days.len(), 7);
}

/// The rc37 persona walks: PD4's working sessions and PM4's writer read
/// "Ended · AI Juggler quit while it ran" on the lab's first open, since
/// no process ran them. Seeded now, each runs a bounded process recorded
/// as its root, and the app's next open picks it up again in its state
/// (working, or waiting on its user), the writer's conversation still
/// linked to carry; the ended session with a leftover stays ended.
#[cfg(unix)]
#[test]
fn seeded_working_sessions_are_picked_up_running_when_the_app_opens() {
    let mut pd4 = scenario_seed("PD4");
    // A session waiting on its user, too (PD3's needs-you).
    pd4["sessions"][0]["state"] = Value::String("needs-you".to_owned());
    let cases = [
        (
            pd4,
            "pd4-live",
            vec![
                ("api-tests", SessionStatus::WaitingUser),
                ("web-refactor", SessionStatus::Running),
                ("billing-dev", SessionStatus::Finished),
            ],
        ),
        (
            scenario_seed("PM4"),
            "pm4-live",
            vec![("writer", SessionStatus::Running)],
        ),
    ];
    for (seed, name, expected) in cases {
        let (lab, state) = seeded(&seed, name);
        assert_eq!(
            lab.groups.len(),
            expected.len(),
            "{name}: one process group per running session or leftover"
        );
        drop(state);
        let reopened =
            AppState::init_with_registry(lab.path().join("data"), default_registry()).unwrap();
        let adopted = reopened.supervisor.alive_adopted_sessions();
        for (id, status) in expected {
            let record = reopened
                .storage
                .lock()
                .unwrap()
                .sessions()
                .get(&SessionId::new(id))
                .unwrap();
            assert_eq!(record.status, status, "{name}: {id}");
            assert_eq!(
                adopted.contains(&record.id),
                status != SessionStatus::Finished,
                "{name}: {id} picked up again: {adopted:?}"
            );
            if id == "writer" {
                assert!(
                    record.transcript_ref.is_some() && record.conversation.is_some(),
                    "its conversation stays linked"
                );
            }
        }
    }
}
