//! AJ-E1542: before a switch, the target is offered what it lacks, and gets
//! only what the user picks; secrets never travel; it can all be undone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::switch_setup::{apply_impl, check_impl, undo_impl};
use juggler_app::state::AppState;

/// A `claude` that records its arguments (for `claude mcp add-json`).
fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/mcp-args.tmp\"\nmv \"$CLAUDE_CONFIG_DIR/mcp-args.tmp\" \"$CLAUDE_CONFIG_DIR/mcp-args\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

struct Lab {
    state: Arc<AppState>,
    session: String,
    source_dir: PathBuf,
    claude_b: (String, PathBuf),
    codex: (String, PathBuf),
    _work: tempfile::TempDir,
}

fn lab() -> Lab {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let codex = Arc::new(CodexAdapter::with_bin(&stub));
    registry.register(codex.clone());
    registry.register_handover(codex);
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let make = |tool: &str, label: &str| {
        let account = commands::create_account(&state, tool, label, None).unwrap();
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        let dir = state.accounts_dir().join(tool).join(&account.id);
        std::fs::create_dir_all(&dir).unwrap();
        (account.id, profile.id, dir)
    };
    let (_, profile_a, a) = make(CLAUDE_CODE_ID, "Work");
    let (_, profile_b, b) = make(CLAUDE_CODE_ID, "Personal");
    let (_, profile_x, x) = make(CODEX_ID, "Codex");

    // What Work has: an MCP server holding a secret, a skill, a command,
    // a skill synced from claude.ai, and rules.
    std::fs::write(
        a.join(".claude.json"),
        serde_json::json!({"mcpServers": {"github": {
            "type": "stdio", "command": "npx", "args": ["-y", "gh-mcp"],
            "env": {"GITHUB_TOKEN": "ghp_this_must_never_travel_0123456789", "LOG": "info"}
        }}})
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(a.join("skills/release-notes")).unwrap();
    std::fs::write(
        a.join("skills/release-notes/SKILL.md"),
        "---\nname: release-notes\ndescription: x\n---\nWrite them.\n",
    )
    .unwrap();
    std::fs::create_dir_all(a.join("skills/synced/set/team-style")).unwrap();
    std::fs::create_dir_all(a.join("commands")).unwrap();
    std::fs::write(
        a.join("commands/review.md"),
        "# Review the diff\nLook at $ARGUMENTS\n",
    )
    .unwrap();
    std::fs::write(a.join("CLAUDE.md"), "Be brief.\n").unwrap();
    // Personal already has its own rules: never given Work's.
    std::fs::write(b.join("CLAUDE.md"), "Personal rules.\n").unwrap();
    std::fs::write(x.join("config.toml"), "model = \"gpt-6\"\n").unwrap();

    let launched = commands::launch_profile(
        &state,
        &profile_a,
        Some(project.to_string_lossy().into_owned()),
    )
    .unwrap();
    let _ = profile_b.clone();
    Lab {
        state,
        session: launched.session_id,
        source_dir: a,
        claude_b: (profile_b, b),
        codex: (profile_x, x),
        _work: work,
    }
}

#[test]
fn the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked() {
    let lab = lab();
    let (profile_b, b) = &lab.claude_b;
    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    let verdict = |name: &str| {
        check
            .offers
            .iter()
            .find(|offer| offer.name == name)
            .map(|offer| (offer.verdict.as_str(), offer.suggested))
    };
    assert_eq!(verdict("release-notes"), Some(("copy", true)));
    assert_eq!(verdict("review"), Some(("copy", true)));
    assert_eq!(
        verdict("github"),
        Some(("needs-you", false)),
        "{:?}",
        check.offers
    );
    assert_eq!(verdict("team-style"), Some(("impossible", false)));
    assert_eq!(verdict("CLAUDE.md"), None, "Personal has its own rules");
    assert!(check.already_there >= 1);
    let note = check
        .offers
        .iter()
        .find(|o| o.name == "github")
        .unwrap()
        .note
        .clone()
        .unwrap();
    assert!(note.contains("GITHUB_TOKEN"), "{note}");

    // Nothing is written by the check.
    assert!(!b.join("skills").exists());

    let pick: Vec<String> = check
        .offers
        .iter()
        .filter(|offer| ["release-notes", "github"].contains(&offer.name.as_str()))
        .map(|offer| offer.id.clone())
        .collect();
    let applied = apply_impl(&lab.state, &lab.session, profile_b, &pick).unwrap();
    assert!(applied.iter().all(|a| a.record.is_some()), "{applied:?}");
    assert_eq!(
        std::fs::read_to_string(b.join("skills/release-notes/SKILL.md")).unwrap(),
        std::fs::read_to_string(lab.source_dir.join("skills/release-notes/SKILL.md")).unwrap()
    );
    assert!(!b.join("commands").exists(), "only what was picked");
    let args = std::fs::read_to_string(b.join("mcp-args")).unwrap();
    assert!(
        args.starts_with("mcp\nadd-json\n-s\nuser\ngithub\n"),
        "{args}"
    );
    assert!(
        args.contains("${GITHUB_TOKEN}"),
        "the secret becomes a reference: {args}"
    );
    assert!(
        !args.contains("ghp_this"),
        "the secret's value never travels"
    );

    // Undo removes the copied skill, exactly as it was made.
    let skill = applied
        .iter()
        .find(|a| a.message.contains("release-notes"))
        .unwrap()
        .record
        .clone()
        .unwrap();
    undo_impl(&lab.state, &skill).unwrap();
    assert!(!b.join("skills/release-notes").exists());
    assert!(undo_impl(&lab.state, &skill).is_err(), "undone once");
}

#[test]
fn codex_is_offered_the_claude_setup_in_its_own_form() {
    let lab = lab();
    let (profile_x, x) = &lab.codex;
    let check = check_impl(&lab.state, &lab.session, profile_x).unwrap();
    let find = |name: &str| {
        check
            .offers
            .iter()
            .find(|offer| offer.name == name)
            .unwrap()
    };
    assert_eq!(find("review").verdict, "translate");
    assert_eq!(find("CLAUDE.md").verdict, "translate");
    let pick = vec![find("github").id.clone(), find("CLAUDE.md").id.clone()];
    let applied = apply_impl(&lab.state, &lab.session, profile_x, &pick).unwrap();
    assert!(applied.iter().all(|a| a.record.is_some()), "{applied:?}");
    let config = std::fs::read_to_string(x.join("config.toml")).unwrap();
    assert!(
        config.starts_with("model = \"gpt-6\"\n"),
        "Codex's config keeps what it had"
    );
    let table: toml::Table = config.parse().unwrap();
    assert_eq!(
        table["mcp_servers"]["github"]["env_vars"][0].as_str(),
        Some("GITHUB_TOKEN")
    );
    assert!(!config.contains("ghp_this"));
    assert_eq!(
        std::fs::read_to_string(x.join("AGENTS.md")).unwrap(),
        "Be brief.\n"
    );

    // Undoing the server takes out exactly what was appended.
    let server = applied
        .iter()
        .find(|a| a.message.contains("github"))
        .unwrap()
        .record
        .clone()
        .unwrap();
    undo_impl(&lab.state, &server).unwrap();
    assert_eq!(
        std::fs::read_to_string(x.join("config.toml")).unwrap(),
        "model = \"gpt-6\"\n"
    );
}
