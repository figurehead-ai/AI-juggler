//! Watches (AJ-E3814, D-132, ADR-0036 §2): a typed, closed-set rule AI
//! Juggler evaluates itself on its own event stream, with no model in the
//! loop. "Tell me when the api and web agents finish or get stuck" is a
//! [`model::Watch`] the user confirmed, evaluated deterministically by
//! [`engine::Engine`] on the observation bus ([`crate::features::observe`]).
//!
//! - **The model** ([`model`]): targets (a session, an App, a project, a
//!   pool, an account, any session), conditions from the closed set with
//!   typed, bounded parameters, each or all, a lifetime (with ADR-0036's
//!   default when unsaid), and the user's own "Even during quiet hours",
//!   off by default. No action field.
//! - **The closed set** ([`conditions`]): one module per condition,
//!   registering one rule. The engine has no per-condition branch, so a
//!   new condition changes nothing else.
//! - **The engine** ([`engine`]): pure; the same observations give the
//!   same fires; deadlines rather than polling; an ended or removed
//!   target ends its watch with a reason, never silently.
//! - **What is offered** ([`catalogue`]): only conditions a target's Apps
//!   can report (`ToolSpec.observes`, D-127).
//! - **The words** ([`echo`], [`words`]): code-written, English and German.
//! - **Storage** ([`store`]): ADR-0037's store and log, written only by
//!   the commands the user's click calls ([`commands`]); the engine never
//!   writes a watch. Behind V54, which waits for 0.4.6's V53.
//! - **At run time** ([`runtime`]): the engine on its own thread, fed by
//!   the bus, its fires kept as typed alerts for AJ-E3817's notification
//!   path (quiet hours, consolidation and delivery are AJ-E3817's).
//!
//! What today's event stream feeds (published from the notification
//! path): a session's statuses (finishes, asks you, fails, looks stuck,
//! ends, hits a limit, idle for N minutes), processes left, a crash's
//! restart, an account's reset. Waiting for a source: exact turn ends of
//! driven sessions and other apps' conversation states (AJ-E3705), the
//! account's share left for "runs below" (AJ-E3704), new sessions'
//! starts, pool messages and task moves (the bus at their journals,
//! AJ-E3712), commits (AJ-E3706; not offered until then), removals, and
//! the replay of the journals past each watch's cursor on start.

pub mod catalogue;
pub mod commands;
pub mod conditions;
pub mod echo;
pub mod engine;
pub mod model;
pub mod runtime;
pub mod store;
pub mod words;

pub use runtime::Watches;

#[cfg(test)]
mod tests;
