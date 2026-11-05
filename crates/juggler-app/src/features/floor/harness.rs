//! The Floor's measurement harness (AJ-E3513 req. 3, AJ-E3501's frame log,
//! AJ-E3502's memory marks): only in a build with the `floor-harness`
//! feature, which no shipped build has, and only when
//! `AIJ_FLOOR_HARNESS` names the file to write. `scripts/floor_harness.py`
//! builds it, runs it on the replica and writes `docs/qa/floor/`.
//!
//! It opens the Floor itself and walks SPEC §9.7's budget cases: never
//! opened (the app's own baseline), at rest (Motion Off), Calm, Full for
//! 5 minutes with memory at three marks, covered, minimised, a data change
//! and closed. CPU is the whole app's (`ps`), memory its footprint
//! (`footprint`, which counts the GPU's surfaces as Activity Monitor
//! does), frames and drawing time the render thread's own counters. Then
//! it puts the settings back and quits.

use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::Manager;

use super::render::COUNTERS;
use super::store::{MOTION_KEY, ON_TOP_KEY};
use super::window::{self, LABEL};
use crate::state::AppState;

/// The variable naming the results file.
pub const VAR: &str = "AIJ_FLOOR_HARNESS";
/// Set: every case shorter (a check while fixing, not the record).
pub const QUICK: &str = "AIJ_FLOOR_HARNESS_QUICK";

/// Starts the harness when [`VAR`] is set.
pub fn start(app: &tauri::AppHandle) {
    let Some(out) = std::env::var_os(VAR) else {
        return;
    };
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("floor-harness".into())
        .spawn(move || {
            let report = run(&app);
            let _ = std::fs::write(&out, report);
            app.exit(0);
        });
}

/// One reading of the whole app.
#[derive(Clone, Copy, Debug)]
struct Reading {
    at: Instant,
    cpu_s: f64,
    footprint_mb: f64,
    frames: u64,
    draw_ns: u64,
    snapshots: u64,
    inputs: u64,
    resizes: u64,
    /// The Floor's own time: drawing, building scenes and the feed.
    floor_ns: u64,
}

fn cpu_seconds(pid: u32) -> f64 {
    let out = Command::new("/bin/ps")
        .args(["-o", "time=", "-p", &pid.to_string()])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    // [[hh:]mm:]ss.cc
    out.split(':').fold(0.0, |acc, part| {
        acc * 60.0 + part.parse::<f64>().unwrap_or(0.0)
    })
}

fn footprint_mb(pid: u32) -> f64 {
    let out = Command::new(juggler_adapter::system_tools::FOOTPRINT)
        .args(["-p", &pid.to_string()])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let Some(rest) = out.split("Footprint:").nth(1) else {
        return f64::NAN;
    };
    let mut words = rest.split_whitespace();
    let n: f64 = words
        .next()
        .and_then(|w| w.parse().ok())
        .unwrap_or(f64::NAN);
    match words.next() {
        Some("KB") => n / 1024.0,
        Some("GB") => n * 1024.0,
        Some("B") => n / 1024.0 / 1024.0,
        _ => n,
    }
}

/// The footprint's biggest categories (dirty memory), MB.
fn footprint_top(pid: u32) -> Vec<(String, f64)> {
    let out = Command::new(juggler_adapter::system_tools::FOOTPRINT)
        .args(["-p", &pid.to_string()])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mb = |n: &str, unit: &str| -> Option<f64> {
        let n: f64 = n.parse().ok()?;
        Some(match unit {
            "KB" => n / 1024.0,
            "GB" => n * 1024.0,
            "B" => n / 1024.0 / 1024.0,
            _ => n,
        })
    };
    let mut rows: Vec<(String, f64)> = out
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            // dirty unit clean unit reclaimable unit regions category…
            if words.len() < 8 {
                return None;
            }
            let dirty = mb(words[0], words[1])?;
            words[6].parse::<u64>().ok()?;
            Some((words[7..].join(" "), dirty))
        })
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    rows.truncate(10);
    rows
}

fn read() -> Reading {
    let pid = std::process::id();
    Reading {
        at: Instant::now(),
        cpu_s: cpu_seconds(pid),
        footprint_mb: footprint_mb(pid),
        frames: COUNTERS.frames.load(Ordering::Relaxed),
        draw_ns: COUNTERS.draw_ns.load(Ordering::Relaxed),
        snapshots: COUNTERS.snapshots.load(Ordering::Relaxed),
        inputs: COUNTERS.inputs.load(Ordering::Relaxed),
        resizes: COUNTERS.resizes.load(Ordering::Relaxed),
        floor_ns: COUNTERS.draw_ns.load(Ordering::Relaxed)
            + COUNTERS.build_ns.load(Ordering::Relaxed)
            + COUNTERS.feed_ns.load(Ordering::Relaxed),
    }
}

/// One case's numbers.
struct Case {
    name: &'static str,
    secs: f64,
    cpu_pct: f64,
    frames: u64,
    first_second_frames: u64,
    draw_ms_per_frame: f64,
    footprint_mb: f64,
    /// The window's last visibility answer (0 none, 1 seen, 2 not).
    visible: u8,
    /// Whether the window says it's minimised, and can be seen, now.
    window: (Option<bool>, Option<bool>),
    /// Snapshots (data news) the Floor took in meanwhile.
    snapshots: u64,
    /// Inputs, and of those resizes, meanwhile.
    inputs: (u64, u64),
    /// The Floor's own time as a share of one core, %.
    floor_pct: f64,
}

fn measure(
    app: &tauri::AppHandle,
    name: &'static str,
    secs: u64,
    marks: &[u64],
    memory: &mut Vec<(u64, f64)>,
) -> Case {
    let a = read();
    std::thread::sleep(Duration::from_secs(1));
    let first_second_frames = COUNTERS.frames.load(Ordering::Relaxed) - a.frames;
    let mut waited = 1;
    for &mark in marks {
        std::thread::sleep(Duration::from_secs(mark.saturating_sub(waited)));
        waited = mark.max(waited);
        memory.push((mark, footprint_mb(std::process::id())));
    }
    std::thread::sleep(Duration::from_secs(secs.saturating_sub(waited)));
    let b = read();
    let wall = b.at.duration_since(a.at).as_secs_f64();
    let frames = b.frames - a.frames;
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    let draw_ms_per_frame = if frames == 0 {
        0.0
    } else {
        (b.draw_ns - a.draw_ns) as f64 / 1e6 / frames as f64
    };
    Case {
        name,
        secs: wall,
        cpu_pct: 100.0 * (b.cpu_s - a.cpu_s) / wall,
        frames,
        first_second_frames,
        draw_ms_per_frame,
        footprint_mb: b.footprint_mb,
        visible: COUNTERS.visible.load(Ordering::Relaxed),
        window: window_now(app),
        snapshots: b.snapshots - a.snapshots,
        inputs: (b.inputs - a.inputs, b.resizes - a.resizes),
        #[allow(clippy::cast_precision_loss)] // nanoseconds over minutes: far below 2^52
        floor_pct: 100.0 * (b.floor_ns - a.floor_ns) as f64 / 1e9 / wall,
    }
}

/// The Floor window's own answers now (on the main thread).
fn window_now(app: &tauri::AppHandle) -> (Option<bool>, Option<bool>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_window(LABEL) {
            let _ = tx.send(format!(
                "{:?} {:?} on_top={:?} focused={:?}",
                w.outer_position().ok(),
                w.outer_size().ok(),
                w.is_always_on_top().ok(),
                w.is_focused().ok()
            ));
        }
    });
    if let Ok(line) = rx.recv_timeout(Duration::from_secs(2)) {
        eprintln!("floor-harness: window {line}");
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let answer = handle.get_window(LABEL).map_or((None, None), |w| {
            (
                w.is_minimized().ok(),
                juggler_window_events::visible(&w).ok(),
            )
        });
        let _ = tx.send(answer);
    });
    rx.recv_timeout(Duration::from_secs(2))
        .unwrap_or((None, None))
}

fn set_motion(app: &tauri::AppHandle, word: &str) {
    let state = app.state::<Arc<AppState>>();
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(MOTION_KEY, word);
    }
    window::reload(app);
}

fn on_main(app: &tauri::AppHandle, job: impl FnOnce(&tauri::AppHandle) + Send + 'static) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || job(&handle));
    std::thread::sleep(Duration::from_secs(2));
}

/// The harness's own cover: an opaque window without a webview, a little
/// bigger than the Floor and above it (the main window or another app's
/// may not cover it whole, and the desktop is shared).
const COVER: &str = "floor-harness-cover";

fn cover(app: &tauri::AppHandle) {
    on_main(app, |app| {
        let Some(floor) = app.get_window(LABEL) else {
            return;
        };
        let (Ok(pos), Ok(size), Ok(scale)) = (
            floor.outer_position(),
            floor.outer_size(),
            floor.scale_factor(),
        ) else {
            return;
        };
        let pos = pos.to_logical::<f64>(scale);
        let size = size.to_logical::<f64>(scale);
        let _ = tauri::window::WindowBuilder::new(app, COVER)
            .title("Cover")
            .decorations(false)
            .always_on_top(true)
            .position(pos.x - 40.0, pos.y - 40.0)
            .inner_size(size.width + 80.0, size.height + 80.0)
            .build();
    });
}

fn uncover(app: &tauri::AppHandle) {
    on_main(app, |app| {
        if let Some(cover) = app.get_window(COVER) {
            let _ = cover.close();
        }
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.set_always_on_top(true);
        }
    });
}

/// Keeps the Floor above the shared desktop's other windows while it's
/// measured (the harness's app isn't the one in front).
fn on_top(app: &tauri::AppHandle) {
    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.set_always_on_top(true);
        }
    });
}

/// Seconds for a case: as given, or a fifth in quick mode (10 at least).
fn secs(full: u64, quick: bool) -> u64 {
    if quick { (full / 5).max(10) } else { full }
}

#[allow(clippy::too_many_lines)] // one flat script of cases, read top to bottom
fn run(app: &tauri::AppHandle) -> String {
    let quick = std::env::var_os(QUICK).is_some();
    let state = Arc::clone(app.state::<Arc<AppState>>().inner());
    let kept = |key: &str| {
        state
            .storage
            .lock()
            .ok()
            .and_then(|s| s.app_settings().get(key).ok().flatten())
    };
    let (kept_motion, kept_on_top) = (kept(MOTION_KEY), kept(ON_TOP_KEY));
    // The desktop is shared: the Floor stays above other apps' windows
    // while it's measured (as Settings' Keep it on top does), so only the
    // harness's own cover hides it.
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(ON_TOP_KEY, "on");
    }
    let mut memory = Vec::new();
    let mut cases = Vec::new();
    // The app settles: its first scans and samples.
    std::thread::sleep(Duration::from_secs(if quick { 15 } else { 30 }));
    cases.push(measure(
        app,
        "Never opened (the app alone)",
        secs(60, quick),
        &[],
        &mut memory,
    ));
    let before = read();
    let memory_before = footprint_top(std::process::id());

    set_motion(app, "off");
    let _ = window::open(app);
    std::thread::sleep(Duration::from_secs(3));
    on_top(app);
    let first_ms = COUNTERS.first_ms.load(Ordering::Relaxed);
    let open_steps: Vec<serde_json::Value> = super::render::OPEN_STEPS
        .lock()
        .map(|steps| {
            steps
                .iter()
                .map(|(name, ms)| serde_json::json!({ "step": name, "at_ms": ms }))
                .collect()
        })
        .unwrap_or_default();
    cases.push(measure(
        app,
        "Open, at rest (Motion Off)",
        secs(60, quick),
        &[],
        &mut memory,
    ));
    let still = read();
    let memory_still = footprint_top(std::process::id());

    set_motion(app, "calm");
    std::thread::sleep(Duration::from_secs(3));
    cases.push(measure(
        app,
        "Animating, Calm",
        secs(60, quick),
        &[],
        &mut memory,
    ));

    set_motion(app, "full");
    std::thread::sleep(Duration::from_secs(3));
    let marks: &[u64] = if quick {
        &[10, 30, 55]
    } else {
        &[30, 150, 290]
    };
    cases.push(measure(
        app,
        "Animating, Full (5 minutes)",
        secs(300, quick),
        marks,
        &mut memory,
    ));
    let memory_animating = footprint_top(std::process::id());

    cover(app);
    cases.push(measure(
        app,
        "Covered by another window",
        secs(30, quick),
        &[],
        &mut memory,
    ));
    uncover(app);

    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.minimize();
        }
    });
    cases.push(measure(app, "Minimised", secs(30, quick), &[], &mut memory));
    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.unminimize();
        }
    });
    on_top(app);
    let after_minimised = window_now(app);

    // A data change: the scene rebuilt and on screen (Motion Off, so the
    // frame is the change's own).
    set_motion(app, "off");
    std::thread::sleep(Duration::from_secs(2));
    let mut changes = Vec::new();
    for _ in 0..10 {
        let frames = COUNTERS.frames.load(Ordering::Relaxed);
        window::reload(app);
        // poll: one data change at a time, 600 ms apart (its frame comes
        // within 100 ms); ten in all.
        std::thread::sleep(Duration::from_millis(600));
        let drawn = COUNTERS.frames.load(Ordering::Relaxed) > frames;
        #[allow(clippy::cast_precision_loss)] // microseconds, far below 2^52
        changes.push(drawn.then(|| COUNTERS.change_us.load(Ordering::Relaxed) as f64 / 1000.0));
    }
    #[allow(clippy::cast_precision_loss)]
    let build_ms = COUNTERS.build_ns.load(Ordering::Relaxed) as f64
        / 1e6
        / COUNTERS.builds.load(Ordering::Relaxed).max(1) as f64;

    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.close();
        }
    });
    cases.push(measure(app, "Closed", secs(30, quick), &[], &mut memory));
    let closed = read();
    let memory_closed = footprint_top(std::process::id());

    // Three more opens and closes, animating: what stays after closing
    // must not grow with each (a leak) — the system's allocator may keep
    // some of what was freed for reuse.
    set_motion(app, "full");
    let mut cycles = Vec::new();
    for _ in 0..6 {
        let _ = window::open(app);
        std::thread::sleep(Duration::from_secs(3));
        on_top(app);
        std::thread::sleep(Duration::from_secs(5));
        on_main(app, |app| {
            if let Some(floor) = app.get_window(LABEL) {
                let _ = floor.close();
            }
        });
        std::thread::sleep(Duration::from_secs(3));
        cycles.push(footprint_mb(std::process::id()));
    }
    std::thread::sleep(Duration::from_secs(if quick { 5 } else { 20 }));
    let closed_again = read();
    let memory_closed_again = footprint_top(std::process::id());

    if let Ok(storage) = state.storage.lock() {
        let settings = storage.app_settings();
        let _ = settings.set(MOTION_KEY, kept_motion.as_deref().unwrap_or("system"));
        let _ = settings.set(ON_TOP_KEY, kept_on_top.as_deref().unwrap_or("off"));
    }

    let (apps, sessions) = super::current(&state).map_or((0, 0), |s| {
        (
            s.apps.len(),
            s.apps.iter().map(|a| a.sessions.len()).sum::<usize>(),
        )
    });
    let top = |rows: &[(String, f64)]| -> Vec<serde_json::Value> {
        rows.iter()
            .map(|(what, mb)| serde_json::json!({ "category": what, "mb": (mb * 10.0).round() / 10.0 }))
            .collect()
    };
    let r1 = |x: f64| (x * 10.0).round() / 10.0;
    let report = serde_json::json!({
        "quick": quick,
        "cases": cases.iter().map(|c| serde_json::json!({
            "name": c.name,
            "secs": r1(c.secs),
            "cpu_pct": (c.cpu_pct * 1000.0).round() / 1000.0,
            "frames": c.frames,
            "first_second_frames": c.first_second_frames,
            "draw_ms_per_frame": (c.draw_ms_per_frame * 1000.0).round() / 1000.0,
            "footprint_mb": r1(c.footprint_mb),
            "visible_answer": c.visible,
            "minimised_now": c.window.0,
            "seen_now": c.window.1,
            "snapshots": c.snapshots,
            "inputs": c.inputs.0,
            "resizes": c.inputs.1,
            "floor_pct": (c.floor_pct * 1000.0).round() / 1000.0,
        })).collect::<Vec<_>>(),
        "apps": apps,
        "sessions": sessions,
        "first_frame_ms": first_ms,
        "open_steps": open_steps,
        "after_unminimise": { "minimised": after_minimised.0, "seen": after_minimised.1 },
        "footprint_before_mb": r1(before.footprint_mb),
        "footprint_open_still_mb": r1(still.footprint_mb),
        "footprint_closed_mb": r1(closed.footprint_mb),
        "footprint_closed_after_3_more_mb": r1(closed_again.footprint_mb),
        "footprint_after_each_more_mb": cycles.iter().map(|mb| r1(*mb)).collect::<Vec<_>>(),
        "memory_closed_after_3_more": top(&memory_closed_again),
        "full_memory_marks": memory.iter().map(|(s, mb)| serde_json::json!({ "at_s": s, "footprint_mb": r1(*mb) })).collect::<Vec<_>>(),
        "news": super::NEWS.lock().map(|n| n.clone()).unwrap_or_default(),
        "memory_before": top(&memory_before),
        "memory_still": top(&memory_still),
        "memory_animating": top(&memory_animating),
        "memory_closed": top(&memory_closed),
        "change_to_screen_ms": changes,
        "scene_build_ms_mean": (build_ms * 100.0).round() / 100.0,
    });
    serde_json::to_string_pretty(&report).unwrap_or_default()
}
