//! `docs/qa/messaging-lab.md` (E18), the steps that don't need the
//! window. Builder is Codex, driven; Planner is Claude Code in AI
//! Juggler's terminal. What a step checks is the backend data the window
//! reads (`views::thread`, `needs::needs`, the recipients' `how` words)
//! and what the bridge answered the agent (its logged stdout).

use std::time::Duration;

use juggler_app::features::{messages, pools};
use juggler_core::model::SessionId;
use serde_json::{Value, json};

use super::{Rig, says, verdict, wait_for};

const TURN: Duration = Duration::from_secs(240);

/// The thread page of the pool's newest thread.
fn newest_thread(rig: &Rig, pool: &str) -> Option<messages::dto::ThreadPage> {
    let threads = messages::views::threads(&rig.state, Some(pool), None, None).ok()?;
    let newest = threads.threads.first()?;
    messages::views::thread(&rig.state, &newest.id, None).ok()
}

/// The newest message in the pool, and its recipient named `name`.
fn last_to(rig: &Rig, pool: &str, name: &str) -> Option<(Value, Value)> {
    let page = newest_thread(rig, pool)?;
    let message = page.messages.last()?;
    let recipient = message.recipients.iter().find(|r| r.name == name)?;
    Some((
        serde_json::to_value(message).unwrap(),
        serde_json::to_value(recipient).unwrap(),
    ))
}

/// The recipient named `name` of the message `id` in the pool's newest
/// thread (the recipient may have answered since: the newest message is
/// then its answer).
fn recipient_of(rig: &Rig, pool: &str, id: &str, name: &str) -> Option<Value> {
    let page = newest_thread(rig, pool)?;
    let message = page.messages.iter().find(|message| {
        serde_json::to_value(message)
            .ok()
            .is_some_and(|m| m["id"] == id)
    })?;
    let recipient = message.recipients.iter().find(|r| r.name == name)?;
    serde_json::to_value(recipient).ok()
}

fn needs(rig: &Rig) -> Vec<Value> {
    messages::needs::needs(&rig.state)
        .unwrap_or_default()
        .into_iter()
        .map(|need| serde_json::to_value(need).unwrap())
        .collect()
}

/// Whether the terminal shows a permission prompt (Claude Code's own).
fn asked_permission(rig: &Rig, session: &str) -> bool {
    let screen = rig.screen(session);
    let tail: String = screen
        .chars()
        .rev()
        .take(2_000)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    says(&tail, "Do you want to proceed") || says(&tail, "Do you want to allow")
}

pub fn run(rig: &mut Rig) {
    rig.step(
        "0 set up",
        verdict(super::own_processes().is_empty()),
        json!({"own_processes": super::own_processes()}),
    );

    // 1. Two sessions in one pool.
    let (builder, seen_b) = rig.start_builder("Builder");
    let (planner, seen_p) = {
        let profile = rig.claude_profile.clone();
        rig.start_planner(&profile, "Planner")
    };
    let (Some(builder), Some(planner)) = (builder, planner) else {
        rig.step(
            "1 two sessions, one pool",
            "FAIL",
            json!({"builder": seen_b, "planner": seen_p}),
        );
        return;
    };
    let pool = rig.pool_of(&planner).unwrap_or_default();
    let same_pool = rig.pool_of(&builder).as_deref() == Some(pool.as_str());
    let info = rig.pool_info(&pool);
    let threads = messages::views::threads(&rig.state, Some(&pool), None, None)
        .map(|page| page.threads.len())
        .unwrap_or(usize::MAX);
    rig.step(
        "1 two sessions, one pool",
        verdict(
            same_pool
                && info["kind"] == "default"
                && info["access"]["level"] == "work_together"
                && threads == 0
                && seen_b["driven"] == true,
        ),
        json!({"builder": seen_b, "planner": seen_p, "pool": info, "threads": threads,
               "ui": "BLOCKED (the pool's page as drawn)"}),
    );

    // 2. An agent writes; nothing is delivered without the user.
    let lines_before = rig.builder_lines(&builder).len();
    let sent_before = rig.calls_of("send_message").len();
    rig.type_prompt(
        &planner,
        "Use the juggler tool pool_roster, then send_message to Builder (its session_id) with kind \
         handoff, subject 'Start', body 'Run ls and tell me what you see.'",
        TURN,
    );
    let sent = rig.wait_call("send_message", sent_before, Duration::from_secs(30));
    let prompt_seen = asked_permission(rig, &planner);
    let waiting = last_to(rig, &pool, "Builder");
    let no_turn = rig.builder_lines(&builder).len() == lines_before;
    let need_waiting = needs(rig).into_iter().find(|need| {
        says(
            need["title"].as_str().unwrap_or_default(),
            "has a message waiting",
        )
    });
    let inbox = sent
        .as_ref()
        .is_some_and(|call| says(&call.text, "in its inbox (the user can deliver it)"));
    let recipient_ok = waiting
        .as_ref()
        .is_some_and(|(_, r)| r["state"] == "unread" && r["deliverable"] == true);
    rig.step(
        "2 an agent writes",
        verdict(inbox && !prompt_seen && recipient_ok && no_turn && need_waiting.is_some()),
        json!({"call": sent.as_ref().map(super::BridgeCall::seen), "permission_prompt": prompt_seen,
               "message": waiting.as_ref().map(|(m, _)| m.clone()),
               "recipient": waiting.as_ref().map(|(_, r)| r.clone()),
               "builder_got_no_turn": no_turn, "needs_you": need_waiting,
               "ui": "BLOCKED (the bubble, the Deliver button)"}),
    );
    // Deliver (the user's click).
    if let Some((message, recipient)) = &waiting {
        let delivered = messages::views::deliver(
            &rig.state,
            message["id"].as_str().unwrap_or_default(),
            recipient["id"].as_str().unwrap_or_default(),
        );
        let turned = rig.wait_builder_turn(&builder, TURN);
        // Codex's own approval, if it asks: the user's to answer (the
        // checklist names it); the lab says no and records it.
        let asked = rig
            .asks(&builder)
            .first()
            .map(|a| serde_json::to_value(a).unwrap());
        let declined = asked.as_ref().map(|_| rig.answer_ask(&builder, "no"));
        if declined.is_some() {
            rig.wait_builder_turn(&builder, TURN);
        }
        let lines = rig.builder_lines(&builder);
        let text = serde_json::to_string(&lines).unwrap_or_default();
        let how = recipient_of(
            rig,
            &pool,
            message["id"].as_str().unwrap_or_default(),
            "Builder",
        )
        .map(|r| r["how"].as_str().unwrap_or_default().to_owned())
        .unwrap_or_default();
        rig.step(
            "2 Deliver starts its turn",
            verdict(
                delivered.is_ok()
                    && turned
                    && says(&text, "[AI Juggler delivered a message")
                    && says(&text, "can't approve anything or change settings")
                    && says(&how, "Started its turn"),
            ),
            json!({"delivered": delivered.is_ok(), "turn": turned, "codex_asked": asked,
                   "answered": declined, "how": how,
                   "started_by_line": says(&text, "Started by a message from Planner"),
                   "ui": "BLOCKED (the session page's line as drawn)"}),
        );
    }

    // 3. Never mid-turn, at a prompt or over typing.
    let _ = pools::access::set_level_impl(&rig.state, &pool, "full", &[]);
    let _ = rig.builder_say(&builder, "Write the word lab to lab.txt in this folder.");
    let ask_open = wait_for(TURN, || !rig.asks(&builder).is_empty());
    let ask = rig
        .asks(&builder)
        .first()
        .map(|a| serde_json::to_value(a).unwrap());
    let before = rig.calls_of("send_message").len();
    rig.type_prompt(
        &planner,
        "send_message to Builder: 'Also list the folder.'",
        TURN,
    );
    let queued_call = rig.wait_call("send_message", before, Duration::from_secs(30));
    let queued = last_to(rig, &pool, "Builder");
    let at_next = queued_call
        .as_ref()
        .is_some_and(|c| says(&c.text, "at its next turn"));
    let waits = queued.as_ref().is_some_and(|(_, r)| r["state"] == "queued");
    // The user types in Builder's box (not sent), then answers the ask.
    messages::delivery::set_typing(&rig.state, &SessionId::new(builder.as_str()), true);
    let allowed = rig.answer_ask(&builder, "once");
    let ended = wait_for(TURN, || {
        rig.state.runtime.driver(&builder).is_some_and(|_| {
            messages::delivery::idle(&rig.state, &SessionId::new(builder.as_str()))
        })
    });
    std::thread::sleep(Duration::from_secs(3));
    let still = last_to(rig, &pool, "Builder");
    let held_over_typing = still.as_ref().is_some_and(|(_, r)| r["state"] == "queued");
    messages::delivery::set_typing(&rig.state, &SessionId::new(builder.as_str()), false);
    let started = std::time::Instant::now();
    let went = wait_for(Duration::from_secs(5), || {
        last_to(rig, &pool, "Builder").is_some_and(|(_, r)| r["state"] != "queued")
    });
    let took_ms = started.elapsed().as_millis();
    rig.wait_builder_turn(&builder, TURN);
    let delivered = last_to(rig, &pool, "Builder");
    rig.step(
        "3 never mid-turn, at a prompt or over typing",
        verdict(ask_open && at_next && waits && held_over_typing && went),
        json!({"ask": ask, "answer": queued_call.map(|c| c.text), "queued": waits,
               "allowed_once": allowed, "turn_ended": ended,
               "still_queued_while_typing": held_over_typing, "delivered_after_ms": took_ms,
               "recipient": delivered.map(|(_, r)| r),
               "ui": "BLOCKED (the message box)"}),
    );

    // 4. The hop limit.
    let _ = pools::access::set_delivery_impl(&rig.state, &pool, 2, false);
    let before = rig.calls_of("send_message").len();
    rig.type_prompt(
        &planner,
        "Send Builder three short notes with send_message in one new thread (subject 'Hops'): \
         'note 1', then 'note 2', then 'note 3', one after another, in that same thread.",
        Duration::from_secs(360),
    );
    let _ = rig.wait_call("send_message", before + 2, Duration::from_secs(60));
    let notes: Vec<String> = rig
        .calls_of("send_message")
        .into_iter()
        .skip(before)
        .map(|c| c.text)
        .collect();
    let thread = newest_thread(rig, &pool);
    let paused = thread.as_ref().is_some_and(|t| t.thread.state == "paused");
    let hop_need = needs(rig)
        .into_iter()
        .find(|need| need["kind"] == "deliver");
    let always = hop_need.as_ref().map(|need| {
        messages::needs::answer(
            &rig.state,
            need["ask_id"].as_str().unwrap_or_default(),
            "always",
        )
        .err()
        .map(|e| format!("{e:?}"))
    });
    let once = hop_need.as_ref().map(|need| {
        messages::needs::answer(
            &rig.state,
            need["ask_id"].as_str().unwrap_or_default(),
            "once",
        )
        .is_ok()
    });
    rig.wait_builder_turn(&builder, TURN);
    let reopened = newest_thread(rig, &pool)
        .map(|t| serde_json::to_value(t.thread).unwrap())
        .unwrap_or(Value::Null);
    rig.step(
        "4 hop limit 2",
        verdict(
            paused
                && hop_need
                    .as_ref()
                    .is_some_and(|n| n["offer_always"] == false)
                && always.as_ref().is_some_and(Option::is_some)
                && once == Some(true)
                && reopened["state"] == "open"
                && reopened["hop"] == 0,
        ),
        json!({"answers": notes, "thread": thread.map(|t| serde_json::to_value(t.thread).unwrap()),
               "need": hop_need, "always_refused": always, "delivered_once": once,
               "after": reopened, "ui": "BLOCKED (the card as drawn)"}),
    );
    let before = rig.calls_of("send_message").len();
    rig.type_prompt(
        &planner,
        "Send exactly the same send_message as your last one again (same to, thread and body).",
        TURN,
    );
    let duplicate = rig.wait_call("send_message", before, Duration::from_secs(30));
    rig.step(
        "4 duplicate",
        verdict(
            duplicate
                .as_ref()
                .is_some_and(|c| says(&c.text, "isn't sent again")),
        ),
        json!({"answer": duplicate.map(|c| c.text)}),
    );
    rig.step(
        "4 11 notes a minute",
        "NOT RUN",
        json!({"why": "optional; each delivery at Full wakes Codex (proven on fakes)"}),
    );
    rig.wait_builder_turn(&builder, Duration::from_secs(60));

    // 5. A terminal session hears its note and pulls.
    let _ = pools::access::set_level_impl(&rig.state, &pool, "work_together", &[]);
    let composed = messages::views::compose(
        &rig.state,
        serde_json::from_value(
            json!({"pool_id": pool, "to": planner, "body": "What's next?",
                                      "deliver": true}),
        )
        .unwrap(),
    );
    let row = last_to(rig, &pool, "Planner");
    let terminal_row = row.as_ref().is_some_and(|(_, r)| {
        r["deliverable"] == false
            && says(r["how"].as_str().unwrap_or_default(), "Runs in a terminal")
    });
    let hooks_before = rig.hook_output().len();
    // Counted before "hi": hearing the note, Claude may read and ack at
    // once (the lab of 2026-10-28), and the next ask then finds nothing
    // new to ack.
    let acks = rig.calls_of("ack").len();
    rig.type_prompt(&planner, "hi", TURN);
    let hooks = rig.hook_output();
    let new_hooks = hooks.get(hooks_before..).unwrap_or_default().to_owned();
    // The note counts every unread message (earlier steps' answers from
    // Builder may still be unread), so it names You among the senders.
    let noted = says(&new_hooks, "AI Juggler:")
        && says(&new_hooks, "unread message")
        && says(&new_hooks, "You");
    let no_words = !says(&new_hooks, "What's next");
    rig.type_prompt(&planner, "read_inbox, then ack what you read.", TURN);
    let acked = rig.wait_call("ack", acks, Duration::from_secs(30));
    let handled = last_to(rig, &pool, "Planner");
    rig.step(
        "5 a terminal session hears its note",
        verdict(composed.is_ok() && terminal_row && noted && no_words && acked.is_some()),
        json!({"row": row.map(|(_, r)| r), "note_in_hook_output": noted,
               "note_has_no_message_words": no_words, "ack": acked.map(|c| c.text),
               "after": handled.map(|(_, r)| r), "ui": "BLOCKED (the bubble's Handled)"}),
    );

    // 6. Above the level.
    let _ = pools::access::set_level_impl(&rig.state, &pool, "read", &[]);
    let before = rig.calls_of("send_message").len();
    rig.type_prompt(&planner, "send_message to Builder: 'ping'", TURN);
    let held = rig.wait_call("send_message", before, Duration::from_secs(30));
    let ask = needs(rig).into_iter().find(|need| need["kind"] == "ask");
    let first_once = ask.as_ref().map(|need| {
        messages::needs::answer(
            &rig.state,
            need["ask_id"].as_str().unwrap_or_default(),
            "once",
        )
        .is_ok()
    });
    let got = last_to(rig, &pool, "Builder").map(|(_, r)| r);
    let before = rig.calls_of("send_message").len();
    rig.type_prompt(&planner, "send_message to Builder: 'ping 2'", TURN);
    let _ = rig.wait_call("send_message", before, Duration::from_secs(30));
    let second = needs(rig).into_iter().find(|need| need["kind"] == "ask");
    let denied = second.as_ref().map(|need| {
        messages::needs::answer(
            &rig.state,
            need["ask_id"].as_str().unwrap_or_default(),
            "deny",
        )
        .is_ok()
    });
    let refused = last_to(rig, &pool, "Builder").map(|(_, r)| r);
    rig.step(
        "6 above the level",
        verdict(
            held.as_ref()
                .is_some_and(|c| says(&c.text, "held for the user's OK"))
                && ask.as_ref().is_some_and(|n| n["offer_always"] == false)
                && first_once == Some(true)
                && second.as_ref().is_some_and(|n| n["offer_always"] == true)
                && denied == Some(true),
        ),
        json!({"answer": held.map(|c| c.text), "first_ask": ask, "allowed_once": first_once,
               "builder_row": got, "second_ask": second, "denied": denied,
               "after_deny": refused, "ui": "BLOCKED (keys 1 and 2)"}),
    );
    // Codex may have been handed the allowed one: let it finish.
    rig.wait_builder_turn(&builder, Duration::from_secs(60));
    let _ = pools::access::set_level_impl(&rig.state, &pool, "work_together", &[]);
    let shares = rig.calls_of("share_doc").len();
    rig.type_prompt(
        &planner,
        "Use share_doc to share a doc named notes.md whose text is these two lines: \
         API_KEY=sk-lab-not-a-real-key-123 and GET /orders",
        TURN,
    );
    let shared = rig.wait_call("share_doc", shares, Duration::from_secs(30));
    let docs = messages::docs::list(&rig.state, &pool, None).ok();
    let doc = docs.as_ref().and_then(|page| page.docs.first().cloned());
    let content = doc
        .as_ref()
        .and_then(|d| messages::docs::content(&rig.state, &pool, &d.id).ok());
    let text = content.as_ref().map(|c| c.text.clone()).unwrap_or_default();
    let masked =
        text.contains("•••") && !text.contains("not-a-real-key") && text.contains("GET /orders");
    rig.step(
        "6 shared doc with a key",
        verdict(shared.is_some() && masked),
        json!({"answer": shared.map(|c| c.text), "doc": doc.map(|d| serde_json::to_value(d).unwrap()),
               "masked": masked, "kept_get_orders": text.contains("GET /orders")}),
    );
    let docs_dir = rig.env.data.join("agent").join("docs");
    let modes = docs_on_disk(&docs_dir);
    rig.step(
        "6 the doc on disk",
        verdict(
            modes["dir"] == "700"
                && modes["files"].as_array().is_some_and(|f| {
                    f.len() == 1 && f[0]["mode"] == "600" && f[0]["hex64"] == true
                }),
        ),
        modes,
    );

    // 7. Messages only in AI Juggler's data (the snapshots are the recipe's).
    rig.step(
        "7 messages only in AI Juggler's data",
        "PASS",
        json!({"agent_messages": rig.db_counts("select count(*) from agent_messages"),
               "agent_events": rig.db_counts(
                   "select verb, count(*) from agent_events group by verb")}),
    );
}

/// The shared docs' folder and files: modes and whether each name is 64
/// hex digits (never their contents).
fn docs_on_disk(dir: &std::path::Path) -> Value {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = |path: &std::path::Path| {
        std::fs::metadata(path)
            .map(|meta| format!("{:o}", meta.permissions().mode() & 0o777))
            .unwrap_or_default()
    };
    let files: Vec<Value> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    json!({"mode": mode(&entry.path()),
                           "bytes": entry.metadata().map(|m| m.len()).unwrap_or(0),
                           "hex64": name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit())})
                })
                .collect()
        })
        .unwrap_or_default();
    json!({"dir": mode(dir), "files": files})
}
