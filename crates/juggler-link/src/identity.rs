//! Machine identity (AJ-E702; ADR-0013 architecture contract).
//!
//! Each install owns one Ed25519 **machine identity key** — generated at
//! first run, stored owner-only (0600) under `fleet/` in the app data
//! dir, and never logged, exported, or sent over any link. Its public
//! half is what peers pin. A separate X25519 key is kept for the
//! ADR-0014 key-transfer envelope; neither is the account device token
//! or the backup vault key.
//!
//! Secret material lives only in [`MachineIdentity`], which deliberately
//! implements neither `Serialize` nor `Debug` output of key bytes.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// The identity key file (32-byte Ed25519 seed, base64, one line).
const IDENTITY_FILE: &str = "identity.key";
/// The X25519 key-transfer secret (32 bytes, base64, one line).
const EXCHANGE_FILE: &str = "exchange.key";

/// Errors loading or creating the machine identity.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// The OS random source failed.
    #[error("could not generate a key: the system random source failed")]
    Random,
    /// The key file exists but is not a valid key.
    #[error("the machine identity file is damaged — reset this Mac's identity to continue")]
    Corrupt,
    /// The key file is readable by other users.
    #[error("the machine identity file is readable by other users — fix its permissions (0600)")]
    TooOpen,
    /// Filesystem failure.
    #[error("machine identity storage: {0}")]
    Io(String),
}

/// A peer's public identity: the Ed25519 key peers pin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PublicIdentity(VerifyingKey);

impl PublicIdentity {
    /// Parses 32 raw public-key bytes.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Corrupt`] when the bytes are not a valid
    /// Ed25519 point.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, IdentityError> {
        VerifyingKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| IdentityError::Corrupt)
    }

    /// Parses the standard-base64 form ([`Self::to_base64`]).
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Corrupt`] on bad base64 or a bad key.
    pub fn from_base64(text: &str) -> Result<Self, IdentityError> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(text.trim())
            .map_err(|_| IdentityError::Corrupt)?;
        let bytes: [u8; 32] = raw.try_into().map_err(|_| IdentityError::Corrupt)?;
        Self::from_bytes(&bytes)
    }

    /// The 32 raw public-key bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Standard base64 of the public key (what the registry stores).
    #[must_use]
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.0.to_bytes())
    }

    /// The display fingerprint: SHA-256 of the public key, first 16
    /// bytes as 8 groups of 4 upper-case hex digits. Pinning always
    /// compares the full key — this is for humans.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        fingerprint_of(&self.0.to_bytes())
    }

    /// Verifies `signature` over `message` with this key.
    #[must_use]
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        self.0
            .verify(message, &Signature::from_bytes(signature))
            .is_ok()
    }
}

/// The fingerprint for raw public-key bytes (see
/// [`PublicIdentity::fingerprint`]).
#[must_use]
pub fn fingerprint_of(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    digest[..16]
        .chunks(2)
        .map(|pair| format!("{:02X}{:02X}", pair[0], pair[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Where the identity's two secrets are kept (AJ-E2005): the system's
/// keychain in the app, owner-only files in tests and where no keychain
/// is wired yet. Each secret is a base64 line, named `IDENTITY_FILE` or
/// `EXCHANGE_FILE`.
pub trait KeyKeeper: Send + Sync {
    /// The secret called `name`, if kept.
    fn get(&self, name: &str) -> Option<String>;
    /// Keeps `secret` as `name`.
    ///
    /// # Errors
    ///
    /// The keeper's refusal, in words.
    fn set(&self, name: &str, secret: &str) -> Result<(), String>;
    /// Forgets `name` (absent is fine).
    fn delete(&self, name: &str);
    /// Whether it keeps secrets in the Fleet folder itself (files).
    fn is_files(&self) -> bool {
        false
    }
}

/// Owner-only files in the Fleet folder: the way before AJ-E2005, kept
/// for tests and for systems without a keychain seam yet.
#[derive(Debug, Clone)]
pub struct FileKeeper(pub PathBuf);

impl KeyKeeper for FileKeeper {
    fn get(&self, name: &str) -> Option<String> {
        let path = self.0.join(name);
        std::fs::read_to_string(&path).ok()
    }
    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        write_owner_only(&self.0.join(name), secret.as_bytes()).map_err(|e| e.to_string())
    }
    fn delete(&self, name: &str) {
        let _ = std::fs::remove_file(self.0.join(name));
    }
    fn is_files(&self) -> bool {
        true
    }
}

/// This machine's identity: the signing key and the X25519 exchange key.
pub struct MachineIdentity {
    signing: SigningKey,
    exchange: x25519_dalek::StaticSecret,
    dir: PathBuf,
    keeper: std::sync::Arc<dyn KeyKeeper>,
}

impl std::fmt::Debug for MachineIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material — only the public fingerprint.
        f.debug_struct("MachineIdentity")
            .field("fingerprint", &self.public().fingerprint())
            .finish_non_exhaustive()
    }
}

impl MachineIdentity {
    /// Loads the identity from `fleet_dir`, generating it on first run.
    /// Files are created owner-only (0600) from the first byte.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] when the random source fails, a file is
    /// damaged or readable by others, or storage fails.
    pub fn load_or_create(fleet_dir: &Path) -> Result<Self, IdentityError> {
        Self::load_kept(
            fleet_dir,
            std::sync::Arc::new(FileKeeper(fleet_dir.to_path_buf())),
        )
    }

    /// Loads the identity from `keeper`, generating it on first run.
    /// AJ-E2005: a key still in the Fleet folder as a file (from before)
    /// moves into the keeper — read back to be sure — and the file is
    /// deleted, so the folder holds no private key.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] as [`Self::load_or_create`], or
    /// [`IdentityError::Io`] when the keeper refuses.
    pub fn load_kept(
        fleet_dir: &Path,
        keeper: std::sync::Arc<dyn KeyKeeper>,
    ) -> Result<Self, IdentityError> {
        std::fs::create_dir_all(fleet_dir).map_err(|e| IdentityError::Io(e.to_string()))?;
        restrict_dir(fleet_dir)?;
        let signing_seed = kept_secret(fleet_dir, keeper.as_ref(), IDENTITY_FILE)?;
        let exchange_seed = kept_secret(fleet_dir, keeper.as_ref(), EXCHANGE_FILE)?;
        Ok(Self {
            signing: SigningKey::from_bytes(&signing_seed),
            exchange: x25519_dalek::StaticSecret::from(exchange_seed),
            dir: fleet_dir.to_path_buf(),
            keeper,
        })
    }

    /// Deletes and regenerates both keys ("Reset this Mac's identity").
    /// Every peer must re-pair afterwards.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] per [`Self::load_or_create`].
    pub fn regenerate(self) -> Result<Self, IdentityError> {
        for name in [IDENTITY_FILE, EXCHANGE_FILE] {
            self.keeper.delete(name);
            match std::fs::remove_file(self.dir.join(name)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(IdentityError::Io(e.to_string())),
            }
        }
        Self::load_kept(&self.dir, self.keeper)
    }

    /// The public identity peers pin.
    #[must_use]
    pub fn public(&self) -> PublicIdentity {
        PublicIdentity(self.signing.verifying_key())
    }

    /// The X25519 public key (ADR-0014 key-transfer envelope).
    #[must_use]
    pub fn exchange_public(&self) -> [u8; 32] {
        x25519_dalek::PublicKey::from(&self.exchange).to_bytes()
    }

    /// The account-link `publicKey` field (ADR-0014):
    /// `ed25519:<base64>;x25519:<base64>`.
    #[must_use]
    pub fn link_public_key(&self) -> String {
        let engine = base64::engine::general_purpose::STANDARD;
        format!(
            "ed25519:{};x25519:{}",
            engine.encode(self.public().to_bytes()),
            engine.encode(self.exchange_public())
        )
    }

    /// Signs `message` with the machine identity key.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    /// The raw 32-byte Ed25519 seed, for the transport's certificate
    /// builder only (AJ-E703). Never log or persist the result.
    #[must_use]
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "consumed by the AJ-E703 certificate builder")
    )]
    pub(crate) fn signing_seed(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }
}

/// 32 bytes from the OS random source.
fn random_seed() -> Result<[u8; 32], IdentityError> {
    let mut seed = [0_u8; 32];
    getrandom::fill(&mut seed).map_err(|_| IdentityError::Random)?;
    Ok(seed)
}

fn decode(text: &str) -> Result<[u8; 32], IdentityError> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|_| IdentityError::Corrupt)?;
    raw.try_into().map_err(|_| IdentityError::Corrupt)
}

/// The secret `name` from `keeper`; moved in from its old file when only
/// the file has it; created when neither does.
fn kept_secret(dir: &Path, keeper: &dyn KeyKeeper, name: &str) -> Result<[u8; 32], IdentityError> {
    let file = dir.join(name);
    if keeper.is_files() && file.exists() {
        check_owner_only(&file)?;
    }
    if let Some(text) = keeper.get(name) {
        let seed = decode(&text)?;
        if !keeper.is_files() {
            // Kept elsewhere already: a file left from before goes.
            let _ = std::fs::remove_file(&file);
        }
        return Ok(seed);
    }
    let encoded = |seed: &[u8; 32]| base64::engine::general_purpose::STANDARD.encode(seed);
    let seed = match std::fs::read_to_string(&file) {
        Ok(text) => {
            check_owner_only(&file)?;
            decode(&text)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => random_seed()?,
        Err(e) => return Err(IdentityError::Io(e.to_string())),
    };
    keeper
        .set(name, &encoded(&seed))
        .map_err(IdentityError::Io)?;
    // Only once the keeper gives it back is the file let go.
    if keeper.get(name).as_deref().map(str::trim) != Some(encoded(&seed).as_str()) {
        return Err(IdentityError::Io("the key couldn't be kept".to_owned()));
    }
    if !keeper.is_files() {
        let _ = std::fs::remove_file(&file);
    }
    Ok(seed)
}

/// Atomic, owner-only write: the temp file is 0600 from creation, then
/// renamed into place, so the secret is never briefly world-readable.
pub(crate) fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<(), IdentityError> {
    use std::io::Write;
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|e| IdentityError::Io(e.to_string()))
}

/// Refuses a secret file that group/others can read (unix).
fn check_owner_only(path: &Path) -> Result<(), IdentityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .map_err(|e| IdentityError::Io(e.to_string()))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(IdentityError::TooOpen);
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Makes the fleet folder owner-only (unix).
fn restrict_dir(dir: &Path) -> Result<(), IdentityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| IdentityError::Io(e.to_string()))?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_generates_owner_only_keys_and_later_runs_reload_them() {
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        let first = MachineIdentity::load_or_create(&fleet).unwrap();
        let again = MachineIdentity::load_or_create(&fleet).unwrap();
        assert_eq!(first.public(), again.public(), "the identity is durable");
        assert_eq!(first.exchange_public(), again.exchange_public());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for name in [IDENTITY_FILE, EXCHANGE_FILE] {
                let mode = std::fs::metadata(fleet.join(name))
                    .unwrap()
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o600, "{name} must be 0600");
            }
            let dir_mode = std::fs::metadata(&fleet).unwrap().permissions().mode();
            assert_eq!(dir_mode & 0o777, 0o700);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_key_file_readable_by_others_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        MachineIdentity::load_or_create(&fleet).unwrap();
        std::fs::set_permissions(
            fleet.join(IDENTITY_FILE),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(matches!(
            MachineIdentity::load_or_create(&fleet),
            Err(IdentityError::TooOpen)
        ));
    }

    #[test]
    fn a_damaged_key_file_is_refused_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        MachineIdentity::load_or_create(&fleet).unwrap();
        std::fs::remove_file(fleet.join(IDENTITY_FILE)).unwrap();
        write_owner_only(&fleet.join(IDENTITY_FILE), b"not-base64!").unwrap();
        assert!(matches!(
            MachineIdentity::load_or_create(&fleet),
            Err(IdentityError::Corrupt)
        ));
    }

    /// A keychain stand-in.
    #[derive(Default)]
    struct Memory(std::sync::Mutex<std::collections::HashMap<String, String>>);

    impl KeyKeeper for Memory {
        fn get(&self, name: &str) -> Option<String> {
            self.0.lock().unwrap().get(name).cloned()
        }
        fn set(&self, name: &str, secret: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .insert(name.to_owned(), secret.to_owned());
            Ok(())
        }
        fn delete(&self, name: &str) {
            self.0.lock().unwrap().remove(name);
        }
    }

    #[test]
    fn keys_from_before_move_into_the_keeper_and_the_folder_keeps_none() {
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        let before = MachineIdentity::load_or_create(&fleet).unwrap().public();
        assert!(fleet.join(IDENTITY_FILE).exists());
        let keeper = std::sync::Arc::new(Memory::default());
        let moved = MachineIdentity::load_kept(&fleet, keeper.clone()).unwrap();
        assert_eq!(
            moved.public(),
            before,
            "the same identity: no peer re-pairs"
        );
        assert!(!fleet.join(IDENTITY_FILE).exists() && !fleet.join(EXCHANGE_FILE).exists());
        assert_eq!(
            std::fs::read_dir(&fleet).unwrap().count(),
            0,
            "no private key in the folder"
        );
        // And back again: later loads read the keeper alone.
        assert_eq!(
            MachineIdentity::load_kept(&fleet, keeper.clone())
                .unwrap()
                .public(),
            before
        );
        // Reset forgets the kept keys and makes new ones, kept the same way.
        let fresh = moved.regenerate().unwrap();
        assert_ne!(fresh.public(), before);
        assert!(!fleet.join(IDENTITY_FILE).exists());
        assert_eq!(
            MachineIdentity::load_kept(&fleet, keeper).unwrap().public(),
            fresh.public()
        );
    }

    #[test]
    fn a_keeper_that_refuses_leaves_the_file_where_it_was() {
        struct Refuses;
        impl KeyKeeper for Refuses {
            fn get(&self, _: &str) -> Option<String> {
                None
            }
            fn set(&self, _: &str, _: &str) -> Result<(), String> {
                Err("locked".to_owned())
            }
            fn delete(&self, _: &str) {}
        }
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        drop(MachineIdentity::load_or_create(&fleet).unwrap());
        assert!(MachineIdentity::load_kept(&fleet, std::sync::Arc::new(Refuses)).is_err());
        assert!(fleet.join(IDENTITY_FILE).exists(), "never lost on the way");
    }

    #[test]
    fn regenerate_changes_the_identity() {
        let dir = tempfile::tempdir().unwrap();
        let identity = MachineIdentity::load_or_create(&dir.path().join("fleet")).unwrap();
        let before = identity.public();
        let after = identity.regenerate().unwrap();
        assert_ne!(
            before,
            after.public(),
            "a reset forces every peer to re-pair"
        );
    }

    #[test]
    fn signatures_verify_only_with_the_right_key_and_message() {
        let dir = tempfile::tempdir().unwrap();
        let a = MachineIdentity::load_or_create(&dir.path().join("a")).unwrap();
        let b = MachineIdentity::load_or_create(&dir.path().join("b")).unwrap();
        let signature = a.sign(b"registry v1");
        assert!(a.public().verify(b"registry v1", &signature));
        assert!(!a.public().verify(b"registry v2", &signature));
        assert!(!b.public().verify(b"registry v1", &signature));
    }

    #[test]
    fn fingerprint_is_eight_groups_and_base64_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let identity = MachineIdentity::load_or_create(&dir.path().join("fleet")).unwrap();
        let fingerprint = identity.public().fingerprint();
        let groups: Vec<&str> = fingerprint.split(' ').collect();
        assert_eq!(groups.len(), 8);
        assert!(groups.iter().all(|group| {
            group.len() == 4
                && group
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
        }));
        let decoded = PublicIdentity::from_base64(&identity.public().to_base64()).unwrap();
        assert_eq!(decoded, identity.public());
        assert!(PublicIdentity::from_base64("AAAA").is_err());
        let link = identity.link_public_key();
        assert!(link.starts_with("ed25519:") && link.contains(";x25519:"));
    }

    #[test]
    fn debug_output_never_contains_key_material() {
        let dir = tempfile::tempdir().unwrap();
        let identity = MachineIdentity::load_or_create(&dir.path().join("fleet")).unwrap();
        let seed = base64::engine::general_purpose::STANDARD.encode(identity.signing_seed());
        let printed = format!("{identity:?}");
        assert!(!printed.contains(&seed));
        assert!(printed.contains("fingerprint"));
    }
}
