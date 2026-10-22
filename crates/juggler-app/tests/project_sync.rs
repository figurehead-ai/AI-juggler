//! AJ-E1507: a project's rules files and MCP servers, made the same across
//! tools and accounts in one reviewed click. Nothing is written until the
//! user applies it; a file that changed since it was shown, or a link, is
//! never written; Settings undoes what was applied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::project_sync::{
    RulesSyncInfo, apply_impl, diff_impl, rules_impl, servers_apply_impl, servers_impl,
};
use juggler_app::features::switch_setup::{list_impl, undo_impl};
use juggler_app::state::AppState;

struct Lab {
    state: Arc<AppState>,
    project: String,
    root: PathBuf,
    work: PathBuf,
    claude: PathBuf,
    personal: PathBuf,
    codex: PathBuf,
    codex_profile: String,
    _dir: tempfile::TempDir,
}

/// A `claude` (and `codex`): `claude mcp add-json` writes the entry into
/// the account's state file, as Claude does; anything else just exits.
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

/// A Claude Code account and a Codex one, and a project, `shop`, that only
/// Claude Code has worked in.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().canonicalize().unwrap();
    let bin = stub(&work);
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&bin)));
    registry.register(Arc::new(CodexAdapter::with_bin(&bin)));
    let state = Arc::new(AppState::init_with_registry(work.join("app-data"), registry).unwrap());
    let make = |tool: &str, label: &str| {
        let account = commands::create_account(&state, tool, label, None).unwrap();
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        let dir = state.accounts_dir().join(tool).join(&account.id);
        std::fs::create_dir_all(&dir).unwrap();
        (profile.id, dir)
    };
    let (claude_profile, claude) = make(CLAUDE_CODE_ID, "Work");
    let (_, personal) = make(CLAUDE_CODE_ID, "Personal");
    let (codex_profile, codex) = make(CODEX_ID, "Codex");
    std::fs::write(codex.join("config.toml"), "model = \"gpt-6\"\n").unwrap();
    let root = work.join("shop");
    std::fs::create_dir_all(&root).unwrap();
    let launched = commands::launch_profile(
        &state,
        &claude_profile,
        Some(root.to_string_lossy().into_owned()),
    )
    .unwrap();
    let project = {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.as_str(),
            ))
            .unwrap()
            .project
            .expect("the launch is in the project")
            .to_string()
    };
    Lab {
        state,
        project,
        root,
        work,
        claude,
        personal,
        codex,
        codex_profile,
        _dir: dir,
    }
}

/// Writes `text` to `name` in the project, changed `age` ago.
fn write(lab: &Lab, name: &str, text: &str, age: u64) {
    let path = lab.root.join(name);
    std::fs::write(&path, text).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(age))
        .unwrap();
}

fn read(lab: &Lab, name: &str) -> String {
    std::fs::read_to_string(lab.root.join(name)).unwrap()
}

/// Every file under `dir`: its bytes, or where a link points.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.file_type().is_symlink() {
                let to = std::fs::read_link(&path).unwrap();
                out.insert(path, to.to_string_lossy().as_bytes().to_vec());
            } else if meta.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

/// AI Juggler's own SQLite files, right in its data folder.
fn is_database(path: &Path, data_dir: &Path) -> bool {
    path.parent() == Some(data_dir)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                [".db", ".db-wal", ".db-shm", ".sqlite"]
                    .iter()
                    .any(|end| name.ends_with(end))
            })
}

fn file<'a>(
    info: &'a RulesSyncInfo,
    name: &str,
) -> &'a juggler_app::features::project_sync::RulesFileInfo {
    info.files.iter().find(|file| file.name == name).unwrap()
}

#[test]
fn nothing_is_written_until_the_user_applies() {
    // AJ-E1507 Req.5, the guardrail: listing, comparing and the diffs
    // change no file anywhere: the project, the accounts or the tools'
    // configs.
    let lab = lab();
    write(&lab, "AGENTS.md", "# Rules\nUse tabs.\n", 60);
    write(&lab, "CLAUDE.md", "# Rules\nUse spaces.\n", 1);
    std::fs::write(
        lab.claude.join(".claude.json"),
        serde_json::json!({"mcpServers": {"docs": {"type": "stdio", "command": "docs-mcp"}}})
            .to_string(),
    )
    .unwrap();
    let before = snapshot(&lab.work);

    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    rules_impl(&lab.state, &lab.project, Some("AGENTS.md")).unwrap();
    diff_impl(&lab.state, &lab.project, "CLAUDE.md", "AGENTS.md", "copy").unwrap();
    diff_impl(&lab.state, &lab.project, "AGENTS.md", "CLAUDE.md", "read").unwrap();
    servers_impl(&lab.state, &lab.project).unwrap();

    let after = snapshot(&lab.work);
    let changed: Vec<&PathBuf> = after
        .iter()
        .filter(|(path, bytes)| before.get(*path) != Some(bytes))
        .map(|(path, _)| path)
        // AI Juggler's own database may note the reads; nothing else may
        // change, the accounts' folders inside app-data included.
        .filter(|path| !is_database(path, &lab.work.join("app-data")))
        .collect();
    assert!(changed.is_empty(), "written without an apply: {changed:?}");
    assert_eq!(before.len(), after.len());

    assert_eq!(
        rules.master.as_deref(),
        Some("CLAUDE.md"),
        "the newest is the master"
    );
    assert_eq!(file(&rules, "AGENTS.md").standing, "differs");
    assert_eq!(rules.summary.as_deref(), Some("1 rules file differs"));
    assert!(!rules.in_sync);
}

#[test]
fn the_master_applied_writes_that_file_only_and_settings_undoes_it() {
    let lab = lab();
    write(&lab, "AGENTS.md", "# Rules\nUse tabs.\n", 60);
    write(
        &lab,
        "CLAUDE.md",
        "# Rules\nUse spaces.\nKeep it short.\n",
        1,
    );
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    let diff = diff_impl(&lab.state, &lab.project, "CLAUDE.md", "AGENTS.md", "copy").unwrap();
    assert_eq!((diff.added, diff.removed), (2, 1));
    assert!(
        diff.groups[0]
            .iter()
            .any(|line| line.change == "removed" && line.text == "Use tabs.")
    );

    let applied = apply_impl(
        &lab.state,
        &lab.project,
        "CLAUDE.md",
        "AGENTS.md",
        "copy",
        file(&rules, "AGENTS.md").sha256.as_deref(),
        file(&rules, "CLAUDE.md").sha256.as_deref().unwrap(),
    )
    .unwrap();
    assert_eq!(
        applied.message,
        "AGENTS.md now has the rules from CLAUDE.md"
    );
    assert_eq!(read(&lab, "AGENTS.md"), read(&lab, "CLAUDE.md"));
    let now = rules_impl(&lab.state, &lab.project, None).unwrap();
    assert!(now.in_sync);
    assert_eq!(now.summary.as_deref(), Some("Rules in sync"));

    // Settings lists it under the project, and undoes it.
    let listed = list_impl(&lab.state).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].place, "shop");
    assert_eq!(listed[0].what, "the rules in AGENTS.md");
    undo_impl(&lab.state, &listed[0].record).unwrap();
    assert_eq!(read(&lab, "AGENTS.md"), "# Rules\nUse tabs.\n");
    assert!(list_impl(&lab.state).unwrap().is_empty());
    let backups = lab.state.data_dir.join("sync-backups");
    assert_eq!(
        std::fs::read_dir(backups).unwrap().count(),
        0,
        "the backup goes with the undo"
    );
}

#[test]
fn a_file_changed_since_it_was_shown_is_never_overwritten() {
    let lab = lab();
    write(&lab, "AGENTS.md", "Old.\n", 60);
    write(&lab, "CLAUDE.md", "New.\n", 1);
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    // The user edits AGENTS.md while the sheet is open.
    write(&lab, "AGENTS.md", "Edited meanwhile.\n", 0);
    let refused = apply_impl(
        &lab.state,
        &lab.project,
        "CLAUDE.md",
        "AGENTS.md",
        "copy",
        file(&rules, "AGENTS.md").sha256.as_deref(),
        file(&rules, "CLAUDE.md").sha256.as_deref().unwrap(),
    );
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("changed since you looked")
    );
    assert_eq!(read(&lab, "AGENTS.md"), "Edited meanwhile.\n");
    // Undo refuses too once the user has changed what was applied.
    let rules = rules_impl(&lab.state, &lab.project, Some("CLAUDE.md")).unwrap();
    apply_impl(
        &lab.state,
        &lab.project,
        "CLAUDE.md",
        "AGENTS.md",
        "copy",
        file(&rules, "AGENTS.md").sha256.as_deref(),
        file(&rules, "CLAUDE.md").sha256.as_deref().unwrap(),
    )
    .unwrap();
    write(&lab, "AGENTS.md", "New.\nAnd mine.\n", 0);
    let record = list_impl(&lab.state).unwrap()[0].record.clone();
    assert!(undo_impl(&lab.state, &record).is_err());
    assert_eq!(read(&lab, "AGENTS.md"), "New.\nAnd mine.\n");
}

#[test]
fn a_claude_md_that_reads_agents_md_is_in_sync_and_can_be_made_to() {
    let lab = lab();
    write(&lab, "AGENTS.md", "# Rules\n", 60);
    write(
        &lab,
        "CLAUDE.md",
        "@AGENTS.md\n\nClaude only: be brief.\n",
        1,
    );
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    assert_eq!(
        rules.master.as_deref(),
        Some("AGENTS.md"),
        "a file that reads another isn't a master"
    );
    assert_eq!(file(&rules, "CLAUDE.md").standing, "reads");
    assert_eq!(file(&rules, "CLAUDE.md").words, "Reads AGENTS.md");
    assert!(!file(&rules, "CLAUDE.md").can_be_master);
    assert!(rules.in_sync);

    // A CLAUDE.md of its own can be pointed at AGENTS.md instead of copied.
    write(&lab, "CLAUDE.md", "Different rules.\n", 1);
    let rules = rules_impl(&lab.state, &lab.project, Some("AGENTS.md")).unwrap();
    assert_eq!(
        file(&rules, "CLAUDE.md").import_line.as_deref(),
        Some("@AGENTS.md")
    );
    assert_eq!(
        file(&rules, "AGENTS.md").import_line,
        None,
        "Codex has no import line"
    );
    let applied = apply_impl(
        &lab.state,
        &lab.project,
        "AGENTS.md",
        "CLAUDE.md",
        "read",
        file(&rules, "CLAUDE.md").sha256.as_deref(),
        file(&rules, "AGENTS.md").sha256.as_deref().unwrap(),
    )
    .unwrap();
    assert_eq!(applied.message, "CLAUDE.md now reads AGENTS.md");
    assert_eq!(read(&lab, "CLAUDE.md"), "@AGENTS.md\n");
    // AGENTS.md can't be told to read CLAUDE.md: Codex has no such line.
    assert!(diff_impl(&lab.state, &lab.project, "CLAUDE.md", "AGENTS.md", "read").is_err());
}

#[test]
fn a_missing_file_counts_only_for_a_tool_that_works_in_the_project() {
    let lab = lab();
    write(&lab, "CLAUDE.md", "# Rules\n", 1);
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    assert_eq!(file(&rules, "AGENTS.md").standing, "missing");
    assert_eq!(
        rules.summary.as_deref(),
        Some("Rules in sync"),
        "Codex hasn't worked here: its missing file isn't a difference"
    );

    commands::launch_profile(
        &lab.state,
        &lab.codex_profile,
        Some(lab.root.to_string_lossy().into_owned()),
    )
    .unwrap();
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    assert_eq!(rules.summary.as_deref(), Some("1 rules file differs"));

    // Applying creates it; the undo removes it again.
    apply_impl(
        &lab.state,
        &lab.project,
        "CLAUDE.md",
        "AGENTS.md",
        "copy",
        None,
        file(&rules, "CLAUDE.md").sha256.as_deref().unwrap(),
    )
    .unwrap();
    assert_eq!(read(&lab, "AGENTS.md"), "# Rules\n");
    let record = list_impl(&lab.state).unwrap()[0].record.clone();
    undo_impl(&lab.state, &record).unwrap();
    assert!(!lab.root.join("AGENTS.md").exists());
}

#[test]
fn a_link_is_never_written() {
    let lab = lab();
    write(&lab, "CLAUDE.md", "# Rules\n", 1);
    std::os::unix::fs::symlink("CLAUDE.md", lab.root.join("AGENTS.md")).unwrap();
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    assert_eq!(file(&rules, "AGENTS.md").standing, "link");
    assert!(rules.in_sync);

    // A link to a file outside the rules: left as it is, whatever it says.
    let elsewhere = lab.work.join("team-rules.md");
    std::fs::write(&elsewhere, "Team rules.\n").unwrap();
    std::fs::remove_file(lab.root.join("AGENTS.md")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, lab.root.join("AGENTS.md")).unwrap();
    let rules = rules_impl(&lab.state, &lab.project, Some("CLAUDE.md")).unwrap();
    let agents = file(&rules, "AGENTS.md");
    assert!(!agents.can_apply);
    assert_eq!(
        agents.words,
        "A link to another file, so it's left as it is"
    );
    let refused = apply_impl(
        &lab.state,
        &lab.project,
        "CLAUDE.md",
        "AGENTS.md",
        "copy",
        agents.sha256.as_deref(),
        file(&rules, "CLAUDE.md").sha256.as_deref().unwrap(),
    );
    assert!(refused.is_err());
    assert_eq!(
        std::fs::read_to_string(&elsewhere).unwrap(),
        "Team rules.\n"
    );
}

#[test]
fn only_the_projects_rules_files_can_be_written() {
    let lab = lab();
    write(&lab, "CLAUDE.md", "# Rules\n", 1);
    let rules = rules_impl(&lab.state, &lab.project, None).unwrap();
    let sha = file(&rules, "CLAUDE.md").sha256.clone().unwrap();
    for name in ["../escape.md", "README.md", "/tmp/x.md"] {
        assert!(
            apply_impl(
                &lab.state,
                &lab.project,
                "CLAUDE.md",
                name,
                "copy",
                None,
                &sha
            )
            .is_err(),
            "{name}"
        );
    }
    assert!(!lab.work.join("escape.md").exists());
    assert!(!lab.root.join("README.md").exists());
}

#[test]
fn a_server_is_copied_across_tools_without_its_secret_and_undone() {
    // AJ-E1507 Req.3: MCP servers per account, copied on request in the
    // target tool's form; a value that looks like a secret stays behind,
    // and the user is told.
    let lab = lab();
    std::fs::write(
        lab.claude.join(".claude.json"),
        serde_json::json!({"mcpServers": {"github": {
            "type": "stdio", "command": "npx", "args": ["-y", "gh-mcp"],
            "env": {"GITHUB_TOKEN": "ghp_this_must_never_travel_0123456789", "LOG": "info"}
        }}})
        .to_string(),
    )
    .unwrap();
    let servers = servers_impl(&lab.state, &lab.project).unwrap();
    let work = servers
        .accounts
        .iter()
        .find(|account| account.label == "Work")
        .unwrap();
    assert_eq!(work.servers, vec!["github".to_owned()]);
    let codex = servers
        .accounts
        .iter()
        .find(|account| account.label == "Codex")
        .unwrap();
    assert!(codex.servers.is_empty());
    let copy = servers
        .copies
        .iter()
        .find(|copy| copy.server == "github" && copy.to_label == "Codex")
        .expect("Codex is offered Work's server");
    assert_eq!(copy.from_label, "Work");
    assert!(
        copy.note
            .as_deref()
            .unwrap_or_default()
            .contains("GITHUB_TOKEN"),
        "{copy:?}"
    );

    let before = std::fs::read_to_string(lab.codex.join("config.toml")).unwrap();
    let applied =
        servers_apply_impl(&lab.state, &lab.project, &copy.from, &copy.to, &copy.offer).unwrap();
    assert!(applied.record.is_some(), "{}", applied.message);
    let config = std::fs::read_to_string(lab.codex.join("config.toml")).unwrap();
    assert!(config.contains("[mcp_servers.github]"));
    assert!(!config.contains("ghp_this_must_never_travel"));

    let record = applied.record.unwrap();
    undo_impl(&lab.state, &record).unwrap();
    assert_eq!(
        std::fs::read_to_string(lab.codex.join("config.toml")).unwrap(),
        before
    );
}

#[test]
fn every_pair_of_tools_is_offered_what_it_lacks_the_same_tool_first() {
    // AJ-E1507's quality gate: fixtures for each pair of tools. Codex has
    // a server neither Claude Code account has; Work has one Personal
    // lacks. Personal gets Work's as it is (the same tool first), and both
    // can get Codex's in their own form.
    let lab = lab();
    std::fs::write(
        lab.codex.join("config.toml"),
        "model = \"gpt-6\"\n\n[mcp_servers.search]\ncommand = \"search-mcp\"\nargs = [\"--fast\"]\n",
    )
    .unwrap();
    std::fs::write(
        lab.claude.join(".claude.json"),
        serde_json::json!({"mcpServers": {"docs": {"type": "stdio", "command": "docs-mcp"}}})
            .to_string(),
    )
    .unwrap();
    let servers = servers_impl(&lab.state, &lab.project).unwrap();
    let offer = |server: &str, to: &str| {
        servers
            .copies
            .iter()
            .find(|copy| copy.server == server && copy.to_label == to)
            .unwrap_or_else(|| panic!("{to} is offered {server}: {:?}", servers.copies))
            .clone()
    };
    let docs = offer("docs", "Personal");
    assert_eq!(
        (docs.from_label.as_str(), docs.verdict.as_str()),
        ("Work", "copy")
    );
    // Across tools: whatever the target tool's adapter says, as long as
    // it can be set up there.
    let can = |verdict: &str| matches!(verdict, "copy" | "translate");
    assert!(can(&offer("docs", "Codex").verdict));
    assert_eq!(offer("search", "Work").from_label, "Codex");
    assert!(can(&offer("search", "Personal").verdict));

    // Codex → Claude Code: the server lands in Personal's own config.
    let search = offer("search", "Personal");
    let applied = servers_apply_impl(
        &lab.state,
        &lab.project,
        &search.from,
        &search.to,
        &search.offer,
    )
    .unwrap();
    assert!(applied.record.is_some(), "{}", applied.message);
    let config = std::fs::read_to_string(lab.personal.join(".claude.json")).unwrap();
    assert!(config.contains("search-mcp"), "{config}");
}

#[test]
fn listing_stays_inside_its_budget() {
    // AJ-E1507's budget: listing under 300 ms per project. This thread's
    // CPU time, the best of five, so a busy Mac doesn't decide it.
    let lab = lab();
    write(&lab, "AGENTS.md", &"Use tabs.\n".repeat(2000), 60);
    write(&lab, "CLAUDE.md", &"Use spaces.\n".repeat(2000), 1);
    let mut best = Duration::MAX;
    for _ in 0..5 {
        let started = cpu_time::ThreadTime::now();
        rules_impl(&lab.state, &lab.project, None).unwrap();
        servers_impl(&lab.state, &lab.project).unwrap();
        best = best.min(started.elapsed());
    }
    assert!(best < Duration::from_millis(300), "{best:?} CPU");
}
