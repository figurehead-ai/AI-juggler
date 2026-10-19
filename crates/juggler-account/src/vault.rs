//! AJ-E1204: the backup vault — a key only this user holds, and the
//! encrypted container that Cloud backup uploads (ADR-0017).
//!
//! - **The vault key** is 32 random bytes made on this Mac the first time
//!   Cloud backup is turned on. It lives in AI Juggler's own Keychain item
//!   (service [`VAULT_SERVICE`]) and is never derived from the device
//!   token or the Fleet identity. Figurehead never sees it.
//! - **The recovery code** is the same key written for people: base32
//!   (RFC 4648, `A–Z 2–7`) in groups of four, with one checksum group at
//!   the end so a typo is caught as a typo, not as a wrong key.
//! - **The container** is
//!   `FHB1 | version 0x01 | key id (8) | nonce (24) | ciphertext+tag`,
//!   sealed with XChaCha20-Poly1305 and the associated data
//!   `figurehead-backup-v1`. The key id (the first 8 bytes of
//!   `sha256("figurehead-vault-key-id" ‖ key)`) lets a wrong key be named
//!   as a wrong key; any other change fails authentication and is named
//!   as damaged or altered.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};

use crate::secrets::{SecretError, SecretStore};

/// AI Juggler's Keychain service for the vault key (its own item, apart
/// from the device token's).
pub const VAULT_SERVICE: &str = "ai.figurehead.juggler.vault";
/// The Keychain account name the key is stored under.
pub const VAULT_ACCOUNT: &str = "vault-key";
/// The container's magic bytes.
pub const MAGIC: &[u8; 4] = b"FHB1";
/// The container version this build writes and reads.
pub const VERSION: u8 = 1;
/// The associated data every container is sealed with.
pub const AAD: &[u8] = b"figurehead-backup-v1";
const KEY_ID_DOMAIN: &[u8] = b"figurehead-vault-key-id";
const CHECK_DOMAIN: &[u8] = b"figurehead-recovery-check";
const HEADER_LEN: usize = 4 + 1 + 8 + 24;
const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Why a vault step failed, in words the user can act on.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VaultError {
    /// Not an AI Juggler backup container at all.
    #[error("this isn't an AI Juggler cloud backup")]
    WrongMagic,
    /// A newer container format.
    #[error("this backup uses format {0}; update AI Juggler to open it")]
    UnsupportedVersion(u8),
    /// Sealed with a different vault key.
    #[error("this backup was made with a different recovery code")]
    WrongKey,
    /// Authentication failed: the bytes were changed.
    #[error("this backup is damaged or was altered — it can't be opened")]
    Tampered,
    /// The recovery code has the wrong length or characters.
    #[error("that recovery code isn't complete — it has 14 groups of 4 letters and digits")]
    MalformedCode,
    /// The recovery code's checksum does not match (a typo).
    #[error("that recovery code has a typo — check each group")]
    CodeTypo,
    /// The system random source failed.
    #[error("couldn't make a vault key: no secure random source")]
    Random,
    /// The Keychain refused.
    #[error(transparent)]
    Store(#[from] SecretError),
}

/// The 32-byte vault key. Its bytes never appear in `Debug` output.
#[derive(Clone, PartialEq, Eq)]
pub struct VaultKey([u8; 32]);

impl std::fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VaultKey({})", hex(&self.id()))
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

impl VaultKey {
    /// A fresh random key.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Random`] when the system random source fails.
    pub fn generate() -> Result<Self, VaultError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| VaultError::Random)?;
        Ok(Self(bytes))
    }

    /// A key from known bytes (tests and known-answer vectors).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The key id written into each container header.
    #[must_use]
    pub fn id(&self) -> [u8; 8] {
        let digest = Sha256::new()
            .chain_update(KEY_ID_DOMAIN)
            .chain_update(self.0)
            .finalize();
        let mut id = [0u8; 8];
        id.copy_from_slice(&digest[..8]);
        id
    }

    /// The recovery code: 13 groups of four base32 characters for the
    /// key, then one checksum group (14 groups).
    #[must_use]
    pub fn recovery_code(&self) -> String {
        let mut chars = base32(&self.0); // 52 characters
        chars.push_str(&checksum(&self.0));
        chars
            .as_bytes()
            .chunks(4)
            .map(|group| String::from_utf8_lossy(group).into_owned())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Reads a recovery code back: spaces, dashes and case are ignored,
    /// and the digits people confuse with letters (0, 1, 8) are read as
    /// O, I and B.
    ///
    /// # Errors
    ///
    /// [`VaultError::MalformedCode`] for the wrong length or characters,
    /// [`VaultError::CodeTypo`] when the checksum group does not match.
    pub fn from_recovery_code(code: &str) -> Result<Self, VaultError> {
        let cleaned: String = code
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| match c.to_ascii_uppercase() {
                '0' => 'O',
                '1' => 'I',
                '8' => 'B',
                other => other,
            })
            .collect();
        if cleaned.len() != 56 || !cleaned.bytes().all(|b| BASE32.contains(&b)) {
            return Err(VaultError::MalformedCode);
        }
        let (body, check) = cleaned.split_at(52);
        let bytes = unbase32(body).ok_or(VaultError::MalformedCode)?;
        let key: [u8; 32] = bytes
            .get(..32)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(VaultError::MalformedCode)?;
        if checksum(&key) != check {
            return Err(VaultError::CodeTypo);
        }
        Ok(Self(key))
    }

    /// Reads the key from `store`, if one was made.
    #[must_use]
    pub fn load(store: &dyn SecretStore) -> Option<Self> {
        Self::from_recovery_code(&store.get(VAULT_ACCOUNT)?).ok()
    }

    /// Saves the key in `store` (as its recovery code).
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Store`] when the Keychain refuses.
    pub fn save(&self, store: &dyn SecretStore) -> Result<(), VaultError> {
        Ok(store.set(VAULT_ACCOUNT, &self.recovery_code())?)
    }
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn unbase32(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for c in text.bytes() {
        let value = u32::try_from(BASE32.iter().position(|b| *b == c)?).ok()?;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

/// Four base32 characters (20 bits) over the key.
fn checksum(key: &[u8; 32]) -> String {
    let digest = Sha256::new()
        .chain_update(CHECK_DOMAIN)
        .chain_update(key)
        .finalize();
    base32(&digest[..3])[..4].to_owned()
}

/// Seals `plaintext` into a container with a fresh random nonce.
///
/// # Errors
///
/// Returns [`VaultError::Random`] when no nonce can be made.
pub fn seal(key: &VaultKey, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce).map_err(|_| VaultError::Random)?;
    Ok(seal_with_nonce(key, &nonce, plaintext))
}

/// Seals with a given nonce (known-answer tests only; production always
/// uses [`seal`]).
#[must_use]
pub fn seal_with_nonce(key: &VaultKey, nonce: &[u8; 24], plaintext: &[u8]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new((&key.0).into());
    let ciphertext = cipher
        .encrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: plaintext,
                aad: AAD,
            },
        )
        .unwrap_or_default(); // only fails for > 256 GiB inputs
    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&key.id());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ciphertext);
    out
}

/// Opens a container.
///
/// # Errors
///
/// [`VaultError::WrongMagic`], [`VaultError::UnsupportedVersion`],
/// [`VaultError::WrongKey`] or [`VaultError::Tampered`], each with its
/// own reason.
pub fn open(key: &VaultKey, container: &[u8]) -> Result<Vec<u8>, VaultError> {
    if container.len() < 5 || &container[..4] != MAGIC {
        return Err(VaultError::WrongMagic);
    }
    if container[4] != VERSION {
        return Err(VaultError::UnsupportedVersion(container[4]));
    }
    if container.len() < HEADER_LEN + 16 {
        return Err(VaultError::Tampered);
    }
    if container[5..13] != key.id() {
        return Err(VaultError::WrongKey);
    }
    let nonce = XNonce::try_from(&container[13..HEADER_LEN]).map_err(|_| VaultError::Tampered)?;
    let cipher = XChaCha20Poly1305::new((&key.0).into());
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &container[HEADER_LEN..],
                aad: AAD,
            },
        )
        .map_err(|_| VaultError::Tampered)
}

/// The vault key's Keychain item: [`VAULT_SERVICE`] / [`VAULT_ACCOUNT`].
#[derive(Debug, Default, Clone, Copy)]
pub struct VaultKeychain;

#[cfg(target_os = "macos")]
impl SecretStore for VaultKeychain {
    fn get(&self, account: &str) -> Option<String> {
        security_framework::passwords::get_generic_password(VAULT_SERVICE, account)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        security_framework::passwords::set_generic_password(
            VAULT_SERVICE,
            account,
            secret.as_bytes(),
        )
        .map_err(|_| SecretError::Store("save the vault key"))
    }

    fn delete(&self, account: &str) {
        let _ = security_framework::passwords::delete_generic_password(VAULT_SERVICE, account);
    }
}

#[cfg(not(target_os = "macos"))]
impl SecretStore for VaultKeychain {
    fn get(&self, _account: &str) -> Option<String> {
        None
    }

    fn set(&self, _account: &str, _secret: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported)
    }

    fn delete(&self, _account: &str) {}
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::secrets::MemoryStore;

    fn key() -> VaultKey {
        VaultKey::from_bytes(core::array::from_fn(|i| u8::try_from(i).unwrap()))
    }

    #[test]
    fn round_trip() {
        let sealed = seal(&key(), b"the backup set").unwrap();
        assert_eq!(&sealed[..4], b"FHB1");
        assert_eq!(sealed[4], 1);
        assert_eq!(open(&key(), &sealed).unwrap(), b"the backup set");
    }

    #[test]
    fn known_answer() {
        let nonce: [u8; 24] = core::array::from_fn(|i| u8::try_from(0x40 + i).unwrap());
        let sealed = seal_with_nonce(&key(), &nonce, b"figurehead");
        assert_eq!(
            hex(&sealed),
            KNOWN_ANSWER,
            "the container bytes for a fixed key, nonce and message never change"
        );
    }

    /// FHB1 · 01 · key id · nonce · ciphertext+tag for [`known_answer`].
    const KNOWN_ANSWER: &str = include_str!("vault_kat.hex");

    #[test]
    fn wrong_key_magic_version_and_tampering_are_named() {
        let sealed = seal(&key(), b"payload").unwrap();
        let other = VaultKey::generate().unwrap();
        assert_eq!(open(&other, &sealed), Err(VaultError::WrongKey));
        let mut bad = sealed.clone();
        bad[0] = b'X';
        assert_eq!(open(&key(), &bad), Err(VaultError::WrongMagic));
        let mut bad = sealed.clone();
        bad[4] = 2;
        assert_eq!(open(&key(), &bad), Err(VaultError::UnsupportedVersion(2)));
        for at in [13, 20, HEADER_LEN, sealed.len() - 1] {
            let mut bad = sealed.clone();
            bad[at] ^= 0x01;
            assert_eq!(open(&key(), &bad), Err(VaultError::Tampered), "byte {at}");
        }
        assert_eq!(
            open(&key(), &sealed[..sealed.len() - 3]),
            Err(VaultError::Tampered)
        );
    }

    #[test]
    fn the_recovery_code_round_trips_and_catches_typos() {
        let key = VaultKey::generate().unwrap();
        let code = key.recovery_code();
        let groups: Vec<&str> = code.split('-').collect();
        assert_eq!(groups.len(), 14);
        assert!(groups.iter().all(|g| g.len() == 4));
        assert_eq!(VaultKey::from_recovery_code(&code).unwrap(), key);
        // Case, spaces and look-alike digits are forgiven.
        let relaxed = code.to_lowercase().replace('-', " ").replace('o', "0");
        assert_eq!(VaultKey::from_recovery_code(&relaxed).unwrap(), key);
        // One wrong character is a typo, not a wrong key.
        let mut chars: Vec<char> = code.chars().collect();
        chars[2] = if chars[2] == 'A' { 'B' } else { 'A' };
        let typo: String = chars.into_iter().collect();
        assert_eq!(
            VaultKey::from_recovery_code(&typo),
            Err(VaultError::CodeTypo)
        );
        assert_eq!(
            VaultKey::from_recovery_code("ABCD-EFGH"),
            Err(VaultError::MalformedCode)
        );
    }

    #[test]
    fn the_key_is_stored_as_its_own_keychain_item_and_never_printed() {
        let store = MemoryStore::default();
        assert!(VaultKey::load(&store).is_none());
        let key = VaultKey::generate().unwrap();
        key.save(&store).unwrap();
        assert_eq!(VaultKey::load(&store).unwrap(), key);
        let debug = format!("{key:?}");
        assert!(!debug.contains(&key.recovery_code()[..9]), "{debug}");
        assert_ne!(VAULT_SERVICE, crate::secrets::KEYCHAIN_SERVICE);
    }

    #[test]
    fn no_plaintext_canary_survives_in_the_ciphertext() {
        let canary = b"CANARY-sk-ant-api03-plaintext-must-not-appear";
        let mut message = vec![b'x'; 4096];
        message.extend_from_slice(canary);
        let sealed = seal(&key(), &message).unwrap();
        assert!(!sealed.windows(canary.len()).any(|w| w == canary));
        assert!(!sealed.windows(12).any(|w| w == &canary[..12]));
    }

    #[test]
    fn a_25_mb_container_seals_and_opens_inside_a_second() {
        let data = vec![7u8; 25 * 1024 * 1024];
        let started = std::time::Instant::now();
        let sealed = seal(&key(), &data).unwrap();
        let opened = open(&key(), &sealed).unwrap();
        let took = started.elapsed();
        eprintln!(
            "[perf] 25 MB seal + open: {took:?} (≈{:.0} MB/s each way)",
            50.0 / took.as_secs_f64()
        );
        assert_eq!(opened.len(), data.len());
        assert!(took.as_secs_f64() < 2.0 * 25.0 / 100.0 + 0.5, "{took:?}");
    }
}
