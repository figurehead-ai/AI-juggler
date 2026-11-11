#![no_main]
//! Fuzz target: the cloud backup container and the recovery code
//! (AJ-E1204, AJ-E2008). Opening any bytes with a key, or parsing any typed
//! recovery code, must never panic: both come from outside (a download,
//! the user's typing). Run: `cargo +nightly fuzz run cloud_container`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The input's first 32 bytes are the key, the rest the container: a
    // run reproduces from its input alone, no key is written here, and the
    // fuzzer can still learn a key's id and reach the decrypt step.
    let (head, container) = data.split_at(data.len().min(32));
    if !head.is_empty() {
        let key = juggler_account::vault::VaultKey::from_bytes(std::array::from_fn(|i| {
            head[i % head.len()]
        }));
        let _ = juggler_account::vault::open(&key, container);
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(parsed) = juggler_account::vault::VaultKey::from_recovery_code(text) {
            // A code that parses names a key that writes the same code back.
            let again = juggler_account::vault::VaultKey::from_recovery_code(parsed.recovery_code().as_str());
            assert_eq!(again.ok(), Some(parsed));
        }
    }
});
