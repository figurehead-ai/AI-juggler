use super::*;
use crate::conversation_feed::{CallInput, StepResult};

fn step(tool: &str, input: CallInput) -> Step {
    Step {
        line: crate::tool_summary::summarize(tool, &serde_json::Value::Null),
        tool: tool.to_owned(),
        call_id: None,
        input,
        at_ms: None,
        result: None,
    }
}

fn bash(command: &str, description: Option<&str>) -> Step {
    step(
        "Bash",
        CallInput {
            command: Some(command.to_owned()),
            description: description.map(str::to_owned),
            ..CallInput::default()
        },
    )
}

fn file(tool: &str, path: &str) -> Step {
    step(
        tool,
        CallInput {
            file_path: Some(path.to_owned()),
            ..CallInput::default()
        },
    )
}

fn run(steps: Vec<Step>) -> StepRun {
    StepRun {
        steps,
        ..StepRun::default()
    }
}

#[test]
fn a_commands_description_goes_into_the_past_tense_verb_by_verb() {
    let one = |description: &str| title(&run(vec![bash("git commit", Some(description))]), false);
    assert_eq!(
        one("Record the Codex case and commit the on-screen app name"),
        "Recorded the Codex case and committed the on-screen app name"
    );
    // Not opening with a known verb: as written.
    assert_eq!(
        one("Restamp the critique and commit req. 2"),
        "Restamp the critique and commit req. 2"
    );
    assert_eq!(
        one("Run the tests then copy the report"),
        "Ran the tests then copied the report"
    );
    assert_eq!(one("Build and test the app"), "Built and tested the app");
    assert_eq!(
        one("Check the header and footer"),
        "Checked the header and footer"
    );
    assert_eq!(one("Stage, then push"), "Staged, then pushed");
    assert_eq!(one("List files"), "Listed files");
    assert_eq!(one("Verify the build"), "Verified the build");
    assert_eq!(one("Play it back"), "Played it back");
    assert_eq!(one("Git status"), "Git status");
}

#[test]
fn while_it_runs_the_verbs_are_going() {
    let going = |description: &str| title(&run(vec![bash("x", Some(description))]), true);
    assert_eq!(
        going("Record the Codex case and commit the on-screen app name"),
        "Recording the Codex case and committing the on-screen app name"
    );
    assert_eq!(going("Run the tests"), "Running the tests");
    assert_eq!(
        going("Make the build and use it"),
        "Making the build and using it"
    );
    assert_eq!(going("Restamp it"), "Restamp it");
    assert_eq!(
        title(&run(vec![file("Read", "/w/src/app.ts")]), true),
        "Reading app.ts"
    );
}

#[test]
fn a_command_with_no_description_shows_itself_or_says_it_ran() {
    assert_eq!(
        title(&run(vec![bash("pnpm test", None)]), false),
        "pnpm test"
    );
    let neither = step("Bash", CallInput::default());
    assert_eq!(title(&run(vec![neither]), false), "Ran a command");
    let long = "x".repeat(200);
    assert!(title(&run(vec![bash(&long, None)]), false).ends_with('…'));
}

#[test]
fn files_agents_and_searches_name_what_they_touched() {
    assert_eq!(
        title(&run(vec![file("Read", "/w/a/notes.md")]), false),
        "Read notes.md"
    );
    let mut created = file("Write", "/w/new.ts");
    created.result = Some(StepResult {
        failed: false,
        kind: Some("create".to_owned()),
    });
    assert_eq!(title(&run(vec![created]), false), "Created new.ts");
    let mut updated = file("Write", "/w/old.ts");
    updated.result = Some(StepResult {
        failed: false,
        kind: Some("update".to_owned()),
    });
    assert_eq!(title(&run(vec![updated]), false), "Updated old.ts");
    assert_eq!(
        title(&run(vec![file("Edit", "/w/b.rs")]), false),
        "Updated b.rs"
    );
    let agent = step(
        "Agent",
        CallInput {
            description: Some("Find the parser".to_owned()),
            subagent_type: Some("Explore".to_owned()),
            ..CallInput::default()
        },
    );
    assert_eq!(title(&run(vec![agent]), false), "Ran agent Find the parser");
    let web = step(
        "WebSearch",
        CallInput {
            query: Some("tauri channels".to_owned()),
            ..CallInput::default()
        },
    );
    assert_eq!(title(&run(vec![web]), false), "Searched web tauri channels");
}

#[test]
fn several_calls_read_as_kinds_in_order_with_distinct_counts() {
    let six: Vec<Step> = (0..6)
        .map(|n| bash(&format!("make step-{n}"), None))
        .collect();
    assert_eq!(title(&run(six), false), "Ran 6 commands");
    // The same command twice is one.
    let twice = vec![bash("make", None), bash("make", None)];
    assert_eq!(title(&run(twice), false), "Ran a command");
    assert_eq!(
        title(
            &run(vec![
                bash("ls docs", Some("List the captures")),
                file("Read", "/w/docs/accounts-screen-light.png"),
            ]),
            false
        ),
        "Ran a command, read accounts-screen-light.png"
    );
    assert_eq!(
        title(
            &run(vec![
                file("Read", "/w/a.ts"),
                file("Read", "/w/b.ts"),
                bash("pnpm test", None),
                file("Edit", "/w/a.ts"),
                step("Grep", CallInput::default()),
                file("Read", "/w/a.ts"),
            ]),
            false
        ),
        "Read 2 files, ran a command, edited a file, searched code"
    );
    // One file read and edited: named.
    assert_eq!(
        title(
            &run(vec![file("Read", "/w/a.ts"), file("Edit", "/w/a.ts")]),
            false
        ),
        "Read a.ts, edited a.ts"
    );
}

#[test]
fn a_run_of_thinking_alone() {
    let thoughts = StepRun {
        thoughts: vec!["hmm".to_owned()],
        ..StepRun::default()
    };
    assert_eq!(title(&thoughts, false), "Thought");
    assert_eq!(title(&thoughts, true), "Thinking");
}
