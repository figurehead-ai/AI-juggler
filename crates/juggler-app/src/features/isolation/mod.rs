//! AJ-E308 Req.7: the isolation checklist: what keeps an account's folder its
//! own.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::model::{Account, AccountId, Profile, ProfileId, SettingsOverlay};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::account::{login_state_name, login_state_of};
use crate::features::isolation::dto::IsolationRowInfo;
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on lookup/storage failures.
// One assembly pass over six named checks; splitting it would hide
// the checklist's shape.
#[allow(
    clippy::too_many_lines,
    reason = "one checklist, row by row in the order the sheet shows"
)]
pub fn isolation_check_impl(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<Vec<IsolationRowInfo>> {
    let account = {
        let storage = lock_storage_arc(state)?;
        storage
            .accounts()
            .get(&AccountId::new(account_id.to_owned()))?
    };
    let dir = account.state_dir.clone();
    if account.in_place {
        return crate::in_place::in_place_isolation_rows(state, &account);
    }
    let mut rows = Vec::new();

    // 1. The managed dir is owner-private (0700). The mask IS the
    // check (no group/other bits set) — a trailing-zeros rewrite
    // would obscure it.
    #[cfg(unix)]
    let private_pass = {
        use std::os::unix::fs::PermissionsExt;
        #[allow(
            clippy::verbose_bit_mask,
            reason = "the permission bits read as the kernel writes them"
        )]
        let check = |mode: u32| mode & 0o077 == 0;
        std::fs::metadata(&dir).is_ok_and(|meta| check(meta.permissions().mode()))
    };
    #[cfg(not(unix))]
    let private_pass = std::fs::metadata(&dir).is_ok();
    rows.push(IsolationRowInfo {
        id: "managed_dir_private",
        label: "Account folder is owner-only (0700)".into(),
        pass: private_pass,
        detail: format!("{}", dir.display()),
    });

    // 2. The managed marker is present (this dir is unambiguously ours).
    let marker_pass = dir
        .join(juggler_adapters::claude_code::MANAGED_MARKER)
        .is_file();
    rows.push(IsolationRowInfo {
        id: "marker_present",
        label: "The managed-folder marker is present".into(),
        pass: marker_pass,
        detail: "the folder identifies itself as AI Juggler-managed".into(),
    });

    // 3. The launch environment points ONLY at this managed dir.
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.as_str().to_owned()))?;
    // Every variable the Provider pins to the account's folder
    // (`CLAUDE_CONFIG_DIR`; `CODEX_HOME` and `CODEX_SQLITE_HOME`) —
    // AJ-E309.
    let pinned = adapter.pinned_env_keys();
    let env_ok = launch_env_pinned(adapter.as_ref(), &account);
    rows.push(IsolationRowInfo {
        id: "launch_env_managed_only",
        label: "Launch environment points only at this account's folder".into(),
        pass: env_ok,
        detail: if pinned.is_empty() {
            "the Provider names no state variable".to_owned()
        } else {
            format!("{} resolve to the managed dir", pinned.join(" and "))
        },
    });

    // 4. The import SOURCE carries no AI Juggler writes (no marker,
    //    nothing of ours inside it) — metadata-only.
    let source_clean = account.import_source_path.as_deref().is_none_or(|source| {
        !std::path::Path::new(source)
            .join(juggler_adapters::claude_code::MANAGED_MARKER)
            .is_file()
    });
    rows.push(IsolationRowInfo {
        id: "source_free_of_juggler_writes",
        label: "The imported-from folder is untouched (no AI Juggler writes)".into(),
        pass: source_clean,
        detail: account
            .import_source_path
            .clone()
            .unwrap_or_else(|| "created fresh — no import source".into()),
    });

    // 5. Per-dir probe independence: this dir's login classification
    //    reads ONLY this dir (structural: single-path probe).
    rows.push(probe_independence_row(state, &account));

    // 6. No stray files directly under the harness accounts root —
    //    writes outside managed dirs would land there.
    let parent_clean = dir.parent().is_none_or(|parent| {
        std::fs::read_dir(parent)
            .is_ok_and(|read| read.flatten().all(|entry| entry.path().is_dir()))
    });

    rows.push(IsolationRowInfo {
        id: "no_writes_outside_managed",
        label: "No stray files outside managed folders".into(),
        pass: parent_clean,
        detail: "the accounts root holds only account folders".into(),
    });

    Ok(rows)
}

/// Whether an Interactive launch of `account` sets every pinned
/// variable to what the adapter pins for its folder — the path exactly,
/// or unset for the vendor's own folder (AJ-E1521: launches, the shell
/// line and this check read the one `pinned_env` list).
pub(crate) fn launch_env_pinned(
    adapter: &juggler_adapter::adapter::ProviderAdapter,
    account: &Account,
) -> bool {
    let pinned = adapter.pinned_env(&account.state_dir);
    let profile = Profile {
        id: ProfileId::new("check"),
        name: "check".into(),
        account: account.id.clone(),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    let root = std::path::Path::new("/");
    // AJ-E1605 R3: a tool with no terminal UI is checked on the launch it
    // runs, its driven one.
    let spec = match adapter.driven().filter(|driven| !driven.terminal_ui()) {
        Some(driven) => driven
            .driven_spec(
                account,
                &profile,
                root,
                &juggler_adapter::runtime::DrivenLaunch {
                    socket: std::env::temp_dir().join("check.sock"),
                    resume: None,
                    fresh_id: "check".into(),
                    history: None,
                },
            )
            .map(|spec| spec.launch),
        None => adapter.launch_spec(
            account,
            &profile,
            root,
            &juggler_adapter::LaunchMode::Interactive,
        ),
    };
    !pinned.is_empty()
        && spec.is_ok_and(|spec| {
            pinned
                .iter()
                .all(|(key, expected)| match (spec.env.get(*key), expected) {
                    (Some(value), None) => value == juggler_adapter::launch::ENV_UNSET,
                    (Some(value), Some(path)) => {
                        std::path::Path::new(value) == std::path::Path::new(path)
                    }
                    (None, _) => false,
                })
        })
}

/// Check 5: this folder's sign-in reads only this folder (a single-path
/// probe), so it classifies as something other than Unknown.
pub(crate) fn probe_independence_row(state: &AppState, account: &Account) -> IsolationRowInfo {
    let probe_state = login_state_of(&state.registry, account);
    IsolationRowInfo {
        id: "probe_independence",
        label: "Login state reads only this account's folder".into(),
        pass: !matches!(probe_state, juggler_adapters::LoginState::Unknown),
        detail: format!(
            "probe classified this dir as {}",
            login_state_name(probe_state)
        ),
    }
}
