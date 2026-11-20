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

/// The lab's A11 (2026-11-15): Remove left a changed file and took the
/// rest, half-removing the connector. Now it's all or nothing: a changed
/// file refuses it, naming the file, with nothing taken out; Remove anyway
/// (the person's confirmed choice) takes it all out, that file too.
#[test]
fn remove_refuses_naming_a_changed_file_and_remove_anyway_takes_it_all_out() {
    let root = claude_folder();
    let before = (snapshot(root.path()), dirs(root.path()));
    let plan = claude("0.4.6");
    let record = add(root.path(), &plan, None).unwrap();
    let skill = root
        .path()
        .join("skills/ai-juggler/skills/pool-tasks/SKILL.md");
    std::fs::write(&skill, "my own notes\n").unwrap();
    let found = status(root.path(), &plan, Some(&record));
    assert_eq!(found.state, ConnectorState::Changed);
    let with_change = snapshot(root.path());
    let refused = remove(root.path(), &record);
    let Err(ConnectorError::Refused(why)) = &refused else {
        unreachable!("{refused:?}");
    };
    assert!(
        why.contains("skills/ai-juggler/skills/pool-tasks/SKILL.md"),
        "{why}"
    );
    assert!(why.contains("Remove anyway"), "{why}");
    assert_eq!(snapshot(root.path()), with_change, "nothing taken out");
    let removal = remove_anyway(root.path(), &record).unwrap();
    assert_eq!(removal.removed.len(), record.parts.len());
    assert!(removal.kept.is_empty() && removal.left.is_none());
    assert_eq!(
        (snapshot(root.path()), dirs(root.path())),
        before,
        "the folder as it was before Add, the changed file gone too"
    );
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
fn codex_keeps_a_users_juggler_server_and_refuses_removing_a_table_they_changed() {
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
    // A11: all or nothing; Remove anyway unmerges the table as it is now,
    // the rest of the user's file untouched.
    assert!(matches!(
        remove(home.path(), &record),
        Err(ConnectorError::Refused(_))
    ));
    assert_eq!(std::fs::read_to_string(&config).unwrap(), changed);
    remove_anyway(home.path(), &record).unwrap();
    let after = std::fs::read_to_string(&config).unwrap();
    assert!(!after.contains("[mcp_servers.juggler]"), "{after}");
    assert_eq!(after, CODEX_WITH_SERVERS, "the user's own file, as it was");
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
    // Each verdict is checked, never printed: a verdict's note can be
    // an inline secret's (`inline_secret_verdict`).
    for (inventory, verdict) in [
        (
            "claude_inventory",
            crate::claude_inventory::can_set_up(connector[0]),
        ),
        (
            "codex_inventory",
            crate::codex_inventory::can_set_up(connector[0]),
        ),
    ] {
        assert!(
            matches!(
                verdict,
                SetUp::TurnOn {
                    can_copy: false,
                    ..
                }
            ),
            "{inventory}: the connector is turned on, never copied"
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

/// L6 (round 2): a link or a folder where Add writes a file first isn't
/// AI Juggler's leftover: it's never removed, and Add is refused naming
/// it, with nothing written; a plain leftover file there is taken away.
#[cfg(unix)]
#[test]
fn something_not_ours_at_the_temporary_name_is_named_and_never_removed() {
    let root = claude_folder();
    let old = add(root.path(), &claude("0.4.5"), None).unwrap();
    let plan = claude("0.4.6");
    let replaced = preview_of(root.path(), &plan, &old)
        .into_iter()
        .find(|file| file.action == Action::Replace)
        .expect("a file the new edition replaces")
        .path;
    let file = root.path().join(&replaced);
    let temp = file.with_file_name(format!(
        ".{}.ai-juggler-new",
        file.file_name().unwrap().to_string_lossy()
    ));
    let elsewhere = root.path().join("elsewhere");
    std::fs::write(&elsewhere, "keep").unwrap();
    std::os::unix::fs::symlink(&elsewhere, &temp).unwrap();
    let before = snapshot(root.path());
    let refused = add(root.path(), &plan, Some(&old));
    let Err(ConnectorError::Refused(why)) = &refused else {
        unreachable!("{refused:?}");
    };
    assert!(why.contains(&temp.display().to_string()), "{why}");
    assert!(why.contains("nothing was written"), "{why}");
    assert_eq!(snapshot(root.path()), before, "every file as it was");
    assert!(std::fs::symlink_metadata(&temp).is_ok(), "the link kept");
    assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "keep");
    std::fs::remove_file(&temp).unwrap();
    std::fs::create_dir(&temp).unwrap();
    assert!(matches!(
        add(root.path(), &plan, Some(&old)),
        Err(ConnectorError::Refused(_))
    ));
    assert!(temp.is_dir(), "the folder kept");
    std::fs::remove_dir(&temp).unwrap();
    std::fs::write(&temp, "half").unwrap();
    let new = add(root.path(), &plan, Some(&old)).unwrap();
    assert!(!temp.exists(), "a plain leftover file is ours: taken away");
    assert_eq!(
        status(root.path(), &plan, Some(&new)).state,
        ConnectorState::Current
    );
}

/// L6: Add is all or nothing: a failure part-way puts back every file it
/// replaced, takes away every file and folder it made, and leaves no
/// temporary file behind; a later Add then goes through.
#[cfg(unix)]
#[test]
fn a_failure_part_way_takes_back_everything_add_wrote() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let file = |path: &str, text: String| ConnectorPart::File {
        path: PathBuf::from(path),
        text,
    };
    let plan = |version: &str, with_two: bool| ConnectorPlan {
        parts: [
            Some(file("a/one", format!("one {version}"))),
            with_two.then(|| file("b/two", format!("two {version}"))),
            Some(file("c/three", format!("three {version}"))),
        ]
        .into_iter()
        .flatten()
        .collect(),
        picked_up: String::new(),
        guide: juggler_adapter::inventory::GetIt::default(),
    };
    let first = add(root.path(), &plan("v1", false), None).unwrap();
    let before = snapshot(root.path());
    // The third part's folder can't be written: its replacement fails
    // after the first was replaced and the second made.
    let locked = root.path().join("c");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();
    let failed = add(root.path(), &plan("v2", true), Some(&first));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(failed, Err(ConnectorError::Io(_))), "{failed:?}");
    assert_eq!(snapshot(root.path()), before, "every file as it was");
    assert!(
        !root.path().join("b").exists(),
        "the folder it made is gone"
    );
    assert!(
        !snapshot(root.path())
            .iter()
            .any(|(path, _)| path.to_string_lossy().ends_with(".ai-juggler-new")),
        "no temporary file left"
    );
    let second = add(root.path(), &plan("v2", true), Some(&first)).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join("c/three")).unwrap(),
        "three v2"
    );
    remove(root.path(), &second).unwrap();
    assert!(snapshot(root.path()).is_empty(), "Remove takes it all out");
}
