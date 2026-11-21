use super::*;

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener};

const NOW: i64 = 1_792_580_400_000;

fn ok(usages: &str, extra: &str) -> Vec<u8> {
    format!(
        r#"{{"code":0,"msg":"ok","request_id":"r1","data":{{"kind":"ok","quota":{{"usages":{usages},"extraUsage":{extra}}}}}}}"#
    )
    .into_bytes()
}

const ALL: &str = r#"{"limit5h":{"usedRatio":0.25,"resetAt":"2026-11-13T12:00:00Z"},"limit7d":{"usedRatio":0.5,"resetAt":"2026-11-17T00:00:00+08:00"},"monthTotal":{"usedRatio":1.02},"monthCode":{"usedRatio":0.4}}"#;
const WALLET: &str = r#"{"balanceCents":500,"totalCents":1000,"monthlyChargeLimitEnabled":true,"monthlyChargeLimitCents":2000,"monthlyUsedCents":300,"currency":"USD"}"#;

fn windows(reading: &PlanReading) -> Vec<(&'static str, f64, Option<i64>)> {
    reading
        .windows
        .iter()
        .map(|(kind, sample)| (kind.as_str(), sample.used_pct, sample.resets_at))
        .collect()
}

#[test]
fn every_window_maps_with_its_ratio_and_reset_and_the_code_share_is_a_part() {
    let reading = parse_answer(200, &ok(ALL, WALLET), NOW).unwrap();
    assert_eq!(
        windows(&reading),
        [
            (
                "five_hour",
                25.0,
                juggler_core::time::timestamp_ms("2026-11-13T12:00:00Z")
            ),
            (
                "weekly",
                50.0,
                juggler_core::time::timestamp_ms("2026-11-16T16:00:00Z")
            ),
            // A little over full is believed, and shown full.
            ("monthly", 100.0, None),
        ]
    );
    assert!(
        reading
            .windows
            .iter()
            .all(|(_, sample)| sample.ts == NOW && sample.confidence == Confidence::Measured)
    );
    assert_eq!(
        reading.parts,
        [PlanPart {
            window: WindowKind::Monthly,
            part: "code".into(),
            used_pct: 40.0,
        }]
    );
    // No wallet is fine too, and so are keys the reader doesn't use.
    let lean = r#"{"code":0,"msg":"ok","request_id":"r","extra":1,"data":{"kind":"ok","note":"x","quota":{"usages":{"limit5h":{"usedRatio":0,"later":true}},"extraUsage":null,"more":[]}}}"#;
    assert_eq!(
        windows(&parse_answer(200, lean.as_bytes(), NOW).unwrap()),
        [("five_hour", 0.0, None)]
    );
}

#[test]
fn a_window_left_out_isnt_there() {
    // A new member has no week yet.
    let reading = parse_answer(
        200,
        &ok(
            r#"{"limit5h":{"usedRatio":0.1},"monthTotal":{"usedRatio":0.2}}"#,
            "null",
        ),
        NOW,
    )
    .unwrap();
    let kinds: Vec<&str> = reading.windows.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, ["five_hour", "monthly"]);
    assert!(
        parse_answer(200, &ok("{}", "null"), NOW)
            .unwrap()
            .windows
            .is_empty()
    );
}

#[test]
fn an_answer_that_fails_the_format_check_is_unknown_never_zero() {
    let bad_entries = [
        r#"{"limit5h":{"usedRatio":"0.5"}}"#,
        r#"{"limit5h":{"usedRatio":1.5}}"#,
        r#"{"limit5h":{"usedRatio":-0.1}}"#,
        r#"{"limit5h":{"usedRatio":null}}"#,
        r#"{"limit5h":{}}"#,
        r#"{"limit5h":{"usedRatio":0.5,"resetAt":"tomorrow"}}"#,
        r#"{"limit5h":{"usedRatio":0.5,"resetAt":12}}"#,
        // A renamed window: the format changed.
        r#"{"limit_5h":{"usedRatio":0.5},"limit7d":{"usedRatio":0.1}}"#,
        r#"[]"#,
    ];
    for usages in bad_entries {
        let error = parse_answer(200, &ok(usages, "null"), NOW).unwrap_err();
        assert!(
            error.contains("form AI Juggler doesn't read"),
            "{usages}: {error}"
        );
    }
    for extra in [
        r#"{"balanceCents":"5"}"#,
        r#"{"balanceCents":1,"totalCents":2,"monthlyChargeLimitEnabled":"yes","monthlyChargeLimitCents":3,"monthlyUsedCents":4,"currency":"USD"}"#,
        "7",
    ] {
        assert!(parse_answer(200, &ok(ALL, extra), NOW).is_err(), "{extra}");
    }
    for body in [
        "not json",
        r#"{"code":0}"#,
        r#"{"code":0,"data":{"kind":"maybe"}}"#,
        r#"{"code":0,"data":{"kind":"ok"}}"#,
        r#"{"code":0,"data":{"kind":"ok","quota":{"usages":7}}}"#,
    ] {
        assert!(parse_answer(200, body.as_bytes(), NOW).is_err(), "{body}");
    }
}

#[test]
fn refusals_and_kimis_own_errors_say_so_without_their_text() {
    assert!(
        parse_answer(401, br#"{"code":40101,"msg":"Unauthorized"}"#, NOW)
            .unwrap_err()
            .contains("HTTP 401")
    );
    assert!(
        parse_answer(403, br#"{"code":40301,"msg":"Forbidden"}"#, NOW)
            .unwrap_err()
            .contains("HTTP 403")
    );
    assert!(
        parse_answer(500, b"", NOW)
            .unwrap_err()
            .contains("HTTP 500")
    );
    assert_eq!(
        parse_answer(200, br#"{"code":50001,"msg":"x","data":null}"#, NOW).unwrap_err(),
        "Kimi Code's server answered with an error"
    );
    let with_status = br#"{"code":0,"data":{"kind":"error","message":"Authorization failed. secret words","status":401}}"#;
    let error = parse_answer(200, with_status, NOW).unwrap_err();
    assert_eq!(
        error,
        "Kimi Code couldn't get the usage from Moonshot (HTTP 401)"
    );
    let without = br#"{"code":0,"data":{"kind":"error","message":"secret words"}}"#;
    let error = parse_answer(200, without, NOW).unwrap_err();
    assert!(!error.contains("secret words"), "{error}");
}

fn instance(home: &Path, name: &str, body: &str) {
    let dir = instances_dir(home);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), body).unwrap();
}

fn record(
    pid: u32,
    host: &str,
    port: u16,
    started: i64,
    beat: i64,
    version: Option<&str>,
) -> String {
    let version = version.map_or_else(String::new, |v| format!(r#","host_version":"{v}""#));
    format!(
        r#"{{"server_id":"01HX","pid":{pid},"host":"{host}","port":{port},"started_at":{started},"heartbeat_at":{beat}{version}}}"#
    )
}

#[test]
fn the_live_instance_is_the_oldest_live_reachable_kimi_code_2() {
    let home = tempfile::tempdir().unwrap();
    let alive = |pid: u32| pid < 1000;
    assert_eq!(live_instance(home.path(), NOW, &alive), None, "no folder");
    std::fs::create_dir_all(instances_dir(home.path())).unwrap();
    assert_eq!(
        live_instance(home.path(), NOW, &alive),
        None,
        "empty folder"
    );
    let beat = NOW - 10_000;
    instance(
        home.path(),
        "dead.json",
        &record(5000, "127.0.0.1", 4001, 1, beat, Some("2.6.1")),
    );
    instance(
        home.path(),
        "stale.json",
        &record(10, "127.0.0.1", 4002, 2, NOW - 120_000, None),
    );
    instance(
        home.path(),
        "lan.json",
        &record(11, "192.168.1.20", 4003, 3, beat, None),
    );
    instance(
        home.path(),
        "ipv6.json",
        &record(12, "::1", 4004, 4, beat, None),
    );
    instance(
        home.path(),
        "v3.json",
        &record(13, "127.0.0.1", 4005, 5, beat, Some("3.0.0")),
    );
    instance(
        home.path(),
        "unbound.json",
        &record(14, "127.0.0.1", 0, 6, beat, None),
    );
    instance(home.path(), "bad.json", r#"{"server_id":"x","pid":"one"}"#);
    instance(
        home.path(),
        "notes.txt",
        &record(15, "127.0.0.1", 4006, 0, beat, None),
    );
    assert_eq!(live_instance(home.path(), NOW, &alive), None);
    // Every interface, no version recorded: reachable at 127.0.0.1.
    instance(
        home.path(),
        "all.json",
        &record(20, "0.0.0.0", 4010, 30, beat, None),
    );
    instance(
        home.path(),
        "older.json",
        &record(21, "127.0.0.1", 4011, 20, beat, Some("2.5.3")),
    );
    let found = live_instance(home.path(), NOW, &alive).unwrap();
    assert_eq!((found.pid, found.port), (21, 4011), "the oldest");
    std::fs::remove_file(instances_dir(home.path()).join("older.json")).unwrap();
    assert_eq!(live_instance(home.path(), NOW, &alive).unwrap().port, 4010);
}

#[test]
fn the_token_is_trimmed_and_an_empty_or_missing_one_is_none() {
    let home = tempfile::tempdir().unwrap();
    assert!(read_token(home.path()).is_none());
    std::fs::write(home.path().join(TOKEN_FILE), "\n").unwrap();
    assert!(read_token(home.path()).is_none());
    std::fs::write(home.path().join(TOKEN_FILE), "abc-123\n").unwrap();
    assert_eq!(read_token(home.path()).unwrap().as_str(), "abc-123");
}

/// A stand-in `kimi web` on 127.0.0.1: answers `count` requests, by path,
/// and returns each request as it came.
fn stand_in(count: usize, usage: &'static str) -> (u16, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..count {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = Vec::new();
            let mut buffer = [0_u8; 2048];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..read]);
            }
            let request = String::from_utf8(raw).unwrap();
            let body = if request.starts_with("GET /api/v1/healthz ") {
                r#"{"code":0,"data":{"ok":true}}"#
            } else {
                usage
            };
            let answer = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(answer.as_bytes()).unwrap();
            seen.push(request);
        }
        seen
    });
    (port, handle)
}

const CANARY: &str = "canary-3f9a1c7e-kimi-server-token";

#[test]
fn attaching_sends_the_homes_token_only_to_its_server_and_keeps_it_nowhere() {
    let home = tempfile::tempdir().unwrap();
    let usage_body: &'static str = Box::leak(
        String::from_utf8(ok(r#"{"limit5h":{"usedRatio":0.3}}"#, "null"))
            .unwrap()
            .into_boxed_str(),
    );
    let (port, server) = stand_in(2, usage_body);
    std::fs::write(home.path().join(TOKEN_FILE), format!("{CANARY}\n")).unwrap();
    let me = std::process::id();
    instance(
        home.path(),
        "a.json",
        &record(me, "127.0.0.1", port, 1, now_ms(), Some("2.6.1")),
    );
    // A LAN instance beside it is never asked.
    instance(
        home.path(),
        "lan.json",
        &record(me, "10.0.0.9", 1, 0, now_ms(), None),
    );

    let reading = usage(None, home.path(), RunLimits::default()).unwrap();
    assert_eq!(windows(&reading), [("five_hour", 30.0, None)]);
    assert!(!format!("{reading:?}").contains(CANARY));

    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].starts_with("GET /api/v1/healthz HTTP/1.1\r\n"));
    assert!(
        !seen[0].contains("Authorization"),
        "healthz goes without it"
    );
    assert!(seen[1].starts_with("GET /api/v1/oauth/usage HTTP/1.1\r\n"));
    assert!(
        seen[1].contains(&format!("Authorization: Bearer {CANARY}\r\n")),
        "trimmed"
    );
    for request in &seen {
        assert!(request.contains(&format!("Host: 127.0.0.1:{port}\r\n")));
        assert!(!request.to_ascii_lowercase().contains("origin:"));
        assert!(!request.to_ascii_lowercase().contains("cookie"));
    }
    // The home is as it was: nothing written beside the token.
    assert_eq!(
        std::fs::read_to_string(home.path().join(TOKEN_FILE)).unwrap(),
        format!("{CANARY}\n")
    );
}

#[test]
fn an_error_while_attached_never_carries_the_token() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join(TOKEN_FILE), CANARY).unwrap();
    // A live record whose port answers nothing.
    let unused = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = unused.local_addr().unwrap().port();
    drop(unused);
    instance(
        home.path(),
        "a.json",
        &record(std::process::id(), "127.0.0.1", port, 1, now_ms(), None),
    );
    let error = usage(None, home.path(), RunLimits::default()).unwrap_err();
    assert!(!error.contains(CANARY), "{error}");
    // A live server whose token is gone: said, not guessed.
    std::fs::remove_file(home.path().join(TOKEN_FILE)).unwrap();
    let error = usage(None, home.path(), RunLimits::default()).unwrap_err();
    assert!(error.contains("token can't be read"), "{error}");
}

#[test]
fn no_server_and_no_kimi_is_unknown() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        usage(None, home.path(), RunLimits::default()).unwrap_err(),
        "Kimi Code isn't installed"
    );
}

#[test]
fn the_child_path_starts_where_kimi_is() {
    assert_eq!(
        child_path(Path::new("/opt/kimi/bin/kimi")),
        format!("/opt/kimi/bin:{}", juggler_adapter::process::SYSTEM_PATH)
    );
    assert_eq!(
        child_path(Path::new("kimi")),
        juggler_adapter::process::SYSTEM_PATH
    );
}
