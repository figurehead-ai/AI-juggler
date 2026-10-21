# Evidence manifest — AJ-E1408

Task: `docs/tasks/E14-machine-health.md` AJ-E1408 (runtime overload alerts).
Status: **DONE-EVIDENCED**.

## What exists now

`crates/juggler-app/src/machine_alerts.rs` holds the alert state machine. It
is evaluated with This Mac on the 5 s cadence, so it adds no work of its own.

| Kind | Fires when | Hold |
|---|---|---|
| memory_pressure | macOS pressure warning/critical | 30 s |
| swap | swap growing > 200 MB a minute (its rate is over the last minute) | +60 s (≈ 2 min) |
| cpu | whole-Mac CPU > 90 % for every sample of 2 minutes | — |
| thermal | thermal serious/critical | — |
| disk | a volume in use under 5 GB free (the pressure level; under 20 GB is a calm "Getting tight" in This Mac, never an alert — review of #67) | — |
| heavy | a session's heavy rule (already a minute long) | — |

- Each alert names its biggest contributor: memory for memory, swap and
  disk; CPU for cpu and heat. A tool AI Juggler didn't start is named as
  "The Claude app" / "claude in a terminal". Those tools are never alerted
  on as heavy themselves; This Mac shows them.
- Told once per kind per 30 minutes, after holding; cleared when it clears;
  active alerts stay in Home's Needs you, amber when tight and red when
  under pressure. The notice region drains newly told alerts on its
  existing 30 s poll. A drained notice lives outside the component, so it
  survives switching screens until dismissed. Found walking it: React's dev
  double-mount drained it into an unmounted component.
- Passive only: no stop, pause or launch path. Like a quota wall, alerts
  are immediate; quiet hours only delay the hourly digest. Usage stats count
  them as `ui.notification` kind `other`, because the closed schema has no
  machine kind. Adding one is a schema change for the maintainer.
- After the AJ-E1408 review:
  - An alert clears only after its condition has been gone for 45 s. A
    swap or CPU alert stays on above a lower level (100 MB a minute; 80 %
    over 30 s).
  - Each key, and each kind, is told at most once per 30 minutes. A
    track that's off is forgotten after its 30 minutes, ended sessions'
    `heavy:` keys included.
  - Memory blame prefers measured memory. A tool AI Juggler didn't start
    is named only when it's still the largest at half its sum.
  - An alert tells only the figures that are high: a CPU-heavy session is
    told by its CPU.
  - A notice goes once This Mac no longer has its alert (after 10 s to
    settle). Notices queued while hidden collapse to the newest per kind.
- After AJ-E1402's review (memory honesty), an alert naming a tool AI
  Juggler didn't start carries `subject_estimate`. The words then say
  "about 2.0 GB", because that tree's sum counts shared memory more than
  once.

## Fixture corpus (SPEC §5.3, §12)

- Rules, on fixture This Mac snapshots: `cargo test -p juggler-app --lib --
  machine_alerts`. A calm Mac tells nothing for 10 minutes. Pressure is
  told once after 30 s (a 25 s blip is not) and names the biggest. It clears
  and is not retold within 30 minutes, though it stays active in Needs you,
  and is told again after. 150 MB/min of swap is not thrash; 240 is, after a
  minute. Busy CPU needs a full 2 minutes, and its contributor is the
  busiest. Heat, disk and heavy sessions fire at once, and tools AI Juggler
  didn't start are not alerted. Nothing fires before the first sample.
- Real process trees: memory-hog children (`machine_fixtures.rs`) are
  measured by the collector; the heavy rule on fixture rings is in
  AJ-E1403's tests.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1, Req.4 rules on the corpus | `cargo test -p juggler-app --lib -- machine_alerts` | every alert fires on its fixture, none on calm | 7 passed | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Req.2 once per 30 min, clears, names the contributor | same (`it_clears_and_is_not_retold_within_thirty_minutes`, `memory_pressure_is_told_once…`) | pass | pass | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Req.3 Needs you + the notice | `pnpm --dir ui exec vitest run src/components/this-mac.test.tsx` (alert in Needs you with "busy-one is using 3.2 GB", amber when tight; the notice told once and dismissed) | pass | 12 passed | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Memory honesty (AJ-E1402 review) | `cargo test -p juggler-app --lib -- machine_alerts` (`a_tool_we_did_not_start_is_named_with_its_memory_as_an_estimate`); vitest "says a tool we didn't start uses about its memory…" | pass | pass | PASS | be61a592+ | 2026-10-20 | claude-opus |
| No flapping, per-kind limit, blame (review) | `cargo test -p juggler-app --lib -- machine_alerts`: `a_flicker_neither_clears_nor_retells_it` (fails on the old code: one calm sample cleared it), `busy_cpu_stays_on_while_it_hovers_above_eighty`, `a_session_dipping_in_and_out_is_told_once_per_kind` (tracks forgotten after 30 min), `a_tool_we_did_not_start_is_blamed_only_when_clearly_the_largest`, `a_cpu_heavy_session_is_told_by_its_cpu_not_its_memory` | pass | 13 passed | PASS | 4a03078c | 2026-10-20 | claude-opus |
| Stale notices (review) | `this-mac.test.tsx`: "keeps the newest notice per kind, and lets one go once its condition ends", "clears a told notice when This Mac no longer has its alert", "tells a CPU-heavy session by its CPU, not its memory" | pass | pass | PASS | 4a03078c | 2026-10-20 | claude-opus |
| Gallery + axe | `pnpm --dir ui exec playwright test tests/visual/overload-gallery.spec.ts` → [`_gallery/74-overload/`](docs/qa/evidence/_gallery/74-overload/) | 0 critical/serious | 6 passed | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Walked by hand | built-in browser, 1440×900 dark and light, `?aij_machine=tight&aij_alert=cpu` / `pressure&aij_alert=memory`: the notice shows once; it survives Accounts → Home; dismiss clears it; Needs you item and headline | as described | as described | PASS | 42012b58+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
