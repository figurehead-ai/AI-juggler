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
    }
}
