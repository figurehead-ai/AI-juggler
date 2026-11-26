//! ADR-0034 §2 (AJ-E3802): the AI jobs the router serves, each with its
//! three needs and its own flag (D-130).
//!
//! **A job's flag decides whether it exists in this build at all;** every
//! flag is `OffOnly`: the remote file may stop a job everywhere, never
//! start one. **Within a job that exists, its verdict decides** (the
//! `models` section, data): a model serves the job only with a `beta` or
//! `ships` verdict, and `not_yet` keeps the plain path with its one-line
//! why on the Models page. D-149 (AJ-E3830, AJ-E3831): the feed line, the
//! digest and Bot's watch drafts are built, so their flags are on and each
//! stays plain behind its `not_yet` verdict until a harness v6 round
//! passes; that round flips it by a recorded verdict, not a code change
//! (`docs/qa/evidence/AJ-E3709/harness/v6/v6-runbook.md`). AJ-E3820 built
//! the turn review, so its flag is on too, and Apple's `beta` verdict
//! offers it as an opt-in Beta.
//!
//! | Job | Flag default | Verdict on Apple's model today |
//! |---|---|---|
//! | Feed line | on | `not_yet`: A's excerpt; "pick, don't write" is re-measured (D-149) |
//! | Turn review | on | `beta`: opt-in Beta (D-149); the code-only review is the no-model path (AJ-E3820) |
//! | Bot | on | `beta`: opt-in Beta |
//! | Digest | on | `not_yet`: the code roundup; map and reduce is re-measured (D-149) |
//! | Watches from Bot | on | `not_yet`: the Watches form; Bot fills it at >= 90 % exact (D-149) |
//!
//! Attention isn't routed: it is decided by code from the facts (D-117),
//! with no model in the loop, so it needs no engine. Its verdict in the
//! `models` section is a record, not a route.
//!
//! **Beta is the user's own switch, per job,** off by default (D-130's
//! "opt-in"): a model whose verdict for a job is `beta` serves it only
//! with that job's Beta on; `ships` needs no switch.

use juggler_remote_config::RemoteMode;

use super::engine::{Needs, Tier};
use super::models::Job;
use crate::features::flags::Flag;

/// A job the router serves: which one, what it needs, and its flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobSpec {
    /// The job (its verdicts' field in the `models` section).
    pub job: Job,
    /// Its three needs.
    pub needs: Needs,
    /// Its own flag: whether it exists in this build.
    pub flag: &'static Flag,
}

/// The feed line's flag: on; its verdict keeps A's excerpt until a
/// model's line is preferred to it (ADR-0033 §6.1, D-149).
pub const FEED_ON: Flag = Flag {
    name: "ai_feed_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// The turn review's flag: on (AJ-E3820 R4, D-149): the review is built
/// (`features::review`), so its verdict decides: Apple's `beta` serves
/// it once the user turns its Beta on; otherwise the code-only review.
pub const REVIEW_ON: Flag = Flag {
    name: "ai_review_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// Bot's flag (opt-in Beta, D-131; named `ai_chat_on` on the wire, D-136 kept it).
pub const BOT_ON: Flag = Flag {
    name: "ai_chat_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// The digest's flag: on; its verdict keeps the code roundup until the
/// map-and-reduce digest is preferred to it (D-149).
pub const DIGEST_ON: Flag = Flag {
    name: "ai_digest_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// Watches drafted from Bot: on; its verdict keeps the plain Watches
/// form until drafts match at least 90 % of requests (D-149).
pub const WATCHES_ON: Flag = Flag {
    name: "ai_watches_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// The feed line: a calm item's one line (ADR-0033).
pub const FEED: JobSpec = JobSpec {
    job: Job::Feed,
    needs: Needs {
        context_tokens: 1_024,
        tools: false,
        tier: Tier::Unknown,
    },
    flag: &FEED_ON,
};

/// The turn review: what a turn did, checked against the facts.
pub const REVIEW: JobSpec = JobSpec {
    job: Job::Review,
    needs: Needs {
        context_tokens: 2_048,
        tools: false,
        tier: Tier::Unknown,
    },
    flag: &REVIEW_ON,
};

/// Bot's answer: look-ups, then a cited answer (ADR-0034 §6).
pub const BOT: JobSpec = JobSpec {
    job: Job::Bot,
    needs: Needs {
        context_tokens: 4_096,
        tools: true,
        tier: Tier::Unknown,
    },
    flag: &BOT_ON,
};

/// The digest: a rollup of what happened while away.
pub const DIGEST: JobSpec = JobSpec {
    job: Job::Digest,
    needs: Needs {
        context_tokens: 4_096,
        tools: false,
        tier: Tier::Unknown,
    },
    flag: &DIGEST_ON,
};

/// A watch drafted from a Bot request (a typed rule, ADR-0036).
pub const WATCHES: JobSpec = JobSpec {
    job: Job::Watches,
    needs: Needs {
        context_tokens: 2_048,
        tools: false,
        tier: Tier::Unknown,
    },
    flag: &WATCHES_ON,
};

/// Every job the router serves, in the order the query lists them.
pub const JOBS: [&JobSpec; 5] = [&FEED, &REVIEW, &BOT, &DIGEST, &WATCHES];

/// Every job's flag, for the app's flag list ([`crate::features::flags`]).
pub const FLAGS: [&Flag; 5] = [&FEED_ON, &REVIEW_ON, &BOT_ON, &DIGEST_ON, &WATCHES_ON];

/// The app setting that holds `job`'s Beta switch (`1` on; anything else,
/// or none, off). Written only through `ai_beta_set`, so the answer is
/// told at once.
#[must_use]
pub fn beta_key(job: Job) -> String {
    format!("{BETA_PREFIX}{}", job.name())
}

/// Every Beta switch's key starts with this (reserved: the general
/// settings command refuses it).
pub const BETA_PREFIX: &str = "ai_beta_";

/// The app setting that holds the user's own model for `job` (AJ-E3812's
/// per-job override): a model id, or empty for Automatic. Written only
/// through `ai_job_pick_set`, which checks the model may serve the job.
#[must_use]
pub fn pick_key(job: Job) -> String {
    format!("{PICK_PREFIX}{}", job.name())
}

/// Every per-job pick's key starts with this (reserved: the general
/// settings command refuses it).
pub const PICK_PREFIX: &str = "ai_pick_";

/// The routed job named `name` (its field name in the `models` section).
#[must_use]
pub fn by_name(name: &str) -> Option<&'static JobSpec> {
    JOBS.iter().copied().find(|spec| spec.job.name() == name)
}
