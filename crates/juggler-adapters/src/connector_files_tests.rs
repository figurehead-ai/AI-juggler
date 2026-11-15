//! AJ-E3013: the connector's merge and unmerge per App type (the Claude
//! app's Code-tab folder, the Codex app's `config.toml` with and without
//! servers and comments), a changed file kept on Remove, and the
//! refusals (a link, a path outside the folder, someone else's file).

use std::path::{Path, PathBuf};

use super::*;
use juggler_adapter::connector::ConnectorInput;

const TOOLS: [&str; 2] = ["whoami", "send_message"];

fn input(version: &str) -> ConnectorInput<'static> {
    let version: &'static str = Box::leak(version.to_owned().into_boxed_str());
    ConnectorInput {
        agent: Path::new("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
        socket: Path::new("/u/Library/Application Support/AI Juggler/agent/bridge.sock"),
        version,
        server: "juggler",
        tools: &TOOLS,
    }
}

fn claude(version: &str) -> ConnectorPlan {
    crate::claude_connector::plan(&input(version))
}

fn codex() -> ConnectorPlan {
    crate::codex_connector::plan(&input("0.4.6"))
}

/// Every file under `dir` and its bytes, sorted.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walkdirs(dir)
        .into_iter()
        .map(|path| path.strip_prefix(dir).unwrap().to_path_buf())
        .collect();
    out.sort();
    out
}

fn walkdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdirs(&path));
            out.push(path);
        }
    }
    out
}

/// A Claude Code folder with a skill and settings of the user's own.
fn claude_folder() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("skills/release-notes")).unwrap();
    std::fs::write(
        root.path().join("skills/release-notes/SKILL.md"),
        "---\nname: release-notes\n---\n",
    )
    .unwrap();
    std::fs::write(root.path().join("settings.json"), "{\"model\":\"opus\"}\n").unwrap();
    root
}

#[test]
fn the_claude_folder_is_written_whole_hidden_files_too_and_removed_exactly() {
    let root = claude_folder();
    let before = (snapshot(root.path()), dirs(root.path()));
    let plan = claude("0.4.6");
    let preview = preview(root.path(), &plan, None).unwrap();
    assert!(preview.iter().all(|file| file.action == Action::Create));
    assert_eq!(
        preview[0].path,
        Path::new("skills/ai-juggler/.claude-plugin/plugin.json")
    );
    assert_eq!(
        status(root.path(), &plan, None).state,
        ConnectorState::Absent
    );

    let record = add(root.path(), &plan, None).unwrap();
    let manifest = root
        .path()
        .join("skills/ai-juggler/.claude-plugin/plugin.json");
    assert!(manifest.is_file(), "the hidden folder is written");
    assert_eq!(record.parts.len(), 2);
    assert!(
        record
            .dirs
            .contains(&PathBuf::from("skills/ai-juggler/.claude-plugin"))
    );
    for part in &record.parts {
        let RecordedPart::File { path, sha256 } = part else {
            unreachable!("whole files");
        };
        assert_eq!(
            sha256,
            &sha256_hex(&std::fs::read(root.path().join(path)).unwrap())
        );
    }
    assert_eq!(
        status(root.path(), &plan, Some(&record)).state,
        ConnectorState::Current
    );
    assert!(
        preview_of(root.path(), &plan, &record)
            .iter()
            .all(|file| file.action == Action::Same)
    );
    // Adding again writes nothing new.
    assert_eq!(add(root.path(), &plan, Some(&record)).unwrap(), record);

    let removal = remove(root.path(), &record).unwrap();
    assert_eq!(removal.removed.len(), 2);
    assert!(removal.kept.is_empty() && removal.left.is_none());
    assert_eq!(
        (snapshot(root.path()), dirs(root.path())),
        before,
        "the folder is as it was, the user's skill untouched"
    );
}

fn preview_of(dir: &Path, plan: &ConnectorPlan, record: &ConnectorRecord) -> Vec<PreviewFile> {
    preview(dir, plan, Some(record)).unwrap()
}

#[test]
fn remove_keeps_and_names_a_file_someone_changed() {
    let root = claude_folder();
    let plan = claude("0.4.6");
    let record = add(root.path(), &plan, None).unwrap();
    let skill = root
        .path()
        .join("skills/ai-juggler/skills/pool-tasks/SKILL.md");
    std::fs::write(&skill, "my own notes\n").unwrap();
    let found = status(root.path(), &plan, Some(&record));
    assert_eq!(found.state, ConnectorState::Changed);
    assert_eq!(
        found.changed,
        [PathBuf::from(
            "skills/ai-juggler/skills/pool-tasks/SKILL.md"
        )]
    );
    let removal = remove(root.path(), &record).unwrap();
    assert_eq!(
        removal.kept,
        [PathBuf::from(
            "skills/ai-juggler/skills/pool-tasks/SKILL.md"
        )]
    );
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), "my own notes\n");
    assert!(
        !root
            .path()
            .join("skills/ai-juggler/.claude-plugin")
            .exists()
    );
    let left = removal.left.unwrap();
    assert_eq!(left.parts.len(), 1, "the record keeps what stays");
    // Add won't write over it either.
    assert!(matches!(
        add(root.path(), &plan, Some(&left)),
        Err(ConnectorError::Refused(_))
    ));
}

#[test]
fn an_older_editions_files_are_outdated_and_add_brings_them_current() {
    let root = claude_folder();
    let old = add(root.path(), &claude("0.4.5"), None).unwrap();
    let plan = claude("0.4.6");
    assert_eq!(
        status(root.path(), &plan, Some(&old)).state,
        ConnectorState::Outdated
    );
    assert!(
        preview_of(root.path(), &plan, &old)
            .iter()
            .any(|file| file.action == Action::Replace)
    );
    let new = add(root.path(), &plan, Some(&old)).unwrap();
    assert_eq!(
        status(root.path(), &plan, Some(&new)).state,
        ConnectorState::Current
    );
    remove(root.path(), &new).unwrap();
    assert!(!root.path().join("skills/ai-juggler").exists());
}

#[test]
fn something_ai_juggler_didnt_write_is_never_replaced() {
    let root = claude_folder();
    let theirs = root
        .path()
        .join("skills/ai-juggler/.claude-plugin/plugin.json");
    std::fs::create_dir_all(theirs.parent().unwrap()).unwrap();
    std::fs::write(&theirs, "{\"name\":\"ai-juggler\",\"theirs\":true}").unwrap();
    let plan = claude("0.4.6");
    let shown = preview(root.path(), &plan, None).unwrap();
    assert_eq!(shown[0].action, Action::Refuse);
    assert_eq!(shown[0].before, None, "someone else's file is never shown");
    assert_eq!(
        status(root.path(), &plan, None).state,
        ConnectorState::Changed
    );
    let before = snapshot(root.path());
    assert!(matches!(
        add(root.path(), &plan, None),
        Err(ConnectorError::Refused(_))
    ));
    assert_eq!(snapshot(root.path()), before, "nothing written");
}

#[cfg(unix)]
#[test]
fn links_are_refused_wherever_they_are() {
    let plan = claude("0.4.6");
    // The skills folder is a link to somewhere else.
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), root.path().join("skills")).unwrap();
    assert!(matches!(
        add(root.path(), &plan, None),
        Err(ConnectorError::Refused(_))
    ));
    assert!(
        std::fs::read_dir(elsewhere.path())
            .unwrap()
            .next()
            .is_none()
    );
    // The config folder itself is a link.
    let link = elsewhere.path().join("config-link");
    let real = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(real.path(), &link).unwrap();
    assert!(matches!(
        add(&link, &plan, None),
        Err(ConnectorError::Refused(_))
    ));
    // config.toml is a link.
    let codex_home = tempfile::tempdir().unwrap();
    let target = real.path().join("other.toml");
    std::fs::write(&target, "model = \"x\"\n").unwrap();
    std::os::unix::fs::symlink(&target, codex_home.path().join("config.toml")).unwrap();
    assert!(matches!(
        add(codex_home.path(), &codex(), None),
        Err(ConnectorError::Refused(_))
    ));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "model = \"x\"\n");
}

#[test]
fn a_part_outside_the_folder_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let plan = ConnectorPlan {
        parts: vec![ConnectorPart::File {
            path: PathBuf::from("../escape.json"),
            text: "{}".into(),
        }],
        picked_up: String::new(),
        guide: juggler_adapter::inventory::GetIt::default(),
    };
    assert!(matches!(
        add(root.path(), &plan, None),
        Err(ConnectorError::Refused(_))
    ));
    assert!(!root.path().parent().unwrap().join("escape.json").exists());
    assert!(inside(root.path(), Path::new("/etc/hosts")).is_err());
}

const CODEX_WITH_SERVERS: &str = r#"# My Codex settings
model = "gpt-5.5"   # the one I like

[projects."/Users/u/code/app"]
trust_level = "trusted"

[mcp_servers.github]
command = "github-mcp"  # keep this comment
args = ["--stdio"]

[hooks.state."hooks.json:session_start:0:0"]
trusted_hash = "abc"
"#;

#[test]
fn codex_config_with_servers_and_comments_merges_one_table_and_unmerges_byte_for_byte() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    std::fs::write(&config, CODEX_WITH_SERVERS).unwrap();
    let plan = codex();
    let shown = preview(home.path(), &plan, None).unwrap();
    assert_eq!(shown[0].action, Action::Merge);
    assert_eq!(shown[0].before, None, "the user's own file is never shown");
    assert!(
        !shown[0].after.contains("github"),
        "only AI Juggler's table"
    );
    let record = add(home.path(), &plan, None).unwrap();
    let merged = std::fs::read_to_string(&config).unwrap();
    assert!(
        merged.contains(shown[0].after.trim()),
        "the preview is what's added:\n{merged}"
    );
    let again = preview(home.path(), &plan, Some(&record)).unwrap();
    assert_eq!(again[0].action, Action::Same);
    assert_eq!(again[0].before.as_deref(), Some(shown[0].after.as_str()));
    assert!(merged.contains("command = \"github-mcp\"  # keep this comment"));
    assert!(merged.starts_with("# My Codex settings\nmodel = \"gpt-5.5\"   # the one I like\n"));
    let parsed: toml::Table = merged.parse().unwrap();
    assert_eq!(
        parsed["mcp_servers"]["juggler"]["tools"]["whoami"]["approval_mode"].as_str(),
        Some("approve")
    );
    assert!(parsed["mcp_servers"].get("github").is_some());
    assert_eq!(
        status(home.path(), &plan, Some(&record)).state,
        ConnectorState::Current
    );
    remove(home.path(), &record).unwrap();
    assert_eq!(
        std::fs::read_to_string(&config).unwrap(),
        CODEX_WITH_SERVERS
    );
}

#[test]
fn codex_config_without_servers_or_without_a_file_comes_back_as_it_was() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    let plain =
        "# just a model\nmodel = \"gpt-5.5\"\n\n[projects.\"/w\"]\ntrust_level = \"trusted\"\n";
    std::fs::write(&config, plain).unwrap();
    let record = add(home.path(), &codex(), None).unwrap();
    assert!(
        std::fs::read_to_string(&config)
            .unwrap()
            .contains("[mcp_servers.juggler]")
    );
    remove(home.path(), &record).unwrap();
    assert_eq!(std::fs::read_to_string(&config).unwrap(), plain);

    let empty = tempfile::tempdir().unwrap();
    let record = add(empty.path(), &codex(), None).unwrap();
    assert!(matches!(
        &record.parts[0],
        RecordedPart::TomlTable {
            created_file: true,
            ..
        }
    ));
    remove(empty.path(), &record).unwrap();
    assert!(
        !empty.path().join("config.toml").exists(),
        "the file Add made is gone"
    );
}

#[test]
fn codex_keeps_a_users_juggler_server_and_a_table_they_changed() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    let theirs = "[mcp_servers.juggler]\ncommand = \"/opt/my-juggler\"\n";
    std::fs::write(&config, theirs).unwrap();
    assert!(matches!(
        add(home.path(), &codex(), None),
        Err(ConnectorError::Refused(_))
    ));
    assert_eq!(std::fs::read_to_string(&config).unwrap(), theirs);

    std::fs::write(&config, CODEX_WITH_SERVERS).unwrap();
    let record = add(home.path(), &codex(), None).unwrap();
    let changed = std::fs::read_to_string(&config).unwrap().replace(
        "default_tools_approval_mode = \"prompt\"",
        "default_tools_approval_mode = \"approve\"",
    );
    std::fs::write(&config, &changed).unwrap();
    assert_eq!(
        status(home.path(), &codex(), Some(&record)).state,
        ConnectorState::Changed
    );
    let removal = remove(home.path(), &record).unwrap();
    assert_eq!(removal.kept, [PathBuf::from("config.toml")]);
    assert_eq!(std::fs::read_to_string(&config).unwrap(), changed);
}

#[test]
fn codex_reformatting_our_table_isnt_a_change() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    std::fs::write(&config, CODEX_WITH_SERVERS).unwrap();
    let record = add(home.path(), &codex(), None).unwrap();
    // The tool rewrites the file in its own layout: the same values.
    let text = std::fs::read_to_string(&config).unwrap();
    let relaid: toml::Table = text.parse().unwrap();
    std::fs::write(&config, toml::to_string(&relaid).unwrap()).unwrap();
    assert_eq!(
        status(home.path(), &codex(), Some(&record)).state,
        ConnectorState::Current
    );
    let removal = remove(home.path(), &record).unwrap();
    assert_eq!(removal.removed, [PathBuf::from("config.toml")]);
    let rest: toml::Table = std::fs::read_to_string(&config).unwrap().parse().unwrap();
    assert!(rest["mcp_servers"].get("juggler").is_none());
    assert!(rest["mcp_servers"].get("github").is_some());
}

#[test]
fn inventories_list_the_connector_as_ai_jugglers_never_a_skill_or_server_to_copy() {
    use juggler_adapter::inventory::{ItemKind, SetUp};
    let root = claude_folder();
    add(root.path(), &claude("0.4.6"), None).unwrap();
    let items =
        crate::claude_inventory::inventory(root.path(), &root.path().join(".claude.json"), None);
    let connector: Vec<_> = items.iter().filter(|item| is_connector(item)).collect();
    assert_eq!(connector.len(), 1, "{items:?}");
    assert!(
        !items
            .iter()
            .any(|item| item.kind == ItemKind::Skill && item.name == "ai-juggler"),
        "never a skill"
    );
    assert!(
        items
            .iter()
            .any(|item| item.kind == ItemKind::Skill && item.name == "release-notes"),
        "the user's own skill is still theirs"
    );
    for verdict in [
        crate::claude_inventory::can_set_up(connector[0]),
        crate::codex_inventory::can_set_up(connector[0]),
    ] {
        assert!(
            matches!(
                verdict,
                SetUp::TurnOn {
                    can_copy: false,
                    ..
                }
            ),
            "{verdict:?}"
        );
    }

    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("config.toml"), CODEX_WITH_SERVERS).unwrap();
    add(home.path(), &codex(), None).unwrap();
    let items = crate::codex_inventory::inventory(home.path(), None);
    assert_eq!(items.iter().filter(|item| is_connector(item)).count(), 1);
    assert!(
        !items
            .iter()
            .any(|item| item.kind == ItemKind::McpServer && item.name == "juggler"),
        "never a server to copy"
    );
    assert!(
        items
            .iter()
            .any(|item| item.kind == ItemKind::McpServer && item.name == "github")
    );
}
