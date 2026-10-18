# ADR-0010: Spawn ledger and verified process ownership

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-08)
**Implements:** D-08 (and SPEC §3.2, §5.2)

## Context

Verified failure mode: Instances exit and leave dev servers, MCP
transports, watchers and language servers running for days (ports +
RAM held). The original product invariant said *all* automation is
passive — but the maintainer's workflow explicitly asks for automatic
and manual cleanups "all with verifications"
(docs/expected-worfklow.md:18). D-08 is the narrow, written amendment.

## Decision

1. **Spawn ledger is the ownership authority**: intent recorded before
   spawn; row = pid, pgid, machine, instance, provider, account, argv
   hash, cwd, started_at, exit, reaped. Not-in-ledger ⇒ never touched
   (**fail closed**, AJ-E503).
2. **ProcessIdentity verify-before-kill** at click time (pid + pgid +
   start-time + argv hash re-verified; full command line, cwd, ports
   shown), then a staged TERM→grace→KILL ladder with per-process
   outcomes and port re-probe.
3. **Tiers, per Account (not per Provider):** Tier 0 notify-only
   (default); Tier 1 "Clean safe (this Account)" opt-in — automatic
   kill of *verified-owned* leftovers only, every kill audit-logged.
   Manual verified cleanup always available and always audited.
4. Preserved: no SIGSTOP; unowned processes untouchable in every tier;
   every ledger row carries MachineId (fleet-shaped, D-13).

## Alternatives considered

- **Per-Provider tiers** — rejected (D-08 wording is per-Account; a
  blanket Provider kill risks exactly the wrong-account failure).
- **Pure notify-only forever** — rejected by the maintainer's explicit
  ask; the amendment keeps the default passive anyway.
- **ppid-walk as ownership source** — rejected: racy after
  intermediate exits; the ledger is written with intent before spawn.

## Consequences

The §1 automation invariant now carries exactly two recorded
amendments (this and D-14's user-owned policy). E5 builds the ledger,
reconciler, and Cleanup Centre; zero-false-kill is a beta-exit metric.

## Amendment (AJ-E508 — the operating tiers, per pass2 §10 as settled
by D-08)

Accepted 2026-10-13 with the tier implementation (same change; this
ADR and the SPEC §5.2 wording land before any tier code merges).

1. **Tier 0 — Observe (default everywhere).** Detect, list, alert,
   pre-stage. Never signals. The Cleanup Centre plus the manual
   verify-before-kill sheet IS Tier 0's full surface.
2. **Tier 1 — Tidy verified orphans (opt-in per Account, off by
   default; exact label `Automatically tidy left-behind dev
   servers`).** After `orphan_grace` and a same-tick identity
   re-verification, TERM→KILL through the AJ-E507 ladder — ONLY
   helpers whose root exited cleanly (code 0) and whose role guess is
   `DevServer|Watcher`. Audit intent before the signal (AJ-E509).
3. **Tier 2 — Tidy after crash (a SEPARATE per-Account opt-in, off by
   default; exact label `Also tidy after a crash (with
   notification)`).** As Tier 1 but for crash-exited roots, only
   after 5 minutes AND one user-visible notification with an Undo
   window of ≥30 s before any signal. The Undo window, not a settings
   page, is the consent surface at crash time.
4. **Tier 3 — Hog shed.** NO automatic implementation exists or may
   be added without a new decision-log entry: alert + the staged
   sheet only.
5. **Hard exclusions at EVERY tier:** roots; role guesses
   `McpServer`, `TerminalShell`, any database-role guess; re-adopted
   (T3) survivors; unverifiable (T5) rows; escaped-group helpers;
   processes of non-opted-in Accounts; any process on another
   `machine_id`. The eligible role set (`DevServer|Watcher`) may not
   be widened without a new decision-log entry — role is a GUESS
   (pass2 C4), which is exactly why only these two are ever
   auto-eligible.
6. **Demotion, not escalation:** if same-tick re-verification fails
   for an automatic candidate, it is silently demoted to Tier 0
   (list-only) with an audit note — never signalled on stale
   evidence.
7. **Global pause kill-switch** (`Pause automatic cleanup`, in
   Settings and the Cleanup Centre header): takes effect within one
   tick; a persistent banner names it while active. **Crash
   suspension:** after any Juggler crash recovery, Tiers 1-2 are
   suspended for 24 h until the user re-confirms in the Cleanup
   Centre — shown, never silent.
8. **Storage shape:** per-Account `cleanup_settings`
   (`tier1_opt_in`, `tier2_opt_in` — never one bundled toggle). The
   E410 prototype's single `accounts.cleanup_opt_in` migrates to
   Tier 1 only (its promise was tidy-after-exit); Tier 2 starts off
   for everyone, including previous opt-ins.
