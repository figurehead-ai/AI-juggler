//! The signed file and its checks (§1, §3).
//!
//! # The file
//!
//! ```text
//! {
//!   "format": 1,
//!   "config":      { "key_id": "…", "payload": "<base64>", "signature": "<base64>" },
//!   "revocations": { "key_id": "…", "payload": "<base64>", "signature": "<base64>" }   (optional)
//! }
//! ```
//!
//! Each `payload` is the canonical JSON ([`crate::canonical`]) of a signed
//! document, carried as opaque bytes. Each `signature` is ed25519 over
//! [`signing_message`]: a context string naming the document's kind, then
//! the payload bytes, so a signature for one kind can never be replayed as
//! the other.
//!
//! The configuration payload is the envelope: `format`, `key_id`,
//! `channel`, `serial`, `issued_at`, `expires_at` (Unix seconds, UTC),
//! optional `min_app`/`max_app`, and `sections`. The revocation payload is
//! `format`, `key_id`, `issued_at` and `revoked` (key ids).
//!
//! # The order of the checks
//!
//! 1. The size cap, then the wrapper: a small fixed shape of strings.
//! 2. The revocation part, if present: signature first (emergency key
//!    only), then its canonical payload. Its ids join the revoked set.
//! 3. The configuration part's signature, by an unrevoked current or next
//!    key. **Nothing in the payload is parsed before this passes.**
//! 4. Only then: the payload must be canonical JSON, and an envelope whose
//!    `key_id` is the key that verified it.
//! 5. Channel, validity window, expiry, serial (rollback, equivocation),
//!    and the app-version range.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::Rejection;
use crate::canonical::parse_canonical;
use crate::keys::{KeyRing, Purpose, valid_id};
use crate::state::{AcceptedFile, TrustState};
use crate::version::Version;

/// The largest file accepted, in bytes (§7).
pub const MAX_FILE_BYTES: usize = 256 * 1024;

/// The longest validity window a file may claim (`expires_at − issued_at`).
/// The publisher uses about 14 days (§3); this bounds a mistake or a stolen
/// key's reach.
pub const MAX_VALIDITY_SECS: u64 = 31 * 24 * 60 * 60;

/// The wrapper and envelope format this app reads.
pub const FORMAT: u64 = 1;

/// The context a configuration signature covers before the payload.
pub const CONFIG_CONTEXT: &[u8] = b"AIJuggler remote-config envelope v1\0";

/// The context a revocation signature covers before the payload.
pub const REVOCATIONS_CONTEXT: &[u8] = b"AIJuggler remote-config revocations v1\0";

/// The bytes a signature covers: `context` then `payload`.
#[must_use]
pub fn signing_message(context: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(context.len() + payload.len());
    message.extend_from_slice(context);
    message.extend_from_slice(payload);
    message
}

/// A release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Nightly builds.
    Nightly,
    /// Beta builds.
    Beta,
    /// Stable builds.
    Stable,
}

impl Channel {
    /// The channel's name in files and URLs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nightly => "nightly",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One signed part of the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedPart {
    /// The signing key's id.
    pub key_id: String,
    /// The canonical payload, base64.
    pub payload: String,
    /// The ed25519 signature over [`signing_message`], base64.
    pub signature: String,
}

/// The file as served.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedFile {
    /// [`FORMAT`].
    pub format: u64,
    /// The signed envelope. A file without it is an unsigned file.
    #[serde(default)]
    pub config: Option<SignedPart>,
    /// A signed revocation list, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocations: Option<SignedPart>,
}

/// The verified envelope.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Envelope {
    /// [`FORMAT`].
    pub format: u64,
    /// The key that signed it.
    pub key_id: String,
    /// The channel it is for.
    pub channel: String,
    /// Strictly increasing per channel.
    pub serial: u64,
    /// When it was signed, Unix seconds.
    pub issued_at: u64,
    /// When it stops being valid, Unix seconds.
    pub expires_at: u64,
    /// The lowest app version it applies to.
    #[serde(default)]
    pub min_app: Option<Version>,
    /// The first app version it no longer applies to.
    #[serde(default)]
    pub max_app: Option<Version>,
    /// The sections, by name, still raw: [`crate::Registry`] validates the
    /// ones it knows and ignores the rest.
    #[serde(default)]
    pub sections: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Revocations {
    format: u64,
    key_id: String,
    revoked: Vec<String>,
}

/// What the checks need besides the file.
#[derive(Debug, Clone, Copy)]
pub struct VerifyContext<'a> {
    /// The pinned keys.
    pub keys: &'a KeyRing,
    /// This app's channel.
    pub channel: Channel,
    /// This app's version.
    pub app_version: &'a Version,
    /// Now, Unix seconds.
    pub now: u64,
}

/// A file that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The envelope.
    pub envelope: Envelope,
    /// The SHA-256 of the signed payload, hex.
    pub digest: String,
}

/// The outcome of checking one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Key ids a verified revocation list in the file revoked that weren't
    /// revoked before. Kept even when the configuration part is rejected.
    pub newly_revoked: BTreeSet<String>,
    /// The verified envelope, or why the file was rejected.
    pub result: Result<Verified, Rejection>,
}

/// Runs every check of the module docs over `file`.
#[must_use]
pub fn check_file(file: &[u8], ctx: &VerifyContext<'_>, state: &TrustState) -> Checked {
    let mut newly_revoked = BTreeSet::new();
    let result = check(file, ctx, state, &mut newly_revoked);
    Checked {
        newly_revoked,
        result,
    }
}

fn check(
    file: &[u8],
    ctx: &VerifyContext<'_>,
    state: &TrustState,
    newly_revoked: &mut BTreeSet<String>,
) -> Result<Verified, Rejection> {
    if file.len() > MAX_FILE_BYTES {
        return Err(Rejection::TooLarge(file.len()));
    }
    let wrapper: SignedFile =
        serde_json::from_slice(file).map_err(|_| Rejection::Malformed("not a signed file"))?;
    if wrapper.format != FORMAT {
        return Err(Rejection::UnsupportedFormat(wrapper.format));
    }
    let mut revoked = state.revoked.clone();
    if let Some(part) = &wrapper.revocations {
        for id in check_revocations(part, ctx.keys, &revoked)? {
            if revoked.insert(id.clone()) {
                newly_revoked.insert(id);
            }
        }
    }
    let part = wrapper
        .config
        .as_ref()
        .ok_or(Rejection::Malformed("no signed configuration"))?;
    let payload = verify_part(part, Purpose::Config, CONFIG_CONTEXT, ctx.keys, &revoked)?;

    // Verified: only now is the payload parsed.
    let value = parse_canonical(&payload).map_err(Rejection::NotCanonical)?;
    let envelope: Envelope =
        serde_json::from_value(value).map_err(|e| Rejection::BadEnvelope(e.to_string()))?;
    if envelope.format != FORMAT {
        return Err(Rejection::UnsupportedFormat(envelope.format));
    }
    if envelope.key_id != part.key_id {
        return Err(Rejection::KeyIdMismatch);
    }
    if envelope.channel != ctx.channel.as_str() {
        return Err(Rejection::WrongChannel {
            expected: ctx.channel.as_str(),
            found: envelope.channel,
        });
    }
    if envelope.expires_at <= envelope.issued_at
        || envelope.expires_at - envelope.issued_at > MAX_VALIDITY_SECS
    {
        return Err(Rejection::BadValidity);
    }
    if ctx.now >= envelope.expires_at {
        return Err(Rejection::Expired {
            expires_at: envelope.expires_at,
            now: ctx.now,
        });
    }
    let digest = hex(&Sha256::digest(&payload));
    let mut view = state.clone();
    view.revoked = revoked;
    if let Some(floor) = view.floor(ctx.channel) {
        if envelope.serial < floor {
            return Err(Rejection::Rollback {
                serial: envelope.serial,
                floor,
            });
        }
        if envelope.serial == floor
            && view.accepted_digest(ctx.channel, floor) != Some(digest.as_str())
        {
            return Err(Rejection::Equivocation(floor));
        }
    }
    let applies = envelope
        .min_app
        .as_ref()
        .is_none_or(|min| ctx.app_version >= min)
        && envelope
            .max_app
            .as_ref()
            .is_none_or(|max| ctx.app_version < max);
    if !applies {
        return Err(Rejection::NotForThisApp);
    }
    Ok(Verified { envelope, digest })
}

fn check_revocations(
    part: &SignedPart,
    keys: &KeyRing,
    revoked: &BTreeSet<String>,
) -> Result<Vec<String>, Rejection> {
    let payload = verify_part(
        part,
        Purpose::Revocations,
        REVOCATIONS_CONTEXT,
        keys,
        revoked,
    )?;
    let value = parse_canonical(&payload).map_err(Rejection::NotCanonical)?;
    let list: Revocations =
        serde_json::from_value(value).map_err(|e| Rejection::BadEnvelope(e.to_string()))?;
    if list.format != FORMAT {
        return Err(Rejection::UnsupportedFormat(list.format));
    }
    if list.key_id != part.key_id {
        return Err(Rejection::KeyIdMismatch);
    }
    if list.revoked.len() > 64 || !list.revoked.iter().all(|id| valid_id(id)) {
        return Err(Rejection::BadEnvelope("revoked ids".to_owned()));
    }
    Ok(list.revoked)
}

/// Decodes one part and verifies its signature; returns the payload bytes,
/// unparsed.
fn verify_part(
    part: &SignedPart,
    purpose: Purpose,
    context: &[u8],
    keys: &KeyRing,
    revoked: &BTreeSet<String>,
) -> Result<Vec<u8>, Rejection> {
    if !valid_id(&part.key_id) {
        return Err(Rejection::Malformed("key id"));
    }
    let payload = BASE64
        .decode(&part.payload)
        .map_err(|_| Rejection::Malformed("payload is not base64"))?;
    let signature: [u8; 64] = BASE64
        .decode(&part.signature)
        .map_err(|_| Rejection::Malformed("signature is not base64"))?
        .try_into()
        .map_err(|_| Rejection::Malformed("signature is not 64 bytes"))?;
    keys.verify(
        &part.key_id,
        purpose,
        &signing_message(context, &payload),
        &signature,
        revoked,
    )?;
    Ok(payload)
}

/// Records an accepted file: its serial against its key, and the file
/// itself as the last good one.
///
/// # Errors
///
/// [`Rejection::Malformed`] when `file` isn't UTF-8 (it was parsed as JSON,
/// so it is).
pub fn record_accepted(
    state: &mut TrustState,
    channel: Channel,
    verified: &Verified,
    file: &[u8],
) -> Result<(), Rejection> {
    let text = std::str::from_utf8(file).map_err(|_| Rejection::Malformed("not UTF-8"))?;
    let slot = state.channels.entry(channel).or_default();
    let serial = verified.envelope.serial;
    let held = slot
        .serials
        .entry(verified.envelope.key_id.clone())
        .or_insert(serial);
    *held = (*held).max(serial);
    slot.accepted = Some(AcceptedFile {
        serial,
        digest: verified.digest.clone(),
        file: text.to_owned(),
    });
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}
