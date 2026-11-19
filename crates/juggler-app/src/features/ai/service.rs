//! ADR-0034 §3.3–3.4 (AJ-E3802): the one query, "which AI jobs are
//! available now", per job with the model chosen and why. Every AI
//! surface shows or hides itself through it, never by a check of its own.
//!
//! **What it combines, per job:** the job's flag (compiled default and the
//! remote kill, D-130), the user's Beta switch for it, the `models`
//! section's verdicts (ADR-0035 §11; the built-in list when the section
//! isn't read), every engine's card, and the recommendation for this Mac
//! as the Automatic default (AJ-E3811).
//!
//! **Lazy, and no timer.** Nothing exists until the first query: then the
//! engines are made and probed (the Apple helper says hello and exits; no
//! model is loaded). A later query probes again only when the last probe
//! is older than [`FRESH_FOR`]; the window asks when it is shown, so
//! turning Apple Intelligence on or off in System Settings is seen when
//! the user comes back. A change to the configuration or a Beta switch
//! re-answers without a probe. Whenever the answer changes, the window is
//! told (`AppEvent::AiJobsChanged`).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::dto::{AiJob, AiJobInfo};
use super::engines::Registry;
use super::jobs::{self, JobSpec};
use super::models::recommend::{MacFacts, Recommendation, recommend};
use super::models::{self, Job, ModelEntry};
use super::router::{self, Inputs, Switches};
use crate::error::{AppError, AppResult};
use crate::features::flags;
use crate::features::remote_config::Snapshot;
use crate::state::AppState;

/// How long a probe's answer stands before a query asks the engines again.
pub const FRESH_FOR: Duration = Duration::from_secs(30);

/// The engines and what the last probe and answer were.
#[derive(Debug)]
pub struct Service {
    engines: Registry,
    probed: Mutex<Option<(Instant, MacFacts)>>,
    last: Mutex<Option<Vec<AiJobInfo>>>,
}

/// Whether an answer may ask the engines again first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// When the last probe is older than [`FRESH_FOR`] (a query).
    IfStale,
    /// Now (a test, or after the user said something changed).
    Now,
    /// Never: re-answer from what is known (a switch or the
    /// configuration changed).
    Never,
}

impl Service {
    /// A service over `engines`, nothing probed.
    #[must_use]
    pub const fn new(engines: Registry) -> Self {
        Self {
            engines,
            probed: Mutex::new(None),
            last: Mutex::new(None),
        }
    }

    /// The engines.
    #[must_use]
    pub const fn engines(&self) -> &Registry {
        &self.engines
    }

    /// Every routed job's answer, and whether it differs from the last
    /// one given. `beta` reads a job's Beta switch; `data_dir` is where AI
    /// files would live (the recommendation's free disk).
    pub fn answer(
        &self,
        snapshot: &Snapshot,
        beta: &dyn Fn(Job) -> bool,
        data_dir: &Path,
        probe: Probe,
    ) -> (Vec<AiJobInfo>, bool) {
        let mac = self.mac_facts(data_dir, probe);
        let listing = models::listing(&snapshot.lists);
        let offered = models::offered(&listing);
        let installed = models::installed_entries(&models::store::shared_installed(), &listing);
        let jobs = answer_for(
            &self.engines,
            &offered,
            &installed,
            mac.as_ref(),
            snapshot,
            beta,
        );
        let changed = self.last.lock().is_ok_and(|mut last| {
            let changed = last.as_ref() != Some(&jobs);
            *last = Some(jobs.clone());
            changed
        });
        (jobs, changed)
    }

    /// This Mac's facts, probing first when `probe` says so.
    fn mac_facts(&self, data_dir: &Path, probe: Probe) -> Option<MacFacts> {
        let mut probed = self.probed.lock().ok()?;
        let stale = probed
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= FRESH_FOR);
        if probe == Probe::Now || (probe == Probe::IfStale && stale) {
            self.engines.probe();
            *probed = Some((Instant::now(), self.engines.mac_facts(data_dir)));
        }
        probed.as_ref().map(|(_, mac)| *mac)
    }
}

/// The answer over `offered` models and the `installed` ones (pure but
/// for the engines' last probe): the router per job over both (an
/// installed model keeps working from its copy, listed or not, ADR-0035
/// §11.4), the recommendation among the offered ones as Automatic.
#[must_use]
pub fn answer_for(
    engines: &Registry,
    offered: &[ModelEntry],
    installed: &[ModelEntry],
    mac: Option<&MacFacts>,
    snapshot: &Snapshot,
    beta: &dyn Fn(Job) -> bool,
) -> Vec<AiJobInfo> {
    let installed_ids: Vec<&str> = installed.iter().map(|m| m.id.as_str()).collect();
    let mut usable = offered.to_vec();
    for model in installed {
        if !usable.iter().any(|known| known.id == model.id) {
            usable.push(model.clone());
        }
    }
    let offered = usable.as_slice();
    let cards = engines.cards(offered);
    let automatic = mac.and_then(|mac| {
        let listed: Vec<ModelEntry> = offered.iter().filter(|m| !m.withdrawn).cloned().collect();
        match recommend(mac, &listed, &installed_ids) {
            Recommendation::Model { id, .. } => Some(id),
            Recommendation::Nothing(_) => None,
        }
    });
    let picks = BTreeMap::new();
    let inputs = Inputs {
        cards: &cards,
        models: offered,
        automatic: automatic.as_deref(),
        picks: &picks,
    };
    let switches = |spec: &JobSpec| Switches {
        on: flags::on_in(&snapshot.effective, spec.flag),
        beta: beta(spec.job),
    };
    router::routes(&jobs::JOBS, switches, &inputs)
        .into_iter()
        .filter_map(|(spec, route)| {
            let job = AiJob::of(spec.job)?;
            Some(AiJobInfo::new(
                job,
                route,
                beta(spec.job),
                |card| cards.get(card).map(|card| card.model.clone()),
                |id| offered.iter().find(|m| m.id == id).map(|m| m.name.clone()),
            ))
        })
        .collect()
}

static SERVICE: OnceLock<Service> = OnceLock::new();

/// The app's service, made at the first query.
fn service() -> &'static Service {
    SERVICE.get_or_init(|| Service::new(Registry::new()))
}

/// A job's Beta switch as stored (`1` is on).
fn beta_switches(state: &AppState) -> AppResult<BTreeMap<Job, bool>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let settings = storage.app_settings();
    jobs::JOBS
        .iter()
        .map(|spec| {
            let on = settings.get(&jobs::beta_key(spec.job))?.as_deref() == Some("1");
            Ok((spec.job, on))
        })
        .collect()
}

fn answer_now(state: &AppState, probe: Probe) -> AppResult<(Vec<AiJobInfo>, bool)> {
    let switches = beta_switches(state)?;
    let beta = |job: Job| switches.get(&job).copied().unwrap_or(false);
    let snapshot = crate::features::remote_config::current();
    Ok(service().answer(&snapshot, &beta, &state.data_dir, probe))
}

/// The query: every routed job's answer, probing the engines when the
/// last probe is stale; and whether the answer changed.
///
/// # Errors
///
/// The Beta switches can't be read.
pub fn jobs_now(state: &AppState) -> AppResult<(Vec<AiJobInfo>, bool)> {
    answer_now(state, Probe::IfStale)
}

/// Turns `job`'s Beta switch on or off, then answers again.
///
/// # Errors
///
/// Settings that can't be written or read.
pub fn set_beta(state: &AppState, job: AiJob, on: bool) -> AppResult<(Vec<AiJobInfo>, bool)> {
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .app_settings()
            .set(&jobs::beta_key(job.job()), if on { "1" } else { "0" })?;
    }
    answer_now(state, Probe::Never)
}

/// After the configuration changed: the new answer when it differs from
/// the last one given. Nothing when no one has asked yet (lazy).
#[must_use]
pub fn recheck(state: &AppState) -> Option<Vec<AiJobInfo>> {
    SERVICE.get()?;
    match answer_now(state, Probe::Never) {
        Ok((jobs, true)) => Some(jobs),
        _ => None,
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
