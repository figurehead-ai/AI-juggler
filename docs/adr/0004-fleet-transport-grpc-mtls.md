# ADR-0004: Fleet transport — gRPC over mTLS with mDNS pairing

**Status:** Accepted (wire-format detail protobuf vs JSON-RPC decided at the M5 spike)
**Date:** 2026-10-12
**Deciders:** maintainer + spec review (round 3)

## Context

AI Juggler instances must link across machines (spec §11): stream telemetry, carry user-clicked control commands, pair with zero config, and never leak vendor credentials. Requirements: standard and secure transport, versioned protocol across app versions, LAN-first with no cloud relay.

## Decision

gRPC over TLS 1.3 with **mutual certificate authentication** (`tonic` + `rustls`); per-instance Ed25519 identities generated at first run; pairing via user-verified 6-digit code / QR that pins both identities; discovery via mDNS/Bonjour (`mdns-sd`); remote machines join via the user's own VPN. Contracts are versioned in-repo with breaking-change linting; mixed-version fleets degrade to core telemetry.

## Alternatives considered

- **libp2p** — powerful peer networking but heavyweight and exotic for a LAN fleet; harder to audit.
- **Plain WebSocket + JSON** — simpler, but no standard schema/versioning story and weaker tooling for streaming contracts.
- **Cloud relay / vendor backend** — violates the local-first/privacy posture and adds a trust target; rejected outright.

## Consequences

- A standard, well-tooled transport with schema'd streaming; the pattern is proven in this exact ecosystem (Codex app-server is JSON-RPC over stdio; tonic is the Rust default).
- We own protocol versioning discipline (CI lint + degradation matrix tests).
- No relay means remote fleets depend on the user's VPN — acceptable, documented, and honest.
