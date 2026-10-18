//! AJ-E210 evidence: the ordered-pair validation matrix (D-04) —
//! every researched pair is enumerated and exercised, NONE silently
//! skipped. For each pair the MECHANISM layer runs in CI: a fixture
//! pack built over the pair's E203 disposition shape survives an IR
//! golden round-trip, the distilled render keeps only the tail, and
//! the carry-list names every Drop with its reason. The VALIDATION
//! layer stays honest: only `claude-code→claude-code` claims a green
//! real-adapter CI row today; every other row records itself
//! unvalidated with its reason — and a recorded live dogfood cell
//! (pass OR fail) drives the advertised badge exactly per Req.4.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_handover::fidelity::base_dispositions;
use juggler_handover::pack::{
    ConversationMode, IrRole, IrTurn, PackArtifacts, PackFidelity, PackGit, PackInput, PackLineage,
    PackProject, PackQuota, PackSource, TurnTokens, generate_pack,
};
use juggler_handover::validation::{
    DogfoodCell, DogfoodVerdict, advertised_badge, pair_cell, validation_matrix, with_dogfood,
};

/// The per-pair mechanism fixture: a two-turn conversation plus the
/// pair's honest Drops (from the E203 disposition table — the same
/// table the badges read, so the fixture can never drift from the
/// advertised carry-list).
fn mechanism_fixture(from: &str, to: &str) -> PackInput {
    let carry = base_dispositions(from, to);
    let dropped: Vec<_> = carry
        .dispositions
        .iter()
        .filter(|(_, disposition)| {
            matches!(disposition, juggler_handover::fidelity::Disposition::Drop)
        })
        .map(|(kind, _)| juggler_handover::pack::DropEntry {
            artifact: format!("{kind:?}"),
            reason: "dropped by the pair's disposition table".to_owned(),
        })
        .collect();
    PackInput {
        pack_id: format!("pack-matrix-{from}-{to}"),
        created_at_ms: 1_792_094_400_000,
        work_id: "work-matrix".to_owned(),
        edge_id: None,
        source: PackSource {
            provider: from.to_owned(),
            account_label: "Fixture".to_owned(),
            profile_name: None,
            native_session_id: "native-matrix".to_owned(),
            harness_version: Some("test".to_owned()),
            epoch_id: "ep-matrix".to_owned(),
        },
        target_hint: Some(juggler_handover::pack::PackTarget {
            provider: to.to_owned(),
            account_label: "Target".to_owned(),
        }),
        project: PackProject {
            root: "/w/matrix".to_owned(),
            project_id: "proj-matrix".to_owned(),
        },
        git: PackGit {
            branch: "main".to_owned(),
            head_sha: None,
            dirty: false,
            diff_stat: None,
            worktree_path: None,
        },
        turns: vec![
            IrTurn {
                uuid: Some("m1".to_owned()),
                role: IrRole::User,
                text: "ship the matrix fixture".to_owned(),
                at_ms: Some(1_792_000_000_000),
                tokens: Some(TurnTokens {
                    input: 10,
                    output: 0,
                }),
            },
            IrTurn {
                uuid: Some("m2".to_owned()),
                role: IrRole::Assistant,
                text: "suite green".to_owned(),
                at_ms: Some(1_792_000_010_000),
                tokens: Some(TurnTokens {
                    input: 0,
                    output: 20,
                }),
            },
        ],
        compacted: false,
        artifacts: PackArtifacts {
            rules: vec!["AGENTS.md".to_owned()],
            mcp: vec![],
            skills: vec![],
            todos: vec![],
            drops: dropped,
        },
        fidelity: PackFidelity {
            level: "F1".to_owned(),
            score: 0.5,
            caps_applied: vec!["unvalidated-pair".to_owned()],
        },
        quota_at_creation: Some(PackQuota {
            account: "Fixture".to_owned(),
            used_pct: Some(40.0),
            confidence: "measured".to_owned(),
        }),
        lineage: PackLineage::default(),
        staged_files: Vec::new(),
    }
}

/// THE release gate: every researched pair is present, exercised,
/// and honestly labelled. A pair added to the research without a
/// fixture here fails this test; a fixture passing here never
/// validates a pair by itself (that needs the live cell).
#[test]
fn every_pair_is_enumerated_exercised_and_honestly_labelled() {
    let rows = validation_matrix();
    assert_eq!(
        rows.len(),
        11,
        "pass2/02 §10 pairs (3 same + 6 cross + 2 pack-only)"
    );

    let claude_row = rows
        .iter()
        .find(|row| row.from == "claude-code" && row.to == "claude-code")
        .unwrap();
    assert!(claude_row.ci_green, "the one real-adapter row today");
    assert!(!claude_row.validated(), "no live cell recorded yet");
    for row in &rows {
        // Mechanism fixtures run for EVERY row — nothing skipped.
        assert!(
            row.mechanism_green,
            "{}→{} mechanism exercised",
            row.from, row.to
        );
        // Unproven rows say so — in the table, not in a skip.
        if !row.ci_green {
            assert!(
                !row.notes.is_empty(),
                "{}→{} records WHY its CI row is not green",
                row.from,
                row.to
            );
        }
        // D-11 holds everywhere.
        assert!(!row.badge().to_lowercase().contains("lossless"));
    }
    // The zcode blocks are recorded (Req.3), never hidden.
    let z2z = pair_cell("zcode", "zcode").unwrap();
    assert!(z2z.notes.iter().any(|note| note.contains("BLOCKED")));
    let c2z = pair_cell("claude-code", "zcode").unwrap();
    assert_eq!(
        c2z.researched_ceiling,
        juggler_handover::fidelity::FidelityLabel::F0
    );
}

/// The mechanism layer for every pair: the fixture pack survives an
/// IR golden round-trip (serialize → deserialize → byte-identical
/// reserialize), the distilled render keeps only the tail turn, and
/// every Drop is a manifest row with its reason (Req.2).
#[test]
fn mechanism_fixtures_round_trip_every_pair() {
    for row in validation_matrix() {
        let input = mechanism_fixture(&row.from, &row.to);
        let pack = generate_pack(&input, ConversationMode::Full);

        // IR golden round-trip: the pack's versioned state parses
        // whole, survives a value-level round-trip unchanged, and
        // serializes deterministically (same bytes every time — the
        // E202 determinism contract, the IR IS the golden).
        let state = &pack.state_json;
        let parsed: serde_json::Value = serde_json::from_slice(state).unwrap();
        let bytes_a = serde_json::to_vec(&parsed).unwrap();
        let reparsed: serde_json::Value = serde_json::from_slice(&bytes_a).unwrap();
        let bytes_b = serde_json::to_vec(&reparsed).unwrap();
        assert_eq!(
            parsed, reparsed,
            "{}→{} IR value round-trips",
            row.from, row.to
        );
        assert_eq!(
            bytes_a, bytes_b,
            "{}→{} IR serialization is deterministic",
            row.from, row.to
        );

        // Distilled: only the tail turn renders (the tool-outcome
        // contract — the target reads outcomes, not the full story).
        let distilled = generate_pack(&input, ConversationMode::Distilled { last_n: 1 });
        let text = String::from_utf8(distilled.conversation_md.clone()).unwrap();
        assert!(
            text.contains("suite green"),
            "{}→{} tail kept",
            row.from,
            row.to
        );
        assert!(
            text.contains("distilled — 1 earlier turn"),
            "{}→{} earlier turns named, not silently gone",
            row.from,
            row.to
        );

        // Every Drop is a manifest row (drops never silently omitted).
        let manifest: Vec<serde_json::Value> = serde_json::from_slice(&pack.manifest_json).unwrap();
        let drop_rows: Vec<_> = manifest
            .iter()
            .filter(|entry| entry.get("disposition").and_then(|d| d.as_str()) == Some("drop"))
            .collect();
        assert_eq!(
            drop_rows.len(),
            input.artifacts.drops.len(),
            "{}→{} every Drop is a manifest row",
            row.from,
            row.to
        );
    }
}

/// Req.4 end to end at the matrix layer: a PASSING live cell
/// validates only real-CI rows; a FAILING cell lowers the advertised
/// ceiling one level until fixed; pending cells change nothing.
#[test]
fn dogfood_cells_drive_the_advertised_badge_per_the_rules() {
    let pass = DogfoodCell {
        at: "2026-10-16".to_owned(),
        project_shape: "tiny".to_owned(),
        source_sha256: "s1".to_owned(),
        target_sha256: "t1".to_owned(),
        verdict: DogfoodVerdict::Pass {
            continuity_note: "target answered the pre-switch question".to_owned(),
        },
    };
    // Pass on the real-CI row → validated, badge graduates.
    let claude = pair_cell("claude-code", "claude-code").unwrap();
    let validated = with_dogfood(claude.clone(), pass.clone());
    assert!(validated.validated());
    assert_eq!(validated.badge(), "F1 · Validated");
    assert_eq!(
        advertised_badge("claude-code", "claude-code", Some(&pass)),
        "F1 · Validated"
    );

    // The SAME pass on a fake-only row never validates it.
    let codex = pair_cell("codex", "codex").unwrap();
    let still_beta = with_dogfood(codex.clone(), pass);
    assert!(!still_beta.validated());
    assert_eq!(still_beta.badge(), "F1 · Beta");

    // A FAIL on the real-CI row lowers the ceiling one level.
    let mut fail = DogfoodCell {
        at: "2026-10-16".to_owned(),
        project_shape: "monorepo".to_owned(),
        source_sha256: "s2".to_owned(),
        target_sha256: "t2".to_owned(),
        verdict: DogfoodVerdict::Fail {
            reason: "S-F4: rules ring lost".to_owned(),
        },
    };
    let lowered = with_dogfood(claude, fail.clone());
    assert_eq!(lowered.badge(), "F0 · Beta", "F1 lowered one level");
    assert_eq!(
        advertised_badge("claude-code", "claude-code", Some(&fail)),
        "F0 · Beta"
    );

    // A later PASS replaces the fail (fixed and re-run) — the ceiling
    // recovers AND the pair validates.
    fail.verdict = DogfoodVerdict::Pass {
        continuity_note: "fixed; target answered".to_owned(),
    };
    fail.at = "2026-10-17".to_owned();
    assert_eq!(
        advertised_badge("claude-code", "claude-code", Some(&fail)),
        "F1 · Validated"
    );
}
