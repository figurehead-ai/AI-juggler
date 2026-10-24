//! AJ-E1611: the once-a-day check for new tool versions. One request per
//! tool a day, only for tools with an account here, none with the check
//! off; what it found is on the tool's page with the command to update it
//! yourself; and every registered tool's list of versions is on npm's
//! public registry. A recording transport stands in for the network.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_account::RecordingTransport;
use juggler_adapter::{AdapterRegistry, ReleaseFeed};
use juggler_adapters::{CODEX_ID, CodexAdapter, OpenCodeAdapter};
use juggler_app::commands;
use juggler_app::features::tool_updates::{self, EVERY_MS};
use juggler_app::state::AppState;

fn stub(dir: &Path, name: &str, version: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\necho '{version}'\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        CodexAdapter::with_bin(stub(dir, "codex", "codex-cli 0.163.0")).with_home(dir.join("home")),
    ));
    registry.register(Arc::new(
        OpenCodeAdapter::with_bin(stub(dir, "opencode", "1.19.3")).with_home(dir.join("home")),
    ));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

const NOW: i64 = 1_792_580_400_000;

#[test]
fn one_request_per_tool_a_day_only_for_tools_in_use_and_none_when_off() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let transport = RecordingTransport::default();
    // No account: nothing is asked.
    assert_eq!(tool_updates::check_due(&state, &transport, NOW), 0);
    commands::create_account(&state, CODEX_ID, "Codex", None).unwrap();
    commands::create_account(&state, CODEX_ID, "Codex 2", None).unwrap();
    transport.respond(200, r#"{"name":"@openai/codex","version":"0.170.0"}"#);
    assert_eq!(
        tool_updates::check_due(&state, &transport, NOW),
        1,
        "one per tool"
    );
    let sent = transport.requests();
    assert_eq!(sent[0].method, "GET");
    assert_eq!(
        sent[0].url,
        "https://registry.npmjs.org/@openai%2Fcodex/latest"
    );
    assert_eq!(sent[0].bearer, None, "nothing of the user's goes with it");
    assert!(sent[0].body.is_empty());
    // Within the day: asked once only.
    assert_eq!(
        tool_updates::check_due(&state, &transport, NOW + EVERY_MS - 1),
        0
    );
    // Off: never.
    tool_updates::set_on(&state, false).unwrap();
    assert_eq!(
        tool_updates::check_due(&state, &transport, NOW + 2 * EVERY_MS),
        0
    );
    assert_eq!(transport.count(), 1);
    // On again, a day later: asked again; a failed answer keeps what was known.
    tool_updates::set_on(&state, true).unwrap();
    transport.respond(503, "busy");
    assert_eq!(
        tool_updates::check_due(&state, &transport, NOW + 2 * EVERY_MS),
        1
    );

    let status = tool_updates::status_impl(&state, CODEX_ID).unwrap();
    assert_eq!(status.installed.as_deref(), Some("0.163.0"));
    assert_eq!(status.latest.as_deref(), Some("0.170.0"));
    assert!(status.newer && !status.verified);
    assert_eq!(
        status.update_command.as_deref(),
        Some("npm install -g @openai/codex@latest")
    );
    assert!(status.checks_on && status.has_feed);
    assert_eq!(status.checked_ms, Some(NOW + 2 * EVERY_MS));
}

#[test]
fn every_registered_tool_lists_its_versions_on_npm_and_says_how_to_update() {
    let registry = juggler_app::state::default_registry();
    let ids = registry.ids();
    assert!(ids.len() >= 4);
    for id in ids {
        let spec = registry.get(&id).unwrap().spec();
        // A tool with no public package list (Grok Build updates itself)
        // is never asked about; every other one is on npm.
        if let Some(ReleaseFeed::Npm { package }) = spec.release_feed.clone() {
            let url = tool_updates::feed_url(spec.release_feed.as_ref().unwrap());
            assert!(
                url.starts_with("https://registry.npmjs.org/") && url.ends_with("/latest"),
                "{url}"
            );
            assert!(!package.is_empty());
        }
        let command = spec.update_command.expect("an update command");
        assert!(!command.is_empty(), "{id}");
    }
    // A fixture answer per tool reads as its version.
    for (package, answer) in [
        (
            "@anthropic-ai/claude-code",
            r#"{"name":"@anthropic-ai/claude-code","version":"2.1.300"}"#,
        ),
        (
            "@openai/codex",
            r#"{"name":"@openai/codex","version":"0.170.0"}"#,
        ),
        (
            "opencode-ai",
            r#"{"name":"opencode-ai","version":"1.19.4"}"#,
        ),
        (
            "@moonshot-ai/kimi-code",
            r#"{"name":"@moonshot-ai/kimi-code","version":"2.3.1"}"#,
        ),
    ] {
        let version = tool_updates::latest_of(answer.as_bytes()).unwrap();
        assert!(
            answer.contains(package) && answer.contains(&version),
            "{package}"
        );
    }
}
