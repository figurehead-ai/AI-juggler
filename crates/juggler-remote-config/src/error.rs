//! Why a file was not used.

use crate::canonical::CanonicalError;

/// Why a fetched or cached file was rejected. Every rejection leaves the
/// app on its last good file, or on its compiled defaults.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rejection {
    /// Larger than [`crate::MAX_FILE_BYTES`].
    #[error("the file is {0} bytes, over the cap")]
    TooLarge(usize),
    /// The wrapper isn't the signed-file shape, or a part isn't base64, or
    /// a signature isn't 64 bytes, or the configuration part is missing
    /// (an unsigned file).
    #[error("malformed file: {0}")]
    Malformed(&'static str),
    /// A wrapper or envelope `format` this app doesn't read.
    #[error("unsupported format {0}")]
    UnsupportedFormat(u64),
    /// No pinned key has this id.
    #[error("unknown key {0:?}")]
    UnknownKey(String),
    /// The key was revoked by a signed revocation list.
    #[error("revoked key {0:?}")]
    RevokedKey(String),
    /// The key's role may not sign this kind of document.
    #[error("key {0:?} may not sign this")]
    WrongKeyRole(String),
    /// The signature doesn't verify.
    #[error("bad signature")]
    BadSignature,
    /// The signed payload isn't canonical JSON (checked after the
    /// signature).
    #[error("signed payload: {0}")]
    NotCanonical(CanonicalError),
    /// The signed payload is canonical JSON but not an envelope.
    #[error("signed payload is not an envelope: {0}")]
    BadEnvelope(String),
    /// The key id inside the signed payload differs from the one that
    /// verified it.
    #[error("the signed key id differs from the signing key")]
    KeyIdMismatch,
    /// A file for another channel.
    #[error("a file for the {found} channel, expected {expected}")]
    WrongChannel {
        /// This app's channel.
        expected: &'static str,
        /// The file's channel.
        found: String,
    },
    /// Past its `expires_at`.
    #[error("expired at {expires_at} (now {now})")]
    Expired {
        /// The file's expiry, Unix seconds.
        expires_at: u64,
        /// The time checked, Unix seconds.
        now: u64,
    },
    /// `expires_at` not after `issued_at`, or further than
    /// [`crate::MAX_VALIDITY_SECS`] from it.
    #[error("the validity window is not allowed")]
    BadValidity,
    /// A serial below the highest accepted for this channel.
    #[error("serial {serial} is below the accepted {floor}")]
    Rollback {
        /// The file's serial.
        serial: u64,
        /// The highest serial accepted for the channel.
        floor: u64,
    },
    /// The accepted serial again, with different content.
    #[error("serial {0} was accepted with different content")]
    Equivocation(u64),
    /// The file's `min_app`/`max_app` leave this app out.
    #[error("the file does not apply to this app version")]
    NotForThisApp,
}
