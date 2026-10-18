# Evidence manifest — AJ-E1005

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1005 (morning
triage and multi-project day board). 2026-10-18, reviewer claude-opus.

## How it works

**Triage** is the first of the global views (header, before Overview).
It lists every local Instance across all projects, grouped by what it
needs from the user:

| Group | Holds |
|---|---|
| Needs you | waiting for an answer, stuck, quota blocked (not parked), crashed |
| Parked until a reset | parked by AJ-E1003 |
| Running | starting or running |
| Finished — not reviewed | finished or stopped in the last 7 days and not yet marked reviewed |

Older ended Instances are left to search and history (AJ-E1007).
Handed-over Instances continue elsewhere, so they are not triage work.

- **Sorting:** waiting longest (the default: oldest blocker first),
  project, or quota risk. Quota risk puts the soonest reset first for
  parked rows, and otherwise the highest measured share of the window
  used; unmeasured rows go last and are never guessed.
- **Rows:** each row is the E4 rail row, so it has the same indicators
  and both densities. Beside it is one primary action, always visible
  and run in place:

  | Instance | Action |
  |---|---|
  | quota blocked | Park until reset |
  | parked, reset reached | Resume |
  | finished | Mark reviewed |
  | anything else | Open |

- **Keyboard:** J/K or the arrows move between rows, Enter opens, and A
  runs the row's action. The header says so, and each action button
  carries its `A` key cap.

"Mark reviewed" is the only state the view adds. It is an appended,
timestamped event in the new lifecycle journal (`instance_lifecycle`,
V26), which AJ-E1007's archive builds on. The command refuses an
Instance that is still live.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Groups: needs attention, parked by reset, running, finished-unreviewed; sortable by waiting time, project, quota risk | Req.1 | vitest `triage.test.tsx`: each status lands in its group; reviewed, handed-over and week-old ended rows are left out; the three sorts order as stated | PASS | 2026-10-18 | claude-opus |
| Each row carries the E4 indicator set and one primary action, without a second screen | Req.2 | Rows render `SessionRow` (the rail row). The action runs in place: Park (`instance_park`), Resume (`parked_resume`), Mark reviewed, or Open. Gallery `55-triage` | PASS | 2026-10-18 | claude-opus |
| Fully keyboard operable (j/k or arrows, Enter opens, shortcuts shown); 15 Instances in a timed script | Req.3 | vitest: j moves, A parks, ArrowDown + A marks reviewed, k k + Enter opens, with no mouse. Playwright `triage — axe 0 and keyboard only` (both themes). The timed 15-Instance run by a person is in the dogfood backlog | PASS (timed run DEFERRED-TO-DOGFOOD) | 2026-10-18 | claude-opus |
| Finished-unreviewed rows leave only through an explicit reviewed action, recorded in history | Req.4 | `triage_lifecycle :: reviewed_is_an_explicit_recorded_step…`: unreviewed until marked; `lifecycle = reviewed` with its time; a live Instance is refused; kept across a restart. `storage_roundtrip :: lifecycle_events_append_and_the_latest_wins`: events append, the latest wins | PASS | 2026-10-18 | claude-opus |
| Render < 300 ms at 50 Instances; action round trip < 200 ms to a visible change | perf | vitest renders 50 rows under the 300 ms bound (jsdom, slower than the webview). Mark reviewed → visible in the snapshot: **0.29 ms** (`[perf]` in the test) | PASS | 2026-10-18 | claude-opus |
| Visual: groups populated and empty, 1440×900 and 1024×680, both themes and densities; group meaning text-labelled; primary actions visible without hover | visual | Gallery `55-triage`: 8 populated (2 viewports × 2 densities × 2 themes) and 4 empty. Each group has a word title, an icon and a count, and its own empty line; actions are plain buttons | PASS | 2026-10-18 | claude-opus |
| Timed triage run (AJ-E809 cohort): 15 Instances in under 2 min, friction filed | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test triage_lifecycle`: 1/1.
- `cargo test -p juggler-core --test storage_roundtrip`: all green at
  schema v26 (+1 lifecycle test).
- vitest `triage.test.tsx` 5/5. Playwright `55 triage` 12,
  `triage — axe 0 and keyboard only` 2.
