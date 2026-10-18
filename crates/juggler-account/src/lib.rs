//! The optional Figurehead account (D-18, ADR-0014).
//!
//! AI Juggler is complete without an account. When the user connects
//! one, this crate talks to the Figurehead API contract (device linking,
//! the account and its entitlements, this Mac's device record) — and
//! nothing else:
//!
//! - **Unlinked is silent.** Nothing here runs unless the app asks, and
//!   the app asks only after the user chose to connect.
//! - **The device token lives only in the Keychain** ([`secrets`]); it
//!   is never logged, never serialised, never written to settings.
//! - **Every request is explicit** ([`transport`]): the method, path,
//!   bearer and body are visible at one seam, so a recording transport
//!   can prove what was (and wasn't) sent.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod client;
pub mod model;
pub mod secrets;
pub mod transport;
pub mod vault;

pub use client::{AccountClient, ApiError};
pub use model::{
    AccountView, DeviceFields, Entitlement, LinkRequest, PHASE1_FEATURES, PollOutcome,
};
pub use secrets::{KeychainStore, MemoryStore, SecretStore};
pub use transport::{HttpRequest, HttpResponse, RecordingTransport, Transport, UreqTransport};
pub use vault::{VaultError, VaultKey, VaultKeychain};

/// The production service. Release builds always use it.
pub const PRODUCTION_URL: &str = "https://figurehead.ai";

/// The environment variable debug builds honour for a local server.
pub const URL_OVERRIDE_ENV: &str = "AI_JUGGLER_FIGUREHEAD_URL";

/// The service base URL: `AI_JUGGLER_FIGUREHEAD_URL` in debug builds
/// (for a local development server), otherwise always production.
#[must_use]
pub fn base_url() -> String {
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var(URL_OVERRIDE_ENV)
        && (url.starts_with("http://") || url.starts_with("https://"))
    {
        return url.trim_end_matches('/').to_owned();
    }
    PRODUCTION_URL.to_owned()
}
