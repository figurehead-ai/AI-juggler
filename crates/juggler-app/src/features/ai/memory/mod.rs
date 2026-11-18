//! Chat's memory, the app side (ADR-0037, AJ-E3819). The store, the log,
//! retrieval and the backup rule live in
//! `juggler_core::storage::memory`; the app hands them what the core must
//! not know: the D-14 secret detector (`juggler-handover`'s redaction
//! engine). Step 1 has no command and no UI.

use std::collections::BTreeSet;

use juggler_core::storage::memory::SecretScan;
use juggler_handover::redaction;

/// What the backup page and the forget sheet say about backups (ADR-0037
/// §1.8; shown from step 4). A backup file the user exported is theirs:
/// AI Juggler can't reach it once saved, and it doesn't rotate, so the
/// words say so plainly. AI Juggler's own copies keep no memory at all.
pub const FORGET_AND_BACKUPS: &str = "Forgotten memories are left out of every backup made after \
you forget them. A backup file you exported before still holds them until you delete that file: \
AI Juggler can't reach a file once it's saved. AI Juggler's own copies, kept before an update or \
a restore, hold no memories at all.";

/// The D-14 secret-hiding pass for the memory store's write gate and the
/// log's sidecars: the same engine that hides secrets in packs and backups.
#[derive(Clone, Copy, Debug, Default)]
pub struct RedactionSecrets;

impl SecretScan for RedactionSecrets {
    fn finds_secret(&self, text: &str) -> bool {
        !redaction::list_segments(&[text]).is_empty()
    }

    fn mask(&self, text: &str) -> String {
        let (mut texts, _) = redaction::strip_segments(&[text.to_owned()], true, &BTreeSet::new());
        texts.pop().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backup_words_say_what_forget_cant_reach() {
        assert!(FORGET_AND_BACKUPS.contains("exported before still holds them"));
        assert!(FORGET_AND_BACKUPS.contains("hold no memories at all"));
    }

    #[test]
    fn the_gate_sees_what_the_backup_check_sees() {
        let key = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let said = format!("my key is {key}");
        assert!(RedactionSecrets.finds_secret(&said));
        assert!(!RedactionSecrets.mask(&said).contains(key));
        assert!(!RedactionSecrets.finds_secret("the api project matters most"));
        assert_eq!(
            RedactionSecrets.mask("the api project matters most"),
            "the api project matters most"
        );
    }
}
