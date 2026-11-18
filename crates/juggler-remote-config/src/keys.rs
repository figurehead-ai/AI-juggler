//! Pinned ed25519 public keys (§3).
//!
//! The app ships two or three public keys, each with an id and a role:
//!
//! - **current** and **next** sign configuration files. Rotation is
//!   publishing with `next`, then shipping an app whose `current` is the
//!   old `next` and that carries a fresh `next`.
//! - **emergency** signs only revocation lists. It can't sign a
//!   configuration, so stealing it gains nothing but the power to revoke
//!   the other keys, which leaves the app on its compiled defaults (safe,
//!   and complete). A file can't revoke the emergency key; it is retired
//!   by shipping an app without it.
//!
//! A key is retired by shipping an app without it, or revoked by a
//! revocation list (see [`crate::envelope`]).

use std::collections::BTreeSet;

use ed25519_dalek::{Signature, VerifyingKey};

use crate::Rejection;

/// The longest key id accepted.
pub const MAX_KEY_ID_LEN: usize = 64;

/// The most keys a key ring holds.
pub const MAX_KEYS: usize = 3;

/// The release keys compiled into the app: `(id, role, public key)`.
///
/// Empty until the publish workflow's keys exist. Their private halves are
/// made in, and never leave, the release environment (ADR-0035 §3), so
/// they can't be generated here; tests make their own key rings. With no
/// key, nothing verifies and the app runs on its compiled defaults, which
/// is the fail-closed state.
pub const COMPILED_KEYS: &[(&str, KeyRole, [u8; 32])] = &[];

/// What a pinned key may sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyRole {
    /// Signs configuration files today.
    Current,
    /// Signs configuration files after (or during) a rotation.
    Next,
    /// Signs revocation lists only.
    Emergency,
}

/// What a signature is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// A configuration envelope.
    Config,
    /// A revocation list.
    Revocations,
}

impl KeyRole {
    /// Whether a key of this role may sign for `purpose`.
    #[must_use]
    pub const fn may_sign(self, purpose: Purpose) -> bool {
        matches!(
            (self, purpose),
            (Self::Current | Self::Next, Purpose::Config) | (Self::Emergency, Purpose::Revocations)
        )
    }
}

/// A key ring that can't be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    /// The bytes are not a valid, non-weak ed25519 public key.
    #[error("key {0:?} is not a usable ed25519 public key")]
    BadKey(String),
    /// Two keys share an id.
    #[error("key id {0:?} is pinned twice")]
    DuplicateId(String),
    /// An empty or overlong id, or one with characters outside `[a-z0-9-]`.
    #[error("key id {0:?} is not allowed")]
    BadId(String),
    /// More than [`MAX_KEYS`] keys, or two keys with one role.
    #[error("a key ring holds at most one key per role")]
    BadShape,
}

/// One pinned public key.
#[derive(Debug, Clone)]
pub struct PinnedKey {
    id: String,
    role: KeyRole,
    key: VerifyingKey,
}

impl PinnedKey {
    /// A pinned key from its id, role and 32 public-key bytes.
    ///
    /// # Errors
    ///
    /// [`KeyError::BadId`] for an id that isn't 1–64 of `[a-z0-9-]`, and
    /// [`KeyError::BadKey`] for bytes that aren't a point, or are a weak
    /// (small-order) point.
    pub fn new(id: &str, role: KeyRole, public: &[u8; 32]) -> Result<Self, KeyError> {
        if !valid_id(id) {
            return Err(KeyError::BadId(id.to_owned()));
        }
        let key = VerifyingKey::from_bytes(public).map_err(|_| KeyError::BadKey(id.to_owned()))?;
        if key.is_weak() {
            return Err(KeyError::BadKey(id.to_owned()));
        }
        Ok(Self {
            id: id.to_owned(),
            role,
            key,
        })
    }

    /// The key's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The key's role.
    #[must_use]
    pub const fn role(&self) -> KeyRole {
        self.role
    }
}

/// Whether `id` is a well-formed key id.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_KEY_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The pinned keys a file is checked against.
#[derive(Debug, Clone, Default)]
pub struct KeyRing {
    keys: Vec<PinnedKey>,
}

impl KeyRing {
    /// A key ring of at most [`MAX_KEYS`] keys, one per role, unique ids.
    ///
    /// # Errors
    ///
    /// [`KeyError::DuplicateId`] or [`KeyError::BadShape`].
    pub fn new(keys: Vec<PinnedKey>) -> Result<Self, KeyError> {
        if keys.len() > MAX_KEYS {
            return Err(KeyError::BadShape);
        }
        let mut ids = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for key in &keys {
            if !ids.insert(key.id.as_str()) {
                return Err(KeyError::DuplicateId(key.id.clone()));
            }
            if !roles.insert(key.role) {
                return Err(KeyError::BadShape);
            }
        }
        Ok(Self { keys })
    }

    /// The keys compiled into this build ([`COMPILED_KEYS`]).
    ///
    /// # Errors
    ///
    /// As [`KeyRing::new`] and [`PinnedKey::new`]; a test holds the
    /// compiled set to them.
    pub fn compiled() -> Result<Self, KeyError> {
        let keys = COMPILED_KEYS
            .iter()
            .map(|(id, role, public)| PinnedKey::new(id, *role, public))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(keys)
    }

    /// The pinned key with `id`, if any.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PinnedKey> {
        self.keys.iter().find(|key| key.id == id)
    }

    /// The pinned keys.
    #[must_use]
    pub fn keys(&self) -> &[PinnedKey] {
        &self.keys
    }

    /// Checks that `signature` over `message` was made by the pinned,
    /// unrevoked key `key_id`, whose role may sign for `purpose`.
    ///
    /// Uses `verify_strict` (no malleable signatures, no small-order
    /// components).
    ///
    /// # Errors
    ///
    /// [`Rejection::UnknownKey`], [`Rejection::RevokedKey`],
    /// [`Rejection::WrongKeyRole`] or [`Rejection::BadSignature`].
    pub fn verify(
        &self,
        key_id: &str,
        purpose: Purpose,
        message: &[u8],
        signature: &[u8; 64],
        revoked: &BTreeSet<String>,
    ) -> Result<&PinnedKey, Rejection> {
        let key = self
            .get(key_id)
            .ok_or_else(|| Rejection::UnknownKey(key_id.to_owned()))?;
        // The emergency key can't be revoked by a file (see module docs).
        if key.role != KeyRole::Emergency && revoked.contains(key_id) {
            return Err(Rejection::RevokedKey(key_id.to_owned()));
        }
        if !key.role.may_sign(purpose) {
            return Err(Rejection::WrongKeyRole(key_id.to_owned()));
        }
        key.key
            .verify_strict(message, &Signature::from_bytes(signature))
            .map_err(|_| Rejection::BadSignature)?;
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_keys_are_well_formed() {
        let ring = KeyRing::compiled().unwrap();
        assert!(ring.keys().len() <= MAX_KEYS);
    }

    #[test]
    fn rejects_bad_ids_weak_keys_and_bad_shapes() {
        let good = ed25519_dalek::SigningKey::from_bytes(&[7; 32])
            .verifying_key()
            .to_bytes();
        assert!(matches!(
            PinnedKey::new("Bad Id", KeyRole::Current, &good),
            Err(KeyError::BadId(_))
        ));
        assert!(matches!(
            PinnedKey::new("", KeyRole::Current, &good),
            Err(KeyError::BadId(_))
        ));
        // The identity point is a valid encoding but a weak key.
        let mut identity = [0u8; 32];
        identity[0] = 1;
        assert!(matches!(
            PinnedKey::new("weak", KeyRole::Current, &identity),
            Err(KeyError::BadKey(_))
        ));
        let a = PinnedKey::new("a", KeyRole::Current, &good).unwrap();
        let b = PinnedKey::new("a", KeyRole::Next, &good).unwrap();
        assert!(matches!(
            KeyRing::new(vec![a.clone(), b]),
            Err(KeyError::DuplicateId(_))
        ));
        let c = PinnedKey::new("c", KeyRole::Current, &good).unwrap();
        assert!(matches!(KeyRing::new(vec![a, c]), Err(KeyError::BadShape)));
    }

    #[test]
    fn roles_sign_only_their_purpose() {
        assert!(KeyRole::Current.may_sign(Purpose::Config));
        assert!(KeyRole::Next.may_sign(Purpose::Config));
        assert!(!KeyRole::Emergency.may_sign(Purpose::Config));
        assert!(KeyRole::Emergency.may_sign(Purpose::Revocations));
        assert!(!KeyRole::Current.may_sign(Purpose::Revocations));
    }
}
