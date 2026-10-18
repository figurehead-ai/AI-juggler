//! AJ-E1202: the account client against every status code the Figurehead
//! API contract defines — over a recording transport (what was sent),
//! and once over the real transport against a local server (headers on
//! the wire).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::io::{Read, Write};
use std::sync::Arc;

use juggler_account::model::DeviceToken;
use juggler_account::transport::NetworkError;
use juggler_account::{
    AccountClient, ApiError, DeviceFields, PollOutcome, RecordingTransport, Transport,
    UreqTransport,
};

fn device() -> DeviceFields {
    DeviceFields {
        name: "Studio Mac".into(),
        platform: "macOS 26 · arm64".into(),
        app_version: "0.0.1".into(),
        public_key: "ed25519:AAAA;x25519:BBBB".into(),
    }
}

fn client(transport: &Arc<RecordingTransport>) -> AccountClient {
    AccountClient::new(
        "https://figurehead.test",
        transport.clone() as Arc<dyn Transport>,
    )
    .with_retry_delays(vec![std::time::Duration::ZERO; 3])
}

const LINK: &str = r#"{"requestId":"lnk_1","userCode":"QVM3-GABM","verifyUrl":"https://figurehead.test/link?code=QVM3-GABM","pollToken":"fhp_secret","expiresIn":600,"interval":3}"#;

#[test]
fn a_browser_link_starts_polls_and_delivers_the_token_once() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    transport.respond(201, LINK);
    let link = client.start_link(&device()).unwrap();
    assert_eq!(link.user_code, "QVM3-GABM");
    assert_eq!(link.interval_s, 3);
    let sent = &transport.requests()[0];
    assert_eq!(sent.method, "POST");
    assert_eq!(
        sent.url,
        "https://figurehead.test/api/v1/devices/link-requests"
    );
    assert_eq!(sent.content_type, "application/json");
    assert!(sent.bearer.is_none(), "starting a link is unauthenticated");
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(body["name"], "Studio Mac");
    assert_eq!(body["platform"], "macOS 26 · arm64");
    assert_eq!(body["appVersion"], "0.0.1");
    assert_eq!(body["publicKey"], "ed25519:AAAA;x25519:BBBB");

    transport.respond(200, r#"{"status":"pending"}"#);
    assert_eq!(client.poll_link(&link).unwrap(), PollOutcome::Pending);
    let poll = &transport.requests()[1];
    assert_eq!(poll.method, "GET");
    assert_eq!(
        poll.url,
        "https://figurehead.test/api/v1/devices/link-requests/lnk_1"
    );
    assert_eq!(
        poll.bearer.as_deref(),
        Some("fhp_secret"),
        "polls carry the poll token"
    );

    transport.respond(
        200,
        r#"{"status":"approved","deviceId":"dev_1","token":"fhd_secret"}"#,
    );
    match client.poll_link(&link).unwrap() {
        PollOutcome::Approved { device_id, token } => {
            assert_eq!(device_id, "dev_1");
            assert_eq!(token.expose(), "fhd_secret");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_poll_answer_maps_to_a_plain_outcome() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    transport.respond(201, LINK);
    let link = client.start_link(&device()).unwrap();
    for (status, body, expected) in [
        (410, r#"{"status":"denied"}"#, PollOutcome::Denied),
        (410, r#"{"status":"expired"}"#, PollOutcome::Expired),
        (
            429,
            r#"{"error":"slow_down","interval":6}"#,
            PollOutcome::SlowDown { interval_s: 6 },
        ),
        (404, r#"{"error":"not_found"}"#, PollOutcome::NotFound),
    ] {
        transport.respond(status, body);
        assert_eq!(
            client.poll_link(&link).unwrap(),
            expected,
            "{status} {body}"
        );
    }
}

#[test]
fn a_lost_poll_answer_is_never_retried() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    transport.respond(201, LINK);
    let link = client.start_link(&device()).unwrap();
    transport.push(Err(NetworkError("reset".into())));
    transport.respond(
        200,
        r#"{"status":"approved","deviceId":"dev_1","token":"fhd_x"}"#,
    );
    assert!(matches!(client.poll_link(&link), Err(ApiError::Network(_))));
    assert_eq!(
        transport.count(),
        2,
        "one start + one poll; no automatic re-poll"
    );
}

#[test]
fn a_pairing_code_is_normalised_and_a_bad_one_says_so() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    transport.respond(201, r#"{"deviceId":"dev_2","token":"fhd_two"}"#);
    let (device_id, token) = client.claim(" qvm3-gabm ", &device()).unwrap();
    assert_eq!(device_id, "dev_2");
    assert_eq!(token.expose(), "fhd_two");
    let body: serde_json::Value = serde_json::from_slice(&transport.requests()[0].body).unwrap();
    assert_eq!(body["code"], "QVM3GABM");
    assert_eq!(
        transport.requests()[0].url,
        "https://figurehead.test/api/v1/devices/claim"
    );

    transport.respond(400, r#"{"error":"invalid_or_expired_code"}"#);
    assert_eq!(
        client.claim("AAAA-BBBB", &device()),
        Err(ApiError::InvalidCode)
    );
    assert_eq!(client.claim("--", &device()), Err(ApiError::InvalidCode));
    assert_eq!(transport.count(), 2, "an empty code never leaves the Mac");
}

#[test]
fn the_account_drives_entitlements_and_unknown_keys_are_kept_not_trusted() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    let token = DeviceToken::new("fhd_t".into());
    transport.respond(
        200,
        r#"{"user":{"email":"a@example.com","name":"A"},"device":{"id":"dev_1","name":"Studio Mac"},
            "plan":{"id":"free","name":"Free"},
            "entitlements":{
              "cloudBackup":{"enabled":true,"tier":"free","maxBackups":30,"maxBytes":26214400},
              "transcriptVault":{"enabled":false,"tier":"paid"},
              "alerts":{"enabled":false,"tier":"free","available":false},
              "somethingNew":{"enabled":true,"tier":"free"}},
            "upgradeUrl":"https://figurehead.test/dashboard"}"#,
    );
    let account = client.account(&token).unwrap();
    assert_eq!(account.email(), Some("a@example.com"));
    assert_eq!(account.plan_name(), Some("Free"));
    assert!(account.allows("cloudBackup"));
    assert_eq!(account.entitlements["cloudBackup"].limits["maxBackups"], 30);
    assert!(account.entitlements["transcriptVault"].needs_upgrade());
    assert!(!account.allows("transcriptVault"));
    assert!(
        !account.entitlements["alerts"].available,
        "unavailable = hidden"
    );
    assert!(!account.fallback);
    let sent = &transport.requests()[0];
    assert_eq!(sent.bearer.as_deref(), Some("fhd_t"));

    transport.respond(404, r#"{"error":"not_found"}"#);
    let fallback = client.account(&token).unwrap();
    assert!(fallback.fallback);
    for feature in ["cloudBackup", "fleetDiscovery", "usageSync"] {
        assert!(fallback.allows(feature), "{feature} on by Phase-1 default");
    }
    assert!(!fallback.allows("transcriptVault"));
}

#[test]
fn unlinked_and_upgrade_required_are_typed() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    let token = DeviceToken::new("fhd_t".into());
    transport.respond(401, r#"{"error":"unauthorized"}"#);
    assert_eq!(client.account(&token), Err(ApiError::Unlinked));
    transport.respond(
        402,
        r#"{"error":"upgrade_required","feature":"cloudBackup","upgradeUrl":"https://figurehead.test/dashboard"}"#,
    );
    assert_eq!(
        client.account(&token),
        Err(ApiError::UpgradeRequired {
            feature: "cloudBackup".into(),
            upgrade_url: Some("https://figurehead.test/dashboard".into()),
        })
    );
}

#[test]
fn network_errors_and_5xx_back_off_a_bounded_number_of_times() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    let token = DeviceToken::new("fhd_t".into());
    transport.respond(503, "");
    transport.push(Err(NetworkError("offline".into())));
    transport.respond(200, r#"{"entitlements":{}}"#);
    assert!(client.account(&token).is_ok());
    assert_eq!(transport.count(), 3);
    for _ in 0..4 {
        transport.push(Err(NetworkError("offline".into())));
    }
    assert!(matches!(client.account(&token), Err(ApiError::Network(_))));
    assert_eq!(transport.count(), 7, "1 try + 3 retries, then it gives up");
}

#[test]
fn rename_and_unlink_hit_devices_self_with_json_even_without_a_body() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    let token = DeviceToken::new("fhd_t".into());
    transport.respond(200, r#"{"id":"dev_1","name":"Mini","appVersion":"0.0.1"}"#);
    client.rename(&token, " Mini ", "0.0.1").unwrap();
    transport.respond(204, "");
    client.unlink(&token).unwrap();
    let requests = transport.requests();
    assert_eq!(requests[0].method, "PATCH");
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["name"], "Mini");
    assert_eq!(requests[1].method, "DELETE");
    assert_eq!(
        requests[1].url,
        "https://figurehead.test/api/v1/devices/self"
    );
    assert_eq!(requests[1].content_type, "application/json");
    assert!(requests[1].body.is_empty());
    // An already-unlinked Mac is unlinked: fine.
    transport.respond(401, "");
    assert!(client.unlink(&token).is_ok());
}

#[test]
fn no_debug_output_ever_shows_a_secret() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    transport.respond(201, LINK);
    let link = client.start_link(&device()).unwrap();
    transport.respond(
        200,
        r#"{"status":"approved","deviceId":"dev_1","token":"fhd_secret"}"#,
    );
    let outcome = client.poll_link(&link).unwrap();
    let printed = format!("{link:?} {outcome:?} {:?} {client:?}", transport.requests());
    assert!(!printed.contains("fhp_"), "{printed}");
    assert!(!printed.contains("fhd_"), "{printed}");
}

#[test]
fn field_limits_are_checked_before_anything_is_sent() {
    let transport = Arc::new(RecordingTransport::default());
    let client = client(&transport);
    let mut long = device();
    long.name = "x".repeat(65);
    assert!(matches!(
        client.start_link(&long),
        Err(ApiError::Rejected { .. })
    ));
    assert_eq!(transport.count(), 0);
}

/// One request read off the wire by the local server.
fn serve_once(
    status_line: &'static str,
    body: &'static str,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut raw = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            raw.extend_from_slice(&buffer[..read]);
            if read == 0 || raw.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&raw).into_owned()
    });
    (base, handle)
}

#[test]
fn the_real_transport_sends_the_contract_headers() {
    let (base, server) = serve_once("HTTP/1.1 204 No Content", "");
    let client = AccountClient::new(&base, Arc::new(UreqTransport::new("9.9.9")));
    client.unlink(&DeviceToken::new("fhd_wire".into())).unwrap();
    let raw = server.join().unwrap().to_ascii_lowercase();
    assert!(
        raw.starts_with("delete /api/v1/devices/self http/1.1"),
        "{raw}"
    );
    assert!(raw.contains("user-agent: aijuggler/9.9.9"), "{raw}");
    assert!(
        raw.contains("content-type: application/json"),
        "bodiless DELETE still sends it: {raw}"
    );
    assert!(raw.contains("authorization: bearer fhd_wire"), "{raw}");
}

#[test]
fn the_real_transport_reports_an_unreachable_server_as_a_network_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = AccountClient::new(&base, Arc::new(UreqTransport::new("9.9.9")))
        .with_retry_delays(Vec::new());
    let error = client
        .account(&DeviceToken::new("fhd_x".into()))
        .unwrap_err();
    assert!(matches!(error, ApiError::Network(_)), "{error:?}");
}
