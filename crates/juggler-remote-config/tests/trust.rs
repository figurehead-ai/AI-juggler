//! ADR-0035 §3 and §10: signatures, keys, revocation, rollback, replay,
//! expiry, channel binding, and verification before parsing.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

mod support;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use juggler_remote_config::canonical::CanonicalError;
use juggler_remote_config::envelope::{CONFIG_CONTEXT, REVOCATIONS_CONTEXT};
use juggler_remote_config::{Channel, Rejection, SignedPart, TrustState, check_file};
use serde_json::json;
use support::{
    CURRENT, DAY, EMERGENCY, Lab, NEXT, NOW, accept, app, envelope, fresh_key, sign_part, wrap,
};

fn check(lab: &Lab, file: &[u8], state: &TrustState) -> Result<u64, Rejection> {
    let app = app();
    check_file(file, &lab.ctx(Channel::Stable, &app, NOW), state)
        .result
        .map(|v| v.envelope.serial)
}

fn stable(serial: u64) -> serde_json::Value {
    envelope(
        CURRENT,
        "stable",
        serial,
        json!({"flags": {"schema": 1, "values": {"x": false}}}),
    )
}

#[test]
fn a_good_signature_passes() {
    let lab = Lab::new();
    assert_eq!(
        check(&lab, &lab.file(&stable(7)), &TrustState::default()),
        Ok(7)
    );
    // The next key may sign configuration too (rotation).
    let file = wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 8, json!({})))),
        None,
    );
    assert_eq!(check(&lab, &file, &TrustState::default()), Ok(8));
}

#[test]
fn a_single_flipped_byte_fails() {
    let lab = Lab::new();
    let good = lab.config_part(&lab.current, CURRENT, &stable(7));
    let payload = BASE64.decode(&good.payload).unwrap();
    let signature = BASE64.decode(&good.signature).unwrap();
    // Every byte of the payload, one bit at a time in turn.
    for i in 0..payload.len() {
        let mut bad = payload.clone();
        bad[i] ^= 1 << (i % 8);
        let part = SignedPart {
            payload: BASE64.encode(&bad),
            ..good.clone()
        };
        assert_eq!(
            check(&lab, &wrap(Some(part), None), &TrustState::default()),
            Err(Rejection::BadSignature),
            "payload byte {i}"
        );
    }
    for i in 0..signature.len() {
        let mut bad = signature.clone();
        bad[i] ^= 0x80;
        let part = SignedPart {
            signature: BASE64.encode(&bad),
            ..good.clone()
        };
        assert!(
            check(&lab, &wrap(Some(part), None), &TrustState::default()).is_err(),
            "signature byte {i}"
        );
    }
}

#[test]
fn an_unknown_key_id_fails() {
    let lab = Lab::new();
    let stranger = fresh_key();
    let file = wrap(
        Some(lab.config_part(
            &stranger,
            "rc-stranger",
            &envelope("rc-stranger", "stable", 7, json!({})),
        )),
        None,
    );
    assert_eq!(
        check(&lab, &file, &TrustState::default()),
        Err(Rejection::UnknownKey("rc-stranger".to_owned()))
    );
    // A stranger's key under a pinned id is just a bad signature.
    let file = wrap(Some(lab.config_part(&stranger, CURRENT, &stable(7))), None);
    assert_eq!(
        check(&lab, &file, &TrustState::default()),
        Err(Rejection::BadSignature)
    );
}

#[test]
fn a_revoked_key_fails() {
    let lab = Lab::new();
    // The file itself carries the revocation of its own signer.
    let file = wrap(
        Some(lab.config_part(&lab.current, CURRENT, &stable(7))),
        Some(lab.revocations(&[CURRENT])),
    );
    let app = app();
    let checked = check_file(
        &file,
        &lab.ctx(Channel::Stable, &app, NOW),
        &TrustState::default(),
    );
    assert_eq!(
        checked.result,
        Err(Rejection::RevokedKey(CURRENT.to_owned()))
    );
    assert!(
        checked.newly_revoked.contains(CURRENT),
        "the revocation is kept"
    );

    // Remembered: a later file from that key, with no list, still fails.
    let mut state = TrustState::default();
    state.revoked.extend(checked.newly_revoked);
    assert_eq!(
        check(&lab, &lab.file(&stable(8)), &state),
        Err(Rejection::RevokedKey(CURRENT.to_owned()))
    );
    // The next key carries on.
    let file = wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 8, json!({})))),
        None,
    );
    assert_eq!(check(&lab, &file, &state), Ok(8));
}

#[test]
fn only_the_emergency_key_signs_revocations_and_it_signs_nothing_else() {
    let lab = Lab::new();
    // A revocation list signed by the current key.
    let payload =
        serde_json::to_vec(&json!({"format":1,"issued_at":NOW,"key_id":CURRENT,"revoked":[NEXT]}))
            .unwrap();
    let forged = sign_part(&lab.current, CURRENT, REVOCATIONS_CONTEXT, &payload);
    let file = wrap(
        Some(lab.config_part(&lab.current, CURRENT, &stable(7))),
        Some(forged),
    );
    assert_eq!(
        check(&lab, &file, &TrustState::default()),
        Err(Rejection::WrongKeyRole(CURRENT.to_owned()))
    );
    // A configuration signed by the emergency key.
    let file = wrap(
        Some(lab.config_part(
            &lab.emergency,
            EMERGENCY,
            &envelope(EMERGENCY, "stable", 7, json!({})),
        )),
        None,
    );
    assert_eq!(
        check(&lab, &file, &TrustState::default()),
        Err(Rejection::WrongKeyRole(EMERGENCY.to_owned()))
    );
    // A list can't revoke the emergency key itself.
    let file = wrap(
        Some(lab.config_part(&lab.current, CURRENT, &stable(7))),
        Some(lab.revocations(&[EMERGENCY])),
    );
    let mut state = TrustState::default();
    accept(&lab, &mut state, &file);
    let again = wrap(
        Some(lab.config_part(&lab.current, CURRENT, &stable(8))),
        Some(lab.revocations(&[NEXT])),
    );
    assert_eq!(
        check(&lab, &again, &state),
        Ok(8),
        "the emergency key still verifies"
    );
}

#[test]
fn signatures_are_bound_to_their_kind() {
    let lab = Lab::new();
    // The current key's signature over the envelope bytes, but in the
    // revocation context: it must not pass as a configuration signature.
    let payload = juggler_remote_config::canonical::to_canonical_bytes(&stable(7)).unwrap();
    let wrong_context = sign_part(&lab.current, CURRENT, REVOCATIONS_CONTEXT, &payload);
    assert_eq!(
        check(
            &lab,
            &wrap(Some(wrong_context), None),
            &TrustState::default()
        ),
        Err(Rejection::BadSignature)
    );
    let right = sign_part(&lab.current, CURRENT, CONFIG_CONTEXT, &payload);
    assert_eq!(
        check(&lab, &wrap(Some(right), None), &TrustState::default()),
        Ok(7)
    );
}

#[test]
fn a_lower_serial_is_rejected() {
    let lab = Lab::new();
    let mut state = TrustState::default();
    accept(&lab, &mut state, &lab.file(&stable(10)));
    assert_eq!(
        check(&lab, &lab.file(&stable(9)), &state),
        Err(Rejection::Rollback {
            serial: 9,
            floor: 10
        })
    );
    assert_eq!(check(&lab, &lab.file(&stable(11)), &state), Ok(11));
}

#[test]
fn an_older_file_replayed_after_a_newer_one_is_rejected() {
    let lab = Lab::new();
    let older = lab.file(&stable(5));
    let newer = lab.file(&stable(6));
    let mut state = TrustState::default();
    accept(&lab, &mut state, &older);
    accept(&lab, &mut state, &newer);
    assert_eq!(
        check(&lab, &older, &state),
        Err(Rejection::Rollback {
            serial: 5,
            floor: 6
        })
    );
    // The accepted file itself re-verifies (the cache)...
    assert_eq!(check(&lab, &newer, &state), Ok(6));
    // ...but other content under its serial is equivocation.
    let twin = lab.file(&envelope(
        CURRENT,
        "stable",
        6,
        json!({"flags": {"schema": 1}}),
    ));
    assert_eq!(check(&lab, &twin, &state), Err(Rejection::Equivocation(6)));
    // An older file signed by the next key is held back by the floor too.
    let other_key = wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 4, json!({})))),
        None,
    );
    assert_eq!(
        check(&lab, &other_key, &state),
        Err(Rejection::Rollback {
            serial: 4,
            floor: 6
        })
    );
}

#[test]
fn a_revoked_keys_burned_serial_no_longer_holds_the_floor() {
    let lab = Lab::new();
    let mut state = TrustState::default();
    accept(&lab, &mut state, &lab.file(&stable(5)));
    // A stolen current key burns a huge serial.
    accept(&lab, &mut state, &lab.file(&stable(9_000_000_000_000)));
    let blocked = wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 6, json!({})))),
        None,
    );
    assert!(matches!(
        check(&lab, &blocked, &state),
        Err(Rejection::Rollback { .. })
    ));
    // The emergency key revokes it; the next key publishes again.
    let recovered = wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 6, json!({})))),
        Some(lab.revocations(&[CURRENT])),
    );
    accept(&lab, &mut state, &recovered);
    assert_eq!(state.floor(Channel::Stable), Some(6));
}

#[test]
fn an_expired_file_is_rejected() {
    let lab = Lab::new();
    let file = lab.file(&stable(7));
    let app = app();
    let expires_at = NOW - 3600 + 14 * DAY;
    let at = |now| {
        check_file(
            &file,
            &lab.ctx(Channel::Stable, &app, now),
            &TrustState::default(),
        )
        .result
    };
    assert!(at(expires_at - 1).is_ok());
    assert_eq!(
        at(expires_at),
        Err(Rejection::Expired {
            expires_at,
            now: expires_at
        })
    );
    // A validity window over the cap, or reversed.
    let mut long = stable(7);
    long["expires_at"] = json!(NOW + 40 * DAY);
    assert_eq!(
        check(&lab, &lab.file(&long), &TrustState::default()),
        Err(Rejection::BadValidity)
    );
    let mut reversed = stable(7);
    reversed["expires_at"] = json!(NOW - 7200);
    assert_eq!(
        check(&lab, &lab.file(&reversed), &TrustState::default()),
        Err(Rejection::BadValidity)
    );
}

#[test]
fn the_wrong_channel_is_rejected() {
    let lab = Lab::new();
    let beta = lab.file(&envelope(CURRENT, "beta", 7, json!({})));
    assert_eq!(
        check(&lab, &beta, &TrustState::default()),
        Err(Rejection::WrongChannel {
            expected: "stable",
            found: "beta".to_owned()
        })
    );
    let app = app();
    assert!(
        check_file(
            &beta,
            &lab.ctx(Channel::Beta, &app, NOW),
            &TrustState::default()
        )
        .result
        .is_ok()
    );
}

#[test]
fn verification_happens_before_parsing() {
    let lab = Lab::new();
    // Payloads a parser would choke on, or that hold a malformed section;
    // under a bad signature none of them is ever parsed.
    let hostile: Vec<Vec<u8>> = vec![
        b"not json at all \xff\xfe".to_vec(),
        "[".repeat(100_000).into_bytes(),
        br#"{"sections":{"flags":{"schema":"one","values":[1,2,3]}},"serial":1.5e400}"#.to_vec(),
        br#"{"a":1,"a":2}"#.to_vec(),
    ];
    for payload in &hostile {
        // Signed by a key that isn't pinned under that id: bad signature.
        let part = sign_part(&fresh_key(), CURRENT, CONFIG_CONTEXT, payload);
        assert_eq!(
            check(&lab, &wrap(Some(part), None), &TrustState::default()),
            Err(Rejection::BadSignature)
        );
    }
    // The same payloads correctly signed get as far as the parser, which
    // refuses them: proof the order above is what kept them out.
    let part = sign_part(&lab.current, CURRENT, CONFIG_CONTEXT, &hostile[0]);
    assert!(matches!(
        check(&lab, &wrap(Some(part), None), &TrustState::default()),
        Err(Rejection::NotCanonical(CanonicalError::NotJson(_)))
    ));
    let part = sign_part(&lab.current, CURRENT, CONFIG_CONTEXT, &hostile[3]);
    assert_eq!(
        check(&lab, &wrap(Some(part), None), &TrustState::default()),
        Err(Rejection::NotCanonical(CanonicalError::NotCanonical))
    );
}

#[test]
fn a_signed_but_non_canonical_payload_is_rejected() {
    let lab = Lab::new();
    let pretty = serde_json::to_vec_pretty(&stable(7)).unwrap();
    let part = sign_part(&lab.current, CURRENT, CONFIG_CONTEXT, &pretty);
    assert_eq!(
        check(&lab, &wrap(Some(part), None), &TrustState::default()),
        Err(Rejection::NotCanonical(CanonicalError::NotCanonical))
    );
}

#[test]
fn unsigned_malformed_and_oversize_files_are_rejected() {
    let lab = Lab::new();
    let none = TrustState::default();
    assert_eq!(
        check(&lab, br#"{"format":1}"#, &none),
        Err(Rejection::Malformed("no signed configuration"))
    );
    assert!(matches!(
        check(&lab, b"<html>", &none),
        Err(Rejection::Malformed(_))
    ));
    let mut part = lab.config_part(&lab.current, CURRENT, &stable(7));
    part.signature = String::new();
    assert!(matches!(
        check(&lab, &wrap(Some(part), None), &none),
        Err(Rejection::Malformed(_))
    ));
    // The unsigned envelope served bare.
    let bare = serde_json::to_vec(&stable(7)).unwrap();
    assert!(matches!(
        check(&lab, &bare, &none),
        Err(Rejection::Malformed(_))
    ));
    let mut future = serde_json::to_value(
        serde_json::from_slice::<serde_json::Value>(&lab.file(&stable(7))).unwrap(),
    )
    .unwrap();
    future["format"] = json!(2);
    assert_eq!(
        check(&lab, &serde_json::to_vec(&future).unwrap(), &none),
        Err(Rejection::UnsupportedFormat(2))
    );
    let big = vec![b' '; juggler_remote_config::MAX_FILE_BYTES + 1];
    assert_eq!(
        check(&lab, &big, &none),
        Err(Rejection::TooLarge(big.len()))
    );
}

#[test]
fn the_signed_key_id_must_be_the_signer() {
    let lab = Lab::new();
    // Signed by current, but claiming next inside.
    let file = wrap(
        Some(lab.config_part(
            &lab.current,
            CURRENT,
            &envelope(NEXT, "stable", 7, json!({})),
        )),
        None,
    );
    assert_eq!(
        check(&lab, &file, &TrustState::default()),
        Err(Rejection::KeyIdMismatch)
    );
}

#[test]
fn the_app_version_range_is_honoured() {
    let lab = Lab::new();
    let mut ranged = stable(7);
    ranged["min_app"] = json!("0.5.1");
    assert_eq!(
        check(&lab, &lab.file(&ranged), &TrustState::default()),
        Err(Rejection::NotForThisApp)
    );
    ranged["min_app"] = json!("0.5.0");
    ranged["max_app"] = json!("0.6.0");
    assert_eq!(
        check(&lab, &lab.file(&ranged), &TrustState::default()),
        Ok(7)
    );
}
