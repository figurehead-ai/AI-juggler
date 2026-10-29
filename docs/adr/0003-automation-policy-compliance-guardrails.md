# ADR-0003: Automation policy & compliance guardrails

**Status:** Accepted
**Date:** 2026-10-12 (revised after review round 2; widened 2026-10-22 for AJ-E1509, points 3 and 5; amended 2026-10-25 for D-40, points 7 and 8)
**Deciders:** maintainer

## Context

Vendors' terms (analysis in [research/landscape-and-stack.md §3](../research/landscape-and-stack.md)) prohibit account sharing, reselling/proxying access, circumventing per-account limits, and automated logins — but not owning multiple paid accounts. The product's core flows (switching, handover) must stay clearly on the right side of those lines while remaining one-click.

## Decision

1. The only automated behaviors are **passive**: monitoring, indexing, accounting, notifications, alerts, suggestions, and staging of actions.
2. Every state-changing action — account switch for new work, session transplant, target priming, project sync, remote command — **executes on an explicit user click**.
3. One narrow, opt-in exception (off by default): **auto-restart on crash**, globally or per launch setting, which relaunches into the *same* profile only — never after a quota stop, a stop the user made or a sign-in failure, and at most 3 times in 10 minutes (so it can never work around a limit).
4. Additional hard guardrails (spec proposal §5): no login automation; no traffic proxying/MITM; spawn unmodified real CLIs; local-only data; vendor terms surfaced in-product.
5. **Keeping AI Juggler and the Mac working** is not an account action: holding the Mac awake while sessions work (a visible default, on when plugged in, never against a sleep the user asks for), opting out of App Nap while sessions are supervised, opening at login (opt-in) and coming back after a crash. Each is a setting the user can see and change, and none of them touches an account or starts new work (revised 2026-10-22, AJ-E1509).
6. These are enforced by a dedicated guardrail test suite in CI (writes confinement, network allowlist, no account mutation without user command).

## Alternatives considered

- **Autonomous quota-driven rotation** — squarely inside "circumvent rate limits" clauses; rejected.
- **Proxy/gateway model (claude-code-router style)** — inside Anthropic's "route requests through consumer subscriptions" prohibition; rejected.

## Consequences

- The product is the most conservative tool in its category — a supervisor, never a middleman — which is also its differentiation.
- Every feature must degrade gracefully if a vendor tightens terms (e.g. usage dashboards remain valuable single-account).
- Handover UX must do the preparation work so the "one click" is genuinely one click.

## Amendment, 2026-10-25 — sessions AI Juggler drives (D-40, ADR-0022)

ADR-0022 lets AI Juggler drive a session through the tool's own protocol: Codex's app-server by default, and Claude Code's stream-json as a Beta choice. The guardrails above hold for those sessions, and two points are added.

**The wording this rests on** (Anthropic's *Legal and compliance* page, code.claude.com/docs/en/legal-and-compliance, read 2026-10-25; the full table is [research/session-runtime.md §2.1](../research/session-runtime.md#21-anthropic-claude)):

- Developers may not "route requests through Free, Pro, or Max plan credentials on behalf of their users", nor "collect, store, or intermediate Claude.ai credentials or session tokens"; sign-in "must complete through Anthropic's own flow".
- That "does not … prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription".
- The Agent SDK page adds: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK." Anthropic calls `claude -p` "the Agent SDK via the CLI", which is why driving Claude Code is a Beta, off by default, behind the four gates in ADR-0022 decision 4. The first gate is that Anthropic's published terms are rechecked each release ([RELEASING.md](../RELEASING.md) step 3) and still allow it.
- OpenAI's app-server page allows "a local or open-source application" to use app-server with Codex's own sign-in, and says it "has never been permitted for commercial or hosted services" (research §2.2).

**What follows for every driven session:**

7. **The binary is the user's own, unmodified, and signed in through its own flow.** AI Juggler bundles no SDK, reads no token, sets no credential variable and names itself honestly (`clientInfo.name` `ai_juggler`; no Claude Code entrypoint or header is spoofed). No launch drops a tool's own permission prompts: the flags that would (`--dangerously-skip-permissions`, `--yolo`, a bypass mode and the like) are refused for every runtime (`FORBIDDEN_ARGS`).
8. **Answers are always the user's (ADR-0022 decision 5).** A driven session's approvals and questions reach the user with the tool's own options, the tool's own "always allow" rules among them. AI Juggler carries the user's click to the tool and records it in the account log. It never answers by itself, on a timer or in bulk. This amends D-28 to "never by itself". The guardrail suite tests it (`crates/juggler-app/tests/runtime_guardrails.rs`).
