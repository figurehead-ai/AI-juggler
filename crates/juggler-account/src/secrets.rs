//! Where the device token lives: AI Juggler's OWN Keychain item
//! (service `ai.figurehead.juggler`, account = the device id) — never a
//! vendor's, never SQLite, settings, logs, packs or backups.

use std::collections::HashMap;
use std::sync::Mutex;

/// AI Juggler's Keychain service for the Figurehead device token.
pub const KEYCHAIN_SERVICE: &str = "ai.figurehead.juggler";

/// Secret storage failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    /// The platform store refused or failed.
    #[error("the Keychain couldn't {0}")]
    Store(&'static str),
    /// This platform has no supported secret store in this build.
    #[error("connecting an account needs the macOS Keychain")]
    Unsupported,
}

/// A store for one secret per account name.
pub trait SecretStore: Send + Sync {
    /// The secret for `account`, if any.
    fn get(&self, account: &str) -> Option<String>;
    /// Stores the secret for `account`.
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] when the store refuses.
    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError>;
    /// Deletes the secret for `account` (absent is fine).
    fn delete(&self, account: &str);
    /// Whether a secret for `account` is there. A store that can tell
    /// without reading the secret does (the Keychain reads only the item's
    /// attributes); the default reads it and drops it.
    fn has(&self, account: &str) -> bool {
        self.get(account).is_some()
    }
}

/// The macOS Keychain (generic password items) under one service. Each
/// edition of the app (D-23) has its own service, so AI Juggler Nightly
/// and AI Juggler Dev never read or replace AI Juggler's token.
#[derive(Debug, Clone, Copy)]
pub struct KeychainStore {
    service: &'static str,
    /// What a failed save couldn't do, in words.
    what: &'static str,
}

impl KeychainStore {
    /// The store for `service` (the edition's bundle identifier).
    #[must_use]
    pub const fn for_service(service: &'static str) -> Self {
        Self::for_item(service, "save the device token")
    }

    /// The store for `service`, whose failed save says it couldn't `what`
    /// (`save the key`).
    #[must_use]
    pub const fn for_item(service: &'static str, what: &'static str) -> Self {
        Self { service, what }
    }

    /// The Keychain service this store reads and writes.
    #[must_use]
    pub const fn service(&self) -> &'static str {
        self.service
    }
}

impl Default for KeychainStore {
    /// AI Juggler's own service ([`KEYCHAIN_SERVICE`]).
    fn default() -> Self {
        Self::for_service(KEYCHAIN_SERVICE)
    }
}

impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Option<String> {
        keychain::get(self.service, account)
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        keychain::set(self.service, account, secret, self.what)
    }

    fn delete(&self, account: &str) {
        keychain::delete(self.service, account);
    }

    fn has(&self, account: &str) -> bool {
        keychain::has(self.service, account)
    }
}

/// Where AI Juggler's own secrets go when the system keychain must not be
/// used: a private folder of files, one per item, readable by this user
/// only. Chosen once at start ([`use_files`]); see [`keychain`].
static FILE_STORE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// From now on every keychain item AI Juggler keeps (the device token,
/// the vault key, service keys, the Fleet identity) lives in owner-only
/// files under `dir` instead of the system keychain. The app chooses this
/// at start for a Dev build, a build run with a `HOME` that isn't the
/// user's (a lab, a replica, a test harness), or when no keychain is
/// reachable, so such a run never touches, or prompts for, the user's
/// keychain. Returns whether this call set it (a second call is ignored).
pub fn use_files(dir: std::path::PathBuf) -> bool {
    FILE_STORE.set(dir).is_ok()
}

/// Whether secrets go to files ([`use_files`]) rather than the keychain.
#[must_use]
pub fn in_files() -> bool {
    FILE_STORE.get().is_some()
}

/// The keychain calls every store shares, routed to [`FILE_STORE`] when
/// it's set. On the system keychain they never show the user anything: a
/// read skips any item that would ask, and nothing is written unless a
/// default keychain exists (else macOS offers to create or reset one).
mod keychain {
    use super::{FILE_STORE, SecretError};

    pub(super) fn get(service: &str, account: &str) -> Option<String> {
        if let Some(dir) = FILE_STORE.get() {
            return super::files::get(dir, service, account);
        }
        system::get(service, account)
    }

    pub(super) fn set(
        service: &str,
        account: &str,
        secret: &str,
        what: &'static str,
    ) -> Result<(), SecretError> {
        if let Some(dir) = FILE_STORE.get() {
            return super::files::set(dir, service, account, secret)
                .map_err(|_| SecretError::Store(what));
        }
        system::set(service, account, secret, what)
    }

    pub(super) fn delete(service: &str, account: &str) {
        if let Some(dir) = FILE_STORE.get() {
            super::files::delete(dir, service, account);
            return;
        }
        system::delete(service, account);
    }

    pub(super) fn has(service: &str, account: &str) -> bool {
        if let Some(dir) = FILE_STORE.get() {
            return super::files::get(dir, service, account).is_some();
        }
        system::has(service, account)
    }

    #[cfg(target_os = "macos")]
    pub(super) mod system {
        use super::SecretError;
        use security_framework::item::{ItemClass, ItemSearchOptions, SearchResult};
        use security_framework::os::macos::keychain::SecKeychain;

        /// One keychain call at a time, each with the keychain's user
        /// interaction turned off: a call that would show a dialog (an
        /// unlock, an access prompt, "a keychain cannot be found") fails
        /// instead. The setting is the process's, so the calls are
        /// serialised to keep one call's guard from turning it back on
        /// under another.
        static QUIET: std::sync::Mutex<()> = std::sync::Mutex::new(());

        fn quietly<T>(call: impl FnOnce() -> T) -> Option<T> {
            let _one = QUIET
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let _no_ui = SecKeychain::disable_user_interaction().ok()?;
            Some(call())
        }

        /// See [`super::super::keychain_reachable`].
        pub(in super::super) fn reachable() -> bool {
            quietly(|| SecKeychain::default().is_ok()).unwrap_or(false)
        }

        fn search(service: &str, account: &str) -> ItemSearchOptions {
            let mut options = ItemSearchOptions::new();
            options
                .class(ItemClass::generic_password())
                .service(service)
                .account(account)
                // Never a prompt: an item that would ask is left out.
                .skip_authenticated_items(true)
                .limit(1_i64);
            options
        }

        pub(super) fn get(service: &str, account: &str) -> Option<String> {
            let found = quietly(|| search(service, account).load_data(true).search())?.ok()?;
            found.into_iter().find_map(|result| match result {
                SearchResult::Data(bytes) => String::from_utf8(bytes).ok(),
                _ => None,
            })
        }

        pub(super) fn has(service: &str, account: &str) -> bool {
            // The item's attributes only: the secret itself isn't read.
            quietly(|| search(service, account).load_attributes(true).search())
                .is_some_and(|found| found.is_ok_and(|found| !found.is_empty()))
        }

        pub(super) fn set(
            service: &str,
            account: &str,
            secret: &str,
            what: &'static str,
        ) -> Result<(), SecretError> {
            // No default keychain: macOS would ask the user to create or
            // reset one. Refuse quietly instead.
            if !reachable() {
                return Err(SecretError::Store(what));
            }
            quietly(|| {
                security_framework::passwords::set_generic_password(
                    service,
                    account,
                    secret.as_bytes(),
                )
            })
            .and_then(Result::ok)
            .ok_or(SecretError::Store(what))
        }

        pub(super) fn delete(service: &str, account: &str) {
            if reachable() {
                let _ = quietly(|| {
                    security_framework::passwords::delete_generic_password(service, account)
                });
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    mod system {
        use super::SecretError;

        pub(super) fn get(_service: &str, _account: &str) -> Option<String> {
            None
        }

        pub(super) fn has(_service: &str, _account: &str) -> bool {
            false
        }

        pub(super) fn set(
            _service: &str,
            _account: &str,
            _secret: &str,
            _what: &'static str,
        ) -> Result<(), SecretError> {
            Err(SecretError::Unsupported)
        }

        pub(super) fn delete(_service: &str, _account: &str) {}
    }
}

/// Whether this user's default keychain exists and can be used without
/// asking: macOS answers this without any dialog. False when `HOME` points
/// at a folder with no keychain (a lab, a replica), where a write would
/// make macOS offer to create or reset one.
#[cfg(target_os = "macos")]
#[must_use]
pub fn keychain_reachable() -> bool {
    keychain::system::reachable()
}

/// No system keychain on this platform yet.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn keychain_reachable() -> bool {
    false
}

/// Owner-only files, one per item (the item's service and account name,
/// hashed, as the file name).
mod files {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    fn path(dir: &Path, service: &str, account: &str) -> PathBuf {
        use sha2::Digest as _;
        let digest = sha2::Sha256::digest(format!("{service}\0{account}").as_bytes());
        let name = digest.iter().fold(String::new(), |mut name, byte| {
            use std::fmt::Write as _;
            let _ = write!(name, "{byte:02x}");
            name
        });
        dir.join(name)
    }

    pub(super) fn get(dir: &Path, service: &str, account: &str) -> Option<String> {
        std::fs::read_to_string(path(dir, service, account)).ok()
    }

    pub(super) fn set(
        dir: &Path,
        service: &str,
        account: &str,
        secret: &str,
    ) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
        let target = path(dir, service, account);
        let staged = target.with_extension("new");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&staged)?;
        file.write_all(secret.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(staged, target)
    }

    pub(super) fn delete(dir: &Path, service: &str, account: &str) {
        let _ = std::fs::remove_file(path(dir, service, account));
    }

    #[cfg(test)]
    mod tests {
        #![allow(clippy::unwrap_used, reason = "a test")]

        #[test]
        fn an_item_round_trips_owner_only_and_apart_from_others() {
            let dir = tempfile::tempdir().unwrap();
            let store = dir.path().join("secrets");
            super::set(
                &store,
                "ai.figurehead.juggler.dev",
                "fleet/identity.key",
                "seed",
            )
            .unwrap();
            super::set(
                &store,
                "ai.figurehead.juggler.dev.vault",
                "fleet/identity.key",
                "other",
            )
            .unwrap();
            assert_eq!(
                super::get(&store, "ai.figurehead.juggler.dev", "fleet/identity.key").as_deref(),
                Some("seed")
            );
            assert_eq!(
                super::get(
                    &store,
                    "ai.figurehead.juggler.dev.vault",
                    "fleet/identity.key"
                )
                .as_deref(),
                Some("other")
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                assert_eq!(
                    std::fs::metadata(&store).unwrap().permissions().mode() & 0o777,
                    0o700
                );
                let file = super::path(&store, "ai.figurehead.juggler.dev", "fleet/identity.key");
                assert_eq!(
                    std::fs::metadata(file).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
            super::delete(&store, "ai.figurehead.juggler.dev", "fleet/identity.key");
            assert_eq!(
                super::get(&store, "ai.figurehead.juggler.dev", "fleet/identity.key"),
                None
            );
        }
    }
}

/// An in-memory store (tests; never touches the real Keychain).
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Option<String> {
        self.0.lock().ok().and_then(|map| map.get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        self.0
            .lock()
            .map_err(|_| SecretError::Store("save the device token"))?
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) {
        if let Ok(mut map) = self.0.lock() {
            map.remove(account);
        }
    }
}
