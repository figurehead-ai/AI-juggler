//! AJ-E209: the badged switch entry point: each target's verdict and rank.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_core::model::Timestamp;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::handover::source::handover_source;
use crate::features::quota::quota_display_inner;
use crate::features::ranking::dto::RankedRowInfo;
use crate::features::ranking::{SwitchRankContext, quota_tte, rank_candidates_for};
use crate::features::switch::dto::{HeldBack, SwitchTargetInfo};
use crate::features::switch::preflight::carry_lists_of;
use crate::features::validation::{beta_switches_on, dogfood_cells_of, dogfood_key};
use crate::state::AppState;

/// The §Q6 switch context for one source: its TTE (return-bonus
/// input), the returnable originals (accounts an OPEN excursion leads
/// back to), and the source's measured replay floor (its transcript
/// usage).
fn switch_rank_context_of(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    source_account: &juggler_core::model::Account,
) -> AppResult<SwitchRankContext> {
    let storage = lock_storage_arc(state)?;
    let now = Timestamp::now().millis();
    let source_tte_min = quota_tte(&quota_display_inner(&storage, source_account, now));
    let mut returnable = std::collections::BTreeSet::new();
    let graph = storage.switch_graph();
    for project in storage.projects().list()? {
        for work in graph.works_for_project(&project.id)? {
            let edges = graph.edges_for_work(&work.id)?;
            for edge in edges.iter().filter(|edge| {
                matches!(edge.state, juggler_core::model::EdgeState::Confirmed)
                    && edge.parent_edge_a.is_none()
            }) {
                let Some(to_epoch) = &edge.to_epoch else {
                    continue;
                };
                let returned = edges.iter().any(|other| {
                    matches!(other.state, juggler_core::model::EdgeState::Confirmed)
                        && &other.from_epoch == to_epoch
                });
                if !returned {
                    returnable.insert(edge.from_account.to_string());
                }
            }
        }
    }
    // What the usage pipeline already counted for it; the transcript is
    // read (once per size and time) only when it counted nothing (0.1.5
    // review: a session's page read its whole transcript twice on open).
    let counted = source
        .usage
        .tokens
        .input
        .saturating_add(source.usage.tokens.output);
    let source_replay_tokens = if counted > 0 {
        Some(counted)
    } else {
        source
            .transcript_ref
            .as_ref()
            .and_then(|reference| transcript_tokens_once(&reference.path))
    };
    Ok(SwitchRankContext {
        source_provider: source_account.harness.to_string(),
        source_side: crate::features::service_accounts::switch_side_in(&storage, source_account),
        source_tte_min,
        returnable_accounts: returnable,
        source_replay_tokens,
    })
}

/// A transcript's input + output tokens, read again only when its size or
/// time changed (the last few kept).
fn transcript_tokens_once(path: &std::path::Path) -> Option<u64> {
    type Seen =
        std::collections::HashMap<PathBuf, (u64, Option<std::time::SystemTime>, Option<u64>)>;
    static SEEN: std::sync::LazyLock<std::sync::Mutex<Seen>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(Seen::new()));
    let meta = std::fs::metadata(path).ok()?;
    let stamp = (meta.len(), meta.modified().ok());
    if let Ok(seen) = SEEN.lock()
        && let Some((len, modified, tokens)) = seen.get(path)
        && (*len, *modified) == stamp
    {
        return *tokens;
    }
    let tokens = juggler_adapters::transcript_tokens(path)
        .map(|tokens| tokens.input.saturating_add(tokens.output));
    if let Ok(mut seen) = SEEN.lock() {
        if seen.len() >= 64 {
            seen.clear();
        }
        seen.insert(path.to_path_buf(), (stamp.0, stamp.1, tokens));
    }
    tokens
}

/// A row's words, leaving out its account's room when the target isn't
/// signed in (0.4.1, the rc6 walk: "Not signed in" beside "74% left").
fn unless_signed_out<T: Clone>(
    rows: &[T],
    signed_out: bool,
    words: impl Fn(&T) -> &String,
) -> Vec<T> {
    rows.iter()
        .filter(|row| !(signed_out && juggler_core::ranking::is_room_words(words(row))))
        .cloned()
        .collect()
}

/// 0.4.1: the ranked accounts a desktop app profile signed in on them
/// takes a switch in (Desktop), each with those profiles' place ids (the
/// one seen last first), read before storage is held. AJ-E2317: each is a
/// row of its own, so the App picked is the one a move goes into.
fn desktop_accounts(
    state: &Arc<AppState>,
    ranked: &[RankedRowInfo],
) -> std::collections::HashMap<String, Vec<String>> {
    ranked
        .iter()
        .filter_map(|row| {
            let mut places = crate::features::places::places_on(state, &row.account, |_, host| {
                host.handoff_link.is_some() && host.handoff_record.is_some()
            })?;
            places.sort_by_key(|(id, place)| (std::cmp::Reverse(place.seen_ms), id.clone()));
            let ids: Vec<String> = places.into_iter().map(|(id, _)| id).collect();
            (!ids.is_empty()).then(|| (row.account.clone(), ids))
        })
        .collect()
}

/// The desktop Apps (place ids) of `account` that take a switch of this
/// tool's (none across tools).
fn apps_of<'a>(
    desktops: &'a std::collections::HashMap<String, Vec<String>>,
    account: &str,
    cross: bool,
) -> &'a [String] {
    desktops
        .get(account)
        .filter(|_| !cross)
        .map_or(&[][..], Vec::as_slice)
}

/// AJ-E2317: an account's rows, one per desktop App (`apps`), else one.
fn rows_of(apps: &[String]) -> Vec<Option<String>> {
    if apps.is_empty() {
        vec![None]
    } else {
        apps.iter().cloned().map(Some).collect()
    }
}

/// F94 (the AJ-E2317 class): whether a row is where the conversation is
/// already: the desktop App it is in (`source_app`, by its id, whatever
/// launch settings list it), or its own launch settings in a terminal
/// (`own`; that is Continue in AI Juggler, not a switch). Another desktop
/// App on its own folder that takes sessions is a row: a terminal
/// conversation continues there, a desktop App's moves to its twin.
fn is_source(place_id: Option<&str>, own: bool, source_app: &str) -> bool {
    match place_id {
        Some(place) => place == source_app,
        None => own,
    }
}

/// Why a target is greyed: the source has no turn yet (the rc29 walk:
/// refused at the door, never at the switch's last step), the ranking's
/// reason, or the target's own launch check (a first run the switch
/// finishes never holds it back).
fn door_reason(
    not_yet: Option<&String>,
    row: &RankedRowInfo,
    blocker: Option<&juggler_adapter::readiness::NotReady>,
    first_run: bool,
) -> Option<String> {
    not_yet
        .cloned()
        .or_else(|| row.filtered_reason.clone())
        .or_else(|| {
            blocker
                .filter(|_| !first_run)
                .map(crate::features::handover::access::blocker_words)
        })
}

/// AJ-E210: one pair's badge and validated state, from the validation
/// matrix merged with any recorded live cell (a passing cell validates only
/// real-CI rows — D-04). A profile through another service is its own side
/// (AJ-E1601).
fn matrix_row_of(
    storage: &juggler_core::storage::Storage,
    dogfood: &std::collections::BTreeMap<String, juggler_handover::validation::DogfoodCell>,
    source_side: &str,
    account: &juggler_core::model::Account,
) -> Option<juggler_handover::validation::PairCell> {
    let side = crate::features::service_accounts::switch_side_in(storage, account);
    let cell = dogfood.get(&dogfood_key(source_side, &side));
    juggler_handover::validation::pair_cell(source_side, &side).map(|row| match cell {
        Some(record) => juggler_handover::validation::with_dogfood(row, record.clone()),
        None => row,
    })
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn switch_targets_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<Vec<SwitchTargetInfo>> {
    let (source, source_account) = handover_source(state, session_id)?;
    // F94: the App it is in, by its id (a desktop App's Code tab
    // conversation is that desktop App's), read before storage is held.
    let from = super::source_app::source_app_of(state, session_id, &source, &source_account);
    let not_yet = super::execute::not_yet_words(state, &source, &source_account);
    let context = switch_rank_context_of(state, &source, &source_account)?;
    let ranked = rank_candidates_for(
        state,
        source.project.as_ref().map(ToString::to_string).as_deref(),
        Some(&context),
    )?;
    let by_account: std::collections::BTreeMap<String, &RankedRowInfo> = ranked
        .iter()
        .map(|row| (row.account.clone(), row))
        .collect();
    let desktops = desktop_accounts(state, &ranked);
    let storage = lock_storage_arc(state)?;
    let dogfood = dogfood_cells_of(&storage);
    // D-127: a switch not decided live in every part isn't offered unless
    // the user turned on beta switches.
    let beta = beta_switches_on(&storage);
    let mut out = Vec::new();
    // In the ranking's order (the 0.2 walk, F1: they came in the order the
    // accounts were added).
    let mut accounts = storage.accounts().list()?;
    let place: std::collections::BTreeMap<&str, usize> = ranked
        .iter()
        .enumerate()
        .map(|(index, row)| (row.account.as_str(), index))
        .collect();
    accounts.sort_by_key(|account| {
        place
            .get(account.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    for account in accounts {
        let Some(row) = by_account.get(account.id.as_str()) else {
            continue;
        };
        // 0.3.4: the switch's own launch check, before it's offered.
        let blocker = crate::features::handover::access::launch_blocker(state, &account);
        let set_up = matches!(
            blocker,
            Some(juggler_adapter::readiness::NotReady::NotSetUp(_))
        );
        // 0.3.5: a first run the switch finishes never holds a target back.
        let first_run = blocker == Some(juggler_adapter::readiness::NotReady::FirstRun);
        let signed_out = !super::execute::terminal_signed_in(state, &account);
        let reason = door_reason(not_yet.as_ref(), row, blocker.as_ref(), first_run);
        for recipe in storage.profiles().list_by_account(&account.id)? {
            let own = recipe.id == source.profile;
            let cross = account.harness.as_str() != context.source_provider;
            // A signed-in desktop app profile takes it: the account's room
            // is its own then, whatever the terminal says.
            let apps = apps_of(&desktops, account.id.as_str(), cross);
            let no_room = signed_out && apps.is_empty();
            let carry = carry_lists_of(&juggler_handover::fidelity::base_dispositions(
                context.source_provider.as_str(),
                account.harness.as_str(),
            ));
            let matrix_row = matrix_row_of(&storage, &dogfood, &context.source_side, &account);
            let validated = matrix_row
                .as_ref()
                .is_some_and(juggler_handover::validation::PairCell::validated);
            if !validated && !beta {
                continue;
            }
            // AJ-E2317: one row per desktop App on the account (else one).
            // F94: each App a row by its own id; the source's App never.
            let rows = rows_of(apps)
                .into_iter()
                .filter(|place_id| !is_source(place_id.as_deref(), own, &from.id));
            out.extend(rows.map(|place_id| {
                SwitchTargetInfo {
                    app_id: place_id
                        .clone()
                        .unwrap_or_else(|| juggler_core::storage::cli_app_id(account.id.as_str())),
                    recipe_id: recipe.id.to_string(),
                    provider: account.harness.to_string(),
                    account_id: account.id.to_string(),
                    account_label: account.label.clone(),
                    badge: matrix_row.as_ref().map_or_else(
                        || "Handover notes · Beta".to_owned(),
                        juggler_handover::validation::PairCell::badge,
                    ),
                    validated,
                    cross_provider: cross,
                    drops: carry.dropped.clone(),
                    score: row.score,
                    filtered_reason: reason.clone(),
                    not_yet: not_yet.clone(),
                    held_back: (row.filtered_reason.is_none() && set_up).then_some(HeldBack::SetUp),
                    first_run,
                    signed_out,
                    desktop: !apps.is_empty(),
                    place_id,
                    top_factors: unless_signed_out(&row.top_factors, no_room, |words| words),
                    factors: unless_signed_out(&row.factors, no_room, |(words, _)| words),
                    confidence: row.confidence.clone(),
                    tte_min: row.tte_min,
                    // Its own account gives no new room: never the suggestion.
                    suggested: row.suggested && reason.is_none() && account.id != source_account.id,
                }
            }));
        }
    }
    Ok(out)
}
