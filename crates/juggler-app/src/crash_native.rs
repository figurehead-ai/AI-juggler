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

/// Where macOS keeps this user's crash reports.
#[must_use]
pub fn system_reports() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library/Logs/DiagnosticReports"))
    } else {
        None
    }
}

/// Turns this app's new crash reports in `reports` into kept reports in
/// `crashes` (`setting` is the effective `never` | `ask` | `always`).
/// Returns how many were kept.
#[must_use]
pub fn collect(reports: &Path, crashes: &Path, bundle_id: &str, setting: &str) -> usize {
    if setting == "never" {
        return 0;
    }
    let seen_path = crashes.join(SEEN);
    let seen: Option<i64> = std::fs::read_to_string(&seen_path)
        .ok()
        .and_then(|text| text.trim().parse().ok());
    let mut newest = seen.unwrap_or(0);
    let mut kept = 0;
    let mut entries: Vec<(i64, PathBuf)> = std::fs::read_dir(reports)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ips"))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let at = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
            let at = i64::try_from(at.as_millis()).ok()?;
            (metadata.is_file() && metadata.len() <= MAX_REPORT_BYTES).then(|| (at, entry.path()))
        })
        .collect();
    entries.sort();
    for (at, path) in entries {
        newest = newest.max(at);
        // The first look only marks where to start.
        if seen.is_none_or(|seen| at <= seen) {
            continue;
        }
        let Some(report) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| native_report(&text, bundle_id, at))
        else {
            continue;
        };
        if keep(crashes, &report, setting).unwrap_or(false) {
            kept += 1;
        }
    }
    if seen.is_none() {
        newest = newest.max(juggler_core::model::Timestamp::now().millis());
    }
    if std::fs::create_dir_all(crashes).is_ok() {
        let _ = std::fs::write(&seen_path, format!("{newest}\n"));
    }
    kept
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
    Some(report_of(&message, &location, &frames.join("\n"), at_ms))
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

    #[test]
    fn only_crashes_after_the_first_look_are_kept_and_never_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("DiagnosticReports");
        let crashes = dir.path().join("crashes");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(reports.join("AI Juggler-old.ips"), ips(OURS, "309")).unwrap();

        // Never: nothing read, nothing noted.
        assert_eq!(collect(&reports, &crashes, OURS, "never"), 0);
        assert!(!crashes.exists());

        // The first look marks where to start: the old crash isn't kept.
        assert_eq!(collect(&reports, &crashes, OURS, "ask"), 0);
        assert!(crashes.join("native.seen").exists());

        // A crash after it is kept, waiting for the user's answer; another
        // app's is not.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let seen: i64 = std::fs::read_to_string(crashes.join("native.seen"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let later = std::time::UNIX_EPOCH + std::time::Duration::from_millis((seen + 5_000) as u64);
        for (name, bundle) in [
            ("AI Juggler-new.ips", OURS),
            ("Other-new.ips", "com.example.other"),
        ] {
            let path = reports.join(name);
            std::fs::write(&path, ips(bundle, "309")).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(later)
                .unwrap();
        }
        assert_eq!(collect(&reports, &crashes, OURS, "ask"), 1);
        assert_eq!(
            std::fs::read_dir(crashes.join("asking")).unwrap().count(),
            1
        );
        // Looked at once: not kept again.
        assert_eq!(collect(&reports, &crashes, OURS, "ask"), 0);
    }
}
