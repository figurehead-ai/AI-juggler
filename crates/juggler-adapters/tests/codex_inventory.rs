//! AJ-E1542's Codex inventory, at the adapter (AJ-E1522: adapter
//! coverage): what a Codex home has set up, read from its own files in a
//! temp folder; whether each item can be set up in Codex; and setting it
//! up — an MCP server appended to `config.toml` (after a backup, never
//! replacing one there), files copied as new files, a Claude command as a
//! skill, settings merged key by key. Nothing of the user's is read.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::inventory::{
    ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, SettingCheck,
    Undo, Value,
};
use juggler_adapters::codex_inventory::{can_set_up, inventory, set_up};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A Codex home (and the user's home, for `~/.agents/skills`) with one of
/// everything the inventory reads.
fn home(root: &Path) -> (PathBuf, PathBuf) {
    let home = root.join("codex-a");
    let user = root.join("user");
    write(
        &home.join("config.toml"),
        r#"model = "gpt-6"
approval_policy = "on-request"
sandbox_mode = "workspace-write"

[tui]
status_line = ["model", "context"]

[mcp_servers.github]
command = "npx"
args = ["-y", "gh-mcp"]
env = { LOG = "info" }
env_vars = ["GITHUB_TOKEN"]

[mcp_servers.docs]
url = "https://docs.example.com/mcp"
bearer_token_env_var = "DOCS_TOKEN"
http_headers = { "X-Team" = "core" }
env_http_headers = { "X-Key" = "DOCS_KEY" }

[mcp_servers.linear]
url = "https://mcp.linear.example/mcp"
auth = "oauth"

[mcp_servers.off]
command = "x"
enabled = false

[plugins."lint@market"]
enabled = true

[plugins."old@market"]
enabled = false
"#,
    );
    write(&user.join(".agents/skills/pdf/SKILL.md"), "# PDF\n");
    write(
        &home.join("skills/pdf/SKILL.md"),
        "# PDF, the home's copy\n",
    );
    write(&home.join("skills/notes/SKILL.md"), "# Notes\n");
    write(&home.join("skills/empty/readme.txt"), "no SKILL.md");
    write(&home.join("prompts/fix.md"), "Fix $1\n");
    write(&home.join("prompts/draft.txt"), "not a prompt");
    write(&home.join("agents/reviewer.toml"), "name = \"reviewer\"\n");
    write(&home.join("AGENTS.md"), "Use spaces.\n");
    write(&home.join("AGENTS.override.md"), "Use tabs.\n");
    (home, user)
}

fn find<'a>(items: &'a [Item], kind: ItemKind, name: &str) -> &'a Item {
    items
        .iter()
        .find(|item| item.kind == kind && item.name == name)
        .unwrap_or_else(|| panic!("no {kind:?} {name}: {items:#?}"))
}

#[test]
fn a_codex_home_lists_each_kind_once() {
    let root = tempfile::tempdir().unwrap();
    let (home, user) = home(root.path());
    let items = inventory(&home, Some(&user));

    let Detail::Mcp(github) = &find(&items, ItemKind::McpServer, "github").detail else {
        panic!()
    };
    assert_eq!(
        github.transport,
        McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "gh-mcp".into()],
            env: vec![
                ("LOG".into(), Value::Literal("info".into())),
                ("GITHUB_TOKEN".into(), Value::Env("GITHUB_TOKEN".into())),
            ],
        }
    );
    let Detail::Mcp(docs) = &find(&items, ItemKind::McpServer, "docs").detail else {
        panic!()
    };
    let McpTransport::Http { headers, .. } = &docs.transport else {
        panic!()
    };
    assert!(headers.contains(&("X-Team".into(), Value::Literal("core".into()))));
    assert!(headers.contains(&("Authorization".into(), Value::Env("DOCS_TOKEN".into()))));
    assert!(headers.contains(&("X-Key".into(), Value::Env("DOCS_KEY".into()))));
    let Detail::Mcp(McpServer { oauth, .. }) = &find(&items, ItemKind::McpServer, "linear").detail
    else {
        panic!()
    };
    assert!(oauth);
    assert!(
        !items
            .iter()
            .any(|item| item.name == "off" || item.name == "old@market")
    );
    find(&items, ItemKind::Plugin, "lint@market");

    // The shared skills folder first; the home's copy of the same skill
    // isn't listed twice.
    let Detail::Files { path } = &find(&items, ItemKind::Skill, "pdf").detail else {
        panic!()
    };
    assert!(path.starts_with(&user));
    assert_eq!(items.iter().filter(|item| item.name == "pdf").count(), 1);
    find(&items, ItemKind::Skill, "notes");
    assert!(!items.iter().any(|item| item.name == "empty"));
    find(&items, ItemKind::Command, "fix");
    assert!(!items.iter().any(|item| item.name == "draft"));
    find(&items, ItemKind::Subagent, "reviewer");
    // AGENTS.override.md is the one Codex reads.
    let Detail::Files { path } = &find(&items, ItemKind::MainRules, "AGENTS.md").detail else {
        panic!()
    };
    assert!(path.ends_with("AGENTS.override.md"));

    let Detail::Setting { about, check, .. } =
        &find(&items, ItemKind::Permissions, "permissions").detail
    else {
        panic!()
    };
    assert_eq!(
        about.as_deref(),
        Some("approval_policy on-request, sandbox_mode workspace-write")
    );
    assert_eq!(check, &SettingCheck::Fine);
    find(&items, ItemKind::StatusLine, "status line");
}

#[test]
fn an_empty_or_broken_home_lists_nothing() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("codex");
    std::fs::create_dir_all(&home).unwrap();
    assert!(inventory(&home, None).is_empty());
    write(&home.join("config.toml"), "not = [toml");
    assert!(inventory(&home, None).is_empty());
}

#[test]
fn no_sandbox_is_offered_never_ticked_and_a_secret_setting_never_kept() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("codex");
    write(
        &home.join("config.toml"),
        "sandbox_mode = \"danger-full-access\"\n",
    );
    let items = inventory(&home, None);
    let permissions = find(&items, ItemKind::Permissions, "permissions");
    assert!(
        matches!(can_set_up(permissions), SetUp::NeedsYou(note) if note.contains("danger-full-access"))
    );

    write(
        &home.join("config.toml"),
        "[tui]\nstatus_line = [\"ghp_0123456789abcdefghij0123456789abcd\"]\n",
    );
    let items = inventory(&home, None);
    let status = find(&items, ItemKind::StatusLine, "status line");
    let Detail::Setting { entries, .. } = &status.detail else {
        panic!()
    };
    assert!(entries.is_empty());
    assert!(matches!(can_set_up(status), SetUp::Impossible(note) if note.contains("secret")));
}

fn claude_item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: "claude-code".into(),
        kind,
        name: name.into(),
        scope: Scope::User,
        detail,
    }
}

#[test]
fn each_kind_says_whether_codex_can_set_it_up() {
    let root = tempfile::tempdir().unwrap();
    let (home, user) = home(root.path());
    let items = inventory(&home, Some(&user));
    let verdict = |kind, name| can_set_up(find(&items, kind, name));
    assert_eq!(verdict(ItemKind::McpServer, "github"), SetUp::Copy);
    assert!(
        matches!(verdict(ItemKind::McpServer, "linear"), SetUp::NeedsYou(note) if note.contains("codex mcp login linear"))
    );
    assert_eq!(verdict(ItemKind::Skill, "pdf"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::Command, "fix"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::Subagent, "reviewer"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::MainRules, "AGENTS.md"), SetUp::Copy);
    assert!(
        matches!(verdict(ItemKind::Plugin, "lint@market"), SetUp::NeedsYou(note) if note.contains("codex plugin add lint@market"))
    );
    assert_eq!(verdict(ItemKind::Permissions, "permissions"), SetUp::Copy);

    // From Claude Code.
    let file = Detail::Files {
        path: home.join("prompts/fix.md"),
    };
    let stdio = |scope| Item {
        scope,
        ..claude_item(
            ItemKind::McpServer,
            "db",
            Detail::Mcp(McpServer {
                transport: McpTransport::Stdio {
                    command: "db".into(),
                    args: vec![],
                    env: vec![],
                },
                oauth: false,
            }),
        )
    };
    assert_eq!(can_set_up(&stdio(Scope::User)), SetUp::Copy);
    assert!(
        matches!(can_set_up(&stdio(Scope::ProjectPrivate)), SetUp::Translate(note) if note.contains("every project"))
    );
    let sse = claude_item(
        ItemKind::McpServer,
        "events",
        Detail::Mcp(McpServer {
            transport: McpTransport::Other {
                kind: "sse".into(),
                url: None,
            },
            oauth: false,
        }),
    );
    assert!(matches!(can_set_up(&sse), SetUp::Impossible(note) if note.contains("sse")));
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::Skill, "s", file.clone())), SetUp::Translate(note) if note.contains("~/.agents/skills"))
    );
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::Command, "c", file.clone())), SetUp::Translate(note) if note.contains("skill"))
    );
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::MainRules, "CLAUDE.md", file.clone())), SetUp::Translate(note) if note.contains("AGENTS.md"))
    );
    assert!(matches!(
        can_set_up(&claude_item(ItemKind::Rules, "rules/x.md", file.clone())),
        SetUp::NeedsYou(_)
    ));
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::Subagent, "a", file)), SetUp::Impossible(note) if note.contains("no Codex equivalent"))
    );
    assert!(matches!(
        can_set_up(&claude_item(
            ItemKind::Plugin,
            "p",
            Detail::Plugin { id: "p".into() }
        )),
        SetUp::Impossible(_)
    ));
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::Skill, "pdf", Detail::FromTheAccount)), SetUp::Impossible(note) if note.contains("claude.ai"))
    );
    let setting = Detail::Setting {
        file: home.join("settings.json"),
        format: ConfigFormat::Json,
        entries: Vec::new(),
        about: None,
        check: SettingCheck::Fine,
    };
    assert!(
        matches!(can_set_up(&claude_item(ItemKind::Permissions, "permissions", setting)), SetUp::Impossible(note) if note.contains("work differently"))
    );
}

#[test]
fn an_mcp_server_is_appended_after_a_backup_and_never_replaces_one() {
    let root = tempfile::tempdir().unwrap();
    let (from, user) = home(root.path());
    let items = inventory(&from, Some(&user));
    let to = root.path().join("codex-b");
    write(&to.join("config.toml"), "model = \"gpt-6\"");

    let applied = set_up(&to, Some(&user), find(&items, ItemKind::McpServer, "docs")).unwrap();
    let text = std::fs::read_to_string(to.join("config.toml")).unwrap();
    let parsed: toml::Table = text.parse().unwrap();
    let docs = &parsed["mcp_servers"]["docs"];
    assert_eq!(docs["url"].as_str(), Some("https://docs.example.com/mcp"));
    assert_eq!(docs["bearer_token_env_var"].as_str(), Some("DOCS_TOKEN"));
    assert_eq!(docs["http_headers"]["X-Team"].as_str(), Some("core"));
    assert_eq!(docs["env_http_headers"]["X-Key"].as_str(), Some("DOCS_KEY"));
    assert!(
        text.starts_with("model = \"gpt-6\"\n"),
        "what was there is unchanged"
    );
    assert_eq!(
        std::fs::read_to_string(to.join("config.toml.before-ai-juggler")).unwrap(),
        "model = \"gpt-6\""
    );
    let Undo::RemoveAppended {
        text: appended,
        backup,
        ..
    } = applied.undo
    else {
        panic!()
    };
    assert!(text.ends_with(&appended) && backup.is_some());

    set_up(
        &to,
        Some(&user),
        find(&items, ItemKind::McpServer, "github"),
    )
    .unwrap();
    let parsed: toml::Table = std::fs::read_to_string(to.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let github = &parsed["mcp_servers"]["github"];
    assert_eq!(github["env"]["LOG"].as_str(), Some("info"));
    assert_eq!(github["env_vars"][0].as_str(), Some("GITHUB_TOKEN"));

    // The same name again: refused, nothing replaced.
    let error = set_up(&to, Some(&user), find(&items, ItemKind::McpServer, "docs")).unwrap_err();
    assert!(error.to_string().contains("already has"), "{error}");

    // Into a home with no config.toml yet: written new, no backup.
    let fresh = root.path().join("codex-c");
    std::fs::create_dir_all(&fresh).unwrap();
    set_up(
        &fresh,
        Some(&user),
        find(&items, ItemKind::McpServer, "github"),
    )
    .unwrap();
    assert!(fresh.join("config.toml").is_file());
    assert!(!fresh.join("config.toml.before-ai-juggler").exists());

    // A Claude server with a secret header becomes a forwarded variable.
    let secret = claude_item(
        ItemKind::McpServer,
        "search",
        Detail::Mcp(McpServer {
            transport: McpTransport::Http {
                url: "https://search.example/mcp".into(),
                headers: vec![("X-Api".into(), Value::Env("SEARCH_KEY".into()))],
            },
            oauth: false,
        }),
    );
    set_up(&fresh, Some(&user), &secret).unwrap();
    let parsed: toml::Table = std::fs::read_to_string(fresh.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        parsed["mcp_servers"]["search"]["env_http_headers"]["X-Api"].as_str(),
        Some("SEARCH_KEY")
    );
}

#[test]
fn files_and_settings_are_set_up_as_new_files_and_keys() {
    let root = tempfile::tempdir().unwrap();
    let (from, user) = home(root.path());
    let items = inventory(&from, Some(&user));
    let to = root.path().join("codex-b");
    std::fs::create_dir_all(&to).unwrap();
    let other_user = root.path().join("user-b");

    // A skill goes to the shared folder.
    set_up(
        &to,
        Some(&other_user),
        find(&items, ItemKind::Skill, "notes"),
    )
    .unwrap();
    assert!(other_user.join(".agents/skills/notes/SKILL.md").is_file());
    assert!(
        set_up(&to, None, find(&items, ItemKind::Skill, "notes")).is_err(),
        "no home folder"
    );

    set_up(&to, None, find(&items, ItemKind::Command, "fix")).unwrap();
    assert_eq!(
        std::fs::read_to_string(to.join("prompts/fix.md")).unwrap(),
        "Fix $1\n"
    );
    set_up(&to, None, find(&items, ItemKind::Subagent, "reviewer")).unwrap();
    assert!(to.join("agents/reviewer.toml").is_file());
    set_up(&to, None, find(&items, ItemKind::MainRules, "AGENTS.md")).unwrap();
    assert_eq!(
        std::fs::read_to_string(to.join("AGENTS.md")).unwrap(),
        "Use tabs.\n"
    );
    assert!(
        set_up(&to, None, find(&items, ItemKind::MainRules, "AGENTS.md")).is_err(),
        "never over a file"
    );

    let applied = set_up(
        &to,
        None,
        find(&items, ItemKind::Permissions, "permissions"),
    )
    .unwrap();
    let parsed: toml::Table = std::fs::read_to_string(to.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(parsed["sandbox_mode"].as_str(), Some("workspace-write"));
    assert!(matches!(applied.undo, Undo::RemoveSetting { .. }));
    set_up(&to, None, find(&items, ItemKind::StatusLine, "status line")).unwrap();

    // A Claude command becomes a skill named after it.
    let command = root.path().join("claude/commands/ship.md");
    write(
        &command,
        "---\nallowed-tools: Bash\n---\n# Ship the \"release\"\nRun it.\n",
    );
    set_up(
        &to,
        Some(&other_user),
        &claude_item(ItemKind::Command, "ship", Detail::Files { path: command }),
    )
    .unwrap();
    let skill = std::fs::read_to_string(other_user.join(".agents/skills/ship/SKILL.md")).unwrap();
    assert!(
        skill.starts_with("---\nname: ship\ndescription: \"Ship the 'release'\"\n---\n"),
        "{skill}"
    );
    assert!(skill.contains("Run it."));

    // What can't be set up says why.
    assert!(set_up(&to, None, find(&items, ItemKind::Plugin, "lint@market")).is_err());
}
