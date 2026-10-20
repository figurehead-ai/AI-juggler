# Evidence manifest — AJ-E1403

Task: `docs/tasks/E14-machine-health.md` AJ-E1403 (the This Mac read
model). Status: **DONE-EVIDENCED** (backend and bridge; Home shows it in
AJ-E1404).

## What exists now

- `this_mac` command and the `machine://this-mac` event on the 5 s
  resource cadence. Like the resource tick, an idle app costs the UI nothing:
  with no AI tool running, it emits only when the machine's level changes.
- Payload: the machine (AJ-E1401); totals for all AI tools, split into ours
  and theirs (CPU as a share of the whole Mac, memory); the top three trees
  by memory; every heavy tree with its note; leftover processes and the
  ports they hold (the ledger's orphaned helpers).
- **Heavy**: above 90 % of one core, or above 2 GB, for every sample of the
  last minute; or flagged by an AJ-E512 hog rule. Note: "CPU 95%",
  "3.2 GB", or both.
- `ui/src/lib/machine.ts`: `thisMac()` and the event-driven `useThisMac()`
  hook. The preview derives This Mac from the sessions on screen plus one
  Claude app window (seams `__AIJ_THIS_MAC__`, `__AIJ_OBSERVED__`,
  `__AIJ_LEFTOVERS__`).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 totals, split, top 3 | `cargo test -p juggler-app --lib -- machine_health` (`this_mac_splits_ours_from_theirs_and_ranks_by_memory`, `an_unmeasured_tree_leaves_the_cpu_total_unknown_never_zero`) | pass | pass | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.2 heavy rule and note | `heavy_means_sustained_for_a_minute` (CPU, memory, both, 20 s spike ≠ heavy, hog rule alone) | pass | pass | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.3 ≤ 200 ms at 50 Instances | `cargo test -p juggler-app --test load_budgets this_mac -- --nocapture` (50 live supervised trees with helpers) | ≤ 200 ms | median 5.9 ms (runs 4.4–7.3 ms), 50 trees counted | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Bridge + preview mock | `pnpm --dir ui exec vitest run src/lib/machine.test.ts` | pass | 6 passed | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
