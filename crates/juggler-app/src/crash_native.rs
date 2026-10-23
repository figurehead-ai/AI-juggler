//! AJ-E1516 Req.2: crashes outside Rust (in a system library, the web
//! view, the Swift runtime) leave no panic, so no report of AI Juggler's
//! own. macOS writes one for any process that crashes:
//! `~/Library/Logs/DiagnosticReports/<name>-<time>.ips`, a header line
//! (`bundleID`, `bug_type` `309` for a crash) and a body (`exception`,
//! `faultingThread`, `threads[].frames`, `usedImages`).
//!
//! At the next launch, with crash reports on (Ask or Always), the reports
//! of this build's own app that arrived since the last look become crash
//! reports like a panic's: the same scrub, the same limits, the same
//! Ask/Always. Only the kind of crash and the crashing thread's frames
//! (library name and function) are kept; the body's identifiers, paths,
//! registers and memory figures never leave it. Under Never nothing is
//! read. The first look only marks where to start: crashes from before
//! the user turned reports on aren't picked up. Elsewhere than macOS
//! there's nothing to read.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::crash::{CrashReport, MAX_FRAMES, keep, report_of};

/// Where the newest report already looked at is noted (its time, epoch
/// ms), in the crash reports' folder.
const SEEN: &str = "native.seen";
/// macOS's crash type in the header.
const CRASH_BUG_TYPE: &str = "309";
/// A report bigger than this isn't read (they're tens of KB).
const MAX_REPORT_BYTES: u64 = 4 * 1024 * 1024;
/// The most of a header line read to tell whose report it is.
const MAX_HEADER_BYTES: u64 = 64 * 1024;
/// macOS can still be writing a report this young: one that doesn't
/// parse yet is looked at again next time.
const STILL_WRITING_MS: i64 = 60_000;

/// Where macOS keeps this user's crash reports.
#[must_use]
pub fn system_reports() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library/Logs/DiagnosticReports"))
    } else {
        None
    }
}

/// From now on: crash reports from `at_ms` on are picked up (the user
/// turned reports on). Writes the mark in the crash reports' folder.
pub fn start_from(crashes: &Path, at_ms: i64) {
    if std::fs::create_dir_all(crashes).is_ok() {
        let _ = std::fs::write(crashes.join(SEEN), format!("{at_ms}\n"));
    }
}

/// Turns this app's new crash reports in `reports` into kept reports in
/// `crashes` (`setting` is the effective `never` | `ask` | `always`).
/// Another app's report is told apart by its first line alone. Returns
/// how many were kept.
#[must_use]
pub fn collect(
    reports: &Path,
    crashes: &Path,
    bundle_id: &str,
    setting: &str,
    now_ms: i64,
) -> usize {
    if setting == "never" {
        return 0;
    }
    let seen: Option<i64> = std::fs::read_to_string(crashes.join(SEEN))
        .ok()
        .and_then(|text| text.trim().parse().ok());
    let mut entries: Vec<(i64, PathBuf)> = std::fs::read_dir(reports)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ips"))
        .filter_map(|entry| {
            // Not followed: a link isn't a report macOS wrote.
            let metadata = entry.metadata().ok()?;
            let at = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
            let at = i64::try_from(at.as_millis()).ok()?;
            (metadata.is_file() && metadata.len() <= MAX_REPORT_BYTES).then(|| (at, entry.path()))
        })
        .collect();
    entries.sort();
    let Some(seen) = seen else {
        // The first look only marks where to start: the newest report there
        // now (or now, when there's none).
        let mark = entries.last().map_or(now_ms, |(at, _)| *at);
        start_from(crashes, mark);
        return 0;
    };
    let mut mark = seen;
    let mut kept = 0;
    for (at, path) in entries.into_iter().filter(|(at, _)| *at > seen) {
        match read_ours(&path, bundle_id) {
            Read::Ours(text) => match native_report(&text, bundle_id, at) {
                Some(report) => {
                    if keep(crashes, &report, setting).unwrap_or(false) {
                        kept += 1;
                    }
                }
                // Still being written: next time.
                None if now_ms - at < STILL_WRITING_MS => break,
                None => {}
            },
            Read::Unreadable if now_ms - at < STILL_WRITING_MS => break,
            Read::NotOurs | Read::Unreadable => {}
        }
        mark = at;
    }
    if mark != seen {
        start_from(crashes, mark);
    }
    kept
}

enum Read {
    /// This app's report, whole.
    Ours(String),
    /// Another app's (only its first line was read).
    NotOurs,
    /// Couldn't be read or its first line isn't a header.
    Unreadable,
}

/// Reads a report's first line; the rest only when it's `bundle_id`'s.
fn read_ours(path: &Path, bundle_id: &str) -> Read {
    use std::io::{BufRead as _, Read as _};
    let Ok(file) = std::fs::File::open(path) else {
        return Read::Unreadable;
    };
    let mut reader = std::io::BufReader::new(file);
    let mut header = String::new();
    if (&mut reader)
        .take(MAX_HEADER_BYTES)
        .read_line(&mut header)
        .is_err()
    {
        return Read::Unreadable;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&header) else {
        return Read::Unreadable;
    };
    if value.get("bundleID").and_then(serde_json::Value::as_str) != Some(bundle_id) {
        return Read::NotOurs;
    }
    let mut rest = String::new();
    if reader
        .take(MAX_REPORT_BYTES)
        .read_to_string(&mut rest)
        .is_err()
    {
        return Read::Unreadable;
    }
    Read::Ours(header + &rest)
}

/// A crash report of `bundle_id`'s, as a scrubbed report; `None` for
/// another app's, a report that isn't a crash, or one that doesn't parse.
#[must_use]
pub fn native_report(text: &str, bundle_id: &str, at_ms: i64) -> Option<CrashReport> {
    let (header, body) = text.split_once('\n')?;
    let header: serde_json::Value = serde_json::from_str(header).ok()?;
    let field = |value: &serde_json::Value, key: &str| -> String {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    if field(&header, "bundleID") != bundle_id || field(&header, "bug_type") != CRASH_BUG_TYPE {
        return None;
    }
    let body: serde_json::Value = serde_json::from_str(body).ok()?;
    let exception = body.get("exception").unwrap_or(&serde_json::Value::Null);
    let kind = [field(exception, "type"), field(exception, "signal")]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let kind = match kind.as_slice() {
        [kind, signal] => format!("{kind} ({signal})"),
        [kind] => kind.clone(),
        _ => "crash".to_owned(),
    };
    let message = match field(exception, "subtype") {
        subtype if subtype.is_empty() => kind,
        subtype => format!("{kind} {subtype}"),
    };
    let images: Vec<String> = body
        .get("usedImages")
        .and_then(serde_json::Value::as_array)
        .map(|images| images.iter().map(|image| field(image, "name")).collect())
        .unwrap_or_default();
    let faulting = body
        .get("faultingThread")
        .and_then(serde_json::Value::as_u64)
        .and_then(|index| usize::try_from(index).ok());
    let thread = body
        .get("threads")
        .and_then(serde_json::Value::as_array)
        .and_then(|threads| {
            faulting.and_then(|index| threads.get(index)).or_else(|| {
                threads.iter().find(|thread| {
                    thread.get("triggered").and_then(serde_json::Value::as_bool) == Some(true)
                })
            })
        });
    let frames: Vec<String> = thread
        .and_then(|thread| thread.get("frames"))
        .and_then(serde_json::Value::as_array)
        .map(|frames| {
            frames
                .iter()
                .take(MAX_FRAMES)
                .map(|frame| {
                    let image = frame
                        .get("imageIndex")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|index| images.get(usize::try_from(index).ok()?))
                        .filter(|name| !name.is_empty())
                        .map_or("?", String::as_str);
                    match field(frame, "symbol") {
                        symbol if symbol.is_empty() => image.to_owned(),
                        symbol => format!("{image}: {symbol}"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let location = frames.first().cloned().unwrap_or_default();
    let mut report = report_of(&message, &location, &frames.join("\n"), at_ms);
    // The version that crashed (an update since doesn't change it).
    let version = field(&header, "app_version");
    if !version.is_empty() {
        report.app_version = version;
    }
    Some(report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{collect, native_report};

    const OURS: &str = "ai.figurehead.juggler";

    /// A crash report in macOS's shape (the keys of a real one; synthetic
    /// values), with identifiers and paths that must never be kept.
    fn ips(bundle: &str, bug_type: &str) -> String {
        let header = serde_json::json!({
            "app_name": "AI Juggler", "timestamp": "2026-10-23 09:00:00.00 +0200",
            "app_version": "0.0.6", "bundleID": bundle, "bug_type": bug_type,
            "os_version": "macOS 15.5 (24F74)", "name": "AI Juggler",
            "incident_id": "77D477FF-583A-4646-90CF-F5D14ABDEE6C"
        });
        let body = serde_json::json!({
            "pid": 4242, "userID": 501, "crashReporterKey": "C0FFEE-KEY",
            "procPath": "/Users/olivia/Applications/AI Juggler.app/Contents/MacOS/ai-juggler",
            "exception": { "type": "EXC_BAD_ACCESS", "signal": "SIGSEGV",
                           "subtype": "KERN_INVALID_ADDRESS at 0x0000000000000010",
                           "codes": "0x0000000000000001, 0x0000000000000010" },
            "faultingThread": 1,
            "threads": [
                { "id": 1, "frames": [ { "imageIndex": 0, "symbol": "main", "imageOffset": 1 } ] },
                { "id": 2, "triggered": true, "threadState": { "x": [ { "value": 12345 } ] },
                  "frames": [
                    { "imageIndex": 1, "symbol": "WebCore::Node::ref()", "symbolLocation": 12, "imageOffset": 4120 },
                    { "imageIndex": 2, "symbol": "objc_msgSend", "imageOffset": 77 },
                    { "imageIndex": 0, "imageOffset": 5 }
                  ] }
            ],
            "usedImages": [
                { "name": "ai-juggler", "path": "/Users/olivia/Applications/AI Juggler.app/Contents/MacOS/ai-juggler", "uuid": "1b071767-a2a3-3416-943c-37500bb073d8" },
                { "name": "WebCore", "path": "/System/Library/Frameworks/WebKit.framework/WebCore" },
                { "name": "libobjc.A.dylib", "path": "/usr/lib/libobjc.A.dylib" }
            ],
            "vmSummary": "ReadOnly portion of Libraries: Total=1.0G"
        });
        format!("{header}\n{body:#}")
    }

    #[test]
    fn an_ips_crash_keeps_its_kind_and_frames_and_nothing_else() {
        let report = native_report(&ips(OURS, "309"), OURS, 1_000).unwrap();
        assert_eq!(
            report.message,
            "EXC_BAD_ACCESS (SIGSEGV) KERN_INVALID_ADDRESS at 0x0000000000000010"
        );
        assert_eq!(report.location, "WebCore: WebCore::Node::ref()");
        assert_eq!(
            report.frames,
            vec![
                "WebCore: WebCore::Node::ref()",
                "libobjc.A.dylib: objc_msgSend",
                "ai-juggler"
            ]
        );
        let all = serde_json::to_string(&report).unwrap();
        for never in [
            "olivia",
            "C0FFEE",
            "4242",
            "77D477FF",
            "1b071767",
            "vmSummary",
            "12345",
            "/Users/",
        ] {
            assert!(!all.contains(never), "{never} in {all}");
        }
        // The same crash twice is one signature (the daily limit's key).
        let again = native_report(&ips(OURS, "309"), OURS, 2_000).unwrap();
        assert_eq!(report.signature, again.signature);
    }

    #[test]
    fn another_apps_report_or_a_non_crash_is_left_alone() {
        assert!(native_report(&ips("com.example.other", "309"), OURS, 1).is_none());
        assert!(
            native_report(&ips(OURS, "288"), OURS, 1).is_none(),
            "a hang report"
        );
        assert!(native_report("not json\n{}", OURS, 1).is_none());
    }

    /// Writes `body` to `reports/<name>` with the given modification time.
    fn report_at(reports: &std::path::Path, name: &str, body: &str, at_ms: i64) {
        let path = reports.join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(at_ms as u64))
            .unwrap();
    }

    const T0: i64 = 1_800_000_000_000;

    #[test]
    fn only_crashes_after_the_first_look_are_kept_and_never_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("DiagnosticReports");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        report_at(&reports, "AI Juggler-old.ips", &ips(OURS, "309"), T0);

        // Never: nothing read, nothing noted.
        assert_eq!(collect(&reports, &crashes, OURS, "never", T0 + 1_000), 0);
        assert!(!crashes.exists());

        // The first look marks where to start: the old crash isn't kept.
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 1_000), 0);
        assert!(crashes.join("native.seen").exists());

        // A crash after it is kept, waiting for the user's answer; another
        // app's is not.
        report_at(
            &reports,
            "AI Juggler-new.ips",
            &ips(OURS, "309"),
            T0 + 5_000,
        );
        report_at(
            &reports,
            "Other-new.ips",
            &ips("com.example.other", "309"),
            T0 + 5_000,
        );
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 300_000), 1);
        assert_eq!(
            std::fs::read_dir(crashes.join("asking")).unwrap().count(),
            1
        );
        // Looked at once: not kept again.
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 400_000), 0);
    }

    #[test]
    fn under_always_a_native_crash_is_queued_with_the_version_that_crashed() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("r");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        super::start_from(&crashes, T0);
        report_at(&reports, "AI Juggler-1.ips", &ips(OURS, "309"), T0 + 1_000);
        assert_eq!(collect(&reports, &crashes, OURS, "always", T0 + 300_000), 1);
        let queued: Vec<_> = std::fs::read_dir(crashes.join("queued"))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(queued.len(), 1);
        let report: crate::crash::CrashReport =
            serde_json::from_slice(&std::fs::read(queued[0].path()).unwrap()).unwrap();
        assert_eq!(
            report.app_version, "0.0.6",
            "the header's version, not this build's"
        );
    }

    /// Turned on in this session (`crash_set` moves the mark): a crash
    /// later in the same session is picked up at the next launch.
    #[test]
    fn a_crash_after_turning_reports_on_is_picked_up() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("r");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        super::start_from(&crashes, T0);
        report_at(&reports, "AI Juggler-1.ips", &ips(OURS, "309"), T0 + 10);
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 300_000), 1);
    }

    /// The watchdog can relaunch before macOS has finished writing: a young
    /// report that doesn't parse yet is looked at again next time.
    #[test]
    fn a_report_still_being_written_waits_for_the_next_look() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("r");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        super::start_from(&crashes, T0);
        let whole = ips(OURS, "309");
        let half = &whole[..whole.len() / 2];
        report_at(&reports, "AI Juggler-1.ips", half, T0 + 1_000);
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 2_000), 0);
        report_at(&reports, "AI Juggler-1.ips", &whole, T0 + 1_000);
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 70_000), 1);
    }

    #[test]
    fn links_and_oversized_files_are_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("r");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        super::start_from(&crashes, T0);
        let elsewhere = dir.path().join("elsewhere.ips");
        report_at(dir.path(), "elsewhere.ips", &ips(OURS, "309"), T0 + 1_000);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&elsewhere, reports.join("AI Juggler-link.ips")).unwrap();
        let mut big = ips(OURS, "309");
        big.push_str(&" ".repeat(5 * 1024 * 1024));
        report_at(&reports, "AI Juggler-big.ips", &big, T0 + 1_000);
        assert_eq!(collect(&reports, &crashes, OURS, "ask", T0 + 300_000), 0);
    }
}
