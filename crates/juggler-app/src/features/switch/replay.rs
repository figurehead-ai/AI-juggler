//! Replay: the handover brief a target reads when it can't resume natively
//! (AJ-E1541).

use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::LaunchMode;
use juggler_core::model::{Account, SessionId, SessionStatus};

use crate::commands::{lock_storage_arc, now_millis};
use crate::error::{AppError, AppResult};
use crate::features::redaction::{redaction_global_of, redaction_override_of};
use crate::features::switch::back::transcript_side;
use crate::features::switch::dto::SwitchExecutedInfo;
use crate::features::switch::edges::{EdgeCtx, record_edge_failed, record_edge_staged, source_row};
use crate::features::switch::execute::{
    SwitchSides, freeze_source, loop_trigger, tasks_after_switch,
};
use crate::features::switch::undo::record_redaction_decision;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The most a replay brief may weigh, in BYTES: what the target reads
/// from its handover file (AJ-E1541) — about 24k tokens, which a tool
/// reads in a part or two. Newest turns are kept, the newest always;
/// older ones are named by what was asked.
const REPLAY_PROMPT_BYTES: usize = 96 * 1024;

/// What the brief's own words (the Provider's header, the earlier turns'
/// topics) may take on top of the turns.
const REPLAY_HEADER_BYTES: usize = 8 * 1024;

/// AJ-E1304: a cross-Provider switch (T3 replay, beta — D-04/D-11). The
/// source conversation is read in ITS format, redacted under the user's
/// policy for the target Provider (unset blocks, D-14), and primes a new
/// session on the target account in the source's folder. The source file
/// is only read — hash-checked before and after (copy-never-move).
pub(crate) fn switch_replay(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    freeze: juggler_handover::pipeline::FreezeChoice,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
    trigger: &'static str,
    approved: &[String],
) -> AppResult<SwitchExecutedInfo> {
    use juggler_handover::pipeline::SwitchFailure;

    // Review of #132 (M4): what can refuse the switch without the final
    // conversation is checked before anything is stopped.
    let strip = replay_strip(state, sides, redaction_choice)?;
    replayable_turns(&sides.reference.path)?;
    // AJ-E1540: the source stops first, so the replay holds its last turn.
    freeze_source(state, &sides.source, &sides.reference, freeze)?;
    let brief = replay_prompt(state, sides, strip, redaction_unredact)?;
    let (source_sha, source_bytes) = (brief.sha.clone(), brief.bytes);
    let (work, from_epoch, edge) = record_edge_staged(
        state,
        &sides.source,
        &sides.reference,
        &sides.account_a,
        &sides.account_b,
        juggler_core::model::SwitchMechanism::Replay,
    )?;
    let switch = crate::usage_stats::Switch::started(
        sides.account_a.harness.as_str(),
        sides.account_b.harness.as_str(),
        "pack_replay",
        loop_trigger(&sides.source, trigger),
    );
    let project = {
        let storage = lock_storage_arc(state)?;
        let id = sides.source.project.clone().ok_or_else(|| AppError::Io {
            context: "hand over",
            message: "AI Juggler doesn't know which project this session belongs to, so it can't switch it".into(),
        })?;
        let mut project = storage.projects().get(&id)?;
        // The continuation runs where the source ran (its worktree).
        project.root.clone_from(&sides.source.cwd);
        project
    };
    let launched = launch_with_handover(state, sides, &edge, &brief, &project, approved);
    // Copy-never-move, checked after the launch as the pipeline does.
    let checked = launched.and_then(|started| {
        juggler_adapters::session_file_sha256(&sides.reference.path)
            .is_ok_and(|(now, _)| now == source_sha)
            .then_some(started)
            .ok_or_else(|| {
                SwitchFailure::StagingAborted("the source changed during the switch".into())
            })
    });
    let (session, pid) = match checked {
        Ok(value) => value,
        Err(failure) => {
            switch.failed(failure.stage());
            let error = AppError::switch(failure, &sides.account_a.label, &sides.account_b.label);
            record_edge_failed(state, &edge, &error);
            return Err(error);
        }
    };
    record_replay_succeeded(
        state,
        sides,
        &EdgeCtx {
            work,
            from_epoch,
            edge: edge.clone(),
            session: session.clone(),
        },
        source_sha,
        source_bytes,
    )?;
    switch.succeeded();
    tasks_after_switch(state, &session, &sides.account_a, &sides.account_b);
    record_redaction_decision(
        state,
        &edge,
        &sides.account_b.harness,
        redaction_choice,
        redaction_unredact,
    );
    Ok(SwitchExecutedInfo {
        session_id: session.to_string(),
        pid,
        edge_id: edge.to_string(),
        source_session_id: sides.source.id.to_string(),
        continued_in: "cli".to_owned(),
        conversation_key: None,
        // Across tools a conversation replays into a new one: its family
        // can't follow (the preflight says so).
        family_moved: 0,
        family_nests: String::new(),
        family_place: None,
        automation_moved: Vec::new(),
        automation_copy_place: None,
        automation_original_waits: false,
        automation_kept: Vec::new(),
    })
}

/// The replay's prime prompt plus the source's hash and size: the
/// conversation read in its own format, redacted under the user's policy
/// for the target Provider (unset blocks — D-14), bounded, and handed to
/// the target's prime.
/// Whether the replay strips secrets: the choice made on this switch, else
/// the policy. Unset, or "Ask each time" with no answer, refuses (the
/// conversation never leaves unredacted by default). Reads no
/// conversation, so it runs before the source is stopped.
fn replay_strip(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    redaction_choice: Option<&str>,
) -> AppResult<bool> {
    use juggler_handover::redaction::RedactionPolicy;
    let refused = |message: String| AppError::Io {
        context: "hand over",
        message,
    };
    let policy = {
        let storage = lock_storage_arc(state)?;
        juggler_handover::redaction::resolve_policy(
            redaction_global_of(&storage),
            redaction_override_of(&storage, sides.account_b.harness.as_str()),
        )
    };
    match redaction_choice
        .and_then(RedactionPolicy::from_wire)
        .or(policy)
    {
        Some(RedactionPolicy::Strip) => Ok(true),
        Some(RedactionPolicy::Keep) => Ok(false),
        Some(RedactionPolicy::AskEachTime) | None => Err(refused(
            "Choose how secrets in the conversation are handled for this switch — \
             a replay sends the conversation to the other Provider."
                .to_owned(),
        )),
    }
}

/// The conversation's turns, refusing one that can't be read or has none.
fn replayable_turns(path: &std::path::Path) -> AppResult<Vec<juggler_handover::pack::IrTurn>> {
    let refused = |message: &str| AppError::Io {
        context: "hand over",
        message: message.to_owned(),
    };
    let (_, turns) = transcript_side(path).ok_or_else(|| {
        refused("Couldn't read this session's conversation, so it can't be replayed.")
    })?;
    if turns.is_empty() {
        return Err(refused("This session has no conversation to replay yet."));
    }
    Ok(turns)
}

/// AJ-E1541: the brief goes in a handover file; the target starts with a
/// short prompt that points at it (and may read its folder).
fn launch_with_handover(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    edge: &juggler_core::model::SwitchEdgeId,
    brief: &ReplayBrief,
    project: &juggler_core::model::ProjectRecord,
    approved: &[String],
) -> Result<(SessionId, u32), juggler_handover::pipeline::SwitchFailure> {
    use juggler_handover::pipeline::SwitchFailure;
    let file = write_handover(state, sides, edge, brief, approved)
        .map_err(|error| SwitchFailure::StagingAborted(error.to_string()))?;
    let mode = LaunchMode::Primed {
        prompt: crate::features::handover::pointer(
            &replay_header(state, &sides.account_b.harness),
            &file,
        ),
        read_dirs: file
            .parent()
            .map(std::path::Path::to_path_buf)
            .into_iter()
            .collect(),
    };
    crate::crossmachine::launch_here(
        state,
        &sides.profile_b,
        &sides.account_b,
        project,
        &mode,
        None,
        "switch",
    )
    .map_err(|error| SwitchFailure::LaunchFailed(error.to_string()))
}

/// A replay's brief: its text, the source's hash and size, how many
/// turns it holds and leaves out, and what the keys check hid and left
/// as it was (kinds and counts).
struct ReplayBrief {
    text: String,
    sha: String,
    bytes: u64,
    included: usize,
    left_out: usize,
    withheld: Vec<juggler_handover::pack::RedactionCount>,
    kept: Vec<juggler_handover::pack::RedactionCount>,
}

/// AJ-E1541: writes the brief, with the rules files in play, as the
/// switch's handover file; returns its path. D-51: with this project's
/// notes, quoted, when the user ticked them (`approved`).
fn write_handover(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    edge: &juggler_core::model::SwitchEdgeId,
    brief: &ReplayBrief,
    approved: &[String],
) -> std::io::Result<PathBuf> {
    use crate::features::handover;
    let specs: Vec<_> = [&sides.account_a.harness, &sides.account_b.harness]
        .into_iter()
        .filter_map(|harness| state.registry.get(harness).map(|adapter| adapter.spec()))
        .collect();
    let rules = handover::rules_in_play(&sides.source.cwd, &specs);
    let notes = crate::features::switch_setup::memory::handover_notes(
        state,
        &sides.account_a,
        &sides.account_b,
        &sides.source.cwd,
        approved,
    );
    let text = [
        Some(brief.text.trim_end().to_owned()),
        handover::rules_section(&rules),
        notes.as_ref().map(|notes| notes.text.clone()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n")
        + "\n";
    let side = |account: &Account| handover::Side {
        tool: account.harness.to_string(),
        account: account.label.clone(),
    };
    handover::write(
        &state.data_dir,
        &text,
        &handover::HandoverState {
            format: handover::FORMAT.to_owned(),
            version: handover::VERSION,
            edge: edge.to_string(),
            from: side(&sides.account_a),
            to: side(&sides.account_b),
            folder: sides.source.cwd.display().to_string(),
            written_ms: now_millis(),
            turns_included: brief.included,
            turns_left_out: brief.left_out,
            rules_files: rules,
            notes_quoted: notes
                .as_ref()
                .map(|notes| notes.quoted.clone())
                .unwrap_or_default(),
            notes_left_out: notes.map(|notes| notes.left_out).unwrap_or_default(),
            withheld: Some(brief.withheld.clone()),
            kept: brief.kept.clone(),
        },
    )
}

fn replay_prompt(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    strip: bool,
    redaction_unredact: &[u32],
) -> AppResult<ReplayBrief> {
    use juggler_handover::pipeline::SwitchFailure;
    use juggler_handover::redaction::strip_segments_counted;

    let stopped =
        |failure| AppError::switch(failure, &sides.account_a.label, &sides.account_b.label);
    let (sha, len) = juggler_adapters::session_file_sha256(&sides.reference.path)
        .map_err(|_| stopped(SwitchFailure::UnresolvedSource))?;
    if !juggler_adapters::last_line_parses(&sides.reference.path) {
        return Err(stopped(SwitchFailure::TornTail));
    }
    // The preview's own segments, so a per-item un-redact lines up.
    let turns = replayable_turns(&sides.reference.path)?;
    let segments: Vec<String> = turns.iter().map(|turn| turn.text.clone()).collect();
    let keep: std::collections::BTreeSet<usize> = redaction_unredact
        .iter()
        .filter_map(|index| usize::try_from(*index).ok())
        .collect();
    let stripped = strip_segments_counted(&segments, strip, &keep);
    let kept = stripped.kept();
    let (prime_turns, left_out) = replay_turns(&turns, stripped.texts);
    let included = prime_turns.len() - usize::from(left_out > 0);
    let prompt = state
        .registry
        .handover(&sides.account_b.harness)
        .and_then(|support| support.prime_prompt(&prime_turns).ok())
        .unwrap_or_else(|| crate::features::handover::plain_prime(&prime_turns));
    // AJ-E1541: where the work stands, after the conversation: the task
    // list as last set, and what changed in the project and isn't
    // committed, so the other tool continues the work, not just the chat.
    let tasks = juggler_adapters::read_session_file(&sides.reference.path)
        .ok()
        .and_then(|bytes| juggler_adapters::parse_any(&bytes).tasks);
    let prompt = match work_state_note(tasks.as_deref(), &sides.source.cwd) {
        Some(note) => format!("{prompt}\n\n{note}"),
        None => prompt,
    };
    Ok(ReplayBrief {
        text: prompt,
        sha,
        bytes: len,
        included,
        left_out,
        withheld: stripped.withheld,
        kept,
    })
}

/// AJ-E1541: "where the work stands" for a handover: the task list as the
/// last call set it, and the project's uncommitted changes. `None` when
/// there's neither.
fn work_state_note(
    tasks: Option<&[juggler_adapters::tool_summary::TaskItem]>,
    cwd: &std::path::Path,
) -> Option<String> {
    use std::fmt::Write as _;
    let changes = crate::worktree::work_changes(cwd).filter(|changes| !changes.files.is_empty());
    let tasks = tasks.filter(|tasks| !tasks.is_empty());
    if changes.is_none() && tasks.is_none() {
        return None;
    }
    let mut note = String::from("Where the work stands now:\n");
    if let Some(tasks) = tasks {
        let _ = write!(
            note,
            "\nTask list:\n{}",
            juggler_adapters::tool_summary::render_tasks(tasks)
        );
    }
    if let Some(changes) = changes {
        let branch = changes
            .branch
            .as_deref()
            .map(|branch| format!(" on {branch}"))
            .unwrap_or_default();
        let _ = writeln!(note, "\nNot committed yet{branch}:");
        for file in &changes.files {
            let _ = writeln!(note, "  {file}");
        }
        if changes.more > 0 {
            let _ = writeln!(note, "  …and {} more", changes.more);
        }
        if let Some(stat) = &changes.stat {
            let _ = writeln!(note, "  ({stat})");
        }
    }
    Some(note.trim_end().to_owned())
}

/// Records a succeeded replay: the source rests (`HandedOver`), the
/// target epoch starts (no native id yet — the vendor assigns it), the
/// edge launches and confirms, and the source's hash lands as a
/// `replay` manifest row.
fn record_replay_succeeded(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    ctx: &EdgeCtx,
    source_sha: String,
    source_bytes: u64,
) -> AppResult<()> {
    use juggler_core::model::{Epoch, EpochId, ManifestEntry};

    let storage = lock_storage_arc(state)?;
    if source_row(&sides.source).is_some() {
        let mut handed = sides.source.clone();
        handed.status = SessionStatus::HandedOver;
        storage.sessions().update(&handed)?;
    }
    let graph = storage.switch_graph();
    let to_epoch = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: to_epoch.clone(),
        work: ctx.work.clone(),
        session_ref: Some(ctx.session.clone()),
        native_id: None,
        account: sides.account_b.id.clone(),
        provider: sides.account_b.harness.clone(),
        resumed_from_epoch: Some(ctx.from_epoch.clone()),
        machine: None,
    })?;
    graph.mark_launched(&ctx.edge, &to_epoch)?;
    graph.mark_confirmed(&ctx.edge)?;
    graph.append_manifest(
        &ctx.edge,
        &[ManifestEntry {
            rel_path: sides
                .reference
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            sha256: source_sha,
            bytes: source_bytes,
            kind: "transcript".to_owned(),
            disposition: "replay".to_owned(),
        }],
    )?;
    Ok(())
}

/// The replay's turns: user and assistant text after redaction, the
/// newest kept within [`REPLAY_PROMPT_BYTES`] as they will be rendered
/// (quoted); when older turns are left out the first turn says how many
/// and what was asked in them (never a silent cut). Also returns how
/// many were left out.
fn replay_turns(
    turns: &[juggler_handover::pack::IrTurn],
    texts: Vec<String>,
) -> (Vec<juggler_adapter::handover::TurnText>, usize) {
    use juggler_adapter::handover::TurnText;
    let mut out: Vec<TurnText> = turns
        .iter()
        .zip(texts)
        .map(|(turn, text)| TurnText {
            uuid: turn.uuid.clone(),
            role: match turn.role {
                juggler_handover::pack::IrRole::User => "user",
                juggler_handover::pack::IrRole::Assistant => "assistant",
            },
            text,
        })
        .collect();
    let budget = REPLAY_PROMPT_BYTES - REPLAY_HEADER_BYTES;
    // Newest first, while they fit — measured as rendered (quoted) bytes.
    let rendered = |turn: &TurnText| turn.quoted().len() + 1;
    let mut total = 0usize;
    let mut keep_from = out.len();
    for (index, turn) in out.iter().enumerate().rev() {
        let size = rendered(turn);
        if total + size > budget {
            break;
        }
        total += size;
        keep_from = index;
    }
    // The newest turn always travels — cut on a character boundary until
    // its rendering fits (never an empty replay).
    if keep_from == out.len()
        && let Some(last) = out.last_mut()
    {
        let note = "\n(… the rest of this turn is cut to fit.)";
        let mut keep = last.text.len().min(budget);
        loop {
            while !last.text.is_char_boundary(keep) {
                keep -= 1;
            }
            let candidate = TurnText {
                uuid: None,
                role: last.role,
                text: format!("{}{note}", &last.text[..keep]),
            };
            if rendered(&candidate) <= budget || keep == 0 {
                last.text = candidate.text;
                break;
            }
            // Quoting adds bytes per line: shrink by the overshoot.
            keep = keep.saturating_sub((rendered(&candidate) - budget).max(64));
        }
        keep_from = out.len() - 1;
    }
    if keep_from > 0 {
        let left_out: Vec<TurnText> = out.drain(..keep_from).collect();
        out.insert(
            0,
            TurnText {
                uuid: None,
                role: "user",
                text: crate::features::handover::earlier_turns(&left_out),
            },
        );
    }
    (out, keep_from)
}

/// The replay brief when the target Provider has no prime of its own.
/// The first line every replay into `harness` opens with (its tool's
/// prime, else the plain one): how a replay's conversation is told apart.
/// The words a replay into `harness` opens with (its tool's prime with no
/// turns, else the plain one): the handover's preface.
pub(crate) fn replay_preface(
    state: &Arc<AppState>,
    harness: &juggler_core::model::HarnessId,
) -> String {
    state
        .registry
        .handover(harness)
        .and_then(|support| support.prime_prompt(&[]).ok())
        .unwrap_or_else(|| crate::features::handover::plain_prime(&[]))
}

pub(crate) fn replay_header(
    state: &Arc<AppState>,
    harness: &juggler_core::model::HarnessId,
) -> String {
    let prime = state
        .registry
        .handover(harness)
        .and_then(|support| support.prime_prompt(&[]).ok())
        .unwrap_or_else(|| crate::features::handover::plain_prime(&[]));
    prime.lines().next().unwrap_or_default().to_owned()
}
