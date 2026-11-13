//! AJ-E207: switch-back: resume the original native session plus one delta
//! turn; never a copy-back (D-05; pass2/02 §6.1-6.2).

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::LaunchMode;
use juggler_core::model::{Account, SessionId, SessionStatus, Timestamp};

use crate::commands::{lock_storage_arc, not_found_or_core};
use crate::error::{AppError, AppResult};
use crate::features::handover::source::handover_source_linked;
use crate::features::launch::dto::LaunchResult;
use crate::features::quota::quota_display_inner;
use crate::features::switch::dto::{
    FileConflictInfo, SwitchBackChoiceIn, SwitchBackPlanInfo, SwitchBackReturnInfo,
    SwitchBackSuggestionInfo,
};
use crate::features::switch::edges::source_row;
use crate::features::switch::execute::SwitchSlot;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The lineage facts every switch-back path walks once.
struct BackLineage {
    /// The work both epochs belong to.
    work: juggler_core::model::WorkItemId,
    /// The away side's epoch (what the excursion became).
    away_epoch: juggler_core::model::Epoch,
    /// The original side's epoch at the fork.
    original_epoch: juggler_core::model::Epoch,
    /// The confirmed fork edge (switch A→B).
    fork_edge: juggler_core::model::SwitchEdge,
    /// The away instance's session row.
    away_session: juggler_core::model::InstanceRecord,
    /// The original session row, when it still exists; for work that began
    /// as a session another app runs, that conversation as the switch reads
    /// it (AJ-E2108).
    original_session: Option<juggler_core::model::InstanceRecord>,
    /// AJ-E1541: the epochs the work went through between the original
    /// and the away one, oldest first (A → B → Codex, returning to A: B).
    intermediate: Vec<BackHop>,
    /// Whether the away epoch began as a replay (another tool): its first
    /// user turn is the replay, not new work.
    away_primed: bool,
    /// Every epoch this away one can return to, nearest first.
    returns: Vec<(juggler_core::model::EpochId, juggler_core::model::AccountId)>,
}

/// One epoch between the original and the away one (AJ-E1541).
struct BackHop {
    /// The epoch.
    epoch: juggler_core::model::Epoch,
    /// Its session row, when it still exists.
    session: Option<juggler_core::model::InstanceRecord>,
    /// Whether it began as a replay from another tool.
    primed: bool,
}

/// Walks the graph from an away instance to its fork: the confirmed
/// edge that created it, both epochs, and both session rows.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the session has no switch
/// lineage (never switched away) or the graph is torn.
fn back_lineage(
    state: &Arc<AppState>,
    session_id: &str,
    to_epoch: Option<&str>,
) -> AppResult<(
    BackLineage,
    juggler_core::model::Account,
    juggler_core::model::Account,
)> {
    use juggler_core::model::SessionId;

    let storage = lock_storage_arc(state)?;
    let away_session = storage
        .sessions()
        .get(&SessionId::new(session_id))
        .map_err(|error| not_found_or_core(error, "session", session_id))?;
    let Some(_project) = away_session.project.clone() else {
        return Err(AppError::Io {
            context: "switch back",
            message: "AI Juggler doesn't know which project this session belongs to, so it can't switch it".into(),
        });
    };
    let graph = storage.switch_graph();
    let mut works = Vec::new();
    for record in storage.projects().list()? {
        works.extend(graph.works_for_project(&record.id)?);
    }
    let mut lineage: Option<BackLineage> = None;
    for work in &works {
        let epochs = graph.epochs_for_work(&work.id)?;
        let Some(away_epoch) = epochs
            .iter()
            .find(|epoch| {
                epoch
                    .session_ref
                    .as_ref()
                    .is_some_and(|id| id.as_str() == session_id)
            })
            .cloned()
        else {
            continue;
        };
        let mut chain = chain_back(&epochs, &graph.edges_for_work(&work.id)?, &away_epoch)?;
        if chain.is_empty() {
            continue;
        }
        let target = match to_epoch {
            Some(id) => chain
                .iter()
                .position(|(_, epoch)| epoch.id.as_str() == id)
                .ok_or_else(|| {
                    AppError::Refused("That account isn't one this work came from".to_owned())
                })?,
            None => 0,
        };
        let away_primed = is_replay(&chain[0].0);
        let intermediate = hops_of(&storage, &chain, target);
        let returns = chain
            .iter()
            .map(|(_, epoch)| (epoch.id.clone(), epoch.account.clone()))
            .collect();
        let (fork_edge, original_epoch) = chain.swap_remove(target);
        let original_session = original_epoch
            .session_ref
            .as_ref()
            .and_then(|id| storage.sessions().get(id).ok());
        lineage = Some(BackLineage {
            work: work.id.clone(),
            away_epoch,
            original_epoch,
            fork_edge,
            away_session,
            original_session,
            intermediate,
            away_primed,
            returns,
        });
        break;
    }
    let Some(lineage) = lineage else {
        return Err(AppError::Io {
            context: "switch back",
            message: "This session never switched away — nothing to return to".to_owned(),
        });
    };
    let away_account = storage
        .accounts()
        .get(&lineage.away_epoch.account)
        .map_err(|error| {
            not_found_or_core(error, "account", &lineage.away_epoch.account.to_string())
        })?;
    let original_account = storage
        .accounts()
        .get(&lineage.original_epoch.account)
        .map_err(|error| {
            not_found_or_core(
                error,
                "account",
                &lineage.original_epoch.account.to_string(),
            )
        })?;
    drop(storage);
    let lineage = with_outside_original(state, lineage, &original_account);
    Ok((lineage, away_account, original_account))
}

/// AJ-E2108: work that began as a session another app runs goes back to
/// that conversation, read from the index as the first switch read it.
fn with_outside_original(
    state: &Arc<AppState>,
    mut lineage: BackLineage,
    original_account: &Account,
) -> BackLineage {
    if lineage.original_session.is_none()
        && lineage.original_epoch.session_ref.is_none()
        && let Some(native) = lineage.original_epoch.native_id.as_deref()
    {
        lineage.original_session = crate::features::conversations::switch_source::as_switch_source(
            state,
            original_account.id.as_str(),
            native,
            false,
        )
        .ok()
        .map(|(row, _)| row);
    }
    lineage
}

/// AJ-E1541: the confirmed edges back from `away`, nearest first: (the
/// edge into each epoch, the epoch it came from). A loop back to an epoch
/// already visited stops the walk at its first visit.
fn chain_back(
    epochs: &[juggler_core::model::Epoch],
    edges: &[juggler_core::model::SwitchEdge],
    away: &juggler_core::model::Epoch,
) -> AppResult<Vec<(juggler_core::model::SwitchEdge, juggler_core::model::Epoch)>> {
    let mut chain: Vec<(juggler_core::model::SwitchEdge, juggler_core::model::Epoch)> = Vec::new();
    let mut current = away.id.clone();
    while let Some(edge) = edges.iter().find(|edge| {
        edge.to_epoch.as_ref() == Some(&current)
            && matches!(edge.state, juggler_core::model::EdgeState::Confirmed)
    }) {
        let Some(from) = epochs
            .iter()
            .find(|epoch| epoch.id == edge.from_epoch)
            .cloned()
        else {
            return Err(AppError::Io {
                context: "switch back",
                message: "AI Juggler can't find where this work was before the switch, so it can't go back".to_owned(),
            });
        };
        if chain.iter().any(|(_, seen)| seen.id == from.id) {
            break;
        }
        current = from.id.clone();
        chain.push((edge.clone(), from));
    }
    Ok(chain)
}

/// AJ-E1541: the epochs between the return target (`chain[target]`) and
/// the away one, oldest first: the epoch each nearer edge came from, with
/// whether that epoch's own way in was a replay.
fn hops_of(
    storage: &juggler_core::storage::Storage,
    chain: &[(juggler_core::model::SwitchEdge, juggler_core::model::Epoch)],
    target: usize,
) -> Vec<BackHop> {
    (0..target)
        .rev()
        .map(|index| {
            let epoch = chain[index].1.clone();
            let session = epoch
                .session_ref
                .as_ref()
                .and_then(|id| storage.sessions().get(id).ok());
            BackHop {
                primed: is_replay(&chain[index + 1].0),
                epoch,
                session,
            }
        })
        .collect()
}

/// AJ-E1541: a session started by a switch to another tool has no
/// transcript link until its tool writes one. Links every session on the
/// way back (persisted, the same lookup the switch uses) before the facts
/// are read.
fn link_chain(state: &Arc<AppState>, session_id: &str, to_epoch: Option<&str>) -> AppResult<()> {
    let (lineage, _, _) = back_lineage(state, session_id, to_epoch)?;
    let unlinked: Vec<String> = std::iter::once(&lineage.away_session)
        .chain(
            lineage
                .intermediate
                .iter()
                .filter_map(|hop| hop.session.as_ref()),
        )
        .chain(lineage.original_session.iter())
        .filter(|row| row.transcript_ref.is_none())
        .map(|row| row.id.to_string())
        .collect();
    drop(lineage);
    for id in &unlinked {
        let _ = handover_source_linked(state, id);
    }
    Ok(())
}

/// Whether an edge was a replay into another tool (AJ-E1541).
pub(crate) fn is_replay(edge: &juggler_core::model::SwitchEdge) -> bool {
    matches!(edge.mechanism, juggler_core::model::SwitchMechanism::Replay)
}

/// AJ-E1541: what was new on each hop between the account returned to and
/// the away one, oldest first. `previous` starts as the returned-to
/// account's uuids and gains each hop's, so a later hop counts only its own.
/// A hop whose conversation can't be found goes into `unread`, so the
/// update names it.
fn hops_between(
    state: &Arc<AppState>,
    hops: &[BackHop],
    previous: &mut std::collections::HashSet<String>,
    unread: &mut Vec<String>,
) -> AppResult<Vec<juggler_handover::switch_back::DeltaHop>> {
    let mut out = Vec::new();
    for hop in hops {
        let account = {
            let storage = lock_storage_arc(state)?;
            storage.accounts().get(&hop.epoch.account).ok()
        };
        // A hop that can't be read is named, never left out (review of
        // #132, M5).
        let (Some(row), Some(account)) = (hop.session.as_ref(), account.as_ref()) else {
            unread.push(account.map_or_else(
                || "an account that's no longer here".to_owned(),
                |account| account.label.clone(),
            ));
            continue;
        };
        let reference = newest_transcript(state, account, row.transcript_ref.clone());
        let Some((uuids, turns)) = reference.as_ref().and_then(|r| transcript_side(&r.path)) else {
            unread.push(account.label.clone());
            continue;
        };
        let turns = new_turns(turns, previous, hop.primed);
        previous.extend(uuids);
        out.push(juggler_handover::switch_back::DeltaHop {
            account: account.label.clone(),
            turns,
        });
    }
    Ok(out)
}

/// One side's conversation facts: the uuid set (reconciliation) and
/// the user/assistant turns in file order (the delta body).
pub(crate) fn transcript_side(
    path: &std::path::Path,
) -> Option<(
    std::collections::HashSet<String>,
    Vec<juggler_handover::pack::IrTurn>,
)> {
    // AJ-E1540: a compressed rollout (`.jsonl.zst`) reads like any other.
    let bytes = juggler_adapters::read_session_file(path).ok()?;
    Some(turns_of(juggler_adapters::parse_any(&bytes)))
}

/// AJ-E1605 Req.6: the user/assistant turns of the conversation at
/// `path` (its link) for a pack: read by its tool's adapter when the tool
/// keeps them its own way (a store: `ZCode`), else parsed as a transcript
/// file ([`transcript_side`]). Empty when neither reads it.
pub(crate) fn conversation_turns(
    state: &AppState,
    harness: &juggler_core::model::HarnessId,
    path: &std::path::Path,
) -> Vec<juggler_handover::pack::IrTurn> {
    let own = state
        .registry
        .get(harness)
        .and_then(|adapter| juggler_adapter::ToolTranscripts::conversation_turns(&*adapter, path));
    match own {
        Some(turns) => turns
            .into_iter()
            .map(|turn| juggler_handover::pack::IrTurn {
                uuid: turn.uuid,
                role: if turn.role == "user" {
                    juggler_handover::pack::IrRole::User
                } else {
                    juggler_handover::pack::IrRole::Assistant
                },
                text: turn.text,
                at_ms: None,
                tokens: None,
            })
            .collect(),
        None => transcript_side(path)
            .map(|(_, turns)| turns)
            .unwrap_or_default(),
    }
}

/// A parsed transcript's uuid set and user/assistant turns, in file order.
pub(crate) fn turns_of(
    parsed: juggler_adapters::claude_transcript::ParsedTranscript,
) -> (
    std::collections::HashSet<String>,
    Vec<juggler_handover::pack::IrTurn>,
) {
    use juggler_adapters::claude_transcript::MessageRole;
    let mut uuids = std::collections::HashSet::new();
    let mut turns = Vec::new();
    for message in parsed.messages {
        if let Some(uuid) = &message.uuid {
            uuids.insert(uuid.clone());
        }
        let role = match message.role {
            MessageRole::User => juggler_handover::pack::IrRole::User,
            MessageRole::Assistant => juggler_handover::pack::IrRole::Assistant,
            MessageRole::Other => continue,
        };
        // AJ-E1541: what the turn's tool calls did, after what it said, so
        // a handover carries the work and not only the chat.
        let text = match (message.text.trim().is_empty(), message.tools.is_empty()) {
            // A message that only carries tool results says nothing of its
            // own: quoted, it was an empty turn (the measured lab of
            // 2026-10-28: 27 of a handover's 69 turns).
            (true, true) => continue,
            (_, true) => message.text,
            (true, false) => format!("({})", message.tools.join("; ")),
            (false, false) => format!("{}\n({})", message.text, message.tools.join("; ")),
        };
        turns.push(juggler_handover::pack::IrTurn {
            uuid: message.uuid,
            role,
            text,
            at_ms: None,
            tokens: None,
        });
    }
    (uuids, turns)
}

/// AJ-E1541: what changed in the project while away, for the switch-back
/// delta: git's diff stat, else how many files changed.
fn away_diff_stat(cwd: &std::path::Path) -> Option<String> {
    crate::worktree::work_changes(cwd).and_then(|changes| {
        changes.stat.or_else(|| {
            (!changes.files.is_empty()).then(|| format!("{} files changed", changes.files.len()))
        })
    })
}

/// The rules files the tools the work went through while away read, which
/// the tool it returns to doesn't load itself (the measured lab of
/// 2026-10-28), in words for the update; `None` when there are none.
fn away_rules_note(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away: &juggler_core::model::Account,
    original: &juggler_core::model::Account,
) -> Option<String> {
    use crate::features::handover;
    let returning = state.registry.get(&original.harness)?.spec();
    let mut tools: Vec<juggler_core::model::HarnessId> = lineage
        .intermediate
        .iter()
        .map(|hop| hop.epoch.provider.clone())
        .collect();
    tools.push(away.harness.clone());
    tools.retain(|tool| *tool != original.harness);
    tools.dedup();
    let specs: Vec<_> = tools
        .iter()
        .filter_map(|tool| state.registry.get(tool).map(|adapter| adapter.spec()))
        .collect();
    let rules = handover::rules_in_play(&lineage.away_session.cwd, &specs);
    handover::rules_while_away(&rules, &returning)
}

/// The task list as the work left it while away: the away session's, else
/// (a tool with no task list of its own, such as `codex exec`) the newest
/// hop's before it that set one, saying whose it is (the measured lab of
/// 2026-10-28: back from Codex, the update said nothing of the list B had
/// moved on, and the original's was stale).
fn chain_task_note(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away: &juggler_core::model::Account,
    away_reference: Option<&juggler_core::model::TranscriptRef>,
) -> Option<String> {
    if let Some(note) = away_task_note(away_reference) {
        return Some(note);
    }
    lineage.intermediate.iter().rev().find_map(|hop| {
        let account = lock_storage_arc(state)
            .ok()?
            .accounts()
            .get(&hop.epoch.account)
            .ok()?;
        if account.id == away.id {
            return None;
        }
        let reference = newest_transcript(
            state,
            &account,
            hop.session.as_ref()?.transcript_ref.clone(),
        );
        away_task_note(reference.as_ref())
            .map(|note| format!("{note} (as {} left it)", account.label))
    })
}

/// AJ-E1541: the task list as the away session left it, on one line.
fn away_task_note(reference: Option<&juggler_core::model::TranscriptRef>) -> Option<String> {
    let bytes = juggler_adapters::read_session_file(&reference?.path).ok()?;
    let tasks = juggler_adapters::parse_any(&bytes).tasks?;
    (!tasks.is_empty()).then(|| {
        juggler_adapters::tool_summary::render_tasks(&tasks)
            .lines()
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// AJ-E1541: the turns of one hop that are new on it: not in what came
/// before (`previous`), and without its first user turn when it began as
/// a replay from another tool (that turn is the replay, not new work).
fn new_turns(
    turns: Vec<juggler_handover::pack::IrTurn>,
    previous: &std::collections::HashSet<String>,
    primed: bool,
) -> Vec<juggler_handover::pack::IrTurn> {
    let mut skipped_prime = !primed;
    turns
        .into_iter()
        .filter(|turn| {
            if !skipped_prime && turn.role == juggler_handover::pack::IrRole::User {
                skipped_prime = true;
                return false;
            }
            turn.uuid
                .as_ref()
                .is_none_or(|uuid| !previous.contains(uuid))
        })
        .collect()
}

/// AJ-E1541: the newest file of a linked conversation. A tool that writes
/// a new file on every resume (Codex) leaves the first link behind; the
/// conversation's id finds the file that holds the latest turns.
fn newest_transcript(
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
    reference: Option<juggler_core::model::TranscriptRef>,
) -> Option<juggler_core::model::TranscriptRef> {
    let reference = reference?;
    let Some(id) = reference.native_id.clone() else {
        return Some(reference);
    };
    crate::features::conversations::transcript_of(state, account, &id, &[]).or(Some(reference))
}

/// Hashes the fork manifest's FILE-domain rows on one side now
/// (transcript-kind rows are the conversation domain — uuid sets,
/// never file choices; that split is how the copy-back ban stays
/// structural).
fn file_domain_hashes_now(
    fork_rows: &[juggler_core::model::ManifestEntry],
    side_transcript: Option<&juggler_core::model::TranscriptRef>,
) -> juggler_handover::reconcile::ArtifactHashes {
    let mut hashes = juggler_handover::reconcile::ArtifactHashes::new();
    let Some(reference) = side_transcript else {
        return hashes;
    };
    let Some(dir) = reference.path.parent() else {
        return hashes;
    };
    for row in fork_rows.iter().filter(|row| row.kind != "transcript") {
        if let Ok(bytes) = std::fs::read(dir.join(&row.rel_path)) {
            hashes.insert(
                row.rel_path.clone(),
                juggler_handover::manifest::sha256_hex(&bytes),
            );
        }
    }
    hashes
}

/// The conversation side of a switch back: the away turns, how the two
/// sides' conversations relate, the hops between, and the accounts whose
/// conversation couldn't be found.
struct BackConversation {
    away_turns: Vec<juggler_handover::pack::IrTurn>,
    conversation: juggler_handover::reconcile::ConversationReconciliation,
    earlier_hops: Vec<juggler_handover::switch_back::DeltaHop>,
    unread: Vec<String>,
}

fn back_conversation(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away_label: &str,
    away_reference: Option<&juggler_core::model::TranscriptRef>,
    original_reference: Option<&juggler_core::model::TranscriptRef>,
) -> AppResult<BackConversation> {
    // Conversation: base = intersection (B began as A's bytes);
    // away-only renders the delta; original-only is a no-op.
    let away_side = away_reference.and_then(|reference| transcript_side(&reference.path));
    let original_side = original_reference.and_then(|reference| transcript_side(&reference.path));
    let empty = std::collections::HashSet::new();
    let away_side_missing = away_side.is_none();
    let (mut away_uuids, mut away_turns) = away_side.unwrap_or_else(|| (empty.clone(), Vec::new()));
    let (original_uuids, _) = original_side.unwrap_or_else(|| (empty, Vec::new()));
    // AJ-E1541: the hops between, each reduced to what was new on it; a
    // hop (or the away epoch) that began as a replay drops its first user
    // turn, which is the replay itself.
    let mut previous_uuids = original_uuids.clone();
    let mut unread = Vec::new();
    let earlier_hops = hops_between(
        state,
        &lineage.intermediate,
        &mut previous_uuids,
        &mut unread,
    )?;
    if away_side_missing {
        unread.push(away_label.to_owned());
    }
    if lineage.away_primed {
        away_turns = new_turns(away_turns, &std::collections::HashSet::new(), true);
        away_uuids = away_turns
            .iter()
            .filter_map(|turn| turn.uuid.clone())
            .collect();
    }
    let mut conversation =
        juggler_handover::reconcile::reconcile_conversation(&away_uuids, &previous_uuids);
    // A replay shares no message ids with what it replayed, so "shared
    // before the switch" and "new here since" come from the original as it
    // was when the switch read it (live lab 12: back from Codex, the delta
    // said "0 shared before the switch, 14 new here" for a conversation
    // that hadn't moved).
    if lineage.away_primed
        && let Some(at_fork) = original_at_fork(state, lineage, original_reference)
    {
        conversation.base_uuids = original_uuids.intersection(&at_fork).count();
        conversation.original_only_uuids = original_uuids.difference(&at_fork).count();
    }
    Ok(BackConversation {
        away_turns,
        conversation,
        earlier_hops,
        unread,
    })
}

/// The original conversation's message ids as the fork read them: its file
/// cut at the size the fork edge's manifest recorded for it (a
/// conversation file only grows). `None` when the manifest has no row for
/// that file or the file is shorter now.
fn original_at_fork(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    original: Option<&juggler_core::model::TranscriptRef>,
) -> Option<std::collections::HashSet<String>> {
    let path = &original?.path;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let rows = lock_storage_arc(state)
        .ok()?
        .switch_graph()
        .manifest_for(&lineage.fork_edge.id)
        .ok()?;
    let row = rows
        .iter()
        .find(|row| row.kind == "transcript" && row.rel_path == name)?;
    let bytes = juggler_adapters::read_session_file(path).ok()?;
    let cut = usize::try_from(row.bytes).ok()?;
    let prefix = bytes.get(..cut)?;
    Some(turns_of(juggler_adapters::parse_any(prefix)).0)
}

/// Assembles the plan facts shared by plan/execute (one walk, one
/// reconcile — the same truth both halves render and act on).
#[allow(
    clippy::type_complexity,
    reason = "the plan's facts as one tuple, read once"
)]
fn back_facts(
    state: &Arc<AppState>,
    session_id: &str,
    to_epoch: Option<&str>,
) -> AppResult<(
    BackLineage,
    juggler_core::model::Account,
    juggler_core::model::Account,
    juggler_handover::switch_back::DeltaInput,
    Vec<juggler_handover::reconcile::FileConflict>,
    bool,
    juggler_handover::switch_back::OriginalSession,
)> {
    use juggler_handover::reconcile::reconcile_files;

    link_chain(state, session_id, to_epoch)?;
    let (lineage, away_account, original_account) = back_lineage(state, session_id, to_epoch)?;
    let away_reference = newest_transcript(
        state,
        &away_account,
        lineage.away_session.transcript_ref.clone(),
    );
    let original_reference = lineage
        .original_session
        .as_ref()
        .and_then(|row| row.transcript_ref.clone());
    let original_alive = original_reference
        .as_ref()
        .is_some_and(|reference| reference.path.is_file());

    // S-F7: never fabricate — nearest surviving epoch on the
    // ORIGINAL account whose transcript still exists.
    let nearest_surviving = {
        let storage = lock_storage_arc(state)?;
        let graph = storage.switch_graph();
        graph
            .epochs_for_work(&lineage.work)?
            .into_iter()
            .filter(|epoch| {
                epoch.account == original_account.id && epoch.id != lineage.original_epoch.id
            })
            .find_map(|epoch| {
                let row = epoch
                    .session_ref
                    .as_ref()
                    .and_then(|id| storage.sessions().get(id).ok())?;
                let reference = row.transcript_ref?;
                reference.native_id.filter(|_| reference.path.is_file())
            })
    };
    let verdict = juggler_handover::switch_back::original_session_verdict(
        original_alive,
        lineage
            .original_epoch
            .native_id
            .as_deref()
            .filter(|id| !id.is_empty()),
        nearest_surviving.as_deref(),
    );

    let BackConversation {
        away_turns,
        conversation,
        earlier_hops,
        unread,
    } = back_conversation(
        state,
        &lineage,
        &away_account.label,
        away_reference.as_ref(),
        original_reference.as_ref(),
    )?;

    // File domain: the fork manifest's non-transcript rows, hashed
    // on both sides now (three-way; conflicts never auto-merge).
    let fork_rows = {
        let storage = lock_storage_arc(state)?;
        storage
            .switch_graph()
            .manifest_for(&lineage.fork_edge.id)
            .unwrap_or_default()
    };
    let base: juggler_handover::reconcile::ArtifactHashes = fork_rows
        .iter()
        .filter(|row| row.kind != "transcript")
        .map(|row| (row.rel_path.clone(), row.sha256.clone()))
        .collect();
    let away_files = file_domain_hashes_now(&fork_rows, away_reference.as_ref());
    let original_files = file_domain_hashes_now(&fork_rows, original_reference.as_ref());
    let conflicts = reconcile_files(&base, &away_files, &original_files);

    let delta_input = juggler_handover::switch_back::DeltaInput {
        away_account: away_account.label.clone(),
        original_account: original_account.label.clone(),
        away_turns,
        away_only_uuids: conversation.away_only_uuids.iter().cloned().collect(),
        conversation: conversation.clone(),
        diff_stat: away_diff_stat(&lineage.away_session.cwd),
        todo_note: chain_task_note(state, &lineage, &away_account, away_reference.as_ref()),
        earlier_hops,
        unread,
        rules_note: away_rules_note(state, &lineage, &away_account, &original_account),
    };
    let away_live = matches!(
        lineage.away_session.status.as_str(),
        "running" | "starting" | "waiting_user"
    );
    Ok((
        lineage,
        away_account,
        original_account,
        delta_input,
        conflicts,
        away_live,
        verdict,
    ))
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_back_plan_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<SwitchBackPlanInfo> {
    switch_back_plan_to_impl(state, session_id, None)
}

/// [`switch_back_plan_impl`] returning to a chosen earlier epoch
/// (AJ-E1541): `None` is the account the work came from last.
///
/// # Errors
///
/// [`AppError::Refused`] when `to_epoch` isn't one this work came from;
/// otherwise as [`switch_back_plan_impl`].
pub fn switch_back_plan_to_impl(
    state: &Arc<AppState>,
    session_id: &str,
    to_epoch: Option<&str>,
) -> AppResult<SwitchBackPlanInfo> {
    use juggler_handover::switch_back::{
        OriginalSession, Sf7Fallback, both_live_copy, delta_turn_text, divergence_copy,
        suggestion_copy,
    };

    let (lineage, _away_account, original_account, delta_input, conflicts, away_live, verdict) =
        back_facts(state, session_id, to_epoch)?;
    let returns = {
        let storage = lock_storage_arc(state)?;
        let last = lineage.returns.len().saturating_sub(1);
        lineage
            .returns
            .iter()
            .enumerate()
            .map(|(index, (epoch, account))| SwitchBackReturnInfo {
                epoch_id: epoch.to_string(),
                account: storage
                    .accounts()
                    .get(account)
                    .map_or_else(|_| account.to_string(), |found| found.label),
                start: index == last,
            })
            .collect::<Vec<_>>()
    };
    let return_to = lineage.original_epoch.id.to_string();
    let away_only_turns =
        u32::try_from(juggler_handover::switch_back::delta_turns(&delta_input).len())
            .unwrap_or(u32::MAX);
    let (original_native_id, sf7_fallback, sf7_copy) = match &verdict {
        OriginalSession::Alive { native_id } => (native_id.clone(), String::new(), String::new()),
        OriginalSession::Gone { fallback } => (
            String::new(),
            match fallback {
                Sf7Fallback::NearestEpoch { native_id } => {
                    format!("nearest-epoch:{native_id}")
                }
                Sf7Fallback::FreshPackF0 => "fresh-f0".to_owned(),
            },
            // Plain words (AJ-E1543): the screen adds what happens instead.
            format!(
                "The original conversation on {} is gone, so it can't be resumed",
                original_account.label
            ),
        ),
    };
    if away_only_turns > 0 || !conflicts.is_empty() {
        crate::usage_stats::count("switch.divergence_shown", &[]);
    }
    // D-62: the reset is the original folder's Account's (its folders
    // share its limits), so the suggestion names it; a folder no Account
    // lists keeps its own name. The rest names the folder: the App the
    // work goes back to.
    let reset = crate::features::subscriptions::account_name_of(
        state,
        original_account.id.as_str(),
        &original_account.label,
    );
    Ok(SwitchBackPlanInfo {
        suggestion: suggestion_copy(&reset, &delta_input.away_account),
        original_account: original_account.label.clone(),
        away_account: delta_input.away_account.clone(),
        original_native_id,
        original_alive: matches!(verdict, OriginalSession::Alive { .. }),
        sf7_fallback,
        sf7_copy,
        away_live,
        both_live_copy: both_live_copy(&original_account.label, &delta_input.away_account),
        divergence: divergence_copy(away_only_turns as usize, conflicts.len()),
        delta_preview: delta_turn_text(&delta_input),
        away_only_turns,
        original_only_turns: u32::try_from(delta_input.conversation.original_only_uuids)
            .unwrap_or(u32::MAX),
        base_turns: u32::try_from(delta_input.conversation.base_uuids).unwrap_or(u32::MAX),
        conflicts: conflicts.iter().map(FileConflictInfo::of).collect(),
        returns,
        return_to,
    })
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_back_execute_impl(
    state: &Arc<AppState>,
    session_id: &str,
    choices: &[SwitchBackChoiceIn],
    keep_away_running: bool,
) -> AppResult<LaunchResult> {
    switch_back_execute_to_impl(state, session_id, choices, keep_away_running, None)
}

/// [`switch_back_execute_impl`] returning to a chosen earlier epoch
/// (AJ-E1541): `None` is the account the work came from last.
///
/// # Errors
///
/// [`AppError::Refused`] when `to_epoch` isn't one this work came from;
/// otherwise as [`switch_back_execute_impl`].
pub fn switch_back_execute_to_impl(
    state: &Arc<AppState>,
    session_id: &str,
    choices: &[SwitchBackChoiceIn],
    keep_away_running: bool,
    to_epoch: Option<&str>,
) -> AppResult<LaunchResult> {
    use juggler_handover::switch_back::{plan_resolutions, unresolved};

    let _slot = SwitchSlot::take(state, session_id)?;
    let (lineage, away_account, original_account, delta_input, conflicts, away_live, verdict) =
        back_facts(state, session_id, to_epoch)?;
    // AJ-E2108: an original another app has open now is left alone:
    // resuming it here too would give one conversation two writers.
    if let Some(original) = &lineage.original_session
        && source_row(original).is_none()
        && original.status == SessionStatus::Running
    {
        return Err(AppError::Refused(format!(
            "The original on {} is still open in its app: close it there first, so one \
             conversation never has two writers",
            original_account.label
        )));
    }
    // S-F8: every conflict resolved or the work stays forked — an
    // error, never a guess.
    let resolutions = parse_switch_back_choices(choices);
    if !unresolved(&conflicts, &resolutions).is_empty() {
        return Err(AppError::switch(
            juggler_handover::pipeline::SwitchFailure::MergeConflictOpen,
            &original_account.label,
            &away_account.label,
        ));
    }

    // Apply the file plan (file domain only — the transcript is not
    // an artifact, so no path here can touch it).
    let actions = plan_resolutions(&conflicts, &resolutions, &chrono_date_compact());
    apply_conflict_choices(&lineage, &actions);

    // The away instance: stop per the explicit choice, or keep it
    // running (legal — the plan's both-live copy already said so).
    if !keep_away_running && away_live {
        stop_away_instance(state, &lineage.away_session.id)?;
    }

    let resumed = resume_original_with_delta(
        state,
        &lineage,
        &away_account,
        original_account.clone(),
        &delta_input,
        &verdict,
        &actions,
    )?;
    crate::usage_stats::count(
        "switch.back",
        &[(
            "kind",
            if away_account.harness == original_account.harness {
                "account"
            } else {
                "provider"
            },
        )],
    );
    // ADR-0028: the automation comes back with the work: the original
    // takes over the copy's and leaves the archive, the copy's is off.
    crate::features::handover::automation::switch_back(state, lineage.fork_edge.id.as_str());
    // 0.4.1: a Desktop copy the work left leaves the pools it stayed in.
    if let Err(error) = crate::features::pools::switching::leave_watched(state, session_id) {
        crate::error_log::record("switch_back", &error.to_string());
    }
    Ok(resumed)
}

/// Parses the UI's wire choices into resolutions (unknown wire
/// names are dropped — an unresolved conflict then stops the
/// execute, never guessed).
fn parse_switch_back_choices(
    choices: &[SwitchBackChoiceIn],
) -> Vec<juggler_handover::switch_back::ConflictResolution> {
    use juggler_handover::reconcile::ConflictChoice;
    choices
        .iter()
        .filter_map(|row| {
            let choice = match row.choice.as_str() {
                "keep-original" => ConflictChoice::KeepOriginal,
                "take-away" => ConflictChoice::TakeAway,
                "keep-both" => ConflictChoice::KeepBoth,
                _ => return None,
            };
            Some(juggler_handover::switch_back::ConflictResolution {
                rel_path: row.rel_path.clone(),
                choice,
            })
        })
        .collect()
}

/// Applies the chosen file actions between the two transcript
/// directories (file domain only — the transcript itself is never
/// an action target, which keeps the copy-back ban structural).
fn apply_conflict_choices(
    lineage: &BackLineage,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) {
    use juggler_handover::reconcile::FileAction;
    let away_dir = lineage
        .away_session
        .transcript_ref
        .as_ref()
        .and_then(|reference| reference.path.parent().map(std::path::Path::to_path_buf));
    let original_dir = lineage
        .original_session
        .as_ref()
        .and_then(|row| row.transcript_ref.as_ref())
        .and_then(|reference| reference.path.parent().map(std::path::Path::to_path_buf));
    for (resolution, action) in actions {
        let (Some(away_dir), Some(original_dir)) = (away_dir.as_ref(), original_dir.as_ref())
        else {
            continue;
        };
        let source = away_dir.join(&resolution.rel_path);
        match action {
            FileAction::ReplaceWithAway => {
                let _ = std::fs::copy(&source, original_dir.join(&resolution.rel_path));
            }
            FileAction::SaveAwaySibling { sibling_rel_path } => {
                let _ = std::fs::copy(&source, original_dir.join(sibling_rel_path));
            }
            FileAction::Unchanged => {}
        }
    }
}

/// Stops the away instance (freeze-or-fork's stop side). Already-
/// gone is fine; a REAL stop failure aborts before the resume.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the stop ladder fails.
fn stop_away_instance(state: &Arc<AppState>, session: &SessionId) -> AppResult<()> {
    // The rc27 P1 walk: a switch's stop, not the user's.
    state.mark_switch_stop(session);
    match state.supervisor.stop(session, Duration::from_secs(5)) {
        Ok(()) => Ok(()),
        Err(juggler_supervisor::SupervisorError::UnknownSession(_)) => {
            state.forget_switch_stop(session);
            Ok(())
        }
        Err(error) => {
            state.forget_switch_stop(session);
            crate::error_log::record("switch_back", &error.to_string());
            Err(AppError::Io {
                context: "switch back",
                message: "AI Juggler couldn't stop the session on the other account, so nothing moved. Stop it from its page and try again".to_owned(),
            })
        }
    }
}

/// Spawns the return (resume + delta, or the honest S-F7 fallback)
/// and records the switch-back edge: resumes the ORIGINAL epoch,
/// parent = the fork; the conflict choices ride the manifest (E201).
///
/// # Errors
///
/// Returns the typed [`AppError`] on resolution, spawn, or graph
/// write failure — the original transcript is never written on any
/// path (copy-back has no code path).
#[allow(
    clippy::too_many_arguments,
    reason = "the inputs of switching back, each its own"
)]
fn resume_original_with_delta(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away_account: &juggler_core::model::Account,
    original_account: juggler_core::model::Account,
    delta_input: &juggler_handover::switch_back::DeltaInput,
    verdict: &juggler_handover::switch_back::OriginalSession,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) -> AppResult<LaunchResult> {
    use juggler_handover::switch_back::{OriginalSession, Sf7Fallback, delta_turn_text};

    // What resumes: the original native id, or the S-F7 fallback
    // (never fabricated). Fresh-f0 = a fresh interactive start
    // honestly labelled F0 (the pack carries the detail; nothing is
    // primed silently); mechanism + fidelity are decided at the
    // edge (record_switch_back_edge).
    let (resume_id, fresh) = match verdict {
        OriginalSession::Alive { native_id }
        | OriginalSession::Gone {
            fallback: Sf7Fallback::NearestEpoch { native_id },
        } => (native_id.clone(), false),
        OriginalSession::Gone {
            fallback: Sf7Fallback::FreshPackF0,
        } => (String::new(), true),
    };
    let mode = if fresh {
        LaunchMode::Interactive
    } else {
        LaunchMode::Resume {
            session_id: resume_id.clone(),
            delta_prompt: Some(delta_turn_text(delta_input)),
        }
    };

    let original_session = lineage
        .original_session
        .clone()
        .ok_or_else(|| AppError::Io {
            context: "switch back",
            message:
                "The original session isn't listed any more — resume it from the switch history"
                    .to_owned(),
        })?;
    let (profile_a, account_a) = {
        let storage = lock_storage_arc(state)?;
        let profile = storage
            .profiles()
            .get(&original_session.profile)
            .map_err(|error| {
                not_found_or_core(error, "profile", &original_session.profile.to_string())
            })?;
        (profile, original_account)
    };
    // AJ-E1001: the original Account must still be who it was.
    let assertion = crate::identity::assert_identity(state, &account_a)?;
    let (new_session, pid) = crate::features::switch::resume::relaunch(
        state,
        &crate::features::switch::resume::Relaunch {
            account: &account_a,
            profile: &profile_a,
            from: &original_session,
            mode,
            conversation: resume_id.clone(),
            origin: "switch",
        },
    )?;

    record_switch_back_edge(
        state,
        lineage,
        away_account,
        &account_a,
        &new_session,
        &resume_id,
        fresh,
        actions,
    )?;
    crate::identity::record(state, &new_session, &account_a, &assertion, "switch_back");
    crate::features::tasks::switched::after_switch(
        state,
        &new_session,
        &away_account.id,
        &account_a.id,
    );

    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid,
    })
}

/// Records the return: the resumed instance's epoch
/// (`resumed_from_epoch` = the ORIGINAL epoch), the switch-back
/// edge (parent = the fork; two-parent shape ready for merges),
/// and the per-conflict choices as manifest rows (E201).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage or graph writes fail.
#[allow(
    clippy::too_many_arguments,
    reason = "the graph edge's columns, each its own"
)]
fn record_switch_back_edge(
    state: &Arc<AppState>,
    lineage: &BackLineage,
    away_account: &juggler_core::model::Account,
    account_a: &juggler_core::model::Account,
    new_session: &SessionId,
    resume_id: &str,
    fresh: bool,
    actions: &[(
        juggler_handover::switch_back::ConflictResolution,
        juggler_handover::reconcile::FileAction,
    )],
) -> AppResult<()> {
    use juggler_core::model::{
        EdgeActor, EdgeState, Epoch, EpochId, ManifestEntry, SwitchEdge, SwitchEdgeId,
        SwitchMechanism,
    };
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let epoch_id = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: epoch_id.clone(),
        work: lineage.work.clone(),
        session_ref: Some(new_session.clone()),
        native_id: Some(resume_id.to_owned()),
        account: lineage.original_epoch.account.clone(),
        provider: lineage.original_epoch.provider.clone(),
        resumed_from_epoch: Some(lineage.original_epoch.id.clone()),
        machine: None,
    })?;
    let edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph.append_edge(&SwitchEdge {
        id: edge_id.clone(),
        work: lineage.work.clone(),
        from_epoch: lineage.away_epoch.id.clone(),
        to_epoch: None, // filled by mark_launched (S-G1)
        from_account: away_account.id.clone(),
        to_account: account_a.id.clone(),
        from_provider: away_account.harness.clone(),
        to_provider: account_a.harness.clone(),
        // Fresh-f0 is a pack-labelled fresh start; a resume is T1.
        mechanism: if fresh {
            SwitchMechanism::Pack
        } else {
            SwitchMechanism::NativeResume
        },
        pack_id: None,
        fidelity_shown: Some(if fresh { "F0" } else { "F4" }.to_owned()),
        manifest_sha256: None,
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: Some(lineage.fork_edge.id.clone()),
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: None,
    })?;
    graph.mark_launched(&edge_id, &epoch_id)?;
    graph.mark_confirmed(&edge_id)?;
    let choice_rows: Vec<ManifestEntry> = actions
        .iter()
        .map(|(resolution, _)| ManifestEntry {
            rel_path: resolution.rel_path.clone(),
            sha256: String::new(),
            bytes: 0,
            kind: "conflict-choice".to_owned(),
            disposition: resolution.choice.as_str().to_owned(),
        })
        .collect();
    if !choice_rows.is_empty() {
        graph.append_manifest(&edge_id, &choice_rows)?;
    }
    Ok(())
}

/// Today's date as YYYYMMDD (the Keep-both sibling suffix).
fn chrono_date_compact() -> String {
    date_compact_from_millis(Timestamp::now().millis())
}

/// Civil-from-days (Howard Hinnant) — one suffix, no calendar
/// dependency; pinned by test against known dates.
pub(crate) fn date_compact_from_millis(millis: i64) -> String {
    let days = millis.div_euclid(86_400_000);
    // Civil-from-days (Howard Hinnant's algorithm) — no calendar
    // dependency for one suffix.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    let day = doy - (153 * mp + 2) / 5 + 1;
    format!("{year:04}{month:02}{day:02}")
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
pub fn switch_back_suggestions_impl(
    state: &Arc<AppState>,
) -> AppResult<Vec<SwitchBackSuggestionInfo>> {
    use juggler_core::model::EdgeState;

    let now = Timestamp::now().millis();
    let storage = lock_storage_arc(state)?;
    let accounts = storage.accounts().list()?;
    let graph = storage.switch_graph();
    let mut works = Vec::new();
    for project in storage.projects().list()? {
        works.extend(graph.works_for_project(&project.id)?);
    }
    let mut out = Vec::new();
    for work in &works {
        let edges = graph.edges_for_work(&work.id)?;
        let epochs = graph.epochs_for_work(&work.id)?;
        let epoch_account = |id: &juggler_core::model::EpochId| {
            epochs
                .iter()
                .find(|epoch| &epoch.id == id)
                .map(|epoch| epoch.account.clone())
        };
        for edge in edges
            .iter()
            .filter(|edge| matches!(edge.state, EdgeState::Confirmed))
        {
            // Open excursion: no confirmed edge returns from this
            // away epoch yet.
            let returned = edge.to_epoch.as_ref().is_some_and(|away_epoch| {
                edges.iter().any(|other| {
                    matches!(other.state, EdgeState::Confirmed) && &other.from_epoch == away_epoch
                })
            });
            if returned || edge.to_epoch.is_none() {
                continue;
            }
            // A parent-linked edge is a return/compensation, not a
            // fresh excursion — suggesting its undo would bounce the
            // user straight back.
            if edge.parent_edge_a.is_some() {
                continue;
            }
            let Some(original_account_id) = epoch_account(&edge.from_epoch) else {
                continue;
            };
            let Some(original_account) = accounts
                .iter()
                .find(|account| account.id == original_account_id)
            else {
                continue;
            };
            // The reset trigger: the original account has headroom
            // again (not exhausted, not tight).
            let display = quota_display_inner(&storage, original_account, now);
            let headroom = match display.five_hour.as_ref().map(|w| w.state.as_str()) {
                Some("exhausted") => false,
                Some(_) => display.forecast.tte_min.is_none_or(|tte| tte >= 45.0),
                None => true,
            };
            if !headroom {
                continue;
            }
            let Some(away_session) = edge
                .to_epoch
                .as_ref()
                .and_then(|id| epochs.iter().find(|epoch| &epoch.id == id))
                .and_then(|epoch| epoch.session_ref.clone())
            else {
                continue;
            };
            let away_label = accounts
                .iter()
                .find(|account| account.id == edge.to_account)
                .map(|account| account.label.clone())
                .unwrap_or_default();
            out.push(SwitchBackSuggestionInfo {
                session_id: away_session.to_string(),
                original_account: original_account.label.clone(),
                original_account_id: original_account.id.to_string(),
                // Worded below, once the Accounts are read (that reads
                // storage itself).
                copy: String::new(),
                away_account: away_label,
            });
        }
    }
    drop(storage);
    crate::features::handover::word_switch_back_cards(state, &mut out);
    Ok(out)
}

#[cfg(test)]
#[path = "../../../../juggler-adapters/tests/zcode_fixture/mod.rs"]
mod zcode_fixture;

#[cfg(test)]
mod tests {
    use super::date_compact_from_millis;
    use super::zcode_fixture;

    /// AJ-E1605 Req.6: a pack takes a `ZCode` session's chat from its store,
    /// through its adapter; any other tool's link is still read as its
    /// transcript file.
    #[test]
    fn a_packs_turns_come_from_the_tools_own_store_or_its_transcript_file() {
        use juggler_handover::pack::IrRole;
        let work = tempfile::tempdir().unwrap();
        let mut registry = juggler_adapter::AdapterRegistry::new();
        registry.register(std::sync::Arc::new(juggler_adapters::ZCodeAdapter::new()));
        let state =
            crate::state::AppState::init_with_registry(work.path().join("data"), registry).unwrap();
        let app = work.path().join("zcode-app");
        {
            let conn = zcode_fixture::create(&app);
            zcode_fixture::add(
                &conn,
                &zcode_fixture::Row::plain("sess_1", std::path::Path::new("/w"), 1),
            );
            zcode_fixture::message(
                &conn,
                "sess_1",
                "msg_1",
                "user",
                1,
                &[r#"{"type":"text","text":"Fix the cart"}"#],
            );
            zcode_fixture::message(
                &conn,
                "sess_1",
                "msg_2",
                "assistant",
                2,
                &[r#"{"type":"text","text":"Fixed."}"#],
            );
        }
        let reference =
            juggler_adapters::zcode_sessions::session_ref(&zcode_fixture::db(&app), "sess_1");
        let zcode = juggler_core::model::HarnessId::new(juggler_adapters::ZCODE_ID);
        let turns = super::conversation_turns(&state, &zcode, &reference);
        let said: Vec<(IrRole, &str)> = turns.iter().map(|t| (t.role, t.text.as_str())).collect();
        assert_eq!(
            said,
            [
                (IrRole::User, "Fix the cart"),
                (IrRole::Assistant, "Fixed.")
            ]
        );
        // A tool the registry reads no other way: its transcript file.
        let file = work.path().join("t.jsonl");
        std::fs::write(
            &file,
            "{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
        )
        .unwrap();
        let claude = juggler_core::model::HarnessId::new("claude-code");
        let turns = super::conversation_turns(&state, &claude, &file);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "hello");
    }

    #[test]
    fn date_compact_matches_known_civil_dates() {
        let day = |days: i64| days * 86_400_000;
        assert_eq!(date_compact_from_millis(0), "19700101");
        assert_eq!(date_compact_from_millis(day(20_738)), "20261012");
        assert_eq!(date_compact_from_millis(day(20_739)), "20261013");
        // Leap-year boundary: 2024-02-29 is day 19782.
        assert_eq!(date_compact_from_millis(day(19_782)), "20240229");
    }
}
