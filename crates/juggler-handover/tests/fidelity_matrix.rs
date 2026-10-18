//! AJ-E203 evidence: the per-pair fidelity matrix — table-driven
//! cell coverage (pass2/02 §3.2–§3.3), each cap lowering the label
//! independently, direction-awareness (ordered pairs differ), the
//! two worked examples from the research (proper Claude A→B ≈F3;
//! v0 single-file capped F1), Translate-without-mapping scored as
//! Drop with named keys, todos defaulting Drop, and the committed
//! full-matrix snapshot.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_handover::fidelity::{
    ArtifactKind, Disposition, FidelityInput, FidelityLabel, PairFidelity, base_dispositions,
    full_matrix, score_pair,
};
use std::collections::BTreeMap;
use std::path::Path;

fn all_copy() -> BTreeMap<ArtifactKind, Disposition> {
    let mut map = BTreeMap::new();
    for kind in [
        ArtifactKind::Chat,
        ArtifactKind::Rules,
        ArtifactKind::GitDiff,
        ArtifactKind::Todos,
        ArtifactKind::Mcp,
        ArtifactKind::Skills,
        ArtifactKind::EnvModel,
        ArtifactKind::Subagents,
    ] {
        map.insert(kind, Disposition::Copy);
    }
    map
}

fn input(dispositions: BTreeMap<ArtifactKind, Disposition>) -> FidelityInput {
    FidelityInput {
        from_provider: "claude-code".to_owned(),
        to_provider: "claude-code".to_owned(),
        dispositions,
        compact_boundary: false,
        flags: juggler_handover::fidelity::PairFlags {
            validated: true,
            same_account: false,
            native_resume: false,
        },
        unmapped_keys: Vec::new(),
    }
}

/// Weights sum to 1.0 and match §3.4 exactly.
#[test]
fn weights_match_the_spec_and_sum_to_one() {
    let sum: f64 = juggler_handover::fidelity::WEIGHTS
        .iter()
        .map(|(_, w)| w)
        .sum();
    assert!((sum - 1.0).abs() < 1e-9, "weights sum to {sum}");
    let chat = juggler_handover::fidelity::WEIGHTS
        .iter()
        .find(|(k, _)| *k == ArtifactKind::Chat)
        .unwrap()
        .1;
    assert!((chat - 0.35).abs() < 1e-9);
}

/// All-Copy, validated, same-account native resume ⇒ F4 with the
/// exact user label; anything else caps at F3.
#[test]
fn f4_requires_same_account_native_resume() {
    let mut i = input(all_copy());
    i.flags.same_account = true;
    i.flags.native_resume = true;
    let scored = score_pair(&i);
    assert_eq!(scored.label, FidelityLabel::F4);
    assert_eq!(scored.label.user_label(), "Continues exactly");
    assert!(scored.caps.is_empty());

    let mut other = input(all_copy());
    other.flags.same_account = false; // different account transplant
    let capped = score_pair(&other);
    assert_eq!(capped.label, FidelityLabel::F3);
    assert_eq!(
        capped.caps.first().unwrap().cap,
        "f4-requires-same-account-native-resume"
    );
}

/// Each cap lowers the label independently (Req.3: assert each cap
/// alone drops the ceiling).
#[test]
fn caps_lower_independently() {
    // Compact boundary alone: F3 → F2 (all-copy validated).
    let mut compact = input(all_copy());
    compact.compact_boundary = true;
    assert_eq!(score_pair(&compact).label, FidelityLabel::F2);

    // Drop in chat alone (todos already Drop-able without capping):
    // the critical-artifact cap hits F1.
    let mut chat_drop = input(all_copy());
    chat_drop
        .dispositions
        .insert(ArtifactKind::Chat, Disposition::Drop);
    assert_eq!(score_pair(&chat_drop).label, FidelityLabel::F1);

    // Rules drop alone, git drop alone.
    let mut rules_drop = input(all_copy());
    rules_drop
        .dispositions
        .insert(ArtifactKind::Rules, Disposition::Drop);
    assert_eq!(score_pair(&rules_drop).label, FidelityLabel::F1);
    let mut git_drop = input(all_copy());
    git_drop
        .dispositions
        .insert(ArtifactKind::GitDiff, Disposition::Drop);
    assert_eq!(score_pair(&git_drop).label, FidelityLabel::F1);

    // Unvalidated alone caps a would-be F3 at F1.
    let mut unvalidated = input(all_copy());
    unvalidated.flags.validated = false;
    let scored = score_pair(&unvalidated);
    assert_eq!(scored.label, FidelityLabel::F1);
    assert!(
        scored.caps.iter().any(|cap| cap.cap == "unvalidated-pair"),
        "caps: {:?}",
        scored.caps
    );

    // zcode target alone caps everything at F0.
    let mut ztarget = input(all_copy());
    ztarget.to_provider = "zcode".to_owned();
    let scored = score_pair(&ztarget);
    assert_eq!(scored.label, FidelityLabel::F0);
    assert!(
        scored.caps.iter().any(|cap| cap.cap == "zcode-target"),
        "caps: {:?}",
        scored.caps
    );
}

/// Worked example 1 (pass2/02 §3.4): proper Claude A→B (siblings
/// copied, todos Drop) ⇒ mean ≈.90 band F3; with the pair
/// unvalidated it caps at F1.
#[test]
fn worked_example_proper_claude_a_to_b() {
    let mut i = input(
        base_dispositions("claude-code", "claude-code")
            .dispositions
            .clone(),
    );
    i.flags.validated = true;
    let scored = score_pair(&i);
    // .35 + .15 + .15 + 0 + .10 + .05 + .05 + .05 = .90
    assert!((scored.score - 0.90).abs() < 1e-9, "score {}", scored.score);
    assert_eq!(scored.label, FidelityLabel::F3, "validated F3");
    assert_eq!(scored.label.user_label(), "Same conversation, new account");

    // Unvalidated (today's truth): capped F1 — the cap, not the
    // mean, is the honesty mechanism.
    let mut v0 = i.clone();
    v0.flags.validated = false;
    let v0_scored = score_pair(&v0);
    assert!((v0_scored.score - 0.90).abs() < 1e-9);
    assert_eq!(
        v0_scored.label,
        FidelityLabel::F1,
        "v0 ≈.86-.90 but capped F1"
    );
}

/// Worked example 2: v0 single-file transplant (siblings Drop)
/// scores lower AND caps F1.
#[test]
fn worked_example_v0_single_file() {
    let mut i = input(
        base_dispositions("claude-code", "claude-code")
            .dispositions
            .clone(),
    );
    i.dispositions
        .insert(ArtifactKind::Subagents, Disposition::Drop);
    i.flags.validated = true;
    let scored = score_pair(&i);
    // .90 − .05 = .85
    assert!((scored.score - 0.85).abs() < 1e-9);
    assert_eq!(scored.label, FidelityLabel::F3);
    // And unvalidated ⇒ F1 regardless.
    let mut v0 = i;
    v0.flags.validated = false;
    assert_eq!(score_pair(&v0).label, FidelityLabel::F1);
}

/// Ordered pairs differ (Req.2): Claude→ZCode (F1 source ceiling)
/// vs ZCode→Claude — and any zcode TARGET is F0.
#[test]
fn direction_matters() {
    let to_z = score_pair(&base_dispositions("claude-code", "zcode"));
    let from_z = score_pair(&base_dispositions("zcode", "claude-code"));
    assert_eq!(to_z.label, FidelityLabel::F0, "zcode target ⇒ F0 today");
    assert!(to_z.score > 0.0, "source score nonzero; the cap speaks");
    // The pair strings differ — no direction-blind aggregation.
    assert_ne!(to_z.pair, from_z.pair);
    // zcode→claude-code: mean .67 (chat Translate carries) — today
    // the label never exceeds F1 (the unvalidated ceiling; at mean
    // .67 the band alone already lands F1, so no cap row records).
    assert_eq!(from_z.label, FidelityLabel::F1);
}

/// Translate without a written mapping scores as Drop and NAMES the
/// unmapped keys (Req.5 — never silently omitted).
#[test]
fn untranslated_scores_as_drop_and_names_keys() {
    let mut i = input(all_copy());
    i.dispositions.insert(
        ArtifactKind::Skills,
        Disposition::Translate {
            keys_dropped: false,
            untranslated: true,
        },
    );
    i.unmapped_keys = vec!["desktop-commander".to_owned(), "playwright".to_owned()];
    let scored = score_pair(&i);
    let skills_row = scored
        .carry_list
        .iter()
        .find(|row| row.artifact == ArtifactKind::Skills)
        .unwrap();
    assert_eq!(skills_row.word, "stays (no written mapping)");
    assert!(skills_row.reason.contains("desktop-commander"));
    assert!(skills_row.reason.contains("playwright"));
}

/// Keys-dropped translate scores 0.6 and lists the dropped keys.
#[test]
fn keys_dropped_translate_scores_lower() {
    let mut map = all_copy();
    map.insert(
        ArtifactKind::Mcp,
        Disposition::Translate {
            keys_dropped: true,
            untranslated: false,
        },
    );
    let mut i = input(map);
    i.unmapped_keys = vec!["browser".to_owned()];
    let scored = score_pair(&i);
    // all-Copy base 1.0 − .10·(1.0−0.6) = 0.96
    assert!((scored.score - 0.96).abs() < 1e-9, "score {}", scored.score);
    let mcp = scored
        .carry_list
        .iter()
        .find(|r| r.artifact == ArtifactKind::Mcp)
        .unwrap();
    assert!(mcp.reason.contains("browser"));
}

/// Todos default Drop until a fixture proves carriage (Req.6) — the
/// base tables never invent carriage.
#[test]
fn todos_default_drop_in_every_base_cell() {
    for scored in full_matrix() {
        let todos = scored
            .carry_list
            .iter()
            .find(|row| row.artifact == ArtifactKind::Todos)
            .unwrap();
        assert_eq!(
            todos.word, "stays",
            "{} todos must default Drop",
            scored.pair
        );
    }
}

/// ZCode→ZCode chat is BLOCKED today (Drop) — honest until the
/// SQLite clone passes (SPEC:282).
#[test]
fn zcode_to_zcode_chat_is_blocked_today() {
    let scored = score_pair(&base_dispositions("zcode", "zcode"));
    assert_eq!(scored.label, FidelityLabel::F0);
    let chat = scored
        .carry_list
        .iter()
        .find(|r| r.artifact == ArtifactKind::Chat)
        .unwrap();
    assert_eq!(chat.word, "stays");
}

/// The committed full-matrix snapshot: pair, score, label, caps —
/// byte-stable (regenerate with UPDATE_GOLDENS=1).
#[test]
fn full_matrix_snapshot_is_committed_and_stable() {
    let matrix: Vec<PairFidelity> = full_matrix();
    let snapshot = serde_json::to_vec_pretty(&matrix).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/fidelity-matrix.json");
    if std::env::var("UPDATE_GOLDENS").is_ok() {
        std::fs::write(&path, &snapshot).unwrap();
    }
    let committed = std::fs::read(&path).expect("run with UPDATE_GOLDENS=1 once");
    assert_eq!(
        committed, snapshot,
        "matrix drifted — the badge ceiling changed"
    );
    // Every cell starts at or below F1 (D-04: beta labelling until
    // E210 flips cells green) — via the recorded cap when the mean
    // exceeded it, or an already-honest lower label.
    for scored in &matrix {
        assert!(
            scored.label <= FidelityLabel::F1,
            "{} advertises {} — above the unvalidated ceiling",
            scored.pair,
            scored.label.as_str()
        );
    }
}

/// Missing dispositions score Drop — the honest default, never an
/// invented Copy.
#[test]
fn missing_artifacts_default_to_drop() {
    let i = input(BTreeMap::new());
    let scored = score_pair(&i);
    assert!(scored.score.abs() < 1e-9);
    assert_eq!(scored.label, FidelityLabel::F0);
}
