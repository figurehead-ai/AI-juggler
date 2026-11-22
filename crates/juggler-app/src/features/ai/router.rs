//! ADR-0034 §3 (AJ-E3802): the router, one pure function per job over
//! the engines' cards. Under a millisecond, no I/O.
//!
//! **What decides a job's route, in order:**
//! 1. **Its flag** (D-130): off, the job doesn't exist ([`Route::Off`]).
//! 2. **A capable card:** context, tools and tier meet the job's needs,
//!    and the `models` section's verdict for the card's model passed the
//!    job (`beta` or `ships`, ADR-0035 §11.1). A card that is
//!    [`Readiness::NotHere`] isn't capable. None: [`Route::NoEngine`],
//!    with the reason the best match isn't here, if one is.
//! 3. **Allowed:** `ships`, or `beta` with the job's Beta on. Capable
//!    cards and none allowed: [`Route::BetaOff`] (the Beta switch is worth
//!    showing; nothing else is).
//! 4. **Ready:** allowed and none ready: [`Route::Waiting`].
//! 5. **The order among the ready ones, first that differs wins:** the
//!    user's own pick for the job; the recommendation for this Mac
//!    (AJ-E3811, the Automatic default); no downloaded file before one
//!    (D-121); higher tier; already running; the registry's order.
//!
//! One model attempt per job, never a chain: if the chosen engine fails,
//! the job shows its plain path (ADR-0033 §3).

use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::engine::{Card, Readiness, Reason};
use super::jobs::JobSpec;
use super::models::{Job, ModelEntry, Verdict};

/// A job's two switches now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switches {
    /// Its flag: compiled default, or the remote kill (D-130).
    pub on: bool,
    /// The user's Beta switch for it.
    pub beta: bool,
}

/// What the router reads besides a job's switches.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    /// Every engine's cards, in the registry's order.
    pub cards: &'a [Card],
    /// The models the cards may run, with their verdicts (the offered
    /// list and installed copies, ADR-0035 §11.4).
    pub models: &'a [ModelEntry],
    /// The recommended model for this Mac (AJ-E3811), the Automatic
    /// default.
    pub automatic: Option<&'a str>,
    /// The user's own model per job (AJ-E3812's override).
    pub picks: &'a BTreeMap<Job, String>,
}

/// Why the chosen card won: the first ordering rule that told it apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// The user picked this model for the job.
    Picked,
    /// It is the recommendation for this Mac.
    Recommended,
    /// It is the only one that can serve the job.
    Only,
    /// It needs no downloaded file.
    NoDownload,
    /// Its model's tier is higher.
    Quality,
    /// It is running already.
    Running,
    /// The registry's order.
    First,
}

/// A job's route now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The job's flag is off: it doesn't exist in this build.
    Off,
    /// No engine here can serve it: hidden. The reason the best match
    /// isn't here, when one is (a switch the user can turn on).
    NoEngine(Option<Reason>),
    /// An engine could serve it as a Beta, and the job's Beta is off.
    BetaOff,
    /// The chosen card (an index into [`Inputs::cards`]) isn't ready yet.
    Waiting {
        /// The card.
        card: usize,
        /// Why it waits.
        why: Reason,
    },
    /// The job runs on this card.
    Run {
        /// The card.
        card: usize,
        /// Why it won.
        why: Choice,
        /// It runs as a Beta (its verdict is `beta`).
        beta: bool,
    },
}

impl Route {
    /// Whether the job's surface is shown (it runs, or waits briefly).
    #[must_use]
    pub const fn shown(self) -> bool {
        matches!(self, Self::Run { .. } | Self::Waiting { .. })
    }
}

/// A card's place in the order for a job (lower wins): not picked, not
/// recommended, uses a download, tier (higher first), not running, the
/// registry's order.
type OrderKey = (bool, bool, bool, Reverse<u8>, bool, usize);

fn verdict(models: &[ModelEntry], card: &Card, job: Job) -> Verdict {
    models
        .iter()
        .find(|model| model.id == card.model)
        .map_or(Verdict::NotTested, |model| model.verdict(job))
}

fn fits(spec: &JobSpec, card: &Card) -> bool {
    card.context_tokens >= spec.needs.context_tokens
        && (card.tools || !spec.needs.tools)
        && card.tier >= spec.needs.tier
}

/// The ordering key of a card for `job` (lower wins).
fn key(inputs: &Inputs<'_>, job: Job, card: &Card) -> OrderKey {
    let picked = inputs.picks.get(&job).is_some_and(|id| *id == card.model);
    let recommended = inputs.automatic == Some(card.model.as_str());
    (
        !picked,
        !recommended,
        card.uses_download,
        Reverse(card.tier as u8),
        !card.running,
        card.engine,
    )
}

/// `spec`'s route with `switches` over `inputs`.
#[must_use]
pub fn route(spec: &JobSpec, switches: Switches, inputs: &Inputs<'_>) -> Route {
    if !switches.on {
        return Route::Off;
    }
    let job = spec.job;
    let passed = |card: &Card| fits(spec, card) && verdict(inputs.models, card, job).passed();
    let capable: Vec<usize> = (0..inputs.cards.len())
        .filter(|&i| {
            let card = &inputs.cards[i];
            passed(card) && !matches!(card.ready, Readiness::NotHere(_))
        })
        .collect();
    if capable.is_empty() {
        let why = inputs
            .cards
            .iter()
            .filter(|card| passed(card))
            .min_by_key(|card| key(inputs, job, card))
            .and_then(|card| match card.ready {
                Readiness::NotHere(reason) => Some(reason),
                _ => None,
            });
        return Route::NoEngine(why);
    }
    let allowed: Vec<usize> = capable
        .into_iter()
        .filter(|&i| match verdict(inputs.models, &inputs.cards[i], job) {
            Verdict::Ships => true,
            Verdict::Beta => switches.beta,
            Verdict::NotYet | Verdict::NotTested => false,
        })
        .collect();
    let by_key = |i: &usize| key(inputs, job, &inputs.cards[*i]);
    let mut ready: Vec<usize> = allowed
        .iter()
        .copied()
        .filter(|&i| inputs.cards[i].ready == Readiness::Ready)
        .collect();
    ready.sort_by_key(by_key);
    let Some(&winner) = ready.first() else {
        return match allowed.iter().copied().min_by_key(by_key) {
            Some(card) => Route::Waiting {
                card,
                why: match inputs.cards[card].ready {
                    Readiness::Preparing(why) | Readiness::NotHere(why) => why,
                    Readiness::Ready => Reason::Unknown,
                },
            },
            None => Route::BetaOff,
        };
    };
    Route::Run {
        card: winner,
        why: why_chosen(&by_key(&winner), ready.get(1).map(by_key).as_ref()),
        beta: verdict(inputs.models, &inputs.cards[winner], job) == Verdict::Beta,
    }
}

/// The first ordering rule that told the winner from the runner-up.
fn why_chosen(winner: &OrderKey, runner_up: Option<&OrderKey>) -> Choice {
    if !winner.0 {
        return Choice::Picked;
    }
    if !winner.1 {
        return Choice::Recommended;
    }
    let Some(next) = runner_up else {
        return Choice::Only;
    };
    if winner.2 != next.2 {
        Choice::NoDownload
    } else if winner.3 != next.3 {
        Choice::Quality
    } else if winner.4 != next.4 {
        Choice::Running
    } else {
        Choice::First
    }
}

/// Every job's route, in `jobs`' order, with each job's switches.
#[must_use]
pub fn routes(
    jobs: &[&'static JobSpec],
    switches: impl Fn(&JobSpec) -> Switches,
    inputs: &Inputs<'_>,
) -> Vec<(&'static JobSpec, Route)> {
    jobs.iter()
        .map(|spec| (*spec, route(spec, switches(spec), inputs)))
        .collect()
}
