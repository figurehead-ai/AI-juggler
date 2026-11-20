//! Chat's memory, the app side (ADR-0037, AJ-E3819). The store, the log,
//! retrieval and the backup rule live in
//! `juggler_core::storage::memory`; the app hands them what the core must
//! not know: the D-14 secret detector (`juggler-handover`'s redaction
//! engine). Bot's remember flow writes through [`MemoryWriters::with_memory`]
//! (`features/bot/memory.rs`); the forget sheet shows [`FORGET_AND_BACKUPS`].

use std::collections::BTreeSet;

use std::sync::Arc;

use juggler_core::storage::memory::{
    Actor, Forgotten, KindWriter, MemoryError, SecretScan, Writers,
};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use juggler_handover::redaction;

/// The memory store's writers, taken once when the app's state is built
/// (the composition root), so no code later in the process can mint one
/// (the security review's M2): `Writers::issue` answers `None` from then
/// on. Each owning module takes its own writer from here, once, as it
/// starts; no module can take another's.
#[derive(Debug)]
pub struct MemoryWriters(std::sync::Mutex<Slots>);

/// Each owner's writer, until it takes it. The owners not built yet
/// (Chat's memory flow and history, the digest) add their `take_…` as
/// they land, and with it the reads this `expect` stands in for.
#[derive(Debug, Default)]
#[expect(
    dead_code,
    reason = "held for their owners, which take them as they are built (AJ-E3819, AJ-E3712)"
)]
struct Slots {
    memory: Option<KindWriter>,
    watches: Option<KindWriter>,
    chat: Option<KindWriter>,
    digest: Option<KindWriter>,
}

impl MemoryWriters {
    /// Takes the process's writers (each slot empty when another state
    /// took them first: a second state in a test process).
    #[must_use]
    pub fn taken_at_start() -> Self {
        let slots = issued().map_or_else(Slots::default, |writers| {
            let Writers {
                memory,
                watches,
                chat,
                digest,
            } = writers;
            Slots {
                memory: Some(memory),
                watches: Some(watches),
                chat: Some(chat),
                digest: Some(digest),
            }
        });
        Self(std::sync::Mutex::new(slots))
    }

    /// AJ-E3814: the watches' own writer, once (`features/watches` takes
    /// it on first use). The other owners' writers stay for them; each
    /// adds its own `take_…` as it is built (Chat's, the digest's, the
    /// memory flow's).
    pub fn take_watches(&self) -> Option<KindWriter> {
        self.0
            .lock()
            .ok()
            .and_then(|mut slots| slots.watches.take())
    }

    /// Bot's remember flow (AJ-E3819): the memory writer, lent for one
    /// write and kept here, its owner's module (`None` when another state
    /// in this process took the writers first).
    pub fn with_memory<T>(&self, write: impl FnOnce(&KindWriter) -> T) -> Option<T> {
        self.0
            .lock()
            .ok()
            .and_then(|slots| slots.memory.as_ref().map(write))
    }

    /// AJ-E3712: the digest's own writer, once (`features/digest` takes it
    /// on its first digest).
    pub fn take_digest(&self) -> Option<KindWriter> {
        self.0.lock().ok().and_then(|mut slots| slots.digest.take())
    }
}

/// The process's one issue of the writers. A unit test's every state gets
/// a fresh set (the process's issue went to the first state built), so a
/// test can run Bot's remember flow end to end; a build never does.
#[cfg(test)]
fn issued() -> Option<Writers> {
    Writers::issue().or_else(|| Some(Writers::for_tests()))
}

#[cfg(not(test))]
fn issued() -> Option<Writers> {
    Writers::issue()
}

/// A memory store's error as the app's: a refusal by its code, under
/// "memory".
pub(crate) fn memory_error(error: MemoryError) -> AppError {
    match error {
        MemoryError::Core(core) => AppError::from(core),
        MemoryError::Refused(refusal) => AppError::Validation {
            subject: "memory",
            message: refusal.code().to_owned(),
        },
        other => AppError::Validation {
            subject: "memory",
            message: other.to_string(),
        },
    }
}

/// The app's one way to forget a subject (step 4's surfaces call it): the
/// store's forget, then, in the same storage hold, the backup a verify
/// staged is dropped, so no copy made before the forget outlives it in the
/// data folder (the second re-check's L5). A restore asks to verify again.
///
/// # Errors
///
/// The store's refusal or storage failure; removing the staged backup.
pub fn forget(state: &Arc<AppState>, subject_key: &str, actor: &Actor) -> AppResult<Forgotten> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let now = juggler_core::model::Timestamp::now().millis();
    let forgotten = storage
        .memory()
        .forget(subject_key, actor, now)
        .map_err(memory_error)?;
    crate::backup::drop_staged(&storage, &state.data_dir)?;
    Ok(forgotten)
}

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
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

    use super::*;

    /// M2: once the app's state is built, no code can mint a writer.
    #[test]
    fn the_writers_are_taken_when_the_state_is_built() {
        let dir = tempfile::tempdir().unwrap();
        let _state = crate::state::AppState::init_with_registry(
            dir.path().to_path_buf(),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap();
        assert!(Writers::issue().is_none(), "a writer could still be minted");
    }

    /// L5 (second re-check): a backup verified before a forget, never
    /// restored, holds the words no more once the forget is done.
    #[test]
    fn a_forget_drops_the_backup_a_verify_staged() {
        const CANARY: &str = "qzxwvstagedcanary";
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().to_path_buf();
        let state = crate::test_support::bare_state(data.clone());
        {
            let storage = state.storage.lock().unwrap();
            storage
                .raw_conn()
                .execute(
                    "INSERT INTO memory_records (id, subject_key, subject_id, version, kind,
                         scope_kind, tier, priority, authority, untrusted, source_kind,
                         confidence, as_of_ms, text, created_ms)
                     VALUES ('r1', 'note:gone', 'sub-gone', 1, 'note', 'global', 'long', 1,
                             'user', 0, 'settings', 1, 1, ?1, 1)",
                    [format!("remember {CANARY}")],
                )
                .unwrap();
            // As a verify leaves it: the backup staged, its confirmation set.
            std::fs::create_dir_all(data.join("restore-staging/db")).unwrap();
            storage
                .snapshot_to(&data.join("restore-staging/db/juggler.sqlite"))
                .unwrap();
            storage
                .app_settings()
                .set("restore_token", "tok:x")
                .unwrap();
        }
        let gone = forget(&state, "note:gone", &Actor::User).unwrap();
        assert_eq!(gone.subjects, 1);
        assert!(
            !data.join("restore-staging").exists(),
            "the staged backup is still there"
        );
        let storage = state.storage.lock().unwrap();
        let token = storage.app_settings().get("restore_token").unwrap();
        assert!(
            token.unwrap_or_default().is_empty(),
            "the confirmation stands"
        );
        storage
            .raw_conn()
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
        drop(storage);
        let holding = crate::test_support::holding(&data, CANARY);
        assert!(holding.is_empty(), "these still hold it: {holding:?}");
    }

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
