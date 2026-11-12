//! 0.4.3 (D-102): a script's run is History only, and each Codex thread is
//! in exactly one App.
//!
//! The maintainer's Mac: Codex CLI said 92 sessions where the Codex store
//! lists 33 (59 were `codex exec` runs), and CLI Apps stood in Inactive
//! apps for profiles' folders that only scripts' `claude -p` ran in. Both
//! tools' own pickers leave a script's run out (`claude --resume` hides a
//! session whose first entrypoint is `sdk-*`; Codex lists only the sources
//! `cli` and `vscode`), so AI Juggler lists it in History only: no App
//! counts it and none makes a CLI App. AI Juggler's own Claude sessions run
//! as `claude -p` too: they stay, counted once as their AI Juggler session.
//! All paths and names are invented.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step is its failure"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_app::features::apps::list_impl;
use juggler_app::features::relations::{self, Change, EntityKind, EntityRef};
use juggler_app::features::sessions::rows::{SessionsQuery, sessions_page_impl};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    AccountId, ConversationId, InstanceKind, InstanceRecord, MachineId, SessionId, SessionStatus,
    Timestamp,
};
use juggler_core::storage::{App, cli_app_id, desktop_app_id};

/// The Claude app profile's instance folder (never read: a name in a row).
const PROFILE_DIR: &str = "/Users/you/Library/Application Support/ClaudeProfiles/lab";

/// A Claude Code folder a Claude app profile holds, and a Codex home the
/// Codex app holds, both brought in where they are; nothing run yet.
struct World {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    /// The folders' ids.
    claude: String,
    codex: String,
    /// Where the Claude folder's transcripts of the project go.
    transcripts: PathBuf,
    /// The Codex home.
    home: PathBuf,
    repo: PathBuf,
}

fn world() -> World {
    use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let repo = root.join("Dev/shop");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let claude_dir = root.join("ClaudeCodeProfiles/lab");
    let transcripts = claude_dir
        .join("projects")
        .join(repo.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&transcripts).unwrap();
    std::fs::write(claude_dir.join("settings.json"), "{}").unwrap();
    let home = root.join("codex-home");
    std::fs::create_dir_all(home.join("sessions/2026/10/21")).unwrap();
    std::fs::create_dir_all(home.join("archived_sessions")).unwrap();
    std::fs::write(home.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    registry.register(Arc::new(
        CodexAdapter::with_bin(Path::new("/bin/true")).with_home(&root),
    ));
    let state = Arc::new(AppState::init_with_registry(root.join("data"), registry).unwrap());
    juggler_app::features::cache::register_defaults(&state);
    let import = |tool: &str, folder: &Path, label: &str| {
        juggler_app::features::account::import::account_import_mode_impl(
            &state,
            tool,
            folder.to_str().unwrap(),
            label,
            juggler_app::features::account::import::ImportRequest {
                in_place: true,
                ..Default::default()
            },
        )
        .unwrap()
        .account
        .id
        .to_string()
    };
    let claude = import("claude-code", &claude_dir, "Lab");
    let codex = import("codex", &home, "Codex home");
    {
        let storage = state.storage.lock().unwrap();
        storage.projects().touch(&repo, None).unwrap();
        for (id, tool, host, instance, folder, name) in [
            (
                desktop_app_id("claude-app", Some(PROFILE_DIR)),
                "claude-code",
                "claude-app",
                Some(PROFILE_DIR),
                &claude,
                serde_json::json!({"name": "Claude", "instance": "Lab", "bundle": "/Applications/Claude.app", "given": PROFILE_DIR}),
            ),
            (
                desktop_app_id("codex-app", None),
                "codex",
                "codex-app",
                None,
                &codex,
                serde_json::json!({"name": "ChatGPT", "instance": null, "bundle": "/Applications/ChatGPT.app"}),
            ),
        ] {
            storage
                .apps()
                .put_desktop(&App {
                    id: id.clone(),
                    kind: "desktop".into(),
                    tool: tool.into(),
                    host_app_id: Some(host.into()),
                    instance_dir: instance.map(str::to_owned),
                    folder_id: Some(folder.clone()),
                    label: None,
                    hidden: false,
                    keep_alive: false,
                    paused_json: None,
                    launch_json: Some(name.to_string()),
                    sessions_json: None,
                    first_seen: 1,
                    last_seen: 1,
                    added: true,
                })
                .unwrap();
            storage.apps().add(&id).unwrap();
        }
    }
    World {
        _dir: dir,
        state,
        claude,
        codex,
        transcripts,
        home,
        repo,
    }
}

impl World {
    /// A Claude Code transcript `id` whose lines say `entrypoint`.
    fn transcript(&self, id: &str, entrypoint: &str) {
        std::fs::write(
            self.transcripts.join(format!("{id}.jsonl")),
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"entrypoint\":\"{entrypoint}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Tidy the logs\"}}}}\n",
                self.repo.display()
            ),
        )
        .unwrap();
    }

    /// A Codex rollout of thread `n` (`source` as JSON), in `sessions/` or,
    /// archived, in `archived_sessions/`; its thread id.
    fn rollout(&self, n: u32, source: &str, originator: &str, archived: bool) -> String {
        let id = format!("0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a{n:02}");
        let at = if archived {
            self.home.join("archived_sessions")
        } else {
            self.home.join("sessions/2026/10/21")
        };
        std::fs::write(
            at.join(format!("rollout-2026-10-21T10-00-{n:02}-{id}.jsonl")),
            format!(
                "{{\"timestamp\":\"2026-10-21T10:00:{n:02}.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"cwd\":\"{}\",\"source\":{source},\"originator\":\"{originator}\"}}}}\n{{\"timestamp\":\"2026-10-21T10:01:00.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Speed up the import job\"}}]}}}}\n",
                self.repo.display()
            ),
        )
        .unwrap();
        id
    }

    /// Reads the files again, as the background pass does.
    fn read(&self) {
        juggler_app::features::conversations::refresh_impl(&self.state).unwrap();
        relations::changed(&self.state, Change::ConversationsIndex);
    }

    /// AI Juggler's own terminal session `id` on the Claude folder, on
    /// conversation `conversation` once it knows it.
    fn own_session(&self, id: &str, conversation: Option<&str>) {
        let storage = self.state.storage.lock().unwrap();
        let recipe = storage
            .profiles()
            .list_by_account(&AccountId::new(self.claude.as_str()))
            .unwrap()
            .remove(0)
            .id;
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new(id),
                conversation: conversation.map(ConversationId::new),
                profile: recipe,
                machine: MachineId::new("mac-here"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: self.repo.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::from_millis(1_760_000_000_000),
                last_activity: Some(Timestamp::from_millis(1_760_000_001_000)),
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        drop(storage);
        relations::changed(&self.state, Change::SessionsUpdated);
    }

    fn count(&self, kind: EntityKind, id: &str) -> u64 {
        relations::counts(
            &self.state,
            &EntityRef {
                kind,
                id: id.to_owned(),
            },
        )
        .unwrap()
        .sessions
        .all
    }

    /// The ids of a Sessions list's rows (a conversation's, or AI
    /// Juggler's session's).
    fn page(&self, app: Option<&str>) -> BTreeSet<String> {
        let query = SessionsQuery {
            app_id: app.map(str::to_owned),
            ..SessionsQuery::default()
        };
        sessions_page_impl(
            &self.state,
            &query,
            &PageRequest {
                limit: Some(50),
                cursor: None,
            },
        )
        .unwrap()
        .items
        .into_iter()
        .map(|row| match (row.conversation, row.session) {
            (Some(conversation), _) => conversation.id,
            (None, Some(session)) => session.id,
            (None, None) => panic!("a row of neither"),
        })
        .collect()
    }

    fn apps(&self) -> Vec<String> {
        list_impl(&self.state)
            .unwrap()
            .into_iter()
            .map(|app| app.id)
            .collect()
    }

    /// No Account lists App `id` (each lists the Apps list's Apps on its
    /// folders).
    fn no_account_lists(&self, id: &str) {
        for account in
            juggler_app::features::subscriptions::list_with_apps_impl(&self.state).unwrap()
        {
            assert!(
                !account.apps.unwrap_or_default().iter().any(|app| app == id),
                "{} lists {id}",
                account.display_name
            );
        }
    }

    fn row(&self, id: &str) -> juggler_app::features::conversations::dto::ConversationInfo {
        juggler_app::features::conversations::commands::list_impl(&self.state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("{id} isn't listed"))
    }
}

/// A folder only scripts ran in (`claude -p`, the Python SDK, `codex
/// exec`, `codex mcp`): History has each, no App counts one, and neither
/// folder's CLI App is in the Apps list or any Account's. A terminal's
/// session and AI Juggler's own Codex thread then make each.
#[test]
fn a_folder_with_only_script_runs_has_no_cli_app() {
    let w = world();
    w.transcript("run-1", "sdk-cli");
    w.transcript("run-2", "sdk-py");
    let exec = w.rollout(1, r#""exec""#, "codex_exec", false);
    let mcp = w.rollout(2, r#""mcp""#, "codex_mcp", false);
    w.read();
    let (claude_cli, codex_cli) = (cli_app_id(&w.claude), cli_app_id(&w.codex));
    for id in ["run-1", "run-2", exec.as_str(), mcp.as_str()] {
        assert!(w.row(id).app_unlisted, "{id}: History only");
    }
    assert_eq!(w.page(None).len(), 4, "History has all four");
    for cli in [&claude_cli, &codex_cli] {
        assert_eq!(w.count(EntityKind::App, cli), 0, "{cli}");
        assert!(w.page(Some(cli)).is_empty(), "{cli}");
        assert!(!w.apps().contains(cli), "{cli}: {:?}", w.apps());
        w.no_account_lists(cli);
    }
    for (folder, n) in [(&w.claude, 2), (&w.codex, 2)] {
        assert_eq!(w.count(EntityKind::Folder, folder), n, "its folder has all");
    }

    // A terminal's session and AI Juggler's own Codex thread: each folder's
    // CLI App is one now, holding just that.
    w.transcript("term-1", "cli");
    let ours = w.rollout(3, r#""vscode""#, "ai_juggler", false);
    w.read();
    for (cli, id) in [(&claude_cli, "term-1"), (&codex_cli, ours.as_str())] {
        assert!(w.apps().contains(cli), "{cli}: {:?}", w.apps());
        assert_eq!(w.count(EntityKind::App, cli), 1, "{cli}");
        assert_eq!(w.page(Some(cli)), BTreeSet::from([id.to_owned()]));
    }
}

/// AI Juggler's own Claude sessions run as `claude -p --input-format
/// stream-json` (entrypoint `sdk-cli`): one with its AI Juggler session on
/// it isn't left out, and counts once, as that session. The same run with
/// no AI Juggler session on it is a script's.
#[test]
fn ai_juggler_s_own_claude_session_is_kept_and_counted_once() {
    let w = world();
    w.transcript("ours-1", "sdk-cli");
    w.transcript("script-1", "sdk-cli");
    w.own_session("s-ours", Some("ours-1"));
    w.read();
    let cli = cli_app_id(&w.claude);
    let ours = w.row("ours-1");
    assert_eq!(ours.session_id.as_deref(), Some("s-ours"));
    assert!(
        !ours.app_unlisted,
        "AI Juggler's own run is the user's session"
    );
    assert!(w.row("script-1").app_unlisted);
    assert!(w.apps().contains(&cli), "{:?}", w.apps());
    assert_eq!(w.count(EntityKind::App, &cli), 1, "once, as its session");
    assert_eq!(w.page(Some(&cli)), BTreeSet::from(["s-ours".to_owned()]));
    assert_eq!(
        w.page(None),
        BTreeSet::from(["s-ours".to_owned(), "script-1".to_owned()]),
        "History: the session once, and the script's run"
    );
}

/// The invariant: each Codex thread a Codex surface lists (source `cli`
/// or `vscode`, not archived, not a subagent's) is in exactly one App, by
/// its originator (the Codex app's own, `Codex Desktop` and its work
/// edition; Codex CLI's, the rest with AI Juggler's), so the Codex app's
/// count and Codex CLI's add up to the store's listed threads. Exec runs
/// are in History only; subagents' and archived threads in neither.
#[test]
fn each_codex_thread_is_in_one_app_and_the_apps_add_up_to_the_listed_threads() {
    let w = world();
    let subagent = r#"{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}"#;
    // (source, originator, archived, listed by a Codex surface)
    let kinds = [
        (r#""vscode""#, "Codex Desktop", false, true),
        (r#""vscode""#, "codex_work_desktop", false, true),
        (r#""vscode""#, "ai_juggler", false, true),
        (r#""cli""#, "codex_cli_rs", false, true),
        (r#""exec""#, "codex_exec", false, false),
        (subagent, "Codex Desktop", false, false),
        (r#""vscode""#, "Codex Desktop", true, false),
    ];
    let mut listed = BTreeSet::new();
    let mut desktop = 0;
    let mut exec = String::new();
    for (n, (source, originator, archived, shown)) in (1..).zip(kinds) {
        let id = w.rollout(n, source, originator, archived);
        if shown {
            listed.insert(id.clone());
            desktop += u64::from(originator.to_ascii_lowercase().contains("desktop"));
        }
        if originator == "codex_exec" {
            exec = id;
        }
    }
    w.read();
    let (app, cli) = (desktop_app_id("codex-app", None), cli_app_id(&w.codex));
    let (in_app, in_cli) = (w.page(Some(&app)), w.page(Some(&cli)));
    assert_eq!(
        (
            w.count(EntityKind::App, &app),
            w.count(EntityKind::App, &cli)
        ),
        (desktop, listed.len() as u64 - desktop),
        "the Codex app's count and Codex CLI's, by originator"
    );
    assert_eq!(
        w.count(EntityKind::App, &app) + w.count(EntityKind::App, &cli),
        listed.len() as u64,
        "they add up to the listed threads"
    );
    assert!(
        in_app.is_disjoint(&in_cli),
        "a thread in both: {in_app:?} {in_cli:?}"
    );
    assert_eq!(
        in_app.union(&in_cli).cloned().collect::<BTreeSet<_>>(),
        listed
    );
    assert!(w.apps().contains(&cli) && w.apps().contains(&app));
    // History: the listed threads and the exec run, nothing else.
    let mut history = listed.clone();
    history.insert(exec.clone());
    assert_eq!(w.page(None), history);
    assert!(w.row(&exec).app_unlisted);
}
