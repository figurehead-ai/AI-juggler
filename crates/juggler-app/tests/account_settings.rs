//! AJ-E1508: a new account starts with the user's usual settings from an
//! account of the same tool they pick: MCP servers, permissions, the status
//! line, plugins. Credentials, history and caches are never copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::account_settings::{apply_impl, check_impl};
use juggler_app::features::switch_setup::{self, list_impl, undo_impl};
use juggler_app::state::AppState;

/// A `claude` whose `mcp add-json` writes the entry into the account's
/// state file, as Claude does.
fn stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("tool-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nif [ \"$2\" = add-json ]; then printf '{\"mcpServers\":{\"%s\":%s}}' \"$6\" \"$7\" > \"$CLAUDE_CONFIG_DIR/.claude.json\"; fi\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

struct Lab {
    state: Arc<AppState>,
    _dir: tempfile::TempDir,
    work: PathBuf,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().canonicalize().unwrap();
    let bin = stub(&work);
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&bin)));
    registry.register(Arc::new(CodexAdapter::with_bin(&bin)));
    let state = Arc::new(AppState::init_with_registry(work.join("app-data"), registry).unwrap());
    Lab {
        state,
        _dir: dir,
        work,
    }
}

/// A fresh account of `tool` (as Add account makes one), and its folder.
fn fresh(lab: &Lab, tool: &str, label: &str) -> (String, PathBuf) {
    let account = commands::create_account(&lab.state, tool, label, None).unwrap();
    let dir = lab.state.accounts_dir().join(tool).join(&account.id);
    (account.id, dir)
}

/// Every file under `dir`, relative.
fn files(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(dir)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    out
}

/// Work: a Claude Code account with its sign-in, history, caches and its
/// usual settings, its status line under AI Juggler's capture.
fn claude_work(lab: &Lab) -> (String, PathBuf) {
    let (id, dir) = fresh(lab, CLAUDE_CODE_ID, "Work");
    std::fs::write(
        dir.join(".credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-secret"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        serde_json::json!({
            "oauthAccount": {"emailAddress": "you@example.com"},
            "mcpServers": {"docs": {"type": "stdio", "command": "docs-mcp"}}
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("projects/-Users-you-shop")).unwrap();
    std::fs::write(dir.join("projects/-Users-you-shop/conv.jsonl"), "{}\n").unwrap();
    std::fs::create_dir_all(dir.join("todos")).unwrap();
    std::fs::write(dir.join("todos/t.json"), "[]").unwrap();
    std::fs::create_dir_all(dir.join("statsig")).unwrap();
    std::fs::write(dir.join("statsig/cache"), "x").unwrap();
    std::fs::write(
        dir.join("history.jsonl"),
        "{\"display\":\"secret prompt\"}\n",
    )
    .unwrap();
    // AI Juggler's capture holds the status line; the user's own is in the
    // backup beside it.
    std::fs::write(
        dir.join("juggler-statusline-backup.json"),
        r#"{"statusLine":{"type":"command","command":"~/bin/my-line"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("settings.json"),
        r#"{
  "apiKey": "sk-should-never-travel",
  "permissions": {"allow": ["Bash(npm test)"], "deny": ["Read(.env)"]},
  "statusLine": {"type": "command", "command": "/x/juggler-statusline.sh"},
  "enabledPlugins": {"tidy@market": true}
}
"#,
    )
    .unwrap();
    (id, dir)
}

#[test]
fn a_new_account_gets_the_usual_settings_and_never_a_credential_history_or_cache() {
    // AJ-E1508 Req.1 and Req.2, the guardrail: after the copy, the new
    // account's folder holds exactly its own marker and the settings files;
    // nothing of the source's sign-in, history or caches.
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let (new, new_dir) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let before = files(&new_dir);

    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let verdict = |kind: &str| {
        check
            .offers
            .iter()
            .find(|offer| offer.kind == kind)
            .unwrap_or_else(|| panic!("{kind} offered: {:?}", check.offers))
            .clone()
    };
    assert_eq!(verdict("mcp-server").verdict, "copy");
    assert_eq!(verdict("permissions").verdict, "copy");
    assert_eq!(verdict("status-line").verdict, "copy");
    assert_eq!(
        verdict("plugin").verdict,
        "needs-you",
        "a plugin needs the tool's own install"
    );
    assert!(
        check
            .offers
            .iter()
            .all(|offer| offer.kind != "skill" && offer.kind != "main-rules")
    );
    // Every one that can be set up starts ticked.
    let picked: Vec<String> = check
        .offers
        .iter()
        .filter(|offer| offer.suggested)
        .map(|offer| offer.id.clone())
        .collect();
    assert_eq!(picked.len(), 3);

    let applied = apply_impl(&lab.state, &new, &work, &picked).unwrap();
    assert!(applied.iter().all(|a| a.record.is_some()), "{applied:?}");

    let after = files(&new_dir);
    let added: BTreeSet<&String> = after.difference(&before).collect();
    assert_eq!(
        added,
        [".claude.json".to_owned(), "settings.json".to_owned()]
            .iter()
            .collect(),
        "only settings files are new"
    );
    let settings = std::fs::read_to_string(new_dir.join("settings.json")).unwrap();
    assert!(settings.contains("Bash(npm test)"), "{settings}");
    assert!(
        settings.contains("~/bin/my-line"),
        "the user's own status line, not the capture: {settings}"
    );
    assert!(!settings.contains("juggler-statusline"), "{settings}");
    for secret in [
        "sk-should-never-travel",
        "sk-ant-secret",
        "you@example.com",
        "secret prompt",
    ] {
        for file in &after {
            let text = std::fs::read_to_string(new_dir.join(file)).unwrap_or_default();
            assert!(!text.contains(secret), "{secret} reached {file}");
        }
    }

    // Each can be undone from Settings, named by the account.
    let listed = list_impl(&lab.state).unwrap();
    assert_eq!(listed.len(), 3);
    assert!(listed.iter().all(|row| row.place == "Side project"));
    for row in listed
        .iter()
        .filter(|row| row.what != "the MCP server docs")
    {
        undo_impl(&lab.state, &row.record).unwrap();
    }
    let settings = std::fs::read_to_string(new_dir.join("settings.json")).unwrap();
    assert!(
        !settings.contains("permissions") && !settings.contains("statusLine"),
        "{settings}"
    );
}

#[test]
fn a_setting_the_new_account_has_is_left_as_it_is() {
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let (new, new_dir) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    std::fs::write(
        new_dir.join("settings.json"),
        r#"{"permissions": {"allow": []}}"#,
    )
    .unwrap();
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    assert!(
        check.offers.iter().all(|offer| offer.kind != "permissions"),
        "it has its own"
    );
    assert!(check.already_there >= 1);
}

#[test]
fn codex_homes_copy_their_approval_sandbox_and_status_line() {
    let lab = lab();
    let (work, work_dir) = fresh(&lab, CODEX_ID, "Codex work");
    std::fs::write(
        work_dir.join("auth.json"),
        r#"{"OPENAI_API_KEY":"sk-codex-secret"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(work_dir.join("sessions/2026")).unwrap();
    std::fs::write(work_dir.join("sessions/2026/r.jsonl"), "{}\n").unwrap();
    std::fs::write(work_dir.join("history.jsonl"), "{}\n").unwrap();
    std::fs::write(
        work_dir.join("config.toml"),
        "model = \"gpt-6\"\napproval_policy = \"on-request\"\nsandbox_mode = \"workspace-write\"\n\n[tui]\nstatus_line = [\"model\", \"context\"]\n",
    )
    .unwrap();
    let (new, new_dir) = fresh(&lab, CODEX_ID, "Codex side");
    let before = files(&new_dir);
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let picked: Vec<String> = check
        .offers
        .iter()
        .filter(|offer| offer.suggested)
        .map(|offer| offer.id.clone())
        .collect();
    assert_eq!(picked.len(), 2, "{:?}", check.offers);
    apply_impl(&lab.state, &new, &work, &picked).unwrap();
    let added: BTreeSet<String> = files(&new_dir).difference(&before).cloned().collect();
    assert_eq!(added, ["config.toml".to_owned()].into_iter().collect());
    let config: toml::Table = std::fs::read_to_string(new_dir.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(config["approval_policy"].as_str(), Some("on-request"));
    assert_eq!(config["sandbox_mode"].as_str(), Some("workspace-write"));
    assert_eq!(config["tui"]["status_line"].as_array().unwrap().len(), 2);
    assert!(config.get("model").is_none(), "only what was picked");
}

#[test]
fn another_tools_account_is_refused() {
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let (codex, _) = fresh(&lab, CODEX_ID, "Codex");
    let refused = check_impl(&lab.state, Some(&codex), &work)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("same tool"), "{refused}");
}

#[test]
fn a_switch_still_offers_no_account_settings() {
    // AJ-E1508 keeps the switch's check as it was (AJ-E1542): the target of
    // a switch keeps its own permissions and status line.
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let (personal, _) = fresh(&lab, CLAUDE_CODE_ID, "Personal");
    let work_profile = commands::create_profile(&lab.state, &work, "Main", None, None)
        .unwrap()
        .id;
    let personal_profile = commands::create_profile(&lab.state, &personal, "Main", None, None)
        .unwrap()
        .id;
    let project = lab.work.join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let launched = commands::launch_profile(
        &lab.state,
        &work_profile,
        Some(project.to_string_lossy().into_owned()),
    )
    .unwrap();
    let check =
        switch_setup::check_impl(&lab.state, &launched.session_id, &personal_profile).unwrap();
    assert!(
        check
            .offers
            .iter()
            .all(|offer| offer.kind != "permissions" && offer.kind != "status-line"),
        "{:?}",
        check.offers
    );
    assert!(check.offers.iter().any(|offer| offer.kind == "mcp-server"));
}

#[test]
fn the_offers_before_the_account_exists_are_the_ones_it_gets() {
    // Add account shows the list before "Create": the ids picked then are
    // set up once the account is made.
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let before = check_impl(&lab.state, None, &work).unwrap();
    let (new, _) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let after = check_impl(&lab.state, Some(&new), &work).unwrap();
    let ids = |check: &juggler_app::features::switch_setup::SetupCheckInfo| {
        check
            .offers
            .iter()
            .map(|o| (o.id.clone(), o.verdict.clone(), o.suggested))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&before), ids(&after));
}

/// Replaces Work's settings.json with `text`.
fn work_settings(dir: &Path, text: &str) {
    std::fs::write(dir.join("settings.json"), text).unwrap();
}

#[test]
fn a_setting_with_a_secret_written_in_it_is_never_copied_or_recorded() {
    // Review of AJ-E1508 (H1): a token in the status line's command, and a
    // bearer token in a permission rule.
    let lab = lab();
    let (work, work_dir) = claude_work(&lab);
    work_settings(
        &work_dir,
        r#"{"permissions": {"allow": ["Bash(curl -H 'Authorization: Bearer sk-ant-hidden-1234')"]},
            "statusLine": {"type": "command", "command": "GITHUB_TOKEN=ghp_hidden5678 ~/bin/line"}}"#,
    );
    let (new, new_dir) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    for kind in ["permissions", "status-line"] {
        let offer = check.offers.iter().find(|o| o.kind == kind).unwrap();
        assert_eq!(offer.verdict, "impossible", "{offer:?}");
        assert!(!offer.suggested);
        assert!(
            offer.note.as_deref().unwrap_or_default().contains("secret"),
            "{offer:?}"
        );
    }
    // Picked anyway, by id: nothing of them is written.
    let ids: Vec<String> = check.offers.iter().map(|o| o.id.clone()).collect();
    apply_impl(&lab.state, &new, &work, &ids).unwrap();
    let records =
        std::fs::read_to_string(lab.state.data_dir.join("switch-setups.json")).unwrap_or_default();
    for secret in ["sk-ant-hidden-1234", "ghp_hidden5678"] {
        assert!(!records.contains(secret), "{secret} in the records");
        for file in files(&new_dir) {
            let text = std::fs::read_to_string(new_dir.join(&file)).unwrap_or_default();
            assert!(!text.contains(secret), "{secret} reached {file}");
        }
    }
}

#[test]
fn ai_jugglers_fail_safe_status_line_is_never_copied_nor_a_stale_backup() {
    // Review of AJ-E1508 (M1): under the opt-in capture's fail-safe line,
    // the user's own isn't known here; a backup left beside it is stale.
    let lab = lab();
    let (work, work_dir) = claude_work(&lab);
    work_settings(
        &work_dir,
        r#"{"statusLine": {"type": "command", "command": "sh -c 'exec \"$1\"' aij-statusline /x/w 'old'"}}"#,
    );
    let (new, _) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let line = check
        .offers
        .iter()
        .find(|o| o.kind == "status-line")
        .unwrap();
    assert_eq!(line.verdict, "impossible");
    assert!(
        line.note
            .as_deref()
            .unwrap_or_default()
            .contains("usage capture"),
        "{line:?}"
    );
}

#[test]
fn permissions_say_what_they_hold_and_a_bypass_is_never_ticked() {
    // Review of AJ-E1508 (M5).
    let lab = lab();
    let (work, work_dir) = claude_work(&lab);
    let (new, _) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let permissions = check
        .offers
        .iter()
        .find(|o| o.kind == "permissions")
        .unwrap();
    assert_eq!(permissions.about.as_deref(), Some("1 allowed, 1 denied"));
    assert!(permissions.suggested);

    work_settings(
        &work_dir,
        r#"{"permissions": {"allow": [], "defaultMode": "bypassPermissions"}}"#,
    );
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let permissions = check
        .offers
        .iter()
        .find(|o| o.kind == "permissions")
        .unwrap();
    assert_eq!(permissions.verdict, "needs-you");
    assert!(!permissions.suggested, "never ticked for the user");
    assert_eq!(permissions.about.as_deref(), Some("mode bypassPermissions"));
    // Ticked by the user, it's set up.
    let applied = apply_impl(&lab.state, &new, &work, &[permissions.id.clone()]).unwrap();
    assert!(applied[0].record.is_some(), "{applied:?}");
}

#[test]
fn settings_name_a_setting_once_and_records_hold_no_values() {
    let lab = lab();
    let (work, _) = claude_work(&lab);
    let (new, _) = fresh(&lab, CLAUDE_CODE_ID, "Side project");
    let check = check_impl(&lab.state, Some(&new), &work).unwrap();
    let ids: Vec<String> = check
        .offers
        .iter()
        .filter(|o| o.suggested)
        .map(|o| o.id.clone())
        .collect();
    let applied = apply_impl(&lab.state, &new, &work, &ids).unwrap();
    assert!(
        applied
            .iter()
            .any(|a| a.message == "Set up the permissions"),
        "{applied:?}"
    );
    let listed: Vec<String> = list_impl(&lab.state)
        .unwrap()
        .into_iter()
        .map(|row| row.what)
        .collect();
    assert!(listed.contains(&"the permissions".to_owned()), "{listed:?}");
    assert!(listed.contains(&"the status line".to_owned()), "{listed:?}");
    // The undo keeps each value's fingerprint, never the value.
    let records = std::fs::read_to_string(lab.state.data_dir.join("switch-setups.json")).unwrap();
    assert!(
        !records.contains("Bash(npm test)") && !records.contains("my-line"),
        "{records}"
    );
}
