//! AJ-E1204 integration — the Cloud backup container end to end: a real
//! AJ-E1009 backup is sealed (zstd tar inside XChaCha20-Poly1305) and
//! opened back into a `.ajbackup` that AJ-E1009 verifies; nothing from
//! the backup is readable in the container; a wrong recovery code or a
//! changed byte is refused by name; the recovery code is shown until it
//! is confirmed saved, then never again.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_account::{MemoryStore, VaultKey};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::{backup, vault};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap())
}

const CANARY: &str = "storefront-canary-project-name-7f3a";

#[test]
fn a_real_backup_seals_and_opens_into_a_verifiable_ajbackup() {
    let work = tempfile::tempdir().unwrap();
    let mac = work.path().join("mac-a");
    let state = app_state(&mac);
    state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(Path::new(&format!("/work/{CANARY}")), None)
        .unwrap();
    std::fs::write(mac.join("heuristics.json"), r#"{"stuck_no_output_secs":300,"stuck_cpu_max_pct":5.0,"stuck_transcript_stale_secs":300,"waiting_prompt_secs":120}"#).unwrap();
    let local = work.path().join("local.ajbackup");
    let exported = backup::backup_export_impl(&state, &local, &mut |_, _| {}).unwrap();

    let store = MemoryStore::default();
    let code = vault::vault_setup_impl(&state, &store).unwrap().code;
    let key = VaultKey::from_recovery_code(&code).unwrap();
    let started = std::time::Instant::now();
    let container = vault::seal_backup(&local, &key).unwrap();
    eprintln!(
        "[perf] seal {} bytes: {:?}",
        container.len(),
        started.elapsed()
    );
    assert_eq!(&container[..4], b"FHB1");
    assert!(
        !container
            .windows(CANARY.len())
            .any(|w| w == CANARY.as_bytes()),
        "nothing from the backup is readable in the container"
    );

    // A second Mac with only the recovery code.
    let mac_b = work.path().join("mac-b");
    let b = app_state(&mac_b);
    let b_store = MemoryStore::default();
    vault::vault_enter_code_impl(&b, &b_store, &code.to_lowercase()).unwrap();
    let b_key = VaultKey::load(&b_store).unwrap();
    let rebuilt = work.path().join("from-cloud.ajbackup");
    vault::open_backup(&container, &b_key, &rebuilt).unwrap();
    let plan = backup::backup_verify_impl(&b, &rebuilt).unwrap();
    assert_eq!(
        plan.manifest.counts, exported.manifest.counts,
        "every checksum verified, same counts"
    );
    let restored = backup::backup_restore_impl(&b, &rebuilt, &plan.token).unwrap();
    assert_eq!(restored.counts, exported.manifest.counts);

    // Wrong code and a changed byte are refused by name.
    let other = VaultKey::generate().unwrap();
    let refused = vault::open_backup(&container, &other, &work.path().join("x.ajbackup"))
        .unwrap_err()
        .to_string();
    assert!(refused.contains("different recovery code"), "{refused}");
    let mut altered = container.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    let refused = vault::open_backup(&altered, &key, &work.path().join("y.ajbackup"))
        .unwrap_err()
        .to_string();
    assert!(refused.contains("damaged or was altered"), "{refused}");
    let typo = code.replacen(&code[..1], if &code[..1] == "A" { "B" } else { "A" }, 1);
    let refused = vault::vault_enter_code_impl(&b, &MemoryStore::default(), &typo)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("typo"), "{refused}");
}

#[test]
fn the_recovery_code_is_shown_until_confirmed_then_never_again() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let store = MemoryStore::default();
    assert!(!vault::vault_status_impl(&state, &store).has_key);
    assert!(
        vault::vault_confirm_saved_impl(&state, &store).is_err(),
        "nothing to confirm yet"
    );
    let first = vault::vault_setup_impl(&state, &store).unwrap().code;
    // Closing the sheet before confirming shows the same code again.
    assert_eq!(
        vault::vault_setup_impl(&state, &store).unwrap().code,
        first,
        "same key, not a new one"
    );
    let file = work.path().join("code.txt");
    vault::vault_save_code_impl(&state, &store, &file).unwrap();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains(&first) && saved.contains("We can't recover it for you"));
    let status = vault::vault_confirm_saved_impl(&state, &store).unwrap();
    assert!(status.has_key && status.code_saved);
    let refused = vault::vault_setup_impl(&state, &store)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("isn't shown again"), "{refused}");
    assert!(vault::vault_save_code_impl(&state, &store, &work.path().join("again.txt")).is_err());
}
