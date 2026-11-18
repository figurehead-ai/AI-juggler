//! The signed remote configuration (D-128, D-129, ADR-0035).
//!
//! One signed envelope per channel, around named sections that feature
//! modules register with typed schemas. The file can only select among
//! behaviour already compiled into the app: switch a compiled feature off
//! (or, where the code says so, on), widen or narrow a version range within
//! compiled bounds, set a tunable within compiled bounds, or pick a
//! compiled option. The app is complete without it.
//!
//! - [`canonical`]: the canonical JSON bytes a signature covers.
//! - [`keys`]: the pinned ed25519 keys, their ids and roles.
//! - [`envelope`]: the signed file and the order of its checks.
//! - [`state`]: the rollback floor, revoked keys and the last good file,
//!   behind an injectable store.
//! - [`schema`]: the section registry and what a value may do.
//! - [`merge`](mod@merge): defaults, then public, then account, then the user.
//! - [`target`]: targeting and rollout buckets, evaluated locally.
//! - [`fetch`]: the daily anonymous fetch through an injected transport,
//!   the cache and every fallback.
//!
//! Must not know: any feature, tool, screen, Tauri or the UI. Sections,
//! their controls and the transport are handed in.
//!
//! # Security self-review (ADR-0035 §3)
//!
//! **Assets.** The app's behaviour selection (kill switches, compat ranges,
//! rollouts, tunables), and the user's anonymity on the fetch.
//!
//! **Trust anchor.** Two or three ed25519 public keys compiled into the
//! binary, each with an id and a role ([`keys`]). The private halves live
//! only in the release environment. Until the publish workflow's keys are
//! compiled in, [`keys::COMPILED_KEYS`] is empty and nothing verifies: the
//! fail-closed state is the compiled defaults.
//!
//! | Threat | Answer |
//! |---|---|
//! | **A tampered file** (CDN, TLS interception, cache on disk) | ed25519 `verify_strict` over a context string plus the exact payload bytes; a cached file is re-verified on every load. A single flipped byte anywhere in the payload or signature fails, and so does a small-order `R` that plain verification would take. Pinned keys must be prime-order, and no key's bytes may be pinned under two roles. |
//! | **A parser attack before verification** | Before the signature passes, only the size cap (256 KB) and the wrapper are read: a fixed shape of strings, unknown fields refused; the payload is opaque base64. It is parsed only after verification, and only if it is canonical, so duplicate keys, odd numbers or escapes can't give the signed bytes a second meaning. Depth is capped. |
//! | **A downgrade to unsigned** | There is no unsigned path: a file without a signed `config` part, with an unknown key id, or with an unknown `format`, is rejected, and the app keeps its last good file or its compiled defaults. TLS alone is never trusted. |
//! | **Rollback** (an older, validly signed file) | The highest serial accepted is kept per channel and key fingerprint; a lower serial is refused, and the same serial is accepted only with byte-identical content (no equivocation). The state lives in a durable, checksummed, atomically written file outside any cache ([`state`]); if it is missing or corrupt the app starts from the release's compiled floor and revocations, never from "no floor". Saves only move forward, and refreshes are serialised, so a race can't lower the floor. |
//! | **Replay** | Rollback covers replays after a newer file. `expires_at` (at most 31 days after `issued_at`) bounds how long any file can be replayed, judged against the trusted time `max(now, newest issued_at accepted)`, so a clock set back revives nothing; an `issued_at` more than a day past it is refused, so a file can't carry its validity window into the future. The day of skew is a trade-off: a slow or wrong clock isn't shut out, at the cost that a file may apply up to a day before its `issued_at`, and its window ends at most a day plus 31 days past the trusted time; signing one still takes a pinned key. The channel inside the signed envelope stops a beta file being served to stable; context strings stop a revocation signature passing as a configuration one. |
//! | **Key compromise** | Current or next stolen: **the real defence is a release without that key** (and with the key's fingerprint in the compiled revocations, which reach fresh installs through the signed update channel). In-band revocation helps only installs that fetch a file carrying the list: an attacker who holds the key and controls the CDN or the path simply never serves it, so every install it serves, and every fresh install, keeps trusting the key until the update. Where the list does arrive, the key's files (cached ones too) stop verifying and its serials stop holding the floor up. The file can still only select compiled behaviour, so a stolen key adds none. Emergency stolen: it signs no configuration, and a list may only name fingerprints of keys this build pins, so it can't pre-revoke a later release's keys; the worst it does is revoke current and next here, leaving the compiled defaults until an update. |
//! | **Rotation** | Publish with `next`; ship an app whose `current` is the old `next` with a fresh `next`; retire a key by shipping without it. A file may carry several revocation parts (one per emergency-key generation); a part this app can't verify is skipped, never fatal. A revocation only ever grows the revoked set. |
//! | **Remote code by stealth** | Section values are checked against compiled controls ([`schema`]); a kill switch can't be turned on, a range can't pass its bounds, a choice must be compiled in. What an app can't read is dropped one value at a time, and kills and compat narrowing always reach older apps. Only an `OffOnly` switch's kill beats the user; an `OnOff` or `Rollout` "off" is a preference. |
//! | **Tracking on the fetch** | A request type with no field for an id, cookie, account or query; a fixed URL per channel; targeting and buckets evaluated locally from a random id that is never sent ([`target::InstallId`]). `If-None-Match` is off by default (a unique entity tag would work as a cookie). |
//! | **Outages and bad state** | Missing, invalid, unsigned, expired or oversize files and network errors fall back to the last good file or the compiled defaults; a state that can't be saved serves nothing new (its floor wasn't kept), with the daily limit kept in memory; a corrupt state (read in full, failing its checksum or parse) fails safe to the compiled baseline; one that can't be read *now* fails the refresh closed (nothing fetched, saved or newly served; the next refresh reads again), and a save never overwrites a file it couldn't read. |
//!
//! **Out of scope.** An attacker who can write the app's own state file or
//! binary already controls the app. (Two app processes, or two stores on
//! one path, are serialised by an advisory lock beside the state file.)

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "tests assert by unwrapping"
    )
)]

pub mod canonical;
pub mod envelope;
mod error;
pub mod fetch;
pub mod keys;
pub mod merge;
pub mod schema;
pub mod state;
pub mod target;
pub mod version;

pub use envelope::{
    Channel, Checked, Envelope, MAX_FILE_BYTES, MAX_VALIDITY_SECS, SignedFile, SignedPart,
    Verified, VerifyContext, check_file,
};
pub use error::Rejection;
pub use fetch::{
    AppContext, Attempt, ConfigRequest, ConfigResponse, Endpoints, Failure, FetchPolicy, Fetcher,
    NetworkError, Recorder, RecordingTransport, Refresh, ReplayTransport, Transport,
};
pub use keys::{KeyRing, KeyRole, PinnedKey};
pub use merge::{Effective, Layer, Resolved, UserChoices, merge};
pub use schema::{
    Control, LayerValues, Registry, RemoteMode, SectionIssue, SectionSpec, Setting, SettingKey,
};
pub use state::{Baseline, FileStore, MemoryStore, StateStore, TrustState};
pub use target::{Facts, InstallId, Os};
pub use version::{Version, VersionRange};

// A build that pins keys must ship the floors of the newest files known at
// build time with them: without them, a missing or corrupt trust state
// would fall back to "no floor" and accept any old file still in its
// window. This fails the build, not a test run.
const _: () = assert!(
    keys::COMPILED_KEYS.is_empty() || !state::COMPILED_FLOORS.is_empty(),
    "keys::COMPILED_KEYS needs state::COMPILED_FLOORS (ADR-0035 §3)"
);

/// Lower-case hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}
