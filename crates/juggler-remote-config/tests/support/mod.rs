//! Test keys and a test publisher. Every key is generated at test time
//! from the OS random source; no private key is ever committed.

#![allow(
    dead_code,
    clippy::pedantic,
    clippy::expect_used,
    reason = "each test binary uses a different part; test-only helpers"
)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signer as _, SigningKey};
use juggler_remote_config::canonical::to_canonical_bytes;
use juggler_remote_config::envelope::{CONFIG_CONTEXT, REVOCATIONS_CONTEXT, signing_message};
use juggler_remote_config::{
    Channel, KeyRing, KeyRole, PinnedKey, SignedFile, SignedPart, TrustState, VerifyContext,
    Version,
};
use serde_json::{Value, json};

/// A fixed "now" for the tests, Unix seconds.
pub const NOW: u64 = 1_800_000_000;
/// A day.
pub const DAY: u64 = 24 * 60 * 60;

pub const CURRENT: &str = "rc-current";
pub const NEXT: &str = "rc-next";
pub const EMERGENCY: &str = "rc-emergency";

pub fn fresh_key() -> SigningKey {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("random seed");
    SigningKey::from_bytes(&seed)
}

/// Three freshly generated keys and the ring pinning their public halves.
pub struct Lab {
    pub current: SigningKey,
    pub next: SigningKey,
    pub emergency: SigningKey,
    pub ring: KeyRing,
}

impl Lab {
    pub fn new() -> Self {
        let (current, next, emergency) = (fresh_key(), fresh_key(), fresh_key());
        let pin = |id, role, key: &SigningKey| {
            PinnedKey::new(id, role, &key.verifying_key().to_bytes()).expect("pin")
        };
        let ring = KeyRing::new(vec![
            pin(CURRENT, KeyRole::Current, &current),
            pin(NEXT, KeyRole::Next, &next),
            pin(EMERGENCY, KeyRole::Emergency, &emergency),
        ])
        .expect("ring");
        Self {
            current,
            next,
            emergency,
            ring,
        }
    }

    pub fn ctx<'a>(&'a self, channel: Channel, app: &'a Version, now: u64) -> VerifyContext<'a> {
        VerifyContext {
            keys: &self.ring,
            channel,
            app_version: app,
            now,
        }
    }

    /// A signed config part over the canonical bytes of `envelope`.
    pub fn config_part(&self, key: &SigningKey, key_id: &str, envelope: &Value) -> SignedPart {
        sign_part(
            key,
            key_id,
            CONFIG_CONTEXT,
            &to_canonical_bytes(envelope).expect("canonical"),
        )
    }

    /// A revocation list signed by the emergency key.
    pub fn revocations(&self, revoked: &[&str]) -> SignedPart {
        let payload =
            json!({"format": 1, "key_id": EMERGENCY, "issued_at": NOW, "revoked": revoked});
        sign_part(
            &self.emergency,
            EMERGENCY,
            REVOCATIONS_CONTEXT,
            &to_canonical_bytes(&payload).expect("canonical"),
        )
    }

    /// A file signed by the current key.
    pub fn file(&self, envelope: &Value) -> Vec<u8> {
        wrap(
            Some(self.config_part(&self.current, CURRENT, envelope)),
            None,
        )
    }
}

pub fn sign_part(key: &SigningKey, key_id: &str, context: &[u8], payload: &[u8]) -> SignedPart {
    let signature = key.sign(&signing_message(context, payload));
    SignedPart {
        key_id: key_id.to_owned(),
        payload: BASE64.encode(payload),
        signature: BASE64.encode(signature.to_bytes()),
    }
}

pub fn wrap(config: Option<SignedPart>, revocations: Option<SignedPart>) -> Vec<u8> {
    serde_json::to_vec(&SignedFile {
        format: 1,
        config,
        revocations,
    })
    .expect("json")
}

/// An envelope for `channel` with `serial`, valid for 14 days from an hour
/// before [`NOW`], signed (in its `key_id`) by `key_id`.
pub fn envelope(key_id: &str, channel: &str, serial: u64, sections: Value) -> Value {
    json!({
        "format": 1,
        "key_id": key_id,
        "channel": channel,
        "serial": serial,
        "issued_at": NOW - 3600,
        "expires_at": NOW - 3600 + 14 * DAY,
        "sections": sections,
    })
}

pub fn app() -> Version {
    "0.5.0".parse().expect("version")
}

/// Accepts `file` into `state` as the fetcher would.
pub fn accept(lab: &Lab, state: &mut TrustState, file: &[u8]) {
    let app = app();
    let checked =
        juggler_remote_config::check_file(file, &lab.ctx(Channel::Stable, &app, NOW), state);
    state.revoked.extend(checked.newly_revoked);
    let verified = checked.result.expect("accepted");
    juggler_remote_config::envelope::record_accepted(state, Channel::Stable, &verified, file)
        .expect("recorded");
}
