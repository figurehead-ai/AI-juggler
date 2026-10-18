# ADR-0002: Harness integration via adapter trait + capabilities

**Status:** Accepted
**Date:** 2026-10-12
**Deciders:** maintainer + spec review (round 1)

## Context

We manage multiple AI coding harnesses (Claude Code, Codex, ZCode first, more later) whose config systems, auth storage, session formats, resume flags, and quota surfaces all differ and change version-to-version (e.g. Codex broke profile semantics in 0.134.0; Claude ships weekly). Extensibility is a stated product goal (spec G5).

## Decision

All harness specifics live behind a `HarnessAdapter` trait (spec §4) with explicit `Capabilities` flags, a runtime `discover()` probe with `verified_against` version ranges, read-only session watchers, and a neutral versioned Transcript IR shared by the indexer and handover engine. First-class Rust adapters ship in-repo; a declarative manifest for simple CLIs comes post-v1. Adapters are the only code that touches harness internals — enforced by layering rules in CI (QUALITY.md §4).

## Alternatives considered

- **Per-harness modules inside core** — couples core to format churn; blocks external adapters.
- **Config-only "profile switcher" (cc-switch model)** — cannot support live sessions, health, or handover.
- **Driver binaries per harness** — heavier distribution/testing burden with no benefit at our scale.

## Consequences

- New harness = one crate/module implementing the trait + fixture corpus; core, supervisor, and UI unchanged.
- Format drift is handled in one place with version-pinned parsers and capability probing; unknown versions degrade gracefully instead of crashing.
- The trait is semver-protected public API (`cargo-semver-checks` in CI) since external adapters depend on it.
