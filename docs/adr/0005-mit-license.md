# ADR-0005: License is MIT

**Status:** Accepted
**Date:** 2026-10-12
**Deciders:** maintainer (confirmed in review round 3)

## Context

Choice of OSS license for a Rust desktop application intended for community contribution and possible commercial use by third parties.

## Decision

Release under the **MIT License** (see [LICENSE](../../LICENSE)), copyright "Faruk Brbovic and AI Juggler contributors". Contributions are accepted under the same terms (CONTRIBUTING.md).

## Alternatives considered

- **Apache-2.0** — patent grant is attractive for corporate adopters; heavier to read. Revisit only if a contributor/adopter materially needs it.
- **MIT/Apache dual** (Rust ecosystem default for *libraries*) — overkill for an application; adds process weight without user benefit.
- **AGPL/copyleft** — suppresses adoption by companies, contradicting the goal of becoming the default manager.

## Consequences

- Maximum simplicity and adoption; macOS binaries are signed by the publisher (Vuxtra Developer ID), which is independent of the code license (spec §10).
- If we later extract libraries worth publishing (e.g. the adapter SDK), those crates can be dual-licensed MIT/Apache by decision of the maintainers.
