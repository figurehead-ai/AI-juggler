//! Pairing (AJ-E703): this Mac hosting a pairing code and deciding who
//! joins, and joining another Mac's — the host and join phases the UI
//! drives. Split from `link` (the size budget).

#[allow(clippy::wildcard_imports)] // the link manager's own types
use super::*;

/// Core: opens the host pairing window ("Add a machine").
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Link is off or pairing is locked.
pub fn pairing_host_open_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    if !link_status_impl(state).enabled {
        return Err(AppError::Validation {
            subject: "pairing",
            message: "Turn Link on first.".to_owned(),
        });
    }
    let manager = &state.link;
    let opened = {
        let mut guard = manager.guard.lock().map_err(|_| AppError::StatePoisoned)?;
        guard
            .get_or_insert_with(|| PairingGuard::new(SettingsGuardStore(Arc::downgrade(state))))
            .open(now_ms())
    };
    if let Err(error) = opened {
        crate::usage_stats::fleet_pairing(Some(error.code()));
        *manager.host.lock().map_err(|_| AppError::StatePoisoned)? = HostPhase::Failed {
            reason: error.code().to_owned(),
            message: error.to_string(),
        };
        return pairing_host_state_impl(state);
    }
    if let Ok(mut message) = manager.host_message.lock() {
        *message = None;
    }
    *manager.host.lock().map_err(|_| AppError::StatePoisoned)? = HostPhase::Waiting;
    pairing_host_state_impl(state)
}

/// Core: the host's pairing state (the UI polls it).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn pairing_host_state_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    let manager = &state.link;
    let message = manager
        .host_message
        .lock()
        .ok()
        .and_then(|message| message.clone());
    let fingerprint = crate::fleet::fleet_self_impl(state)
        .map(|info| info.fingerprint)
        .unwrap_or_default();
    let phase = manager.host.lock().map_err(|_| AppError::StatePoisoned)?;
    let mut info = PairingStateInfo::idle();
    if let Some((reason, message)) = message {
        info.reason = Some(reason);
        info.message = Some(message);
    }
    match &*phase {
        HostPhase::Idle => {}
        HostPhase::Failed { reason, message } => {
            info.phase = String::from("failed");
            info.reason = Some(reason.clone());
            info.message = Some(message.clone());
        }
        HostPhase::Waiting => {
            let (code, opened) = manager
                .guard
                .lock()
                .ok()
                .and_then(|guard| {
                    guard
                        .as_ref()
                        .and_then(|g| g.current_code(now_ms()).map(|c| (c, g.opened_at_ms())))
                })
                .unzip();
            if let Some(code) = code {
                info.phase = String::from("waiting");
                info.link = Some(PairingInput::link(&code, &fingerprint));
                info.code = Some(code.display());
                info.expires_at_ms = opened.flatten().map(|at| at + PAIRING_WINDOW_MS);
            } else {
                info.phase = String::from("failed");
                info.reason = Some("expired".to_owned());
                info.message = Some("The pairing window closed after 2 minutes.".to_owned());
            }
        }
        HostPhase::Confirm { candidate, .. } => {
            info.phase = String::from("confirm");
            info.peer_label = Some(candidate.label.clone());
            info.peer_fingerprint = Some(candidate.fingerprint.clone());
        }
        HostPhase::Paired { label } => {
            info.phase = String::from("paired");
            info.peer_label = Some(label.clone());
        }
    }
    Ok(info)
}

/// Core: this Mac's user confirms (with the permission to grant) or
/// cancels the machine shown.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when nothing awaits a decision.
pub fn pairing_host_decide_impl(
    state: &Arc<AppState>,
    commit: bool,
    permission: Permission,
) -> AppResult<PairingStateInfo> {
    respond(&state.link.host, commit, permission, |phase| match phase {
        HostPhase::Confirm { respond, .. } => respond.take(),
        _ => None,
    })?;
    pairing_host_state_impl(state)
}

fn respond<T>(
    phase: &Arc<Mutex<T>>,
    commit: bool,
    permission: Permission,
    take: impl FnOnce(&mut T) -> Option<tokio::sync::oneshot::Sender<Option<Permission>>>,
) -> AppResult<()> {
    let sender =
        take(&mut *phase.lock().map_err(|_| AppError::StatePoisoned)?).ok_or_else(|| {
            AppError::Validation {
                subject: "pairing",
                message: "Nothing is waiting for a decision.".to_owned(),
            }
        })?;
    let _ = sender.send(commit.then_some(permission));
    Ok(())
}

/// Core: closes the host window (Cancel / leaving Fleet).
pub fn pairing_host_close_impl(state: &Arc<AppState>) {
    let manager = &state.link;
    if let Ok(mut guard) = manager.guard.lock()
        && let Some(guard) = guard.as_mut()
    {
        guard.close();
    }
    if let Ok(mut phase) = manager.host.lock() {
        if let HostPhase::Confirm { respond, .. } = &mut *phase
            && let Some(sender) = respond.take()
        {
            let _ = sender.send(None);
        }
        *phase = HostPhase::Idle;
    }
}

/// Core: re-enables pairing after the daily disable (a user click).
pub fn pairing_re_enable_impl(state: &Arc<AppState>) {
    let mut guard = state.link.guard.lock().ok();
    if let Some(guard) = guard.as_mut() {
        guard
            .get_or_insert_with(|| PairingGuard::new(SettingsGuardStore(Arc::downgrade(state))))
            .re_enable();
    }
}

/// Where to reach the host: the typed address, or the discovered node's.
fn join_addresses(
    state: &Arc<AppState>,
    instance_id: Option<&str>,
    address: Option<&str>,
) -> AppResult<Vec<SocketAddr>> {
    if let Some(address) = address {
        let parsed = address
            .trim()
            .parse::<SocketAddr>()
            .or_else(|_| format!("{}:{}", address.trim(), preferred_port()).parse())
            .map_err(|_| AppError::Validation {
                subject: "address",
                message: "Use an IP address, optionally with :port (for example 100.64.0.2)."
                    .to_owned(),
            })?;
        Ok(vec![parsed])
    } else {
        let running = state
            .link
            .running
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        let browser = running
            .as_ref()
            .and_then(|running| running.browser.as_ref())
            .ok_or_else(|| AppError::Validation {
                subject: "pairing",
                message: "This Mac can't see the local network — pair by address instead."
                    .to_owned(),
            })?;
        let wanted = instance_id.unwrap_or_default();
        let node = browser
            .nodes()
            .into_iter()
            .find(|node| node.instance_id == wanted)
            .ok_or_else(|| AppError::Validation {
                subject: "pairing",
                message: "That machine is no longer on the network.".to_owned(),
            })?;
        Ok(ordered(&node))
    }
}

/// Core: starts joining a host found on the LAN (by instance id) or at
/// an address (`host:port`, e.g. over the user's VPN).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a bad code/address or Link off.
pub fn pairing_join_start_impl(
    state: &Arc<AppState>,
    instance_id: Option<&str>,
    address: Option<&str>,
    typed: &str,
) -> AppResult<PairingStateInfo> {
    let input = PairingInput::parse(typed).map_err(|_| AppError::Validation {
        subject: "pairing code",
        message: "Enter the 6-digit code shown on the other Mac.".to_owned(),
    })?;
    let manager = &state.link;
    // Held until the join is recorded, so Link can't go off in between
    // (it would find no join to end).
    let _lifecycle = manager
        .lifecycle
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let run = link_run(state);
    let Some(own_port) = link_status_impl(state).port else {
        return Err(AppError::Validation {
            subject: "pairing",
            message: "Turn Link on first.".to_owned(),
        });
    };
    let addresses = join_addresses(state, instance_id, address)?;
    let certificate = crate::fleet::node_certificate(state)?;
    let key = crate::fleet::local_public_key(state)?;
    let own = crate::commands::local_machine_id(state).to_string();
    let label = crate::fleet::fleet_self_impl(state)?.label;
    // A newer join replaces one still in flight, which ends first, so it
    // can't write "failed" over this one's state.
    let older = manager
        .join_task
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .take();
    if let Some(older) = older {
        older.abort();
        let _ = manager.runtime()?.block_on(older);
    }
    *manager.join.lock().map_err(|_| AppError::StatePoisoned)? = JoinPhase::Connecting;
    let join = manager.join.clone();
    let weak = Arc::downgrade(state);
    let task = manager.runtime()?.spawn(async move {
        let mut last = JoinError::Cancelled;
        for address in addresses {
            let phase = join.clone();
            let result = pair_with(
                address,
                juggler_link::pairing_rpc::Joiner {
                    certificate: &certificate,
                    key,
                    machine_id: &own,
                    label: &label,
                    port: own_port,
                },
                &input,
                move |candidate| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    if let Ok(mut phase) = phase.lock() {
                        *phase = JoinPhase::Confirm {
                            candidate,
                            respond: Some(tx),
                        };
                    }
                    async move { rx.await.ok().flatten() }
                },
            )
            .await;
            match result {
                Ok(joined) => {
                    let Some(app) = weak.upgrade() else {
                        return;
                    };
                    // Only while this run's Link is on: the pin is the effect.
                    let Some(open) = link_open(&app, run) else {
                        return;
                    };
                    let _ = crate::fleet::pin_candidate(&app, &joined.candidate, joined.permission);
                    SettingsReplayStore(weak.clone()).reset(&joined.candidate.machine_id);
                    remember_address(&app, &joined.candidate.machine_id, address);
                    mark_reconnect(&app, &joined.candidate.machine_id);
                    drop(open);
                    if let Ok(mut phase) = join.lock() {
                        *phase = JoinPhase::Paired {
                            label: joined.candidate.label,
                        };
                    }
                    crate::usage_stats::fleet_pairing(None);
                    return;
                }
                Err(JoinError::Connect(reason)) => last = JoinError::Connect(reason),
                Err(other) => {
                    last = other;
                    break;
                }
            }
        }
        crate::usage_stats::fleet_pairing(Some(last.reason_code()));
        if let Ok(mut phase) = join.lock() {
            *phase = JoinPhase::Failed {
                reason: last.reason_code().to_owned(),
                message: last.to_string(),
            };
        }
    });
    *manager
        .join_task
        .lock()
        .map_err(|_| AppError::StatePoisoned)? = Some(task);
    pairing_join_state_impl(state)
}

/// Core: the joiner's pairing state (the UI polls it).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn pairing_join_state_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    let phase = state
        .link
        .join
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let mut info = PairingStateInfo::idle();
    match &*phase {
        JoinPhase::Idle => {}
        JoinPhase::Connecting => info.phase = String::from("connecting"),
        JoinPhase::Confirm { candidate, .. } => {
            info.phase = String::from("confirm");
            info.peer_label = Some(candidate.label.clone());
            info.peer_fingerprint = Some(candidate.fingerprint.clone());
        }
        JoinPhase::Paired { label } => {
            info.phase = String::from("paired");
            info.peer_label = Some(label.clone());
        }
        JoinPhase::Failed { reason, message } => {
            info.phase = String::from("failed");
            info.reason = Some(reason.clone());
            info.message = Some(message.clone());
        }
    }
    Ok(info)
}

/// Core: this Mac's user confirms or cancels the host shown.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when nothing awaits a decision.
pub fn pairing_join_decide_impl(
    state: &Arc<AppState>,
    commit: bool,
    permission: Permission,
) -> AppResult<PairingStateInfo> {
    respond(&state.link.join, commit, permission, |phase| match phase {
        JoinPhase::Confirm { respond, .. } => respond.take(),
        _ => None,
    })?;
    pairing_join_state_impl(state)
}

/// Core: resets the joiner state (Done / Start over).
pub fn pairing_join_reset_impl(state: &Arc<AppState>) {
    if let Ok(mut phase) = state.link.join.lock() {
        if let JoinPhase::Confirm { respond, .. } = &mut *phase
            && let Some(sender) = respond.take()
        {
            let _ = sender.send(None);
        }
        *phase = JoinPhase::Idle;
    }
}

fn parse_permission(wire: &str) -> AppResult<Permission> {
    match wire {
        "view" => Ok(Permission::View),
        "control" => Ok(Permission::Control),
        _ => Err(AppError::Validation {
            subject: "permission",
            message: "Choose View only or View + control.".to_owned(),
        }),
    }
}

// --- the commands ------------------------------------------------------------

/// AJ-E704: opens "Add a machine".
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_host_open_impl`].
#[tauri::command]
pub fn pairing_host_open(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_host_open_impl(state.inner())
}

/// AJ-E704: the host's pairing state.
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn pairing_host_state(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_host_state_impl(state.inner())
}

/// AJ-E704: the host user's decision.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_host_decide_impl`].
#[tauri::command]
pub fn pairing_host_decide(
    state: tauri::State<'_, Arc<AppState>>,
    commit: bool,
    permission: String,
) -> AppResult<PairingStateInfo> {
    pairing_host_decide_impl(state.inner(), commit, parse_permission(&permission)?)
}

/// AJ-E704: closes "Add a machine".
#[tauri::command]
pub fn pairing_host_close(state: tauri::State<'_, Arc<AppState>>) {
    pairing_host_close_impl(state.inner());
}

/// AJ-E704: re-enables pairing after the daily disable.
#[tauri::command]
pub fn pairing_re_enable(state: tauri::State<'_, Arc<AppState>>) {
    pairing_re_enable_impl(state.inner());
}

/// AJ-E704: starts joining a host.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_join_start_impl`].
#[tauri::command]
pub async fn pairing_join_start(
    state: tauri::State<'_, Arc<AppState>>,
    instance_id: Option<String>,
    address: Option<String>,
    code: String,
) -> AppResult<PairingStateInfo> {
    // It waits its turn with Link on/off and for a join it replaces:
    // never on the main thread or an async worker.
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        pairing_join_start_impl(&state, instance_id.as_deref(), address.as_deref(), &code)
    })
    .await
    .map_err(|e| AppError::Io {
        context: "pairing",
        message: e.to_string(),
    })?
}

/// AJ-E704: the joiner's pairing state.
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn pairing_join_state(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_join_state_impl(state.inner())
}

/// AJ-E704: the joiner user's decision.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_join_decide_impl`].
#[tauri::command]
pub fn pairing_join_decide(
    state: tauri::State<'_, Arc<AppState>>,
    commit: bool,
    permission: String,
) -> AppResult<PairingStateInfo> {
    pairing_join_decide_impl(state.inner(), commit, parse_permission(&permission)?)
}

/// AJ-E704: resets the joiner state.
#[tauri::command]
pub fn pairing_join_reset(state: tauri::State<'_, Arc<AppState>>) {
    pairing_join_reset_impl(state.inner());
}
