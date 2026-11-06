//! ADR-0028, live: a real Claude app family through the real switch
//! pipeline into the maintainer's test profile (`aij-test`), its records
//! carried so the app nests them when it starts. Opt-in (`--ignored`),
//! run by hand with the target app quit:
//!
//! ```sh
//! AIJ_FAMILY_SRC_INSTANCE=…/ClaudeProfiles/<source> \
//! AIJ_FAMILY_PARENT=<conversation id> \
//! AIJ_FAMILY_DST_CONFIG=…/ClaudeCodeProfiles/aij-test \
//! AIJ_FAMILY_DST_RECORDS=…/ClaudeProfiles/aij-test/claude-code-sessions/<account>/<org> \
//! AIJ_FAMILY_MANIFEST=<file to list what was placed> \
//! cargo test -p juggler-adapters --test family_live -- --ignored --nocapture
//! ```
//!
//! The source is only read. Everything placed is listed in the manifest,
//! so the test's leftovers can be taken back exactly.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapters::{ClaudeCodeAdapter, claude_app_family};
use juggler_core::model::{Account, AccountId, HarnessId, TranscriptRef};
use juggler_handover::pipeline::{self, FreezeChoice, Mechanism};

fn var(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

/// The conversation file `id` under a Claude Code folder's projects.
fn transcript(config: &Path, id: &str) -> Option<PathBuf> {
    let name = format!("{id}.jsonl");
    std::fs::read_dir(config.join("projects"))
        .ok()?
        .flatten()
        .map(|dir| dir.path().join(&name))
        .find(|path| path.is_file())
}

#[test]
#[ignore = "live: a real family into the aij-test profile, run by hand"]
fn a_real_family_moves_into_the_test_profile() {
    let source_instance = var("AIJ_FAMILY_SRC_INSTANCE");
    let source_config = var("AIJ_FAMILY_SRC_CONFIG");
    let parent = std::env::var("AIJ_FAMILY_PARENT").expect("set AIJ_FAMILY_PARENT");
    let target_config = var("AIJ_FAMILY_DST_CONFIG");
    let target_records = var("AIJ_FAMILY_DST_RECORDS");
    let manifest = var("AIJ_FAMILY_MANIFEST");
    assert!(
        target_config.to_string_lossy().contains("aij-test")
            && target_records.to_string_lossy().contains("aij-test"),
        "the live test writes only into the aij-test profile"
    );

    let root = claude_app_family::root(&source_instance, &parent).expect("the parent's own record");
    let children = claude_app_family::descendants(&source_instance, &parent);
    // The conversations are already there (an earlier run): only the app's
    // records are carried, beside the parent's record the app made when it
    // imported it, as a switch does.
    if std::env::var("AIJ_FAMILY_RECORDS_ONLY").is_ok() {
        let (children, absent): (Vec<_>, Vec<_>) = children
            .into_iter()
            .partition(|child| transcript(&target_config, &child.conversation).is_some());
        println!(
            "{} without their conversation there, left out",
            absent.len()
        );
        let placed =
            claude_app_family::place(&children, &target_records, &[]).expect("records placed");
        let lines: Vec<String> = placed.iter().map(|p| p.display().to_string()).collect();
        std::fs::write(&manifest, lines.join("\n") + "\n").expect("manifest");
        println!("placed {} records beside the imported parent", placed.len());
        assert_eq!(placed.len(), children.len());
        return;
    }
    println!(
        "family: the conversation and {} sessions started from it",
        children.len()
    );
    let refs: Vec<TranscriptRef> = children
        .iter()
        .map(|child| TranscriptRef {
            path: transcript(&source_config, &child.conversation)
                .unwrap_or_else(|| panic!("no conversation file for {}", child.conversation)),
            native_id: Some(child.conversation.clone()),
        })
        .collect();
    let target = Account {
        id: AccountId::new("aij-test"),
        harness: HarnessId::new("claude-code"),
        label: "aij-test".into(),
        plan: None,
        state_dir: target_config.clone(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let started = std::time::Instant::now();
    let (record, _) = pipeline::execute_family(
        &ClaudeCodeAdapter::new(),
        &TranscriptRef {
            path: transcript(&source_config, &parent).expect("the parent's conversation file"),
            native_id: Some(parent.clone()),
        },
        &refs,
        &target,
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        None,
        &mut |_| Ok(0),
    )
    .unwrap_or_else(|failure| panic!("the switch failed: {failure} (at {:?})", failure.stage()));
    println!("copied and verified in {:?}", started.elapsed());

    // The family's records, the conversation's own at the top.
    let mut family = vec![root];
    family.extend(children);
    let renamed: Vec<(String, String)> = std::iter::once((&parent, &record.install))
        .chain(
            refs.iter()
                .filter_map(|r| r.native_id.as_ref())
                .zip(&record.family),
        )
        .filter_map(|(old, install)| {
            let new = install.reference.native_id.clone()?;
            (new != *old).then(|| (old.clone(), new))
        })
        .collect();
    let placed_records =
        claude_app_family::place(&family, &target_records, &renamed).expect("records placed");

    let mut lines: Vec<String> = Vec::new();
    for install in std::iter::once(&record.install).chain(&record.family) {
        for (_, path) in &install.placed {
            lines.push(path.display().to_string());
        }
    }
    for path in &placed_records {
        lines.push(path.display().to_string());
    }
    std::fs::write(&manifest, lines.join("\n") + "\n").expect("manifest");
    println!(
        "placed {} files and {} records (listed in {})",
        lines.len() - placed_records.len(),
        placed_records.len(),
        manifest.display()
    );
    assert_eq!(placed_records.len(), family.len());
    assert!(
        renamed.is_empty(),
        "nothing collided on a fresh profile: {renamed:?}"
    );
}

/// ADR-0028 (`automation_handover`), live: the new quieting and waking of
/// conversations in the `aij-test` profile only, with the app quit:
/// `AIJ_AUTOMATION_INSTANCE=…/ClaudeProfiles/aij-test AIJ_AUTOMATION_OP=quiet|wake
/// AIJ_AUTOMATION_CONVERSATIONS=<id,id> AIJ_AUTOMATION_RESTORE=<file>`.
#[test]
#[ignore = "live: quiet or wake records in the aij-test profile, run by hand"]
fn quiet_and_wake_in_the_test_profile() {
    use juggler_adapters::claude_app_automation as automation;
    let instance = var("AIJ_AUTOMATION_INSTANCE");
    assert!(
        instance.to_string_lossy().contains("aij-test"),
        "only the aij-test profile"
    );
    let restore_file = var("AIJ_AUTOMATION_RESTORE");
    match std::env::var("AIJ_AUTOMATION_OP").as_deref() {
        Ok("quiet") => {
            let conversations: Vec<String> = std::env::var("AIJ_AUTOMATION_CONVERSATIONS")
                .expect("set AIJ_AUTOMATION_CONVERSATIONS")
                .split(',')
                .map(str::to_owned)
                .collect();
            let restore = automation::quiet(&instance, &conversations).expect("quiet");
            std::fs::write(&restore_file, serde_json::to_vec_pretty(&restore).unwrap()).unwrap();
            println!("quieted: {restore}");
        }
        Ok("wake") => {
            let restore: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&restore_file).unwrap()).unwrap();
            automation::wake(&instance, &restore).expect("wake");
            println!("woken");
        }
        other => panic!("AIJ_AUTOMATION_OP must be quiet or wake, not {other:?}"),
    }
}
