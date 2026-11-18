//! AJ-E2023: what the remote file may do to a flag is what the flag
//! declares, and with no usable file every flag is its compiled default.

use std::sync::Arc;

use juggler_remote_config::{
    Channel, Endpoints, FetchPolicy, Fetcher, KeyRing, MemoryStore, RecordingTransport, Registry,
    RemoteMode, Version,
};
use serde_json::json;

use super::{DECLARED, Flag, on_in, section};
use crate::features::remote_config::Snapshot;
use crate::features::remote_config::dto::{RemoteSection, RemoteValuesInfo};
use crate::features::remote_config::snapshot_fixtures::{facts, file, registry};

const KILL: Flag = Flag {
    name: "test_kill",
    default: true,
    remote: RemoteMode::OffOnly,
};
const DARK: Flag = Flag {
    name: "test_dark",
    default: false,
    remote: RemoteMode::OffOnly,
};
const TOGGLE: Flag = Flag {
    name: "test_toggle",
    default: false,
    remote: RemoteMode::OnOff,
};
const ROLLOUT: Flag = Flag {
    name: "test_rollout",
    default: false,
    remote: RemoteMode::Rollout,
};

fn test_registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(section(&[&KILL, &DARK, &TOGGLE, &ROLLOUT]).unwrap())
        .unwrap();
    registry
}

/// The flags in effect for install `install`, given the file's values.
fn with(values: serde_json::Value, install: u8) -> Snapshot {
    let file = file(json!({ "flags": { "schema": 1, "values": values } }));
    Snapshot::build(&test_registry(), Some(&file), &facts(install))
}

#[test]
fn an_off_only_flag_is_never_turned_on_remotely() {
    // A compiled-off feature stays off whatever the file says, plainly or
    // in a rule, bucketed or not.
    for value in [
        json!(true),
        json!({ "rules": [{ "value": true }] }),
        json!({ "rules": [{ "when": { "buckets": [0, 99] }, "value": true }] }),
    ] {
        let snapshot = with(json!({ "test_dark": value.clone() }), 1);
        assert!(!on_in(&snapshot.effective, &DARK), "{value}");
    }
    // Its kill switch works.
    let snapshot = with(json!({ "test_kill": false }), 1);
    assert!(!on_in(&snapshot.effective, &KILL));
    // And "on" for a kill switch that's on by default changes nothing.
    let snapshot = with(json!({ "test_kill": true }), 1);
    assert!(on_in(&snapshot.effective, &KILL));
}

#[test]
fn an_on_off_flag_follows_the_file() {
    assert!(on_in(
        &with(json!({ "test_toggle": true }), 1).effective,
        &TOGGLE
    ));
    assert!(!on_in(
        &with(json!({ "test_toggle": false }), 1).effective,
        &TOGGLE
    ));
    // Targeted: on only where the rule matches.
    let stable_only = json!({ "test_toggle": { "rules": [{ "when": { "channels": ["beta"] }, "value": true }] } });
    assert!(!on_in(&with(stable_only, 1).effective, &TOGGLE));
}

#[test]
fn a_rollout_turns_on_only_for_its_buckets() {
    // On for everyone is refused: a rollout turns on only by bucket.
    assert!(!on_in(
        &with(json!({ "test_rollout": true }), 1).effective,
        &ROLLOUT
    ));
    // The bucket is the crate's, salted with `flags.<name>`, from the
    // install's own random id.
    let bucket = facts(1).install.bucket("flags.test_rollout");
    let only_mine = json!({ "test_rollout": { "rules": [{ "when": { "buckets": [bucket, bucket] }, "value": true }] } });
    assert!(on_in(&with(only_mine.clone(), 1).effective, &ROLLOUT));
    // Another install, in another bucket, stays off.
    let other = (2..=255u8)
        .find(|i| facts(*i).install.bucket("flags.test_rollout") != bucket)
        .unwrap();
    assert!(!on_in(&with(only_mine, other).effective, &ROLLOUT));
    // A 100 % rollout reaches everyone; off turns it off.
    let all =
        json!({ "test_rollout": { "rules": [{ "when": { "buckets": [0, 99] }, "value": true }] } });
    assert!(on_in(&with(all, other).effective, &ROLLOUT));
    assert!(!on_in(
        &with(json!({ "test_rollout": false }), 1).effective,
        &ROLLOUT
    ));
}

#[test]
fn the_rollout_bucket_is_stable() {
    let ten =
        json!({ "test_rollout": { "rules": [{ "when": { "buckets": [0, 9] }, "value": true }] } });
    for install in 0..=255u8 {
        let first = on_in(&with(ten.clone(), install).effective, &ROLLOUT);
        // The same install, read again (a new launch makes the same id
        // from its saved bytes), lands the same way.
        let again = on_in(&with(ten.clone(), install).effective, &ROLLOUT);
        assert_eq!(first, again, "install {install}");
        assert_eq!(
            first,
            facts(install).install.bucket("flags.test_rollout") < 10,
            "install {install}"
        );
    }
}

#[test]
fn a_missing_or_invalid_file_gives_the_compiled_defaults() {
    let registry = test_registry();
    let defaults = Snapshot::build(&registry, None, &facts(1));
    for flag in [KILL, DARK, TOGGLE, ROLLOUT] {
        assert_eq!(
            on_in(&defaults.effective, &flag),
            flag.default,
            "{}",
            flag.name
        );
    }
    // A section that's broken, newer or full of values this app can't read.
    for flags in [
        json!("not an object"),
        json!({ "schema": 1, "values": [1, 2] }),
        json!({ "schema": 1, "values": { "test_toggle": "yes", "test_kill": 0 } }),
        json!({ "schema": 9, "values": { "test_toggle": true } }),
    ] {
        let file = file(json!({ "flags": flags.clone() }));
        let snapshot = Snapshot::build(&registry, Some(&file), &facts(1));
        assert_eq!(snapshot.effective, defaults.effective, "{flags}");
    }
    // A file that doesn't verify, or isn't there, is never used: the
    // fetcher gives no file, so the compiled defaults apply.
    let transport = Arc::new(RecordingTransport::default());
    transport.respond(
        200,
        br#"{"flags":{"schema":1,"values":{"test_toggle":true}}}"#,
    );
    transport.respond(404, b"");
    let fetcher = Fetcher::new(
        transport,
        Endpoints::new(["https://config.figurehead.ai"]).unwrap(),
        KeyRing::default(),
        Arc::new(MemoryStore::default()),
        FetchPolicy {
            min_interval: std::time::Duration::ZERO,
            ..FetchPolicy::default()
        },
    );
    let version: Version = "0.5.0".parse().unwrap();
    for now in [1_800_000_000, 1_800_000_001] {
        let refresh = fetcher.refresh(&juggler_remote_config::AppContext {
            channel: Channel::Stable,
            app_version: &version,
            now,
        });
        assert!(refresh.file.is_none(), "{:?}", refresh.attempt);
        let snapshot = Snapshot::build(&registry, refresh.file.as_ref(), &facts(1));
        assert_eq!(snapshot.effective, defaults.effective);
    }
}

#[test]
fn the_apps_flags_are_declared_once_and_registered() {
    assert!(section(&[&KILL, &KILL]).is_err(), "a name declared twice");
    // The app's own registry carries every declared flag at its default.
    let snapshot = Snapshot::build(&registry(), None, &facts(1));
    for flag in DECLARED {
        assert_eq!(
            snapshot.effective.switch(super::SECTION, flag.name),
            Some(flag.default),
            "{}",
            flag.name
        );
    }
    // useRemote("flags") lists them.
    let RemoteValuesInfo::Flags { flags } = snapshot.values(RemoteSection::Flags);
    assert_eq!(flags.len(), DECLARED.len());
    assert_eq!(flags.get("notices_on"), Some(&true));
}
