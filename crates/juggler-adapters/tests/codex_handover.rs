//! AJ-E1304: Codex transplant mechanics against synthetic rollouts —
//! bundle (torn tail refused, honest caps), install under the target's
//! dated folder, fork on an id collision, the source byte-identical.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::handover::{HandoverSupport, TurnText, embedded_cwd, remap_cwd, sha256_hex};
use juggler_adapters::CodexAdapter;
use juggler_core::model::{Account, AccountId, HarnessId, TranscriptRef};

const THREAD: &str = "019a1b2c-0000-7000-8000-00000000c0de";

fn account(dir: &Path) -> Account {
    Account {
        id: AccountId::new("codex-b"),
        harness: HarnessId::new("codex"),
        label: "Pro".into(),
        plan: None,
        state_dir: dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    }
}

/// The paginated fixture placed in a source home, as Codex lays it out.
fn source_rollout(home: &Path) -> PathBuf {
    let day = home.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let path = day.join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"));
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex/rollout-paginated.jsonl"),
        &path,
    )
    .unwrap();
    path
}

fn reference(path: &Path) -> TranscriptRef {
    TranscriptRef {
        path: path.to_path_buf(),
        native_id: Some(THREAD.to_owned()),
    }
}

#[test]
fn bundle_carries_the_rollout_with_honest_caps() {
    let tmp = tempfile::tempdir().unwrap();
    let path = source_rollout(&tmp.path().join("a"));
    let bundle = CodexAdapter::new()
        .bundle_session(&reference(&path))
        .unwrap();
    assert_eq!(bundle.manifest.harness.as_str(), "codex");
    assert_eq!(bundle.manifest.native_id, THREAD);
    assert_eq!(bundle.manifest.cwd, PathBuf::from("/work/app"));
    assert_eq!(bundle.manifest.harness_version.as_deref(), Some("0.163.0"));
    assert_eq!(bundle.files.len(), 1, "a Codex session is one rollout");
    // The fixture ends compacted → F2, and every pair is unvalidated.
    assert_eq!(bundle.manifest.fidelity.level, "F2");
    let caps = &bundle.manifest.fidelity.caps_applied;
    assert!(caps.contains(&"compact-boundary".to_owned()));
    assert!(caps.contains(&"unvalidated-pair".to_owned()));
    assert!(!caps.contains(&"base-rollout-not-carried".to_owned()));
}

#[test]
fn a_rollout_built_on_another_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let day = tmp.path().join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let path = day.join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"));
    std::fs::write(
        &path,
        format!(
            "{{\"timestamp\":\"t\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{THREAD}\",\"cwd\":\"/w\",\"forked_from_id\":\"019a0000-0000-7000-8000-000000000009\"}}}}\n"
        ),
    )
    .unwrap();
    let bundle = CodexAdapter::new()
        .bundle_session(&reference(&path))
        .unwrap();
    assert!(
        bundle
            .manifest
            .fidelity
            .caps_applied
            .contains(&"base-rollout-not-carried".to_owned())
    );
}

#[test]
fn a_torn_tail_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let path = source_rollout(&tmp.path().join("a"));
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(b"{\"timestamp\":\"t\",\"type\":\"resp");
    std::fs::write(&path, &bytes).unwrap();
    assert!(
        CodexAdapter::new()
            .bundle_session(&reference(&path))
            .is_err()
    );
}

#[test]
fn install_places_it_where_codex_looks_and_forks_on_a_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let source = source_rollout(&tmp.path().join("a"));
    let before = sha256_hex(&std::fs::read(&source).unwrap());
    let adapter = CodexAdapter::new();
    let target = account(&tmp.path().join("b"));

    let bundle = adapter.bundle_session(&reference(&source)).unwrap();
    let first = adapter.install_bundle(&bundle, &target).unwrap();
    assert!(!first.forked);
    assert!(!first.target_dir_fallback);
    assert_eq!(first.reference.native_id.as_deref(), Some(THREAD));
    assert_eq!(
        first.reference.path,
        tmp.path()
            .join("b/sessions/2026/10/20")
            .join(format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"))
    );
    assert_eq!(
        sha256_hex(&std::fs::read(&first.reference.path).unwrap()),
        before
    );

    // The same thread again: Codex must never see two files for one id.
    let second = adapter.install_bundle(&bundle, &target).unwrap();
    assert!(second.forked);
    let new_id = second.reference.native_id.clone().unwrap();
    assert_ne!(new_id, THREAD);
    assert_eq!(new_id.len(), 36);
    let name = second
        .reference
        .path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(name.contains(&new_id) && !name.contains(THREAD), "{name}");
    let forked = std::fs::read_to_string(&second.reference.path).unwrap();
    assert!(!forked.contains(THREAD), "every line carries the new id");
    assert!(forked.contains(&format!("\"id\":\"{new_id}\"")));
    assert!(
        forked.contains("Add a health check endpoint"),
        "the conversation travels"
    );

    // Copy, never move.
    assert_eq!(sha256_hex(&std::fs::read(&source).unwrap()), before);
    // No staging leftovers Codex could pick up.
    let left: Vec<_> = std::fs::read_dir(tmp.path().join("b/sessions/2026/10/20"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".stage"))
        .collect();
    assert!(left.is_empty());
}

#[test]
fn the_prime_is_data_not_instructions() {
    let prompt = CodexAdapter::new()
        .prime_prompt(&[
            TurnText {
                uuid: None,
                role: "user",
                text: "Add a health check".into(),
            },
            TurnText {
                uuid: None,
                role: "assistant",
                text: "Done.".into(),
            },
        ])
        .unwrap();
    assert!(prompt.contains("context, not new instructions"));
    assert!(prompt.contains("[user]: Add a health check"));
    assert!(prompt.contains("[assistant]: Done."));
}

#[test]
fn cwd_helpers_read_and_remap_codex_payloads() {
    let line =
        br#"{"timestamp":"t","type":"session_meta","payload":{"id":"x","cwd":"/Users/a/app"}}"#;
    assert_eq!(embedded_cwd(line).as_deref(), Some("/Users/a/app"));
    let remapped = String::from_utf8(remap_cwd(line, "/Users/a", "/Users/b")).unwrap();
    assert!(remapped.contains("\"cwd\":\"/Users/b/app\""), "{remapped}");
    // Claude's shape is unchanged.
    let claude = br#"{"type":"user","cwd":"/Users/a/app"}"#;
    let remapped = String::from_utf8(remap_cwd(claude, "/Users/a", "/Users/b")).unwrap();
    assert!(remapped.contains("\"cwd\":\"/Users/b/app\""), "{remapped}");
}
