//! AJ-E208 evidence: the redaction policy end-to-end over REAL pack
//! bytes — the secret canary appears in ZERO pack bytes under Strip
//! (conversation, staged diffs, human brief, state), counts land in
//! the manifest under every policy (Keep included), per-item
//! un-redact keeps exactly that item for this pack only, and the
//! always-on credential guard refuses credential material under
//! EVERY policy (D-14's hard line: credentials never enter a pack).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_handover::pack::{
    ConversationMode, IrRole, IrTurn, PackArtifacts, PackFidelity, PackGit, PackInput, PackLineage,
    PackProject, PackQuota, PackSource, write_pack,
};
use juggler_handover::redaction::{
    RedactionApply, RedactionPolicy, list_segments, redacted_pack, resolve_policy, strip_segments,
};
use std::collections::BTreeSet;

const CANARY_SK: &str = "sk-canary12345678";
const CANARY_ENV: &str = "canary-env-value-99";
const CANARY_BEARER: &str = "Bearer canary-bearer-777777";

fn canary_input() -> PackInput {
    PackInput {
        pack_id: "01936657-6c2e-7a30-9a1f-3f2a11e9d000".to_owned(),
        created_at_ms: 1_792_094_400_000,
        work_id: "work-000123".to_owned(),
        edge_id: Some("edge-000045".to_owned()),
        source: PackSource {
            provider: "claude-code".to_owned(),
            account_label: "Work Max".to_owned(),
            profile_name: None,
            native_session_id: "b2632f0e-9c1d-4a17-9f6f-0f1d2e3b4c5d".to_owned(),
            harness_version: None,
            epoch_id: "ep-000001".to_owned(),
        },
        target_hint: None,
        project: PackProject {
            root: "/Users/dev/ai-juggler".to_owned(),
            project_id: "proj-000007".to_owned(),
        },
        git: PackGit {
            branch: "main".to_owned(),
            head_sha: None,
            dirty: true,
            diff_stat: None,
            worktree_path: None,
        },
        turns: vec![
            IrTurn {
                uuid: Some("11111111-1111-1111-1111-111111111111".to_owned()),
                role: IrRole::User,
                text: format!("here is my key {CANARY_SK} for the deploy"),
                at_ms: Some(1_792_000_000_000),
                tokens: None,
            },
            IrTurn {
                uuid: Some("22222222-2222-2222-2222-222222222222".to_owned()),
                role: IrRole::Assistant,
                text: format!("auth via {CANARY_BEARER} works"),
                at_ms: Some(1_792_000_100_000),
                tokens: None,
            },
        ],
        compacted: false,
        artifacts: PackArtifacts::default(),
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
            "diffs/env.patch".to_owned(),
            format!("+API_TOKEN={CANARY_ENV}\n").into_bytes(),
        )],
    }
}

fn pack_bytes(pack: &juggler_handover::pack::GeneratedPack) -> Vec<Vec<u8>> {
    let mut all = vec![
        pack.state_json.clone(),
        pack.conversation_md.clone(),
        pack.pack_md.clone(),
    ];
    all.extend(pack.extra_files.iter().map(|(_, bytes)| bytes.clone()));
    all
}

#[test]
fn zero_canary_bytes_under_strip() {
    let pack = redacted_pack(
        &canary_input(),
        ConversationMode::Full,
        &RedactionApply {
            strip: true,
            keep: BTreeSet::new(),
        },
    );
    for (index, bytes) in pack_bytes(&pack).iter().enumerate() {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains(CANARY_SK), "canary sk in pack file {index}");
        assert!(
            !text.contains(CANARY_ENV),
            "canary env in pack file {index}"
        );
        assert!(
            !text.contains("canary-bearer-777777"),
            "canary bearer in pack file {index}"
        );
    }
    // The strip writes and the WRITE guard agree: a stripped pack
    // passes the always-on credential scan.
    let dir = tempfile::tempdir().unwrap();
    assert!(write_pack(&dir.path().join("pack"), &pack).is_ok());
}

#[test]
fn counts_land_in_the_manifest_under_every_policy() {
    // Strip: counts recorded, values gone.
    let pack = redacted_pack(
        &canary_input(),
        ConversationMode::Full,
        &RedactionApply {
            strip: true,
            keep: BTreeSet::new(),
        },
    );
    let state =
        serde_json::from_slice::<juggler_handover::pack::PackState>(&pack.state_json).unwrap();
    let kinds: Vec<(String, u32)> = state
        .redactions
        .iter()
        .map(|row| (row.kind.clone(), row.count))
        .collect();
    assert!(kinds.contains(&("sk-key".to_owned(), 1)), "{kinds:?}");
    assert!(kinds.contains(&("bearer".to_owned(), 1)), "{kinds:?}");
    assert!(kinds.contains(&("env-value".to_owned(), 1)), "{kinds:?}");

    // Keep: counts STILL recorded (D-14: the manifest lists what was
    // detected and how the policy treated it) — and the values stay
    // on this Mac.
    let pack = redacted_pack(
        &canary_input(),
        ConversationMode::Full,
        &RedactionApply {
            strip: false,
            keep: BTreeSet::new(),
        },
    );
    let state =
        serde_json::from_slice::<juggler_handover::pack::PackState>(&pack.state_json).unwrap();
    assert!(!state.redactions.is_empty());
    let conversation = String::from_utf8_lossy(&pack.conversation_md);
    assert!(conversation.contains(CANARY_SK));
}

#[test]
fn un_redact_keeps_exactly_that_item_for_this_pack() {
    let segments: Vec<String> = canary_input()
        .turns
        .iter()
        .map(|turn| turn.text.clone())
        .collect();
    let refs: Vec<&str> = segments.iter().map(String::as_str).collect();
    let listed = list_segments(&refs);
    assert_eq!(listed.len(), 2, "sk-key + bearer across the turns");

    // Un-redact ONLY the sk-key (index 0) for this pack.
    let keep: BTreeSet<usize> = [0].into_iter().collect();
    let pack = redacted_pack(
        &canary_input(),
        ConversationMode::Full,
        &RedactionApply { strip: true, keep },
    );
    let conversation = String::from_utf8_lossy(&pack.conversation_md);
    assert!(
        conversation.contains(CANARY_SK),
        "the un-redacted item survives"
    );
    assert!(
        !conversation.contains("canary-bearer-777777"),
        "the rest still strips"
    );
    let env = pack
        .extra_files
        .iter()
        .find(|(path, _)| path == "diffs/env.patch")
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).to_string())
        .unwrap_or_default();
    assert!(!env.contains(CANARY_ENV));

    // Un-redacting is NOT a policy change: the next pack (same
    // input, no keep) strips everything again.
    let again = redacted_pack(
        &canary_input(),
        ConversationMode::Full,
        &RedactionApply {
            strip: true,
            keep: BTreeSet::new(),
        },
    );
    assert!(!String::from_utf8_lossy(&again.conversation_md).contains(CANARY_SK));
}

#[test]
fn credentials_never_enter_any_pack_including_keep() {
    // The hard line under D-14: credential MATERIAL (private-key
    // blocks, embedded API keys) is refused by the ALWAYS-ON write
    // guard regardless of policy — Keep is for conversation values,
    // not credential files.
    let mut input = canary_input();
    input.staged_files.push((
        "auth.json".to_owned(),
        b"{\"sk-embedded-canary-key-99\"}".to_vec(),
    ));
    let pack = redacted_pack(
        &input,
        ConversationMode::Full,
        &RedactionApply {
            strip: false,
            keep: BTreeSet::new(),
        },
    );
    let dir = tempfile::tempdir().unwrap();
    assert!(
        write_pack(&dir.path().join("pack"), &pack).is_err(),
        "the credential guard refuses even under Keep"
    );
}

#[test]
fn policy_resolution_and_persistence_shape() {
    assert_eq!(resolve_policy(None, None), None);
    assert_eq!(
        resolve_policy(Some(RedactionPolicy::Keep), Some(RedactionPolicy::Strip)),
        Some(RedactionPolicy::Strip)
    );
    // Segment listing order is deterministic (preview ↔ apply agree).
    let segments: Vec<String> = canary_input()
        .turns
        .iter()
        .map(|turn| turn.text.clone())
        .collect();
    let refs: Vec<&str> = segments.iter().map(String::as_str).collect();
    let once = list_segments(&refs);
    let twice = list_segments(&refs);
    assert_eq!(once, twice);
    // strip_segments is pure — same inputs, same outputs.
    let left = strip_segments(&segments, true, &BTreeSet::new());
    let right = strip_segments(&segments, true, &BTreeSet::new());
    assert_eq!(left, right);
}
