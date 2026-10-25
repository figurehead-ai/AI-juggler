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

#[cfg(target_os = "macos")]
impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Option<String> {
        security_framework::passwords::get_generic_password(self.service, account)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        security_framework::passwords::set_generic_password(
            self.service,
            account,
            secret.as_bytes(),
        )
        .map_err(|_| SecretError::Store(self.what))
    }

    fn delete(&self, account: &str) {
        let _ = security_framework::passwords::delete_generic_password(self.service, account);
    }

    fn has(&self, account: &str) -> bool {
        // The item's attributes only: the secret itself is never read here.
        security_framework::item::ItemSearchOptions::new()
            .class(security_framework::item::ItemClass::generic_password())
            .service(self.service)
            .account(account)
            .load_attributes(true)
            .limit(1_i64)
            .search()
            .is_ok_and(|found| !found.is_empty())
    }
}

#[cfg(not(target_os = "macos"))]
impl SecretStore for KeychainStore {
    fn get(&self, _account: &str) -> Option<String> {
        None
    }

    fn set(&self, _account: &str, _secret: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported)
    }

    fn delete(&self, _account: &str) {}
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
