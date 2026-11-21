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
