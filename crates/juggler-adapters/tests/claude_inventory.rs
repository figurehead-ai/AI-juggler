//! AJ-E1542's Claude Code inventory, at the adapter (AJ-E1522: adapter
//! coverage): what an account has set up, read from its own files in a
//! temp folder; whether each item can be set up in a Claude account; and
//! setting it up — files copied as new files, settings merged key by key,
//! an MCP server added through `claude mcp add-json` (a stub here that
//! records how it was called). Nothing of the user's is read.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::inventory::{
    Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo, Value,
};
use juggler_adapters::claude_inventory::{can_set_up, inventory, set_up};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A Claude account folder with one of everything the inventory reads.
fn account(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let dir = root.join("claude-a");
    let project = root.join("work/app");
    std::fs::create_dir_all(&project).unwrap();
    let state = dir.join(".claude.json");
    write(
        &state,
        &serde_json::json!({
            "mcpServers": {
                "files": { "command": "npx", "args": ["-y", "server-files"], "env": { "ROOT": "/w" } },
                "search": { "type": "http", "url": "https://search.example/mcp",
                            "headers": { "Authorization": "Bearer sk-live-0123456789abcdef0123" } },
                "linear": { "type": "http", "url": "https://mcp.linear.example/mcp", "oauth": {} },
                "events": { "type": "sse", "url": "https://events.example/sse" },
                "keyed": { "command": "run-keyed", "env": { "API_KEY": "${MY_KEY}" } }
            },
            "projects": {
                project.to_string_lossy(): { "mcpServers": { "db": { "command": "db-mcp" } } },
                "/elsewhere": { "mcpServers": { "other": { "command": "x" } } }
            },
            "oauthAccount": { "emailAddress": "never-read@example.com" }
        })
        .to_string(),
    );
    write(&dir.join("skills/review/SKILL.md"), "# Review\n");
    write(
        &dir.join("skills/not-a-skill/notes.txt"),
        "no SKILL.md here",
    );
    // claude.ai's synced skills: the same skill under two sync ids, and
    // one the account also has as its own files.
    write(&dir.join("skills/synced/set-1/pdf/SKILL.md"), "# PDF\n");
    write(&dir.join("skills/synced/set-2/pdf/SKILL.md"), "# PDF\n");
    write(
        &dir.join("skills/synced/set-2/review/SKILL.md"),
        "# Review\n",
    );
    write(&dir.join("commands/ship.md"), "Ship it: $ARGUMENTS\n");
    write(&dir.join("commands/readme.txt"), "not a command");
    write(
        &dir.join("agents/tester.md"),
        "---\nname: tester\n---\nWrite tests.\n",
    );
    write(&dir.join("CLAUDE.md"), "Use tabs.\n");
    write(&dir.join("rules/style.md"), "Short names.\n");
    write(&dir.join("rules/draft.txt"), "not a rule");
    write(
        &dir.join("settings.json"),
        &serde_json::json!({
            "enabledPlugins": {
                "lint@market": true,
                "off@market": false,
                "notes@synced": true
            },
            "permissions": {
                "allow": ["Bash(npm test)", "Bash(curl -H 'Authorization: Bearer sk-live-0123456789abcdef0123' x)"],
                "deny": ["Read(.env)"],
                "ask": ["Bash(git push)"],
                "defaultMode": "acceptEdits",
                "additionalDirectories": ["../shared"]
            },
            "statusLine": { "type": "command", "command": "~/bin/my-status" },
            "env": { "NEVER_READ": "x" }
        })
        .to_string(),
    );
    (dir, state, project)
}

fn find<'a>(items: &'a [Item], kind: ItemKind, name: &str) -> &'a Item {
    items
        .iter()
        .find(|item| item.kind == kind && item.name == name)
        .unwrap_or_else(|| panic!("no {kind:?} {name}: {items:#?}"))
}

#[test]
fn the_inventory_lists_what_the_account_has_and_nothing_else() {
    let root = tempfile::tempdir().unwrap();
    let (dir, state, project) = account(root.path());
    let items = inventory(&dir, &state, Some(&project));

    // MCP servers: the user's, and this project's own (not another's).
    for name in ["files", "search", "linear", "events", "keyed"] {
        assert_eq!(find(&items, ItemKind::McpServer, name).scope, Scope::User);
    }
    assert_eq!(
        find(&items, ItemKind::McpServer, "db").scope,
        Scope::ProjectPrivate
    );
    assert!(!items.iter().any(|item| item.name == "other"));
    let Detail::Mcp(McpServer { transport, oauth }) =
        &find(&items, ItemKind::McpServer, "files").detail
    else {
        panic!()
    };
    assert!(!oauth);
    assert_eq!(
        transport,
        &McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "server-files".into()],
            env: vec![("ROOT".into(), Value::Literal("/w".into()))],
        }
    );
    let Detail::Mcp(search) = &find(&items, ItemKind::McpServer, "search").detail else {
        panic!()
    };
    let McpTransport::Http { headers, .. } = &search.transport else {
        panic!()
    };
    assert_eq!(headers[0].1, Value::Secret, "the token is never kept");
    let Detail::Mcp(events) = &find(&items, ItemKind::McpServer, "events").detail else {
        panic!()
    };
    assert!(matches!(&events.transport, McpTransport::Other { kind, .. } if kind == "sse"));

    // Skills: its own, and claude.ai's synced ones once each, minus one it
    // also has as files.
    assert!(matches!(
        find(&items, ItemKind::Skill, "review").detail,
        Detail::Files { .. }
    ));
    assert_eq!(
        find(&items, ItemKind::Skill, "pdf").detail,
        Detail::FromTheAccount
    );
    assert_eq!(items.iter().filter(|item| item.name == "pdf").count(), 1);
    assert_eq!(items.iter().filter(|item| item.name == "review").count(), 1);
    assert!(!items.iter().any(|item| item.name == "not-a-skill"));

    // Commands, subagents, rules: Markdown only.
    find(&items, ItemKind::Command, "ship");
    find(&items, ItemKind::Subagent, "tester");
    find(&items, ItemKind::MainRules, "CLAUDE.md");
    find(&items, ItemKind::Rules, "rules/style.md");
    assert!(
        !items
            .iter()
            .any(|item| item.name.contains("readme") || item.name.contains("draft"))
    );

    // Plugins: enabled ones; a synced one comes with the account.
    assert_eq!(
        find(&items, ItemKind::Plugin, "lint@market").detail,
        Detail::Plugin {
            id: "lint@market".into()
        }
    );
    assert_eq!(
        find(&items, ItemKind::Plugin, "notes@synced").detail,
        Detail::FromTheAccount
    );
    assert!(!items.iter().any(|item| item.name == "off@market"));

    // Permissions in words, the rule with a secret left out of the copy.
    let Detail::Setting { about, entries, .. } =
        &find(&items, ItemKind::Permissions, "permissions").detail
    else {
        panic!()
    };
    let about = about.as_deref().unwrap();
    for words in [
        "1 allowed",
        "1 denied",
        "1 asked first",
        "mode acceptEdits",
        "1 more folders",
        "1 rule with a secret left out",
    ] {
        assert!(about.contains(words), "{about}");
    }
    let copied: String = entries.iter().map(|(_, value)| value.as_str()).collect();
    assert!(
        copied.contains("npm test") && !copied.contains("sk-live"),
        "{copied}"
    );

    // The user's own status line, described by what it runs.
    let Detail::Setting { about, .. } = &find(&items, ItemKind::StatusLine, "status line").detail
    else {
        panic!()
    };
    assert_eq!(about.as_deref(), Some("runs ~/bin/my-status"));

    // Nothing else of settings.json or the state file is kept.
    let all = format!("{items:?}");
    for never in ["NEVER_READ", "never-read@example.com", "sk-live"] {
        assert!(!all.contains(never), "{never}");
    }
}

#[test]
fn an_account_with_nothing_set_up_lists_nothing() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("empty");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(inventory(&dir, &dir.join(".claude.json"), None).is_empty());
    // A broken state file or settings file is read as nothing.
    write(&dir.join(".claude.json"), "{ not json");
    write(&dir.join("settings.json"), "{ also not json");
    assert!(inventory(&dir, &dir.join(".claude.json"), None).is_empty());
}

#[test]
fn bypassing_permissions_is_offered_never_ticked_and_secret_only_rules_stay() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("a");
    write(
        &dir.join("settings.json"),
        r#"{ "permissions": { "defaultMode": "bypassPermissions" } }"#,
    );
    let items = inventory(&dir, &dir.join(".claude.json"), None);
    let permissions = find(&items, ItemKind::Permissions, "permissions");
    assert!(
        matches!(can_set_up(permissions), SetUp::NeedsYou(note) if note.contains("bypassPermissions"))
    );

    write(
        &dir.join("settings.json"),
        r#"{ "permissions": { "allow": ["Bash(curl -H 'Authorization: Bearer sk-live-0123456789abcdef0123' x)", "Bash(export TOKEN=ghp_0123456789abcdefghij0123456789abcd)"] } }"#,
    );
    let items = inventory(&dir, &dir.join(".claude.json"), None);
    let permissions = find(&items, ItemKind::Permissions, "permissions");
    assert!(matches!(can_set_up(permissions), SetUp::Impossible(note) if note.contains("secret")));
    let Detail::Setting { about, .. } = &permissions.detail else {
        panic!()
    };
    assert_eq!(about.as_deref(), Some("2 rules with a secret left out"));

    // Empty permissions are nothing to list.
    write(&dir.join("settings.json"), r#"{ "permissions": { } }"#);
    assert!(inventory(&dir, &dir.join(".claude.json"), None).is_empty());
}

#[test]
fn a_status_line_with_a_secret_is_never_kept() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("a");
    write(
        &dir.join("settings.json"),
        r#"{ "statusLine": { "type": "command", "command": "status --token ghp_0123456789abcdefghij0123456789abcd" } }"#,
    );
    let items = inventory(&dir, &dir.join(".claude.json"), None);
    let status = find(&items, ItemKind::StatusLine, "status line");
    let Detail::Setting { entries, about, .. } = &status.detail else {
        panic!()
    };
    assert!(entries.is_empty() && about.is_none());
    assert!(matches!(can_set_up(status), SetUp::Impossible(note) if note.contains("secret")));
}

#[test]
fn each_kind_says_whether_claude_can_set_it_up() {
    let root = tempfile::tempdir().unwrap();
    let (dir, state, project) = account(root.path());
    let items = inventory(&dir, &state, Some(&project));
    let verdict = |kind, name| can_set_up(find(&items, kind, name));
    assert_eq!(verdict(ItemKind::McpServer, "files"), SetUp::Copy);
    assert!(
        matches!(verdict(ItemKind::McpServer, "linear"), SetUp::NeedsYou(note) if note.contains("claude mcp login linear"))
    );
    // A reference to the user's own environment travels as it is.
    assert_eq!(verdict(ItemKind::McpServer, "keyed"), SetUp::Copy);
    let Detail::Mcp(keyed) = &find(&items, ItemKind::McpServer, "keyed").detail else {
        panic!()
    };
    assert!(
        matches!(&keyed.transport, McpTransport::Stdio { env, .. } if env[0].1 == Value::Env("MY_KEY".into()))
    );
    assert!(
        !matches!(verdict(ItemKind::McpServer, "search"), SetUp::Copy),
        "a written-in token"
    );
    assert_eq!(verdict(ItemKind::Skill, "review"), SetUp::Copy);
    assert!(
        matches!(verdict(ItemKind::Skill, "pdf"), SetUp::Impossible(note) if note.contains("claude.ai"))
    );
    assert_eq!(verdict(ItemKind::Command, "ship"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::Subagent, "tester"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::Rules, "rules/style.md"), SetUp::Copy);
    assert_eq!(verdict(ItemKind::MainRules, "CLAUDE.md"), SetUp::Copy);
    assert!(
        matches!(verdict(ItemKind::Plugin, "lint@market"), SetUp::NeedsYou(note) if note.contains("claude plugin install lint@market"))
    );
    assert_eq!(verdict(ItemKind::StatusLine, "status line"), SetUp::Copy);

    // Items from another tool.
    let from_codex = |kind, detail| Item {
        tool: "codex".into(),
        kind,
        name: "x".into(),
        scope: Scope::User,
        detail,
    };
    let files = Detail::Files {
        path: dir.join("commands/ship.md"),
    };
    assert_eq!(
        can_set_up(&from_codex(ItemKind::Skill, files.clone())),
        SetUp::Copy
    );
    assert!(
        matches!(can_set_up(&from_codex(ItemKind::Command, files.clone())), SetUp::Translate(note) if note.contains("$0"))
    );
    assert!(
        matches!(can_set_up(&from_codex(ItemKind::MainRules, files.clone())), SetUp::Translate(note) if note.contains("CLAUDE.md"))
    );
    assert!(
        matches!(can_set_up(&from_codex(ItemKind::Subagent, files)), SetUp::Impossible(note) if note.contains("no Claude Code equivalent"))
    );
    let setting = Detail::Setting {
        file: dir.join("config.toml"),
        format: juggler_adapter::inventory::ConfigFormat::Toml,
        entries: Vec::new(),
        about: None,
        check: juggler_adapter::inventory::SettingCheck::Fine,
    };
    assert!(
        matches!(can_set_up(&from_codex(ItemKind::Permissions, setting)), SetUp::Impossible(note) if note.contains("work differently"))
    );
}

/// A stub `claude` that records its arguments, folder and config dir.
fn stub_claude(root: &Path, exit: i32) -> (PathBuf, PathBuf) {
    let log = root.join("claude-calls.log");
    let stub = root.join("claude");
    write(
        &stub,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"${{CLAUDE_CONFIG_DIR:-unset}}\" \"$@\" > '{}'\necho 'added' ; echo 'boom' >&2\nexit {exit}\n",
            log.display()
        ),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (stub, log)
}

#[test]
fn files_are_copied_as_new_files_and_settings_merged() {
    let root = tempfile::tempdir().unwrap();
    let (from, state, project) = account(root.path());
    let items = inventory(&from, &state, Some(&project));
    let to = root.path().join("claude-b");
    std::fs::create_dir_all(&to).unwrap();
    let (claude, _) = stub_claude(root.path(), 0);
    let to_state = to.join(".claude.json");
    let go = |kind, name| {
        set_up(
            &to,
            &claude,
            "",
            &to_state,
            Some(&project),
            find(&items, kind, name),
        )
    };

    go(ItemKind::Skill, "review").unwrap();
    assert!(to.join("skills/review/SKILL.md").is_file());
    go(ItemKind::Command, "ship").unwrap();
    assert_eq!(
        std::fs::read_to_string(to.join("commands/ship.md")).unwrap(),
        "Ship it: $ARGUMENTS\n"
    );
    go(ItemKind::Subagent, "tester").unwrap();
    assert!(to.join("agents/tester.md").is_file());
    go(ItemKind::Rules, "rules/style.md").unwrap();
    assert!(to.join("rules/style.md").is_file());
    go(ItemKind::MainRules, "CLAUDE.md").unwrap();
    assert_eq!(
        std::fs::read_to_string(to.join("CLAUDE.md")).unwrap(),
        "Use tabs.\n"
    );
    // Never over a file that's there.
    assert!(go(ItemKind::MainRules, "CLAUDE.md").is_err());

    let applied = go(ItemKind::Permissions, "permissions").unwrap();
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(to.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["permissions"]["allow"],
        serde_json::json!(["Bash(npm test)"])
    );
    let Undo::RemoveSetting { entries, .. } = applied.undo else {
        panic!("{:?}", applied.undo)
    };
    assert!(
        entries
            .iter()
            .all(|(_, fingerprint)| !fingerprint.contains("npm")),
        "fingerprints, not values"
    );

    let applied = go(ItemKind::StatusLine, "status line").unwrap();
    assert!(applied.summary.contains("status line"));

    // What can't be set up says why.
    assert!(go(ItemKind::Skill, "pdf").is_err());
    assert!(go(ItemKind::Plugin, "lint@market").is_err());
}

#[test]
fn a_codex_prompt_becomes_a_claude_command_with_its_arguments_renumbered() {
    let root = tempfile::tempdir().unwrap();
    let prompt = root.path().join("prompts/fix.md");
    write(&prompt, "Fix $1 in $2, not $10 or $$.\n");
    let to = root.path().join("claude");
    std::fs::create_dir_all(&to).unwrap();
    let item = Item {
        tool: "codex".into(),
        kind: ItemKind::Command,
        name: "fix".into(),
        scope: Scope::User,
        detail: Detail::Files { path: prompt },
    };
    set_up(
        &to,
        Path::new("claude"),
        "",
        &to.join(".claude.json"),
        None,
        &item,
    )
    .unwrap();
    let text = std::fs::read_to_string(to.join("commands/fix.md")).unwrap();
    assert!(text.starts_with("Fix $0 in $1"), "{text}");
}

#[test]
fn an_mcp_server_is_added_through_claude_itself() {
    let root = tempfile::tempdir().unwrap();
    let (from, state, project) = account(root.path());
    let items = inventory(&from, &state, Some(&project));
    let to = root.path().join("claude-b");
    std::fs::create_dir_all(&to).unwrap();
    let (claude, log) = stub_claude(root.path(), 0);
    let to_state = to.join(".claude.json");

    // The user's server, in the account given by CLAUDE_CONFIG_DIR.
    let applied = set_up(
        &to,
        &claude,
        &to.to_string_lossy(),
        &to_state,
        None,
        find(&items, ItemKind::McpServer, "files"),
    )
    .unwrap();
    let calls = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = calls.lines().collect();
    assert_eq!(lines[1], to.to_string_lossy());
    assert_eq!(
        &lines[2..8],
        &["mcp", "add-json", "-s", "user", "--", "files"]
    );
    let json: serde_json::Value = serde_json::from_str(lines[8]).unwrap();
    assert_eq!(json["command"], "npx");
    let Undo::Command { argv, env, .. } = &applied.undo else {
        panic!("{:?}", applied.undo)
    };
    assert_eq!(&argv[1..], &["mcp", "remove", "-s", "user", "--", "files"]);
    assert_eq!(env[0].0, "CLAUDE_CONFIG_DIR");

    // The project's own server, in the project's folder, the vendor's
    // default account (CLAUDE_CONFIG_DIR unset).
    let applied = set_up(
        &to,
        &claude,
        "",
        &to_state,
        Some(&project),
        find(&items, ItemKind::McpServer, "db"),
    )
    .unwrap();
    let calls = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = calls.lines().collect();
    assert_eq!(
        std::fs::canonicalize(lines[0]).unwrap(),
        std::fs::canonicalize(&project).unwrap()
    );
    assert_eq!(lines[1], "unset");
    assert_eq!(lines[5], "local");
    let Undo::Command { cwd, unset, .. } = &applied.undo else {
        panic!()
    };
    assert_eq!(cwd.as_deref(), Some(project.as_path()));
    assert_eq!(unset, &["CLAUDE_CONFIG_DIR".to_owned()]);

    // A project's server with no project folder known is refused.
    assert!(
        set_up(
            &to,
            &claude,
            "",
            &to_state,
            None,
            find(&items, ItemKind::McpServer, "db")
        )
        .is_err()
    );

    // Claude refusing is an error, with what it said.
    let (failing, _) = stub_claude(&root.path().join("f"), 1);
    let error = set_up(
        &to,
        &failing,
        "",
        &to_state,
        None,
        find(&items, ItemKind::McpServer, "files"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("boom"), "{error}");
}
