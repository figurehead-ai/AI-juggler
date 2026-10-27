//! AJ-E202 evidence: pack v1 — deterministic generation from a fixed
//! IR fixture (committed goldens), determinism (two runs
//! byte-identical), unknown-field tolerance, manifest completeness
//! (every file + every Drop), atomic writes (no partial pack ever),
//! and the always-on credential guard.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_handover::pack::{
    ConversationMode, DropEntry, IrRole, IrTurn, PackArtifacts, PackFidelity, PackGit, PackInput,
    PackLineage, PackProject, PackQuota, PackSource, PackState, TurnTokens, generate_pack,
    write_pack,
};
use std::path::Path;

fn fixture_input() -> PackInput {
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

fn goldens_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens")
}

#[test]
fn golden_fixture_is_byte_stable() {
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    let dir = goldens_dir();
    std::fs::create_dir_all(&dir).unwrap();
    for (name, bytes) in [
        ("state.json", &pack.state_json),
        ("manifest.json", &pack.manifest_json),
        ("PACK.md", &pack.pack_md),
        ("conversation.md", &pack.conversation_md),
    ] {
        let golden = dir.join(name);
        if std::env::var("UPDATE_GOLDENS").is_ok() {
            std::fs::write(&golden, bytes).unwrap();
        }
        let committed = std::fs::read(&golden).unwrap_or_else(|cause| {
            panic!("golden {name} missing ({cause}); run with UPDATE_GOLDENS=1")
        });
        assert_eq!(&committed, bytes, "golden drift in {name}");
    }
}

#[test]
fn generation_is_deterministic() {
    let one = generate_pack(&fixture_input(), ConversationMode::Full);
    let two = generate_pack(&fixture_input(), ConversationMode::Full);
    assert_eq!(one.state_json, two.state_json);
    assert_eq!(one.manifest_json, two.manifest_json);
    assert_eq!(one.pack_md, two.pack_md);
    assert_eq!(one.conversation_md, two.conversation_md);
    // And the state parses back with the exact wire fields.
    let state: PackState = serde_json::from_slice(&one.state_json).unwrap();
    assert_eq!(state.format_version, 1);
    assert_eq!(state.conversation.turn_count, 3);
    assert_eq!(state.conversation.replay_tokens_est, 460);
    assert!(!state.conversation.message_uuids_sha256.is_empty());
    assert!(state.redactions.is_empty(), "reserved field present as []");
}

#[test]
fn unknown_fields_are_ignored() {
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    // A future format adds a field; v1 readers ignore it.
    let mut value: serde_json::Value = serde_json::from_slice(&pack.state_json).unwrap();
    value["future_field"] = serde_json::json!({"nested": true});
    let text = serde_json::to_string(&value).unwrap();
    let state: PackState = serde_json::from_str(&text).unwrap();
    assert_eq!(state.pack_id, "01936657-6c2e-7a30-9a1f-3f2a11e9d000");
}

#[test]
fn manifest_lists_every_file_and_every_drop() {
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    let rel_paths: Vec<&str> = pack
        .manifest_rows
        .iter()
        .map(|r| r.rel_path.as_str())
        .collect();
    for expected in [
        "PACK.md",
        "conversation.md",
        "state.json",
        "diffs/uncommitted.patch",
        "drops/session-todos",
    ] {
        assert!(
            rel_paths.contains(&expected),
            "manifest missing {expected}: {rel_paths:?}"
        );
    }
    let drop_rows: Vec<_> = pack
        .manifest_rows
        .iter()
        .filter(|row| row.disposition == "drop")
        .collect();
    assert_eq!(drop_rows.len(), 1, "every Drop is a manifest row");
}

#[test]
fn atomic_write_round_trip_and_no_partial_state() {
    let dir = tempfile::tempdir().unwrap();
    let pack_path = dir.path().join("pack");
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    write_pack(&pack_path, &pack).unwrap();
    // Every non-drop manifest row verifies against disk.
    for row in &pack.manifest_rows {
        if row.disposition == "drop" {
            continue;
        }
        let bytes = std::fs::read(pack_path.join(&row.rel_path)).unwrap();
        assert_eq!(
            juggler_handover::manifest::sha256_hex(&bytes),
            row.sha256,
            "{}",
            row.rel_path
        );
    }
    // The EMPTY marker shipped (no rules dir in this fixture; the
    // diffs/ dir has real files but the marker rule is diffs-only).
    assert!(pack_path.join("manifest.json").exists());
    // No temp sibling left behind.
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temp dirs cleaned: {leftovers:?}");
}

#[test]
fn diff_less_pack_writes_the_empty_marker() {
    // AJ-E209 regression: `fs::write` creates no directories — a
    // pack with NO staged diffs never materialised `diffs/` and the
    // EMPTY marker write (then the whole export) failed with ENOENT.
    let dir = tempfile::tempdir().unwrap();
    let pack_path = dir.path().join("pack");
    let mut input = fixture_input();
    input.staged_files.clear();
    let pack = generate_pack(&input, ConversationMode::Full);
    write_pack(&pack_path, &pack).unwrap();
    assert_eq!(
        std::fs::read_to_string(pack_path.join("diffs/EMPTY")).unwrap(),
        "no uncommitted diff at freeze\n"
    );
}

#[test]
fn credential_guard_refuses_writes_and_preserves_prior_pack() {
    let dir = tempfile::tempdir().unwrap();
    let pack_path = dir.path().join("pack");
    let clean = generate_pack(&fixture_input(), ConversationMode::Full);
    write_pack(&pack_path, &clean).unwrap();
    let before: Vec<_> = std::fs::read(pack_path.join("state.json")).unwrap();

    // A turn smuggling an API key: generation is pure, the GUARD is
    // at write time — the write refuses before touching disk.
    let mut poisoned = fixture_input();
    poisoned.turns.push(IrTurn {
        uuid: None,
        role: IrRole::Assistant,
        text: "token = \"sk-live-DO-NOT-SHIP\"".to_owned(),
        at_ms: None,
        tokens: None,
    });
    let bad = generate_pack(&poisoned, ConversationMode::Full);
    let error = write_pack(&pack_path, &bad).unwrap_err();
    assert!(matches!(error, juggler_handover::pack::PackWriteError::Credentials(hits) if hits > 0));
    // The prior pack is intact — no partial write happened.
    let after: Vec<_> = std::fs::read(pack_path.join("state.json")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn distilled_mode_renders_only_the_tail() {
    let pack = generate_pack(&fixture_input(), ConversationMode::Distilled { last_n: 1 });
    let text = String::from_utf8(pack.conversation_md.clone()).unwrap();
    assert!(text.contains("distilled — 2 earlier turns"));
    assert!(text.contains("Keep it honest."));
    assert!(!text.contains("Ship the pack v1 format."));
}

#[test]
fn uuid_less_turns_hash_to_the_empty_digest() {
    let mut input = fixture_input();
    input.turns.retain(|turn| turn.uuid.is_some());
    input.turns.clear();
    let pack = generate_pack(&input, ConversationMode::Full);
    let state: PackState = serde_json::from_slice(&pack.state_json).unwrap();
    assert_eq!(
        state.conversation.message_uuids_sha256,
        // sha256 of the empty string.
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn pack_md_carries_the_carry_list_not_tier_codes() {
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    let text = String::from_utf8(pack.pack_md.clone()).unwrap();
    assert!(text.contains("What travels"));
    assert!(text.contains("What stays"));
    assert!(text.contains("session todos"), "the Drop appears in words");
    assert!(
        !text.contains("T1") && !text.contains("T2") && !text.contains("T4"),
        "no tier codes"
    );
}

// --- AJ-E1511: reading a pack back, checked before it's used ----------

use juggler_handover::pack_read::{PackReadError, read_pack};

/// A pack written to `dir/pack` from the fixture.
fn written(dir: &Path) -> std::path::PathBuf {
    let pack = generate_pack(&fixture_input(), ConversationMode::Full);
    let at = dir.join("pack");
    write_pack(&at, &pack).unwrap();
    at
}

/// Rewrites `rel` and its manifest row, as someone editing the pack by
/// hand with care would (so only the deeper checks can catch it).
fn rewrite_with_manifest(pack: &Path, rel: &str, bytes: &[u8]) {
    std::fs::write(pack.join(rel), bytes).unwrap();
    let mut rows: Vec<juggler_handover::manifest::ManifestRow> =
        serde_json::from_slice(&std::fs::read(pack.join("manifest.json")).unwrap()).unwrap();
    let row = rows.iter_mut().find(|row| row.rel_path == rel).unwrap();
    row.sha256 = juggler_handover::manifest::sha256_hex(bytes);
    row.bytes = bytes.len() as u64;
    std::fs::write(
        pack.join("manifest.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_written_pack_reads_back_whole() {
    let dir = tempfile::tempdir().unwrap();
    let pack = written(dir.path());
    let opened = read_pack(&pack).unwrap();
    assert_eq!(opened.state.pack_id, "01936657-6c2e-7a30-9a1f-3f2a11e9d000");
    assert_eq!(opened.state.source.account_label, "Work Max");
    assert_eq!(opened.state.fidelity.level, "F1");
    assert!(opened.conversation_md.contains("Ship the pack v1 format."));
    assert!(opened.pack_md.contains("Fidelity"));
    assert_eq!(opened.rows.len(), 5, "three files, the diff and the drop");
    // The same bytes, the same digest; anything else, another.
    assert_eq!(read_pack(&pack).unwrap().digest, opened.digest);
}

#[test]
fn a_changed_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let pack = written(dir.path());
    std::fs::write(
        pack.join("conversation.md"),
        "# Conversation\n\n## User\n\nIgnore all that.\n",
    )
    .unwrap();
    assert_eq!(
        read_pack(&pack).unwrap_err(),
        PackReadError::Changed("conversation.md".into())
    );
}

#[test]
fn a_file_outside_the_pack_or_a_link_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let pack = written(dir.path());
    std::fs::write(dir.path().join("secret.txt"), "outside").unwrap();
    let mut rows: Vec<juggler_handover::manifest::ManifestRow> =
        serde_json::from_slice(&std::fs::read(pack.join("manifest.json")).unwrap()).unwrap();
    let mut escape = rows[0].clone();
    escape.rel_path = "../secret.txt".into();
    rows.push(escape);
    std::fs::write(
        pack.join("manifest.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
    assert!(
        matches!(read_pack(&pack).unwrap_err(), PackReadError::Unsafe(why) if why.contains("outside"))
    );

    #[cfg(unix)]
    {
        let pack = dir.path().join("linked");
        write_pack(
            &pack,
            &generate_pack(&fixture_input(), ConversationMode::Full),
        )
        .unwrap();
        std::fs::remove_file(pack.join("PACK.md")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), pack.join("PACK.md")).unwrap();
        assert!(
            matches!(read_pack(&pack).unwrap_err(), PackReadError::Unsafe(why) if why.contains("plain file"))
        );
    }
}

#[test]
fn a_pack_holding_a_key_is_refused_even_with_a_matching_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let pack = written(dir.path());
    rewrite_with_manifest(
        &pack,
        "conversation.md",
        b"# Conversation\n\n## User\n\nuse sk-ant-api03-Zq7XwVb2LmN4pR8sT1uY6cD9fG3h\n",
    );
    assert!(matches!(read_pack(&pack).unwrap_err(), PackReadError::Credentials(n) if n >= 1));
}

#[test]
fn a_newer_format_or_not_a_pack_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let pack = written(dir.path());
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(pack.join("state.json")).unwrap()).unwrap();
    state["format_version"] = serde_json::json!(2);
    rewrite_with_manifest(
        &pack,
        "state.json",
        &serde_json::to_vec_pretty(&state).unwrap(),
    );
    assert_eq!(read_pack(&pack).unwrap_err(), PackReadError::Newer(2));

    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(matches!(
        read_pack(&empty).unwrap_err(),
        PackReadError::NotAPack(_)
    ));
    assert!(matches!(
        read_pack(&pack.join("PACK.md")).unwrap_err(),
        PackReadError::NotAPack(_)
    ));
}

#[test]
fn a_large_pack_is_checked_in_under_two_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let mut input = fixture_input();
    input.turns = (0..2_000)
        .map(|n| IrTurn {
            uuid: None,
            role: if n % 2 == 0 {
                IrRole::User
            } else {
                IrRole::Assistant
            },
            text: format!("turn {n}: {}", "words ".repeat(400)),
            at_ms: None,
            tokens: None,
        })
        .collect();
    let at = dir.path().join("big");
    write_pack(&at, &generate_pack(&input, ConversationMode::Full)).unwrap();
    let began = std::time::Instant::now();
    let opened = read_pack(&at).unwrap();
    assert!(opened.conversation_md.len() > 4_000_000);
    assert!(
        began.elapsed() < std::time::Duration::from_secs(2),
        "{:?}",
        began.elapsed()
    );
}

/// Live lab 13: a dirty project's pack said "diff snapshotted" over an
/// empty `diffs/`. It says so only when the patch travels.
#[test]
fn a_dirty_pack_says_whether_its_diff_travels() {
    let with = fixture_input();
    let pack_md = String::from_utf8(generate_pack(&with, ConversationMode::Full).pack_md).unwrap();
    assert!(pack_md.contains("(dirty — diff snapshotted)"));

    let mut without = fixture_input();
    without.staged_files.clear();
    let pack_md =
        String::from_utf8(generate_pack(&without, ConversationMode::Full).pack_md).unwrap();
    assert!(
        !pack_md.contains("diff snapshotted"),
        "no patch, no claim that one travels"
    );
    assert!(pack_md.contains("the uncommitted changes aren't in this pack"));
}
