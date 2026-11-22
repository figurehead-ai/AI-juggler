//! The agent digest (AJ-E3712, D-132, ADR-0036 §1): every so often while
//! agents work, a few short lines on Home's feed saying what each agent
//! is on and what is notable, and one notification for the calm news it
//! carries, so seven agents finishing are one ping, not seven.
//!
//! - [`scheduler`]: event-driven and pure; no digest while nothing
//!   changes, every 15 to 30 minutes while agents work, a minute after a
//!   notable event (a burst is one digest), none while the user is away
//!   and one on return.
//! - [`rollup`]: the non-AI digest, deterministic, every agent accounted
//!   for, needs and failures first, with round 3's rules on state words.
//!   It is the feature in 0.5: the AI digest is "not yet"
//!   (`docs/qa/evidence/AJ-E3709/v5-final.md`) and stays behind its flag
//!   (`ai_digest_on`, off) and the router.
//! - [`store`]: each digest on ADR-0037's store, kept 30 days.
//! - [`source`]: the feed's digest item, one more `FeedSource`.
//! - [`runtime`]: the thread, the facts gathered, the notification
//!   handed to the existing path (the batch, quiet hours, the user's
//!   kinds; "The digest" is off by default).
//!
//! Its input today: what the observation bus hears (a session's states,
//! its ending, processes left, a crash's restart) and the session rows
//! (name, App, files changed). Turn ends, tests and the agents' own words
//! (A's excerpt) join when AJ-E3705 and AJ-E3708 record them; the rollup
//! already takes them and leaves out what isn't known.

pub mod rollup;
pub mod runtime;
pub mod scheduler;
pub mod source;
pub mod store;

pub use runtime::{Digests, carries_finishes};
