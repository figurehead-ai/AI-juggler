//! A minimal Tauri app, shaped like AI Juggler, to measure two ways of drawing the Floor in a window
//! that pops out beside the main one:
//!   option 1 (`--open web`): a second webview window drawing with Canvas 2D (web/floor.html);
//!   option 3 (`--open native`): a second, native window of the same process drawn with wgpu (native.rs).
//! Both draw the same scene (scene.rs) and only when something changes.
//!
//! Switches for measuring: --anim 0|10|30 (frames a second while people animate; 0 = nothing moves),
//! --cover-after S, --minimize-after S, --exit-after S, --capture DIR.

mod scene;
#[cfg(feature = "native")]
mod native;

use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

static SCENE: OnceLock<scene::Scene> = OnceLock::new();
static OPENED: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Clone, Default)]
struct Args {
    open: Option<String>,
    anim: u32,
    cover_after: Option<f64>,
    minimize_after: Option<f64>,
    exit_after: Option<f64>,
    capture: Option<String>,
    /// Keep the Floor window above other apps' windows, so a measured "visible" run is really visible.
    on_top: bool,
    /// Native only: a surface the size of the art, scaled up by the window system.
    art_surface: bool,
    /// The Floor window's size in points.
    size: (f64, f64),
}

fn args() -> Args {
    let v: Vec<String> = std::env::args().collect();
    let get = |k: &str| v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned();
    Args {
        open: get("--open"),
        anim: get("--anim").and_then(|s| s.parse().ok()).unwrap_or(10),
        cover_after: get("--cover-after").and_then(|s| s.parse().ok()),
        minimize_after: get("--minimize-after").and_then(|s| s.parse().ok()),
        exit_after: get("--exit-after").and_then(|s| s.parse().ok()),
        capture: get("--capture"),
        on_top: v.iter().any(|a| a == "--on-top"),
        art_surface: v.iter().any(|a| a == "--art-surface"),
        size: get("--size").and_then(|s| { let (w, h) = s.split_once('x')?; Some((w.parse().ok()?, h.parse().ok()?)) }).unwrap_or((1278.0, 798.0)),
    }
}

fn log(v: serde_json::Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn scene() -> &'static scene::Scene {
    SCENE.get_or_init(|| {
        let t0 = Instant::now();
        let s = scene::build();
        log(serde_json::json!({ "event": "scene_built", "ms": t0.elapsed().as_secs_f64() * 1000.0, "desks": s.meta.desks, "people": s.meta.people, "animated": s.meta.animated, "sprites": s.meta.sprites.len(), "atlas": [s.atlas.w, s.atlas.h] }));
        s
    })
}

#[tauri::command]
fn scene_meta(a: tauri::State<'_, Args>) -> serde_json::Value {
    serde_json::json!({ "meta": &scene().meta, "anim": a.anim })
}

/// The background, then the atlas, as raw RGBA bytes (no JSON).
#[tauri::command]
fn scene_pixels() -> tauri::ipc::Response {
    let s = scene();
    let mut v = Vec::with_capacity(s.bg.px.len() + s.atlas.px.len());
    v.extend_from_slice(&s.bg.px);
    v.extend_from_slice(&s.atlas.px);
    tauri::ipc::Response::new(v)
}

#[tauri::command]
fn first_frame(kind: String, marks: Option<serde_json::Value>) {
    let ms = OPENED.lock().unwrap().map(|t| t.elapsed().as_secs_f64() * 1000.0).unwrap_or(0.0);
    log(serde_json::json!({ "event": "first_frame", "kind": kind, "ms": ms, "page_marks_ms": marks }));
}

#[tauri::command]
fn frame_stats(kind: String, frames: u64, draw_ms: f64, hidden_events: u64, visibility: String) {
    log(serde_json::json!({ "event": "frame_stats", "kind": kind, "frames": frames, "draw_ms": draw_ms, "hidden_events": hidden_events, "visibility": visibility }));
}

#[tauri::command]
fn save_capture(request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let name = request.headers().get("x-name").and_then(|v| v.to_str().ok()).ok_or("x-name")?.to_string();
    let dir = args().capture.ok_or("no --capture")?;
    if let tauri::ipc::InvokeBody::Raw(bytes) = request.body() {
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.rgba")), bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The round trip: anything on the Floor that means "go there" brings the main window forward on that screen.
#[tauri::command]
fn open_main(app: AppHandle, screen: String) {
    show_main(&app, &screen);
}

#[tauri::command]
async fn open_floor(app: AppHandle, kind: String) -> Result<(), String> {
    open_floor_window(&app, &kind).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_floor(app: AppHandle) {
    if let Some(w) = app.get_webview_window("floor-web") { let _ = w.set_focus(); }
    #[cfg(feature = "native")]
    if let Some(w) = app.get_window("floor-native") { let _ = w.set_focus(); }
}

fn show_main(app: &AppHandle, screen: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app.emit_to("main", "navigate", screen.to_string());
    log(serde_json::json!({ "event": "open_main", "screen": screen }));
}

fn open_floor_window(app: &AppHandle, kind: &str) -> tauri::Result<()> {
    let a = args();
    *OPENED.lock().unwrap() = Some(Instant::now());
    match kind {
        "web" => {
            WebviewWindowBuilder::new(app, "floor-web", WebviewUrl::App(format!("floor.html?anim={}", a.anim).into()))
                .title("Floor — Canvas 2D")
                .inner_size(a.size.0, a.size.1)
                .always_on_top(a.on_top)
                .focused(true)
                .build()?;
        }
        #[cfg(feature = "native")]
        "native" => {
            let opened = OPENED.lock().unwrap().unwrap();
            let n = native::open(app, scene(), a.anim, a.on_top, a.art_surface, a.size, opened)?;
            app.manage(n);
        }
        _ => {}
    }
    Ok(())
}

fn floor_window_geometry(app: &AppHandle) -> Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>, f64)> {
    if let Some(w) = app.get_webview_window("floor-web") {
        return Some((w.outer_position().ok()?, w.outer_size().ok()?, w.scale_factor().ok()?));
    }
    let w = app.get_window("floor-native")?;
    Some((w.outer_position().ok()?, w.outer_size().ok()?, w.scale_factor().ok()?))
}

fn report(app: &AppHandle) {
    let _ = app.emit_to("floor-web", "report", ());
    #[cfg(feature = "native")]
    if let Some(n) = app.try_state::<native::Native>() {
        let s = *n.stats.lock().unwrap();
        log(serde_json::json!({ "event": "frame_stats", "kind": "native", "frames": s.frames, "draw_ms": s.draw_ms, "visibility_checks": s.visibility_checks }));
    }
}

fn after(secs: f64, app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f64(secs));
        f(&app);
    });
}

fn main() {
    let a = args();
    tauri::Builder::default()
        .manage(a.clone())
        .invoke_handler(tauri::generate_handler![scene_meta, scene_pixels, first_frame, frame_stats, save_capture, open_main, open_floor, focus_floor])
        .setup(move |app| {
            let h = app.handle().clone();
            // A native menu: its one item proves the round trip from the Floor to the main app.
            let open_sessions = MenuItemBuilder::with_id("open-sessions", "Open Sessions in the main window").accelerator("CmdOrCtrl+Shift+S").build(app)?;
            let app_menu = SubmenuBuilder::new(app, "Floor Spike").quit().build()?;
            let floor_menu = SubmenuBuilder::new(app, "Floor").item(&open_sessions).build()?;
            app.set_menu(MenuBuilder::new(app).items(&[&app_menu, &floor_menu]).build()?)?;
            app.on_menu_event(|app, ev| if ev.id() == "open-sessions" { show_main(app, "Sessions") });
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into())).title("AI Juggler (spike)").inner_size(900.0, 620.0).position(40.0, 60.0).build()?;
            if let Some(kind) = a.open.clone() {
                let _ = scene();
                open_floor_window(&h, &kind)?;
            }
            if let Some(s) = a.cover_after {
                after(s, &h, |app| {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Some((_pos, _size, _sf)) = floor_window_geometry(&app2) {
                            let (pos, size, sf) = match app2.primary_monitor() { Ok(Some(m)) => (*m.position(), *m.size(), m.scale_factor()), _ => return };
                            // An opaque window with a solid page over the whole Floor window.
                            let _ = WebviewWindowBuilder::new(&app2, "cover", WebviewUrl::App("cover.html".into()))
                                .title("cover")
                                .position(pos.x as f64 / sf, pos.y as f64 / sf)
                                .inner_size(size.width as f64 / sf, size.height as f64 / sf)
                                .always_on_top(true)
                                .build();
                            log(serde_json::json!({ "event": "covered" }));
                        }
                    });
                });
            }
            if let Some(s) = a.minimize_after {
                after(s, &h, |app| {
                    if let Some(w) = app.get_webview_window("floor-web") { let _ = w.minimize(); }
                    if let Some(w) = app.get_window("floor-native") { let _ = w.minimize(); }
                    log(serde_json::json!({ "event": "minimized" }));
                });
            }
            if let Some(dir) = a.capture.clone() {
                let _ = std::fs::create_dir_all(&dir);
                after(3.0, &h, move |app| {
                    for i in 0..6 {
                        let _ = app.emit_to("floor-web", "capture", format!("web-{i}"));
                        #[cfg(feature = "native")]
                        if let Some(n) = app.try_state::<native::Native>() {
                            let _ = n.tx.send(native::Msg::Capture(format!("{dir}/native-{i}.rgba")));
                        }
                        std::thread::sleep(Duration::from_millis(180));
                    }
                });
            }
            if let Some(s) = a.exit_after {
                after(s, &h, |app| {
                    report(app);
                    std::thread::sleep(Duration::from_millis(600));
                    log(serde_json::json!({ "event": "exit" }));
                    app.exit(0);
                });
            }
            Ok(())
        })
        .on_window_event(|w, ev| {
            #[cfg(feature = "native")]
            native::handle_event(w.app_handle(), w.label(), ev);
            #[cfg(not(feature = "native"))]
            let _ = (w, ev);
        })
        .run(tauri::generate_context!())
        .expect("run the spike");
}
