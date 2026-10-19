//! D-23: the editions of AI Juggler. Three can be installed side by side
//! on one Mac — like a browser and its preview build — and never share
//! anything:
//!
//! | Edition | App | Bundle id | Data folder | Updates from |
//! |---|---|---|---|---|
//! | Main | AI Juggler | `ai.figurehead.juggler` | `AI Juggler` | beta or stable |
//! | Nightly | AI Juggler Nightly | `….nightly` | `AI Juggler Nightly` | nightly |
//! | Dev | AI Juggler Dev | `….dev` | `AI Juggler Dev` | never (built locally) |
//!
//! The edition follows the bundle identifier the app was built with
//! (`tauri.conf.json`, or an edition overlay passed with `--config`). From
//! it come the data folder, the Keychain services, the Fleet port and the
//! single-instance lock (keyed by the identifier), so a second edition
//! can never open, change or lock the first one's data.
//!
//! A debug build is always Dev, whatever its identifier: `cargo tauri
//! dev` without the overlay must still never touch a real install.

use std::sync::OnceLock;

use juggler_account::{KeychainStore, VaultKeychain};

/// AI Juggler's (the main edition's) bundle identifier.
pub const MAIN_IDENTIFIER: &str = "ai.figurehead.juggler";

/// One edition of the app.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edition {
    /// AI Juggler: the one for daily use (beta or stable channel).
    Main,
    /// AI Juggler Nightly: built from `main` every night.
    Nightly,
    /// AI Juggler Dev: built locally by developers; never updates.
    Dev,
}

impl Edition {
    /// The edition a build with `identifier` is. Anything that isn't
    /// exactly the main or nightly identifier is Dev, so an unexpected
    /// build can only ever reach the Dev data folder.
    #[must_use]
    pub fn from_identifier(identifier: &str, debug_build: bool) -> Self {
        if debug_build {
            return Self::Dev;
        }
        match identifier.strip_prefix(MAIN_IDENTIFIER) {
            Some("") => Self::Main,
            Some(".nightly") => Self::Nightly,
            _ => Self::Dev,
        }
    }

    /// The wire name: `main` | `nightly` | `dev`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Nightly => "nightly",
            Self::Dev => "dev",
        }
    }

    /// The app's name, as in Finder and the Dock.
    #[must_use]
    pub fn app_name(self) -> &'static str {
        match self {
            Self::Main => "AI Juggler",
            Self::Nightly => "AI Juggler Nightly",
            Self::Dev => "AI Juggler Dev",
        }
    }

    /// The bundle identifier.
    #[must_use]
    pub fn identifier(self) -> &'static str {
        match self {
            Self::Main => MAIN_IDENTIFIER,
            Self::Nightly => "ai.figurehead.juggler.nightly",
            Self::Dev => "ai.figurehead.juggler.dev",
        }
    }

    /// The data folder's name under the OS data directory (Application
    /// Support on macOS). AI Juggler's is unchanged from before editions.
    #[must_use]
    pub fn data_dir_name(self) -> &'static str {
        self.app_name()
    }

    /// The Keychain item holding the Figurehead device token.
    #[must_use]
    pub fn keychain(self) -> KeychainStore {
        KeychainStore::for_service(self.identifier())
    }

    /// The Keychain item holding the vault key.
    #[must_use]
    pub fn vault_keychain(self) -> VaultKeychain {
        VaultKeychain::for_service(match self {
            Self::Main => juggler_account::vault::VAULT_SERVICE,
            Self::Nightly => "ai.figurehead.juggler.nightly.vault",
            Self::Dev => "ai.figurehead.juggler.dev.vault",
        })
    }

    /// The Fleet port this edition listens on first (any free port when
    /// it's taken).
    #[must_use]
    pub fn preferred_link_port(self) -> u16 {
        match self {
            Self::Main => 47_821,
            Self::Nightly => 47_822,
            Self::Dev => 47_823,
        }
    }
}

static CURRENT: OnceLock<Edition> = OnceLock::new();

/// Fixes the running edition (once, first thing at startup).
pub fn set_current(edition: Edition) {
    let _ = CURRENT.set(edition);
}

/// The running edition. Before [`set_current`] (unit and integration
/// tests, which use their own temporary data folders) it is Main, so the
/// tests see AI Juggler's names.
#[must_use]
pub fn current() -> Edition {
    CURRENT.get().copied().unwrap_or(Edition::Main)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Edition; 3] = [Edition::Main, Edition::Nightly, Edition::Dev];

    #[test]
    fn the_identifier_decides_the_edition_and_anything_unexpected_is_dev() {
        assert_eq!(
            Edition::from_identifier(MAIN_IDENTIFIER, false),
            Edition::Main
        );
        assert_eq!(
            Edition::from_identifier("ai.figurehead.juggler.nightly", false),
            Edition::Nightly
        );
        assert_eq!(
            Edition::from_identifier("ai.figurehead.juggler.dev", false),
            Edition::Dev
        );
        for other in [
            "com.vuxtra.ai-juggler",
            "ai.figurehead.jugglerx",
            "",
            "ai.figurehead.juggler.beta",
        ] {
            assert_eq!(
                Edition::from_identifier(other, false),
                Edition::Dev,
                "{other}"
            );
        }
    }

    #[test]
    fn a_debug_build_is_always_dev() {
        for identifier in [MAIN_IDENTIFIER, "ai.figurehead.juggler.nightly"] {
            assert_eq!(Edition::from_identifier(identifier, true), Edition::Dev);
        }
    }

    #[test]
    fn every_edition_identifies_as_itself() {
        for edition in ALL {
            assert_eq!(
                Edition::from_identifier(edition.identifier(), false),
                edition
            );
        }
    }

    #[test]
    fn editions_share_nothing() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a.identifier(), b.identifier());
                assert_ne!(a.data_dir_name(), b.data_dir_name());
                assert_ne!(a.keychain().service(), b.keychain().service());
                assert_ne!(a.vault_keychain().service(), b.vault_keychain().service());
                assert_ne!(a.preferred_link_port(), b.preferred_link_port());
            }
            assert_ne!(a.keychain().service(), a.vault_keychain().service());
        }
    }

    #[test]
    fn ai_juggler_keeps_its_existing_folder_keychain_items_and_port() {
        // Installs from before editions carry on with no migration.
        assert_eq!(Edition::Main.data_dir_name(), "AI Juggler");
        assert_eq!(
            Edition::Main.keychain().service(),
            juggler_account::secrets::KEYCHAIN_SERVICE
        );
        assert_eq!(
            Edition::Main.vault_keychain().service(),
            juggler_account::vault::VAULT_SERVICE
        );
        assert_eq!(Edition::Main.preferred_link_port(), 47_821);
    }
}
