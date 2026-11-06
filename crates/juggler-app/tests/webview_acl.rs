//! AJ-E2003: the app's real capability files decide what each window may
//! call. Built from `tauri::generate_context!()` (this crate's own
//! `tauri.conf.json`, `capabilities/` and `permissions/`), on Tauri's mock
//! runtime, so the access list itself is under test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// A stand-in for a registered command: the access list is what's tested,
/// not the command.
#[tauri::command]
fn app_info() -> &'static str {
    "ok"
}

/// Another registered command's name, not in the terminal window's set.
#[tauri::command]
fn projects_list() -> &'static str {
    "ok"
}

fn call(window: &tauri::WebviewWindow<tauri::test::MockRuntime>, cmd: &str) -> Result<(), String> {
    call_from(window, cmd, "tauri://localhost")
}

fn call_from(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    url: &str,
) -> Result<(), String> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::default(),
            headers: tauri::http::HeaderMap::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn app() -> tauri::App<tauri::test::MockRuntime> {
    let mut context: tauri::Context<tauri::test::MockRuntime> = tauri::generate_context!();
    // The real assets aren't needed (or built) for the access list.
    context.set_assets(Box::new(tauri::test::noop_assets()));
    mock_builder()
        .invoke_handler(tauri::generate_handler![app_info, projects_list])
        .build(context)
        .unwrap()
}

#[test]
fn the_main_window_may_call_the_registered_commands() {
    let app = app();
    let main = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    assert_eq!(call(&main, "app_info"), Ok(()));
    assert_eq!(call(&main, "projects_list"), Ok(()));
}

#[test]
fn a_terminal_window_may_call_only_its_own() {
    let app = app();
    let terminal = WebviewWindowBuilder::new(&app, "terminal-abc", WebviewUrl::default())
        .build()
        .unwrap();
    assert_eq!(call(&terminal, "app_info"), Ok(()));
    assert!(
        call(&terminal, "projects_list").is_err(),
        "not in the terminal window's set"
    );
}

#[test]
fn an_unknown_window_may_call_nothing() {
    let app = app();
    let other = WebviewWindowBuilder::new(&app, "elsewhere", WebviewUrl::default())
        .build()
        .unwrap();
    assert!(call(&other, "app_info").is_err());
}

#[test]
fn the_dev_server_page_counts_as_the_apps_own() {
    let app = app();
    let main = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    assert_eq!(
        call_from(&main, "app_info", "http://localhost:5173/"),
        Ok(())
    );
    assert!(call_from(&main, "app_info", "https://example.com/").is_err());
}
