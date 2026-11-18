//! ADR-0035 §7 and §10: the anonymous daily fetch, the cache, every
//! fallback, and a recorded transport replayed.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_remote_config::state::StoreError;
use juggler_remote_config::{
    AppContext, Attempt, Baseline, Channel, ConfigRequest, ConfigResponse, Endpoints, Failure,
    FetchPolicy, Fetcher, MemoryStore, NetworkError, Recorder, RecordingTransport, Rejection,
    ReplayTransport, StateStore, Transport, TrustState, Version,
};
use serde_json::json;
use support::{CURRENT, DAY, Lab, NEXT, NOW, envelope};

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
    let mut state = rig.store.load().unwrap().unwrap();
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
    file["revocations"] = json!([rig.lab.revocations(&[CURRENT])]);
    file["config"]["signature"] = json!("AAAA");
    rig.transport
        .respond(200, &serde_json::to_vec(&file).unwrap());
    let refresh = rig.fetcher.refresh(&rig.at(NOW + DAY));
    assert!(matches!(
        refresh.attempt,
        Attempt::Failed(Failure::Rejected(_))
    ));
    assert_eq!(refresh.file, None, "the cached file's key is now revoked");
    assert!(
        rig.store
            .load()
            .unwrap()
            .unwrap()
            .revoked
            .contains(&rig.lab.print(CURRENT))
    );
}

#[test]
fn off_and_no_endpoint_send_nothing() {
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
    assert!(off.transport.requests().is_empty() && none.transport.requests().is_empty());
}

/// H1: an unreadable state fails safe: the compiled baseline's floor and
/// revocations hold, never "no floor".
#[test]
fn an_unreadable_state_falls_back_to_the_compiled_baseline() {
    struct Corrupt(MemoryStore);
    impl StateStore for Corrupt {
        fn load(&self) -> Result<Option<TrustState>, StoreError> {
            Err(StoreError::Corrupt("checksum".to_owned()))
        }
        fn save(&self, state: &TrustState) -> Result<(), StoreError> {
            self.0.save(state)
        }
    }
    let lab = Lab::new();
    let transport = Arc::new(RecordingTransport::default());
    let baseline = Baseline {
        floors: [(Channel::Stable, 10)].into(),
        revoked: [lab.print(NEXT)].into(),
    };
    let fetcher = Fetcher::new(
        transport.clone(),
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        Arc::new(Corrupt(MemoryStore::default())),
        FetchPolicy::default(),
    )
    .with_baseline(baseline);
    let app: Version = "0.5.0".parse().unwrap();
    let at = |now| AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now,
    };
    // An old, validly signed file below the compiled floor is refused.
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 9, json!({}))));
    assert_eq!(
        fetcher.refresh(&at(NOW)).attempt,
        Attempt::Failed(Failure::Rejected(Rejection::Rollback {
            serial: 9,
            floor: 10
        }))
    );
    // A compiled revocation holds too.
    let by_next = support::wrap(
        Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 11, json!({})))),
        None,
    );
    transport.respond(200, &by_next);
    assert_eq!(
        fetcher.refresh(&at(NOW + DAY)).attempt,
        Attempt::Failed(Failure::Rejected(Rejection::RevokedKey(NEXT.to_owned())))
    );
    // The floor's own file is accepted.
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 10, json!({}))));
    assert_eq!(
        fetcher.refresh(&at(NOW + 2 * DAY)).attempt,
        Attempt::Accepted { serial: 10 }
    );
}

/// L1: when the state can't be saved, nothing new is used (its floor and
/// revocations weren't kept), and the daily limit still holds in memory.
#[test]
fn a_store_that_cannot_save_serves_nothing_new() {
    struct ReadOnly(MemoryStore);
    impl StateStore for ReadOnly {
        fn load(&self) -> Result<Option<TrustState>, StoreError> {
            self.0.load()
        }
        fn save(&self, _: &TrustState) -> Result<(), StoreError> {
            Err(StoreError::Unavailable("disk full".to_owned()))
        }
    }
    let lab = Lab::new();
    let transport = Arc::new(RecordingTransport::default());
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 1, json!({}))));
    let fetcher = Fetcher::new(
        transport.clone(),
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        Arc::new(ReadOnly(MemoryStore::default())),
        FetchPolicy::default(),
    );
    let app: Version = "0.5.0".parse().unwrap();
    let at = |now| AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now,
    };
    let refresh = fetcher.refresh(&at(NOW));
    assert_eq!(refresh.file, None);
    assert!(matches!(
        refresh.attempt,
        Attempt::Failed(Failure::Store(_))
    ));
    assert_eq!(fetcher.refresh(&at(NOW + 60)).attempt, Attempt::NotDue);
    assert_eq!(transport.requests().len(), 1);
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

/// L5: an entity tag longer than 128 bytes is not kept or sent.
#[test]
fn an_overlong_entity_tag_is_not_kept() {
    let rig = Rig::with(
        FetchPolicy {
            conditional: true,
            ..FetchPolicy::default()
        },
        &[BASE],
    );
    rig.transport.push(Ok(ConfigResponse {
        status: 200,
        etag: Some("x".repeat(129)),
        body: rig.good(1),
    }));
    let _ = rig.fetcher.refresh(&rig.at(NOW));
    rig.transport.respond(304, b"");
    let _ = rig.fetcher.refresh(&rig.at(NOW + DAY));
    assert_eq!(rig.transport.requests()[1].if_none_match, None);
}

/// M2: two refreshes race over one store (two fetchers, so no shared
/// lock). The slow one fails and saves what it loaded before the fast one
/// accepted a newer file that revokes the old key: the floor and the
/// revocation survive, and the slow caller doesn't get the revoked file.
#[test]
fn racing_refreshes_never_lower_the_floor_or_lose_a_revocation() {
    use std::sync::mpsc;

    struct Gate {
        entered: Mutex<mpsc::Sender<()>>,
        release: Mutex<mpsc::Receiver<()>>,
    }
    impl Transport for Gate {
        fn get(&self, _: &ConfigRequest) -> Result<ConfigResponse, NetworkError> {
            self.entered.lock().unwrap().send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
            Err(NetworkError("slow, then down".to_owned()))
        }
    }

    let lab = Arc::new(Lab::new());
    let store = Arc::new(MemoryStore::default());
    let app: Version = "0.5.0".parse().unwrap();
    // Both start from serial 5, signed by the current key.
    let mut start = TrustState::default();
    support::accept(
        &lab,
        &mut start,
        &lab.file(&envelope(CURRENT, "stable", 5, json!({}))),
    );
    store.save(&start).unwrap();

    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let slow = Fetcher::new(
        Arc::new(Gate {
            entered: Mutex::new(entered_tx),
            release: Mutex::new(release_rx),
        }),
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        store.clone(),
        FetchPolicy::default(),
    );
    let fast_transport = Arc::new(RecordingTransport::default());
    fast_transport.respond(
        200,
        &support::wrap(
            Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 9, json!({})))),
            Some(lab.revocations(&[CURRENT])),
        ),
    );
    let fast = Fetcher::new(
        fast_transport,
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        store.clone(),
        FetchPolicy::default(),
    );

    let slow_result = std::thread::scope(|scope| {
        let slow_app = app.clone();
        let handle = scope.spawn(move || {
            slow.refresh(&AppContext {
                channel: Channel::Stable,
                app_version: &slow_app,
                now: NOW + DAY,
            })
        });
        entered_rx.recv().unwrap();
        let fast_result = fast.refresh(&AppContext {
            channel: Channel::Stable,
            app_version: &app,
            now: NOW + DAY,
        });
        assert_eq!(fast_result.attempt, Attempt::Accepted { serial: 9 });
        release_tx.send(()).unwrap();
        handle.join().unwrap()
    });

    let state = store.load().unwrap().unwrap();
    assert_eq!(state.floor(Channel::Stable), Some(9));
    assert!(state.revoked.contains(&lab.print(CURRENT)));
    assert_eq!(slow_result.file.map(|f| f.envelope.serial), Some(9));
}

/// M2: one fetcher serialises its own refreshes: the second waits for the
/// first, then finds it not due.
#[test]
fn one_fetcher_refreshes_one_at_a_time() {
    let rig = Rig::new();
    rig.transport.respond(200, &rig.good(1));
    rig.transport.respond(200, &rig.good(2));
    let attempts = std::thread::scope(|scope| {
        let a = scope.spawn(|| rig.fetcher.refresh(&rig.at(NOW)).attempt);
        let b = scope.spawn(|| rig.fetcher.refresh(&rig.at(NOW)).attempt);
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert!(attempts.contains(&Attempt::Accepted { serial: 1 }));
    assert!(attempts.contains(&Attempt::NotDue));
    assert_eq!(rig.transport.requests().len(), 1);
}

/// H1: the durable store. Missing is "nothing saved"; a corrupt or
/// tampered file is an error (the caller then uses the baseline); a save
/// merges forward; no temp file is left behind.
#[test]
fn the_file_store_is_durable_checksummed_and_forward_only() {
    use juggler_remote_config::FileStore;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("remote-config-state.json");
    let store = FileStore::new(&path);
    assert_eq!(store.load(), Ok(None));

    let lab = Lab::new();
    let mut high = TrustState::default();
    support::accept(
        &lab,
        &mut high,
        &lab.file(&envelope(CURRENT, "stable", 8, json!({}))),
    );
    store.save(&high).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap().floor(Channel::Stable),
        Some(8)
    );
    // A stale state saved later can't lower the floor.
    let mut low = TrustState::default();
    support::accept(
        &lab,
        &mut low,
        &lab.file(&envelope(CURRENT, "stable", 3, json!({}))),
    );
    store.save(&low).unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(loaded.floor(Channel::Stable), Some(8));
    assert_eq!(
        loaded.channels[&Channel::Stable]
            .accepted
            .as_ref()
            .unwrap()
            .serial,
        8
    );
    let temps = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(temps, 0, "no temp file left");

    // A flipped byte inside the state fails its checksum.
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("8", "9", 1)).unwrap();
    assert!(matches!(store.load(), Err(StoreError::Corrupt(_))));
    // Garbage, or a torn write, is corrupt too.
    std::fs::write(&path, &text.as_bytes()[..text.len() / 2]).unwrap();
    assert!(store.load().is_err());
    // A save over a corrupt file starts again from what it is given.
    store.save(&high).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap().floor(Channel::Stable),
        Some(8)
    );
}

/// N1: a state that can't be read *now* is not corrupt. The refresh fails
/// closed (no request, no save, nothing new served, the last state this
/// process knew kept) and the next refresh reads it again.
#[test]
fn an_unreadable_state_fails_the_refresh_closed() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct Flaky {
        inner: MemoryStore,
        down: AtomicBool,
        saves: AtomicUsize,
    }
    impl StateStore for Flaky {
        fn load(&self) -> Result<Option<TrustState>, StoreError> {
            if self.down.load(Ordering::SeqCst) {
                Err(StoreError::Unavailable("EIO".to_owned()))
            } else {
                self.inner.load()
            }
        }
        fn save(&self, state: &TrustState) -> Result<(), StoreError> {
            self.saves.fetch_add(1, Ordering::SeqCst);
            self.inner.save(state)
        }
    }
    let lab = Lab::new();
    let store = Arc::new(Flaky {
        inner: MemoryStore::default(),
        down: AtomicBool::new(false),
        saves: AtomicUsize::new(0),
    });
    let transport = Arc::new(RecordingTransport::default());
    let fetcher = Fetcher::new(
        transport.clone(),
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        store.clone(),
        FetchPolicy::default(),
    );
    let app: Version = "0.5.0".parse().unwrap();
    let at = |now| AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now,
    };
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 8, json!({}))));
    assert_eq!(
        fetcher.refresh(&at(NOW)).attempt,
        Attempt::Accepted { serial: 8 }
    );
    let saves = store.saves.load(Ordering::SeqCst);

    store.down.store(true, Ordering::SeqCst);
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 9, json!({}))));
    let refresh = fetcher.refresh(&at(NOW + DAY));
    assert!(matches!(
        refresh.attempt,
        Attempt::Failed(Failure::Store(_))
    ));
    assert_eq!(
        refresh.file.map(|f| f.envelope.serial),
        Some(8),
        "the last known state is kept"
    );
    assert_eq!(transport.requests().len(), 1, "nothing fetched");
    assert_eq!(store.saves.load(Ordering::SeqCst), saves, "nothing saved");
    assert_eq!(
        fetcher.cached(&at(NOW + DAY)).map(|f| f.envelope.serial),
        Some(8)
    );

    // Readable again: the next refresh reads it and carries on.
    store.down.store(false, Ordering::SeqCst);
    assert_eq!(
        fetcher.refresh(&at(NOW + DAY)).attempt,
        Attempt::Accepted { serial: 9 }
    );
}

/// N1 (the reviewer's proof of concept): a state at floor 8 that has
/// revoked the current key is made unreadable, and one refresh runs. The
/// file must come back unchanged: the floor and the revocation survive,
/// so an older file from the revoked key still doesn't verify.
#[cfg(unix)]
#[test]
fn a_failed_read_never_wipes_the_saved_state() {
    use juggler_remote_config::FileStore;
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("remote-config-state.json");
    let store = Arc::new(FileStore::new(&path));
    let lab = Lab::new();
    let mut state = TrustState::default();
    support::accept(
        &lab,
        &mut state,
        &support::wrap(
            Some(lab.config_part(&lab.next, NEXT, &envelope(NEXT, "stable", 8, json!({})))),
            Some(lab.revocations(&[CURRENT])),
        ),
    );
    store.save(&state).unwrap();
    let before = std::fs::read(&path).unwrap();

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let readable_anyway = std::fs::read(&path).is_ok(); // e.g. running as root
    let transport = Arc::new(RecordingTransport::default());
    transport.respond(200, &lab.file(&envelope(CURRENT, "stable", 3, json!({}))));
    let fetcher = Fetcher::new(
        transport,
        Endpoints::new([BASE]).unwrap(),
        lab.ring.clone(),
        store.clone(),
        FetchPolicy::default(),
    );
    let app: Version = "0.5.0".parse().unwrap();
    let refresh = fetcher.refresh(&AppContext {
        channel: Channel::Stable,
        app_version: &app,
        now: NOW + DAY,
    });
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    if !readable_anyway {
        assert!(matches!(
            refresh.attempt,
            Attempt::Failed(Failure::Store(_))
        ));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "the file is untouched"
        );
    }
    let saved = store.load().unwrap().unwrap();
    assert_eq!(saved.floor(Channel::Stable), Some(8));
    assert!(saved.revoked.contains(&lab.print(CURRENT)));
}

/// N2: two stores on one path (as two app processes would be) can't lose
/// each other's updates: each read-modify-write holds the lock.
#[test]
fn two_file_stores_on_one_path_keep_every_update() {
    use juggler_remote_config::FileStore;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("remote-config-state.json");
    let rounds = 40u64;
    std::thread::scope(|scope| {
        for writer in ["a", "b"] {
            let store = FileStore::new(&path);
            scope.spawn(move || {
                for serial in 1..=rounds {
                    let mut state = TrustState::default();
                    state
                        .channels
                        .entry(Channel::Stable)
                        .or_default()
                        .serials
                        .insert(writer.repeat(64), serial);
                    state.revoked.insert(format!("{writer}-{serial}"));
                    store.save(&state).unwrap();
                }
            });
        }
    });
    let saved = FileStore::new(&path).load().unwrap().unwrap();
    let serials = &saved.channels[&Channel::Stable].serials;
    assert_eq!(serials["a".repeat(64).as_str()], rounds);
    assert_eq!(serials["b".repeat(64).as_str()], rounds);
    assert_eq!(
        saved.revoked.len(),
        2 * rounds as usize,
        "every revocation kept"
    );
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty());
}
