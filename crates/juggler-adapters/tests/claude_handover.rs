//! AJ-E204 evidence: the honest Claude transplant — H-F1 sibling set,
//! H-F2 torn-tail refusal, H-F3 collision fork (tree preserved,
//! first-wins dedupe keys), H-F6 pathological-cwd discovery, and the
//! copy-never-move source-hash invariant asserted in every test. The
//! `TRANSPLANT` capability bit stays OFF until AJ-E210 (grep gate).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::capabilities::Capabilities;
use juggler_adapter::handover::HandoverSupport;
use juggler_adapters::{ClaudeCodeAdapter, claude_transcript};
use juggler_core::model::{Account, AccountId, HarnessId, TranscriptRef};

fn adapter() -> ClaudeCodeAdapter {
    ClaudeCodeAdapter::new()
}

fn account(state_dir: &Path, label: &str) -> Account {
    Account {
        id: AccountId::new(label),
        harness: HarnessId::new("claude-code"),
        label: label.to_owned(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    }
}

fn line(uuid: &str, parent: Option<&str>, session: &str, cwd: &str) -> String {
    serde_json::json!({
        "uuid": uuid,
        "parentUuid": parent,
        "sessionId": session,
        "cwd": cwd,
        "type": "user",
        "message": {"role": "user", "content": "work"},
    })
    .to_string()
}

/// The H-F1 fixture tree: main + two subagent siblings + spill.
fn seed_sibling_tree(config: &Path, cwd: &str, session: &str) -> PathBuf {
    let dir = config.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let main = format!(
        "{}\n{}\n",
        line("u1", None, session, cwd),
        line("u2", Some("u1"), session, cwd)
    );
    std::fs::write(dir.join(format!("{session}.jsonl")), &main).unwrap();
    std::fs::write(
        dir.join(format!("{session}-agent-1.jsonl")),
        format!("{}\n", line("s1", None, session, cwd)),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{session}-agent-2.jsonl")),
        format!("{}\n", line("s2", None, session, cwd)),
    )
    .unwrap();
    std::fs::write(dir.join(format!("{session}-spill.jsonl")), b"{}\n").unwrap();
    dir.join(format!("{session}.jsonl"))
}

fn dir_hashes(dir: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let hash = juggler_adapter::handover::sha256_hex(&std::fs::read(&path).unwrap());
            (name, hash)
        })
        .collect();
    out.sort();
    out
}

/// H-F1: the bundle carries the FULL sibling set with a manifest row
/// per file; a single-file bundle is unreachable.
#[test]
fn hf1_sibling_set_bundles_completely() {
    let work = tempfile::tempdir().unwrap();
    let config_a = work.path().join("a");
    let main = seed_sibling_tree(&config_a, "/Users/dev/ai-juggler", "sess-42");
    let before = dir_hashes(main.parent().unwrap());

    let bundle = adapter()
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-42".to_owned()),
        })
        .unwrap();
    assert_eq!(
        bundle.files.len(),
        4,
        "main + 2 subagents + spill: {:?}",
        bundle.files.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert_eq!(bundle.manifest.cwd, PathBuf::from("/Users/dev/ai-juggler"));
    assert_eq!(bundle.manifest.native_id, "sess-42");
    assert_eq!(bundle.manifest.fidelity.level, "F1");
    assert!(
        bundle
            .manifest
            .fidelity
            .caps_applied
            .contains(&"unvalidated-pair".to_owned())
    );

    // Copy-never-move.
    assert_eq!(before, dir_hashes(main.parent().unwrap()));
}

/// H-F2: a torn tail (source still appending) refuses the bundle —
/// never copy torn bytes.
#[test]
fn hf2_torn_tail_refuses() {
    let work = tempfile::tempdir().unwrap();
    let config_a = work.path().join("a");
    let main = seed_sibling_tree(&config_a, "/w/x", "sess-torn");
    let mut torn = std::fs::read(&main).unwrap();
    torn.extend_from_slice(b"{\"uuid\": \"u3\", \"pare"); // mid-append
    std::fs::write(&main, &torn).unwrap();

    let error = adapter()
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-torn".to_owned()),
        })
        .unwrap_err();
    assert!(
        error.to_string().contains("tail does not parse"),
        "torn tail refused: {error}"
    );
}

/// H-F3: an id collision on the target forks a NEW native id, the
/// uuid/parentUuid tree preserved, source untouched.
#[test]
fn hf3_collision_forks_preserving_the_tree() {
    let work = tempfile::tempdir().unwrap();
    let config_a = work.path().join("a");
    let config_b = work.path().join("b");
    let main = seed_sibling_tree(&config_a, "/w/collide", "sess-dup");
    // The target already has a session with the SAME id.
    seed_sibling_tree(&config_b, "/w/collide", "sess-dup");
    let before_a = dir_hashes(&config_a.join("projects").join("-w-collide"));

    let support = adapter();
    let bundle = support
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-dup".to_owned()),
        })
        .unwrap();
    let outcome = support
        .install_bundle(&bundle, &account(&config_b, "acc-b"))
        .unwrap();
    assert!(outcome.forked, "collision detected");
    let installed_id = outcome.reference.native_id.clone().unwrap();
    assert!(installed_id.contains("fork"), "new id: {installed_id}");
    assert!(outcome.reference.path.is_file());

    // The forked transcript kept the tree, rewrote the id chain.
    let forked = std::fs::read(&outcome.reference.path).unwrap();
    let text = String::from_utf8_lossy(&forked).into_owned();
    assert!(text.contains(&installed_id));
    assert!(!text.contains("\"sess-dup\""));
    assert!(text.contains("\"parentUuid\":\"u1\""));
    // Dedupe key intact: the uuid set is unchanged by the fork.
    let uuids = juggler_adapter::handover::uuid_set(&forked);
    assert!(uuids.contains("u1") && uuids.contains("u2"));

    // Source untouched.
    assert_eq!(
        before_a,
        dir_hashes(&config_a.join("projects").join("-w-collide"))
    );
}

/// H-F6 + H-3: a pathological cwd (dots, dashes, spaces, >200 chars)
/// round-trips via DISCOVERY (embedded cwd match), and the naive
/// mangle is only a FLAGGED fallback.
#[test]
fn hf6_pathological_cwd_discovers_and_flags_fallback() {
    let work = tempfile::tempdir().unwrap();
    let long_segment = "p".repeat(180);
    let weird_cwd = format!("/Users/dev/weird.dir name-with-dashes/{long_segment}/x y");
    assert!(weird_cwd.len() > 200);

    // Target already worked in a DIFFERENTLY-NAMED dir whose
    // transcripts embed the same cwd — discovery must find it.
    let config_a = work.path().join("a");
    let config_b = work.path().join("b");
    let main = seed_sibling_tree(&config_a, &weird_cwd, "sess-weird");
    let discovered_dir = config_b.join("projects").join("totally-unmangled-name");
    std::fs::create_dir_all(&discovered_dir).unwrap();
    std::fs::write(
        discovered_dir.join("other.jsonl"),
        format!("{}\n", line("x1", None, "other", &weird_cwd)),
    )
    .unwrap();

    let support = adapter();
    let bundle = support
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-weird".to_owned()),
        })
        .unwrap();
    let outcome = support
        .install_bundle(&bundle, &account(&config_b, "acc-b"))
        .unwrap();
    assert!(!outcome.target_dir_fallback, "discovery found the dir");
    assert!(outcome.reference.path.starts_with(&discovered_dir));

    // And into an EMPTY target: the flagged mangle fallback.
    let config_c = work.path().join("c");
    let outcome2 = support
        .install_bundle(&bundle, &account(&config_c, "acc-c"))
        .unwrap();
    assert!(
        outcome2.target_dir_fallback,
        "fallback flagged, never hidden"
    );
    assert!(outcome2.reference.path.is_file());
}

/// The TRANSPLANT capability bit stays OFF until AJ-E210 proves the
/// pairs (Req.5 grep gate) — the interim label is honest.
#[test]
fn transplant_capability_is_off_until_e210() {
    let capabilities = adapter().capabilities();
    assert!(
        !capabilities.contains(Capabilities::TRANSPLANT),
        "TRANSPLANT off until H-F fixtures + live proof (deep/09 D4)"
    );
}

/// The bundle's compaction signal feeds the F2 cap (§H2.1).
#[test]
fn compact_boundary_marks_the_bundle_compacted() {
    let work = tempfile::tempdir().unwrap();
    let config = work.path().join("a");
    let dir = config.join("projects").join("-w-compact");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sess-c.jsonl"),
        format!(
            "{}\n{{\"type\":\"system\",\"subtype\":\"compact_boundary\",\"cwd\":\"/w/compact\"}}\n",
            line("u1", None, "sess-c", "/w/compact")
        ),
    )
    .unwrap();
    let parsed =
        claude_transcript::parse_transcript(&std::fs::read(dir.join("sess-c.jsonl")).unwrap());
    assert!(parsed.has_compact_boundary());
    // And a clean one does not.
    let clean = format!("{}\n", line("u1", None, "sess-c", "/w/compact"));
    assert!(!claude_transcript::parse_transcript(clean.as_bytes()).has_compact_boundary());
}

/// H-F4 (AJ-E210): a compacted source labels the bundle F2 with the
/// compact-boundary cap recorded — and the raw source bytes stay
/// hash-identical through the bundle read.
#[test]
fn hf4_compacted_source_labels_f2_and_bytes_stay_identical() {
    let work = tempfile::tempdir().unwrap();
    let config = work.path().join("a");
    let dir = config.join("projects").join("-w-compact-hf4");
    std::fs::create_dir_all(&dir).unwrap();
    let main_path = dir.join("sess-c4.jsonl");
    std::fs::write(
        &main_path,
        format!(
            "{}\n{}\n{{\"type\":\"system\",\"subtype\":\"compact_boundary\",\"cwd\":\"/w/compact-hf4\"}}\n{}\n",
            line("u1", None, "sess-c4", "/w/compact-hf4"),
            line("u2", Some("u1"), "sess-c4", "/w/compact-hf4"),
            line("u3", Some("u2"), "sess-c4", "/w/compact-hf4"),
        ),
    )
    .unwrap();
    let before = dir_hashes(&dir);

    let bundle = adapter()
        .bundle_session(&TranscriptRef {
            path: main_path.clone(),
            native_id: Some("sess-c4".to_owned()),
        })
        .unwrap();
    assert_eq!(bundle.manifest.fidelity.level, "F2", "compacted caps at F2");
    assert!(
        bundle
            .manifest
            .fidelity
            .caps_applied
            .contains(&"compact-boundary".to_owned())
    );
    assert!(
        bundle
            .manifest
            .fidelity
            .caps_applied
            .contains(&"unvalidated-pair".to_owned()),
        "still unvalidated until a live cell"
    );

    // Raw bytes identical (copy-never-move).
    assert_eq!(before, dir_hashes(&dir));
}

/// H-F7 (AJ-E210): the golden path runs PTY-INDEPENDENT end to end —
/// bundle → install → the installed transcript parses and preserves
/// the turn tree. Nothing here touches a terminal; it is the CI
/// shape of the ship gate.
#[test]
fn hf7_golden_path_is_pty_independent_end_to_end() {
    let work = tempfile::tempdir().unwrap();
    let config_a = work.path().join("a");
    let config_b = work.path().join("b");
    let main = seed_sibling_tree(&config_a, "/w/golden", "sess-golden");
    let before_a = dir_hashes(main.parent().unwrap());

    let support = adapter();
    let bundle = support
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-golden".to_owned()),
        })
        .unwrap();
    let outcome = support
        .install_bundle(&bundle, &account(&config_b, "acc-b"))
        .unwrap();
    assert!(!outcome.forked, "clean target, no collision");

    // The installed main parses whole (no torn line) and keeps the
    // uuid tree; the sibling set travelled with it.
    let installed_dir = outcome.reference.path.parent().unwrap();
    let installed_main = std::fs::read(&outcome.reference.path).unwrap();
    let uuids = juggler_adapter::handover::uuid_set(&installed_main);
    for expected in ["u1", "u2"] {
        assert!(uuids.contains(expected), "tree kept: {expected}");
    }
    let parsed_installed = claude_transcript::parse_transcript(&installed_main);
    assert!(
        !parsed_installed.messages.is_empty(),
        "installed transcript parses whole"
    );
    assert_eq!(parsed_installed.lines_malformed, 0, "no torn line");
    assert_eq!(
        bundle.files.len(),
        std::fs::read_dir(installed_dir).unwrap().count(),
        "sibling set installed 1:1"
    );
    // Source bytes identical both sides.
    assert_eq!(before_a, dir_hashes(main.parent().unwrap()));
}

/// H-F8 (AJ-E210): the A→B→A chain over the adapter — bundle A →
/// install B → bundle B → return to A (the id collides with A's own
/// session, so the return forks a fresh id). The uuid TREE survives
/// the whole chain with ZERO duplicate uuids inside any transcript,
/// and both sides stay resumable (neither source ever modified).
#[test]
fn hf8_a_to_b_to_a_chain_preserves_tree_with_no_duplicate_uuids() {
    let work = tempfile::tempdir().unwrap();
    let config_a = work.path().join("a");
    let config_b = work.path().join("b");
    let main = seed_sibling_tree(&config_a, "/w/chain", "sess-chain");
    let before_a = dir_hashes(main.parent().unwrap());
    let support = adapter();

    // A → B.
    let bundle_ab = support
        .bundle_session(&TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-chain".to_owned()),
        })
        .unwrap();
    let installed_b = support
        .install_bundle(&bundle_ab, &account(&config_b, "acc-b"))
        .unwrap();

    // B progresses one turn (the away work).
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&installed_b.reference.path)
            .unwrap();
        writeln!(
            file,
            "{}",
            line("b1", Some("u2"), "sess-chain", "/w/chain").replace("\"user\"", "\"assistant\"")
        )
        .unwrap();
    }
    let before_b = dir_hashes(installed_b.reference.path.parent().unwrap());

    // B → A (the return): the id exists on A, so the return forks.
    let bundle_ba = support
        .bundle_session(&TranscriptRef {
            path: installed_b.reference.path.clone(),
            native_id: installed_b.reference.native_id.clone(),
        })
        .unwrap();
    let returned = support
        .install_bundle(&bundle_ba, &account(&config_a, "acc-a"))
        .unwrap();
    assert!(returned.forked, "the return forks a fresh id on A");

    // The chain's tree: u1, u2 (base) + b1 (away) all present in the
    // returned transcript; every uuid appears EXACTLY once (the
    // dedupe-first-wins contract — zero duplicate uuids).
    let returned_bytes = std::fs::read(&returned.reference.path).unwrap();
    let text = String::from_utf8_lossy(&returned_bytes).into_owned();
    for expected in ["u1", "u2", "b1"] {
        let count = text.matches(&format!("\"uuid\":\"{expected}\"")).count()
            + text.matches(&format!("\"uuid\": \"{expected}\"")).count();
        assert_eq!(count, 1, "{expected} exactly once in the returned tree");
    }

    // Both originals stay byte-identical (copy-never-move both
    // legs). The return ADDS its forked session files to A's dir —
    // the originals themselves must be untouched, so compare the
    // original names only.
    let after_a: Vec<_> = dir_hashes(main.parent().unwrap())
        .into_iter()
        .filter(|(name, _)| before_a.iter().any(|(n, _)| n == name))
        .collect();
    assert_eq!(before_a, after_a, "A's original files untouched");
    assert_eq!(
        before_b,
        dir_hashes(installed_b.reference.path.parent().unwrap())
    );
}
