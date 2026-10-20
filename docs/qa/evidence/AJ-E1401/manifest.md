# Evidence manifest — AJ-E1401

Task: `docs/tasks/E14-machine-health.md` AJ-E1401 (machine signals: CPU,
memory and pressure, swap, disks in use, power and thermal). Status:
**DONE-EVIDENCED** for the backend and bridge; the surfaces that show it are
AJ-E1404/E1405/E1407.

## What exists now

- `crates/juggler-supervisor/src/machine.rs`: `MachineSampler` on the 2 s
  processes tick — whole-Mac CPU (from the second tick), memory total and
  available, the kernel's pressure level (`kern.memorystatus_vm_pressure_level`
  via the `sysctl` crate), swap total/used every tick; free space on the
  disks in use every 30 s (the disk list itself every 10 min); power
  (battery or adapter, minutes left, low-battery warning, Low Power Mode via
  IOKit's safe `IOPSGetTimeRemainingEstimate`/`IOPSGetBatteryWarningLevel`)
  and thermal state (`NSProcessInfo.thermalState`) every 60 s. 5-minute ring.
  No `unsafe` (the workspace forbids it).
- `summarize()`: the one summary — "Plenty of room" / "Getting tight" /
  "Under pressure" with reasons in words, worst first.
- `crates/juggler-app/src/machine_health.rs`: the read model and the
  `machine_health` command; `ui/src/lib/machine.ts`: the bridge call and the
  preview mock (`window.__AIJ_MACHINE__` = plenty | tight | pressure |
  unsampled).

## Decisions taken while building

- **Disk room is absolute, not a share.** The spec draft said "< 5 % or
  < 10 GB"; on the reference Mac (1 TB, 59 GB free = 5.9 %) a share rule
  would have flagged a comfortable disk. Now: tight under 20 GB free,
  pressure under 5 GB (requirement text updated).
- **Swap used is not a reason; swap growth is.** The reference Mac carries
  22.5 GB of swap at normal pressure — old swap. Thrash = swap growing
  > 100 MB a minute (tight) or > 500 MB a minute (pressure) over the last
  minute, needing ≥ 30 s of history.
- **The disk list is slow, a disk's free space is not.** `Disks::refresh`
  re-lists every volume (47–150 ms measured); refreshing one listed disk's
  storage costs ~1 µs. The sampler lists every 10 minutes and refreshes only
  the disks in use every 30 s.

## Review fixes (coordinating session's review of PR #67, 2026-10-20)

1. **No main-thread stall.** The sampler's `tick` now does only the cheap
   part under its lock: CPU, memory, pressure, swap, and power/thermal on
   their cadence. When disks are due it hands back a `DiskWork`. The folders
   in use (now one SQL query for sessions that haven't ended,
   `SessionsRepo::active_cwds`, instead of the whole history) and the disk
   listing (50–150 ms) run outside the lock, and the result is put back.
   The `machine_health` command is async (`spawn_blocking`), so it never
   runs on the main thread. Measured: under the lock p95 38–114 µs, max
   208–304 µs (was up to 150 ms whenever the list was rebuilt).
2. **"Memory used" is app + wired + compressed** (see the re-review below for the exact label). That is app memory
   (anonymous pages minus purgeable) plus wired plus what the compressor
   occupies, read through sysctl (`vm.page_pageable_internal_count`,
   `vm.page_purgeable_count`, `vm.page_wired_count`,
   `vm.compressor_bytes_used`, `hw.pagesize`). On the reference Mac it is
   82.3 GB of 103.1 GB, where total − available said 54.0 GB.
   `sysinfo`'s total − available stays as the fallback on other OSes. The
   preview mock now uses a realistic 22 of 32 GB.
3. **A disk is never credited to the wrong volume.** A folder under
   `/Volumes/` that matches only "/" (a disk plugged in since the list was
   built, or a network share `sysinfo` doesn't list) triggers an early
   rebuild of the list, at most every 5 minutes. If it still matches only
   "/", it is left out rather than counted as Macintosh HD.
4. **The macOS-only readings now run in CI:** the `build-macos` job runs
   `cargo test -p juggler-supervisor --lib machine` and `--test machine`.
5. **Disk threshold, kept absolute (20 GB tight, 5 GB pressure).**
   - A share would be wrong both ways: 5 % of a 1 TB disk (50 GB) is
     comfortable, and a proportional rule such as min(20 GB, 10 %) changes
     nothing for a 256 GB Mac (10 % = 25.6 GB). It would only relax small
     disks, where 15 GB free is genuinely tight for development builds
     (one Rust `target/` is 10–20 GB).
   - So a 256 GB Mac kept under 20 GB free does read "Getting tight". That
     is the calm level: an amber word in This Mac, never Needs you, never a
     notification. Disk alerts (AJ-E1408) fire only at the pressure level
     (under 5 GB).

## Re-review fixes (2026-10-20, at 4d481d8d)

- **An honest label for memory.** `vm.compressor_bytes_used` is the
  compressor's *bytes*. Activity Monitor counts the *pages* the compressor
  occupies, which only `host_statistics64` reports, and that call needs
  `unsafe` FFI, which the workspace forbids. So the figure is named for what
  it is: `memory_basis = app_wired_compressed`, "close to Activity Monitor's
  Memory Used, a few percent lower" (≈ 5 % on the reviewer's Mac). Off macOS
  the basis is `total_minus_available`. If any part of the macOS reading
  fails, `memory_used` is `null` and nothing is shown, never the other
  measure under the same name. A partial sum is never taken either: every
  part is required.
- **Free memory on the same basis.** Free is total − used, so used + free =
  total; "only X of memory free" uses it. The OS's own "available" figure is
  used only when used couldn't be read. The mock is now 22 + 10 = 32 GB.
- **This Mac's folders only.** `active_cwds(machine)` filters to this Mac,
  so a session synced from another Mac with a `/Volumes/` path can't force
  rebuilds.
- **Stale disk work dropped.** A disk generation counter: work from an older
  hand-out never overwrites newer volumes (unit test).
- **Tests.**
  - `machine_lock.rs` holds storage for 800 ms while the first tick (a disk
    tick that also builds the disk list) waits on it. `try_lock` on the
    machine lock succeeds the whole time, then the volumes land.
  - `an_unknown_volume_rebuilds_the_list_at_most_every_five_minutes` gives 1
    list in 160 ticks and 2 by tick 170.
  - The macOS host test checks the parts: app, wired and compressed are each
    ≤ total, used ≤ total, and used is within 2 % of the parts' sum. The
    sampler test checks used + free = total.
- Small: the stale lock comment in `state.rs` is fixed, and `is_active` is
  private again.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Review fix 1 — lock held briefly | `cargo test -p juggler-supervisor --test machine -- --nocapture` (lock-held time per tick, disk half done outside) | p95 < 2 ms, max < 20 ms | p95 38–114 µs, max 208–304 µs (3 runs) | PASS | (see PR) | 2026-10-20 | claude-opus |
| Review fixes 2–3 | `cargo test -p juggler-supervisor --lib machine` (`an_unlisted_disk_is_never_credited_to_the_startup_disk`, host `memory_used`), `cargo test -p juggler-app --lib -- machine_health` (`memory_used_is_activity_monitors_figure…`), `cargo test -p juggler-core --test storage_roundtrip active_cwds` | pass | pass; live: memory used 82.3 GB of 103.1 GB | PASS | (see PR) | 2026-10-20 | claude-opus |
| Req.1 signals, Req.2 ring | `cargo test -p juggler-supervisor --test machine` (real OS: one System, real memory, CPU from tick 2, a volume) and `--lib machine` (ring keeps 150) | all pass | 2 + 14 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Req.3 summary rules | `cargo test -p juggler-supervisor --lib machine`: each reason alone (pressure warning/critical, memory 6 %/3 %, swap 120 MB/min, sustained CPU, thermal, disk 15 GB/1 GB), spike ≠ sustained, unmeasured CPU never busy, too-short history, missing values, worst-first order | all pass | 14 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Req.4 read model + mock | `cargo test -p juggler-app --lib machine_health` (unsampled, words/series/volumes); `pnpm --dir ui exec vitest run src/lib/machine.test.ts` | all pass | 2 + 3 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Budget: ≤ 2 ms p95 tick, no System per tick | `cargo test -p juggler-supervisor --test machine -- --nocapture` (60 steady ticks, debug build, load average 21–43) | p95 < 2 ms; constructions = 1 | median 4 µs, p95 31–72 µs; 1 construction | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Live reading (reference Mac) | `cargo test -p juggler-supervisor --test machine -- --ignored --nocapture` | real values | CPU 44.8 %; memory 103.1 GB total, 61.9 GB available; pressure normal; swap 22.5 of 23.6 GB used; Macintosh HD 58.9 GB free of 994.6 GB; on power; thermal nominal → "Plenty of room" | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
