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
use juggler_app::features::switch_setup::{
    apply_impl, check_impl, forget_impl, list_impl, turn_ons, undo_impl,
};
use juggler_app::state::AppState;

/// A `claude` that records its arguments (for `claude mcp add-json`).
fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        // Records its arguments; `add-json` also writes the entry into the
        // account's state file, as Claude does.
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/mcp-args.tmp\"\nmv \"$CLAUDE_CONFIG_DIR/mcp-args.tmp\" \"$CLAUDE_CONFIG_DIR/mcp-args\"\nif [ \"$2\" = add-json ]; then printf '{\"mcpServers\":{\"%s\":%s}}' \"$6\" \"$7\" > \"$CLAUDE_CONFIG_DIR/.claude.json\"; fi\n",
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
    // Synced from claude.ai with nothing saying whose: turned on there.
    assert_eq!(verdict("team-style"), Some(("turn-on", false)));
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
        args.starts_with("mcp\nadd-json\n-s\nuser\n--\ngithub\n"),
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

    // Review of #132 (M1): a server the user changed since is left alone;
    // as it was made, it's removed with Claude's own command.
    let server = applied
        .iter()
        .find(|a| a.message.contains("github"))
        .unwrap()
        .record
        .clone()
        .unwrap();
    let state_file = b.join(".claude.json");
    let made = std::fs::read_to_string(&state_file).unwrap();
    std::fs::write(&state_file, made.replace("GITHUB_TOKEN", "MY_TOKEN")).unwrap();
    assert!(
        undo_impl(&lab.state, &server).is_err(),
        "changed since: left as it is"
    );
    std::fs::write(&state_file, &made).unwrap();
    undo_impl(&lab.state, &server).unwrap();
    let args = std::fs::read_to_string(b.join("mcp-args")).unwrap();
    assert!(
        args.starts_with("mcp\nremove\n-s\nuser\n--\ngithub\n"),
        "{args}"
    );

    // Set up again, then removed by the user: the undo has nothing left to
    // do, and says it's done rather than failing for ever (re-review).
    std::fs::write(&state_file, "{}").unwrap(); // what Claude's remove left
    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    let again = check
        .offers
        .iter()
        .find(|o| o.name == "github")
        .unwrap()
        .id
        .clone();
    let record = apply_impl(&lab.state, &lab.session, profile_b, &[again]).unwrap()[0]
        .record
        .clone()
        .unwrap();
    std::fs::write(&state_file, "{\"mcpServers\":{}}").unwrap();
    undo_impl(&lab.state, &record).unwrap();
    assert!(
        list_impl(&lab.state)
            .unwrap()
            .iter()
            .all(|s| s.record != record)
    );
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
    let backup = x.join("config.toml.before-ai-juggler");
    assert_eq!(
        std::fs::read_to_string(&backup).unwrap(),
        "model = \"gpt-6\"\n",
        "the backup is the file from before the set-up"
    );
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
    assert!(
        applied
            .iter()
            .any(|a| a.message == "Set up the MCP server github"),
        "{applied:?}"
    );
    // Settings lists it until it's undone (review of #132, M3).
    let listed = list_impl(&lab.state).unwrap();
    assert!(
        listed
            .iter()
            .any(|s| s.what == "the MCP server github" && s.record == server),
        "{listed:?}"
    );
    assert_eq!(
        undo_impl(&lab.state, &server).unwrap(),
        "Removed the MCP server github"
    );
    assert!(
        list_impl(&lab.state)
            .unwrap()
            .iter()
            .all(|s| s.record != server)
    );
    assert_eq!(
        std::fs::read_to_string(x.join("config.toml")).unwrap(),
        "model = \"gpt-6\"\n"
    );
    // Lab 6: the backup outlived the undo in the user's Codex folder.
    assert!(!backup.exists(), "the backup goes with the last undo");
}

#[test]
fn codex_config_stays_a_link_with_its_permissions_and_its_users_settings() {
    // Review of #132 (M2): the rewrite replaced a dotfiles link with a
    // plain file at default permissions, and the undo moved settings the
    // user added under our table into the one before.
    use std::os::unix::fs::PermissionsExt;
    let lab = lab();
    let (profile_x, x) = &lab.codex;
    let real = x.join("dotfiles-config.toml");
    std::fs::rename(x.join("config.toml"), &real).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&real, x.join("config.toml")).unwrap();

    let check = check_impl(&lab.state, &lab.session, profile_x).unwrap();
    let github = check
        .offers
        .iter()
        .find(|o| o.name == "github")
        .unwrap()
        .id
        .clone();
    let applied = apply_impl(&lab.state, &lab.session, profile_x, &[github]).unwrap();
    let record = applied[0].record.clone().expect("set up");
    let link = std::fs::symlink_metadata(x.join("config.toml")).unwrap();
    assert!(link.file_type().is_symlink(), "still a link");
    let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "its permissions kept");
    assert!(
        std::fs::read_to_string(&real)
            .unwrap()
            .contains("[mcp_servers.github]")
    );

    // The user adds a setting under our table: the undo leaves it all.
    let with_user = std::fs::read_to_string(&real).unwrap() + "startup_timeout_sec = 30\n";
    std::fs::write(&real, &with_user).unwrap();
    assert!(undo_impl(&lab.state, &record).is_err());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), with_user);

    // Without it, the undo restores the file, through the link.
    std::fs::write(&real, with_user.replace("startup_timeout_sec = 30\n", "")).unwrap();
    undo_impl(&lab.state, &record).unwrap();
    assert_eq!(
        std::fs::read_to_string(&real).unwrap(),
        "model = \"gpt-6\"\n"
    );
    assert!(
        std::fs::symlink_metadata(x.join("config.toml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a_set_up_that_cant_be_undone_can_be_forgotten_leaving_it_as_it_is() {
    // Re-review of #132 (Low): a set-up the user has changed since can't
    // be undone, and Settings kept offering an Undo that always failed.
    let lab = lab();
    let (profile_x, x) = &lab.codex;
    let check = check_impl(&lab.state, &lab.session, profile_x).unwrap();
    let github = check
        .offers
        .iter()
        .find(|o| o.name == "github")
        .unwrap()
        .id
        .clone();
    let applied = apply_impl(&lab.state, &lab.session, profile_x, &[github]).unwrap();
    let record = applied[0].record.clone().expect("set up");
    let config = x.join("config.toml");
    let changed = std::fs::read_to_string(&config).unwrap() + "startup_timeout_sec = 30\n";
    std::fs::write(&config, &changed).unwrap();
    assert!(undo_impl(&lab.state, &record).is_err());

    let message = forget_impl(&lab.state, &record).unwrap();
    assert!(message.starts_with("Forgot the "), "{message}");
    assert_eq!(
        std::fs::read_to_string(&config).unwrap(),
        changed,
        "nothing changes"
    );
    assert!(list_impl(&lab.state).unwrap().is_empty());
    assert!(forget_impl(&lab.state, &record).is_err(), "forgotten once");
}

/// Sets `session`'s transcript to `text` (synthetic), in `dir`.
fn with_transcript(state: &Arc<AppState>, session: &str, dir: &Path, text: &str) {
    let path = dir.join("conversation.jsonl");
    std::fs::write(&path, text).unwrap();
    let storage = state.storage.lock().unwrap();
    let mut record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session))
        .unwrap();
    record.transcript_ref = Some(juggler_core::model::TranscriptRef {
        path,
        native_id: None,
    });
    storage.sessions().update(&record).unwrap();
}

/// switch-setup-items.md: what the conversation used leads; what comes
/// with the claude.ai account is turned on for the other account, never
/// copied; what may be the user's own can be copied on their say-so; the
/// switch keeps a reminder until the other account has it (Check again).
#[test]
fn what_was_used_leads_and_what_comes_with_the_account_is_turned_on() {
    let lab = lab();
    let (profile_b, b) = &lab.claude_b;
    let set = lab.source_dir.join("skills/synced/org_acct");
    std::fs::create_dir_all(&set).unwrap();
    std::fs::write(
        set.join("manifest.json"),
        serde_json::json!({"skills": [
            {"skillId": "computer-use", "name": "computer-use", "source": "anthropic-example"},
            {"skillId": "docs", "name": "docs", "source": "anthropic-example"},
            {"skillId": "team-notes", "name": "team-notes", "source": "custom"}
        ]})
        .to_string(),
    )
    .unwrap();
    for skill in ["computer-use", "docs", "team-notes"] {
        std::fs::create_dir_all(set.join(skill)).unwrap();
        std::fs::write(
            set.join(skill).join("SKILL.md"),
            format!("---\nname: {skill}\n---\n"),
        )
        .unwrap();
    }
    // Used: release-notes (the Skill tool), computer-use (typed as a
    // synced skill), team-notes (typed), github (MCP). Not used: review,
    // docs, team-style.
    with_transcript(
        &lab.state,
        &lab.session,
        lab._work.path(),
        concat!(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Skill","input":{"skill":"release-notes"}}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<command-name>/anthropic-skills:computer-use</command-name>"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<command-name>/team-notes</command-name>"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t2","name":"mcp__github__create_issue","input":{}}]}}"#,
            "\n",
        ),
    );

    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    let offer = |name: &str| {
        check
            .offers
            .iter()
            .find(|offer| offer.name == name)
            .expect("offered")
    };
    // Used first, unused after; an unused one is never ticked for you.
    let first_unused = check
        .offers
        .iter()
        .position(|offer| offer.used != Some(true))
        .unwrap();
    assert!(
        check.offers[first_unused..]
            .iter()
            .all(|offer| offer.used != Some(true)),
        "what was used leads"
    );
    assert_eq!(offer("release-notes").used, Some(true));
    assert!(offer("release-notes").suggested);
    assert_eq!(offer("review").used, Some(false));
    assert_eq!(offer("review").verdict, "copy");
    assert!(
        !offer("review").suggested,
        "not used here: offered, not ticked"
    );
    assert_eq!(offer("docs").used, Some(false));

    // Anthropic's: steps for Personal, its page, never copied.
    let computer = offer("computer-use");
    assert_eq!(computer.verdict, "turn-on");
    assert_eq!(computer.used, Some(true));
    assert_eq!(computer.origin, "provided");
    assert_eq!(computer.provided_by.as_deref(), Some("Anthropic"));
    assert!(!computer.can_copy);
    assert!(!computer.suggested);
    let get_it = computer.get_it.as_ref().expect("steps");
    assert!(get_it.steps[0].contains("as Personal"));
    assert!(
        get_it
            .steps
            .last()
            .unwrap()
            .contains("Start a session on Personal")
    );
    assert_eq!(
        get_it.open_url.as_deref(),
        Some("https://claude.ai/customize/skills")
    );
    assert!(get_it.needs.iter().any(|need| need.contains("Pro or Max")));
    assert!(!format!("{:?}", check.offers).contains("{account}"));
    assert!(!format!("{:?}", check.offers).contains("{tool}"));

    // The account's own content: turned on there, or copied if yours.
    let notes = offer("team-notes");
    assert_eq!(
        (
            notes.verdict.as_str(),
            notes.origin.as_str(),
            notes.can_copy
        ),
        ("turn-on", "account", true)
    );

    // Applying: the provided skill is refused whatever id is sent; the
    // account's is copied, with an undo.
    let picked = vec![computer.id.clone(), notes.id.clone()];
    let applied = apply_impl(&lab.state, &lab.session, profile_b, &picked).unwrap();
    let refused = applied
        .iter()
        .find(|result| result.offer == computer.id)
        .unwrap();
    assert!(refused.record.is_none());
    assert!(refused.message.contains("turn it on for Personal"));
    assert!(!b.join("skills/computer-use").exists(), "never copied");
    assert!(b.join("skills/team-notes/SKILL.md").is_file());

    // The switch kept a reminder for the used, provided one only (not
    // the copied one, not the unused ones).
    let reminders = turn_ons::list_impl(&lab.state).unwrap();
    let names: Vec<&str> = reminders.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["computer-use"]);
    assert_eq!(reminders[0].account, "Personal");
    // Applying again keeps one reminder, not two.
    apply_impl(&lab.state, &lab.session, profile_b, &[]).unwrap();
    assert_eq!(turn_ons::list_impl(&lab.state).unwrap().len(), 1);

    // Check again: once Personal's own sync brings it, the reminder goes.
    std::fs::create_dir_all(b.join("skills/synced/org_b/computer-use")).unwrap();
    assert!(turn_ons::list_impl(&lab.state).unwrap().is_empty());
    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    assert!(
        !check
            .offers
            .iter()
            .any(|offer| offer.name == "computer-use")
    );
}

/// Another tool's notes name the tool, never its id.
#[test]
fn notes_name_the_tool_not_its_id() {
    let lab = lab();
    let (profile_x, _) = &lab.codex;
    let check = check_impl(&lab.state, &lab.session, profile_x).unwrap();
    let synced = check
        .offers
        .iter()
        .find(|offer| offer.name == "team-style")
        .unwrap();
    assert_eq!(synced.verdict, "impossible");
    let note = synced.note.as_deref().unwrap();
    assert!(note.contains("Claude Code"));
    assert!(!note.contains("claude-code"));
    assert!(!note.contains("{tool}"));
}
