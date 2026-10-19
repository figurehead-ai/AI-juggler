//! AJ-E805: the switch pack for a 500-turn conversation is assembled
//! (redaction scan included) and written in under 5 s.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    missing_docs
)]

use juggler_handover::pack::{
    ConversationMode, DropEntry, IrRole, IrTurn, PackArtifacts, PackFidelity, PackGit, PackInput,
    PackLineage, PackProject, PackQuota, PackSource, TurnTokens, write_pack,
};
use juggler_handover::redaction::{RedactionApply, redacted_pack};
use std::time::{Duration, Instant};

fn base_input() -> PackInput {
    PackInput {
        pack_id: "01936657-6c2e-7a30-9a1f-3f2a11e9d000".to_owned(),
        created_at_ms: 1_792_094_400_000,
        work_id: "work-000123".to_owned(),
        edge_id: Some("edge-000045".to_owned()),
        source: PackSource {
            provider: "claude-code".to_owned(),
            account_label: "Work Max".to_owned(),
            profile_name: Some("deep-work".to_owned()),
            native_session_id: "b2632f0e-9c1d-4a17-9f6f-0f1d2e3b4c5d".to_owned(),
            harness_version: Some("2.1.277".to_owned()),
            epoch_id: "ep-000001".to_owned(),
        },
        target_hint: None,
        project: PackProject {
            root: "/Users/dev/ai-juggler".to_owned(),
            project_id: "proj-000007".to_owned(),
        },
        git: PackGit {
            branch: "main".to_owned(),
            head_sha: Some("9f8e7d6c5b4a39281706f5e4d3c2b1a0f9e8d7c6".to_owned()),
            dirty: true,
            diff_stat: Some("3 files changed, 42 insertions(+), 7 deletions(-)".to_owned()),
            worktree_path: None,
        },
        turns: vec![
            IrTurn {
                uuid: Some("11111111-1111-1111-1111-111111111111".to_owned()),
                role: IrRole::User,
                text: "Ship the pack v1 format.".to_owned(),
                at_ms: Some(1_792_000_000_000),
                tokens: Some(TurnTokens {
                    input: 120,
                    output: 0,
                }),
            },
            IrTurn {
                uuid: Some("22222222-2222-2222-2222-222222222222".to_owned()),
                role: IrRole::Assistant,
                text: "The pack is the preview — what travels, what stays.".to_owned(),
                at_ms: Some(1_792_000_060_000),
                tokens: Some(TurnTokens {
                    input: 0,
                    output: 340,
                }),
            },
            IrTurn {
                uuid: None,
                role: IrRole::User,
                text: "Keep it honest.".to_owned(),
                at_ms: Some(1_792_000_120_000),
                tokens: None,
            },
        ],
        compacted: false,
        artifacts: PackArtifacts {
            rules: vec!["AGENTS.md".to_owned()],
            mcp: vec!["filesystem".to_owned()],
            skills: Vec::new(),
            todos: Vec::new(),
            drops: vec![DropEntry {
                artifact: "session todos".to_owned(),
                reason: "in-session todos are unverified — treat as Drop".to_owned(),
            }],
        },
        fidelity: PackFidelity {
            level: "F1".to_owned(),
            score: 0.86,
            caps_applied: vec!["unvalidated-pair".to_owned()],
        },
        quota_at_creation: Some(PackQuota {
            account: "Work Max".to_owned(),
            used_pct: Some(55.0),
            confidence: "measured".to_owned(),
        }),
        lineage: PackLineage::default(),
        staged_files: vec![(
            "diffs/uncommitted.patch".to_owned(),
            b"--- a/src/lib.rs\n+++ b/src/lib.rs\n".to_vec(),
        )],
    }
}

/// A long, realistic conversation: 500 turns of a few hundred to a few
/// thousand characters, some carrying secret-shaped strings.
fn five_hundred_turns() -> PackInput {
    let mut input = base_input();
    let body =
        "Refactor the reconciler so the tick reads the snapshot once; then rerun the soak and \
                compare p95 against the budget. "
            .repeat(12);
    input.turns = (0..500)
        .map(|n| IrTurn {
            uuid: Some(format!("{n:08}-0000-4000-8000-000000000000")),
            role: if n % 2 == 0 {
                IrRole::User
            } else {
                IrRole::Assistant
            },
            text: if n % 50 == 7 {
                format!("{body} export ANTHROPIC_API_KEY=sk-ant-api03-{n:040}")
            } else {
                body.repeat(1 + n % 3)
            },
            at_ms: Some(1_792_000_000_000 + n as i64 * 60_000),
            tokens: Some(TurnTokens {
                input: 400,
                output: 900,
            }),
        })
        .collect();
    input
}

#[test]
fn a_500_turn_pack_is_assembled_and_written_in_under_5_s() {
    let input = five_hundred_turns();
    let chars: usize = input.turns.iter().map(|t| t.text.len()).sum();
    let dir = tempfile::tempdir().unwrap();
    for (name, mode) in [
        ("full", ConversationMode::Full),
        ("distilled", ConversationMode::Distilled { last_n: 12 }),
    ] {
        let started = Instant::now();
        let pack = redacted_pack(
            &input,
            mode,
            &RedactionApply {
                strip: true,
                keep: Default::default(),
            },
        );
        write_pack(&dir.path().join(name), &pack).unwrap();
        let took = started.elapsed();
        println!(
            "[perf] {name} pack, 500 turns ({} KB of text): {took:?} (budget 5 s)",
            chars / 1024
        );
        assert!(took < Duration::from_secs(5), "{name}: {took:?}");
        // The work really happened: every file written, no secret in any.
        let mut bytes = 0;
        for entry in std::fs::read_dir(dir.path().join(name)).unwrap().flatten() {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                assert!(
                    !text.contains("sk-ant-api03"),
                    "{name}: a secret was written"
                );
                bytes += text.len();
            }
        }
        assert!(bytes > 10_000, "{name}: only {bytes} bytes written");
    }
}
