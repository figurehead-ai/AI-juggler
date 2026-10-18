# ADR-0007: Instance-centric information architecture (design iteration 4)

- Status: Accepted
- Date: 2026-10-12
- Deciders: Vuxtra engineering + maintainer design review
- Relates to: ADR-0002 (adapter architecture), SPEC §9 (App & UX)

## Context

The M1 build-out drifted toward a "manager for CLI sessions": the left
rail exposed Dashboard / Sessions / Profiles / Usage / Fleet as peers, and
the terminal dock became the de-facto home of everything. Maintainer
feedback at the M1-4 checkpoint called this out — the product was
described as drifting toward generic "t3.codes-style" multi-session
tools. The original product intent is broader: **AI Juggler manages
multiple parallel accounts across harness tools of any form — CLI now,
desktop-class harnesses next** — with onboarding, quotas, handover, and
machine awareness around them.

The actual user workflow has two modes:

1. **Global / setup mode** — add and configure accounts and profiles,
   see the all-up dashboard, usage, fleet. Occasional, oversight-shaped.
2. **Work mode** — navigate into a specific running instance (a CLI in
   its terminal, or a desktop harness window) or a specific session and
   act on it: watch it, focus it, stop it, hand it over. Daily,
   object-shaped.

## Decision

0. **Projects are the juggling anchor.** The product's namesake power is
   *switching accounts on the same local project* — run out on one
   account, another takes over. A **project** (a canonical directory) is
   a first-class entity: sessions and instances are associated with the
   project they operate on, so the UI can always answer "which instance
   is managing this project, with which account, and what happened here
   before" — the precondition for switch / migrate / hand-over.
1. **Domain: instances are first-class.** An *instance* is one
   running/tracked occurrence of a harness under an account + profile —
   `kind: Terminal | Desktop`. A *session* is the work conversation
   living inside an instance (resumable, handover-able). What the code
   called a launched "session" is, in product language, an instance;
   storage carries `kind` from schema v3.
2. **Left rail = work mode.** Three anchors: **Projects** (the juggling
   surface — which instance is on it, account history, launch-with,
   hand-over), **Instances** (every running/tracked occurrence), and
   **Sessions** (the work conversations). Clicking an instance opens its
   **instance panel** (status, account/profile, kind, age, resources,
   actions: open terminal / focus window, stop, relaunch — hand-over
   when it lands).
3. **Header = global mode.** Overview, Accounts & Profiles, Usage, Fleet
   move to top-right global menus; they are configuration and oversight,
   not the daily workflow. The primary "+ Add" action lives there too
   (onboarding entry point).
4. **Terminal dock stays, demoted to a surface.** Terminal tabs are how
   *terminal-kind* instances run; desktop-kind instances have no terminal
   — their panel is the surface (focus window, status via transcript
   watching). The dock is an execution surface, not the navigation model.
5. **Desktop harnesses are first-class roadmap citizens** (not
   "post-1.0 extensibility"): adapters launch them as supervised
   processes (`needs_pty = false`), track status via transcripts where
   the harness writes them, and declare their surface kind. Per-vendor
   ToS and multi-instance capability checks gate each desktop adapter.

## Consequences

- The rail says what the product is: you manage instances, you continue
  sessions. Global concerns are one click away without competing for
  rail space.
- The old "session board" milestone (M1-6) is reframed as the
  **Instances list + instance panel** and the **Sessions screen**; the
  acceptance criteria (live updates < 300 ms, attach/stop actions,
  Playwright smoke) carry over.
- Schema v3 adds `kind` to the launch-record table; UI badges render it.
  Desktop-kind instances appear in the UI before the first desktop
  adapter ships (empty-state honest about that).
- Future harness additions (desktop Claude Code, IDE-extension harnesses)
  extend the adapter trait with a surface concept rather than forcing
  terminals.
