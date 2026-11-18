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
//! - [`merge`]: defaults, then public, then account, then the user.
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
//! | **A tampered file** (CDN, TLS interception, cache on disk) | ed25519 `verify_strict` over a context string plus the exact payload bytes; a cached file is re-verified on every load. A single flipped byte anywhere in the payload or signature fails. |
//! | **A parser attack before verification** | Before the signature passes, only the size cap (256 KB) and a fixed wrapper of four strings are read; the payload is opaque base64. It is parsed only after verification, and only if it is canonical, so duplicate keys, odd numbers or escapes can't give the signed bytes a second meaning. Depth is capped. |
//! | **A downgrade to unsigned** | There is no unsigned path: a file without a signed `config` part, with an unknown key id, or with an unknown `format`, is rejected, and the app keeps its last good file or its compiled defaults. TLS alone is never trusted. |
//! | **Rollback** (an older, validly signed file) | The highest serial accepted is kept per channel and key; a lower serial is refused, and the same serial is accepted only with byte-identical content (no equivocation). |
//! | **Replay** | Rollback covers replays after a newer file; `expires_at` (at most 31 days after `issued_at`) bounds how long any file can be replayed at all, including to an install that never saw a newer one; the channel inside the signed envelope stops a beta file being served to stable; context strings stop a revocation signature passing as a configuration one. |
//! | **Key compromise** | Current or next stolen: the emergency key signs a revocation list; the revoked key's files (cached ones too) stop verifying, and its serials stop holding the floor up, so a burned huge serial doesn't lock publishing out. The file still can only select compiled behaviour, so a stolen key can't add any. Emergency stolen: it signs no configuration, so the worst it can do is revoke current and next, which leaves the app on its compiled defaults until an update ships new keys. |
//! | **Rotation** | Publish with `next`; ship an app whose `current` is the old `next` with a fresh `next`; retire a key by shipping without it. A revocation only ever grows the revoked set. |
//! | **Remote code by stealth** | Section values are checked against compiled controls ([`schema`]); a kill switch can't be turned on, a range can't pass its bounds, a choice must be compiled in. A section that breaks its schema is dropped alone. |
//! | **Tracking on the fetch** | A request type with no field for an id, cookie, account or query; a fixed URL per channel; targeting and buckets evaluated locally from a random id that is never sent ([`target::InstallId`]). `If-None-Match` is off by default (a unique entity tag would work as a cookie). |
//! | **Outages and bad state** | Missing, invalid, unsigned, expired or oversize files, network errors and a store that can't be written all fall back; a store that can't be *read* fails closed (no remote file is trusted without its rollback floor). |
//!
//! **Out of scope.** An attacker who can write the app's own state store or
//! binary already controls the app. A user's clock set far back revives
//! nothing below the serial floor, and one set forward only makes files
//! expire early (compiled defaults).

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
pub use state::{MemoryStore, StateStore, TrustState};
pub use target::{Facts, InstallId, Os};
pub use version::{Version, VersionRange};
