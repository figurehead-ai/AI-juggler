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

use std::fmt::Write as _;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::Manager;

use super::render::COUNTERS;
use super::store::MOTION_KEY;
use super::window::{self, LABEL};
use crate::state::AppState;

/// The variable naming the results file.
pub const VAR: &str = "AIJ_FLOOR_HARNESS";

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

fn read() -> Reading {
    let pid = std::process::id();
    Reading {
        at: Instant::now(),
        cpu_s: cpu_seconds(pid),
        footprint_mb: footprint_mb(pid),
        frames: COUNTERS.frames.load(Ordering::Relaxed),
        draw_ns: COUNTERS.draw_ns.load(Ordering::Relaxed),
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
}

fn measure(name: &'static str, secs: u64, marks: &[u64], memory: &mut Vec<(u64, f64)>) -> Case {
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
    }
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

/// Covers the Floor with the main window, the size of the screen.
fn cover(app: &tauri::AppHandle) {
    on_main(app, |app| {
        if let (Some(main), Some(floor)) = (app.get_window("main"), app.get_window(LABEL))
            && let (Ok(pos), Ok(size)) = (floor.outer_position(), floor.outer_size())
        {
            let _ = main.set_position(tauri::PhysicalPosition::new(pos.x - 40, pos.y - 40));
            let _ = main.set_size(tauri::PhysicalSize::new(size.width + 80, size.height + 80));
            let _ = main.set_always_on_top(true);
            crate::main_window::bring_forward(app);
        }
    });
}

#[allow(clippy::too_many_lines)] // one flat script of cases, read top to bottom
fn run(app: &tauri::AppHandle) -> String {
    let state = Arc::clone(app.state::<Arc<AppState>>().inner());
    let kept_motion = state
        .storage
        .lock()
        .ok()
        .and_then(|s| s.app_settings().get(MOTION_KEY).ok().flatten());
    let mut memory = Vec::new();
    let mut cases = Vec::new();
    // The app settles: its first scans and samples.
    std::thread::sleep(Duration::from_secs(30));
    cases.push(measure(
        "Never opened (the app alone)",
        60,
        &[],
        &mut memory,
    ));
    let before = read();

    set_motion(app, "off");
    let _ = window::open(app);
    std::thread::sleep(Duration::from_secs(5));
    let first_ms = COUNTERS.first_ms.load(Ordering::Relaxed);
    cases.push(measure("Open, at rest (Motion Off)", 60, &[], &mut memory));
    let still = read();

    set_motion(app, "calm");
    std::thread::sleep(Duration::from_secs(3));
    cases.push(measure("Animating, Calm", 60, &[], &mut memory));

    set_motion(app, "full");
    std::thread::sleep(Duration::from_secs(3));
    cases.push(measure(
        "Animating, Full (5 minutes)",
        300,
        &[30, 150, 290],
        &mut memory,
    ));

    cover(app);
    cases.push(measure("Covered by another window", 30, &[], &mut memory));
    on_main(app, |app| {
        if let Some(main) = app.get_window("main") {
            let _ = main.set_always_on_top(false);
        }
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.set_focus();
        }
    });

    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.minimize();
        }
    });
    cases.push(measure("Minimised", 30, &[], &mut memory));
    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.unminimize();
            let _ = floor.set_focus();
        }
    });

    // A data change: the scene rebuilt and on screen (Motion Off, so the
    // frame is the change's own).
    set_motion(app, "off");
    std::thread::sleep(Duration::from_secs(2));
    let mut changes = Vec::new();
    for _ in 0..10 {
        window::reload(app);
        // poll: one data change at a time, 600 ms apart (its frame comes
        // within 100 ms); ten in all.
        std::thread::sleep(Duration::from_millis(600));
        changes.push(COUNTERS.change_us.load(Ordering::Relaxed));
    }
    changes.sort_unstable();
    #[allow(clippy::cast_precision_loss)]
    let build_ms = COUNTERS.build_ns.load(Ordering::Relaxed) as f64
        / 1e6
        / COUNTERS.builds.load(Ordering::Relaxed).max(1) as f64;

    on_main(app, |app| {
        if let Some(floor) = app.get_window(LABEL) {
            let _ = floor.close();
        }
    });
    cases.push(measure("Closed", 30, &[], &mut memory));
    let closed = read();

    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .app_settings()
            .set(MOTION_KEY, kept_motion.as_deref().unwrap_or("system"));
    }

    let mut out = String::from("{\n  \"cases\": [\n");
    for (i, c) in cases.iter().enumerate() {
        let _ = writeln!(
            out,
            "    {{\"name\": \"{}\", \"secs\": {:.1}, \"cpu_pct\": {:.3}, \"frames\": {}, \"first_second_frames\": {}, \"draw_ms_per_frame\": {:.3}, \"footprint_mb\": {:.1}}}{}",
            c.name,
            c.secs,
            c.cpu_pct,
            c.frames,
            c.first_second_frames,
            c.draw_ms_per_frame,
            c.footprint_mb,
            if i + 1 < cases.len() { "," } else { "" }
        );
    }
    let marks: Vec<String> = memory
        .iter()
        .map(|(s, mb)| format!("{{\"at_s\": {s}, \"footprint_mb\": {mb:.1}}}"))
        .collect();
    let changes_ms: Vec<String> = changes
        .iter()
        .map(|us| {
            format!(
                "{:.1}",
                f64::from(u32::try_from(*us).unwrap_or(u32::MAX)) / 1000.0
            )
        })
        .collect();
    let (apps, sessions) = super::current(&state).map_or((0, 0), |s| {
        (
            s.apps.len(),
            s.apps.iter().map(|a| a.sessions.len()).sum::<usize>(),
        )
    });
    let _ = write!(
        out,
        "  ],\n  \"apps\": {apps},\n  \"sessions\": {sessions},\n  \"first_frame_ms\": {first_ms},\n  \"footprint_before_mb\": {:.1},\n  \"footprint_open_still_mb\": {:.1},\n  \"footprint_closed_mb\": {:.1},\n  \"full_memory_marks\": [{}],\n  \"change_to_screen_ms\": [{}],\n  \"scene_build_ms_mean\": {build_ms:.2}\n}}\n",
        before.footprint_mb,
        still.footprint_mb,
        closed.footprint_mb,
        marks.join(", "),
        changes_ms.join(", "),
    );
    out
}
