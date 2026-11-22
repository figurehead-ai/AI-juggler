//! ADR-0035 §8: the publisher's tool. Sign then verify round-trips with
//! the app's own checks; a key the app doesn't pin, or the emergency key,
//! is refused; the source is held to the exported schema; the live file's
//! serial and revocations carry over.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

mod support;

use juggler_remote_config::publish::{
    self, PublishError, Source, Stamp, check_source, read_schema, read_source,
};
use juggler_remote_config::{
    Channel, Control, Facts, InstallId, Os, Registry, Rejection, RemoteMode, SectionSpec, Setting,
    SettingKey, VersionRange,
};
use serde_json::json;
use support::{DAY, Lab, NOW, app, fresh_key};

fn v(text: &str) -> juggler_remote_config::Version {
    text.parse().unwrap()
}

/// A made-up module's section; this crate knows no real feature.
fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(
            SectionSpec::new("compat", 1)
                .switch("tool_move_on", true, RemoteMode::OffOnly)
                .control(
                    "tool_move_versions",
                    Control::Versions {
                        default: VersionRange::new(Some(v("1.0.0")), None),
                        bounds: VersionRange::new(Some(v("1.0.0")), None),
                    },
                ),
        )
        .unwrap();
    registry
}

fn source(sections: serde_json::Value) -> Source {
    read_source(
        serde_json::to_vec(&json!({"channel": "stable", "sections": sections}))
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}

fn kill() -> Source {
    source(json!({"compat": {"schema": 1, "values": {"tool_move_on": false}}}))
}

fn stamp(serial: u64) -> Stamp {
    Stamp::new(serial, NOW, 14).unwrap()
}

#[test]
fn sign_then_verify_round_trips_through_the_apps_checks() {
    let lab = Lab::new();
    let file = publish::sign(&kill(), &stamp(5), &lab.current, &lab.ring, &[]).unwrap();
    let verified = publish::verify(&file, &lab.ring, Channel::Stable, &app(), NOW + DAY).unwrap();
    assert_eq!(verified.envelope.serial, 5);
    assert_eq!(verified.envelope.key_id, support::CURRENT);
    assert_eq!(
        verified.envelope.expires_at - verified.envelope.issued_at,
        14 * DAY
    );
    // What the app then reads from it: the kill.
    let facts = Facts {
        app_version: app(),
        channel: Channel::Stable,
        os: Os::Macos,
        os_version: None,
        tools: Default::default(),
        install: InstallId::from_bytes([1; 16]),
    };
    let (values, issues) = registry().resolve(&verified.envelope.sections, &facts);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(
        values[&SettingKey::new("compat", "tool_move_on")],
        Setting::Switch(false)
    );
    // `next` signs too (a rotation).
    let file = publish::sign(&kill(), &stamp(6), &lab.next, &lab.ring, &[]).unwrap();
    assert!(publish::verify(&file, &lab.ring, Channel::Stable, &app(), NOW).is_ok());
}

#[test]
fn a_wrong_key_is_refused() {
    let lab = Lab::new();
    // A key this build doesn't pin can't sign…
    let stranger = fresh_key();
    assert!(matches!(
        publish::sign(&kill(), &stamp(1), &stranger, &lab.ring, &[]),
        Err(PublishError::Key(_))
    ));
    // …nor can the emergency key, which signs only revocations.
    assert!(matches!(
        publish::sign(&kill(), &stamp(1), &lab.emergency, &lab.ring, &[]),
        Err(PublishError::Key(_))
    ));
    // A file signed for another build's keys doesn't verify here.
    let other = Lab::new();
    let file = publish::sign(&kill(), &stamp(1), &other.current, &other.ring, &[]).unwrap();
    assert_eq!(
        publish::verify(&file, &lab.ring, Channel::Stable, &app(), NOW),
        Err(PublishError::Rejected(Rejection::BadSignature))
    );
    // And a build with no pinned keys (today's) verifies nothing.
    assert!(matches!(
        publish::verify(&file, &Default::default(), Channel::Stable, &app(), NOW),
        Err(PublishError::Rejected(Rejection::UnknownKey(_)))
    ));
}

#[test]
fn the_app_rejects_what_it_would_reject() {
    let lab = Lab::new();
    let file = publish::sign(&kill(), &stamp(1), &lab.current, &lab.ring, &[]).unwrap();
    assert!(matches!(
        publish::verify(&file, &lab.ring, Channel::Beta, &app(), NOW),
        Err(PublishError::Rejected(Rejection::WrongChannel { .. }))
    ));
    assert!(matches!(
        publish::verify(&file, &lab.ring, Channel::Stable, &app(), NOW + 15 * DAY),
        Err(PublishError::Rejected(Rejection::Expired { .. }))
    ));
    // Inspect still reads an expired live file's serial (for the next one).
    let live = publish::inspect(&file, &lab.ring, Channel::Stable).unwrap();
    assert_eq!(live.envelope.serial, 1);
    assert_eq!(publish::next_serial(Some(&live), Channel::Stable), 2);
    assert_eq!(publish::next_serial(None, Channel::Stable), 1);
    // A tampered file is not inspected.
    let mut tampered = String::from_utf8(file).unwrap();
    tampered = tampered.replacen("\"signature\":\"", "\"signature\":\"AAAA", 1);
    assert!(publish::inspect(tampered.as_bytes(), &lab.ring, Channel::Stable).is_err());
}

#[test]
fn the_live_files_revocations_carry_over() {
    let lab = Lab::new();
    let live = support::wrap(
        Some(lab.config_part(
            &lab.current,
            support::CURRENT,
            &support::envelope(support::CURRENT, "stable", 1, json!({})),
        )),
        Some(lab.revocations(&["0".repeat(64).as_str()])),
    );
    let carried = publish::revocations_of(&live).unwrap();
    assert_eq!(carried.len(), 1);
    let file = publish::sign(&kill(), &stamp(2), &lab.current, &lab.ring, &carried).unwrap();
    assert_eq!(publish::revocations_of(&file).unwrap(), carried);
    assert!(publish::verify(&file, &lab.ring, Channel::Stable, &app(), NOW).is_ok());
}

#[test]
fn the_source_is_held_to_the_exported_schema() {
    // The schema round-trips through its export, as the workflow reads it.
    let exported = serde_json::to_vec(&registry().export()).unwrap();
    let schema = read_schema(&exported).unwrap();
    assert!(check_source(&kill(), &schema, Channel::Stable).is_ok());
    let invalid = [
        // A kill switch turned on.
        json!({"compat": {"schema": 1, "values": {"tool_move_on": true}}}),
        // A range past the compiled bounds (the app would clamp it).
        json!({"compat": {"schema": 1, "values": {"tool_move_versions": {"min": "0.5.0"}}}}),
        // A control or section no module reads.
        json!({"compat": {"schema": 1, "values": {"tool_mvoe_on": false}}}),
        json!({"compta": {"schema": 1, "values": {}}}),
        // A newer schema than the app's, or none.
        json!({"compat": {"schema": 2, "values": {}}}),
        json!({"compat": {"values": {}}}),
        // A `when` the app can't read.
        json!({"compat": {"schema": 1, "values": {"tool_move_on":
            {"rules": [{"when": {"region": "eu"}, "value": false}]}}}}),
    ];
    for sections in invalid {
        assert!(
            matches!(check_source(&source(sections.clone()), &schema, Channel::Stable),
                Err(PublishError::Invalid(ref problems)) if !problems.is_empty()),
            "{sections}"
        );
    }
    assert!(check_source(&kill(), &schema, Channel::Beta).is_err());
    // Unknown fields in the source are refused, not ignored.
    assert!(read_source(br#"{"channel": "stable", "sections": {}, "serial": 9}"#).is_err());
    // A schema that breaks its own declarations is refused.
    let broken = json!({"format": 1, "sections": [{"name": "compat", "schema": 1, "controls": {
        "x": {"number": {"default": 9, "min": 0, "max": 5}}}}]});
    assert!(read_schema(&serde_json::to_vec(&broken).unwrap()).is_err());
}

/// The weekly re-sign, unattended: only the serial and the dates change.
#[test]
fn a_resign_bumps_only_the_serial_and_expiry() {
    let lab = Lab::new();
    let live = publish::sign(&kill(), &stamp(4), &lab.current, &lab.ring, &[]).unwrap();
    let resigned = publish::resign(
        &live,
        &lab.current,
        &lab.ring,
        Channel::Stable,
        NOW + 7 * DAY,
        14,
    )
    .unwrap();
    let before = publish::inspect(&live, &lab.ring, Channel::Stable).unwrap();
    let after = publish::check_resign(&live, &resigned, &lab.ring, Channel::Stable).unwrap();
    assert_eq!(after.envelope.serial, 5);
    assert_eq!(after.envelope.expires_at, NOW + 21 * DAY);
    assert_eq!(after.envelope.sections, before.envelope.sections);
    assert!(publish::verify(&resigned, &lab.ring, Channel::Stable, &app(), NOW + 8 * DAY).is_ok());
    // Even an expired live file is re-signed as it was.
    assert!(
        publish::resign(
            &live,
            &lab.current,
            &lab.ring,
            Channel::Stable,
            NOW + 20 * DAY,
            14
        )
        .is_ok()
    );
}

#[test]
fn a_resign_refuses_changed_content() {
    let lab = Lab::new();
    let live = publish::sign(&kill(), &stamp(4), &lab.current, &lab.ring, &[]).unwrap();
    let bumped = |source: &Source| {
        publish::sign(
            source,
            &Stamp::new(5, NOW + 7 * DAY, 14).unwrap(),
            &lab.current,
            &lab.ring,
            &[],
        )
        .unwrap()
    };
    // A changed section: a value turned around, a control added.
    for changed in [
        json!({"compat": {"schema": 1, "values": {"tool_move_on": true}}}),
        json!({"compat": {"schema": 1, "values": {"tool_move_on": false,
            "tool_move_versions": {"min": "1.0.0"}}}}),
    ] {
        let candidate = bumped(&source(changed.clone()));
        assert!(
            matches!(
                publish::check_resign(&live, &candidate, &lab.ring, Channel::Stable),
                Err(PublishError::Invalid(_))
            ),
            "{changed}"
        );
    }
    // One changed byte in a value ("1.0.0" → "1.0.1").
    let narrow = |max: &str| {
        source(json!({"compat": {"schema": 1, "values":
            {"tool_move_versions": {"min": "1.0.0", "max": max}}}}))
    };
    let live = bumped(&narrow("2.0.0"));
    let one_byte = publish::sign(
        &narrow("2.0.1"),
        &Stamp::new(6, NOW + 8 * DAY, 14).unwrap(),
        &lab.current,
        &lab.ring,
        &[],
    )
    .unwrap();
    assert!(matches!(
        publish::check_resign(&live, &one_byte, &lab.ring, Channel::Stable),
        Err(PublishError::Invalid(_))
    ));
    // The same content with a serial that doesn't rise is refused too.
    assert!(publish::check_resign(&live, &live, &lab.ring, Channel::Stable).is_err());
    // And a candidate signed by a key this build doesn't pin.
    let other = Lab::new();
    let forged = publish::sign(
        &narrow("2.0.0"),
        &stamp(9),
        &other.current,
        &other.ring,
        &[],
    )
    .unwrap();
    assert!(publish::check_resign(&live, &forged, &lab.ring, Channel::Stable).is_err());
}
