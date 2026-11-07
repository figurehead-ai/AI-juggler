//! L2 for pools (`docs/qa/0.4-validation-plan.md` §3; AJ-E3304): who is in
//! which pool, with what role and how, on data shaped like a heavy real
//! Mac (the replica), checked by the reference model
//! (`scripts/qa/pool_oracle.py l2-check`).
//!
//! Ignored, and a no-op unless `AIJ_QA_REAL_DATA`, `AIJ_QA_POOLS` (the
//! setup `pool_oracle.py l2-setup` wrote) and `AIJ_QA_REAL_OUT` are set:
//! `just replica-pools` runs it with HOME the replica's, never a real one.
//!
//! It does what the user would: first run brings every setup in where it
//! is; each conversation is continued in AI Juggler (the row Continue
//! writes, from the same checks and choices, without starting the tool),
//! except the ones the setup keeps as Watched; then the setup's pools are
//! made, scoped, sealed and filled through the commands the Pools screens
//! call. Then one Watched conversation is continued. What it records is
//! what the app answers: each session's pools (its Pools tab), each
//! pool's Watched members and member count, and every refusal in words.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use juggler_app::features::conversations::continue_target;
use juggler_app::features::pools::{self, model, watched};
use juggler_app::state::AppState;
use juggler_core::model::{
    ConversationId, InstanceKind, InstanceRecord, SessionId, SessionStatus, Timestamp,
};
use serde_json::{Value, json};

mod common;

/// One conversation the setup names: its account (by the replica's
/// label) and its id.
fn conv(value: &Value) -> (String, String) {
    (
        value["account"].as_str().unwrap().to_owned(),
        value["id"].as_str().unwrap().to_owned(),
    )
}

/// Records `id` (on the account `account_id`) as Continue would: its
/// target, then the row, ended. The launch itself is L4's (P-walks).
fn continue_row(state: &Arc<AppState>, account_id: &str, id: &str, n: usize) -> Value {
    match continue_target(state, account_id, id) {
        Ok(target) => {
            let session = SessionId::new(format!("sess-l2-{n:04}"));
            let machine = juggler_app::commands::local_machine_id(state);
            let storage = state.storage.lock().unwrap();
            storage
                .sessions()
                .insert(&InstanceRecord {
                    id: session.clone(),
                    conversation: Some(ConversationId::new(id)),
                    profile: target.profile.id.clone(),
                    machine,
                    kind: InstanceKind::Terminal,
                    project: Some(target.project.id.clone()),
                    cwd: target.cwd.clone(),
                    status: SessionStatus::Finished,
                    started_at: Timestamp::now(),
                    last_activity: None,
                    usage: juggler_core::model::UsageSnapshot::default(),
                    attention: None,
                    running: None,
                    window: None,
                    transcript_ref: None,
                })
                .unwrap();
            json!({"session": session.as_str()})
        }
        Err(error) => json!({"refused": error.to_string()}),
    }
}

#[test]
#[ignore = "L2 on the replica; needs AIJ_QA_REAL_DATA, AIJ_QA_POOLS and AIJ_QA_REAL_OUT"]
fn pools_on_the_replica() {
    let (Some(data), Some(setup), Some(out)) = (
        std::env::var_os("AIJ_QA_REAL_DATA").map(PathBuf::from),
        std::env::var_os("AIJ_QA_POOLS").map(PathBuf::from),
        std::env::var_os("AIJ_QA_REAL_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_REAL_DATA / AIJ_QA_POOLS / AIJ_QA_REAL_OUT not set: nothing to do");
        return;
    };
    let home = dirs::home_dir().unwrap_or_default();
    // Only the replica: it marks its HOME (`replica.py`'s MARKER).
    assert!(
        home.join(".replica").is_dir(),
        "pools L2 runs on the replica's HOME only"
    );
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_QA_REAL_DATA must be a fresh, empty folder"
    );
    let setup: Value = serde_json::from_slice(&std::fs::read(&setup).unwrap()).unwrap();
    let state = Arc::new(AppState::init(data).unwrap());
    let (_scan, imports) = common::first_run::bring_in_all(&state, &home, false);
    juggler_app::features::conversations::tick(&state).unwrap();

    // The replica's account labels, by the folder each was brought in from.
    let folders: HashMap<String, String> = setup["folders"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(label, folder)| {
            (
                folder.as_str().unwrap().trim_end_matches('/').to_owned(),
                label.clone(),
            )
        })
        .collect();
    let accounts = state.storage.lock().unwrap().accounts().list().unwrap();
    let account_of_label: HashMap<String, juggler_core::model::Account> = accounts
        .into_iter()
        .filter_map(|account| {
            let folder = account
                .state_dir
                .to_string_lossy()
                .trim_end_matches('/')
                .to_owned();
            Some((folders.get(&folder)?.clone(), account))
        })
        .collect();
    let label_of_account: HashMap<String, String> = account_of_label
        .iter()
        .map(|(label, account)| (account.id.as_str().to_owned(), label.clone()))
        .collect();

    // Continue in AI Juggler, every conversation the index lists but the
    // ones kept Watched.
    let kept: Vec<(String, String)> = setup["watched"]
        .as_array()
        .unwrap()
        .iter()
        .map(conv)
        .collect();
    let listed = juggler_app::features::conversations::commands::list_impl(&state).unwrap();
    let mut continued = Vec::new();
    let mut session_of: HashMap<(String, String), String> = HashMap::new();
    for (n, row) in listed.iter().enumerate() {
        let Some(label) = label_of_account.get(&row.account_id) else {
            continue;
        };
        let key = (label.clone(), row.id.clone());
        if kept.contains(&key) {
            continue;
        }
        let mut line = continue_row(&state, &row.account_id, &row.id, n);
        if let Some(session) = line["session"].as_str() {
            session_of.insert(key.clone(), session.to_owned());
        }
        line["account"] = key.0.into();
        line["id"] = key.1.into();
        continued.push(line);
    }

    // The ids the Pools screens offer, as the UI forms them.
    let apps = juggler_app::features::apps::list_impl(&state).unwrap();
    let subscriptions = juggler_app::features::subscriptions::list_impl(&state).unwrap();
    let mac = juggler_app::commands::local_machine_id(&state);
    let projects = juggler_app::features::projects::list::projects_list_impl(&state).unwrap();
    let project_path: HashMap<String, String> = projects
        .iter()
        .map(|p| (p.id.clone(), p.root.clone()))
        .collect();
    let project_of_path: HashMap<String, String> = projects
        .iter()
        .map(|p| (p.root.trim_end_matches('/').to_owned(), p.id.clone()))
        .collect();
    let member_wire = |member: &Value| -> String {
        let account = || &account_of_label[member["account"].as_str().unwrap()];
        match member["kind"].as_str().unwrap() {
            "app" => {
                let account = account();
                let app = apps
                    .iter()
                    .find(|app| app.folder_id.as_deref() == Some(account.id.as_str()))
                    .expect("the folder's App");
                format!("app:{}", app.id)
            }
            "account" => {
                let account = account();
                let sub = subscriptions
                    .iter()
                    .find(|sub| sub.profiles.iter().any(|p| p.id == account.id.as_str()))
                    .expect("the folder's Account");
                // The stored Account, as New pool sends it.
                format!(
                    "account:{}",
                    sub.account_id.as_deref().expect("a stored Account")
                )
            }
            // A Tool is its provider, the harness id (`accounts.provider_id`).
            "tool" => format!("tool:{}", member["tool"].as_str().unwrap()),
            "mac" => format!("mac:{}", mac.as_str()),
            "everyone" => "all".to_owned(),
            "session" => {
                let key = conv(member);
                format!("session:{}", session_of[&key])
            }
            "conversation" => {
                let account = account();
                format!(
                    "conversation:{}/{}",
                    account.id.as_str(),
                    member["id"].as_str().unwrap()
                )
            }
            other => panic!("{other} isn't a member kind"),
        }
    };

    // The setup's pools, through the commands the screens call.
    let mut pool_ids: HashMap<String, String> = HashMap::new();
    let mut steps = Vec::new();
    for pool in setup["pools"].as_array().unwrap() {
        let name = pool["name"].as_str().unwrap();
        let made = pools::pool_create_impl(&state, name, None).unwrap();
        let id = made
            .pools
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .id
            .clone();
        pool_ids.insert(name.to_owned(), id.clone());
        let (every, scope) = match &pool["scope"] {
            Value::String(word) if word == "every" => (true, Vec::new()),
            Value::Array(paths) => (
                false,
                paths
                    .iter()
                    .map(|path| {
                        let path = path.as_str().unwrap().trim_end_matches('/');
                        Some(
                            project_of_path
                                .get(path)
                                .cloned()
                                .unwrap_or_else(|| panic!("no project at {path}")),
                        )
                    })
                    .collect(),
            ),
            other => panic!("{other} isn't a scope"),
        };
        model::set_scope_impl(&state, &id, every, &scope).unwrap();
        model::set_sealed_impl(&state, &id, pool["sealed"].as_bool().unwrap_or(false)).unwrap();
        model::set_skip_held_impl(&state, &id, pool["skip_held"].as_bool().unwrap_or(false))
            .unwrap();
    }
    for join in setup["joins"].as_array().unwrap() {
        let pool = join["pool"].as_str().unwrap();
        let wire = member_wire(&join["member"]);
        let role = join["role"].as_str().unwrap();
        let result = model::join_impl(&state, &pool_ids[pool], &wire, role, None, &[]);
        steps.push(json!({
            "step": join["step"],
            "ok": result.is_ok(),
            "refused": result.err().map(|error| error.to_string()),
        }));
    }

    let watched_now = |state: &Arc<AppState>| -> Value {
        let list = pools::pools_list_impl(state).unwrap();
        let mut out = serde_json::Map::new();
        for (name, id) in &pool_ids {
            let members = list
                .pools
                .iter()
                .find(|p| &p.id == id)
                .map_or(0, |p| p.members);
            let rows: Vec<Value> = watched::watched_impl(state, id)
                .unwrap()
                .into_iter()
                .map(|w| {
                    json!({
                        "account": label_of_account.get(&w.folder),
                        "id": w.conversation_id,
                        "title": w.title,
                        "tool": w.tool,
                        "role": w.role,
                        "continued": w.continued_session.is_some(),
                    })
                })
                .collect();
            out.insert(name.clone(), json!({"members": members, "watched": rows}));
        }
        out.into()
    };
    let before = watched_now(&state);
    // Continue in AI Juggler, one Watched conversation.
    let mut later = Vec::new();
    for one in setup["continue_later"].as_array().unwrap() {
        let key = conv(one);
        let account = &account_of_label[&key.0];
        let mut line = continue_row(&state, account.id.as_str(), &key.1, 9000 + later.len());
        if let Some(session) = line["session"].as_str() {
            session_of.insert(key.clone(), session.to_owned());
        }
        line["account"] = key.0.into();
        line["id"] = key.1.into();
        later.push(line);
    }
    let after = watched_now(&state);

    // Each session's Pools tab, in the replica's words: pools by name,
    // members by their kind.
    let name_of: HashMap<String, String> = pools::pools_list_impl(&state)
        .unwrap()
        .pools
        .into_iter()
        .map(|p| {
            let name = match (&p.kind[..], &p.project_id) {
                ("default", Some(project)) => format!(
                    "project:{}",
                    project_path.get(project).cloned().unwrap_or_default()
                ),
                ("default", None) => "project:none".to_owned(),
                _ => p.name,
            };
            (p.id, name)
        })
        .collect();
    let mut sessions = Vec::new();
    for ((label, id), session) in &session_of {
        let tab = model::session_pools_impl(&state, session).unwrap();
        sessions.push(json!({
            "account": label,
            "id": id,
            "account_unknown": tab.account_unknown,
            "pools": tab.pools.iter().map(|p| json!({
                "pool": name_of.get(&p.pool_id), "role": p.role, "via": p.via, "sealed": p.sealed,
            })).collect::<Vec<_>>(),
            "skipped": tab.skipped.iter().map(|s| json!({
                "pool": name_of.get(&s.pool_id), "reason": s.reason,
                "by": s.by_pool.as_ref().and_then(|by| name_of.get(by)),
            })).collect::<Vec<_>>(),
        }));
    }

    let report = json!({
        "imports": imports,
        "continued": continued,
        "joins": steps,
        "watched_before": before,
        "continued_later": later,
        "watched_after": after,
        "sessions": sessions,
    });
    std::fs::write(&out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    eprintln!("wrote {}", out.display());
}
