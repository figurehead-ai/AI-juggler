//! AJ-E302: the ten-signal scan (pass2/03 §3 S1 to S10): parallel probes off
//! the UI thread, per-probe health and a cancel flag. Every probe is
//! metadata-only.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::AppResult;
use crate::features::environment::dto::{ProbeInfo, ScanReportInfo};
use crate::features::tools::scan_wire::{DiscoveredInfo, discovered_info_of, inference_info_of};
use crate::state::AppState;

/// The scan's cancellation flag (one scan at a time in practice; the
/// wizard sets it on close/unmount).
pub(crate) static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

/// The ten probe labels (progress: "n of 10 checks").
const PROBE_LABELS: [(&str, &str); 10] = [
    ("s1-config-dirs", "Folders named .claude…"),
    (
        "s2-env-overrides",
        "A config folder set for this app (CLAUDE_CONFIG_DIR)",
    ),
    ("s3-manager-t3", "Claude profiles (ClaudeCodeProfiles)"),
    ("s4-manager-cc-switch", "cc-switch profiles"),
    (
        "s5-shell-aliases",
        "Your shell settings (aliases, CLAUDE_CONFIG_DIR)",
    ),
    ("s6-installed-clis", "Claude Code installed"),
    ("s7-login-metadata", "Sign-in status"),
    ("s8-usage-recency", "When each was last used"),
    ("s9-projects", "Recent projects"),
    ("s10-desktop", "Desktop apps"),
];

#[allow(
    clippy::needless_pass_by_value,
    reason = "its signature as it was in commands.rs; AJ-E2017 moved it unchanged"
)]
pub(crate) fn probe(
    id: &str,
    health: juggler_adapter::environment::ScanHealth,
    detail: String,
) -> ProbeInfo {
    ProbeInfo {
        id: id.to_owned(),
        label: PROBE_LABELS
            .iter()
            .find(|(pid, _)| *pid == id)
            .map_or_else(|| id.to_owned(), |(_, label)| (*label).to_owned()),
        health: match &health {
            juggler_adapter::environment::ScanHealth::Ok => "ok".to_owned(),
            juggler_adapter::environment::ScanHealth::Partial(_) => "partial".to_owned(),
            juggler_adapter::environment::ScanHealth::Denied(_) => "denied".to_owned(),
        },
        reason: health.reason().map(ToOwned::to_owned),
        detail,
    }
}

/// Where each found setup is open right now: one process-table read (no
/// environments, AJ-E2101), each setup's own session files naming its
/// pids.
fn mark_in_use(state: &Arc<AppState>, installs: &mut [DiscoveredInfo]) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let mut processes = crate::running_setups::running_processes();
    let adapters: Vec<juggler_adapter::SharedAdapter> = state
        .registry
        .ids()
        .iter()
        .filter_map(|id| state.registry.get(id))
        .collect();
    let dirs: Vec<std::path::PathBuf> = installs
        .iter()
        .map(|install| std::path::PathBuf::from(&install.path))
        .collect();
    let pid_dirs = crate::running_setups::pid_dirs_in(&adapters, &dirs, &processes);
    let shapes = crate::running_setups::ToolShapes::from_adapters(&adapters);
    crate::running_setups::with_folders(&mut processes, &shapes, &pid_dirs);
    let open = crate::running_setups::classify(&processes, &shapes, &home);
    for install in installs {
        install.in_use = crate::running_setups::in_use_for(&open, &install.path, &home);
    }
}

/// Core implementation shared with integration tests. `progress` is
/// the optional UI sink (done, total) — the command layer wires it to
/// Tauri events.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn environment_scan_full_impl(
    state: &Arc<AppState>,
    progress: Option<&dyn Fn(u32, u32)>,
) -> AppResult<ScanReportInfo> {
    SCAN_CANCEL.store(false, Ordering::SeqCst);
    let total = u32::try_from(PROBE_LABELS.len()).unwrap_or_default();
    let mut done = 0_u32;
    let tick = |done: &mut u32| {
        *done += 1;
        if let Some(sink) = progress {
            sink(*done, total);
        }
    };

    // The parallel walk feeds S1/S2/S3/S6/S7/S8; S5/S4/S9/S10 are the
    // folds below. Cancellation stops between groups.
    let (found_installs, alias_rows, managers, discoveries) = scan_walk(state);
    let cancelled = SCAN_CANCEL.load(Ordering::SeqCst);
    tick(&mut done); // S1 — the walk
    tick(&mut done); // S2 — env overrides (app env, cheap)
    tick(&mut done); // S7 — login metadata
    tick(&mut done); // S8 — usage recency
    tick(&mut done); // S3 — manager t3

    let mut installs: Vec<DiscoveredInfo> =
        found_installs.into_iter().map(discovered_info_of).collect();

    // S5: assignments whose target exists become installs (a
    // forgotten manual setup); dangling ones stay hints.
    let (alias_installs, _alias_health, dangling) = alias_installs_of(&alias_rows, &mut installs);
    installs.extend(alias_installs);
    tick(&mut done); // S5

    // ADR-0018: a folder an in-place account uses is already in, though
    // it carries no marker (nothing is written into it).
    if let Ok(storage) = state.storage.lock()
        && let Ok(accounts) = storage.accounts().list()
    {
        let canonical =
            |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        // …and so is the source of a copy: bringing it in again (either
        // way) is refused, so it isn't offered (review of ADR-0018).
        let already: Vec<std::path::PathBuf> = accounts
            .iter()
            .flat_map(|account| {
                let source = account
                    .import_source_path
                    .as_deref()
                    .map(std::path::Path::new)
                    .map(canonical);
                let own = account.in_place.then(|| canonical(&account.state_dir));
                source.into_iter().chain(own)
            })
            .collect();
        for install in &mut installs {
            if already.contains(&canonical(std::path::Path::new(&install.path))) {
                install.managed = true;
            }
        }
    }

    mark_in_use(state, &mut installs);

    // S4: the cc-switch health rides its manager hint.
    let cc_health = managers
        .iter()
        .find(|manager| manager.manager == "cc-switch")
        .map_or_else(
            || juggler_adapter::environment::ScanHealth::Ok,
            |manager| manager.health.clone(),
        );
    tick(&mut done); // S4
    tick(&mut done); // S6 (the --version runs happened in the walk)

    let config_dirs: Vec<std::path::PathBuf> = installs
        .iter()
        .map(|install| std::path::PathBuf::from(&install.path))
        .collect();
    let seeds = juggler_adapters::scan_signals::project_seed_names(&config_dirs);
    tick(&mut done); // S9
    // The tools' own desktop apps, from their adapters (ADR-0019).
    let desktop_apps = juggler_adapters::scan_signals::desktop_apps_present(
        std::path::Path::new("/Applications"),
        &crate::running_setups::ToolShapes::of(&state.registry).app_bundles(),
    );
    tick(&mut done); // S10

    let probes = scan_probe_rows(
        &installs,
        &alias_rows,
        dangling,
        &managers,
        cc_health,
        &discoveries,
        &seeds,
        &desktop_apps,
        cancelled,
    );
    if !cancelled {
        crate::usage_stats::scan_completed(&installs, &managers);
    }

    Ok(ScanReportInfo {
        installs,
        probes,
        seeds,
        desktop_apps,
        cancelled,
    })
}

/// The parallel metadata walk: per adapter, the config-dir scan, the
/// rc assignments, the manager hints, and the `--version` probe all
/// run on their own threads (off the UI thread; ≤ one subprocess per
/// adapter).
/// The parallel walk's collected parts: installs, alias hints,
/// manager hints, and per-adapter CLI presence.
type ScanWalk = (
    Vec<juggler_adapter::environment::DiscoveredInstall>,
    Vec<juggler_adapter::environment::AliasHint>,
    Vec<juggler_adapter::environment::ManagerHint>,
    Vec<(String, &'static str)>,
);

fn scan_walk(state: &Arc<AppState>) -> ScanWalk {
    let mut found_installs = Vec::new();
    let mut alias_rows = Vec::new();
    let mut managers = Vec::new();
    let mut discoveries = Vec::new();
    let probes = crate::features::tools::per_adapter(&state.registry, |_, adapter| {
        let Some(adapter) = adapter else {
            return (Vec::new(), Vec::new(), Vec::new(), None);
        };
        let installs = adapter.scan_environment();
        let aliases = adapter.scan_shell_aliases();
        let manager_hints = adapter.scan_managers();
        let report = crate::features::tools::discover(state, adapter.as_ref());
        let words = if report.is_installed() {
            "installed"
        } else if matches!(report.install, juggler_adapter::InstallStatus::NotAsked) {
            "found; its version isn't known"
        } else if report.is_found() {
            "found; couldn't check its version"
        } else {
            "missing"
        };
        (
            installs,
            aliases,
            manager_hints,
            Some((adapter.id().to_string(), words)),
        )
    });
    for (installs, aliases, mgrs, discovery) in probes {
        found_installs.extend(installs);
        alias_rows.extend(aliases);
        managers.extend(mgrs);
        discoveries.extend(discovery);
    }
    (found_installs, alias_rows, managers, discoveries)
}

/// The Provider whose state dir a shell variable points at (S5).
fn provider_of_variable(variable: &str) -> &'static str {
    match variable {
        juggler_adapters::codex::ENV_CODEX_HOME => juggler_adapters::CODEX_ID,
        _ => juggler_adapters::CLAUDE_CODE_ID,
    }
}

/// S5 + the duplicate merge (Req.2): an alias whose target exists
/// MERGES into the known candidate as an extra signal (alias +
/// home-pattern → ONE candidate, signals listed); only a target no
/// other signal found becomes a new candidate. Dangling rows are
/// counted for the probe health.
pub fn alias_installs_of(
    alias_rows: &[juggler_adapter::environment::AliasHint],
    known: &mut [DiscoveredInfo],
) -> (
    Vec<DiscoveredInfo>,
    juggler_adapter::environment::ScanHealth,
    u32,
) {
    let mut out = Vec::new();
    let mut dangling = 0_u32;
    for hint in alias_rows {
        let path = hint.path.to_string_lossy().into_owned();
        // AJ-E1519: another manager's profile names its file, not a line.
        let signal = match &hint.set_by {
            Some(_) => format!("{} in {}", hint.variable, hint.source_file),
            None => format!("{} in {}:{}", hint.variable, hint.source_file, hint.line),
        };
        if !hint.target_exists {
            dangling += 1;
            continue;
        }
        // The duplicate merge: an existing candidate for the same dir
        // gains the alias as a signal — never a second card.
        if let Some(existing) = known.iter_mut().find(|install| install.path == path) {
            if !existing.signals.contains(&signal) {
                existing.signals.push(signal);
            }
            continue;
        }
        let harness = provider_of_variable(&hint.variable);
        let origin = if hint.set_by.is_some() {
            "other-manager"
        } else {
            "shell-alias"
        };
        let credentials_len =
            juggler_adapters::sign_in::metadata(harness, &hint.path).map(|meta| meta.len());
        out.push(DiscoveredInfo {
            inference: inference_info_of(&hint.path, false, credentials_len, None, (false, false)),
            set_by: hint.set_by.clone(),
            signals: vec![origin.to_owned()],
            harness: harness.to_owned(),
            path,
            origin: origin.to_owned(),
            login: "logged_out".to_owned(),
            bytes: 0,
            last_used_secs: None,
            managed: false,
            in_use: Vec::new(),
            notes: vec![match &hint.set_by {
                Some(manager) => format!(
                    "Set by {} in {manager}'s settings — {}.",
                    hint.variable, hint.source_file
                ),
                None => format!(
                    "Set by {} in your shell — {}:{}.",
                    hint.variable, hint.source_file, hint.line
                ),
            }],
        });
    }
    let health = if dangling > 0 {
        juggler_adapter::environment::ScanHealth::Partial(format!(
            "{dangling} assignment(s) point at directories that no longer exist"
        ))
    } else {
        juggler_adapter::environment::ScanHealth::Ok
    };
    (out, health, dangling)
}

/// Builds the ten probe rows (S1–S10) with their health.
#[allow(
    clippy::too_many_arguments,
    reason = "the scan's inputs, each its own source"
)]
fn scan_probe_rows(
    installs: &[DiscoveredInfo],
    alias_rows: &[juggler_adapter::environment::AliasHint],
    dangling: u32,
    managers: &[juggler_adapter::environment::ManagerHint],
    cc_health: juggler_adapter::environment::ScanHealth,
    discoveries: &[(String, &'static str)],
    seeds: &[String],
    desktop_apps: &[String],
    cancelled: bool,
) -> Vec<ProbeInfo> {
    let installs_health = if cancelled {
        juggler_adapter::environment::ScanHealth::Partial("cancelled partway".to_owned())
    } else {
        juggler_adapter::environment::ScanHealth::Ok
    };
    let count = |rows: &[DiscoveredInfo]| u32::try_from(rows.len()).unwrap_or_default();
    vec![
        probe(
            "s1-config-dirs",
            installs_health.clone(),
            format!("{} candidate setup(s)", count(installs)),
        ),
        probe(
            "s2-env-overrides",
            juggler_adapter::environment::ScanHealth::Ok,
            env_override_detail(),
        ),
        probe(
            "s3-manager-t3",
            juggler_adapter::environment::ScanHealth::Ok,
            manager_detail(managers, "t3"),
        ),
        probe(
            "s4-manager-cc-switch",
            cc_health,
            manager_detail(managers, "cc-switch"),
        ),
        probe(
            "s5-shell-aliases",
            if dangling > 0 {
                juggler_adapter::environment::ScanHealth::Partial(format!(
                    "{dangling} assignment(s) point at directories that no longer exist"
                ))
            } else {
                juggler_adapter::environment::ScanHealth::Ok
            },
            format!(
                "{} assignment(s){}",
                // The shell's own: other managers' profiles aren't shell settings.
                alias_rows
                    .iter()
                    .filter(|hint| hint.set_by.is_none())
                    .count(),
                if dangling > 0 {
                    format!(" ({dangling} dangling)")
                } else {
                    String::new()
                }
            ),
        ),
        probe(
            "s6-installed-clis",
            juggler_adapter::environment::ScanHealth::Ok,
            discoveries
                .iter()
                .map(|(id, words)| format!("{id}: {words}"))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        probe("s7-login-metadata", installs_health.clone(), {
            let signed = installs
                .iter()
                .filter(|install| install.login == "logged_in")
                .count();
            format!("{signed} of {} signed in", installs.len())
        }),
        probe("s8-usage-recency", installs_health.clone(), {
            let active = installs
                .iter()
                .filter(|install| install.last_used_secs.is_some())
                .count();
            format!("{active} setup(s) with recent activity")
        }),
        probe(
            "s9-projects",
            juggler_adapter::environment::ScanHealth::Ok,
            format!("{} project name(s)", seeds.len()),
        ),
        probe(
            "s10-desktop",
            juggler_adapter::environment::ScanHealth::Ok,
            if desktop_apps.is_empty() {
                "none of the known desktop apps present".to_owned()
            } else {
                desktop_apps.join(", ")
            },
        ),
    ]
}

/// S2 detail: which of the two known override names are set in the
/// app's environment (name match only).
fn env_override_detail() -> String {
    let set: Vec<&str> = ["CLAUDE_CONFIG_DIR", "CODEX_HOME"]
        .iter()
        .filter(|name| std::env::var_os(*name).is_some())
        .copied()
        .collect();
    if set.is_empty() {
        "none set in this app's environment".to_owned()
    } else {
        format!("set: {}", set.join(", "))
    }
}

/// S3/S4 detail: one manager's metadata line ("not present" when
/// absent).
fn manager_detail(managers: &[juggler_adapter::environment::ManagerHint], manager: &str) -> String {
    managers
        .iter()
        .find(|hint| hint.manager == manager)
        .map_or_else(
            || "not present".to_owned(),
            |hint| {
                if hint.present {
                    hint.detail.clone()
                } else {
                    "not present".to_owned()
                }
            },
        )
}
