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

use juggler_app::commands;
use juggler_app::state::{AppState, default_registry};
use juggler_core::agent_access::AccessLevel;
use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, ProjectId, SessionId, SessionStatus,
    Timestamp,
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
        other => panic!("unknown session state {other:?}"),
    }
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
    for app in seed["apps"].as_array().cloned().unwrap_or_default() {
        let (id, tool) = (app["id"].as_str().unwrap(), app["tool"].as_str().unwrap());
        if tool == "claude-desktop" {
            notes.push(format!(
                "{id}: a desktop app's profile comes from the running app — start it in the lab"
            ));
            continue;
        }
        let label = format!(
            "{} · {}",
            app["account"].as_str().unwrap_or(""),
            app["profile"].as_str().unwrap_or(id)
        );
        let Ok(account) = commands::create_account(&state, tool, &label, None) else {
            notes.push(format!(
                "{id}: {tool} isn't a tool this build knows; left out"
            ));
            continue;
        };
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
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
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
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
                Some((account, _)) => MemberRef::App(account.clone()),
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
