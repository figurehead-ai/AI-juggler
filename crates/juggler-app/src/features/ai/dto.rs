//! The AI jobs' wire types (ADR-0034 §3.4, AJ-E3802): the one answer the
//! window reads to show or hide every AI feature.

use super::engine::Reason;
use super::models::Job;
use super::router::{Choice, Route};

/// A routed AI job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AiJob {
    /// The feed line.
    Feed,
    /// The turn review.
    Review,
    /// Chat's answer.
    Chat,
    /// The digest.
    Digest,
    /// Watches drafted from Chat.
    Watches,
}

impl AiJob {
    /// The job it stands for.
    #[must_use]
    pub const fn job(self) -> Job {
        match self {
            Self::Feed => Job::Feed,
            Self::Review => Job::Review,
            Self::Chat => Job::Chat,
            Self::Digest => Job::Digest,
            Self::Watches => Job::Watches,
        }
    }

    /// The wire job for a routed job (`None` for attention, which code
    /// decides with no engine).
    #[must_use]
    pub const fn of(job: Job) -> Option<Self> {
        match job {
            Job::Feed => Some(Self::Feed),
            Job::Review => Some(Self::Review),
            Job::Chat => Some(Self::Chat),
            Job::Digest => Some(Self::Digest),
            Job::Watches => Some(Self::Watches),
            Job::Attention => None,
        }
    }
}

/// Where a job stands now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AiJobState {
    /// It runs: show the feature.
    Ready,
    /// It will run soon (a model being prepared): show it, waiting.
    Waiting,
    /// A model could serve it as a Beta and its Beta is off: hide the
    /// feature; Settings may offer the job's Beta switch.
    BetaOff,
    /// No engine on this Mac can serve it: hide it.
    NoEngine,
    /// Not in this build (its flag is off): hide it, offer nothing.
    Off,
}

/// Why the engine was chosen, or why the job waits or is hidden: a key
/// for the words module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AiJobWhy {
    /// The user picked this model for the job.
    Picked,
    /// The recommendation for this Mac.
    Recommended,
    /// The only one that can serve it.
    Only,
    /// It needs no download.
    NoDownload,
    /// Its model is stronger.
    Quality,
    /// It is running already.
    Running,
    /// The first in order.
    First,
    /// This system has no such engine.
    NotThisSystem,
    /// This computer can't run it.
    NotThisComputer,
    /// The system's AI is turned off (the user can turn it on).
    TurnedOff,
    /// The system is getting its model ready.
    GettingReady,
    /// The helper is missing from this build.
    HelperMissing,
    /// Not built in this version.
    NotBuilt,
    /// It failed three times within an hour.
    Unhealthy,
    /// Its state couldn't be read.
    Unknown,
}

impl From<Choice> for AiJobWhy {
    fn from(choice: Choice) -> Self {
        match choice {
            Choice::Picked => Self::Picked,
            Choice::Recommended => Self::Recommended,
            Choice::Only => Self::Only,
            Choice::NoDownload => Self::NoDownload,
            Choice::Quality => Self::Quality,
            Choice::Running => Self::Running,
            Choice::First => Self::First,
        }
    }
}

impl From<Reason> for AiJobWhy {
    fn from(reason: Reason) -> Self {
        match reason {
            Reason::NotThisSystem => Self::NotThisSystem,
            Reason::NotThisComputer => Self::NotThisComputer,
            Reason::TurnedOff => Self::TurnedOff,
            Reason::GettingReady => Self::GettingReady,
            Reason::HelperMissing => Self::HelperMissing,
            Reason::NotBuilt => Self::NotBuilt,
            Reason::Unhealthy => Self::Unhealthy,
            Reason::Unknown => Self::Unknown,
        }
    }
}

/// One job's answer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AiJobInfo {
    /// The job.
    pub job: AiJob,
    /// Where it stands.
    pub state: AiJobState,
    /// It runs as a Beta (label it so, D-130).
    pub beta: bool,
    /// The user's Beta switch for it.
    pub beta_on: bool,
    /// The chosen (or waiting) model's id in the `models` section.
    pub model_id: Option<String>,
    /// Its display name, from the section (data, never a literal).
    pub model_name: Option<String>,
    /// Why that model, or why the job waits or is hidden.
    pub why: Option<AiJobWhy>,
    /// The user's own model for the job (AJ-E3812), when they picked one
    /// rather than Automatic; it wins whenever it can run.
    pub pick: Option<String>,
}

impl AiJobInfo {
    /// The answer for `job`'s `route`; `name` gives a model id's display
    /// name; `model` gives a card index's model id.
    #[must_use]
    pub fn new(
        job: AiJob,
        route: Route,
        beta_on: bool,
        model: impl Fn(usize) -> Option<String>,
        name: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let (state, beta, card, why) = match route {
            Route::Off => (AiJobState::Off, false, None, None),
            Route::NoEngine(why) => (AiJobState::NoEngine, false, None, why.map(AiJobWhy::from)),
            Route::BetaOff => (AiJobState::BetaOff, false, None, None),
            Route::Waiting { card, why } => {
                (AiJobState::Waiting, false, Some(card), Some(why.into()))
            }
            Route::Run { card, why, beta } => {
                (AiJobState::Ready, beta, Some(card), Some(why.into()))
            }
        };
        let model_id = card.and_then(&model);
        Self {
            job,
            state,
            beta,
            beta_on,
            model_name: model_id.as_deref().and_then(name),
            model_id,
            why,
            pick: None,
        }
    }
}
