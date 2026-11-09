# AI Juggler — Roadmap

The versions ahead, what each is for, and what it holds. Locked in by
the maintainer on 2026-10-30 (D-59); 0.5 and 0.6 merged on 2026-11-02 (D-65). Each item names its task, so the
detail and the evidence live in [`docs/tasks/`](../tasks/README.md); this
page is the order and the reason.

## Where we are

| Version | Channel | What it brought |
|---|---|---|
| 0.1 | Beta | Accounts and profiles, sessions, switching A→B→Codex→A with history, tasks and context kept, honest usage, This Mac, onboarding |
| 0.2 | Beta | Switching to the account with the most room, one Needs you, notifications, pools with messages and tasks between agents, OpenCode, Grok Build and Kimi Code, Processes |
| 0.3 | Beta (2026-10-30) | Projects, Pools and Tasks in the header, the pool canvas, Subagents, the local cache (D-58), the redesign (E28) |
| 0.3.1–0.3.5 | Beta | Deleting launch settings keeps their sessions (D-60); Open starts a closed app, Keep alive (D-63); inactive apps in the App grouping (D-64); a switch continues in Desktop or the CLI; the conversation drawn as the Claude app draws it, growing live; lower CPU at rest |

Nothing is on Stable yet: the first Stable is a 0.4.x (D-85; it was a 0.5.x under D-65).

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
  commit, a fast low-load check on this Mac; every heavy gate runs on
  GitHub's runners, split across jobs, and a merge needs them all green
  ([QUALITY.md](./QUALITY.md) §7, ADR-0025).

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
| Account and App | One word per thing, and an Apps section beside Accounts (2026-11-01), and Restart for a running app after a confirm (D-71): an Account is a subscription, an App is an app or one of its profiles, several Apps share one Account. Both stored and named, each with its own page; every name on screen opens its page; Claude app profiles shown even when closed. Approved (D-62) | E32: AJ-E3201…AJ-E3206, AJ-E3208, AJ-E3209 |
| New session, redesigned | It opens with your usual folder and App chosen and said in one sentence; ⌘↩ starts it; changing either is one step away; rare options folded (D-62) | AJ-E3207 |
| Everything links to everything | Every page (project, session, App, Account, pool, Mac, Tool) leads to everything related, both ways, with counts; one list and one row everywhere; lists open on what needs you and what's active, with search and filters; every picker shows what fits where you are, then All. Approved (D-62) | E31: AJ-E3102…AJ-E3105 |
| Pools, redesigned from how people work | A pool covers every project or the ones you pick; members are a session, an App (its new sessions join by themselves), an Account, a Tool, a Mac or everyone; a session can be in several pools; Lead, Member, Reviewer and Observer; who may talk to whom; sealed pools; other apps' sessions shown as Watched with Continue in AI Juggler; a wizard with nine templates beside a canvas that teaches; today's pools moved with nothing lost. Approved (D-62) | AJ-E3001…AJ-E3009 |
| The Floor | The whole company as a live office in its own native window: every App an area with its tool's look and logo, every session a desk, subagents beside them, Accounts' usage and This Mac in the lobby; every visual a signal; turn, tilt and zoom (D-67; moved into 0.4 by D-68; ADR-0026, SPEC §9.7) | E35: AJ-E3501…AJ-E3514 |
| A task's session starts from a pointer | The first prompt names the task and the session reads the brief through the task tool; nothing private in the process list (D-72) | AJ-E1907 |
| Validated by real use | Every option combination that changes behaviour run against an independent reference; every real use case (writer and reviewer, feature team, across repositories, a reviewer for everything, rules by account, a build Mac, a sealed client, scouts, the upgrade, and the Account, App, links and New session journeys) walked by hand in the app, timed and scored; all of it again on the release candidate ([the plan](../qa/0.4-validation-plan.md)); and personas walking their day from first run, as newcomers, each step scored on whether it worked, gave value, was clear and felt good, so 0.4 is ready to announce | E33: AJ-E3301…AJ-E3307 |
| Proven on real tools | Codex on real accounts; the runtime lab on real tools; the switching-fidelity matrix; Fleet on two Macs | AJ-E1305, AJ-E2301, AJ-E808, AJ-E710 |
| An honest picture | The shipped work of E21, E23, E24, E25 and E28 evidenced and its statuses brought up to date; the README's roadmap matches this page | E21, E23, E24, E25, E28 |
| Releases that don't stall | A fast local preflight, the heavy gates on GitHub's free runners, release PRs merged as merge commits, deterministic tests | AJ-E3308 (#198's follow-up; ADR-0025) |

**Order inside 0.4** for the approved designs: the words and the stored
Account and App (E32), then the pages and links (E31), then pools (E30),
each validated as it's built (E33), not after. The Floor (E35) is built
alongside, in its own crates and window (ADR-0026), and validated the
same way.

**Not split** (2026-11-02): 0.4 ships whole and announcement ready; a release costs more time than building one.

**Completed in 0.4.x** (D-83, 2026-11-06): 0.4.0 reached beta without
its whole scope; every task above is finished in 0.4.x patch releases with
the maintainer's findings, before any 0.5 feature work. The audit and the
order: [0.4 completion](../qa/0.4-completion.md). `scripts/release_scope.py`
now fails a release PR, a new minor's nightly and every promotion while a
task named here is neither proven nor deferred by the maintainer.

**The first Stable** (D-85, 2026-11-06): no 0.5 feature work starts
until 0.4.x is fully tested, validated and fixed, and a 0.4.x becomes
the first Stable. It doesn't have to be perfect; its main functionality
has to work fully. As AJ-E809 describes, that 0.4.x is a release
candidate the maintainer has used, with their findings fixed. The Stable
channel opens with it (SPEC §15, open question 2), and the stricter Beta
rule (D-24's three days of use) applies from then on.

**Done when** every task above is evidenced, every scenario in the
[validation plan](../qa/0.4-validation-plan.md) passes on the release
candidate with nothing scored under 4, the full preflight passes, the
maintainer has used it, and its findings are fixed.

### 0.4.1 — the Floor from the maintainer's findings

Added after 0.4.0 from the maintainer's Floor findings. D-90
(2026-11-06): they ship in 0.4.1, and there is no 0.4.2; the release
check holds 0.4.1 to these rows, and the first Stable holds them all
(D-85).
D-93 (2026-11-08) adds Move to account, the default for a Claude → Claude
switch, to 0.4.1.

| Area | Tasks |
|---|---|
| The Floor reworked: labels by zoom, one floor at a time, hover cards, inactive Apps dimmed, the View menu, day and night, the reworked light | AJ-E3515…AJ-E3521 |
| Move to account: a Claude → Claude switch moves the whole family, journaled and undoable (D-93) | AJ-E212 |

## 0.5 — More tools and teams

**Intent.** Bring the tools people use next, let work move between
people as easily as between accounts, and make AI Juggler a macOS
release people can rely on. Planned work that starts only after the
first Stable, a 0.4.x (D-85); 0.5 ships to Beta first, then to Stable
under D-24.

| Area | Tasks |
|---|---|
| ZCode, Kiro CLI, Hermes Agent, Muse, Cursor CLI, Antigravity on work plans; the desktop apps, track-only | AJ-E1605…AJ-E1610, AJ-E1613 |
| Kimi Code's and Grok Build's usage finished | AJ-E1603, AJ-E1604 |
| Every new tool checked live on spare accounts | AJ-E1612 |
| The Claude Code plugin published | AJ-E1905 (waits on D-43 point 6) |
| Pools across your Macs, after the pools spike's rulings | AJ-E1701, AJ-E1705 |
| A context pack handed to a colleague | AJ-E1011 |
| The switch picker lists each App (desktop profile, CLI folder) as its own row with its own sign-in | AJ-E213 |
| Crash reports behind their own setting | AJ-E1107 |
| Pair review, model arbitrage and an outage day: falsified first, then built or dropped | AJ-E1006, AJ-E1008 |
| Reply and read: the conversation drawn as richly as the tools' apps (images, links, diffs, output, plans, thinking, tokens), find in it, one composer for sessions AI Juggler runs, and where a desktop app holds a session, Go to app and respond or Continue here as CLI (D-66) | AJ-E3401…AJ-E3405 |
| A pool line's own talk rule set from the member before its first message (the role pair covers it until then) | AJ-E3010 |
| The 10-account, 50-session load proof; a cold start budget on the release build | AJ-E805 |
| Updates and release integrity finished | AJ-E1010 |
| The gate packet signed | AJ-E807 |

Accessibility stays proven by the automated gates on every release
(axe on every gallery state, keyboard traversals, contrast); the manual
VoiceOver pass per release is dropped (D-65).

A tool added in 0.5 that hasn't been proven live when 0.5 reaches
Stable is marked as a preview inside it, so it never holds the Stable
back.

## 0.6 — AI Juggler for Windows

A dedicated first release, then Windows in every release on the same
version and channels (D-52, [E26](../tasks/E26-windows-release.md)).
Packaging, signing and scope are the maintainer's decisions first
(AJ-E2601).

## 0.7 — AI Juggler for Linux

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
| Kimi Code's usage source | AJ-E1603 |
| Windows: packaging, signing, scope | AJ-E2601, then 0.6 |
| Linux: formats, repository, support matrix | AJ-E2701, then 0.7 |
