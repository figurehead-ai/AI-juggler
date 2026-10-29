//! AJ-E1511: open a saved context pack in any tool. The pack is read and
//! checked whole first (`juggler_handover::pack_read`) and shown as the
//! switch screen shows a switch: where it came from, its fidelity, what
//! carries and what doesn't, and what redaction withheld. Nothing is
//! written until the user picks a tool's account and a folder and starts:
//! then the pack, checked unchanged, becomes the session's handover file
//! (as a switch's is, AJ-E1541), the session starts primed with it, and
//! the switch graph records where it came from.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::model::{
    AccountId, EdgeActor, EdgeState, Epoch, EpochId, FailureStage, HarnessId, ManifestEntry,
    PackId, ProfileId, SwitchEdge, SwitchEdgeId, SwitchMechanism, Timestamp, WorkItemId,
};
use juggler_handover::pack_read::{OpenedPack, read_pack};

use crate::error::{AppError, AppResult};
use crate::features::quota::capture::home_relative;
use crate::state::AppState;

/// One kind of thing redaction withheld, and how many.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WithheldInfo {
    /// What it was, in words ("API keys").
    pub what: String,
    /// How many.
    pub count: u32,
}

/// A pack, as the open sheet shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PackOpenInfo {
    /// Names exactly these bytes: the start checks the pack is unchanged.
    pub digest: String,
    /// The pack's folder (home-relative).
    pub path: String,
    /// The tool it came from (its name).
    pub from_tool: String,
    /// The account it came from.
    pub from_account: String,
    /// When it was made (epoch ms).
    #[ts(type = "number")]
    pub created_at_ms: i64,
    /// The project's folder where it was made (home-relative).
    pub project_root: String,
    /// That folder exists on this Mac.
    pub project_here: bool,
    /// The folder itself, when it's on this Mac: offered as where to work.
    pub project_folder: Option<String>,
    /// The branch it was on.
    pub branch: String,
    /// Turns in its conversation.
    pub turns: u32,
    /// Its fidelity, in the switch screen's words.
    pub fidelity: String,
    /// What it carries, in words.
    pub carries: Vec<String>,
    /// What didn't travel, and why.
    pub not_carried: Vec<String>,
    /// What redaction withheld when it was made.
    pub withheld: Vec<WithheldInfo>,
    /// Its brief (`PACK.md`), to read in full.
    pub brief: String,
}

/// A session started from a pack.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PackStartedInfo {
    /// The new session.
    pub session_id: String,
    /// The switch-graph edge that records where it came from.
    pub edge_id: String,
}

fn refused(message: String) -> AppError {
    AppError::Validation {
        subject: "context pack",
        message,
    }
}

fn opened(path: &Path) -> AppResult<OpenedPack> {
    read_pack(path).map_err(|error| refused(error.to_string()))
}

/// The switch screen's words for a fidelity label (`F0`–`F4`), Beta when
/// the pair was never proven.
fn fidelity_words(pack: &OpenedPack) -> String {
    let label = serde_json::from_value::<juggler_handover::fidelity::FidelityLabel>(
        serde_json::json!(pack.state.fidelity.level),
    )
    .map_or(
        "Full story, fresh start",
        juggler_handover::fidelity::FidelityLabel::user_label,
    );
    if pack
        .state
        .fidelity
        .caps_applied
        .iter()
        .any(|cap| cap == "unvalidated-pair")
    {
        format!("{label} · Beta")
    } else {
        label.to_owned()
    }
}

/// A detector's kind in words, as packs and shared docs show it.
pub(crate) fn withheld_words(kind: &str) -> String {
    match kind {
        "sk-key" | "api-key" => "API keys",
        "bearer" => "bearer tokens",
        "env-value" => "secret values",
        "private-key" => "private keys",
        other => other,
    }
    .to_owned()
}

fn tool_name(state: &Arc<AppState>, id: &str) -> String {
    state
        .registry
        .get(&HarnessId::new(id))
        .map_or_else(|| id.to_owned(), |adapter| adapter.spec().display_name)
}

/// Core: reads and checks the pack at `path`, and describes it. Writes
/// nothing.
///
/// # Errors
///
/// [`AppError::Validation`] with the reason, when it isn't a pack this
/// build can open safely.
pub fn preview_impl(state: &Arc<AppState>, path: &Path) -> AppResult<PackOpenInfo> {
    let pack = opened(path)?;
    let artifacts = &pack.state.artifacts;
    let mut carries = vec![format!(
        "the conversation ({} turns)",
        pack.state.conversation.turn_count
    )];
    for (count, one, many) in [
        (artifacts.rules.len(), "rules file", "rules files"),
        (artifacts.mcp.len(), "MCP server", "MCP servers"),
        (artifacts.skills.len(), "skill", "skills"),
        (artifacts.todos.len(), "task", "tasks"),
    ] {
        if count > 0 {
            carries.push(format!("{count} {}", if count == 1 { one } else { many }));
        }
    }
    if pack
        .rows
        .iter()
        .any(|row| row.rel_path.starts_with("diffs/") && row.rel_path != "diffs/EMPTY")
    {
        carries.push("the uncommitted changes".to_owned());
    }
    let root = PathBuf::from(&pack.state.project.root);
    Ok(PackOpenInfo {
        digest: pack.digest.clone(),
        path: home_relative(path),
        from_tool: tool_name(state, &pack.state.source.provider),
        from_account: pack.state.source.account_label.clone(),
        created_at_ms: pack.state.created_at,
        project_root: home_relative(&root),
        project_here: root.is_dir(),
        project_folder: root.is_dir().then(|| root.display().to_string()),
        branch: pack.state.git.branch.clone(),
        turns: pack.state.conversation.turn_count,
        fidelity: fidelity_words(&pack),
        carries,
        not_carried: artifacts
            .drops
            .iter()
            .map(|drop| format!("{}: {}", drop.artifact, drop.reason))
            .collect(),
        withheld: pack
            .state
            .redactions
            .iter()
            .filter(|count| count.count > 0)
            .map(|count| WithheldInfo {
                what: withheld_words(&count.kind),
                count: count.count,
            })
            .collect(),
        brief: pack.pack_md.clone(),
    })
}

/// Quotes `text` line by line: inside the handover it is data.
fn quoted(text: &str) -> String {
    text.lines().fold(String::new(), |mut out, line| {
        let _ = writeln!(out, "> {line}");
        out
    })
}

/// The handover for a session started from `pack`.
fn brief(state: &Arc<AppState>, pack: &OpenedPack, target: &HarnessId, folder: &Path) -> String {
    let source = &pack.state.source;
    let mut out = crate::commands::replay_preface(state, target)
        .trim_end()
        .to_owned();
    let _ = write!(
        out,
        "\n\nThis session continues from a saved handover: {} on {}, in {}, on the branch {}. What carries: {}.\n\n\
         The handover's brief, quoted:\n\n{}\nThe handover's conversation, quoted:\n\n{}",
        tool_name(state, &source.provider),
        source.account_label,
        pack.state.project.root,
        pack.state.git.branch,
        fidelity_words(pack),
        quoted(&pack.pack_md),
        quoted(&pack.conversation_md),
    );
    let specs: Vec<_> = [&HarnessId::new(source.provider.as_str()), target]
        .into_iter()
        .filter_map(|harness| state.registry.get(harness).map(|adapter| adapter.spec()))
        .collect();
    if let Some(rules) = crate::features::handover::rules_section(
        &crate::features::handover::rules_in_play(folder, &specs),
    ) {
        let _ = write!(out, "\n{rules}\n");
    }
    out
}

/// Core: starts a session on launch settings `profile_id` in `folder`,
/// primed with the pack at `path` — the same bytes the preview showed.
///
/// # Errors
///
/// [`AppError::Validation`] when the pack changed since the preview, isn't
/// safe to open, or the folder doesn't exist; the typed storage, IO or
/// launch error otherwise.
pub fn start_impl(
    state: &Arc<AppState>,
    path: &Path,
    digest: &str,
    profile_id: &str,
    folder: &Path,
) -> AppResult<PackStartedInfo> {
    let pack = opened(path)?;
    if pack.digest != digest {
        return Err(refused(
            "the handover changed since you looked at it: open it again".to_owned(),
        ));
    }
    if !folder.is_dir() {
        return Err(refused(
            "pick a folder that exists for the session to work in".to_owned(),
        ));
    }
    let (profile, account, project) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let profile = storage.profiles().get(&ProfileId::new(profile_id))?;
        let account = storage.accounts().get(&profile.account)?;
        let project = storage.projects().touch(folder, None)?;
        (profile, account, project)
    };
    let edge = SwitchEdgeId::new(format!("edge-{}", crate::commands::unique_suffix()));
    let from_epoch = record_staged(state, &pack, &account, &project.id, &edge)?;
    let file = crate::features::handover::write(
        &state.data_dir,
        &brief(state, &pack, &account.harness, folder),
        &crate::features::handover::HandoverState {
            format: "ai-juggler-handover",
            version: 1,
            edge: edge.to_string(),
            from: crate::features::handover::Side {
                tool: pack.state.source.provider.clone(),
                account: pack.state.source.account_label.clone(),
            },
            to: crate::features::handover::Side {
                tool: account.harness.to_string(),
                account: account.label.clone(),
            },
            folder: folder.display().to_string(),
            written_ms: crate::commands::now_millis(),
            turns_included: usize::try_from(pack.state.conversation.turn_count).unwrap_or(0),
            turns_left_out: 0,
            rules_files: Vec::new(),
            notes_quoted: Vec::new(),
            notes_left_out: Vec::new(),
        },
    )
    .map_err(|error| {
        failed(state, &edge, FailureStage::Stage, &error.to_string());
        AppError::Io {
            context: "context pack",
            message: error.to_string(),
        }
    })?;
    let mode = juggler_adapter::LaunchMode::Primed {
        prompt: crate::features::handover::pointer(
            &crate::commands::replay_header(state, &account.harness),
            &file,
        ),
        read_dirs: file.parent().map(Path::to_path_buf).into_iter().collect(),
    };
    let (session, _pid) =
        crate::crossmachine::launch_here(state, &profile, &account, &project, &mode, None, "pack")
            .inspect_err(|error| failed(state, &edge, FailureStage::Launch, &error.to_string()))?;
    let storage = crate::commands::lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let to_epoch = EpochId::new(format!("ep-{}", crate::commands::unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: to_epoch.clone(),
        work: WorkItemId::new(pack.state.work_id.clone()),
        session_ref: Some(session.clone()),
        native_id: None,
        account: account.id.clone(),
        provider: account.harness.clone(),
        resumed_from_epoch: Some(from_epoch),
        machine: None,
    })?;
    graph.mark_launched(&edge, &to_epoch)?;
    graph.mark_confirmed(&edge)?;
    Ok(PackStartedInfo {
        session_id: session.to_string(),
        edge_id: edge.to_string(),
    })
}

fn failed(state: &Arc<AppState>, edge: &SwitchEdgeId, at: FailureStage, why: &str) {
    if let Ok(storage) = crate::commands::lock_storage_arc(state) {
        let _ = storage.switch_graph().mark_failed(edge, at, why);
    }
}

/// The pack's lineage in the switch graph: its work item and source
/// epoch (as it recorded them; mirrored when they came from elsewhere),
/// and a `pack` edge carrying its id, fidelity and manifest. Returns the
/// source epoch.
fn record_staged(
    state: &Arc<AppState>,
    pack: &OpenedPack,
    account: &juggler_core::model::Account,
    project: &juggler_core::model::ProjectId,
    edge: &SwitchEdgeId,
) -> AppResult<EpochId> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let work = WorkItemId::new(pack.state.work_id.clone());
    graph.adopt_work(&work, project, "Work")?;
    let source = &pack.state.source;
    let from_epoch = EpochId::new(source.epoch_id.clone());
    graph.mirror_epoch(&Epoch {
        id: from_epoch.clone(),
        work: work.clone(),
        session_ref: None,
        native_id: Some(source.native_session_id.clone()).filter(|id| !id.is_empty()),
        account: AccountId::new(source.account_label.clone()),
        provider: HarnessId::new(source.provider.clone()),
        resumed_from_epoch: None,
        machine: None,
    })?;
    // Its own edge, when that edge is in this Mac's graph.
    let parent = pack
        .state
        .edge_id
        .as_ref()
        .map(|id| SwitchEdgeId::new(id.clone()))
        .filter(|id| graph.edge(id).is_ok());
    graph.append_edge(&SwitchEdge {
        id: edge.clone(),
        work,
        from_epoch: from_epoch.clone(),
        to_epoch: None,
        from_account: AccountId::new(source.account_label.clone()),
        to_account: account.id.clone(),
        from_provider: HarnessId::new(source.provider.clone()),
        to_provider: account.harness.clone(),
        mechanism: SwitchMechanism::Pack,
        pack_id: Some(PackId::new(pack.state.pack_id.clone())),
        fidelity_shown: Some(pack.state.fidelity.level.clone()),
        manifest_sha256: Some(pack.digest.clone()),
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: parent,
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: None,
    })?;
    let rows: Vec<ManifestEntry> = pack
        .rows
        .iter()
        .map(|row| ManifestEntry {
            rel_path: row.rel_path.clone(),
            sha256: row.sha256.clone(),
            bytes: row.bytes,
            kind: row.kind.clone(),
            disposition: row.disposition.clone(),
        })
        .collect();
    graph.append_manifest(edge, &rows)?;
    Ok(from_epoch)
}

/// Reads and checks a pack, and says what it holds (nothing is written).
///
/// # Errors
///
/// As [`preview_impl`].
#[tauri::command]
pub async fn packs_open_preview(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> AppResult<PackOpenInfo> {
    crate::ipc::blocking(state.inner(), "context pack", move |state| {
        preview_impl(state, Path::new(&path))
    })
    .await
}

/// Starts a session primed with the pack, then shows it.
///
/// # Errors
///
/// As [`start_impl`].
#[tauri::command]
pub async fn packs_open_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
    digest: String,
    profile_id: String,
    folder: String,
) -> AppResult<PackStartedInfo> {
    let started = crate::ipc::blocking(state.inner(), "context pack", move |state| {
        start_impl(
            state,
            Path::new(&path),
            &digest,
            &profile_id,
            Path::new(&folder),
        )
    })
    .await?;
    let _ = crate::commands::emit_sessions(&app, state.inner());
    Ok(started)
}
