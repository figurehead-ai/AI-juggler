#![allow(clippy::unwrap_used)]

use super::*;
use juggler_handover::pack::RedactionCount;

fn spec(id: &str, name: &str, rules: &[&str]) -> ToolSpec {
    use juggler_adapter::HarnessAdapter as _;
    let mut spec = juggler_adapters::FakeHarnessAdapter::new("fake").spec();
    spec.id = id.to_owned();
    spec.display_name = name.to_owned();
    spec.rules_files = rules.iter().map(|rule| (*rule).to_owned()).collect();
    spec
}

fn turn(role: &'static str, text: &str) -> TurnText {
    TurnText {
        uuid: None,
        role,
        text: text.to_owned(),
    }
}

#[test]
fn the_rules_in_play_are_the_files_there_with_whose_they_are() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "be brief").unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "be brief").unwrap();
    let specs = [
        spec("claude-code", "Claude Code", &["CLAUDE.md"]),
        spec("codex", "Codex", &["AGENTS.md"]),
        spec(
            "other",
            "Other",
            &["AGENTS.md", "../outside.md", "MISSING.md"],
        ),
    ];
    let rules = rules_in_play(dir.path(), &specs);
    assert_eq!(
        rules,
        [
            RulesFile {
                name: "CLAUDE.md".into(),
                tools: vec!["Claude Code".into()]
            },
            RulesFile {
                name: "AGENTS.md".into(),
                tools: vec!["Codex".into(), "Other".into()]
            },
        ]
    );
    let section = rules_section(&rules).unwrap();
    assert!(
        section.contains("CLAUDE.md (read by Claude Code)"),
        "{section}"
    );
    assert!(
        section.contains("AGENTS.md (read by Codex, Other)"),
        "{section}"
    );
    assert_eq!(rules_section(&[]), None);
}

#[test]
fn earlier_turns_are_counted_and_named_by_what_was_asked() {
    let long = "x".repeat(150);
    let left_out = [
        turn("user", "\nAdd a health check endpoint\nwith details"),
        turn("assistant", "Added GET /health."),
        turn("user", &long),
    ];
    let note = earlier_turns(&left_out);
    assert!(
        note.starts_with("(3 earlier turns are not included."),
        "{note}"
    );
    assert!(note.contains("\n- Add a health check endpoint\n"), "{note}");
    assert!(
        !note.contains("with details") && !note.contains("Added GET"),
        "{note}"
    );
    assert!(
        note.contains(&format!("\n- {}…)", "x".repeat(100))),
        "{note}"
    );

    let many: Vec<TurnText> = (0..45).map(|n| turn("user", &format!("ask {n}"))).collect();
    let note = earlier_turns(&many);
    assert!(note.contains("- ask 39\n- …and 5 more)"), "{note}");
    assert!(!note.contains("ask 40"));
}

#[test]
fn the_handover_is_written_for_this_user_only_and_pointed_at() {
    let dir = tempfile::tempdir().unwrap();
    let state = HandoverState {
        format: FORMAT.to_owned(),
        version: VERSION,
        edge: "edge-42".into(),
        from: Side {
            tool: "claude-code".into(),
            account: "Work Max".into(),
        },
        to: Side {
            tool: "codex".into(),
            account: "Codex".into(),
        },
        folder: "/w/acme".into(),
        written_ms: 1,
        turns_included: 2,
        turns_left_out: 0,
        rules_files: Vec::new(),
        notes_quoted: Vec::new(),
        notes_left_out: Vec::new(),
        withheld: Some(vec![RedactionCount {
            kind: "sk-key".into(),
            count: 2,
        }]),
        kept: Vec::new(),
    };
    let file = write(dir.path(), "the brief", &state).unwrap();
    assert_eq!(file, dir.path().join("switches/edge-42/HANDOVER.md"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "the brief");
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("switches/edge-42/state.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(json["edge"], "edge-42");
    assert_eq!(json["from"]["account"], "Work Max");
    assert_eq!(
        json["withheld"],
        serde_json::json!([{"kind": "sk-key", "count": 2}]),
        "what was hidden: kinds and counts"
    );
    let back: HandoverState = serde_json::from_value(json).unwrap();
    assert_eq!(back, state, "it reads back as it was written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(&dir.path().join("switches/edge-42")), 0o700);
    }

    let header = "You are continuing a session that previously ran under another account or tool. \
                  The conversation so far is below, each turn quoted.";
    let prompt = pointer(header, &file);
    let mut lines = prompt.lines();
    assert_eq!(
        lines.next(),
        Some("You are continuing a session that previously ran under another account or tool."),
        "the opening the new conversation is found by"
    );
    assert!(
        prompt
            .lines()
            .any(|line| line == file.display().to_string()),
        "{prompt}"
    );
    assert!(!prompt.contains("below"), "{prompt}");
}

#[test]
fn switching_back_names_only_the_rules_the_returning_tool_doesnt_load() {
    let claude = spec("claude-code", "Claude Code", &["CLAUDE.md"]);
    let codex = spec("codex", "Codex", &["AGENTS.md"]);
    let rules = vec![
        RulesFile {
            name: "CLAUDE.md".into(),
            tools: vec!["Claude Code".into()],
        },
        RulesFile {
            name: "AGENTS.md".into(),
            tools: vec!["Codex".into()],
        },
    ];
    let note = rules_while_away(&rules, &codex).unwrap();
    assert!(
        note.starts_with(
            "While away the work also followed rules files Codex doesn't load itself:"
        )
    );
    assert!(note.contains("  CLAUDE.md (read by Claude Code)"));
    assert!(!note.contains("AGENTS.md"));
    assert_eq!(rules_while_away(&rules[..1], &claude), None);
}

#[test]
fn a_handover_from_before_0_3_reads_with_nothing_said_about_keys() {
    let old = serde_json::json!({
        "format": "ai-juggler-handover",
        "version": 1,
        "edge": "edge-1",
        "from": {"tool": "claude-code", "account": "Work Max"},
        "to": {"tool": "codex", "account": "Codex"},
        "folder": "/w/acme",
        "written_ms": 1,
        "turns_included": 3,
        "turns_left_out": 2,
        "rules_files": [],
    });
    let state: HandoverState = serde_json::from_value(old).unwrap();
    assert_eq!(
        state.withheld, None,
        "not recorded is not \"nothing found\""
    );
    assert!(state.kept.is_empty() && state.notes_left_out.is_empty());
}

fn rows(pairs: &[(&str, u32)]) -> Vec<crate::features::packs::WithheldInfo> {
    crate::features::packs::withheld_rows(
        &pairs
            .iter()
            .map(|(kind, count)| RedactionCount {
                kind: (*kind).to_owned(),
                count: *count,
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn the_line_says_what_was_hidden_kept_and_left_out_in_plain_words() {
    use view::told_line;
    let left_out = ["12 earlier turns".to_owned(), "1 note".to_owned()];
    assert_eq!(
        told_line(Some("Codex"), &rows(&[("sk-key", 2)]), &[], &left_out).as_deref(),
        Some("Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note")
    );
    assert_eq!(
        told_line(
            Some("Codex"),
            &rows(&[("sk-key", 1), ("bearer", 1)]),
            &[],
            &[]
        )
        .as_deref(),
        Some("Hidden from Codex: 1 API key, 1 bearer token"),
        "one is said as one"
    );
    assert_eq!(
        told_line(Some("Codex"), &[], &rows(&[("env-value", 3)]), &[]).as_deref(),
        Some("Sent to Codex as you chose: 3 secret values"),
        "kept keys are never called hidden"
    );
    assert_eq!(
        told_line(None, &rows(&[("private-key", 1)]), &[], &[]).as_deref(),
        Some("Hidden: 1 private key")
    );
    assert_eq!(
        told_line(Some("Codex"), &[], &[], &[]),
        None,
        "nothing to say"
    );
    assert_eq!(
        rows(&[("zcode-rollout", 2), ("some-new-kind", 1), ("api-key", 0)])
            .iter()
            .map(|row| row.what.as_str())
            .collect::<Vec<_>>(),
        ["access tokens", "other secret"],
        "no internal kind reaches the screen, and empty counts drop"
    );
}
