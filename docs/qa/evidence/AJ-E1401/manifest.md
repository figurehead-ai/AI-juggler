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

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 signals, Req.2 ring | `cargo test -p juggler-supervisor --test machine` (real OS: one System, real memory, CPU from tick 2, a volume) and `--lib machine` (ring keeps 150) | all pass | 2 + 14 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Req.3 summary rules | `cargo test -p juggler-supervisor --lib machine`: each reason alone (pressure warning/critical, memory 6 %/3 %, swap 120 MB/min, sustained CPU, thermal, disk 15 GB/1 GB), spike ≠ sustained, unmeasured CPU never busy, too-short history, missing values, worst-first order | all pass | 14 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Req.4 read model + mock | `cargo test -p juggler-app --lib machine_health` (unsampled, words/series/volumes); `pnpm --dir ui exec vitest run src/lib/machine.test.ts` | all pass | 2 + 3 passed | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Budget: ≤ 2 ms p95 tick, no System per tick | `cargo test -p juggler-supervisor --test machine -- --nocapture` (60 steady ticks, debug build, load average 21–43) | p95 < 2 ms; constructions = 1 | median 4 µs, p95 31–72 µs; 1 construction | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |
| Live reading (reference Mac) | `cargo test -p juggler-supervisor --test machine -- --ignored --nocapture` | real values | CPU 44.8 %; memory 103.1 GB total, 61.9 GB available; pressure normal; swap 22.5 of 23.6 GB used; Macintosh HD 58.9 GB free of 994.6 GB; on power; thermal nominal → "Plenty of room" | PASS | 63cf65d9+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
