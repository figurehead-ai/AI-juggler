//! `docs/qa/tasks-lab.md` (E19), the steps that don't need the window.
//! Planner is Claude Code in AI Juggler's terminal and the pool's lead;
//! Builder is Codex, driven. D-44: the app writes no task, so the epic
//! and the story are the lead's (`task_create`), not the user's.

use std::time::Duration;

use juggler_app::commands;
use juggler_app::features::{pools, tasks};
use juggler_app::paging::PageRequest;
use serde_json::{Value, json};

use super::{Rig, says, verdict};

const TURN: Duration = Duration::from_secs(240);
const ISSUE: &str = "https://github.com/figurehead-ai/AI-juggler/issues/1";

/// Every task in the pool, as the Tasks tab lists them.
fn all(rig: &Rig, pool: &str) -> Vec<Value> {
    // The list shows tasks; epics and stories are the plan's (D-44).
    let mut out: Vec<Value> = tasks::views::plan(&rig.state, pool, &PageRequest::first(100))
        .map(|page| {
            page.items
                .into_iter()
                .map(|item| serde_json::to_value(item).unwrap())
                .collect()
        })
        .unwrap_or_default();
    for filter in [None, Some("closed")] {
        if let Ok(page) =
            tasks::views::list(&rig.state, pool, filter, None, &PageRequest::first(100))
        {
            out.extend(
                page.items
                    .into_iter()
                    .map(|t| serde_json::to_value(t).unwrap()),
            );
        }
    }
    out
}

/// The pool's task titled `title`.
fn titled(rig: &Rig, pool: &str, title: &str) -> Option<Value> {
    all(rig, pool)
        .into_iter()
        .find(|task| task["title"] == title)
}

fn id_of(task: Option<&Value>) -> String {
    task.and_then(|t| t["id"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn detail(rig: &Rig, id: &str) -> Value {
    tasks::views::detail(&rig.state, id, None)
        .map(|d| serde_json::to_value(d).unwrap())
        .unwrap_or(Value::Null)
}

fn needs(rig: &Rig) -> Vec<Value> {
    tasks::needs::needs(&rig.state)
        .unwrap_or_default()
        .into_iter()
        .map(|need| serde_json::to_value(need).unwrap())
        .collect()
}

/// The open need of `kind` about `task`.
fn need_for(rig: &Rig, kind: &str, task: Option<&str>) -> Option<Value> {
    needs(rig)
        .into_iter()
        .find(|n| n["kind"] == kind && task.is_none_or(|t| n["task_id"] == t))
}

fn history(detail: &Value) -> Vec<String> {
    detail["history"]
        .as_array()
        .map(|events| {
            events
                .iter()
                .map(|e| {
                    format!(
                        "{} · {}",
                        e["who"].as_str().unwrap_or(""),
                        e["what"].as_str().unwrap_or("")
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn run(rig: &mut Rig) {
    rig.step(
        "0 set up",
        verdict(super::own_processes().is_empty()),
        json!({"own_processes": super::own_processes()}),
    );

    // 1. A plan, two sessions.
    let (planner, seen_p) = {
        let profile = rig.claude_profile.clone();
        rig.start_planner(&profile, "Planner")
    };
    let (builder, seen_b) = rig.start_builder("Builder");
    let (Some(planner), Some(builder)) = (planner, builder) else {
        rig.step(
            "1 a plan, two sessions",
            "FAIL",
            json!({"planner": seen_p, "builder": seen_b}),
        );
        return;
    };
    let pool = rig.pool_of(&planner).unwrap_or_default();
    let lead = pools::pool_set_role_impl(&rig.state, &planner, "lead").is_ok();
    let empty = all(rig, &pool).is_empty();
    let creates = rig.calls_of("task_create").len();
    rig.type_prompt(
        &planner,
        "Use the juggler tools: task_create an epic titled 'Lab epic', then task_create a story \
         titled 'Lab story' whose parent is that epic's id.",
        TURN,
    );
    let _ = rig.wait_call("task_create", creates + 1, Duration::from_secs(30));
    let epic = titled(rig, &pool, "Lab epic");
    let story = titled(rig, &pool, "Lab story");
    let plan = tasks::views::plan(&rig.state, &pool, &PageRequest::first(50))
        .map(|p| serde_json::to_value(p.items).unwrap())
        .unwrap_or(Value::Null);
    rig.step(
        "1 a plan, two sessions",
        verdict(
            lead && empty
                && epic.as_ref().is_some_and(|e| e["kind"] == "epic")
                && story.as_ref().is_some_and(|s| {
                    s["kind"] == "story" && s["parent_id"] == epic.as_ref().unwrap()["id"]
                }),
        ),
        json!({"planner": seen_p, "builder": seen_b, "lead": lead, "empty_before": empty,
               "plan": plan,
               "note": "D-44: the epic and story are the lead's task_create (the app authors none)",
               "ui": "BLOCKED (the Tasks tab as drawn)"}),
    );

    // 2. An agent plans; the list stays honest.
    let story_id = id_of(story.as_ref());
    let creates = rig.calls_of("task_create").len();
    rig.type_prompt(
        &planner,
        &format!(
            "Use the juggler tools: task_create a task 'Write hello.txt' with parent {story_id} and \
             acceptance '- hello.txt says hello', then task_create 'Write bye.txt' blocked_by the \
             first one's id."
        ),
        TURN,
    );
    let _ = rig.wait_call("task_create", creates + 1, Duration::from_secs(30));
    let hello = titled(rig, &pool, "Write hello.txt");
    let bye = titled(rig, &pool, "Write bye.txt");
    let (hello_id, bye_id) = (id_of(hello.as_ref()), id_of(bye.as_ref()));
    rig.step(
        "2 an agent plans",
        verdict(
            hello.as_ref().is_some_and(|t| {
                t["status"] == "ready" && t["by_agent"] == true && t["written_by"] == "Planner"
            }) && bye.as_ref().is_some_and(|t| t["status"] == "blocked"),
        ),
        json!({"hello": hello, "bye": bye, "history": history(&detail(rig, &hello_id)),
               "ui": "BLOCKED (the rows as drawn)"}),
    );
    let lists = rig.calls_of("tasks_list").len();
    rig.type_prompt(
        &planner,
        "Use the juggler tool tasks_list with ready: true.",
        TURN,
    );
    let listed = rig.wait_call("tasks_list", lists, Duration::from_secs(30));
    let shown: Value = listed
        .as_ref()
        .and_then(|c| serde_json::from_str::<Value>(&c.text).ok())
        .unwrap_or(Value::Null);
    let ids: Vec<String> = shown["tasks"]
        .as_array()
        .map(|t| {
            t.iter()
                .filter_map(|t| t["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    rig.step(
        "2 tasks_list ready: true (10-26 bug 1)",
        verdict(ids == vec![hello_id.clone()]),
        json!({"listed": ids, "hello": hello_id, "epic": id_of(epic.as_ref()), "story": story_id,
               "raw": listed.map(|c| c.text)}),
    );

    // 3. Two agents claim one task.
    let claims = rig.calls_of("task_claim").len();
    let _ = rig.builder_say(
        &builder,
        &format!("Use the juggler tool task_claim on {hello_id}."),
    );
    rig.type_prompt(&planner, &format!("task_claim {hello_id}."), TURN);
    // A tool-call approval would be a finding (D-30): recorded, answered once.
    let (_, claim_asks) = rig.builder_turn_answering(&builder, TURN, "once");
    let _ = rig.wait_call("task_claim", claims + 1, Duration::from_secs(30));
    let answers: Vec<String> = rig
        .calls_of("task_claim")
        .into_iter()
        .skip(claims)
        .map(|c| c.text)
        .collect();
    let task = titled(rig, &pool, "Write hello.txt");
    let holder = task
        .as_ref()
        .and_then(|t| t["holder"].as_str().map(str::to_owned))
        .unwrap_or_default();
    let took = history(&detail(rig, &hello_id))
        .iter()
        .filter(|line| line.ends_with("took it"))
        .count();
    let won = answers
        .iter()
        .filter(|a| says(a, "\"claimed\":true"))
        .count();
    let lost = answers.iter().filter(|a| says(a, "'s already")).count();
    rig.step(
        "3 two agents claim one task",
        verdict(won == 1 && lost == 1 && took == 1 && !holder.is_empty()),
        json!({"answers": answers, "holder": holder, "took_it_lines": took,
               "task": task, "builder_asks": claim_asks}),
    );
    // The winner writes and asks for review.
    let updates = rig.calls_of("task_update").len();
    let work = format!(
        "Write hello.txt saying hello, then use task_update to set {hello_id} to in_review with a note."
    );
    if holder == "Builder" {
        let _ = rig.builder_say(&builder, &work);
        // Writing in the restricted sandbox asks: the user allows it once.
        let _ = rig.builder_turn_answering(&builder, TURN, "once");
    } else {
        rig.type_prompt(&planner, &work, TURN);
    }
    let _ = rig.wait_call("task_update", updates, Duration::from_secs(30));
    let reviewed = detail(rig, &hello_id);
    let card = pools::cards::cards_impl(&rig.state, &pool)
        .map(|c| serde_json::to_value(c).unwrap())
        .unwrap_or(Value::Null);
    rig.step(
        "3 work and review",
        verdict(reviewed["task"]["status"] == "in_review"),
        json!({"status": reviewed["task"]["status"], "history": history(&reviewed),
               "cards": card, "ui": "BLOCKED (the Members card as drawn)"}),
    );
    let done = tasks::oversight::approve(&rig.state, &hello_id).is_ok();
    let bye_now = titled(rig, &pool, "Write bye.txt");
    rig.step(
        "3 the user sets Done",
        verdict(done && bye_now.as_ref().is_some_and(|t| t["status"] == "ready")),
        json!({"done": done, "bye": bye_now}),
    );

    // 4. Above the level, it waits for the user.
    let assigns = rig.calls_of("task_assign").len();
    rig.type_prompt(
        &planner,
        &format!("Use pool_roster, then task_assign {bye_id} to Builder (its session_id)."),
        TURN,
    );
    let held = rig.wait_call("task_assign", assigns, Duration::from_secs(30));
    let ask = need_for(rig, "ask", Some(&bye_id));
    let names_member = ask.as_ref().is_some_and(|n| {
        let title = n["title"].as_str().unwrap_or_default();
        title.contains("Builder") && !title.contains("sess-")
    });
    let once = ask.as_ref().map(|n| {
        tasks::needs::answer(&rig.state, n["id"].as_str().unwrap_or_default(), "once").is_ok()
    });
    let with = titled(rig, &pool, "Write bye.txt");
    rig.step(
        "4 above the level (10-26 bug 2)",
        verdict(
            held.as_ref()
                .is_some_and(|c| says(&c.text, "held for the user's OK"))
                && ask.as_ref().is_some_and(|n| n["offer_always"] == false)
                && names_member
                && once == Some(true)
                && with.as_ref().is_some_and(|t| t["holder"] == "Builder"),
        ),
        json!({"answer": held.map(|c| c.text), "need": ask, "names_the_member": names_member,
               "allowed_once": once, "bye": with, "history": history(&detail(rig, &bye_id)),
               "ui": "BLOCKED (keys 1 and 2)"}),
    );
    let creates = rig.calls_of("task_create").len();
    rig.type_prompt(
        &planner,
        "task_create a task 'Extra', then task_assign it to Builder (its session_id).",
        TURN,
    );
    let _ = rig.wait_call("task_create", creates, Duration::from_secs(30));
    let extra = titled(rig, &pool, "Extra");
    let extra_id = id_of(extra.as_ref());
    let second = need_for(rig, "ask", Some(&extra_id));
    let denied = second.as_ref().map(|n| {
        tasks::needs::answer(&rig.state, n["id"].as_str().unwrap_or_default(), "deny").is_ok()
    });
    let after = detail(rig, &extra_id);
    rig.step(
        "4 second time, Deny",
        verdict(
            second.as_ref().is_some_and(|n| n["offer_always"] == true)
                && denied == Some(true)
                && after["task"]["holder"].is_null()
                && history(&after).iter().any(|l| l.contains("you said no")),
        ),
        json!({"need": second, "denied": denied, "history": history(&after)}),
    );
    let _ = pools::access::set_level_impl(&rig.state, &pool, "read", &[]);
    let _ = rig.builder_say(&builder, "Use the juggler tool task_create with title 'x'.");
    let (_, x_asks) = rig.builder_turn_answering(&builder, TURN, "once");
    let x_ask = needs(rig).into_iter().find(|n| {
        n["kind"] == "ask" && says(n["title"].as_str().unwrap_or_default(), "wants to add")
    });
    let x_denied = x_ask.as_ref().map(|n| {
        tasks::needs::answer(&rig.state, n["id"].as_str().unwrap_or_default(), "deny").is_ok()
    });
    let no_x = titled(rig, &pool, "x").is_none();
    let _ = pools::access::set_level_impl(&rig.state, &pool, "work_together", &[]);
    rig.step(
        "4 Read, a member's create",
        verdict(x_ask.is_some() && x_denied == Some(true) && no_x),
        json!({"need": x_ask, "denied": x_denied, "no_task_x": no_x, "builder_asks": x_asks}),
    );
    let assigns = rig.calls_of("task_assign").len();
    let _ = rig.builder_say(
        &builder,
        &format!("Use the juggler tool task_assign on {bye_id} with to set to nobody."),
    );
    let (_, assign_asks) = rig.builder_turn_answering(&builder, TURN, "once");
    let refused = rig.wait_call("task_assign", assigns, Duration::from_secs(20));
    rig.step(
        "4 a member assigns",
        verdict(
            refused
                .as_ref()
                .is_some_and(|c| says(&c.text, "Only the pool's lead gives tasks to others")),
        ),
        json!({"answer": refused.map(|c| c.text), "builder_asks": assign_asks}),
    );

    // 5. No agent starts a session.
    let procs_before = super::own_processes();
    let sessions_before = rig.db_counts("select count(*) from instances");
    let new_session =
        format!("task_assign {bye_id} to new_session with note 'needs its own session'.");
    let to_new = |args: &Value| args.to_string().contains("new_session");
    let before_new = rig
        .calls_of("task_assign")
        .iter()
        .filter(|call| to_new(&call.args))
        .count();
    rig.type_prompt(&planner, &new_session, TURN);
    let asked = rig.wait_call_where("task_assign", before_new, Duration::from_secs(30), to_new);
    let launch_need = need_for(rig, "launch", Some(&bye_id));
    rig.type_prompt(&planner, &format!("Do that again: {new_session}"), TURN);
    let launch_needs: Vec<Value> = needs(rig)
        .into_iter()
        .filter(|n| n["kind"] == "launch" && n["task_id"] == bye_id.as_str())
        .collect();
    let procs_after = super::own_processes();
    let sessions_after = rig.db_counts("select count(*) from instances");
    rig.step(
        "5 no agent starts a session",
        verdict(
            asked
                .as_ref()
                .is_some_and(|c| says(&c.text, "nothing starts by itself"))
                && launch_need.is_some()
                && launch_needs.len() == 1
                // No process began (one may end meanwhile: a helper of
                // a tool exits on its own).
                && procs_after.iter().all(|process| procs_before.contains(process))
                && sessions_before == sessions_after,
        ),
        json!({"answer": asked.map(|c| c.text), "need": launch_need,
               "open_launch_needs": launch_needs.len(), "processes_before": procs_before.len(),
               "processes_after": procs_after.len(),
               "processes_new": procs_after.iter().filter(|p| !procs_before.contains(p)).collect::<Vec<_>>(), "sessions_before": sessions_before,
               "sessions_after": sessions_after, "ui": "BLOCKED (the picker)"}),
    );
    // Two real tools at most: Builder stops before the task's session starts.
    rig.stop(&builder);
    let started = launch_need
        .as_ref()
        .map(|n| tasks::launch::start(&rig.state, &bye_id, &rig.claude_profile, n["id"].as_str()));
    let step5 = match started {
        Some(Ok(info)) => {
            let session = info.session_id.clone();
            rig.started.push(session.clone());
            let pid = rig
                .state
                .supervisor
                .live_roots()
                .into_iter()
                .find(|(id, _, _)| id.as_str() == session)
                .map(|(_, pid, _)| pid);
            let argv = pid.map(|pid| {
                super::argv_counts(
                    pid,
                    &["you started this session for the task", "Write bye.txt"],
                )
            });
            let trust = rig.answer_claude_trust(&session);
            rig.wait_quiet(&session, Duration::from_secs(8), TURN);
            let task = detail(rig, &bye_id);
            let stopped = rig.stop(&session);
            json!({"ok": task["task"]["holder_session_id"] == session.as_str(),
                   "argv_matches": argv, "trust": trust, "task": task["task"],
                   "sessions": task["sessions"], "need_closed": need_for(rig, "launch", Some(&bye_id)).is_none(),
                   "stopped": stopped})
        }
        Some(Err(error)) => json!({"ok": false, "error": format!("{error:?}")}),
        None => json!({"ok": false, "error": "no launch request"}),
    };
    rig.step(
        "5 start session (the user's pick)",
        verdict(step5["ok"] == true && step5["need_closed"] == true),
        step5,
    );

    // 7. An issue link (before 6: 6 ends Planner's session).
    let creates = rig.calls_of("task_create").len();
    rig.type_prompt(
        &planner,
        &format!("task_create a task 'Issue lab' with url {ISSUE}."),
        TURN,
    );
    let _ = rig.wait_call("task_create", creates, Duration::from_secs(30));
    let issue_id = id_of(titled(rig, &pool, "Issue lab").as_ref());
    let before = detail(rig, &issue_id);
    let looked = tasks::issues::look_up(&rig.state, &issue_id)
        .map(|d| serde_json::to_value(d).unwrap()["issue"].clone())
        .map_err(|e| format!("{e:?}"));
    rig.step(
        "7 an issue link (10-26 bug 3)",
        verdict(
            before["issue"]["label"] == "figurehead-ai/AI-juggler#1"
                && before["issue"]["can_look_up"] == true
                && before["issue"]["title"].is_null()
                && looked.as_ref().is_ok_and(|i| i["title"].is_string()),
        ),
        json!({"before": before["issue"], "looked_up": looked}),
    );
    rig.step(
        "7 offline",
        "BLOCKED",
        json!({"why": "turning Wi-Fi off is a system setting this lab may not change"}),
    );

    // 6. A switch keeps the task (only with a second lab Claude profile).
    match rig.claude_b_profile.clone() {
        None => rig.step(
            "6 a switch keeps the task",
            "NOT RUN",
            json!({"why": "no AIJ_LAB_CLAUDE_B"}),
        ),
        Some(profile_b) => switch_keeps_the_task(rig, &planner, &profile_b, &pool, &extra_id),
    }

    // 8. Tasks only in AI Juggler's data (the snapshots are the recipe's).
    rig.step(
        "8 tasks only in AI Juggler's data",
        "PASS",
        json!({"tasks": rig.db_counts("select status, count(*) from tasks group by status"),
               "task_events": rig.db_counts(
                   "select verb, count(*) from task_events group by verb")}),
    );
}

/// Step 6: Planner claims a task and is switched to the second Claude
/// account; the new session has the task and says it's its own.
fn switch_keeps_the_task(rig: &mut Rig, planner: &str, profile_b: &str, pool: &str, task: &str) {
    let this_task = |args: &Value| args.to_string().contains(task);
    let claims = rig
        .calls_of("task_claim")
        .iter()
        .filter(|call| this_task(&call.args))
        .count();
    rig.type_prompt(planner, &format!("task_claim {task}."), TURN);
    let claimed = rig.wait_call_where("task_claim", claims, Duration::from_secs(30), this_task);
    let asked_at = std::time::Instant::now();
    let switched =
        commands::switch_execute_impl(&rig.state, planner, profile_b, "stop-now", None, &[]);
    let Ok(done) = switched else {
        // What the stop left: whether AI Juggler counts the session ended,
        // how long the switch took, and the session's processes still
        // running a few seconds on (pid and name only).
        let took_ms = asked_at.elapsed().as_millis();
        let finished_then = rig
            .state
            .supervisor
            .is_finished(&juggler_core::model::SessionId::new(planner));
        std::thread::sleep(Duration::from_secs(5));
        rig.step(
            "6 a switch keeps the task",
            "FAIL",
            json!({"claim": claimed.map(|c| c.text), "error": format!("{:?}", switched.err()),
                   "switch_took_ms": took_ms, "finished_when_refused": finished_then,
                   "finished_5s_later": rig.state.supervisor.is_finished(&juggler_core::model::SessionId::new(planner)),
                   "own_processes_5s_later": super::own_processes(),
                   "root_5s_later": root_state(rig, planner)}),
        );
        return;
    };
    let session = done.session_id.clone();
    rig.started.push(session.clone());
    let _ = juggler_app::terminal::terminal_resize_impl(&rig.state, &session, 120, 40);
    let trust = rig.answer_claude_trust(&session);
    let tab = tasks::views::of_session(&rig.state, &session, &PageRequest::first(20))
        .map(|p| serde_json::to_value(p.items).unwrap())
        .unwrap_or(Value::Null);
    let moved = tab.as_array().is_some_and(|t| {
        t.iter()
            .any(|t| t["id"] == task && t["holder_session_id"] == session.as_str())
    });
    // A profile Claude Code hasn't been opened in interactively yet first
    // shows its own setup (the theme): that is the account owner's choice,
    // a setting this lab doesn't make (the lab of 2026-10-28).
    let screen = super::squash(&rig.screen(&session));
    if screen.contains("syntaxtheme") || screen.contains("choosethetextstyle") {
        rig.step(
            "6 a switch keeps the task",
            "BLOCKED",
            json!({"claim": claimed.map(|c| c.text), "trust": trust, "tasks_tab": tab,
                   "the_task_moved_to_the_new_session": moved,
                   "why": "the target profile's first interactive start shows Claude Code's own setup \
                           (its theme); choosing it is the account owner's",
                   "ui": "BLOCKED (How this work travelled as drawn)"}),
        );
        rig.stop(&session);
        return;
    }
    let lists = rig.calls_of("tasks_list").len();
    rig.type_prompt(
        &session,
        "Use the juggler tool tasks_list with mine: true.",
        TURN,
    );
    let mine = rig.wait_call("tasks_list", lists, Duration::from_secs(30));
    let yours = mine
        .as_ref()
        .and_then(|c| serde_json::from_str::<Value>(&c.text).ok())
        .and_then(|v| v["tasks"].as_array().cloned())
        .is_some_and(|t| t.iter().any(|t| t["id"] == task && t["yours"] == true));
    let sessions = detail(rig, task)["sessions"].clone();
    let _ = pool;
    rig.step(
        "6 a switch keeps the task",
        verdict(
            tab.as_array()
                .is_some_and(|t| t.iter().any(|t| t["id"] == task))
                && yours,
        ),
        json!({"claim": claimed.map(|c| c.text), "trust": trust, "tasks_tab": tab,
               "mine": mine.map(|c| c.text), "task_sessions": sessions,
               "ui": "BLOCKED (How this work travelled as drawn)"}),
    );
    rig.stop(&session);
}

/// The session's root process as the supervisor and the system see it:
/// its pid, whether the supervisor counts it finished, and its state
/// letter and name from `ps` (`Z` is a zombie: dead, not yet reaped).
fn root_state(rig: &Rig, session: &str) -> Value {
    let root = rig
        .state
        .supervisor
        .live_roots()
        .into_iter()
        .find(|(id, _, _)| id.as_str() == session);
    let Some((_, pid, finished)) = root else {
        return Value::Null;
    };
    let ps = std::process::Command::new("ps")
        .args(["-o", "stat=,comm=", "-p", &pid.to_string()])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default();
    json!({"pid": pid, "finished": finished, "ps": ps})
}
