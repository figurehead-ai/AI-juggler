# Gate E5 — Process ownership & cleanup epic acceptance

Status: **AGENT-VERIFIED 2026-10-13 (fixtures + soak + real-claude
mechanics dogfood) — two honest open items below; no "done" claim
until they close**

The gate law (AJ-E512 Req.4): no gate, no "done" claim. This report
collects the epic's evidence, the collateral-suite result, the CPU
soak, and the live-dogfood record.

## 1. The epic's success metric — zero false cleanup kills (D-01)

**Collateral suite** (adversarial fixtures with per-fixture signal
trap logs; every assertion is on the trap log, never on UI state):

- `cargo test -p juggler-supervisor --test collateral` — the ladder's
  signal GEOMETRY: `stop()` TERMs exactly the session's OWN group,
  KILLs only survivors after grace; the `setsid` escapee, a
  neighbour session's tree, and an unmanaged user server receive ZERO
  signals; the TERM-ignorer falls only to the post-grace KILL; the
  port-holder's port is verifiably free afterwards.
  Result: **1/1 PASS** (`evidence/AJ-E512/collateral-supervisor.txt`).
- `cargo test -p juggler-app --test guardrails` — EVERY kill path ×
  the full fixture matrix (vite-like port holder, TERM-ignorer,
  `setsid`-escapee, MCP-role holder, shared-PGID shell sibling,
  unmanaged user server, PID-reuse stranger):
  - Tier 1 sweep: only the verified DevServer orphan is TERMed;
    MCP/shell/ignorer/escapee/neighbour/unmanaged/stranger receive
    ZERO signals.
  - Tier 2 crash flow (5 min → ONE notice → lapsed Undo): same
    zero-signal law.
  - Manual ladder, pid scope: ONE process; the stranger is REFUSED
    (`This PID now belongs to a different program…`) with zero
    signals and NO audit row.
  - Manual ladder, pgid scope (the E507-law exception — explicit,
    warned, user-confirmed): the sheet NAMES every shell/MCP member
    (`Includes a {role} that may hold unsaved state.`) before the
    group TERM; everything OUTSIDE the confirmed group (escapee,
    neighbour, unmanaged, stranger) still receives zero signals.
  - Crash recovery (AJ-E510): recovery itself signals NOTHING; the
    adopted stop is scoped to the verified group.
  - TERM-ignorer as the intended target: TERM logged, grace survived,
    KILL after it (`needed KILL`), collateral untouched.
  - Hog law (Req.3): a hog-flagged DevServer orphan is NEVER
    auto-signalled (the sweep stands down), the alert carries the
    exact template, and the manual sheet remains the only path.
  Result: **8/8 PASS** (`evidence/AJ-E512/guardrails-app.txt`).
- CI: the suites are required gates — Linux via the `verify` job
  (`cargo test --workspace`), macOS via an explicit
  `Collateral suite (AJ-E512)` step in `build-macos`.

## 2. Task evidence (AJ-E501 → E511)

| Task | Evidence | Status |
|---|---|---|
| AJ-E501 spawn ledger | `docs/qa/evidence/AJ-E501/manifest.md` | DONE-EVIDENCED |
| AJ-E502 PGID ownership | `docs/qa/evidence/AJ-E502/manifest.md` | DONE-EVIDENCED |
| AJ-E503 ProcessIdentity verify | `docs/qa/evidence/AJ-E503/manifest.md` | DONE-EVIDENCED |
| AJ-E504 reconciler + orphans | `docs/qa/evidence/AJ-E504/manifest.md` | DONE-EVIDENCED |
| AJ-E505 ports + hogs flags | `docs/qa/evidence/AJ-E505/manifest.md` | DONE-EVIDENCED |
| AJ-E506 Cleanup Centre | `docs/qa/evidence/AJ-E506/manifest.md` | DONE-EVIDENCED |
| AJ-E507 verify-before-kill sheet | `docs/qa/evidence/AJ-E507/manifest.md` | DONE-EVIDENCED |
| AJ-E508 tiers 0-3 | `docs/qa/evidence/AJ-E508/manifest.md` | DONE-EVIDENCED |
| AJ-E509 audit log | `docs/qa/evidence/AJ-E509/manifest.md` | DONE-EVIDENCED |
| AJ-E510 crash recovery | `docs/qa/evidence/AJ-E510/manifest.md` | DONE-EVIDENCED |
| AJ-E512 this gate | `docs/qa/evidence/AJ-E512/manifest.md` | DONE-EVIDENCED |
| AJ-E511 fleet identity | — | **TODO (P1, next)** |

## 3. Reconciler CPU at 50 Instances (budget <1%)

`fifty_instance_soak_and_construction_regression`
(`evidence/AJ-E512/cpu-soak.txt`), developer box at loadavg 9.3:

```text
[soak] instances=50 ticks=20 avg_tick=15ms p95=37ms max=38ms
       system_constructions=0
[soak] self cpu over soak: 0.03%
```

0.03% self CPU at 2 s tick cadence — 30× inside the budget, measured
ON A LOADED MACHINE (the honest case). Cleanup Centre first paint
rides the E506 budget (re-measured by its suite; the alerts region
adds one bridge read, ≤1 row per hog session).

## 4. Live dogfood record

`dogfood_e5` (operational, `evidence/AJ-E512/dogfood-claude.txt`):
a REAL Claude Code 2.1.300 instance under the real
supervisor/PTY/app stack with a throwaway `CLAUDE_CONFIG_DIR`:

- real UI streamed (the unauthenticated login screen — gate-M1's
  honest auth boundary), real root in the durable ledger with
  attribution (`claude-code`, the real binary path);
- `kill_verify` → verified; the E507 ladder → `exited on TERM`
  (code 143), `all_succeeded`, audit row present, zero survivors.

**Open item 1 (auth)**: an *authenticated* working session spawning
real helpers (dev server via a prompt, MCP servers) requires the
maintainer's one-click re-login in the gate-M1 prepared environment —
the same standing user step gate-M1 records. The helper SHAPE is
meanwhile proven against real OS processes by the collateral suite
(trap logs), and the real-root verified-cleanup path by this dogfood.
No credential is ever automated or read (ADR-0003).

**Open item 2 (E511)**: fleet identity (P1) is the epic's remaining
task; its WrongMachine guardrail is intentionally NOT counted in
this gate's zero-signal matrix (foreign-machine rows are excluded
from candidates by machine scope today).

## 5. Sign-off

- [x] Collateral suite green (macOS local; CI both platforms)
- [x] CPU soak <1% at 50 Instances
- [x] Hog alerts passive-only; staged sheet is the only path
- [x] Real-claude mechanics dogfood archived
- [ ] Authenticated-helper dogfood (maintainer re-login)
- [ ] AJ-E511 DONE-EVIDENCED

Prepared: 2026-10-13 (zcode-primary)
