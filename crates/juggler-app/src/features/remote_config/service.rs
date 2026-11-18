//! ADR-0035 §7, D-129: the public channel file, fetched at most daily, off
//! the main thread and after the first paint, and applied live.
//!
//! - **The sections** are registered in one place ([`registry`]): today
//!   the adapters' `compat` section (D-128).
//! - **The trust state** lives in the app's durable data folder
//!   (Application Support), `remote-config/trust-state.json`, through
//!   [`FileStore`]: never in a cache, so clearing the cache never touches
//!   it.
//! - **On by default** (D-129): `remote_config_check` set to `off` turns
//!   the check off (the compiled defaults then apply).
//! - **With no pinned key** (today, until the maintainer makes them)
//!   nothing could verify, so nothing is fetched at all: the app sends no
//!   request it can't use, and runs on its compiled defaults.
//! - **Applying:** after each pass, the merged values reach the adapters
//!   as a compat [`Gate`].

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::compat::{self, Gate};
use juggler_remote_config::{
    AppContext, Channel, Endpoints, Facts, FetchPolicy, Fetcher, FileStore, InstallId, KeyRing, Os,
    Registry, UserChoices, Version, merge,
};

use super::RemoteConfigTransport;
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
    Ok(registry)
}

/// Starts the background pass (see the module docs).
pub fn spawn(state: Arc<AppState>) {
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
    if keys.keys().is_empty() {
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
        enabled: setting(&state, CHECK_KEY).as_deref() != Some("off"),
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
            pass(&state, &fetcher, &registry);
        },
    );
}

/// One pass: refresh if due, then apply what's in effect.
fn pass(state: &Arc<AppState>, fetcher: &Fetcher, registry: &Registry) {
    let Ok(app_version) = env!("CARGO_PKG_VERSION").parse::<Version>() else {
        return;
    };
    let channel = channel(crate::updates::channel_of(state));
    let refresh = fetcher.refresh(&AppContext {
        channel,
        app_version: &app_version,
        now: crate::commands::now_millis().max(0).unsigned_abs() / 1000,
    });
    tracing::debug!(attempt = ?refresh.attempt, "remote config: pass");
    let facts = Facts {
        app_version,
        channel,
        os: Os::this(),
        os_version: None,
        tools: tool_versions(state),
        install: install_id(state),
    };
    let public = refresh
        .file
        .map(|file| registry.resolve(&file.envelope.sections, &facts).0);
    let effective = merge(registry, public.as_ref(), None, &UserChoices::new());
    compat::install(Gate::from_effective(&effective));
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
        assert_eq!(
            committed, exported,
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
