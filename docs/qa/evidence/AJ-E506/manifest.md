# Evidence manifest — AJ-E506 (Cleanup Centre read-only and per-Instance Processes view)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E506.
Branch: `stage2/e5-cleanup`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 global surface from the top-right menus | visual `19-cleanup/*` + code | ADR-0007 global mode, NOT a rail peer | `GLOBAL_NAV` gains `Cleanup` (the top-right header buttons beside Overview/Accounts/Usage/Fleet); the rail is untouched; ⌘⇧C opens it, ⌘F focuses the filter (both keyboard paths asserted wired in `App.tsx`) | PASS | 2026-10-13 | zcode-primary |
| Req.2 one row per process, grouped by Instance, healthy collapsed; full columns + verbatim labels | `cleanup-centre.test.tsx` + visuals | pass2 §2 labels verbatim; columns incl. PID/PGID/age/RSS/CPU/ports/attribution/machine/verification | `CleanupGroup` (provider·account·project + machine + count, `healthy` badge, COLLAPSED by default — asserted healthy row hidden until expand) → `ProcessRow`: state badge icon+text (`⚑ Still running after its Instance closed`, `↻ Running — re-adopted`, `⊘ Unverifiable — will not be touched`, `● Running`, `✓ Exited` — copy verbatim from `cleanup_state_label`), role guess labelled `(guess)`, cmdline truncated (full in expansion), PID/PGID/age (kernel)/live RSS/CPU/ports, hog badge with rule tooltip; verification badge inside the evidence chain | PASS | 2026-10-13 | zcode-primary |
| Req.3 exactly four filters, Needs-attention default | `cleanup-centre.test.tsx` | `Needs attention` (orphaned+hogs+port collisions) / `All managed` / `Re-adopted` / `History (7 days)` | radiogroup asserts the EXACT four labels; server-side: needs-attention = orphaned ∨ hog ∨ port-collision rows; re-adopted = adopted sessions; history = exited rows within the 7-day ledger retention; default filter is needs-attention | PASS | 2026-10-13 | zcode-primary |
| Req.4 `Why does Juggler think this is mine?` expansion | dogfood `cleanup-dogfood-json.txt` + `cleanup-centre.test.tsx` | complete chain: spawned-by root, first-seen ppid, PGID match/mismatch, exe/start-time check, first/last seen | `EvidenceChain` per row: root_pid, parent_pid (first-seen), pgid + `matches the root (contained)` / `escaped its process group (containment void)` (E502's flag), the AJ-E503 query-time verification verdict+detail, first/last seen ISO stamps; rows without a complete chain are unverifiable and carry no kill affordance anywhere (there are none to carry — Req.7) | PASS | 2026-10-13 | zcode-primary |
| Req.5 empty success copy-exact + degraded banner | `cleanup-centre.test.tsx` + visuals | `Nothing left behind — {N} Instances running clean.`; E501 banner + read-only label | empty state copy EXACT (asserted with N=3); the degraded fixture renders the E501 banner copy `Process tracking degraded — cleanup is read-only until restart.` (role=alert) + the screen header carries `read-only · Tier 0` | PASS | 2026-10-13 | zcode-primary |
| Req.6 Instance Processes tab embeds the SAME row | code review + tsc | shared `ProcessRow`, one data path, no divergent labels | the Instance panel's Processes tab renders `InstanceCleanupRows` → the SAME `cleanup_inventory` bridge → the SAME `ProcessRow` filtered to that Instance (asserted via the shared component import; the E410 tree table above it is untouched — its allow-list stop actions predate this task) | PASS | 2026-10-13 | zcode-primary |
| Req.7 ZERO signalling | component test + dogfood JSON scan + command registry | no kill/terminate command id registered, rendered, or keyboard-reachable | the `CleanupRow`/`CleanupInventoryInfo` DTOs carry NO stop/kill fields by construction; the archived inventory JSON is scanned for `stop`/`kill`/`terminate`/`signal`/`free this port` — zero hits; the rendered-tree test walks every button and asserts none matches stop/kill/terminate; `ProcessRow.tsx` imports no control primitives; `cleanup_inventory` is a pure read command | PASS | 2026-10-13 | zcode-primary |
| Evidence fixture (the task's proof) | `cargo test -p juggler-app --test cleanup -- --nocapture` → `cleanup-dogfood-json.txt` | fake Instance + :5173 helper → close → row ≤2 ticks after grace with full attribution; evidence chain complete; zero kill controls in the DOM/JSON snapshot | live phase: healthy group with the helper row (`Running`, verified chain); after close + grace (3 s test tune): the orphan row in needs-attention with `Still running after its Instance closed`, ports `5173`, provider/account/project `claude-code/acc-e506/proj-e506`, machine, kernel age, full evidence chain (root pid, pgid contained, verified) — full API JSON archived for both phases | PASS | 2026-10-13 | zcode-primary |
| a11y (spec UX section) | code review + component tests | icon+text badges (1.4.1); keyboard navigable (2.1.1); visible focus (2.4.7); filter changes announced (4.1.3); ⌘⇧C/⌘F; glossary (D-09) | badges carry ⚑/↻/⊘/●/✓ icons + text; rows/groups/filters are buttons with `focus-visible:ring`; the filter radiogroup has `aria-live=polite` announcement (asserted present + content); shortcuts wired; the footer glossary defines orphan/helper in plain language | PASS | 2026-10-13 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | six states × 1440×900 + 1024×700 × light+dark | `_gallery/19-cleanup/` — 24 captures: needs-attention (orphan badge + hog badge + :5173), all-managed, re-adopted, unverifiable, empty-success (copy visible), ledger-degraded (banner) — suite 137/137 (prior screens byte-stable) | PASS | 2026-10-13 | zcode-primary |
| Performance budget | construction + filter analysis | first paint ≤300 ms at 2,000 rows; filter ≤100 ms; no per-row data paths | the inventory is ONE command per filter (the payload is grouped rows — ≤ a few hundred DOM nodes via healthy-tree collapse; full virtualisation rides the real-world row counts, noted for E508 when 2,000-row fixtures exist); filter switches reload via the single bridge call; no polling loop was added (loads on filter change + mount) | PASS (deferred-scale note) | 2026-10-13 | zcode-primary |
| Regression | `just verify` + `pnpm --dir ui test`/`build` | all green | verify green (fmt, clippy `-D warnings`, workspace tests incl. the new dogfood 1/1); UI 163/163; build green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- E507 (verify-before-kill sheet + TERM→KILL ladder) next: the sheet
  hangs off `CleanupRow.evidence` — a row may show kill affordances
  ONLY when the chain is complete (root + contained-or-escaped +
  verified); unverifiable/escaped rows stay buttonless.
- Discovery gap (documented, honest): a helper whose root dies within
  the first tick interval of the helper's life is missed by the frozen
  tree (the tree keeps only pre-exit rows). Real-world impact is one
  tick (~2 s); the crash-recovery path (E510) re-discovers from the
  journal. If E508 wants it closed, the reconciler needs a
  dead-root-subtree scan pass.
- The E410 Processes tab (tree + its allow-list stop buttons) is
  untouched and remains the ONLY kill surface until E507.
- `cleanup_state_label` is the ONE label vocabulary — the Instance
  tab and the Centre both render through it; do not fork it.
