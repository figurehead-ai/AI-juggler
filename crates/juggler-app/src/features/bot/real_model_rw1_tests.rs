//! 0.5.2 G57, G58, G59 on Apple's model: the RW1 re-walk's exact states
//! (Part R on c808d5f0a). The lab after the walk's turns: four of AI
//! Juggler's own Claude Code sessions (R2's edits and R7's revert
//! finished, R18's api question finished, R9's parser rewrite failed), the
//! outside session open and idle in docs (R12's line), and the two Codex
//! CLI threads AI Juggler didn't start (seeded earlier, closed); with
//! `web_open`, R20's session of AI Juggler's working in web (R31, R33).
//! Each question asked as the walk asked it, every draft and verdict
//! printed (`ask_recorded`), so a refusal is read, never guessed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use super::{
    Asked, Service, ask_recorded, ask_scoped, beta_on, insert_accounts, repo, service,
    session_scope, words_of,
};
use crate::features::bot::dto::{BotOutcome, BotScope};
use crate::features::bot::model::ask_for_check;

/// The re-walk's lab as Bot reads it after R2–R18's turns (see the
/// module); `web_open` adds R20's working session in web.
pub(super) fn rewalk_fixture(
    web_open: bool,
) -> (tempfile::TempDir, Arc<crate::state::AppState>, Service) {
    use crate::features::conversations::scan::State;
    use juggler_core::model::{SessionId, SessionStatus, Timestamp};
    let now = crate::commands::now_millis();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = crate::test_support::bare_state(root.join("data"));
    let (web, api, docs) = (repo(root, "web"), repo(root, "api"), repo(root, "docs"));
    {
        let storage = state.storage.lock().unwrap();
        insert_accounts(
            &storage,
            root,
            &[
                ("acc-work", "claude-code", "Work"),
                ("acc-personal", "claude-code", "Personal"),
                ("acc-chatgpt", "codex", "ChatGPT"),
            ],
        );
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings(
                "rec-work", "Work", "acc-work",
            ))
            .unwrap();
        let mut project = std::collections::HashMap::new();
        for (name, folder) in [("web", &web), ("api", &api), ("docs", &docs)] {
            project.insert(name, storage.projects().ensure(folder).unwrap().id);
        }
        // Minutes ago each turn ended, as the walk ran them (R2 first).
        let mut own = vec![
            (
                "s-edits",
                "web",
                "Split the cart total into its own module and test it",
                SessionStatus::Finished,
                40,
            ),
            (
                "s-revert",
                "api",
                "Change the date format in the API responses to ISO 8601",
                SessionStatus::Finished,
                34,
            ),
            (
                "s-fail",
                "web",
                "Rewrite the date parser to accept two-digit years",
                SessionStatus::Failed,
                30,
            ),
            (
                "s-question",
                "api",
                "Add retries to the sync client",
                SessionStatus::Finished,
                12,
            ),
        ];
        if web_open {
            own.push(("s-web", "web", "", SessionStatus::Running, 1));
        }
        for (id, name, title, status, ago) in own {
            let folder = match name {
                "web" => &web,
                "api" => &api,
                _ => &docs,
            };
            let mut record = crate::test_support::instance(id, "rec-work", status, folder.clone());
            record.project = project.get(name).cloned();
            record.started_at = Timestamp::from_millis(now - (ago + 3) * 60_000);
            record.last_activity = Some(Timestamp::from_millis(now - ago * 60_000));
            storage.sessions().insert(&record).unwrap();
            if !title.is_empty() {
                storage
                    .sessions()
                    .set_label(&SessionId::new(id), Some(title))
                    .unwrap();
            }
        }
        beta_on(&storage);
    }
    // The outside session (R12's line, the stand-in's notebook turn), open
    // and idle at its prompt; the two Codex threads, seeded before the
    // walk, closed.
    crate::test_support::outside_conversation(
        &state,
        ("acc-work", "claude-code"),
        "c-notebook",
        "Update the summary cell in the report notebook",
        &docs,
        now - 2 * 60_000,
    );
    crate::test_support::outside_conversation(
        &state,
        ("acc-chatgpt", "codex"),
        "x-patch",
        "Add a retry helper to the API client",
        &web,
        now - 50 * 60_000,
    );
    crate::test_support::outside_conversation(
        &state,
        ("acc-chatgpt", "codex"),
        "x-delete",
        "Remove the old handlers module",
        &api,
        now - 55 * 60_000,
    );
    crate::test_support::live_look(
        &state,
        &[("acc-work", "claude-code", "c-notebook", State::Idle)],
    );
    (dir, state, service())
}

/// R7's post, as the feed's Ask Bot sends it: its sentence before the
/// user's words, the post's session as the chip.
fn post_ask() -> (String, BotScope) {
    (
        "About this post: \u{201c}Claude Code CLI changed the date format in the API responses to ISO 8601 in api, in under a minute.\u{201d} did it finish cleanly?"
            .to_owned(),
        session_scope(
            "s-revert",
            "Change the date format in the API responses to ISO 8601",
        ),
    )
}

/// G57: the answer names no session by its project ("“api” is finished",
/// "“web” is the project"): a project's name in quotes only after "in".
fn named_by_project(words: &str) -> bool {
    [
        "\u{201c}api\u{201d}",
        "\u{201c}web\u{201d}",
        "\u{201c}docs\u{201d}",
    ]
    .iter()
    .any(|quoted| {
        words.match_indices(quoted).any(|(at, _)| {
            let before = words[..at].trim_end().to_lowercase();
            !(before.ends_with(" in") || before.ends_with("project") || before == "in")
        })
    })
}

/// One answer read as the walk records it, with what it must say.
fn expect(fails: &mut Vec<String>, what: &str, a: &Asked, ok: bool) {
    if !ok {
        fails.push(format!(
            "{what}: {:?} {:?} cites {:?}",
            a.outcome, a.words, a.cites
        ));
    }
}

/// 0.5.2 G58, G57 (Bot's part), G59: RW1 Part R's R12, R13, R14 and R30
/// on the re-walk's states, three times (the model samples). R12: "what
/// are my sessions" names the outside session and a Codex thread is among
/// its records (the bound keeps each kind); "what needs me" (twice) and
/// "what failed?" name the failed session, none refused, no session named
/// by its project; R13: 3+5 is Bot's own 8 after them. R14: the alias
/// answers; the memory question is answered in the topic with the earlier
/// answer and in a fresh one; Ask Bot from R7's post looks up the post's
/// session by its key (never the post's words); 3+5 after it is Bot's own.
#[test]
#[ignore = "needs the built helper and Apple Intelligence on; 0.5.2 G57, G58, G59"]
fn bot_answers_the_rewalks_part_r_on_the_real_model() {
    let (_dir, state, service) = rewalk_fixture(false);
    let world = crate::features::bot::world::gather(&state).unwrap();
    println!(
        "{}",
        serde_json::json!({"rows": world.sessions.iter().map(|r| (r.id.clone(), r.title.clone(), r.state.map(|s| s.code()), r.need, r.provider.clone(), r.project.clone())).collect::<Vec<_>>()})
    );
    let kept = ask_for_check(&state, &service, "call api the payments agent").unwrap();
    assert_eq!(kept.turn.outcome, BotOutcome::Remembered);
    let mut fails = Vec::new();
    for _ in 0..3 {
        // R12, R30.
        let mine = ask_recorded(&state, &service, "what are my sessions", false);
        expect(
            &mut fails,
            "what are my sessions",
            &mine,
            mine.outcome == BotOutcome::Answered
                && !mine.cites.is_empty()
                && mine.words.contains("summary cell")
                && !named_by_project(&mine.words),
        );
        for question in ["what needs me", "what needs me?", "what failed?"] {
            let a = ask_recorded(&state, &service, question, false);
            expect(
                &mut fails,
                question,
                &a,
                a.outcome == BotOutcome::Answered
                    && !a.cites.is_empty()
                    && a.words.contains("date parser")
                    && !named_by_project(&a.words),
            );
        }
        // R13: in R12's last topic.
        let sum = ask_recorded(&state, &service, "what is 3+5", true);
        expect(
            &mut fails,
            "3+5 after what failed?",
            &sum,
            sum.outcome == BotOutcome::General && sum.words.contains('8'),
        );
        // R14.
        let alias = ask_recorded(&state, &service, "what is the payments agent doing?", false);
        expect(
            &mut fails,
            "payments agent",
            &alias,
            alias.outcome == BotOutcome::Answered
                && !alias.cites.is_empty()
                && !named_by_project(&alias.words),
        );
        for fresh in [false, true] {
            let memory = ask_recorded(
                &state,
                &service,
                "what did I ask you to remember about api?",
                !fresh,
            );
            expect(
                &mut fails,
                if fresh {
                    "memory, fresh topic"
                } else {
                    "memory, after the alias's answer"
                },
                &memory,
                memory.outcome == BotOutcome::Answered && memory.words.contains("payments"),
            );
        }
        crate::features::bot::commands::bot_new_topic_impl(&state).unwrap();
        let (question, scope) = post_ask();
        let post = ask_scoped(&state, &service, &question, Some(scope));
        let post_words = words_of(&post).to_lowercase();
        if post.outcome != BotOutcome::Answered
            || !post.rows.iter().any(|r| r.id == "s-revert")
            || post_words.contains("nothing matches")
        {
            fails.push(format!(
                "Ask Bot from the post: {:?} {post_words:?} rows {:?}",
                post.outcome,
                post.rows
                    .iter()
                    .map(|r| r.title.clone())
                    .collect::<Vec<_>>()
            ));
        }
        let sum = ask_recorded(&state, &service, "what is 3+5", true);
        expect(
            &mut fails,
            "3+5 after the post's ask",
            &sum,
            sum.outcome == BotOutcome::General && sum.words.contains('8'),
        );
    }
    service.engines().release();
    assert!(fails.is_empty(), "not real answers:\n{}", fails.join("\n"));
}

/// 0.5.2 G58 (R33) and G57: with R20's session of AI Juggler's working in
/// web, "when did web finish?" and "what is web doing?" are answered from
/// the records (cited, or what isn't recorded), never refused, and name
/// the web session by its own name, never by its project. Three times.
#[test]
#[ignore = "needs the built helper and Apple Intelligence on; 0.5.2 G57, G58"]
fn bot_answers_about_an_open_web_session_on_the_real_model() {
    let (_dir, state, service) = rewalk_fixture(true);
    let mut fails = Vec::new();
    for _ in 0..3 {
        for question in ["when did web finish?", "what is web doing?"] {
            let a = ask_recorded(&state, &service, question, false);
            expect(
                &mut fails,
                question,
                &a,
                a.outcome == BotOutcome::Answered && !named_by_project(&a.words),
            );
        }
    }
    service.engines().release();
    assert!(fails.is_empty(), "not real answers:\n{}", fails.join("\n"));
}
