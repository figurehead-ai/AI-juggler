//! The commands the bridge serves: each one a query that changes
//! nothing — no writes, no processes started, no network, no secrets.
//!
//! An entry names the command's IPC name and the function it calls. The
//! same tokens are the call and the [`ReadRoute::path`] the allowlist
//! test checks against the registry ([`crate::COMMANDS`]), so a route can
//! only call a function classed `Read` there.
//!
//! Entry forms: `value` (returns a value), `result` (returns an
//! `AppResult`), and their `async_` forms. `state` passes the app state;
//! `; name: Type` arguments arrive as the UI sends them (camelCase); an
//! `if` guard on them refuses the call (403) before it is made.

use std::sync::Arc;

use hyper::StatusCode;
use tauri::Manager as _;

use crate::state::AppState;
use crate::{
    commands, crash, features, history, link, machine_health, manage, newer_data, os_access,
    overnight, parked, quit, settings_policy, updates, usage_stats,
};

/// The settings the UI reads; others (the one-time confirmation tokens
/// among them) stay in the app.
const SETTING_KEYS: &[&str] = &[crate::GO_TO_HOTKEY_KEY, "renewal_warn_days"];

/// A served command.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadRoute {
    /// The name the UI invokes.
    pub ipc: &'static str,
    /// The function it calls, as in the registry (`module::function`).
    pub path: &'static str,
}

/// A command's answer: the JSON the UI gets, and its HTTP status.
pub(super) struct Reply {
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl Reply {
    fn value<T: serde::Serialize>(value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => Self {
                status: StatusCode::OK,
                body,
            },
            Err(_) => Self::internal(),
        }
    }

    /// A command's own error, in its wire form.
    fn failed(error: &crate::error::AppError) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: serde_json::to_vec(error).unwrap_or_default(),
        }
    }

    fn bad_arguments(error: &serde_json::Error) -> Self {
        // serde's message names the field, never its value.
        let body = serde_json::json!({
            "code": "bad_arguments",
            "retryable": false,
            "user_message": format!("Bad arguments: {error}"),
        });
        Self {
            status: StatusCode::BAD_REQUEST,
            body: body.to_string().into_bytes(),
        }
    }

    /// A call the bridge doesn't make: the same 403 as a write.
    fn refused() -> Self {
        let body = serde_json::json!({
            "code": "read_only_bridge",
            "retryable": false,
            "user_message": super::server::READ_ONLY_MESSAGE,
        });
        Self {
            status: StatusCode::FORBIDDEN,
            body: body.to_string().into_bytes(),
        }
    }

    pub(super) fn internal() -> Self {
        let body = serde_json::json!({
            "code": "internal",
            "retryable": true,
            "user_message": "The test bridge couldn't answer that.",
        });
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: body.to_string().into_bytes(),
        }
    }
}

macro_rules! reply {
    (value, $out:expr) => {
        Reply::value(&$out)
    };
    (result, $out:expr) => {
        match $out {
            Ok(value) => Reply::value(&value),
            Err(error) => Reply::failed(&error),
        }
    };
}

macro_rules! call {
    // Sync commands run on a blocking thread, as they would off Tauri's
    // main thread.
    ($kind:ident, $app:ident, $args:ident, ($($path:tt)+), ($($state:ident)?), ($($arg:ident : $ty:ty),*), ($($guard:expr)?)) => {{
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Args {
            $($arg: $ty),*
        }
        match serde_json::from_value::<Args>($args) {
            Err(error) => Reply::bad_arguments(&error),
            Ok(Args { $($arg),* }) => {
                if false $(|| !($guard))? {
                    Reply::refused()
                } else {
                    call!(@run $kind, $app, ($($path)+), ($($state)?), ($($arg),*))
                }
            }
        }
    }};
    (@run value, $app:ident, $path:tt, $state:tt, $args:tt) => {
        call!(@blocking value, $app, $path, $state, $args)
    };
    (@run result, $app:ident, $path:tt, $state:tt, $args:tt) => {
        call!(@blocking result, $app, $path, $state, $args)
    };
    (@run async_value, $app:ident, $path:tt, $state:tt, $args:tt) => {
        call!(@async value, $app, $path, $state, $args)
    };
    (@run async_result, $app:ident, $path:tt, $state:tt, $args:tt) => {
        call!(@async result, $app, $path, $state, $args)
    };
    (@blocking $kind:ident, $app:ident, ($($path:tt)+), ($($state:ident)?), ($($arg:ident),*)) => {{
        let app = $app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            $(let $state = app.state::<Arc<AppState>>();)?
            let _ = &app;
            reply!($kind, $($path)+($($state,)? $($arg),*))
        })
        .await
        .unwrap_or_else(|_| Reply::internal())
    }};
    (@async $kind:ident, $app:ident, ($($path:tt)+), ($($state:ident)?), ($($arg:ident),*)) => {{
        $(let $state = $app.state::<Arc<AppState>>();)?
        reply!($kind, $($path)+($($state,)? $($arg),*).await)
    }};
}

macro_rules! read_routes {
    ($(
        $ipc:literal => $kind:ident $($seg:ident)::+ ($($state:ident)? $(; $($arg:ident : $ty:ty),+)?) $(if $guard:expr)?;
    )*) => {
        /// Every command the bridge serves (the allowlist).
        pub const READ_ROUTES: &[ReadRoute] = &[
            $(ReadRoute { ipc: $ipc, path: stringify!($($seg)::+) },)*
        ];

        /// Calls the route named `ipc`; `None` for any other name.
        pub(super) async fn dispatch<R: tauri::Runtime>(
            app: &tauri::AppHandle<R>,
            ipc: &str,
            args: serde_json::Value,
        ) -> Option<Reply> {
            Some(match ipc {
                $($ipc => call!($kind, app, args, ($($seg)::+), ($($state)?), ($($($arg : $ty),+)?), ($($guard)?)),)*
                _ => return None,
            })
        }
    };
}

/// The served route for an IPC name, if the bridge serves it.
pub(super) fn find(ipc: &str) -> Option<&'static ReadRoute> {
    READ_ROUTES.iter().find(|route| route.ipc == ipc)
}

read_routes! {
    "app_info" => async_value commands::app_info();
    "providers_list" => async_value commands::harnesses_list();
    "providers_capabilities" => async_result commands::providers_capabilities(state);
    "accounts_list" => async_result commands::accounts_list(state);
    "subscriptions_list" => async_result features::subscriptions::commands::subscriptions_list(state);
    "service_accounts_catalog" => async_result features::service_accounts::commands::service_accounts_catalog(state);
    "account_audit" => async_result manage::account_audit(state; account_id: String);
    "account_storage" => async_result history::account_storage(state);
    "launch_recipes_list" => async_result commands::profiles_list(state);
    "instances_list" => async_result commands::sessions_list(state);
    "instance_quality" => async_result commands::instance_quality(state; session_id: String);
    "instance_replay" => async_result history::instance_replay(state; session_id: String);
    "sessions_history" => async_result features::sessions::commands::sessions_history(state; session_id: String, page: crate::paging::PageRequest);
    "transcript_search" => async_result history::transcript_search(state; query: String);
    "projects_list" => async_result commands::projects_list(state);
    "ranking_rows" => async_result commands::ranking_rows(state; project: Option<String>);
    "switch_back_suggestions" => async_result commands::switch_back_suggestions(state);
    // The switch screen (AJ-E1543): its targets, its check and its
    // set-up list, all reads. The check's secret previews are masked
    // before they leave ([`masked`]).
    "switch_targets" => async_result commands::switch_targets(state; session_id: String);
    "switch_preflight" => async_result commands::switch_preflight(state; session_id: String, target_profile_id: String);
    "switch_setup_check" => async_result features::switch_setup::switch_setup_check(state; session_id: String, target_profile_id: String);
    "parked_list" => async_result parked::parked_list(state);
    "overnight_summary" => async_result overnight::overnight_summary(state; since_ms: Option<i64>);
    "dashboard_stats" => async_result commands::dashboard_stats(state);
    "quota_display" => async_result commands::quota_display(state; account: String);
    "quota_display_all" => async_result commands::quota_display_all(state);
    "quota_capture_status" => async_result features::quota::capture::quota_capture_status(state; account_id: String);
    "quota_capture_preview" => async_result features::quota::capture::quota_capture_preview(state; account_id: String);
    "quota_probe_status" => async_result features::quota::probe::quota_probe_status(state; account_id: String);
    "cost_rollups" => async_result commands::cost_rollups(state; account: Option<String>);
    "cost_rows_page" => async_result commands::cost_rows_page(state; account: Option<String>, page: crate::paging::PageRequest);
    "pricing_catalog_info" => async_result commands::pricing_catalog_info(state);
    "usage_source_health" => async_result commands::usage_source_health(state);
    "reset_queue_list" => async_result commands::reset_queue_list(state);
    "this_mac" => async_result machine_health::this_mac(state);
    "machine_health" => async_result machine_health::machine_health(state);
    "machine_advice" => async_result machine_health::machine_advice(state);
    "port_inventory" => async_result commands::port_inventory(state);
    "cleanup_status" => async_result commands::cleanup_status(state);
    "cleanup_tiers_get" => async_result commands::cleanup_tiers_get(state; account_id: String);
    "recovery_summary_get" => async_result commands::recovery_summary_get(state);
    "redaction_policy_get" => async_result commands::redaction_policy_get(state; provider: String);
    "app_setting_get" => async_result commands::app_setting_get(state; key: String)
        if SETTING_KEYS.contains(&key.as_str());
    "notification_prefs_get" => async_result settings_policy::notification_prefs_get(state);
    "cleanup_default_get" => async_result settings_policy::cleanup_default_get(state);
    "os_access_list" => async_result os_access::os_access_list(state);
    "update_status" => async_result updates::update_status(state);
    "going_back_options" => async_result newer_data::going_back_options(state);
    "crash_info" => async_result crash::crash_info(state);
    "usage_stats_info" => async_result usage_stats::usage_stats_info(state);
    "link_status" => async_result link::link_status(state);
    // release/0.1's reads (AJ-E1525, E1502, E1504, E1529, E1533).
    "tools_catalog" => async_result features::tools::commands::tools_catalog(state);
    "tool_updates_status" => async_result features::tool_updates::commands::tool_updates_status(state; tool: String);
    "conversations_list" => async_result features::conversations::commands::conversations_list(state);
    "conversations_get" => async_result features::conversations::commands::conversations_get(state; account_id: String, conversation_id: String);
    "conversations_resume_command" => async_result features::conversations::commands::conversations_resume_command(state; account_id: String, conversation_id: String);
    "sessions_resume_command" => async_result features::sessions::commands::sessions_resume_command(state; session_id: String);
    "sessions_page" => async_result features::sessions::commands::sessions_page(state; query: features::sessions::rows::SessionsQuery, page: crate::paging::PageRequest);
    "sessions_counts" => async_result features::sessions::commands::sessions_counts(state; query: features::sessions::rows::SessionsQuery);
    "sessions_get" => async_result features::sessions::commands::sessions_get(state; ids: Vec<String>);
    "projects_git_status" => async_result features::projects::git::projects_git_status(state; project_id: String);
    "projects_icon_read" => async_result features::projects::icon::projects_icon_read(state; project_id: String);
    "app_icons_read" => async_result features::app_icons::app_icons_read(state; app_id: Option<String>, process: Option<u32>, px: u32);
    "editor_choices" => async_result manage::editor_choices(state);
    "session_prefs_get" => async_result settings_policy::session_prefs_get(state);
    "notifications_get" => async_result features::notifications::prefs::notifications_get(state);
    // AJ-E2501: the notification model's summary and lists (reads only).
    "notifications_summary" => async_result features::notifications::commands::notifications_summary(state);
    "notifications_page" => async_result features::notifications::commands::notifications_page(state; section: String, request: crate::paging::PageRequest);
    "notifications_mutes" => async_result features::notifications::commands::notifications_mutes(state; request: crate::paging::PageRequest);
    "notifications_snoozes" => async_result features::notifications::commands::notifications_snoozes(state; request: crate::paging::PageRequest);
    // AJ-E1507: the project's rules and servers, compared (reads only).
    "project_sync_rules" => async_result features::project_sync::commands::project_sync_rules(state; project_id: String, master: Option<String>);
    "project_sync_diff" => async_result features::project_sync::commands::project_sync_diff(state; project_id: String, master: String, file: String, how: String);
    "project_sync_servers" => async_result features::project_sync::commands::project_sync_servers(state; project_id: String);
    // AJ-E1508: what a new account could start with (reads only).
    "account_settings_check" => async_result features::account_settings::commands::account_settings_check(state; target: Option<String>, source: String);
    // The rest of release/0.1's reads (AJ-E1503, E1513, E1514, E1538,
    // E1543, E1546), found missing in the 0.1 acceptance walk.
    "budgets_list" => async_result features::budgets::commands::budgets_list(state; month_start_ms: i64);
    // 0.1.5: the usage history's summaries (reads only).
    "usage_history_summary" => async_result features::usage_history::commands::usage_history_summary(state; input: features::usage_history::UsageQueryInput);
    "usage_history_sessions" => async_result features::usage_history::commands::usage_history_sessions(state; input: features::usage_history::UsageQueryInput, page: crate::paging::PageRequest);
    "usage_history_first" => async_result features::usage_history::commands::usage_history_first(state; profile: Option<String>);
    "usage_history_limits" => async_result features::usage_history::commands::usage_history_limits(state; profile: String, start_ms: i64, end_ms: i64);
    "usage_history_session" => async_result features::usage_history::commands::usage_history_session(state; profile: String, conversation: String);
    "outside_search_status" => async_result features::outside_search::commands::outside_search_status(state);
    "outside_search_estimate" => async_result features::outside_search::commands::outside_search_estimate(state);
    "cache_status" => async_result features::cache::commands::cache_status(state);
    "switch_setup_list" => async_result features::switch_setup::switch_setup_list(state);
    "switch_back_plan" => async_result commands::switch_back_plan(state; session_id: String, to_epoch: Option<String>);
    "switch_graph_overview" => async_result commands::switch_graph_overview(state; project_id: String);
    "staging_ready" => async_result features::staging::staging_ready(state; session_id: String);
    "app_quit_plan" => async_result quit::app_quit_plan(state);
    "fleet_launch_options" => async_result features::fleet::fleet_launch_options(state);
    "fleet_usage_by_machine" => async_result features::fleet::fleet_usage_by_machine(state);
    // AJ-E1702/E1703: the pools, a pool's members a page at a time, and
    // one session's place (reads only).
    "pools_list" => async_result features::pools::commands::pools_list(state);
    "pools_members" => async_result features::pools::commands::pools_members(state; pool_id: String, page: crate::paging::PageRequest);
    "pools_place" => async_result features::pools::commands::pools_place(state; session_id: String);
    // 0.3: a pool's canvas and the Pools screen's cards (reads only).
    "pools_canvas" => async_result features::pools::commands::pools_canvas(state; pool_id: String, session_ids: Vec<String>);
    "pools_canvas_previews" => async_result features::pools::commands::pools_canvas_previews(state; pool_ids: Vec<String>);
    // E18: messages' lists (reads only; opening a thread marks it read).
    "messages_threads" => async_result features::messages::commands::messages_threads(state; pool_id: Option<String>, session_id: Option<String>, cursor: Option<String>);
    "messages_needs" => async_result features::messages::commands::messages_needs(state);
    "messages_docs" => async_result features::messages::commands::messages_docs(state; pool_id: String, cursor: Option<String>);
    "messages_links" => async_result features::messages::commands::messages_links(state; pool_id: String);
    // E17 and E23: the agent tools' settings, and the runtimes' reads.
    "agent_bridge_get" => async_result features::agent_bridge::commands::agent_bridge_get(state);
    "runtime_get" => async_result features::runtime::commands::runtime_get(state; session_id: String);
    "runtime_asks" => async_result features::runtime::commands::runtime_asks(state; offset: Option<u32>, limit: Option<u32>);
    "runtime_trust" => async_result features::runtime::trust::runtime_trust(state; profile_id: String, cwd: Option<String>);
    "runtime_choices" => async_result features::runtime::commands::runtime_choices(state);
    // AJ-E2401/E2405: the Processes view's rows, counts, history and
    // rules (reads only; nothing is stopped or marked).
    "processes_page" => async_result features::processes::commands::processes_page(state; input: features::processes::dto::ProcessQueryInput, page: crate::paging::PageRequest);
    "processes_status" => async_result features::processes::commands::processes_status(state);
    "processes_counts" => async_result features::processes::commands::processes_counts(state);
    "processes_history" => async_result features::processes::commands::processes_history(state; session_id: Option<String>, project_id: Option<String>, page: crate::paging::PageRequest);
    "processes_dry_run" => async_result features::processes::commands::processes_dry_run(state; account_id: String, tier: u8);
    "processes_policy" => async_result features::processes::commands::processes_policy(state);
    // E19: tasks' lists (reads only).
    "tasks_list" => async_result features::tasks::commands::tasks_list(state; pool_id: String, filter: Option<String>, words: Option<String>, page: crate::paging::PageRequest);
    "tasks_counts" => async_result features::tasks::commands::tasks_counts(state; pool_id: String);
    "tasks_across" => async_result features::tasks::commands::tasks_across(state; filter: features::tasks::dto::TasksFilterInput, page: crate::paging::PageRequest);
    "tasks_across_counts" => async_result features::tasks::commands::tasks_across_counts(state; filter: features::tasks::dto::TasksFilterInput);
    "tasks_project_counts" => async_result features::tasks::commands::tasks_project_counts(state; project_ids: Vec<String>, page: crate::paging::PageRequest);
    "tasks_plan" => async_result features::tasks::commands::tasks_plan(state; pool_id: String, page: crate::paging::PageRequest);
    "tasks_children" => async_result features::tasks::commands::tasks_children(state; pool_id: String, parent: String, page: crate::paging::PageRequest);
    "tasks_session" => async_result features::tasks::commands::tasks_session(state; session_id: String, page: crate::paging::PageRequest);
    "tasks_detail" => async_result features::tasks::commands::tasks_detail(state; task_id: String, cursor: Option<String>);
    "tasks_needs" => async_result features::tasks::commands::tasks_needs(state);
}

/// The `Read` commands the bridge doesn't serve, and why. Every other
/// `Read` command is served: a new one is either routed above or named
/// here, so the bridge can't fall behind the app unnoticed (a test
/// checks it).
#[cfg(test)]
pub const NOT_SERVED: &[(&str, &str)] = &[
    (
        "features::messages::commands::messages_doc",
        "a shared document's whole text; the list of documents is served",
    ),
    (
        "features::messages::commands::messages_search",
        "searches agents' message words; the threads are served",
    ),
    (
        "usage_stats::usage_stats_report",
        "a usage-statistics report the user reviews before it's sent",
    ),
    (
        "commands::legacy_aliases::harnesses_list_alias",
        "an old name for providers_list, which is served",
    ),
    (
        "commands::legacy_aliases::profiles_list_alias",
        "an old name for launch_recipes_list, which is served",
    ),
    (
        "commands::legacy_aliases::sessions_list_alias",
        "an old name for instances_list, which is served",
    ),
    (
        "commands::account_login_state",
        "the sign-in probe; QA reads sign-in state through accounts_list",
    ),
    (
        "commands::account_login_command",
        "the command line for signing in; the bridge never signs in",
    ),
    (
        "commands::environment_scan",
        "the setup scan first run makes; QA runs it through its own harness",
    ),
    (
        "link::pairing::pairing_join_state",
        "pairing, a flow the bridge never takes part in",
    ),
    (
        "link::fleet_prompts",
        "requests waiting for this Mac's Allow, answered only in the app",
    ),
    (
        "identity::identity_check",
        "a Launch setting's check, made just before a launch",
    ),
    (
        "crossmachine::switch_back::fleet_divergences",
        "conflicts left after switching back between paired Macs",
    ),
    ("commands::funnel_export", "an export the user saves"),
    ("commands::cleanup_audit_export", "an export the user saves"),
    (
        "features::processes::commands::processes_snapshot",
        "an export the user saves",
    ),
    (
        "features::processes::commands::processes_observed_verify",
        "the check made just before the user stops another app's process; the bridge stops nothing",
    ),
    (
        "commands::launch_port_warning",
        "checked just before a launch",
    ),
    (
        "commands::kill_liveness",
        "polled by the stop sheet while a stop runs; the bridge stops nothing",
    ),
    (
        "features::terminal::commands::terminal_windows",
        "the app's own terminal windows",
    ),
    (
        "features::app::login_item::app_login_item_get",
        "asks macOS about the login item, not the app's data",
    ),
    (
        "features::app::user::app_user_get",
        "the user's name from the system, not the app's data",
    ),
];

/// The fields a reply may not carry out of the app, by command: each is
/// set to a mask in every object that holds it. A secret's masked preview
/// (its first character and last four) still says too much for a walk's
/// record; its kind and count stay.
const MASKED: &[(&str, &[&str])] = &[("switch_preflight", &["preview"])];

/// `reply` with the fields [`MASKED`] names for `ipc` replaced, wherever
/// they are in its JSON. Other replies pass as they are.
pub(super) fn masked(ipc: &str, reply: Reply) -> Reply {
    let Some((_, fields)) = MASKED.iter().find(|(name, _)| *name == ipc) else {
        return reply;
    };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&reply.body) else {
        return Reply::internal();
    };
    mask_fields(&mut value, fields);
    match serde_json::to_vec(&value) {
        Ok(body) => Reply {
            status: reply.status,
            body,
        },
        Err(_) => Reply::internal(),
    }
}

fn mask_fields(value: &mut serde_json::Value, fields: &[&str]) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map.iter_mut() {
                if fields.contains(&key.as_str()) && inner.is_string() {
                    *inner = serde_json::Value::String("•••".to_owned());
                } else {
                    mask_fields(inner, fields);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                mask_fields(item, fields);
            }
        }
        _ => {}
    }
}
