//! D-108 (AJ-E1107 R5): a kept crash report as the Sentry event that
//! would go, built by Sentry's own SDK. The one file in the app that uses
//! the SDK.
//!
//! - **Lazy.** Nothing here runs under Never: no report is kept then, so
//!   no event is built (`crash::crash_info_impl` asks for a preview only
//!   of a waiting report, `crash_send::drain` returns first under Never).
//!   No global client or hub is set up (`sentry::init` is never called),
//!   so the SDK never captures anything by itself.
//! - **No defaults that read the Mac.** The client has no integrations
//!   (no OS, device or environment contexts, no debug images, no panic
//!   hook), no transport of its own, no sessions, `send_default_pii`
//!   off, no server name. The event carries only what the report holds:
//!   the failure, where, its frames, the version (`release`), the edition
//!   (`environment`), the channel and the OS name.
//! - **Our scrubber in `before_send`.** Every string the event carries
//!   (its breadcrumbs too, at most [`MAX_BREADCRUMBS`]) goes through
//!   [`crate::crash::scrub`] once more, and the user, server name,
//!   request and contexts are emptied, so the scrub corpus gates what the
//!   SDK hands over. An event that doesn't survive the round trip is
//!   dropped, never sent unscrubbed.
//! - **One event, two uses.** [`preview`] is the next-launch question's
//!   read-only view; the sender (`crash_send`, release builds given the
//!   address) sends `delivery`'s envelope of the same event. Both come
//!   from [`event`], so what the user reads is what goes.
//!
//! The SDK's own HTTP transports (reqwest, curl, ureq) send from a
//! background thread and report nothing back
//! (`sentry/src/transports/thread.rs`: `send_envelope` queues and drops on
//! a full channel), which can't keep a report until Sentry took it. The
//! SDK supports a custom transport (`ClientOptions::transport`, the
//! `Transport` trait); here the envelope goes over the app's own HTTPS
//! client (`juggler-account`'s ureq + rustls), which `crash_send` calls
//! and reads the answer of, so no second HTTP stack enters the app.

use std::sync::Arc;

use sentry::protocol::{Event, Exception, Frame, Level, Map, Mechanism, Stacktrace, Uuid, Values};
use sentry::{Client, ClientOptions};

use crate::crash::{CrashReport, Identity, MAX_FRAMES};

/// The most breadcrumbs an event keeps. AI Juggler's reports carry none
/// today (it keeps no breadcrumbs); this caps any the SDK would add.
pub const MAX_BREADCRUMBS: usize = 50;

/// The event's parts that are the SDK's or AI Juggler's own values, never
/// text from the crash: left as they are by the scrubber.
const OWN_FIELDS: &[&str] = &[
    "event_id",
    "timestamp",
    "level",
    "platform",
    "sdk",
    "release",
    "environment",
    "fingerprint",
];

/// The SDK's client for crash events: no address, no transport, no
/// integrations; only the options below and our scrubber.
fn client(identity: Identity) -> Client {
    let mut options = ClientOptions::new();
    options.dsn = None;
    options.default_integrations = false;
    options.integrations = Vec::new();
    options.send_default_pii = false;
    options.server_name = None;
    options.attach_stacktrace = false;
    options.auto_session_tracking = false;
    options.max_breadcrumbs = MAX_BREADCRUMBS;
    options.environment = Some(crate::edition::current().as_str().into());
    options.before_send = Some(Arc::new(move |event| scrubbed(event, &identity)));
    Client::with_options(options)
}

/// What the event's `release` is for a version: `ai-juggler@0.5.0`.
#[must_use]
pub fn release(version: &str) -> String {
    format!("ai-juggler@{version}")
}

/// The report's id as the event's: the same report is always the same
/// event, so a retry is one issue in Sentry, and the preview matches.
fn event_id(report: &CrashReport) -> Uuid {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(format!("ai-juggler-crash:{}", report.id).as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

/// `(mechanism, type)`: a panic, a screen's error or a crash outside Rust.
fn kind(report: &CrashReport) -> (&'static str, String) {
    if report.location.starts_with("ui/") {
        ("ui", "UI error".to_owned())
    } else if report.message.starts_with("EXC_") {
        let kind = report.message.split_whitespace().next().unwrap_or_default();
        ("native", kind.to_owned())
    } else {
        ("panic", "panic".to_owned())
    }
}

/// The report's lines as frames, oldest call first (Sentry's order). A
/// Rust backtrace's `N: function` line starts a frame and its `at
/// file:line:column` line names where.
fn frames(lines: &[String]) -> Vec<Frame> {
    let mut out: Vec<Frame> = Vec::new();
    for line in lines {
        if let (Some(at), Some(last)) = (line.strip_prefix("at "), out.last_mut()) {
            let mut parts = at.rsplitn(3, ':');
            let (column, number, path) = (parts.next(), parts.next(), parts.next());
            match (path, number.and_then(|n| n.parse().ok()), column) {
                (Some(path), Some(number), Some(column)) if column.parse::<u64>().is_ok() => {
                    last.filename = Some(path.to_owned());
                    last.lineno = Some(number);
                }
                _ => last.filename = Some(at.to_owned()),
            }
            continue;
        }
        let function = line
            .split_once(": ")
            .filter(|(index, _)| !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()))
            .map_or(line.as_str(), |(_, rest)| rest);
        out.push(Frame {
            function: Some(function.to_owned()),
            in_app: Some(function.starts_with("juggler_")),
            ..Frame::default()
        });
    }
    out.reverse();
    out
}

/// The event for `report`, before the SDK prepares it.
fn raw_event(report: &CrashReport) -> Event<'static> {
    let (mechanism, ty) = kind(report);
    let frames = frames(&report.frames);
    Event {
        event_id: event_id(report),
        level: if mechanism == "ui" {
            Level::Error
        } else {
            Level::Fatal
        },
        timestamp: std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(u64::try_from(report.at_ms).unwrap_or(0)),
        platform: "native".into(),
        release: Some(release(&report.app_version).into()),
        fingerprint: vec![report.signature.clone().into()].into(),
        culprit: (!report.location.is_empty()).then(|| report.location.clone()),
        exception: Values::from(vec![Exception {
            ty,
            value: Some(report.message.clone()),
            stacktrace: (!frames.is_empty()).then(|| Stacktrace {
                frames,
                ..Stacktrace::default()
            }),
            mechanism: Some(Mechanism {
                ty: mechanism.to_owned(),
                handled: Some(mechanism == "ui"),
                ..Mechanism::default()
            }),
            ..Exception::default()
        }]),
        tags: Map::from([
            ("channel".to_owned(), report.channel.clone()),
            ("os".to_owned(), report.os.clone()),
        ]),
        ..Event::default()
    }
}

/// The event that would go for `report`, as the SDK prepares it (its SDK
/// line, the edition) and after our `before_send`; `None` when the
/// scrubber dropped it.
#[must_use]
pub fn event(report: &CrashReport, identity: &Identity) -> Option<Event<'static>> {
    client(identity.clone()).prepare_event(raw_event(report), None)
}

/// The read-only view the next-launch question shows: the whole event,
/// exactly as it would go (pretty-printed JSON of the same value).
#[must_use]
pub fn preview(report: &CrashReport) -> Option<String> {
    serde_json::to_string_pretty(&event(report, &Identity::here())?).ok()
}

/// `before_send`: the event with no user, server name, request or
/// contexts, at most [`MAX_BREADCRUMBS`] breadcrumbs and [`MAX_FRAMES`]
/// frames a stack, and every string of crash text scrubbed. `None` (not
/// sent) if it can't be read back.
fn scrubbed(mut event: Event<'static>, identity: &Identity) -> Option<Event<'static>> {
    event.user = None;
    event.server_name = None;
    event.request = None;
    event.contexts.clear();
    event.modules.clear();
    let crumbs = &mut event.breadcrumbs.values;
    crumbs.drain(..crumbs.len().saturating_sub(MAX_BREADCRUMBS));
    for exception in &mut event.exception.values {
        if let Some(stack) = exception.stacktrace.as_mut() {
            // The newest calls are last.
            stack
                .frames
                .drain(..stack.frames.len().saturating_sub(MAX_FRAMES));
        }
    }
    let mut value = serde_json::to_value(&event).ok()?;
    if let serde_json::Value::Object(fields) = &mut value {
        for (name, field) in fields.iter_mut() {
            if !OWN_FIELDS.contains(&name.as_str()) {
                scrub_value(field, identity);
            }
        }
    }
    serde_json::from_value(value).ok()
}

fn scrub_value(value: &mut serde_json::Value, identity: &Identity) {
    match value {
        serde_json::Value::String(text) => *text = identity.scrub(text),
        serde_json::Value::Array(items) => {
            for item in items {
                scrub_value(item, identity);
            }
        }
        serde_json::Value::Object(fields) => {
            for field in fields.values_mut() {
                scrub_value(field, identity);
            }
        }
        _ => {}
    }
}

/// Whether `address` is a Sentry address (a DSN) over HTTPS.
#[cfg(any(crash_sender, test))]
#[must_use]
pub fn address_ok(address: &str) -> bool {
    address
        .parse::<sentry::types::Dsn>()
        .is_ok_and(|dsn| dsn.scheme() == sentry::types::Scheme::Https)
}

/// One event, ready for the sender's HTTPS client.
#[cfg(any(crash_sender, test))]
#[derive(Debug)]
pub struct Delivery {
    /// Sentry's envelope endpoint for the address's project.
    pub url: String,
    /// The `X-Sentry-Auth` header: the address's public key, never a
    /// secret.
    pub auth: String,
    /// The envelope: one item, the event.
    pub body: Vec<u8>,
}

/// The envelope for `report` to the Sentry project at `address`: the
/// SDK's event (as [`event`]), its envelope encoding, endpoint and auth
/// header. `None` when the address isn't an HTTPS DSN or the scrubber
/// dropped the event.
#[cfg(any(crash_sender, test))]
#[must_use]
pub fn delivery(report: &CrashReport, address: &str, identity: &Identity) -> Option<Delivery> {
    let dsn: sentry::types::Dsn = address.parse().ok()?;
    if dsn.scheme() != sentry::types::Scheme::Https {
        return None;
    }
    let client = client(identity.clone());
    let event = client.prepare_event(raw_event(report), None)?;
    let mut body = Vec::new();
    sentry::Envelope::from(event).to_writer(&mut body).ok()?;
    Some(Delivery {
        url: dsn.envelope_api_url().to_string(),
        auth: dsn.to_auth(Some(&client.options().user_agent)).to_string(),
        body,
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "tests fail loudly on an unexpected value"
    )]

    use super::*;

    /// A fake address: `.invalid` never resolves (RFC 2606).
    const ADDRESS: &str = "https://public@example.invalid/1";

    fn someone() -> Identity {
        Identity {
            home: "/Users/someone-q9".to_owned(),
            user: "someone-q9".to_owned(),
        }
    }

    /// A report as if its scrub had missed everything: the SDK's
    /// `before_send` must still take it all out.
    fn unscrubbed() -> CrashReport {
        let home = "/Users/someone-q9";
        CrashReport {
            id: "0011223344556677".to_owned(),
            at_ms: 1_792_000_000_000,
            signature: "9f2c1a0b7e3d4c5f".to_owned(),
            message: format!(
                "called `Result::unwrap()` on {home}/projects/acme with sk-ant-api03-c2VjcmV0LXNlY3JldC1zZWNyZXQtc2VjcmV0 for someone-q9 ANTHROPIC_API_KEY=q8Vt3LmZx9RwP2sN"
            ),
            location: format!("{home}/Dev/AIJuggler/crates/juggler-app/src/commands.rs:42"),
            frames: vec![
                "0: juggler_app::commands::switch".to_owned(),
                format!("at {home}/Dev/AIJuggler/crates/juggler-app/src/commands.rs:42:9"),
                "1: fetch https://figurehead.ai/api?token=abc".to_owned(),
                format!("2: open {home}/x?query=1"),
            ],
            app_version: "0.5.0".to_owned(),
            channel: "beta".to_owned(),
            os: "macos".to_owned(),
        }
    }

    const LEAKS: &[&str] = &[
        "/Users/someone-q9",
        "someone-q9",
        "sk-ant-api03",
        "q8Vt3LmZx9RwP2sN",
        "https://figurehead",
        "token=abc",
        "query=1",
    ];

    /// The quality gate: a seeded crash, built into the SDK's envelope and
    /// handed to a transport that captures it (no network), carries none
    /// of the home path, user name, secrets, URLs or query strings, and
    /// none of the fields that would say who or where.
    #[test]
    fn a_seeded_crash_reaches_the_sdks_envelope_scrubbed() {
        let delivery = delivery(&unscrubbed(), ADDRESS, &someone()).unwrap();
        let body = String::from_utf8(delivery.body).unwrap();
        for leaked in LEAKS {
            assert!(!body.contains(leaked), "{leaked} survived: {body}");
        }
        assert_eq!(delivery.url, "https://example.invalid/api/1/envelope/");
        assert!(
            delivery
                .auth
                .starts_with("Sentry sentry_key=public, sentry_version=7")
        );
        assert!(!delivery.auth.contains("sentry_secret"));
        // The envelope: a header line, the item's header, the event.
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 3, "one item, the event: {body}");
        let item: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(item["type"], "event");
        let event: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        for absent in [
            "user",
            "server_name",
            "request",
            "contexts",
            "modules",
            "debug_meta",
        ] {
            assert!(event.get(absent).is_none(), "{absent} in {event}");
        }
        assert_eq!(event["release"], "ai-juggler@0.5.0");
        assert_eq!(event["environment"], crate::edition::current().as_str());
        assert_eq!(event["level"], "fatal");
        assert_eq!(
            event["fingerprint"],
            serde_json::json!(["9f2c1a0b7e3d4c5f"])
        );
        assert_eq!(
            event["tags"],
            serde_json::json!({"channel": "beta", "os": "macos"})
        );
        // No integration ran (an empty list isn't written).
        assert!(
            event["sdk"]["integrations"]
                .as_array()
                .is_none_or(Vec::is_empty),
            "{event}"
        );
        let exception = &event["exception"]["values"][0];
        assert_eq!(exception["type"], "panic");
        assert!(
            exception["value"]
                .as_str()
                .unwrap()
                .contains("~/projects/acme")
        );
        let frames = exception["stacktrace"]["frames"].as_array().unwrap();
        // Oldest first: the crashing frame is last, with where it was.
        let last = frames.last().unwrap();
        assert_eq!(last["function"], "juggler_app::commands::switch");
        assert_eq!(
            last["filename"],
            "~/Dev/AIJuggler/crates/juggler-app/src/commands.rs"
        );
        assert_eq!(last["lineno"], 42);
        assert_eq!(last["in_app"], true);
    }

    /// The preview is the event that goes, value for value.
    #[test]
    fn the_preview_is_exactly_the_event_that_goes() {
        let report = crate::crash::report_of("boom at ~/x", "src/a.rs:1", "0: juggler_app::a", 7);
        let shown: serde_json::Value = serde_json::from_str(&preview(&report).unwrap()).unwrap();
        let body =
            String::from_utf8(delivery(&report, ADDRESS, &Identity::here()).unwrap().body).unwrap();
        let sent: serde_json::Value = serde_json::from_str(body.lines().nth(2).unwrap()).unwrap();
        assert_eq!(shown, sent);
        // And again: the same report is the same event (a retry is one issue).
        let again: serde_json::Value = serde_json::from_str(&preview(&report).unwrap()).unwrap();
        assert_eq!(shown, again);
    }

    #[test]
    fn breadcrumbs_and_frames_are_capped_and_scrubbed() {
        let mut event = raw_event(&unscrubbed());
        event.breadcrumbs = Values::from(
            (0..MAX_BREADCRUMBS + 10)
                .map(|n| sentry::Breadcrumb {
                    message: Some(format!("step {n} in /Users/someone-q9/a")),
                    ..sentry::Breadcrumb::default()
                })
                .collect::<Vec<_>>(),
        );
        event.exception.values[0]
            .stacktrace
            .as_mut()
            .unwrap()
            .frames = (0..MAX_FRAMES + 5)
            .map(|n| Frame {
                function: Some(format!("f{n}")),
                ..Frame::default()
            })
            .collect();
        event.user = Some(sentry::User {
            username: Some("someone-q9".to_owned()),
            ..sentry::User::default()
        });
        event.server_name = Some("someones-mac".into());
        let event = scrubbed(event, &someone()).unwrap();
        assert_eq!(event.breadcrumbs.values.len(), MAX_BREADCRUMBS);
        assert_eq!(
            event.breadcrumbs.values.last().unwrap().message.as_deref(),
            Some(format!("step {} in ~/a", MAX_BREADCRUMBS + 9).as_str())
        );
        let frames = &event.exception.values[0]
            .stacktrace
            .as_ref()
            .unwrap()
            .frames;
        assert_eq!(frames.len(), MAX_FRAMES);
        assert_eq!(
            frames.last().unwrap().function.as_deref(),
            Some(format!("f{}", MAX_FRAMES + 4).as_str())
        );
        assert!(event.user.is_none() && event.server_name.is_none());
    }

    #[test]
    fn a_screen_error_and_a_native_crash_say_what_they_are() {
        let mut ui = crate::crash::report_of("ui: TypeError", "ui/usage", "", 1);
        let event = event(&ui, &someone()).unwrap();
        assert_eq!(event.level, Level::Error);
        assert_eq!(event.exception.values[0].ty, "UI error");
        assert_eq!(
            event.exception.values[0]
                .mechanism
                .as_ref()
                .unwrap()
                .handled,
            Some(true)
        );
        ui.location = "libsystem_kernel.dylib: __pthread_kill".to_owned();
        ui.message = "EXC_BAD_ACCESS (SIGSEGV) KERN_INVALID_ADDRESS".to_owned();
        let native = super::event(&ui, &someone()).unwrap();
        assert_eq!(native.level, Level::Fatal);
        assert_eq!(native.exception.values[0].ty, "EXC_BAD_ACCESS");
    }

    #[test]
    fn only_an_https_sentry_address_is_one() {
        assert!(address_ok(ADDRESS));
        for bad in [
            "http://public@example.invalid/1",
            "https://example.invalid/v1/reports",
            "",
        ] {
            assert!(!address_ok(bad), "{bad}");
            assert!(delivery(&unscrubbed(), bad, &someone()).is_none(), "{bad}");
        }
    }
}
