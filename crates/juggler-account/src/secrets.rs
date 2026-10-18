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
}

/// The macOS Keychain (generic password items).
#[derive(Debug, Default, Clone, Copy)]
pub struct KeychainStore;

#[cfg(target_os = "macos")]
impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Option<String> {
        security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, account)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        security_framework::passwords::set_generic_password(
            KEYCHAIN_SERVICE,
            account,
            secret.as_bytes(),
        )
        .map_err(|_| SecretError::Store("save the device token"))
    }

    fn delete(&self, account: &str) {
        let _ = security_framework::passwords::delete_generic_password(KEYCHAIN_SERVICE, account);
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
