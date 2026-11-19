//! ADR-0035 §7, D-129: the public channel file, fetched at most daily, off
//! the main thread and after the first paint, and applied live.
//!
//! - **The sections** are registered in one place ([`registry`]), each
//!   declared by the module that owns it: the adapters' `compat` (D-128),
//!   the features' `flags` and the `notices` (AJ-E2023), and the AI
//!   `models` (AJ-E3810).
//! - **The trust state** lives in the app's durable data folder
//!   (Application Support), `remote-config/trust-state.json`, through
//!   [`FileStore`]: never in a cache, so clearing the cache never touches
//!   it.
//! - **On by default** (D-129): `remote_config_check` set to `off` turns
//!   the check off (the compiled defaults then apply).
//! - **With no pinned key** (today, until the maintainer makes them)
//!   nothing could verify, so nothing is fetched at all: the app sends no
//!   request it can't use, and runs on its compiled defaults.
//! - **Applying:** at start the compiled values are put in effect; after
//!   each pass the merged values reach the adapters as a compat [`Gate`],
//!   and the [`Snapshot`] is installed; if it changed, the window is told.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::compat::{self, Gate};
use juggler_remote_config::{
    AppContext, Channel, Endpoints, Facts, FetchPolicy, Fetcher, FileStore, InstallId, KeyRing, Os,
    Registry, Verified, Version,
};

use super::RemoteConfigTransport;
use super::snapshot::{self, Snapshot};
use crate::events::{AppEvent, EmitEvent as _};
use crate::features::{ai, flags, notices};
use crate::state::AppState;

/// The trust state's file under the data folder.
pub const STATE_FILE: &str = "remote-config/trust-state.json";

/// Settings › Privacy "Check for configuration updates": `off` turns it off.
pub const CHECK_KEY: &str = "remote_config_check";

/// The random per-install id rollouts are bucketed by (never sent).
const INSTALL_ID_KEY: &str = "remote_config_install_id";

/// How often a pass looks whether the daily fetch is due (the fetcher
/// itself keeps to once a day, failed attempts included).
const PASS_EVERY: Duration = Duration::from_hours(1);

/// How long the first pass waits for the first paint at most.
const FIRST_PAINT_AT_MOST: Duration = Duration::from_secs(10);

/// Every section the app reads, registered in one place.
///
/// # Errors
///
/// A section that can't be registered (a bad compiled declaration).
pub fn registry(adapters: &juggler_adapter::AdapterRegistry) -> Result<Registry, String> {
    let mut registry = Registry::new();
    let compat = compat::section(&adapters.compat_features()).map_err(|e| e.to_string())?;
    registry.register(compat).map_err(|e| e.to_string())?;
    registry
        .register(flags::section(flags::DECLARED)?)
        .map_err(|e| e.to_string())?;
    registry
        .register(notices::section())
        .map_err(|e| e.to_string())?;
    registry
        .register(ai::models::section())
        .map_err(|e| e.to_string())?;
    Ok(registry)
}

/// What every snapshot of this run shares.
#[derive(Clone, Copy)]
struct Run {
    checking: bool,
    keys_pinned: bool,
}

/// Starts the background pass (see the module docs).
pub fn spawn(app: tauri::AppHandle, state: Arc<AppState>) {
    let registry = match registry(&state.registry) {
        Ok(registry) => registry,
        Err(e) => {
            tracing::warn!(error = %e, "remote config: sections not registered; compiled defaults");
            return;
        }
    };
    let keys = KeyRing::compiled().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "remote config: compiled keys unusable");
        KeyRing::default()
    });
    let run = Run {
        checking: setting(&state, CHECK_KEY).as_deref() != Some("off"),
        keys_pinned: !keys.keys().is_empty(),
    };
    // The compiled values, in effect from the start (no I/O beyond the
    // settings already read).
    snapshot::install(Snapshot {
        checking: run.checking,
        keys_pinned: run.keys_pinned,
        channel: Some(channel(crate::updates::channel_of(&state))),
        ..Snapshot::build(&registry, None, &facts(&state, app_version()))
    });
    if !run.keys_pinned {
        tracing::info!("remote config: no pinned key yet; nothing fetched, compiled defaults");
        return;
    }
    let endpoints = Endpoints::compiled().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "remote config: compiled endpoints unusable");
        Endpoints::default()
    });
    let path = state.data_dir.join(STATE_FILE);
    if let Some(folder) = path.parent()
        && let Err(e) = std::fs::create_dir_all(folder)
    {
        tracing::warn!(error = %e, "remote config: no state folder; compiled defaults");
        return;
    }
    let policy = FetchPolicy {
        enabled: run.checking,
        ..FetchPolicy::default()
    };
    let fetcher = Fetcher::new(
        Arc::new(RemoteConfigTransport::production()),
        endpoints,
        keys,
        Arc::new(FileStore::new(path)),
        policy,
    );
    crate::ticks::every_after_paint(
        "remote-config",
        FIRST_PAINT_AT_MOST,
        PASS_EVERY,
        move || {
            if pass(&state, &fetcher, &registry, run) {
                app.emit_event(AppEvent::RemoteConfigChanged, ());
                // AJ-E3802: a flag or the `models` section may change what
                // the AI jobs' query answers.
                if crate::features::ai::service::recheck(&state).is_some() {
                    app.emit_event(AppEvent::AiJobsChanged, ());
                }
                // ADR-0035 §11.4: installed models' copies follow their
                // entry while it names the same file.
                if let Some(store) = crate::features::ai::models::store::shared_if_made() {
                    let listing = crate::features::ai::models::listing(
                        &crate::features::remote_config::current().lists,
                    );
                    if store.refresh(&listing) {
                        app.emit_event(AppEvent::AiModelsChanged, ());
                    }
                }
            }
        },
    );
}

/// This build's version (a release's version always parses).
fn app_version() -> Version {
    env!("CARGO_PKG_VERSION")
        .parse()
        .unwrap_or_else(|_| Version::new(0, 0, 0))
}

/// What this app knows about itself, for targeting.
fn facts(state: &Arc<AppState>, app_version: Version) -> Facts {
    Facts {
        app_version,
        channel: channel(crate::updates::channel_of(state)),
        os: Os::this(),
        os_version: None,
        tools: tool_versions(state),
        install: install_id(state),
    }
}

/// One pass: refresh if due, then apply what's in effect; whether it
/// changed.
fn pass(state: &Arc<AppState>, fetcher: &Fetcher, registry: &Registry, run: Run) -> bool {
    let facts = facts(state, app_version());
    let refresh = fetcher.refresh(&AppContext {
        channel: facts.channel,
        app_version: &facts.app_version,
        now: crate::commands::now_millis().max(0).unsigned_abs() / 1000,
    });
    tracing::debug!(attempt = ?refresh.attempt, "remote config: pass");
    let snapshot = applied(
        registry,
        refresh.file.as_ref(),
        &facts,
        refresh.checked_at,
        run,
    );
    compat::install(Gate::from_effective(&snapshot.effective));
    snapshot::install(snapshot)
}

/// The snapshot a pass puts in effect.
fn applied(
    registry: &Registry,
    file: Option<&Verified>,
    facts: &Facts,
    checked_at: Option<u64>,
    run: Run,
) -> Snapshot {
    Snapshot {
        checked_at,
        checking: run.checking,
        keys_pinned: run.keys_pinned,
        channel: Some(facts.channel),
        ..Snapshot::build(registry, file, facts)
    }
}

/// The update channel's remote-config twin.
const fn channel(channel: crate::updates::Channel) -> Channel {
    match channel {
        crate::updates::Channel::Stable => Channel::Stable,
        crate::updates::Channel::Beta => Channel::Beta,
        crate::updates::Channel::Nightly => Channel::Nightly,
    }
}

fn setting(state: &Arc<AppState>, key: &str) -> Option<String> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
}

/// The tools' versions as already read (never read here), for targeting.
fn tool_versions(state: &Arc<AppState>) -> BTreeMap<String, Version> {
    state
        .provider_versions
        .lock()
        .map(|versions| {
            versions
                .iter()
                .filter_map(|(tool, text)| {
                    let word = text.split_whitespace().next()?;
                    Some((tool.clone(), word.trim_start_matches('v').parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// This install's random id, made once and kept in the app's settings.
fn install_id(state: &Arc<AppState>) -> InstallId {
    if let Some(id) = setting(state, INSTALL_ID_KEY).as_deref().and_then(from_hex) {
        return InstallId::from_bytes(id);
    }
    let id = InstallId::generate().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "remote config: no random id; bucket 0 this run");
        InstallId::from_bytes([0; 16])
    });
    if let Ok(storage) = state.storage.lock()
        && let Err(e) = storage
            .app_settings()
            .set(INSTALL_ID_KEY, &to_hex(&id.to_bytes()))
    {
        tracing::warn!(error = %e, "remote config: install id not kept");
    }
    id
}

fn to_hex(bytes: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(32), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn from_hex(text: &str) -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    if text.len() != 32 || !text.is_ascii() {
        return None;
    }
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The schema the publisher validates against is the app's own:
    /// `config/remote/schema.json` is this registry's export. To refresh
    /// it: `AIJ_WRITE_REMOTE_SCHEMA=1 cargo test -p juggler-app
    /// remote_config::service`.
    #[test]
    fn the_published_schema_is_the_apps_own() {
        let registry = registry(&crate::state::default_registry()).unwrap();
        let mut exported = serde_json::to_string_pretty(&registry.export()).unwrap();
        exported.push('\n');
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/remote/schema.json");
        if std::env::var_os("AIJ_WRITE_REMOTE_SCHEMA").is_some() {
            std::fs::write(&path, &exported).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        // Compared as JSON, not as text: a workspace build turns on
        // serde_json's `preserve_order` (juggler-stats' tests), which
        // keeps the export's keys in their written order, not sorted.
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&committed).ok(),
            Some(registry.export()),
            "config/remote/schema.json is stale: rerun with AIJ_WRITE_REMOTE_SCHEMA=1"
        );
        // Every channel's source fits it.
        let schema = juggler_remote_config::publish::read_schema(committed.as_bytes()).unwrap();
        for (name, channel) in [
            ("nightly", Channel::Nightly),
            ("beta", Channel::Beta),
            ("stable", Channel::Stable),
        ] {
            let source = std::fs::read(path.with_file_name(format!("{name}.json"))).unwrap();
            let source = juggler_remote_config::publish::read_source(&source).unwrap();
            juggler_remote_config::publish::check_source(&source, &schema, channel).unwrap();
        }
    }

    #[test]
    fn the_install_id_round_trips() {
        let bytes = [0xab; 16];
        assert_eq!(from_hex(&to_hex(&bytes)), Some(bytes));
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex(&"é".repeat(16)), None);
    }
}
