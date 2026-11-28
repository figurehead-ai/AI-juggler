//! Fixtures the crate's unit tests share (D-94: test setup in one place,
//! not copied per module).

use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
    UsageSnapshot,
};

/// A terminal session on "this-mac", started at a fixed time and active a
/// minute later, with nothing else known.
pub(crate) fn instance(
    id: &str,
    profile: &str,
    status: SessionStatus,
    cwd: std::path::PathBuf,
) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(profile),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd,
        status,
        started_at: Timestamp::from_millis(1_700_000_000_000),
        last_activity: Some(Timestamp::from_millis(1_700_000_060_000)),
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    }
}

/// An app state over `data` with no adapters registered: the plain state a
/// unit test reads and writes AI Juggler's own data through.
pub(crate) fn bare_state(data: std::path::PathBuf) -> std::sync::Arc<crate::state::AppState> {
    std::sync::Arc::new(
        crate::state::AppState::init_with_registry(data, juggler_adapter::AdapterRegistry::new())
            .expect("a bare app state"),
    )
}

/// The Claude Code setup at `config` brought in where it is, as Account
/// `label` (the conversations tests' own setup).
pub(crate) fn claude_in_place(
    state: &std::sync::Arc<crate::state::AppState>,
    config: &std::path::Path,
    label: &str,
) -> crate::features::account::dto::AccountInfo {
    crate::features::account::import::account_import_mode_impl(
        state,
        juggler_adapters::CLAUDE_CODE_ID,
        &config.to_string_lossy(),
        label,
        crate::features::account::import::ImportRequest {
            in_place: true,
            ..crate::features::account::import::ImportRequest::default()
        },
    )
    .expect("the setup brought in")
    .account
}

/// A plain Account: not imported, no plan, nothing opted in.
pub(crate) fn account(
    id: &str,
    harness: &str,
    label: &str,
    state_dir: std::path::PathBuf,
) -> juggler_core::model::Account {
    juggler_core::model::Account {
        id: juggler_core::model::AccountId::new(id),
        harness: juggler_core::model::HarnessId::new(harness),
        label: label.into(),
        plan: None,
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

/// Launch settings `id` named `name` on `account`, with no overlay or folder.
pub(crate) fn launch_settings(
    id: &str,
    name: &str,
    account: &str,
) -> juggler_core::model::LaunchRecipe {
    juggler_core::model::LaunchRecipe {
        id: ProfileId::new(id),
        name: name.into(),
        account: juggler_core::model::AccountId::new(account),
        settings_overlay: juggler_core::model::SettingsOverlay::default(),
        default_cwd: None,
    }
}

/// The Claude desktop app as a host: profiles by `--user-data-dir`, the
/// default one in Application Support, no links or scripts.
pub(crate) fn claude_app_host(bundle_id: Option<&str>) -> juggler_adapter::live::HostApp {
    juggler_adapter::live::HostApp {
        id: "claude-app".into(),
        name: "Claude".into(),
        bundle: "Claude.app".into(),
        bundle_id: bundle_id.map(Into::into),
        kind: juggler_adapter::live::HostKind::Desktop,
        instance_arg: Some("--user-data-dir".into()),
        default_instance: Some("Library/Application Support/Claude".into()),
        instance_lock: Some("Local Storage/leveldb/LOCK".into()),
        thread_link: None,
        folder_link: None,
        folder_command: None,
        tab_script: None,
        shown: None,
        siblings: false,
        folder_env: None,
        handoff_link: None,
        handoff_record: None,
        session_link: None,
        default_folder: None,
        desktop: juggler_adapter::desktop_app::DesktopFacts::default(),
    }
}

/// An app state over `data` with `adapters` registered (Claude Code's,
/// say): the state a test of a tool's own files works in.
pub(crate) fn state_with(
    data: std::path::PathBuf,
    adapters: Vec<juggler_adapter::SharedAdapter>,
) -> std::sync::Arc<crate::state::AppState> {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    for adapter in adapters {
        registry.register(adapter);
    }
    std::sync::Arc::new(crate::state::AppState::init_with_registry(data, registry).unwrap())
}

/// The terminal session `s-cli` (launch settings `rec`, "Main", on
/// Account `acc-cli`), running in `dir/w`.
pub(crate) fn cli_session(storage: &juggler_core::storage::Storage, dir: &std::path::Path) {
    storage
        .profiles()
        .insert(&launch_settings("rec", "Main", "acc-cli"))
        .unwrap();
    storage
        .sessions()
        .insert(&instance(
            "s-cli",
            "rec",
            SessionStatus::Running,
            dir.join("w"),
        ))
        .unwrap();
}

/// A Claude app profile as an App: its Account `id` (Claude Code's,
/// `label`, at `config`) and the desktop App on `instance` holding that
/// folder. The App's id.
pub(crate) fn desktop_app(
    storage: &juggler_core::storage::Storage,
    id: &str,
    label: &str,
    config: std::path::PathBuf,
    instance: &std::path::Path,
) -> String {
    storage
        .accounts()
        .insert(&account(
            id,
            juggler_adapters::CLAUDE_CODE_ID,
            label,
            config,
        ))
        .unwrap();
    let app = storage
        .apps()
        .upsert_desktop(
            juggler_adapters::CLAUDE_CODE_ID,
            "claude-app",
            Some(&instance.to_string_lossy()),
            1,
        )
        .unwrap();
    storage.apps().set_folder(&app, Some(id)).unwrap();
    // D-84, AJ-E1552: an App because the user added it (a folder link
    // alone never makes one).
    assert!(storage.apps().add(&app).unwrap());
    app
}

/// The facts an attestation keeps for `conversation`'s Code-tab session
/// in the profile of folder `folder`, of App `app`.
pub(crate) fn desktop_facts(
    app: &str,
    folder: &str,
    conversation: &str,
) -> juggler_core::pools::SessionFacts {
    juggler_core::pools::SessionFacts {
        app: Some(app.into()),
        tool: Some(juggler_adapters::CLAUDE_CODE_ID.into()),
        conversation: Some((folder.into(), conversation.into())),
        ..juggler_core::pools::SessionFacts::bare(
            juggler_core::storage::attested::desktop_id(folder, conversation),
            None,
        )
    }
}

/// What an attestation keeps in memory for that session: its App `app`,
/// its record `local_t1`, its tool's process `tool_pid` started at
/// `tool_start`.
pub(crate) fn keep_desktop(
    state: &crate::state::AppState,
    app: &str,
    folder: &str,
    conversation: &str,
    tool_pid: u32,
    tool_start: i64,
) {
    state.agent_bridge.desktop.lock().unwrap().insert(
        juggler_core::storage::attested::desktop_id(folder, conversation)
            .as_str()
            .to_owned(),
        (
            app.to_owned(),
            juggler_adapter::peer::AttestedSession {
                conversation: conversation.into(),
                host_session: "local_t1".into(),
                tool_pid,
                tool_start,
                cwd: None,
                title: None,
            },
        ),
    );
}

/// `text` written to `path` as an executable script (a stand-in helper
/// or tool); `path` back.
#[cfg(unix)]
pub(crate) fn script(path: &std::path::Path, text: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}

/// The files in `dir` (the database, its WAL) that hold `canary`.
pub(crate) fn holding(dir: &std::path::Path, canary: &str) -> Vec<std::path::PathBuf> {
    files_under(dir)
        .into_iter()
        .filter(|path| {
            std::fs::read(path)
                .unwrap_or_default()
                .windows(canary.len())
                .any(|w| w == canary.as_bytes())
        })
        .collect()
}

/// The Rust sources under `dir`.
pub(crate) fn rust_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    files_under(dir)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect()
}

/// Every file under `dir`, at any depth.
fn files_under(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    let mut out = Vec::new();
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// A state over `dir/data` with an account, an App and seven running
/// sessions, each in a folder named for it (`s1` is api … `s7` is cli).
pub(crate) fn seven_sessions(dir: &std::path::Path) -> std::sync::Arc<crate::state::AppState> {
    const NAMES: [&str; 7] = ["api", "web", "docs", "infra", "ui", "db", "cli"];
    let state = bare_state(dir.join("data"));
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&account("acct-1", "claude-code", "Lab", dir.join("acct")))
            .unwrap();
        storage
            .profiles()
            .insert(&launch_settings("p1", "Lab", "acct-1"))
            .unwrap();
        for (i, name) in NAMES.iter().enumerate() {
            storage
                .sessions()
                .insert(&instance(
                    &format!("s{}", i + 1),
                    "p1",
                    SessionStatus::Running,
                    dir.join(name),
                ))
                .unwrap();
        }
    }
    state
}

/// [`seven_sessions`]' sessions named as a user names them (each by its
/// folder's word: `s1` "api" … `s7` "cli"): what every list says of them
/// by the one short-name rule (0.5.2 G27; untitled, they're named by their
/// facts, "api · started 9:41 AM").
pub(crate) fn name_seven(state: &crate::state::AppState) {
    const NAMES: [&str; 7] = ["api", "web", "docs", "infra", "ui", "db", "cli"];
    let storage = state.storage.lock().unwrap();
    for (i, name) in NAMES.iter().enumerate() {
        storage
            .sessions()
            .set_label(
                &juggler_core::model::SessionId::new(format!("s{}", i + 1)),
                Some(name),
            )
            .unwrap();
    }
}

/// A Claude app profile AI Juggler knows, on folder `dir`, run from the
/// app in `/Applications` (the process-tree and ownership fixtures).
pub(crate) fn claude_profile(dir: &str) -> crate::features::places::book::KnownPlace {
    let app = "/Applications/Claude.app";
    crate::features::places::book::KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude".into(),
        instance: Some("work".into()),
        bundle: std::path::PathBuf::from(app),
        exe: std::path::PathBuf::from(format!("{app}/Contents/MacOS/Claude")),
        given: Some(std::path::PathBuf::from(dir)),
        instance_dir: Some(std::path::PathBuf::from(dir)),
        account_id: None,
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users: true,
        declined: false,
    }
}

/// Two Claude Code accounts, Work and Home, each with launch settings
/// "Main" (`true` stands in for the tool: nothing real runs), Home set up
/// as a signed-in account is (a switch checks, 0.3.2), and one
/// conversation `native` on Work in `root/site`, not yet listed.
pub(crate) struct WorkAndHome {
    pub(crate) state: std::sync::Arc<crate::state::AppState>,
    pub(crate) work: String,
    pub(crate) work_recipe: String,
    pub(crate) home_recipe: String,
}

/// [`WorkAndHome`] over `root` (canonical), its data in `root/app-data`.
pub(crate) fn work_and_home(root: &std::path::Path, native: &str) -> WorkAndHome {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    let claude = std::sync::Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
        "/usr/bin/true",
    ));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state = std::sync::Arc::new(
        crate::state::AppState::init_with_registry(root.join("app-data"), registry).unwrap(),
    );
    let tool = juggler_adapters::CLAUDE_CODE_ID;
    let work = crate::features::account::create_account(&state, tool, "Work", None).unwrap();
    let work_recipe =
        crate::features::profile::create_profile(&state, &work.id, "Main", None, None).unwrap();
    let home = crate::features::account::create_account(&state, tool, "Home", None).unwrap();
    let home_recipe =
        crate::features::profile::create_profile(&state, &home.id, "Main", None, None).unwrap();
    let home_dir = state.accounts_dir().join(tool).join(&home.id);
    std::fs::create_dir_all(&home_dir).unwrap();
    std::fs::write(
        home_dir.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
    let folder = root.join("site");
    std::fs::create_dir_all(&folder).unwrap();
    let transcripts = state
        .accounts_dir()
        .join(tool)
        .join(&work.id)
        .join("projects")
        .join(folder.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&transcripts).unwrap();
    let cwd = folder.display();
    std::fs::write(
        transcripts.join(format!("{native}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"sessionId\":\"{native}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"Fix the footer\"}}}}\n\
             {{\"type\":\"assistant\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"sessionId\":\"{native}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"Fixed.\"}}]}}}}\n"
        ),
    )
    .unwrap();
    WorkAndHome {
        state,
        work: work.id,
        work_recipe: work_recipe.id,
        home_recipe: home_recipe.id,
    }
}

/// 0.5.2 G45: conversation `id` of `tool` on `account`'s folder in the
/// index of `state`, as a pass reads it from its file, run in `cwd` and
/// last active at `last_ms` (a minute after it started); no AI Juggler
/// session is on it (one started in the user's own terminal, or by
/// another app).
pub(crate) fn outside_conversation(
    state: &std::sync::Arc<crate::state::AppState>,
    (account, tool): (&str, &str),
    id: &str,
    title: &str,
    cwd: &std::path::Path,
    last_ms: i64,
) {
    state
        .conversations
        .put_for_test(crate::features::conversations::Conversation {
            summary: juggler_adapter::conversation::ConversationSummary {
                id: id.into(),
                cwd: Some(cwd.to_path_buf()),
                title: Some(title.into()),
                started_ms: Some(last_ms - 60_000),
                last_ms: Some(last_ms),
                ..Default::default()
            },
            account_id: account.into(),
            tool: tool.into(),
            path: cwd.join(".lab-transcripts").join(format!("{id}.jsonl")),
            folder: Some(cwd.to_path_buf()),
        });
}

/// 0.5.2 G45: one live look, as the live thread makes one: these
/// conversations open in a terminal (`(account, tool, id, state)`, each on
/// its own made-up pid), every other one closed.
pub(crate) fn live_look(
    state: &std::sync::Arc<crate::state::AppState>,
    open: &[(
        &str,
        &str,
        &str,
        crate::features::conversations::scan::State,
    )],
) {
    use crate::features::conversations::scan::{LiveEntry, LiveModel, Place};
    let entries = open
        .iter()
        .zip(990_201_u32..)
        .map(|(&(account, tool, id, state), pid)| LiveEntry {
            tool: tool.into(),
            account_id: Some(account.into()),
            session_id: id.into(),
            places: vec![Place {
                pid,
                start: None,
                host: None,
                profile: None,
                app_session: None,
            }],
            state: Some(state),
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        })
        .collect();
    crate::features::conversations::ConversationIndex::look_for_test(
        state,
        LiveModel {
            entries,
            ..LiveModel::default()
        },
    )
    .expect("a live look");
}

/// 0.5.2 (PR #257 CI): the name an untitled session takes by its facts
/// (D-96, the rail's one rule) as this system says it: "<folder> ·
/// started <time>" where the system's clock gives the words (macOS), the
/// folder alone where it can't (`juggler_platform::clock::started_words`
/// is `None` there). Built from the same platform words the naming uses,
/// so a test holds on every system.
pub(crate) fn facts_name(folder: &str, started_ms: i64, now_ms: i64) -> String {
    juggler_platform::clock::started_words(started_ms, now_ms)
        .map_or_else(|| folder.to_owned(), |words| format!("{folder} · {words}"))
}

/// [`facts_name`] for session `id` as storage holds it (its own start),
/// now.
pub(crate) fn facts_name_of(state: &crate::state::AppState, id: &str, folder: &str) -> String {
    let started = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&juggler_core::model::SessionId::new(id))
        .unwrap()
        .started_at
        .millis();
    facts_name(folder, started, crate::commands::now_millis())
}
