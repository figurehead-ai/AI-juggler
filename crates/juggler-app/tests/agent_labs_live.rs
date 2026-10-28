//! AJ-E1803 / AJ-E1902: the live agent labs. The messaging checklist
//! (`docs/qa/messaging-lab.md`) and the tasks checklist
//! (`docs/qa/tasks-lab.md`), the steps that don't need the window, run
//! against AI Juggler's own backend with real Claude Code and Codex on
//! accounts whose owner agreed.
//!
//! Ignored, and a no-op unless every `AIJ_LAB_*` variable is set, so it
//! never runs in CI or on a plain `cargo test`. Run it with
//! `just agent-lab <messaging|tasks> …` (see the justfile), which takes the
//! before and after snapshots (metadata only) around it.
//!
//! What stands in for the window (as on 2026-10-26, now committed):
//! - an `AppState` over a fresh data folder, with the app's background
//!   work as `lib.rs` runs it without a window: the lifecycle forwarder,
//!   the 2 s tick (processes, the graph, the pools' cards) and the Needs
//!   you refresh; a panic in a tick part is caught and counted;
//! - Builder is Codex, driven (its app-server), after AI Juggler's trust
//!   sheet is answered Open restricted; Planner is Claude Code in AI
//!   Juggler's terminal, its prompts typed into the terminal as a user
//!   types them, and Claude's own folder-trust question answered yes for
//!   the lab folder only;
//! - the bridge is the real `juggler-agent`, behind a wrapper that logs its
//!   stdin and stdout lines (never its environment), so what each tool
//!   answered the agent is recorded;
//! - each step calls the same backend function as the window's command
//!   (`views::deliver`, `needs::answer`, `set_level_impl`, …); what the
//!   window draws (bubbles, cards, keys, pickers) is BLOCKED, and the data
//!   it reads is checked instead.
//!
//! At most two real tools run at once (Builder stops before the task's
//! own session starts). It signs in to nothing, reads no credential and
//! starts nothing outside the lab folder. The report holds real paths and
//! ids: keep it out of the repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "agent_labs/mod.rs"]
mod agent_labs;

#[test]
#[ignore = "the live agent labs; need AIJ_LAB_* and real, agreed accounts"]
fn agent_lab_live() {
    let Some(env) = agent_labs::Env::read() else {
        return;
    };
    let suite = env.suite.clone();
    let mut rig = agent_labs::Rig::new(env);
    match suite.as_str() {
        "messaging" => agent_labs::messaging::run(&mut rig),
        _ => agent_labs::tasks::run(&mut rig),
    }
    rig.finish();
}
