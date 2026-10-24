//! AJ-E1611: whether a tool has a newer version than the one on this Mac,
//! and whether AI Juggler has been checked with it.
//!
//! - **Once a day per tool, only tools you use.** For each tool that has
//!   an account here and a public list of its versions (its spec's
//!   [`juggler_adapter::ReleaseFeed`]), one request a day asks for the
//!   newest version: npm's public registry, the package's `latest`. No
//!   request carries anything of the user's. With the setting off
//!   ([`CHECK_KEY`]), none is made; the first pass waits until well after
//!   start (nothing here is needed for the first window).
//! - **Never an update.** Nothing here starts a process or changes a
//!   tool: the result is a calm line on the tool's page with the command
//!   the user runs themselves ([`juggler_adapter::ToolSpec::update_command`]).
//!   `no_update_is_ever_run` (tests) checks the module's own code.
//! - What was found is kept in AI Juggler's settings: the newest version
//!   and when it was asked, per tool.

pub mod commands;
pub mod dto;

pub use dto::ToolUpdateInfo;

use std::collections::BTreeMap;
use std::sync::Arc;

use juggler_account::{HttpRequest, Transport};
use juggler_adapter::ReleaseFeed;

use crate::state::AppState;

/// The setting: `off` stops every check; anything else (or nothing) is on.
pub const CHECK_KEY: &str = "tool_update_check";
/// What the checks found, per tool (JSON).
const STATE_KEY: &str = "tool_update_state";
/// How long a check's answer stands: a day.
pub const EVERY_MS: i64 = 24 * 60 * 60 * 1000;
/// npm's public registry.
pub const NPM_REGISTRY: &str = "https://registry.npmjs.org";
/// How long after start the first pass waits.
const FIRST_PASS_AFTER: std::time::Duration = std::time::Duration::from_mins(10);
/// How often the background pass looks for a check that's due.
const PASS_EVERY: std::time::Duration = std::time::Duration::from_hours(1);

/// What one check found.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Found {
    /// The newest version, when the answer named one.
    pub latest: Option<String>,
    /// When it was asked, epoch ms.
    pub checked_ms: i64,
}

/// The address that lists `feed`'s newest version.
#[must_use]
pub fn feed_url(feed: &ReleaseFeed) -> String {
    match feed {
        // A scoped name's `/` is written `%2F` (npm's registry API).
        ReleaseFeed::Npm { package } => {
            format!("{NPM_REGISTRY}/{}/latest", package.replace('/', "%2F"))
        }
    }
}

/// The version an npm `latest` answer names: its `version`, only when it
/// reads as a version (digits, dots, letters, `-` and `+`, short).
#[must_use]
pub fn latest_of(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let version = value.get("version")?.as_str()?;
    (version.len() <= 64
        && version.starts_with(|c: char| c.is_ascii_digit())
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')))
    .then(|| version.to_owned())
}

/// A version's numbers, and whether it is a pre-release (`-beta.1`).
fn parts(version: &str) -> (Vec<u64>, bool) {
    let version = version.strip_prefix('v').unwrap_or(version);
    let (core, pre) = match version.split_once('-') {
        Some((core, _)) => (core, true),
        None => (version.split('+').next().unwrap_or(version), false),
    };
    let numbers = core
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect();
    (numbers, pre)
}

/// Whether `latest` is newer than `installed` (by their numbers; a
/// pre-release comes before its release).
#[must_use]
pub fn newer(latest: &str, installed: &str) -> bool {
    let (mut a, a_pre) = parts(latest);
    let (mut b, b_pre) = parts(installed);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    match a.cmp(&b) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => b_pre && !a_pre,
    }
}

/// Whether `version` is one AI Juggler was checked with: listed exactly,
/// or matched by a listed `2.1.x`.
#[must_use]
pub fn verified(version: &str, verified_against: &[&str]) -> bool {
    verified_against.iter().any(|listed| {
        *listed == version
            || listed.strip_suffix(".x").is_some_and(|prefix| {
                version
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('.'))
            })
    })
}

/// Whether checks are on (the default).
#[must_use]
pub fn is_on(state: &AppState) -> bool {
    state.storage.lock().is_ok_and(|storage| {
        storage
            .app_settings()
            .get(CHECK_KEY)
            .ok()
            .flatten()
            .as_deref()
            != Some("off")
    })
}

/// What the checks have found so far.
fn load(state: &AppState) -> BTreeMap<String, Found> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(STATE_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(state: &AppState, found: &BTreeMap<String, Found>) {
    if let (Ok(storage), Ok(raw)) = (state.storage.lock(), serde_json::to_string(found)) {
        let _ = storage.app_settings().set(STATE_KEY, &raw);
    }
}

/// The tools to ask about: each one the user has an account on, with a
/// public list of its versions.
fn tools_in_use(state: &AppState) -> Vec<(String, ReleaseFeed)> {
    let Ok(accounts) = state
        .storage
        .lock()
        .map_err(|_| ())
        .and_then(|storage| storage.accounts().list().map_err(|_| ()))
    else {
        return Vec::new();
    };
    let mut tools: Vec<(String, ReleaseFeed)> = Vec::new();
    for account in accounts {
        let id = account.harness.to_string();
        if tools.iter().any(|(known, _)| *known == id) {
            continue;
        }
        if let Some(feed) = state
            .registry
            .get(&account.harness)
            .and_then(|adapter| adapter.spec().release_feed)
        {
            tools.push((id, feed));
        }
    }
    tools
}

/// One pass: for each tool in use whose last check is a day old or more,
/// one request; what it found is kept. Returns how many requests were
/// sent. Nothing when checks are off.
pub fn check_due(state: &Arc<AppState>, transport: &dyn Transport, now_ms: i64) -> usize {
    if !is_on(state) {
        return 0;
    }
    let mut found = load(state);
    let mut sent = 0;
    for (tool, feed) in tools_in_use(state) {
        let due = found
            .get(&tool)
            .is_none_or(|last| now_ms.saturating_sub(last.checked_ms) >= EVERY_MS);
        if !due {
            continue;
        }
        let request = HttpRequest {
            method: "GET",
            url: feed_url(&feed),
            bearer: None,
            content_type: "application/json",
            headers: Vec::new(),
            body: Vec::new(),
            long_transfer: false,
        };
        sent += 1;
        // A failed or odd answer is asked again tomorrow, keeping what was
        // known.
        let latest = transport
            .send(&request)
            .ok()
            .filter(|response| response.status == 200)
            .and_then(|response| latest_of(&response.body));
        let entry = found.entry(tool).or_default();
        entry.checked_ms = now_ms;
        if latest.is_some() {
            entry.latest = latest;
        }
    }
    if sent > 0 {
        save(state, &found);
    }
    sent
}

/// The background pass: well after start, then hourly, checks what's due.
pub fn spawn(state: Arc<AppState>) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_PASS_AFTER);
        let transport = juggler_account::UreqTransport::new(env!("CARGO_PKG_VERSION"));
        loop {
            check_due(&state, &transport, crate::commands::now_millis());
            std::thread::sleep(PASS_EVERY);
        }
    });
}

/// What the tool's page says about `tool`'s versions: the one installed
/// (its own `--version`), the newest the last check found, whether that
/// is newer and checked with AI Juggler, and how to update.
#[must_use]
pub fn status_impl(state: &Arc<AppState>, tool: &str) -> Option<ToolUpdateInfo> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(tool))?;
    let spec = adapter.spec();
    let found = load(state).remove(tool).unwrap_or_default();
    let installed = match adapter.discover().install {
        juggler_adapter::InstallStatus::Installed { version } => Some(version),
        _ => None,
    };
    let latest = found.latest;
    let is_newer = match (&latest, &installed) {
        (Some(latest), Some(installed)) => newer(latest, installed),
        _ => false,
    };
    Some(ToolUpdateInfo {
        tool: tool.to_owned(),
        installed,
        verified: latest
            .as_deref()
            .is_some_and(|latest| verified(latest, adapter.verified_against())),
        latest,
        newer: is_newer,
        checked_ms: (found.checked_ms > 0).then_some(found.checked_ms),
        update_command: spec.update_command,
        checks_on: is_on(state),
        has_feed: spec.release_feed.is_some(),
    })
}

/// Turns the checks on or off.
///
/// # Errors
///
/// Storage errors.
pub fn set_on(state: &AppState, on: bool) -> crate::error::AppResult<()> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    storage
        .app_settings()
        .set(CHECK_KEY, if on { "on" } else { "off" })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(newer("2.1.300", "2.1.296"));
        assert!(newer("0.164.0", "0.163.0"));
        assert!(newer("2.0.0", "1.19.3"));
        assert!(!newer("1.19.3", "2.1.2"), "a newer line installed");
        assert!(!newer("2.1.296", "2.1.296"));
        assert!(newer("2.1.296", "2.1.296-beta.1"));
        assert!(!newer("2.1.297-beta.1", "2.1.297"));
        assert!(newer("v2.4.0", "2.3.0"));
    }

    #[test]
    fn a_version_is_checked_when_listed_or_matched() {
        assert!(verified("2.1.296", &["2.1.296"]));
        assert!(verified("2.1.300", &["2.0.x", "2.1.x"]));
        assert!(!verified("2.10.0", &["2.1.x"]));
        assert!(!verified("1.19.3", &[]));
    }

    #[test]
    fn an_answer_is_read_only_for_a_version() {
        assert_eq!(
            latest_of(br#"{"name":"x","version":"2.3.0"}"#).as_deref(),
            Some("2.3.0")
        );
        assert_eq!(latest_of(br#"{"version":"<script>"}"#), None);
        assert_eq!(latest_of(b"not json"), None);
        assert_eq!(
            feed_url(&ReleaseFeed::Npm {
                package: "@openai/codex".into()
            }),
            "https://registry.npmjs.org/@openai%2Fcodex/latest"
        );
    }

    /// The guardrail: this module never starts a process, so it can't
    /// update a tool (or anything else).
    #[test]
    fn no_update_is_ever_run() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/features/tool_updates");
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap();
            let code = text.split("#[cfg(test)]").next().unwrap_or_default();
            for spawn in [
                "Command::new",
                "user_path::command",
                ".spawn(",
                "launch_spec",
                "supervisor",
            ] {
                assert!(
                    !code.contains(spawn),
                    "{} names {spawn}",
                    entry.path().display()
                );
            }
        }
    }
}
