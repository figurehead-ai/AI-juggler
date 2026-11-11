#![no_main]
//! Fuzz target: pairing (AJ-E705, AJ-E2008). The typed pairing code and
//! the other side's SPAKE2 message and confirmation come from outside the
//! Mac; parsing and checking them must never panic, and a made-up message
//! must never verify. Run: `cargo +nightly fuzz run pairing_messages`.

use juggler_link::pairing::{PairingCode, PairingSession, Role, Transcript};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = PairingCode::parse(text);
    }
    let Ok(code) = PairingCode::generate() else { return };
    let transcript = Transcript {
        host_key: [1; 32],
        joiner_key: [2; 32],
        host_machine_id: "host".to_owned(),
        joiner_machine_id: "joiner".to_owned(),
        protocol: (1, 0),
        tls_exporter: [3; 32],
    };
    let (mut session, _ours) = PairingSession::start(Role::Host, &code);
    if session.finish(data, &transcript).is_ok() {
        let mut theirs = [0_u8; 32];
        let n = data.len().min(32);
        theirs[..n].copy_from_slice(&data[..n]);
        assert!(session.verify(&theirs, &transcript).is_err(), "a made-up confirmation never verifies");
    }
});
