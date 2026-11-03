//! The Floor, second prototype: a low-poly 3D office in a native wgpu window beside a minimal main
//! window. The office's choices (each App's layout and look) are saved in a small state file.
//!
//! Switches: --anim FPS (8; 0 = nothing moves), --on-top, --cover-after S, --minimize-after S,
//! --exit-after S, --tour DIR (captures a scripted walk), --alert, --state PATH.
//! In the Floor window: drag or two-finger scroll to pan; right-drag, ⌥-drag, a two-finger twist or
//! Q / E and the arrow keys to turn and tilt; pinch, ⌘-scroll or + / - to zoom; H resets the view;
//! 0 building, 1 and 2 floors; click an area, a desk, a person or a helper to open it in the main
//! window; click plants, ping-pong, the coffee machine, beanbags or the cat for fun; A red alert;
//! G grows one App (the reflow demo).

mod input;
mod kit;
mod layout;
mod math;
mod mesh;
mod models;
mod paint;
mod render;
mod scene;
mod tex;

use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Default)]
struct Args { anim: u32, on_top: bool, cover_after: Option<f64>, minimize_after: Option<f64>, exit_after: Option<f64>, tour: Option<String>, alert: bool, state: Option<String> }

fn args() -> Args {
    let v: Vec<String> = std::env::args().collect();
    let get = |k: &str| v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned();
    Args {
        anim: get("--anim").and_then(|s| s.parse().ok()).unwrap_or(8),
        on_top: v.iter().any(|a| a == "--on-top"),
        cover_after: get("--cover-after").and_then(|s| s.parse().ok()),
        minimize_after: get("--minimize-after").and_then(|s| s.parse().ok()),
        exit_after: get("--exit-after").and_then(|s| s.parse().ok()),
        tour: get("--tour"),
        alert: v.iter().any(|a| a == "--alert"),
        state: get("--state"),
    }
}

fn log(v: serde_json::Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn show_main(app: &AppHandle, screen: &str, detail: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app.emit_to("main", "navigate", serde_json::json!({ "screen": screen, "detail": detail }));
    log(serde_json::json!({ "event": "open_main", "screen": screen, "detail": detail }));
}

#[tauri::command]
fn focus_floor(app: AppHandle) {
    if let Some(w) = app.get_window("floor") { let _ = w.set_focus(); }
}

#[cfg(target_os = "macos")]
fn window_visible(w: &tauri::Window) -> bool {
    let Ok(ptr) = w.ns_window() else { return true };
    let nsw: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
    nsw.occlusionState().contains(objc2_app_kit::NSWindowOcclusionState::Visible) && !nsw.isMiniaturized()
}
#[cfg(not(target_os = "macos"))]
fn window_visible(w: &tauri::Window) -> bool {
    !w.is_minimized().unwrap_or(false) && w.is_visible().unwrap_or(true)
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
        .invoke_handler(tauri::generate_handler![focus_floor])
        .setup(move |app| {
            let h = app.handle().clone();
            let state_path = match &a.state { Some(p) => std::path::PathBuf::from(p), None => app.path().app_data_dir()?.join("floor-state.json") };
            // the menu: the way back to the main window, and the floors
            let items = [
                MenuItemBuilder::with_id("main", "Open the main window").accelerator("CmdOrCtrl+Shift+M").build(app)?,
                MenuItemBuilder::with_id("bld", "Building").accelerator("CmdOrCtrl+0").build(app)?,
                MenuItemBuilder::with_id("f1", "Floor 1").accelerator("CmdOrCtrl+1").build(app)?,
                MenuItemBuilder::with_id("f2", "Floor 2").accelerator("CmdOrCtrl+2").build(app)?,
                MenuItemBuilder::with_id("reset", "Reset the view").accelerator("CmdOrCtrl+R").build(app)?,
                MenuItemBuilder::with_id("alert", "Red alert (demo)").build(app)?,
                MenuItemBuilder::with_id("grow", "Grow an App (reflow demo)").build(app)?,
                MenuItemBuilder::with_id("recreate", "Re-create the office…").build(app)?,
            ];
            let app_menu = SubmenuBuilder::new(app, "Floor Office").quit().build()?;
            let mut floor_menu = SubmenuBuilder::new(app, "Floor");
            for it in &items { floor_menu = floor_menu.item(it); }
            app.set_menu(MenuBuilder::new(app).items(&[&app_menu, &floor_menu.build()?]).build()?)?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into())).title("AI Juggler (prototype)").inner_size(760.0, 520.0).position(30.0, 60.0).build()?;
            // the Floor: a native window drawn with wgpu
            let opened = Instant::now();
            let fw = tauri::window::WindowBuilder::new(app, "floor").title("The Floor").inner_size(1278.0, 798.0).always_on_top(a.on_top).focused(true).build()?;
            let size = fw.inner_size()?;
            let sf = fw.scale_factor()? as f32;
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(fw.clone())));
            let (stx, srx) = channel();
            {
                let (w, inst) = (fw.clone(), instance.clone());
                app.run_on_main_thread(move || {
                    let target = unsafe { wgpu::SurfaceTargetUnsafe::from_window(&w) }.expect("window handle");
                    let _ = stx.send(unsafe { inst.create_surface_unsafe(target) }.expect("surface"));
                })?;
            }
            let surface = srx.recv().expect("surface");
            let ho = h.clone();
            let on_open: render::OnOpen = Arc::new(move |t, label| match t {
                scene::Target::App(_) => show_main(&ho, "App", &label),
                scene::Target::Session(..) => show_main(&ho, "Session", &label),
                scene::Target::Helpers(..) => show_main(&ho, "Subagents", &label),
                scene::Target::Account(_) => show_main(&ho, "Usage", &label),
                scene::Target::ThisMac => show_main(&ho, "This Mac", &label),
                _ => {}
            });
            let (hv, wv) = (h.clone(), fw.clone());
            let ask_visible: Arc<dyn Fn(Sender<render::Msg>) + Send + Sync> = Arc::new(move |tx| {
                let w = wv.clone();
                let _ = hv.run_on_main_thread(move || { let _ = tx.send(render::Msg::Visible(window_visible(&w))); });
            });
            let floor = render::start(instance, surface, state_path, (size.width, size.height, sf), a.anim, opened, on_open, ask_visible, a.alert);
            {
                let (w, tx) = (fw.clone(), floor.tx.clone());
                app.run_on_main_thread(move || input::install(&w, tx))?;
            }
            let txm = floor.tx.clone();
            app.on_menu_event(move |app, ev| match ev.id().as_ref() {
                "main" => show_main(app, "Home", ""),
                "bld" => { let _ = txm.send(render::Msg::Key('0')); }
                "f1" => { let _ = txm.send(render::Msg::Key('1')); }
                "f2" => { let _ = txm.send(render::Msg::Key('2')); }
                "reset" => { let _ = txm.send(render::Msg::Key('h')); }
                "alert" => { let _ = txm.send(render::Msg::Key('a')); }
                "grow" => { let _ = txm.send(render::Msg::Key('g')); }
                "recreate" => { let _ = txm.send(render::Msg::Recreate); }
                _ => {}
            });
            let txr = floor.tx.clone();
            fw.on_window_event(move |ev| {
                if let tauri::WindowEvent::Resized(s) = ev { let _ = txr.send(render::Msg::Resize(s.width, s.height, sf)); }
            });
            let stats = floor.stats.clone();
            app.manage(floor);
            if let Some(dir) = a.tour.clone() {
                let tx = app.state::<render::Floor>().tx.clone();
                after(2.5, &h, move |_| { let _ = tx.send(render::Msg::Tour(dir)); });
            }
            if let Some(s) = a.cover_after {
                after(s, &h, |app| {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        // cover the display the Floor is on, which need not be the primary one
                        if let Some(Ok(Some(m))) = app2.get_window("floor").map(|w| w.current_monitor()) {
                            let (p, sz, k) = (*m.position(), *m.size(), m.scale_factor());
                            let _ = WebviewWindowBuilder::new(&app2, "cover", WebviewUrl::App("cover.html".into())).position(p.x as f64 / k, p.y as f64 / k).inner_size(sz.width as f64 / k, sz.height as f64 / k).always_on_top(true).build();
                            log(serde_json::json!({ "event": "covered" }));
                        }
                    });
                });
            }
            if let Some(s) = a.minimize_after {
                after(s, &h, |app| { if let Some(w) = app.get_window("floor") { let _ = w.minimize(); } log(serde_json::json!({ "event": "minimized" })); });
            }
            if let Some(s) = a.exit_after {
                after(s, &h, move |app| {
                    let st = *stats.lock().unwrap();
                    log(serde_json::json!({ "event": "frame_stats", "frames": st.frames, "draw_ms": st.draw_ms, "visibility_checks": st.visibility_checks, "instances": st.instances }));
                    std::thread::sleep(Duration::from_millis(300));
                    log(serde_json::json!({ "event": "exit" }));
                    app.exit(0);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run the prototype");
}
