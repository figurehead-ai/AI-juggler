//! AJ-E1516 Req.1: sends the crash reports the user chose to send.
//!
//! Built only where the build was given the reporting service's address
//! (`build.rs` sets `crash_sender` for a release build with
//! `AI_JUGGLER_CRASH_ENDPOINT`; no workflow sets it today, so no build
//! has this code), and in this module's own tests. The crash code itself
//! (`crash.rs`) holds no way to send.
//!
//! It sends only what waits in `queued/`: a report the user answered
//! Send to (Ask), or one kept under Always. Each goes exactly as stored —
//! the JSON the user could read — over HTTPS, at most
//! [`MAX_SENT_PER_DAY`] a day. A report is removed once the service took
//! it; one it didn't stays for the next try. Under Never there is nothing
//! to send (Never removes every kept report) and nothing is read.

use std::path::Path;

use juggler_account::{HttpRequest, Transport};

/// At most this many reports sent a day.
pub const MAX_SENT_PER_DAY: usize = 5;

/// Where the days of reports sent are noted (one a line), for the limit.
const SENT_LOG: &str = "sent.log";

/// Sends what's queued in `dir` to `endpoint`; returns how many went.
/// `effective` is the setting that applies (`never` sends nothing). Each
/// report is read just before it's sent: one removed meanwhile (the user
/// chose Never) doesn't go.
pub fn drain(
    dir: &Path,
    effective: &str,
    transport: &dyn Transport,
    endpoint: &str,
    now_ms: i64,
) -> usize {
    if effective == "never" || !endpoint.starts_with("https://") {
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
        let Ok(body) = std::fs::read(&path) else {
            continue;
        };
        let request = HttpRequest {
            method: "POST",
            url: endpoint.to_owned(),
            bearer: None,
            content_type: "application/json",
            headers: Vec::new(),
            body,
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
/// to send goes to `address`.
pub fn run(state: &crate::state::AppState, transport: &dyn Transport, address: &str) -> ! {
    loop {
        let _ = drain(
            &crate::crash::reports_dir(state),
            &crate::crash::effective(state),
            transport,
            address,
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
    use juggler_account::RecordingTransport;

    const URL: &str = "https://crash.example/v1/reports";
    const DAY: i64 = 86_400_000;

    fn queue(dir: &std::path::Path, n: usize) -> Vec<Vec<u8>> {
        std::fs::create_dir_all(dir.join("queued")).unwrap();
        (0..n)
            .map(|i| {
                let report = crate::crash::report_of("boom", "src/x.rs:1", "", 1_000 + i as i64);
                let bytes = serde_json::to_vec_pretty(&report).unwrap();
                std::fs::write(
                    dir.join("queued").join(format!("{}.json", report.id)),
                    &bytes,
                )
                .unwrap();
                bytes
            })
            .collect()
    }

    fn left(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir.join("queued"))
            .map(Iterator::count)
            .unwrap_or(0)
    }

    #[test]
    fn each_queued_report_goes_exactly_as_stored_and_is_then_removed() {
        let dir = tempfile::tempdir().unwrap();
        let stored = queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(202, "");
        transport.respond(202, "");
        assert_eq!(drain(dir.path(), "ask", &transport, URL, 10 * DAY), 2);
        let requests = transport.requests();
        assert_eq!(requests.len(), 2);
        assert!(
            requests
                .iter()
                .all(|r| r.method == "POST" && r.url == URL && r.bearer.is_none())
        );
        assert_eq!(
            requests.iter().map(|r| r.body.clone()).collect::<Vec<_>>(),
            stored
        );
        assert_eq!(left(dir.path()), 0);
    }

    #[test]
    fn never_sends_nothing_and_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 1);
        let transport = RecordingTransport::default();
        assert_eq!(drain(dir.path(), "never", &transport, URL, DAY), 0);
        assert_eq!(transport.count(), 0);
        assert_eq!(left(dir.path()), 1);
    }

    #[test]
    fn a_report_not_taken_stays_for_the_next_try() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(503, "");
        assert_eq!(drain(dir.path(), "always", &transport, URL, DAY), 0);
        assert_eq!(transport.count(), 1, "stops at the first failure");
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
            drain(dir.path(), "always", &transport, URL, DAY),
            MAX_SENT_PER_DAY
        );
        assert_eq!(
            drain(dir.path(), "always", &transport, URL, DAY + 1_000),
            0,
            "same day"
        );
        assert_eq!(
            drain(dir.path(), "always", &transport, URL, 2 * DAY),
            2,
            "the next day"
        );
        assert_eq!(left(dir.path()), 0);
    }

    #[test]
    fn only_https() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 1);
        let transport = RecordingTransport::default();
        assert_eq!(
            drain(
                dir.path(),
                "always",
                &transport,
                "http://crash.example/v1",
                DAY
            ),
            0
        );
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
        assert_eq!(drain(dir.path(), "ask", &transport, URL, DAY), 0);
        assert_eq!(drain(dir.path(), "always", &transport, URL, DAY), 0);
        assert_eq!(transport.count(), 0);
    }

    #[test]
    fn a_report_refused_for_good_is_dropped_and_the_rest_go() {
        let dir = tempfile::tempdir().unwrap();
        queue(dir.path(), 2);
        let transport = RecordingTransport::default();
        transport.respond(413, "");
        transport.respond(202, "");
        assert_eq!(drain(dir.path(), "always", &transport, URL, DAY), 1);
        assert_eq!(left(dir.path()), 0);
        // Too many requests: kept for later.
        queue(dir.path(), 1);
        transport.respond(429, "");
        assert_eq!(drain(dir.path(), "always", &transport, URL, DAY), 0);
        assert_eq!(left(dir.path()), 1);
    }

    #[test]
    fn the_sent_log_keeps_a_week() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sent.log"), "1\n2\n9\n").unwrap();
        let transport = RecordingTransport::default();
        assert_eq!(drain(dir.path(), "always", &transport, URL, 10 * DAY), 0);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sent.log")).unwrap(),
            "9\n"
        );
    }
}
