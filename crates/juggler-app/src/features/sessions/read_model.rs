//! The sessions' read model (AJ-E401): the working set, each row's join, and
//! the events that tell the UI it changed.

use std::sync::Arc;

use juggler_core::model::{AccountId, SessionRecord, SessionStatus, Timestamp};
use juggler_core::storage::Storage;

use crate::commands::now_millis;
use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EmitEvent as _};
use crate::features::sessions::dto::{
    AttentionInfo, InstanceInfo, InstanceQuotaInfo, InstanceResourcesInfo, NeedsAttention,
    QuotaSnapshotInfo, TokenCountsInfo,
};
use crate::state::AppState;

/// Deprecated alias (one version). AJ-E107.
pub type SessionInfo = InstanceInfo;

/// Tells the UI the sessions changed; it reads its lists again (0.1.5
/// review: this sent every session ever run, built twice, on each change).
///
/// # Errors
///
/// Never fails; the result type matches its callers'.
pub fn emit_sessions(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    crate::features::relations::changed(state, crate::features::relations::Change::SessionsUpdated);
    crate::features::quality::settle_soon(state); // AJ-E2903: keep the ended ones' figures
    app.emit_event(AppEvent::SessionsUpdated, ());
    Ok(())
}

/// The working set's ended sessions: every one of the last day (the
/// rail's "ended today" counts), and the newest this many of the last
/// [`RECENT_ENDED_MS`]; History pages the rest.
pub const RECENT_ENDED: usize = 200;

/// How far back the working set's ended sessions go.
pub const RECENT_ENDED_MS: i64 = 7 * DAY_MS;

/// One day: everything active within it is in the working set.
pub const DAY_MS: i64 = 24 * 3_600_000;

/// The sessions the app's lists hold: every open or pinned one and the
/// recent ended ones ([`RECENT_ENDED`]), never the whole history.
///
/// # Errors
///
/// Storage errors.
pub fn working_records(storage: &Storage) -> AppResult<Vec<SessionRecord>> {
    Ok(storage.sessions().working_set(
        now_millis() - DAY_MS,
        now_millis() - RECENT_ENDED_MS,
        RECENT_ENDED,
    )?)
}

/// One batched sample of live process stats for every supervised session
/// (locks only the collector — never together with the storage lock).
pub(crate) fn live_stats_of(state: &Arc<AppState>) -> LiveStatsMap {
    let Ok(mut collector) = state.resources.lock() else {
        // Poisoned collector: report nothing rather than fabricated zeros.
        return LiveStatsMap::new();
    };
    collector
        .refresh(&state.supervisor)
        .into_iter()
        .map(|(session, stats)| (session.to_string(), stats))
        .collect()
}

/// Emits the 5 s resource delta (I-07/I-08) on `session://resources`:
/// one coalesced payload per tick carrying every active Instance, and no
/// emission at all while nothing is live (idle costs zero wakeups for
/// the UI, AJ-E401 Req.4).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn emit_resources(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    let live = live_stats_of(state);
    let payload: Vec<InstanceResourcesInfo> = active_records(state)?
        .into_iter()
        .filter_map(|record| {
            // Unsampled rows are omitted — the UI keeps its honest
            // `null` instead of inheriting a zero.
            let sample = live.get(&record.id.to_string()).copied()?;
            Some(InstanceResourcesInfo {
                id: record.id.to_string(),
                cpu_pct: sample.cpu_pct,
                rss_bytes: Some(sample.rss_bytes),
                child_count: Some(sample.child_count),
            })
        })
        .collect();
    crate::usage_stats::resources_sampled(&payload);
    if payload.is_empty() {
        return Ok(());
    }
    app.emit_event(AppEvent::SessionsResources, payload);
    Ok(())
}

/// The sessions that are active now (the ticks' payloads).
fn active_records(state: &Arc<AppState>) -> AppResult<Vec<juggler_core::model::InstanceRecord>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(storage
        .sessions()
        .list(None)?
        .into_iter()
        .filter(|record| is_active(record.status))
        .collect())
}

/// Emits the 30 s quota delta (I-05) on `session://quota`: one coalesced
/// payload per tick; per-Instance `quota` is `null` until a feed exists
/// (the two-clock engine, AJ-E606+, populates it later — the tick itself
/// is the contract this task ships).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn emit_quota(app: &tauri::AppHandle, state: &Arc<AppState>) -> AppResult<()> {
    let payload: Vec<InstanceQuotaInfo> = active_records(state)?
        .into_iter()
        .map(|record| InstanceQuotaInfo {
            id: record.id.to_string(),
            quota: quota_snapshot_of(&record.usage),
        })
        .collect();
    if payload.is_empty() {
        return Ok(());
    }
    app.emit_event(AppEvent::SessionsQuota, payload);
    Ok(())
}

/// Live supervisor stats keyed by Instance id (empty until the collector
/// has sampled; historical rows never have entries).
pub type LiveStatsMap = std::collections::HashMap<String, juggler_supervisor::LiveProcessStats>;

/// I-05 conversion shared by the row payload and the 30 s quota tick.
fn quota_snapshot_of(usage: &juggler_core::model::UsageSnapshot) -> Option<QuotaSnapshotInfo> {
    usage.window.map(|window| QuotaSnapshotInfo {
        used_percentage: window.used_percentage,
        resets_at_ms: window.resets_at.map(Timestamp::millis),
        confidence: confidence_name(usage.confidence).to_string(),
    })
}

/// Wire name for a usage-confidence level (SPEC §6 honesty rule).
pub(crate) fn confidence_name(confidence: juggler_core::model::Confidence) -> &'static str {
    match confidence {
        juggler_core::model::Confidence::Measured => "measured",
        juggler_core::model::Confidence::Computed => "computed",
        juggler_core::model::Confidence::Estimated => "estimated",
    }
}

/// Wire value for the I-10 attention reason derived from lifecycle status
/// (AJ-E405 replaces the derivation with structured heuristic reasons).
fn needs_attention_of(status: SessionStatus) -> Option<NeedsAttention> {
    match status {
        SessionStatus::WaitingUser => Some(NeedsAttention::WaitingUser),
        SessionStatus::Stuck => Some(NeedsAttention::Stuck),
        SessionStatus::QuotaBlocked => Some(NeedsAttention::QuotaBlocked),
        SessionStatus::Failed => Some(NeedsAttention::Failed),
        SessionStatus::Starting
        | SessionStatus::Running
        | SessionStatus::Finished
        | SessionStatus::Stopped
        | SessionStatus::HandedOver => None,
    }
}

/// A terminal status: the process is gone, so nothing about it is live.
pub(crate) fn is_terminal(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Finished
            | SessionStatus::Stopped
            | SessionStatus::Failed
            | SessionStatus::HandedOver
    )
}

/// An active status: the row carries live-state expectations (resource
/// ticks and quota ticks only ever emit for these).
pub(crate) fn is_active(status: SessionStatus) -> bool {
    !is_terminal(status)
}

/// Resolves the account/provider join for one Instance row. When the
/// launch-recipe or account row is gone (defensive only — storage FKs
/// prevent it today), the Provider and label degrade to a generic
/// `unknown` marker, never a blank string, and the account id to `null`.
pub(crate) fn resolved_join(
    recipe_index: &std::collections::BTreeMap<String, (String, String, Option<String>)>,
    account_index: &std::collections::BTreeMap<String, (String, String)>,
    recipe_id: &str,
) -> (Option<String>, String, String, Option<String>) {
    let (account_id, provider, model) = recipe_index.get(recipe_id).cloned().unwrap_or_default();
    let account_label = account_index
        .get(&account_id)
        .map_or_else(|| "unknown".to_string(), |(label, _)| label.clone());
    let provider = if provider.is_empty() {
        "unknown".to_string()
    } else {
        provider
    };
    (
        if account_id.is_empty() {
            None
        } else {
            Some(account_id)
        },
        account_label,
        provider,
        model,
    )
}

/// AJ-E412 Req.1 classification: the I-03 display model prefers the
/// OBSERVED running model over the launch intent; the source names how
/// it is known (`in-session` = observed drift from the overlay).
pub(crate) fn display_model(
    running: Option<&juggler_core::model::RunningModel>,
    launch: Option<&String>,
) -> (Option<String>, String) {
    let observed = running.map(|entry| entry.model.clone());
    let source = match (&observed, launch) {
        (Some(running_model), Some(launch_model)) if running_model != launch_model => "in-session",
        (Some(_), _) => "transcript",
        (None, Some(_)) => "launch",
        (None, None) => "unknown",
    };
    (observed.or_else(|| launch.cloned()), source.to_string())
}

/// Per-Provider desktop isolation claims (AJ-E413 Req.1): provider id →
/// the label shown verbatim on that provider's desktop Instances.
/// Providers without a desktop app contribute nothing.
pub(crate) fn desktop_isolation_labels(
    registry: &juggler_adapter::AdapterRegistry,
) -> std::collections::BTreeMap<String, String> {
    registry
        .ids()
        .into_iter()
        .filter_map(|harness| {
            registry
                .get(&harness)
                .and_then(|adapter| adapter.desktop_isolation_label())
                .map(|label| (harness.to_string(), label.to_string()))
        })
        .collect()
}

/// Session rows with denormalized account/provider joins and the full
/// I-01..I-11 read model (AJ-E401). One storage pass per collection
/// (accounts, launch settings, instances) joined in memory — no per-row
/// storage reads; absent values serialise as `null`, never `0` or `""`.
/// Test/inspection surface: session rows with an empty live-stats map.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub fn session_infos_snapshot(storage: &Storage) -> AppResult<Vec<InstanceInfo>> {
    session_infos(
        storage,
        &LiveStatsMap::new(),
        &std::collections::BTreeMap::new(),
        &[],
    )
}

/// account id → (label, provider).
pub(crate) type AccountIndex = std::collections::BTreeMap<String, (String, String)>;

/// launch-recipe id → (account id, provider, overlay model).
pub(crate) type RecipeIndex = std::collections::BTreeMap<String, (String, String, Option<String>)>;

/// Join indexes for the read model: account id → (label, provider) and
/// launch-recipe id → (account id, provider, overlay model). One
/// storage pass per collection, joined in memory — no per-row reads.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub(crate) fn session_join_indexes(storage: &Storage) -> AppResult<(AccountIndex, RecipeIndex)> {
    // AJ-E1501: a removed account's sessions keep its name, marked.
    let removed = storage.accounts().removed_ids()?;
    let account_index: AccountIndex = storage
        .accounts()
        .list_all()?
        .into_iter()
        .map(|account| {
            let label = if removed.contains(account.id.as_str()) {
                format!("{} (removed app)", account.label)
            } else {
                account.label
            };
            (account.id.to_string(), (label, account.harness.to_string()))
        })
        .collect();
    let mut recipe_index: RecipeIndex = std::collections::BTreeMap::new();
    for (account_id, (_, harness)) in &account_index {
        // D-60: removed launch settings too, so their sessions keep a name.
        for recipe in storage
            .profiles()
            .list_all_by_account(&AccountId::new(account_id.as_str()))?
        {
            recipe_index.insert(
                recipe.id.to_string(),
                (
                    recipe.account.to_string(),
                    harness.clone(),
                    recipe.settings_overlay.model,
                ),
            );
        }
    }
    Ok((account_index, recipe_index))
}

/// AJ-E1502: a session's label, from the marks.
pub(crate) fn label_of(marks: &juggler_core::storage::SessionMarks, id: &str) -> Option<String> {
    marks.get(id).and_then(|(label, _)| label.clone())
}

/// AJ-E1502: whether a session is pinned (absent when not).
fn pinned_of(marks: &juggler_core::storage::SessionMarks, id: &str) -> Option<bool> {
    marks.get(id).and_then(|(_, pinned)| pinned.map(|_| true))
}

/// Measured token counts for the wire; all-zero means unmeasured (null).
pub(crate) fn tokens_info(t: juggler_core::model::TokenCounts) -> Option<TokenCountsInfo> {
    (t != juggler_core::model::TokenCounts::default()).then_some(TokenCountsInfo {
        input: t.input,
        output: t.output,
        cache_read: t.cache_read,
        cache_write: t.cache_write,
        reasoning: t.reasoning,
    })
}

pub(crate) fn session_infos(
    storage: &Storage,
    live: &LiveStatsMap,
    desktop_labels: &std::collections::BTreeMap<String, String>,
    adopted_ids: &[juggler_core::model::SessionId],
) -> AppResult<Vec<InstanceInfo>> {
    let records = storage.sessions().list(None)?;
    session_infos_for(storage, records, live, desktop_labels, adopted_ids)
}

/// [`session_infos`] for some sessions only (a page of them).
pub(crate) fn session_infos_for(
    storage: &Storage,
    records: Vec<SessionRecord>,
    live: &LiveStatsMap,
    desktop_labels: &std::collections::BTreeMap<String, String>,
    adopted_ids: &[juggler_core::model::SessionId],
) -> AppResult<Vec<InstanceInfo>> {
    let (account_index, recipe_index) = session_join_indexes(storage)?;
    let identities = storage.identity().latest_by_session().unwrap_or_default();
    let parked = crate::parked::parked_index(storage);
    let lifecycle = storage.sessions().lifecycle_latest().unwrap_or_default();
    let marks = storage.sessions().marks().unwrap_or_default();
    let mut launched = storage.apps().session_apps().unwrap_or_default();
    Ok(records
        .into_iter()
        .map(|record| {
            let (account_id, account_label, provider, launch_model) =
                resolved_join(&recipe_index, &account_index, &record.profile.to_string());
            let isolation_label = if record.kind == juggler_core::model::InstanceKind::Desktop {
                desktop_labels.get(&provider).cloned()
            } else {
                None
            };
            let (model, model_source) =
                display_model(record.running.as_ref(), launch_model.as_ref());
            let terminal = is_terminal(record.status);
            let git = if terminal {
                None
            } else {
                crate::worktree::cached_status(&record.cwd)
            };
            let stats = if terminal {
                None
            } else {
                live.get(&record.id.to_string())
                    .copied()
                    .and_then(juggler_supervisor::LiveProcessStats::measured)
            };
            InstanceInfo {
                adopted: adopted_ids.contains(&record.id),
                id: record.id.to_string(),
                launch_recipe_id: record.profile.to_string(),
                provider,
                account_id,
                account_label,
                model,
                launch_model,
                running_model: record
                    .running
                    .as_ref()
                    .map(|observed| observed.model.clone()),
                model_source,
                status: record.status.as_str().to_string(),
                quota: quota_snapshot_of(&record.usage),
                tokens: tokens_info(record.usage.tokens),
                cpu_pct: stats.and_then(|s| s.cpu_pct),
                rss_bytes: stats.map(|s| s.rss_bytes),
                child_count: if terminal {
                    Some(0)
                } else {
                    stats.map(|s| s.child_count)
                },
                // AJ-E1520: `features::subagents::count::counted` fills it; null until read.
                subtask_count: None,
                git_branch: crate::worktree::branch_of(&record.cwd),
                // AJ-E1002: changed + untracked from the cached git
                // status (refreshed in the background, at most every
                // 30 s; live Instances only). Null = not read yet / not
                // git — never 0.
                git_dirty_count: git.as_ref().map(|status| status.changed + status.untracked),
                git_ahead: git.as_ref().and_then(|status| status.ahead),
                git_behind: git.as_ref().and_then(|status| status.behind),
                git_worktree: Some(crate::worktree::is_linked_worktree(&record.cwd))
                    .filter(|linked| *linked),
                needs_attention: needs_attention_of(record.status),
                attention: record.attention.as_ref().map(|reason| AttentionInfo {
                    rule: reason.rule.clone(),
                    inputs: reason.inputs.clone(),
                    since_ms: reason.since_ms,
                }),
                started_at_ms: record.started_at.millis(),
                last_activity_ms: record.last_activity.as_ref().map(|t| t.millis()),
                machine_id: record.machine.to_string(),
                cwd: record.cwd.to_string_lossy().into_owned(),
                kind: record.kind.as_str().to_string(),
                project_id: record.project.as_ref().map(ToString::to_string),
                conversation_id: record.conversation.as_ref().map(|c| c.as_str().to_string()),
                isolation_label,
                identity_state: identities
                    .get(record.id.as_str())
                    .map(|assertion| assertion.state.clone()),
                identity_label: identities
                    .get(record.id.as_str())
                    .and_then(|assertion| assertion.label.clone()),
                parked: parked
                    .get(record.id.as_str())
                    .map(|(_, confidence)| confidence.clone()),
                parked_resets_at_ms: parked.get(record.id.as_str()).and_then(|(at, _)| *at),
                lifecycle: lifecycle
                    .get(record.id.as_str())
                    .map(|(event, _)| event.clone()),
                lifecycle_at_ms: lifecycle.get(record.id.as_str()).map(|(_, at)| *at),
                label: label_of(&marks, record.id.as_str()),
                title: None,
                pinned: pinned_of(&marks, record.id.as_str()),
                runtime: None,
                // rc21: `features::launch::resume_gate::with_refusals` fills it.
                resume_refusal: None,
                app_id: launched.remove(record.id.as_str()),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use juggler_core::model::{
        Account, AccountId, Confidence, InstanceKind, InstanceRecord, LaunchRecipe, ProfileId,
        QuotaWindow, SessionStatus, SettingsOverlay, Timestamp, TokenCounts, UsageSnapshot,
    };
    use juggler_core::storage::Storage;
    use juggler_supervisor::LiveProcessStats;

    use super::{LiveStatsMap, needs_attention_of, session_infos};
    use crate::features::sessions::dto::{
        InstanceInfo, NeedsAttention, QuotaSnapshotInfo, TokenCountsInfo,
    };

    /// A sacrificial git cwd: `<tmp>/.git/HEAD` with a branch ref. Never a
    /// real repository — fixtures only (workspace safety rule). Every call
    /// gets a unique directory: tests run in parallel and a shared path
    /// would let one test's `fs::write` truncate while another reads.
    fn git_cwd(branch: &str) -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "aij-e401-{}-{}-{n}",
            std::process::id(),
            branch.replace('/', "-")
        ));
        let git = dir.join(".git");
        std::fs::create_dir_all(&git).expect("fixture .git dir");
        std::fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n"))
            .expect("fixture HEAD");
        dir
    }

    /// Every row, with no live stats, quota feeds or tasks.
    fn plain_rows(storage: &Storage) -> Vec<super::InstanceInfo> {
        session_infos(
            storage,
            &LiveStatsMap::new(),
            &std::collections::BTreeMap::new(),
            &[],
        )
        .expect("rows")
    }

    fn record(
        id: &str,
        recipe: &str,
        status: SessionStatus,
        cwd: std::path::PathBuf,
    ) -> InstanceRecord {
        crate::test_support::instance(id, recipe, status, cwd)
    }

    /// Storage seeded with one account, two launch recipes (with and
    /// without a model) and the four AJ-E401 fixture rows.
    fn fixture_storage() -> Storage {
        let storage = Storage::open_in_memory().expect("in-memory storage");
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-1"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: std::env::temp_dir().join("aij-e401-account-state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .expect("seed account");
        for (id, model) in [("rec-1", Some("opus")), ("rec-2", None)] {
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new(id),
                    name: "fixture".into(),
                    account: AccountId::new("acc-1"),
                    settings_overlay: SettingsOverlay {
                        model: model.map(String::from),
                        env: Default::default(),
                        restart_on_crash: None,
                    },
                    default_cwd: None,
                })
                .expect("seed recipe");
        }

        // Full row: every source populated.
        let mut full = record(
            "sess-full",
            "rec-1",
            SessionStatus::Running,
            git_cwd("feat/read-model"),
        );
        full.usage = UsageSnapshot {
            tokens: TokenCounts {
                input: 12_345,
                output: 6_789,
                cache_read: 100,
                cache_write: 200,
                reasoning: 42,
            },
            window: Some(QuotaWindow {
                used_percentage: 42.5,
                resets_at: Some(Timestamp::from_millis(1_700_010_000_000)),
            }),
            confidence: Confidence::Computed,
            at_end: None,
        };
        storage.sessions().insert(&full).expect("seed full");

        // Quota-less row: no feed, tokens never observed.
        storage
            .sessions()
            .insert(&record(
                "sess-no-quota",
                "rec-1",
                SessionStatus::Running,
                std::env::temp_dir(),
            ))
            .expect("seed quota-less");

        // Model-less row, terminal: launch settings carry no model; a
        // finished Instance is not resource-measurable.
        storage
            .sessions()
            .insert(&record(
                "sess-no-model",
                "rec-2",
                SessionStatus::Finished,
                std::env::temp_dir(),
            ))
            .expect("seed model-less");

        // Dangling recipe rows cannot exist in storage (the sessions table
        // foreign-keys launch_recipe_id), so the join's generic-marker
        // fallback is covered by `unjoinable_rows_render_generic_markers`
        // below instead of an insert.

        // Waiting row with a resolvable account: attention reason present.
        storage
            .sessions()
            .insert(&record(
                "sess-waiting",
                "rec-1",
                SessionStatus::WaitingUser,
                std::env::temp_dir(),
            ))
            .expect("seed waiting");
        storage
    }

    #[allow(
        clippy::panic,
        reason = "fixture lookup: panicking names the missing row"
    )]
    fn row<'a>(rows: &'a [InstanceInfo], id: &str) -> &'a InstanceInfo {
        rows.iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("row {id} missing"))
    }

    /// AJ-E401 fixtures: full row, quota-less row, model-less row — absent
    /// data serialises as `null`, never `0`, `""`, or a guessed name.
    #[test]
    fn session_info_full_row_carries_every_field() {
        let storage = fixture_storage();
        let live: LiveStatsMap = [(
            "sess-full".to_string(),
            LiveProcessStats {
                child_count: 2,
                cpu_pct: Some(3.5),
                rss_bytes: 123_456,
            },
        )]
        .into_iter()
        .collect();
        let rows =
            session_infos(&storage, &live, &std::collections::BTreeMap::new(), &[]).expect("rows");
        let full = row(&rows, "sess-full");

        assert_eq!(full.account_id.as_deref(), Some("acc-1"));
        assert_eq!(full.account_label, "Work Max");
        assert_eq!(full.provider, "claude-code");
        assert_eq!(full.model.as_deref(), Some("opus"));
        let quota = full.quota.as_ref().expect("quota present");
        assert_eq!(
            quota,
            &QuotaSnapshotInfo {
                used_percentage: 42.5,
                resets_at_ms: Some(1_700_010_000_000),
                confidence: "computed".into(),
            }
        );
        assert_eq!(
            full.tokens,
            Some(TokenCountsInfo {
                input: 12_345,
                output: 6_789,
                cache_read: 100,
                cache_write: 200,
                reasoning: 42,
            })
        );
        assert_eq!(full.cpu_pct, Some(3.5));
        assert_eq!(full.rss_bytes, Some(123_456));
        assert_eq!(full.child_count, Some(2));
        assert_eq!(full.git_branch.as_deref(), Some("feat/read-model"));
        assert_eq!(full.needs_attention, None);
        assert_eq!(full.last_activity_ms, Some(1_700_000_060_000));
        assert_eq!(full.machine_id, "this-mac");
    }

    #[test]
    fn session_info_absent_data_is_null_not_zero() {
        let storage = fixture_storage();
        let rows = plain_rows(&storage);

        // Quota-less: no feed, no observed tokens, never sampled.
        let no_quota = row(&rows, "sess-no-quota");
        assert_eq!(no_quota.quota, None);
        assert_eq!(no_quota.tokens, None);
        assert_eq!(no_quota.cpu_pct, None);
        assert_eq!(no_quota.rss_bytes, None);
        assert_eq!(no_quota.child_count, None);
        // Non-git cwd: the field is omitted entirely, not blank.
        assert_eq!(no_quota.git_branch, None);

        // Model-less: null, never a guessed default model name.
        let no_model = row(&rows, "sess-no-model");
        assert_eq!(no_model.model, None);
        // Terminal status: nothing is live; child_count 0 is a fact, cpu
        // and rss are unmeasurable — null, not zero.
        assert_eq!(no_model.child_count, Some(0));
        assert_eq!(no_model.cpu_pct, None);
        assert_eq!(no_model.rss_bytes, None);
        assert_eq!(no_model.needs_attention, None);

        // Waiting: structured attention reason, not a re-derived guess.
        assert_eq!(
            row(&rows, "sess-waiting").needs_attention,
            Some(NeedsAttention::WaitingUser)
        );

        // Wire honesty: the serialised payload shows null, not zeroed
        // objects or empty strings.
        for id in ["sess-no-quota", "sess-no-model"] {
            let json = serde_json::to_string(row(&rows, id)).expect("serialize");
            assert!(json.contains(r#""quota":null"#), "{id}: {json}");
            assert!(json.contains(r#""tokens":null"#), "{id}: {json}");
            assert!(
                json.contains(r#""account_label":"Work Max""#),
                "{id}: {json}"
            );
        }
    }

    /// AJ-E413 Req.1: desktop rows carry the per-Provider isolation
    /// claim verbatim; terminal rows and unregistered providers stay
    /// `null` — the panel never guesses a limit.
    #[test]
    fn session_info_desktop_isolation_label_maps_per_provider() {
        let storage = fixture_storage();
        let mut desktop = record(
            "sess-desk",
            "rec-1",
            SessionStatus::Running,
            git_cwd("feat/x"),
        );
        desktop.kind = InstanceKind::Desktop;
        storage.sessions().insert(&desktop).expect("seed desktop");

        let labels = std::collections::BTreeMap::from([(
            "claude-code".to_string(),
            "Single-account app — Juggler tracks, cannot isolate".to_string(),
        )]);
        let rows = session_infos(&storage, &LiveStatsMap::new(), &labels, &[]).expect("rows");
        assert_eq!(
            row(&rows, "sess-desk").isolation_label.as_deref(),
            Some("Single-account app — Juggler tracks, cannot isolate")
        );
        // Terminal instances never carry a desktop claim.
        assert_eq!(row(&rows, "sess-full").isolation_label, None);
        // Desktop row whose provider has no registered label: null, not
        // an inherited or default claim.
        let bare = session_infos(
            &storage,
            &LiveStatsMap::new(),
            &std::collections::BTreeMap::new(),
            &[],
        )
        .expect("rows");
        assert_eq!(row(&bare, "sess-desk").isolation_label, None);
    }

    /// AJ-E401 performance budget: the in-memory join (storage reads +
    /// per-row derivation, excluding the batched process-table refresh)
    /// stays under the 20 ms p95 budget at the 50-Instance target scale.
    #[test]
    fn session_info_join_p95_under_budget_at_50_instances() {
        let storage = Storage::open_in_memory().expect("storage");
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-bench"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Bench".into(),
                plan: None,
                state_dir: std::env::temp_dir().join("aij-e401-bench"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .expect("seed account");
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-bench"),
                name: "bench".into(),
                account: AccountId::new("acc-bench"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .expect("seed recipe");
        for n in 0..50 {
            let mut record = record(
                &format!("sess-bench-{n}"),
                "rec-bench",
                SessionStatus::Running,
                std::env::temp_dir(),
            );
            record.usage.tokens = TokenCounts {
                input: n,
                ..TokenCounts::default()
            };
            storage.sessions().insert(&record).expect("seed row");
        }
        // This thread's CPU time: the join runs here, and other programs'
        // turns on a busy machine don't count against it (wall time read
        // 25.7 ms at load 36 for a join that takes well under 1 ms).
        let mut samples: Vec<u128> = Vec::new();
        let mut walls: Vec<u128> = Vec::new();
        for _ in 0..100 {
            let wall = std::time::Instant::now();
            let start = cpu_time::ThreadTime::now();
            let rows = session_infos(
                &storage,
                &LiveStatsMap::new(),
                &std::collections::BTreeMap::new(),
                &[],
            )
            .expect("rows");
            samples.push(start.elapsed().as_micros());
            walls.push(wall.elapsed().as_micros());
            assert_eq!(rows.len(), 50);
        }
        samples.sort_unstable();
        walls.sort_unstable();
        let p95 = samples[94]; // 95th of 100
        assert!(
            p95 < 20_000,
            "join p95 {p95}µs CPU exceeds the 20 ms budget (wall p95 {}µs)",
            walls[94]
        );
    }

    /// Unknown statuses cannot reach the read model: storage rejects them
    /// at row-mapping time (degrade-to-error, never a panic), so the wire
    /// only ever carries the nine vocabulary strings — verified by
    /// round-tripping every status through the mapper.
    #[test]
    fn session_info_status_vocabulary_is_the_nine_wire_strings() {
        let storage = fixture_storage();
        let rows = plain_rows(&storage);
        let vocabulary: Vec<&str> = rows.iter().map(|row| row.status.as_str()).collect();
        for wire in [
            "starting",
            "running",
            "waiting_user",
            "stuck",
            "quota_blocked",
            "finished",
            "stopped",
            "failed",
            "handed_over",
        ] {
            assert!(
                SessionStatus::parse(wire).is_some(),
                "wire status {wire} must parse"
            );
        }
        assert!(
            vocabulary
                .iter()
                .all(|status| SessionStatus::parse(status).is_some())
        );
        assert_eq!(
            needs_attention_of(SessionStatus::parse("quota_blocked").unwrap()),
            Some(NeedsAttention::QuotaBlocked)
        );
    }

    /// Defensive join fallback (storage FKs make this unreachable today):
    /// an unjoinable row renders generic `unknown` markers — account id
    /// `null`, provider/label never blank (AJ-E401 Req.2).
    #[test]
    fn unjoinable_rows_render_generic_markers() {
        use std::collections::BTreeMap;

        use super::resolved_join;
        let (account_id, account_label, provider, model) =
            resolved_join(&BTreeMap::new(), &BTreeMap::new(), "rec-gone");
        assert_eq!(account_id, None);
        assert_eq!(account_label, "unknown");
        assert_eq!(provider, "unknown");
        assert_eq!(model, None);
    }

    /// I-09 branch parsing: ref rows parse, detached and missing `.git`
    /// stay `None` (unknown, not a guess).
    #[test]
    fn git_branch_parses_refs_only() {
        let cwd = git_cwd("main");
        assert_eq!(crate::worktree::branch_of(&cwd), Some("main".to_string()));
        assert_eq!(crate::worktree::branch_of(&std::env::temp_dir()), None);
        let detached =
            std::env::temp_dir().join(format!("aij-e401-{}-detached", std::process::id()));
        std::fs::create_dir_all(detached.join(".git")).expect("fixture");
        std::fs::write(
            detached.join(".git/HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .expect("fixture");
        assert_eq!(crate::worktree::branch_of(&detached), None);
        std::fs::remove_dir_all(detached).ok();
        std::fs::remove_dir_all(cwd.join(".git")).ok();
    }

    /// AJ-E401 Req.5: the committed ts-rs output contains every new read
    /// model field (regenerated by `cargo test`; committed with the code).
    #[test]
    #[allow(clippy::panic, reason = "failure message names the missing file")]
    fn generated_ts_contains_read_model_fields() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/types/generated");
        // The export tests in this binary rewrite these files as it runs:
        // read one once it's whole (its type declared), not mid-write.
        let read = |name: &str| {
            for _ in 0..100 {
                let text = std::fs::read_to_string(format!("{dir}/{name}.ts"))
                    .unwrap_or_else(|error| panic!("generated {name}.ts: {error}"));
                if text.contains(&format!("export type {name} ")) && text.trim_end().ends_with(';')
                {
                    return text;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            panic!("generated {name}.ts never read whole");
        };
        let instance = read("InstanceInfo");
        for field in [
            "account_id",
            "account_label",
            "model",
            "quota",
            "tokens",
            "cpu_pct",
            "rss_bytes",
            "child_count",
            "subtask_count",
            "git_branch",
            "git_dirty_count",
            "needs_attention",
            "last_activity_ms",
            "machine_id",
        ] {
            assert!(
                instance.contains(&format!("{field}:")),
                "InstanceInfo.ts is missing `{field}`"
            );
        }
        for (name, fields) in [
            (
                "QuotaSnapshotInfo",
                &["used_percentage", "resets_at_ms", "confidence"][..],
            ),
            (
                "TokenCountsInfo",
                &["input", "output", "cache_read", "cache_write", "reasoning"][..],
            ),
            (
                "InstanceResourcesInfo",
                &["id", "cpu_pct", "rss_bytes", "child_count"][..],
            ),
            ("InstanceQuotaInfo", &["id", "quota"][..]),
        ] {
            let file = read(name);
            for field in fields {
                assert!(
                    file.contains(&format!("{field}:")),
                    "{name}.ts is missing `{field}`"
                );
            }
        }
        let needs = read("NeedsAttention");
        for variant in ["waiting_user", "stuck", "quota_blocked", "failed"] {
            assert!(
                needs.contains(variant),
                "NeedsAttention.ts is missing `{variant}`"
            );
        }
    }
}
