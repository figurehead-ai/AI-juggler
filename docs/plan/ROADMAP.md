# AI Juggler — Roadmap

The versions ahead, what each is for, and what it holds. Locked in by
the maintainer on 2026-10-30 (D-59). Each item names its task, so the
detail and the evidence live in [`docs/tasks/`](../tasks/README.md); this
page is the order and the reason.

## Where we are

| Version | Channel | What it brought |
|---|---|---|
| 0.1 | Beta | Accounts and profiles, sessions, switching A→B→Codex→A with history, tasks and context kept, honest usage, This Mac, onboarding |
| 0.2 | Beta | Switching to the account with the most room, one Needs you, notifications, pools with messages and tasks between agents, OpenCode, Grok Build and Kimi Code, Processes |
| 0.3 | Beta (2026-10-30) | Projects, Pools and Tasks in the header, the pool canvas, Subagents, the local cache (D-58), the redesign (E28). 0.3.1, in preparation: deleting launch settings keeps their sessions (D-60), Esc on New session, steadier tests |

Nothing is on Stable yet: the first Stable is 0.6.

## How releases flow

- **A bug in a shipped release goes to its patch line.** A bug found in
  0.3 ships as 0.3.1, 0.3.2 and so on, from `release/0.3` through
  `main`; it is never held for 0.4. Planned work goes to the next
  minor.
- **The next minor is built on its own branch** (`release/0.4`, version
  0.4.0) and merged into `main` once, when it is ready. Until then
  `main` carries the 0.3 line, so the nightly builds 0.3.x. Each patch
  that lands on `main` is merged into `release/0.4` so the fix is in
  both.
- **At most one release a day,** a hotfix or security fix excepted; a
  round of fixes ships together ([RELEASING.md](../RELEASING.md)).
- **Before a push to `main` or `release/*`:** `just preflight` on that
  commit, the whole of CI on this Mac ([QUALITY.md](./QUALITY.md) §7).

## 0.4 — Trust and the finished basics

**Intent.** Make AI Juggler trustworthy and complete for daily use:
secure enough to recommend, the spec's promises for the core workflows
actually there, and proven on real tools rather than only in tests.

**Why now.** 0.3 made everything easy to find. Before AI Juggler is
promoted, or becomes the base of a first Stable, it needs a security
pass (it holds sign-ins for many AI accounts, runs agents, has a local
agent bridge and pairs Macs), the features the spec and wireframes
promised but the code never got (`docs/review/spec-gap-audit.md`), and
proof on real accounts of what has so far been proven with fakes.

| Area | What you get | Tasks |
|---|---|---|
| Security foundations | Signing apart from the build; a locked-down webview (command allowlist, capabilities, navigation); the agent bridge's security design; the Fleet key in the Keychain; safe restore and import of hostile files; supply-chain evidence; scheduled fuzzing; a key-compromise runbook; process, crypto and listener hygiene; a UI linter and stricter TypeScript | E20: AJ-E2001…AJ-E2019 |
| The command palette | ⌘K does things: global commands, Accounts and apps, recents, fuzzy match, a preview, a real dialog | E29 |
| Cost and quality you can act on | Cost per finished task, wasted spend on crashed or stuck sessions, a weekly review, top project per account, retry rate and stuck episodes per account | E29 |
| Notifications tuned to you | A waiting threshold, quota at 80 and 95 %, per-project mute, Send test; a failed save puts the setting back | E29 |
| Settings that fit your hands | Search across Settings; shortcuts you can rebind, with a conflict check and reset | E29 |
| A first run that shows its confidence | Found, Likely and Guessed, with where each guess came from; Rescan at any time | E29 |
| Safe edges | Resume, Archive, Replay and Copy ID on an ended session; a keyboard-operable rail splitter; Home's filter chips and keys. (Deleting launch settings without losing their sessions ships earlier, in 0.3.1, D-60.) | E29 |
| Pools, redesigned from how people work | Research and a proposal for the maintainer's review first: pools for every project or one or several; members that are a session, an app on an account (its new sessions join by themselves), an account or a Mac; roles, access and who may talk to whom; a wizard with templates beside a clear manual setup. Built once the proposal is approved (D-61) | E30 |
| Everything links to everything | Every page (project, session, app, account, pool, Mac) leads to everything related, both ways, with counts; long lists open on the active and recent, with search and filters. Planned with the pools proposal, then built | E31 |
| Proven on real tools | Codex on real accounts; the runtime lab on real tools; the switching-fidelity matrix; Fleet on two Macs | AJ-E1305, AJ-E2301, AJ-E808, AJ-E710 |
| An honest picture | The shipped work of E21, E23, E24 and E25 evidenced and its statuses brought up to date; the README's roadmap matches this page | E21, E23, E24, E25 |
| Releases that don't stall | `just preflight`, the pre-push hook, CI parity, deterministic tests (already on `release/0.4`) | #198's follow-up |

**Done when** every task above is evidenced, the full preflight passes,
the maintainer has used it, and its findings are fixed.

## 0.5 — More tools, and teams

**Intent.** Bring the tools people use next, and let work move between
people as easily as between accounts.

| Area | Tasks |
|---|---|
| ZCode, Kiro CLI, Hermes Agent, Muse, Cursor CLI, Antigravity on work plans; the desktop apps, track-only | AJ-E1605…AJ-E1610, AJ-E1613 |
| Kimi Code's and Grok Build's usage finished | AJ-E1603, AJ-E1604 |
| Every new tool checked live on spare accounts | AJ-E1612 |
| The Claude Code plugin published | AJ-E1905 (waits on D-43 point 6) |
| Pools across your Macs, after the pools spike's rulings | AJ-E1701, AJ-E1705 |
| A context pack handed to a colleague | AJ-E1011 |
| Crash reports behind their own setting | AJ-E1107 |
| Pair review, model arbitrage and an outage day: falsified first, then built or dropped | AJ-E1006, AJ-E1008 |

## 0.6 — The first Stable

**Intent.** A polished macOS release people can rely on, and the start
of public promotion.

| Area | Tasks |
|---|---|
| The 10-account, 50-session load proof; a cold start budget on the release build | AJ-E805 |
| VoiceOver on every release | AJ-E804 |
| Updates and release integrity finished | AJ-E1010 |
| The gate packet signed | AJ-E807 |
| Two weeks of the maintainer's use, then a 0.6.x release candidate | AJ-E809 |

The Stable channel opens with it (SPEC §15, open question 2), and the
stricter Beta rule (D-24's three days of use) applies from then on.

## 0.7 — AI Juggler for Windows

A dedicated first release, then Windows in every release on the same
version and channels (D-52, [E26](../tasks/E26-windows-release.md)).
Packaging, signing and scope are the maintainer's decisions first
(AJ-E2601).

## 0.8 — AI Juggler for Linux

A dedicated first release, then Linux in every release (D-52,
[E27](../tasks/E27-linux-release.md)). Formats, repository and the
support matrix are the maintainer's decisions first (AJ-E2701).

## 1.0

Stabilization from beta feedback (the Fleet protocol, the adapters, the
UX), only after Windows' first release has succeeded (D-53). Stable on
macOS, Windows and Linux.

## Decisions owed by the maintainer

| Decision | Holds up |
|---|---|
| Publishing the Claude Code plugin (D-43 point 6) | AJ-E1905 |
| Kimi Code's usage source | AJ-E1603 |
| The pools spike's policy rulings | AJ-E1701, AJ-E1705 |
| When public promotion starts | D-24's strict Beta rule; 0.6 |
| Windows: packaging, signing, scope | AJ-E2601, then 0.7 |
| Linux: formats, repository, support matrix | AJ-E2701, then 0.8 |
