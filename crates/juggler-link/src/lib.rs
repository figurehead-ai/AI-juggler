//! Fleet link — multi-machine operation (spec §11, ADR-0004).
//!
//! Connects AI Juggler instances over the local network: `mDNS`/Bonjour
//! discovery, code-based pairing with per-instance `Ed25519` identities,
//! `gRPC`-over-`mTLS` telemetry and control channels. Invariants: no cloud relay,
//! vendor credentials never travel over the link, transcripts move only on
//! explicit user-initiated handover.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]
