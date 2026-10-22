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
    commands, crash, history, link, machine_health, manage, newer_data, os_access, overnight,
    parked, settings_policy, updates, usage_stats,
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
    "app_info" => value commands::app_info();
    "providers_list" => value commands::harnesses_list();
    "providers_capabilities" => result commands::providers_capabilities(state);
    "accounts_list" => result commands::accounts_list(state);
    "account_audit" => result manage::account_audit(state; account_id: String);
    "account_storage" => result history::account_storage(state);
    "launch_recipes_list" => result commands::profiles_list(state);
    "instances_list" => result commands::sessions_list(state);
    "instance_quality" => result commands::instance_quality(state; session_id: String);
    "instance_processes" => result commands::instance_processes(state; session_id: String);
    "instance_replay" => result history::instance_replay(state; session_id: String);
    "transcript_search" => result history::transcript_search(state; query: String);
    "projects_list" => result commands::projects_list(state);
    "ranking_rows" => result commands::ranking_rows(state; project: Option<String>);
    "switch_back_suggestions" => result commands::switch_back_suggestions(state);
    "parked_list" => value parked::parked_list(state);
    "overnight_summary" => result overnight::overnight_summary(state; since_ms: Option<i64>);
    "dashboard_stats" => result commands::dashboard_stats(state);
    "quota_display" => result commands::quota_display(state; account: String);
    "quota_display_all" => value commands::quota_display_all(state);
    "cost_rollups" => value commands::cost_rollups(state; account: Option<String>);
    "pricing_catalog_info" => value commands::pricing_catalog_info(state);
    "usage_source_health" => value commands::usage_source_health(state);
    "reset_queue_list" => value commands::reset_queue_list(state);
    "this_mac" => async_result machine_health::this_mac(state);
    "machine_health" => async_result machine_health::machine_health(state);
    "machine_advice" => async_result machine_health::machine_advice(state);
    "port_inventory" => result commands::port_inventory(state);
    "cleanup_inventory" => result commands::cleanup_inventory(state; filter: String, machine_scope: Option<String>);
    "cleanup_status" => result commands::cleanup_status(state);
    "cleanup_tiers_get" => result commands::cleanup_tiers_get(state; account_id: String);
    "recovery_summary_get" => result commands::recovery_summary_get(state);
    "redaction_policy_get" => result commands::redaction_policy_get(state; provider: String);
    "app_setting_get" => result commands::app_setting_get(state; key: String)
        if SETTING_KEYS.contains(&key.as_str());
    "notification_prefs_get" => result settings_policy::notification_prefs_get(state);
    "cleanup_default_get" => result settings_policy::cleanup_default_get(state);
    "os_access_list" => value os_access::os_access_list(state);
    "update_status" => value updates::update_status(state);
    "going_back_options" => result newer_data::going_back_options(state);
    "crash_info" => value crash::crash_info(state);
    "usage_stats_info" => value usage_stats::usage_stats_info(state);
    "link_status" => value link::link_status(state);
}
