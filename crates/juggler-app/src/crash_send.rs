//! AJ-E1516 Req.1, D-108: sends the crash reports the user chose to send
//! to Sentry.
//!
//! Built only where the build was given Sentry's address, the DSN
//! (`build.rs` sets `crash_sender` for a release build whose
//! `AI_JUGGLER_CRASH_ENDPOINT` is an `https://` DSN; no workflow sets it
//! until the maintainer adds the secret, so no build has this code yet),
//! and in this module's own tests. The crash code itself (`crash.rs`)
//! holds no way to send.
//!
//! It sends only what waits in `queued/`: a report the user answered
//! Send to (Ask), or one kept under Always. Each goes as the Sentry event
//! the SDK builds from it (`crash_sentry::delivery`: scrubbed again in
//! `before_send`, the event the next-launch preview showed), over the
//! app's own HTTPS client, at most [`MAX_SENT_PER_DAY`] a day. A report is
//! removed once Sentry took it; one it didn't stays for the next try.
//! Under Never there is nothing to send (Never removes every kept report)
//! and nothing is read or built.

use std::path::Path;

use juggler_account::{HttpRequest, Transport};

/// At most this many reports sent a day.
pub const MAX_SENT_PER_DAY: usize = 5;

/// Where the days of reports sent are noted (one a line), for the limit.
const SENT_LOG: &str = "sent.log";

/// Sends what's queued in `dir` to the Sentry project at `address` (a
/// DSN), scrubbed for `identity`; returns how many went. `effective` is
/// the setting that applies (`never` sends nothing). Each report is read
/// just before it's sent: one removed meanwhile (the user chose Never)
/// doesn't go.
pub fn drain(
    dir: &Path,
    effective: &str,
    transport: &dyn Transport,
    address: &str,
    identity: &crate::crash::Identity,
    now_ms: i64,
) -> usize {
    if effective == "never" || !crate::crash_sentry::address_ok(address) {
        return 0;
    }
    let today = now_ms.div_euclid(86_400_000);
    let log_path = dir.join(SENT_LOG);
    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
    let mut days: Vec<i64> = log
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter(|day| today - day < 7)
        .collect();
    let pruned = days.len() != log.lines().count();
    // Oldest first, by when the report was made.
    let mut queued: Vec<(i64, std::path::PathBuf)> = std::fs::read_dir(dir.join("queued"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let report: crate::crash::CrashReport =
                serde_json::from_slice(&std::fs::read(entry.path()).ok()?).ok()?;
            Some((report.at_ms, entry.path()))
        })
        .collect();
    queued.sort();
    let mut sent = 0;
    for (_, path) in queued {
        if days.iter().filter(|day| **day == today).count() >= MAX_SENT_PER_DAY {
            break;
        }
        let Some(report) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        else {
            continue;
        };
        // The scrubber dropped it: it never goes, so it waits no longer.
        let Some(delivery) = crate::crash_sentry::delivery(&report, address, identity) else {
            let _ = std::fs::remove_file(&path);
            continue;
        };
        let request = HttpRequest {
            method: "POST",
            url: delivery.url,
            bearer: None,
            content_type: "application/x-sentry-envelope",
            headers: vec![("X-Sentry-Auth", delivery.auth)],
            body: delivery.body,
            long_transfer: false,
        };
        match transport.send(&request) {
            Ok(response) if (200..300).contains(&response.status) => {
                let _ = std::fs::remove_file(&path);
                days.push(today);
                sent += 1;
            }
            // Refused for good (malformed, too large): it won't be taken
            // later either, and must not hold up the others.
            Ok(response)
                if (400..500).contains(&response.status)
                    && !matches!(response.status, 408 | 429) =>
            {
                let _ = std::fs::remove_file(&path);
            }
            // Busy, down or not reached: kept for the next try.
            _ => break,
        }
    }
    if sent > 0 || pruned {
        let text = days.iter().fold(String::new(), |mut text, day| {
            use std::fmt::Write as _;
            let _ = writeln!(text, "{day}");
            text
        });
        let _ = std::fs::write(&log_path, text);
    }
    sent
}

/// The sending loop: at once, then every 30 minutes, what the user chose
/// to send goes to the Sentry project at `address`.
pub fn run(state: &crate::state::AppState, transport: &dyn Transport, address: &str) -> ! {
    let identity = crate::crash::Identity::here();
    loop {
        let _ = drain(
            &crate::crash::reports_dir(state),
            &crate::crash::effective(state),
            transport,
            address,
            &identity,
            juggler_core::model::Timestamp::now().millis(),
        );
        std::thread::sleep(std::time::Duration::from_mins(30));
    }
}

/// Starts the loop with the address this build was given.
#[cfg(crash_sender)]
pub fn start(state: &std::sync::Arc<crate::state::AppState>) {
    let state = std::sync::Arc::clone(state);
    std::thread::spawn(move || {
        let transport = juggler_account::UreqTransport::new(env!("CARGO_PKG_VERSION"));
        run(&state, &transport, env!("AI_JUGGLER_CRASH_ENDPOINT"))
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{MAX_SENT_PER_DAY, drain};
    use crate::crash::{CrashReport, Identity};
    use juggler_account::RecordingTransport;

    /// A fake Sentry address: `.invalid` never resolves (RFC 2606).
    const ADDRESS: &str = "https://public@example.invalid/1";
    const ENVELOPES: &str = "https://example.invalid/api/1/envelope/";
    const DAY: i64 = 86_400_000;

    fn someone() -> Identity {
        Identity {
            home: "/Users/someone-q9".to_owned(),
            user: "someone-q9".to_owned(),
        }
    }

    fn put(dir: &std::path::Path, report: &CrashReport) {
        std::fs::create_dir_all(dir.join("queued")).unwrap();
        std::fs::write(
            dir.join("queued").join(format!("{}.json", report.id)),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }

    fn queue(dir: &std::path::Path, n: usize) -> Vec<CrashReport> {
        (0..n)
            .map(|i| {
                let report = crate::crash::report_of("boom", "src/x.rs:1", "", 1_000 + i as i64);
                put(dir, &report);
                report
            })
            .collect()
    }

    fn left(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir.join("queued"))
            .map(Iterator::count)
            .unwrap_or(0)
    }

    fn drained(dir: &std::path::Path, effective: &str, t: &RecordingTransport, now: i64) -> usize {
        drain(dir, effective, t, ADDRESS, &someone(), now)
    }

    /// Each queued report goes as the SDK's envelope of its event (the
    /// event the preview showed), to Sentry's envelope endpoint with the
    /// public key only, and is then removed.
    #[test]
    fn each_queued_report_goes_as_its_sentry_event_and_is_then_removed() {
        let dir = tempfile::tempdir().unwrap();
        let reports = queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(200, "");
        transport.respond(200, "");
        assert_eq!(drained(dir.path(), "ask", &transport, 10 * DAY), 2);
        let requests = transport.requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!((request.method, request.url.as_str()), ("POST", ENVELOPES));
            assert!(request.bearer.is_none());
            assert_eq!(request.content_type, "application/x-sentry-envelope");
            assert_eq!(request.headers.len(), 1);
            assert_eq!(request.headers[0].0, "X-Sentry-Auth");
            assert!(request.headers[0].1.contains("sentry_key=public"));
        }
        // Oldest first, each exactly the event built for it.
        let expected: Vec<Vec<u8>> = reports
            .iter()
            .map(|r| {
                crate::crash_sentry::delivery(r, ADDRESS, &someone())
                    .unwrap()
                    .body
            })
            .collect();
        assert_eq!(
            requests.iter().map(|r| r.body.clone()).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(left(dir.path()), 0);
    }

    /// The quality gate at the sender: a report kept unscrubbed (as if
    /// the scrub had missed it all) reaches the captured request scrubbed
    /// by the SDK's `before_send`.
    #[test]
    fn a_seeded_crash_leaves_scrubbed_through_the_sdk() {
        let dir = tempfile::tempdir().unwrap();
        let home = "/Users/someone-q9";
        let mut report = crate::crash::report_of("x", "src/x.rs:1", "", 5);
        report.message = format!(
            "panicked in {home}/projects/acme for someone-q9 with sk-ant-api03-c2VjcmV0LXNlY3JldC1zZWNyZXQtc2VjcmV0"
        );
        report.frames = vec![
            "0: juggler_app::x".to_owned(),
            format!("at {home}/Dev/x.rs:1:1"),
            "1: get https://figurehead.ai/api?token=abc".to_owned(),
        ];
        put(dir.path(), &report);
        let transport = RecordingTransport::default();
        transport.respond(200, "");
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 1);
        let body = String::from_utf8(transport.requests()[0].body.clone()).unwrap();
        for leaked in [
            home,
            "someone-q9",
            "sk-ant-api03",
            "https://figurehead",
            "token=abc",
        ] {
            assert!(!body.contains(leaked), "{leaked} survived: {body}");
        }
        assert!(
            body.contains("~/projects/acme") && body.contains("[user]"),
            "{body}"
        );
    }

    #[test]
    fn never_sends_nothing_and_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 1);
        let transport = RecordingTransport::default();
        assert_eq!(drained(dir.path(), "never", &transport, DAY), 0);
        assert_eq!(transport.count(), 0);
        assert_eq!(left(dir.path()), 1);
    }

    #[test]
    fn a_report_not_taken_stays_for_the_next_try() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(503, "");
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 0);
        assert_eq!(transport.count(), 1, "stops at the first failure");
        assert_eq!(left(dir.path()), 2);
        // Not reached at all: kept too.
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 0);
        assert_eq!(left(dir.path()), 2);
    }

    #[test]
    fn at_most_five_go_a_day() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), MAX_SENT_PER_DAY + 2);
        let transport = RecordingTransport::default();
        for _ in 0..MAX_SENT_PER_DAY + 2 {
            transport.respond(200, "");
        }
        assert_eq!(
            drained(dir.path(), "always", &transport, DAY),
            MAX_SENT_PER_DAY
        );
        assert_eq!(
            drained(dir.path(), "always", &transport, DAY + 1_000),
            0,
            "same day"
        );
        assert_eq!(
            drained(dir.path(), "always", &transport, 2 * DAY),
            2,
            "the next day"
        );
        assert_eq!(left(dir.path()), 0);
    }

    /// Only an HTTPS Sentry address: plain HTTP, or an address that isn't
    /// a DSN, sends nothing and keeps the reports.
    #[test]
    fn only_https() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 1);
        let transport = RecordingTransport::default();
        for address in [
            "http://public@example.invalid/1",
            "https://example.invalid/v1/reports",
        ] {
            assert_eq!(
                drain(dir.path(), "always", &transport, address, &someone(), DAY),
                0
            );
        }
        assert_eq!(transport.count(), 0);
        assert_eq!(left(dir.path()), 1);
    }

    /// Consent: a report kept under Ask that the user hasn't answered
    /// (`asking/`) is never sent.
    #[test]
    fn a_report_waiting_for_the_users_answer_is_never_sent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("asking")).unwrap();
        let waiting = crate::crash::report_of("boom", "src/y.rs:2", "", 5);
        std::fs::write(
            dir.path()
                .join("asking")
                .join(format!("{}.json", waiting.id)),
            serde_json::to_vec(&waiting).unwrap(),
        )
        .unwrap();
        let transport = RecordingTransport::default();
        assert_eq!(drained(dir.path(), "ask", &transport, DAY), 0);
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 0);
        assert_eq!(transport.count(), 0);
    }

    #[test]
    fn a_report_refused_for_good_is_dropped_and_the_rest_go() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(413, "");
        transport.respond(200, "");
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 1);
        assert_eq!(left(dir.path()), 0);
        // Too many requests (Sentry's rate limit): kept for later.
        queue(dir.path(), 1);
        transport.respond(429, "");
        assert_eq!(drained(dir.path(), "always", &transport, DAY), 0);
        assert_eq!(left(dir.path()), 1);
    }

    #[test]
    fn the_sent_log_keeps_a_week() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sent.log"), "1\n2\n9\n").unwrap();
        let transport = RecordingTransport::default();
        assert_eq!(drained(dir.path(), "always", &transport, 10 * DAY), 0);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sent.log")).unwrap(),
            "9\n"
        );
    }
}
