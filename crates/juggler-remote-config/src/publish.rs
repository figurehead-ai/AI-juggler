//! The publisher's side (ADR-0035 §8): what the `Publish config`
//! workflow does, step by step, through `remote-config-publish`.
//!
//! 1. **Validate** a human-edited, unsigned source file
//!    (`config/remote/<channel>.json`, a [`Source`]) against the section
//!    schemas the app's build exports ([`crate::Registry::export`]),
//!    strictly ([`crate::Registry::validate`]).
//! 2. **Canonicalise and stamp** it: the next serial, `issued_at`, and an
//!    `expires_at` ([`Stamp`]).
//! 3. **Sign** it with a key this build pins as `current` or `next`
//!    ([`sign`]); a key it doesn't pin, or the emergency key, is refused.
//!    The live file's revocation parts are carried over unchanged (§3: the
//!    publisher keeps the list in every file).
//! 4. **Verify** the signed file exactly as the app does ([`verify`]),
//!    with this build's pinned keys and compiled baseline.
//!
//! [`inspect`] reads a live file's serial and expiry, its signature
//! checked but not its expiry, for the next serial and the expiry alert.
//!
//! [`resign`] is the unattended weekly re-sign: the live file's own
//! content, with only a new serial and dates, held to that by
//! [`check_resign`] (canonical content byte-identical).
//!
//! The private key is handed in from the release environment's secret and
//! never written anywhere; [`signing_key`] reads its 32-byte seed.

use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signer as _, SigningKey};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::Rejection;
use crate::canonical::{MAX_SAFE_INTEGER, to_canonical_bytes};
use crate::envelope::{
    CONFIG_CONTEXT, Channel, FORMAT, MAX_FILE_BYTES, MAX_REVOCATION_PARTS, MAX_VALIDITY_SECS,
    SignedFile, SignedPart, Verified, VerifyContext, check_file, signing_message,
};
use crate::keys::{KeyRing, KeyRole, fingerprint};
use crate::schema::Registry;
use crate::state::{Baseline, TrustState};
use crate::version::Version;

/// The validity the publisher uses (§3: about 14 days).
pub const DEFAULT_VALIDITY_DAYS: u64 = 14;

/// A day in seconds.
pub const DAY_SECS: u64 = 24 * 60 * 60;

/// The unsigned source of one channel's file, as people edit it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The channel it is for.
    pub channel: Channel,
    /// The lowest app version it applies to, if any.
    #[serde(default)]
    pub min_app: Option<Version>,
    /// The first app version it no longer applies to, if any.
    #[serde(default)]
    pub max_app: Option<Version>,
    /// The sections, by name.
    #[serde(default)]
    pub sections: BTreeMap<String, Value>,
}

/// Why a step of publishing failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PublishError {
    /// The source or schema file can't be read.
    #[error("{0}")]
    Read(String),
    /// The source breaks the schema; every problem.
    #[error("the source doesn't fit the schema:\n  {}", .0.join("\n  "))]
    Invalid(Vec<String>),
    /// The signing key is unusable or not pinned for configuration.
    #[error("signing key: {0}")]
    Key(String),
    /// The serial or dates are out of range.
    #[error("stamp: {0}")]
    Stamp(String),
    /// The signed file is over the size the app accepts.
    #[error("the signed file is {0} bytes; the app accepts at most {MAX_FILE_BYTES}")]
    TooLarge(usize),
    /// The file failed the app's own checks.
    #[error("the app would reject it: {0}")]
    Rejected(Rejection),
}

/// Reads a source file.
///
/// # Errors
///
/// [`PublishError::Read`] for anything but a [`Source`].
pub fn read_source(bytes: &[u8]) -> Result<Source, PublishError> {
    serde_json::from_slice(bytes).map_err(|e| PublishError::Read(format!("source: {e}")))
}

/// Reads an exported schema ([`Registry::export`]).
///
/// # Errors
///
/// [`PublishError::Read`] for a schema that can't be read or registered.
pub fn read_schema(bytes: &[u8]) -> Result<Registry, PublishError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| PublishError::Read(format!("schema: {e}")))?;
    Registry::import(&value).map_err(|e| PublishError::Read(format!("schema: {e}")))
}

/// Checks `source` for `channel` against `registry`, strictly.
///
/// # Errors
///
/// [`PublishError::Invalid`] with every problem.
pub fn check_source(
    source: &Source,
    registry: &Registry,
    channel: Channel,
) -> Result<(), PublishError> {
    let mut problems = Vec::new();
    if source.channel != channel {
        problems.push(format!(
            "the source is for {}, not {channel}",
            source.channel
        ));
    }
    if let (Some(min), Some(max)) = (&source.min_app, &source.max_app)
        && min >= max
    {
        problems.push("min_app must be below max_app".to_owned());
    }
    problems.extend(registry.validate(&source.sections));
    if problems.is_empty() {
        Ok(())
    } else {
        Err(PublishError::Invalid(problems))
    }
}

/// A file's serial and validity window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    /// The serial (strictly increasing per channel).
    pub serial: u64,
    /// When it is signed, Unix seconds.
    pub issued_at: u64,
    /// When it stops being valid, Unix seconds.
    pub expires_at: u64,
}

impl Stamp {
    /// `serial`, issued `now`, valid for `days`.
    ///
    /// # Errors
    ///
    /// [`PublishError::Stamp`] for serial 0 or past 2⁵³ − 1, no validity,
    /// or a validity longer than the app accepts.
    pub fn new(serial: u64, now: u64, days: u64) -> Result<Self, PublishError> {
        let max_serial = MAX_SAFE_INTEGER.unsigned_abs();
        if serial == 0 || serial > max_serial {
            return Err(PublishError::Stamp(format!(
                "serial {serial} is not 1 to {max_serial}"
            )));
        }
        let validity = days.saturating_mul(DAY_SECS);
        if validity == 0 || validity > MAX_VALIDITY_SECS {
            return Err(PublishError::Stamp(format!(
                "{days} days is not 1 to {}",
                MAX_VALIDITY_SECS / DAY_SECS
            )));
        }
        let expires_at = now
            .checked_add(validity)
            .filter(|at| *at <= max_serial)
            .ok_or_else(|| PublishError::Stamp("the dates are out of range".to_owned()))?;
        Ok(Self {
            serial,
            issued_at: now,
            expires_at,
        })
    }
}

/// The signing key from its secret: the 32-byte ed25519 seed, base64.
///
/// # Errors
///
/// [`PublishError::Key`] for anything else (the secret is never echoed).
pub fn signing_key(secret: &str) -> Result<SigningKey, PublishError> {
    let mut seed = [0u8; 32];
    let decoded = BASE64.decode_slice(secret.trim(), &mut seed);
    let key = match decoded {
        Ok(32) => Ok(SigningKey::from_bytes(&seed)),
        _ => Err(PublishError::Key(
            "not the base64 of a 32-byte ed25519 seed".to_owned(),
        )),
    };
    seed.fill(0);
    std::hint::black_box(&seed);
    key
}

/// The serial after `live` (the verified live file, if any), never at or
/// below this build's compiled floor for `channel`.
#[must_use]
pub fn next_serial(live: Option<&Verified>, channel: Channel) -> u64 {
    let floor = Baseline::compiled()
        .floors
        .get(&channel)
        .copied()
        .unwrap_or(0);
    live.map_or(0, |file| file.envelope.serial).max(floor) + 1
}

/// Signs `source` with `key`, stamped by `stamp`, carrying `revocations`
/// (the live file's, unchanged; at most as many as the app reads).
///
/// The key must be pinned in `ring` as `current` or `next`: its id is
/// taken from there.
///
/// # Errors
///
/// [`PublishError::Key`] for a key `ring` doesn't pin for configuration,
/// [`PublishError::Read`] for a source that has no canonical form, and
/// [`PublishError::TooLarge`] for a file past the app's cap.
pub fn sign(
    source: &Source,
    stamp: &Stamp,
    key: &SigningKey,
    ring: &KeyRing,
    revocations: &[SignedPart],
) -> Result<Vec<u8>, PublishError> {
    let print = fingerprint(&key.verifying_key().to_bytes());
    let pinned = ring
        .keys()
        .iter()
        .find(|pinned| pinned.fingerprint() == print)
        .ok_or_else(|| {
            PublishError::Key(format!(
                "its public key (fingerprint {print}) isn't pinned in this build"
            ))
        })?;
    if !matches!(pinned.role(), KeyRole::Current | KeyRole::Next) {
        return Err(PublishError::Key(format!(
            "{} is the emergency key, which signs only revocations",
            pinned.id()
        )));
    }
    let mut envelope = json!({
        "format": FORMAT,
        "key_id": pinned.id(),
        "channel": source.channel.as_str(),
        "serial": stamp.serial,
        "issued_at": stamp.issued_at,
        "expires_at": stamp.expires_at,
        "sections": source.sections,
    });
    if let Some(object) = envelope.as_object_mut() {
        if let Some(min) = &source.min_app {
            object.insert("min_app".to_owned(), json!(min.to_string()));
        }
        if let Some(max) = &source.max_app {
            object.insert("max_app".to_owned(), json!(max.to_string()));
        }
    }
    let payload =
        to_canonical_bytes(&envelope).map_err(|e| PublishError::Read(format!("source: {e}")))?;
    let signature = key.sign(&signing_message(CONFIG_CONTEXT, &payload));
    let file = SignedFile {
        format: FORMAT,
        config: Some(SignedPart {
            key_id: pinned.id().to_owned(),
            payload: BASE64.encode(&payload),
            signature: BASE64.encode(signature.to_bytes()),
        }),
        revocations: revocations
            .iter()
            .take(MAX_REVOCATION_PARTS)
            .cloned()
            .collect(),
    };
    let bytes = serde_json::to_vec(&file).map_err(|e| PublishError::Read(e.to_string()))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(PublishError::TooLarge(bytes.len()));
    }
    Ok(bytes)
}

/// The revocation parts of a signed file (to carry into the next one).
///
/// # Errors
///
/// [`PublishError::Read`] for a file that isn't a signed file.
pub fn revocations_of(file: &[u8]) -> Result<Vec<SignedPart>, PublishError> {
    let wrapper: SignedFile = serde_json::from_slice(file)
        .map_err(|e| PublishError::Read(format!("signed file: {e}")))?;
    Ok(wrapper.revocations)
}

/// Checks `file` exactly as an app of `app_version` on `channel` would at
/// `now`, with `ring` and this build's compiled baseline, as on a first
/// run.
///
/// # Errors
///
/// [`PublishError::Rejected`] with the app's reason.
pub fn verify(
    file: &[u8],
    ring: &KeyRing,
    channel: Channel,
    app_version: &Version,
    now: u64,
) -> Result<Verified, PublishError> {
    let mut state = TrustState::default();
    state.apply_baseline(&Baseline::compiled());
    let ctx = VerifyContext {
        keys: ring,
        channel,
        app_version,
        now,
    };
    check_file(file, &ctx, &state)
        .result
        .map_err(PublishError::Rejected)
}

/// A live file's verified envelope, its expiry aside: every other check
/// of [`verify`] runs, at the moment it was issued, for an app it applies
/// to. For the next serial and the expiry alert.
///
/// # Errors
///
/// [`PublishError::Read`] for a file that isn't a signed file, and
/// [`PublishError::Rejected`] for one that fails.
pub fn inspect(file: &[u8], ring: &KeyRing, channel: Channel) -> Result<Verified, PublishError> {
    // Read unverified only to choose the moment and the app version to
    // check at; the check itself then verifies these same bytes.
    #[derive(Deserialize)]
    struct When {
        issued_at: u64,
        #[serde(default)]
        min_app: Option<Version>,
    }
    let wrapper: SignedFile = serde_json::from_slice(file)
        .map_err(|e| PublishError::Read(format!("signed file: {e}")))?;
    let payload = wrapper
        .config
        .as_ref()
        .and_then(|part| BASE64.decode(&part.payload).ok())
        .ok_or_else(|| PublishError::Read("no signed configuration".to_owned()))?;
    let when: When = serde_json::from_slice(&payload)
        .map_err(|e| PublishError::Read(format!("envelope: {e}")))?;
    let app = when.min_app.unwrap_or_else(|| Version::new(0, 0, 0));
    verify(file, ring, channel, &app, when.issued_at)
}

/// The content a re-sign must keep: the channel, the app range and the
/// sections, as canonical bytes.
fn content(verified: &Verified) -> Result<Vec<u8>, PublishError> {
    let envelope = &verified.envelope;
    let value = json!({
        "channel": envelope.channel,
        "min_app": envelope.min_app.as_ref().map(ToString::to_string),
        "max_app": envelope.max_app.as_ref().map(ToString::to_string),
        "sections": envelope.sections,
    });
    to_canonical_bytes(&value).map_err(|e| PublishError::Read(format!("envelope: {e}")))
}

/// Whether `candidate` only re-signs `live`: both verify with `ring` for
/// `channel` (the live one's expiry aside), the candidate's serial is
/// higher and its expiry later, and its channel, app range and sections
/// are byte-identical in canonical form. What the unattended weekly
/// re-sign is held to: any content change goes through the reviewed
/// `Publish config`.
///
/// # Errors
///
/// [`PublishError::Rejected`] for a file that doesn't verify, and
/// [`PublishError::Invalid`] for one that changes anything but the serial
/// and the dates.
pub fn check_resign(
    live: &[u8],
    candidate: &[u8],
    ring: &KeyRing,
    channel: Channel,
) -> Result<Verified, PublishError> {
    let before = inspect(live, ring, channel)?;
    let after = inspect(candidate, ring, channel)?;
    let mut problems = Vec::new();
    if content(&after)? != content(&before)? {
        problems.push("the content differs from the live file's".to_owned());
    }
    if after.envelope.serial <= before.envelope.serial {
        problems.push("the serial doesn't rise".to_owned());
    }
    if after.envelope.expires_at <= before.envelope.expires_at {
        problems.push("the expiry doesn't move later".to_owned());
    }
    if problems.is_empty() {
        Ok(after)
    } else {
        Err(PublishError::Invalid(problems))
    }
}

/// Re-signs the live file's own content (and its revocation lists)
/// unchanged, with the next serial, issued `now` and valid for `days`:
/// the weekly re-sign. It reads no source, so it can't publish new
/// content, and [`check_resign`] holds the result to that.
///
/// # Errors
///
/// As [`inspect`], [`Stamp::new`], [`sign`] and [`check_resign`].
pub fn resign(
    live: &[u8],
    key: &SigningKey,
    ring: &KeyRing,
    channel: Channel,
    now: u64,
    days: u64,
) -> Result<Vec<u8>, PublishError> {
    let before = inspect(live, ring, channel)?;
    let envelope = before.envelope.clone();
    let source = Source {
        channel,
        min_app: envelope.min_app,
        max_app: envelope.max_app,
        sections: envelope.sections,
    };
    let stamp = Stamp::new(next_serial(Some(&before), channel), now, days)?;
    let file = sign(&source, &stamp, key, ring, &revocations_of(live)?)?;
    check_resign(live, &file, ring, channel)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_stays_in_what_the_app_accepts() {
        let stamp = Stamp::new(7, 1_000, DEFAULT_VALIDITY_DAYS).unwrap();
        assert_eq!(stamp.expires_at - stamp.issued_at, 14 * DAY_SECS);
        assert!(Stamp::new(0, 1_000, 14).is_err());
        assert!(Stamp::new(1, 1_000, 0).is_err());
        assert!(Stamp::new(1, 1_000, 32).is_err(), "past 31 days");
        assert!(Stamp::new(1 << 53, 1_000, 14).is_err());
        assert!(Stamp::new(1, u64::MAX - 10, 14).is_err());
    }

    #[test]
    fn a_secret_is_a_32_byte_seed() {
        let seed = BASE64.encode([7u8; 32]);
        assert!(signing_key(&format!(" {seed}\n")).is_ok());
        assert!(signing_key(&BASE64.encode([7u8; 31])).is_err());
        assert!(signing_key(&BASE64.encode([7u8; 33])).is_err());
        assert!(signing_key("not base64!").is_err());
        let message = signing_key("qqqq-secret").unwrap_err().to_string();
        assert!(!message.contains("qqqq"), "the secret is never echoed");
    }
}
