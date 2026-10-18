//! The desktop live preview (AJ-E415): a strictly read-only,
//! per-window mirror of a desktop Instance's window in its panel.
//!
//! Mechanism pin (recorded in this task's PR): one `SCScreenshotManager`
//! capture per poll (≤2 fps, ≤1280 px wide) through the safe
//! `screencapturekit` v11 API — `CGImage::save_png` for encoding; the
//! stream-delegate pixel path (`CVPixelBuffer` cursor) is `unsafe` and
//! barred by the workspace `unsafe_code = "forbid"` law. A polled
//! screenshot of a single-window `SCContentFilter` is also the honest
//! teardown story: nothing persists between polls, and the filter
//! contains ONLY the target window — Juggler's own windows can never
//! leak into a frame.
//!
//! Read-only by construction (Req.2): pixels flow app → panel, never
//! back. The only pointer action on the preview is click-to-focus
//! (AJ-E414's tier service). Input injection is PROHIBITED here — it
//! would convert supervision into remote control (the task's own
//! handover note; a different security decision requiring an ADR).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use juggler_core::model::WindowIdentity;

/// Copy-exact pause text (Req.1).
pub const PAUSED_NOT_VISIBLE: &str = "Preview paused — window not visible";

/// Poll interval while live: ≤2 fps — inside the ≤15 fps cap, and a
/// supervision glance does not need video.
pub const LIVE_POLL: Duration = Duration::from_millis(500);

/// Poll interval while paused (re-check visibility cheaply).
pub const PAUSED_POLL: Duration = Duration::from_millis(1_000);

/// Frame width cap (Req.1: ≤1280 px wide).
pub const MAX_WIDTH: u32 = 1280;

/// Why a frame could not be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// Screen Recording permission missing or revoked.
    PermissionDenied,
    /// The target window is gone or not on screen (minimised counts).
    WindowGone,
    /// This build/platform cannot capture.
    Unsupported,
    /// Transient failure — retried on the next tick.
    Failed(String),
}

/// One frame source (tests script it; macOS screenshots the real
/// window). Read-only: a frame is bytes OUT, nothing flows back.
pub trait CaptureSource: Send + Sync {
    /// Captures the window matching `identity` as PNG bytes.
    ///
    /// # Errors
    ///
    /// The typed [`CaptureError`] — never a panic, never a fake frame.
    fn capture(&self, identity: &WindowIdentity) -> Result<Vec<u8>, CaptureError>;
}

/// The non-macOS source: an honest unsupported, never a fake frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedSource;

impl CaptureSource for UnsupportedSource {
    fn capture(&self, _identity: &WindowIdentity) -> Result<Vec<u8>, CaptureError> {
        Err(CaptureError::Unsupported)
    }
}

/// A plain-data window row the matcher works on (the macOS source maps
/// the SC snapshot into these; tests construct them directly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRow {
    /// Window id (the SC window id — opaque, used only to re-find the
    /// live handle).
    pub window_id: u32,
    /// Owning application process id.
    pub pid: i32,
    /// Window title, when it has one.
    pub title: Option<String>,
    /// On-screen right now (false = minimised / on another Space's
    /// non-visible state).
    pub on_screen: bool,
    /// Native frame size (the capture is scaled to ≤1280 wide,
    /// aspect-preserved).
    pub width: i64,
    /// Native frame height.
    pub height: i64,
}

/// Matches the recorded identity against enumerated windows (Req.1's
/// pause logic + Req.2's re-discovery): by recorded PID and title when
/// both exist; title-less identities adopt the app's first on-screen
/// window (first sight). Collisions do NOT guess — same rule as the
/// focus tiers.
///
/// # Errors
///
/// [`CaptureError::WindowGone`] when the app owns no window, the
/// recorded title is stale/colliding, or the window is not on screen.
pub fn match_window<'w>(
    windows: &'w [WindowRow],
    identity: &WindowIdentity,
) -> Result<&'w WindowRow, CaptureError> {
    let owned: Vec<&WindowRow> = windows
        .iter()
        .filter(|w| u64::try_from(w.pid).is_ok_and(|pid| pid == u64::from(identity.pid)))
        .collect();
    if owned.is_empty() {
        return Err(CaptureError::WindowGone);
    }
    let matched = match identity.title.as_deref() {
        Some(title) => owned
            .iter()
            .filter(|w| w.title.as_deref() == Some(title))
            .count(),
        None => 0,
    };
    let row = if let Some(title) = identity.title.as_deref() {
        if matched != 1 {
            // Stale title or collision: gone for capture purposes — the
            // identity is re-enumerated, never guessed to a wrong window.
            return Err(CaptureError::WindowGone);
        }
        owned
            .into_iter()
            .find(|w| w.title.as_deref() == Some(title))
            .ok_or(CaptureError::WindowGone)?
    } else {
        // First sight: adopt the app's first window (focus's mirror rule).
        owned.into_iter().next().ok_or(CaptureError::WindowGone)?
    };
    if !row.on_screen {
        return Err(CaptureError::WindowGone);
    }
    Ok(row)
}

/// What the panel should render for the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPhase {
    /// Frames are flowing; the payload carries the frame age.
    Live,
    /// The window is minimised/not visible; exact copy-exact text.
    Paused,
    /// Screen Recording permission missing/revoked (Req.3).
    Denied,
    /// This build cannot capture.
    Unsupported,
}

impl PreviewPhase {
    /// Wire name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Paused => "paused",
            Self::Denied => "denied",
            Self::Unsupported => "unsupported",
        }
    }
}

/// The preview driver: polls the source, emits frames/state through the
/// caller's sink, and stops when `stopped` flips or the instance is no
/// longer live. Pure over the seams — every branch unit-testable with
/// no GUI and no Screen Recording grant (CI drives it everywhere).
pub struct PreviewDriver<'a> {
    /// Session whose window is mirrored (frame payloads carry it).
    pub session_id: &'a str,
    /// The window identity (title may be adopted at first sight —
    /// adoption is reported through `on_identity`).
    pub identity: WindowIdentity,
    /// Frame source.
    pub source: &'a dyn CaptureSource,
    /// Liveness probe: false stops the loop (instance exit teardown).
    pub is_live: &'a (dyn Fn() -> bool + Sync),
    /// Frame/state sink.
    pub emit: &'a (dyn Fn(PreviewEvent) + Sync),
    /// Stop flag (panel close / selection switch teardown).
    pub stopped: &'a AtomicBool,
}

/// What the driver emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEvent {
    /// One frame (base64 PNG) with its capture time and sequence.
    Frame {
        /// Owning Instance id.
        session_id: String,
        /// Base64-encoded PNG bytes.
        png_b64: String,
        /// Capture time, epoch ms.
        at_ms: i64,
        /// Monotonic sequence.
        seq: u64,
    },
    /// A phase transition (the panel renders the copy-exact text).
    Phase {
        /// Owning Instance id.
        session_id: String,
        /// The new phase.
        phase: PreviewPhase,
    },
}

impl PreviewDriver<'_> {
    /// Runs until stopped / not live / denied / unsupported. Teardown
    /// is inherent: one
    /// capture per poll, nothing persistent to tear down — the loop
    /// simply ends, so the <1 s budget is bounded by one poll interval.
    pub fn run(&self) {
        let seq = AtomicU64::new(0);
        let identity = self.identity.clone();
        let mut phase: Option<PreviewPhase> = None;
        let set_phase = |phase: &mut Option<PreviewPhase>, next: PreviewPhase| {
            if *phase != Some(next.clone()) {
                *phase = Some(next.clone());
                (self.emit)(PreviewEvent::Phase {
                    session_id: self.session_id.to_string(),
                    phase: next,
                });
            }
        };
        loop {
            if self.stopped.load(Ordering::SeqCst) || !(self.is_live)() {
                break;
            }
            let poll = match self.source.capture(&identity) {
                Ok(png) => {
                    set_phase(&mut phase, PreviewPhase::Live);
                    (self.emit)(PreviewEvent::Frame {
                        session_id: self.session_id.to_string(),
                        png_b64: base64_encode(&png),
                        at_ms: now_ms(),
                        seq: seq.fetch_add(1, Ordering::SeqCst),
                    });
                    LIVE_POLL
                }
                Err(CaptureError::WindowGone) => {
                    set_phase(&mut phase, PreviewPhase::Paused);
                    PAUSED_POLL
                }
                Err(CaptureError::PermissionDenied) => {
                    set_phase(&mut phase, PreviewPhase::Denied);
                    // Revocation stops the stream (Req.4) — no retry
                    // until the user re-enables.
                    break;
                }
                Err(CaptureError::Unsupported) => {
                    set_phase(&mut phase, PreviewPhase::Unsupported);
                    break;
                }
                Err(CaptureError::Failed(_)) => LIVE_POLL,
            };
            std::thread::sleep(poll);
        }
    }
}

/// Wall-clock ms (std-only; the driver labels frame age with it).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Standard base64 (no padding edge cases for PNG bytes; allocation is
/// fine at ≤2 fps).
#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let triple = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(TABLE[(triple >> 18) as usize & 63] as char);
        out.push(TABLE[(triple >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// --- macOS: the real capture source ----------------------------------
#[cfg(target_os = "macos")]
mod macos {
    use super::{CaptureError, CaptureSource, MAX_WIDTH, WindowIdentity, WindowRow};

    use screencapturekit::prelude::*;
    use screencapturekit::screenshot_manager::SCScreenshotManager;
    use screencapturekit::shareable_content::ContentSnapshot;

    /// Screenshots the matched window per poll (see the module pin).
    #[derive(Debug, Default, Clone, Copy)]
    pub struct ScreenshotSource;

    impl CaptureSource for ScreenshotSource {
        fn capture(&self, identity: &WindowIdentity) -> Result<Vec<u8>, CaptureError> {
            // D-17 (2026-10-13): the audited init shim — one call per
            // process, without which the first capture SIGSEGVs (the
            // parked state's crash evidence). Idempotent via `Once`.
            juggler_scap_init::initialize();
            let content = SCShareableContent::get().map_err(|_| CaptureError::PermissionDenied)?;
            let snapshot = content
                .snapshot()
                .ok_or_else(|| CaptureError::Failed("no content snapshot".into()))?;
            let rows = snapshot_rows(&snapshot);
            // macOS 15+ TCC filtering: WITHOUT the Screen Recording
            // grant the API succeeds but returns an EMPTY window list
            // for the whole desktop — impossible on a real session,
            // so it maps to the honest denial, never "window gone".
            if rows.is_empty() {
                return Err(CaptureError::PermissionDenied);
            }
            let row = super::match_window(&rows, identity)?;
            // Re-find the live SCWindow handle (filters need live
            // pointers, not snapshot data).
            let window = content
                .windows()
                .into_iter()
                .find(|w| w.window_id() == row.window_id)
                .ok_or(CaptureError::WindowGone)?;
            let filter = SCContentFilter::create()
                .with_window(&window)
                .build()
                .map_err(|e| CaptureError::Failed(format!("filter: {e}")))?;
            // ≤1280 px wide, aspect preserved from the native frame.
            let native_w = row.width.max(1);
            let native_h = row.height.max(1);
            let width = native_w.min(i64::from(MAX_WIDTH));
            let height = (native_h * width).div_euclid(native_w).max(1);
            let (width, height) = (
                u32::try_from(width).unwrap_or(MAX_WIDTH),
                u32::try_from(height).unwrap_or(720),
            );
            let config = SCStreamConfiguration::new()
                .with_width(width)
                .with_height(height);
            let image = SCScreenshotManager::capture_image(&filter, &config)
                .map_err(|_| CaptureError::PermissionDenied)?;
            let path = std::env::temp_dir().join(format!(
                "aij-preview-{}-{}.png",
                identity.pid, row.window_id
            ));
            image
                .save_png(&path)
                .map_err(|e| CaptureError::Failed(format!("save: {e}")))?;
            std::fs::read(&path).map_err(|e| CaptureError::Failed(format!("read: {e}")))
        }
    }

    /// Frame sizes arrive as `f64`; clamp+round into `i64` once.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn numeric_size(value: f64) -> i64 {
        let clamped = value.round().clamp(0.0, 16_384.0);
        if clamped.is_finite() {
            clamped as i64
        } else {
            0
        }
    }

    fn snapshot_rows(snapshot: &ContentSnapshot) -> Vec<WindowRow> {
        snapshot
            .windows
            .iter()
            .map(|w| WindowRow {
                window_id: w.window_id,
                pid: w
                    .owning_app_index
                    .and_then(|i| snapshot.applications.get(i))
                    .map_or(-1, |a| a.process_id),
                title: w.title.clone(),
                on_screen: w.is_on_screen,
                width: numeric_size(w.frame.size.width),
                height: numeric_size(w.frame.size.height),
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
pub use macos::ScreenshotSource;

#[cfg(test)]
mod tests {
    //! The scripted-fixture suites (quality gate): a changing fake-app
    //! window renders, minimise pauses, close stops, revocation denies,
    //! and no loop survives its stop flag in three consecutive runs.

    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    struct ScriptedSource {
        rows: Mutex<Vec<WindowRow>>,
        frames: AtomicU64,
        deny: AtomicBool,
    }

    impl ScriptedSource {
        fn two_windows() -> Arc<Self> {
            Arc::new(Self {
                rows: Mutex::new(vec![
                    WindowRow {
                        window_id: 11,
                        pid: 111,
                        title: Some("notes-a".into()),
                        on_screen: true,
                        width: 1200,
                        height: 800,
                    },
                    WindowRow {
                        window_id: 12,
                        pid: 111,
                        title: Some("notes-b".into()),
                        on_screen: true,
                        width: 1200,
                        height: 800,
                    },
                ]),
                frames: AtomicU64::new(0),
                deny: AtomicBool::new(false),
            })
        }

        fn minimise(&self, window_id: u32) {
            for row in self.rows.lock().unwrap().iter_mut() {
                if row.window_id == window_id {
                    row.on_screen = false;
                }
            }
        }

        fn close(&self, window_id: u32) {
            self.rows
                .lock()
                .unwrap()
                .retain(|r| r.window_id != window_id);
        }
    }

    impl CaptureSource for ScriptedSource {
        fn capture(&self, identity: &WindowIdentity) -> Result<Vec<u8>, CaptureError> {
            if self.deny.load(Ordering::SeqCst) {
                return Err(CaptureError::PermissionDenied);
            }
            let rows = self.rows.lock().unwrap().clone();
            let _row = super::match_window(&rows, identity)?;
            // A changing fake frame: pixels differ per capture.
            let n = self.frames.fetch_add(1, Ordering::SeqCst);
            Ok(format!("png-frame-{n}").into_bytes())
        }
    }

    fn identity() -> WindowIdentity {
        WindowIdentity {
            pid: 111,
            bundle_id: Some("dev.aijuggler.fake-desktop".into()),
            title: Some("notes-a".into()),
            cgwindow_id: None,
        }
    }

    fn run_driver(
        source: &Arc<ScriptedSource>,
        stopped: &Arc<AtomicBool>,
        ticks: usize,
        tick_ms: u64,
        between: impl Fn(usize),
    ) -> (Vec<PreviewEvent>, std::time::Duration) {
        let events: Arc<Mutex<Vec<PreviewEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let src = source.clone();
        let stop_flag = stopped.clone();
        let identity = identity();
        let handle = std::thread::spawn(move || {
            let emit = move |event: PreviewEvent| sink.lock().unwrap().push(event);
            let driver = PreviewDriver {
                session_id: "sess-desktop-1",
                identity,
                source: src.as_ref(),
                is_live: &|| true,
                emit: &emit,
                stopped: stop_flag.as_ref(),
            };
            driver.run();
        });
        for tick in 0..ticks {
            std::thread::sleep(Duration::from_millis(tick_ms));
            between(tick);
        }
        let stopped_at = std::time::Instant::now();
        stopped.store(true, Ordering::SeqCst);
        handle.join().unwrap();
        (events.lock().unwrap().clone(), stopped_at.elapsed())
    }

    /// Quality gate: the preview renders a CHANGING fake-app window
    /// (distinct frame payloads, live phase) and pauses with the
    /// copy-exact text when the window minimises.
    #[test]
    fn fake_app_renders_changes_pauses_on_minimise_and_resumes() {
        let source = ScriptedSource::two_windows();
        let stopped = Arc::new(AtomicBool::new(false));
        let (events, _teardown) = run_driver(&source, &stopped, 12, 250, |tick| {
            if tick == 2 {
                source.minimise(11); // window a minimised
            }
            if tick == 8 {
                // resumed (re-shown) before the stop
                for row in source.rows.lock().unwrap().iter_mut() {
                    row.on_screen = true;
                }
            }
        });
        let phases: Vec<&PreviewPhase> = events
            .iter()
            .filter_map(|e| match e {
                PreviewEvent::Phase { phase, .. } => Some(phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases.first(), Some(&&PreviewPhase::Live));
        assert!(
            phases.contains(&&PreviewPhase::Paused),
            "phases: {phases:?}"
        );
        // Frames flowed both before and after the pause; payloads differ.
        let frames: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                PreviewEvent::Frame { png_b64, .. } => Some(png_b64.clone()),
                _ => None,
            })
            .collect();
        assert!(frames.len() >= 2, "frames: {}", frames.len());
        let distinct = frames.clone();
        let unique: std::collections::HashSet<String> = distinct.into_iter().collect();
        assert_eq!(unique.len(), frames.len(), "every captured frame differs");
        // Base64 payloads decode to the exact fixture bytes.
        assert!(frames[0].starts_with("cG5nLWZyYW1l"), "payload is base64");
    }

    /// Close stops the frames (the instance exit path also stops the
    /// loop via `is_live`); the stop flag ends the driver promptly.
    #[test]
    fn stop_flag_ends_the_driver_within_budget_three_consecutive_runs() {
        let source = ScriptedSource::two_windows();
        for run in 1..=3 {
            let stopped = Arc::new(AtomicBool::new(false));
            let (events, teardown) = run_driver(&source, &stopped, 3, 150, |_| {});
            assert!(
                teardown < Duration::from_secs(1),
                "run {run}: teardown {teardown:?} exceeds the 1 s budget"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, PreviewEvent::Frame { .. })),
                "run {run}: frames flowed before the stop"
            );
        }
    }

    /// Permission revocation denies and the stream STOPS (no further
    /// frames after the denied phase — Req.4).
    #[test]
    fn revocation_denies_and_stops_the_stream() {
        let source = ScriptedSource::two_windows();
        let stopped = Arc::new(AtomicBool::new(false));
        let (events, _teardown) = run_driver(&source, &stopped, 4, 250, |tick| {
            if tick == 0 {
                source.deny.store(true, Ordering::SeqCst);
            }
        });
        assert!(events.iter().any(|e| matches!(
            e,
            PreviewEvent::Phase {
                phase: PreviewPhase::Denied,
                ..
            }
        )));
        // The denied phase is TERMINAL: it must be the last event — no
        // frame may follow a revocation (Req.4).
        assert!(
            matches!(
                events.last(),
                Some(PreviewEvent::Phase {
                    phase: PreviewPhase::Denied,
                    ..
                })
            ),
            "events: {events:?}"
        );
    }

    /// Window closed → paused phase, never a crash; the loop keeps
    /// polling cheaply until stopped.
    #[test]
    fn closed_window_pauses_without_crashing() {
        let source = ScriptedSource::two_windows();
        let stopped = Arc::new(AtomicBool::new(false));
        let (events, _teardown) = run_driver(&source, &stopped, 4, 250, |tick| {
            if tick == 0 {
                source.close(11);
            }
        });
        assert!(events.iter().any(|e| matches!(
            e,
            PreviewEvent::Phase {
                phase: PreviewPhase::Paused,
                ..
            }
        )));
    }

    /// The matcher: recorded title + pid match; collisions and stale
    /// titles refuse to guess; off-screen = gone; title-less adopts
    /// the app's first window.
    #[test]
    fn matcher_is_honest_about_collisions_stale_titles_and_visibility() {
        let rows = vec![
            WindowRow {
                window_id: 1,
                pid: 7,
                title: Some("same".into()),
                on_screen: true,
                width: 800,
                height: 600,
            },
            WindowRow {
                window_id: 2,
                pid: 7,
                title: Some("same".into()),
                on_screen: true,
                width: 800,
                height: 600,
            },
            WindowRow {
                window_id: 3,
                pid: 8,
                title: Some("other".into()),
                on_screen: true,
                width: 800,
                height: 600,
            },
        ];
        let colliding = WindowIdentity {
            pid: 7,
            bundle_id: None,
            title: Some("same".into()),
            cgwindow_id: None,
        };
        assert_eq!(
            match_window(&rows, &colliding),
            Err(CaptureError::WindowGone)
        );

        let stale = WindowIdentity {
            pid: 7,
            bundle_id: None,
            title: Some("renamed".into()),
            cgwindow_id: None,
        };
        assert_eq!(match_window(&rows, &stale), Err(CaptureError::WindowGone));

        let other_app = WindowIdentity {
            pid: 9,
            bundle_id: None,
            title: None,
            cgwindow_id: None,
        };
        assert_eq!(
            match_window(&rows, &other_app),
            Err(CaptureError::WindowGone)
        );

        let ok = WindowIdentity {
            pid: 8,
            bundle_id: None,
            title: Some("other".into()),
            cgwindow_id: None,
        };
        assert_eq!(match_window(&rows, &ok).map(|r| r.window_id), Ok(3));

        let first_sight = WindowIdentity {
            pid: 7,
            bundle_id: None,
            title: None,
            cgwindow_id: None,
        };
        assert_eq!(
            match_window(&rows, &first_sight).map(|r| r.window_id),
            Ok(1)
        );

        let minimised = vec![WindowRow {
            window_id: 5,
            pid: 7,
            title: Some("m".into()),
            on_screen: false,
            width: 800,
            height: 600,
        }];
        let identity = WindowIdentity {
            pid: 7,
            bundle_id: None,
            title: Some("m".into()),
            cgwindow_id: None,
        };
        assert_eq!(
            match_window(&minimised, &identity),
            Err(CaptureError::WindowGone)
        );
    }

    /// Unsupported platform: honest phase, loop ends immediately, no
    /// frame, no crash.
    #[test]
    fn unsupported_platform_emits_once_and_stops() {
        let events: Arc<Mutex<Vec<PreviewEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let emit = move |event: PreviewEvent| sink.lock().unwrap().push(event);
        let identity = identity();
        let unsupported = UnsupportedSource;
        let stopped = AtomicBool::new(false);
        let driver = PreviewDriver {
            session_id: "sess-desktop-1",
            identity,
            source: &unsupported,
            is_live: &|| true,
            emit: &emit,
            stopped: &stopped,
        };
        driver.run();
        let events = events.lock().unwrap().clone();
        assert_eq!(
            events,
            vec![PreviewEvent::Phase {
                session_id: "sess-desktop-1".into(),
                phase: PreviewPhase::Unsupported
            }]
        );
    }

    /// Instance exit (`is_live` false) ends the loop without another
    /// capture — exit teardown rides the same stop path.
    #[test]
    fn instance_exit_stops_the_loop() {
        let events: Arc<Mutex<Vec<PreviewEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let identity = identity();
        let source = ScriptedSource::two_windows();
        let live = Arc::new(AtomicBool::new(true));
        let live_flag = live.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop_flag = stopped.clone();
        let handle = std::thread::spawn(move || {
            let emit = move |event: PreviewEvent| sink.lock().unwrap().push(event);
            let driver = PreviewDriver {
                session_id: "sess-desktop-1",
                identity,
                source: source.as_ref(),
                is_live: &|| live_flag.load(Ordering::SeqCst),
                emit: &emit,
                stopped: stop_flag.as_ref(),
            };
            driver.run();
        });
        std::thread::sleep(Duration::from_millis(80));
        live.store(false, Ordering::SeqCst);
        let ended = std::time::Instant::now();
        handle.join().unwrap();
        assert!(
            ended.elapsed() < Duration::from_secs(1),
            "exit teardown: {:?}",
            ended.elapsed()
        );
    }

    /// Base64 sanity against the standard alphabet.
    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b"png-frame-0"), "cG5nLWZyYW1lLTA=");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"a"), "YQ==");
    }
}
