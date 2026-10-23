//! The bridge's guarantees: the allowlist holds only `Read` commands,
//! every write or unknown command is refused with 403, and a request needs
//! the token, a loopback Host and the allowed Origin.

use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::{Method, StatusCode};
use tauri::{Emitter as _, Manager as _};

use super::server::{READ_ONLY_MESSAGE, RateLimit, Rejection, Route, admit, sse_frame};
use super::{Config, READ_ROUTES, Token, checked_origin};
use crate::{COMMANDS, CommandAccess};

/// `crate :: commands :: app_info` and `commands::app_info` alike.
fn normalized(path: &str) -> String {
    let compact: String = path.chars().filter(|c| !c.is_whitespace()).collect();
    compact
        .strip_prefix("crate::")
        .map_or(compact.clone(), str::to_string)
}

fn last_segment(path: &str) -> String {
    normalized(path)
        .rsplit("::")
        .next()
        .unwrap_or_default()
        .to_string()
}

fn access_of(path: &str) -> Option<CommandAccess> {
    let wanted = normalized(path);
    COMMANDS
        .iter()
        .find(|(registered, _)| normalized(registered) == wanted)
        .map(|(_, access)| *access)
}

#[test]
fn every_route_calls_a_command_the_registry_classes_read() {
    assert!(!READ_ROUTES.is_empty());
    for route in READ_ROUTES {
        assert_eq!(
            access_of(route.path),
            Some(CommandAccess::Read),
            "{} ({}) is served by the bridge but is not a registered Read command",
            route.ipc,
            route.path
        );
    }
}

#[test]
fn no_write_command_is_on_the_allowlist() {
    let writes: Vec<String> = COMMANDS
        .iter()
        .filter(|(_, access)| *access == CommandAccess::Write)
        .map(|(path, _)| normalized(path))
        .collect();
    assert!(!writes.is_empty());
    for route in READ_ROUTES {
        assert!(
            !writes.contains(&normalized(route.path)),
            "write command {} is on the bridge allowlist",
            route.path
        );
        // Nor under the name of a write command.
        assert!(
            !writes.iter().any(|write| last_segment(write) == route.ipc),
            "{} is the IPC name of a write command",
            route.ipc
        );
    }
}

#[test]
fn the_registry_lists_each_command_once() {
    let mut seen = std::collections::BTreeSet::new();
    for (path, _) in COMMANDS {
        assert!(seen.insert(normalized(path)), "{path} is registered twice");
    }
    let mut ipcs = std::collections::BTreeSet::new();
    for route in READ_ROUTES {
        assert!(ipcs.insert(route.ipc), "{} is routed twice", route.ipc);
    }
}

#[test]
fn each_route_answers_to_the_commands_real_ipc_name() {
    // The three commands Tauri renames; every other IPC name is the
    // function's own.
    let renamed = [
        (
            "commands::harnesses_list",
            crate::commands::__tauri_command_name_harnesses_list!(),
        ),
        (
            "commands::profiles_list",
            crate::commands::__tauri_command_name_profiles_list!(),
        ),
        (
            "commands::sessions_list",
            crate::commands::__tauri_command_name_sessions_list!(),
        ),
    ];
    for route in READ_ROUTES {
        let path = normalized(route.path);
        let expected = renamed
            .iter()
            .find(|(renamed_path, _)| *renamed_path == path)
            .map_or_else(|| last_segment(&path), |(_, ipc)| (*ipc).to_string());
        assert_eq!(route.ipc, expected, "{path}");
    }
}

fn config(port: u16) -> Config {
    Config {
        port,
        origin: "http://localhost:5173".into(),
        token: Token::generate().unwrap(),
    }
}

fn headers(config: &Config, pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    map.insert(
        header::HOST,
        HeaderValue::from_str(&format!("127.0.0.1:{}", config.port)).unwrap(),
    );
    map.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", config.token.expose())).unwrap(),
    );
    for (name, value) in pairs {
        map.insert(name.clone(), HeaderValue::from_str(value).unwrap());
    }
    map
}

fn invoke_path(name: &str) -> String {
    format!("/v1/invoke/{name}")
}

#[test]
fn every_write_and_unknown_command_is_refused_with_403() {
    let config = config(4321);
    let mut names: Vec<String> = COMMANDS
        .iter()
        .filter(|(_, access)| *access == CommandAccess::Write)
        .map(|(path, _)| last_segment(path))
        .collect();
    names.extend(
        [
            "launch_session",
            "app_setting_set",
            "terminal_write",
            "account_import",
            "environment_scan_full",
            "providers_discover",
            "no_such_command",
            "APP_INFO",
            "app_info/../launch_session",
            "",
        ]
        .map(String::from),
    );
    for name in names {
        // Its own limiter each: the refusal is what's tested here, and
        // the commands outnumber one limiter's burst (the limit has its
        // own test).
        let rate = RateLimit::new();
        let headers = headers(&config, &[]);
        let admitted = admit(&Method::POST, &invoke_path(&name), &headers, &config, &rate);
        let Rejection {
            status,
            code,
            message,
        } = admitted.expect_err(&name);
        assert_eq!(status, StatusCode::FORBIDDEN, "{name}");
        assert_eq!(code, "read_only_bridge", "{name}");
        assert_eq!(message, READ_ONLY_MESSAGE);
    }
}

#[test]
fn a_read_command_is_admitted_with_token_host_and_origin() {
    let config = config(4321);
    let rate = RateLimit::new();
    let ok = headers(&config, &[(header::ORIGIN, "http://localhost:5173")]);
    assert!(matches!(
        admit(&Method::POST, "/v1/invoke/app_info", &ok, &config, &rate),
        Ok(Route::Invoke(route)) if route.ipc == "app_info"
    ));
    assert_eq!(
        admit(&Method::GET, "/v1/commands", &ok, &config, &rate),
        Ok(Route::Commands)
    );
    assert_eq!(
        admit(&Method::GET, "/v1/events", &ok, &config, &rate),
        Ok(Route::Events)
    );
    // Not a route: 404, only once the token checks out.
    let refused = admit(&Method::GET, "/v1/invoke/app_info", &ok, &config, &rate);
    assert_eq!(refused.unwrap_err().status, StatusCode::NOT_FOUND);
}

#[test]
fn a_request_without_the_right_token_host_or_origin_is_refused() {
    let config = config(4321);
    let rate = RateLimit::new();
    let status = |headers: &HeaderMap| {
        admit(
            &Method::POST,
            "/v1/invoke/app_info",
            headers,
            &config,
            &rate,
        )
        .unwrap_err()
        .status
    };

    let mut no_token = headers(&config, &[]);
    no_token.remove(header::AUTHORIZATION);
    assert_eq!(status(&no_token), StatusCode::UNAUTHORIZED);

    let wrong = headers(&config, &[(header::AUTHORIZATION, "Bearer 00")]);
    assert_eq!(status(&wrong), StatusCode::UNAUTHORIZED);

    let other = Token::generate().unwrap();
    let other_token = format!("Bearer {}", other.expose());
    let another_runs = headers(&config, &[(header::AUTHORIZATION, other_token.as_str())]);
    assert_eq!(status(&another_runs), StatusCode::UNAUTHORIZED);

    let basic = format!("Basic {}", config.token.expose());
    let not_bearer = headers(&config, &[(header::AUTHORIZATION, basic.as_str())]);
    assert_eq!(status(&not_bearer), StatusCode::UNAUTHORIZED);

    // DNS rebinding: the right token can't help a page on another name.
    for host in [
        "evil.example:4321",
        "127.0.0.1:1",
        "localhost",
        "127.0.0.1.nip.io:4321",
    ] {
        let rebound = headers(&config, &[(header::HOST, host)]);
        assert_eq!(status(&rebound), StatusCode::FORBIDDEN, "{host}");
    }
    let mut no_host = headers(&config, &[]);
    no_host.remove(header::HOST);
    assert_eq!(status(&no_host), StatusCode::FORBIDDEN);

    for origin in [
        "https://evil.example",
        "http://localhost:5174",
        "null",
        "http://localhost:5173/",
    ] {
        let cross = headers(&config, &[(header::ORIGIN, origin)]);
        assert_eq!(status(&cross), StatusCode::FORBIDDEN, "{origin}");
    }
    let mut preflight = headers(&config, &[(header::ORIGIN, "https://evil.example")]);
    preflight.remove(header::AUTHORIZATION);
    assert_eq!(
        admit(
            &Method::OPTIONS,
            "/v1/invoke/app_info",
            &preflight,
            &config,
            &rate
        )
        .unwrap_err()
        .status,
        StatusCode::FORBIDDEN
    );
}

#[test]
fn the_rate_limit_runs_out_and_refills() {
    let config = config(4321);
    let rate = RateLimit::new();
    let ok = headers(&config, &[]);
    let mut limited = false;
    for _ in 0..1_000 {
        if let Err(rejection) = admit(&Method::GET, "/v1/commands", &ok, &config, &rate) {
            assert_eq!(rejection.status, StatusCode::TOO_MANY_REQUESTS);
            limited = true;
            break;
        }
    }
    assert!(limited, "a burst of 1000 requests was never limited");
    // poll: once — long enough for the bucket to refill a request (50/s).
    std::thread::sleep(Duration::from_millis(100));
    assert!(admit(&Method::GET, "/v1/commands", &ok, &config, &rate).is_ok());
}

#[test]
fn only_a_loopback_http_origin_is_allowed() {
    assert!(checked_origin("http://localhost:5173").is_ok());
    assert!(checked_origin("http://127.0.0.1:4173").is_ok());
    for bad in [
        "https://localhost:5173",
        "http://localhost",
        "http://localhost:",
        "http://localhost:0",
        "http://localhost:99999",
        "http://evil.example:5173",
        "http://localhost.evil.example:5173",
        "http://localhost:5173/path",
        "*",
        "",
    ] {
        assert!(checked_origin(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_token_is_256_bits_never_debug_printed_and_compared_whole() {
    let token = Token::generate().unwrap();
    assert_eq!(token.expose().len(), 64);
    assert!(token.expose().bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(
        token.expose() != Token::generate().unwrap().expose(),
        "two runs got the same token"
    );
    assert!(!format!("{token:?}").contains(token.expose()));
    assert!(token.matches(token.expose().as_bytes()));
    assert!(!token.matches(&token.expose().as_bytes()[..63]));
    assert!(!token.matches(b""));
}

#[test]
fn an_event_payload_cannot_break_out_of_its_frame() {
    let frame = sse_frame("session://updated", "[1]\n\nevent: x\r\ndata: y");
    let text = std::str::from_utf8(&frame).unwrap();
    assert_eq!(
        text,
        "event: session://updated\ndata: [1]\ndata: \ndata: event: x\ndata: data: y\n\n"
    );
}

// ---- over the wire, against a mock app ----------------------------------

struct Running {
    _app: tauri::App<tauri::test::MockRuntime>,
    handle: tauri::AppHandle<tauri::test::MockRuntime>,
    config: Arc<Config>,
    data_dir: tempfile::TempDir,
}

fn running() -> Running {
    let data_dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_app();
    let state = crate::state::AppState::init_with_registry(
        data_dir.path().join("app-data"),
        juggler_adapter::AdapterRegistry::new(),
    )
    .unwrap();
    app.manage(Arc::new(state));
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let config = Arc::new(Config {
        port: listener.local_addr().unwrap().port(),
        origin: "http://localhost:5173".into(),
        token: Token::generate().unwrap(),
    });
    super::write_token_file(data_dir.path(), &config).unwrap();
    super::server::spawn(app.handle().clone(), listener, Arc::clone(&config)).unwrap();
    Running {
        handle: app.handle().clone(),
        _app: app,
        config,
        data_dir,
    }
}

/// One HTTP/1.1 request; the status and the body.
fn send(
    config: &Config,
    method: &str,
    path: &str,
    extra: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", config.port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nContent-Length: {}\r\n",
        config.port,
        body.len()
    );
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    // A refused body may be cut off mid-write.
    let _ = stream.write_all(body.as_bytes());
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_default();
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    (status, body)
}

fn bearer(config: &Config) -> String {
    format!("Bearer {}", config.token.expose())
}

#[test]
fn over_the_wire_reads_answer_and_writes_get_403() {
    let bridge = running();
    let config = &bridge.config;
    let auth = bearer(config);
    let with_token = [("Authorization", auth.as_str())];

    let (status, body) = send(config, "POST", "/v1/invoke/app_info", &with_token, "{}");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"edition\""), "{body}");

    let (status, body) = send(config, "POST", "/v1/invoke/accounts_list", &with_token, "");
    assert_eq!((status, body.as_str()), (200, "[]"));

    let (status, body) = send(config, "GET", "/v1/commands", &with_token, "");
    assert_eq!(status, 200);
    let names: Vec<String> = serde_json::from_str(&body).unwrap();
    assert_eq!(names.len(), READ_ROUTES.len());

    for write in [
        "launch_session",
        "app_setting_set",
        "account_remove",
        "no_such_command",
    ] {
        let (status, body) = send(
            config,
            "POST",
            &invoke_path(write),
            &with_token,
            r#"{"profileId":"x","key":"k","value":"v"}"#,
        );
        assert_eq!(status, 403, "{write}: {body}");
        assert!(body.contains("read_only_bridge"), "{body}");
    }
    // The refused write changed nothing.
    let (status, body) = send(
        config,
        "POST",
        "/v1/invoke/app_setting_get",
        &with_token,
        r#"{"key":"renewal_warn_days"}"#,
    );
    assert_eq!((status, body.as_str()), (200, "null"));
    // Settings the UI doesn't read — the one-time confirmation tokens
    // among them — stay in the app.
    for key in ["history_tokens", "restore_token", "k"] {
        let (status, _) = send(
            config,
            "POST",
            "/v1/invoke/app_setting_get",
            &with_token,
            &format!(r#"{{"key":"{key}"}}"#),
        );
        assert_eq!(status, 403, "{key}");
    }

    let (status, _) = send(config, "POST", "/v1/invoke/app_info", &[], "{}");
    assert_eq!(status, 401);

    let (status, body) = send(
        config,
        "POST",
        "/v1/invoke/app_setting_get",
        &with_token,
        r#"{"nope":1}"#,
    );
    assert_eq!(status, 400, "{body}");
    let (status, _) = send(config, "POST", "/v1/invoke/app_info", &with_token, "[1]");
    assert_eq!(status, 400);

    let big = format!(
        "{{\"key\":\"{}\"}}",
        "a".repeat(super::server::MAX_BODY_BYTES)
    );
    let (status, _) = send(
        config,
        "POST",
        "/v1/invoke/app_setting_get",
        &with_token,
        &big,
    );
    assert_eq!(status, 413);
}

#[test]
fn over_the_wire_cors_answers_only_the_dev_origin() {
    let bridge = running();
    let config = &bridge.config;
    let (status, _) = send(
        config,
        "OPTIONS",
        "/v1/invoke/app_info",
        &[
            ("Origin", "http://localhost:5173"),
            ("Access-Control-Request-Method", "POST"),
            (
                "Access-Control-Request-Headers",
                "authorization, content-type",
            ),
        ],
        "",
    );
    assert_eq!(status, 204);
    let (status, _) = send(
        config,
        "OPTIONS",
        "/v1/invoke/app_info",
        &[("Origin", "https://evil.example")],
        "",
    );
    assert_eq!(status, 403);
}

#[test]
fn the_token_file_is_owner_only_and_names_the_port() {
    let bridge = running();
    let path = bridge.data_dir.path().join(super::TOKEN_FILE);
    let text = std::fs::read_to_string(&path).unwrap();
    let file: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(file["port"], bridge.config.port);
    // Compared, never echoed.
    assert!(
        file["token"] == bridge.config.token.expose(),
        "the file holds this run's token"
    );
    assert_eq!(file["origin"], "http://localhost:5173");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    super::remove_token_file(bridge.data_dir.path());
    assert!(!path.exists());
}

#[test]
fn over_the_wire_refresh_events_stream_behind_the_token() {
    let bridge = running();
    let config = &bridge.config;
    let (status, _) = send(config, "GET", "/v1/events", &[], "");
    assert_eq!(status, 401);

    let mut stream = TcpStream::connect(("127.0.0.1", config.port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let head = format!(
        "GET /v1/events HTTP/1.1\r\nHost: localhost:{}\r\nAuthorization: {}\r\n\r\n",
        config.port,
        bearer(config)
    );
    stream.write_all(head.as_bytes()).unwrap();
    let mut seen = String::new();
    let mut buffer = [0_u8; 4096];
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        // The stream opens before the event is sent, so it can't miss it.
        if seen.contains(": open") {
            let _ = bridge
                .handle
                .emit(crate::EVENT_SESSIONS_UPDATED, vec![1, 2]);
            // A terminal's output never leaves through the bridge.
            let _ = bridge
                .handle
                .emit(crate::terminal::EVENT_TERMINAL_OUTPUT, "secret");
        }
        let read = stream.read(&mut buffer).unwrap_or(0);
        seen.push_str(&String::from_utf8_lossy(&buffer[..read]));
        if seen.contains("event: session://updated\ndata: [1,2]") {
            break;
        }
    }
    assert!(seen.contains("text/event-stream"), "{seen}");
    assert!(
        seen.contains("event: session://updated\ndata: [1,2]"),
        "{seen}"
    );
    assert!(!seen.contains("secret"), "{seen}");
}
