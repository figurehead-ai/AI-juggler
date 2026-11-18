//! ADR-0035 §7 and §10: the anonymous daily fetch, the cache, every
//! fallback, and a recorded transport replayed.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use juggler_remote_config::state::StoreError;
use juggler_remote_config::{
    AppContext, Attempt, Channel, ConfigRequest, ConfigResponse, Endpoints, Failure, FetchPolicy,
    Fetcher, MemoryStore, NetworkError, Recorder, RecordingTransport, Rejection, ReplayTransport,
    StateStore, Transport, TrustState, Version,
};
use serde_json::json;
use support::{CURRENT, DAY, Lab, NOW, envelope};

const BASE: &str = "https://config.example.test/aij";
const URL: &str = "https://config.example.test/aij/remote-config/v1/stable.json";

struct Rig {
    lab: Lab,
    transport: Arc<RecordingTransport>,
    store: Arc<MemoryStore>,
    fetcher: Fetcher,
    app: Version,
}

impl Rig {
    fn new() -> Self {
        Self::with(FetchPolicy::default(), &[BASE])
    }

    fn with(policy: FetchPolicy, bases: &[&str]) -> Self {
        let lab = Lab::new();
        let transport = Arc::new(RecordingTransport::default());
        let store = Arc::new(MemoryStore::default());
        let fetcher = Fetcher::new(
            transport.clone(),
            Endpoints::new(bases.iter().copied()).unwrap(),
            lab.ring.clone(),
            store.clone(),
            policy,
        );
        Self {
            lab,
            transport,
            store,
            fetcher,
            app: "0.5.0".parse().unwrap(),
        }
    }

    fn at(&self, now: u64) -> AppContext<'_> {
        AppContext {
            channel: Channel::Stable,
            app_version: &self.app,
            now,
        }
    }

    fn good(&self, serial: u64) -> Vec<u8> {
        self.lab
            .file(&envelope(CURRENT, "stable", serial, json!({})))
    }

    fn serial_in_use(&self, now: u64) -> Option<u64> {
        self.fetcher
            .refresh(&self.at(now))
            .file
            .map(|f| f.envelope.serial)
    }
}

#[test]
fn the_request_carries_nothing_identifying() {
    let rig = Rig::new();
    rig.transport.respond(200, &rig.good(1));
    let refresh = rig.fetcher.refresh(&rig.at(NOW));
    assert_eq!(refresh.attempt, Attempt::Accepted { serial: 1 });
    // The whole request: a fixed URL, no query, no conditional header.
    assert_eq!(
        rig.transport.requests(),
        vec![ConfigRequest {
            url: URL.to_owned(),
            if_none_match: None
        }]
    );
    // Even with an entity tag on hand, none is echoed by default.
    rig.transport.push(Ok(ConfigResponse {
        status: 200,
        etag: Some("\"unique-per-install\"".to_owned()),
        body: rig.good(2),
    }));
    let _ = rig.fetcher.refresh(&rig.at(NOW + DAY));
    rig.transport.respond(200, &rig.good(2));
    let _ = rig.fetcher.refresh(&rig.at(NOW + 2 * DAY));
    assert!(
        rig.transport
            .requests()
            .iter()
            .all(|r| r.url == URL && r.if_none_match.is_none())
    );
}

#[test]
fn it_fetches_at_most_daily() {
    let rig = Rig::new();
    rig.transport.respond(500, b"");
    let _ = rig.fetcher.refresh(&rig.at(NOW));
    assert_eq!(
        rig.fetcher.refresh(&rig.at(NOW + DAY - 1)).attempt,
        Attempt::NotDue
    );
    assert_eq!(
        rig.transport.requests().len(),
        1,
        "a failed attempt counts too"
    );
    rig.transport.respond(200, &rig.good(1));
    assert_eq!(
        rig.fetcher.refresh(&rig.at(NOW + DAY)).attempt,
        Attempt::Accepted { serial: 1 }
    );
    assert_eq!(rig.transport.requests().len(), 2);
    // A clock set back doesn't stall it.
    rig.transport.respond(200, &rig.good(2));
    assert_eq!(
        rig.fetcher.refresh(&rig.at(NOW - DAY)).attempt,
        Attempt::Accepted { serial: 2 }
    );
}

#[test]
fn every_failure_falls_back_to_the_compiled_defaults() {
    let lab = Lab::new();
    let expired = {
        let mut e = envelope(CURRENT, "stable", 1, json!({}));
        e["issued_at"] = json!(NOW - 20 * DAY);
        e["expires_at"] = json!(NOW - 6 * DAY);
        lab.file(&e)
    };
    let mut bad_signature = lab.file(&envelope(CURRENT, "stable", 1, json!({})));
    let at = bad_signature.iter().position(|&b| b == b'=').unwrap_or(40) - 3;
    bad_signature[at] = if bad_signature[at] == b'A' {
        b'B'
    } else {
        b'A'
    };
    let cases: Vec<(&str, Result<ConfigResponse, NetworkError>)> = vec![
        ("missing", Ok(response(404, b"not found"))),
        ("server error", Ok(response(503, b""))),
        ("no network", Err(NetworkError("offline".to_owned()))),
        ("invalid", Ok(response(200, b"{\"format\":1,\"config\":7}"))),
        (
            "unsigned",
            Ok(response(
                200,
                &serde_json::to_vec(&envelope(CURRENT, "stable", 1, json!({}))).unwrap(),
            )),
        ),
        ("expired", Ok(response(200, &expired))),
        ("bad signature", Ok(response(200, &bad_signature))),
        (
            "wrong channel",
            Ok(response(
                200,
                &lab.file(&envelope(CURRENT, "beta", 1, json!({}))),
            )),
        ),
        (
            "oversize",
            Ok(response(
                200,
                &vec![b' '; juggler_remote_config::MAX_FILE_BYTES + 1],
            )),
        ),
        (
            "truncated",
            Ok(response(
                200,
                &lab.file(&envelope(CURRENT, "stable", 1, json!({})))[..60],
            )),
        ),
        ("not modified, nothing cached", Ok(response(304, b""))),
    ];
    for (name, outcome) in cases {
        let transport = Arc::new(RecordingTransport::default());
        transport.push(outcome);
        let fetcher = Fetcher::new(
            transport,
            Endpoints::new([BASE]).unwrap(),
            lab.ring.clone(),
            Arc::new(MemoryStore::default()),
            FetchPolicy::default(),
        );
        let app: Version = "0.5.0".parse().unwrap();
        let refresh = fetcher.refresh(&AppContext {
            channel: Channel::Stable,
            app_version: &app,
            now: NOW,
        });
        assert_eq!(refresh.file, None, "{name}: compiled defaults");
        assert!(
            matches!(refresh.attempt, Attempt::Failed(_)),
            "{name}: {:?}",
            refresh.attempt
        );
    }
}

fn response(status: u16, body: &[u8]) -> ConfigResponse {
    ConfigResponse {
        status,
        etag: None,
        body: body.to_vec(),
    }
}

#[test]
fn a_failure_keeps_the_last_good_file_until_it_expires() {
    let rig = Rig::new();
    rig.transport.respond(200, &rig.good(3));
    assert_eq!(rig.serial_in_use(NOW), Some(3));
    for failure in [
        Err(NetworkError("offline".to_owned())),
        Ok(response(404, b"")),
        Ok(response(200, b"garbage")),
    ] {
        rig.transport.push(failure);
    }
    assert_eq!(rig.serial_in_use(NOW + DAY), Some(3));
    assert_eq!(rig.serial_in_use(NOW + 2 * DAY), Some(3));
    assert_eq!(rig.serial_in_use(NOW + 3 * DAY), Some(3));
    // A replay of an older file is refused and the newer stays.
    rig.transport.respond(200, &rig.good(2));
    let refresh = rig.fetcher.refresh(&rig.at(NOW + 4 * DAY));
    assert_eq!(
        refresh.attempt,
        Attempt::Failed(Failure::Rejected(Rejection::Rollback {
            serial: 2,
            floor: 3
        }))
    );
    assert_eq!(refresh.file.map(|f| f.envelope.serial), Some(3));
    // Past its expiry the cache is dropped: compiled defaults.
    assert_eq!(rig.fetcher.cached(&rig.at(NOW + 14 * DAY)), None);
    assert_eq!(rig.serial_in_use(NOW + 14 * DAY), None);
}

#[test]
fn the_cache_is_re_verified_on_load() {
    let rig = Rig::new();
    rig.transport.respond(200, &rig.good(3));
    let _ = rig.fetcher.refresh(&rig.at(NOW));
    assert_eq!(
        rig.fetcher.cached(&rig.at(NOW)).map(|f| f.envelope.serial),
        Some(3)
    );
    // Someone edits the cached file on disk.
    let mut state = rig.store.load().unwrap();
    let slot = state.channels.get_mut(&Channel::Stable).unwrap();
    let accepted = slot.accepted.as_mut().unwrap();
    accepted.file = accepted
        .file
        .replacen("\"payload\":\"", "\"payload\":\"A", 1);
    rig.store.save(&state).unwrap();
    assert_eq!(rig.fetcher.cached(&rig.at(NOW)), None);
}

#[test]
fn a_revocation_retires_the_cached_file() {
    let rig = Rig::new();
    rig.transport.respond(200, &rig.good(3));
    let _ = rig.fetcher.refresh(&rig.at(NOW));
    // A file whose config part is junk but whose revocation list is good.
    let mut file: serde_json::Value = serde_json::from_slice(&rig.good(4)).unwrap();
    file["revocations"] = serde_json::to_value(rig.lab.revocations(&[CURRENT])).unwrap();
    file["config"]["signature"] = json!("AAAA");
    rig.transport
        .respond(200, &serde_json::to_vec(&file).unwrap());
    let refresh = rig.fetcher.refresh(&rig.at(NOW + DAY));
    assert!(matches!(
        refresh.attempt,
        Attempt::Failed(Failure::Rejected(_))
    ));
    assert_eq!(refresh.file, None, "the cached file's key is now revoked");
    assert!(rig.store.load().unwrap().revoked.contains(CURRENT));
}

#[test]
fn off_no_endpoint_and_an_unreadable_store_send_nothing() {
    let off = Rig::with(
        FetchPolicy {
            enabled: false,
            ..FetchPolicy::default()
        },
        &[BASE],
    );
    assert_eq!(off.fetcher.refresh(&off.at(NOW)).attempt, Attempt::Disabled);
    assert_eq!(off.fetcher.cached(&off.at(NOW)), None);
    let none = Rig::with(FetchPolicy::default(), &[]);
    assert_eq!(
        none.fetcher.refresh(&none.at(NOW)).attempt,
        Attempt::NoEndpoint
    );

    struct Broken;
    impl StateStore for Broken {
        fn load(&self) -> Result<TrustState, StoreError> {
            Err(StoreError("corrupt".to_owned()))
        }
        fn save(&self, _: &TrustState) -> Result<(), StoreError> {
            Err(StoreError("read-only".to_owned()))
        }
    }
    let lab = Lab::new();
    let transport = Arc::new(RecordingTransport::default());
    let fetcher = Fetcher::new(
        transport.clone(),
        Endpoints::new([BASE]).unwrap(),
        lab.ring,
        Arc::new(Broken),
        FetchPolicy::default(),
    );
    let app: Version = "0.5.0".parse().unwrap();
    let refresh = fetcher.refresh(&AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now: NOW,
    });
    assert_eq!(refresh.file, None);
    assert_eq!(
        refresh.attempt,
        Attempt::Failed(Failure::Store("corrupt".to_owned()))
    );
    for rig_requests in [
        off.transport.requests(),
        none.transport.requests(),
        transport.requests(),
    ] {
        assert!(rig_requests.is_empty());
    }
}

#[test]
fn a_store_that_cannot_save_still_serves_the_fetched_file() {
    struct ReadOnly(MemoryStore);
    impl StateStore for ReadOnly {
        fn load(&self) -> Result<TrustState, StoreError> {
            self.0.load()
        }
        fn save(&self, _: &TrustState) -> Result<(), StoreError> {
            Err(StoreError("disk full".to_owned()))
        }
    }
    let lab = Lab::new();
    let transport = Arc::new(RecordingTransport::default());
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 1, json!({}))));
    let fetcher = Fetcher::new(
        transport,
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        Arc::new(ReadOnly(MemoryStore::default())),
        FetchPolicy::default(),
    );
    let app: Version = "0.5.0".parse().unwrap();
    let refresh = fetcher.refresh(&AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now: NOW,
    });
    assert_eq!(refresh.attempt, Attempt::Accepted { serial: 1 });
}

#[test]
fn a_second_endpoint_is_tried_when_the_first_fails() {
    let rig = Rig::with(
        FetchPolicy::default(),
        &["https://a.example.test", "https://b.example.test/"],
    );
    rig.transport.push(Err(NetworkError("timeout".to_owned())));
    rig.transport.respond(200, &rig.good(1));
    assert_eq!(
        rig.fetcher.refresh(&rig.at(NOW)).attempt,
        Attempt::Accepted { serial: 1 }
    );
    let urls: Vec<String> = rig
        .transport
        .requests()
        .into_iter()
        .map(|r| r.url)
        .collect();
    assert_eq!(
        urls,
        [
            "https://a.example.test/remote-config/v1/stable.json",
            "https://b.example.test/remote-config/v1/stable.json"
        ]
    );
}

#[test]
fn conditional_requests_when_allowed() {
    let rig = Rig::with(
        FetchPolicy {
            conditional: true,
            min_interval: Duration::from_hours(6),
            ..FetchPolicy::default()
        },
        &[BASE],
    );
    rig.transport.push(Ok(ConfigResponse {
        status: 200,
        etag: Some("\"v1\"".to_owned()),
        body: rig.good(1),
    }));
    let _ = rig.fetcher.refresh(&rig.at(NOW));
    rig.transport.respond(304, b"");
    let refresh = rig.fetcher.refresh(&rig.at(NOW + 6 * 3600));
    assert_eq!(refresh.attempt, Attempt::NotModified);
    assert_eq!(refresh.file.map(|f| f.envelope.serial), Some(1));
    assert_eq!(
        rig.transport.requests()[1].if_none_match.as_deref(),
        Some("\"v1\"")
    );
}

#[test]
fn endpoints_are_https_bases_only() {
    assert!(Endpoints::compiled().is_ok());
    for bad in [
        "http://config.example.test",
        "https://",
        "https:///path",
        "https://config.example.test/?id=1",
        "https://user@config.example.test",
        "https://config.example.test/#x",
        "ftp://config.example.test",
        "https://config.example.test/a b",
    ] {
        assert!(Endpoints::new([bad]).is_err(), "{bad} accepted");
    }
}

#[test]
fn the_transport_is_recorded_and_replayed() {
    // Record a session against a scripted "real" transport...
    let lab = Lab::new();
    let live = RecordingTransport::default();
    live.respond(404, b"");
    live.respond(200, &lab.file(&envelope(CURRENT, "stable", 9, json!({}))));
    let recorder = Arc::new(Recorder::new(live));
    let policy = FetchPolicy::default();
    let session = |transport: Arc<dyn Transport>| {
        let fetcher = Fetcher::new(
            transport,
            Endpoints::new([BASE]).unwrap(),
            lab.ring.clone(),
            Arc::new(MemoryStore::default()),
            policy,
        );
        let app: Version = "0.5.0".parse().unwrap();
        [NOW, NOW + DAY]
            .map(|now| {
                fetcher.refresh(&AppContext {
                    channel: Channel::Stable,
                    app_version: &app,
                    now,
                })
            })
            .map(|r| (r.attempt, r.file.map(|f| f.envelope.serial)))
    };
    let recorded = session(recorder.clone());
    let cassette = recorder.recording();
    assert_eq!(cassette.len(), 2);
    // ...save it, load it, and replay it: the same outcome, request for request.
    let saved = serde_json::to_string(&cassette).unwrap();
    let replay = Arc::new(ReplayTransport::new(serde_json::from_str(&saved).unwrap()));
    assert_eq!(session(replay.clone()), recorded);
    assert_eq!(replay.remaining(), 0);
    assert_eq!(recorded[1], (Attempt::Accepted { serial: 9 }, Some(9)));
    // A replay refuses a request that differs from the recording.
    let strict = ReplayTransport::new(cassette);
    let other = ConfigRequest {
        url: "https://elsewhere.example.test/x".to_owned(),
        if_none_match: None,
    };
    assert!(strict.get(&other).is_err());
}
