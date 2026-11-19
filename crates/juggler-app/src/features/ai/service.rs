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
    /// one given. `beta` reads a job's Beta switch, `picks` the user's own
    /// model per job; `data_dir` is where AI files would live (the
    /// recommendation's free disk).
    pub fn answer(
        &self,
        snapshot: &Snapshot,
        beta: &dyn Fn(Job) -> bool,
        picks: &BTreeMap<Job, String>,
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
            picks,
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

/// The models a job may run on: the `offered` ones, then each
/// `installed` copy the list no longer has (it keeps working from its
/// copy, ADR-0035 §11.4).
#[must_use]
pub fn usable(offered: &[ModelEntry], installed: &[ModelEntry]) -> Vec<ModelEntry> {
    let mut usable = offered.to_vec();
    for model in installed {
        if !usable.iter().any(|known| known.id == model.id) {
            usable.push(model.clone());
        }
    }
    usable
}

/// The recommendation for `mac` among the `usable` models that aren't
/// withdrawn (AJ-E3811): Automatic's model, and the Models screen's
/// Recommended. One function, so the two never disagree.
#[must_use]
pub fn recommendation(
    mac: &MacFacts,
    usable: &[ModelEntry],
    installed: &[ModelEntry],
) -> Recommendation {
    let installed_ids: Vec<&str> = installed.iter().map(|m| m.id.as_str()).collect();
    let listed: Vec<ModelEntry> = usable.iter().filter(|m| !m.withdrawn).cloned().collect();
    recommend(mac, &listed, &installed_ids)
}

/// The answer over `offered` models and the `installed` ones (pure but
/// for the engines' last probe): the router per job over both (an
/// installed model keeps working from its copy, listed or not, ADR-0035
/// §11.4), the recommendation among the offered ones as Automatic, and
/// the user's own `picks` first.
#[must_use]
pub fn answer_for(
    engines: &Registry,
    offered: &[ModelEntry],
    installed: &[ModelEntry],
    mac: Option<&MacFacts>,
    snapshot: &Snapshot,
    beta: &dyn Fn(Job) -> bool,
    picks: &BTreeMap<Job, String>,
) -> Vec<AiJobInfo> {
    let usable = usable(offered, installed);
    let offered = usable.as_slice();
    let cards = engines.cards(offered);
    let automatic = mac.and_then(|mac| match recommendation(mac, offered, installed) {
        Recommendation::Model { id, .. } => Some(id),
        Recommendation::Nothing(_) => None,
    });
    let inputs = Inputs {
        cards: &cards,
        models: offered,
        automatic: automatic.as_deref(),
        picks,
    };
    let switches = |spec: &JobSpec| Switches {
        on: flags::on_in(&snapshot.effective, spec.flag),
        beta: beta(spec.job),
    };
    router::routes(&jobs::JOBS, switches, &inputs)
        .into_iter()
        .filter_map(|(spec, route)| {
            let job = AiJob::of(spec.job)?;
            let info = AiJobInfo::new(
                job,
                route,
                beta(spec.job),
                |card| cards.get(card).map(|card| card.model.clone()),
                |id| offered.iter().find(|m| m.id == id).map(|m| m.name.clone()),
            );
            Some(AiJobInfo {
                pick: picks.get(&spec.job).cloned(),
                ..info
            })
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

/// The user's own model per job, as stored (empty is Automatic).
fn picks(state: &AppState) -> AppResult<BTreeMap<Job, String>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let settings = storage.app_settings();
    let mut picks = BTreeMap::new();
    for spec in jobs::JOBS {
        if let Some(id) = settings.get(&jobs::pick_key(spec.job))?
            && !id.is_empty()
        {
            picks.insert(spec.job, id);
        }
    }
    Ok(picks)
}

fn answer_now(state: &AppState, probe: Probe) -> AppResult<(Vec<AiJobInfo>, bool)> {
    let switches = beta_switches(state)?;
    let picks = picks(state)?;
    let beta = |job: Job| switches.get(&job).copied().unwrap_or(false);
    let snapshot = crate::features::remote_config::current();
    Ok(service().answer(&snapshot, &beta, &picks, &state.data_dir, probe))
}

/// This Mac's facts as the recommendation reads them, probing the
/// engines first when the last probe is stale (the Models screen).
#[must_use]
pub fn mac_now(state: &AppState) -> Option<MacFacts> {
    service().mac_facts(&state.data_dir, Probe::IfStale)
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

/// Whether `model_id` may be picked for `job`: a model the app offers or
/// one on this Mac, whose verdict for the job passed (D-127: the picker
/// offers nothing else, and the command takes nothing else).
#[must_use]
pub fn may_pick(usable: &[ModelEntry], job: Job, model_id: &str) -> bool {
    usable
        .iter()
        .any(|model| model.id == model_id && model.verdict(job).passed())
}

/// Sets the user's own model for `job` (`None`: back to Automatic), then
/// answers again (AJ-E3812's per-job override).
///
/// # Errors
///
/// A model that may not serve the job, or settings that can't be written.
pub fn set_pick(
    state: &AppState,
    job: AiJob,
    model_id: Option<&str>,
) -> AppResult<(Vec<AiJobInfo>, bool)> {
    if let Some(id) = model_id {
        let snapshot = crate::features::remote_config::current();
        let listing = models::listing(&snapshot.lists);
        let installed = models::installed_entries(&models::store::shared_installed(), &listing);
        if !may_pick(
            &usable(&models::offered(&listing), &installed),
            job.job(),
            id,
        ) {
            return Err(AppError::Validation {
                subject: "model",
                message: "that model can't do this job".to_owned(),
            });
        }
    }
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .app_settings()
            .set(&jobs::pick_key(job.job()), model_id.unwrap_or(""))?;
    }
    answer_now(state, Probe::Never)
}

/// After `model_id` was removed: every job picked for it goes back to
/// Automatic, as the Remove confirmation says.
///
/// # Errors
///
/// Settings that can't be read or written.
pub fn clear_picks_of(state: &AppState, model_id: &str) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let settings = storage.app_settings();
    for spec in jobs::JOBS {
        let key = jobs::pick_key(spec.job);
        if settings.get(&key)?.as_deref() == Some(model_id) {
            settings.set(&key, "")?;
        }
    }
    Ok(())
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

/// One generation for `job` on the card the router picks now, with the
/// model's display name (ADR-0034 §3: one engine, never a chain). The
/// job's flag, its Beta switch and the user's pick hold as they do for
/// the query: a job that isn't `Run` doesn't run.
///
/// # Errors
///
/// [`EngineError::NotHere`] when the job doesn't run now; the engine's
/// own error otherwise.
pub fn respond_for(
    state: &AppState,
    job: Job,
    request: &super::engine::Request,
    stop: &super::engine::Stop,
) -> Result<(super::engine::Draft, String), super::engine::EngineError> {
    use super::engine::{EngineError, Reason};
    let not_here = EngineError::NotHere(Reason::Unknown);
    let switches = beta_switches(state).map_err(|_| not_here)?;
    let picks = picks(state).map_err(|_| not_here)?;
    let snapshot = crate::features::remote_config::current();
    let service = service();
    let mac = service.mac_facts(&state.data_dir, Probe::Never);
    let listing = models::listing(&snapshot.lists);
    let offered = models::offered(&listing);
    let installed = models::installed_entries(&models::store::shared_installed(), &listing);
    let usable = usable(&offered, &installed);
    let cards = service.engines.cards(&usable);
    let automatic = mac
        .as_ref()
        .and_then(|mac| match recommendation(mac, &usable, &installed) {
            Recommendation::Model { id, .. } => Some(id),
            Recommendation::Nothing(_) => None,
        });
    let inputs = Inputs {
        cards: &cards,
        models: &usable,
        automatic: automatic.as_deref(),
        picks: &picks,
    };
    let spec = jobs::JOBS
        .iter()
        .copied()
        .find(|spec| spec.job == job)
        .ok_or(not_here)?;
    let on = Switches {
        on: flags::on_in(&snapshot.effective, spec.flag),
        beta: switches.get(&job).copied().unwrap_or(false),
    };
    let router::Route::Run { card, .. } = router::route(spec, on, &inputs) else {
        return Err(not_here);
    };
    let card = cards.get(card).ok_or(not_here)?;
    let draft = service.engines.respond(card, request, stop)?;
    let name = usable
        .iter()
        .find(|m| m.id == card.model)
        .map_or_else(|| card.model.clone(), |m| m.name.clone());
    Ok((draft, name))
}
