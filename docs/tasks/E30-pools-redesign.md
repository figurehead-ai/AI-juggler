# E30 — Pools, redesigned from how people work

> Source: the maintainer, 2026-10-30, on 0.3.0's new-pool screen
> (verbatim):
> - "for the new pool we should have an additional wizard, that helps you
>   setup by picking from existing things and choosing from several
>   templates and helping you build it. And also obviously have manual
>   setup as is now."
> - "where are all of the existing projects, apps and sessions to create a
>   pool, all I see is one I copied ???"
> - "it is very confusing to understand how to create the pool in manual."
> - "How do pools work, have you really thought about it ?? Is it assigned
>   per project, general, where does that gets setup ?? I think pools
>   should have ability to be general, meaning all projects are visible,
>   but also pools should be able to be restricted to specific or multiple
>   projects ??? Also members of the pool can be a single session, or
>   whole app / account, and even whole machine (fleet)."
> - "remember the main purpose of the pool, to group Apps / accounts and
>   sessions, and give them certain access and roles, and possibly enable
>   or restrict communication between them. Have you fully validated and
>   researched capabilities and possibilities from multiple real world use
>   cases that will provide value ???"
> - On where it goes: "Add it to 0.4" (D-61).
>
> **What 0.3 has, and why it falls short.** Two kinds of pool: one per
> project, made automatically, and pools the user makes for one project or
> for none (no several-projects scope). Members are single sessions AI
> Juggler started, one pool each; a session another app runs can't join
> (it has no agent tools), and the add list leaves those out without
> saying why. No app, account or Mac as a member, nothing joins by itself,
> no templates, no guide. The pools spike that should have settled the
> model first (AJ-E1701) was never done.
>
> **Order.** AJ-E3001 is research and a proposal, reviewed by the
> maintainer before anything is built. The build tasks are written from
> the approved proposal and added here then; until that review, this epic
> holds one task.

## Step 1 — research and a proposal

### AJ-E3001 Pools from real use: research, the model and a proposal for review | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: A pool model grounded in how people actually run several agents, with the maintainer's approval before any build.
**User story**: As the maintainer, I read one proposal that shows the real situations pools serve, what each tool's agents can actually do, the model (scope, members, roles, communication), the wizard with its templates and the manual setup, and I approve or change it before code is written.
**Requirements**:
1. Use cases SHALL be researched from real practice (published multi-agent workflows, the tools' own docs on sub-agents and teams, the maintainer's own projects as read-only examples) and written as situations with who, what and the value each pool brings; at least: a feature team on one project; agents across repositories; a reviewer for everything; rules by account; a build Mac via Fleet; a sealed client project.
2. Each tool's capability SHALL be checked, not assumed: which can carry AI Juggler's agent tools (sessions AI Juggler starts; Claude Code with the plugin, AJ-E1905; Codex; the desktop apps), what a session another app runs can and can't do in a pool, and what that means for the user.
3. The model SHALL cover: scope (every project; one or several projects); member types (a session; a profile, so an app on an account, with its future sessions joining by themselves; an account; a Mac through Fleet) and what a member type means for rules and roles; roles and access per member (D-28's levels); communication between members (allowed, ask first, blocked; per pair or per role); how membership resolves when a session is covered twice (a session in its profile's pool and named in another); the project pools made automatically today and how they move to the new model without losing anything.
4. The creation flow SHALL be proposed with wireframes: a wizard (templates from requirement 1, picking from what exists: projects, apps, accounts, sessions, Macs) and the manual setup made clear (what to do first, what each control does); the add list shows every candidate with why one can't join and what would let it.
5. The proposal SHALL be a document the maintainer reviews (a shared page), with the decisions it asks for listed plainly; the build tasks of this epic are written from what's approved.
**UI/UX reasoning**: progressive disclosure (a template gets most people to a working pool in one step; depth stays one step away); one surface to see and manage (the canvas stays where a pool is seen and changed); plain words (D-25).
**Performance budget**: n/a (research); the proposal states the budgets the build will hold (a pool of 50 members opens at once; membership resolves without re-reading every session).
**Quality gates**: the proposal's claims about each tool carry evidence (a doc link or a live read-only check); no claim from memory.
**Evidence/validation proof**: `docs/research/pools-model.md` (the research), the proposal page, and the maintainer's recorded decision in the decision log.
**Visual testing**: wireframes in the proposal (both themes, 1280 and 1024).
**Files to touch**: `docs/research/pools-model.md` (new), `docs/review/decision-log.md` (the ruling), this epic (the build tasks, once approved).
**Research basis**: `docs/research/pools-messaging-tasks.md`; E17, E18, E19 and E28's pool tasks; ADR-0013 (Fleet); D-28 to D-33, D-55; AJ-E1701 (the spike never done).
**Handover note**: Takes over AJ-E1701's policy part; the bridge's live proof stays there. Readability (2026-10-30): the pool screens get the colour hierarchy pass with every screen.
