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
> maintainer before anything is built. The maintainer approved it on
> 2026-10-30 (D-62, decisions 8–18); the build tasks below are written
> from it. They follow E32 (the stored Account and App, which the App,
> Account and Tool members use) and E31 (the pages and the one list).
> AJ-E3002's labs come first, because decisions 13, 14 and 17 are built
> only where a lab says yes.
>
> **Validated by use (D-62).** Each task names the scenarios of
> [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md) it must
> pass (P1–P11) and the layers that run them; it isn't done until its
> sheets in `docs/qa/0.4-walks/` pass with nothing scored under 4. E33
> holds the shared tools and the release-candidate pass.
>
> **Words.** The proposal's "Profile" member is App and its "App" member
> is Tool (D-62 decision 1).

## Step 1 — research and a proposal

### AJ-E3001 Pools from real use: research, the model and a proposal for review | Status: DONE (2026-10-30: `docs/research/pools-model.md`, the review page, approved as D-62) | Priority P0 | Effort L | Depends: —
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

## Step 2 — labs that gate the rules

### AJ-E3002 Labs: talk enforced per session, Claude's own channel, OpenCode and Grok tools per launch | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3001
**Goal**: Know, from live runs, which of the approved rules AI Juggler can actually enforce, before building them.
**User story**: As the maintainer, I read a yes or no per rule with its evidence, so nothing on the canvas promises what the tools won't do.
**Requirements**:
1. A lab SHALL show that a held or refused message between two sessions AI Juggler started (Claude Code, Codex) is enforced by the bridge per receiving session and per pool.
2. A lab SHALL show whether Claude Code's cross-session inbound setting, passed through the launch's `--settings` overlay, refuses or holds Claude's own peer messages for that one session, and what the Claude desktop app does with a held one.
3. A lab SHALL show whether OpenCode (`OPENCODE_CONFIG_CONTENT`) and Grok Build (its config overlay) load AI Juggler's agent tools per launch with no prompt; Kimi Code stays on its shell.
4. Each result SHALL be recorded in `docs/qa/pools-lab.md` with its commands and output; a "no" changes the build tasks before they start (the rule is shown as not enforceable, never faked).
**UI/UX reasoning**: honest capability: a rule the tool can't keep is labelled, never drawn as if kept.
**Performance budget**: n/a.
**Quality gates**: throwaway lab only, spare accounts; no real profile launched.
**Evidence/validation proof**: `docs/qa/pools-lab.md`; the decision log notes any "no".
**Visual testing**: n/a.
**Files to touch**: `docs/qa/pools-lab.md` (new), `crates/juggler-app/src/agent_bridge/` (lab probes only).
**Research basis**: `docs/research/pools-model.md` (capability table); ADR-0020; the 2026-10-26 lab.
**Handover note**: Gates D-62 decisions 13, 14 and 17.

## Step 3 — the model

### AJ-E3003 The pool model stored: scope, member kinds, several pools, levels per member, pair rules, sealed, and today's pools moved | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3202, AJ-E3002
**Goal**: Store everything D-62 decisions 8–16 need, additively, and move 0.3's pools with nothing lost.
**User story**: As someone upgrading from 0.3, every pool, member, message, task and canvas position I had is still there, and my pools with no project now say "Every project".
**Requirements**:
1. Schema v40+ SHALL add scope rows (every project, these projects, no project), member kinds (Session, App, Account, Tool, Mac, everyone in scope), a per-member level, role-pair talk rules with per-connection overrides, a sealed flag and a project on tasks in multi-project pools; nothing existing is dropped or rewritten.
2. 0.3's one-pool-per-session index SHALL become one row per pool and member.
3. The move SHALL map: a project's default pool → that project, everyone in scope, "skip sessions another pool holds" on, same id, can't be deleted; the no-project pool → no project, everyone; a user pool with a project → that project; one with none → every project. A backup is taken first.
4. The Reviewer role SHALL be added beside Lead, Member and Observer.
**UI/UX reasoning**: nothing the user made changes meaning on upgrade (P9).
**Performance budget**: the move runs in < 1 s for 100 pools and 10 000 messages.
**Quality gates**: L1 migration table over every 0.3 pool kind × content; the reference diff shows additions only.
**Evidence/validation proof**: scenario P9; `crates/juggler-core/tests/pools_v40.rs`.
**Visual testing**: project pools render as before (gallery unchanged).
**Files to touch**: `crates/juggler-core/src/storage/mod.rs`, `storage/pools.rs`, `crates/juggler-app/src/newer_data.rs`.
**Research basis**: `docs/research/pools-model.md` "Moving today's pools".
**Handover note**: The schema number follows whatever 0.3.x has taken by then.

### AJ-E3004 Membership resolved: who is in which pool, how, and with what role and level | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3003
**Goal**: Every session's pools, role and level come from one resolver that follows D-62 decision 11 exactly.
**User story**: As someone whose App is a pool member, every new session of that App in scope joins by itself, and its page says how it got in.
**Requirements**:
1. Membership SHALL resolve from in-memory maps (App, Account, Tool, Mac and project to pools), rebuilt on change events, never by re-reading sessions.
2. Inside one pool the most specific member SHALL win (Session › App › Account › Tool › Mac › everyone); across pools each pool's rules govern its own messages, tasks and docs; a sealed pool takes its sessions out of every other pool.
3. An out-of-scope or sealed-elsewhere session SHALL be refused with its reason and the step that would let it in; an unreadable sign-in reads "Account unknown" and the session stays in through any other member.
4. The bridge's whoami SHALL list the session's pools; a write names its pool and is refused if the session isn't in it (ADR-0020 §6 amended).
**UI/UX reasoning**: every "why is this here / why not" has an answer on screen.
**Performance budget**: a membership change resolves in ≤ 10 ms for 500 sessions and 50 pools; a pool of 50 opens from the cached roster.
**Quality gates**: L1 exhaustive (28 224 cases) against `scripts/qa/pool_oracle.py`; L2 on the replica.
**Evidence/validation proof**: scenarios P4, P5, P11; X2.
**Visual testing**: the session page's Pools tab (how it got in, skipped from).
**Files to touch**: `crates/juggler-core/src/pools/` (resolver), `crates/juggler-app/src/agent_bridge/`, `docs/adr/ADR-0020*.md`.
**Research basis**: `docs/research/pools-model.md` "Precedence", "Several pools per session".
**Handover note**: The same maps feed E31's Pools tabs and counts.

### AJ-E3005 Roles, levels and talk enforced: Reviewer, per-member level, allowed / ask first / blocked, sealed | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3004, AJ-E3002
**Goal**: What an agent may do and whom it may tell is decided by the pool's rules, and the person sees every hold.
**User story**: As a feature-team lead, a member's message to another member waits in Needs you until I deliver it, and a sealed pool lets nothing out.
**Requirements**:
1. Talk SHALL be decided per role pair with any single connection overridden: allowed delivers (waking only at Full or by a rule, D-29), ask first holds in Needs you, blocked refuses and tells the agent why.
2. The Reviewer SHALL read every task, doc and card, message anyone, take review tasks and move one to approved or sent back as its own action, which asks the person at the pool's level; the final Approve stays the person's (D-44).
3. A member's own level SHALL replace the pool's for that member.
4. Sealed SHALL refuse links and every message or doc out; Claude's own channel per receiving Claude session SHALL be set to refuse in a sealed pool and hold where any pair into it holds or blocks, as far as AJ-E3002 proved.
5. Two pools waking one session SHALL respect the session's rate limit and each thread's hop limit; Stop all is per pool.
**UI/UX reasoning**: one count of holds everywhere; a refusal always says why.
**Performance budget**: a talk decision is ≤ 1 ms.
**Quality gates**: L1 exhaustive talk table (9 216 cases) and the Claude channel table (27) against the reference model; live checks in the lab.
**Evidence/validation proof**: scenarios P2, P4, P7, P11.
**Visual testing**: the held message in Needs you and on the canvas edge.
**Files to touch**: `crates/juggler-core/src/pools/`, `crates/juggler-app/src/agent_bridge/`, `crates/juggler-app/src/features/messages/`.
**Research basis**: D-28, D-29, D-44; `docs/research/pools-model.md` "Roles and access", "Communication".
**Handover note**: A rule AJ-E3002 found unenforceable shows as "can't be enforced for this tool", never as kept.

## Step 4 — on screen

### AJ-E3006 Sessions other apps run, as Watched members, and Continue in AI Juggler | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3004
**Goal**: Every session a person has can be shown in a pool, and the one step that gives it tools is right there.
**User story**: As someone with a Claude app session in my project, I add it to the pool as Watched, and Continue in AI Juggler gives it the tools.
**Requirements**:
1. A session another app runs SHALL be addable as Watched: on the canvas dashed, in counts, in other agents' rosters as "can't reply"; no tools and no rules apply; a message to it is held with the reason.
2. Continue in AI Juggler SHALL resume the conversation in a launch that carries the tools, keep its membership and say a new process starts.
3. Shell-only members (OpenCode, Grok Build, Kimi Code until AJ-E3002 says otherwise) SHALL be labelled and do what their shell allows.
**UI/UX reasoning**: no hidden candidates (the maintainer's "where are all of the existing … sessions").
**Performance budget**: Continue shows progress at once.
**Quality gates**: L2 on the replica's Claude app sessions; L3 pairs.
**Evidence/validation proof**: scenarios P1 step 7, P8.
**Visual testing**: the Watched node and its fix, both themes.
**Files to touch**: `ui/src/screens/pools/`, `crates/juggler-app/src/commands/pools.rs`.
**Research basis**: `docs/research/pools-model.md` "Sessions other apps run".
**Handover note**: Joining from anywhere stays a later spike with its own ADR.

### AJ-E3007 New pool opens a wizard: templates, where and who from what exists, roles and talk, a review in sentences | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3005, AJ-E3105
**Goal**: A template gets most people to a working pool in one step, with every choice shown before it's made.
**User story**: As someone new to pools, I pick "Writer and reviewer", pick my two sessions from Open now, and make the pool; the review said exactly what it would do.
**Requirements**:
1. Step 1 SHALL offer the nine templates (writer and reviewer, feature team, across repositories, a reviewer for everything, rules by account, a build Mac, a sealed client project, scouts, blank), each with its roles and talk as chips.
2. Step 2 SHALL pick where (every project or these) and who from what exists: sessions, Apps, Accounts, Tools, Macs, everyone; Open now, Recent, then All, search past 8; every greyed candidate says why and offers the step that lets it in; a side panel counts what each choice brings and what joins later.
3. Step 3 SHALL set roles per member, a level per member ("pool's" by default), the role-pair talk grid and the seal, with its consequence in plain words.
4. Step 4 SHALL say each rule in a sentence and what each member can actually do here (full, shell only, watched).
5. "Set up by hand on the canvas" SHALL be on every step; Back keeps choices; Esc and Cancel leave nothing made.
**UI/UX reasoning**: prominence follows frequency; one primary action per step; the 14 principles (D-62 decision 22).
**Performance budget**: every step opens ≤ 100 ms; the candidate lists from cache.
**Quality gates**: L3 all-pairs (about 60 rows) with timings; a critique per step.
**Evidence/validation proof**: scenarios P1–P8.
**Visual testing**: each step, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/pools/NewPool*.tsx` (new), `ui/src/lib/pool-templates.ts` (new).
**Research basis**: the review's wireframes 1–4.
**Handover note**: Templates are data, so one is added without new screens.

### AJ-E3008 Set up by hand: the canvas that teaches, member group nodes, the add palette with reasons, an edge's rule | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3005
**Goal**: The canvas is where a pool is seen and changed, and it's clear what to do first.
**User story**: As someone who skipped the wizard, the empty canvas shows three numbered steps, and I make a working pool without reading docs.
**Requirements**:
1. The empty canvas SHALL show the three steps (where, who, lead and talk) with the first as the only primary button, and "Use a template".
2. A broad member SHALL draw as one dashed group node with its sessions and "new ones join"; past 50 members broad members group.
3. The add palette SHALL list every candidate by type with its reason; drag or Return adds.
4. Selecting a line between two members SHALL show its rule, let it be changed for that pair only, and list what's held between them with Deliver and Refuse.
**UI/UX reasoning**: one surface to see and manage; the inspector follows the selection.
**Performance budget**: a 50-member pool opens at once; a drag responds in one frame.
**Quality gates**: L3 pairs (about 40 rows); keyboard-only path.
**Evidence/validation proof**: scenarios P2, P10.
**Visual testing**: wireframes 5–6, both themes.
**Files to touch**: `ui/src/screens/pools/` (canvas, inspector, palette).
**Research basis**: E28's canvas; the review's wireframes 5–6.
**Handover note**: —

### AJ-E3009 Pools fit everything else: the switch check, notices, mutes, the Tasks filter, usage per pool | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3005
**Goal**: Pools work with switching, notifications, tasks and usage as the proposal's fit table says.
**User story**: As someone switching a pool member to another account, the switch check tells me what happens to its pools before I confirm.
**Requirements**:
1. The switch check SHALL give one line per pool: stays with full tools, shell only, watched, out of scope, or leaves a sealed pool (confirm).
2. Each notice SHALL name its pool; mutes SHALL include a pool.
3. The Tasks screen's pool filter SHALL follow the pickers rule, empty pools last.
4. A pool SHALL show "used this week" per member as a view only; it never ranks or picks by room (D-31).
**UI/UX reasoning**: everything flows together (D-62 decision 22, principle 1).
**Performance budget**: the switch check stays ≤ 150 ms.
**Quality gates**: L3 pairs; live switch in the lab.
**Evidence/validation proof**: scenarios P1 step 5, P7, X1.
**Visual testing**: the switch sheet's pool lines.
**Files to touch**: `ui/src/components/switch/`, `crates/juggler-app/src/features/notifications/`, `ui/src/screens/Tasks.tsx`.
**Research basis**: `docs/research/pools-model.md` "Fit".
**Handover note**: —
