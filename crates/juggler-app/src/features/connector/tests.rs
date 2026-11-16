//! AJ-E3013: the connector through the app (a fixture Claude app profile
//! and Codex app over temporary folders; nothing launched), and a pool's
//! desktop members as the canvas reads them.

use std::sync::Arc;

use juggler_core::agent_access::AccessLevel;
use juggler_core::model::SessionId;
use juggler_core::pools::SessionFacts;
use juggler_core::storage::attested::desktop_id;
use juggler_core::storage::{MemberRef, PoolRole};

use crate::state::AppState;

struct Lab {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    claude_app: String,
    codex_app: String,
    claude_config: std::path::PathBuf,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::new()));
    registry.register(Arc::new(juggler_adapters::CodexAdapter::new()));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("data"), registry).unwrap());
    // Any program of root's stands in for juggler-agent (never run here).
    state
        .agent_bridge
        .set_program(Some(std::path::PathBuf::from("/bin/echo")));
    let claude_config = dir.path().join("claude-config");
    let codex_home = dir.path().join("codex-home");
    std::fs::create_dir_all(&claude_config).unwrap();
    std::fs::create_dir_all(&codex_home).unwrap();
    let (claude_app, codex_app) = {
        let storage = state.storage.lock().unwrap();
        for (id, tool, folder) in [
            (
                "acc-claude",
                juggler_adapters::CLAUDE_CODE_ID,
                &claude_config,
            ),
            ("acc-codex", juggler_adapters::CODEX_ID, &codex_home),
        ] {
            storage
                .accounts()
                .insert(&crate::test_support::account(id, tool, id, folder.clone()))
                .unwrap();
        }
        let apps = storage.apps();
        let claude = apps
            .upsert_desktop(
                juggler_adapters::CLAUDE_CODE_ID,
                "claude-app",
                Some(&dir.path().join("instance").to_string_lossy()),
                1,
            )
            .unwrap();
        apps.set_folder(&claude, Some("acc-claude")).unwrap();
        let codex = apps
            .upsert_desktop(juggler_adapters::CODEX_ID, "codex-app", None, 1)
            .unwrap();
        apps.set_folder(&codex, Some("acc-codex")).unwrap();
        (claude, codex)
    };
    Lab {
        _dir: dir,
        state,
        claude_app,
        codex_app,
        claude_config,
    }
}

#[test]
fn preview_add_status_and_remove_through_the_app() {
    let lab = lab();
    let status = super::status_impl(&lab.state, &lab.claude_app).unwrap();
    assert_eq!(status.state, "absent", "{status:?}");
    assert!(status.attests && status.can_wake);
    let preview = super::preview_impl(&lab.state, &lab.claude_app).unwrap();
    assert!(preview.refused.is_none());
    assert!(preview.files.iter().all(|file| file.action == "create"));
    assert!(
        preview.files[0]
            .path
            .ends_with("skills/ai-juggler/.claude-plugin/plugin.json")
    );
    assert!(
        preview.files[0].after.contains("/bin/echo"),
        "the bridge by its path"
    );
    assert!(preview.guide.is_some());

    let added = super::add_impl(&lab.state, &lab.claude_app).unwrap();
    assert_eq!(added.state, "current");
    assert!(
        lab.claude_config
            .join("skills/ai-juggler/.claude-plugin/plugin.json")
            .is_file()
    );
    let records = std::fs::read_to_string(lab.state.data_dir.join(super::RECORDS)).unwrap();
    assert!(records.contains("sha256"), "each file's hash is kept");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(lab.state.data_dir.join(super::RECORDS))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "the record is the user's alone");
    }

    let skill = lab
        .claude_config
        .join("skills/ai-juggler/skills/pool-tasks/SKILL.md");
    std::fs::write(&skill, "mine now\n").unwrap();
    assert_eq!(
        super::status_impl(&lab.state, &lab.claude_app)
            .unwrap()
            .state,
        "changed"
    );
    // A11: all or nothing, the changed file named; then put back as Add
    // wrote it, Remove takes it all out.
    assert!(super::remove_impl(&lab.state, &lab.claude_app).is_err());
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), "mine now\n");
    assert!(
        lab.claude_config
            .join("skills/ai-juggler/.claude-plugin")
            .exists(),
        "nothing taken out"
    );
    super::add_impl(&lab.state, &lab.claude_app).unwrap_err();
    let removal = super::remove_anyway_impl(&lab.state, &lab.claude_app).unwrap();
    assert!(removal.kept.is_empty());
    assert!(!lab.claude_config.join("skills/ai-juggler").exists());
}

/// The lab's A11: Remove is all or nothing; a file changed since Add
/// refuses it, naming the file, and Remove anyway (the window's confirmed
/// choice) takes it all out.
#[test]
fn remove_refuses_on_a_changed_file_and_remove_anyway_takes_it_all() {
    let lab = lab();
    super::add_impl(&lab.state, &lab.claude_app).unwrap();
    let skill = lab
        .claude_config
        .join("skills/ai-juggler/skills/pool-tasks/SKILL.md");
    std::fs::write(&skill, "my own notes\n").unwrap();
    let refused = super::remove_impl(&lab.state, &lab.claude_app).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("skills/ai-juggler/skills/pool-tasks/SKILL.md"),
        "{refused}"
    );
    assert!(skill.exists(), "nothing taken out");
    assert!(
        lab.claude_config
            .join("skills/ai-juggler/.claude-plugin/plugin.json")
            .exists()
    );
    assert_eq!(
        super::status_impl(&lab.state, &lab.claude_app)
            .unwrap()
            .state,
        "changed"
    );
    let removal = super::remove_anyway_impl(&lab.state, &lab.claude_app).unwrap();
    assert!(removal.kept.is_empty());
    assert!(!lab.claude_config.join("skills/ai-juggler").exists());
    assert_eq!(removal.status.state, "absent");
}

/// The window words each reason from its code (`unavailable`), never from
/// `why_not`'s raw words: one code a reason.
#[cfg(unix)]
#[test]
fn every_reason_the_connector_isnt_there_has_its_code() {
    use std::os::unix::fs::PermissionsExt as _;
    let lab = lab();
    let code = |app: &str| super::status_impl(&lab.state, app).unwrap().unavailable;
    // Offered, not added.
    assert_eq!(code(&lab.claude_app).as_deref(), Some("no_connector"));
    // Added: nothing to say.
    super::add_impl(&lab.state, &lab.claude_app).unwrap();
    assert_eq!(code(&lab.claude_app), None);
    // Not offered: the Codex app until C2.
    assert_eq!(code(&lab.codex_app).as_deref(), Some("not_offered"));
    // An App AI Juggler doesn't know.
    assert_eq!(code("app:nope:").as_deref(), Some("unknown_app"));
    // A desktop App with no config folder yet.
    let bare = lab
        .state
        .storage
        .lock()
        .unwrap()
        .apps()
        .upsert_desktop(
            juggler_adapters::CLAUDE_CODE_ID,
            "claude-app",
            Some("/nowhere/x"),
            1,
        )
        .unwrap();
    assert_eq!(code(&bare).as_deref(), Some("no_folder"));
    // No juggler-agent beside AI Juggler.
    let away = tempfile::tempdir().unwrap();
    lab.state
        .agent_bridge
        .set_program(Some(away.path().join("juggler-agent")));
    assert_eq!(code(&lab.claude_app).as_deref(), Some("missing"));
    // In a folder others can write (as from /tmp): named.
    let open = away.path().join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
    std::fs::copy("/bin/echo", open.join("juggler-agent")).unwrap();
    lab.state
        .agent_bridge
        .set_program(Some(open.join("juggler-agent")));
    let status = super::status_impl(&lab.state, &lab.claude_app).unwrap();
    assert_eq!(status.unavailable.as_deref(), Some("unsafe_location"));
    let folder = status.unsafe_folder.unwrap();
    let canonical = std::fs::canonicalize(&open).unwrap();
    assert!(
        [open.display().to_string(), canonical.display().to_string()].contains(&folder),
        "{folder}"
    );
    assert!(status.why_not.is_some(), "the words stay for logs");
}

#[test]
fn an_app_it_cant_go_into_says_why() {
    let lab = lab();
    let unknown = super::status_impl(&lab.state, "app:nope:").unwrap();
    assert_eq!(unknown.state, "unavailable");
    assert!(unknown.why_not.is_some());
    let cli = super::status_impl(&lab.state, "cli:acc-claude").unwrap();
    assert_eq!(cli.state, "unavailable");
    assert!(super::add_impl(&lab.state, "app:nope:").is_err());
    assert!(
        super::remove_impl(&lab.state, &lab.claude_app).is_err(),
        "nothing recorded, nothing taken out"
    );
    // Not offered for the Codex app until lab probe C2 (2026-11-15).
    let codex = super::status_impl(&lab.state, &lab.codex_app).unwrap();
    assert_eq!(codex.state, "unavailable");
    assert_eq!(
        codex.why_not.as_deref(),
        Some("Codex can't yet tell which thread is calling")
    );
    let preview = super::preview_impl(&lab.state, &lab.codex_app).unwrap();
    assert_eq!(preview.status.state, "unavailable");
    assert!(preview.files.is_empty() && preview.guide.is_none());
    assert!(
        super::add_impl(&lab.state, &lab.codex_app).is_err(),
        "no Add"
    );
}

#[test]
fn a_pools_desktop_members_say_their_connector_connection_and_wake() {
    let lab = lab();
    let pool = {
        let storage = lab.state.storage.lock().unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        for member in [
            MemberRef::Conversation {
                folder: "acc-claude".into(),
                id: "conv-1".into(),
            },
            MemberRef::App(lab.codex_app.clone()),
            MemberRef::Session(SessionId::new("s-cli")),
        ] {
            storage
                .pools()
                .join(&pool, &member, &[], PoolRole::Member, None)
                .unwrap();
        }
        pool
    };
    let members = super::desktop_members_impl(&lab.state, &pool).unwrap();
    assert_eq!(
        members.len(),
        2,
        "the terminal session isn't a desktop member"
    );
    let claude = &members[0];
    assert_eq!(claude.member, "conversation:acc-claude/conv-1");
    assert_eq!(claude.app_id.as_deref(), Some(lab.claude_app.as_str()));
    assert_eq!(
        (
            claude.connector.as_str(),
            claude.connected,
            claude.wake.as_str()
        ),
        ("absent", false, "next_turn"),
        "Work together: it reads on its next turn"
    );
    assert_eq!(members[1].wake, "cant_wake");
    // Attested: connected; the pool at Full: woken.
    lab.state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(SessionFacts {
            app: Some(lab.claude_app.clone()),
            conversation: Some(("acc-claude".into(), "conv-1".into())),
            ..SessionFacts::bare(desktop_id("acc-claude", "conv-1"), None)
        });
    lab.state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .set_level(&pool, AccessLevel::Full, &std::collections::BTreeSet::new())
        .unwrap();
    let members = super::desktop_members_impl(&lab.state, &pool).unwrap();
    assert!(members[0].connected);
    assert_eq!(members[0].wake, "wakes");
    assert_eq!(
        members[1].wake, "cant_wake",
        "a Codex app thread, even at Full"
    );
    assert!(super::desktop_members_impl(&lab.state, "no-such-pool").is_err());
}
