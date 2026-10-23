//! AJ-E1409 — This Mac's advice from this Mac's own history: what a launch
//! setting's sessions usually take, against the room left now. Nothing
//! until a setting has at least three measured sessions. Split from
//! `machine_health` (the size budget: a feature to a module).

use std::sync::Arc;

use juggler_supervisor::machine::bytes_words;

use crate::machine_health::{MIN_FOOTPRINTS, MachineHealthInfo, ThisMacInfo};
use crate::state::AppState;

/// The storage half of the recommendations, cached between 5 s reads.
#[derive(Debug, Clone)]
pub(crate) struct AdviceInputs {
    footprints_written: u64,
    read_ms: i64,
    stats: Vec<juggler_core::storage::FootprintStats>,
    names: std::collections::HashMap<String, String>,
}

/// Launch-setting names may change without a footprint write; the cached
/// advice inputs are re-read at least this often.
const ADVICE_INPUTS_MAX_AGE_MS: i64 = 60_000;

/// Pure (AJ-E1409): advice from this Mac's own footprints. `names` maps a
/// launch setting id to how the user knows it. Nothing until a setting has
/// at least three measured sessions.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn recommendations<S: std::hash::BuildHasher>(
    machine: &MachineHealthInfo,
    ours_rss: u64,
    footprints: &[juggler_core::storage::FootprintStats],
    names: &std::collections::HashMap<String, String, S>,
) -> Vec<String> {
    // Only launch settings that still exist (a deleted one's footprints
    // are pruned within 10 minutes; until then they don't count).
    let measured: Vec<&juggler_core::storage::FootprintStats> = footprints
        .iter()
        .filter(|stats| {
            stats
                .key
                .strip_prefix("recipe:")
                .is_some_and(|id| names.contains_key(id))
                && stats.count >= MIN_FOOTPRINTS
        })
        .collect();
    if measured.is_empty() || !machine.sampled || machine.memory_total == 0 {
        return Vec::new();
    }
    let sessions: u32 = measured.iter().map(|stats| stats.count).sum();
    let typical = measured
        .iter()
        .map(|stats| stats.median_peak_rss as f64 * f64::from(stats.count))
        .sum::<f64>()
        / f64::from(sessions);
    let mut out = Vec::new();
    // Keep a fifth of memory free; what isn't AI Juggler's sessions stays
    // as it is now. Only our trees' memory is measured — a tool AI Juggler
    // didn't start is an estimate that counts shared memory more than once
    // (AJ-E1402), so it stays in "the rest" rather than inflate the room.
    // Used as the reading measured it; total − available only when it
    // couldn't (the OS's available figure is then the best there is).
    let used = machine.memory_used.unwrap_or_else(|| {
        machine
            .memory_total
            .saturating_sub(machine.memory_available)
    });
    let others = used.saturating_sub(ours_rss) as f64;
    let room = machine.memory_total as f64 * 0.8 - others;
    if typical > 0.0 {
        let fits = (room / typical).floor().max(0.0) as u64;
        out.push(if fits == 0 {
            format!(
                "With what else is open, this Mac is short of room for even one session (about {} each, from your last {sessions})",
                bytes_words(typical as u64)
            )
        } else {
            format!(
                "This Mac comfortably runs about {fits} session{} at once (about {} each, from your last {sessions})",
                if fits == 1 { "" } else { "s" },
                bytes_words(typical as u64)
            )
        });
    }
    if measured.len() >= 2 {
        let mut by_weight = measured.clone();
        by_weight.sort_by_key(|stats| std::cmp::Reverse(stats.median_peak_rss));
        let heaviest = by_weight[0];
        let next = by_weight[1];
        if heaviest.median_peak_rss as f64 >= next.median_peak_rss as f64 * 1.5 {
            let id = heaviest.key.trim_start_matches("recipe:");
            let name = names
                .get(id)
                .cloned()
                .unwrap_or_else(|| "One launch setting".into());
            out.push(format!(
                "{name} is the heaviest launch setting — about {} a session",
                bytes_words(heaviest.median_peak_rss)
            ));
        }
    }
    out
}

pub(crate) fn recommendations_for(
    state: &Arc<AppState>,
    info: &ThisMacInfo,
    now_ms: i64,
) -> Vec<String> {
    let (cached, written) = match state.machine.lock() {
        Ok(watch) => (watch.advice_inputs.clone(), watch.footprints_written),
        Err(_) => return Vec::new(),
    };
    let inputs = match cached {
        Some(inputs)
            if inputs.footprints_written == written
                && now_ms - inputs.read_ms < ADVICE_INPUTS_MAX_AGE_MS =>
        {
            inputs
        }
        _ => {
            let Some(inputs) = read_advice_inputs(state, written, now_ms) else {
                return Vec::new();
            };
            if let Ok(mut watch) = state.machine.lock() {
                watch.advice_inputs = Some(inputs.clone());
            }
            inputs
        }
    };
    recommendations(
        &info.machine,
        info.ours.rss_bytes,
        &inputs.stats,
        &inputs.names,
    )
}

/// The footprint medians and launch-setting names, from storage.
fn read_advice_inputs(
    state: &Arc<AppState>,
    footprints_written: u64,
    now_ms: i64,
) -> Option<AdviceInputs> {
    let storage = state.storage.lock().ok()?;
    let medians = storage.footprints().stats().ok()?;
    let mut names = std::collections::HashMap::new();
    if !medians.is_empty() {
        for account in storage.accounts().list().unwrap_or_default() {
            for recipe in storage
                .profiles()
                .list_by_account(&account.id)
                .unwrap_or_default()
            {
                names.insert(
                    recipe.id.to_string(),
                    format!("\u{201c}{}\u{201d} ({})", recipe.name, account.label),
                );
            }
        }
    }
    Some(AdviceInputs {
        footprints_written,
        read_ms: now_ms,
        stats: medians,
        names,
    })
}
