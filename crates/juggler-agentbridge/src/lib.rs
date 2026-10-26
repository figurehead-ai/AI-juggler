//! The agent bridge (AJ-E1704, ADR-0020): `juggler-agent`, the program
//! that gives the agents in AI Juggler's sessions its pool tools.
//!
//! - With `--mcp` it is an MCP server on stdio ([`mcp`]); otherwise a
//!   plain command line ([`cli`]) for tools without MCP, called from
//!   their shell.
//! - It holds no state. Every call goes to the running app over its
//!   per-edition Unix socket ([`client`]), with the session's key.
//! - Its identity is that key: a random token per session epoch, which
//!   AI Juggler puts in the environment of the session it starts (never
//!   in its arguments) and renews on every switch or resume. The program
//!   reads it from its own environment ([`wire::TOKEN_ENV`]); nothing
//!   else carries it.
//! - The roster and the agent's own status line: `whoami`,
//!   `pool_roster`, `set_status` and the pool's cards as a resource; E18's
//!   messages and shared docs: `send_message`, `read_inbox`, `ack`,
//!   `share_doc`, `read_doc` ([`wire::TOOLS`]). What a call may do is the
//!   pool's access level, decided in the app (D-28); this program only
//!   asks.

// Test code is pragmatic by policy (QUALITY.md §2).
#![cfg_attr(
    test,
    allow(
        clippy::pedantic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

pub mod cli;
pub mod client;
pub mod hook;
pub mod mcp;
pub mod wire;
