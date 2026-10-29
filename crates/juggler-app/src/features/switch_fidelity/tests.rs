//! The measurement on the conformance kit's synthetic transcripts: no
//! real tool, no real account.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::facts::{self, Fact, Kind, Score};
use super::*;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../juggler-adapters/tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn around(folder: &str) -> Surroundings {
    Surroundings {
        folder: PathBuf::from(folder),
        diff_stat: Some("1 file changed, 2 insertions(+)".to_owned()),
        changed: vec![" M calc.py".to_owned()],
        rules: vec!["CLAUDE.md".to_owned()],
        ..Surroundings::default()
    }
}

fn hop(source: Conversation, received: Received, probed: Vec<facts::Probed>) -> Hop {
    Hop {
        name: "A → B".to_owned(),
        mechanism: "transplant".to_owned(),
        source: Side {
            label: "Lab A (Tool)".to_owned(),
            history: source,
            around: around("/lab"),
        },
        target: Side {
            label: "Lab B (Tool)".to_owned(),
            history: received,
            around: around("/lab"),
        },
        facts: probed,
        openings: vec!["You are continuing a session".to_owned()],
    }
}

fn codename() -> Fact {
    Fact {
        key: "CODENAME".to_owned(),
        ask: "the project's codename".to_owned(),
        expect: vec!["flaky".to_owned()],
        planted: "A, turn 1 of 1 (early)".to_owned(),
        kind: Kind::Said,
    }
}

/// The handover a replay writes for `source`: each turn as the switch
/// renders it (its words, then what its tool calls did), quoted.
fn handover_of(source: &Conversation, leave_out: usize) -> String {
    use juggler_adapter::handover::TurnText;
    let turns: Vec<TurnText> = source
        .turns
        .iter()
        .map(|turn| TurnText {
            uuid: turn.uuid.clone(),
            role: if turn.user { "user" } else { "assistant" },
            text: match (turn.text.trim().is_empty(), turn.tools.is_empty()) {
                (_, true) => turn.text.clone(),
                (true, false) => format!("({})", turn.tools.join("; ")),
                (false, false) => format!("{}\n({})", turn.text, turn.tools.join("; ")),
            },
        })
        .collect();
    let mut out = String::from("You are continuing a session.\n\n");
    if leave_out > 0 {
        out.push_str(&crate::features::handover::earlier_turns(
            &turns[..leave_out],
        ));
        out.push('\n');
    }
    for turn in &turns[leave_out..] {
        out.push_str(&turn.quoted());
    }
    out
}

#[test]
fn a_conversation_is_read_through_the_tools_own_reader() {
    let claude = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    // The tool-result message carries no turn of its own.
    assert_eq!(claude.user_turns(), 1);
    assert_eq!(claude.assistant_messages(), 3);
    assert_eq!(claude.content.tool_calls, 1);
    assert_eq!(claude.content.tool_results, 1);
    assert_eq!(
        claude.turns[2].tools,
        vec!["ran \"npm test -- login\"".to_owned()]
    );

    let codex = Conversation::from_bytes(&fixture("codex/rollout-paginated.jsonl"));
    // Codex's own context (environment, permissions) is no turn.
    assert_eq!(codex.user_turns(), 2);
    assert_eq!(codex.assistant_messages(), 2);
    assert_eq!(codex.content.tool_calls, 1);
    assert_eq!(codex.content.tool_results, 1);
}

#[test]
fn a_subagents_own_lines_are_not_the_conversations() {
    let with = Conversation::from_bytes(&fixture("claude-code/session-subagent.jsonl"));
    let parsed = juggler_adapters::parse_any(&fixture("claude-code/session-subagent.jsonl"));
    let sidechain = parsed.messages.iter().filter(|m| m.sidechain).count();
    assert!(sidechain > 0, "the fixture has a subagent's lines");
    assert!(with.turns.len() < parsed.messages.len());
}

#[test]
fn a_transplant_that_carried_everything_passes() {
    let bytes = fixture("claude-code/session-basic.jsonl");
    let source = Conversation::from_bytes(&bytes);
    // The target's copy: the same lines under another session id.
    let copied = String::from_utf8(bytes)
        .unwrap()
        .replace("7f0c1a55-0001", "b-copy");
    let target = Conversation::from_bytes(copied.as_bytes());
    let answer = "CODENAME=the flaky login test";
    let report = measure(&hop(
        source,
        Received::Native(target),
        vec![facts::score(answer, &codename(), false)],
    ));
    assert_eq!(report.grade, "PASS", "{}", report.markdown());
    let users = &report.rows[0];
    assert_eq!((users.source, users.carried), (1, 1));
    let results = report
        .rows
        .iter()
        .find(|row| row.measure == "Tool results")
        .unwrap();
    assert_eq!(
        (results.source, results.carried, results.carry),
        (1, 1, Carry::Whole)
    );
    assert!(
        report.form.contains("4 whole, 0 shortened, 0 missing"),
        "{}",
        report.form
    );
    assert!(report.facts[0].in_source && report.facts[0].received);
}

#[test]
fn a_turn_missing_from_the_copy_is_lost_and_named() {
    let bytes = fixture("claude-code/session-basic.jsonl");
    let source = Conversation::from_bytes(&bytes);
    let mut target = source.clone();
    target.turns.remove(3);
    let report = measure(&hop(source, Received::Native(target), Vec::new()));
    assert_eq!(report.grade, "PARTIAL");
    let assistant = &report.rows[1];
    assert_eq!(
        (assistant.source, assistant.carried, assistant.carry),
        (3, 2, Carry::Lost)
    );
    assert!(
        report.verdict.contains("lost: assistant messages"),
        "{}",
        report.verdict
    );
}

#[test]
fn a_replay_counts_whole_turns_and_summarised_results() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let text = handover_of(&source, 0);
    let report = measure(&Hop {
        mechanism: "replay".to_owned(),
        ..hop(
            source,
            Received::Handover {
                text,
                included: 4,
                left_out: 0,
            },
            Vec::new(),
        )
    });
    let row = |name: &str| {
        report
            .rows
            .iter()
            .find(|row| row.measure == name)
            .unwrap()
            .clone()
    };
    assert_eq!(row("User turns").carried, 1);
    assert_eq!(row("Assistant messages").carried, 3);
    let calls = row("Tool calls");
    assert_eq!(
        (calls.source, calls.carried, calls.carry),
        (1, 1, Carry::ByDesign)
    );
    let results = row("Tool results");
    assert_eq!(
        (results.source, results.carried, results.carry),
        (1, 0, Carry::ByDesign)
    );
    assert_eq!(report.grade, "PASS", "{}", report.markdown());
    assert!(
        report.form.starts_with("a handover file"),
        "{}",
        report.form
    );
}

#[test]
fn turns_a_handover_left_out_are_shortened_not_missing() {
    let mut source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    // A request longer than the topic a left-out turn is named by.
    source.turns[0].text = format!(
        "Fix the flaky login test. {}",
        "Then tidy it up. ".repeat(10)
    );
    let text = handover_of(&source, 1);
    let report = measure(&hop(
        source,
        Received::Handover {
            text,
            included: 3,
            left_out: 1,
        },
        Vec::new(),
    ));
    let users = &report.rows[0];
    assert_eq!((users.carried, users.carry), (0, Carry::ByDesign));
    assert!(
        users.notes.starts_with("0 whole, 1 shortened"),
        "{}",
        users.notes
    );
}

#[test]
fn switching_back_measures_only_what_was_new_away() {
    use juggler_handover::pack::{IrRole, IrTurn};
    use juggler_handover::switch_back::{DeltaInput, delta_turn_text};
    let original = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let away_rollout = Conversation::from_bytes(&fixture("codex/rollout-paginated.jsonl"));
    let away = original.clone().then(away_rollout.clone());
    let ir: Vec<IrTurn> = away_rollout
        .turns
        .iter()
        .map(|turn| IrTurn {
            uuid: turn.uuid.clone(),
            role: if turn.user {
                IrRole::User
            } else {
                IrRole::Assistant
            },
            text: match turn.tools.is_empty() {
                true => turn.text.clone(),
                false => format!("{}\n({})", turn.text, turn.tools.join("; ")),
            },
            at_ms: None,
            tokens: None,
        })
        .collect();
    let update = delta_turn_text(&DeltaInput {
        away_account: "Lab Codex".to_owned(),
        original_account: "Lab A".to_owned(),
        away_only_uuids: ir.iter().filter_map(|turn| turn.uuid.clone()).collect(),
        away_turns: ir,
        conversation: juggler_handover::reconcile::ConversationReconciliation {
            base_uuids: 0,
            away_only_uuids: Vec::new(),
            original_only_uuids: 0,
        },
        diff_stat: None,
        todo_note: None,
        earlier_hops: Vec::new(),
        unread: Vec::new(),
        rules_note: None,
    });
    let report = measure(&hop(
        away,
        Received::Return { original, update },
        Vec::new(),
    ));
    // Only the away rollout's 4 turns are new; the original's are its own.
    let users = &report.rows[0];
    assert_eq!(
        (users.source, users.carried),
        (2, 2),
        "{}",
        report.markdown()
    );
    assert_eq!(report.rows[1].carried, 2);
    assert!(
        report
            .form
            .contains("resumed as it was (4 turns of its own)")
    );
}

#[test]
fn a_replays_opening_is_not_work_done_away() {
    let original = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut away = Conversation::default();
    away.turns.push(Turn {
        uuid: Some("x1".to_owned()),
        user: true,
        text: "You are continuing a session that ran elsewhere. Its handover is in a file."
            .to_owned(),
        tools: Vec::new(),
    });
    let report = measure(&hop(
        away,
        Received::Return {
            original,
            update: "While you were away — 0 new turns.".to_owned(),
        },
        Vec::new(),
    ));
    assert_eq!(report.rows[0].source, 0);
}

#[test]
fn task_items_carry_with_their_state() {
    let line = |content: &str| {
        format!(
            r#"{{"type":"assistant","uuid":"t1","sessionId":"s","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"u1","name":"TodoWrite","input":{{"todos":[{content}]}}}}]}}}}"#
        )
    };
    let source = Conversation::from_bytes(
        line(r#"{"content":"add","status":"completed"},{"content":"sub","status":"pending"}"#)
            .as_bytes(),
    );
    assert_eq!(
        source
            .tasks
            .as_ref()
            .unwrap()
            .iter()
            .map(Task::rendered)
            .collect::<Vec<_>>(),
        vec!["[x] add", "[ ] sub"]
    );
    let report = measure(&hop(
        source.clone(),
        Received::Handover {
            text: "Task list:\n[x] add\n[x] sub\n".to_owned(),
            included: 1,
            left_out: 0,
        },
        Vec::new(),
    ));
    let tasks = report
        .rows
        .iter()
        .find(|row| row.measure.starts_with("Task list"))
        .unwrap();
    // `sub` arrived with the wrong state: not carried.
    assert_eq!(
        (tasks.source, tasks.carried, tasks.carry),
        (2, 1, Carry::Lost)
    );
}

#[test]
fn a_moved_folder_or_changed_diff_fails_the_hop() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut moved = hop(source.clone(), Received::Native(source), Vec::new());
    moved.target.around.folder = PathBuf::from("/elsewhere");
    let report = measure(&moved);
    assert_eq!(report.grade, "FAIL");
}

#[test]
fn rules_a_replay_names_count_as_carried() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut replay = hop(
        source,
        Received::Handover {
            text: "Rules files in this folder:\n  CLAUDE.md (read by Tool)\n".to_owned(),
            included: 0,
            left_out: 0,
        },
        Vec::new(),
    );
    replay.target.around.rules = vec!["AGENTS.md".to_owned()];
    let report = measure(&replay);
    let rules = report
        .rows
        .iter()
        .find(|row| row.measure == "Rules files in force")
        .unwrap();
    assert_eq!((rules.source, rules.carried), (1, 1));
}

#[test]
fn subagents_left_behind_by_a_replay_are_by_design_but_lost_in_a_transplant() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut transplant = hop(source.clone(), Received::Native(source.clone()), Vec::new());
    transplant.source.around.subagents = vec!["agent-1".to_owned()];
    let row = |report: &HopReport| {
        report
            .rows
            .iter()
            .find(|row| row.measure == "Subagents")
            .unwrap()
            .carry
    };
    assert_eq!(row(&measure(&transplant)), Carry::Lost);
    transplant.target.around.subagents = vec!["agent-1".to_owned()];
    assert_eq!(row(&measure(&transplant)), Carry::Whole);
    let mut replay = hop(
        source,
        Received::Handover {
            text: String::new(),
            included: 0,
            left_out: 0,
        },
        Vec::new(),
    );
    replay.source.around.subagents = vec!["agent-1".to_owned()];
    assert_eq!(row(&measure(&replay)), Carry::ByDesign);
}

#[test]
fn facts_are_scored_answered_wrong_or_unknown() {
    let fact = |key: &str, expect: &[&str]| Fact {
        key: key.to_owned(),
        ask: "?".to_owned(),
        expect: expect.iter().map(|word| (*word).to_owned()).collect(),
        planted: "early".to_owned(),
        kind: Kind::ReplyOnly,
    };
    let answer = "PORT=5123\n- **JOB**: nightly-owl\nTICKET=UNKNOWN\nTASKS: sub, mul";
    let of = |key: &str, expect: &[&str]| facts::score(answer, &fact(key, expect), false).score;
    assert_eq!(of("PORT", &["5123"]), Score::Answered);
    assert_eq!(of("JOB", &["owl"]), Score::Answered);
    assert_eq!(of("PORT", &["5999"]), Score::Wrong);
    assert_eq!(of("TICKET", &["LAB-7341"]), Score::Unknown);
    assert_eq!(of("BUCKET", &["ruby"]), Score::Unknown);
    assert_eq!(of("TASKS", &["sub", "mul"]), Score::Answered);
    // A key that only starts another's name is not that one.
    assert_eq!(
        facts::score("JOBS=x", &fact("JOB", &["x"]), false).given,
        ""
    );
    let question = facts::question(&[fact("PORT", &["1"])]);
    assert!(question.contains("KEY=value") && question.contains("\nPORT: ?"));
}

#[test]
fn a_value_said_only_in_a_reply_is_found_by_its_marker() {
    let conversation = Conversation {
        turns: vec![
            Turn {
                uuid: None,
                user: true,
                text: "Pick a port; reply as PORT=<n>.".to_owned(),
                tools: Vec::new(),
            },
            Turn {
                uuid: None,
                user: false,
                text: "Chosen: **PORT=5417**.".to_owned(),
                tools: Vec::new(),
            },
        ],
        ..Conversation::default()
    };
    assert_eq!(
        facts::said_as(&conversation, "PORT").as_deref(),
        Some("5417")
    );
    assert_eq!(facts::said_as(&conversation, "JOB"), None);
}

#[test]
fn the_report_renders_a_table_and_a_verdict() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let report = measure(&hop(
        source.clone(),
        Received::Native(source),
        vec![facts::score("CODENAME=UNKNOWN", &codename(), false)],
    ));
    let text = report.markdown();
    assert!(text.starts_with("### A → B: Lab A (Tool) → Lab B (Tool) (transplant)"));
    assert!(text.contains("| Measure | Source | Carried | % | Notes |"));
    assert!(text.contains("| User turns | 1 | 1 | 100% |"), "{text}");
    assert!(text.contains(
        "| CODENAME | A, turn 1 of 1 (early) | said by the person | no | yes | unknown |"
    ));
    assert!(
        text.contains("**Verdict.** PARTIAL: nothing lost; 0 of 1 facts answered."),
        "{text}"
    );
}

#[test]
fn rules_a_switch_back_update_names_count_as_carried() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut back = hop(
        source.clone(),
        Received::Return {
            original: Conversation::default(),
            update: "While you were away …".to_owned(),
        },
        Vec::new(),
    );
    back.target.around.rules = vec!["AGENTS.md".to_owned()];
    let carried = |report: &HopReport| {
        report
            .rows
            .iter()
            .find(|row| row.measure == "Rules files in force")
            .unwrap()
            .carried
    };
    assert_eq!(carried(&measure(&back)), 0);
    back.target.history = Received::Return {
        original: Conversation::default(),
        update: "…rules files Codex doesn't load itself:\n  CLAUDE.md (read by Claude Code)"
            .to_owned(),
    };
    assert_eq!(carried(&measure(&back)), 1);
}

#[test]
fn switching_back_counts_new_work_with_the_same_words_as_old() {
    let original = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let mut away = original.clone();
    // A second "ran the tests" away, its own message: new work.
    let mut again = original.turns[2].clone();
    again.uuid = Some("away-1".to_owned());
    away.turns.push(again);
    let report = measure(&hop(
        away,
        Received::Return {
            original,
            update: "- [assistant] (ran \"npm test -- login\")".to_owned(),
        },
        Vec::new(),
    ));
    let calls = report
        .rows
        .iter()
        .find(|row| row.measure == "Tool calls")
        .unwrap();
    assert_eq!((calls.source, calls.carried), (1, 1));
}

fn note_fact() -> Fact {
    Fact {
        key: "NOTE_CODENAME".to_owned(),
        ask: "the codename this project's notes hold".to_owned(),
        expect: vec!["marsh-wren".to_owned()],
        planted: "A's notes for this project".to_owned(),
        kind: Kind::Note,
    }
}

#[test]
fn project_notes_copied_or_quoted_are_whole_and_a_planted_note_is_found() {
    let source = Conversation::from_bytes(&fixture("claude-code/session-basic.jsonl"));
    let with_notes = |text: &str| Surroundings {
        notes: vec!["lab-codename.md".to_owned()],
        notes_text: text.to_owned(),
        ..around("/lab")
    };
    // Copied to an account whose tool keeps notes.
    let mut copied = hop(
        source.clone(),
        Received::Native(source.clone()),
        vec![facts::score(
            "NOTE_CODENAME=MARSH-WREN",
            &note_fact(),
            false,
        )],
    );
    copied.source.around = with_notes("The codename is MARSH-WREN.");
    copied.target.around = with_notes("The codename is MARSH-WREN.");
    let report = measure(&copied);
    let row = report
        .rows
        .iter()
        .find(|row| row.measure == "Project notes")
        .unwrap();
    assert_eq!((row.source, row.carried, row.carry), (1, 1, Carry::Whole));
    assert!(
        row.notes.contains("on the target: lab-codename.md"),
        "{}",
        row.notes
    );
    assert!(report.facts[0].in_source && report.facts[0].received);
    assert_eq!(report.grade, "PASS", "{}", report.markdown());
    assert!(report.markdown().contains("| NOTE_CODENAME | A's notes for this project | a project note (the tool's own memory) | no | yes | answered |"));

    // Quoted in a handover, for a tool that keeps none.
    let mut quoted = hop(
        source.clone(),
        Received::Handover {
            text: "This project's notes from Lab A:\n\nlab-codename.md:\n> The codename is MARSH-WREN.\n".to_owned(),
            included: 1,
            left_out: 0,
        },
        Vec::new(),
    );
    quoted.source.around = with_notes("The codename is MARSH-WREN.");
    let row = measure(&quoted)
        .rows
        .into_iter()
        .find(|row| row.measure == "Project notes")
        .unwrap();
    assert_eq!((row.source, row.carried, row.carry), (1, 1, Carry::Whole));
    assert!(
        row.notes.contains("quoted in the handover"),
        "{}",
        row.notes
    );

    // Not ticked: by design, never a loss.
    let mut left = hop(source.clone(), Received::Native(source), Vec::new());
    left.source.around = with_notes("The codename is MARSH-WREN.");
    let row = measure(&left)
        .rows
        .into_iter()
        .find(|row| row.measure == "Project notes")
        .unwrap();
    assert_eq!(
        (row.source, row.carried, row.carry),
        (1, 0, Carry::ByDesign)
    );
    assert!(
        row.notes.contains("they travel only with your tick"),
        "{}",
        row.notes
    );
}
