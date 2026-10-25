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

pub mod app_relaunch;
pub mod backup;
pub mod cloud_backup;
pub mod commands;
pub mod crash;
pub mod crash_native;
#[cfg(any(crash_sender, test))]
pub mod crash_send;
pub mod crossmachine;
pub mod ids;
// The read-only test bridge (docs/qa/DEV-BRIDGE.md): debug builds with
// the `dev-bridge` feature only.
#[cfg(all(debug_assertions, feature = "dev-bridge"))]
pub mod dev_bridge;
pub mod edition;
pub mod env_overlay;
pub mod error;
pub mod features;
pub mod figurehead;
pub mod fleet;
pub mod fleet_account;
pub mod focus;
pub mod handover;
pub mod history;
pub mod identity;
pub mod in_place;
pub mod ipc;
pub mod launch_alert;
pub mod lifecycle;
pub mod link;
pub mod machine_alerts;
pub mod machine_health;
pub mod machine_recommendations;
pub mod machine_tools;
pub mod manage;
pub mod newer_data;
pub mod os_access;
pub mod overnight;
pub mod paging;
pub mod parked;
pub mod preview;
pub mod quit;
pub mod remote;
pub mod running_setups;
pub mod settings_policy;
pub mod state;
pub mod terminal;
pub mod transcript_text;
pub mod updates;
pub mod usage_pipeline;
pub mod usage_stats;
pub mod vault;
pub mod worktree;

// Never in a release build: with the feature on, it doesn't compile.
#[cfg(all(feature = "dev-bridge", not(debug_assertions)))]
compile_error!("the `dev-bridge` feature is for debug builds only (docs/qa/DEV-BRIDGE.md)");

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

/// Name of the live-preview frame event (AJ-E415): payload
/// `PreviewFrameInfo`.
pub const EVENT_PREVIEW_FRAME: &str = "preview://frame";

/// Name of the live-preview phase event (AJ-E415): payload
/// `PreviewStateInfo`.
pub const EVENT_PREVIEW_STATE: &str = "preview://state";

/// How a command treats the app, the Mac and the user's accounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAccess {
    /// Answers a question. It changes nothing the user or a later answer
    /// would see: nothing written to their files, vendor folders or
    /// AI Juggler's settings, no vendor CLI started, no process
    /// signalled, no network call, no secret returned, no one-time token
    /// issued, no permission prompt.
    ///
    /// On the way, some reads do the bookkeeping the app's own timers do
    /// anyway: the spawn-ledger reconcile, a stale cached `git status`, the
    /// search index's incremental pass, a usage-statistics count. The
    /// login-state probe opens a sign-in file only for its expiry, and
    /// asks the Keychain whether an item exists — never for its secret.
    ///
    /// Only reads may be served without the app's window: by the Dev-only
    /// read-only test bridge (`docs/qa/DEV-BRIDGE.md`).
    Read,
    /// Everything else, including reads that touch a secret (the vault
    /// key), return one (a pairing code), or create something on first
    /// use (the Fleet identity).
    Write,
}

/// Every IPC command, registered once, each with its [`CommandAccess`]:
/// the one table both the invoke handler and the read-only test bridge's
/// allowlist test come from. A new command is classed when it is added;
/// when in doubt it is `Write`.
macro_rules! app_commands {
    ($then:ident) => {
        $then! {
            Read  commands::app_info,
            Read  usage_stats::usage_stats_info,
            Write usage_stats::usage_stats_set,
            Write usage_stats::usage_stats_ui_event,
            Write usage_stats::usage_stats_ui_gauge,
            Write usage_stats::usage_stats_overview,
            Read  usage_stats::usage_stats_report,
            Write usage_stats::usage_stats_delete_local,
            Write usage_stats::usage_stats_open_docs,
            Write usage_stats::usage_stats_set_sync,
            Read  crash::crash_info,
            Write crash::crash_set,
            Write crash::crash_answer,
            // A screen's error to the crash-report path (kept only if allowed).
            Write crash::crash_ui_error,
            Read  commands::usage_source_health,
            Read  commands::cost_rollups,
            Write commands::cost_csv_export,
            Write commands::pricing_override_set,
            Write commands::pricing_override_clear,
            Read  commands::pricing_catalog_info,
            Read  commands::quota_display,
            Read  commands::quota_display_all,
            Read  features::quota::capture::quota_capture_status,
            Read  features::quota::capture::quota_capture_preview,
            Write features::quota::capture::quota_capture_turn_on,
            Write features::quota::capture::quota_capture_turn_off,
            Write features::app::uninstall::app_prepare_uninstall,
            Write commands::account_set_fee,
            Read  commands::ranking_rows,
            Write commands::alerts_tick,
            Write commands::reset_queue_queue,
            Read  commands::reset_queue_list,
            Write commands::reset_queue_dismiss,
            Read  commands::harnesses_list,
            Read  commands::legacy_aliases::harnesses_list_alias,
            Read  commands::accounts_list,
            Write commands::account_create,
            Read  commands::account_login_state,
            Read  commands::account_login_command,
            Read  commands::environment_scan,
            Write commands::account_import,
            Write commands::account_use_original,
            Write manage::account_rename,
            Write manage::account_remove,
            Write manage::account_set_default,
            Write manage::project_set_default_account,
            Read  manage::account_audit,
            Write commands::source_inspect,
            Write commands::isolation_check,
            Write fleet::fleet_self,
            Write fleet::fleet_peers,
            Write fleet::fleet_unpair,
            Write fleet::fleet_set_label,
            Write fleet::fleet_reset_trust,
            Write fleet::fleet_reset_identity,
            Read  link::link_status,
            Write link::link_set_enabled,
            Write link::fleet_discovered,
            Write link::pairing::pairing_host_open,
            Write link::pairing::pairing_host_state,
            Write link::pairing::pairing_host_decide,
            Write link::pairing::pairing_host_close,
            Write link::pairing::pairing_re_enable,
            Write link::pairing::pairing_join_start,
            Read  link::pairing::pairing_join_state,
            Write link::pairing::pairing_join_decide,
            Write link::pairing::pairing_join_reset,
            Write figurehead::figurehead_status,
            Write figurehead::figurehead_link_start,
            Write figurehead::figurehead_link_poll,
            Write figurehead::figurehead_link_cancel,
            Write figurehead::figurehead_claim,
            Write figurehead::figurehead_refresh,
            Write figurehead::figurehead_rename,
            Write figurehead::figurehead_unlink,
            Write figurehead::figurehead_dismiss_notice,
            Write figurehead::figurehead_save_choices,
            Write figurehead::figurehead_open_page,
            Write remote::fleet_remote,
            Read  link::fleet_prompts,
            Write link::fleet_prompt_decide,
            Write fleet::fleet_set_permission,
            Write remote::fleet_remote_stop,
            Write remote::fleet_remote_focus,
            Write remote::fleet_remote_clean,
            Read  overnight::overnight_summary,
            Write overnight::overnight_summary_seen,
            Write parked::instance_park,
            Write lifecycle::instance_mark_reviewed,
            Write fleet_account::fleet_account_macs,
            Write fleet_account::fleet_account_pin,
            Write cloud_backup::cloud_backup_status,
            Write cloud_backup::cloud_backup_set_schedule,
            Write cloud_backup::cloud_backup_now,
            Write cloud_backup::cloud_backup_list,
            Write cloud_backup::cloud_backup_prepare_restore,
            Write vault::vault_status,
            Write vault::vault_setup,
            Write vault::vault_confirm_saved,
            Write vault::vault_save_code,
            Write vault::vault_enter_code,
            Read  updates::update_status,
            Write updates::update_set_check_on_launch,
            Write updates::update_set_channel,
            Write updates::update_check,
            Write updates::update_install,
            Write updates::update_restart,
            Read  newer_data::going_back_options,
            Write newer_data::going_back_prepare,
            Write newer_data::going_back_open_releases,
            Write backup::backup_export,
            Write backup::backup_verify,
            Write backup::backup_restore,
            Write history::instance_archive,
            Write history::instance_unarchive,
            Write history::instance_delete_preview,
            Write history::instance_delete,
            Read  history::transcript_search,
            Read  history::instance_replay,
            Read  history::account_storage,
            Write history::retention_set,
            Write history::retention_preview,
            Write history::retention_apply,
            Read  parked::parked_list,
            Write parked::parked_offers,
            Write parked::parked_dismiss,
            Write parked::parked_check_now,
            Write parked::parked_resume,
            Write worktree::isolation_offer,
            Write worktree::worktree_create,
            Write worktree::worktree_removal_preview,
            Write worktree::worktree_remove,
            Write worktree::git_status_refresh,
            Read  identity::identity_check,
            Write identity::account_forget_identity,
            Write crossmachine::fleet_switch_targets,
            Write crossmachine::fleet_switch_execute,
            Write crossmachine::fleet_switch_cancel,
            Write crossmachine::switch_back::fleet_switch_back_plan,
            Write crossmachine::switch_back::fleet_switch_back_execute,
            Read  crossmachine::switch_back::fleet_divergences,
            Write crossmachine::switch_back::fleet_divergence_keep_theirs,
            Read  commands::funnel_export,
            Write commands::funnel_event,
            Write commands::expert_setup_info,
            Write commands::expert_reveal,
            Write commands::launch_recipe_env_get,
            Write commands::launch_recipe_env_set,
            Write commands::login_open_web,
            Read  commands::profiles_list,
            Read  commands::legacy_aliases::profiles_list_alias,
            Write commands::profile_create,
            Write commands::profile_delete,
            Write commands::launch_session,
            Read  commands::sessions_list,
            Read  commands::legacy_aliases::sessions_list_alias,
            Read  commands::projects_list,
            Write commands::projects_backfill,
            Write commands::session_handover,
            Read  commands::switch_preflight,
            Write commands::switch_door_count,
            Read  features::switch_setup::switch_setup_check,
            Read  features::switch_setup::switch_setup_list,
            Write features::switch_setup::switch_setup_apply,
            Write features::switch_setup::switch_setup_undo,
            Write features::switch_setup::switch_setup_forget,
            Read  features::account_settings::commands::account_settings_check,
            Write features::account_settings::commands::account_settings_apply,
            Write features::trust::commands::trust_open,
            Read  features::budgets::commands::budgets_list,
            Write features::budgets::commands::budgets_save,
            Write features::budgets::commands::budgets_remove,
            Write features::budgets::commands::budgets_seen,
            Read  features::outside_search::commands::outside_search_status,
            Read  features::outside_search::commands::outside_search_estimate,
            Write features::outside_search::commands::outside_search_set,
            Write commands::switch_execute,
            Write commands::switch_undo,
            Read  commands::switch_back_plan,
            Write commands::switch_back_execute,
            Read  commands::switch_back_suggestions,
            Read  commands::redaction_policy_get,
            Write commands::redaction_policy_set_global,
            Write commands::redaction_policy_set_override,
            Read  commands::switch_targets,
            Read  commands::switch_graph_overview,
            Write commands::epoch_resume,
            Write commands::pack_export,
            Write commands::validation_matrix,
            Write commands::validation_dogfood_record,
            Write commands::providers_discover,
            Read  commands::providers_capabilities,
            Write commands::environment_scan_full,
            Write commands::scan_cancel,
            Write commands::recommendation_for,
            Write commands::stop_session,
            Read  commands::dashboard_stats,
            Write commands::terminal_spawn_login,
            Write commands::terminal_attach,
            Write commands::terminal_write,
            Write commands::terminal_resize,
            Write commands::terminal_kill,
            Write commands::terminal_snapshot,
            Write features::terminal::commands::terminal_open_link,
            Write features::terminal::commands::terminal_pop_out,
            Write features::terminal::commands::terminal_pop_in,
            Read  features::terminal::commands::terminal_windows,
            Read  commands::instance_quality,
            Read  commands::instance_processes,
            Read  features::subtasks::commands::subtasks_list,
            Read  commands::cleanup_inventory,
            Read  commands::cleanup_audit_export,
            Read  commands::port_inventory,
            Read  commands::launch_port_warning,
            Write commands::orphan_stop,
            Write commands::orphan_stop_all,
            Write commands::orphan_verify,
            Write commands::kill_verify,
            Write commands::kill_execute,
            Write commands::kill_cancel,
            Read  commands::kill_liveness,
            Read  commands::cleanup_status,
            Write commands::cleanup_pause_set,
            Write commands::cleanup_tier2_decide,
            Write commands::cleanup_resume_after_crash,
            Read  commands::cleanup_tiers_get,
            Write commands::cleanup_tiers_set,
            Write commands::cleanup_alerts,
            Read  commands::recovery_summary_get,
            Write commands::recovery_summary_dismiss,
            Write commands::account_set_cleanup,
            Write commands::account_set_renewal,
            Write commands::desktop_launch,
            Write commands::desktop_focus,
            Read  commands::app_setting_get,
            Write commands::app_setting_set,
            Read  machine_health::machine_health,
            Read  machine_health::this_mac,
            Read  machine_health::machine_advice,
            Write machine_health::machine_alerts_take,
            Read  os_access::os_access_list,
            Write os_access::os_access_request,
            Write os_access::os_access_open_settings,
            // Relaunches the app: never from the read-only bridge.
            Write app_relaunch::app_relaunch,
            Read  settings_policy::notification_prefs_get,
            Write settings_policy::notification_prefs_set,
            Read  settings_policy::cleanup_default_get,
            Write settings_policy::cleanup_default_set,
            Write commands::go_to_hotkey_set,
            Write commands::open_accessibility_settings,
            Write commands::preview_start,
            Write commands::preview_stop,
            Write commands::account_set_preview_opt_in,
            // release/0.1 (AJ-E1503 quit; E1502 manage; E1504 session
            // prefs; E1525 tools; E1529 conversations; E1533 project git).
            Read  quit::app_quit_plan,
            Write quit::app_quit,
            Write quit::app_quit_ack,
            Write quit::app_quit_cancel,
            Read  settings_policy::session_prefs_get,
            Write settings_policy::session_prefs_set,
            Read  features::tools::commands::tools_catalog,
            Write lifecycle::instance_unmark_reviewed,
            Read  manage::editor_choices,
            Write manage::editor_set,
            Write manage::project_rename,
            Write manage::project_set_pinned,
            Write manage::project_forget,
            // Opens Finder or the editor: never from the read-only bridge.
            Write manage::project_open,
            Write manage::project_relocate,
            Write manage::session_rename,
            Write manage::session_set_pinned,
            Read  features::conversations::commands::conversations_list,
            Read  features::conversations::commands::conversations_resume_command,
            // AJ-E2108: reads message text: never over the bridge.
            Write features::conversations::commands::conversations_last_messages,
            Write features::conversations::commands::conversations_go_to,
            // Brings a running app forward (never launches it).
            Write features::conversations::commands::conversations_bring_forward,
            // Launches a session: never from the read-only bridge.
            Write features::conversations::commands::conversations_continue,
            Read  features::subscriptions::commands::subscriptions_list,
            Read  features::sessions::commands::sessions_resume_command,
            Write features::sessions::commands::sessions_restart_choice_set,
            Read  features::app::login_item::app_login_item_get,
            Write features::app::login_item::app_login_item_set,
            // AJ-E1517: reads the logs (scrubbed): never over the bridge.
            Write features::app::diagnostics::app_diagnostics_preview,
            Write features::app::diagnostics::app_diagnostics_export,
            Write features::app::diagnostics::app_report_bug_open,
            // AJ-E1511: reads a folder the user picked; starts a session.
            Write features::packs::packs_open_preview,
            Write features::packs::packs_open_start,
            // AJ-E1515: what a paired Mac offers; starting there; usage by machine.
            Read  features::fleet::fleet_launch_options,
            Write features::fleet::fleet_launch_start,
            Read  features::fleet::fleet_usage_by_machine,
            Read  features::staging::staging_ready,
            Read  features::notifications::prefs::notifications_get,
            Write features::notifications::prefs::notifications_set,
            // Reads `git status` (the 2 s cache, like a session's).
            Read  features::projects::git::projects_git_status,
            Read  features::projects::icon::projects_icon_read,
            Write features::projects::mark::projects_mark_set,
            Read  features::project_sync::commands::project_sync_rules,
            Read  features::project_sync::commands::project_sync_diff,
            Write features::project_sync::commands::project_sync_apply,
            Read  features::project_sync::commands::project_sync_servers,
            Write features::project_sync::commands::project_sync_servers_apply,
        }
    };
}

/// The registry as Tauri's invoke handler.
macro_rules! tauri_handler {
    ($($access:ident $($segment:ident)::+,)*) => {
        tauri::generate_handler![$($($segment)::+),*]
    };
}

/// The registry as data: `(module::function, access)`.
macro_rules! command_table {
    ($($access:ident $($segment:ident)::+,)*) => {
        &[$((stringify!($($segment)::+), CommandAccess::$access)),*]
    };
}

/// Every registered command and its access class (see [`CommandAccess`]).
pub const COMMANDS: &[(&str, CommandAccess)] = app_commands!(command_table);

/// Builds and runs the desktop application.
#[allow(clippy::too_many_lines)] // the command registry is one flat list by design
pub fn run() {
    // AJ-E1509: a launch that only watches for this app's crash.
    features::app::crash::watch_if_asked();
    // A launch that only shows another launch's "can't start" alert.
    launch_alert::show_if_asked();
    // AJ-E1103: cold start runs from here to the end of setup.
    let launched = std::time::Instant::now();
    // The user's PATH from their login shell, on its own thread: opened
    // from Finder, the app inherits one where `node` (and so any
    // npm-installed CLI) isn't found. Probes and launches use it.
    juggler_adapter::user_path::resolve_at_startup();
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
    // 0.0.11 walk (Q8): the hand-over socket is this data folder's; a
    // second launch asks the running app to come forward and waits for
    // its answer.
    let handover = handover::Handover::for_data_dir(&context.config().identifier, &data_dir);
    if !newer_data::hold_instance_lock(&data_dir) {
        match handover.as_ref().map(|h| h.ask(handover::ANSWER_WAIT)) {
            Some(handover::Answer::Answered) => std::process::exit(0),
            // QA batch 9: an AI Juggler that was just force-quit can hold
            // the lock a moment longer: wait for it before calling it open,
            // and keep offering Try again while it really is.
            Some(handover::Answer::NotAnswering) => {
                not_answering(&data_dir, handover.as_ref());
            }
            // One still starting (a quick double-click), an older AI
            // Juggler (no such socket), or one just ended: wait for it.
            // A socket that isn't ours is never asked, only waited past.
            answer @ (Some(handover::Answer::NoneListening | handover::Answer::Untrusted)
            | None) => {
                if answer == Some(handover::Answer::Untrusted) {
                    eprintln!("ai-juggler: the hand-over socket isn't ours; not asking it");
                }
                if wait_for_an_answer(&data_dir, handover.as_ref(), &context.config().identifier)
                    == Waited::OlderApp
                {
                    context.config_mut().app.windows.clear();
                    hand_over_to_the_running_instance(context);
                }
            }
        }
    }
    // Live-preview frames an earlier version left in the temp folder.
    preview::sweep_frames(&std::env::temp_dir());
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
    // AJ-E1516: a crash outside Rust last time, from macOS's own report;
    // and, in a build given the service's address, sending what the user
    // chose to send.
    {
        let state = Arc::clone(&app_state);
        let bundle_id = context.config().identifier.clone();
        std::thread::spawn(move || {
            let Some(reports) = crash_native::system_reports() else {
                return;
            };
            // Again a minute on: after a crash, the watchdog can reopen the
            // app before macOS has written its report.
            for wait in [0, 60] {
                std::thread::sleep(std::time::Duration::from_secs(wait));
                let _ = crash_native::collect(
                    &reports,
                    &crash::reports_dir(&state),
                    &bundle_id,
                    &crash::effective(&state),
                    juggler_core::model::Timestamp::now().millis(),
                );
            }
        });
    }
    #[cfg(crash_sender)]
    crash_send::start(&app_state);
    spawn_lifecycle_forwarder(Arc::clone(&app_state));
    // AJ-E1509: the watchdog that reopens the app after a crash (when on),
    // and the sessions the last crash ended, resumed where that's on.
    {
        let reopen = app_state
            .storage
            .lock()
            .is_ok_and(|storage| settings_policy::always_on(&storage).reopen_after_crash);
        features::app::crash::follow_setting(&app_state, reopen);
        features::app::crash::resume_after_crash(&app_state);
    }
    // AJ-E1513: prepared switches a crash left for sessions now ended.
    features::staging::sweep_at_startup(&app_state);

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

    let serving = handover.clone();
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
        // AJ-E1510: system notifications; the webview gets no permission
        // for them (the backend shows them, and only when asked).
        .plugin(tauri_plugin_notification::init())
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
                let _ = quit::request(app);
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
            // AJ-E1510: no system notification while AI Juggler is in front.
            if let tauri::WindowEvent::Focused(focused) = event
                && window.label() == "main"
            {
                window
                    .state::<Arc<state::AppState>>()
                    .notifier
                    .set_in_front(*focused);
            }
        })
        .invoke_handler(app_commands!(tauri_handler))
        .setup(move |app| {
            let _ = app.emit(EVENT_APP_READY, commands::AppInfo::current());
            // Q8: answer a second launch only once the window really came
            // forward on the main thread (a stuck app stays silent, and the
            // second launch says so).
            let raise_handle = app.handle().clone();
            let served = serving.as_ref().map(|h| {
                h.serve(move || {
                    let (done, came) = std::sync::mpsc::channel();
                    let window_handle = raise_handle.clone();
                    let queued = raise_handle.run_on_main_thread(move || {
                        use tauri::Manager;
                        if let Some(window) = window_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        let _ = done.send(());
                    });
                    queued.is_ok() && came.recv_timeout(handover::RAISE_WAIT).is_ok()
                })
            });
            if let Some(Err(error)) = served {
                // Without it a second launch uses the plugin's hand-over.
                eprintln!("ai-juggler: no hand-over socket: {error}");
            }
            register_go_to_hotkey(app.handle());
            features::notifications::connect(
                app.handle(),
                &tauri::Manager::state::<Arc<state::AppState>>(app),
            );
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
            // QA on real data: only with AIJ_DEV_BRIDGE=1, in a debug
            // build with the feature (docs/qa/DEV-BRIDGE.md).
            #[cfg(all(debug_assertions, feature = "dev-bridge"))]
            dev_bridge::start_if_enabled(app.handle());
            Ok(())
        })
        .build(context)
        .unwrap_or_else(|error| {
            eprintln!("ai-juggler: fatal: {error}");
            std::process::exit(1);
        })
        .run(move |app, event| {
            if let tauri::RunEvent::Exit = event {
                // AJ-E1509: quit as asked — the run is clean and the
                // watchdog ends with the app.
                use tauri::Manager;
                match app.try_state::<Arc<state::AppState>>() {
                    Some(state) => features::app::crash::before_exit(&state),
                    None => features::app::crash::end_run(&data_dir),
                }
                if let Some(handover) = &handover {
                    handover.stop();
                }
                #[cfg(all(debug_assertions, feature = "dev-bridge"))]
                dev_bridge::remove_token_file(&data_dir);
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
    // A Tauri app was built and dropped here: the alert shows from a
    // fresh launch (see `launch_alert::in_fresh_process`).
    launch_alert::in_fresh_process(
        &format!("{app} can't start"),
        &not_answering_words(&format!("Then open {app} again.")),
    );
}

/// How long a second launch waits for an AI Juggler that holds the data
/// to answer: one still starting (a quick double-click) opens its socket
/// within moments.
const STARTING_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// How a second launch's wait for the open AI Juggler ended.
#[derive(Debug, PartialEq, Eq)]
enum Waited {
    /// The data is free (it ended): this launch goes on, holding it.
    Free,
    /// An older AI Juggler, which answers only on Tauri's single-instance
    /// socket: hand over through the plugin.
    OlderApp,
}

/// The data is held and nothing answers on its hand-over socket yet. Asks
/// again until [`STARTING_WAIT`]: an AI Juggler still starting answers
/// (this launch then quits), an older one is found on the plugin's socket
/// (macOS), or the data comes free. Past it, the "isn't answering" alert
/// (Try again, Quit): never a launch left running without a window
/// (the coordinator, 2026-10-22).
fn wait_for_an_answer(
    data_dir: &std::path::Path,
    handover: Option<&handover::Handover>,
    identifier: &str,
) -> Waited {
    let deadline = std::time::Instant::now() + STARTING_WAIT;
    loop {
        if newer_data::hold_instance_lock(data_dir) {
            return Waited::Free;
        }
        match handover.map(|h| h.ask(handover::ANSWER_WAIT)) {
            Some(handover::Answer::Answered) => std::process::exit(0),
            Some(handover::Answer::NotAnswering) => {
                not_answering(data_dir, handover);
                return Waited::Free;
            }
            _ => {}
        }
        if older_app_listening(identifier) {
            eprintln!(
                "ai-juggler: an older AI Juggler answers on the single-instance socket; handing over"
            );
            return Waited::OlderApp;
        }
        if std::time::Instant::now() >= deadline {
            not_answering(data_dir, handover);
            return Waited::Free;
        }
        // poll: a starting app's socket has no "ready" event; a second
        // launch only, until the deadline above.
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// An older AI Juggler (before its own hand-over socket) answering on
/// Tauri's single-instance socket. macOS only; elsewhere the plugin's
/// hand-over ends with its own alert.
fn older_app_listening(identifier: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let socket = format!("/tmp/{}_si.sock", identifier.replace(['.', '-'], "_"));
        std::os::unix::net::UnixStream::connect(socket).is_ok()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = identifier;
        true
    }
}

/// The open AI Juggler isn't answering. If it lets go of the data within
/// a few seconds (it was ending), this launch goes on; else it says how to
/// quit it and offers Try again, and Quit quits without opening the data.
fn not_answering(data_dir: &std::path::Path, handover: Option<&handover::Handover>) {
    let mut asked = false;
    loop {
        if newer_data::wait_for_instance_lock(data_dir, newer_data::LOCK_GRACE) {
            // The alert made this process an app before Tauri's own: start
            // again in a fresh one, which finds the data free.
            if asked {
                relaunch();
            }
            return;
        }
        if handover.is_some_and(|h| h.ask(handover::ANSWER_WAIT) == handover::Answer::Answered) {
            std::process::exit(0);
        }
        if !newer_data::try_again_or_quit(&not_answering_words("Then choose Try again.")) {
            std::process::exit(1);
        }
        asked = true;
    }
}

/// Starts this app again, as it was started, and quits this launch.
fn relaunch() -> ! {
    eprintln!("ai-juggler: starting again");
    let started = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .args(std::env::args_os().skip(1))
            .spawn()
    });
    if let Err(error) = started {
        newer_data::cannot_start(&format!("It couldn't start again: {error}"));
        std::process::exit(1);
    }
    std::process::exit(0);
}

/// What to do when the open AI Juggler isn't answering, in words, ending
/// with `then`.
fn not_answering_words(then: &str) -> String {
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
    format!(
        "Another {app} is open with this data and isn't answering. It may be showing a message, or be stuck.\n\n{how}\n\n{then}"
    )
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

/// Registers the "Go to instance…" hotkey the user chose (AJ-E414
/// Req.4). There is no default (AJ-E2110): `⌥Space` belongs to `ChatGPT`,
/// so nothing is registered until the user picks a key in Settings ›
/// Desktop apps. A conflict or a locked-down system key logs and
/// continues — the palette stays reachable in-app (⌘K).
fn register_go_to_hotkey(app: &tauri::AppHandle) {
    use tauri::Manager;
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let Some(hotkey) = app
        .state::<std::sync::Arc<state::AppState>>()
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(GO_TO_HOTKEY_KEY).ok())
        .flatten()
    else {
        return;
    };
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
    let conversations_app = app.clone();
    let index_state = quota_state.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = commands::emit_resources(&app, &resource_state);
            // AJ-E1509: the Mac kept awake (and App Nap off) for the
            // sessions working here, before This Mac says so.
            features::app::awake::tick(&resource_state);
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
            if features::subtasks::tick(&processes_state) {
                let _ = commands::emit_sessions(&processes_app, &processes_state);
            }
        }
    });
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let _ = commands::emit_quota(&quota_app, &quota_state);
            // AJ-E1513: the switch prepared before the limit (never run).
            features::staging::tick(&quota_state, commands::now_millis());
        }
    });
    let live_app = conversations_app.clone();
    // AJ-E1529: every account's conversations and which are live, read
    // from the tools' own files every 15 s (only changed files are read
    // again); the UI hears of it only when something changed.
    {
        let conversations_state = index_state.clone();
        std::thread::spawn(move || {
            loop {
                use tauri::Emitter;
                // QA batch 8: a long first pass tells the UI as it goes.
                let tell = || {
                    let _ = conversations_app.emit(features::conversations::EVENT_CHANGED, ());
                };
                if let Ok(true) = features::conversations::tick_with(
                    &conversations_state,
                    features::conversations::PROGRESS_EVERY,
                    &tell,
                ) {
                    tell();
                }
                std::thread::sleep(std::time::Duration::from_secs(15));
            }
        });
    }
    // AJ-E2101: what is open now, from the tools' own session files and
    // the process table (never an environment): small files every 2 s,
    // known pids every 5 s, the whole table at most once a minute. The UI
    // hears of it only when something changed.
    {
        let live_state = index_state.clone();
        let _ = std::thread::Builder::new()
            .name("live-sessions".to_owned())
            .spawn(move || {
                loop {
                    use tauri::Emitter;
                    if let Ok(true) = features::conversations::live_tick(&live_state) {
                        let _ = live_app.emit(features::conversations::EVENT_CHANGED, ());
                    }
                    // poll: the live view's cadence (scan::TICK, 2 s).
                    std::thread::sleep(features::conversations::scan::TICK);
                }
            });
    }
    // AJ-E1538: the opt-in index of outside sessions' words (idle while off).
    features::outside_search::spawn(index_state.clone());
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
            // AJ-E1509: how it ended, for the restarter (after the status
            // is stored, with storage free again).
            let ending = match &event {
                juggler_supervisor::LifecycleEvent::Exited { kind, session, pid } => {
                    // AJ-E410: freeze the pre-exit descendant rows into the
                    // orphan tracker + journal the durable allow-list BEFORE
                    // the status mapping consumes the event.
                    commands::record_exit_snapshot(&state, session, *pid, kind);
                    let tail = if kind.is_success() {
                        String::new()
                    } else {
                        state
                            .supervisor
                            .output_tail(session, 4096)
                            .unwrap_or_default()
                    };
                    Some(features::sessions::restart::Ending::of(
                        kind,
                        stopped_by_user,
                        quota_blocked.is_some(),
                        &tail,
                    ))
                }
                _ => None,
            };
            let mut now = None;
            if let Ok(storage) = state.storage.lock() {
                now = commands::apply_lifecycle_event(
                    &storage,
                    event,
                    stopped_by_user,
                    quota_blocked.as_ref().map(|transition| &transition.reason),
                );
                // A quota exit ends the process — forget debounce.
                if quota_blocked.is_some() {
                    state.heuristics.forget(&quota_session);
                }
            }
            // AJ-E1510: finished or at its limit, as a notification when
            // asked for.
            if let Some(record) = now {
                features::notifications::session_status(
                    &state,
                    record.id.as_str(),
                    record.status.as_str(),
                );
            }
            if let Some(ending) = ending {
                features::sessions::restart::after_exit(&state, &quota_session, ending);
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
