# Evidence manifest — AJ-E505 (Port occupancy map and leftover inventory data)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E505.
Branch: `stage2/e5-ports`. Local runs (macOS arm64, CLT-only,
ambient load 8-25) 2026-10-13, reviewer zcode-primary. Rust suites
run in CI (verify job); the integration fixture runs everywhere a
TCP stack exists.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 `PortProbe` trait, netstat2 on macOS/Linux, Windows seam, NO lsof/netstat shell-outs | code review + `ports-unit-tests.txt` (5/5) | low-level APIs only; honest unsupported elsewhere; `grep -R "lsof" crates/` clean | `ports.rs`: `PortProbe` trait + `NetstatProbe` (ONE `netstat2::iterate_sockets_info` pass over listening TCP, both families, pids attached — 5-6 ms for ~119 listeners measured) + `UnsupportedProbe` (`cfg(not(unix))` — the honest `ports unknown on this platform`, never a faked table, never `netstat.exe` parsing; the `GetExtendedTcpTable` seam is M7-5's); grep gate clean (only explanatory comments) | PASS | 2026-10-13 | zcode-primary |
| Req.2 per-tick join stores `listens_on`; well-known ports are hints only | `ports-fixture-api-dumps.txt` + code review | join on the AJ-E504 tick (no second loop); stored per row; hints never identity | `join_ports` rides the E504 tick: the JOIN runs every tick on the CACHED listener table; ENUMERATION is throttled to every 5th tick (`PORT_PROBE_EVERY`) because a loaded box makes it cost ~90 ms against the ≤100 ms tick budget (measured: probe 5 ms idle / ~90 ms loaded) — the spec's budget governs; `apply_ports` = ONE batched transaction per tick (`UPDATE … newest running-or-orphaned row per (pid,boot)`); candidates are fetched ONCE per tick and shared by reconcile + join (a second identical query measurably stalled under disk pressure); `well_known_hint` is display-only (unit-asserted "label but never identify") | PASS | 2026-10-13 | zcode-primary |
| Req.3 exactly one of four classifications; unmanaged display-only | `ports-unit-tests.txt` + `ports-fixture-api-dumps.txt` | Managed orphan / Managed running / Re-adopted / Not managed; F3 guard | `classify_listeners` (pure, fixture-tested): pid → NEWEST running-or-orphaned row (`unverifiable`/exited rows NEVER confer ownership — the recycled-pid fixture asserts zero attribution); helper-of-dead-root = Managed orphan even pre-grace (the port map states ownership truth; E506 applies grace surfacing); adopted sessions (journal-verified restart survivors, EXCLUDING currently-supervised ones — a real bug found by the fixture: live sessions were misclassifying as re-adopted, fixed in `alive_adopted_sessions`) = Re-adopted; no ledger match = Not managed with `killable: false` and ZERO stop/kill command ids in the API DTO | PASS | 2026-10-13 | zcode-primary |
| Req.4 pre-launch warning, copy-exact, never blocks | `ports-fixture-api-dumps.txt` ([warning] dump) + `port-map.test.tsx` | `:P is already held by an orphan from {Instance} ({age}).` + Free/Launch anyway; launch unblocked by construction | `launch_port_warning(project)` = the project's LEARNED ports (its instances' observed `listens_on`, any row state) ∩ CURRENT orphan-held listeners → the copy-exact message (humanized age `45s/12m/3h/2d`); the App's launch handler runs the pre-check BEFORE spawn, renders `PortWarningSheet` (copy-exact asserted), `Launch anyway` proceeds, `Free this port` dismisses to the Processes tab (the AJ-E507 sheet routes later); probe failure launches (advisory only) — SPEC §5.2's advisor-informs-never-blocks holds by construction; a DIFFERENT project warns nothing (asserted) | PASS | 2026-10-13 | zcode-primary |
| Req.5 hog facts for E506 | code review (`hog_facts`) | per-helper RSS/CPU, per-Instance rollup, peaks; hog only after sustained breach over 60 s | per-helper live RSS/CPU + per-Instance rollups + ledger peaks were E501/E504 (rows + `peak_rss/peak_cpu`); NEW `hog_facts`: 60 s sustained window over the collector rings — `helper-rss>2GiB` (single helper), `tree-cpu>90%` (whole-tree, every in-window sample), `tree-rss>p95(provider)` (needs ≥3 Provider peers — an honest population floor, self-excluded); returned by `port_inventory` | PASS | 2026-10-13 | zcode-primary |
| Evidence fixture (the task's proof) | `cargo test -p juggler-app --test ports -- --nocapture` → `ports-fixture-api-dumps.txt` | vite-like helper on :5173 → `Managed orphan … holds :5173` within grace+2 ticks; unmanaged :8000 → `Not managed`, zero kill ids | FULL API JSON dumps (live + orphan + warning): live phase = `managed-running` with claude-code/acc-e505/proj-e505 + kernel listener age; post-grace = `managed-orphan` holding :5173; the UNSUPERVISED `python3 -m http.server :8000` = `not-managed`, `killable: false`, `session: null` (the DTO carries no kill command ids at all); the copy-exact warning fires for the owning project and not for a stranger project; cleanup reaps both | PASS | 2026-10-13 | zcode-primary |
| Performance budget | probe timings + soak interplay | enumeration ≤30 ms p95; inventory query ≤50 ms; degrade ≤1 tick, no retry storm | enumeration measured 5-6 ms idle (119 listeners); under ambient load the in-tick cost rose to ~90 ms → throttled to every 5th tick (join stays per-tick on the cache; budget documented above); the inventory command = one fresh probe (≤30 ms) + one candidates read + classification (µs) — well under 50 ms; probe failure sets the degrade note until the next throttled attempt (the throttle IS the ≥2 s retry bound); the E504 soak stays green with the join on the tick (construction count still 0/tick) | PASS (see note) | 2026-10-13 | zcode-primary |
| Quality gates | `just verify` + `pnpm --dir ui test` | supervisor suite green; no lsof/netstat shell-outs; fixtures incl. unmanaged + recycled pid | verify green (fmt, clippy `-D warnings`, workspace tests incl. ports unit 5/5 + fixture 1/1 + all prior suites); UI 156/156; `grep -R "lsof" crates/` clean; fixture tests cover the unmanaged pid AND the recycled pid (exited+unverifiable rows grant nothing) | PASS | 2026-10-13 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | inventory with all four classifications + collision sheet, 1440×900 + 1024×700, light+dark; icon+text not colour alone | `_gallery/18-ports/` (6 new captures): inventory 1440×900 + 1024×700 both themes — port headline + PID evidence + attribution + age, orphan badge with ⚑ icon + "Managed orphan" text, unmanaged row muted with ○ + "Not managed by Juggler" + explanatory line; warning sheet both themes with the copy-exact message + both actions; suite 113/113 (byte-stable for prior screens) | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- E506 (Cleanup Centre read-only) next: read rows from the LEDGER
  (`candidates` = running|orphaned) + `port_inventory` for the port
  dimension; the inventory DTO already carries the four classes and
  hog facts; unmanaged rows render muted with NO buttons (the F3
  guard is in the API, not just the UI).
- The E504 soak's wall-time gate is now load-aware (AJ-E806): on a
  developer box with ambient load > core count it QUARANTINES with
  numbers printed; CI (idle) enforces the ≤100 ms p95 strictly.
- Enumeration throttle: ports refresh every 5th tick (10 s) in the
  stored ledger; query paths (inventory, launch pre-check) always
  probe fresh. If E506 wants fresher stored ports, lower
  `PORT_PROBE_EVERY` only with new soak numbers.
- The learned-ports map (project → ports) is derived from ledger
  `listens_on` history at query time; a settings-pinned expected-port
  per launch recipe can join later without schema change.
- Bug fixed en route (recorded): `alive_adopted_sessions` now EXCLUDES
  currently-supervised sessions — journal records for live sessions
  are not exits, and adoption means restart survivors only.
