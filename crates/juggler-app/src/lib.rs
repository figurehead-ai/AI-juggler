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

pub mod backup;
pub mod cloud_backup;
pub mod commands;
pub mod crash;
pub mod crossmachine;
pub mod edition;
pub mod error;
pub mod figurehead;
pub mod fleet;
pub mod fleet_account;
pub mod focus;
pub mod history;
pub mod identity;
pub mod lifecycle;
pub mod link;
pub mod machine_alerts;
pub mod machine_health;
pub mod newer_data;
pub mod os_access;
pub mod overnight;
pub mod parked;
pub mod preview;
pub mod quit;
pub mod remote;
pub mod running_setups;
pub mod settings_policy;
pub mod state;
pub mod terminal;
pub mod updates;
pub mod usage_pipeline;
pub mod usage_stats;
pub mod vault;
pub mod worktree;

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
/// This Mac (AJ-E1403) on the 5 s resource cadence.
pub const EVENT_THIS_MAC: &str = "machine://this-mac";

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
    // AJ-E1103: cold start runs from here to the end of setup.
    let launched = std::time::Instant::now();
    // D-23: the edition comes from the identifier this build carries, and
    // decides everything that must never be shared between editions.
    let mut context = tauri::generate_context!();
    edition::set_current(edition::Edition::from_identifier(
        &context.config().identifier,
        cfg!(debug_assertions),
    ));
    let data_dir = bootstrap_data_dir();
    // AJ-E1010: this edition's data folder is ours until we quit, taken
    // before the data is opened. A second launch while another AI Juggler
    // of this edition holds it touches nothing — no migration, no
    // recovery — and only hands over to the running one.
    if !newer_data::hold_instance_lock(&data_dir) {
        context.config_mut().app.windows.clear();
        hand_over_to_the_running_instance(context);
    }
    let app_state = Arc::new(match state::AppState::init(data_dir.clone()) {
        Ok(state) => state,
        // AJ-E1010: an older AI Juggler opened data a newer one saved — say
        // so, and offer the copy or backup from before the update (never a
        // silent quit).
        Err(error::AppError::Core(juggler_core::CoreError::SchemaTooNew { found, supported })) => {
            newer_data::at_launch(&data_dir, found, supported)
        }
        Err(error) => {
            eprintln!("ai-juggler: fatal: {error}");
            newer_data::cannot_start(&error.to_string());
            std::process::exit(1);
        }
    });

    // D-20: the chosen update channel, for reports built without state.
    let _ = updates::channel_of(&app_state);
    // AJ-E1107: keep a scrubbed crash report if the user allows it.
    crash::install(&app_state);
    spawn_lifecycle_forwarder(Arc::clone(&app_state));

    let attention_state = Arc::clone(&app_state);
    let heuristic_tick_state = Arc::clone(&app_state);
    let resource_tick_state = Arc::clone(&app_state);
    let processes_tick_state = Arc::clone(&app_state);
    let quota_tick_state = Arc::clone(&app_state);
    let pipeline_state = Arc::clone(&app_state);
    let link_state = Arc::clone(&app_state);
    let figurehead_state = Arc::clone(&app_state);
    let update_state = Arc::clone(&app_state);
    let cloud_backup_state = Arc::clone(&app_state);
    let stats_state = Arc::clone(&app_state);

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
        // AJ-E1010: registering the updater makes no request. The feed is
        // only built in release builds that carry the signing key, and
        // the webview gets no updater permission.
        .plugin(tauri_plugin_updater::Builder::new().build())
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
        // AJ-E1503: ⌘Q and the close button ask first while sessions
        // are live (the predefined Quit item would quit without asking).
        .menu(quit::app_menu)
        .on_menu_event(|app, event| {
            if event.id() == quit::QUIT_MENU_ID {
                quit::request(app);
            }
        })
        .on_window_event(|window, event| {
            use tauri::Manager;
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
                && quit::request(window.app_handle())
            {
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            quit::app_quit_plan,
            quit::app_quit,
            usage_stats::usage_stats_info,
            usage_stats::usage_stats_set,
            usage_stats::usage_stats_ui_event,
            usage_stats::usage_stats_ui_gauge,
            usage_stats::usage_stats_overview,
            usage_stats::usage_stats_report,
            usage_stats::usage_stats_delete_local,
            usage_stats::usage_stats_open_docs,
            usage_stats::usage_stats_set_sync,
            crash::crash_info,
            crash::crash_set,
            crash::crash_answer,
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
            commands::account_use_original,
            commands::source_inspect,
            commands::isolation_check,
            fleet::fleet_self,
            fleet::fleet_peers,
            fleet::fleet_unpair,
            fleet::fleet_set_label,
            fleet::fleet_reset_trust,
            fleet::fleet_reset_identity,
            link::link_status,
            link::link_set_enabled,
            link::fleet_discovered,
            link::pairing_host_open,
            link::pairing_host_state,
            link::pairing_host_decide,
            link::pairing_host_close,
            link::pairing_re_enable,
            link::pairing_join_start,
            link::pairing_join_state,
            link::pairing_join_decide,
            link::pairing_join_reset,
            figurehead::figurehead_status,
            figurehead::figurehead_link_start,
            figurehead::figurehead_link_poll,
            figurehead::figurehead_link_cancel,
            figurehead::figurehead_claim,
            figurehead::figurehead_refresh,
            figurehead::figurehead_rename,
            figurehead::figurehead_unlink,
            figurehead::figurehead_dismiss_notice,
            figurehead::figurehead_save_choices,
            figurehead::figurehead_open_page,
            remote::fleet_remote,
            link::fleet_prompts,
            link::fleet_prompt_decide,
            fleet::fleet_set_permission,
            remote::fleet_remote_stop,
            remote::fleet_remote_focus,
            remote::fleet_remote_clean,
            overnight::overnight_summary,
            overnight::overnight_summary_seen,
            parked::instance_park,
            lifecycle::instance_mark_reviewed,
            lifecycle::instance_unmark_reviewed,
            fleet_account::fleet_account_macs,
            fleet_account::fleet_account_pin,
            cloud_backup::cloud_backup_status,
            cloud_backup::cloud_backup_set_schedule,
            cloud_backup::cloud_backup_now,
            cloud_backup::cloud_backup_list,
            cloud_backup::cloud_backup_prepare_restore,
            vault::vault_status,
            vault::vault_setup,
            vault::vault_confirm_saved,
            vault::vault_save_code,
            vault::vault_enter_code,
            updates::update_status,
            updates::update_set_check_on_launch,
            updates::update_set_channel,
            updates::update_check,
            updates::update_install,
            updates::update_restart,
            newer_data::going_back_options,
            newer_data::going_back_prepare,
            backup::backup_export,
            backup::backup_verify,
            backup::backup_restore,
            history::instance_archive,
            history::instance_unarchive,
            history::instance_delete_preview,
            history::instance_delete,
            history::transcript_search,
            history::instance_replay,
            history::account_storage,
            history::retention_set,
            history::retention_preview,
            history::retention_apply,
            parked::parked_list,
            parked::parked_offers,
            parked::parked_dismiss,
            parked::parked_check_now,
            parked::parked_resume,
            worktree::isolation_offer,
            worktree::worktree_create,
            worktree::worktree_removal_preview,
            worktree::worktree_remove,
            worktree::git_status_refresh,
            identity::identity_check,
            identity::account_forget_identity,
            crossmachine::fleet_switch_targets,
            crossmachine::fleet_switch_execute,
            crossmachine::fleet_switch_cancel,
            crossmachine::switch_back::fleet_switch_back_plan,
            crossmachine::switch_back::fleet_switch_back_execute,
            crossmachine::switch_back::fleet_divergences,
            crossmachine::switch_back::fleet_divergence_keep_theirs,
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
            commands::providers_capabilities,
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
            machine_health::machine_health,
            machine_health::this_mac,
            machine_health::machine_advice,
            machine_health::machine_alerts_take,
            os_access::os_access_list,
            os_access::os_access_request,
            os_access::os_access_open_settings,
            settings_policy::session_prefs_get,
            settings_policy::session_prefs_set,
            settings_policy::notification_prefs_get,
            settings_policy::notification_prefs_set,
            settings_policy::cleanup_default_get,
            settings_policy::cleanup_default_set,
            commands::go_to_hotkey_set,
            commands::open_accessibility_settings,
            commands::preview_start,
            commands::preview_stop,
            commands::account_set_preview_opt_in,
        ])
        .setup(move |app| {
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
            // AJ-E704: Link resumes only if the user left it on.
            link::resume_if_enabled(link_state);
            // AJ-E1202: refresh the Figurehead account only if linked —
            // signed out, nothing runs and nothing is sent.
            figurehead::resume_if_linked(figurehead_state);
            // AJ-E1010: release builds check for an update in the
            // background, if the user left that on; never installs.
            updates::spawn_launch_check(app.handle().clone(), update_state);
            // AJ-E1205: the backup schedule (nothing runs while Cloud
            // backup is off or the recovery code isn't saved).
            cloud_backup::spawn_schedule(cloud_backup_state);
            // AJ-E1102: usage stats start only if the user opted in (and
            // no override says off); off creates nothing.
            usage_stats::start(stats_state, launched.elapsed());
            Ok(())
        })
        .build(context)
        .unwrap_or_else(|error| {
            eprintln!("ai-juggler: fatal: {error}");
            std::process::exit(1);
        })
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                usage_stats::shutdown();
            }
        });
}

/// A second launch while another AI Juggler of this edition holds the
/// data: a bare app (no state, no windows) whose single-instance plugin
/// tells the running one to come forward, then exits. If the running one
/// doesn't answer — it is still starting up, sitting on its launch
/// message, closing, or stuck — this launch has become the plugin's
/// "running instance" itself: it lets go of that at once (so the other one
/// keeps working as the single instance once it is ready), says what to
/// do, and quits without opening the data.
fn hand_over_to_the_running_instance(context: tauri::Context<tauri::Wry>) -> ! {
    let built = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .build(context);
    if let Ok(app) = &built {
        // On macOS the plugin binds its socket a moment later, on its own
        // task: wait for it (up to a second), or releasing it now would
        // miss it and leave this launch answering hand-overs while its
        // message is up.
        #[cfg(target_os = "macos")]
        {
            let socket = std::path::PathBuf::from(format!(
                "/tmp/{}_si.sock",
                app.config().identifier.replace(['.', '-'], "_")
            ));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while !socket.exists() && std::time::Instant::now() < deadline {
                // poll: the plugin's socket appearing (bound on its own
                // task, within moments); a second launch only, 1 s at most.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        tauri_plugin_single_instance::destroy(app);
    }
    drop(built);
    let app = edition::current().app_name();
    let how = if cfg!(target_os = "macos") {
        format!(
            "Switch to it and quit it ({app} › Quit, or ⌘Q). If it doesn't respond, force it to quit: press ⌥⌘Esc, choose {app}, then Force Quit."
        )
    } else if cfg!(windows) {
        format!(
            "Switch to it and quit it. If it doesn't respond, end it in Task Manager (Ctrl+Shift+Esc), under {app}."
        )
    } else {
        format!(
            "Switch to it and quit it. If it doesn't respond, end it from your system monitor ({app})."
        )
    };
    newer_data::cannot_start(&format!(
        "Another {app} is already open with this data and isn't answering. It may be showing a message, or be stuck.\n\n{how}\n\nThen open {app} again."
    ));
    std::process::exit(1);
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
                // poll: AJ-E405 Req.5 — recovery shows within 300 ms; an
                // idle drain finds nothing and returns.
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
    let index_state = quota_state.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = commands::emit_resources(&app, &resource_state);
            let _ = machine_health::emit_this_mac(&app, &resource_state);
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
    // AJ-E1007: the transcript search index, kept up to date in the
    // background (incremental: unchanged transcripts cost one metadata
    // read), so a search never waits for a first build.
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let _ = history::index_transcripts(&index_state);
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
