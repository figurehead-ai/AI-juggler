# Gate V1 — Milestone M1 acceptance (M1-7)

Status: **MECHANICS VERIFIED (agent dogfood, 2026-10-12) — awaiting user confirmation**

## Agent-run dogfood evidence (mechanics, real claude binary)

- `dogfood_v1` (operational runner, repeated 3×): two imported accounts
  (client-d + personal-max) launched in parallel on
  `scratch/dogfood/project-x` through the full app stack — real
  `claude` 2.1.296 under our PTY, real "Welcome to Claude Code" UI
  streamed into both panels, graceful stop ladder, sessions recorded
  `stopped`, ONE project tracked with both accounts in history.
- Auth boundary confirmed honestly: copies are not authenticated
  (claude 2.1.296 consults only the namespaced per-path keychain entry;
  the t3 originals' shared legacy entry does not carry). Authenticated
  working sessions require the user's one-click re-login per account —
  the single remaining user step.
- Machine health after all runs: t3 originals manifest-verified (only
  live-session transcript/backup writes by t3 itself), `~/.claude`
  untouched, legacy keychain entry present, t3.app running its 3
  sessions, zero stray AI Juggler processes.
- Dogfood finding fixed: user-stopped sessions now record `stopped`
  (not `failed`) — PTY kills surface as plain exit codes, so supervisor
  stop-intent tracking feeds the status mapping.
Prepared: 2026-10-12

M1 scope: two (or more) Claude Code accounts running in parallel through
the real app, fully isolated, for a full working session each — the
founding pain point (EPICS.md M1).

## Dogfood environment (prepared, real)

The maintainer's machine carries **five t3.codes-managed Claude profiles**
under `~/Library/Application Support/ClaudeCodeProfiles/`. All five were
imported into AI Juggler as read-only copies (PR #37 operational runner):

| Account | Imported from | Size | Login state | Needs |
|---|---|---|---|---|
| client-a | t3.codes profile | 293 MB | logged out in copy | one guided re-login |
| client-b | t3.codes profile | 541 MB | logged out in copy | one guided re-login |
| client-c | t3.codes profile | 425 MB | logged out in copy | one guided re-login |
| client-d | t3.codes profile | 6.7 MB | logged out in copy | one guided re-login |
| personal-max | t3.codes profile | 340 MB | logged out in copy | one guided re-login |

Why re-login: the originals authenticate through the **shared pre-2.1.56
keychain entry** (`Claude Code-credentials`), which is path-locked and
shared — copies cannot (and should not) carry it. The wizard's login step
performs this per account (Sign in inside AI Juggler → embedded terminal →
Anthropic OAuth). **t3.codes originals were never modified** (manifest
evidence in PR #37) and its live sessions kept running throughout.

## Acceptance checklist

- [ ] Two accounts re-logged-in via the in-app guided login
- [ ] Both launched in parallel on the **same project** (juggling anchor):
      distinct `CLAUDE_CONFIG_DIR`s, separate transcripts, no cross-talk
- [ ] A full working session in each (prompt → edit → verify) through the
      embedded terminals
- [ ] Projects rail shows the directory once with both accounts in history;
      each session's panel embeds its terminal
- [ ] Session statuses transition running → finished in the rail
- [ ] `~/.claude` and the t3 originals remain untouched after the run
      (re-run the manifest check)
- [ ] Guardrail suite green: `just verify` (isolation, write containment,
      import read-only, scanner honesty)
- [ ] Quality audit #2 (code review pass over M1 diff)
- [ ] **User confirmation** — M1 closes

## Out of scope (tracked, not blockers)

- Quota dashboards / usage timelines (M2–M4)
- Hand-over between accounts (data lineage already recorded: project ↔
  session ↔ account)
- Desktop-class harness adapters (Claude.app on this machine is the Claude
  chat desktop app, not Claude Code — separate product; adapter research
  is its own milestone with a ToS check)

## Sign-off

Added by AJ-E807: a gate is signed here with the decision, the reviewer,
the date and the commit it approves (`scripts/gate_packet.py signed`
checks it). Until then this gate is open.

| Field | Value |
|---|---|
| Decision | UNSIGNED |
| Reviewer | |
| Date | |
| Commit | |
