//! Tauri application shell (ADR-0001).
//!
//! Hosts the Tauri 2 commands and typed events that connect the webview UI
//! (React + `TypeScript`) to the Rust core, plus updater wiring (M5) and the
//! desktop entry point. The UI is a view: it holds no business logic; all
//! state originates in Rust and arrives as pre-aggregated, typed events
//! (`docs/plan/QUALITY.md` §4).

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod commands;
pub mod error;
pub mod focus;
pub mod preview;
pub mod state;
pub mod terminal;
pub mod usage_pipeline;

use std::sync::Arc;

use tauri::Emitter;

/// Name of the event emitted once the backend is ready; the payload is
/// the `AppInfo` value.
pub const EVENT_APP_READY: &str = "app://ready";

/// Name of the event carrying the refreshed session list.
pub const EVENT_SESSIONS_UPDATED: &str = "session://updated";

/// Name of the 5 s resource tick (I-07/I-08 deltas, AJ-E401): one
/// coalesced payload per tick for every active Instance.
pub const EVENT_SESSIONS_RESOURCES: &str = "session://resources";

/// Name of the 30 s quota tick (I-05 deltas, AJ-E401): one coalesced
/// payload per tick; per-Instance `quota` stays `null` until a feed
/// exists (AJ-E606+).
pub const EVENT_SESSIONS_QUOTA: &str = "session://quota";

/// Name of the event emitted when the global "Go to instance…"
/// hotkey fires (AJ-E414 Req.4): the payload is empty — the UI lists
/// live instances itself.
pub const EVENT_GO_TO: &str = "app://go-to";

/// The app-settings key holding the "Go to instance…" hotkey binding.
pub const GO_TO_HOTKEY_KEY: &str = "go_to_hotkey";

/// Default "Go to instance…" hotkey (SPEC §9.2 / deep-review §4.17:
/// `⌥Space`, rebindable).
pub const DEFAULT_GO_TO_HOTKEY: &str = "Alt+Space";

/// Name of the live-preview frame event (AJ-E415): payload
/// `PreviewFrameInfo`.
pub const EVENT_PREVIEW_FRAME: &str = "preview://frame";

/// Name of the live-preview phase event (AJ-E415): payload
/// `PreviewStateInfo`.
pub const EVENT_PREVIEW_STATE: &str = "preview://state";

/// Builds and runs the desktop application.
#[allow(clippy::too_many_lines)] // the command registry is one flat list by design
pub fn run() {
    let data_dir = bootstrap_data_dir();
    let app_state = Arc::new(state::AppState::init(data_dir).unwrap_or_else(|error| {
        eprintln!("ai-juggler: fatal: {error}");
        std::process::exit(1);
    }));

    spawn_lifecycle_forwarder(Arc::clone(&app_state));

    let attention_state = Arc::clone(&app_state);
    let heuristic_tick_state = Arc::clone(&app_state);
    let resource_tick_state = Arc::clone(&app_state);
    let processes_tick_state = Arc::clone(&app_state);
    let quota_tick_state = Arc::clone(&app_state);
    let pipeline_state = Arc::clone(&app_state);

    tauri::Builder::default()
        // Registered first: a second launch defers to the running app
        // (focus its window) instead of stacking ghost instances.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // The global "Go to instance…" hotkey (AJ-E414 Req.4): the
        // registration itself happens in setup (stored binding or the
        // default); a conflict there logs and leaves the in-app ⌘K path.
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = app.emit(EVENT_GO_TO, ());
                    }
                })
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::usage_source_health,
            commands::cost_rollups,
            commands::cost_csv_export,
            commands::pricing_override_set,
            commands::pricing_override_clear,
            commands::pricing_catalog_info,
            commands::quota_display,
            commands::quota_display_all,
            commands::account_set_fee,
            commands::ranking_rows,
            commands::alerts_tick,
            commands::reset_queue_queue,
            commands::reset_queue_list,
            commands::reset_queue_dismiss,
            commands::harnesses_list,
            commands::legacy_aliases::harnesses_list_alias,
            commands::accounts_list,
            commands::account_create,
            commands::account_login_state,
            commands::account_login_command,
            commands::environment_scan,
            commands::account_import,
            commands::source_inspect,
            commands::isolation_check,
            commands::funnel_export,
            commands::funnel_event,
            commands::expert_setup_info,
            commands::expert_reveal,
            commands::launch_recipe_env_get,
            commands::launch_recipe_env_set,
            commands::login_open_web,
            commands::profiles_list,
            commands::legacy_aliases::profiles_list_alias,
            commands::profile_create,
            commands::profile_delete,
            commands::launch_session,
            commands::sessions_list,
            commands::legacy_aliases::sessions_list_alias,
            commands::projects_list,
            commands::projects_backfill,
            commands::session_handover,
            commands::switch_preflight,
            commands::switch_execute,
            commands::switch_undo,
            commands::switch_back_plan,
            commands::switch_back_execute,
            commands::switch_back_suggestions,
            commands::redaction_policy_get,
            commands::redaction_policy_set_global,
            commands::redaction_policy_set_override,
            commands::switch_targets,
            commands::switch_graph_overview,
            commands::epoch_resume,
            commands::pack_export,
            commands::validation_matrix,
            commands::validation_dogfood_record,
            commands::providers_discover,
            commands::environment_scan_full,
            commands::scan_cancel,
            commands::recommendation_for,
            commands::stop_session,
            commands::dashboard_stats,
            commands::terminal_spawn_login,
            commands::terminal_attach,
            commands::terminal_write,
            commands::terminal_resize,
            commands::terminal_kill,
            commands::terminal_snapshot,
            commands::instance_quality,
            commands::instance_processes,
            commands::cleanup_inventory,
            commands::cleanup_audit_export,
            commands::port_inventory,
            commands::launch_port_warning,
            commands::orphan_stop,
            commands::orphan_stop_all,
            commands::orphan_verify,
            commands::kill_verify,
            commands::kill_execute,
            commands::kill_cancel,
            commands::kill_liveness,
            commands::cleanup_status,
            commands::cleanup_pause_set,
            commands::cleanup_tier2_decide,
            commands::cleanup_resume_after_crash,
            commands::cleanup_tiers_get,
            commands::cleanup_tiers_set,
            commands::cleanup_alerts,
            commands::recovery_summary_get,
            commands::recovery_summary_dismiss,
            commands::account_set_cleanup,
            commands::account_set_renewal,
            commands::desktop_launch,
            commands::desktop_focus,
            commands::app_setting_get,
            commands::app_setting_set,
            commands::go_to_hotkey_set,
            commands::open_accessibility_settings,
            commands::preview_start,
            commands::preview_stop,
            commands::account_set_preview_opt_in,
        ])
        .setup(|app| {
            let _ = app.emit(EVENT_APP_READY, commands::AppInfo::current());
            register_go_to_hotkey(app.handle());
            spawn_read_model_ticks(
                app.handle().clone(),
                resource_tick_state,
                quota_tick_state,
                attention_state,
                heuristic_tick_state,
                processes_tick_state,
            );
            // AJ-E607: the ingest pipeline — passive, single-writer,
            // 1 s spool cadence (event→engine ≤2 s budget).
            usage_pipeline::spawn(pipeline_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("ai-juggler: fatal: {error}");
            std::process::exit(1);
        });
}

/// Resolves and creates the data dir, exiting with a plain-language
/// message on failure (the shell entry point's first duty).
fn bootstrap_data_dir() -> std::path::PathBuf {
    let data_dir = state::resolve_data_dir().unwrap_or_else(|message| {
        eprintln!("ai-juggler: cannot resolve data dir: {message}");
        std::process::exit(1);
    });
    if let Err(message) = std::fs::create_dir_all(&data_dir) {
        eprintln!(
            "ai-juggler: cannot create {}: {message}",
            data_dir.display()
        );
        std::process::exit(1);
    }
    data_dir
}

/// Registers the stored (or default) "Go to instance…" hotkey
/// (AJ-E414 Req.4). A conflict or a locked-down system key logs and
/// continues — the palette stays reachable in-app (⌘K).
fn register_go_to_hotkey(app: &tauri::AppHandle) {
    use tauri::Manager;
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let hotkey = app
        .state::<std::sync::Arc<state::AppState>>()
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(GO_TO_HOTKEY_KEY).ok())
        .flatten()
        .unwrap_or_else(|| DEFAULT_GO_TO_HOTKEY.to_string());
    if let Err(error) = app.global_shortcut().register(hotkey.as_str()) {
        eprintln!("ai-juggler: hotkey {hotkey} not registered: {error}");
    }
}

/// Read-model ticks (AJ-E401 Req.4): `session://updated` stays event-driven
/// (<300 ms via the lifecycle forwarder); these two add the 5 s resource
/// and 30 s quota refreshes as separate, coalesced emissions. Each tick
/// emits at most one payload and nothing at all while no Instance is
/// active, so an idle app costs the UI zero wakeups.
fn spawn_read_model_ticks(
    app: tauri::AppHandle,
    resource_state: Arc<state::AppState>,
    quota_state: Arc<state::AppState>,
    attention_state: Arc<state::AppState>,
    heuristic_state: Arc<state::AppState>,
    processes_state: Arc<state::AppState>,
) {
    // Event-driven recovery (AJ-E405 Req.5): drain fresh-output signals
    // at 200 ms — well inside the 300 ms budget. The drain also feeds
    // running-model detection for the freshly-active Instances (AJ-E412
    // Req.2: transcript-tail parse, emitted within the same budget).
    {
        let attention_app = app.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = commands::attention_recovery_drain(&attention_app, &attention_state);
            }
        });
    }
    // The 15 s heuristic cycle (SPEC §5.1): debounced Stuck/WaitingUser
    // evaluation; quota exits classify immediately at the forwarder.
    {
        let heuristic_app = app.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(15));
                let _ = commands::heuristic_tick(&heuristic_app, &heuristic_state);
            }
        });
    }
    let quota_app = app.clone();
    let processes_app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = commands::emit_resources(&app, &resource_state);
        }
    });
    // The 2 s per-tree sampling tick (AJ-E410): collector rings/rows,
    // orphan grace poll, and the opted-in Tier-1 auto-cleanup sweep.
    // Also the running-model safety net (AJ-E412): offset-tracked tail
    // scans skip unchanged transcripts with one metadata read.
    // UI emissions stay on their own coalesced cadences above.
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            commands::processes_tick(&processes_state);
            commands::detect_running_models(&processes_app, &processes_state, None);
        }
    });
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let _ = commands::emit_quota(&quota_app, &quota_state);
        }
    });
}

/// The lifecycle forwarder: supervisor events → storage status
/// transitions, with AJ-E405 exit classification (rate-limit signature →
/// [`SessionStatus::QuotaBlocked`]) applied before the status mapping runs.
fn spawn_lifecycle_forwarder(state: Arc<state::AppState>) {
    std::thread::spawn(move || {
        while let Some(event) = state
            .supervisor
            .recv_timeout(std::time::Duration::from_millis(500))
        {
            let stopped_by_user = state.supervisor.was_stopped(event.session());
            // AJ-E405: classify non-zero exits against the captured
            // output tail BEFORE the status mapping runs — a rate-limit
            // signature is QuotaBlocked (immediate), not a crash.
            let quota_session = event.session().clone();
            let quota_blocked = match &event {
                juggler_supervisor::LifecycleEvent::Exited { kind, session, .. } => {
                    if stopped_by_user || kind.is_success() {
                        None
                    } else if let juggler_supervisor::ExitKind::Code(code) = kind {
                        let tail = state
                            .supervisor
                            .output_tail(session, 4096)
                            .unwrap_or_default();
                        state
                            .heuristics
                            .classify_exit(*code, &tail, now_millis(), None)
                    } else {
                        None
                    }
                }
                juggler_supervisor::LifecycleEvent::Started { .. }
                | juggler_supervisor::LifecycleEvent::HelperOrphaned { .. } => None,
            };
            if let juggler_supervisor::LifecycleEvent::Exited { kind, session, pid } = &event {
                // AJ-E410: freeze the pre-exit descendant rows into the
                // orphan tracker + journal the durable allow-list BEFORE
                // the status mapping consumes the event.
                commands::record_exit_snapshot(&state, session, *pid, kind);
            }
            if let Ok(storage) = state.storage.lock() {
                drop(commands::apply_lifecycle_event(
                    &storage,
                    event,
                    stopped_by_user,
                    quota_blocked.as_ref().map(|transition| &transition.reason),
                ));
                // A quota exit ends the process — forget debounce.
                if quota_blocked.is_some() {
                    state.heuristics.forget(&quota_session);
                }
            }
        }
    });
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
        .unwrap_or_default()
}
