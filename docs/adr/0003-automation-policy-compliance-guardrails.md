# ADR-0003: Automation policy & compliance guardrails

**Status:** Accepted
**Date:** 2026-10-12 (revised after review round 2)
**Deciders:** maintainer

## Context

Vendors' terms (analysis in [research/landscape-and-stack.md §3](../research/landscape-and-stack.md)) prohibit account sharing, reselling/proxying access, circumventing per-account limits, and automated logins — but not owning multiple paid accounts. The product's core flows (switching, handover) must stay clearly on the right side of those lines while remaining one-click.

## Decision

1. The only automated behaviors are **passive**: monitoring, indexing, accounting, notifications, alerts, suggestions, and staging of actions.
2. Every state-changing action — account switch for new work, session transplant, target priming, project sync, remote command — **executes on an explicit user click**.
3. One narrow, opt-in, per-profile exception (off by default): **auto-restart on crash**, which relaunches into the *same* profile only.
4. Additional hard guardrails (spec proposal §5): no login automation; no traffic proxying/MITM; spawn unmodified real CLIs; local-only data; vendor terms surfaced in-product.
5. These are enforced by a dedicated guardrail test suite in CI (writes confinement, network allowlist, no account mutation without user command).

## Alternatives considered

- **Autonomous quota-driven rotation** — squarely inside "circumvent rate limits" clauses; rejected.
- **Proxy/gateway model (claude-code-router style)** — inside Anthropic's "route requests through consumer subscriptions" prohibition; rejected.

## Consequences

- The product is the most conservative tool in its category — a supervisor, never a middleman — which is also its differentiation.
- Every feature must degrade gracefully if a vendor tightens terms (e.g. usage dashboards remain valuable single-account).
- Handover UX must do the preparation work so the "one click" is genuinely one click.
